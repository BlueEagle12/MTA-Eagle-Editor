use super::super::*;

#[derive(Debug, Default, PartialEq, Eq)]
struct InstanceLodRemovalSelection {
    detail_indices: Vec<usize>,
    lod_indices: Vec<usize>,
    instance_count: usize,
}

struct InstanceLodRemovalResult {
    target_id: String,
    selection: InstanceLodRemovalSelection,
    before: WorldHistorySnapshot,
    after: WorldHistorySnapshot,
    active_placements: Vec<Placement>,
}

pub(crate) struct InstanceLodRemovalJob {
    rx: mpsc::Receiver<Result<InstanceLodRemovalResult, String>>,
    result: Option<InstanceLodRemovalResult>,
    apply_phase: u8,
    render_lod_ids: HashSet<String>,
    started_at: Instant,
}

fn live_in_snapshot(states: &[ElementState], index: usize) -> bool {
    !states.get(index).is_some_and(|state| state.deleted)
}

/// Find every live instance of `target_id`, the LOD assignments that need to
/// be cleared, and LOD placements that can be removed without affecting a
/// different model.
///
/// A parent id with no remaining references is unambiguous, so all of its live
/// placements can be removed. If another model still references the same
/// repeated parent id, only uniqueID-paired LOD placements are safe to remove.
fn plan_instance_lod_removal(
    placements: &[Placement],
    states: &[ElementState],
    target_id: &str,
) -> InstanceLodRemovalSelection {
    let target_indices: HashSet<usize> = placements
        .iter()
        .enumerate()
        .filter(|(idx, placement)| {
            live_in_snapshot(states, *idx)
                && !is_default_world_placement(placement)
                && placement.id == target_id
        })
        .map(|(idx, _)| idx)
        .collect();
    let mut selection = InstanceLodRemovalSelection {
        instance_count: target_indices.len(),
        ..InstanceLodRemovalSelection::default()
    };
    if target_indices.is_empty() {
        return selection;
    }

    #[derive(Default)]
    struct ParentReferences {
        target: Vec<usize>,
        remaining: Vec<usize>,
    }

    let mut references: HashMap<String, ParentReferences> = HashMap::new();
    for (idx, placement) in placements.iter().enumerate() {
        if !live_in_snapshot(states, idx) {
            continue;
        }
        let Some(parent) = placement
            .attrs
            .get("lodParent")
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        if target_indices.contains(&idx) {
            selection.detail_indices.push(idx);
        }
        if parent.eq_ignore_ascii_case("self") {
            continue;
        }
        let group = references.entry(parent.to_ascii_lowercase()).or_default();
        if target_indices.contains(&idx) {
            group.target.push(idx);
        } else {
            group.remaining.push(idx);
        }
    }

    let mut lod_indices = BTreeSet::new();
    for (parent_id, refs) in references {
        if refs.target.is_empty() {
            continue;
        }
        let candidates: Vec<usize> = placements
            .iter()
            .enumerate()
            .filter(|(idx, placement)| {
                live_in_snapshot(states, *idx)
                    && !target_indices.contains(idx)
                    && placement.id.eq_ignore_ascii_case(&parent_id)
            })
            .map(|(idx, _)| idx)
            .collect();
        if refs.remaining.is_empty() {
            lod_indices.extend(candidates);
            continue;
        }

        // Shared repeated names are only safe to separate when every remaining
        // reference is explicitly paired. Otherwise keep the parent placements
        // and only clear the selected model's assignments.
        let remaining_uids: Option<HashSet<String>> = refs
            .remaining
            .iter()
            .map(|idx| {
                placements[*idx]
                    .attrs
                    .get("uniqueID")
                    .map(|value| value.trim())
                    .filter(|value| !value.is_empty())
                    .map(|value| value.to_ascii_lowercase())
            })
            .collect();
        let Some(remaining_uids) = remaining_uids else {
            continue;
        };
        let target_uids: HashSet<String> = refs
            .target
            .iter()
            .filter_map(|idx| placements[*idx].attrs.get("uniqueID"))
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(|value| value.to_ascii_lowercase())
            .collect();
        for candidate in candidates {
            let Some(uid) = placements[candidate]
                .attrs
                .get("uniqueID")
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
                .map(|value| value.to_ascii_lowercase())
            else {
                continue;
            };
            if target_uids.contains(&uid) && !remaining_uids.contains(&uid) {
                lod_indices.insert(candidate);
            }
        }
    }
    selection.detail_indices.sort_unstable();
    selection.lod_indices = lod_indices.into_iter().collect();
    selection
}

fn build_instance_lod_removal_result(
    target_id: String,
    before: WorldHistorySnapshot,
) -> InstanceLodRemovalResult {
    let selection =
        plan_instance_lod_removal(&before.placements, &before.element_states, &target_id);
    let mut after = before.clone();
    for idx in &selection.detail_indices {
        if let Some(placement) = after.placements.get_mut(*idx) {
            placement.attrs.remove("lodParent");
        }
    }
    for idx in &selection.lod_indices {
        if is_default_world_placement(&after.placements[*idx]) {
            continue;
        }
        if let Some(state) = after.element_states.get_mut(*idx) {
            state.deleted = true;
        }
    }
    let active_placements = active_placements(&after.placements, &after.element_states);
    InstanceLodRemovalResult {
        target_id,
        selection,
        before,
        after,
        active_placements,
    }
}

pub(crate) fn request_remove_lods_from_all_instances(app: &mut AppState) {
    if app.instance_lod_removal_job.is_some() {
        app.status_message = "Instance LOD removal is already running.".to_string();
        return;
    }
    let Some(placement) = app
        .placements
        .get(app.selected)
        .filter(|_| is_live_element(app, app.selected))
    else {
        app.status_message = "Select a live element first.".to_string();
        return;
    };
    let target_id = placement.id.clone();
    app.confirm_dialog = Some(ConfirmDialog {
        action: ConfirmAction::StartInstanceLodRemoval(target_id.clone()),
        title: "Remove LODs From All Instances?".to_string(),
        body: format!(
            "Clear every LOD assignment from all live instances of {target_id}?"
        ),
        detail: "LOD elements that are no longer referenced by another model will also be removed. Shared or ambiguous LOD elements are kept. This can be undone."
            .to_string(),
        primary_label: "Remove All LODs".to_string(),
        secondary_label: None,
        secondary_action: None,
    });
}

pub(crate) fn start_instance_lod_removal(app: &mut AppState, target_id: String) {
    if app.instance_lod_removal_job.is_some() {
        app.status_message = "Instance LOD removal is already running.".to_string();
        return;
    }
    let before = world_history_snapshot(app);
    let (tx, rx) = mpsc::channel();
    let status_target_id = target_id.clone();
    thread::spawn(move || {
        let result =
            std::panic::catch_unwind(|| build_instance_lod_removal_result(target_id, before))
                .map_err(|panic| {
                    panic
                        .downcast_ref::<&str>()
                        .map(|message| (*message).to_string())
                        .or_else(|| panic.downcast_ref::<String>().cloned())
                        .unwrap_or_else(|| "unknown instance LOD removal panic".to_string())
                });
        let _ = tx.send(result);
    });
    app.instance_lod_removal_job = Some(InstanceLodRemovalJob {
        rx,
        result: None,
        apply_phase: 0,
        render_lod_ids: HashSet::new(),
        started_at: Instant::now(),
    });
    app.status_message =
        format!("Finding LODs for every live instance of {status_target_id} in the background...");
}

pub(crate) fn update_instance_lod_removal_job(app: &mut AppState) {
    let Some(mut job) = app.instance_lod_removal_job.take() else {
        return;
    };
    if job.result.is_none() {
        match job.rx.try_recv() {
            Ok(Ok(result)) => {
                if result.selection.detail_indices.is_empty() {
                    app.status_message = format!(
                        "No LOD assignments found on {} live instance(s) of {}.",
                        result.selection.instance_count, result.target_id
                    );
                    return;
                }
                job.result = Some(result);
                app.status_message =
                    "LOD scan finished; applying assignment changes...".to_string();
            }
            Ok(Err(error)) => {
                app.status_message = format!("Instance LOD removal failed: {error}");
                return;
            }
            Err(mpsc::TryRecvError::Empty) => {
                app.instance_lod_removal_job = Some(job);
                return;
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                app.status_message =
                    "Instance LOD removal worker disconnected unexpectedly.".to_string();
                return;
            }
        }
    }

    let result = job
        .result
        .as_ref()
        .expect("instance LOD result exists while applying");
    match job.apply_phase {
        0 => {
            for idx in &result.selection.detail_indices {
                if let Some(placement) = app.placements.get_mut(*idx) {
                    placement.attrs.remove("lodParent");
                }
            }
            for idx in &result.selection.lod_indices {
                if let Some(state) = app.element_states.get_mut(*idx) {
                    state.deleted = true;
                }
            }
            invalidate_outliner_labels(app);
            invalidate_validation_cache(app);
            job.apply_phase = 1;
            app.status_message = "LOD assignments removed; refreshing the outliner...".to_string();
            app.instance_lod_removal_job = Some(job);
        }
        1 => {
            rebuild_outliner_filter(app);
            delete_render_cells(&app.scene_cells, &app.world_cells);
            delete_render_cells(&app.lod_scene_cells, &app.lod_world_cells);
            app.scene_cells = Vec::new();
            app.world_cells = Vec::new();
            app.lod_scene_cells = Vec::new();
            app.lod_world_cells = Vec::new();
            app.lod_ids = collect_lod_ids(&app.placements);
            let world_source =
                if app.options.vbo_selected_only && !result.active_placements.is_empty() {
                    &result.active_placements[0..1]
                } else {
                    result.active_placements.as_slice()
                };
            job.render_lod_ids = collect_lod_ids(world_source);
            job.apply_phase = 2;
            app.status_message =
                "LOD assignments removed; rebuilding detail render cells...".to_string();
            app.instance_lod_removal_job = Some(job);
        }
        2 => {
            let world_source =
                if app.options.vbo_selected_only && !result.active_placements.is_empty() {
                    &result.active_placements[0..1]
                } else {
                    result.active_placements.as_slice()
                };
            let ambient_lift = scene_ambient_lift_from_timecyc(&app.timecyc);
            if app.options.fast_vbo {
                app.world_cells = build_world_cells(
                    world_source,
                    &app.definitions,
                    &app.meshes,
                    app.options.vbo_immediate,
                    &job.render_lod_ids,
                    false,
                    ambient_lift,
                );
            } else {
                app.scene_cells = build_scene_cells(
                    world_source,
                    &app.definitions,
                    &app.meshes,
                    &job.render_lod_ids,
                    false,
                    ambient_lift,
                );
            }
            job.apply_phase = 3;
            app.status_message =
                "LOD assignments removed; rebuilding LOD render cells...".to_string();
            app.instance_lod_removal_job = Some(job);
        }
        3 => {
            let world_source =
                if app.options.vbo_selected_only && !result.active_placements.is_empty() {
                    &result.active_placements[0..1]
                } else {
                    result.active_placements.as_slice()
                };
            let ambient_lift = scene_ambient_lift_from_timecyc(&app.timecyc);
            if app.options.fast_vbo {
                app.lod_world_cells = build_world_cells(
                    world_source,
                    &app.definitions,
                    &app.meshes,
                    app.options.vbo_immediate,
                    &job.render_lod_ids,
                    true,
                    ambient_lift,
                );
            } else {
                app.lod_scene_cells = build_scene_cells(
                    world_source,
                    &app.definitions,
                    &app.meshes,
                    &job.render_lod_ids,
                    true,
                    ambient_lift,
                );
            }
            job.apply_phase = 4;
            app.status_message =
                "LOD render cells refreshed; recording undo history...".to_string();
            app.instance_lod_removal_job = Some(job);
        }
        _ => {
            let result = job
                .result
                .take()
                .expect("instance LOD result exists while finishing");
            commit_world_history_snapshots(
                app,
                "Remove All Instance LODs",
                result.before,
                result.after,
            );
            app.status_message = format!(
                "Removed {} LOD assignment(s) from {} instance(s) of {}; removed {} unshared LOD element(s) in {:.1}s.",
                result.selection.detail_indices.len(),
                result.selection.instance_count,
                result.target_id,
                result.selection.lod_indices.len(),
                job.started_at.elapsed().as_secs_f32()
            );
        }
    }
}

pub(crate) fn open_export_dff_dialog(app: &mut AppState) {
    let Some(dff_name) = selected_dff_name(app) else {
        app.status_message = "Select an element before exporting DFF".to_string();
        return;
    };
    drain_text_input();
    if app.dff_picker_rx.is_some() {
        app.status_message = "DFF file browser is already open".to_string();
        return;
    }
    let default_path = load_last_dff_export_dir().join(dff_name);
    let (tx, rx) = mpsc::channel();
    app.dff_picker_rx = Some(rx);
    app.status_message = "Opening DFF export browser...".to_string();
    thread::spawn(move || {
        let _ = tx.send((DffPickerKind::Export, choose_export_dff_path(default_path)));
    });
}

pub(crate) fn open_export_col_dialog(app: &mut AppState) {
    let Some(col_name) = selected_col_name(app) else {
        app.status_message = "Select an element before exporting COL".to_string();
        return;
    };
    drain_text_input();
    if app.dff_picker_rx.is_some() {
        app.status_message = "File browser is already open".to_string();
        return;
    }
    let default_path = load_last_dff_export_dir().join(col_name);
    let (tx, rx) = mpsc::channel();
    app.dff_picker_rx = Some(rx);
    app.status_message = "Opening COL export browser...".to_string();
    thread::spawn(move || {
        let _ = tx.send((
            DffPickerKind::ExportCol,
            choose_export_col_path(default_path),
        ));
    });
}

pub(crate) fn start_export_dff(app: &mut AppState, target: PathBuf) {
    let Some(dff_name) = selected_dff_name(app) else {
        app.status_message = "Select an element before exporting DFF".to_string();
        return;
    };
    let target = if target.is_dir() {
        target.join(&dff_name)
    } else {
        target
    };
    let Some(parent) = target.parent() else {
        app.status_message = "Export DFF needs a target folder".to_string();
        return;
    };
    if let Err(err) = fs::create_dir_all(parent) {
        app.status_message = format!("Failed to create export folder: {err}");
        return;
    }
    let Some(entry) = find_dff_entry(&app.root, &dff_name) else {
        app.status_message = format!("Could not find {dff_name} in resource IMG archives");
        return;
    };
    let bytes = read_img_entry(&entry);
    if bytes.is_empty() {
        app.status_message = format!("Could not read {dff_name} from IMG archive");
        return;
    }
    match fs::write(&target, bytes) {
        Ok(()) => {
            save_last_dff_export_dir(parent);
            app.status_message = format!(
                "Exported {dff_name} to {}",
                ellipsize(target.to_string_lossy().as_ref(), 52)
            );
        }
        Err(err) => app.status_message = format!("Failed to export {dff_name}: {err}"),
    }
}

pub(crate) fn open_export_named_dff_dialog(app: &mut AppState, dff_name: &str, vehicle_txd: &str) {
    let dff_name = with_ext(dff_name, ".dff");
    let vehicle_txd = vehicle_txd.to_string();
    drain_text_input();
    if app.dff_picker_rx.is_some() {
        app.status_message = "DFF file browser is already open".to_string();
        return;
    }
    let default_path = load_last_dff_export_dir().join(&dff_name);
    let (tx, rx) = mpsc::channel();
    app.dff_picker_rx = Some(rx);
    app.status_message = "Opening DFF export browser...".to_string();
    thread::spawn(move || {
        let _ = tx.send((
            DffPickerKind::ExportNamedDff {
                dff_name,
                vehicle_txd,
            },
            choose_export_dff_path(default_path),
        ));
    });
}

pub(crate) fn start_export_named_dff(
    app: &mut AppState,
    dff_name: String,
    vehicle_txd: String,
    target: PathBuf,
) {
    let dff_name = with_ext(&dff_name, ".dff");
    let target = if target.is_dir() {
        target.join(&dff_name)
    } else {
        target
    };
    let Some(parent) = target.parent() else {
        app.status_message = "Export DFF needs a target folder".to_string();
        return;
    };
    if let Err(err) = fs::create_dir_all(parent) {
        app.status_message = format!("Failed to create export folder: {err}");
        return;
    }
    let Some(entry) = find_dff_entry(&app.root, &dff_name) else {
        app.status_message = format!("Could not find {dff_name} in resource or GTA IMG archives");
        return;
    };
    let bytes = read_img_entry(&entry);
    if bytes.is_empty() {
        app.status_message = format!("Could not read {dff_name} from IMG archive");
        return;
    }
    match fs::write(&target, bytes) {
        Ok(()) => {
            save_last_dff_export_dir(parent);
            app.status_message = format!(
                "Exported {dff_name} to {}",
                ellipsize(target.to_string_lossy().as_ref(), 52)
            );
            export_vehicle_textures_beside_dff(app, &target, &vehicle_txd);
        }
        Err(err) => app.status_message = format!("Failed to export {dff_name}: {err}"),
    }
}

pub(crate) fn start_export_col(app: &mut AppState, target: PathBuf) {
    let Some(col_name) = selected_col_name(app) else {
        app.status_message = "Select an element before exporting COL".to_string();
        return;
    };
    let target = if target.is_dir() {
        target.join(&col_name)
    } else {
        target
    };
    let Some(parent) = target.parent() else {
        app.status_message = "Export COL needs a target folder".to_string();
        return;
    };
    if let Err(err) = fs::create_dir_all(parent) {
        app.status_message = format!("Failed to create export folder: {err}");
        return;
    }
    let Some(entry) = find_col_entry(&app.root, &col_name) else {
        app.status_message = format!("Could not find {col_name} in resource IMG archives");
        return;
    };
    let bytes = read_img_entry(&entry);
    if bytes.is_empty() {
        app.status_message = format!("Could not read {col_name} from IMG archive");
        return;
    }
    match fs::write(&target, bytes) {
        Ok(()) => {
            save_last_dff_export_dir(parent);
            app.status_message = format!(
                "Exported {col_name} to {}",
                ellipsize(target.to_string_lossy().as_ref(), 52)
            );
        }
        Err(err) => app.status_message = format!("Failed to export {col_name}: {err}"),
    }
}

pub(crate) fn export_loose_file_to_path(
    app: &mut AppState,
    source_path: PathBuf,
    asset_name: String,
    vehicle_txd: String,
    target: PathBuf,
) {
    let target = if target.is_dir() {
        target.join(&asset_name)
    } else {
        target
    };
    let Some(parent) = target.parent() else {
        app.status_message = "Export needs a target folder".to_string();
        return;
    };
    if let Err(err) = fs::create_dir_all(parent) {
        app.status_message = format!("Failed to create export folder: {err}");
        return;
    }
    match fs::copy(&source_path, &target) {
        Ok(_) => {
            save_last_dff_export_dir(parent);
            app.status_message = format!(
                "Exported {asset_name} to {}",
                ellipsize(target.to_string_lossy().as_ref(), 52)
            );
            if asset_name.to_ascii_lowercase().ends_with(".dff") {
                export_vehicle_textures_beside_dff(app, &target, &vehicle_txd);
            }
        }
        Err(err) => app.status_message = format!("Failed to export {asset_name}: {err}"),
    }
}

pub(crate) fn export_texture_png_name(name: &str) -> String {
    let trimmed = name.trim();
    let stem = Path::new(trimmed)
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or(trimmed);
    format!("{stem}.PNG")
}

pub(crate) fn blender_script_string(value: &str) -> String {
    format!("{value:?}")
}

pub(crate) fn blender_object_aliases(placement: &Placement) -> Vec<String> {
    let dff_name = with_ext(&placement.dff, ".dff");
    let dff_stem = Path::new(&dff_name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(placement.dff.as_str());
    let mut aliases = Vec::new();
    for value in [
        placement.id.as_str(),
        placement.dff.as_str(),
        dff_name.as_str(),
        dff_stem,
    ] {
        let trimmed = value.trim();
        if !trimmed.is_empty() && !aliases.iter().any(|alias| alias == trimmed) {
            aliases.push(trimmed.to_string());
        }
    }
    aliases
}

pub(crate) fn write_blender_position_script(app: &mut AppState) {
    let indices = selected_live_indices(app);
    if indices.len() < 2 {
        app.status_message = "Select two or more live elements for Blender position".to_string();
        return;
    }
    let base_idx = if indices.contains(&app.selected) {
        app.selected
    } else {
        let Some(first) = indices.first().copied() else {
            return;
        };
        first
    };
    let Some(base) = app.placements.get(base_idx) else {
        app.status_message = "First selected element is missing".to_string();
        return;
    };
    let base_pos = base.pos;
    let base_rot = base.rot;
    let mut ordered_indices = vec![base_idx];
    ordered_indices.extend(indices.into_iter().filter(|idx| *idx != base_idx));
    let mut script = String::new();
    script.push_str("import bpy\n");
    script.push_str("from mathutils import Euler\n\n");
    script.push_str("# Generated by Light Mapper. Run this after importing the selected DFFs.\n");
    script.push_str("# Elements are positioned relative to the first selected mapper element.\n\n");
    script.push_str("def norm(value):\n");
    script.push_str("    return value.lower().removesuffix('.dff')\n\n");
    script.push_str("def find_object(names, used):\n");
    script.push_str("    wanted = {norm(name) for name in names}\n");
    script.push_str("    for obj in bpy.context.scene.objects:\n");
    script.push_str("        if obj.name in used:\n");
    script.push_str("            continue\n");
    script.push_str("        candidates = {obj.name, obj.name.split('.')[0]}\n");
    script.push_str("        data = getattr(obj, 'data', None)\n");
    script.push_str("        if data is not None:\n");
    script.push_str("            candidates.add(data.name)\n");
    script.push_str("            candidates.add(data.name.split('.')[0])\n");
    script.push_str("        if any(norm(candidate) in wanted for candidate in candidates):\n");
    script.push_str("            return obj\n");
    script.push_str("    return None\n\n");
    script.push_str("def weld_object(obj, distance=0.0001):\n");
    script.push_str("    if obj.type != 'MESH':\n");
    script.push_str("        return 0\n");
    script.push_str("    before = len(obj.data.vertices)\n");
    script.push_str("    old_active = bpy.context.view_layer.objects.active\n");
    script.push_str("    old_selected = list(bpy.context.selected_objects)\n");
    script.push_str("    try:\n");
    script.push_str("        if old_active is not None and old_active.mode != 'OBJECT':\n");
    script.push_str("            bpy.ops.object.mode_set(mode='OBJECT')\n");
    script.push_str("        bpy.ops.object.select_all(action='DESELECT')\n");
    script.push_str("        obj.select_set(True)\n");
    script.push_str("        bpy.context.view_layer.objects.active = obj\n");
    script.push_str("        bpy.ops.object.mode_set(mode='EDIT')\n");
    script.push_str("        bpy.ops.mesh.select_mode(type='VERT')\n");
    script.push_str("        bpy.ops.mesh.select_all(action='SELECT')\n");
    script.push_str("        try:\n");
    script.push_str("            bpy.ops.mesh.remove_doubles(threshold=distance)\n");
    script.push_str("        except Exception:\n");
    script.push_str("            bpy.ops.mesh.merge_by_distance(distance=distance)\n");
    script.push_str("        bpy.ops.object.mode_set(mode='OBJECT')\n");
    script.push_str("        for poly in obj.data.polygons:\n");
    script.push_str("            poly.use_smooth = True\n");
    script.push_str("        obj.data.update()\n");
    script.push_str("        return before - len(obj.data.vertices)\n");
    script.push_str("    finally:\n");
    script.push_str(
        "        if bpy.context.object is not None and bpy.context.object.mode != 'OBJECT':\n",
    );
    script.push_str("            bpy.ops.object.mode_set(mode='OBJECT')\n");
    script.push_str("        bpy.ops.object.select_all(action='DESELECT')\n");
    script.push_str("        for selected in old_selected:\n");
    script.push_str("            if selected.name in bpy.context.scene.objects:\n");
    script.push_str("                selected.select_set(True)\n");
    script.push_str(
        "        if old_active is not None and old_active.name in bpy.context.scene.objects:\n",
    );
    script.push_str("            bpy.context.view_layer.objects.active = old_active\n\n");
    script.push_str("placed = 0\n");
    script.push_str("welded = 0\n");
    script.push_str("missing = []\n");
    script.push_str("used = set()\n\n");
    script.push_str("items = [\n");
    for idx in &ordered_indices {
        let Some(placement) = app.placements.get(*idx) else {
            continue;
        };
        let aliases = blender_object_aliases(placement)
            .into_iter()
            .map(|alias| blender_script_string(&alias))
            .collect::<Vec<_>>()
            .join(", ");
        script.push_str("    {\n");
        script.push_str(&format!("        'names': [{aliases}],\n"));
        script.push_str(&format!(
            "        'location': ({:.6}, {:.6}, {:.6}),\n",
            placement.pos.x - base_pos.x,
            placement.pos.y - base_pos.y,
            placement.pos.z - base_pos.z
        ));
        script.push_str(&format!(
            "        'rotation_deg': ({:.6}, {:.6}, {:.6}),\n",
            placement.rot.x - base_rot.x,
            placement.rot.y - base_rot.y,
            placement.rot.z - base_rot.z
        ));
        script.push_str("    },\n");
    }
    script.push_str("]\n\n");
    script.push_str("for item in items:\n");
    script.push_str("    obj = find_object(item['names'], used)\n");
    script.push_str("    if obj is None:\n");
    script.push_str("        missing.append(item['names'][0])\n");
    script.push_str("        continue\n");
    script.push_str("    obj.location = item['location']\n");
    script.push_str("    obj.rotation_euler = Euler(tuple(v * 0.017453292519943295 for v in item['rotation_deg']), 'XYZ')\n");
    script.push_str("    welded += weld_object(obj)\n");
    script.push_str("    used.add(obj.name)\n");
    script.push_str("    placed += 1\n\n");
    script.push_str("print(f'Light Mapper positioned {placed} object(s), welded {welded} vertex/vertices; missing {len(missing)}')\n");
    script.push_str("if missing:\n");
    script.push_str("    print('Missing:', ', '.join(missing))\n");

    match set_system_clipboard(&script) {
        Ok(()) => {
            app.status_message = format!(
                "Copied Blender position script for {} element(s)",
                ordered_indices.len()
            );
        }
        Err(err) => {
            app.status_message = format!("Could not copy Blender position script: {err}");
        }
    }
}

pub(crate) fn open_replace_dff_dialog(app: &mut AppState) {
    if selected_placement(app).is_none() {
        app.status_message = "Select an element before replacing DFF".to_string();
        return;
    }
    if selected_definition_is_readonly(app) {
        app.status_message = "GTA:SA fallback definitions are read-only".to_string();
        return;
    }
    drain_text_input();
    if app.dff_picker_rx.is_some() {
        app.status_message = "DFF file browser is already open".to_string();
        return;
    }
    let start_dir = load_last_dff_export_dir();
    let (tx, rx) = mpsc::channel();
    app.dff_picker_rx = Some(rx);
    app.status_message = "Opening DFF replacement browser...".to_string();
    thread::spawn(move || {
        let _ = tx.send((DffPickerKind::Replace, choose_replace_dff_path(start_dir)));
    });
}

pub(crate) fn open_import_prelight_dialog(app: &mut AppState) {
    if selected_live_indices(app).is_empty() {
        app.status_message = "Select an element before importing DFF prelight".to_string();
        return;
    }
    drain_text_input();
    if app.dff_picker_rx.is_some() {
        app.status_message = "DFF file browser is already open".to_string();
        return;
    }
    let start_dir = load_last_dff_export_dir();
    let (tx, rx) = mpsc::channel();
    app.dff_picker_rx = Some(rx);
    app.status_message = "Opening prelight import browser...".to_string();
    thread::spawn(move || {
        let _ = tx.send((
            DffPickerKind::ImportPrelight,
            choose_import_prelight_path(start_dir),
        ));
    });
}

pub(crate) fn open_replace_col_dialog(app: &mut AppState) {
    if selected_placement(app).is_none() {
        app.status_message = "Select an element before replacing COL".to_string();
        return;
    }
    if selected_definition_is_readonly(app) {
        app.status_message = "GTA:SA fallback definitions are read-only".to_string();
        return;
    }
    drain_text_input();
    if app.dff_picker_rx.is_some() {
        app.status_message = "File browser is already open".to_string();
        return;
    }
    let start_dir = load_last_dff_export_dir();
    let (tx, rx) = mpsc::channel();
    app.dff_picker_rx = Some(rx);
    app.status_message = "Opening COL replacement browser...".to_string();
    thread::spawn(move || {
        let _ = tx.send((
            DffPickerKind::ReplaceCol,
            choose_replace_col_path(start_dir),
        ));
    });
}

pub(crate) fn selected_definition_id_and_txd(app: &AppState) -> Option<(String, String)> {
    let placement = selected_placement(app)?;
    let txd = definition_txd_name(&app.definitions, &placement.id)?;
    Some((placement.id.clone(), asset_key(txd, ".txd")))
}

pub(crate) fn open_texture_image_picker(app: &mut AppState, kind: DffPickerKind) {
    drain_text_input();
    if app.dff_picker_rx.is_some() {
        app.status_message = "File browser is already open".to_string();
        return;
    }
    let start_dir = load_last_dff_export_dir();
    let (tx, rx) = mpsc::channel();
    app.dff_picker_rx = Some(rx);
    app.status_message = "Opening texture image browser...".to_string();
    thread::spawn(move || {
        let _ = tx.send((kind, choose_texture_image_path(start_dir)));
    });
}

pub(crate) fn add_texture_to_archive(
    app: &mut AppState,
    definition_id: String,
    txd_name: String,
    path: PathBuf,
) {
    let texture_name = texture_name_from_path(&path);
    replace_texture_in_archive(app, definition_id, txd_name, texture_name, path);
}

pub(crate) fn replace_texture_in_archive(
    app: &mut AppState,
    definition_id: String,
    txd_name: String,
    texture_name: String,
    path: PathBuf,
) {
    if !path.is_file() {
        app.status_message = format!(
            "Texture image is not a file: {}",
            ellipsize(path.to_string_lossy().as_ref(), 54)
        );
        return;
    }
    let texture_name = sanitize_texture_name(&texture_name);
    let native = match imported_image_texture_native(&path, &texture_name) {
        Ok(native) => native,
        Err(err) => {
            app.status_message = format!("Could not import texture: {err}");
            return;
        }
    };
    match stage_texture_native_into_txd(app, &txd_name, &texture_name, &native) {
        Ok(recompiled) => {
            app.status_message = format!(
                "Stored {} in {} for {}; recompiled {} definition mesh(es).",
                texture_name, txd_name, definition_id, recompiled
            );
        }
        Err(err) => app.status_message = format!("Could not update {txd_name}: {err}"),
    }
}

pub(crate) fn start_replace_asset_choice(
    app: &mut AppState,
    path: PathBuf,
    kind: ReplacementAssetKind,
) {
    if selected_placement(app).is_none() {
        app.status_message = match kind {
            ReplacementAssetKind::Dff => "Select an element before replacing DFF".to_string(),
            ReplacementAssetKind::Col => "Select an element before replacing COL".to_string(),
        };
        return;
    }
    if !path.is_file() {
        let label = match kind {
            ReplacementAssetKind::Dff => "DFF",
            ReplacementAssetKind::Col => "COL",
        };
        app.status_message = format!(
            "Replacement {label} is not a file: {}",
            ellipsize(path.to_string_lossy().as_ref(), 54)
        );
        return;
    }
    let (ext, label) = match kind {
        ReplacementAssetKind::Dff => ("dff", "DFF"),
        ReplacementAssetKind::Col => ("col", "COL"),
    };
    if !path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case(ext))
    {
        app.status_message = format!("Replacement file must end in .{ext}");
        return;
    }
    if let Some(parent) = path.parent() {
        save_last_dff_export_dir(parent);
    }
    app.dff_replace_choice_dialog = Some(DffReplaceChoiceDialog { path, kind });
    app.status_message = format!("Choose how to apply replacement {label}");
}

pub(crate) fn unique_definition_id(app: &AppState, base: &str) -> String {
    let mut candidate = format!("{base}_unique");
    let mut suffix = 2usize;
    while app.definitions.contains_key(&candidate)
        || app
            .placements
            .iter()
            .any(|placement| placement.id == candidate)
    {
        candidate = format!("{base}_unique{suffix}");
        suffix += 1;
    }
    candidate
}

pub(crate) fn selected_replacement_index(app: &AppState) -> Option<usize> {
    let live = selected_live_indices(app);
    if live.contains(&app.selected) {
        Some(app.selected)
    } else {
        live.first().copied()
    }
}

pub(crate) fn replace_selected_dff(app: &mut AppState, path: PathBuf, make_unique: bool) {
    let Some(selected_idx) = selected_replacement_index(app) else {
        app.status_message = "Select an element before replacing DFF".to_string();
        return;
    };
    let Ok(bytes) = fs::read(&path) else {
        app.status_message = format!(
            "Failed to read replacement DFF: {}",
            ellipsize(path.to_string_lossy().as_ref(), 54)
        );
        return;
    };
    let raw = parse_dff_mesh(&bytes);
    if raw.vertices.is_empty() || raw.triangles.is_empty() {
        app.status_message = "Replacement DFF did not contain readable mesh geometry".to_string();
        return;
    }
    let old_id = app.placements[selected_idx].id.clone();
    let dff_ref = app
        .definitions
        .get(&old_id)
        .and_then(|def| def.attrs.get("dff"))
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| app.placements[selected_idx].dff.clone());
    let dff_ref = normalize_legacy_light_mapper_asset_stem(&dff_ref);
    let dff_name = with_ext(&dff_ref, ".dff");
    let asset_root = replacement_asset_root(app);
    // Replacement is a file transfer, not an edit.  DFFs can contain hierarchy,
    // plugin, and BinMesh data that the normalized writer cannot reproduce
    // byte-for-byte (including the original face order), so retain the imported
    // payload exactly as supplied.
    if let Err(err) = upsert_replacement_dff(&asset_root, &dff_name, &bytes) {
        app.status_message = err;
        return;
    }
    let txd_scope = app
        .placements
        .get(selected_idx)
        .and_then(|placement| definition_txd_name(&app.definitions, &placement.id))
        .map(ToOwned::to_owned);
    let mesh_key = mesh_key_from_dff_txd(&dff_name, txd_scope.as_deref());
    let texture_files = collect_texture_files(&app.root);
    let ambient_lift = scene_ambient_lift_from_timecyc(&app.timecyc);
    let Some(mesh) = compile_render_mesh(
        raw,
        txd_scope.as_deref(),
        None,
        None,
        &texture_files,
        &app.txd_textures,
        &mut app.textures,
        &mut app.textured_parts,
        app.options.textures,
        ambient_lift,
    ) else {
        app.status_message = "Replacement DFF did not compile into a render mesh".to_string();
        return;
    };

    replace_render_mesh(&mut app.meshes, mesh_key, mesh);
    if make_unique {
        let new_id = unique_definition_id(app, &old_id);
        let mut def = app
            .definitions
            .get(&old_id)
            .cloned()
            .unwrap_or_else(|| Definition {
                id: old_id.clone(),
                zone: app.placements[selected_idx].zone.clone(),
                attrs: BTreeMap::new(),
            });
        def.id = new_id.clone();
        def.attrs.insert("id".to_string(), new_id.clone());
        def.attrs.insert("dff".to_string(), dff_ref.clone());
        app.definitions.insert(new_id.clone(), def);
        if let Some(placement) = app.placements.get_mut(selected_idx) {
            placement.id = new_id;
            placement.dff = dff_ref.clone();
            sync_placement_attrs(placement);
        }
        invalidate_outliner_label(app, selected_idx);
    } else {
        if let Some(def) = app.definitions.get_mut(&old_id) {
            def.attrs.insert("dff".to_string(), dff_ref.clone());
        }
        for idx in 0..app.placements.len() {
            if app.placements[idx].id == old_id {
                app.placements[idx].dff = dff_ref.clone();
                invalidate_outliner_label(app, idx);
            }
        }
    }
    rebuild_render_cells(app);
    clear_history_for_external_change(app);
    app.status_message = format!(
        "Replaced DFF with {} ({}) and staged {}; undo history cleared",
        dff_name,
        if make_unique { "unique" } else { "instance" },
        asset_root.join("imgs").join(REPLACEMENT_IMG).display()
    );
}

pub(crate) fn replacement_img_entry(root: &Path, asset_name: &str) -> Option<ImgEntry> {
    let target = lower(asset_name);
    let img_path = root.join("imgs").join(REPLACEMENT_IMG);
    parse_img(&img_path)
        .into_iter()
        .find(|entry| lower(&entry.name) == target)
}

pub(crate) fn replace_selected_col(app: &mut AppState, path: PathBuf, make_unique: bool) {
    let Some(selected_idx) = selected_replacement_index(app) else {
        app.status_message = "Select an element before replacing COL".to_string();
        return;
    };
    let Ok(bytes) = fs::read(&path) else {
        app.status_message = format!(
            "Failed to read replacement COL: {}",
            ellipsize(path.to_string_lossy().as_ref(), 54)
        );
        return;
    };
    let col_len = col_chunk_len(&bytes);
    let col_bytes = &bytes[..col_len.min(bytes.len())];
    let loose_entry = ImgEntry {
        img_path: path.clone(),
        name: path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("replacement.col")
            .to_string(),
        offset: 0,
        size: col_bytes.len().div_ceil(2048) as u32,
    };
    if parse_col_mesh(col_bytes, &loose_entry).is_none() {
        app.status_message =
            "Replacement COL did not contain readable collision geometry".to_string();
        return;
    }

    let old_id = app.placements[selected_idx].id.clone();
    let old_col = app
        .definitions
        .get(&old_id)
        .and_then(|def| def.attrs.get("col"))
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| app.placements[selected_idx].dff.clone());
    let old_col = normalize_legacy_light_mapper_asset_stem(&old_col);
    let col_name = if make_unique {
        img_safe_col_name(app, selected_idx, true)
    } else {
        with_ext(&old_col, ".col")
    };
    let col_attr = Path::new(&col_name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(&col_name)
        .to_string();
    // Bind the collision to the target model by rewriting each COL model's internal
    // name to match the staged entry stem (mirrors how a replacement DFF's frame name
    // is normalized to the target). Without this the engine registers the collision
    // under the source file's stale name and it never attaches to the object.
    let mut staged_col = col_bytes.to_vec();
    set_col_model_names_from_entry(&mut staged_col, &col_attr);
    let asset_root = replacement_asset_root(app);
    if let Err(err) = upsert_replacement_col(&asset_root, &col_name, &staged_col) {
        app.status_message = err;
        return;
    }
    let Some(entry) = replacement_img_entry(&asset_root, &col_name) else {
        app.status_message = "Replacement COL was written but could not be re-indexed".to_string();
        return;
    };
    let Some(mesh) = parse_col_mesh(&read_img_entry(&entry), &entry) else {
        app.status_message =
            "Replacement COL was written but did not parse from the staged IMG".to_string();
        return;
    };
    let normalized_col_name = normalize_legacy_light_mapper_asset_name(&col_name);
    app.pending_replacement_assets.insert(
        lower(&normalized_col_name),
        (normalized_col_name.clone(), staged_col.clone()),
    );

    let mesh_key = lower(with_ext(&col_name, ".col"));
    invalidate_collision_render_cache(app, &mesh_key);
    app.collisions.insert(mesh_key, mesh);
    app.selected_col_face = None;
    if make_unique {
        let new_id = unique_definition_id(app, &old_id);
        let mut def = app
            .definitions
            .get(&old_id)
            .cloned()
            .unwrap_or_else(|| Definition {
                id: old_id.clone(),
                zone: app.placements[selected_idx].zone.clone(),
                attrs: BTreeMap::new(),
            });
        def.id = new_id.clone();
        def.attrs.insert("id".to_string(), new_id.clone());
        def.attrs.insert("col".to_string(), col_attr.clone());
        app.definitions.insert(new_id.clone(), def);
        if let Some(placement) = app.placements.get_mut(selected_idx) {
            placement.id = new_id;
            sync_placement_attrs(placement);
        }
        invalidate_outliner_label(app, selected_idx);
    } else {
        if !app.definitions.contains_key(&old_id) {
            app.definitions.insert(
                old_id.clone(),
                Definition {
                    id: old_id.clone(),
                    zone: app.placements[selected_idx].zone.clone(),
                    attrs: BTreeMap::new(),
                },
            );
        }
        let mut updated = 0usize;
        let mut affected_ids = HashSet::new();
        for def in app.definitions.values_mut() {
            let def_col = def
                .attrs
                .get("col")
                .filter(|value| !value.trim().is_empty())
                .cloned()
                .unwrap_or_else(|| {
                    def.attrs
                        .get("dff")
                        .cloned()
                        .unwrap_or_else(|| def.id.clone())
                });
            if asset_key(&def_col, ".col") == asset_key(&old_col, ".col") || def.id == old_id {
                def.attrs.insert("col".to_string(), col_attr.clone());
                affected_ids.insert(def.id.clone());
                updated += 1;
            }
        }
        if updated == 0 {
            app.status_message =
                "No matching definition was available for COL replacement".to_string();
            return;
        }
        for idx in 0..app.placements.len() {
            if affected_ids.contains(&app.placements[idx].id) {
                invalidate_outliner_label(app, idx);
            }
        }
    }
    invalidate_validation_cache(app);
    clear_history_for_external_change(app);
    app.status_message = format!(
        "Replaced COL with {} ({}) and staged {}; undo history cleared",
        col_name,
        if make_unique { "unique" } else { "instance" },
        asset_root.join("imgs").join(REPLACEMENT_IMG).display()
    );
}

pub(crate) fn camera_vectors(camera: &CameraState) -> (Vec3, Vec3) {
    let forward = vec3(
        camera.yaw.sin() * camera.pitch.cos(),
        camera.yaw.cos() * camera.pitch.cos(),
        camera.pitch.sin(),
    )
    .normalize();
    let right = vec3(camera.yaw.cos(), -camera.yaw.sin(), 0.0);
    (forward, right)
}

pub(crate) fn snap_to(app: &mut AppState, index: usize) {
    if index >= app.placements.len() {
        return;
    }
    select_element(app, index);
    let p = &app.placements[index];
    let pos = to_mq(p.pos);
    let (forward, _) = camera_vectors(&app.camera);
    app.camera.pos = pos - forward * 120.0 + vec3(0.0, 0.0, 65.0);
    app.camera.pitch = -24.0_f32.to_radians();
}

pub(crate) fn is_live_element(app: &AppState, index: usize) -> bool {
    index < app.placements.len()
        && !app
            .element_states
            .get(index)
            .is_some_and(|state| state.deleted)
}

pub(crate) fn is_visible_element(app: &AppState, index: usize) -> bool {
    is_live_element(app, index)
        && !app
            .element_states
            .get(index)
            .is_some_and(|state| state.hidden)
}

pub(crate) fn prune_selected_elements(app: &mut AppState) {
    let len = app.placements.len();
    app.selected_elements.retain(|idx| *idx < len);
    app.selected_element_order
        .retain(|idx| *idx < len && app.selected_elements.contains(idx));
    if app.selected < len {
        app.selected_elements.insert(app.selected);
        if !app.selected_element_order.contains(&app.selected) {
            app.selected_element_order.push(app.selected);
        }
    } else if let Some(idx) = app
        .selected_element_order
        .iter()
        .rev()
        .copied()
        .find(|idx| app.selected_elements.contains(idx))
    {
        app.selected = idx;
    } else {
        app.selected = NO_SELECTION;
    }
}

pub(crate) fn selected_indices(app: &AppState) -> Vec<usize> {
    let mut indices: Vec<usize> = app
        .selected_elements
        .iter()
        .copied()
        .filter(|idx| *idx < app.placements.len())
        .collect();
    if indices.is_empty() && app.selected < app.placements.len() {
        indices.push(app.selected);
    }
    indices
}

pub(crate) fn selected_live_indices(app: &AppState) -> Vec<usize> {
    selected_indices(app)
        .into_iter()
        .filter(|idx| is_live_element(app, *idx))
        .collect()
}

pub(crate) fn selected_live_indices_in_selection_order(app: &AppState) -> Vec<usize> {
    let mut ordered = Vec::new();
    let mut seen = BTreeSet::new();
    for idx in app.selected_element_order.iter().copied() {
        if app.selected_elements.contains(&idx)
            && is_live_element(app, idx)
            && !is_default_world_placement(&app.placements[idx])
            && seen.insert(idx)
        {
            ordered.push(idx);
        }
    }
    for idx in selected_editable_indices(app) {
        if seen.insert(idx) {
            ordered.push(idx);
        }
    }
    ordered
}

/// Returns true only when every live selected element uses the special `self`
/// LOD marker. The marker is an MTA value, not the id of another placement.
pub(crate) fn selected_live_elements_are_self_lod(app: &AppState) -> bool {
    let selected = selected_live_indices(app);
    !selected.is_empty()
        && selected.into_iter().all(|idx| {
            app.placements
                .get(idx)
                .and_then(|placement| placement.attrs.get("lodParent"))
                .is_some_and(|parent| parent.trim().eq_ignore_ascii_case("self"))
        })
}

/// Toggle the special `lodParent="self"` value across the live selection.
/// If the full selection is already self-LOD, remove the value from every
/// selected element; otherwise assign it to every selected element.
pub(crate) fn toggle_self_lod_for_selection(app: &mut AppState) {
    let selected = selected_editable_indices(app);
    if selected.is_empty() {
        app.status_message = "Select at least one live element".to_string();
        return;
    }
    let enable = !selected_live_elements_are_self_lod(app);
    let before = local_world_history_snapshot(app, selected.iter().copied(), []);
    for idx in &selected {
        if let Some(placement) = app.placements.get_mut(*idx) {
            set_optional_attr(
                &mut placement.attrs,
                "lodParent",
                if enable { "self" } else { "" }.to_string(),
            );
        }
    }
    commit_local_world_history(
        app,
        if enable {
            "Assign Self LOD"
        } else {
            "Clear Self LOD"
        },
        before,
    );
    app.status_message = if enable {
        format!("Assigned self LOD to {} element(s)", selected.len())
    } else {
        format!("Cleared self LOD from {} element(s)", selected.len())
    };
}

pub(crate) fn has_selection(app: &AppState) -> bool {
    !selected_indices(app).is_empty()
}

pub(crate) fn has_active_selection(app: &AppState) -> bool {
    if app.active_tab == AppTab::Lights {
        app.selected_light < app.lights.len()
    } else {
        has_selection(app)
    }
}

pub(crate) fn active_selection_deleted(app: &AppState) -> bool {
    app.active_tab != AppTab::Lights && primary_selected_deleted(app)
}

pub(crate) fn primary_selected_deleted(app: &AppState) -> bool {
    app.element_states
        .get(app.selected)
        .is_some_and(|state| state.deleted)
}

pub(crate) fn selection_origin(app: &AppState) -> Option<Vec3> {
    let indices = selected_live_indices(app);
    if indices.is_empty() {
        return None;
    }
    let sum = indices
        .iter()
        .filter_map(|idx| app.placements.get(*idx))
        .fold(Vec3::ZERO, |acc, placement| acc + to_mq(placement.pos));
    Some(sum / indices.len() as f32)
}

pub(crate) fn selected_lod_parent_index(app: &AppState) -> Option<usize> {
    let selected = app.placements.get(app.selected)?;
    let lod_parent = selected.attrs.get("lodParent")?.trim();
    if lod_parent.is_empty() || lod_parent.eq_ignore_ascii_case("self") {
        return None;
    }

    // All placements sharing this LOD id (case-insensitive).
    let candidates: Vec<usize> = app
        .placements
        .iter()
        .enumerate()
        .filter(|(_, placement)| placement.id.eq_ignore_ascii_case(lod_parent))
        .map(|(idx, _)| idx)
        .collect();
    match candidates.as_slice() {
        [] => return None,
        [only] => return Some(*only),
        _ => {}
    }

    // Repeated LOD: prefer the instance whose uniqueID matches the detail's, so
    // selection follows the LOD index rather than always landing on the first
    // model of that name.
    if let Some(unique_id) = selected
        .attrs
        .get("uniqueID")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        if let Some(idx) = candidates.iter().copied().find(|&idx| {
            app.placements[idx]
                .attrs
                .get("uniqueID")
                .map(|value| value.trim())
                .is_some_and(|value| value.eq_ignore_ascii_case(unique_id))
        }) {
            return Some(idx);
        }
    }

    // Fall back to the spatially closest instance when no uniqueID match exists.
    let detail_pos = selected.pos;
    candidates.into_iter().min_by(|&a, &b| {
        v3_dist_sq(app.placements[a].pos, detail_pos)
            .total_cmp(&v3_dist_sq(app.placements[b].pos, detail_pos))
    })
}

pub(crate) fn select_lod_parent_for_selected(app: &mut AppState) {
    let Some(selected) = app.placements.get(app.selected) else {
        app.status_message = "Select an element with an LOD parent".to_string();
        return;
    };
    let lod_parent = selected
        .attrs
        .get("lodParent")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());
    let Some(lod_parent) = lod_parent else {
        app.status_message = "Selected element has no LOD parent".to_string();
        return;
    };
    if lod_parent.eq_ignore_ascii_case("self") {
        app.status_message = "Selected element uses self LOD".to_string();
        return;
    }
    let lod_parent = lod_parent.to_string();
    let Some(idx) = selected_lod_parent_index(app) else {
        app.status_message = format!("LOD parent {lod_parent} not found");
        return;
    };
    select_element(app, idx);
    app.status_message = format!("Selected LOD parent {lod_parent}");
}

pub(crate) fn assign_lod_parent_from_selection(app: &mut AppState) {
    let ordered = selected_live_indices_in_selection_order(app);
    if ordered.len() < 2 {
        app.status_message = "Select the LOD parent first, then at least one child".to_string();
        return;
    }
    let parent_idx = ordered[0];
    let Some(parent_id) = app
        .placements
        .get(parent_idx)
        .map(|placement| placement.id.clone())
    else {
        app.status_message = "LOD parent selection is invalid".to_string();
        return;
    };
    if parent_id.trim().is_empty() {
        app.status_message = "LOD parent needs an ID".to_string();
        return;
    }
    let before = local_world_history_snapshot(app, selected_editable_indices(app), []);

    // When several LODs share this name, `lodParent` alone can't say which LOD
    // instance a child belongs to. Pair them with a shared, scene-unique
    // `uniqueID` on both the LOD and the source elements. Reuse the LOD's
    // existing id if it already has one so repeated assigns stay stable.
    let shared_unique_id = if lod_name_is_repeated(&app.placements, &parent_id) {
        let uid = app.placements[parent_idx]
            .attrs
            .get("uniqueID")
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| next_scene_unique_id(&app.placements).to_string());
        set_optional_attr(
            &mut app.placements[parent_idx].attrs,
            "uniqueID",
            uid.clone(),
        );
        Some(uid)
    } else {
        None
    };

    let mut assigned = 0usize;
    for child_idx in ordered.into_iter().skip(1) {
        if child_idx == parent_idx {
            continue;
        }
        if let Some(child) = app.placements.get_mut(child_idx) {
            set_optional_attr(&mut child.attrs, "lodParent", parent_id.clone());
            if let Some(uid) = &shared_unique_id {
                set_optional_attr(&mut child.attrs, "uniqueID", uid.clone());
            }
            assigned += 1;
        }
    }
    if assigned == 0 {
        app.status_message = "Select at least one child after the LOD parent".to_string();
        return;
    }
    commit_local_world_history(app, "Assign LOD", before);
    app.status_message = format!("Assigned {parent_id} as LOD parent for {assigned} element(s)");
}

fn v3_dist_sq(a: V3, b: V3) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    dx * dx + dy * dy + dz * dz
}

/// One past the largest integer `uniqueID` currently in the scene, or 1 if none.
pub(crate) fn next_scene_unique_id(placements: &[Placement]) -> u64 {
    let used = placements
        .iter()
        .filter_map(|p| p.attrs.get("uniqueID"))
        .filter_map(|v| v.trim().parse::<u64>().ok())
        .collect::<BTreeSet<_>>();
    used.iter()
        .next_back()
        .and_then(|maximum| maximum.checked_add(1))
        .or_else(|| (1..).find(|candidate| !used.contains(candidate)))
        .unwrap_or(0)
}

/// Number of placements sharing an id (case-insensitive); >1 means the LOD name
/// is ambiguous and needs a `uniqueID` to disambiguate pairings.
fn lod_name_is_repeated(placements: &[Placement], id: &str) -> bool {
    placements
        .iter()
        .filter(|p| p.id.eq_ignore_ascii_case(id))
        .count()
        > 1
}

const LOW_LOD_EFFECTIVE_DISTANCE: u16 = 1500;
const MINIMUM_LOW_LOD_EFFECTIVE_DISTANCE: u16 = 300;
const OBJECT_LOW_LOD_DISTANCE_MULTIPLIER: u16 = 5;
const OBJECT_DETAIL_WITH_LOD_DISTANCE: u16 = 200;
const DETAIL_MINIMUM_LOD_DISTANCE: u16 = 80;
const DETAIL_MAXIMUM_LOD_DISTANCE: u16 = 299;
const DETAIL_BASE_LOD_DISTANCE: f32 = 100.0;
const LOD_REFERENCE_VIEWPORT_HEIGHT: f32 = 1080.0;
const LOD_REFERENCE_VERTICAL_FOV_DEGREES: f32 = 70.0;
const LOD_CULL_TARGET_PIXELS: f32 = 48.0;
// Used when a building's mesh bounds are unavailable (including during LOD
// generation, before the generated definition is installed into the scene).
const BUILDING_DETAIL_WITH_LOD_DISTANCE: u16 = DETAIL_MAXIMUM_LOD_DISTANCE;

pub(crate) fn generated_lod_distance(is_building: bool) -> u16 {
    if is_building {
        LOW_LOD_EFFECTIVE_DISTANCE
    } else {
        LOW_LOD_EFFECTIVE_DISTANCE.div_ceil(OBJECT_LOW_LOD_DISTANCE_MULTIPLIER)
    }
}

pub(crate) fn detail_with_lod_distance(is_building: bool) -> u16 {
    if is_building {
        BUILDING_DETAIL_WITH_LOD_DISTANCE
    } else {
        OBJECT_DETAIL_WITH_LOD_DISTANCE
    }
}

pub(crate) fn sa_detail_lod_distance(bounds: Bounds) -> u16 {
    let size = bounds.max - bounds.min;
    let radius = size.length() * 0.5;
    let distance = (DETAIL_BASE_LOD_DISTANCE + radius).ceil();
    if !distance.is_finite() || distance <= 0.0 {
        return OBJECT_DETAIL_WITH_LOD_DISTANCE;
    }
    distance.clamp(
        f32::from(DETAIL_MINIMUM_LOD_DISTANCE),
        f32::from(DETAIL_MAXIMUM_LOD_DISTANCE),
    ) as u16
}

fn sa_effective_low_lod_distance(bounds: Bounds) -> u16 {
    let size = bounds.max - bounds.min;
    let max_dimension = size.x.abs().max(size.y.abs()).max(size.z.abs());
    let half_fov = (LOD_REFERENCE_VERTICAL_FOV_DEGREES * 0.5).to_radians();
    let distance = (max_dimension * LOD_REFERENCE_VIEWPORT_HEIGHT
        / (2.0 * half_fov.tan() * LOD_CULL_TARGET_PIXELS))
        .ceil();
    if !distance.is_finite() || distance <= 0.0 {
        return LOW_LOD_EFFECTIVE_DISTANCE;
    }
    distance.clamp(
        f32::from(MINIMUM_LOW_LOD_EFFECTIVE_DISTANCE),
        f32::from(LOW_LOD_EFFECTIVE_DISTANCE),
    ) as u16
}

pub(crate) fn sa_lod_model_distance(is_building: bool, bounds: Bounds) -> u16 {
    let effective_distance = sa_effective_low_lod_distance(bounds);
    if is_building {
        effective_distance
    } else {
        effective_distance.div_ceil(OBJECT_LOW_LOD_DISTANCE_MULTIPLIER)
    }
}

/// Calculate the definition distance used by Repair LODs for a model's scene
/// role. LOD generation also calls this helper so newly-created pairs use the
/// exact same rules without requiring a later repair pass.
pub(crate) fn repaired_lod_distance(
    is_lod: bool,
    is_building: bool,
    has_lod: bool,
    mesh_bounds: Option<Bounds>,
) -> u16 {
    if is_lod {
        mesh_bounds
            .map(|bounds| sa_lod_model_distance(is_building, bounds))
            .unwrap_or_else(|| generated_lod_distance(is_building))
    } else if let Some(bounds) = mesh_bounds {
        sa_detail_lod_distance(bounds)
    } else if has_lod {
        detail_with_lod_distance(is_building)
    } else {
        170
    }
}

fn lod_distance_matches(value: Option<&str>, expected: u16) -> bool {
    value
        .and_then(|value| value.trim().parse::<f32>().ok())
        .is_some_and(|value| value.is_finite() && value == f32::from(expected))
}

/// Recalculate definition draw distances using the scene role and dimensions
/// of each model.
///
/// Stock SA detail distances correlate most strongly with a 100-unit base plus
/// the model bounds radius, capped at 299. Low-LOD distance uses a 1080p/70-degree
/// reference projection and remains loaded until its largest dimension occupies
/// about 48 vertical pixels, with an effective range clamped to 300..1500.
/// Building LODs store that actual distance; object LODs store one fifth because
/// MTA applies its 5-times low-LOD rule. Missing bounds retain the maximum-range
/// fallback. `lodParent` does not make the detail model itself an LOD.
/// Definitions already set to their calculated distance are left unchanged.
/// Returns `(lod_definitions, detail_definitions)` updated.
pub(crate) fn fix_lod_distances(app: &mut AppState) -> (usize, usize) {
    use std::collections::{HashMap, HashSet};

    let placement_ids: HashSet<String> = app
        .placements
        .iter()
        .map(|placement| placement.id.to_ascii_lowercase())
        .collect();

    // Draw distance belongs to the definition, so collapse placement roles by
    // definition id. If a definition is used in more than one role, retain the
    // longest required distance.
    let mut targets: HashMap<String, (String, bool, u16)> = HashMap::new();
    for placement in &app.placements {
        let is_lod = placement_is_lod(placement, &app.lod_ids);
        let has_lod = placement
            .attrs
            .get("lodParent")
            .map(|parent| parent.trim())
            .filter(|parent| !parent.is_empty())
            .is_some_and(|parent| {
                parent.eq_ignore_ascii_case("self")
                    || placement_ids.contains(&parent.to_ascii_lowercase())
            });
        let mesh_bounds = element_mesh(app, placement).map(|mesh| mesh.bounds);
        let distance = repaired_lod_distance(
            is_lod,
            placement.tag.eq_ignore_ascii_case("building"),
            has_lod,
            mesh_bounds,
        );
        targets
            .entry(placement.id.clone())
            .and_modify(|(_, target_is_lod, target_distance)| {
                *target_is_lod |= is_lod;
                *target_distance = (*target_distance).max(distance);
            })
            .or_insert_with(|| (placement.zone.clone(), is_lod, distance));
    }

    let mut lod_definitions = 0usize;
    let mut detail_definitions = 0usize;
    for (id, (zone, is_lod, distance)) in targets {
        let already_correct = app.definitions.get(&id).is_some_and(|definition| {
            lod_distance_matches(
                definition.attrs.get("lodDistance").map(String::as_str),
                distance,
            )
        });
        if already_correct {
            continue;
        }

        // Built-in GTA definitions are read-only until an override is made.
        // Converting here ensures the new default is actually persisted.
        if app.readonly_definition_ids.contains(&id) {
            make_definition_override_writable(app, &id, zone);
        }
        let Some(definition) = app.definitions.get_mut(&id) else {
            continue;
        };
        definition
            .attrs
            .insert("lodDistance".to_string(), distance.to_string());
        mark_definition_override_attr(definition, "lodDistance");
        if is_lod {
            lod_definitions += 1;
        } else {
            detail_definitions += 1;
        }
    }

    (lod_definitions, detail_definitions)
}

fn resolve_lod_child_target(
    placements: &[Placement],
    child_index: usize,
    candidates: &[usize],
) -> Option<usize> {
    match candidates {
        [] => return None,
        [only] => return Some(*only),
        _ => {}
    }

    let child = placements.get(child_index)?;
    if let Some(unique_id) = child
        .attrs
        .get("uniqueID")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        if let Some(index) = candidates.iter().copied().find(|index| {
            placements
                .get(*index)
                .and_then(|placement| placement.attrs.get("uniqueID"))
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
                .is_some_and(|candidate| candidate.eq_ignore_ascii_case(unique_id))
        }) {
            return Some(index);
        }
    }

    candidates.iter().copied().min_by(|a, b| {
        v3_dist_sq(placements[*a].pos, child.pos)
            .total_cmp(&v3_dist_sq(placements[*b].pos, child.pos))
    })
}

/// Make each live LOD placement use the same MTA world element type as the
/// live detail placement it represents. Repeated LOD ids use the same
/// uniqueID/nearest-instance resolution as the orphan repair pass.
///
/// A single LOD placement can be shared by several detail placements. When
/// those details disagree between `building` and `object`, there is no type
/// that can match them all, so the ambiguous LOD is left unchanged.
fn fix_lod_types_in(placements: &mut [Placement], states: &[ElementState]) -> usize {
    use std::collections::{HashMap, HashSet};

    let mut target_candidates = HashMap::<String, Vec<usize>>::new();
    for (index, placement) in placements.iter().enumerate() {
        if live_in_snapshot(states, index) {
            target_candidates
                .entry(placement.id.trim().to_ascii_lowercase())
                .or_default()
                .push(index);
        }
    }

    let mut target_types = HashMap::<usize, &'static str>::new();
    let mut conflicts = HashSet::<usize>::new();
    for (child_index, child) in placements.iter().enumerate() {
        if !live_in_snapshot(states, child_index) {
            continue;
        }
        let desired_type = if child.tag.eq_ignore_ascii_case("building") {
            "building"
        } else if child.tag.eq_ignore_ascii_case("object") {
            "object"
        } else {
            continue;
        };
        let Some(parent) = child
            .attrs
            .get("lodParent")
            .map(|value| value.trim())
            .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("self"))
        else {
            continue;
        };
        let Some(candidates) = target_candidates.get(&parent.to_ascii_lowercase()) else {
            continue;
        };
        let Some(target) = resolve_lod_child_target(placements, child_index, candidates) else {
            continue;
        };
        match target_types.get(&target) {
            Some(existing) if *existing != desired_type => {
                conflicts.insert(target);
            }
            None => {
                target_types.insert(target, desired_type);
            }
            _ => {}
        }
    }

    let mut changed = 0usize;
    for (index, desired_type) in target_types {
        if conflicts.contains(&index) {
            continue;
        }
        let Some(lod) = placements.get_mut(index) else {
            continue;
        };
        if is_default_world_placement(lod) {
            continue;
        }
        if !lod.tag.eq_ignore_ascii_case(desired_type) {
            lod.tag = desired_type.to_string();
            changed += 1;
        }
    }
    changed
}

/// Repair LOD placement types. The caller owns history and render/outliner
/// refreshes. Returns the number of LOD placements changed.
pub(crate) fn fix_lod_types(app: &mut AppState) -> usize {
    fix_lod_types_in(&mut app.placements, &app.element_states)
}

#[cfg(test)]
mod lod_type_tests {
    use super::*;

    fn placement(
        id: &str,
        tag: &str,
        lod_parent: Option<&str>,
        unique_id: Option<&str>,
        x: f32,
    ) -> Placement {
        let mut attrs = BTreeMap::new();
        if let Some(lod_parent) = lod_parent {
            attrs.insert("lodParent".to_string(), lod_parent.to_string());
        }
        if let Some(unique_id) = unique_id {
            attrs.insert("uniqueID".to_string(), unique_id.to_string());
        }
        Placement {
            id: id.to_string(),
            dff: id.to_string(),
            zone: "test".to_string(),
            tag: tag.to_string(),
            attrs,
            pos: V3 { x, y: 0.0, z: 0.0 },
            rot: V3::default(),
        }
    }

    #[test]
    fn matches_lod_types_to_building_and_object_bases() {
        let mut placements = vec![
            placement("tower", "building", Some("lod_tower"), None, 0.0),
            placement("lod_tower", "object", None, None, 0.0),
            placement("prop", "object", Some("lod_prop"), None, 100.0),
            placement("lod_prop", "building", None, None, 100.0),
        ];
        let states = vec![ElementState::default(); placements.len()];

        assert_eq!(fix_lod_types_in(&mut placements, &states), 2);
        assert_eq!(placements[1].tag, "building");
        assert_eq!(placements[3].tag, "object");
    }

    #[test]
    fn repeated_lods_use_unique_ids_when_matching_types() {
        let mut placements = vec![
            placement("tower", "building", Some("shared_lod"), Some("1"), 0.0),
            placement("prop", "object", Some("shared_lod"), Some("2"), 100.0),
            placement("shared_lod", "object", None, Some("1"), 100.0),
            placement("shared_lod", "building", None, Some("2"), 0.0),
        ];
        let states = vec![ElementState::default(); placements.len()];

        assert_eq!(fix_lod_types_in(&mut placements, &states), 2);
        assert_eq!(placements[2].tag, "building");
        assert_eq!(placements[3].tag, "object");
    }

    #[test]
    fn shared_lod_with_conflicting_base_types_is_left_unchanged() {
        let mut placements = vec![
            placement("tower", "building", Some("shared_lod"), None, 0.0),
            placement("prop", "object", Some("shared_lod"), None, 10.0),
            placement("shared_lod", "object", None, None, 5.0),
        ];
        let states = vec![ElementState::default(); placements.len()];

        assert_eq!(fix_lod_types_in(&mut placements, &states), 0);
        assert_eq!(placements[2].tag, "object");
    }
}

/// Find live LOD placements that no live detail placement resolves to.
///
/// Repeated LOD names are resolved the same way as the LOD Audit: an exact
/// `uniqueID` match wins, otherwise the closest candidate is treated as the
/// child's target. Name-prefixed LODs are included even when no `lodParent`
/// references their id, which is how completely stray generated LODs are
/// discovered.
fn orphan_lod_indices(placements: &[Placement], states: &[ElementState]) -> Vec<usize> {
    use std::collections::{HashMap, HashSet};

    let referenced_ids = placements
        .iter()
        .filter_map(|placement| placement.attrs.get("lodParent"))
        .map(|parent| parent.trim())
        .filter(|parent| !parent.is_empty() && !parent.eq_ignore_ascii_case("self"))
        .map(str::to_ascii_lowercase)
        .collect::<HashSet<_>>();

    let mut target_candidates = HashMap::<String, Vec<usize>>::new();
    for (index, placement) in placements.iter().enumerate() {
        if !live_in_snapshot(states, index) {
            continue;
        }
        let id = placement.id.trim().to_ascii_lowercase();
        if !id.is_empty()
            && (referenced_ids.contains(&id) || is_lod_name(&placement.id, &placement.dff))
        {
            target_candidates.entry(id).or_default().push(index);
        }
    }

    let mut targets_with_children = HashSet::<usize>::new();
    for (child_index, child) in placements.iter().enumerate() {
        if !live_in_snapshot(states, child_index) {
            continue;
        }
        let Some(parent) = child
            .attrs
            .get("lodParent")
            .map(|value| value.trim())
            .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("self"))
        else {
            continue;
        };
        let Some(candidates) = target_candidates.get(&parent.to_ascii_lowercase()) else {
            continue;
        };
        if let Some(target) = resolve_lod_child_target(placements, child_index, candidates) {
            targets_with_children.insert(target);
        }
    }

    let mut orphans = target_candidates
        .into_values()
        .flatten()
        .filter(|index| !targets_with_children.contains(index))
        .collect::<Vec<_>>();
    orphans.sort_unstable();
    orphans
}

/// Mark every live LOD with no live children as deleted. Returns the number of
/// placements removed. The caller owns history and render/outliner refreshes.
pub(crate) fn remove_orphan_lods(app: &mut AppState) -> usize {
    let indices = orphan_lod_indices(&app.placements, &app.element_states);
    let mut removed = 0usize;
    for index in indices {
        if is_default_world_placement(&app.placements[index]) {
            continue;
        }
        let Some(state) = app.element_states.get_mut(index) else {
            continue;
        };
        if !state.deleted {
            state.deleted = true;
            removed += 1;
        }
    }
    removed
}

pub(crate) fn request_clear_all_lods(app: &mut AppState) {
    if writer_is_busy(app) {
        app.status_message =
            "LOD removal cannot start while another asset writer is running.".to_string();
        return;
    }
    let assignment_count = app
        .placements
        .iter()
        .enumerate()
        .filter(|(index, placement)| {
            !app.element_states
                .get(*index)
                .is_some_and(|state| state.deleted)
                && placement
                    .attrs
                    .get("lodParent")
                    .is_some_and(|parent| !parent.trim().is_empty())
        })
        .count();
    let lod_ids = collect_lod_ids(&app.placements);
    let lod_count = app
        .placements
        .iter()
        .enumerate()
        .filter(|(index, placement)| {
            !app.element_states
                .get(*index)
                .is_some_and(|state| state.deleted)
                && placement_is_lod(placement, &lod_ids)
        })
        .count();
    if assignment_count == 0 && lod_count == 0 {
        app.status_message = "The scene has no LOD assignments or LOD elements.".to_string();
        return;
    }
    app.confirm_dialog = Some(ConfirmDialog {
        action: ConfirmAction::ClearAllLods,
        title: "Clear All LODs?".to_string(),
        body: format!(
            "Clear {assignment_count} LOD assignment(s) and remove {lod_count} LOD element(s) from the scene?"
        ),
        detail: "This clears both lodParent=\"self\" and links to separate LOD elements, stages their unused DFF/COL/TXD assets for deletion, and removes their unused textures from shared TXDs. Asset cleanup clears Undo and Redo history."
            .to_string(),
        primary_label: "Clear All LODs".to_string(),
        secondary_label: None,
        secondary_action: None,
    });
}

#[derive(Default)]
struct LodAssetCandidates {
    definitions: HashSet<String>,
    dffs: HashSet<String>,
    cols: HashSet<String>,
    txds: HashSet<String>,
}

fn collect_lod_asset_candidates(
    placements: &[Placement],
    states: &[ElementState],
    definitions: &HashMap<String, Definition>,
) -> LodAssetCandidates {
    let lod_ids = collect_lod_ids(placements);
    let mut candidates = LodAssetCandidates::default();
    for (index, placement) in placements.iter().enumerate() {
        if states.get(index).is_some_and(|state| state.deleted)
            || !placement_is_lod(placement, &lod_ids)
        {
            continue;
        }
        let Some(definition) = definitions.get(&placement.id) else {
            candidates.dffs.insert(asset_key(&placement.dff, ".dff"));
            candidates.cols.insert(asset_key(&placement.dff, ".col"));
            continue;
        };
        candidates.definitions.insert(definition.id.clone());
        let dff = asset_key_opt(definition.attrs.get("dff"), &definition.id, ".dff");
        let col_fallback = dff.strip_suffix(".dff").unwrap_or(&dff);
        let col = asset_key_opt(definition.attrs.get("col"), col_fallback, ".col");
        candidates.dffs.insert(dff);
        candidates.cols.insert(col);
        if let Some(txd) = definition_txd_name_from_attrs(definition) {
            candidates.txds.insert(asset_key(txd, ".txd"));
        }
    }
    candidates
}

fn active_texture_references_for_txd(
    app: &AppState,
    txd_key: &str,
) -> Result<HashSet<String>, String> {
    let mut dffs = HashSet::new();
    for (index, placement) in app.placements.iter().enumerate() {
        if app
            .element_states
            .get(index)
            .is_some_and(|state| state.deleted)
        {
            continue;
        }
        let Some(definition) = app.definitions.get(&placement.id) else {
            continue;
        };
        if !definition_txd_name_from_attrs(definition)
            .is_some_and(|txd| asset_key(txd, ".txd") == txd_key)
        {
            continue;
        }
        dffs.insert(asset_key_opt(
            definition.attrs.get("dff"),
            &definition.id,
            ".dff",
        ));
    }

    let mut textures = HashSet::new();
    for dff in dffs {
        let entry = find_dff_entry_for_app(app, &dff).ok_or_else(|| {
            format!("Could not read {dff}; shared TXD texture cleanup was skipped")
        })?;
        let bytes = read_img_entry(&entry);
        let raw = parse_dff_mesh(&bytes[..dff_chunk_len(&bytes).min(bytes.len())]);
        textures.extend(
            raw.material_textures
                .into_iter()
                .map(|name| lower(name.trim()))
                .filter(|name| !name.is_empty()),
        );
    }
    Ok(textures)
}

fn stage_lod_asset_cleanup(
    app: &mut AppState,
    candidates: &LodAssetCandidates,
) -> (usize, usize, Vec<String>) {
    let summary = validation_summary(app);
    let unused_dffs = summary.unused_dffs.into_iter().collect::<HashSet<_>>();
    let unused_cols = summary.unused_cols.into_iter().collect::<HashSet<_>>();
    let unused_txds = summary.unused_txds.into_iter().collect::<HashSet<_>>();
    let mut deletes = candidates
        .dffs
        .intersection(&unused_dffs)
        .chain(candidates.cols.intersection(&unused_cols))
        .chain(candidates.txds.intersection(&unused_txds))
        .cloned()
        .collect::<HashSet<_>>();
    let deleted = deletes.len();
    let mut removed_textures = 0usize;
    let mut warnings = Vec::new();

    for txd_key in candidates.txds.difference(&deletes) {
        let used = match active_texture_references_for_txd(app, txd_key) {
            Ok(used) if !used.is_empty() => used,
            Ok(_) => continue,
            Err(err) => {
                warnings.push(err);
                continue;
            }
        };
        let Some(entry) = find_txd_entry_for_app(app, txd_key) else {
            warnings.push(format!(
                "Could not find {txd_key}; shared TXD texture cleanup was skipped"
            ));
            continue;
        };
        match purge_unused_texture_natives_from_txd(read_txd_entry_bytes(&entry), &used) {
            Ok((bytes, removed, _)) if removed > 0 => {
                let name = with_ext(txd_key, ".txd");
                if let Err(err) = upsert_replacement_asset(&wip_root_path(&app.root), &name, &bytes)
                {
                    warnings.push(format!("Could not stage cleaned {name}: {err}"));
                    continue;
                }
                app.pending_replacement_assets
                    .insert(txd_key.clone(), (name, bytes));
                app.pending_txd_writes.insert(txd_key.clone());
                app.pending_asset_deletes.remove(txd_key);
                reindex_staged_txd(app, txd_key);
                removed_textures += removed;
            }
            Ok(_) => {}
            Err(err) => warnings.push(format!("Could not clean {txd_key}: {err}")),
        }
    }

    for key in &deletes {
        app.pending_replacement_assets.remove(key);
        app.pending_txd_writes.remove(key);
    }
    app.pending_asset_deletes.extend(deletes.drain());
    for root in [&app.root, &wip_root_path(&app.root)] {
        if let Err(err) = remove_replacement_archive_entries(root, &app.pending_asset_deletes) {
            warnings.push(format!(
                "Could not prune staged assets in {}: {err}",
                root.display()
            ));
        }
    }
    let deleted_txds = candidates
        .txds
        .intersection(&app.pending_asset_deletes)
        .cloned()
        .collect::<Vec<_>>();
    for txd in deleted_txds {
        remove_txd_from_texture_index(app, &txd);
    }
    app.meshes.retain(|key, _| {
        key.split('|')
            .next()
            .is_none_or(|dff| !app.pending_asset_deletes.contains(dff))
    });
    app.collisions
        .retain(|key, _| !app.pending_asset_deletes.contains(key));
    if deleted > 0 || removed_textures > 0 {
        app.loaded_wip = true;
    }
    (deleted, removed_textures, warnings)
}

fn clear_all_lods_in_world(
    placements: &mut [Placement],
    states: &mut [ElementState],
) -> (usize, usize) {
    let lod_ids = collect_lod_ids(placements);
    let mut cleared = 0usize;
    let mut removed = 0usize;
    for (index, placement) in placements.iter_mut().enumerate() {
        if is_default_world_placement(placement)
            || states.get(index).is_some_and(|state| state.deleted)
        {
            continue;
        }
        if placement.attrs.remove("lodParent").is_some() {
            cleared += 1;
        }
        if placement_is_lod(placement, &lod_ids)
            && let Some(state) = states.get_mut(index)
        {
            state.deleted = true;
            removed += 1;
        }
    }
    (cleared, removed)
}

pub(crate) fn clear_all_lods(app: &mut AppState) {
    let candidates =
        collect_lod_asset_candidates(&app.placements, &app.element_states, &app.definitions);
    let (cleared, removed) = clear_all_lods_in_world(&mut app.placements, &mut app.element_states);
    if cleared == 0 && removed == 0 {
        app.status_message = "The scene no longer has any LODs to clear.".to_string();
        return;
    }
    let active_definition_ids = app
        .placements
        .iter()
        .enumerate()
        .filter(|(index, _)| {
            !app.element_states
                .get(*index)
                .is_some_and(|state| state.deleted)
        })
        .map(|(_, placement)| placement.id.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let removed_definitions = candidates
        .definitions
        .iter()
        .filter(|id| {
            !active_definition_ids.contains(&id.to_ascii_lowercase())
                && !app.readonly_definition_ids.contains(*id)
        })
        .cloned()
        .collect::<Vec<_>>();
    for id in &removed_definitions {
        app.definitions.remove(id);
    }
    let (deleted_assets, removed_textures, warnings) = stage_lod_asset_cleanup(app, &candidates);
    app.lod_ids = collect_lod_ids(&app.placements);
    invalidate_outliner_labels(app);
    rebuild_outliner_filter(app);
    invalidate_validation_cache(app);
    rebuild_render_cells(app);
    clear_history_for_external_change(app);
    app.status_message = format!(
        "Cleared {cleared} LOD assignment(s), removed {removed} LOD element(s) and {} definition(s), staged {deleted_assets} unused DFF/COL/TXD asset(s) for deletion, and removed {removed_textures} unused texture(s). Save to apply; Undo/Redo history cleared.",
        removed_definitions.len()
    );
    if !warnings.is_empty() {
        app.status_message.push_str(&format!(
            " Completed with {} cleanup warning(s); click the status bar for details.",
            warnings.len()
        ));
        let mut log = vec![app.status_message.clone()];
        log.extend(warnings);
        set_save_log(app, "Clear All LODs completed with warnings", log, true);
    }
}

#[cfg(test)]
mod lod_distance_tests {
    use super::*;

    fn bounds(size: Vec3) -> Bounds {
        Bounds {
            min: Vec3::ZERO,
            max: size,
        }
    }

    #[test]
    fn distance_defaults_follow_lod_graph() {
        let small = bounds(vec3(20.0, 10.0, 5.0));
        let large = bounds(vec3(100.0, 90.0, 2.0));
        assert_eq!(repaired_lod_distance(false, false, false, Some(small)), 112);
        assert_eq!(repaired_lod_distance(false, false, false, Some(large)), 168);
        assert_eq!(repaired_lod_distance(false, false, true, Some(large)), 168);
        assert_eq!(repaired_lod_distance(false, true, true, None), 299);
        assert_eq!(repaired_lod_distance(true, false, false, None), 300);
        assert_eq!(repaired_lod_distance(true, true, false, None), 1500);
    }

    #[test]
    fn sa_detail_distance_adds_100_to_the_aabb_radius() {
        let mesh_bounds = bounds(vec3(104.0, 0.0, 0.0));
        assert_eq!(sa_detail_lod_distance(mesh_bounds), 152);
        assert_eq!(
            repaired_lod_distance(false, false, true, Some(mesh_bounds)),
            152
        );
    }

    #[test]
    fn sa_detail_distance_stays_below_the_low_lod_boundary() {
        let mesh_bounds = bounds(vec3(500.0, 500.0, 500.0));
        assert_eq!(sa_detail_lod_distance(mesh_bounds), 299);
    }

    #[test]
    fn object_lod_model_distance_accounts_for_the_five_times_rule() {
        assert_eq!(generated_lod_distance(false), 300);
        assert_eq!(generated_lod_distance(false) * 5, 1500);
        let mesh_bounds = bounds(vec3(47.0, 13.0, 83.0));
        let model_distance = sa_lod_model_distance(false, mesh_bounds);
        assert_eq!(sa_effective_low_lod_distance(mesh_bounds), 1334);
        assert_eq!(model_distance, 267);
        assert!((1334..=1338).contains(&(model_distance * 5)));
    }

    #[test]
    fn building_lod_model_distance_is_the_screen_space_distance() {
        assert_eq!(generated_lod_distance(true), 1500);
        assert_eq!(
            sa_lod_model_distance(true, bounds(vec3(47.0, 13.0, 83.0))),
            1334
        );
        assert_eq!(
            sa_lod_model_distance(true, bounds(vec3(108.0, 124.0, 13.0))),
            1500
        );
    }

    #[test]
    fn effective_lod_distance_respects_the_300_minimum_and_1500_maximum() {
        assert_eq!(
            sa_effective_low_lod_distance(bounds(vec3(10.0, 10.0, 10.0))),
            300
        );
        assert_eq!(
            sa_effective_low_lod_distance(bounds(vec3(500.0, 500.0, 500.0))),
            1500
        );
    }

    #[test]
    fn existing_distance_only_skips_the_calculated_value() {
        assert!(lod_distance_matches(Some("170"), 170));
        assert!(lod_distance_matches(Some(" 170.0 "), 170));
        assert!(!lod_distance_matches(Some("170"), 300));
        assert!(!lod_distance_matches(Some("invalid"), 170));
        assert!(!lod_distance_matches(None, 170));
    }
}

#[cfg(test)]
mod orphan_lod_tests {
    use super::*;

    fn placement(id: &str, lod_parent: Option<&str>, unique_id: Option<&str>, x: f32) -> Placement {
        let mut attrs = BTreeMap::new();
        if let Some(lod_parent) = lod_parent {
            attrs.insert("lodParent".to_string(), lod_parent.to_string());
        }
        if let Some(unique_id) = unique_id {
            attrs.insert("uniqueID".to_string(), unique_id.to_string());
        }
        Placement {
            id: id.to_string(),
            dff: id.to_string(),
            zone: "test".to_string(),
            tag: "object".to_string(),
            attrs,
            pos: V3 { x, y: 0.0, z: 0.0 },
            rot: V3::default(),
        }
    }

    #[test]
    fn finds_unreferenced_name_prefixed_lod() {
        let placements = vec![
            placement("building", None, None, 0.0),
            placement("lod_building", None, None, 0.0),
        ];
        let states = vec![ElementState::default(); placements.len()];

        assert_eq!(orphan_lod_indices(&placements, &states), vec![1]);
    }

    #[test]
    fn keeps_lod_with_a_live_child() {
        let placements = vec![
            placement("building", Some("lod_building"), None, 0.0),
            placement("lod_building", None, None, 0.0),
        ];
        let states = vec![ElementState::default(); placements.len()];

        assert!(orphan_lod_indices(&placements, &states).is_empty());
    }

    #[test]
    fn repeated_lods_use_unique_ids_to_find_the_stray_instance() {
        let placements = vec![
            placement("building", Some("lod_building"), Some("12"), 100.0),
            placement("lod_building", None, Some("11"), 100.0),
            placement("lod_building", None, Some("12"), 0.0),
        ];
        let states = vec![ElementState::default(); placements.len()];

        assert_eq!(orphan_lod_indices(&placements, &states), vec![1]);
    }

    #[test]
    fn repeated_lods_without_unique_ids_keep_only_the_nearest_target() {
        let placements = vec![
            placement("building", Some("lod_building"), None, 90.0),
            placement("lod_building", None, None, 0.0),
            placement("lod_building", None, None, 100.0),
        ];
        let states = vec![ElementState::default(); placements.len()];

        assert_eq!(orphan_lod_indices(&placements, &states), vec![1]);
    }

    #[test]
    fn deleted_children_do_not_keep_lods_alive() {
        let placements = vec![
            placement("building", Some("low_building"), None, 0.0),
            placement("low_building", None, None, 0.0),
        ];
        let states = vec![
            ElementState {
                deleted: true,
                hidden: false,
            },
            ElementState::default(),
        ];

        assert_eq!(orphan_lod_indices(&placements, &states), vec![1]);
    }

    #[test]
    fn ignores_deleted_lods_and_non_lod_placements() {
        let placements = vec![
            placement("building", None, None, 0.0),
            placement("lod_building", None, None, 0.0),
        ];
        let states = vec![
            ElementState::default(),
            ElementState {
                deleted: true,
                hidden: false,
            },
        ];

        assert!(orphan_lod_indices(&placements, &states).is_empty());
    }
}

/// Fix ambiguous LOD pairings for *repeated* LOD models.
///
/// Some scenes contain many detail objects that all point at the same LOD id
/// (e.g. Liberty City's rail track has many `railtrax_straight` details sharing
/// a repeated `railtrax_straight` LOD). When the LOD id is not unique, the
/// `lodParent` attribute alone can't say which LOD instance belongs to which
/// detail, and no `uniqueID` is assigned to disambiguate them. This pairs each
/// such detail with its spatially closest LOD instance and stamps both with a
/// matching `uniqueID`.
///
/// Only LOD names shared by two or more LOD instances are touched; LODs with a
/// unique id are left alone. Existing non-empty `uniqueID`s are respected, so
/// the pass is idempotent. Returns the number of detail objects assigned an id.
pub(crate) fn fix_repeated_lod_unique_ids(app: &mut AppState) -> usize {
    use std::collections::{HashMap, HashSet};

    // Group LOD placements by lowercased id.
    let mut lod_groups: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, placement) in app.placements.iter().enumerate() {
        if placement_is_lod(placement, &app.lod_ids) {
            let name = placement.id.trim().to_ascii_lowercase();
            if !name.is_empty() {
                lod_groups.entry(name).or_default().push(idx);
            }
        }
    }
    // Only repeated LODs (a name shared by 2+ LOD instances) are ambiguous.
    lod_groups.retain(|_, indices| indices.len() > 1);
    if lod_groups.is_empty() {
        return 0;
    }

    // Fresh ids start one past the largest integer uniqueID already in the scene.
    let mut next_uid: u64 = next_scene_unique_id(&app.placements);

    let mut assigned = 0usize;

    for (name, lod_indices) in &lod_groups {
        // Detail objects pointing at this repeated LOD that still lack a uniqueID.
        let detail_indices: Vec<usize> = app
            .placements
            .iter()
            .enumerate()
            .filter(|(idx, p)| {
                !lod_indices.contains(idx)
                    && p.attrs
                        .get("lodParent")
                        .map(|v| v.trim().to_ascii_lowercase())
                        .as_deref()
                        == Some(name.as_str())
                    && p.attrs
                        .get("uniqueID")
                        .map(|v| v.trim().is_empty())
                        .unwrap_or(true)
            })
            .map(|(idx, _)| idx)
            .collect();
        if detail_indices.is_empty() {
            continue;
        }

        // Greedily pair each detail with its nearest still-unpaired LOD instance.
        let mut consumed: HashSet<usize> = HashSet::new();
        for detail_idx in detail_indices {
            let detail_pos = app.placements[detail_idx].pos;
            let nearest = lod_indices
                .iter()
                .copied()
                .filter(|idx| !consumed.contains(idx))
                .min_by(|a, b| {
                    v3_dist_sq(app.placements[*a].pos, detail_pos)
                        .total_cmp(&v3_dist_sq(app.placements[*b].pos, detail_pos))
                });
            let Some(lod_idx) = nearest else {
                break; // no LOD instances left to pair for this name
            };

            // Reuse the LOD's own uniqueID if it has one, otherwise mint a new one.
            let uid = app.placements[lod_idx]
                .attrs
                .get("uniqueID")
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| {
                    let uid = next_uid.to_string();
                    next_uid += 1;
                    uid
                });

            if is_default_world_placement(&app.placements[lod_idx])
                || is_default_world_placement(&app.placements[detail_idx])
            {
                continue;
            }
            set_optional_attr(&mut app.placements[lod_idx].attrs, "uniqueID", uid.clone());
            set_optional_attr(&mut app.placements[detail_idx].attrs, "uniqueID", uid);
            consumed.insert(lod_idx);
            assigned += 1;
        }
    }

    assigned
}

pub(crate) fn select_element_additive(app: &mut AppState, index: usize) {
    if index >= app.placements.len() {
        return;
    }
    app.selected_group = None;
    if app.selected_elements.contains(&index) {
        app.selected_elements.remove(&index);
        app.selected_element_order.retain(|idx| *idx != index);
        if app.selected == index {
            app.selected = app
                .selected_element_order
                .iter()
                .rev()
                .copied()
                .find(|idx| app.selected_elements.contains(idx))
                .unwrap_or(NO_SELECTION);
        }
    } else {
        app.selected_elements.insert(index);
        app.selected_element_order.push(index);
        app.selected = index;
    }
    if app
        .selected_col_face
        .is_some_and(|face| face.placement != app.selected)
    {
        app.selected_col_face = None;
    }
}

pub(crate) fn select_element_with_mode(app: &mut AppState, index: usize, additive: bool) {
    if additive {
        select_element_additive(app, index);
    } else {
        select_element(app, index);
    }
}

/// Selects the inclusive element range between the active element and `index`
/// in the currently displayed outliner. Group header rows are skipped.
pub(crate) fn select_outliner_range(app: &mut AppState, index: usize) {
    let anchor = app.selected;
    let Some(range) = outliner_element_range(&app.outliner_filter, anchor, index) else {
        select_element_additive(app, index);
        return;
    };
    for idx in range {
        if app.selected_elements.insert(idx) {
            app.selected_element_order.push(idx);
        }
    }
    app.selected = index;
    app.selected_group = None;
    app.selected_col_face = None;
}

fn outliner_element_range(
    entries: &[OutlinerEntry],
    anchor: usize,
    target: usize,
) -> Option<Vec<usize>> {
    let row_for = |wanted| {
        entries.iter().position(|entry| {
            matches!(entry, OutlinerEntry::Element(idx) | OutlinerEntry::GroupChild(idx) if *idx == wanted)
        })
    };
    let anchor_row = row_for(anchor)?;
    let target_row = row_for(target)?;
    let (start, end) = if anchor_row <= target_row {
        (anchor_row, target_row)
    } else {
        (target_row, anchor_row)
    };
    Some(
        entries[start..=end]
            .iter()
            .filter_map(|entry| match entry {
                OutlinerEntry::Element(idx) | OutlinerEntry::GroupChild(idx) => Some(*idx),
                OutlinerEntry::Group(_) => None,
            })
            .collect(),
    )
}

#[cfg(test)]
mod outliner_range_tests {
    use super::*;

    #[test]
    fn inclusive_range_skips_group_headers_and_works_in_reverse() {
        let entries = vec![
            OutlinerEntry::Element(4),
            OutlinerEntry::Group("signals".to_string()),
            OutlinerEntry::GroupChild(8),
            OutlinerEntry::GroupChild(12),
            OutlinerEntry::Element(20),
        ];

        assert_eq!(
            outliner_element_range(&entries, 4, 20),
            Some(vec![4, 8, 12, 20])
        );
        assert_eq!(
            outliner_element_range(&entries, 20, 8),
            Some(vec![8, 12, 20])
        );
        assert_eq!(outliner_element_range(&entries, 999, 8), None);
    }
}

pub(crate) const OUTLINER_ROW_H: f32 = 28.0;

pub(crate) fn outliner_list_top() -> f32 {
    TOP_H + 168.0
}

pub(crate) fn outliner_rows() -> usize {
    ((screen_height() - STATUS_H - 26.0 - outliner_list_top()) / OUTLINER_ROW_H).max(1.0) as usize
}

pub(crate) fn outliner_search_rect() -> Rect {
    Rect::new(20.0, TOP_H + 52.0, left_panel_width() - 52.0, 34.0)
}

pub(crate) fn outliner_type_filter_rect(slot: usize) -> Rect {
    let x0 = 20.0;
    let gap = 8.0;
    let w = (left_panel_width() - 52.0 - gap * 2.0) / 3.0;
    Rect::new(x0 + slot as f32 * (w + gap), TOP_H + 94.0, w, 26.0)
}

pub(crate) fn outliner_row_at(mouse: Vec2) -> Option<usize> {
    let list_top = outliner_list_top();
    if mouse.x >= left_panel_width() - 22.0
        || mouse.y <= list_top
        || mouse.y >= screen_height() - STATUS_H - 14.0
    {
        return None;
    }
    let row = ((mouse.y - list_top) / OUTLINER_ROW_H).floor() as usize;
    if row < outliner_rows() {
        Some(row)
    } else {
        None
    }
}

pub(crate) fn outliner_scrollbar_track() -> Rect {
    Rect::new(
        left_panel_width() - 22.0,
        outliner_list_top(),
        8.0,
        screen_height() - STATUS_H - 14.0 - outliner_list_top(),
    )
}

pub(crate) fn outliner_scrollbar_thumb(app: &AppState) -> Option<Rect> {
    let track = outliner_scrollbar_track();
    let rows = outliner_rows();
    if app.outliner_filter.len() <= rows {
        return None;
    }
    let visible_frac = (rows as f32 / app.outliner_filter.len() as f32).clamp(0.04, 1.0);
    let thumb_h = (track.h * visible_frac).max(28.0).min(track.h);
    let max_scroll = app.outliner_filter.len().saturating_sub(rows) as f32;
    let t = if max_scroll > 0.0 {
        app.scroll / max_scroll
    } else {
        0.0
    };
    let thumb_y = track.y + (track.h - thumb_h) * t.clamp(0.0, 1.0);
    Some(Rect::new(track.x, thumb_y, track.w, thumb_h))
}

pub(crate) fn set_outliner_scroll_from_thumb_y(app: &mut AppState, thumb_y: f32) {
    let track = outliner_scrollbar_track();
    let rows = outliner_rows();
    let max_scroll = app.outliner_filter.len().saturating_sub(rows) as f32;
    let Some(thumb) = outliner_scrollbar_thumb(app) else {
        app.scroll = 0.0;
        return;
    };
    if max_scroll <= 0.0 || track.h <= thumb.h {
        app.scroll = 0.0;
        return;
    }
    let t = ((thumb_y - track.y) / (track.h - thumb.h)).clamp(0.0, 1.0);
    app.scroll = (t * max_scroll).round();
}

pub(crate) fn update_outliner_scroll_from_mouse(app: &mut AppState, mouse: Vec2) -> bool {
    if app.outliner_scroll_drag.is_some() {
        if is_mouse_button_down(MouseButton::Left) {
            app.scroll_interaction_until = get_time() + 0.18;
            let grab_offset = app
                .outliner_scroll_drag
                .as_ref()
                .map(|drag| drag.grab_offset)
                .unwrap_or(0.0);
            set_outliner_scroll_from_thumb_y(app, mouse.y - grab_offset);
            return true;
        }
        app.outliner_scroll_drag = None;
        return true;
    }

    let track = outliner_scrollbar_track();
    let thumb = outliner_scrollbar_thumb(app);
    if is_mouse_button_pressed(MouseButton::Left) {
        if let Some(thumb) = thumb {
            if thumb.contains(mouse) {
                app.outliner_scroll_drag = Some(OutlinerScrollDrag {
                    grab_offset: mouse.y - thumb.y,
                });
                app.scroll_interaction_until = get_time() + 0.18;
                return true;
            }
        }
        let hit_area = Rect::new(track.x - 6.0, track.y, track.w + 12.0, track.h);
        if hit_area.contains(mouse) {
            let thumb_h = thumb.map(|thumb| thumb.h).unwrap_or(28.0);
            app.outliner_scroll_drag = Some(OutlinerScrollDrag {
                grab_offset: thumb_h * 0.5,
            });
            app.scroll_interaction_until = get_time() + 0.18;
            set_outliner_scroll_from_thumb_y(app, mouse.y - thumb_h * 0.5);
            return true;
        }
    }
    if !track.contains(mouse) || !is_mouse_button_down(MouseButton::Left) {
        return false;
    }
    false
}

const TOOLBAR_LOAD: usize = 9;
const TOOLBAR_SAVE_AS: usize = 10;
const TOOLBAR_SAVE_WIP: usize = 11;
const TOOLBAR_GENERATE_TXD: usize = 13;
const TOOLBAR_IMPORT_BLENDER: usize = 14;
const TOOLBAR_PREFERENCES: usize = 15;
const TOOLBAR_IMPORT_ASSET: usize = 16;

const FILE_ACTION_CONTROLS: [(usize, f32); 7] = [
    (TOOLBAR_LOAD, 70.0),
    (TOOLBAR_SAVE_AS, 92.0),
    (TOOLBAR_SAVE_WIP, 104.0),
    (TOOLBAR_GENERATE_TXD, 120.0),
    (TOOLBAR_IMPORT_BLENDER, 136.0),
    (TOOLBAR_PREFERENCES, 118.0),
    (TOOLBAR_IMPORT_ASSET, 150.0),
];
const FILE_ACTION_GAP: f32 = 8.0;
const FILE_ACTION_SCREEN_MARGIN: f32 = 18.0;

fn compact_file_actions_for_width(screen_w: f32) -> bool {
    let start_x = file_save_rect().x + file_save_rect().w + 10.0;
    let desired_width: f32 = FILE_ACTION_CONTROLS.iter().map(|(_, width)| *width).sum();
    let gaps = FILE_ACTION_GAP * FILE_ACTION_CONTROLS.len().saturating_sub(1) as f32;
    screen_w - start_x - FILE_ACTION_SCREEN_MARGIN < desired_width + gaps
}

pub(crate) fn compact_file_actions() -> bool {
    compact_file_actions_for_width(screen_width())
}

#[cfg(test)]
mod file_action_toolbar_tests {
    use super::compact_file_actions_for_width;

    #[test]
    fn compact_menu_replaces_controls_before_they_need_to_shrink() {
        assert!(compact_file_actions_for_width(640.0));
        assert!(compact_file_actions_for_width(1337.0));
        assert!(!compact_file_actions_for_width(1338.0));
        assert!(!compact_file_actions_for_width(1600.0));
    }
}

pub(crate) fn file_actions_overflow_rect() -> Rect {
    let x = file_save_rect().x + file_save_rect().w + 10.0;
    let available = (screen_width() - x - FILE_ACTION_SCREEN_MARGIN).max(1.0);
    Rect::new(x, 18.0, available.min(140.0), 36.0)
}

pub(crate) fn file_action_menu_row_rect(row: usize) -> Rect {
    let button = file_actions_overflow_rect();
    let width = 196.0_f32.min((screen_width() - 12.0).max(1.0));
    let x = (button.x + button.w - width).clamp(6.0, (screen_width() - width - 6.0).max(6.0));
    Rect::new(
        x,
        button.y + button.h + 8.0 + row as f32 * 34.0,
        width,
        30.0,
    )
}

pub(crate) fn file_actions_menu_bounds() -> Rect {
    let first = file_action_menu_row_rect(0);
    Rect::new(
        first.x - 5.0,
        first.y - 5.0,
        first.w + 10.0,
        FILE_ACTION_MENU_LABELS.len() as f32 * 34.0 + 6.0,
    )
}

fn toolbar_control_rect(control: usize) -> Rect {
    // File/project actions live in the top row. Transform actions have their
    // own strip below workspace navigation, so this row never fights the
    // project identity for horizontal space.
    let start_x = file_save_rect().x + file_save_rect().w + 10.0;
    let available = (screen_width() - start_x - 18.0).max(1.0);
    if compact_file_actions() {
        return Rect::new(0.0, 0.0, 0.0, 0.0);
    }
    let gap_total = FILE_ACTION_GAP * FILE_ACTION_CONTROLS.len().saturating_sub(1) as f32;
    let desired_total: f32 = FILE_ACTION_CONTROLS.iter().map(|(_, width)| *width).sum();
    debug_assert!(available >= desired_total + gap_total);
    let mut x = start_x;
    for (id, width) in FILE_ACTION_CONTROLS {
        if id == control {
            return Rect::new(x, 18.0, width, 36.0);
        }
        x += width + FILE_ACTION_GAP;
    }
    Rect::new(0.0, 0.0, 0.0, 0.0)
}

pub(crate) fn toolbar_button_rect(slot: usize) -> Rect {
    Rect::new(14.0 + slot as f32 * 44.0, TOP_H - 38.0, 36.0, 36.0)
}

pub(crate) fn snap_mode_rect() -> Rect {
    Rect::new(374.0, TOP_H - 38.0, 66.0, 36.0)
}

pub(crate) fn transform_space_rect() -> Rect {
    Rect::new(448.0, TOP_H - 38.0, 78.0, 36.0)
}

pub(crate) fn file_save_rect() -> Rect {
    Rect::new(400.0, 18.0, 72.0, 36.0)
}

pub(crate) fn load_resource_rect() -> Rect {
    toolbar_control_rect(TOOLBAR_LOAD)
}

pub(crate) fn save_as_rect() -> Rect {
    toolbar_control_rect(TOOLBAR_SAVE_AS)
}

pub(crate) fn save_wip_rect() -> Rect {
    toolbar_control_rect(TOOLBAR_SAVE_WIP)
}

pub(crate) fn preferences_button_rect() -> Rect {
    toolbar_control_rect(TOOLBAR_PREFERENCES)
}

pub(crate) fn import_asset_button_rect() -> Rect {
    toolbar_control_rect(TOOLBAR_IMPORT_ASSET)
}

pub(crate) fn import_blender_button_rect() -> Rect {
    toolbar_control_rect(TOOLBAR_IMPORT_BLENDER)
}

pub(crate) fn generate_txd_button_rect() -> Rect {
    toolbar_control_rect(TOOLBAR_GENERATE_TXD)
}

pub(crate) const SHOW_BLENDER_IMPORT: bool = true;

/// Tooltips are queued while controls are drawn, then flushed after dialogs
/// and menus. Macroquad uses draw order for layering, so this is the UI
/// equivalent of a dedicated, always-on-top tooltip layer.
static PENDING_UI_TOOLTIP: OnceLock<Mutex<Option<(Rect, String)>>> = OnceLock::new();

fn pending_ui_tooltip() -> std::sync::MutexGuard<'static, Option<(Rect, String)>> {
    PENDING_UI_TOOLTIP
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub(crate) fn icon_button(
    font: &Font,
    rect: Rect,
    icon: &Texture2D,
    active: bool,
    enabled: bool,
    label: &str,
) {
    let mouse: Vec2 = mouse_position().into();
    let pointer_over = !scrollbar_hover_suppressed() && rect.contains(mouse);
    let hovered = enabled && pointer_over;
    let bg = if !enabled {
        ui_input_bg()
    } else if active {
        ui_surface_active()
    } else if hovered {
        ui_surface_hover()
    } else {
        ui_surface()
    };
    let border = if !enabled {
        ui_border()
    } else if active {
        ui_accent()
    } else if hovered {
        Color::new(0.34, 0.36, 0.40, 1.0)
    } else {
        ui_border()
    };
    draw_rrect(
        rect.x + 1.0,
        rect.y + 2.0,
        rect.w,
        rect.h,
        9.0,
        Color::new(0.0, 0.0, 0.0, if hovered { 0.30 } else { 0.20 }),
    );
    draw_rrect_bordered(rect.x, rect.y, rect.w, rect.h, 9.0, 1.0, bg, border);
    if enabled && !active {
        draw_line(
            rect.x + 8.0,
            rect.y + 1.0,
            rect.x + rect.w - 8.0,
            rect.y + 1.0,
            1.0,
            Color::new(0.52, 0.55, 0.60, 0.14),
        );
    }
    let tint = if enabled {
        WHITE
    } else {
        Color::new(0.42, 0.44, 0.48, 1.0)
    };
    let icon_size = (rect.w - 8.0).min(28.0).max(8.0);
    draw_texture_ex(
        icon,
        rect.x + (rect.w - icon_size) * 0.5,
        rect.y + (rect.h - icon_size) * 0.5,
        tint,
        DrawTextureParams {
            dest_size: Some(vec2(icon_size, icon_size)),
            ..Default::default()
        },
    );
    if pointer_over {
        let tooltip = if enabled {
            label.to_string()
        } else {
            format!("{label} — unavailable")
        };
        draw_text_tooltip(font, rect, &tooltip);
    }
}

pub(crate) fn toolbar_contains(mouse: Vec2) -> bool {
    (0..7).any(|slot| toolbar_button_rect(slot).contains(mouse))
        || snap_mode_rect().contains(mouse)
        || file_save_rect().contains(mouse)
        || load_resource_rect().contains(mouse)
        || save_as_rect().contains(mouse)
        || save_wip_rect().contains(mouse)
        || generate_txd_button_rect().contains(mouse)
        || (SHOW_BLENDER_IMPORT && import_blender_button_rect().contains(mouse))
        || preferences_button_rect().contains(mouse)
        || import_asset_button_rect().contains(mouse)
        || (compact_file_actions() && file_actions_overflow_rect().contains(mouse))
        || transform_space_rect().contains(mouse)
}

pub(crate) const FILE_ACTION_MENU_LABELS: [&str; 7] = [
    "Load resource",
    "Save As",
    "Save WIP",
    "Build TXD",
    "Import Blender",
    "Preferences",
    "Import new asset",
];

fn run_file_action(app: &mut AppState, row: usize) {
    match row {
        0 => open_load_picker(app),
        1 => open_save_as_dialog(app),
        2 => {
            save_wip_scene(app);
        }
        3 => open_generate_txd_folder_picker(app),
        4 => open_blender_import_dialog(app),
        5 => open_preferences_dialog(app),
        6 => open_import_new_asset_picker(app),
        _ => {}
    }
}

pub(crate) fn app_tab_label(tab: AppTab) -> &'static str {
    match tab {
        AppTab::Preview => "Game World",
        AppTab::LodAudit => "LOD Audit",
        AppTab::TextureReview => "Texture Review",
        AppTab::Scene => "Scene",
        AppTab::Vehicles => "Vehicles",
        AppTab::Validation => "Validation",
        AppTab::Editing => "Editing",
        AppTab::Collisions => "Collisions",
        AppTab::Lights => "Lights",
        AppTab::Bake => "Vertex Lighting",
        AppTab::Water => "Water",
        AppTab::Cull => "CULL",
        AppTab::Race => "Race",
        AppTab::Simulate => "Simulate",
    }
}

pub(crate) fn app_tab_tooltip(tab: AppTab) -> &'static str {
    match tab {
        AppTab::Preview => "Browse and place map elements in the game world.",
        AppTab::LodAudit => "Review level-of-detail assignments and find LOD issues.",
        AppTab::TextureReview => "Find and resolve missing or mismatched textures.",
        AppTab::Scene => "Manage scene elements, selection, and asset placement.",
        AppTab::Vehicles => "Browse and edit vehicle models and their assets.",
        AppTab::Validation => "Check the project for problems and apply available fixes.",
        AppTab::Editing => "Edit an asset's geometry, materials, and textures.",
        AppTab::Collisions => "View collision geometry (read-only).",
        AppTab::Lights => "Place and configure scene lighting.",
        AppTab::Bake => "Bake vertex lighting for the current project.",
        AppTab::Water => "Create and adjust water planes.",
        AppTab::Cull => "Create water hiding volumes stored as .map entries.",
        AppTab::Race => "Create and edit race tracks, checkpoints, and radar settings.",
        AppTab::Simulate => "Test the project in simulation mode.",
    }
}

pub(crate) fn app_tabs_for_mode(mode: LaunchMode) -> Vec<AppTab> {
    let tabs = [
        AppTab::Preview,
        AppTab::LodAudit,
        AppTab::TextureReview,
        AppTab::Scene,
        AppTab::Vehicles,
        AppTab::Validation,
        AppTab::Editing,
        AppTab::Collisions,
        AppTab::Lights,
        AppTab::Bake,
        AppTab::Water,
        AppTab::Cull,
        AppTab::Race,
    ];
    tabs.into_iter()
        .filter(|tab| mode != LaunchMode::Editor || !matches!(tab, AppTab::Preview | AppTab::Bake))
        .collect()
}

/// The top-level workspaces are deliberately limited to the tasks used while
/// authoring. Audits and specialised tools remain one click away in More.
pub(crate) fn primary_app_tabs_for_mode(mode: LaunchMode) -> Vec<AppTab> {
    app_tabs_for_mode(mode)
        .into_iter()
        .filter(|tab| {
            matches!(
                tab,
                AppTab::Preview
                    | AppTab::Scene
                    | AppTab::Vehicles
                    | AppTab::Editing
                    | AppTab::Collisions
                    | AppTab::Lights
            )
        })
        .collect()
}

pub(crate) fn overflow_app_tabs_for_mode(mode: LaunchMode) -> Vec<AppTab> {
    app_tabs_for_mode(mode)
        .into_iter()
        .filter(|tab| !primary_app_tabs_for_mode(mode).contains(tab))
        .collect()
}

pub(crate) fn app_tab_rect(app: &AppState, tab: AppTab) -> Rect {
    let tab_y = 96.0;
    let tabs = primary_app_tabs_for_mode(app.options.launch_mode);
    let gap = if screen_width() < 1000.0 { 3.0 } else { 6.0 };
    let available = (screen_width() - 12.0 - gap * tabs.len().saturating_sub(1) as f32).max(1.0);
    let natural_widths: Vec<f32> = tabs
        .iter()
        .map(|candidate| ui_text_width(app_tab_label(*candidate), 16) + 16.0)
        .collect();
    let natural_total: f32 = natural_widths.iter().sum();
    let scale = (available / natural_total).min(1.0);
    let mut tab_x = 6.0;
    for (candidate, natural_w) in tabs.into_iter().zip(natural_widths) {
        let rect = Rect::new(tab_x, tab_y - 18.0, natural_w * scale, 26.0);
        if candidate == tab {
            return rect;
        }
        tab_x += rect.w + gap;
    }
    Rect::new(0.0, 0.0, 0.0, 0.0)
}

pub(crate) fn more_tabs_rect(app: &AppState) -> Rect {
    let tabs = primary_app_tabs_for_mode(app.options.launch_mode);
    let gap = if screen_width() < 1000.0 { 3.0 } else { 6.0 };
    let last_right = tabs
        .last()
        .map(|tab| {
            let rect = app_tab_rect(app, *tab);
            rect.x + rect.w
        })
        .unwrap_or(6.0);
    Rect::new(last_right + gap, 78.0, 72.0, 26.0)
}

pub(crate) fn overflow_tab_rect(app: &AppState, tab: AppTab) -> Rect {
    let tabs = overflow_app_tabs_for_mode(app.options.launch_mode);
    let menu = more_tabs_rect(app);
    let row = tabs
        .iter()
        .position(|candidate| *candidate == tab)
        .unwrap_or(0) as f32;
    Rect::new(
        menu.x - 112.0,
        menu.y + menu.h + 8.0 + row * 32.0,
        176.0,
        28.0,
    )
}

pub(crate) fn navigation_menu_bounds(app: &AppState) -> Rect {
    let tabs = overflow_app_tabs_for_mode(app.options.launch_mode);
    let first = overflow_tab_rect(app, tabs.first().copied().unwrap_or(AppTab::Validation));
    Rect::new(
        first.x - 5.0,
        first.y - 5.0,
        first.w + 10.0,
        tabs.len() as f32 * 32.0 + 10.0,
    )
}

pub(crate) fn handle_tab_click(app: &mut AppState, mouse: Vec2) -> bool {
    if !is_mouse_button_pressed(MouseButton::Left) {
        return false;
    }
    if more_tabs_rect(app).contains(mouse) {
        app.navigation_menu_open = !app.navigation_menu_open;
        if app.navigation_menu_open {
            app.file_actions_menu_open = false;
        }
        return true;
    }
    if app.navigation_menu_open {
        for tab in overflow_app_tabs_for_mode(app.options.launch_mode) {
            if overflow_tab_rect(app, tab).contains(mouse) {
                activate_app_tab(app, tab);
                app.navigation_menu_open = false;
                return true;
            }
        }
        app.navigation_menu_open = false;
        return navigation_menu_bounds(app).contains(mouse);
    }
    for tab in primary_app_tabs_for_mode(app.options.launch_mode) {
        if app_tab_rect(app, tab).contains(mouse) {
            activate_app_tab(app, tab);
            return true;
        }
    }
    false
}

fn activate_app_tab(app: &mut AppState, tab: AppTab) {
    if app.active_tab == AppTab::Simulate && tab != AppTab::Simulate {
        app.sim.playing = false;
        restore_sim_editor_camera(app);
    }
    switch_app_tab(app, tab);
    app.inspector_edit = None;
    app.properties_scroll = 0.0;
    if tab == AppTab::Validation {
        refresh_validation_cache(app);
    }
    if tab == AppTab::LodAudit
        && app.lod_audit.rx.is_none()
        && (app.lod_audit.result.is_none() || app.lod_audit.stale)
    {
        request_lod_audit(app);
    }
    if tab == AppTab::TextureReview
        && app.missing_texture_review.rx.is_none()
        && (app.missing_texture_review.result.is_none() || app.missing_texture_review.stale)
    {
        request_missing_texture_review(app);
    }
}

fn apply_camera_after_tab_restore(
    app: &mut AppState,
    camera: CameraState,
    mode: CameraMode,
    focus: Option<Vec3>,
) {
    app.camera = camera;
    app.camera_mode = mode;
    app.camera_focus = focus;
    app.camera.looking = false;
    app.camera.last_mouse = mouse_position().into();
    set_cursor_grab(false);
    show_mouse(true);
}

pub(crate) fn remember_world_camera_before_editing(app: &mut AppState) {
    if app.active_tab != AppTab::Editing {
        remember_active_tab_camera(app);
    }
}

fn remember_active_tab_camera(app: &mut AppState) {
    match app.active_tab {
        AppTab::Editing => {
            app.editing.camera = Some(app.camera);
            app.editing.camera_mode = Some(app.camera_mode);
            app.editing.camera_focus = Some(app.camera_focus);
        }
        AppTab::Vehicles => {
            app.vehicle_browser.camera = Some(app.camera);
            app.vehicle_browser.camera_mode = Some(app.camera_mode);
            app.vehicle_browser.camera_focus = Some(app.camera_focus);
        }
        _ => {
            app.gameworld_camera = Some(app.camera);
            app.gameworld_camera_mode = Some(app.camera_mode);
            app.gameworld_camera_focus = Some(app.camera_focus);
        }
    }
}

pub(crate) fn switch_app_tab(app: &mut AppState, tab: AppTab) {
    if app.active_tab == tab {
        return;
    }
    if app.active_tab == AppTab::Validation {
        app.asset_optimization_menu_open = false;
    }
    if app.active_tab == AppTab::Vehicles {
        if app.vehicle_browser.photo_mode {
            set_vehicle_photo_mode(app, false);
        }
    }
    remember_active_tab_camera(app);
    let camera = match tab {
        AppTab::Editing => app.editing.camera.or(app.gameworld_camera),
        AppTab::Vehicles => app.vehicle_browser.camera.or(app.gameworld_camera),
        _ => app.gameworld_camera,
    }
    .unwrap_or(app.camera);
    let (mode, focus) = match tab {
        AppTab::Editing => (
            app.editing.camera_mode.unwrap_or(CameraMode::Freeroam),
            app.editing.camera_focus.unwrap_or(None),
        ),
        AppTab::Vehicles => (
            app.vehicle_browser
                .camera_mode
                .unwrap_or(CameraMode::Freeroam),
            app.vehicle_browser.camera_focus.unwrap_or(None),
        ),
        _ => (
            app.gameworld_camera_mode.unwrap_or(app.camera_mode),
            app.gameworld_camera_focus.unwrap_or(app.camera_focus),
        ),
    };
    apply_camera_after_tab_restore(app, camera, mode, focus);
    app.active_tab = tab;
}

pub(crate) fn draw_text_tooltip(font: &Font, anchor: Rect, label: &str) {
    let _ = font;
    *pending_ui_tooltip() = Some((anchor, label.to_string()));
}

pub(crate) fn clear_pending_ui_tooltip() {
    *pending_ui_tooltip() = None;
}

pub(crate) fn draw_pending_ui_tooltip(font: &Font) {
    let Some((anchor, label)) = pending_ui_tooltip().take() else {
        return;
    };
    let screen_w = screen_width();
    let screen_h = screen_height();
    let max_tip_w = (screen_w - 16.0).clamp(48.0, 380.0);
    let max_text_w = (max_tip_w - 16.0).max(32.0);
    let mut lines = Vec::<String>::new();
    for paragraph in label.lines() {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
            if !line.is_empty() && ui_text_width(&candidate, 14) > max_text_w {
                lines.push(line);
                line = word.to_string();
            } else {
                line = candidate;
            }
        }
        if !line.is_empty() {
            lines.push(line);
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    let tip_w = (lines
        .iter()
        .map(|line| ui_text_width(line, 14))
        .fold(0.0_f32, f32::max)
        + 16.0)
        .clamp(48.0, max_tip_w);
    let line_h = 18.0;
    let tip_h = lines.len() as f32 * line_h + 10.0;
    // Right-panel controls sit against the screen edge. Clamp both axes so
    // their tooltips extend inward and remain fully visible at any window size.
    let tip_x = anchor.x.min(screen_w - tip_w - 8.0).max(8.0);
    let below_y = anchor.y + anchor.h + 7.0;
    let tip_y = if below_y + tip_h <= screen_h - 8.0 {
        below_y
    } else {
        (anchor.y - tip_h - 7.0)
            .min(screen_h - tip_h - 8.0)
            .max(8.0)
    };
    draw_rrect_bordered(
        tip_x,
        tip_y,
        tip_w,
        tip_h,
        8.0,
        1.0,
        Color::new(0.025, 0.035, 0.050, 0.98),
        Color::new(0.25, 0.27, 0.30, 1.0),
    );
    for (index, line) in lines.iter().enumerate() {
        let visible = ellipsize_width(line, 14, tip_w - 16.0);
        ui_text_size(
            font,
            &visible,
            tip_x + 8.0,
            tip_y + 17.0 + index as f32 * line_h,
            14,
            WHITE,
        );
    }
}

fn draw_text_button_state(
    font: &Font,
    rect: Rect,
    label: &str,
    active: bool,
    enabled: bool,
    busy: bool,
    disabled_reason: Option<&str>,
) {
    let mouse: Vec2 = mouse_position().into();
    let hovered = !scrollbar_hover_suppressed() && rect.contains(mouse);
    let bg = if !enabled && !busy {
        Color::new(0.045, 0.058, 0.078, 1.0)
    } else if active || busy {
        ui_surface_active()
    } else if hovered {
        ui_surface_hover()
    } else {
        ui_surface()
    };
    let border = if enabled && (active || hovered || busy) {
        ui_accent()
    } else {
        ui_border()
    };
    draw_rrect(
        rect.x + 1.0,
        rect.y + 2.0,
        rect.w,
        rect.h,
        9.0,
        Color::new(0.0, 0.0, 0.0, if hovered { 0.28 } else { 0.18 }),
    );
    draw_rrect_bordered(rect.x, rect.y, rect.w, rect.h, 9.0, 1.0, bg, border);
    if enabled && !active && !busy {
        draw_line(
            rect.x + 9.0,
            rect.y + 1.0,
            rect.x + rect.w - 9.0,
            rect.y + 1.0,
            1.0,
            Color::new(0.52, 0.55, 0.60, 0.12),
        );
    }
    let display_label = if busy {
        format!("{label}...")
    } else {
        label.to_string()
    };
    let visible = ellipsize_width(&display_label, 16, rect.w - 18.0);
    let tw = ui_text_width(&visible, 16);
    ui_text(
        font,
        &visible,
        rect.x + ((rect.w - tw) * 0.5).max(9.0),
        rect.y + 23.0,
        if enabled || busy {
            WHITE
        } else {
            Color::new(0.37, 0.42, 0.49, 1.0)
        },
    );
    if hovered {
        if let Some(reason) = disabled_reason.filter(|_| !enabled) {
            draw_text_tooltip(font, rect, reason);
        } else if visible != display_label {
            draw_text_tooltip(font, rect, &display_label);
        }
    }
}

pub(crate) fn text_button(font: &Font, rect: Rect, label: &str, active: bool) {
    draw_text_button_state(font, rect, label, active, true, false, None);
}

#[allow(dead_code)]
pub(crate) fn text_button_disabled(font: &Font, rect: Rect, label: &str, reason: &str) {
    draw_text_button_state(font, rect, label, false, false, false, Some(reason));
}

pub(crate) fn text_button_busy(font: &Font, rect: Rect, label: &str) {
    draw_text_button_state(font, rect, label, true, false, true, Some("Working..."));
}

pub(crate) fn toolbar_primary_button(font: &Font, rect: Rect, label: &str) {
    let mouse: Vec2 = mouse_position().into();
    let hovered = !scrollbar_hover_suppressed() && rect.contains(mouse);
    let fill = if hovered {
        Color::new(0.19, 0.48, 0.78, 1.0)
    } else {
        Color::new(0.12, 0.38, 0.67, 1.0)
    };
    draw_rrect_bordered(rect.x, rect.y, rect.w, rect.h, 9.0, 1.0, fill, ui_accent());
    let tw = ui_text_width(label, 16);
    ui_text(
        font,
        label,
        rect.x + (rect.w - tw) * 0.5,
        rect.y + 23.0,
        WHITE,
    );
    if hovered {
        draw_text_tooltip(font, rect, "Save project (Ctrl+S)");
    }
}

pub(crate) fn handle_toolbar_click(app: &mut AppState, mouse: Vec2) -> bool {
    if !compact_file_actions() {
        app.file_actions_menu_open = false;
    }
    let open_file_menu = compact_file_actions() && app.file_actions_menu_open;
    if !toolbar_contains(mouse) && !(open_file_menu && file_actions_menu_bounds().contains(mouse)) {
        if open_file_menu && is_mouse_button_pressed(MouseButton::Left) {
            app.file_actions_menu_open = false;
            return true;
        }
        return false;
    }
    if !is_mouse_button_pressed(MouseButton::Left) {
        return true;
    }
    if compact_file_actions() && file_actions_overflow_rect().contains(mouse) {
        app.file_actions_menu_open = !app.file_actions_menu_open;
        if app.file_actions_menu_open {
            app.navigation_menu_open = false;
        }
        return true;
    }
    if open_file_menu {
        for row in 0..FILE_ACTION_MENU_LABELS.len() {
            if file_action_menu_row_rect(row).contains(mouse) {
                app.file_actions_menu_open = false;
                run_file_action(app, row);
                return true;
            }
        }
        // Consume padding clicks in the popup instead of passing them through
        // to viewport tools underneath it.
        return true;
    }
    let has_selection = has_active_selection(app);
    let selected_deleted = active_selection_deleted(app);
    if toolbar_button_rect(0).contains(mouse) {
        app.transform_mode = TransformMode::Select;
    } else if toolbar_button_rect(1).contains(mouse) {
        if has_selection && !selected_deleted {
            app.transform_mode = TransformMode::Move;
        }
    } else if toolbar_button_rect(2).contains(mouse) {
        if has_selection && !selected_deleted {
            app.transform_mode = TransformMode::Rotate;
        }
    } else if toolbar_button_rect(3).contains(mouse) {
        if app.active_tab == AppTab::Editing && !selected_editing_dff_vertices(app).is_empty() {
            app.transform_mode = TransformMode::Scale;
        }
    } else if toolbar_button_rect(4).contains(mouse) {
        if has_selection && !selected_deleted {
            if app.active_tab == AppTab::Lights {
                duplicate_selected_light(app);
            } else {
                duplicate_selected(app);
            }
        }
    } else if toolbar_button_rect(5).contains(mouse) {
        if has_selection {
            if app.active_tab == AppTab::Lights {
                delete_selected_light(app);
            } else if selected_deleted {
                restore_selected(app);
            } else {
                delete_selected(app);
            }
        }
    } else if toolbar_button_rect(6).contains(mouse) {
        undo(app);
    } else if toolbar_button_rect(7).contains(mouse) {
        redo(app);
    } else if file_save_rect().contains(mouse) {
        save_scene(app);
    } else if snap_mode_rect().contains(mouse) {
        app.snap_enabled = !app.snap_enabled;
        app.status_message = if app.snap_enabled {
            format!(
                "Snap on: move {:.3}, rotate {:.3}",
                app.snap_move, app.snap_rotate
            )
        } else {
            "Snap off".to_string()
        };
    } else if load_resource_rect().contains(mouse) {
        open_load_picker(app);
    } else if save_as_rect().contains(mouse) {
        open_save_as_dialog(app);
    } else if save_wip_rect().contains(mouse) {
        save_wip_scene(app);
    } else if generate_txd_button_rect().contains(mouse) {
        open_generate_txd_folder_picker(app);
    } else if SHOW_BLENDER_IMPORT && import_blender_button_rect().contains(mouse) {
        open_blender_import_dialog(app);
    } else if preferences_button_rect().contains(mouse) {
        open_preferences_dialog(app);
    } else if import_asset_button_rect().contains(mouse) {
        open_import_new_asset_picker(app);
    } else if transform_space_rect().contains(mouse) {
        if selected_editing_dff_freeform_pivot(app).is_some() {
            app.transform_space = TransformSpace::Local;
            app.status_message = "Freeform pivot gimbal follows the pivot's local axes".to_string();
        } else {
            app.transform_space = if app.transform_space == TransformSpace::World {
                TransformSpace::Local
            } else {
                TransformSpace::World
            };
        }
    }
    true
}

pub(crate) fn sim_button_rect(slot: usize) -> Rect {
    let x = screen_width() - right_panel_width() + 26.0;
    Rect::new(x + slot as f32 * 112.0, TOP_H + 74.0, 104.0, 34.0)
}

pub(crate) fn sim_action_rect(slot: usize) -> Rect {
    let x = screen_width() - right_panel_width() + 26.0;
    Rect::new(x + slot as f32 * 112.0, TOP_H + 126.0, 104.0, 34.0)
}

pub(crate) fn sim_object_mesh<'a>(
    app: &'a AppState,
    kind: SimObjectKind,
) -> Option<&'a RenderMesh> {
    let key = match kind {
        SimObjectKind::Player => SIM_PLAYER_DFF,
        SimObjectKind::Vehicle => SIM_VEHICLE_DFF,
    };
    app.meshes.get(&lower(key))
}

const SIM_PLAYER_RADIUS: f32 = 2.0;
const SIM_PLAYER_HEIGHT: f32 = 58.0;
const SIM_SLOPE_SNAP_UP: f32 = 96.0;
const SIM_PLAYER_CAMERA_DISTANCE: f32 = 45.0;
const SIM_PLAYER_CAMERA_HEIGHT: f32 = 30.0;
const SIM_VEHICLE_CAMERA_DISTANCE: f32 = 210.0;
const SIM_VEHICLE_CAMERA_HEIGHT: f32 = 95.0;
pub(crate) const SIM_PLAYER_VISUAL_SCALE: f32 = 4.0;
pub(crate) const SIM_PLAYER_VISUAL_ROT_OFFSET: f32 = -std::f32::consts::FRAC_PI_2;
pub(crate) const SIM_PLAYER_VISUAL_UPRIGHT_ROT: f32 = std::f32::consts::FRAC_PI_2;
pub(crate) const SIM_VEHICLE_VISUAL_SCALE: f32 = 1.0;
const SIM_GROUND_RAY_UP: f32 = SIM_SLOPE_SNAP_UP;
const SIM_GROUND_RAY_DOWN: f32 = 16000.0;

#[derive(Clone, Copy)]
struct SimCollisionHit {
    point: Vec3,
    normal: Vec3,
}

fn bounds_overlap(a_min: Vec3, a_max: Vec3, b_min: Vec3, b_max: Vec3) -> bool {
    a_min.x <= b_max.x
        && a_max.x >= b_min.x
        && a_min.y <= b_max.y
        && a_max.y >= b_min.y
        && a_min.z <= b_max.z
        && a_max.z >= b_min.z
}

fn closest_point_on_triangle(point: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let ab = b - a;
    let ac = c - a;
    let ap = point - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }

    let bp = point - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }

    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return a + ab * v;
    }

    let cp = point - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }

    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return a + ac * w;
    }

    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return b + (c - b) * w;
    }

    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    a + ab * v + ac * w
}

fn sim_collision_raycast(
    app: &AppState,
    origin: Vec3,
    dir: Vec3,
    max_dist: f32,
) -> Option<SimCollisionHit> {
    let dir = dir.try_normalize()?;
    let mut best: Option<SimCollisionHit> = None;
    let mut best_dist = max_dist;
    for (idx, placement) in app.placements.iter().enumerate() {
        if !is_visible_element(app, idx) || placement_is_app_lod(app, placement) {
            continue;
        }
        let Some(mesh) = element_collision_mesh(app, placement) else {
            continue;
        };
        let model = placement_matrix(placement);
        let model_cols = model.to_cols_array();
        let world_bounds = transformed_bounds(mesh.bounds, &model_cols);
        let ray_end = origin + dir * best_dist;
        let ray_min = origin.min(ray_end);
        let ray_max = origin.max(ray_end);
        if !bounds_overlap(ray_min, ray_max, world_bounds.min, world_bounds.max) {
            continue;
        }

        let inv = model.inverse().to_cols_array();
        let local_origin = transform_point_gl(&inv, from_mq(origin));
        let local_far = transform_point_gl(&inv, from_mq(origin + dir));
        let local_dir = local_far - local_origin;
        if local_dir.length_squared() < 0.0001 {
            continue;
        }
        let local_dir = local_dir.normalize();
        if ray_aabb(local_origin, local_dir, mesh.bounds.min, mesh.bounds.max).is_none() {
            continue;
        }
        for face in &mesh.faces {
            let Some((a, b, c)) = collision_face_points(mesh, face) else {
                continue;
            };
            let Some(local_t) = ray_triangle(local_origin, local_dir, a, b, c) else {
                continue;
            };
            let local_hit = local_origin + local_dir * local_t;
            let world_hit = transform_point_gl(&model_cols, from_mq(local_hit));
            let dist = world_hit.distance(origin);
            if dist >= best_dist {
                continue;
            }
            let mut normal = model
                .transform_vector3((b - a).cross(c - a))
                .normalize_or_zero();
            if normal.length_squared() < 0.0001 {
                normal = Vec3::Z;
            }
            if normal.dot(dir) > 0.0 {
                normal = -normal;
            }
            best_dist = dist;
            best = Some(SimCollisionHit {
                point: world_hit,
                normal,
            });
        }
    }
    best
}

fn sim_ground_hit(app: &AppState, pos: Vec3) -> Option<SimCollisionHit> {
    let origin = vec3(pos.x, pos.y, pos.z + SIM_GROUND_RAY_UP);
    sim_collision_raycast(app, origin, -Vec3::Z, SIM_GROUND_RAY_DOWN)
        .filter(|hit| hit.normal.z > 0.25)
}

pub(crate) fn sim_ground_z(app: &AppState, pos: Vec3) -> f32 {
    sim_ground_hit(app, pos).map_or(0.0, |hit| hit.point.z)
}

pub(crate) fn sim_place_point(app: &AppState, viewport: Rect, mouse: Vec2) -> Option<Vec3> {
    let (origin, dir) = viewport_ray(app, viewport, mouse)?;
    if let Some(hit) = sim_collision_raycast(app, origin, dir, 16000.0) {
        return Some(hit.point);
    }
    if dir.z.abs() < 0.0001 {
        return None;
    }
    let t = -origin.z / dir.z;
    if t <= 0.0 {
        return None;
    }
    Some(origin + dir * t)
}

fn resolve_sim_player_collisions(app: &AppState, mut pos: Vec3) -> Vec3 {
    let sample_offsets = [
        SIM_PLAYER_RADIUS,
        SIM_PLAYER_HEIGHT * 0.5,
        SIM_PLAYER_HEIGHT - SIM_PLAYER_RADIUS,
    ];
    for _ in 0..3 {
        let player_min = pos + vec3(-SIM_PLAYER_RADIUS, -SIM_PLAYER_RADIUS, 0.0);
        let player_max = pos + vec3(SIM_PLAYER_RADIUS, SIM_PLAYER_RADIUS, SIM_PLAYER_HEIGHT);
        let mut correction = Vec3::ZERO;
        let mut correction_len2 = 0.0f32;
        for (idx, placement) in app.placements.iter().enumerate() {
            if !is_visible_element(app, idx) || placement_is_app_lod(app, placement) {
                continue;
            }
            let Some(mesh) = element_collision_mesh(app, placement) else {
                continue;
            };
            let model = placement_matrix(placement);
            let model_cols = model.to_cols_array();
            let world_bounds = transformed_bounds(mesh.bounds, &model_cols);
            if !bounds_overlap(player_min, player_max, world_bounds.min, world_bounds.max) {
                continue;
            }
            for face in &mesh.faces {
                let Some((la, lb, lc)) = collision_face_points(mesh, face) else {
                    continue;
                };
                let a = transform_point_gl(&model_cols, from_mq(la));
                let b = transform_point_gl(&model_cols, from_mq(lb));
                let c = transform_point_gl(&model_cols, from_mq(lc));
                let tri_min = a.min(b).min(c);
                let tri_max = a.max(b).max(c);
                if !bounds_overlap(player_min, player_max, tri_min, tri_max) {
                    continue;
                }
                let normal = (b - a).cross(c - a).normalize_or_zero();
                if normal.length_squared() < 0.0001 || normal.z.abs() > 0.55 {
                    continue;
                }
                for z in sample_offsets {
                    let sample = pos + vec3(0.0, 0.0, z);
                    let closest = closest_point_on_triangle(sample, a, b, c);
                    let mut away = sample - closest;
                    away.z = 0.0;
                    let dist2 = away.length_squared();
                    if dist2 >= SIM_PLAYER_RADIUS * SIM_PLAYER_RADIUS {
                        continue;
                    }
                    let dir = if dist2 > 0.0001 {
                        away.normalize()
                    } else {
                        let horizontal = vec3(normal.x, normal.y, 0.0);
                        if horizontal.length_squared() > 0.0001 {
                            horizontal.normalize()
                        } else {
                            Vec3::ZERO
                        }
                    };
                    if dir.length_squared() > 0.0001 {
                        let candidate = dir * (SIM_PLAYER_RADIUS - dist2.sqrt());
                        let candidate_len2 = candidate.length_squared();
                        if candidate_len2 > correction_len2 {
                            correction = candidate;
                            correction_len2 = candidate_len2;
                        }
                    }
                }
            }
        }
        if correction.length_squared() < 0.0001 {
            break;
        }
        let limit = SIM_PLAYER_RADIUS * 0.5;
        pos += correction.clamp_length_max(limit);
    }
    pos
}

pub(crate) fn set_sim_player(app: &mut AppState, pos: Vec3) {
    let object = SimObject {
        kind: SimObjectKind::Player,
        pos,
        rot_z: 0.0,
    };
    if let Some(idx) = app
        .sim
        .player_index
        .filter(|idx| *idx < app.sim.objects.len())
    {
        app.sim.objects[idx] = object;
        app.sim.player_index = Some(idx);
    } else {
        app.sim.objects.push(object);
        app.sim.player_index = Some(app.sim.objects.len() - 1);
    }
    app.status_message = "Simulation player placed".to_string();
}

pub(crate) fn set_sim_vehicle(app: &mut AppState, pos: Vec3) {
    let object = SimObject {
        kind: SimObjectKind::Vehicle,
        pos,
        rot_z: 0.0,
    };
    if let Some(idx) = app
        .sim
        .vehicle_index
        .filter(|idx| *idx < app.sim.objects.len())
    {
        app.sim.objects[idx] = object;
        app.sim.vehicle_index = Some(idx);
    } else {
        app.sim.objects.push(object);
        app.sim.vehicle_index = Some(app.sim.objects.len() - 1);
    }
    app.status_message = "Simulation vehicle placed".to_string();
}

pub(crate) fn clear_simulation(app: &mut AppState) {
    app.sim.objects.clear();
    app.sim.player_index = None;
    app.sim.vehicle_index = None;
    app.sim.controlling_vehicle = None;
    app.sim.playing = false;
    restore_sim_editor_camera(app);
    app.sim.velocity = Vec3::ZERO;
    app.sim.vehicle_speed = 0.0;
    app.sim.on_ground = true;
    app.status_message = "Simulation cleared".to_string();
}

pub(crate) fn restore_sim_editor_camera(app: &mut AppState) {
    let Some(camera) = app.sim_editor_camera.take() else {
        return;
    };
    app.camera = camera;
    app.camera.looking = false;
    app.camera.last_mouse = mouse_position().into();
    set_cursor_grab(false);
    show_mouse(true);
}

pub(crate) fn toggle_sim_play(app: &mut AppState) {
    if app.sim.player_index.is_none() {
        let pos = app.camera.pos + camera_vectors(&app.camera).0 * 160.0;
        set_sim_player(app, vec3(pos.x, pos.y, sim_ground_z(app, pos)));
    }
    if !app.sim.playing && app.sim_editor_camera.is_none() {
        app.sim_editor_camera = Some(app.camera);
    }
    app.sim.playing = !app.sim.playing;
    if !app.sim.playing {
        restore_sim_editor_camera(app);
    }
    app.camera.looking = false;
    set_cursor_grab(false);
    show_mouse(true);
    app.status_message = if app.sim.playing {
        "Simulation playing".to_string()
    } else {
        "Simulation paused".to_string()
    };
}

pub(crate) fn handle_simulate_click(app: &mut AppState, viewport: Rect, mouse: Vec2) -> bool {
    if app.active_tab != AppTab::Simulate {
        return false;
    }
    if is_mouse_button_pressed(MouseButton::Left) {
        if sim_button_rect(0).contains(mouse) {
            app.sim.place_tool = SimPlaceTool::Player;
            return true;
        }
        if sim_button_rect(1).contains(mouse) {
            app.sim.place_tool = SimPlaceTool::Vehicle;
            return true;
        }
        if sim_action_rect(0).contains(mouse) {
            toggle_sim_play(app);
            return true;
        }
        if sim_action_rect(1).contains(mouse) {
            clear_simulation(app);
            return true;
        }
        if viewport.contains(mouse) && !app.sim.playing {
            if let Some(pos) = sim_place_point(app, viewport, mouse) {
                match app.sim.place_tool {
                    SimPlaceTool::Player => set_sim_player(app, pos),
                    SimPlaceTool::Vehicle => set_sim_vehicle(app, pos),
                }
                return true;
            }
        }
    }
    false
}

pub(crate) fn nearest_sim_vehicle(app: &AppState, pos: Vec3, max_dist: f32) -> Option<usize> {
    app.sim
        .objects
        .iter()
        .enumerate()
        .filter(|(_, object)| object.kind == SimObjectKind::Vehicle)
        .filter_map(|(idx, object)| {
            let dist = object.pos.distance(pos);
            (dist <= max_dist).then_some((idx, dist))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(idx, _)| idx)
}

pub(crate) fn update_simulation(app: &mut AppState) {
    if app.active_tab != AppTab::Simulate || !app.sim.playing {
        return;
    }
    let dt = get_frame_time().min(0.05);
    if is_key_pressed(KeyCode::Escape) {
        app.sim.playing = false;
        app.sim.controlling_vehicle = None;
        restore_sim_editor_camera(app);
        app.status_message = "Simulation paused".to_string();
        return;
    }

    if let Some(vehicle_idx) = app.sim.controlling_vehicle {
        if vehicle_idx >= app.sim.objects.len() {
            app.sim.controlling_vehicle = None;
            return;
        }
        if is_key_pressed(KeyCode::F) {
            if let Some(player_idx) = app.sim.player_index {
                let exit_pos = app.sim.objects[vehicle_idx].pos
                    + vec3(
                        app.sim.objects[vehicle_idx].rot_z.cos(),
                        app.sim.objects[vehicle_idx].rot_z.sin(),
                        0.0,
                    ) * 42.0;
                if let Some(player) = app.sim.objects.get_mut(player_idx) {
                    player.pos = exit_pos;
                }
            }
            app.sim.controlling_vehicle = None;
            app.sim.vehicle_speed = 0.0;
            app.status_message = "Exited vehicle".to_string();
            return;
        }
        let steer = if is_key_down(KeyCode::A) {
            1.0
        } else if is_key_down(KeyCode::D) {
            -1.0
        } else {
            0.0
        };
        let throttle = if is_key_down(KeyCode::W) {
            1.0
        } else if is_key_down(KeyCode::S) {
            -0.65
        } else {
            0.0
        };
        app.sim.vehicle_speed += throttle * 620.0 * dt;
        app.sim.vehicle_speed *= (1.0 - 1.8 * dt).clamp(0.0, 1.0);
        app.sim.vehicle_speed = app.sim.vehicle_speed.clamp(-260.0, 520.0);
        let turn_rate = steer * (1.2 + app.sim.vehicle_speed.abs() / 260.0) * dt;
        let (mut vehicle_pos, vehicle_rot) = {
            let vehicle = &mut app.sim.objects[vehicle_idx];
            vehicle.rot_z += turn_rate;
            let forward = vec3(vehicle.rot_z.sin(), vehicle.rot_z.cos(), 0.0);
            vehicle.pos += forward * app.sim.vehicle_speed * dt;
            (vehicle.pos, vehicle.rot_z)
        };
        vehicle_pos.z = sim_ground_z(app, vehicle_pos);
        if let Some(vehicle) = app.sim.objects.get_mut(vehicle_idx) {
            vehicle.pos.z = vehicle_pos.z;
        }
        if let Some(player_idx) = app.sim.player_index {
            if let Some(player) = app.sim.objects.get_mut(player_idx) {
                player.pos = vehicle_pos + vec3(0.0, 0.0, 24.0);
                player.rot_z = vehicle_rot;
            }
        }
        let forward = vec3(vehicle_rot.sin(), vehicle_rot.cos(), 0.0);
        app.camera.pos = vehicle_pos - forward * SIM_VEHICLE_CAMERA_DISTANCE
            + vec3(0.0, 0.0, SIM_VEHICLE_CAMERA_HEIGHT);
        app.camera.yaw = vehicle_rot;
        app.camera.pitch = -20.0_f32.to_radians();
        return;
    }

    let Some(player_idx) = app.sim.player_index else {
        return;
    };
    if player_idx >= app.sim.objects.len() {
        app.sim.player_index = None;
        return;
    }
    let player_pos = app.sim.objects[player_idx].pos;
    if is_key_pressed(KeyCode::F) {
        if let Some(vehicle_idx) = nearest_sim_vehicle(app, player_pos, 130.0) {
            app.sim.controlling_vehicle = Some(vehicle_idx);
            app.sim.vehicle_speed = 0.0;
            app.status_message = "Entered vehicle".to_string();
            return;
        }
    }

    let yaw = app.camera.yaw;
    let forward = vec3(yaw.sin(), yaw.cos(), 0.0).normalize_or_zero();
    let right = vec3(yaw.cos(), -yaw.sin(), 0.0).normalize_or_zero();
    let mut wish = Vec3::ZERO;
    if is_key_down(KeyCode::W) {
        wish += forward;
    }
    if is_key_down(KeyCode::S) {
        wish -= forward;
    }
    if is_key_down(KeyCode::D) {
        wish += right;
    }
    if is_key_down(KeyCode::A) {
        wish -= right;
    }
    if wish.length_squared() > 0.0001 {
        wish = wish.normalize();
    }
    let speed = if is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl) {
        24.0
    } else {
        70.0
    };
    let ground_z = sim_ground_z(app, player_pos);
    if app.sim.on_ground && is_key_pressed(KeyCode::Space) {
        app.sim.velocity.z = 360.0;
        app.sim.on_ground = false;
    }
    if is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift) {
        app.sim.velocity.z += 900.0 * dt;
        app.sim.velocity.z = app.sim.velocity.z.min(520.0);
        app.sim.on_ground = false;
    }
    app.sim.velocity.x = wish.x * speed;
    app.sim.velocity.y = wish.y * speed;
    app.sim.velocity.z -= 820.0 * dt;
    let mut new_pos = player_pos + app.sim.velocity * dt;
    new_pos = resolve_sim_player_collisions(app, new_pos);
    let new_ground_z = sim_ground_z(app, new_pos);
    let can_step_to_ground = app.sim.on_ground && new_ground_z <= new_pos.z + SIM_SLOPE_SNAP_UP;
    if new_pos.z <= new_ground_z || (can_step_to_ground && app.sim.velocity.z <= 0.0) {
        new_pos.z = new_ground_z;
        app.sim.velocity.z = 0.0;
        app.sim.on_ground = true;
    } else if new_pos.z > ground_z + SIM_SLOPE_SNAP_UP {
        app.sim.on_ground = false;
    }
    new_pos = resolve_sim_player_collisions(app, new_pos);
    let player = &mut app.sim.objects[player_idx];
    player.pos = new_pos;
    if wish.length_squared() > 0.0001 {
        player.rot_z = wish.x.atan2(wish.y);
    }
    let look_dir = vec3(app.camera.yaw.sin(), app.camera.yaw.cos(), 0.0);
    app.camera.pos = player.pos - look_dir * SIM_PLAYER_CAMERA_DISTANCE
        + vec3(0.0, 0.0, SIM_PLAYER_CAMERA_HEIGHT);
    app.camera.pitch = -14.0_f32.to_radians();
}

pub(crate) fn element_mesh<'a>(app: &'a AppState, placement: &Placement) -> Option<&'a RenderMesh> {
    app.meshes
        .get(&placement_mesh_key(placement, &app.definitions))
}

pub(crate) fn placement_collision_key_cow<'a>(
    definitions: &'a HashMap<String, Definition>,
    placement: &'a Placement,
) -> std::borrow::Cow<'a, str> {
    let col_name = definitions
        .get(&placement.id)
        .and_then(|def| def.attrs.get("col"))
        .filter(|value| !value.trim().is_empty())
        .map(String::as_str)
        .unwrap_or(placement.dff.as_str());
    let bytes = col_name.as_bytes();
    let has_col_extension = bytes
        .get(bytes.len().saturating_sub(4)..)
        .is_some_and(|suffix| suffix.eq_ignore_ascii_case(b".col"));
    let already_lowercase = !bytes.iter().any(u8::is_ascii_uppercase);
    if has_col_extension && already_lowercase {
        std::borrow::Cow::Borrowed(col_name)
    } else {
        std::borrow::Cow::Owned(lower(with_ext(col_name, ".col")))
    }
}

pub(crate) fn element_collision_key_cow<'a>(
    app: &'a AppState,
    placement: &'a Placement,
) -> std::borrow::Cow<'a, str> {
    placement_collision_key_cow(&app.definitions, placement)
}

pub(crate) fn element_collision_key(app: &AppState, placement: &Placement) -> String {
    element_collision_key_cow(app, placement).into_owned()
}

pub(crate) fn element_collision_mesh<'a>(
    app: &'a AppState,
    placement: &Placement,
) -> Option<&'a CollisionMesh> {
    if placement_disable_collisions(placement, &app.definitions) {
        return None;
    }
    app.collisions.get(&element_collision_key(app, placement))
}

#[cfg(test)]
mod collision_key_tests {
    use super::*;

    fn placement(dff: &str) -> Placement {
        Placement {
            id: "test".to_string(),
            dff: dff.to_string(),
            zone: String::new(),
            tag: "object".to_string(),
            attrs: BTreeMap::new(),
            pos: V3::default(),
            rot: V3::default(),
        }
    }

    #[test]
    fn normalized_collision_key_is_borrowed() {
        let placement = placement("building.col");
        let definitions = HashMap::new();
        let key = placement_collision_key_cow(&definitions, &placement);
        assert!(matches!(key, std::borrow::Cow::Borrowed("building.col")));
    }

    #[test]
    fn collision_key_normalization_allocates_only_when_needed() {
        let placement = placement("Building.DFF");
        let definitions = HashMap::new();
        let key = placement_collision_key_cow(&definitions, &placement);
        assert!(matches!(key, std::borrow::Cow::Owned(_)));
        assert_eq!(key, "building.dff.col");
    }
}

pub(crate) fn ray_aabb(origin: Vec3, dir: Vec3, min: Vec3, max: Vec3) -> Option<f32> {
    let mut t_min = 0.0f32;
    let mut t_max = f32::MAX;
    for (o, d, mn, mx) in [
        (origin.x, dir.x, min.x, max.x),
        (origin.y, dir.y, min.y, max.y),
        (origin.z, dir.z, min.z, max.z),
    ] {
        if d.abs() < 0.00001 {
            if o < mn || o > mx {
                return None;
            }
            continue;
        }
        let inv_d = 1.0 / d;
        let mut t1 = (mn - o) * inv_d;
        let mut t2 = (mx - o) * inv_d;
        if t1 > t2 {
            std::mem::swap(&mut t1, &mut t2);
        }
        t_min = t_min.max(t1);
        t_max = t_max.min(t2);
        if t_min > t_max {
            return None;
        }
    }
    Some(t_min.max(0.0))
}

pub(crate) fn ray_triangle(origin: Vec3, dir: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<f32> {
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
    if t > 0.0001 { Some(t) } else { None }
}

pub(crate) fn ray_mesh_triangles(origin: Vec3, dir: Vec3, mesh: &RenderMesh) -> Option<f32> {
    let mut best = f32::MAX;
    for part in &mesh.parts {
        for tri in part.cpu_vertices.chunks_exact(3) {
            let a = to_mq(tri[0].pos);
            let b = to_mq(tri[1].pos);
            let c = to_mq(tri[2].pos);
            if let Some(t) = ray_triangle(origin, dir, a, b, c) {
                best = best.min(t);
            }
        }
    }
    if best < f32::MAX { Some(best) } else { None }
}

pub(crate) fn collision_face_points(
    mesh: &CollisionMesh,
    face: &CollisionFace,
) -> Option<(Vec3, Vec3, Vec3)> {
    let a = mesh.vertices.get(face.a as usize).copied()?;
    let b = mesh.vertices.get(face.b as usize).copied()?;
    let c = mesh.vertices.get(face.c as usize).copied()?;
    Some((to_mq(a), to_mq(b), to_mq(c)))
}

pub(crate) fn ray_collision_triangles(
    origin: Vec3,
    dir: Vec3,
    mesh: &CollisionMesh,
) -> Option<(usize, f32)> {
    let mut best = None;
    let mut best_t = f32::MAX;
    for (idx, face) in mesh.faces.iter().enumerate() {
        let Some((a, b, c)) = collision_face_points(mesh, face) else {
            continue;
        };
        if let Some(t) = ray_triangle(origin, dir, a, b, c) {
            if t < best_t {
                best_t = t;
                best = Some(idx);
            }
        }
    }
    best.map(|idx| (idx, best_t))
}

pub(crate) fn viewport_ray(app: &AppState, viewport: Rect, mouse: Vec2) -> Option<(Vec3, Vec3)> {
    if !viewport.contains(mouse) {
        return None;
    }
    let (forward, _) = camera_vectors(&app.camera);
    let projection = Mat4::perspective_rh_gl(
        70.0_f32.to_radians(),
        viewport.w / viewport.h.max(1.0),
        1.0,
        16000.0,
    );
    let view = Mat4::look_at_rh(app.camera.pos, app.camera.pos + forward, Vec3::Z);
    let inv = (projection * view).inverse();
    let x = ((mouse.x - viewport.x) / viewport.w) * 2.0 - 1.0;
    let y = 1.0 - ((mouse.y - viewport.y) / viewport.h) * 2.0;
    let near = inv.project_point3(vec3(x, y, -1.0));
    let far = inv.project_point3(vec3(x, y, 1.0));
    let dir = far - near;
    if dir.length_squared() < 0.0001 {
        None
    } else {
        Some((near, dir.normalize()))
    }
}

pub(crate) fn view_projection(app: &AppState, viewport: Rect) -> Mat4 {
    let (forward, _) = camera_vectors(&app.camera);
    let projection = Mat4::perspective_rh_gl(
        70.0_f32.to_radians(),
        viewport.w / viewport.h.max(1.0),
        1.0,
        16000.0,
    );
    let view = Mat4::look_at_rh(app.camera.pos, app.camera.pos + forward, Vec3::Z);
    projection * view
}

pub(crate) fn world_to_screen(app: &AppState, viewport: Rect, world: Vec3) -> Option<Vec2> {
    let clip = view_projection(app, viewport) * vec4(world.x, world.y, world.z, 1.0);
    if clip.w.abs() < 0.0001 {
        return None;
    }
    let ndc = clip.truncate() / clip.w;
    if ndc.z < -1.0 || ndc.z > 1.0 {
        return None;
    }
    Some(vec2(
        viewport.x + (ndc.x + 1.0) * 0.5 * viewport.w,
        viewport.y + (1.0 - ndc.y) * 0.5 * viewport.h,
    ))
}

pub(crate) fn normalized_screen_rect(a: Vec2, b: Vec2) -> Rect {
    let x = a.x.min(b.x);
    let y = a.y.min(b.y);
    Rect::new(x, y, (a.x - b.x).abs(), (a.y - b.y).abs())
}

pub(crate) fn placement_world_bounds_points(placement: &Placement, bounds: Bounds) -> [Vec3; 9] {
    let m = placement_matrix(placement).to_cols_array();
    let corners = [
        vec3(bounds.min.x, bounds.min.y, bounds.min.z),
        vec3(bounds.max.x, bounds.min.y, bounds.min.z),
        vec3(bounds.min.x, bounds.max.y, bounds.min.z),
        vec3(bounds.max.x, bounds.max.y, bounds.min.z),
        vec3(bounds.min.x, bounds.min.y, bounds.max.z),
        vec3(bounds.max.x, bounds.min.y, bounds.max.z),
        vec3(bounds.min.x, bounds.max.y, bounds.max.z),
        vec3(bounds.max.x, bounds.max.y, bounds.max.z),
        (bounds.min + bounds.max) * 0.5,
    ];
    corners.map(|point| transform_point_gl(&m, from_mq(point)))
}

pub(crate) fn box_select_elements(
    app: &AppState,
    viewport: Rect,
    start: Vec2,
    end: Vec2,
) -> BTreeSet<usize> {
    let rect = normalized_screen_rect(start, end);
    let mut selected = BTreeSet::new();
    if rect.w < 4.0 || rect.h < 4.0 {
        return selected;
    }
    let intersects = |a: Rect, b: Rect| {
        a.x <= b.x + b.w && a.x + a.w >= b.x && a.y <= b.y + b.h && a.y + a.h >= b.y
    };
    let max_dist2 = app.box_select_distance.max(1.0).powi(2);
    for (idx, placement) in app.placements.iter().enumerate() {
        if !is_visible_element(app, idx)
            || (!app.lod_selectable && placement_is_app_lod(app, placement))
        {
            continue;
        }
        let Some(mesh) = element_mesh(app, placement) else {
            continue;
        };
        let points = placement_world_bounds_points(placement, mesh.bounds);
        if !points
            .iter()
            .any(|point| point.distance_squared(app.camera.pos) <= max_dist2)
        {
            continue;
        }
        let screens = points
            .iter()
            .filter_map(|point| world_to_screen(app, viewport, *point))
            .collect::<Vec<_>>();
        if screens.is_empty() {
            continue;
        }
        let mut min = screens[0];
        let mut max = screens[0];
        for screen in screens.iter().skip(1) {
            min = min.min(*screen);
            max = max.max(*screen);
        }
        let asset_rect = Rect::new(
            min.x,
            min.y,
            (max.x - min.x).max(1.0),
            (max.y - min.y).max(1.0),
        );
        if intersects(rect, asset_rect) {
            selected.insert(idx);
        }
    }
    selected
}

pub(crate) fn axis_vector(axis: GizmoAxis) -> Vec3 {
    match axis {
        GizmoAxis::X => Vec3::X,
        GizmoAxis::Y => Vec3::Y,
        GizmoAxis::Z => Vec3::Z,
    }
}

pub(crate) fn light_local_basis(direction: V3) -> (Vec3, Vec3, Vec3) {
    let mut z = vec3(direction.x, direction.y, direction.z);
    if z.length_squared() < 0.0001 {
        z = Vec3::NEG_Z;
    }
    z = z.normalize();
    let up_hint = if z.z.abs() < 0.92 { Vec3::Z } else { Vec3::Y };
    let mut x = up_hint.cross(z);
    if x.length_squared() < 0.0001 {
        x = Vec3::X;
    } else {
        x = x.normalize();
    }
    let y = z.cross(x).normalize_or_zero();
    (x, y, z)
}

pub(crate) fn light_local_axis(direction: V3, axis: GizmoAxis) -> Vec3 {
    let (x, y, z) = light_local_basis(direction);
    match axis {
        GizmoAxis::X => x,
        GizmoAxis::Y => y,
        GizmoAxis::Z => z,
    }
}

pub(crate) fn placement_rotation_matrix(p: &Placement) -> Mat4 {
    Mat4::from_rotation_z(p.rot.z.to_radians())
        * Mat4::from_rotation_y(p.rot.y.to_radians())
        * Mat4::from_rotation_x(p.rot.x.to_radians())
}

pub(crate) fn transform_vec(m: Mat4, v: Vec3) -> Vec3 {
    let cols = m.to_cols_array();
    let out = transform_normal_gl(&cols, from_mq(v));
    if out.length_squared() > 0.0001 {
        out.normalize()
    } else {
        v
    }
}

fn selected_editing_cuboid_rotation(app: &AppState) -> Option<Mat4> {
    let EditingAsset::Col(col) = app.editing.asset.as_ref()? else {
        return None;
    };
    let selected = col
        .selected_primitive
        .filter(|selected| selected.kind == CollisionPrimitiveKind::Cuboid)?;
    let rotation = col.cuboids.get(selected.index)?.rotation;
    Some(
        Mat4::from_rotation_z(rotation.z.to_radians())
            * Mat4::from_rotation_y(rotation.y.to_radians())
            * Mat4::from_rotation_x(rotation.x.to_radians()),
    )
}

fn selected_editing_dff_2dfx_rotation_matrix(app: &AppState) -> Option<Mat4> {
    selected_editing_dff_2dfx_rotation(app).map(dff_2dfx_rotation_matrix)
}

pub(crate) fn selected_axis_vector(app: &AppState, axis: GizmoAxis) -> Vec3 {
    let base = axis_vector(axis);
    // Cull volumes are always axis-aligned, so Local and World space are
    // intentionally identical for their translation handles.
    if app.active_tab == AppTab::Cull {
        return base;
    }
    if app.active_tab == AppTab::Lights {
        if app.transform_space == TransformSpace::Local {
            if let Some(light) = app.lights.get(app.selected_light) {
                let (_, direction) = light_world_transform(
                    light,
                    light_reference_placement(app, light).map(|(_, placement)| placement),
                );
                return light_local_axis(from_mq(direction), axis);
            }
        }
        return base;
    }
    // A freeform pivot gimbal represents the pivot's authored basis itself.
    // Its handles must rotate with that basis even if some earlier editor
    // state left the shared transform-space toggle on World.
    if app.active_tab == AppTab::Editing
        && let Some(pivot) = selected_editing_dff_freeform_pivot(app)
    {
        return transform_vec(dff_pivot_rotation_matrix(pivot.rotation), base);
    }
    if app.transform_space == TransformSpace::Local {
        if app.active_tab == AppTab::Editing {
            if let Some(rotation) = selected_editing_dff_2dfx_rotation_matrix(app) {
                return transform_vec(rotation, base);
            }
            if let Some(rotation) = selected_editing_cuboid_rotation(app) {
                return transform_vec(rotation, base);
            }
        }
        if app.active_tab != AppTab::Race
            && selected_live_indices(app).len() <= 1
            && let Some(placement) = app.placements.get(app.selected)
        {
            return transform_vec(placement_rotation_matrix(placement), base);
        }
    }
    base
}

pub(crate) fn selected_ring_basis(app: &AppState, axis: GizmoAxis) -> (Vec3, Vec3) {
    let (a, b) = match axis {
        GizmoAxis::X => (Vec3::Y, Vec3::Z),
        GizmoAxis::Y => (Vec3::X, Vec3::Z),
        GizmoAxis::Z => (Vec3::X, Vec3::Y),
    };
    if app.active_tab == AppTab::Lights && app.transform_space == TransformSpace::Local {
        if let Some(light) = app.lights.get(app.selected_light) {
            let (_, direction) = light_world_transform(
                light,
                light_reference_placement(app, light).map(|(_, placement)| placement),
            );
            let (x, y, z) = light_local_basis(from_mq(direction));
            return match axis {
                GizmoAxis::X => (y, z),
                GizmoAxis::Y => (x, z),
                GizmoAxis::Z => (x, y),
            };
        }
    }
    if app.active_tab == AppTab::Editing
        && let Some(pivot) = selected_editing_dff_freeform_pivot(app)
    {
        let rotation = dff_pivot_rotation_matrix(pivot.rotation);
        return (transform_vec(rotation, a), transform_vec(rotation, b));
    }
    if app.transform_space == TransformSpace::Local {
        if app.active_tab == AppTab::Editing {
            if let Some(rotation) = selected_editing_dff_2dfx_rotation_matrix(app) {
                return (transform_vec(rotation, a), transform_vec(rotation, b));
            }
            if let Some(rotation) = selected_editing_cuboid_rotation(app) {
                return (transform_vec(rotation, a), transform_vec(rotation, b));
            }
        }
        if app.active_tab != AppTab::Lights
            && selected_live_indices(app).len() <= 1
            && let Some(placement) = app.placements.get(app.selected)
        {
            let rot = placement_rotation_matrix(placement);
            return (transform_vec(rot, a), transform_vec(rot, b));
        }
    }
    (a, b)
}

pub(crate) fn axis_color(axis: GizmoAxis) -> [f32; 4] {
    match axis {
        GizmoAxis::X => [1.0, 0.18, 0.14, 1.0],
        GizmoAxis::Y => [0.20, 0.90, 0.24, 1.0],
        GizmoAxis::Z => [0.25, 0.55, 1.0, 1.0],
    }
}

pub(crate) fn glow_color(color: [f32; 4]) -> [f32; 4] {
    [
        (color[0] + 0.35).min(1.0),
        (color[1] + 0.35).min(1.0),
        (color[2] + 0.35).min(1.0),
        1.0,
    ]
}

pub(crate) fn dist_to_segment(point: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let len2 = ab.length_squared();
    if len2 < 0.0001 {
        return point.distance(a);
    }
    let t = ((point - a).dot(ab) / len2).clamp(0.0, 1.0);
    point.distance(a + ab * t)
}

/// User-tunable multiplier for gimbal/gizmo size. Callers that apply their own
/// clamps must scale those bounds by this so the preference is not clamped away.
pub(crate) fn gizmo_scale(app: &AppState) -> f32 {
    clamp_gizmo_scale(app.gizmo_scale)
}

pub(crate) fn gizmo_visual_length(app: &AppState, origin: Vec3) -> f32 {
    let distance = (origin - app.camera.pos).length().max(80.0);
    let length = if app.active_tab == AppTab::Lights {
        // Light handles are frequently manipulated against an empty sky or
        // at long range, so give them a substantially larger screen presence.
        (distance * 0.006).clamp(5.0, 14.0)
    } else {
        (distance * 0.003).clamp(2.0, 7.0)
    };
    length * gizmo_scale(app)
}

pub(crate) fn selected_origin(app: &AppState) -> Option<Vec3> {
    if app.transform_mode == TransformMode::Scale
        && !(app.active_tab == AppTab::Editing
            && matches!(app.editing.asset, Some(EditingAsset::Dff(_))))
    {
        return None;
    }
    if app.active_tab == AppTab::Cull {
        if app.transform_mode == TransformMode::Rotate {
            return None;
        }
        return selected_cull_zone(app)
            .map(|zone| vec3(zone.center.x, zone.center.y, zone.center.z));
    }
    if app.active_tab == AppTab::Lights {
        return app.lights.get(app.selected_light).map(|light| {
            light_world_transform(
                light,
                light_reference_placement(app, light).map(|(_, placement)| placement),
            )
            .0
        });
    }
    if app.active_tab == AppTab::Collisions && app.collision_edit_mode {
        return selected_collision_tab_vertex_position(app);
    }
    if app.active_tab == AppTab::Race {
        return selected_race_point_position(app);
    }
    if app.active_tab == AppTab::Editing {
        match app.editing.asset.as_ref() {
            Some(EditingAsset::Col(col)) => {
                if col.selected_primitive.is_some() {
                    return selected_editing_col_primitive_position(app);
                }
                return selected_editing_col_vertex_position(app);
            }
            Some(EditingAsset::Dff(dff)) => {
                if let Some(pivot) = dff.freeform_pivot {
                    return (app.transform_mode != TransformMode::Scale)
                        .then_some(to_mq(pivot.position));
                }
                if app.transform_mode == TransformMode::Scale {
                    return selected_editing_dff_vertex_position(app);
                }
                if dff.boolean_box.is_some() {
                    return selected_editing_dff_boolean_box_position(app);
                }
                if dff.selected_2dfx.is_some() {
                    if app.transform_mode == TransformMode::Rotate
                        && selected_editing_dff_2dfx_rotation(app).is_none()
                    {
                        return None;
                    }
                    return selected_editing_dff_2dfx_position(app);
                }
                return selected_editing_dff_vertex_position(app);
            }
            _ => {}
        }
    }
    selection_origin(app)
}

fn gizmo_axis_at_threshold(
    app: &AppState,
    viewport: Rect,
    mouse: Vec2,
    threshold: f32,
) -> Option<GizmoAxis> {
    let origin = selected_origin(app)?;
    let length = gizmo_visual_length(app, origin);
    let origin_2d = world_to_screen(app, viewport, origin)?;
    let mut best = None;
    let mut best_dist = threshold;
    match app.transform_mode {
        TransformMode::Select => {}
        TransformMode::Move | TransformMode::Scale => {
            for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
                let axis_dir = selected_axis_vector(app, axis);
                let mut prev = Some(origin_2d);
                for i in 1..=16 {
                    let p = origin + axis_dir * length * (i as f32 / 16.0);
                    let Some(screen) = world_to_screen(app, viewport, p) else {
                        prev = None;
                        continue;
                    };
                    if let Some(prev_screen) = prev {
                        let dist = dist_to_segment(mouse, prev_screen, screen);
                        if dist < best_dist {
                            best_dist = dist;
                            best = Some(axis);
                        }
                    }
                    prev = Some(screen);
                }
                let end = origin + axis_dir * length;
                if let Some(end_2d) = world_to_screen(app, viewport, end) {
                    let dist = mouse.distance(end_2d);
                    if dist < best_dist + 4.0 && dist < threshold + 4.0 {
                        best_dist = dist;
                        best = Some(axis);
                    }
                }
            }
        }
        TransformMode::Rotate => {
            for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
                let (a, b) = selected_ring_basis(app, axis);
                let mut prev = None;
                for i in 0..=96 {
                    let t = i as f32 / 96.0 * std::f32::consts::TAU;
                    let p = origin + a * t.cos() * length * 0.75 + b * t.sin() * length * 0.75;
                    let Some(screen) = world_to_screen(app, viewport, p) else {
                        prev = None;
                        continue;
                    };
                    if let Some(prev_screen) = prev {
                        let dist = dist_to_segment(mouse, prev_screen, screen);
                        if dist < best_dist {
                            best_dist = dist;
                            best = Some(axis);
                        }
                    }
                    prev = Some(screen);
                }
            }
        }
    }
    best
}

pub(crate) fn gizmo_plane_axes(plane: GizmoPlane) -> (GizmoAxis, GizmoAxis) {
    match plane {
        GizmoPlane::XY => (GizmoAxis::X, GizmoAxis::Y),
        GizmoPlane::XZ => (GizmoAxis::X, GizmoAxis::Z),
        GizmoPlane::YZ => (GizmoAxis::Y, GizmoAxis::Z),
    }
}

pub(crate) fn gizmo_scale_plane_at(
    app: &AppState,
    viewport: Rect,
    mouse: Vec2,
) -> Option<GizmoPlane> {
    if app.transform_mode != TransformMode::Scale || app.active_tab != AppTab::Editing {
        return None;
    }
    let origin = selected_origin(app)?;
    let length = gizmo_visual_length(app, origin);
    let mut best = None;
    let mut best_distance = 16.0;
    for plane in [GizmoPlane::XY, GizmoPlane::XZ, GizmoPlane::YZ] {
        let (first, second) = gizmo_plane_axes(plane);
        let center = origin
            + (selected_axis_vector(app, first) + selected_axis_vector(app, second))
                * length
                * 0.28;
        if let Some(screen) = world_to_screen(app, viewport, center) {
            let distance = mouse.distance(screen);
            if distance < best_distance {
                best_distance = distance;
                best = Some(plane);
            }
        }
    }
    best
}

pub(crate) fn gizmo_axis_at(app: &AppState, viewport: Rect, mouse: Vec2) -> Option<GizmoAxis> {
    let threshold = if app.active_tab == AppTab::Lights {
        26.0
    } else {
        18.0
    };
    gizmo_axis_at_threshold(app, viewport, mouse, threshold)
}

pub(crate) fn gizmo_axis_direct_at(
    app: &AppState,
    viewport: Rect,
    mouse: Vec2,
) -> Option<GizmoAxis> {
    gizmo_axis_at_threshold(app, viewport, mouse, 14.0)
}

pub(crate) fn axis_drag_amount(
    app: &AppState,
    viewport: Rect,
    axis_origin: Vec3,
    axis_dir: Vec3,
    start_mouse: Vec2,
    mouse: Vec2,
) -> f32 {
    let Some(axis_dir) = axis_dir.try_normalize() else {
        return 0.0;
    };
    let Some((start_ray_origin, start_ray_dir)) = viewport_ray(app, viewport, start_mouse) else {
        return 0.0;
    };
    let Some((ray_origin, ray_dir)) = viewport_ray(app, viewport, mouse) else {
        return 0.0;
    };
    let Some(start_t) = ray_axis_parameter(axis_origin, axis_dir, start_ray_origin, start_ray_dir)
    else {
        return 0.0;
    };
    let Some(current_t) = ray_axis_parameter(axis_origin, axis_dir, ray_origin, ray_dir) else {
        return 0.0;
    };
    current_t - start_t
}

pub(crate) fn ray_axis_parameter(
    axis_origin: Vec3,
    axis_dir: Vec3,
    ray_origin: Vec3,
    ray_dir: Vec3,
) -> Option<f32> {
    let u = axis_dir.try_normalize()?;
    let v = ray_dir.try_normalize()?;
    let w0 = axis_origin - ray_origin;
    let b = u.dot(v);
    let d = u.dot(w0);
    let e = v.dot(w0);
    let denom = 1.0 - b * b;
    if denom.abs() < 0.0001 {
        None
    } else {
        Some((b * e - d) / denom)
    }
}

pub(crate) fn snap_delta(value: f32, step: f32) -> f32 {
    if step <= 0.0001 {
        value
    } else {
        (value / step).round() * step
    }
}

pub(crate) fn screen_angle_delta_degrees(center: Vec2, start_mouse: Vec2, mouse: Vec2) -> f32 {
    let start = start_mouse - center;
    let current = mouse - center;
    if start.length_squared() < 0.0001 || current.length_squared() < 0.0001 {
        return 0.0;
    }
    let start_angle = start.y.atan2(start.x);
    let current_angle = current.y.atan2(current.x);
    let mut delta = current_angle - start_angle;
    while delta > std::f32::consts::PI {
        delta -= std::f32::consts::TAU;
    }
    while delta < -std::f32::consts::PI {
        delta += std::f32::consts::TAU;
    }
    delta.to_degrees()
}

pub(crate) fn ring_drag_degrees(
    app: &AppState,
    viewport: Rect,
    start_mouse: Vec2,
    mouse: Vec2,
) -> f32 {
    let Some(origin) = selected_origin(app) else {
        return 0.0;
    };
    let Some(center) = world_to_screen(app, viewport, origin) else {
        return 0.0;
    };
    screen_angle_delta_degrees(center, start_mouse, mouse)
}

pub(crate) fn pick_scene_element(app: &AppState, viewport: Rect, mouse: Vec2) -> Option<usize> {
    let (origin, dir) = viewport_ray(app, viewport, mouse)?;
    let mut best = None;
    let mut best_t = f32::MAX;
    for (idx, state) in app.element_states.iter().enumerate() {
        if state.deleted || state.hidden {
            continue;
        }
        let Some(placement) = app.placements.get(idx) else {
            continue;
        };
        if !app.lod_selectable && placement_is_app_lod(app, placement) {
            continue;
        }
        let Some(mesh) = element_mesh(app, placement) else {
            continue;
        };
        let inv = placement_matrix(placement).inverse().to_cols_array();
        let local_origin = transform_point_gl(&inv, from_mq(origin));
        let local_far = transform_point_gl(&inv, from_mq(origin + dir));
        let local_dir = local_far - local_origin;
        if local_dir.length_squared() < 0.0001 {
            continue;
        }
        let local_dir = local_dir.normalize();
        if ray_aabb(local_origin, local_dir, mesh.bounds.min, mesh.bounds.max).is_none() {
            continue;
        }
        if let Some(t) = ray_mesh_triangles(local_origin, local_dir, mesh) {
            if t < best_t {
                best_t = t;
                best = Some(idx);
            }
        }
    }
    best
}

/// Returns the closest exact geometry hit under the pointer, with its placement.
pub(crate) fn pick_scene_geometry_point(
    app: &AppState,
    viewport: Rect,
    mouse: Vec2,
) -> Option<(usize, Vec3)> {
    let (origin, dir) = viewport_ray(app, viewport, mouse)?;
    trace_scene_geometry(app, origin, dir, f32::MAX)
}

/// Trace visible scene triangles, returning the nearest hit in world space.
pub(crate) fn trace_scene_geometry(
    app: &AppState,
    origin: Vec3,
    dir: Vec3,
    max_distance: f32,
) -> Option<(usize, Vec3)> {
    let dir = dir.try_normalize()?;
    let mut best: Option<(usize, Vec3, f32)> = None;
    for (idx, state) in app.element_states.iter().enumerate() {
        if state.deleted || state.hidden {
            continue;
        }
        let Some(placement) = app.placements.get(idx) else {
            continue;
        };
        let Some(mesh) = element_mesh(app, placement) else {
            continue;
        };
        let model = placement_matrix(placement);
        let inv = model.inverse().to_cols_array();
        let local_origin = transform_point_gl(&inv, from_mq(origin));
        let local_far = transform_point_gl(&inv, from_mq(origin + dir));
        let local_dir = (local_far - local_origin).normalize_or_zero();
        if local_dir.length_squared() < 0.0001
            || ray_aabb(local_origin, local_dir, mesh.bounds.min, mesh.bounds.max).is_none()
        {
            continue;
        }
        let Some(local_t) = ray_mesh_triangles(local_origin, local_dir, mesh) else {
            continue;
        };
        let world = model.transform_point3(local_origin + local_dir * local_t);
        let world_t = (world - origin).dot(dir);
        if world_t >= 0.0
            && world_t <= max_distance
            && best.as_ref().is_none_or(|(_, _, t)| world_t < *t)
        {
            best = Some((idx, world, world_t));
        }
    }
    best.map(|(idx, point, _)| (idx, point))
}

pub(crate) fn light_reference_placement<'a>(
    app: &'a AppState,
    light: &EditorLight,
) -> Option<(usize, &'a Placement)> {
    let model = light.attached_to.as_deref()?;
    let usable = |idx: usize, placement: &Placement| {
        placement.id == model
            && !app
                .element_states
                .get(idx)
                .is_some_and(|state| state.deleted)
    };
    app.placements
        .get(app.selected)
        .filter(|placement| usable(app.selected, placement))
        .map(|placement| (app.selected, placement))
        .or_else(|| {
            app.placements
                .iter()
                .enumerate()
                .find(|(idx, placement)| usable(*idx, placement))
        })
}

pub(crate) fn light_world_transform(
    light: &EditorLight,
    placement: Option<&Placement>,
) -> (Vec3, Vec3) {
    let position = to_mq(light.position);
    let direction = to_mq(light.direction);
    if light.attached_to.is_some() {
        if let Some(placement) = placement {
            let model = placement_matrix(placement);
            return (
                model.transform_point3(position),
                model.transform_vector3(direction).normalize_or_zero(),
            );
        }
    }
    (position, direction)
}

/// Expands shared model-local lights into transient world-space occurrences.
pub(crate) fn expanded_scene_lights(app: &AppState) -> Vec<(usize, Option<usize>, EditorLight)> {
    let mut expanded = Vec::new();
    for (light_idx, light) in app.lights.iter().enumerate() {
        if let Some(model) = light.attached_to.as_deref() {
            for (placement_idx, placement) in app.placements.iter().enumerate() {
                if placement.id != model
                    || app
                        .element_states
                        .get(placement_idx)
                        .is_some_and(|state| state.deleted)
                {
                    continue;
                }
                let (position, direction) = light_world_transform(light, Some(placement));
                let mut world = light.clone();
                world.position = from_mq(position);
                world.direction = from_mq(direction);
                expanded.push((light_idx, Some(placement_idx), world));
            }
        } else {
            expanded.push((light_idx, None, light.clone()));
        }
    }
    expanded
}

pub(crate) fn pick_scene_light(
    app: &AppState,
    viewport: Rect,
    mouse: Vec2,
) -> Option<(usize, Option<usize>)> {
    let mut best = None;
    let mut best_dist = 16.0f32;
    for (idx, placement, light) in expanded_scene_lights(app) {
        let pos = vec3(light.position.x, light.position.y, light.position.z);
        let Some(screen) = world_to_screen(app, viewport, pos) else {
            continue;
        };
        let dist = screen.distance(mouse);
        if dist < best_dist {
            best_dist = dist;
            best = Some((idx, placement));
        }
    }
    best
}

pub(crate) fn pick_collision_face(
    app: &AppState,
    viewport: Rect,
    mouse: Vec2,
) -> Option<SelectedCollisionFace> {
    let (origin, dir) = viewport_ray(app, viewport, mouse)?;
    let mut best = None;
    let mut best_t = f32::MAX;
    for (idx, state) in app.element_states.iter().enumerate() {
        if state.deleted || state.hidden {
            continue;
        }
        let Some(placement) = app.placements.get(idx) else {
            continue;
        };
        if !app.lod_selectable && placement_is_app_lod(app, placement) {
            continue;
        }
        let Some(mesh) = element_collision_mesh(app, placement) else {
            continue;
        };
        let inv = placement_matrix(placement).inverse().to_cols_array();
        let local_origin = transform_point_gl(&inv, from_mq(origin));
        let local_far = transform_point_gl(&inv, from_mq(origin + dir));
        let local_dir = local_far - local_origin;
        if local_dir.length_squared() < 0.0001 {
            continue;
        }
        let local_dir = local_dir.normalize();
        if ray_aabb(local_origin, local_dir, mesh.bounds.min, mesh.bounds.max).is_none() {
            continue;
        }
        if let Some((face, t)) = ray_collision_triangles(local_origin, local_dir, mesh) {
            if t < best_t {
                best_t = t;
                best = Some(SelectedCollisionFace {
                    placement: idx,
                    face,
                });
            }
        }
    }
    best
}

pub(crate) fn select_element(app: &mut AppState, index: usize) {
    if index < app.placements.len() {
        app.selected = index;
        app.selected_elements.clear();
        app.selected_elements.insert(index);
        app.selected_element_order.clear();
        app.selected_element_order.push(index);
        app.selected_group = None;
        if app
            .selected_col_face
            .is_some_and(|face| face.placement != index)
        {
            app.selected_col_face = None;
        }
    }
}

pub(crate) fn select_asset_group(app: &mut AppState, group: &str) {
    let mut selected = BTreeSet::new();
    for (idx, placement) in app.placements.iter().enumerate() {
        if !is_live_element(app, idx) {
            continue;
        }
        if placement_group(placement) == Some(group) {
            selected.insert(idx);
        }
    }
    if let Some(primary) = selected.iter().next_back().copied() {
        app.selected = primary;
        app.selected_elements = selected;
        app.selected_element_order = app.selected_elements.iter().copied().collect();
        app.selected_group = Some(group.to_string());
        app.selected_col_face = None;
        app.status_message = format!(
            "Selected group {group} ({} assets)",
            app.selected_elements.len()
        );
    }
}

pub(crate) fn toggle_asset_group_expanded(app: &mut AppState, group: &str) {
    if app.expanded_groups.contains(group) {
        app.expanded_groups.remove(group);
    } else {
        app.expanded_groups.insert(group.to_string());
    }
    rebuild_outliner_filter(app);
}

pub(crate) fn deselect_element(app: &mut AppState) {
    app.selected = NO_SELECTION;
    app.selected_elements.clear();
    app.selected_element_order.clear();
    app.selected_group = None;
    app.hovered_gizmo = None;
    app.selected_col_face = None;
}

pub(crate) fn start_group_rename(app: &mut AppState, group: &str) {
    app.group_rename = Some(GroupRenameEdit {
        original: group.to_string(),
        buffer: group.to_string(),
        cursor: group.len(),
        selection_anchor: None,
        before: local_world_history_snapshot(
            app,
            app.placements
                .iter()
                .enumerate()
                .filter(|(_, p)| {
                    !is_default_world_placement(p) && placement_group(p) == Some(group)
                })
                .map(|(index, _)| index),
            [],
        ),
    });
    app.outliner_search_active = false;
    drain_text_input();
    app.status_message = format!("Renaming group {group}");
}

pub(crate) fn cancel_group_rename(app: &mut AppState) {
    if let Some(edit) = app.group_rename.take() {
        app.status_message = format!("Cancelled rename for {}", edit.original);
    }
}

pub(crate) fn apply_group_rename(app: &mut AppState) {
    let Some(edit) = app.group_rename.take() else {
        return;
    };
    let new_name = edit.buffer.trim().to_string();
    if new_name.is_empty() {
        app.status_message = "Group name cannot be empty".to_string();
        return;
    }
    if new_name == edit.original {
        app.status_message = format!("Group name unchanged: {new_name}");
        return;
    }
    let mut changed = 0usize;
    for placement in &mut app.placements {
        if is_default_world_placement(placement) {
            continue;
        }
        if !is_default_world_placement(placement)
            && placement_group(placement) == Some(edit.original.as_str())
        {
            placement
                .attrs
                .insert(EDITOR_GROUP_ATTR.to_string(), new_name.clone());
            changed += 1;
        }
    }
    if changed == 0 {
        app.status_message = format!("Group {} no longer exists", edit.original);
        return;
    }
    if app.selected_group.as_deref() == Some(edit.original.as_str()) {
        app.selected_group = Some(new_name.clone());
    }
    if app.expanded_groups.remove(&edit.original) {
        app.expanded_groups.insert(new_name.clone());
    }
    commit_local_world_history(app, "Rename Group", edit.before);
    app.status_message = format!("Renamed group to {new_name} ({changed} assets)");
}

pub(crate) fn next_asset_group_name(app: &AppState) -> String {
    let existing: BTreeSet<_> = asset_group_names(&app.placements).into_iter().collect();
    for n in 1.. {
        let name = format!("Group {n}");
        if !existing.contains(&name) {
            return name;
        }
    }
    "Group".to_string()
}

pub(crate) fn assign_selected_to_group(app: &mut AppState) {
    let indices = selected_editable_indices(app);
    if indices.is_empty() {
        app.status_message = "No assets selected for grouping".to_string();
        return;
    }
    let before = local_world_history_snapshot(app, indices.iter().copied(), []);
    let group = app
        .selected_group
        .clone()
        .or_else(|| {
            indices
                .iter()
                .filter_map(|idx| app.placements.get(*idx).and_then(placement_group_owned))
                .next()
        })
        .unwrap_or_else(|| next_asset_group_name(app));
    for idx in &indices {
        if let Some(placement) = app.placements.get_mut(*idx) {
            placement
                .attrs
                .insert(EDITOR_GROUP_ATTR.to_string(), group.clone());
        }
    }
    app.selected_group = Some(group.clone());
    commit_local_world_history(app, "Assign Group", before);
    app.status_message = format!("Assigned {} asset(s) to {group}", indices.len());
}

pub(crate) fn clear_selected_group(app: &mut AppState) {
    let indices = selected_editable_indices(app);
    if indices.is_empty() {
        app.status_message = "No assets selected".to_string();
        return;
    }
    let before = local_world_history_snapshot(app, indices.iter().copied(), []);
    let mut changed = 0usize;
    for idx in &indices {
        if let Some(placement) = app.placements.get_mut(*idx) {
            if placement.attrs.remove(EDITOR_GROUP_ATTR).is_some() {
                changed += 1;
            }
        }
    }
    app.selected_group = None;
    if changed > 0 {
        commit_local_world_history(app, "Clear Group", before);
    }
    app.status_message = format!("Cleared group from {changed} asset(s)");
}

pub(crate) fn selected_placement(app: &AppState) -> Option<&Placement> {
    app.placements.get(app.selected)
}

pub(crate) fn selected_light(app: &AppState) -> Option<&EditorLight> {
    app.lights.get(app.selected_light)
}

pub(crate) fn set_system_clipboard(value: &str) -> Result<(), String> {
    // Miniquad only offers UTF8_STRING on Wayland, while most native clients
    // request one of the standard text/plain MIME types. Arboard advertises the
    // complete text set and falls back to X11 when data-control is unavailable.
    //
    // Keep the handle alive: on Linux the process that owns a clipboard value
    // must continue serving it until another owner replaces the selection.
    static CLIPBOARD: OnceLock<Mutex<Option<arboard::Clipboard>>> = OnceLock::new();
    let clipboard = CLIPBOARD.get_or_init(|| Mutex::new(None));
    let mut clipboard = clipboard
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if clipboard.is_none() {
        *clipboard =
            Some(arboard::Clipboard::new().map_err(|err| format!("Clipboard unavailable: {err}"))?);
    }
    let result = clipboard
        .as_mut()
        .expect("clipboard initialized")
        .set_text(value.to_owned())
        .map_err(|err| format!("Could not store clipboard text: {err}"));
    if result.is_err() {
        // Recreate the platform connection on the next attempt.
        *clipboard = None;
    }
    result
}

pub(crate) fn read_system_clipboard_text() -> Result<String, String> {
    // This may wait for an external X11/Wayland clipboard owner, so callers
    // must run it on a worker. Arboard has a bounded X11 timeout; mini/quad's
    // fallback is intentionally not used because it can wait indefinitely.
    let mut clipboard =
        arboard::Clipboard::new().map_err(|err| format!("Clipboard unavailable: {err}"))?;
    clipboard
        .get_text()
        .map_err(|err| format!("Clipboard does not contain readable text: {err}"))
}

pub(crate) fn copy_to_clipboard(app: &mut AppState, label: &str, value: String) {
    match set_system_clipboard(&value) {
        Ok(()) => {
            app.status_message = format!("Copied {label}: {}", ellipsize(&value, 48));
        }
        Err(err) => {
            app.status_message = format!("Could not copy {label}: {err}");
        }
    }
}

pub(crate) fn copy_inspector_value(app: &mut AppState, action: InspectorCopyAction) {
    match action {
        InspectorCopyAction::Field(field) => {
            let label = match field {
                InspectorField::ElementId => "ID",
                InspectorField::ElementLodParent => "LOD Parent",
                InspectorField::ElementUniqueId => "Unique ID",
                InspectorField::DefinitionDff => "DFF override",
                InspectorField::DefinitionNativeModel => "Native Behavior Model",
                InspectorField::DefinitionTxd => "TXD",
                InspectorField::DefinitionCol => "COL",
                InspectorField::DefinitionLod => "Draw Distance",
                _ => "field",
            };
            let value = match field {
                InspectorField::DefinitionDff => selected_definition(app)
                    .map(|def| {
                        dff_override_stem(&def.id, def.attrs.get("dff").map(String::as_str))
                            .unwrap_or_else(|| def.id.clone())
                    })
                    .unwrap_or_default(),
                _ => inspector_field_value(app, field),
            };
            copy_to_clipboard(app, label, value);
        }
        InspectorCopyAction::ElementPosition => {
            if let Some(value) = selected_placement(app).map(|p| vec3_csv(p.pos)) {
                copy_to_clipboard(app, "position", value);
            }
        }
        InspectorCopyAction::ElementRotation => {
            if let Some(value) = selected_placement(app).map(|p| vec3_csv(p.rot)) {
                copy_to_clipboard(app, "rotation", value);
            }
        }
        InspectorCopyAction::LightPosition => {
            if let Some(value) = selected_light(app).map(|light| vec3_csv(light.position)) {
                copy_to_clipboard(app, "light position", value);
            }
        }
        InspectorCopyAction::LightRotation => {
            if let Some(value) = selected_light(app).map(|light| vec3_csv(light.direction)) {
                copy_to_clipboard(app, "light rotation", value);
            }
        }
    }
}

pub(crate) fn selected_map_tag(placement: &Placement) -> String {
    write_tag(&placement.tag, &placement.attrs)
        .trim()
        .to_string()
}

const WATER_TEXTURE_MAX_Z_VARIANCE: f32 = 0.05;
const WATER_TEXTURE_MIN_XY_SPAN: f32 = 0.01;
const WATER_TEXTURE_MIN_PROJECTED_AREA: f32 = 0.0001;
const WATER_WORLD_MIN: f32 = -3000.0;
const WATER_WORLD_MAX: f32 = 3000.0;
const WATER_Z_MIN: f32 = -1000.0;
const WATER_Z_MAX: f32 = 1000.0;

pub(crate) fn water_texture_source_bytes(
    root: &Path,
    gta_sa_dir: &Path,
    dff_name: &str,
) -> Result<Vec<u8>, String> {
    let target = asset_key(dff_name, ".dff");
    for replacement_root in [wip_root_path(root), root.to_path_buf()] {
        if let Some(entry) = replacement_img_entry(&replacement_root, &target) {
            let bytes = read_img_entry(&entry);
            if !bytes.is_empty() {
                return Ok(bytes);
            }
        }
    }
    for path in collect_resource_img_files(root)
        .into_iter()
        .chain(gta_sa_img_files(gta_sa_dir))
    {
        if let Some(entry) = parse_img(&path)
            .into_iter()
            .find(|entry| asset_key(&entry.name, ".dff") == target)
        {
            let bytes = read_img_entry(&entry);
            if !bytes.is_empty() {
                return Ok(bytes);
            }
        }
    }
    Err(format!("{target}: source DFF could not be found or read"))
}

fn water_plane_for_material_instance(
    raw: &RawMesh,
    material: usize,
    matrix: &[f32; 16],
) -> Result<WaterPlane, String> {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    let mut z_sum = 0.0f64;
    let mut corner_count = 0usize;
    let mut projected_area = 0.0f64;
    let mut face_count = 0usize;

    for tri in raw
        .triangles
        .iter()
        .filter(|tri| tri.material as usize == material)
    {
        let indices = [tri.a as usize, tri.b as usize, tri.c as usize];
        if indices.iter().any(|index| *index >= raw.vertices.len()) {
            return Err(format!(
                "material {material} contains a face with an invalid vertex index"
            ));
        }
        let points = indices.map(|index| transform_point_gl(matrix, raw.vertices[index]));
        for point in points {
            if !point.is_finite() {
                return Err(format!(
                    "material {material} contains non-finite transformed geometry"
                ));
            }
            min = min.min(point);
            max = max.max(point);
            z_sum += point.z as f64;
            corner_count += 1;
        }
        let ab = points[1] - points[0];
        let ac = points[2] - points[0];
        projected_area += ((ab.x as f64 * ac.y as f64 - ab.y as f64 * ac.x as f64).abs()) * 0.5;
        face_count += 1;
    }

    if face_count == 0 {
        return Err(format!(
            "material {material} has no faces in the source DFF"
        ));
    }
    let z_variance = max.z - min.z;
    if z_variance > WATER_TEXTURE_MAX_Z_VARIANCE {
        return Err(format!(
            "the textured surface is not flat and horizontal (Z variance {z_variance:.3}; maximum {:.3})",
            WATER_TEXTURE_MAX_Z_VARIANCE
        ));
    }
    if max.x - min.x < WATER_TEXTURE_MIN_XY_SPAN
        || max.y - min.y < WATER_TEXTURE_MIN_XY_SPAN
        || projected_area < WATER_TEXTURE_MIN_PROJECTED_AREA as f64
    {
        return Err("the textured surface has no usable horizontal area".to_string());
    }
    if min.x < WATER_WORLD_MIN
        || min.y < WATER_WORLD_MIN
        || max.x > WATER_WORLD_MAX
        || max.y > WATER_WORLD_MAX
        || min.z < WATER_Z_MIN
        || max.z > WATER_Z_MAX
    {
        return Err(format!(
            "the water bounds exceed the loader limits (XY {WATER_WORLD_MIN:.0}..{WATER_WORLD_MAX:.0}, Z {WATER_Z_MIN:.0}..{WATER_Z_MAX:.0})"
        ));
    }

    let z = (z_sum / corner_count as f64) as f32;
    Ok(WaterPlane {
        corners: [
            default_water_corner(min.x, min.y, z),
            default_water_corner(max.x, min.y, z),
            default_water_corner(min.x, max.y, z),
            default_water_corner(max.x, max.y, z),
        ],
        kind: 1,
    })
}

fn convert_water_texture(
    source: WaterTextureDffSource,
    dff_name: String,
    material: usize,
    texture_name: String,
    placement_matrices: Vec<[f32; 16]>,
    write_options: DffWriteOptions,
) -> Result<WaterTextureConversionResult, String> {
    let mut raw = match source {
        WaterTextureDffSource::Raw(raw) => *raw,
        WaterTextureDffSource::Bytes(bytes) => parse_dff_mesh(&bytes),
        WaterTextureDffSource::Entry(entry) => parse_dff_mesh(&read_img_entry(&entry)),
        WaterTextureDffSource::Archive {
            root,
            gta_sa_dir,
            dff_name,
        } => parse_dff_mesh(&water_texture_source_bytes(&root, &gta_sa_dir, &dff_name)?),
    };
    if raw.vertices.is_empty() || raw.triangles.is_empty() {
        return Err(format!("{dff_name}: DFF has no readable geometry"));
    }
    let frame_name = Path::new(&dff_name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("model");
    if !raw_mesh_is_safe_for_normalized_rewrite(&raw, frame_name) {
        return Err(format!(
            "{dff_name}: its multi-frame/component hierarchy cannot be safely rewritten"
        ));
    }
    if raw
        .components
        .iter()
        .any(|component| component.breakable.is_some())
    {
        return Err(format!(
            "{dff_name}: fracture geometry is attached; remove or regenerate it before deleting material faces"
        ));
    }
    if let Some(actual_name) = raw.material_textures.get(material)
        && !texture_name.trim().is_empty()
        && !actual_name.trim().eq_ignore_ascii_case(texture_name.trim())
    {
        return Err(format!(
            "{dff_name}: material {material} changed from '{}' to '{}'; select the texture again",
            texture_name, actual_name
        ));
    }
    if placement_matrices.is_empty() {
        return Err(format!("{dff_name}: no live object instances remain"));
    }

    let mut water_planes = Vec::with_capacity(placement_matrices.len());
    for (index, matrix) in placement_matrices.iter().enumerate() {
        water_planes.push(
            water_plane_for_material_instance(&raw, material, matrix)
                .map_err(|error| format!("instance {}: {error}", index + 1))?,
        );
    }

    let keep = raw
        .triangles
        .iter()
        .map(|tri| tri.material as usize != material)
        .collect::<Vec<_>>();
    let removed_faces = retain_raw_triangles(&mut raw, &keep);
    if removed_faces == 0 {
        return Err(format!("{dff_name}: material {material} has no faces"));
    }
    if raw.triangles.is_empty() {
        return Err(format!(
            "{dff_name}: removing this material would leave the DFF with no faces"
        ));
    }
    compact_raw_vertices(&mut raw);
    let dff_bytes = write_normalized_dff_with_options(&raw, frame_name, write_options)?;
    Ok(WaterTextureConversionResult {
        dff_name,
        raw,
        dff_bytes,
        water_planes,
        removed_faces,
        texture_name,
    })
}

pub(crate) fn water_texture_source_for_app(
    app: &AppState,
    dff_name: &str,
) -> WaterTextureDffSource {
    let key = asset_key(dff_name, ".dff");
    if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_ref()
        && asset_key(&dff.name, ".dff") == key
    {
        return WaterTextureDffSource::Raw(Box::new(dff.raw.clone()));
    }
    if let Some(bytes) = app.editing.modified_entries.get(&key) {
        return WaterTextureDffSource::Bytes(bytes.clone());
    }
    if let Some((_, bytes)) = app.pending_replacement_assets.get(&key) {
        return WaterTextureDffSource::Bytes(bytes.clone());
    }
    WaterTextureDffSource::Archive {
        root: app.root.clone(),
        gta_sa_dir: app.gta_sa_dir.clone(),
        dff_name: dff_name.to_string(),
    }
}

pub(crate) fn request_water_texture_conversion(
    app: &mut AppState,
    placement_index: usize,
    material: usize,
) {
    if app.water_texture_conversion_job.is_some() || app.preview_world_uv_job.is_some() {
        app.status_message = "A texture-to-water conversion is already running.".to_string();
        return;
    }
    if app.manual_save_job.is_some()
        || app.autosave_rx.is_some()
        || app.editing.save_rx.is_some()
        || app.bake_job.is_some()
        || app.dff_repair_rx.is_some()
        || app.txd_cleanup_job.is_some()
        || app.asset_optimization_scan_rx.is_some()
        || app.asset_optimization_job.is_some()
        || app.object_bounds_fix_job.is_some()
        || app.corona_generation_job.is_some()
        || app.fracture_generation_job.is_some()
        || app.dff_geometry_job.is_some()
        || app.collision_generation_job.is_some()
        || app.shadow_mesh_generation_job.is_some()
        || app.lod_generation_job.is_some()
        || app.instance_lod_removal_job.is_some()
    {
        app.status_message =
            "Wait for the active asset-writing job before converting a texture to water."
                .to_string();
        return;
    }
    let Some(placement) = app.placements.get(placement_index) else {
        app.status_message = "The selected object no longer exists.".to_string();
        return;
    };
    let Some(texture) = preview_material_entry(app, placement_index, material) else {
        app.status_message = "The selected texture material is no longer available.".to_string();
        return;
    };
    let dff_name = with_ext(&placement.dff, ".dff");
    let dff_key = asset_key(&dff_name, ".dff");
    if app.editing.deleted_entries.contains(&dff_key) {
        app.status_message =
            format!("{dff_name} is staged for deletion; restore it before converting its texture.");
        return;
    }
    let source = water_texture_source_for_app(app, &dff_name);
    let write_options = dff_write_options_for_asset(app, &dff_name);
    let label = if texture.texture_name.trim().is_empty() {
        format!("material #{material}")
    } else {
        texture.texture_name.clone()
    };
    app.water_texture_conversion_job = Some(WaterTextureConversionJob {
        dff_name,
        dff_key,
        material,
        texture_name: texture.texture_name,
        source: Some(source),
        write_options,
        snapshot_index: 0,
        placement_matrices: Vec::new(),
        txd_scopes: BTreeSet::new(),
        rx: None,
        result: None,
        apply_phase: 0,
        refresh_scopes: Vec::new(),
        refresh_index: 0,
        refreshed_any: false,
        started_at: Instant::now(),
    });
    app.status_message = format!("Preparing {label} for background texture-to-water conversion...");
}

pub(crate) fn update_water_texture_conversion_job(app: &mut AppState) {
    const SNAPSHOT_BATCH_LIMIT: usize = 256;
    const FRAME_BUDGET: Duration = Duration::from_millis(4);

    let Some(mut job) = app.water_texture_conversion_job.take() else {
        return;
    };

    if job.rx.is_none() && job.result.is_none() {
        let started = Instant::now();
        let mut processed = 0usize;
        while job.snapshot_index < app.placements.len()
            && processed < SNAPSHOT_BATCH_LIMIT
            && started.elapsed() < FRAME_BUDGET
        {
            let index = job.snapshot_index;
            job.snapshot_index += 1;
            processed += 1;
            if !is_live_element(app, index) {
                continue;
            }
            let Some(placement) = app.placements.get(index) else {
                continue;
            };
            if asset_key(&placement.dff, ".dff") == job.dff_key {
                job.placement_matrices
                    .push(placement_matrix(placement).to_cols_array());
                job.txd_scopes.insert(
                    definition_txd_name(&app.definitions, &placement.id).map(str::to_owned),
                );
            }
        }
        if job.snapshot_index < app.placements.len() {
            app.status_message = format!(
                "Finding DFF instances for texture-to-water... {}/{}",
                job.snapshot_index,
                app.placements.len()
            );
            app.water_texture_conversion_job = Some(job);
            return;
        }

        let Some(source) = job.source.take() else {
            app.status_message = "Texture-to-water source snapshot is missing.".to_string();
            return;
        };
        let dff_name = job.dff_name.clone();
        let material = job.material;
        let texture_name = job.texture_name.clone();
        let matrices = std::mem::take(&mut job.placement_matrices);
        let write_options = job.write_options;
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let result = std::panic::catch_unwind(|| {
                convert_water_texture(
                    source,
                    dff_name,
                    material,
                    texture_name,
                    matrices,
                    write_options,
                )
            })
            .map_err(|panic| {
                panic
                    .downcast_ref::<&str>()
                    .map(|message| (*message).to_string())
                    .or_else(|| panic.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "unknown texture-to-water worker panic".to_string())
            })
            .and_then(|result| result);
            let _ = tx.send(result);
        });
        job.rx = Some(rx);
        app.status_message =
            "Checking water planarity and rewriting the DFF in the background...".to_string();
        app.water_texture_conversion_job = Some(job);
        return;
    }

    if job.result.is_none() {
        let Some(rx) = job.rx.as_ref() else {
            app.status_message = "Texture-to-water worker failed to start.".to_string();
            return;
        };
        match rx.try_recv() {
            Ok(Ok(result)) => {
                job.result = Some(result);
                app.status_message =
                    "Texture-to-water conversion finished; staging results...".to_string();
            }
            Ok(Err(error)) => {
                app.status_message = format!("Texture-to-water conversion failed: {error}");
                return;
            }
            Err(mpsc::TryRecvError::Empty) => {
                app.water_texture_conversion_job = Some(job);
                return;
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                app.status_message =
                    "Texture-to-water worker disconnected unexpectedly.".to_string();
                return;
            }
        }
    }

    let result = job
        .result
        .as_ref()
        .expect("texture-to-water result exists while applying");
    match job.apply_phase {
        0 => {
            let first_water = app.water_planes.len();
            app.water_planes.extend(result.water_planes.iter().cloned());
            app.selected_water = first_water;
            app.selected_water_planes =
                (first_water..first_water + result.water_planes.len()).collect();
            app.pending_replacement_assets.insert(
                job.dff_key.clone(),
                (result.dff_name.clone(), result.dff_bytes.clone()),
            );
            app.pending_asset_deletes.remove(&job.dff_key);
            if app.editing.modified_entries.contains_key(&job.dff_key) {
                app.editing
                    .modified_entries
                    .insert(job.dff_key.clone(), result.dff_bytes.clone());
            }
            if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut()
                && asset_key(&dff.name, ".dff") == job.dff_key
            {
                dff.raw = result.raw.clone();
                dff.dirty = false;
                dff.selected_face = None;
                dff.selected_faces.clear();
                dff.selected_edges.clear();
                dff.selected_vertex = None;
                dff.selected_vertices.clear();
                app.editing
                    .modified_entries
                    .insert(job.dff_key.clone(), result.dff_bytes.clone());
            }
            app.loaded_wip = true;
            invalidate_validation_cache(app);
            clear_history_for_external_change(app);
            job.apply_phase = 1;
            job.refresh_scopes = job.txd_scopes.iter().cloned().collect();
            app.status_message = "Staged water and DFF changes; refreshing Preview...".to_string();
            app.water_texture_conversion_job = Some(job);
        }
        _ => {
            const REFRESH_BATCH_LIMIT: usize = 4;
            const REFRESH_FRAME_BUDGET: Duration = Duration::from_millis(4);

            let started = Instant::now();
            let mut processed = 0usize;
            while job.refresh_index < job.refresh_scopes.len()
                && processed < REFRESH_BATCH_LIMIT
                && started.elapsed() < REFRESH_FRAME_BUDGET
            {
                let txd_scope = job.refresh_scopes[job.refresh_index].as_deref();
                job.refreshed_any |= refresh_loaded_dff_scope_from_raw(
                    app,
                    &result.dff_name,
                    &result.raw,
                    txd_scope,
                );
                job.refresh_index += 1;
                processed += 1;
            }
            if job.refresh_index < job.refresh_scopes.len() {
                app.status_message = format!(
                    "Refreshing Preview DFF variants... {}/{}",
                    job.refresh_index,
                    job.refresh_scopes.len()
                );
                app.water_texture_conversion_job = Some(job);
                return;
            }
            if job.refreshed_any {
                rebuild_render_cells(app);
            }
            app.preview_selected_material = None;
            app.status_message = format!(
                "Converted '{}' to {} water plane{} across all live DFF instances and removed {} face{} from {}; undo history cleared{} ({:.1}s).",
                if result.texture_name.trim().is_empty() {
                    format!("material #{}", job.material)
                } else {
                    result.texture_name.clone()
                },
                result.water_planes.len(),
                if result.water_planes.len() == 1 {
                    ""
                } else {
                    "s"
                },
                result.removed_faces,
                if result.removed_faces == 1 { "" } else { "s" },
                result.dff_name,
                if job.refreshed_any {
                    ""
                } else {
                    "; Preview refresh failed"
                },
                job.started_at.elapsed().as_secs_f32()
            );
        }
    }
}

fn texture_match_indices(
    candidates: &[TextureMatchCandidate],
    mode: TextureMatchMode,
    target_name: &str,
    target_fingerprint: Option<TextureContentFingerprint>,
) -> Vec<usize> {
    candidates
        .iter()
        .filter(|candidate| {
            candidate
                .textures
                .iter()
                .any(|(name, fingerprint)| match mode {
                    TextureMatchMode::Name => {
                        !target_name.trim().is_empty()
                            && name.trim().eq_ignore_ascii_case(target_name.trim())
                    }
                    TextureMatchMode::Content => target_fingerprint
                        .is_some_and(|target| fingerprint.is_some_and(|value| value == target)),
                })
        })
        .map(|candidate| candidate.placement)
        .collect()
}

pub(crate) fn request_texture_match_selection(
    app: &mut AppState,
    placement: usize,
    material: usize,
    mode: TextureMatchMode,
) {
    if app.texture_match_selection_job.is_some() {
        app.status_message = "A matching-texture selection is already running.".to_string();
        return;
    }
    let Some(texture) = preview_material_entry(app, placement, material) else {
        app.status_message = "The selected texture material is no longer available.".to_string();
        return;
    };
    if mode == TextureMatchMode::Name && texture.texture_name.trim().is_empty() {
        app.status_message = "This material has no texture name to match.".to_string();
        return;
    }
    if mode == TextureMatchMode::Content && texture.fingerprint.is_none() {
        app.status_message =
            "This texture has no resolved content available for matching.".to_string();
        return;
    }
    let target_label = if texture.texture_name.trim().is_empty() {
        format!("material #{material}")
    } else {
        texture.texture_name.clone()
    };
    app.texture_match_selection_job = Some(TextureMatchSelectionJob {
        mode,
        target_name: texture.texture_name,
        target_fingerprint: texture.fingerprint,
        snapshot_index: 0,
        candidates: Vec::new(),
        rx: None,
        matches: None,
        apply_index: 0,
        newly_selected: 0,
        selection_before: None,
    });
    app.status_message = format!(
        "Preparing background {} match for {target_label}...",
        match mode {
            TextureMatchMode::Name => "texture-name",
            TextureMatchMode::Content => "texture-content",
        }
    );
}

pub(crate) fn update_texture_match_selection_job(app: &mut AppState) {
    const SNAPSHOT_BATCH_LIMIT: usize = 256;
    const APPLY_BATCH_LIMIT: usize = 256;
    const FRAME_BUDGET: Duration = Duration::from_millis(4);

    let Some(mut job) = app.texture_match_selection_job.take() else {
        return;
    };

    if job.rx.is_none() && job.matches.is_none() {
        let started = Instant::now();
        let mut processed = 0usize;
        while job.snapshot_index < app.placements.len()
            && processed < SNAPSHOT_BATCH_LIMIT
            && started.elapsed() < FRAME_BUDGET
        {
            let index = job.snapshot_index;
            job.snapshot_index += 1;
            processed += 1;
            if !is_live_element(app, index) {
                continue;
            }
            let Some(placement) = app.placements.get(index) else {
                continue;
            };
            let Some(mesh) = element_mesh(app, placement) else {
                continue;
            };
            let mut textures = Vec::with_capacity(mesh.parts.len());
            let mut seen = HashSet::new();
            for part in &mesh.parts {
                let key = (
                    part.texture_name.trim().to_ascii_lowercase(),
                    part.texture_fingerprint,
                );
                if seen.insert(key) {
                    textures.push((part.texture_name.clone(), part.texture_fingerprint));
                }
            }
            if !textures.is_empty() {
                job.candidates.push(TextureMatchCandidate {
                    placement: index,
                    textures,
                });
            }
        }
        if job.snapshot_index < app.placements.len() {
            app.status_message = format!(
                "Preparing texture match... {}/{} elements",
                job.snapshot_index,
                app.placements.len()
            );
            app.texture_match_selection_job = Some(job);
            return;
        }

        let candidates = std::mem::take(&mut job.candidates);
        let mode = job.mode;
        let target_name = job.target_name.clone();
        let target_fingerprint = job.target_fingerprint;
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let result = std::panic::catch_unwind(|| {
                texture_match_indices(&candidates, mode, &target_name, target_fingerprint)
            })
            .map_err(|panic| {
                panic
                    .downcast_ref::<&str>()
                    .map(|message| (*message).to_string())
                    .or_else(|| panic.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "unknown matching-texture worker panic".to_string())
            });
            let _ = tx.send(result);
        });
        job.rx = Some(rx);
        app.status_message = "Matching textures in the background...".to_string();
        app.texture_match_selection_job = Some(job);
        return;
    }

    if job.matches.is_none() {
        let Some(rx) = job.rx.as_ref() else {
            app.status_message = "Matching-texture worker failed to start.".to_string();
            return;
        };
        match rx.try_recv() {
            Ok(Ok(matches)) => {
                job.matches = Some(matches);
                app.status_message =
                    "Texture match finished; adding elements to the selection...".to_string();
            }
            Ok(Err(error)) => {
                app.status_message = format!("Matching-texture selection failed: {error}");
                return;
            }
            Err(mpsc::TryRecvError::Empty) => {
                app.texture_match_selection_job = Some(job);
                return;
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                app.status_message =
                    "Matching-texture worker disconnected unexpectedly.".to_string();
                return;
            }
        }
    }

    let matches = job
        .matches
        .as_ref()
        .expect("matching-texture results exist while applying");
    if job.selection_before.is_none() {
        job.selection_before = Some(selection_history_snapshot(app));
    }
    let started = Instant::now();
    let mut applied = 0usize;
    while job.apply_index < matches.len()
        && applied < APPLY_BATCH_LIMIT
        && started.elapsed() < FRAME_BUDGET
    {
        let index = matches[job.apply_index];
        job.apply_index += 1;
        applied += 1;
        if is_live_element(app, index) && app.selected_elements.insert(index) {
            app.selected_element_order.push(index);
            job.newly_selected += 1;
        }
    }
    if job.apply_index < matches.len() {
        app.status_message = format!(
            "Adding texture matches... {}/{}",
            job.apply_index,
            matches.len()
        );
        app.texture_match_selection_job = Some(job);
        return;
    }

    app.selected_group = None;
    let match_count = matches.len();
    let mode_label = match job.mode {
        TextureMatchMode::Name => "name",
        TextureMatchMode::Content => "content",
    };
    if let Some(before) = job.selection_before.take() {
        commit_selection_history(
            app,
            match job.mode {
                TextureMatchMode::Name => "Select Matching Texture Name",
                TextureMatchMode::Content => "Select Matching Texture Content",
            },
            before,
        );
    }
    app.status_message = format!(
        "Texture {mode_label} matched {match_count} element{}; added {} to the existing selection ({} total).",
        if match_count == 1 { "" } else { "s" },
        job.newly_selected,
        app.selected_elements.len()
    );
}

pub(crate) fn cancel_texture_match_selection_for_undo(app: &mut AppState) -> bool {
    let Some(mut job) = app.texture_match_selection_job.take() else {
        return false;
    };
    if let Some(before) = job.selection_before.take() {
        apply_selection_history_snapshot(app, before);
    }
    app.status_message =
        "Cancelled matching-texture selection and restored the selection.".to_string();
    true
}

pub(crate) fn context_menu_items(app: &AppState) -> Vec<(ContextAction, &'static str, bool)> {
    if matches!(
        app.context_menu.as_ref().map(|menu| &menu.target),
        Some(ContextMenuTarget::EditingTexture { .. })
    ) {
        return vec![(ContextAction::CategorizeTexture, "Categorize...", true)];
    }
    if matches!(
        app.context_menu.as_ref().map(|menu| &menu.target),
        Some(ContextMenuTarget::AssetBrowser { .. })
    ) {
        return vec![(ContextAction::ViewAssetPreview, "View preview", true)];
    }
    if matches!(
        app.context_menu.as_ref().map(|menu| menu.target.clone()),
        Some(ContextMenuTarget::Scene { .. })
    ) {
        let mut items = vec![(ContextAction::AddLight, "Add a light", true)];
        let has_live_element = app.placements.get(app.selected).is_some()
            && !app
                .element_states
                .get(app.selected)
                .is_some_and(|state| state.deleted);
        if has_live_element {
            items.push((
                ContextAction::AddLightToInstance,
                "Add a light to instance",
                true,
            ));
        }
        return items;
    }
    if let Some(ContextMenuTarget::PreviewTexture {
        placement,
        material,
    }) = app.context_menu.as_ref().map(|menu| menu.target.clone())
    {
        let texture = preview_material_entry(app, placement, material);
        let has_name = texture
            .as_ref()
            .is_some_and(|entry| !entry.texture_name.trim().is_empty());
        let has_content = texture
            .as_ref()
            .is_some_and(|entry| entry.fingerprint.is_some());
        return vec![
            (
                ContextAction::AddTextureToWater,
                "Add texture -> Water system",
                texture.as_ref().is_some_and(|entry| entry.face_count > 0)
                    && app.water_texture_conversion_job.is_none(),
            ),
            (
                ContextAction::SelectMatchingTextureName,
                "Select all with matching texture (NAME)",
                has_name,
            ),
            (
                ContextAction::SelectMatchingTextureContent,
                "Select all with matching texture (Content)",
                has_content,
            ),
        ];
    }
    if let Some(ContextMenuTarget::EditingDffMaterial { dff_name, material }) =
        app.context_menu.as_ref().map(|menu| menu.target.clone())
    {
        let selected = app.editing.asset.as_ref().and_then(|asset| match asset {
            EditingAsset::Dff(dff)
                if dff.name.eq_ignore_ascii_case(&dff_name)
                    && material < dff_material_slot_count(&dff.raw) =>
            {
                Some(dff)
            }
            _ => None,
        });
        let valid = selected.is_some();
        let has_texture = selected.is_some_and(|dff| {
            dff.raw
                .material_textures
                .get(material)
                .is_some_and(|name| !name.trim().is_empty())
        });
        let can_replace =
            has_texture && selected.is_some_and(|dff| dff.txd_context.is_some() && !dff.read_only);
        let emitter_enabled = valid && dff_material_emitter(app, &dff_name, material).enabled;
        return vec![
            (
                ContextAction::ViewDffMaterialTexture,
                "View full size",
                has_texture,
            ),
            (
                ContextAction::ExportDffMaterialTexture,
                "Export",
                has_texture,
            ),
            (
                ContextAction::ReplaceDffMaterialTexture,
                "Replace",
                can_replace,
            ),
            (
                ContextAction::CategorizeTexture,
                "Categorize...",
                has_texture && selected.is_some_and(|dff| dff.txd_context.is_some()),
            ),
            (
                ContextAction::SelectAllDffMaterialFaces,
                "Select all",
                valid,
            ),
            (
                ContextAction::MarkDffTextureElementsDoubleSided,
                "Mark as double sided",
                has_texture,
            ),
            (
                ContextAction::ToggleDffMaterialEmitter,
                if emitter_enabled {
                    "Disable light source emitter"
                } else {
                    "Mark as light source emitter"
                },
                valid,
            ),
        ];
    }
    if let Some(ContextMenuTarget::EditingDffFaceLighting {
        dff_name,
        emitter_key,
    }) = app.context_menu.as_ref().map(|menu| menu.target.clone())
    {
        let dff_key = asset_key(&dff_name, ".dff");
        let key_matches_dff = material_emitter_face_group_from_key(&emitter_key)
            .is_some_and(|(entry_dff, _)| entry_dff == dff_key)
            || material_emitter_face_from_key(&emitter_key)
                .is_some_and(|(entry_dff, _)| entry_dff == dff_key);
        let valid = matches!(
            app.editing.asset.as_ref(),
            Some(EditingAsset::Dff(dff)) if dff.name.eq_ignore_ascii_case(&dff_name)
        ) && key_matches_dff
            && app.material_emitters.contains_key(&emitter_key);
        return vec![(ContextAction::ClearDffFaceLighting, "Clear", valid)];
    }
    if let Some(ContextMenuTarget::LodAuditIssue(issue_index)) =
        app.context_menu.as_ref().map(|menu| menu.target.clone())
    {
        let issue = app
            .lod_audit
            .result
            .as_ref()
            .and_then(|result| result.issues.get(issue_index));
        let is_coverage = issue.is_some_and(|issue| issue.kind == LodAuditIssueKind::Missing);
        let ignored = issue.is_some_and(|issue| lod_audit_issue_is_ignored(app, issue));
        return vec![
            (
                ContextAction::LodAuditGenerate,
                "Generate LOD",
                !lod_audit_issue_generation_indices(app, issue_index).is_empty(),
            ),
            (
                ContextAction::LodAuditToggleIgnore,
                if ignored { "Unignore" } else { "Ignore" },
                is_coverage,
            ),
        ];
    }
    if app.active_tab == AppTab::Lights {
        let has_selection = selected_light(app).is_some();
        return vec![
            (
                ContextAction::AddLightToInstance,
                "Add to instance",
                has_selection
                    && app.placements.get(app.selected).is_some()
                    && !app
                        .element_states
                        .get(app.selected)
                        .is_some_and(|state| state.deleted),
            ),
            (ContextAction::CopyId, "Copy Name", has_selection),
            (ContextAction::CopyPosition, "Copy Position", has_selection),
            (ContextAction::Duplicate, "Duplicate", has_selection),
            (ContextAction::DeleteRestore, "Delete", has_selection),
            (ContextAction::Deselect, "Deselect", has_selection),
        ];
    }
    let has_selection = selected_placement(app).is_some();
    let deleted = app
        .element_states
        .get(app.selected)
        .is_some_and(|state| state.deleted);
    let hidden = app
        .element_states
        .get(app.selected)
        .is_some_and(|state| state.hidden);
    let any_hidden = app
        .element_states
        .iter()
        .any(|state| state.hidden && !state.deleted);
    vec![
        (ContextAction::CopyId, "Copy ID", has_selection),
        (ContextAction::CopyDff, "Copy DFF", has_selection),
        (ContextAction::ExportDff, "Export DFF", has_selection),
        (ContextAction::ExportCol, "Export COL", has_selection),
        (
            ContextAction::ReplaceDff,
            "Replace DFF",
            has_selection && !deleted,
        ),
        (
            ContextAction::ReplaceCol,
            "Replace COL",
            has_selection && !deleted,
        ),
        (
            ContextAction::BlenderPosition,
            "Blender Position",
            selected_live_indices(app).len() >= 2,
        ),
        (ContextAction::CopyPosition, "Copy Position", has_selection),
        (ContextAction::CopyMapTag, "Copy Map Tag", has_selection),
        (ContextAction::SnapTo, "Snap Camera", has_selection),
        (
            ContextAction::HideUnhide,
            if hidden { "Unhide" } else { "Hide" },
            has_selection && !deleted,
        ),
        (
            ContextAction::HideEverythingBut,
            "Hide Everything But Selection",
            !selected_live_indices(app).is_empty(),
        ),
        (ContextAction::UnhideAll, "Unhide All", any_hidden),
        (
            ContextAction::Join,
            "Join",
            selected_live_indices(app).len() >= 2,
        ),
        (
            ContextAction::Duplicate,
            "Duplicate",
            has_selection && !deleted,
        ),
        (
            ContextAction::DeleteRestore,
            if deleted { "Restore" } else { "Delete" },
            has_selection,
        ),
        (ContextAction::Deselect, "Deselect", has_selection),
    ]
}

pub(crate) fn context_menu_rect(app: &AppState) -> Option<Rect> {
    let menu = app.context_menu.as_ref()?;
    let w = match menu.target {
        ContextMenuTarget::PreviewTexture { .. } => 370.0,
        ContextMenuTarget::EditingDffMaterial { .. } => 250.0,
        ContextMenuTarget::EditingDffFaceLighting { .. } => 178.0,
        ContextMenuTarget::EditingTexture { .. } => 190.0,
        _ => 178.0,
    };
    let h = context_menu_items(app).len() as f32 * 26.0 + 12.0;
    let x = menu.pos.x.min(screen_width() - w - 8.0).max(8.0);
    let y = menu
        .pos
        .y
        .min(screen_height() - h - STATUS_H - 8.0)
        .max(TOP_H + 8.0);
    Some(Rect::new(x, y, w, h))
}

pub(crate) fn context_action_at(app: &AppState, mouse: Vec2) -> Option<ContextAction> {
    let rect = context_menu_rect(app)?;
    if !rect.contains(mouse) {
        return None;
    }
    let row_h = 26.0;
    let row = ((mouse.y - rect.y - 6.0) / row_h).floor() as usize;
    let items = context_menu_items(app);
    let (action, _, enabled) = items.get(row).copied()?;
    enabled.then_some(action)
}

pub(crate) fn run_context_action(app: &mut AppState, action: ContextAction) {
    if let Some(ContextMenuTarget::EditingTexture {
        txd_name,
        texture_name,
    }) = app.context_menu.as_ref().map(|menu| menu.target.clone())
    {
        if action == ContextAction::CategorizeTexture {
            let position = app
                .context_menu
                .as_ref()
                .map(|menu| menu.pos)
                .unwrap_or_else(|| mouse_position().into());
            app.editing.texture_category_menu = Some(TextureCategoryMenu {
                txd_name,
                texture_name,
                position,
            });
        }
        return;
    }
    if let Some(ContextMenuTarget::AssetBrowser { entry_id }) =
        app.context_menu.as_ref().map(|menu| menu.target.clone())
    {
        if action == ContextAction::ViewAssetPreview
            && let Some(entry) = asset_browser_entries(app)
                .iter()
                .find(|entry| entry.id == entry_id)
                .cloned()
        {
            preview_asset_from_browser(app, &entry);
        }
        return;
    }
    if let Some(ContextMenuTarget::Scene { position }) =
        app.context_menu.as_ref().map(|menu| menu.target.clone())
    {
        match action {
            ContextAction::AddLight => add_light_at(app, to_mq(position)),
            ContextAction::AddLightToInstance => {
                add_light_to_selected_instance_at(app, to_mq(position));
            }
            _ => {}
        }
        return;
    }
    if let Some(ContextMenuTarget::PreviewTexture {
        placement,
        material,
    }) = app.context_menu.as_ref().map(|menu| menu.target.clone())
    {
        match action {
            ContextAction::SelectMatchingTextureName => {
                request_texture_match_selection(app, placement, material, TextureMatchMode::Name);
            }
            ContextAction::SelectMatchingTextureContent => {
                request_texture_match_selection(
                    app,
                    placement,
                    material,
                    TextureMatchMode::Content,
                );
            }
            ContextAction::AddTextureToWater => {
                request_water_texture_conversion(app, placement, material);
            }
            _ => {}
        }
        return;
    }
    if let Some(ContextMenuTarget::EditingDffMaterial { dff_name, material }) =
        app.context_menu.as_ref().map(|menu| menu.target.clone())
    {
        match action {
            ContextAction::ViewDffMaterialTexture => {
                open_dff_material_texture_view_dialog(app, &dff_name, material);
            }
            ContextAction::ExportDffMaterialTexture => {
                open_dff_material_texture_export_picker(app, &dff_name, material);
            }
            ContextAction::ReplaceDffMaterialTexture => {
                start_dff_material_texture_replace_browse(app, &dff_name, material);
            }
            ContextAction::CategorizeTexture => {
                let target = app.editing.asset.as_ref().and_then(|asset| match asset {
                    EditingAsset::Dff(dff) if dff.name.eq_ignore_ascii_case(&dff_name) => Some((
                        dff.txd_context.clone()?,
                        dff.raw.material_textures.get(material)?.clone(),
                    )),
                    _ => None,
                });
                if let Some((txd_name, texture_name)) = target {
                    let position = app
                        .context_menu
                        .as_ref()
                        .map(|menu| menu.pos)
                        .unwrap_or_else(|| mouse_position().into());
                    app.editing.texture_category_menu = Some(TextureCategoryMenu {
                        txd_name,
                        texture_name,
                        position,
                    });
                }
            }
            ContextAction::SelectAllDffMaterialFaces => {
                editing_select_all_dff_material_faces(app, &dff_name, material);
            }
            ContextAction::MarkDffTextureElementsDoubleSided => {
                request_mark_dff_texture_elements_double_sided(app, &dff_name, material);
            }
            ContextAction::ToggleDffMaterialEmitter => {
                editing_toggle_dff_material_emitter(app, &dff_name, material);
            }
            _ => {}
        }
        return;
    }
    if let Some(ContextMenuTarget::EditingDffFaceLighting {
        dff_name,
        emitter_key,
    }) = app.context_menu.as_ref().map(|menu| menu.target.clone())
    {
        if action == ContextAction::ClearDffFaceLighting {
            editing_clear_dff_face_lighting(app, &dff_name, &emitter_key);
        }
        return;
    }
    if let Some(ContextMenuTarget::LodAuditIssue(issue_index)) =
        app.context_menu.as_ref().map(|menu| menu.target.clone())
    {
        match action {
            ContextAction::LodAuditGenerate => {
                let indices = lod_audit_issue_generation_indices(app, issue_index);
                if indices.is_empty() {
                    app.status_message =
                        "This Coverage warning does not have a detail object that can generate an LOD."
                            .to_string();
                    return;
                }
                app.selected_elements = indices.iter().copied().collect();
                app.selected_element_order = indices;
                app.selected = app
                    .selected_element_order
                    .last()
                    .copied()
                    .unwrap_or(NO_SELECTION);
                app.selected_group = None;
                app.selected_col_face = None;
                request_selected_element_lod(app);
            }
            ContextAction::LodAuditToggleIgnore => {
                toggle_lod_audit_coverage_ignore(app, issue_index);
            }
            _ => {}
        }
        return;
    }
    if app.active_tab == AppTab::Lights {
        match action {
            ContextAction::AddLightToInstance => {
                attach_selected_light_to_instance(app);
            }
            ContextAction::CopyId => {
                if let Some(light) = selected_light(app) {
                    copy_to_clipboard(app, "light name", light.name.clone());
                }
            }
            ContextAction::CopyPosition => {
                if let Some(light) = selected_light(app) {
                    copy_to_clipboard(
                        app,
                        "light position",
                        format!(
                            "{:.3}, {:.3}, {:.3}",
                            light.position.x, light.position.y, light.position.z
                        ),
                    );
                }
            }
            ContextAction::Duplicate => duplicate_selected_light(app),
            ContextAction::DeleteRestore => delete_selected_light(app),
            ContextAction::Deselect => {
                app.selected_light = app.lights.len();
                app.hovered_gizmo = None;
            }
            ContextAction::AddLight
            | ContextAction::ViewAssetPreview
            | ContextAction::CopyDff
            | ContextAction::ExportDff
            | ContextAction::ExportCol
            | ContextAction::ReplaceDff
            | ContextAction::ReplaceCol
            | ContextAction::BlenderPosition
            | ContextAction::CopyMapTag
            | ContextAction::SnapTo
            | ContextAction::HideUnhide
            | ContextAction::HideEverythingBut
            | ContextAction::UnhideAll
            | ContextAction::Join
            | ContextAction::SelectMatchingTextureName
            | ContextAction::SelectMatchingTextureContent
            | ContextAction::AddTextureToWater
            | ContextAction::LodAuditGenerate
            | ContextAction::LodAuditToggleIgnore
            | ContextAction::ViewDffMaterialTexture
            | ContextAction::ExportDffMaterialTexture
            | ContextAction::ReplaceDffMaterialTexture
            | ContextAction::SelectAllDffMaterialFaces
            | ContextAction::MarkDffTextureElementsDoubleSided
            | ContextAction::ToggleDffMaterialEmitter
            | ContextAction::ClearDffFaceLighting
            | ContextAction::CategorizeTexture => {}
        }
        return;
    }
    match action {
        ContextAction::AddLight
        | ContextAction::AddLightToInstance
        | ContextAction::ViewAssetPreview => {}
        ContextAction::CopyId => {
            if let Some(p) = selected_placement(app) {
                copy_to_clipboard(app, "ID", p.id.clone());
            }
        }
        ContextAction::CopyDff => {
            if let Some(p) = selected_placement(app) {
                copy_to_clipboard(app, "DFF", p.dff.clone());
            }
        }
        ContextAction::ExportDff => open_export_dff_dialog(app),
        ContextAction::ExportCol => open_export_col_dialog(app),
        ContextAction::ReplaceDff => open_replace_dff_dialog(app),
        ContextAction::ReplaceCol => open_replace_col_dialog(app),
        ContextAction::BlenderPosition => write_blender_position_script(app),
        ContextAction::CopyPosition => {
            if let Some(p) = selected_placement(app) {
                copy_to_clipboard(
                    app,
                    "position",
                    format!("{:.3}, {:.3}, {:.3}", p.pos.x, p.pos.y, p.pos.z),
                );
            }
        }
        ContextAction::CopyMapTag => {
            if let Some(p) = selected_placement(app) {
                let tag = selected_map_tag(p);
                copy_to_clipboard(app, "map tag", tag);
            }
        }
        ContextAction::SnapTo => {
            if app.selected < app.placements.len() {
                snap_to(app, app.selected);
            }
        }
        ContextAction::HideUnhide => {
            if app
                .element_states
                .get(app.selected)
                .is_some_and(|state| state.hidden)
            {
                unhide_selected(app);
            } else {
                hide_selected(app);
            }
        }
        ContextAction::HideEverythingBut => hide_everything_but_selected(app),
        ContextAction::UnhideAll => unhide_all(app),
        ContextAction::Join => join_selected_elements(app),
        ContextAction::Duplicate => duplicate_selected(app),
        ContextAction::DeleteRestore => {
            if app
                .element_states
                .get(app.selected)
                .is_some_and(|state| state.deleted)
            {
                restore_selected(app);
            } else {
                delete_selected(app);
            }
        }
        ContextAction::Deselect => deselect_element(app),
        ContextAction::SelectMatchingTextureName
        | ContextAction::SelectMatchingTextureContent
        | ContextAction::AddTextureToWater
        | ContextAction::LodAuditGenerate
        | ContextAction::LodAuditToggleIgnore
        | ContextAction::ViewDffMaterialTexture
        | ContextAction::ExportDffMaterialTexture
        | ContextAction::ReplaceDffMaterialTexture
        | ContextAction::SelectAllDffMaterialFaces
        | ContextAction::MarkDffTextureElementsDoubleSided
        | ContextAction::ToggleDffMaterialEmitter
        | ContextAction::ClearDffFaceLighting
        | ContextAction::CategorizeTexture => {}
    }
}

fn placement_indices_referencing_texture(app: &AppState, texture_name: &str) -> Vec<usize> {
    let texture_name = texture_name.trim();
    if texture_name.is_empty() {
        return Vec::new();
    }
    app.placements
        .iter()
        .enumerate()
        .filter_map(|(index, placement)| {
            if !is_live_element(app, index) {
                return None;
            }
            element_mesh(app, placement)
                .is_some_and(|mesh| {
                    mesh.parts
                        .iter()
                        .any(|part| part.texture_name.trim().eq_ignore_ascii_case(texture_name))
                })
                .then_some(index)
        })
        .collect()
}

pub(crate) fn request_mark_dff_texture_elements_double_sided(
    app: &mut AppState,
    dff_name: &str,
    material: usize,
) {
    let texture_name = match app.editing.asset.as_ref() {
        Some(EditingAsset::Dff(dff))
            if dff.name.eq_ignore_ascii_case(dff_name)
                && material < dff_material_slot_count(&dff.raw) =>
        {
            dff.raw
                .material_textures
                .get(material)
                .map(|name| name.trim().to_string())
                .filter(|name| !name.is_empty())
        }
        _ => None,
    };
    let Some(texture_name) = texture_name else {
        app.status_message = "The selected DFF material has no texture to match".to_string();
        return;
    };
    let indices = placement_indices_referencing_texture(app, &texture_name);
    if indices.is_empty() {
        app.status_message = format!("No live elements reference texture '{texture_name}'");
        return;
    }
    let change_count = indices
        .iter()
        .filter(|index| {
            app.placements.get(**index).is_some_and(|placement| {
                !placement_override_flag_enabled(placement, "double_sided")
            })
        })
        .count();
    app.confirm_dialog = Some(ConfirmDialog {
        action: ConfirmAction::MarkTextureElementsDoubleSided {
            indices,
            texture_name: texture_name.clone(),
        },
        title: "Mark Elements Double Sided?".to_string(),
        body: format!(
            "Mark every live element that references texture '{texture_name}' as double sided?"
        ),
        detail: format!(
            "This will update {change_count} element{}; elements already marked double sided will be left unchanged.",
            if change_count == 1 { "" } else { "s" }
        ),
        primary_label: "Mark Double Sided".to_string(),
        secondary_label: None,
        secondary_action: None,
    });
    app.status_message =
        format!("Confirm marking elements that reference '{texture_name}' double sided");
}

pub(crate) fn mark_texture_elements_double_sided(
    app: &mut AppState,
    indices: Vec<usize>,
    texture_name: &str,
) {
    let before = local_world_history_snapshot(app, indices.iter().copied(), []);
    let changed = mark_placements_double_sided(&mut app.placements, &indices);
    if changed > 0 {
        commit_local_world_history(app, "Mark Texture Elements Double Sided", before);
    }
    app.status_message = format!(
        "Marked {changed} element{} referencing texture '{}' as double sided",
        if changed == 1 { "" } else { "s" },
        texture_name
    );
}

fn mark_placements_double_sided(placements: &mut [Placement], indices: &[usize]) -> usize {
    let mut changed = 0usize;
    for &index in indices {
        let Some(placement) = placements.get_mut(index) else {
            continue;
        };
        if is_default_world_placement(placement) {
            continue;
        }
        if placement_override_flag_enabled(placement, "double_sided") {
            continue;
        }
        set_placement_override_flag(placement, "double_sided", true);
        changed += 1;
    }
    changed
}

#[cfg(test)]
mod double_sided_texture_tests {
    use super::*;

    fn placement(attrs: &[(&str, &str)]) -> Placement {
        Placement {
            id: "test".to_string(),
            dff: "test.dff".to_string(),
            zone: "test".to_string(),
            tag: "object".to_string(),
            attrs: attrs
                .iter()
                .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
                .collect(),
            pos: V3::default(),
            rot: V3::default(),
        }
    }

    #[test]
    fn bulk_double_sided_marks_only_requested_unmarked_placements() {
        let mut placements = vec![
            placement(&[]),
            placement(&[("overrideFlags", "double_sided")]),
            placement(&[]),
        ];

        assert_eq!(mark_placements_double_sided(&mut placements, &[0, 1, 8]), 1);
        assert!(placement_override_flag_enabled(
            &placements[0],
            "double_sided"
        ));
        assert!(placement_override_flag_enabled(
            &placements[1],
            "double_sided"
        ));
        assert!(!placement_override_flag_enabled(
            &placements[2],
            "double_sided"
        ));
    }
}

fn duplicate_placement(source: &Placement) -> Placement {
    let mut copy = source.clone();
    sync_placement_attrs(&mut copy);
    copy
}

pub(crate) fn duplicate_selected(app: &mut AppState) {
    let indices = selected_editable_indices(app);
    if indices.is_empty() {
        return;
    }
    let before = local_world_history_snapshot(app, [], []);
    let mut new_selection = BTreeSet::new();
    for idx in indices {
        let Some(source) = app.placements.get(idx) else {
            continue;
        };
        let copy = duplicate_placement(source);
        app.placements.push(copy);
        app.element_states.push(ElementState::default());
        app.outliner_labels.push(None);
        let new_idx = app.placements.len() - 1;
        new_selection.insert(new_idx);
        app.selected = new_idx;
    }
    app.selected_elements = new_selection;
    app.selected_element_order = app.selected_elements.iter().copied().collect();
    commit_local_world_history(app, "Duplicate", before);
}

#[cfg(test)]
mod duplicate_placement_tests {
    use super::*;

    #[test]
    fn duplicate_keeps_the_source_position() {
        let source = Placement {
            id: "test_object".to_string(),
            dff: "test_object".to_string(),
            zone: "test".to_string(),
            tag: "object".to_string(),
            attrs: BTreeMap::new(),
            pos: V3 {
                x: 123.0,
                y: -456.0,
                z: 78.0,
            },
            rot: V3 {
                x: 10.0,
                y: 20.0,
                z: 30.0,
            },
        };

        let copy = duplicate_placement(&source);

        assert_eq!(copy.pos, source.pos);
        assert_eq!(copy.rot, source.rot);
    }
}

pub(crate) fn assigned_lod_indices_for_delete(app: &AppState, indices: &[usize]) -> Vec<usize> {
    let selected: HashSet<usize> = indices.iter().copied().collect();
    let mut lod_indices = Vec::new();
    let mut seen = HashSet::new();
    for idx in indices {
        let Some(placement) = app.placements.get(*idx) else {
            continue;
        };
        let Some(lod_parent) = placement
            .attrs
            .get("lodParent")
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        let Some((lod_idx, _)) = app.placements.iter().enumerate().find(|(lod_idx, lod)| {
            !selected.contains(lod_idx)
                && lod.id.eq_ignore_ascii_case(lod_parent)
                && !app
                    .element_states
                    .get(*lod_idx)
                    .is_some_and(|state| state.deleted)
        }) else {
            continue;
        };
        if seen.insert(lod_idx) {
            lod_indices.push(lod_idx);
        }
    }
    lod_indices
}

pub(crate) fn delete_elements_by_index(
    app: &mut AppState,
    indices: Vec<usize>,
    lod_indices: Vec<usize>,
    delete_lods: bool,
) {
    if indices.is_empty() {
        return;
    }
    let before = local_world_history_snapshot(app, [], []);
    let mut changed = false;
    let mut delete_indices = indices;
    if delete_lods {
        delete_indices.extend(lod_indices);
    }
    delete_indices.sort_unstable();
    delete_indices.dedup();
    for idx in delete_indices {
        if app
            .placements
            .get(idx)
            .is_some_and(is_default_world_placement)
            && !app.element_states.get(idx).is_some_and(|s| s.deleted)
        {
            remove_default_world_element(app, idx);
            changed = true;
            continue;
        }
        if let Some(state) = app.element_states.get_mut(idx) {
            if state.deleted {
                continue;
            }
            state.deleted = true;
            changed = true;
        }
    }
    if changed {
        commit_local_world_history(
            app,
            if delete_lods {
                "Delete with LOD"
            } else {
                "Delete"
            },
            before,
        );
    }
}

pub(crate) fn delete_selected(app: &mut AppState) {
    let indices = selected_indices(app);
    if indices.is_empty() {
        return;
    }
    let editable: Vec<_> = indices
        .iter()
        .copied()
        .filter(|&i| !is_default_world_placement(&app.placements[i]))
        .collect();
    let lod_indices = assigned_lod_indices_for_delete(app, &editable);
    if !lod_indices.is_empty() {
        let primary_label = if lod_indices.len() == 1 {
            "Delete LOD"
        } else {
            "Delete LODs"
        };
        let body = if lod_indices.len() == 1 {
            "Selected element has an assigned LOD. Delete it too?"
        } else {
            "Selected elements have assigned LODs. Delete them too?"
        };
        let detail = lod_indices
            .iter()
            .filter_map(|idx| app.placements.get(*idx))
            .map(|placement| placement.id.as_str())
            .take(3)
            .collect::<Vec<_>>()
            .join(", ");
        let extra = lod_indices.len().saturating_sub(3);
        let detail = if extra > 0 {
            format!("{detail}, +{extra} more")
        } else {
            detail
        };
        app.confirm_dialog = Some(ConfirmDialog {
            action: ConfirmAction::DeleteElements {
                indices: indices.clone(),
                lod_indices: lod_indices.clone(),
                delete_lods: true,
            },
            title: "Delete Assigned LOD".to_string(),
            body: body.to_string(),
            detail,
            primary_label: primary_label.to_string(),
            secondary_label: Some("Keep LOD".to_string()),
            secondary_action: Some(ConfirmAction::DeleteElements {
                indices,
                lod_indices,
                delete_lods: false,
            }),
        });
        return;
    }
    delete_elements_by_index(app, indices, Vec::new(), false);
}

pub(crate) fn restore_selected(app: &mut AppState) {
    let indices = selected_indices(app);
    if indices.is_empty() {
        return;
    }
    let before = local_world_history_snapshot(app, [], []);
    let mut changed = false;
    for idx in indices {
        if app
            .placements
            .get(idx)
            .is_some_and(is_default_world_placement)
        {
            restore_default_world_element(app, idx);
            changed = true;
            continue;
        }
        if let Some(state) = app.element_states.get_mut(idx) {
            if !state.deleted {
                continue;
            }
            state.deleted = false;
            changed = true;
        }
    }
    if changed {
        commit_local_world_history(app, "Restore", before);
    }
}

pub(crate) fn hide_selected(app: &mut AppState) {
    let indices = selected_indices(app);
    if indices.is_empty() {
        return;
    }
    let before = local_world_history_snapshot(app, [], []);
    let mut changed = 0usize;
    for idx in indices {
        if let Some(state) = app.element_states.get_mut(idx) {
            if state.deleted || state.hidden {
                continue;
            }
            state.hidden = true;
            changed += 1;
        }
    }
    if changed > 0 {
        commit_local_world_history(app, "Hide", before);
    }
    app.status_message = format!("Hidden {changed} asset(s)");
}

pub(crate) fn unhide_selected(app: &mut AppState) {
    let indices = selected_indices(app);
    if indices.is_empty() {
        return;
    }
    let before = local_world_history_snapshot(app, [], []);
    let mut changed = 0usize;
    for idx in indices {
        if let Some(state) = app.element_states.get_mut(idx) {
            if !state.hidden {
                continue;
            }
            state.hidden = false;
            changed += 1;
        }
    }
    if changed > 0 {
        commit_local_world_history(app, "Unhide", before);
    }
    app.status_message = format!("Unhidden {changed} asset(s)");
}

pub(crate) fn hide_everything_but_selected(app: &mut AppState) {
    let keep: BTreeSet<_> = selected_live_indices(app).into_iter().collect();
    if keep.is_empty() {
        app.status_message = "No visible selection to isolate".to_string();
        return;
    }
    let before = local_world_history_snapshot(app, [], []);
    let mut changed = 0usize;
    for (idx, state) in app.element_states.iter_mut().enumerate() {
        if state.deleted || keep.contains(&idx) || state.hidden {
            continue;
        }
        state.hidden = true;
        changed += 1;
    }
    if changed > 0 {
        commit_local_world_history(app, "Hide Everything But Selection", before);
    }
    app.status_message = format!("Isolated {} asset(s); hidden {changed}", keep.len());
}

pub(crate) fn unhide_all(app: &mut AppState) {
    let before = local_world_history_snapshot(app, [], []);
    let mut changed = 0usize;
    for state in &mut app.element_states {
        if state.hidden {
            state.hidden = false;
            changed += 1;
        }
    }
    if changed > 0 {
        commit_local_world_history(app, "Unhide All", before);
    }
    app.status_message = format!("Unhidden {changed} asset(s)");
}

pub(crate) fn camera_speed_for_tab(app: &AppState, tab: AppTab) -> f32 {
    match tab {
        AppTab::Vehicles => app.vehicle_camera_speed,
        AppTab::Editing => app.editing_camera_speed,
        _ => app.camera_speed,
    }
}

fn clamp_camera_speed_for_tab(speed: f32, tab: AppTab) -> f32 {
    match tab {
        AppTab::Vehicles => clamp_detail_camera_speed(speed),
        AppTab::Editing => clamp_editing_camera_speed(speed),
        _ => clamp_camera_speed(speed),
    }
}

pub(crate) fn set_camera_speed_for_tab(app: &mut AppState, tab: AppTab, speed: f32) {
    let speed = clamp_camera_speed_for_tab(speed, tab);
    let target = match tab {
        AppTab::Vehicles => &mut app.vehicle_camera_speed,
        AppTab::Editing => &mut app.editing_camera_speed,
        _ => &mut app.camera_speed,
    };
    if (*target - speed).abs() < 0.01 {
        return;
    }
    *target = speed;
    app.status_message = format!("Camera speed {:.0}", speed);
}

pub(crate) fn set_camera_speed(app: &mut AppState, speed: f32) {
    let active_tab = app.active_tab;
    set_camera_speed_for_tab(app, active_tab, speed);
}

pub(crate) fn camera_speed_after_wheel(speed: f32, wheel: f32, tab: AppTab) -> f32 {
    let factor = if matches!(tab, AppTab::Vehicles | AppTab::Editing) {
        DETAIL_CAMERA_SPEED_WHEEL_FACTOR
    } else {
        CAMERA_SPEED_WHEEL_FACTOR
    };
    clamp_camera_speed_for_tab(speed * factor.powf(wheel), tab)
}

#[cfg(test)]
#[test]
fn camera_wheel_adjustment_handles_both_directions() {
    let speed = 360.0;
    let faster = camera_speed_after_wheel(speed, 1.0, AppTab::Preview);
    let slower = camera_speed_after_wheel(speed, -1.0, AppTab::Preview);
    assert!(faster > speed);
    assert!(slower < speed);
    assert!((camera_speed_after_wheel(faster, -1.0, AppTab::Preview) - speed).abs() < 0.001);
}

#[cfg(test)]
#[test]
fn camera_speed_defaults_match_viewport_scope() {
    assert_eq!(DEFAULT_CAMERA_SPEED, 50.0);
    assert_eq!(DEFAULT_DETAIL_CAMERA_SPEED, 20.0);
    assert_eq!(MIN_DETAIL_CAMERA_SPEED, 2.0);
    assert_eq!(MIN_EDITING_CAMERA_SPEED, 2.0);
    assert!(
        camera_speed_after_wheel(20.0, 1.0, AppTab::Editing)
            < camera_speed_after_wheel(20.0, 1.0, AppTab::Preview)
    );
    assert_eq!(camera_speed_after_wheel(2.0, -1.0, AppTab::Vehicles), 2.0);
    assert_eq!(camera_speed_after_wheel(2.0, -1.0, AppTab::Editing), 2.0);
}

pub(crate) fn set_camera_rotation_speed(app: &mut AppState, speed: f32) {
    let speed = clamp_camera_rotation_speed(speed);
    if (app.camera_rotation_speed - speed).abs() < 0.001 {
        return;
    }
    app.camera_rotation_speed = speed;
    save_camera_rotation_speed_preference(speed);
    app.status_message = format!("Camera rotation speed {speed:.2}x");
}

pub(crate) fn set_gizmo_scale(app: &mut AppState, scale: f32) {
    let scale = clamp_gizmo_scale(scale);
    if (app.gizmo_scale - scale).abs() < 0.001 {
        return;
    }
    app.gizmo_scale = scale;
    save_gizmo_scale_preference(scale);
    app.status_message = format!("Gimbal size {scale:.2}x");
}

pub(crate) fn save_lights_for_app(app: &mut AppState) {
    apply_pending_inspector_edit_for_save(app);
    // EagleScene is a shared, potentially large document. Route this through
    // the background Resource Save transaction instead of parsing and
    // rewriting it on the input/render thread.
    save_scene(app);
}

pub(crate) fn mark_lights_changed(app: &mut AppState) {
    app.status_message =
        "Light changes pending. Save, Save As, or Save WIP to keep them.".to_string();
}

pub(crate) fn import_lights_from_path(app: &mut AppState, path: &Path) {
    let attr_re = Regex::new(r#"([A-Za-z_][A-Za-z0-9_]*)="([^"]*)""#).unwrap();
    match load_lights_xml(path, &attr_re) {
        Ok(lights) => {
            if let Some(drag_before) = app.light_color_drag_before.take() {
                commit_light_history(app, "Edit Light Color", drag_before);
            }
            let before = light_history_snapshot(app);
            app.lights = lights;
            app.selected_light = app.selected_light.min(app.lights.len().saturating_sub(1));
            commit_light_history(app, "Import Light List", before);
            app.status_message = format!(
                "Imported {} light(s) from {}. Changes are pending.",
                app.lights.len(),
                path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("Light_List.xml")
            );
        }
        Err(err) => app.status_message = format!("Failed to import lights: {err}"),
    }
}

pub(crate) fn update_dropped_resource_files(app: &mut AppState) -> bool {
    let editing_txd = if app.active_tab == AppTab::Editing {
        match app.editing.asset.as_ref() {
            Some(EditingAsset::Txd(txd)) => Some(txd.name.clone()),
            _ => None,
        }
    } else {
        None
    };
    let mut dropped_textures = Vec::new();
    let dropped_files = get_dropped_files();
    let had_drop = !dropped_files.is_empty();
    for dropped in dropped_files {
        if let Some(path) = dropped.path {
            if path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("dff"))
            {
                if app.active_tab == AppTab::Editing || app.active_tab == AppTab::Vehicles {
                    app.status_message = "Drop new scene assets from a game-world tab".to_string();
                    continue;
                }
                // A native DFF drop is a complete import request of its own.
                // It must not remain hidden behind, or later be overwritten by,
                // a file-picker import modal that was just dismissed.
                app.import_asset_dialog = None;
                app.dff_picker_rx = None;
                let stem = path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or("new_asset");
                let mut id = stem.to_string();
                let mut suffix = 2usize;
                while app.definitions.contains_key(&id) {
                    id = format!("{stem}_{suffix}");
                    suffix += 1;
                }
                let mouse: Vec2 = mouse_position().into();
                let position = asset_browser_drag_position(app, editor_viewport_rect(), mouse);
                let textures = automatic_texture_folder(&path);
                match position {
                    Some(position) => {
                        if let Err(err) =
                            import_new_asset(app, &path, textures.as_deref(), &id, Some(position))
                        {
                            app.status_message = format!("Import failed: {err}");
                        }
                    }
                    None => {
                        app.status_message =
                            "Drop the DFF inside the 3D viewport to place it".to_string();
                    }
                }
            } else if editing_txd.is_some() && path.is_dir() {
                dropped_textures.extend(texture_paths_in_folder(&path));
            } else if editing_txd.is_some()
                && path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
            {
                dropped_textures.push(path);
            } else if path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("blend"))
            {
                start_blender_import(app, path);
            } else if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case("Light_List.xml"))
            {
                import_lights_from_path(app, &path);
            } else if path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("xml"))
            {
                import_lights_from_path(app, &path);
            }
        }
    }
    if let Some(entry_name) = editing_txd
        && !dropped_textures.is_empty()
    {
        editing_import_textures_from_paths(app, entry_name, None, dropped_textures);
    }
    had_drop
}

pub(crate) fn add_light(app: &mut AppState) {
    add_light_at(app, Vec3::ZERO);
}

pub(crate) fn add_light_at(app: &mut AppState, position: Vec3) {
    let before = light_history_snapshot(app);
    app.lights
        .push(default_editor_light(app.lights.len(), position));
    app.selected_light = app.lights.len() - 1;
    mark_lights_changed(app);
    commit_light_history(app, "Add Light", before);
}

fn default_editor_light(existing_light_count: usize, position: Vec3) -> EditorLight {
    EditorLight {
        name: format!("Light {}", existing_light_count + 1),
        attached_to: None,
        kind: LightKind::Point,
        profile: LightProfile::Both,
        position: from_mq(position),
        direction: V3 {
            x: 0.0,
            y: 0.0,
            z: -1.0,
        },
        color: V3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        },
        temperature: 6500.0,
        use_temperature: false,
        intensity: 1.0,
        radius: DEFAULT_EDITOR_LIGHT_RADIUS,
        casts_shadow: false,
        point_lobe: PointLightLobe::Omni,
    }
}

pub(crate) fn add_light_to_selected_instance_at(app: &mut AppState, position: Vec3) -> bool {
    let Some(placement) = app.placements.get(app.selected).cloned() else {
        app.status_message = "ALT-select a scene instance before adding the light".to_string();
        return false;
    };
    if app
        .element_states
        .get(app.selected)
        .is_some_and(|state| state.deleted)
    {
        return false;
    }

    let before = light_history_snapshot(app);
    let mut light = default_editor_light(app.lights.len(), position);
    let inv = placement_matrix(&placement).inverse();
    light.position = from_mq(inv.transform_point3(position));
    light.direction = from_mq(
        inv.transform_vector3(to_mq(light.direction))
            .normalize_or_zero(),
    );
    light.attached_to = Some(placement.id.clone());
    app.lights.push(light);
    app.selected_light = app.lights.len() - 1;
    mark_lights_changed(app);
    commit_light_history(app, "Add Light to Instance", before);
    app.status_message = format!("Light added to every instance of {}", placement.id);
    true
}

pub(crate) fn attach_selected_light_to_instance(app: &mut AppState) -> bool {
    let Some(placement) = app.placements.get(app.selected).cloned() else {
        app.status_message = "ALT-select a scene instance before attaching the light".to_string();
        return false;
    };
    if app
        .element_states
        .get(app.selected)
        .is_some_and(|state| state.deleted)
    {
        return false;
    }
    let Some(light) = app.lights.get(app.selected_light).cloned() else {
        return false;
    };
    let before = light_history_snapshot(app);
    let (world_position, world_direction) = light_world_transform(
        &light,
        light_reference_placement(app, &light).map(|(_, placement)| placement),
    );
    let inv = placement_matrix(&placement).inverse();
    if let Some(light) = app.lights.get_mut(app.selected_light) {
        light.position = from_mq(inv.transform_point3(world_position));
        light.direction = from_mq(inv.transform_vector3(world_direction).normalize_or_zero());
        light.attached_to = Some(placement.id.clone());
    }
    mark_lights_changed(app);
    commit_light_history(app, "Add Light to Instance", before);
    app.status_message = format!("Light attached to every instance of {}", placement.id);
    true
}

pub(crate) fn duplicate_selected_light_with_offset(
    app: &mut AppState,
    offset: Vec3,
    label: &str,
) -> bool {
    let Some(source) = app.lights.get(app.selected_light).cloned() else {
        return false;
    };
    let before = light_history_snapshot(app);
    let mut copy = source;
    copy.name = format!("{} Copy", copy.name);
    copy.position = from_mq(to_mq(copy.position) + offset);
    app.lights.push(copy);
    app.selected_light = app.lights.len() - 1;
    mark_lights_changed(app);
    commit_light_history(app, label, before);
    true
}

pub(crate) fn duplicate_selected_light(app: &mut AppState) {
    duplicate_selected_light_with_offset(app, vec3(32.0, 32.0, 0.0), "Duplicate Light");
}

pub(crate) fn delete_selected_light(app: &mut AppState) {
    if app.selected_light >= app.lights.len() {
        return;
    }
    let before = light_history_snapshot(app);
    let idx = app.selected_light;
    app.lights.remove(idx);
    app.selected_light = idx.min(app.lights.len().saturating_sub(1));
    mark_lights_changed(app);
    commit_light_history(app, "Delete Light", before);
}

#[cfg(test)]
mod light_instance_tests {
    use super::*;

    #[test]
    fn new_lights_use_the_file_format_default_radius() {
        let light = default_editor_light(0, Vec3::ZERO);
        assert_eq!(light.radius, DEFAULT_EDITOR_LIGHT_RADIUS);
    }

    #[test]
    fn attached_light_transform_follows_instance_position_and_rotation() {
        let placement = Placement {
            id: "streetlight".to_string(),
            dff: "streetlight".to_string(),
            zone: "test".to_string(),
            tag: "object".to_string(),
            attrs: BTreeMap::new(),
            pos: V3 {
                x: 100.0,
                y: 200.0,
                z: 5.0,
            },
            rot: V3 {
                x: 0.0,
                y: 0.0,
                z: 90.0,
            },
        };
        let light = EditorLight {
            name: "Lamp".to_string(),
            attached_to: Some("streetlight".to_string()),
            kind: LightKind::Point,
            profile: LightProfile::Both,
            position: V3 {
                x: 10.0,
                y: 0.0,
                z: 3.0,
            },
            direction: V3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            color: V3::default(),
            temperature: 6500.0,
            use_temperature: false,
            intensity: 1.0,
            radius: 50.0,
            casts_shadow: false,
            point_lobe: PointLightLobe::Omni,
        };

        let (position, direction) = light_world_transform(&light, Some(&placement));
        assert!((position - vec3(100.0, 210.0, 8.0)).length() < 0.001);
        assert!((direction - Vec3::Y).length() < 0.001);
    }
}

#[cfg(test)]
mod instance_lod_removal_tests {
    use super::*;

    fn placement(id: &str, lod_parent: Option<&str>, unique_id: Option<&str>) -> Placement {
        let mut attrs = BTreeMap::new();
        if let Some(lod_parent) = lod_parent {
            attrs.insert("lodParent".to_string(), lod_parent.to_string());
        }
        if let Some(unique_id) = unique_id {
            attrs.insert("uniqueID".to_string(), unique_id.to_string());
        }
        Placement {
            id: id.to_string(),
            dff: id.to_string(),
            zone: "test".to_string(),
            tag: "object".to_string(),
            attrs,
            pos: V3::default(),
            rot: V3::default(),
        }
    }

    #[test]
    fn removes_assignments_and_every_unshared_repeated_lod() {
        let placements = vec![
            placement("barrier", Some("lod_barrier"), None),
            placement("barrier", Some("lod_barrier"), None),
            placement("lod_barrier", None, None),
            placement("lod_barrier", None, None),
        ];
        let states = vec![ElementState::default(); placements.len()];

        let plan = plan_instance_lod_removal(&placements, &states, "barrier");

        assert_eq!(plan.instance_count, 2);
        assert_eq!(plan.detail_indices, vec![0, 1]);
        assert_eq!(plan.lod_indices, vec![2, 3]);
    }

    #[test]
    fn clear_all_removes_self_links_external_links_and_lod_elements() {
        let mut placements = vec![
            placement("building", Some("lod_building"), None),
            placement("tree", Some("self"), None),
            placement("lod_building", None, None),
            placement("lod_stray", None, None),
            placement("bench", None, None),
        ];
        let mut states = vec![ElementState::default(); placements.len()];

        let (cleared, removed) = clear_all_lods_in_world(&mut placements, &mut states);

        assert_eq!((cleared, removed), (2, 2));
        assert!(
            placements
                .iter()
                .all(|item| !item.attrs.contains_key("lodParent"))
        );
        assert!(!states[0].deleted);
        assert!(!states[1].deleted);
        assert!(states[2].deleted);
        assert!(states[3].deleted);
        assert!(!states[4].deleted);
    }

    #[test]
    fn clear_all_collects_only_lod_dff_col_and_txd_assets() {
        let placements = vec![
            placement("building", Some("lod_building"), None),
            placement("lod_building", None, None),
            placement("bench", None, None),
        ];
        let states = vec![ElementState::default(); placements.len()];
        let definitions = HashMap::from([
            (
                "lod_building".to_string(),
                Definition {
                    id: "lod_building".to_string(),
                    zone: "test".to_string(),
                    attrs: BTreeMap::from([
                        ("dff".to_string(), "generated_visual".to_string()),
                        ("col".to_string(), "generated_collision".to_string()),
                        ("txd".to_string(), "world_lod".to_string()),
                    ]),
                },
            ),
            (
                "bench".to_string(),
                Definition {
                    id: "bench".to_string(),
                    zone: "test".to_string(),
                    attrs: BTreeMap::from([
                        ("dff".to_string(), "bench".to_string()),
                        ("txd".to_string(), "shared".to_string()),
                    ]),
                },
            ),
        ]);

        let candidates = collect_lod_asset_candidates(&placements, &states, &definitions);

        assert_eq!(
            candidates.definitions,
            HashSet::from(["lod_building".to_string()])
        );
        assert_eq!(
            candidates.dffs,
            HashSet::from(["generated_visual.dff".to_string()])
        );
        assert_eq!(
            candidates.cols,
            HashSet::from(["generated_collision.col".to_string()])
        );
        assert_eq!(
            candidates.txds,
            HashSet::from(["world_lod.txd".to_string()])
        );
    }

    #[test]
    fn clear_all_uses_the_lod_dff_stem_for_implicit_collision() {
        let placements = vec![placement("lod_bridge", None, None)];
        let states = vec![ElementState::default()];
        let definitions = HashMap::from([(
            "lod_bridge".to_string(),
            Definition {
                id: "lod_bridge".to_string(),
                zone: "test".to_string(),
                attrs: BTreeMap::from([("dff".to_string(), "bridge_low".to_string())]),
            },
        )]);

        let candidates = collect_lod_asset_candidates(&placements, &states, &definitions);

        assert_eq!(
            candidates.dffs,
            HashSet::from(["bridge_low.dff".to_string()])
        );
        assert_eq!(
            candidates.cols,
            HashSet::from(["bridge_low.col".to_string()])
        );
    }

    #[test]
    fn keeps_a_parent_still_referenced_by_another_model() {
        let placements = vec![
            placement("barrier", Some("lod_shared"), None),
            placement("bench", Some("lod_shared"), None),
            placement("lod_shared", None, None),
        ];
        let states = vec![ElementState::default(); placements.len()];

        let plan = plan_instance_lod_removal(&placements, &states, "barrier");

        assert_eq!(plan.detail_indices, vec![0]);
        assert!(plan.lod_indices.is_empty());
    }

    #[test]
    fn removes_only_the_unique_id_paired_lod_from_a_shared_parent_name() {
        let placements = vec![
            placement("barrier", Some("lod_shared"), Some("41")),
            placement("bench", Some("lod_shared"), Some("42")),
            placement("lod_shared", None, Some("41")),
            placement("lod_shared", None, Some("42")),
        ];
        let states = vec![ElementState::default(); placements.len()];

        let plan = plan_instance_lod_removal(&placements, &states, "barrier");

        assert_eq!(plan.detail_indices, vec![0]);
        assert_eq!(plan.lod_indices, vec![2]);
    }

    #[test]
    fn clears_self_lod_without_deleting_an_element() {
        let placements = vec![placement("barrier", Some("self"), None)];
        let states = vec![ElementState::default()];

        let plan = plan_instance_lod_removal(&placements, &states, "barrier");

        assert_eq!(plan.detail_indices, vec![0]);
        assert!(plan.lod_indices.is_empty());
    }

    #[test]
    fn ignores_deleted_instances_and_deleted_lod_elements() {
        let placements = vec![
            placement("barrier", Some("lod_barrier"), None),
            placement("barrier", Some("lod_barrier"), None),
            placement("lod_barrier", None, None),
            placement("lod_barrier", None, None),
        ];
        let states = vec![
            ElementState::default(),
            ElementState {
                deleted: true,
                hidden: false,
            },
            ElementState::default(),
            ElementState {
                deleted: true,
                hidden: false,
            },
        ];

        let plan = plan_instance_lod_removal(&placements, &states, "barrier");

        assert_eq!(plan.instance_count, 1);
        assert_eq!(plan.detail_indices, vec![0]);
        assert_eq!(plan.lod_indices, vec![2]);
    }

    #[test]
    fn texture_name_matching_is_case_insensitive_and_element_scoped() {
        let candidates = vec![
            TextureMatchCandidate {
                placement: 3,
                textures: vec![("Road_Asphalt".to_string(), Some([1, 2]))],
            },
            TextureMatchCandidate {
                placement: 8,
                textures: vec![
                    ("grass".to_string(), Some([3, 4])),
                    ("road_asphalt".to_string(), Some([5, 6])),
                ],
            },
            TextureMatchCandidate {
                placement: 13,
                textures: vec![("road_cracked".to_string(), Some([1, 2]))],
            },
        ];

        assert_eq!(
            texture_match_indices(&candidates, TextureMatchMode::Name, " road_asphalt ", None),
            vec![3, 8]
        );
    }

    #[test]
    fn texture_content_matching_ignores_names_and_requires_exact_fingerprint() {
        let candidates = vec![
            TextureMatchCandidate {
                placement: 2,
                textures: vec![("road".to_string(), Some([10, 20]))],
            },
            TextureMatchCandidate {
                placement: 4,
                textures: vec![("landmass".to_string(), Some([10, 20]))],
            },
            TextureMatchCandidate {
                placement: 6,
                textures: vec![("road".to_string(), Some([10, 21]))],
            },
            TextureMatchCandidate {
                placement: 7,
                textures: vec![("unresolved".to_string(), None)],
            },
        ];

        assert_eq!(
            texture_match_indices(
                &candidates,
                TextureMatchMode::Content,
                "unused",
                Some([10, 20])
            ),
            vec![2, 4]
        );
        assert!(
            texture_match_indices(&candidates, TextureMatchMode::Content, "unused", None)
                .is_empty()
        );
    }
}

#[cfg(test)]
mod water_texture_conversion_tests {
    use super::*;

    fn raw_with_water_height(high_corner_z: f32) -> RawMesh {
        RawMesh {
            vertices: vec![
                V3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                V3 {
                    x: 10.0,
                    y: 0.0,
                    z: 0.0,
                },
                V3 {
                    x: 0.0,
                    y: 20.0,
                    z: 0.0,
                },
                V3 {
                    x: 10.0,
                    y: 20.0,
                    z: high_corner_z,
                },
                V3 {
                    x: -2.0,
                    y: -2.0,
                    z: -1.0,
                },
                V3 {
                    x: -1.0,
                    y: -2.0,
                    z: -1.0,
                },
                V3 {
                    x: -2.0,
                    y: -1.0,
                    z: -1.0,
                },
            ],
            triangles: vec![
                Tri {
                    a: 0,
                    b: 1,
                    c: 2,
                    material: 0,
                },
                Tri {
                    a: 1,
                    b: 3,
                    c: 2,
                    material: 0,
                },
                Tri {
                    a: 4,
                    b: 5,
                    c: 6,
                    material: 1,
                },
            ],
            material_textures: vec!["water".to_string(), "ground".to_string()],
            materials: vec![RawMaterial::default(), RawMaterial::default()],
            ..RawMesh::default()
        }
    }

    #[test]
    fn flat_material_is_encapsulated_by_world_space_water_bounds() {
        let raw = raw_with_water_height(0.0);
        let matrix = Mat4::from_translation(vec3(100.0, 200.0, 7.0)).to_cols_array();

        let plane = water_plane_for_material_instance(&raw, 0, &matrix).unwrap();
        let (min_x, min_y, max_x, max_y, z) = water_plane_bounds(&plane);

        assert_eq!((min_x, min_y, max_x, max_y), (100.0, 200.0, 110.0, 220.0));
        assert!((z - 7.0).abs() < 0.0001);
    }

    #[test]
    fn very_low_height_variance_is_accepted() {
        let raw = raw_with_water_height(WATER_TEXTURE_MAX_Z_VARIANCE * 0.8);

        assert!(
            water_plane_for_material_instance(&raw, 0, &Mat4::IDENTITY.to_cols_array()).is_ok()
        );
    }

    #[test]
    fn material_above_height_tolerance_is_rejected_without_a_plane() {
        let raw = raw_with_water_height(WATER_TEXTURE_MAX_Z_VARIANCE + 0.001);

        let error =
            match water_plane_for_material_instance(&raw, 0, &Mat4::IDENTITY.to_cols_array()) {
                Ok(_) => panic!("surface above the Z tolerance unexpectedly produced water"),
                Err(error) => error,
            };

        assert!(error.contains("not flat and horizontal"));
    }

    #[test]
    fn successful_conversion_removes_only_the_water_material_faces() {
        let source_bytes = write_normalized_dff(&raw_with_water_height(0.0), "pond").unwrap();
        let result = convert_water_texture(
            WaterTextureDffSource::Bytes(source_bytes),
            "pond.dff".to_string(),
            0,
            "water".to_string(),
            vec![Mat4::IDENTITY.to_cols_array()],
            DffWriteOptions::default(),
        )
        .unwrap();

        assert_eq!(result.removed_faces, 2);
        assert_eq!(result.water_planes.len(), 1);
        assert_eq!(result.raw.triangles.len(), 1);
        assert_eq!(result.raw.triangles[0].material, 1);
        assert_eq!(result.raw.vertices.len(), 3);
        assert_eq!(result.raw.components.len(), 1);
        assert_eq!(result.raw.components[0].tri_start, 0);
        assert_eq!(result.raw.components[0].tri_end, 1);
        assert!(!result.dff_bytes.is_empty());
        let round_trip = parse_dff_mesh(&result.dff_bytes);
        assert_eq!(round_trip.triangles.len(), 1);
        assert_eq!(round_trip.triangles[0].material, 1);
    }
}
