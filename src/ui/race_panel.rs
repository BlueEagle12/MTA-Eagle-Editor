use super::super::*;
use std::sync::mpsc;
use std::thread;

// ============================================================================
// Race Editor tab.
//
// Layout:
//   * LEFT panel  ("Races")  — the list of tracks for this map + New/Delete/Save.
//   * RIGHT panel ("Track")  — per-track settings, placement modes, point tools,
//                              radar backdrop + preview generation, point list.
//
// Lets the user define, per map:
//   * the race start point,
//   * gameplay checkpoints (the actual race route),
//   * a separate dense overlay path (preview + live radar drawing only),
//   * a full track path,
//   * and generate a cropped preview image + transparent overlay image from a
//     radar backdrop.
//
// Gameplay checkpoints and overlay points are stored SEPARATELY so the overlay
// can be much denser without affecting the race route/checkpoint logic.
//
// The Preview tab can also show a radar minimap overlay (toggle) with a marker
// at the current camera position, so you can see where on the map you are.
// ============================================================================

const RACE_LEFT_X: f32 = 10.0;
const PREVIEW_MINIMAP_SIZE: f32 = 260.0;
const PREVIEW_MINIMAP_DOUBLE_CLICK_SECONDS: f64 = 0.4;
const PREVIEW_MINIMAP_DOUBLE_CLICK_DISTANCE: f32 = 8.0;

fn race_left_w() -> f32 {
    PANEL_W - 20.0
}

fn race_panel_x() -> f32 {
    screen_width() - RIGHT_PANEL_W + 12.0
}

/// Left-panel ("Races") button rect.
fn race_left_btn(row: usize, col: usize, cols: usize) -> Rect {
    let px = RACE_LEFT_X + 12.0;
    let total = race_left_w() - 24.0;
    let gap = 6.0;
    let w = (total - gap * (cols as f32 - 1.0)) / cols as f32;
    let x = px + col as f32 * (w + gap);
    let y = TOP_H + 54.0 + row as f32 * 34.0;
    Rect::new(x, y, w, 28.0)
}

fn race_track_list_rect() -> Rect {
    let px = RACE_LEFT_X + 12.0;
    let y = TOP_H + 54.0 + 34.0 + 10.0;
    let h = (screen_height() - STATUS_H - y - 12.0).max(120.0);
    Rect::new(px, y, race_left_w() - 24.0, h)
}

fn race_track_visible_rows() -> usize {
    ((race_track_list_rect().h - 12.0) / 26.0).floor().max(1.0) as usize
}

/// Right-panel grid button rect. `row` counts from the top button row.
fn race_grid_rect(row: usize, col: usize, cols: usize) -> Rect {
    let px = screen_width() - RIGHT_PANEL_W + 22.0;
    let total = RIGHT_PANEL_W - 44.0;
    let gap = 6.0;
    let w = (total - gap * (cols as f32 - 1.0)) / cols as f32;
    let x = px + col as f32 * (w + gap);
    let y = TOP_H + 54.0 + row as f32 * 34.0;
    Rect::new(x, y, w, 28.0)
}

/// Y of the info-text block under the button grid (rows 0-8 used).
fn race_info_y() -> f32 {
    TOP_H + 54.0 + 9.0 * 34.0 + 8.0
}

fn race_list_rect() -> Rect {
    let px = screen_width() - RIGHT_PANEL_W + 22.0;
    // Below the button grid (9 rows) + 4 info lines.
    let y = race_info_y() + 4.0 * 18.0 + 10.0;
    let h = (screen_height() - STATUS_H - y - 12.0).max(120.0);
    Rect::new(px, y, RIGHT_PANEL_W - 44.0, h)
}

fn race_visible_rows() -> usize {
    ((race_list_rect().h - 24.0) / 22.0).floor().max(1.0) as usize
}

/// Small toggle button pinned to the top-left of the viewport in the Preview
/// tab to switch the radar minimap overlay on/off.
fn race_overlay_toggle_rect() -> Rect {
    Rect::new(PANEL_W + 16.0, TOP_H + 12.0, 150.0, 26.0)
}

fn race_preview_minimap_rect() -> Rect {
    let size = PREVIEW_MINIMAP_SIZE;
    Rect::new(
        screen_width() - RIGHT_PANEL_W - size - 20.0,
        screen_height() - STATUS_H - size - 20.0,
        size,
        size,
    )
}

fn race_preview_minimap_screen_to_world(
    rect: Rect,
    mouse: Vec2,
    world_size: f32,
    center_x: f32,
    center_y: f32,
) -> Vec2 {
    let u = ((mouse.x - rect.x) / rect.w).clamp(0.0, 1.0);
    let v = ((mouse.y - rect.y) / rect.h).clamp(0.0, 1.0);
    let left = center_x - world_size * 0.5;
    let top = center_y + world_size * 0.5;
    vec2(left + u * world_size, top - v * world_size)
}

fn mode_label(mode: RacePlaceMode) -> &'static str {
    match mode {
        RacePlaceMode::None => "None",
        RacePlaceMode::Start => "Start",
        RacePlaceMode::Checkpoint => "Checkpoint",
        RacePlaceMode::Overlay => "Overlay",
        RacePlaceMode::Path => "Path",
    }
}

fn active_len(track: &RaceTrack, mode: RacePlaceMode) -> usize {
    match mode {
        RacePlaceMode::Checkpoint => track.checkpoints.len(),
        RacePlaceMode::Overlay => track.overlay.len(),
        RacePlaceMode::Path => track.path.len(),
        RacePlaceMode::Start => 1,
        RacePlaceMode::None => 0,
    }
}

// ---------------------------------------------------------------------------
// Lazy load of existing per-map tracks + settings (called from input dispatch).
// ---------------------------------------------------------------------------

pub(crate) fn ensure_race_loaded(app: &mut AppState) {
    if app.race.loaded {
        return;
    }
    app.race.loaded = true;
    let tracks = load_race_tracks(&app.root);
    if !tracks.is_empty() {
        app.race.tracks = tracks;
        app.race.selected_track = 0;
        if let Some(t) = app.race.tracks.first() {
            app.race.place_z = t.start.z;
        }
    }
    if let Some(s) = load_race_settings(&app.root) {
        if !s.radar.trim().is_empty() {
            app.race.radar_path = s.radar;
        }
        app.race.world_size = s.world_size;
        app.race.world_center_x = s.center_x;
        app.race.world_center_y = s.center_y;
    }
    mark_race_saved(app);
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

fn new_track(app: &mut AppState) {
    let before = race_history_snapshot(app);
    let idx = app.race.tracks.len();
    app.race.tracks.push(RaceTrack::new(idx));
    app.race.selected_track = idx;
    app.race.place_mode = RacePlaceMode::Start;
    app.race.selected_point = None;
    app.status_message = format!("Added race track {}", idx + 1);
    commit_race_history(app, "Add Race Track", before);
}

fn delete_track(app: &mut AppState) {
    let idx = app.race.selected_track;
    if idx >= app.race.tracks.len() {
        app.status_message = "No track selected".to_string();
        return;
    }
    let before = race_history_snapshot(app);
    app.race.tracks.remove(idx);
    app.race.selected_track = if app.race.tracks.is_empty() {
        NO_SELECTION
    } else {
        idx.min(app.race.tracks.len() - 1)
    };
    app.race.selected_point = None;
    app.status_message = "Deleted race track".to_string();
    commit_race_history(app, "Delete Race Track", before);
}

fn select_track(app: &mut AppState, idx: usize) {
    if idx >= app.race.tracks.len() {
        return;
    }
    app.race.selected_track = idx;
    app.race.selected_point = None;
    if let Some(t) = app.race.tracks.get(idx) {
        app.race.place_z = t.start.z;
        app.status_message = format!("Selected {}", t.name);
    }
}

// ---------------------------------------------------------------------------
// Subtracks
// ---------------------------------------------------------------------------

/// Short "N/M: Name" descriptor for the selected track's active subtrack.
fn subtrack_summary(track: &RaceTrack) -> String {
    let n = track.subtracks.len().max(1);
    let cur = track.active_subtrack.min(n - 1) + 1;
    let name = if track.subtrack_name.is_empty() {
        "(unnamed)"
    } else {
        &track.subtrack_name
    };
    format!("Subtrack {cur}/{n}: {name}")
}

fn add_subtrack(app: &mut AppState) {
    let idx = app.race.selected_track;
    let before = race_history_snapshot(app);
    let Some(track) = app.race.tracks.get_mut(idx) else {
        app.status_message = "No track selected".to_string();
        return;
    };
    track.commit_active_subtrack();
    let n = track.subtracks.len();
    track.subtracks.push(RaceSubtrack::new(n));
    track.load_subtrack(n);
    app.race.selected_point = None;
    app.race.place_mode = RacePlaceMode::Checkpoint;
    app.status_message = format!("Added subtrack {}", n + 1);
    commit_race_history(app, "Add Subtrack", before);
}

fn delete_subtrack(app: &mut AppState) {
    let idx = app.race.selected_track;
    let before = race_history_snapshot(app);
    let Some(track) = app.race.tracks.get_mut(idx) else {
        return;
    };
    if track.subtracks.len() <= 1 {
        app.status_message = "A track needs at least one subtrack".to_string();
        return;
    }
    let cur = track.active_subtrack.min(track.subtracks.len() - 1);
    track.subtracks.remove(cur);
    let new = cur.min(track.subtracks.len() - 1);
    track.load_subtrack(new);
    app.race.selected_point = None;
    app.status_message = "Deleted subtrack".to_string();
    commit_race_history(app, "Delete Subtrack", before);
}

fn cycle_subtrack(app: &mut AppState, delta: isize) {
    let idx = app.race.selected_track;
    let before = race_history_snapshot(app);
    let Some(track) = app.race.tracks.get_mut(idx) else {
        return;
    };
    track.commit_active_subtrack();
    let n = track.subtracks.len();
    if n <= 1 {
        app.status_message = "Only one subtrack".to_string();
        return;
    }
    let next = (track.active_subtrack as isize + delta).rem_euclid(n as isize) as usize;
    track.load_subtrack(next);
    app.race.selected_point = None;
    if let Some(t) = app.race.tracks.get(idx) {
        app.status_message = format!("Subtrack {}/{}: {}", next + 1, n, t.subtrack_name);
    }
    commit_race_history(app, "Switch Subtrack", before);
}

// ---------------------------------------------------------------------------
// Inline track / subtrack rename
// ---------------------------------------------------------------------------

fn start_race_name_edit(app: &mut AppState, target: RaceNameTarget) {
    let idx = app.race.selected_track;
    let Some(track) = app.race.tracks.get(idx) else {
        app.status_message = "No track selected".to_string();
        return;
    };
    let current = match target {
        RaceNameTarget::Track => track.name.clone(),
        RaceNameTarget::Subtrack => track.subtrack_name.clone(),
    };
    app.race_name_edit = Some(RaceNameEdit {
        target,
        cursor: current.len(),
        buffer: current,
        selection_anchor: None,
        before: race_history_snapshot(app),
    });
    drain_text_input();
    app.status_message = match target {
        RaceNameTarget::Track => "Renaming track — Enter confirms, Esc cancels".to_string(),
        RaceNameTarget::Subtrack => "Renaming subtrack — Enter confirms, Esc cancels".to_string(),
    };
}

pub(crate) fn cancel_race_name_edit(app: &mut AppState) {
    if app.race_name_edit.take().is_some() {
        app.status_message = "Cancelled rename".to_string();
    }
}

pub(crate) fn apply_race_name_edit(app: &mut AppState) {
    let Some(edit) = app.race_name_edit.take() else {
        return;
    };
    let new_name = edit.buffer.trim().to_string();
    if new_name.is_empty() {
        app.status_message = "Name cannot be empty".to_string();
        return;
    }
    let idx = app.race.selected_track;
    let Some(track) = app.race.tracks.get_mut(idx) else {
        return;
    };
    match edit.target {
        RaceNameTarget::Track => track.name = new_name.clone(),
        RaceNameTarget::Subtrack => {
            track.subtrack_name = new_name.clone();
            track.commit_active_subtrack();
        }
    }
    commit_race_history(app, "Rename Race Name", edit.before);
    app.status_message = format!("Renamed to {new_name}");
}

fn set_mode(app: &mut AppState, mode: RacePlaceMode) {
    app.race.place_mode = if app.race.place_mode == mode {
        RacePlaceMode::None
    } else {
        mode
    };
    app.race.selected_point = None;
    app.status_message = format!("Placement mode: {}", mode_label(app.race.place_mode));
}

fn delete_selected_point(app: &mut AppState) {
    let mode = app.race.place_mode;
    let Some(pt) = app.race.selected_point else {
        app.status_message = "No point selected".to_string();
        return;
    };
    let idx = app.race.selected_track;
    let before = race_history_snapshot(app);
    let Some(track) = app.race.tracks.get_mut(idx) else {
        return;
    };
    match mode {
        RacePlaceMode::Checkpoint if pt < track.checkpoints.len() => {
            track.checkpoints.remove(pt);
        }
        RacePlaceMode::Overlay if pt < track.overlay.len() => {
            track.overlay.remove(pt);
        }
        RacePlaceMode::Path if pt < track.path.len() => {
            track.path.remove(pt);
        }
        _ => return,
    }
    app.race.selected_point = None;
    app.status_message = "Deleted point".to_string();
    commit_race_history(app, "Delete Race Point", before);
}

fn clear_active(app: &mut AppState) {
    let mode = app.race.place_mode;
    let idx = app.race.selected_track;
    let before = race_history_snapshot(app);
    let Some(track) = app.race.tracks.get_mut(idx) else {
        return;
    };
    match mode {
        RacePlaceMode::Checkpoint => track.checkpoints.clear(),
        RacePlaceMode::Overlay => track.overlay.clear(),
        RacePlaceMode::Path => track.path.clear(),
        _ => {}
    }
    app.race.selected_point = None;
    app.status_message = format!("Cleared {} points", mode_label(mode));
    commit_race_history(app, format!("Clear Race {}", mode_label(mode)), before);
}

fn nudge_point_z(app: &mut AppState, delta: f32) {
    let mode = app.race.place_mode;
    let pt = app.race.selected_point;
    let idx = app.race.selected_track;
    let before = race_history_snapshot(app);
    let Some(track) = app.race.tracks.get_mut(idx) else {
        return;
    };
    match (mode, pt) {
        (RacePlaceMode::Start, _) => track.start.z += delta,
        (RacePlaceMode::Checkpoint, Some(i)) => {
            if let Some(c) = track.checkpoints.get_mut(i) {
                c.pos.z += delta;
            }
        }
        (RacePlaceMode::Overlay, Some(i)) => {
            if let Some(p) = track.overlay.get_mut(i) {
                p.z += delta;
            }
        }
        (RacePlaceMode::Path, Some(i)) => {
            if let Some(p) = track.path.get_mut(i) {
                p.z += delta;
            }
        }
        _ => {
            app.race.place_z += delta;
            app.status_message = format!("Place Z = {:.1}", app.race.place_z);
            commit_race_history(app, "Adjust Race Place Z", before);
            return;
        }
    }
    app.status_message = format!("Nudged Z by {:+.1}", delta);
    commit_race_history(app, "Nudge Race Point Z", before);
}

fn move_index<T>(items: &mut [T], index: usize, delta: isize) -> Option<usize> {
    let new_index = index.checked_add_signed(delta)?;
    if index >= items.len() || new_index >= items.len() || index == new_index {
        return None;
    }
    items.swap(index, new_index);
    Some(new_index)
}

fn move_selected_point_order(app: &mut AppState, delta: isize) {
    let mode = app.race.place_mode;
    let Some(pt) = app.race.selected_point else {
        app.status_message = "No point selected".to_string();
        return;
    };
    let idx = app.race.selected_track;
    let before = race_history_snapshot(app);
    let Some(track) = app.race.tracks.get_mut(idx) else {
        return;
    };
    let moved = match mode {
        RacePlaceMode::Checkpoint => move_index(&mut track.checkpoints, pt, delta),
        RacePlaceMode::Overlay => move_index(&mut track.overlay, pt, delta),
        RacePlaceMode::Path => move_index(&mut track.path, pt, delta),
        RacePlaceMode::Start | RacePlaceMode::None => None,
    };
    if let Some(new_idx) = moved {
        app.race.selected_point = Some(new_idx);
        app.status_message = format!("Moved {} point to {}", mode_label(mode), new_idx + 1);
        commit_race_history(app, "Reorder Race Points", before);
    } else {
        app.status_message = "Point cannot move further".to_string();
    }
}

/// World position of the currently-selected race point (start / checkpoint /
/// overlay / path), if any. Used to anchor the transform gizmo.
pub(crate) fn selected_race_point_position(app: &AppState) -> Option<Vec3> {
    let track = app.race.tracks.get(app.race.selected_track)?;
    let pt = app.race.selected_point;
    let v = match (app.race.place_mode, pt) {
        (RacePlaceMode::Start, _) => track.start,
        (RacePlaceMode::Checkpoint, Some(i)) => track.checkpoints.get(i)?.pos,
        (RacePlaceMode::Overlay, Some(i)) => *track.overlay.get(i)?,
        (RacePlaceMode::Path, Some(i)) => *track.path.get(i)?,
        _ => return None,
    };
    Some(vec3(v.x, v.y, v.z))
}

/// Moves the currently-selected race point to an absolute world position.
pub(crate) fn set_selected_race_point_position(app: &mut AppState, pos: Vec3) {
    let mode = app.race.place_mode;
    let pt = app.race.selected_point;
    let idx = app.race.selected_track;
    let Some(track) = app.race.tracks.get_mut(idx) else {
        return;
    };
    let new = V3 {
        x: pos.x,
        y: pos.y,
        z: pos.z,
    };
    match (mode, pt) {
        (RacePlaceMode::Start, _) => track.start = new,
        (RacePlaceMode::Checkpoint, Some(i)) => {
            if let Some(c) = track.checkpoints.get_mut(i) {
                c.pos = new;
            }
        }
        (RacePlaceMode::Overlay, Some(i)) => {
            if let Some(p) = track.overlay.get_mut(i) {
                *p = new;
            }
        }
        (RacePlaceMode::Path, Some(i)) => {
            if let Some(p) = track.path.get_mut(i) {
                *p = new;
            }
        }
        _ => {}
    }
}

fn adjust_laps(app: &mut AppState, delta: i32) {
    let idx = app.race.selected_track;
    let before = race_history_snapshot(app);
    if let Some(t) = app.race.tracks.get_mut(idx) {
        t.laps = (t.laps + delta).clamp(1, 99);
        app.status_message = format!("Laps: {}", t.laps);
        commit_race_history(app, "Adjust Race Laps", before);
    }
}

fn adjust_radius(app: &mut AppState, delta: f32) {
    // Adjusts the selected checkpoint radius, else the default radius for new
    // checkpoints.
    let mode = app.race.place_mode;
    let pt = app.race.selected_point;
    let idx = app.race.selected_track;
    let before = race_history_snapshot(app);
    if let (RacePlaceMode::Checkpoint, Some(i)) = (mode, pt) {
        if let Some(track) = app.race.tracks.get_mut(idx) {
            if let Some(c) = track.checkpoints.get_mut(i) {
                c.r = (c.r + delta).clamp(2.0, 60.0);
                app.status_message = format!("Checkpoint radius: {:.0}", c.r);
                commit_race_history(app, "Adjust Race Checkpoint Radius", before);
                return;
            }
        }
    }
    app.race.default_radius = (app.race.default_radius + delta).clamp(2.0, 60.0);
    app.status_message = format!("Default checkpoint radius: {:.0}", app.race.default_radius);
    commit_race_history(app, "Adjust Race Default Radius", before);
}

fn auto_radar(app: &mut AppState) {
    match resolve_radar_path(&app.root, &app.race.radar_path) {
        Some(path) => {
            let before = race_history_snapshot(app);
            // Store relative to the map root when possible.
            let rel = path
                .strip_prefix(&app.root)
                .ok()
                .and_then(|p| p.to_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| path.display().to_string());
            app.race.radar_path = rel.clone();
            app.race.radar_tex = None;
            app.race.radar_tex_key.clear();
            app.status_message = format!("Radar backdrop: {rel}");
            commit_race_history(app, "Set Race Radar", before);
        }
        None => {
            app.status_message =
                "No radar.png found — browse to one or place it in the map's tracks/ folder"
                    .to_string();
        }
    }
}

fn generate_radar_image(app: &mut AppState) {
    let before = race_history_snapshot(app);
    match generate_map_radar_png(app) {
        Ok(path) => {
            let rel = path
                .strip_prefix(&app.root)
                .ok()
                .and_then(|path| path.to_str())
                .map(str::to_string)
                .unwrap_or_else(|| path.display().to_string());
            app.race.radar_path = rel.clone();
            app.race.world_size = RADAR_WORLD_SIZE;
            app.race.world_center_x = RADAR_WORLD_CENTER;
            app.race.world_center_y = RADAR_WORLD_CENTER;
            app.race.radar_tex = None;
            app.race.radar_tex_key.clear();
            commit_race_history(app, "Generate Race Radar", before);
            match save_race_tracks(
                &app.root,
                &app.race.tracks,
                &app.race.radar_path,
                app.race.world_size,
                app.race.world_center_x,
                app.race.world_center_y,
            ) {
                Ok(()) => {
                    mark_race_saved(app);
                    app.status_message = format!(
                        "Generated {}x{} radar for -3000..3000: {rel}",
                        RADAR_IMAGE_SIZE, RADAR_IMAGE_SIZE
                    );
                }
                Err(error) => {
                    app.status_message =
                        format!("Generated radar {rel}, but settings save failed: {error}");
                }
            }
        }
        Err(error) => app.status_message = format!("Radar generation failed: {error}"),
    }
}

/// Opens the native file picker so the user can browse to a radar image.
pub(crate) fn start_race_radar_browse(app: &mut AppState) {
    drain_text_input();
    if app.dff_picker_rx.is_some() {
        app.status_message = "File browser is already open".to_string();
        return;
    }
    let candidate = race_dir(&app.root);
    let start_dir = if candidate.is_dir() {
        candidate
    } else {
        app.root.clone()
    };
    let (tx, rx) = mpsc::channel();
    app.dff_picker_rx = Some(rx);
    thread::spawn(move || {
        let _ = tx.send((
            DffPickerKind::RaceRadar,
            choose_texture_image_path(start_dir),
        ));
    });
}

/// Stores a chosen radar image path (relative to the map root when possible) and
/// persists it into tracks.xml so it is remembered for this map.
pub(crate) fn set_race_radar_from_path(app: &mut AppState, path: PathBuf) {
    let before = race_history_snapshot(app);
    let rel = path
        .strip_prefix(&app.root)
        .ok()
        .and_then(|p| p.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| path.display().to_string());
    app.race.radar_path = rel.clone();
    app.race.radar_tex = None;
    app.race.radar_tex_key.clear();
    commit_race_history(app, "Set Race Radar", before);
    match save_race_tracks(
        &app.root,
        &app.race.tracks,
        &app.race.radar_path,
        app.race.world_size,
        app.race.world_center_x,
        app.race.world_center_y,
    ) {
        Ok(()) => {
            mark_race_saved(app);
            app.status_message = format!("Radar image saved: {rel}");
        }
        Err(e) => app.status_message = format!("Radar set to {rel}, but save failed: {e}"),
    }
}

fn generate_preview(app: &mut AppState) {
    let idx = app.race.selected_track;
    let before = race_history_snapshot(app);
    let root = app.root.clone();
    let radar = app.race.radar_path.clone();
    let ws = app.race.world_size;
    let cx = app.race.world_center_x;
    let cy = app.race.world_center_y;
    let Some(track) = app.race.tracks.get(idx) else {
        app.status_message = "No track selected".to_string();
        return;
    };
    // Preview the active subtrack, using the naming convention so the generated
    // PNG lands where the M_Race pack loader will look for it.
    let sub_count = track.effective_subtracks().len();
    let si = track.active_subtrack.min(sub_count.saturating_sub(1));
    let (image_rel, overlay_rel) =
        subtrack_preview_rel(&track.name, &track.subtrack_name, si, sub_count);
    let checkpoints = track.checkpoints.clone();
    let overlay = track.overlay.clone();
    let path = track.path.clone();
    let start = track.start;
    match generate_race_preview(
        &root,
        &checkpoints,
        &overlay,
        &path,
        start,
        &image_rel,
        &overlay_rel,
        &radar,
        ws,
        cx,
        cy,
    ) {
        Ok(res) => {
            if let Some(t) = app.race.tracks.get_mut(idx) {
                t.image = res.image_rel;
                t.overlay_image = res.overlay_rel;
                t.commit_active_subtrack();
            }
            app.status_message = "Generated race preview + overlay in tracks/".to_string();
            commit_race_history(app, "Generate Race Preview", before);
        }
        Err(e) => app.status_message = format!("Preview failed: {e}"),
    }
}

fn save_tracks(app: &mut AppState) {
    if app.race.tracks.is_empty() {
        app.status_message = "No tracks to save".to_string();
        return;
    }
    match save_race_tracks(
        &app.root,
        &app.race.tracks,
        &app.race.radar_path,
        app.race.world_size,
        app.race.world_center_x,
        app.race.world_center_y,
    ) {
        Ok(()) => match patch_map_meta_for_tracks(&app.root) {
            Ok(true) => {
                mark_race_saved(app);
                app.status_message = format!(
                    "Saved {} track(s) to tracks/tracks.xml + updated meta.xml",
                    app.race.tracks.len()
                )
            }
            Ok(false) => {
                mark_race_saved(app);
                app.status_message = format!(
                    "Saved {} track(s) to tracks/tracks.xml",
                    app.race.tracks.len()
                )
            }
            Err(e) => app.status_message = format!("Saved tracks, but meta.xml patch failed: {e}"),
        },
        Err(e) => app.status_message = format!("Save failed: {e}"),
    }
}

// ---------------------------------------------------------------------------
// Viewport placement
// ---------------------------------------------------------------------------

fn ground_hit(app: &AppState, viewport: Rect, mouse: Vec2, z: f32) -> Option<V3> {
    let (origin, dir) = viewport_ray(app, viewport, mouse)?;
    if dir.z.abs() < 0.00001 {
        return None;
    }
    let t = (z - origin.z) / dir.z;
    if t <= 0.0 {
        return None;
    }
    let hit = origin + dir * t;
    Some(V3 {
        x: hit.x,
        y: hit.y,
        z,
    })
}

/// Selects the nearest race point of the active kind under the cursor (within a
/// screen-space threshold) so the transform gizmo snaps to it. Returns true if a
/// point was selected.
pub(crate) fn pick_race_point(app: &mut AppState, viewport: Rect, mouse: Vec2) -> bool {
    // Collect candidate world positions up front so we don't hold a borrow of the
    // track while mutating the selection afterwards.
    let (start, points): (V3, Vec<V3>) = {
        let Some(track) = app.race.tracks.get(app.race.selected_track) else {
            return false;
        };
        let points = match app.race.place_mode {
            RacePlaceMode::Checkpoint => track.checkpoints.iter().map(|c| c.pos).collect(),
            RacePlaceMode::Overlay => track.overlay.clone(),
            RacePlaceMode::Path => track.path.clone(),
            _ => Vec::new(),
        };
        (track.start, points)
    };
    if app.race.place_mode == RacePlaceMode::Start {
        if let Some(screen) = world_to_screen(app, viewport, vec3(start.x, start.y, start.z)) {
            if mouse.distance(screen) < 14.0 {
                app.race.selected_point = None;
                return true;
            }
        }
        return false;
    }
    let mut best: Option<usize> = None;
    let mut best_dist = 14.0_f32;
    for (i, p) in points.iter().enumerate() {
        if let Some(screen) = world_to_screen(app, viewport, vec3(p.x, p.y, p.z)) {
            let d = mouse.distance(screen);
            if d < best_dist {
                best_dist = d;
                best = Some(i);
            }
        }
    }
    if let Some(i) = best {
        app.race.selected_point = Some(i);
        true
    } else {
        false
    }
}

/// When placing a checkpoint in the 3D viewport, returns the index at which a
/// new checkpoint should be inserted if the cursor is near an existing route
/// segment (start -> cp0 -> cp1 -> ...), or `None` to append at the end.
fn checkpoint_insert_index(app: &AppState, viewport: Rect, mouse: Vec2) -> Option<usize> {
    let track = app.race.tracks.get(app.race.selected_track)?;
    if track.checkpoints.is_empty() {
        return None;
    }
    let mut best_dist = 24.0_f32;
    let mut best_at: Option<usize> = None;
    // Previous route node in screen space (start, if present, then each cp).
    let mut prev: Option<Vec2> = if track.start != V3::default() {
        world_to_screen(
            app,
            viewport,
            vec3(track.start.x, track.start.y, track.start.z),
        )
    } else {
        None
    };
    for (i, cp) in track.checkpoints.iter().enumerate() {
        let Some(cur) = world_to_screen(app, viewport, vec3(cp.pos.x, cp.pos.y, cp.pos.z)) else {
            prev = None;
            continue;
        };
        if let Some(pv) = prev {
            let d = dist_to_segment(mouse, pv, cur);
            if d < best_dist {
                best_dist = d;
                // Hit on the segment ending at checkpoint i -> insert before i.
                best_at = Some(i);
            }
        }
        prev = Some(cur);
    }
    best_at
}

pub(crate) fn handle_race_viewport_click(app: &mut AppState, viewport: Rect, mouse: Vec2) -> bool {
    if app.active_tab != AppTab::Race {
        return false;
    }
    if !viewport.contains(mouse) || !is_mouse_button_pressed(MouseButton::Left) {
        return false;
    }
    // In a transform mode (Move/Rotate), viewport clicks don't place new points.
    // A click on a gizmo axis is left for the gizmo drag system; otherwise the
    // click selects the nearest existing point so the gizmo can grab it.
    if app.transform_mode != TransformMode::Select {
        if gizmo_axis_at(app, viewport, mouse).is_some() {
            return false;
        }
        return pick_race_point(app, viewport, mouse);
    }
    if app.race.place_mode == RacePlaceMode::None {
        return false;
    }
    let mode = app.race.place_mode;
    let z = app.race.place_z;
    let Some(hit) = ground_hit(app, viewport, mouse, z) else {
        return false;
    };
    let radius = app.race.default_radius;
    let idx = app.race.selected_track;
    let before = race_history_snapshot(app);
    // For checkpoints, clicking on an existing route segment inserts between it.
    let cp_insert = if mode == RacePlaceMode::Checkpoint {
        checkpoint_insert_index(app, viewport, mouse)
    } else {
        None
    };
    let Some(track) = app.race.tracks.get_mut(idx) else {
        app.status_message = "Create a track first (New Track)".to_string();
        return true;
    };
    match mode {
        RacePlaceMode::Start => {
            track.start = hit;
            if track.find == V3::default() {
                track.find = hit;
            }
            app.race.selected_point = None;
            app.status_message = format!("Start set to {:.0}, {:.0}", hit.x, hit.y);
        }
        RacePlaceMode::Checkpoint => {
            let cp = RaceCheckpoint {
                pos: hit,
                r: radius,
            };
            if let Some(at) = cp_insert {
                let at = at.min(track.checkpoints.len());
                track.checkpoints.insert(at, cp);
                app.race.selected_point = Some(at);
                app.status_message = format!("Inserted checkpoint at position {}", at + 1);
            } else {
                track.checkpoints.push(cp);
                app.race.selected_point = Some(track.checkpoints.len() - 1);
                app.status_message = format!("Added checkpoint {}", track.checkpoints.len());
            }
        }
        RacePlaceMode::Overlay => {
            track.overlay.push(hit);
            app.race.selected_point = Some(track.overlay.len() - 1);
            app.status_message = format!("Added overlay point {}", track.overlay.len());
        }
        RacePlaceMode::Path => {
            track.path.push(hit);
            app.race.selected_point = Some(track.path.len() - 1);
            app.status_message = format!("Added path point {}", track.path.len());
        }
        RacePlaceMode::None => {}
    }
    commit_race_history(app, "Add Race Point", before);
    true
}

// ---------------------------------------------------------------------------
// Side-panel click + scroll handling
// ---------------------------------------------------------------------------

fn race_row_at(app: &AppState, mouse: Vec2) -> Option<usize> {
    let rect = race_list_rect();
    if !rect.contains(mouse) {
        return None;
    }
    let local = ((mouse.y - rect.y - 12.0) / 22.0).floor().max(0.0) as usize;
    Some(app.race.list_scroll.floor() as usize + local)
}

fn race_track_row_at(app: &AppState, mouse: Vec2) -> Option<usize> {
    let rect = race_track_list_rect();
    if !rect.contains(mouse) {
        return None;
    }
    let local = ((mouse.y - rect.y - 8.0) / 26.0).floor().max(0.0) as usize;
    Some(app.race.track_scroll.floor() as usize + local)
}

pub(crate) fn scroll_race_list(app: &mut AppState, wheel: f32) {
    let count = app
        .race
        .tracks
        .get(app.race.selected_track)
        .map(|t| active_len(t, app.race.place_mode))
        .unwrap_or(0);
    let max_scroll = count.saturating_sub(race_visible_rows()) as f32;
    app.race.list_scroll = (app.race.list_scroll - wheel * 3.0).clamp(0.0, max_scroll);
}

pub(crate) fn scroll_race_tracks(app: &mut AppState, wheel: f32) {
    let max_scroll = app
        .race
        .tracks
        .len()
        .saturating_sub(race_track_visible_rows()) as f32;
    app.race.track_scroll = (app.race.track_scroll - wheel * 3.0).clamp(0.0, max_scroll);
}

/// Toggles the Preview-tab radar minimap overlay. Returns true if consumed.
pub(crate) fn handle_race_overlay_toggle(app: &mut AppState, mouse: Vec2) -> bool {
    if app.active_tab != AppTab::Preview || !is_mouse_button_pressed(MouseButton::Left) {
        return false;
    }
    if race_overlay_toggle_rect().contains(mouse) {
        app.race.preview_overlay = !app.race.preview_overlay;
        app.status_message = if app.race.preview_overlay {
            "Radar overlay on".to_string()
        } else {
            "Radar overlay off".to_string()
        };
        return true;
    }
    false
}

/// Consumes Preview-minimap clicks and opens a confirmation prompt when the
/// user double-clicks a point on the radar.
pub(crate) fn handle_race_minimap_click(app: &mut AppState, mouse: Vec2) -> bool {
    if app.active_tab != AppTab::Preview
        || !app.race.preview_overlay
        || !is_mouse_button_pressed(MouseButton::Left)
    {
        return false;
    }
    let rect = race_preview_minimap_rect();
    if !rect.contains(mouse) {
        return false;
    }
    if app.race.radar_tex.is_none() {
        app.status_message = "No radar image is available for camera teleport".to_string();
        return true;
    }
    let world_size = app.race.world_size;
    if !world_size.is_finite() || world_size <= 0.0 {
        app.status_message = "Radar world size must be greater than zero".to_string();
        return true;
    }

    let now = get_time();
    let is_double_click = app.race.preview_overlay_last_click_at >= 0.0
        && now - app.race.preview_overlay_last_click_at <= PREVIEW_MINIMAP_DOUBLE_CLICK_SECONDS
        && mouse.distance(app.race.preview_overlay_last_click_pos)
            <= PREVIEW_MINIMAP_DOUBLE_CLICK_DISTANCE;
    if !is_double_click {
        app.race.preview_overlay_last_click_at = now;
        app.race.preview_overlay_last_click_pos = mouse;
        app.status_message = "Double-click the radar to teleport the camera".to_string();
        return true;
    }

    // Clear the click history so a third click cannot immediately reopen the
    // prompt after it is dismissed or accepted.
    app.race.preview_overlay_last_click_at = -1.0;
    let target_xy = race_preview_minimap_screen_to_world(
        rect,
        mouse,
        world_size,
        app.race.world_center_x,
        app.race.world_center_y,
    );
    let target = vec3(target_xy.x, target_xy.y, app.camera.pos.z);
    app.confirm_dialog = Some(ConfirmDialog {
        action: ConfirmAction::TeleportCamera(target),
        title: "Teleport Camera?".to_string(),
        body: format!("Move the camera to X {:.1}, Y {:.1}?", target.x, target.y),
        detail: format!(
            "The current height ({:.1}) and viewing direction will be preserved.",
            target.z
        ),
        primary_label: "Teleport".to_string(),
        secondary_label: None,
        secondary_action: None,
    });
    app.status_message = "Confirm camera teleport".to_string();
    true
}

pub(crate) fn handle_race_click(app: &mut AppState, mouse: Vec2) -> bool {
    if app.active_tab != AppTab::Race || !is_mouse_button_pressed(MouseButton::Left) {
        return false;
    }

    // A click anywhere commits an in-progress track/subtrack rename.
    if app.race_name_edit.is_some() {
        apply_race_name_edit(app);
        return true;
    }

    // ---- Left "Races" panel ----
    if race_left_btn(0, 0, 3).contains(mouse) {
        new_track(app);
        return true;
    }
    if race_left_btn(0, 1, 3).contains(mouse) {
        delete_track(app);
        return true;
    }
    if race_left_btn(0, 2, 3).contains(mouse) {
        save_tracks(app);
        return true;
    }
    if let Some(row) = race_track_row_at(app, mouse) {
        if row < app.race.tracks.len() {
            select_track(app, row);
        }
        return true;
    }

    // ---- Right "Track" panel ----
    // Row 0: placement modes.
    let modes = [
        RacePlaceMode::Start,
        RacePlaceMode::Checkpoint,
        RacePlaceMode::Overlay,
        RacePlaceMode::Path,
    ];
    for (i, m) in modes.iter().enumerate() {
        if race_grid_rect(0, i, 4).contains(mouse) {
            set_mode(app, *m);
            return true;
        }
    }
    // Row 1: point ops.
    if race_grid_rect(1, 0, 4).contains(mouse) {
        delete_selected_point(app);
        return true;
    }
    if race_grid_rect(1, 1, 4).contains(mouse) {
        clear_active(app);
        return true;
    }
    if race_grid_rect(1, 2, 4).contains(mouse) {
        nudge_point_z(app, -1.0);
        return true;
    }
    if race_grid_rect(1, 3, 4).contains(mouse) {
        nudge_point_z(app, 1.0);
        return true;
    }
    // Row 2: laps / radius.
    if race_grid_rect(2, 0, 4).contains(mouse) {
        adjust_laps(app, -1);
        return true;
    }
    if race_grid_rect(2, 1, 4).contains(mouse) {
        adjust_laps(app, 1);
        return true;
    }
    if race_grid_rect(2, 2, 4).contains(mouse) {
        adjust_radius(app, -1.0);
        return true;
    }
    if race_grid_rect(2, 3, 4).contains(mouse) {
        adjust_radius(app, 1.0);
        return true;
    }
    // Row 3: radar backdrop.
    if race_grid_rect(3, 0, 2).contains(mouse) {
        start_race_radar_browse(app);
        return true;
    }
    if race_grid_rect(3, 1, 2).contains(mouse) {
        auto_radar(app);
        return true;
    }
    // Row 4: generate the full-map radar backdrop.
    if race_grid_rect(4, 0, 1).contains(mouse) {
        generate_radar_image(app);
        return true;
    }
    // Row 5: generate preview.
    if race_grid_rect(5, 0, 1).contains(mouse) {
        generate_preview(app);
        return true;
    }
    // Row 6: reorder selected point.
    if race_grid_rect(6, 0, 2).contains(mouse) {
        move_selected_point_order(app, -1);
        return true;
    }
    if race_grid_rect(6, 1, 2).contains(mouse) {
        move_selected_point_order(app, 1);
        return true;
    }
    // Row 7: subtrack navigation + add/delete.
    if race_grid_rect(7, 0, 4).contains(mouse) {
        cycle_subtrack(app, -1);
        return true;
    }
    if race_grid_rect(7, 1, 4).contains(mouse) {
        cycle_subtrack(app, 1);
        return true;
    }
    if race_grid_rect(7, 2, 4).contains(mouse) {
        add_subtrack(app);
        return true;
    }
    if race_grid_rect(7, 3, 4).contains(mouse) {
        delete_subtrack(app);
        return true;
    }
    // Row 8: rename track / subtrack.
    if race_grid_rect(8, 0, 2).contains(mouse) {
        start_race_name_edit(app, RaceNameTarget::Track);
        return true;
    }
    if race_grid_rect(8, 1, 2).contains(mouse) {
        start_race_name_edit(app, RaceNameTarget::Subtrack);
        return true;
    }
    // Point list row selection.
    if let Some(row) = race_row_at(app, mouse) {
        let count = app
            .race
            .tracks
            .get(app.race.selected_track)
            .map(|t| active_len(t, app.race.place_mode))
            .unwrap_or(0);
        if row < count {
            app.race.selected_point = Some(row);
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Preview-tab radar minimap overlay
// ---------------------------------------------------------------------------

/// Ensures the radar texture used by the Preview minimap is loaded and matches
/// the currently configured radar path. Runs each frame while the overlay is on.
pub(crate) fn update_race_minimap(app: &mut AppState) {
    let want = (app.active_tab == AppTab::Preview && app.race.preview_overlay)
        || (app.active_tab == AppTab::Race && app.race.radar_2d);
    if !want {
        return;
    }
    ensure_race_loaded(app);
    if app.race.radar_tex.is_some() && app.race.radar_tex_key == app.race.radar_path {
        return;
    }
    match load_radar_rgba(&app.root, &app.race.radar_path) {
        Some((w, h, rgba)) => {
            let tex = Texture2D::from_rgba8(w as u16, h as u16, &rgba);
            tex.set_filter(FilterMode::Linear);
            app.race.radar_tex = Some(tex);
            app.race.radar_tex_key = app.race.radar_path.clone();
        }
        None => {
            app.race.radar_tex = None;
            app.race.radar_tex_key = app.race.radar_path.clone();
        }
    }
}

pub(crate) fn draw_race_overlay_toggle(app: &AppState) {
    if app.active_tab != AppTab::Preview {
        return;
    }
    text_button(
        &app.ui_font,
        race_overlay_toggle_rect(),
        "Radar Overlay",
        app.race.preview_overlay,
    );
}

pub(crate) fn draw_race_minimap_overlay(app: &AppState) {
    if app.active_tab != AppTab::Preview || !app.race.preview_overlay {
        return;
    }
    let rect = race_preview_minimap_rect();
    let size = rect.w;
    let x = rect.x;
    let y = rect.y;
    draw_rrect_bordered(
        x - 6.0,
        y - 6.0,
        size + 12.0,
        size + 12.0,
        8.0,
        1.0,
        Color::new(0.050, 0.058, 0.070, 0.92),
        ui_border(),
    );
    let Some(tex) = app.race.radar_tex.as_ref() else {
        ui_text(
            &app.ui_font,
            "No radar image — set one in the Race tab",
            x + 10.0,
            y + size * 0.5,
            ui_dim(),
        );
        return;
    };
    draw_texture_ex(
        tex,
        x,
        y,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(size, size)),
            ..Default::default()
        },
    );

    let ws = app.race.world_size;
    let cx = app.race.world_center_x;
    let cy = app.race.world_center_y;

    // Selected track outline (checkpoints) for context.
    if let Some(track) = app.race.tracks.get(app.race.selected_track) {
        let pts: Vec<(f32, f32)> = track
            .checkpoints
            .iter()
            .map(|c| race_world_to_radar_uv(c.pos.x, c.pos.y, ws, cx, cy))
            .collect();
        for w in pts.windows(2) {
            draw_line(
                x + w[0].0 * size,
                y + w[0].1 * size,
                x + w[1].0 * size,
                y + w[1].1 * size,
                1.5,
                Color::new(1.0, 0.85, 0.2, 0.9),
            );
        }
        for (idx, cp) in track.checkpoints.iter().enumerate() {
            let (u, v) = race_world_to_radar_uv(cp.pos.x, cp.pos.y, ws, cx, cy);
            if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
                continue;
            }
            let selected = app.race.place_mode == RacePlaceMode::Checkpoint
                && app.race.selected_point == Some(idx);
            let px = x + u * size;
            let py = y + v * size;
            draw_circle(
                px,
                py,
                if selected { 5.5 } else { 3.0 },
                if selected {
                    Color::new(1.0, 1.0, 0.25, 1.0)
                } else {
                    Color::new(1.0, 0.7, 0.15, 0.9)
                },
            );
            if selected {
                draw_circle_lines(px, py, 8.0, 2.0, WHITE);
            }
        }
    }

    // Camera position and facing marker. Camera yaw 0 faces world +Y, which is
    // north/up on the radar.
    let (u, v) = race_world_to_radar_uv(app.camera.pos.x, app.camera.pos.y, ws, cx, cy);
    if (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v) {
        let mx = x + u * size;
        let my = y + v * size;
        let facing = vec2(app.camera.yaw.sin(), -app.camera.yaw.cos());
        let tip = vec2(mx, my) + facing * 18.0;
        let side = vec2(-facing.y, facing.x);
        let arrow_base = vec2(mx, my) + facing * 11.0;
        draw_line(mx, my, tip.x, tip.y, 2.5, Color::new(0.4, 0.9, 1.0, 1.0));
        draw_triangle(
            tip,
            arrow_base + side * 4.0,
            arrow_base - side * 4.0,
            Color::new(0.4, 0.9, 1.0, 1.0),
        );
        draw_circle(mx, my, 5.0, Color::new(0.1, 0.55, 0.8, 1.0));
        draw_circle_lines(mx, my, 5.0, 1.5, WHITE);
    }
    ui_text(
        &app.ui_font,
        &format!("Cam {:.0}, {:.0}", app.camera.pos.x, app.camera.pos.y),
        x + 8.0,
        y + size - 10.0,
        Color::new(0.8, 0.9, 1.0, 1.0),
    );
    ui_text(
        &app.ui_font,
        "Double-click to teleport",
        x + size - 172.0,
        y + 20.0,
        Color::new(0.8, 0.9, 1.0, 1.0),
    );
}

// ---------------------------------------------------------------------------
// Drawing (Race tab: left Races panel + right Track panel)
// ---------------------------------------------------------------------------

fn draw_race_left_panel(app: &AppState) {
    let x = RACE_LEFT_X;
    let y = TOP_H + 12.0;
    let w = race_left_w();
    draw_panel_rect(
        &app.ui_font,
        Rect::new(x, y, w, (screen_height() - STATUS_H - y - 12.0).max(200.0)),
        Some("Races"),
    );

    text_button(&app.ui_font, race_left_btn(0, 0, 3), "New Track", false);
    text_button(&app.ui_font, race_left_btn(0, 1, 3), "Delete Track", false);
    text_button(
        &app.ui_font,
        race_left_btn(0, 2, 3),
        "Save Race Data",
        false,
    );

    let list = race_track_list_rect();
    draw_rrect_bordered(
        list.x,
        list.y,
        list.w,
        list.h,
        6.0,
        1.0,
        Color::new(0.050, 0.058, 0.070, 1.0),
        ui_border(),
    );

    if app.race.tracks.is_empty() {
        ui_text(
            &app.ui_font,
            "No tracks yet — click New Track",
            list.x + 12.0,
            list.y + 26.0,
            ui_dim(),
        );
        return;
    }

    let start = app.race.track_scroll.floor() as usize;
    let end = (start + race_track_visible_rows()).min(app.race.tracks.len());
    for (vis, idx) in (start..end).enumerate() {
        let Some(track) = app.race.tracks.get(idx) else {
            break;
        };
        let row_y = list.y + 8.0 + vis as f32 * 26.0;
        let selected = idx == app.race.selected_track;
        if selected {
            draw_rectangle(list.x + 4.0, row_y, list.w - 8.0, 24.0, ui_surface_active());
        }
        ui_text(
            &app.ui_font,
            &ellipsize_width(&track.name, 16, list.w - 152.0),
            list.x + 12.0,
            row_y + 17.0,
            if selected { WHITE } else { LIGHTGRAY },
        );
        let track_stats = format!(
            "{} lap{} · {} checkpoint{}",
            track.laps,
            if track.laps == 1 { "" } else { "s" },
            track.checkpoints.len(),
            if track.checkpoints.len() == 1 {
                ""
            } else {
                "s"
            }
        );
        let track_stats = ellipsize_width(&track_stats, 16, 144.0);
        let stats_w = ui_text_width(&track_stats, 16);
        ui_text(
            &app.ui_font,
            &track_stats,
            list.x + list.w - stats_w - 12.0,
            row_y + 17.0,
            if selected {
                Color::new(0.7, 0.85, 1.0, 1.0)
            } else {
                ui_dim()
            },
        );
    }
}

fn draw_race_right_panel(app: &AppState) {
    let x = race_panel_x();
    let y = TOP_H + 12.0;
    let w = RIGHT_PANEL_W - 24.0;
    draw_panel_rect(
        &app.ui_font,
        Rect::new(x, y, w, (screen_height() - STATUS_H - y - 12.0).max(520.0)),
        Some("Track"),
    );

    let track = app.race.tracks.get(app.race.selected_track);

    // Row 0: placement modes (active highlighted).
    let modes = [
        (RacePlaceMode::Start, "Start"),
        (RacePlaceMode::Checkpoint, "Checkpoint"),
        (RacePlaceMode::Overlay, "Overlay"),
        (RacePlaceMode::Path, "Path"),
    ];
    for (i, (m, label)) in modes.iter().enumerate() {
        text_button(
            &app.ui_font,
            race_grid_rect(0, i, 4),
            label,
            app.race.place_mode == *m,
        );
    }

    // Row 1: point ops.
    text_button(&app.ui_font, race_grid_rect(1, 0, 4), "Delete Point", false);
    text_button(&app.ui_font, race_grid_rect(1, 1, 4), "Clear Points", false);
    text_button(&app.ui_font, race_grid_rect(1, 2, 4), "Lower Z", false);
    text_button(&app.ui_font, race_grid_rect(1, 3, 4), "Raise Z", false);

    // Row 2: laps / radius.
    text_button(&app.ui_font, race_grid_rect(2, 0, 4), "Lap -", false);
    text_button(&app.ui_font, race_grid_rect(2, 1, 4), "Lap +", false);
    text_button(&app.ui_font, race_grid_rect(2, 2, 4), "Radius -", false);
    text_button(&app.ui_font, race_grid_rect(2, 3, 4), "Radius +", false);

    // Row 3: radar backdrop.
    text_button(&app.ui_font, race_grid_rect(3, 0, 2), "Browse Radar", false);
    text_button(&app.ui_font, race_grid_rect(3, 1, 2), "Auto Radar", false);

    // Row 4: generate the full-map radar backdrop.
    text_button(
        &app.ui_font,
        race_grid_rect(4, 0, 1),
        "Generate Radar Image",
        false,
    );

    // Row 5: generate the selected race preview.
    text_button(
        &app.ui_font,
        race_grid_rect(5, 0, 1),
        "Generate Preview",
        false,
    );

    // Row 6: reorder selected point.
    text_button(&app.ui_font, race_grid_rect(6, 0, 2), "Move Up", false);
    text_button(&app.ui_font, race_grid_rect(6, 1, 2), "Move Down", false);

    // Row 7: subtrack navigation + add/delete.
    text_button(&app.ui_font, race_grid_rect(7, 0, 4), "Previous", false);
    text_button(&app.ui_font, race_grid_rect(7, 1, 4), "Next", false);
    text_button(&app.ui_font, race_grid_rect(7, 2, 4), "Add Subtrack", false);
    text_button(
        &app.ui_font,
        race_grid_rect(7, 3, 4),
        "Delete Subtrack",
        false,
    );

    // Row 8: rename track / subtrack.
    let renaming_track = app
        .race_name_edit
        .as_ref()
        .is_some_and(|e| e.target == RaceNameTarget::Track);
    let renaming_sub = app
        .race_name_edit
        .as_ref()
        .is_some_and(|e| e.target == RaceNameTarget::Subtrack);
    text_button(
        &app.ui_font,
        race_grid_rect(8, 0, 2),
        "Rename Track",
        renaming_track,
    );
    text_button(
        &app.ui_font,
        race_grid_rect(8, 1, 2),
        "Rename Sub",
        renaming_sub,
    );

    // Info lines.
    let info_y = race_info_y();
    let summary = match track {
        Some(t) => format!(
            "{}  |  {} lap{}  |  {} checkpoint{}  |  {} overlay  |  {} path",
            t.name,
            t.laps,
            if t.laps == 1 { "" } else { "s" },
            t.checkpoints.len(),
            if t.checkpoints.len() == 1 { "" } else { "s" },
            t.overlay.len(),
            t.path.len()
        ),
        None => "No track selected — pick one on the left".to_string(),
    };
    ui_text(
        &app.ui_font,
        &ellipsize_width(&summary, 16, w - 28.0),
        x + 14.0,
        info_y,
        LIGHTGRAY,
    );

    // Subtrack line — or an inline edit box while renaming.
    let sub_y = info_y + 18.0;
    if let Some(edit) = app.race_name_edit.as_ref() {
        let label = match edit.target {
            RaceNameTarget::Track => "Track name:",
            RaceNameTarget::Subtrack => "Subtrack name:",
        };
        ui_text(&app.ui_font, label, x + 14.0, sub_y, ui_accent());
        let edit_rect = Rect::new(x + 120.0, sub_y - 15.0, w - 130.0, 20.0);
        draw_rrect_bordered(
            edit_rect.x,
            edit_rect.y,
            edit_rect.w,
            edit_rect.h,
            5.0,
            1.0,
            Color::new(0.050, 0.058, 0.070, 1.0),
            ui_accent(),
        );
        let cursor = clamp_char_boundary(&edit.buffer, edit.cursor);
        let visible = ellipsize_width(&edit.buffer, 22, edit_rect.w - 14.0);
        ui_text(&app.ui_font, &visible, edit_rect.x + 7.0, sub_y, WHITE);
        if (get_time() * 2.0) as i32 % 2 == 0 {
            let prefix = &edit.buffer[..cursor];
            let caret_x = (edit_rect.x + 7.0 + ui_text_width(prefix, 16))
                .min(edit_rect.x + edit_rect.w - 7.0);
            draw_line(
                caret_x,
                edit_rect.y + 4.0,
                caret_x,
                edit_rect.y + edit_rect.h - 4.0,
                1.0,
                WHITE,
            );
        }
    } else {
        let sub_txt = match track {
            Some(t) => subtrack_summary(t),
            None => "No subtrack".to_string(),
        };
        ui_text(
            &app.ui_font,
            &ellipsize(&sub_txt, 46),
            x + 14.0,
            sub_y,
            Color::new(0.80, 0.86, 0.70, 1.0),
        );
    }

    let radar_txt = if app.race.radar_path.trim().is_empty() {
        "Radar: (none — Browse/Auto)".to_string()
    } else {
        format!("Radar: {}", ellipsize(&app.race.radar_path, 40))
    };
    ui_text(
        &app.ui_font,
        &radar_txt,
        x + 14.0,
        info_y + 36.0,
        Color::new(0.66, 0.82, 0.96, 1.0),
    );
    let placement_help = format!(
        "Placement: {} — click viewport to add; Z {:.1}, radius {:.0}",
        mode_label(app.race.place_mode),
        app.race.place_z,
        app.race.default_radius
    );
    ui_text(
        &app.ui_font,
        &ellipsize_width(&placement_help, 16, w - 28.0),
        x + 14.0,
        info_y + 54.0,
        ui_dim(),
    );

    // Active point list.
    let list = race_list_rect();
    draw_rrect_bordered(
        list.x,
        list.y,
        list.w,
        list.h,
        6.0,
        1.0,
        Color::new(0.050, 0.058, 0.070, 1.0),
        ui_border(),
    );
    let Some(track) = track else {
        return;
    };
    let mode = app.race.place_mode;
    let count = active_len(track, mode);
    let start = app.race.list_scroll.floor() as usize;
    let end = (start + race_visible_rows()).min(count);
    for (vis, idx) in (start..end).enumerate() {
        let row_y = list.y + 24.0 + vis as f32 * 22.0;
        let selected = app.race.selected_point == Some(idx);
        if selected {
            draw_rectangle(
                list.x + 4.0,
                row_y - 15.0,
                list.w - 8.0,
                20.0,
                ui_surface_active(),
            );
        }
        let label = match mode {
            RacePlaceMode::Checkpoint => track.checkpoints.get(idx).map(|c| {
                format!(
                    "#{:02}  {:.0}, {:.0}, {:.0}  r{:.0}",
                    idx + 1,
                    c.pos.x,
                    c.pos.y,
                    c.pos.z,
                    c.r
                )
            }),
            RacePlaceMode::Overlay => track
                .overlay
                .get(idx)
                .map(|p| format!("#{:02}  {:.0}, {:.0}, {:.0}", idx + 1, p.x, p.y, p.z)),
            RacePlaceMode::Path => track
                .path
                .get(idx)
                .map(|p| format!("#{:02}  {:.0}, {:.0}, {:.0}", idx + 1, p.x, p.y, p.z)),
            RacePlaceMode::Start => Some(format!(
                "start  {:.0}, {:.0}, {:.0}",
                track.start.x, track.start.y, track.start.z
            )),
            RacePlaceMode::None => None,
        };
        if let Some(label) = label {
            ui_text(
                &app.ui_font,
                &label,
                x + 18.0,
                row_y,
                if selected { ui_accent() } else { LIGHTGRAY },
            );
        }
    }
    if count == 0 {
        ui_text(
            &app.ui_font,
            "Pick a mode, then left-click in the viewport to place points",
            x + 18.0,
            list.y + 24.0,
            ui_dim(),
        );
    }
}

pub(crate) fn draw_race_panel(app: &AppState) {
    draw_race_left_panel(app);
    draw_race_right_panel(app);
}

// ---------------------------------------------------------------------------
// 2D radar editor (Race tab): a top-down view of the radar image where race
// points can be placed / selected / dragged directly on the map.
// ---------------------------------------------------------------------------

/// Toggle button pinned to the top-left of the central viewport in the Race tab.
fn race_2d_toggle_rect() -> Rect {
    Rect::new(PANEL_W + 16.0, TOP_H + 12.0, 160.0, 26.0)
}

/// Square region within the central viewport where the radar image is drawn.
pub(crate) fn race_2d_view_rect() -> Rect {
    let vp = editor_viewport_rect();
    let top = vp.y + 48.0;
    let avail_w = (vp.w - 32.0).max(64.0);
    let avail_h = (vp.h - 48.0 - 16.0).max(64.0);
    let size = avail_w.min(avail_h);
    let x = vp.x + (vp.w - size) * 0.5;
    let y = top + (avail_h - size) * 0.5;
    Rect::new(x, y, size, size)
}

fn race_2d_uv_to_screen(app: &AppState, view: Rect, u: f32, v: f32) -> (f32, f32) {
    let z = app.race.radar_2d_zoom.max(1.0);
    (
        view.x + (u - app.race.radar_2d_pan_u) * z * view.w,
        view.y + (v - app.race.radar_2d_pan_v) * z * view.h,
    )
}

fn race_2d_world_to_screen(app: &AppState, view: Rect, wx: f32, wy: f32) -> (f32, f32) {
    let (u, v) = race_world_to_radar_uv(
        wx,
        wy,
        app.race.world_size,
        app.race.world_center_x,
        app.race.world_center_y,
    );
    race_2d_uv_to_screen(app, view, u, v)
}

fn race_2d_screen_to_world(app: &AppState, view: Rect, mouse: Vec2) -> (f32, f32) {
    let z = app.race.radar_2d_zoom.max(1.0);
    let u = (app.race.radar_2d_pan_u + (mouse.x - view.x) / (z * view.w)).clamp(0.0, 1.0);
    let v = (app.race.radar_2d_pan_v + (mouse.y - view.y) / (z * view.h)).clamp(0.0, 1.0);
    let ws = app.race.world_size;
    let left = app.race.world_center_x - ws * 0.5;
    let top = app.race.world_center_y + ws * 0.5;
    (left + u * ws, top - v * ws)
}

/// Clamps the pan window so the zoomed view stays within the radar bounds.
fn race_2d_clamp_pan(app: &mut AppState) {
    let z = app.race.radar_2d_zoom.max(1.0);
    let win = 1.0 / z;
    let max_pan = (1.0 - win).max(0.0);
    app.race.radar_2d_pan_u = app.race.radar_2d_pan_u.clamp(0.0, max_pan);
    app.race.radar_2d_pan_v = app.race.radar_2d_pan_v.clamp(0.0, max_pan);
}

/// Scroll-wheel zoom for the 2D radar editor, keeping the point under the cursor
/// fixed. Called from the input dispatch when the editor is active.
pub(crate) fn scroll_race_2d(app: &mut AppState, wheel: f32, mouse: Vec2) {
    if app.active_tab != AppTab::Race || !app.race.radar_2d {
        return;
    }
    let view = race_2d_view_rect();
    if !view.contains(mouse) {
        return;
    }
    let z_old = app.race.radar_2d_zoom.max(1.0);
    // UV under the cursor before zooming.
    let u_cursor = app.race.radar_2d_pan_u + (mouse.x - view.x) / (z_old * view.w);
    let v_cursor = app.race.radar_2d_pan_v + (mouse.y - view.y) / (z_old * view.h);
    let factor = if wheel > 0.0 { 1.2 } else { 1.0 / 1.2 };
    let z_new = (z_old * factor).clamp(1.0, 16.0);
    // Keep the cursor UV anchored.
    app.race.radar_2d_pan_u = u_cursor - (mouse.x - view.x) / (z_new * view.w);
    app.race.radar_2d_pan_v = v_cursor - (mouse.y - view.y) / (z_new * view.h);
    app.race.radar_2d_zoom = z_new;
    race_2d_clamp_pan(app);
}

pub(crate) fn draw_race_2d_toggle(app: &AppState) {
    if app.active_tab != AppTab::Race {
        return;
    }
    text_button(
        &app.ui_font,
        race_2d_toggle_rect(),
        "2D Radar Editor",
        app.race.radar_2d,
    );
}

fn race_2d_in_view(view: Rect, sx: f32, sy: f32) -> bool {
    sx >= view.x - 2.0
        && sx <= view.x + view.w + 2.0
        && sy >= view.y - 2.0
        && sy <= view.y + view.h + 2.0
}

fn draw_race_2d_polyline(app: &AppState, view: Rect, pts: &[V3], color: Color, thick: f32) {
    if pts.len() < 2 {
        return;
    }
    for w in pts.windows(2) {
        let (x0, y0) = race_2d_world_to_screen(app, view, w[0].x, w[0].y);
        let (x1, y1) = race_2d_world_to_screen(app, view, w[1].x, w[1].y);
        if !race_2d_in_view(view, x0, y0) && !race_2d_in_view(view, x1, y1) {
            continue;
        }
        draw_line(x0, y0, x1, y1, thick, color);
    }
}

fn draw_race_2d_handles(
    app: &AppState,
    view: Rect,
    pts: &[V3],
    color: Color,
    active: bool,
    sel: Option<usize>,
) {
    for (i, p) in pts.iter().enumerate() {
        let (sx, sy) = race_2d_world_to_screen(app, view, p.x, p.y);
        if !race_2d_in_view(view, sx, sy) {
            continue;
        }
        let selected = active && sel == Some(i);
        let r = if selected { 5.0 } else { 3.0 };
        draw_circle(sx, sy, r, color);
        if selected {
            draw_circle_lines(sx, sy, r + 2.0, 1.5, WHITE);
        }
    }
}

pub(crate) fn draw_race_2d_editor(app: &AppState) {
    if app.active_tab != AppTab::Race || !app.race.radar_2d {
        return;
    }
    let vp = editor_viewport_rect();
    // Cover the 3D scene with a dark backdrop.
    draw_rectangle(vp.x, vp.y, vp.w, vp.h, Color::new(0.04, 0.05, 0.06, 1.0));
    let view = race_2d_view_rect();
    draw_rrect_bordered(
        view.x - 4.0,
        view.y - 4.0,
        view.w + 8.0,
        view.h + 8.0,
        6.0,
        1.0,
        Color::new(0.06, 0.07, 0.09, 1.0),
        ui_border(),
    );

    match app.race.radar_tex.as_ref() {
        Some(tex) => {
            // Draw the zoomed sub-region of the radar via a source sub-rect.
            let z = app.race.radar_2d_zoom.max(1.0);
            let win = 1.0 / z;
            let tw = tex.width();
            let th = tex.height();
            draw_texture_ex(
                tex,
                view.x,
                view.y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(view.w, view.h)),
                    source: Some(Rect::new(
                        app.race.radar_2d_pan_u * tw,
                        app.race.radar_2d_pan_v * th,
                        win * tw,
                        win * th,
                    )),
                    ..Default::default()
                },
            );
        }
        None => {
            ui_text(
                &app.ui_font,
                "No radar image set — use Browse Radar in the Track panel",
                view.x + 16.0,
                view.y + view.h * 0.5,
                ui_dim(),
            );
        }
    }

    ui_text(
        &app.ui_font,
        &format!(
            "2D Radar Editor — outline points  |  zoom {:.1}x  (scroll to zoom, click to add/move, checkpoints are locked)",
            app.race.radar_2d_zoom.max(1.0)
        ),
        view.x + 2.0,
        view.y - 12.0,
        Color::new(0.72, 0.86, 1.0, 1.0),
    );

    let Some(track) = app.race.tracks.get(app.race.selected_track) else {
        ui_text(
            &app.ui_font,
            "Select a track on the left first",
            view.x + 16.0,
            view.y + 24.0,
            ui_dim(),
        );
        return;
    };

    let cp_sel = if app.race.place_mode == RacePlaceMode::Checkpoint {
        app.race.selected_point
    } else {
        None
    };
    let overlay_sel = if app.race.place_mode == RacePlaceMode::Overlay {
        app.race.selected_point
    } else {
        None
    };

    // --- Static reference: checkpoints (locked) + their route line. ---
    let cp_pts: Vec<V3> = track.checkpoints.iter().map(|c| c.pos).collect();
    draw_race_2d_polyline(app, view, &cp_pts, Color::new(1.0, 0.6, 0.15, 0.5), 1.5);
    for (idx, c) in track.checkpoints.iter().enumerate() {
        let (sx, sy) = race_2d_world_to_screen(app, view, c.pos.x, c.pos.y);
        if !race_2d_in_view(view, sx, sy) {
            continue;
        }
        let selected = cp_sel == Some(idx);
        let r_px = (c.r / app.race.world_size * z_scale(app) * view.w).max(3.0);
        let col = if selected {
            Color::new(1.0, 1.0, 0.25, 1.0)
        } else {
            Color::new(1.0, 0.62, 0.1, 0.85)
        };
        draw_circle_lines(sx, sy, r_px, if selected { 3.0 } else { 1.5 }, col);
        draw_circle(sx, sy, if selected { 5.0 } else { 3.0 }, col);
        if selected {
            draw_circle_lines(sx, sy, 9.0, 2.0, WHITE);
        }
    }

    // --- Editable radar outline (overlay) points. ---
    draw_race_2d_polyline(
        app,
        view,
        &track.overlay,
        Color::new(0.2, 1.0, 0.45, 0.9),
        2.0,
    );
    draw_race_2d_handles(
        app,
        view,
        &track.overlay,
        Color::new(0.25, 1.0, 0.5, 1.0),
        true,
        overlay_sel,
    );

    // Start marker (reference only, locked here).
    let s = track.start;
    if s != V3::default() {
        let (sx, sy) = race_2d_world_to_screen(app, view, s.x, s.y);
        if race_2d_in_view(view, sx, sy) {
            let col = Color::new(1.0, 0.2, 0.85, 0.9);
            draw_line(sx - 7.0, sy, sx + 7.0, sy, 2.0, col);
            draw_line(sx, sy - 7.0, sx, sy + 7.0, 2.0, col);
            draw_circle_lines(sx, sy, 8.0, 1.5, col);
        }
    }
}

/// Zoom scale used to size on-screen radii in the 2D editor.
fn z_scale(app: &AppState) -> f32 {
    app.race.radar_2d_zoom.max(1.0)
}

/// Moves the currently-selected overlay point to a new world XY (keeps Z).
/// The 2D radar editor only ever edits overlay (outline) points; checkpoints
/// and the start marker are locked reference geometry.
fn move_selected_overlay_point(app: &mut AppState, wx: f32, wy: f32) {
    let idx = app.race.selected_track;
    let Some(sel) = app.race.selected_point else {
        return;
    };
    if let Some(track) = app.race.tracks.get_mut(idx)
        && let Some(p) = track.overlay.get_mut(sel)
    {
        p.x = wx;
        p.y = wy;
    }
}

fn move_selected_checkpoint_2d(app: &mut AppState, wx: f32, wy: f32) {
    let idx = app.race.selected_track;
    let Some(sel) = app.race.selected_point else {
        return;
    };
    if let Some(track) = app.race.tracks.get_mut(idx)
        && let Some(cp) = track.checkpoints.get_mut(sel)
    {
        cp.pos.x = wx;
        cp.pos.y = wy;
    }
}

/// Picks the nearest checkpoint to the cursor. Used when the 2D radar editor is
/// open in Checkpoint mode so the right-panel reorder controls can target it.
fn pick_checkpoint_2d(app: &mut AppState, view: Rect, mouse: Vec2) -> bool {
    let track = match app.race.tracks.get(app.race.selected_track) {
        Some(t) => t,
        None => return false,
    };
    let mut best: Option<usize> = None;
    let mut best_dist = 14.0_f32;
    for (i, cp) in track.checkpoints.iter().enumerate() {
        let (sx, sy) = race_2d_world_to_screen(app, view, cp.pos.x, cp.pos.y);
        let d = mouse.distance(vec2(sx, sy));
        if d < best_dist {
            best_dist = d;
            best = Some(i);
        }
    }
    if let Some(i) = best {
        app.race.selected_point = Some(i);
        app.status_message = format!("Selected checkpoint {}", i + 1);
        true
    } else {
        false
    }
}

/// Picks the nearest overlay point to the cursor. Overlay points remain the only
/// directly draggable points in the 2D radar editor.
fn pick_overlay_point_2d(app: &mut AppState, view: Rect, mouse: Vec2) -> bool {
    let track = match app.race.tracks.get(app.race.selected_track) {
        Some(t) => t,
        None => return false,
    };
    let mut best: Option<usize> = None;
    let mut best_dist = 12.0_f32;
    for (i, p) in track.overlay.iter().enumerate() {
        let (sx, sy) = race_2d_world_to_screen(app, view, p.x, p.y);
        let d = mouse.distance(vec2(sx, sy));
        if d < best_dist {
            best_dist = d;
            best = Some(i);
        }
    }
    if let Some(i) = best {
        app.race.selected_point = Some(i);
        true
    } else {
        false
    }
}

fn selected_overlay_drag_start(app: &AppState) -> Option<Race2dPointDrag> {
    let track_idx = app.race.selected_track;
    let point_idx = app.race.selected_point?;
    let before = *app.race.tracks.get(track_idx)?.overlay.get(point_idx)?;
    Some(Race2dPointDrag::Overlay {
        track: track_idx,
        point: point_idx,
        before,
    })
}

fn selected_checkpoint_drag_start(app: &AppState) -> Option<Race2dPointDrag> {
    let track_idx = app.race.selected_track;
    let point_idx = app.race.selected_point?;
    let before = *app.race.tracks.get(track_idx)?.checkpoints.get(point_idx)?;
    Some(Race2dPointDrag::Checkpoint {
        track: track_idx,
        point: point_idx,
        before,
    })
}

fn insert_checkpoint_2d(app: &mut AppState, view: Rect, mouse: Vec2, wx: f32, wy: f32) {
    let idx = app.race.selected_track;
    let checkpoints: Vec<RaceCheckpoint> = match app.race.tracks.get(idx) {
        Some(t) => t.checkpoints.clone(),
        None => {
            app.status_message = "Create a track first (New Track)".to_string();
            return;
        }
    };
    let mut insert_at = checkpoints.len();
    let mut z = app.race.place_z;
    if checkpoints.len() >= 2 {
        let mut best_seg = 0usize;
        let mut best_dist = f32::MAX;
        for i in 0..(checkpoints.len() - 1) {
            let a = checkpoints[i].pos;
            let b = checkpoints[i + 1].pos;
            let (ax, ay) = race_2d_world_to_screen(app, view, a.x, a.y);
            let (bx, by) = race_2d_world_to_screen(app, view, b.x, b.y);
            let d = dist_to_segment(mouse, vec2(ax, ay), vec2(bx, by));
            if d < best_dist {
                best_dist = d;
                best_seg = i;
            }
        }
        if best_dist < 40.0 {
            insert_at = best_seg + 1;
            z = (checkpoints[best_seg].pos.z + checkpoints[best_seg + 1].pos.z) * 0.5;
        }
    }
    if let Some(track) = app.race.tracks.get_mut(idx) {
        let at = insert_at.min(track.checkpoints.len());
        track.checkpoints.insert(
            at,
            RaceCheckpoint {
                pos: V3 { x: wx, y: wy, z },
                r: app.race.default_radius,
            },
        );
        app.race.selected_point = Some(at);
        app.status_message = format!("Added checkpoint {} at Z {:.1}", at + 1, z);
    }
}

/// Inserts a new overlay point. If the cursor is near an existing overlay
/// segment, the point is inserted *into* that segment so the outline is
/// refined between existing points; otherwise it is appended to the end.
fn insert_overlay_point_2d(app: &mut AppState, view: Rect, mouse: Vec2, wx: f32, wy: f32) {
    let z = app.race.place_z;
    let idx = app.race.selected_track;
    let overlay: Vec<V3> = match app.race.tracks.get(idx) {
        Some(t) => t.overlay.clone(),
        None => {
            app.status_message = "Create a track first (New Track)".to_string();
            return;
        }
    };
    let hit = V3 { x: wx, y: wy, z };
    let n = overlay.len();
    // Find the nearest outline segment in screen space. The outline is treated
    // as a closed loop, so the final segment wraps back to the first point.
    let insert_at = if n >= 2 {
        let mut best_seg = 0usize;
        let mut best_dist = f32::MAX;
        for i in 0..n {
            let a = overlay[i];
            let b = overlay[(i + 1) % n];
            let (ax, ay) = race_2d_world_to_screen(app, view, a.x, a.y);
            let (bx, by) = race_2d_world_to_screen(app, view, b.x, b.y);
            let d = dist_to_segment(mouse, vec2(ax, ay), vec2(bx, by));
            if d < best_dist {
                best_dist = d;
                best_seg = i;
            }
        }
        if best_dist < 40.0 { best_seg + 1 } else { n }
    } else {
        n
    };
    if let Some(t) = app.race.tracks.get_mut(idx) {
        let at = insert_at.min(t.overlay.len());
        t.overlay.insert(at, hit);
        app.race.selected_point = Some(at);
    }
    app.status_message = format!("Added outline point at {:.0}, {:.0}", wx, wy);
}

/// Deletes the currently-selected overlay point, if any.
fn delete_selected_overlay_point(app: &mut AppState) {
    let idx = app.race.selected_track;
    let Some(sel) = app.race.selected_point else {
        return;
    };
    if let Some(t) = app.race.tracks.get_mut(idx)
        && sel < t.overlay.len()
    {
        t.overlay.remove(sel);
        app.race.selected_point = None;
        app.status_message = "Removed outline point".to_string();
    }
}

/// Handles the 2D radar editor toggle button and all pointer interaction within
/// the 2D editor. Returns true when the interaction was consumed (so the caller
/// skips 3D viewport / camera handling).
pub(crate) fn handle_race_2d(app: &mut AppState, mouse: Vec2) -> bool {
    if app.active_tab != AppTab::Race {
        return false;
    }
    // Toggle button (works whether the editor is on or off).
    if is_mouse_button_pressed(MouseButton::Left) && race_2d_toggle_rect().contains(mouse) {
        app.race.radar_2d = !app.race.radar_2d;
        app.race.radar_2d_dragging = false;
        app.race.radar_2d_drag_before = None;
        if app.race.radar_2d {
            app.status_message =
                "2D radar editor on — checkpoints are visible; click to edit outline points"
                    .to_string();
        } else {
            app.status_message = "2D radar editor off".to_string();
        }
        return true;
    }
    if !app.race.radar_2d {
        return false;
    }

    if is_mouse_button_released(MouseButton::Left) {
        app.race.radar_2d_dragging = false;
        if let Some(before) = app.race.radar_2d_drag_before.take() {
            commit_race_2d_point_move(app, "Move Race Point", before);
        }
    }
    if is_mouse_button_released(MouseButton::Middle) {
        app.race.radar_2d_panning = false;
    }

    let vp = editor_viewport_rect();
    let view = race_2d_view_rect();

    // Hold middle mouse to pan the zoomed radar image around (grab-style drag).
    if is_mouse_button_pressed(MouseButton::Middle) && view.contains(mouse) {
        app.race.radar_2d_panning = true;
        app.race.radar_2d_pan_last = mouse;
        return true;
    }
    if app.race.radar_2d_panning && is_mouse_button_down(MouseButton::Middle) {
        let delta = mouse - app.race.radar_2d_pan_last;
        app.race.radar_2d_pan_last = mouse;
        let z = app.race.radar_2d_zoom.max(1.0);
        app.race.radar_2d_pan_u -= delta.x / (z * view.w);
        app.race.radar_2d_pan_v -= delta.y / (z * view.h);
        race_2d_clamp_pan(app);
        return true;
    }

    // Continue an in-progress drag anywhere on screen.
    if app.race.radar_2d_dragging && is_mouse_button_down(MouseButton::Left) {
        let (wx, wy) = race_2d_screen_to_world(app, view, mouse);
        match app.race.radar_2d_drag_before {
            Some(Race2dPointDrag::Checkpoint { .. }) => move_selected_checkpoint_2d(app, wx, wy),
            Some(Race2dPointDrag::Overlay { .. }) | None => {
                move_selected_overlay_point(app, wx, wy)
            }
        }
        return true;
    }

    if !vp.contains(mouse) {
        // Let side-panel clicks (New Track, modes, etc.) pass through.
        return false;
    }

    // Right-click removes the outline point under the cursor.
    if is_mouse_button_pressed(MouseButton::Right) {
        if view.contains(mouse) && pick_overlay_point_2d(app, view, mouse) {
            app.race.place_mode = RacePlaceMode::Overlay;
            let before = race_history_snapshot(app);
            delete_selected_overlay_point(app);
            commit_race_history(app, "Delete Race Outline Point", before);
        }
        return true;
    }

    if is_mouse_button_pressed(MouseButton::Left) {
        if view.contains(mouse) {
            if app.race.place_mode == RacePlaceMode::Checkpoint {
                if pick_checkpoint_2d(app, view, mouse) {
                    app.race.radar_2d_drag_before = selected_checkpoint_drag_start(app);
                    app.race.radar_2d_dragging = true;
                    return true;
                }
                let (wx, wy) = race_2d_screen_to_world(app, view, mouse);
                let before = race_history_snapshot(app);
                insert_checkpoint_2d(app, view, mouse, wx, wy);
                commit_race_history(app, "Add Race Checkpoint", before);
                app.race.radar_2d_drag_before = selected_checkpoint_drag_start(app);
                app.race.radar_2d_dragging = true;
                return true;
            }
            // Grab an existing outline point, else insert a new one on the
            // nearest segment (checkpoints stay locked as reference geometry).
            if pick_overlay_point_2d(app, view, mouse) {
                app.race.place_mode = RacePlaceMode::Overlay;
                app.race.radar_2d_drag_before = selected_overlay_drag_start(app);
                app.race.radar_2d_dragging = true;
                return true;
            }
            let (wx, wy) = race_2d_screen_to_world(app, view, mouse);
            let before = race_history_snapshot(app);
            app.race.place_mode = RacePlaceMode::Overlay;
            insert_overlay_point_2d(app, view, mouse, wx, wy);
            commit_race_history(app, "Add Race Outline Point", before);
            app.race.radar_2d_drag_before = selected_overlay_drag_start(app);
            app.race.radar_2d_dragging = true;
            return true;
        }
        // Swallow other viewport clicks so the hidden 3D scene isn't disturbed.
        return true;
    }
    // Block camera look / other viewport interactions while the 2D editor is up.
    is_mouse_button_down(MouseButton::Left) || is_mouse_button_down(MouseButton::Right)
}
