use super::super::*;

pub(crate) fn validation_panel_rect() -> Rect {
    Rect::new(
        PANEL_W + 12.0,
        TOP_H + 12.0,
        (screen_width() - PANEL_W - 24.0).max(480.0),
        (screen_height() - STATUS_H - TOP_H - 24.0).max(480.0),
    )
}

const VALIDATION_CATEGORIES: [ValidationActionCategory; 4] = [
    ValidationActionCategory::Review,
    ValidationActionCategory::Repair,
    ValidationActionCategory::Optimize,
    ValidationActionCategory::Collision,
];

fn validation_category_label(category: ValidationActionCategory) -> &'static str {
    match category {
        ValidationActionCategory::Review => "Review",
        ValidationActionCategory::Repair => "Repair",
        ValidationActionCategory::Optimize => "Project Assets",
        ValidationActionCategory::Collision => "Collision Setup",
    }
}

fn validation_category_actions(category: ValidationActionCategory) -> &'static [usize] {
    match category {
        ValidationActionCategory::Review => &[10, 0, 3, 5],
        ValidationActionCategory::Repair => &[14, 1, 4, 11, 12, 15],
        ValidationActionCategory::Optimize => &[13, 2, 9],
        ValidationActionCategory::Collision => &[6, 7, 8],
    }
}

/// Slot an action occupies in its category row. Popup menus anchor through
/// this so reordering a category cannot detach a menu from its button.
fn validation_action_position(category: ValidationActionCategory, action: usize) -> usize {
    validation_category_actions(category)
        .iter()
        .position(|candidate| *candidate == action)
        .unwrap_or(0)
}

fn validation_action_anchor_rect(category: ValidationActionCategory, action: usize) -> Rect {
    validation_action_button_rect(category, validation_action_position(category, action))
}

fn validation_category_tab_rect(slot: usize) -> Rect {
    let panel = validation_panel_rect();
    let gap = 8.0;
    let compact = panel.w < 760.0;
    let start_x = if compact {
        panel.x + 14.0
    } else {
        panel.x + 190.0
    };
    let available = panel.x + panel.w - 14.0 - start_x;
    let w = ((available - gap * 3.0) / 4.0).max(76.0);
    let x = start_x + slot.min(3) as f32 * (w + gap);
    let y = panel.y + if compact { 66.0 } else { 16.0 };
    Rect::new(x, y, w, 30.0)
}

fn validation_action_button_rect(category: ValidationActionCategory, position: usize) -> Rect {
    let panel = validation_panel_rect();
    let gap = 8.0;
    let compact = panel.w < 760.0;
    let start_x = if compact {
        panel.x + 14.0
    } else {
        panel.x + 190.0
    };
    let available = panel.x + panel.w - 14.0 - start_x;
    let columns = validation_category_actions(category).len().max(1);
    let w = ((available - gap * columns.saturating_sub(1) as f32) / columns as f32).max(96.0);
    let x = start_x + position.min(columns - 1) as f32 * (w + gap);
    let y = panel.y + if compact { 104.0 } else { 54.0 };
    Rect::new(x, y, w, 30.0)
}

fn validation_collision_material_dropdown_rect_for(panel: Rect, anchor: Rect) -> Rect {
    let width = 360.0_f32.min(panel.w - 28.0);
    let height = 340.0_f32.min(panel.y + panel.h - anchor.y - anchor.h - 20.0);
    let x = (anchor.x + anchor.w - width).clamp(panel.x + 14.0, panel.x + panel.w - width - 14.0);
    Rect::new(x, anchor.y + anchor.h + 6.0, width, height.max(220.0))
}

fn validation_collision_material_dropdown_rect() -> Rect {
    validation_collision_material_dropdown_rect_for(
        validation_panel_rect(),
        validation_action_anchor_rect(ValidationActionCategory::Collision, 7),
    )
}

fn validation_collision_material_search_rect() -> Rect {
    let dropdown = validation_collision_material_dropdown_rect();
    Rect::new(
        dropdown.x + 14.0,
        dropdown.y + 42.0,
        dropdown.w - 28.0,
        30.0,
    )
}

fn validation_collision_material_list_rect() -> Rect {
    let dropdown = validation_collision_material_dropdown_rect();
    Rect::new(
        dropdown.x + 14.0,
        dropdown.y + 82.0,
        dropdown.w - 28.0,
        dropdown.h - 96.0,
    )
}

fn filtered_validation_collision_materials(search: &str) -> Vec<(u8, &'static str)> {
    let needle = search.trim().to_ascii_lowercase();
    GTA_SA_COL_MATERIALS
        .iter()
        .copied()
        .filter(|(id, name)| {
            needle.is_empty()
                || name.to_ascii_lowercase().contains(&needle)
                || id.to_string().contains(&needle)
        })
        .collect()
}

fn close_validation_collision_material_dropdown(app: &mut AppState) {
    app.validation_collision_material_dropdown_open = false;
    app.validation_collision_material_search.clear();
    app.validation_collision_material_scroll = 0.0;
}

fn validation_toolbar_bottom() -> f32 {
    let button = validation_action_button_rect(ValidationActionCategory::Review, 0);
    button.y + button.h
}

fn validation_summary_card_rect(slot: usize) -> Rect {
    let panel = validation_panel_rect();
    let gap = 12.0;
    let columns = if panel.w < 800.0 { 2 } else { 4 };
    let card_y = validation_toolbar_bottom() + 14.0;
    let card_w = (panel.w - 28.0 - gap * (columns - 1) as f32) / columns as f32;
    Rect::new(
        panel.x + 14.0 + (slot % columns) as f32 * (card_w + gap),
        card_y + (slot / columns) as f32 * (64.0 + gap),
        card_w,
        64.0,
    )
}

fn validation_summary_cards_bottom() -> f32 {
    let last = validation_summary_card_rect(3);
    last.y + last.h
}

fn validation_optimization_menu_rect_for(panel: Rect, anchor: Rect) -> Rect {
    let width = 280.0;
    let height = 174.0;
    let x = (anchor.x + anchor.w - width).clamp(panel.x + 12.0, panel.x + panel.w - width - 12.0);
    Rect::new(x, anchor.y + anchor.h + 6.0, width, height)
}

pub(crate) fn validation_optimization_menu_rect() -> Rect {
    let panel = validation_panel_rect();
    validation_optimization_menu_rect_for(
        panel,
        validation_action_anchor_rect(ValidationActionCategory::Optimize, 2),
    )
}

fn validation_optimization_scope_rect_for(menu: Rect, slot: usize) -> Rect {
    Rect::new(
        menu.x + 16.0,
        menu.y + 38.0 + slot as f32 * 29.0,
        menu.w - 32.0,
        23.0,
    )
}

pub(crate) fn validation_optimization_scope_rect(slot: usize) -> Rect {
    validation_optimization_scope_rect_for(validation_optimization_menu_rect(), slot)
}

fn validation_optimization_continue_rect_for(menu: Rect) -> Rect {
    Rect::new(menu.x + 16.0, menu.y + menu.h - 42.0, menu.w - 32.0, 28.0)
}

pub(crate) fn validation_optimization_continue_rect() -> Rect {
    validation_optimization_continue_rect_for(validation_optimization_menu_rect())
}

fn validation_dff_repair_menu_rect() -> Rect {
    let panel = validation_panel_rect();
    let anchor = validation_action_anchor_rect(ValidationActionCategory::Repair, 4);
    let width = 280.0;
    let height = 174.0;
    let x = (anchor.x + anchor.w - width).clamp(panel.x + 12.0, panel.x + panel.w - width - 12.0);
    Rect::new(x, anchor.y + anchor.h + 6.0, width, height)
}

fn validation_dff_repair_scope_rect(slot: usize) -> Rect {
    let menu = validation_dff_repair_menu_rect();
    Rect::new(
        menu.x + 16.0,
        menu.y + 38.0 + slot as f32 * 29.0,
        menu.w - 32.0,
        23.0,
    )
}

fn validation_dff_repair_continue_rect() -> Rect {
    let menu = validation_dff_repair_menu_rect();
    Rect::new(menu.x + 16.0, menu.y + menu.h - 42.0, menu.w - 32.0, 28.0)
}

#[derive(Clone, Copy)]
pub(crate) enum ValidationListKind {
    Missing,
    Unused,
    Warnings,
}

impl ValidationListKind {
    const fn slot(self) -> usize {
        match self {
            Self::Missing => 0,
            Self::Unused => 1,
            Self::Warnings => 2,
        }
    }
}

pub(crate) fn validation_list_count(
    summary: &ValidationSummary,
    kind: ValidationListKind,
) -> usize {
    match kind {
        ValidationListKind::Missing => {
            summary.missing_dffs.len()
                + summary.missing_cols.len()
                + summary.missing_txds.len()
                + summary.missing_definition_ids.len()
        }
        ValidationListKind::Unused => {
            summary.unused_dffs.len()
                + summary.unused_cols.len()
                + summary.unused_txds.len()
                + summary.unused_definitions.len()
        }
        ValidationListKind::Warnings => {
            summary.missing_col_attrs.len()
                + summary.duplicate_dffs.len()
                + summary.invalid_texture_formats.len()
                + summary.invalid_dff_material_counts.len()
                + summary.invalid_col_loads.len()
                + summary.breakable_warnings.len()
        }
    }
}

pub(crate) fn validation_list_item(
    summary: &ValidationSummary,
    kind: ValidationListKind,
    mut index: usize,
) -> Option<ValidationItem> {
    match kind {
        ValidationListKind::Missing => {
            if index < summary.missing_dffs.len() {
                let key = summary.missing_dffs[index].clone();
                return Some(ValidationItem {
                    kind: MissingAssetKind::Dff,
                    label: format!("DFF {key}"),
                    key,
                });
            }
            index -= summary.missing_dffs.len();
            if index < summary.missing_cols.len() {
                let key = summary.missing_cols[index].clone();
                return Some(ValidationItem {
                    kind: MissingAssetKind::Col,
                    label: format!("COL {key}"),
                    key,
                });
            }
            index -= summary.missing_cols.len();
            if index < summary.missing_txds.len() {
                let key = summary.missing_txds[index].clone();
                return Some(ValidationItem {
                    kind: MissingAssetKind::Txd,
                    label: format!("TXD {key}"),
                    key,
                });
            }
            index -= summary.missing_txds.len();
            summary
                .missing_definition_ids
                .get(index)
                .map(|id| ValidationItem {
                    kind: MissingAssetKind::Definition,
                    label: format!("Definition {id}"),
                    key: id.clone(),
                })
        }
        ValidationListKind::Unused => {
            if index < summary.unused_dffs.len() {
                return Some(ValidationItem {
                    kind: MissingAssetKind::Dff,
                    key: summary.unused_dffs[index].clone(),
                    label: format!("DFF {}", summary.unused_dffs[index]),
                });
            }
            index -= summary.unused_dffs.len();
            if index < summary.unused_cols.len() {
                return Some(ValidationItem {
                    kind: MissingAssetKind::Col,
                    key: summary.unused_cols[index].clone(),
                    label: format!("COL {}", summary.unused_cols[index]),
                });
            }
            index -= summary.unused_cols.len();
            if index < summary.unused_txds.len() {
                return Some(ValidationItem {
                    kind: MissingAssetKind::Txd,
                    key: summary.unused_txds[index].clone(),
                    label: format!("TXD {}", summary.unused_txds[index]),
                });
            }
            index -= summary.unused_txds.len();
            summary
                .unused_definitions
                .get(index)
                .map(|id| ValidationItem {
                    kind: MissingAssetKind::Definition,
                    label: format!("Definition {id}"),
                    key: id.clone(),
                })
        }
        ValidationListKind::Warnings => {
            if index < summary.missing_col_attrs.len() {
                return Some(ValidationItem {
                    kind: MissingAssetKind::Col,
                    key: summary.missing_col_attrs[index].clone(),
                    label: format!("{} has no COL", summary.missing_col_attrs[index]),
                });
            }
            index -= summary.missing_col_attrs.len();
            if index < summary.duplicate_dffs.len() {
                return summary
                    .duplicate_dffs
                    .get(index)
                    .map(|name| ValidationItem {
                        kind: MissingAssetKind::Dff,
                        key: name.clone(),
                        label: format!("Duplicate loose/IMG {name}"),
                    });
            }
            index -= summary.duplicate_dffs.len();
            if index < summary.invalid_texture_formats.len() {
                return summary
                    .invalid_texture_formats
                    .get(index)
                    .map(|warning| ValidationItem {
                        kind: MissingAssetKind::Txd,
                        key: warning.clone(),
                        label: warning.clone(),
                    });
            }
            index -= summary.invalid_texture_formats.len();
            if index < summary.invalid_dff_material_counts.len() {
                return summary
                    .invalid_dff_material_counts
                    .get(index)
                    .map(|warning| ValidationItem {
                        kind: MissingAssetKind::Dff,
                        key: warning
                            .strip_prefix("DFF ")
                            .and_then(|value| value.split(':').next())
                            .unwrap_or(warning)
                            .to_string(),
                        label: warning.clone(),
                    });
            }
            index -= summary.invalid_dff_material_counts.len();
            if index < summary.invalid_col_loads.len() {
                return summary
                    .invalid_col_loads
                    .get(index)
                    .map(|warning| ValidationItem {
                        kind: MissingAssetKind::Col,
                        key: warning.clone(),
                        label: warning.clone(),
                    });
            }
            index -= summary.invalid_col_loads.len();
            summary
                .breakable_warnings
                .get(index)
                .map(|warning| ValidationItem {
                    kind: MissingAssetKind::Dff,
                    key: warning.clone(),
                    label: warning.clone(),
                })
        }
    }
}

pub(crate) fn validation_lists_rect() -> Rect {
    let panel = validation_panel_rect();
    let list_y = validation_summary_cards_bottom() + 14.0;
    Rect::new(
        panel.x + 14.0,
        list_y,
        panel.w - 28.0,
        (panel.y + panel.h - list_y - 14.0).max(120.0),
    )
}

fn validation_list_column_rects_for(lists: Rect) -> [Rect; 3] {
    let gap = 14.0;
    let usable = (lists.w - gap * 2.0).max(1.0);
    let missing_w = usable * 0.30;
    let unused_w = usable * 0.34;
    let warnings_w = usable - missing_w - unused_w;
    let missing = Rect::new(lists.x, lists.y, missing_w, lists.h);
    let unused = Rect::new(lists.x + missing_w + gap, lists.y, unused_w, lists.h);
    let warnings = Rect::new(unused.x + unused_w + gap, lists.y, warnings_w, lists.h);
    [missing, unused, warnings]
}

fn validation_list_column_rects() -> [Rect; 3] {
    validation_list_column_rects_for(validation_lists_rect())
}

pub(crate) fn validation_list_height(count: usize) -> f32 {
    12.0 + count.max(1) as f32 * 24.0
}

fn validation_list_scroll_max(
    summary: &ValidationSummary,
    kind: ValidationListKind,
    rect: Rect,
) -> f32 {
    (validation_list_height(validation_list_count(summary, kind)) - (rect.h - 66.0).max(1.0))
        .max(0.0)
}

pub(crate) fn clamp_validation_list_scroll(app: &mut AppState) {
    let Some(summary) = app.validation_cache.as_ref() else {
        return;
    };
    for (kind, rect) in [
        (
            ValidationListKind::Missing,
            validation_list_column_rects()[0],
        ),
        (
            ValidationListKind::Unused,
            validation_list_column_rects()[1],
        ),
        (
            ValidationListKind::Warnings,
            validation_list_column_rects()[2],
        ),
    ] {
        let slot = kind.slot();
        app.validation_list_scroll[slot] = app.validation_list_scroll[slot]
            .clamp(0.0, validation_list_scroll_max(summary, kind, rect));
    }
}

fn validation_list_scrollbar(
    summary: &ValidationSummary,
    kind: ValidationListKind,
    rect: Rect,
    scroll: f32,
    dragging: bool,
) {
    let max_scroll = validation_list_scroll_max(summary, kind, rect);
    if max_scroll <= 0.5 {
        return;
    }
    let track = Rect::new(
        rect.x + rect.w - 10.0,
        rect.y + 54.0,
        5.0,
        (rect.h - 64.0).max(20.0),
    );
    if let Some(metrics) = scrollbar_metrics(track, track.h, track.h + max_scroll, 24.0, scroll) {
        draw_scrollbar(metrics, scrollbar_visual_state(track, dragging));
    }
}

pub(crate) fn draw_validation_list(
    font: &Font,
    summary: &ValidationSummary,
    title: &str,
    kind: ValidationListKind,
    rect: Rect,
    scroll: f32,
    clip_top: f32,
    clip_bottom: f32,
) -> f32 {
    let count = validation_list_count(summary, kind);
    let accent = match kind {
        ValidationListKind::Missing => Color::new(0.95, 0.55, 0.18, 1.0),
        ValidationListKind::Unused => Color::new(0.42, 0.60, 0.78, 1.0),
        ValidationListKind::Warnings => Color::new(0.86, 0.72, 0.30, 1.0),
    };
    draw_rrect_bordered(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        12.0,
        1.0,
        ui_panel_bg(),
        ui_border(),
    );
    let compact = rect.w < 180.0;
    ui_text_size(
        font,
        title,
        rect.x + 14.0,
        rect.y + 28.0,
        if compact { 15 } else { 18 },
        WHITE,
    );
    let chip_w = if compact { 36.0 } else { 54.0 };
    let chip = Rect::new(rect.x + rect.w - chip_w - 12.0, rect.y + 12.0, chip_w, 22.0);
    draw_rrect(
        chip.x,
        chip.y,
        chip.w,
        chip.h,
        6.0,
        Color::new(accent.r * 0.22, accent.g * 0.22, accent.b * 0.22, 1.0),
    );
    ui_text_size(
        font,
        &count.to_string(),
        chip.x + if compact { 8.0 } else { 12.0 },
        chip.y + 16.0,
        14,
        accent,
    );
    draw_line(
        rect.x + 14.0,
        rect.y + 44.0,
        rect.x + rect.w - 14.0,
        rect.y + 44.0,
        1.0,
        Color::new(0.13, 0.16, 0.20, 1.0),
    );

    let x = rect.x + 12.0;
    let y = rect.y + 64.0 - scroll;
    if count == 0 {
        let row_y = rect.y + 78.0;
        if row_y >= clip_top && row_y <= clip_bottom {
            ui_text(font, "None", x, row_y, ui_dim());
        }
        return y + validation_list_height(count);
    }
    let first = ((clip_top - y) / 24.0).floor().max(0.0) as usize;
    let last = ((clip_bottom - y) / 24.0).ceil().max(0.0) as usize;
    let mouse: Vec2 = mouse_position().into();
    for idx in first..=last.min(count.saturating_sub(1)) {
        let row_y = y + idx as f32 * 24.0;
        if row_y + 18.0 < clip_top || row_y > clip_bottom {
            continue;
        }
        if let Some(item) = validation_list_item(summary, kind, idx) {
            let row = Rect::new(rect.x + 8.0, row_y - 16.0, rect.w - 16.0, 22.0);
            let hovered = row.contains(mouse);
            let bg = if hovered {
                ui_surface_hover()
            } else if idx % 2 == 0 {
                Color::new(0.065, 0.083, 0.111, 0.72)
            } else {
                Color::new(0.046, 0.061, 0.083, 0.72)
            };
            draw_rrect(row.x, row.y, row.w, row.h, 5.0, bg);
            let (kind_label, name) = validation_item_label_parts(&item);
            let pill_w = if compact { 42.0 } else { 64.0 };
            let pill = Rect::new(row.x + 6.0, row.y + 4.0, pill_w, 14.0);
            draw_rrect(
                pill.x,
                pill.y,
                pill.w,
                pill.h,
                4.0,
                Color::new(accent.r * 0.20, accent.g * 0.20, accent.b * 0.20, 1.0),
            );
            ui_text_size(font, kind_label, pill.x + 6.0, pill.y + 11.0, 11, accent);
            let name_x = pill.x + pill.w + 10.0;
            let max_chars = ((row.x + row.w - name_x - 6.0) / 8.0).max(3.0) as usize;
            ui_text_size(
                font,
                &ellipsize(&name, max_chars),
                name_x,
                row_y,
                14,
                if count == 0 { ui_dim() } else { LIGHTGRAY },
            );
        }
    }
    y + validation_list_height(count)
}

pub(crate) fn validation_item_label_parts(item: &ValidationItem) -> (&'static str, String) {
    match item.kind {
        MissingAssetKind::Dff => ("DFF", item.key.clone()),
        MissingAssetKind::Col => ("COL", item.key.clone()),
        MissingAssetKind::Txd => ("TXD", item.key.clone()),
        MissingAssetKind::Definition => ("DEF", item.key.clone()),
    }
}

pub(crate) fn draw_validation_summary_card(
    font: &Font,
    rect: Rect,
    label: &str,
    value: impl std::fmt::Display,
    accent: Color,
) {
    draw_rrect(
        rect.x + 1.0,
        rect.y + 3.0,
        rect.w,
        rect.h,
        11.0,
        ui_shadow(),
    );
    draw_rrect_bordered(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        11.0,
        1.0,
        ui_surface(),
        ui_border(),
    );
    draw_rrect(rect.x, rect.y + 10.0, 4.0, rect.h - 20.0, 2.0, accent);
    ui_text_size(font, label, rect.x + 16.0, rect.y + 24.0, 13, ui_dim());
    ui_text_bold(&value.to_string(), rect.x + 16.0, rect.y + 54.0, 24, WHITE);
}

fn validation_action_label(app: &AppState, action: usize) -> String {
    match action {
        0 => "Review Unused Assets".to_string(),
        1 => "Repair Object Bounds".to_string(),
        2 => "Optimize Loaded Assets".to_string(),
        3 => "Review TXD Cleanup".to_string(),
        4 => "Repair DFFs".to_string(),
        5 => "Review LOD Repairs".to_string(),
        6 => format!(
            "Geometry: {}",
            collision_generation_preset_label(app.collision_generation_preset)
        ),
        7 => format!(
            "Default Override: {}",
            col_material_label(app.collision_generation_fallback_material)
        ),
        8 => "Rebuild All Collisions".to_string(),
        9 => "Fix & Organize IMG Archives".to_string(),
        10 => "Review Missing Textures".to_string(),
        11 => "Fix Duplicate IMG Entries".to_string(),
        12 => "Fix DFF Material Limits".to_string(),
        13 => "Chunk Oversized Elements".to_string(),
        14 => "Classify Elements".to_string(),
        15 => app
            .light_lod_job
            .as_ref()
            .map(LightLodJob::progress_label)
            .unwrap_or_else(|| "Light LODs".to_string()),
        _ => "Unknown Action".to_string(),
    }
}

fn validation_action_description(action: usize) -> &'static str {
    match action {
        0 => "Review unused assets and definitions before removing them.",
        1 => "Repair object bounds using DFF and collision geometry.",
        2 => "Optimize textures, DFF geometry, and COL geometry.",
        3 => "Review unused textures, formats, mipmaps, and TXD consolidation.",
        4 => "Choose DFF structure/data repairs and/or texture-name sanitation.",
        5 => "Review and repair missing or invalid LOD relationships.",
        6 => "Cycle the geometry detail used when rebuilding collisions.",
        7 => "Choose the default material used when a source material has no collision properties.",
        8 => "Rebuild collision assets for the full project with the current setup.",
        9 => "Repair MTA-incompatible IMG names, then organize assets into balanced archives.",
        10 => "Flag non-SA scene models with unresolved material textures, including LODs.",
        11 => {
            "Find duplicate names across project IMG archives and repair them after confirmation."
        }
        12 => {
            "Automatically compact or split every referenced DFF that exceeds GTA:SA's 152-material geometry limit."
        }
        13 => {
            "Review placed DFFs larger than the chosen cell size, then spatially cut selected models into separate render elements."
        }
        14 => {
            "Retag elements as object or building: physics elements always stay objects, everything else is decided by a size you choose."
        }
        15 => {
            "Asynchronously transfer matching day and night vertex lighting from every detail model to its assigned LOD. Each unique LOD DFF is processed once."
        }
        _ => "Validation action.",
    }
}

fn validation_action_busy(app: &AppState, action: usize) -> bool {
    match action {
        0 => app.purge_unused_job.is_some(),
        1 => app.object_bounds_fix_job.is_some(),
        2 => app.asset_optimization_scan_rx.is_some() || app.asset_optimization_job.is_some(),
        3 => app.txd_cleanup_job.is_some(),
        4 => app.dff_repair_rx.is_some() || app.dff_repair_refresh.is_some(),
        5..=7 | 10 | 14 => false,
        8 => app.collision_generation_job.is_some() || app.shadow_mesh_generation_job.is_some(),
        9 | 11 => app.img_archive_rebalance_job.is_some(),
        12 => app.dff_material_limit_repair_job.is_some(),
        13 => app.oversized_chunk_job.is_some(),
        15 => app.light_lod_job.is_some(),
        _ => false,
    }
}

pub(crate) fn draw_validation_panel(app: &mut AppState) {
    ensure_validation_cache(app);
    let summary = app.validation_cache.as_ref().unwrap();
    let panel = validation_panel_rect();
    let x = panel.x;
    let y = panel.y;
    draw_rrect(
        panel.x + 2.0,
        panel.y + 4.0,
        panel.w,
        panel.h,
        15.0,
        ui_shadow(),
    );
    draw_rrect_bordered(
        panel.x,
        panel.y,
        panel.w,
        panel.h,
        15.0,
        1.0,
        Color::new(0.045, 0.048, 0.054, 1.0),
        ui_border(),
    );

    draw_rrect(x + 18.0, y + 15.0, 3.0, 40.0, 1.5, ui_accent());
    ui_text_bold("Validation", x + 30.0, y + 31.0, 21, WHITE);
    ui_text_size(
        &app.ui_font,
        "Resource audit",
        x + 30.0,
        y + 54.0,
        13,
        ui_dim(),
    );

    draw_validation_summary_card(
        &app.ui_font,
        validation_summary_card_rect(0),
        "Issues",
        summary.issue_count(),
        Color::new(0.90, 0.35, 0.28, 1.0),
    );
    draw_validation_summary_card(
        &app.ui_font,
        validation_summary_card_rect(1),
        "Referenced DFF",
        summary.referenced_dffs.len(),
        Color::new(0.48, 0.52, 0.58, 1.0),
    );
    draw_validation_summary_card(
        &app.ui_font,
        validation_summary_card_rect(2),
        "IMG DFF/COL/TXD",
        format!(
            "{}/{}/{}",
            summary.img_dffs.len(),
            summary.img_cols.len(),
            summary.img_txds.len()
        ),
        Color::new(0.40, 0.70, 0.55, 1.0),
    );
    draw_validation_summary_card(
        &app.ui_font,
        validation_summary_card_rect(3),
        "Loose DFF/COL/TXD",
        format!(
            "{}/{}/{}",
            summary.loose_dffs.len(),
            summary.loose_cols.len(),
            summary.loose_txds.len()
        ),
        Color::new(0.84, 0.68, 0.34, 1.0),
    );

    let [missing_rect, unused_rect, warnings_rect] = validation_list_column_rects();
    draw_validation_list(
        &app.ui_font,
        summary,
        "Missing",
        ValidationListKind::Missing,
        missing_rect,
        app.validation_list_scroll[ValidationListKind::Missing.slot()],
        missing_rect.y + 54.0,
        missing_rect.y + missing_rect.h - 10.0,
    );

    draw_validation_list(
        &app.ui_font,
        summary,
        "Unused",
        ValidationListKind::Unused,
        unused_rect,
        app.validation_list_scroll[ValidationListKind::Unused.slot()],
        unused_rect.y + 54.0,
        unused_rect.y + unused_rect.h - 10.0,
    );

    draw_validation_list(
        &app.ui_font,
        summary,
        "Warnings",
        ValidationListKind::Warnings,
        warnings_rect,
        app.validation_list_scroll[ValidationListKind::Warnings.slot()],
        warnings_rect.y + 54.0,
        warnings_rect.y + warnings_rect.h - 10.0,
    );
    validation_list_scrollbar(
        summary,
        ValidationListKind::Missing,
        missing_rect,
        app.validation_list_scroll[ValidationListKind::Missing.slot()],
        app.validation_list_scroll_drag == Some(ValidationListKind::Missing.slot()),
    );
    validation_list_scrollbar(
        summary,
        ValidationListKind::Unused,
        unused_rect,
        app.validation_list_scroll[ValidationListKind::Unused.slot()],
        app.validation_list_scroll_drag == Some(ValidationListKind::Unused.slot()),
    );
    validation_list_scrollbar(
        summary,
        ValidationListKind::Warnings,
        warnings_rect,
        app.validation_list_scroll[ValidationListKind::Warnings.slot()],
        app.validation_list_scroll_drag == Some(ValidationListKind::Warnings.slot()),
    );

    for (slot, category) in VALIDATION_CATEGORIES.iter().copied().enumerate() {
        text_button(
            &app.ui_font,
            validation_category_tab_rect(slot),
            validation_category_label(category),
            app.validation_action_category == category,
        );
    }

    let validation_busy = validation_operation_conflict(app).is_some();
    let mouse: Vec2 = mouse_position().into();
    let draw_action = |rect: Rect, action: usize| {
        let label = validation_action_label(app, action);
        let busy = validation_action_busy(app, action);
        if busy {
            text_button_busy(&app.ui_font, rect, &label);
        } else if validation_busy {
            text_button_disabled(
                &app.ui_font,
                rect,
                &label,
                "Wait for the current Validation operation to finish.",
            );
        } else {
            text_button(&app.ui_font, rect, &label, false);
            if rect.contains(mouse)
                && !app.asset_optimization_menu_open
                && !app.dff_repair_menu_open
                && !app.validation_collision_material_dropdown_open
            {
                draw_text_tooltip(&app.ui_font, rect, validation_action_description(action));
            }
        }
    };
    for (position, action) in validation_category_actions(app.validation_action_category)
        .iter()
        .copied()
        .enumerate()
    {
        draw_action(
            validation_action_button_rect(app.validation_action_category, position),
            action,
        );
    }

    if app.asset_optimization_menu_open {
        let menu = validation_optimization_menu_rect();
        draw_rectangle(
            menu.x + 5.0,
            menu.y + 6.0,
            menu.w,
            menu.h,
            Color::new(0.0, 0.0, 0.0, 0.38),
        );
        draw_rrect_bordered(
            menu.x,
            menu.y,
            menu.w,
            menu.h,
            8.0,
            1.0,
            Color::new(0.055, 0.064, 0.078, 0.99),
            Color::new(0.30, 0.38, 0.48, 1.0),
        );
        ui_text_bold(
            "Optimize Loaded Assets",
            menu.x + 16.0,
            menu.y + 25.0,
            16,
            WHITE,
        );
        draw_checkbox(
            &app.ui_font,
            validation_optimization_scope_rect(0),
            "Textures",
            app.asset_optimization_scope.textures,
        );
        draw_checkbox(
            &app.ui_font,
            validation_optimization_scope_rect(1),
            "DFF Geometry",
            app.asset_optimization_scope.dffs,
        );
        draw_checkbox(
            &app.ui_font,
            validation_optimization_scope_rect(2),
            "COL Geometry",
            app.asset_optimization_scope.cols,
        );
        text_button(
            &app.ui_font,
            validation_optimization_continue_rect(),
            if app.asset_optimization_scope.any() {
                "Continue"
            } else {
                "Select at least one"
            },
            false,
        );
    }
    if app.dff_repair_menu_open {
        let menu = validation_dff_repair_menu_rect();
        draw_rectangle(
            menu.x + 5.0,
            menu.y + 6.0,
            menu.w,
            menu.h,
            Color::new(0.0, 0.0, 0.0, 0.38),
        );
        draw_rrect_bordered(
            menu.x,
            menu.y,
            menu.w,
            menu.h,
            8.0,
            1.0,
            Color::new(0.055, 0.064, 0.078, 0.99),
            Color::new(0.30, 0.38, 0.48, 1.0),
        );
        ui_text_bold("Repair DFFs", menu.x + 16.0, menu.y + 25.0, 16, WHITE);
        draw_checkbox(
            &app.ui_font,
            validation_dff_repair_scope_rect(0),
            "DFF Structure & Data",
            app.dff_repair_scope.dff_issues,
        );
        draw_checkbox(
            &app.ui_font,
            validation_dff_repair_scope_rect(1),
            "Missing Prelighting",
            app.dff_repair_scope.prelighting,
        );
        draw_checkbox(
            &app.ui_font,
            validation_dff_repair_scope_rect(2),
            "Texture Names",
            app.dff_repair_scope.texture_names,
        );
        text_button(
            &app.ui_font,
            validation_dff_repair_continue_rect(),
            if app.dff_repair_scope.any() {
                "Continue"
            } else {
                "Select at least one"
            },
            false,
        );
    }
    if app.validation_collision_material_dropdown_open {
        let dropdown = validation_collision_material_dropdown_rect();
        draw_rectangle(
            dropdown.x + 5.0,
            dropdown.y + 6.0,
            dropdown.w,
            dropdown.h,
            Color::new(0.0, 0.0, 0.0, 0.38),
        );
        draw_rrect_bordered(
            dropdown.x,
            dropdown.y,
            dropdown.w,
            dropdown.h,
            8.0,
            1.0,
            Color::new(0.035, 0.043, 0.054, 0.99),
            ui_accent(),
        );
        ui_text_bold(
            "Default Collision Material",
            dropdown.x + 14.0,
            dropdown.y + 27.0,
            16,
            WHITE,
        );
        let search = validation_collision_material_search_rect();
        draw_rrect_bordered(
            search.x,
            search.y,
            search.w,
            search.h,
            6.0,
            1.0,
            Color::new(0.055, 0.064, 0.078, 1.0),
            ui_accent(),
        );
        ui_text(
            &app.ui_font,
            if app.validation_collision_material_search.is_empty() {
                "Search material name or ID..."
            } else {
                &app.validation_collision_material_search
            },
            search.x + 9.0,
            search.y + 20.0,
            if app.validation_collision_material_search.is_empty() {
                ui_muted()
            } else {
                WHITE
            },
        );

        let options =
            filtered_validation_collision_materials(&app.validation_collision_material_search);
        let list = validation_collision_material_list_rect();
        let row_h = 26.0;
        let visible = (list.h / row_h).floor().max(1.0) as usize;
        let max_start = options.len().saturating_sub(visible);
        let start = app
            .validation_collision_material_scroll
            .floor()
            .max(0.0)
            .min(max_start as f32) as usize;
        begin_ui_clip(list);
        for row in 0..visible {
            let Some((id, name)) = options.get(start + row).copied() else {
                break;
            };
            let rect = Rect::new(list.x, list.y + row as f32 * row_h, list.w, row_h - 2.0);
            let selected = id == app.collision_generation_fallback_material;
            if selected || rect.contains(mouse) {
                draw_rrect(
                    rect.x,
                    rect.y,
                    rect.w,
                    rect.h,
                    5.0,
                    if selected {
                        ui_surface_active()
                    } else {
                        ui_surface_hover()
                    },
                );
            }
            let swatch = collision_material_color(id, 1.0);
            draw_rrect(
                rect.x + 6.0,
                rect.y + 5.0,
                14.0,
                14.0,
                3.0,
                Color::new(swatch[0], swatch[1], swatch[2], 1.0),
            );
            ui_text_size(
                &app.ui_font,
                &format!("{id}: {name}"),
                rect.x + 28.0,
                rect.y + 18.0,
                14,
                if selected { ui_accent() } else { LIGHTGRAY },
            );
        }
        end_ui_clip();

        if max_start > 0 {
            let track = Rect::new(list.x + list.w - 8.0, list.y, 8.0, list.h);
            if let Some(metrics) = scrollbar_metrics(
                track,
                visible as f32,
                options.len() as f32,
                24.0,
                start as f32,
            ) {
                let state = if scrollbar_hit_area(track).contains(mouse) {
                    ScrollbarVisualState::Hovered
                } else {
                    ScrollbarVisualState::Idle
                };
                draw_scrollbar(metrics, state);
            }
        }
    }
}

pub(crate) fn validation_missing_item_at(
    app: &AppState,
    summary: &ValidationSummary,
    mouse: Vec2,
) -> Option<ValidationItem> {
    let lists = validation_lists_rect();
    let clip_top = lists.y + 54.0;
    let clip_bottom = lists.y + lists.h - 10.0;
    for (kind, column) in [
        ValidationListKind::Missing,
        ValidationListKind::Unused,
        ValidationListKind::Warnings,
    ]
    .into_iter()
    .zip(validation_list_column_rects())
    {
        let x = column.x + 8.0;
        let y = lists.y + 64.0 - app.validation_list_scroll[kind.slot()];
        for idx in 0..validation_list_count(summary, kind) {
            let row_y = y + idx as f32 * 24.0;
            let rect = Rect::new(x, row_y - 16.0, column.w - 16.0, 22.0);
            if row_y + 18.0 >= clip_top && row_y <= clip_bottom && rect.contains(mouse) {
                return validation_list_item(summary, kind, idx);
            }
        }
    }
    None
}

pub(crate) fn missing_item_target_index(app: &AppState, item: &ValidationItem) -> Option<usize> {
    let key = lower(&item.key);
    let mut fallback = None;
    for (idx, placement) in app.placements.iter().enumerate() {
        let matched = match item.kind {
            MissingAssetKind::Dff => {
                asset_key(&placement.dff, ".dff") == key
                    || app.definitions.get(&placement.id).is_some_and(|def| {
                        asset_key_opt(def.attrs.get("dff"), &def.id, ".dff") == key
                    })
            }
            MissingAssetKind::Col => app.definitions.get(&placement.id).is_some_and(|def| {
                let dff_key = asset_key_opt(def.attrs.get("dff"), &def.id, ".dff");
                let col_key = def
                    .attrs
                    .get("col")
                    .map(|value| value.trim())
                    .filter(|value| !value.is_empty())
                    .map(|value| asset_key(value, ".col"))
                    .unwrap_or_else(|| {
                        dff_key
                            .strip_suffix(".dff")
                            .map(|base| format!("{base}.col"))
                            .unwrap_or_else(|| asset_key(&def.id, ".col"))
                    });
                col_key == key
            }),
            MissingAssetKind::Txd => app.definitions.get(&placement.id).is_some_and(|def| {
                definition_txd_name_from_attrs(def).is_some_and(|txd| asset_key(txd, ".txd") == key)
            }),
            MissingAssetKind::Definition => placement.id.eq_ignore_ascii_case(&item.key),
        };
        if !matched {
            continue;
        }
        if is_live_element(app, idx) {
            return Some(idx);
        }
        fallback.get_or_insert(idx);
    }
    fallback
}

pub(crate) fn snap_to_validation_missing_item(app: &mut AppState, item: ValidationItem) {
    if let Some(index) = missing_item_target_index(app, &item) {
        snap_to(app, index);
        if item.kind == MissingAssetKind::Dff && item.label.contains("materials; GTA:SA is limited")
        {
            open_selected_dff_in_editing(app);
            return;
        }
        app.active_tab = AppTab::Preview;
        app.properties_scroll = 0.0;
        app.status_message = format!("Snapped to scene item referencing {}.", item.label);
    } else {
        app.status_message = format!("No scene item references {}.", item.label);
    }
}

pub(crate) fn handle_validation_collision_material_dropdown_input(
    app: &mut AppState,
    mouse: Vec2,
    wheel: f32,
) -> bool {
    let active_context = app.active_tab == AppTab::Validation
        && app.validation_action_category == ValidationActionCategory::Collision;
    if !active_context {
        close_validation_collision_material_dropdown(app);
        return false;
    }
    if !app.validation_collision_material_dropdown_open {
        return false;
    }

    if is_key_pressed(KeyCode::Escape) {
        close_validation_collision_material_dropdown(app);
        return true;
    }
    let mut search_changed = false;
    if is_key_pressed(KeyCode::Backspace) {
        app.validation_collision_material_search.pop();
        search_changed = true;
    }
    while let Some(ch) = get_char_pressed() {
        if !ch.is_control() && app.validation_collision_material_search.len() < 64 {
            app.validation_collision_material_search.push(ch);
            search_changed = true;
        }
    }
    if search_changed {
        app.validation_collision_material_scroll = 0.0;
    }

    let options =
        filtered_validation_collision_materials(&app.validation_collision_material_search);
    let list = validation_collision_material_list_rect();
    let row_h = 26.0;
    let visible = (list.h / row_h).floor().max(1.0) as usize;
    let max_start = options.len().saturating_sub(visible);
    if wheel.abs() > 0.0 && validation_collision_material_dropdown_rect().contains(mouse) {
        app.validation_collision_material_scroll =
            (app.validation_collision_material_scroll - wheel * 3.0).clamp(0.0, max_start as f32);
        return true;
    }
    if is_key_pressed(KeyCode::Down) {
        app.validation_collision_material_scroll =
            (app.validation_collision_material_scroll + 1.0).min(max_start as f32);
        return true;
    }
    if is_key_pressed(KeyCode::Up) {
        app.validation_collision_material_scroll =
            (app.validation_collision_material_scroll - 1.0).max(0.0);
        return true;
    }
    if is_key_pressed(KeyCode::PageDown) {
        app.validation_collision_material_scroll =
            (app.validation_collision_material_scroll + visible as f32).min(max_start as f32);
        return true;
    }
    if is_key_pressed(KeyCode::PageUp) {
        app.validation_collision_material_scroll =
            (app.validation_collision_material_scroll - visible as f32).max(0.0);
        return true;
    }

    let start = app
        .validation_collision_material_scroll
        .floor()
        .max(0.0)
        .min(max_start as f32) as usize;
    if is_key_pressed(KeyCode::Enter) {
        if let Some((id, name)) = options.get(start).copied() {
            app.collision_generation_fallback_material = id;
            app.status_message = format!("Default collision material: {id}: {name}");
            close_validation_collision_material_dropdown(app);
        }
        return true;
    }

    let track = Rect::new(list.x + list.w - 8.0, list.y, 8.0, list.h);
    if is_mouse_button_down(MouseButton::Left) && track.contains(mouse) && max_start > 0 {
        let ratio = ((mouse.y - track.y) / track.h).clamp(0.0, 1.0);
        app.validation_collision_material_scroll = ratio * max_start as f32;
        return true;
    }
    if !is_mouse_button_pressed(MouseButton::Left) {
        return true;
    }

    let anchor = validation_action_button_rect(ValidationActionCategory::Collision, 1);
    if anchor.contains(mouse) {
        close_validation_collision_material_dropdown(app);
        return true;
    }
    if list.contains(mouse) {
        let row = ((mouse.y - list.y) / row_h).floor().max(0.0) as usize;
        if let Some((id, name)) = options.get(start + row).copied() {
            app.collision_generation_fallback_material = id;
            app.status_message = format!("Default collision material: {id}: {name}");
            close_validation_collision_material_dropdown(app);
        }
        return true;
    }
    if validation_collision_material_dropdown_rect().contains(mouse) {
        return true;
    }

    close_validation_collision_material_dropdown(app);
    false
}

pub(crate) fn handle_validation_scroll(app: &mut AppState, mouse: Vec2, wheel: f32) -> bool {
    if app.active_tab != AppTab::Validation || wheel.abs() <= 0.0 {
        return false;
    }
    ensure_validation_cache(app);
    let summary = app.validation_cache.as_ref().unwrap();
    for (kind, rect) in [
        (
            ValidationListKind::Missing,
            validation_list_column_rects()[0],
        ),
        (
            ValidationListKind::Unused,
            validation_list_column_rects()[1],
        ),
        (
            ValidationListKind::Warnings,
            validation_list_column_rects()[2],
        ),
    ] {
        if rect.contains(mouse) {
            let max_scroll = validation_list_scroll_max(summary, kind, rect);
            let slot = kind.slot();
            app.validation_list_scroll[slot] =
                (app.validation_list_scroll[slot] - wheel * 36.0).clamp(0.0, max_scroll);
            return true;
        }
    }
    false
}

pub(crate) fn handle_validation_scrollbar_drag(app: &mut AppState, mouse: Vec2) -> bool {
    let Some(slot) = app.validation_list_scroll_drag else {
        return false;
    };
    if !is_mouse_button_down(MouseButton::Left) {
        app.validation_list_scroll_drag = None;
        return true;
    }
    ensure_validation_cache(app);
    let summary = app.validation_cache.as_ref().unwrap();
    let kind = match slot {
        0 => ValidationListKind::Missing,
        1 => ValidationListKind::Unused,
        2 => ValidationListKind::Warnings,
        _ => return false,
    };
    let rect = validation_list_column_rects()[slot];
    let track = Rect::new(
        rect.x + rect.w - 10.0,
        rect.y + 54.0,
        5.0,
        (rect.h - 64.0).max(20.0),
    );
    let max_scroll = validation_list_scroll_max(summary, kind, rect);
    if let Some(metrics) = scrollbar_metrics(
        track,
        track.h,
        track.h + max_scroll,
        24.0,
        app.validation_list_scroll[slot],
    ) {
        app.validation_list_scroll[slot] = scrollbar_scroll_for_drag(
            metrics,
            ScrollbarDrag {
                grab_offset_y: app.validation_list_scroll_grab_offset_y,
            },
            mouse,
        );
    }
    true
}

pub(crate) fn handle_validation_click(app: &mut AppState, mouse: Vec2) -> bool {
    if app.active_tab != AppTab::Validation {
        return false;
    }
    if is_mouse_button_pressed(MouseButton::Left) {
        ensure_validation_cache(app);
        let summary = app.validation_cache.as_ref().unwrap();
        for (kind, rect) in [
            (
                ValidationListKind::Missing,
                validation_list_column_rects()[0],
            ),
            (
                ValidationListKind::Unused,
                validation_list_column_rects()[1],
            ),
            (
                ValidationListKind::Warnings,
                validation_list_column_rects()[2],
            ),
        ] {
            let track = Rect::new(
                rect.x + rect.w - 10.0,
                rect.y + 54.0,
                5.0,
                (rect.h - 64.0).max(20.0),
            );
            let max_scroll = validation_list_scroll_max(summary, kind, rect);
            if let Some(metrics) = scrollbar_metrics(
                track,
                track.h,
                track.h + max_scroll,
                24.0,
                app.validation_list_scroll[kind.slot()],
            ) && let Some(drag) = scrollbar_begin_drag(metrics, mouse)
            {
                app.validation_list_scroll_drag = Some(kind.slot());
                app.validation_list_scroll_grab_offset_y = drag.grab_offset_y;
                app.scrollbar_pointer_captured = true;
                set_scrollbar_hover_suppressed(true);
                return true;
            }
        }
    }
    let left_clicked = is_mouse_button_pressed(MouseButton::Left);
    if app.dff_repair_menu_open && left_clicked {
        if validation_action_anchor_rect(ValidationActionCategory::Repair, 4).contains(mouse) {
            app.dff_repair_menu_open = false;
            return true;
        }
        for slot in 0..3 {
            if validation_dff_repair_scope_rect(slot).contains(mouse) {
                match slot {
                    0 => app.dff_repair_scope.dff_issues = !app.dff_repair_scope.dff_issues,
                    1 => app.dff_repair_scope.prelighting = !app.dff_repair_scope.prelighting,
                    2 => app.dff_repair_scope.texture_names = !app.dff_repair_scope.texture_names,
                    _ => {}
                }
                app.status_message = "Updated Repair DFFs selection.".to_string();
                return true;
            }
        }
        if validation_dff_repair_continue_rect().contains(mouse) {
            if app.dff_repair_scope.any() {
                app.dff_repair_menu_open = false;
                request_dff_repair(app);
            } else {
                app.status_message = "Select at least one Repair DFFs category first.".to_string();
            }
            return true;
        }
        if validation_dff_repair_menu_rect().contains(mouse) {
            return true;
        }
        app.dff_repair_menu_open = false;
        return true;
    }
    if app.asset_optimization_menu_open && left_clicked {
        if validation_action_anchor_rect(ValidationActionCategory::Optimize, 2).contains(mouse) {
            app.asset_optimization_menu_open = false;
            return true;
        }
        for slot in 0..3 {
            if validation_optimization_scope_rect(slot).contains(mouse) {
                match slot {
                    0 => {
                        app.asset_optimization_scope.textures =
                            !app.asset_optimization_scope.textures
                    }
                    1 => app.asset_optimization_scope.dffs = !app.asset_optimization_scope.dffs,
                    2 => app.asset_optimization_scope.cols = !app.asset_optimization_scope.cols,
                    _ => {}
                }
                app.status_message = "Updated Optimize Loaded Assets selection.".to_string();
                return true;
            }
        }
        if validation_optimization_continue_rect().contains(mouse) {
            if app.asset_optimization_scope.any() {
                app.asset_optimization_menu_open = false;
                show_asset_optimization_profile_choice(app);
            } else {
                app.status_message =
                    "Select at least one Optimize Loaded Assets category first.".to_string();
            }
            return true;
        }
        if validation_optimization_menu_rect().contains(mouse) {
            return true;
        }
        app.asset_optimization_menu_open = false;
        return true;
    }
    if left_clicked {
        for (slot, category) in VALIDATION_CATEGORIES.iter().copied().enumerate() {
            if validation_category_tab_rect(slot).contains(mouse) {
                app.validation_action_category = category;
                app.asset_optimization_menu_open = false;
                app.dff_repair_menu_open = false;
                close_validation_collision_material_dropdown(app);
                app.status_message =
                    format!("Validation tools: {}.", validation_category_label(category));
                return true;
            }
        }
    }
    for (position, action) in validation_category_actions(app.validation_action_category)
        .iter()
        .copied()
        .enumerate()
    {
        if validation_action_button_rect(app.validation_action_category, position).contains(mouse) {
            if left_clicked {
                if let Some(conflict) = validation_operation_conflict(app) {
                    app.status_message =
                        format!("Wait for the background {conflict} to finish first.");
                    return true;
                }
                match action {
                    0 => request_purge_unused_assets(app),
                    1 => {
                        request_object_bounds_fix(app);
                    }
                    2 => {
                        app.asset_optimization_menu_open = true;
                        app.dff_repair_menu_open = false;
                        close_validation_collision_material_dropdown(app);
                    }
                    3 => request_txd_cleanup(app),
                    4 => {
                        app.dff_repair_menu_open = true;
                        app.asset_optimization_menu_open = false;
                        close_validation_collision_material_dropdown(app);
                    }
                    5 => request_fix_lods(app),
                    6 => cycle_collision_generation_preset(app),
                    7 => {
                        app.validation_collision_material_dropdown_open = true;
                        app.validation_collision_material_search.clear();
                        app.validation_collision_material_scroll = 0.0;
                        app.asset_optimization_menu_open = false;
                        app.dff_repair_menu_open = false;
                        drain_text_input();
                    }
                    8 => {
                        request_global_collision_generation(
                            app,
                            app.collision_generation_preset,
                            app.collision_generation_fallback_material,
                        );
                    }
                    9 => request_img_archive_rebalance(app),
                    10 => {
                        switch_app_tab(app, AppTab::TextureReview);
                        app.properties_scroll = 0.0;
                        if app.missing_texture_review.rx.is_none() {
                            request_missing_texture_review(app);
                        }
                    }
                    11 => request_img_duplicate_fix(app),
                    12 => request_automatic_dff_material_limit_repair(app),
                    13 => open_oversized_chunk_dialog(app),
                    14 => open_classify_dialog(app),
                    15 => {
                        request_validation_light_lods(app);
                    }
                    _ => {}
                }
            }
            return true;
        }
    }
    if is_mouse_button_pressed(MouseButton::Left) {
        ensure_validation_cache(app);
        if let Some(summary) = app.validation_cache.as_ref() {
            if let Some(item) = validation_missing_item_at(app, summary, mouse) {
                snap_to_validation_missing_item(app, item);
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect_contains_rect(outer: Rect, inner: Rect) -> bool {
        inner.x >= outer.x
            && inner.y >= outer.y
            && inner.x + inner.w <= outer.x + outer.w
            && inner.y + inner.h <= outer.y + outer.h
    }

    #[test]
    fn optimization_menu_controls_fit_without_cutoff() {
        let panel = Rect::new(312.0, 117.0, 480.0, 480.0);
        let anchor = Rect::new(430.0, 135.0, 120.0, 30.0);
        let menu = validation_optimization_menu_rect_for(panel, anchor);
        let continue_rect = validation_optimization_continue_rect_for(menu);

        assert!(rect_contains_rect(panel, menu));
        assert!(rect_contains_rect(menu, continue_rect));
        for slot in 0..3 {
            let checkbox = validation_optimization_scope_rect_for(menu, slot);
            assert!(rect_contains_rect(menu, checkbox));
            assert!(checkbox.y + checkbox.h < continue_rect.y);
        }
    }

    #[test]
    fn validation_actions_are_grouped_once_into_named_categories() {
        let mut actions = VALIDATION_CATEGORIES
            .iter()
            .copied()
            .flat_map(validation_category_actions)
            .copied()
            .collect::<Vec<_>>();
        actions.sort_unstable();

        assert_eq!(actions, (0..16).collect::<Vec<_>>());
        assert_eq!(
            validation_category_actions(ValidationActionCategory::Review),
            &[10, 0, 3, 5]
        );
        assert_eq!(
            validation_category_actions(ValidationActionCategory::Repair),
            &[14, 1, 4, 11, 12, 15]
        );
        assert_eq!(
            validation_category_actions(ValidationActionCategory::Optimize),
            &[13, 2, 9]
        );
        assert_eq!(
            validation_category_actions(ValidationActionCategory::Collision),
            &[6, 7, 8]
        );
    }

    #[test]
    fn popup_menus_anchor_to_the_button_that_opens_them() {
        assert_eq!(
            validation_action_position(ValidationActionCategory::Repair, 4),
            2
        );
        assert_eq!(
            validation_action_position(ValidationActionCategory::Optimize, 2),
            1
        );
        assert_eq!(
            validation_action_position(ValidationActionCategory::Collision, 7),
            1
        );
    }

    #[test]
    fn collision_material_override_search_matches_names_and_ids() {
        let metal = filtered_validation_collision_materials("metal");
        assert!(metal.iter().any(|(id, _)| *id == 51));
        assert!(metal.iter().any(|(id, _)| *id == 162));

        let material_178 = filtered_validation_collision_materials("178");
        assert_eq!(material_178, vec![(178, "Rail Track")]);

        assert_eq!(
            filtered_validation_collision_materials("default"),
            vec![(0, "Default")]
        );
    }

    #[test]
    fn collision_material_dropdown_stays_inside_validation_panel() {
        let panel = Rect::new(200.0, 80.0, 500.0, 480.0);
        let anchor = Rect::new(360.0, 130.0, 140.0, 30.0);
        let dropdown = validation_collision_material_dropdown_rect_for(panel, anchor);

        assert!(rect_contains_rect(panel, dropdown));
        assert_eq!(dropdown.y, anchor.y + anchor.h + 6.0);
    }

    #[test]
    fn validation_columns_stay_inside_narrow_list_area() {
        let area = Rect::new(10.0, 20.0, 452.0, 180.0);
        let columns = validation_list_column_rects_for(area);

        for column in columns {
            assert!(rect_contains_rect(area, column));
            assert!(column.w > 100.0);
        }
        assert!(columns[0].x + columns[0].w < columns[1].x);
        assert!(columns[1].x + columns[1].w < columns[2].x);
        assert!((columns[2].x + columns[2].w - (area.x + area.w)).abs() < 0.001);
    }
}
