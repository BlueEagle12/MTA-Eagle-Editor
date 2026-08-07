use super::super::*;

const MISSING_TEXTURE_REVIEW_ROW_H: f32 = 70.0;
const MODEL_TEXTURE_ROW_H: f32 = 64.0;

pub(crate) fn missing_texture_review_panel_rect() -> Rect {
    Rect::new(
        10.0,
        TOP_H + 12.0,
        PANEL_W - 20.0,
        screen_height() - TOP_H - STATUS_H - 24.0,
    )
}

fn missing_texture_review_details_panel_rect() -> Rect {
    let right = right_panel_rect();
    Rect::new(
        right.x + 12.0,
        right.y + 12.0,
        right.w - 24.0,
        right.h - 24.0,
    )
}

fn missing_texture_review_contains(mouse: Vec2) -> bool {
    missing_texture_review_panel_rect().contains(mouse)
        || missing_texture_review_details_panel_rect().contains(mouse)
}

fn missing_texture_review_run_rect() -> Rect {
    let panel = missing_texture_review_panel_rect();
    Rect::new(panel.x + 14.0, panel.y + 58.0, panel.w - 28.0, 30.0)
}

fn missing_texture_review_filter_rect(slot: usize) -> Rect {
    let panel = missing_texture_review_panel_rect();
    let gap = 8.0;
    let width = (panel.w - 28.0 - gap * 2.0) / 3.0;
    Rect::new(
        panel.x + 14.0 + slot as f32 * (width + gap),
        panel.y + 164.0,
        width,
        28.0,
    )
}

pub(crate) fn missing_texture_review_list_rect() -> Rect {
    let panel = missing_texture_review_panel_rect();
    Rect::new(
        panel.x + 10.0,
        panel.y + 202.0,
        panel.w - 20.0,
        (panel.h - 216.0).max(80.0),
    )
}

fn missing_texture_review_texture_list_rect() -> Rect {
    let panel = missing_texture_review_details_panel_rect();
    Rect::new(
        panel.x + 10.0,
        panel.y + 158.0,
        panel.w - 20.0,
        (panel.h - 172.0).max(80.0),
    )
}

fn missing_texture_review_regenerate_rect() -> Rect {
    let panel = missing_texture_review_details_panel_rect();
    Rect::new(panel.x + 14.0, panel.y + 94.0, panel.w - 28.0, 30.0)
}

fn missing_texture_review_texture_action_rect(row: Rect, missing: bool) -> Rect {
    let width = if missing { 148.0 } else { 62.0 };
    Rect::new(row.x + row.w - width - 8.0, row.y + 17.0, width, 30.0)
}

pub(crate) fn missing_texture_review_scroll_max(app: &AppState) -> f32 {
    let rows = filtered_missing_texture_review_indices(app).len();
    let list = missing_texture_review_list_rect();
    (rows as f32 * MISSING_TEXTURE_REVIEW_ROW_H - list.h).max(0.0)
}

fn missing_texture_review_texture_scroll_max(app: &AppState) -> f32 {
    let rows = selected_missing_texture_review_item(app)
        .map(|item| preview_material_entries(app, item.placement_index).len())
        .unwrap_or(0);
    let list = missing_texture_review_texture_list_rect();
    (rows as f32 * MODEL_TEXTURE_ROW_H - list.h).max(0.0)
}

fn selected_missing_texture_review_item(app: &AppState) -> Option<&MissingTextureReviewItem> {
    let selected = app.missing_texture_review.selected_item?;
    app.missing_texture_review
        .result
        .as_ref()?
        .items
        .get(selected)
}

fn review_texture_is_missing(
    app: &AppState,
    item: &MissingTextureReviewItem,
    entry: &PreviewMaterialEntry,
) -> bool {
    let texture_name = lower(entry.texture_name.trim());
    entry.missing
        && !texture_name.is_empty()
        && !app
            .texture_overrides
            .contains_key(&(item.id.clone(), texture_name))
}

fn draw_review_summary_card(app: &AppState, rect: Rect, label: &str, value: usize, color: Color) {
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

fn draw_missing_texture_review_queue(app: &AppState) {
    let panel = missing_texture_review_panel_rect();
    draw_panel_rect(&app.ui_font, panel, Some("Missing Texture Review"));
    let subtitle = app
        .missing_texture_review
        .result
        .as_ref()
        .map(|result| format!("Non-SA models · {} scanned", result.scanned_models))
        .unwrap_or_else(|| "Non-SA model flags, including LODs".to_string());
    ui_text_size(
        &app.ui_font,
        &ellipsize_width(&subtitle, 12, panel.w - 28.0),
        panel.x + 14.0,
        panel.y + 48.0,
        12,
        ui_dim(),
    );
    text_button(
        &app.ui_font,
        missing_texture_review_run_rect(),
        if app.missing_texture_review.rx.is_some() {
            "Scanning..."
        } else if app.missing_texture_review.stale {
            "Review (stale)"
        } else {
            "Review again"
        },
        app.missing_texture_review.rx.is_some(),
    );

    if let Some(result) = app.missing_texture_review.result.as_ref() {
        let detail = result.items.iter().filter(|item| !item.is_lod).count();
        let lods = result.items.iter().filter(|item| item.is_lod).count();
        let unique_textures = result
            .items
            .iter()
            .flat_map(|item| item.textures.iter())
            .collect::<HashSet<_>>()
            .len();
        let gap = 8.0;
        let card_y = panel.y + 96.0;
        let card_w = (panel.w - 28.0 - gap * 2.0) / 3.0;
        draw_review_summary_card(
            app,
            Rect::new(panel.x + 14.0, card_y, card_w, 58.0),
            "DETAIL",
            detail,
            Color::new(0.96, 0.36, 0.24, 1.0),
        );
        draw_review_summary_card(
            app,
            Rect::new(panel.x + 14.0 + card_w + gap, card_y, card_w, 58.0),
            "LODS",
            lods,
            Color::new(1.0, 0.66, 0.18, 1.0),
        );
        draw_review_summary_card(
            app,
            Rect::new(panel.x + 14.0 + (card_w + gap) * 2.0, card_y, card_w, 58.0),
            "TEXTURES",
            unique_textures,
            Color::new(0.76, 0.44, 0.96, 1.0),
        );
    } else {
        let message = if app.missing_texture_review.rx.is_some() {
            let (scanned, total) = app.missing_texture_review.progress;
            format!("Background scan: {scanned}/{total} models")
        } else {
            "Run the review to flag affected scene models.".to_string()
        };
        ui_text_size(
            &app.ui_font,
            &ellipsize_width(&message, 12, panel.w - 28.0),
            panel.x + 14.0,
            panel.y + 128.0,
            12,
            ui_dim(),
        );
    }

    for (slot, filter) in MissingTextureReviewFilter::ALL.into_iter().enumerate() {
        text_button(
            &app.ui_font,
            missing_texture_review_filter_rect(slot),
            filter.label(),
            app.missing_texture_review.filter == filter,
        );
    }

    let list = missing_texture_review_list_rect();
    draw_rrect(
        list.x,
        list.y,
        list.w,
        list.h,
        7.0,
        Color::new(0.047, 0.054, 0.066, 1.0),
    );
    let Some(result) = app.missing_texture_review.result.as_ref() else {
        return;
    };
    let filtered = filtered_missing_texture_review_indices(app);
    if filtered.is_empty() {
        ui_text(
            &app.ui_font,
            "No missing textures in this category.",
            list.x + 14.0,
            list.y + 28.0,
            ui_dim(),
        );
        return;
    }
    begin_ui_clip(list);
    for (row, item_index) in filtered.into_iter().enumerate() {
        let y = list.y + row as f32 * MISSING_TEXTURE_REVIEW_ROW_H
            - app.missing_texture_review.list_scroll;
        if y + MISSING_TEXTURE_REVIEW_ROW_H < list.y || y > list.y + list.h {
            continue;
        }
        let item = &result.items[item_index];
        let selected = app.missing_texture_review.selected_item == Some(item_index);
        let color = if item.is_lod {
            Color::new(1.0, 0.66, 0.18, 1.0)
        } else {
            Color::new(0.96, 0.36, 0.24, 1.0)
        };
        let rect = Rect::new(
            list.x + 4.0,
            y + 3.0,
            list.w - 8.0,
            MISSING_TEXTURE_REVIEW_ROW_H - 6.0,
        );
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
            if selected { color } else { ui_border() },
        );
        ui_text_size(
            &app.ui_font,
            if item.is_lod { "LOD" } else { "DETAIL" },
            rect.x + 10.0,
            rect.y + 17.0,
            10,
            color,
        );
        ui_text_size(
            &app.ui_font,
            &ellipsize_width(&item.id, 14, rect.w - 96.0),
            rect.x + 68.0,
            rect.y + 18.0,
            14,
            WHITE,
        );
        ui_text_size(
            &app.ui_font,
            &ellipsize_width(&item.textures.join(", "), 12, rect.w - 20.0),
            rect.x + 10.0,
            rect.y + 39.0,
            12,
            Color::new(1.0, 0.72, 0.44, 1.0),
        );
        let asset = if item.txd.is_empty() {
            item.dff.clone()
        } else {
            format!("{}  /  {}", item.dff, item.txd)
        };
        ui_text_size(
            &app.ui_font,
            &ellipsize_width(&asset, 10, rect.w - 20.0),
            rect.x + 10.0,
            rect.y + 56.0,
            10,
            ui_muted(),
        );
    }
    end_ui_clip();
}

fn draw_missing_texture_review_details(app: &AppState) {
    let panel = missing_texture_review_details_panel_rect();
    draw_panel_rect(&app.ui_font, panel, Some("Model Textures"));
    let Some(item) = selected_missing_texture_review_item(app) else {
        ui_text(
            &app.ui_font,
            "Select a flagged model from the review list.",
            panel.x + 14.0,
            panel.y + 58.0,
            ui_dim(),
        );
        ui_text_size(
            &app.ui_font,
            "Its complete material texture list and repair actions will appear here.",
            panel.x + 14.0,
            panel.y + 82.0,
            12,
            ui_muted(),
        );
        return;
    };

    let type_color = if item.is_lod {
        Color::new(1.0, 0.66, 0.18, 1.0)
    } else {
        Color::new(0.96, 0.36, 0.24, 1.0)
    };
    ui_text_size(
        &app.ui_font,
        if item.is_lod { "LOD" } else { "DETAIL" },
        panel.x + 14.0,
        panel.y + 52.0,
        11,
        type_color,
    );
    ui_text(
        &app.ui_font,
        &ellipsize_width(&item.id, 16, panel.w - 94.0),
        panel.x + 76.0,
        panel.y + 53.0,
        WHITE,
    );
    let asset = if item.txd.is_empty() {
        item.dff.clone()
    } else {
        format!("{}  /  {}", item.dff, item.txd)
    };
    ui_text_size(
        &app.ui_font,
        &ellipsize_width(&asset, 12, panel.w - 28.0),
        panel.x + 14.0,
        panel.y + 76.0,
        12,
        ui_muted(),
    );
    if item.is_lod {
        if app.lod_generation_job.is_some() {
            text_button_busy(
                &app.ui_font,
                missing_texture_review_regenerate_rect(),
                "Regenerating...",
            );
        } else {
            text_button(
                &app.ui_font,
                missing_texture_review_regenerate_rect(),
                "Regenerate",
                false,
            );
        }
    } else {
        ui_text_size(
            &app.ui_font,
            "Missing textures can be repaired individually below.",
            panel.x + 14.0,
            panel.y + 114.0,
            12,
            ui_dim(),
        );
    }

    let entries = preview_material_entries(app, item.placement_index);
    let missing = entries
        .iter()
        .filter(|entry| review_texture_is_missing(app, item, entry))
        .count();
    ui_text_size(
        &app.ui_font,
        &format!(
            "TEXTURES · {} material(s) · {missing} missing",
            entries.len()
        ),
        panel.x + 14.0,
        panel.y + 148.0,
        11,
        ui_dim(),
    );
    let list = missing_texture_review_texture_list_rect();
    draw_rrect(
        list.x,
        list.y,
        list.w,
        list.h,
        7.0,
        Color::new(0.047, 0.054, 0.066, 1.0),
    );
    if entries.is_empty() {
        ui_text(
            &app.ui_font,
            "This model has no texture materials.",
            list.x + 14.0,
            list.y + 28.0,
            ui_dim(),
        );
        return;
    }

    let mut thumbnails = Vec::new();
    begin_ui_clip(list);
    for (row, entry) in entries.iter().enumerate() {
        let y =
            list.y + row as f32 * MODEL_TEXTURE_ROW_H - app.missing_texture_review.texture_scroll;
        if y + MODEL_TEXTURE_ROW_H < list.y || y > list.y + list.h {
            continue;
        }
        let missing = review_texture_is_missing(app, item, entry);
        let rect = Rect::new(
            list.x + 4.0,
            y + 3.0,
            list.w - 8.0,
            MODEL_TEXTURE_ROW_H - 6.0,
        );
        draw_rrect_bordered(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            6.0,
            1.0,
            ui_panel_bg(),
            if missing {
                Color::new(0.82, 0.28, 0.22, 1.0)
            } else {
                ui_border()
            },
        );
        let thumb = Rect::new(rect.x + 7.0, rect.y + 7.0, 44.0, 44.0);
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
        if entry.texture_id != 0 {
            let available = thumb.w - 4.0;
            let source_width = entry.texture_width.max(1) as f32;
            let source_height = entry.texture_height.max(1) as f32;
            let scale = (available / source_width).min(available / source_height);
            let width = (source_width * scale).max(1.0);
            let height = (source_height * scale).max(1.0);
            thumbnails.push((
                entry.texture_id,
                Rect::new(
                    thumb.x + (thumb.w - width) * 0.5,
                    thumb.y + (thumb.h - height) * 0.5,
                    width,
                    height,
                ),
            ));
        } else {
            ui_text_size(
                &app.ui_font,
                "N/A",
                thumb.x + 10.0,
                thumb.y + 27.0,
                12,
                if missing { RED } else { ui_muted() },
            );
        }
        let action = missing_texture_review_texture_action_rect(rect, missing);
        let text_width = (action.x - (rect.x + 62.0) - 8.0).max(24.0);
        let texture_label = if entry.texture_name.trim().is_empty() {
            "<empty material>"
        } else {
            entry.texture_name.as_str()
        };
        ui_text(
            &app.ui_font,
            &ellipsize_width(texture_label, 16, text_width),
            rect.x + 59.0,
            rect.y + 22.0,
            if missing {
                Color::new(1.0, 0.58, 0.48, 1.0)
            } else {
                WHITE
            },
        );
        ui_text_size(
            &app.ui_font,
            &ellipsize_width(
                &if missing {
                    format!("Material #{} · MISSING", entry.material_index)
                } else {
                    format!(
                        "Material #{} · {} face{}",
                        entry.material_index,
                        entry.face_count,
                        if entry.face_count == 1 { "" } else { "s" }
                    )
                },
                12,
                text_width,
            ),
            rect.x + 59.0,
            rect.y + 43.0,
            12,
            if missing { RED } else { ui_muted() },
        );
        if missing {
            text_button(&app.ui_font, action, "Find missing texture", false);
        } else if entry.texture_id != 0 {
            text_button(&app.ui_font, action, "View", false);
        }
    }
    draw_raw_texture_quads(&thumbnails);
    end_ui_clip();
}

pub(crate) fn draw_missing_texture_review_panel(app: &AppState) {
    draw_missing_texture_review_queue(app);
    draw_missing_texture_review_details(app);
}

pub(crate) fn handle_missing_texture_review_scroll(
    app: &mut AppState,
    mouse: Vec2,
    wheel: f32,
) -> bool {
    if app.active_tab != AppTab::TextureReview || wheel.abs() <= f32::EPSILON {
        return false;
    }
    let queue = missing_texture_review_panel_rect();
    if queue.contains(mouse) {
        let max_scroll = missing_texture_review_scroll_max(app);
        app.missing_texture_review.list_scroll =
            (app.missing_texture_review.list_scroll - wheel * 36.0).clamp(0.0, max_scroll);
        return true;
    }
    let details = missing_texture_review_details_panel_rect();
    if details.contains(mouse) {
        let max_scroll = missing_texture_review_texture_scroll_max(app);
        app.missing_texture_review.texture_scroll =
            (app.missing_texture_review.texture_scroll - wheel * 36.0).clamp(0.0, max_scroll);
        return true;
    }
    false
}

pub(crate) fn handle_missing_texture_review_click(app: &mut AppState, mouse: Vec2) -> bool {
    if app.active_tab != AppTab::TextureReview {
        return false;
    }
    if missing_texture_review_run_rect().contains(mouse) {
        if is_mouse_button_pressed(MouseButton::Left) && app.missing_texture_review.rx.is_none() {
            request_missing_texture_review(app);
        }
        return true;
    }
    for (slot, filter) in MissingTextureReviewFilter::ALL.into_iter().enumerate() {
        if missing_texture_review_filter_rect(slot).contains(mouse) {
            if is_mouse_button_pressed(MouseButton::Left) {
                app.missing_texture_review.filter = filter;
                app.missing_texture_review.selected_item = None;
                app.missing_texture_review.list_scroll = 0.0;
                app.missing_texture_review.texture_scroll = 0.0;
            }
            return true;
        }
    }
    if !is_mouse_button_pressed(MouseButton::Left) {
        return missing_texture_review_contains(mouse);
    }

    let selected_item = selected_missing_texture_review_item(app).cloned();
    if let Some(item) = selected_item.as_ref() {
        if item.is_lod && missing_texture_review_regenerate_rect().contains(mouse) {
            request_lod_target_regeneration(app, item.placement_index);
            return true;
        }
        let entries = preview_material_entries(app, item.placement_index);
        let list = missing_texture_review_texture_list_rect();
        if list.contains(mouse) {
            for (row, entry) in entries.into_iter().enumerate() {
                let y = list.y + row as f32 * MODEL_TEXTURE_ROW_H
                    - app.missing_texture_review.texture_scroll;
                let row_rect = Rect::new(
                    list.x + 4.0,
                    y + 3.0,
                    list.w - 8.0,
                    MODEL_TEXTURE_ROW_H - 6.0,
                );
                let missing = review_texture_is_missing(app, item, &entry);
                if !missing_texture_review_texture_action_rect(row_rect, missing).contains(mouse) {
                    continue;
                }
                app.selected = item.placement_index;
                if missing {
                    find_missing_texture_for_selected_definition(app, &entry.texture_name);
                } else if entry.texture_id != 0 {
                    open_preview_texture_view_dialog(
                        app,
                        item.placement_index,
                        entry.material_index,
                    );
                }
                return true;
            }
            return true;
        }
    }

    let list = missing_texture_review_list_rect();
    if list.contains(mouse) {
        let filtered = filtered_missing_texture_review_indices(app);
        let row = ((mouse.y - list.y + app.missing_texture_review.list_scroll)
            / MISSING_TEXTURE_REVIEW_ROW_H)
            .floor() as usize;
        let Some(item_index) = filtered.get(row).copied() else {
            return true;
        };
        let Some(item) = app
            .missing_texture_review
            .result
            .as_ref()
            .and_then(|result| result.items.get(item_index))
            .cloned()
        else {
            return true;
        };
        if !is_live_element(app, item.placement_index) {
            invalidate_missing_texture_review(app);
            app.status_message = "That review item is stale; run the review again.".to_string();
            return true;
        }
        app.missing_texture_review.selected_item = Some(item_index);
        app.missing_texture_review.texture_scroll = 0.0;
        app.selected_elements = BTreeSet::from([item.placement_index]);
        app.selected_element_order = vec![item.placement_index];
        app.selected = item.placement_index;
        app.selected_group = None;
        app.selected_col_face = None;
        let target = to_mq(item.position);
        let extent = (to_mq(item.max) - to_mq(item.min)).length();
        let distance = (extent * 1.8).clamp(90.0, 320.0);
        let (forward, _) = camera_vectors(&app.camera);
        app.camera.pos = target - forward * distance + Vec3::Z * (distance * 0.24);
        app.camera_focus = Some(target);
        app.status_message = format!(
            "{}{} is missing: {}",
            item.id,
            if item.is_lod { " (LOD)" } else { "" },
            item.textures.join(", ")
        );
        return true;
    }
    missing_texture_review_contains(mouse)
}
