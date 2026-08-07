use super::super::*;

const JOIN_MAX_EXTENT: f32 = 512.0;
const JOIN_WELD_DISTANCE: f32 = 0.001;
const JOIN_UV_EPSILON: f32 = 1.0 / 1024.0;

#[derive(Clone)]
struct JoinSource {
    placement: Placement,
    raw: RawMesh,
    col: Option<CollisionMesh>,
}

fn staged_asset_bytes(app: &AppState, name: &str, ext: &str) -> Option<Vec<u8>> {
    let key = asset_key(name, ext);
    app.pending_replacement_assets
        .get(&key)
        .map(|(_, bytes)| bytes.clone())
}

fn dff_bytes_for_placement(app: &AppState, placement: &Placement) -> Option<Vec<u8>> {
    staged_asset_bytes(app, &placement.dff, ".dff")
        .or_else(|| find_dff_entry_for_app(app, &placement.dff).map(|entry| read_img_entry(&entry)))
}

fn collect_join_sources(app: &AppState, indices: &[usize]) -> Result<Vec<JoinSource>, String> {
    let mut sources = Vec::with_capacity(indices.len());
    for &idx in indices {
        let placement = app
            .placements
            .get(idx)
            .ok_or_else(|| "A selected element no longer exists".to_string())?
            .clone();
        let bytes = dff_bytes_for_placement(app, &placement)
            .ok_or_else(|| format!("{}: DFF could not be found", placement.dff))?;
        let raw = parse_dff_mesh(&bytes);
        if raw.vertices.is_empty() || raw.triangles.is_empty() {
            return Err(format!("{}: DFF has no readable geometry", placement.dff));
        }
        let col = app
            .collisions
            .get(&element_collision_key(app, &placement))
            .cloned();
        sources.push(JoinSource {
            placement,
            raw,
            col,
        });
    }
    Ok(sources)
}

fn grow_bounds(min: &mut Vec3, max: &mut Vec3, point: Vec3) {
    *min = min.min(point);
    *max = max.max(point);
}

fn joined_world_bounds(sources: &[JoinSource]) -> Bounds {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for source in sources {
        let model = placement_matrix(&source.placement);
        for vertex in &source.raw.vertices {
            grow_bounds(&mut min, &mut max, model.transform_point3(to_mq(*vertex)));
        }
        if let Some(col) = &source.col {
            for vertex in &col.vertices {
                grow_bounds(&mut min, &mut max, model.transform_point3(to_mq(*vertex)));
            }
            let scale = placement_scale(&source.placement);
            for sphere in &col.spheres {
                let center = model.transform_point3(to_mq(sphere.center));
                let radius = sphere.radius.abs() * scale;
                grow_bounds(&mut min, &mut max, center - Vec3::splat(radius));
                grow_bounds(&mut min, &mut max, center + Vec3::splat(radius));
            }
            for col_box in &col.boxes {
                for corner in box_corners(col_box.min, col_box.max) {
                    grow_bounds(&mut min, &mut max, model.transform_point3(to_mq(corner)));
                }
            }
        }
    }
    Bounds { min, max }
}

fn box_corners(min: V3, max: V3) -> [V3; 8] {
    let (min_x, max_x) = (min.x.min(max.x), min.x.max(max.x));
    let (min_y, max_y) = (min.y.min(max.y), min.y.max(max.y));
    let (min_z, max_z) = (min.z.min(max.z), min.z.max(max.z));
    [
        V3 {
            x: min_x,
            y: min_y,
            z: min_z,
        },
        V3 {
            x: max_x,
            y: min_y,
            z: min_z,
        },
        V3 {
            x: min_x,
            y: max_y,
            z: min_z,
        },
        V3 {
            x: max_x,
            y: max_y,
            z: min_z,
        },
        V3 {
            x: min_x,
            y: min_y,
            z: max_z,
        },
        V3 {
            x: max_x,
            y: min_y,
            z: max_z,
        },
        V3 {
            x: min_x,
            y: max_y,
            z: max_z,
        },
        V3 {
            x: max_x,
            y: max_y,
            z: max_z,
        },
    ]
}

fn raw_material_count(raw: &RawMesh) -> usize {
    raw.material_textures
        .len()
        .max(raw.materials.len())
        .max(raw.material_animations.len())
        .max(
            raw.triangles
                .iter()
                .map(|tri| tri.material as usize + 1)
                .max()
                .unwrap_or(1),
        )
}

fn face_normals(raw: &RawMesh) -> Vec<V3> {
    let mut sums = vec![Vec3::ZERO; raw.vertices.len()];
    for tri in &raw.triangles {
        let [a, b, c] = [tri.a as usize, tri.b as usize, tri.c as usize];
        if a >= raw.vertices.len() || b >= raw.vertices.len() || c >= raw.vertices.len() {
            continue;
        }
        let normal = (to_mq(raw.vertices[b]) - to_mq(raw.vertices[a]))
            .cross(to_mq(raw.vertices[c]) - to_mq(raw.vertices[a]))
            .normalize_or_zero();
        sums[a] += normal;
        sums[b] += normal;
        sums[c] += normal;
    }
    sums.into_iter()
        .map(|normal| from_mq(normal.normalize_or_zero()))
        .collect()
}

fn combine_dffs(sources: &[JoinSource], anchor: &Placement) -> Result<RawMesh, String> {
    let anchor_inv = placement_matrix(anchor).inverse();
    let any_uvs = sources
        .iter()
        .any(|source| source.raw.uvs.len() == source.raw.vertices.len());
    let secondary_uv_count = sources
        .iter()
        .map(|source| {
            source
                .raw
                .secondary_uvs
                .iter()
                .filter(|uvs| uvs.len() == source.raw.vertices.len())
                .count()
        })
        .max()
        .unwrap_or(0);
    let any_day = sources
        .iter()
        .any(|source| source.raw.prelit_colors.len() == source.raw.vertices.len());
    let any_night = sources
        .iter()
        .any(|source| source.raw.night_prelit_colors.len() == source.raw.vertices.len());
    let any_day_alpha = sources
        .iter()
        .any(|source| source.raw.prelit_alphas.len() == source.raw.vertices.len());
    let any_night_alpha = sources
        .iter()
        .any(|source| source.raw.night_prelit_alphas.len() == source.raw.vertices.len());
    let any_flags = sources
        .iter()
        .any(|source| source.raw.light_flags.len() == source.raw.vertices.len());
    let mut out = RawMesh::default();
    out.secondary_uvs = vec![Vec::new(); secondary_uv_count];

    for source in sources {
        let relative = anchor_inv * placement_matrix(&source.placement);
        let normal_matrix = relative.inverse().transpose();
        let vertex_start = out.vertices.len();
        let tri_start = out.triangles.len();
        let material_count = raw_material_count(&source.raw);
        let mut material_remap = Vec::with_capacity(material_count);
        for idx in 0..material_count {
            let texture = source
                .raw
                .material_textures
                .get(idx)
                .cloned()
                .unwrap_or_default();
            let material = source.raw.materials.get(idx).copied().unwrap_or_default();
            let animation = source
                .raw
                .material_animations
                .get(idx)
                .cloned()
                .unwrap_or_default();
            let existing = (0..out.material_textures.len()).find(|existing| {
                out.material_textures[*existing].eq_ignore_ascii_case(&texture)
                    && out.materials.get(*existing).copied().unwrap_or_default() == material
                    && out
                        .material_animations
                        .get(*existing)
                        .cloned()
                        .unwrap_or_default()
                        == animation
            });
            let mapped = if let Some(existing) = existing {
                existing
            } else {
                if out.material_textures.len() > u16::MAX as usize {
                    return Err("Joined DFF has too many distinct materials".to_string());
                }
                let mapped = out.material_textures.len();
                out.material_textures.push(texture);
                out.materials.push(material);
                out.material_animations.push(animation);
                mapped
            };
            material_remap.push(mapped as u16);
        }
        let source_normals = if source.raw.normals.len() == source.raw.vertices.len() {
            source.raw.normals.clone()
        } else {
            face_normals(&source.raw)
        };
        for (idx, vertex) in source.raw.vertices.iter().enumerate() {
            out.vertices
                .push(from_mq(relative.transform_point3(to_mq(*vertex))));
            out.normals.push(from_mq(
                normal_matrix
                    .transform_vector3(to_mq(source_normals[idx]))
                    .normalize_or_zero(),
            ));
            if any_uvs {
                out.uvs
                    .push(source.raw.uvs.get(idx).copied().unwrap_or_default());
            }
            for (set_index, out_uvs) in out.secondary_uvs.iter_mut().enumerate() {
                out_uvs.push(
                    source
                        .raw
                        .secondary_uvs
                        .get(set_index)
                        .and_then(|uvs| uvs.get(idx))
                        .copied()
                        .unwrap_or_default(),
                );
            }
            if any_day {
                out.prelit_colors.push(
                    source
                        .raw
                        .prelit_colors
                        .get(idx)
                        .copied()
                        .unwrap_or_else(neutral_vertex_color),
                );
            }
            if any_night {
                out.night_prelit_colors.push(
                    source
                        .raw
                        .night_prelit_colors
                        .get(idx)
                        .copied()
                        .unwrap_or_else(neutral_vertex_color),
                );
            }
            if any_day_alpha {
                out.prelit_alphas
                    .push(source.raw.prelit_alphas.get(idx).copied().unwrap_or(1.0));
            }
            if any_night_alpha {
                out.night_prelit_alphas.push(
                    source
                        .raw
                        .night_prelit_alphas
                        .get(idx)
                        .copied()
                        .unwrap_or(1.0),
                );
            }
            if any_flags {
                out.light_flags
                    .push(source.raw.light_flags.get(idx).copied().unwrap_or(false));
            }
        }
        for tri in &source.raw.triangles {
            out.triangles.push(Tri {
                a: tri.a + vertex_start as u32,
                b: tri.b + vertex_start as u32,
                c: tri.c + vertex_start as u32,
                material: material_remap
                    .get(tri.material as usize)
                    .copied()
                    .unwrap_or(0),
            });
        }
        for effect in &source.raw.effects_2dfx {
            let mut effect = effect.clone();
            effect.position = from_mq(relative.transform_point3(to_mq(effect.position)));
            out.effects_2dfx.push(effect);
        }
        out.uv_anim_dictionaries
            .extend(source.raw.uv_anim_dictionaries.clone());
        for animation in &source.raw.uv_animations {
            if !out
                .uv_animations
                .iter()
                .any(|existing| existing.name == animation.name)
            {
                out.uv_animations.push(animation.clone());
            }
        }
        out.components.push(RawMeshComponent {
            name: source.placement.id.clone(),
            vertex_start,
            vertex_end: out.vertices.len(),
            tri_start,
            tri_end: out.triangles.len(),
            breakable: None,
        });
    }
    weld_raw_vertices(&mut out, JOIN_WELD_DISTANCE);
    Ok(out)
}

fn weld_raw_vertices(raw: &mut RawMesh, distance: f32) -> usize {
    let original_len = raw.vertices.len();
    if original_len < 2 || distance <= 0.0 {
        return 0;
    }
    let distance2 = distance * distance;
    let uv2 = JOIN_UV_EPSILON * JOIN_UV_EPSILON;
    let mut remap = vec![usize::MAX; original_len];
    let mut representatives = Vec::<usize>::new();
    let mut grid = HashMap::<[i32; 3], Vec<usize>>::new();
    for (idx, vertex) in raw.vertices.iter().enumerate() {
        let cell = [
            (vertex.x / distance).floor() as i32,
            (vertex.y / distance).floor() as i32,
            (vertex.z / distance).floor() as i32,
        ];
        let mut found = None;
        'neighbors: for x in -1..=1 {
            for y in -1..=1 {
                for z in -1..=1 {
                    if let Some(candidates) = grid.get(&[cell[0] + x, cell[1] + y, cell[2] + z]) {
                        for &slot in candidates {
                            let rep = representatives[slot];
                            let delta = to_mq(*vertex) - to_mq(raw.vertices[rep]);
                            if delta.length_squared() > distance2 {
                                continue;
                            }
                            if raw.uvs.len() == original_len {
                                let a = raw.uvs[idx];
                                let b = raw.uvs[rep];
                                if (a.u - b.u).powi(2) + (a.v - b.v).powi(2) > uv2 {
                                    continue;
                                }
                            }
                            let secondary_uv_compatible = raw.secondary_uvs.iter().all(|uvs| {
                                if uvs.len() != original_len {
                                    return true;
                                }
                                let a = uvs[idx];
                                let b = uvs[rep];
                                (a.u - b.u).powi(2) + (a.v - b.v).powi(2) <= uv2
                            });
                            if !secondary_uv_compatible {
                                continue;
                            }
                            found = Some(slot);
                            break 'neighbors;
                        }
                    }
                }
            }
        }
        let slot = found.unwrap_or_else(|| {
            let slot = representatives.len();
            representatives.push(idx);
            grid.entry(cell).or_default().push(slot);
            slot
        });
        remap[idx] = slot;
    }
    let merged = original_len - representatives.len();
    if merged == 0 {
        return 0;
    }
    let select_v3 = |values: &Vec<V3>| -> Vec<V3> {
        if values.len() == original_len {
            representatives.iter().map(|&idx| values[idx]).collect()
        } else {
            values.clone()
        }
    };
    raw.vertices = select_v3(&raw.vertices);
    raw.normals = select_v3(&raw.normals);
    raw.prelit_colors = select_v3(&raw.prelit_colors);
    raw.night_prelit_colors = select_v3(&raw.night_prelit_colors);
    let select_f32 = |values: &Vec<f32>| -> Vec<f32> {
        if values.len() == original_len {
            representatives.iter().map(|&idx| values[idx]).collect()
        } else {
            values.clone()
        }
    };
    raw.prelit_alphas = select_f32(&raw.prelit_alphas);
    raw.night_prelit_alphas = select_f32(&raw.night_prelit_alphas);
    if raw.uvs.len() == original_len {
        raw.uvs = representatives.iter().map(|&idx| raw.uvs[idx]).collect();
    }
    for uvs in &mut raw.secondary_uvs {
        if uvs.len() == original_len {
            *uvs = representatives.iter().map(|&idx| uvs[idx]).collect();
        }
    }
    if raw.light_flags.len() == original_len {
        raw.light_flags = representatives
            .iter()
            .map(|&idx| raw.light_flags[idx])
            .collect();
    }
    raw.triangles.retain_mut(|tri| {
        tri.a = remap[tri.a as usize] as u32;
        tri.b = remap[tri.b as usize] as u32;
        tri.c = remap[tri.c as usize] as u32;
        tri.a != tri.b && tri.b != tri.c && tri.c != tri.a
    });
    raw.components.clear();
    merged
}

fn append_box_as_faces(
    mesh: &mut CollisionMesh,
    corners: [V3; 8],
    surface: &CollisionSurface,
) -> Result<(), String> {
    if mesh.vertices.len() + 8 > u16::MAX as usize {
        return Err(format!(
            "COL {} would contain {} vertices while expanding a collision box (limit {})",
            mesh.name,
            mesh.vertices.len() + 8,
            u16::MAX
        ));
    }
    let base = mesh.vertices.len() as u16;
    mesh.vertices.extend(corners);
    for [a, b, c] in [
        [0, 2, 1],
        [1, 2, 3],
        [4, 5, 6],
        [5, 7, 6],
        [0, 1, 4],
        [1, 5, 4],
        [2, 6, 3],
        [3, 6, 7],
        [0, 4, 2],
        [2, 4, 6],
        [1, 3, 5],
        [3, 7, 5],
    ] {
        mesh.faces.push(CollisionFace {
            a: base + a,
            b: base + b,
            c: base + c,
            material: surface.material,
            light: surface.light,
            img_path: PathBuf::new(),
            material_file_offset: 0,
            light_file_offset: 0,
        });
    }
    Ok(())
}

fn combine_cols(
    sources: &[JoinSource],
    anchor: &Placement,
    name: &str,
) -> Result<Option<CollisionMesh>, String> {
    if !sources.iter().any(|source| source.col.is_some()) {
        return Ok(None);
    }
    let anchor_inv = placement_matrix(anchor).inverse();
    let anchor_scale = placement_scale(anchor);
    let mut out = CollisionMesh {
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
    for source in sources {
        let Some(col) = &source.col else {
            continue;
        };
        let relative = anchor_inv * placement_matrix(&source.placement);
        let added_vertices = col
            .vertices
            .len()
            .saturating_add(col.boxes.len().saturating_mul(8));
        let added_faces = col
            .faces
            .len()
            .saturating_add(col.boxes.len().saturating_mul(12));
        let next_vertices = out.vertices.len().saturating_add(added_vertices);
        let next_faces = out.faces.len().saturating_add(added_faces);
        if next_vertices > u16::MAX as usize {
            return Err(format!(
                "COL {name} would contain {next_vertices} vertices (limit {}), while adding source {} using {}; reduce the Join selection",
                u16::MAX,
                source.placement.id,
                source.placement.dff
            ));
        }
        if next_faces > u16::MAX as usize {
            return Err(format!(
                "COL {name} would contain {next_faces} faces (limit {}), while adding source {} using {}; reduce the Join selection",
                u16::MAX,
                source.placement.id,
                source.placement.dff
            ));
        }
        let base = out.vertices.len() as u16;
        out.vertices.extend(
            col.vertices
                .iter()
                .map(|vertex| from_mq(relative.transform_point3(to_mq(*vertex)))),
        );
        for face in &col.faces {
            out.faces.push(CollisionFace {
                a: base + face.a,
                b: base + face.b,
                c: base + face.c,
                material: face.material,
                light: face.light,
                img_path: PathBuf::new(),
                material_file_offset: 0,
                light_file_offset: 0,
            });
        }
        let radius_scale = placement_scale(&source.placement) / anchor_scale;
        out.spheres
            .extend(col.spheres.iter().map(|sphere| CollisionSphere {
                center: from_mq(relative.transform_point3(to_mq(sphere.center))),
                radius: sphere.radius * radius_scale,
                surface: sphere.surface.clone(),
            }));
        for col_box in &col.boxes {
            let transformed = box_corners(col_box.min, col_box.max)
                .map(|corner| from_mq(relative.transform_point3(to_mq(corner))));
            append_box_as_faces(&mut out, transformed, &col_box.surface)?;
        }
    }
    weld_col_vertices(&mut out, JOIN_WELD_DISTANCE);
    out.bounds = collision_mesh_bounds(&out.vertices, &out.spheres, &out.boxes);
    if out.faces.len() > u16::MAX as usize {
        return Err(format!(
            "COL {name} contains {} faces (limit {})",
            out.faces.len(),
            u16::MAX
        ));
    }
    Ok(Some(out))
}

fn weld_col_vertices(mesh: &mut CollisionMesh, distance: f32) -> usize {
    let original_len = mesh.vertices.len();
    if original_len < 2 || distance <= 0.0 {
        return 0;
    }
    let distance2 = distance * distance;
    let mut remap = vec![usize::MAX; original_len];
    let mut representatives = Vec::<usize>::new();
    let mut grid = HashMap::<[i32; 3], Vec<usize>>::new();
    for (idx, vertex) in mesh.vertices.iter().enumerate() {
        let cell = [
            (vertex.x / distance).floor() as i32,
            (vertex.y / distance).floor() as i32,
            (vertex.z / distance).floor() as i32,
        ];
        let mut found = None;
        'neighbors: for x in -1..=1 {
            for y in -1..=1 {
                for z in -1..=1 {
                    if let Some(candidates) = grid.get(&[cell[0] + x, cell[1] + y, cell[2] + z]) {
                        for &slot in candidates {
                            let rep = representatives[slot];
                            if (to_mq(*vertex) - to_mq(mesh.vertices[rep])).length_squared()
                                <= distance2
                            {
                                found = Some(slot);
                                break 'neighbors;
                            }
                        }
                    }
                }
            }
        }
        let slot = found.unwrap_or_else(|| {
            let slot = representatives.len();
            representatives.push(idx);
            grid.entry(cell).or_default().push(slot);
            slot
        });
        remap[idx] = slot;
    }
    let merged = original_len - representatives.len();
    if merged == 0 {
        return 0;
    }
    mesh.vertices = representatives
        .iter()
        .map(|&idx| mesh.vertices[idx])
        .collect();
    mesh.faces.retain_mut(|face| {
        face.a = remap[face.a as usize] as u16;
        face.b = remap[face.b as usize] as u16;
        face.c = remap[face.c as usize] as u16;
        face.a != face.b && face.b != face.c && face.c != face.a
    });
    merged
}

fn join_asset_stem(app: &AppState, primary_idx: usize) -> String {
    let base_name = img_safe_replacement_name(app, primary_idx, false, ".dff");
    let base = Path::new(&base_name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("joined");
    let mut stem = format!("{base}_join");
    let max_stem_len = IMG_RUNTIME_SAFE_ENTRY_NAME_BYTES - ".dff".len();
    stem.truncate(max_stem_len);
    let mut candidate = stem.clone();
    let mut suffix = 2usize;
    while app.definitions.contains_key(&candidate)
        || app
            .placements
            .iter()
            .any(|placement| placement.id.eq_ignore_ascii_case(&candidate))
        || app
            .pending_replacement_assets
            .contains_key(&asset_key(&candidate, ".dff"))
    {
        let tail = format!("_{suffix}");
        let keep = max_stem_len.saturating_sub(tail.len());
        candidate = format!("{}{}", &stem[..stem.len().min(keep)], tail);
        suffix += 1;
    }
    candidate
}

fn show_join_size_warning(app: &mut AppState, size: Vec3) {
    app.status_message = format!(
        "Join blocked: combined size {:.1} x {:.1} x {:.1} exceeds 512 x 512 x 512",
        size.x, size.y, size.z
    );
    app.confirm_dialog = Some(ConfirmDialog {
        action: ConfirmAction::DismissWarning,
        title: "Join Too Large".to_string(),
        body: "These elements cannot be joined because the combined bounds exceed 512 on at least one axis.".to_string(),
        detail: format!("Combined size: {:.1} x {:.1} x {:.1}   Maximum: 512 x 512 x 512", size.x, size.y, size.z),
        primary_label: "OK".to_string(),
        secondary_label: None,
        secondary_action: None,
    });
}

fn join_size_exceeds_limit(size: Vec3) -> bool {
    size.x > JOIN_MAX_EXTENT || size.y > JOIN_MAX_EXTENT || size.z > JOIN_MAX_EXTENT
}

pub(crate) fn join_selected_elements(app: &mut AppState) {
    let indices = selected_live_indices_in_selection_order(app);
    if indices.len() < 2 {
        app.status_message = "Select at least two live elements to Join".to_string();
        return;
    }
    let sources = match collect_join_sources(app, &indices) {
        Ok(sources) => sources,
        Err(err) => {
            app.status_message = format!("Join failed: {err}");
            return;
        }
    };
    let bounds = joined_world_bounds(&sources);
    let size = bounds.max - bounds.min;
    if join_size_exceeds_limit(size) {
        show_join_size_warning(app, size);
        return;
    }
    let primary_idx = indices[0];
    let anchor = sources[0].placement.clone();
    let stem = join_asset_stem(app, primary_idx);
    let dff_name = format!("{stem}.dff");
    let col_name = format!("{stem}.col");
    let raw = match combine_dffs(&sources, &anchor) {
        Ok(raw) => raw,
        Err(err) => {
            app.status_message = format!("Join failed: {err}");
            return;
        }
    };
    let col = match combine_cols(&sources, &anchor, &stem) {
        Ok(col) => col,
        Err(err) => {
            app.status_message = format!("Join failed: {err}");
            return;
        }
    };
    let dff_bytes = match write_normalized_dff(&raw, &stem) {
        Ok(bytes) => bytes,
        Err(err) => {
            app.status_message = format!("Join failed: {err}");
            return;
        }
    };
    let col_bytes = match col.as_ref() {
        Some(mesh) => match write_col_mesh_from_template(&minimal_col2_template(&col_name), mesh) {
            Ok(mut bytes) => {
                set_col_model_names_from_entry(&mut bytes, &col_name);
                Some(bytes)
            }
            Err(err) => {
                app.status_message = format!("Join failed: {err}");
                return;
            }
        },
        None => None,
    };
    let asset_root = replacement_asset_root(app);
    if let Err(err) = upsert_replacement_dff(&asset_root, &dff_name, &dff_bytes) {
        app.status_message = format!("Join failed while staging DFF: {err}");
        return;
    }
    if let Some(bytes) = &col_bytes {
        if let Err(err) = upsert_replacement_col(&asset_root, &col_name, bytes) {
            app.status_message = format!("Join failed while staging COL: {err}");
            return;
        }
    }

    let txd = app
        .definitions
        .get(&anchor.id)
        .and_then(|def| def.attrs.get("txd"))
        .cloned();
    let ambient_lift = scene_ambient_lift_from_timecyc(&app.timecyc);
    let Some(render_mesh) = compile_render_mesh(
        raw.clone(),
        txd.as_deref(),
        None,
        None,
        &app.texture_files,
        &app.txd_textures,
        &mut app.textures,
        &mut app.textured_parts,
        app.options.textures,
        ambient_lift,
    ) else {
        app.status_message = "Join failed: combined DFF did not compile".to_string();
        return;
    };
    let mesh_key = mesh_key_from_dff_txd(&dff_name, txd.as_deref());
    replace_render_mesh(&mut app.meshes, mesh_key, render_mesh);
    if let Some(mesh) = &col {
        app.collisions
            .insert(asset_key(&col_name, ".col"), mesh.clone());
    }
    app.pending_replacement_assets
        .insert(asset_key(&dff_name, ".dff"), (dff_name.clone(), dff_bytes));
    if let Some(bytes) = col_bytes {
        app.pending_replacement_assets
            .insert(asset_key(&col_name, ".col"), (col_name.clone(), bytes));
    }

    let mut definition = app
        .definitions
        .get(&anchor.id)
        .cloned()
        .unwrap_or(Definition {
            id: stem.clone(),
            zone: anchor.zone.clone(),
            attrs: BTreeMap::new(),
        });
    definition.id = stem.clone();
    definition.zone = anchor.zone.clone();
    definition.attrs.insert("id".to_string(), stem.clone());
    definition.attrs.insert("dff".to_string(), stem.clone());
    if col.is_some() {
        definition.attrs.insert("col".to_string(), stem.clone());
    } else {
        definition.attrs.remove("col");
    }
    app.definitions.insert(stem.clone(), definition);
    let mut joined = anchor;
    joined.id = stem.clone();
    joined.dff = stem.clone();
    joined.attrs.remove("lodParent");
    joined.attrs.remove("uniqueID");
    sync_placement_attrs(&mut joined);
    app.placements.push(joined);
    app.element_states.push(ElementState::default());
    app.outliner_labels.push(None);
    let joined_idx = app.placements.len() - 1;
    for idx in indices {
        if let Some(state) = app.element_states.get_mut(idx) {
            state.deleted = true;
        }
    }
    app.selected = joined_idx;
    app.selected_elements.clear();
    app.selected_elements.insert(joined_idx);
    app.selected_element_order = vec![joined_idx];
    app.selected_col_face = None;
    invalidate_outliner_labels(app);
    rebuild_outliner_filter(app);
    invalidate_validation_cache(app);
    rebuild_render_cells(app);
    clear_history_for_external_change(app);
    app.loaded_wip = true;
    app.status_message = format!(
        "Joined {} elements into {} ({} DFF vertices, {} COL vertices); welded adjoining vertices; undo history cleared",
        sources.len(),
        stem,
        raw.vertices.len(),
        col.as_ref().map(|mesh| mesh.vertices.len()).unwrap_or(0)
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weld_raw_vertices_merges_coincident_vertices_and_remaps_faces() {
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
                    x: 0.0005,
                    y: 0.0,
                    z: 0.0,
                },
            ],
            triangles: vec![Tri {
                a: 3,
                b: 1,
                c: 2,
                material: 0,
            }],
            ..RawMesh::default()
        };
        assert_eq!(weld_raw_vertices(&mut raw, 0.001), 1);
        assert_eq!(raw.vertices.len(), 3);
        assert_eq!(raw.triangles[0].a, 0);
    }

    #[test]
    fn weld_raw_vertices_preserves_uv_seams() {
        let mut raw = RawMesh {
            vertices: vec![V3::default(), V3::default()],
            uvs: vec![V2 { u: 0.0, v: 0.0 }, V2 { u: 1.0, v: 0.0 }],
            ..RawMesh::default()
        };
        assert_eq!(weld_raw_vertices(&mut raw, 0.001), 0);
    }

    #[test]
    fn joined_bounds_detect_axis_over_limit() {
        assert!(!join_size_exceeds_limit(Vec3::splat(512.0)));
        assert!(join_size_exceeds_limit(Vec3::new(512.01, 20.0, 30.0)));
        assert!(join_size_exceeds_limit(Vec3::new(20.0, 513.0, 30.0)));
        assert!(join_size_exceeds_limit(Vec3::new(20.0, 30.0, 900.0)));
    }

    #[test]
    fn combine_dffs_deduplicates_identical_material_signatures() {
        let placement = |id: &str| Placement {
            id: id.to_string(),
            dff: id.to_string(),
            zone: "test".to_string(),
            tag: "building".to_string(),
            attrs: BTreeMap::new(),
            pos: V3::default(),
            rot: V3::default(),
        };
        let raw = RawMesh {
            vertices: vec![
                V3::default(),
                V3 {
                    x: 1.0,
                    ..V3::default()
                },
                V3 {
                    y: 1.0,
                    ..V3::default()
                },
            ],
            triangles: vec![Tri {
                a: 0,
                b: 1,
                c: 2,
                material: 0,
            }],
            material_textures: vec!["brick".to_string()],
            materials: vec![RawMaterial::default()],
            ..RawMesh::default()
        };
        let sources = vec![
            JoinSource {
                placement: placement("a"),
                raw: raw.clone(),
                col: None,
            },
            JoinSource {
                placement: placement("b"),
                raw,
                col: None,
            },
        ];
        let combined = combine_dffs(&sources, &placement("anchor")).unwrap();
        assert_eq!(combined.material_textures, vec!["brick"]);
        assert!(
            combined
                .triangles
                .iter()
                .all(|triangle| triangle.material == 0)
        );
    }
}
