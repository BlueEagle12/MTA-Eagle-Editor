//! In-editor DFF optimization and repair passes.
//!
//! Everything in this module operates on an already parsed [`RawMesh`], which
//! is what the DFF editor holds while a model is open. That keeps the "Optimize
//! DFF" button in the editor working on exactly the geometry the viewport is
//! showing, and it lets every pass be unit tested without touching RenderWare
//! byte streams.
//!
//! Two invariants matter for every pass in here:
//!
//! * `RawMeshComponent` stores half-open `tri_start..tri_end` and
//!   `vertex_start..vertex_end` ranges into the flat mesh arrays. Triangles may
//!   only be reordered *within* a component range, and any removal has to shift
//!   the ranges of the components that follow.
//! * Several vertex streams are optional. A stream is only valid when it is
//!   empty or exactly `vertices.len()` long, so every remap has to preserve
//!   that.

use super::super::*;

/// Which passes the user enabled in the Optimize DFF dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DffOptimizeOptions {
    /// Move alpha-blended faces to the end of each component's triangle list.
    pub(crate) reorder_transparent_faces: bool,
    /// Additionally sort those alpha faces back-to-front about the mesh centre.
    pub(crate) depth_sort_transparent_faces: bool,
    /// Drop triangles with repeated indices or zero area.
    pub(crate) remove_degenerate_faces: bool,
    /// Drop triangles that duplicate an earlier triangle in the same material.
    pub(crate) remove_duplicate_faces: bool,
    /// Collapse vertices that share every attribute.
    pub(crate) weld_vertices: bool,
    /// Drop vertices no triangle references.
    pub(crate) remove_unused_vertices: bool,
    /// Collapse materials whose texture and RGBA/surface values all match.
    pub(crate) merge_duplicate_materials: bool,
    /// Drop material slots no triangle references.
    pub(crate) remove_unused_materials: bool,
    /// Reorder indices for the post-transform vertex cache.
    pub(crate) optimize_vertex_cache: bool,
    /// Fill in a missing day or night prelight stream.
    pub(crate) fix_prelighting: bool,
}

impl Default for DffOptimizeOptions {
    fn default() -> Self {
        Self {
            reorder_transparent_faces: true,
            depth_sort_transparent_faces: true,
            remove_degenerate_faces: true,
            remove_duplicate_faces: true,
            weld_vertices: false,
            remove_unused_vertices: true,
            merge_duplicate_materials: false,
            remove_unused_materials: true,
            optimize_vertex_cache: false,
            fix_prelighting: true,
        }
    }
}

impl DffOptimizeOptions {
    pub(crate) fn any(self) -> bool {
        self.reorder_transparent_faces
            || self.remove_degenerate_faces
            || self.remove_duplicate_faces
            || self.weld_vertices
            || self.remove_unused_vertices
            || self.merge_duplicate_materials
            || self.remove_unused_materials
            || self.optimize_vertex_cache
            || self.fix_prelighting
    }
}

/// Per-pass counters, used to build the status line after a run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct DffOptimizeReport {
    pub(crate) reordered_faces: usize,
    pub(crate) degenerate_faces_removed: usize,
    pub(crate) duplicate_faces_removed: usize,
    pub(crate) vertices_welded: usize,
    pub(crate) unused_vertices_removed: usize,
    pub(crate) duplicate_materials_merged: usize,
    pub(crate) unused_materials_removed: usize,
    pub(crate) cache_optimized_components: usize,
    pub(crate) prelight_streams_filled: usize,
}

impl DffOptimizeReport {
    pub(crate) fn changed(self) -> bool {
        self != Self::default()
    }

    /// Human-readable one-line summary for the editor status bar.
    pub(crate) fn summary(self) -> String {
        if !self.changed() {
            return "Optimize DFF: nothing to change".to_string();
        }
        let mut parts = Vec::new();
        if self.reordered_faces > 0 {
            parts.push(format!("{} face(s) reordered", self.reordered_faces));
        }
        if self.degenerate_faces_removed > 0 {
            parts.push(format!(
                "{} degenerate face(s)",
                self.degenerate_faces_removed
            ));
        }
        if self.duplicate_faces_removed > 0 {
            parts.push(format!(
                "{} duplicate face(s)",
                self.duplicate_faces_removed
            ));
        }
        if self.vertices_welded > 0 {
            parts.push(format!("{} vertex weld(s)", self.vertices_welded));
        }
        if self.unused_vertices_removed > 0 {
            parts.push(format!(
                "{} unused vertex(es)",
                self.unused_vertices_removed
            ));
        }
        if self.duplicate_materials_merged > 0 {
            parts.push(format!(
                "{} duplicate material(s)",
                self.duplicate_materials_merged
            ));
        }
        if self.unused_materials_removed > 0 {
            parts.push(format!(
                "{} unused material(s)",
                self.unused_materials_removed
            ));
        }
        if self.cache_optimized_components > 0 {
            parts.push(format!(
                "{} component(s) cache-optimized",
                self.cache_optimized_components
            ));
        }
        if self.prelight_streams_filled > 0 {
            parts.push(format!(
                "{} prelight stream(s) filled",
                self.prelight_streams_filled
            ));
        }
        format!("Optimize DFF: {}", parts.join(", "))
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Half-open triangle ranges that may be reordered independently.
///
/// A DFF without component records is treated as one range covering the whole
/// triangle list, which is what `parse_dff_mesh` produces for simple models.
fn component_tri_ranges(raw: &RawMesh) -> Vec<(usize, usize)> {
    let total = raw.triangles.len();
    if raw.components.is_empty() {
        return if total == 0 {
            Vec::new()
        } else {
            vec![(0, total)]
        };
    }
    raw.components
        .iter()
        .map(|component| (component.tri_start.min(total), component.tri_end.min(total)))
        .filter(|(start, end)| start < end)
        .collect()
}

/// Half-open vertex ranges. Welding never merges across a component boundary,
/// otherwise the component vertex ranges stop describing contiguous blocks.
fn component_vertex_ranges(raw: &RawMesh) -> Vec<(usize, usize)> {
    let total = raw.vertices.len();
    if raw.components.is_empty() {
        return if total == 0 {
            Vec::new()
        } else {
            vec![(0, total)]
        };
    }
    raw.components
        .iter()
        .map(|component| {
            (
                component.vertex_start.min(total),
                component.vertex_end.min(total),
            )
        })
        .filter(|(start, end)| start < end)
        .collect()
}

fn quantize(value: f32) -> i64 {
    // 1/4096 of a unit is far below anything RenderWare stores meaningfully and
    // keeps welding stable against float noise from earlier edits.
    if value.is_finite() {
        (value * 4096.0).round() as i64
    } else {
        i64::MIN
    }
}

fn triangle_centroid(raw: &RawMesh, tri: &Tri) -> Option<V3> {
    let a = raw.vertices.get(tri.a as usize)?;
    let b = raw.vertices.get(tri.b as usize)?;
    let c = raw.vertices.get(tri.c as usize)?;
    Some(V3 {
        x: (a.x + b.x + c.x) / 3.0,
        y: (a.y + b.y + c.y) / 3.0,
        z: (a.z + b.z + c.z) / 3.0,
    })
}

/// Centre of the axis-aligned bounds of the whole mesh. Used as the reference
/// point for the back-to-front alpha sort; a static model has no camera, and
/// sorting outward from the model centre is the ordering that renders correctly
/// for the common cases (glass shells, foliage cards, fences).
fn mesh_center(raw: &RawMesh) -> V3 {
    if raw.vertices.is_empty() {
        return V3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
    }
    let mut min = raw.vertices[0];
    let mut max = raw.vertices[0];
    for vertex in &raw.vertices {
        min.x = min.x.min(vertex.x);
        min.y = min.y.min(vertex.y);
        min.z = min.z.min(vertex.z);
        max.x = max.x.max(vertex.x);
        max.y = max.y.max(vertex.y);
        max.z = max.z.max(vertex.z);
    }
    V3 {
        x: (min.x + max.x) * 0.5,
        y: (min.y + max.y) * 0.5,
        z: (min.z + max.z) * 0.5,
    }
}

/// Apply a permutation to the triangles inside `start..end`.
///
/// `order` holds absolute triangle indices drawn from that same range.
fn apply_triangle_order(raw: &mut RawMesh, start: usize, order: &[usize]) -> usize {
    let reordered = order
        .iter()
        .map(|index| raw.triangles[*index])
        .collect::<Vec<_>>();
    let mut moved = 0;
    for (offset, triangle) in reordered.into_iter().enumerate() {
        let slot = start + offset;
        if order[offset] != slot {
            moved += 1;
        }
        raw.triangles[slot] = triangle;
    }
    moved
}

/// Drop the triangles whose `keep` flag is false, shifting component triangle
/// ranges so they keep describing the same faces.
pub(crate) fn retain_raw_triangles(raw: &mut RawMesh, keep: &[bool]) -> usize {
    debug_assert_eq!(keep.len(), raw.triangles.len());
    let removed = keep.iter().filter(|flag| !**flag).count();
    if removed == 0 {
        return 0;
    }
    // remaining_before[i] == how many kept triangles appear before index i.
    let mut remaining_before = Vec::with_capacity(keep.len() + 1);
    let mut running = 0usize;
    for flag in keep {
        remaining_before.push(running);
        if *flag {
            running += 1;
        }
    }
    remaining_before.push(running);
    let total = raw.triangles.len();
    for component in &mut raw.components {
        let start = component.tri_start.min(total);
        let end = component.tri_end.min(total);
        component.tri_start = remaining_before[start];
        component.tri_end = remaining_before[end];
    }
    let mut index = 0;
    raw.triangles.retain(|_| {
        let flag = keep[index];
        index += 1;
        flag
    });
    removed
}

/// Rewrite every triangle index through `remap`, which must be
/// `vertices.len()` long.
fn remap_triangle_indices(raw: &mut RawMesh, remap: &[u32]) {
    for triangle in &mut raw.triangles {
        if let Some(target) = remap.get(triangle.a as usize) {
            triangle.a = *target;
        }
        if let Some(target) = remap.get(triangle.b as usize) {
            triangle.b = *target;
        }
        if let Some(target) = remap.get(triangle.c as usize) {
            triangle.c = *target;
        }
    }
}

/// Keep only the flagged vertices across every vertex-aligned stream and shift
/// the component vertex ranges to match.
fn compact_vertices(raw: &mut RawMesh, keep: &[bool]) -> usize {
    debug_assert_eq!(keep.len(), raw.vertices.len());
    let removed = keep.iter().filter(|flag| !**flag).count();
    if removed == 0 {
        return 0;
    }
    let mut remaining_before = Vec::with_capacity(keep.len() + 1);
    let mut running = 0usize;
    for flag in keep {
        remaining_before.push(running);
        if *flag {
            running += 1;
        }
    }
    remaining_before.push(running);

    let mut remap = vec![0u32; keep.len()];
    for (index, slot) in remaining_before.iter().take(keep.len()).enumerate() {
        // Removed vertices are unreferenced by construction, so pointing them at
        // their surviving neighbour is only there to keep the remap total.
        remap[index] = *slot as u32;
    }
    remap_triangle_indices(raw, &remap);

    let total = raw.vertices.len();
    for component in &mut raw.components {
        let start = component.vertex_start.min(total);
        let end = component.vertex_end.min(total);
        component.vertex_start = remaining_before[start];
        component.vertex_end = remaining_before[end];
    }

    retain_vertex_stream(&mut raw.vertices, keep);
    retain_vertex_stream(&mut raw.normals, keep);
    retain_vertex_stream(&mut raw.uvs, keep);
    for stream in &mut raw.secondary_uvs {
        retain_vertex_stream(stream, keep);
    }
    retain_vertex_stream(&mut raw.prelit_colors, keep);
    retain_vertex_stream(&mut raw.prelit_alphas, keep);
    retain_vertex_stream(&mut raw.night_prelit_colors, keep);
    retain_vertex_stream(&mut raw.night_prelit_alphas, keep);
    retain_vertex_stream(&mut raw.light_flags, keep);
    removed
}

/// Filter one optional vertex stream. Streams that are not vertex-aligned are
/// left untouched, matching how the importer treats them.
fn retain_vertex_stream<T>(stream: &mut Vec<T>, keep: &[bool]) {
    if stream.len() != keep.len() {
        return;
    }
    let mut index = 0;
    stream.retain(|_| {
        let flag = keep[index];
        index += 1;
        flag
    });
}

// ---------------------------------------------------------------------------
// Passes
// ---------------------------------------------------------------------------

/// Which material slots should be treated as alpha-blended.
///
/// Material RGBA alpha is authoritative on its own, but a fully opaque material
/// can still be alpha-blended because its texture carries an alpha channel, so
/// the caller passes those in from the texture index.
pub(crate) fn dff_transparent_material_set(
    raw: &RawMesh,
    texture_has_alpha: impl Fn(&str) -> bool,
) -> BTreeSet<usize> {
    (0..raw.materials.len().max(raw.material_textures.len()))
        .filter(|index| {
            let alpha = raw
                .materials
                .get(*index)
                .map(|material| material.alpha)
                .unwrap_or(1.0);
            if alpha < 0.996 {
                return true;
            }
            raw.material_textures
                .get(*index)
                .map(|texture| texture_has_alpha(texture.trim()))
                .unwrap_or(false)
        })
        .collect()
}

/// Move alpha faces after opaque faces inside every component, optionally
/// sorting them back-to-front about the mesh centre.
///
/// This is the fix for the classic San Andreas artifact where a window pane
/// stored before the wall behind it writes depth first and punches a hole
/// through the geometry that should be visible through it.
pub(crate) fn dff_reorder_transparent_faces(
    raw: &mut RawMesh,
    transparent: &BTreeSet<usize>,
    depth_sort: bool,
) -> usize {
    if transparent.is_empty() {
        return 0;
    }
    let center = mesh_center(raw);
    let mut moved = 0;
    for (start, end) in component_tri_ranges(raw) {
        let mut opaque = Vec::new();
        let mut alpha = Vec::new();
        for index in start..end {
            if transparent.contains(&(raw.triangles[index].material as usize)) {
                alpha.push(index);
            } else {
                opaque.push(index);
            }
        }
        if alpha.is_empty() || opaque.is_empty() && !depth_sort {
            continue;
        }
        if depth_sort {
            // Farthest from the model centre first so nearer alpha surfaces
            // blend over the ones behind them.
            let mut keyed = alpha
                .iter()
                .map(|index| {
                    let distance = triangle_centroid(raw, &raw.triangles[*index])
                        .map(|centroid| {
                            let dx = centroid.x - center.x;
                            let dy = centroid.y - center.y;
                            let dz = centroid.z - center.z;
                            dx * dx + dy * dy + dz * dz
                        })
                        .unwrap_or(0.0);
                    (distance, *index)
                })
                .collect::<Vec<_>>();
            // Stable, and ties keep their original file order.
            keyed.sort_by(|a, b| b.0.total_cmp(&a.0));
            alpha = keyed.into_iter().map(|(_, index)| index).collect();
        }
        let mut order = opaque;
        order.extend(alpha);
        moved += apply_triangle_order(raw, start, &order);
    }
    moved
}

/// Remove triangles with a repeated index, an out-of-range index, or three
/// coincident positions.
pub(crate) fn dff_remove_degenerate_faces(raw: &mut RawMesh) -> usize {
    let vertex_count = raw.vertices.len() as u32;
    let keep = raw
        .triangles
        .iter()
        .map(|triangle| {
            if triangle.a >= vertex_count
                || triangle.b >= vertex_count
                || triangle.c >= vertex_count
            {
                return false;
            }
            if triangle.a == triangle.b || triangle.b == triangle.c || triangle.a == triangle.c {
                return false;
            }
            let a = raw.vertices[triangle.a as usize];
            let b = raw.vertices[triangle.b as usize];
            let c = raw.vertices[triangle.c as usize];
            let key = |v: V3| (quantize(v.x), quantize(v.y), quantize(v.z));
            let (ka, kb, kc) = (key(a), key(b), key(c));
            ka != kb && kb != kc && ka != kc
        })
        .collect::<Vec<_>>();
    retain_raw_triangles(raw, &keep)
}

/// Remove triangles that repeat an earlier triangle with the same material and
/// the same three corner vertices, in any winding.
pub(crate) fn dff_remove_duplicate_faces(raw: &mut RawMesh) -> usize {
    let mut seen = BTreeSet::new();
    let keep = raw
        .triangles
        .iter()
        .map(|triangle| {
            let mut corners = [triangle.a, triangle.b, triangle.c];
            corners.sort_unstable();
            seen.insert((triangle.material, corners))
        })
        .collect::<Vec<_>>();
    retain_raw_triangles(raw, &keep)
}

/// Collapse vertices inside the same component that agree on every stream.
///
/// Returns the number of vertices that were merged away. Callers normally run
/// [`dff_remove_unused_vertices`] straight afterwards to actually shrink the
/// buffers.
pub(crate) fn dff_weld_vertices(raw: &mut RawMesh) -> usize {
    let count = raw.vertices.len();
    if count == 0 {
        return 0;
    }
    let has_normals = raw.normals.len() == count;
    let has_uvs = raw.uvs.len() == count;
    let has_prelit = raw.prelit_colors.len() == count;
    let has_prelit_alpha = raw.prelit_alphas.len() == count;
    let has_night = raw.night_prelit_colors.len() == count;
    let has_night_alpha = raw.night_prelit_alphas.len() == count;
    let has_flags = raw.light_flags.len() == count;
    let secondary = raw
        .secondary_uvs
        .iter()
        .filter(|stream| stream.len() == count)
        .collect::<Vec<_>>();

    let mut remap = (0..count as u32).collect::<Vec<_>>();
    let mut welded = 0;
    let ranges = component_vertex_ranges(raw);
    for (start, end) in ranges {
        let mut first_seen: BTreeMap<Vec<i64>, u32> = BTreeMap::new();
        for index in start..end {
            let mut key = Vec::with_capacity(16);
            let position = raw.vertices[index];
            key.extend([
                quantize(position.x),
                quantize(position.y),
                quantize(position.z),
            ]);
            if has_normals {
                let normal = raw.normals[index];
                key.extend([quantize(normal.x), quantize(normal.y), quantize(normal.z)]);
            }
            if has_uvs {
                let uv = raw.uvs[index];
                key.extend([quantize(uv.u), quantize(uv.v)]);
            }
            for stream in &secondary {
                let uv = stream[index];
                key.extend([quantize(uv.u), quantize(uv.v)]);
            }
            if has_prelit {
                let color = raw.prelit_colors[index];
                key.extend([quantize(color.x), quantize(color.y), quantize(color.z)]);
            }
            if has_prelit_alpha {
                key.push(quantize(raw.prelit_alphas[index]));
            }
            if has_night {
                let color = raw.night_prelit_colors[index];
                key.extend([quantize(color.x), quantize(color.y), quantize(color.z)]);
            }
            if has_night_alpha {
                key.push(quantize(raw.night_prelit_alphas[index]));
            }
            if has_flags {
                key.push(i64::from(raw.light_flags[index]));
            }
            match first_seen.get(&key) {
                Some(target) => {
                    remap[index] = *target;
                    welded += 1;
                }
                None => {
                    first_seen.insert(key, index as u32);
                }
            }
        }
    }
    if welded == 0 {
        return 0;
    }
    remap_triangle_indices(raw, &remap);
    welded
}

/// Drop vertices that no triangle references.
///
/// A component that owns no triangles keeps all of its vertices: those are
/// dummy/frame-only records where an "unused" vertex is not actually garbage.
pub(crate) fn dff_remove_unused_vertices(raw: &mut RawMesh) -> usize {
    let count = raw.vertices.len();
    if count == 0 {
        return 0;
    }
    let mut used = vec![false; count];
    for triangle in &raw.triangles {
        for index in [triangle.a, triangle.b, triangle.c] {
            if let Some(slot) = used.get_mut(index as usize) {
                *slot = true;
            }
        }
    }
    let mut keep = used.clone();
    if !raw.components.is_empty() {
        // Start from "keep everything" and only prune inside components that
        // actually draw something.
        keep = vec![true; count];
        let triangle_total = raw.triangles.len();
        for component in &raw.components {
            let tri_start = component.tri_start.min(triangle_total);
            let tri_end = component.tri_end.min(triangle_total);
            if tri_start >= tri_end {
                continue;
            }
            let start = component.vertex_start.min(count);
            let end = component.vertex_end.min(count);
            for slot in keep.iter_mut().take(end).skip(start) {
                *slot = false;
            }
            for index in start..end {
                if used[index] {
                    keep[index] = true;
                }
            }
        }
    }
    compact_vertices(raw, &keep)
}

fn dff_material_identity(raw: &RawMesh, index: usize) -> (String, [i64; 7]) {
    let texture = raw
        .material_textures
        .get(index)
        .map(|name| lower(name.trim()))
        .unwrap_or_default();
    let material = raw
        .materials
        .get(index)
        .copied()
        .unwrap_or_else(default_dff_material);
    // Animated materials are keyed by slot, so never fold two slots that
    // carry different animation name lists together.
    let animation = raw
        .material_animations
        .get(index)
        .map(|anim| anim.names.join("\u{1}"))
        .unwrap_or_default();
    (
        format!("{texture}\u{0}{animation}"),
        [
            quantize(material.color.x),
            quantize(material.color.y),
            quantize(material.color.z),
            quantize(material.alpha),
            quantize(material.ambient),
            quantize(material.diffuse),
            quantize(material.specular),
        ],
    )
}

/// Count material slots that can be folded into an earlier identical slot.
/// This is the read-only counterpart to [`dff_merge_duplicate_materials`] and
/// lets Validation report the same issue that the optimizer can safely fix.
pub(crate) fn dff_duplicate_material_count(raw: &RawMesh) -> usize {
    let slots = raw.materials.len().max(raw.material_textures.len());
    let mut seen = BTreeSet::new();
    (0..slots)
        .filter(|index| !seen.insert(dff_material_identity(raw, *index)))
        .count()
}

/// Count non-empty texture names referenced by more than one material slot,
/// regardless of surface properties. This is intentionally broader than the
/// safe duplicate-material check: generated LOD atlases must have exactly one
/// material per packed sheet even when their source slots differed slightly.
pub(crate) fn dff_repeated_texture_material_count(raw: &RawMesh) -> usize {
    let slots = raw.materials.len().max(raw.material_textures.len());
    let mut seen = BTreeSet::new();
    (0..slots)
        .filter_map(|index| raw.material_textures.get(index))
        .map(|name| lower(name.trim()))
        .filter(|name| !name.is_empty())
        .filter(|name| !seen.insert(name.clone()))
        .count()
}

/// Collapse material slots whose texture name and every material value match,
/// pointing their triangles at the first slot that used those values.
pub(crate) fn dff_merge_duplicate_materials(raw: &mut RawMesh) -> usize {
    let slots = raw.materials.len().max(raw.material_textures.len());
    if slots < 2 {
        return 0;
    }
    let mut first_seen: BTreeMap<(String, [i64; 7]), usize> = BTreeMap::new();
    let mut remap = (0..slots).collect::<Vec<_>>();
    let mut merged = 0;
    for index in 0..slots {
        let key = dff_material_identity(raw, index);
        match first_seen.get(&key) {
            Some(target) => {
                remap[index] = *target;
                merged += 1;
            }
            None => {
                first_seen.insert(key, index);
            }
        }
    }
    if merged == 0 {
        return 0;
    }
    for triangle in &mut raw.triangles {
        if let Some(target) = remap.get(triangle.material as usize) {
            triangle.material = *target as u16;
        }
    }
    merged
}

/// Drop material slots no triangle references, renumbering the survivors.
pub(crate) fn dff_remove_unused_materials(raw: &mut RawMesh) -> usize {
    let slots = raw.materials.len().max(raw.material_textures.len());
    if slots == 0 {
        return 0;
    }
    let mut used = vec![false; slots];
    for triangle in &raw.triangles {
        if let Some(slot) = used.get_mut(triangle.material as usize) {
            *slot = true;
        }
    }
    // Never leave a DFF with zero materials; RenderWare needs at least one.
    if used.iter().all(|flag| !*flag) {
        return 0;
    }
    let removed = used.iter().filter(|flag| !**flag).count();
    if removed == 0 {
        return 0;
    }
    let mut remap = vec![0u16; slots];
    let mut next = 0u16;
    for (index, flag) in used.iter().enumerate() {
        if *flag {
            remap[index] = next;
            next += 1;
        }
    }
    for triangle in &mut raw.triangles {
        if let Some(target) = remap.get(triangle.material as usize) {
            triangle.material = *target;
        }
    }
    let mut index = 0;
    raw.materials.retain(|_| {
        let flag = used.get(index).copied().unwrap_or(false);
        index += 1;
        flag
    });
    let mut index = 0;
    raw.material_textures.retain(|_| {
        let flag = used.get(index).copied().unwrap_or(false);
        index += 1;
        flag
    });
    if raw.material_animations.len() == slots {
        let mut index = 0;
        raw.material_animations.retain(|_| {
            let flag = used[index];
            index += 1;
            flag
        });
    }
    removed
}

/// Reorder each component's triangles for the post-transform vertex cache.
///
/// Runs per material run so material batching is preserved, and is a no-op for
/// components that would be split across alpha ordering later.
pub(crate) fn dff_optimize_vertex_cache(raw: &mut RawMesh) -> usize {
    let vertex_count = raw.vertices.len();
    if vertex_count == 0 {
        return 0;
    }
    let mut components = 0;
    for (start, end) in component_tri_ranges(raw) {
        // Group by material, keeping the materials in the order they first
        // appear. Reordering *across* materials would rewrite the material
        // splits RenderWare writes on export, so only the triangles inside one
        // split are allowed to move.
        let mut groups: Vec<(u16, Vec<usize>)> = Vec::new();
        for index in start..end {
            let material = raw.triangles[index].material;
            match groups.iter_mut().find(|(id, _)| *id == material) {
                Some((_, group)) => group.push(index),
                None => groups.push((material, vec![index])),
            }
        }
        let mut order = Vec::with_capacity(end - start);
        let mut changed = false;
        for (_, group) in groups {
            if group.len() < 3 {
                order.extend(group);
                continue;
            }
            let indices = group
                .iter()
                .flat_map(|index| {
                    let triangle = raw.triangles[*index];
                    [triangle.a, triangle.b, triangle.c]
                })
                .collect::<Vec<u32>>();
            let optimized = meshopt::optimize_vertex_cache(&indices, vertex_count);
            // Map the optimized index stream back onto whole triangles. Corners
            // are sorted for the lookup so a rotated-but-equivalent triple still
            // matches; if anything fails to line up, this group is left alone.
            let mut lookup: BTreeMap<[u32; 3], Vec<usize>> = BTreeMap::new();
            for index in group.iter().rev() {
                let triangle = raw.triangles[*index];
                let mut key = [triangle.a, triangle.b, triangle.c];
                key.sort_unstable();
                lookup.entry(key).or_default().push(*index);
            }
            let mut remapped = Vec::with_capacity(group.len());
            let mut ok = true;
            for triple in optimized.chunks_exact(3) {
                let mut key = [triple[0], triple[1], triple[2]];
                key.sort_unstable();
                match lookup.get_mut(&key).and_then(|slots| slots.pop()) {
                    Some(index) => remapped.push(index),
                    None => {
                        ok = false;
                        break;
                    }
                }
            }
            if ok && remapped.len() == group.len() {
                changed |= remapped != group;
                order.extend(remapped);
            } else {
                order.extend(group);
            }
        }
        if changed && order.len() == end - start {
            apply_triangle_order(raw, start, &order);
            components += 1;
        }
    }
    components
}

/// Fill a missing day or night prelight stream from the other one, or with
/// neutral white when neither exists.
///
/// A DFF with no prelighting at all renders black under MTA's vertex lighting,
/// which is the single most common "my model is pitch black" report.
pub(crate) fn dff_fix_prelighting(raw: &mut RawMesh) -> usize {
    let count = raw.vertices.len();
    if count == 0 {
        return 0;
    }
    let day_ok = raw.prelit_colors.len() == count;
    let night_ok = raw.night_prelit_colors.len() == count;
    let mut filled = 0;
    if !day_ok && !night_ok {
        raw.prelit_colors = vec![
            V3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            };
            count
        ];
        raw.night_prelit_colors = raw.prelit_colors.clone();
        filled += 2;
    } else if !day_ok {
        raw.prelit_colors = raw.night_prelit_colors.clone();
        filled += 1;
    } else if !night_ok {
        raw.night_prelit_colors = raw.prelit_colors.clone();
        filled += 1;
    }
    if raw.prelit_alphas.len() != count {
        raw.prelit_alphas = if raw.night_prelit_alphas.len() == count {
            raw.night_prelit_alphas.clone()
        } else {
            vec![1.0; count]
        };
    }
    if raw.night_prelit_alphas.len() != count {
        raw.night_prelit_alphas = raw.prelit_alphas.clone();
    }
    filled
}

// ---------------------------------------------------------------------------
// Driver
// ---------------------------------------------------------------------------

/// Run every enabled pass in the only order that composes correctly.
///
/// The ordering matters and is not arbitrary:
///
/// 1. Face removal first, so later passes do not spend work on faces that are
///    about to disappear. Removal preserves the relative order of survivors.
/// 2. Vertex welding and compaction, which only touch indices.
/// 3. Vertex cache optimisation, which *does* permute triangles.
/// 4. Transparent face ordering, so nothing after it can shuffle alpha faces
///    back out of place.
/// 5. Material folding last, because it only renumbers material slots and
///    never moves a triangle — which also means `transparent` stays valid for
///    step 4 above.
pub(crate) fn run_dff_optimize(
    raw: &mut RawMesh,
    options: DffOptimizeOptions,
    transparent: &BTreeSet<usize>,
) -> DffOptimizeReport {
    let mut report = DffOptimizeReport::default();
    if options.remove_degenerate_faces {
        report.degenerate_faces_removed = dff_remove_degenerate_faces(raw);
    }
    if options.remove_duplicate_faces {
        report.duplicate_faces_removed = dff_remove_duplicate_faces(raw);
    }
    if options.weld_vertices {
        report.vertices_welded = dff_weld_vertices(raw);
    }
    if options.remove_unused_vertices || options.weld_vertices {
        report.unused_vertices_removed = dff_remove_unused_vertices(raw);
    }
    if options.fix_prelighting {
        report.prelight_streams_filled = dff_fix_prelighting(raw);
    }
    if options.optimize_vertex_cache {
        report.cache_optimized_components = dff_optimize_vertex_cache(raw);
    }
    if options.reorder_transparent_faces {
        report.reordered_faces =
            dff_reorder_transparent_faces(raw, transparent, options.depth_sort_transparent_faces);
    }
    if options.merge_duplicate_materials {
        report.duplicate_materials_merged = dff_merge_duplicate_materials(raw);
    }
    if options.remove_unused_materials || options.merge_duplicate_materials {
        report.unused_materials_removed = dff_remove_unused_materials(raw);
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn material(alpha: f32) -> RawMaterial {
        RawMaterial {
            color: V3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
            alpha,
            ambient: 1.0,
            specular: 0.0,
            diffuse: 1.0,
        }
    }

    /// Two stacked quads: an opaque wall at y=0 and a glass pane at y=1, with
    /// the glass stored first. That is exactly the ordering that makes San
    /// Andreas cull the wall behind the glass.
    fn glass_before_wall() -> RawMesh {
        let mut raw = RawMesh::default();
        raw.vertices = vec![
            // glass, farther from centre
            V3 {
                x: -1.0,
                y: 4.0,
                z: 0.0,
            },
            V3 {
                x: 1.0,
                y: 4.0,
                z: 0.0,
            },
            V3 {
                x: 0.0,
                y: 4.0,
                z: 1.0,
            },
            // wall, nearer the centre
            V3 {
                x: -1.0,
                y: 0.5,
                z: 0.0,
            },
            V3 {
                x: 1.0,
                y: 0.5,
                z: 0.0,
            },
            V3 {
                x: 0.0,
                y: 0.5,
                z: 1.0,
            },
        ];
        raw.materials = vec![material(0.5), material(1.0)];
        raw.material_textures = vec!["glass".to_string(), "wall".to_string()];
        raw.triangles = vec![
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
        ];
        raw
    }

    #[test]
    fn transparent_faces_move_after_opaque_faces() {
        let mut raw = glass_before_wall();
        let transparent = BTreeSet::from([0usize]);
        let moved = dff_reorder_transparent_faces(&mut raw, &transparent, false);
        assert_eq!(moved, 2);
        assert_eq!(
            raw.triangles[0].material, 1,
            "opaque wall must render first"
        );
        assert_eq!(raw.triangles[1].material, 0, "glass must render last");
    }

    #[test]
    fn already_ordered_transparent_faces_are_left_alone() {
        let mut raw = glass_before_wall();
        raw.triangles.swap(0, 1);
        let transparent = BTreeSet::from([0usize]);
        assert_eq!(
            dff_reorder_transparent_faces(&mut raw, &transparent, false),
            0
        );
    }

    #[test]
    fn depth_sort_orders_alpha_faces_back_to_front() {
        let mut raw = RawMesh::default();
        raw.materials = vec![material(0.4)];
        raw.material_textures = vec!["glass".to_string()];
        // Four coplanar cards spread unevenly along Z so the bounds centre does
        // not sit at equal distance from any two of them.
        for z in [0.0_f32, 1.0, 3.0, 30.0] {
            let base = raw.vertices.len() as u32;
            raw.vertices.extend([
                V3 { x: 0.0, y: 0.0, z },
                V3 { x: 1.0, y: 0.0, z },
                V3 { x: 0.0, y: 1.0, z },
            ]);
            raw.triangles.push(Tri {
                a: base,
                b: base + 1,
                c: base + 2,
                material: 0,
            });
        }
        let transparent = BTreeSet::from([0usize]);
        dff_reorder_transparent_faces(&mut raw, &transparent, true);
        let center = mesh_center(&raw);
        let distances = raw
            .triangles
            .iter()
            .map(|triangle| {
                let centroid = triangle_centroid(&raw, triangle).unwrap();
                let dx = centroid.x - center.x;
                let dy = centroid.y - center.y;
                let dz = centroid.z - center.z;
                dx * dx + dy * dy + dz * dz
            })
            .collect::<Vec<_>>();
        assert!(
            distances.windows(2).all(|pair| pair[0] >= pair[1]),
            "alpha faces should end up sorted farthest-first, got {distances:?}"
        );
    }

    #[test]
    fn reordering_stays_inside_component_triangle_ranges() {
        let mut raw = glass_before_wall();
        raw.components = vec![
            RawMeshComponent {
                name: "a".to_string(),
                frame_index: None,
                vertex_start: 0,
                vertex_end: 3,
                tri_start: 0,
                tri_end: 1,
                breakable: None,
            },
            RawMeshComponent {
                name: "b".to_string(),
                frame_index: None,
                vertex_start: 3,
                vertex_end: 6,
                tri_start: 1,
                tri_end: 2,
                breakable: None,
            },
        ];
        let transparent = BTreeSet::from([0usize]);
        assert_eq!(
            dff_reorder_transparent_faces(&mut raw, &transparent, true),
            0,
            "single-triangle components have nothing to reorder"
        );
        assert_eq!(raw.triangles[0].material, 0);
    }

    #[test]
    fn degenerate_and_duplicate_faces_are_removed() {
        let mut raw = glass_before_wall();
        raw.triangles.push(Tri {
            a: 0,
            b: 0,
            c: 2,
            material: 0,
        });
        raw.triangles.push(Tri {
            a: 2,
            b: 1,
            c: 0,
            material: 0,
        });
        assert_eq!(dff_remove_degenerate_faces(&mut raw), 1);
        assert_eq!(dff_remove_duplicate_faces(&mut raw), 1);
        assert_eq!(raw.triangles.len(), 2);
    }

    #[test]
    fn removing_triangles_shifts_component_ranges() {
        let mut raw = glass_before_wall();
        raw.triangles.insert(
            0,
            Tri {
                a: 0,
                b: 0,
                c: 0,
                material: 0,
            },
        );
        raw.components = vec![RawMeshComponent {
            name: "a".to_string(),
            frame_index: None,
            vertex_start: 0,
            vertex_end: 6,
            tri_start: 0,
            tri_end: 3,
            breakable: None,
        }];
        dff_remove_degenerate_faces(&mut raw);
        assert_eq!(raw.components[0].tri_start, 0);
        assert_eq!(raw.components[0].tri_end, 2);
    }

    #[test]
    fn unused_materials_are_removed_and_renumbered() {
        let mut raw = glass_before_wall();
        raw.materials.push(material(1.0));
        raw.material_textures.push("unused".to_string());
        assert_eq!(dff_remove_unused_materials(&mut raw), 1);
        assert_eq!(raw.materials.len(), 2);
        assert_eq!(raw.material_textures, vec!["glass", "wall"]);
    }

    #[test]
    fn duplicate_material_count_matches_safe_merge_rules() {
        let mut raw = glass_before_wall();
        raw.materials.push(raw.materials[1]);
        raw.material_textures.push("WALL".to_string());
        raw.material_animations.push(DffMaterialAnim::default());

        assert_eq!(dff_duplicate_material_count(&raw), 1);
        assert_eq!(dff_merge_duplicate_materials(&mut raw), 1);
        assert_eq!(dff_remove_unused_materials(&mut raw), 1);
        assert_eq!(raw.material_textures, vec!["glass", "wall"]);
    }

    #[test]
    fn same_texture_with_different_properties_is_not_redundant() {
        let mut raw = glass_before_wall();
        raw.material_textures[1] = raw.material_textures[0].clone();

        assert_eq!(dff_duplicate_material_count(&raw), 0);
        assert_eq!(dff_repeated_texture_material_count(&raw), 1);
        assert_eq!(dff_merge_duplicate_materials(&mut raw), 0);
    }

    #[test]
    fn welding_collapses_identical_vertices_and_compacts_them() {
        let mut raw = RawMesh::default();
        raw.vertices = vec![
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
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        ];
        raw.materials = vec![material(1.0)];
        raw.material_textures = vec!["wall".to_string()];
        raw.triangles = vec![Tri {
            a: 3,
            b: 1,
            c: 2,
            material: 0,
        }];
        assert_eq!(dff_weld_vertices(&mut raw), 1);
        assert_eq!(dff_remove_unused_vertices(&mut raw), 1);
        assert_eq!(raw.vertices.len(), 3);
        assert!(
            raw.triangles
                .iter()
                .all(|tri| [tri.a, tri.b, tri.c].iter().all(|i| (*i as usize) < 3))
        );
    }

    #[test]
    fn missing_prelight_streams_are_filled() {
        let mut raw = glass_before_wall();
        assert_eq!(dff_fix_prelighting(&mut raw), 2);
        assert_eq!(raw.prelit_colors.len(), raw.vertices.len());
        assert_eq!(raw.night_prelit_colors.len(), raw.vertices.len());
        assert_eq!(raw.prelit_alphas.len(), raw.vertices.len());
    }

    #[test]
    fn driver_reorders_alpha_last_even_with_material_removal_enabled() {
        let mut raw = glass_before_wall();
        raw.materials.push(material(1.0));
        raw.material_textures.push("unused".to_string());
        let transparent = BTreeSet::from([0usize]);
        let report = run_dff_optimize(&mut raw, DffOptimizeOptions::default(), &transparent);
        assert!(report.changed());
        assert_eq!(raw.triangles.last().unwrap().material, 0);
    }
}
