use super::super::*;

struct VertexLightHit {
    mesh_key: String,
    part_idx: usize,
    tri_start: usize,
    world_pos: Vec3,
    world_normal: Vec3,
    weights: [f32; 3],
}

fn ray_triangle_barycentric(
    origin: Vec3,
    dir: Vec3,
    a: Vec3,
    b: Vec3,
    c: Vec3,
) -> Option<(f32, [f32; 3])> {
    let edge1 = b - a;
    let edge2 = c - a;
    let h = dir.cross(edge2);
    let det = edge1.dot(h);
    if det.abs() < 0.000001 {
        return None;
    }
    let inv_det = 1.0 / det;
    let s = origin - a;
    let u = inv_det * s.dot(h);
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(edge1);
    let v = inv_det * dir.dot(q);
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = inv_det * edge2.dot(q);
    (t > 0.0001).then_some((t, [1.0 - u - v, u, v]))
}

fn mix_v3(a: V3, b: V3, t: f32) -> V3 {
    let t = t.clamp(0.0, 1.0);
    v3_clamp01(V3 {
        x: a.x + (b.x - a.x) * t,
        y: a.y + (b.y - a.y) * t,
        z: a.z + (b.z - a.z) * t,
    })
}

fn world_vertex(placement: &Placement, vertex: &Vertex) -> Vec3 {
    let model = placement_matrix(placement).to_cols_array();
    transform_point_gl(&model, vertex.pos)
}

fn projected_brush_distance(pos: Vec3, center: Vec3, normal: Vec3) -> (f32, f32) {
    let delta = pos - center;
    let depth = delta.dot(normal);
    let planar = delta - normal * depth;
    (planar.length(), depth.abs())
}

fn point_aabb_distance(point: Vec3, min: Vec3, max: Vec3) -> f32 {
    let dx = if point.x < min.x {
        min.x - point.x
    } else if point.x > max.x {
        point.x - max.x
    } else {
        0.0
    };
    let dy = if point.y < min.y {
        min.y - point.y
    } else if point.y > max.y {
        point.y - max.y
    } else {
        0.0
    };
    let dz = if point.z < min.z {
        min.z - point.z
    } else if point.z > max.z {
        point.z - max.z
    } else {
        0.0
    };
    vec3(dx, dy, dz).length()
}

fn vertex_pos_key(pos: Vec3) -> (i32, i32, i32) {
    const SCALE: f32 = 1000.0;
    (
        (pos.x * SCALE).round() as i32,
        (pos.y * SCALE).round() as i32,
        (pos.z * SCALE).round() as i32,
    )
}

fn edge_key(a: Vec3, b: Vec3) -> ((i32, i32, i32), (i32, i32, i32)) {
    let a = vertex_pos_key(a);
    let b = vertex_pos_key(b);
    if a <= b { (a, b) } else { (b, a) }
}

fn placement_vertex_samples(
    app: &AppState,
    placement_idx: usize,
    mode: BakeLightMode,
) -> Vec<(String, usize, usize, Vec3, V3)> {
    let Some(placement) = app.placements.get(placement_idx) else {
        return Vec::new();
    };
    let mesh_key = placement_mesh_key(placement, &app.definitions);
    let Some(mesh) = app.meshes.get(&mesh_key) else {
        return Vec::new();
    };
    let mut vertices = Vec::new();
    for (part_idx, part) in mesh.parts.iter().enumerate() {
        for (vertex_idx, vertex) in part.cpu_vertices.iter().enumerate() {
            vertices.push((
                mesh_key.clone(),
                part_idx,
                vertex_idx,
                world_vertex(placement, vertex),
                vertex_bake_color(vertex, mode),
            ));
        }
    }
    vertices
}

fn placement_boundary_vertex_samples(
    app: &AppState,
    placement_idx: usize,
    mode: BakeLightMode,
) -> Vec<(String, usize, usize, Vec3, V3)> {
    let Some(placement) = app.placements.get(placement_idx) else {
        return Vec::new();
    };
    let mesh_key = placement_mesh_key(placement, &app.definitions);
    let Some(mesh) = app.meshes.get(&mesh_key) else {
        return Vec::new();
    };
    let mut edge_counts = HashMap::<((i32, i32, i32), (i32, i32, i32)), usize>::new();
    let mut part_world_positions = Vec::<Vec<Vec3>>::new();
    for part in &mesh.parts {
        let positions: Vec<Vec3> = part
            .cpu_vertices
            .iter()
            .map(|vertex| world_vertex(placement, vertex))
            .collect();
        for tri in positions.chunks_exact(3) {
            for (a, b) in [(0usize, 1usize), (1, 2), (2, 0)] {
                *edge_counts.entry(edge_key(tri[a], tri[b])).or_insert(0) += 1;
            }
        }
        part_world_positions.push(positions);
    }
    let mut boundary_keys = HashSet::<(i32, i32, i32)>::new();
    for ((a, b), count) in edge_counts {
        if count == 1 {
            boundary_keys.insert(a);
            boundary_keys.insert(b);
        }
    }
    let mut vertices = Vec::new();
    for (part_idx, part) in mesh.parts.iter().enumerate() {
        let Some(positions) = part_world_positions.get(part_idx) else {
            continue;
        };
        for (vertex_idx, vertex) in part.cpu_vertices.iter().enumerate() {
            let Some(world_pos) = positions.get(vertex_idx).copied() else {
                continue;
            };
            if boundary_keys.contains(&vertex_pos_key(world_pos)) {
                vertices.push((
                    mesh_key.clone(),
                    part_idx,
                    vertex_idx,
                    world_pos,
                    vertex_bake_color(vertex, mode),
                ));
            }
        }
    }
    vertices
}

/// Reconciles the duplicated triangle corners used by the runtime mesh. DFFs
/// often split a geometric corner into several vertices with different face
/// normals. Baking those copies independently creates a hard light step even
/// though they occupy exactly the same position. Average compatible corners
/// after lighting while keeping nearly-opposite faces separate, so thin walls
/// do not leak lighting from front to back.
fn reconcile_corner_mode(app: &mut AppState, mode: BakeLightMode) -> usize {
    debug_assert!(matches!(mode, BakeLightMode::Day | BakeLightMode::Night));
    const POSITION_SCALE: f32 = 10.0; // 0.1 world-unit matching tolerance.
    // Blend corners up through roughly a right angle, but reject the opposite
    // sides of thin or closed geometry.
    const MIN_NORMAL_DOT: f32 = -0.25;
    type VertexRef = (String, usize, usize, V3, Vec3);
    let mut groups = HashMap::<(i32, i32, i32), Vec<VertexRef>>::new();
    for (placement_idx, placement) in app.placements.iter().enumerate() {
        if app
            .element_states
            .get(placement_idx)
            .is_some_and(|state| state.deleted)
        {
            continue;
        }
        let mesh_key = placement_mesh_key(placement, &app.definitions);
        let Some(mesh) = app.meshes.get(&mesh_key) else {
            continue;
        };
        let model = placement_matrix(placement).to_cols_array();
        for (part_idx, part) in mesh.parts.iter().enumerate() {
            for (vertex_idx, vertex) in part.cpu_vertices.iter().enumerate() {
                let pos = transform_point_gl(&model, vertex.pos);
                let normal = transform_normal_gl(&model, vertex.normal).normalize_or_zero();
                let key = (
                    (pos.x * POSITION_SCALE).round() as i32,
                    (pos.y * POSITION_SCALE).round() as i32,
                    (pos.z * POSITION_SCALE).round() as i32,
                );
                groups.entry(key).or_default().push((
                    mesh_key.clone(),
                    part_idx,
                    vertex_idx,
                    vertex_bake_color(vertex, mode),
                    normal,
                ));
            }
        }
    }

    let mut proposals = HashMap::<(String, usize, usize), (Vec3, usize)>::new();
    for group in groups.values() {
        if group.len() < 2 {
            continue;
        }
        for (mesh_key, part_idx, vertex_idx, _, normal) in group {
            let mut sum = Vec3::ZERO;
            let mut compatible = 0usize;
            for (_, _, _, color, other_normal) in group {
                if normal.dot(*other_normal) >= MIN_NORMAL_DOT {
                    sum += vec3(color.x, color.y, color.z);
                    compatible += 1;
                }
            }
            if compatible < 2 {
                continue;
            }
            let average = sum / compatible as f32;
            let proposal = proposals
                .entry((mesh_key.clone(), *part_idx, *vertex_idx))
                .or_insert((Vec3::ZERO, 0));
            proposal.0 += average;
            proposal.1 += 1;
        }
    }

    let mut applied = 0usize;
    for ((mesh_key, part_idx, vertex_idx), (sum, samples)) in proposals {
        let Some(vertex) = app
            .meshes
            .get_mut(&mesh_key)
            .and_then(|mesh| mesh.parts.get_mut(part_idx))
            .and_then(|part| part.cpu_vertices.get_mut(vertex_idx))
        else {
            continue;
        };
        let color = baked_vec3_to_v3(sum / samples.max(1) as f32);
        match mode {
            BakeLightMode::Day => vertex.day_color = color,
            BakeLightMode::Night => vertex.night_color = color,
            BakeLightMode::Both => unreachable!(),
        }
        apply_vertex_bake_display(vertex, app.bake_settings.light_mode);
        applied += 1;
    }
    applied
}

pub(crate) fn reconcile_baked_corners(app: &mut AppState, mode: BakeLightMode) -> usize {
    match mode {
        BakeLightMode::Day => reconcile_corner_mode(app, BakeLightMode::Day),
        BakeLightMode::Night => reconcile_corner_mode(app, BakeLightMode::Night),
        BakeLightMode::Both => {
            reconcile_corner_mode(app, BakeLightMode::Day)
                + reconcile_corner_mode(app, BakeLightMode::Night)
        }
    }
}

fn source_cell_key(pos: Vec3, cell_size: f32) -> (i32, i32, i32) {
    (
        (pos.x / cell_size).floor() as i32,
        (pos.y / cell_size).floor() as i32,
        (pos.z / cell_size).floor() as i32,
    )
}

fn build_source_vertex_grid(
    sources: &[(Vec3, V3)],
    cell_size: f32,
) -> HashMap<(i32, i32, i32), Vec<usize>> {
    let mut grid = HashMap::<(i32, i32, i32), Vec<usize>>::new();
    for (idx, (pos, _)) in sources.iter().enumerate() {
        grid.entry(source_cell_key(*pos, cell_size))
            .or_default()
            .push(idx);
    }
    grid
}

fn closest_source_vertex(
    sources: &[(Vec3, V3)],
    grid: &HashMap<(i32, i32, i32), Vec<usize>>,
    pos: Vec3,
    cell_size: f32,
    max_distance: f32,
) -> Option<(f32, V3)> {
    if sources.is_empty() || max_distance <= 0.0 || !max_distance.is_finite() {
        return None;
    }
    let base = source_cell_key(pos, cell_size);
    let max_ring = (max_distance / cell_size).ceil().max(1.0) as i32;
    let max_dist2 = max_distance * max_distance;
    let mut best_dist2 = max_dist2;
    let mut best_color = None;
    for ring in 0..=max_ring {
        for x in base.0 - ring..=base.0 + ring {
            for y in base.1 - ring..=base.1 + ring {
                for z in base.2 - ring..=base.2 + ring {
                    if ring > 0
                        && x > base.0 - ring
                        && x < base.0 + ring
                        && y > base.1 - ring
                        && y < base.1 + ring
                        && z > base.2 - ring
                        && z < base.2 + ring
                    {
                        continue;
                    }
                    let Some(indices) = grid.get(&(x, y, z)) else {
                        continue;
                    };
                    for source_idx in indices {
                        let (source_pos, source_color) = sources[*source_idx];
                        let dist2 = (source_pos - pos).length_squared();
                        if dist2 <= best_dist2 {
                            best_dist2 = dist2;
                            best_color = Some(source_color);
                        }
                    }
                }
            }
        }
    }
    best_color.map(|color| (best_dist2.sqrt(), color))
}

fn remove_material_tint(color: V3, material: V3) -> V3 {
    fn channel(value: f32, tint: f32) -> f32 {
        if tint.abs() <= 0.0001 {
            0.0
        } else {
            (value / tint).clamp(0.0, 1.0)
        }
    }
    V3 {
        x: channel(color.x, material.x),
        y: channel(color.y, material.y),
        z: channel(color.z, material.z),
    }
}

/// Returns the material multiplier that was applied to parsed prelight when
/// this render part was compiled.
///
/// The world renderer intentionally leaves textured materials untinted (some
/// GTA assets use their material color as metadata/markers). Untextured parts
/// do receive the authored material color. Vertex-light serialization must
/// invert that exact choice; dividing every part by `material_color` corrupts
/// textured assets into per-material color bands after save/reload.
fn runtime_prelight_tint(part: &RenderPart) -> V3 {
    if part.texture_name.trim().is_empty() {
        part.material_color
    } else {
        neutral_vertex_color()
    }
}

fn serialized_prelight_from_runtime(part: &RenderPart, color: V3) -> V3 {
    remove_material_tint(color, runtime_prelight_tint(part))
}

fn runtime_prelight_from_serialized(part: &RenderPart, color: V3) -> V3 {
    let tint = runtime_prelight_tint(part);
    v3_clamp01(V3 {
        x: color.x * tint.x,
        y: color.y * tint.y,
        z: color.z * tint.z,
    })
}

/// Copies edited colors back into the parsed source mesh without rebuilding
/// its topology or sidecar chunks. Render parts are compiled in sorted
/// (component, material) order, so recreating that grouping gives an exact
/// mapping back to the source vertex indices.
fn apply_runtime_vertex_colors_to_raw(mesh: &RenderMesh, raw: &mut RawMesh) -> Result<(), String> {
    if raw.vertices.is_empty() || raw.triangles.is_empty() {
        return Err("Source DFF has no readable geometry".to_string());
    }
    let component_ranges: Vec<(usize, usize)> = raw
        .components
        .iter()
        .map(|component| (component.tri_start, component.tri_end))
        .collect();
    let mut component_cursor = 0usize;
    let material_count = raw.materials.len().max(raw.material_textures.len()).max(1);
    let mut tris_by_key = BTreeMap::<(usize, usize), Vec<Tri>>::new();
    for (tri_idx, tri) in raw.triangles.iter().enumerate() {
        while component_cursor + 1 < component_ranges.len()
            && tri_idx >= component_ranges[component_cursor].1
        {
            component_cursor += 1;
        }
        let component = if component_ranges.is_empty() {
            0
        } else {
            component_cursor
        };
        let material = (tri.material as usize).min(material_count - 1);
        tris_by_key
            .entry((component, material))
            .or_default()
            .push(*tri);
    }

    let mut day_sum = vec![Vec3::ZERO; raw.vertices.len()];
    let mut night_sum = vec![Vec3::ZERO; raw.vertices.len()];
    let mut samples = vec![0usize; raw.vertices.len()];
    for part in &mesh.parts {
        let Some(tris) = tris_by_key.get(&(part.component, part.material_index)) else {
            return Err(format!(
                "Source DFF topology no longer matches render part {}/{}",
                part.component, part.material_index
            ));
        };
        let expected = tris.len() * 3;
        if part.cpu_vertices.len() != expected {
            return Err(format!(
                "Source DFF topology changed: expected {expected} vertices, found {}",
                part.cpu_vertices.len()
            ));
        }
        for (runtime_vertex, source_idx) in part.cpu_vertices.iter().zip(
            tris.iter()
                .flat_map(|tri| [tri.a as usize, tri.b as usize, tri.c as usize]),
        ) {
            if source_idx >= raw.vertices.len() {
                return Err("Source DFF contains an invalid triangle index".to_string());
            }
            let day = serialized_prelight_from_runtime(part, runtime_vertex.day_color);
            let night = serialized_prelight_from_runtime(part, runtime_vertex.night_color);
            day_sum[source_idx] += vec3(day.x, day.y, day.z);
            night_sum[source_idx] += vec3(night.x, night.y, night.z);
            samples[source_idx] += 1;
        }
    }
    raw.prelit_colors
        .resize(raw.vertices.len(), neutral_vertex_color());
    raw.night_prelit_colors
        .resize(raw.vertices.len(), neutral_vertex_color());
    for idx in 0..raw.vertices.len() {
        if samples[idx] == 0 {
            continue;
        }
        raw.prelit_colors[idx] = baked_vec3_to_v3(day_sum[idx] / samples[idx] as f32);
        raw.night_prelit_colors[idx] = baked_vec3_to_v3(night_sum[idx] / samples[idx] as f32);
    }
    Ok(())
}

fn runtime_writeback_prelight_samples(
    mesh: &RenderMesh,
    mode: BakeLightMode,
) -> Vec<PrelightClipboardSample> {
    let mut samples = Vec::new();
    for part in &mesh.parts {
        for vertices in part.cpu_vertices.chunks_exact(3) {
            let triangle = Some([vertices[0].pos, vertices[1].pos, vertices[2].pos]);
            samples.extend(vertices.iter().map(|vertex| PrelightClipboardSample {
                geometry: PrelightSampleGeometry {
                    pos: vertex.pos,
                    normal: Some(vertex.normal),
                    uv: Some(vertex.uv),
                    triangle,
                    material: Some(part.material_index),
                },
                color: serialized_prelight_from_runtime(part, vertex_bake_color(vertex, mode)),
            }));
        }
    }
    samples
}

fn raw_writeback_prelight_targets(raw: &RawMesh) -> Vec<(usize, PrelightSampleGeometry)> {
    let mut targets = Vec::with_capacity(raw.triangles.len() * 3);
    for tri in &raw.triangles {
        let triangle = [tri.a, tri.b, tri.c].map(|idx| raw.vertices.get(idx as usize).copied());
        let triangle = match triangle {
            [Some(a), Some(b), Some(c)] => Some([a, b, c]),
            _ => None,
        };
        for source_idx in [tri.a as usize, tri.b as usize, tri.c as usize] {
            let Some(pos) = raw.vertices.get(source_idx).copied() else {
                continue;
            };
            targets.push((
                source_idx,
                PrelightSampleGeometry {
                    pos,
                    normal: (raw.normals.len() == raw.vertices.len())
                        .then(|| raw.normals[source_idx]),
                    uv: (raw.uvs.len() == raw.vertices.len()).then(|| raw.uvs[source_idx]),
                    triangle,
                    material: Some(tri.material as usize),
                },
            ));
        }
    }
    targets
}

/// Recovers prelight when the live mesh and source DFF have compatible
/// geometry but different face counts/order. Only position-coincident matches
/// are accepted automatically; unmatched source vertices retain their current
/// colors.
fn apply_runtime_vertex_colors_to_raw_by_geometry(
    mesh: &RenderMesh,
    raw: &mut RawMesh,
) -> Result<(usize, usize), String> {
    if raw.vertices.is_empty() || raw.triangles.is_empty() {
        return Err("Source DFF has no readable geometry".to_string());
    }
    let targets = raw_writeback_prelight_targets(raw);
    let day_samples = runtime_writeback_prelight_samples(mesh, BakeLightMode::Day);
    let night_samples = runtime_writeback_prelight_samples(mesh, BakeLightMode::Night);
    if targets.is_empty() || day_samples.is_empty() || day_samples.len() != night_samples.len() {
        return Err("live/source geometry has no transferable prelight samples".to_string());
    }
    let geometries = targets
        .iter()
        .map(|(_, geometry)| geometry.clone())
        .collect::<Vec<_>>();
    let (matches, _) = match_prelight_samples(&geometries, &day_samples);
    let mut day_sum = vec![Vec3::ZERO; raw.vertices.len()];
    let mut night_sum = vec![Vec3::ZERO; raw.vertices.len()];
    let mut counts = vec![0usize; raw.vertices.len()];
    let mut matched = 0usize;
    for ((source_idx, _), (sample_idx, dist2)) in targets.iter().zip(matches) {
        // local_vertex_distance2 uses squared units. This permits at most a
        // 0.001-unit drift while rejecting a genuinely different model.
        if dist2 > 0.000001 {
            continue;
        }
        let Some(day) = day_samples.get(sample_idx) else {
            continue;
        };
        let Some(night) = night_samples.get(sample_idx) else {
            continue;
        };
        day_sum[*source_idx] += vec3(day.color.x, day.color.y, day.color.z);
        night_sum[*source_idx] += vec3(night.color.x, night.color.y, night.color.z);
        counts[*source_idx] += 1;
        matched += 1;
    }
    let total = targets.len();
    if matched == 0 || matched * 5 < total * 4 {
        return Err(format!(
            "geometry overlap is too low for safe prelight repair ({matched}/{total} corners matched)"
        ));
    }
    raw.prelit_colors
        .resize(raw.vertices.len(), neutral_vertex_color());
    raw.night_prelit_colors
        .resize(raw.vertices.len(), neutral_vertex_color());
    for idx in 0..raw.vertices.len() {
        if counts[idx] == 0 {
            continue;
        }
        raw.prelit_colors[idx] = baked_vec3_to_v3(day_sum[idx] / counts[idx] as f32);
        raw.night_prelit_colors[idx] = baked_vec3_to_v3(night_sum[idx] / counts[idx] as f32);
    }
    Ok((matched, total))
}

/// Applies every live TXD/runtime variant of one DFF to a parsed source mesh.
/// Multiple variants are averaged exactly as they are during normal vertex
/// lighting writeback, so other asset-rewrite jobs can preserve unsaved
/// prelight instead of reloading stale colors from disk.
pub(crate) fn apply_runtime_vertex_meshes_to_raw(
    meshes: &[&RenderMesh],
    raw: &mut RawMesh,
) -> Result<(), String> {
    let Some((first, rest)) = meshes.split_first() else {
        return Ok(());
    };
    if rest.is_empty() {
        return apply_runtime_vertex_colors_to_raw(first, raw);
    }

    let source_raw = raw.clone();
    let mut day_sum = vec![Vec3::ZERO; raw.vertices.len()];
    let mut night_sum = vec![Vec3::ZERO; raw.vertices.len()];
    for mesh in meshes {
        let mut variant_raw = source_raw.clone();
        apply_runtime_vertex_colors_to_raw(mesh, &mut variant_raw)?;
        for (sum, color) in day_sum.iter_mut().zip(&variant_raw.prelit_colors) {
            *sum += vec3(color.x, color.y, color.z);
        }
        for (sum, color) in night_sum.iter_mut().zip(&variant_raw.night_prelit_colors) {
            *sum += vec3(color.x, color.y, color.z);
        }
    }
    let divisor = meshes.len() as f32;
    raw.prelit_colors = day_sum
        .into_iter()
        .map(|sum| baked_vec3_to_v3(sum / divisor))
        .collect();
    raw.night_prelit_colors = night_sum
        .into_iter()
        .map(|sum| baked_vec3_to_v3(sum / divisor))
        .collect();
    Ok(())
}

fn apply_runtime_vertex_meshes_to_raw_with_repair(
    meshes: &[&RenderMesh],
    raw: &mut RawMesh,
) -> Result<Option<String>, String> {
    match apply_runtime_vertex_meshes_to_raw(meshes, raw) {
        Ok(()) => return Ok(None),
        Err(exact_error) => {
            let source_raw = raw.clone();
            let mut day_sum = vec![Vec3::ZERO; raw.vertices.len()];
            let mut night_sum = vec![Vec3::ZERO; raw.vertices.len()];
            let mut matched = 0usize;
            let mut total = 0usize;
            for mesh in meshes {
                let mut variant_raw = source_raw.clone();
                let (variant_matched, variant_total) =
                    apply_runtime_vertex_colors_to_raw_by_geometry(mesh, &mut variant_raw)
                        .map_err(|repair_error| {
                            format!(
                                "{exact_error}; geometry-aware prelight repair failed: {repair_error}"
                            )
                        })?;
                for (sum, color) in day_sum.iter_mut().zip(&variant_raw.prelit_colors) {
                    *sum += vec3(color.x, color.y, color.z);
                }
                for (sum, color) in night_sum.iter_mut().zip(&variant_raw.night_prelit_colors) {
                    *sum += vec3(color.x, color.y, color.z);
                }
                matched += variant_matched;
                total += variant_total;
            }
            let divisor = meshes.len() as f32;
            raw.prelit_colors = day_sum
                .into_iter()
                .map(|sum| baked_vec3_to_v3(sum / divisor))
                .collect();
            raw.night_prelit_colors = night_sum
                .into_iter()
                .map(|sum| baked_vec3_to_v3(sum / divisor))
                .collect();
            Ok(Some(format!(
                "repaired vertex-light topology by geometry matching ({matched}/{total} corners)"
            )))
        }
    }
}

fn vertex_lighting_dff_name(mesh_key: &str) -> Option<String> {
    let dff_key = mesh_key
        .split('|')
        .next()
        .filter(|value| !value.is_empty())?;
    Some(normalize_legacy_light_mapper_asset_name(&with_ext(
        dff_key, ".dff",
    )))
}

fn find_vertex_lighting_sources(
    source_root: &Path,
    fallback_root: &Path,
    targets: &HashSet<String>,
) -> HashMap<String, ImgEntry> {
    // Prefer the destination snapshot and staged replacement archives over
    // the saved base IMG. Those archives may contain an optimized or edited
    // DFF whose topology is already reflected by the live RenderMesh.
    let mut paths = vec![
        fallback_root.join("imgs").join(REPLACEMENT_IMG),
        wip_root_path(source_root)
            .join("imgs")
            .join(REPLACEMENT_IMG),
        source_root.join("imgs").join(REPLACEMENT_IMG),
    ];
    paths.extend(collect_resource_img_files(source_root));
    paths.extend(gta_sa_img_files(&load_gta_sa_dir_preference()));
    paths.extend(collect_resource_img_files(fallback_root));
    let mut seen_paths = HashSet::new();
    let mut found = HashMap::new();
    for path in paths {
        if !seen_paths.insert(path.clone()) {
            continue;
        }
        for entry in parse_img(&path) {
            let key = lower(&entry.name);
            if targets.contains(&key) {
                found.entry(key).or_insert(entry);
            }
        }
        if found.len() == targets.len() {
            break;
        }
    }
    found
}

/// Serializes all dirty runtime meshes as one replacement-archive update.
/// A DFF may have several runtime mesh keys when it is used with different
/// TXDs. Those keys still target one on-disk DFF, so combine their colors
/// deterministically instead of allowing HashSet iteration order to decide
/// which variant overwrites all the others.
pub(crate) fn write_vertex_lighting_meshes<'a, I>(
    source_root: &Path,
    root: &Path,
    meshes: I,
) -> Result<usize, String>
where
    I: IntoIterator<Item = (&'a str, &'a RenderMesh, DffWriteOptions)>,
{
    let mut groups = BTreeMap::<String, Vec<(&RenderMesh, DffWriteOptions)>>::new();
    for (mesh_key, mesh, options) in meshes {
        if is_simulation_mesh_key(mesh_key) {
            continue;
        }
        let Some(dff_name) = vertex_lighting_dff_name(mesh_key) else {
            continue;
        };
        groups.entry(dff_name).or_default().push((mesh, options));
    }
    if groups.is_empty() {
        return Ok(0);
    }

    write_vertex_lighting_groups(source_root, root, groups, false).map(|outcome| outcome.written)
}

pub(crate) struct VertexLightingWriteOutcome {
    pub(crate) written: usize,
    pub(crate) warnings: Vec<String>,
}

fn write_vertex_lighting_groups(
    source_root: &Path,
    root: &Path,
    groups: BTreeMap<String, Vec<(&RenderMesh, DffWriteOptions)>>,
    tolerate_asset_failures: bool,
) -> Result<VertexLightingWriteOutcome, String> {
    let targets = groups
        .keys()
        .map(|name| lower(name))
        .collect::<HashSet<_>>();
    let sources = find_vertex_lighting_sources(source_root, root, &targets);
    let mut replacements = Vec::with_capacity(groups.len());
    let mut warnings = Vec::new();
    for (dff_name, variants) in groups {
        let result = (|| {
            let source_entry = sources.get(&lower(&dff_name)).ok_or_else(|| {
                format!("Could not find source {dff_name} for vertex-light writeback")
            })?;
            let source_bytes = read_img_entry(source_entry);
            let chunk_len = dff_chunk_len(&source_bytes).min(source_bytes.len());
            let mut raw = parse_dff_mesh(&source_bytes[..chunk_len]);
            let runtime_meshes = variants.iter().map(|(mesh, _)| *mesh).collect::<Vec<_>>();
            let repair_note =
                apply_runtime_vertex_meshes_to_raw_with_repair(&runtime_meshes, &mut raw)
                    .map_err(|err| format!("{dff_name}: {err}"))?;
            let frame_name = Path::new(&dff_name)
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("vertex_lighting");
            let bytes = write_normalized_dff_with_options(&raw, frame_name, variants[0].1)
                .map_err(|err| format!("{dff_name}: {err}"))?;
            Ok::<_, String>((bytes, repair_note))
        })();
        match result {
            Ok((bytes, repair_note)) => {
                if let Some(note) = repair_note {
                    warnings.push(format!("{dff_name}: {note}"));
                }
                replacements.push((dff_name, bytes));
            }
            Err(error) if tolerate_asset_failures => warnings.push(format!(
                "Skipped vertex-light writeback for {dff_name}; the rest of the save continued: {error}"
            )),
            Err(error) => return Err(error),
        }
    }
    let written = replacements.len();
    upsert_replacement_assets(root, &replacements)?;
    Ok(VertexLightingWriteOutcome { written, warnings })
}

pub(crate) fn write_vertex_lighting_meshes_resilient<'a, I>(
    source_root: &Path,
    root: &Path,
    meshes: I,
) -> Result<VertexLightingWriteOutcome, String>
where
    I: IntoIterator<Item = (&'a str, &'a RenderMesh, DffWriteOptions)>,
{
    let mut groups = BTreeMap::<String, Vec<(&RenderMesh, DffWriteOptions)>>::new();
    for (mesh_key, mesh, options) in meshes {
        if is_simulation_mesh_key(mesh_key) {
            continue;
        }
        let Some(dff_name) = vertex_lighting_dff_name(mesh_key) else {
            continue;
        };
        groups.entry(dff_name).or_default().push((mesh, options));
    }
    if groups.is_empty() {
        return Ok(VertexLightingWriteOutcome {
            written: 0,
            warnings: Vec::new(),
        });
    }
    write_vertex_lighting_groups(source_root, root, groups, true)
}

pub(crate) fn queue_vertex_lighting_meshes<'a, I>(app: &mut AppState, mesh_keys: I) -> usize
where
    I: IntoIterator<Item = &'a String>,
{
    let before = app.pending_vertex_light_meshes.len();
    app.pending_vertex_light_meshes.extend(
        mesh_keys
            .into_iter()
            .filter(|key| !is_simulation_mesh_key(key))
            .cloned(),
    );
    app.pending_vertex_light_meshes.len().saturating_sub(before)
}

pub(crate) fn is_simulation_mesh_key(mesh_key: &str) -> bool {
    let dff_key = mesh_key.split('|').next().unwrap_or(mesh_key);
    dff_key.eq_ignore_ascii_case(SIM_PLAYER_DFF) || dff_key.eq_ignore_ascii_case(SIM_VEHICLE_DFF)
}

pub(crate) fn vertex_lighting_targets_deleted_asset(
    mesh_key: &str,
    pending_asset_deletes: &HashSet<String>,
    editing_deleted_entries: &BTreeSet<String>,
) -> bool {
    let dff_name = mesh_key.split('|').next().unwrap_or(mesh_key);
    let dff_key = asset_key(dff_name, ".dff");
    pending_asset_deletes.contains(&dff_key) || editing_deleted_entries.contains(&dff_key)
}

pub(crate) fn flush_pending_vertex_lighting(app: &AppState, root: &Path) -> Result<usize, String> {
    write_vertex_lighting_meshes(
        &app.root,
        root,
        app.pending_vertex_light_meshes
            .iter()
            .filter(|mesh_key| {
                !vertex_lighting_targets_deleted_asset(
                    mesh_key,
                    &app.pending_asset_deletes,
                    &app.editing.deleted_entries,
                )
            })
            .filter_map(|mesh_key| {
                let mesh = app.meshes.get(mesh_key)?;
                let dff_name = mesh_key.split('|').next().unwrap_or(mesh_key);
                Some((
                    mesh_key.as_str(),
                    mesh,
                    dff_write_options_for_asset(app, dff_name),
                ))
            }),
    )
}

#[cfg(test)]
mod simulation_save_tests {
    use super::*;

    #[test]
    fn simulation_meshes_are_transient_save_assets() {
        assert!(is_simulation_mesh_key("__sim_player.dff"));
        assert!(is_simulation_mesh_key("__SIM_VEHICLE.DFF|variant"));
        assert!(!is_simulation_mesh_key("player_1.dff"));
        assert!(!is_simulation_mesh_key("vehicle.dff"));
    }

    #[test]
    fn deleted_dffs_are_not_vertex_lighting_writeback_targets() {
        let pending_asset_deletes = HashSet::from(["unused.dff".to_string()]);
        let editing_deleted_entries = BTreeSet::from(["editing_only.dff".to_string()]);

        assert!(vertex_lighting_targets_deleted_asset(
            "UNUSED.DFF|shared.txd",
            &pending_asset_deletes,
            &editing_deleted_entries,
        ));
        assert!(vertex_lighting_targets_deleted_asset(
            "editing_only.dff",
            &pending_asset_deletes,
            &editing_deleted_entries,
        ));
        assert!(!vertex_lighting_targets_deleted_asset(
            "kept.dff|shared.txd",
            &pending_asset_deletes,
            &editing_deleted_entries,
        ));
    }
}

pub(crate) fn flush_vertex_paint_preview(app: &mut AppState, force: bool) {
    if app.vertex_paint_dirty_meshes.is_empty() {
        return;
    }
    let now = get_time();
    if !force && now < app.vertex_paint_next_rebuild_at {
        return;
    }
    let dirty: Vec<String> = app.vertex_paint_dirty_meshes.drain().collect();
    for mesh_key in &dirty {
        if let Some(mesh) = app.meshes.get_mut(mesh_key) {
            for part in &mut mesh.parts {
                rebuild_render_part_list_with_lift(part, V3::default());
            }
        }
    }
    rebuild_render_cells(app);
    app.vertex_paint_next_rebuild_at = now + 0.045;
}

fn pick_vertex_light_hit(app: &AppState, viewport: Rect, mouse: Vec2) -> Option<VertexLightHit> {
    let (origin, dir) = viewport_ray(app, viewport, mouse)?;
    let mut best: Option<VertexLightHit> = None;
    let mut best_t = f32::MAX;
    for (idx, placement) in app.placements.iter().enumerate() {
        if app
            .element_states
            .get(idx)
            .is_some_and(|state| state.deleted)
        {
            continue;
        }
        let mesh_key = placement_mesh_key(placement, &app.definitions);
        let Some(mesh) = app.meshes.get(&mesh_key) else {
            continue;
        };
        let model = placement_matrix(placement);
        let inv = model.inverse();
        let local_origin = inv.transform_point3(origin);
        let local_dir = inv.transform_vector3(dir).normalize_or_zero();
        if local_dir.length_squared() < 0.0001
            || ray_aabb(local_origin, local_dir, mesh.bounds.min, mesh.bounds.max).is_none()
        {
            continue;
        }
        for (part_idx, part) in mesh.parts.iter().enumerate() {
            for (tri_idx, tri) in part.cpu_vertices.chunks_exact(3).enumerate() {
                let a = to_mq(tri[0].pos);
                let b = to_mq(tri[1].pos);
                let c = to_mq(tri[2].pos);
                let Some((local_t, weights)) =
                    ray_triangle_barycentric(local_origin, local_dir, a, b, c)
                else {
                    continue;
                };
                let local_hit = local_origin + local_dir * local_t;
                let world_hit = model.transform_point3(local_hit);
                let local_normal = (b - a).cross(c - a).normalize_or_zero();
                let mut world_normal = model.transform_vector3(local_normal).normalize_or_zero();
                if world_normal.length_squared() < 0.0001 {
                    world_normal = Vec3::Z;
                }
                let world_t = (world_hit - origin).length();
                if world_t < best_t {
                    best_t = world_t;
                    best = Some(VertexLightHit {
                        mesh_key: mesh_key.clone(),
                        part_idx,
                        tri_start: tri_idx * 3,
                        world_pos: world_hit,
                        world_normal,
                        weights,
                    });
                }
            }
        }
    }
    best
}

pub(crate) fn sample_vertex_lighting(app: &mut AppState, viewport: Rect, mouse: Vec2) -> bool {
    let Some(hit) = pick_vertex_light_hit(app, viewport, mouse) else {
        app.status_message = "No vertex lighting under cursor to sample".to_string();
        return false;
    };
    let Some(part) = app
        .meshes
        .get(&hit.mesh_key)
        .and_then(|mesh| mesh.parts.get(hit.part_idx))
    else {
        return false;
    };
    let mut sampled = V3::default();
    for slot in 0..3 {
        if let Some(vertex) = part.cpu_vertices.get(hit.tri_start + slot) {
            let color = vertex_bake_color(vertex, app.bake_settings.light_mode);
            sampled.x += color.x * hit.weights[slot];
            sampled.y += color.y * hit.weights[slot];
            sampled.z += color.z * hit.weights[slot];
        }
    }
    app.vertex_paint.color = v3_clamp01(sampled);
    save_vertex_paint_settings_preference(app.vertex_paint);
    app.status_message = format!(
        "Sampled {} vertex lighting RGB {:.2}/{:.2}/{:.2}",
        bake_light_mode_label(app.bake_settings.light_mode),
        app.vertex_paint.color.x,
        app.vertex_paint.color.y,
        app.vertex_paint.color.z
    );
    true
}

pub(crate) fn paint_vertex_lighting(app: &mut AppState, viewport: Rect, mouse: Vec2) -> bool {
    let Some(hit) = pick_vertex_light_hit(app, viewport, mouse) else {
        return false;
    };
    let radius = app.vertex_paint.radius.max(1.0);
    let strength = app.vertex_paint.strength.clamp(0.0, 1.0);
    let target = app.vertex_paint.color;
    let mode = app.bake_settings.light_mode;
    const OVERLAP_SURFACE_TOLERANCE: f32 = 0.2;
    let depth_tolerance = OVERLAP_SURFACE_TOLERANCE.max(radius * 0.01);
    let mut painted = 0usize;
    let mut paint_updates = Vec::<(String, usize, usize, V3)>::new();
    let mut touched_meshes = HashSet::<String>::new();
    for (placement_idx, placement) in app.placements.iter().enumerate() {
        if app
            .element_states
            .get(placement_idx)
            .is_some_and(|state| state.deleted)
        {
            continue;
        }
        let mesh_key = placement_mesh_key(placement, &app.definitions);
        let Some(mesh) = app.meshes.get(&mesh_key) else {
            continue;
        };
        let model = placement_matrix(placement).to_cols_array();
        let bounds = transformed_bounds(mesh.bounds, &model);
        if point_aabb_distance(hit.world_pos, bounds.min, bounds.max) > radius + depth_tolerance {
            continue;
        }
        for (part_idx, part) in mesh.parts.iter().enumerate() {
            for (vertex_idx, vertex) in part.cpu_vertices.iter().enumerate() {
                let pos = transform_point_gl(&model, vertex.pos);
                let (dist, depth) = projected_brush_distance(pos, hit.world_pos, hit.world_normal);
                if dist > radius || depth > depth_tolerance {
                    continue;
                }
                let falloff = 1.0 - dist / radius;
                let color = mix_v3(vertex_bake_color(vertex, mode), target, strength * falloff);
                paint_updates.push((mesh_key.clone(), part_idx, vertex_idx, color));
            }
        }
    }
    for (mesh_key, part_idx, vertex_idx, color) in paint_updates {
        if let Some(vertex) = app
            .meshes
            .get_mut(&mesh_key)
            .and_then(|mesh| mesh.parts.get_mut(part_idx))
            .and_then(|part| part.cpu_vertices.get_mut(vertex_idx))
        {
            set_vertex_bake_color(vertex, mode, color);
            apply_vertex_bake_display(vertex, mode);
            touched_meshes.insert(mesh_key);
            painted += 1;
        }
    }
    if painted == 0 {
        return false;
    }
    app.vertex_paint_dirty_meshes
        .extend(touched_meshes.iter().cloned());
    flush_vertex_paint_preview(app, false);
    queue_vertex_lighting_meshes(app, touched_meshes.iter());
    app.status_message = format!(
        "Painted {painted} shared mesh vertex/vertices for {} lighting; queued DFF for Save/Save WIP",
        bake_light_mode_label(mode)
    );
    true
}

pub(crate) fn update_vertex_lighting_tool(app: &mut AppState, viewport: Rect, mouse: Vec2) -> bool {
    if !app.vertex_paint.enabled
        || app.active_tab != AppTab::Bake
        || app.bake_job.is_some()
        || !viewport.contains(mouse)
    {
        flush_vertex_paint_preview(app, is_mouse_button_released(MouseButton::Left));
        return false;
    }
    if is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt) {
        flush_vertex_paint_preview(app, is_mouse_button_released(MouseButton::Left));
        return false;
    }
    if is_mouse_button_released(MouseButton::Left) {
        let had_dirty = !app.vertex_paint_dirty_meshes.is_empty();
        flush_vertex_paint_preview(app, true);
        return had_dirty;
    }
    match app.vertex_paint.tool {
        VertexLightTool::Paint => {
            if is_mouse_button_down(MouseButton::Left) {
                return paint_vertex_lighting(app, viewport, mouse);
            }
        }
        VertexLightTool::Sample => {
            if is_mouse_button_pressed(MouseButton::Left) {
                return sample_vertex_lighting(app, viewport, mouse);
            }
        }
    }
    false
}

pub(crate) fn draw_vertex_paint_marker(app: &AppState, viewport: Rect) {
    if !app.vertex_paint.enabled || app.active_tab != AppTab::Bake || app.bake_job.is_some() {
        return;
    }
    if is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt) {
        return;
    }
    let mouse: Vec2 = mouse_position().into();
    let Some(hit) = pick_vertex_light_hit(app, viewport, mouse) else {
        return;
    };
    let radius = app.vertex_paint.radius.max(1.0);
    let normal = hit.world_normal.normalize_or_zero();
    let (_, right) = camera_vectors(&app.camera);
    let mut tangent = right - normal * right.dot(normal);
    if tangent.length_squared() < 0.0001 {
        tangent = normal.cross(Vec3::X);
    }
    if tangent.length_squared() < 0.0001 {
        tangent = normal.cross(Vec3::Y);
    }
    let tangent = tangent.normalize_or_zero();
    let bitangent = normal.cross(tangent).normalize_or_zero();
    if tangent.length_squared() < 0.0001 || bitangent.length_squared() < 0.0001 {
        return;
    }
    let center = hit.world_pos + normal * 0.6;
    let color = app.vertex_paint.color;
    const OVERLAP_SURFACE_TOLERANCE: f32 = 0.2;
    let depth_tolerance = OVERLAP_SURFACE_TOLERANCE.max(radius * 0.01);
    let outside_band = (radius * 0.20).clamp(1.0, 12.0);
    let mut inside_vertices = Vec::<Vec3>::new();
    let mut outside_vertices = Vec::<Vec3>::new();
    for (placement_idx, placement) in app.placements.iter().enumerate() {
        if app
            .element_states
            .get(placement_idx)
            .is_some_and(|state| state.deleted)
        {
            continue;
        }
        let mesh_key = placement_mesh_key(placement, &app.definitions);
        let Some(mesh) = app.meshes.get(&mesh_key) else {
            continue;
        };
        let model = placement_matrix(placement).to_cols_array();
        let bounds = transformed_bounds(mesh.bounds, &model);
        if point_aabb_distance(hit.world_pos, bounds.min, bounds.max)
            > radius + outside_band + depth_tolerance
        {
            continue;
        }
        for part in &mesh.parts {
            for vertex in &part.cpu_vertices {
                let pos = transform_point_gl(&model, vertex.pos);
                let (dist, depth) = projected_brush_distance(pos, hit.world_pos, hit.world_normal);
                if depth > depth_tolerance || dist > radius + outside_band {
                    continue;
                }
                let draw_pos = pos + normal * 0.9;
                if dist <= radius {
                    inside_vertices.push(draw_pos);
                } else {
                    outside_vertices.push(draw_pos);
                }
            }
        }
    }
    unsafe {
        gl::PushAttrib(
            gl::ENABLE_BIT | gl::LINE_BIT | gl::POINT_BIT | gl::CURRENT_BIT | gl::DEPTH_BUFFER_BIT,
        );
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::ALPHA_TEST);
        gl::Disable(gl::CULL_FACE);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::Disable(gl::DEPTH_TEST);
        gl::DepthMask(gl::FALSE);
        gl::LineWidth(2.0);
        gl::Color4f(
            color.x.max(0.18),
            color.y.max(0.18),
            color.z.max(0.18),
            0.72,
        );
        gl::Begin(gl::LINE_LOOP);
        for i in 0..48 {
            let angle = i as f32 / 48.0 * std::f32::consts::TAU;
            let p = center + tangent * angle.cos() * radius + bitangent * angle.sin() * radius;
            gl::Vertex3f(p.x, p.y, p.z);
        }
        gl::End();
        gl::LineWidth(1.5);
        gl::Begin(gl::LINES);
        let cross = radius.min(96.0).max(12.0);
        for dir in [tangent, bitangent] {
            let a = center - dir * cross;
            let b = center + dir * cross;
            gl::Vertex3f(a.x, a.y, a.z);
            gl::Vertex3f(b.x, b.y, b.z);
        }
        gl::End();
        gl::PointSize(7.0);
        gl::Begin(gl::POINTS);
        gl::Vertex3f(center.x, center.y, center.z);
        gl::End();
        gl::PointSize(5.0);
        gl::Color4f(
            color.x.max(0.25),
            color.y.max(0.25),
            color.z.max(0.25),
            0.68,
        );
        gl::Begin(gl::POINTS);
        for vertex in &inside_vertices {
            gl::Vertex3f(vertex.x, vertex.y, vertex.z);
        }
        gl::End();
        gl::PointSize(3.0);
        gl::Color4f(0.90, 0.95, 1.0, 0.32);
        gl::Begin(gl::POINTS);
        for vertex in &outside_vertices {
            gl::Vertex3f(vertex.x, vertex.y, vertex.z);
        }
        gl::End();
        gl::PopAttrib();
    }
}

pub(crate) fn blend_selected_vertex_lighting(app: &mut AppState) {
    let selected = selected_live_indices(app);
    if selected.is_empty() {
        app.status_message =
            "Select one or more elements before blending vertex lighting".to_string();
        return;
    }
    let selected_set: HashSet<usize> = selected.iter().copied().collect();
    let mode = app.bake_settings.light_mode;
    let radius = app.vertex_paint.radius.max(1.0);
    let strength = app.vertex_paint.strength.clamp(0.0, 1.0);
    let mut sources = Vec::<(Vec3, V3)>::new();
    for idx in 0..app.placements.len() {
        if selected_set.contains(&idx)
            || app
                .element_states
                .get(idx)
                .is_some_and(|state| state.deleted)
        {
            continue;
        }
        sources.extend(
            placement_vertex_samples(app, idx, mode)
                .into_iter()
                .map(|(_, _, _, pos, color)| (pos, color)),
        );
    }
    if sources.is_empty() {
        app.status_message = "No neighboring vertex lighting found to blend from".to_string();
        return;
    }
    let cell_size = radius.max(64.0);
    let source_grid = build_source_vertex_grid(&sources, cell_size);
    let mut updates = Vec::<(String, usize, usize, V3)>::new();
    let mut closest_total = 0.0f32;
    let mut closest_count = 0usize;
    for idx in selected {
        let mut targets = placement_boundary_vertex_samples(app, idx, mode);
        if targets.is_empty() {
            targets = placement_vertex_samples(app, idx, mode);
        }
        for (mesh_key, part_idx, vertex_idx, pos, current_color) in targets {
            if let Some((dist, source_color)) =
                closest_source_vertex(&sources, &source_grid, pos, cell_size, radius)
            {
                closest_total += dist;
                closest_count += 1;
                let color = mix_v3(current_color, source_color, strength);
                updates.push((mesh_key, part_idx, vertex_idx, color));
            }
        }
    }
    let mut applied = 0usize;
    let changed_mesh_keys: Vec<String> = updates
        .iter()
        .map(|(mesh_key, _, _, _)| mesh_key.clone())
        .collect();
    for (mesh_key, part_idx, vertex_idx, color) in updates {
        if let Some(vertex) = app
            .meshes
            .get_mut(&mesh_key)
            .and_then(|mesh| mesh.parts.get_mut(part_idx))
            .and_then(|part| part.cpu_vertices.get_mut(vertex_idx))
        {
            set_vertex_bake_color(vertex, mode, color);
            apply_vertex_bake_display(vertex, mode);
            applied += 1;
        }
    }
    if applied == 0 {
        app.status_message = "No selected vertices were close enough to blend".to_string();
        return;
    }
    rebuild_mesh_part_lists(&mut app.meshes);
    rebuild_render_cells(app);
    let queued = queue_vertex_lighting_meshes(app, changed_mesh_keys.iter());
    let avg_distance = if closest_count > 0 {
        closest_total / closest_count as f32
    } else {
        0.0
    };
    app.status_message = format!(
        "Blended {applied} selected edge/corner vertex/vertices from closest {} lighting verts (avg {:.1}); queued {queued} DFF(s) for Save/Save WIP",
        bake_light_mode_label(mode),
        avg_distance
    );
}

fn local_vertex_distance2(a: V3, b: V3) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    dx * dx + dy * dy + dz * dz
}

fn closest_prelight_sample(samples: &[PrelightClipboardSample], pos: V3) -> Option<(usize, f32)> {
    let mut best_dist2 = f32::MAX;
    let mut best_idx = None;
    for (idx, sample) in samples.iter().enumerate() {
        let dist2 = local_vertex_distance2(sample.geometry.pos, pos);
        if dist2 < best_dist2 {
            best_dist2 = dist2;
            best_idx = Some(idx);
            if dist2 <= 0.000001 {
                break;
            }
        }
    }
    best_idx.map(|idx| (idx, best_dist2))
}

fn prelight_positions_match(a: V3, b: V3) -> bool {
    local_vertex_distance2(a, b) <= 0.000001
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct PrelightVec3Key(i64, i64, i64);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct PrelightVec2Key(i64, i64);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct PrelightTriangleKey([PrelightVec3Key; 3]);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct PrelightFullKey {
    pos: PrelightVec3Key,
    normal: Option<PrelightVec3Key>,
    uv: Option<PrelightVec2Key>,
    triangle: Option<PrelightTriangleKey>,
    material: Option<usize>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct PrelightTriangleVertexKey {
    pos: PrelightVec3Key,
    triangle: PrelightTriangleKey,
    material: Option<usize>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct PrelightVertexKey {
    pos: PrelightVec3Key,
    normal: Option<PrelightVec3Key>,
    uv: Option<PrelightVec2Key>,
}

fn prelight_vec3_key(value: V3) -> PrelightVec3Key {
    const SCALE: f32 = 10_000.0;
    PrelightVec3Key(
        (value.x * SCALE).round() as i64,
        (value.y * SCALE).round() as i64,
        (value.z * SCALE).round() as i64,
    )
}

fn prelight_vec2_key(value: V2) -> PrelightVec2Key {
    const SCALE: f32 = 10_000.0;
    PrelightVec2Key(
        (value.u * SCALE).round() as i64,
        (value.v * SCALE).round() as i64,
    )
}

fn prelight_triangle_key(triangle: [V3; 3]) -> PrelightTriangleKey {
    let mut points = triangle.map(prelight_vec3_key);
    points.sort_unstable();
    PrelightTriangleKey(points)
}

fn prelight_full_key(geometry: &PrelightSampleGeometry) -> PrelightFullKey {
    PrelightFullKey {
        pos: prelight_vec3_key(geometry.pos),
        normal: geometry.normal.map(prelight_vec3_key),
        uv: geometry.uv.map(prelight_vec2_key),
        triangle: geometry.triangle.map(prelight_triangle_key),
        material: geometry.material,
    }
}

fn prelight_triangle_vertex_key(
    geometry: &PrelightSampleGeometry,
    include_material: bool,
) -> Option<PrelightTriangleVertexKey> {
    Some(PrelightTriangleVertexKey {
        pos: prelight_vec3_key(geometry.pos),
        triangle: prelight_triangle_key(geometry.triangle?),
        material: if include_material {
            geometry.material
        } else {
            None
        },
    })
}

fn prelight_vertex_key(geometry: &PrelightSampleGeometry) -> PrelightVertexKey {
    PrelightVertexKey {
        pos: prelight_vec3_key(geometry.pos),
        normal: geometry.normal.map(prelight_vec3_key),
        uv: geometry.uv.map(prelight_vec2_key),
    }
}

fn pop_unused_prelight_sample<K: std::hash::Hash + Eq>(
    candidates: &mut HashMap<K, Vec<usize>>,
    key: &K,
    used: &mut [bool],
) -> Option<usize> {
    let indices = candidates.get_mut(key)?;
    while let Some(idx) = indices.pop() {
        if !used[idx] {
            used[idx] = true;
            return Some(idx);
        }
    }
    None
}

/// Matches an expanded destination mesh to copied/imported prelight samples.
///
/// A position-only nearest-neighbor transfer is not sufficient for DFFs:
/// several face corners can occupy the same position while intentionally
/// carrying different colors. Prefer a one-to-one match using the containing
/// triangle, normal, UV, and material, then progressively relax the signature
/// before falling back to the legacy nearest-position behavior.
fn match_prelight_samples(
    targets: &[PrelightSampleGeometry],
    samples: &[PrelightClipboardSample],
) -> (Vec<(usize, f32)>, bool) {
    if targets.len() == samples.len()
        && targets
            .iter()
            .zip(samples.iter())
            .all(|(target, sample)| prelight_positions_match(target.pos, sample.geometry.pos))
    {
        return ((0..targets.len()).map(|idx| (idx, 0.0)).collect(), true);
    }

    let mut full = HashMap::<PrelightFullKey, Vec<usize>>::new();
    let mut triangle_material = HashMap::<PrelightTriangleVertexKey, Vec<usize>>::new();
    let mut triangle = HashMap::<PrelightTriangleVertexKey, Vec<usize>>::new();
    let mut vertex = HashMap::<PrelightVertexKey, Vec<usize>>::new();
    let mut position = HashMap::<PrelightVec3Key, Vec<usize>>::new();
    // Push in reverse so pop() preserves source order for genuinely duplicate
    // signatures while keeping removal O(1).
    for (idx, sample) in samples.iter().enumerate().rev() {
        let geometry = &sample.geometry;
        full.entry(prelight_full_key(geometry))
            .or_default()
            .push(idx);
        if let Some(key) = prelight_triangle_vertex_key(geometry, true) {
            triangle_material.entry(key).or_default().push(idx);
        }
        if let Some(key) = prelight_triangle_vertex_key(geometry, false) {
            triangle.entry(key).or_default().push(idx);
        }
        vertex
            .entry(prelight_vertex_key(geometry))
            .or_default()
            .push(idx);
        position
            .entry(prelight_vec3_key(geometry.pos))
            .or_default()
            .push(idx);
    }

    let mut used = vec![false; samples.len()];
    let mut matches = Vec::with_capacity(targets.len());
    for target in targets {
        let mut sample_idx =
            pop_unused_prelight_sample(&mut full, &prelight_full_key(target), &mut used);
        if sample_idx.is_none() {
            if let Some(key) = prelight_triangle_vertex_key(target, true) {
                sample_idx = pop_unused_prelight_sample(&mut triangle_material, &key, &mut used);
            }
        }
        if sample_idx.is_none() {
            if let Some(key) = prelight_triangle_vertex_key(target, false) {
                sample_idx = pop_unused_prelight_sample(&mut triangle, &key, &mut used);
            }
        }
        if sample_idx.is_none() {
            sample_idx =
                pop_unused_prelight_sample(&mut vertex, &prelight_vertex_key(target), &mut used);
        }
        if sample_idx.is_none() {
            sample_idx = pop_unused_prelight_sample(
                &mut position,
                &prelight_vec3_key(target.pos),
                &mut used,
            );
        }
        let (sample_idx, dist2) = if let Some(sample_idx) = sample_idx {
            (
                sample_idx,
                local_vertex_distance2(target.pos, samples[sample_idx].geometry.pos),
            )
        } else if let Some(nearest) = closest_prelight_sample(samples, target.pos) {
            nearest
        } else {
            continue;
        };
        matches.push((sample_idx, dist2));
    }
    (matches, false)
}

fn raw_mesh_prelight_samples(raw: &RawMesh, mode: BakeLightMode) -> Vec<PrelightClipboardSample> {
    let has_day = raw.prelit_colors.len() == raw.vertices.len();
    let has_night = raw.night_prelit_colors.len() == raw.vertices.len();
    if !has_day && !has_night {
        return Vec::new();
    }

    fn sample_at(
        raw: &RawMesh,
        mode: BakeLightMode,
        idx: usize,
        triangle: Option<[V3; 3]>,
        material: Option<usize>,
    ) -> Option<PrelightClipboardSample> {
        let pos = *raw.vertices.get(idx)?;
        let color = match mode {
            BakeLightMode::Day => raw
                .prelit_colors
                .get(idx)
                .copied()
                .or_else(|| raw.night_prelit_colors.get(idx).copied()),
            BakeLightMode::Night => raw
                .night_prelit_colors
                .get(idx)
                .copied()
                .or_else(|| raw.prelit_colors.get(idx).copied()),
            BakeLightMode::Both => raw
                .prelit_colors
                .get(idx)
                .copied()
                .or_else(|| raw.night_prelit_colors.get(idx).copied()),
        }?;
        Some(PrelightClipboardSample {
            geometry: PrelightSampleGeometry {
                pos,
                normal: (raw.normals.len() == raw.vertices.len()).then(|| raw.normals[idx]),
                uv: (raw.uvs.len() == raw.vertices.len()).then(|| raw.uvs[idx]),
                triangle,
                material,
            },
            color,
        })
    }

    let mut samples = Vec::new();
    if !raw.triangles.is_empty() {
        for tri in &raw.triangles {
            let triangle = [tri.a, tri.b, tri.c].map(|idx| raw.vertices.get(idx as usize).copied());
            let triangle = match triangle {
                [Some(a), Some(b), Some(c)] => Some([a, b, c]),
                _ => None,
            };
            for idx in [tri.a, tri.b, tri.c] {
                if let Some(sample) = sample_at(
                    raw,
                    mode,
                    idx as usize,
                    triangle,
                    Some(tri.material as usize),
                ) {
                    samples.push(sample);
                }
            }
        }
        if !samples.is_empty() {
            return samples;
        }
    }
    (0..raw.vertices.len())
        .filter_map(|idx| sample_at(raw, mode, idx, None, None))
        .collect()
}

fn collect_prelight_updates(
    app: &AppState,
    selected: Vec<usize>,
    samples: &[PrelightClipboardSample],
) -> (Vec<(String, usize, usize, V3, f32)>, bool) {
    let mut updates = Vec::new();
    let mut all_exact_order = true;
    for idx in selected {
        let Some(placement) = app.placements.get(idx) else {
            continue;
        };
        let mesh_key = placement_mesh_key(placement, &app.definitions);
        let Some(mesh) = app.meshes.get(&mesh_key) else {
            continue;
        };
        let mut targets = Vec::<(usize, usize, PrelightSampleGeometry)>::new();
        for (part_idx, part) in mesh.parts.iter().enumerate() {
            for (tri_idx, vertices) in part.cpu_vertices.chunks(3).enumerate() {
                let triangle = (vertices.len() == 3)
                    .then(|| [vertices[0].pos, vertices[1].pos, vertices[2].pos]);
                for (corner, vertex) in vertices.iter().enumerate() {
                    targets.push((
                        part_idx,
                        tri_idx * 3 + corner,
                        PrelightSampleGeometry {
                            pos: vertex.pos,
                            normal: Some(vertex.normal),
                            uv: Some(vertex.uv),
                            triangle,
                            material: Some(part.material_index),
                        },
                    ));
                }
            }
        }

        let geometries: Vec<_> = targets
            .iter()
            .map(|(_, _, geometry)| geometry.clone())
            .collect();
        let (matches, exact_order) = match_prelight_samples(&geometries, samples);
        all_exact_order &= exact_order;
        for ((part_idx, vertex_idx, _), (sample_idx, dist2)) in targets.into_iter().zip(matches) {
            let Some(part) = mesh.parts.get(part_idx) else {
                continue;
            };
            updates.push((
                mesh_key.clone(),
                part_idx,
                vertex_idx,
                runtime_prelight_from_serialized(part, samples[sample_idx].color),
                dist2,
            ));
        }
    }
    (updates, all_exact_order)
}

pub(crate) fn import_selected_prelight_from_dff_bytes(
    app: &mut AppState,
    bytes: &[u8],
    source_label: &str,
) {
    let selected = selected_live_indices(app);
    if selected.is_empty() {
        app.status_message = "Select one or more elements before importing prelight".to_string();
        return;
    }
    let raw = parse_dff_mesh(&bytes[..dff_chunk_len(bytes).min(bytes.len())]);
    if raw.vertices.is_empty() {
        app.status_message = format!("{} has no readable DFF geometry", source_label);
        return;
    }
    let mode = app.bake_settings.light_mode;
    let samples = raw_mesh_prelight_samples(&raw, mode);
    if samples.is_empty() {
        app.status_message = format!(
            "{} has no {} prelight data",
            source_label,
            bake_light_mode_label(mode)
        );
        return;
    }

    let (updates, exact_order) = collect_prelight_updates(app, selected, &samples);

    let changed_mesh_keys: Vec<String> = updates
        .iter()
        .map(|(mesh_key, _, _, _, _)| mesh_key.clone())
        .collect();
    let mut applied = 0usize;
    let mut exact = 0usize;
    let mut distance_total = 0.0f32;
    for (mesh_key, part_idx, vertex_idx, color, dist2) in updates {
        if let Some(vertex) = app
            .meshes
            .get_mut(&mesh_key)
            .and_then(|mesh| mesh.parts.get_mut(part_idx))
            .and_then(|part| part.cpu_vertices.get_mut(vertex_idx))
        {
            set_vertex_bake_color(vertex, mode, color);
            apply_vertex_bake_display(vertex, mode);
            if dist2 <= 0.000001 {
                exact += 1;
            }
            distance_total += dist2.sqrt();
            applied += 1;
        }
    }
    if applied == 0 {
        app.status_message = "No selected prelight vertices were imported".to_string();
        return;
    }
    rebuild_mesh_part_lists(&mut app.meshes);
    rebuild_render_cells(app);
    app.vertex_paint_dirty_meshes
        .extend(changed_mesh_keys.iter().cloned());
    let queued = queue_vertex_lighting_meshes(app, changed_mesh_keys.iter());
    let avg_distance = distance_total / applied as f32;
    let method = if exact_order {
        "exact ordered"
    } else {
        "geometry-aware"
    };
    app.status_message = format!(
        "Imported {applied} {} prelight vertex/vertices from {} using {method} transfer ({exact} exact, avg local distance {:.3}); queued {queued} DFF(s)",
        bake_light_mode_label(mode),
        source_label,
        avg_distance
    );
}

pub(crate) fn start_import_prelight_from_path(app: &mut AppState, path: PathBuf) {
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match ext.as_str() {
        "dff" => match fs::read(&path) {
            Ok(bytes) => {
                let label = path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("source DFF")
                    .to_string();
                import_selected_prelight_from_dff_bytes(app, &bytes, &label);
            }
            Err(err) => app.status_message = format!("Could not read DFF: {err}"),
        },
        "img" => {
            let entries: Vec<ImgEntry> = parse_img(&path)
                .into_iter()
                .filter(|entry| {
                    entry
                        .name
                        .rsplit_once('.')
                        .is_some_and(|(_, ext)| ext.eq_ignore_ascii_case("dff"))
                })
                .collect();
            if entries.is_empty() {
                app.status_message = format!(
                    "{} has no DFF entries",
                    path.file_name()
                        .and_then(|value| value.to_str())
                        .unwrap_or("IMG archive")
                );
                return;
            }
            app.dff_prelight_import_dialog = Some(DffPrelightImportDialog {
                img_path: path.clone(),
                entries,
                selected: 0,
                scroll: 0.0,
            });
            app.status_message = format!(
                "Choose DFF prelight source from {}",
                path.file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("IMG archive")
            );
        }
        _ => {
            app.status_message =
                "Import Lighting From DFF needs a .dff file or .img archive".to_string();
        }
    }
}

pub(crate) fn import_selected_prelight_from_img_entry(app: &mut AppState, entry: ImgEntry) {
    let bytes = read_img_entry(&entry);
    if bytes.is_empty() {
        app.status_message = format!("Could not read {} from IMG archive", entry.name);
        return;
    }
    import_selected_prelight_from_dff_bytes(app, &bytes, &entry.name);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn color(r: f32, g: f32, b: f32) -> V3 {
        V3 { x: r, y: g, z: b }
    }

    fn writeback_test_mesh(day: V3, night: V3) -> RenderMesh {
        let positions = [
            color(0.0, 0.0, 0.0),
            color(1.0, 0.0, 0.0),
            color(0.0, 1.0, 0.0),
        ];
        RenderMesh {
            parts: vec![RenderPart {
                list: 0,
                vbo: 0,
                vertices: 3,
                material_index: 0,
                component: 0,
                texture: 0,
                texture_width: 0,
                texture_height: 0,
                texture_name: String::new(),
                texture_fingerprint: None,
                texture_missing: false,
                transparency: TransparencyMode::Opaque,
                alpha: 1.0,
                material_color: neutral_vertex_color(),
                material_ambient: 1.0,
                use_lighting: false,
                vehicle_material_role: None,
                emissive: false,
                cpu_vertices: positions
                    .into_iter()
                    .map(|pos| Vertex {
                        pos,
                        normal: color(0.0, 0.0, 1.0),
                        uv: V2::default(),
                        color: day,
                        day_color: day,
                        night_color: night,
                        base_day_color: neutral_vertex_color(),
                        base_night_color: neutral_vertex_color(),
                        day_alpha: 1.0,
                        night_alpha: 1.0,
                        alpha: 1.0,
                    })
                    .collect(),
                face_indices: vec![0],
            }],
            bounds: Bounds {
                min: Vec3::ZERO,
                max: Vec3::ONE,
            },
            material_animations: Vec::new(),
            uv_animations: Vec::new(),
            effects_2dfx: Vec::new(),
            components: vec!["shared".to_string()],
            component_pivots: vec![None],
        }
    }

    #[test]
    fn refreshed_mesh_can_reapply_night_display_without_changing_prelight_streams() {
        let day = color(0.9, 0.8, 0.7);
        let night = color(0.1, 0.2, 0.3);
        let mut mesh = writeback_test_mesh(day, night);
        for vertex in &mut mesh.parts[0].cpu_vertices {
            vertex.day_alpha = 0.25;
            vertex.night_alpha = 0.75;
            vertex.alpha = vertex.day_alpha;
        }

        apply_bake_light_mode_to_mesh(&mut mesh, BakeLightMode::Night);

        for vertex in &mesh.parts[0].cpu_vertices {
            assert_eq!(vertex.color, night);
            assert_eq!(vertex.day_color, day);
            assert_eq!(vertex.night_color, night);
            assert_eq!(vertex.alpha, 0.75);
            assert_eq!(vertex.day_alpha, 0.25);
            assert_eq!(vertex.night_alpha, 0.75);
        }
    }

    #[test]
    fn manual_clipboard_materializes_distinct_channels_across_save_reload() {
        let copied_day = color(0.8, 0.6, 0.3);
        let copied_night = color(0.1, 0.2, 0.5);
        let source = writeback_test_mesh(copied_day, copied_night);
        let day_samples = render_mesh_prelight_samples(&source, BakeLightMode::Day);
        let night_samples = render_mesh_prelight_samples(&source, BakeLightMode::Night);
        assert_eq!(day_samples.len(), night_samples.len());
        assert!(
            day_samples
                .iter()
                .zip(&night_samples)
                .all(|(day, night)| day.color != night.color)
        );

        // A legacy single-channel DFF is represented in the preview with the
        // same initial value in both runtime channels. A Both-mode paste must
        // replace those channels independently before writeback.
        let original = color(0.35, 0.35, 0.35);
        let mut target = writeback_test_mesh(original, original);
        for ((vertex, day), night) in target.parts[0]
            .cpu_vertices
            .iter_mut()
            .zip(&day_samples)
            .zip(&night_samples)
        {
            set_vertex_bake_color(vertex, BakeLightMode::Day, day.color);
            set_vertex_bake_color(vertex, BakeLightMode::Night, night.color);
        }
        let mut raw = variant_test_raw(original);
        assert!(raw.night_prelit_colors.is_empty());
        apply_runtime_vertex_colors_to_raw(&target, &mut raw).unwrap();
        assert_eq!(raw.prelit_colors, vec![copied_day; 3]);
        assert_eq!(raw.night_prelit_colors, vec![copied_night; 3]);

        let bytes = write_normalized_dff(&raw, "clipboard_channels").unwrap();
        let reloaded = parse_dff_mesh(&bytes);
        assert!(prelight_stream_matches_serialized(
            &vec![copied_day; 3],
            &reloaded.prelit_colors
        ));
        assert!(prelight_stream_matches_serialized(
            &vec![copied_night; 3],
            &reloaded.night_prelit_colors
        ));
        assert_ne!(reloaded.prelit_colors, reloaded.night_prelit_colors);
    }

    #[test]
    fn textured_material_color_does_not_corrupt_prelight_on_save_reload() {
        let day = color(0.2, 0.4, 0.6);
        let night = color(0.1, 0.3, 0.5);
        let mut target = writeback_test_mesh(day, night);
        let part = &mut target.parts[0];
        part.texture_name = "facade".to_string();
        // Textured world parts are compiled without this authored material
        // tint, so their runtime colors already equal serialized prelight.
        part.material_color = color(0.25, 0.5, 0.75);

        let mut raw = variant_test_raw(neutral_vertex_color());
        raw.material_textures = vec![part.texture_name.clone()];
        raw.materials[0].color = part.material_color;
        apply_runtime_vertex_colors_to_raw(&target, &mut raw).unwrap();

        assert_eq!(raw.prelit_colors, vec![day; 3]);
        assert_eq!(raw.night_prelit_colors, vec![night; 3]);

        let bytes = write_normalized_dff(&raw, "textured_prelight").unwrap();
        let reloaded = parse_dff_mesh(&bytes);
        assert!(prelight_stream_matches_serialized(
            &vec![day; 3],
            &reloaded.prelit_colors
        ));
        assert!(prelight_stream_matches_serialized(
            &vec![night; 3],
            &reloaded.night_prelit_colors
        ));
    }

    #[test]
    fn clipboard_transfers_serialized_prelight_across_material_tints() {
        let serialized = color(0.8, 0.6, 0.4);
        let mut source = writeback_test_mesh(V3::default(), V3::default());
        source.parts[0].material_color = color(0.5, 0.25, 1.0);
        let source_runtime = runtime_prelight_from_serialized(&source.parts[0], serialized);
        for vertex in &mut source.parts[0].cpu_vertices {
            vertex.day_color = source_runtime;
            vertex.color = source_runtime;
        }

        let samples = render_mesh_prelight_samples(&source, BakeLightMode::Day);
        assert!(samples.iter().all(|sample| sample.color == serialized));

        let mut target = writeback_test_mesh(V3::default(), V3::default());
        target.parts[0].material_color = color(0.25, 1.0, 0.5);
        let target_runtime = runtime_prelight_from_serialized(&target.parts[0], samples[0].color);
        for vertex in &mut target.parts[0].cpu_vertices {
            vertex.day_color = target_runtime;
            vertex.color = target_runtime;
        }
        let mut raw = variant_test_raw(neutral_vertex_color());
        raw.materials[0].color = target.parts[0].material_color;
        apply_runtime_vertex_colors_to_raw(&target, &mut raw).unwrap();

        assert_eq!(raw.prelit_colors, vec![serialized; 3]);
    }

    #[test]
    fn single_mode_paste_changes_only_selected_channel_and_fills_missing_stream() {
        let original = color(0.35, 0.4, 0.45);
        let copied = color(0.8, 0.2, 0.1);

        for mode in [BakeLightMode::Day, BakeLightMode::Night] {
            let mut target = writeback_test_mesh(original, original);
            for vertex in &mut target.parts[0].cpu_vertices {
                set_vertex_bake_color(vertex, mode, copied);
            }
            let mut raw = variant_test_raw(original);
            if mode == BakeLightMode::Day {
                raw.prelit_colors.clear();
                raw.prelit_alphas.clear();
                raw.night_prelit_colors = vec![original; raw.vertices.len()];
                raw.night_prelit_alphas = vec![1.0; raw.vertices.len()];
            } else {
                raw.night_prelit_colors.clear();
                raw.night_prelit_alphas.clear();
            }

            apply_runtime_vertex_colors_to_raw(&target, &mut raw).unwrap();
            let bytes = write_normalized_dff(&raw, "single_channel_paste").unwrap();
            let reloaded = parse_dff_mesh(&bytes);
            let expected_day = if mode == BakeLightMode::Day {
                copied
            } else {
                original
            };
            let expected_night = if mode == BakeLightMode::Night {
                copied
            } else {
                original
            };
            assert!(prelight_stream_matches_serialized(
                &vec![expected_day; 3],
                &reloaded.prelit_colors
            ));
            assert!(prelight_stream_matches_serialized(
                &vec![expected_night; 3],
                &reloaded.night_prelit_colors
            ));
        }
    }

    #[test]
    fn raw_prelight_samples_use_night_stream_when_available() {
        let raw = RawMesh {
            vertices: vec![color(0.0, 0.0, 0.0), color(1.0, 0.0, 0.0)],
            prelit_colors: vec![color(1.0, 0.0, 0.0), color(0.0, 1.0, 0.0)],
            night_prelit_colors: vec![color(0.0, 0.0, 1.0), color(0.5, 0.5, 0.5)],
            ..RawMesh::default()
        };

        let day = raw_mesh_prelight_samples(&raw, BakeLightMode::Day);
        let night = raw_mesh_prelight_samples(&raw, BakeLightMode::Night);

        assert_eq!(day[0].color, color(1.0, 0.0, 0.0));
        assert_eq!(night[0].color, color(0.0, 0.0, 1.0));
    }

    #[test]
    fn raw_prelight_samples_fall_back_to_day_for_night_imports() {
        let raw = RawMesh {
            vertices: vec![color(0.0, 0.0, 0.0)],
            prelit_colors: vec![color(0.25, 0.5, 0.75)],
            ..RawMesh::default()
        };

        let night = raw_mesh_prelight_samples(&raw, BakeLightMode::Night);

        assert_eq!(night.len(), 1);
        assert_eq!(night[0].color, color(0.25, 0.5, 0.75));
    }

    #[test]
    fn raw_prelight_samples_fall_back_to_night_for_day_imports() {
        let raw = RawMesh {
            vertices: vec![color(0.0, 0.0, 0.0)],
            night_prelit_colors: vec![color(0.1, 0.3, 0.7)],
            ..RawMesh::default()
        };

        let day = raw_mesh_prelight_samples(&raw, BakeLightMode::Day);

        assert_eq!(day.len(), 1);
        assert_eq!(day[0].color, color(0.1, 0.3, 0.7));
    }

    #[test]
    fn raw_prelight_samples_preserve_triangle_order_for_duplicate_positions() {
        let raw = RawMesh {
            vertices: vec![
                color(0.0, 0.0, 0.0),
                color(1.0, 0.0, 0.0),
                color(0.0, 1.0, 0.0),
                color(0.0, 0.0, 0.0),
            ],
            prelit_colors: vec![
                color(1.0, 0.0, 0.0),
                color(0.0, 1.0, 0.0),
                color(0.0, 0.0, 1.0),
                color(1.0, 1.0, 0.0),
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
                    b: 2,
                    c: 1,
                    material: 0,
                },
            ],
            ..RawMesh::default()
        };

        let samples = raw_mesh_prelight_samples(&raw, BakeLightMode::Day);

        assert_eq!(samples.len(), 6);
        assert_eq!(samples[0].geometry.pos, samples[3].geometry.pos);
        assert_eq!(samples[0].color, color(1.0, 0.0, 0.0));
        assert_eq!(samples[3].color, color(1.0, 1.0, 0.0));
    }

    #[test]
    fn prelight_transfer_preserves_distinct_colors_at_coincident_vertices() {
        let shared = color(0.0, 0.0, 0.0);
        let a = color(1.0, 0.0, 0.0);
        let b = color(0.0, 1.0, 0.0);
        let c = color(-1.0, 0.0, 0.0);
        let d = color(0.0, -1.0, 0.0);
        let tri_a = [shared, a, b];
        let tri_b = [shared, c, d];
        let geometry = |pos, triangle| PrelightSampleGeometry {
            pos,
            normal: None,
            uv: None,
            triangle: Some(triangle),
            material: Some(0),
        };
        let source_colors = [
            color(1.0, 0.0, 0.0),
            color(0.8, 0.0, 0.0),
            color(0.6, 0.0, 0.0),
            color(1.0, 1.0, 0.0),
            color(0.8, 0.8, 0.0),
            color(0.6, 0.6, 0.0),
        ];
        let source_geometry = [
            geometry(shared, tri_a),
            geometry(a, tri_a),
            geometry(b, tri_a),
            geometry(shared, tri_b),
            geometry(c, tri_b),
            geometry(d, tri_b),
        ];
        let samples: Vec<_> = source_geometry
            .into_iter()
            .zip(source_colors)
            .map(|(geometry, color)| PrelightClipboardSample { geometry, color })
            .collect();
        // Destination triangles contain the same geometry in a different
        // stream order, which forces the non-ordered transfer path.
        let targets = vec![
            geometry(shared, tri_b),
            geometry(c, tri_b),
            geometry(d, tri_b),
            geometry(shared, tri_a),
            geometry(a, tri_a),
            geometry(b, tri_a),
        ];

        let (matches, exact_order) = match_prelight_samples(&targets, &samples);
        let transferred: Vec<_> = matches
            .iter()
            .map(|(sample_idx, _)| samples[*sample_idx].color)
            .collect();

        assert!(!exact_order);
        assert_eq!(
            transferred,
            vec![
                source_colors[3],
                source_colors[4],
                source_colors[5],
                source_colors[0],
                source_colors[1],
                source_colors[2],
            ]
        );
        assert_ne!(transferred[0], transferred[3]);
    }

    #[test]
    fn blend_source_search_respects_brush_radius() {
        let red = color(1.0, 0.0, 0.0);
        let sources = vec![(vec3(150.0, 0.0, 0.0), red)];
        let grid = build_source_vertex_grid(&sources, 100.0);

        assert!(closest_source_vertex(&sources, &grid, Vec3::ZERO, 100.0, 100.0).is_none());
        let (distance, sampled) =
            closest_source_vertex(&sources, &grid, Vec3::ZERO, 100.0, 200.0).unwrap();
        assert!((distance - 150.0).abs() < 0.001);
        assert_eq!(sampled, red);
    }

    #[test]
    fn vertex_writeback_preserves_source_dff_sidecars_and_removes_preview_tint() {
        let positions = [
            color(0.0, 0.0, 0.0),
            color(1.0, 0.0, 0.0),
            color(0.0, 1.0, 0.0),
        ];
        let component = RawMeshComponent {
            name: "door".to_string(),
            vertex_start: 0,
            vertex_end: 3,
            tri_start: 0,
            tri_end: 1,
            breakable: None,
        };
        let effect = Dff2dEffect {
            position: color(2.0, 3.0, 4.0),
            effect_id: 7,
            payload: b"light\0".to_vec(),
        };
        let mut raw = RawMesh {
            vertices: positions.to_vec(),
            triangles: vec![Tri {
                a: 0,
                b: 1,
                c: 2,
                material: 0,
            }],
            materials: vec![RawMaterial {
                color: color(0.5, 0.5, 0.5),
                alpha: 1.0,
                ambient: 1.0,
                specular: 1.0,
                diffuse: 1.0,
            }],
            material_textures: vec![String::new()],
            components: vec![component.clone()],
            effects_2dfx: vec![effect.clone()],
            ..RawMesh::default()
        };
        let vertices = positions
            .into_iter()
            .map(|pos| Vertex {
                pos,
                normal: color(0.0, 0.0, 1.0),
                uv: V2::default(),
                color: color(0.25, 0.25, 0.25),
                day_color: color(0.25, 0.25, 0.25),
                night_color: color(0.125, 0.125, 0.125),
                base_day_color: neutral_vertex_color(),
                base_night_color: neutral_vertex_color(),
                day_alpha: 1.0,
                night_alpha: 1.0,
                alpha: 1.0,
            })
            .collect();
        let mesh = RenderMesh {
            parts: vec![RenderPart {
                list: 0,
                vbo: 0,
                vertices: 3,
                material_index: 0,
                component: 0,
                texture: 0,
                texture_width: 0,
                texture_height: 0,
                texture_name: String::new(),
                texture_fingerprint: None,
                texture_missing: false,
                transparency: TransparencyMode::Opaque,
                alpha: 1.0,
                material_color: color(0.5, 0.5, 0.5),
                material_ambient: 1.0,
                use_lighting: false,
                vehicle_material_role: None,
                emissive: false,
                cpu_vertices: vertices,
                face_indices: vec![0],
            }],
            bounds: Bounds {
                min: Vec3::ZERO,
                max: Vec3::ONE,
            },
            material_animations: Vec::new(),
            uv_animations: Vec::new(),
            effects_2dfx: vec![effect.clone()],
            components: vec![component.name.clone()],
            component_pivots: vec![None],
        };

        apply_runtime_vertex_colors_to_raw(&mesh, &mut raw).unwrap();

        assert!(raw.components == vec![component]);
        assert!(raw.effects_2dfx == vec![effect]);
        assert_eq!(raw.materials[0].color, color(0.5, 0.5, 0.5));
        assert_eq!(raw.prelit_colors, vec![color(0.5, 0.5, 0.5); 3]);
        assert_eq!(raw.night_prelit_colors, vec![color(0.25, 0.25, 0.25); 3]);
    }

    #[test]
    fn vertex_writeback_batches_and_blends_txd_variants_of_one_dff() {
        let root = std::env::temp_dir().join(format!(
            "eagle_vertex_batch_source_{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let output = root.join("output");
        let img_dir = root.join("imgs");
        fs::create_dir_all(&img_dir).unwrap();
        let raw = RawMesh {
            vertices: vec![
                color(0.0, 0.0, 0.0),
                color(1.0, 0.0, 0.0),
                color(0.0, 1.0, 0.0),
            ],
            triangles: vec![Tri {
                a: 0,
                b: 1,
                c: 2,
                material: 0,
            }],
            materials: vec![RawMaterial {
                color: neutral_vertex_color(),
                alpha: 1.0,
                ambient: 1.0,
                specular: 1.0,
                diffuse: 1.0,
            }],
            material_textures: vec![String::new()],
            components: vec![RawMeshComponent {
                name: "shared".to_string(),
                vertex_start: 0,
                vertex_end: 3,
                tri_start: 0,
                tri_end: 1,
                breakable: None,
            }],
            ..RawMesh::default()
        };
        write_img_archive(
            &img_dir.join("dff.img"),
            &[(
                "shared.dff".to_string(),
                write_normalized_dff(&raw, "shared").unwrap(),
            )],
        )
        .unwrap();
        let red = writeback_test_mesh(color(1.0, 0.0, 0.0), color(0.5, 0.0, 0.0));
        let blue = writeback_test_mesh(color(0.0, 0.0, 1.0), color(0.0, 0.0, 0.5));

        let written = write_vertex_lighting_meshes(
            &root,
            &output,
            [
                ("shared.dff|red.txd", &red, DffWriteOptions::default()),
                ("shared.dff|blue.txd", &blue, DffWriteOptions::default()),
            ],
        )
        .unwrap();

        assert_eq!(written, 1);
        let entries = parse_img(&output.join("imgs").join(REPLACEMENT_IMG));
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "shared.dff");
        let bytes = read_img_entry(&entries[0]);
        let saved = parse_dff_mesh(&bytes[..dff_chunk_len(&bytes)]);
        let half = 128.0 / 255.0;
        let quarter = 64.0 / 255.0;
        assert_eq!(saved.prelit_colors, vec![color(half, 0.0, half); 3]);
        assert_eq!(
            saved.night_prelit_colors,
            vec![color(quarter, 0.0, quarter); 3]
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn vertex_writeback_prefers_staged_topology_and_repairs_compatible_mismatches() {
        let root = std::env::temp_dir().join(format!(
            "eagle_vertex_staged_source_{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let output = root.join("output");
        fs::create_dir_all(root.join("imgs")).unwrap();
        fs::create_dir_all(output.join("imgs")).unwrap();

        let mut source = RawMesh {
            vertices: vec![
                color(0.0, 0.0, 0.0),
                color(1.0, 0.0, 0.0),
                color(0.0, 1.0, 0.0),
                color(1.0, 1.0, 0.0),
            ],
            triangles: vec![Tri {
                a: 0,
                b: 1,
                c: 2,
                material: 0,
            }],
            materials: vec![RawMaterial {
                color: neutral_vertex_color(),
                alpha: 1.0,
                ambient: 1.0,
                specular: 1.0,
                diffuse: 1.0,
            }],
            material_textures: vec![String::new()],
            components: vec![RawMeshComponent {
                name: "shared".to_string(),
                vertex_start: 0,
                vertex_end: 4,
                tri_start: 0,
                tri_end: 1,
                breakable: None,
            }],
            ..RawMesh::default()
        };
        let staged_bytes = write_normalized_dff(&source, "shared").unwrap();

        // The saved base archive still has an extra face. This is the state
        // seen while an optimized replacement is staged but not promoted.
        source.triangles.push(Tri {
            a: 1,
            b: 3,
            c: 2,
            material: 0,
        });
        source.components[0].tri_end = 2;
        write_img_archive(
            &root.join("imgs").join("dff.img"),
            &[(
                "shared.dff".to_string(),
                write_normalized_dff(&source, "shared").unwrap(),
            )],
        )
        .unwrap();
        write_img_archive(
            &output.join("imgs").join(REPLACEMENT_IMG),
            &[("shared.dff".to_string(), staged_bytes)],
        )
        .unwrap();

        let mesh = writeback_test_mesh(color(0.25, 0.5, 0.75), color(0.1, 0.2, 0.3));
        assert_eq!(
            write_vertex_lighting_meshes(
                &root,
                &output,
                [("shared.dff|shared.txd", &mesh, DffWriteOptions::default())],
            )
            .unwrap(),
            1
        );

        // Without a staged source, geometry-aware repair should preserve the
        // shared corners and safely handle the extra source face.
        let empty_output = root.join("empty_output");
        let outcome = write_vertex_lighting_meshes_resilient(
            &root,
            &empty_output,
            [("shared.dff|shared.txd", &mesh, DffWriteOptions::default())],
        )
        .unwrap();
        assert_eq!(outcome.written, 1);
        assert!(
            outcome
                .warnings
                .iter()
                .any(|warning| warning.contains("repaired vertex-light topology"))
        );

        // A genuinely unrelated live mesh is not safe to repair, but its
        // failure must skip only this DFF instead of cancelling the save.
        let mut unrelated = mesh.clone();
        for part in &mut unrelated.parts {
            for vertex in &mut part.cpu_vertices {
                vertex.pos.x += 100.0;
            }
        }
        let skipped_output = root.join("skipped_output");
        let outcome = write_vertex_lighting_meshes_resilient(
            &root,
            &skipped_output,
            [(
                "shared.dff|shared.txd",
                &unrelated,
                DffWriteOptions::default(),
            )],
        )
        .unwrap();
        assert_eq!(outcome.written, 0);
        assert!(
            outcome
                .warnings
                .iter()
                .any(|warning| warning.contains("rest of the save continued"))
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn final_wip_vertex_colors_are_the_bytes_promoted_to_the_resource() {
        let root = std::env::temp_dir().join(format!(
            "eagle_vertex_promotion_{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let wip_root = wip_root_path(&root);
        fs::create_dir_all(root.join("imgs")).unwrap();
        fs::create_dir_all(wip_root.join("imgs")).unwrap();
        let raw = RawMesh {
            vertices: vec![
                color(0.0, 0.0, 0.0),
                color(1.0, 0.0, 0.0),
                color(0.0, 1.0, 0.0),
            ],
            triangles: vec![Tri {
                a: 0,
                b: 1,
                c: 2,
                material: 0,
            }],
            prelit_colors: vec![neutral_vertex_color(); 3],
            night_prelit_colors: vec![neutral_vertex_color(); 3],
            materials: vec![RawMaterial {
                color: neutral_vertex_color(),
                alpha: 1.0,
                ambient: 1.0,
                specular: 1.0,
                diffuse: 1.0,
            }],
            material_textures: vec![String::new()],
            components: vec![RawMeshComponent {
                name: "shared".to_string(),
                vertex_start: 0,
                vertex_end: 3,
                tri_start: 0,
                tri_end: 1,
                breakable: None,
            }],
            ..RawMesh::default()
        };
        let staged = write_normalized_dff(&raw, "shared").unwrap();
        write_img_archive(
            &root.join("imgs").join("dff.img"),
            &[("shared.dff".to_string(), staged.clone())],
        )
        .unwrap();
        // This represents a DFF already staged by the Editing tab. Lighting
        // writeback must update these bytes and promotion must not replace
        // them with the older in-memory staged version afterward.
        write_img_archive(
            &wip_root.join("imgs").join(REPLACEMENT_IMG),
            &[("shared.dff".to_string(), staged)],
        )
        .unwrap();
        let day = color(0.2, 0.4, 0.6);
        let night = color(0.1, 0.3, 0.5);
        let mesh = writeback_test_mesh(day, night);

        write_vertex_lighting_meshes(
            &root,
            &wip_root,
            [("shared.dff|shared.txd", &mesh, DffWriteOptions::default())],
        )
        .unwrap();
        let final_entries = replacement_archive_entries(&wip_root).unwrap();
        merge_replacement_entries_into_root(&root, &root, final_entries).unwrap();

        let entry = parse_img(&root.join("imgs").join("dff.img"))
            .into_iter()
            .find(|entry| entry.name.eq_ignore_ascii_case("shared.dff"))
            .unwrap();
        let bytes = read_img_entry(&entry);
        let saved = parse_dff_mesh(&bytes[..dff_chunk_len(&bytes)]);
        let quantized = |value: f32| (value * 255.0).round() / 255.0;
        assert_eq!(
            saved.prelit_colors,
            vec![color(quantized(0.2), quantized(0.4), quantized(0.6)); 3]
        );
        assert_eq!(
            saved.night_prelit_colors,
            vec![color(quantized(0.1), quantized(0.3), quantized(0.5)); 3]
        );

        let _ = fs::remove_dir_all(root);
    }

    fn variant_test_placement(id: &str, pos: V3) -> Placement {
        let mut attrs = BTreeMap::new();
        attrs.insert("id".to_string(), id.to_string());
        Placement {
            id: id.to_string(),
            dff: id.to_string(),
            zone: "zone".to_string(),
            tag: "object".to_string(),
            attrs,
            pos,
            rot: V3::default(),
        }
    }

    fn variant_test_raw(prelight: V3) -> RawMesh {
        RawMesh {
            vertices: vec![
                color(0.0, 0.0, 0.0),
                color(1.0, 0.0, 0.0),
                color(0.0, 1.0, 0.0),
            ],
            normals: vec![color(0.0, 0.0, 1.0); 3],
            uvs: vec![
                V2 { u: 0.0, v: 0.0 },
                V2 { u: 1.0, v: 0.0 },
                V2 { u: 0.0, v: 1.0 },
            ],
            prelit_colors: vec![prelight; 3],
            prelit_alphas: vec![1.0; 3],
            triangles: vec![Tri {
                a: 0,
                b: 1,
                c: 2,
                material: 0,
            }],
            materials: vec![RawMaterial {
                color: neutral_vertex_color(),
                alpha: 1.0,
                ambient: 1.0,
                specular: 1.0,
                diffuse: 1.0,
            }],
            material_textures: vec!["wall".to_string()],
            ..RawMesh::default()
        }
    }

    #[test]
    fn day_night_variant_names_accept_vc_suffixes_and_mixed_case_extensions() {
        assert_eq!(
            day_night_variant_name("mall_nt.DfF"),
            Some(DayNightVariantName {
                base: "mall".to_string(),
                variant: DayNightVariant::Night,
            })
        );
        assert_eq!(
            day_night_variant_name("MALL_DY"),
            Some(DayNightVariantName {
                base: "mall".to_string(),
                variant: DayNightVariant::Day,
            })
        );
        assert_eq!(
            day_night_variant_name("mall_dt.dff"),
            Some(DayNightVariantName {
                base: "mall".to_string(),
                variant: DayNightVariant::Day,
            })
        );
        assert_eq!(day_night_variant_name("mall"), None);
        assert_eq!(day_night_base_id("MALL_DY"), Some("MALL".to_string()));
        assert_eq!(day_night_base_id("mall_nt"), Some("mall".to_string()));
        assert_eq!(day_night_base_id("mall"), None);

        let suffixed_id = variant_test_placement("mall_nt", V3::default());
        assert!(placement_has_day_night_variant_suffix(&suffixed_id));
        let mut suffixed_dff = variant_test_placement("mall", V3::default());
        suffixed_dff.dff = "mall_dt".to_string();
        assert!(placement_has_day_night_variant_suffix(&suffixed_dff));
        let plain = variant_test_placement("mall", V3::default());
        assert!(!placement_has_day_night_variant_suffix(&plain));
    }

    #[test]
    fn day_night_counterpart_respects_position_tolerance() {
        let placements = vec![
            variant_test_placement("mall_nt", color(10.0, 20.0, 30.0)),
            variant_test_placement("mall_dt", color(10.0, 20.0, 30.015)),
            variant_test_placement("mall_dy", color(11.0, 20.0, 30.0)),
        ];
        let states = vec![ElementState::default(); placements.len()];

        assert_eq!(
            find_day_night_counterpart(&placements, &states, 0, DEFAULT_DAY_NIGHT_MERGE_TOLERANCE)
                .unwrap(),
            1
        );
        assert!(find_day_night_counterpart(&placements, &states, 0, 0.01).is_err());
    }

    #[test]
    fn day_night_merge_keeps_both_prelight_streams_when_night_is_selected() {
        let night = color(0.1, 0.2, 0.3);
        let day = color(0.7, 0.8, 0.9);
        let target_night = variant_test_raw(night);
        let source_day = variant_test_raw(day);

        let merged =
            merge_day_night_raw_meshes(&target_night, &source_day, DayNightVariant::Night).unwrap();

        assert_eq!(merged.prelit_colors, vec![day; 3]);
        assert_eq!(merged.night_prelit_colors, vec![night; 3]);
        assert_eq!(merged.prelit_alphas, vec![1.0; 3]);
        assert_eq!(merged.night_prelit_alphas, vec![1.0; 3]);

        let bytes = write_normalized_dff(&merged, "mall_nt").unwrap();
        let serialized = parse_dff_mesh(&bytes);
        assert!(prelight_stream_matches_serialized(
            &merged.prelit_colors,
            &serialized.prelit_colors
        ));
        assert!(prelight_stream_matches_serialized(
            &merged.night_prelit_colors,
            &serialized.night_prelit_colors
        ));
        assert!(prelight_alpha_matches_serialized(
            &merged.prelit_alphas,
            &serialized.prelit_alphas
        ));
        assert!(prelight_alpha_matches_serialized(
            &merged.night_prelit_alphas,
            &serialized.night_prelit_alphas
        ));
    }

    #[test]
    fn day_night_merge_maps_reordered_vertices_faces_and_material_slots() {
        let mut day = variant_test_raw(color(0.8, 0.8, 0.8));
        day.vertices.push(color(1.0, 1.0, 0.0));
        day.normals.push(color(0.0, 0.0, 1.0));
        day.uvs.push(V2 { u: 1.0, v: 1.0 });
        day.prelit_colors.push(color(0.8, 0.8, 0.8));
        day.prelit_alphas.push(1.0);
        day.triangles.push(Tri {
            a: 2,
            b: 1,
            c: 3,
            material: 1,
        });
        day.materials.push(RawMaterial {
            color: color(0.5, 0.5, 0.5),
            alpha: 1.0,
            ambient: 1.0,
            specular: 0.0,
            diffuse: 1.0,
        });
        day.material_textures.push("road".to_string());
        day.light_flags = vec![true, false, true, false];

        let old_night = [
            color(0.1, 0.2, 0.3),
            color(0.2, 0.3, 0.4),
            color(0.3, 0.4, 0.5),
            color(0.4, 0.5, 0.6),
        ];
        let new_to_old = [2usize, 0, 3, 1];
        let old_to_new = [1u32, 3, 0, 2];
        let mut night = day.clone();
        night.vertices = new_to_old.map(|index| day.vertices[index]).to_vec();
        night.normals = new_to_old.map(|index| day.normals[index]).to_vec();
        night.uvs = new_to_old
            .map(|index| V2 {
                u: day.uvs[index].u + 0.25,
                v: day.uvs[index].v - 0.5,
            })
            .to_vec();
        night.prelit_colors = new_to_old.map(|index| old_night[index]).to_vec();
        night.prelit_alphas = vec![1.0; 4];
        night.light_flags = new_to_old.map(|index| day.light_flags[index]).to_vec();
        night.materials.swap(0, 1);
        night.material_textures.swap(0, 1);
        night.triangles = vec![
            Tri {
                a: old_to_new[2],
                b: old_to_new[1],
                c: old_to_new[3],
                material: 0,
            },
            Tri {
                a: old_to_new[0],
                b: old_to_new[1],
                c: old_to_new[2],
                material: 1,
            },
        ];

        let analysis = analyze_day_night_raw_meshes(&day, &night).unwrap();
        assert!(
            analysis
                .notes
                .iter()
                .any(|note| note.contains("raw stream/index ordering differs"))
        );
        assert!(
            analysis
                .notes
                .iter()
                .any(|note| note.contains("primary UVs differ"))
        );

        let merged = merge_day_night_raw_meshes(&day, &night, DayNightVariant::Day).unwrap();
        assert_eq!(merged.prelit_colors, day.prelit_colors);
        assert_eq!(merged.night_prelit_colors, old_night);
        assert_eq!(merged.vertices, day.vertices);
        assert_eq!(merged.uvs, day.uvs);
        assert_eq!(merged.material_textures, day.material_textures);

        let mut serialized = night;
        serialized.uvs = new_to_old.map(|index| merged.uvs[index]).to_vec();
        serialized.prelit_colors = new_to_old.map(|index| merged.prelit_colors[index]).to_vec();
        serialized.prelit_alphas = new_to_old.map(|index| merged.prelit_alphas[index]).to_vec();
        serialized.night_prelit_colors = new_to_old
            .map(|index| merged.night_prelit_colors[index])
            .to_vec();
        serialized.night_prelit_alphas = new_to_old
            .map(|index| merged.night_prelit_alphas[index])
            .to_vec();
        assert!(verify_serialized_day_night_prelight(&merged, &serialized, true).is_ok());

        serialized.night_prelit_colors[0] = color(1.0, 0.0, 1.0);
        let error = verify_serialized_day_night_prelight(&merged, &serialized, true).unwrap_err();
        assert!(error.contains("night prelight colors changed"));
    }

    #[test]
    fn serialized_prelight_verification_accepts_writer_collapsed_triangle() {
        let mut expected = variant_test_raw(color(0.2, 0.4, 0.6));
        expected.night_prelit_colors = vec![color(0.1, 0.3, 0.5); 3];
        expected.night_prelit_alphas = vec![1.0; 3];
        expected.vertices.push(color(0.0004, 0.0, 0.0));
        expected.normals.push(color(0.0, 0.0, 1.0));
        expected.uvs.push(V2 { u: 0.0, v: 0.0 });
        expected.prelit_colors.push(color(0.2, 0.4, 0.6));
        expected.prelit_alphas.push(1.0);
        expected.night_prelit_colors.push(color(0.1, 0.3, 0.5));
        expected.night_prelit_alphas.push(1.0);
        expected.triangles.push(Tri {
            a: 0,
            b: 3,
            c: 1,
            material: 0,
        });

        let bytes = write_normalized_dff_with_options(
            &expected,
            "lod_test",
            DffWriteOptions {
                include_normals: false,
                include_bin_mesh: true,
            },
        )
        .unwrap();
        let mut actual = parse_dff_mesh(&bytes);
        assert_eq!(expected.triangles.len(), 2);
        assert_eq!(actual.triangles.len(), 1);
        verify_serialized_day_night_prelight(&expected, &actual, false).unwrap();

        actual.night_prelit_colors[0] = color(1.0, 0.0, 1.0);
        let error = verify_serialized_day_night_prelight(&expected, &actual, false).unwrap_err();
        assert!(error.contains("night prelight colors changed"));
    }

    #[test]
    fn serialized_verification_disambiguates_coincident_color_seams() {
        let positions = [
            color(0.0, 0.0, 0.0),
            color(1.0, 0.0, 0.0),
            color(0.0, 1.0, 0.0),
            color(0.0, 0.0, 0.0),
            color(1.0, 0.0, 0.0),
            color(0.0, 1.0, 0.0),
        ];
        let expected = RawMesh {
            vertices: positions.to_vec(),
            normals: vec![color(0.0, 0.0, 1.0); positions.len()],
            uvs: vec![
                V2 { u: 0.0, v: 0.0 },
                V2 { u: 1.0, v: 0.0 },
                V2 { u: 0.0, v: 1.0 },
                V2 { u: 0.0, v: 0.0 },
                V2 { u: 1.0, v: 0.0 },
                V2 { u: 0.0, v: 1.0 },
            ],
            prelit_colors: [vec![color(0.8, 0.1, 0.1); 3], vec![color(0.1, 0.8, 0.1); 3]].concat(),
            prelit_alphas: vec![1.0; positions.len()],
            night_prelit_colors: [vec![color(0.1, 0.1, 0.8); 3], vec![color(0.8, 0.8, 0.1); 3]]
                .concat(),
            night_prelit_alphas: vec![1.0; positions.len()],
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
                    material: 0,
                },
            ],
            materials: vec![RawMaterial {
                color: neutral_vertex_color(),
                alpha: 1.0,
                ambient: 1.0,
                specular: 0.0,
                diffuse: 1.0,
            }],
            material_textures: vec!["bridge".to_string()],
            ..RawMesh::default()
        };
        let bytes = write_normalized_dff(&expected, "bridge_seams").unwrap();
        let mut actual = parse_dff_mesh(&bytes);
        assert_eq!(actual.triangles.len(), 2);

        // RenderWare/bin-mesh ordering is not a stable identity. Reverse two
        // geometrically identical triangles to exercise the ambiguous mapping.
        actual.triangles.reverse();

        verify_serialized_day_night_prelight(&expected, &actual, true).unwrap();
    }

    #[test]
    fn day_night_merge_reports_and_preserves_material_differences() {
        let night = variant_test_raw(color(0.1, 0.2, 0.3));
        let mut day = variant_test_raw(color(0.7, 0.8, 0.9));
        day.material_textures[0] = "lit_window".to_string();

        let differences = analyze_day_night_raw_meshes(&night, &day).unwrap_err();

        assert!(
            differences
                .iter()
                .any(|difference| difference.contains("material texture assignments differ"))
        );
        assert!(merge_day_night_raw_meshes(&night, &day, DayNightVariant::Night).is_err());
        // The comparison itself must never mutate either source.
        assert_eq!(night.material_textures, vec!["wall".to_string()]);
        assert_eq!(day.material_textures, vec!["lit_window".to_string()]);
    }

    fn variant_test_definition(id: &str, time_in: &str, time_out: &str) -> Definition {
        let mut attrs = BTreeMap::new();
        attrs.insert("id".to_string(), id.to_string());
        attrs.insert("dff".to_string(), id.to_string());
        attrs.insert("col".to_string(), id.to_string());
        attrs.insert("timeIn".to_string(), time_in.to_string());
        attrs.insert("timeOut".to_string(), time_out.to_string());
        Definition {
            id: id.to_string(),
            zone: "zone".to_string(),
            attrs,
        }
    }

    fn variant_test_context(root: PathBuf, day: RawMesh) -> DayNightMergeContext {
        let night = variant_test_raw(color(0.1, 0.2, 0.3));
        let placements = vec![
            variant_test_placement("mall_nt", color(10.0, 20.0, 30.0)),
            variant_test_placement("mall_dt", color(10.0, 20.0, 30.0)),
        ];
        let definitions = HashMap::from([
            (
                "mall_nt".to_string(),
                variant_test_definition("mall_nt", "20", "6"),
            ),
            (
                "mall_dt".to_string(),
                variant_test_definition("mall_dt", "6", "20"),
            ),
        ]);
        let byte_overrides = BTreeMap::from([
            (
                asset_key("mall_nt", ".dff"),
                (
                    "mall_nt.dff".to_string(),
                    write_normalized_dff(&night, "mall_nt").unwrap(),
                ),
            ),
            (
                asset_key("mall_dt", ".dff"),
                (
                    "mall_dt.dff".to_string(),
                    write_normalized_dff(&day, "mall_dt").unwrap(),
                ),
            ),
        ]);
        DayNightMergeContext {
            root,
            gta_sa_dir: PathBuf::new(),
            selected_index: 0,
            explicit_source_index: None,
            states: vec![ElementState::default(); placements.len()],
            placements,
            definitions,
            readonly_definition_ids: HashSet::new(),
            byte_overrides,
            raw_overrides: BTreeMap::new(),
            vertex_mesh_overrides: BTreeMap::new(),
            building_dffs: HashSet::new(),
            tolerance: 0.01,
            force_review_override: false,
        }
    }

    #[test]
    fn day_night_worker_deletes_only_after_verified_serialization() {
        let root = std::env::temp_dir().join(format!(
            "eagle_day_night_merge_test_{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let day = variant_test_raw(color(0.7, 0.8, 0.9));
        let (progress, _) = mpsc::channel();

        let merged =
            run_day_night_merge(variant_test_context(root.clone(), day.clone()), &progress);

        assert_eq!(
            merged.replacements.len(),
            1,
            "unexpected review: {:?}",
            merged.reviews
        );
        assert_eq!(merged.delete_indices, vec![1]);
        assert!(merged.reviews.is_empty());
        let written = parse_dff_mesh(&merged.replacements[0].bytes);
        assert!(prelight_stream_matches_serialized(
            &day.prelit_colors,
            &written.prelit_colors
        ));

        let mut different_day = day;
        different_day.material_textures[0] = "lit_window".to_string();
        let reviewed = run_day_night_merge(
            variant_test_context(root.clone(), different_day.clone()),
            &progress,
        );
        assert!(reviewed.replacements.is_empty());
        assert!(reviewed.delete_indices.is_empty());
        assert_eq!(reviewed.reviews.len(), 1);
        assert!(reviewed.override_available);

        let mut override_context = variant_test_context(root.clone(), different_day);
        override_context.force_review_override = true;
        let overridden = run_day_night_merge(override_context, &progress);
        assert_eq!(
            overridden.replacements.len(),
            1,
            "unexpected override review: {:?}",
            overridden.reviews
        );
        assert_eq!(overridden.delete_indices, vec![1]);
        assert!(overridden.reviews.is_empty());
        assert!(!overridden.override_available);
        assert!(
            overridden.replacements[0]
                .notes
                .iter()
                .any(|note| note.contains("explicit second-click override"))
        );
        let overridden_raw = parse_dff_mesh(&overridden.replacements[0].bytes);
        assert_eq!(
            overridden_raw.material_textures,
            vec!["wall".to_string()],
            "the retained model's non-lighting material data must remain intact"
        );
        assert!(prelight_stream_matches_serialized(
            &variant_test_raw(color(0.7, 0.8, 0.9)).prelit_colors,
            &overridden_raw.prelit_colors
        ));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn explicitly_selected_pair_bypasses_name_and_origin_matching() {
        let root = std::env::temp_dir().join(format!(
            "eagle_selected_day_night_merge_test_{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let day = variant_test_raw(color(0.7, 0.8, 0.9));
        let night = variant_test_raw(color(0.1, 0.2, 0.3));
        let mut context = variant_test_context(root.clone(), day.clone());
        context.explicit_source_index = Some(1);
        context.placements = vec![
            variant_test_placement("sunlit_version", color(10.0, 20.0, 30.0)),
            variant_test_placement("after_dark_version", color(40.0, 50.0, 60.0)),
        ];
        context.placements[0].dff = "custom_a".to_string();
        context.placements[1].dff = "custom_b".to_string();
        context.definitions = HashMap::from([
            (
                "sunlit_version".to_string(),
                variant_test_definition("sunlit_version", "6", "20"),
            ),
            (
                "after_dark_version".to_string(),
                variant_test_definition("after_dark_version", "20", "6"),
            ),
        ]);
        for definition in context.definitions.values_mut() {
            definition
                .attrs
                .insert("col".to_string(), "shared_collision".to_string());
        }
        context.byte_overrides = BTreeMap::from([
            (
                asset_key("custom_a", ".dff"),
                (
                    "custom_a.dff".to_string(),
                    write_normalized_dff(&day, "shared_model").unwrap(),
                ),
            ),
            (
                asset_key("custom_b", ".dff"),
                (
                    "custom_b.dff".to_string(),
                    write_normalized_dff(&night, "shared_model").unwrap(),
                ),
            ),
        ]);
        let (progress, _) = mpsc::channel();

        let merged = run_day_night_merge(context, &progress);

        assert_eq!(
            merged.replacements.len(),
            1,
            "unexpected review: {:?}",
            merged.reviews
        );
        assert_eq!(merged.delete_indices, vec![1]);
        assert!(merged.reviews.is_empty());
        assert_eq!(merged.replacements[0].target_name, "custom_a.dff");
        let written = parse_dff_mesh(&merged.replacements[0].bytes);
        assert!(prelight_stream_matches_serialized(
            &day.prelit_colors,
            &written.prelit_colors
        ));
        assert!(prelight_stream_matches_serialized(
            &night.prelit_colors,
            &written.night_prelit_colors
        ));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn light_lod_worker_transfers_both_streams_and_preserves_lod_model_data() {
        let root = std::env::temp_dir().join(format!(
            "eagle_light_lod_test_{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let mut detail = variant_test_raw(color(0.8, 0.6, 0.4));
        detail.night_prelit_colors = vec![color(0.1, 0.2, 0.5); detail.vertices.len()];
        detail.night_prelit_alphas = vec![0.75; detail.vertices.len()];
        let mut lod = variant_test_raw(color(0.0, 0.0, 0.0));
        lod.material_textures[0] = "lod_only_material".to_string();
        lod.vertices[1].x = 0.9;
        lod.vertices[2].y = 0.9;

        let mut detail_placement = variant_test_placement("main", V3::default());
        detail_placement
            .attrs
            .insert("lodParent".to_string(), "lodmain".to_string());
        let lod_placement = variant_test_placement("lodmain", V3::default());
        let mut context = variant_test_context(root.clone(), detail.clone());
        context.placements = vec![detail_placement, lod_placement];
        context.states = vec![ElementState::default(); 2];
        context.definitions.clear();
        context.byte_overrides = BTreeMap::from([
            (
                asset_key("main", ".dff"),
                (
                    "main.dff".to_string(),
                    write_normalized_dff(&detail, "main").unwrap(),
                ),
            ),
            (
                asset_key("lodmain", ".dff"),
                (
                    "lodmain.dff".to_string(),
                    write_normalized_dff(&lod, "lodmain").unwrap(),
                ),
            ),
        ]);

        let output = run_light_lod(context).expect("Light LOD should succeed");
        let written = parse_dff_mesh(&output.bytes);

        assert_eq!(output.target_name, "lodmain.dff");
        assert_eq!(
            written.material_textures,
            vec!["lod_only_material".to_string()]
        );
        assert_eq!(written.vertices.len(), lod.vertices.len());
        assert_eq!(written.triangles.len(), lod.triangles.len());
        assert!(prelight_stream_matches_serialized(
            &vec![color(0.8, 0.6, 0.4); lod.vertices.len()],
            &written.prelit_colors
        ));
        assert!(prelight_stream_matches_serialized(
            &vec![color(0.1, 0.2, 0.5); lod.vertices.len()],
            &written.night_prelit_colors
        ));
        assert_ne!(written.prelit_colors, written.night_prelit_colors);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn successful_day_night_merge_removes_suffix_without_changing_dff_reference() {
        let root = std::env::temp_dir().join(format!(
            "eagle_day_night_rename_test_{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let day = variant_test_raw(color(0.7, 0.8, 0.9));
        let context = variant_test_context(root.clone(), day);
        let mut placements = context.placements.clone();
        let mut states = context.states.clone();
        let mut definitions = context.definitions.clone();
        let (progress, _) = mpsc::channel();

        let merged = run_day_night_merge(context, &progress);
        for index in &merged.delete_indices {
            states[*index].deleted = true;
        }
        let renamed = apply_successful_day_night_id_renames(
            &mut placements,
            &states,
            &mut definitions,
            &merged.replacements,
        );

        assert_eq!(renamed, vec![("mall_nt".to_string(), "mall".to_string())]);
        assert_eq!(placements[0].id, "mall");
        assert_eq!(placements[0].dff, "mall_nt");
        assert_eq!(
            placements[0].attrs.get("id").map(String::as_str),
            Some("mall")
        );
        let definition = definitions.get("mall").unwrap();
        assert_eq!(
            definition.attrs.get("dff").map(String::as_str),
            Some("mall_nt")
        );
        assert!(!definition_has_day_night_time_window(definition));
        assert!(!definitions.contains_key("mall_nt"));
        assert!(!definitions.contains_key("mall_dt"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn day_night_merge_adds_letter_suffix_when_base_id_exists() {
        let root = std::env::temp_dir().join(format!(
            "eagle_day_night_rename_conflict_test_{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let day = variant_test_raw(color(0.7, 0.8, 0.9));
        let mut context = variant_test_context(root.clone(), day);
        context
            .placements
            .push(variant_test_placement("mall", color(100.0, 0.0, 0.0)));
        context.states.push(ElementState::default());
        let mut placements = context.placements.clone();
        let mut states = context.states.clone();
        let mut definitions = context.definitions.clone();
        let (progress, _) = mpsc::channel();

        let merged = run_day_night_merge(context, &progress);
        for index in &merged.delete_indices {
            states[*index].deleted = true;
        }
        let renamed = apply_successful_day_night_id_renames(
            &mut placements,
            &states,
            &mut definitions,
            &merged.replacements,
        );

        assert_eq!(merged.replacements.len(), 1);
        assert_eq!(merged.delete_indices, vec![1]);
        assert!(merged.reviews.is_empty());
        assert_eq!(renamed, vec![("mall_nt".to_string(), "mall_A".to_string())]);
        assert_eq!(placements[0].id, "mall_A");
        assert_eq!(placements[2].id, "mall");
        assert!(definitions.contains_key("mall_A"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn day_night_merge_uses_existing_opposite_lod_when_selected_lod_is_missing() {
        let root = std::env::temp_dir().join(format!(
            "eagle_day_night_one_lod_test_{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let day = variant_test_raw(color(0.7, 0.8, 0.9));
        let mut context = variant_test_context(root.clone(), day.clone());
        context.selected_index = 1;
        context.placements[0]
            .attrs
            .insert("lodParent".to_string(), "lodmall_nt".to_string());
        context.placements[1]
            .attrs
            .insert("lodParent".to_string(), "lodmall_dt".to_string());
        context.placements.push(variant_test_placement(
            "lodmall_nt",
            color(10.0, 20.0, 30.0),
        ));
        context.states.push(ElementState::default());
        context.definitions.insert(
            "lodmall_nt".to_string(),
            variant_test_definition("lodmall_nt", "20", "6"),
        );
        let night_lod = variant_test_raw(color(0.1, 0.2, 0.3));
        context.byte_overrides.insert(
            asset_key("lodmall_nt", ".dff"),
            (
                "lodmall_nt.dff".to_string(),
                write_normalized_dff(&night_lod, "lodmall_nt").unwrap(),
            ),
        );
        let mut placements = context.placements.clone();
        let mut states = context.states.clone();
        let mut definitions = context.definitions.clone();
        let (progress, _) = mpsc::channel();

        let merged = run_day_night_merge(context, &progress);

        assert_eq!(merged.replacements.len(), 2, "{:#?}", merged.reviews);
        assert_eq!(merged.delete_indices, vec![0]);
        assert!(merged.reviews.is_empty());
        assert_eq!(
            merged.lod_assignment_updates,
            vec![(1, Some("lodmall_nt".to_string()), None)]
        );
        let lod_replacement = merged
            .replacements
            .iter()
            .find(|replacement| replacement.pair.target_index == 2)
            .unwrap();
        assert!(!lod_replacement.pair.delete_source);
        let written_lod = parse_dff_mesh(&lod_replacement.bytes);
        assert!(prelight_stream_matches_serialized(
            &day.prelit_colors,
            &written_lod.prelit_colors
        ));
        assert!(prelight_stream_matches_serialized(
            &night_lod.prelit_colors,
            &written_lod.night_prelit_colors
        ));

        for index in &merged.delete_indices {
            states[*index].deleted = true;
        }
        for (child_index, parent_id, _) in &merged.lod_assignment_updates {
            placements[*child_index]
                .attrs
                .insert("lodParent".to_string(), parent_id.clone().unwrap());
        }
        apply_successful_day_night_id_renames(
            &mut placements,
            &states,
            &mut definitions,
            &merged.replacements,
        );
        assert_eq!(placements[1].id, "mall");
        assert_eq!(placements[2].id, "lodmall");
        assert_eq!(
            placements[1].attrs.get("lodParent").map(String::as_str),
            Some("lodmall")
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn day_night_merge_reassigns_secondary_lod_references_to_retained_lod() {
        let root = std::env::temp_dir().join(format!(
            "eagle_day_night_reassign_lod_test_{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let day = variant_test_raw(color(0.7, 0.8, 0.9));
        let night = variant_test_raw(color(0.1, 0.2, 0.3));
        let mut context = variant_test_context(root.clone(), day.clone());
        context.placements[0]
            .attrs
            .insert("lodParent".to_string(), "lodmall_nt".to_string());
        context.placements[1]
            .attrs
            .insert("lodParent".to_string(), "lodmall_dt".to_string());
        context.placements.push(variant_test_placement(
            "lodmall_nt",
            color(10.0, 20.0, 30.0),
        ));
        context.placements.push(variant_test_placement(
            "lodmall_dt",
            color(10.0, 20.0, 30.0),
        ));
        context.placements[2]
            .attrs
            .insert("uniqueID".to_string(), "500".to_string());
        context.placements[3]
            .attrs
            .insert("uniqueID".to_string(), "600".to_string());
        for id in ["annex_a", "annex_b"] {
            let mut placement = variant_test_placement(id, color(12.0, 20.0, 30.0));
            placement
                .attrs
                .insert("lodParent".to_string(), "lodmall_dt".to_string());
            placement
                .attrs
                .insert("uniqueID".to_string(), "600".to_string());
            context.placements.push(placement);
        }
        context
            .states
            .resize(context.placements.len(), ElementState::default());
        context.definitions.insert(
            "lodmall_nt".to_string(),
            variant_test_definition("lodmall_nt", "20", "6"),
        );
        context.definitions.insert(
            "lodmall_dt".to_string(),
            variant_test_definition("lodmall_dt", "6", "20"),
        );
        context.byte_overrides.insert(
            asset_key("lodmall_nt", ".dff"),
            (
                "lodmall_nt.dff".to_string(),
                write_normalized_dff(&night, "lodmall_nt").unwrap(),
            ),
        );
        context.byte_overrides.insert(
            asset_key("lodmall_dt", ".dff"),
            (
                "lodmall_dt.dff".to_string(),
                write_normalized_dff(&day, "lodmall_dt").unwrap(),
            ),
        );
        let mut placements = context.placements.clone();
        let mut states = context.states.clone();
        let mut definitions = context.definitions.clone();
        let (progress, _) = mpsc::channel();

        let merged = run_day_night_merge(context, &progress);

        assert_eq!(merged.replacements.len(), 2, "{:#?}", merged.reviews);
        assert_eq!(merged.delete_indices, vec![1, 3]);
        assert!(merged.reviews.is_empty());
        assert_eq!(
            merged.lod_assignment_updates,
            vec![
                (4, Some("lodmall_nt".to_string()), Some("500".to_string())),
                (5, Some("lodmall_nt".to_string()), Some("500".to_string())),
            ]
        );

        for (child_index, parent_id, parent_unique_id) in &merged.lod_assignment_updates {
            let child = &mut placements[*child_index];
            child
                .attrs
                .insert("lodParent".to_string(), parent_id.clone().unwrap());
            if let Some(parent_unique_id) = parent_unique_id {
                child
                    .attrs
                    .insert("uniqueID".to_string(), parent_unique_id.clone());
            }
        }
        for index in &merged.delete_indices {
            states[*index].deleted = true;
        }
        apply_successful_day_night_id_renames(
            &mut placements,
            &states,
            &mut definitions,
            &merged.replacements,
        );
        assert_eq!(placements[2].id, "lodmall");
        assert_eq!(
            placements[4].attrs.get("lodParent").map(String::as_str),
            Some("lodmall")
        );
        assert_eq!(
            placements[5].attrs.get("lodParent").map(String::as_str),
            Some("lodmall")
        );
        assert_eq!(
            placements[4].attrs.get("uniqueID").map(String::as_str),
            Some("500")
        );
        assert_eq!(
            placements[5].attrs.get("uniqueID").map(String::as_str),
            Some("500")
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn day_night_merge_continues_and_clears_assignment_when_both_lods_are_missing() {
        let root = std::env::temp_dir().join(format!(
            "eagle_day_night_both_lods_missing_test_{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let day = variant_test_raw(color(0.7, 0.8, 0.9));
        let mut context = variant_test_context(root.clone(), day);
        context.placements[0]
            .attrs
            .insert("lodParent".to_string(), "lodmall_nt".to_string());
        context.placements[1]
            .attrs
            .insert("lodParent".to_string(), "lodmall_dt".to_string());
        let mut placements = context.placements.clone();
        let mut states = context.states.clone();
        let mut definitions = context.definitions.clone();
        let (progress, _) = mpsc::channel();

        let merged = run_day_night_merge(context, &progress);

        assert_eq!(merged.replacements.len(), 1, "{:#?}", merged.reviews);
        assert_eq!(merged.delete_indices, vec![1]);
        assert_eq!(merged.lod_assignment_updates, vec![(0, None, None)]);
        assert_eq!(merged.reviews.len(), 1);
        assert!(merged.reviews[0].contains("neither day/night object has a usable assigned LOD"));

        for index in &merged.delete_indices {
            states[*index].deleted = true;
        }
        placements[0].attrs.remove("lodParent");
        apply_successful_day_night_id_renames(
            &mut placements,
            &states,
            &mut definitions,
            &merged.replacements,
        );
        assert_eq!(placements[0].id, "mall");
        assert!(!placements[0].attrs.contains_key("lodParent"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn day_night_worker_preserves_an_incompatible_lod_without_blocking_detail_merge() {
        let root = std::env::temp_dir().join(format!(
            "eagle_day_night_lod_review_test_{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let day = variant_test_raw(color(0.7, 0.8, 0.9));
        let mut context = variant_test_context(root.clone(), day.clone());
        context.placements[0]
            .attrs
            .insert("lodParent".to_string(), "lodmall_nt".to_string());
        context.placements[1]
            .attrs
            .insert("lodParent".to_string(), "lodmall_dt".to_string());
        context.placements.push(variant_test_placement(
            "lodmall_nt",
            color(10.0, 20.0, 30.0),
        ));
        context.placements.push(variant_test_placement(
            "lodmall_dt",
            color(10.0, 20.0, 30.0),
        ));
        context.states.resize(4, ElementState::default());
        context.definitions.insert(
            "lodmall_nt".to_string(),
            variant_test_definition("lodmall_nt", "20", "6"),
        );
        context.definitions.insert(
            "lodmall_dt".to_string(),
            variant_test_definition("lodmall_dt", "6", "20"),
        );
        let night_lod = variant_test_raw(color(0.1, 0.2, 0.3));
        let mut incompatible_day_lod = variant_test_raw(color(0.7, 0.8, 0.9));
        incompatible_day_lod.vertices[0].x += 5.0;
        context.byte_overrides.insert(
            asset_key("lodmall_nt", ".dff"),
            (
                "lodmall_nt.dff".to_string(),
                write_normalized_dff(&night_lod, "lodmall_nt").unwrap(),
            ),
        );
        context.byte_overrides.insert(
            asset_key("lodmall_dt", ".dff"),
            (
                "lodmall_dt.dff".to_string(),
                write_normalized_dff(&incompatible_day_lod, "lodmall_dt").unwrap(),
            ),
        );
        let (progress, _) = mpsc::channel();

        let reviewed = run_day_night_merge(context, &progress);

        assert!(reviewed.replacements.is_empty());
        assert!(reviewed.delete_indices.is_empty());
        assert_eq!(reviewed.reviews.len(), 1);
        assert!(reviewed.reviews[0].contains("lodmall_nt + lodmall_dt"));
        assert!(reviewed.override_available);

        let mut override_context = variant_test_context(root.clone(), day);
        override_context.placements[0]
            .attrs
            .insert("lodParent".to_string(), "lodmall_nt".to_string());
        override_context.placements[1]
            .attrs
            .insert("lodParent".to_string(), "lodmall_dt".to_string());
        override_context.placements.push(variant_test_placement(
            "lodmall_nt",
            color(10.0, 20.0, 30.0),
        ));
        override_context.placements.push(variant_test_placement(
            "lodmall_dt",
            color(10.0, 20.0, 30.0),
        ));
        override_context.states.resize(4, ElementState::default());
        override_context.definitions.insert(
            "lodmall_nt".to_string(),
            variant_test_definition("lodmall_nt", "20", "6"),
        );
        override_context.definitions.insert(
            "lodmall_dt".to_string(),
            variant_test_definition("lodmall_dt", "6", "20"),
        );
        override_context.byte_overrides.insert(
            asset_key("lodmall_nt", ".dff"),
            (
                "lodmall_nt.dff".to_string(),
                write_normalized_dff(&night_lod, "lodmall_nt").unwrap(),
            ),
        );
        override_context.byte_overrides.insert(
            asset_key("lodmall_dt", ".dff"),
            (
                "lodmall_dt.dff".to_string(),
                write_normalized_dff(&incompatible_day_lod, "lodmall_dt").unwrap(),
            ),
        );
        override_context.force_review_override = true;

        let overridden = run_day_night_merge(override_context, &progress);

        assert_eq!(
            overridden.replacements.len(),
            2,
            "unexpected override review: {:?}",
            overridden.reviews
        );
        assert_eq!(overridden.delete_indices, vec![1, 3]);
        assert!(overridden.reviews.is_empty());
        assert!(
            overridden.replacements[1]
                .notes
                .iter()
                .any(|note| note.contains("explicit second-click override"))
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn day_night_worker_merges_a_near_matching_assigned_lod() {
        let root = std::env::temp_dir().join(format!(
            "eagle_day_night_lod_fallback_test_{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let detail_day = variant_test_raw(color(0.7, 0.8, 0.9));
        let mut context = variant_test_context(root.clone(), detail_day);
        context.placements[0]
            .attrs
            .insert("lodParent".to_string(), "lodmall_nt".to_string());
        context.placements[1]
            .attrs
            .insert("lodParent".to_string(), "lodmall_dt".to_string());
        context.placements.push(variant_test_placement(
            "lodmall_nt",
            color(10.0, 20.0, 30.0),
        ));
        context.placements.push(variant_test_placement(
            "lodmall_dt",
            color(10.0, 20.0, 30.0),
        ));
        context
            .placements
            .push(variant_test_placement("lodmall", color(100.0, 0.0, 0.0)));
        context.states.resize(5, ElementState::default());
        context.definitions.insert(
            "lodmall_nt".to_string(),
            variant_test_definition("lodmall_nt", "20", "6"),
        );
        context.definitions.insert(
            "lodmall_dt".to_string(),
            variant_test_definition("lodmall_dt", "6", "20"),
        );
        let night_lod = near_matching_lod_test_raw(0.04);
        let mut day_lod = near_matching_lod_test_raw(0.02);
        day_lod.triangles[8] = Tri {
            a: 9,
            b: 10,
            c: 1,
            material: 0,
        };
        day_lod.triangles[9] = Tri {
            a: 9,
            b: 1,
            c: 0,
            material: 0,
        };
        context.byte_overrides.insert(
            asset_key("lodmall_nt", ".dff"),
            (
                "lodmall_nt.dff".to_string(),
                write_normalized_dff(&night_lod, "lodmall_nt").unwrap(),
            ),
        );
        context.byte_overrides.insert(
            asset_key("lodmall_dt", ".dff"),
            (
                "lodmall_dt.dff".to_string(),
                write_normalized_dff(&day_lod, "lodmall_dt").unwrap(),
            ),
        );
        let mut placements = context.placements.clone();
        let mut states = context.states.clone();
        let mut definitions = context.definitions.clone();
        let (progress, _) = mpsc::channel();

        let merged = run_day_night_merge(context, &progress);

        assert_eq!(merged.replacements.len(), 2, "{:#?}", merged.reviews);
        assert_eq!(merged.delete_indices, vec![1, 3]);
        assert!(merged.reviews.is_empty());
        assert!(
            merged.replacements[1]
                .notes
                .iter()
                .any(|note| note.contains("near-matching LOD fallback"))
        );
        for index in &merged.delete_indices {
            states[*index].deleted = true;
        }
        let renamed = apply_successful_day_night_id_renames(
            &mut placements,
            &states,
            &mut definitions,
            &merged.replacements,
        );
        assert_eq!(
            renamed,
            vec![
                ("mall_nt".to_string(), "mall".to_string()),
                ("lodmall_nt".to_string(), "lodmall_A".to_string()),
            ]
        );
        assert_eq!(placements[0].id, "mall");
        assert_eq!(placements[2].id, "lodmall_A");
        assert_eq!(placements[4].id, "lodmall");
        assert_eq!(
            placements[0].attrs.get("lodParent").map(String::as_str),
            Some("lodmall_A")
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn prelight_clipboard_resolves_and_deduplicates_assigned_lods() {
        let mut child_a = variant_test_placement("bridge_a", color(0.0, 0.0, 0.0));
        child_a
            .attrs
            .insert("lodParent".to_string(), "lod_bridge".to_string());
        let mut child_b = variant_test_placement("bridge_b", color(10.0, 0.0, 0.0));
        child_b
            .attrs
            .insert("lodParent".to_string(), "LOD_BRIDGE".to_string());
        let lod = variant_test_placement("lod_bridge", color(5.0, 0.0, 0.0));
        let placements = vec![child_a, child_b, lod];
        let states = vec![ElementState::default(); placements.len()];

        let (indices, issues) = assigned_lod_indices(&placements, &states, [0usize, 1, 0]);

        assert_eq!(indices, vec![2]);
        assert!(issues.is_empty());
    }

    #[test]
    fn prelight_clipboard_skips_missing_or_deleted_assigned_lods() {
        let mut child = variant_test_placement("bridge", color(0.0, 0.0, 0.0));
        child
            .attrs
            .insert("lodParent".to_string(), "lod_bridge".to_string());
        let lod = variant_test_placement("lod_bridge", color(0.0, 0.0, 0.0));
        let placements = vec![child, lod];
        let states = vec![
            ElementState::default(),
            ElementState {
                deleted: true,
                hidden: false,
            },
        ];

        let (indices, issues) = assigned_lod_indices(&placements, &states, [0usize]);

        assert!(indices.is_empty());
        assert_eq!(issues.len(), 1);
        assert!(issues[0].contains("missing LOD"));
    }

    #[test]
    fn prelight_clipboard_uses_nearest_repeated_lod_instance() {
        let mut child = variant_test_placement("bridge", color(99.0, 0.0, 0.0));
        child
            .attrs
            .insert("lodParent".to_string(), "lod_bridge".to_string());
        let placements = vec![
            child,
            variant_test_placement("lod_bridge", color(0.0, 0.0, 0.0)),
            variant_test_placement("lod_bridge", color(100.0, 0.0, 0.0)),
        ];
        let states = vec![ElementState::default(); placements.len()];

        let (indices, issues) = assigned_lod_indices(&placements, &states, [0usize]);

        assert_eq!(indices, vec![2]);
        assert!(issues.is_empty());
    }

    fn surface_equivalent_detail_test_raw(alternate_topology: bool, night: bool) -> RawMesh {
        let (vertices, triangles) = if alternate_topology {
            (
                vec![
                    color(0.0, 0.0, 0.0),
                    color(1.0, 0.0, 0.0),
                    color(0.0, 1.0, 0.0),
                    color(1.0, 0.0, 0.0),
                    color(1.0, 1.0, 0.0),
                    color(0.0, 1.0, 0.0),
                ],
                vec![
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
                        material: 0,
                    },
                ],
            )
        } else {
            (
                vec![
                    color(0.0, 0.0, 0.0),
                    color(1.0, 0.0, 0.0),
                    color(1.0, 1.0, 0.0),
                    color(0.0, 1.0, 0.0),
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
            )
        };
        let prelit_colors = vertices
            .iter()
            .map(|vertex| {
                let offset = if night { 0.25 } else { 0.0 };
                color(vertex.x * 0.2 + offset, vertex.y * 0.2 + offset, offset)
            })
            .collect();
        RawMesh {
            normals: vec![color(0.0, 0.0, 1.0); vertices.len()],
            uvs: vertices
                .iter()
                .map(|vertex| V2 {
                    u: vertex.x,
                    v: vertex.y,
                })
                .collect(),
            prelit_colors,
            prelit_alphas: vec![1.0; vertices.len()],
            vertices,
            triangles,
            materials: vec![RawMaterial {
                color: neutral_vertex_color(),
                alpha: 1.0,
                ambient: 1.0,
                specular: 0.0,
                diffuse: 1.0,
            }],
            material_textures: vec!["road".to_string()],
            ..RawMesh::default()
        }
    }

    #[test]
    fn detail_merge_accepts_surface_equivalent_alternate_topology() {
        let day = surface_equivalent_detail_test_raw(false, false);
        let night = surface_equivalent_detail_test_raw(true, true);
        assert!(analyze_day_night_raw_meshes(&day, &night).is_err());

        let (merged, notes) =
            merge_surface_equivalent_day_night_details(&day, &night, DayNightVariant::Day).unwrap();

        assert_eq!(merged.vertices, day.vertices);
        assert_eq!(merged.triangles, day.triangles);
        assert_eq!(merged.prelit_colors, day.prelit_colors);
        assert_eq!(
            merged.night_prelit_colors,
            vec![
                color(0.25, 0.25, 0.25),
                color(0.45, 0.25, 0.25),
                color(0.45, 0.45, 0.25),
                color(0.25, 0.45, 0.25),
            ]
        );
        assert!(
            notes
                .iter()
                .any(|note| note.contains("surface-equivalent detail fallback"))
        );
        let bytes = write_normalized_dff(&merged, "road").unwrap();
        verify_serialized_day_night_prelight(&merged, &parse_dff_mesh(&bytes), true).unwrap();
    }

    #[test]
    fn detail_surface_fallback_preserves_and_reports_uv_differences() {
        let day = surface_equivalent_detail_test_raw(false, false);
        let mut night = surface_equivalent_detail_test_raw(true, true);
        night.uvs[4].u += 0.1;

        let (merged, notes) =
            merge_surface_equivalent_day_night_details(&day, &night, DayNightVariant::Day).unwrap();

        assert_eq!(merged.uvs, day.uvs);
        assert!(notes.iter().any(|note| note.contains("primary UVs differ")));
    }

    fn near_matching_lod_test_raw(prelight_scale: f32) -> RawMesh {
        let mut vertices = vec![color(0.0, 0.0, 0.0)];
        for index in 0..10 {
            let angle = index as f32 / 10.0 * std::f32::consts::TAU;
            vertices.push(color(angle.cos(), angle.sin(), 0.0));
        }
        let triangles = (0..10)
            .map(|index| Tri {
                a: 0,
                b: index + 1,
                c: ((index + 1) % 10) + 1,
                material: 0,
            })
            .collect::<Vec<_>>();
        RawMesh {
            normals: vec![color(0.0, 0.0, 1.0); vertices.len()],
            uvs: vertices
                .iter()
                .map(|vertex| V2 {
                    u: vertex.x * 0.5 + 0.5,
                    v: vertex.y * 0.5 + 0.5,
                })
                .collect(),
            prelit_colors: (0..vertices.len())
                .map(|index| {
                    let value = (index as f32 + 1.0) * prelight_scale;
                    color(value, value * 0.5, value * 0.25)
                })
                .collect(),
            prelit_alphas: vec![1.0; vertices.len()],
            vertices,
            triangles,
            materials: vec![RawMaterial {
                color: neutral_vertex_color(),
                alpha: 1.0,
                ambient: 1.0,
                specular: 0.0,
                diffuse: 1.0,
            }],
            material_textures: vec!["lod_bridge".to_string()],
            ..RawMesh::default()
        }
    }

    #[test]
    fn near_matching_lod_blends_lighting_across_a_small_topology_mismatch() {
        let day = near_matching_lod_test_raw(0.02);
        let mut night = near_matching_lod_test_raw(0.04);
        night.triangles[8] = Tri {
            a: 9,
            b: 10,
            c: 1,
            material: 0,
        };
        night.triangles[9] = Tri {
            a: 9,
            b: 1,
            c: 0,
            material: 0,
        };
        let expected_night = night.prelit_colors.clone();

        let (merged, notes) =
            merge_near_matching_day_night_lods(&day, &night, DayNightVariant::Day).unwrap();

        assert_eq!(merged.vertices, day.vertices);
        assert_eq!(merged.triangles, day.triangles);
        assert_eq!(merged.prelit_colors, day.prelit_colors);
        assert_eq!(merged.night_prelit_colors, expected_night);
        assert!(notes.iter().any(|note| note.contains("matched 8/10")));
        assert!(
            notes
                .iter()
                .any(|note| note.contains("blended 1 unmatched"))
        );

        let bytes = write_normalized_dff(&merged, "lod_bridge").unwrap();
        verify_serialized_day_night_prelight(&merged, &parse_dff_mesh(&bytes), true).unwrap();
    }

    #[test]
    fn near_matching_lod_surface_matches_zero_canonical_overlap() {
        let day = surface_equivalent_detail_test_raw(false, false);
        let night = surface_equivalent_detail_test_raw(true, true);

        let (merged, notes) =
            merge_near_matching_day_night_lods(&day, &night, DayNightVariant::Day).unwrap();

        assert_eq!(merged.vertices, night.vertices);
        assert_eq!(merged.triangles, night.triangles);
        assert_eq!(merged.night_prelit_colors, night.prelit_colors);
        assert!(notes.iter().any(|note| {
            note.contains("LOD overlap is only 0.0%")
                && note.contains("alternate topology was accepted")
        }));
    }

    #[test]
    fn near_matching_lod_uses_bounded_rough_transfer_for_related_geometry() {
        let mut day = near_matching_lod_test_raw(0.02);
        let mut night = near_matching_lod_test_raw(0.04);
        for raw in [&mut day, &mut night] {
            for vertex in &mut raw.vertices {
                vertex.x *= 100.0;
                vertex.y *= 100.0;
            }
        }
        for index in 4..=6 {
            night.vertices[index].x += 1.0;
        }

        let (merged, notes) =
            merge_near_matching_day_night_lods(&day, &night, DayNightVariant::Day).unwrap();

        assert_eq!(merged.vertices.len(), day.vertices.len());
        assert!(notes.iter().any(|note| {
            note.contains("exact surface matching was unavailable")
                && note.contains("bounded rough LOD lighting transfer")
        }));
    }

    #[test]
    fn near_matching_lod_keeps_the_richer_secondary_geometry() {
        let day = near_matching_lod_test_raw(0.02);
        let mut night = near_matching_lod_test_raw(0.04);
        night.vertices.push(color(1.01, 0.0, 0.0));
        night.normals.push(color(0.0, 0.0, 1.0));
        night.uvs.push(V2 { u: 1.0, v: 0.5 });
        night.prelit_colors.push(color(0.9, 0.4, 0.2));
        night.prelit_alphas.push(1.0);
        night.triangles.push(Tri {
            a: 0,
            b: 1,
            c: 11,
            material: 0,
        });

        let (merged, notes) =
            merge_near_matching_day_night_lods(&day, &night, DayNightVariant::Day).unwrap();

        assert_eq!(merged.vertices.len(), 12);
        assert_eq!(merged.triangles.len(), 11);
        assert_eq!(merged.night_prelit_colors, night.prelit_colors);
        assert_eq!(merged.prelit_colors.len(), merged.vertices.len());
        assert!(notes.iter().any(|note| note.contains("secondary (richer)")));
        assert!(
            notes
                .iter()
                .any(|note| note.contains("blended 1 unmatched"))
        );
    }

    #[test]
    fn near_matching_lod_rejects_low_geometry_overlap() {
        let day = near_matching_lod_test_raw(0.02);
        let mut unrelated = near_matching_lod_test_raw(0.04);
        for vertex in &mut unrelated.vertices {
            vertex.x += 20.0;
        }

        let error = merge_near_matching_day_night_lods(&day, &unrelated, DayNightVariant::Day)
            .err()
            .unwrap();

        assert!(error.contains("LOD overlap is only"));
    }
}

fn render_mesh_prelight_samples(
    mesh: &RenderMesh,
    mode: BakeLightMode,
) -> Vec<PrelightClipboardSample> {
    let mut samples = Vec::new();
    for part in &mesh.parts {
        for vertices in part.cpu_vertices.chunks(3) {
            let triangle =
                (vertices.len() == 3).then(|| [vertices[0].pos, vertices[1].pos, vertices[2].pos]);
            samples.extend(vertices.iter().map(|vertex| PrelightClipboardSample {
                geometry: PrelightSampleGeometry {
                    pos: vertex.pos,
                    normal: Some(vertex.normal),
                    uv: Some(vertex.uv),
                    triangle,
                    material: Some(part.material_index),
                },
                color: serialized_prelight_from_runtime(part, vertex_bake_color(vertex, mode)),
            }));
        }
    }
    samples
}

fn placement_prelight_samples(
    app: &AppState,
    indices: impl IntoIterator<Item = usize>,
    mode: BakeLightMode,
    deduplicate_meshes: bool,
) -> Vec<PrelightClipboardSample> {
    let mut samples = Vec::new();
    let mut seen_meshes = HashSet::new();
    for idx in indices {
        let Some(placement) = app.placements.get(idx) else {
            continue;
        };
        let mesh_key = placement_mesh_key(placement, &app.definitions);
        if deduplicate_meshes && !seen_meshes.insert(mesh_key.clone()) {
            continue;
        }
        let Some(mesh) = app.meshes.get(&mesh_key) else {
            continue;
        };
        samples.extend(render_mesh_prelight_samples(mesh, mode));
    }
    samples
}

fn assigned_lod_indices(
    placements: &[Placement],
    states: &[ElementState],
    children: impl IntoIterator<Item = usize>,
) -> (Vec<usize>, Vec<String>) {
    let mut indices = BTreeSet::new();
    let mut issues = Vec::new();
    for child_index in children {
        match resolve_assigned_lod(placements, states, child_index) {
            Ok(Some(lod_index)) => {
                indices.insert(lod_index);
            }
            Ok(None) => {}
            Err(error) => issues.push(error),
        }
    }
    (indices.into_iter().collect(), issues)
}

#[derive(Default)]
struct AppliedPrelightStats {
    applied: usize,
    exact: usize,
    distance_total: f32,
    changed_mesh_keys: BTreeSet<String>,
}

fn apply_prelight_updates(
    app: &mut AppState,
    updates: Vec<(String, usize, usize, V3, f32)>,
    mode: BakeLightMode,
) -> AppliedPrelightStats {
    let mut stats = AppliedPrelightStats::default();
    for (mesh_key, part_idx, vertex_idx, color, dist2) in updates {
        if let Some(vertex) = app
            .meshes
            .get_mut(&mesh_key)
            .and_then(|mesh| mesh.parts.get_mut(part_idx))
            .and_then(|part| part.cpu_vertices.get_mut(vertex_idx))
        {
            set_vertex_bake_color(vertex, mode, color);
            apply_vertex_bake_display(vertex, mode);
            if dist2 <= 0.000001 {
                stats.exact += 1;
            }
            stats.distance_total += dist2.sqrt();
            stats.applied += 1;
            stats.changed_mesh_keys.insert(mesh_key);
        }
    }
    stats
}

pub(crate) fn copy_selected_prelight(app: &mut AppState) {
    let selected = selected_live_indices(app);
    if selected.is_empty() {
        app.status_message = "Select one or more elements before copying prelight".to_string();
        return;
    }
    let mode = app.bake_settings.light_mode;
    let samples = placement_prelight_samples(app, selected.iter().copied(), mode, false);
    let secondary_samples = if mode == BakeLightMode::Both {
        placement_prelight_samples(app, selected.iter().copied(), BakeLightMode::Night, false)
    } else {
        Vec::new()
    };
    let (lod_indices, lod_issues) =
        assigned_lod_indices(&app.placements, &app.element_states, selected);
    let lod_samples = placement_prelight_samples(app, lod_indices.iter().copied(), mode, true);
    let lod_secondary_samples = if mode == BakeLightMode::Both {
        placement_prelight_samples(app, lod_indices.iter().copied(), BakeLightMode::Night, true)
    } else {
        Vec::new()
    };
    if samples.is_empty()
        && secondary_samples.is_empty()
        && lod_samples.is_empty()
        && lod_secondary_samples.is_empty()
    {
        app.status_message = "Selected elements have no prelight vertices to copy".to_string();
        return;
    }
    let count = samples.len();
    let lod_count = lod_samples.len();
    app.prelight_clipboard = Some(PrelightClipboard {
        mode,
        samples,
        secondary_samples,
        lod_samples,
        lod_secondary_samples,
    });
    let lod_summary = if lod_count > 0 {
        format!(" and {lod_count} assigned-LOD vertices")
    } else {
        String::new()
    };
    let issue_summary = if lod_issues.is_empty() {
        String::new()
    } else {
        format!(
            "; skipped {} unresolved LOD assignment(s)",
            lod_issues.len()
        )
    };
    app.status_message = format!(
        "Copied {count} local {} prelight vertex/vertices{lod_summary}{issue_summary}",
        bake_light_mode_label(mode)
    );
}

pub(crate) fn paste_selected_prelight(app: &mut AppState) {
    let Some(clipboard) = app.prelight_clipboard.clone() else {
        app.status_message = "Copy prelight before pasting".to_string();
        return;
    };
    if clipboard.samples.is_empty()
        && clipboard.secondary_samples.is_empty()
        && clipboard.lod_samples.is_empty()
        && clipboard.lod_secondary_samples.is_empty()
    {
        app.status_message = "Copied prelight is empty".to_string();
        return;
    }
    let selected = selected_live_indices(app);
    if selected.is_empty() {
        app.status_message = "Select one or more elements before pasting prelight".to_string();
        return;
    }
    let mode = app.bake_settings.light_mode;
    let primary_mode = if clipboard.mode == BakeLightMode::Both {
        BakeLightMode::Day
    } else {
        mode
    };
    let (detail_updates, detail_exact_order) = if clipboard.samples.is_empty() {
        (Vec::new(), true)
    } else {
        collect_prelight_updates(app, selected.clone(), &clipboard.samples)
    };
    let (detail_secondary_updates, detail_secondary_exact_order) =
        if clipboard.secondary_samples.is_empty() {
            (Vec::new(), true)
        } else {
            collect_prelight_updates(app, selected.clone(), &clipboard.secondary_samples)
        };
    let (lod_indices, lod_issues) =
        assigned_lod_indices(&app.placements, &app.element_states, selected);
    let (lod_updates, lod_exact_order) =
        if clipboard.lod_samples.is_empty() || lod_indices.is_empty() {
            (Vec::new(), true)
        } else {
            collect_prelight_updates(app, lod_indices.clone(), &clipboard.lod_samples)
        };
    let (lod_secondary_updates, lod_secondary_exact_order) =
        if clipboard.lod_secondary_samples.is_empty() || lod_indices.is_empty() {
            (Vec::new(), true)
        } else {
            collect_prelight_updates(app, lod_indices.clone(), &clipboard.lod_secondary_samples)
        };
    let detail_stats = apply_prelight_updates(app, detail_updates, primary_mode);
    let lod_stats = apply_prelight_updates(app, lod_updates, primary_mode);
    let detail_secondary_stats =
        apply_prelight_updates(app, detail_secondary_updates, BakeLightMode::Night);
    let lod_secondary_stats =
        apply_prelight_updates(app, lod_secondary_updates, BakeLightMode::Night);
    let applied = detail_stats.applied
        + lod_stats.applied
        + detail_secondary_stats.applied
        + lod_secondary_stats.applied;
    if applied == 0 {
        app.status_message = "No selected prelight vertices were pasted".to_string();
        return;
    }
    let changed_mesh_keys = detail_stats
        .changed_mesh_keys
        .union(&lod_stats.changed_mesh_keys)
        .cloned()
        .chain(detail_secondary_stats.changed_mesh_keys.iter().cloned())
        .chain(lod_secondary_stats.changed_mesh_keys.iter().cloned())
        .collect::<BTreeSet<_>>();
    for mesh_key in &changed_mesh_keys {
        if let Some(mesh) = app.meshes.get_mut(mesh_key) {
            apply_bake_light_mode_to_mesh(mesh, mode);
        }
    }
    rebuild_mesh_part_lists(&mut app.meshes);
    rebuild_render_cells(app);
    app.vertex_paint_dirty_meshes
        .extend(changed_mesh_keys.iter().cloned());
    let queued = queue_vertex_lighting_meshes(app, changed_mesh_keys.iter());
    let exact = detail_stats.exact
        + lod_stats.exact
        + detail_secondary_stats.exact
        + lod_secondary_stats.exact;
    let avg_distance = (detail_stats.distance_total
        + lod_stats.distance_total
        + detail_secondary_stats.distance_total
        + lod_secondary_stats.distance_total)
        / applied as f32;
    let method = if detail_exact_order
        && lod_exact_order
        && detail_secondary_exact_order
        && lod_secondary_exact_order
    {
        "exact ordered"
    } else {
        "geometry-aware"
    };
    let lod_applied = lod_stats.applied + lod_secondary_stats.applied;
    let lod_summary = if lod_applied > 0 {
        format!(", including {lod_applied} assigned-LOD channel update(s)")
    } else if (!clipboard.lod_samples.is_empty() || !clipboard.lod_secondary_samples.is_empty())
        && lod_indices.is_empty()
    {
        "; copied LOD lighting was not applied because the target has no resolvable assigned LOD"
            .to_string()
    } else if clipboard.lod_samples.is_empty()
        && clipboard.lod_secondary_samples.is_empty()
        && !lod_indices.is_empty()
    {
        "; target LOD was unchanged because the copied object had no assigned LOD lighting"
            .to_string()
    } else {
        String::new()
    };
    let issue_summary = if lod_issues.is_empty() {
        String::new()
    } else {
        format!(
            "; skipped {} unresolved target LOD assignment(s)",
            lod_issues.len()
        )
    };
    let pasted_mode = if clipboard.mode == BakeLightMode::Both {
        BakeLightMode::Both
    } else {
        mode
    };
    let channel_summary = if clipboard.mode == BakeLightMode::Both {
        " across distinct day and night channels"
    } else {
        ""
    };
    app.status_message = format!(
        "Pasted {applied} {} prelight channel update(s){channel_summary}{lod_summary} from copied {} prelight using {method} transfer ({exact} exact, avg local distance {:.3}){issue_summary}; queued {queued} DFF(s)",
        bake_light_mode_label(pasted_mode),
        bake_light_mode_label(clipboard.mode),
        avg_distance
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DayNightVariant {
    Day,
    Night,
}

impl DayNightVariant {
    fn label(self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::Night => "night",
        }
    }

    fn opposite(self) -> Self {
        match self {
            Self::Day => Self::Night,
            Self::Night => Self::Day,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DayNightVariantName {
    base: String,
    variant: DayNightVariant,
}

fn day_night_variant_name(value: &str) -> Option<DayNightVariantName> {
    let file_name = Path::new(value.trim())
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(value.trim());
    let stem = if Path::new(file_name)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("dff"))
    {
        Path::new(file_name)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or(file_name)
    } else {
        file_name
    }
    .to_ascii_lowercase();
    for (suffix, variant) in [
        ("_nt", DayNightVariant::Night),
        ("_dt", DayNightVariant::Day),
        ("_dy", DayNightVariant::Day),
    ] {
        if let Some(base) = stem.strip_suffix(suffix).filter(|base| !base.is_empty()) {
            return Some(DayNightVariantName {
                base: base.to_string(),
                variant,
            });
        }
    }
    None
}

fn day_night_base_id(value: &str) -> Option<String> {
    let value = value.trim();
    let lower_value = value.to_ascii_lowercase();
    for suffix in ["_nt", "_dt", "_dy"] {
        if lower_value.ends_with(suffix) && value.len() > suffix.len() {
            return Some(value[..value.len() - suffix.len()].to_string());
        }
    }
    None
}

fn placement_day_night_variant(placement: &Placement) -> Option<DayNightVariantName> {
    day_night_variant_name(&placement.id).or_else(|| day_night_variant_name(&placement.dff))
}

pub(crate) fn placement_has_day_night_variant_suffix(placement: &Placement) -> bool {
    placement_day_night_variant(placement).is_some()
}

pub(crate) fn selected_day_night_merge_available(app: &AppState) -> bool {
    let selected = selected_live_indices(app);
    selected.len() == 2
        || (selected.len() == 1
            && app
                .placements
                .get(selected[0])
                .is_some_and(placement_has_day_night_variant_suffix))
}

fn selected_explicit_day_night_pair(app: &AppState) -> Option<(usize, usize)> {
    let selected = selected_live_indices_in_selection_order(app);
    let [first, second] = selected.as_slice() else {
        return None;
    };
    let target = if app.selected == *first || app.selected == *second {
        app.selected
    } else {
        *second
    };
    let source = if target == *first { *second } else { *first };
    Some((target, source))
}

fn day_night_merge_live(states: &[ElementState], index: usize) -> bool {
    !states.get(index).is_some_and(|state| state.deleted)
}

fn day_night_position_distance2(a: V3, b: V3) -> f32 {
    local_vertex_distance2(a, b)
}

fn find_day_night_counterpart(
    placements: &[Placement],
    states: &[ElementState],
    selected: usize,
    tolerance: f32,
) -> Result<usize, String> {
    let placement = placements
        .get(selected)
        .ok_or_else(|| "The selected object no longer exists".to_string())?;
    let variant = placement_day_night_variant(placement)
        .ok_or_else(|| format!("{} does not end in _nt, _dt, or _dy", placement.id.trim()))?;
    let lod_ids = collect_lod_ids(placements);
    let selected_is_lod = placement_is_lod(placement, &lod_ids);
    let tolerance2 = tolerance.max(0.0).powi(2);
    let mut candidates = placements
        .iter()
        .enumerate()
        .filter(|(index, candidate)| {
            *index != selected
                && day_night_merge_live(states, *index)
                && placement_is_lod(candidate, &lod_ids) == selected_is_lod
                && placement_day_night_variant(candidate).is_some_and(|candidate_variant| {
                    candidate_variant.base == variant.base
                        && candidate_variant.variant == variant.variant.opposite()
                })
                && day_night_position_distance2(placement.pos, candidate.pos) <= tolerance2
        })
        .map(|(index, candidate)| {
            (
                index,
                day_night_position_distance2(placement.pos, candidate.pos),
            )
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    match candidates.as_slice() {
        [] => Err(format!(
            "No matching {} variant for {} was found within {:.6} units",
            variant.variant.opposite().label(),
            placement.id,
            tolerance
        )),
        [(index, _)] => Ok(*index),
        _ => Err(format!(
            "{} matching {} variants for {} were found within {:.6} units; the pair is ambiguous",
            candidates.len(),
            variant.variant.opposite().label(),
            placement.id,
            tolerance
        )),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DayNightMergeOverrideKey {
    first_index: usize,
    second_index: usize,
    first_id: String,
    second_id: String,
}

fn day_night_pair_override_key(
    placements: &[Placement],
    first: usize,
    second: usize,
) -> Option<DayNightMergeOverrideKey> {
    if first == second {
        return None;
    }
    let mut pair = [
        (first, placements.get(first)?.id.trim().to_ascii_lowercase()),
        (
            second,
            placements.get(second)?.id.trim().to_ascii_lowercase(),
        ),
    ];
    pair.sort_by_key(|(index, _)| *index);
    Some(DayNightMergeOverrideKey {
        first_index: pair[0].0,
        second_index: pair[1].0,
        first_id: pair[0].1.clone(),
        second_id: pair[1].1.clone(),
    })
}

fn day_night_merge_override_key(
    placements: &[Placement],
    states: &[ElementState],
    selected: usize,
    tolerance: f32,
) -> Option<DayNightMergeOverrideKey> {
    let counterpart = find_day_night_counterpart(placements, states, selected, tolerance).ok()?;
    day_night_pair_override_key(placements, selected, counterpart)
}

pub(crate) fn selected_day_night_merge_override_ready(app: &AppState) -> bool {
    let selected = selected_live_indices_in_selection_order(app);
    let key = match selected.as_slice() {
        [selected] => day_night_merge_override_key(
            &app.placements,
            &app.element_states,
            *selected,
            app.bake_settings.day_night_merge_tolerance,
        ),
        [_, _] => selected_explicit_day_night_pair(app).and_then(|(target, source)| {
            day_night_pair_override_key(&app.placements, target, source)
        }),
        _ => None,
    };
    key.as_ref()
        .is_some_and(|key| app.day_night_merge_override.as_ref() == Some(key))
}

fn resolve_assigned_lod(
    placements: &[Placement],
    states: &[ElementState],
    child_index: usize,
) -> Result<Option<usize>, String> {
    let Some(child) = placements.get(child_index) else {
        return Err("A day/night object disappeared while resolving its LOD".to_string());
    };
    let Some(parent) = child
        .attrs
        .get("lodParent")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("self"))
    else {
        return Ok(None);
    };
    let candidates = placements
        .iter()
        .enumerate()
        .filter(|(index, placement)| {
            day_night_merge_live(states, *index) && placement.id.eq_ignore_ascii_case(parent)
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return Err(format!("{} references missing LOD {parent}", child.id));
    }
    if candidates.len() == 1 {
        return Ok(candidates.first().copied());
    }
    if let Some(unique_id) = child
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
        return Ok(Some(index));
    }
    Ok(candidates.into_iter().min_by(|a, b| {
        day_night_position_distance2(placements[*a].pos, child.pos)
            .total_cmp(&day_night_position_distance2(placements[*b].pos, child.pos))
    }))
}

fn filtered_day_night_placement_attrs(placement: &Placement) -> BTreeMap<String, String> {
    const IGNORED: [&str; 9] = [
        "id",
        "posX",
        "posY",
        "posZ",
        "rotX",
        "rotY",
        "rotZ",
        "lodParent",
        "uniqueID",
    ];
    placement
        .attrs
        .iter()
        .filter(|(key, _)| {
            !IGNORED
                .iter()
                .any(|ignored| key.eq_ignore_ascii_case(ignored))
        })
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

fn filtered_day_night_definition_attrs(
    definition: Option<&Definition>,
) -> Option<BTreeMap<String, String>> {
    const IGNORED: [&str; 6] = ["id", "dff", "timeIn", "timeOut", "source", "zone"];
    definition.map(|definition| {
        definition
            .attrs
            .iter()
            .filter(|(key, _)| {
                !IGNORED
                    .iter()
                    .any(|ignored| key.eq_ignore_ascii_case(ignored))
            })
            .map(|(key, value)| {
                let value = if key.eq_ignore_ascii_case("col") {
                    day_night_semantic_name(value)
                } else {
                    value.clone()
                };
                (key.clone(), value)
            })
            .collect()
    })
}

fn definition_has_day_night_time_window(definition: &Definition) -> bool {
    definition
        .attrs
        .keys()
        .any(|key| key.eq_ignore_ascii_case("timeIn") || key.eq_ignore_ascii_case("timeOut"))
}

fn definition_day_night_variant(definition: Option<&Definition>) -> Option<DayNightVariant> {
    let definition = definition?;
    let time_value = |name: &str| {
        definition
            .attrs
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .and_then(|(_, value)| value.trim().parse::<f32>().ok())
    };
    let time_in = time_value("timeIn")?;
    let time_out = time_value("timeOut")?;
    if (time_in - time_out).abs() <= f32::EPSILON {
        None
    } else if time_in > time_out {
        Some(DayNightVariant::Night)
    } else {
        Some(DayNightVariant::Day)
    }
}

fn explicit_pair_target_variant(
    target: &Placement,
    source: &Placement,
    definitions: &HashMap<String, Definition>,
) -> DayNightVariant {
    let target_hint = definition_day_night_variant(definitions.get(&target.id))
        .or_else(|| placement_day_night_variant(target).map(|name| name.variant));
    let source_hint = definition_day_night_variant(definitions.get(&source.id))
        .or_else(|| placement_day_night_variant(source).map(|name| name.variant));
    target_hint
        .or_else(|| source_hint.map(DayNightVariant::opposite))
        .unwrap_or(DayNightVariant::Day)
}

fn remove_definition_day_night_time_window(definition: &mut Definition) {
    definition.attrs.retain(|key, _| {
        !key.eq_ignore_ascii_case("timeIn") && !key.eq_ignore_ascii_case("timeOut")
    });
}

fn compare_day_night_placement_pair(
    target: &Placement,
    source: &Placement,
    definitions: &HashMap<String, Definition>,
) -> Vec<String> {
    let mut differences = Vec::new();
    if target.tag != source.tag {
        differences.push(format!(
            "map element type differs ({} vs {})",
            target.tag, source.tag
        ));
    }
    if target.zone != source.zone {
        differences.push(format!("zone differs ({} vs {})", target.zone, source.zone));
    }
    if local_vertex_distance2(target.rot, source.rot) > 0.000001 {
        differences.push(format!(
            "rotation differs ({:.3},{:.3},{:.3} vs {:.3},{:.3},{:.3})",
            target.rot.x, target.rot.y, target.rot.z, source.rot.x, source.rot.y, source.rot.z
        ));
    }
    if filtered_day_night_placement_attrs(target) != filtered_day_night_placement_attrs(source) {
        differences.push("non-transform map attributes differ".to_string());
    }
    let target_definition = definitions.get(&target.id);
    let source_definition = definitions.get(&source.id);
    if filtered_day_night_definition_attrs(target_definition)
        != filtered_day_night_definition_attrs(source_definition)
    {
        differences.push(
            "definition attributes differ (excluding id, DFF, source, and time window)".to_string(),
        );
    }
    differences
}

fn day_night_semantic_name(value: &str) -> String {
    day_night_variant_name(value)
        .map(|name| name.base)
        .unwrap_or_else(|| value.trim().to_ascii_lowercase())
}

fn day_night_components_match(target: &[RawMeshComponent], source: &[RawMeshComponent]) -> bool {
    target.len() == source.len()
        && target.iter().zip(source).all(|(target, source)| {
            day_night_semantic_name(&target.name) == day_night_semantic_name(&source.name)
                && target.vertex_start == source.vertex_start
                && target.vertex_end == source.vertex_end
                && target.tri_start == source.tri_start
                && target.tri_end == source.tri_end
                && target.breakable == source.breakable
        })
}

fn day_night_frames_match(target: &[RawMeshFrame], source: &[RawMeshFrame]) -> bool {
    target.len() == source.len()
        && target.iter().zip(source).all(|(target, source)| {
            day_night_semantic_name(&target.name) == day_night_semantic_name(&source.name)
                && target.parent == source.parent
                && target.right == source.right
                && target.up == source.up
                && target.at == source.at
                && target.pos == source.pos
        })
}

const DAY_NIGHT_GEOMETRY_EPSILON: f32 = 0.0001;
const DAY_NIGHT_NORMAL_EPSILON: f32 = 0.0001;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct DayNightPositionKey([i64; 3]);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct DayNightMaterialKey {
    texture: String,
    properties: [i64; 7],
    animation_names: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct DayNightTriangleKey {
    material: DayNightMaterialKey,
    positions: [DayNightPositionKey; 3],
}

#[derive(Clone, Debug)]
struct DayNightRawAnalysis {
    target_to_sources: Vec<Vec<usize>>,
    notes: Vec<String>,
}

fn day_night_quantize(value: f32, scale: f64) -> i64 {
    ((value as f64) * scale).round() as i64
}

fn day_night_position_key(value: V3) -> DayNightPositionKey {
    let scale = 1.0 / DAY_NIGHT_GEOMETRY_EPSILON as f64;
    DayNightPositionKey([
        day_night_quantize(value.x, scale),
        day_night_quantize(value.y, scale),
        day_night_quantize(value.z, scale),
    ])
}

fn day_night_material_key(raw: &RawMesh, index: usize) -> Result<DayNightMaterialKey, String> {
    let material = raw
        .materials
        .get(index)
        .ok_or_else(|| format!("triangle references missing material slot {index}"))?;
    let texture = raw
        .material_textures
        .get(index)
        .ok_or_else(|| format!("material slot {index} has no texture assignment"))?;
    let scale = 1_000_000.0;
    Ok(DayNightMaterialKey {
        texture: texture.trim().to_ascii_lowercase(),
        properties: [
            day_night_quantize(material.color.x, scale),
            day_night_quantize(material.color.y, scale),
            day_night_quantize(material.color.z, scale),
            day_night_quantize(material.alpha, scale),
            day_night_quantize(material.ambient, scale),
            day_night_quantize(material.specular, scale),
            day_night_quantize(material.diffuse, scale),
        ],
        animation_names: raw
            .material_animations
            .get(index)
            .map(|animation| {
                animation
                    .names
                    .iter()
                    .map(|name| name.trim().to_ascii_lowercase())
                    .collect()
            })
            .unwrap_or_default(),
    })
}

fn day_night_triangle_key(raw: &RawMesh, triangle: Tri) -> Result<DayNightTriangleKey, String> {
    let indices = [
        triangle.a as usize,
        triangle.b as usize,
        triangle.c as usize,
    ];
    let mut positions = [DayNightPositionKey([0; 3]); 3];
    for (corner, index) in indices.into_iter().enumerate() {
        let position = raw
            .vertices
            .get(index)
            .ok_or_else(|| format!("triangle references missing vertex {index}"))?;
        positions[corner] = day_night_position_key(*position);
    }
    positions.sort();
    Ok(DayNightTriangleKey {
        material: day_night_material_key(raw, triangle.material as usize)?,
        positions,
    })
}

fn serialized_prelight_tiebreak_score(
    target: &RawMesh,
    target_index: usize,
    source: &RawMesh,
    source_index: usize,
) -> f64 {
    fn stream(raw: &RawMesh, index: usize, night: bool) -> Option<(V3, f32)> {
        let vertex_count = raw.vertices.len();
        let (primary_colors, primary_alphas, fallback_colors, fallback_alphas) = if night {
            (
                &raw.night_prelit_colors,
                &raw.night_prelit_alphas,
                &raw.prelit_colors,
                &raw.prelit_alphas,
            )
        } else {
            (
                &raw.prelit_colors,
                &raw.prelit_alphas,
                &raw.night_prelit_colors,
                &raw.night_prelit_alphas,
            )
        };
        let (colors, alphas) = if primary_colors.len() == vertex_count {
            (primary_colors, primary_alphas)
        } else if fallback_colors.len() == vertex_count {
            (fallback_colors, fallback_alphas)
        } else {
            return None;
        };
        Some((
            *colors.get(index)?,
            alphas.get(index).copied().unwrap_or(1.0),
        ))
    }

    let mut difference = 0u32;
    for night in [false, true] {
        let (Some((target_color, target_alpha)), Some((source_color, source_alpha))) = (
            stream(target, target_index, night),
            stream(source, source_index, night),
        ) else {
            continue;
        };
        for (target_channel, source_channel) in [
            (target_color.x, source_color.x),
            (target_color.y, source_color.y),
            (target_color.z, source_color.z),
            (target_alpha, source_alpha),
        ] {
            difference += color_channel_to_u8(target_channel)
                .abs_diff(color_channel_to_u8(source_channel)) as u32;
        }
    }
    difference as f64 * 0.001
}

fn day_night_triangle_corner_mapping_with_prelight_tiebreak(
    target: &RawMesh,
    target_triangle: Tri,
    source: &RawMesh,
    source_triangle: Tri,
    prefer_serialized_prelight: bool,
) -> Option<([usize; 3], f64)> {
    const PERMUTATIONS: [[usize; 3]; 6] = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];
    let target_indices = [
        target_triangle.a as usize,
        target_triangle.b as usize,
        target_triangle.c as usize,
    ];
    let source_indices = [
        source_triangle.a as usize,
        source_triangle.b as usize,
        source_triangle.c as usize,
    ];
    let target_has_normals = target.normals.len() == target.vertices.len();
    let source_has_normals = source.normals.len() == source.vertices.len();
    let target_has_uvs = target.uvs.len() == target.vertices.len();
    let source_has_uvs = source.uvs.len() == source.vertices.len();
    let mut best = None;
    for permutation in PERMUTATIONS {
        let mut score = 0.0f64;
        let mut mapped = [0usize; 3];
        let mut valid = true;
        for corner in 0..3 {
            let target_index = target_indices[corner];
            let source_index = source_indices[permutation[corner]];
            let (Some(target_position), Some(source_position)) = (
                target.vertices.get(target_index),
                source.vertices.get(source_index),
            ) else {
                valid = false;
                break;
            };
            let position_delta = local_vertex_distance2(*target_position, *source_position).sqrt();
            if position_delta > DAY_NIGHT_GEOMETRY_EPSILON {
                valid = false;
                break;
            }
            score += position_delta as f64 * 1_000_000_000.0;
            if target_has_normals && source_has_normals {
                score += local_vertex_distance2(
                    target.normals[target_index],
                    source.normals[source_index],
                ) as f64
                    * 1_000.0;
            }
            if target_has_uvs && source_has_uvs {
                let target_uv = target.uvs[target_index];
                let source_uv = source.uvs[source_index];
                let du = target_uv.u - source_uv.u;
                let dv = target_uv.v - source_uv.v;
                score += (du * du + dv * dv) as f64;
            }
            if prefer_serialized_prelight {
                score +=
                    serialized_prelight_tiebreak_score(target, target_index, source, source_index);
            }
            mapped[corner] = source_index;
        }
        if valid && best.is_none_or(|(_, best_score)| score < best_score) {
            best = Some((mapped, score));
        }
    }
    best
}

fn day_night_triangle_corner_mapping(
    target: &RawMesh,
    target_triangle: Tri,
    source: &RawMesh,
    source_triangle: Tri,
) -> Option<([usize; 3], f64)> {
    day_night_triangle_corner_mapping_with_prelight_tiebreak(
        target,
        target_triangle,
        source,
        source_triangle,
        false,
    )
}

fn mapped_uv_difference(
    target: &[V2],
    source: &[V2],
    target_to_sources: &[Vec<usize>],
) -> Option<(usize, f32)> {
    if target.len() != target_to_sources.len() {
        return None;
    }
    let mut mismatches = 0usize;
    let mut max_delta = 0.0f32;
    for (target_index, source_indices) in target_to_sources.iter().enumerate() {
        for source_index in source_indices {
            let source_uv = source.get(*source_index)?;
            let du = target[target_index].u - source_uv.u;
            let dv = target[target_index].v - source_uv.v;
            let delta = (du * du + dv * dv).sqrt();
            if delta > 0.000001 {
                mismatches += 1;
                max_delta = max_delta.max(delta);
            }
        }
    }
    Some((mismatches, max_delta))
}

fn analyze_day_night_raw_meshes(
    target: &RawMesh,
    source: &RawMesh,
) -> Result<DayNightRawAnalysis, Vec<String>> {
    analyze_day_night_raw_meshes_with_prelight_tiebreak(target, source, false)
}

fn analyze_day_night_raw_meshes_with_prelight_tiebreak(
    target: &RawMesh,
    source: &RawMesh,
    prefer_serialized_prelight: bool,
) -> Result<DayNightRawAnalysis, Vec<String>> {
    let mut differences = Vec::new();
    if target.triangles.len() != source.triangles.len() {
        differences.push(format!(
            "triangle count differs ({} vs {})",
            target.triangles.len(),
            source.triangles.len()
        ));
    }
    if (target.normals.len() == target.vertices.len())
        != (source.normals.len() == source.vertices.len())
    {
        differences.push("normal stream presence differs".to_string());
    }
    if target.uv_anim_dictionaries != source.uv_anim_dictionaries
        || target.uv_animations != source.uv_animations
    {
        differences.push("UV animations differ".to_string());
    }
    if target.effects_2dfx != source.effects_2dfx {
        differences.push("2DFX data differs".to_string());
    }
    let target_breakables = target
        .components
        .iter()
        .filter_map(|component| component.breakable.as_ref())
        .collect::<Vec<_>>();
    let source_breakables = source
        .components
        .iter()
        .filter_map(|component| component.breakable.as_ref())
        .collect::<Vec<_>>();
    if target_breakables != source_breakables {
        differences.push("breakable geometry differs".to_string());
    }

    let mut target_groups = BTreeMap::<DayNightTriangleKey, Vec<usize>>::new();
    let mut source_groups = BTreeMap::<DayNightTriangleKey, Vec<usize>>::new();
    for (triangle_index, triangle) in target.triangles.iter().copied().enumerate() {
        match day_night_triangle_key(target, triangle) {
            Ok(key) => target_groups.entry(key).or_default().push(triangle_index),
            Err(error) => differences.push(format!("target {error}")),
        }
    }
    for (triangle_index, triangle) in source.triangles.iter().copied().enumerate() {
        match day_night_triangle_key(source, triangle) {
            Ok(key) => source_groups.entry(key).or_default().push(triangle_index),
            Err(error) => differences.push(format!("source {error}")),
        }
    }
    let target_material_counts = target_groups.iter().fold(
        BTreeMap::<DayNightMaterialKey, usize>::new(),
        |mut counts, (key, values)| {
            *counts.entry(key.material.clone()).or_default() += values.len();
            counts
        },
    );
    let source_material_counts = source_groups.iter().fold(
        BTreeMap::<DayNightMaterialKey, usize>::new(),
        |mut counts, (key, values)| {
            *counts.entry(key.material.clone()).or_default() += values.len();
            counts
        },
    );
    if target_material_counts != source_material_counts {
        differences.push("material properties or material texture assignments differ".to_string());
    }
    let unmatched_target = target_groups
        .iter()
        .map(|(key, values)| {
            values
                .len()
                .abs_diff(source_groups.get(key).map(Vec::len).unwrap_or_default())
        })
        .sum::<usize>();
    let unmatched_source = source_groups
        .iter()
        .filter(|(key, _)| !target_groups.contains_key(*key))
        .map(|(_, values)| values.len())
        .sum::<usize>();
    if unmatched_target + unmatched_source > 0 {
        differences.push(format!(
            "canonical triangle geometry/material assignments differ ({} unmatched target, {} unmatched source)",
            unmatched_target, unmatched_source
        ));
    }
    if !differences.is_empty() {
        differences.sort();
        differences.dedup();
        return Err(differences);
    }

    let mut target_to_sources = vec![BTreeSet::<usize>::new(); target.vertices.len()];
    for (key, target_triangles) in target_groups {
        let mut source_triangles = source_groups.remove(&key).unwrap_or_default();
        for target_triangle_index in target_triangles {
            let target_triangle = target.triangles[target_triangle_index];
            let Some((best_position, best_mapping, _)) = source_triangles
                .iter()
                .enumerate()
                .filter_map(|(position, source_triangle_index)| {
                    day_night_triangle_corner_mapping_with_prelight_tiebreak(
                        target,
                        target_triangle,
                        source,
                        source.triangles[*source_triangle_index],
                        prefer_serialized_prelight,
                    )
                    .map(|(mapping, score)| (position, mapping, score))
                })
                .min_by(|left, right| left.2.total_cmp(&right.2))
            else {
                return Err(vec![format!(
                    "canonical triangle {} could not be paired safely",
                    target_triangle_index + 1
                )]);
            };
            source_triangles.swap_remove(best_position);
            for (target_index, source_index) in [
                target_triangle.a as usize,
                target_triangle.b as usize,
                target_triangle.c as usize,
            ]
            .into_iter()
            .zip(best_mapping)
            {
                target_to_sources[target_index].insert(source_index);
            }
        }
    }
    if let Some(index) = target_to_sources.iter().position(BTreeSet::is_empty) {
        return Err(vec![format!(
            "target vertex {index} is not referenced by matching geometry"
        )]);
    }
    let target_to_sources = target_to_sources
        .into_iter()
        .map(BTreeSet::into_iter)
        .map(Iterator::collect)
        .collect::<Vec<Vec<usize>>>();

    let target_has_normals = target.normals.len() == target.vertices.len();
    if target_has_normals {
        let mut mismatches = 0usize;
        let mut max_delta = 0.0f32;
        for (target_index, source_indices) in target_to_sources.iter().enumerate() {
            for source_index in source_indices {
                let delta = local_vertex_distance2(
                    target.normals[target_index],
                    source.normals[*source_index],
                )
                .sqrt();
                if delta > DAY_NIGHT_NORMAL_EPSILON {
                    mismatches += 1;
                    max_delta = max_delta.max(delta);
                }
            }
        }
        if mismatches > 0 {
            return Err(vec![format!(
                "normals differ at {mismatches} canonically mapped vertex pair(s), maximum delta {max_delta:.6}"
            )]);
        }
    }

    let target_has_flags = target.light_flags.len() == target.vertices.len();
    let source_has_flags = source.light_flags.len() == source.vertices.len();
    if target_has_flags != source_has_flags {
        return Err(vec![
            "vertex light flag stream presence differs".to_string(),
        ]);
    }
    if target_has_flags
        && target_to_sources
            .iter()
            .enumerate()
            .any(|(target_index, sources)| {
                sources.iter().any(|source_index| {
                    target.light_flags[target_index] != source.light_flags[*source_index]
                })
            })
    {
        return Err(vec![
            "vertex light flags differ after canonical geometry mapping".to_string(),
        ]);
    }

    let mut notes = Vec::new();
    if target.vertices.len() != source.vertices.len()
        || target.triangles != source.triangles
        || target.material_textures != source.material_textures
    {
        notes.push(format!(
            "raw stream/index ordering differs ({} vs {} vertices); canonical triangle geometry and material assignments matched",
            target.vertices.len(),
            source.vertices.len()
        ));
    }
    match mapped_uv_difference(&target.uvs, &source.uvs, &target_to_sources) {
        Some((mismatches, max_delta)) if mismatches > 0 => notes.push(format!(
            "primary UVs differ at {mismatches} mapped vertex pair(s), maximum delta {max_delta:.6}; selected model UVs were preserved"
        )),
        None if target.uvs.len() == target.vertices.len()
            || source.uvs.len() == source.vertices.len() =>
        {
            notes.push(
                "primary UV stream presence differs; selected model UVs were preserved".to_string(),
            )
        }
        _ => {}
    }
    if target.secondary_uvs.len() != source.secondary_uvs.len() {
        notes.push(format!(
            "secondary UV stream count differs ({} vs {}); selected model streams were preserved",
            target.secondary_uvs.len(),
            source.secondary_uvs.len()
        ));
    } else {
        for (stream, (target_uvs, source_uvs)) in target
            .secondary_uvs
            .iter()
            .zip(&source.secondary_uvs)
            .enumerate()
        {
            if let Some((mismatches, max_delta)) =
                mapped_uv_difference(target_uvs, source_uvs, &target_to_sources)
                && mismatches > 0
            {
                notes.push(format!(
                    "secondary UV stream {} differs at {mismatches} mapped vertex pair(s), maximum delta {max_delta:.6}; selected model stream was preserved",
                    stream + 1
                ));
            }
        }
    }
    if !day_night_components_match(&target.components, &source.components) {
        notes.push(
            "component/index metadata differs; selected model components were preserved"
                .to_string(),
        );
    }
    if !day_night_frames_match(&target.frames, &source.frames) {
        notes.push(
            "frame hierarchy metadata differs; selected model hierarchy was preserved".to_string(),
        );
    }
    Ok(DayNightRawAnalysis {
        target_to_sources,
        notes,
    })
}

const DAY_NIGHT_LOD_MIN_TRIANGLE_OVERLAP: f32 = 0.8;
const DAY_NIGHT_LOD_MAX_BLEND_DIAGONAL_FRACTION: f32 = 0.1;
const DAY_NIGHT_LOD_BLEND_NEIGHBORS: usize = 4;
const DAY_NIGHT_LOD_ROUGH_MIN_TRIANGLE_OVERLAP: f32 = 0.25;
const DAY_NIGHT_LOD_ROUGH_MAX_BLEND_DIAGONAL_FRACTION: f32 = 0.02;
const DAY_NIGHT_LOD_ROUGH_MIN_SURFACE_AREA_RATIO: f64 = 0.9;

#[derive(Clone, Copy)]
struct DayNightLodBlendSource {
    index: usize,
    weight: f32,
}

struct DayNightLodCorrespondence {
    base_to_other: Vec<Vec<DayNightLodBlendSource>>,
    matched_triangles: usize,
    fallback_vertices: usize,
    max_fallback_distance: f32,
}

fn day_night_raw_surface_area(raw: &RawMesh) -> f64 {
    raw.triangles
        .iter()
        .filter_map(|triangle| {
            let a = raw.vertices.get(triangle.a as usize)?;
            let b = raw.vertices.get(triangle.b as usize)?;
            let c = raw.vertices.get(triangle.c as usize)?;
            let ab = [
                b.x as f64 - a.x as f64,
                b.y as f64 - a.y as f64,
                b.z as f64 - a.z as f64,
            ];
            let ac = [
                c.x as f64 - a.x as f64,
                c.y as f64 - a.y as f64,
                c.z as f64 - a.z as f64,
            ];
            let cross = [
                ab[1] * ac[2] - ab[2] * ac[1],
                ab[2] * ac[0] - ab[0] * ac[2],
                ab[0] * ac[1] - ab[1] * ac[0],
            ];
            Some((cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt() * 0.5)
        })
        .sum()
}

fn day_night_source_is_richer_lod(target: &RawMesh, source: &RawMesh) -> bool {
    if source.triangles.len() != target.triangles.len() {
        return source.triangles.len() > target.triangles.len();
    }
    if source.vertices.len() != target.vertices.len() {
        return source.vertices.len() > target.vertices.len();
    }
    let target_area = day_night_raw_surface_area(target);
    let source_area = day_night_raw_surface_area(source);
    source_area > target_area * 1.0001
}

fn day_night_used_material_keys(raw: &RawMesh) -> Result<BTreeSet<DayNightMaterialKey>, String> {
    raw.triangles
        .iter()
        .map(|triangle| day_night_material_key(raw, triangle.material as usize))
        .collect()
}

fn day_night_vertex_material_keys(
    raw: &RawMesh,
) -> Result<Vec<BTreeSet<DayNightMaterialKey>>, String> {
    let mut keys = vec![BTreeSet::new(); raw.vertices.len()];
    for triangle in &raw.triangles {
        let material = day_night_material_key(raw, triangle.material as usize)?;
        for index in [triangle.a, triangle.b, triangle.c] {
            let Some(vertex_keys) = keys.get_mut(index as usize) else {
                return Err(format!("triangle references missing vertex {index}"));
            };
            vertex_keys.insert(material.clone());
        }
    }
    Ok(keys)
}

fn day_night_raw_diagonal(raw: &RawMesh) -> f32 {
    let Some(first) = raw.vertices.first().copied() else {
        return 0.0;
    };
    let (min, max) =
        raw.vertices
            .iter()
            .copied()
            .fold((first, first), |(mut min, mut max), value| {
                min.x = min.x.min(value.x);
                min.y = min.y.min(value.y);
                min.z = min.z.min(value.z);
                max.x = max.x.max(value.x);
                max.y = max.y.max(value.y);
                max.z = max.z.max(value.z);
                (min, max)
            });
    local_vertex_distance2(min, max).sqrt()
}

fn partial_day_night_lod_correspondence(
    base: &RawMesh,
    other: &RawMesh,
    minimum_triangle_overlap: f32,
) -> Result<DayNightLodCorrespondence, String> {
    if base.triangles.is_empty() || other.triangles.is_empty() {
        return Err("one LOD has no triangle geometry".to_string());
    }
    if day_night_used_material_keys(base)? != day_night_used_material_keys(other)? {
        return Err("LOD material properties or texture assignments differ".to_string());
    }
    if base.uv_anim_dictionaries != other.uv_anim_dictionaries
        || base.uv_animations != other.uv_animations
    {
        return Err("LOD UV animations differ".to_string());
    }
    if base.effects_2dfx != other.effects_2dfx {
        return Err("LOD 2DFX data differs".to_string());
    }
    let base_breakables = base
        .components
        .iter()
        .filter_map(|component| component.breakable.as_ref())
        .collect::<Vec<_>>();
    let other_breakables = other
        .components
        .iter()
        .filter_map(|component| component.breakable.as_ref())
        .collect::<Vec<_>>();
    if base_breakables != other_breakables {
        return Err("LOD breakable geometry differs".to_string());
    }

    let mut base_groups = BTreeMap::<DayNightTriangleKey, Vec<usize>>::new();
    let mut other_groups = BTreeMap::<DayNightTriangleKey, Vec<usize>>::new();
    for (index, triangle) in base.triangles.iter().copied().enumerate() {
        base_groups
            .entry(day_night_triangle_key(base, triangle)?)
            .or_default()
            .push(index);
    }
    for (index, triangle) in other.triangles.iter().copied().enumerate() {
        other_groups
            .entry(day_night_triangle_key(other, triangle)?)
            .or_default()
            .push(index);
    }

    let mut mapped = vec![BTreeSet::<usize>::new(); base.vertices.len()];
    let mut matched_triangles = 0usize;
    for (key, base_triangles) in base_groups {
        let Some(other_triangles) = other_groups.get_mut(&key) else {
            continue;
        };
        for base_triangle_index in base_triangles {
            let base_triangle = base.triangles[base_triangle_index];
            let Some((best_position, best_mapping, _)) = other_triangles
                .iter()
                .enumerate()
                .filter_map(|(position, other_triangle_index)| {
                    day_night_triangle_corner_mapping(
                        base,
                        base_triangle,
                        other,
                        other.triangles[*other_triangle_index],
                    )
                    .map(|(mapping, score)| (position, mapping, score))
                })
                .min_by(|left, right| left.2.total_cmp(&right.2))
            else {
                continue;
            };
            other_triangles.swap_remove(best_position);
            for (base_index, other_index) in [
                base_triangle.a as usize,
                base_triangle.b as usize,
                base_triangle.c as usize,
            ]
            .into_iter()
            .zip(best_mapping)
            {
                mapped[base_index].insert(other_index);
            }
            matched_triangles += 1;
        }
    }
    let smaller_triangle_count = base.triangles.len().min(other.triangles.len());
    let overlap = matched_triangles as f32 / smaller_triangle_count as f32;
    if overlap + f32::EPSILON < minimum_triangle_overlap {
        return Err(format!(
            "LOD overlap is only {:.1}% ({matched_triangles}/{smaller_triangle_count} triangles); at least {:.0}% is required",
            overlap * 100.0,
            minimum_triangle_overlap * 100.0
        ));
    }

    let base_materials = day_night_vertex_material_keys(base)?;
    let other_materials = day_night_vertex_material_keys(other)?;
    let max_fallback_distance = day_night_raw_diagonal(base).max(day_night_raw_diagonal(other))
        * DAY_NIGHT_LOD_MAX_BLEND_DIAGONAL_FRACTION;
    let max_fallback_distance = max_fallback_distance.max(DAY_NIGHT_GEOMETRY_EPSILON);
    let mut fallback_vertices = 0usize;
    let mut furthest_fallback = 0.0f32;
    let mut base_to_other = Vec::with_capacity(base.vertices.len());
    for (base_index, exact_sources) in mapped.into_iter().enumerate() {
        if !exact_sources.is_empty() {
            base_to_other.push(
                exact_sources
                    .into_iter()
                    .map(|index| DayNightLodBlendSource { index, weight: 1.0 })
                    .collect(),
            );
            continue;
        }
        let mut candidates = other
            .vertices
            .iter()
            .enumerate()
            .filter(|(other_index, _)| {
                base_materials[base_index].is_empty()
                    || other_materials[*other_index].is_empty()
                    || !base_materials[base_index].is_disjoint(&other_materials[*other_index])
            })
            .map(|(other_index, position)| {
                (
                    other_index,
                    local_vertex_distance2(base.vertices[base_index], *position).sqrt(),
                )
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| left.1.total_cmp(&right.1));
        let Some((_, nearest_distance)) = candidates.first().copied() else {
            return Err(format!(
                "LOD vertex {base_index} has no material-compatible lighting source"
            ));
        };
        if nearest_distance > max_fallback_distance {
            return Err(format!(
                "LOD vertex {base_index} is {:.3} units from its nearest compatible lighting source; maximum blend distance is {:.3}",
                nearest_distance, max_fallback_distance
            ));
        }
        let exact = candidates
            .iter()
            .take_while(|(_, distance)| *distance <= DAY_NIGHT_GEOMETRY_EPSILON)
            .copied()
            .collect::<Vec<_>>();
        let sources = if exact.is_empty() {
            candidates
                .into_iter()
                .take(DAY_NIGHT_LOD_BLEND_NEIGHBORS)
                .map(|(index, distance)| DayNightLodBlendSource {
                    index,
                    weight: 1.0 / distance.max(DAY_NIGHT_GEOMETRY_EPSILON),
                })
                .collect()
        } else {
            exact
                .into_iter()
                .map(|(index, _)| DayNightLodBlendSource { index, weight: 1.0 })
                .collect()
        };
        fallback_vertices += 1;
        furthest_fallback = furthest_fallback.max(nearest_distance);
        base_to_other.push(sources);
    }
    Ok(DayNightLodCorrespondence {
        base_to_other,
        matched_triangles,
        fallback_vertices,
        max_fallback_distance: furthest_fallback,
    })
}

fn blended_day_night_variant_stream(
    source: &RawMesh,
    variant: DayNightVariant,
    mapping: &[Vec<DayNightLodBlendSource>],
) -> Result<(Vec<V3>, Vec<f32>), String> {
    let (source_colors, source_alphas) = day_night_variant_stream(source, variant)?;
    let mut colors = Vec::with_capacity(mapping.len());
    let mut alphas = Vec::with_capacity(mapping.len());
    for (base_index, sources) in mapping.iter().enumerate() {
        if sources.is_empty() {
            return Err(format!(
                "LOD vertex {base_index} has no lighting blend sources"
            ));
        }
        let mut color = V3::default();
        let mut alpha = 0.0f32;
        let mut total_weight = 0.0f32;
        for source in sources {
            let source_color = source_colors
                .get(source.index)
                .ok_or_else(|| format!("LOD blend references missing vertex {}", source.index))?;
            let source_alpha = source_alphas
                .get(source.index)
                .copied()
                .ok_or_else(|| format!("LOD blend references missing alpha {}", source.index))?;
            color.x += source_color.x * source.weight;
            color.y += source_color.y * source.weight;
            color.z += source_color.z * source.weight;
            alpha += source_alpha * source.weight;
            total_weight += source.weight;
        }
        if !total_weight.is_finite() || total_weight <= 0.0 {
            return Err(format!("LOD vertex {base_index} has invalid blend weights"));
        }
        colors.push(V3 {
            x: color.x / total_weight,
            y: color.y / total_weight,
            z: color.z / total_weight,
        });
        alphas.push(alpha / total_weight);
    }
    Ok((colors, alphas))
}

const DAY_NIGHT_SURFACE_POSITION_EPSILON: f32 = 0.1;
const DAY_NIGHT_SURFACE_UV_EPSILON: f32 = 0.01;
const DAY_NIGHT_SURFACE_AREA_RELATIVE_EPSILON: f64 = 0.001;

struct DayNightSurfaceSample {
    position: V3,
    normal: Option<V3>,
    primary_uv: Option<V2>,
    secondary_uvs: Vec<V2>,
}

fn day_night_triangle_indices(triangle: Tri) -> [usize; 3] {
    [
        triangle.a as usize,
        triangle.b as usize,
        triangle.c as usize,
    ]
}

fn day_night_interpolate_v3(values: &[V3], indices: [usize; 3], weights: [f32; 3]) -> V3 {
    V3 {
        x: values[indices[0]].x * weights[0]
            + values[indices[1]].x * weights[1]
            + values[indices[2]].x * weights[2],
        y: values[indices[0]].y * weights[0]
            + values[indices[1]].y * weights[1]
            + values[indices[2]].y * weights[2],
        z: values[indices[0]].z * weights[0]
            + values[indices[1]].z * weights[1]
            + values[indices[2]].z * weights[2],
    }
}

fn day_night_interpolate_v2(values: &[V2], indices: [usize; 3], weights: [f32; 3]) -> V2 {
    V2 {
        u: values[indices[0]].u * weights[0]
            + values[indices[1]].u * weights[1]
            + values[indices[2]].u * weights[2],
        v: values[indices[0]].v * weights[0]
            + values[indices[1]].v * weights[1]
            + values[indices[2]].v * weights[2],
    }
}

fn day_night_wrapped_uv_delta(a: V2, b: V2) -> f32 {
    let du = a.u - b.u;
    let dv = a.v - b.v;
    let du = du - du.round();
    let dv = dv - dv.round();
    du * du + dv * dv
}

fn day_night_surface_sample(
    raw: &RawMesh,
    triangle: Tri,
    weights: [f32; 3],
) -> Result<DayNightSurfaceSample, String> {
    let indices = day_night_triangle_indices(triangle);
    if indices.iter().any(|index| *index >= raw.vertices.len()) {
        return Err("triangle references a missing vertex".to_string());
    }
    Ok(DayNightSurfaceSample {
        position: day_night_interpolate_v3(&raw.vertices, indices, weights),
        normal: (raw.normals.len() == raw.vertices.len())
            .then(|| day_night_interpolate_v3(&raw.normals, indices, weights)),
        primary_uv: (raw.uvs.len() == raw.vertices.len())
            .then(|| day_night_interpolate_v2(&raw.uvs, indices, weights)),
        secondary_uvs: raw
            .secondary_uvs
            .iter()
            .map(|stream| day_night_interpolate_v2(stream, indices, weights))
            .collect(),
    })
}

fn day_night_closest_triangle_barycentric(
    point: V3,
    a: V3,
    b: V3,
    c: V3,
) -> Option<([f32; 3], f32)> {
    let point = Vec3::new(point.x, point.y, point.z);
    let a = Vec3::new(a.x, a.y, a.z);
    let b = Vec3::new(b.x, b.y, b.z);
    let c = Vec3::new(c.x, c.y, c.z);
    let ab = b - a;
    let ac = c - a;
    if ab.cross(ac).length_squared() <= f32::EPSILON {
        return None;
    }

    let ap = point - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    let weights = if d1 <= 0.0 && d2 <= 0.0 {
        [1.0, 0.0, 0.0]
    } else {
        let bp = point - b;
        let d3 = ab.dot(bp);
        let d4 = ac.dot(bp);
        if d3 >= 0.0 && d4 <= d3 {
            [0.0, 1.0, 0.0]
        } else {
            let vc = d1 * d4 - d3 * d2;
            if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
                let v = d1 / (d1 - d3);
                [1.0 - v, v, 0.0]
            } else {
                let cp = point - c;
                let d5 = ab.dot(cp);
                let d6 = ac.dot(cp);
                if d6 >= 0.0 && d5 <= d6 {
                    [0.0, 0.0, 1.0]
                } else {
                    let vb = d5 * d2 - d1 * d6;
                    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
                        let w = d2 / (d2 - d6);
                        [1.0 - w, 0.0, w]
                    } else {
                        let va = d3 * d6 - d5 * d4;
                        if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
                            let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
                            [0.0, 1.0 - w, w]
                        } else {
                            let denominator = va + vb + vc;
                            if denominator.abs() <= f32::EPSILON {
                                return None;
                            }
                            let inverse = 1.0 / denominator;
                            let v = vb * inverse;
                            let w = vc * inverse;
                            [1.0 - v - w, v, w]
                        }
                    }
                }
            }
        }
    };
    let closest = a * weights[0] + b * weights[1] + c * weights[2];
    Some((weights, point.distance_squared(closest)))
}

fn day_night_surface_streams_match(target: &RawMesh, source: &RawMesh) -> Result<(), String> {
    let aligned = |length: usize, vertices: usize| length == 0 || length == vertices;
    for (label, target_length, source_length) in [
        ("normal", target.normals.len(), source.normals.len()),
        ("primary UV", target.uvs.len(), source.uvs.len()),
        (
            "vertex light flag",
            target.light_flags.len(),
            source.light_flags.len(),
        ),
    ] {
        if !aligned(target_length, target.vertices.len())
            || !aligned(source_length, source.vertices.len())
        {
            return Err(format!("{label} stream is not vertex-aligned"));
        }
        if (target_length > 0) != (source_length > 0) {
            return Err(format!("{label} stream presence differs"));
        }
    }
    if target.secondary_uvs.len() != source.secondary_uvs.len() {
        return Err(format!(
            "secondary UV stream count differs ({} vs {})",
            target.secondary_uvs.len(),
            source.secondary_uvs.len()
        ));
    }
    for (index, (target_uvs, source_uvs)) in target
        .secondary_uvs
        .iter()
        .zip(&source.secondary_uvs)
        .enumerate()
    {
        if target_uvs.len() != target.vertices.len() || source_uvs.len() != source.vertices.len() {
            return Err(format!(
                "secondary UV stream {} is not vertex-aligned",
                index + 1
            ));
        }
    }
    Ok(())
}

fn day_night_material_surface_areas(
    raw: &RawMesh,
) -> Result<BTreeMap<DayNightMaterialKey, f64>, String> {
    let mut areas = BTreeMap::new();
    for triangle in &raw.triangles {
        let indices = day_night_triangle_indices(*triangle);
        let [Some(a), Some(b), Some(c)] = indices.map(|index| raw.vertices.get(index)) else {
            return Err("triangle references a missing vertex".to_string());
        };
        let ab = Vec3::new(b.x - a.x, b.y - a.y, b.z - a.z);
        let ac = Vec3::new(c.x - a.x, c.y - a.y, c.z - a.z);
        *areas
            .entry(day_night_material_key(raw, triangle.material as usize)?)
            .or_insert(0.0) += (ab.cross(ac).length() * 0.5) as f64;
    }
    Ok(areas)
}

fn day_night_surface_material_triangles(
    raw: &RawMesh,
) -> Result<BTreeMap<DayNightMaterialKey, Vec<usize>>, String> {
    let mut triangles = BTreeMap::new();
    for (index, triangle) in raw.triangles.iter().enumerate() {
        triangles
            .entry(day_night_material_key(raw, triangle.material as usize)?)
            .or_insert_with(Vec::new)
            .push(index);
    }
    Ok(triangles)
}

fn day_night_match_surface_sample(
    source: &RawMesh,
    source_triangles: &BTreeMap<DayNightMaterialKey, Vec<usize>>,
    material: &DayNightMaterialKey,
    sample: &DayNightSurfaceSample,
) -> Option<Vec<DayNightLodBlendSource>> {
    let candidates = source_triangles.get(material)?;
    let mut best: Option<(f32, [usize; 3], [f32; 3])> = None;
    for triangle_index in candidates {
        let triangle = source.triangles[*triangle_index];
        let indices = day_night_triangle_indices(triangle);
        let [Some(a), Some(b), Some(c)] = indices.map(|index| source.vertices.get(index)) else {
            continue;
        };
        let Some((weights, distance2)) =
            day_night_closest_triangle_barycentric(sample.position, *a, *b, *c)
        else {
            continue;
        };
        if distance2 > DAY_NIGHT_SURFACE_POSITION_EPSILON * DAY_NIGHT_SURFACE_POSITION_EPSILON {
            continue;
        }

        let mut score = distance2 * 1_000_000.0;
        if let Some(target_normal) = sample.normal {
            let source_normal = day_night_interpolate_v3(&source.normals, indices, weights);
            let normal_delta2 = local_vertex_distance2(target_normal, source_normal);
            score += normal_delta2;
        }
        if let Some(target_uv) = sample.primary_uv {
            let source_uv = day_night_interpolate_v2(&source.uvs, indices, weights);
            score += day_night_wrapped_uv_delta(target_uv, source_uv);
        }
        for (target_uv, source_uvs) in sample.secondary_uvs.iter().zip(&source.secondary_uvs) {
            let source_uv = day_night_interpolate_v2(source_uvs, indices, weights);
            score += day_night_wrapped_uv_delta(*target_uv, source_uv);
        }
        if best.is_none_or(|(best_score, _, _)| score < best_score) {
            best = Some((score, indices, weights));
        }
    }
    best.map(|(_, indices, weights)| {
        indices
            .into_iter()
            .zip(weights)
            .filter(|(_, weight)| *weight > 0.000001)
            .map(|(index, weight)| DayNightLodBlendSource { index, weight })
            .collect()
    })
}

fn day_night_surface_correspondence(
    base: &RawMesh,
    other: &RawMesh,
) -> Result<Vec<Vec<DayNightLodBlendSource>>, String> {
    let other_triangles = day_night_surface_material_triangles(other)?;
    let base_vertex_materials = day_night_vertex_material_keys(base)?;
    let mut mapping = Vec::with_capacity(base.vertices.len());
    for (vertex_index, materials) in base_vertex_materials.iter().enumerate() {
        if materials.is_empty() {
            return Err(format!(
                "selected vertex {vertex_index} is not referenced by geometry"
            ));
        }
        let mut sources = BTreeMap::<usize, f32>::new();
        for material in materials {
            let sample = DayNightSurfaceSample {
                position: base.vertices[vertex_index],
                normal: (base.normals.len() == base.vertices.len())
                    .then(|| base.normals[vertex_index]),
                primary_uv: (base.uvs.len() == base.vertices.len()).then(|| base.uvs[vertex_index]),
                secondary_uvs: base
                    .secondary_uvs
                    .iter()
                    .map(|stream| stream[vertex_index])
                    .collect(),
            };
            let Some(matches) =
                day_night_match_surface_sample(other, &other_triangles, material, &sample)
            else {
                return Err(format!(
                    "selected vertex {vertex_index} has no position/normal/UV-equivalent surface in the other model"
                ));
            };
            if base.light_flags.len() == base.vertices.len()
                && matches.iter().any(|source| {
                    other.light_flags.get(source.index).copied()
                        != Some(base.light_flags[vertex_index])
                })
            {
                return Err(format!(
                    "vertex light flags differ at surface-equivalent vertex {vertex_index}"
                ));
            }
            for source in matches {
                *sources.entry(source.index).or_default() += source.weight;
            }
        }
        mapping.push(
            sources
                .into_iter()
                .map(|(index, weight)| DayNightLodBlendSource { index, weight })
                .collect(),
        );
    }

    const TRIANGLE_SAMPLES: [[f32; 3]; 4] = [
        [1.0 / 3.0; 3],
        [0.5, 0.5, 0.0],
        [0.0, 0.5, 0.5],
        [0.5, 0.0, 0.5],
    ];
    for (triangle_index, triangle) in base.triangles.iter().copied().enumerate() {
        let indices = day_night_triangle_indices(triangle);
        let [Some(a), Some(b), Some(c)] = indices.map(|index| base.vertices.get(index)) else {
            return Err(format!(
                "selected triangle {triangle_index} references a missing vertex"
            ));
        };
        let ab = Vec3::new(b.x - a.x, b.y - a.y, b.z - a.z);
        let ac = Vec3::new(c.x - a.x, c.y - a.y, c.z - a.z);
        if ab.cross(ac).length_squared() <= f32::EPSILON {
            continue;
        }
        let material = day_night_material_key(base, triangle.material as usize)?;
        for weights in TRIANGLE_SAMPLES {
            let sample = day_night_surface_sample(base, triangle, weights)?;
            if day_night_match_surface_sample(other, &other_triangles, &material, &sample).is_none()
            {
                return Err(format!(
                    "selected triangle {} has no position/normal/UV-equivalent surface in the other model",
                    triangle_index + 1
                ));
            }
        }
    }
    Ok(mapping)
}

fn day_night_canonical_triangle_overlap(
    target: &RawMesh,
    source: &RawMesh,
) -> Result<usize, String> {
    let mut target_groups = BTreeMap::<DayNightTriangleKey, usize>::new();
    let mut source_groups = BTreeMap::<DayNightTriangleKey, usize>::new();
    for triangle in target.triangles.iter().copied() {
        *target_groups
            .entry(day_night_triangle_key(target, triangle)?)
            .or_default() += 1;
    }
    for triangle in source.triangles.iter().copied() {
        *source_groups
            .entry(day_night_triangle_key(source, triangle)?)
            .or_default() += 1;
    }
    Ok(target_groups
        .iter()
        .map(|(key, count)| count.min(source_groups.get(key).unwrap_or(&0)))
        .sum())
}

fn day_night_blend_surface_v3(values: &[V3], sources: &[DayNightLodBlendSource]) -> Option<V3> {
    let total_weight = sources.iter().map(|source| source.weight).sum::<f32>();
    if !total_weight.is_finite() || total_weight <= 0.0 {
        return None;
    }
    let mut value = V3::default();
    for source in sources {
        let source_value = values.get(source.index)?;
        value.x += source_value.x * source.weight;
        value.y += source_value.y * source.weight;
        value.z += source_value.z * source.weight;
    }
    Some(V3 {
        x: value.x / total_weight,
        y: value.y / total_weight,
        z: value.z / total_weight,
    })
}

fn day_night_blend_surface_v2(values: &[V2], sources: &[DayNightLodBlendSource]) -> Option<V2> {
    let total_weight = sources.iter().map(|source| source.weight).sum::<f32>();
    if !total_weight.is_finite() || total_weight <= 0.0 {
        return None;
    }
    let mut value = V2::default();
    for source in sources {
        let source_value = values.get(source.index)?;
        value.u += source_value.u * source.weight;
        value.v += source_value.v * source.weight;
    }
    Some(V2 {
        u: value.u / total_weight,
        v: value.v / total_weight,
    })
}

fn day_night_surface_preservation_notes(
    target: &RawMesh,
    source: &RawMesh,
    target_to_source: &[Vec<DayNightLodBlendSource>],
) -> Vec<String> {
    let mut notes = Vec::new();
    let mut position_mismatches = 0usize;
    let mut max_position_delta = 0.0f32;
    for (target_index, sources) in target_to_source.iter().enumerate() {
        let Some(source_position) = day_night_blend_surface_v3(&source.vertices, sources) else {
            continue;
        };
        let delta = local_vertex_distance2(target.vertices[target_index], source_position).sqrt();
        if delta > DAY_NIGHT_GEOMETRY_EPSILON {
            position_mismatches += 1;
            max_position_delta = max_position_delta.max(delta);
        }
    }
    if position_mismatches > 0 {
        notes.push(format!(
            "surface positions differ at {position_mismatches} mapped vertex/vertices (maximum {max_position_delta:.6}); selected geometry was preserved"
        ));
    }

    if target.normals.len() == target.vertices.len() {
        let mut mismatches = 0usize;
        let mut max_delta = 0.0f32;
        for (target_index, sources) in target_to_source.iter().enumerate() {
            let Some(source_normal) = day_night_blend_surface_v3(&source.normals, sources) else {
                continue;
            };
            let delta = local_vertex_distance2(target.normals[target_index], source_normal).sqrt();
            if delta > DAY_NIGHT_NORMAL_EPSILON {
                mismatches += 1;
                max_delta = max_delta.max(delta);
            }
        }
        if mismatches > 0 {
            notes.push(format!(
                "normals differ at {mismatches} mapped vertex/vertices (maximum {max_delta:.6}); selected normals were preserved"
            ));
        }
    }

    let mut add_uv_note = |label: &str, target_uvs: &[V2], source_uvs: &[V2]| {
        let mut mismatches = 0usize;
        let mut max_delta = 0.0f32;
        for (target_index, sources) in target_to_source.iter().enumerate() {
            let Some(source_uv) = day_night_blend_surface_v2(source_uvs, sources) else {
                continue;
            };
            let delta = day_night_wrapped_uv_delta(target_uvs[target_index], source_uv).sqrt();
            if delta > DAY_NIGHT_SURFACE_UV_EPSILON {
                mismatches += 1;
                max_delta = max_delta.max(delta);
            }
        }
        if mismatches > 0 {
            notes.push(format!(
                "{label} differ at {mismatches} mapped vertex/vertices modulo whole-texture wrapping (maximum {max_delta:.6}); selected UVs were preserved"
            ));
        }
    };
    if target.uvs.len() == target.vertices.len() {
        add_uv_note("primary UVs", &target.uvs, &source.uvs);
    }
    for (index, (target_uvs, source_uvs)) in target
        .secondary_uvs
        .iter()
        .zip(&source.secondary_uvs)
        .enumerate()
    {
        add_uv_note(
            &format!("secondary UV stream {}", index + 1),
            target_uvs,
            source_uvs,
        );
    }
    notes
}

fn surface_matched_day_night_lod_correspondence(
    base: &RawMesh,
    other: &RawMesh,
) -> Result<DayNightLodCorrespondence, String> {
    let base_to_other = day_night_surface_correspondence(base, other)?;
    let matched_triangles = day_night_canonical_triangle_overlap(base, other)?;
    let mut fallback_vertices = 0usize;
    let mut max_fallback_distance = 0.0f32;
    for (base_index, sources) in base_to_other.iter().enumerate() {
        let Some(source_position) = day_night_blend_surface_v3(&other.vertices, sources) else {
            return Err(format!(
                "LOD vertex {base_index} has invalid surface blend sources"
            ));
        };
        let distance = local_vertex_distance2(base.vertices[base_index], source_position).sqrt();
        if distance > DAY_NIGHT_GEOMETRY_EPSILON || sources.len() != 1 {
            fallback_vertices += 1;
            max_fallback_distance = max_fallback_distance.max(distance);
        }
    }
    Ok(DayNightLodCorrespondence {
        base_to_other,
        matched_triangles,
        fallback_vertices,
        max_fallback_distance,
    })
}

fn rough_day_night_lod_correspondence(
    base: &RawMesh,
    other: &RawMesh,
) -> Result<DayNightLodCorrespondence, String> {
    let correspondence = partial_day_night_lod_correspondence(
        base,
        other,
        DAY_NIGHT_LOD_ROUGH_MIN_TRIANGLE_OVERLAP,
    )?;
    let base_area = day_night_raw_surface_area(base);
    let other_area = day_night_raw_surface_area(other);
    let area_ratio = base_area.min(other_area) / base_area.max(other_area).max(f64::EPSILON);
    if area_ratio < DAY_NIGHT_LOD_ROUGH_MIN_SURFACE_AREA_RATIO {
        return Err(format!(
            "LOD surface areas differ too much for rough lighting transfer ({base_area:.3} vs {other_area:.3}, {:.1}% similarity)",
            area_ratio * 100.0
        ));
    }
    let diagonal = day_night_raw_diagonal(base).max(day_night_raw_diagonal(other));
    let maximum_distance = (diagonal * DAY_NIGHT_LOD_ROUGH_MAX_BLEND_DIAGONAL_FRACTION)
        .max(DAY_NIGHT_GEOMETRY_EPSILON);
    if correspondence.max_fallback_distance > maximum_distance {
        return Err(format!(
            "rough LOD lighting transfer needs a {:.3}-unit nearest blend, exceeding the {:.3}-unit limit",
            correspondence.max_fallback_distance, maximum_distance
        ));
    }
    Ok(correspondence)
}

fn merge_surface_equivalent_day_night_details(
    target: &RawMesh,
    source: &RawMesh,
    target_variant: DayNightVariant,
) -> Result<(RawMesh, Vec<String>), String> {
    if target.triangles.is_empty() || source.triangles.is_empty() {
        return Err("one detail model has no triangle geometry".to_string());
    }
    day_night_surface_streams_match(target, source)?;
    if day_night_used_material_keys(target)? != day_night_used_material_keys(source)? {
        return Err("material properties or texture assignments differ".to_string());
    }
    if target.uv_anim_dictionaries != source.uv_anim_dictionaries
        || target.uv_animations != source.uv_animations
    {
        return Err("UV animations differ".to_string());
    }
    if target.effects_2dfx != source.effects_2dfx {
        return Err("2DFX data differs".to_string());
    }
    let target_breakables = target
        .components
        .iter()
        .filter_map(|component| component.breakable.as_ref())
        .collect::<Vec<_>>();
    let source_breakables = source
        .components
        .iter()
        .filter_map(|component| component.breakable.as_ref())
        .collect::<Vec<_>>();
    if target_breakables != source_breakables {
        return Err("breakable geometry differs".to_string());
    }

    let target_areas = day_night_material_surface_areas(target)?;
    let source_areas = day_night_material_surface_areas(source)?;
    if target_areas.keys().ne(source_areas.keys()) {
        return Err("material surface assignments differ".to_string());
    }
    for (material, target_area) in &target_areas {
        let source_area = source_areas[material];
        let tolerance = target_area.abs().max(source_area.abs()).max(1.0)
            * DAY_NIGHT_SURFACE_AREA_RELATIVE_EPSILON;
        if (target_area - source_area).abs() > tolerance {
            return Err(format!(
                "material surface area differs ({target_area:.6} vs {source_area:.6})"
            ));
        }
    }

    let target_to_source = day_night_surface_correspondence(target, source)?;
    day_night_surface_correspondence(source, target).map_err(|error| {
        format!("other model contains geometry not present in the selected model: {error}")
    })?;
    let (target_colors, target_alphas) = day_night_variant_stream(target, target_variant)?;
    let (source_colors, source_alphas) =
        blended_day_night_variant_stream(source, target_variant.opposite(), &target_to_source)?;
    let mut merged = target.clone();
    if target_variant == DayNightVariant::Day {
        merged.prelit_colors = target_colors;
        merged.prelit_alphas = target_alphas;
        merged.night_prelit_colors = source_colors;
        merged.night_prelit_alphas = source_alphas;
    } else {
        merged.prelit_colors = source_colors;
        merged.prelit_alphas = source_alphas;
        merged.night_prelit_colors = target_colors;
        merged.night_prelit_alphas = target_alphas;
    }

    let canonical_overlap = day_night_canonical_triangle_overlap(target, source)?;
    let mut notes = vec![format!(
        "surface-equivalent detail fallback retained selected geometry ({} vertices, {} triangles); {}/{} triangles matched canonically and the remainder used equivalent surface interpolation",
        target.vertices.len(),
        target.triangles.len(),
        canonical_overlap,
        target.triangles.len().max(source.triangles.len())
    )];
    notes.extend(day_night_surface_preservation_notes(
        target,
        source,
        &target_to_source,
    ));
    if !day_night_components_match(&target.components, &source.components) {
        notes.push("selected component/index metadata was preserved".to_string());
    }
    if !day_night_frames_match(&target.frames, &source.frames) {
        notes.push("selected frame hierarchy was preserved".to_string());
    }
    Ok((merged, notes))
}

fn merge_near_matching_day_night_lods(
    target: &RawMesh,
    source: &RawMesh,
    target_variant: DayNightVariant,
) -> Result<(RawMesh, Vec<String>), String> {
    let source_is_richer = day_night_source_is_richer_lod(target, source);
    let (base, base_variant, other, retained) = if source_is_richer {
        (
            source,
            target_variant.opposite(),
            target,
            "secondary (richer)",
        )
    } else {
        (target, target_variant, source, "selected")
    };
    let (correspondence, fallback_note) = match partial_day_night_lod_correspondence(
        base,
        other,
        DAY_NIGHT_LOD_MIN_TRIANGLE_OVERLAP,
    ) {
        Ok(correspondence) => (correspondence, None),
        Err(error) if error.starts_with("LOD overlap is only ") => {
            match surface_matched_day_night_lod_correspondence(base, other) {
                Ok(correspondence) => (
                    correspondence,
                    Some(format!(
                        "{error}; alternate topology was accepted after the retained LOD surface mapped safely onto the other variant"
                    )),
                ),
                Err(surface_error) => {
                    let correspondence = rough_day_night_lod_correspondence(base, other).map_err(
                        |rough_error| {
                            format!(
                                "{error}; alternate-topology LOD surface matching was not safe: {surface_error}; rough LOD lighting transfer was not safe: {rough_error}"
                            )
                        },
                    )?;
                    (
                        correspondence,
                        Some(format!(
                            "{error}; exact surface matching was unavailable ({surface_error}); used bounded rough LOD lighting transfer"
                        )),
                    )
                }
            }
        }
        Err(error) => return Err(error),
    };
    let (base_colors, base_alphas) = day_night_variant_stream(base, base_variant)?;
    let (other_colors, other_alphas) = blended_day_night_variant_stream(
        other,
        base_variant.opposite(),
        &correspondence.base_to_other,
    )?;
    let mut merged = base.clone();
    if base_variant == DayNightVariant::Day {
        merged.prelit_colors = base_colors;
        merged.prelit_alphas = base_alphas;
        merged.night_prelit_colors = other_colors;
        merged.night_prelit_alphas = other_alphas;
    } else {
        merged.prelit_colors = other_colors;
        merged.prelit_alphas = other_alphas;
        merged.night_prelit_colors = base_colors;
        merged.night_prelit_alphas = base_alphas;
    }

    let smaller_triangle_count = base.triangles.len().min(other.triangles.len());
    let mut notes = vec![format!(
        "near-matching LOD fallback retained {retained} geometry ({} vertices, {} triangles), matched {}/{} shared triangles, preserved {} retained-only triangle(s), and discarded {} non-retained-only triangle(s)",
        base.vertices.len(),
        base.triangles.len(),
        correspondence.matched_triangles,
        smaller_triangle_count,
        base.triangles
            .len()
            .saturating_sub(correspondence.matched_triangles),
        other
            .triangles
            .len()
            .saturating_sub(correspondence.matched_triangles),
    )];
    if let Some(fallback_note) = fallback_note {
        notes.push(fallback_note);
    }
    if correspondence.fallback_vertices > 0 {
        notes.push(format!(
            "blended {} unmatched retained LOD vertex/vertices from nearby compatible lighting (maximum nearest distance {:.3})",
            correspondence.fallback_vertices, correspondence.max_fallback_distance
        ));
    }
    if !day_night_components_match(&base.components, &other.components) {
        notes.push("retained LOD component/index metadata was preserved".to_string());
    }
    if !day_night_frames_match(&base.frames, &other.frames) {
        notes.push("retained LOD frame hierarchy was preserved".to_string());
    }
    Ok((merged, notes))
}

struct NearestPrelightTransfer {
    colors: Vec<V3>,
    alphas: Vec<f32>,
    average_nearest_distance: f32,
    maximum_nearest_distance: f32,
}

fn geometry_nearest_prelight_transfer(
    target_vertices: &[V3],
    donor: &RawMesh,
    donor_variant: DayNightVariant,
) -> Result<NearestPrelightTransfer, String> {
    if target_vertices.is_empty() {
        return Err("lighting target has no vertices".to_string());
    }
    if donor.vertices.is_empty() {
        return Err("lighting donor has no vertices".to_string());
    }
    let (donor_colors, donor_alphas) = day_night_variant_stream(donor, donor_variant)?;
    let mut transferred_colors = Vec::with_capacity(target_vertices.len());
    let mut transferred_alphas = Vec::with_capacity(target_vertices.len());
    let mut nearest_distance_total = 0.0f32;
    let mut furthest_nearest_distance = 0.0f32;

    for target_vertex in target_vertices {
        let mut candidates = donor
            .vertices
            .iter()
            .enumerate()
            .map(|(index, donor_vertex)| {
                (
                    index,
                    local_vertex_distance2(*target_vertex, *donor_vertex).sqrt(),
                )
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| left.1.total_cmp(&right.1));
        let nearest_distance = candidates
            .first()
            .map(|(_, distance)| *distance)
            .ok_or_else(|| "lighting donor has no usable vertices".to_string())?;
        nearest_distance_total += nearest_distance;
        furthest_nearest_distance = furthest_nearest_distance.max(nearest_distance);

        let exact = candidates
            .iter()
            .take_while(|(_, distance)| *distance <= DAY_NIGHT_GEOMETRY_EPSILON)
            .copied()
            .collect::<Vec<_>>();
        let sources = if exact.is_empty() {
            candidates
                .into_iter()
                .take(DAY_NIGHT_LOD_BLEND_NEIGHBORS)
                .map(|(index, distance)| (index, 1.0 / distance.max(DAY_NIGHT_GEOMETRY_EPSILON)))
                .collect::<Vec<_>>()
        } else {
            exact
                .into_iter()
                .map(|(index, _)| (index, 1.0))
                .collect::<Vec<_>>()
        };
        let total_weight = sources.iter().map(|(_, weight)| *weight).sum::<f32>();
        if !total_weight.is_finite() || total_weight <= 0.0 {
            return Err("detail-to-LOD lighting transfer produced invalid weights".to_string());
        }
        let mut color = V3::default();
        let mut alpha = 0.0f32;
        for (index, weight) in sources {
            let source_color = donor_colors
                .get(index)
                .ok_or_else(|| format!("lighting donor is missing vertex {index}"))?;
            let source_alpha = donor_alphas
                .get(index)
                .copied()
                .ok_or_else(|| format!("lighting donor is missing alpha {index}"))?;
            color.x += source_color.x * weight;
            color.y += source_color.y * weight;
            color.z += source_color.z * weight;
            alpha += source_alpha * weight;
        }
        transferred_colors.push(V3 {
            x: color.x / total_weight,
            y: color.y / total_weight,
            z: color.z / total_weight,
        });
        transferred_alphas.push(alpha / total_weight);
    }
    Ok(NearestPrelightTransfer {
        colors: transferred_colors,
        alphas: transferred_alphas,
        average_nearest_distance: nearest_distance_total / target_vertices.len() as f32,
        maximum_nearest_distance: furthest_nearest_distance,
    })
}

fn merge_lod_with_detail_prelight(
    lod: &RawMesh,
    detail: &RawMesh,
    lod_variant: DayNightVariant,
) -> Result<(RawMesh, Vec<String>), String> {
    let (lod_colors, lod_alphas) = day_night_variant_stream(lod, lod_variant)?;
    let transferred =
        geometry_nearest_prelight_transfer(&lod.vertices, detail, lod_variant.opposite())?;
    let mut merged = lod.clone();
    if lod_variant == DayNightVariant::Day {
        merged.prelit_colors = lod_colors;
        merged.prelit_alphas = lod_alphas;
        merged.night_prelit_colors = transferred.colors;
        merged.night_prelit_alphas = transferred.alphas;
    } else {
        merged.prelit_colors = transferred.colors;
        merged.prelit_alphas = transferred.alphas;
        merged.night_prelit_colors = lod_colors;
        merged.night_prelit_alphas = lod_alphas;
    }
    Ok((
        merged,
        vec![format!(
            "existing LOD had no counterpart; synthesized {} prelight from the {} detail model using {}-neighbor geometry-aware blending (average nearest distance {:.3}, maximum {:.3})",
            lod_variant.opposite().label(),
            lod_variant.opposite().label(),
            DAY_NIGHT_LOD_BLEND_NEIGHBORS,
            transferred.average_nearest_distance,
            transferred.maximum_nearest_distance
        )],
    ))
}

fn light_lod_from_detail(lod: &RawMesh, detail: &RawMesh) -> Result<(RawMesh, String), String> {
    let day = geometry_nearest_prelight_transfer(&lod.vertices, detail, DayNightVariant::Day)?;
    let night = geometry_nearest_prelight_transfer(&lod.vertices, detail, DayNightVariant::Night)?;
    let mut lit_lod = lod.clone();
    lit_lod.prelit_colors = day.colors;
    lit_lod.prelit_alphas = day.alphas;
    lit_lod.night_prelit_colors = night.colors;
    lit_lod.night_prelit_alphas = night.alphas;
    Ok((
        lit_lod,
        format!(
            "transferred day and night prelight to {} LOD vertices using {}-neighbor geometry-nearest blending (day average/max {:.3}/{:.3}, night average/max {:.3}/{:.3})",
            lod.vertices.len(),
            DAY_NIGHT_LOD_BLEND_NEIGHBORS,
            day.average_nearest_distance,
            day.maximum_nearest_distance,
            night.average_nearest_distance,
            night.maximum_nearest_distance
        ),
    ))
}

fn merge_forced_day_night_prelight(
    target: &RawMesh,
    source: &RawMesh,
    target_variant: DayNightVariant,
    retain_richer_lod: bool,
) -> Result<(RawMesh, Vec<String>), String> {
    let source_is_richer = retain_richer_lod && day_night_source_is_richer_lod(target, source);
    let (base, base_variant, other, retained) = if source_is_richer {
        (
            source,
            target_variant.opposite(),
            target,
            "secondary richer LOD",
        )
    } else {
        (target, target_variant, source, "selected model")
    };
    let (merged, _) = merge_lod_with_detail_prelight(base, other, base_variant)?;
    Ok((
        merged,
        vec![format!(
            "explicit second-click override retained the {retained} ({} vertices, {} triangles) and transferred the opposite prelight with geometry-nearest blending",
            base.vertices.len(),
            base.triangles.len()
        )],
    ))
}

fn day_night_variant_stream(
    raw: &RawMesh,
    variant: DayNightVariant,
) -> Result<(Vec<V3>, Vec<f32>), String> {
    let vertex_count = raw.vertices.len();
    let (colors, alphas) = match variant {
        DayNightVariant::Day => {
            if raw.prelit_colors.len() == vertex_count {
                (&raw.prelit_colors, &raw.prelit_alphas)
            } else {
                (&raw.night_prelit_colors, &raw.night_prelit_alphas)
            }
        }
        DayNightVariant::Night => {
            if raw.night_prelit_colors.len() == vertex_count {
                (&raw.night_prelit_colors, &raw.night_prelit_alphas)
            } else {
                (&raw.prelit_colors, &raw.prelit_alphas)
            }
        }
    };
    if colors.len() != vertex_count {
        return Err(format!(
            "{} model has {} prelight values for {vertex_count} vertices",
            variant.label(),
            colors.len()
        ));
    }
    let alphas = if alphas.len() == vertex_count {
        alphas.clone()
    } else {
        vec![1.0; vertex_count]
    };
    Ok((colors.clone(), alphas))
}

fn mapped_day_night_variant_stream(
    source: &RawMesh,
    variant: DayNightVariant,
    target_to_sources: &[Vec<usize>],
) -> Result<(Vec<V3>, Vec<f32>), String> {
    let (source_colors, source_alphas) = day_night_variant_stream(source, variant)?;
    let mut colors = Vec::with_capacity(target_to_sources.len());
    let mut alphas = Vec::with_capacity(target_to_sources.len());
    for (target_index, source_indices) in target_to_sources.iter().enumerate() {
        let Some(first_index) = source_indices.first().copied() else {
            return Err(format!(
                "target vertex {target_index} has no corresponding source prelight"
            ));
        };
        let first_color = source_colors[first_index];
        let first_alpha = source_alphas[first_index];
        if source_indices.iter().skip(1).any(|source_index| {
            let color = source_colors[*source_index];
            color_channel_to_u8(color.x) != color_channel_to_u8(first_color.x)
                || color_channel_to_u8(color.y) != color_channel_to_u8(first_color.y)
                || color_channel_to_u8(color.z) != color_channel_to_u8(first_color.z)
                || color_channel_to_u8(source_alphas[*source_index])
                    != color_channel_to_u8(first_alpha)
        }) {
            return Err(format!(
                "{} prelight cannot be represented at target vertex {target_index}: equivalent source vertices have different colors",
                variant.label()
            ));
        }
        colors.push(first_color);
        alphas.push(first_alpha);
    }
    Ok((colors, alphas))
}

fn merge_day_night_raw_meshes(
    target: &RawMesh,
    source: &RawMesh,
    target_variant: DayNightVariant,
) -> Result<RawMesh, String> {
    let analysis =
        analyze_day_night_raw_meshes(target, source).map_err(|errors| errors.join(" | "))?;
    let (day_colors, day_alphas) = if target_variant == DayNightVariant::Day {
        day_night_variant_stream(target, DayNightVariant::Day)?
    } else {
        mapped_day_night_variant_stream(source, DayNightVariant::Day, &analysis.target_to_sources)?
    };
    let (night_colors, night_alphas) = if target_variant == DayNightVariant::Night {
        day_night_variant_stream(target, DayNightVariant::Night)?
    } else {
        mapped_day_night_variant_stream(
            source,
            DayNightVariant::Night,
            &analysis.target_to_sources,
        )?
    };
    let mut merged = target.clone();
    merged.prelit_colors = day_colors;
    merged.prelit_alphas = day_alphas;
    merged.night_prelit_colors = night_colors;
    merged.night_prelit_alphas = night_alphas;
    Ok(merged)
}

fn prelight_stream_matches_serialized(expected: &[V3], actual: &[V3]) -> bool {
    expected.len() == actual.len()
        && expected.iter().zip(actual).all(|(expected, actual)| {
            color_channel_to_u8(expected.x) == color_channel_to_u8(actual.x)
                && color_channel_to_u8(expected.y) == color_channel_to_u8(actual.y)
                && color_channel_to_u8(expected.z) == color_channel_to_u8(actual.z)
        })
}

fn prelight_alpha_matches_serialized(expected: &[f32], actual: &[f32]) -> bool {
    expected.len() == actual.len()
        && expected.iter().zip(actual).all(|(expected, actual)| {
            color_channel_to_u8(*expected) == color_channel_to_u8(*actual)
        })
}

fn verify_serialized_day_night_prelight(
    expected: &RawMesh,
    actual: &RawMesh,
    include_normals: bool,
) -> Result<(), String> {
    if actual.prelit_colors.len() != actual.vertices.len()
        || actual.prelit_alphas.len() != actual.vertices.len()
    {
        return Err(format!(
            "serialized day prelight is incomplete ({} colors, {} alpha values, {} vertices)",
            actual.prelit_colors.len(),
            actual.prelit_alphas.len(),
            actual.vertices.len()
        ));
    }
    if actual.night_prelit_colors.len() != actual.vertices.len()
        || actual.night_prelit_alphas.len() != actual.vertices.len()
    {
        return Err(format!(
            "serialized night prelight is incomplete ({} colors, {} alpha values, {} vertices)",
            actual.night_prelit_colors.len(),
            actual.night_prelit_alphas.len(),
            actual.vertices.len()
        ));
    }

    // Building export may intentionally omit the stored normal stream, and the
    // parser may reconstruct normals and legacy per-vertex light flags on read.
    // Their presence is therefore not a stable round-trip identity. Geometry,
    // materials, UVs, and every sidecar remain part of the comparison.
    let expected = raw_mesh_after_normalized_export(expected, include_normals);
    let mut expected_geometry = expected.clone();
    let mut actual_geometry = actual.clone();
    expected_geometry.normals.clear();
    actual_geometry.normals.clear();
    expected_geometry.light_flags.clear();
    actual_geometry.light_flags.clear();
    let analysis = analyze_day_night_raw_meshes_with_prelight_tiebreak(
        &expected_geometry,
        &actual_geometry,
        true,
    )
    .map_err(|differences| differences.join(" | "))?;

    if (expected.uvs.len() == expected.vertices.len())
        != (actual.uvs.len() == actual.vertices.len())
    {
        return Err("serialized primary UV stream presence changed".to_string());
    }
    if let Some((mismatches, max_delta)) =
        mapped_uv_difference(&expected.uvs, &actual.uvs, &analysis.target_to_sources)
        && mismatches > 0
    {
        return Err(format!(
            "serialized primary UVs changed at {mismatches} mapped vertex pair(s), maximum delta {max_delta:.6}"
        ));
    }
    if expected.secondary_uvs.len() != actual.secondary_uvs.len() {
        return Err(format!(
            "serialized secondary UV stream count changed ({} vs {})",
            expected.secondary_uvs.len(),
            actual.secondary_uvs.len()
        ));
    }
    for (stream, (expected_uvs, actual_uvs)) in expected
        .secondary_uvs
        .iter()
        .zip(&actual.secondary_uvs)
        .enumerate()
    {
        if (expected_uvs.len() == expected.vertices.len())
            != (actual_uvs.len() == actual.vertices.len())
        {
            return Err(format!(
                "serialized secondary UV stream {} presence changed",
                stream + 1
            ));
        }
        if let Some((mismatches, max_delta)) =
            mapped_uv_difference(expected_uvs, actual_uvs, &analysis.target_to_sources)
            && mismatches > 0
        {
            return Err(format!(
                "serialized secondary UV stream {} changed at {mismatches} mapped vertex pair(s), maximum delta {max_delta:.6}",
                stream + 1
            ));
        }
    }

    for variant in [DayNightVariant::Day, DayNightVariant::Night] {
        let (mapped_colors, mapped_alphas) =
            mapped_day_night_variant_stream(actual, variant, &analysis.target_to_sources)?;
        let (expected_colors, expected_alphas) = day_night_variant_stream(&expected, variant)?;
        if !prelight_stream_matches_serialized(&expected_colors, &mapped_colors) {
            return Err(format!(
                "serialized {} prelight colors changed after canonical vertex mapping",
                variant.label()
            ));
        }
        if !prelight_alpha_matches_serialized(&expected_alphas, &mapped_alphas) {
            return Err(format!(
                "serialized {} prelight alpha changed after canonical vertex mapping",
                variant.label()
            ));
        }
    }
    Ok(())
}

#[derive(Clone)]
struct DayNightMergePair {
    target_index: usize,
    source_index: usize,
    target_variant: DayNightVariant,
    is_lod: bool,
    kind: DayNightMergePairKind,
    delete_source: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DayNightMergePairKind {
    Standard,
    LodFromDetail,
}

struct DayNightLodAssignmentUpdate {
    child_index: usize,
    parent_id: Option<String>,
    parent_unique_id: Option<String>,
    required_pair_target_index: usize,
}

struct DayNightMergePlan {
    pairs: Vec<DayNightMergePair>,
    reviews: Vec<String>,
    lod_assignment_updates: Vec<DayNightLodAssignmentUpdate>,
}

struct DayNightMergeContext {
    root: PathBuf,
    gta_sa_dir: PathBuf,
    selected_index: usize,
    explicit_source_index: Option<usize>,
    placements: Vec<Placement>,
    states: Vec<ElementState>,
    definitions: HashMap<String, Definition>,
    readonly_definition_ids: HashSet<String>,
    byte_overrides: BTreeMap<String, (String, Vec<u8>)>,
    raw_overrides: BTreeMap<String, (String, RawMesh)>,
    vertex_mesh_overrides: BTreeMap<String, Vec<(String, RenderMesh)>>,
    building_dffs: HashSet<String>,
    tolerance: f32,
    force_review_override: bool,
}

#[derive(Clone)]
struct DayNightMergeReplacement {
    pair: DayNightMergePair,
    target_name: String,
    source_name: String,
    target_definition_id: String,
    renamed_target_id: Option<String>,
    bytes: Vec<u8>,
    raw: RawMesh,
    preserved_vertex_mesh_keys: Vec<String>,
    notes: Vec<String>,
}

struct DayNightMergeResult {
    replacements: Vec<DayNightMergeReplacement>,
    expected_placements: Vec<(usize, Placement)>,
    delete_indices: Vec<usize>,
    target_definition_ids: Vec<String>,
    lod_assignment_updates: Vec<(usize, Option<String>, Option<String>)>,
    reviews: Vec<String>,
    texture_files: HashMap<String, PathBuf>,
    override_available: bool,
}

fn read_day_night_raw(
    context: &DayNightMergeContext,
    dff_name: &str,
) -> Result<(RawMesh, Vec<String>), String> {
    let key = asset_key(dff_name, ".dff");
    let mut raw = if let Some((_, raw)) = context.raw_overrides.get(&key) {
        raw.clone()
    } else {
        let bytes = if let Some((_, bytes)) = context.byte_overrides.get(&key) {
            bytes.clone()
        } else {
            let mut entry = replacement_img_entry(&wip_root_path(&context.root), &key)
                .or_else(|| replacement_img_entry(&context.root, &key));
            if entry.is_none() {
                'outer: for path in collect_resource_img_files(&context.root)
                    .into_iter()
                    .chain(gta_sa_img_files(&context.gta_sa_dir))
                {
                    for candidate in parse_img(&path) {
                        if asset_key(&candidate.name, ".dff") == key {
                            entry = Some(candidate);
                            break 'outer;
                        }
                    }
                }
            }
            let entry = entry.ok_or_else(|| format!("Could not find DFF {dff_name}"))?;
            let bytes = read_img_entry(&entry);
            if bytes.is_empty() {
                return Err(format!("Could not read DFF {dff_name}"));
            }
            bytes
        };
        parse_dff_mesh(&bytes[..dff_chunk_len(&bytes).min(bytes.len())])
    };
    if raw.vertices.is_empty() || raw.triangles.is_empty() {
        return Err(format!("{dff_name} has no readable DFF geometry"));
    }
    let mut preserved = Vec::new();
    if let Some(runtime_meshes) = context.vertex_mesh_overrides.get(&key) {
        let refs = runtime_meshes
            .iter()
            .map(|(_, mesh)| mesh)
            .collect::<Vec<_>>();
        apply_runtime_vertex_meshes_to_raw(&refs, &mut raw).map_err(|error| {
            format!("Could not preserve unsaved prelight for {dff_name}: {error}")
        })?;
        preserved.extend(runtime_meshes.iter().map(|(mesh_key, _)| mesh_key.clone()));
    }
    Ok((raw, preserved))
}

fn external_day_night_lod_reference_indices(
    context: &DayNightMergeContext,
    lod_index: usize,
    scheduled_deletes: &HashSet<usize>,
) -> Vec<usize> {
    let Some(lod) = context.placements.get(lod_index) else {
        return Vec::new();
    };
    context
        .placements
        .iter()
        .enumerate()
        .filter(|(index, placement)| {
            !scheduled_deletes.contains(index)
                && day_night_merge_live(&context.states, *index)
                && placement
                    .attrs
                    .get("lodParent")
                    .is_some_and(|parent| parent.eq_ignore_ascii_case(&lod.id))
                && resolve_assigned_lod(&context.placements, &context.states, *index)
                    .is_ok_and(|resolved| resolved == Some(lod_index))
        })
        .map(|(index, _)| index)
        .collect()
}

fn day_night_lod_unique_id(context: &DayNightMergeContext, lod_index: usize) -> Option<String> {
    context
        .placements
        .get(lod_index)
        .and_then(|placement| placement.attrs.get("uniqueID"))
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn planned_day_night_target_id(
    context: &DayNightMergeContext,
    pair: &DayNightMergePair,
    reserved_ids: &mut HashSet<String>,
) -> Result<Option<String>, String> {
    let target = &context.placements[pair.target_index];
    let source = &context.placements[pair.source_index];
    let Some(base_id) = day_night_base_id(&target.id) else {
        return Ok(None);
    };
    let available = |candidate: &str, reserved_ids: &HashSet<String>| {
        !reserved_ids.contains(&candidate.to_ascii_lowercase())
            && !context
                .placements
                .iter()
                .enumerate()
                .any(|(index, placement)| {
                    index != pair.target_index
                        && index != pair.source_index
                        && day_night_merge_live(&context.states, index)
                        && placement.id.eq_ignore_ascii_case(candidate)
                })
            && !context.definitions.values().any(|definition| {
                definition.id.eq_ignore_ascii_case(candidate)
                    && !definition.id.eq_ignore_ascii_case(&target.id)
                    && !definition.id.eq_ignore_ascii_case(&source.id)
            })
    };
    let new_id = if available(&base_id, reserved_ids) {
        base_id
    } else {
        let mut suffix_index = 0usize;
        loop {
            let mut value = suffix_index;
            let mut suffix = String::new();
            loop {
                suffix.insert(0, (b'A' + (value % 26) as u8) as char);
                if value < 26 {
                    break;
                }
                value = value / 26 - 1;
            }
            let candidate = format!("{base_id}_{suffix}");
            if available(&candidate, reserved_ids) {
                break candidate;
            }
            suffix_index += 1;
        }
    };
    reserved_ids.insert(new_id.to_ascii_lowercase());
    Ok(Some(new_id))
}

fn plan_day_night_merge_pairs(context: &DayNightMergeContext) -> Result<DayNightMergePlan, String> {
    let source_index = if let Some(source_index) = context.explicit_source_index {
        if source_index == context.selected_index
            || !day_night_merge_live(&context.states, source_index)
            || context.placements.get(source_index).is_none()
        {
            return Err("The explicitly selected source object no longer exists".to_string());
        }
        source_index
    } else {
        find_day_night_counterpart(
            &context.placements,
            &context.states,
            context.selected_index,
            context.tolerance,
        )?
    };
    let target = &context.placements[context.selected_index];
    let source = &context.placements[source_index];
    let target_variant = if context.explicit_source_index.is_some() {
        explicit_pair_target_variant(target, source, &context.definitions)
    } else {
        placement_day_night_variant(target)
            .ok_or_else(|| format!("{} has no recognized day/night suffix", target.id))?
            .variant
    };
    let lod_ids = collect_lod_ids(&context.placements);
    let selected_is_lod = placement_is_lod(target, &lod_ids);
    if context.explicit_source_index.is_some()
        && placement_is_lod(source, &lod_ids) != selected_is_lod
    {
        return Err(
            "The explicitly selected pair must both be detail objects or both be LOD objects"
                .to_string(),
        );
    }
    let mut pairs = vec![DayNightMergePair {
        target_index: context.selected_index,
        source_index,
        target_variant,
        is_lod: selected_is_lod,
        kind: DayNightMergePairKind::Standard,
        delete_source: true,
    }];
    let mut plan = DayNightMergePlan {
        pairs: Vec::new(),
        reviews: Vec::new(),
        lod_assignment_updates: Vec::new(),
    };
    if selected_is_lod {
        let scheduled = HashSet::from([source_index]);
        let target_parent_id = target.id.clone();
        let target_unique_id = day_night_lod_unique_id(context, context.selected_index);
        for child_index in
            external_day_night_lod_reference_indices(context, source_index, &scheduled)
        {
            plan.lod_assignment_updates
                .push(DayNightLodAssignmentUpdate {
                    child_index,
                    parent_id: Some(target_parent_id.clone()),
                    parent_unique_id: target_unique_id.clone(),
                    required_pair_target_index: context.selected_index,
                });
        }
        plan.pairs = pairs;
        return Ok(plan);
    }

    let target_lod =
        resolve_assigned_lod(&context.placements, &context.states, context.selected_index);
    let source_lod = resolve_assigned_lod(&context.placements, &context.states, source_index);
    for resolution in [&target_lod, &source_lod] {
        if let Err(error) = resolution
            && !error.contains(" references missing LOD ")
        {
            return Err(error.clone());
        }
    }
    match (target_lod, source_lod) {
        (Ok(None), Ok(None)) => {}
        (Ok(Some(target_lod)), Ok(Some(source_lod))) if target_lod == source_lod => {}
        (Ok(Some(target_lod)), Ok(Some(source_lod))) => {
            let scheduled = HashSet::from([source_index, source_lod]);
            let target_parent_id = context.placements[target_lod].id.clone();
            let target_unique_id = day_night_lod_unique_id(context, target_lod);
            for child_index in
                external_day_night_lod_reference_indices(context, source_lod, &scheduled)
            {
                plan.lod_assignment_updates
                    .push(DayNightLodAssignmentUpdate {
                        child_index,
                        parent_id: Some(target_parent_id.clone()),
                        parent_unique_id: target_unique_id.clone(),
                        required_pair_target_index: target_lod,
                    });
            }
            pairs.push(DayNightMergePair {
                target_index: target_lod,
                source_index: source_lod,
                target_variant,
                is_lod: true,
                kind: DayNightMergePairKind::Standard,
                delete_source: true,
            });
        }
        (Ok(Some(target_lod)), _) => {
            pairs.push(DayNightMergePair {
                target_index: target_lod,
                source_index,
                target_variant,
                is_lod: true,
                kind: DayNightMergePairKind::LodFromDetail,
                delete_source: false,
            });
            plan.lod_assignment_updates
                .push(DayNightLodAssignmentUpdate {
                    child_index: context.selected_index,
                    parent_id: Some(context.placements[target_lod].id.clone()),
                    parent_unique_id: day_night_lod_unique_id(context, target_lod),
                    required_pair_target_index: target_lod,
                });
        }
        (_, Ok(Some(source_lod))) => {
            pairs.push(DayNightMergePair {
                target_index: source_lod,
                source_index: context.selected_index,
                target_variant: target_variant.opposite(),
                is_lod: true,
                kind: DayNightMergePairKind::LodFromDetail,
                delete_source: false,
            });
            plan.lod_assignment_updates
                .push(DayNightLodAssignmentUpdate {
                    child_index: context.selected_index,
                    parent_id: Some(context.placements[source_lod].id.clone()),
                    parent_unique_id: day_night_lod_unique_id(context, source_lod),
                    required_pair_target_index: source_lod,
                });
        }
        (target_missing, source_missing) => {
            let errors = [target_missing.err(), source_missing.err()]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            plan.reviews.push(format!(
                "Review required: neither day/night object has a usable assigned LOD ({}). The detail merge continued and the retained object's LOD assignment was removed.",
                errors.join(" | ")
            ));
            plan.lod_assignment_updates
                .push(DayNightLodAssignmentUpdate {
                    child_index: context.selected_index,
                    parent_id: None,
                    parent_unique_id: None,
                    required_pair_target_index: context.selected_index,
                });
        }
    }
    plan.pairs = pairs;
    Ok(plan)
}

fn run_day_night_merge(
    context: DayNightMergeContext,
    progress: &mpsc::Sender<String>,
) -> DayNightMergeResult {
    let mut result = DayNightMergeResult {
        replacements: Vec::new(),
        expected_placements: Vec::new(),
        delete_indices: Vec::new(),
        target_definition_ids: Vec::new(),
        lod_assignment_updates: Vec::new(),
        reviews: Vec::new(),
        texture_files: HashMap::new(),
        override_available: false,
    };
    let plan = match plan_day_night_merge_pairs(&context) {
        Ok(plan) => plan,
        Err(error) => {
            result.reviews.push(format!("Review required: {error}"));
            return result;
        }
    };
    let DayNightMergePlan {
        pairs,
        reviews,
        lod_assignment_updates,
    } = plan;
    result.reviews.extend(reviews);
    let mut prepared = Vec::new();
    let mut reserved_rename_ids = HashSet::new();
    for (pair_number, pair) in pairs.iter().enumerate() {
        let target = &context.placements[pair.target_index];
        let source = &context.placements[pair.source_index];
        let pair_kind = match pair.kind {
            DayNightMergePairKind::Standard if pair.is_lod => "LOD",
            DayNightMergePairKind::Standard => "detail",
            DayNightMergePairKind::LodFromDetail => "LOD-from-detail fallback",
        };
        let _ = progress.send(format!(
            "Day/night merge: comparing {pair_kind} pair {}/{} ({} + {})",
            pair_number + 1,
            pairs.len(),
            target.id,
            source.id
        ));
        let mut differences = if pair.kind == DayNightMergePairKind::Standard {
            compare_day_night_placement_pair(target, source, &context.definitions)
        } else {
            Vec::new()
        };
        let mut force_blockers = Vec::new();
        let renamed_target_id =
            match planned_day_night_target_id(&context, pair, &mut reserved_rename_ids) {
                Ok(renamed_target_id) => renamed_target_id,
                Err(error) => {
                    differences.push(error.clone());
                    force_blockers.push(error);
                    None
                }
            };
        let placement_distance = day_night_position_distance2(target.pos, source.pos).sqrt();
        if context.explicit_source_index.is_none()
            && pair.kind == DayNightMergePairKind::Standard
            && placement_distance > context.tolerance
        {
            differences.push(format!(
                "placement distance {placement_distance:.6} exceeds tolerance {:.6}",
                context.tolerance
            ));
        }
        if context.explicit_source_index.is_none()
            && let Some(target_name) = placement_day_night_variant(target)
            && target_name.variant != pair.target_variant
        {
            let error = format!(
                "{} suffix identifies it as {}, but its paired detail model is {}",
                target.id,
                target_name.variant.label(),
                pair.target_variant.label()
            );
            differences.push(error.clone());
            force_blockers.push(error);
        }
        if context.explicit_source_index.is_none()
            && let Some(source_name) = placement_day_night_variant(source)
            && source_name.variant != pair.target_variant.opposite()
        {
            let error = format!(
                "{} suffix identifies it as {}, but the required source is {}",
                source.id,
                source_name.variant.label(),
                pair.target_variant.opposite().label()
            );
            differences.push(error.clone());
            force_blockers.push(error);
        }
        if context.readonly_definition_ids.contains(&target.id)
            && context
                .definitions
                .get(&target.id)
                .is_some_and(definition_has_day_night_time_window)
        {
            let error =
                "selected definition is read-only and its day/night time window cannot be removed"
                    .to_string();
            differences.push(error.clone());
            force_blockers.push(error);
        }
        let target_raw = read_day_night_raw(&context, &target.dff);
        let source_raw = read_day_night_raw(&context, &source.dff);
        let (
            (target_raw, mut preserved_vertex_mesh_keys),
            (source_raw, source_preserved_mesh_keys),
        ) = match (target_raw, source_raw) {
            (Ok(target_raw), Ok(source_raw)) => (target_raw, source_raw),
            (target_result, source_result) => {
                if let Err(error) = target_result {
                    differences.push(error);
                }
                if let Err(error) = source_result {
                    differences.push(error);
                }
                result.reviews.push(format!(
                    "Review required for {} + {}: {}. Both objects were preserved.",
                    target.id,
                    source.id,
                    differences.join(" | ")
                ));
                if pair.is_lod {
                    continue;
                }
                return result;
            }
        };
        preserved_vertex_mesh_keys.extend(source_preserved_mesh_keys);
        preserved_vertex_mesh_keys.sort();
        preserved_vertex_mesh_keys.dedup();

        let mut notes = Vec::new();
        let mut merged = None;
        if differences.is_empty() {
            if pair.kind == DayNightMergePairKind::LodFromDetail {
                match merge_lod_with_detail_prelight(&target_raw, &source_raw, pair.target_variant)
                {
                    Ok((lod_merged, lod_notes)) => {
                        merged = Some(lod_merged);
                        notes = lod_notes;
                    }
                    Err(error) => differences.push(format!(
                        "detail-to-LOD lighting fallback was not safe: {error}"
                    )),
                }
            } else {
                match analyze_day_night_raw_meshes(&target_raw, &source_raw) {
                    Ok(analysis) => {
                        notes = analysis.notes;
                        match merge_day_night_raw_meshes(
                            &target_raw,
                            &source_raw,
                            pair.target_variant,
                        ) {
                            Ok(strict_merged) => merged = Some(strict_merged),
                            Err(error) => differences.push(error),
                        }
                    }
                    Err(raw_differences) if pair.is_lod => {
                        match merge_near_matching_day_night_lods(
                            &target_raw,
                            &source_raw,
                            pair.target_variant,
                        ) {
                            Ok((lod_merged, lod_notes)) => {
                                merged = Some(lod_merged);
                                notes = lod_notes;
                            }
                            Err(error) => {
                                differences.extend(raw_differences);
                                differences.push(format!(
                                    "near-matching LOD lighting fallback was not safe: {error}"
                                ));
                            }
                        }
                    }
                    Err(raw_differences) => {
                        match merge_surface_equivalent_day_night_details(
                            &target_raw,
                            &source_raw,
                            pair.target_variant,
                        ) {
                            Ok((detail_merged, detail_notes)) => {
                                merged = Some(detail_merged);
                                notes = detail_notes;
                            }
                            Err(error) => {
                                differences.extend(raw_differences);
                                differences.push(format!(
                                    "surface-equivalent detail lighting fallback was not safe: {error}"
                                ));
                            }
                        }
                    }
                }
            }
        }
        if !differences.is_empty()
            && context.force_review_override
            && force_blockers.is_empty()
            && pair.kind == DayNightMergePairKind::Standard
        {
            let accepted_differences = differences.join(" | ");
            match merge_forced_day_night_prelight(
                &target_raw,
                &source_raw,
                pair.target_variant,
                pair.is_lod,
            ) {
                Ok((forced_merged, mut forced_notes)) => {
                    forced_notes.insert(
                        0,
                        format!(
                            "explicit second-click override accepted these review differences: {accepted_differences}"
                        ),
                    );
                    merged = Some(forced_merged);
                    notes = forced_notes;
                    differences.clear();
                }
                Err(error) => differences.push(format!(
                    "explicit second-click override could not safely transfer prelight: {error}"
                )),
            }
        }
        if !differences.is_empty() {
            if !context.force_review_override && force_blockers.is_empty() {
                result.override_available = true;
                if !prepared.is_empty() {
                    prepared.clear();
                }
            }
            result.reviews.push(format!(
                "Review required for {} + {}: {}. Automatic geometry/lighting reconciliation was not safe; both objects were preserved.",
                target.id,
                source.id,
                differences.join(" | ")
            ));
            if result.override_available {
                return result;
            }
            if pair.is_lod {
                continue;
            }
            return result;
        }
        let merged = merged.expect("successful day/night reconciliation produces a mesh");
        let frame = Path::new(&target.dff)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("model");
        let options = DffWriteOptions {
            include_normals: !context
                .building_dffs
                .contains(&asset_key(&target.dff, ".dff")),
            include_bin_mesh: true,
        };
        let bytes = match write_normalized_dff_with_options(&merged, frame, options) {
            Ok(bytes) => bytes,
            Err(error) => {
                result.reviews.push(format!(
                    "Review required for {} + {}: merged DFF could not be serialized: {error}. Both objects were preserved.",
                    target.id, source.id
                ));
                if pair.is_lod {
                    continue;
                }
                return result;
            }
        };
        let normalized = parse_dff_mesh(&bytes);
        if let Err(error) =
            verify_serialized_day_night_prelight(&merged, &normalized, options.include_normals)
        {
            result.reviews.push(format!(
                "Review required for {} + {}: serialized DFF verification failed: {error}. Both objects were preserved.",
                target.id, source.id,
            ));
            if pair.is_lod {
                continue;
            }
            return result;
        }
        prepared.push(DayNightMergeReplacement {
            pair: pair.clone(),
            target_name: with_ext(&target.dff, ".dff"),
            source_name: with_ext(&source.dff, ".dff"),
            target_definition_id: target.id.clone(),
            renamed_target_id,
            bytes,
            raw: normalized,
            preserved_vertex_mesh_keys,
            notes,
        });
    }

    result.texture_files = collect_texture_files(&context.root);
    let mut expected_indices = BTreeSet::new();
    for replacement in &prepared {
        expected_indices.insert(replacement.pair.target_index);
        expected_indices.insert(replacement.pair.source_index);
        if replacement.pair.delete_source {
            result.delete_indices.push(replacement.pair.source_index);
        }
        result
            .target_definition_ids
            .push(replacement.target_definition_id.clone());
    }
    result.delete_indices.sort_unstable();
    result.delete_indices.dedup();
    result.target_definition_ids.sort();
    result.target_definition_ids.dedup();
    let successful_pair_targets = prepared
        .iter()
        .map(|replacement| replacement.pair.target_index)
        .collect::<HashSet<_>>();
    result.lod_assignment_updates = lod_assignment_updates
        .into_iter()
        .filter(|update| successful_pair_targets.contains(&update.required_pair_target_index))
        .map(|update| {
            (
                update.child_index,
                update.parent_id,
                update.parent_unique_id,
            )
        })
        .collect();
    expected_indices.extend(
        result
            .lod_assignment_updates
            .iter()
            .map(|(child_index, _, _)| *child_index),
    );
    result.expected_placements = expected_indices
        .into_iter()
        .filter_map(|index| {
            context
                .placements
                .get(index)
                .cloned()
                .map(|placement| (index, placement))
        })
        .collect();
    result.replacements = prepared;
    result
}

fn apply_successful_day_night_id_renames(
    placements: &mut [Placement],
    states: &[ElementState],
    definitions: &mut HashMap<String, Definition>,
    replacements: &[DayNightMergeReplacement],
) -> Vec<(String, String)> {
    let mut obsolete_definition_ids = replacements
        .iter()
        .flat_map(|replacement| {
            [
                Some(replacement.target_definition_id.clone()),
                placements
                    .get(replacement.pair.source_index)
                    .map(|source| source.id.clone()),
            ]
        })
        .flatten()
        .collect::<BTreeSet<_>>();
    let mut renamed = Vec::new();
    for replacement in replacements {
        let Some(new_id) = replacement.renamed_target_id.as_ref() else {
            continue;
        };
        let Some(target) = placements.get_mut(replacement.pair.target_index) else {
            continue;
        };
        let old_id = target.id.clone();
        let mut definition = definitions
            .get(&old_id)
            .cloned()
            .unwrap_or_else(|| Definition {
                id: old_id.clone(),
                zone: target.zone.clone(),
                attrs: BTreeMap::new(),
            });
        definition.id = new_id.clone();
        definition.attrs.insert("id".to_string(), new_id.clone());
        definition
            .attrs
            .insert("dff".to_string(), target.dff.clone());
        remove_definition_day_night_time_window(&mut definition);
        definitions.insert(new_id.clone(), definition);

        target.id = new_id.clone();
        sync_placement_attrs(target);
        renamed.push((old_id, new_id.clone()));
    }

    for (index, placement) in placements.iter_mut().enumerate() {
        if !day_night_merge_live(states, index) {
            continue;
        }
        let Some(parent) = placement.attrs.get("lodParent").cloned() else {
            continue;
        };
        if let Some((_, new_id)) = renamed
            .iter()
            .find(|(old_id, _)| parent.eq_ignore_ascii_case(old_id))
        {
            placement
                .attrs
                .insert("lodParent".to_string(), new_id.clone());
        }
    }
    obsolete_definition_ids.retain(|id| {
        !placements.iter().enumerate().any(|(index, placement)| {
            day_night_merge_live(states, index) && placement.id.eq_ignore_ascii_case(id)
        })
    });
    for id in obsolete_definition_ids {
        definitions.remove(&id);
    }
    renamed
}

pub(crate) struct DayNightMergeJob {
    rx: mpsc::Receiver<DayNightMergeResult>,
    progress_rx: mpsc::Receiver<String>,
    result: Option<DayNightMergeResult>,
    override_key: Option<DayNightMergeOverrideKey>,
    forced: bool,
    apply_index: usize,
    lod_update_index: usize,
    validated: bool,
    refreshed_any: bool,
    started_at: Instant,
}

const DAY_NIGHT_MERGE_APPLY_LIMIT: usize = 2;
const DAY_NIGHT_LOD_UPDATE_APPLY_LIMIT: usize = 64;
const DAY_NIGHT_MERGE_FRAME_BUDGET: Duration = Duration::from_millis(5);

impl DayNightMergeJob {
    fn new(
        context: DayNightMergeContext,
        override_key: Option<DayNightMergeOverrideKey>,
        forced: bool,
    ) -> Self {
        let (tx, rx) = mpsc::channel();
        let (progress_tx, progress_rx) = mpsc::channel();
        thread::spawn(move || {
            let result = std::panic::catch_unwind(|| run_day_night_merge(context, &progress_tx))
                .unwrap_or_else(|panic| {
                    let detail = panic
                        .downcast_ref::<&str>()
                        .map(|message| (*message).to_string())
                        .or_else(|| panic.downcast_ref::<String>().cloned())
                        .unwrap_or_else(|| "unknown merge worker panic".to_string());
                    DayNightMergeResult {
                        replacements: Vec::new(),
                        expected_placements: Vec::new(),
                        delete_indices: Vec::new(),
                        target_definition_ids: Vec::new(),
                        lod_assignment_updates: Vec::new(),
                        reviews: vec![format!(
                            "Day/night merge worker stopped unexpectedly: {detail}"
                        )],
                        texture_files: HashMap::new(),
                        override_available: false,
                    }
                });
            let _ = tx.send(result);
        });
        Self {
            rx,
            progress_rx,
            result: None,
            override_key,
            forced,
            apply_index: 0,
            lod_update_index: 0,
            validated: false,
            refreshed_any: false,
            started_at: Instant::now(),
        }
    }

    fn step(&mut self, app: &mut AppState) -> bool {
        if let Some(message) = self.progress_rx.try_iter().last() {
            app.status_message = message;
        }
        if self.result.is_none() {
            match self.rx.try_recv() {
                Ok(result) => self.result = Some(result),
                Err(mpsc::TryRecvError::Empty) => return false,
                Err(mpsc::TryRecvError::Disconnected) => {
                    app.status_message = "Day/night merge worker disconnected".to_string();
                    set_save_log(
                        app,
                        "Day/Night Variant Merge",
                        vec![app.status_message.clone()],
                        true,
                    );
                    return true;
                }
            }
        }
        let result = self.result.as_ref().expect("day/night merge result is set");
        if !self.validated {
            let stale = result.expected_placements.iter().any(|(index, expected)| {
                app.placements.get(*index) != Some(expected)
                    || app
                        .element_states
                        .get(*index)
                        .is_some_and(|state| state.deleted)
            });
            if stale {
                app.status_message =
                    "Day/night merge cancelled because a paired object changed".to_string();
                set_save_log(
                    app,
                    "Day/Night Variant Merge",
                    vec![app.status_message.clone()],
                    true,
                );
                return true;
            }
            self.validated = true;
        }
        if self.apply_index < result.replacements.len() {
            let started = Instant::now();
            let mut applied = 0usize;
            while let Some(replacement) = result.replacements.get(self.apply_index) {
                let key = asset_key(&replacement.target_name, ".dff");
                let open_dirty = matches!(
                    app.editing.asset.as_ref(),
                    Some(EditingAsset::Dff(dff))
                        if asset_key(&dff.name, ".dff") == key && dff.dirty
                );
                if open_dirty || app.editing.modified_entries.contains_key(&key) {
                    app.editing
                        .modified_entries
                        .insert(key.clone(), replacement.bytes.clone());
                } else {
                    app.pending_replacement_assets.insert(
                        key.clone(),
                        (replacement.target_name.clone(), replacement.bytes.clone()),
                    );
                }
                if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut()
                    && asset_key(&dff.name, ".dff") == key
                {
                    dff.raw = replacement.raw.clone();
                    dff.dirty = false;
                }
                for mesh_key in &replacement.preserved_vertex_mesh_keys {
                    app.pending_vertex_light_meshes.remove(mesh_key);
                    app.vertex_paint_dirty_meshes.remove(mesh_key);
                }
                self.refreshed_any |= refresh_live_dff_from_raw(
                    app,
                    &replacement.target_name,
                    &replacement.raw,
                    &result.texture_files,
                );
                self.apply_index += 1;
                applied += 1;
                if applied >= DAY_NIGHT_MERGE_APPLY_LIMIT
                    || started.elapsed() >= DAY_NIGHT_MERGE_FRAME_BUDGET
                {
                    break;
                }
            }
            if self.refreshed_any {
                rebuild_render_cells(app);
                self.refreshed_any = false;
            }
            app.status_message = format!(
                "Day/night merge: applying model {}/{}",
                self.apply_index,
                result.replacements.len()
            );
            return false;
        }

        if self.lod_update_index < result.lod_assignment_updates.len() {
            let started = Instant::now();
            let mut applied = 0usize;
            while let Some((child_index, parent_id, parent_unique_id)) =
                result.lod_assignment_updates.get(self.lod_update_index)
            {
                if day_night_merge_live(&app.element_states, *child_index)
                    && let Some(child) = app.placements.get_mut(*child_index)
                {
                    if let Some(parent_id) = parent_id {
                        child
                            .attrs
                            .insert("lodParent".to_string(), parent_id.clone());
                        if let Some(parent_unique_id) = parent_unique_id {
                            child
                                .attrs
                                .insert("uniqueID".to_string(), parent_unique_id.clone());
                        }
                    } else {
                        child.attrs.remove("lodParent");
                    }
                }
                self.lod_update_index += 1;
                applied += 1;
                if applied >= DAY_NIGHT_LOD_UPDATE_APPLY_LIMIT
                    || started.elapsed() >= DAY_NIGHT_MERGE_FRAME_BUDGET
                {
                    break;
                }
            }
            app.status_message = format!(
                "Day/night merge: reassigning LOD reference {}/{}",
                self.lod_update_index,
                result.lod_assignment_updates.len()
            );
            return false;
        }

        let mut renamed_count = 0usize;
        if !result.replacements.is_empty() {
            for index in &result.delete_indices {
                if let Some(state) = app.element_states.get_mut(*index) {
                    state.deleted = true;
                }
            }
            for id in &result.target_definition_ids {
                if let Some(definition) = app.definitions.get_mut(id) {
                    remove_definition_day_night_time_window(definition);
                }
            }
            renamed_count = apply_successful_day_night_id_renames(
                &mut app.placements,
                &app.element_states,
                &mut app.definitions,
                &result.replacements,
            )
            .len();
            invalidate_outliner_labels(app);
            rebuild_outliner_filter(app);
            invalidate_validation_cache(app);
            rebuild_render_cells(app);
            clear_history_for_external_change(app);
            app.loaded_wip = true;
        }
        let elapsed = self.started_at.elapsed().as_secs_f32();
        let mut log = result
            .replacements
            .iter()
            .flat_map(|replacement| {
                let source_action = if replacement.pair.delete_source {
                    format!(
                        "deleted duplicate placement {}",
                        app.placements
                            .get(replacement.pair.source_index)
                            .map(|placement| placement.id.as_str())
                            .unwrap_or("unknown")
                    )
                } else {
                    format!(
                        "retained LOD {} and transferred missing prelight from detail {}",
                        app.placements
                            .get(replacement.pair.target_index)
                            .map(|placement| placement.id.as_str())
                            .unwrap_or("unknown"),
                        app.placements
                            .get(replacement.pair.source_index)
                            .map(|placement| placement.id.as_str())
                            .unwrap_or("unknown")
                    )
                };
                let mut lines = vec![format!(
                    "Merged {} + {} into {} with complete day/night prelight; {source_action}.{}",
                    replacement.target_name,
                    replacement.source_name,
                    replacement.target_name,
                    replacement
                        .renamed_target_id
                        .as_ref()
                        .map(|id| format!(" Renamed the retained object to {id}."))
                        .unwrap_or_default()
                )];
                lines.extend(replacement.notes.iter().map(|note| {
                    format!(
                        "Merge note for {} + {}: {note}.",
                        replacement.target_name, replacement.source_name
                    )
                }));
                lines
            })
            .collect::<Vec<_>>();
        if !result.lod_assignment_updates.is_empty() {
            log.push(format!(
                "Updated {} LOD assignment(s) to follow the retained merged model.",
                result.lod_assignment_updates.len()
            ));
        }
        log.extend(result.reviews.iter().cloned());
        if result.override_available && !self.forced {
            app.day_night_merge_override = self.override_key.clone();
            let retry_message = "Click Merge Vertex Lighting again on this same pair to override the reported model-difference checks.";
            log.push(retry_message.to_string());
        } else if app.day_night_merge_override == self.override_key {
            app.day_night_merge_override = None;
        }
        if result.replacements.is_empty() {
            app.status_message = format!(
                "Day/night merge made no changes; {} review issue(s) were reported ({elapsed:.1}s)",
                result.reviews.len()
            );
        } else {
            let rename_summary = if renamed_count == 0 {
                String::new()
            } else {
                format!(", removed variant suffixes from {renamed_count} retained object(s)")
            };
            let lod_assignment_summary = if result.lod_assignment_updates.is_empty() {
                String::new()
            } else {
                format!(
                    ", updated {} LOD assignment(s)",
                    result.lod_assignment_updates.len()
                )
            };
            app.status_message = format!(
                "Merged {} day/night model pair(s), deleted {} duplicate placement(s){rename_summary}{lod_assignment_summary}, and reported {} review issue(s) in {elapsed:.1}s. Save to apply. Undo history cleared.",
                result.replacements.len(),
                result.delete_indices.len(),
                result.reviews.len()
            );
        }
        if result.override_available && !self.forced {
            app.status_message.push_str(
                " Click Merge Vertex Lighting again on this pair to override the warning.",
            );
        }
        set_save_log(
            app,
            "Day/Night Variant Merge",
            log,
            !result.reviews.is_empty(),
        );
        true
    }
}

fn day_night_asset_writer_running(app: &AppState) -> bool {
    app.manual_save_job.is_some()
        || app.editing.save_rx.is_some()
        || app.autosave_rx.is_some()
        || app.autosave_cleanup_rx.is_some()
        || app.blender_import_rx.is_some()
        || app.dff_picker_rx.is_some()
        || app.dff_repair_rx.is_some()
        || app.txd_cleanup_job.is_some()
        || app.asset_optimization_scan_rx.is_some()
        || app.asset_optimization_job.is_some()
        || app.object_bounds_fix_job.is_some()
        || app.corona_generation_job.is_some()
        || app.day_night_merge_job.is_some()
        || app.light_lod_job.is_some()
        || app.fracture_generation_job.is_some()
        || app.dff_geometry_job.is_some()
        || app.collision_generation_job.is_some()
        || app.shadow_mesh_generation_job.is_some()
        || app.collision_cuboid_audit_job.is_some()
        || app.lod_generation_job.is_some()
        || app.instance_lod_removal_job.is_some()
        || app.water_texture_conversion_job.is_some()
        || app.vehicle_build_rx.is_some()
        || app.vehicle_browser.texture_replace_rx.is_some()
        || app.vehicle_browser.collision_copy_rx.is_some()
        || app.bake_job.is_some()
}

pub(crate) fn request_day_night_variant_merge(app: &mut AppState) -> bool {
    if app.day_night_merge_job.is_some() {
        app.status_message = "A day/night variant merge is already running".to_string();
        return false;
    }
    if day_night_asset_writer_running(app) {
        app.status_message =
            "Day/night variant merge cannot start while another asset writer is running"
                .to_string();
        return false;
    }
    let selected = selected_live_indices_in_selection_order(app);
    let (selected_index, explicit_source_index) = match selected.as_slice() {
        [selected_index] => {
            if placement_day_night_variant(&app.placements[*selected_index]).is_none() {
                app.status_message =
                    "Select one _nt, _dt, or _dy object, or explicitly select a pair".to_string();
                return false;
            }
            (*selected_index, None)
        }
        [_, _] => {
            let (target_index, source_index) =
                selected_explicit_day_night_pair(app).expect("two live selections form a pair");
            (target_index, Some(source_index))
        }
        _ => {
            app.status_message =
                "Select one named day/night variant, or exactly two live objects to pair"
                    .to_string();
            return false;
        }
    };
    let override_key = if let Some(source_index) = explicit_source_index {
        day_night_pair_override_key(&app.placements, selected_index, source_index)
    } else {
        day_night_merge_override_key(
            &app.placements,
            &app.element_states,
            selected_index,
            app.bake_settings.day_night_merge_tolerance,
        )
    };
    let force_review_override = override_key
        .as_ref()
        .is_some_and(|key| app.day_night_merge_override.as_ref() == Some(key));
    app.day_night_merge_override = None;
    let context = snapshot_lighting_asset_context(
        app,
        selected_index,
        explicit_source_index,
        force_review_override,
    );
    app.day_night_merge_job = Some(DayNightMergeJob::new(
        context,
        override_key,
        force_review_override,
    ));
    app.status_message = if force_review_override {
        "Day/night variant override merge started in background...".to_string()
    } else if explicit_source_index.is_some() {
        "Selected-pair day/night comparison started in background...".to_string()
    } else {
        "Day/night variant comparison started in background...".to_string()
    };
    true
}

pub(crate) fn update_day_night_merge_job(app: &mut AppState) {
    let Some(mut job) = app.day_night_merge_job.take() else {
        return;
    };
    if !job.step(app) {
        app.day_night_merge_job = Some(job);
    }
}

struct LightLodResult {
    expected_detail: (usize, Placement),
    expected_lod: (usize, Placement),
    target_name: String,
    bytes: Vec<u8>,
    raw: RawMesh,
    preserved_vertex_mesh_keys: Vec<String>,
    texture_files: HashMap<String, PathBuf>,
    note: String,
}

pub(crate) struct LightLodJob {
    rx: mpsc::Receiver<Result<LightLodResult, String>>,
    started_at: Instant,
}

impl LightLodJob {
    fn new(context: DayNightMergeContext) -> Self {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let result = std::panic::catch_unwind(|| run_light_lod(context))
                .map_err(|panic| {
                    panic
                        .downcast_ref::<&str>()
                        .map(|message| (*message).to_string())
                        .or_else(|| panic.downcast_ref::<String>().cloned())
                        .unwrap_or_else(|| "unknown Light LOD worker panic".to_string())
                })
                .and_then(|result| result);
            let _ = tx.send(result);
        });
        Self {
            rx,
            started_at: Instant::now(),
        }
    }

    fn step(&mut self, app: &mut AppState) -> bool {
        let result = match self.rx.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return false,
            Err(mpsc::TryRecvError::Disconnected) => {
                app.status_message = "Light LOD worker disconnected".to_string();
                set_save_log(app, "Light LOD", vec![app.status_message.clone()], true);
                return true;
            }
        };
        let output = match result {
            Ok(output) => output,
            Err(error) => {
                app.status_message = format!("Light LOD failed: {error}");
                set_save_log(app, "Light LOD", vec![app.status_message.clone()], true);
                return true;
            }
        };
        let stale = [&output.expected_detail, &output.expected_lod]
            .into_iter()
            .any(|(index, expected)| {
                app.placements.get(*index) != Some(expected)
                    || app
                        .element_states
                        .get(*index)
                        .is_some_and(|state| state.deleted)
            });
        if stale {
            app.status_message =
                "Light LOD cancelled because the detail object or assigned LOD changed".to_string();
            set_save_log(app, "Light LOD", vec![app.status_message.clone()], true);
            return true;
        }

        let key = asset_key(&output.target_name, ".dff");
        let open_dirty = matches!(
            app.editing.asset.as_ref(),
            Some(EditingAsset::Dff(dff))
                if asset_key(&dff.name, ".dff") == key && dff.dirty
        );
        if open_dirty || app.editing.modified_entries.contains_key(&key) {
            app.editing
                .modified_entries
                .insert(key.clone(), output.bytes.clone());
        } else {
            app.pending_replacement_assets.insert(
                key.clone(),
                (output.target_name.clone(), output.bytes.clone()),
            );
        }
        if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut()
            && asset_key(&dff.name, ".dff") == key
        {
            dff.raw = output.raw.clone();
            dff.dirty = false;
        }
        for mesh_key in &output.preserved_vertex_mesh_keys {
            app.pending_vertex_light_meshes.remove(mesh_key);
            app.vertex_paint_dirty_meshes.remove(mesh_key);
        }
        if refresh_live_dff_from_raw(app, &output.target_name, &output.raw, &output.texture_files) {
            rebuild_render_cells(app);
        }
        clear_history_for_external_change(app);
        app.loaded_wip = true;
        let elapsed = self.started_at.elapsed().as_secs_f32();
        app.status_message = format!(
            "Lit assigned LOD {} from {} with approximate day/night prelight in {elapsed:.1}s. Save to apply. Undo history cleared.",
            output.expected_lod.1.id, output.expected_detail.1.id
        );
        set_save_log(
            app,
            "Light LOD",
            vec![format!(
                "Lit {} from detail {}: {}. The LOD geometry and materials were preserved.",
                output.target_name, output.expected_detail.1.dff, output.note
            )],
            false,
        );
        true
    }
}

fn run_light_lod(context: DayNightMergeContext) -> Result<LightLodResult, String> {
    let detail_index = context.selected_index;
    let lod_index = resolve_assigned_lod(&context.placements, &context.states, detail_index)?
        .ok_or_else(|| {
            format!(
                "{} has no assigned LOD",
                context
                    .placements
                    .get(detail_index)
                    .map(|placement| placement.id.as_str())
                    .unwrap_or("Selected object")
            )
        })?;
    let detail = context
        .placements
        .get(detail_index)
        .ok_or_else(|| "The selected detail object no longer exists".to_string())?;
    let lod = context
        .placements
        .get(lod_index)
        .ok_or_else(|| "The assigned LOD no longer exists".to_string())?;
    if detail.dff.eq_ignore_ascii_case(&lod.dff) {
        return Err("the detail object and assigned LOD use the same DFF".to_string());
    }
    let (detail_raw, _) = read_day_night_raw(&context, &detail.dff)?;
    let (lod_raw, preserved_vertex_mesh_keys) = read_day_night_raw(&context, &lod.dff)?;
    let (lit_lod, note) = light_lod_from_detail(&lod_raw, &detail_raw)?;
    let target_name = with_ext(&lod.dff, ".dff");
    let frame = Path::new(&target_name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("lod");
    let options = DffWriteOptions {
        include_normals: !context
            .building_dffs
            .contains(&asset_key(&target_name, ".dff")),
        include_bin_mesh: true,
    };
    let bytes = write_normalized_dff_with_options(&lit_lod, frame, options)
        .map_err(|error| format!("lit LOD could not be serialized: {error}"))?;
    let normalized = parse_dff_mesh(&bytes);
    verify_serialized_day_night_prelight(&lit_lod, &normalized, options.include_normals)
        .map_err(|error| format!("serialized lit LOD verification failed: {error}"))?;
    Ok(LightLodResult {
        expected_detail: (detail_index, detail.clone()),
        expected_lod: (lod_index, lod.clone()),
        target_name,
        bytes,
        raw: normalized,
        preserved_vertex_mesh_keys,
        texture_files: collect_texture_files(&context.root),
        note,
    })
}

fn snapshot_lighting_asset_context(
    app: &AppState,
    selected_index: usize,
    explicit_source_index: Option<usize>,
    force_review_override: bool,
) -> DayNightMergeContext {
    let mut byte_overrides = app.pending_replacement_assets.clone();
    for (key, bytes) in &app.editing.modified_entries {
        if app.editing.deleted_entries.contains(key) || !key.ends_with(".dff") {
            continue;
        }
        let name = app
            .editing
            .rows
            .iter()
            .find(|row| lower(&row.entry.name) == *key)
            .map(|row| row.entry.name.clone())
            .unwrap_or_else(|| key.clone());
        byte_overrides.insert(key.clone(), (name, bytes.clone()));
    }
    let mut raw_overrides = BTreeMap::new();
    if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_ref()
        && dff.dirty
    {
        raw_overrides.insert(
            asset_key(&dff.name, ".dff"),
            (dff.name.clone(), dff.raw.clone()),
        );
    }
    let mut vertex_mesh_overrides = BTreeMap::<String, Vec<(String, RenderMesh)>>::new();
    for mesh_key in &app.pending_vertex_light_meshes {
        let Some(mesh) = app.meshes.get(mesh_key) else {
            continue;
        };
        let dff_name = mesh_key.split('|').next().unwrap_or(mesh_key);
        vertex_mesh_overrides
            .entry(asset_key(dff_name, ".dff"))
            .or_default()
            .push((mesh_key.clone(), mesh.clone()));
    }
    DayNightMergeContext {
        root: app.root.clone(),
        gta_sa_dir: app.gta_sa_dir.clone(),
        selected_index,
        explicit_source_index,
        placements: app.placements.clone(),
        states: app.element_states.clone(),
        definitions: app.definitions.clone(),
        readonly_definition_ids: app.readonly_definition_ids.clone(),
        byte_overrides,
        raw_overrides,
        vertex_mesh_overrides,
        building_dffs: building_dff_set(app),
        tolerance: app.bake_settings.day_night_merge_tolerance,
        force_review_override,
    }
}

pub(crate) fn request_light_lod(app: &mut AppState) -> bool {
    if app.light_lod_job.is_some() {
        app.status_message = "Light LOD is already running".to_string();
        return false;
    }
    if day_night_asset_writer_running(app) {
        app.status_message =
            "Light LOD cannot start while another asset writer is running".to_string();
        return false;
    }
    let selected = selected_live_indices(app);
    if selected.len() != 1 {
        app.status_message = "Select exactly one detail object with an assigned LOD".to_string();
        return false;
    }
    match resolve_assigned_lod(&app.placements, &app.element_states, selected[0]) {
        Ok(Some(_)) => {}
        Ok(None) => {
            app.status_message = "The selected detail object has no assigned LOD".to_string();
            return false;
        }
        Err(error) => {
            app.status_message = format!("Light LOD cannot resolve the assignment: {error}");
            return false;
        }
    }
    let context = snapshot_lighting_asset_context(app, selected[0], None, false);
    app.light_lod_job = Some(LightLodJob::new(context));
    app.status_message = "Light LOD started in background...".to_string();
    true
}

pub(crate) fn update_light_lod_job(app: &mut AppState) {
    let Some(mut job) = app.light_lod_job.take() else {
        return;
    };
    if !job.step(app) {
        app.light_lod_job = Some(job);
    }
}
