use super::super::*;

pub(crate) fn water_button_rect(app: &AppState, slot: usize) -> Rect {
    let step = (right_panel_width() - 30.0) / 4.0;
    let x = screen_width() - right_panel_width() + 18.0 + slot as f32 * step;
    Rect::new(x, TOP_H + 54.0 - app.properties_scroll, step - 6.0, 28.0)
}

pub(crate) fn water_split_button_rect(app: &AppState, axis: WaterSplitAxis) -> Rect {
    let x = screen_width() - right_panel_width()
        + match axis {
            WaterSplitAxis::X => 18.0,
            WaterSplitAxis::Y => 186.0,
        };
    Rect::new(x, TOP_H + 522.0 - app.properties_scroll, 150.0, 28.0)
}

pub(crate) fn water_edge_snap_toggle_rect(app: &AppState) -> Rect {
    Rect::new(
        screen_width() - 150.0,
        TOP_H + 432.0 - app.properties_scroll,
        126.0,
        28.0,
    )
}

pub(crate) fn water_list_rect(app: &AppState) -> Rect {
    Rect::new(
        screen_width() - right_panel_width() + 22.0,
        TOP_H + 116.0 - app.properties_scroll,
        right_panel_width() - 44.0,
        304.0,
    )
}

pub(crate) fn water_visible_rows() -> usize {
    12
}

pub(crate) fn clamp_water_scroll(app: &mut AppState) {
    let max_scroll = app.water_planes.len().saturating_sub(water_visible_rows()) as f32;
    app.water_scroll = app.water_scroll.clamp(0.0, max_scroll);
}

pub(crate) fn water_row_at(app: &AppState, mouse: Vec2) -> Option<usize> {
    let rect = water_list_rect(app);
    let row_h = 24.0;
    if !rect.contains(mouse) {
        return None;
    }
    let local_row = ((mouse.y - rect.y - 12.0) / row_h).floor().max(0.0) as usize;
    Some(app.water_scroll.floor() as usize + local_row)
}

pub(crate) fn selected_water_plane(app: &AppState) -> Option<&WaterPlane> {
    app.water_planes.get(app.selected_water)
}

pub(crate) fn selected_water_plane_mut(app: &mut AppState) -> Option<&mut WaterPlane> {
    app.water_planes.get_mut(app.selected_water)
}

pub(crate) fn prune_selected_water(app: &mut AppState) {
    let len = app.water_planes.len();
    app.selected_water_planes.retain(|idx| *idx < len);
    if len == 0 {
        app.selected_water = NO_SELECTION;
        return;
    }
    if app.selected_water >= len {
        app.selected_water = app
            .selected_water_planes
            .iter()
            .next_back()
            .copied()
            .unwrap_or(0);
    }
    if app.selected_water_planes.is_empty() {
        app.selected_water_planes.insert(app.selected_water);
    }
}

pub(crate) fn select_water_with_mode(app: &mut AppState, index: usize, additive: bool) {
    if index >= app.water_planes.len() {
        return;
    }
    if additive {
        if app.selected_water_planes.contains(&index) {
            app.selected_water_planes.remove(&index);
            if app.selected_water == index {
                app.selected_water = app
                    .selected_water_planes
                    .iter()
                    .next_back()
                    .copied()
                    .unwrap_or(NO_SELECTION);
            }
        } else {
            app.selected_water_planes.insert(index);
            app.selected_water = index;
        }
    } else {
        app.selected_water_planes.clear();
        app.selected_water_planes.insert(index);
        app.selected_water = index;
    }
    prune_selected_water(app);
}

pub(crate) fn deselect_water(app: &mut AppState) {
    app.selected_water_planes.clear();
    app.selected_water = NO_SELECTION;
}

pub(crate) fn selected_water_indices(app: &AppState) -> Vec<usize> {
    let mut indices: Vec<usize> = app
        .selected_water_planes
        .iter()
        .copied()
        .filter(|idx| *idx < app.water_planes.len())
        .collect();
    if indices.is_empty() && app.selected_water < app.water_planes.len() {
        indices.push(app.selected_water);
    }
    indices
}

pub(crate) fn pick_water_plane(app: &AppState, viewport: Rect, mouse: Vec2) -> Option<usize> {
    let (origin, dir) = viewport_ray(app, viewport, mouse)?;
    let mut best = None;
    let mut best_t = f32::MAX;
    for (idx, plane) in app.water_planes.iter().enumerate() {
        let points = plane
            .corners
            .map(|corner| vec3(corner.pos.x, corner.pos.y, corner.pos.z));
        for (a, b, c) in [(0usize, 1usize, 2usize), (1, 3, 2)] {
            if let Some(t) = ray_triangle(origin, dir, points[a], points[b], points[c]) {
                if t < best_t {
                    best_t = t;
                    best = Some(idx);
                }
            }
        }
    }
    best
}

fn water_edge_points(plane: &WaterPlane, edge: WaterEdge) -> (Vec3, Vec3) {
    let points = plane
        .corners
        .map(|corner| vec3(corner.pos.x, corner.pos.y, corner.pos.z));
    match edge {
        WaterEdge::South => (points[0], points[1]),
        WaterEdge::North => (points[2], points[3]),
        WaterEdge::West => (points[0], points[2]),
        WaterEdge::East => (points[1], points[3]),
    }
}

fn water_edge_value(plane: &WaterPlane, edge: WaterEdge) -> f32 {
    let (min_x, min_y, max_x, max_y, _) = water_plane_bounds(plane);
    match edge {
        WaterEdge::South => min_y,
        WaterEdge::North => max_y,
        WaterEdge::West => min_x,
        WaterEdge::East => max_x,
    }
}

fn water_edge_axis(edge: WaterEdge) -> usize {
    match edge {
        WaterEdge::North | WaterEdge::South => 1,
        WaterEdge::East | WaterEdge::West => 0,
    }
}

fn water_edge_label(edge: WaterEdge) -> &'static str {
    match edge {
        WaterEdge::North => "north",
        WaterEdge::South => "south",
        WaterEdge::East => "east",
        WaterEdge::West => "west",
    }
}

fn screen_segment_distance(point: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let len2 = ab.length_squared();
    if len2 <= 0.0001 {
        return point.distance(a);
    }
    let t = ((point - a).dot(ab) / len2).clamp(0.0, 1.0);
    point.distance(a + ab * t)
}

pub(crate) fn pick_water_edge(
    app: &AppState,
    viewport: Rect,
    mouse: Vec2,
) -> Option<(usize, WaterEdge)> {
    if app.active_tab != AppTab::Water || !viewport.contains(mouse) {
        return None;
    }
    let mut best = None;
    let mut best_dist = 12.0f32;
    for (idx, plane) in app.water_planes.iter().enumerate() {
        for edge in [
            WaterEdge::North,
            WaterEdge::South,
            WaterEdge::East,
            WaterEdge::West,
        ] {
            let (a, b) = water_edge_points(plane, edge);
            let Some(a2) = world_to_screen(app, viewport, a) else {
                continue;
            };
            let Some(b2) = world_to_screen(app, viewport, b) else {
                continue;
            };
            let dist = screen_segment_distance(mouse, a2, b2);
            if dist < best_dist {
                best_dist = dist;
                best = Some((idx, edge));
            }
        }
    }
    best
}

pub(crate) fn update_water_edge_hover(app: &mut AppState, viewport: Rect, mouse: Vec2) {
    if app.active_tab != AppTab::Water || app.water_edge_drag.is_some() {
        app.hovered_water_edge = None;
        return;
    }
    app.hovered_water_edge = pick_water_edge(app, viewport, mouse);
    if let Some((idx, _)) = app.hovered_water_edge {
        app.hovered_water = Some(idx);
    }
}

fn water_drag_value_at_mouse(
    app: &AppState,
    viewport: Rect,
    mouse: Vec2,
    plane_idx: usize,
    edge: WaterEdge,
) -> Option<f32> {
    let (origin, dir) = viewport_ray(app, viewport, mouse)?;
    let plane = app.water_planes.get(plane_idx)?;
    let (_, _, _, _, z) = water_plane_bounds(plane);
    if dir.z.abs() < 0.00001 {
        return None;
    }
    let t = (z - origin.z) / dir.z;
    const MAX_WATER_DRAG_RAY_DISTANCE: f32 = 16_000.0;
    if !t.is_finite() || t <= 0.0 || t > MAX_WATER_DRAG_RAY_DISTANCE {
        return None;
    }
    let hit = origin + dir * t;
    let value = if water_edge_axis(edge) == 0 {
        hit.x
    } else {
        hit.y
    };
    value.is_finite().then_some(value)
}

fn snapped_water_edge_value(
    planes: &[WaterPlane],
    plane_idx: usize,
    edge: WaterEdge,
    start_value: f32,
    value: f32,
) -> f32 {
    const SNAP_DISTANCE: f32 = 8.0;
    let drag_delta = value - start_value;
    if drag_delta.abs() < 0.001 {
        return value;
    }
    let drag_dir = drag_delta.signum();
    let axis = water_edge_axis(edge);
    let mut best = value;
    let mut best_dist = SNAP_DISTANCE;
    for (idx, plane) in planes.iter().enumerate() {
        if idx == plane_idx {
            continue;
        }
        for candidate in [
            WaterEdge::North,
            WaterEdge::South,
            WaterEdge::East,
            WaterEdge::West,
        ] {
            if water_edge_axis(candidate) != axis {
                continue;
            }
            let candidate_value = water_edge_value(plane, candidate);
            let candidate_delta = candidate_value - start_value;
            if candidate_delta.abs() < 0.001 || candidate_delta.signum() != drag_dir {
                continue;
            }
            let dist = (candidate_value - value).abs();
            if dist < best_dist {
                best_dist = dist;
                best = candidate_value;
            }
        }
    }
    best
}

fn water_edge_drag_snap_value(
    enabled: bool,
    planes: &[WaterPlane],
    plane_idx: usize,
    edge: WaterEdge,
    start_value: f32,
    value: f32,
) -> f32 {
    if enabled {
        snapped_water_edge_value(planes, plane_idx, edge, start_value, value)
    } else {
        value
    }
}

fn set_water_edge_value(plane: &mut WaterPlane, edge: WaterEdge, value: f32) {
    let (mut min_x, mut min_y, mut max_x, mut max_y, z) = water_plane_bounds(plane);
    match edge {
        WaterEdge::North => max_y = value.max(min_y + 0.1),
        WaterEdge::South => min_y = value.min(max_y - 0.1),
        WaterEdge::East => max_x = value.max(min_x + 0.1),
        WaterEdge::West => min_x = value.min(max_x - 0.1),
    }
    set_water_plane_rect(plane, min_x, min_y, max_x, max_y, z);
}

pub(crate) fn start_water_edge_drag(app: &mut AppState, viewport: Rect, mouse: Vec2) -> bool {
    if app.active_tab != AppTab::Water || !viewport.contains(mouse) {
        return false;
    }
    if !is_mouse_button_pressed(MouseButton::Left) {
        return false;
    }
    let Some((plane, edge)) = pick_water_edge(app, viewport, mouse) else {
        return false;
    };
    let Some(start_value) = app
        .water_planes
        .get(plane)
        .map(|plane| water_edge_value(plane, edge))
    else {
        return false;
    };
    let additive = is_key_down(KeyCode::LeftShift)
        || is_key_down(KeyCode::RightShift)
        || is_key_down(KeyCode::LeftControl)
        || is_key_down(KeyCode::RightControl);
    select_water_with_mode(app, plane, additive);
    app.hovered_water = Some(plane);
    app.hovered_water_edge = Some((plane, edge));
    let before = water_history_snapshot(app);
    app.water_edge_drag = Some(WaterEdgeDrag {
        plane,
        edge,
        start_value,
        before,
    });
    app.status_message = format!("Dragging {} water edge", water_edge_label(edge));
    true
}

pub(crate) fn update_water_edge_drag(app: &mut AppState, viewport: Rect, mouse: Vec2) -> bool {
    let Some(drag) = app.water_edge_drag.as_ref() else {
        return false;
    };
    let plane_idx = drag.plane;
    let edge = drag.edge;
    let start_value = drag.start_value;
    let Some(value) = water_drag_value_at_mouse(app, viewport, mouse, plane_idx, edge) else {
        return true;
    };
    let value = water_edge_drag_snap_value(
        app.water_edge_snap_enabled,
        &app.water_planes,
        plane_idx,
        edge,
        start_value,
        value,
    );
    if let Some(plane) = app.water_planes.get_mut(plane_idx) {
        set_water_edge_value(plane, edge, value);
    }
    app.hovered_water = Some(plane_idx);
    app.hovered_water_edge = Some((plane_idx, edge));
    true
}

pub(crate) fn finish_water_edge_drag(app: &mut AppState) {
    let Some(drag) = app.water_edge_drag.take() else {
        return;
    };
    app.status_message = format!("Moved {} water edge", water_edge_label(drag.edge));
    commit_water_history(app, "Move water edge", drag.before);
}

pub(crate) fn handle_water_viewport_click(app: &mut AppState, viewport: Rect, mouse: Vec2) -> bool {
    if app.active_tab != AppTab::Water || !viewport.contains(mouse) {
        return false;
    }
    if !is_mouse_button_pressed(MouseButton::Left) {
        return false;
    }
    let additive = is_key_down(KeyCode::LeftShift)
        || is_key_down(KeyCode::RightShift)
        || is_key_down(KeyCode::LeftControl)
        || is_key_down(KeyCode::RightControl);
    if let Some(idx) = pick_water_plane(app, viewport, mouse) {
        select_water_with_mode(app, idx, additive);
        app.inspector_edit = None;
        app.status_message = format!(
            "Selected {} water plane(s)",
            selected_water_indices(app).len()
        );
    } else if !additive {
        deselect_water(app);
        app.status_message = "Deselected water planes".to_string();
    }
    true
}

pub(crate) fn update_water_hover(app: &mut AppState, mouse: Vec2) {
    if app.active_tab != AppTab::Water {
        app.hovered_water = None;
        return;
    }
    app.hovered_water = water_row_at(app, mouse).filter(|idx| *idx < app.water_planes.len());
}

pub(crate) fn scroll_water_list(app: &mut AppState, wheel: f32) {
    app.water_scroll = (app.water_scroll - wheel * 4.0).max(0.0);
    clamp_water_scroll(app);
}

pub(crate) fn add_water_plane(app: &mut AppState) {
    let before = water_history_snapshot(app);
    let forward = camera_vectors(&app.camera).0;
    let center = app.camera.pos + forward * 256.0;
    app.water_planes.push(new_water_plane(center));
    app.selected_water = app.water_planes.len().saturating_sub(1);
    app.selected_water_planes.clear();
    app.selected_water_planes.insert(app.selected_water);
    app.status_message = format!("Added water plane {}", app.selected_water + 1);
    commit_water_history(app, "Add water plane", before);
}

pub(crate) fn duplicate_water_plane(app: &mut AppState) {
    let indices = selected_water_indices(app);
    if indices.is_empty() {
        app.status_message = "No water plane selected".to_string();
        return;
    };
    let before = water_history_snapshot(app);
    let copies: Vec<WaterPlane> = indices
        .iter()
        .filter_map(|idx| app.water_planes.get(*idx).cloned())
        .collect();
    app.selected_water_planes.clear();
    for plane in copies {
        app.water_planes.push(plane);
        let idx = app.water_planes.len() - 1;
        app.selected_water_planes.insert(idx);
        app.selected_water = idx;
    }
    app.status_message = format!(
        "Duplicated {} water plane(s)",
        app.selected_water_planes.len()
    );
    commit_water_history(app, "Duplicate water plane(s)", before);
}

pub(crate) fn delete_water_plane(app: &mut AppState) {
    let indices = selected_water_indices(app);
    if indices.is_empty() {
        app.status_message = "No water plane selected".to_string();
        return;
    }
    let before = water_history_snapshot(app);
    for idx in indices.iter().rev() {
        app.water_planes.remove(*idx);
    }
    app.selected_water_planes.clear();
    app.selected_water = app
        .water_planes
        .len()
        .checked_sub(1)
        .unwrap_or(NO_SELECTION);
    if app.selected_water < app.water_planes.len() {
        app.selected_water_planes.insert(app.selected_water);
    }
    app.status_message = format!("Deleted {} water plane(s)", indices.len());
    commit_water_history(app, "Delete water plane(s)", before);
}

pub(crate) fn move_selected_water(app: &mut AppState, delta: Vec3) {
    let indices = selected_water_indices(app);
    if indices.is_empty() || delta.length_squared() <= 0.0 {
        return;
    }
    let before = water_history_snapshot(app);
    for idx in indices {
        if let Some(plane) = app.water_planes.get_mut(idx) {
            for corner in &mut plane.corners {
                corner.pos.x += delta.x;
                corner.pos.y += delta.y;
                corner.pos.z += delta.z;
            }
        }
    }
    app.status_message = "Moved selected water plane(s)".to_string();
    commit_water_history(app, "Move water plane(s)", before);
}

pub(crate) fn split_selected_water_plane(app: &mut AppState, axis: WaterSplitAxis) {
    let index = app.selected_water;
    let Some(plane) = app.water_planes.get(index).cloned() else {
        app.status_message = "No water plane selected".to_string();
        return;
    };
    let before = water_history_snapshot(app);
    let (first, second) = split_water_plane(&plane, axis);
    app.water_planes[index] = first;
    app.water_planes.insert(index + 1, second);
    app.selected_water = index + 1;
    app.selected_water_planes.clear();
    app.selected_water_planes.insert(index);
    app.selected_water_planes.insert(index + 1);
    clamp_water_scroll(app);
    let axis_label = match axis {
        WaterSplitAxis::X => "X",
        WaterSplitAxis::Y => "Y",
    };
    app.status_message = format!("Split water plane {} along {axis_label}", index + 1);
    commit_water_history(app, format!("Split water plane ({axis_label})"), before);
}

pub(crate) fn save_water_for_app(app: &mut AppState) {
    match save_water_dat(&water_dat_path(&app.root), &app.water_planes) {
        Ok(()) => {
            mark_water_saved(app);
            app.status_message = format!(
                "Saved {} water plane(s) to water.dat",
                app.water_planes.len()
            );
        }
        Err(err) => app.status_message = format!("Failed to save water.dat: {err}"),
    }
}

pub(crate) fn handle_water_click(app: &mut AppState, mouse: Vec2) -> bool {
    if app.active_tab != AppTab::Water || !is_mouse_button_pressed(MouseButton::Left) {
        return false;
    }
    if !inspector_panel_content_rect().contains(mouse) {
        return false;
    }
    if water_button_rect(app, 0).contains(mouse) {
        add_water_plane(app);
        return true;
    }
    if water_button_rect(app, 1).contains(mouse) {
        duplicate_water_plane(app);
        return true;
    }
    if water_button_rect(app, 2).contains(mouse) {
        delete_water_plane(app);
        return true;
    }
    if water_button_rect(app, 3).contains(mouse) {
        save_water_for_app(app);
        return true;
    }
    if water_edge_snap_toggle_rect(app).contains(mouse) {
        app.water_edge_snap_enabled = !app.water_edge_snap_enabled;
        app.status_message = if app.water_edge_snap_enabled {
            "Water edge snapping enabled".to_string()
        } else {
            "Water edge snapping disabled".to_string()
        };
        return true;
    }
    if water_split_button_rect(app, WaterSplitAxis::X).contains(mouse) {
        split_selected_water_plane(app, WaterSplitAxis::X);
        return true;
    }
    if water_split_button_rect(app, WaterSplitAxis::Y).contains(mouse) {
        split_selected_water_plane(app, WaterSplitAxis::Y);
        return true;
    }
    if let Some(row) = water_row_at(app, mouse) {
        if row < app.water_planes.len() {
            let additive = is_key_down(KeyCode::LeftShift)
                || is_key_down(KeyCode::RightShift)
                || is_key_down(KeyCode::LeftControl)
                || is_key_down(KeyCode::RightControl);
            select_water_with_mode(app, row, additive);
            app.inspector_edit = None;
            app.status_message = format!(
                "Selected {} water plane(s)",
                selected_water_indices(app).len()
            );
            return true;
        }
    }
    false
}

pub(crate) fn draw_water_panel(app: &AppState) {
    let x = screen_width() - right_panel_width() + 12.0;
    let panel_y = TOP_H + 12.0;
    let y = panel_y - app.properties_scroll;
    let w = right_panel_width() - 24.0;
    draw_panel_rect(
        &app.ui_font,
        Rect::new(
            x,
            panel_y,
            w,
            (screen_height() - STATUS_H - panel_y - 12.0).max(1.0),
        ),
        Some("Water"),
    );
    begin_ui_clip(inspector_panel_content_rect());
    text_button(&app.ui_font, water_button_rect(app, 0), "Add", false);
    text_button(&app.ui_font, water_button_rect(app, 1), "Duplicate", false);
    text_button(&app.ui_font, water_button_rect(app, 2), "Delete", false);
    text_button(&app.ui_font, water_button_rect(app, 3), "Write .dat", false);

    let water_dirty = app
        .saved_snapshot
        .as_ref()
        .is_none_or(|saved| saved.water_planes != app.water_planes);
    ui_text(
        &app.ui_font,
        &format!(
            "{} plane(s) from water.dat{}",
            app.water_planes.len(),
            if water_dirty {
                "  •  unsaved changes"
            } else {
                ""
            }
        ),
        x + 14.0,
        y + 86.0,
        if water_dirty { ui_accent() } else { LIGHTGRAY },
    );

    let list = water_list_rect(app);
    draw_rrect_bordered(
        list.x,
        list.y,
        list.w,
        list.h,
        9.0,
        1.0,
        ui_input_bg(),
        ui_border(),
    );
    let start = app.water_scroll.floor() as usize;
    let end = (start + water_visible_rows()).min(app.water_planes.len());
    for (visible_row, idx) in (start..end).enumerate() {
        let Some(plane) = app.water_planes.get(idx) else {
            continue;
        };
        let row_y = list.y + 24.0 + visible_row as f32 * 24.0;
        let row_selected = app.selected_water_planes.contains(&idx) || idx == app.selected_water;
        let row_hovered = app.hovered_water == Some(idx);
        if row_selected || row_hovered {
            draw_rectangle(
                list.x + 4.0,
                row_y - 16.0,
                list.w - 8.0,
                22.0,
                if row_selected {
                    ui_accent_soft()
                } else {
                    ui_surface_hover()
                },
            );
        }
        let (min_x, min_y, max_x, max_y, z) = water_plane_bounds(plane);
        ui_text(
            &app.ui_font,
            &format!(
                "#{:03}  type {}  z {:.1}  {:.0},{:.0} -> {:.0},{:.0}",
                idx + 1,
                plane.kind,
                z,
                min_x,
                min_y,
                max_x,
                max_y
            ),
            x + 18.0,
            row_y,
            if row_selected {
                ui_accent()
            } else if row_hovered {
                Color::new(0.66, 0.82, 0.96, 1.0)
            } else {
                LIGHTGRAY
            },
        );
    }
    if app.water_planes.len() > water_visible_rows() {
        let max_scroll = app.water_planes.len().saturating_sub(water_visible_rows()) as f32;
        let track = Rect::new(list.x + list.w - 8.0, list.y + 4.0, 4.0, list.h - 8.0);
        if let Some(metrics) = scrollbar_metrics(
            track,
            water_visible_rows() as f32,
            app.water_planes.len() as f32,
            28.0,
            app.water_scroll.min(max_scroll),
        ) {
            draw_scrollbar(metrics, scrollbar_visual_state(track, false));
        }
    }

    ui_text_size(
        &app.ui_font,
        &format!("Selected Plane ({})", selected_water_indices(app).len()),
        x + 14.0,
        y + 448.0,
        18,
        WHITE,
    );
    text_button(
        &app.ui_font,
        water_edge_snap_toggle_rect(app),
        "Edge Snap",
        app.water_edge_snap_enabled,
    );
    if selected_water_plane(app).is_some() {
        ui_text(
            &app.ui_font,
            "Split selected plane",
            x + 14.0,
            y + 498.0,
            ui_dim(),
        );
        text_button(
            &app.ui_font,
            water_split_button_rect(app, WaterSplitAxis::X),
            "Split X",
            false,
        );
        text_button(
            &app.ui_font,
            water_split_button_rect(app, WaterSplitAxis::Y),
            "Split Y",
            false,
        );
        draw_input_box(app, InspectorField::WaterMinX, "Min X");
        draw_input_box(app, InspectorField::WaterMinY, "Min Y");
        draw_input_box(app, InspectorField::WaterMaxX, "Max X");
        draw_input_box(app, InspectorField::WaterMaxY, "Max Y");
        draw_input_box(app, InspectorField::WaterHeight, "Height");
        draw_input_box(app, InspectorField::WaterType, "Type");
    } else {
        ui_text(
            &app.ui_font,
            "No water plane selected",
            x + 14.0,
            y + 486.0,
            ui_dim(),
        );
    }
    end_ui_clip();
    draw_inspector_scrollbar(app, water_panel_scroll_max());
}

pub(crate) fn water_panel_scroll_max() -> f32 {
    panel_scroll_max_for_bottom(TOP_H + 794.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adjacent_planes() -> Vec<WaterPlane> {
        vec![
            new_water_plane(Vec3::ZERO),
            new_water_plane(vec3(164.0, 0.0, 0.0)),
        ]
    }

    #[test]
    fn edge_snap_aligns_to_a_nearby_parallel_edge_when_enabled() {
        let planes = adjacent_planes();

        let value = water_edge_drag_snap_value(true, &planes, 0, WaterEdge::East, 64.0, 95.0);

        assert_eq!(value, 100.0);
    }

    #[test]
    fn edge_snap_toggle_allows_free_edge_dragging() {
        let planes = adjacent_planes();

        let value = water_edge_drag_snap_value(false, &planes, 0, WaterEdge::East, 64.0, 95.0);

        assert_eq!(value, 95.0);
    }
}
