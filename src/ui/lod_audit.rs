use super::super::*;

const LOD_AUDIT_ROW_H: f32 = 62.0;

pub(crate) fn lod_audit_panel_rect() -> Rect {
    let right = right_panel_rect();
    Rect::new(
        right.x + 12.0,
        right.y + 12.0,
        right.w - 24.0,
        right.h - 24.0,
    )
}

pub(crate) fn lod_audit_run_rect() -> Rect {
    let panel = lod_audit_panel_rect();
    Rect::new(panel.x + 14.0, panel.y + 54.0, (panel.w - 36.0) * 0.5, 30.0)
}

pub(crate) fn lod_audit_client_rect() -> Rect {
    let panel = lod_audit_panel_rect();
    let run = lod_audit_run_rect();
    Rect::new(run.x + run.w + 8.0, panel.y + 54.0, run.w, 30.0)
}

pub(crate) fn lod_audit_generate_lods_rect() -> Rect {
    let panel = lod_audit_panel_rect();
    let width = (panel.w - 34.0) * 0.5;
    Rect::new(panel.x + 14.0, panel.y + 90.0, width, 30.0)
}

pub(crate) fn lod_audit_clear_lods_rect() -> Rect {
    let panel = lod_audit_panel_rect();
    let width = (panel.w - 34.0) * 0.5;
    Rect::new(panel.x + 20.0 + width, panel.y + 90.0, width, 30.0)
}

pub(crate) fn lod_audit_filter_rect(slot: usize) -> Rect {
    let panel = lod_audit_panel_rect();
    let gap = 6.0;
    let width = (panel.w - 28.0 - gap * 4.0) / 5.0;
    Rect::new(
        panel.x + 14.0 + slot as f32 * (width + gap),
        panel.y + 204.0,
        width,
        28.0,
    )
}

pub(crate) fn lod_audit_small_minus_rect() -> Rect {
    let panel = lod_audit_panel_rect();
    Rect::new(panel.x + 90.0, panel.y + 240.0, 28.0, 28.0)
}

pub(crate) fn lod_audit_small_plus_rect() -> Rect {
    let panel = lod_audit_panel_rect();
    Rect::new(panel.x + 190.0, panel.y + 240.0, 28.0, 28.0)
}

pub(crate) fn lod_audit_small_sort_rect() -> Rect {
    let panel = lod_audit_panel_rect();
    Rect::new(panel.x + 226.0, panel.y + 240.0, panel.w - 240.0, 28.0)
}

pub(crate) fn lod_audit_list_rect() -> Rect {
    let panel = lod_audit_panel_rect();
    Rect::new(
        panel.x + 10.0,
        panel.y + 276.0,
        panel.w - 20.0,
        (panel.h - 288.0).max(80.0),
    )
}

pub(crate) fn lod_audit_scroll_max(app: &AppState) -> f32 {
    let rows = filtered_lod_audit_issue_indices(app).len();
    let list = lod_audit_list_rect();
    (rows as f32 * LOD_AUDIT_ROW_H - list.h).max(0.0)
}

fn draw_summary_card(app: &AppState, rect: Rect, label: &str, value: usize, color: Color) {
    draw_rrect_bordered(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        7.0,
        1.0,
        ui_surface(),
        ui_border(),
    );
    draw_rectangle(rect.x, rect.y, 4.0, rect.h, color);
    ui_text_size(
        &app.ui_font,
        label,
        rect.x + 11.0,
        rect.y + 18.0,
        11,
        ui_dim(),
    );
    ui_text_bold(&value.to_string(), rect.x + 11.0, rect.y + 43.0, 20, WHITE);
}

pub(crate) fn draw_lod_audit_panel(app: &AppState) {
    let panel = lod_audit_panel_rect();
    draw_panel_rect(&app.ui_font, panel, Some("LOD Audit"));
    ui_text_size(
        &app.ui_font,
        "Game-distance simulation + review flags",
        panel.x + 14.0,
        panel.y + 48.0,
        12,
        ui_dim(),
    );
    text_button(
        &app.ui_font,
        lod_audit_run_rect(),
        if app.lod_audit.rx.is_some() {
            "Auditing..."
        } else if app.lod_audit.stale {
            "Run Audit (stale)"
        } else {
            "Run Audit"
        },
        app.lod_audit.rx.is_some(),
    );
    text_button(
        &app.ui_font,
        lod_audit_client_rect(),
        &format!("Client draw {}%", app.lod_audit.client_draw_percent),
        false,
    );
    if app.lod_generation_job.is_some() {
        text_button_busy(
            &app.ui_font,
            lod_audit_generate_lods_rect(),
            "Generating LODs",
        );
    } else {
        text_button(
            &app.ui_font,
            lod_audit_generate_lods_rect(),
            "Generate LODs",
            false,
        );
    }
    text_button(
        &app.ui_font,
        lod_audit_clear_lods_rect(),
        "Clear All LODs",
        false,
    );

    if app.lod_audit.result.is_some() {
        let gap = 8.0;
        let card_y = panel.y + 132.0;
        let card_w = (panel.w - 28.0 - gap * 2.0) / 3.0;
        draw_summary_card(
            app,
            Rect::new(panel.x + 14.0, card_y, card_w, 60.0),
            "COVERAGE",
            lod_audit_issue_count(app, LodAuditIssueKind::Missing),
            LodAuditIssueKind::Missing.color(),
        );
        draw_summary_card(
            app,
            Rect::new(panel.x + 14.0 + card_w + gap, card_y, card_w, 60.0),
            "DENSE",
            lod_audit_issue_count(app, LodAuditIssueKind::Dense),
            LodAuditIssueKind::Dense.color(),
        );
        draw_summary_card(
            app,
            Rect::new(panel.x + 14.0 + (card_w + gap) * 2.0, card_y, card_w, 60.0),
            "SMALL",
            lod_audit_issue_count(app, LodAuditIssueKind::Small),
            LodAuditIssueKind::Small.color(),
        );
    } else {
        ui_text_size(
            &app.ui_font,
            if app.lod_audit.rx.is_some() {
                "Analysis is running on a background worker."
            } else {
                "Run the audit to build visibility and review data."
            },
            panel.x + 14.0,
            panel.y + 160.0,
            12,
            ui_dim(),
        );
    }

    for (slot, filter) in LodAuditFilter::ALL.into_iter().enumerate() {
        text_button(
            &app.ui_font,
            lod_audit_filter_rect(slot),
            filter.label(),
            app.lod_audit.filter == filter,
        );
    }
    ui_text_size(
        &app.ui_font,
        "Small <=",
        panel.x + 14.0,
        panel.y + 259.0,
        12,
        ui_dim(),
    );
    text_button(&app.ui_font, lod_audit_small_minus_rect(), "-", false);
    let threshold_rect = Rect::new(panel.x + 124.0, panel.y + 240.0, 60.0, 28.0);
    draw_rrect_bordered(
        threshold_rect.x,
        threshold_rect.y,
        threshold_rect.w,
        threshold_rect.h,
        7.0,
        1.0,
        Color::new(0.075, 0.086, 0.104, 1.0),
        ui_border(),
    );
    let threshold = format!("{:.0}u", app.lod_audit.small_threshold);
    let threshold_width = ui_text_width(&threshold, 13);
    ui_text_size(
        &app.ui_font,
        &threshold,
        threshold_rect.x + (threshold_rect.w - threshold_width) * 0.5,
        threshold_rect.y + 19.0,
        13,
        WHITE,
    );
    text_button(&app.ui_font, lod_audit_small_plus_rect(), "+", false);
    text_button(
        &app.ui_font,
        lod_audit_small_sort_rect(),
        if app.lod_audit.small_sort_ascending {
            "Smallest first"
        } else {
            "Largest first"
        },
        app.lod_audit.filter == LodAuditFilter::Small,
    );

    let list = lod_audit_list_rect();
    draw_rrect(
        list.x,
        list.y,
        list.w,
        list.h,
        7.0,
        Color::new(0.047, 0.054, 0.066, 1.0),
    );
    let Some(result) = app.lod_audit.result.as_ref() else {
        return;
    };
    let filtered = filtered_lod_audit_issue_indices(app);
    if filtered.is_empty() {
        ui_text(
            &app.ui_font,
            "No review items in this category.",
            list.x + 14.0,
            list.y + 28.0,
            ui_dim(),
        );
        return;
    }
    begin_ui_clip(list);
    for (row, issue_index) in filtered.into_iter().enumerate() {
        let y = list.y + row as f32 * LOD_AUDIT_ROW_H - app.properties_scroll;
        if y + LOD_AUDIT_ROW_H < list.y || y > list.y + list.h {
            continue;
        }
        let issue = &result.issues[issue_index];
        let selected = app.lod_audit.selected_issue == Some(issue_index);
        let rect = Rect::new(list.x + 4.0, y + 3.0, list.w - 8.0, LOD_AUDIT_ROW_H - 6.0);
        draw_rrect_bordered(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            6.0,
            1.0,
            if selected {
                ui_surface_active()
            } else {
                ui_panel_bg()
            },
            if selected {
                issue.kind.color()
            } else {
                ui_border()
            },
        );
        ui_text_size(
            &app.ui_font,
            issue.kind.label(),
            rect.x + 10.0,
            rect.y + 17.0,
            10,
            issue.kind.color(),
        );
        ui_text_size(
            &app.ui_font,
            &ellipsize_width(
                &issue.title,
                14,
                if issue.kind == LodAuditIssueKind::Small {
                    rect.w - 176.0
                } else {
                    rect.w - 90.0
                },
            ),
            rect.x + 76.0,
            rect.y + 18.0,
            13,
            WHITE,
        );
        if issue.kind == LodAuditIssueKind::Small {
            let size = format!("{:.1}u", issue.size);
            let size_width = ui_text_width(&size, 11);
            ui_text_size(
                &app.ui_font,
                &size,
                rect.x + rect.w - size_width - 10.0,
                rect.y + 17.0,
                11,
                issue.kind.color(),
            );
        }
        ui_text_size(
            &app.ui_font,
            &ellipsize_width(&issue.detail, 12, rect.w - 20.0),
            rect.x + 10.0,
            rect.y + 41.0,
            11,
            ui_dim(),
        );
    }
    end_ui_clip();

    let max_scroll = lod_audit_scroll_max(app);
    if max_scroll > 0.5 {
        let track = Rect::new(list.x + list.w - 6.0, list.y + 4.0, 3.0, list.h - 8.0);
        if let Some(metrics) = scrollbar_metrics(
            track,
            track.h,
            track.h + max_scroll,
            24.0,
            app.properties_scroll,
        ) {
            draw_scrollbar(metrics, scrollbar_visual_state(track, false));
        }
    }

    let config_label = if result.loader_config_found {
        format!(
            "eagleLoader {:.2} | dense {}/{} tris | small <= {:.0}u | {} ignored",
            result.loader_distance_multiplier,
            24,
            100_000,
            app.lod_audit.small_threshold,
            result_ignored_count(app)
        )
    } else {
        "Canonical loader config unavailable; using multiplier 1.0".to_string()
    };
    ui_text_size(
        &app.ui_font,
        &ellipsize_width(&config_label, 11, panel.w - 28.0),
        panel.x + 14.0,
        panel.y + panel.h - 8.0,
        10,
        ui_muted(),
    );
}

pub(crate) fn handle_lod_audit_click(app: &mut AppState, mouse: Vec2) -> bool {
    if app.active_tab != AppTab::LodAudit {
        return false;
    }
    if is_mouse_button_down(MouseButton::Left) {
        let list = lod_audit_list_rect();
        let max_scroll = lod_audit_scroll_max(app);
        let track = Rect::new(list.x + list.w - 6.0, list.y + 4.0, 3.0, list.h - 8.0);
        let hit_area = Rect::new(track.x - 6.0, track.y, track.w + 12.0, track.h);
        if max_scroll > 0.5 && hit_area.contains(mouse) {
            let thumb_h = (track.h * track.h / (track.h + max_scroll)).clamp(24.0, track.h);
            let travel = (track.h - thumb_h).max(1.0);
            app.properties_scroll =
                ((mouse.y - track.y - thumb_h * 0.5).clamp(0.0, travel) / travel) * max_scroll;
            return true;
        }
    }
    if lod_audit_run_rect().contains(mouse) {
        if is_mouse_button_pressed(MouseButton::Left) && app.lod_audit.rx.is_none() {
            request_lod_audit(app);
        }
        return true;
    }
    if lod_audit_client_rect().contains(mouse) {
        if is_mouse_button_pressed(MouseButton::Left) {
            app.lod_audit.client_draw_percent = match app.lod_audit.client_draw_percent {
                0 => 50,
                50 => 100,
                _ => 0,
            };
            app.status_message = format!(
                "LOD audit client draw-distance setting: {}%",
                app.lod_audit.client_draw_percent
            );
        }
        return true;
    }
    if lod_audit_generate_lods_rect().contains(mouse) {
        if is_mouse_button_pressed(MouseButton::Left) && app.lod_generation_job.is_none() {
            request_scene_lod_generation(app);
        }
        return true;
    }
    if lod_audit_clear_lods_rect().contains(mouse) {
        if is_mouse_button_pressed(MouseButton::Left) {
            request_clear_all_lods(app);
        }
        return true;
    }
    for (slot, filter) in LodAuditFilter::ALL.into_iter().enumerate() {
        if lod_audit_filter_rect(slot).contains(mouse) {
            if is_mouse_button_pressed(MouseButton::Left) {
                app.lod_audit.filter = filter;
                app.lod_audit.selected_issue = None;
                app.properties_scroll = 0.0;
            }
            return true;
        }
    }
    if lod_audit_small_minus_rect().contains(mouse) || lod_audit_small_plus_rect().contains(mouse) {
        if is_mouse_button_pressed(MouseButton::Left) && app.lod_audit.rx.is_none() {
            let fine_step = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
            let step = if fine_step { 1.0 } else { 5.0 };
            if lod_audit_small_minus_rect().contains(mouse) {
                app.lod_audit.small_threshold = (app.lod_audit.small_threshold - step).max(1.0);
            } else {
                app.lod_audit.small_threshold = (app.lod_audit.small_threshold + step).min(500.0);
            }
            let saved = save_lod_audit_small_threshold_preference(app.lod_audit.small_threshold);
            invalidate_lod_audit(app);
            app.status_message = match saved {
                Ok(()) => format!(
                    "Small LOD threshold set to {:.0} units and saved; run the audit to refresh flags",
                    app.lod_audit.small_threshold
                ),
                Err(error) => format!(
                    "Small LOD threshold set to {:.0} for this session, but could not save it: {error}",
                    app.lod_audit.small_threshold
                ),
            };
        }
        return true;
    }
    if lod_audit_small_sort_rect().contains(mouse) {
        if is_mouse_button_pressed(MouseButton::Left) {
            app.lod_audit.small_sort_ascending = !app.lod_audit.small_sort_ascending;
            app.properties_scroll = 0.0;
            app.status_message = if app.lod_audit.small_sort_ascending {
                "Small LOD review sorted smallest first".to_string()
            } else {
                "Small LOD review sorted largest first".to_string()
            };
        }
        return true;
    }
    let left_click = is_mouse_button_pressed(MouseButton::Left);
    let right_click = is_mouse_button_pressed(MouseButton::Right);
    if !left_click && !right_click {
        return lod_audit_panel_rect().contains(mouse);
    }
    let list = lod_audit_list_rect();
    if !list.contains(mouse) {
        return lod_audit_panel_rect().contains(mouse);
    }
    let filtered = filtered_lod_audit_issue_indices(app);
    let row = ((mouse.y - list.y + app.properties_scroll) / LOD_AUDIT_ROW_H).floor() as usize;
    let Some(issue_index) = filtered.get(row).copied() else {
        return true;
    };
    let Some(issue) = app
        .lod_audit
        .result
        .as_ref()
        .and_then(|result| result.issues.get(issue_index))
        .cloned()
    else {
        return true;
    };
    app.lod_audit.selected_issue = Some(issue_index);
    app.selected_elements = if issue.kind == LodAuditIssueKind::Dense {
        BTreeSet::new()
    } else {
        issue
            .placement_indices
            .iter()
            .copied()
            .filter(|index| is_live_element(app, *index))
            .collect()
    };
    app.selected_element_order = app.selected_elements.iter().copied().collect();
    app.selected = app
        .selected_element_order
        .last()
        .copied()
        .unwrap_or(NO_SELECTION);
    app.selected_group = None;
    app.selected_col_face = None;
    if right_click {
        app.context_menu = Some(ContextMenu {
            pos: mouse,
            target: ContextMenuTarget::LodAuditIssue(issue_index),
        });
    } else {
        let target = to_mq(issue.position);
        let distance = if issue.kind == LodAuditIssueKind::Dense {
            (issue.size * 1.15).clamp(220.0, 1_200.0)
        } else {
            (issue.size * 1.6).clamp(110.0, 240.0)
        };
        let (forward, _) = camera_vectors(&app.camera);
        app.camera.pos = target - forward * distance + Vec3::Z * (distance * 0.28);
        app.camera_focus = Some(target);
        app.status_message = issue.detail;
    }
    true
}
