use super::super::*;

pub(crate) const LIGHT_LIST_VISIBLE_ROWS: usize = 7;
const LIGHT_TEMPERATURE_MIN: f32 = 1_000.0;
const LIGHT_TEMPERATURE_MAX: f32 = 40_000.0;

pub(crate) fn light_button_rect(app: &AppState, slot: usize) -> Rect {
    let panel_x = screen_width() - RIGHT_PANEL_W;
    let (x, w) = match slot {
        0 => (panel_x + 24.0, 80.0),
        1 => (panel_x + 112.0, 80.0),
        _ => (panel_x + 200.0, 126.0),
    };
    Rect::new(x, TOP_H + 52.0 - app.properties_scroll, w, 28.0)
}

pub(crate) fn light_list_rect(app: &AppState) -> Rect {
    let x = screen_width() - RIGHT_PANEL_W + 22.0;
    let y = TOP_H + 116.0 - app.properties_scroll;
    let row_h = 24.0;
    Rect::new(
        x,
        y,
        RIGHT_PANEL_W - 44.0,
        row_h * LIGHT_LIST_VISIBLE_ROWS as f32,
    )
}

pub(crate) fn light_row_at(app: &AppState, mouse: Vec2) -> Option<usize> {
    let rect = light_list_rect(app);
    let row_h = 24.0;
    if rect.contains(mouse) {
        let start = (app.light_list_scroll.floor() as usize)
            .min(app.lights.len().saturating_sub(LIGHT_LIST_VISIBLE_ROWS));
        Some(start + ((mouse.y - rect.y) / row_h).floor() as usize)
    } else {
        None
    }
}

pub(crate) fn clamp_light_list_scroll(app: &mut AppState) {
    let max_scroll = app.lights.len().saturating_sub(LIGHT_LIST_VISIBLE_ROWS) as f32;
    app.light_list_scroll = app.light_list_scroll.clamp(0.0, max_scroll);
}

pub(crate) fn scroll_light_list(app: &mut AppState, wheel: f32) {
    app.light_list_scroll = (app.light_list_scroll - wheel * 3.0).max(0.0);
    clamp_light_list_scroll(app);
}

fn reveal_selected_light(app: &mut AppState) {
    if app.selected_light < app.light_list_scroll as usize {
        app.light_list_scroll = app.selected_light as f32;
    } else {
        let end = app.light_list_scroll as usize + LIGHT_LIST_VISIBLE_ROWS;
        if app.selected_light >= end {
            app.light_list_scroll =
                (app.selected_light + 1).saturating_sub(LIGHT_LIST_VISIBLE_ROWS) as f32;
        }
    }
    clamp_light_list_scroll(app);
}

pub(crate) fn light_color_bar_rect(app: &AppState, channel: usize) -> Rect {
    let x = screen_width() - RIGHT_PANEL_W + 58.0;
    Rect::new(
        x,
        TOP_H + 742.0 - app.properties_scroll + channel as f32 * 24.0,
        212.0,
        16.0,
    )
}

pub(crate) fn light_color_swatch_rect(app: &AppState) -> Rect {
    Rect::new(
        screen_width() - RIGHT_PANEL_W + 286.0,
        TOP_H + 742.0 - app.properties_scroll,
        38.0,
        64.0,
    )
}

pub(crate) fn light_use_temperature_rect(app: &AppState) -> Rect {
    Rect::new(
        screen_width() - RIGHT_PANEL_W + 18.0,
        TOP_H + 704.0 - app.properties_scroll,
        170.0,
        22.0,
    )
}

pub(crate) fn light_casts_shadow_rect(app: &AppState) -> Rect {
    Rect::new(
        screen_width() - RIGHT_PANEL_W + 206.0,
        TOP_H + 704.0 - app.properties_scroll,
        150.0,
        22.0,
    )
}

pub(crate) fn light_kind_options() -> [LightKind; 4] {
    [
        LightKind::Point,
        LightKind::Directional,
        LightKind::Spot,
        LightKind::Ambient,
    ]
}

pub(crate) fn light_kind_option_rect(app: &AppState, row: usize) -> Rect {
    let base = inspector_field_rect(app, InspectorField::LightKind);
    Rect::new(
        base.x,
        base.y + base.h + 4.0 + row as f32 * 28.0,
        base.w,
        28.0,
    )
}

pub(crate) fn light_profile_options() -> [LightProfile; 3] {
    [LightProfile::Day, LightProfile::Night, LightProfile::Both]
}

pub(crate) fn light_profile_option_rect(app: &AppState, row: usize) -> Rect {
    let base = inspector_field_rect(app, InspectorField::LightProfile);
    Rect::new(
        base.x,
        base.y + base.h + 4.0 + row as f32 * 28.0,
        base.w,
        28.0,
    )
}

/// Every rect and label position in the Vertex Lighting panel, computed in one
/// place with a running y-cursor so sections never overlap.
pub(crate) struct BakePanelLayout {
    pub panel: Rect,
    pub bake_header_y: f32,
    pub bake_button: Rect,
    pub clear_button: Rect,
    pub info_y: f32,
    pub backend: [Rect; 2],
    pub preset: [Rect; 3],
    pub light_mode: [Rect; 3],
    pub scope: [Rect; 2],
    pub face_emitters: Rect,
    pub regenerate_coronas: Rect,
    pub shadow_samples: Rect,
    pub shadow_chunks: Rect,
    pub bounces: Rect,
    pub bounce_strength: Rect,
    pub bounce_maximum: Rect,
    pub exposure: Rect,
    pub ambient_bump: Rect,
    pub ambient_bump_button: Rect,
    pub shadow_softness: Rect,
    pub ao_header_y: f32,
    pub ao_button: Rect,
    pub ao_hint: (f32, f32),
    pub ao_samples: Rect,
    pub ao_radius: Rect,
    pub ao_strength: Rect,
    pub editor_header_y: f32,
    pub brush_toggle: Rect,
    pub tools: [Rect; 2],
    pub blend: Rect,
    pub copy: Rect,
    pub paste: Rect,
    pub import: Rect,
    pub variant_merge: Rect,
    pub variant_tolerance: Rect,
    pub rgb_label_y: f32,
    pub color_bars: [Rect; 3],
    pub color_swatch: Rect,
    pub paint_temperature: Rect,
    pub paint_radius: Rect,
    pub paint_strength: Rect,
    pub progress: Rect,
    pub footer_y: f32,
}

pub(crate) fn bake_panel_layout(app: &AppState) -> BakePanelLayout {
    const BTN_H: f32 = 30.0;
    const BTN_ROW: f32 = 36.0;
    const INPUT_LABEL: f32 = 18.0;
    const INPUT_BOX: f32 = 30.0;
    const INPUT_GAP: f32 = 2.0;
    const HEADER_GAP: f32 = 8.0;
    const HEADER_H: f32 = 22.0;

    let panel_x = screen_width() - RIGHT_PANEL_W + 12.0;
    let panel_y = TOP_H + 12.0;
    let panel_w = RIGHT_PANEL_W - 24.0;
    let left = panel_x + 14.0;
    let two_col = |cy: f32| {
        (
            Rect::new(left, cy, 150.0, INPUT_BOX),
            Rect::new(left + 168.0, cy, 150.0, INPUT_BOX),
        )
    };
    let three_col = |cy: f32| {
        [
            Rect::new(left, cy, 108.0, INPUT_BOX),
            Rect::new(left + 124.0, cy, 108.0, INPUT_BOX),
            Rect::new(left + 248.0, cy, 108.0, INPUT_BOX),
        ]
    };

    let mut cy = panel_y + 34.0 - app.properties_scroll;

    // --- Bake section ---
    let bake_header_y = cy + 14.0;
    cy += HEADER_H;
    let bake_button = Rect::new(left, cy, 142.0, BTN_H);
    let clear_button = Rect::new(left + 154.0, cy, 142.0, BTN_H);
    cy += BTN_ROW;
    let info_y = cy + 12.0;
    cy += 20.0;
    let backend = [
        Rect::new(left, cy, 142.0, BTN_H),
        Rect::new(left + 154.0, cy, 142.0, BTN_H),
    ];
    cy += BTN_ROW;
    let preset = [
        Rect::new(left, cy, 108.0, BTN_H),
        Rect::new(left + 124.0, cy, 108.0, BTN_H),
        Rect::new(left + 248.0, cy, 108.0, BTN_H),
    ];
    cy += BTN_ROW;
    let light_mode = [
        Rect::new(left, cy, 108.0, BTN_H),
        Rect::new(left + 124.0, cy, 108.0, BTN_H),
        Rect::new(left + 248.0, cy, 108.0, BTN_H),
    ];
    cy += BTN_ROW;
    let scope = [
        Rect::new(left, cy, 142.0, BTN_H),
        Rect::new(left + 154.0, cy, 142.0, BTN_H),
    ];
    cy += BTN_ROW;
    let face_emitters = Rect::new(left, cy, 220.0, BTN_H);
    cy += BTN_ROW;
    let regenerate_coronas = Rect::new(left, cy, panel_w - 28.0, BTN_H);
    cy += BTN_ROW;
    cy += INPUT_LABEL;
    let (shadow_samples, shadow_chunks) = two_col(cy);
    cy += INPUT_BOX + INPUT_GAP;
    cy += INPUT_LABEL;
    let (bounces, bounce_strength) = two_col(cy);
    cy += INPUT_BOX + INPUT_GAP;
    cy += INPUT_LABEL;
    let (shadow_softness, bounce_maximum) = two_col(cy);
    cy += INPUT_BOX + INPUT_GAP;
    cy += INPUT_LABEL;
    let (exposure, ambient_bump) = two_col(cy);
    cy += INPUT_BOX + INPUT_GAP;
    let ambient_bump_button = Rect::new(left, cy, panel_w - 28.0, BTN_H);
    cy += BTN_ROW;

    // --- Ambient Occlusion section ---
    cy += HEADER_GAP;
    let ao_header_y = cy + 14.0;
    cy += HEADER_H;
    let ao_button = Rect::new(left, cy, 142.0, BTN_H);
    let ao_hint = (left + 154.0, cy + 20.0);
    cy += BTN_ROW;
    cy += INPUT_LABEL;
    let [ao_samples, ao_radius, ao_strength] = three_col(cy);
    cy += INPUT_BOX + INPUT_GAP;

    // --- Paint Editor section ---
    cy += HEADER_GAP;
    let editor_header_y = cy + 14.0;
    cy += HEADER_H;
    let brush_toggle = Rect::new(left, cy, 81.0, BTN_H);
    let tools = [
        Rect::new(left + 89.0, cy, 81.0, BTN_H),
        Rect::new(left + 178.0, cy, 81.0, BTN_H),
    ];
    let blend = Rect::new(left + 267.0, cy, 81.0, BTN_H);
    cy += BTN_ROW;
    let copy = Rect::new(left, cy, 94.0, BTN_H);
    let paste = Rect::new(left + 102.0, cy, 94.0, BTN_H);
    let import = Rect::new(left + 204.0, cy, 98.0, BTN_H);
    cy += BTN_ROW;
    let variant_merge = Rect::new(left, cy, 202.0, BTN_H);
    let variant_tolerance = Rect::new(left + 234.0, cy, 68.0, INPUT_BOX);
    cy += BTN_ROW;
    let rgb_label_y = cy + 12.0;
    let bars_y = cy + 20.0;
    let color_bars = [
        Rect::new(left + 32.0, bars_y, 206.0, 16.0),
        Rect::new(left + 32.0, bars_y + 24.0, 206.0, 16.0),
        Rect::new(left + 32.0, bars_y + 48.0, 206.0, 16.0),
    ];
    let color_swatch = Rect::new(left + 254.0, bars_y, 38.0, 64.0);
    cy += 20.0 + 72.0 + 8.0;
    cy += INPUT_LABEL;
    let [paint_temperature, paint_radius, paint_strength] = three_col(cy);
    cy += INPUT_BOX + INPUT_GAP;

    // --- Progress + footer ---
    cy += HEADER_GAP;
    let progress = Rect::new(left, cy, panel_w - 28.0, 16.0);
    cy += 40.0;
    let footer_y = cy + 10.0;

    BakePanelLayout {
        panel: Rect::new(
            panel_x,
            panel_y,
            panel_w,
            (screen_height() - STATUS_H - panel_y - 12.0).max(1.0),
        ),
        bake_header_y,
        bake_button,
        clear_button,
        info_y,
        backend,
        preset,
        light_mode,
        scope,
        face_emitters,
        regenerate_coronas,
        shadow_samples,
        shadow_chunks,
        bounces,
        bounce_strength,
        bounce_maximum,
        exposure,
        ambient_bump,
        ambient_bump_button,
        shadow_softness,
        ao_header_y,
        ao_button,
        ao_hint,
        ao_samples,
        ao_radius,
        ao_strength,
        editor_header_y,
        brush_toggle,
        tools,
        blend,
        copy,
        paste,
        import,
        variant_merge,
        variant_tolerance,
        rgb_label_y,
        color_bars,
        color_swatch,
        paint_temperature,
        paint_radius,
        paint_strength,
        progress,
        footer_y,
    }
}

pub(crate) fn set_bake_quality(app: &mut AppState, preset: BakeQualityPreset) {
    let backend = app.bake_settings.backend;
    let light_mode = app.bake_settings.light_mode;
    let scope = app.bake_settings.scope;
    let face_emitters_enabled = app.bake_settings.face_emitters_enabled;
    let exposure = app.bake_settings.exposure;
    let ambient_bump = app.bake_settings.ambient_bump;
    let day_night_merge_tolerance = app.bake_settings.day_night_merge_tolerance;
    app.bake_settings = bake_quality_settings(preset);
    app.bake_settings.backend = backend;
    app.bake_settings.light_mode = light_mode;
    app.bake_settings.scope = scope;
    app.bake_settings.face_emitters_enabled = face_emitters_enabled;
    app.bake_settings.exposure = exposure;
    app.bake_settings.ambient_bump = ambient_bump;
    app.bake_settings.day_night_merge_tolerance = day_night_merge_tolerance;
    save_bake_settings_preference(app.bake_settings);
    app.status_message = format!("Bake quality {}", bake_quality_label(preset));
}

pub(crate) fn set_bake_backend(app: &mut AppState, backend: BakeBackend) {
    app.bake_settings.backend = backend;
    save_bake_settings_preference(app.bake_settings);
    app.status_message = format!("Bake backend {}", bake_backend_label(backend));
}

pub(crate) fn set_bake_light_mode(app: &mut AppState, mode: BakeLightMode) {
    app.bake_settings.light_mode = mode;
    save_bake_settings_preference(app.bake_settings);
    apply_bake_light_mode_to_meshes(&mut app.meshes, mode);
    rebuild_mesh_part_lists(&mut app.meshes);
    rebuild_render_cells(app);
    app.status_message = format!("Bake target {}", bake_light_mode_label(mode));
}

pub(crate) fn set_bake_scope(app: &mut AppState, scope: BakeScope) {
    app.bake_settings.scope = scope;
    save_bake_settings_preference(app.bake_settings);
    app.status_message = format!("Bake scope {}", bake_scope_label(scope));
}

pub(crate) fn update_light_color_picker(app: &mut AppState, mouse: Vec2) -> bool {
    if !is_mouse_button_down(MouseButton::Left) {
        if let Some(before) = app.light_color_drag_before.take() {
            commit_light_history(app, "Edit Light Color", before);
            return true;
        }
    }
    if app.active_tab != AppTab::Lights || app.selected_light >= app.lights.len() {
        app.light_color_drag_before = None;
        return false;
    }
    if !inspector_panel_content_rect().contains(mouse) {
        return false;
    }
    if app
        .lights
        .get(app.selected_light)
        .is_some_and(|light| light.use_temperature)
    {
        return false;
    }
    if !is_mouse_button_down(MouseButton::Left) {
        return false;
    }
    for channel in 0..3 {
        let rect = light_color_bar_rect(app, channel);
        if rect.contains(mouse) {
            if app.light_color_drag_before.is_none() {
                app.light_color_drag_before = Some(light_history_snapshot(app));
            }
            let value = ((mouse.x - rect.x) / rect.w).clamp(0.0, 1.0);
            if let Some(light) = app.lights.get_mut(app.selected_light) {
                match channel {
                    0 => light.color.x = value,
                    1 => light.color.y = value,
                    _ => light.color.z = value,
                }
                mark_lights_changed(app);
            }
            return true;
        }
    }
    false
}

fn light_temperature_from_fraction(fraction: f32) -> f32 {
    ui_temperature_from_fraction(fraction, LIGHT_TEMPERATURE_MIN, LIGHT_TEMPERATURE_MAX)
}

pub(crate) fn update_light_temperature_slider(app: &mut AppState, mouse: Vec2) -> bool {
    if !is_mouse_button_down(MouseButton::Left) {
        if let Some(before) = app.light_temperature_drag_before.take() {
            commit_light_history(app, "Set Light Temperature", before);
            return true;
        }
        return false;
    }
    if app.active_tab != AppTab::Lights || app.selected_light >= app.lights.len() {
        app.light_temperature_drag_before = None;
        return false;
    }
    if !app.lights[app.selected_light].use_temperature {
        app.light_temperature_drag_before = None;
        return false;
    }
    let rect = inspector_field_rect(app, InspectorField::LightTemperature);
    if app.light_temperature_drag_before.is_none() && !rect.contains(mouse) {
        return false;
    }
    if app.light_temperature_drag_before.is_none() {
        app.light_temperature_drag_before = Some(light_history_snapshot(app));
        app.inspector_edit = None;
        app.light_kind_dropdown_open = false;
        app.light_profile_dropdown_open = false;
    }
    let fraction = ((mouse.x - rect.x) / rect.w).clamp(0.0, 1.0);
    if let Some(light) = app.lights.get_mut(app.selected_light) {
        light.temperature = light_temperature_from_fraction(fraction).round();
        mark_lights_changed(app);
    }
    true
}

pub(crate) fn handle_lights_click(app: &mut AppState, mouse: Vec2) -> bool {
    if update_light_color_picker(app, mouse) {
        return true;
    }
    if app.active_tab != AppTab::Lights {
        return false;
    }
    if !inspector_panel_content_rect().contains(mouse) {
        return false;
    }
    if ctrl_down() && is_mouse_button_pressed(MouseButton::Right) {
        if let Some(row) = light_row_at(app, mouse) {
            if row < app.lights.len() {
                app.selected_light = row;
                reveal_selected_light(app);
                app.inspector_edit = None;
                app.light_kind_dropdown_open = false;
                app.light_profile_dropdown_open = false;
                app.context_menu = Some(ContextMenu {
                    pos: mouse,
                    target: ContextMenuTarget::Selection,
                });
                return true;
            }
        }
    }
    if !is_mouse_button_pressed(MouseButton::Left) {
        return false;
    }
    if app.light_kind_dropdown_open {
        for (row, kind) in light_kind_options().into_iter().enumerate() {
            if light_kind_option_rect(app, row).contains(mouse) {
                let before = light_history_snapshot(app);
                if let Some(light) = app.lights.get_mut(app.selected_light) {
                    light.kind = kind;
                    mark_lights_changed(app);
                }
                commit_light_history(app, "Set Light Kind", before);
                app.light_kind_dropdown_open = false;
                return true;
            }
        }
        app.light_kind_dropdown_open = false;
        return true;
    }
    if app.light_profile_dropdown_open {
        for (row, profile) in light_profile_options().into_iter().enumerate() {
            if light_profile_option_rect(app, row).contains(mouse) {
                let before = light_history_snapshot(app);
                if let Some(light) = app.lights.get_mut(app.selected_light) {
                    light.profile = profile;
                    mark_lights_changed(app);
                }
                commit_light_history(app, "Set Light Profile", before);
                app.light_profile_dropdown_open = false;
                return true;
            }
        }
        app.light_profile_dropdown_open = false;
        return true;
    }
    if light_use_temperature_rect(app).contains(mouse) {
        let before = light_history_snapshot(app);
        if let Some(light) = app.lights.get_mut(app.selected_light) {
            light.use_temperature = !light.use_temperature;
            mark_lights_changed(app);
        }
        commit_light_history(app, "Toggle Light Temperature", before);
        return true;
    }
    if light_casts_shadow_rect(app).contains(mouse) {
        let before = light_history_snapshot(app);
        if let Some(light) = app.lights.get_mut(app.selected_light) {
            light.casts_shadow = !light.casts_shadow;
            mark_lights_changed(app);
        }
        commit_light_history(app, "Toggle Light Shadows", before);
        return true;
    }
    if inspector_field_rect(app, InspectorField::LightKind).contains(mouse)
        && app.lights.get(app.selected_light).is_some()
    {
        app.inspector_edit = None;
        app.light_kind_dropdown_open = true;
        app.light_profile_dropdown_open = false;
        return true;
    }
    if inspector_field_rect(app, InspectorField::LightProfile).contains(mouse)
        && app.lights.get(app.selected_light).is_some()
    {
        app.inspector_edit = None;
        app.light_kind_dropdown_open = false;
        app.light_profile_dropdown_open = true;
        return true;
    }
    if light_button_rect(app, 0).contains(mouse) {
        add_light(app);
        reveal_selected_light(app);
        return true;
    }
    if light_button_rect(app, 1).contains(mouse) {
        if app.selected_light < app.lights.len() {
            delete_selected_light(app);
            reveal_selected_light(app);
        }
        return true;
    }
    if light_button_rect(app, 2).contains(mouse) {
        save_lights_for_app(app);
        return true;
    }
    if let Some(row) = light_row_at(app, mouse) {
        if row < app.lights.len() {
            app.selected_light = row;
            reveal_selected_light(app);
            app.inspector_edit = None;
            app.light_kind_dropdown_open = false;
            app.light_profile_dropdown_open = false;
            return true;
        }
    }
    false
}

/// Open light dropdowns are modal within the inspector: the first click is
/// always handled by the popup, so controls visually underneath cannot fire.
pub(crate) fn handle_open_light_dropdown_click(app: &mut AppState, mouse: Vec2) -> bool {
    if app.active_tab != AppTab::Lights
        || (!app.light_kind_dropdown_open && !app.light_profile_dropdown_open)
        || !is_mouse_button_pressed(MouseButton::Left)
    {
        return false;
    }
    if app.light_kind_dropdown_open {
        for (row, kind) in light_kind_options().into_iter().enumerate() {
            if light_kind_option_rect(app, row).contains(mouse) {
                let before = light_history_snapshot(app);
                if let Some(light) = app.lights.get_mut(app.selected_light) {
                    light.kind = kind;
                    mark_lights_changed(app);
                }
                commit_light_history(app, "Set Light Kind", before);
                app.light_kind_dropdown_open = false;
                return true;
            }
        }
        if inspector_field_rect(app, InspectorField::LightProfile).contains(mouse) {
            app.light_kind_dropdown_open = false;
            app.light_profile_dropdown_open = true;
            app.inspector_edit = None;
            return true;
        }
        app.light_kind_dropdown_open = false;
        return true;
    }
    for (row, profile) in light_profile_options().into_iter().enumerate() {
        if light_profile_option_rect(app, row).contains(mouse) {
            let before = light_history_snapshot(app);
            if let Some(light) = app.lights.get_mut(app.selected_light) {
                light.profile = profile;
                mark_lights_changed(app);
            }
            commit_light_history(app, "Set Light Profile", before);
            app.light_profile_dropdown_open = false;
            return true;
        }
    }
    if inspector_field_rect(app, InspectorField::LightKind).contains(mouse) {
        app.light_profile_dropdown_open = false;
        app.light_kind_dropdown_open = true;
        app.inspector_edit = None;
        return true;
    }
    app.light_profile_dropdown_open = false;
    true
}

pub(crate) fn handle_bake_click(app: &mut AppState, mouse: Vec2) -> bool {
    if app.active_tab != AppTab::Bake {
        return false;
    }
    if !inspector_panel_content_rect().contains(mouse) {
        return false;
    }
    let layout = bake_panel_layout(app);
    if is_mouse_button_down(MouseButton::Left) {
        for channel in 0..3 {
            let rect = layout.color_bars[channel];
            if rect.contains(mouse) {
                let value = ((mouse.x - rect.x) / rect.w).clamp(0.0, 1.0);
                match channel {
                    0 => app.vertex_paint.color.x = value,
                    1 => app.vertex_paint.color.y = value,
                    2 => app.vertex_paint.color.z = value,
                    _ => {}
                }
                save_vertex_paint_settings_preference(app.vertex_paint);
                app.status_message = format!(
                    "Vertex paint RGB {:.2}/{:.2}/{:.2}",
                    app.vertex_paint.color.x, app.vertex_paint.color.y, app.vertex_paint.color.z
                );
                return true;
            }
        }
    }
    if !is_mouse_button_pressed(MouseButton::Left) {
        return false;
    }
    if layout.bake_button.contains(mouse) {
        if app.bake_job.is_none() {
            request_bake_pass(app);
        }
        return true;
    }
    if layout.clear_button.contains(mouse) {
        request_clear_bake(app);
        return true;
    }
    if layout.ao_button.contains(mouse) {
        if app.bake_job.is_none() {
            request_ao_pass(app);
        }
        return true;
    }
    if layout.ambient_bump_button.contains(mouse) {
        if app.bake_job.is_none() {
            request_ambient_bump_pass(app);
        }
        return true;
    }
    if layout.face_emitters.contains(mouse) {
        if app.bake_job.is_none() {
            app.bake_settings.face_emitters_enabled = !app.bake_settings.face_emitters_enabled;
            save_bake_settings_preference(app.bake_settings);
            app.status_message = format!(
                "Face emitters {} for baking",
                if app.bake_settings.face_emitters_enabled {
                    "enabled"
                } else {
                    "disabled"
                }
            );
        }
        return true;
    }
    if layout.regenerate_coronas.contains(mouse) {
        regenerate_all_dff_2dfx_coronas(app);
        return true;
    }
    if layout.brush_toggle.contains(mouse) {
        app.vertex_paint.enabled = !app.vertex_paint.enabled;
        if !app.vertex_paint.enabled {
            flush_vertex_paint_preview(app, true);
        }
        app.status_message = format!(
            "Vertex brush {}",
            if app.vertex_paint.enabled {
                "enabled"
            } else {
                "disabled"
            }
        );
        return true;
    }
    for (slot, tool) in [VertexLightTool::Paint, VertexLightTool::Sample]
        .into_iter()
        .enumerate()
    {
        if layout.tools[slot].contains(mouse) {
            app.vertex_paint.tool = tool;
            save_vertex_paint_settings_preference(app.vertex_paint);
            app.status_message = format!("Vertex lighting tool {}", vertex_light_tool_label(tool));
            return true;
        }
    }
    if layout.blend.contains(mouse) {
        blend_selected_vertex_lighting(app);
        return true;
    }
    if layout.copy.contains(mouse) {
        copy_selected_prelight(app);
        return true;
    }
    if layout.paste.contains(mouse) {
        paste_selected_prelight(app);
        return true;
    }
    if layout.import.contains(mouse) {
        open_import_prelight_dialog(app);
        return true;
    }
    if layout.variant_merge.contains(mouse) {
        request_day_night_variant_merge(app);
        return true;
    }
    for (slot, backend) in [BakeBackend::Cpu, BakeBackend::Gpu].into_iter().enumerate() {
        if layout.backend[slot].contains(mouse) {
            if app.bake_job.is_none() {
                set_bake_backend(app, backend);
            }
            return true;
        }
    }
    for (slot, mode) in [
        BakeLightMode::Day,
        BakeLightMode::Night,
        BakeLightMode::Both,
    ]
    .into_iter()
    .enumerate()
    {
        if layout.light_mode[slot].contains(mouse) {
            if app.bake_job.is_none() {
                set_bake_light_mode(app, mode);
            }
            return true;
        }
    }
    for (slot, scope) in [BakeScope::WholeScene, BakeScope::Selected]
        .into_iter()
        .enumerate()
    {
        if layout.scope[slot].contains(mouse) {
            if app.bake_job.is_none() {
                set_bake_scope(app, scope);
            }
            return true;
        }
    }
    for (slot, preset) in [
        BakeQualityPreset::Preview,
        BakeQualityPreset::Balanced,
        BakeQualityPreset::Final,
    ]
    .into_iter()
    .enumerate()
    {
        if layout.preset[slot].contains(mouse) {
            if app.bake_job.is_none() {
                set_bake_quality(app, preset);
            }
            return true;
        }
    }
    false
}

pub(crate) fn draw_lights_panel(app: &AppState) {
    let x = screen_width() - RIGHT_PANEL_W + 12.0;
    let panel_y = TOP_H + 12.0;
    let y = panel_y - app.properties_scroll;
    let w = RIGHT_PANEL_W - 24.0;
    draw_panel_rect(
        &app.ui_font,
        Rect::new(
            x,
            panel_y,
            w,
            (screen_height() - panel_y - STATUS_H - 12.0).max(1.0),
        ),
        Some("Lights"),
    );
    begin_ui_clip(inspector_panel_content_rect());
    text_button(&app.ui_font, light_button_rect(app, 0), "Add", false);
    text_button(&app.ui_font, light_button_rect(app, 1), "Delete", false);
    text_button(
        &app.ui_font,
        light_button_rect(app, 2),
        if app.manual_save_job.is_some() {
            "Saving..."
        } else {
            "Save Lights"
        },
        app.manual_save_job.is_some(),
    );
    let lights_dirty = app
        .saved_snapshot
        .as_ref()
        .is_none_or(|saved| saved.lights != app.lights);
    ui_text(
        &app.ui_font,
        if app.manual_save_job.is_some() {
            "Project save in progress"
        } else if lights_dirty {
            "Unsaved light changes"
        } else {
            "Lights match the saved project"
        },
        x + 14.0,
        y + 92.0,
        if lights_dirty {
            ui_accent()
        } else {
            ui_muted()
        },
    );

    let row_h = 24.0;
    let list_x = x + 10.0;
    let list_y = TOP_H + 116.0 - app.properties_scroll;
    let start = (app.light_list_scroll.floor() as usize)
        .min(app.lights.len().saturating_sub(LIGHT_LIST_VISIBLE_ROWS));
    let end = (start + LIGHT_LIST_VISIBLE_ROWS).min(app.lights.len());
    for (visible_row, row) in (start..end).enumerate() {
        let row_rect = Rect::new(list_x, list_y + visible_row as f32 * row_h, w - 20.0, row_h);
        if row == app.selected_light {
            draw_rrect(
                row_rect.x,
                row_rect.y,
                row_rect.w,
                row_rect.h,
                7.0,
                ui_accent_soft(),
            );
        } else if row % 2 == 0 {
            draw_rrect(
                row_rect.x,
                row_rect.y,
                row_rect.w,
                row_rect.h,
                7.0,
                Color::new(0.065, 0.083, 0.111, 0.72),
            );
        }
        let light = &app.lights[row];
        ui_text(
            &app.ui_font,
            &ellipsize(
                &match light.attached_to.as_deref() {
                    Some(model) => format!("{}  @ {}", light.name, model),
                    None => light.name.clone(),
                },
                19,
            ),
            row_rect.x + 8.0,
            row_rect.y + 17.0,
            WHITE,
        );
        ui_text(
            &app.ui_font,
            light_kind_label(light.kind),
            row_rect.x + 190.0,
            row_rect.y + 17.0,
            ui_dim(),
        );
        ui_text(
            &app.ui_font,
            light_profile_label(light.profile),
            row_rect.x + 300.0,
            row_rect.y + 17.0,
            ui_muted(),
        );
    }
    if app.lights.len() > LIGHT_LIST_VISIBLE_ROWS {
        let list = light_list_rect(app);
        let max_scroll = app.lights.len().saturating_sub(LIGHT_LIST_VISIBLE_ROWS) as f32;
        let track = Rect::new(list.x + list.w - 6.0, list.y + 2.0, 4.0, list.h - 4.0);
        if let Some(metrics) = scrollbar_metrics(
            track,
            LIGHT_LIST_VISIBLE_ROWS as f32,
            app.lights.len() as f32,
            24.0,
            app.light_list_scroll.min(max_scroll),
        ) {
            draw_scrollbar(metrics, scrollbar_visual_state(track, false));
        }
    }
    ui_text(
        &app.ui_font,
        "Drop Light_List.xml on the window to import",
        x + 14.0,
        y + 304.0,
        ui_muted(),
    );

    if app.lights.get(app.selected_light).is_some() {
        draw_input_box(app, InspectorField::LightName, "Name");
        draw_light_kind_dropdown(app);
        draw_light_profile_dropdown(app);
        draw_input_box(app, InspectorField::LightIntensity, "Intensity");
        draw_inspector_copy_button(app, InspectorCopyAction::LightPosition);
        draw_input_box(app, InspectorField::LightPosition, "Position");
        draw_inspector_copy_button(app, InspectorCopyAction::LightRotation);
        let direction_label = match app.lights[app.selected_light].kind {
            LightKind::Directional => "Direction (Light Travel)",
            LightKind::Area => "Surface Normal",
            _ => "Direction (Light Travel)",
        };
        draw_input_box(app, InspectorField::LightDirection, direction_label);
        if app.lights[app.selected_light].use_temperature {
            draw_light_temperature_slider(app);
        }
        draw_input_box(app, InspectorField::LightRadius, "Radius");
        if let Some(light) = app.lights.get(app.selected_light) {
            draw_checkbox(
                &app.ui_font,
                light_use_temperature_rect(app),
                "Use Temperature",
                light.use_temperature,
            );
            draw_checkbox(
                &app.ui_font,
                light_casts_shadow_rect(app),
                "Cast Shadows",
                light.casts_shadow,
            );
        }
        if !app.lights[app.selected_light].use_temperature {
            draw_light_color_picker(app);
        }
        // Popups are deliberately rendered last so later inspector controls do
        // not paint over them.
        draw_light_dropdown_overlays(app);
    }
    end_ui_clip();
    draw_inspector_scrollbar(app, lights_panel_scroll_max());
}

pub(crate) fn lights_panel_scroll_max() -> f32 {
    panel_scroll_max_for_bottom(TOP_H + 830.0)
}

pub(crate) fn draw_light_kind_dropdown(app: &AppState) {
    let Some(light) = app.lights.get(app.selected_light) else {
        return;
    };
    let rect = inspector_field_rect(app, InspectorField::LightKind);
    ui_text_size(&app.ui_font, "Kind", rect.x, rect.y - 7.0, 14, ui_dim());
    let hovered = rect.contains(mouse_position().into());
    let border = if app.light_kind_dropdown_open || hovered {
        ui_accent()
    } else {
        ui_border()
    };
    draw_rrect_bordered(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        8.0,
        1.0,
        ui_input_bg(),
        border,
    );
    ui_text(
        &app.ui_font,
        light_kind_label(light.kind),
        rect.x + 10.0,
        rect.y + 20.0,
        WHITE,
    );
    ui_text(
        &app.ui_font,
        if app.light_kind_dropdown_open {
            "^"
        } else {
            "v"
        },
        rect.x + rect.w - 22.0,
        rect.y + 20.0,
        ui_dim(),
    );
}

pub(crate) fn draw_light_dropdown_overlays(app: &AppState) {
    let Some(light) = app.lights.get(app.selected_light) else {
        return;
    };
    if app.light_kind_dropdown_open {
        for (row, kind) in light_kind_options().into_iter().enumerate() {
            let option = light_kind_option_rect(app, row);
            let option_hovered = option.contains(mouse_position().into());
            let selected = kind == light.kind;
            let bg = if selected {
                ui_accent_soft()
            } else if option_hovered {
                ui_surface_hover()
            } else {
                ui_surface()
            };
            draw_rrect_bordered(
                option.x,
                option.y,
                option.w,
                option.h,
                6.0,
                1.0,
                bg,
                ui_border(),
            );
            ui_text(
                &app.ui_font,
                light_kind_label(kind),
                option.x + 10.0,
                option.y + 19.0,
                if selected {
                    Color::new(0.76, 0.78, 0.82, 1.0)
                } else {
                    WHITE
                },
            );
        }
        return;
    }
    if app.light_profile_dropdown_open {
        for (row, profile) in light_profile_options().into_iter().enumerate() {
            let option = light_profile_option_rect(app, row);
            let option_hovered = option.contains(mouse_position().into());
            let selected = profile == light.profile;
            let bg = if selected {
                ui_accent_soft()
            } else if option_hovered {
                ui_surface_hover()
            } else {
                ui_surface()
            };
            draw_rrect_bordered(
                option.x,
                option.y,
                option.w,
                option.h,
                6.0,
                1.0,
                bg,
                ui_border(),
            );
            ui_text(
                &app.ui_font,
                light_profile_label(profile),
                option.x + 10.0,
                option.y + 19.0,
                if selected {
                    Color::new(0.76, 0.78, 0.82, 1.0)
                } else {
                    WHITE
                },
            );
        }
    }
}

pub(crate) fn draw_light_profile_dropdown(app: &AppState) {
    let Some(light) = app.lights.get(app.selected_light) else {
        return;
    };
    let rect = inspector_field_rect(app, InspectorField::LightProfile);
    ui_text_size(&app.ui_font, "Profile", rect.x, rect.y - 7.0, 14, ui_dim());
    let hovered = rect.contains(mouse_position().into());
    let border = if app.light_profile_dropdown_open || hovered {
        ui_accent()
    } else {
        ui_border()
    };
    draw_rrect_bordered(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        8.0,
        1.0,
        ui_input_bg(),
        border,
    );
    ui_text(
        &app.ui_font,
        light_profile_label(light.profile),
        rect.x + 10.0,
        rect.y + 20.0,
        WHITE,
    );
    ui_text(
        &app.ui_font,
        if app.light_profile_dropdown_open {
            "^"
        } else {
            "v"
        },
        rect.x + rect.w - 22.0,
        rect.y + 20.0,
        ui_dim(),
    );
}

pub(crate) fn draw_light_temperature_slider(app: &AppState) {
    let Some(light) = app.lights.get(app.selected_light) else {
        return;
    };
    let rect = inspector_field_rect(app, InspectorField::LightTemperature);
    let alpha = if light.use_temperature { 1.0 } else { 0.45 };
    ui_text_size(
        &app.ui_font,
        &format!("Temperature  {:.0} K", light.temperature),
        rect.x,
        rect.y - 7.0,
        14,
        Color::new(ui_dim().r, ui_dim().g, ui_dim().b, alpha),
    );
    draw_temperature_channel_slider(
        rect,
        light.temperature,
        LIGHT_TEMPERATURE_MIN,
        LIGHT_TEMPERATURE_MAX,
        alpha,
    );
}

pub(crate) fn draw_light_color_picker(app: &AppState) {
    let Some(light) = app.lights.get(app.selected_light) else {
        return;
    };
    let color_value = if light.use_temperature {
        light_effective_color(light)
    } else {
        light.color
    };
    let alpha = if light.use_temperature { 0.45 } else { 1.0 };
    let labels = [
        ("R", RED, color_value.x),
        ("G", GREEN, color_value.y),
        ("B", BLUE, color_value.z),
    ];
    ui_text(
        &app.ui_font,
        "Color",
        screen_width() - RIGHT_PANEL_W + 18.0,
        TOP_H + 736.0 - app.properties_scroll,
        ui_dim(),
    );
    for (idx, (label, color, value)) in labels.into_iter().enumerate() {
        let rect = light_color_bar_rect(app, idx);
        ui_text(
            &app.ui_font,
            label,
            rect.x - 28.0,
            rect.y + 14.0,
            Color::new(LIGHTGRAY.r, LIGHTGRAY.g, LIGHTGRAY.b, alpha),
        );
        draw_color_channel_slider(rect, color, value, alpha);
    }
    let swatch = light_color_swatch_rect(app);
    draw_rrect_bordered(
        swatch.x,
        swatch.y,
        swatch.w,
        swatch.h,
        7.0,
        1.0,
        Color::new(color_value.x, color_value.y, color_value.z, 1.0),
        ui_border(),
    );
}

fn draw_bake_section_header(app: &AppState, layout: &BakePanelLayout, label: &str, label_y: f32) {
    let left = layout.panel.x + 14.0;
    let right = layout.panel.x + layout.panel.w - 14.0;
    draw_line(
        left,
        label_y - 16.0,
        right,
        label_y - 16.0,
        1.0,
        ui_border(),
    );
    ui_text(&app.ui_font, label, left, label_y, WHITE);
}

pub(crate) fn draw_bake_panel(app: &AppState) {
    let layout = bake_panel_layout(app);
    draw_panel_rect(&app.ui_font, layout.panel, Some("Vertex Lighting"));
    begin_ui_clip(inspector_panel_content_rect());
    ui_text(
        &app.ui_font,
        "Bake",
        layout.panel.x + 14.0,
        layout.bake_header_y,
        WHITE,
    );
    text_button(
        &app.ui_font,
        layout.bake_button,
        if app.bake_job.is_some() {
            "Baking..."
        } else {
            "Bake Pass"
        },
        app.bake_job.is_some(),
    );
    text_button(&app.ui_font, layout.clear_button, "Clear", false);
    let active_lights = app
        .lights
        .iter()
        .filter(|light| light_active_for_bake(light, app.bake_settings.light_mode))
        .count();
    ui_text(
        &app.ui_font,
        &format!(
            "Lights: {}/{} {}   GPU: {}   GLSL {}",
            active_lights,
            app.lights.len(),
            bake_light_mode_label(app.bake_settings.light_mode),
            gpu_lightmap_label(&app.gpu_lightmap),
            ellipsize(&app.gpu_lightmap.glsl_version, 10)
        ),
        layout.panel.x + 14.0,
        layout.info_y,
        LIGHTGRAY,
    );
    for (slot, backend) in [BakeBackend::Cpu, BakeBackend::Gpu].into_iter().enumerate() {
        text_button(
            &app.ui_font,
            layout.backend[slot],
            bake_backend_label(backend),
            app.bake_settings.backend == backend,
        );
    }
    for (slot, preset) in [
        BakeQualityPreset::Preview,
        BakeQualityPreset::Balanced,
        BakeQualityPreset::Final,
    ]
    .into_iter()
    .enumerate()
    {
        text_button(
            &app.ui_font,
            layout.preset[slot],
            bake_quality_label(preset),
            app.bake_settings.preset == preset,
        );
    }
    for (slot, mode) in [
        BakeLightMode::Day,
        BakeLightMode::Night,
        BakeLightMode::Both,
    ]
    .into_iter()
    .enumerate()
    {
        text_button(
            &app.ui_font,
            layout.light_mode[slot],
            bake_light_mode_label(mode),
            app.bake_settings.light_mode == mode,
        );
    }
    for (slot, scope) in [BakeScope::WholeScene, BakeScope::Selected]
        .into_iter()
        .enumerate()
    {
        text_button(
            &app.ui_font,
            layout.scope[slot],
            bake_scope_label(scope),
            app.bake_settings.scope == scope,
        );
    }
    draw_checkbox(
        &app.ui_font,
        layout.face_emitters,
        "Enable Face Emitters",
        app.bake_settings.face_emitters_enabled,
    );
    text_button(
        &app.ui_font,
        layout.regenerate_coronas,
        if app.corona_generation_job.is_some() {
            "Generating 2DFX Corona Lights..."
        } else {
            "Regenerate All 2DFX Corona Lights"
        },
        app.corona_generation_job.is_some(),
    );
    draw_input_box(app, InspectorField::BakeShadowSamples, "Shadow Samples");
    draw_input_box(app, InspectorField::BakeShadowChunks, "Shadow Chunks");
    draw_input_box(app, InspectorField::BakeBounces, "Bounces");
    draw_input_box(app, InspectorField::BakeBounceStrength, "Bounce Strength");
    draw_input_box(app, InspectorField::BakeShadowSoftness, "Shadow Softness");
    draw_input_box(app, InspectorField::BakeBounceMaximum, "Bounce Maximum");
    draw_input_box(app, InspectorField::BakeExposure, "Exposure (1 = neutral)");
    draw_input_box(app, InspectorField::BakeAmbientBump, "Ambient Bump (+RGB)");
    text_button(
        &app.ui_font,
        layout.ambient_bump_button,
        if app.bake_job.is_some() {
            "Working..."
        } else {
            "Apply Ambient Bump to Existing"
        },
        app.bake_job.is_some(),
    );

    draw_bake_section_header(app, &layout, "Ambient Occlusion", layout.ao_header_y);
    text_button(
        &app.ui_font,
        layout.ao_button,
        if app.bake_job.is_some() {
            "Working..."
        } else {
            "AO Pass"
        },
        app.bake_job.is_some(),
    );
    ui_text(
        &app.ui_font,
        "Darkens the existing bake",
        layout.ao_hint.0,
        layout.ao_hint.1,
        ui_muted(),
    );
    draw_input_box(app, InspectorField::BakeAoSamples, "Samples");
    draw_input_box(app, InspectorField::BakeAoRadius, "Radius");
    draw_input_box(app, InspectorField::BakeAoStrength, "Strength");

    draw_bake_section_header(app, &layout, "Paint Editor", layout.editor_header_y);
    text_button(
        &app.ui_font,
        layout.brush_toggle,
        if app.vertex_paint.enabled {
            "Brush On"
        } else {
            "Brush Off"
        },
        app.vertex_paint.enabled,
    );
    for (slot, tool) in [VertexLightTool::Paint, VertexLightTool::Sample]
        .into_iter()
        .enumerate()
    {
        text_button(
            &app.ui_font,
            layout.tools[slot],
            vertex_light_tool_label(tool),
            app.vertex_paint.tool == tool,
        );
    }
    text_button(&app.ui_font, layout.blend, "Blend", false);
    text_button(&app.ui_font, layout.copy, "Copy", false);
    text_button(&app.ui_font, layout.paste, "Paste", false);
    text_button(&app.ui_font, layout.import, "Import DFF", false);
    text_button(
        &app.ui_font,
        layout.variant_merge,
        if app.day_night_merge_job.is_some() {
            "Comparing Day/Night..."
        } else if selected_day_night_merge_override_ready(app) {
            "Override Day/Night Warning"
        } else {
            "Merge Day + Night"
        },
        app.day_night_merge_job.is_some(),
    );
    ui_text(
        &app.ui_font,
        "Tol",
        layout.variant_tolerance.x - 28.0,
        layout.variant_tolerance.y + 20.0,
        ui_dim(),
    );
    draw_input_box(app, InspectorField::DayNightMergeTolerance, "");
    ui_text(
        &app.ui_font,
        "RGB",
        layout.panel.x + 14.0,
        layout.rgb_label_y,
        ui_dim(),
    );
    let swatch = layout.color_swatch;
    draw_rrect_bordered(
        swatch.x,
        swatch.y,
        swatch.w,
        swatch.h,
        6.0,
        1.0,
        Color::new(
            app.vertex_paint.color.x,
            app.vertex_paint.color.y,
            app.vertex_paint.color.z,
            1.0,
        ),
        ui_border(),
    );
    let channels = [
        ("R", RED, app.vertex_paint.color.x),
        ("G", GREEN, app.vertex_paint.color.y),
        ("B", BLUE, app.vertex_paint.color.z),
    ];
    for (idx, (label, color, value)) in channels.into_iter().enumerate() {
        let rect = layout.color_bars[idx];
        ui_text(&app.ui_font, label, rect.x - 28.0, rect.y + 14.0, LIGHTGRAY);
        draw_color_channel_slider(rect, color, value, 1.0);
    }
    draw_input_box(app, InspectorField::VertexPaintTemperature, "Temp K");
    draw_input_box(app, InspectorField::VertexPaintRadius, "Radius");
    draw_input_box(app, InspectorField::VertexPaintStrength, "Strength");
    let progress = app.bake_job.as_ref().map(bake_progress).unwrap_or(0.0);
    let progress_rect = layout.progress;
    draw_rrect_bordered(
        progress_rect.x,
        progress_rect.y,
        progress_rect.w,
        progress_rect.h,
        6.0,
        1.0,
        Color::new(0.050, 0.058, 0.070, 1.0),
        ui_border(),
    );
    if progress > 0.0 {
        draw_rrect(
            progress_rect.x,
            progress_rect.y,
            progress_rect.w * progress,
            progress_rect.h,
            6.0,
            Color::new(0.42, 0.45, 0.50, 1.0),
        );
    }
    let progress_label = if let Some(job) = app.bake_job.as_ref() {
        if let Some(gpu) = job.gpu_state.as_ref() {
            format!(
                "{:.0}%  GPU batch {}/{}  {} verts",
                progress * 100.0,
                gpu.completed_batches,
                gpu.total_batches,
                gpu.mapping.len(),
            )
        } else {
            let pass_label = match job.pass {
                BakePassKind::Light => bake_backend_label(job.settings.backend),
                BakePassKind::AmbientOcclusion => "AO",
                BakePassKind::AmbientBump => "Bump",
            };
            format!(
                "{:.0}%  {} verts  {}",
                progress * 100.0,
                job.vertices,
                pass_label
            )
        }
    } else {
        format!("Idle  {}", bake_backend_label(app.bake_settings.backend))
    };
    ui_text(
        &app.ui_font,
        &progress_label,
        progress_rect.x,
        progress_rect.y + 30.0,
        ui_dim(),
    );
    ui_text(
        &app.ui_font,
        "Paint edits shared DFF/TXD mesh vertex colors. Save the resource to keep them.",
        layout.panel.x + 14.0,
        layout.footer_y,
        ui_muted(),
    );
    end_ui_clip();
    draw_inspector_scrollbar(app, bake_panel_scroll_max(app));
}

pub(crate) fn bake_panel_scroll_max(app: &AppState) -> f32 {
    let layout = bake_panel_layout(app);
    panel_scroll_max_for_bottom(layout.footer_y + app.properties_scroll + 24.0)
}

pub(crate) fn draw_simulate_panel(app: &AppState) {
    let x = screen_width() - RIGHT_PANEL_W + 16.0;
    let y = TOP_H + 22.0;
    let w = RIGHT_PANEL_W - 32.0;
    draw_panel_rect(&app.ui_font, Rect::new(x, y, w, 330.0), Some("Simulate"));
    text_button(
        &app.ui_font,
        sim_button_rect(0),
        "Place Player",
        app.sim.place_tool == SimPlaceTool::Player,
    );
    text_button(
        &app.ui_font,
        sim_button_rect(1),
        "Place Vehicle",
        app.sim.place_tool == SimPlaceTool::Vehicle,
    );
    text_button(
        &app.ui_font,
        sim_action_rect(0),
        if app.sim.playing { "Pause" } else { "Play" },
        app.sim.playing,
    );
    text_button(&app.ui_font, sim_action_rect(1), "Clear", false);
    let placement_hint = match app.sim.place_tool {
        SimPlaceTool::Player => "Placement tool active: click the viewport to place the player.",
        SimPlaceTool::Vehicle => "Placement tool active: click the viewport to place a vehicle.",
    };
    ui_text(
        &app.ui_font,
        &ellipsize(placement_hint, 48),
        x + 16.0,
        y + 164.0,
        ui_muted(),
    );

    let player_label = if app.sim.player_index.is_some() {
        "Placed"
    } else {
        "Missing"
    };
    let vehicle_label = if app.sim.vehicle_index.is_some() {
        "Placed"
    } else {
        "Missing"
    };
    let mode = if app.sim.controlling_vehicle.is_some() {
        "Driving"
    } else if app.sim.playing {
        "On foot"
    } else {
        "Edit"
    };
    draw_metric_row(&app.ui_font, "Player", player_label, x + 16.0, y + 194.0);
    draw_metric_row(&app.ui_font, "Vehicle", vehicle_label, x + 16.0, y + 228.0);
    draw_metric_row(&app.ui_font, "Mode", mode, x + 16.0, y + 262.0);
    let asset_state = if sim_object_mesh(app, SimObjectKind::Player).is_some()
        && sim_object_mesh(app, SimObjectKind::Vehicle).is_some()
    {
        "Loaded"
    } else {
        "Missing"
    };
    draw_metric_row(&app.ui_font, "Assets", asset_state, x + 16.0, y + 296.0);
}
