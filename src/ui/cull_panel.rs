use super::super::*;

fn button_rect(app: &AppState, slot: usize) -> Rect {
    Rect::new(
        screen_width() - RIGHT_PANEL_W + 18.0 + slot as f32 * 95.0,
        TOP_H + 54.0 - app.properties_scroll,
        89.0,
        28.0,
    )
}

pub(crate) fn cull_list_rect(app: &AppState) -> Rect {
    Rect::new(
        screen_width() - RIGHT_PANEL_W + 22.0,
        TOP_H + 116.0 - app.properties_scroll,
        RIGHT_PANEL_W - 44.0,
        304.0,
    )
}

pub(crate) fn selected_cull_zone(app: &AppState) -> Option<&CullZone> {
    app.cull_zones.get(app.selected_cull)
}

pub(crate) fn selected_cull_zone_mut(app: &mut AppState) -> Option<&mut CullZone> {
    app.cull_zones.get_mut(app.selected_cull)
}

pub(crate) fn add_cull_zone(app: &mut AppState) {
    let before = cull_history_snapshot(app);
    let forward = camera_vectors(&app.camera).0;
    let center = app.camera.pos + forward * 128.0;
    let number = app.cull_zones.len() + 1;
    app.cull_zones.push(CullZone {
        id: format!("water_cull_{number:03}"),
        center: V3 {
            x: center.x,
            y: center.y,
            z: center.z,
        },
        size: V3 {
            x: 64.0,
            y: 64.0,
            z: 32.0,
        },
    });
    app.selected_cull = app.cull_zones.len() - 1;
    app.cull_scroll = app.cull_zones.len().saturating_sub(12) as f32;
    app.status_message = format!("Added water cull zone {number}");
    commit_cull_history(app, "Add water cull zone", before);
}

pub(crate) fn duplicate_cull_zone(app: &mut AppState) {
    let before = cull_history_snapshot(app);
    let Some(mut zone) = selected_cull_zone(app).cloned() else {
        return;
    };
    zone.id = format!("water_cull_{:03}", app.cull_zones.len() + 1);
    zone.center.x += 8.0;
    zone.center.y += 8.0;
    app.cull_zones.push(zone);
    app.selected_cull = app.cull_zones.len() - 1;
    app.cull_scroll = app.cull_zones.len().saturating_sub(12) as f32;
    app.status_message = "Duplicated water cull zone".to_string();
    commit_cull_history(app, "Duplicate water cull zone", before);
}

pub(crate) fn delete_cull_zone(app: &mut AppState) {
    if app.selected_cull >= app.cull_zones.len() {
        return;
    }
    let before = cull_history_snapshot(app);
    app.cull_zones.remove(app.selected_cull);
    app.selected_cull = app
        .selected_cull
        .min(app.cull_zones.len().saturating_sub(1));
    clamp_cull_scroll(app);
    app.status_message = "Deleted water cull zone".to_string();
    commit_cull_history(app, "Delete water cull zone", before);
}

pub(crate) fn save_cull_for_app(app: &mut AppState) {
    match save_cull_system(&app.root, &app.cull_zones, true) {
        Ok(()) => {
            if let Some(saved) = app.saved_snapshot.as_mut() {
                saved.cull_zones = app.cull_zones.clone();
            }
            app.status_message = format!(
                "Saved {} water cull zone(s) to cull.map",
                app.cull_zones.len()
            );
        }
        Err(err) => app.status_message = format!("Failed to save cull system: {err}"),
    }
}

pub(crate) fn move_selected_cull(app: &mut AppState, delta: Vec3) {
    let before = cull_history_snapshot(app);
    if let Some(zone) = selected_cull_zone_mut(app) {
        zone.center.x += delta.x;
        zone.center.y += delta.y;
        zone.center.z += delta.z;
    }
    commit_cull_history(app, "Move water cull zone", before);
}

pub(crate) fn pick_selected_cull_face(
    app: &AppState,
    viewport: Rect,
    mouse: Vec2,
) -> Option<CullFacePick> {
    let zone = selected_cull_zone(app)?;
    let (ray_origin, ray_dir) = viewport_ray(app, viewport, mouse)?;
    let center = vec3(zone.center.x, zone.center.y, zone.center.z);
    let half_extents = vec3(zone.size.x, zone.size.y, zone.size.z) * 0.5;
    let local_origin = ray_origin - center;
    let mut best: Option<(f32, CullFacePick)> = None;

    for axis in 0..3 {
        for side_is_max in [false, true] {
            let side = if side_is_max { 1.0 } else { -1.0 };
            let denom = ray_dir[axis];
            if denom.abs() < 1.0e-6 {
                continue;
            }
            let t = (side * half_extents[axis] - local_origin[axis]) / denom;
            if t <= 0.0 {
                continue;
            }
            let hit = local_origin + ray_dir * t;
            let inside = (0..3).filter(|other| *other != axis).all(|other| {
                let pad = half_extents[other] * 0.02 + 0.05;
                hit[other] >= -half_extents[other] - pad && hit[other] <= half_extents[other] + pad
            });
            if !inside || best.as_ref().is_some_and(|(best_t, _)| t >= *best_t) {
                continue;
            }
            let mut axis_dir = Vec3::ZERO;
            axis_dir[axis] = 1.0;
            best = Some((
                t,
                CullFacePick {
                    axis,
                    side_is_max,
                    center,
                    half_extents,
                    axis_dir,
                    face_origin: center + axis_dir * (side * half_extents[axis]),
                },
            ));
        }
    }
    best.map(|(_, pick)| pick)
}

pub(crate) fn start_cull_face_drag(app: &mut AppState, viewport: Rect, mouse: Vec2) -> bool {
    if app.active_tab != AppTab::Cull
        || !viewport.contains(mouse)
        || !is_mouse_button_pressed(MouseButton::Left)
    {
        return false;
    }
    // The translation gimbal owns clicks on its handles even though the axes
    // visually overlap the selected box faces.
    if app.transform_mode == TransformMode::Move && gizmo_axis_at(app, viewport, mouse).is_some() {
        return false;
    }
    let Some(pick) = pick_selected_cull_face(app, viewport, mouse) else {
        return false;
    };
    app.cull_face_drag = Some(CullFaceDrag {
        zone: app.selected_cull,
        axis: pick.axis,
        side_is_max: pick.side_is_max,
        center: pick.center,
        half_extents: pick.half_extents,
        axis_dir: pick.axis_dir,
        face_origin: pick.face_origin,
        start_mouse: mouse,
        before: cull_history_snapshot(app),
    });
    app.camera.looking = false;
    set_cursor_grab(false);
    show_mouse(true);
    true
}

pub(crate) fn apply_cull_face_drag(app: &mut AppState, viewport: Rect, mouse: Vec2) {
    let Some(drag) = app.cull_face_drag.as_ref() else {
        return;
    };
    let zone_index = drag.zone;
    let axis = drag.axis;
    let side_is_max = drag.side_is_max;
    let center = drag.center;
    let mut half_extents = drag.half_extents;
    let axis_dir = drag.axis_dir;
    let mut amount = axis_drag_amount(
        app,
        viewport,
        drag.face_origin,
        axis_dir,
        drag.start_mouse,
        mouse,
    );
    if app.snap_enabled {
        amount = snap_delta(amount, app.snap_move);
    }
    const MIN_SIZE: f32 = 0.1;
    if side_is_max {
        amount = amount.max(-half_extents[axis] * 2.0 + MIN_SIZE);
    } else {
        amount = amount.min(half_extents[axis] * 2.0 - MIN_SIZE);
    }
    let side_sign = if side_is_max { 1.0 } else { -1.0 };
    half_extents[axis] = (half_extents[axis] + side_sign * amount * 0.5).max(MIN_SIZE * 0.5);
    let center = center + axis_dir * (amount * 0.5);
    if let Some(zone) = app.cull_zones.get_mut(zone_index) {
        zone.center = from_mq(center);
        zone.size = from_mq(half_extents * 2.0);
    }
}

fn row_at(app: &AppState, mouse: Vec2) -> Option<usize> {
    let rect = cull_list_rect(app);
    if !rect.contains(mouse) {
        return None;
    }
    let row = ((mouse.y - rect.y - 12.0) / 24.0).floor().max(0.0) as usize;
    let row = app.cull_scroll.floor() as usize + row;
    (row < app.cull_zones.len()).then_some(row)
}

pub(crate) fn clamp_cull_scroll(app: &mut AppState) {
    app.cull_scroll = app
        .cull_scroll
        .clamp(0.0, app.cull_zones.len().saturating_sub(12) as f32);
}

pub(crate) fn scroll_cull_list(app: &mut AppState, wheel: f32) {
    app.cull_scroll = (app.cull_scroll - wheel * 4.0).max(0.0);
    clamp_cull_scroll(app);
}

pub(crate) fn handle_cull_click(app: &mut AppState, mouse: Vec2) -> bool {
    if app.active_tab != AppTab::Cull || !is_mouse_button_pressed(MouseButton::Left) {
        return false;
    }
    if button_rect(app, 0).contains(mouse) {
        add_cull_zone(app);
        return true;
    }
    if button_rect(app, 1).contains(mouse) {
        duplicate_cull_zone(app);
        return true;
    }
    if button_rect(app, 2).contains(mouse) {
        delete_cull_zone(app);
        return true;
    }
    if button_rect(app, 3).contains(mouse) {
        save_cull_for_app(app);
        return true;
    }
    if let Some(row) = row_at(app, mouse) {
        app.selected_cull = row;
        app.inspector_edit = None;
        return true;
    }
    false
}

pub(crate) fn handle_cull_viewport_click(app: &mut AppState, viewport: Rect, mouse: Vec2) -> bool {
    if app.active_tab != AppTab::Cull
        || !viewport.contains(mouse)
        || !is_mouse_button_pressed(MouseButton::Left)
    {
        return false;
    }
    if app.transform_mode == TransformMode::Move && gizmo_axis_at(app, viewport, mouse).is_some() {
        return false;
    }
    let Some((origin, dir)) = viewport_ray(app, viewport, mouse) else {
        return true;
    };
    let mut best = None;
    let mut best_t = f32::MAX;
    for (idx, zone) in app.cull_zones.iter().enumerate() {
        let half = Vec3::new(zone.size.x, zone.size.y, zone.size.z) * 0.5;
        let center = Vec3::new(zone.center.x, zone.center.y, zone.center.z);
        if let Some(t) = ray_aabb(origin, dir, center - half, center + half)
            && t < best_t
        {
            best = Some(idx);
            best_t = t;
        }
    }
    if let Some(idx) = best {
        app.selected_cull = idx;
        app.status_message = format!("Selected water cull zone {}", idx + 1);
    }
    true
}

pub(crate) fn draw_cull_panel(app: &AppState) {
    let x = screen_width() - RIGHT_PANEL_W + 12.0;
    let panel_y = TOP_H + 12.0;
    let y = panel_y - app.properties_scroll;
    draw_panel_rect(
        &app.ui_font,
        Rect::new(
            x,
            panel_y,
            RIGHT_PANEL_W - 24.0,
            (screen_height() - STATUS_H - panel_y - 12.0).max(1.0),
        ),
        Some("CULL — Water Hide Zones"),
    );
    begin_ui_clip(inspector_panel_content_rect());
    text_button(&app.ui_font, button_rect(app, 0), "Add", false);
    text_button(&app.ui_font, button_rect(app, 1), "Duplicate", false);
    text_button(&app.ui_font, button_rect(app, 2), "Delete", false);
    text_button(&app.ui_font, button_rect(app, 3), "Write .map", false);
    let dirty = app
        .saved_snapshot
        .as_ref()
        .is_none_or(|saved| saved.cull_zones != app.cull_zones);
    ui_text(
        &app.ui_font,
        &format!(
            "{} water hide zone(s){}",
            app.cull_zones.len(),
            if dirty { "  •  unsaved changes" } else { "" }
        ),
        x + 14.0,
        y + 86.0,
        if dirty { ui_accent() } else { LIGHTGRAY },
    );
    let list = cull_list_rect(app);
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
    let start = app.cull_scroll.floor() as usize;
    for (visible, (idx, zone)) in app
        .cull_zones
        .iter()
        .enumerate()
        .skip(start)
        .take(12)
        .enumerate()
    {
        let row_y = list.y + 24.0 + visible as f32 * 24.0;
        if app.selected_cull == idx {
            draw_rectangle(
                list.x + 4.0,
                row_y - 16.0,
                list.w - 8.0,
                22.0,
                ui_accent_soft(),
            );
        }
        ui_text(
            &app.ui_font,
            &format!(
                "#{:03}  {}  {:.0} × {:.0} × {:.0}",
                idx + 1,
                zone.id,
                zone.size.x,
                zone.size.y,
                zone.size.z
            ),
            x + 18.0,
            row_y,
            if app.selected_cull == idx {
                ui_accent()
            } else {
                LIGHTGRAY
            },
        );
    }
    if app.cull_zones.len() > 12 {
        let track = Rect::new(list.x + list.w - 8.0, list.y + 4.0, 4.0, list.h - 8.0);
        if let Some(metrics) = scrollbar_metrics(
            track,
            12.0,
            app.cull_zones.len() as f32,
            28.0,
            app.cull_scroll,
        ) {
            draw_scrollbar(metrics, scrollbar_visual_state(track, false));
        }
    }
    ui_text_size(
        &app.ui_font,
        "Selected Zone",
        x + 14.0,
        y + 448.0,
        18,
        WHITE,
    );
    if selected_cull_zone(app).is_some() {
        draw_input_box(app, InspectorField::CullPosX, "Center X");
        draw_input_box(app, InspectorField::CullPosY, "Center Y");
        draw_input_box(app, InspectorField::CullPosZ, "Center Z");
        draw_input_box(app, InspectorField::CullSizeX, "Size X");
        draw_input_box(app, InspectorField::CullSizeY, "Size Y");
        draw_input_box(app, InspectorField::CullSizeZ, "Size Z");
        ui_text(
            &app.ui_font,
            "Inside this volume, all water elements move to dimension 65535.",
            x + 14.0,
            y + 720.0,
            ui_dim(),
        );
    } else {
        ui_text(
            &app.ui_font,
            "No cull zone selected",
            x + 14.0,
            y + 486.0,
            ui_dim(),
        );
    }
    end_ui_clip();
    draw_inspector_scrollbar(app, cull_panel_scroll_max());
}

pub(crate) fn cull_panel_scroll_max() -> f32 {
    panel_scroll_max_for_bottom(TOP_H + 760.0)
}

pub(crate) fn draw_cull_zones(app: &AppState) {
    if app.active_tab != AppTab::Cull {
        return;
    }
    unsafe {
        gl::PushAttrib(gl::ENABLE_BIT | gl::CURRENT_BIT | gl::LINE_BIT | gl::DEPTH_BUFFER_BIT);
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        for (idx, zone) in app.cull_zones.iter().enumerate() {
            let h = V3 {
                x: zone.size.x * 0.5,
                y: zone.size.y * 0.5,
                z: zone.size.z * 0.5,
            };
            let min = V3 {
                x: zone.center.x - h.x,
                y: zone.center.y - h.y,
                z: zone.center.z - h.z,
            };
            let max = V3 {
                x: zone.center.x + h.x,
                y: zone.center.y + h.y,
                z: zone.center.z + h.z,
            };
            gl::LineWidth(if idx == app.selected_cull { 3.0 } else { 1.4 });
            if idx == app.selected_cull {
                gl::Color4f(0.2, 0.9, 1.0, 0.95)
            } else {
                gl::Color4f(0.1, 0.65, 0.9, 0.55)
            }
            let p = [
                [min.x, min.y, min.z],
                [max.x, min.y, min.z],
                [max.x, max.y, min.z],
                [min.x, max.y, min.z],
                [min.x, min.y, max.z],
                [max.x, min.y, max.z],
                [max.x, max.y, max.z],
                [min.x, max.y, max.z],
            ];
            let edges = [
                (0, 1),
                (1, 2),
                (2, 3),
                (3, 0),
                (4, 5),
                (5, 6),
                (6, 7),
                (7, 4),
                (0, 4),
                (1, 5),
                (2, 6),
                (3, 7),
            ];
            gl::Begin(gl::LINES);
            for (a, b) in edges {
                gl::Vertex3f(p[a][0], p[a][1], p[a][2]);
                gl::Vertex3f(p[b][0], p[b][1], p[b][2]);
            }
            gl::End();

            if idx == app.selected_cull {
                let face = app
                    .cull_face_drag
                    .as_ref()
                    .filter(|drag| drag.zone == idx)
                    .map(|drag| (drag.axis, drag.side_is_max, true))
                    .or_else(|| {
                        app.cull_hovered_face
                            .map(|(axis, side_is_max)| (axis, side_is_max, false))
                    });
                if let Some((axis, side_is_max, active)) = face {
                    draw_cull_face_highlight(
                        vec3(zone.center.x, zone.center.y, zone.center.z),
                        vec3(zone.size.x, zone.size.y, zone.size.z) * 0.5,
                        axis,
                        side_is_max,
                        active,
                    );
                }
            }
        }
        gl::PopAttrib();
    }
}

fn draw_cull_face_highlight(
    center: Vec3,
    half_extents: Vec3,
    axis: usize,
    side_is_max: bool,
    active: bool,
) {
    let coord = if side_is_max {
        half_extents[axis]
    } else {
        -half_extents[axis]
    };
    let (u, v) = match axis {
        0 => (1usize, 2usize),
        1 => (0usize, 2usize),
        _ => (0usize, 1usize),
    };
    let corner = |su: bool, sv: bool| {
        let mut point = center;
        point[axis] += coord;
        point[u] += if su {
            half_extents[u]
        } else {
            -half_extents[u]
        };
        point[v] += if sv {
            half_extents[v]
        } else {
            -half_extents[v]
        };
        point
    };
    let corners = [
        corner(false, false),
        corner(true, false),
        corner(true, true),
        corner(false, true),
    ];
    unsafe {
        gl::Disable(gl::CULL_FACE);
        gl::Enable(gl::DEPTH_TEST);
        gl::DepthMask(gl::FALSE);
        gl::Color4f(1.0, 0.72, 0.10, if active { 0.46 } else { 0.24 });
        gl::Begin(gl::TRIANGLES);
        for point in [
            corners[0], corners[1], corners[2], corners[0], corners[2], corners[3],
        ] {
            gl::Vertex3f(point.x, point.y, point.z);
        }
        gl::End();
        gl::DepthMask(gl::TRUE);
        gl::LineWidth(if active { 4.0 } else { 3.0 });
        gl::Color4f(1.0, 0.86, 0.15, 0.96);
        gl::Begin(gl::LINE_LOOP);
        for point in corners {
            gl::Vertex3f(point.x, point.y, point.z);
        }
        gl::End();
    }
}
