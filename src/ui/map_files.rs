use super::super::*;
use crate::resource::mta_maps::*;

pub(crate) struct MapDialog {
    path: String,
    cursor: usize,
    scroll: usize,
}
fn dialog_rect() -> Rect {
    let w = 660.0_f32.min(screen_width() - 40.0);
    let h = 470.0_f32.min(screen_height() - 40.0);
    Rect::new(
        (screen_width() - w) / 2.0,
        (screen_height() - h) / 2.0,
        w,
        h,
    )
}
fn path_rect() -> Rect {
    let r = dialog_rect();
    Rect::new(r.x + 20.0, r.y + 82.0, r.w - 180.0, 32.0)
}
fn register_rect() -> Rect {
    let r = dialog_rect();
    Rect::new(r.x + r.w - 148.0, r.y + 82.0, 128.0, 32.0)
}
fn close_rect() -> Rect {
    let r = dialog_rect();
    Rect::new(r.x + r.w - 110.0, r.y + r.h - 44.0, 90.0, 28.0)
}
fn row_rect(slot: usize) -> Rect {
    let r = dialog_rect();
    Rect::new(
        r.x + 20.0,
        r.y + 151.0 + slot as f32 * 36.0,
        r.w - 40.0,
        32.0,
    )
}
fn visible_rows() -> usize {
    ((dialog_rect().h - 210.0) / 36.0).floor().max(1.0) as usize
}
fn destinations(app: &AppState) -> Vec<String> {
    let mut list = app.zones.clone();
    if !list.iter().any(|z| map_path(z).is_none()) {
        list.insert(0, "default".into());
    }
    for doc in &app.map_documents {
        let z = map_zone(&doc.path);
        if !list.contains(&z) {
            list.push(z);
        }
    }
    list
}
pub(crate) fn map_files_button_rect(app: &AppState) -> Rect {
    Rect::new(
        screen_width() - right_panel_width() + 26.0,
        TOP_H + 48.0 - app.properties_scroll,
        right_panel_width() - 52.0,
        28.0,
    )
}
pub(crate) fn element_destination_rect(app: &AppState) -> Rect {
    let l = element_panel_layout(app);
    Rect::new(
        l.content.x + 6.0,
        l.info_top + 46.0,
        right_panel_width() - 52.0,
        24.0,
    )
}
pub(crate) fn open_map_files_dialog(app: &mut AppState) {
    app.map_dialog = Some(MapDialog {
        path: String::new(),
        cursor: 0,
        scroll: 0,
    });
    drain_text_input();
}
pub(crate) fn register_map_file(app: &mut AppState, path: &str) -> Result<(), String> {
    let path = path.trim();
    if !safe_map_path(path) {
        return Err("Enter a project-relative .map path, such as maps/main.map".into());
    }
    let zone = map_zone(path);
    if app.map_documents.iter().any(|d| d.path == path) {
        app.placement_destination = Some(zone);
        return Ok(());
    }
    let doc = if app.root.join(path).exists() {
        read_document(&app.root, path)?
    } else {
        crate::resource::mta_maps::new_document(&app.root, path)?
    };
    let before = world_history_snapshot(app);
    let new_placements = placements(&doc, &app.definitions);
    for p in &new_placements {
        if let Some(d) = app.definitions.get(&p.id) {
            let entry = AssetBrowserEntry {
                id: p.id.clone(),
                dff: p.dff.clone(),
                txd: d.attrs.get("txd").cloned().unwrap_or_default(),
                zone: d.zone.clone(),
                category: String::new(),
                readonly: app.readonly_definition_ids.contains(&p.id),
                lod: false,
            };
            ensure_asset_browser_mesh_loaded(app, &entry);
        }
    }
    app.placements.extend(new_placements);
    app.element_states
        .resize(app.placements.len(), ElementState::default());
    app.outliner_labels.resize(app.placements.len(), None);
    app.map_documents.push(doc);
    if !app.zones.contains(&zone) {
        app.zones.push(zone.clone());
    }
    app.placement_destination = Some(zone);
    rebuild_outliner_filter(app);
    rebuild_render_cells(app);
    commit_world_history(app, "Register Map File", before);
    Ok(())
}
pub(crate) fn add_eagle_zone(app: &mut AppState, name: &str) -> Result<(), String> {
    let name = name.trim();
    if !is_safe_zone_name(name) || map_path(name).is_some() {
        return Err("Use a plain zone name without folders, such as downtown".into());
    }
    let before = world_history_snapshot(app);
    if !app.zones.iter().any(|z| z == name) {
        app.zones.push(name.into());
    }
    app.placement_destination = Some(name.into());
    commit_world_history(app, "Add Eagle Zone", before);
    Ok(())
}

pub(crate) fn assign_selected_destination(app: &mut AppState, zone: &str) -> Result<(), String> {
    if map_path(zone).is_none() && !is_safe_zone_name(zone) {
        return Err("Invalid Eagle zone name".into());
    }
    if let Some(path) = map_path(zone) {
        if !app.map_documents.iter().any(|d| d.path == path) {
            return Err("Register the map before assigning objects".into());
        }
    }
    if selection_has_default_world(app) {
        return Err("Default SA world elements are locked; use Turn into placement".into());
    }
    let indices = selected_live_indices(app);
    if indices.is_empty() {
        return Err("Select elements to change their zone or map file".into());
    }
    if map_path(zone).is_some()
        && indices.iter().any(|i| {
            app.placements
                .get(*i)
                .is_some_and(|p| p.id.parse::<u32>().is_err())
        })
    {
        return Err(
            "Standard MTA maps require GTA:SA model IDs; custom models belong in Eagle zones"
                .into(),
        );
    }
    let before = world_history_snapshot(app);
    for i in indices {
        if let Some(p) = app.placements.get_mut(i) {
            // Preserve an existing MTA element ID, but do not turn Eagle's model ID into one.
            if map_path(zone).is_some() && map_path(&p.zone).is_none() {
                p.attrs.remove("id");
            }
            p.zone = zone.into();
            if p.attrs.contains_key("zone") {
                p.attrs.insert("zone".into(), zone.into());
            }
            sync_placement_attrs(p);
        }
    }
    if !app.zones.iter().any(|z| z == zone) {
        app.zones.push(zone.into());
    }
    rebuild_outliner_filter(app);
    rebuild_render_cells(app);
    commit_world_history(app, "Change Placement File", before);
    Ok(())
}
pub(crate) fn update_map_files_input(app: &mut AppState, mouse: Vec2) -> bool {
    if app.map_dialog.is_none() {
        if is_mouse_button_pressed(MouseButton::Left)
            && ((app.active_tab == AppTab::Scene && map_files_button_rect(app).contains(mouse))
                || (app.active_tab == AppTab::Preview
                    && app.properties_tab == PropertiesTab::Element
                    && app.placements.get(app.selected).is_some()
                    && !selection_has_default_world(app)
                    && element_destination_rect(app).contains(mouse)))
        {
            open_map_files_dialog(app);
            return true;
        }
        return false;
    }
    if is_key_pressed(KeyCode::Escape)
        || (is_mouse_button_pressed(MouseButton::Left) && close_rect().contains(mouse))
    {
        app.map_dialog = None;
        return true;
    }
    let list = destinations(app);
    let count = visible_rows();
    let (_, wheel) = mouse_wheel();
    if let Some(d) = app.map_dialog.as_mut() {
        d.scroll = ((d.scroll as i32 - wheel as i32).max(0) as usize)
            .min(list.len().saturating_sub(count));
    }
    if is_mouse_button_pressed(MouseButton::Left) {
        let scroll = app.map_dialog.as_ref().unwrap().scroll;
        for (slot, zone) in list.iter().skip(scroll).take(count).enumerate() {
            let r = row_rect(slot);
            let assign = Rect::new(r.x + r.w - 106.0, r.y, 106.0, r.h);
            if assign.contains(mouse) {
                match assign_selected_destination(app, zone) {
                    Ok(()) => {
                        app.status_message =
                            format!("Selection belongs to {}", destination_label(zone))
                    }
                    Err(e) => app.status_message = e,
                }
                return true;
            }
            if r.contains(mouse) {
                app.placement_destination = Some(zone.clone());
                app.status_message = format!("New objects go to {}", destination_label(zone));
                return true;
            }
        }
    }
    if is_key_pressed(KeyCode::Enter)
        || (is_mouse_button_pressed(MouseButton::Left) && register_rect().contains(mouse))
    {
        let path = app.map_dialog.as_ref().unwrap().path.clone();
        let is_map = path.trim().to_ascii_lowercase().ends_with(".map");
        let result = if is_map {
            register_map_file(app, &path)
        } else {
            add_eagle_zone(app, &path)
        };
        match result {
            Ok(()) => {
                app.status_message = if is_map {
                    format!("Registered {path}; Save to write the project")
                } else {
                    format!("Zone {path} is ready; click Assign to move selected elements")
                };
                app.map_dialog.as_mut().unwrap().path.clear();
                app.map_dialog.as_mut().unwrap().cursor = 0;
            }
            Err(e) => app.status_message = e,
        }
        return true;
    }
    let d = app.map_dialog.as_mut().unwrap();
    d.cursor = clamp_char_boundary(&d.path, d.cursor);
    if is_key_pressed(KeyCode::Left) {
        d.cursor = prev_char_boundary(&d.path, d.cursor);
    }
    if is_key_pressed(KeyCode::Right) {
        d.cursor = next_char_boundary(&d.path, d.cursor);
    }
    if is_key_pressed(KeyCode::Backspace) && d.cursor > 0 {
        let prev = prev_char_boundary(&d.path, d.cursor);
        d.path.replace_range(prev..d.cursor, "");
        d.cursor = prev;
    }
    if is_key_pressed(KeyCode::Delete) && d.cursor < d.path.len() {
        let next = next_char_boundary(&d.path, d.cursor);
        d.path.replace_range(d.cursor..next, "");
    }
    while let Some(ch) = get_char_pressed() {
        if !ch.is_control() && d.path.len() < 240 {
            d.path.insert(d.cursor, ch);
            d.cursor += ch.len_utf8();
        }
    }
    true
}
pub(crate) fn draw_map_files_dialog(app: &AppState) {
    let Some(d) = app.map_dialog.as_ref() else {
        return;
    };
    draw_modal_backdrop();
    let r = dialog_rect();
    draw_panel_rect(&app.ui_font, r, Some("Placement files"));
    ui_text(
        &app.ui_font,
        "Enter a zone name (downtown) or a .map path (maps/main.map)",
        r.x + 20.0,
        r.y + 64.0,
        LIGHTGRAY,
    );
    let input = path_rect();
    draw_rectangle(
        input.x,
        input.y,
        input.w,
        input.h,
        Color::new(0.05, 0.06, 0.08, 1.0),
    );
    ui_text(
        &app.ui_font,
        &ellipsize_width(&d.path, 16, input.w - 20.0),
        input.x + 10.0,
        input.y + 22.0,
        WHITE,
    );
    text_button(&app.ui_font, register_rect(), "Add / Register", false);
    ui_text(
        &app.ui_font,
        "Click a file for new objects; Assign moves the selected objects.",
        r.x + 20.0,
        r.y + 138.0,
        ui_dim(),
    );
    for (slot, zone) in destinations(app)
        .iter()
        .skip(d.scroll)
        .take(visible_rows())
        .enumerate()
    {
        let row = row_rect(slot);
        let selected = app.placement_destination.as_ref() == Some(zone);
        let name = Rect::new(row.x, row.y, row.w - 112.0, row.h);
        text_button(
            &app.ui_font,
            name,
            &ellipsize_width(&destination_label(zone), 16, name.w - 20.0),
            selected,
        );
        text_button(
            &app.ui_font,
            Rect::new(row.x + row.w - 106.0, row.y, 106.0, row.h),
            "Assign",
            false,
        );
    }
    ui_text(
        &app.ui_font,
        "Definitions follow the zone with the most instances. Scroll for more.",
        r.x + 20.0,
        r.y + r.h - 58.0,
        ui_dim(),
    );
    text_button(&app.ui_font, close_rect(), "Close", false);
}
