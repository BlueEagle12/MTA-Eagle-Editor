use super::super::*;

pub(crate) fn draw_context_menu(app: &AppState) {
    let Some(rect) = context_menu_rect(app) else {
        return;
    };
    // Controls underneath the floating menu were drawn first and may already
    // have queued a tooltip for the same pointer position. A context menu owns
    // the pointer while open, so discard that underlying hover UI.
    clear_pending_ui_tooltip();
    let mouse: Vec2 = mouse_position().into();
    draw_rrect_bordered(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        10.0,
        1.0,
        Color::new(0.052, 0.055, 0.061, 0.99),
        ui_border(),
    );
    let row_h = 26.0;
    for (row, (_, label, enabled)) in context_menu_items(app).iter().enumerate() {
        let y = rect.y + 6.0 + row as f32 * row_h;
        let row_rect = Rect::new(rect.x + 6.0, y, rect.w - 12.0, row_h);
        if *enabled && row_rect.contains(mouse) {
            draw_rrect(
                row_rect.x,
                row_rect.y + 2.0,
                row_rect.w,
                row_rect.h - 4.0,
                5.0,
                ui_accent_soft(),
            );
        }
        let color = if *enabled { WHITE } else { ui_muted() };
        ui_text(&app.ui_font, label, row_rect.x + 8.0, y + 18.0, color);
    }
}

pub(crate) fn draw_modal_backdrop() {
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        screen_height(),
        Color::new(0.0, 0.0, 0.0, 0.58),
    );
}

pub(crate) fn draw_dialog_button(font: &Font, rect: Rect, label: &str, primary: bool) {
    let hovered = rect.contains(mouse_position().into());
    let bg = if primary {
        if hovered {
            Color::new(0.27, 0.29, 0.32, 1.0)
        } else {
            Color::new(0.19, 0.21, 0.24, 1.0)
        }
    } else if hovered {
        ui_surface_hover()
    } else {
        ui_surface()
    };
    draw_rrect(rect.x + 1.0, rect.y + 2.0, rect.w, rect.h, 9.0, ui_shadow());
    draw_rrect_bordered(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        9.0,
        1.0,
        bg,
        if primary { ui_accent() } else { ui_border() },
    );
    let tw = ui_text_width(label, 16);
    ui_text(
        font,
        label,
        rect.x + (rect.w - tw) * 0.5,
        rect.y + 22.0,
        WHITE,
    );
}

pub(crate) fn draw_save_as_dialog(app: &AppState) {
    let Some(dialog) = app.save_as_dialog.as_ref() else {
        return;
    };
    draw_modal_backdrop();
    let rect = save_as_dialog_rect();
    draw_panel_rect(&app.ui_font, rect, Some("Save As"));
    ui_text(
        &app.ui_font,
        "Create a separate resource copy. Your currently loaded resource is not changed.",
        rect.x + 24.0,
        rect.y + 58.0,
        LIGHTGRAY,
    );
    let input = save_as_input_rect();
    draw_rrect_bordered(
        input.x,
        input.y,
        input.w,
        input.h,
        7.0,
        1.0,
        ui_input_bg(),
        ui_accent(),
    );
    let max_chars = ((input.w - 20.0) / 8.5).max(1.0) as usize;
    let cursor = clamp_char_boundary(&dialog.path, dialog.cursor);
    let cursor_char = dialog.path[..cursor].chars().count();
    let total_chars = dialog.path.chars().count();
    let start_char = cursor_char.saturating_sub(max_chars.saturating_sub(1));
    let end_char = (start_char + max_chars).min(total_chars);
    let visible: String = dialog
        .path
        .chars()
        .skip(start_char)
        .take(end_char - start_char)
        .collect();
    let caret_prefix: String = visible
        .chars()
        .take(cursor_char.saturating_sub(start_char))
        .collect();
    let text_x = input.x + 10.0;
    draw_visible_text_selection(
        &dialog.path,
        cursor,
        dialog.selection_anchor,
        start_char,
        &visible,
        text_x,
        input,
    );
    ui_text(&app.ui_font, &visible, text_x, input.y + 21.0, WHITE);
    if (get_time() * 2.0) as i32 % 2 == 0 {
        let caret_x = text_x + ui_text_width(&caret_prefix, 16).round();
        draw_line(
            caret_x,
            input.y + 7.0,
            caret_x,
            input.y + input.h - 7.0,
            1.0,
            WHITE,
        );
    }
    ui_text(
        &app.ui_font,
        "Enter creates the copy   Esc cancels   Existing IMG files are copied",
        rect.x + 24.0,
        rect.y + 128.0,
        ui_muted(),
    );
    draw_dialog_button(
        &app.ui_font,
        Rect::new(rect.x + rect.w - 216.0, rect.y + rect.h - 50.0, 88.0, 32.0),
        "Save As",
        true,
    );
    draw_dialog_button(
        &app.ui_font,
        Rect::new(rect.x + rect.w - 116.0, rect.y + rect.h - 50.0, 88.0, 32.0),
        "Cancel",
        false,
    );
}

pub(crate) fn draw_save_log_dialog(app: &AppState) {
    if !app.save_log_open {
        return;
    }
    draw_modal_backdrop();
    let rect = save_log_dialog_rect();
    draw_panel_rect(&app.ui_font, rect, Some("Activity Console"));
    ui_text(
        &app.ui_font,
        "Live session messages   Mouse wheel scrolls   Home/End jumps   Esc closes",
        rect.x + 24.0,
        rect.y + 58.0,
        ui_muted(),
    );

    let list = Rect::new(rect.x + 24.0, rect.y + 78.0, rect.w - 48.0, rect.h - 142.0);
    draw_rrect_bordered(
        list.x,
        list.y,
        list.w,
        list.h,
        7.0,
        1.0,
        ui_input_bg(),
        ui_border(),
    );
    let rows = save_log_display_rows(app, list.w);
    let visible_rows = (list.h / 22.0).max(1.0) as usize;
    let max_scroll = rows.len().saturating_sub(visible_rows) as f32;
    let scroll = app.save_log_scroll.clamp(0.0, max_scroll);
    let start = scroll.floor() as usize;
    if rows.is_empty() {
        ui_text(
            &app.ui_font,
            "No activity recorded yet.",
            list.x + 14.0,
            list.y + 26.0,
            LIGHTGRAY,
        );
    } else {
        for row in 0..visible_rows {
            let idx = start + row;
            let Some(line) = rows.get(idx) else {
                break;
            };
            let y = list.y + 24.0 + row as f32 * 22.0;
            ui_text(&app.ui_font, line, list.x + 14.0, y, LIGHTGRAY);
        }
    }
    if rows.len() > visible_rows {
        let track = Rect::new(list.x + list.w - 10.0, list.y + 8.0, 4.0, list.h - 16.0);
        if let Some(metrics) =
            scrollbar_metrics(track, visible_rows as f32, rows.len() as f32, 24.0, scroll)
        {
            draw_scrollbar(metrics, scrollbar_visual_state(track, false));
        }
    }
    draw_dialog_button(&app.ui_font, activity_console_clear_rect(), "Clear", false);
    draw_dialog_button(&app.ui_font, save_log_close_rect(), "Close", false);
}

pub(crate) fn draw_blender_import_dialog(app: &AppState) {
    if !app.blender_import_dialog_open {
        return;
    }
    draw_modal_backdrop();
    let rect = blender_import_dialog_rect();
    draw_panel_rect(&app.ui_font, rect, Some("Import Blender Map"));
    ui_text(
        &app.ui_font,
        &ellipsize_width(&app.blender_import_phase, 16, rect.w - 170.0),
        rect.x + 24.0,
        rect.y + 62.0,
        WHITE,
    );
    ui_text(
        &app.ui_font,
        &format!(
            "{:>3.0}%",
            app.blender_import_progress.clamp(0.0, 1.0) * 100.0
        ),
        rect.x + rect.w - 70.0,
        rect.y + 62.0,
        LIGHTGRAY,
    );

    let progress = Rect::new(rect.x + 24.0, rect.y + 78.0, rect.w - 48.0, 18.0);
    draw_rrect_bordered(
        progress.x,
        progress.y,
        progress.w,
        progress.h,
        6.0,
        1.0,
        ui_input_bg(),
        ui_border(),
    );
    let fill = (progress.w - 4.0) * app.blender_import_progress.clamp(0.0, 1.0);
    if fill > 0.5 {
        draw_rrect(
            progress.x + 2.0,
            progress.y + 2.0,
            fill,
            progress.h - 4.0,
            4.0,
            ui_accent(),
        );
    }
    ui_text(
        &app.ui_font,
        "Live Blender / RRW:MTA output   Mouse wheel scrolls   Home/End jumps",
        rect.x + 24.0,
        rect.y + 126.0,
        ui_muted(),
    );

    let list = Rect::new(rect.x + 24.0, rect.y + 142.0, rect.w - 48.0, rect.h - 206.0);
    draw_rrect_bordered(
        list.x,
        list.y,
        list.w,
        list.h,
        7.0,
        1.0,
        ui_input_bg(),
        ui_border(),
    );
    let rows = blender_import_display_rows(app, list.w);
    let visible_rows = (list.h / 20.0).max(1.0) as usize;
    let max_scroll = rows.len().saturating_sub(visible_rows) as f32;
    let scroll = if app.blender_import_log_follow_tail {
        max_scroll
    } else {
        app.blender_import_log_scroll.clamp(0.0, max_scroll)
    };
    let start = scroll.floor() as usize;
    for row in 0..visible_rows {
        let Some(line) = rows.get(start + row) else {
            break;
        };
        let color = if line.contains("ERROR") || line.contains("Traceback") {
            Color::new(1.0, 0.48, 0.48, 1.0)
        } else if line.contains("WARN") || line.contains("warning") {
            Color::new(1.0, 0.78, 0.36, 1.0)
        } else {
            LIGHTGRAY
        };
        ui_text(
            &app.ui_font,
            line,
            list.x + 12.0,
            list.y + 21.0 + row as f32 * 20.0,
            color,
        );
    }
    if rows.len() > visible_rows {
        let track = Rect::new(list.x + list.w - 10.0, list.y + 8.0, 4.0, list.h - 16.0);
        if let Some(metrics) =
            scrollbar_metrics(track, visible_rows as f32, rows.len() as f32, 24.0, scroll)
        {
            draw_scrollbar(metrics, scrollbar_visual_state(track, false));
        }
    }

    if app.blender_import_finished {
        ui_text(
            &app.ui_font,
            "The saved log remains in the project's logs folder.",
            rect.x + 24.0,
            rect.y + rect.h - 27.0,
            ui_muted(),
        );
        draw_dialog_button(&app.ui_font, blender_import_close_rect(), "Close", false);
    } else {
        ui_text(
            &app.ui_font,
            "Import is running. Keep Eagle Editor open.",
            rect.x + 24.0,
            rect.y + rect.h - 27.0,
            ui_muted(),
        );
    }
}

pub(crate) fn draw_blender_import_setup_dialog(app: &AppState) {
    let Some(setup) = app.blender_import_setup.as_ref() else {
        return;
    };
    draw_modal_backdrop();
    let rect = blender_import_setup_dialog_rect();
    draw_panel_rect(&app.ui_font, rect, Some("Blender Import Setup"));
    ui_text(
        &app.ui_font,
        "Source",
        rect.x + 24.0,
        rect.y + 62.0,
        ui_dim(),
    );
    ui_text(
        &app.ui_font,
        &ellipsize_width(&setup.source.to_string_lossy(), 16, rect.w - 116.0),
        rect.x + 88.0,
        rect.y + 62.0,
        LIGHTGRAY,
    );
    ui_text(&app.ui_font, "TXDs", rect.x + 24.0, rect.y + 96.0, ui_dim());
    ui_text(
        &app.ui_font,
        "Follow Blender definition settings; default is texture.txd",
        rect.x + 104.0,
        rect.y + 96.0,
        LIGHTGRAY,
    );
    ui_text(
        &app.ui_font,
        "Chunk size",
        rect.x + 24.0,
        rect.y + 146.0,
        WHITE,
    );
    ui_text_size(
        &app.ui_font,
        "Maximum world-space size for generated visual mesh chunks",
        rect.x + 24.0,
        rect.y + 166.0,
        13,
        ui_muted(),
    );
    let input = blender_import_setup_chunk_rect();
    draw_rrect_bordered(
        input.x,
        input.y,
        input.w,
        input.h,
        7.0,
        1.0,
        ui_input_bg(),
        if setup.error.is_some() {
            RED
        } else {
            ui_accent()
        },
    );
    ui_text(
        &app.ui_font,
        &setup.chunk_size,
        input.x + 10.0,
        input.y + 21.0,
        WHITE,
    );
    if (get_time() * 2.0) as i32 % 2 == 0 {
        let cursor = clamp_char_boundary(&setup.chunk_size, setup.chunk_size_cursor);
        let prefix = &setup.chunk_size[..cursor];
        let caret_x = input.x + 10.0 + ui_text_width(prefix, 16);
        draw_line(caret_x, input.y + 7.0, caret_x, input.y + 25.0, 1.0, WHITE);
    }

    let chunk_row = blender_import_setup_toggle_rect(0);
    draw_checkbox(
        &app.ui_font,
        chunk_row,
        "Split oversized visual meshes into chunks",
        setup.options.chunk_meshes,
    );
    ui_text_size(
        &app.ui_font,
        "Disable only when the Blender scene is already partitioned for streaming",
        chunk_row.x + 28.0,
        chunk_row.y + 35.0,
        13,
        ui_muted(),
    );
    let origins_row = blender_import_setup_toggle_rect(1);
    draw_checkbox(
        &app.ui_font,
        origins_row,
        "Center generated model origins",
        setup.options.center_origins,
    );
    ui_text_size(
        &app.ui_font,
        "Keeps model pivots compact while preserving placement in the generated map",
        origins_row.x + 28.0,
        origins_row.y + 35.0,
        13,
        ui_muted(),
    );

    if let Some(error) = setup.error.as_deref() {
        ui_text_size(
            &app.ui_font,
            error,
            rect.x + 24.0,
            rect.y + rect.h - 26.0,
            13,
            Color::new(1.0, 0.48, 0.48, 1.0),
        );
    } else {
        ui_text_size(
            &app.ui_font,
            "Collision generation is handled after import by Eagle Editor.",
            rect.x + 24.0,
            rect.y + rect.h - 26.0,
            13,
            ui_muted(),
        );
    }
    draw_dialog_button(
        &app.ui_font,
        blender_import_setup_start_rect(),
        "Start Import",
        true,
    );
    draw_dialog_button(
        &app.ui_font,
        blender_import_setup_cancel_rect(),
        "Cancel",
        false,
    );
}

pub(crate) fn oversized_chunk_dialog_rect() -> Rect {
    let w = 780.0_f32.min(screen_width() - 64.0);
    let h = 610.0_f32.min(screen_height() - 64.0);
    Rect::new(
        (screen_width() - w) * 0.5,
        (screen_height() - h) * 0.5,
        w,
        h,
    )
}

pub(crate) fn oversized_chunk_size_rect() -> Rect {
    let rect = oversized_chunk_dialog_rect();
    Rect::new(rect.x + 164.0, rect.y + 72.0, 130.0, 32.0)
}

pub(crate) fn oversized_chunk_rescan_rect() -> Rect {
    let rect = oversized_chunk_dialog_rect();
    Rect::new(rect.x + 306.0, rect.y + 72.0, 92.0, 32.0)
}

pub(crate) fn oversized_chunk_list_rect() -> Rect {
    let rect = oversized_chunk_dialog_rect();
    Rect::new(rect.x + 24.0, rect.y + 128.0, rect.w - 48.0, rect.h - 210.0)
}

pub(crate) fn oversized_chunk_candidate_rect(app: &AppState, index: usize) -> Rect {
    let list = oversized_chunk_list_rect();
    Rect::new(
        list.x + 6.0,
        list.y + 6.0 + index as f32 * 58.0
            - app
                .oversized_chunk_dialog
                .as_ref()
                .map_or(0.0, |dialog| dialog.scroll),
        list.w - 12.0,
        52.0,
    )
}

pub(crate) fn oversized_chunk_apply_rect() -> Rect {
    let rect = oversized_chunk_dialog_rect();
    Rect::new(rect.x + rect.w - 246.0, rect.y + rect.h - 52.0, 132.0, 32.0)
}

pub(crate) fn oversized_chunk_cancel_rect() -> Rect {
    let rect = oversized_chunk_dialog_rect();
    Rect::new(rect.x + rect.w - 102.0, rect.y + rect.h - 52.0, 78.0, 32.0)
}

pub(crate) fn draw_oversized_chunk_dialog(app: &AppState) {
    let Some(dialog) = app.oversized_chunk_dialog.as_ref() else {
        return;
    };
    draw_modal_backdrop();
    let rect = oversized_chunk_dialog_rect();
    draw_panel_rect(&app.ui_font, rect, Some("Chunk Oversized Elements"));
    ui_text(
        &app.ui_font,
        "Cell size (X/Y/Z)",
        rect.x + 24.0,
        rect.y + 94.0,
        WHITE,
    );
    let input = oversized_chunk_size_rect();
    draw_rrect_bordered(
        input.x,
        input.y,
        input.w,
        input.h,
        7.0,
        1.0,
        ui_input_bg(),
        if dialog.error.is_some() {
            RED
        } else {
            ui_accent()
        },
    );
    ui_text(
        &app.ui_font,
        &dialog.chunk_size,
        input.x + 10.0,
        input.y + 21.0,
        WHITE,
    );
    if (get_time() * 2.0) as i32 % 2 == 0 {
        let cursor = clamp_char_boundary(&dialog.chunk_size, dialog.chunk_size_cursor);
        let prefix = &dialog.chunk_size[..cursor];
        let caret_x = input.x + 10.0 + ui_text_width(prefix, 16);
        draw_line(caret_x, input.y + 7.0, caret_x, input.y + 25.0, 1.0, WHITE);
    }
    draw_dialog_button(&app.ui_font, oversized_chunk_rescan_rect(), "Review", false);
    let supported = dialog
        .candidates
        .iter()
        .filter(|candidate| candidate.blocked_reason.is_none())
        .count();
    let selected = dialog
        .candidates
        .iter()
        .filter(|candidate| candidate.selected)
        .count();
    ui_text_size(
        &app.ui_font,
        &format!(
            "{} oversized DFF(s), {} safe to cut, {} selected",
            dialog.candidates.len(),
            supported,
            selected
        ),
        rect.x + 418.0,
        rect.y + 93.0,
        13,
        ui_muted(),
    );

    let list = oversized_chunk_list_rect();
    draw_rrect_bordered(
        list.x,
        list.y,
        list.w,
        list.h,
        8.0,
        1.0,
        ui_input_bg(),
        ui_border(),
    );
    begin_ui_clip(list);
    for (index, candidate) in dialog.candidates.iter().enumerate() {
        let row = oversized_chunk_candidate_rect(app, index);
        if row.y + row.h < list.y || row.y > list.y + list.h {
            continue;
        }
        draw_rrect(
            row.x,
            row.y,
            row.w,
            row.h,
            6.0,
            Color::new(0.085, 0.09, 0.10, 1.0),
        );
        let checkbox = Rect::new(row.x + 10.0, row.y + 8.0, 24.0, 24.0);
        draw_checkbox(
            &app.ui_font,
            checkbox,
            "",
            candidate.selected && candidate.blocked_reason.is_none(),
        );
        ui_text(
            &app.ui_font,
            &candidate.dff_name,
            row.x + 44.0,
            row.y + 22.0,
            WHITE,
        );
        let detail = if let Some(reason) = candidate.blocked_reason.as_deref() {
            format!(
                "{:.1} x {:.1} x {:.1} | {} placement(s) | skipped: {reason}",
                candidate.extents.x,
                candidate.extents.y,
                candidate.extents.z,
                candidate.placement_count
            )
        } else {
            format!(
                "{:.1} x {:.1} x {:.1} | {} placement(s)",
                candidate.extents.x,
                candidate.extents.y,
                candidate.extents.z,
                candidate.placement_count
            )
        };
        ui_text_size(
            &app.ui_font,
            &detail,
            row.x + 44.0,
            row.y + 42.0,
            13,
            if candidate.blocked_reason.is_some() {
                Color::new(1.0, 0.55, 0.4, 1.0)
            } else {
                ui_muted()
            },
        );
    }
    if dialog.candidates.is_empty() {
        ui_text(
            &app.ui_font,
            "No loaded elements exceed this cell size.",
            list.x + 18.0,
            list.y + 34.0,
            ui_muted(),
        );
    }
    end_ui_clip();
    if let Some(error) = dialog.error.as_deref() {
        ui_text_size(
            &app.ui_font,
            error,
            rect.x + 24.0,
            rect.y + rect.h - 28.0,
            13,
            Color::new(1.0, 0.48, 0.48, 1.0),
        );
    } else {
        ui_text_size(
            &app.ui_font,
            "Visual DFFs are clipped at cell boundaries; the source keeps its original collision and siblings are render-only.",
            rect.x + 24.0,
            rect.y + rect.h - 28.0,
            13,
            ui_muted(),
        );
    }
    draw_dialog_button(
        &app.ui_font,
        oversized_chunk_apply_rect(),
        "Cut Selected",
        true,
    );
    draw_dialog_button(&app.ui_font, oversized_chunk_cancel_rect(), "Cancel", false);
}

pub(crate) fn classify_dialog_rect() -> Rect {
    let w = 780.0_f32.min(screen_width() - 64.0);
    let h = 610.0_f32.min(screen_height() - 64.0);
    Rect::new(
        (screen_width() - w) * 0.5,
        (screen_height() - h) * 0.5,
        w,
        h,
    )
}

pub(crate) fn classify_size_rect() -> Rect {
    let rect = classify_dialog_rect();
    Rect::new(rect.x + 214.0, rect.y + 72.0, 130.0, 32.0)
}

pub(crate) fn classify_review_rect() -> Rect {
    let rect = classify_dialog_rect();
    Rect::new(rect.x + 356.0, rect.y + 72.0, 92.0, 32.0)
}

pub(crate) fn classify_list_rect() -> Rect {
    let rect = classify_dialog_rect();
    Rect::new(rect.x + 24.0, rect.y + 128.0, rect.w - 48.0, rect.h - 210.0)
}

pub(crate) fn classify_candidate_rect(app: &AppState, index: usize) -> Rect {
    let list = classify_list_rect();
    Rect::new(
        list.x + 6.0,
        list.y + 6.0 + index as f32 * 58.0
            - app
                .classify_dialog
                .as_ref()
                .map_or(0.0, |dialog| dialog.scroll),
        list.w - 12.0,
        52.0,
    )
}

pub(crate) fn classify_apply_rect() -> Rect {
    let rect = classify_dialog_rect();
    Rect::new(rect.x + rect.w - 262.0, rect.y + rect.h - 52.0, 148.0, 32.0)
}

pub(crate) fn classify_cancel_rect() -> Rect {
    let rect = classify_dialog_rect();
    Rect::new(rect.x + rect.w - 102.0, rect.y + rect.h - 52.0, 78.0, 32.0)
}

pub(crate) fn draw_classify_dialog(app: &AppState) {
    let Some(dialog) = app.classify_dialog.as_ref() else {
        return;
    };
    draw_modal_backdrop();
    let rect = classify_dialog_rect();
    draw_panel_rect(&app.ui_font, rect, Some("Classify Elements"));
    ui_text(
        &app.ui_font,
        "Max object size (units)",
        rect.x + 24.0,
        rect.y + 94.0,
        WHITE,
    );
    let input = classify_size_rect();
    draw_rrect_bordered(
        input.x,
        input.y,
        input.w,
        input.h,
        7.0,
        1.0,
        ui_input_bg(),
        if dialog.error.is_some() {
            RED
        } else {
            ui_accent()
        },
    );
    ui_text(
        &app.ui_font,
        &dialog.size,
        input.x + 10.0,
        input.y + 21.0,
        WHITE,
    );
    if (get_time() * 2.0) as i32 % 2 == 0 {
        let cursor = clamp_char_boundary(&dialog.size, dialog.size_cursor);
        let prefix = &dialog.size[..cursor];
        let caret_x = input.x + 10.0 + ui_text_width(prefix, 16);
        draw_line(caret_x, input.y + 7.0, caret_x, input.y + 25.0, 1.0, WHITE);
    }
    draw_dialog_button(
        &app.ui_font,
        classify_review_rect(),
        if dialog.scanning {
            "Reviewing..."
        } else {
            "Review"
        },
        false,
    );
    let blocked = dialog
        .candidates
        .iter()
        .filter(|candidate| candidate.blocked_reason.is_some())
        .count();
    let selected = dialog
        .candidates
        .iter()
        .filter(|candidate| candidate.selected && candidate.blocked_reason.is_none())
        .count();
    ui_text_size(
        &app.ui_font,
        &format!(
            "{}{} element(s) scanned, {} to retag, {} skipped, {} selected",
            if dialog.scanning { "Scanning: " } else { "" },
            dialog.scanned,
            dialog.candidates.len(),
            blocked,
            selected
        ),
        rect.x + 468.0,
        rect.y + 93.0,
        13,
        ui_muted(),
    );

    let list = classify_list_rect();
    draw_rrect_bordered(
        list.x,
        list.y,
        list.w,
        list.h,
        8.0,
        1.0,
        ui_input_bg(),
        ui_border(),
    );
    begin_ui_clip(list);
    for (index, candidate) in dialog.candidates.iter().enumerate() {
        let row = classify_candidate_rect(app, index);
        if row.y + row.h < list.y || row.y > list.y + list.h {
            continue;
        }
        draw_rrect(
            row.x,
            row.y,
            row.w,
            row.h,
            6.0,
            Color::new(0.085, 0.09, 0.10, 1.0),
        );
        let checkbox = Rect::new(row.x + 10.0, row.y + 8.0, 24.0, 24.0);
        draw_checkbox(
            &app.ui_font,
            checkbox,
            "",
            candidate.selected && candidate.blocked_reason.is_none(),
        );
        ui_text(
            &app.ui_font,
            &format!(
                "{}  {} -> {}",
                candidate.id, candidate.current_tag, candidate.target_tag
            ),
            row.x + 44.0,
            row.y + 22.0,
            WHITE,
        );
        let size = candidate.size.map_or_else(
            || "model not loaded".to_string(),
            |size| format!("{size:.1} units"),
        );
        let detail = if let Some(reason) = candidate.blocked_reason.as_deref() {
            format!("{size} | {} | skipped: {reason}", candidate.reason.label())
        } else {
            format!("{size} | {}", candidate.reason.label())
        };
        ui_text_size(
            &app.ui_font,
            &detail,
            row.x + 44.0,
            row.y + 42.0,
            13,
            if candidate.blocked_reason.is_some() {
                Color::new(1.0, 0.55, 0.4, 1.0)
            } else {
                ui_muted()
            },
        );
    }
    if dialog.candidates.is_empty() && !dialog.scanning {
        ui_text(
            &app.ui_font,
            "Every element already matches this classification.",
            list.x + 18.0,
            list.y + 34.0,
            ui_muted(),
        );
    }
    end_ui_clip();
    if let Some(error) = dialog.error.as_deref() {
        ui_text_size(
            &app.ui_font,
            error,
            rect.x + 24.0,
            rect.y + rect.h - 28.0,
            13,
            Color::new(1.0, 0.48, 0.48, 1.0),
        );
    } else {
        ui_text_size(
            &app.ui_font,
            "Physics elements always stay objects; everything else larger than the size becomes a building. Scenery and LOD elements keep their type.",
            rect.x + 24.0,
            rect.y + rect.h - 28.0,
            13,
            ui_muted(),
        );
    }
    draw_dialog_button(
        &app.ui_font,
        classify_apply_rect(),
        if dialog.scanning {
            "Scanning..."
        } else {
            "Classify Selected"
        },
        true,
    );
    draw_dialog_button(&app.ui_font, classify_cancel_rect(), "Cancel", false);
}

fn wrap_save_log_line(line: &str, max_chars: usize) -> Vec<String> {
    let mut rows = Vec::new();
    for source in line.split('\n') {
        if source.is_empty() {
            rows.push(String::new());
            continue;
        }
        let mut current = String::new();
        for word in source.split_whitespace() {
            let extra = if current.is_empty() { 0 } else { 1 };
            if !current.is_empty() && current.len() + extra + word.len() > max_chars {
                rows.push(current);
                current = format!("  {word}");
            } else {
                if !current.is_empty() {
                    current.push(' ');
                }
                current.push_str(word);
            }
        }
        if !current.is_empty() {
            rows.push(current);
        }
    }
    rows
}

pub(crate) fn blender_import_display_rows(app: &AppState, list_width: f32) -> Vec<String> {
    let max_chars = ((list_width - 28.0) / 8.0).max(16.0) as usize;
    let mut rows = Vec::new();
    for entry in &app.blender_import_log {
        rows.extend(wrap_save_log_line(entry, max_chars));
    }
    rows
}

pub(crate) fn save_log_display_rows(app: &AppState, list_width: f32) -> Vec<String> {
    let max_chars = ((list_width - 32.0) / 8.0).max(16.0) as usize;
    let mut rows = Vec::new();
    for entry in &app.activity_log {
        rows.extend(wrap_save_log_line(entry, max_chars));
    }
    rows
}

pub(crate) fn draw_load_dialog(app: &AppState) {
    let Some(dialog) = app.load_dialog.as_ref() else {
        return;
    };
    draw_modal_backdrop();
    let rect = load_dialog_rect();
    draw_panel_rect(&app.ui_font, rect, Some("Load Resource"));
    ui_text(
        &app.ui_font,
        "Load another Eagle resource folder into the editor.",
        rect.x + 24.0,
        rect.y + 58.0,
        LIGHTGRAY,
    );
    let input = load_input_rect();
    draw_rrect_bordered(
        input.x,
        input.y,
        input.w,
        input.h,
        7.0,
        1.0,
        Color::new(0.055, 0.064, 0.078, 1.0),
        ui_accent(),
    );
    let max_chars = ((input.w - 20.0) / 8.5).max(1.0) as usize;
    let cursor = clamp_char_boundary(&dialog.path, dialog.cursor);
    let cursor_char = dialog.path[..cursor].chars().count();
    let total_chars = dialog.path.chars().count();
    let start_char = cursor_char.saturating_sub(max_chars.saturating_sub(1));
    let end_char = (start_char + max_chars).min(total_chars);
    let visible: String = dialog
        .path
        .chars()
        .skip(start_char)
        .take(end_char - start_char)
        .collect();
    let caret_prefix: String = visible
        .chars()
        .take(cursor_char.saturating_sub(start_char))
        .collect();
    let text_x = input.x + 10.0;
    draw_visible_text_selection(
        &dialog.path,
        cursor,
        dialog.selection_anchor,
        start_char,
        &visible,
        text_x,
        input,
    );
    ui_text(&app.ui_font, &visible, text_x, input.y + 21.0, WHITE);
    if (get_time() * 2.0) as i32 % 2 == 0 {
        let caret_x = text_x + ui_text_width(&caret_prefix, 16).round();
        draw_line(
            caret_x,
            input.y + 7.0,
            caret_x,
            input.y + input.h - 7.0,
            1.0,
            WHITE,
        );
    }
    ui_text(
        &app.ui_font,
        "Enter loads   Esc cancels   Relative paths start at the current directory",
        rect.x + 24.0,
        rect.y + 128.0,
        ui_muted(),
    );
    draw_dialog_button(
        &app.ui_font,
        Rect::new(rect.x + rect.w - 216.0, rect.y + rect.h - 50.0, 88.0, 32.0),
        "Load",
        true,
    );
    draw_dialog_button(
        &app.ui_font,
        Rect::new(rect.x + rect.w - 116.0, rect.y + rect.h - 50.0, 88.0, 32.0),
        "Cancel",
        false,
    );
}

pub(crate) struct ImportAssetDialogLayout {
    pub(crate) rect: Rect,
    pub(crate) id: Rect,
    pub(crate) browse: Rect,
    pub(crate) import: Rect,
    pub(crate) cancel: Rect,
}

pub(crate) fn import_asset_dialog_layout() -> ImportAssetDialogLayout {
    let w = 680.0_f32.min(screen_width() - 48.0);
    let rect = Rect::new(
        (screen_width() - w) * 0.5,
        screen_height() * 0.5 - 150.0,
        w,
        300.0,
    );
    ImportAssetDialogLayout {
        id: Rect::new(rect.x + 24.0, rect.y + 100.0, rect.w - 48.0, 34.0),
        browse: Rect::new(rect.x + rect.w - 144.0, rect.y + 184.0, 120.0, 32.0),
        import: Rect::new(rect.x + rect.w - 224.0, rect.y + rect.h - 50.0, 96.0, 32.0),
        cancel: Rect::new(rect.x + rect.w - 116.0, rect.y + rect.h - 50.0, 92.0, 32.0),
        rect,
    }
}

pub(crate) fn draw_import_asset_dialog(app: &AppState) {
    let Some(dialog) = app.import_asset_dialog.as_ref() else {
        return;
    };
    draw_modal_backdrop();
    let layout = import_asset_dialog_layout();
    draw_panel_rect(&app.ui_font, layout.rect, Some("Import new asset"));
    ui_text(
        &app.ui_font,
        "DFF",
        layout.rect.x + 24.0,
        layout.rect.y + 58.0,
        ui_dim(),
    );
    ui_text(
        &app.ui_font,
        &ellipsize(dialog.dff_path.to_string_lossy().as_ref(), 74),
        layout.rect.x + 74.0,
        layout.rect.y + 58.0,
        LIGHTGRAY,
    );
    ui_text(
        &app.ui_font,
        "Asset ID",
        layout.id.x,
        layout.id.y - 10.0,
        ui_dim(),
    );
    draw_rrect_bordered(
        layout.id.x,
        layout.id.y,
        layout.id.w,
        layout.id.h,
        7.0,
        1.0,
        Color::new(0.055, 0.064, 0.078, 1.0),
        ui_accent(),
    );
    ui_text(
        &app.ui_font,
        &ellipsize_width(&dialog.id, 16, layout.id.w - 20.0),
        layout.id.x + 10.0,
        layout.id.y + 23.0,
        WHITE,
    );
    ui_text(
        &app.ui_font,
        "Texture folder",
        layout.rect.x + 24.0,
        layout.rect.y + 166.0,
        ui_dim(),
    );
    ui_text(
        &app.ui_font,
        &ellipsize(dialog.texture_dir.to_string_lossy().as_ref(), 62),
        layout.rect.x + 24.0,
        layout.rect.y + 205.0,
        LIGHTGRAY,
    );
    draw_dialog_button(&app.ui_font, layout.browse, "Choose folder", false);
    draw_dialog_button(&app.ui_font, layout.import, "Import", true);
    draw_dialog_button(&app.ui_font, layout.cancel, "Cancel", false);
}

fn draw_preferences_path_input(
    app: &AppState,
    dialog: &PreferencesDialog,
    field: PreferencesPathField,
    value: &str,
    input: Rect,
) {
    let active = dialog.active_path_field == field;
    draw_rrect_bordered(
        input.x,
        input.y,
        input.w,
        input.h,
        7.0,
        1.0,
        Color::new(0.055, 0.064, 0.078, 1.0),
        if active { ui_accent() } else { ui_border() },
    );
    let max_chars = ((input.w - 20.0) / 8.5).max(1.0) as usize;
    let cursor = if active {
        clamp_char_boundary(value, dialog.cursor)
    } else {
        0
    };
    let cursor_char = value[..cursor].chars().count();
    let total_chars = value.chars().count();
    let start_char = cursor_char.saturating_sub(max_chars.saturating_sub(1));
    let end_char = (start_char + max_chars).min(total_chars);
    let visible: String = value
        .chars()
        .skip(start_char)
        .take(end_char - start_char)
        .collect();
    let caret_prefix: String = visible
        .chars()
        .take(cursor_char.saturating_sub(start_char))
        .collect();
    let text_x = input.x + 10.0;
    draw_visible_text_selection(
        value,
        cursor,
        if active {
            dialog.selection_anchor
        } else {
            None
        },
        start_char,
        &visible,
        text_x,
        input,
    );
    ui_text(&app.ui_font, &visible, text_x, input.y + 21.0, WHITE);
    if active && (get_time() * 2.0) as i32 % 2 == 0 {
        let caret_x = text_x + ui_text_width(&caret_prefix, 16).round();
        draw_line(
            caret_x,
            input.y + 7.0,
            caret_x,
            input.y + input.h - 7.0,
            1.0,
            WHITE,
        );
    }
}

pub(crate) fn draw_preferences_dialog(app: &AppState) {
    let Some(dialog) = app.preferences_dialog.as_ref() else {
        return;
    };
    draw_modal_backdrop();
    let rect = preferences_dialog_rect();
    draw_panel_rect(&app.ui_font, rect, Some("Preferences"));
    for (tab, label) in [
        (PreferencesTab::General, "General"),
        (PreferencesTab::Viewport, "Viewport"),
        (PreferencesTab::Plugins, "Plugins"),
    ] {
        draw_dialog_button(
            &app.ui_font,
            preferences_tab_rect(tab),
            label,
            dialog.tab == tab,
        );
    }
    match dialog.tab {
        PreferencesTab::General => {
            ui_text(
                &app.ui_font,
                "GTA:SA Install Directory",
                rect.x + 24.0,
                rect.y + 122.0,
                ui_dim(),
            );
            draw_preferences_path_input(
                app,
                dialog,
                PreferencesPathField::GtaSa,
                &dialog.gta_sa_dir,
                preferences_input_rect(),
            );
            let gta_path = PathBuf::from(dialog.gta_sa_dir.trim());
            let ide_count = app.readonly_definition_ids.len();
            let has_models = gta_path.join("models").is_dir();
            let has_ide = gta_path.join("data").join("maps").is_dir();
            let status = if has_models && has_ide {
                format!("{ide_count} fallback IDE definition(s) loaded")
            } else {
                "Folder should contain data/maps and models".to_string()
            };
            ui_text(
                &app.ui_font,
                &status,
                rect.x + 24.0,
                rect.y + 191.0,
                ui_muted(),
            );
            ui_text(
                &app.ui_font,
                "Blender Install Directory (optional)",
                rect.x + 24.0,
                rect.y + 220.0,
                ui_dim(),
            );
            draw_preferences_path_input(
                app,
                dialog,
                PreferencesPathField::Blender,
                &dialog.blender_install_dir,
                preferences_blender_input_rect(),
            );
            draw_dialog_button(
                &app.ui_font,
                preferences_blender_search_rect(),
                "Search for Blender install",
                true,
            );
            let blender_status = if let Some(message) = dialog.blender_search_message.as_ref() {
                message.clone()
            } else if dialog.blender_install_dir.trim().is_empty() {
                "Leave blank to use BLENDER_PATH and automatic discovery".to_string()
            } else if blender_executable_in_install_dir(Path::new(
                dialog.blender_install_dir.trim(),
            ))
            .is_some()
            {
                "Blender executable found".to_string()
            } else {
                "Folder should contain the Blender executable".to_string()
            };
            ui_text(
                &app.ui_font,
                &blender_status,
                rect.x + 24.0,
                rect.y + 295.0,
                ui_muted(),
            );
            ui_text(
                &app.ui_font,
                "Enter saves   Esc cancels   Reload the resource after changing the GTA:SA path",
                rect.x + 24.0,
                rect.y + 324.0,
                ui_muted(),
            );
            draw_dialog_button(
                &app.ui_font,
                preferences_cleanup_autosaves_rect(),
                "Clean Up Autosaves",
                app.autosave_cleanup_rx.is_none() && app.autosave_rx.is_none(),
            );
        }
        PreferencesTab::Viewport => draw_preferences_viewport_section(app, dialog, rect),
        PreferencesTab::Plugins => draw_preferences_plugins_section(app, dialog),
    }
    if let Some(error) = &dialog.error {
        ui_text(
            &app.ui_font,
            &ellipsize_width(error, 16, rect.w - 48.0),
            rect.x + 24.0,
            rect.y + rect.h - 72.0,
            Color::new(1.0, 0.45, 0.35, 1.0),
        );
    }
    draw_dialog_button(
        &app.ui_font,
        Rect::new(rect.x + rect.w - 216.0, rect.y + rect.h - 50.0, 88.0, 32.0),
        "Save",
        true,
    );
    draw_dialog_button(
        &app.ui_font,
        Rect::new(rect.x + rect.w - 116.0, rect.y + rect.h - 50.0, 88.0, 32.0),
        "Cancel",
        false,
    );
    if !dialog.blender_install_candidates.is_empty() {
        let count = dialog.blender_install_candidates.len();
        let chooser = preferences_blender_candidate_dialog_rect(count);
        draw_rrect(
            chooser.x + 3.0,
            chooser.y + 4.0,
            chooser.w,
            chooser.h,
            10.0,
            Color::new(0.0, 0.0, 0.0, 0.38),
        );
        draw_panel_rect(&app.ui_font, chooser, Some("Select Blender Installation"));
        let start = dialog.blender_candidate_scroll;
        let visible = count.min(BLENDER_CANDIDATE_VISIBLE_ROWS);
        for slot in 0..visible {
            let Some(path) = dialog.blender_install_candidates.get(start + slot) else {
                break;
            };
            let row = preferences_blender_candidate_row_rect(count, slot);
            let hovered = row.contains(mouse_position().into());
            draw_rrect_bordered(
                row.x,
                row.y,
                row.w,
                row.h,
                7.0,
                1.0,
                if hovered {
                    ui_surface_hover()
                } else {
                    ui_surface()
                },
                if hovered { ui_accent() } else { ui_border() },
            );
            ui_text(
                &app.ui_font,
                &ellipsize_width(&path.to_string_lossy(), 16, row.w - 20.0),
                row.x + 10.0,
                row.y + 21.0,
                WHITE,
            );
        }
        if count > BLENDER_CANDIDATE_VISIBLE_ROWS {
            ui_text(
                &app.ui_font,
                &format!(
                    "Showing {}–{} of {} • scroll for more",
                    start + 1,
                    (start + visible).min(count),
                    count
                ),
                chooser.x + 18.0,
                chooser.y + chooser.h - 20.0,
                ui_muted(),
            );
        }
        draw_dialog_button(
            &app.ui_font,
            preferences_blender_candidate_cancel_rect(count),
            "Cancel",
            false,
        );
    }
}

pub(crate) fn preferences_plugin_detail_lines(dialog: &PreferencesDialog) -> Vec<String> {
    let Some(plugin) = dialog
        .selected_plugin
        .and_then(|index| dialog.plugins.get(index))
    else {
        return Vec::new();
    };
    let mut details = vec![
        plugin.name.clone(),
        format!("ID: {}", plugin.id),
        format!(
            "Status: {}",
            if plugin.enabled {
                "Enabled"
            } else {
                "Disabled"
            }
        ),
    ];
    if let Some(error) = &plugin.error {
        details.push(format!("Unavailable: {error}"));
    }
    details.extend(plugin.details.iter().cloned());
    details.push(format!("Location: {}", plugin.folder.display()));
    let width = (preferences_plugin_details_rect().w - 28.0).max(1.0);
    details
        .iter()
        .flat_map(|text| wrap_text_width(text, 16, width))
        .flat_map(|line| {
            // Folder paths and manifest IDs can contain no spaces. Keep every
            // character readable within the details pane instead of clipping.
            let mut lines = Vec::new();
            let mut current = String::new();
            for ch in line.chars() {
                let mut next = current.clone();
                next.push(ch);
                if !current.is_empty() && ui_text_width(&next, 16) > width {
                    lines.push(std::mem::take(&mut current));
                }
                current.push(ch);
            }
            if !current.is_empty() {
                lines.push(current);
            }
            lines
        })
        .collect()
}

fn draw_preferences_plugins_section(app: &AppState, dialog: &PreferencesDialog) {
    let rect = preferences_dialog_rect();
    ui_text(
        &app.ui_font,
        "Installed plugins",
        rect.x + 24.0,
        rect.y + 122.0,
        WHITE,
    );
    let list = preferences_plugin_list_rect();
    let details = preferences_plugin_details_rect();
    for panel in [list, details] {
        draw_rrect_bordered(
            panel.x,
            panel.y,
            panel.w,
            panel.h,
            7.0,
            1.0,
            ui_surface(),
            ui_border(),
        );
    }
    begin_ui_clip(list);
    if dialog.plugins.is_empty() {
        ui_text(
            &app.ui_font,
            "No plugins installed",
            list.x + 12.0,
            list.y + 26.0,
            ui_muted(),
        );
    }
    for (index, plugin) in dialog.plugins.iter().enumerate() {
        let row = preferences_plugin_row_rect(index, dialog.plugin_scroll);
        if row.y + row.h < list.y || row.y > list.y + list.h {
            continue;
        }
        let selected = dialog.selected_plugin == Some(index);
        draw_rrect_bordered(
            row.x,
            row.y,
            row.w,
            row.h,
            6.0,
            1.0,
            if selected {
                ui_surface_hover()
            } else {
                ui_surface()
            },
            if selected { ui_accent() } else { ui_border() },
        );
        ui_text(
            &app.ui_font,
            &ellipsize_width(&plugin.name, 16, row.w - 20.0),
            row.x + 8.0,
            row.y + 21.0,
            WHITE,
        );
        let state = if plugin.error.is_some() {
            "Unavailable"
        } else if plugin.enabled {
            "Enabled"
        } else {
            "Disabled"
        };
        ui_text(
            &app.ui_font,
            &ellipsize_width(state, 16, row.w - 80.0),
            row.x + 8.0,
            row.y + 47.0,
            ui_muted(),
        );
        draw_dialog_button(
            &app.ui_font,
            preferences_plugin_toggle_rect(index, dialog.plugin_scroll),
            if plugin.enabled { "On" } else { "Off" },
            plugin.enabled,
        );
    }
    end_ui_clip();
    begin_ui_clip(details);
    let lines = preferences_plugin_detail_lines(dialog);
    if lines.is_empty() {
        ui_text(
            &app.ui_font,
            "Select a plugin to see its details",
            details.x + 12.0,
            details.y + 26.0,
            ui_muted(),
        );
    }
    for (index, line) in lines.iter().enumerate() {
        ui_text(
            &app.ui_font,
            line,
            details.x + 12.0,
            details.y + 26.0 + index as f32 * 24.0 - dialog.plugin_details_scroll,
            if index == 0 { WHITE } else { ui_dim() },
        );
    }
    end_ui_clip();
    draw_preferences_scrollbar(
        list,
        dialog.plugin_scroll,
        (dialog.plugins.len() as f32 * PREFERENCES_PLUGIN_ROW_H - list.h).max(0.0),
    );
    draw_preferences_scrollbar(
        details,
        dialog.plugin_details_scroll,
        (lines.len() as f32 * 24.0 + 20.0 - details.h).max(0.0),
    );
    ui_text(
        &app.ui_font,
        "Plugin changes take effect when you save.",
        rect.x + 24.0,
        rect.y + rect.h - 78.0,
        ui_muted(),
    );
}

fn draw_preferences_scrollbar(clip: Rect, scroll: f32, max_scroll: f32) {
    if max_scroll <= 0.0 {
        return;
    }
    let track = Rect::new(
        clip.x + clip.w - 7.0,
        clip.y + 4.0,
        3.0,
        (clip.h - 8.0).max(1.0),
    );
    draw_rrect(track.x, track.y, track.w, track.h, 1.5, ui_border());
    let thumb_h = (track.h * clip.h / (clip.h + max_scroll))
        .max(24.0)
        .min(track.h);
    let thumb_y = track.y + (track.h - thumb_h) * scroll / max_scroll;
    draw_rrect(track.x, thumb_y, track.w, thumb_h, 1.5, ui_muted());
}

/// Read-only value chip sitting between a stepper's - and + buttons.
fn draw_preferences_value_chip(app: &AppState, rect: Rect, value: &str) {
    draw_rrect_bordered(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        7.0,
        1.0,
        Color::new(0.055, 0.064, 0.078, 1.0),
        ui_border(),
    );
    let width = ui_text_width(value, 16);
    ui_text(
        &app.ui_font,
        value,
        rect.x + (rect.w - width) * 0.5,
        rect.y + 20.0,
        WHITE,
    );
}

fn draw_preferences_stepper(
    app: &AppState,
    scroll: f32,
    row: usize,
    label: &str,
    value: &str,
    can_decrease: bool,
    can_increase: bool,
) {
    let (label_x, row_y) = preferences_row_origin(row, scroll);
    ui_text(&app.ui_font, label, label_x, row_y + 20.0, ui_dim());
    let (minus, chip, plus) = preferences_stepper_rects(row, scroll);
    draw_dialog_button(&app.ui_font, minus, "-", can_decrease);
    draw_preferences_value_chip(app, chip, value);
    draw_dialog_button(&app.ui_font, plus, "+", can_increase);
}

fn draw_preferences_viewport_section(app: &AppState, dialog: &PreferencesDialog, rect: Rect) {
    let scroll = dialog
        .viewport_scroll
        .clamp(0.0, preferences_viewport_max_scroll());
    let clip = preferences_viewport_clip_rect();
    begin_ui_clip(clip);
    ui_text(
        &app.ui_font,
        "Viewport",
        rect.x + 24.0,
        rect.y + 122.0 - scroll,
        WHITE,
    );
    draw_preferences_stepper(
        app,
        scroll,
        0,
        "Gimbal Size",
        &format!("{:.2}x", dialog.gizmo_scale),
        dialog.gizmo_scale > MIN_GIZMO_SCALE,
        dialog.gizmo_scale < MAX_GIZMO_SCALE,
    );
    draw_preferences_stepper(
        app,
        scroll,
        1,
        "World Camera Speed",
        &format!("{:.0}", dialog.camera_speed),
        dialog.camera_speed > MIN_CAMERA_SPEED,
        dialog.camera_speed < MAX_CAMERA_SPEED,
    );
    draw_preferences_stepper(
        app,
        scroll,
        2,
        "Vehicle Camera Speed",
        &format!("{:.0}", dialog.vehicle_camera_speed),
        dialog.vehicle_camera_speed > MIN_DETAIL_CAMERA_SPEED,
        dialog.vehicle_camera_speed < MAX_CAMERA_SPEED,
    );
    draw_preferences_stepper(
        app,
        scroll,
        3,
        "Editing Camera Speed",
        &format!("{:.0}", dialog.editing_camera_speed),
        dialog.editing_camera_speed > MIN_EDITING_CAMERA_SPEED,
        dialog.editing_camera_speed < MAX_CAMERA_SPEED,
    );
    draw_preferences_stepper(
        app,
        scroll,
        4,
        "Camera Rotation Speed",
        &format!("{:.2}x", dialog.camera_rotation_speed),
        dialog.camera_rotation_speed > MIN_CAMERA_ROTATION_SPEED,
        dialog.camera_rotation_speed < MAX_CAMERA_ROTATION_SPEED,
    );
    let (label_x, msaa_y) = preferences_row_origin(PREFERENCES_MSAA_ROW, scroll);
    ui_text(
        &app.ui_font,
        "Anti-aliasing",
        label_x,
        msaa_y + 20.0,
        ui_dim(),
    );
    draw_dialog_button(
        &app.ui_font,
        preferences_msaa_rect(scroll),
        msaa_samples_label(dialog.msaa_samples),
        clamp_msaa_samples(dialog.msaa_samples) != 1,
    );
    if clamp_msaa_samples(dialog.msaa_samples) != clamp_msaa_samples(dialog.msaa_samples_saved) {
        ui_text(
            &app.ui_font,
            "Takes effect after restarting the editor",
            label_x,
            msaa_y + 52.0,
            ui_muted(),
        );
    }
    draw_preferences_stepper(
        app,
        scroll,
        PREFERENCES_DRAW_DISTANCE_ROW,
        "Draw Distance",
        &if dialog.draw_distance_percent == 0 {
            "Disabled".to_string()
        } else {
            format!("{}%", dialog.draw_distance_percent)
        },
        dialog.draw_distance_percent > 0,
        dialog.draw_distance_percent < MAX_DRAW_DISTANCE_PERCENT,
    );
    end_ui_clip();

    let max_scroll = preferences_viewport_max_scroll();
    if max_scroll > 0.0 {
        let track = Rect::new(clip.x + clip.w - 8.0, clip.y + 4.0, 4.0, clip.h - 8.0);
        draw_rrect(track.x, track.y, track.w, track.h, 2.0, ui_border());
        let thumb_h = (track.h * clip.h / (clip.h + max_scroll))
            .max(24.0)
            .min(track.h);
        let thumb_y = track.y + (track.h - thumb_h) * scroll / max_scroll;
        draw_rrect(track.x, thumb_y, track.w, thumb_h, 2.0, ui_muted());
    }
}

pub(crate) fn draw_dff_replace_choice_dialog(app: &AppState) {
    let Some(dialog) = app.dff_replace_choice_dialog.as_ref() else {
        return;
    };
    let label = match dialog.kind {
        ReplacementAssetKind::Dff => "DFF",
        ReplacementAssetKind::Col => "COL",
    };
    let title = format!("Replace {label}");
    draw_modal_backdrop();
    let rect = dff_replace_choice_dialog_rect();
    draw_panel_rect(&app.ui_font, rect, Some(&title));
    ui_text(
        &app.ui_font,
        "Replace every instance of this model, or make the selected element unique?",
        rect.x + 24.0,
        rect.y + 58.0,
        WHITE,
    );
    ui_text(
        &app.ui_font,
        &ellipsize(dialog.path.to_string_lossy().as_ref(), 66),
        rect.x + 24.0,
        rect.y + 86.0,
        LIGHTGRAY,
    );
    ui_text(
        &app.ui_font,
        "Instance updates the shared definition. Unique creates a new definition for this element.",
        rect.x + 24.0,
        rect.y + 112.0,
        ui_muted(),
    );
    draw_dialog_button(
        &app.ui_font,
        Rect::new(rect.x + rect.w - 358.0, rect.y + rect.h - 50.0, 110.0, 32.0),
        "Instance",
        true,
    );
    draw_dialog_button(
        &app.ui_font,
        Rect::new(rect.x + rect.w - 236.0, rect.y + rect.h - 50.0, 120.0, 32.0),
        "Make Unique",
        false,
    );
    draw_dialog_button(
        &app.ui_font,
        Rect::new(rect.x + rect.w - 104.0, rect.y + rect.h - 50.0, 76.0, 32.0),
        "Cancel",
        false,
    );
}

pub(crate) fn draw_element_id_rename_dialog(app: &AppState) {
    let Some(dialog) = app.element_id_rename_dialog.as_ref() else {
        return;
    };
    draw_modal_backdrop();
    let rect = element_id_rename_dialog_rect();
    draw_panel_rect(&app.ui_font, rect, Some("Change Element ID"));
    ui_text(
        &app.ui_font,
        &format!("Change ID {} -> {}", dialog.old_id, dialog.new_id),
        rect.x + 24.0,
        rect.y + 58.0,
        WHITE,
    );
    ui_text(
        &app.ui_font,
        "Choose how DFF/COL references should behave after the ID changes.",
        rect.x + 24.0,
        rect.y + 86.0,
        LIGHTGRAY,
    );
    ui_text(
        &app.ui_font,
        if dialog.selected_indices.len() > 1 {
            "Rename/keep updates the shared model. Unique changes only the selected elements."
        } else {
            "Rename assets points DFF/COL at the new ID. Keep old assets points them at the old ID. Unique changes only this element."
        },
        rect.x + 24.0,
        rect.y + 114.0,
        ui_muted(),
    );
    draw_dialog_button(
        &app.ui_font,
        Rect::new(rect.x + rect.w - 496.0, rect.y + rect.h - 50.0, 128.0, 32.0),
        "Rename assets",
        true,
    );
    draw_dialog_button(
        &app.ui_font,
        Rect::new(rect.x + rect.w - 356.0, rect.y + rect.h - 50.0, 128.0, 32.0),
        "Keep old assets",
        false,
    );
    draw_dialog_button(
        &app.ui_font,
        Rect::new(rect.x + rect.w - 216.0, rect.y + rect.h - 50.0, 116.0, 32.0),
        "Make unique",
        false,
    );
    draw_dialog_button(
        &app.ui_font,
        Rect::new(rect.x + rect.w - 88.0, rect.y + rect.h - 50.0, 64.0, 32.0),
        "Cancel",
        false,
    );
}

pub(crate) fn draw_element_replace_with_dialog(app: &AppState) {
    let Some(dialog) = app.element_replace_with_dialog.as_ref() else {
        return;
    };
    if dialog.picking_scene {
        let banner = Rect::new((screen_width() - 520.0) * 0.5, TOP_H + 14.0, 520.0, 48.0);
        draw_rrect_bordered(
            banner.x,
            banner.y,
            banner.w,
            banner.h,
            10.0,
            1.0,
            ui_surface(),
            ui_accent(),
        );
        ui_text(
            &app.ui_font,
            "Click the replacement element in the scene  |  Esc to return",
            banner.x + 18.0,
            banner.y + 29.0,
            WHITE,
        );
        return;
    }
    draw_modal_backdrop();
    let rect = element_replace_with_dialog_rect();
    draw_panel_rect(&app.ui_font, rect, Some("Replace with Model"));
    ui_text(
        &app.ui_font,
        &format!(
            "Replace {} selected element(s) with an existing model ID.",
            dialog.source_indices.len()
        ),
        rect.x + 24.0,
        rect.y + 60.0,
        LIGHTGRAY,
    );
    let search = element_replace_with_search_rect();
    draw_rrect_bordered(
        search.x,
        search.y,
        search.w,
        search.h,
        8.0,
        1.0,
        ui_input_bg(),
        ui_accent(),
    );
    let search_text = if dialog.search.is_empty() {
        "Search by ID..."
    } else {
        dialog.search.as_str()
    };
    ui_text(
        &app.ui_font,
        &ellipsize_width(search_text, 16, search.w - 20.0),
        search.x + 10.0,
        search.y + 23.0,
        if dialog.search.is_empty() {
            ui_muted()
        } else {
            WHITE
        },
    );
    let options = element_replace_target_ids(app, &dialog.search);
    let start = (dialog.scroll.floor() as usize).min(options.len().saturating_sub(1));
    for row in 0..8 {
        let Some(id) = options.get(start + row) else {
            break;
        };
        let row_rect = element_replace_with_row_rect(row);
        let selected = dialog.selected_target.as_deref() == Some(id.as_str());
        if selected || row_rect.contains(mouse_position().into()) {
            draw_rrect(
                row_rect.x,
                row_rect.y,
                row_rect.w,
                row_rect.h,
                7.0,
                if selected {
                    ui_accent_soft()
                } else {
                    ui_surface_hover()
                },
            );
        }
        let instances = app
            .placements
            .iter()
            .filter(|placement| placement.id.eq_ignore_ascii_case(id))
            .count();
        ui_text(
            &app.ui_font,
            id,
            row_rect.x + 10.0,
            row_rect.y + 20.0,
            WHITE,
        );
        let count = format!(
            "{instances} instance{}",
            if instances == 1 { "" } else { "s" }
        );
        let count_w = ui_text_width(&count, 16);
        ui_text(
            &app.ui_font,
            &count,
            row_rect.x + row_rect.w - count_w - 10.0,
            row_rect.y + 20.0,
            ui_muted(),
        );
    }
    draw_dialog_button(
        &app.ui_font,
        element_replace_with_pick_rect(),
        "Pick in scene",
        false,
    );
    draw_dialog_button(
        &app.ui_font,
        element_replace_with_apply_rect(),
        "Replace",
        true,
    );
    draw_dialog_button(
        &app.ui_font,
        element_replace_with_cancel_rect(),
        "Cancel",
        false,
    );
}

pub(crate) fn draw_missing_texture_dialog(app: &AppState) {
    let Some(dialog) = app.missing_texture_dialog.as_ref() else {
        return;
    };
    draw_modal_backdrop();
    let rect = missing_texture_dialog_rect();
    draw_panel_rect(&app.ui_font, rect, Some("Find Missing Textures"));
    ui_text(
        &app.ui_font,
        &format!(
            "{} is missing on {}",
            dialog.choice.texture_name, dialog.choice.definition_id
        ),
        rect.x + 24.0,
        rect.y + 58.0,
        LIGHTGRAY,
    );
    ui_text(
        &app.ui_font,
        "Choose which TXD should supply this texture.",
        rect.x + 24.0,
        rect.y + 84.0,
        ui_muted(),
    );
    for (idx, candidate) in dialog.candidates.iter().take(5).enumerate() {
        let row = missing_texture_candidate_rect(idx);
        let hovered = row.contains(mouse_position().into());
        draw_rrect_bordered(
            row.x,
            row.y,
            row.w,
            row.h,
            7.0,
            1.0,
            if hovered {
                ui_surface_hover()
            } else {
                Color::new(0.070, 0.082, 0.100, 1.0)
            },
            if hovered { ui_accent() } else { ui_border() },
        );
        let thumb = Rect::new(row.x + 8.0, row.y + 5.0, 40.0, 40.0);
        draw_rrect_bordered(
            thumb.x,
            thumb.y,
            thumb.w,
            thumb.h,
            5.0,
            1.0,
            Color::new(0.035, 0.040, 0.050, 1.0),
            ui_border(),
        );
        if let Some(texture) = candidate.thumbnail.as_ref() {
            let size = texture.size();
            let scale = (thumb.w / size.x).min(thumb.h / size.y).min(1.0);
            let w = (size.x * scale).max(1.0);
            let h = (size.y * scale).max(1.0);
            draw_texture_ex(
                texture,
                thumb.x + (thumb.w - w) * 0.5,
                thumb.y + (thumb.h - h) * 0.5,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(w, h)),
                    ..Default::default()
                },
            );
        }
        ui_text(
            &app.ui_font,
            &ellipsize_width(
                &if !candidate
                    .source_texture_name
                    .eq_ignore_ascii_case(&dialog.choice.texture_name)
                {
                    format!(
                        "{}  ·  source '{}'{}",
                        candidate.txd_name,
                        candidate.source_texture_name,
                        if candidate.duplicate_count > 1 {
                            format!("  (+{} identical matches)", candidate.duplicate_count - 1)
                        } else {
                            String::new()
                        }
                    )
                } else if candidate.duplicate_count > 1 {
                    format!(
                        "{}  (+{} identical matches)",
                        candidate.txd_name,
                        candidate.duplicate_count - 1
                    )
                } else {
                    candidate.txd_name.clone()
                },
                16,
                row.w - 220.0,
            ),
            row.x + 60.0,
            row.y + 22.0,
            WHITE,
        );
        ui_text(
            &app.ui_font,
            &format!(
                "{}x{}  {}",
                candidate.width,
                candidate.height,
                tx_format_label(candidate.format)
            ),
            row.x + 60.0,
            row.y + 42.0,
            ui_muted(),
        );
    }
    if dialog.candidates.len() > 5 {
        ui_text(
            &app.ui_font,
            &format!("{} more candidate(s) omitted", dialog.candidates.len() - 5),
            rect.x + 24.0,
            rect.y + 112.0 + 5.0 * 56.0 + 24.0,
            ui_muted(),
        );
    }
    draw_dialog_button(
        &app.ui_font,
        Rect::new(rect.x + rect.w - 116.0, rect.y + rect.h - 50.0, 88.0, 32.0),
        "Cancel",
        false,
    );
}

pub(crate) fn draw_texture_archive_dialog(app: &AppState) {
    let Some(dialog) = app.texture_archive_dialog.as_ref() else {
        return;
    };
    draw_modal_backdrop();
    let rect = texture_archive_dialog_rect();
    draw_panel_rect(&app.ui_font, rect, Some("Texture Archive"));
    ui_text(
        &app.ui_font,
        &format!("{} linked to {}", dialog.txd_name, dialog.definition_id),
        rect.x + 24.0,
        rect.y + 58.0,
        LIGHTGRAY,
    );
    ui_text(
        &app.ui_font,
        &format!("{} texture(s)", dialog.textures.len()),
        rect.x + 24.0,
        rect.y + 84.0,
        ui_muted(),
    );

    let visible_rows = ((rect.h - 180.0) / 38.0).floor().max(1.0) as usize;
    let start = dialog.scroll.floor() as usize;
    if dialog.textures.is_empty() {
        ui_text(
            &app.ui_font,
            "No textures were indexed in this TXD.",
            rect.x + 24.0,
            rect.y + 132.0,
            ui_muted(),
        );
    } else {
        for row in 0..visible_rows {
            let idx = start + row;
            let Some(entry) = dialog.textures.get(idx) else {
                break;
            };
            let row_rect = texture_archive_row_rect(row);
            let selected = idx == dialog.selected;
            let hovered = row_rect.contains(mouse_position().into());
            if selected || hovered {
                draw_rrect(
                    row_rect.x,
                    row_rect.y,
                    row_rect.w,
                    row_rect.h,
                    6.0,
                    if selected {
                        ui_surface_active()
                    } else {
                        ui_surface_hover()
                    },
                );
            }
            let thumb_rect = Rect::new(row_rect.x + 6.0, row_rect.y + 4.0, 26.0, 26.0);
            draw_rrect_bordered(
                thumb_rect.x,
                thumb_rect.y,
                thumb_rect.w,
                thumb_rect.h,
                4.0,
                1.0,
                Color::new(0.035, 0.040, 0.050, 1.0),
                ui_border(),
            );
            if let Some(texture) = entry.thumbnail.as_ref() {
                let size = texture.size();
                let scale = (thumb_rect.w / size.x).min(thumb_rect.h / size.y).min(1.0);
                let w = (size.x * scale).max(1.0);
                let h = (size.y * scale).max(1.0);
                draw_texture_ex(
                    texture,
                    thumb_rect.x + (thumb_rect.w - w) * 0.5,
                    thumb_rect.y + (thumb_rect.h - h) * 0.5,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(vec2(w, h)),
                        ..Default::default()
                    },
                );
            }
            ui_text(
                &app.ui_font,
                &ellipsize_width(&entry.name, 16, row_rect.w - 152.0),
                row_rect.x + 42.0,
                row_rect.y + 22.0,
                if selected { ui_accent() } else { WHITE },
            );
            ui_text(
                &app.ui_font,
                &format!("{}x{}", entry.width, entry.height),
                row_rect.x + row_rect.w - 86.0,
                row_rect.y + 22.0,
                ui_muted(),
            );
        }
    }

    let preview = texture_archive_preview_rect();
    draw_rrect_bordered(
        preview.x,
        preview.y,
        preview.w,
        preview.h,
        8.0,
        1.0,
        Color::new(0.045, 0.052, 0.064, 1.0),
        ui_border(),
    );
    if let Some(texture) = dialog.preview_texture.as_ref() {
        let size = texture.size();
        let scale = (preview.w / size.x).min(preview.h / size.y).min(1.0);
        let w = (size.x * scale).max(1.0);
        let h = (size.y * scale).max(1.0);
        draw_texture_ex(
            texture,
            preview.x + (preview.w - w) * 0.5,
            preview.y + (preview.h - h) * 0.5,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(w, h)),
                ..Default::default()
            },
        );
    } else {
        ui_text(
            &app.ui_font,
            "No preview",
            preview.x + 70.0,
            preview.y + 116.0,
            ui_muted(),
        );
    }
    if let Some(entry) = dialog.textures.get(dialog.selected) {
        let details_y = preview.y + preview.h + 28.0;
        ui_text_bold(
            &ellipsize_width(&entry.name, 18, preview.w),
            preview.x,
            details_y,
            18,
            WHITE,
        );
        ui_text(
            &app.ui_font,
            &format!("Size  {} x {}", entry.width, entry.height),
            preview.x,
            details_y + 28.0,
            LIGHTGRAY,
        );
        ui_text(
            &app.ui_font,
            &format!("Format  {}", tx_format_label(entry.format)),
            preview.x,
            details_y + 52.0,
            LIGHTGRAY,
        );
        ui_text(
            &app.ui_font,
            &format!("TXD  {}", ellipsize(&dialog.txd_name, 26)),
            preview.x,
            details_y + 76.0,
            ui_muted(),
        );
    }

    draw_dialog_button(
        &app.ui_font,
        texture_archive_add_rect(),
        "Add Texture",
        true,
    );
    draw_dialog_button(
        &app.ui_font,
        texture_archive_replace_rect(),
        "Replace Selected",
        !dialog.textures.is_empty(),
    );
    if dialog.textures.is_empty() {
        draw_rectangle(
            texture_archive_replace_rect().x,
            texture_archive_replace_rect().y,
            texture_archive_replace_rect().w,
            texture_archive_replace_rect().h,
            Color::new(0.0, 0.0, 0.0, 0.36),
        );
    }
    draw_dialog_button(&app.ui_font, texture_archive_close_rect(), "Close", false);
}

pub(crate) fn draw_dff_prelight_import_dialog(app: &AppState) {
    let Some(dialog) = app.dff_prelight_import_dialog.as_ref() else {
        return;
    };
    draw_modal_backdrop();
    let rect = dff_prelight_import_dialog_rect();
    draw_panel_rect(&app.ui_font, rect, Some("Import Lighting From DFF"));
    let archive = dialog
        .img_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("IMG archive");
    ui_text(
        &app.ui_font,
        &format!(
            "{} DFF source(s) in {}",
            dialog.entries.len(),
            ellipsize(archive, 42)
        ),
        rect.x + 24.0,
        rect.y + 58.0,
        LIGHTGRAY,
    );
    ui_text(
        &app.ui_font,
        &format!(
            "Target stream: {} prelight   Mouse wheel scrolls   Enter imports   Esc cancels",
            bake_light_mode_label(app.bake_settings.light_mode)
        ),
        rect.x + 24.0,
        rect.y + 82.0,
        ui_muted(),
    );

    let list_rect = Rect::new(rect.x + 24.0, rect.y + 108.0, rect.w - 48.0, rect.h - 176.0);
    draw_rrect_bordered(
        list_rect.x,
        list_rect.y,
        list_rect.w,
        list_rect.h,
        7.0,
        1.0,
        Color::new(0.045, 0.052, 0.064, 1.0),
        ui_border(),
    );
    let visible_rows = ((list_rect.h - 8.0) / 34.0).max(1.0) as usize;
    let start = dialog.scroll.floor() as usize;
    let max_chars = ((list_rect.w - 48.0) / 8.0).max(12.0) as usize;
    let mouse: Vec2 = mouse_position().into();
    for row in 0..visible_rows {
        let idx = start + row;
        let Some(entry) = dialog.entries.get(idx) else {
            break;
        };
        let row_rect = dff_prelight_import_row_rect(row);
        let selected = idx == dialog.selected;
        let hovered = row_rect.contains(mouse);
        if selected || hovered {
            draw_rrect(
                row_rect.x,
                row_rect.y + 2.0,
                row_rect.w,
                row_rect.h - 4.0,
                5.0,
                if selected {
                    ui_surface_active()
                } else {
                    ui_surface_hover()
                },
            );
        }
        let label = format!("{:>3}  {}", idx + 1, ellipsize(&entry.name, max_chars));
        ui_text(
            &app.ui_font,
            &label,
            row_rect.x + 10.0,
            row_rect.y + 20.0,
            if selected { ui_accent() } else { LIGHTGRAY },
        );
    }
    if dialog.entries.len() > visible_rows {
        let track = Rect::new(
            list_rect.x + list_rect.w - 10.0,
            list_rect.y + 8.0,
            4.0,
            list_rect.h - 16.0,
        );
        if let Some(metrics) = scrollbar_metrics(
            track,
            visible_rows as f32,
            dialog.entries.len() as f32,
            24.0,
            dialog.scroll,
        ) {
            draw_scrollbar(metrics, scrollbar_visual_state(track, false));
        }
    }

    draw_dialog_button(
        &app.ui_font,
        dff_prelight_import_apply_rect(),
        "Import Prelight",
        true,
    );
    draw_dialog_button(
        &app.ui_font,
        dff_prelight_import_close_rect(),
        "Close",
        false,
    );
}

pub(crate) fn draw_missing_col_dialog(app: &AppState) {
    let Some(dialog) = app.missing_col_dialog.as_ref() else {
        return;
    };
    draw_modal_backdrop();
    let rect = missing_col_dialog_rect();
    draw_panel_rect(&app.ui_font, rect, Some("Missing COL Assignments"));
    ui_text(
        &app.ui_font,
        &format!("{} definition(s) have no COL assigned.", dialog.missing),
        rect.x + 24.0,
        rect.y + 58.0,
        WHITE,
    );
    ui_text(
        &app.ui_font,
        "Fill missing COL fields from DFF names before output?",
        rect.x + 24.0,
        rect.y + 84.0,
        LIGHTGRAY,
    );
    ui_text(
        &app.ui_font,
        "This updates definitions and copies existing IMG files; it does not synthesize new COL geometry.",
        rect.x + 24.0,
        rect.y + 110.0,
        ui_muted(),
    );
    draw_dialog_button(
        &app.ui_font,
        Rect::new(rect.x + rect.w - 294.0, rect.y + rect.h - 50.0, 164.0, 32.0),
        "Fill and Save",
        true,
    );
    draw_dialog_button(
        &app.ui_font,
        Rect::new(rect.x + rect.w - 116.0, rect.y + rect.h - 50.0, 88.0, 32.0),
        "Cancel",
        false,
    );
}

pub(crate) fn draw_lod_batch_dialog(app: &AppState) {
    let Some(dialog) = app.lod_batch_dialog.as_ref() else {
        return;
    };
    if dialog.mode == LodBatchMode::GenerateSelection {
        draw_modal_backdrop();
    } else {
        // Keep the scene legible while the blue generation preview is active.
        draw_rectangle(
            0.0,
            0.0,
            screen_width(),
            screen_height(),
            Color::new(0.0, 0.0, 0.0, 0.24),
        );
    }
    let rect = lod_batch_dialog_rect();
    draw_panel_rect(&app.ui_font, rect, Some("Generate LODs"));
    ui_text(
        &app.ui_font,
        &format!(
            "{} {} elements. Elements smaller than the minimum are filtered out.",
            dialog.candidates.len(),
            if dialog.mode != LodBatchMode::GenerateSelection {
                "scene"
            } else {
                "selected"
            }
        ),
        rect.x + 24.0,
        rect.y + 56.0,
        LIGHTGRAY,
    );
    ui_text(
        &app.ui_font,
        "Minimum size",
        rect.x + 24.0,
        rect.y + 99.0,
        WHITE,
    );
    if dialog.mode != LodBatchMode::GenerateSelection {
        ui_text(&app.ui_font, "Mode", rect.x + 24.0, rect.y + 143.0, WHITE);
        text_button(
            &app.ui_font,
            lod_batch_missing_only_rect(),
            "Missing only",
            dialog.mode == LodBatchMode::GenerateSceneMissing,
        );
        text_button(
            &app.ui_font,
            lod_batch_regenerate_all_rect(),
            "Regenerate all",
            dialog.mode == LodBatchMode::RegenerateScene,
        );
    }
    let input = lod_batch_minimum_size_rect();
    let valid_minimum = lod_batch_minimum_size(dialog);
    draw_rrect_bordered(
        input.x,
        input.y,
        input.w,
        input.h,
        7.0,
        1.0,
        Color::new(0.055, 0.064, 0.078, 1.0),
        if valid_minimum.is_some() {
            ui_accent()
        } else {
            Color::new(0.82, 0.25, 0.22, 1.0)
        },
    );
    ui_text(
        &app.ui_font,
        &dialog.minimum_size,
        input.x + 10.0,
        input.y + 22.0,
        WHITE,
    );
    if (get_time() * 2.0) as i32 % 2 == 0 {
        let cursor = clamp_char_boundary(&dialog.minimum_size, dialog.cursor);
        let prefix = &dialog.minimum_size[..cursor];
        let caret_x = input.x + 10.0 + ui_text_width(prefix, 16);
        draw_line(
            caret_x,
            input.y + 7.0,
            caret_x,
            input.y + input.h - 7.0,
            1.0,
            WHITE,
        );
    }
    let included = lod_batch_included_count(dialog);
    ui_text(
        &app.ui_font,
        &format!(
            "{included} included   {} filtered/skipped",
            dialog.candidates.len().saturating_sub(included)
        ),
        input.x + input.w + 18.0,
        input.y + 22.0,
        if valid_minimum.is_some() {
            ui_accent()
        } else {
            Color::new(1.0, 0.42, 0.38, 1.0)
        },
    );

    let list = lod_batch_list_rect();
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
    let row_h = 30.0;
    let visible_rows = (list.h / row_h).max(1.0) as usize;
    let max_scroll = dialog.candidates.len().saturating_sub(visible_rows) as f32;
    let scroll = dialog.scroll.clamp(0.0, max_scroll);
    let start = scroll.floor() as usize;
    let minimum_size = valid_minimum.unwrap_or(f32::INFINITY);
    for slot in 0..visible_rows {
        let index = start + slot;
        let Some(candidate) = dialog.candidates.get(index) else {
            break;
        };
        let row = Rect::new(
            list.x + 6.0,
            list.y + slot as f32 * row_h + 3.0,
            list.w - 12.0,
            row_h - 4.0,
        );
        if slot % 2 == 0 {
            draw_rrect(
                row.x,
                row.y,
                row.w,
                row.h,
                4.0,
                Color::new(0.065, 0.074, 0.088, 1.0),
            );
        }
        let included = lod_batch_candidate_included(candidate, minimum_size, dialog.mode);
        let (state, color) = if candidate.size < minimum_size {
            ("FILTERED".to_string(), ui_muted())
        } else if included && candidate.existing_lod.is_some() {
            ("REGENERATE".to_string(), Color::new(0.30, 0.72, 1.0, 1.0))
        } else if let Some(parent) = candidate.existing_lod.as_deref() {
            (
                format!("SKIP · LOD {parent}"),
                Color::new(0.95, 0.68, 0.22, 1.0),
            )
        } else if included {
            (
                if dialog.mode != LodBatchMode::GenerateSelection {
                    "GENERATE"
                } else {
                    "INCLUDE"
                }
                .to_string(),
                Color::new(0.30, 0.85, 0.55, 1.0),
            )
        } else {
            ("FILTERED".to_string(), ui_muted())
        };
        let label = format!(
            "{}   {}   {:.2} units",
            ellipsize(&candidate.id, 26),
            ellipsize(&candidate.dff, 32),
            candidate.size
        );
        ui_text(&app.ui_font, &label, row.x + 8.0, row.y + 19.0, LIGHTGRAY);
        let state_w = ui_text_width(&state, 14);
        ui_text(
            &app.ui_font,
            &state,
            row.x + row.w - state_w - 10.0,
            row.y + 19.0,
            color,
        );
    }
    if dialog.candidates.len() > visible_rows {
        let track = Rect::new(list.x + list.w - 8.0, list.y + 7.0, 3.0, list.h - 14.0);
        if let Some(metrics) = scrollbar_metrics(
            track,
            visible_rows as f32,
            dialog.candidates.len() as f32,
            24.0,
            scroll,
        ) {
            draw_scrollbar(metrics, scrollbar_visual_state(track, false));
        }
    }
    ui_text(
        &app.ui_font,
        if dialog.mode != LodBatchMode::GenerateSelection {
            "Size is the longest rendered bounds dimension after element scale. Blue outlines preview elements included by the selected mode."
        } else {
            "Size is the longest rendered bounds dimension after element scale. Existing LODs are always skipped."
        },
        rect.x + 24.0,
        rect.y + rect.h - 58.0,
        ui_muted(),
    );
    draw_dialog_button(
        &app.ui_font,
        lod_batch_continue_rect(),
        if dialog.mode != LodBatchMode::GenerateSelection {
            "Generate"
        } else {
            "Continue"
        },
        true,
    );
    draw_dialog_button(&app.ui_font, lod_batch_cancel_rect(), "Cancel", false);
}

pub(crate) fn wrap_text_width(text: &str, size: u16, max_w: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };
        if current.is_empty() || ui_text_width(&candidate, size) <= max_w {
            current = candidate;
        } else {
            lines.push(std::mem::take(&mut current));
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

pub(crate) struct ConfirmDialogLayout {
    pub(crate) rect: Rect,
    pub(crate) body_lines: Vec<String>,
    pub(crate) detail_lines: Vec<String>,
    pub(crate) body_top: f32,
    pub(crate) detail_top: f32,
    pub(crate) primary: Rect,
    pub(crate) secondary: Option<Rect>,
    pub(crate) cancel: Rect,
}

pub(crate) fn confirm_dialog_layout(dialog: &ConfirmDialog) -> ConfirmDialogLayout {
    let w = 560.0_f32.min(screen_width() - 80.0);
    let text_w = w - 48.0;
    let body_lines = wrap_text_width(&dialog.body, 16, text_w);
    let detail_lines = wrap_text_width(&dialog.detail, 16, text_w);
    let body_h = body_lines.len() as f32 * 24.0;
    let detail_h = detail_lines.len() as f32 * 22.0;
    let btn_h = 38.0;
    let h = 56.0 + body_h + 10.0 + detail_h + 22.0 + btn_h + 20.0;
    let rect = Rect::new(
        (screen_width() - w) * 0.5,
        (screen_height() - h) * 0.5,
        w,
        h,
    );
    let body_top = rect.y + 74.0;
    let detail_top = body_top + body_h + 10.0;
    let btn_y = rect.y + h - btn_h - 20.0;
    let btn_w = |label: &str| (ui_text_width(label, 16) + 40.0).max(104.0);
    let cancel_w = btn_w("Cancel");
    let mut right = rect.x + rect.w - 20.0;
    let cancel = Rect::new(right - cancel_w, btn_y, cancel_w, btn_h);
    right -= cancel_w + 10.0;
    let secondary = dialog.secondary_label.as_deref().map(|label| {
        let w = btn_w(label);
        let r = Rect::new(right - w, btn_y, w, btn_h);
        right -= w + 10.0;
        r
    });
    let primary_w = btn_w(&dialog.primary_label);
    let primary = Rect::new(right - primary_w, btn_y, primary_w, btn_h);
    ConfirmDialogLayout {
        rect,
        body_lines,
        detail_lines,
        body_top,
        detail_top,
        primary,
        secondary,
        cancel,
    }
}

pub(crate) fn draw_confirm_dialog(app: &AppState) {
    let Some(dialog) = app.confirm_dialog.as_ref() else {
        return;
    };
    draw_modal_backdrop();
    let layout = confirm_dialog_layout(dialog);
    let rect = layout.rect;
    draw_panel_rect(&app.ui_font, rect, Some(&dialog.title));
    for (row, line) in layout.body_lines.iter().enumerate() {
        ui_text(
            &app.ui_font,
            line,
            rect.x + 24.0,
            layout.body_top + row as f32 * 24.0,
            WHITE,
        );
    }
    for (row, line) in layout.detail_lines.iter().enumerate() {
        ui_text(
            &app.ui_font,
            line,
            rect.x + 24.0,
            layout.detail_top + row as f32 * 22.0,
            LIGHTGRAY,
        );
    }
    draw_dialog_button(&app.ui_font, layout.primary, &dialog.primary_label, true);
    if let (Some(rect), Some(label)) = (layout.secondary, dialog.secondary_label.as_deref()) {
        draw_dialog_button(&app.ui_font, rect, label, false);
    }
    draw_dialog_button(&app.ui_font, layout.cancel, "Cancel", false);
}

pub(crate) struct DffMergeChoiceDialogLayout {
    pub(crate) rect: Rect,
    pub(crate) center: Rect,
    pub(crate) first: Rect,
    pub(crate) last: Rect,
    pub(crate) cancel: Rect,
}

pub(crate) fn dff_merge_choice_dialog_layout() -> DffMergeChoiceDialogLayout {
    let w = 520.0_f32.min(screen_width() - 80.0);
    let h = 184.0;
    let rect = Rect::new(
        (screen_width() - w) * 0.5,
        (screen_height() - h) * 0.5,
        w,
        h,
    );
    let btn_h = 38.0;
    let btn_y = rect.y + rect.h - btn_h - 20.0;
    let gap = 10.0;
    let cancel_w = 104.0;
    let action_w = ((rect.w - 40.0 - cancel_w - gap * 3.0) / 3.0).max(88.0);
    let center = Rect::new(rect.x + 20.0, btn_y, action_w, btn_h);
    let first = Rect::new(center.x + action_w + gap, btn_y, action_w, btn_h);
    let last = Rect::new(first.x + action_w + gap, btn_y, action_w, btn_h);
    let cancel = Rect::new(rect.x + rect.w - 20.0 - cancel_w, btn_y, cancel_w, btn_h);
    DffMergeChoiceDialogLayout {
        rect,
        center,
        first,
        last,
        cancel,
    }
}

pub(crate) fn draw_dff_merge_choice_dialog(app: &AppState) {
    let Some(dialog) = app.dff_merge_choice_dialog.as_ref() else {
        return;
    };
    draw_modal_backdrop();
    let layout = dff_merge_choice_dialog_layout();
    draw_panel_rect(&app.ui_font, layout.rect, Some("Merge Vertices"));
    ui_text(
        &app.ui_font,
        &format!("Merge {} selected vertex(es) at:", dialog.selected_count),
        layout.rect.x + 24.0,
        layout.rect.y + 76.0,
        WHITE,
    );
    ui_text(
        &app.ui_font,
        "UV seam groups stay separate when their coordinates differ.",
        layout.rect.x + 24.0,
        layout.rect.y + 104.0,
        LIGHTGRAY,
    );
    draw_dialog_button(&app.ui_font, layout.center, "Center", true);
    draw_dialog_button(&app.ui_font, layout.first, "First", false);
    draw_dialog_button(&app.ui_font, layout.last, "Last", false);
    draw_dialog_button(&app.ui_font, layout.cancel, "Cancel", false);
}

// ---------------------------------------------------------------------------
// Optimize / Repair DFF
// ---------------------------------------------------------------------------

/// One toggle row in the optimize sheet: label, help text, and accessors for
/// the flag it drives.
pub(crate) struct DffOptimizeToggle {
    pub(crate) label: &'static str,
    pub(crate) detail: &'static str,
    pub(crate) get: fn(&DffOptimizeOptions) -> bool,
    pub(crate) set: fn(&mut DffOptimizeOptions, bool),
    /// Indented sub-option, only interactive while its parent is enabled.
    pub(crate) nested: bool,
}

/// The toggle list, in the order it is drawn. Ordering here is presentational;
/// the passes always execute in the fixed order `run_dff_optimize` documents.
pub(crate) fn dff_optimize_toggles() -> Vec<DffOptimizeToggle> {
    vec![
        DffOptimizeToggle {
            label: "Reorder transparent faces",
            detail: "Draw alpha faces after opaque ones so glass stops culling what is behind it",
            get: |options| options.reorder_transparent_faces,
            set: |options, value| options.reorder_transparent_faces = value,
            nested: false,
        },
        DffOptimizeToggle {
            label: "Sort alpha faces back-to-front",
            detail: "Order those alpha faces outward from the model centre",
            get: |options| options.depth_sort_transparent_faces,
            set: |options, value| options.depth_sort_transparent_faces = value,
            nested: true,
        },
        DffOptimizeToggle {
            label: "Remove degenerate faces",
            detail: "Triangles with a repeated corner or zero area",
            get: |options| options.remove_degenerate_faces,
            set: |options, value| options.remove_degenerate_faces = value,
            nested: false,
        },
        DffOptimizeToggle {
            label: "Remove duplicate faces",
            detail: "Triangles repeating an earlier one in the same material",
            get: |options| options.remove_duplicate_faces,
            set: |options, value| options.remove_duplicate_faces = value,
            nested: false,
        },
        DffOptimizeToggle {
            label: "Weld duplicate vertices",
            detail: "Merge vertices identical across every stream (splits UV seams stay separate)",
            get: |options| options.weld_vertices,
            set: |options, value| options.weld_vertices = value,
            nested: false,
        },
        DffOptimizeToggle {
            label: "Remove unused vertices",
            detail: "Drop vertices no triangle references",
            get: |options| options.remove_unused_vertices,
            set: |options, value| options.remove_unused_vertices = value,
            nested: false,
        },
        DffOptimizeToggle {
            label: "Merge duplicate materials",
            detail: "Fold material slots sharing a texture, colour, and surface values",
            get: |options| options.merge_duplicate_materials,
            set: |options, value| options.merge_duplicate_materials = value,
            nested: false,
        },
        DffOptimizeToggle {
            label: "Remove unused materials",
            detail: "Drop material slots no triangle uses",
            get: |options| options.remove_unused_materials,
            set: |options, value| options.remove_unused_materials = value,
            nested: false,
        },
        DffOptimizeToggle {
            label: "Optimize vertex cache order",
            detail: "Reorder indices per material for GPU cache locality",
            get: |options| options.optimize_vertex_cache,
            set: |options, value| options.optimize_vertex_cache = value,
            nested: false,
        },
        DffOptimizeToggle {
            label: "Fix missing prelighting",
            detail: "Fill an absent day or night vertex colour stream so the model is not black",
            get: |options| options.fix_prelighting,
            set: |options, value| options.fix_prelighting = value,
            nested: false,
        },
    ]
}

pub(crate) const DFF_OPTIMIZE_ROW_H: f32 = 42.0;

pub(crate) struct DffOptimizeDialogLayout {
    pub(crate) rect: Rect,
    pub(crate) rows: Vec<Rect>,
    pub(crate) select_all: Rect,
    pub(crate) select_none: Rect,
    pub(crate) run: Rect,
    pub(crate) cancel: Rect,
}

pub(crate) fn dff_optimize_dialog_layout() -> DffOptimizeDialogLayout {
    let toggles = dff_optimize_toggles().len();
    let w = 640.0_f32.min(screen_width() - 80.0);
    let list_h = toggles as f32 * DFF_OPTIMIZE_ROW_H;
    let h = (list_h + 190.0).min(screen_height() - 60.0);
    let rect = Rect::new(
        (screen_width() - w) * 0.5,
        (screen_height() - h) * 0.5,
        w,
        h,
    );
    let list_y = rect.y + 74.0;
    let rows = (0..toggles)
        .map(|index| {
            Rect::new(
                rect.x + 24.0,
                list_y + index as f32 * DFF_OPTIMIZE_ROW_H,
                rect.w - 48.0,
                DFF_OPTIMIZE_ROW_H - 6.0,
            )
        })
        .collect::<Vec<_>>();
    let btn_h = 34.0;
    let btn_y = rect.y + rect.h - btn_h - 20.0;
    DffOptimizeDialogLayout {
        select_all: Rect::new(rect.x + 24.0, btn_y, 96.0, btn_h),
        select_none: Rect::new(rect.x + 128.0, btn_y, 96.0, btn_h),
        run: Rect::new(rect.x + rect.w - 24.0 - 210.0, btn_y, 120.0, btn_h),
        cancel: Rect::new(rect.x + rect.w - 24.0 - 84.0, btn_y, 84.0, btn_h),
        rows,
        rect,
    }
}

pub(crate) fn draw_dff_optimize_dialog(app: &AppState) {
    let Some(dialog) = app.dff_optimize_dialog.as_ref() else {
        return;
    };
    draw_modal_backdrop();
    let layout = dff_optimize_dialog_layout();
    draw_panel_rect(&app.ui_font, layout.rect, Some("Optimize / Repair DFF"));
    ui_text(
        &app.ui_font,
        &format!(
            "Choose what to apply to {}:",
            ellipsize(&dialog.dff_name, 44)
        ),
        layout.rect.x + 24.0,
        layout.rect.y + 62.0,
        LIGHTGRAY,
    );
    let toggles = dff_optimize_toggles();
    for (index, toggle) in toggles.iter().enumerate() {
        let Some(rect) = layout.rows.get(index) else {
            break;
        };
        let indent = if toggle.nested { 22.0 } else { 0.0 };
        // A sub-option is meaningless with its parent off, so grey it out.
        let parent_on = !toggle.nested || dialog.options.reorder_transparent_faces;
        let checked = (toggle.get)(&dialog.options) && parent_on;
        draw_checkbox(
            &app.ui_font,
            Rect::new(rect.x + indent, rect.y, rect.w - indent, 22.0),
            toggle.label,
            checked,
        );
        ui_text_size(
            &app.ui_font,
            toggle.detail,
            rect.x + indent + 28.0,
            rect.y + 34.0,
            13,
            if parent_on { ui_dim() } else { ui_muted() },
        );
    }
    draw_dialog_button(&app.ui_font, layout.select_all, "All", false);
    draw_dialog_button(&app.ui_font, layout.select_none, "None", false);
    draw_dialog_button(&app.ui_font, layout.run, "Run", true);
    draw_dialog_button(&app.ui_font, layout.cancel, "Cancel", false);
}

// ---------------------------------------------------------------------------
// Pair an external TXD with an opened DFF
// ---------------------------------------------------------------------------

pub(crate) struct DffTxdPairDialogLayout {
    pub(crate) rect: Rect,
    pub(crate) list: Rect,
    pub(crate) browse: Rect,
    pub(crate) skip: Rect,
}

pub(crate) fn dff_txd_pair_dialog_layout() -> DffTxdPairDialogLayout {
    let w = 560.0_f32.min(screen_width() - 80.0);
    let h = 320.0_f32.min(screen_height() - 60.0);
    let rect = Rect::new(
        (screen_width() - w) * 0.5,
        (screen_height() - h) * 0.5,
        w,
        h,
    );
    let btn_h = 36.0;
    let btn_y = rect.y + rect.h - btn_h - 20.0;
    DffTxdPairDialogLayout {
        list: Rect::new(rect.x + 24.0, rect.y + 118.0, rect.w - 48.0, rect.h - 190.0),
        browse: Rect::new(rect.x + rect.w - 24.0 - 268.0, btn_y, 180.0, btn_h),
        skip: Rect::new(rect.x + rect.w - 24.0 - 80.0, btn_y, 80.0, btn_h),
        rect,
    }
}

pub(crate) fn draw_dff_txd_pair_dialog(app: &AppState) {
    let Some(dialog) = app.dff_txd_pair_dialog.as_ref() else {
        return;
    };
    draw_modal_backdrop();
    let layout = dff_txd_pair_dialog_layout();
    draw_panel_rect(&app.ui_font, layout.rect, Some("Textures Not Found"));
    ui_text(
        &app.ui_font,
        &format!(
            "{} has {} of {} material texture(s) the editor cannot resolve.",
            ellipsize(&dialog.dff_name, 30),
            dialog.missing_textures.len(),
            dialog.material_count
        ),
        layout.rect.x + 24.0,
        layout.rect.y + 66.0,
        WHITE,
    );
    ui_text_size(
        &app.ui_font,
        "Pair a TXD to texture this model, or skip and edit it untextured.",
        layout.rect.x + 24.0,
        layout.rect.y + 92.0,
        14,
        LIGHTGRAY,
    );
    draw_rrect_bordered(
        layout.list.x,
        layout.list.y,
        layout.list.w,
        layout.list.h,
        8.0,
        1.0,
        ui_input_bg(),
        ui_border(),
    );
    let row_h = 22.0;
    let visible = (layout.list.h / row_h).floor().max(1.0) as usize;
    let max_start = dialog.missing_textures.len().saturating_sub(visible);
    let start = (dialog.scroll.floor().max(0.0) as usize).min(max_start);
    for (row, texture) in dialog
        .missing_textures
        .iter()
        .skip(start)
        .take(visible)
        .enumerate()
    {
        ui_text_size(
            &app.ui_font,
            texture,
            layout.list.x + 12.0,
            layout.list.y + 18.0 + row as f32 * row_h,
            14,
            ui_muted(),
        );
    }
    if dialog.missing_textures.len() > visible {
        ui_text_size(
            &app.ui_font,
            &format!(
                "{} more...",
                dialog.missing_textures.len() - (start + visible)
            ),
            layout.list.x + layout.list.w - 96.0,
            layout.list.y + layout.list.h - 8.0,
            13,
            ui_dim(),
        );
    }
    draw_dialog_button(&app.ui_font, layout.browse, "Choose TXD...", true);
    draw_dialog_button(&app.ui_font, layout.skip, "Skip", false);
}

pub(crate) struct DffTextureDuplicateDialogLayout {
    pub(crate) rect: Rect,
    pub(crate) input: Rect,
    pub(crate) duplicate: Rect,
    pub(crate) cancel: Rect,
}

pub(crate) fn dff_texture_duplicate_dialog_layout() -> DffTextureDuplicateDialogLayout {
    let w = 520.0_f32.min(screen_width() - 80.0);
    let h = 224.0;
    let rect = Rect::new(
        (screen_width() - w) * 0.5,
        (screen_height() - h) * 0.5,
        w,
        h,
    );
    DffTextureDuplicateDialogLayout {
        input: Rect::new(rect.x + 24.0, rect.y + 104.0, rect.w - 48.0, 36.0),
        duplicate: Rect::new(rect.x + rect.w - 232.0, rect.y + rect.h - 52.0, 112.0, 32.0),
        cancel: Rect::new(rect.x + rect.w - 108.0, rect.y + rect.h - 52.0, 84.0, 32.0),
        rect,
    }
}

pub(crate) fn draw_dff_texture_duplicate_dialog(app: &AppState) {
    let Some(dialog) = app.dff_texture_duplicate_dialog.as_ref() else {
        return;
    };
    draw_modal_backdrop();
    let layout = dff_texture_duplicate_dialog_layout();
    let separating = dialog.action == DffTextureNameAction::SeparateGeometry;
    let separating_internal = dialog.action == DffTextureNameAction::SeparateInternalElement;
    let renaming = matches!(
        dialog.action,
        DffTextureNameAction::RenameDff | DffTextureNameAction::RenameTxd
    );
    draw_panel_rect(
        &app.ui_font,
        layout.rect,
        Some(if separating {
            "Separate from Object"
        } else if separating_internal {
            "Separate to Internal Element"
        } else if renaming {
            "Rename Texture"
        } else {
            "Duplicate Texture"
        }),
    );
    ui_text(
        &app.ui_font,
        &if separating {
            format!(
                "Move the selected faces out of '{}' into a separate map object.",
                dialog.source_texture
            )
        } else if separating_internal {
            format!(
                "Keep the selected faces inside '{}' as a separately pivoted internal element.",
                dialog.source_texture
            )
        } else if renaming {
            format!(
                "Rename texture '{}' in its TXD and every DFF that uses that TXD.",
                dialog.source_texture
            )
        } else {
            format!(
                "Copy texture '{}' in the selected material's TXD.",
                dialog.source_texture
            )
        },
        layout.rect.x + 24.0,
        layout.rect.y + 58.0,
        WHITE,
    );
    ui_text(
        &app.ui_font,
        if separating {
            "New object/DFF name (letters, numbers, underscores, and hyphens):"
        } else if separating_internal {
            "Internal element/frame name (letters, numbers, underscores, and hyphens):"
        } else if renaming {
            "New texture name (the TXD and all affected DFFs are staged together):"
        } else {
            "New texture name (materials and faces will not be changed):"
        },
        layout.rect.x + 24.0,
        layout.rect.y + 84.0,
        LIGHTGRAY,
    );
    draw_rrect_bordered(
        layout.input.x,
        layout.input.y,
        layout.input.w,
        layout.input.h,
        7.0,
        1.0,
        Color::new(0.055, 0.064, 0.078, 1.0),
        ui_accent(),
    );
    let max_chars = ((layout.input.w - 20.0) / 8.5).max(1.0) as usize;
    let cursor = clamp_char_boundary(&dialog.buffer, dialog.cursor);
    let cursor_char = dialog.buffer[..cursor].chars().count();
    let total_chars = dialog.buffer.chars().count();
    let start_char = cursor_char.saturating_sub(max_chars.saturating_sub(1));
    let end_char = (start_char + max_chars).min(total_chars);
    let visible: String = dialog
        .buffer
        .chars()
        .skip(start_char)
        .take(end_char - start_char)
        .collect();
    let caret_prefix: String = visible
        .chars()
        .take(cursor_char.saturating_sub(start_char))
        .collect();
    let text_x = layout.input.x + 10.0;
    draw_visible_text_selection(
        &dialog.buffer,
        cursor,
        dialog.selection_anchor,
        start_char,
        &visible,
        text_x,
        layout.input,
    );
    ui_text(&app.ui_font, &visible, text_x, layout.input.y + 23.0, WHITE);
    if (get_time() * 2.0) as i32 % 2 == 0 {
        let caret_x = text_x + ui_text_width(&caret_prefix, 16).round();
        draw_line(
            caret_x,
            layout.input.y + 7.0,
            caret_x,
            layout.input.y + layout.input.h - 7.0,
            1.0,
            WHITE,
        );
    }
    draw_dialog_button(
        &app.ui_font,
        layout.duplicate,
        if separating || separating_internal {
            "Separate"
        } else if renaming {
            "Rename"
        } else {
            "Duplicate"
        },
        true,
    );
    draw_dialog_button(&app.ui_font, layout.cancel, "Cancel", false);
}

pub(crate) struct DffTextureViewDialogLayout {
    pub(crate) rect: Rect,
    pub(crate) image: Rect,
    pub(crate) close: Rect,
    pub(crate) export: Rect,
}

pub(crate) fn dff_texture_view_dialog_layout(
    dialog: &DffTextureViewDialog,
) -> DffTextureViewDialogLayout {
    let max_w = (screen_width() - 48.0).max(320.0);
    let max_h = (screen_height() - 48.0).max(260.0);
    let min_w = 420.0_f32.min(max_w);
    let min_h = 300.0_f32.min(max_h);
    let w = (dialog.width as f32 + 48.0).clamp(min_w, max_w);
    let h = (dialog.height as f32 + 126.0).clamp(min_h, max_h);
    let rect = Rect::new(
        (screen_width() - w) * 0.5,
        (screen_height() - h) * 0.5,
        w,
        h,
    );
    DffTextureViewDialogLayout {
        image: Rect::new(rect.x + 24.0, rect.y + 70.0, rect.w - 48.0, rect.h - 126.0),
        close: Rect::new(rect.x + rect.w - 108.0, rect.y + rect.h - 44.0, 84.0, 30.0),
        export: Rect::new(rect.x + 24.0, rect.y + rect.h - 44.0, 84.0, 30.0),
        rect,
    }
}

pub(crate) fn draw_dff_texture_view_dialog(app: &AppState) {
    let Some(dialog) = app.dff_texture_view_dialog.as_ref() else {
        return;
    };
    draw_modal_backdrop();
    let layout = dff_texture_view_dialog_layout(dialog);
    draw_panel_rect(&app.ui_font, layout.rect, Some("Full-size Texture"));
    ui_text(
        &app.ui_font,
        &format!(
            "{}   {} x {}   {}",
            dialog.texture_name, dialog.width, dialog.height, dialog.txd_name
        ),
        layout.rect.x + 24.0,
        layout.rect.y + 56.0,
        LIGHTGRAY,
    );
    draw_rrect_bordered(
        layout.image.x,
        layout.image.y,
        layout.image.w,
        layout.image.h,
        6.0,
        1.0,
        Color::new(0.025, 0.030, 0.038, 1.0),
        ui_border(),
    );
    let scale = (layout.image.w / dialog.width as f32)
        .min(layout.image.h / dialog.height as f32)
        .min(1.0);
    let width = (dialog.width as f32 * scale).max(1.0);
    let height = (dialog.height as f32 * scale).max(1.0);
    let destination = Rect::new(
        layout.image.x + (layout.image.w - width) * 0.5,
        layout.image.y + (layout.image.h - height) * 0.5,
        width,
        height,
    );
    if let Some(texture) = dialog.texture.as_ref() {
        draw_texture_ex(
            texture,
            destination.x,
            destination.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(destination.w, destination.h)),
                ..Default::default()
            },
        );
    } else if dialog.raw_texture != 0 {
        draw_raw_texture_quads(&[(dialog.raw_texture, destination)]);
    }
    if scale < 1.0 {
        ui_text_size(
            &app.ui_font,
            &format!("Scaled to {:.0}% to fit", scale * 100.0),
            layout.image.x + 8.0,
            layout.image.y + layout.image.h - 8.0,
            13,
            ui_muted(),
        );
    }
    draw_dialog_button(&app.ui_font, layout.close, "Close", true);
    draw_dialog_button(&app.ui_font, layout.export, "Export", dialog.rgba.is_some());
}

pub(crate) fn draw_loading_resource(root: &Path, status: &str) {
    reset_gl_for_ui();
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        screen_height(),
        Color::new(0.050, 0.058, 0.070, 1.0),
    );
    let w = 620.0_f32.min(screen_width() - 80.0);
    let rect = Rect::new(
        (screen_width() - w) * 0.5,
        screen_height() * 0.5 - 68.0,
        w,
        136.0,
    );
    draw_rrect_bordered(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        10.0,
        1.0,
        Color::new(0.070, 0.080, 0.096, 1.0),
        ui_border(),
    );
    ui_text_bold("Loading Resource", rect.x + 24.0, rect.y + 40.0, 22, WHITE);
    ui_text(
        &Font::default(),
        &ellipsize(root.to_string_lossy().as_ref(), 68),
        rect.x + 24.0,
        rect.y + 72.0,
        LIGHTGRAY,
    );
    let status_line = if status.is_empty() {
        "Parsing DFF/IMG/TXD data and rebuilding the scene."
    } else {
        status
    };
    ui_text(
        &Font::default(),
        status_line,
        rect.x + 24.0,
        rect.y + 104.0,
        ui_muted(),
    );
}
