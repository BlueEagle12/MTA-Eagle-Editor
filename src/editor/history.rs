use super::super::*;

pub(crate) fn global_transform_matrix(transform: GlobalTransformState) -> Mat4 {
    Mat4::from_translation(to_mq(transform.offset))
        * Mat4::from_rotation_z(transform.rotation.z.to_radians())
        * Mat4::from_rotation_y(transform.rotation.y.to_radians())
        * Mat4::from_rotation_x(transform.rotation.x.to_radians())
}

fn transform_v3(matrix: Mat4, value: V3) -> V3 {
    from_mq(matrix.transform_point3(to_mq(value)))
}

fn transform_direction(matrix: Mat4, value: V3) -> V3 {
    let transformed = matrix.transform_vector3(to_mq(value));
    if transformed.length_squared() > 0.000001 {
        from_mq(transformed.normalize())
    } else {
        value
    }
}

pub(crate) fn matrix_rotation_degrees(matrix: Mat4) -> V3 {
    // Decompose the editor's Rz * Ry * Rx convention.
    let y = (-matrix.x_axis.z).clamp(-1.0, 1.0).asin();
    let cy = y.cos();
    let (x, z) = if cy.abs() > 0.00001 {
        (
            matrix.y_axis.z.atan2(matrix.z_axis.z),
            matrix.x_axis.y.atan2(matrix.x_axis.x),
        )
    } else {
        (0.0, (-matrix.y_axis.x).atan2(matrix.y_axis.y))
    };
    V3 {
        x: x.to_degrees(),
        y: y.to_degrees(),
        z: z.to_degrees(),
    }
}

fn transform_race_points(matrix: Mat4, subtrack: &mut RaceSubtrack) {
    for checkpoint in &mut subtrack.checkpoints {
        checkpoint.pos = transform_v3(matrix, checkpoint.pos);
    }
    for point in &mut subtrack.overlay {
        *point = transform_v3(matrix, *point);
    }
    for point in &mut subtrack.path {
        *point = transform_v3(matrix, *point);
    }
}

pub(crate) fn apply_global_transform(app: &mut AppState) {
    let transform = app.global_transform;
    let is_identity = transform.offset == V3::default() && transform.rotation == V3::default();
    let has_targets = transform.transform_elements || transform.transform_water;
    if is_identity || !has_targets {
        app.global_transform.preview = false;
        app.status_message = if is_identity {
            "Global transform is identity; nothing to apply".to_string()
        } else {
            "Enable Elements and/or Water before applying".to_string()
        };
        return;
    }

    let before = global_transform_history_snapshot(app);
    let matrix = global_transform_matrix(transform);
    let rotation = Mat4::from_rotation_z(transform.rotation.z.to_radians())
        * Mat4::from_rotation_y(transform.rotation.y.to_radians())
        * Mat4::from_rotation_x(transform.rotation.x.to_radians());

    if transform.transform_elements {
        for placement in &mut app.placements {
            if is_default_world_placement(placement) {
                continue;
            }
            placement.pos = transform_v3(matrix, placement.pos);
            let local_rotation = Mat4::from_rotation_z(placement.rot.z.to_radians())
                * Mat4::from_rotation_y(placement.rot.y.to_radians())
                * Mat4::from_rotation_x(placement.rot.x.to_radians());
            placement.rot = matrix_rotation_degrees(rotation * local_rotation);
            sync_placement_attrs(placement);
        }
        for light in &mut app.lights {
            if light.attached_to.is_none() {
                light.position = transform_v3(matrix, light.position);
                light.direction = transform_direction(rotation, light.direction);
            }
        }
        for track in &mut app.race.tracks {
            track.commit_active_subtrack();
            track.start = transform_v3(matrix, track.start);
            track.find = transform_v3(matrix, track.find);
            for subtrack in &mut track.subtracks {
                transform_race_points(matrix, subtrack);
            }
            track.load_subtrack(track.active_subtrack);
        }
        let radar_center =
            matrix.transform_point3(vec3(app.race.world_center_x, app.race.world_center_y, 0.0));
        app.race.world_center_x = radar_center.x;
        app.race.world_center_y = radar_center.y;
    }
    if transform.transform_water {
        for plane in &mut app.water_planes {
            for corner in &mut plane.corners {
                corner.pos = transform_v3(matrix, corner.pos);
            }
        }
    }

    app.global_transform = GlobalTransformState::default();
    rebuild_render_cells(app);
    commit_global_transform_history(app, "Apply Global Transform", before);
    app.status_message = "Applied global transform to the world".to_string();
}

pub(crate) fn active_placements(
    placements: &[Placement],
    states: &[ElementState],
) -> Vec<Placement> {
    placements
        .iter()
        .zip(states.iter())
        .filter(|(_, state)| !state.deleted && !state.hidden)
        .map(|(placement, _)| placement.clone())
        .collect()
}

pub(crate) fn delete_render_cells(scene_cells: &[SceneCell], world_cells: &[WorldCell]) {
    unsafe {
        for cell in scene_cells {
            if cell.list != 0 {
                gl::DeleteLists(cell.list, 1);
            }
        }
        for cell in world_cells {
            if cell.vbo != 0 {
                gl::DeleteBuffers(1, &cell.vbo);
            }
        }
    }
}

pub(crate) fn invalidate_collision_render_cache(app: &mut AppState, collision_key: &str) {
    if let Some(cache) = app.collision_render_cache.remove(collision_key) {
        unsafe {
            if cache.list != 0 {
                gl::DeleteLists(cache.list, 1);
            }
        }
    }
}

pub(crate) fn clear_collision_render_cache(app: &mut AppState) {
    let caches = std::mem::take(&mut app.collision_render_cache);
    unsafe {
        for cache in caches.into_values() {
            if cache.list != 0 {
                gl::DeleteLists(cache.list, 1);
            }
        }
    }
}

pub(crate) fn delete_app_gl_resources(app: &mut AppState) {
    delete_render_cells(&app.scene_cells, &app.world_cells);
    delete_render_cells(&app.lod_scene_cells, &app.lod_world_cells);
    clear_collision_render_cache(app);
    clear_material_plugins();
    unsafe {
        for mesh in app.meshes.values() {
            for part in &mesh.parts {
                if part.list != 0 {
                    gl::DeleteLists(part.list, 1);
                }
                if part.vbo != 0 {
                    gl::DeleteBuffers(1, &part.vbo);
                }
            }
        }
        for texture in app.textures.values() {
            if *texture != 0 {
                gl::DeleteTextures(1, texture);
            }
        }
    }
}

pub(crate) fn delete_gpu_lightmap_resources(pipeline: &GpuLightmapPipeline) {
    unsafe {
        if pipeline.vertex_buffer != 0 {
            gl::DeleteBuffers(1, &pipeline.vertex_buffer);
        }
        if pipeline.light_buffer != 0 {
            gl::DeleteBuffers(1, &pipeline.light_buffer);
        }
        if pipeline.output_buffer != 0 {
            gl::DeleteBuffers(1, &pipeline.output_buffer);
        }
        if pipeline.transform_feedback != 0 {
            gl::DeleteTransformFeedbacks(1, &pipeline.transform_feedback);
        }
        if pipeline.shadow_fbo != 0 {
            gl::DeleteFramebuffers(1, &pipeline.shadow_fbo);
        }
        if pipeline.point_shadow_fbo != 0 {
            gl::DeleteFramebuffers(1, &pipeline.point_shadow_fbo);
        }
        if pipeline.shadow_depth_texture != 0 {
            gl::DeleteTextures(1, &pipeline.shadow_depth_texture);
        }
        if pipeline.point_shadow_depth_texture != 0 {
            gl::DeleteTextures(1, &pipeline.point_shadow_depth_texture);
        }
        if pipeline.program != 0 {
            gl::DeleteProgram(pipeline.program);
        }
    }
}

pub(crate) fn clear_gpu_lightmap_pipeline(pipeline: &mut GpuLightmapPipeline) {
    pipeline.supported = false;
    pipeline.vertex_buffer = 0;
    pipeline.light_buffer = 0;
    pipeline.output_buffer = 0;
    pipeline.transform_feedback = 0;
    pipeline.shadow_fbo = 0;
    pipeline.point_shadow_fbo = 0;
    pipeline.shadow_depth_texture = 0;
    pipeline.point_shadow_depth_texture = 0;
    pipeline.program = 0;
}

pub(crate) fn release_loaded_resource(app: &mut AppState) {
    delete_app_gl_resources(app);
    delete_gpu_lightmap_resources(&app.gpu_lightmap);
    unsafe {
        if app.dff_pointlight_preview_program != 0 {
            gl::DeleteProgram(app.dff_pointlight_preview_program);
        }
        if app.vehicle_browser.photo_reflection_program != 0 {
            gl::DeleteProgram(app.vehicle_browser.photo_reflection_program);
        }
        gl::Finish();
    }
    app.dff_pointlight_preview_program = 0;
    app.dff_pointlight_preview_failed = false;
    app.vehicle_browser.photo_reflection_program = 0;
    app.vehicle_browser.photo_reflection_failed = false;
    app.vehicle_browser.photo_mode = false;
    app.vehicle_browser.photo_mode_camera = None;
    app.vehicle_browser.photo_mode_camera_mode = None;
    app.vehicle_browser.photo_mode_focus = None;
    clear_gpu_lightmap_pipeline(&mut app.gpu_lightmap);
    app.placements = Vec::new();
    app.definitions = HashMap::new();
    app.zones = Vec::new();
    app.map_documents.clear();
    app.placement_destination = None;
    app.map_dialog = None;
    app.meshes = HashMap::new();
    app.collisions = HashMap::new();
    app.collision_render_cache.clear();
    app.scene_cells = Vec::new();
    app.world_cells = Vec::new();
    app.lod_scene_cells = Vec::new();
    app.lod_world_cells = Vec::new();
    app.textures = HashMap::new();
    app.txd_textures = HashMap::new();
    app.texture_files = HashMap::new();
    app.texture_overrides = HashMap::new();
    app.pending_replacement_assets.clear();
    app.pending_txd_writes.clear();
    app.pending_asset_deletes.clear();
    app.pending_vertex_light_meshes.clear();
    app.corona_generation_job = None;
    app.day_night_merge_job = None;
    app.day_night_merge_override = None;
    app.light_lod_job = None;
    app.fracture_generation_job = None;
    app.dff_geometry_job = None;
    app.collision_generation_job = None;
    app.shadow_mesh_generation_job = None;
    app.vertex_paint_dirty_meshes.clear();
    app.vertex_paint_next_rebuild_at = 0.0;
    app.outliner_labels = Vec::new();
    app.outliner_filter = Vec::new();
    app.outliner_show_objects = true;
    app.outliner_show_buildings = true;
    app.outliner_show_lods = true;
    app.asset_browser.search.clear();
    app.asset_browser.cursor = 0;
    app.asset_browser.selection_anchor = None;
    app.asset_browser.search_active = false;
    app.asset_browser.scroll = 0.0;
    app.asset_browser.thumbnails.clear();
    app.validation_cache = None;
    app.lod_audit = lod_audit_state_from_preferences(&app.root);
    app.missing_texture_review = MissingTextureReviewState::default();
    app.element_states = Vec::new();
    app.pending_col_writes = HashMap::new();
    app.bake_job = None;
    app.selected = NO_SELECTION;
    app.selected_elements.clear();
    app.selected_element_order.clear();
    app.hovered = None;
    app.selected_col_face = None;
    app.dff_texture_duplicate_dialog = None;
    app.loaded_message = "Loading resource...".to_string();
    app.last_drawn_placements = 0;
    app.last_drawn_parts = 0;
    app.last_drawn_vertices = 0;
}

pub(crate) fn rebuild_render_cells(app: &mut AppState) {
    rebuild_render_cells_with_mesh_lift(app, false);
}

pub(crate) fn build_placement_2dfx_indices(
    placements: &[Placement],
    definitions: &HashMap<String, Definition>,
    meshes: &HashMap<String, RenderMesh>,
) -> Vec<usize> {
    placements
        .iter()
        .enumerate()
        .filter_map(|(index, placement)| {
            meshes
                .get(&placement_mesh_key(placement, definitions))
                .is_some_and(|mesh| !mesh.effects_2dfx.is_empty())
                .then_some(index)
        })
        .collect()
}

pub(crate) fn rebuild_placement_2dfx_index(app: &mut AppState) {
    app.placement_2dfx_indices =
        build_placement_2dfx_indices(&app.placements, &app.definitions, &app.meshes);
    app.placement_2dfx_index_placement_count = app.placements.len();
}

fn placement_cell_rebuild_source(
    placements: &[Placement],
    states: &[ElementState],
    keys: &HashSet<WorldCellKey>,
) -> (Vec<Placement>, HashSet<String>) {
    let mut affected = Vec::new();
    let mut lod_ids = HashSet::new();
    for (placement, state) in placements.iter().zip(states) {
        if state.deleted || state.hidden {
            continue;
        }
        if let Some(parent) = placement.attrs.get("lodParent") {
            let parent = parent.trim();
            if !parent.is_empty() && !parent.eq_ignore_ascii_case("self") {
                lod_ids.insert(parent.to_ascii_lowercase());
            }
        }
        if keys.contains(&world_cell_key(placement.pos)) {
            affected.push(placement.clone());
        }
    }
    (affected, lod_ids)
}

fn placement_requires_cell_rebuild(old: &Placement, new: &Placement) -> bool {
    old.pos != new.pos
        || old.rot != new.rot
        || old.id != new.id
        || old.dff != new.dff
        || old.tag != new.tag
        || placement_scale(old) != placement_scale(new)
        || placement_alpha(old) != placement_alpha(new)
        || old.attrs.get("lodParent") != new.attrs.get("lodParent")
        || placement_override_flag_enabled(old, "double_sided")
            != placement_override_flag_enabled(new, "double_sided")
        || camera_follow_override(&old.attrs) != camera_follow_override(&new.attrs)
        || placement_is_background_scenery(old) != placement_is_background_scenery(new)
}

/// Rebuilds only the complete fast-VBO cells touched by an in-place placement
/// transform. Both the old and current origin keys are included so crossing a
/// cell boundary removes the placement from its former cell and inserts it in
/// its new one without disturbing unrelated GPU buffers.
pub(crate) fn rebuild_render_cells_for_placement_transforms(
    app: &mut AppState,
    before: &PlacementTransformHistorySnapshot,
) {
    let mut dirty_keys = HashSet::with_capacity(before.placements.len() * 2);
    for (index, old_placement) in &before.placements {
        if app
            .placements
            .get(*index)
            .is_some_and(|current| !placement_requires_cell_rebuild(old_placement, current))
        {
            continue;
        }
        dirty_keys.insert(world_cell_key(old_placement.pos));
        if let Some(current) = app.placements.get(*index) {
            dirty_keys.insert(world_cell_key(current.pos));
        }
    }
    rebuild_render_cells_for_keys(app, &dirty_keys);
}

pub(crate) fn rebuild_render_cells_for_keys(
    app: &mut AppState,
    dirty_keys: &HashSet<WorldCellKey>,
) {
    if dirty_keys.is_empty() {
        return;
    }
    if (!app.options.fast_vbo && !has_material_previews()) || app.options.vbo_selected_only {
        rebuild_render_cells(app);
        return;
    }

    invalidate_lod_audit(app);
    invalidate_missing_texture_review(app);
    // Scan the map once, then pack and validate only affected active cells.
    // Keep the global active LOD references: a parent may live in another cell.
    let (affected, lod_ids) =
        placement_cell_rebuild_source(&app.placements, &app.element_states, dirty_keys);
    let ambient_lift = scene_ambient_lift_from_timecyc(&app.timecyc);

    // Build replacements before releasing currently drawable cells. Runtime
    // residency uploads them on demand from their compact CPU payload.
    let replacement_world_cells = build_world_cells(
        &affected,
        &app.definitions,
        &app.meshes,
        app.options.vbo_immediate,
        &lod_ids,
        false,
        ambient_lift,
    );
    let replacement_lod_world_cells = build_world_cells(
        &affected,
        &app.definitions,
        &app.meshes,
        app.options.vbo_immediate,
        &lod_ids,
        true,
        ambient_lift,
    );
    let expected_buckets = |want_lod| {
        affected
            .iter()
            .filter(|placement| {
                !placement_is_background_scenery(placement)
                    && !placement_follows_camera(placement, &app.definitions)
                    && placement_is_lod(placement, &lod_ids) == want_lod
                    && app
                        .meshes
                        .contains_key(&placement_mesh_key(placement, &app.definitions))
            })
            .map(|placement| {
                (
                    world_cell_key(placement.pos),
                    placement_world_draw_distance_tier_for_render(
                        placement,
                        &app.definitions,
                        want_lod,
                    ),
                )
            })
            .collect::<HashSet<_>>()
    };
    let replacement_buckets_complete =
        |expected: HashSet<(WorldCellKey, WorldDrawDistanceTier)>, cells: &[WorldCell]| {
            expected.iter().all(|(key, tier)| {
                cells
                    .iter()
                    .any(|cell| cell.key == *key && cell.draw_distance_tier == *tier)
            })
        };
    if !replacement_buckets_complete(expected_buckets(false), &replacement_world_cells)
        || !replacement_buckets_complete(expected_buckets(true), &replacement_lod_world_cells)
    {
        // A requested bucket with renderable placements but no replacement is
        // a build failure, not an empty (vacated) cell.
        delete_render_cells(&[], &replacement_world_cells);
        delete_render_cells(&[], &replacement_lod_world_cells);
        return;
    }

    let mut displaced_vbos = HashSet::new();
    app.world_cells.retain(|cell| {
        let keep = !dirty_keys.contains(&cell.key);
        if !keep && cell.vbo != 0 {
            displaced_vbos.insert(cell.vbo);
        }
        keep
    });
    app.lod_world_cells.retain(|cell| {
        let keep = !dirty_keys.contains(&cell.key);
        if !keep && cell.vbo != 0 {
            displaced_vbos.insert(cell.vbo);
        }
        keep
    });
    unsafe {
        for vbo in displaced_vbos {
            gl::DeleteBuffers(1, &vbo);
        }
    }

    app.world_cells.extend(replacement_world_cells);
    app.lod_world_cells.extend(replacement_lod_world_cells);
    app.world_cells
        .sort_unstable_by(|a, b| b.vertices.cmp(&a.vertices));
    app.lod_world_cells
        .sort_unstable_by(|a, b| b.vertices.cmp(&a.vertices));
}

pub(crate) fn rebuild_render_cells_with_mesh_lift(app: &mut AppState, refresh_mesh_lift: bool) {
    invalidate_lod_audit(app);
    invalidate_missing_texture_review(app);
    let active = active_placements(&app.placements, &app.element_states);
    let world_source = if app.options.vbo_selected_only && !active.is_empty() {
        &active[0..1]
    } else {
        active.as_slice()
    };
    delete_render_cells(&app.scene_cells, &app.world_cells);
    delete_render_cells(&app.lod_scene_cells, &app.lod_world_cells);
    let lod_ids = collect_lod_ids(world_source);
    app.lod_ids = collect_lod_ids(&app.placements);
    let ambient_lift = scene_ambient_lift_from_timecyc(&app.timecyc);
    if refresh_mesh_lift {
        rebuild_mesh_part_lists_with_lift(&mut app.meshes, ambient_lift);
    }
    rebuild_placement_2dfx_index(app);
    // Only build the cell set the active render path will draw (see finish()).
    if app.options.fast_vbo || has_material_previews() {
        app.scene_cells = Vec::new();
        app.lod_scene_cells = Vec::new();
        app.world_cells = build_world_cells(
            world_source,
            &app.definitions,
            &app.meshes,
            app.options.vbo_immediate,
            &lod_ids,
            false,
            ambient_lift,
        );
        app.lod_world_cells = build_world_cells(
            world_source,
            &app.definitions,
            &app.meshes,
            app.options.vbo_immediate,
            &lod_ids,
            true,
            ambient_lift,
        );
    } else {
        app.world_cells = Vec::new();
        app.lod_world_cells = Vec::new();
        app.scene_cells = build_scene_cells(
            world_source,
            &app.definitions,
            &app.meshes,
            &lod_ids,
            false,
            ambient_lift,
        );
        app.lod_scene_cells = build_scene_cells(
            world_source,
            &app.definitions,
            &app.meshes,
            &lod_ids,
            true,
            ambient_lift,
        );
    }
    app.loaded_message = format!(
        "Loaded {} DFF meshes, {} placements ({} active), {} cells, {} batches, {} textures, {} textured parts in {:.2}s",
        app.meshes.len(),
        app.placements.len(),
        app.element_states
            .iter()
            .filter(|state| !state.deleted && !state.hidden)
            .count(),
        app.world_cells.len(),
        app.world_cells
            .iter()
            .map(|cell| cell.batches.len())
            .sum::<usize>(),
        app.textures.len(),
        app.textured_parts,
        app.load_seconds
    );
}

pub(crate) fn saved_content_snapshot(app: &AppState) -> SavedContentSnapshot {
    SavedContentSnapshot {
        zones: app.zones.clone(),
        map_documents: app.map_documents.clone(),
        placements: app.placements.clone(),
        definitions: app.definitions.clone(),
        readonly_definition_ids: app.readonly_definition_ids.clone(),
        element_states: app.element_states.clone(),
        lights: app.lights.clone(),
        water_planes: app.water_planes.clone(),
        cull_zones: app.cull_zones.clone(),
        race_tracks: app.race.tracks.clone(),
        race_radar_path: app.race.radar_path.clone(),
        race_world_size: app.race.world_size,
        race_world_center_x: app.race.world_center_x,
        race_world_center_y: app.race.world_center_y,
        collisions: app.collisions.clone(),
        editing_asset: app.editing.asset.clone(),
        editing_modified_entries: app.editing.modified_entries.clone(),
        editing_deleted_entries: app.editing.deleted_entries.clone(),
        editing_added_entries: app.editing.added_entries.clone(),
    }
}

pub(crate) fn water_history_snapshot(app: &AppState) -> WaterHistorySnapshot {
    WaterHistorySnapshot {
        planes: app.water_planes.clone(),
        selected: app.selected_water,
        selected_planes: app.selected_water_planes.clone(),
    }
}

pub(crate) fn cull_history_snapshot(app: &AppState) -> CullHistorySnapshot {
    CullHistorySnapshot {
        zones: app.cull_zones.clone(),
        selected: app.selected_cull,
    }
}

pub(crate) fn light_history_snapshot(app: &AppState) -> LightHistorySnapshot {
    LightHistorySnapshot {
        lights: app.lights.clone(),
        selected: app.selected_light,
    }
}

pub(crate) fn race_history_snapshot(app: &AppState) -> RaceHistorySnapshot {
    RaceHistorySnapshot {
        tracks: app.race.tracks.clone(),
        selected_track: app.race.selected_track,
        place_mode: app.race.place_mode,
        selected_point: app.race.selected_point,
        place_z: app.race.place_z,
        default_radius: app.race.default_radius,
        radar_path: app.race.radar_path.clone(),
        world_size: app.race.world_size,
        world_center_x: app.race.world_center_x,
        world_center_y: app.race.world_center_y,
    }
}

pub(crate) fn world_history_snapshot(app: &AppState) -> WorldHistorySnapshot {
    WorldHistorySnapshot {
        zones: app.zones.clone(),
        map_documents: app.map_documents.clone(),
        placements: app.placements.clone(),
        definitions: app.definitions.clone(),
        readonly_definition_ids: app.readonly_definition_ids.clone(),
        element_states: app.element_states.clone(),
        selected: app.selected,
        selected_elements: app.selected_elements.clone(),
        selected_element_order: app.selected_element_order.clone(),
        selected_col_face: app.selected_col_face,
        selected_col_vertex: app.selected_col_vertex,
    }
}

pub(crate) fn placement_transform_history_snapshot<I>(
    app: &AppState,
    indices: I,
) -> PlacementTransformHistorySnapshot
where
    I: IntoIterator<Item = usize>,
{
    PlacementTransformHistorySnapshot {
        placements: indices
            .into_iter()
            .filter_map(|index| {
                app.placements
                    .get(index)
                    .cloned()
                    .map(|placement| (index, placement))
            })
            .collect(),
    }
}

fn active_lod_ids(placements: &[Placement], states: &[ElementState]) -> HashSet<String> {
    placements
        .iter()
        .zip(states)
        .filter(|(_, state)| !state.deleted && !state.hidden)
        .filter_map(|(placement, _)| placement.attrs.get("lodParent"))
        .map(|parent| parent.trim())
        .filter(|parent| !parent.is_empty() && !parent.eq_ignore_ascii_case("self"))
        .map(str::to_ascii_lowercase)
        .collect()
}

pub(crate) fn local_world_history_snapshot(
    app: &AppState,
    indices: impl IntoIterator<Item = usize>,
    definition_ids: impl IntoIterator<Item = String>,
) -> LocalWorldHistorySnapshot {
    LocalWorldHistorySnapshot {
        placement_count: app.placements.len(),
        placements: placement_transform_history_snapshot(app, indices).placements,
        element_states: app.element_states.clone(),
        definitions: definition_ids
            .into_iter()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(|id| {
                let definition = app.definitions.get(&id).cloned();
                let readonly = app.readonly_definition_ids.contains(&id);
                (id, definition, readonly)
            })
            .collect(),
        zones: app.zones.clone(),
        world_edits_document: app
            .map_documents
            .iter()
            .find(|document| document.path == "maps/world_edits.map")
            .cloned(),
        active_lod_ids: active_lod_ids(&app.placements, &app.element_states),
        selection: selection_history_snapshot(app),
        selected_col_face: app.selected_col_face,
        selected_col_vertex: app.selected_col_vertex,
    }
}

fn local_world_snapshot_after(
    app: &AppState,
    before: &LocalWorldHistorySnapshot,
) -> LocalWorldHistorySnapshot {
    local_world_history_snapshot(
        app,
        before
            .placements
            .iter()
            .map(|(index, _)| *index)
            .chain(before.placement_count..app.placements.len()),
        before.definitions.iter().map(|(id, _, _)| id.clone()),
    )
}

fn definition_requires_cell_rebuild(old: Option<&Definition>, new: Option<&Definition>) -> bool {
    match (old, new) {
        (Some(old), Some(new)) => {
            old.id != new.id
                || definition_flag_enabled(old, "disable_backface_culling")
                    != definition_flag_enabled(new, "disable_backface_culling")
                || [
                    "dff",
                    "txd",
                    "flags",
                    "overrideFlags",
                    "overrideflags",
                    "override_flags",
                    "followCamera",
                    "followcamera",
                    "follow_camera",
                    "follow-camera",
                    "lodDistance",
                    "drawDistance",
                    "timeIn",
                    "timeOut",
                ]
                .iter()
                .any(|key| old.attrs.get(*key) != new.attrs.get(*key))
        }
        (None, None) => false,
        _ => true,
    }
}

/// State changes and additions affect their cells. A changed LOD relationship
/// can also reclassify parents elsewhere; include those cells in the update.
fn local_world_dirty_keys(
    placements: &[Placement],
    states: &[ElementState],
    definitions: &HashMap<String, Definition>,
    before: &LocalWorldHistorySnapshot,
    lod_ids: &HashSet<String>,
) -> HashSet<WorldCellKey> {
    let mut keys = HashSet::new();
    for (index, old) in &before.placements {
        if placements
            .get(*index)
            .is_none_or(|current| placement_requires_cell_rebuild(old, current))
        {
            keys.insert(world_cell_key(old.pos));
            if let Some(current) = placements.get(*index) {
                keys.insert(world_cell_key(current.pos));
            }
        }
    }
    let changed_definitions: HashSet<_> = before
        .definitions
        .iter()
        .filter(|(id, old, _)| definition_requires_cell_rebuild(old.as_ref(), definitions.get(id)))
        .map(|(id, _, _)| id.as_str())
        .collect();
    let lod_changed = before.active_lod_ids != *lod_ids;
    for (index, placement) in placements.iter().enumerate() {
        if index >= before.placement_count
            || states.get(index) != before.element_states.get(index)
            || changed_definitions.contains(placement.id.as_str())
            || (lod_changed
                && placement_is_lod(placement, &before.active_lod_ids)
                    != placement_is_lod(placement, lod_ids))
        {
            keys.insert(world_cell_key(placement.pos));
        }
    }
    keys
}

fn refresh_local_placement_2dfx_index(app: &mut AppState, before: &LocalWorldHistorySnapshot) {
    update_local_placement_2dfx_index(
        &mut app.placement_2dfx_indices,
        &mut app.placement_2dfx_index_placement_count,
        &app.placements,
        &app.definitions,
        &app.meshes,
        before,
    );
}

fn update_local_placement_2dfx_index(
    indices: &mut Vec<usize>,
    indexed_count: &mut usize,
    placements: &[Placement],
    definitions: &HashMap<String, Definition>,
    meshes: &HashMap<String, RenderMesh>,
    before: &LocalWorldHistorySnapshot,
) {
    if *indexed_count != before.placement_count && *indexed_count != placements.len() {
        *indices = build_placement_2dfx_indices(placements, definitions, meshes);
        *indexed_count = placements.len();
        return;
    }
    let mut changed: BTreeSet<_> = before
        .placements
        .iter()
        .filter(|(index, old)| {
            placements
                .get(*index)
                .is_none_or(|p| p.id != old.id || p.dff != old.dff)
        })
        .map(|(index, _)| *index)
        .chain(before.placement_count..placements.len())
        .collect();
    let mesh_changed: HashSet<_> = before
        .definitions
        .iter()
        .filter(|(id, old, _)| {
            let new = definitions.get(id);
            match (old.as_ref(), new) {
                (Some(old), Some(new)) => {
                    old.id != new.id
                        || old.attrs.get("dff") != new.attrs.get("dff")
                        || old.attrs.get("txd") != new.attrs.get("txd")
                }
                (None, None) => false,
                _ => true,
            }
        })
        .map(|(id, _, _)| id.as_str())
        .collect();
    if !mesh_changed.is_empty() {
        changed.extend(
            placements
                .iter()
                .enumerate()
                .filter(|(_, p)| mesh_changed.contains(p.id.as_str()))
                .map(|(index, _)| index),
        );
    }
    indices.retain(|index| *index < placements.len() && !changed.contains(index));
    for index in changed {
        if let Some(placement) = placements.get(index) {
            if meshes
                .get(&placement_mesh_key(placement, definitions))
                .is_some_and(|mesh| !mesh.effects_2dfx.is_empty())
            {
                indices.push(index);
            }
        }
    }
    indices.sort_unstable();
    *indexed_count = placements.len();
}

fn refresh_local_world_edit(app: &mut AppState, before: &LocalWorldHistorySnapshot) {
    let lod_ids = active_lod_ids(&app.placements, &app.element_states);
    let keys = local_world_dirty_keys(
        &app.placements,
        &app.element_states,
        &app.definitions,
        before,
        &lod_ids,
    );
    app.lod_ids = collect_lod_ids(&app.placements);
    refresh_local_placement_2dfx_index(app, before);
    app.outliner_labels.resize(app.placements.len(), None);
    for (index, old) in &before.placements {
        if app
            .placements
            .get(*index)
            .is_none_or(|p| p.id != old.id || p.dff != old.dff)
        {
            invalidate_outliner_label(app, *index);
        }
    }
    // Structural edits and type/LOD/group edits change the outliner. Visibility
    // bits and transforms do not change its entries or cached labels.
    if app.placements.len() != before.placement_count
        || lod_ids != before.active_lod_ids
        || before.placements.iter().any(|(index, old)| {
            app.placements.get(*index).is_none_or(|p| {
                p.id != old.id
                    || p.dff != old.dff
                    || p.tag != old.tag
                    || placement_group(p) != placement_group(old)
            })
        })
    {
        rebuild_outliner_filter(app);
    }
    rebuild_render_cells_for_keys(app, &keys);
    invalidate_validation_cache(app);
}

pub(crate) fn commit_local_world_history(
    app: &mut AppState,
    label: impl Into<String>,
    before: LocalWorldHistorySnapshot,
) {
    let after = local_world_snapshot_after(app, &before);
    if before != after {
        refresh_local_world_edit(app, &before);
        push_scoped_history(
            app,
            label,
            UndoState::LocalWorld(before),
            UndoState::LocalWorld(after),
        );
    }
}

fn restore_local_placements(placements: &mut Vec<Placement>, snapshot: &LocalWorldHistorySnapshot) {
    placements.truncate(snapshot.placement_count);
    for (index, placement) in &snapshot.placements {
        if let Some(current) = placements.get_mut(*index) {
            *current = placement.clone();
        } else {
            // Appended records are stored in ascending index order for redo.
            assert_eq!(*index, placements.len());
            placements.push(placement.clone());
        }
    }
}

fn restore_world_edits_document(
    documents: &mut Vec<crate::resource::mta_maps::MapDocument>,
    saved: Option<&crate::resource::mta_maps::MapDocument>,
) {
    if let Some(document) = saved {
        if let Some(current) = documents.iter_mut().find(|d| d.path == document.path) {
            current.clone_from(document);
        } else {
            documents.push(document.clone());
        }
    } else {
        documents.retain(|d| d.path != "maps/world_edits.map");
    }
}

fn apply_local_world_history_snapshot(app: &mut AppState, snapshot: &LocalWorldHistorySnapshot) {
    // Capture removed tail records before truncating, so undo vacates every
    // cell touched by a placement/duplicate (including an Alt-drag copy).
    let before = local_world_history_snapshot(
        app,
        snapshot
            .placements
            .iter()
            .map(|(index, _)| *index)
            .chain(snapshot.placement_count..app.placements.len()),
        snapshot.definitions.iter().map(|(id, _, _)| id.clone()),
    );
    restore_local_placements(&mut app.placements, snapshot);
    app.element_states.clone_from(&snapshot.element_states);
    app.zones.clone_from(&snapshot.zones);
    for (id, definition, readonly) in &snapshot.definitions {
        if let Some(definition) = definition {
            app.definitions.insert(id.clone(), definition.clone());
        } else {
            app.definitions.remove(id);
        }
        if *readonly {
            app.readonly_definition_ids.insert(id.clone());
        } else {
            app.readonly_definition_ids.remove(id);
        }
    }
    restore_world_edits_document(
        &mut app.map_documents,
        snapshot.world_edits_document.as_ref(),
    );
    apply_selection_history_snapshot(app, snapshot.selection.clone());
    app.selected_col_face = snapshot.selected_col_face;
    app.selected_col_vertex = snapshot.selected_col_vertex;
    refresh_local_world_edit(app, &before);
}

pub(crate) fn selection_history_snapshot(app: &AppState) -> SelectionHistorySnapshot {
    SelectionHistorySnapshot {
        selected: app.selected,
        selected_elements: app.selected_elements.clone(),
        selected_element_order: app.selected_element_order.clone(),
        selected_group: app.selected_group.clone(),
    }
}

pub(crate) fn collision_history_snapshot(app: &AppState) -> CollisionHistorySnapshot {
    CollisionHistorySnapshot {
        collisions: app.collisions.clone(),
        selected_col_face: app.selected_col_face,
        selected_col_vertex: app.selected_col_vertex,
        pending_writes: app.pending_col_writes.clone(),
        pending_replacements: app.pending_replacement_assets.clone(),
    }
}

pub(crate) fn editing_history_snapshot(app: &AppState) -> EditingHistorySnapshot {
    EditingHistorySnapshot {
        asset: app.editing.asset.clone(),
        modified_entries: app.editing.modified_entries.clone(),
        deleted_entries: app.editing.deleted_entries.clone(),
        added_entries: app.editing.added_entries.clone(),
        material_emitters: None,
        shadow_casting: None,
    }
}

pub(crate) fn editing_history_snapshot_with_material_sidecars(
    app: &AppState,
) -> EditingHistorySnapshot {
    let mut snapshot = editing_history_snapshot(app);
    snapshot.material_emitters = Some(app.material_emitters.clone());
    snapshot.shadow_casting = Some(app.shadow_casting.clone());
    snapshot
}

fn global_transform_history_snapshot(app: &AppState) -> GlobalTransformHistorySnapshot {
    GlobalTransformHistorySnapshot {
        world: world_history_snapshot(app),
        lights: light_history_snapshot(app),
        race: race_history_snapshot(app),
        water: water_history_snapshot(app),
    }
}

pub(crate) fn world_editing_history_snapshot(app: &AppState) -> WorldEditingHistorySnapshot {
    WorldEditingHistorySnapshot {
        world: world_history_snapshot(app),
        editing: editing_history_snapshot(app),
    }
}

pub(crate) fn world_race_history_snapshot(app: &AppState) -> WorldRaceHistorySnapshot {
    WorldRaceHistorySnapshot {
        world: world_history_snapshot(app),
        race: race_history_snapshot(app),
    }
}

pub(crate) fn capture_vertex_colors<'a, I>(app: &AppState, mesh_keys: I) -> VertexColorSnapshot
where
    I: IntoIterator<Item = &'a String>,
{
    let mut meshes = HashMap::new();
    for mesh_key in mesh_keys {
        if meshes.contains_key(mesh_key) {
            continue;
        }
        let Some(mesh) = app.meshes.get(mesh_key) else {
            continue;
        };
        let parts: Vec<Vec<[u8; 6]>> = mesh
            .parts
            .iter()
            .map(|part| part.cpu_vertices.iter().map(pack_vertex_prelight).collect())
            .collect();
        meshes.insert(mesh_key.clone(), parts);
    }
    VertexColorSnapshot { meshes }
}

pub(crate) fn snapshot_with_vertex_colors<'a, I>(
    app: &AppState,
    mesh_keys: I,
) -> VertexColorHistorySnapshot
where
    I: IntoIterator<Item = &'a String>,
{
    VertexColorHistorySnapshot {
        colors: capture_vertex_colors(app, mesh_keys),
    }
}

// Push an undo entry for a one-shot vertex lighting operation (bake, AO,
// clear). `before` must be taken with snapshot_with_vertex_colors() prior to
// mutating the meshes.
pub(crate) fn commit_vertex_lighting_history<'a, I>(
    app: &mut AppState,
    label: impl Into<String>,
    before: VertexColorHistorySnapshot,
    mesh_keys: I,
) where
    I: IntoIterator<Item = &'a String>,
{
    let after = snapshot_with_vertex_colors(app, mesh_keys);
    if before == after {
        return;
    }
    push_scoped_history(
        app,
        label,
        UndoState::VertexColors(before),
        UndoState::VertexColors(after),
    );
}

fn editing_txd_assets_equal(a: &EditingTxdState, b: &EditingTxdState) -> bool {
    a.name == b.name
        && a.selected == b.selected
        && a.textures
            .iter()
            .map(|texture| (&texture.name, texture.width, texture.height, texture.format))
            .eq(b
                .textures
                .iter()
                .map(|texture| (&texture.name, texture.width, texture.height, texture.format)))
}

fn editing_dff_open_models_equal(a: &EditingDffState, b: &EditingDffState) -> bool {
    a.active_open_model == b.active_open_model
        && a.multi_select == b.multi_select
        && a.open_models.len() == b.open_models.len()
        && a.open_models.iter().zip(&b.open_models).all(|(a, b)| {
            a.name == b.name
                && a.placement_index == b.placement_index
                && a.to_workspace == b.to_workspace
                && a.raw == b.raw
                && a.txd_context == b.txd_context
                && a.txd_source_label == b.txd_source_label
                && a.vehicle_collision_override == b.vehicle_collision_override
                && a.dirty == b.dirty
                && a.selected_face == b.selected_face
                && a.selected_faces == b.selected_faces
                && a.selected_edges == b.selected_edges
                && a.selected_vertex == b.selected_vertex
                && a.selected_vertices == b.selected_vertices
        })
}

fn editing_assets_equal(a: &Option<EditingAsset>, b: &Option<EditingAsset>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(EditingAsset::Txd(a)), Some(EditingAsset::Txd(b))) => editing_txd_assets_equal(a, b),
        (Some(EditingAsset::Dff(a)), Some(EditingAsset::Dff(b))) => {
            a.name == b.name
                && a.raw == b.raw
                && a.txd_context == b.txd_context
                && a.txd_source_label == b.txd_source_label
                && a.selected_material == b.selected_material
                && a.selected_face == b.selected_face
                && a.selected_faces == b.selected_faces
                && a.selected_edges == b.selected_edges
                && a.select_mode == b.select_mode
                && a.selected_vertex == b.selected_vertex
                && a.selected_vertices == b.selected_vertices
                && a.boolean_box == b.boolean_box
                && a.dirty == b.dirty
                && a.normalized_warning == b.normalized_warning
                && a.normalized_rewrite_confirmed == b.normalized_rewrite_confirmed
                && a.vehicle_collision_override == b.vehicle_collision_override
                && editing_dff_open_models_equal(a, b)
        }
        (Some(EditingAsset::Col(a)), Some(EditingAsset::Col(b))) => {
            a.name == b.name
                && a.mesh == b.mesh
                && a.bytes == b.bytes
                && a.selected_face == b.selected_face
                && a.selected_faces == b.selected_faces
                && a.selected_edges == b.selected_edges
                && a.select_mode == b.select_mode
                && (a.primitive_scroll - b.primitive_scroll).abs() < f32::EPSILON
                && a.selected_vertex == b.selected_vertex
                && a.selected_primitive == b.selected_primitive
                && a.box_pick_enabled == b.box_pick_enabled
                && a.selected_vertices == b.selected_vertices
                && a.dirty == b.dirty
        }
        _ => false,
    }
}

fn editing_asset_content_equal(a: &Option<EditingAsset>, b: &Option<EditingAsset>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(EditingAsset::Txd(a)), Some(EditingAsset::Txd(b))) => {
            a.name == b.name
                && a.textures
                    .iter()
                    .map(|texture| (&texture.name, texture.width, texture.height, texture.format))
                    .eq(b.textures.iter().map(|texture| {
                        (&texture.name, texture.width, texture.height, texture.format)
                    }))
        }
        (Some(EditingAsset::Dff(a)), Some(EditingAsset::Dff(b))) => {
            a.name == b.name
                && a.raw == b.raw
                && a.txd_context == b.txd_context
                && a.txd_source_label == b.txd_source_label
                && a.vehicle_collision_override == b.vehicle_collision_override
                && a.open_models.len() == b.open_models.len()
                && a.open_models.iter().zip(&b.open_models).all(|(a, b)| {
                    a.name == b.name
                        && a.placement_index == b.placement_index
                        && a.to_workspace == b.to_workspace
                        && a.raw == b.raw
                        && a.txd_context == b.txd_context
                        && a.txd_source_label == b.txd_source_label
                        && a.vehicle_collision_override == b.vehicle_collision_override
                })
        }
        (Some(EditingAsset::Col(a)), Some(EditingAsset::Col(b))) => {
            a.name == b.name && a.mesh == b.mesh && a.bytes == b.bytes
        }
        _ => false,
    }
}

fn app_has_content_changes_from_snapshot(app: &AppState, snapshot: &SavedContentSnapshot) -> bool {
    snapshot.zones != app.zones
        || snapshot.map_documents != app.map_documents
        || snapshot.placements != app.placements
        || snapshot.definitions != app.definitions
        || snapshot.readonly_definition_ids != app.readonly_definition_ids
        || snapshot.element_states != app.element_states
        || snapshot.lights != app.lights
        || snapshot.water_planes != app.water_planes
        || snapshot.cull_zones != app.cull_zones
        || snapshot.race_tracks != app.race.tracks
        || snapshot.race_radar_path != app.race.radar_path
        || (snapshot.race_world_size - app.race.world_size).abs() > f32::EPSILON
        || (snapshot.race_world_center_x - app.race.world_center_x).abs() > f32::EPSILON
        || (snapshot.race_world_center_y - app.race.world_center_y).abs() > f32::EPSILON
        || snapshot.collisions != app.collisions
        || snapshot.editing_modified_entries != app.editing.modified_entries
        || snapshot.editing_deleted_entries != app.editing.deleted_entries
        || snapshot.editing_added_entries != app.editing.added_entries
        || !editing_asset_content_equal(&snapshot.editing_asset, &app.editing.asset)
}

pub(crate) fn has_unsaved_changes(app: &AppState) -> bool {
    if app.loaded_autosave {
        return true;
    }
    if app.material_classes_dirty {
        return true;
    }
    if app.safe_collisions_dirty {
        return true;
    }
    if pending_replacement_asset_count(app) > 0 {
        return true;
    }
    if !app.pending_col_writes.is_empty() {
        return true;
    }
    if !app.pending_txd_writes.is_empty() {
        return true;
    }
    if !app.pending_asset_deletes.is_empty() {
        return true;
    }
    if !app.pending_vertex_light_meshes.is_empty() {
        return true;
    }
    app.saved_snapshot
        .as_ref()
        .is_some_and(|saved| app_has_content_changes_from_snapshot(app, saved))
}

pub(crate) fn autosave_dirty_snapshot(app: &AppState) -> AutosaveDirtySnapshot {
    AutosaveDirtySnapshot {
        content: saved_content_snapshot(app),
        material_classes: app.material_classes.clone(),
        material_classes_dirty: app.material_classes_dirty,
        safe_collisions: app.safe_collisions.clone(),
        safe_collisions_dirty: app.safe_collisions_dirty,
        pending_replacement_assets: app.pending_replacement_assets.clone(),
        pending_col_writes: app.pending_col_writes.clone(),
        pending_txd_writes: app.pending_txd_writes.clone(),
        pending_asset_deletes: app.pending_asset_deletes.clone(),
        pending_vertex_light_meshes: app.pending_vertex_light_meshes.clone(),
    }
}

fn app_matches_autosave_dirty_snapshot(app: &AppState, snapshot: &AutosaveDirtySnapshot) -> bool {
    !app_has_content_changes_from_snapshot(app, &snapshot.content)
        && snapshot.material_classes == app.material_classes
        && snapshot.material_classes_dirty == app.material_classes_dirty
        && snapshot.safe_collisions == app.safe_collisions
        && snapshot.safe_collisions_dirty == app.safe_collisions_dirty
        && snapshot.pending_replacement_assets == app.pending_replacement_assets
        && snapshot.pending_col_writes == app.pending_col_writes
        && snapshot.pending_txd_writes == app.pending_txd_writes
        && snapshot.pending_asset_deletes == app.pending_asset_deletes
        && snapshot.pending_vertex_light_meshes == app.pending_vertex_light_meshes
}

pub(crate) fn has_unautosaved_changes(app: &AppState) -> bool {
    if !has_unsaved_changes(app) {
        return false;
    }
    app.autosave_dirty_snapshot
        .as_ref()
        .is_none_or(|snapshot| !app_matches_autosave_dirty_snapshot(app, snapshot))
}

pub(crate) fn mark_saved_snapshot(app: &mut AppState) {
    app.saved_snapshot = Some(saved_content_snapshot(app));
}

pub(crate) fn mark_water_saved(app: &mut AppState) {
    let water_planes = app.water_planes.clone();
    if let Some(snapshot) = app.saved_snapshot.as_mut() {
        snapshot.water_planes = water_planes;
    } else {
        app.saved_snapshot = Some(saved_content_snapshot(app));
    }
}

pub(crate) fn mark_race_saved(app: &mut AppState) {
    let tracks = app.race.tracks.clone();
    let radar_path = app.race.radar_path.clone();
    let world_size = app.race.world_size;
    let center_x = app.race.world_center_x;
    let center_y = app.race.world_center_y;
    if let Some(snapshot) = app.saved_snapshot.as_mut() {
        snapshot.race_tracks = tracks;
        snapshot.race_radar_path = radar_path;
        snapshot.race_world_size = world_size;
        snapshot.race_world_center_x = center_x;
        snapshot.race_world_center_y = center_y;
    } else {
        app.saved_snapshot = Some(saved_content_snapshot(app));
    }
}

pub(crate) fn mark_editing_img_saved(app: &mut AppState, modified_entry_keys: &[String]) {
    let collision_updates = modified_entry_keys
        .iter()
        .filter(|key| key.ends_with(".col"))
        .filter_map(|key| {
            app.collisions
                .get(key)
                .cloned()
                .map(|mesh| (key.clone(), mesh))
        })
        .collect::<Vec<_>>();
    let editing_asset = app.editing.asset.clone();
    let modified_entries = app.editing.modified_entries.clone();
    let deleted_entries = app.editing.deleted_entries.clone();
    let added_entries = app.editing.added_entries.clone();
    let Some(snapshot) = app.saved_snapshot.as_mut() else {
        return;
    };
    snapshot.editing_asset = editing_asset;
    snapshot.editing_modified_entries = modified_entries;
    snapshot.editing_deleted_entries = deleted_entries;
    snapshot.editing_added_entries = added_entries;
    for (key, mesh) in collision_updates {
        snapshot.collisions.insert(key, mesh);
    }
}

fn prune_selected_race_point(app: &mut AppState) {
    if app.race.selected_track >= app.race.tracks.len() {
        app.race.selected_track = if app.race.tracks.is_empty() {
            NO_SELECTION
        } else {
            app.race.tracks.len() - 1
        };
        app.race.selected_point = None;
        return;
    }
    let Some(point) = app.race.selected_point else {
        return;
    };
    let Some(track) = app.race.tracks.get(app.race.selected_track) else {
        app.race.selected_point = None;
        return;
    };
    let count = match app.race.place_mode {
        RacePlaceMode::Checkpoint => track.checkpoints.len(),
        RacePlaceMode::Overlay => track.overlay.len(),
        RacePlaceMode::Path => track.path.len(),
        RacePlaceMode::Start => 1,
        RacePlaceMode::None => 0,
    };
    if point >= count {
        app.race.selected_point = None;
    }
}

fn apply_vertex_color_snapshot(app: &mut AppState, colors: VertexColorSnapshot) {
    let mode = app.bake_settings.light_mode;
    let mut changed: Vec<String> = Vec::new();
    for (mesh_key, parts) in &colors.meshes {
        if let Some(mesh) = app.meshes.get_mut(mesh_key) {
            for (part, saved) in mesh.parts.iter_mut().zip(parts.iter()) {
                for (vertex, packed) in part.cpu_vertices.iter_mut().zip(saved.iter()) {
                    unpack_vertex_prelight(vertex, *packed);
                    apply_vertex_bake_display(vertex, mode);
                }
            }
            changed.push(mesh_key.clone());
        }
    }
    if !changed.is_empty() {
        rebuild_mesh_part_lists(&mut app.meshes);
        queue_vertex_lighting_meshes(app, changed.iter());
    }
}

fn apply_water_history_snapshot(app: &mut AppState, snapshot: WaterHistorySnapshot) {
    app.water_planes = snapshot.planes;
    app.selected_water = snapshot.selected;
    app.selected_water_planes = snapshot.selected_planes;
    prune_selected_water(app);
    app.hovered_water = None;
    app.hovered_water_edge = None;
    app.water_edge_drag = None;
    clamp_water_scroll(app);
    invalidate_validation_cache(app);
}

fn apply_cull_history_snapshot(app: &mut AppState, snapshot: CullHistorySnapshot) {
    app.cull_zones = snapshot.zones;
    app.selected_cull = snapshot
        .selected
        .min(app.cull_zones.len().saturating_sub(1));
    clamp_cull_scroll(app);
    app.cull_face_drag = None;
    app.cull_hovered_face = None;
}

fn apply_light_history_snapshot(app: &mut AppState, snapshot: LightHistorySnapshot) {
    app.lights = snapshot.lights;
    app.selected_light = snapshot.selected.min(app.lights.len().saturating_sub(1));
    app.light_color_drag_before = None;
    app.light_temperature_drag_before = None;
    invalidate_validation_cache(app);
}

fn apply_race_history_snapshot(app: &mut AppState, snapshot: RaceHistorySnapshot) {
    app.race.tracks = snapshot.tracks;
    app.race.selected_track = snapshot
        .selected_track
        .min(app.race.tracks.len().saturating_sub(1));
    app.race.place_mode = snapshot.place_mode;
    app.race.selected_point = snapshot.selected_point;
    app.race.place_z = snapshot.place_z;
    app.race.default_radius = snapshot.default_radius;
    if app.race.radar_path != snapshot.radar_path {
        app.race.radar_tex = None;
        app.race.radar_tex_key.clear();
    }
    app.race.radar_path = snapshot.radar_path;
    app.race.world_size = snapshot.world_size;
    app.race.world_center_x = snapshot.world_center_x;
    app.race.world_center_y = snapshot.world_center_y;
    prune_selected_race_point(app);
    invalidate_validation_cache(app);
}

fn apply_world_history_snapshot(app: &mut AppState, snapshot: WorldHistorySnapshot) {
    app.zones = snapshot.zones;
    app.map_documents = snapshot.map_documents;
    if app
        .placement_destination
        .as_ref()
        .is_some_and(|d| !app.zones.contains(d))
    {
        app.placement_destination = None;
    }
    app.placements = snapshot.placements;
    app.definitions = snapshot.definitions;
    app.readonly_definition_ids = snapshot.readonly_definition_ids;
    app.element_states = snapshot.element_states;
    app.selected = snapshot
        .selected
        .min(app.placements.len().saturating_sub(1));
    app.selected_elements = snapshot.selected_elements;
    app.selected_element_order = snapshot.selected_element_order;
    app.selected_col_face = snapshot.selected_col_face;
    app.selected_col_vertex = snapshot.selected_col_vertex;
    prune_selected_elements(app);
    invalidate_outliner_labels(app);
    rebuild_outliner_filter(app);
    rebuild_render_cells(app);
    invalidate_validation_cache(app);
}

fn apply_placement_transform_history_snapshot(
    app: &mut AppState,
    snapshot: PlacementTransformHistorySnapshot,
) {
    let before = placement_transform_history_snapshot(
        app,
        snapshot.placements.iter().map(|(index, _)| *index),
    );
    for (index, placement) in snapshot.placements {
        if let Some(current) = app.placements.get_mut(index) {
            *current = placement;
        }
    }
    rebuild_render_cells_for_placement_transforms(app, &before);
    invalidate_validation_cache(app);
}

pub(crate) fn apply_selection_history_snapshot(
    app: &mut AppState,
    snapshot: SelectionHistorySnapshot,
) {
    app.selected = snapshot.selected;
    app.selected_elements = snapshot.selected_elements;
    app.selected_element_order = snapshot.selected_element_order;
    app.selected_group = snapshot.selected_group;
    prune_selected_elements(app);
}

fn apply_collision_history_snapshot(app: &mut AppState, snapshot: CollisionHistorySnapshot) {
    clear_collision_render_cache(app);
    clear_material_plugins();
    app.collisions = snapshot.collisions;
    app.selected_col_face = snapshot.selected_col_face;
    app.selected_col_vertex = snapshot.selected_col_vertex;
    app.pending_col_writes = snapshot.pending_writes;
    app.pending_replacement_assets = snapshot.pending_replacements;
    invalidate_validation_cache(app);
    rebuild_render_cells(app);
}

fn apply_editing_history_snapshot(app: &mut AppState, snapshot: EditingHistorySnapshot) {
    app.editing.asset = snapshot.asset;
    if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
        dff.fracture_preview_started_at = None;
    }
    app.editing.modified_entries = snapshot.modified_entries;
    app.editing.deleted_entries = snapshot.deleted_entries;
    app.editing.added_entries = snapshot.added_entries;
    if let Some(material_emitters) = snapshot.material_emitters {
        app.material_emitters = material_emitters;
    }
    if let Some(shadow_casting) = snapshot.shadow_casting {
        app.shadow_casting = shadow_casting;
    }
    invalidate_validation_cache(app);
}

fn apply_global_transform_history_snapshot(
    app: &mut AppState,
    snapshot: GlobalTransformHistorySnapshot,
) {
    apply_world_history_snapshot(app, snapshot.world);
    apply_light_history_snapshot(app, snapshot.lights);
    apply_race_history_snapshot(app, snapshot.race);
    apply_water_history_snapshot(app, snapshot.water);
}

fn apply_world_editing_history_snapshot(app: &mut AppState, snapshot: WorldEditingHistorySnapshot) {
    apply_world_history_snapshot(app, snapshot.world);
    apply_editing_history_snapshot(app, snapshot.editing);
}

fn apply_world_race_history_snapshot(app: &mut AppState, snapshot: WorldRaceHistorySnapshot) {
    apply_world_history_snapshot(app, snapshot.world);
    apply_race_history_snapshot(app, snapshot.race);
}

fn apply_lod_generation_history_snapshot(
    app: &mut AppState,
    snapshot: LodGenerationHistorySnapshot,
) {
    apply_world_history_snapshot(app, snapshot.world);
    let artifacts: Box<dyn Iterator<Item = &LodGenerationHistoryArtifact>> =
        if snapshot.generated_present {
            Box::new(snapshot.artifacts.iter())
        } else {
            Box::new(snapshot.artifacts.iter().rev())
        };
    for artifact in artifacts {
        if snapshot.generated_present {
            for (key, name, bytes) in &artifact.assets {
                app.pending_replacement_assets
                    .insert(key.clone(), (name.clone(), bytes.clone()));
                app.pending_asset_deletes.remove(key);
            }
            let replacements = artifact
                .assets
                .iter()
                .map(|(_, name, bytes)| (name.clone(), bytes.clone()))
                .collect::<Vec<_>>();
            let _ = upsert_replacement_assets(&wip_root_path(&app.root), &replacements);
            if let Some(mesh) = artifact.mesh.as_ref() {
                app.meshes.insert(artifact.mesh_key.clone(), mesh.clone());
            }
            invalidate_collision_render_cache(app, &artifact.collision_key);
            app.collisions
                .insert(artifact.collision_key.clone(), artifact.collision.clone());
        } else {
            for (key, _, _) in &artifact.assets {
                app.pending_replacement_assets.remove(key);
                // The background generator has already written these entries
                // to the WIP archive. Marking them deleted keeps Undo durable:
                // the next Save cannot accidentally promote orphan LOD assets.
                app.pending_asset_deletes.insert(key.clone());
            }
            app.meshes.remove(&artifact.mesh_key);
            invalidate_collision_render_cache(app, &artifact.collision_key);
            app.collisions.remove(&artifact.collision_key);
        }
    }
    for (key, (name, bytes)) in &snapshot.txd_assets {
        if let Some(bytes) = bytes {
            app.pending_replacement_assets
                .insert(key.clone(), (name.clone(), bytes.clone()));
            app.pending_asset_deletes.remove(key);
            let _ = upsert_replacement_assets(
                &wip_root_path(&app.root),
                &[(name.clone(), bytes.clone())],
            );
            app.pending_txd_writes.insert(key.clone());
            reindex_staged_txd(app, name);
        } else {
            app.pending_replacement_assets.remove(key);
            app.pending_txd_writes.remove(key);
            remove_txd_from_texture_index(app, name);
            if !snapshot.generated_present {
                app.pending_asset_deletes.insert(key.clone());
            }
        }
    }
    app.loaded_wip = true;
    rebuild_render_cells(app);
    invalidate_validation_cache(app);
}

fn apply_undo_state(app: &mut AppState, state: &UndoState) {
    match state {
        UndoState::Water(snapshot) => apply_water_history_snapshot(app, snapshot.clone()),
        UndoState::Cull(snapshot) => apply_cull_history_snapshot(app, snapshot.clone()),
        UndoState::Lights(snapshot) => apply_light_history_snapshot(app, snapshot.clone()),
        UndoState::Race(snapshot) => apply_race_history_snapshot(app, snapshot.clone()),
        UndoState::World(snapshot) => apply_world_history_snapshot(app, snapshot.clone()),
        UndoState::LocalWorld(snapshot) => apply_local_world_history_snapshot(app, snapshot),
        UndoState::PlacementTransforms(snapshot) => {
            apply_placement_transform_history_snapshot(app, snapshot.clone());
        }
        UndoState::Selection(snapshot) => apply_selection_history_snapshot(app, snapshot.clone()),
        UndoState::Collision(snapshot) => apply_collision_history_snapshot(app, snapshot.clone()),
        UndoState::Editing(snapshot) => apply_editing_history_snapshot(app, snapshot.clone()),
        UndoState::GlobalTransform(snapshot) => {
            apply_global_transform_history_snapshot(app, snapshot.clone());
        }
        UndoState::VertexColors(snapshot) => {
            apply_vertex_color_snapshot(app, snapshot.colors.clone());
        }
        UndoState::WorldEditing(snapshot) => {
            apply_world_editing_history_snapshot(app, snapshot.clone());
        }
        UndoState::WorldRace(snapshot) => {
            apply_world_race_history_snapshot(app, snapshot.clone());
        }
        UndoState::LodGeneration(snapshot) => {
            apply_lod_generation_history_snapshot(app, snapshot.clone());
        }
    }
}

pub(crate) fn commit_water_history(
    app: &mut AppState,
    label: impl Into<String>,
    before: WaterHistorySnapshot,
) {
    let after = water_history_snapshot(app);
    if before == after {
        return;
    }
    invalidate_validation_cache(app);
    app.undo_stack.push(UndoEntry {
        label: label.into(),
        before: UndoState::Water(before),
        after: UndoState::Water(after),
    });
    if app.undo_stack.len() > 64 {
        app.undo_stack.remove(0);
    }
    app.redo_stack.clear();
}

pub(crate) fn commit_cull_history(
    app: &mut AppState,
    label: impl Into<String>,
    before: CullHistorySnapshot,
) {
    let after = cull_history_snapshot(app);
    if before != after {
        push_scoped_history(app, label, UndoState::Cull(before), UndoState::Cull(after));
    }
}

fn push_scoped_history(
    app: &mut AppState,
    label: impl Into<String>,
    before: UndoState,
    after: UndoState,
) {
    invalidate_validation_cache(app);
    app.undo_stack.push(UndoEntry {
        label: label.into(),
        before,
        after,
    });
    if app.undo_stack.len() > 64 {
        app.undo_stack.remove(0);
    }
    app.redo_stack.clear();
}

pub(crate) fn clear_history_for_external_change(app: &mut AppState) {
    app.undo_stack.clear();
    app.redo_stack.clear();
}

fn undo_state_uses_editing_context(state: &UndoState) -> bool {
    matches!(state, UndoState::Editing(_) | UndoState::WorldEditing(_))
}

/// Editing snapshots own a complete active asset but not its archive/path
/// context. Drop only those entries when the active Editing asset changes so
/// Undo cannot resurrect an asset from an unrelated archive.
pub(crate) fn clear_editing_history(app: &mut AppState) {
    let is_editing_entry = |entry: &UndoEntry| {
        undo_state_uses_editing_context(&entry.before)
            || undo_state_uses_editing_context(&entry.after)
    };
    app.undo_stack.retain(|entry| !is_editing_entry(entry));
    app.redo_stack.retain(|entry| !is_editing_entry(entry));
}

pub(crate) fn commit_lod_generation_history(
    app: &mut AppState,
    before_world: WorldHistorySnapshot,
    artifacts: Vec<LodGenerationHistoryArtifact>,
    txd_before: BTreeMap<String, (String, Option<Vec<u8>>)>,
) {
    if artifacts.is_empty() {
        return;
    }
    let artifacts = Arc::new(artifacts);
    let after_world = world_history_snapshot(app);
    let txd_after = txd_before
        .iter()
        .map(|(key, (name, _))| {
            let bytes = app
                .pending_replacement_assets
                .get(key)
                .map(|(_, bytes)| bytes.clone());
            (key.clone(), (name.clone(), bytes))
        })
        .collect();
    let count = artifacts.len();
    push_scoped_history(
        app,
        if count == 1 {
            "Generate LOD".to_string()
        } else {
            format!("Generate {count} LODs")
        },
        UndoState::LodGeneration(LodGenerationHistorySnapshot {
            world: before_world,
            artifacts: Arc::clone(&artifacts),
            generated_present: false,
            txd_assets: txd_before,
        }),
        UndoState::LodGeneration(LodGenerationHistorySnapshot {
            world: after_world,
            artifacts,
            generated_present: true,
            txd_assets: txd_after,
        }),
    );
}

pub(crate) fn commit_light_history(
    app: &mut AppState,
    label: impl Into<String>,
    before: LightHistorySnapshot,
) {
    let after = light_history_snapshot(app);
    if before != after {
        push_scoped_history(
            app,
            label,
            UndoState::Lights(before),
            UndoState::Lights(after),
        );
    }
}

pub(crate) fn commit_race_history(
    app: &mut AppState,
    label: impl Into<String>,
    before: RaceHistorySnapshot,
) {
    let after = race_history_snapshot(app);
    if before != after {
        push_scoped_history(app, label, UndoState::Race(before), UndoState::Race(after));
    }
}

pub(crate) fn commit_world_history(
    app: &mut AppState,
    label: impl Into<String>,
    before: WorldHistorySnapshot,
) {
    crate::resource::zones::reconcile_definition_zones(app);
    let after = world_history_snapshot(app);
    if before != after {
        push_scoped_history(
            app,
            label,
            UndoState::World(before),
            UndoState::World(after),
        );
    }
}

pub(crate) fn commit_placement_transform_history(
    app: &mut AppState,
    label: impl Into<String>,
    before: PlacementTransformHistorySnapshot,
) {
    let after = placement_transform_history_snapshot(
        app,
        before.placements.iter().map(|(index, _)| *index),
    );
    if before != after {
        push_scoped_history(
            app,
            label,
            UndoState::PlacementTransforms(before),
            UndoState::PlacementTransforms(after),
        );
    }
}

pub(crate) fn commit_selection_history(
    app: &mut AppState,
    label: impl Into<String>,
    before: SelectionHistorySnapshot,
) {
    let after = selection_history_snapshot(app);
    if let Some((before, after)) = selection_history_states(before, after) {
        push_scoped_history(app, label, before, after);
    }
}

fn selection_history_states(
    before: SelectionHistorySnapshot,
    after: SelectionHistorySnapshot,
) -> Option<(UndoState, UndoState)> {
    (before != after).then(|| (UndoState::Selection(before), UndoState::Selection(after)))
}

pub(crate) fn commit_world_history_snapshots(
    app: &mut AppState,
    label: impl Into<String>,
    before: WorldHistorySnapshot,
    after: WorldHistorySnapshot,
) {
    if before != after {
        push_scoped_history(
            app,
            label,
            UndoState::World(before),
            UndoState::World(after),
        );
    }
}

pub(crate) fn commit_collision_history(
    app: &mut AppState,
    label: impl Into<String>,
    before: CollisionHistorySnapshot,
) {
    let after = collision_history_snapshot(app);
    if before != after {
        push_scoped_history(
            app,
            label,
            UndoState::Collision(before),
            UndoState::Collision(after),
        );
    }
}

fn editing_history_snapshots_equal(
    before: &EditingHistorySnapshot,
    after: &EditingHistorySnapshot,
) -> bool {
    before.modified_entries == after.modified_entries
        && before.deleted_entries == after.deleted_entries
        && before.added_entries == after.added_entries
        && before.material_emitters == after.material_emitters
        && before.shadow_casting == after.shadow_casting
        && editing_assets_equal(&before.asset, &after.asset)
}

pub(crate) fn commit_editing_history(
    app: &mut AppState,
    label: impl Into<String>,
    before: EditingHistorySnapshot,
) {
    if let (Some(EditingAsset::Dff(before_dff)), Some(EditingAsset::Dff(current_dff))) =
        (before.asset.as_ref(), app.editing.asset.as_mut())
    {
        current_dff.fracture_preview_started_at = None;
        if before_dff.raw.vertices != current_dff.raw.vertices
            || before_dff.raw.triangles != current_dff.raw.triangles
        {
            for component in &mut current_dff.raw.components {
                if let Some(breakable) = component.breakable.as_mut() {
                    breakable.stale = true;
                }
            }
        }
    }
    let mut after = editing_history_snapshot(app);
    if before.material_emitters.is_some() {
        after.material_emitters = Some(app.material_emitters.clone());
    }
    if before.shadow_casting.is_some() {
        after.shadow_casting = Some(app.shadow_casting.clone());
    }
    if !editing_history_snapshots_equal(&before, &after) {
        push_scoped_history(
            app,
            label,
            UndoState::Editing(before),
            UndoState::Editing(after),
        );
    }
}

fn commit_global_transform_history(
    app: &mut AppState,
    label: impl Into<String>,
    before: GlobalTransformHistorySnapshot,
) {
    let after = global_transform_history_snapshot(app);
    let unchanged = before.world == after.world
        && before.lights == after.lights
        && before.race == after.race
        && before.water == after.water;
    if !unchanged {
        push_scoped_history(
            app,
            label,
            UndoState::GlobalTransform(before),
            UndoState::GlobalTransform(after),
        );
    }
}

fn commit_world_editing_history(
    app: &mut AppState,
    label: impl Into<String>,
    before: WorldEditingHistorySnapshot,
) {
    let after = world_editing_history_snapshot(app);
    let unchanged = before.world == after.world
        && editing_history_snapshots_equal(&before.editing, &after.editing);
    if !unchanged {
        push_scoped_history(
            app,
            label,
            UndoState::WorldEditing(before),
            UndoState::WorldEditing(after),
        );
    }
}

fn commit_world_race_history(
    app: &mut AppState,
    label: impl Into<String>,
    before: WorldRaceHistorySnapshot,
) {
    let after = world_race_history_snapshot(app);
    if before != after {
        push_scoped_history(
            app,
            label,
            UndoState::WorldRace(before),
            UndoState::WorldRace(after),
        );
    }
}

pub(crate) fn commit_scoped_history(
    app: &mut AppState,
    label: impl Into<String>,
    before: ScopedHistorySnapshot,
) {
    let label = label.into();
    match before {
        ScopedHistorySnapshot::None => {}
        ScopedHistorySnapshot::Water(snapshot) => commit_water_history(app, label, snapshot),
        ScopedHistorySnapshot::Cull(snapshot) => commit_cull_history(app, label, snapshot),
        ScopedHistorySnapshot::Lights(snapshot) => commit_light_history(app, label, snapshot),
        ScopedHistorySnapshot::Race(snapshot) => commit_race_history(app, label, snapshot),
        ScopedHistorySnapshot::World(snapshot) => commit_world_history(app, label, snapshot),
        ScopedHistorySnapshot::LocalWorld(snapshot) => {
            commit_local_world_history(app, label, snapshot)
        }
        ScopedHistorySnapshot::PlacementTransforms(snapshot) => {
            commit_placement_transform_history(app, label, snapshot);
        }
        ScopedHistorySnapshot::Collision(snapshot) => {
            commit_collision_history(app, label, snapshot);
        }
        ScopedHistorySnapshot::Editing(snapshot) => commit_editing_history(app, label, snapshot),
        ScopedHistorySnapshot::WorldEditing(snapshot) => {
            commit_world_editing_history(app, label, snapshot);
        }
        ScopedHistorySnapshot::WorldRace(snapshot) => {
            commit_world_race_history(app, label, snapshot);
        }
    }
}

pub(crate) fn commit_race_2d_point_move(
    app: &mut AppState,
    label: impl Into<String>,
    drag: Race2dPointDrag,
) {
    let after = race_history_snapshot(app);
    let mut before = after.clone();
    let label = label.into();
    match drag {
        Race2dPointDrag::Overlay {
            track,
            point,
            before: before_pos,
        } => {
            let Some(after_pos) = after
                .tracks
                .get(track)
                .and_then(|race_track| race_track.overlay.get(point))
            else {
                return;
            };
            if *after_pos == before_pos {
                return;
            }
            if let Some(race_track) = before.tracks.get_mut(track) {
                if let Some(pos) = race_track.overlay.get_mut(point) {
                    *pos = before_pos;
                }
            }
            before.selected_track = track;
            before.place_mode = RacePlaceMode::Overlay;
            before.selected_point = Some(point);
        }
        Race2dPointDrag::Checkpoint {
            track,
            point,
            before: before_checkpoint,
        } => {
            let Some(after_checkpoint) = after
                .tracks
                .get(track)
                .and_then(|race_track| race_track.checkpoints.get(point))
            else {
                return;
            };
            if *after_checkpoint == before_checkpoint {
                return;
            }
            if let Some(race_track) = before.tracks.get_mut(track) {
                if let Some(checkpoint) = race_track.checkpoints.get_mut(point) {
                    *checkpoint = before_checkpoint;
                }
            }
            before.selected_track = track;
            before.place_mode = RacePlaceMode::Checkpoint;
            before.selected_point = Some(point);
        }
    }
    push_scoped_history(app, label, UndoState::Race(before), UndoState::Race(after));
}

pub(crate) fn undo(app: &mut AppState) {
    let Some(entry) = app.undo_stack.pop() else {
        return;
    };
    apply_undo_state(app, &entry.before);
    app.redo_stack.push(entry);
}

pub(crate) fn redo(app: &mut AppState) {
    let Some(entry) = app.redo_stack.pop() else {
        return;
    };
    apply_undo_state(app, &entry.after);
    app.undo_stack.push(entry);
}

#[cfg(test)]
mod selection_history_tests {
    use super::*;

    fn test_placement(dff: &str) -> Placement {
        Placement {
            id: String::new(),
            dff: dff.to_string(),
            zone: String::new(),
            tag: String::new(),
            attrs: BTreeMap::new(),
            pos: V3::default(),
            rot: V3::default(),
        }
    }

    fn test_render_mesh(has_2dfx: bool) -> RenderMesh {
        RenderMesh {
            parts: Vec::new(),
            bounds: Bounds {
                min: Vec3::ZERO,
                max: Vec3::ZERO,
            },
            material_animations: Vec::new(),
            uv_animations: Vec::new(),
            effects_2dfx: has_2dfx
                .then(|| Dff2dEffect::default())
                .into_iter()
                .collect(),
            components: Vec::new(),
            component_pivots: Vec::new(),
        }
    }

    fn local_test_snapshot(
        placements: &[Placement],
        states: &[ElementState],
    ) -> LocalWorldHistorySnapshot {
        LocalWorldHistorySnapshot {
            placement_count: placements.len(),
            placements: Vec::new(),
            element_states: states.to_vec(),
            definitions: Vec::new(),
            zones: Vec::new(),
            world_edits_document: None,
            active_lod_ids: active_lod_ids(placements, states),
            selection: SelectionHistorySnapshot {
                selected: NO_SELECTION,
                selected_elements: BTreeSet::new(),
                selected_element_order: Vec::new(),
                selected_group: None,
            },
            selected_col_face: None,
            selected_col_vertex: 0,
        }
    }

    #[test]
    fn local_history_undo_redo_restores_appended_copies_and_vacates_their_cells() {
        let original = vec![test_placement("existing")];
        let original_states = vec![ElementState::default()];
        let before = local_test_snapshot(&original, &original_states);
        let mut appended = test_placement("copy");
        appended.pos.x = 800.0;
        let mut placements = vec![original[0].clone(), appended.clone()];
        let states = vec![ElementState::default(); 2];
        let mut after = local_test_snapshot(&placements, &states);
        after.placements.push((1, appended.clone()));
        assert!(
            local_world_dirty_keys(
                &placements,
                &states,
                &HashMap::new(),
                &before,
                &HashSet::new()
            ) == HashSet::from([world_cell_key(appended.pos)])
        );
        restore_local_placements(&mut placements, &before);
        assert!(placements == original);
        assert!(
            local_world_dirty_keys(
                &placements,
                &original_states,
                &HashMap::new(),
                &after,
                &HashSet::new()
            ) == HashSet::from([world_cell_key(appended.pos)])
        );
        restore_local_placements(&mut placements, &after);
        assert!(placements == vec![original[0].clone(), appended]);
    }

    #[test]
    fn local_delete_and_hide_rebuild_remote_lod_cells_when_classification_changes() {
        let mut child = test_placement("child");
        child.attrs.insert("lodParent".into(), "parent".into());
        let mut parent = test_placement("parent");
        parent.id = "parent".into();
        parent.pos.x = 4096.0;
        let placements = vec![child.clone(), parent.clone(), test_placement("unrelated")];
        let initial = vec![ElementState::default(); 3];
        let before = local_test_snapshot(&placements, &initial);
        for changed in [
            ElementState {
                hidden: false,
                deleted: true,
            },
            ElementState {
                hidden: true,
                deleted: false,
            },
        ] {
            let states = vec![changed, initial[1], initial[2]];
            let new_lods = active_lod_ids(&placements, &states);
            let keys =
                local_world_dirty_keys(&placements, &states, &HashMap::new(), &before, &new_lods);
            assert!(keys == HashSet::from([world_cell_key(child.pos), world_cell_key(parent.pos)]));
            let (affected, _) = placement_cell_rebuild_source(&placements, &states, &keys);
            assert!(affected == vec![parent.clone(), placements[2].clone()]);
        }
    }

    #[test]
    fn local_definition_edit_rebuilds_all_instances_of_that_model_only() {
        let mut second = test_placement("shared");
        second.pos.x = 4096.0;
        let mut unrelated = test_placement("other");
        unrelated.pos.x = 8192.0;
        let mut first = test_placement("shared");
        first.id = "shared".into();
        second.id = first.id.clone();
        unrelated.id = "other".into();
        let placements = vec![first.clone(), second.clone(), unrelated];
        let states = vec![ElementState::default(); 3];
        let mut before = local_test_snapshot(&placements, &states);
        let old = Definition {
            id: "shared".into(),
            zone: String::new(),
            attrs: BTreeMap::new(),
        };
        before
            .definitions
            .push(("shared".into(), Some(old.clone()), true));
        let mut updated = old;
        updated
            .attrs
            .insert("flags".into(), "disable_backface_culling".into());
        let definitions = HashMap::from([("shared".into(), updated)]);
        assert!(
            local_world_dirty_keys(&placements, &states, &definitions, &before, &HashSet::new())
                == HashSet::from([world_cell_key(first.pos), world_cell_key(second.pos)])
        );
    }

    #[test]
    fn placing_readonly_model_with_zone_override_does_not_repack_existing_instances() {
        let mut existing = test_placement("1337");
        existing.id = "1337".into();
        let old = Definition {
            id: "1337".into(),
            zone: "SA".into(),
            attrs: BTreeMap::from([("dff".into(), "bin".into()), ("source".into(), "SA".into())]),
        };
        let original_states = vec![ElementState::default()];
        let mut before = local_test_snapshot(&[existing.clone()], &original_states);
        before
            .definitions
            .push((old.id.clone(), Some(old.clone()), true));
        let mut updated = old;
        updated.zone = "custom".into();
        updated.attrs.insert("zone".into(), "custom".into());
        updated.attrs.remove("source");
        let definitions = HashMap::from([("1337".into(), updated)]);
        let mut added = existing.clone();
        added.pos.x = 8192.0;
        let placements = vec![existing, added.clone()];
        let states = vec![ElementState::default(); 2];
        assert!(
            local_world_dirty_keys(&placements, &states, &definitions, &before, &HashSet::new())
                == HashSet::from([world_cell_key(added.pos)])
        );
    }

    #[test]
    fn inherited_sa_backface_flag_override_requires_repacking() {
        let old = Definition {
            id: "1337".into(),
            zone: "SA".into(),
            attrs: BTreeMap::from([("gtaFlags".into(), (1u64 << 21).to_string())]),
        };
        let mut new = old.clone();
        new.attrs
            .insert("disable_backface_culling".into(), "false".into());
        assert!(definition_requires_cell_rebuild(Some(&old), Some(&new)));
    }

    #[test]
    fn group_and_physics_metadata_edits_do_not_repack_geometry() {
        let original = test_placement("road");
        let mut changed = original.clone();
        changed
            .attrs
            .insert(EDITOR_GROUP_ATTR.into(), "Group 1".into());
        changed.attrs.insert("mass".into(), "100".into());
        changed.attrs.insert("uniqueID".into(), "123".into());
        assert!(!placement_requires_cell_rebuild(&original, &changed));
        changed.attrs.insert("scale".into(), "2".into());
        assert!(placement_requires_cell_rebuild(&original, &changed));
    }

    #[test]
    fn local_history_restores_sa_removal_document_without_copying_other_maps() {
        use crate::resource::mta_maps::MapDocument;
        let unrelated = MapDocument {
            path: "maps/track.map".into(),
            text: "<map>track</map>".into(),
        };
        let removed = MapDocument {
            path: "maps/world_edits.map".into(),
            text: "<map><removeWorldObject/></map>".into(),
        };
        let mut documents = vec![unrelated.clone(), removed.clone()];
        restore_world_edits_document(&mut documents, None);
        assert!(documents == vec![unrelated.clone()]);
        restore_world_edits_document(&mut documents, Some(&removed));
        assert!(documents == vec![unrelated, removed]);
    }

    #[test]
    fn local_2dfx_index_tracks_added_replaced_and_removed_placements() {
        let mut placements = vec![test_placement("lamp"), test_placement("road")];
        let states = vec![ElementState::default(); 2];
        let mut before = local_test_snapshot(&placements, &states);
        before.placements.push((1, placements[1].clone()));
        let definitions = HashMap::new();
        let meshes = HashMap::from([
            (
                placement_mesh_key(&placements[0], &definitions),
                test_render_mesh(true),
            ),
            (
                placement_mesh_key(&placements[1], &definitions),
                test_render_mesh(false),
            ),
        ]);
        let mut indices = vec![0];
        let mut count = 2;
        placements[1].dff = "lamp".into();
        placements.push(test_placement("lamp"));
        update_local_placement_2dfx_index(
            &mut indices,
            &mut count,
            &placements,
            &definitions,
            &meshes,
            &before,
        );
        assert_eq!(indices, vec![0, 1, 2]);
        let mut after = local_test_snapshot(&placements, &vec![ElementState::default(); 3]);
        after.placements = vec![(1, placements[1].clone()), (2, placements[2].clone())];
        restore_local_placements(&mut placements, &before);
        update_local_placement_2dfx_index(
            &mut indices,
            &mut count,
            &placements,
            &definitions,
            &meshes,
            &after,
        );
        assert_eq!(indices, vec![0]);
        assert_eq!(count, 2);
        // An inconsistent external index still recovers with a full scan.
        count = 99;
        indices = vec![100];
        update_local_placement_2dfx_index(
            &mut indices,
            &mut count,
            &placements,
            &definitions,
            &meshes,
            &before,
        );
        assert_eq!(indices, vec![0]);
        assert_eq!(count, 2);
    }

    #[test]
    fn placement_cell_source_keeps_neighbors_and_remote_lod_references() {
        let mut moved = test_placement("edited");
        moved.pos.x = 256.0;
        let neighbor = test_placement("neighbor");
        let mut remote = test_placement("detail");
        remote.pos.x = 4096.0;
        remote
            .attrs
            .insert("lodParent".into(), " RemoteLOD ".into());
        let mut hidden = test_placement("hidden");
        hidden.attrs.insert("lodParent".into(), "HiddenLOD".into());
        let deleted = test_placement("deleted");
        let placements = vec![moved.clone(), neighbor.clone(), remote, hidden, deleted];
        let states = vec![
            ElementState::default(),
            ElementState::default(),
            ElementState::default(),
            ElementState {
                hidden: true,
                deleted: false,
            },
            ElementState {
                hidden: false,
                deleted: true,
            },
        ];
        let keys = HashSet::from([WorldCellKey { x: 0, y: 0 }, WorldCellKey { x: 1, y: 0 }]);
        let (affected, lod_ids) = placement_cell_rebuild_source(&placements, &states, &keys);
        assert!(affected == vec![moved, neighbor]);
        assert_eq!(lod_ids, HashSet::from(["remotelod".to_string()]));
    }

    #[test]
    fn placement_cell_source_does_not_clone_a_heavy_map() {
        let mut placements = vec![test_placement("road"); 50_000];
        for (index, placement) in placements.iter_mut().enumerate() {
            placement.pos.x = index as f32 * 256.0;
        }
        let states = vec![ElementState::default(); placements.len()];
        let keys = HashSet::from([WorldCellKey { x: 25_000, y: 0 }]);
        let (affected, _) = placement_cell_rebuild_source(&placements, &states, &keys);
        assert_eq!(affected.len(), 1);
        assert!(affected[0] == placements[25_000]);
        let empty_keys = HashSet::from([WorldCellKey { x: -1, y: 0 }]);
        assert!(
            placement_cell_rebuild_source(&placements, &states, &empty_keys)
                .0
                .is_empty()
        );
    }

    #[test]
    fn local_cell_packing_matches_full_rebuild_after_transform_and_alpha_edit() {
        let mut placements = vec![test_placement("road"); 10_000];
        for (index, placement) in placements.iter_mut().enumerate() {
            placement.pos.x = (index % 100) as f32 * 256.0;
            placement.pos.y = (index / 100) as f32 * 256.0;
        }
        // Move across a cell boundary into an occupied cell, rotate, scale and
        // change transparency. The vacated origin cell must stay empty.
        placements[0].pos.x = 260.0;
        placements[0].rot.z = 45.0;
        placements[0].attrs.insert("scale".into(), "2".into());
        placements[0].attrs.insert("alpha".into(), "128".into());
        let states = vec![ElementState::default(); placements.len()];
        let keys = HashSet::from([WorldCellKey { x: 0, y: 0 }, WorldCellKey { x: 1, y: 0 }]);
        let white = V3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        };
        let mut mesh = test_render_mesh(false);
        mesh.parts.push(RenderPart {
            list: 0,
            vbo: 0,
            vertices: 3,
            material_index: 0,
            component: 0,
            texture: 0,
            lightmap_texture: 0,
            texture_width: 0,
            texture_height: 0,
            texture_name: String::new(),
            texture_fingerprint: None,
            texture_missing: false,
            transparency: TransparencyMode::Opaque,
            alpha: 1.0,
            material_color: white,
            material_ambient: 1.0,
            use_lighting: false,
            vehicle_material_role: None,
            emissive: false,
            face_indices: vec![0],
            cpu_vertices: [
                V3::default(),
                V3 {
                    x: 1.0,
                    ..V3::default()
                },
                V3 {
                    y: 1.0,
                    ..V3::default()
                },
            ]
            .into_iter()
            .map(|pos| Vertex {
                pos,
                normal: V3 {
                    z: 1.0,
                    ..V3::default()
                },
                uv: V2::default(),
                lightmap_uv: V2::default(),
                color: white,
                day_color: white,
                night_color: white,
                base_day_color: white,
                base_night_color: white,
                day_alpha: 1.0,
                night_alpha: 1.0,
                alpha: 1.0,
            })
            .collect(),
        });
        let definitions = HashMap::new();
        let meshes = HashMap::from([(placement_mesh_key(&placements[0], &definitions), mesh)]);
        let (affected, lod_ids) = placement_cell_rebuild_source(&placements, &states, &keys);
        assert_eq!(affected.len(), 2);
        let local = build_world_cells(
            &affected,
            &definitions,
            &meshes,
            false,
            &lod_ids,
            false,
            V3::default(),
        );
        let full = build_world_cells(
            &placements,
            &definitions,
            &meshes,
            false,
            &lod_ids,
            false,
            V3::default(),
        );
        assert_eq!(local.len(), 1);
        assert_eq!(full.len(), 9_999);
        let actual = &local[0];
        let expected = full.iter().find(|cell| cell.key == actual.key).unwrap();
        assert!(actual.key == WorldCellKey { x: 1, y: 0 });
        assert_eq!(actual.placements, expected.placements);
        assert_eq!(actual.vertices, expected.vertices);
        assert_eq!(actual.min, expected.min);
        assert_eq!(actual.max, expected.max);
        assert_eq!(actual.batches.len(), expected.batches.len());
        for (a, b) in actual.batches.iter().zip(&expected.batches) {
            assert_eq!(a.data, b.data);
            assert!(a.transparency == b.transparency);
        }
    }

    #[test]
    fn placement_2dfx_index_contains_only_resolved_effect_meshes() {
        let placements = vec![
            test_placement("lamp"),
            test_placement("road"),
            test_placement("missing"),
            test_placement("lamp"),
        ];
        let definitions = HashMap::new();
        let mut meshes = HashMap::new();
        meshes.insert(
            placement_mesh_key(&placements[0], &definitions),
            test_render_mesh(true),
        );
        meshes.insert(
            placement_mesh_key(&placements[1], &definitions),
            test_render_mesh(false),
        );

        assert_eq!(
            build_placement_2dfx_indices(&placements, &definitions, &meshes),
            vec![0, 3]
        );
    }

    #[test]
    fn matching_texture_selection_preserves_exact_undo_and_redo_states() {
        let before = SelectionHistorySnapshot {
            selected: 2,
            selected_elements: BTreeSet::from([2, 5]),
            selected_element_order: vec![5, 2],
            selected_group: Some("roads".to_string()),
        };
        let after = SelectionHistorySnapshot {
            selected: 2,
            selected_elements: BTreeSet::from([2, 5, 8, 13]),
            selected_element_order: vec![5, 2, 8, 13],
            selected_group: None,
        };

        let (undo_state, redo_state) =
            selection_history_states(before.clone(), after.clone()).unwrap();
        assert!(matches!(undo_state, UndoState::Selection(snapshot) if snapshot == before));
        assert!(matches!(redo_state, UndoState::Selection(snapshot) if snapshot == after));
        assert!(selection_history_states(before.clone(), before).is_none());
    }

    #[test]
    fn editing_history_detects_material_sidecar_only_changes() {
        let before = EditingHistorySnapshot {
            asset: None,
            modified_entries: BTreeMap::new(),
            deleted_entries: BTreeSet::new(),
            added_entries: BTreeSet::new(),
            material_emitters: Some(HashMap::new()),
            shadow_casting: Some(HashMap::new()),
        };
        let mut after = before.clone();
        after
            .shadow_casting
            .as_mut()
            .unwrap()
            .insert(material_emitter_key("lamp.dff", 2), false);

        assert!(!editing_history_snapshots_equal(&before, &after));
        assert!(editing_history_snapshots_equal(&before, &before));
    }

    #[test]
    fn asset_switch_scoping_identifies_only_editing_context_history() {
        let editing = EditingHistorySnapshot {
            asset: None,
            modified_entries: BTreeMap::new(),
            deleted_entries: BTreeSet::new(),
            added_entries: BTreeSet::new(),
            material_emitters: None,
            shadow_casting: None,
        };
        let selection = SelectionHistorySnapshot {
            selected: 0,
            selected_elements: BTreeSet::new(),
            selected_element_order: Vec::new(),
            selected_group: None,
        };

        assert!(undo_state_uses_editing_context(&UndoState::Editing(
            editing
        )));
        assert!(!undo_state_uses_editing_context(&UndoState::Selection(
            selection
        )));
    }
}
