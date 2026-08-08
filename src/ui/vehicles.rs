use super::super::*;

const VEHICLE_ROW_H: f32 = 34.0;
const VEHICLE_COMPONENT_ROW_H: f32 = 28.0;
const VEHICLE_PREVIEW_H: f32 = 148.0;
const VEHICLE_DICTIONARY_ROW_H: f32 = 42.0;
const VEHICLE_BUILD_GENERAL_FIELDS: usize = 6;
const VEHICLE_BUILD_DROPDOWN_ROW_H: f32 = 28.0;
const VEHICLE_BUILD_DROPDOWN_VISIBLE: usize = 8;
const VEHICLE_COLLISION_COPY_ROW_H: f32 = 38.0;

pub(crate) fn vehicle_panel_rect() -> Rect {
    Rect::new(
        10.0,
        TOP_H + 12.0,
        PANEL_W - 20.0,
        screen_height() - TOP_H - STATUS_H - 24.0,
    )
}

pub(crate) fn vehicle_search_rect() -> Rect {
    let rect = vehicle_panel_rect();
    Rect::new(rect.x + 14.0, rect.y + 64.0, rect.w - 28.0, 30.0)
}

pub(crate) fn vehicle_filter_rect(slot: usize) -> Rect {
    let rect = vehicle_panel_rect();
    Rect::new(
        rect.x + 14.0 + slot as f32 * 98.0,
        rect.y + 102.0,
        88.0,
        30.0,
    )
}

pub(crate) fn vehicle_action_rect(slot: usize) -> Rect {
    let rect = vehicle_panel_rect();
    let bw = (rect.w - 42.0) / 2.0;
    let x = rect.x + 14.0 + (slot % 2) as f32 * (bw + 14.0);
    let y = rect.y + 140.0 + (slot / 2) as f32 * 38.0;
    Rect::new(x, y, bw, 32.0)
}

pub(crate) fn vehicle_dictionary_dialog_rect() -> Rect {
    let width = 760.0_f32.min(screen_width() - 80.0).max(420.0);
    let height = 480.0_f32.min(screen_height() - 80.0).max(300.0);
    Rect::new(
        (screen_width() - width) * 0.5,
        (screen_height() - height) * 0.5,
        width,
        height,
    )
}

pub(crate) fn vehicle_dictionary_list_rect() -> Rect {
    let dialog = vehicle_dictionary_dialog_rect();
    Rect::new(
        dialog.x + 24.0,
        dialog.y + 82.0,
        dialog.w - 48.0,
        dialog.h - 148.0,
    )
}

pub(crate) fn vehicle_dictionary_add_rect() -> Rect {
    let dialog = vehicle_dictionary_dialog_rect();
    Rect::new(
        dialog.x + dialog.w - 222.0,
        dialog.y + dialog.h - 50.0,
        94.0,
        32.0,
    )
}

pub(crate) fn vehicle_dictionary_close_rect() -> Rect {
    let dialog = vehicle_dictionary_dialog_rect();
    Rect::new(
        dialog.x + dialog.w - 116.0,
        dialog.y + dialog.h - 50.0,
        88.0,
        32.0,
    )
}

pub(crate) fn vehicle_dictionary_remove_rect(visible_row: usize) -> Rect {
    let list = vehicle_dictionary_list_rect();
    Rect::new(
        list.x + list.w - 92.0,
        list.y + 5.0 + visible_row as f32 * VEHICLE_DICTIONARY_ROW_H,
        78.0,
        30.0,
    )
}

pub(crate) fn vehicle_list_rect() -> Rect {
    let rect = vehicle_panel_rect();
    Rect::new(rect.x + 14.0, rect.y + 294.0, rect.w - 28.0, rect.h - 372.0)
}

fn vehicle_build_dialog_rect() -> Rect {
    let width = 920.0_f32.min(screen_width() - 60.0).max(620.0);
    let height = 720.0_f32.min(screen_height() - 50.0).max(560.0);
    Rect::new(
        (screen_width() - width) * 0.5,
        (screen_height() - height) * 0.5,
        width,
        height,
    )
}

fn vehicle_build_field_rect(index: usize) -> Rect {
    let dialog = vehicle_build_dialog_rect();
    if index < VEHICLE_BUILD_GENERAL_FIELDS {
        let column = index % 2;
        let row = index / 2;
        let width = (dialog.w - 70.0) * 0.5;
        return Rect::new(
            dialog.x + 24.0 + column as f32 * (width + 22.0),
            dialog.y + 92.0 + row as f32 * 44.0,
            width,
            32.0,
        );
    }
    let handling = index - VEHICLE_BUILD_GENERAL_FIELDS;
    let rows = 11;
    let column = handling / rows;
    let row = handling % rows;
    let width = (dialog.w - 70.0) * 0.5;
    Rect::new(
        dialog.x + 24.0 + column as f32 * (width + 22.0),
        dialog.y + 292.0 + row as f32 * 32.0,
        width,
        27.0,
    )
}

fn vehicle_build_value_rect(index: usize) -> Rect {
    let row = vehicle_build_field_rect(index);
    let label_width = if index < VEHICLE_BUILD_GENERAL_FIELDS {
        148.0
    } else {
        218.0
    };
    Rect::new(
        row.x + label_width,
        row.y,
        (row.w - label_width).max(80.0),
        row.h,
    )
}

fn vehicle_build_mode_rect(new_vehicle: bool) -> Rect {
    let dialog = vehicle_build_dialog_rect();
    Rect::new(
        dialog.x + if new_vehicle { 24.0 } else { 180.0 },
        dialog.y + 224.0,
        144.0,
        32.0,
    )
}

fn vehicle_build_dropdown_option_rect(field: usize, row: usize, visible: usize) -> Rect {
    let value = vehicle_build_value_rect(field);
    let popup_h = visible as f32 * VEHICLE_BUILD_DROPDOWN_ROW_H;
    let below_y = value.y + value.h + 4.0;
    let y = if below_y + popup_h
        < vehicle_build_dialog_rect().y + vehicle_build_dialog_rect().h - 12.0
    {
        below_y + row as f32 * VEHICLE_BUILD_DROPDOWN_ROW_H
    } else {
        value.y - 4.0 - popup_h + row as f32 * VEHICLE_BUILD_DROPDOWN_ROW_H
    };
    Rect::new(value.x, y, value.w, VEHICLE_BUILD_DROPDOWN_ROW_H)
}

fn vehicle_category_manage_rect(visible_categories: usize) -> Rect {
    vehicle_build_dropdown_option_rect(1, visible_categories, visible_categories + 1)
}

fn vehicle_category_manager_rect() -> Rect {
    let width = 520.0_f32.min(screen_width() - 80.0).max(380.0);
    let height = 460.0_f32.min(screen_height() - 80.0).max(340.0);
    Rect::new(
        (screen_width() - width) * 0.5,
        (screen_height() - height) * 0.5,
        width,
        height,
    )
}

fn vehicle_category_manager_input_rect() -> Rect {
    let panel = vehicle_category_manager_rect();
    Rect::new(panel.x + 24.0, panel.y + 72.0, panel.w - 142.0, 32.0)
}

fn vehicle_category_manager_add_rect() -> Rect {
    let input = vehicle_category_manager_input_rect();
    Rect::new(input.x + input.w + 12.0, input.y, 82.0, 32.0)
}

fn vehicle_category_manager_list_rect() -> Rect {
    let panel = vehicle_category_manager_rect();
    Rect::new(
        panel.x + 24.0,
        panel.y + 122.0,
        panel.w - 48.0,
        panel.h - 188.0,
    )
}

fn vehicle_category_manager_remove_rect(row: usize) -> Rect {
    let list = vehicle_category_manager_list_rect();
    Rect::new(
        list.x + list.w - 88.0,
        list.y + 5.0 + row as f32 * 38.0,
        76.0,
        28.0,
    )
}

fn vehicle_category_manager_done_rect() -> Rect {
    let panel = vehicle_category_manager_rect();
    Rect::new(
        panel.x + panel.w - 112.0,
        panel.y + panel.h - 48.0,
        88.0,
        32.0,
    )
}

fn vehicle_build_submit_rect() -> Rect {
    let dialog = vehicle_build_dialog_rect();
    Rect::new(
        dialog.x + dialog.w - 236.0,
        dialog.y + dialog.h - 48.0,
        104.0,
        32.0,
    )
}

fn vehicle_build_cancel_rect() -> Rect {
    let dialog = vehicle_build_dialog_rect();
    Rect::new(
        dialog.x + dialog.w - 120.0,
        dialog.y + dialog.h - 48.0,
        92.0,
        32.0,
    )
}

pub(crate) fn vehicle_details_rect() -> Rect {
    Rect::new(
        screen_width() - RIGHT_PANEL_W + 12.0,
        TOP_H + 12.0,
        RIGHT_PANEL_W - 24.0,
        screen_height() - TOP_H - STATUS_H - 24.0,
    )
}

pub(crate) fn vehicle_preview_viewport_rect(app: &AppState) -> Rect {
    if app.vehicle_browser.photo_mode {
        return Rect::new(
            0.0,
            TOP_H,
            screen_width().max(80.0),
            (screen_height() - TOP_H - STATUS_H).max(80.0),
        );
    }
    let left = vehicle_panel_rect();
    let right = vehicle_details_rect();
    Rect::new(
        left.x + left.w + 8.0,
        TOP_H + 12.0,
        (right.x - left.x - left.w - 16.0).max(80.0),
        screen_height() - TOP_H - STATUS_H - 24.0,
    )
}

pub(crate) fn vehicle_photo_mode_rect(app: &AppState) -> Rect {
    let viewport = vehicle_preview_viewport_rect(app);
    Rect::new(
        viewport.x + viewport.w - 154.0,
        viewport.y + 14.0,
        140.0,
        34.0,
    )
}

pub(crate) fn vehicle_toggle_rect(slot: usize) -> Rect {
    let rect = vehicle_details_rect();
    let bw = (rect.w - 42.0) / 2.0;
    let x = rect.x + 14.0 + (slot % 2) as f32 * (bw + 14.0);
    let y = rect.y + 90.0 + (slot / 2) as f32 * 38.0;
    Rect::new(x, y, bw, 32.0)
}

pub(crate) fn vehicle_show_all_rect() -> Rect {
    let list = vehicle_component_list_rect();
    Rect::new(list.x + list.w - 104.0, list.y - 26.0, 48.0, 24.0)
}

pub(crate) fn vehicle_hide_all_rect() -> Rect {
    let list = vehicle_component_list_rect();
    Rect::new(list.x + list.w - 48.0, list.y - 26.0, 48.0, 24.0)
}

pub(crate) fn vehicle_component_list_rect() -> Rect {
    let rect = vehicle_details_rect();
    Rect::new(
        rect.x + 14.0,
        rect.y + 382.0,
        rect.w - 28.0,
        rect.h - 382.0 - VEHICLE_PREVIEW_H - 60.0,
    )
}

pub(crate) fn vehicle_body_color_rect(slot: usize) -> Rect {
    let rect = vehicle_details_rect();
    let bw = (rect.w - 42.0) / 2.0;
    Rect::new(
        rect.x + 14.0 + slot as f32 * (bw + 14.0),
        rect.y + 244.0,
        bw,
        34.0,
    )
}

pub(crate) fn vehicle_body_color_channel_rect(slot: usize, channel: usize) -> Rect {
    let color = vehicle_body_color_rect(slot);
    Rect::new(
        color.x,
        color.y + 48.0 + channel as f32 * 20.0,
        color.w,
        14.0,
    )
}

pub(crate) fn vehicle_copy_collision_rect() -> Rect {
    let rect = vehicle_details_rect();
    Rect::new(rect.x + 14.0, rect.y + 298.0, rect.w - 28.0, 32.0)
}

fn vehicle_collision_copy_dialog_rect() -> Rect {
    let width = 700.0_f32.min(screen_width() - 80.0).max(460.0);
    let height = 570.0_f32.min(screen_height() - 80.0).max(380.0);
    Rect::new(
        (screen_width() - width) * 0.5,
        (screen_height() - height) * 0.5,
        width,
        height,
    )
}

fn vehicle_collision_copy_search_rect() -> Rect {
    let dialog = vehicle_collision_copy_dialog_rect();
    Rect::new(dialog.x + 24.0, dialog.y + 108.0, dialog.w - 48.0, 32.0)
}

fn vehicle_collision_copy_list_rect() -> Rect {
    let dialog = vehicle_collision_copy_dialog_rect();
    Rect::new(
        dialog.x + 24.0,
        dialog.y + 154.0,
        dialog.w - 48.0,
        dialog.h - 224.0,
    )
}

fn vehicle_collision_copy_submit_rect() -> Rect {
    let dialog = vehicle_collision_copy_dialog_rect();
    Rect::new(
        dialog.x + dialog.w - 232.0,
        dialog.y + dialog.h - 50.0,
        104.0,
        32.0,
    )
}

fn vehicle_collision_copy_cancel_rect() -> Rect {
    let dialog = vehicle_collision_copy_dialog_rect();
    Rect::new(
        dialog.x + dialog.w - 116.0,
        dialog.y + dialog.h - 50.0,
        88.0,
        32.0,
    )
}

pub(crate) fn vehicle_texture_preview_rect() -> Rect {
    let rect = vehicle_details_rect();
    Rect::new(
        rect.x + 14.0,
        rect.y + rect.h - VEHICLE_PREVIEW_H - 38.0,
        rect.w - 28.0,
        VEHICLE_PREVIEW_H,
    )
}

pub(crate) fn vehicle_texture_export_rect() -> Rect {
    let pv = vehicle_texture_preview_rect();
    Rect::new(pv.x + pv.w - 64.0, pv.y - 26.0, 64.0, 22.0)
}

pub(crate) fn vehicle_texture_copy_rect() -> Rect {
    let export = vehicle_texture_export_rect();
    Rect::new(export.x - 58.0, export.y, 52.0, export.h)
}

pub(crate) fn vehicle_texture_replace_rect() -> Rect {
    let copy = vehicle_texture_copy_rect();
    Rect::new(copy.x - 70.0, copy.y, 64.0, copy.h)
}

/// A visible row in the component tree: either a component (frame) header or
/// one of the render parts inside it.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum VehicleComponentRow {
    Component(usize),
    Part(usize),
}

pub(crate) fn vehicle_component_rows(app: &AppState) -> Vec<VehicleComponentRow> {
    let Some(mesh) = app.vehicle_browser.preview_mesh.as_ref() else {
        return Vec::new();
    };
    let component_count = mesh.components.len().max(1);
    let mut rows = Vec::new();
    for component in 0..component_count {
        let has_parts = mesh.parts.iter().any(|part| part.component == component);
        if !has_parts {
            continue;
        }
        rows.push(VehicleComponentRow::Component(component));
        if !app
            .vehicle_browser
            .collapsed_components
            .contains(&component)
        {
            rows.extend(
                mesh.parts
                    .iter()
                    .enumerate()
                    .filter(|(_, part)| part.component == component)
                    .map(|(idx, _)| VehicleComponentRow::Part(idx)),
            );
        }
    }
    rows
}

pub(crate) fn filtered_vehicle_indices(app: &AppState) -> Vec<usize> {
    let needle = lower(app.vehicle_browser.search.trim());
    let mut indices = app
        .vehicles
        .iter()
        .enumerate()
        .filter_map(|(idx, vehicle)| {
            if vehicle.readonly && !app.vehicle_browser.show_default {
                return None;
            }
            if !vehicle.readonly && !app.vehicle_browser.show_custom {
                return None;
            }
            if !needle.is_empty() {
                let hay = format!(
                    "{} {} {} {} {}",
                    lower(&vehicle.id),
                    lower(&vehicle.dff),
                    lower(&vehicle.txd),
                    lower(&vehicle.col),
                    lower(&vehicle.source)
                );
                if !hay.contains(&needle) {
                    return None;
                }
            }
            Some(idx)
        })
        .collect::<Vec<_>>();
    indices.sort_by(|a, b| lower(&app.vehicles[*a].id).cmp(&lower(&app.vehicles[*b].id)));
    indices
}

pub(crate) fn selected_vehicle(app: &AppState) -> Option<&VehicleAsset> {
    app.vehicles.get(app.vehicle_browser.selected)
}

fn vehicle_preview_identity(vehicle: &VehicleAsset) -> String {
    format!(
        "{}|{}",
        lower(&vehicle.dff),
        vehicle
            .loose_dff_path
            .as_ref()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default(),
    )
}

fn collision_copy_source_indices(
    vehicles: &[VehicleAsset],
    dialog: &VehicleCollisionCopyDialog,
) -> Vec<usize> {
    let needle = lower(dialog.search.trim());
    let target_identity = vehicle_preview_identity(&dialog.target);
    let mut indices = vehicles
        .iter()
        .enumerate()
        .filter_map(|(index, vehicle)| {
            if vehicle_preview_identity(vehicle) == target_identity {
                return None;
            }
            let searchable = lower(&format!(
                "{} {} {} {}",
                vehicle.id,
                vehicle.dff,
                vehicle.source,
                vehicle.model_id.map_or(String::new(), |id| id.to_string())
            ));
            (needle.is_empty() || searchable.contains(&needle)).then_some(index)
        })
        .collect::<Vec<_>>();
    indices.sort_unstable_by(|left, right| {
        lower(&vehicles[*left].id).cmp(&lower(&vehicles[*right].id))
    });
    indices
}

fn vehicle_collision_copy_writer_conflict(app: &AppState) -> Option<&'static str> {
    if app.vehicle_browser.collision_copy_rx.is_some() {
        Some("vehicle collision copy")
    } else if app.manual_save_job.is_some() {
        Some("manual save")
    } else if app.editing.save_rx.is_some() {
        Some("Editing IMG save")
    } else if app.autosave_rx.is_some() {
        Some("autosave")
    } else if app.autosave_cleanup_rx.is_some() {
        Some("autosave cleanup")
    } else if app.dff_repair_rx.is_some() {
        Some("DFF repair")
    } else if app.asset_optimization_scan_rx.is_some() {
        Some("asset optimization scan")
    } else if app.asset_optimization_job.is_some() {
        Some("asset optimization")
    } else if app.object_bounds_fix_job.is_some() {
        Some("object bounds repair")
    } else if app.txd_cleanup_job.is_some() {
        Some("TXD cleanup")
    } else if app.dff_picker_rx.is_some() {
        Some("asset file operation")
    } else if app.blender_import_rx.is_some() {
        Some("Blender import")
    } else if app.bake_job.is_some() {
        Some("vertex-light bake")
    } else if app.corona_generation_job.is_some() {
        Some("2DFX corona generation")
    } else if app.fracture_generation_job.is_some() {
        Some("fracture generation")
    } else if app.dff_geometry_job.is_some() {
        Some("DFF geometry generation")
    } else if app.collision_generation_job.is_some() || app.shadow_mesh_generation_job.is_some() {
        Some("collision generation")
    } else if app.lod_generation_job.is_some() {
        Some("LOD generation")
    } else if app.instance_lod_removal_job.is_some() {
        Some("instance LOD removal")
    } else if app.water_texture_conversion_job.is_some() {
        Some("texture-to-water conversion")
    } else if app.vehicle_build_rx.is_some() {
        Some("vehicle build")
    } else if app.vehicle_browser.reload_rx.is_some() {
        Some("vehicle reload")
    } else {
        None
    }
}

fn vehicle_collision_copy_target_has_unstaged_edit(app: &AppState, target: &VehicleAsset) -> bool {
    match app.editing.asset.as_ref() {
        Some(EditingAsset::Dff(dff)) => {
            dff.dirty && asset_key(&dff.name, ".dff") == asset_key(&target.dff, ".dff")
        }
        Some(EditingAsset::Col(col)) => {
            col.dirty
                && col.embedded_vehicle_dff
                && asset_key(&col.name, ".dff") == asset_key(&target.dff, ".dff")
        }
        _ => false,
    }
}

pub(crate) fn open_vehicle_collision_copy_dialog(app: &mut AppState) {
    let Some(target) = selected_vehicle(app).cloned() else {
        app.status_message = "Select a custom vehicle to receive the collision".to_string();
        return;
    };
    if target.readonly {
        app.status_message =
            "Collision copying can only replace a custom vehicle's collision".to_string();
        return;
    }
    let Some(target_path) = target.loose_dff_path.as_ref() else {
        app.status_message =
            "The selected custom vehicle is not backed by an editable loose DFF".to_string();
        return;
    };
    if !target_path.is_file() {
        app.status_message = format!(
            "Could not find custom vehicle DFF {}",
            target_path.display()
        );
        return;
    }
    if vehicle_collision_copy_target_has_unstaged_edit(app, &target) {
        app.status_message =
            "Stage or discard the target vehicle's active Editing changes before replacing its collision"
                .to_string();
        return;
    }
    if let Some(job) = vehicle_collision_copy_writer_conflict(app) {
        app.status_message =
            format!("Wait for the background {job} before copying a vehicle collision");
        return;
    }
    let mut dialog = VehicleCollisionCopyDialog {
        target,
        search: String::new(),
        cursor: 0,
        selection_anchor: None,
        selected_source: None,
        scroll: 0.0,
    };
    dialog.selected_source = collision_copy_source_indices(&app.vehicles, &dialog)
        .first()
        .copied();
    app.vehicle_browser.collision_copy_dialog = Some(dialog);
    drain_text_input();
}

fn read_vehicle_dff_for_collision_copy(
    vehicle: &VehicleAsset,
    root: &Path,
    gta_sa_dir: &Path,
) -> Result<Vec<u8>, String> {
    if let Some(path) = vehicle.loose_dff_path.as_ref() {
        return fs::read(path)
            .map_err(|err| format!("Could not read DFF {}: {err}", path.display()));
    }
    let entry = find_vehicle_img_entry(root, gta_sa_dir, &vehicle.dff, ".dff")
        .ok_or_else(|| format!("Could not find source vehicle DFF {}", vehicle.dff))?;
    let bytes = read_img_entry(&entry);
    if bytes.is_empty() {
        Err(format!("Source vehicle DFF {} is empty", vehicle.dff))
    } else {
        Ok(bytes)
    }
}

fn copy_vehicle_collision_worker(
    target: VehicleAsset,
    source: VehicleAsset,
    root: PathBuf,
    gta_sa_dir: PathBuf,
) -> VehicleCollisionCopyResult {
    let target_identity = vehicle_preview_identity(&target);
    let target_id = target.id.clone();
    let target_dff = target.dff.clone();
    let source_id = source.id.clone();
    let result = (|| -> Result<VehicleCollisionCopyOutput, String> {
        let target_path = target.loose_dff_path.as_ref().ok_or_else(|| {
            "Target custom vehicle is not backed by an editable loose DFF".to_string()
        })?;
        let source_dff = read_vehicle_dff_for_collision_copy(&source, &root, &gta_sa_dir)?;
        let (_source_mesh, mut collision_bytes) =
            parse_embedded_vehicle_collision(&source_dff, &source.dff).ok_or_else(|| {
                format!(
                    "Source vehicle {} has no embedded collision model",
                    source.id
                )
            })?;
        set_col_model_names_from_entry(&mut collision_bytes, &target.dff);
        let validation = validate_col_for_game_load(&target.dff, &collision_bytes);
        if col_validation_has_errors(&validation) {
            let reason = validation
                .iter()
                .find(|issue| issue.severity == ColLoadIssueSeverity::Error)
                .map(|issue| issue.message.as_str())
                .unwrap_or("game-load validation failed");
            return Err(format!("Source collision is not safe to load: {reason}"));
        }

        let target_dff = fs::read(target_path)
            .map_err(|err| format!("Could not read target DFF {}: {err}", target_path.display()))?;
        let replaced = replace_embedded_vehicle_collision(&target_dff, &collision_bytes)?;
        let (verified_mesh, verified_bytes) =
            parse_embedded_vehicle_collision(&replaced, &target.dff).ok_or_else(|| {
                "Rewritten target DFF did not contain a readable collision".to_string()
            })?;
        if verified_bytes != collision_bytes {
            return Err("Rewritten target DFF failed collision verification".to_string());
        }
        let backup = write_custom_vehicle_asset(target_path, &replaced, "DFF")?;
        Ok(VehicleCollisionCopyOutput {
            mesh: verified_mesh,
            collision_bytes,
            dff_bytes: replaced,
            backup,
        })
    })();
    VehicleCollisionCopyResult {
        target_identity,
        target_id,
        target_dff,
        source_id,
        result,
    }
}

fn start_vehicle_collision_copy(app: &mut AppState) {
    let Some(dialog) = app.vehicle_browser.collision_copy_dialog.as_ref() else {
        return;
    };
    let Some(source_index) = dialog.selected_source else {
        app.status_message = "Select a source vehicle collision first".to_string();
        return;
    };
    let Some(source) = app.vehicles.get(source_index).cloned() else {
        app.status_message = "The selected source vehicle is no longer available".to_string();
        return;
    };
    let target = dialog.target.clone();
    if vehicle_preview_identity(&source) == vehicle_preview_identity(&target) {
        app.status_message = "Choose a different vehicle as the collision source".to_string();
        return;
    }
    if vehicle_collision_copy_target_has_unstaged_edit(app, &target) {
        app.status_message =
            "Stage or discard the target vehicle's active Editing changes before replacing its collision"
                .to_string();
        return;
    }
    if let Some(job) = vehicle_collision_copy_writer_conflict(app) {
        app.status_message =
            format!("Wait for the background {job} before copying a vehicle collision");
        return;
    }
    let root = app.root.clone();
    let gta_sa_dir = app.gta_sa_dir.clone();
    let source_id = source.id.clone();
    let target_id = target.id.clone();
    let (tx, rx) = mpsc::channel();
    app.vehicle_browser.collision_copy_rx = Some(rx);
    app.vehicle_browser.collision_copy_dialog = None;
    app.status_message = format!("Replacing all {target_id} collision data from {source_id}...");
    thread::spawn(move || {
        let result = copy_vehicle_collision_worker(target, source, root, gta_sa_dir);
        let _ = tx.send(result);
    });
}

pub(crate) fn poll_vehicle_collision_copy(app: &mut AppState) {
    let Some(rx) = app.vehicle_browser.collision_copy_rx.as_ref() else {
        return;
    };
    let result = match rx.try_recv() {
        Ok(result) => result,
        Err(mpsc::TryRecvError::Empty) => return,
        Err(mpsc::TryRecvError::Disconnected) => {
            app.vehicle_browser.collision_copy_rx = None;
            app.status_message = "Vehicle collision copy stopped unexpectedly".to_string();
            return;
        }
    };
    app.vehicle_browser.collision_copy_rx = None;
    let output = match result.result {
        Ok(output) => output,
        Err(err) => {
            app.status_message = format!("Could not copy vehicle collision: {err}");
            return;
        }
    };
    if selected_vehicle(app)
        .is_some_and(|vehicle| vehicle_preview_identity(vehicle) == result.target_identity)
    {
        app.vehicle_browser.embedded_collision = Some(output.mesh.clone());
    }
    if let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut()
        && col.embedded_vehicle_dff
        && asset_key(&col.name, ".dff") == asset_key(&result.target_dff, ".dff")
        && !col.dirty
    {
        col.mesh = output.mesh.clone();
        col.bytes = output.collision_bytes.clone();
        col.embedded_source_dff_bytes = Some(output.dff_bytes.clone());
        // Editor-only primitive metadata belongs to the collision that was
        // completely replaced, so it must not remain attached to the copy.
        col.capsules.clear();
        col.cuboids.clear();
        if let Some(identity) = col_model_ranges(&col.bytes)
            .into_iter()
            .next()
            .map(|range| range.identity)
        {
            col.source_model = identity;
        }
        col.selected_face = col
            .selected_face
            .min(col.mesh.faces.len().saturating_sub(1));
        col.selected_faces
            .retain(|face| *face < col.mesh.faces.len());
        col.selected_vertex = col
            .selected_vertex
            .min(col.mesh.vertices.len().saturating_sub(1));
        col.selected_edges.clear();
        col.selected_vertices.clear();
        col.selected_primitive = None;
        col.hovered_face = None;
        col.hovered_vertex = None;
    }
    let backup_note = output
        .backup
        .as_ref()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .map(|name| format!("; original backed up as {name}"))
        .unwrap_or_default();
    app.status_message = format!(
        "Replaced all collision data on {} from {}{}",
        result.target_id, result.source_id, backup_note
    );
}

fn vehicle_wheel_dummy_slot(name: &str) -> Option<(&'static str, &'static str)> {
    let n = lower(name);
    if !n.contains("dummy") || n.contains("steer") {
        return None;
    }
    let side = if n.contains("wheel_lf")
        || n.contains("wheel_lr")
        || n.contains("wheel_lb")
        || n.contains("wheel_lm")
    {
        "left"
    } else if n.contains("wheel_rf")
        || n.contains("wheel_rr")
        || n.contains("wheel_rb")
        || n.contains("wheel_rm")
    {
        "right"
    } else {
        return None;
    };
    let axle = if n.contains("wheel_lr")
        || n.contains("wheel_rr")
        || n.contains("wheel_lb")
        || n.contains("wheel_rb")
    {
        "rear"
    } else if n.contains("wheel_lf") || n.contains("wheel_rf") {
        "front"
    } else {
        "any"
    };
    Some((axle, side))
}

fn vehicle_preview_wheel_axle(name: &str) -> Option<&'static str> {
    if let Some((axle, _)) = vehicle_wheel_dummy_slot(name) {
        return Some(axle);
    }
    let name = lower(name.trim());
    if !name.contains("wheel") || name.contains("steer") {
        return None;
    }
    if name.contains("wheel_front")
        || name.contains("wheel_lf")
        || name.contains("wheel_rf")
        || name.contains("wheel_f")
    {
        Some("front")
    } else if name.contains("wheel_rear")
        || name.contains("wheel_back")
        || name.contains("wheel_lr")
        || name.contains("wheel_rr")
        || name.contains("wheel_lb")
        || name.contains("wheel_rb")
    {
        Some("rear")
    } else {
        Some("any")
    }
}

fn vehicle_component_wheel_anchor<'a>(
    raw: &'a RawMesh,
    component: &RawMeshComponent,
) -> Option<&'a RawMeshFrame> {
    let mut frame_idx = raw
        .frames
        .iter()
        .position(|frame| frame.name.eq_ignore_ascii_case(&component.name))?;
    for _ in 0..raw.frames.len() {
        let frame = raw.frames.get(frame_idx)?;
        if vehicle_wheel_dummy_slot(&frame.name).is_some() {
            return Some(frame);
        }
        if frame.parent < 0 {
            break;
        }
        frame_idx = frame.parent as usize;
    }
    None
}

fn vehicle_body_color(app: &AppState, slot: usize) -> V3 {
    if slot == 0 {
        app.vehicle_browser.body_color_a
    } else {
        app.vehicle_browser.body_color_b
    }
}

fn set_vehicle_body_color_channel(app: &mut AppState, slot: usize, channel: usize, value: f32) {
    let color = if slot == 0 {
        &mut app.vehicle_browser.body_color_a
    } else {
        &mut app.vehicle_browser.body_color_b
    };
    match channel {
        0 => color.x = value,
        1 => color.y = value,
        2 => color.z = value,
        _ => {}
    }
    apply_vehicle_body_color_to_preview(app, slot);
}

fn apply_vehicle_body_color_to_preview(app: &mut AppState, slot: usize) {
    let role = if slot == 0 {
        VehicleMaterialRole::BodyA
    } else {
        VehicleMaterialRole::BodyB
    };
    let color = vehicle_body_color(app, slot);
    let Some(mesh) = app.vehicle_browser.preview_mesh.as_mut() else {
        app.vehicle_browser.preview_key.clear();
        return;
    };
    for part in &mut mesh.parts {
        if part.vehicle_material_role != Some(role) {
            continue;
        }
        for vertex in &mut part.cpu_vertices {
            vertex.day_color = V3 {
                x: vertex.base_day_color.x * color.x,
                y: vertex.base_day_color.y * color.y,
                z: vertex.base_day_color.z * color.z,
            };
            vertex.night_color = V3 {
                x: vertex.base_night_color.x * color.x,
                y: vertex.base_night_color.y * color.y,
                z: vertex.base_night_color.z * color.z,
            };
            vertex.color = vertex.day_color;
        }
    }
}

fn duplicate_vehicle_wheel_components(raw: &mut RawMesh) {
    if raw.frames.is_empty() || raw.components.is_empty() {
        return;
    }
    let wheel_targets = raw
        .frames
        .iter()
        .filter(|frame| vehicle_wheel_dummy_slot(&frame.name).is_some())
        .cloned()
        .collect::<Vec<_>>();
    if wheel_targets.is_empty() {
        return;
    }
    let existing = raw
        .components
        .iter()
        .map(|component| lower(&component.name))
        .collect::<HashSet<_>>();
    let sources = raw
        .components
        .iter()
        .enumerate()
        .filter_map(|(idx, component)| {
            if component.vertex_start >= component.vertex_end {
                return None;
            }
            let frame = vehicle_component_wheel_anchor(raw, component)?;
            let (axle, side) = vehicle_wheel_dummy_slot(&frame.name)?;
            Some((idx, axle, side, frame.clone()))
        })
        .collect::<Vec<_>>();
    if sources.is_empty() {
        return;
    }
    // The source wheel geometry is commonly named after a rim mesh rather than
    // its wheel dummy. Give it the same semantic name as the duplicated wheels
    // so all four wheels can be recognized and transformed consistently by the
    // live preview (and shown consistently in the component tree).
    for (component, _, _, frame) in &sources {
        raw.components[*component].name = frame.name.clone();
    }
    let occupied_targets = sources
        .iter()
        .map(|(_, _, _, frame)| lower(&frame.name))
        .collect::<HashSet<_>>();
    let has_normals = raw.normals.len() == raw.vertices.len();
    let has_uvs = raw.uvs.len() == raw.vertices.len();
    let secondary_uv_valid = raw
        .secondary_uvs
        .iter()
        .map(|uvs| uvs.len() == raw.vertices.len())
        .collect::<Vec<_>>();
    let has_prelit = raw.prelit_colors.len() == raw.vertices.len();
    let has_prelit_alpha = raw.prelit_alphas.len() == raw.vertices.len();
    let has_night = raw.night_prelit_colors.len() == raw.vertices.len();
    let has_night_alpha = raw.night_prelit_alphas.len() == raw.vertices.len();
    let has_flags = raw.light_flags.len() == raw.vertices.len();
    for target in wheel_targets {
        let target_name = lower(&target.name);
        if existing.contains(&target_name) || occupied_targets.contains(&target_name) {
            continue;
        }
        let Some((target_axle, target_side)) = vehicle_wheel_dummy_slot(&target.name) else {
            continue;
        };
        let Some((source_idx, _, source_side, source_frame)) = sources
            .iter()
            .find(|(_, axle, side, _)| *axle == target_axle && *side == target_side)
            .or_else(|| {
                sources
                    .iter()
                    .find(|(_, axle, side, _)| *axle == target_axle && *side != target_side)
            })
            .or_else(|| sources.iter().find(|(_, axle, _, _)| *axle == "any"))
            .or_else(|| sources.first())
            .cloned()
        else {
            continue;
        };
        let source = raw.components[source_idx].clone();
        // Decide mirroring from the actual world-X side of each wheel dummy rather
        // than the frame name. Vehicles usually ship a single generic `wheel`
        // model (side "any"), so a name-only comparison never mirrors anything and
        // the whole left side ends up facing inward. Comparing which side of the
        // vehicle centreline (X = 0) the target sits on vs. the source frame
        // reverses the left dummies correctly, and we fall back to the name-based
        // side when a dummy sits on the centreline.
        let mirror_side = if target.pos.x.abs() > 0.01 && source_frame.pos.x.abs() > 0.01 {
            (target.pos.x >= 0.0) != (source_frame.pos.x >= 0.0)
        } else {
            target_side != "any" && source_side != "any" && target_side != source_side
        };
        let offset = V3 {
            x: target.pos.x - source_frame.pos.x,
            y: target.pos.y - source_frame.pos.y,
            z: target.pos.z - source_frame.pos.z,
        };
        let vertex_start = raw.vertices.len();
        let source_start = source.vertex_start.min(raw.vertices.len());
        let source_end = source.vertex_end.min(raw.vertices.len());
        for src in source_start..source_end {
            let mut vertex = raw.vertices[src];
            if mirror_side {
                vertex.x = source_frame.pos.x - (vertex.x - source_frame.pos.x);
            }
            raw.vertices.push(V3 {
                x: vertex.x + offset.x,
                y: vertex.y + offset.y,
                z: vertex.z + offset.z,
            });
            if has_normals {
                let mut normal = raw.normals[src];
                if mirror_side {
                    normal.x = -normal.x;
                }
                raw.normals.push(normal);
            }
            if has_uvs {
                raw.uvs.push(raw.uvs[src]);
            }
            for (uvs, valid) in raw.secondary_uvs.iter_mut().zip(&secondary_uv_valid) {
                if *valid {
                    uvs.push(uvs[src]);
                }
            }
            if has_prelit {
                raw.prelit_colors.push(raw.prelit_colors[src]);
            }
            if has_prelit_alpha {
                raw.prelit_alphas.push(raw.prelit_alphas[src]);
            }
            if has_night {
                raw.night_prelit_colors.push(raw.night_prelit_colors[src]);
            }
            if has_night_alpha {
                raw.night_prelit_alphas.push(raw.night_prelit_alphas[src]);
            }
            if has_flags {
                raw.light_flags.push(raw.light_flags[src]);
            }
        }
        let vertex_end = raw.vertices.len();
        let tri_start = raw.triangles.len();
        for tri in raw.triangles
            [source.tri_start.min(raw.triangles.len())..source.tri_end.min(raw.triangles.len())]
            .to_vec()
        {
            let remap = |idx: u32| -> Option<u32> {
                let idx = idx as usize;
                if idx < source_start || idx >= source_end {
                    return None;
                }
                Some((vertex_start + idx - source_start) as u32)
            };
            if let (Some(a), Some(b), Some(c)) = (remap(tri.a), remap(tri.b), remap(tri.c)) {
                if mirror_side {
                    raw.triangles.push(Tri {
                        a,
                        b: c,
                        c: b,
                        material: tri.material,
                    });
                } else {
                    raw.triangles.push(Tri {
                        a,
                        b,
                        c,
                        material: tri.material,
                    });
                }
            }
        }
        raw.components.push(RawMeshComponent {
            name: target.name.clone(),
            frame_index: None,
            vertex_start,
            vertex_end,
            tri_start,
            tri_end: raw.triangles.len(),
            breakable: None,
        });
    }
}

pub(crate) fn vehicle_read_dff_bytes(app: &AppState, vehicle: &VehicleAsset) -> Option<Vec<u8>> {
    if let Some(path) = &vehicle.loose_dff_path {
        return fs::read(path).ok();
    }
    find_dff_entry(&app.root, &vehicle.dff).map(|entry| read_img_entry(&entry))
}

const RW_COLLISION_MODEL_ID: u32 = 0x0253_F2FA;

#[derive(Clone, Debug, PartialEq, Eq)]
struct EmbeddedVehicleCollisionChunk {
    header_start: usize,
    payload_start: usize,
    payload_end: usize,
    ancestor_headers: Vec<usize>,
}

fn rw_chunk_can_contain_children(chunk_id: u32) -> bool {
    matches!(
        chunk_id,
        0x03 // Extension
            | 0x06 // Texture
            | 0x07 // Material
            | 0x08 // Material List
            | 0x0E // Frame List
            | 0x0F // Geometry
            | 0x10 // Clump
            | 0x16 // Texture Dictionary
            | 0x1A // Geometry List
    )
}

fn find_embedded_vehicle_collision_in(
    bytes: &[u8],
    start: usize,
    end: usize,
    ancestors: &mut Vec<usize>,
) -> Option<EmbeddedVehicleCollisionChunk> {
    let mut cursor = start;
    while cursor + 12 <= end {
        let chunk_id = rd32(bytes, cursor);
        let payload_len = rd32(bytes, cursor + 4) as usize;
        let payload_start = cursor + 12;
        let payload_end = payload_start.checked_add(payload_len)?;
        if payload_end > end || payload_end > bytes.len() {
            break;
        }
        if chunk_id == RW_COLLISION_MODEL_ID
            && payload_len >= 8
            && matches!(
                bytes.get(payload_start..payload_start + 4),
                Some(b"COLL" | b"COL2" | b"COL3" | b"COL4")
            )
        {
            return Some(EmbeddedVehicleCollisionChunk {
                header_start: cursor,
                payload_start,
                payload_end,
                ancestor_headers: ancestors.clone(),
            });
        }
        if rw_chunk_can_contain_children(chunk_id) {
            ancestors.push(cursor);
            if let Some(found) =
                find_embedded_vehicle_collision_in(bytes, payload_start, payload_end, ancestors)
            {
                return Some(found);
            }
            ancestors.pop();
        }
        cursor = payload_end;
    }
    None
}

fn find_embedded_vehicle_collision(bytes: &[u8]) -> Option<EmbeddedVehicleCollisionChunk> {
    find_embedded_vehicle_collision_in(bytes, 0, bytes.len(), &mut Vec::new())
}

fn parse_embedded_vehicle_collision(
    dff_bytes: &[u8],
    dff_name: &str,
) -> Option<(CollisionMesh, Vec<u8>)> {
    let chunk = find_embedded_vehicle_collision(dff_bytes)?;
    let payload = dff_bytes[chunk.payload_start..chunk.payload_end].to_vec();
    let entry = ImgEntry {
        img_path: PathBuf::from(dff_name),
        name: with_ext(dff_name, ".col"),
        offset: 0,
        size: payload.len().min(u32::MAX as usize) as u32,
    };
    parse_col_mesh(&payload, &entry).map(|mesh| (mesh, payload))
}

/// Replace the complete COL payload in a vehicle's Rockstar collision plug-in
/// and update every enclosing RenderWare chunk size.
pub(crate) fn replace_embedded_vehicle_collision(
    dff_bytes: &[u8],
    collision_bytes: &[u8],
) -> Result<Vec<u8>, String> {
    let chunk = find_embedded_vehicle_collision(dff_bytes)
        .ok_or_else(|| "Vehicle DFF has no embedded collision model".to_string())?;
    if collision_bytes.len() > u32::MAX as usize {
        return Err("Embedded collision is too large for a RenderWare chunk".to_string());
    }
    let old_len = chunk.payload_end - chunk.payload_start;
    let delta = collision_bytes.len() as i64 - old_len as i64;
    let mut out = Vec::with_capacity((dff_bytes.len() as i64 + delta).max(0) as usize);
    out.extend_from_slice(&dff_bytes[..chunk.payload_start]);
    out.extend_from_slice(collision_bytes);
    out.extend_from_slice(&dff_bytes[chunk.payload_end..]);
    out[chunk.header_start + 4..chunk.header_start + 8]
        .copy_from_slice(&(collision_bytes.len() as u32).to_le_bytes());
    for header in chunk.ancestor_headers {
        let old_size = rd32(dff_bytes, header + 4) as i64;
        let new_size = old_size + delta;
        if !(0..=u32::MAX as i64).contains(&new_size) {
            return Err("Embedded collision resize overflowed its DFF container".to_string());
        }
        out[header + 4..header + 8].copy_from_slice(&(new_size as u32).to_le_bytes());
    }
    Ok(out)
}

/// Locate the standalone TXD paired with a loose vehicle DFF. Dictionary scans
/// retain the exact matched path; project-local loose assets fall back to a
/// case-insensitive sibling lookup.
fn find_loose_vehicle_txd(vehicle: &VehicleAsset) -> Option<PathBuf> {
    if let Some(path) = vehicle
        .loose_txd_path
        .as_ref()
        .filter(|path| path.is_file())
    {
        return Some(path.clone());
    }
    let stem = vehicle.txd.trim();
    if stem.is_empty() {
        return None;
    }
    let matches = |path: &Path| {
        path.is_file()
            && path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("txd"))
            && path
                .file_stem()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case(stem))
    };
    if let Some(dff_path) = vehicle.loose_dff_path.as_ref() {
        if let Some(dir) = dff_path.parent() {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.filter_map(Result::ok) {
                    let path = entry.path();
                    if matches(&path) {
                        return Some(path);
                    }
                }
            }
        }
    }
    None
}

/// Custom dictionary vehicles ship a standalone TXD alongside the DFF. These
/// dictionaries are not part of the project's TXD index, so register the exact
/// matched `.txd` on demand the first time the vehicle is previewed.
pub(crate) fn ensure_custom_vehicle_txd_indexed(app: &mut AppState, vehicle: &VehicleAsset) {
    if vehicle.loose_txd_path.is_none() {
        return;
    }
    let Some(txd_path) = find_loose_vehicle_txd(vehicle) else {
        return;
    };
    // Skip only if this exact file has already been indexed.
    let already_indexed = app
        .txd_textures
        .values()
        .any(|entries| entries.iter().any(|entry| entry.img_path == txd_path));
    if already_indexed {
        return;
    }
    // A base-game dictionary of the same name may already be indexed (e.g.
    // `copcarsf.txd` from gta3.img). Those entries would otherwise shadow the
    // external vehicle's own textures, so drop them and let the external TXD
    // fully own this scope.
    let txd_key = asset_key(&vehicle.txd, ".txd");
    for entries in app.txd_textures.values_mut() {
        entries.retain(|entry| !entry.txd_name.eq_ignore_ascii_case(&txd_key));
    }
    app.txd_textures.retain(|_, entries| !entries.is_empty());
    index_standalone_txd_file(&txd_path, &mut app.txd_textures);
}

pub(crate) fn clear_vehicle_preview_mesh(app: &mut AppState) {
    if let Some(mesh) = app.vehicle_browser.preview_mesh.take() {
        delete_render_mesh_gpu(&mesh);
    }
}

fn install_vehicle_preview(app: &mut AppState, vehicle: &VehicleAsset, mut raw: RawMesh) -> bool {
    let ambient_lift = scene_ambient_lift_from_timecyc(&app.timecyc);
    let txd_scope = (!vehicle.txd.trim().is_empty()).then_some(vehicle.txd.as_str());
    duplicate_vehicle_wheel_components(&mut raw);
    // Shared vehicle textures (vehiclegeneric256, vehiclelights128, ...) live
    // in the global vehicle.txd rather than the vehicle's own TXD; redirect
    // any texture the scoped TXD is missing to whichever TXD provides it,
    // preferring the shared "vehicle" dictionary.
    let vehicle_txd_key = txd_scope
        .map(|txd| asset_key(txd, ".txd"))
        .unwrap_or_default();
    let sa_generic_txd_key = asset_key("vehicle", ".txd");
    let mut texture_overrides: HashMap<String, String> = HashMap::new();
    for name in &raw.material_textures {
        let key = lower(name.trim());
        if key.is_empty() || texture_overrides.contains_key(&key) {
            continue;
        }
        let Some(entries) = app.txd_textures.get(&key) else {
            continue;
        };
        if entries
            .iter()
            .any(|entry| entry.txd_name.eq_ignore_ascii_case(&vehicle_txd_key))
        {
            continue;
        }
        let entry = entries
            .iter()
            .find(|entry| entry.txd_name.eq_ignore_ascii_case(&sa_generic_txd_key))
            .or_else(|| entries.first());
        if let Some(entry) = entry {
            texture_overrides.insert(key, entry.txd_name.clone());
        }
    }
    let Some(mesh) = compile_render_mesh(
        raw,
        txd_scope,
        Some(&texture_overrides),
        Some(VehicleMaterialPreview {
            body_a: app.vehicle_browser.body_color_a,
            body_b: app.vehicle_browser.body_color_b,
            lights_on: app.vehicle_browser.lights_on,
        }),
        &app.texture_files,
        &app.txd_textures,
        &mut app.textures,
        &mut app.textured_parts,
        app.options.textures,
        ambient_lift,
    ) else {
        return false;
    };
    if let Some(previous) = app.vehicle_browser.preview_mesh.replace(mesh) {
        delete_render_mesh_gpu(&previous);
    }
    true
}

pub(crate) fn ensure_vehicle_preview(app: &mut AppState) {
    let Some(vehicle) = selected_vehicle(app).cloned() else {
        clear_vehicle_preview_mesh(app);
        app.vehicle_browser.embedded_collision = None;
        app.vehicle_browser.preview_key.clear();
        return;
    };
    // Identity = which vehicle is loaded, independent of view toggles like the
    // lights switch. The full key adds the lights state so the mesh recompiles
    // (emissive + "on" textures) when it changes.
    let identity = vehicle_preview_identity(&vehicle);
    let key = format!("{}|lights={}", identity, app.vehicle_browser.lights_on);
    if app.vehicle_browser.preview_key == key {
        return;
    }
    // Same vehicle, only a view toggle changed (e.g. the lights switch): keep
    // component rotations, selection, and hidden/collapsed state so toggling
    // lights doesn't blow away the user's work.
    let same_vehicle = app
        .vehicle_browser
        .preview_key
        .rsplit_once("|lights=")
        .map(|(prefix, _)| prefix == identity)
        .unwrap_or(false);
    app.vehicle_browser.preview_key = key;
    clear_vehicle_preview_mesh(app);
    if !same_vehicle {
        app.vehicle_browser.embedded_collision = None;
        app.vehicle_browser.hidden_parts.clear();
        app.vehicle_browser.hidden_components.clear();
        app.vehicle_browser.collapsed_components.clear();
        app.vehicle_browser.selected_component = None;
        app.vehicle_browser.component_rotations.clear();
        app.vehicle_browser.selected_part = None;
        app.vehicle_browser.texture_previews.clear();
        app.vehicle_browser.component_scroll = 0.0;
    }
    ensure_custom_vehicle_txd_indexed(app, &vehicle);
    let Some(bytes) = vehicle_read_dff_bytes(app, &vehicle) else {
        app.status_message = format!("Could not find vehicle DFF {}", vehicle.dff);
        return;
    };
    app.vehicle_browser.embedded_collision =
        parse_embedded_vehicle_collision(&bytes, &vehicle.dff).map(|(mesh, _)| mesh);
    let raw = parse_dff_mesh(&bytes);
    if raw.vertices.is_empty() {
        app.status_message = format!("Vehicle DFF {} did not parse", vehicle.dff);
        return;
    }
    if !install_vehicle_preview(app, &vehicle, raw) {
        app.status_message = format!("Vehicle DFF {} has no renderable geometry", vehicle.dff);
        return;
    }
    if !same_vehicle {
        focus_selected_vehicle(app);
    }
    app.status_message = format!("Viewing vehicle {}", vehicle.id);
}

fn find_vehicle_img_entry(
    root: &Path,
    gta_sa_dir: &Path,
    name: &str,
    extension: &str,
) -> Option<ImgEntry> {
    let target = asset_key(name, extension);
    for replacement_root in [wip_root_path(root), root.to_path_buf()] {
        if let Some(entry) = replacement_img_entry(&replacement_root, &target) {
            return Some(entry);
        }
    }
    collect_resource_img_files(root)
        .into_iter()
        .chain(gta_sa_img_files(gta_sa_dir))
        .find_map(|path| {
            parse_img(&path)
                .into_iter()
                .find(|entry| lower(&entry.name) == target)
        })
}

pub(crate) fn find_vehicle_txd_entry(
    root: &Path,
    gta_sa_dir: &Path,
    vehicle: &VehicleAsset,
) -> Option<ImgEntry> {
    if let Some(path) = find_loose_vehicle_txd(vehicle) {
        return loose_txd_entry(&path);
    }
    let target = asset_key(&vehicle.txd, ".txd");
    for replacement_root in [wip_root_path(root), root.to_path_buf()] {
        if let Some(entry) = replacement_img_entry(&replacement_root, &target) {
            return Some(entry);
        }
    }
    if let Some(path) = collect_resource_txd_files(root).into_iter().find(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| asset_key(name, ".txd") == target)
    }) {
        return loose_txd_entry(&path);
    }
    find_vehicle_img_entry(root, gta_sa_dir, &vehicle.txd, ".txd")
}

fn scan_vehicle_loader_membership(loader: &Path) -> Result<HashSet<String>, String> {
    if !loader.join("meta.xml").is_file() {
        return Err(format!("{} is not an MTA resource", loader.display()));
    }
    let mut membership = HashSet::new();
    let registry_path = loader.join("vehicle_registry.xml");
    if let Ok(text) = fs::read_to_string(&registry_path) {
        let attr = Regex::new(r#"(?:id|key|dff)="([^"]+)""#).unwrap();
        for capture in attr.captures_iter(&text) {
            let value = capture[1].trim();
            membership.insert(lower(value));
            if let Some(stem) = Path::new(value)
                .file_stem()
                .and_then(|value| value.to_str())
            {
                membership.insert(lower(stem));
            }
        }
    }
    let models = loader.join("Models");
    if models.is_dir() {
        for entry in WalkDir::new(models).into_iter().filter_map(Result::ok) {
            let path = entry.path();
            if path.is_file()
                && path
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|value| value.eq_ignore_ascii_case("dff"))
                && let Some(stem) = path.file_stem().and_then(|value| value.to_str())
            {
                membership.insert(lower(stem));
            }
        }
    }
    Ok(membership)
}

fn scan_vehicle_loader(loader: &Path) -> Result<VehicleLoaderScanResult, String> {
    Ok(VehicleLoaderScanResult {
        membership: scan_vehicle_loader_membership(loader)?,
        categories: load_vehicle_categories(loader),
        used_categories: load_used_vehicle_categories(loader),
    })
}

pub(crate) fn start_vehicle_loader_scan(app: &mut AppState) {
    let Some(loader) = app.vehicle_loader_resource.clone() else {
        app.vehicle_loader_membership.clear();
        app.vehicle_loader_categories.clear();
        app.vehicle_loader_used_categories.clear();
        return;
    };
    if app.vehicle_loader_scan_rx.is_some() {
        return;
    }
    let (tx, rx) = mpsc::channel();
    app.vehicle_loader_scan_rx = Some(rx);
    thread::spawn(move || {
        let _ = tx.send(scan_vehicle_loader(&loader));
    });
}

pub(crate) fn open_vehicle_loader_picker(app: &mut AppState) {
    if app.vehicle_loader_picker_rx.is_some() {
        app.status_message = "Vehicle loader browser is already open".to_string();
        return;
    }
    let start = app
        .vehicle_loader_resource
        .clone()
        .or_else(|| app.root.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let (tx, rx) = mpsc::channel();
    app.vehicle_loader_picker_rx = Some(rx);
    app.status_message = "Choose the MTA vehicle loader resource...".to_string();
    thread::spawn(move || {
        let _ = tx.send(choose_vehicle_folder(start));
    });
}

pub(crate) fn poll_vehicle_loader_picker(app: &mut AppState) {
    let Some(rx) = app.vehicle_loader_picker_rx.as_ref() else {
        return;
    };
    let result = match rx.try_recv() {
        Ok(result) => result,
        Err(mpsc::TryRecvError::Empty) => return,
        Err(mpsc::TryRecvError::Disconnected) => {
            app.vehicle_loader_picker_rx = None;
            app.status_message = "Vehicle loader browser closed unexpectedly".to_string();
            return;
        }
    };
    app.vehicle_loader_picker_rx = None;
    match result {
        Ok(Some(path)) if path.join("meta.xml").is_file() => {
            save_vehicle_loader_resource_preference(&path);
            app.vehicle_loader_resource = Some(path.clone());
            app.vehicle_loader_membership.clear();
            app.vehicle_loader_categories.clear();
            app.vehicle_loader_used_categories.clear();
            app.status_message = format!("Vehicle loader set to {}", path.display());
            start_vehicle_loader_scan(app);
        }
        Ok(Some(path)) => {
            app.status_message = format!(
                "{} is not an MTA resource (meta.xml missing)",
                path.display()
            )
        }
        Ok(None) => app.status_message = "Vehicle loader selection cancelled".to_string(),
        Err(err) => app.status_message = format!("Could not choose vehicle loader: {err}"),
    }
}

pub(crate) fn poll_vehicle_loader_scan(app: &mut AppState) {
    let Some(rx) = app.vehicle_loader_scan_rx.as_ref() else {
        return;
    };
    let result = match rx.try_recv() {
        Ok(result) => result,
        Err(mpsc::TryRecvError::Empty) => return,
        Err(mpsc::TryRecvError::Disconnected) => {
            app.vehicle_loader_scan_rx = None;
            return;
        }
    };
    app.vehicle_loader_scan_rx = None;
    match result {
        Ok(result) => {
            app.vehicle_loader_membership = result.membership;
            app.vehicle_loader_categories = result.categories;
            app.vehicle_loader_used_categories = result.used_categories;
        }
        Err(err) => app.status_message = err,
    }
}

fn vehicle_loaded_in_resource(app: &AppState, vehicle: &VehicleAsset) -> bool {
    if vehicle.readonly {
        return true;
    }
    [
        vehicle.id.as_str(),
        vehicle.dff.as_str(),
        vehicle
            .model_id
            .map(|value| value.to_string())
            .as_deref()
            .unwrap_or(""),
    ]
    .iter()
    .any(|value| app.vehicle_loader_membership.contains(&lower(value)))
}

fn next_custom_vehicle_id(loader: &Path) -> u32 {
    let text = fs::read_to_string(loader.join("vehicle_registry.xml")).unwrap_or_default();
    let mut next = Regex::new(r#"nextCustomId="(\d+)""#)
        .unwrap()
        .captures(&text)
        .and_then(|capture| capture[1].parse::<u32>().ok())
        .unwrap_or(80000);
    let id_re = Regex::new(r#"\bid="(\d+)""#).unwrap();
    let used = id_re
        .captures_iter(&text)
        .filter_map(|capture| capture[1].parse::<u32>().ok())
        .collect::<HashSet<_>>();
    while used.contains(&next) || (400..=611).contains(&next) {
        next += 1;
    }
    next
}

const VEHICLE_HANDLING_FIELDS: &[&str] = &[
    "mass",
    "turnMass",
    "dragCoeff",
    "tractionMultiplier",
    "tractionLoss",
    "tractionBias",
    "numberOfGears",
    "maxVelocity",
    "engineAcceleration",
    "engineInertia",
    "driveType",
    "engineType",
    "brakeDeceleration",
    "brakeBias",
    "steeringLock",
    "suspensionForceLevel",
    "suspensionDamping",
    "suspensionUpperLimit",
    "suspensionLowerLimit",
    "suspensionFrontRearBias",
    "collisionDamageMultiplier",
];

const DEFAULT_VEHICLE_CATEGORIES: &[&str] = &[
    "Car",
    "Motorcycle",
    "Bicycle",
    "Boat",
    "Aircraft",
    "Emergency",
    "Utility",
];

fn vehicle_handling_label(property: &str) -> &str {
    match property {
        "mass" => "Mass",
        "turnMass" => "Turn mass",
        "dragCoeff" => "Drag",
        "tractionMultiplier" => "Traction",
        "tractionLoss" => "Traction loss",
        "tractionBias" => "Traction bias",
        "numberOfGears" => "Gears",
        "maxVelocity" => "Top speed",
        "engineAcceleration" => "Acceleration",
        "engineInertia" => "Engine inertia",
        "driveType" => "Drivetrain",
        "engineType" => "Engine",
        "brakeDeceleration" => "Braking",
        "brakeBias" => "Brake bias",
        "steeringLock" => "Steering lock",
        "suspensionForceLevel" => "Suspension force",
        "suspensionDamping" => "Suspension damping",
        "suspensionUpperLimit" => "Suspension upper",
        "suspensionLowerLimit" => "Suspension lower",
        "suspensionFrontRearBias" => "Suspension bias",
        "collisionDamageMultiplier" => "Collision damage",
        _ => property,
    }
}

fn vehicle_handling_options(property: &str) -> Option<&'static [(&'static str, &'static str)]> {
    match property {
        "numberOfGears" => Some(&[
            ("", "Inherit"),
            ("1", "1 gear"),
            ("2", "2 gears"),
            ("3", "3 gears"),
            ("4", "4 gears"),
            ("5", "5 gears"),
            ("6", "6 gears"),
        ]),
        "driveType" => Some(&[
            ("", "Inherit"),
            ("F", "Front-wheel drive"),
            ("R", "Rear-wheel drive"),
            ("4", "All-wheel drive"),
        ]),
        "engineType" => Some(&[
            ("", "Inherit"),
            ("P", "Petrol"),
            ("D", "Diesel"),
            ("E", "Electric"),
        ]),
        _ => None,
    }
}

fn vehicle_handling_display(property: &str, value: &str) -> String {
    vehicle_handling_options(property)
        .and_then(|options| {
            options
                .iter()
                .find(|(stored, _)| stored.eq_ignore_ascii_case(value))
                .map(|(_, label)| (*label).to_string())
        })
        .unwrap_or_else(|| value.to_string())
}

fn load_vehicle_categories(loader: &Path) -> Vec<String> {
    let mut categories = DEFAULT_VEHICLE_CATEGORIES
        .iter()
        .map(|value| (*value).to_string())
        .collect::<BTreeSet<_>>();
    if let Ok(text) = fs::read_to_string(loader.join("vehicle_categories.txt")) {
        categories.extend(
            text.lines()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string),
        );
    }
    if let Ok(text) = fs::read_to_string(loader.join("vehicle_registry.xml")) {
        let category_re = Regex::new(r#"\bcategory="([^"]+)""#).unwrap();
        categories.extend(
            category_re
                .captures_iter(&text)
                .map(|capture| capture[1].trim().to_string())
                .filter(|value| !value.is_empty()),
        );
    }
    categories.into_iter().collect()
}

fn load_used_vehicle_categories(loader: &Path) -> HashSet<String> {
    let Ok(text) = fs::read_to_string(loader.join("vehicle_registry.xml")) else {
        return HashSet::new();
    };
    let category_re = Regex::new(r#"\bcategory="([^"]+)""#).unwrap();
    category_re
        .captures_iter(&text)
        .map(|capture| lower(capture[1].trim()))
        .filter(|value| !value.is_empty())
        .collect()
}

fn save_vehicle_categories(loader: &Path, categories: &[String]) -> Result<(), String> {
    let mut values = categories
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    values.sort_unstable_by_key(|value| value.to_ascii_lowercase());
    values.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
    fs::write(
        loader.join("vehicle_categories.txt"),
        format!("{}\n", values.join("\n")),
    )
    .map_err(|err| format!("Could not save vehicle categories: {err}"))
}

fn base_vehicle_options(app: &AppState, query: &str) -> Vec<(u32, String)> {
    let query = lower(query.trim());
    let mut by_id = BTreeMap::<u32, String>::new();
    for vehicle in &app.vehicles {
        let Some(id) = vehicle.model_id.filter(|id| (400..=611).contains(id)) else {
            continue;
        };
        if !vehicle.readonly {
            continue;
        }
        let label = vehicle.id.replace('_', " ");
        let searchable = lower(&format!("{label} {id}"));
        if query.is_empty() || searchable.contains(&query) {
            by_id.entry(id).or_insert(label);
        }
    }
    by_id.into_iter().collect()
}

fn selected_base_vehicle<'a>(
    app: &'a AppState,
    dialog: &VehicleBuildDialog,
) -> Option<&'a VehicleAsset> {
    let id = dialog.base_model.parse::<u32>().ok()?;
    app.vehicles
        .iter()
        .find(|vehicle| vehicle.readonly && vehicle.model_id == Some(id))
}

fn selected_base_vehicle_label(app: &AppState, dialog: &VehicleBuildDialog) -> String {
    selected_base_vehicle(app, dialog)
        .map(|vehicle| format!("{}  ({})", vehicle.id.replace('_', " "), dialog.base_model))
        .unwrap_or_else(|| format!("GTA model {}", dialog.base_model))
}

pub(crate) fn open_vehicle_build_dialog(app: &mut AppState) {
    if app.vehicle_loader_resource.is_none() {
        open_vehicle_loader_picker(app);
        return;
    }
    let Some(vehicle) = selected_vehicle(app).cloned() else {
        app.status_message = "Select a vehicle to duplicate or load".to_string();
        return;
    };
    let base_model = vehicle
        .model_id
        .filter(|id| (400..=611).contains(id))
        .unwrap_or(400);
    let categories = if app.vehicle_loader_categories.is_empty() {
        DEFAULT_VEHICLE_CATEGORIES
            .iter()
            .map(|value| (*value).to_string())
            .collect()
    } else {
        app.vehicle_loader_categories.clone()
    };
    let category_used = app.vehicle_loader_used_categories.clone();
    app.vehicle_browser.build_dialog = Some(VehicleBuildDialog {
        name: vehicle.id.replace('_', " "),
        category: "Car".to_string(),
        categories,
        category_used,
        category_dropdown_open: false,
        category_scroll: 0.0,
        category_manager_open: false,
        category_manager_scroll: 0.0,
        category_new: String::new(),
        category_new_cursor: 0,
        category_new_anchor: None,
        base_model: base_model.to_string(),
        base_search: String::new(),
        base_dropdown_open: false,
        base_scroll: 0.0,
        wheel_front: format!("{:.3}", vehicle.wheel_front.unwrap_or(0.83)),
        wheel_rear: format!("{:.3}", vehicle.wheel_rear.unwrap_or(0.83)),
        wheel_width: "1.000".to_string(),
        new_vehicle: true,
        active_field: None,
        cursor: 0,
        selection_anchor: None,
        handling: VEHICLE_HANDLING_FIELDS
            .iter()
            .map(|field| (*field, String::new()))
            .collect(),
        handling_dropdown: None,
    });
}

fn build_dialog_field_mut(dialog: &mut VehicleBuildDialog, index: usize) -> Option<&mut String> {
    match index {
        0 => Some(&mut dialog.name),
        2 => Some(&mut dialog.base_search),
        3 => Some(&mut dialog.wheel_front),
        4 => Some(&mut dialog.wheel_rear),
        5 => Some(&mut dialog.wheel_width),
        _ => dialog
            .handling
            .get_mut(index - VEHICLE_BUILD_GENERAL_FIELDS)
            .map(|(_, value)| value),
    }
}

fn slugify_vehicle_name(name: &str) -> String {
    let mut slug = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
        } else if !slug.ends_with('_') && !slug.is_empty() {
            slug.push('_');
        }
    }
    while slug.ends_with('_') {
        slug.pop();
    }
    if slug.is_empty() {
        "custom_vehicle".to_string()
    } else {
        slug
    }
}

fn patch_dff_wheel_track_width(bytes: &mut [u8], width: f32) {
    if (width - 1.0).abs() < 0.0001 {
        return;
    }
    fn walk(bytes: &mut [u8], start: usize, end: usize, width: f32) {
        let mut offset = start;
        while offset + 12 <= end && offset + 12 <= bytes.len() {
            let id = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
            let size =
                u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
            let child_start = offset + 12;
            let child_end = child_start.saturating_add(size);
            if child_end > end || child_end > bytes.len() {
                break;
            }
            if id == 0x0e {
                patch_frame_list(bytes, child_start, child_end, width);
            } else if id != 0x01 {
                walk(bytes, child_start, child_end, width);
            }
            offset = child_end;
        }
    }
    fn patch_frame_list(bytes: &mut [u8], start: usize, end: usize, width: f32) {
        let mut frame_offsets = Vec::new();
        let mut frame_names = Vec::new();
        let mut offset = start;
        while offset + 12 <= end {
            let id = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
            let size =
                u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
            let child_start = offset + 12;
            let child_end = child_start.saturating_add(size);
            if child_end > end || child_end > bytes.len() {
                break;
            }
            if id == 0x01 && size >= 4 {
                let count =
                    u32::from_le_bytes(bytes[child_start..child_start + 4].try_into().unwrap())
                        as usize;
                let mut frame = child_start + 4;
                for _ in 0..count {
                    if frame + 56 > child_end {
                        break;
                    }
                    frame_offsets.push(frame);
                    frame += 56;
                }
            } else if id == 0x03 {
                let mut name = String::new();
                let mut extension = child_start;
                while extension + 12 <= child_end {
                    let ext_id =
                        u32::from_le_bytes(bytes[extension..extension + 4].try_into().unwrap());
                    let ext_size =
                        u32::from_le_bytes(bytes[extension + 4..extension + 8].try_into().unwrap())
                            as usize;
                    let value_start = extension + 12;
                    let value_end = value_start.saturating_add(ext_size);
                    if value_end > child_end {
                        break;
                    }
                    if ext_id == 0x0253_f2fe {
                        let raw = &bytes[value_start..value_end];
                        let len = raw
                            .iter()
                            .position(|value| *value == 0)
                            .unwrap_or(raw.len());
                        name = String::from_utf8_lossy(&raw[..len]).to_string();
                    }
                    extension = value_end;
                }
                frame_names.push(name);
            }
            offset = child_end;
        }
        for (frame, name) in frame_offsets.into_iter().zip(frame_names) {
            if vehicle_wheel_dummy_slot(&name).is_some() {
                let pos = frame + 36;
                let x = f32::from_le_bytes(bytes[pos..pos + 4].try_into().unwrap()) * width;
                bytes[pos..pos + 4].copy_from_slice(&x.to_le_bytes());
            }
        }
    }
    walk(bytes, 0, bytes.len(), width);
}

fn replace_or_append_ide_line(
    path: &Path,
    model: u32,
    slug: &str,
    front: f32,
    rear: f32,
) -> Result<(), String> {
    let old = fs::read_to_string(path).unwrap_or_default();
    let prefix = format!("{model},");
    let mut lines = old
        .lines()
        .filter(|line| !line.trim_start().starts_with(&prefix))
        .map(str::to_string)
        .collect::<Vec<_>>();
    lines.push(format!(
        "{model},\t{slug},\t{slug},\tcar,\tCUSTOM,\tCUSTOM,\tnull,\tnormal,\t10,\t0,\t1f10,\t-1,\t{front:.3},\t{rear:.3},\t0"
    ));
    fs::write(path, format!("{}\n", lines.join("\n")))
        .map_err(|err| format!("Could not update {}: {err}", path.display()))
}

fn write_vehicle_registry_entry(
    loader: &Path,
    dialog: &VehicleBuildDialog,
    id: u32,
    base_model: u32,
    slug: &str,
    front: f32,
    rear: f32,
    width: f32,
) -> Result<(), String> {
    let path = loader.join("vehicle_registry.xml");
    let mut text = fs::read_to_string(&path)
        .unwrap_or_else(|_| "<vehicles nextCustomId=\"80000\">\n</vehicles>\n".to_string());
    let entry_re = Regex::new(&format!(
        r#"(?s)\s*<vehicle\b[^>]*\bid="{}"[^>]*>.*?</vehicle>"#,
        id
    ))
    .unwrap();
    let next_re = Regex::new(r#"nextCustomId="(\d+)""#).unwrap();
    let current_next = next_re
        .captures(&text)
        .and_then(|capture| capture[1].parse::<u32>().ok())
        .unwrap_or(80000);
    text = entry_re.replace_all(&text, "").to_string();
    let next = current_next.max(id.saturating_add(1)).max(80000);
    let next_attr_re = Regex::new(r#"nextCustomId="\d+""#).unwrap();
    if next_attr_re.is_match(&text) {
        text = next_attr_re
            .replace(&text, format!("nextCustomId=\"{next}\""))
            .to_string();
    }
    let mut entry = format!(
        "    <vehicle id=\"{id}\" key=\"{}\" name=\"{}\" category=\"{}\" mode=\"{}\" baseModel=\"{base_model}\" dff=\"Models/{slug}.dff\" txd=\"Models/{slug}.txd\" wheelFront=\"{front:.3}\" wheelRear=\"{rear:.3}\" wheelWidth=\"{width:.3}\">\n",
        xml_escape(slug),
        xml_escape(dialog.name.trim()),
        xml_escape(dialog.category.trim()),
        if dialog.new_vehicle { "new" } else { "replace" },
    );
    for (property, value) in &dialog.handling {
        let value = value.trim();
        if !value.is_empty() {
            entry.push_str(&format!(
                "        <handling property=\"{}\" value=\"{}\" />\n",
                xml_escape(property),
                xml_escape(value)
            ));
        }
    }
    entry.push_str("    </vehicle>\n");
    let marker = "</vehicles>";
    if let Some(position) = text.rfind(marker) {
        text.insert_str(position, &entry);
    } else {
        text = format!("<vehicles nextCustomId=\"{next}\">\n{entry}</vehicles>\n");
    }
    fs::write(&path, text).map_err(|err| format!("Could not update {}: {err}", path.display()))
}

fn build_vehicle_into_loader(
    vehicle: VehicleAsset,
    dialog: VehicleBuildDialog,
    root: PathBuf,
    gta_sa_dir: PathBuf,
    loader: PathBuf,
) -> Result<VehicleBuildResult, String> {
    let base_model = dialog
        .base_model
        .trim()
        .parse::<u32>()
        .map_err(|_| "Base model must be a numeric GTA vehicle ID".to_string())?;
    if !(400..=611).contains(&base_model) {
        return Err("Base model must be between 400 and 611".to_string());
    }
    let id = if dialog.new_vehicle {
        next_custom_vehicle_id(&loader)
    } else {
        base_model
    };
    let front = dialog
        .wheel_front
        .trim()
        .parse::<f32>()
        .map_err(|_| "Front wheel size must be numeric".to_string())?;
    let rear = dialog
        .wheel_rear
        .trim()
        .parse::<f32>()
        .map_err(|_| "Rear wheel size must be numeric".to_string())?;
    let width = dialog
        .wheel_width
        .trim()
        .parse::<f32>()
        .map_err(|_| "Wheel width must be numeric".to_string())?;
    if !(0.1..=4.0).contains(&front)
        || !(0.1..=4.0).contains(&rear)
        || !(0.25..=3.0).contains(&width)
    {
        return Err("Wheel values are outside the supported range".to_string());
    }
    let slug = slugify_vehicle_name(&dialog.name);
    let mut dff = if let Some(path) = vehicle.loose_dff_path.as_ref() {
        fs::read(path).map_err(|err| format!("Could not read {}: {err}", path.display()))?
    } else {
        let entry = find_vehicle_img_entry(&root, &gta_sa_dir, &vehicle.dff, ".dff")
            .ok_or_else(|| format!("Could not find DFF {}", vehicle.dff))?;
        read_img_entry(&entry)
    };
    let txd_entry = find_vehicle_txd_entry(&root, &gta_sa_dir, &vehicle)
        .ok_or_else(|| format!("Could not find TXD {}", vehicle.txd))?;
    let txd = read_img_entry(&txd_entry);
    if dff.is_empty() || txd.is_empty() {
        return Err("The selected DFF or TXD is empty".to_string());
    }
    patch_dff_wheel_track_width(&mut dff, width);
    let models = loader.join("Models");
    fs::create_dir_all(&models)
        .map_err(|err| format!("Could not create {}: {err}", models.display()))?;
    fs::write(models.join(format!("{slug}.dff")), dff)
        .map_err(|err| format!("Could not write DFF: {err}"))?;
    fs::write(models.join(format!("{slug}.txd")), txd)
        .map_err(|err| format!("Could not write TXD: {err}"))?;
    write_vehicle_registry_entry(&loader, &dialog, id, base_model, &slug, front, rear, width)?;
    replace_or_append_ide_line(&loader.join("vehicles.ide"), id, &slug, front, rear)?;
    let loader_scan = scan_vehicle_loader(&loader)?;
    Ok(VehicleBuildResult {
        message: format!(
            "Loaded {} as {} vehicle #{} in {}",
            dialog.name.trim(),
            if dialog.new_vehicle {
                "new"
            } else {
                "replacement"
            },
            id,
            loader.display()
        ),
        loader: loader_scan,
    })
}

pub(crate) fn submit_vehicle_build(app: &mut AppState) {
    if app.vehicle_build_rx.is_some() {
        app.status_message = "A vehicle build is already running".to_string();
        return;
    }
    let Some(dialog) = app.vehicle_browser.build_dialog.clone() else {
        return;
    };
    let Some(vehicle) = selected_vehicle(app).cloned() else {
        return;
    };
    let Some(loader) = app.vehicle_loader_resource.clone() else {
        return;
    };
    let root = app.root.clone();
    let gta_sa_dir = app.gta_sa_dir.clone();
    let (tx, rx) = mpsc::channel();
    app.vehicle_build_rx = Some(rx);
    app.vehicle_browser.build_dialog = None;
    app.status_message = format!("Extracting and loading {}...", dialog.name);
    thread::spawn(move || {
        let _ = tx.send(build_vehicle_into_loader(
            vehicle, dialog, root, gta_sa_dir, loader,
        ));
    });
}

pub(crate) fn poll_vehicle_build(app: &mut AppState) {
    let Some(rx) = app.vehicle_build_rx.as_ref() else {
        return;
    };
    let result = match rx.try_recv() {
        Ok(result) => result,
        Err(mpsc::TryRecvError::Empty) => return,
        Err(mpsc::TryRecvError::Disconnected) => {
            app.vehicle_build_rx = None;
            app.status_message = "Vehicle build worker closed unexpectedly".to_string();
            return;
        }
    };
    app.vehicle_build_rx = None;
    match result {
        Ok(result) => {
            app.vehicle_loader_membership = result.loader.membership;
            app.vehicle_loader_categories = result.loader.categories;
            app.vehicle_loader_used_categories = result.loader.used_categories;
            app.status_message = result.message;
        }
        Err(err) => app.status_message = format!("Vehicle build failed: {err}"),
    }
}

fn index_vehicle_txd_entry(entry: &ImgEntry, txd_name: &str) -> Result<TxdTextureIndex, String> {
    let bytes = fs::read(&entry.img_path)
        .map_err(|err| format!("Could not read TXD {}: {err}", entry.img_path.display()))?;
    let start = entry.offset as usize;
    let end = start.saturating_add(entry.size as usize).min(bytes.len());
    if start >= end {
        return Err(format!("TXD {} is empty", entry.name));
    }
    let mut index = TxdTextureIndex::new();
    index_one_txd(
        &bytes,
        start,
        end,
        &entry.img_path,
        &asset_key(txd_name, ".txd"),
        &mut index,
    );
    Ok(index)
}

fn reload_vehicle_assets_worker(
    vehicle: VehicleAsset,
    root: PathBuf,
    gta_sa_dir: PathBuf,
) -> VehicleReloadResult {
    let identity = vehicle_preview_identity(&vehicle);
    let parsed = (|| {
        let bytes = if let Some(path) = vehicle.loose_dff_path.as_ref() {
            fs::read(path).map_err(|err| format!("Could not read DFF {}: {err}", path.display()))?
        } else {
            let entry = find_vehicle_img_entry(&root, &gta_sa_dir, &vehicle.dff, ".dff")
                .ok_or_else(|| format!("Could not find vehicle DFF {}", vehicle.dff))?;
            read_img_entry(&entry)
        };
        let raw = parse_dff_mesh(&bytes);
        if raw.vertices.is_empty() || raw.triangles.is_empty() {
            Err(format!("Vehicle DFF {} did not parse", vehicle.dff))
        } else {
            let collision =
                parse_embedded_vehicle_collision(&bytes, &vehicle.dff).map(|(mesh, _)| mesh);
            Ok((raw, collision))
        }
    })();
    let (raw, embedded_collision) = match parsed {
        Ok((raw, collision)) => (Ok(raw), collision),
        Err(err) => (Err(err), None),
    };

    let (txd_index, txd_error) = if vehicle.txd.trim().is_empty() {
        (TxdTextureIndex::new(), None)
    } else {
        match find_vehicle_txd_entry(&root, &gta_sa_dir, &vehicle) {
            Some(entry) => match index_vehicle_txd_entry(&entry, &vehicle.txd) {
                Ok(index) => (index, None),
                Err(err) => (TxdTextureIndex::new(), Some(err)),
            },
            None => (
                TxdTextureIndex::new(),
                Some(format!("Could not find vehicle TXD {}", vehicle.txd)),
            ),
        }
    };

    VehicleReloadResult {
        identity,
        vehicle_id: vehicle.id,
        txd_name: vehicle.txd,
        raw,
        embedded_collision,
        txd_index,
        txd_error,
    }
}

pub(crate) fn reload_selected_vehicle(app: &mut AppState) {
    let Some(vehicle) = selected_vehicle(app).cloned() else {
        app.status_message = "Select a custom vehicle before reloading".to_string();
        return;
    };
    if vehicle.readonly {
        app.status_message = "Only custom vehicles can be reloaded".to_string();
        return;
    }
    if app.vehicle_browser.reload_rx.is_some() {
        app.status_message = "A custom vehicle reload is already running".to_string();
        return;
    }

    let root = app.root.clone();
    let gta_sa_dir = app.gta_sa_dir.clone();
    let vehicle_id = vehicle.id.clone();
    let (tx, rx) = mpsc::channel();
    app.vehicle_browser.reload_rx = Some(rx);
    app.status_message = format!("Reloading custom vehicle {vehicle_id}...");
    thread::spawn(move || {
        let _ = tx.send(reload_vehicle_assets_worker(vehicle, root, gta_sa_dir));
    });
}

pub(crate) fn poll_vehicle_reload(app: &mut AppState) {
    let Some(rx) = app.vehicle_browser.reload_rx.as_ref() else {
        return;
    };
    let result = match rx.try_recv() {
        Ok(result) => result,
        Err(mpsc::TryRecvError::Empty) => return,
        Err(mpsc::TryRecvError::Disconnected) => {
            app.vehicle_browser.reload_rx = None;
            app.status_message = "Custom vehicle reload stopped unexpectedly".to_string();
            return;
        }
    };
    app.vehicle_browser.reload_rx = None;

    let VehicleReloadResult {
        identity,
        vehicle_id,
        txd_name,
        raw,
        embedded_collision,
        txd_index,
        txd_error,
    } = result;
    let raw = match raw {
        Ok(raw) => raw,
        Err(err) => {
            app.status_message = err;
            return;
        }
    };

    if !txd_name.trim().is_empty() {
        remove_txd_from_texture_index(app, &txd_name);
        for (texture_name, mut entries) in txd_index {
            app.txd_textures
                .entry(texture_name)
                .or_default()
                .append(&mut entries);
        }
        invalidate_cached_txd_textures(app, &txd_name, None);
        app.vehicle_browser.texture_previews.clear();
    }

    let Some(vehicle) = selected_vehicle(app).cloned() else {
        app.status_message = format!("Reloaded custom vehicle {vehicle_id}");
        return;
    };
    if vehicle_preview_identity(&vehicle) != identity {
        app.status_message = format!("Reloaded assets for custom vehicle {vehicle_id}");
        return;
    }

    app.vehicle_browser.preview_key =
        format!("{}|lights={}", identity, app.vehicle_browser.lights_on);
    app.vehicle_browser.embedded_collision = embedded_collision;
    if !install_vehicle_preview(app, &vehicle, raw) {
        app.status_message = format!("Vehicle DFF {} has no renderable geometry", vehicle.dff);
        return;
    }
    app.status_message = if let Some(err) = txd_error {
        format!("Reloaded {vehicle_id} DFF; {err}")
    } else if txd_name.trim().is_empty() {
        format!("Reloaded custom vehicle {vehicle_id} DFF")
    } else {
        format!("Reloaded custom vehicle {vehicle_id} DFF + TXD")
    };
}

pub(crate) fn vehicle_preview_placement(vehicle: &VehicleAsset) -> Placement {
    Placement {
        id: vehicle.id.clone(),
        dff: vehicle.dff.clone(),
        zone: vehicle.source.clone(),
        tag: "vehicle".to_string(),
        attrs: BTreeMap::new(),
        pos: V3::default(),
        rot: V3::default(),
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct VehiclePreviewMotion {
    chassis_heave: f32,
    chassis_pitch: f32,
    front_wheel_travel: f32,
    rear_wheel_travel: f32,
    front_wheel_angle: f32,
    rear_wheel_angle: f32,
}

fn vehicle_handling_f32(vehicle: &VehicleAsset, property: &str, fallback: f32) -> f32 {
    vehicle
        .handling
        .get(property)
        .and_then(|value| value.trim().parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(fallback)
}

fn vehicle_preview_phase(id: &str) -> f32 {
    let hash = id.bytes().fold(2_166_136_261u32, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(16_777_619)
    });
    hash as f32 / u32::MAX as f32
}

fn vehicle_suspension_response(time: f32, frequency: f32, decay: f32) -> f32 {
    if time < 0.0 {
        0.0
    } else {
        (-decay * time).exp() * (frequency * time).sin()
    }
}

fn vehicle_obstacle_pulse(time: f32, duration: f32) -> f32 {
    if !(0.0..duration).contains(&time) {
        0.0
    } else {
        (std::f32::consts::PI * time / duration).sin()
    }
}

fn vehicle_preview_motion(vehicle: &VehicleAsset, time: f32) -> VehiclePreviewMotion {
    // handling.cfg velocities are km/h. The preview cruises at a restrained
    // fraction of top speed so individual spokes remain readable.
    let max_velocity = vehicle_handling_f32(vehicle, "maxVelocity", 160.0).clamp(30.0, 420.0);
    let engine_acceleration =
        vehicle_handling_f32(vehicle, "engineAcceleration", 20.0).clamp(1.0, 80.0);
    let cruise_fraction = 0.14 + (engine_acceleration / 80.0) * 0.10;
    let linear_speed = (max_velocity / 3.6 * cruise_fraction).clamp(3.5, 20.0);
    let front_radius = vehicle.wheel_front.unwrap_or(0.83).abs().clamp(0.25, 2.0);
    let rear_radius = vehicle.wheel_rear.unwrap_or(0.83).abs().clamp(0.25, 2.0);

    let suspension_upper =
        vehicle_handling_f32(vehicle, "suspensionUpperLimit", 0.28).clamp(-1.5, 1.5);
    let suspension_lower =
        vehicle_handling_f32(vehicle, "suspensionLowerLimit", -0.16).clamp(-1.5, 1.5);
    let suspension_travel = (suspension_upper - suspension_lower).abs().clamp(0.08, 0.9);
    let suspension_force =
        vehicle_handling_f32(vehicle, "suspensionForceLevel", 1.0).clamp(0.2, 4.0);
    let suspension_damping =
        vehicle_handling_f32(vehicle, "suspensionDamping", 0.1).clamp(0.0, 2.0);
    let suspension_bias =
        vehicle_handling_f32(vehicle, "suspensionFrontRearBias", 0.5).clamp(0.15, 0.85);
    let mass = vehicle_handling_f32(vehicle, "mass", 1_400.0).clamp(400.0, 8_000.0);
    let mass_response = (1_400.0 / mass).sqrt().clamp(0.55, 1.35);

    // Each vehicle gets a stable offset and a 6–9 second interval. The rear
    // axle reaches the same obstacle after a speed-dependent wheelbase delay.
    let phase = vehicle_preview_phase(&vehicle.id);
    let interval = 6.0 + phase * 3.0;
    let event_time = (time + phase * interval).rem_euclid(interval);
    let axle_delay = (2.65 / linear_speed).clamp(0.16, 0.55);
    let frequency = 6.0 + suspension_force.sqrt() * 3.2;
    let decay = 1.8 + suspension_damping * 5.5;
    let front_response = vehicle_suspension_response(event_time, frequency, decay);
    let rear_response = vehicle_suspension_response(event_time - axle_delay, frequency, decay);
    let contact_duration = (0.24 / suspension_force.sqrt()).clamp(0.11, 0.28);
    let front_obstacle = vehicle_obstacle_pulse(event_time, contact_duration);
    let rear_obstacle = vehicle_obstacle_pulse(event_time - axle_delay, contact_duration);

    let body_amplitude = suspension_travel * 0.18 * mass_response;
    let chassis_heave = body_amplitude
        * (front_response * suspension_bias + rear_response * (1.0 - suspension_bias));
    let chassis_pitch =
        (front_response - rear_response) * suspension_travel * mass_response * 0.018;
    let wheel_amplitude = suspension_travel * 0.32;

    VehiclePreviewMotion {
        chassis_heave,
        chassis_pitch,
        front_wheel_travel: wheel_amplitude * front_obstacle - chassis_heave * 0.18,
        rear_wheel_travel: wheel_amplitude * rear_obstacle - chassis_heave * 0.18,
        // GTA vehicles face +Y, so forward rolling is a negative rotation
        // around the left-to-right (+X) axle.
        front_wheel_angle: (-time * linear_speed / front_radius).rem_euclid(std::f32::consts::TAU),
        rear_wheel_angle: (-time * linear_speed / rear_radius).rem_euclid(std::f32::consts::TAU),
    }
}

fn vehicle_preview_model_matrix(
    placement: &Placement,
    mesh: &RenderMesh,
    motion: VehiclePreviewMotion,
) -> Mat4 {
    if !mesh
        .components
        .iter()
        .any(|name| vehicle_preview_wheel_axle(name).is_some())
    {
        return placement_matrix(placement);
    }
    let center = (mesh.bounds.min + mesh.bounds.max) * 0.5;
    placement_matrix(placement)
        * Mat4::from_translation(Vec3::Z * motion.chassis_heave)
        * Mat4::from_translation(center)
        * Mat4::from_rotation_x(motion.chassis_pitch)
        * Mat4::from_translation(-center)
}

fn draw_vehicle_photo_backdrop() {
    unsafe {
        gl::PushAttrib(gl::ALL_ATTRIB_BITS);
        gl::Disable(gl::DEPTH_TEST);
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::ALPHA_TEST);
        gl::Disable(gl::CULL_FACE);
        gl::Disable(gl::BLEND);
        gl::MatrixMode(gl::PROJECTION);
        gl::PushMatrix();
        gl::LoadIdentity();
        gl::MatrixMode(gl::MODELVIEW);
        gl::PushMatrix();
        gl::LoadIdentity();

        gl::Begin(gl::QUADS);
        gl::Color4f(0.025, 0.038, 0.065, 1.0);
        gl::Vertex2f(-1.0, 1.0);
        gl::Vertex2f(1.0, 1.0);
        gl::Color4f(0.12, 0.17, 0.25, 1.0);
        gl::Vertex2f(1.0, -1.0);
        gl::Vertex2f(-1.0, -1.0);
        gl::End();

        // Broad studio pool behind the car, fading into the darker backdrop.
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE);
        gl::Begin(gl::TRIANGLE_FAN);
        gl::Color4f(0.16, 0.25, 0.42, 0.62);
        gl::Vertex2f(0.12, 0.02);
        gl::Color4f(0.08, 0.12, 0.20, 0.0);
        for step in 0..=48 {
            let angle = step as f32 / 48.0 * std::f32::consts::TAU;
            gl::Vertex2f(0.12 + angle.cos() * 1.15, 0.02 + angle.sin() * 0.82);
        }
        gl::End();

        gl::PopMatrix();
        gl::MatrixMode(gl::PROJECTION);
        gl::PopMatrix();
        gl::MatrixMode(gl::MODELVIEW);
        gl::DepthMask(gl::TRUE);
        gl::Clear(gl::DEPTH_BUFFER_BIT);
        gl::PopAttrib();
    }
}

fn draw_vehicle_photo_stage(mesh: &RenderMesh) {
    let center = (mesh.bounds.min + mesh.bounds.max) * 0.5;
    let extents = (mesh.bounds.max - mesh.bounds.min) * 0.5;
    let floor_z = mesh.bounds.min.z - 0.04;
    let radius = (extents.length() * 7.0).max(32.0);
    unsafe {
        gl::PushAttrib(gl::ALL_ATTRIB_BITS);
        gl::Enable(gl::DEPTH_TEST);
        gl::DepthMask(gl::TRUE);
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::ALPHA_TEST);
        gl::Disable(gl::CULL_FACE);
        gl::Disable(gl::BLEND);
        gl::Begin(gl::TRIANGLE_FAN);
        gl::Color4f(0.15, 0.17, 0.21, 1.0);
        gl::Vertex3f(center.x, center.y, floor_z);
        gl::Color4f(0.035, 0.045, 0.065, 1.0);
        for step in 0..=64 {
            let angle = step as f32 / 64.0 * std::f32::consts::TAU;
            gl::Vertex3f(
                center.x + angle.cos() * radius,
                center.y + angle.sin() * radius,
                floor_z,
            );
        }
        gl::End();

        // Soft contact shadow grounds the vehicle without a hard-edged decal.
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::DepthMask(gl::FALSE);
        gl::Begin(gl::TRIANGLE_FAN);
        gl::Color4f(0.0, 0.0, 0.0, 0.58);
        gl::Vertex3f(center.x, center.y, floor_z + 0.008);
        gl::Color4f(0.0, 0.0, 0.0, 0.0);
        for step in 0..=64 {
            let angle = step as f32 / 64.0 * std::f32::consts::TAU;
            gl::Vertex3f(
                center.x + angle.cos() * extents.x.max(1.0) * 1.08,
                center.y + angle.sin() * extents.y.max(1.0) * 0.98,
                floor_z + 0.008,
            );
        }
        gl::End();
        gl::PopAttrib();
    }
}

fn vehicle_generated_light_coronas(
    app: &AppState,
    mesh: &RenderMesh,
    model: Mat4,
    motion: VehiclePreviewMotion,
) -> Vec<Dff2dEffect> {
    let center = (mesh.bounds.min + mesh.bounds.max) * 0.5;
    let extents = (mesh.bounds.max - mesh.bounds.min) * 0.5;
    let local_camera = model.inverse().transform_point3(app.camera.pos);
    // The default showcase angle looks at the vehicle's -Y/front side. Only
    // synthesize the pair on the side facing the camera so rear lamps cannot
    // bleed through the body while the corona layer intentionally ignores depth.
    let camera_at_front = local_camera.y <= center.y;
    let mut sums = [Vec3::ZERO; 4];
    let mut counts = [0usize; 4];
    for part in mesh.parts.iter().filter(|part| {
        part.emissive || part.vehicle_material_role == Some(VehicleMaterialRole::Light)
    }) {
        let component = vehicle_component_preview_matrix(app, mesh, part.component, motion);
        for vertex in &part.cpu_vertices {
            let position = component.transform_point3(to_mq(vertex.pos));
            let front = usize::from(position.y <= center.y);
            let right = usize::from(position.x >= center.x);
            let bucket = front * 2 + right;
            sums[bucket] += position;
            counts[bucket] += 1;
        }
    }

    let sprite_radius = extents.x.abs().clamp(0.8, 4.0) * 0.17;
    sums.into_iter()
        .zip(counts)
        .enumerate()
        .filter_map(|(bucket, (sum, count))| {
            if count == 0 {
                return None;
            }
            let front = bucket >= 2;
            if front != camera_at_front {
                return None;
            }
            let mut payload = vec![0u8; 49];
            let color = if front {
                [255, 244, 212, 232]
            } else {
                [255, 42, 24, 224]
            };
            payload[0..4].copy_from_slice(&color);
            payload[4..8].copy_from_slice(&1_000.0f32.to_le_bytes());
            payload[12..16].copy_from_slice(&(sprite_radius / 12.0).to_le_bytes());
            let texture = b"coronastar\0";
            payload[25..25 + texture.len()].copy_from_slice(texture);
            Some(Dff2dEffect {
                position: from_mq(sum / count as f32),
                effect_id: 0,
                payload,
            })
        })
        .collect()
}

pub(crate) fn draw_vehicle_preview(app: &mut AppState) -> bool {
    if app.active_tab != AppTab::Vehicles {
        return false;
    }
    ensure_vehicle_preview(app);
    let Some(vehicle) = selected_vehicle(app).cloned() else {
        return false;
    };
    let placement = vehicle_preview_placement(&vehicle);
    let ambient_lift = scene_ambient_lift_from_timecyc(&app.timecyc);
    let motion = vehicle_preview_motion(&vehicle, get_time() as f32);
    let photo_mode = app.vehicle_browser.photo_mode;
    let photo_reflection_program = if photo_mode {
        ensure_vehicle_photo_reflection_program(&mut app.vehicle_browser)
    } else {
        0
    };
    if photo_mode {
        draw_vehicle_photo_backdrop();
    }
    let mut preview_model = None;
    let mut preview_coronas = Vec::new();
    if let Some(mesh) = app.vehicle_browser.preview_mesh.as_ref() {
        if photo_mode {
            draw_vehicle_photo_stage(mesh);
        }
        let model = vehicle_preview_model_matrix(&placement, mesh, motion);
        let (parts, vertices) = draw_vehicle_render_mesh(
            app,
            mesh,
            model,
            motion,
            ambient_lift,
            photo_reflection_program,
        );
        preview_model = Some(model);
        if app.vehicle_browser.lights_on {
            preview_coronas = mesh
                .effects_2dfx
                .iter()
                .filter(|effect| effect.effect_id == 0)
                .cloned()
                .collect();
            if preview_coronas.is_empty() {
                preview_coronas = vehicle_generated_light_coronas(app, mesh, model, motion);
            }
        }
        app.last_drawn_placements = usize::from(parts > 0);
        app.last_drawn_parts = parts;
        app.last_drawn_vertices = vertices;
    }
    if !photo_mode && let Some(collision) = app.vehicle_browser.embedded_collision.as_ref() {
        unsafe {
            gl::Disable(gl::LIGHTING);
            gl::Disable(gl::TEXTURE_2D);
        }
        if app.vehicle_browser.show_collision_mesh {
            let model = preview_model.unwrap_or(Mat4::IDENTITY).to_cols_array();
            draw_collision_mesh_with_options(
                collision,
                None,
                &BTreeSet::new(),
                &BTreeSet::new(),
                None,
                &BTreeSet::new(),
                None,
                None,
                &model,
                app.camera.pos,
                false,
                false,
                false,
                false,
            );
        }
        if app.vehicle_browser.show_collision_volumes {
            unsafe {
                gl::PushMatrix();
                if let Some(model) = preview_model {
                    gl::MultMatrixf(model.to_cols_array().as_ptr());
                }
                draw_collision_primitives(collision);
                gl::PopMatrix();
            }
        }
    }
    if let Some(model) = preview_model
        && !preview_coronas.is_empty()
    {
        draw_vehicle_2dfx_coronas(app, &preview_coronas, model);
    }
    if !photo_mode {
        draw_vehicle_rotation_gimbal(app);
    }
    true
}

pub(crate) fn vehicle_part_kind(part: &RenderPart) -> &'static str {
    let texture = lower(part.texture_name.trim());
    if texture.contains("vol") || texture.contains("shadow") {
        "volume"
    } else if texture.is_empty()
        || texture.contains("coll")
        || texture.contains("collision")
        || texture.contains("dam")
    {
        "collision"
    } else {
        "body"
    }
}

pub(crate) fn vehicle_part_visible_by_group(app: &AppState, part: &RenderPart) -> bool {
    match vehicle_part_kind(part) {
        "collision" => app.vehicle_browser.show_collision_mesh,
        "volume" => app.vehicle_browser.show_collision_volumes,
        _ => app.vehicle_browser.show_body,
    }
}

fn vehicle_reflection_texture(texture_name: &str) -> bool {
    matches!(
        lower(texture_name.trim()).as_str(),
        "vehiclegrunge256" | "vehiclelights128"
    )
}

fn selected_vehicle_texture_name(app: &AppState) -> Option<String> {
    let mesh = app.vehicle_browser.preview_mesh.as_ref()?;
    let selected = app.vehicle_browser.selected_part?;
    let name = mesh.parts.get(selected)?.texture_name.trim();
    (!name.is_empty()).then(|| lower(name))
}

fn vehicle_component_hidden_by_options(app: &AppState, name: &str) -> bool {
    let key = lower(name.trim());
    (app.vehicle_browser.hide_damaged && key.contains("_dam"))
        || (app.vehicle_browser.hide_vlo && key.contains("_vlo"))
}

fn vehicle_part_component_name<'a>(mesh: &'a RenderMesh, part: &RenderPart) -> &'a str {
    mesh.components
        .get(part.component)
        .map(String::as_str)
        .unwrap_or_default()
}

fn vehicle_component_center(mesh: &RenderMesh, component: usize) -> Option<Vec3> {
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    let mut any = false;
    for part in mesh.parts.iter().filter(|part| part.component == component) {
        for vertex in &part.cpu_vertices {
            let p = to_mq(vertex.pos);
            min = min.min(p);
            max = max.max(p);
            any = true;
        }
    }
    any.then_some((min + max) * 0.5)
}

fn vehicle_component_pivot(mesh: &RenderMesh, component: usize) -> Option<Vec3> {
    mesh.component_pivots
        .get(component)
        .copied()
        .flatten()
        .or_else(|| vehicle_component_center(mesh, component))
}

fn vehicle_gizmo_visual_length(app: &AppState, origin: Vec3) -> f32 {
    let scale = gizmo_scale(app);
    (gizmo_visual_length(app, origin) * 0.62).clamp(1.6 * scale, 4.6 * scale)
}

fn vehicle_component_rotation_matrix(rotation: V3) -> Mat4 {
    Mat4::from_rotation_z(rotation.z)
        * Mat4::from_rotation_y(rotation.y)
        * Mat4::from_rotation_x(rotation.x)
}

fn vehicle_component_preview_matrix(
    app: &AppState,
    mesh: &RenderMesh,
    component: usize,
    motion: VehiclePreviewMotion,
) -> Mat4 {
    let rotation = app
        .vehicle_browser
        .component_rotations
        .get(&component)
        .copied()
        .unwrap_or_default();
    let wheel = mesh
        .components
        .get(component)
        .and_then(|name| vehicle_preview_wheel_axle(name));
    let (wheel_travel, wheel_angle) = match wheel {
        Some("front") => (motion.front_wheel_travel, motion.front_wheel_angle),
        Some("rear") => (motion.rear_wheel_travel, motion.rear_wheel_angle),
        Some(_) => (
            (motion.front_wheel_travel + motion.rear_wheel_travel) * 0.5,
            (motion.front_wheel_angle + motion.rear_wheel_angle) * 0.5,
        ),
        None => (0.0, 0.0),
    };
    let animate = wheel.is_some() || rotation != V3::default();
    if !animate {
        return Mat4::IDENTITY;
    }
    let center = vehicle_component_pivot(mesh, component)
        .unwrap_or((mesh.bounds.min + mesh.bounds.max) * 0.5);
    Mat4::from_translation(Vec3::Z * wheel_travel)
        * Mat4::from_translation(center)
        * Mat4::from_rotation_x(rotation.x + wheel_angle)
        * Mat4::from_rotation_y(rotation.y)
        * Mat4::from_rotation_z(rotation.z)
        * Mat4::from_translation(-center)
}

fn vehicle_ring_basis(app: &AppState, component: usize, axis: GizmoAxis) -> (Vec3, Vec3) {
    let (a, b) = match axis {
        GizmoAxis::X => (Vec3::Y, Vec3::Z),
        GizmoAxis::Y => (Vec3::X, Vec3::Z),
        GizmoAxis::Z => (Vec3::X, Vec3::Y),
    };
    if app.transform_space == TransformSpace::Local {
        let rotation = app
            .vehicle_browser
            .component_rotations
            .get(&component)
            .copied()
            .unwrap_or_default();
        let matrix = vehicle_component_rotation_matrix(rotation);
        (transform_vec(matrix, a), transform_vec(matrix, b))
    } else {
        (a, b)
    }
}

fn vehicle_gizmo_axis_at(app: &AppState, viewport: Rect, mouse: Vec2) -> Option<GizmoAxis> {
    if app.transform_mode != TransformMode::Rotate {
        return None;
    }
    let component = app.vehicle_browser.selected_component?;
    let mesh = app.vehicle_browser.preview_mesh.as_ref()?;
    let vehicle = selected_vehicle(app)?;
    let placement = vehicle_preview_placement(vehicle);
    let motion = vehicle_preview_motion(vehicle, get_time() as f32);
    let model = vehicle_preview_model_matrix(&placement, mesh, motion);
    let origin = vehicle_component_world_matrix(app, mesh, component, model, motion)
        .transform_point3(vehicle_component_pivot(mesh, component)?);
    let length = vehicle_gizmo_visual_length(app, origin);
    let mut best = None;
    let mut best_dist = 18.0;
    for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
        let (a, b) = vehicle_ring_basis(app, component, axis);
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
    best
}

/// World transform for a component's geometry, matching what `draw_vehicle_render_mesh`
/// applies (model placement plus any live rotation about the component pivot).
fn vehicle_component_world_matrix(
    app: &AppState,
    mesh: &RenderMesh,
    component: usize,
    model: Mat4,
    motion: VehiclePreviewMotion,
) -> Mat4 {
    model * vehicle_component_preview_matrix(app, mesh, component, motion)
}

/// Pick the component under the cursor by casting a ray through the actual
/// geometry and taking the nearest triangle hit. This is far more consistent
/// than matching against projected component centres, which misfires on large,
/// offset, or overlapping components.
fn pick_vehicle_component(app: &AppState, mouse: Vec2) -> Option<usize> {
    let mesh = app.vehicle_browser.preview_mesh.as_ref()?;
    let vehicle = selected_vehicle(app)?;
    let viewport = vehicle_preview_viewport_rect(app);
    let (origin, dir) = viewport_ray(app, viewport, mouse)?;
    let placement = vehicle_preview_placement(vehicle);
    let motion = vehicle_preview_motion(vehicle, get_time() as f32);
    let model = vehicle_preview_model_matrix(&placement, mesh, motion);
    let mut best: Option<(usize, f32)> = None;
    for (idx, part) in mesh.parts.iter().enumerate() {
        // Honour the same visibility filters the viewport uses so you can only
        // click parts you can actually see.
        let component_name = vehicle_part_component_name(mesh, part);
        if vehicle_component_hidden_by_options(app, component_name)
            || app.vehicle_browser.hidden_parts.contains(&idx)
            || app
                .vehicle_browser
                .hidden_components
                .contains(&part.component)
            || !vehicle_part_visible_by_group(app, part)
        {
            continue;
        }
        let world = vehicle_component_world_matrix(app, mesh, part.component, model, motion);
        let verts = &part.cpu_vertices;
        let mut i = 0;
        while i + 3 <= verts.len() {
            let a = world.transform_point3(to_mq(verts[i].pos));
            let b = world.transform_point3(to_mq(verts[i + 1].pos));
            let c = world.transform_point3(to_mq(verts[i + 2].pos));
            if let Some(t) = ray_triangle(origin, dir, a, b, c) {
                if best.is_none_or(|(_, best_t)| t < best_t) {
                    best = Some((part.component, t));
                }
            }
            i += 3;
        }
    }
    best.map(|(component, _)| component)
}

fn draw_vehicle_rotation_gimbal(app: &AppState) {
    if app.transform_mode != TransformMode::Rotate {
        return;
    }
    let Some(component) = app.vehicle_browser.selected_component else {
        return;
    };
    let Some(mesh) = app.vehicle_browser.preview_mesh.as_ref() else {
        return;
    };
    let Some(vehicle) = selected_vehicle(app) else {
        return;
    };
    let placement = vehicle_preview_placement(vehicle);
    let motion = vehicle_preview_motion(vehicle, get_time() as f32);
    let model = vehicle_preview_model_matrix(&placement, mesh, motion);
    let Some(local_center) = vehicle_component_pivot(mesh, component) else {
        return;
    };
    let center = vehicle_component_world_matrix(app, mesh, component, model, motion)
        .transform_point3(local_center);
    let length = vehicle_gizmo_visual_length(app, center);
    unsafe {
        gl::PushAttrib(
            gl::ENABLE_BIT | gl::LINE_BIT | gl::POINT_BIT | gl::CURRENT_BIT | gl::DEPTH_BUFFER_BIT,
        );
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::ALPHA_TEST);
        gl::Disable(gl::DEPTH_TEST);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        draw_vehicle_gizmo_center_ring(center, app.camera.pos, length, gizmo_scale(app));
        for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
            let (a, b) = vehicle_ring_basis(app, component, axis);
            draw_rotation_ring(
                center,
                a,
                b,
                length * 0.75,
                axis_color(axis),
                app.vehicle_browser.hovered_gizmo == Some(axis)
                    || app
                        .vehicle_browser
                        .gizmo_drag
                        .is_some_and(|drag| drag.axis == axis),
            );
        }
        gl::PopAttrib();
    }
}

fn draw_vehicle_gizmo_center_ring(origin: Vec3, camera_pos: Vec3, length: f32, scale: f32) {
    let view_dir = (camera_pos - origin).normalize_or_zero();
    let view_dir = if view_dir.length_squared() > 0.0001 {
        view_dir
    } else {
        Vec3::Z
    };
    let tangent = if view_dir.z.abs() < 0.9 {
        view_dir.cross(Vec3::Z).normalize_or_zero()
    } else {
        view_dir.cross(Vec3::X).normalize_or_zero()
    };
    let bitangent = view_dir.cross(tangent).normalize_or_zero();
    let radius = (length * 0.12).clamp(3.0 * scale, 13.0 * scale);
    unsafe {
        gl::LineWidth(1.5);
        gl::Color4f(0.88, 0.88, 0.88, 0.72);
        gl::Begin(gl::LINE_LOOP);
        for i in 0..40 {
            let t = i as f32 / 40.0 * std::f32::consts::TAU;
            let p = origin + tangent * t.cos() * radius + bitangent * t.sin() * radius;
            gl::Vertex3f(p.x, p.y, p.z);
        }
        gl::End();
    }
}

fn draw_vehicle_render_mesh(
    app: &AppState,
    mesh: &RenderMesh,
    model: Mat4,
    motion: VehiclePreviewMotion,
    ambient_lift: V3,
    photo_reflection_program: u32,
) -> (usize, usize) {
    let model_cols = model.to_cols_array();
    let mut parts = 0usize;
    let mut vertices = 0usize;
    let selected_texture = selected_vehicle_texture_name(app);
    unsafe {
        gl::Disable(gl::CULL_FACE);
        gl::LightModeli(gl::LIGHT_MODEL_TWO_SIDE, gl::TRUE as i32);
        gl::PushMatrix();
        gl::MultMatrixf(model_cols.as_ptr());
        for (idx, part) in mesh.parts.iter().enumerate() {
            let component_name = vehicle_part_component_name(mesh, part);
            if vehicle_component_hidden_by_options(app, component_name) {
                continue;
            }
            if app.vehicle_browser.hidden_parts.contains(&idx) {
                continue;
            }
            if app
                .vehicle_browser
                .hidden_components
                .contains(&part.component)
            {
                continue;
            }
            if !vehicle_part_visible_by_group(app, part) {
                continue;
            }
            let component_matrix =
                vehicle_component_preview_matrix(app, mesh, part.component, motion);
            let transform_component = component_matrix != Mat4::IDENTITY;
            let world_matrix = if transform_component {
                gl::PushMatrix();
                gl::MultMatrixf(component_matrix.to_cols_array().as_ptr());
                // Match the GL stack above so the reflection uses the same world
                // transform the geometry is actually drawn with.
                model * component_matrix
            } else {
                model
            };
            draw_vehicle_render_part(part, app.vehicle_browser.show_textures, ambient_lift);
            if photo_reflection_program != 0 {
                draw_vehicle_photo_reflection_pass(
                    part,
                    world_matrix,
                    app.camera.pos,
                    photo_reflection_program,
                    app.vehicle_browser.show_textures,
                );
            } else if app.vehicle_browser.show_textures
                && vehicle_reflection_texture(&part.texture_name)
            {
                draw_vehicle_reflection_pass(part, world_matrix, app.camera.pos);
            }
            let texture_selected = selected_texture
                .as_ref()
                .is_some_and(|texture| *texture == lower(part.texture_name.trim()));
            if !app.vehicle_browser.photo_mode
                && texture_selected
                && app.vehicle_browser.selected_part != Some(idx)
            {
                draw_vehicle_texture_match_highlight(part);
            }
            if !app.vehicle_browser.photo_mode
                && (app.vehicle_browser.selected_component == Some(part.component)
                    || app.vehicle_browser.selected_part == Some(idx))
            {
                draw_vehicle_render_part_highlight(
                    part,
                    app.vehicle_browser.selected_part == Some(idx),
                );
            }
            if transform_component {
                gl::PopMatrix();
            }
            parts += 1;
            vertices += part.vertices;
        }
        gl::PopMatrix();
        gl::LightModeli(gl::LIGHT_MODEL_TWO_SIDE, gl::FALSE as i32);
        gl::Enable(gl::CULL_FACE);
    }
    (parts, vertices)
}

pub(crate) fn draw_vehicle_render_part(part: &RenderPart, show_textures: bool, ambient_lift: V3) {
    unsafe {
        gl::Disable(gl::CULL_FACE);
        gl::LightModeli(gl::LIGHT_MODEL_TWO_SIDE, gl::TRUE as i32);
        match part.transparency {
            TransparencyMode::Blend => {
                gl::Enable(gl::BLEND);
                gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
                gl::Disable(gl::ALPHA_TEST);
                gl::DepthMask(gl::FALSE);
            }
            TransparencyMode::Opaque | TransparencyMode::Cutout => {
                gl::Disable(gl::BLEND);
                gl::Enable(gl::ALPHA_TEST);
                gl::AlphaFunc(gl::GREATER, 0.08);
                gl::DepthMask(gl::TRUE);
            }
        }
        if part.use_lighting && !part.emissive {
            gl::Enable(gl::LIGHTING);
        } else {
            gl::Disable(gl::LIGHTING);
        }
        if show_textures && part.texture != 0 {
            gl::Enable(gl::TEXTURE_2D);
            gl::BindTexture(gl::TEXTURE_2D, part.texture);
        } else {
            gl::Disable(gl::TEXTURE_2D);
        }
        // Vehicle bodies ship with prelit vertex colours, so GL lighting is off
        // for them and every face renders at one flat tone. Add a cheap Lambert
        // term (key light from above matching the scene sun, plus a dim fill from
        // below so undersides don't go black) to give the surfaces some form.
        // Emissive light parts skip this shading entirely and draw full-bright.
        let cpu_shade = !part.use_lighting && !part.emissive;
        let key_dir = Vec3::new(-0.28, -0.36, 0.89).normalize_or_zero();
        let fill_dir = Vec3::new(0.30, 0.45, -0.15).normalize_or_zero();
        gl::Begin(gl::TRIANGLES);
        for vertex in &part.cpu_vertices {
            let mut color = if part.emissive {
                // Switched-on light: draw the texture at full brightness,
                // ignoring scene lighting so head/tail lights actually glow.
                V3 {
                    x: 1.0,
                    y: 1.0,
                    z: 1.0,
                }
            } else {
                display_vertex_color(
                    vertex.color,
                    ambient_lift,
                    part.material_color,
                    part.material_ambient,
                )
            };
            if cpu_shade {
                let normal = to_mq(vertex.normal).normalize_or_zero();
                let key = normal.dot(key_dir).max(0.0);
                let fill = normal.dot(fill_dir).max(0.0);
                // Ambient floor keeps colours readable; key/fill add the gradient.
                let shade = (0.55 + 0.45 * key + 0.12 * fill).clamp(0.45, 1.05);
                color = V3 {
                    x: (color.x * shade).clamp(0.0, 1.0),
                    y: (color.y * shade).clamp(0.0, 1.0),
                    z: (color.z * shade).clamp(0.0, 1.0),
                };
            }
            gl::Color4f(color.x, color.y, color.z, part.alpha * vertex.alpha);
            gl::Normal3f(vertex.normal.x, vertex.normal.y, vertex.normal.z);
            gl::TexCoord2f(vertex.uv.u, vertex.uv.v);
            gl::Vertex3f(vertex.pos.x, vertex.pos.y, vertex.pos.z);
        }
        gl::End();
        // Keep the authored lens texture visible and brighten the geometry it
        // sits on. Re-sampling the texture additively clips its detail to white;
        // an untextured face-level lift reads as an illuminated lens while
        // preserving the pattern from the single base pass above.
        if part.emissive {
            gl::Disable(gl::LIGHTING);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE);
            gl::Disable(gl::ALPHA_TEST);
            gl::DepthMask(gl::FALSE);
            gl::Disable(gl::TEXTURE_2D);
            gl::Begin(gl::TRIANGLES);
            for vertex in &part.cpu_vertices {
                gl::Color4f(1.0, 1.0, 1.0, 0.52 * part.alpha * vertex.alpha);
                gl::Vertex3f(vertex.pos.x, vertex.pos.y, vertex.pos.z);
            }
            gl::End();
        }
        gl::BindTexture(gl::TEXTURE_2D, 0);
        gl::Disable(gl::BLEND);
        gl::Enable(gl::ALPHA_TEST);
        gl::DepthMask(gl::TRUE);
    }
}

pub(crate) fn draw_vehicle_reflection_pass(part: &RenderPart, world: Mat4, camera_pos: Vec3) {
    // Two bright "sky" lights sitting above the vehicle. The reflection is
    // computed per vertex from the true view direction, so these glints slide
    // across the bodywork as the camera orbits, like a real environment map.
    let key_light = Vec3::new(0.25, -0.35, 1.0).normalize_or_zero();
    let fill_light = Vec3::new(-0.45, 0.55, 0.85).normalize_or_zero();
    unsafe {
        gl::Enable(gl::DEPTH_TEST);
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::ALPHA_TEST);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE);
        gl::DepthMask(gl::FALSE);
        gl::Begin(gl::TRIANGLES);
        for vertex in &part.cpu_vertices {
            // Move the surface into world space so the reflection is anchored to
            // the scene (bright sky above), not to the model's local axes.
            let normal = world
                .transform_vector3(to_mq(vertex.normal))
                .normalize_or_zero();
            let world_pos = world.transform_point3(to_mq(vertex.pos));
            let view = (camera_pos - world_pos).normalize_or_zero();
            // Reflect the eye ray about the surface normal.
            let reflect = (normal * (2.0 * normal.dot(view)) - view).normalize_or_zero();
            // Ambient environment: reflections pointing up catch the bright sky.
            let sky = reflect.z.max(0.0).powf(1.6);
            // Fresnel: edges facing away from the camera reflect more strongly.
            let fresnel = (1.0 - normal.dot(view).clamp(0.0, 1.0)).powf(3.0);
            // Sharp specular glints where the reflection lines up with a top light.
            let key = reflect.dot(key_light).max(0.0).powf(48.0);
            let fill = reflect.dot(fill_light).max(0.0).powf(80.0);
            let spec = (key + fill).clamp(0.0, 1.0);
            let env = (sky * 0.22 + fresnel * 0.12).clamp(0.0, 0.5);
            let strength = (env + spec * 0.75).clamp(0.0, 0.9);
            // Cool blue tint for the broad sky reflection, near-white for hot glints.
            let color = Vec3::new(0.72, 0.82, 1.0).lerp(Vec3::splat(1.0), spec);
            gl::Color4f(
                color.x,
                color.y,
                color.z,
                strength * part.alpha * vertex.alpha,
            );
            gl::Vertex3f(vertex.pos.x, vertex.pos.y, vertex.pos.z);
        }
        gl::End();
        gl::DepthMask(gl::TRUE);
        gl::Disable(gl::BLEND);
        gl::Enable(gl::ALPHA_TEST);
    }
}

fn vehicle_photo_reflective_part(part: &RenderPart) -> bool {
    if part.emissive
        || part.vehicle_material_role == Some(VehicleMaterialRole::Light)
        || part.transparency == TransparencyMode::Blend
        || vehicle_part_kind(part) != "body"
    {
        return false;
    }
    if matches!(
        part.vehicle_material_role,
        Some(VehicleMaterialRole::BodyA) | Some(VehicleMaterialRole::BodyB)
    ) {
        return true;
    }
    let texture = lower(part.texture_name.trim());
    ![
        "light",
        "lamp",
        "glass",
        "window",
        "windscreen",
        "windshield",
        "wheel",
        "tyre",
        "tire",
        "rubber",
        "interior",
        "seat",
        "dash",
        "dial",
        "speedo",
        "plate",
        "number",
        "decal",
        "sticker",
        "logo",
        "shadow",
        "corona",
        "beam",
        "flare",
        "glow",
        "halo",
    ]
    .iter()
    .any(|marker| texture.contains(marker))
}

fn create_vehicle_photo_reflection_program() -> Result<u32, String> {
    const VERTEX_SOURCE: &str = r#"#version 120
uniform mat4 model_to_world;
varying vec3 world_position;
varying vec3 world_normal;
varying vec2 texture_uv;
varying float vertex_alpha;

void main() {
    vec4 world = model_to_world * gl_Vertex;
    world_position = world.xyz;
    world_normal = normalize(mat3(model_to_world) * gl_Normal);
    texture_uv = gl_MultiTexCoord0.xy;
    vertex_alpha = gl_Color.a;
    gl_Position = gl_ModelViewProjectionMatrix * gl_Vertex;
}
"#;
    const FRAGMENT_SOURCE: &str = r#"#version 120
uniform sampler2D diffuse_texture;
uniform int has_texture;
uniform vec3 camera_position;
uniform vec3 material_color;
uniform float reflection_strength;
varying vec3 world_position;
varying vec3 world_normal;
varying vec2 texture_uv;
varying float vertex_alpha;

void main() {
    vec4 texel = has_texture != 0 ? texture2D(diffuse_texture, texture_uv)
                                  : vec4(1.0);
    float alpha = texel.a * vertex_alpha;
    if (alpha <= 0.02) {
        discard;
    }

    vec3 normal = normalize(world_normal);
    vec3 view_direction = normalize(camera_position - world_position);
    vec3 reflection_direction = normalize(reflect(-view_direction, normal));
    float view_dot = clamp(dot(normal, view_direction), 0.0, 1.0);
    float fresnel = pow(1.0 - view_dot, 3.0);

    // Procedural studio environment: a cool upper sweep, a warm horizon, and
    // two soft-box highlights. This adapts Gen_Shader's bright-pass,
    // clear-coat, and Fresnel ideas without depending on MTA's screen source.
    float sky_mix = smoothstep(-0.28, 0.82, reflection_direction.z);
    vec3 environment = mix(vec3(0.055, 0.040, 0.052),
                           vec3(0.30, 0.48, 0.78), sky_mix);
    float horizon = pow(max(0.0, 1.0 - abs(reflection_direction.z + 0.02)), 12.0);
    environment += vec3(0.42, 0.22, 0.12) * horizon * 0.32;

    vec3 key_direction = normalize(vec3(-0.42, -0.24, 0.88));
    vec3 fill_direction = normalize(vec3(0.58, 0.34, 0.74));
    float key = pow(max(dot(reflection_direction, key_direction), 0.0), 34.0);
    float fill = pow(max(dot(reflection_direction, fill_direction), 0.0), 52.0);
    float softbox = pow(max(0.0, 1.0 - abs(reflection_direction.x + 0.32)), 18.0)
                  * smoothstep(-0.1, 0.8, reflection_direction.z);

    vec3 paint = clamp(texel.rgb * material_color, 0.0, 1.0);
    float paint_luminance = dot(paint, vec3(0.299, 0.587, 0.114));
    float dark_paint_floor = mix(0.58, 1.0, smoothstep(0.06, 0.62, paint_luminance));
    float coat = (0.12 + fresnel * 0.48) * dark_paint_floor;
    vec3 reflection = environment * coat;
    reflection += vec3(1.0, 0.96, 0.90) * key * 0.72;
    reflection += vec3(0.72, 0.84, 1.0) * fill * 0.42;
    reflection += vec3(0.82, 0.90, 1.0) * softbox * 0.10;
    gl_FragColor = vec4(reflection * reflection_strength, alpha);
}
"#;

    let vertex = compile_shader(gl::VERTEX_SHADER, VERTEX_SOURCE)?;
    let fragment = match compile_shader(gl::FRAGMENT_SHADER, FRAGMENT_SOURCE) {
        Ok(shader) => shader,
        Err(err) => {
            unsafe { gl::DeleteShader(vertex) };
            return Err(err);
        }
    };
    unsafe {
        let program = gl::CreateProgram();
        if program == 0 {
            gl::DeleteShader(vertex);
            gl::DeleteShader(fragment);
            return Err("glCreateProgram returned 0".to_string());
        }
        gl::AttachShader(program, vertex);
        gl::AttachShader(program, fragment);
        gl::LinkProgram(program);
        gl::DeleteShader(vertex);
        gl::DeleteShader(fragment);
        let mut ok = 0;
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut ok);
        if ok == 0 {
            let log = program_info_log(program);
            gl::DeleteProgram(program);
            return Err(log);
        }
        Ok(program)
    }
}

fn ensure_vehicle_photo_reflection_program(browser: &mut VehicleBrowserState) -> u32 {
    if browser.photo_reflection_program != 0 || browser.photo_reflection_failed {
        return browser.photo_reflection_program;
    }
    match create_vehicle_photo_reflection_program() {
        Ok(program) => {
            browser.photo_reflection_program = program;
            program
        }
        Err(err) => {
            browser.photo_reflection_failed = true;
            eprintln!("Vehicle photo reflection unavailable: {err}");
            0
        }
    }
}

fn draw_vehicle_photo_reflection_pass(
    part: &RenderPart,
    world: Mat4,
    camera_pos: Vec3,
    program: u32,
    show_textures: bool,
) {
    if program == 0 || !vehicle_photo_reflective_part(part) {
        return;
    }
    let uniform = |name: &str| {
        let name = CString::new(name).unwrap();
        unsafe { gl::GetUniformLocation(program, name.as_ptr()) }
    };
    unsafe {
        gl::PushAttrib(gl::ALL_ATTRIB_BITS);
        gl::UseProgram(program);
        gl::UniformMatrix4fv(
            uniform("model_to_world"),
            1,
            gl::FALSE,
            world.to_cols_array().as_ptr(),
        );
        gl::Uniform3f(
            uniform("camera_position"),
            camera_pos.x,
            camera_pos.y,
            camera_pos.z,
        );
        gl::Uniform3f(
            uniform("material_color"),
            part.material_color.x,
            part.material_color.y,
            part.material_color.z,
        );
        gl::Uniform1f(uniform("reflection_strength"), 1.0);
        gl::Uniform1i(uniform("diffuse_texture"), 0);
        let has_texture = show_textures && part.texture != 0;
        gl::Uniform1i(uniform("has_texture"), i32::from(has_texture));
        gl::ActiveTexture(gl::TEXTURE0);
        if has_texture {
            gl::Enable(gl::TEXTURE_2D);
            gl::BindTexture(gl::TEXTURE_2D, part.texture);
        } else {
            gl::Disable(gl::TEXTURE_2D);
        }
        gl::Enable(gl::DEPTH_TEST);
        gl::DepthFunc(gl::LEQUAL);
        gl::DepthMask(gl::FALSE);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::ALPHA_TEST);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE);
        gl::Enable(gl::POLYGON_OFFSET_FILL);
        gl::PolygonOffset(-1.0, -1.0);
        gl::Begin(gl::TRIANGLES);
        for vertex in &part.cpu_vertices {
            gl::Color4f(1.0, 1.0, 1.0, part.alpha * vertex.alpha);
            gl::Normal3f(vertex.normal.x, vertex.normal.y, vertex.normal.z);
            gl::TexCoord2f(vertex.uv.u, vertex.uv.v);
            gl::Vertex3f(vertex.pos.x, vertex.pos.y, vertex.pos.z);
        }
        gl::End();
        gl::UseProgram(0);
        gl::BindTexture(gl::TEXTURE_2D, 0);
        gl::PopAttrib();
    }
}

pub(crate) fn draw_vehicle_render_part_highlight(part: &RenderPart, selected_part: bool) {
    let color = if selected_part {
        V3 {
            x: 1.0,
            y: 0.95,
            z: 0.20,
        }
    } else {
        V3 {
            x: 0.22,
            y: 0.68,
            z: 1.0,
        }
    };
    unsafe {
        gl::Disable(gl::DEPTH_TEST);
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::ALPHA_TEST);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::LineWidth(if selected_part { 3.0 } else { 2.0 });
        gl::Color4f(color.x, color.y, color.z, 0.86);
        gl::Begin(gl::LINES);
        for tri in part.cpu_vertices.chunks_exact(3) {
            let a = tri[0].pos;
            let b = tri[1].pos;
            let c = tri[2].pos;
            gl::Vertex3f(a.x, a.y, a.z);
            gl::Vertex3f(b.x, b.y, b.z);
            gl::Vertex3f(b.x, b.y, b.z);
            gl::Vertex3f(c.x, c.y, c.z);
            gl::Vertex3f(c.x, c.y, c.z);
            gl::Vertex3f(a.x, a.y, a.z);
        }
        gl::End();
        gl::LineWidth(1.0);
        gl::Disable(gl::BLEND);
        gl::Enable(gl::ALPHA_TEST);
        gl::Enable(gl::DEPTH_TEST);
    }
}

pub(crate) fn draw_vehicle_texture_match_highlight(part: &RenderPart) {
    unsafe {
        gl::Disable(gl::DEPTH_TEST);
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::ALPHA_TEST);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::LineWidth(1.0);
        gl::Color4f(1.0, 0.92, 0.24, 0.28);
        gl::Begin(gl::LINES);
        for tri in part.cpu_vertices.chunks_exact(3) {
            let a = tri[0].pos;
            let b = tri[1].pos;
            let c = tri[2].pos;
            gl::Vertex3f(a.x, a.y, a.z);
            gl::Vertex3f(b.x, b.y, b.z);
            gl::Vertex3f(b.x, b.y, b.z);
            gl::Vertex3f(c.x, c.y, c.z);
            gl::Vertex3f(c.x, c.y, c.z);
            gl::Vertex3f(a.x, a.y, a.z);
        }
        gl::End();
        gl::LineWidth(1.0);
        gl::Disable(gl::BLEND);
        gl::Enable(gl::ALPHA_TEST);
        gl::Enable(gl::DEPTH_TEST);
    }
}

pub(crate) fn focus_selected_vehicle(app: &mut AppState) {
    let Some(mesh) = app.vehicle_browser.preview_mesh.as_ref() else {
        return;
    };
    let Some(vehicle) = selected_vehicle(app) else {
        return;
    };
    let motion = vehicle_preview_motion(vehicle, get_time() as f32);
    let placement = vehicle_preview_placement(vehicle);
    let model = vehicle_preview_model_matrix(&placement, mesh, motion);
    let center = model.transform_point3((mesh.bounds.min + mesh.bounds.max) * 0.5);
    let size = (mesh.bounds.max - mesh.bounds.min).length().max(8.0);
    app.camera_focus = Some(center);
    app.camera.pos = center + vec3(-size * 1.45, -size * 1.75, size * 0.75);
    // Derive the camera angles from the actual showcase position. The previous
    // approximate 45°/-20° angles did not point exactly at `camera_focus`, so
    // the first orbit visibly swung around an offset pivot.
    let toward_center = (center - app.camera.pos).normalize_or_zero();
    app.camera.yaw = toward_center.x.atan2(toward_center.y);
    app.camera.pitch = toward_center
        .z
        .atan2(Vec2::new(toward_center.x, toward_center.y).length());
}

pub(crate) fn set_vehicle_photo_mode(app: &mut AppState, enabled: bool) {
    if app.vehicle_browser.photo_mode == enabled {
        return;
    }
    if enabled {
        ensure_vehicle_preview(app);
        if app.vehicle_browser.preview_mesh.is_none() {
            app.status_message = "Select a vehicle before entering Photo Mode".to_string();
            return;
        }
        app.vehicle_browser.photo_mode_camera = Some(app.camera);
        app.vehicle_browser.photo_mode_camera_mode = Some(app.camera_mode);
        app.vehicle_browser.photo_mode_focus = Some(app.camera_focus);
        app.vehicle_browser.photo_mode = true;
        app.vehicle_browser.show_body = true;
        app.vehicle_browser.show_textures = true;
        app.vehicle_browser.body_color_picker = None;
        app.vehicle_browser.selected_component = None;
        app.vehicle_browser.selected_part = None;
        app.vehicle_browser.hovered_gizmo = None;
        app.vehicle_browser.gizmo_drag = None;
        app.camera_mode = CameraMode::Freeroam;
        focus_selected_vehicle(app);
        app.status_message =
            "Photo Mode enabled — free look with RMB, move with WASD/QE, Esc to exit".to_string();
    } else {
        app.vehicle_browser.photo_mode = false;
        if let Some(camera) = app.vehicle_browser.photo_mode_camera.take() {
            app.camera = camera;
        }
        if let Some(mode) = app.vehicle_browser.photo_mode_camera_mode.take() {
            app.camera_mode = mode;
        }
        if let Some(focus) = app.vehicle_browser.photo_mode_focus.take() {
            app.camera_focus = focus;
        }
        app.camera.looking = false;
        app.camera.last_mouse = mouse_position().into();
        set_cursor_grab(false);
        show_mouse(true);
        app.status_message = "Photo Mode closed".to_string();
    }
}

pub(crate) fn open_selected_vehicle_collision_editor(app: &mut AppState) {
    let target = selected_vehicle(app)
        .map(|vehicle| vehicle.dff.clone())
        .unwrap_or_else(|| "the selected vehicle".to_string());
    if request_discard_editing_context(app, ConfirmAction::OpenSelectedVehicleCollision, &target) {
        return;
    }
    open_selected_vehicle_collision_editor_unchecked(app);
}

pub(crate) fn open_selected_vehicle_collision_editor_unchecked(app: &mut AppState) {
    let Some(vehicle) = selected_vehicle(app).cloned() else {
        app.status_message = "Select a vehicle before opening its collision editor".to_string();
        return;
    };
    if vehicle.readonly {
        app.confirm_dialog = Some(ConfirmDialog {
            action: ConfirmAction::DismissWarning,
            title: "Default Vehicle Is Read-Only".to_string(),
            body: format!("{} must be duplicated before it can be edited.", vehicle.id),
            detail:
                "Use “Duplicate / Load” to create a custom vehicle, then edit the embedded collision on that custom copy."
                    .to_string(),
            primary_label: "OK".to_string(),
            secondary_label: None,
            secondary_action: None,
        });
        app.status_message =
            "Default vehicles must be duplicated as custom before editing".to_string();
        return;
    }
    let Some(dff_bytes) = vehicle_read_dff_bytes(app, &vehicle) else {
        app.status_message = format!("Could not resolve DFF {}", vehicle.dff);
        return;
    };
    let Some((mut mesh, collision_bytes)) =
        parse_embedded_vehicle_collision(&dff_bytes, &vehicle.dff)
    else {
        app.status_message = format!(
            "Vehicle DFF {} has no embedded collision model",
            vehicle.dff
        );
        return;
    };

    if let Some(entry) = find_dff_entry_for_app(app, &vehicle.dff) {
        open_img_entry_in_editing_unchecked(app, entry);
    } else if let Some(path) = vehicle.loose_dff_path.clone() {
        remember_world_camera_before_editing(app);
        open_editing_file_unchecked(app, path);
        app.editing.camera = Some(app.camera);
        app.active_tab = AppTab::Editing;
    } else {
        app.status_message = format!("Could not resolve DFF {}", vehicle.dff);
        return;
    }

    let dff_overlay = match app.editing.asset.take() {
        Some(EditingAsset::Dff(dff))
            if asset_key(&dff.name, ".dff") == asset_key(&vehicle.dff, ".dff") =>
        {
            dff.preview_mesh
        }
        other => {
            app.editing.asset = other;
            app.status_message = format!(
                "Could not open vehicle DFF {} for collision editing",
                vehicle.dff
            );
            return;
        }
    };
    let collision_bounds = mesh.bounds;
    let collision_name = with_ext(&vehicle.dff, ".dff");
    let Some(source_model) = col_model_ranges(&collision_bytes)
        .into_iter()
        .next()
        .map(|range| range.identity)
    else {
        app.status_message = format!(
            "Could not identify the embedded collision model in {}",
            vehicle.dff
        );
        return;
    };
    let mut capsules = valid_capsules_for_mesh(
        &mut mesh,
        load_collision_capsules(&app.root, &collision_name),
    );
    let mut cuboids = valid_cuboids_for_mesh(
        &mut mesh,
        load_collision_cuboids(&app.root, &collision_name),
    );
    let _ = sync_generated_primitive_face_ranges(&mesh, &mut capsules, &mut cuboids);
    app.editing.asset = Some(EditingAsset::Col(EditingColState {
        name: collision_name,
        mesh,
        bytes: collision_bytes,
        source_model,
        embedded_vehicle_dff: true,
        embedded_source_dff_bytes: Some(dff_bytes),
        dff_overlay,
        dff_overlay_name: Some(with_ext(&vehicle.dff, ".dff")),
        dff_overlay_visible: true,
        editing_shadow: false,
        selected_face: 0,
        selected_faces: BTreeSet::new(),
        selected_edges: BTreeSet::new(),
        select_mode: EditingSelectMode::Vertex,
        face_scroll: 0.0,
        primitive_scroll: 0.0,
        selected_vertex: 0,
        selected_primitive: None,
        capsules,
        cuboids,
        box_pick_enabled: true,
        selected_vertices: BTreeSet::new(),
        hovered_face: None,
        hovered_vertex: None,
        dirty: false,
        panel_scroll: 0.0,
        panel_collapsed: col_default_collapsed(),
    }));
    start_editing_col_cuboid_audit(app);
    frame_editing_camera(app, collision_bounds);
    app.status_message = format!(
        "Editing embedded collision for custom vehicle {}",
        vehicle.id
    );
}

pub(crate) fn export_selected_vehicle_dff(app: &mut AppState) {
    if app.vehicle_browser.texture_export_rx.is_some() {
        app.status_message = "Wait for the current vehicle texture export to finish".to_string();
        return;
    }
    let Some(vehicle) = selected_vehicle(app).cloned() else {
        app.status_message = "Select a vehicle before exporting DFF".to_string();
        return;
    };
    if let Some(path) = &vehicle.loose_dff_path {
        let Some(name) = path
            .file_name()
            .and_then(|value| value.to_str())
            .map(str::to_string)
        else {
            app.status_message = "Loose DFF path has no filename".to_string();
            return;
        };
        open_export_loose_file_dialog(app, path.clone(), name, &vehicle.txd, "DFF");
    } else {
        open_export_named_dff_dialog(app, &vehicle.dff, &vehicle.txd);
    }
}

/// Export the currently previewed part texture as a PNG so it can be used in
/// an external modeling / image editing program.
pub(crate) fn export_selected_vehicle_texture(app: &mut AppState) {
    let Some(vehicle) = selected_vehicle(app).cloned() else {
        app.status_message = "Select a vehicle first".to_string();
        return;
    };
    let Some(texture_name) = app.vehicle_browser.selected_part.and_then(|idx| {
        app.vehicle_browser
            .preview_mesh
            .as_ref()
            .and_then(|mesh| mesh.parts.get(idx))
            .map(|part| part.texture_name.trim().to_string())
    }) else {
        app.status_message = "Select a part to export its texture".to_string();
        return;
    };
    if texture_name.is_empty() {
        app.status_message = "Selected part has no texture".to_string();
        return;
    }
    let Some((width, height, rgba)) = vehicle_texture_rgba(app, &texture_name, &vehicle.txd) else {
        app.status_message = format!("Could not decode texture {texture_name}");
        return;
    };
    drain_text_input();
    if app.dff_picker_rx.is_some() {
        app.status_message = "File browser is already open".to_string();
        return;
    }
    let default_path = load_last_dff_export_dir().join(format!("{}.png", lower(&texture_name)));
    let (tx, rx) = mpsc::channel();
    app.dff_picker_rx = Some(rx);
    app.status_message = format!("Opening texture export browser for {texture_name}...");
    thread::spawn(move || {
        let _ = tx.send((
            DffPickerKind::ExportTexturePng {
                texture_name,
                width,
                height,
                rgba,
            },
            choose_export_png_path(default_path),
        ));
    });
}

pub(crate) fn copy_selected_vehicle_texture_name(app: &mut AppState) {
    let Some(texture_name) = app.vehicle_browser.selected_part.and_then(|idx| {
        app.vehicle_browser
            .preview_mesh
            .as_ref()
            .and_then(|mesh| mesh.parts.get(idx))
            .map(|part| part.texture_name.trim().to_string())
    }) else {
        app.status_message = "Select a part to copy its texture name".to_string();
        return;
    };
    if texture_name.is_empty() {
        app.status_message = "Selected part has no texture name".to_string();
        return;
    }
    match set_system_clipboard(&texture_name) {
        Ok(()) => app.status_message = format!("Copied texture name {texture_name}"),
        Err(err) => app.status_message = format!("Could not copy texture name: {err}"),
    }
}

pub(crate) fn open_selected_vehicle_texture_replace(app: &mut AppState) {
    let Some(vehicle) = selected_vehicle(app).cloned() else {
        app.status_message = "Select a custom vehicle first".to_string();
        return;
    };
    if vehicle.readonly {
        app.status_message =
            "Default vehicles must be duplicated before editing materials".to_string();
        return;
    }
    if app.vehicle_browser.texture_replace_rx.is_some() {
        app.status_message = "A vehicle material replacement is already running".to_string();
        return;
    }
    if app.vehicle_browser.texture_export_rx.is_some() {
        app.status_message = "Wait for the current vehicle texture export to finish".to_string();
        return;
    }
    let Some((material_index, texture_name)) = app
        .vehicle_browser
        .selected_part
        .and_then(|idx| {
            app.vehicle_browser
                .preview_mesh
                .as_ref()
                .and_then(|mesh| mesh.parts.get(idx))
        })
        .map(|part| (part.material_index, part.texture_name.trim().to_string()))
    else {
        app.status_message = "Select a material in the Components list first".to_string();
        return;
    };
    if texture_name.is_empty() {
        app.status_message = format!("Material {material_index} has no texture to replace");
        return;
    }
    let Some(dff_path) = vehicle.loose_dff_path.clone() else {
        app.status_message =
            "This custom vehicle is not backed by an editable loose DFF".to_string();
        return;
    };
    let Some(txd_path) = find_loose_vehicle_txd(&vehicle) else {
        app.status_message = format!("Could not find custom vehicle TXD {}", vehicle.txd);
        return;
    };
    drain_text_input();
    if app.dff_picker_rx.is_some() {
        app.status_message = "File browser is already open".to_string();
        return;
    }
    let start_dir = txd_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| app.root.join("txd_build"));
    let kind = DffPickerKind::VehicleTextureReplace(VehicleTextureReplaceRequest {
        vehicle_identity: vehicle_preview_identity(&vehicle),
        vehicle_id: vehicle.id.clone(),
        dff_path,
        txd_path,
        txd_name: vehicle.txd.clone(),
        texture_name: texture_name.clone(),
    });
    let (tx, rx) = mpsc::channel();
    app.dff_picker_rx = Some(rx);
    app.status_message = format!("Choose a PNG to replace material {texture_name}...");
    thread::spawn(move || {
        let _ = tx.send((kind, choose_texture_image_path(start_dir)));
    });
}

fn write_custom_vehicle_asset(
    asset_path: &Path,
    updated: &[u8],
    label: &str,
) -> Result<Option<PathBuf>, String> {
    let extension = asset_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("asset");
    let temp_path = asset_path.with_extension(format!("{extension}.eagle-tmp"));
    let backup_path = asset_path.with_extension(format!("{extension}.eagle-backup"));
    let previous_path = asset_path.with_extension(format!("{extension}.eagle-previous"));
    fs::write(&temp_path, updated)
        .map_err(|err| format!("Could not stage {}: {err}", asset_path.display()))?;
    let staged = fs::read(&temp_path).map_err(|err| {
        let _ = fs::remove_file(&temp_path);
        format!("Could not verify staged {label}: {err}")
    })?;
    if staged != updated {
        let _ = fs::remove_file(&temp_path);
        return Err(format!("Staged {label} verification failed"));
    }
    let backup = if !backup_path.exists() {
        if let Err(err) = fs::copy(asset_path, &backup_path) {
            let _ = fs::remove_file(&temp_path);
            return Err(format!("Could not back up {}: {err}", asset_path.display()));
        }
        Some(backup_path)
    } else {
        None
    };
    if previous_path.exists() {
        fs::remove_file(&previous_path).map_err(|err| {
            let _ = fs::remove_file(&temp_path);
            format!(
                "Could not clear stale TXD swap file {}: {err}",
                previous_path.display()
            )
        })?;
    }
    fs::rename(asset_path, &previous_path).map_err(|err| {
        let _ = fs::remove_file(&temp_path);
        format!(
            "Could not prepare {} for replacement: {err}",
            asset_path.display()
        )
    })?;
    if let Err(err) = fs::rename(&temp_path, asset_path) {
        let restore = fs::rename(&previous_path, asset_path);
        let _ = fs::remove_file(&temp_path);
        return Err(match restore {
            Ok(()) => format!("Could not replace {}: {err}", asset_path.display()),
            Err(restore_err) => format!(
                "Could not replace {}: {err}; original remains at {} because restore failed: {restore_err}",
                asset_path.display(),
                previous_path.display()
            ),
        });
    }
    let _ = fs::remove_file(previous_path);
    Ok(backup)
}

fn write_custom_vehicle_txd(txd_path: &Path, updated: &[u8]) -> Result<Option<PathBuf>, String> {
    write_custom_vehicle_asset(txd_path, updated, "TXD")
}

fn replace_vehicle_texture_worker(
    request: VehicleTextureReplaceRequest,
    image_path: PathBuf,
) -> VehicleTextureReplaceResult {
    let result = (|| -> Result<(TxdTextureIndex, RawMesh, Option<PathBuf>), String> {
        let native = imported_image_texture_native(&image_path, &request.texture_name)
            .map_err(|err| format!("Could not import replacement image: {err}"))?;
        let txd_bytes = fs::read(&request.txd_path)
            .map_err(|err| format!("Could not read {}: {err}", request.txd_path.display()))?;
        let updated =
            replace_or_append_texture_native_in_txd(txd_bytes, &native, &request.texture_name)?;

        let txd_key = asset_key(&request.txd_name, ".txd");
        let mut txd_index = TxdTextureIndex::new();
        index_one_txd(
            &updated,
            0,
            updated.len(),
            &request.txd_path,
            &txd_key,
            &mut txd_index,
        );
        let indexed = txd_index
            .get(&lower(&request.texture_name))
            .is_some_and(|entries| {
                entries
                    .iter()
                    .any(|entry| entry.txd_name.eq_ignore_ascii_case(&txd_key))
            });
        if !indexed {
            return Err(format!(
                "Updated TXD did not contain replacement texture {}",
                request.texture_name
            ));
        }

        let dff_bytes = fs::read(&request.dff_path)
            .map_err(|err| format!("Could not read {}: {err}", request.dff_path.display()))?;
        let raw = parse_dff_mesh(&dff_bytes);
        if raw.vertices.is_empty() || raw.triangles.is_empty() {
            return Err(format!(
                "Vehicle DFF {} did not parse",
                request.dff_path.display()
            ));
        }
        let backup = write_custom_vehicle_txd(&request.txd_path, &updated)?;
        Ok((txd_index, raw, backup))
    })();
    VehicleTextureReplaceResult {
        vehicle_identity: request.vehicle_identity,
        vehicle_id: request.vehicle_id,
        txd_name: request.txd_name,
        texture_name: request.texture_name,
        result,
    }
}

pub(crate) fn start_vehicle_texture_replace(
    app: &mut AppState,
    request: VehicleTextureReplaceRequest,
    image_path: PathBuf,
) {
    if app.vehicle_browser.texture_replace_rx.is_some() {
        app.status_message = "A vehicle material replacement is already running".to_string();
        return;
    }
    let (tx, rx) = mpsc::channel();
    app.vehicle_browser.texture_replace_rx = Some(rx);
    app.status_message = format!("Replacing vehicle material {}...", request.texture_name);
    thread::spawn(move || {
        let result = replace_vehicle_texture_worker(request, image_path);
        let _ = tx.send(result);
    });
}

pub(crate) fn poll_vehicle_texture_replace(app: &mut AppState) {
    let Some(rx) = app.vehicle_browser.texture_replace_rx.as_ref() else {
        return;
    };
    let result = match rx.try_recv() {
        Ok(result) => result,
        Err(mpsc::TryRecvError::Empty) => return,
        Err(mpsc::TryRecvError::Disconnected) => {
            app.vehicle_browser.texture_replace_rx = None;
            app.status_message = "Vehicle material replacement stopped unexpectedly".to_string();
            return;
        }
    };
    app.vehicle_browser.texture_replace_rx = None;
    let (mut txd_index, raw, backup) = match result.result {
        Ok(output) => output,
        Err(err) => {
            app.status_message = format!("Could not replace vehicle material: {err}");
            return;
        }
    };

    remove_txd_from_texture_index(app, &result.txd_name);
    for (texture_name, mut entries) in txd_index.drain() {
        app.txd_textures
            .entry(texture_name)
            .or_default()
            .append(&mut entries);
    }
    invalidate_cached_txd_textures(app, &result.txd_name, None);
    app.vehicle_browser.texture_previews.clear();

    let selected = selected_vehicle(app).cloned();
    if selected
        .as_ref()
        .is_some_and(|vehicle| vehicle_preview_identity(vehicle) == result.vehicle_identity)
    {
        let vehicle = selected.unwrap();
        if !install_vehicle_preview(app, &vehicle, raw) {
            app.vehicle_browser.preview_key.clear();
        }
    }
    let backup_note = backup
        .as_ref()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .map(|name| format!("; original backed up as {name}"))
        .unwrap_or_default();
    app.status_message = format!(
        "Replaced material {} for custom vehicle {}{}",
        result.texture_name, result.vehicle_id, backup_note
    );
}

fn referenced_sa_generic_textures(
    referenced_names: &[String],
    vehicle_texture_names: &HashSet<String>,
    sa_generic_texture_names: &HashSet<String>,
) -> HashSet<String> {
    referenced_names
        .iter()
        .map(|name| lower(name.trim()))
        .filter(|name| {
            !name.is_empty()
                && !vehicle_texture_names.contains(name)
                && sa_generic_texture_names.contains(name)
        })
        .collect()
}

fn export_vehicle_texture_worker(
    dff_path: PathBuf,
    dir: PathBuf,
    vehicle_textures: Vec<(String, TxdTexture)>,
    sa_generic_textures: Vec<(String, TxdTexture)>,
    base_message: String,
    tx: mpsc::Sender<VehicleTextureExportUpdate>,
) {
    let result = (|| -> Result<(usize, usize, usize), String> {
        fs::create_dir_all(&dir)
            .map_err(|err| format!("Could not create {}: {err}", dir.display()))?;
        let dff_bytes = fs::read(&dff_path)
            .map_err(|err| format!("Could not inspect {}: {err}", dff_path.display()))?;
        let raw = parse_dff_mesh(&dff_bytes);
        let vehicle_texture_names = vehicle_textures
            .iter()
            .map(|(name, _)| lower(name))
            .collect::<HashSet<_>>();
        let sa_generic_texture_names = sa_generic_textures
            .iter()
            .map(|(name, _)| lower(name))
            .collect::<HashSet<_>>();
        let needed_generic = referenced_sa_generic_textures(
            &raw.material_textures,
            &vehicle_texture_names,
            &sa_generic_texture_names,
        );
        let mut textures = vehicle_textures
            .into_iter()
            .map(|(name, texture)| (name, texture, false))
            .collect::<Vec<_>>();
        textures.extend(
            sa_generic_textures
                .into_iter()
                .filter(|(name, _)| needed_generic.contains(&lower(name)))
                .map(|(name, texture)| (name, texture, true)),
        );
        textures.sort_by(|left, right| lower(&left.0).cmp(&lower(&right.0)));

        let total = textures.len();
        let mut used_names: HashSet<String> = HashSet::new();
        let mut exported = 0usize;
        let mut generic_exported = 0usize;
        let mut failed = 0usize;
        for (index, (texture_name, entry, is_generic)) in textures.into_iter().enumerate() {
            let Some((width, height, rgba)) = decode_txd_texture(&entry) else {
                failed += 1;
                let _ = tx.send(VehicleTextureExportUpdate::Progress {
                    completed: index + 1,
                    total,
                });
                continue;
            };
            let path = unique_export_texture_path(&dir, &texture_name, &mut used_names);
            match image::save_buffer(&path, &rgba, width, height, image::ColorType::Rgba8) {
                Ok(()) => {
                    exported += 1;
                    generic_exported += usize::from(is_generic);
                }
                Err(_) => failed += 1,
            }
            let _ = tx.send(VehicleTextureExportUpdate::Progress {
                completed: index + 1,
                total,
            });
        }
        Ok((exported, generic_exported, failed))
    })();

    let (exported, generic_exported, failed, error) = match result {
        Ok((exported, generic_exported, failed)) => (exported, generic_exported, failed, None),
        Err(error) => (0, 0, 0, Some(error)),
    };
    let _ = tx.send(VehicleTextureExportUpdate::Complete(
        VehicleTextureExportResult {
            base_message,
            folder: dir,
            exported,
            generic_exported,
            failed,
            error,
        },
    ));
}

fn snapshot_vehicle_export_textures(
    app: &AppState,
    txd: &str,
) -> (Vec<(String, TxdTexture)>, Vec<(String, TxdTexture)>) {
    let txd_key = asset_key(txd, ".txd");
    let sa_generic_txd_key = asset_key("vehicle", ".txd");
    let mut vehicle_textures = Vec::new();
    let mut sa_generic_textures = Vec::new();
    for (texture_name, entries) in &app.txd_textures {
        if let Some(entry) = entries
            .iter()
            .find(|entry| entry.txd_name.eq_ignore_ascii_case(&txd_key))
        {
            vehicle_textures.push((texture_name.clone(), entry.clone()));
        }
        if !txd_key.eq_ignore_ascii_case(&sa_generic_txd_key) {
            if let Some(entry) = entries
                .iter()
                .find(|entry| entry.txd_name.eq_ignore_ascii_case(&sa_generic_txd_key))
            {
                sa_generic_textures.push((texture_name.clone(), entry.clone()));
            }
        }
    }
    (vehicle_textures, sa_generic_textures)
}

pub(crate) fn poll_vehicle_texture_export(app: &mut AppState) {
    loop {
        let Some(rx) = app.vehicle_browser.texture_export_rx.as_ref() else {
            return;
        };
        match rx.try_recv() {
            Ok(VehicleTextureExportUpdate::Progress { completed, total }) => {
                app.status_message = format!("Exporting vehicle textures... {completed}/{total}");
            }
            Ok(VehicleTextureExportUpdate::Complete(result)) => {
                app.vehicle_browser.texture_export_rx = None;
                if let Some(error) = result.error {
                    app.status_message =
                        format!("{} (texture export failed: {error})", result.base_message);
                    return;
                }
                if result.exported == 0 && result.failed == 0 {
                    app.status_message = result.base_message;
                    return;
                }
                let folder = result
                    .folder
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("textures");
                let generic = if result.generic_exported > 0 {
                    format!(", including {} SA generic", result.generic_exported)
                } else {
                    String::new()
                };
                let failed = if result.failed > 0 {
                    format!("; {} failed", result.failed)
                } else {
                    String::new()
                };
                app.status_message = format!(
                    "{} (+{} .PNG texture(s){} in {}{})",
                    result.base_message, result.exported, generic, folder, failed
                );
                return;
            }
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => {
                app.vehicle_browser.texture_export_rx = None;
                app.status_message =
                    "Vehicle texture export worker closed unexpectedly".to_string();
                return;
            }
        }
    }
}

/// After a vehicle DFF is exported, dump its own TXD textures plus any
/// referenced shared San Andreas `vehicle.txd` textures into a sibling folder
/// (`<dff_stem>_textures`) as uppercase `.PNG` files.
pub(crate) fn export_vehicle_textures_beside_dff(
    app: &mut AppState,
    dff_path: &Path,
    vehicle_txd: &str,
) {
    if app.vehicle_browser.texture_export_rx.is_some() {
        app.status_message = format!(
            "{} (another vehicle texture export is still running)",
            app.status_message
        );
        return;
    }
    if vehicle_txd.trim().is_empty() {
        return;
    }
    let Some(parent) = dff_path.parent() else {
        return;
    };
    let stem = dff_path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("vehicle");
    let dir = parent.join(format!("{stem}_textures"));
    let (vehicle_textures, sa_generic_textures) =
        snapshot_vehicle_export_textures(app, vehicle_txd);
    let base_message = app.status_message.clone();
    let dff_path = dff_path.to_path_buf();
    let (tx, rx) = mpsc::channel();
    app.vehicle_browser.texture_export_rx = Some(rx);
    app.status_message = format!("{base_message} (exporting companion textures...)");
    thread::spawn(move || {
        export_vehicle_texture_worker(
            dff_path,
            dir,
            vehicle_textures,
            sa_generic_textures,
            base_message,
            tx,
        );
    });
}

pub(crate) fn open_export_loose_file_dialog(
    app: &mut AppState,
    source_path: PathBuf,
    asset_name: String,
    vehicle_txd: &str,
    label: &str,
) {
    let vehicle_txd = vehicle_txd.to_string();
    drain_text_input();
    if app.dff_picker_rx.is_some() {
        app.status_message = "File browser is already open".to_string();
        return;
    }
    let default_path = load_last_dff_export_dir().join(&asset_name);
    let (tx, rx) = mpsc::channel();
    app.dff_picker_rx = Some(rx);
    app.status_message = format!("Opening {label} export browser...");
    thread::spawn(move || {
        let _ = tx.send((
            DffPickerKind::ExportLooseFile {
                source_path,
                asset_name,
                vehicle_txd,
            },
            choose_export_dff_path(default_path),
        ));
    });
}

pub(crate) fn update_vehicle_text_input(app: &mut AppState) {
    if !app.vehicle_browser.search_active {
        return;
    }
    app.vehicle_browser.cursor =
        clamp_char_boundary(&app.vehicle_browser.search, app.vehicle_browser.cursor);
    if handle_text_clipboard_shortcuts(
        &mut app.vehicle_browser.search,
        &mut app.vehicle_browser.cursor,
        &mut app.vehicle_browser.selection_anchor,
    ) {
        drain_text_input();
        app.vehicle_browser.scroll = 0.0;
        return;
    }
    if is_key_pressed(KeyCode::Backspace)
        && !delete_text_selection(
            &mut app.vehicle_browser.search,
            &mut app.vehicle_browser.cursor,
            &mut app.vehicle_browser.selection_anchor,
        )
        && app.vehicle_browser.cursor > 0
    {
        let prev = prev_char_boundary(&app.vehicle_browser.search, app.vehicle_browser.cursor);
        app.vehicle_browser
            .search
            .replace_range(prev..app.vehicle_browser.cursor, "");
        app.vehicle_browser.cursor = prev;
        app.vehicle_browser.scroll = 0.0;
    }
    while let Some(ch) = get_char_pressed() {
        if handle_text_control_char(
            &mut app.vehicle_browser.search,
            &mut app.vehicle_browser.cursor,
            &mut app.vehicle_browser.selection_anchor,
            ch,
        ) {
            continue;
        }
        if !ch.is_control() {
            insert_text_at_cursor(
                &mut app.vehicle_browser.search,
                &mut app.vehicle_browser.cursor,
                &mut app.vehicle_browser.selection_anchor,
                &ch.to_string(),
            );
            app.vehicle_browser.scroll = 0.0;
        }
    }
}

fn apply_vehicle_gizmo_drag(app: &mut AppState, viewport: Rect, mouse: Vec2) {
    let Some(drag) = app.vehicle_browser.gizmo_drag else {
        return;
    };
    let Some(mesh) = app.vehicle_browser.preview_mesh.as_ref() else {
        return;
    };
    let Some(origin) = vehicle_component_pivot(mesh, drag.component) else {
        return;
    };
    let Some(center) = world_to_screen(app, viewport, origin) else {
        return;
    };
    let mut degrees = screen_angle_delta_degrees(center, drag.start_mouse, mouse);
    if drag.axis == GizmoAxis::Z {
        degrees = -degrees;
    }
    if app.snap_enabled {
        degrees = snap_delta(degrees, app.snap_rotate);
    }
    let rotation = app
        .vehicle_browser
        .component_rotations
        .entry(drag.component)
        .or_default();
    *rotation = drag.start_rotation;
    match drag.axis {
        GizmoAxis::X => rotation.x += degrees.to_radians(),
        GizmoAxis::Y => rotation.y += degrees.to_radians(),
        GizmoAxis::Z => rotation.z += degrees.to_radians(),
    }
    app.vehicle_browser.hovered_gizmo = Some(drag.axis);
}

fn update_vehicle_collision_copy_dialog(app: &mut AppState, mouse: Vec2) -> bool {
    if is_key_pressed(KeyCode::Escape) {
        app.vehicle_browser.collision_copy_dialog = None;
        drain_text_input();
        return true;
    }
    let list = vehicle_collision_copy_list_rect();
    let visible_rows = (list.h / VEHICLE_COLLISION_COPY_ROW_H).floor().max(1.0) as usize;
    let indices = app
        .vehicle_browser
        .collision_copy_dialog
        .as_ref()
        .map(|dialog| collision_copy_source_indices(&app.vehicles, dialog))
        .unwrap_or_default();
    if let Some(dialog) = app.vehicle_browser.collision_copy_dialog.as_mut() {
        if !dialog
            .selected_source
            .is_some_and(|selected| indices.contains(&selected))
        {
            dialog.selected_source = indices.first().copied();
        }
        let max_scroll = indices.len().saturating_sub(visible_rows) as f32;
        dialog.scroll = dialog.scroll.min(max_scroll);
    }

    let wheel = mouse_wheel().1;
    if wheel.abs() > 0.0 && list.contains(mouse) {
        if let Some(dialog) = app.vehicle_browser.collision_copy_dialog.as_mut() {
            let max_scroll = indices.len().saturating_sub(visible_rows) as f32;
            dialog.scroll = (dialog.scroll - wheel).clamp(0.0, max_scroll);
        }
        return true;
    }

    let mut submit = is_key_pressed(KeyCode::Enter);
    if is_mouse_button_pressed(MouseButton::Left) {
        if vehicle_collision_copy_cancel_rect().contains(mouse) {
            app.vehicle_browser.collision_copy_dialog = None;
            drain_text_input();
            return true;
        }
        if vehicle_collision_copy_submit_rect().contains(mouse) {
            submit = true;
        } else if list.contains(mouse) {
            let start = app
                .vehicle_browser
                .collision_copy_dialog
                .as_ref()
                .map(|dialog| dialog.scroll.floor() as usize)
                .unwrap_or(0);
            let row = ((mouse.y - list.y) / VEHICLE_COLLISION_COPY_ROW_H)
                .floor()
                .max(0.0) as usize;
            if let Some(source) = indices.get(start + row).copied()
                && let Some(dialog) = app.vehicle_browser.collision_copy_dialog.as_mut()
            {
                dialog.selected_source = Some(source);
            }
            return true;
        }
    }
    if submit {
        start_vehicle_collision_copy(app);
        drain_text_input();
        return true;
    }

    let Some(dialog) = app.vehicle_browser.collision_copy_dialog.as_mut() else {
        return true;
    };
    dialog.cursor = clamp_char_boundary(&dialog.search, dialog.cursor);
    let original = dialog.search.clone();
    if handle_text_clipboard_shortcuts(
        &mut dialog.search,
        &mut dialog.cursor,
        &mut dialog.selection_anchor,
    ) {
        drain_text_input();
    }
    if is_key_pressed(KeyCode::Backspace)
        && !delete_text_selection(
            &mut dialog.search,
            &mut dialog.cursor,
            &mut dialog.selection_anchor,
        )
        && dialog.cursor > 0
    {
        let previous = prev_char_boundary(&dialog.search, dialog.cursor);
        dialog.search.replace_range(previous..dialog.cursor, "");
        dialog.cursor = previous;
    }
    if is_key_pressed(KeyCode::Delete)
        && !delete_text_selection(
            &mut dialog.search,
            &mut dialog.cursor,
            &mut dialog.selection_anchor,
        )
        && dialog.cursor < dialog.search.len()
    {
        let next = next_char_boundary(&dialog.search, dialog.cursor);
        dialog.search.replace_range(dialog.cursor..next, "");
    }
    while let Some(ch) = get_char_pressed() {
        if handle_text_control_char(
            &mut dialog.search,
            &mut dialog.cursor,
            &mut dialog.selection_anchor,
            ch,
        ) {
            continue;
        }
        if !ch.is_control() {
            insert_text_at_cursor(
                &mut dialog.search,
                &mut dialog.cursor,
                &mut dialog.selection_anchor,
                &ch.to_string(),
            );
        }
    }
    if dialog.search != original {
        dialog.scroll = 0.0;
    }
    true
}

pub(crate) fn update_vehicle_browser(app: &mut AppState, mouse: Vec2) -> bool {
    if app.active_tab != AppTab::Vehicles {
        app.vehicle_browser.manage_dictionaries = false;
        app.vehicle_browser.build_dialog = None;
        app.vehicle_browser.collision_copy_dialog = None;
        return false;
    }
    if app.vehicle_browser.collision_copy_dialog.is_some() {
        return update_vehicle_collision_copy_dialog(app, mouse);
    }
    if app.vehicle_browser.build_dialog.is_some() {
        return update_vehicle_build_dialog(app, mouse);
    }
    if app.vehicle_browser.manage_dictionaries {
        return update_vehicle_dictionary_manager(app, mouse);
    }
    let photo_mode = app.vehicle_browser.photo_mode;
    if photo_mode {
        if is_key_pressed(KeyCode::Escape)
            || is_key_pressed(KeyCode::P)
            || (is_mouse_button_pressed(MouseButton::Left)
                && vehicle_photo_mode_rect(app).contains(mouse))
        {
            set_vehicle_photo_mode(app, false);
            return true;
        }
        if !app.vehicle_browser.search_active
            && (is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::Down))
        {
            let indices = filtered_vehicle_indices(app);
            if !indices.is_empty() {
                let current = indices
                    .iter()
                    .position(|index| *index == app.vehicle_browser.selected)
                    .unwrap_or(0);
                let next = if is_key_pressed(KeyCode::Up) {
                    current.checked_sub(1).unwrap_or(indices.len() - 1)
                } else {
                    (current + 1) % indices.len()
                };
                app.vehicle_browser.selected = indices[next];
                app.vehicle_browser.preview_key.clear();
                ensure_vehicle_preview(app);
                let visible_rows =
                    (vehicle_list_rect().h / VEHICLE_ROW_H).floor().max(1.0) as usize;
                if next < app.vehicle_browser.scroll as usize {
                    app.vehicle_browser.scroll = next as f32;
                } else if next >= app.vehicle_browser.scroll as usize + visible_rows {
                    app.vehicle_browser.scroll =
                        next.saturating_add(1).saturating_sub(visible_rows) as f32;
                }
            }
            return true;
        }
    }
    if !photo_mode && !app.vehicle_browser.search_active && is_key_pressed(KeyCode::P) {
        set_vehicle_photo_mode(app, true);
        return true;
    }
    let panel = vehicle_panel_rect();
    let details_panel = vehicle_details_rect();
    let preview = vehicle_preview_viewport_rect(app);
    if !photo_mode
        && is_mouse_button_pressed(MouseButton::Left)
        && vehicle_photo_mode_rect(app).contains(mouse)
    {
        set_vehicle_photo_mode(app, true);
        return true;
    }
    if !photo_mode && app.vehicle_browser.gizmo_drag.is_some() {
        if is_mouse_button_down(MouseButton::Left) {
            apply_vehicle_gizmo_drag(app, preview, mouse);
            return true;
        }
        app.vehicle_browser.gizmo_drag = None;
    }
    if !photo_mode
        && preview.contains(mouse)
        && !panel.contains(mouse)
        && !details_panel.contains(mouse)
    {
        ensure_vehicle_preview(app);
        app.vehicle_browser.hovered_gizmo = vehicle_gizmo_axis_at(app, preview, mouse);
        if is_mouse_button_pressed(MouseButton::Left) {
            if let (Some(component), Some(axis)) = (
                app.vehicle_browser.selected_component,
                app.vehicle_browser.hovered_gizmo,
            ) {
                let start_rotation = app
                    .vehicle_browser
                    .component_rotations
                    .get(&component)
                    .copied()
                    .unwrap_or_default();
                app.vehicle_browser.gizmo_drag = Some(VehicleGizmoDrag {
                    component,
                    axis,
                    start_mouse: mouse,
                    start_rotation,
                });
                return true;
            }
            if let Some(component) = pick_vehicle_component(app, mouse) {
                app.vehicle_browser.selected_component = Some(component);
                app.vehicle_browser.selected_part = None;
                return true;
            }
            app.vehicle_browser.selected_component = None;
            app.vehicle_browser.selected_part = None;
            app.vehicle_browser.hovered_gizmo = None;
            return true;
        }
    } else {
        app.vehicle_browser.hovered_gizmo = None;
    }
    if !panel.contains(mouse) && !details_panel.contains(mouse) {
        if app.vehicle_browser.search_active && is_mouse_button_pressed(MouseButton::Left) {
            app.vehicle_browser.search_active = false;
        }
        update_vehicle_text_input(app);
        return photo_mode;
    }
    if vehicle_search_rect().contains(mouse) {
        if is_mouse_button_pressed(MouseButton::Left) {
            app.vehicle_browser.search_active = true;
            app.vehicle_browser.cursor = app.vehicle_browser.search.len();
        }
        update_vehicle_text_input(app);
        return true;
    }
    if !photo_mode
        && is_mouse_button_down(MouseButton::Left)
        && let Some(slot) = app.vehicle_browser.body_color_picker
    {
        for channel in 0..3 {
            let rect = vehicle_body_color_channel_rect(slot, channel);
            if rect.contains(mouse) {
                let value = ((mouse.x - (rect.x + 18.0)) / (rect.w - 18.0)).clamp(0.0, 1.0);
                set_vehicle_body_color_channel(app, slot, channel, value);
                return true;
            }
        }
    }
    if is_mouse_button_pressed(MouseButton::Left) {
        app.vehicle_browser.search_active = false;
        if vehicle_filter_rect(0).contains(mouse) {
            app.vehicle_browser.show_default = !app.vehicle_browser.show_default;
            return true;
        }
        if vehicle_filter_rect(1).contains(mouse) {
            app.vehicle_browser.show_custom = !app.vehicle_browser.show_custom;
            return true;
        }
        if vehicle_action_rect(0).contains(mouse) {
            open_selected_vehicle_collision_editor(app);
            return true;
        }
        if vehicle_action_rect(1).contains(mouse) {
            export_selected_vehicle_dff(app);
            return true;
        }
        if vehicle_action_rect(2).contains(mouse) {
            reload_selected_vehicle(app);
            return true;
        }
        if vehicle_action_rect(3).contains(mouse) {
            ensure_vehicle_preview(app);
            focus_selected_vehicle(app);
            return true;
        }
        if vehicle_action_rect(4).contains(mouse) {
            open_vehicle_folder_picker(app);
            return true;
        }
        if vehicle_action_rect(5).contains(mouse) {
            app.vehicle_browser.manage_dictionaries = true;
            app.vehicle_browser.dictionary_scroll = 0.0;
            return true;
        }
        if vehicle_action_rect(6).contains(mouse) {
            open_vehicle_loader_picker(app);
            return true;
        }
        if vehicle_action_rect(7).contains(mouse) {
            open_vehicle_build_dialog(app);
            return true;
        }
        if !photo_mode {
            for slot in 0..2 {
                if vehicle_body_color_rect(slot).contains(mouse) {
                    app.vehicle_browser.body_color_picker =
                        if app.vehicle_browser.body_color_picker == Some(slot) {
                            None
                        } else {
                            Some(slot)
                        };
                    return true;
                }
            }
        }
        let list = vehicle_list_rect();
        if list.contains(mouse) {
            let indices = filtered_vehicle_indices(app);
            let row = ((mouse.y - list.y) / VEHICLE_ROW_H).floor().max(0.0) as usize;
            let idx = app.vehicle_browser.scroll as usize + row;
            if let Some(vehicle_idx) = indices.get(idx).copied() {
                app.vehicle_browser.selected = vehicle_idx;
                app.vehicle_browser.preview_key.clear();
                ensure_vehicle_preview(app);
            }
            return true;
        }
    }
    if !photo_mode {
        for slot in 0..8 {
            if vehicle_toggle_rect(slot).contains(mouse)
                && is_mouse_button_pressed(MouseButton::Left)
            {
                match slot {
                    0 => app.vehicle_browser.show_body = !app.vehicle_browser.show_body,
                    1 => app.vehicle_browser.show_textures = !app.vehicle_browser.show_textures,
                    2 => {
                        app.vehicle_browser.show_collision_mesh =
                            !app.vehicle_browser.show_collision_mesh
                    }
                    3 => {
                        app.vehicle_browser.show_collision_volumes =
                            !app.vehicle_browser.show_collision_volumes
                    }
                    4 => {
                        // The mesh recompiles on the next ensure_vehicle_preview
                        // because the lights flag is part of the preview key; leaving
                        // the key intact lets it detect this is the same vehicle and
                        // preserve component rotations.
                        app.vehicle_browser.lights_on = !app.vehicle_browser.lights_on;
                    }
                    5 => app.vehicle_browser.hide_damaged = !app.vehicle_browser.hide_damaged,
                    6 => app.vehicle_browser.hide_vlo = !app.vehicle_browser.hide_vlo,
                    7 => app.vehicle_browser.component_rotations.clear(),
                    _ => {}
                }
                return true;
            }
        }
    }
    let details = vehicle_details_rect();
    if !photo_mode && details.contains(mouse) {
        if is_mouse_button_pressed(MouseButton::Left) {
            if vehicle_texture_export_rect().contains(mouse) {
                export_selected_vehicle_texture(app);
                return true;
            }
            if vehicle_texture_copy_rect().contains(mouse) {
                copy_selected_vehicle_texture_name(app);
                return true;
            }
            if app.vehicle_browser.body_color_picker.is_none()
                && vehicle_copy_collision_rect().contains(mouse)
            {
                open_vehicle_collision_copy_dialog(app);
                return true;
            }
            let replace_visible = app.vehicle_browser.selected_part.is_some()
                && selected_vehicle(app).is_some_and(|vehicle| !vehicle.readonly);
            if replace_visible && vehicle_texture_replace_rect().contains(mouse) {
                open_selected_vehicle_texture_replace(app);
                return true;
            }
            if vehicle_show_all_rect().contains(mouse) {
                app.vehicle_browser.hidden_parts.clear();
                app.vehicle_browser.hidden_components.clear();
                return true;
            }
            if vehicle_hide_all_rect().contains(mouse) {
                if let Some(mesh) = app.vehicle_browser.preview_mesh.as_ref() {
                    let component_count = mesh.components.len().max(1);
                    app.vehicle_browser.hidden_components = (0..component_count).collect();
                }
                return true;
            }
            let list = vehicle_component_list_rect();
            if list.contains(mouse) {
                let row = ((mouse.y - list.y) / VEHICLE_COMPONENT_ROW_H)
                    .floor()
                    .max(0.0) as usize;
                let idx = app.vehicle_browser.component_scroll as usize + row;
                let rows = vehicle_component_rows(app);
                if let Some(row) = rows.get(idx).copied() {
                    match row {
                        VehicleComponentRow::Component(component) => {
                            if mouse.x < list.x + 22.0 {
                                // Caret / name: expand or collapse the component.
                                if !app.vehicle_browser.collapsed_components.insert(component) {
                                    app.vehicle_browser.collapsed_components.remove(&component);
                                }
                            } else if mouse.x < list.x + 44.0 {
                                if !app.vehicle_browser.hidden_components.insert(component) {
                                    app.vehicle_browser.hidden_components.remove(&component);
                                }
                            } else {
                                app.vehicle_browser.selected_component = Some(component);
                                app.vehicle_browser.selected_part = None;
                            }
                        }
                        VehicleComponentRow::Part(part) => {
                            if mouse.x < list.x + 58.0 {
                                if !app.vehicle_browser.hidden_parts.insert(part) {
                                    app.vehicle_browser.hidden_parts.remove(&part);
                                }
                            } else {
                                app.vehicle_browser.selected_part =
                                    if app.vehicle_browser.selected_part == Some(part) {
                                        None
                                    } else {
                                        Some(part)
                                    };
                                if let Some(mesh) = app.vehicle_browser.preview_mesh.as_ref()
                                    && let Some(render_part) = mesh.parts.get(part)
                                {
                                    app.vehicle_browser.selected_component =
                                        Some(render_part.component);
                                }
                            }
                        }
                    }
                }
                return true;
            }
        }
        let wheel = mouse_wheel().1;
        if wheel.abs() > 0.0 && vehicle_component_list_rect().contains(mouse) {
            let list = vehicle_component_list_rect();
            let rows = (list.h / VEHICLE_COMPONENT_ROW_H).floor().max(1.0) as usize;
            let count = vehicle_component_rows(app).len();
            let max_scroll = count.saturating_sub(rows) as f32;
            app.vehicle_browser.component_scroll =
                (app.vehicle_browser.component_scroll - wheel * 3.0).clamp(0.0, max_scroll);
            return true;
        }
        return true;
    }
    let wheel = mouse_wheel().1;
    if wheel.abs() > 0.0 && vehicle_list_rect().contains(mouse) {
        let rows = (vehicle_list_rect().h / VEHICLE_ROW_H).floor().max(1.0) as usize;
        let max_scroll = filtered_vehicle_indices(app).len().saturating_sub(rows) as f32;
        app.vehicle_browser.scroll =
            (app.vehicle_browser.scroll - wheel * 3.0).clamp(0.0, max_scroll);
        return true;
    }
    update_vehicle_text_input(app);
    true
}

fn update_vehicle_build_dialog(app: &mut AppState, mouse: Vec2) -> bool {
    if app
        .vehicle_browser
        .build_dialog
        .as_ref()
        .is_some_and(|dialog| dialog.category_manager_open)
    {
        return update_vehicle_category_manager(app, mouse);
    }
    if is_key_pressed(KeyCode::Escape) {
        app.vehicle_browser.build_dialog = None;
        return true;
    }
    let field_count = VEHICLE_BUILD_GENERAL_FIELDS + VEHICLE_HANDLING_FIELDS.len();
    let mut submit = false;

    let wheel = mouse_wheel().1;
    if wheel.abs() > 0.0 {
        let category_open = app
            .vehicle_browser
            .build_dialog
            .as_ref()
            .is_some_and(|dialog| dialog.category_dropdown_open);
        if category_open {
            let Some(dialog) = app.vehicle_browser.build_dialog.as_mut() else {
                return true;
            };
            let max_scroll = dialog
                .categories
                .len()
                .saturating_sub(VEHICLE_BUILD_DROPDOWN_VISIBLE) as f32;
            dialog.category_scroll = (dialog.category_scroll - wheel).clamp(0.0, max_scroll);
            return true;
        }
        let base_open = app
            .vehicle_browser
            .build_dialog
            .as_ref()
            .is_some_and(|dialog| dialog.base_dropdown_open);
        if base_open {
            let query = app
                .vehicle_browser
                .build_dialog
                .as_ref()
                .map(|dialog| dialog.base_search.clone())
                .unwrap_or_default();
            let max_scroll = base_vehicle_options(app, &query)
                .len()
                .saturating_sub(VEHICLE_BUILD_DROPDOWN_VISIBLE) as f32;
            if let Some(dialog) = app.vehicle_browser.build_dialog.as_mut() {
                dialog.base_scroll = (dialog.base_scroll - wheel).clamp(0.0, max_scroll);
            }
            return true;
        }
    }

    if is_mouse_button_pressed(MouseButton::Left) {
        let category_open = app
            .vehicle_browser
            .build_dialog
            .as_ref()
            .is_some_and(|dialog| dialog.category_dropdown_open);
        if category_open {
            let (start, visible, categories_len) = app
                .vehicle_browser
                .build_dialog
                .as_ref()
                .map(|dialog| {
                    let visible = VEHICLE_BUILD_DROPDOWN_VISIBLE.min(dialog.categories.len());
                    (
                        dialog.category_scroll.floor() as usize,
                        visible,
                        dialog.categories.len(),
                    )
                })
                .unwrap_or_default();
            for row in 0..visible {
                if vehicle_build_dropdown_option_rect(1, row, visible + 1).contains(mouse) {
                    if let Some(dialog) = app.vehicle_browser.build_dialog.as_mut()
                        && let Some(category) = dialog.categories.get(start + row).cloned()
                    {
                        dialog.category = category;
                        dialog.category_dropdown_open = false;
                    }
                    return true;
                }
            }
            if vehicle_category_manage_rect(visible).contains(mouse) {
                if let Some(dialog) = app.vehicle_browser.build_dialog.as_mut() {
                    dialog.category_dropdown_open = false;
                    dialog.category_manager_open = true;
                    dialog.category_new.clear();
                    dialog.category_new_cursor = 0;
                    dialog.category_new_anchor = None;
                }
                return true;
            }
            if let Some(dialog) = app.vehicle_browser.build_dialog.as_mut() {
                dialog.category_dropdown_open = false;
            }
            if !vehicle_build_value_rect(1).contains(mouse) || categories_len == 0 {
                return true;
            }
        }

        let base_open = app
            .vehicle_browser
            .build_dialog
            .as_ref()
            .is_some_and(|dialog| dialog.base_dropdown_open);
        if base_open {
            let query = app
                .vehicle_browser
                .build_dialog
                .as_ref()
                .map(|dialog| dialog.base_search.clone())
                .unwrap_or_default();
            let options = base_vehicle_options(app, &query);
            let visible = VEHICLE_BUILD_DROPDOWN_VISIBLE.min(options.len());
            let start = app
                .vehicle_browser
                .build_dialog
                .as_ref()
                .map(|dialog| dialog.base_scroll.floor() as usize)
                .unwrap_or(0);
            for row in 0..visible {
                if vehicle_build_dropdown_option_rect(2, row, visible.max(1)).contains(mouse) {
                    if let Some((id, _)) = options.get(start + row)
                        && let Some(dialog) = app.vehicle_browser.build_dialog.as_mut()
                    {
                        dialog.base_model = id.to_string();
                        dialog.base_search.clear();
                        dialog.base_dropdown_open = false;
                        dialog.active_field = None;
                    }
                    return true;
                }
            }
            if !vehicle_build_value_rect(2).contains(mouse) {
                if let Some(dialog) = app.vehicle_browser.build_dialog.as_mut() {
                    dialog.base_dropdown_open = false;
                    dialog.base_search.clear();
                    dialog.active_field = None;
                }
                return true;
            }
        }

        let handling_dropdown = app
            .vehicle_browser
            .build_dialog
            .as_ref()
            .and_then(|dialog| dialog.handling_dropdown);
        if let Some(index) = handling_dropdown {
            let property = VEHICLE_HANDLING_FIELDS[index];
            if let Some(options) = vehicle_handling_options(property) {
                let field = index + VEHICLE_BUILD_GENERAL_FIELDS;
                for (row, (value, _)) in options.iter().enumerate() {
                    if vehicle_build_dropdown_option_rect(field, row, options.len()).contains(mouse)
                    {
                        if let Some(dialog) = app.vehicle_browser.build_dialog.as_mut() {
                            dialog.handling[index].1 = (*value).to_string();
                            dialog.handling_dropdown = None;
                            dialog.active_field = None;
                        }
                        return true;
                    }
                }
            }
            if let Some(dialog) = app.vehicle_browser.build_dialog.as_mut() {
                dialog.handling_dropdown = None;
            }
            return true;
        }

        if vehicle_build_cancel_rect().contains(mouse) {
            app.vehicle_browser.build_dialog = None;
            return true;
        }
        if vehicle_build_submit_rect().contains(mouse) {
            submit = true;
        } else if vehicle_build_mode_rect(true).contains(mouse) {
            if let Some(dialog) = app.vehicle_browser.build_dialog.as_mut() {
                dialog.new_vehicle = true;
                dialog.active_field = None;
                dialog.category_dropdown_open = false;
                dialog.base_dropdown_open = false;
            }
        } else if vehicle_build_mode_rect(false).contains(mouse) {
            if let Some(dialog) = app.vehicle_browser.build_dialog.as_mut() {
                dialog.new_vehicle = false;
                dialog.active_field = None;
                dialog.category_dropdown_open = false;
                dialog.base_dropdown_open = false;
            }
        } else if vehicle_build_value_rect(1).contains(mouse) {
            if let Some(dialog) = app.vehicle_browser.build_dialog.as_mut() {
                dialog.category_dropdown_open = !dialog.category_dropdown_open;
                dialog.base_dropdown_open = false;
                dialog.handling_dropdown = None;
                dialog.active_field = None;
            }
        } else if vehicle_build_value_rect(2).contains(mouse) {
            if let Some(dialog) = app.vehicle_browser.build_dialog.as_mut() {
                dialog.base_dropdown_open = true;
                dialog.base_search.clear();
                dialog.base_scroll = 0.0;
                dialog.category_dropdown_open = false;
                dialog.handling_dropdown = None;
                dialog.active_field = Some(2);
                dialog.cursor = 0;
                dialog.selection_anchor = None;
            }
        } else {
            let selected =
                (0..field_count).find(|index| vehicle_build_value_rect(*index).contains(mouse));
            if let Some(dialog) = app.vehicle_browser.build_dialog.as_mut() {
                let dropdown = selected.and_then(|field| {
                    (field >= VEHICLE_BUILD_GENERAL_FIELDS)
                        .then_some(field - VEHICLE_BUILD_GENERAL_FIELDS)
                        .filter(|index| {
                            vehicle_handling_options(VEHICLE_HANDLING_FIELDS[*index]).is_some()
                        })
                });
                dialog.handling_dropdown = dropdown;
                dialog.active_field = if dropdown.is_some() { None } else { selected };
                dialog.category_dropdown_open = false;
                dialog.base_dropdown_open = false;
                dialog.selection_anchor = None;
                dialog.cursor = selected
                    .and_then(|index| {
                        build_dialog_field_mut(dialog, index).map(|value| value.len())
                    })
                    .unwrap_or(0);
            }
        }
    }
    if submit {
        submit_vehicle_build(app);
        return true;
    }

    let Some(dialog) = app.vehicle_browser.build_dialog.as_mut() else {
        return true;
    };
    let Some(active) = dialog.active_field else {
        drain_text_input();
        return true;
    };
    let mut value = build_dialog_field_mut(dialog, active)
        .map(std::mem::take)
        .unwrap_or_default();
    let original_value = value.clone();
    dialog.cursor = clamp_char_boundary(&value, dialog.cursor);
    if handle_text_clipboard_shortcuts(&mut value, &mut dialog.cursor, &mut dialog.selection_anchor)
    {
        drain_text_input();
    }
    if is_key_pressed(KeyCode::Backspace)
        && !delete_text_selection(&mut value, &mut dialog.cursor, &mut dialog.selection_anchor)
        && dialog.cursor > 0
    {
        let previous = prev_char_boundary(&value, dialog.cursor);
        value.replace_range(previous..dialog.cursor, "");
        dialog.cursor = previous;
    }
    if is_key_pressed(KeyCode::Delete)
        && !delete_text_selection(&mut value, &mut dialog.cursor, &mut dialog.selection_anchor)
        && dialog.cursor < value.len()
    {
        let next = next_char_boundary(&value, dialog.cursor);
        value.replace_range(dialog.cursor..next, "");
    }
    if is_key_pressed(KeyCode::Left) {
        dialog.cursor = prev_char_boundary(&value, dialog.cursor);
        dialog.selection_anchor = None;
    }
    if is_key_pressed(KeyCode::Right) {
        dialog.cursor = next_char_boundary(&value, dialog.cursor);
        dialog.selection_anchor = None;
    }
    while let Some(ch) = get_char_pressed() {
        if handle_text_control_char(
            &mut value,
            &mut dialog.cursor,
            &mut dialog.selection_anchor,
            ch,
        ) {
            continue;
        }
        if !ch.is_control() {
            insert_text_at_cursor(
                &mut value,
                &mut dialog.cursor,
                &mut dialog.selection_anchor,
                &ch.to_string(),
            );
        }
    }
    if let Some(field) = build_dialog_field_mut(dialog, active) {
        *field = value;
    }
    if active == 2 && build_dialog_field_value(dialog, active) != original_value {
        dialog.base_scroll = 0.0;
    }
    true
}

fn update_vehicle_category_manager(app: &mut AppState, mouse: Vec2) -> bool {
    if is_key_pressed(KeyCode::Escape) {
        if let Some(dialog) = app.vehicle_browser.build_dialog.as_mut() {
            dialog.category_manager_open = false;
        }
        drain_text_input();
        return true;
    }

    let list = vehicle_category_manager_list_rect();
    let visible_rows = (list.h / 38.0).floor().max(1.0) as usize;
    let wheel = mouse_wheel().1;
    if wheel.abs() > 0.0 && list.contains(mouse) {
        if let Some(dialog) = app.vehicle_browser.build_dialog.as_mut() {
            let max_scroll = dialog.categories.len().saturating_sub(visible_rows) as f32;
            dialog.category_manager_scroll =
                (dialog.category_manager_scroll - wheel).clamp(0.0, max_scroll);
        }
        return true;
    }

    let mut add = is_key_pressed(KeyCode::Enter);
    if is_mouse_button_pressed(MouseButton::Left) {
        if vehicle_category_manager_done_rect().contains(mouse) {
            if let Some(dialog) = app.vehicle_browser.build_dialog.as_mut() {
                dialog.category_manager_open = false;
            }
            drain_text_input();
            return true;
        }
        if vehicle_category_manager_add_rect().contains(mouse) {
            add = true;
        }
        let categories = app
            .vehicle_browser
            .build_dialog
            .as_ref()
            .map(|dialog| dialog.categories.clone())
            .unwrap_or_default();
        let start = app
            .vehicle_browser
            .build_dialog
            .as_ref()
            .map(|dialog| dialog.category_manager_scroll.floor() as usize)
            .unwrap_or(0);
        for row in 0..visible_rows {
            let Some(category) = categories.get(start + row) else {
                break;
            };
            let built_in = DEFAULT_VEHICLE_CATEGORIES
                .iter()
                .any(|value| value.eq_ignore_ascii_case(category));
            let used = app
                .vehicle_browser
                .build_dialog
                .as_ref()
                .is_some_and(|dialog| dialog.category_used.contains(&lower(category)));
            if !built_in && !used && vehicle_category_manager_remove_rect(row).contains(mouse) {
                let Some(loader) = app.vehicle_loader_resource.clone() else {
                    return true;
                };
                if let Some(dialog) = app.vehicle_browser.build_dialog.as_mut() {
                    dialog
                        .categories
                        .retain(|value| !value.eq_ignore_ascii_case(category));
                    if dialog.category.eq_ignore_ascii_case(category) {
                        dialog.category = "Car".to_string();
                    }
                    if let Err(err) = save_vehicle_categories(&loader, &dialog.categories) {
                        app.status_message = err;
                    }
                    app.vehicle_loader_categories = dialog.categories.clone();
                }
                return true;
            }
        }
    }

    if add {
        let Some(loader) = app.vehicle_loader_resource.clone() else {
            return true;
        };
        if let Some(dialog) = app.vehicle_browser.build_dialog.as_mut() {
            let category = dialog.category_new.trim().to_string();
            if !category.is_empty()
                && !dialog
                    .categories
                    .iter()
                    .any(|value| value.eq_ignore_ascii_case(&category))
            {
                dialog.categories.push(category.clone());
                dialog
                    .categories
                    .sort_by_key(|value| value.to_ascii_lowercase());
                dialog.category = category;
                if let Err(err) = save_vehicle_categories(&loader, &dialog.categories) {
                    app.status_message = err;
                }
                app.vehicle_loader_categories = dialog.categories.clone();
            }
            dialog.category_new.clear();
            dialog.category_new_cursor = 0;
            dialog.category_new_anchor = None;
        }
        drain_text_input();
        return true;
    }

    let Some(dialog) = app.vehicle_browser.build_dialog.as_mut() else {
        return true;
    };
    let mut value = std::mem::take(&mut dialog.category_new);
    dialog.category_new_cursor = clamp_char_boundary(&value, dialog.category_new_cursor);
    handle_text_clipboard_shortcuts(
        &mut value,
        &mut dialog.category_new_cursor,
        &mut dialog.category_new_anchor,
    );
    if is_key_pressed(KeyCode::Backspace)
        && !delete_text_selection(
            &mut value,
            &mut dialog.category_new_cursor,
            &mut dialog.category_new_anchor,
        )
        && dialog.category_new_cursor > 0
    {
        let previous = prev_char_boundary(&value, dialog.category_new_cursor);
        value.replace_range(previous..dialog.category_new_cursor, "");
        dialog.category_new_cursor = previous;
    }
    if is_key_pressed(KeyCode::Delete)
        && !delete_text_selection(
            &mut value,
            &mut dialog.category_new_cursor,
            &mut dialog.category_new_anchor,
        )
        && dialog.category_new_cursor < value.len()
    {
        let next = next_char_boundary(&value, dialog.category_new_cursor);
        value.replace_range(dialog.category_new_cursor..next, "");
    }
    while let Some(ch) = get_char_pressed() {
        if handle_text_control_char(
            &mut value,
            &mut dialog.category_new_cursor,
            &mut dialog.category_new_anchor,
            ch,
        ) {
            continue;
        }
        if !ch.is_control() && ch != '\n' && ch != '\r' {
            insert_text_at_cursor(
                &mut value,
                &mut dialog.category_new_cursor,
                &mut dialog.category_new_anchor,
                &ch.to_string(),
            );
        }
    }
    dialog.category_new = value;
    true
}

pub(crate) fn update_vehicle_dictionary_manager(app: &mut AppState, mouse: Vec2) -> bool {
    if !app.vehicle_browser.manage_dictionaries {
        return false;
    }
    let dialog = vehicle_dictionary_dialog_rect();
    let list = vehicle_dictionary_list_rect();
    let visible_rows = (list.h / VEHICLE_DICTIONARY_ROW_H).floor().max(1.0) as usize;
    let max_scroll = app
        .custom_vehicle_dictionaries
        .len()
        .saturating_sub(visible_rows) as f32;
    app.vehicle_browser.dictionary_scroll =
        app.vehicle_browser.dictionary_scroll.clamp(0.0, max_scroll);
    let (_, wheel) = mouse_wheel();
    if list.contains(mouse) && wheel.abs() > 0.0 {
        app.vehicle_browser.dictionary_scroll =
            (app.vehicle_browser.dictionary_scroll - wheel).clamp(0.0, max_scroll);
    }
    if is_key_pressed(KeyCode::Escape) {
        app.vehicle_browser.manage_dictionaries = false;
        return true;
    }
    if !is_mouse_button_pressed(MouseButton::Left) {
        return true;
    }
    if vehicle_dictionary_add_rect().contains(mouse) {
        open_vehicle_folder_picker(app);
        return true;
    }
    if vehicle_dictionary_close_rect().contains(mouse) || !dialog.contains(mouse) {
        app.vehicle_browser.manage_dictionaries = false;
        return true;
    }
    let start = app.vehicle_browser.dictionary_scroll.floor() as usize;
    for visible_row in 0..visible_rows {
        let index = start + visible_row;
        if index >= app.custom_vehicle_dictionaries.len() {
            break;
        }
        if vehicle_dictionary_remove_rect(visible_row).contains(mouse) {
            remove_custom_vehicle_dictionary(app, index);
            let new_max = app
                .custom_vehicle_dictionaries
                .len()
                .saturating_sub(visible_rows) as f32;
            app.vehicle_browser.dictionary_scroll =
                app.vehicle_browser.dictionary_scroll.min(new_max);
            return true;
        }
    }
    true
}

pub(crate) fn draw_vehicle_dictionary_manager(app: &AppState) {
    if !app.vehicle_browser.manage_dictionaries {
        return;
    }
    draw_modal_backdrop();
    let dialog = vehicle_dictionary_dialog_rect();
    let list = vehicle_dictionary_list_rect();
    draw_panel_rect(
        &app.ui_font,
        dialog,
        Some("Manage Custom Vehicle Dictionaries"),
    );
    ui_text(
        &app.ui_font,
        "Folders are scanned recursively for same-named DFF and TXD pairs.",
        dialog.x + 24.0,
        dialog.y + 58.0,
        ui_muted(),
    );
    draw_rrect_bordered(
        list.x,
        list.y,
        list.w,
        list.h,
        7.0,
        1.0,
        Color::new(0.045, 0.052, 0.064, 1.0),
        ui_border(),
    );
    let visible_rows = (list.h / VEHICLE_DICTIONARY_ROW_H).floor().max(1.0) as usize;
    let max_scroll = app
        .custom_vehicle_dictionaries
        .len()
        .saturating_sub(visible_rows) as f32;
    let scroll = app.vehicle_browser.dictionary_scroll.clamp(0.0, max_scroll);
    let start = scroll.floor() as usize;
    if app.custom_vehicle_dictionaries.is_empty() {
        ui_text(
            &app.ui_font,
            "No custom vehicle dictionaries have been added.",
            list.x + 16.0,
            list.y + 28.0,
            LIGHTGRAY,
        );
    } else {
        for visible_row in 0..visible_rows {
            let index = start + visible_row;
            let Some(path) = app.custom_vehicle_dictionaries.get(index) else {
                break;
            };
            let y = list.y + visible_row as f32 * VEHICLE_DICTIONARY_ROW_H;
            if visible_row % 2 == 0 {
                draw_rrect(
                    list.x + 5.0,
                    y + 4.0,
                    list.w - 10.0,
                    VEHICLE_DICTIONARY_ROW_H - 8.0,
                    6.0,
                    Color::new(0.072, 0.083, 0.100, 1.0),
                );
            }
            ui_text(
                &app.ui_font,
                &ellipsize_width(path.to_string_lossy().as_ref(), 15, list.w - 126.0),
                list.x + 14.0,
                y + 26.0,
                LIGHTGRAY,
            );
            draw_dialog_button(
                &app.ui_font,
                vehicle_dictionary_remove_rect(visible_row),
                "Remove",
                false,
            );
        }
    }
    if app.custom_vehicle_dictionaries.len() > visible_rows {
        let track = Rect::new(list.x + list.w - 8.0, list.y + 8.0, 3.0, list.h - 16.0);
        draw_rrect(
            track.x,
            track.y,
            track.w,
            track.h,
            2.0,
            Color::new(0.12, 0.14, 0.17, 1.0),
        );
        let thumb_h = (track.h * visible_rows as f32
            / app.custom_vehicle_dictionaries.len() as f32)
            .clamp(24.0, track.h);
        let thumb_y = track.y + (track.h - thumb_h) * (scroll / max_scroll.max(1.0));
        draw_rrect(track.x, thumb_y, track.w, thumb_h, 2.0, ui_accent());
    }
    if app.vehicle_dictionary_scan_rx.is_some() {
        ui_text(
            &app.ui_font,
            "Refreshing vehicle dictionaries...",
            dialog.x + 24.0,
            dialog.y + dialog.h - 28.0,
            ui_accent(),
        );
    }
    draw_dialog_button(&app.ui_font, vehicle_dictionary_add_rect(), "Add", true);
    draw_dialog_button(
        &app.ui_font,
        vehicle_dictionary_close_rect(),
        "Close",
        false,
    );
}

fn build_dialog_field_value(dialog: &VehicleBuildDialog, index: usize) -> &str {
    match index {
        0 => &dialog.name,
        1 => &dialog.category,
        2 => &dialog.base_search,
        3 => &dialog.wheel_front,
        4 => &dialog.wheel_rear,
        5 => &dialog.wheel_width,
        _ => dialog
            .handling
            .get(index - VEHICLE_BUILD_GENERAL_FIELDS)
            .map(|(_, value)| value.as_str())
            .unwrap_or(""),
    }
}

pub(crate) fn draw_vehicle_build_dialog(app: &AppState) {
    let Some(dialog) = app.vehicle_browser.build_dialog.as_ref() else {
        return;
    };
    draw_modal_backdrop();
    let panel = vehicle_build_dialog_rect();
    draw_panel_rect(&app.ui_font, panel, Some("Duplicate / Load Vehicle"));
    let loader = app
        .vehicle_loader_resource
        .as_ref()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|| "No loader selected".to_string());
    ui_text(
        &app.ui_font,
        &ellipsize_width(&format!("Vehicle loader: {loader}"), 15, panel.w - 48.0),
        panel.x + 24.0,
        panel.y + 58.0,
        ui_muted(),
    );

    let labels = [
        "Display name",
        "Category",
        "Base GTA model",
        "Front wheel radius",
        "Rear wheel radius",
        "Wheel/track width",
    ];
    for index in 0..VEHICLE_BUILD_GENERAL_FIELDS {
        let row = vehicle_build_field_rect(index);
        let value_rect = vehicle_build_value_rect(index);
        ui_text(&app.ui_font, labels[index], row.x, row.y + 21.0, LIGHTGRAY);
        draw_rrect_bordered(
            value_rect.x,
            value_rect.y,
            value_rect.w,
            value_rect.h,
            6.0,
            1.0,
            Color::new(0.045, 0.052, 0.064, 1.0),
            if dialog.active_field == Some(index)
                || (index == 1 && dialog.category_dropdown_open)
                || (index == 2 && dialog.base_dropdown_open)
            {
                ui_accent()
            } else {
                ui_border()
            },
        );
        let value = match index {
            2 if dialog.base_dropdown_open => {
                if dialog.base_search.is_empty() {
                    "Search by vehicle name or ID".to_string()
                } else {
                    dialog.base_search.clone()
                }
            }
            2 => selected_base_vehicle_label(app, dialog),
            _ => build_dialog_field_value(dialog, index).to_string(),
        };
        ui_text(
            &app.ui_font,
            &ellipsize_width(&value, 15, value_rect.w - 16.0),
            value_rect.x + 8.0,
            value_rect.y + 21.0,
            if index == 2 && dialog.base_dropdown_open && dialog.base_search.is_empty() {
                ui_muted()
            } else {
                WHITE
            },
        );
        if index == 1 || index == 2 {
            ui_text(
                &app.ui_font,
                if (index == 1 && dialog.category_dropdown_open)
                    || (index == 2 && dialog.base_dropdown_open)
                {
                    "^"
                } else {
                    "v"
                },
                value_rect.x + value_rect.w - 20.0,
                value_rect.y + 21.0,
                ui_dim(),
            );
        }
    }
    text_button(
        &app.ui_font,
        vehicle_build_mode_rect(true),
        "Genuinely New",
        dialog.new_vehicle,
    );
    text_button(
        &app.ui_font,
        vehicle_build_mode_rect(false),
        "Replace Default",
        !dialog.new_vehicle,
    );
    ui_text(
        &app.ui_font,
        if dialog.new_vehicle {
            "ID assigned automatically; handling inherits from the base model."
        } else {
            "The base model is replaced; IDE wheel radii stay absolute and correctly scaled."
        },
        panel.x + 338.0,
        panel.y + 245.0,
        ui_muted(),
    );
    ui_text(
        &app.ui_font,
        "Handling overrides  |  gray values are inherited from the selected GTA model",
        panel.x + 24.0,
        panel.y + 282.0,
        ui_dim(),
    );
    for index in 0..dialog.handling.len() {
        let field = index + VEHICLE_BUILD_GENERAL_FIELDS;
        let row = vehicle_build_field_rect(field);
        let value_rect = vehicle_build_value_rect(field);
        let property = dialog.handling[index].0;
        let override_value = dialog.handling[index].1.trim();
        let inherited = selected_base_vehicle(app, dialog)
            .and_then(|vehicle| vehicle.handling.get(property))
            .map(String::as_str)
            .unwrap_or("");
        let display = if override_value.is_empty() {
            vehicle_handling_display(property, inherited)
        } else {
            vehicle_handling_display(property, override_value)
        };
        ui_text(
            &app.ui_font,
            vehicle_handling_label(property),
            row.x,
            row.y + 19.0,
            LIGHTGRAY,
        );
        draw_rrect_bordered(
            value_rect.x,
            value_rect.y,
            value_rect.w,
            value_rect.h,
            5.0,
            1.0,
            Color::new(0.045, 0.052, 0.064, 1.0),
            if dialog.active_field == Some(field) || dialog.handling_dropdown == Some(index) {
                ui_accent()
            } else {
                ui_border()
            },
        );
        ui_text(
            &app.ui_font,
            &ellipsize_width(&display, 14, value_rect.w - 24.0),
            value_rect.x + 7.0,
            value_rect.y + 19.0,
            if override_value.is_empty() {
                ui_muted()
            } else {
                WHITE
            },
        );
        if vehicle_handling_options(property).is_some() {
            ui_text(
                &app.ui_font,
                if dialog.handling_dropdown == Some(index) {
                    "^"
                } else {
                    "v"
                },
                value_rect.x + value_rect.w - 18.0,
                value_rect.y + 19.0,
                ui_dim(),
            );
        }
    }
    draw_dialog_button(
        &app.ui_font,
        vehicle_build_submit_rect(),
        "Load",
        app.vehicle_build_rx.is_some(),
    );
    draw_dialog_button(&app.ui_font, vehicle_build_cancel_rect(), "Cancel", false);
    draw_vehicle_build_dropdowns(app, dialog);
    if dialog.category_manager_open {
        draw_vehicle_category_manager(app, dialog);
    }
}

pub(crate) fn draw_vehicle_collision_copy_dialog(app: &AppState) {
    let Some(dialog) = app.vehicle_browser.collision_copy_dialog.as_ref() else {
        return;
    };
    draw_modal_backdrop();
    let panel = vehicle_collision_copy_dialog_rect();
    draw_panel_rect(&app.ui_font, panel, Some("Replace Vehicle Collision"));
    ui_text(
        &app.ui_font,
        &ellipsize_width(
            &format!(
                "Target: {}  •  {}",
                dialog.target.id,
                dialog
                    .target
                    .loose_dff_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| dialog.target.dff.clone())
            ),
            16,
            panel.w - 48.0,
        ),
        panel.x + 24.0,
        panel.y + 58.0,
        LIGHTGRAY,
    );
    ui_text(
        &app.ui_font,
        &ellipsize_width(
            "The source completely replaces the target's mesh, volumes, shadow data, bounds, and flags.",
            16,
            panel.w - 48.0,
        ),
        panel.x + 24.0,
        panel.y + 82.0,
        Color::new(0.94, 0.70, 0.32, 1.0),
    );

    let search = vehicle_collision_copy_search_rect();
    draw_rrect_bordered(
        search.x,
        search.y,
        search.w,
        search.h,
        7.0,
        1.0,
        ui_input_bg(),
        ui_accent(),
    );
    let search_label = if dialog.search.is_empty() {
        "Search source vehicle, DFF, or model ID"
    } else {
        &dialog.search
    };
    ui_text(
        &app.ui_font,
        &ellipsize_width(search_label, 16, search.w - 20.0),
        search.x + 10.0,
        search.y + 22.0,
        if dialog.search.is_empty() {
            ui_muted()
        } else {
            WHITE
        },
    );

    let indices = collision_copy_source_indices(&app.vehicles, dialog);
    let list = vehicle_collision_copy_list_rect();
    draw_rrect_bordered(
        list.x,
        list.y,
        list.w,
        list.h,
        7.0,
        1.0,
        Color::new(0.050, 0.058, 0.070, 1.0),
        ui_border(),
    );
    let visible_rows = (list.h / VEHICLE_COLLISION_COPY_ROW_H).floor().max(1.0) as usize;
    let start = dialog.scroll.floor() as usize;
    for row in 0..visible_rows {
        let Some(source_index) = indices.get(start + row).copied() else {
            break;
        };
        let vehicle = &app.vehicles[source_index];
        let row_rect = Rect::new(
            list.x + 4.0,
            list.y + row as f32 * VEHICLE_COLLISION_COPY_ROW_H + 3.0,
            list.w - 8.0,
            VEHICLE_COLLISION_COPY_ROW_H - 6.0,
        );
        if dialog.selected_source == Some(source_index) {
            draw_rrect(
                row_rect.x,
                row_rect.y,
                row_rect.w,
                row_rect.h,
                6.0,
                ui_surface_active(),
            );
        } else if row % 2 == 0 {
            draw_rrect(
                row_rect.x,
                row_rect.y,
                row_rect.w,
                row_rect.h,
                6.0,
                Color::new(0.086, 0.098, 0.116, 1.0),
            );
        }
        let model = vehicle
            .model_id
            .map(|id| format!("  •  model {id}"))
            .unwrap_or_default();
        ui_text(
            &app.ui_font,
            &ellipsize_width(
                &format!("{}  •  DFF {}{}", vehicle.id, vehicle.dff, model),
                16,
                row_rect.w - 24.0,
            ),
            row_rect.x + 10.0,
            row_rect.y + 21.0,
            WHITE,
        );
    }
    if indices.is_empty() {
        ui_text(
            &app.ui_font,
            "No other vehicles match this search.",
            list.x + 12.0,
            list.y + 28.0,
            ui_muted(),
        );
    }
    ui_text(
        &app.ui_font,
        &format!("{} source vehicle(s)", indices.len()),
        panel.x + 24.0,
        panel.y + panel.h - 28.0,
        ui_muted(),
    );
    draw_dialog_button(
        &app.ui_font,
        vehicle_collision_copy_submit_rect(),
        "Replace All",
        dialog.selected_source.is_some(),
    );
    draw_dialog_button(
        &app.ui_font,
        vehicle_collision_copy_cancel_rect(),
        "Cancel",
        false,
    );
}

fn draw_vehicle_build_dropdowns(app: &AppState, dialog: &VehicleBuildDialog) {
    let mouse: Vec2 = mouse_position().into();
    if dialog.category_dropdown_open {
        let visible = VEHICLE_BUILD_DROPDOWN_VISIBLE.min(dialog.categories.len());
        let start = dialog.category_scroll.floor() as usize;
        for row in 0..visible {
            let rect = vehicle_build_dropdown_option_rect(1, row, visible + 1);
            let Some(category) = dialog.categories.get(start + row) else {
                continue;
            };
            let selected = category.eq_ignore_ascii_case(&dialog.category);
            draw_rrect_bordered(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                4.0,
                1.0,
                if selected {
                    ui_surface_active()
                } else if rect.contains(mouse) {
                    ui_surface_hover()
                } else {
                    Color::new(0.04, 0.047, 0.058, 1.0)
                },
                ui_border(),
            );
            ui_text(
                &app.ui_font,
                category,
                rect.x + 9.0,
                rect.y + 19.0,
                if selected { ui_accent() } else { WHITE },
            );
        }
        let manage = vehicle_category_manage_rect(visible);
        draw_rrect_bordered(
            manage.x,
            manage.y,
            manage.w,
            manage.h,
            4.0,
            1.0,
            if manage.contains(mouse) {
                ui_surface_hover()
            } else {
                Color::new(0.055, 0.064, 0.078, 1.0)
            },
            ui_accent(),
        );
        ui_text(
            &app.ui_font,
            "+  Manage categories...",
            manage.x + 9.0,
            manage.y + 19.0,
            ui_accent(),
        );
    }

    if dialog.base_dropdown_open {
        let options = base_vehicle_options(app, &dialog.base_search);
        let visible = VEHICLE_BUILD_DROPDOWN_VISIBLE.min(options.len());
        let start = dialog.base_scroll.floor() as usize;
        if visible == 0 {
            let rect = vehicle_build_dropdown_option_rect(2, 0, 1);
            draw_rrect_bordered(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                4.0,
                1.0,
                Color::new(0.04, 0.047, 0.058, 1.0),
                ui_border(),
            );
            ui_text(
                &app.ui_font,
                "No matching GTA vehicles",
                rect.x + 9.0,
                rect.y + 19.0,
                ui_muted(),
            );
        } else {
            for row in 0..visible {
                let rect = vehicle_build_dropdown_option_rect(2, row, visible);
                let Some((id, name)) = options.get(start + row) else {
                    continue;
                };
                let selected = dialog.base_model == id.to_string();
                draw_rrect_bordered(
                    rect.x,
                    rect.y,
                    rect.w,
                    rect.h,
                    4.0,
                    1.0,
                    if selected {
                        ui_surface_active()
                    } else if rect.contains(mouse) {
                        ui_surface_hover()
                    } else {
                        Color::new(0.04, 0.047, 0.058, 1.0)
                    },
                    ui_border(),
                );
                ui_text(
                    &app.ui_font,
                    &ellipsize_width(name, 14, rect.w - 64.0),
                    rect.x + 9.0,
                    rect.y + 19.0,
                    if selected { ui_accent() } else { WHITE },
                );
                let id_label = id.to_string();
                ui_text(
                    &app.ui_font,
                    &id_label,
                    rect.x + rect.w - ui_text_width(&id_label, 14) - 10.0,
                    rect.y + 19.0,
                    ui_muted(),
                );
            }
        }
    }

    if let Some(index) = dialog.handling_dropdown {
        let property = dialog.handling[index].0;
        let Some(options) = vehicle_handling_options(property) else {
            return;
        };
        let field = index + VEHICLE_BUILD_GENERAL_FIELDS;
        for (row, (value, label)) in options.iter().enumerate() {
            let rect = vehicle_build_dropdown_option_rect(field, row, options.len());
            let selected = dialog.handling[index].1.eq_ignore_ascii_case(value);
            draw_rrect_bordered(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                4.0,
                1.0,
                if selected {
                    ui_surface_active()
                } else if rect.contains(mouse) {
                    ui_surface_hover()
                } else {
                    Color::new(0.04, 0.047, 0.058, 1.0)
                },
                ui_border(),
            );
            let display = if value.is_empty() {
                selected_base_vehicle(app, dialog)
                    .and_then(|vehicle| vehicle.handling.get(property))
                    .map(|inherited| {
                        format!(
                            "Inherit  -  {}",
                            vehicle_handling_display(property, inherited)
                        )
                    })
                    .unwrap_or_else(|| (*label).to_string())
            } else {
                (*label).to_string()
            };
            ui_text(
                &app.ui_font,
                &ellipsize_width(&display, 14, rect.w - 16.0),
                rect.x + 8.0,
                rect.y + 19.0,
                if selected { ui_accent() } else { WHITE },
            );
        }
    }
}

fn draw_vehicle_category_manager(app: &AppState, dialog: &VehicleBuildDialog) {
    draw_modal_backdrop();
    let panel = vehicle_category_manager_rect();
    let list = vehicle_category_manager_list_rect();
    draw_panel_rect(&app.ui_font, panel, Some("Manage Vehicle Categories"));
    let input = vehicle_category_manager_input_rect();
    draw_rrect_bordered(
        input.x,
        input.y,
        input.w,
        input.h,
        6.0,
        1.0,
        Color::new(0.045, 0.052, 0.064, 1.0),
        ui_accent(),
    );
    ui_text(
        &app.ui_font,
        if dialog.category_new.is_empty() {
            "New category name"
        } else {
            &dialog.category_new
        },
        input.x + 9.0,
        input.y + 21.0,
        if dialog.category_new.is_empty() {
            ui_muted()
        } else {
            WHITE
        },
    );
    draw_dialog_button(
        &app.ui_font,
        vehicle_category_manager_add_rect(),
        "Add",
        true,
    );
    draw_rrect_bordered(
        list.x,
        list.y,
        list.w,
        list.h,
        6.0,
        1.0,
        Color::new(0.04, 0.047, 0.058, 1.0),
        ui_border(),
    );
    let visible_rows = (list.h / 38.0).floor().max(1.0) as usize;
    let start = dialog.category_manager_scroll.floor() as usize;
    for row in 0..visible_rows {
        let Some(category) = dialog.categories.get(start + row) else {
            break;
        };
        let y = list.y + row as f32 * 38.0;
        if row % 2 == 0 {
            draw_rrect(
                list.x + 4.0,
                y + 3.0,
                list.w - 8.0,
                32.0,
                4.0,
                Color::new(0.06, 0.07, 0.085, 1.0),
            );
        }
        ui_text(
            &app.ui_font,
            category,
            list.x + 10.0,
            y + 24.0,
            if category.eq_ignore_ascii_case(&dialog.category) {
                ui_accent()
            } else {
                WHITE
            },
        );
        let built_in = DEFAULT_VEHICLE_CATEGORIES
            .iter()
            .any(|value| value.eq_ignore_ascii_case(category));
        let used = dialog.category_used.contains(&lower(category));
        if built_in || used {
            ui_text(
                &app.ui_font,
                if built_in { "Built in" } else { "In use" },
                list.x + list.w - 74.0,
                y + 24.0,
                ui_muted(),
            );
        } else {
            draw_dialog_button(
                &app.ui_font,
                vehicle_category_manager_remove_rect(row),
                "Remove",
                false,
            );
        }
    }
    ui_text(
        &app.ui_font,
        "In-use categories cannot be removed.",
        panel.x + 24.0,
        panel.y + panel.h - 25.0,
        ui_muted(),
    );
    draw_dialog_button(
        &app.ui_font,
        vehicle_category_manager_done_rect(),
        "Done",
        false,
    );
}

/// Small visibility indicator: filled when visible, outlined when hidden.
fn draw_visibility_dot(x: f32, y: f32, visible: bool) {
    if visible {
        draw_rrect(x, y, 12.0, 12.0, 3.0, ui_accent());
    } else {
        draw_rrect_bordered(
            x,
            y,
            12.0,
            12.0,
            3.0,
            1.0,
            Color::new(0.0, 0.0, 0.0, 0.0),
            ui_muted(),
        );
    }
}

fn draw_small_button(font: &Font, rect: Rect, label: &str) {
    let mouse: Vec2 = mouse_position().into();
    let hovered = rect.contains(mouse);
    draw_rrect_bordered(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        6.0,
        1.0,
        if hovered {
            Color::new(0.20, 0.23, 0.27, 1.0)
        } else {
            Color::new(0.13, 0.15, 0.18, 1.0)
        },
        if hovered {
            ui_accent()
        } else {
            Color::new(0.25, 0.29, 0.34, 1.0)
        },
    );
    let tw = ui_text_width(label, 16);
    ui_text(
        font,
        label,
        rect.x + ((rect.w - tw) * 0.5).max(4.0),
        rect.y + 18.0,
        LIGHTGRAY,
    );
}

/// Decode a texture (scoped to the vehicle's TXD when possible, falling back
/// to any TXD or a loose PNG that provides it) into raw RGBA.
pub(crate) fn vehicle_texture_rgba(
    app: &AppState,
    texture_name: &str,
    txd: &str,
) -> Option<(u32, u32, Vec<u8>)> {
    let name = texture_name.trim();
    if name.is_empty() {
        return None;
    }
    let key = lower(name);
    let txd_key = asset_key(txd, ".txd");
    app.txd_textures
        .get(&key)
        .and_then(|entries| {
            entries
                .iter()
                .find(|entry| entry.txd_name.eq_ignore_ascii_case(&txd_key))
                .or_else(|| entries.first())
        })
        .and_then(decode_txd_texture)
        .or_else(|| {
            // Fall back to a pre-rendered PNG (txd_build/) when present.
            let path = app.texture_files.get(&key)?;
            let bytes = fs::read(path).ok()?;
            let image = image::load_from_memory(&bytes).ok()?;
            let rgba = image.to_rgba8();
            let (width, height) = rgba.dimensions();
            Some((width, height, rgba.into_raw()))
        })
}

/// Decode a texture (scoped to the vehicle's TXD when possible) into a UI
/// texture for the preview pane, cached per vehicle.
pub(crate) fn vehicle_texture_preview(
    app: &mut AppState,
    texture_name: &str,
    txd: &str,
) -> Option<(Texture2D, u32, u32)> {
    let name = texture_name.trim();
    if name.is_empty() {
        return None;
    }
    let key = lower(name);
    let txd_key = asset_key(txd, ".txd");
    let cache_key = format!("{txd_key}|{key}");
    if let Some(cached) = app.vehicle_browser.texture_previews.get(&cache_key) {
        return cached.clone();
    }
    let decoded = vehicle_texture_rgba(app, name, txd).and_then(|(width, height, rgba)| {
        if width == 0 || height == 0 || width > u16::MAX as u32 || height > u16::MAX as u32 {
            return None;
        }
        if rgba.len() < (width * height * 4) as usize {
            return None;
        }
        let texture = Texture2D::from_rgba8(width as u16, height as u16, &rgba);
        texture.set_filter(FilterMode::Nearest);
        Some((texture, width, height))
    });
    app.vehicle_browser
        .texture_previews
        .insert(cache_key, decoded.clone());
    decoded
}

fn vehicle_part_label(part: &RenderPart) -> String {
    if part.texture_name.trim().is_empty() {
        format!("Material {}", part.material_index)
    } else {
        part.texture_name.clone()
    }
}

fn draw_vehicle_texture_preview_pane(app: &mut AppState, vehicle_txd: String) {
    let pv = vehicle_texture_preview_rect();
    ui_text(
        &app.ui_font,
        "Texture Preview",
        pv.x + 2.0,
        pv.y - 8.0,
        ui_dim(),
    );
    draw_rrect_bordered(
        pv.x,
        pv.y,
        pv.w,
        pv.h,
        7.0,
        1.0,
        Color::new(0.050, 0.058, 0.070, 1.0),
        ui_border(),
    );
    let selected = app.vehicle_browser.selected_part.and_then(|idx| {
        app.vehicle_browser
            .preview_mesh
            .as_ref()
            .and_then(|mesh| mesh.parts.get(idx))
            .map(|part| {
                (
                    idx,
                    part.texture_name.clone(),
                    part.texture_missing,
                    part.vertices,
                    part.material_index,
                    part.component,
                    vehicle_part_kind(part).to_string(),
                )
            })
    });
    if selected.is_some() {
        draw_small_button(&app.ui_font, vehicle_texture_copy_rect(), "Copy");
        draw_small_button(&app.ui_font, vehicle_texture_export_rect(), "Export");
        if selected_vehicle(app).is_some_and(|vehicle| !vehicle.readonly) {
            draw_small_button(&app.ui_font, vehicle_texture_replace_rect(), "Replace");
        }
    }
    let Some((idx, texture_name, texture_missing, vertices, material_index, component, kind)) =
        selected
    else {
        ui_text(
            &app.ui_font,
            "Select a part to preview its texture",
            pv.x + 12.0,
            pv.y + 26.0,
            ui_muted(),
        );
        return;
    };
    let component_name = app
        .vehicle_browser
        .preview_mesh
        .as_ref()
        .and_then(|mesh| mesh.components.get(component))
        .cloned()
        .unwrap_or_default();
    let thumb = Rect::new(pv.x + 12.0, pv.y + 12.0, pv.h - 24.0, pv.h - 24.0);
    draw_rrect(
        thumb.x,
        thumb.y,
        thumb.w,
        thumb.h,
        4.0,
        Color::new(0.030, 0.035, 0.045, 1.0),
    );
    let preview = vehicle_texture_preview(app, &texture_name, &vehicle_txd);
    if let Some((texture, width, height)) = preview.as_ref() {
        let scale = (thumb.w / *width as f32)
            .min(thumb.h / *height as f32)
            .min(4.0);
        let dw = *width as f32 * scale;
        let dh = *height as f32 * scale;
        draw_texture_ex(
            texture,
            thumb.x + (thumb.w - dw) * 0.5,
            thumb.y + (thumb.h - dh) * 0.5,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(dw, dh)),
                ..Default::default()
            },
        );
    } else {
        ui_text(
            &app.ui_font,
            "no texture",
            thumb.x + 12.0,
            thumb.y + thumb.h * 0.5 + 6.0,
            ui_muted(),
        );
    }
    let info_x = thumb.x + thumb.w + 14.0;
    let info_w = pv.x + pv.w - info_x - 10.0;
    let name_label = if texture_name.trim().is_empty() {
        format!("Material {material_index}")
    } else {
        texture_name.clone()
    };
    ui_text(
        &app.ui_font,
        &ellipsize_width(&name_label, 16, info_w),
        info_x,
        pv.y + 26.0,
        WHITE,
    );
    ui_text(
        &app.ui_font,
        &ellipsize_width(&format!("in {component_name}"), 16, info_w),
        info_x,
        pv.y + 48.0,
        LIGHTGRAY,
    );
    let dims = preview
        .as_ref()
        .map(|(_, width, height)| format!("{width} x {height}"))
        .unwrap_or_else(|| "not resolved".to_string());
    ui_text(&app.ui_font, &dims, info_x, pv.y + 70.0, LIGHTGRAY);
    ui_text(
        &app.ui_font,
        &format!("{kind}   {vertices} verts   part {idx}"),
        info_x,
        pv.y + 92.0,
        ui_dim(),
    );
    if texture_missing {
        ui_text(
            &app.ui_font,
            "missing from TXD",
            info_x,
            pv.y + 114.0,
            Color::new(0.92, 0.54, 0.38, 1.0),
        );
    }
}

pub(crate) fn draw_vehicle_photo_overlay(app: &AppState) {
    let photo_mode = app.vehicle_browser.photo_mode;
    text_button(
        &app.ui_font,
        vehicle_photo_mode_rect(app),
        if photo_mode {
            "Exit Photo Mode"
        } else {
            "Photo Mode"
        },
        photo_mode,
    );
    if !photo_mode {
        return;
    }
    let viewport = vehicle_preview_viewport_rect(app);
    let full_hint = "RMB free look  •  WASD/QE move  •  P or Esc exit";
    let compact_hint = "RMB look  •  WASD/QE move  •  P/Esc exit";
    let available_w = (viewport.w - 36.0).max(1.0);
    let desired_w = ui_text_width(full_hint, 14) + 28.0;
    let plaque_w = desired_w.min(available_w);
    let plaque = Rect::new(
        viewport.x + 18.0,
        viewport.y + viewport.h - 96.0,
        plaque_w,
        72.0,
    );
    draw_rrect(
        plaque.x + 2.0,
        plaque.y + 3.0,
        plaque.w,
        plaque.h,
        10.0,
        Color::new(0.0, 0.0, 0.0, 0.30),
    );
    draw_rrect_bordered(
        plaque.x,
        plaque.y,
        plaque.w,
        plaque.h,
        10.0,
        1.0,
        Color::new(0.045, 0.048, 0.054, 0.98),
        Color::new(0.28, 0.30, 0.34, 1.0),
    );
    draw_rrect(plaque.x + 1.0, plaque.y + 12.0, 3.0, 22.0, 1.5, ui_accent());
    if let Some(vehicle) = selected_vehicle(app) {
        let class = if vehicle.readonly {
            "SAN ANDREAS VEHICLE"
        } else {
            "CUSTOM VEHICLE"
        };
        ui_text_size(
            &app.ui_font,
            class,
            plaque.x + 14.0,
            plaque.y + 21.0,
            13,
            ui_accent(),
        );
        ui_text_size(
            &app.ui_font,
            &ellipsize_width(&vehicle.id, 18, plaque.w - 28.0),
            plaque.x + 14.0,
            plaque.y + 45.0,
            18,
            WHITE,
        );
    }
    let hint = if plaque.w >= desired_w - 1.0 {
        full_hint
    } else {
        compact_hint
    };
    ui_text_size(
        &app.ui_font,
        &ellipsize_width(hint, 14, plaque.w - 28.0),
        plaque.x + 14.0,
        plaque.y + 64.0,
        14,
        ui_muted(),
    );
}

pub(crate) fn draw_vehicle_details_panel(app: &mut AppState) {
    let panel = vehicle_details_rect();
    draw_panel_rect(&app.ui_font, panel, Some("Vehicle Components"));
    let Some(vehicle) = selected_vehicle(app) else {
        ui_text(
            &app.ui_font,
            "No vehicle selected",
            panel.x + 14.0,
            panel.y + 58.0,
            ui_muted(),
        );
        return;
    };
    let vehicle_txd = vehicle.txd.clone();
    ui_text(
        &app.ui_font,
        &ellipsize_width(
            &format!("{}   DFF {}   TXD {}", vehicle.id, vehicle.dff, vehicle.txd),
            16,
            panel.w - 28.0,
        ),
        panel.x + 14.0,
        panel.y + 56.0,
        LIGHTGRAY,
    );
    ui_text(
        &app.ui_font,
        &ellipsize_width(&vehicle.source, 16, panel.w - 28.0),
        panel.x + 14.0,
        panel.y + 76.0,
        ui_muted(),
    );
    text_button(
        &app.ui_font,
        vehicle_toggle_rect(0),
        "Body",
        app.vehicle_browser.show_body,
    );
    text_button(
        &app.ui_font,
        vehicle_toggle_rect(1),
        "Textures",
        app.vehicle_browser.show_textures,
    );
    text_button(
        &app.ui_font,
        vehicle_toggle_rect(2),
        "Collision Mesh",
        app.vehicle_browser.show_collision_mesh,
    );
    text_button(
        &app.ui_font,
        vehicle_toggle_rect(3),
        "Collision Volumes",
        app.vehicle_browser.show_collision_volumes,
    );
    text_button(
        &app.ui_font,
        vehicle_toggle_rect(4),
        "Lights",
        app.vehicle_browser.lights_on,
    );
    text_button(
        &app.ui_font,
        vehicle_toggle_rect(5),
        "Hide Damaged",
        app.vehicle_browser.hide_damaged,
    );
    text_button(
        &app.ui_font,
        vehicle_toggle_rect(6),
        "Hide VLO",
        app.vehicle_browser.hide_vlo,
    );
    text_button(
        &app.ui_font,
        vehicle_toggle_rect(7),
        "Reset Rotation",
        !app.vehicle_browser.component_rotations.is_empty(),
    );
    for (slot, (label, color)) in [
        ("Body A", app.vehicle_browser.body_color_a),
        ("Body B", app.vehicle_browser.body_color_b),
    ]
    .into_iter()
    .enumerate()
    {
        let rect = vehicle_body_color_rect(slot);
        draw_rrect_bordered(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            7.0,
            1.0,
            Color::new(0.13, 0.15, 0.18, 1.0),
            Color::new(0.25, 0.29, 0.34, 1.0),
        );
        let swatch = Rect::new(rect.x + 8.0, rect.y + 7.0, 20.0, 20.0);
        draw_rrect_bordered(
            swatch.x,
            swatch.y,
            swatch.w,
            swatch.h,
            4.0,
            1.0,
            Color::new(color.x, color.y, color.z, 1.0),
            Color::new(0.65, 0.72, 0.80, 1.0),
        );
        ui_text(&app.ui_font, label, rect.x + 36.0, rect.y + 22.0, WHITE);
        ui_text(
            &app.ui_font,
            if app.vehicle_browser.body_color_picker == Some(slot) {
                "v"
            } else {
                ">"
            },
            rect.x + rect.w - 18.0,
            rect.y + 22.0,
            ui_dim(),
        );
    }
    if app.vehicle_browser.body_color_picker.is_none() {
        text_button(
            &app.ui_font,
            vehicle_copy_collision_rect(),
            if app.vehicle_browser.collision_copy_rx.is_some() {
                "Replacing Collision..."
            } else {
                "Copy Collision From Another Vehicle..."
            },
            app.vehicle_browser.collision_copy_rx.is_some(),
        );
    }
    if let Some(slot) = app.vehicle_browser.body_color_picker {
        let color = vehicle_body_color(app, slot);
        for (channel, (label, value, fill)) in [
            ("R", color.x, Color::new(0.82, 0.24, 0.20, 1.0)),
            ("G", color.y, Color::new(0.24, 0.72, 0.30, 1.0)),
            ("B", color.z, Color::new(0.24, 0.48, 0.92, 1.0)),
        ]
        .into_iter()
        .enumerate()
        {
            let bar = vehicle_body_color_channel_rect(slot, channel);
            ui_text(&app.ui_font, label, bar.x, bar.y + 12.0, ui_dim());
            let track = Rect::new(bar.x + 18.0, bar.y + 3.0, bar.w - 18.0, 8.0);
            draw_rrect(
                track.x,
                track.y,
                track.w,
                track.h,
                4.0,
                Color::new(0.050, 0.058, 0.070, 1.0),
            );
            draw_rrect(
                track.x,
                track.y,
                track.w * value.clamp(0.0, 1.0),
                track.h,
                4.0,
                fill,
            );
            draw_circle(
                track.x + track.w * value.clamp(0.0, 1.0),
                track.y + track.h * 0.5,
                5.0,
                WHITE,
            );
        }
    }
    let list = vehicle_component_list_rect();
    ui_text(
        &app.ui_font,
        "Components",
        list.x + 2.0,
        list.y - 14.0,
        ui_dim(),
    );
    draw_small_button(&app.ui_font, vehicle_show_all_rect(), "All");
    draw_small_button(&app.ui_font, vehicle_hide_all_rect(), "None");
    draw_rrect_bordered(
        list.x,
        list.y,
        list.w,
        list.h,
        7.0,
        1.0,
        Color::new(0.050, 0.058, 0.070, 1.0),
        ui_border(),
    );
    if app.vehicle_browser.preview_mesh.is_none() {
        ui_text(
            &app.ui_font,
            "No DFF preview loaded",
            list.x + 10.0,
            list.y + 24.0,
            ui_muted(),
        );
        return;
    }
    let all_rows = vehicle_component_rows(app);
    let visible_rows = (list.h / VEHICLE_COMPONENT_ROW_H).floor().max(1.0) as usize;
    let max_scroll = all_rows.len().saturating_sub(visible_rows) as f32;
    app.vehicle_browser.component_scroll = app.vehicle_browser.component_scroll.min(max_scroll);
    let start = app.vehicle_browser.component_scroll as usize;
    let missing_color = Color::new(0.92, 0.54, 0.38, 1.0);
    for row in 0..visible_rows {
        let Some(entry) = all_rows.get(start + row).copied() else {
            break;
        };
        let Some(mesh) = app.vehicle_browser.preview_mesh.as_ref() else {
            break;
        };
        let y = list.y + row as f32 * VEHICLE_COMPONENT_ROW_H;
        let row_rect = Rect::new(
            list.x + 4.0,
            y + 2.0,
            list.w - 8.0,
            VEHICLE_COMPONENT_ROW_H - 4.0,
        );
        let text_y = y + 20.0;
        match entry {
            VehicleComponentRow::Component(component) => {
                let hidden = app.vehicle_browser.hidden_components.contains(&component);
                let selected = app.vehicle_browser.selected_component == Some(component);
                let collapsed = app
                    .vehicle_browser
                    .collapsed_components
                    .contains(&component);
                draw_rrect(
                    row_rect.x,
                    row_rect.y,
                    row_rect.w,
                    row_rect.h,
                    5.0,
                    if selected {
                        ui_surface_active()
                    } else {
                        ui_surface()
                    },
                );
                ui_text(
                    &app.ui_font,
                    if collapsed { ">" } else { "v" },
                    row_rect.x + 8.0,
                    text_y,
                    ui_dim(),
                );
                let name = mesh
                    .components
                    .get(component)
                    .cloned()
                    .unwrap_or_else(|| format!("geometry {component}"));
                let option_hidden = vehicle_component_hidden_by_options(app, &name);
                draw_visibility_dot(list.x + 26.0, y + 8.0, !hidden && !option_hidden);
                let part_count = mesh
                    .parts
                    .iter()
                    .filter(|part| part.component == component)
                    .count();
                ui_text(
                    &app.ui_font,
                    &ellipsize_width(&name, 16, row_rect.w - 140.0),
                    list.x + 48.0,
                    text_y,
                    if hidden || option_hidden {
                        ui_muted()
                    } else {
                        WHITE
                    },
                );
                ui_text(
                    &app.ui_font,
                    &format!("{part_count}"),
                    row_rect.x + row_rect.w - 34.0,
                    text_y,
                    ui_muted(),
                );
            }
            VehicleComponentRow::Part(part_idx) => {
                let Some(part) = mesh.parts.get(part_idx) else {
                    continue;
                };
                let part_hidden = app.vehicle_browser.hidden_parts.contains(&part_idx);
                let component_hidden = app
                    .vehicle_browser
                    .hidden_components
                    .contains(&part.component);
                let option_hidden = vehicle_component_hidden_by_options(
                    app,
                    vehicle_part_component_name(mesh, part),
                );
                let group_visible = vehicle_part_visible_by_group(app, part);
                let shown = !part_hidden && !component_hidden && !option_hidden && group_visible;
                let selected = app.vehicle_browser.selected_part == Some(part_idx);
                if selected {
                    draw_rrect(
                        row_rect.x,
                        row_rect.y,
                        row_rect.w,
                        row_rect.h,
                        5.0,
                        ui_surface_active(),
                    );
                } else if row % 2 == 0 {
                    draw_rrect(
                        row_rect.x,
                        row_rect.y,
                        row_rect.w,
                        row_rect.h,
                        5.0,
                        Color::new(0.086, 0.098, 0.116, 1.0),
                    );
                }
                draw_visibility_dot(list.x + 42.0, y + 8.0, !part_hidden);
                let label = vehicle_part_label(part);
                ui_text(
                    &app.ui_font,
                    &ellipsize_width(&label, 16, row_rect.w - 180.0),
                    list.x + 64.0,
                    text_y,
                    if shown { LIGHTGRAY } else { ui_muted() },
                );
                ui_text(
                    &app.ui_font,
                    &format!(
                        "{}{}",
                        part.vertices,
                        if part.texture_missing { " !" } else { "" }
                    ),
                    row_rect.x + row_rect.w - 66.0,
                    text_y,
                    if part.texture_missing {
                        missing_color
                    } else {
                        ui_dim()
                    },
                );
            }
        }
    }

    draw_vehicle_texture_preview_pane(app, vehicle_txd);

    if let Some(mesh) = app.vehicle_browser.preview_mesh.as_ref() {
        let components = mesh.components.len().max(1);
        let hidden =
            app.vehicle_browser.hidden_components.len() + app.vehicle_browser.hidden_parts.len();
        let total_verts: usize = mesh.parts.iter().map(|part| part.vertices).sum();
        ui_text(
            &app.ui_font,
            &format!(
                "{components} components   {} parts   {total_verts} verts   {hidden} hidden",
                mesh.parts.len()
            ),
            panel.x + 14.0,
            panel.y + panel.h - 16.0,
            LIGHTGRAY,
        );
    }
}

pub(crate) fn draw_vehicle_panel(app: &mut AppState) {
    let panel = vehicle_panel_rect();
    draw_panel_rect(&app.ui_font, panel, Some("Vehicles"));
    let count = filtered_vehicle_indices(app).len();
    let total = app.vehicles.len();
    ui_text(
        &app.ui_font,
        &format!(
            "{count} / {total} vehicles  •  {} dictionar{}",
            app.custom_vehicle_dictionaries.len(),
            if app.custom_vehicle_dictionaries.len() == 1 {
                "y"
            } else {
                "ies"
            }
        ),
        panel.x + 14.0,
        panel.y + 56.0,
        ui_muted(),
    );
    let search = vehicle_search_rect();
    draw_rrect_bordered(
        search.x,
        search.y,
        search.w,
        search.h,
        7.0,
        1.0,
        if app.vehicle_browser.search_active {
            ui_input_bg()
        } else {
            Color::new(0.050, 0.058, 0.070, 1.0)
        },
        if app.vehicle_browser.search_active {
            ui_accent()
        } else {
            ui_border()
        },
    );
    let text_x = search.x + 10.0;
    if app.vehicle_browser.search.is_empty() && !app.vehicle_browser.search_active {
        ui_text(
            &app.ui_font,
            "Search vehicle, DFF, TXD",
            text_x,
            search.y + 22.0,
            ui_muted(),
        );
    } else {
        let visible = ellipsize_width(&app.vehicle_browser.search, 16, search.w - 20.0);
        ui_text(&app.ui_font, &visible, text_x, search.y + 22.0, WHITE);
        if app.vehicle_browser.search_active && (get_time() * 2.0) as i32 % 2 == 0 {
            let caret_x = (text_x + ui_text_width(&visible, 16)).min(search.x + search.w - 8.0);
            draw_line(
                caret_x,
                search.y + 7.0,
                caret_x,
                search.y + search.h - 7.0,
                1.0,
                WHITE,
            );
        }
    }
    text_button(
        &app.ui_font,
        vehicle_filter_rect(0),
        "Default",
        app.vehicle_browser.show_default,
    );
    text_button(
        &app.ui_font,
        vehicle_filter_rect(1),
        "Custom",
        app.vehicle_browser.show_custom,
    );
    text_button(&app.ui_font, vehicle_action_rect(0), "COL Editor", false);
    text_button(&app.ui_font, vehicle_action_rect(1), "Export DFF", false);
    text_button(
        &app.ui_font,
        vehicle_action_rect(2),
        "Reload",
        app.vehicle_browser.reload_rx.is_some(),
    );
    text_button(&app.ui_font, vehicle_action_rect(3), "Focus", false);
    text_button(
        &app.ui_font,
        vehicle_action_rect(4),
        "Add Dictionary",
        app.vehicle_folder_picker_rx.is_some(),
    );
    text_button(
        &app.ui_font,
        vehicle_action_rect(5),
        "Manage",
        app.vehicle_browser.manage_dictionaries,
    );
    text_button(
        &app.ui_font,
        vehicle_action_rect(6),
        "Set Loader",
        app.vehicle_loader_picker_rx.is_some(),
    );
    text_button(
        &app.ui_font,
        vehicle_action_rect(7),
        if app.vehicle_build_rx.is_some() {
            "Loading..."
        } else {
            "Duplicate / Load"
        },
        app.vehicle_build_rx.is_some(),
    );

    let indices = filtered_vehicle_indices(app);
    let list = vehicle_list_rect();
    let rows = (list.h / VEHICLE_ROW_H).floor().max(1.0) as usize;
    let max_scroll = indices.len().saturating_sub(rows) as f32;
    app.vehicle_browser.scroll = app.vehicle_browser.scroll.min(max_scroll);
    draw_rrect_bordered(
        list.x,
        list.y,
        list.w,
        list.h,
        7.0,
        1.0,
        Color::new(0.050, 0.058, 0.070, 1.0),
        ui_border(),
    );
    let start = app.vehicle_browser.scroll as usize;
    for row in 0..rows {
        let Some(vehicle_idx) = indices.get(start + row).copied() else {
            break;
        };
        let vehicle = &app.vehicles[vehicle_idx];
        let y = list.y + row as f32 * VEHICLE_ROW_H;
        let row_rect = Rect::new(list.x + 4.0, y + 3.0, list.w - 8.0, VEHICLE_ROW_H - 6.0);
        let selected = vehicle_idx == app.vehicle_browser.selected;
        let missing_from_loader = !vehicle_loaded_in_resource(app, vehicle);
        if selected {
            draw_rrect(
                row_rect.x,
                row_rect.y,
                row_rect.w,
                row_rect.h,
                6.0,
                ui_surface_active(),
            );
        } else if row % 2 == 0 {
            draw_rrect(
                row_rect.x,
                row_rect.y,
                row_rect.w,
                row_rect.h,
                6.0,
                Color::new(0.086, 0.098, 0.116, 1.0),
            );
        }
        ui_text(
            &app.ui_font,
            &ellipsize_width(&vehicle.id, 16, 120.0),
            row_rect.x + 8.0,
            y + 23.0,
            if missing_from_loader {
                Color::new(0.96, 0.34, 0.32, 1.0)
            } else {
                WHITE
            },
        );
        ui_text(
            &app.ui_font,
            &ellipsize_width(&vehicle.dff, 16, 92.0),
            row_rect.x + 136.0,
            y + 23.0,
            LIGHTGRAY,
        );
        ui_text(
            &app.ui_font,
            if vehicle.readonly { "SA" } else { "custom" },
            row_rect.x + row_rect.w - 62.0,
            y + 23.0,
            if vehicle.readonly {
                ui_muted()
            } else if missing_from_loader {
                Color::new(0.96, 0.34, 0.32, 1.0)
            } else {
                ui_accent()
            },
        );
    }
    if let Some(vehicle) = selected_vehicle(app) {
        let info_y = panel.y + panel.h - 44.0;
        ui_text(
            &app.ui_font,
            &ellipsize_width(
                &format!("DFF {}   TXD {}", vehicle.dff, vehicle.txd),
                16,
                panel.w - 28.0,
            ),
            panel.x + 14.0,
            info_y,
            LIGHTGRAY,
        );
        ui_text(
            &app.ui_font,
            &ellipsize_width(
                &app.vehicle_browser
                    .embedded_collision
                    .as_ref()
                    .map(|collision| {
                        format!(
                            "Embedded COL   {} faces   {} volumes",
                            collision.faces.len(),
                            collision.spheres.len() + collision.boxes.len()
                        )
                    })
                    .unwrap_or_else(|| "Embedded COL not found".to_string()),
                16,
                panel.w - 28.0,
            ),
            panel.x + 14.0,
            info_y + 24.0,
            ui_muted(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vehicle_export_includes_only_referenced_missing_sa_generic_textures() {
        let referenced = vec![
            "bodypaint".to_string(),
            "VehicleGeneric256".to_string(),
            "vehiclelights128".to_string(),
        ];
        let vehicle_textures =
            HashSet::from(["bodypaint".to_string(), "vehiclelights128".to_string()]);
        let sa_generic_textures = HashSet::from([
            "vehiclegeneric256".to_string(),
            "vehiclelights128".to_string(),
            "vehiclegrunge256".to_string(),
        ]);

        assert_eq!(
            referenced_sa_generic_textures(&referenced, &vehicle_textures, &sa_generic_textures),
            HashSet::from(["vehiclegeneric256".to_string()])
        );
    }

    #[test]
    fn vehicle_loader_allocates_ids_and_round_trips_managed_categories() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let loader = env::temp_dir().join(format!(
            "eagle_vehicle_loader_{}_{}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(&loader).unwrap();
        fs::write(
            loader.join("vehicle_registry.xml"),
            "<vehicles nextCustomId=\"80000\">\n\
             <vehicle id=\"80000\" category=\"Tuner\"></vehicle>\n\
             <vehicle id=\"80002\" category=\"Race\"></vehicle>\n\
             </vehicles>\n",
        )
        .unwrap();

        assert_eq!(next_custom_vehicle_id(&loader), 80001);
        save_vehicle_categories(
            &loader,
            &[
                "Car".to_string(),
                "Tuner".to_string(),
                "Off-road".to_string(),
            ],
        )
        .unwrap();
        let categories = load_vehicle_categories(&loader);
        let used = load_used_vehicle_categories(&loader);

        assert!(categories.iter().any(|value| value == "Off-road"));
        assert!(categories.iter().any(|value| value == "Race"));
        assert!(used.contains("tuner"));
        assert!(used.contains("race"));
        fs::remove_dir_all(loader).unwrap();
    }

    #[test]
    fn custom_vehicle_txd_replacement_keeps_the_first_backup() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = env::temp_dir().join(format!(
            "eagle_vehicle_txd_replace_{}_{}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(&dir).unwrap();
        let txd = dir.join("custom.txd");
        fs::write(&txd, b"original").unwrap();

        let backup = write_custom_vehicle_txd(&txd, b"first replacement")
            .unwrap()
            .expect("first replacement should create a backup");
        assert_eq!(fs::read(&txd).unwrap(), b"first replacement");
        assert_eq!(fs::read(&backup).unwrap(), b"original");

        assert!(
            write_custom_vehicle_txd(&txd, b"second replacement")
                .unwrap()
                .is_none()
        );
        assert_eq!(fs::read(&txd).unwrap(), b"second replacement");
        assert_eq!(fs::read(&backup).unwrap(), b"original");
        assert!(!txd.with_extension("txd.eagle-tmp").exists());
        assert!(!txd.with_extension("txd.eagle-previous").exists());
        fs::remove_dir_all(dir).unwrap();
    }

    fn frame(name: &str, parent: i32, pos: V3) -> RawMeshFrame {
        RawMeshFrame {
            name: name.to_string(),
            parent,
            right: V3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            up: V3 {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            at: V3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
            pos,
        }
    }

    #[test]
    fn vehicle_wheel_duplication_uses_dummy_parent_not_component_name() {
        let mut raw = RawMesh {
            vertices: vec![
                V3 {
                    x: -1.0,
                    y: 2.0,
                    z: 0.0,
                },
                V3 {
                    x: -0.5,
                    y: 2.0,
                    z: 0.0,
                },
                V3 {
                    x: -1.0,
                    y: 2.5,
                    z: 0.0,
                },
            ],
            normals: vec![
                V3 {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                };
                3
            ],
            triangles: vec![Tri {
                a: 0,
                b: 1,
                c: 2,
                material: 0,
            }],
            components: vec![RawMeshComponent {
                name: "custom_rim_mesh".to_string(),
                frame_index: None,
                vertex_start: 0,
                vertex_end: 3,
                tri_start: 0,
                tri_end: 1,
                breakable: None,
            }],
            frames: vec![
                frame("chassis_dummy", -1, V3::default()),
                frame(
                    "wheel_rf_dummy",
                    0,
                    V3 {
                        x: -1.0,
                        y: 2.0,
                        z: 0.0,
                    },
                ),
                frame(
                    "custom_rim_mesh",
                    1,
                    V3 {
                        x: -1.0,
                        y: 2.0,
                        z: 0.0,
                    },
                ),
                frame(
                    "wheel_lf_dummy",
                    0,
                    V3 {
                        x: 1.0,
                        y: 2.0,
                        z: 0.0,
                    },
                ),
                frame(
                    "wheel_rr_dummy",
                    0,
                    V3 {
                        x: -1.0,
                        y: -2.0,
                        z: 0.0,
                    },
                ),
                frame(
                    "wheel_lr_dummy",
                    0,
                    V3 {
                        x: 1.0,
                        y: -2.0,
                        z: 0.0,
                    },
                ),
            ],
            ..RawMesh::default()
        };

        duplicate_vehicle_wheel_components(&mut raw);

        let names = raw
            .components
            .iter()
            .map(|component| component.name.as_str())
            .collect::<Vec<_>>();
        assert!(names.contains(&"wheel_rf_dummy"));
        assert!(names.contains(&"wheel_lf_dummy"));
        assert!(names.contains(&"wheel_rr_dummy"));
        assert!(names.contains(&"wheel_lr_dummy"));
        assert!(!names.contains(&"custom_rim_mesh"));
    }

    #[test]
    fn vehicle_wheel_duplication_ignores_steering_wheel_mesh() {
        let mut raw = RawMesh {
            vertices: vec![
                V3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                V3 {
                    x: 0.5,
                    y: 0.0,
                    z: 0.0,
                },
                V3 {
                    x: 0.0,
                    y: 0.5,
                    z: 0.0,
                },
            ],
            triangles: vec![Tri {
                a: 0,
                b: 1,
                c: 2,
                material: 0,
            }],
            components: vec![RawMeshComponent {
                name: "steering_wheel".to_string(),
                frame_index: None,
                vertex_start: 0,
                vertex_end: 3,
                tri_start: 0,
                tri_end: 1,
                breakable: None,
            }],
            frames: vec![
                frame("chassis_dummy", -1, V3::default()),
                frame("steering_wheel", 0, V3::default()),
                frame(
                    "wheel_rf_dummy",
                    0,
                    V3 {
                        x: -1.0,
                        y: 2.0,
                        z: 0.0,
                    },
                ),
                frame(
                    "wheel_lf_dummy",
                    0,
                    V3 {
                        x: 1.0,
                        y: 2.0,
                        z: 0.0,
                    },
                ),
            ],
            ..RawMesh::default()
        };

        duplicate_vehicle_wheel_components(&mut raw);

        assert_eq!(raw.components.len(), 1);
        assert_eq!(raw.components[0].name, "steering_wheel");
    }

    #[test]
    fn vehicle_preview_motion_uses_wheel_radii_and_suspension_handling() {
        let mut handling = HashMap::new();
        handling.insert("maxVelocity".to_string(), "210".to_string());
        handling.insert("engineAcceleration".to_string(), "36".to_string());
        handling.insert("suspensionUpperLimit".to_string(), "0.42".to_string());
        handling.insert("suspensionLowerLimit".to_string(), "-0.18".to_string());
        handling.insert("suspensionForceLevel".to_string(), "1.4".to_string());
        handling.insert("suspensionDamping".to_string(), "0.16".to_string());
        handling.insert("suspensionFrontRearBias".to_string(), "0.58".to_string());
        handling.insert("mass".to_string(), "1400".to_string());
        let vehicle = VehicleAsset {
            id: "motion_test".to_string(),
            model_id: None,
            handling_id: None,
            handling,
            dff: String::new(),
            txd: String::new(),
            col: String::new(),
            wheel_front: Some(0.72),
            wheel_rear: Some(0.96),
            source: String::new(),
            readonly: true,
            loose_dff_path: None,
            loose_txd_path: None,
        };

        let start = vehicle_preview_motion(&vehicle, 2.0);
        let later = vehicle_preview_motion(&vehicle, 2.01);
        let front_delta =
            (start.front_wheel_angle - later.front_wheel_angle).rem_euclid(std::f32::consts::TAU);
        let rear_delta =
            (start.rear_wheel_angle - later.rear_wheel_angle).rem_euclid(std::f32::consts::TAU);

        assert!(front_delta > rear_delta);
        for sample in (0..1_800).map(|step| step as f32 / 120.0) {
            let motion = vehicle_preview_motion(&vehicle, sample);
            assert!(motion.chassis_heave.is_finite());
            assert!(motion.chassis_pitch.is_finite());
            assert!(motion.front_wheel_travel.abs() <= 0.5);
            assert!(motion.rear_wheel_travel.abs() <= 0.5);
        }
    }

    #[test]
    fn vehicle_preview_motion_tolerates_invalid_handling_values() {
        let vehicle = VehicleAsset {
            id: "fallback_test".to_string(),
            model_id: None,
            handling_id: None,
            handling: HashMap::from([
                ("maxVelocity".to_string(), "not-a-number".to_string()),
                ("suspensionDamping".to_string(), "NaN".to_string()),
            ]),
            dff: String::new(),
            txd: String::new(),
            col: String::new(),
            wheel_front: Some(0.0),
            wheel_rear: Some(-0.8),
            source: String::new(),
            readonly: true,
            loose_dff_path: None,
            loose_txd_path: None,
        };

        let motion = vehicle_preview_motion(&vehicle, 123.0);
        assert!(motion.front_wheel_angle.is_finite());
        assert!(motion.rear_wheel_angle.is_finite());
        assert!(motion.chassis_heave.is_finite());
        assert!(motion.chassis_pitch.is_finite());
    }

    #[test]
    fn dff_wheel_width_patch_moves_only_wheel_dummy_track() {
        fn chunk(id: u32, payload: Vec<u8>) -> Vec<u8> {
            let mut out = Vec::new();
            out.extend_from_slice(&id.to_le_bytes());
            out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            out.extend_from_slice(&0x1803ffffu32.to_le_bytes());
            out.extend(payload);
            out
        }
        fn frame_list(name: &str, x: f32) -> Vec<u8> {
            let mut frame_struct = Vec::new();
            frame_struct.extend_from_slice(&1u32.to_le_bytes());
            for value in [1.0f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0] {
                frame_struct.extend_from_slice(&value.to_le_bytes());
            }
            frame_struct.extend_from_slice(&x.to_le_bytes());
            frame_struct.extend_from_slice(&0.0f32.to_le_bytes());
            frame_struct.extend_from_slice(&0.0f32.to_le_bytes());
            frame_struct.extend_from_slice(&(-1i32).to_le_bytes());
            frame_struct.extend_from_slice(&0u32.to_le_bytes());
            let mut raw_name = name.as_bytes().to_vec();
            raw_name.push(0);
            let extension = chunk(0x03, chunk(0x0253_f2fe, raw_name));
            let mut payload = chunk(0x01, frame_struct);
            payload.extend(extension);
            chunk(0x0e, payload)
        }

        let mut wheel = frame_list("wheel_lf_dummy", 1.25);
        patch_dff_wheel_track_width(&mut wheel, 1.2);
        let patched = f32::from_le_bytes(wheel[64..68].try_into().unwrap());
        assert!((patched - 1.5).abs() < 0.0001);

        let mut chassis = frame_list("chassis_dummy", 1.25);
        patch_dff_wheel_track_width(&mut chassis, 1.2);
        let untouched = f32::from_le_bytes(chassis[64..68].try_into().unwrap());
        assert!((untouched - 1.25).abs() < 0.0001);
    }

    #[test]
    fn embedded_vehicle_collision_replacement_updates_rw_container_sizes() {
        fn chunk(id: u32, payload: Vec<u8>) -> Vec<u8> {
            let mut out = Vec::new();
            out.extend_from_slice(&id.to_le_bytes());
            out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            out.extend_from_slice(&0x1803ffffu32.to_le_bytes());
            out.extend(payload);
            out
        }

        let original_col = minimal_col2_template("testcar");
        let collision_chunk = chunk(RW_COLLISION_MODEL_ID, original_col.clone());
        let extension = chunk(0x03, collision_chunk);
        let dff = chunk(0x10, extension);
        let old_clump_size = rd32(&dff, 4);
        let old_extension_size = rd32(&dff, 16);

        let mut replacement_col = original_col;
        replacement_col.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        let replaced =
            replace_embedded_vehicle_collision(&dff, &replacement_col).expect("replace collision");

        assert_eq!(rd32(&replaced, 4), old_clump_size + 8);
        assert_eq!(rd32(&replaced, 16), old_extension_size + 8);
        assert_eq!(rd32(&replaced, 28), replacement_col.len() as u32);
        assert_eq!(&replaced[36..36 + replacement_col.len()], replacement_col);
    }

    #[test]
    fn collision_copy_worker_replaces_complete_payload_and_retargets_model_name() {
        fn chunk(id: u32, payload: Vec<u8>) -> Vec<u8> {
            let mut out = Vec::new();
            out.extend_from_slice(&id.to_le_bytes());
            out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            out.extend_from_slice(&0x1803ffffu32.to_le_bytes());
            out.extend(payload);
            out
        }
        fn vehicle(id: &str, path: PathBuf) -> VehicleAsset {
            VehicleAsset {
                id: id.to_string(),
                model_id: None,
                handling_id: None,
                handling: HashMap::new(),
                dff: id.to_string(),
                txd: id.to_string(),
                col: id.to_string(),
                wheel_front: None,
                wheel_rear: None,
                source: "test".to_string(),
                readonly: false,
                loose_dff_path: Some(path),
                loose_txd_path: None,
            }
        }
        fn collision(name: &str, radius: f32) -> Vec<u8> {
            let spheres = vec![CollisionSphere {
                center: V3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.5,
                },
                radius,
                surface: CollisionSurface {
                    material: 0,
                    flags: 0,
                    brightness: 0,
                    light: 0,
                },
            }];
            let mesh = CollisionMesh {
                name: name.to_string(),
                spheres: spheres.clone(),
                boxes: Vec::new(),
                vertices: Vec::new(),
                faces: Vec::new(),
                bounds: collision_mesh_bounds(&[], &spheres, &[]),
                shadow_vertices: Vec::new(),
                shadow_faces: Vec::new(),
            };
            write_col_mesh_from_template(&minimal_col2_template(name), &mesh)
                .expect("write test collision")
        }

        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = env::temp_dir().join(format!(
            "eagle_vehicle_collision_copy_{}_{}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(&dir).unwrap();
        let source_path = dir.join("sourcecar.dff");
        let target_path = dir.join("targetcar.dff");
        let source_col = collision("sourcecar", 2.5);
        let target_col = collision("targetcar", 0.75);
        let source_dff = chunk(
            0x10,
            chunk(0x03, chunk(RW_COLLISION_MODEL_ID, source_col.clone())),
        );
        let target_dff = chunk(0x10, chunk(0x03, chunk(RW_COLLISION_MODEL_ID, target_col)));
        fs::write(&source_path, source_dff).unwrap();
        fs::write(&target_path, &target_dff).unwrap();

        let result = copy_vehicle_collision_worker(
            vehicle("targetcar", target_path.clone()),
            vehicle("sourcecar", source_path),
            dir.clone(),
            dir.clone(),
        );
        let output = result.result.expect("copy collision");
        let copied_dff = fs::read(&target_path).unwrap();
        let (_, copied_col) =
            parse_embedded_vehicle_collision(&copied_dff, "targetcar").expect("copied collision");
        let mut expected_col = source_col;
        set_col_model_names_from_entry(&mut expected_col, "targetcar");

        assert_eq!(copied_col, expected_col);
        assert_eq!(output.collision_bytes, expected_col);
        assert_eq!(output.dff_bytes, copied_dff);
        assert_eq!(
            fs::read(output.backup.expect("target backup")).unwrap(),
            target_dff
        );
        assert!(!target_path.with_extension("dff.eagle-tmp").exists());
        assert!(!target_path.with_extension("dff.eagle-previous").exists());
        fs::remove_dir_all(dir).unwrap();
    }
}
