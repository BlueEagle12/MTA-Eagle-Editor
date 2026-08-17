//! Worker-safe DFF-to-COL geometry generation.
//!
//! This module deliberately has no `AppState`, rendering, filesystem, or UI
//! dependencies. Callers can clone a `RawMesh` and settings on the main thread,
//! run [`generate_collision`] on a worker, then apply the returned value later.

use super::super::*;
use meshopt::{SimplifyOptions, VertexDataAdapter, simplify, simplify_with_locks, typed_to_bytes};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::mem;
use std::path::PathBuf;

const COL_VERTEX_SCALE: f32 = 128.0;
const COL_VERTEX_STEP: f32 = 1.0 / COL_VERTEX_SCALE;
const NORMAL_EPSILON: f32 = 1.0e-8;
const MIN_PRIMITIVE_FACES: usize = 8;
const FLAT_COMPONENT_MAX_THICKNESS: f32 = COL_VERTEX_STEP * 2.0;
const FLAT_COMPONENT_NORMAL_DOT: f32 = 0.995;
const FLAT_BACKING_NORMAL_DOT: f32 = 0.985;
const FLAT_BACKING_MIN_GAP: f32 = COL_VERTEX_STEP * 0.5;
const FLAT_BACKING_MAX_GAP: f32 = 0.05;
const FLAT_BACKING_MIN_AREA_RATIO: f32 = 1.5;
const FLAT_BACKING_MIN_COVERAGE: f32 = 0.90;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum CollisionGenerationPreset {
    #[default]
    Auto,
    Architecture,
    Prop,
    Shapes,
    Surface,
}

/// All data required by the worker. `source_materials` is indexed by the DFF
/// material slot and contains the already-resolved COL material, if any.
#[derive(Clone, Debug)]
pub(crate) struct CollisionGenerationSettings {
    pub(crate) fallback_material: u8,
    pub(crate) source_materials: Vec<Option<u8>>,
    excluded_source_materials: BTreeSet<u16>,
    max_error: f32,
    target_face_floor: usize,
    primitive_relative_tolerance: f32,
    box_min_surface_coverage: f32,
    sphere_max_radial_error: f32,
    sphere_min_surface_coverage: f32,
    primitive_material_dominance: f32,
    min_faces_for_simplification: usize,
    allow_boxes: bool,
    allow_spheres: bool,
    force_shapes: bool,
    preserve_mesh_boundaries: bool,
    empty_collision: bool,
}

impl CollisionGenerationSettings {
    pub(crate) fn for_preset(
        preset: CollisionGenerationPreset,
        fallback_material: u8,
        source_materials: Vec<Option<u8>>,
    ) -> Self {
        let mut settings = match preset {
            CollisionGenerationPreset::Auto => Self {
                fallback_material,
                source_materials,
                excluded_source_materials: BTreeSet::new(),
                max_error: 0.08,
                target_face_floor: 4,
                primitive_relative_tolerance: 0.015,
                box_min_surface_coverage: 0.90,
                sphere_max_radial_error: 0.035,
                sphere_min_surface_coverage: 0.78,
                primitive_material_dominance: 0.999,
                min_faces_for_simplification: 12,
                allow_boxes: true,
                allow_spheres: true,
                force_shapes: false,
                preserve_mesh_boundaries: true,
                empty_collision: false,
            },
            CollisionGenerationPreset::Architecture => Self {
                fallback_material,
                source_materials,
                excluded_source_materials: BTreeSet::new(),
                max_error: 0.04,
                target_face_floor: 4,
                primitive_relative_tolerance: 0.008,
                box_min_surface_coverage: 0.96,
                sphere_max_radial_error: 0.025,
                sphere_min_surface_coverage: 0.85,
                primitive_material_dominance: 0.999,
                min_faces_for_simplification: 12,
                allow_boxes: true,
                allow_spheres: true,
                force_shapes: false,
                preserve_mesh_boundaries: true,
                empty_collision: false,
            },
            CollisionGenerationPreset::Prop => Self {
                fallback_material,
                source_materials,
                excluded_source_materials: BTreeSet::new(),
                max_error: 0.12,
                target_face_floor: 4,
                primitive_relative_tolerance: 0.025,
                box_min_surface_coverage: 0.84,
                sphere_max_radial_error: 0.055,
                sphere_min_surface_coverage: 0.70,
                primitive_material_dominance: 0.999,
                min_faces_for_simplification: 8,
                allow_boxes: true,
                allow_spheres: true,
                force_shapes: false,
                preserve_mesh_boundaries: true,
                empty_collision: false,
            },
            CollisionGenerationPreset::Shapes => Self {
                fallback_material,
                source_materials,
                excluded_source_materials: BTreeSet::new(),
                max_error: 0.12,
                target_face_floor: 4,
                primitive_relative_tolerance: 0.025,
                box_min_surface_coverage: 0.84,
                sphere_max_radial_error: 0.055,
                sphere_min_surface_coverage: 0.70,
                // A forced shape can cover source faces with different
                // materials, so use the largest-area material for its surface.
                primitive_material_dominance: 0.0,
                min_faces_for_simplification: 8,
                allow_boxes: true,
                allow_spheres: true,
                force_shapes: true,
                preserve_mesh_boundaries: true,
                empty_collision: false,
            },
            CollisionGenerationPreset::Surface => Self {
                fallback_material,
                source_materials,
                excluded_source_materials: BTreeSet::new(),
                max_error: 0.12,
                target_face_floor: 4,
                primitive_relative_tolerance: 0.0,
                box_min_surface_coverage: 1.0,
                sphere_max_radial_error: 0.0,
                sphere_min_surface_coverage: 1.0,
                primitive_material_dominance: 1.0,
                min_faces_for_simplification: 8,
                allow_boxes: false,
                allow_spheres: false,
                force_shapes: false,
                preserve_mesh_boundaries: true,
                empty_collision: false,
            },
        };
        settings.max_error = settings.max_error.max(COL_VERTEX_STEP);
        settings
    }

    pub(crate) fn with_excluded_source_materials(
        mut self,
        excluded_source_materials: BTreeSet<u16>,
    ) -> Self {
        self.excluded_source_materials = excluded_source_materials;
        self
    }

    pub(crate) fn with_empty_collision(mut self, empty_collision: bool) -> Self {
        self.empty_collision = empty_collision;
        self
    }

    fn collision_material(&self, source_slot: u16) -> u8 {
        self.source_materials
            .get(source_slot as usize)
            .copied()
            .flatten()
            .unwrap_or(self.fallback_material)
    }
}

impl Default for CollisionGenerationSettings {
    fn default() -> Self {
        Self::for_preset(CollisionGenerationPreset::Auto, 0, Vec::new())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GeneratedPrimitiveKind {
    Box,
    Sphere,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SourceMaterialCoverage {
    pub(crate) source_material_slot: u16,
    pub(crate) area: f32,
    pub(crate) fraction: f32,
}

/// Provenance for one generated mesh face. The output face at the same index
/// inherited this DFF material slot, even if mesh simplification changed its
/// vertex triplet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CollisionSourceProvenance {
    pub(crate) source_material_slot: u16,
    pub(crate) source_component: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GeneratedPrimitiveProvenance {
    pub(crate) kind: GeneratedPrimitiveKind,
    /// Index in `mesh.boxes` or `mesh.spheres`, according to `kind`.
    pub(crate) primitive_index: usize,
    pub(crate) source_component: usize,
    pub(crate) source_materials: Vec<SourceMaterialCoverage>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CollisionGenerationStats {
    pub(crate) source_vertices: usize,
    pub(crate) source_faces: usize,
    pub(crate) sanitized_vertices: usize,
    pub(crate) sanitized_faces: usize,
    pub(crate) components: usize,
    pub(crate) discarded_backed_flat_components: usize,
    pub(crate) boxes: usize,
    pub(crate) spheres: usize,
    pub(crate) mesh_vertices: usize,
    pub(crate) mesh_faces: usize,
}

#[derive(Clone, PartialEq)]
pub(crate) struct CollisionGenerationResult {
    pub(crate) mesh: CollisionMesh,
    /// Parallel to `mesh.faces`.
    pub(crate) face_provenance: Vec<CollisionSourceProvenance>,
    pub(crate) primitive_provenance: Vec<GeneratedPrimitiveProvenance>,
    pub(crate) stats: CollisionGenerationStats,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CollisionGenerationError {
    NoUsableGeometry,
    VertexOutsideColRange { vertex: usize },
    TooManyVertices { count: usize },
    TooManyFaces { count: usize },
    Simplifier(String),
}

impl fmt::Display for CollisionGenerationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoUsableGeometry => write!(f, "DFF has no usable collision geometry"),
            Self::VertexOutsideColRange { vertex } => write!(
                f,
                "DFF vertex {vertex} is outside the COL signed 16-bit fixed-point range"
            ),
            Self::TooManyVertices { count } => write!(
                f,
                "generated collision has {count} vertices, exceeding the {} vertex limit",
                u16::MAX
            ),
            Self::TooManyFaces { count } => write!(
                f,
                "generated collision has {count} faces, exceeding the {} face limit",
                u16::MAX
            ),
            Self::Simplifier(message) => write!(f, "collision simplification failed: {message}"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct SanitizedFace {
    indices: [u32; 3],
    source_material: u16,
    source_component: usize,
}

#[derive(Default)]
struct SanitizedMesh {
    vertices: Vec<V3>,
    faces: Vec<SanitizedFace>,
}

#[derive(Clone, Debug)]
struct FaceComponent {
    faces: Vec<usize>,
    source_component: usize,
}

#[derive(Clone, Copy)]
struct PrimitiveFit {
    kind: GeneratedPrimitiveKind,
    min: V3,
    max: V3,
    center: V3,
    radius: f32,
    material: u8,
}

#[derive(Clone, Copy, Debug)]
struct UnionFindNode {
    parent: usize,
    rank: u8,
}

fn union_find_root(nodes: &mut [UnionFindNode], index: usize) -> usize {
    let parent = nodes[index].parent;
    if parent == index {
        index
    } else {
        let root = union_find_root(nodes, parent);
        nodes[index].parent = root;
        root
    }
}

fn union_find_join(nodes: &mut [UnionFindNode], a: usize, b: usize) {
    let mut a = union_find_root(nodes, a);
    let mut b = union_find_root(nodes, b);
    if a == b {
        return;
    }
    if nodes[a].rank < nodes[b].rank {
        mem::swap(&mut a, &mut b);
    }
    nodes[b].parent = a;
    if nodes[a].rank == nodes[b].rank {
        nodes[a].rank = nodes[a].rank.saturating_add(1);
    }
}

fn source_component_for_triangle(raw: &RawMesh, triangle: usize) -> usize {
    raw.components
        .iter()
        .position(|component| triangle >= component.tri_start && triangle < component.tri_end)
        .map_or(0, |index| index + 1)
}

fn quantize_vertex(
    vertex: V3,
    source_index: usize,
) -> Result<(V3, [i16; 3]), CollisionGenerationError> {
    let values = [vertex.x, vertex.y, vertex.z];
    let mut fixed = [0i16; 3];
    let mut quantized = [0.0f32; 3];
    for axis in 0..3 {
        let scaled = (values[axis] * COL_VERTEX_SCALE).round();
        // RenderWare COL vertices use signed 16-bit 1/128 coordinates. Large
        // map chunks commonly end exactly at +256, one fixed-point unit above
        // the positive maximum (255.9921875). Clamp that single boundary unit
        // instead of discarding the entire collision; values farther outside
        // the representable volume still indicate a genuinely invalid chunk.
        let min = i16::MIN as f32;
        let max = i16::MAX as f32;
        if !scaled.is_finite() || scaled < min - 1.0 || scaled > max + 1.0 {
            return Err(CollisionGenerationError::VertexOutsideColRange {
                vertex: source_index,
            });
        }
        fixed[axis] = scaled.clamp(min, max) as i16;
        quantized[axis] = fixed[axis] as f32 / COL_VERTEX_SCALE;
    }
    Ok((
        V3 {
            x: quantized[0],
            y: quantized[1],
            z: quantized[2],
        },
        fixed,
    ))
}

fn sanitize(
    raw: &RawMesh,
    excluded_source_materials: &BTreeSet<u16>,
) -> Result<SanitizedMesh, CollisionGenerationError> {
    let mut out = SanitizedMesh::default();
    let mut position_to_vertex = BTreeMap::<[i16; 3], u32>::new();
    let mut source_to_vertex = BTreeMap::<usize, u32>::new();
    let mut seen_faces = BTreeSet::<[u32; 3]>::new();

    for (triangle_index, triangle) in raw.triangles.iter().enumerate() {
        if excluded_source_materials.contains(&triangle.material) {
            continue;
        }
        let source_indices = [
            triangle.a as usize,
            triangle.b as usize,
            triangle.c as usize,
        ];
        if source_indices
            .iter()
            .any(|&index| index >= raw.vertices.len())
        {
            continue;
        }
        let mut indices = [0u32; 3];
        for (corner, source_index) in source_indices.into_iter().enumerate() {
            let index = if let Some(index) = source_to_vertex.get(&source_index) {
                *index
            } else {
                let (position, fixed) = quantize_vertex(raw.vertices[source_index], source_index)?;
                let index = if let Some(index) = position_to_vertex.get(&fixed) {
                    *index
                } else {
                    let index = out.vertices.len() as u32;
                    out.vertices.push(position);
                    position_to_vertex.insert(fixed, index);
                    index
                };
                source_to_vertex.insert(source_index, index);
                index
            };
            indices[corner] = index;
        }
        if indices[0] == indices[1] || indices[1] == indices[2] || indices[2] == indices[0] {
            continue;
        }
        let mut canonical = indices;
        canonical.sort_unstable();
        if !seen_faces.insert(canonical) {
            continue;
        }
        let [a, b, c] = indices.map(|index| out.vertices[index as usize]);
        if triangle_area(a, b, c) <= NORMAL_EPSILON {
            continue;
        }
        out.faces.push(SanitizedFace {
            indices,
            source_material: triangle.material,
            source_component: source_component_for_triangle(raw, triangle_index),
        });
    }
    // Degenerate or duplicate source faces may have caused vertices to be
    // interned before the face was rejected. Compact them here so the
    // sanitized statistics and every later limit check describe real geometry.
    let mut remap = BTreeMap::<u32, u32>::new();
    let mut compact_vertices = Vec::new();
    for face in &mut out.faces {
        for index in &mut face.indices {
            let compact = if let Some(compact) = remap.get(index) {
                *compact
            } else {
                let compact = compact_vertices.len() as u32;
                compact_vertices.push(out.vertices[*index as usize]);
                remap.insert(*index, compact);
                compact
            };
            *index = compact;
        }
    }
    out.vertices = compact_vertices;
    Ok(out)
}

fn split_components(mesh: &SanitizedMesh) -> Vec<FaceComponent> {
    let mut nodes: Vec<_> = (0..mesh.faces.len())
        .map(|index| UnionFindNode {
            parent: index,
            rank: 0,
        })
        .collect();
    let mut first_face_for_edge = BTreeMap::<(usize, u32, u32), usize>::new();

    for (face_index, face) in mesh.faces.iter().enumerate() {
        let [a, b, c] = face.indices;
        for (u, v) in [(a, b), (b, c), (c, a)] {
            let edge = (face.source_component, u.min(v), u.max(v));
            if let Some(other) = first_face_for_edge.get(&edge).copied() {
                union_find_join(&mut nodes, face_index, other);
            } else {
                first_face_for_edge.insert(edge, face_index);
            }
        }
    }

    let mut groups = BTreeMap::<usize, Vec<usize>>::new();
    for face_index in 0..mesh.faces.len() {
        let root = union_find_root(&mut nodes, face_index);
        groups.entry(root).or_default().push(face_index);
    }
    groups
        .into_values()
        .map(|faces| FaceComponent {
            source_component: mesh.faces[faces[0]].source_component,
            faces,
        })
        .collect()
}

fn v3_sub(a: V3, b: V3) -> V3 {
    V3 {
        x: a.x - b.x,
        y: a.y - b.y,
        z: a.z - b.z,
    }
}

fn v3_cross(a: V3, b: V3) -> V3 {
    V3 {
        x: a.y * b.z - a.z * b.y,
        y: a.z * b.x - a.x * b.z,
        z: a.x * b.y - a.y * b.x,
    }
}

fn v3_dot(a: V3, b: V3) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

fn v3_length(v: V3) -> f32 {
    (v.x * v.x + v.y * v.y + v.z * v.z).sqrt()
}

fn triangle_area(a: V3, b: V3, c: V3) -> f32 {
    v3_length(v3_cross(v3_sub(b, a), v3_sub(c, a))) * 0.5
}

fn component_bounds(mesh: &SanitizedMesh, component: &FaceComponent) -> (V3, V3) {
    let first = mesh.faces[component.faces[0]].indices[0] as usize;
    let mut min = mesh.vertices[first];
    let mut max = min;
    for &face_index in &component.faces {
        for &vertex_index in &mesh.faces[face_index].indices {
            let vertex = mesh.vertices[vertex_index as usize];
            min.x = min.x.min(vertex.x);
            min.y = min.y.min(vertex.y);
            min.z = min.z.min(vertex.z);
            max.x = max.x.max(vertex.x);
            max.y = max.y.max(vertex.y);
            max.z = max.z.max(vertex.z);
        }
    }
    (min, max)
}

fn component_is_closed(mesh: &SanitizedMesh, component: &FaceComponent) -> bool {
    let mut edge_counts = BTreeMap::<(u32, u32), usize>::new();
    for &face_index in &component.faces {
        let [a, b, c] = mesh.faces[face_index].indices;
        for (u, v) in [(a, b), (b, c), (c, a)] {
            *edge_counts.entry((u.min(v), u.max(v))).or_default() += 1;
        }
    }
    !edge_counts.is_empty() && edge_counts.values().all(|&count| count == 2)
}

#[derive(Clone, Copy)]
struct ComponentGeometry {
    min: V3,
    max: V3,
    area: f32,
    plane: Option<(V3, V3)>,
    closed: bool,
}

#[derive(Clone, Copy)]
struct BackingTriangle {
    vertices: [V3; 3],
    normal: V3,
}

fn normalized(v: V3) -> Option<V3> {
    let length = v3_length(v);
    (length > NORMAL_EPSILON).then_some(V3 {
        x: v.x / length,
        y: v.y / length,
        z: v.z / length,
    })
}

fn component_geometry(mesh: &SanitizedMesh, component: &FaceComponent) -> ComponentGeometry {
    let (min, max) = component_bounds(mesh, component);
    let origin = mesh.vertices[mesh.faces[component.faces[0]].indices[0] as usize];
    let mut reference_normal = None;
    let mut normal_sum = V3::default();
    let mut area = 0.0f32;
    let mut normals_agree = true;
    for &face_index in &component.faces {
        let [a, b, c] = mesh.faces[face_index]
            .indices
            .map(|index| mesh.vertices[index as usize]);
        let cross = v3_cross(v3_sub(b, a), v3_sub(c, a));
        let Some(mut face_normal) = normalized(cross) else {
            continue;
        };
        let reference = *reference_normal.get_or_insert(face_normal);
        if v3_dot(face_normal, reference) < 0.0 {
            face_normal = V3 {
                x: -face_normal.x,
                y: -face_normal.y,
                z: -face_normal.z,
            };
        }
        if v3_dot(face_normal, reference) < FLAT_COMPONENT_NORMAL_DOT {
            normals_agree = false;
        }
        let face_area = triangle_area(a, b, c);
        area += face_area;
        normal_sum.x += face_normal.x * face_area;
        normal_sum.y += face_normal.y * face_area;
        normal_sum.z += face_normal.z * face_area;
    }
    let plane = reference_normal
        .filter(|_| normals_agree)
        .and_then(|_| normalized(normal_sum))
        .filter(|normal| {
            component.faces.iter().all(|&face_index| {
                mesh.faces[face_index].indices.iter().all(|&vertex_index| {
                    v3_dot(
                        v3_sub(mesh.vertices[vertex_index as usize], origin),
                        *normal,
                    )
                    .abs()
                        <= FLAT_COMPONENT_MAX_THICKNESS
                })
            })
        })
        .map(|normal| (origin, normal));
    ComponentGeometry {
        min,
        max,
        area,
        plane,
        closed: component_is_closed(mesh, component),
    }
}

fn aabb_intersects_with_margin(a: &ComponentGeometry, b: &ComponentGeometry, margin: f32) -> bool {
    a.max.x + margin >= b.min.x
        && b.max.x + margin >= a.min.x
        && a.max.y + margin >= b.min.y
        && b.max.y + margin >= a.min.y
        && a.max.z + margin >= b.min.z
        && b.max.z + margin >= a.min.z
}

fn projection_axes(normal: V3) -> (usize, usize) {
    let absolute = [normal.x.abs(), normal.y.abs(), normal.z.abs()];
    let dropped = if absolute[0] >= absolute[1] && absolute[0] >= absolute[2] {
        0
    } else if absolute[1] >= absolute[2] {
        1
    } else {
        2
    };
    match dropped {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    }
}

fn axis_value(vertex: V3, axis: usize) -> f32 {
    match axis {
        0 => vertex.x,
        1 => vertex.y,
        _ => vertex.z,
    }
}

fn projected_bounds_contained(
    candidate: ComponentGeometry,
    backing: ComponentGeometry,
    normal: V3,
) -> bool {
    let axes = projection_axes(normal);
    let candidate_min = [
        axis_value(candidate.min, axes.0),
        axis_value(candidate.min, axes.1),
    ];
    let candidate_max = [
        axis_value(candidate.max, axes.0),
        axis_value(candidate.max, axes.1),
    ];
    let backing_min = [
        axis_value(backing.min, axes.0),
        axis_value(backing.min, axes.1),
    ];
    let backing_max = [
        axis_value(backing.max, axes.0),
        axis_value(backing.max, axes.1),
    ];
    (0..2).all(|axis| {
        candidate_min[axis] + COL_VERTEX_STEP >= backing_min[axis]
            && candidate_max[axis] - COL_VERTEX_STEP <= backing_max[axis]
    })
}

fn point_in_triangle(point: V3, a: V3, b: V3, c: V3) -> bool {
    let v0 = v3_sub(c, a);
    let v1 = v3_sub(b, a);
    let v2 = v3_sub(point, a);
    let dot00 = v3_dot(v0, v0);
    let dot01 = v3_dot(v0, v1);
    let dot02 = v3_dot(v0, v2);
    let dot11 = v3_dot(v1, v1);
    let dot12 = v3_dot(v1, v2);
    let denominator = dot00 * dot11 - dot01 * dot01;
    if denominator.abs() <= NORMAL_EPSILON {
        return false;
    }
    let u = (dot11 * dot02 - dot01 * dot12) / denominator;
    let v = (dot00 * dot12 - dot01 * dot02) / denominator;
    const BARYCENTRIC_TOLERANCE: f32 = 1.0e-4;
    u >= -BARYCENTRIC_TOLERANCE
        && v >= -BARYCENTRIC_TOLERANCE
        && u + v <= 1.0 + BARYCENTRIC_TOLERANCE
}

fn point_has_close_parallel_backing(backing: &[BackingTriangle], point: V3) -> bool {
    backing.iter().any(|triangle| {
        let [a, b, c] = triangle.vertices;
        let normal = triangle.normal;
        let signed_gap = v3_dot(v3_sub(point, a), normal);
        let gap = signed_gap.abs();
        if !(FLAT_BACKING_MIN_GAP..=FLAT_BACKING_MAX_GAP).contains(&gap) {
            return false;
        }
        let projected = V3 {
            x: point.x - normal.x * signed_gap,
            y: point.y - normal.y * signed_gap,
            z: point.z - normal.z * signed_gap,
        };
        point_in_triangle(projected, a, b, c)
    })
}

fn face_is_fully_backed(
    mesh: &SanitizedMesh,
    backing: &[BackingTriangle],
    face_index: usize,
) -> bool {
    let [a, b, c] = mesh.faces[face_index]
        .indices
        .map(|index| mesh.vertices[index as usize]);
    let midpoint = |u: V3, v: V3| V3 {
        x: (u.x + v.x) * 0.5,
        y: (u.y + v.y) * 0.5,
        z: (u.z + v.z) * 0.5,
    };
    let samples = [
        a,
        b,
        c,
        midpoint(a, b),
        midpoint(b, c),
        midpoint(c, a),
        V3 {
            x: (a.x + b.x + c.x) / 3.0,
            y: (a.y + b.y + c.y) / 3.0,
            z: (a.z + b.z + c.z) / 3.0,
        },
    ];
    samples
        .into_iter()
        .all(|sample| point_has_close_parallel_backing(backing, sample))
}

fn collect_relevant_backing_triangles(
    mesh: &SanitizedMesh,
    candidate_geometry: ComponentGeometry,
    backing: &FaceComponent,
    candidate_normal: V3,
) -> Vec<BackingTriangle> {
    let Some((candidate_origin, _)) = candidate_geometry.plane else {
        return Vec::new();
    };
    backing
        .faces
        .iter()
        .filter_map(|&face_index| {
            let vertices = mesh.faces[face_index]
                .indices
                .map(|index| mesh.vertices[index as usize]);
            let [a, b, c] = vertices;
            let normal = normalized(v3_cross(v3_sub(b, a), v3_sub(c, a)))?;
            if v3_dot(normal, candidate_normal).abs() < FLAT_BACKING_NORMAL_DOT
                || v3_dot(v3_sub(candidate_origin, a), normal).abs()
                    > FLAT_BACKING_MAX_GAP + FLAT_COMPONENT_MAX_THICKNESS
            {
                return None;
            }
            let triangle_min = V3 {
                x: a.x.min(b.x).min(c.x),
                y: a.y.min(b.y).min(c.y),
                z: a.z.min(b.z).min(c.z),
            };
            let triangle_max = V3 {
                x: a.x.max(b.x).max(c.x),
                y: a.y.max(b.y).max(c.y),
                z: a.z.max(b.z).max(c.z),
            };
            let intersects_candidate = triangle_max.x + FLAT_BACKING_MAX_GAP
                >= candidate_geometry.min.x
                && candidate_geometry.max.x + FLAT_BACKING_MAX_GAP >= triangle_min.x
                && triangle_max.y + FLAT_BACKING_MAX_GAP >= candidate_geometry.min.y
                && candidate_geometry.max.y + FLAT_BACKING_MAX_GAP >= triangle_min.y
                && triangle_max.z + FLAT_BACKING_MAX_GAP >= candidate_geometry.min.z
                && candidate_geometry.max.z + FLAT_BACKING_MAX_GAP >= triangle_min.z;
            intersects_candidate.then_some(BackingTriangle { vertices, normal })
        })
        .collect()
}

fn component_is_backed_flat_card(
    mesh: &SanitizedMesh,
    components: &[FaceComponent],
    geometry: &[ComponentGeometry],
    candidate_index: usize,
) -> bool {
    let candidate = &components[candidate_index];
    let candidate_geometry = geometry[candidate_index];
    let Some((_, candidate_normal)) = candidate_geometry.plane else {
        return false;
    };
    if candidate_geometry.closed || candidate_geometry.area <= NORMAL_EPSILON {
        return false;
    }
    components
        .iter()
        .enumerate()
        .filter(|(backing_index, _)| *backing_index != candidate_index)
        .any(|(backing_index, backing)| {
            let backing_geometry = geometry[backing_index];
            if backing_geometry.area < candidate_geometry.area * FLAT_BACKING_MIN_AREA_RATIO
                || !aabb_intersects_with_margin(
                    &candidate_geometry,
                    &backing_geometry,
                    FLAT_BACKING_MAX_GAP,
                )
                || !projected_bounds_contained(
                    candidate_geometry,
                    backing_geometry,
                    candidate_normal,
                )
            {
                return false;
            }
            if let Some((_, backing_normal)) = backing_geometry.plane
                && v3_dot(candidate_normal, backing_normal).abs() < FLAT_BACKING_NORMAL_DOT
            {
                return false;
            }
            let backing_triangles = collect_relevant_backing_triangles(
                mesh,
                candidate_geometry,
                backing,
                candidate_normal,
            );
            if backing_triangles.is_empty() {
                return false;
            }
            let mut total_area = 0.0f32;
            let mut backed_area = 0.0f32;
            for &face_index in &candidate.faces {
                let [a, b, c] = mesh.faces[face_index]
                    .indices
                    .map(|index| mesh.vertices[index as usize]);
                let area = triangle_area(a, b, c);
                total_area += area;
                if face_is_fully_backed(mesh, &backing_triangles, face_index) {
                    backed_area += area;
                }
            }
            total_area > NORMAL_EPSILON && backed_area / total_area >= FLAT_BACKING_MIN_COVERAGE
        })
}

fn component_material_coverage(
    mesh: &SanitizedMesh,
    component: &FaceComponent,
) -> Vec<SourceMaterialCoverage> {
    let mut areas = BTreeMap::<u16, f32>::new();
    for &face_index in &component.faces {
        let face = mesh.faces[face_index];
        let [a, b, c] = face.indices.map(|index| mesh.vertices[index as usize]);
        *areas.entry(face.source_material).or_default() += triangle_area(a, b, c);
    }
    let total: f32 = areas.values().sum();
    areas
        .into_iter()
        .map(|(source_material_slot, area)| SourceMaterialCoverage {
            source_material_slot,
            area,
            fraction: if total > 0.0 { area / total } else { 0.0 },
        })
        .collect()
}

fn dominant_collision_material(
    settings: &CollisionGenerationSettings,
    coverage: &[SourceMaterialCoverage],
) -> Option<u8> {
    let mut areas = BTreeMap::<u8, f32>::new();
    let mut total = 0.0f32;
    for item in coverage {
        *areas
            .entry(settings.collision_material(item.source_material_slot))
            .or_default() += item.area;
        total += item.area;
    }
    let (&material, &area) =
        areas
            .iter()
            .max_by(|(material_a, area_a), (material_b, area_b)| {
                area_a
                    .total_cmp(area_b)
                    .then_with(|| material_b.cmp(material_a))
            })?;
    (total > 0.0 && area / total >= settings.primitive_material_dominance).then_some(material)
}

fn try_fit_box(
    mesh: &SanitizedMesh,
    component: &FaceComponent,
    settings: &CollisionGenerationSettings,
    material: u8,
) -> Option<PrimitiveFit> {
    if !settings.allow_boxes || component.faces.len() < MIN_PRIMITIVE_FACES {
        return None;
    }
    let closed = component_is_closed(mesh, component);
    let (min, max) = component_bounds(mesh, component);
    let extents = [max.x - min.x, max.y - min.y, max.z - min.z];
    if extents.iter().any(|extent| *extent <= COL_VERTEX_STEP) {
        return None;
    }
    let diagonal =
        (extents[0] * extents[0] + extents[1] * extents[1] + extents[2] * extents[2]).sqrt();
    let tolerance = (diagonal * settings.primitive_relative_tolerance)
        .max(COL_VERTEX_STEP * 1.5)
        .min(settings.max_error.max(COL_VERTEX_STEP * 1.5));
    let mut plane_areas = [0.0f32; 6];
    let mut unclassified_area = 0.0f32;

    for &face_index in &component.faces {
        let face = mesh.faces[face_index];
        let vertices = face.indices.map(|index| mesh.vertices[index as usize]);
        let normal = v3_cross(
            v3_sub(vertices[1], vertices[0]),
            v3_sub(vertices[2], vertices[0]),
        );
        let length = v3_length(normal);
        if length <= NORMAL_EPSILON {
            return None;
        }
        let area = length * 0.5;
        let normalized = [
            normal.x.abs() / length,
            normal.y.abs() / length,
            normal.z.abs() / length,
        ];
        let (axis, alignment) = normalized
            .into_iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(&b.1))?;
        if alignment < 0.995 {
            unclassified_area += area;
            continue;
        }
        let low = [min.x, min.y, min.z][axis];
        let high = [max.x, max.y, max.z][axis];
        let coordinate = [vertices[0].x, vertices[0].y, vertices[0].z][axis];
        let plane = if (coordinate - low).abs() <= tolerance {
            Some(axis * 2)
        } else if (coordinate - high).abs() <= tolerance {
            Some(axis * 2 + 1)
        } else {
            None
        };
        let Some(plane) = plane else {
            unclassified_area += area;
            continue;
        };
        if vertices.iter().any(|vertex| {
            let value = [vertex.x, vertex.y, vertex.z][axis];
            (value - [low, high][plane % 2]).abs() > tolerance
        }) {
            unclassified_area += area;
            continue;
        }
        plane_areas[plane] += area;
    }

    let expected = [
        extents[1] * extents[2],
        extents[1] * extents[2],
        extents[0] * extents[2],
        extents[0] * extents[2],
        extents[0] * extents[1],
        extents[0] * extents[1],
    ];
    let coverage = std::array::from_fn::<_, 6, _>(|plane| plane_areas[plane] / expected[plane]);
    for &value in &coverage {
        // The lower bound detects holes/openings; the upper bound rejects
        // overlapping or folded surfaces masquerading as a box shell.
        if value > 1.08 {
            return None;
        }
    }
    let complete_shell = coverage
        .iter()
        .all(|value| *value >= settings.box_min_surface_coverage);
    // Many building DFFs deliberately omit their underside. Five complete
    // exterior planes still describe an unambiguous solid cuboid, while a
    // missing wall/doorway must remain mesh collision. Permit a small amount
    // of interior detail, but only when the absent plane is the bottom.
    let open_bottom_building = !closed
        && coverage[4] <= 0.10
        && [0usize, 1, 2, 3, 5]
            .into_iter()
            .all(|plane| coverage[plane] >= settings.box_min_surface_coverage)
        && unclassified_area <= expected.iter().sum::<f32>() * 0.02;
    if !(closed && complete_shell && unclassified_area <= NORMAL_EPSILON || open_bottom_building) {
        return None;
    }
    Some(PrimitiveFit {
        kind: GeneratedPrimitiveKind::Box,
        min,
        max,
        center: V3::default(),
        radius: 0.0,
        material,
    })
}

fn try_fit_sphere(
    mesh: &SanitizedMesh,
    component: &FaceComponent,
    settings: &CollisionGenerationSettings,
    material: u8,
) -> Option<PrimitiveFit> {
    if !settings.allow_spheres
        || component.faces.len() < MIN_PRIMITIVE_FACES
        || !component_is_closed(mesh, component)
    {
        return None;
    }
    let (min, max) = component_bounds(mesh, component);
    let extents = [max.x - min.x, max.y - min.y, max.z - min.z];
    let longest = extents.iter().copied().fold(0.0f32, f32::max);
    let shortest = extents.iter().copied().fold(f32::INFINITY, f32::min);
    if longest <= COL_VERTEX_STEP || shortest / longest < 0.92 {
        return None;
    }
    let center = V3 {
        x: (min.x + max.x) * 0.5,
        y: (min.y + max.y) * 0.5,
        z: (min.z + max.z) * 0.5,
    };
    let mut unique_vertices = BTreeSet::new();
    let mut radii = Vec::new();
    for &face_index in &component.faces {
        for &vertex_index in &mesh.faces[face_index].indices {
            if unique_vertices.insert(vertex_index) {
                radii.push(v3_length(v3_sub(
                    mesh.vertices[vertex_index as usize],
                    center,
                )));
            }
        }
    }
    if radii.len() < 8 {
        return None;
    }
    let radius = radii.iter().sum::<f32>() / radii.len() as f32;
    if radius <= COL_VERTEX_STEP {
        return None;
    }
    let max_radial_error = radii
        .iter()
        .map(|value| (value - radius).abs() / radius)
        .fold(0.0f32, f32::max);
    let mean_radial_error = radii
        .iter()
        .map(|value| (value - radius).abs() / radius)
        .sum::<f32>()
        / radii.len() as f32;
    if max_radial_error > settings.sphere_max_radial_error
        || mean_radial_error > settings.sphere_max_radial_error * 0.45
    {
        return None;
    }
    let surface_area: f32 = component
        .faces
        .iter()
        .map(|&face_index| {
            let [a, b, c] = mesh.faces[face_index]
                .indices
                .map(|index| mesh.vertices[index as usize]);
            triangle_area(a, b, c)
        })
        .sum();
    let sphere_area = 4.0 * std::f32::consts::PI * radius * radius;
    let coverage = surface_area / sphere_area;
    if coverage < settings.sphere_min_surface_coverage || coverage > 1.05 {
        return None;
    }
    Some(PrimitiveFit {
        kind: GeneratedPrimitiveKind::Sphere,
        min: V3::default(),
        max: V3::default(),
        center,
        radius,
        material,
    })
}

fn force_fit_box(
    mesh: &SanitizedMesh,
    component: &FaceComponent,
    settings: &CollisionGenerationSettings,
    material: u8,
) -> Option<PrimitiveFit> {
    if !settings.force_shapes {
        return None;
    }
    let (mut min, mut max) = component_bounds(mesh, component);
    // COL boxes need volume. Give planar source components one quantization
    // step of thickness instead of falling back to a triangle mesh.
    let mut low = [min.x, min.y, min.z];
    let mut high = [max.x, max.y, max.z];
    for axis in 0..3 {
        if high[axis] - low[axis] < COL_VERTEX_STEP {
            let center = (low[axis] + high[axis]) * 0.5;
            low[axis] = center - COL_VERTEX_STEP * 0.5;
            high[axis] = center + COL_VERTEX_STEP * 0.5;
        }
    }
    min = V3 {
        x: low[0],
        y: low[1],
        z: low[2],
    };
    max = V3 {
        x: high[0],
        y: high[1],
        z: high[2],
    };
    Some(PrimitiveFit {
        kind: GeneratedPrimitiveKind::Box,
        min,
        max,
        center: V3::default(),
        radius: 0.0,
        material,
    })
}

fn collision_surface(material: u8) -> CollisionSurface {
    CollisionSurface {
        material,
        flags: 0,
        brightness: 0,
        light: 255,
    }
}

fn source_face_to_collision(indices: [u16; 3], material: u8) -> CollisionFace {
    CollisionFace {
        a: indices[0],
        b: indices[1],
        c: indices[2],
        material,
        light: 255,
        img_path: PathBuf::new(),
        material_file_offset: 0,
        light_file_offset: 0,
    }
}

fn edge_counts(mesh: &SanitizedMesh, face_indices: &[usize]) -> BTreeMap<(u32, u32), usize> {
    let mut counts = BTreeMap::new();
    for &face_index in face_indices {
        let [a, b, c] = mesh.faces[face_index].indices;
        for (a, b) in [(a, b), (b, c), (c, a)] {
            *counts.entry((a.min(b), a.max(b))).or_default() += 1;
        }
    }
    counts
}

fn simplify_face_group(
    mesh: &SanitizedMesh,
    face_indices: &[usize],
    component_edge_counts: &BTreeMap<(u32, u32), usize>,
    settings: &CollisionGenerationSettings,
) -> Result<Vec<u32>, CollisionGenerationError> {
    let mut indices = Vec::with_capacity(face_indices.len() * 3);
    for &face_index in face_indices {
        indices.extend_from_slice(&mesh.faces[face_index].indices);
    }
    if face_indices.len() < settings.min_faces_for_simplification {
        return Ok(indices);
    }
    // `max_error` is the actual quality constraint. A percentage target ties
    // collision density to render-mesh density: subdividing an otherwise
    // identical DFF then produces a much denser COL. Ask meshopt to simplify
    // toward a small topology floor instead; it will stop before that target
    // whenever another collapse would exceed the preset's world-space error.
    let target_faces = settings.target_face_floor.clamp(1, face_indices.len());
    if target_faces >= face_indices.len() {
        return Ok(indices);
    }
    let positions: Vec<[f32; 3]> = mesh
        .vertices
        .iter()
        .map(|vertex| [vertex.x, vertex.y, vertex.z])
        .collect();
    let adapter = VertexDataAdapter::new(typed_to_bytes(&positions), mem::size_of::<[f32; 3]>(), 0)
        .map_err(|error| CollisionGenerationError::Simplifier(error.to_string()))?;
    let options = SimplifyOptions::ErrorAbsolute | SimplifyOptions::Regularize;
    let mut vertex_locks = vec![false; mesh.vertices.len()];
    if settings.preserve_mesh_boundaries {
        let group_edge_counts = edge_counts(mesh, face_indices);
        for ((a, b), group_count) in group_edge_counts {
            // Preserve both the component silhouette and internal material
            // seams. Positional simplification error cannot protect an open,
            // planar outline: collapsing its edge inward still lies on the
            // same plane and can report almost zero error while deleting a
            // road, floor, or terrain footprint.
            let is_group_boundary = group_count != 2;
            let is_material_seam = component_edge_counts.get(&(a, b)).copied() != Some(group_count);
            if is_group_boundary || is_material_seam {
                vertex_locks[a as usize] = true;
                vertex_locks[b as usize] = true;
            }
        }
    }
    let mut result_error = 0.0f32;
    let simplified = if settings.preserve_mesh_boundaries {
        simplify_with_locks(
            &indices,
            &adapter,
            &vertex_locks,
            target_faces * 3,
            settings.max_error,
            options,
            Some(&mut result_error),
        )
    } else {
        simplify(
            &indices,
            &adapter,
            target_faces * 3,
            settings.max_error,
            options,
            Some(&mut result_error),
        )
    };
    if simplified.len() < 3
        || simplified.len() % 3 != 0
        || result_error > settings.max_error + COL_VERTEX_STEP
    {
        Ok(indices)
    } else {
        Ok(simplified)
    }
}

fn generate_empty_collision(raw: &RawMesh, name: &str) -> CollisionGenerationResult {
    let mesh = CollisionMesh {
        name: name.to_string(),
        spheres: Vec::new(),
        boxes: Vec::new(),
        vertices: Vec::new(),
        faces: Vec::new(),
        bounds: bounds_from_vertices(&raw.vertices),
        shadow_vertices: Vec::new(),
        shadow_faces: Vec::new(),
    };
    CollisionGenerationResult {
        mesh,
        face_provenance: Vec::new(),
        primitive_provenance: Vec::new(),
        stats: CollisionGenerationStats {
            source_vertices: raw.vertices.len(),
            source_faces: raw.triangles.len(),
            ..Default::default()
        },
    }
}

/// Generate collision geometry from a flattened DFF mesh.
///
/// The operation is deterministic and pure with respect to application state,
/// so it is safe to run on the editor's background worker.
pub(crate) fn generate_collision(
    raw: &RawMesh,
    name: &str,
    settings: &CollisionGenerationSettings,
) -> Result<CollisionGenerationResult, CollisionGenerationError> {
    if settings.empty_collision {
        return Ok(generate_empty_collision(raw, name));
    }
    let sanitized = sanitize(raw, &settings.excluded_source_materials)?;
    if sanitized.faces.is_empty() {
        if !raw.triangles.is_empty()
            && raw.triangles.iter().all(|triangle| {
                settings
                    .excluded_source_materials
                    .contains(&triangle.material)
            })
        {
            let mesh = CollisionMesh {
                name: name.to_string(),
                spheres: Vec::new(),
                boxes: Vec::new(),
                vertices: Vec::new(),
                faces: Vec::new(),
                bounds: bounds_from_vertices(&raw.vertices),
                shadow_vertices: Vec::new(),
                shadow_faces: Vec::new(),
            };
            return Ok(CollisionGenerationResult {
                mesh,
                face_provenance: Vec::new(),
                primitive_provenance: Vec::new(),
                stats: CollisionGenerationStats {
                    source_vertices: raw.vertices.len(),
                    source_faces: raw.triangles.len(),
                    ..Default::default()
                },
            });
        }
        return Err(CollisionGenerationError::NoUsableGeometry);
    }
    let components = split_components(&sanitized);
    let component_geometry = components
        .iter()
        .map(|component| component_geometry(&sanitized, component))
        .collect::<Vec<_>>();
    let discarded_backed_flat_components = (0..components.len())
        .filter(|&component_index| {
            component_is_backed_flat_card(
                &sanitized,
                &components,
                &component_geometry,
                component_index,
            )
        })
        .collect::<BTreeSet<_>>();
    let mut output = CollisionMesh {
        name: name.to_string(),
        spheres: Vec::new(),
        boxes: Vec::new(),
        vertices: Vec::new(),
        faces: Vec::new(),
        bounds: Bounds {
            min: Vec3::ZERO,
            max: Vec3::ZERO,
        },
        shadow_vertices: Vec::new(),
        shadow_faces: Vec::new(),
    };
    let mut face_provenance = Vec::new();
    let mut primitive_provenance = Vec::new();
    let mut output_vertex_map = BTreeMap::<u32, u16>::new();

    for (component_index, component) in components.iter().enumerate() {
        if discarded_backed_flat_components.contains(&component_index) {
            continue;
        }
        let coverage = component_material_coverage(&sanitized, component);
        let primitive_fit = dominant_collision_material(settings, &coverage).and_then(|material| {
            try_fit_box(&sanitized, component, settings, material)
                .or_else(|| try_fit_sphere(&sanitized, component, settings, material))
                .or_else(|| force_fit_box(&sanitized, component, settings, material))
        });
        if let Some(fit) = primitive_fit {
            let primitive_index = match fit.kind {
                GeneratedPrimitiveKind::Box => {
                    let index = output.boxes.len();
                    output.boxes.push(CollisionBox {
                        min: fit.min,
                        max: fit.max,
                        surface: collision_surface(fit.material),
                    });
                    index
                }
                GeneratedPrimitiveKind::Sphere => {
                    let index = output.spheres.len();
                    output.spheres.push(CollisionSphere {
                        center: fit.center,
                        radius: fit.radius,
                        surface: collision_surface(fit.material),
                    });
                    index
                }
            };
            primitive_provenance.push(GeneratedPrimitiveProvenance {
                kind: fit.kind,
                primitive_index,
                source_component: component.source_component,
                source_materials: coverage,
            });
            continue;
        }

        let mut by_material = BTreeMap::<u16, Vec<usize>>::new();
        let component_edge_counts = edge_counts(&sanitized, &component.faces);
        for &face_index in &component.faces {
            by_material
                .entry(sanitized.faces[face_index].source_material)
                .or_default()
                .push(face_index);
        }
        for (source_material, face_indices) in by_material {
            let indices =
                simplify_face_group(&sanitized, &face_indices, &component_edge_counts, settings)?;
            for triangle in indices.chunks_exact(3) {
                let [a, b, c] = [
                    sanitized.vertices[triangle[0] as usize],
                    sanitized.vertices[triangle[1] as usize],
                    sanitized.vertices[triangle[2] as usize],
                ];
                if triangle_area(a, b, c) <= NORMAL_EPSILON {
                    continue;
                }
                let mut converted = [0u16; 3];
                for corner in 0..3 {
                    let source_index = triangle[corner];
                    let output_index = if let Some(index) = output_vertex_map.get(&source_index) {
                        *index
                    } else {
                        let next = output.vertices.len();
                        if next >= u16::MAX as usize {
                            return Err(CollisionGenerationError::TooManyVertices {
                                count: next + 1,
                            });
                        }
                        output
                            .vertices
                            .push(sanitized.vertices[source_index as usize]);
                        let next = next as u16;
                        output_vertex_map.insert(source_index, next);
                        next
                    };
                    converted[corner] = output_index;
                }
                if converted[0] == converted[1]
                    || converted[1] == converted[2]
                    || converted[2] == converted[0]
                {
                    continue;
                }
                output.faces.push(source_face_to_collision(
                    converted,
                    settings.collision_material(source_material),
                ));
                face_provenance.push(CollisionSourceProvenance {
                    source_material_slot: source_material,
                    source_component: component.source_component,
                });
                if output.faces.len() > u16::MAX as usize {
                    return Err(CollisionGenerationError::TooManyFaces {
                        count: output.faces.len(),
                    });
                }
            }
        }
    }
    if output.faces.is_empty() && output.boxes.is_empty() && output.spheres.is_empty() {
        return Err(CollisionGenerationError::NoUsableGeometry);
    }
    // The COL broad-phase describes the visual DFF, not the simplified
    // collision approximation. Empty COLs still retain these bounds.
    output.bounds = bounds_from_vertices(&raw.vertices);
    let stats = CollisionGenerationStats {
        source_vertices: raw.vertices.len(),
        source_faces: raw.triangles.len(),
        sanitized_vertices: sanitized.vertices.len(),
        sanitized_faces: sanitized.faces.len(),
        components: components.len(),
        discarded_backed_flat_components: discarded_backed_flat_components.len(),
        boxes: output.boxes.len(),
        spheres: output.spheres.len(),
        mesh_vertices: output.vertices.len(),
        mesh_faces: output.faces.len(),
    };
    debug_assert_eq!(face_provenance.len(), output.faces.len());
    Ok(CollisionGenerationResult {
        mesh: output,
        face_provenance,
        primitive_provenance,
        stats,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "local Mafia map audit fixture"]
    fn sity6453_008_open_bottom_building_audit() {
        let path = Path::new(
            "/home/codyl/MTA_Server/mods/deathmatch/resources/[Maps]/Mafia2/mafia2/zones/City_Builds/dff/Sity6453_008.dff",
        );
        let raw = parse_dff_mesh(&fs::read(path).expect("real Mafia DFF"));
        let settings = CollisionGenerationSettings::for_preset(
            CollisionGenerationPreset::Auto,
            0,
            vec![None; raw.material_textures.len()],
        );
        let result = generate_collision(&raw, "Sity6453_008", &settings).unwrap();
        let contains_box = |min: V3, max: V3| {
            result.mesh.boxes.iter().any(|candidate| {
                v3_length(v3_sub(candidate.min, min)) < 0.02
                    && v3_length(v3_sub(candidate.max, max)) < 0.02
            })
        };

        assert!(contains_box(
            V3 {
                x: 87.88281,
                y: 218.35156,
                z: -14.359375,
            },
            V3 {
                x: 117.89844,
                y: 233.35156,
                z: 2.8203125,
            },
        ));
        assert!(contains_box(
            V3 {
                x: 87.88281,
                y: 193.44531,
                z: -14.359375,
            },
            V3 {
                x: 117.89844,
                y: 208.44531,
                z: 2.8203125,
            },
        ));
    }
    fn raw_mesh(vertices: Vec<V3>, triangles: Vec<Tri>) -> RawMesh {
        RawMesh {
            vertices,
            triangles,
            material_textures: vec!["test".to_string()],
            materials: vec![RawMaterial::default()],
            ..RawMesh::default()
        }
    }

    #[test]
    fn exact_positive_col_boundary_is_clamped_without_rejecting_the_mesh() {
        let raw = raw_mesh(
            vec![
                V3 {
                    x: 0.0,
                    y: 256.0,
                    z: 0.0,
                },
                V3 {
                    x: 1.0,
                    y: 255.0,
                    z: 0.0,
                },
                V3 {
                    x: 0.0,
                    y: 255.0,
                    z: 1.0,
                },
            ],
            vec![Tri {
                a: 0,
                b: 1,
                c: 2,
                material: 0,
            }],
        );

        let result = generate_collision(&raw, "positive_boundary", &Default::default()).unwrap();

        assert_eq!(result.mesh.faces.len(), 1);
        assert!(
            result
                .mesh
                .vertices
                .iter()
                .any(|vertex| vertex.y == i16::MAX as f32 / COL_VERTEX_SCALE)
        );
        assert_eq!(result.mesh.bounds.max.y, 256.0);
        assert!(matches!(
            quantize_vertex(
                V3 {
                    x: 256.0 + COL_VERTEX_STEP,
                    y: 0.0,
                    z: 0.0,
                },
                0
            ),
            Err(CollisionGenerationError::VertexOutsideColRange { .. })
        ));
    }

    fn push_quad(
        vertices: &mut Vec<V3>,
        triangles: &mut Vec<Tri>,
        corners: [V3; 4],
        material: u16,
    ) {
        // Deliberately duplicate positions between faces. Generation must weld
        // these using only the quantized position, not render attributes.
        let base = vertices.len() as u32;
        vertices.extend_from_slice(&corners);
        triangles.push(Tri {
            a: base,
            b: base + 1,
            c: base + 2,
            material,
        });
        triangles.push(Tri {
            a: base,
            b: base + 2,
            c: base + 3,
            material,
        });
    }

    fn cube_mesh(open_positive_y: bool) -> RawMesh {
        let p = |x, y, z| V3 { x, y, z };
        let mut vertices = Vec::new();
        let mut triangles = Vec::new();
        push_quad(
            &mut vertices,
            &mut triangles,
            [
                p(-1.0, -1.0, -1.0),
                p(-1.0, -1.0, 1.0),
                p(-1.0, 1.0, 1.0),
                p(-1.0, 1.0, -1.0),
            ],
            0,
        );
        push_quad(
            &mut vertices,
            &mut triangles,
            [
                p(1.0, -1.0, -1.0),
                p(1.0, 1.0, -1.0),
                p(1.0, 1.0, 1.0),
                p(1.0, -1.0, 1.0),
            ],
            0,
        );
        push_quad(
            &mut vertices,
            &mut triangles,
            [
                p(-1.0, -1.0, -1.0),
                p(1.0, -1.0, -1.0),
                p(1.0, -1.0, 1.0),
                p(-1.0, -1.0, 1.0),
            ],
            0,
        );
        if !open_positive_y {
            push_quad(
                &mut vertices,
                &mut triangles,
                [
                    p(-1.0, 1.0, -1.0),
                    p(-1.0, 1.0, 1.0),
                    p(1.0, 1.0, 1.0),
                    p(1.0, 1.0, -1.0),
                ],
                0,
            );
        }
        push_quad(
            &mut vertices,
            &mut triangles,
            [
                p(-1.0, -1.0, -1.0),
                p(-1.0, 1.0, -1.0),
                p(1.0, 1.0, -1.0),
                p(1.0, -1.0, -1.0),
            ],
            0,
        );
        push_quad(
            &mut vertices,
            &mut triangles,
            [
                p(-1.0, -1.0, 1.0),
                p(1.0, -1.0, 1.0),
                p(1.0, 1.0, 1.0),
                p(-1.0, 1.0, 1.0),
            ],
            0,
        );
        raw_mesh(vertices, triangles)
    }

    fn sphere_mesh(radius: f32, rings: usize, segments: usize) -> RawMesh {
        let mut vertices = vec![V3 {
            x: 0.0,
            y: 0.0,
            z: radius,
        }];
        for ring in 1..rings {
            let latitude = std::f32::consts::PI * ring as f32 / rings as f32;
            for segment in 0..segments {
                let longitude = std::f32::consts::TAU * segment as f32 / segments as f32;
                vertices.push(V3 {
                    x: radius * latitude.sin() * longitude.cos(),
                    y: radius * latitude.sin() * longitude.sin(),
                    z: radius * latitude.cos(),
                });
            }
        }
        let south = vertices.len() as u32;
        vertices.push(V3 {
            x: 0.0,
            y: 0.0,
            z: -radius,
        });
        let mut triangles = Vec::new();
        let ring_index = |ring: usize, segment: usize| -> u32 {
            1 + ((ring - 1) * segments + segment % segments) as u32
        };
        for segment in 0..segments {
            triangles.push(Tri {
                a: 0,
                b: ring_index(1, segment),
                c: ring_index(1, segment + 1),
                material: 0,
            });
        }
        for ring in 1..rings - 1 {
            for segment in 0..segments {
                let a = ring_index(ring, segment);
                let b = ring_index(ring + 1, segment);
                let c = ring_index(ring + 1, segment + 1);
                let d = ring_index(ring, segment + 1);
                triangles.push(Tri {
                    a,
                    b,
                    c,
                    material: 0,
                });
                triangles.push(Tri {
                    a,
                    b: c,
                    c: d,
                    material: 0,
                });
            }
        }
        for segment in 0..segments {
            triangles.push(Tri {
                a: ring_index(rings - 1, segment),
                b: south,
                c: ring_index(rings - 1, segment + 1),
                material: 0,
            });
        }
        raw_mesh(vertices, triangles)
    }

    #[test]
    fn duplicated_render_cube_welds_and_becomes_material_box() {
        let raw = cube_mesh(false);
        assert_eq!(raw.vertices.len(), 24);
        let settings = CollisionGenerationSettings::for_preset(
            CollisionGenerationPreset::Auto,
            0,
            vec![Some(9)],
        );
        let result = generate_collision(&raw, "cube", &settings).unwrap();
        assert_eq!(result.stats.sanitized_vertices, 8);
        assert_eq!(result.mesh.boxes.len(), 1);
        assert!(result.mesh.spheres.is_empty());
        assert!(result.mesh.faces.is_empty());
        assert_eq!(result.mesh.boxes[0].surface.material, 9);
        assert_eq!(
            result.primitive_provenance[0].source_materials,
            vec![SourceMaterialCoverage {
                source_material_slot: 0,
                area: 24.0,
                fraction: 1.0,
            }]
        );
    }

    #[test]
    fn building_shell_without_underside_becomes_native_box() {
        let mut raw = cube_mesh(false);
        raw.triangles.retain(|triangle| {
            [triangle.a, triangle.b, triangle.c]
                .into_iter()
                .any(|index| raw.vertices[index as usize].z > -1.0)
        });
        let result = generate_collision(&raw, "open_bottom_building", &Default::default()).unwrap();

        assert_eq!(result.mesh.boxes.len(), 1);
        assert!(result.mesh.faces.is_empty());
        assert_eq!(result.mesh.boxes[0].min.z, -1.0);
        assert_eq!(result.mesh.boxes[0].max.z, 1.0);
    }

    #[test]
    fn empty_collision_generation_emits_no_lod_geometry() {
        let raw = cube_mesh(true);
        let settings = CollisionGenerationSettings::for_preset(
            CollisionGenerationPreset::Surface,
            0,
            vec![Some(9)],
        )
        .with_empty_collision(true);
        let result = generate_collision(&raw, "lod_model", &settings).unwrap();

        assert!(result.mesh.boxes.is_empty());
        assert!(result.mesh.spheres.is_empty());
        assert!(result.mesh.vertices.is_empty());
        assert!(result.mesh.faces.is_empty());
        assert!(result.primitive_provenance.is_empty());
        assert!(result.mesh.bounds == bounds_from_vertices(&raw.vertices));
        assert_eq!(result.stats.boxes, 0);
        assert_eq!(result.stats.mesh_faces, 0);
    }

    #[test]
    fn mixed_material_closed_cube_stays_mesh_and_keeps_both_materials() {
        let mut raw = cube_mesh(false);
        let face_count = raw.triangles.len();
        for triangle in &mut raw.triangles[face_count - 2..] {
            triangle.material = 1;
        }
        raw.material_textures.push("grass".to_string());
        raw.materials.push(RawMaterial::default());
        let settings = CollisionGenerationSettings::for_preset(
            CollisionGenerationPreset::Prop,
            0,
            vec![Some(4), Some(9)],
        );
        let result = generate_collision(&raw, "mixed_cube", &settings).unwrap();
        assert!(result.mesh.boxes.is_empty());
        assert!(result.mesh.spheres.is_empty());
        let materials: BTreeSet<_> = result.mesh.faces.iter().map(|face| face.material).collect();
        assert_eq!(materials, BTreeSet::from([4, 9]));
        let source_slots: BTreeSet<_> = result
            .face_provenance
            .iter()
            .map(|provenance| provenance.source_material_slot)
            .collect();
        assert_eq!(source_slots, BTreeSet::from([0, 1]));
    }

    #[test]
    fn shapes_preset_forces_open_geometry_to_a_primitive() {
        let raw = cube_mesh(true);
        let settings = CollisionGenerationSettings::for_preset(
            CollisionGenerationPreset::Shapes,
            0,
            vec![Some(9)],
        );

        let result = generate_collision(&raw, "physics_prop", &settings).unwrap();

        assert_eq!(result.mesh.boxes.len(), 1);
        assert!(result.mesh.spheres.is_empty());
        assert!(result.mesh.vertices.is_empty());
        assert!(result.mesh.faces.is_empty());
        assert_eq!(result.mesh.boxes[0].surface.material, 9);
        assert_eq!(result.stats.boxes, 1);
        assert_eq!(result.stats.mesh_faces, 0);
    }

    #[test]
    fn excluded_material_faces_are_not_generated() {
        let mut raw = cube_mesh(false);
        let face_count = raw.triangles.len();
        for triangle in &mut raw.triangles[face_count - 2..] {
            triangle.material = 1;
        }
        raw.material_textures.push("invisible".to_string());
        raw.materials.push(RawMaterial::default());
        let settings = CollisionGenerationSettings::for_preset(
            CollisionGenerationPreset::Auto,
            0,
            vec![Some(4), Some(9)],
        )
        .with_excluded_source_materials(BTreeSet::from([1]));
        let result = generate_collision(&raw, "excluded_face", &settings).unwrap();
        assert!(result.mesh.boxes.is_empty());
        assert!(result.mesh.spheres.is_empty());
        assert!(!result.mesh.faces.is_empty());
        assert!(result.mesh.faces.iter().all(|face| face.material == 4));
        assert!(
            result
                .face_provenance
                .iter()
                .all(|source| source.source_material_slot == 0)
        );
    }

    #[test]
    fn fully_excluded_mesh_generates_empty_collision() {
        let raw = cube_mesh(false);
        let settings = CollisionGenerationSettings::default()
            .with_excluded_source_materials(BTreeSet::from([0]));
        let result = generate_collision(&raw, "empty", &settings).unwrap();
        assert!(result.mesh.faces.is_empty());
        assert!(result.mesh.vertices.is_empty());
        assert!(result.mesh.boxes.is_empty());
        assert!(result.mesh.spheres.is_empty());
        assert!(result.mesh.bounds == bounds_from_vertices(&raw.vertices));
        assert_eq!(result.stats.source_faces, raw.triangles.len());
        assert_eq!(result.stats.sanitized_faces, 0);
        let template = crate::col::export::minimal_col2_template("empty.col");
        let bytes =
            crate::col::export::write_col_mesh_from_template(&template, &result.mesh).unwrap();
        let issues = crate::col::validation::validate_col_for_game_load("empty.col", &bytes);
        assert!(!crate::col::validation::col_validation_has_errors(&issues));
    }

    #[test]
    fn round_closed_mesh_becomes_native_sphere() {
        let raw = sphere_mesh(2.0, 10, 24);
        let settings = CollisionGenerationSettings::default();
        let result = generate_collision(&raw, "sphere", &settings).unwrap();
        assert_eq!(result.mesh.spheres.len(), 1);
        assert!(result.mesh.boxes.is_empty());
        assert!(result.mesh.faces.is_empty());
        assert!((result.mesh.spheres[0].radius - 2.0).abs() < 0.01);
    }

    #[test]
    fn open_doorway_like_shell_is_never_filled_by_box() {
        let raw = cube_mesh(true);
        let settings =
            CollisionGenerationSettings::for_preset(CollisionGenerationPreset::Prop, 0, vec![None]);
        let result = generate_collision(&raw, "doorway", &settings).unwrap();
        assert!(result.mesh.boxes.is_empty());
        assert!(result.mesh.spheres.is_empty());
        assert!(!result.mesh.faces.is_empty());
    }

    #[test]
    fn curved_dense_surface_reduces_while_retaining_open_boundary() {
        const SIDE: usize = 25;
        let mut vertices = Vec::new();
        for y in 0..SIDE {
            for x in 0..SIDE {
                let px = x as f32 * 0.25;
                let py = y as f32 * 0.25;
                vertices.push(V3 {
                    x: px,
                    y: py,
                    z: (px * 0.7).sin() * (py * 0.5).cos() * 0.12,
                });
            }
        }
        let mut triangles = Vec::new();
        for y in 0..SIDE - 1 {
            for x in 0..SIDE - 1 {
                let a = (y * SIDE + x) as u32;
                let b = a + 1;
                let d = ((y + 1) * SIDE + x) as u32;
                let c = d + 1;
                triangles.push(Tri {
                    a,
                    b,
                    c,
                    material: 0,
                });
                triangles.push(Tri {
                    a,
                    b: c,
                    c: d,
                    material: 0,
                });
            }
        }
        let source_faces = triangles.len();
        let raw = raw_mesh(vertices, triangles);
        let settings = CollisionGenerationSettings::for_preset(
            CollisionGenerationPreset::Surface,
            0,
            vec![None],
        );
        let result = generate_collision(&raw, "terrain", &settings).unwrap();
        assert!(
            result.mesh.faces.len() < source_faces / 2,
            "{} source faces only reduced to {}",
            source_faces,
            result.mesh.faces.len()
        );
        assert_eq!(result.face_provenance.len(), result.mesh.faces.len());

        // Every source perimeter point remains locked. On a planar surface the
        // simplifier's positional error alone cannot detect an outline that
        // shrinks inward, so preserving the complete boundary is required for
        // a safe collision footprint.
        let mut edge_counts = BTreeMap::<(u16, u16), usize>::new();
        for face in &result.mesh.faces {
            for (a, b) in [(face.a, face.b), (face.b, face.c), (face.c, face.a)] {
                *edge_counts.entry((a.min(b), a.max(b))).or_default() += 1;
            }
        }
        let output_boundary_vertices = edge_counts
            .into_iter()
            .filter(|(_, count)| *count == 1)
            .flat_map(|((a, b), _)| [a, b])
            .collect::<BTreeSet<_>>();
        for y in 0..SIDE {
            for x in 0..SIDE {
                if x != 0 && x != SIDE - 1 && y != 0 && y != SIDE - 1 {
                    continue;
                }
                let source_index = y * SIDE + x;
                let expected = quantize_vertex(raw.vertices[source_index], source_index)
                    .unwrap()
                    .0;
                let output_index = result
                    .mesh
                    .vertices
                    .iter()
                    .position(|vertex| *vertex == expected)
                    .expect("source perimeter vertex must survive simplification")
                    as u16;
                assert!(output_boundary_vertices.contains(&output_index));
            }
        }
    }

    #[test]
    fn mesh_material_seams_keep_resolved_material_and_source_slot() {
        let vertices = vec![
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
                x: 1.0,
                y: 1.0,
                z: 0.0,
            },
            V3 {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
        ];
        let triangles = vec![
            Tri {
                a: 0,
                b: 1,
                c: 2,
                material: 0,
            },
            Tri {
                a: 0,
                b: 2,
                c: 3,
                material: 1,
            },
        ];
        let mut raw = raw_mesh(vertices, triangles);
        raw.material_textures.push("grass".to_string());
        raw.materials.push(RawMaterial::default());
        let settings = CollisionGenerationSettings::for_preset(
            CollisionGenerationPreset::Surface,
            0,
            vec![Some(4), Some(9)],
        );
        let result = generate_collision(&raw, "materials", &settings).unwrap();
        let resolved: Vec<_> = result
            .mesh
            .faces
            .iter()
            .zip(&result.face_provenance)
            .map(|(face, provenance)| (face.material, provenance.source_material_slot))
            .collect();
        assert_eq!(resolved, vec![(4, 0), (9, 1)]);
    }

    #[test]
    fn dense_material_seam_stays_watertight_while_surfaces_simplify() {
        const SIDE: usize = 17;
        const SPLIT_X: usize = 8;
        let vertices = (0..SIDE)
            .flat_map(|y| {
                (0..SIDE).map(move |x| V3 {
                    x: x as f32 * 0.25,
                    y: y as f32 * 0.25,
                    z: 0.0,
                })
            })
            .collect::<Vec<_>>();
        let mut triangles = Vec::new();
        for y in 0..SIDE - 1 {
            for x in 0..SIDE - 1 {
                let a = (y * SIDE + x) as u32;
                let b = a + 1;
                let d = ((y + 1) * SIDE + x) as u32;
                let c = d + 1;
                let material = usize::from(x >= SPLIT_X) as u16;
                triangles.push(Tri { a, b, c, material });
                triangles.push(Tri {
                    a,
                    b: c,
                    c: d,
                    material,
                });
            }
        }
        let source_faces = triangles.len();
        let mut raw = raw_mesh(vertices, triangles);
        raw.material_textures.push("other".to_string());
        raw.materials.push(RawMaterial::default());
        let settings = CollisionGenerationSettings::for_preset(
            CollisionGenerationPreset::Surface,
            0,
            vec![Some(4), Some(9)],
        );

        let result = generate_collision(&raw, "material_seam", &settings).unwrap();

        assert!(result.mesh.faces.len() < source_faces / 2);
        for y in 0..SIDE {
            let source_index = y * SIDE + SPLIT_X;
            let seam_vertex = quantize_vertex(raw.vertices[source_index], source_index)
                .unwrap()
                .0;
            assert!(result.mesh.vertices.contains(&seam_vertex));
        }
        assert_eq!(
            result
                .mesh
                .faces
                .iter()
                .map(|face| face.material)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([4, 9])
        );
    }

    #[test]
    fn quantization_welds_near_vertices_and_removes_degenerate_faces() {
        let raw = raw_mesh(
            vec![
                V3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                V3 {
                    x: 0.001,
                    y: 0.0,
                    z: 0.0,
                },
                V3 {
                    x: 1.001,
                    y: 0.0,
                    z: 0.0,
                },
                V3 {
                    x: 0.0,
                    y: 1.001,
                    z: 0.0,
                },
            ],
            vec![
                Tri {
                    a: 0,
                    b: 1,
                    c: 2,
                    material: 0,
                },
                Tri {
                    a: 0,
                    b: 2,
                    c: 3,
                    material: 0,
                },
            ],
        );
        let result =
            generate_collision(&raw, "quantized", &CollisionGenerationSettings::default()).unwrap();
        assert_eq!(result.stats.sanitized_faces, 1);
        assert_eq!(result.stats.sanitized_vertices, 3);
        assert_eq!(result.mesh.faces.len(), 1);
        assert!(result.mesh.vertices.iter().all(|vertex| {
            [vertex.x, vertex.y, vertex.z]
                .iter()
                .all(|value| (value * COL_VERTEX_SCALE).fract() == 0.0)
        }));
    }

    fn vertical_quad(x_min: f32, x_max: f32, y: f32, z_min: f32, z_max: f32) -> [V3; 4] {
        [
            V3 {
                x: x_min,
                y,
                z: z_min,
            },
            V3 {
                x: x_max,
                y,
                z: z_min,
            },
            V3 {
                x: x_max,
                y,
                z: z_max,
            },
            V3 {
                x: x_min,
                y,
                z: z_max,
            },
        ]
    }

    fn surface_settings() -> CollisionGenerationSettings {
        CollisionGenerationSettings::for_preset(
            CollisionGenerationPreset::Surface,
            0,
            vec![Some(4)],
        )
    }

    #[test]
    fn flat_window_card_backed_by_larger_wall_is_discarded() {
        let mut vertices = Vec::new();
        let mut triangles = Vec::new();
        push_quad(
            &mut vertices,
            &mut triangles,
            vertical_quad(-5.0, 5.0, 0.0, -5.0, 5.0),
            0,
        );
        push_quad(
            &mut vertices,
            &mut triangles,
            vertical_quad(-1.0, 1.0, COL_VERTEX_STEP * 4.0, -2.0, 2.0),
            0,
        );
        let result = generate_collision(
            &raw_mesh(vertices, triangles),
            "window_card",
            &surface_settings(),
        )
        .unwrap();

        assert_eq!(result.stats.components, 2);
        assert_eq!(result.stats.discarded_backed_flat_components, 1);
        assert_eq!(result.mesh.faces.len(), 2);
        assert!(
            result
                .mesh
                .vertices
                .iter()
                .all(|vertex| vertex.y.abs() <= COL_VERTEX_STEP)
        );
    }

    #[test]
    fn standalone_flat_wall_is_retained() {
        let mut vertices = Vec::new();
        let mut triangles = Vec::new();
        push_quad(
            &mut vertices,
            &mut triangles,
            vertical_quad(-2.0, 2.0, 0.0, -2.0, 2.0),
            0,
        );
        let result = generate_collision(
            &raw_mesh(vertices, triangles),
            "standalone_wall",
            &surface_settings(),
        )
        .unwrap();

        assert_eq!(result.stats.discarded_backed_flat_components, 0);
        assert_eq!(result.mesh.faces.len(), 2);
    }

    #[test]
    fn standalone_flat_floor_is_retained() {
        let mut vertices = Vec::new();
        let mut triangles = Vec::new();
        push_quad(
            &mut vertices,
            &mut triangles,
            [
                V3 {
                    x: -3.0,
                    y: -3.0,
                    z: 0.0,
                },
                V3 {
                    x: 3.0,
                    y: -3.0,
                    z: 0.0,
                },
                V3 {
                    x: 3.0,
                    y: 3.0,
                    z: 0.0,
                },
                V3 {
                    x: -3.0,
                    y: 3.0,
                    z: 0.0,
                },
            ],
            0,
        );
        let result = generate_collision(
            &raw_mesh(vertices, triangles),
            "standalone_floor",
            &surface_settings(),
        )
        .unwrap();

        assert_eq!(result.stats.discarded_backed_flat_components, 0);
        assert_eq!(result.mesh.faces.len(), 2);
    }

    #[test]
    fn similarly_sized_parallel_planes_are_both_retained() {
        let mut vertices = Vec::new();
        let mut triangles = Vec::new();
        push_quad(
            &mut vertices,
            &mut triangles,
            vertical_quad(-2.0, 2.0, 0.0, -2.0, 2.0),
            0,
        );
        push_quad(
            &mut vertices,
            &mut triangles,
            vertical_quad(-2.0, 2.0, COL_VERTEX_STEP * 4.0, -2.0, 2.0),
            0,
        );
        let result = generate_collision(
            &raw_mesh(vertices, triangles),
            "parallel_walls",
            &surface_settings(),
        )
        .unwrap();

        assert_eq!(result.stats.discarded_backed_flat_components, 0);
        assert_eq!(result.mesh.faces.len(), 4);
    }

    #[test]
    fn flat_card_over_a_real_opening_is_retained() {
        let mut vertices = Vec::new();
        let mut triangles = Vec::new();
        for corners in [
            vertical_quad(-5.0, -1.0, 0.0, -5.0, 5.0),
            vertical_quad(1.0, 5.0, 0.0, -5.0, 5.0),
            vertical_quad(-1.0, 1.0, 0.0, 2.0, 5.0),
            vertical_quad(-1.0, 1.0, 0.0, -5.0, -2.0),
        ] {
            push_quad(&mut vertices, &mut triangles, corners, 0);
        }
        push_quad(
            &mut vertices,
            &mut triangles,
            vertical_quad(-1.0, 1.0, COL_VERTEX_STEP * 4.0, -2.0, 2.0),
            0,
        );
        let result = generate_collision(
            &raw_mesh(vertices, triangles),
            "window_opening",
            &surface_settings(),
        )
        .unwrap();

        assert_eq!(result.stats.discarded_backed_flat_components, 0);
        assert!(result.mesh.faces.len() >= 2);
        assert!(
            result
                .mesh
                .vertices
                .iter()
                .any(|vertex| { (vertex.y - COL_VERTEX_STEP * 4.0).abs() <= COL_VERTEX_STEP })
        );
    }

    #[test]
    fn generation_is_deterministic() {
        let raw = sphere_mesh(2.0, 8, 16);
        let settings = CollisionGenerationSettings::for_preset(
            CollisionGenerationPreset::Auto,
            3,
            vec![Some(9)],
        );
        let first = generate_collision(&raw, "same", &settings).unwrap();
        let second = generate_collision(&raw, "same", &settings).unwrap();
        assert!(first == second);
    }

    /// Developer cleanup utility for an existing replacement archive. Run with:
    /// `EAGLE_CLEAR_LOD_COLLISION_ARCHIVE=/path/to/light_mapper_replacements.img
    /// cargo test clear_generated_lod_collision_archive -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn clear_generated_lod_collision_archive() {
        let archive = PathBuf::from(
            std::env::var("EAGLE_CLEAR_LOD_COLLISION_ARCHIVE")
                .expect("EAGLE_CLEAR_LOD_COLLISION_ARCHIVE must name an IMG archive"),
        );
        let entries = parse_img(&archive);
        assert!(
            !entries.is_empty(),
            "{} contains no IMG entries",
            archive.display()
        );

        let file_name = archive
            .file_name()
            .and_then(|name| name.to_str())
            .expect("archive must have a file name");
        let backup = archive.with_file_name(format!("{file_name}.lod-box-backup"));
        assert!(
            !backup.exists(),
            "refusing to overwrite existing backup {}",
            backup.display()
        );

        let mut packed = Vec::with_capacity(entries.len());
        let mut cleared = 0usize;
        for entry in entries {
            let mut bytes = read_img_entry(&entry);
            bytes.truncate(replacement_entry_len(&entry.name, &bytes));
            let key = lower(&entry.name);
            if key.ends_with("_lod.col") {
                let model_name = entry
                    .name
                    .strip_suffix(".col")
                    .or_else(|| entry.name.strip_suffix(".COL"))
                    .unwrap_or(&entry.name);
                let mesh = CollisionMesh {
                    name: model_name.to_string(),
                    spheres: Vec::new(),
                    boxes: Vec::new(),
                    vertices: Vec::new(),
                    faces: Vec::new(),
                    bounds: Bounds {
                        min: Vec3::ZERO,
                        max: Vec3::ZERO,
                    },
                    shadow_vertices: Vec::new(),
                    shadow_faces: Vec::new(),
                };
                let template = col_write_template(&bytes, &entry.name);
                bytes = write_col_mesh_from_template(&template, &mesh)
                    .expect("could not serialize empty LOD collision");
                set_col_model_names_from_entry(&mut bytes, &entry.name);
                let issues = validate_col_for_game_load(&entry.name, &bytes);
                assert!(
                    !col_validation_has_errors(&issues),
                    "{} failed game-load validation",
                    entry.name
                );
                cleared += 1;
            }
            packed.push((entry.name, bytes));
        }
        assert!(cleared > 0, "archive contains no `_lod.col` entries");

        fs::copy(&archive, &backup).expect("could not create LOD collision backup");
        let temp = archive.with_file_name(format!(".{file_name}.remove-lod.tmp"));
        assert!(
            !temp.exists(),
            "temporary path already exists: {}",
            temp.display()
        );
        write_img_archive(&temp, &packed).expect("could not write cleaned archive");
        assert_eq!(parse_img(&temp).len(), packed.len());
        fs::rename(&temp, &archive).expect("could not install cleaned archive");
        println!(
            "cleared {cleared} LOD collision entries in {}; backup {}",
            archive.display(),
            backup.display()
        );
    }
}
