use super::super::*;

pub(crate) fn current_rss_mb() -> f32 {
    static RSS_SAMPLE: OnceLock<Mutex<Option<(Instant, f32)>>> = OnceLock::new();

    let now = Instant::now();
    let mut sample = RSS_SAMPLE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some((sampled_at, value)) = *sample
        && now.duration_since(sampled_at) < Duration::from_secs(1)
    {
        return value;
    }

    let Ok(status) = fs::read_to_string("/proc/self/status") else {
        *sample = Some((now, 0.0));
        return 0.0;
    };
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            let kb = rest
                .split_whitespace()
                .next()
                .and_then(|v| v.parse::<f32>().ok())
                .unwrap_or(0.0);
            let value = kb / 1024.0;
            *sample = Some((now, value));
            return value;
        }
    }
    *sample = Some((now, 0.0));
    0.0
}

pub(crate) fn budget_label(value: usize) -> String {
    if value == usize::MAX {
        "unlimited".to_string()
    } else {
        value.to_string()
    }
}

pub(crate) fn update_display_fps(app: &mut AppState) {
    let now = Instant::now();
    let dt = now.duration_since(app.fps_last_frame).as_secs_f32();
    app.fps_last_frame = now;
    if dt > 0.0 && dt.is_finite() {
        app.fps_display = (1.0 / dt).round().clamp(1.0, 999.0) as i32;
    }
}

pub(crate) fn ellipsize(input: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for (idx, ch) in input.chars().enumerate() {
        if idx + 1 >= max_chars {
            out.push_str("...");
            return out;
        }
        out.push(ch);
    }
    out
}

pub(crate) fn ellipsize_width(input: &str, size: u16, max_width: f32) -> String {
    let Some(font) = FONT_REGULAR.get() else {
        return ellipsize(input, (max_width / 8.0).max(1.0) as usize);
    };
    let scale = size as f32 / font.raster_px;
    if ui_text_width_with_font(font, input, size) <= max_width {
        return input.to_string();
    }
    let suffix = "...";
    let suffix_w = ui_text_width_with_font(font, suffix, size);
    let mut out = String::new();
    let mut width = 0.0;
    for ch in input.chars() {
        let advance = glyph_advance(font, ch, scale);
        if width + advance + suffix_w > max_width {
            break;
        }
        width += advance;
        out.push(ch);
    }
    out.push_str(suffix);
    out
}

pub(crate) fn build_outliner_label_slots(len: usize) -> Vec<Option<(String, String)>> {
    vec![None; len]
}

pub(crate) fn invalidate_outliner_labels(app: &mut AppState) {
    app.outliner_labels = build_outliner_label_slots(app.placements.len());
}

pub(crate) fn invalidate_outliner_label(app: &mut AppState, idx: usize) {
    if let Some(label) = app.outliner_labels.get_mut(idx) {
        *label = None;
    }
}

pub(crate) fn outliner_label(app: &mut AppState, idx: usize) -> (&str, &str) {
    if idx >= app.outliner_labels.len() {
        return ("", "");
    }
    if app.outliner_labels[idx].is_none() {
        let label = app
            .placements
            .get(idx)
            .map(|placement| {
                (
                    ellipsize_width(&placement.id, 16, 124.0),
                    ellipsize_width(&placement.dff, 16, 116.0),
                )
            })
            .unwrap_or_else(|| (String::new(), String::new()));
        app.outliner_labels[idx] = Some(label);
    }
    app.outliner_labels[idx]
        .as_ref()
        .map(|(id, dff)| (id.as_str(), dff.as_str()))
        .unwrap_or(("", ""))
}

pub(crate) fn placement_group(placement: &Placement) -> Option<&str> {
    placement
        .attrs
        .get(EDITOR_GROUP_ATTR)
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(crate) fn placement_group_owned(placement: &Placement) -> Option<String> {
    placement_group(placement).map(ToOwned::to_owned)
}

pub(crate) fn asset_group_names(placements: &[Placement]) -> Vec<String> {
    let mut groups = BTreeSet::new();
    for placement in placements {
        if let Some(group) = placement_group(placement) {
            groups.insert(group.to_string());
        }
    }
    groups.into_iter().collect()
}

pub(crate) fn build_outliner_filter(
    placements: &[Placement],
    query: &str,
    expanded_groups: &BTreeSet<String>,
    lod_ids: &HashSet<String>,
    show_objects: bool,
    show_buildings: bool,
    show_lods: bool,
) -> Vec<OutlinerEntry> {
    let needle = lower(query.trim());
    let groups = asset_group_names(placements);
    let mut entries = Vec::new();
    let type_matches = |placement: &Placement| {
        if placement_is_lod(placement, lod_ids) {
            return show_lods;
        }
        if placement.tag.eq_ignore_ascii_case("building") {
            return show_buildings;
        }
        show_objects
    };
    for group in groups {
        let group_has_visible_children = placements.iter().any(|placement| {
            placement_group(placement) == Some(group.as_str()) && type_matches(placement)
        });
        let group_matches = needle.is_empty() || lower(&group).contains(&needle);
        if group_matches && group_has_visible_children {
            entries.push(OutlinerEntry::Group(group.clone()));
        }
        if needle.is_empty() && expanded_groups.contains(&group) {
            entries.extend(
                placements
                    .iter()
                    .enumerate()
                    .filter(|(_, placement)| {
                        placement_group(placement) == Some(group.as_str())
                            && type_matches(placement)
                    })
                    .map(|(idx, _)| OutlinerEntry::GroupChild(idx)),
            );
        }
    }
    if needle.is_empty() {
        entries.extend(
            placements
                .iter()
                .enumerate()
                .filter(|(_, placement)| {
                    placement_group(placement).is_none() && type_matches(placement)
                })
                .map(|(idx, _)| OutlinerEntry::Element(idx)),
        );
        return entries;
    }
    entries.extend(
        placements
            .iter()
            .enumerate()
            .filter_map(|(idx, placement)| {
                if !type_matches(placement) {
                    return None;
                }
                let id = lower(&placement.id);
                let dff = lower(&placement.dff);
                let group = placement_group(placement).map(lower).unwrap_or_default();
                if id.contains(&needle) || dff.contains(&needle) || group.contains(&needle) {
                    Some(OutlinerEntry::Element(idx))
                } else {
                    None
                }
            }),
    );
    entries
}

pub(crate) fn rebuild_outliner_filter(app: &mut AppState) {
    app.outliner_filter = build_outliner_filter(
        &app.placements,
        &app.outliner_search,
        &app.expanded_groups,
        &app.lod_ids,
        app.outliner_show_objects,
        app.outliner_show_buildings,
        app.outliner_show_lods,
    );
    let rows = outliner_rows();
    let max_scroll = app.outliner_filter.len().saturating_sub(rows) as f32;
    app.scroll = app.scroll.min(max_scroll);
}

pub(crate) fn draw_history_panel(app: &AppState) {
    let x = screen_width() - RIGHT_PANEL_W + 12.0;
    let y = TOP_H + 900.0;
    if y + 120.0 > screen_height() - STATUS_H {
        return;
    }
    let w = RIGHT_PANEL_W - 24.0;
    let h = (screen_height() - STATUS_H - y - 12.0).max(154.0);
    draw_panel_rect(&app.ui_font, Rect::new(x, y, w, h), Some("History"));
    ui_text(&app.ui_font, "Undo Stack", x + 14.0, y + 58.0, ui_dim());
    for (row, entry) in app.undo_stack.iter().rev().take(6).enumerate() {
        let label = ellipsize(&entry.label, 20);
        ui_text(
            &app.ui_font,
            &label,
            x + 24.0,
            y + 82.0 + row as f32 * 23.0,
            if row == 0 { ui_accent() } else { LIGHTGRAY },
        );
    }
    let redo_y = y + h - 48.0;
    ui_text(&app.ui_font, "Redo", x + 14.0, redo_y, ui_dim());
    if let Some(entry) = app.redo_stack.last() {
        let label = ellipsize(&entry.label, 20);
        ui_text(
            &app.ui_font,
            &label,
            x + 72.0,
            redo_y,
            Color::new(0.74, 0.78, 0.84, 1.0),
        );
    }
}

pub(crate) fn selected_definition(app: &AppState) -> Option<&Definition> {
    let placement = app.placements.get(app.selected)?;
    app.definitions.get(&placement.id)
}

pub(crate) fn selected_definition_is_readonly(app: &AppState) -> bool {
    app.placements
        .get(app.selected)
        .is_some_and(|placement| app.readonly_definition_ids.contains(&placement.id))
}

pub(crate) fn selected_definition_is_gta_sa(app: &AppState) -> bool {
    selected_definition(app).is_some_and(definition_is_gta_sa)
}

pub(crate) fn definition_field_is_readonly(field: InspectorField) -> bool {
    matches!(
        field,
        InspectorField::DefinitionDff
            | InspectorField::DefinitionNativeModel
            | InspectorField::DefinitionTxd
            | InspectorField::DefinitionCol
            | InspectorField::DefinitionLod
            | InspectorField::DefinitionTimeIn
            | InspectorField::DefinitionTimeOut
            | InspectorField::PhysicsSimulated
            | InspectorField::PhysicsMass
            | InspectorField::PhysicsTurnMass
            | InspectorField::PhysicsAirResistance
            | InspectorField::PhysicsElasticity
            | InspectorField::PhysicsBuoyancy
            | InspectorField::PhysicsCenterOfMassX
            | InspectorField::PhysicsCenterOfMassY
            | InspectorField::PhysicsCenterOfMassZ
    )
}

pub(crate) fn draw_panel(app: &mut AppState, viewport: Rect) {
    let viewport = if app.active_tab == AppTab::Editing {
        editing_center_rect()
    } else if app.active_tab == AppTab::Vehicles {
        vehicle_preview_viewport_rect(app)
    } else {
        viewport
    };
    let sw = screen_width();
    let sh = screen_height();
    let right_x = sw - RIGHT_PANEL_W;
    draw_rectangle(0.0, 0.0, sw, TOP_H, ui_shell_bg());
    draw_rectangle(0.0, 76.0, sw, 36.0, ui_canvas_bg());
    draw_rectangle(0.0, 112.0, sw, TOP_H - 112.0, ui_shell_bg());
    draw_line(0.0, 75.0, sw, 75.0, 1.0, ui_border());
    draw_line(0.0, 112.0, sw, 112.0, 1.0, ui_border());
    if app.active_tab != AppTab::Editing && app.active_tab != AppTab::Vehicles {
        if left_sidebar_visible() {
            draw_rectangle(0.0, TOP_H, PANEL_W, sh - TOP_H - STATUS_H, ui_canvas_bg());
        }
        draw_rectangle(
            right_x,
            TOP_H,
            RIGHT_PANEL_W,
            sh - TOP_H - STATUS_H,
            ui_canvas_bg(),
        );
    }
    draw_rectangle(
        0.0,
        sh - STATUS_H,
        sw,
        STATUS_H,
        Color::new(0.038, 0.041, 0.046, 1.0),
    );
    draw_line(0.0, TOP_H, sw, TOP_H, 1.0, ui_border());
    draw_line(0.0, sh - STATUS_H, sw, sh - STATUS_H, 1.0, ui_border());
    if app.active_tab != AppTab::Editing && app.active_tab != AppTab::Vehicles {
        if left_sidebar_visible() {
            draw_line(PANEL_W, TOP_H, PANEL_W, sh - STATUS_H, 1.0, ui_border());
        }
        draw_line(right_x, TOP_H, right_x, sh - STATUS_H, 1.0, ui_border());
    }
    if !(app.active_tab == AppTab::Vehicles && app.vehicle_browser.photo_mode) {
        draw_rectangle_lines(
            viewport.x,
            viewport.y,
            viewport.w,
            viewport.h,
            1.0,
            Color::new(0.17, 0.18, 0.20, 1.0),
        );
    }
    if let Some(drag) = app.box_select_drag {
        let rect = normalized_screen_rect(drag.start, drag.current);
        draw_rectangle(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            Color::new(0.60, 0.63, 0.68, 0.12),
        );
        draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, ui_accent());
    }

    draw_rrect(12.0, 10.0, 3.0, 38.0, 1.5, ui_accent());
    ui_text_bold("MTA:SA Eagle Edit", 24.0, 24.0, 19, WHITE);
    let path = app.root.to_string_lossy();
    let path_size = ui_text_size_to_fit(&path, 16, (snap_mode_rect().x - 34.0).max(1.0));
    ui_text_size(&app.ui_font, &path, 24.0, 46.0, path_size, ui_dim());
    for tab in primary_app_tabs_for_mode(app.options.launch_mode) {
        let label = app_tab_label(tab);
        let active = app.active_tab == tab;
        let rect = app_tab_rect(app, tab);
        let hovered = rect.contains(mouse_position().into());
        if active || hovered {
            draw_rrect(
                rect.x,
                rect.y + 2.0,
                rect.w,
                rect.h - 4.0,
                8.0,
                if active {
                    ui_surface_active()
                } else {
                    ui_surface_hover()
                },
            );
        }
        if active {
            draw_rrect(
                rect.x + 8.0,
                rect.y + rect.h - 3.0,
                (rect.w - 16.0).max(4.0),
                3.0,
                1.5,
                ui_accent(),
            );
        }
        let visible = ellipsize_width(label, 16, (rect.w - 12.0).max(1.0));
        let tw = ui_text_width(&visible, 16);
        ui_text(
            &app.ui_font,
            &visible,
            rect.x + ((rect.w - tw) * 0.5).max(3.0),
            96.0,
            if active {
                WHITE
            } else if hovered {
                Color::new(0.80, 0.86, 0.93, 1.0)
            } else {
                ui_dim()
            },
        );
    }
    let more_rect = more_tabs_rect(app);
    let more_active = overflow_app_tabs_for_mode(app.options.launch_mode).contains(&app.active_tab);
    let more_hovered = more_rect.contains(mouse_position().into());
    if more_active || more_hovered || app.navigation_menu_open {
        draw_rrect(
            more_rect.x,
            more_rect.y + 2.0,
            more_rect.w,
            more_rect.h - 4.0,
            8.0,
            if more_active || app.navigation_menu_open {
                ui_surface_active()
            } else {
                ui_surface_hover()
            },
        );
    }
    ui_text(
        &app.ui_font,
        "More",
        more_rect.x + 10.0,
        96.0,
        if more_active { WHITE } else { ui_dim() },
    );
    let more_label_w = ui_text_width("More", 16);
    ui_text(
        &app.ui_font,
        "v",
        more_rect.x + 10.0 + more_label_w + 6.0,
        96.0,
        if more_active { WHITE } else { ui_dim() },
    );
    if more_hovered {
        draw_text_tooltip(
            &app.ui_font,
            more_rect,
            "More tools, including LOD audits, texture review, and vertex lighting.",
        );
    }

    let has_selection = has_active_selection(app);
    let selected_deleted = active_selection_deleted(app);
    icon_button(
        &app.ui_font,
        toolbar_button_rect(0),
        &app.icons.select,
        app.transform_mode == TransformMode::Select,
        true,
        "Select (1)",
    );
    icon_button(
        &app.ui_font,
        toolbar_button_rect(1),
        &app.icons.move_tool,
        app.transform_mode == TransformMode::Move,
        has_selection && !selected_deleted,
        "Move (2)",
    );
    icon_button(
        &app.ui_font,
        toolbar_button_rect(2),
        &app.icons.rotate,
        app.transform_mode == TransformMode::Rotate,
        has_selection && !selected_deleted,
        "Rotate (3/R)",
    );
    icon_button(
        &app.ui_font,
        toolbar_button_rect(3),
        &app.icons.scale,
        app.transform_mode == TransformMode::Scale,
        app.active_tab == AppTab::Editing && !selected_editing_dff_vertices(app).is_empty(),
        "Scale (Alt+S)",
    );
    icon_button(
        &app.ui_font,
        toolbar_button_rect(4),
        &app.icons.duplicate,
        false,
        has_selection && !selected_deleted,
        "Duplicate",
    );
    icon_button(
        &app.ui_font,
        toolbar_button_rect(5),
        &app.icons.delete,
        false,
        has_selection,
        if selected_deleted {
            "Restore selected item"
        } else {
            "Delete selected item"
        },
    );
    icon_button(
        &app.ui_font,
        toolbar_button_rect(6),
        &app.icons.undo,
        false,
        !app.undo_stack.is_empty(),
        "Undo",
    );
    icon_button(
        &app.ui_font,
        toolbar_button_rect(7),
        &app.icons.redo,
        false,
        !app.redo_stack.is_empty(),
        "Redo",
    );
    for slot in [3_usize, 5, 7] {
        let rect = toolbar_button_rect(slot);
        draw_line(
            rect.x + rect.w + 3.0,
            TOP_H - 34.0,
            rect.x + rect.w + 3.0,
            TOP_H - 8.0,
            1.0,
            ui_border(),
        );
    }
    draw_line(362.0, TOP_H - 34.0, 362.0, TOP_H - 8.0, 1.0, ui_border());
    text_button(&app.ui_font, snap_mode_rect(), "Snap", app.snap_enabled);
    if snap_mode_rect().contains(mouse_position().into()) {
        draw_text_tooltip(
            &app.ui_font,
            snap_mode_rect(),
            "Snap: constrain move and rotate operations to the configured increments.",
        );
    }
    text_button(
        &app.ui_font,
        transform_space_rect(),
        if app.transform_space == TransformSpace::World {
            "World"
        } else {
            "Local"
        },
        app.transform_space == TransformSpace::Local,
    );
    if transform_space_rect().contains(mouse_position().into()) {
        draw_text_tooltip(
            &app.ui_font,
            transform_space_rect(),
            "World / Local: use global axes or the selected object's own axes for transforms.",
        );
    }
    toolbar_primary_button(&app.ui_font, file_save_rect(), "Save");
    text_button(&app.ui_font, load_resource_rect(), "Load", false);
    text_button(&app.ui_font, save_as_rect(), "Save As", false);
    if app.manual_save_job.is_some() {
        text_button_busy(&app.ui_font, save_wip_rect(), "Saving");
    } else {
        text_button(&app.ui_font, save_wip_rect(), "Save WIP", false);
    }
    text_button(&app.ui_font, generate_txd_button_rect(), "Build TXD", false);
    if SHOW_BLENDER_IMPORT {
        if app.blender_import_rx.is_some() || app.blender_import_setup.is_some() {
            text_button_busy(&app.ui_font, import_blender_button_rect(), "Import Blender");
        } else {
            text_button(
                &app.ui_font,
                import_blender_button_rect(),
                "Import Blender",
                false,
            );
        }
    }
    text_button(
        &app.ui_font,
        preferences_button_rect(),
        "Preferences",
        app.preferences_dialog.is_some(),
    );
    text_button(
        &app.ui_font,
        import_asset_button_rect(),
        "Import new asset",
        app.import_asset_dialog.is_some(),
    );
    let hint_x = import_asset_button_rect().x + import_asset_button_rect().w + 12.0;
    let hint_w = (sw - hint_x - 12.0).max(0.0);
    if hint_w > 160.0 {
        ui_text(
            &app.ui_font,
            &ellipsize_width(
                "H hide   Alt+H isolate   Shift+H unhide   Ctrl+H unhide all",
                14,
                hint_w,
            ),
            hint_x,
            49.0,
            ui_dim(),
        );
    }

    if app.active_tab != AppTab::Editing
        && app.active_tab != AppTab::Race
        && app.active_tab != AppTab::Vehicles
        && app.active_tab != AppTab::TextureReview
        && left_sidebar_visible()
    {
        let out_x = 10.0;
        let out_y = TOP_H + 12.0;
        draw_panel_rect(
            &app.ui_font,
            Rect::new(out_x, out_y, PANEL_W - 20.0, sh - TOP_H - STATUS_H - 24.0),
            Some("Elements"),
        );
        let row_h = OUTLINER_ROW_H;
        let col_shift = (PANEL_W - 320.0) * 0.5;
        let dff_col_x = out_x + 174.0 + col_shift;
        let rows = outliner_rows();
        let max_scroll = app.outliner_filter.len().saturating_sub(rows) as f32;
        app.scroll = app.scroll.min(max_scroll);
        let start = app.scroll as usize;
        let visible_end = (start + rows).min(app.outliner_filter.len());
        let showing_all_types =
            app.outliner_show_objects && app.outliner_show_buildings && app.outliner_show_lods;
        let range_text = if app.outliner_filter.is_empty() {
            if app.placements.is_empty() {
                "0 / 0".to_string()
            } else {
                format!("0 / 0 of {}", app.placements.len())
            }
        } else if app.outliner_search.trim().is_empty() && showing_all_types {
            format!("{}-{} / {}", start + 1, visible_end, app.placements.len())
        } else {
            format!(
                "{}-{} / {} of {}",
                start + 1,
                visible_end,
                app.outliner_filter.len(),
                app.placements.len()
            )
        };
        let range_right = out_x + (PANEL_W - 20.0) - 8.0;
        let range_disp = ellipsize_width(&range_text, 16, 92.0);
        let range_w = ui_text_width(&range_disp, 16);
        ui_text(
            &app.ui_font,
            &range_disp,
            range_right - range_w,
            out_y + 135.0,
            ui_muted(),
        );
        let search_rect = outliner_search_rect();
        let search_bg = if app.outliner_search_active {
            Color::new(0.055, 0.090, 0.128, 1.0)
        } else {
            ui_input_bg()
        };
        let search_border = if app.outliner_search_active {
            ui_accent()
        } else {
            ui_border()
        };
        draw_rrect_bordered(
            search_rect.x,
            search_rect.y,
            search_rect.w,
            search_rect.h,
            8.0,
            1.0,
            search_bg,
            search_border,
        );
        let search_text_x = search_rect.x + 10.0;
        if app.outliner_search.is_empty() && !app.outliner_search_active {
            ui_text(
                &app.ui_font,
                "Search ID or DFF",
                search_text_x,
                search_rect.y + 22.0,
                ui_muted(),
            );
        } else {
            let max_chars = ((search_rect.w - 20.0) / 8.5).max(1.0) as usize;
            let cursor = clamp_char_boundary(&app.outliner_search, app.outliner_search_cursor);
            let cursor_char = app.outliner_search[..cursor].chars().count();
            let total_chars = app.outliner_search.chars().count();
            let start_char = cursor_char.saturating_sub(max_chars.saturating_sub(1));
            let end_char = (start_char + max_chars).min(total_chars);
            let visible: String = app
                .outliner_search
                .chars()
                .skip(start_char)
                .take(end_char - start_char)
                .collect();
            let caret_prefix: String = visible
                .chars()
                .take(cursor_char.saturating_sub(start_char))
                .collect();
            draw_visible_text_selection(
                &app.outliner_search,
                cursor,
                app.outliner_search_anchor,
                start_char,
                &visible,
                search_text_x,
                search_rect,
            );
            ui_text(
                &app.ui_font,
                &visible,
                search_text_x,
                search_rect.y + 22.0,
                WHITE,
            );
            if app.outliner_search_active && (get_time() * 2.0) as i32 % 2 == 0 {
                let caret_x = search_text_x + ui_text_width(&caret_prefix, 16).round();
                draw_line(
                    caret_x,
                    search_rect.y + 7.0,
                    caret_x,
                    search_rect.y + search_rect.h - 7.0,
                    1.0,
                    WHITE,
                );
            }
        }
        let filters = [
            ("Objects", app.outliner_show_objects),
            ("Buildings", app.outliner_show_buildings),
            ("LODs", app.outliner_show_lods),
        ];
        for (slot, (label, enabled)) in filters.iter().enumerate() {
            text_button(
                &app.ui_font,
                outliner_type_filter_rect(slot),
                label,
                *enabled,
            );
        }
        draw_rrect_bordered(
            out_x + 10.0,
            out_y + 114.0,
            PANEL_W - 40.0,
            32.0,
            8.0,
            1.0,
            ui_input_bg(),
            ui_border(),
        );
        ui_text(
            &app.ui_font,
            "Group / ID",
            out_x + 18.0,
            out_y + 135.0,
            ui_dim(),
        );
        ui_text(&app.ui_font, "DFF", dff_col_x, out_y + 135.0, ui_dim());

        for row in 0..rows {
            let Some(entry) = app.outliner_filter.get(start + row).cloned() else {
                break;
            };
            let y = outliner_list_top() + row as f32 * row_h + 19.0;
            let row_rect = Rect::new(out_x + 10.0, y - 19.0, PANEL_W - 40.0, row_h);
            let row_selected = match &entry {
                OutlinerEntry::Group(group) => {
                    app.selected_group.as_deref() == Some(group.as_str())
                }
                OutlinerEntry::GroupChild(idx) => {
                    app.selected_elements.contains(idx) || *idx == app.selected
                }
                OutlinerEntry::Element(idx) => {
                    app.selected_elements.contains(idx) || *idx == app.selected
                }
            };
            if row_selected {
                draw_rrect(
                    row_rect.x,
                    row_rect.y,
                    row_rect.w,
                    row_rect.h,
                    7.0,
                    ui_accent_soft(),
                );
                if matches!(
                    entry,
                    OutlinerEntry::Element(idx) | OutlinerEntry::GroupChild(idx) if idx == app.selected
                ) {
                    draw_rrect(
                        row_rect.x,
                        row_rect.y + 2.0,
                        3.0,
                        row_rect.h - 4.0,
                        1.5,
                        ui_accent(),
                    );
                }
            } else if matches!(
                entry,
                OutlinerEntry::Element(idx) | OutlinerEntry::GroupChild(idx) if app.hovered == Some(idx)
            ) {
                draw_rrect(
                    row_rect.x,
                    row_rect.y,
                    row_rect.w,
                    row_rect.h,
                    7.0,
                    ui_surface_hover(),
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
            if let OutlinerEntry::Group(group) = &entry {
                let count = app
                    .placements
                    .iter()
                    .filter(|placement| placement_group(placement) == Some(group.as_str()))
                    .count();
                let expanded = app.expanded_groups.contains(group.as_str());
                ui_text(
                    &app.ui_font,
                    if expanded { "v" } else { ">" },
                    out_x + 20.0,
                    y,
                    ui_accent(),
                );
                ui_text(&app.ui_font, "GROUP", out_x + 38.0, y, ui_accent());
                if app
                    .group_rename
                    .as_ref()
                    .is_some_and(|edit| edit.original == *group)
                {
                    let edit = app.group_rename.as_ref().unwrap();
                    let edit_rect = Rect::new(out_x + 96.0, y - 19.0, 74.0 + col_shift, row_h);
                    draw_rrect_bordered(
                        edit_rect.x,
                        edit_rect.y + 2.0,
                        edit_rect.w,
                        edit_rect.h - 4.0,
                        5.0,
                        1.0,
                        ui_input_bg(),
                        ui_accent(),
                    );
                    let cursor = clamp_char_boundary(&edit.buffer, edit.cursor);
                    let visible = ellipsize_width(&edit.buffer, 16, edit_rect.w - 14.0);
                    ui_text(&app.ui_font, &visible, edit_rect.x + 7.0, y, WHITE);
                    if (get_time() * 2.0) as i32 % 2 == 0 {
                        let prefix = &edit.buffer[..cursor];
                        let caret_x = (edit_rect.x + 7.0 + ui_text_width(prefix, 16))
                            .min(edit_rect.x + edit_rect.w - 7.0);
                        draw_line(
                            caret_x,
                            edit_rect.y + 6.0,
                            caret_x,
                            edit_rect.y + edit_rect.h - 6.0,
                            1.0,
                            WHITE,
                        );
                    }
                } else {
                    ui_text(
                        &app.ui_font,
                        &ellipsize_width(&group, 16, 74.0 + col_shift),
                        out_x + 96.0,
                        y,
                        WHITE,
                    );
                }
                ui_text(
                    &app.ui_font,
                    &format!("{count} assets"),
                    dff_col_x,
                    y,
                    LIGHTGRAY,
                );
                continue;
            }
            let idx = match entry {
                OutlinerEntry::Element(idx) | OutlinerEntry::GroupChild(idx) => idx,
                OutlinerEntry::Group(_) => unreachable!(),
            };
            let deleted = app
                .element_states
                .get(idx)
                .is_some_and(|state| state.deleted);
            let hidden = app
                .element_states
                .get(idx)
                .is_some_and(|state| state.hidden && !state.deleted);
            let child = matches!(entry, OutlinerEntry::GroupChild(_));
            let group = app.placements.get(idx).and_then(placement_group_owned);
            let (id, dff) = if let Some(placement) = app.placements.get(idx) {
                let id_width = if child {
                    126.0 + col_shift
                } else if group.is_some() {
                    86.0 + col_shift
                } else {
                    146.0 + col_shift
                };
                (
                    ellipsize_width(&placement.id, 16, id_width),
                    ellipsize_width(&placement.dff, 16, 104.0 + col_shift),
                )
            } else {
                let (id, dff) = outliner_label(app, idx);
                (id.to_string(), dff.to_string())
            };
            let id_color = if deleted || hidden {
                Color::new(0.42, 0.44, 0.48, 1.0)
            } else {
                WHITE
            };
            let dff_color = if deleted || hidden {
                Color::new(0.34, 0.36, 0.40, 1.0)
            } else {
                LIGHTGRAY
            };
            let id_x = if child { out_x + 38.0 } else { out_x + 18.0 };
            if child {
                ui_text(&app.ui_font, "-", out_x + 24.0, y, ui_muted());
            }
            ui_text(&app.ui_font, &id, id_x, y, id_color);
            if !child {
                if let Some(group) = group {
                    ui_text(
                        &app.ui_font,
                        &ellipsize_width(&format!("[{group}]"), 16, 58.0),
                        out_x + 112.0 + col_shift,
                        y,
                        ui_accent(),
                    );
                }
            }
            ui_text(&app.ui_font, &dff, dff_col_x, y, dff_color);
            if deleted {
                ui_text(
                    &app.ui_font,
                    "DEL",
                    PANEL_W - 52.0,
                    y,
                    Color::new(0.72, 0.38, 0.38, 1.0),
                );
            } else if hidden {
                ui_text(
                    &app.ui_font,
                    "HIDDEN",
                    PANEL_W - 70.0,
                    y,
                    Color::new(0.54, 0.57, 0.62, 1.0),
                );
            }
        }
        let track = outliner_scrollbar_track();
        if let Some(thumb) = outliner_scrollbar_thumb(app) {
            let metrics = ScrollbarMetrics {
                track,
                thumb,
                max_scroll: 1.0,
            };
            draw_scrollbar(
                metrics,
                scrollbar_visual_state(track, app.outliner_scroll_drag.is_some()),
            );
        }
    }
    let loaded_w = if app.active_tab != AppTab::Editing
        && app.active_tab != AppTab::Vehicles
        && left_sidebar_visible()
    {
        PANEL_W
    } else {
        0.0
    };
    if loaded_w > 0.0 {
        let load_summary = ellipsize_width(&app.loaded_message, 16, (loaded_w - 24.0).max(1.0));
        ui_text(
            &app.ui_font,
            &load_summary,
            12.0,
            screen_height() - 11.0,
            LIGHTGRAY,
        );
    }

    // Keep the activity message and Console action in distinct regions. Detailed
    // renderer counters belong in the activity console, not in the one-line
    // status area where they can obscure save/import feedback.
    let view_rect = status_log_rect();
    let console_hovered = view_rect.contains(mouse_position().into());
    let status_x = loaded_w + 16.0;
    let status_w = (view_rect.x - status_x - 12.0).max(1.0);
    let status = ellipsize_width(&app.status_message, 16, (status_w - 12.0).max(1.0));
    draw_circle(status_x + 3.0, screen_height() - 17.0, 3.0, ui_accent());
    ui_text(
        &app.ui_font,
        &status,
        status_x + 12.0,
        screen_height() - 11.0,
        Color::new(0.72, 0.74, 0.78, 1.0),
    );
    draw_rrect_bordered(
        view_rect.x,
        view_rect.y,
        view_rect.w,
        view_rect.h,
        5.0,
        1.0,
        if console_hovered {
            ui_surface_hover()
        } else {
            ui_surface()
        },
        ui_border(),
    );
    ui_text(
        &app.ui_font,
        "Console",
        view_rect.x + 14.0,
        view_rect.y + 16.0,
        WHITE,
    );
    let diagnostics_x = view_rect.x + view_rect.w + 12.0;
    let diagnostics_w = (sw - diagnostics_x - 12.0).max(0.0);
    if diagnostics_w > 180.0 {
        let diagnostics = format!(
            "{} objects · {} parts · {} verts · RSS {:.0} MB",
            app.last_drawn_placements,
            app.last_drawn_parts,
            app.last_drawn_vertices,
            current_rss_mb(),
        );
        ui_text(
            &app.ui_font,
            &ellipsize_width(&diagnostics, 14, diagnostics_w),
            diagnostics_x,
            screen_height() - 11.0,
            ui_muted(),
        );
    }
    ui_text(
        &app.ui_font,
        &format!("Aliases {}", app.texture_alias_count),
        screen_width() - 206.0,
        28.0,
        LIGHTGRAY,
    );
    ui_text(
        &app.ui_font,
        &format!("FPS {}", app.fps_display),
        screen_width() - 86.0,
        28.0,
        GREEN,
    );
    match app.active_tab {
        AppTab::Preview => {
            draw_inspector(app);
        }
        AppTab::LodAudit => draw_lod_audit_panel(app),
        AppTab::TextureReview => draw_missing_texture_review_panel(app),
        AppTab::Scene => draw_scene_panel(app),
        AppTab::Vehicles => {
            if !app.vehicle_browser.photo_mode {
                draw_vehicle_panel(app);
                draw_vehicle_details_panel(app);
                draw_vehicle_dictionary_manager(app);
                draw_vehicle_build_dialog(app);
                draw_vehicle_collision_copy_dialog(app);
            }
            draw_vehicle_photo_overlay(app);
        }
        AppTab::Validation => draw_validation_panel(app),
        AppTab::Editing => draw_editing_panel(app),
        AppTab::Collisions => {
            draw_collision_panel(app);
            draw_history_panel(app);
        }
        AppTab::Lights => draw_lights_panel(app),
        AppTab::Bake => draw_bake_panel(app),
        AppTab::Water => draw_water_panel(app),
        AppTab::Cull => draw_cull_panel(app),
        AppTab::Race => draw_race_panel(app),
        AppTab::Simulate => draw_simulate_panel(app),
    }
    if app.active_tab != AppTab::Editing
        && app.active_tab != AppTab::Vehicles
        && app.active_tab != AppTab::Race
        && app.active_tab != AppTab::LodAudit
        && app.active_tab != AppTab::TextureReview
    {
        draw_asset_browser(app);
    }
    draw_race_2d_editor(app);
    draw_race_2d_toggle(app);
    draw_race_overlay_toggle(app);
    draw_race_minimap_overlay(app);
    draw_viewport_render_mode_control(app, viewport);
    for tab in primary_app_tabs_for_mode(app.options.launch_mode) {
        let rect = app_tab_rect(app, tab);
        if rect.contains(mouse_position().into()) {
            draw_text_tooltip(&app.ui_font, rect, app_tab_tooltip(tab));
        }
    }
    if app.navigation_menu_open {
        let bounds = navigation_menu_bounds(app);
        draw_rrect(
            bounds.x + 2.0,
            bounds.y + 3.0,
            bounds.w,
            bounds.h,
            9.0,
            Color::new(0.0, 0.0, 0.0, 0.32),
        );
        draw_rrect_bordered(
            bounds.x,
            bounds.y,
            bounds.w,
            bounds.h,
            9.0,
            1.0,
            Color::new(0.045, 0.058, 0.078, 0.99),
            ui_border(),
        );
        for tab in overflow_app_tabs_for_mode(app.options.launch_mode) {
            let rect = overflow_tab_rect(app, tab);
            let active = app.active_tab == tab;
            let hovered = rect.contains(mouse_position().into());
            if active || hovered {
                draw_rrect(
                    rect.x,
                    rect.y,
                    rect.w,
                    rect.h,
                    6.0,
                    if active {
                        ui_surface_active()
                    } else {
                        ui_surface_hover()
                    },
                );
            }
            ui_text(
                &app.ui_font,
                app_tab_label(tab),
                rect.x + 11.0,
                rect.y + 19.0,
                if active {
                    WHITE
                } else {
                    Color::new(0.80, 0.86, 0.93, 1.0)
                },
            );
            if hovered {
                draw_text_tooltip(&app.ui_font, rect, app_tab_tooltip(tab));
            }
        }
    }
    draw_context_menu(app);
    if app.last_log.elapsed().as_secs_f32() >= 1.0 {
        let viewport_diagnostics = viewport_render_diagnostics();
        eprintln!(
            "frame: fps={} rss={:.0}MB placements={} parts={} vertices={} msaa={} radius={} part_budget={} vertex_budget={} textures={}",
            app.fps_display,
            current_rss_mb(),
            app.last_drawn_placements,
            app.last_drawn_parts,
            app.last_drawn_vertices,
            app.options.msaa_samples,
            app.options.draw_radius,
            budget_label(app.options.part_budget),
            budget_label(app.options.vertex_budget),
            app.options.textures
        );
        eprintln!(
            "viewport: total={:.2}ms cull={:.2}ms sort={:.2}ms submit={:.2}ms cells={}/{} submitted={} blend_segments={} draw_calls={} state_changes={}",
            viewport_diagnostics.frame_ms,
            viewport_diagnostics.cull_ms,
            viewport_diagnostics.sort_ms,
            viewport_diagnostics.submit_ms,
            viewport_diagnostics.visible_cells,
            viewport_diagnostics.candidate_cells,
            viewport_diagnostics.submitted_cells,
            viewport_diagnostics.blend_segments,
            viewport_diagnostics.draw_calls,
            viewport_diagnostics.state_changes,
        );
        eprintln!(
            "residency: cells={} gpu={:.1}MB uploads={} evictions={} misses={}",
            viewport_diagnostics.resident_cells,
            viewport_diagnostics.resident_bytes as f64 / (1024.0 * 1024.0),
            viewport_diagnostics.residency_uploads,
            viewport_diagnostics.residency_evictions,
            viewport_diagnostics.residency_misses,
        );
        app.last_log = Instant::now();
    }
}

pub(crate) fn viewport_render_mode_available(app: &AppState) -> bool {
    app.active_tab == AppTab::Preview
        || (app.active_tab == AppTab::Editing
            && !editing_material_picker_is_open(app)
            && matches!(
                app.editing.asset,
                Some(EditingAsset::Dff(ref dff)) if !dff.uv_editor.open
            ))
}

pub(crate) fn viewport_render_mode_rects(app: &AppState, viewport: Rect) -> [Rect; 3] {
    let viewport = if app.active_tab == AppTab::Editing {
        editing_center_rect()
    } else {
        viewport
    };
    let gap = 6.0;
    let total_w = 552.0_f32.min((viewport.w - 24.0).max(3.0));
    let button_w = ((total_w - gap * 2.0) / 3.0).max(1.0);
    let x = viewport.x + (viewport.w - total_w) * 0.5;
    let y = viewport.y
        + if app.active_tab == AppTab::Editing {
            58.0
        } else {
            12.0
        };
    [
        Rect::new(x, y, button_w, 32.0),
        Rect::new(x + button_w + gap, y, button_w, 32.0),
        Rect::new(x + (button_w + gap) * 2.0, y, button_w, 32.0),
    ]
}

pub(crate) fn handle_viewport_render_mode_click(
    app: &mut AppState,
    viewport: Rect,
    mouse: Vec2,
) -> bool {
    if !viewport_render_mode_available(app) || !is_mouse_button_pressed(MouseButton::Left) {
        return false;
    }
    for (mode, rect) in ViewportRenderMode::ALL
        .into_iter()
        .zip(viewport_render_mode_rects(app, viewport))
    {
        if rect.contains(mouse) {
            app.viewport_render_mode = mode;
            app.status_message = format!("Viewport: {}", mode.label());
            return true;
        }
    }
    false
}

fn draw_viewport_render_mode_control(app: &AppState, viewport: Rect) {
    if !viewport_render_mode_available(app) {
        return;
    }
    let rects = viewport_render_mode_rects(app, viewport);
    for (mode, rect) in ViewportRenderMode::ALL.into_iter().zip(rects) {
        text_button(
            &app.ui_font,
            rect,
            mode.label(),
            app.viewport_render_mode == mode,
        );
    }
    if app.viewport_render_mode == ViewportRenderMode::CollisionClassification {
        let label = "Magenta = No Collision; other colors = generated GTA surface material";
        let width = ui_text_width(label, 14);
        ui_text_size(
            &app.ui_font,
            label,
            rects[0].x + ((rects[2].x + rects[2].w - rects[0].x - width) * 0.5).max(0.0),
            rects[0].y + rects[0].h + 18.0,
            14,
            WHITE,
        );
    }
}
