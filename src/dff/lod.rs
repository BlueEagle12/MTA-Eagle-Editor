//! Asynchronous DFF level-of-detail generation.
//!
//! Mesh simplification, texture decoding/resizing/compression, DFF/TXD
//! serialization happen on workers. The main thread snapshots each source,
//! merges generated atlases into the shared world LOD dictionary, and installs
//! completed results in deterministic order.

use super::super::*;
use meshopt::{
    SimplifyOptions, VertexDataAdapter, simplify_with_attributes_and_locks, typed_to_bytes,
};
use std::mem;

// Aim aggressively, but allow the topology-aware simplifier to stop before the
// target when reaching it would exceed the silhouette error budget. A slightly
// higher triangle budget and tighter error limit keep the result recognizable
// at the generated definition's long-range LOD distance.
const LOD_TARGET_TRIANGLE_RATIO: f32 = 0.12;
const LOD_SIMPLIFY_ERROR: f32 = 0.06;
const LOD_MAX_TEXTURE_DIMENSION: u32 = 512;
const LOD_MAX_ATLAS_DIMENSION: u32 = 512;
const LOD_MIN_ATLAS_DIMENSION: u32 = 32;
/// Accept a smaller atlas only while it still holds this much of the source
/// texel density. Below it the next power of two is worth the extra bytes.
const LOD_ATLAS_MIN_SCALE: f32 = 0.7;
/// A material is baked into an atlas cell at exactly the repeat count its UVs
/// ask for, so the cell always shows the tiling the source material shows.
/// Past this many repeats the duplicated pixels are not worth the atlas space
/// and the texture stays standalone instead.
const LOD_ATLAS_MAX_BAKED_REPEATS: f32 = 2.0;
/// A standalone tiled texture repeats across its surface, so every screen pixel
/// already averages many texels. It does not need full source resolution.
const LOD_TILED_TEXTURE_DIMENSION: u32 = 128;
const LOD_MIN_SIMPLIFY_TRIANGLES: usize = 4;
/// Share of the surface that may end up in pieces the source did not have.
/// Sized to pass shed slivers and fail an actually shredded model.
const LOD_MAX_DETACHED_AREA_RATIO: f32 = 0.0025;
const DEFAULT_LOD_BATCH_MINIMUM_SIZE: f32 = 100.0;
pub(crate) const WORLD_LOD_TXD_NAME: &str = "world_lod.txd";
/// A VER2 IMG directory stores an entry's streaming size in 16 bits, so no
/// asset may exceed 65,535 sectors (128 MB). Generated atlases roll over into
/// `world_lod_2.txd`, `world_lod_3.txd`, ... well before that ceiling: a
/// dictionary this large is already more than the streamer wants resident.
const WORLD_LOD_TXD_MAX_BYTES: usize = 48 * 1024 * 1024;

/// Name of the nth shared LOD dictionary. The first keeps the historical name
/// so existing projects keep loading their generated LODs.
fn world_lod_txd_name(index: usize) -> String {
    if index == 0 {
        WORLD_LOD_TXD_NAME.to_string()
    } else {
        format!("world_lod_{}.txd", index + 1)
    }
}

#[derive(Clone)]
enum LodMeshSource {
    Raw(RawMesh),
    Bytes(Arc<Vec<u8>>),
    Entry(ImgEntry),
}

#[derive(Clone)]
enum LodTxdSource {
    Bytes(Arc<Vec<u8>>),
    Entry(ImgEntry),
}

struct LodGenerationRequest {
    mesh_source: LodMeshSource,
    txd_source: Option<LodTxdSource>,
    output_stem: String,
    attach_to_placement: Option<usize>,
    source_definition_id: Option<String>,
    source_zone: String,
    source_dff_name: String,
    source_txd_name: Option<String>,
    replaced_lod_parent: Option<String>,
    replaced_lod_index: Option<usize>,
    project_root: PathBuf,
    reserved_stems: Arc<BTreeSet<String>>,
    wip_root: PathBuf,
    stem_preallocated: bool,
}

pub(crate) struct LodGenerationResult {
    stem: String,
    dff_name: String,
    dff_bytes: Vec<u8>,
    col_name: String,
    col_bytes: Vec<u8>,
    col_mesh: CollisionMesh,
    txd_name: Option<String>,
    txd_bytes: Option<Vec<u8>>,
    raw: RawMesh,
    attach_to_placement: Option<usize>,
    source_definition_id: Option<String>,
    source_zone: String,
    source_dff_name: String,
    source_txd_name: Option<String>,
    replaced_lod_parent: Option<String>,
    replaced_lod_index: Option<usize>,
    source_triangles: usize,
    source_vertices: usize,
    output_triangles: usize,
    output_vertices: usize,
    simplification_attempts: usize,
    texture_count: usize,
    source_had_textures: bool,
    atlased: bool,
}

pub(crate) struct LodGenerationJob {
    workers: Vec<LodGenerationWorker>,
    started_at: Instant,
    result: Option<LodGenerationResult>,
    apply_phase: u8,
    attached: bool,
    pending: std::collections::VecDeque<(usize, LodGenerationRequest)>,
    ready: BTreeMap<usize, Result<LodGenerationResult, String>>,
    next_apply_order: usize,
    request_count: usize,
    total: usize,
    processed: usize,
    successes: usize,
    attached_count: usize,
    skipped: Vec<String>,
    failures: Vec<String>,
    source_triangles: usize,
    output_triangles: usize,
    history_before: WorldHistorySnapshot,
    history_artifacts: Vec<LodGenerationHistoryArtifact>,
    /// Prior contents of every shared LOD dictionary this batch wrote to,
    /// keyed by asset key. `None` marks a dictionary the batch created.
    history_txd_before: BTreeMap<String, (String, Option<Vec<u8>>)>,
}

struct LodGenerationWorker {
    order: usize,
    source_name: String,
    rx: mpsc::Receiver<Result<LodGenerationResult, String>>,
}

const LOD_GENERATION_MAX_WORKERS: usize = 4;

#[derive(Clone)]
struct DecodedLodTexture {
    name: String,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

fn read_mesh_source(source: LodMeshSource) -> Result<RawMesh, String> {
    let raw = match source {
        LodMeshSource::Raw(raw) => raw,
        LodMeshSource::Bytes(bytes) => parse_dff_mesh(bytes.as_slice()),
        LodMeshSource::Entry(entry) => parse_dff_mesh(&read_img_entry(&entry)),
    };
    if raw.vertices.is_empty() || raw.triangles.is_empty() {
        Err("The selected DFF has no readable geometry".to_string())
    } else {
        Ok(raw)
    }
}

fn read_txd_source(source: LodTxdSource) -> Vec<u8> {
    match source {
        LodTxdSource::Bytes(bytes) => Arc::unwrap_or_clone(bytes),
        LodTxdSource::Entry(entry) => read_txd_entry_bytes(&entry),
    }
}

fn compact_lod_vertices(raw: &mut RawMesh) {
    let source_count = raw.vertices.len();
    let mut used = vec![false; source_count];
    for tri in &raw.triangles {
        for index in [tri.a, tri.b, tri.c] {
            if let Some(slot) = used.get_mut(index as usize) {
                *slot = true;
            }
        }
    }
    let mut remap = vec![0u32; source_count];
    let mut next = 0u32;
    for (index, keep) in used.iter().copied().enumerate() {
        if keep {
            remap[index] = next;
            next += 1;
        }
    }
    let compact_v3 = |values: &mut Vec<V3>| {
        if values.len() == source_count {
            *values = values
                .iter()
                .copied()
                .zip(&used)
                .filter_map(|(value, keep)| keep.then_some(value))
                .collect();
        }
    };
    compact_v3(&mut raw.vertices);
    compact_v3(&mut raw.normals);
    compact_v3(&mut raw.prelit_colors);
    compact_v3(&mut raw.night_prelit_colors);
    let compact_f32 = |values: &mut Vec<f32>| {
        if values.len() == source_count {
            *values = values
                .iter()
                .copied()
                .zip(&used)
                .filter_map(|(value, keep)| keep.then_some(value))
                .collect();
        }
    };
    compact_f32(&mut raw.prelit_alphas);
    compact_f32(&mut raw.night_prelit_alphas);
    if raw.uvs.len() == source_count {
        raw.uvs = raw
            .uvs
            .iter()
            .copied()
            .zip(&used)
            .filter_map(|(value, keep)| keep.then_some(value))
            .collect();
    }
    for uvs in &mut raw.secondary_uvs {
        if uvs.len() == source_count {
            *uvs = uvs
                .iter()
                .copied()
                .zip(&used)
                .filter_map(|(value, keep)| keep.then_some(value))
                .collect();
        }
    }
    if raw.light_flags.len() == source_count {
        raw.light_flags = raw
            .light_flags
            .iter()
            .copied()
            .zip(&used)
            .filter_map(|(value, keep)| keep.then_some(value))
            .collect();
    }
    for tri in &mut raw.triangles {
        tri.a = remap[tri.a as usize];
        tri.b = remap[tri.b as usize];
        tri.c = remap[tri.c as usize];
    }
    // The normalized writer flattens DFF frame hierarchies. Do not leave stale
    // source ranges on the generated mesh.
    raw.components.clear();
    raw.frames.clear();
}

fn lod_position_key(vertex: V3) -> [u32; 3] {
    [
        if vertex.x == 0.0 {
            0
        } else {
            vertex.x.to_bits()
        },
        if vertex.y == 0.0 {
            0
        } else {
            vertex.y.to_bits()
        },
        if vertex.z == 0.0 {
            0
        } else {
            vertex.z.to_bits()
        },
    ]
}

/// Lock a small set of support points around the mesh rather than every border
/// vertex. This preserves the coarse silhouette from axis and diagonal views
/// while still allowing long, straight outline runs to be simplified.
fn lod_component_support_positions(vertices: &[V3], indices: &[u32]) -> BTreeSet<[u32; 3]> {
    let used = indices
        .iter()
        .filter_map(|index| vertices.get(*index as usize).copied())
        .collect::<Vec<_>>();
    let Some(first) = used.first().copied() else {
        return BTreeSet::new();
    };
    let mut min = first;
    let mut max = first;
    for vertex in used.iter().copied().skip(1) {
        min.x = min.x.min(vertex.x);
        min.y = min.y.min(vertex.y);
        min.z = min.z.min(vertex.z);
        max.x = max.x.max(vertex.x);
        max.y = max.y.max(vertex.y);
        max.z = max.z.max(vertex.z);
    }
    let center = V3 {
        x: (min.x + max.x) * 0.5,
        y: (min.y + max.y) * 0.5,
        z: (min.z + max.z) * 0.5,
    };
    let extent = V3 {
        x: (max.x - min.x).max(f32::EPSILON),
        y: (max.y - min.y).max(f32::EPSILON),
        z: (max.z - min.z).max(f32::EPSILON),
    };
    let normalized: Vec<[f32; 3]> = used
        .iter()
        .map(|vertex| {
            [
                (vertex.x - center.x) / extent.x,
                (vertex.y - center.y) / extent.y,
                (vertex.z - center.z) / extent.z,
            ]
        })
        .collect();
    let mut protected_positions = BTreeSet::<[u32; 3]>::new();
    for dz in -1..=1 {
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 && dz == 0 {
                    continue;
                }
                let direction = [dx as f32, dy as f32, dz as f32];
                let best = normalized
                    .iter()
                    .enumerate()
                    .filter_map(|(index, position)| {
                        let score = position[0] * direction[0]
                            + position[1] * direction[1]
                            + position[2] * direction[2];
                        score.is_finite().then_some((index, score))
                    })
                    .max_by(|(_, a), (_, b)| a.total_cmp(b))
                    .map(|(index, _)| index);
                if let Some(index) = best {
                    protected_positions.insert(lod_position_key(used[index]));
                }
            }
        }
    }
    protected_positions
}

/// Find the position-welded outline of an open mesh.
///
/// Roads, terrain patches, roofs, and similar map pieces are often broad open
/// surfaces. Keeping only their extrema allows the simplifier to bridge across
/// bends or tapers, turning the outline into a handful of large fans. Use
/// position keys here so a UV or normal seam does not make an otherwise closed
/// edge look like part of the silhouette.
fn lod_open_boundary_positions(source: &RawMesh) -> BTreeSet<[u32; 3]> {
    let mut edge_counts = BTreeMap::<([u32; 3], [u32; 3]), usize>::new();
    for triangle in &source.triangles {
        for (a, b) in [
            (triangle.a, triangle.b),
            (triangle.b, triangle.c),
            (triangle.c, triangle.a),
        ] {
            let (Some(a), Some(b)) = (
                source.vertices.get(a as usize),
                source.vertices.get(b as usize),
            ) else {
                continue;
            };
            let mut edge = (lod_position_key(*a), lod_position_key(*b));
            if edge.1 < edge.0 {
                mem::swap(&mut edge.0, &mut edge.1);
            }
            *edge_counts.entry(edge).or_default() += 1;
        }
    }

    edge_counts
        .into_iter()
        .filter(|(_, count)| *count == 1)
        .flat_map(|((a, b), _)| [a, b])
        .collect()
}

/// Split material batches into position-connected pieces for visibility
/// classification. The pieces are recombined by material before simplification
/// so meshopt can prune minor disconnected details instead of reserving at least
/// one triangle for every fragment.
fn lod_material_components(source: &RawMesh) -> Vec<(u16, Vec<u32>)> {
    let position_keys = source
        .vertices
        .iter()
        .copied()
        .map(lod_position_key)
        .collect::<Vec<_>>();
    let mut by_material = BTreeMap::<u16, Vec<[u32; 3]>>::new();
    for triangle in &source.triangles {
        let indices = [triangle.a, triangle.b, triangle.c];
        if indices
            .iter()
            .any(|index| *index as usize >= position_keys.len())
        {
            continue;
        }
        by_material
            .entry(triangle.material)
            .or_default()
            .push(indices);
    }

    let mut groups = Vec::new();
    for (material, triangles) in by_material {
        let mut parents = (0..triangles.len()).collect::<Vec<_>>();
        let mut first_triangle_at_position = BTreeMap::<[u32; 3], usize>::new();
        for (triangle_index, triangle) in triangles.iter().enumerate() {
            for vertex in triangle {
                let key = position_keys[*vertex as usize];
                if let Some(other) = first_triangle_at_position.insert(key, triangle_index) {
                    let mut a = triangle_index;
                    while parents[a] != a {
                        a = parents[a];
                    }
                    let mut b = other;
                    while parents[b] != b {
                        b = parents[b];
                    }
                    if a != b {
                        parents[b] = a;
                    }
                }
            }
        }
        for triangle_index in 0..parents.len() {
            let mut root = triangle_index;
            while parents[root] != root {
                root = parents[root];
            }
            let mut current = triangle_index;
            while parents[current] != current {
                let next = parents[current];
                parents[current] = root;
                current = next;
            }
        }
        let mut connected = BTreeMap::<usize, Vec<u32>>::new();
        for (triangle_index, triangle) in triangles.into_iter().enumerate() {
            connected
                .entry(parents[triangle_index])
                .or_default()
                .extend_from_slice(&triangle);
        }
        groups.extend(connected.into_values().map(|indices| (material, indices)));
    }
    groups
}

fn lod_geometry_components(source: &RawMesh) -> Vec<Vec<u32>> {
    let mut geometry = source.clone();
    for triangle in &mut geometry.triangles {
        triangle.material = 0;
    }
    lod_material_components(&geometry)
        .into_iter()
        .map(|(_, indices)| indices)
        .collect()
}

fn lod_triangle_area(vertices: &[V3], indices: &[u32]) -> f32 {
    indices
        .chunks_exact(3)
        .filter_map(|triangle| {
            let a = vertices.get(triangle[0] as usize)?;
            let b = vertices.get(triangle[1] as usize)?;
            let c = vertices.get(triangle[2] as usize)?;
            let ab = V3 {
                x: b.x - a.x,
                y: b.y - a.y,
                z: b.z - a.z,
            };
            let ac = V3 {
                x: c.x - a.x,
                y: c.y - a.y,
                z: c.z - a.z,
            };
            let cross = V3 {
                x: ab.y * ac.z - ab.z * ac.y,
                y: ab.z * ac.x - ab.x * ac.z,
                z: ab.x * ac.y - ab.y * ac.x,
            };
            Some((cross.x * cross.x + cross.y * cross.y + cross.z * cross.z).sqrt() * 0.5)
        })
        .sum()
}

fn lod_indices_bounds(vertices: &[V3], indices: &[u32]) -> Option<(V3, V3)> {
    let mut used = indices
        .iter()
        .filter_map(|index| vertices.get(*index as usize).copied());
    let first = used.next()?;
    Some(used.fold((first, first), |(mut min, mut max), vertex| {
        min.x = min.x.min(vertex.x);
        min.y = min.y.min(vertex.y);
        min.z = min.z.min(vertex.z);
        max.x = max.x.max(vertex.x);
        max.y = max.y.max(vertex.y);
        max.z = max.z.max(vertex.z);
        (min, max)
    }))
}

/// Select support points only from material components that contribute a
/// meaningful part of the visible outer envelope. Small inset/internal pieces
/// remain unlocked and can be pruned before meshopt has to move silhouette
/// vertices.
fn lod_protected_silhouette_positions(
    source: &RawMesh,
    components: &[(u16, Vec<u32>)],
) -> BTreeSet<[u32; 3]> {
    let all_indices = source
        .triangles
        .iter()
        .flat_map(|triangle| [triangle.a, triangle.b, triangle.c])
        .collect::<Vec<_>>();
    let Some((mesh_min, mesh_max)) = lod_indices_bounds(&source.vertices, &all_indices) else {
        return BTreeSet::new();
    };
    let mesh_span = [
        (mesh_max.x - mesh_min.x).max(0.0),
        (mesh_max.y - mesh_min.y).max(0.0),
        (mesh_max.z - mesh_min.z).max(0.0),
    ];
    let mesh_scale = mesh_span
        .iter()
        .copied()
        .fold(0.0f32, f32::max)
        .max(f32::EPSILON);
    let shell_epsilon = mesh_scale * 0.0025;
    let total_area = components
        .iter()
        .map(|(_, indices)| lod_triangle_area(&source.vertices, indices))
        .sum::<f32>()
        .max(f32::EPSILON);
    let only_component = components.len() == 1;
    let mut protected = BTreeSet::new();

    for (_, indices) in components {
        let Some((min, max)) = lod_indices_bounds(&source.vertices, indices) else {
            continue;
        };
        let component_span = [max.x - min.x, max.y - min.y, max.z - min.z];
        let normalized_span = component_span
            .iter()
            .zip(mesh_span)
            .filter(|(_, mesh_axis)| *mesh_axis > f32::EPSILON)
            .map(|(component_axis, mesh_axis)| (component_axis / mesh_axis).powi(2))
            .sum::<f32>()
            .sqrt();
        let area_ratio = lod_triangle_area(&source.vertices, indices) / total_area;
        let touches_shell = [
            (min.x - mesh_min.x).abs(),
            (max.x - mesh_max.x).abs(),
            (min.y - mesh_min.y).abs(),
            (max.y - mesh_max.y).abs(),
            (min.z - mesh_min.z).abs(),
            (max.z - mesh_max.z).abs(),
        ]
        .into_iter()
        .any(|distance| distance <= shell_epsilon);
        let meaningful = normalized_span >= 0.08 || area_ratio >= 0.01;
        if only_component || (touches_shell && meaningful) {
            protected.extend(lod_component_support_positions(&source.vertices, indices));
        }
    }
    protected
}

fn simplify_lod_mesh_once(
    source: &RawMesh,
    target_triangle_ratio: f32,
    simplify_error: f32,
) -> Result<RawMesh, String> {
    let positions: Vec<[f32; 3]> = source
        .vertices
        .iter()
        .map(|vertex| [vertex.x, vertex.y, vertex.z])
        .collect();
    let adapter = VertexDataAdapter::new(typed_to_bytes(&positions), mem::size_of::<[f32; 3]>(), 0)
        .map_err(|error| format!("Mesh simplifier setup failed: {error}"))?;

    let has_normals = source.normals.len() == source.vertices.len();
    let has_uvs = source.uvs.len() == source.vertices.len();
    let secondary_uv_sets = source
        .secondary_uvs
        .iter()
        .filter(|uvs| uvs.len() == source.vertices.len())
        .collect::<Vec<_>>();
    let attribute_stride = 5usize + secondary_uv_sets.len() * 2;
    let mut attributes = Vec::with_capacity(source.vertices.len() * attribute_stride);
    for index in 0..source.vertices.len() {
        let normal = source.normals.get(index).copied().unwrap_or(V3 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        });
        let uv = source.uvs.get(index).copied().unwrap_or_default();
        attributes.extend_from_slice(&[
            if has_normals { normal.x } else { 0.0 },
            if has_normals { normal.y } else { 0.0 },
            if has_normals { normal.z } else { 0.0 },
            if has_uvs { uv.u } else { 0.0 },
            if has_uvs { uv.v } else { 0.0 },
        ]);
        for uvs in &secondary_uv_sets {
            attributes.extend_from_slice(&[uvs[index].u, uvs[index].v]);
        }
    }
    // Normals and UVs remain part of the collapse error, but topology and
    // positions dominate. UV seams are represented by separate source vertices
    // and permissive cross-seam collapses are deliberately disabled below.
    let mut weights = vec![0.15f32, 0.15, 0.15, 0.035, 0.035];
    weights.extend(std::iter::repeat_n(0.035, secondary_uv_sets.len() * 2));
    let material_components = lod_material_components(source);
    let mut protected_positions = lod_protected_silhouette_positions(source, &material_components);
    protected_positions.extend(lod_open_boundary_positions(source));
    let mut exact_materials = BTreeMap::<[u32; 3], u16>::new();
    let mut vertex_materials = vec![BTreeMap::<u16, usize>::new(); source.vertices.len()];
    for triangle in &source.triangles {
        let mut key = [triangle.a, triangle.b, triangle.c];
        key.sort_unstable();
        exact_materials.insert(key, triangle.material);
        for index in key {
            if let Some(materials) = vertex_materials.get_mut(index as usize) {
                *materials.entry(triangle.material).or_default() += 1;
            }
        }
    }
    let triangle_material = |indices: &[u32]| {
        let mut key = [indices[0], indices[1], indices[2]];
        key.sort_unstable();
        if let Some(material) = exact_materials.get(&key) {
            return *material;
        }
        let mut votes = BTreeMap::<u16, usize>::new();
        for index in indices {
            if let Some(materials) = vertex_materials.get(*index as usize) {
                for (material, count) in materials {
                    *votes.entry(*material).or_default() += count;
                }
            }
        }
        votes
            .into_iter()
            .max_by(|(material_a, count_a), (material_b, count_b)| {
                count_a
                    .cmp(count_b)
                    .then_with(|| material_b.cmp(material_a))
            })
            .map(|(material, _)| material)
            .unwrap_or_default()
    };
    let mut triangles = Vec::new();
    for indices in lod_geometry_components(source) {
        let source_faces = indices.len() / 3;
        let simplified = if source_faces < LOD_MIN_SIMPLIFY_TRIANGLES {
            indices
        } else {
            let locks = source
                .vertices
                .iter()
                .copied()
                .map(|vertex| protected_positions.contains(&lod_position_key(vertex)))
                .collect::<Vec<_>>();
            let target_faces = ((source_faces as f32 * target_triangle_ratio).ceil() as usize)
                .clamp(1, source_faces);
            let candidate = simplify_with_attributes_and_locks(
                &indices,
                &adapter,
                &attributes,
                &weights,
                attribute_stride * mem::size_of::<f32>(),
                &locks,
                target_faces * 3,
                simplify_error,
                SimplifyOptions::Regularize | SimplifyOptions::Sparse,
                None,
            );
            if candidate.len() >= 3 && candidate.len() % 3 == 0 && candidate.len() <= indices.len()
            {
                candidate
            } else {
                indices
            }
        };
        triangles.extend(simplified.chunks_exact(3).map(|indices| Tri {
            a: indices[0],
            b: indices[1],
            c: indices[2],
            material: triangle_material(indices),
        }));
    }
    if triangles.is_empty() {
        return Err("Mesh simplification produced no geometry".to_string());
    }
    let mut out = source.clone();
    out.triangles = triangles;
    // Distant geometry should not duplicate local particle/light effects.
    out.effects_2dfx.clear();
    compact_lod_vertices(&mut out);
    Ok(out)
}

#[cfg(test)]
fn lod_geometry_component_count(raw: &RawMesh) -> usize {
    lod_geometry_components(raw).len()
}

/// Connected pieces of a mesh, treating two triangles as joined when they share
/// an edge rather than a single welded position.
///
/// Kitbashed map geometry routinely has parts that only touch at one corner - a
/// balcony meeting a wall, a kerb meeting a road. Those contacts are
/// incidental: simplification nudges the shared vertex and the contact is gone.
/// Counting them as connections made the LOD validator report a mesh as "split
/// into additional disconnected pieces" when nothing had actually been torn,
/// and it rejected every quality target for such models.
fn lod_connected_pieces(raw: &RawMesh) -> Vec<Vec<u32>> {
    fn root(parents: &mut [usize], mut node: usize) -> usize {
        while parents[node] != node {
            parents[node] = parents[parents[node]];
            node = parents[node];
        }
        node
    }

    let mut parents = (0..raw.triangles.len()).collect::<Vec<_>>();
    let mut triangle_at_edge = BTreeMap::<([u32; 3], [u32; 3]), usize>::new();
    for (index, triangle) in raw.triangles.iter().enumerate() {
        for (a, b) in [
            (triangle.a, triangle.b),
            (triangle.b, triangle.c),
            (triangle.c, triangle.a),
        ] {
            let (Some(a_position), Some(b_position)) =
                (raw.vertices.get(a as usize), raw.vertices.get(b as usize))
            else {
                continue;
            };
            let mut a_key = lod_position_key(*a_position);
            let mut b_key = lod_position_key(*b_position);
            if b_key < a_key {
                mem::swap(&mut a_key, &mut b_key);
            }
            let Some(other) = triangle_at_edge.insert((a_key, b_key), index) else {
                continue;
            };
            let (left, right) = (root(&mut parents, index), root(&mut parents, other));
            if left != right {
                parents[right] = left;
            }
        }
    }
    let mut pieces = BTreeMap::<usize, Vec<u32>>::new();
    for (index, triangle) in raw.triangles.iter().enumerate() {
        pieces
            .entry(root(&mut parents, index))
            .or_default()
            .extend([triangle.a, triangle.b, triangle.c]);
    }
    pieces.into_values().collect()
}

fn lod_edge_topology(raw: &RawMesh) -> (usize, usize, f32) {
    let mut edges = BTreeMap::<([u32; 3], [u32; 3]), (usize, f32)>::new();
    for triangle in &raw.triangles {
        for (a, b) in [
            (triangle.a, triangle.b),
            (triangle.b, triangle.c),
            (triangle.c, triangle.a),
        ] {
            let (Some(a_position), Some(b_position)) =
                (raw.vertices.get(a as usize), raw.vertices.get(b as usize))
            else {
                continue;
            };
            let mut a_key = lod_position_key(*a_position);
            let mut b_key = lod_position_key(*b_position);
            if b_key < a_key {
                mem::swap(&mut a_key, &mut b_key);
            }
            let dx = b_position.x - a_position.x;
            let dy = b_position.y - a_position.y;
            let dz = b_position.z - a_position.z;
            let length = (dx * dx + dy * dy + dz * dz).sqrt();
            let edge = edges.entry((a_key, b_key)).or_insert((0, length));
            edge.0 += 1;
        }
    }
    let open_edges = edges.values().filter(|(count, _)| *count == 1).count();
    let non_manifold_edges = edges.values().filter(|(count, _)| *count > 2).count();
    let open_length = edges
        .values()
        .filter_map(|(count, length)| (*count == 1).then_some(*length))
        .sum();
    (open_edges, non_manifold_edges, open_length)
}

fn lod_flipped_normal_ratio(raw: &RawMesh) -> f32 {
    if raw.normals.len() != raw.vertices.len() || raw.triangles.is_empty() {
        return 0.0;
    }
    let mut measurable = 0usize;
    let mut flipped = 0usize;
    for triangle in &raw.triangles {
        let (Some(a), Some(b), Some(c)) = (
            raw.vertices.get(triangle.a as usize),
            raw.vertices.get(triangle.b as usize),
            raw.vertices.get(triangle.c as usize),
        ) else {
            continue;
        };
        let ab = V3 {
            x: b.x - a.x,
            y: b.y - a.y,
            z: b.z - a.z,
        };
        let ac = V3 {
            x: c.x - a.x,
            y: c.y - a.y,
            z: c.z - a.z,
        };
        let face = V3 {
            x: ab.y * ac.z - ab.z * ac.y,
            y: ab.z * ac.x - ab.x * ac.z,
            z: ab.x * ac.y - ab.y * ac.x,
        };
        let normal = [triangle.a, triangle.b, triangle.c]
            .into_iter()
            .filter_map(|index| raw.normals.get(index as usize))
            .fold(V3::default(), |mut sum, normal| {
                sum.x += normal.x;
                sum.y += normal.y;
                sum.z += normal.z;
                sum
            });
        let face_length = (face.x * face.x + face.y * face.y + face.z * face.z).sqrt();
        let normal_length =
            (normal.x * normal.x + normal.y * normal.y + normal.z * normal.z).sqrt();
        if face_length > 1e-8 && normal_length > 1e-8 {
            measurable += 1;
            if face.x * normal.x + face.y * normal.y + face.z * normal.z < 0.0 {
                flipped += 1;
            }
        }
    }
    flipped as f32 / measurable.max(1) as f32
}

fn lod_uv_distortions(raw: &RawMesh) -> Vec<f32> {
    if raw.uvs.len() != raw.vertices.len() {
        return Vec::new();
    }
    let mut distortions = Vec::new();
    for triangle in &raw.triangles {
        let indices = [
            triangle.a as usize,
            triangle.b as usize,
            triangle.c as usize,
        ];
        let Some(vertices) = indices
            .iter()
            .map(|index| raw.vertices.get(*index).copied())
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        let Some(uvs) = indices
            .iter()
            .map(|index| raw.uvs.get(*index).copied())
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        let mut scales = Vec::with_capacity(3);
        for (a, b) in [(0usize, 1usize), (1, 2), (2, 0)] {
            let dx = vertices[b].x - vertices[a].x;
            let dy = vertices[b].y - vertices[a].y;
            let dz = vertices[b].z - vertices[a].z;
            let geometry_length = (dx * dx + dy * dy + dz * dz).sqrt();
            let du = uvs[b].u - uvs[a].u;
            let dv = uvs[b].v - uvs[a].v;
            let uv_length = (du * du + dv * dv).sqrt();
            if geometry_length > 1e-7 && uv_length > 1e-7 {
                scales.push(uv_length / geometry_length);
            }
        }
        if scales.len() == 3 {
            let minimum = scales.iter().copied().fold(f32::INFINITY, f32::min);
            let maximum = scales.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            let distortion = maximum / minimum.max(1e-8);
            if distortion.is_finite() {
                distortions.push(distortion);
            }
        }
    }
    distortions.sort_by(f32::total_cmp);
    distortions
}

fn lod_uv_degenerate_ratio(raw: &RawMesh) -> f32 {
    if raw.uvs.len() != raw.vertices.len() || raw.triangles.is_empty() {
        return 0.0;
    }
    let degenerate = raw
        .triangles
        .iter()
        .filter(|triangle| {
            let Some(a) = raw.uvs.get(triangle.a as usize) else {
                return true;
            };
            let Some(b) = raw.uvs.get(triangle.b as usize) else {
                return true;
            };
            let Some(c) = raw.uvs.get(triangle.c as usize) else {
                return true;
            };
            let area = (b.u - a.u) * (c.v - a.v) - (b.v - a.v) * (c.u - a.u);
            !area.is_finite() || area.abs() <= 1e-10
        })
        .count();
    degenerate as f32 / raw.triangles.len() as f32
}

fn lod_projection_mask(raw: &RawMesh, axes: (usize, usize), bounds: (V3, V3)) -> Vec<bool> {
    const SIZE: usize = 40;
    let coordinate = |vertex: V3, axis: usize| match axis {
        0 => vertex.x,
        1 => vertex.y,
        _ => vertex.z,
    };
    let axis_min = [bounds.0.x, bounds.0.y, bounds.0.z];
    let axis_max = [bounds.1.x, bounds.1.y, bounds.1.z];
    let span_a = axis_max[axes.0] - axis_min[axes.0];
    let span_b = axis_max[axes.1] - axis_min[axes.1];
    let mut mask = vec![false; SIZE * SIZE];
    if span_a <= f32::EPSILON || span_b <= f32::EPSILON {
        return mask;
    }
    for triangle in &raw.triangles {
        let Some(projected) = [triangle.a, triangle.b, triangle.c]
            .into_iter()
            .map(|index| {
                raw.vertices.get(index as usize).map(|vertex| {
                    (
                        (coordinate(*vertex, axes.0) - axis_min[axes.0]) / span_a
                            * (SIZE - 1) as f32,
                        (coordinate(*vertex, axes.1) - axis_min[axes.1]) / span_b
                            * (SIZE - 1) as f32,
                    )
                })
            })
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        let signed_area = (projected[1].0 - projected[0].0) * (projected[2].1 - projected[0].1)
            - (projected[1].1 - projected[0].1) * (projected[2].0 - projected[0].0);
        if signed_area.abs() <= 1e-6 {
            continue;
        }
        let min_x = projected
            .iter()
            .map(|point| point.0)
            .fold(f32::INFINITY, f32::min)
            .floor()
            .clamp(0.0, (SIZE - 1) as f32) as usize;
        let max_x = projected
            .iter()
            .map(|point| point.0)
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            .clamp(0.0, (SIZE - 1) as f32) as usize;
        let min_y = projected
            .iter()
            .map(|point| point.1)
            .fold(f32::INFINITY, f32::min)
            .floor()
            .clamp(0.0, (SIZE - 1) as f32) as usize;
        let max_y = projected
            .iter()
            .map(|point| point.1)
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            .clamp(0.0, (SIZE - 1) as f32) as usize;
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let point = (x as f32 + 0.5, y as f32 + 0.5);
                let edge = |a: (f32, f32), b: (f32, f32)| {
                    (b.0 - a.0) * (point.1 - a.1) - (b.1 - a.1) * (point.0 - a.0)
                };
                let e0 = edge(projected[0], projected[1]);
                let e1 = edge(projected[1], projected[2]);
                let e2 = edge(projected[2], projected[0]);
                if (e0 >= -1e-5 && e1 >= -1e-5 && e2 >= -1e-5)
                    || (e0 <= 1e-5 && e1 <= 1e-5 && e2 <= 1e-5)
                {
                    mask[y * SIZE + x] = true;
                }
            }
        }
    }
    mask
}

fn validate_lod_candidate(source: &RawMesh, candidate: &RawMesh) -> Result<(), String> {
    if candidate.vertices.is_empty() || candidate.triangles.is_empty() {
        return Err("candidate contains no geometry".to_string());
    }
    if candidate.triangles.iter().any(|triangle| {
        [triangle.a, triangle.b, triangle.c]
            .iter()
            .any(|index| (*index as usize) >= candidate.vertices.len())
    }) {
        return Err("candidate contains an out-of-range triangle index".to_string());
    }

    let source_indices = source
        .triangles
        .iter()
        .flat_map(|triangle| [triangle.a, triangle.b, triangle.c])
        .collect::<Vec<_>>();
    let candidate_indices = candidate
        .triangles
        .iter()
        .flat_map(|triangle| [triangle.a, triangle.b, triangle.c])
        .collect::<Vec<_>>();
    let source_bounds = lod_indices_bounds(&source.vertices, &source_indices)
        .ok_or_else(|| "source bounds are unavailable".to_string())?;
    let candidate_bounds = lod_indices_bounds(&candidate.vertices, &candidate_indices)
        .ok_or_else(|| "candidate bounds are unavailable".to_string())?;
    let mesh_scale = [
        source_bounds.1.x - source_bounds.0.x,
        source_bounds.1.y - source_bounds.0.y,
        source_bounds.1.z - source_bounds.0.z,
    ]
    .into_iter()
    .fold(0.0f32, f32::max)
    .max(1e-5);
    let extent_error = [
        candidate_bounds.0.x - source_bounds.0.x,
        candidate_bounds.0.y - source_bounds.0.y,
        candidate_bounds.0.z - source_bounds.0.z,
        candidate_bounds.1.x - source_bounds.1.x,
        candidate_bounds.1.y - source_bounds.1.y,
        candidate_bounds.1.z - source_bounds.1.z,
    ]
    .into_iter()
    .map(f32::abs)
    .fold(0.0f32, f32::max);
    if extent_error > mesh_scale * 0.005 {
        return Err(format!(
            "silhouette bounds moved by {:.2}% of the object size",
            extent_error / mesh_scale * 100.0
        ));
    }

    let source_area = lod_triangle_area(&source.vertices, &source_indices).max(1e-8);
    let candidate_area = lod_triangle_area(&candidate.vertices, &candidate_indices);
    let area_ratio = candidate_area / source_area;
    if !(0.55..=1.35).contains(&area_ratio) {
        return Err(format!(
            "surface area changed to {:.1}% of the source",
            area_ratio * 100.0
        ));
    }

    let source_pieces = lod_connected_pieces(source).len();
    let candidate_pieces = lod_connected_pieces(candidate);
    if candidate_pieces.len() > source_pieces {
        // Collapsing an edge can shed a sliver a few thousandths of a square
        // unit across. What this check is here to catch is the simplifier
        // shredding the model, so weigh the extra pieces by surface instead of
        // counting them.
        let mut areas = candidate_pieces
            .iter()
            .map(|indices| lod_triangle_area(&candidate.vertices, indices))
            .collect::<Vec<_>>();
        areas.sort_by(|a, b| b.total_cmp(a));
        let detached = areas[source_pieces..].iter().sum::<f32>();
        if detached > candidate_area * LOD_MAX_DETACHED_AREA_RATIO {
            return Err(format!(
                "simplification split {:.2}% of the surface into {} additional disconnected piece(s) ({source_pieces} -> {})",
                detached / candidate_area.max(1e-8) * 100.0,
                candidate_pieces.len() - source_pieces,
                candidate_pieces.len()
            ));
        }
    }
    let (source_open, source_non_manifold, source_open_length) = lod_edge_topology(source);
    let (candidate_open, candidate_non_manifold, candidate_open_length) =
        lod_edge_topology(candidate);
    // meshopt keeps a well-formed mesh well-formed, so new non-manifold edges on
    // a clean source are a real defect. Map geometry that already has coincident
    // surfaces gains and loses a handful of them at every quality target, and
    // rejecting that turned "already slightly non-manifold" into "no LOD at all".
    if candidate_non_manifold > source_non_manifold * 2 + 2 {
        return Err(format!(
            "simplification introduced non-manifold edges ({source_non_manifold} -> {candidate_non_manifold})"
        ));
    }
    if candidate_open > source_open.saturating_add(2)
        && candidate_open_length > source_open_length + mesh_scale * 0.02
    {
        return Err(format!(
            "simplification introduced open edges or holes ({} / {:.3} -> {} / {:.3})",
            source_open, source_open_length, candidate_open, candidate_open_length
        ));
    }
    let source_flipped = lod_flipped_normal_ratio(source);
    let candidate_flipped = lod_flipped_normal_ratio(candidate);
    if candidate_flipped > source_flipped + 0.02 {
        return Err("simplification introduced flipped face normals".to_string());
    }

    for axes in [(0usize, 1usize), (0, 2), (1, 2)] {
        let source_mask = lod_projection_mask(source, axes, source_bounds);
        let source_pixels = source_mask.iter().filter(|covered| **covered).count();
        if source_pixels < 8 {
            continue;
        }
        let candidate_mask = lod_projection_mask(candidate, axes, source_bounds);
        let intersection = source_mask
            .iter()
            .zip(&candidate_mask)
            .filter(|(source, candidate)| **source && **candidate)
            .count();
        if intersection as f32 / (source_pixels as f32) < 0.86 {
            return Err("projected silhouette lost visible surface coverage".to_string());
        }
        let candidate_pixels = candidate_mask.iter().filter(|covered| **covered).count();
        if candidate_pixels >= 8 && intersection as f32 / (candidate_pixels as f32) < 0.86 {
            return Err("projected silhouette gained surface outside the source shape".to_string());
        }
    }

    if source.uvs.len() == source.vertices.len() {
        if candidate.uvs.len() != candidate.vertices.len() {
            return Err("candidate lost its vertex UV mapping".to_string());
        }
        let source_vertex_uvs = source
            .vertices
            .iter()
            .zip(&source.uvs)
            .map(|(vertex, uv)| (lod_position_key(*vertex), [uv.u.to_bits(), uv.v.to_bits()]))
            .collect::<BTreeSet<_>>();
        if candidate
            .vertices
            .iter()
            .zip(&candidate.uvs)
            .any(|(vertex, uv)| {
                !uv.u.is_finite()
                    || !uv.v.is_finite()
                    || !source_vertex_uvs
                        .contains(&(lod_position_key(*vertex), [uv.u.to_bits(), uv.v.to_bits()]))
            })
        {
            return Err("candidate changed or invalidated existing vertex UVs".to_string());
        }
        if lod_uv_degenerate_ratio(candidate) > lod_uv_degenerate_ratio(source) + 0.03 {
            return Err("candidate collapsed too many textured polygons in UV space".to_string());
        }
        let source_distortions = lod_uv_distortions(source);
        let candidate_distortions = lod_uv_distortions(candidate);
        if let (Some(source_limit), Some(candidate_limit)) = (
            source_distortions.get(source_distortions.len().saturating_mul(95) / 100),
            candidate_distortions.get(candidate_distortions.len().saturating_mul(95) / 100),
        ) && *candidate_limit > (*source_limit * 2.0).max(24.0)
        {
            return Err("candidate contains severely stretched UV triangles".to_string());
        }
    }
    Ok(())
}

fn simplify_lod_mesh_with_retry(source: &RawMesh) -> Result<(RawMesh, usize), String> {
    let attempts = [
        (LOD_TARGET_TRIANGLE_RATIO, LOD_SIMPLIFY_ERROR),
        (0.20, 0.035),
        (0.32, 0.02),
        (0.48, 0.01),
        (0.65, 0.004),
    ];
    let mut failures = Vec::new();
    for (attempt_index, (triangle_ratio, error)) in attempts.into_iter().enumerate() {
        match simplify_lod_mesh_once(source, triangle_ratio, error) {
            Ok(candidate) => match validate_lod_candidate(source, &candidate) {
                Ok(()) => return Ok((candidate, attempt_index + 1)),
                Err(reason) => failures.push(format!(
                    "{:.0}% target at {:.3} error: {reason}",
                    triangle_ratio * 100.0,
                    error
                )),
            },
            Err(reason) => failures.push(format!(
                "{:.0}% target at {:.3} error: {reason}",
                triangle_ratio * 100.0,
                error
            )),
        }
    }
    Err(format!(
        "LOD validation rejected every simplification attempt: {}",
        failures.join("; ")
    ))
}

#[cfg(test)]
fn simplify_lod_mesh(source: &RawMesh) -> Result<RawMesh, String> {
    simplify_lod_mesh_with_retry(source).map(|(raw, _)| raw)
}

fn lower_power_of_two(value: u32) -> u32 {
    if value <= 4 {
        return value.max(1);
    }
    1 << (31 - value.leading_zeros())
}

fn lod_texture_size(width: u32, height: u32) -> (u32, u32) {
    let scale = (LOD_MAX_TEXTURE_DIMENSION as f32 / width.max(1) as f32)
        .min(LOD_MAX_TEXTURE_DIMENSION as f32 / height.max(1) as f32)
        .min(1.0);
    let width = lower_power_of_two(((width as f32 * scale).floor() as u32).max(1));
    let height = lower_power_of_two(((height as f32 * scale).floor() as u32).max(1));
    (width, height)
}

/// Power-of-two size for a standalone tiled texture.
fn lod_tiled_texture_size(width: u32, height: u32) -> (u32, u32) {
    let scale = (LOD_TILED_TEXTURE_DIMENSION as f32 / width.max(1) as f32)
        .min(LOD_TILED_TEXTURE_DIMENSION as f32 / height.max(1) as f32)
        .min(1.0);
    let width = lower_power_of_two(((width as f32 * scale).floor() as u32).max(1));
    let height = lower_power_of_two(((height as f32 * scale).floor() as u32).max(1));
    (width, height)
}

fn resize_lod_texture_to(
    texture: &DecodedLodTexture,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, String> {
    if (texture.width, texture.height) == (width, height) {
        return Ok(texture.rgba.clone());
    }
    let image = image::RgbaImage::from_raw(texture.width, texture.height, texture.rgba.clone())
        .ok_or_else(|| format!("Could not decode texture '{}' pixels", texture.name))?;
    Ok(
        image::imageops::resize(&image, width, height, image::imageops::FilterType::Triangle)
            .into_raw(),
    )
}

fn resize_lod_texture(
    name: String,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
) -> Result<DecodedLodTexture, String> {
    let (target_width, target_height) = lod_texture_size(width, height);
    let rgba = if target_width == width && target_height == height {
        rgba
    } else {
        let image = image::RgbaImage::from_raw(width, height, rgba)
            .ok_or_else(|| format!("Could not decode texture '{name}' pixels"))?;
        image::imageops::resize(
            &image,
            target_width,
            target_height,
            image::imageops::FilterType::Triangle,
        )
        .into_raw()
    };
    Ok(DecodedLodTexture {
        name,
        width: target_width,
        height: target_height,
        rgba,
    })
}

fn referenced_texture_names(raw: &RawMesh) -> Vec<String> {
    let used_materials = raw
        .triangles
        .iter()
        .map(|triangle| triangle.material as usize)
        .collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    let mut names = Vec::new();
    for material in used_materials {
        let Some(name) = raw.material_textures.get(material) else {
            continue;
        };
        let trimmed = name.trim();
        if !trimmed.is_empty() && seen.insert(lower(trimmed)) {
            names.push(trimmed.to_string());
        }
        if let Some(animation) = raw.material_animations.get(material) {
            for name in &animation.names {
                let trimmed = name.trim();
                if !trimmed.is_empty() && seen.insert(lower(trimmed)) {
                    names.push(trimmed.to_string());
                }
            }
        }
    }
    names
}

fn decode_lod_textures(
    raw: &RawMesh,
    txd_name: &str,
    txd_bytes: &[u8],
) -> Result<Vec<DecodedLodTexture>, String> {
    let names = referenced_texture_names(raw);
    if names.is_empty() {
        return Ok(Vec::new());
    }
    let mut index = TxdTextureIndex::new();
    let txd_key = asset_key(txd_name, ".txd");
    index_one_txd(
        txd_bytes,
        0,
        txd_bytes.len(),
        Path::new("<lod-source>"),
        &txd_key,
        &mut index,
    );
    let mut textures = Vec::with_capacity(names.len());
    for name in names {
        let native = index
            .get(&lower(&name))
            .and_then(|entries| {
                entries
                    .iter()
                    .find(|entry| entry.txd_name.eq_ignore_ascii_case(&txd_key))
                    .or_else(|| entries.first())
            })
            .ok_or_else(|| format!("Texture '{name}' was not found in {txd_key}"))?;
        let (width, height, rgba) = decode_txd_texture_from_bytes(native, txd_bytes)
            .ok_or_else(|| format!("Texture '{name}' in {txd_key} could not be decoded"))?;
        textures.push(resize_lod_texture(name, width, height, rgba)?);
    }
    Ok(textures)
}

fn atlas_is_possible(raw: &RawMesh, textures: &[DecodedLodTexture]) -> bool {
    let available = textures
        .iter()
        .map(|texture| lower(&texture.name))
        .collect::<BTreeSet<_>>();
    !textures.is_empty()
        && raw.uvs.len() == raw.vertices.len()
        && raw.triangles.iter().all(|triangle| {
            let texture = raw
                .material_textures
                .get(triangle.material as usize)
                .map(|name| name.trim())
                .unwrap_or_default();
            texture.is_empty() || available.contains(&lower(texture))
        })
}

fn build_texture_txd(
    textures: &[DecodedLodTexture],
    mipped: &BTreeSet<String>,
) -> Result<Vec<u8>, String> {
    let mut txd = rw_chunk(0x16, rw_chunk(0x01, vec![0, 0, 0, 0]));
    for texture in textures {
        // Atlas mip levels need per-region filtering to avoid bleeding across
        // cells. Keep the generated atlas to its top level; independent
        // textures can safely receive a complete mip chain.
        let native = if !mipped.contains(&lower(&texture.name)) {
            texture_native_from_rgba(
                &texture.rgba,
                texture.width as u16,
                texture.height as u16,
                &texture.name,
            )
        } else {
            texture_native_from_rgba_with_full_mips(
                &texture.rgba,
                texture.width as u16,
                texture.height as u16,
                &texture.name,
            )
        };
        txd = append_texture_native_to_txd(txd, &native, &texture.name)?;
    }
    Ok(txd)
}

#[derive(Clone, Copy, Debug)]
struct LodAtlasCrop {
    u_min: f32,
    v_min: f32,
    u_span: f32,
    v_span: f32,
    texel_width: f32,
    texel_height: f32,
}

fn lod_atlas_crop(
    texture: &DecodedLodTexture,
    bounds: Option<(f32, f32, f32, f32)>,
) -> LodAtlasCrop {
    let (u_min, v_min, u_max, v_max) = bounds.unwrap_or((0.0, 0.0, 1.0, 1.0));
    // A zero-width UV footprint can still be sampled by filtering. Reserve one
    // source texel instead of expanding it to a complete texture tile.
    let u_span = (u_max - u_min).max(1.0 / texture.width.max(1) as f32);
    let v_span = (v_max - v_min).max(1.0 / texture.height.max(1) as f32);
    LodAtlasCrop {
        u_min,
        v_min,
        u_span,
        v_span,
        texel_width: (texture.width as f32 * u_span).max(1.0),
        texel_height: (texture.height as f32 * v_span).max(1.0),
    }
}

/// Whether a material repeats its texture too often to be worth baking into an
/// atlas cell. Such a material keeps its own wrapped texture: squeezing it into
/// a cell would either duplicate the same pixels many times over or stretch the
/// tiling across the surface.
fn lod_texture_is_tiled(bounds: Option<(f32, f32, f32, f32)>) -> bool {
    let Some((u_min, v_min, u_max, v_max)) = bounds else {
        return false;
    };
    u_max - u_min > LOD_ATLAS_MAX_BAKED_REPEATS || v_max - v_min > LOD_ATLAS_MAX_BAKED_REPEATS
}

/// Name a generated texture after its own contents.
///
/// Neighbouring instances of the same building generate the same sheet, and a
/// tiled road texture is shared by every LOD that touches the road. Naming by
/// content makes those collapse to a single entry in the shared dictionary
/// instead of being stored once per generated LOD.
fn lod_texture_content_name(seed: &str, width: u32, height: u32, rgba: &[u8]) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in width
        .to_le_bytes()
        .iter()
        .chain(height.to_le_bytes().iter())
        .chain(rgba)
    {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    let mut seed = sanitize_texture_name(seed);
    seed.truncate(GTA_SA_TEXTURE_NAME_MAX - 17);
    format!("{seed}_{hash:016x}")
}

/// Shelf-pack the cropped footprints at one shared scale. Every crop receives
/// the same scale, preserving relative texel density across materials.
fn try_pack_lod_atlas_crops(
    crops: &[LodAtlasCrop],
    atlas_width: u32,
    atlas_height: u32,
    gutter: u32,
    scale: f32,
) -> Option<Vec<(u32, u32, u32, u32)>> {
    let available_width = atlas_width.saturating_sub(gutter * 2);
    let available_height = atlas_height.saturating_sub(gutter * 2);
    if crops.is_empty() || available_width == 0 || available_height == 0 {
        return None;
    }
    // A crop that no longer fits must fail the attempt. Clamping it to the
    // sheet instead would silently squash its aspect ratio, which is how a
    // single heavily tiled material used to distort a whole atlas.
    let mut rectangles = Vec::with_capacity(crops.len());
    for (index, crop) in crops.iter().enumerate() {
        let width = (crop.texel_width * scale).round().max(1.0);
        let height = (crop.texel_height * scale).round().max(1.0);
        if width > available_width as f32 || height > available_height as f32 {
            return None;
        }
        rectangles.push((index, width as u32, height as u32));
    }
    rectangles.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| b.1.cmp(&a.1)));
    let mut shelves = Vec::<(u32, u32, u32)>::new(); // y, height, next x
    let mut placements = vec![(0, 0, 0, 0); crops.len()];
    for (index, width, height) in rectangles {
        let outer_width = width + gutter * 2;
        let outer_height = height + gutter * 2;
        let mut placed = None;
        for shelf in &mut shelves {
            if outer_height <= shelf.1 && shelf.2 + outer_width <= atlas_width {
                placed = Some((shelf.2 + gutter, shelf.0 + gutter));
                shelf.2 += outer_width;
                break;
            }
        }
        if placed.is_none() {
            let y = shelves.iter().map(|shelf| shelf.1).sum::<u32>();
            if outer_width > atlas_width || y + outer_height > atlas_height {
                return None;
            }
            shelves.push((y, outer_height, outer_width));
            placed = Some((gutter, y + gutter));
        }
        let (x, y) = placed.expect("a fitting atlas crop has a placement");
        placements[index] = (x, y, width, height);
    }
    Some(placements)
}

/// Largest scale up to `max_scale` that still packs, with its placements.
fn pack_lod_atlas_crops_at_best_scale(
    crops: &[LodAtlasCrop],
    atlas_width: u32,
    atlas_height: u32,
    gutter: u32,
    max_scale: f32,
) -> Option<(f32, Vec<(u32, u32, u32, u32)>)> {
    let pack = |scale| try_pack_lod_atlas_crops(crops, atlas_width, atlas_height, gutter, scale);
    if let Some(placements) = pack(max_scale) {
        return Some((max_scale, placements));
    }
    pack(0.0)?;
    let mut low = 0.0f32;
    let mut high = max_scale;
    for _ in 0..24 {
        let middle = (low + high) * 0.5;
        if pack(middle).is_some() {
            low = middle;
        } else {
            high = middle;
        }
    }
    Some((low, pack(low)?))
}

/// Pick the smallest power-of-two sheet that holds every crop at (close to)
/// its native texel density.
///
/// The generator used to emit a 512x512 sheet for every LOD regardless of how
/// little of it was covered, which upscaled small source textures into an
/// otherwise empty page and cost 128-256 KB per LOD in the shared dictionary.
/// Sizing the sheet to its contents keeps the same on-screen detail in a
/// fraction of the bytes.
fn choose_lod_atlas_size(
    crops: &[LodAtlasCrop],
    gutter: u32,
) -> Result<(u32, u32, Vec<(u32, u32, u32, u32)>), String> {
    if crops.is_empty() {
        return Err("LOD atlas has no usable crop area".to_string());
    }
    let mut sizes = Vec::new();
    let mut width = LOD_MIN_ATLAS_DIMENSION;
    while width <= LOD_MAX_ATLAS_DIMENSION {
        let mut height = LOD_MIN_ATLAS_DIMENSION;
        while height <= LOD_MAX_ATLAS_DIMENSION {
            sizes.push((width, height));
            height *= 2;
        }
        width *= 2;
    }
    // Smallest sheet first, then the squarest of equal-area candidates.
    sizes.sort_by_key(|(width, height)| (width * height, width.abs_diff(*height)));
    for (width, height) in sizes {
        if let Some((scale, placements)) =
            pack_lod_atlas_crops_at_best_scale(crops, width, height, gutter, 1.0)
            && scale >= LOD_ATLAS_MIN_SCALE
        {
            return Ok((width, height, placements));
        }
    }
    // Nothing fits at a usable density even on the largest sheet: fall back to
    // the densest packing the maximum size allows.
    let (_, placements) = pack_lod_atlas_crops_at_best_scale(
        crops,
        LOD_MAX_ATLAS_DIMENSION,
        LOD_MAX_ATLAS_DIMENSION,
        gutter,
        1.0,
    )
    .ok_or_else(|| "There are too many cropped textures to fit in a LOD atlas".to_string())?;
    Ok((LOD_MAX_ATLAS_DIMENSION, LOD_MAX_ATLAS_DIMENSION, placements))
}

/// Textures generated for one LOD: packed atlas sheets plus any material that
/// stayed standalone because it tiles.
struct LodAtlasSet {
    textures: Vec<DecodedLodTexture>,
    /// Lowercased names of the standalone tiled textures in `textures`. They
    /// are sampled with wrapping, so unlike an atlas sheet they want mip maps.
    tiled: BTreeSet<String>,
}

fn remap_raw_for_atlases(
    raw: &mut RawMesh,
    textures: &[DecodedLodTexture],
) -> Result<LodAtlasSet, String> {
    const GUTTER: u32 = 2;

    #[derive(Clone, Copy)]
    struct AtlasRegion {
        atlas: usize,
        atlas_width: u32,
        atlas_height: u32,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        u_min: f32,
        v_min: f32,
        u_span: f32,
        v_span: f32,
    }

    let mut uv_bounds = HashMap::<String, (f32, f32, f32, f32)>::new();
    for triangle in &raw.triangles {
        let texture_name = raw
            .material_textures
            .get(triangle.material as usize)
            .map(|name| lower(name.trim()))
            .unwrap_or_default();
        if texture_name.is_empty() {
            continue;
        }
        for index in [triangle.a, triangle.b, triangle.c] {
            let Some(uv) = raw.uvs.get(index as usize) else {
                continue;
            };
            if !uv.u.is_finite() || !uv.v.is_finite() {
                continue;
            }
            let bounds = uv_bounds
                .entry(texture_name.clone())
                .or_insert((uv.u, uv.v, uv.u, uv.v));
            bounds.0 = bounds.0.min(uv.u);
            bounds.1 = bounds.1.min(uv.v);
            bounds.2 = bounds.2.max(uv.u);
            bounds.3 = bounds.3.max(uv.v);
        }
    }

    // A material that repeats its texture many times cannot be atlased: baking
    // every repeat wastes the sheet, and baking fewer stretches the tiling. It
    // keeps its own texture and its original wrapped UVs instead.
    let mut generated = Vec::new();
    let mut tiled_names = BTreeSet::new();
    let mut standalone = HashMap::<String, String>::new();
    for texture in textures {
        if !lod_texture_is_tiled(uv_bounds.get(&lower(&texture.name)).copied()) {
            continue;
        }
        let (width, height) = lod_tiled_texture_size(texture.width, texture.height);
        let resized = resize_lod_texture_to(texture, width, height)?;
        let name = lod_texture_content_name(&texture.name, width, height, &resized);
        standalone.insert(lower(&texture.name), name.clone());
        tiled_names.insert(lower(&name));
        generated.push(DecodedLodTexture {
            name,
            width,
            height,
            rgba: resized,
        });
    }

    let atlasable = textures
        .iter()
        .filter(|texture| !standalone.contains_key(&lower(&texture.name)))
        .collect::<Vec<_>>();
    let opaque = atlasable
        .iter()
        .copied()
        .filter(|texture| !rgba_has_alpha(&texture.rgba))
        .collect::<Vec<_>>();
    let alpha = atlasable
        .iter()
        .copied()
        .filter(|texture| rgba_has_alpha(&texture.rgba))
        .collect::<Vec<_>>();
    // Opaque is intentionally emitted first. The normalized DFF writer keeps
    // each material's first-occurrence order for BinMesh batches, so this also
    // makes opaque geometry draw before alpha geometry in-game.
    let groups = [opaque, alpha];
    let mut atlases = Vec::new();
    let mut regions = HashMap::<String, AtlasRegion>::new();
    for group in groups {
        if group.is_empty() {
            continue;
        }
        let atlas_index = atlases.len();
        // Seed the content hash with a source texture name so the dictionary
        // stays browsable instead of listing bare hashes.
        let seed_name = group[0].name.clone();
        let crops = group
            .iter()
            .map(|texture| lod_atlas_crop(texture, uv_bounds.get(&lower(&texture.name)).copied()))
            .collect::<Vec<_>>();
        let (atlas_width, atlas_height, placements) = choose_lod_atlas_size(&crops, GUTTER)?;
        // Padding is opaque even for the alpha atlas. This prevents filtering
        // outside an assigned region from introducing a translucent fringe.
        let mut atlas = [0u8, 0, 0, 255].repeat((atlas_width * atlas_height) as usize);
        for ((texture, crop), (x, y, region_width, region_height)) in
            group.into_iter().zip(crops).zip(placements)
        {
            let u_min = crop.u_min;
            let v_min = crop.v_min;
            let u_span = crop.u_span;
            let v_span = crop.v_span;
            for gutter_y in 0..region_height + GUTTER * 2 {
                let local_y = gutter_y as i64 - GUTTER as i64;
                let domain_v = v_min + (local_y as f32 + 0.5) / region_height as f32 * v_span;
                let source_y = (domain_v.rem_euclid(1.0) * texture.height as f32)
                    .floor()
                    .min(texture.height.saturating_sub(1) as f32)
                    as u32;
                for gutter_x in 0..region_width + GUTTER * 2 {
                    let local_x = gutter_x as i64 - GUTTER as i64;
                    let domain_u = u_min + (local_x as f32 + 0.5) / region_width as f32 * u_span;
                    let source_x = (domain_u.rem_euclid(1.0) * texture.width as f32)
                        .floor()
                        .min(texture.width.saturating_sub(1) as f32)
                        as u32;
                    let source = ((source_y * texture.width + source_x) * 4) as usize;
                    let destination = (((y - GUTTER + gutter_y) * atlas_width + x - GUTTER
                        + gutter_x)
                        * 4) as usize;
                    atlas[destination..destination + 4]
                        .copy_from_slice(&texture.rgba[source..source + 4]);
                }
            }
            regions.insert(
                lower(&texture.name),
                AtlasRegion {
                    atlas: atlas_index,
                    atlas_width,
                    atlas_height,
                    x,
                    y,
                    width: region_width,
                    height: region_height,
                    u_min,
                    v_min,
                    u_span,
                    v_span,
                },
            );
        }
        atlases.push(DecodedLodTexture {
            name: lod_texture_content_name(&seed_name, atlas_width, atlas_height, &atlas),
            width: atlas_width,
            height: atlas_height,
            rgba: atlas,
        });
    }

    let source = raw.clone();
    let material_uv_scales = source
        .triangles
        .iter()
        .filter_map(|triangle| {
            let texture_name = source.material_textures.get(triangle.material as usize)?;
            let region = regions.get(&lower(texture_name.trim()))?;
            Some((
                triangle.material,
                (
                    region.width as f64 / region.atlas_width as f64 / region.u_span as f64,
                    region.height as f64 / region.atlas_height as f64 / region.v_span as f64,
                ),
            ))
        })
        .collect::<HashMap<_, _>>();
    let has_normals = source.normals.len() == source.vertices.len();
    let has_prelit = source.prelit_colors.len() == source.vertices.len();
    let has_prelit_alpha = source.prelit_alphas.len() == source.vertices.len();
    let has_night = source.night_prelit_colors.len() == source.vertices.len();
    let has_night_alpha = source.night_prelit_alphas.len() == source.vertices.len();
    let secondary_uv_sets = source
        .secondary_uvs
        .iter()
        .filter(|uvs| uvs.len() == source.vertices.len())
        .collect::<Vec<_>>();
    let has_flags = source.light_flags.len() == source.vertices.len();
    let mut remap = BTreeMap::<(u32, u16), u32>::new();
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut secondary_uvs = vec![Vec::new(); secondary_uv_sets.len()];
    let mut prelit = Vec::new();
    let mut prelit_alphas = Vec::new();
    let mut night = Vec::new();
    let mut night_alphas = Vec::new();
    let mut flags = Vec::new();
    let mut triangles = Vec::with_capacity(source.triangles.len());
    for triangle in &source.triangles {
        let texture_name = source
            .material_textures
            .get(triangle.material as usize)
            .map(|name| name.trim())
            .unwrap_or_default();
        // Standalone tiled materials keep their wrapped UVs untouched; only
        // atlased materials are folded into a cell.
        let region = if texture_name.is_empty() || standalone.contains_key(&lower(texture_name)) {
            None
        } else {
            Some(
                regions
                    .get(&lower(texture_name))
                    .copied()
                    .ok_or_else(|| format!("Texture '{texture_name}' was not decoded"))?,
            )
        };
        let mut mapped = [0u32; 3];
        for (corner, source_index) in [triangle.a, triangle.b, triangle.c].into_iter().enumerate() {
            mapped[corner] =
                if let Some(index) = remap.get(&(source_index, triangle.material)).copied() {
                    index
                } else {
                    let index = vertices.len() as u32;
                    remap.insert((source_index, triangle.material), index);
                    let source_index = source_index as usize;
                    vertices.push(source.vertices[source_index]);
                    if has_normals {
                        normals.push(source.normals[source_index]);
                    }
                    let uv = source.uvs[source_index];
                    uvs.push(if let Some(region) = region {
                        V2 {
                            u: (region.x as f32
                                + (uv.u - region.u_min) / region.u_span * region.width as f32)
                                / region.atlas_width as f32,
                            v: (region.y as f32
                                + (uv.v - region.v_min) / region.v_span * region.height as f32)
                                / region.atlas_height as f32,
                        }
                    } else {
                        uv
                    });
                    for (out, set) in secondary_uvs.iter_mut().zip(&secondary_uv_sets) {
                        out.push(set[source_index]);
                    }
                    if has_prelit {
                        prelit.push(source.prelit_colors[source_index]);
                    }
                    if has_prelit_alpha {
                        prelit_alphas.push(source.prelit_alphas[source_index]);
                    }
                    if has_night {
                        night.push(source.night_prelit_colors[source_index]);
                    }
                    if has_night_alpha {
                        night_alphas.push(source.night_prelit_alphas[source_index]);
                    }
                    if has_flags {
                        flags.push(source.light_flags[source_index]);
                    }
                    index
                };
        }
        triangles.push(Tri {
            a: mapped[0],
            b: mapped[1],
            c: mapped[2],
            material: triangle.material,
        });
    }
    raw.vertices = vertices;
    raw.normals = normals;
    raw.uvs = uvs;
    raw.secondary_uvs = secondary_uvs;
    raw.prelit_colors = prelit;
    raw.prelit_alphas = prelit_alphas;
    raw.night_prelit_colors = night;
    raw.night_prelit_alphas = night_alphas;
    raw.light_flags = flags;
    raw.triangles = triangles;
    raw.material_textures = source.material_textures.clone();
    for material in 0..raw.material_textures.len() {
        let texture = lower(raw.material_textures[material].trim());
        if let Some(region) = regions.get(&texture) {
            raw.material_textures[material] = atlases[region.atlas].name.clone();
        } else if let Some(name) = standalone.get(&texture) {
            raw.material_textures[material] = name.clone();
        }
    }
    raw.material_animations.clear();
    raw.uv_anim_dictionaries.clear();
    raw.uv_animations.clear();
    generated.extend(atlases);
    validate_atlas_layout(&source, raw, &generated, &material_uv_scales)?;

    let alpha_atlas_names = generated
        .iter()
        .filter(|texture| rgba_has_alpha(&texture.rgba))
        .map(|texture| lower(&texture.name))
        .collect::<BTreeSet<_>>();
    let alpha_materials = raw
        .material_textures
        .iter()
        .enumerate()
        .filter_map(|(index, texture)| {
            let material_alpha = raw
                .materials
                .get(index)
                .map_or(1.0, |material| material.alpha);
            (material_alpha < 0.996 || alpha_atlas_names.contains(&lower(texture.trim())))
                .then_some(index as u16)
        })
        .collect::<BTreeSet<_>>();
    raw.triangles
        .sort_by_key(|triangle| alpha_materials.contains(&triangle.material));
    Ok(LodAtlasSet {
        textures: generated,
        tiled: tiled_names,
    })
}

fn validate_atlas_layout(
    source: &RawMesh,
    atlased: &RawMesh,
    atlases: &[DecodedLodTexture],
    material_uv_scales: &HashMap<u16, (f64, f64)>,
) -> Result<(), String> {
    for atlas in atlases {
        if atlas.width > LOD_MAX_ATLAS_DIMENSION || atlas.height > LOD_MAX_ATLAS_DIMENSION {
            return Err(format!(
                "Generated atlas is {}x{}, above the {}x{} limit",
                atlas.width, atlas.height, LOD_MAX_ATLAS_DIMENSION, LOD_MAX_ATLAS_DIMENSION
            ));
        }
        if atlas.rgba.len() != (atlas.width * atlas.height * 4) as usize {
            return Err("Generated atlas pixel buffer has an invalid size".to_string());
        }
    }
    if source.triangles.len() != atlased.triangles.len()
        || source.vertices.is_empty()
        || atlased.uvs.len() != atlased.vertices.len()
    {
        return Err("Atlas remapping lost polygons or vertex UVs".to_string());
    }
    if atlased
        .uvs
        .iter()
        .any(|uv| !uv.u.is_finite() || !uv.v.is_finite())
    {
        return Err("Atlas remapping produced missing UVs".to_string());
    }
    for (triangle_index, (source_triangle, atlas_triangle)) in
        source.triangles.iter().zip(&atlased.triangles).enumerate()
    {
        let source_uv = [
            source.uvs[source_triangle.a as usize],
            source.uvs[source_triangle.b as usize],
            source.uvs[source_triangle.c as usize],
        ];
        let atlas_uv = [
            atlased.uvs[atlas_triangle.a as usize],
            atlased.uvs[atlas_triangle.b as usize],
            atlased.uvs[atlas_triangle.c as usize],
        ];
        // A material left out of the atlas keeps its own wrapped texture, so
        // its UVs must come through untouched, tiling and all.
        if !material_uv_scales.contains_key(&source_triangle.material) {
            if atlas_uv != source_uv {
                return Err(format!(
                    "Standalone tiled UV polygon {triangle_index} was rewritten (material {})",
                    source_triangle.material
                ));
            }
            continue;
        }
        if atlas_uv
            .iter()
            .any(|uv| !(-1e-5..=1.00001).contains(&uv.u) || !(-1e-5..=1.00001).contains(&uv.v))
        {
            return Err(format!(
                "Atlas remapping placed UV polygon {triangle_index} outside its sheet (material {})",
                source_triangle.material
            ));
        }
        let signed_area = |uv: [V2; 3]| {
            let ab_u = uv[1].u as f64 - uv[0].u as f64;
            let ab_v = uv[1].v as f64 - uv[0].v as f64;
            let ac_u = uv[2].u as f64 - uv[0].u as f64;
            let ac_v = uv[2].v as f64 - uv[0].v as f64;
            ab_u * ac_v - ab_v * ac_u
        };
        let source_area = signed_area(source_uv);
        let atlas_area = signed_area(atlas_uv);
        let (scale_u, scale_v) = material_uv_scales
            .get(&source_triangle.material)
            .copied()
            .unwrap_or((1.0, 1.0));
        let expected_atlas_area = source_area.abs() * scale_u.abs() * scale_v.abs();
        let max_atlas_edge = [(0usize, 1usize), (1, 2), (2, 0)]
            .into_iter()
            .flat_map(|(a, b)| {
                [
                    (atlas_uv[b].u as f64 - atlas_uv[a].u as f64).abs(),
                    (atlas_uv[b].v as f64 - atlas_uv[a].v as f64).abs(),
                ]
            })
            .fold(0.0f64, f64::max);
        // Atlas coordinates are serialized as f32. A valid source UV sliver
        // can become narrower than one representable atlas coordinate after
        // scaling into its region. Only enforce area and winding when the
        // expected transformed area is comfortably above that rounding floor.
        let precision_floor =
            8.0 * f32::EPSILON as f64 * max_atlas_edge + 32.0 * (f32::EPSILON as f64).powi(2);
        let transform_is_reliable = expected_atlas_area > precision_floor * 8.0;
        if transform_is_reliable
            && (!atlas_area.is_finite()
                || atlas_area.abs() <= (expected_atlas_area * 0.01).max(precision_floor)
                || source_area.signum() != atlas_area.signum())
        {
            return Err(format!(
                "Atlas remapping collapsed, mirrored, or rotated UV polygon {triangle_index} incorrectly (material {}, source area {source_area:e}, atlas area {atlas_area:e}, source UVs {source_uv:?}, atlas UVs {atlas_uv:?})",
                source_triangle.material,
            ));
        }
    }
    let (source_open, source_non_manifold, _) = lod_edge_topology(source);
    let (atlas_open, atlas_non_manifold, _) = lod_edge_topology(atlased);
    if source_open != atlas_open || source_non_manifold != atlas_non_manifold {
        return Err("Atlas remapping changed the final mesh topology".to_string());
    }
    Ok(())
}

fn build_lod_bounds_collision(
    raw: &RawMesh,
    stem: &str,
) -> Result<(String, Vec<u8>, CollisionMesh), String> {
    let col_name = format!("{stem}.col");
    let bounds = bounds_from_vertices(&raw.vertices);
    let mesh = CollisionMesh {
        name: stem.to_string(),
        spheres: Vec::new(),
        boxes: Vec::new(),
        vertices: Vec::new(),
        faces: Vec::new(),
        bounds,
        shadow_vertices: Vec::new(),
        shadow_faces: Vec::new(),
    };
    let template = col_regeneration_template(&[], &col_name);
    let mut bytes = write_col_mesh_from_template_with_bounds(&template, &mesh, Some(bounds))
        .map_err(|error| format!("Could not serialize LOD bounds COL: {error}"))?;
    set_col_model_names_from_entry(&mut bytes, &col_name);
    let issues = validate_col_for_game_load(&col_name, &bytes);
    if col_validation_has_errors(&issues) {
        return Err("Generated LOD bounds COL failed game-load validation".to_string());
    }
    Ok((col_name, bytes, mesh))
}

fn generate_lod(request: LodGenerationRequest) -> Result<LodGenerationResult, String> {
    let mut request = request;
    if !request.stem_preallocated {
        let base_stem = request.output_stem.clone();
        for suffix in 1usize.. {
            let tail = if suffix == 1 {
                String::new()
            } else {
                format!("_{suffix}")
            };
            let keep =
                (IMG_RUNTIME_SAFE_ENTRY_NAME_BYTES - ".dff".len()).saturating_sub(tail.len());
            let candidate = format!("{}{}", &base_stem[..base_stem.len().min(keep)], tail);
            if !request.reserved_stems.contains(&lower(&candidate))
                && find_dff_entry(&request.project_root, &candidate).is_none()
                && replacement_img_entry(&request.wip_root, &format!("{candidate}.dff")).is_none()
            {
                request.output_stem = candidate;
                break;
            }
        }
    }
    let source = read_mesh_source(request.mesh_source)?;
    let source_triangles = source.triangles.len();
    let source_vertices = source.vertices.len();
    let source_had_textures = !referenced_texture_names(&source).is_empty();
    let (mut raw, simplification_attempts) = simplify_lod_mesh_with_retry(&source)?;
    let mut texture_count = 0usize;
    let mut atlased = false;
    let mut txd_bytes = None;
    if let (Some(txd_source), Some(source_txd_name)) =
        (request.txd_source, request.source_txd_name.as_deref())
    {
        let source_bytes = read_txd_source(txd_source);
        let mut textures = decode_lod_textures(&raw, source_txd_name, &source_bytes)?;
        if !textures.is_empty() {
            let base_textures = raw
                .triangles
                .iter()
                .filter_map(|triangle| {
                    raw.material_textures
                        .get(triangle.material as usize)
                        .map(|name| lower(name.trim()))
                })
                .filter(|name| !name.is_empty())
                .collect::<BTreeSet<_>>();
            textures.retain(|texture| base_textures.contains(&lower(&texture.name)));
            if !atlas_is_possible(&raw, &textures) {
                return Err(
                    "The referenced LOD textures could not be combined into atlases because the mesh has missing UVs or unresolved material textures"
                        .to_string(),
                );
            }
            let generated = remap_raw_for_atlases(&mut raw, &textures)?;
            atlased = true;
            texture_count = generated.textures.len();
            txd_bytes = Some(build_texture_txd(&generated.textures, &generated.tiled)?);
        }
    }
    let dff_name = format!("{}.dff", request.output_stem);
    let dff_bytes = write_normalized_dff(&raw, &request.output_stem)?;
    let reparsed_dff = parse_dff_mesh(&dff_bytes);
    validate_lod_candidate(&raw, &reparsed_dff)
        .map_err(|reason| format!("Serialized LOD DFF failed final validation: {reason}"))?;
    if let Some(bytes) = txd_bytes.as_deref() {
        let contents = parse_txd_texture_contents(bytes)
            .map_err(|reason| format!("Generated LOD TXD failed final validation: {reason}"))?;
        if contents.len() != texture_count
            || contents.iter().any(|texture| {
                texture.width as u32 > LOD_MAX_ATLAS_DIMENSION
                    || texture.height as u32 > LOD_MAX_ATLAS_DIMENSION
            })
        {
            return Err(format!(
                "Generated LOD TXD did not round-trip {texture_count} texture(s) of at most {LOD_MAX_ATLAS_DIMENSION}x{LOD_MAX_ATLAS_DIMENSION}"
            ));
        }
    }
    let (col_name, col_bytes, col_mesh) = build_lod_bounds_collision(&raw, &request.output_stem)?;
    Ok(LodGenerationResult {
        stem: request.output_stem,
        dff_name,
        dff_bytes,
        col_name,
        col_bytes,
        col_mesh,
        txd_name: txd_bytes.as_ref().map(|_| WORLD_LOD_TXD_NAME.to_string()),
        txd_bytes,
        output_triangles: raw.triangles.len(),
        output_vertices: raw.vertices.len(),
        simplification_attempts,
        raw,
        attach_to_placement: request.attach_to_placement,
        source_definition_id: request.source_definition_id,
        source_zone: request.source_zone,
        source_dff_name: request.source_dff_name,
        source_txd_name: request.source_txd_name,
        replaced_lod_parent: request.replaced_lod_parent,
        replaced_lod_index: request.replaced_lod_index,
        source_triangles,
        source_vertices,
        texture_count,
        source_had_textures,
        atlased,
    })
}

pub(crate) fn writer_is_busy(app: &AppState) -> bool {
    app.manual_save_job.is_some()
        || app.editing.save_rx.is_some()
        || app.lod_generation_job.is_some()
        || app.instance_lod_removal_job.is_some()
        || app.autosave_rx.is_some()
        || app.autosave_cleanup_rx.is_some()
        || app.txd_cleanup_job.is_some()
        || app.asset_optimization_scan_rx.is_some()
        || app.asset_optimization_job.is_some()
        || app.object_bounds_fix_job.is_some()
        || app.dff_repair_rx.is_some()
        || app.corona_generation_job.is_some()
        || app.collision_generation_job.is_some()
        || app.shadow_mesh_generation_job.is_some()
        || app.bake_job.is_some()
}

fn lod_asset_stem(source_name: &str) -> String {
    let source = Path::new(source_name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("model");
    let cleaned: String = source
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '_')
        .collect();
    let mut base = format!("lod_{}", cleaned.trim_matches('_'));
    if base == "lod_" {
        base = "lod_model".to_string();
    }
    base.truncate(IMG_RUNTIME_SAFE_ENTRY_NAME_BYTES - ".dff".len());
    base
}

fn reserved_lod_stems(app: &AppState) -> BTreeSet<String> {
    let mut stems = app
        .definitions
        .keys()
        .map(|name| lower(name))
        .collect::<BTreeSet<_>>();
    stems.extend(app.pending_replacement_assets.keys().filter_map(|name| {
        Path::new(name)
            .file_stem()
            .and_then(|value| value.to_str())
            .map(lower)
    }));
    stems
}

fn indexed_lod_sources(app: &AppState) -> (BTreeMap<String, ImgEntry>, BTreeMap<String, ImgEntry>) {
    let mut dffs = collect_resource_dff_entries(&app.root);
    let mut txds = collect_resource_txd_entries(&app.root);
    for path in gta_sa_img_files(&app.gta_sa_dir) {
        for entry in parse_img(&path) {
            let name = lower(&entry.name);
            if name.ends_with(".dff") {
                dffs.entry(asset_key(&name, ".dff")).or_insert(entry);
            } else if name.ends_with(".txd") {
                txds.entry(asset_key(&name, ".txd")).or_insert(entry);
            }
        }
    }
    (dffs, txds)
}

fn dff_source_from_index(
    app: &AppState,
    dff_name: &str,
    entries: &BTreeMap<String, ImgEntry>,
) -> Option<LodMeshSource> {
    let key = asset_key(dff_name, ".dff");
    app.editing
        .modified_entries
        .get(&key)
        .map(|bytes| LodMeshSource::Bytes(Arc::new(bytes.clone())))
        .or_else(|| {
            app.pending_replacement_assets
                .get(&key)
                .map(|(_, bytes)| LodMeshSource::Bytes(Arc::new(bytes.clone())))
        })
        .or_else(|| entries.get(&key).cloned().map(LodMeshSource::Entry))
}

fn txd_source_from_index(
    app: &AppState,
    txd_name: &str,
    entries: &BTreeMap<String, ImgEntry>,
) -> Option<LodTxdSource> {
    let key = asset_key(txd_name, ".txd");
    app.editing
        .modified_entries
        .get(&key)
        .map(|bytes| LodTxdSource::Bytes(Arc::new(bytes.clone())))
        .or_else(|| {
            app.pending_replacement_assets
                .get(&key)
                .map(|(_, bytes)| LodTxdSource::Bytes(Arc::new(bytes.clone())))
        })
        .or_else(|| entries.get(&key).cloned().map(LodTxdSource::Entry))
}

fn txd_source_for_app(app: &AppState, txd_name: &str) -> Option<LodTxdSource> {
    let key = asset_key(txd_name, ".txd");
    app.editing
        .modified_entries
        .get(&key)
        .cloned()
        .map(|bytes| LodTxdSource::Bytes(Arc::new(bytes)))
        .or_else(|| {
            app.pending_replacement_assets
                .get(&key)
                .map(|(_, bytes)| LodTxdSource::Bytes(Arc::new(bytes.clone())))
        })
        .or_else(|| find_txd_entry_for_app(app, &key).map(LodTxdSource::Entry))
}

fn spawn_lod_worker(order: usize, request: LodGenerationRequest) -> LodGenerationWorker {
    let (tx, rx) = mpsc::channel();
    let source_name = request.source_dff_name.clone();
    let worker_source_name = source_name.clone();
    thread::spawn(move || {
        let result = std::panic::catch_unwind(|| generate_lod(request))
            .map_err(|panic| {
                panic
                    .downcast_ref::<&str>()
                    .map(|message| (*message).to_string())
                    .or_else(|| panic.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "unknown LOD generation panic".to_string())
            })
            .and_then(|result| result)
            .map_err(|error| format!("{source_name}: {error}"));
        let _ = tx.send(result);
    });
    LodGenerationWorker {
        order,
        source_name: worker_source_name,
        rx,
    }
}

fn reserve_batch_lod_stems(requests: &mut [LodGenerationRequest]) {
    let verify_filesystem = requests.len() == 1;
    let mut occupied = requests
        .first()
        .map(|request| (*request.reserved_stems).clone())
        .unwrap_or_default();
    let mut next_suffix = HashMap::<String, usize>::new();
    for request in requests {
        let base_stem = request.output_stem.clone();
        let suffix = next_suffix.entry(lower(&base_stem)).or_insert(1);
        loop {
            let tail = if *suffix == 1 {
                String::new()
            } else {
                format!("_{}", *suffix)
            };
            let keep =
                (IMG_RUNTIME_SAFE_ENTRY_NAME_BYTES - ".dff".len()).saturating_sub(tail.len());
            let candidate = format!("{}{}", &base_stem[..base_stem.len().min(keep)], tail);
            *suffix += 1;
            let available = !occupied.contains(&lower(&candidate))
                && (!verify_filesystem
                    || (find_dff_entry(&request.project_root, &candidate).is_none()
                        && replacement_img_entry(&request.wip_root, &format!("{candidate}.dff"))
                            .is_none()));
            if available {
                request.output_stem = candidate.clone();
                request.stem_preallocated = true;
                occupied.insert(lower(&candidate));
                break;
            }
        }
    }
}

fn fill_lod_worker_slots(job: &mut LodGenerationJob) {
    while job.workers.len() + job.ready.len() + usize::from(job.result.is_some())
        < LOD_GENERATION_MAX_WORKERS
    {
        let Some((order, request)) = job.pending.pop_front() else {
            break;
        };
        job.workers.push(spawn_lod_worker(order, request));
    }
}

fn start_lod_jobs(
    app: &mut AppState,
    mut requests: Vec<LodGenerationRequest>,
    skipped: Vec<String>,
    failures: Vec<String>,
    total: usize,
) {
    let history_before = world_history_snapshot(app);
    let queued = requests.len();
    let skipped_count = skipped.len();
    let failed_count = failures.len();
    if requests.is_empty() {
        app.status_message = if !skipped.is_empty() && failures.is_empty() {
            format!(
                "Skipped {} selected element{} because {} already {} an LOD.",
                skipped.len(),
                if skipped.len() == 1 { "" } else { "s" },
                if skipped.len() == 1 { "it" } else { "they" },
                if skipped.len() == 1 { "has" } else { "have" },
            )
        } else if failures.is_empty() {
            "No selected elements could be prepared for LOD generation.".to_string()
        } else {
            format!(
                "LOD generation could not start: {} skipped with existing LODs; {} failed: {}",
                skipped.len(),
                failures.len(),
                failures.join("; ")
            )
        };
        return;
    }
    reserve_batch_lod_stems(&mut requests);
    let request_count = requests.len();
    let pending = requests.into_iter().enumerate().collect();
    let mut job = LodGenerationJob {
        workers: Vec::new(),
        started_at: Instant::now(),
        result: None,
        apply_phase: 0,
        attached: false,
        pending,
        ready: BTreeMap::new(),
        next_apply_order: 0,
        request_count,
        total,
        processed: failures.len() + skipped.len(),
        successes: 0,
        attached_count: 0,
        skipped,
        failures,
        source_triangles: 0,
        output_triangles: 0,
        history_before,
        history_artifacts: Vec::new(),
        history_txd_before: BTreeMap::new(),
    };
    fill_lod_worker_slots(&mut job);
    let active = job.workers.len();
    app.lod_generation_job = Some(job);
    app.status_message = if total > 1 {
        format!(
            "Generating LODs for {queued} of {total} elements with {active} parallel worker(s); {skipped_count} already had LODs and {failed_count} could not be prepared."
        )
    } else {
        "Generating simplified DFF and texture atlas in background...".to_string()
    };
}

fn start_lod_job(app: &mut AppState, request: LodGenerationRequest) {
    start_lod_jobs(app, vec![request], Vec::new(), Vec::new(), 1);
}

fn existing_lod_for_placement<'a>(
    placements: &'a [Placement],
    states: &[ElementState],
    index: usize,
) -> Option<&'a str> {
    let placement = placements.get(index)?;
    let parent = placement
        .attrs
        .get("lodParent")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())?;
    if parent.eq_ignore_ascii_case("self") {
        return Some(parent);
    }
    placements
        .iter()
        .enumerate()
        .any(|(candidate_index, candidate)| {
            !states
                .get(candidate_index)
                .is_some_and(|state| state.deleted)
                && candidate.id.eq_ignore_ascii_case(parent)
        })
        .then_some(parent)
}

fn live_placement_ids(placements: &[Placement], states: &[ElementState]) -> HashSet<String> {
    placements
        .iter()
        .enumerate()
        .filter(|(index, _)| !states.get(*index).is_some_and(|state| state.deleted))
        .map(|(_, placement)| lower(&placement.id))
        .collect()
}

fn existing_lod_with_live_ids<'a>(
    placements: &'a [Placement],
    index: usize,
    live_ids: &HashSet<String>,
) -> Option<&'a str> {
    let parent = placements.get(index)?.attrs.get("lodParent")?.trim();
    (!parent.is_empty()
        && (parent.eq_ignore_ascii_case("self") || live_ids.contains(&lower(parent))))
    .then_some(parent)
}

fn partition_existing_lod_indices(
    placements: &[Placement],
    states: &[ElementState],
    indices: Vec<usize>,
) -> (Vec<usize>, Vec<(usize, String)>) {
    let live_ids = live_placement_ids(placements, states);
    let mut ready = Vec::new();
    let mut skipped = Vec::new();
    for index in indices {
        if let Some(parent) = existing_lod_with_live_ids(placements, index, &live_ids) {
            skipped.push((index, parent.to_string()));
        } else {
            ready.push(index);
        }
    }
    (ready, skipped)
}

fn all_indices_have_existing_lods(
    placements: &[Placement],
    states: &[ElementState],
    indices: &[usize],
) -> bool {
    let live_ids = live_placement_ids(placements, states);
    !indices.is_empty()
        && indices
            .iter()
            .all(|index| existing_lod_with_live_ids(placements, *index, &live_ids).is_some())
}

pub(crate) fn selected_models_all_have_lods(app: &AppState) -> bool {
    let indices = selected_live_indices_in_selection_order(app);
    all_indices_have_existing_lods(&app.placements, &app.element_states, &indices)
}

fn existing_lod_index_for_parent(
    app: &AppState,
    source_index: usize,
    parent: &str,
) -> Option<usize> {
    if parent.eq_ignore_ascii_case("self") {
        return None;
    }
    app.placements
        .iter()
        .enumerate()
        .find(|(index, placement)| {
            *index != source_index
                && placement.id.eq_ignore_ascii_case(parent)
                && !app
                    .element_states
                    .get(*index)
                    .is_some_and(|state| state.deleted)
        })
        .map(|(index, _)| index)
}

fn lod_parent_is_still_used(
    placements: &[Placement],
    states: &[ElementState],
    parent: &str,
) -> bool {
    placements.iter().enumerate().any(|(index, placement)| {
        !states.get(index).is_some_and(|state| state.deleted)
            && placement
                .attrs
                .get("lodParent")
                .is_some_and(|value| value.trim().eq_ignore_ascii_case(parent))
    })
}

fn placement_lod_size(app: &AppState, index: usize) -> f32 {
    let Some(placement) = app.placements.get(index) else {
        return 0.0;
    };
    let key = placement_mesh_key(placement, &app.definitions);
    let Some(mesh) = app.meshes.get(&key) else {
        return 0.0;
    };
    let extent = mesh.bounds.max - mesh.bounds.min;
    let size = extent.x.abs().max(extent.y.abs()).max(extent.z.abs()) * placement_scale(placement);
    if size.is_finite() { size.max(0.0) } else { 0.0 }
}

pub(crate) fn lod_batch_minimum_size(dialog: &LodBatchDialog) -> Option<f32> {
    let trimmed = dialog.minimum_size.trim();
    if trimmed.is_empty() {
        return Some(0.0);
    }
    trimmed
        .parse::<f32>()
        .ok()
        .filter(|value| value.is_finite() && *value >= 0.0)
}

pub(crate) fn lod_batch_candidate_included(
    candidate: &LodBatchCandidate,
    minimum_size: f32,
    mode: LodBatchMode,
) -> bool {
    candidate.size >= minimum_size
        && (mode == LodBatchMode::RegenerateScene || candidate.existing_lod.is_none())
}

pub(crate) fn lod_batch_included_count(dialog: &LodBatchDialog) -> usize {
    let Some(minimum_size) = lod_batch_minimum_size(dialog) else {
        return 0;
    };
    dialog
        .candidates
        .iter()
        .filter(|candidate| lod_batch_candidate_included(candidate, minimum_size, dialog.mode))
        .count()
}

fn default_lod_batch_minimum_size() -> String {
    format!("{DEFAULT_LOD_BATCH_MINIMUM_SIZE:.0}")
}

fn open_lod_batch_dialog(app: &mut AppState, indices: Vec<usize>, mode: LodBatchMode) {
    let live_ids = live_placement_ids(&app.placements, &app.element_states);
    let candidates = indices
        .into_iter()
        .filter_map(|index| {
            let placement = app.placements.get(index)?;
            Some(LodBatchCandidate {
                placement_index: index,
                id: placement.id.clone(),
                dff: placement.dff.clone(),
                size: placement_lod_size(app, index),
                existing_lod: existing_lod_with_live_ids(&app.placements, index, &live_ids)
                    .map(ToOwned::to_owned),
            })
        })
        .collect::<Vec<_>>();
    let minimum_size = default_lod_batch_minimum_size();
    app.lod_batch_dialog = Some(LodBatchDialog {
        mode,
        candidates,
        cursor: minimum_size.len(),
        minimum_size,
        selection_anchor: None,
        scroll: 0.0,
    });
    app.status_message = match mode {
        LodBatchMode::GenerateSelection =>
            "Review the selected elements and set a minimum size for LOD generation.".to_string(),
        LodBatchMode::GenerateSceneMissing =>
            "Set the minimum size; blue scene outlines preview elements missing an LOD."
                .to_string(),
        LodBatchMode::RegenerateScene =>
            "Set the minimum size; blue scene outlines preview every element that will receive a regenerated LOD."
                .to_string(),
    };
}

pub(crate) fn request_scene_lod_generation(app: &mut AppState) {
    if writer_is_busy(app) {
        app.status_message =
            "Scene LOD generation cannot start while another asset writer is running.".to_string();
        return;
    }
    let indices = app
        .placements
        .iter()
        .enumerate()
        .filter(|(index, placement)| {
            !app.element_states
                .get(*index)
                .is_some_and(|state| state.deleted)
                && !placement_is_app_lod(app, placement)
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if indices.is_empty() {
        app.status_message = "The scene has no live detail elements eligible for LODs.".to_string();
        return;
    }
    open_lod_batch_dialog(app, indices, LodBatchMode::GenerateSceneMissing);
}

pub(crate) fn continue_lod_batch_dialog(app: &mut AppState) {
    let Some(dialog) = app.lod_batch_dialog.take() else {
        return;
    };
    let Some(minimum_size) = lod_batch_minimum_size(&dialog) else {
        app.status_message = "Minimum LOD size must be a non-negative number.".to_string();
        app.lod_batch_dialog = Some(dialog);
        return;
    };
    let indices = dialog
        .candidates
        .iter()
        .filter(|candidate| lod_batch_candidate_included(candidate, minimum_size, dialog.mode))
        .map(|candidate| candidate.placement_index)
        .collect::<Vec<_>>();
    if indices.is_empty() {
        app.status_message = if dialog.mode == LodBatchMode::GenerateSelection {
            "No selected elements meet the minimum size and existing-LOD filters.".to_string()
        } else {
            "No scene elements meet the current size and generation-mode filters.".to_string()
        };
        app.lod_batch_dialog = Some(dialog);
        return;
    }
    match dialog.mode {
        LodBatchMode::GenerateSelection | LodBatchMode::GenerateSceneMissing => {
            start_selected_element_lod_generation(app, indices)
        }
        LodBatchMode::RegenerateScene => start_mass_element_lod_regeneration(app, indices),
    }
}

pub(crate) fn request_selected_element_lod(app: &mut AppState) {
    if writer_is_busy(app) {
        app.status_message =
            "LOD generation cannot start while another asset writer is running.".to_string();
        return;
    }
    let selected_indices = selected_live_indices_in_selection_order(app);
    if selected_indices.is_empty() {
        app.status_message = "Select a live element before generating an LOD.".to_string();
        return;
    }
    if all_indices_have_existing_lods(&app.placements, &app.element_states, &selected_indices) {
        let count = selected_indices.len();
        app.confirm_dialog = Some(ConfirmDialog {
            action: ConfirmAction::RegenerateLods(selected_indices),
            title: "Regenerate LODs?".to_string(),
            body: if count == 1 {
                "The selected model already has an assigned LOD. Regenerate it?".to_string()
            } else {
                format!("All {count} selected models already have assigned LODs. Regenerate them?")
            },
            detail: "Each successful result will replace its model's LOD assignment. An old LOD element is removed only when no live model still uses it.".to_string(),
            primary_label: "Regenerate LODs".to_string(),
            secondary_label: None,
            secondary_action: None,
        });
        return;
    }
    if selected_indices.len() > 1 {
        open_lod_batch_dialog(app, selected_indices, LodBatchMode::GenerateSelection);
        return;
    }
    start_selected_element_lod_generation(app, selected_indices);
}

pub(crate) fn regenerate_selected_element_lods(app: &mut AppState, selected_indices: Vec<usize>) {
    if writer_is_busy(app) {
        app.status_message =
            "LOD regeneration cannot start while another asset writer is running.".to_string();
        return;
    }
    let indices = selected_indices
        .into_iter()
        .filter(|index| {
            app.placements.get(*index).is_some()
                && !app
                    .element_states
                    .get(*index)
                    .is_some_and(|state| state.deleted)
                && existing_lod_for_placement(&app.placements, &app.element_states, *index)
                    .is_some()
        })
        .collect::<Vec<_>>();
    if indices.is_empty() {
        app.status_message =
            "The selected models no longer have live LOD assignments to regenerate.".to_string();
        return;
    }
    start_selected_element_lod_regeneration(app, indices);
}

fn lod_target_for_source(
    placements: &[Placement],
    states: &[ElementState],
    source_index: usize,
    lod_id: &str,
) -> Option<usize> {
    let source = placements.get(source_index)?;
    let candidates = placements
        .iter()
        .enumerate()
        .filter(|(index, placement)| {
            *index != source_index
                && placement.id.eq_ignore_ascii_case(lod_id)
                && !states.get(*index).is_some_and(|state| state.deleted)
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    match candidates.as_slice() {
        [] => return None,
        [only] => return Some(*only),
        _ => {}
    }
    if let Some(unique_id) = source
        .attrs
        .get("uniqueID")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        && let Some(index) = candidates.iter().copied().find(|index| {
            placements
                .get(*index)
                .and_then(|placement| placement.attrs.get("uniqueID"))
                .map(|value| value.trim())
                .is_some_and(|candidate| candidate.eq_ignore_ascii_case(unique_id))
        })
    {
        return Some(index);
    }
    candidates.into_iter().min_by(|a, b| {
        let distance_squared = |index: usize| {
            let pos = placements[index].pos;
            let dx = pos.x - source.pos.x;
            let dy = pos.y - source.pos.y;
            let dz = pos.z - source.pos.z;
            dx * dx + dy * dy + dz * dz
        };
        distance_squared(*a).total_cmp(&distance_squared(*b))
    })
}

pub(crate) fn request_lod_target_regeneration(app: &mut AppState, lod_index: usize) {
    if writer_is_busy(app) {
        app.status_message =
            "LOD regeneration cannot start while another asset writer is running.".to_string();
        return;
    }
    let Some(lod) = app.placements.get(lod_index) else {
        app.status_message = "The reviewed LOD no longer exists.".to_string();
        return;
    };
    if app
        .element_states
        .get(lod_index)
        .is_some_and(|state| state.deleted)
    {
        app.status_message = "The reviewed LOD is no longer live.".to_string();
        return;
    }
    let lod_id = lod.id.clone();
    let source_indices = app
        .placements
        .iter()
        .enumerate()
        .filter(|(index, placement)| {
            !app.element_states
                .get(*index)
                .is_some_and(|state| state.deleted)
                && placement.attrs.get("lodParent").is_some_and(|parent| {
                    parent.trim().eq_ignore_ascii_case(&lod_id)
                        && lod_target_for_source(
                            &app.placements,
                            &app.element_states,
                            *index,
                            &lod_id,
                        ) == Some(lod_index)
                })
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if source_indices.is_empty() {
        app.status_message =
            format!("Could not regenerate {lod_id}: no live base model resolves to this LOD.");
        return;
    }
    let count = source_indices.len();
    app.confirm_dialog = Some(ConfirmDialog {
        action: ConfirmAction::RegenerateLods(source_indices),
        title: "Regenerate LOD?".to_string(),
        body: if count == 1 {
            format!("Regenerate {lod_id} from its base model?")
        } else {
            format!("Regenerate {lod_id} from its {count} base model instances?")
        },
        detail: "The existing LOD assignment will be replaced with a newly generated LOD built from the base model. The old LOD is removed only when no live model still uses it.".to_string(),
        primary_label: "Regenerate".to_string(),
        secondary_label: None,
        secondary_action: None,
    });
}

fn start_selected_element_lod_generation(app: &mut AppState, selected_indices: Vec<usize>) {
    let total = selected_indices.len();
    let (indices, existing_lods) =
        partition_existing_lod_indices(&app.placements, &app.element_states, selected_indices);
    start_selected_element_lod_jobs(app, indices, existing_lods, total, false);
}

fn start_selected_element_lod_regeneration(app: &mut AppState, selected_indices: Vec<usize>) {
    let total = selected_indices.len();
    start_selected_element_lod_jobs(app, selected_indices, Vec::new(), total, true);
}

fn start_mass_element_lod_regeneration(app: &mut AppState, indices: Vec<usize>) {
    let total = indices.len();
    start_selected_element_lod_jobs(app, indices, Vec::new(), total, true);
}

fn start_selected_element_lod_jobs(
    app: &mut AppState,
    indices: Vec<usize>,
    existing_lods: Vec<(usize, String)>,
    total: usize,
    replace_existing: bool,
) {
    let (dff_entries, txd_entries) = indexed_lod_sources(app);
    let mut reserved_stems = reserved_lod_stems(app);
    reserved_stems.extend(dff_entries.keys().filter_map(|name| {
        Path::new(name)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(lower)
    }));
    let reserved_stems = Arc::new(reserved_stems);
    let wip_root = wip_root_path(&app.root);
    let mut mesh_sources = HashMap::<String, LodMeshSource>::new();
    let mut txd_sources = HashMap::<String, LodTxdSource>::new();
    let mut requests = Vec::with_capacity(total);
    let skipped = existing_lods
        .into_iter()
        .filter_map(|(index, parent)| {
            app.placements.get(index).map(|placement| {
                format!(
                    "{} ({}): already uses LOD {}",
                    placement.id, placement.dff, parent
                )
            })
        })
        .collect::<Vec<_>>();
    let mut failures = Vec::new();
    for index in indices {
        let Some(placement) = app.placements.get(index).cloned() else {
            failures.push(format!("element #{index}: selection no longer exists"));
            continue;
        };
        let replaced_lod_parent = if replace_existing {
            existing_lod_for_placement(&app.placements, &app.element_states, index)
                .map(ToOwned::to_owned)
        } else {
            None
        };
        let replaced_lod_index = replaced_lod_parent
            .as_deref()
            .and_then(|parent| existing_lod_index_for_parent(app, index, parent));
        let mesh_key = asset_key(&placement.dff, ".dff");
        let mesh_source = mesh_sources.get(&mesh_key).cloned().or_else(|| {
            let source = dff_source_from_index(app, &placement.dff, &dff_entries)?;
            mesh_sources.insert(mesh_key.clone(), source.clone());
            Some(source)
        });
        let Some(mesh_source) = mesh_source else {
            failures.push(format!("{}: DFF could not be located", placement.dff));
            continue;
        };
        let source_txd_name =
            definition_txd_name(&app.definitions, &placement.id).map(ToOwned::to_owned);
        let txd_source = source_txd_name.as_deref().and_then(|name| {
            let key = asset_key(name, ".txd");
            txd_sources.get(&key).cloned().or_else(|| {
                let source = txd_source_from_index(app, name, &txd_entries)?;
                txd_sources.insert(key, source.clone());
                Some(source)
            })
        });
        if source_txd_name.is_some() && txd_source.is_none() {
            failures.push(format!(
                "{}: TXD {} could not be located",
                placement.dff,
                source_txd_name.as_deref().unwrap_or_default()
            ));
            continue;
        }
        requests.push(LodGenerationRequest {
            mesh_source,
            txd_source,
            output_stem: lod_asset_stem(&placement.dff),
            attach_to_placement: Some(index),
            source_definition_id: Some(placement.id.clone()),
            source_zone: placement.zone.clone(),
            source_dff_name: placement.dff.clone(),
            source_txd_name,
            replaced_lod_parent,
            replaced_lod_index,
            project_root: app.root.clone(),
            reserved_stems: Arc::clone(&reserved_stems),
            wip_root: wip_root.clone(),
            stem_preallocated: false,
        });
    }
    start_lod_jobs(app, requests, skipped, failures, total);
}

pub(crate) fn request_editing_dff_lod(app: &mut AppState) {
    if writer_is_busy(app) {
        app.status_message =
            "LOD generation cannot start while another asset writer is running.".to_string();
        return;
    }
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_ref() else {
        app.status_message = "Open a DFF in the DFF editor first.".to_string();
        return;
    };
    let source_name = dff.name.clone();
    let source_txd_name = dff.txd_context.clone();
    let txd_source = source_txd_name
        .as_deref()
        .and_then(|name| txd_source_for_app(app, name));
    if source_txd_name.is_some() && txd_source.is_none() {
        app.status_message = format!(
            "Could not locate TXD {}",
            source_txd_name.as_deref().unwrap_or_default()
        );
        return;
    }
    let attach_to_placement = app.placements.get(app.selected).and_then(|placement| {
        (asset_key(&placement.dff, ".dff") == asset_key(&source_name, ".dff"))
            .then_some(app.selected)
    });
    if let Some(index) = attach_to_placement
        && let Some(parent) =
            existing_lod_for_placement(&app.placements, &app.element_states, index)
    {
        app.status_message = format!(
            "Skipped {}: selected element already uses LOD {}.",
            source_name, parent
        );
        return;
    }
    let source_definition_id =
        attach_to_placement.and_then(|index| app.placements.get(index).map(|p| p.id.clone()));
    let source_zone = attach_to_placement
        .and_then(|index| app.placements.get(index).map(|p| p.zone.clone()))
        .unwrap_or_default();
    let request = LodGenerationRequest {
        mesh_source: LodMeshSource::Raw(dff.raw.clone()),
        txd_source,
        output_stem: lod_asset_stem(&source_name),
        attach_to_placement,
        source_definition_id,
        source_zone,
        source_dff_name: source_name,
        source_txd_name,
        replaced_lod_parent: None,
        replaced_lod_index: None,
        project_root: app.root.clone(),
        reserved_stems: Arc::new(reserved_lod_stems(app)),
        wip_root: wip_root_path(&app.root),
        stem_preallocated: false,
    };
    start_lod_job(app, request);
}

fn write_lod_result_to_wip(wip_root: &Path, result: &LodGenerationResult) -> Result<(), String> {
    let mut replacements = vec![
        (result.dff_name.clone(), result.dff_bytes.clone()),
        (result.col_name.clone(), result.col_bytes.clone()),
    ];
    if let (Some(name), Some(bytes)) = (&result.txd_name, &result.txd_bytes) {
        replacements.push((name.clone(), bytes.clone()));
    }
    upsert_replacement_assets(wip_root, &replacements)
        .map_err(|error| format!("Could not stage generated LOD assets: {error}"))
}

fn empty_lod_txd() -> Vec<u8> {
    rw_chunk(0x16, rw_chunk(0x01, vec![0, 0, 0, 0]))
}

fn merge_generated_lod_txd(mut destination: Vec<u8>, generated: &[u8]) -> Result<Vec<u8>, String> {
    for texture in parse_txd_texture_contents_with_natives(generated)? {
        // Generated names are content addressed, so a name already in the
        // dictionary holds these exact pixels. Skipping it keeps a batch of
        // repeated buildings from rewriting the whole dictionary per LOD.
        if txd_contains_texture_native(&destination, &lower(&texture.name)) {
            continue;
        }
        destination =
            replace_or_append_texture_native_in_txd(destination, &texture.native, &texture.name)?;
    }
    Ok(destination)
}

/// Rebuild a dictionary whose header claims more bytes than the asset actually
/// holds, keeping every texture that is completely present.
///
/// Editor builds before the IMG entry-size guard could store a shared LOD
/// dictionary past the 65,535-sector directory limit, which wrapped the entry
/// size and handed back a cut-off asset. Salvaging what survived lets the next
/// generation continue instead of failing on every LOD; the textures that were
/// cut off come back by regenerating the LODs that referenced them.
fn repair_truncated_lod_txd(bytes: Vec<u8>) -> Vec<u8> {
    if bytes.len() < 12 || rd32(&bytes, 0) != 0x16 {
        return bytes;
    }
    let declared = 12usize.saturating_add(rd32(&bytes, 4) as usize);
    if declared <= bytes.len() {
        return bytes;
    }
    let mut natives = Vec::new();
    let mut chunk = 12usize;
    let mut device = [0u8; 2];
    while chunk + 12 <= bytes.len() {
        let id = rd32(&bytes, chunk);
        let end = chunk
            .saturating_add(12)
            .saturating_add(rd32(&bytes, chunk + 4) as usize);
        if end > bytes.len() {
            break;
        }
        match id {
            0x15 => natives.push(&bytes[chunk..end]),
            0x01 if end >= chunk + 16 => device.copy_from_slice(&bytes[chunk + 14..chunk + 16]),
            _ => {}
        }
        chunk = end;
    }
    let count = u16::try_from(natives.len()).unwrap_or(u16::MAX);
    let mut body = rw_chunk(0x01, [count.to_le_bytes(), device].concat());
    for native in natives {
        body.extend_from_slice(native);
    }
    rw_chunk(0x16, body)
}

/// Pick the shared dictionary this atlas set belongs in, rolling over to the
/// next one once the current dictionary has no room left for it.
///
/// Returns the dictionary name and its current contents, which are empty when
/// the dictionary does not exist yet.
fn world_lod_txd_destination(app: &AppState, generated_len: usize) -> (String, Option<Vec<u8>>) {
    for index in 0.. {
        let name = world_lod_txd_name(index);
        let Some(current) = txd_source_for_app(app, &name)
            .map(read_txd_source)
            .map(repair_truncated_lod_txd)
        else {
            return (name, None);
        };
        if current.len().saturating_add(generated_len) <= WORLD_LOD_TXD_MAX_BYTES {
            return (name, Some(current));
        }
    }
    unreachable!("the dictionary index is unbounded")
}

fn stage_lod_result(
    app: &mut AppState,
    result: &mut LodGenerationResult,
    touched_txds: &mut BTreeMap<String, (String, Option<Vec<u8>>)>,
) -> Result<(), String> {
    if let Some(generated) = result.txd_bytes.take() {
        let (name, current) = world_lod_txd_destination(app, generated.len());
        let key = asset_key(&name, ".txd");
        // Undo restores whatever each touched dictionary held before this
        // batch, so record that the first time the batch reaches it.
        touched_txds
            .entry(key)
            .or_insert_with(|| (name.clone(), current.clone()));
        let destination = current.unwrap_or_else(empty_lod_txd);
        result.txd_bytes = Some(merge_generated_lod_txd(destination, &generated)?);
        result.txd_name = Some(name);
    }
    write_lod_result_to_wip(&wip_root_path(&app.root), result)?;
    app.pending_replacement_assets.insert(
        asset_key(&result.dff_name, ".dff"),
        (result.dff_name.clone(), result.dff_bytes.clone()),
    );
    app.pending_replacement_assets.insert(
        asset_key(&result.col_name, ".col"),
        (result.col_name.clone(), result.col_bytes.clone()),
    );
    if let (Some(name), Some(bytes)) = (&result.txd_name, &result.txd_bytes) {
        app.pending_replacement_assets
            .insert(asset_key(name, ".txd"), (name.clone(), bytes.clone()));
        app.pending_txd_writes.insert(asset_key(name, ".txd"));
        reindex_staged_txd(app, name);
    }
    app.loaded_wip = true;
    Ok(())
}

fn compile_lod_result(app: &mut AppState, result: &LodGenerationResult) {
    let txd_context = result.txd_name.as_deref();
    let ambient_lift = scene_ambient_lift_from_timecyc(&app.timecyc);
    if let Some(render_mesh) = compile_render_mesh(
        result.raw.clone(),
        txd_context,
        None,
        None,
        &app.texture_files,
        &app.txd_textures,
        &mut app.textures,
        &mut app.textured_parts,
        app.options.textures,
        ambient_lift,
    ) {
        let key = mesh_key_from_dff_txd(&result.dff_name, txd_context);
        replace_render_mesh(&mut app.meshes, key, render_mesh);
    }
    let collision_key = asset_key(&result.col_name, ".col");
    invalidate_collision_render_cache(app, &collision_key);
    app.collisions
        .insert(collision_key, result.col_mesh.clone());
}

fn attach_lod_result(
    app: &mut AppState,
    result: &LodGenerationResult,
    rebuild_scene: bool,
) -> bool {
    let mut attached = false;
    if let Some(source_index) = result.attach_to_placement
        && let Some(source) = app.placements.get(source_index).cloned()
        && asset_key(&source.dff, ".dff") == asset_key(&result.source_dff_name, ".dff")
    {
        let source_is_building = source.tag.eq_ignore_ascii_case("building");
        let generated_mesh_key =
            mesh_key_from_dff_txd(&result.dff_name, result.txd_name.as_deref());
        let generated_bounds = app.meshes.get(&generated_mesh_key).map(|mesh| mesh.bounds);
        let generated_model_distance =
            repaired_lod_distance(true, source_is_building, false, generated_bounds);
        let mut definition = result
            .source_definition_id
            .as_ref()
            .and_then(|id| app.definitions.get(id))
            .cloned()
            .unwrap_or(Definition {
                id: result.stem.clone(),
                zone: result.source_zone.clone(),
                attrs: BTreeMap::new(),
            });
        definition.id = result.stem.clone();
        definition.zone = source.zone.clone();
        definition
            .attrs
            .insert("id".to_string(), result.stem.clone());
        definition
            .attrs
            .insert("dff".to_string(), result.stem.clone());
        definition.attrs.remove("col");
        definition.attrs.remove("source");
        definition.attrs.remove("__override");
        definition.attrs.remove("__overrideAttrs");
        definition.attrs.insert(
            "lodDistance".to_string(),
            generated_model_distance.to_string(),
        );
        definition
            .attrs
            .insert("col".to_string(), result.stem.clone());
        if result.txd_name.is_some() {
            let txd = result
                .txd_name
                .as_deref()
                .unwrap_or(WORLD_LOD_TXD_NAME)
                .trim_end_matches(".txd");
            definition.attrs.insert("txd".to_string(), txd.to_string());
        } else if let Some(source_txd) = &result.source_txd_name {
            definition
                .attrs
                .insert("txd".to_string(), source_txd.clone());
        }
        app.definitions.insert(result.stem.clone(), definition);

        // Calculate both sides with the same role-aware helper as Repair LODs.
        let source_detail_distance = repaired_lod_distance(
            false,
            source_is_building,
            true,
            element_mesh(app, &source).map(|mesh| mesh.bounds),
        );
        let source_definition_id = result
            .source_definition_id
            .as_deref()
            .unwrap_or(source.id.as_str());
        if app.readonly_definition_ids.contains(source_definition_id) {
            make_definition_override_writable(app, source_definition_id, source.zone.clone());
        }
        if let Some(source_definition) = app.definitions.get_mut(source_definition_id) {
            source_definition.attrs.insert(
                "lodDistance".to_string(),
                source_detail_distance.to_string(),
            );
            mark_definition_override_attr(source_definition, "lodDistance");
        }

        let mut lod = source.clone();
        lod.id = result.stem.clone();
        lod.dff = result.stem.clone();
        lod.attrs.remove("lodParent");
        lod.attrs.remove("uniqueID");
        sync_placement_attrs(&mut lod);
        app.placements.push(lod);
        app.element_states.push(ElementState::default());
        app.outliner_labels.push(None);
        if let Some(detail) = app.placements.get_mut(source_index) {
            detail
                .attrs
                .insert("lodParent".to_string(), result.stem.clone());
        }
        if let (Some(old_parent), Some(old_index)) = (
            result.replaced_lod_parent.as_deref(),
            result.replaced_lod_index,
        ) {
            let still_used =
                lod_parent_is_still_used(&app.placements, &app.element_states, old_parent);
            if !still_used && let Some(state) = app.element_states.get_mut(old_index) {
                state.deleted = true;
            }
        }
        app.lod_ids.insert(lower(&result.stem));
        attached = true;
        invalidate_outliner_labels(app);
        if rebuild_scene {
            rebuild_outliner_filter(app);
            rebuild_render_cells(app);
        }
    }
    invalidate_validation_cache(app);
    attached
}

fn lod_history_artifact(
    app: &AppState,
    result: &LodGenerationResult,
) -> LodGenerationHistoryArtifact {
    let assets = vec![
        (
            asset_key(&result.dff_name, ".dff"),
            result.dff_name.clone(),
            result.dff_bytes.clone(),
        ),
        (
            asset_key(&result.col_name, ".col"),
            result.col_name.clone(),
            result.col_bytes.clone(),
        ),
    ];
    let mesh_key = mesh_key_from_dff_txd(&result.dff_name, result.txd_name.as_deref());
    LodGenerationHistoryArtifact {
        assets,
        mesh: app.meshes.get(&mesh_key).cloned(),
        mesh_key,
        collision_key: asset_key(&result.col_name, ".col"),
        collision: result.col_mesh.clone(),
    }
}

fn finish_lod_result(
    app: &mut AppState,
    result: &LodGenerationResult,
    attached: bool,
    elapsed: f32,
) {
    let texture_summary = if result.texture_count == 0 && result.source_had_textures {
        "mesh-only; no TXD context was available for texture downscaling".to_string()
    } else if result.texture_count == 0 {
        "no texture TXD was needed".to_string()
    } else if result.atlased {
        format!(
            "packed materials into {} texture{} (opaque and alpha separated; tiled materials kept standalone)",
            result.texture_count,
            if result.texture_count == 1 { "" } else { "s" },
        )
    } else {
        format!(
            "wrote {} lower-resolution texture{}",
            result.texture_count,
            if result.texture_count == 1 { "" } else { "s" }
        )
    };
    app.status_message = format!(
        "Generated {}: {} -> {} triangles, {} -> {} vertices; {}; {}; {} in {:.1}s. Save to apply; Undo removes the generated LOD.",
        result.dff_name,
        result.source_triangles,
        result.output_triangles,
        result.source_vertices,
        result.output_vertices,
        texture_summary,
        if result.simplification_attempts > 1 {
            format!(
                "accepted validation retry {}",
                result.simplification_attempts
            )
        } else {
            "passed geometry/UV validation".to_string()
        },
        if attached {
            "created and assigned the LOD element"
        } else {
            "staged assets (no matching selected scene element to assign)"
        },
        elapsed
    );
}

fn poll_lod_workers(job: &mut LodGenerationJob) {
    let mut index = 0usize;
    while index < job.workers.len() {
        let outcome = match job.workers[index].rx.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Err(format!(
                "{}: LOD generation worker disconnected unexpectedly",
                job.workers[index].source_name
            ))),
        };
        let Some(outcome) = outcome else {
            index += 1;
            continue;
        };
        let worker = job.workers.swap_remove(index);
        job.ready.insert(worker.order, outcome);
    }
    fill_lod_worker_slots(job);
}

fn select_next_lod_result(job: &mut LodGenerationJob) {
    while job.result.is_none() {
        let Some(outcome) = job.ready.remove(&job.next_apply_order) else {
            break;
        };
        job.next_apply_order += 1;
        match outcome {
            Ok(result) => job.result = Some(result),
            Err(error) => {
                job.failures.push(error);
                job.processed += 1;
            }
        }
    }
}

fn lod_job_has_no_more_results(job: &LodGenerationJob) -> bool {
    job.result.is_none()
        && job.next_apply_order >= job.request_count
        && job.pending.is_empty()
        && job.workers.is_empty()
        && job.ready.is_empty()
}

fn finish_lod_batch(app: &mut AppState, job: &mut LodGenerationJob) {
    if job.attached_count > 0 {
        rebuild_outliner_filter(app);
        rebuild_render_cells(app);
    }
    if !job.history_artifacts.is_empty() {
        commit_lod_generation_history(
            app,
            job.history_before.clone(),
            mem::take(&mut job.history_artifacts),
            mem::take(&mut job.history_txd_before),
        );
    }
    let elapsed = job.started_at.elapsed().as_secs_f32();
    let failure_summary = job
        .failures
        .iter()
        .take(3)
        .cloned()
        .collect::<Vec<_>>()
        .join("; ");
    app.status_message = if job.total == 1 && job.successes == 0 {
        format!(
            "LOD generation failed: {}",
            failure_summary.trim().trim_start_matches("; ")
        )
    } else if job.failures.is_empty() && job.skipped.is_empty() {
        format!(
            "Generated {} LODs and assigned {}: {} -> {} total triangles in {:.1}s. Save to apply; one Undo removes the batch.",
            job.successes, job.attached_count, job.source_triangles, job.output_triangles, elapsed
        )
    } else if job.failures.is_empty() {
        format!(
            "Generated {} LOD(s) and assigned {}; skipped {} selected element(s) that already had LODs. Completed in {:.1}s. Save to apply; one Undo removes the batch.",
            job.successes,
            job.attached_count,
            job.skipped.len(),
            elapsed
        )
    } else {
        format!(
            "Generated {} of {} LODs and assigned {}; skipped {} with existing LODs; {} failed: {}. Completed in {:.1}s. Save to apply successful results; one Undo removes them.",
            job.successes,
            job.total,
            job.attached_count,
            job.skipped.len(),
            job.failures.len(),
            failure_summary,
            elapsed
        )
    };
}

pub(crate) fn update_lod_generation_job(app: &mut AppState) {
    let Some(mut job) = app.lod_generation_job.take() else {
        return;
    };
    poll_lod_workers(&mut job);
    if job.result.is_none() {
        select_next_lod_result(&mut job);
        if lod_job_has_no_more_results(&job) {
            finish_lod_batch(app, &mut job);
            return;
        }
        if job.result.is_some() {
            app.status_message = format!(
                "LOD {} of {} generated; installing assets while {} worker(s) continue...",
                (job.processed + 1).min(job.total),
                job.total,
                job.workers.len()
            );
        } else {
            app.status_message = format!(
                "Generating LODs in parallel: {} active, {} completed of {}...",
                job.workers.len(),
                job.processed,
                job.total
            );
        }
        app.lod_generation_job = Some(job);
        return;
    }
    match job.apply_phase {
        0 => {
            let result = job
                .result
                .as_mut()
                .expect("LOD result exists while applying");
            if let Err(error) = stage_lod_result(app, result, &mut job.history_txd_before) {
                job.failures
                    .push(format!("{}: {error}", result.source_dff_name));
                job.processed += 1;
                job.result = None;
                job.apply_phase = 0;
                job.attached = false;
                if lod_job_has_no_more_results(&job) {
                    finish_lod_batch(app, &mut job);
                } else {
                    app.lod_generation_job = Some(job);
                }
                return;
            }
            app.status_message = format!(
                "LOD {} of {} generated; indexed texture atlas...",
                (job.processed + 1).min(job.total),
                job.total
            );
            job.apply_phase = 1;
            app.lod_generation_job = Some(job);
        }
        1 => {
            let result = job
                .result
                .as_ref()
                .expect("LOD result exists while applying");
            compile_lod_result(app, result);
            app.status_message = format!(
                "LOD {} of {} generated; refreshed preview mesh...",
                (job.processed + 1).min(job.total),
                job.total
            );
            job.apply_phase = 2;
            app.lod_generation_job = Some(job);
        }
        2 => {
            let result = job
                .result
                .as_ref()
                .expect("LOD result exists while applying");
            job.attached = attach_lod_result(app, result, job.total == 1);
            job.apply_phase = 3;
            app.lod_generation_job = Some(job);
        }
        _ => {
            let result = job
                .result
                .as_ref()
                .expect("LOD result exists while applying");
            job.successes += 1;
            job.processed += 1;
            job.attached_count += usize::from(job.attached);
            job.source_triangles += result.source_triangles;
            job.output_triangles += result.output_triangles;
            job.history_artifacts
                .push(lod_history_artifact(app, result));
            if job.total == 1 {
                commit_lod_generation_history(
                    app,
                    job.history_before.clone(),
                    mem::take(&mut job.history_artifacts),
                    mem::take(&mut job.history_txd_before),
                );
                finish_lod_result(
                    app,
                    result,
                    job.attached,
                    job.started_at.elapsed().as_secs_f32(),
                );
                return;
            }
            job.result = None;
            job.apply_phase = 0;
            job.attached = false;
            select_next_lod_result(&mut job);
            if lod_job_has_no_more_results(&job) {
                finish_lod_batch(app, &mut job);
            } else {
                app.lod_generation_job = Some(job);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn vc_man_grnd2_lod_generation_audit() {
        let root = PathBuf::from(
            std::env::var("EAGLE_VC_ROOT").expect("EAGLE_VC_ROOT must name the Vice City resource"),
        );
        let dff_entry = find_dff_entry(&root, "man_grnd2").expect("man_grnd2.dff");
        let txd_entry = collect_resource_txd_entries(&root)
            .remove("man_grnds_kb.txd")
            .expect("man_grnds_kb.txd");
        let source = read_mesh_source(LodMeshSource::Entry(dff_entry.clone())).unwrap();
        eprintln!(
            "source: {} vertices, {} triangles, {} components, {} materials, {} UVs, textures {:?}",
            source.vertices.len(),
            source.triangles.len(),
            lod_geometry_component_count(&source),
            source.material_textures.len(),
            source.uvs.len(),
            referenced_texture_names(&source),
        );
        let (mut lod, attempts) = simplify_lod_mesh_with_retry(&source).unwrap();
        eprintln!(
            "lod: {} vertices, {} triangles after {attempts} attempts",
            lod.vertices.len(),
            lod.triangles.len(),
        );
        let txd_bytes = read_txd_entry_bytes(&txd_entry);
        let textures = decode_lod_textures(&lod, "man_grnds_kb.txd", &txd_bytes).unwrap();
        eprintln!("decoded {} textures", textures.len());
        let atlases = remap_raw_for_atlases(&mut lod, &textures).unwrap().textures;
        eprintln!("generated {} atlas/atlases", atlases.len());
        let dff = write_normalized_dff(&lod, "lod_man_grnd2").unwrap();
        validate_lod_candidate(&lod, &parse_dff_mesh(&dff)).unwrap();

        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let staging_root = env::temp_dir().join(format!(
            "eagle_vc_man_grnd2_lod_{}_{}",
            std::process::id(),
            nonce
        ));
        fs::create_dir_all(&staging_root).unwrap();
        let result = generate_lod(LodGenerationRequest {
            mesh_source: LodMeshSource::Entry(dff_entry),
            txd_source: Some(LodTxdSource::Entry(txd_entry)),
            output_stem: "lod_man_grnd2".to_string(),
            attach_to_placement: None,
            source_definition_id: Some("man_grnd2".to_string()),
            source_zone: "mansion".to_string(),
            source_dff_name: "man_grnd2.dff".to_string(),
            source_txd_name: Some("man_grnds_kb.txd".to_string()),
            replaced_lod_parent: None,
            replaced_lod_index: None,
            project_root: root,
            reserved_stems: Arc::new(BTreeSet::new()),
            wip_root: staging_root.clone(),
            stem_preallocated: false,
        })
        .unwrap();
        assert!(result.atlased);
        assert_eq!(result.texture_count, 1);
        assert!(result.txd_bytes.is_some());
        assert!(!col_validation_has_errors(&validate_col_for_game_load(
            &result.col_name,
            &result.col_bytes,
        )));
        fs::remove_dir_all(staging_root).unwrap();
    }

    fn lod_test_placement(id: &str, lod_parent: Option<&str>) -> Placement {
        let mut attrs = BTreeMap::new();
        if let Some(parent) = lod_parent {
            attrs.insert("lodParent".to_string(), parent.to_string());
        }
        Placement {
            id: id.to_string(),
            dff: format!("{id}.dff"),
            zone: "test".to_string(),
            tag: "object".to_string(),
            attrs,
            pos: V3::default(),
            rot: V3::default(),
        }
    }

    #[test]
    fn generated_lod_names_leave_room_for_img_terminator() {
        let stem = lod_asset_stem("a_very_long_vice_city_model_name.dff");
        assert!(stem.len() <= IMG_RUNTIME_SAFE_ENTRY_NAME_BYTES - ".dff".len());
        assert!(format!("{stem}.dff").len() <= IMG_RUNTIME_SAFE_ENTRY_NAME_BYTES);
        assert!(format!("{stem}.col").len() <= IMG_RUNTIME_SAFE_ENTRY_NAME_BYTES);
        assert!(format!("{stem}.txd").len() <= IMG_RUNTIME_SAFE_ENTRY_NAME_BYTES);
    }

    #[test]
    fn existing_lod_detection_requires_a_live_target_or_self_lod() {
        let placements = vec![
            lod_test_placement("detail", Some("lod_detail")),
            lod_test_placement("self_detail", Some("self")),
            lod_test_placement("broken_detail", Some("missing_lod")),
            lod_test_placement("lod_detail", None),
        ];
        let mut states = vec![ElementState::default(); placements.len()];

        assert_eq!(
            existing_lod_for_placement(&placements, &states, 0),
            Some("lod_detail")
        );
        assert_eq!(
            existing_lod_for_placement(&placements, &states, 1),
            Some("self")
        );
        assert_eq!(existing_lod_for_placement(&placements, &states, 2), None);

        states[3].deleted = true;
        assert_eq!(existing_lod_for_placement(&placements, &states, 0), None);
    }

    #[test]
    fn multi_selection_partitions_existing_lods_without_blocking_unassigned_assets() {
        let placements = vec![
            lod_test_placement("assigned", Some("lod_assigned")),
            lod_test_placement("unassigned", None),
            lod_test_placement("lod_assigned", None),
            lod_test_placement("broken", Some("missing_lod")),
        ];
        let states = vec![ElementState::default(); placements.len()];

        let (ready, skipped) = partition_existing_lod_indices(&placements, &states, vec![0, 1, 3]);

        assert_eq!(ready, vec![1, 3]);
        assert_eq!(skipped, vec![(0, "lod_assigned".to_string())]);
    }

    #[test]
    fn regeneration_requires_every_selected_model_to_have_a_live_lod() {
        let placements = vec![
            lod_test_placement("first", Some("lod_first")),
            lod_test_placement("second", Some("lod_second")),
            lod_test_placement("missing", Some("deleted_lod")),
            lod_test_placement("lod_first", None),
            lod_test_placement("lod_second", None),
            lod_test_placement("deleted_lod", None),
        ];
        let mut states = vec![ElementState::default(); placements.len()];
        states[5].deleted = true;

        assert!(all_indices_have_existing_lods(
            &placements,
            &states,
            &[0, 1]
        ));
        assert!(!all_indices_have_existing_lods(
            &placements,
            &states,
            &[0, 2]
        ));
        assert!(!all_indices_have_existing_lods(&placements, &states, &[]));
    }

    #[test]
    fn lod_review_regeneration_resolves_the_correct_repeated_target() {
        let mut detail = lod_test_placement("detail", Some("shared_lod"));
        detail
            .attrs
            .insert("uniqueID".to_string(), "pair_b".to_string());
        let mut lod_a = lod_test_placement("shared_lod", None);
        lod_a
            .attrs
            .insert("uniqueID".to_string(), "pair_a".to_string());
        let mut lod_b = lod_test_placement("shared_lod", None);
        lod_b
            .attrs
            .insert("uniqueID".to_string(), "pair_b".to_string());
        let placements = vec![detail, lod_a, lod_b];
        let states = vec![ElementState::default(); placements.len()];

        assert_eq!(
            lod_target_for_source(&placements, &states, 0, "shared_lod"),
            Some(2)
        );
    }

    #[test]
    fn old_lod_is_retained_until_no_live_model_uses_it() {
        let placements = vec![
            lod_test_placement("first", Some("shared_lod")),
            lod_test_placement("second", Some("shared_lod")),
            lod_test_placement("shared_lod", None),
        ];
        let mut states = vec![ElementState::default(); placements.len()];

        assert!(lod_parent_is_still_used(&placements, &states, "shared_lod"));
        states[0].deleted = true;
        assert!(lod_parent_is_still_used(&placements, &states, "shared_lod"));
        states[1].deleted = true;
        assert!(!lod_parent_is_still_used(
            &placements,
            &states,
            "shared_lod"
        ));
    }

    #[test]
    fn batch_minimum_size_filter_is_inclusive_and_always_skips_existing_lods() {
        let candidate = |size, existing_lod: Option<&str>| LodBatchCandidate {
            placement_index: 0,
            id: "detail".to_string(),
            dff: "detail.dff".to_string(),
            size,
            existing_lod: existing_lod.map(ToOwned::to_owned),
        };

        let selection = LodBatchMode::GenerateSelection;
        assert!(!lod_batch_candidate_included(
            &candidate(9.99, None),
            10.0,
            selection
        ));
        assert!(lod_batch_candidate_included(
            &candidate(10.0, None),
            10.0,
            selection
        ));
        assert!(lod_batch_candidate_included(
            &candidate(12.0, None),
            10.0,
            selection
        ));
        assert!(!lod_batch_candidate_included(
            &candidate(12.0, Some("lod_detail")),
            10.0,
            selection
        ));
        assert!(lod_batch_candidate_included(
            &candidate(12.0, Some("lod_detail")),
            10.0,
            LodBatchMode::RegenerateScene
        ));
    }

    #[test]
    fn batch_minimum_size_rejects_invalid_or_negative_values() {
        let dialog = |minimum_size: &str| LodBatchDialog {
            mode: LodBatchMode::GenerateSelection,
            candidates: Vec::new(),
            minimum_size: minimum_size.to_string(),
            cursor: minimum_size.len(),
            selection_anchor: None,
            scroll: 0.0,
        };

        assert_eq!(lod_batch_minimum_size(&dialog("")), Some(0.0));
        assert_eq!(lod_batch_minimum_size(&dialog("25.5")), Some(25.5));
        assert_eq!(lod_batch_minimum_size(&dialog("-1")), None);
        assert_eq!(lod_batch_minimum_size(&dialog("not a number")), None);
    }

    #[test]
    fn lod_batch_defaults_to_100_units() {
        let minimum_size = default_lod_batch_minimum_size();
        assert_eq!(minimum_size, "100");

        let dialog = LodBatchDialog {
            mode: LodBatchMode::GenerateSelection,
            candidates: Vec::new(),
            cursor: minimum_size.len(),
            minimum_size,
            selection_anchor: None,
            scroll: 0.0,
        };
        assert_eq!(lod_batch_minimum_size(&dialog), Some(100.0));
    }

    fn grid_mesh(size: usize) -> RawMesh {
        let mut raw = RawMesh::default();
        for y in 0..=size {
            for x in 0..=size {
                raw.vertices.push(V3 {
                    x: x as f32,
                    y: y as f32,
                    z: 0.0,
                });
                raw.normals.push(V3 {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                });
                raw.uvs.push(V2 {
                    u: x as f32 / size as f32,
                    v: y as f32 / size as f32,
                });
            }
        }
        for y in 0..size {
            for x in 0..size {
                let a = (y * (size + 1) + x) as u32;
                let b = a + 1;
                let c = a + (size + 1) as u32;
                let d = c + 1;
                raw.triangles.push(Tri {
                    a,
                    b,
                    c: d,
                    material: 0,
                });
                raw.triangles.push(Tri {
                    a,
                    b: d,
                    c,
                    material: 0,
                });
            }
        }
        raw.material_textures.push("grid".to_string());
        raw
    }

    fn mesh_bounds(raw: &RawMesh) -> (V3, V3) {
        let first = raw.vertices[0];
        raw.vertices
            .iter()
            .copied()
            .fold((first, first), |(mut min, mut max), vertex| {
                min.x = min.x.min(vertex.x);
                min.y = min.y.min(vertex.y);
                min.z = min.z.min(vertex.z);
                max.x = max.x.max(vertex.x);
                max.y = max.y.max(vertex.y);
                max.z = max.z.max(vertex.z);
                (min, max)
            })
    }

    fn assert_same_bounds(source: &RawMesh, lod: &RawMesh) {
        let (source_min, source_max) = mesh_bounds(source);
        let (lod_min, lod_max) = mesh_bounds(lod);
        assert_eq!(lod_min, source_min, "LOD minimum extent changed");
        assert_eq!(lod_max, source_max, "LOD maximum extent changed");
    }

    #[test]
    fn lod_simplification_reduces_grid_and_keeps_silhouette() {
        let source = grid_mesh(8);
        let lod = simplify_lod_mesh(&source).unwrap();
        assert!(
            lod.triangles.len() <= 64,
            "expected at least a 50% validated grid reduction, got {} of {} triangles",
            lod.triangles.len(),
            source.triangles.len()
        );
        assert!(lod.triangles.iter().all(|triangle| {
            [triangle.a, triangle.b, triangle.c]
                .iter()
                .all(|index| (*index as usize) < lod.vertices.len())
        }));
        assert_eq!(lod.uvs.len(), lod.vertices.len());
        assert_same_bounds(&source, &lod);
        validate_lod_candidate(&source, &lod).unwrap();
    }

    fn winding_road_mesh() -> RawMesh {
        let center_and_width = [
            (0.0f32, 5.0f32),
            (0.0, 5.0),
            (1.0, 5.5),
            (3.0, 6.0),
            (6.0, 5.0),
            (8.0, 4.0),
            (9.0, 4.5),
            (9.0, 5.0),
            (8.0, 5.0),
        ];
        let mut raw = RawMesh::default();
        for (column, (center, width)) in center_and_width.into_iter().enumerate() {
            for across in [-1.0f32, 0.0, 1.0] {
                raw.vertices.push(V3 {
                    x: column as f32 * 12.0,
                    y: center + width * across,
                    z: (column as f32 * 0.15).sin(),
                });
                raw.uvs.push(V2 {
                    u: column as f32,
                    v: (across + 1.0) * 0.5,
                });
            }
        }
        for column in 0..center_and_width.len() - 1 {
            for across in 0..2u32 {
                let a = column as u32 * 3 + across;
                let b = a + 1;
                let c = a + 3;
                let d = c + 1;
                raw.triangles.push(Tri {
                    a,
                    b,
                    c: d,
                    material: 0,
                });
                raw.triangles.push(Tri {
                    a,
                    b: d,
                    c,
                    material: 0,
                });
            }
        }
        raw.material_textures.push("road".to_string());
        raw
    }

    #[test]
    fn open_road_lod_preserves_every_outline_bend_and_taper() {
        let source = winding_road_mesh();
        let source_boundary = lod_open_boundary_positions(&source);
        assert!(source_boundary.len() > 4);

        let lod = simplify_lod_mesh(&source).unwrap();
        let lod_positions = lod
            .vertices
            .iter()
            .copied()
            .map(lod_position_key)
            .collect::<BTreeSet<_>>();

        assert!(
            source_boundary.is_subset(&lod_positions),
            "LOD dropped a road outline vertex and can bridge across its shape"
        );
        validate_lod_candidate(&source, &lod).unwrap();
    }

    #[test]
    fn lod_validation_rejects_filling_a_concave_outline() {
        let mut source = RawMesh::default();
        for y in 0..=2 {
            for x in 0..=2 {
                source.vertices.push(V3 {
                    x: x as f32,
                    y: y as f32,
                    z: 0.0,
                });
            }
        }
        let add_cell = |raw: &mut RawMesh, x: u32, y: u32| {
            let a = y * 3 + x;
            let b = a + 1;
            let c = a + 3;
            let d = c + 1;
            raw.triangles.push(Tri {
                a,
                b,
                c: d,
                material: 0,
            });
            raw.triangles.push(Tri {
                a,
                b: d,
                c,
                material: 0,
            });
        };
        add_cell(&mut source, 0, 0);
        add_cell(&mut source, 1, 0);
        add_cell(&mut source, 0, 1);

        let mut filled = source.clone();
        add_cell(&mut filled, 1, 1);
        let error = validate_lod_candidate(&source, &filled).unwrap_err();
        assert!(error.contains("outside the source shape"), "{error}");
    }

    /// Two axis-aligned quads in the z=0 plane, as one mesh.
    fn two_quad_mesh(second_origin: (f32, f32), second_size: f32) -> RawMesh {
        let mut raw = RawMesh::default();
        for (origin, size) in [((0.0, 0.0), 40.0), (second_origin, second_size)] {
            let base = raw.vertices.len() as u32;
            for (x, y) in [(0.0, 0.0), (size, 0.0), (0.0, size), (size, size)] {
                raw.vertices.push(V3 {
                    x: origin.0 + x,
                    y: origin.1 + y,
                    z: 0.0,
                });
                raw.uvs.push(V2 {
                    u: x / size,
                    v: y / size,
                });
            }
            raw.triangles.push(Tri {
                a: base,
                b: base + 1,
                c: base + 3,
                material: 0,
            });
            raw.triangles.push(Tri {
                a: base,
                b: base + 3,
                c: base + 2,
                material: 0,
            });
        }
        raw.material_textures.push(String::new());
        raw
    }

    #[test]
    fn pieces_that_only_touch_at_a_corner_count_separately() {
        // The small quad's corner sits exactly on the big quad's corner.
        let touching = two_quad_mesh((40.0, 40.0), 1.0);

        // Welding by position sees one piece, which is why simplification
        // nudging that shared corner used to read as the mesh being torn.
        assert_eq!(lod_geometry_component_count(&touching), 1);
        assert_eq!(lod_connected_pieces(&touching).len(), 2);

        let apart = two_quad_mesh((41.0, 41.0), 1.0);
        assert_eq!(lod_connected_pieces(&apart).len(), 2);
    }

    #[test]
    fn a_shed_sliver_is_tolerated_but_a_shredded_mesh_is_not() {
        // A run of unit quads sharing their vertical edges, so dropping one
        // quad breaks the run in two exactly the way a collapse can.
        const QUADS: u32 = 1000;
        let mut source = RawMesh::default();
        for column in 0..=QUADS {
            for row in [0.0f32, 1.0] {
                source.vertices.push(V3 {
                    x: column as f32,
                    y: row,
                    z: 0.0,
                });
                source.uvs.push(V2 { u: 0.0, v: row });
            }
        }
        for column in 0..QUADS {
            let base = column * 2;
            source.triangles.push(Tri {
                a: base,
                b: base + 1,
                c: base + 3,
                material: 0,
            });
            source.triangles.push(Tri {
                a: base,
                b: base + 3,
                c: base + 2,
                material: 0,
            });
        }
        source.material_textures.push(String::new());
        assert_eq!(lod_connected_pieces(&source).len(), 1);

        let without_quad = |quad: u32| {
            let mut candidate = source.clone();
            candidate
                .triangles
                .drain(quad as usize * 2..quad as usize * 2 + 2);
            candidate
        };

        // Losing the second-to-last quad strands the last one: one part in a
        // thousand, which is a sliver rather than a broken model.
        let sliver = without_quad(QUADS - 2);
        assert_eq!(lod_connected_pieces(&sliver).len(), 2);
        validate_lod_candidate(&source, &sliver).unwrap();

        // Cutting the run down the middle strands half the surface.
        let shredded = without_quad(QUADS / 2);
        assert_eq!(lod_connected_pieces(&shredded).len(), 2);
        let error = validate_lod_candidate(&source, &shredded).unwrap_err();
        assert!(error.contains("additional disconnected piece"), "{error}");
    }

    #[test]
    fn non_manifold_edges_are_budgeted_against_what_the_source_already_had() {
        // Duplicating a triangle in place makes each of its edges non-manifold.
        let with_duplicates = |count: usize| {
            let mut raw = grid_mesh(8);
            let duplicates = raw.triangles[..count].to_vec();
            raw.triangles.extend(duplicates);
            raw
        };
        let clean = grid_mesh(8);
        assert_eq!(lod_edge_topology(&clean).1, 0);

        // A clean source is still held to a clean result.
        let error = validate_lod_candidate(&clean, &with_duplicates(2)).unwrap_err();
        assert!(error.contains("non-manifold"), "{error}");

        // A source that already has them may wobble by a few without losing its
        // LOD entirely.
        let dirty = with_duplicates(4);
        assert!(lod_edge_topology(&dirty).1 >= 4);
        validate_lod_candidate(&dirty, &with_duplicates(5)).unwrap();
    }

    #[test]
    fn lod_simplification_never_joins_disconnected_pieces() {
        let mut source = grid_mesh(8);
        let mut second = grid_mesh(8);
        let index_offset = source.vertices.len() as u32;
        for vertex in &mut second.vertices {
            vertex.x += 100.0;
        }
        source.vertices.extend(second.vertices);
        source.normals.extend(second.normals);
        source.uvs.extend(second.uvs);
        source
            .triangles
            .extend(second.triangles.into_iter().map(|triangle| Tri {
                a: triangle.a + index_offset,
                b: triangle.b + index_offset,
                c: triangle.c + index_offset,
                material: triangle.material,
            }));

        let lod = simplify_lod_mesh(&source).unwrap();
        assert!(lod.vertices.iter().any(|vertex| vertex.x < 50.0));
        assert!(lod.vertices.iter().any(|vertex| vertex.x > 50.0));
        let left = lod
            .vertices
            .iter()
            .filter(|vertex| vertex.x < 50.0)
            .copied()
            .collect::<Vec<_>>();
        let right = lod
            .vertices
            .iter()
            .filter(|vertex| vertex.x > 50.0)
            .copied()
            .collect::<Vec<_>>();
        assert_eq!(
            left.iter()
                .map(|vertex| vertex.x)
                .fold(f32::INFINITY, f32::min),
            0.0
        );
        assert_eq!(
            left.iter()
                .map(|vertex| vertex.x)
                .fold(f32::NEG_INFINITY, f32::max),
            8.0
        );
        assert_eq!(
            right
                .iter()
                .map(|vertex| vertex.x)
                .fold(f32::INFINITY, f32::min),
            100.0
        );
        assert_eq!(
            right
                .iter()
                .map(|vertex| vertex.x)
                .fold(f32::NEG_INFINITY, f32::max),
            108.0
        );
        for triangle in &lod.triangles {
            let xs = [
                lod.vertices[triangle.a as usize].x,
                lod.vertices[triangle.b as usize].x,
                lod.vertices[triangle.c as usize].x,
            ];
            assert!(
                xs.iter().all(|x| *x < 50.0) || xs.iter().all(|x| *x > 50.0),
                "simplification joined disconnected pieces: {xs:?}"
            );
        }
    }

    #[test]
    fn repeated_uvs_are_baked_into_atlas_space() {
        let mut raw = grid_mesh(2);
        raw.material_textures.push("second".to_string());
        raw.triangles[0].material = 1;
        raw.uvs[0].u = 2.0;
        let textures = vec![
            DecodedLodTexture {
                name: "grid".to_string(),
                width: 4,
                height: 4,
                rgba: vec![255; 4 * 4 * 4],
            },
            DecodedLodTexture {
                name: "second".to_string(),
                width: 4,
                height: 4,
                rgba: vec![255; 4 * 4 * 4],
            },
        ];
        assert!(atlas_is_possible(&raw, &textures));
        let source_materials = raw
            .triangles
            .iter()
            .map(|triangle| triangle.material)
            .collect::<Vec<_>>();
        let atlases = remap_raw_for_atlases(&mut raw, &textures).unwrap().textures;
        assert_eq!(atlases.len(), 1);
        // Two 4x4 sources need no more than the smallest sheet.
        assert_eq!(
            (atlases[0].width, atlases[0].height),
            (LOD_MIN_ATLAS_DIMENSION, LOD_MIN_ATLAS_DIMENSION)
        );
        assert_eq!(
            raw.triangles
                .iter()
                .map(|triangle| triangle.material)
                .collect::<Vec<_>>(),
            source_materials
        );
        assert!(
            raw.uvs
                .iter()
                .all(|uv| (0.0..=1.0).contains(&uv.u) && (0.0..=1.0).contains(&uv.v))
        );
    }

    #[test]
    fn heavily_tiled_materials_stay_out_of_the_atlas() {
        assert!(!lod_texture_is_tiled(None));
        assert!(!lod_texture_is_tiled(Some((0.0, 0.0, 1.0, 1.0))));
        // A mild repeat is cheap to bake at its real repeat count.
        assert!(!lod_texture_is_tiled(Some((
            0.0,
            0.0,
            LOD_ATLAS_MAX_BAKED_REPEATS,
            1.0
        ))));
        assert!(lod_texture_is_tiled(Some((0.0, 0.0, 16.0, 1.0))));
        assert!(lod_texture_is_tiled(Some((0.0, 0.0, 1.0, 9.0))));
    }

    #[test]
    fn a_tiled_material_keeps_its_own_texture_and_wrapped_uvs() {
        let mut raw = grid_mesh(2);
        raw.material_textures.push("road".to_string());
        // The first quad's two triangles are the only users of vertex 0.
        raw.triangles[0].material = 1;
        raw.triangles[1].material = 1;
        // Send the tiled material far outside the unit square.
        raw.uvs[0].u = 12.0;
        let source_uvs = raw.uvs.clone();
        let textures = vec![
            DecodedLodTexture {
                name: "grid".to_string(),
                width: 8,
                height: 8,
                rgba: vec![255; 8 * 8 * 4],
            },
            DecodedLodTexture {
                name: "road".to_string(),
                width: 8,
                height: 8,
                rgba: vec![128; 8 * 8 * 4],
            },
        ];

        let generated = remap_raw_for_atlases(&mut raw, &textures).unwrap();

        // One atlas for the ordinary material, one standalone wrapped texture
        // for the tiled one.
        assert_eq!(generated.textures.len(), 2);
        assert_eq!(generated.tiled.len(), 1);
        let tiled_name = generated.tiled.iter().next().unwrap().clone();
        assert_eq!(lower(&raw.material_textures[1]), tiled_name);
        assert_ne!(lower(&raw.material_textures[0]), tiled_name);
        // Its UVs must survive untouched, otherwise the tiling gets stretched
        // across the surface instead of repeating.
        let tiled_vertices = raw
            .triangles
            .iter()
            .filter(|triangle| triangle.material == 1)
            .flat_map(|triangle| [triangle.a, triangle.b, triangle.c]);
        assert!(
            tiled_vertices
                .map(|index| raw.uvs[index as usize])
                .any(|uv| uv.u > 1.0),
            "a wrapped UV must stay outside the unit square"
        );
        assert!(source_uvs.iter().any(|uv| uv.u == 12.0));
    }

    #[test]
    fn the_shared_dictionary_stores_one_copy_of_a_repeated_atlas() {
        let sheet = |name: &str, value: u8| DecodedLodTexture {
            name: name.to_string(),
            width: 8,
            height: 8,
            rgba: vec![value; 8 * 8 * 4],
        };
        // Two LODs of the same building generate the same sheet, so it carries
        // the same content-addressed name both times.
        let shared = build_texture_txd(&[sheet("wall_deadbeef", 70)], &BTreeSet::new()).unwrap();
        let other = build_texture_txd(&[sheet("roof_feedface", 200)], &BTreeSet::new()).unwrap();

        let world_lod = merge_generated_lod_txd(empty_lod_txd(), &shared).unwrap();
        let after_repeat = merge_generated_lod_txd(world_lod.clone(), &shared).unwrap();
        assert_eq!(
            after_repeat, world_lod,
            "a repeated sheet must not grow the dictionary"
        );

        let world_lod = merge_generated_lod_txd(after_repeat, &other).unwrap();
        let contents = parse_txd_texture_contents(&world_lod).unwrap();
        assert_eq!(
            contents
                .iter()
                .map(|texture| texture.name.clone())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["wall_deadbeef".to_string(), "roof_feedface".to_string()])
        );
    }

    #[test]
    fn identical_generated_textures_share_one_name() {
        let pixels = |value: u8| vec![value; 32 * 32 * 4];

        let first = lod_texture_content_name("beach_sand", 32, 32, &pixels(64));
        let same = lod_texture_content_name("beach_sand", 32, 32, &pixels(64));
        let other_pixels = lod_texture_content_name("beach_sand", 32, 32, &pixels(65));
        let other_size = lod_texture_content_name("beach_sand", 32, 16, &pixels(64)[..32 * 16 * 4]);

        assert_eq!(first, same, "identical content must produce one name");
        assert_ne!(first, other_pixels);
        assert_ne!(first, other_size);
        assert!(first.starts_with("beach_sand_"), "{first}");
        assert!(first.len() <= GTA_SA_TEXTURE_NAME_MAX, "{first}");
        assert_eq!(
            lod_texture_content_name("a_very_long_source_texture_name", 4, 4, &[0; 64]).len(),
            GTA_SA_TEXTURE_NAME_MAX
        );
    }

    #[test]
    fn atlas_sheet_shrinks_to_the_crops_it_holds() {
        let crop = |width: f32, height: f32| LodAtlasCrop {
            u_min: 0.0,
            v_min: 0.0,
            u_span: 1.0,
            v_span: 1.0,
            texel_width: width,
            texel_height: height,
        };

        let (width, height, placements) = choose_lod_atlas_size(&[crop(64.0, 64.0)], 2).unwrap();
        // The gutter costs a few texels rather than the next sheet size up.
        assert_eq!((width, height), (64, 64));
        assert_eq!(placements[0], (2, 2, 60, 60));

        // A single tall crop gets a tall sheet rather than a square one that
        // would leave half of its texels unused.
        let (width, height, _) = choose_lod_atlas_size(&[crop(32.0, 128.0)], 2).unwrap();
        assert_eq!((width, height), (32, 128));

        // Content that needs the full sheet still gets it.
        let (width, height, _) = choose_lod_atlas_size(&[crop(500.0, 500.0)], 2).unwrap();
        assert_eq!((width, height), (512, 512));
    }

    #[test]
    fn atlas_packing_keeps_crop_aspect_instead_of_squashing_to_fit() {
        let crop = LodAtlasCrop {
            u_min: 0.0,
            v_min: 0.0,
            u_span: 1.0,
            v_span: 1.0,
            texel_width: 256.0,
            texel_height: 64.0,
        };

        assert!(try_pack_lod_atlas_crops(&[crop], 64, 64, 2, 1.0).is_none());

        let (scale, placements) =
            pack_lod_atlas_crops_at_best_scale(&[crop], 64, 64, 2, 1.0).unwrap();
        assert!(scale < 1.0);
        let (_, _, width, height) = placements[0];
        assert!(
            (width as f32 / height as f32 - 4.0).abs() < 0.5,
            "packed cell {width}x{height} must keep the 4:1 source aspect"
        );
    }

    #[test]
    fn a_truncated_shared_dictionary_keeps_the_textures_that_survived() {
        let texture = |name: &str, value: u8| DecodedLodTexture {
            name: name.to_string(),
            width: 4,
            height: 4,
            rgba: vec![value; 4 * 4 * 4],
        };
        let full = build_texture_txd(
            &[
                texture("lod_first_atlas", 32),
                texture("lod_second_atlas", 96),
                texture("lod_third_atlas", 160),
            ],
            &BTreeSet::new(),
        )
        .unwrap();
        // Cut the asset short the way a wrapped IMG entry size does: the header
        // still advertises the full dictionary.
        let mut cut = full.clone();
        cut.truncate(full.len() * 2 / 3);
        assert!(parse_txd_texture_contents(&cut).is_err());

        let repaired = repair_truncated_lod_txd(cut);
        let contents = parse_txd_texture_contents(&repaired).unwrap();

        assert!(
            !contents.is_empty() && contents.len() < 3,
            "expected a partial but valid dictionary, found {} textures",
            contents.len()
        );
        assert_eq!(contents[0].name, "lod_first_atlas");
        // A healthy dictionary is returned untouched.
        assert_eq!(repair_truncated_lod_txd(full.clone()), full);
        // And the repaired dictionary can take new atlases again.
        let merged = merge_generated_lod_txd(
            repaired,
            &build_texture_txd(&[texture("lod_fourth_atlas", 200)], &BTreeSet::new()).unwrap(),
        )
        .unwrap();
        assert!(
            parse_txd_texture_contents(&merged)
                .unwrap()
                .iter()
                .any(|texture| texture.name == "lod_fourth_atlas")
        );
    }

    #[test]
    fn generated_lod_dictionaries_roll_over_by_name() {
        assert_eq!(world_lod_txd_name(0), WORLD_LOD_TXD_NAME);
        assert_eq!(world_lod_txd_name(1), "world_lod_2.txd");
        assert_eq!(world_lod_txd_name(2), "world_lod_3.txd");
        for index in [0usize, 1, 9, 99] {
            assert!(world_lod_txd_name(index).len() <= IMG_RUNTIME_SAFE_ENTRY_NAME_BYTES);
        }
    }

    #[test]
    fn atlas_crop_uses_only_the_referenced_uv_footprint() {
        let texture = DecodedLodTexture {
            name: "facade".to_string(),
            width: 256,
            height: 128,
            rgba: vec![255; 256 * 128 * 4],
        };
        let crop = lod_atlas_crop(&texture, Some((0.25, 0.50, 0.50, 0.75)));

        assert_eq!(crop.u_min, 0.25);
        assert_eq!(crop.v_min, 0.50);
        assert_eq!(crop.u_span, 0.25);
        assert_eq!(crop.v_span, 0.25);
        assert_eq!(crop.texel_width, 64.0);
        assert_eq!(crop.texel_height, 32.0);
    }

    #[test]
    fn partial_texture_usage_receives_a_proportionally_smaller_atlas_region() {
        let mut raw = RawMesh {
            vertices: vec![
                V3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                V3 {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                V3 {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
                V3 {
                    x: 2.0,
                    y: 0.0,
                    z: 0.0,
                },
                V3 {
                    x: 3.0,
                    y: 0.0,
                    z: 0.0,
                },
                V3 {
                    x: 2.0,
                    y: 1.0,
                    z: 0.0,
                },
            ],
            uvs: vec![
                V2 { u: 0.25, v: 0.25 },
                V2 { u: 0.50, v: 0.25 },
                V2 { u: 0.25, v: 0.50 },
                V2 { u: 0.0, v: 0.0 },
                V2 { u: 1.0, v: 0.0 },
                V2 { u: 0.0, v: 1.0 },
            ],
            triangles: vec![
                Tri {
                    a: 0,
                    b: 1,
                    c: 2,
                    material: 0,
                },
                Tri {
                    a: 3,
                    b: 4,
                    c: 5,
                    material: 1,
                },
            ],
            material_textures: vec!["partial".to_string(), "full".to_string()],
            ..RawMesh::default()
        };
        let textures = [
            DecodedLodTexture {
                name: "partial".to_string(),
                width: 64,
                height: 64,
                rgba: vec![255; 64 * 64 * 4],
            },
            DecodedLodTexture {
                name: "full".to_string(),
                width: 64,
                height: 64,
                rgba: vec![255; 64 * 64 * 4],
            },
        ];

        let atlases = remap_raw_for_atlases(&mut raw, &textures).unwrap().textures;
        assert_eq!(atlases.len(), 1);
        let uv_width = |triangle: &Tri| {
            [triangle.a, triangle.b, triangle.c]
                .into_iter()
                .map(|index| raw.uvs[index as usize].u)
                .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), u| {
                    (min.min(u), max.max(u))
                })
        };
        let partial = uv_width(&raw.triangles[0]);
        let full = uv_width(&raw.triangles[1]);
        let partial_width = partial.1 - partial.0;
        let full_width = full.1 - full.0;
        assert!(
            partial_width < full_width * 0.35,
            "partial footprint {partial_width} should use much less atlas space than {full_width}"
        );
    }

    fn one_triangle_with_uvs(uvs: [V2; 3]) -> RawMesh {
        RawMesh {
            vertices: vec![V3::default(); 3],
            uvs: uvs.to_vec(),
            triangles: vec![Tri {
                a: 0,
                b: 1,
                c: 2,
                material: 2,
            }],
            material_textures: vec![String::new(), String::new(), "atlas_source".to_string()],
            ..RawMesh::default()
        }
    }

    #[test]
    fn atlas_validation_accepts_uv_slivers_below_f32_precision() {
        let source = one_triangle_with_uvs([
            V2 {
                u: -1.4999759,
                v: -4.9994736,
            },
            V2 {
                u: -1.5000192,
                v: -4.7741375,
            },
            V2 {
                u: -1.5000198,
                v: -4.7741375,
            },
        ]);
        let atlased = one_triangle_with_uvs([
            V2 {
                u: 0.70351714,
                v: 0.1875329,
            },
            V2 {
                u: 0.7035144,
                v: 0.2016164,
            },
            V2 {
                u: 0.7035144,
                v: 0.2016164,
            },
        ]);
        let atlas = DecodedLodTexture {
            name: "atlas".to_string(),
            width: 1,
            height: 1,
            rgba: vec![255; 4],
        };
        let scales = HashMap::from([(2, (0.063, 0.0625))]);

        validate_atlas_layout(&source, &atlased, std::slice::from_ref(&atlas), &scales).unwrap();
    }

    #[test]
    fn atlas_validation_still_rejects_representable_polygon_collapse() {
        let source = one_triangle_with_uvs([
            V2 { u: 0.0, v: 0.0 },
            V2 { u: 1.0, v: 0.0 },
            V2 { u: 0.0, v: 1.0 },
        ]);
        let atlased = one_triangle_with_uvs([
            V2 { u: 0.5, v: 0.5 },
            V2 { u: 0.5, v: 0.5 },
            V2 { u: 0.5, v: 0.5 },
        ]);
        let atlas = DecodedLodTexture {
            name: "atlas".to_string(),
            width: 1,
            height: 1,
            rgba: vec![255; 4],
        };
        let scales = HashMap::from([(2, (0.25, 0.25))]);

        let error = validate_atlas_layout(&source, &atlased, std::slice::from_ref(&atlas), &scales)
            .unwrap_err();
        assert!(error.contains("collapsed, mirrored, or rotated"), "{error}");
    }

    #[test]
    fn a_single_source_texture_is_still_remapped_to_one_atlas() {
        let mut raw = grid_mesh(2);
        let source_uvs = raw.uvs.clone();
        let texture = DecodedLodTexture {
            name: "grid".to_string(),
            width: 16,
            height: 8,
            rgba: vec![255; 16 * 8 * 4],
        };
        assert!(atlas_is_possible(&raw, std::slice::from_ref(&texture)));
        let atlases = remap_raw_for_atlases(&mut raw, &[texture])
            .unwrap()
            .textures;
        assert_eq!(atlases.len(), 1);
        assert_eq!(
            (atlases[0].width, atlases[0].height),
            (LOD_MIN_ATLAS_DIMENSION, LOD_MIN_ATLAS_DIMENSION)
        );
        assert_eq!(raw.material_textures, vec![atlases[0].name.clone()]);
        assert!(
            atlases[0].name.starts_with("grid_"),
            "generated names stay browsable: {}",
            atlases[0].name
        );
        assert_ne!(raw.uvs, source_uvs);
        assert!(
            atlases[0].rgba.chunks_exact(4).all(|pixel| pixel[3] == 255),
            "unused atlas pixels must not introduce synthetic transparency"
        );
        let txd = build_texture_txd(&atlases, &BTreeSet::new()).unwrap();
        let contents = parse_txd_texture_contents(&txd).unwrap();
        assert_eq!(contents.len(), 1);
        assert_eq!(
            (contents[0].width as u32, contents[0].height as u32),
            (LOD_MIN_ATLAS_DIMENSION, LOD_MIN_ATLAS_DIMENSION)
        );
        assert!(!contents[0].has_alpha);
    }

    #[test]
    fn shared_world_lod_txd_accumulates_each_generated_atlas() {
        let texture = |name: &str, value: u8| DecodedLodTexture {
            name: name.to_string(),
            width: 4,
            height: 4,
            rgba: vec![value; 4 * 4 * 4],
        };
        let first = build_texture_txd(&[texture("lod_first_atlas", 64)], &BTreeSet::new()).unwrap();
        let second =
            build_texture_txd(&[texture("lod_second_atlas", 192)], &BTreeSet::new()).unwrap();

        let world_lod = merge_generated_lod_txd(empty_lod_txd(), &first).unwrap();
        let world_lod = merge_generated_lod_txd(world_lod, &second).unwrap();
        let contents = parse_txd_texture_contents(&world_lod).unwrap();

        assert_eq!(
            contents
                .iter()
                .map(|texture| texture.name.as_str())
                .collect::<Vec<_>>(),
            vec!["lod_first_atlas", "lod_second_atlas"]
        );
    }

    #[test]
    fn opaque_and_alpha_materials_use_separate_texture_atlases() {
        let mut raw = grid_mesh(2);
        raw.material_textures.push("second".to_string());
        for triangle in raw.triangles.iter_mut().take(4) {
            triangle.material = 1;
        }
        let textures = vec![
            DecodedLodTexture {
                name: "grid".to_string(),
                width: 8,
                height: 8,
                rgba: vec![255; 8 * 8 * 4],
            },
            DecodedLodTexture {
                name: "second".to_string(),
                width: 8,
                height: 8,
                rgba: vec![127; 8 * 8 * 4],
            },
        ];
        assert!(atlas_is_possible(&raw, &textures));
        let atlases = remap_raw_for_atlases(&mut raw, &textures).unwrap().textures;
        assert_eq!(atlases.len(), 2);
        assert!(
            atlases
                .iter()
                .all(|atlas| atlas.width <= LOD_MAX_ATLAS_DIMENSION
                    && atlas.height <= LOD_MAX_ATLAS_DIMENSION)
        );
        assert_eq!(
            raw.material_textures,
            vec![atlases[0].name.clone(), atlases[1].name.clone()],
            "the opaque sheet is emitted before the alpha one"
        );
        assert!(
            raw.triangles[..4]
                .iter()
                .all(|triangle| triangle.material == 0)
        );
        assert!(
            raw.triangles[4..]
                .iter()
                .all(|triangle| triangle.material == 1)
        );
        assert!(
            raw.uvs
                .iter()
                .all(|uv| (0.0..=1.0).contains(&uv.u) && (0.0..=1.0).contains(&uv.v))
        );

        let txd = build_texture_txd(&atlases, &BTreeSet::new()).unwrap();
        let contents = parse_txd_texture_contents(&txd).unwrap();
        assert_eq!(contents.len(), 2);
        assert!(
            contents
                .iter()
                .any(|texture| texture.name == atlases[0].name && !texture.has_alpha)
        );
        assert!(
            contents
                .iter()
                .any(|texture| texture.name == atlases[1].name && texture.has_alpha)
        );

        let dff = write_normalized_dff(&raw, "lod_model").unwrap();
        let reparsed = parse_dff_mesh(&dff);
        assert_eq!(
            reparsed.material_textures,
            vec![atlases[0].name.clone(), atlases[1].name.clone()]
        );
        assert_eq!(reparsed.triangles.len(), raw.triangles.len());
        assert!(
            reparsed.triangles[..4]
                .iter()
                .all(|triangle| triangle.material == 0)
        );
        assert!(
            reparsed.triangles[4..]
                .iter()
                .all(|triangle| triangle.material == 1)
        );
    }

    #[test]
    fn bundled_dff_and_txd_generate_valid_lod_assets() {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/player_vehicle");
        let dff = fs::read(assets.join("player_1.dff")).unwrap();
        let txd = fs::read(assets.join("player_1.txd")).unwrap();
        let source = parse_dff_mesh(&dff);
        let mut lod = simplify_lod_mesh(&source).unwrap();
        assert!(
            lod.triangles.len() * 10 <= source.triangles.len() * 7,
            "expected at least a 30% validated reduction, got {} of {} triangles",
            lod.triangles.len(),
            source.triangles.len()
        );
        assert_same_bounds(&source, &lod);
        validate_lod_candidate(&source, &lod).unwrap();
        let mut textures = decode_lod_textures(&lod, "player_1.txd", &txd).unwrap();
        assert!(!textures.is_empty());
        let base_textures = lod
            .triangles
            .iter()
            .filter_map(|triangle| {
                lod.material_textures
                    .get(triangle.material as usize)
                    .map(|name| lower(name.trim()))
            })
            .filter(|name| !name.is_empty())
            .collect::<BTreeSet<_>>();
        textures.retain(|texture| base_textures.contains(&lower(&texture.name)));
        assert!(atlas_is_possible(&lod, &textures));
        let atlases = remap_raw_for_atlases(&mut lod, &textures).unwrap().textures;
        assert!(atlases.len() <= 2);
        assert!(
            atlases
                .iter()
                .all(|atlas| atlas.width <= LOD_MAX_ATLAS_DIMENSION
                    && atlas.height <= LOD_MAX_ATLAS_DIMENSION)
        );
        let lod_txd = build_texture_txd(&atlases, &BTreeSet::new()).unwrap();
        let contents = parse_txd_texture_contents(&lod_txd).unwrap();
        assert_eq!(contents.len(), atlases.len());
        let lod_dff = write_normalized_dff(&lod, "lod_player_1").unwrap();
        let reparsed = parse_dff_mesh(&lod_dff);
        assert!(!reparsed.vertices.is_empty());
        assert!(!reparsed.triangles.is_empty());
        assert!(reparsed.triangles.len() <= source.triangles.len());
        validate_lod_candidate(&lod, &reparsed).unwrap();
    }

    #[test]
    fn bundled_vehicle_keeps_bounds_without_topology_spikes() {
        let dff = fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/player_vehicle/vehicle.dff"),
        )
        .unwrap();
        let source = parse_dff_mesh(&dff);
        let lod = simplify_lod_mesh(&source).unwrap();
        assert!(
            lod.triangles.len() * 5 <= source.triangles.len() * 2,
            "expected at least a 60% validated reduction, got {} of {} triangles",
            lod.triangles.len(),
            source.triangles.len()
        );
        assert_same_bounds(&source, &lod);
        validate_lod_candidate(&source, &lod).unwrap();
    }

    #[test]
    fn batch_requests_generate_unique_lod_asset_sets_for_each_element() {
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let project_root =
            env::temp_dir().join(format!("eagle_lod_batch_{}_{}", std::process::id(), nonce));
        let wip_root = wip_root_path(&project_root);
        fs::create_dir_all(&project_root).unwrap();
        let reserved_stems = Arc::new(BTreeSet::new());
        let request = |placement_index| LodGenerationRequest {
            mesh_source: LodMeshSource::Raw(grid_mesh(5)),
            txd_source: None,
            output_stem: "lod_nbmiamiland048".to_string(),
            attach_to_placement: Some(placement_index),
            source_definition_id: Some(format!("source_{placement_index}")),
            source_zone: "test".to_string(),
            source_dff_name: "shared.dff".to_string(),
            source_txd_name: None,
            replaced_lod_parent: None,
            replaced_lod_index: None,
            project_root: project_root.clone(),
            reserved_stems: Arc::clone(&reserved_stems),
            wip_root: wip_root.clone(),
            stem_preallocated: false,
        };

        let mut requests = vec![request(3), request(7)];
        assert!(Arc::ptr_eq(
            &requests[0].reserved_stems,
            &requests[1].reserved_stems
        ));
        reserve_batch_lod_stems(&mut requests);
        assert!(Arc::ptr_eq(
            &requests[0].reserved_stems,
            &requests[1].reserved_stems
        ));
        assert_ne!(requests[0].output_stem, requests[1].output_stem);
        let second_request = requests.pop().unwrap();
        let first_request = requests.pop().unwrap();
        let first_worker = thread::spawn(move || generate_lod(first_request));
        let second_worker = thread::spawn(move || generate_lod(second_request));
        let first = first_worker.join().unwrap().unwrap();
        let second = second_worker.join().unwrap().unwrap();
        write_lod_result_to_wip(&wip_root, &first).unwrap();
        write_lod_result_to_wip(&wip_root, &second).unwrap();
        assert_ne!(first.stem, second.stem);
        assert!(first.dff_name.len() <= IMG_RUNTIME_SAFE_ENTRY_NAME_BYTES);
        assert!(second.dff_name.len() <= IMG_RUNTIME_SAFE_ENTRY_NAME_BYTES);
        assert!(second.col_name.len() <= IMG_RUNTIME_SAFE_ENTRY_NAME_BYTES);
        assert_eq!(first.attach_to_placement, Some(3));
        assert_eq!(second.attach_to_placement, Some(7));
        assert!(replacement_img_entry(&wip_root, &first.dff_name).is_some());
        assert!(replacement_img_entry(&wip_root, &second.dff_name).is_some());
        assert!(replacement_img_entry(&wip_root, &first.col_name).is_some());
        assert!(replacement_img_entry(&wip_root, &second.col_name).is_some());
        assert!(first.col_mesh.faces.is_empty());
        assert!(first.col_mesh.boxes.is_empty());
        assert!(first.col_mesh.spheres.is_empty());
        assert!(first.col_mesh.bounds == bounds_from_vertices(&first.raw.vertices));
        assert!(!col_validation_has_errors(&validate_col_for_game_load(
            &first.col_name,
            &first.col_bytes
        )));

        fs::remove_dir_all(project_root).unwrap();
    }

    #[test]
    fn large_batch_name_reservation_is_unique() {
        let reserved_stems = Arc::new(BTreeSet::new());
        let mut requests = (0..2_000usize)
            .map(|index| LodGenerationRequest {
                mesh_source: LodMeshSource::Raw(grid_mesh(2)),
                txd_source: None,
                output_stem: "lod_repeated_model".to_string(),
                attach_to_placement: Some(index),
                source_definition_id: None,
                source_zone: String::new(),
                source_dff_name: "repeated_model.dff".to_string(),
                source_txd_name: None,
                replaced_lod_parent: None,
                replaced_lod_index: None,
                project_root: PathBuf::new(),
                reserved_stems: Arc::clone(&reserved_stems),
                wip_root: PathBuf::new(),
                stem_preallocated: false,
            })
            .collect::<Vec<_>>();

        reserve_batch_lod_stems(&mut requests);

        let names = requests
            .iter()
            .map(|request| request.output_stem.as_str())
            .collect::<HashSet<_>>();
        assert_eq!(names.len(), requests.len());
        assert!(requests.iter().all(|request| request.stem_preallocated));
    }
}
