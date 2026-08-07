#![cfg_attr(windows, allow(unreachable_code))]

use super::super::*;

pub(crate) fn project_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.replace('_', " "))
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

// Shared vertical rhythm for the launcher so the draw code and the hit-test
// helpers stay in agreement. All values are screen-space Y coordinates.
pub(crate) const PICK_HERO_Y: f32 = 56.0;
pub(crate) const PICK_HERO_H: f32 = 140.0;
pub(crate) const PICK_BROWSE_Y: f32 = PICK_HERO_Y + PICK_HERO_H + 26.0; // 222
pub(crate) const PICK_BROWSE_H: f32 = 42.0;
pub(crate) const PICK_STATUS_Y: f32 = PICK_BROWSE_Y + PICK_BROWSE_H + 22.0; // 286
pub(crate) const PICK_HEADER_Y: f32 = PICK_STATUS_Y + 36.0; // 322
pub(crate) const PICK_GRID_Y: f32 = PICK_HEADER_Y + 30.0; // 352

// The launcher shows project creation, browse, and editor-only actions together.
const LAUNCH_BTN_GAP: f32 = 18.0;
const SHOW_NEW_PROJECT: bool = true;
const PROJECT_CARD_H: f32 = 150.0;
const PROJECT_CARD_GAP: f32 = 22.0;

pub(crate) fn project_grid_cols() -> usize {
    if screen_width() >= 1280.0 {
        4
    } else if screen_width() >= 1024.0 {
        3
    } else if screen_width() >= 680.0 {
        2
    } else {
        1
    }
}

pub(crate) fn project_card_w() -> f32 {
    let cols = project_grid_cols();
    let usable_w = (screen_width() - 88.0).max(360.0);
    let card_w = (usable_w - (cols as f32 - 1.0) * PROJECT_CARD_GAP) / cols as f32;
    card_w.clamp(248.0, 320.0)
}

pub(crate) fn project_grid_start_x() -> f32 {
    let cols = project_grid_cols();
    let card_w = project_card_w();
    let total_w = cols as f32 * card_w + (cols - 1) as f32 * PROJECT_CARD_GAP;
    ((screen_width() - total_w) * 0.5).max(44.0)
}

pub(crate) fn project_card_rect(index: usize) -> Rect {
    let cols = project_grid_cols();
    let card_w = project_card_w();
    let start_x = project_grid_start_x();
    let row = index / cols;
    let col = index % cols;
    Rect::new(
        start_x + col as f32 * (card_w + PROJECT_CARD_GAP),
        PICK_GRID_Y + row as f32 * (PROJECT_CARD_H + PROJECT_CARD_GAP),
        card_w,
        PROJECT_CARD_H,
    )
}

fn project_visible_rows() -> usize {
    ((screen_height() - PICK_GRID_Y - 28.0 + PROJECT_CARD_GAP)
        / (PROJECT_CARD_H + PROJECT_CARD_GAP))
        .floor()
        .max(1.0) as usize
}

fn project_total_rows(project_count: usize) -> usize {
    project_count.div_ceil(project_grid_cols())
}

fn project_max_scroll_row(project_count: usize) -> usize {
    project_total_rows(project_count).saturating_sub(project_visible_rows())
}

fn project_scroll_track_rect() -> Rect {
    Rect::new(
        screen_width() - 24.0,
        PICK_GRID_Y,
        8.0,
        (screen_height() - PICK_GRID_Y - 24.0).max(40.0),
    )
}

fn project_scroll_thumb_rect(picker: &ProjectPicker) -> Option<Rect> {
    let total_rows = project_total_rows(picker.projects.len());
    let visible_rows = project_visible_rows();
    if total_rows <= visible_rows {
        return None;
    }
    let track = project_scroll_track_rect();
    let thumb_h = (track.h * visible_rows as f32 / total_rows as f32).clamp(28.0, track.h);
    let max_scroll = project_max_scroll_row(picker.projects.len()).max(1);
    let thumb_y =
        track.y + (track.h - thumb_h) * picker.project_scroll_row as f32 / max_scroll as f32;
    Some(Rect::new(track.x, thumb_y, track.w, thumb_h))
}

fn project_visible_card_rect(slot: usize) -> Rect {
    project_card_rect(slot)
}

fn launch_button_w() -> f32 {
    let button_count = if SHOW_NEW_PROJECT { 4.0 } else { 3.0 };
    let available = (screen_width() - 88.0 - LAUNCH_BTN_GAP * (button_count - 1.0)).max(320.0);
    (available / button_count).min(190.0)
}

fn launch_row_start_x() -> f32 {
    let button_count = if SHOW_NEW_PROJECT { 4.0 } else { 3.0 };
    let total_w = launch_button_w() * button_count + LAUNCH_BTN_GAP * (button_count - 1.0);
    screen_width() * 0.5 - total_w * 0.5
}

pub(crate) fn project_new_rect() -> Rect {
    Rect::new(
        launch_row_start_x(),
        PICK_BROWSE_Y,
        launch_button_w(),
        PICK_BROWSE_H,
    )
}

pub(crate) fn project_browse_rect() -> Rect {
    Rect::new(
        launch_row_start_x()
            + if SHOW_NEW_PROJECT {
                launch_button_w() + LAUNCH_BTN_GAP
            } else {
                0.0
            },
        PICK_BROWSE_Y,
        launch_button_w(),
        PICK_BROWSE_H,
    )
}

pub(crate) fn project_roots_rect() -> Rect {
    Rect::new(
        project_browse_rect().x + launch_button_w() + LAUNCH_BTN_GAP,
        PICK_BROWSE_Y,
        launch_button_w(),
        PICK_BROWSE_H,
    )
}

pub(crate) fn project_open_editor_rect() -> Rect {
    Rect::new(
        project_roots_rect().x + launch_button_w() + LAUNCH_BTN_GAP,
        PICK_BROWSE_Y,
        launch_button_w(),
        PICK_BROWSE_H,
    )
}

pub(crate) fn project_thumb_color(path: &Path) -> Color {
    let mut hash = 0u32;
    for byte in path.to_string_lossy().bytes() {
        hash = hash.wrapping_mul(37).wrapping_add(byte as u32);
    }
    let r = 0.16 + ((hash & 0xff) as f32 / 255.0) * 0.16;
    let g = 0.18 + (((hash >> 8) & 0xff) as f32 / 255.0) * 0.18;
    let b = 0.22 + (((hash >> 16) & 0xff) as f32 / 255.0) * 0.20;
    Color::new(r, g, b, 1.0)
}

pub(crate) fn new_project_picker(options: Options, ui_font: Font, icons: IconSet) -> ProjectPicker {
    let project_roots = load_project_roots();
    let mut picker = ProjectPicker {
        options,
        ui_font,
        icons,
        projects: Vec::new(),
        project_roots,
        thumbnails: HashMap::new(),
        project_discovery_rx: None,
        picker_rx: None,
        project_root_picker_rx: None,
        new_root_picker_rx: None,
        new_project_dialog: None,
        project_roots_dialog: false,
        project_scroll_row: 0,
        status: "Create a project, choose a recent one, browse, or open the editor.".to_string(),
    };
    start_project_discovery(&mut picker);
    picker
}

fn discover_projects(root: &Path) -> Vec<PathBuf> {
    if !root.is_dir() {
        return Vec::new();
    }
    WalkDir::new(root)
        .follow_links(false)
        .max_depth(8)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_dir())
        .map(|entry| entry.into_path())
        .filter(|path| is_eagle_resource(path))
        .collect()
}

fn start_project_discovery(picker: &mut ProjectPicker) {
    let project_roots = picker.project_roots.clone();
    let (tx, rx) = mpsc::channel();
    picker.project_discovery_rx = Some(rx);
    picker.status = if project_roots.is_empty() {
        "Loading recent projects in the background...".to_string()
    } else {
        format!(
            "Discovering projects across {} Project Root{} in the background...",
            project_roots.len(),
            if project_roots.len() == 1 { "" } else { "s" }
        )
    };
    thread::spawn(move || {
        let mut projects = load_recent_projects();
        let total = project_roots.len();
        for (index, root) in project_roots.into_iter().enumerate() {
            if tx
                .send(ProjectDiscoveryUpdate::Scanning {
                    index: index + 1,
                    total,
                    root: root.clone(),
                })
                .is_err()
            {
                return;
            }
            for path in discover_projects(&root) {
                if !projects
                    .iter()
                    .any(|existing| same_resource_path(existing, &path))
                {
                    projects.push(path);
                }
            }
        }
        let _ = tx.send(ProjectDiscoveryUpdate::Finished(projects));
    });
}

fn poll_project_discovery(picker: &mut ProjectPicker) {
    let mut finished = None;
    let mut disconnected = false;
    let Some(rx) = picker.project_discovery_rx.as_ref() else {
        return;
    };
    loop {
        match rx.try_recv() {
            Ok(ProjectDiscoveryUpdate::Scanning { index, total, root }) => {
                picker.status = format!(
                    "Scanning Project Root {index}/{total}: {}",
                    ellipsize(root.to_string_lossy().as_ref(), 64)
                );
            }
            Ok(ProjectDiscoveryUpdate::Finished(projects)) => {
                finished = Some(projects);
                break;
            }
            Err(mpsc::TryRecvError::Empty) => break,
            Err(mpsc::TryRecvError::Disconnected) => {
                disconnected = true;
                break;
            }
        }
    }
    if let Some(projects) = finished {
        picker.projects = projects;
        picker.project_scroll_row = picker
            .project_scroll_row
            .min(project_max_scroll_row(picker.projects.len()));
        picker.project_discovery_rx = None;
        picker.status = format!(
            "Found {} project{}. Create one, browse, or open the editor.",
            picker.projects.len(),
            if picker.projects.len() == 1 { "" } else { "s" }
        );
    } else if disconnected {
        picker.project_discovery_rx = None;
        picker.status = "Project discovery stopped unexpectedly.".to_string();
    }
}

fn project_roots_dialog_rect() -> Rect {
    let w = 700.0_f32.min(screen_width() - 80.0);
    Rect::new(
        (screen_width() - w) * 0.5,
        screen_height() * 0.5 - 210.0,
        w,
        420.0,
    )
}

fn project_roots_add_rect() -> Rect {
    let rect = project_roots_dialog_rect();
    Rect::new(rect.x + 24.0, rect.y + rect.h - 52.0, 112.0, 32.0)
}

fn project_roots_close_rect() -> Rect {
    let rect = project_roots_dialog_rect();
    Rect::new(rect.x + rect.w - 112.0, rect.y + rect.h - 52.0, 88.0, 32.0)
}

fn project_root_remove_rect(index: usize) -> Rect {
    let rect = project_roots_dialog_rect();
    Rect::new(
        rect.x + rect.w - 104.0,
        rect.y + 86.0 + index as f32 * 34.0,
        80.0,
        28.0,
    )
}

fn choose_project_root(start: PathBuf) -> Result<Option<PathBuf>, String> {
    choose_project_parent_folder(start)
}

fn project_picker_open_roots_dialog(picker: &mut ProjectPicker) {
    picker.project_roots_dialog = true;
    picker.status = "Add folders to automatically discover Eagle map projects.".to_string();
}

fn project_picker_add_root(picker: &mut ProjectPicker) {
    if picker.project_root_picker_rx.is_some() {
        picker.status = "Project root browser is already open.".to_string();
        return;
    }
    let start = picker
        .project_roots
        .first()
        .cloned()
        .unwrap_or_else(|| PathBuf::from(BROWSE_ROOT));
    let (tx, rx) = mpsc::channel();
    picker.project_root_picker_rx = Some(rx);
    picker.status = "Choose a folder containing map projects...".to_string();
    thread::spawn(move || {
        let _ = tx.send(choose_project_root(start));
    });
}

pub(crate) fn new_project_dialog_rect() -> Rect {
    let w = 660.0_f32.min(screen_width() - 80.0);
    Rect::new(
        (screen_width() - w) * 0.5,
        screen_height() * 0.5 - 150.0,
        w,
        300.0,
    )
}

pub(crate) fn new_project_root_rect() -> Rect {
    let dialog = new_project_dialog_rect();
    Rect::new(dialog.x + 24.0, dialog.y + 82.0, dialog.w - 160.0, 34.0)
}

pub(crate) fn new_project_browse_root_rect() -> Rect {
    let dialog = new_project_dialog_rect();
    Rect::new(dialog.x + dialog.w - 124.0, dialog.y + 82.0, 100.0, 34.0)
}

pub(crate) fn new_project_name_rect() -> Rect {
    let dialog = new_project_dialog_rect();
    Rect::new(dialog.x + 24.0, dialog.y + 150.0, dialog.w - 48.0, 34.0)
}

pub(crate) fn new_project_create_rect() -> Rect {
    let dialog = new_project_dialog_rect();
    Rect::new(
        dialog.x + dialog.w - 216.0,
        dialog.y + dialog.h - 50.0,
        88.0,
        32.0,
    )
}

pub(crate) fn new_project_cancel_rect() -> Rect {
    let dialog = new_project_dialog_rect();
    Rect::new(
        dialog.x + dialog.w - 116.0,
        dialog.y + dialog.h - 50.0,
        88.0,
        32.0,
    )
}

pub(crate) fn project_picker_open_new_dialog(picker: &mut ProjectPicker) {
    drain_text_input();
    let root = if Path::new(BROWSE_ROOT).is_dir() {
        BROWSE_ROOT.to_string()
    } else {
        env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .to_string_lossy()
            .to_string()
    };
    let name = "New Project".to_string();
    picker.new_project_dialog = Some(NewProjectDialog {
        root,
        cursor: name.len(),
        selection_anchor: Some(0),
        name,
    });
    picker.status = "Choose a project root and enter a project name.".to_string();
}

pub(crate) fn choose_project_parent_folder(start: PathBuf) -> Result<Option<PathBuf>, String> {
    choose_folder_with_title(start, "Choose New Project Root")
}

pub(crate) fn choose_folder_with_title(
    start: PathBuf,
    title: &str,
) -> Result<Option<PathBuf>, String> {
    #[cfg(windows)]
    return windows_pick_folder(title, &start);

    let start = if start.is_dir() {
        start
    } else {
        PathBuf::from(BROWSE_ROOT)
    };
    let mut kdialog = Command::new("kdialog");
    kdialog
        .arg("--title")
        .arg(title)
        .arg("--getexistingdirectory")
        .arg(start.to_string_lossy().to_string());
    match run_folder_picker_command(kdialog) {
        Ok(result) => Ok(result),
        Err(kdialog_err) => {
            let mut zenity = Command::new("zenity");
            zenity
                .arg("--file-selection")
                .arg("--directory")
                .arg(format!("--title={title}"))
                .arg(format!("--filename={}/", start.to_string_lossy()));
            match run_folder_picker_command(zenity) {
                Ok(result) => Ok(result),
                Err(zenity_err) => Err(format!("{kdialog_err}; {zenity_err}")),
            }
        }
    }
}

fn start_gta_sa_folder_picker(start: PathBuf) -> mpsc::Receiver<Result<Option<PathBuf>, String>> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(choose_folder_with_title(start, "Choose GTA:SA Root Folder"));
    });
    rx
}

fn draw_gta_sa_setup(font: &Font, status: &str, choosing: bool) {
    reset_gl_for_ui();
    clear_background(Color::new(0.025, 0.030, 0.040, 1.0));
    let width = 640.0_f32.min(screen_width() - 48.0);
    let panel = Rect::new(
        (screen_width() - width) * 0.5,
        (screen_height() - 330.0) * 0.5,
        width,
        330.0,
    );
    draw_panel_rect(font, panel, Some("GTA: San Andreas Setup"));
    ui_text(
        font,
        "A GTA:SA installation folder is optional.",
        panel.x + 32.0,
        panel.y + 82.0,
        WHITE,
    );
    ui_text(
        font,
        "Choose it for stock game assets, or continue using resource assets only.",
        panel.x + 32.0,
        panel.y + 112.0,
        ui_dim(),
    );
    ui_text(
        font,
        &format!("Eagle verifies the installation using {GTA_SA_MARKER_FILE}."),
        panel.x + 32.0,
        panel.y + 142.0,
        ui_dim(),
    );
    ui_text(
        font,
        status,
        panel.x + 32.0,
        panel.y + 194.0,
        if choosing {
            ui_muted()
        } else {
            Color::new(1.0, 0.62, 0.24, 1.0)
        },
    );
    let choose = Rect::new(panel.x + 32.0, panel.y + 238.0, 240.0, 42.0);
    draw_dialog_button(
        font,
        choose,
        if choosing {
            "Folder picker open..."
        } else {
            "Choose GTA:SA Folder"
        },
        true,
    );
    let skip = Rect::new(panel.x + panel.w - 272.0, panel.y + 238.0, 240.0, 42.0);
    draw_dialog_button(font, skip, "Continue Without GTA:SA", !choosing);
}

pub(crate) async fn ensure_gta_sa_dir_configured(font: &Font) -> bool {
    if load_configured_gta_sa_dir().is_some() || gta_sa_setup_bypassed() {
        return true;
    }

    let mut start = load_gta_sa_dir_candidate();
    if !start.is_dir() {
        start = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    }
    let mut picker_rx: Option<mpsc::Receiver<Result<Option<PathBuf>, String>>> = None;
    let mut status =
        format!("Select the folder containing {GTA_SA_MARKER_FILE}, or continue without it.");

    loop {
        if is_quit_requested() {
            macroquad::miniquad::window::quit();
            return false;
        }

        if let Some(rx) = picker_rx.as_ref() {
            match rx.try_recv() {
                Ok(Ok(Some(path))) => {
                    picker_rx = None;
                    match validate_gta_sa_dir(&path) {
                        Ok(()) => {
                            save_gta_sa_dir_preference(&path);
                            return true;
                        }
                        Err(error) => {
                            start = path;
                            status = error;
                        }
                    }
                }
                Ok(Ok(None)) => {
                    picker_rx = None;
                    status =
                        "Folder selection was cancelled. Choose a folder to continue.".to_string();
                }
                Ok(Err(error)) => {
                    picker_rx = None;
                    status = format!("Could not open the folder picker: {error}");
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    picker_rx = None;
                    status = "The folder picker closed unexpectedly. Try again.".to_string();
                }
            }
        }

        draw_gta_sa_setup(font, &status, picker_rx.is_some());
        let choose = Rect::new(
            (screen_width() - 640.0_f32.min(screen_width() - 48.0)) * 0.5 + 32.0,
            (screen_height() - 330.0) * 0.5 + 238.0,
            240.0,
            42.0,
        );
        let skip = Rect::new(
            (screen_width() - 640.0_f32.min(screen_width() - 48.0)) * 0.5 + 368.0,
            (screen_height() - 330.0) * 0.5 + 238.0,
            240.0,
            42.0,
        );
        if picker_rx.is_none()
            && is_mouse_button_pressed(MouseButton::Left)
            && skip.contains(mouse_position().into())
        {
            save_gta_sa_setup_bypassed();
            return true;
        }
        if picker_rx.is_none()
            && is_mouse_button_pressed(MouseButton::Left)
            && choose.contains(mouse_position().into())
        {
            picker_rx = Some(start_gta_sa_folder_picker(start.clone()));
            status = "Choose your GTA:SA installation folder...".to_string();
        }
        macroquad::miniquad::window::schedule_update();
        next_frame().await;
    }
}

pub(crate) async fn show_graphics_startup_error(font: &Font, error: &str) {
    loop {
        clear_background(Color::new(0.025, 0.030, 0.040, 1.0));
        let width = 760.0_f32.min(screen_width() - 48.0);
        let panel = Rect::new(
            (screen_width() - width) * 0.5,
            (screen_height() - 300.0) * 0.5,
            width,
            300.0,
        );
        draw_panel_rect(font, panel, Some("OpenGL Compatibility Error"));
        ui_text(
            font,
            "Eagle Editor could not start its legacy-compatible 3D renderer.",
            panel.x + 32.0,
            panel.y + 82.0,
            Color::new(1.0, 0.50, 0.42, 1.0),
        );
        let mut y = panel.y + 120.0;
        for line in wrap_text_width(error, 15, panel.w - 64.0) {
            ui_text(font, &line, panel.x + 32.0, y, ui_dim());
            y += 24.0;
        }
        ui_text(
            font,
            "Update the graphics driver or report this message with your GPU model.",
            panel.x + 32.0,
            panel.y + 246.0,
            ui_muted(),
        );
        if is_quit_requested() || is_key_pressed(KeyCode::Escape) {
            macroquad::miniquad::window::quit();
            return;
        }
        macroquad::miniquad::window::schedule_update();
        next_frame().await;
    }
}

pub(crate) fn project_picker_open_new_root_browser(picker: &mut ProjectPicker) {
    if picker.new_root_picker_rx.is_some() {
        picker.status = "Project root browser is already open.".to_string();
        return;
    }
    let start = picker
        .new_project_dialog
        .as_ref()
        .map(|dialog| PathBuf::from(dialog.root.trim()))
        .unwrap_or_else(|| PathBuf::from(BROWSE_ROOT));
    let (tx, rx) = mpsc::channel();
    picker.new_root_picker_rx = Some(rx);
    picker.status = "Choose the parent folder for the new project...".to_string();
    thread::spawn(move || {
        let _ = tx.send(choose_project_parent_folder(start));
    });
}

fn xml_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub(crate) fn create_eagle_project(parent: &Path, name: &str) -> Result<PathBuf, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Project name cannot be empty".to_string());
    }
    if matches!(name, "." | "..") || name.contains(['/', '\\', '\0']) {
        return Err("Project name cannot contain path separators".to_string());
    }
    if !parent.is_dir() {
        return Err(format!("Project root does not exist: {}", parent.display()));
    }
    let project = parent.join(name);
    if project.exists()
        && fs::read_dir(&project)
            .map_err(|err| format!("{}: {err}", project.display()))?
            .next()
            .is_some()
    {
        return Err(format!(
            "Project folder is not empty: {}",
            project.display()
        ));
    }
    for dir in ["zones", "imgs", "textures", "txd_build"] {
        fs::create_dir_all(project.join(dir))
            .map_err(|err| format!("Could not create {dir}: {err}"))?;
    }
    fs::write(project.join("eagleZones.txt"), "")
        .map_err(|err| format!("Could not create eagleZones.txt: {err}"))?;
    let meta = format!(
        "<meta>\n    <info type=\"script\" name=\"{}\" author=\"MTA:SA Eagle Edit\" description=\"Eagle map project\" version=\"1.0\"/>\n\n    <file src=\"eagleZones.txt\" type=\"client\" />\n    <file src=\"zones/*/*.definition\" type=\"client\" />\n    <file src=\"zones/*/*.map\" type=\"client\" />\n    <file src=\"textures/*.txd\" type=\"client\" />\n    <file src=\"imgs/*.img\" type=\"client\" />\n</meta>\n",
        xml_text(name)
    );
    fs::write(project.join("meta.xml"), meta)
        .map_err(|err| format!("Could not create meta.xml: {err}"))?;
    Ok(project)
}

/// Bypass project selection and launch straight into the editors for external
/// objects / IMG files. Uses an empty scratch root so the editor starts with no
/// project scene while still having a valid, writable working folder.
pub(crate) fn project_picker_launch_editor(picker: &mut ProjectPicker) -> Option<LoadJob> {
    let mut options = picker.options.clone();
    options.root = editor_scratch_root();
    options.launch_mode = LaunchMode::Editor;
    Some(LoadJob::new(options, Font::default(), picker.icons.clone()))
}

pub(crate) fn project_picker_open_browser(picker: &mut ProjectPicker) {
    if picker.picker_rx.is_some() {
        picker.status = "Folder browser is already open.".to_string();
        return;
    }
    let (tx, rx) = mpsc::channel();
    picker.picker_rx = Some(rx);
    picker.status = "Opening folder browser...".to_string();
    thread::spawn(move || {
        let _ = tx.send(choose_resource_folder());
    });
}

pub(crate) fn project_picker_start_load(
    picker: &mut ProjectPicker,
    path: PathBuf,
) -> Option<LoadJob> {
    if !is_eagle_resource(&path) {
        picker.status = format!(
            "Not an Eagle resource: {}",
            ellipsize(path.to_string_lossy().as_ref(), 64)
        );
        return None;
    }
    let mut options = picker.options.clone();
    options.root = path;
    options.launch_mode = LaunchMode::Project;
    Some(LoadJob::new(options, Font::default(), picker.icons.clone()))
}

pub(crate) fn update_project_picker(picker: &mut ProjectPicker) -> Option<LoadJob> {
    poll_project_discovery(picker);
    if let Some(rx) = picker.project_root_picker_rx.as_ref() {
        match rx.try_recv() {
            Ok(Ok(Some(path))) => {
                picker.project_root_picker_rx = None;
                if picker
                    .project_roots
                    .iter()
                    .any(|existing| same_resource_path(existing, &path))
                {
                    picker.status = "That Project Root has already been added.".to_string();
                } else if picker.project_roots.len() >= 8 {
                    picker.status = "You can add up to eight Project Roots.".to_string();
                } else {
                    picker.project_roots.push(path);
                    save_project_roots(&picker.project_roots);
                    start_project_discovery(picker);
                }
            }
            Ok(Ok(None)) => {
                picker.project_root_picker_rx = None;
                picker.status = "Project Root selection cancelled.".to_string();
            }
            Ok(Err(err)) => {
                picker.project_root_picker_rx = None;
                picker.status = format!("Folder browser failed: {}", ellipsize(&err, 72));
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                picker.project_root_picker_rx = None;
                picker.status = "Project Root browser closed unexpectedly.".to_string();
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }
    if let Some(rx) = picker.new_root_picker_rx.as_ref() {
        match rx.try_recv() {
            Ok(Ok(Some(path))) => {
                picker.new_root_picker_rx = None;
                if let Some(dialog) = picker.new_project_dialog.as_mut() {
                    dialog.root = path.to_string_lossy().to_string();
                }
                picker.status =
                    "Project root selected. Enter a name and create the project.".to_string();
            }
            Ok(Ok(None)) => {
                picker.new_root_picker_rx = None;
                picker.status = "Project root selection cancelled.".to_string();
            }
            Ok(Err(err)) => {
                picker.new_root_picker_rx = None;
                picker.status = format!("Folder browser failed: {}", ellipsize(&err, 72));
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                picker.new_root_picker_rx = None;
                picker.status = "Project root browser closed unexpectedly.".to_string();
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }
    if let Some(rx) = picker.picker_rx.as_ref() {
        match rx.try_recv() {
            Ok(Ok(Some(path))) => {
                picker.picker_rx = None;
                return project_picker_start_load(picker, path);
            }
            Ok(Ok(None)) => {
                picker.picker_rx = None;
                picker.status = "Browse cancelled.".to_string();
            }
            Ok(Err(err)) => {
                picker.picker_rx = None;
                picker.status = format!("Folder browser failed: {}", ellipsize(&err, 72));
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                picker.picker_rx = None;
                picker.status = "Folder browser closed unexpectedly.".to_string();
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }
    if picker.new_project_dialog.is_some() {
        return update_new_project_dialog(picker);
    }
    if picker.project_roots_dialog {
        return update_project_roots_dialog(picker);
    }
    picker.project_scroll_row = picker
        .project_scroll_row
        .min(project_max_scroll_row(picker.projects.len()));
    let mouse: Vec2 = mouse_position().into();
    let wheel = mouse_wheel().1;
    if wheel.abs() > f32::EPSILON && mouse.y >= PICK_HEADER_Y {
        let max_scroll = project_max_scroll_row(picker.projects.len());
        if wheel > 0.0 {
            picker.project_scroll_row = picker.project_scroll_row.saturating_sub(1);
        } else {
            picker.project_scroll_row = (picker.project_scroll_row + 1).min(max_scroll);
        }
    }
    let page_rows = project_visible_rows().max(1);
    if is_key_pressed(KeyCode::PageUp) {
        picker.project_scroll_row = picker.project_scroll_row.saturating_sub(page_rows);
    }
    if is_key_pressed(KeyCode::PageDown) {
        picker.project_scroll_row = (picker.project_scroll_row + page_rows)
            .min(project_max_scroll_row(picker.projects.len()));
    }
    if is_mouse_button_pressed(MouseButton::Left) {
        if SHOW_NEW_PROJECT && project_new_rect().contains(mouse) {
            project_picker_open_new_dialog(picker);
            return None;
        }
        if project_open_editor_rect().contains(mouse) {
            return project_picker_launch_editor(picker);
        }
        if project_browse_rect().contains(mouse) {
            project_picker_open_browser(picker);
            return None;
        }
        if project_roots_rect().contains(mouse) {
            project_picker_open_roots_dialog(picker);
            return None;
        }
        let scroll_track = project_scroll_track_rect();
        if project_scroll_thumb_rect(picker).is_some() && scroll_track.contains(mouse) {
            let max_scroll = project_max_scroll_row(picker.projects.len());
            let t = ((mouse.y - scroll_track.y) / scroll_track.h).clamp(0.0, 1.0);
            picker.project_scroll_row = (t * max_scroll as f32).round() as usize;
            return None;
        }
        let cols = project_grid_cols();
        let first = picker.project_scroll_row * cols;
        let visible_count = project_visible_rows() * cols;
        for (slot, path) in picker
            .projects
            .clone()
            .into_iter()
            .skip(first)
            .take(visible_count)
            .enumerate()
        {
            if project_visible_card_rect(slot).contains(mouse) {
                return project_picker_start_load(picker, path);
            }
        }
    }
    None
}

fn update_project_roots_dialog(picker: &mut ProjectPicker) -> Option<LoadJob> {
    let mouse: Vec2 = mouse_position().into();
    let dialog = project_roots_dialog_rect();
    if is_key_pressed(KeyCode::Escape)
        || (is_mouse_button_pressed(MouseButton::Left) && !dialog.contains(mouse))
    {
        picker.project_roots_dialog = false;
        return None;
    }
    if !is_mouse_button_pressed(MouseButton::Left) {
        return None;
    }
    if project_roots_add_rect().contains(mouse) {
        project_picker_add_root(picker);
        return None;
    }
    if project_roots_close_rect().contains(mouse) {
        picker.project_roots_dialog = false;
        return None;
    }
    for index in 0..picker.project_roots.len() {
        if project_root_remove_rect(index).contains(mouse) {
            picker.project_roots.remove(index);
            save_project_roots(&picker.project_roots);
            start_project_discovery(picker);
            return None;
        }
    }
    None
}

fn create_new_project_from_dialog(picker: &mut ProjectPicker) -> Option<LoadJob> {
    let Some(dialog) = picker.new_project_dialog.take() else {
        return None;
    };
    match create_eagle_project(Path::new(dialog.root.trim()), &dialog.name) {
        Ok(path) => {
            picker.status = format!("Created {}", path.display());
            project_picker_start_load(picker, path)
        }
        Err(err) => {
            picker.status = format!("Could not create project: {}", ellipsize(&err, 84));
            picker.new_project_dialog = Some(dialog);
            None
        }
    }
}

pub(crate) fn update_new_project_dialog(picker: &mut ProjectPicker) -> Option<LoadJob> {
    let mouse: Vec2 = mouse_position().into();
    let dialog_rect = new_project_dialog_rect();
    if is_mouse_button_pressed(MouseButton::Left) {
        if new_project_browse_root_rect().contains(mouse) {
            project_picker_open_new_root_browser(picker);
            return None;
        }
        if new_project_name_rect().contains(mouse) {
            if let Some(dialog) = picker.new_project_dialog.as_mut() {
                dialog.cursor = dialog.name.len();
                dialog.selection_anchor = None;
            }
            return None;
        }
        if new_project_create_rect().contains(mouse) {
            return create_new_project_from_dialog(picker);
        }
        if new_project_cancel_rect().contains(mouse) || !dialog_rect.contains(mouse) {
            picker.new_project_dialog = None;
            picker.status = "New project cancelled.".to_string();
            return None;
        }
    }
    if is_key_pressed(KeyCode::Escape) {
        picker.new_project_dialog = None;
        picker.status = "New project cancelled.".to_string();
        return None;
    }
    if is_key_pressed(KeyCode::Enter) {
        return create_new_project_from_dialog(picker);
    }
    if let Some(dialog) = picker.new_project_dialog.as_mut() {
        dialog.cursor = clamp_char_boundary(&dialog.name, dialog.cursor);
        if handle_text_clipboard_shortcuts(
            &mut dialog.name,
            &mut dialog.cursor,
            &mut dialog.selection_anchor,
        ) {
            drain_text_input();
            return None;
        }
        if is_key_pressed(KeyCode::Home) {
            dialog.cursor = 0;
            dialog.selection_anchor = None;
        }
        if is_key_pressed(KeyCode::End) {
            dialog.cursor = dialog.name.len();
            dialog.selection_anchor = None;
        }
        if is_key_pressed(KeyCode::Left) {
            dialog.cursor = prev_char_boundary(&dialog.name, dialog.cursor);
            dialog.selection_anchor = None;
        }
        if is_key_pressed(KeyCode::Right) {
            dialog.cursor = next_char_boundary(&dialog.name, dialog.cursor);
            dialog.selection_anchor = None;
        }
        if is_key_pressed(KeyCode::Backspace)
            && !delete_text_selection(
                &mut dialog.name,
                &mut dialog.cursor,
                &mut dialog.selection_anchor,
            )
            && dialog.cursor > 0
        {
            let prev = prev_char_boundary(&dialog.name, dialog.cursor);
            dialog.name.replace_range(prev..dialog.cursor, "");
            dialog.cursor = prev;
        }
        if is_key_pressed(KeyCode::Delete)
            && !delete_text_selection(
                &mut dialog.name,
                &mut dialog.cursor,
                &mut dialog.selection_anchor,
            )
            && dialog.cursor < dialog.name.len()
        {
            let next = next_char_boundary(&dialog.name, dialog.cursor);
            dialog.name.replace_range(dialog.cursor..next, "");
        }
        while let Some(ch) = get_char_pressed() {
            if handle_text_control_char(
                &mut dialog.name,
                &mut dialog.cursor,
                &mut dialog.selection_anchor,
                ch,
            ) {
                continue;
            }
            if !ch.is_control() && !matches!(ch, '/' | '\\' | '\0') {
                insert_text_at_cursor(
                    &mut dialog.name,
                    &mut dialog.cursor,
                    &mut dialog.selection_anchor,
                    &ch.to_string(),
                );
            }
        }
    }
    None
}

pub(crate) fn project_thumbnail_texture(
    picker: &mut ProjectPicker,
    path: &Path,
) -> Option<Texture2D> {
    if !picker.thumbnails.contains_key(path) {
        let texture = project_thumbnail_paths(path)
            .into_iter()
            .find_map(|thumb_path| fs::read(thumb_path).ok())
            .map(|bytes| Texture2D::from_file_with_format(&bytes, Some(ImageFormat::Png)));
        if let Some(texture) = texture.as_ref() {
            texture.set_filter(FilterMode::Linear);
        }
        picker.thumbnails.insert(path.to_path_buf(), texture);
    }
    picker
        .thumbnails
        .get(path)
        .and_then(|texture| texture.as_ref().map(Texture2D::weak_clone))
}

pub(crate) fn draw_project_picker(picker: &mut ProjectPicker) {
    reset_gl_for_ui();
    draw_rectangle(0.0, 0.0, screen_width(), screen_height(), ui_canvas_bg());

    // Hero panel: a single opaque rounded panel with a 1px border. (Layering a
    // translucent rrect over an opaque one with mismatched radii produced the
    // washed-out corner blobs in the old layout.)
    let hero_w = (screen_width() - 48.0).clamp(360.0, 900.0);
    let hero_x = (screen_width() - hero_w) * 0.5;
    let hero = Rect::new(hero_x, PICK_HERO_Y, hero_w, PICK_HERO_H);
    draw_rrect(
        hero.x + 3.0,
        hero.y + 6.0,
        hero.w,
        hero.h,
        18.0,
        ui_shadow(),
    );
    draw_rrect_bordered(
        hero.x,
        hero.y,
        hero.w,
        hero.h,
        18.0,
        1.0,
        Color::new(0.065, 0.069, 0.076, 1.0),
        ui_border(),
    );
    // Accent bar on the left edge of the panel.
    draw_rrect(
        hero.x + 24.0,
        hero.y + 20.0,
        5.0,
        hero.h - 40.0,
        2.5,
        ui_accent(),
    );

    let text_x = hero.x + 46.0;
    ui_text_bold("MTA:SA Eagle Edit", text_x, hero.y + 51.0, 30, WHITE);
    ui_text_bold(
        "PROJECT WORKSPACE",
        text_x,
        hero.y + 82.0,
        13,
        Color::new(0.68, 0.70, 0.74, 1.0),
    );
    ui_text_size(
        &picker.ui_font,
        &ellipsize_width(
            "Create a map project, reopen recent work, or edit external objects and IMG files.",
            14,
            hero.w - 84.0,
        ),
        text_x,
        hero.y + 110.0,
        14,
        ui_dim(),
    );

    if SHOW_NEW_PROJECT {
        text_button(&picker.ui_font, project_new_rect(), "New Project", false);
    }
    text_button(
        &picker.ui_font,
        project_browse_rect(),
        "Browse Project",
        false,
    );
    text_button(
        &picker.ui_font,
        project_roots_rect(),
        "Project Roots",
        false,
    );
    text_button(
        &picker.ui_font,
        project_open_editor_rect(),
        "External Asset Editor",
        false,
    );

    // Status line, centered under the button.
    let status = ellipsize_width(&picker.status, 14, screen_width() - 72.0);
    let status_w = ui_text_width(&status, 14);
    ui_text_size(
        &picker.ui_font,
        &status,
        (screen_width() - status_w) * 0.5,
        PICK_STATUS_Y,
        14,
        ui_muted(),
    );

    // Section header, left-aligned to the card grid.
    let grid_x = project_grid_start_x();
    let cols = project_grid_cols();
    let first = picker.project_scroll_row * cols;
    let visible_count = project_visible_rows() * cols;
    let visible_end = (first + visible_count).min(picker.projects.len());
    let project_heading = if picker.projects.is_empty() {
        "Projects".to_string()
    } else {
        format!(
            "Projects  {}-{} of {}",
            first + 1,
            visible_end,
            picker.projects.len()
        )
    };
    ui_text_bold(&project_heading, grid_x, PICK_HEADER_Y, 18, WHITE);

    if picker.projects.is_empty() {
        ui_text_size(
            &picker.ui_font,
            "No projects found. Add a Project Root or browse to open one.",
            grid_x,
            PICK_GRID_Y + 28.0,
            15,
            ui_dim(),
        );
    }

    for (slot, path) in picker
        .projects
        .clone()
        .iter()
        .skip(first)
        .take(visible_count)
        .enumerate()
    {
        let rect = project_visible_card_rect(slot);
        let hovered = rect.contains(mouse_position().into());
        let bg = if hovered {
            ui_surface_hover()
        } else {
            ui_panel_bg()
        };
        let border = if hovered { ui_accent() } else { ui_border() };
        draw_rrect(
            rect.x + 2.0,
            rect.y + 4.0,
            rect.w,
            rect.h,
            14.0,
            ui_shadow(),
        );
        draw_rrect_bordered(rect.x, rect.y, rect.w, rect.h, 14.0, 1.0, bg, border);
        if hovered {
            draw_rrect(rect.x + 14.0, rect.y, rect.w - 28.0, 3.0, 1.5, ui_accent());
        }

        let thumb = Rect::new(rect.x + 14.0, rect.y + 14.0, rect.w - 28.0, 80.0);
        if let Some(texture) = project_thumbnail_texture(picker, path) {
            draw_texture_ex(
                &texture,
                thumb.x,
                thumb.y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(thumb.w, thumb.h)),
                    ..Default::default()
                },
            );
            draw_rectangle_lines(thumb.x, thumb.y, thumb.w, thumb.h, 1.0, ui_border());
        } else {
            draw_rrect(
                thumb.x,
                thumb.y,
                thumb.w,
                thumb.h,
                8.0,
                project_thumb_color(path),
            );
        }

        let name = project_name(path);
        ui_text_bold(
            &ellipsize_width(&name, 18, rect.w - 32.0),
            rect.x + 16.0,
            rect.y + 118.0,
            18,
            WHITE,
        );
        ui_text_size(
            &picker.ui_font,
            &ellipsize_width(path.to_string_lossy().as_ref(), 13, rect.w - 32.0),
            rect.x + 16.0,
            rect.y + 138.0,
            13,
            ui_muted(),
        );
    }

    let total_rows = project_total_rows(picker.projects.len());
    let visible_rows = project_visible_rows();
    if total_rows > visible_rows {
        let track = project_scroll_track_rect();
        draw_rrect(track.x, track.y, track.w, track.h, 4.0, ui_input_bg());
        if let Some(thumb) = project_scroll_thumb_rect(picker) {
            draw_rrect(
                thumb.x,
                thumb.y,
                thumb.w,
                thumb.h,
                4.0,
                if thumb.contains(mouse_position().into()) {
                    ui_accent()
                } else {
                    Color::new(0.30, 0.32, 0.35, 1.0)
                },
            );
        }
    }

    draw_new_project_dialog(picker);
    draw_project_roots_dialog(picker);
}

fn draw_project_roots_dialog(picker: &ProjectPicker) {
    if !picker.project_roots_dialog {
        return;
    }
    draw_modal_backdrop();
    let rect = project_roots_dialog_rect();
    draw_panel_rect(&picker.ui_font, rect, Some("Project Roots"));
    ui_text_size(
        &picker.ui_font,
        "Folders are searched recursively for Eagle map projects.",
        rect.x + 24.0,
        rect.y + 58.0,
        14,
        ui_dim(),
    );
    if picker.project_roots.is_empty() {
        ui_text_size(
            &picker.ui_font,
            "No Project Roots added.",
            rect.x + 24.0,
            rect.y + 104.0,
            15,
            ui_muted(),
        );
    }
    for (index, root) in picker.project_roots.iter().enumerate() {
        let y = rect.y + 86.0 + index as f32 * 34.0;
        draw_rrect_bordered(
            rect.x + 24.0,
            y,
            rect.w - 136.0,
            28.0,
            6.0,
            1.0,
            Color::new(0.055, 0.064, 0.078, 1.0),
            ui_border(),
        );
        ui_text_size(
            &picker.ui_font,
            &ellipsize_width(root.to_string_lossy().as_ref(), 14, rect.w - 164.0),
            rect.x + 32.0,
            y + 19.0,
            14,
            LIGHTGRAY,
        );
        draw_dialog_button(
            &picker.ui_font,
            project_root_remove_rect(index),
            "Remove",
            false,
        );
    }
    draw_dialog_button(&picker.ui_font, project_roots_add_rect(), "Add Root", true);
    draw_dialog_button(&picker.ui_font, project_roots_close_rect(), "Close", false);
}

pub(crate) fn draw_new_project_dialog(picker: &ProjectPicker) {
    let Some(dialog) = picker.new_project_dialog.as_ref() else {
        return;
    };
    draw_modal_backdrop();
    let rect = new_project_dialog_rect();
    draw_panel_rect(&picker.ui_font, rect, Some("New Project"));
    ui_text_size(
        &picker.ui_font,
        "Project Root (parent folder)",
        rect.x + 24.0,
        rect.y + 70.0,
        14,
        ui_dim(),
    );
    let root_rect = new_project_root_rect();
    draw_rrect_bordered(
        root_rect.x,
        root_rect.y,
        root_rect.w,
        root_rect.h,
        7.0,
        1.0,
        Color::new(0.055, 0.064, 0.078, 1.0),
        ui_border(),
    );
    ui_text_size(
        &picker.ui_font,
        &ellipsize_width(&dialog.root, 16, root_rect.w - 18.0),
        root_rect.x + 9.0,
        root_rect.y + 22.0,
        16,
        LIGHTGRAY,
    );
    draw_dialog_button(
        &picker.ui_font,
        new_project_browse_root_rect(),
        "Browse",
        false,
    );

    ui_text_size(
        &picker.ui_font,
        "Project Name",
        rect.x + 24.0,
        rect.y + 138.0,
        14,
        ui_dim(),
    );
    let name_rect = new_project_name_rect();
    draw_rrect_bordered(
        name_rect.x,
        name_rect.y,
        name_rect.w,
        name_rect.h,
        7.0,
        1.0,
        Color::new(0.055, 0.064, 0.078, 1.0),
        ui_accent(),
    );
    let visible = ellipsize_width(&dialog.name, 16, name_rect.w - 20.0);
    ui_text_size(
        &picker.ui_font,
        &visible,
        name_rect.x + 10.0,
        name_rect.y + 22.0,
        16,
        WHITE,
    );
    if (get_time() * 2.0) as i32 % 2 == 0 {
        let cursor = clamp_char_boundary(&dialog.name, dialog.cursor);
        let prefix = &dialog.name[..cursor];
        let caret_x =
            (name_rect.x + 10.0 + ui_text_width(prefix, 16)).min(name_rect.x + name_rect.w - 8.0);
        draw_line(
            caret_x,
            name_rect.y + 7.0,
            caret_x,
            name_rect.y + name_rect.h - 7.0,
            1.0,
            WHITE,
        );
    }
    let target = Path::new(dialog.root.trim()).join(dialog.name.trim());
    ui_text_size(
        &picker.ui_font,
        &format!(
            "Project folder: {}",
            ellipsize(target.to_string_lossy().as_ref(), 70)
        ),
        rect.x + 24.0,
        rect.y + 214.0,
        14,
        ui_muted(),
    );
    draw_dialog_button(&picker.ui_font, new_project_create_rect(), "Create", true);
    draw_dialog_button(&picker.ui_font, new_project_cancel_rect(), "Cancel", false);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_minimal_loadable_eagle_project() {
        let parent = env::temp_dir().join(format!(
            "eagle_new_project_test_{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&parent).unwrap();
        let project = create_eagle_project(&parent, "My New Map").unwrap();
        assert!(is_eagle_resource(&project));
        assert!(project.join("meta.xml").is_file());
        assert!(project.join("imgs").is_dir());
        assert!(project.join("textures").is_dir());
        let meta = fs::read_to_string(project.join("meta.xml")).unwrap();
        assert!(meta.contains("name=\"My New Map\""));
        let _ = fs::remove_dir_all(parent);
    }

    #[test]
    fn rejects_project_names_with_path_separators() {
        assert!(create_eagle_project(Path::new("/tmp"), "bad/name").is_err());
    }

    #[test]
    fn discovers_projects_below_a_project_root() {
        let root = env::temp_dir().join(format!(
            "eagle_project_root_test_{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("group")).unwrap();
        let direct = create_eagle_project(&root, "Direct Map").unwrap();
        let nested = create_eagle_project(&root.join("group"), "Nested Map").unwrap();

        let projects = discover_projects(&root);
        assert!(
            projects
                .iter()
                .any(|path| same_resource_path(path, &direct))
        );
        assert!(
            projects
                .iter()
                .any(|path| same_resource_path(path, &nested))
        );
        let _ = fs::remove_dir_all(root);
    }
}
