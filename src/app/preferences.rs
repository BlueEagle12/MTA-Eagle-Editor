use super::super::*;

pub(crate) const DEFAULT_DAY_NIGHT_MERGE_TOLERANCE: f32 = 0.025;
const LEGACY_DAY_NIGHT_MERGE_TOLERANCE: f32 = 0.01;
const DAY_NIGHT_MERGE_TOLERANCE_PREFERENCE_VERSION: u32 = 2;
const DAY_NIGHT_MERGE_TOLERANCE_VERSION_KEY: &str = "day_night_merge_position_tolerance_version";

pub(crate) fn conf() -> Conf {
    Conf {
        window_title: "MTA:SA Eagle Edit".to_string(),
        window_width: 1600,
        window_height: 900,
        high_dpi: false,
        sample_count: requested_msaa_samples(),
        icon: Some(macroquad::miniquad::conf::Icon {
            small: app_icon::SMALL,
            medium: app_icon::MEDIUM,
            big: app_icon::BIG,
        }),
        platform: macroquad::miniquad::conf::Platform {
            linux_x11_gl: macroquad::miniquad::conf::LinuxX11Gl::GLXOnly,
            linux_backend: macroquad::miniquad::conf::LinuxBackend::X11Only,
            swap_interval: Some(1),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub(crate) fn clamp_msaa_samples(value: i32) -> i32 {
    match value {
        0 | 1 => 1,
        2 => 2,
        3 | 4 => 4,
        _ => 1,
    }
}

pub(crate) fn load_msaa_samples_preference() -> i32 {
    load_preferences()
        .get("msaa_samples")
        .and_then(|value| value.parse::<i32>().ok())
        .map(clamp_msaa_samples)
        .unwrap_or(DEFAULT_MSAA_SAMPLES)
}

pub(crate) fn save_msaa_samples_preference(samples: i32) {
    let mut values = load_preferences();
    values.insert(
        "msaa_samples".to_string(),
        clamp_msaa_samples(samples).to_string(),
    );
    save_preferences(&values);
}

pub(crate) const MAX_DRAW_DISTANCE_PERCENT: u16 = 500;

pub(crate) fn clamp_draw_distance_percent(value: u16) -> u16 {
    value.min(MAX_DRAW_DISTANCE_PERCENT)
}

/// Zero disables distance culling, so use the full renderable viewport depth.
pub(crate) fn draw_radius_for_percent(value: u16) -> f32 {
    let percent = clamp_draw_distance_percent(value);
    if percent == 0 {
        VIEWPORT_FAR_CLIP
    } else {
        DEFAULT_DRAW * percent as f32 / 100.0
    }
}

#[cfg(test)]
mod draw_distance_preference_tests {
    use super::*;

    #[test]
    fn draw_distance_accepts_disabled_through_five_hundred_percent() {
        assert_eq!(clamp_draw_distance_percent(0), 0);
        assert_eq!(clamp_draw_distance_percent(500), 500);
        assert_eq!(clamp_draw_distance_percent(501), 500);
    }

    #[test]
    fn disabled_draw_distance_uses_the_viewport_far_clip() {
        assert_eq!(draw_radius_for_percent(0), VIEWPORT_FAR_CLIP);
        assert_eq!(draw_radius_for_percent(500), DEFAULT_DRAW * 5.0);
    }
}

pub(crate) fn load_draw_distance_percent_preference() -> u16 {
    load_preferences()
        .get("draw_distance_percent")
        .and_then(|value| value.parse::<u16>().ok())
        .map(clamp_draw_distance_percent)
        .unwrap_or(100)
}

pub(crate) fn save_draw_distance_percent_preference(value: u16) {
    let mut values = load_preferences();
    values.insert(
        "draw_distance_percent".to_string(),
        clamp_draw_distance_percent(value).to_string(),
    );
    save_preferences(&values);
}

pub(crate) fn msaa_samples_label(samples: i32) -> &'static str {
    match clamp_msaa_samples(samples) {
        2 => "2x MSAA",
        4 => "4x MSAA",
        _ => "Off",
    }
}

/// Cycles Off -> 2x -> 4x -> Off.
pub(crate) fn next_msaa_samples(samples: i32) -> i32 {
    match clamp_msaa_samples(samples) {
        1 => 2,
        2 => 4,
        _ => 1,
    }
}

pub(crate) fn requested_msaa_samples() -> i32 {
    // The saved preference is the baseline; the env var and CLI flag still win
    // so a bad setting can be overridden without editing preferences.txt.
    let mut samples = env::var("EAGLE_MSAA")
        .ok()
        .and_then(|value| value.parse::<i32>().ok())
        .map(clamp_msaa_samples)
        .unwrap_or_else(load_msaa_samples_preference);
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--msaa" => {
                if let Some(value) = args.next().and_then(|value| value.parse::<i32>().ok()) {
                    samples = clamp_msaa_samples(value);
                }
            }
            "--no-msaa" => samples = 1,
            _ => {}
        }
    }
    samples
}

pub(crate) fn clamp_camera_speed(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(MIN_CAMERA_SPEED, MAX_CAMERA_SPEED)
    } else {
        DEFAULT_CAMERA_SPEED
    }
}

pub(crate) fn clamp_detail_camera_speed(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(MIN_DETAIL_CAMERA_SPEED, MAX_CAMERA_SPEED)
    } else {
        DEFAULT_DETAIL_CAMERA_SPEED
    }
}

pub(crate) fn clamp_editing_camera_speed(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(MIN_EDITING_CAMERA_SPEED, MAX_CAMERA_SPEED)
    } else {
        DEFAULT_DETAIL_CAMERA_SPEED
    }
}

pub(crate) fn clamp_gizmo_scale(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(MIN_GIZMO_SCALE, MAX_GIZMO_SCALE)
    } else {
        DEFAULT_GIZMO_SCALE
    }
}

/// Step size for the gimbal stepper. Coarser at the top of the range so getting
/// from 1x to 10x does not take three dozen clicks.
pub(crate) fn gizmo_scale_step(value: f32, increasing: bool) -> f32 {
    let band = if increasing {
        value + 0.001
    } else {
        value - 0.001
    };
    if band < 2.0 {
        0.25
    } else if band < 5.0 {
        0.5
    } else {
        1.0
    }
}

pub(crate) fn clamp_camera_rotation_speed(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(MIN_CAMERA_ROTATION_SPEED, MAX_CAMERA_ROTATION_SPEED)
    } else {
        DEFAULT_CAMERA_ROTATION_SPEED
    }
}

pub(crate) fn clamp_fog_strength(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(MIN_FOG_STRENGTH, MAX_FOG_STRENGTH)
    } else {
        DEFAULT_FOG_STRENGTH
    }
}

pub(crate) fn default_bake_settings() -> BakeSettings {
    BakeSettings {
        preset: BakeQualityPreset::Balanced,
        backend: BakeBackend::Cpu,
        light_mode: BakeLightMode::Day,
        scope: BakeScope::WholeScene,
        face_emitters_enabled: true,
        shadow_samples: 32,
        shadow_chunks: 4,
        bounces: 4,
        bounce_strength: 0.45,
        bounce_maximum: 0.12,
        exposure: 1.0,
        ambient_bump: 0.0,
        shadow_softness: 160.0,
        ao_samples: 24,
        ao_radius: 12.0,
        ao_strength: 0.6,
        day_night_merge_tolerance: DEFAULT_DAY_NIGHT_MERGE_TOLERANCE,
    }
}

pub(crate) fn bake_quality_label(preset: BakeQualityPreset) -> &'static str {
    match preset {
        BakeQualityPreset::Preview => "Preview",
        BakeQualityPreset::Balanced => "Balanced",
        BakeQualityPreset::Final => "Final",
    }
}

pub(crate) fn bake_backend_label(backend: BakeBackend) -> &'static str {
    match backend {
        BakeBackend::Cpu => "CPU",
        BakeBackend::Gpu => "GPU",
    }
}

pub(crate) fn bake_light_mode_label(mode: BakeLightMode) -> &'static str {
    match mode {
        BakeLightMode::Day => "Day",
        BakeLightMode::Night => "Night",
        BakeLightMode::Both => "Both",
    }
}

pub(crate) fn bake_scope_label(scope: BakeScope) -> &'static str {
    match scope {
        BakeScope::WholeScene => "Whole Scene",
        BakeScope::Selected => "Selected",
    }
}

pub(crate) fn bake_quality_settings(preset: BakeQualityPreset) -> BakeSettings {
    match preset {
        BakeQualityPreset::Preview => BakeSettings {
            preset,
            backend: BakeBackend::Cpu,
            light_mode: BakeLightMode::Day,
            scope: BakeScope::WholeScene,
            face_emitters_enabled: true,
            shadow_samples: 0,
            shadow_chunks: 1,
            bounces: 0,
            bounce_strength: 0.0,
            bounce_maximum: 0.0,
            exposure: 1.0,
            ambient_bump: 0.0,
            shadow_softness: 0.0,
            ao_samples: 8,
            ao_radius: 12.0,
            ao_strength: 0.6,
            day_night_merge_tolerance: DEFAULT_DAY_NIGHT_MERGE_TOLERANCE,
        },
        BakeQualityPreset::Balanced => default_bake_settings(),
        BakeQualityPreset::Final => BakeSettings {
            preset,
            backend: BakeBackend::Cpu,
            light_mode: BakeLightMode::Day,
            scope: BakeScope::WholeScene,
            face_emitters_enabled: true,
            shadow_samples: 128,
            shadow_chunks: 8,
            bounces: 8,
            bounce_strength: 0.65,
            bounce_maximum: 0.12,
            exposure: 1.0,
            ambient_bump: 0.0,
            shadow_softness: 320.0,
            ao_samples: 64,
            ao_radius: 12.0,
            ao_strength: 0.6,
            day_night_merge_tolerance: DEFAULT_DAY_NIGHT_MERGE_TOLERANCE,
        },
    }
}

pub(crate) fn clamp_bake_settings(mut settings: BakeSettings) -> BakeSettings {
    settings.shadow_samples = settings.shadow_samples.min(1024);
    settings.shadow_chunks = settings.shadow_chunks.clamp(1, 256);
    settings.bounces = settings.bounces.min(64);
    settings.bounce_strength = if settings.bounce_strength.is_finite() {
        settings.bounce_strength.clamp(0.0, 2.0)
    } else {
        default_bake_settings().bounce_strength
    };
    settings.bounce_maximum = if settings.bounce_maximum.is_finite() {
        settings.bounce_maximum.clamp(0.0, 1.0)
    } else {
        default_bake_settings().bounce_maximum
    };
    settings.exposure = if settings.exposure.is_finite() {
        settings.exposure.clamp(0.0, 4.0)
    } else {
        default_bake_settings().exposure
    };
    settings.ambient_bump = if settings.ambient_bump.is_finite() {
        settings.ambient_bump.clamp(0.0, 1.0)
    } else {
        default_bake_settings().ambient_bump
    };
    settings.shadow_softness = if settings.shadow_softness.is_finite() {
        settings.shadow_softness.clamp(0.0, 2048.0)
    } else {
        default_bake_settings().shadow_softness
    };
    settings.ao_samples = settings.ao_samples.clamp(1, 256);
    settings.ao_radius = if settings.ao_radius.is_finite() {
        settings.ao_radius.clamp(0.5, 4096.0)
    } else {
        default_bake_settings().ao_radius
    };
    settings.ao_strength = if settings.ao_strength.is_finite() {
        settings.ao_strength.clamp(0.0, 1.0)
    } else {
        default_bake_settings().ao_strength
    };
    settings.day_night_merge_tolerance = if settings.day_night_merge_tolerance.is_finite() {
        settings.day_night_merge_tolerance.clamp(0.0, 1.0)
    } else {
        default_bake_settings().day_night_merge_tolerance
    };
    settings
}

pub(crate) fn parse_bake_quality_preset(value: &str) -> BakeQualityPreset {
    match lower(value).as_str() {
        "preview" => BakeQualityPreset::Preview,
        "final" => BakeQualityPreset::Final,
        _ => BakeQualityPreset::Balanced,
    }
}

pub(crate) fn parse_bake_backend(value: &str) -> BakeBackend {
    match lower(value).as_str() {
        "gpu" => BakeBackend::Gpu,
        _ => BakeBackend::Cpu,
    }
}

pub(crate) fn parse_bake_light_mode(value: &str) -> BakeLightMode {
    match lower(value).as_str() {
        "night" => BakeLightMode::Night,
        "both" => BakeLightMode::Both,
        _ => BakeLightMode::Day,
    }
}

pub(crate) fn parse_bake_scope(value: &str) -> BakeScope {
    match lower(value).as_str() {
        "selected" | "selected elements" => BakeScope::Selected,
        _ => BakeScope::WholeScene,
    }
}

pub(crate) fn vertex_light_tool_label(tool: VertexLightTool) -> &'static str {
    match tool {
        VertexLightTool::Paint => "Paint",
        VertexLightTool::Sample => "Sample",
    }
}

pub(crate) fn parse_vertex_light_tool(value: &str) -> VertexLightTool {
    match lower(value).as_str() {
        "sample" => VertexLightTool::Sample,
        _ => VertexLightTool::Paint,
    }
}

pub(crate) fn default_vertex_paint_settings() -> VertexPaintSettings {
    VertexPaintSettings {
        enabled: false,
        tool: VertexLightTool::Paint,
        color: neutral_vertex_color(),
        temperature: 5600.0,
        radius: 25.0,
        strength: 0.75,
    }
}

pub(crate) fn clamp_vertex_paint_settings(
    mut settings: VertexPaintSettings,
) -> VertexPaintSettings {
    settings.color = v3_clamp01(settings.color);
    settings.temperature = if settings.temperature.is_finite() {
        settings.temperature.clamp(1000.0, 40000.0)
    } else {
        default_vertex_paint_settings().temperature
    };
    settings.radius = if settings.radius.is_finite() {
        settings.radius.clamp(1.0, 4096.0)
    } else {
        default_vertex_paint_settings().radius
    };
    settings.strength = if settings.strength.is_finite() {
        settings.strength.clamp(0.0, 1.0)
    } else {
        default_vertex_paint_settings().strength
    };
    settings
}

pub(crate) fn preferences_path() -> PathBuf {
    #[cfg(windows)]
    if let Some(config_home) = env::var_os("APPDATA") {
        return PathBuf::from(config_home)
            .join("EagleEditor")
            .join("preferences.txt");
    }
    if let Some(config_home) = env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(config_home)
            .join("mta_sa_eagle_edit")
            .join("preferences.txt");
    }
    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".config")
            .join("mta_sa_eagle_edit")
            .join("preferences.txt");
    }
    PathBuf::from("mta_sa_eagle_edit_preferences.txt")
}

/// Resolves packaged assets beside the executable, with a source-tree fallback
/// for `cargo run` and tests.
pub(crate) fn asset_path(relative: impl AsRef<Path>) -> PathBuf {
    let relative = relative.as_ref();
    if let Ok(executable) = env::current_exe()
        && let Some(dir) = executable.parent()
    {
        let packaged = dir.join(ASSET_DIR_NAME).join(relative);
        if packaged.exists() {
            return packaged;
        }
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(ASSET_DIR_NAME)
        .join(relative)
}

pub(crate) fn load_preferences() -> BTreeMap<String, String> {
    let Ok(text) = fs::read_to_string(preferences_path()) else {
        return BTreeMap::new();
    };
    text.lines()
        .filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            Some((key.trim().to_string(), value.trim().to_string()))
        })
        .collect()
}

pub(crate) fn save_preferences(values: &BTreeMap<String, String>) {
    let _ = save_preferences_checked(values);
}

pub(crate) fn save_preferences_checked(values: &BTreeMap<String, String>) -> std::io::Result<()> {
    let path = preferences_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut out = String::new();
    for (key, value) in values {
        out.push_str(key);
        out.push('=');
        out.push_str(value);
        out.push('\n');
    }
    fs::write(path, out)
}

pub(crate) fn app_config_dir() -> PathBuf {
    preferences_path()
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub(crate) fn project_state_key(path: &Path) -> String {
    let stable = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let mut hasher = DefaultHasher::new();
    stable.to_string_lossy().hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

pub(crate) fn project_thumbnail_path(path: &Path) -> PathBuf {
    app_config_dir()
        .join("project_thumbnails")
        .join(format!("{}.png", project_state_key(path)))
}

pub(crate) fn project_local_thumbnail_path(path: &Path) -> PathBuf {
    path.join(".light_mapper_thumbnail.png")
}

pub(crate) fn project_thumbnail_paths(path: &Path) -> [PathBuf; 2] {
    [
        project_thumbnail_path(path),
        project_local_thumbnail_path(path),
    ]
}

pub(crate) fn project_session_path(path: &Path) -> PathBuf {
    path.join(".light_mapper_session.json")
}

pub(crate) fn project_config_session_path(path: &Path) -> PathBuf {
    app_config_dir()
        .join("project_sessions")
        .join(format!("{}.json", project_state_key(path)))
}

pub(crate) fn project_session_paths(path: &Path) -> [PathBuf; 2] {
    [
        project_session_path(path),
        project_config_session_path(path),
    ]
}

pub(crate) fn project_camera_prefix(path: &Path) -> String {
    format!("project_{}_camera_", project_state_key(path))
}

pub(crate) fn project_camera_to_persist(app: &AppState) -> CameraState {
    if let Some((camera, _)) = app.pending_camera_restore {
        return camera;
    }
    app.sim_editor_camera.unwrap_or_else(|| {
        if matches!(app.active_tab, AppTab::Editing | AppTab::Vehicles) {
            app.gameworld_camera.unwrap_or(app.camera)
        } else {
            app.camera
        }
    })
}

fn editing_asset_name(asset: &EditingAsset) -> &str {
    match asset {
        EditingAsset::Txd(txd) => &txd.name,
        EditingAsset::Dff(dff) => &dff.name,
        EditingAsset::Col(col) => &col.name,
    }
}

pub(crate) fn save_project_session_file(app: &AppState) {
    let camera = project_camera_to_persist(app);
    let mut payload = serde_json::json!({
        "version": 1,
        "camera": {
            "pos": [camera.pos.x, camera.pos.y, camera.pos.z],
            "yaw": camera.yaw,
            "pitch": camera.pitch,
        },
    });
    if let (Some(img_path), Some(asset)) =
        (app.editing.img_path.as_ref(), app.editing.asset.as_ref())
    {
        let editing_camera = if app.active_tab == AppTab::Editing {
            Some(app.camera)
        } else {
            app.editing.camera
        };
        payload["editing"] = serde_json::json!({
            "img_path": img_path,
            "entry": editing_asset_name(asset),
        });
        if let Some(camera) = editing_camera {
            payload["editing"]["camera"] = serde_json::json!({
                "pos": [camera.pos.x, camera.pos.y, camera.pos.z],
                "yaw": camera.yaw,
                "pitch": camera.pitch,
            });
        }
    }
    if let Ok(text) = serde_json::to_string_pretty(&payload) {
        // Session state is user-local rather than shared project metadata.
        // Keep reading the historical root-side file for compatibility until
        // EagleScene migration archives it, but write only to app config.
        let path = project_config_session_path(&app.root);
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(path, &text);
    }
}

pub(crate) fn save_project_camera_state(app: &AppState) {
    save_project_session_file(app);
    let prefix = project_camera_prefix(&app.root);
    let camera = project_camera_to_persist(app);
    let mut values = load_preferences();
    values.insert(format!("{prefix}x"), format!("{:.6}", camera.pos.x));
    values.insert(format!("{prefix}y"), format!("{:.6}", camera.pos.y));
    values.insert(format!("{prefix}z"), format!("{:.6}", camera.pos.z));
    values.insert(format!("{prefix}yaw"), format!("{:.6}", camera.yaw));
    values.insert(format!("{prefix}pitch"), format!("{:.6}", camera.pitch));
    save_preferences(&values);
}

fn camera_from_session_json(camera: &serde_json::Value) -> Option<CameraState> {
    let Some(pos) = camera.get("pos").and_then(|value| value.as_array()) else {
        return None;
    };
    let Some(x) = pos.first().and_then(|value| value.as_f64()) else {
        return None;
    };
    let Some(y) = pos.get(1).and_then(|value| value.as_f64()) else {
        return None;
    };
    let Some(z) = pos.get(2).and_then(|value| value.as_f64()) else {
        return None;
    };
    let Some(yaw) = camera.get("yaw").and_then(|value| value.as_f64()) else {
        return None;
    };
    let Some(pitch) = camera.get("pitch").and_then(|value| value.as_f64()) else {
        return None;
    };
    if !x.is_finite() || !y.is_finite() || !z.is_finite() || !yaw.is_finite() || !pitch.is_finite()
    {
        return None;
    }
    Some(CameraState {
        pos: vec3(x as f32, y as f32, z as f32),
        yaw: yaw as f32,
        pitch: (pitch as f32).clamp(-1.54, 1.54),
        last_mouse: mouse_position().into(),
        looking: false,
    })
}

fn apply_project_session_camera(app: &mut AppState, json: &serde_json::Value) -> bool {
    let Some(camera) = json.get("camera").and_then(camera_from_session_json) else {
        return false;
    };
    app.camera = camera;
    app.gameworld_camera = Some(camera);
    true
}

fn apply_project_session_text(app: &mut AppState, text: &str) -> bool {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(text) else {
        return false;
    };
    apply_project_session_camera(app, &json)
}

pub(crate) fn apply_project_session_file(app: &mut AppState) -> bool {
    let mut paths = project_session_paths(&app.root);
    paths.sort_by_key(|path| {
        fs::metadata(path)
            .and_then(|metadata| metadata.modified())
            .ok()
    });
    for path in paths.into_iter().rev() {
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        if apply_project_session_text(app, &text) {
            return true;
        }
    }
    false
}

fn latest_project_session_json(app: &AppState) -> Option<serde_json::Value> {
    let mut paths = project_session_paths(&app.root);
    paths.sort_by_key(|path| {
        fs::metadata(path)
            .and_then(|metadata| metadata.modified())
            .ok()
    });
    for path in paths.into_iter().rev() {
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
            return Some(json);
        }
    }
    None
}

pub(crate) fn apply_project_camera_state(app: &mut AppState) -> bool {
    if apply_project_session_file(app) {
        return true;
    }
    let prefix = project_camera_prefix(&app.root);
    let values = load_preferences();
    let Some(x) = values
        .get(&format!("{prefix}x"))
        .and_then(|value| value.parse::<f32>().ok())
    else {
        return false;
    };
    let Some(y) = values
        .get(&format!("{prefix}y"))
        .and_then(|value| value.parse::<f32>().ok())
    else {
        return false;
    };
    let Some(z) = values
        .get(&format!("{prefix}z"))
        .and_then(|value| value.parse::<f32>().ok())
    else {
        return false;
    };
    let Some(yaw) = values
        .get(&format!("{prefix}yaw"))
        .and_then(|value| value.parse::<f32>().ok())
    else {
        return false;
    };
    let Some(pitch) = values
        .get(&format!("{prefix}pitch"))
        .and_then(|value| value.parse::<f32>().ok())
    else {
        return false;
    };
    app.camera.pos = vec3(x, y, z);
    app.camera.yaw = yaw;
    app.camera.pitch = pitch.clamp(-1.54, 1.54);
    app.camera.last_mouse = mouse_position().into();
    app.gameworld_camera = Some(app.camera);
    true
}

pub(crate) fn schedule_project_camera_restore(app: &mut AppState) {
    let default_camera = app.camera;
    if apply_project_camera_state(app) {
        app.pending_camera_restore = Some((app.camera, get_time() + 0.35));
        app.camera = default_camera;
        app.camera.last_mouse = mouse_position().into();
    }
}

pub(crate) fn restore_project_editing_session(app: &mut AppState) {
    let Some(json) = latest_project_session_json(app) else {
        return;
    };
    let Some(editing) = json.get("editing") else {
        return;
    };
    let Some(img_path) = editing.get("img_path").and_then(|value| value.as_str()) else {
        return;
    };
    let Some(entry_name) = editing.get("entry").and_then(|value| value.as_str()) else {
        return;
    };
    let path = PathBuf::from(img_path);
    if !path.is_file() {
        return;
    }
    open_editing_img(app, path);
    let Some(row_idx) = app
        .editing
        .rows
        .iter()
        .position(|row| row.entry.name.eq_ignore_ascii_case(entry_name))
    else {
        return;
    };
    app.editing.selected_row = row_idx;
    app.editing.scroll = row_idx.saturating_sub(4) as f32;
    editing_open_selected_asset(app);
    if app.editing.asset.is_none() {
        return;
    }
    if let Some(camera) = editing.get("camera").and_then(camera_from_session_json) {
        app.camera = camera;
    }
    app.editing.camera = Some(app.camera);
    app.active_tab = AppTab::Editing;
}

pub(crate) fn apply_pending_project_camera_restore(app: &mut AppState) {
    let Some((camera, apply_at)) = app.pending_camera_restore else {
        return;
    };
    if get_time() < apply_at {
        return;
    }
    app.pending_camera_restore = None;
    if matches!(app.active_tab, AppTab::Editing | AppTab::Vehicles) {
        app.gameworld_camera = Some(camera);
        app.last_camera_persist_at = get_time();
        return;
    }
    app.camera = camera;
    app.gameworld_camera = Some(camera);
    app.camera.looking = false;
    app.camera.last_mouse = mouse_position().into();
    app.last_camera_persist_at = get_time();
    set_cursor_grab(false);
    show_mouse(true);
}

pub(crate) fn load_camera_rotation_speed_preference() -> f32 {
    load_preferences()
        .get("camera_rotation_speed")
        .and_then(|value| value.parse::<f32>().ok())
        .map(clamp_camera_rotation_speed)
        .unwrap_or(DEFAULT_CAMERA_ROTATION_SPEED)
}

pub(crate) fn save_camera_rotation_speed_preference(speed: f32) {
    let mut values = load_preferences();
    values.insert(
        "camera_rotation_speed".to_string(),
        format!("{:.3}", clamp_camera_rotation_speed(speed)),
    );
    save_preferences(&values);
}

pub(crate) fn load_gizmo_scale_preference() -> f32 {
    load_preferences()
        .get("gizmo_scale")
        .and_then(|value| value.parse::<f32>().ok())
        .map(clamp_gizmo_scale)
        .unwrap_or(DEFAULT_GIZMO_SCALE)
}

pub(crate) fn save_gizmo_scale_preference(scale: f32) {
    let mut values = load_preferences();
    values.insert(
        "gizmo_scale".to_string(),
        format!("{:.2}", clamp_gizmo_scale(scale)),
    );
    save_preferences(&values);
}

pub(crate) const GTA_SA_MARKER_FILE: &str = "models/gta3.img";
const GTA_SA_SETUP_BYPASSED_KEY: &str = "gta_sa_setup_bypassed";

pub(crate) fn validate_gta_sa_dir(path: &Path) -> Result<(), String> {
    if !path.is_dir() {
        return Err(format!("Folder does not exist: {}", path.display()));
    }
    let marker = path.join(GTA_SA_MARKER_FILE);
    if !marker.is_file() {
        return Err(format!(
            "This is not a GTA:SA root folder: {} is missing",
            marker.display()
        ));
    }
    Ok(())
}

pub(crate) fn load_configured_gta_sa_dir() -> Option<PathBuf> {
    load_preferences()
        .get("gta_sa_dir")
        .map(PathBuf::from)
        .filter(|path| validate_gta_sa_dir(path).is_ok())
}

pub(crate) fn gta_sa_setup_bypassed() -> bool {
    load_preferences()
        .get(GTA_SA_SETUP_BYPASSED_KEY)
        .is_some_and(|value| value.eq_ignore_ascii_case("true"))
}

pub(crate) fn load_gta_sa_dir_candidate() -> PathBuf {
    load_preferences()
        .get("gta_sa_dir")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_GTA_SA_DIR))
}

pub(crate) fn load_gta_sa_dir_preference() -> PathBuf {
    load_configured_gta_sa_dir().unwrap_or_else(|| PathBuf::from(DEFAULT_GTA_SA_DIR))
}

pub(crate) fn save_gta_sa_dir_preference(path: &Path) {
    let mut values = load_preferences();
    values.remove(GTA_SA_SETUP_BYPASSED_KEY);
    values.insert(
        "gta_sa_dir".to_string(),
        path.to_string_lossy().trim().to_string(),
    );
    save_preferences(&values);
}

pub(crate) fn load_blender_install_dir_preference() -> Option<PathBuf> {
    load_preferences()
        .get("blender_install_dir")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

pub(crate) fn blender_executable_in_install_dir(path: &Path) -> Option<PathBuf> {
    if path.is_file() {
        return Some(path.to_path_buf());
    }
    if !path.is_dir() {
        return None;
    }
    let candidates = [
        path.join("blender.exe"),
        path.join("blender"),
        path.join("Contents").join("MacOS").join("Blender"),
    ];
    if let Some(candidate) = candidates.into_iter().find(|candidate| candidate.is_file()) {
        return Some(candidate);
    }
    let mut nested = fs::read_dir(path)
        .ok()?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .flat_map(|entry| {
            let path = entry.path();
            [
                path.join("blender.exe"),
                path.join("blender"),
                path.join("Contents").join("MacOS").join("Blender"),
            ]
        })
        .filter(|candidate| candidate.is_file())
        .collect::<Vec<_>>();
    nested.sort();
    nested.pop()
}

pub(crate) fn common_blender_executable_candidates() -> Vec<PathBuf> {
    let mut candidates = vec![
        PathBuf::from("/usr/bin/blender"),
        PathBuf::from("/usr/local/bin/blender"),
        PathBuf::from("/opt/blender/blender"),
        PathBuf::from("/snap/bin/blender"),
        PathBuf::from("/Applications/Blender.app/Contents/MacOS/Blender"),
    ];
    #[cfg(windows)]
    for base in [
        env::var_os("ProgramFiles"),
        env::var_os("ProgramFiles(x86)"),
    ]
    .into_iter()
    .flatten()
    .map(PathBuf::from)
    {
        if let Ok(entries) = fs::read_dir(base) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_dir()
                    && path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| {
                            name.to_ascii_lowercase().starts_with("blender foundation")
                        })
                    && let Ok(versions) = fs::read_dir(path)
                {
                    for version in versions.filter_map(Result::ok) {
                        candidates.push(version.path().join("blender.exe"));
                    }
                }
            }
        }
    }
    if let Some(user_home) = env::var_os("HOME").map(PathBuf::from) {
        for base in [
            user_home.join("Utilities"),
            user_home.join("Applications"),
            user_home.join(".local/bin"),
        ] {
            if let Ok(entries) = fs::read_dir(base) {
                for entry in entries.filter_map(Result::ok) {
                    let path = entry.path();
                    if path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.to_ascii_lowercase().starts_with("blender"))
                    {
                        candidates.push(if path.is_dir() {
                            let macos = path.join("Contents").join("MacOS").join("Blender");
                            if macos.is_file() {
                                macos
                            } else {
                                path.join("blender")
                            }
                        } else {
                            path
                        });
                    }
                }
            }
        }
    }
    candidates.retain(|path| path.is_file());
    candidates.sort();
    candidates.dedup();
    candidates.reverse();
    candidates
}

fn blender_install_dir_for_executable(executable: &Path) -> Option<PathBuf> {
    let parent = executable.parent()?;
    if parent.file_name().and_then(|name| name.to_str()) == Some("MacOS")
        && parent.parent()?.file_name().and_then(|name| name.to_str()) == Some("Contents")
    {
        parent.parent()?.parent().map(Path::to_path_buf)
    } else {
        Some(parent.to_path_buf())
    }
}

pub(crate) fn search_common_blender_install_dirs() -> Vec<PathBuf> {
    let mut installs = common_blender_executable_candidates()
        .iter()
        .filter_map(|path| blender_install_dir_for_executable(path))
        .collect::<Vec<_>>();
    installs.sort();
    installs.dedup();
    installs.reverse();
    installs
}

pub(crate) fn validate_blender_install_dir(path: &Path) -> Result<(), String> {
    if blender_executable_in_install_dir(path).is_some() {
        Ok(())
    } else {
        Err(format!(
            "This is not a Blender install directory: no Blender executable was found in {}",
            path.display()
        ))
    }
}

pub(crate) fn save_blender_install_dir_preference(path: Option<&Path>) {
    let mut values = load_preferences();
    if let Some(path) = path {
        values.insert(
            "blender_install_dir".to_string(),
            path.to_string_lossy().trim().to_string(),
        );
    } else {
        values.remove("blender_install_dir");
    }
    save_preferences(&values);
}

#[cfg(test)]
mod blender_install_dir_tests {
    use super::*;

    fn test_dir(label: &str) -> PathBuf {
        env::temp_dir().join(format!(
            "eagle_editor_blender_dir_{label}_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn blender_install_directory_resolves_direct_and_versioned_executables() {
        let root = test_dir("resolve");
        let version = root.join("Blender 4.5");
        fs::create_dir_all(&version).unwrap();
        let executable = version.join("blender");
        fs::write(&executable, b"").unwrap();

        assert_eq!(
            blender_executable_in_install_dir(&version),
            Some(executable.clone())
        );
        assert_eq!(blender_executable_in_install_dir(&root), Some(executable));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn blender_install_directory_rejects_a_folder_without_an_executable() {
        let root = test_dir("reject");
        fs::create_dir_all(&root).unwrap();

        assert!(blender_executable_in_install_dir(&root).is_none());
        assert!(validate_blender_install_dir(&root).is_err());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn executable_paths_map_back_to_the_install_directory() {
        assert_eq!(
            blender_install_dir_for_executable(Path::new(
                "/Applications/Blender.app/Contents/MacOS/Blender"
            )),
            Some(PathBuf::from("/Applications/Blender.app"))
        );
        assert_eq!(
            blender_install_dir_for_executable(Path::new(
                "C:/Program Files/Blender Foundation/Blender 4.5/blender.exe"
            )),
            Some(PathBuf::from(
                "C:/Program Files/Blender Foundation/Blender 4.5"
            ))
        );
    }
}

pub(crate) fn save_gta_sa_setup_bypassed() {
    let mut values = load_preferences();
    values.insert(GTA_SA_SETUP_BYPASSED_KEY.to_string(), "true".to_string());
    save_preferences(&values);
}

#[cfg(test)]
mod gta_sa_dir_tests {
    use super::*;

    fn test_root(label: &str) -> PathBuf {
        env::temp_dir().join(format!(
            "eagle_editor_gta_root_{label}_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn gta_root_requires_the_main_model_archive() {
        let root = test_root("validation");
        fs::create_dir_all(root.join("models")).unwrap();
        assert!(validate_gta_sa_dir(&root).is_err());

        fs::write(root.join(GTA_SA_MARKER_FILE), b"VER2").unwrap();
        assert_eq!(validate_gta_sa_dir(&root), Ok(()));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn gta_root_rejects_a_missing_folder() {
        let root = test_root("missing");
        assert!(validate_gta_sa_dir(&root).is_err());
    }
}

pub(crate) fn load_custom_vehicle_dictionary_preferences() -> Vec<PathBuf> {
    let values = load_preferences();
    let mut paths = values
        .iter()
        .filter(|(key, _)| key.starts_with("custom_vehicle_dictionary_"))
        .filter_map(|(_, value)| {
            let path = PathBuf::from(value.trim());
            (!path.as_os_str().is_empty() && path.is_dir()).then_some(path)
        })
        .collect::<Vec<_>>();
    // Migrate the original single-folder preference without making users pick
    // the same dictionary again after upgrading.
    if paths.is_empty()
        && let Some(path) = values
            .get("external_vehicle_dir")
            .map(|value| PathBuf::from(value.trim()))
            .filter(|path| !path.as_os_str().is_empty() && path.is_dir())
    {
        paths.push(path);
    }
    paths
}

pub(crate) fn save_custom_vehicle_dictionary_preferences(paths: &[PathBuf]) {
    let mut values = load_preferences();
    values.retain(|key, _| {
        key != "external_vehicle_dir" && !key.starts_with("custom_vehicle_dictionary_")
    });
    for (idx, path) in paths.iter().enumerate() {
        values.insert(
            format!("custom_vehicle_dictionary_{idx:04}"),
            path.to_string_lossy().trim().to_string(),
        );
    }
    save_preferences(&values);
}

pub(crate) fn load_vehicle_loader_resource_preference() -> Option<PathBuf> {
    load_preferences()
        .get("vehicle_loader_resource")
        .map(|value| PathBuf::from(value.trim()))
        .filter(|path| path.is_dir() && path.join("meta.xml").is_file())
}

pub(crate) fn save_vehicle_loader_resource_preference(path: &Path) {
    let mut values = load_preferences();
    values.insert(
        "vehicle_loader_resource".to_string(),
        path.to_string_lossy().trim().to_string(),
    );
    save_preferences(&values);
}

pub(crate) fn load_fog_strength_preference() -> f32 {
    load_preferences()
        .get("fog_strength")
        .and_then(|value| value.parse::<f32>().ok())
        .map(clamp_fog_strength)
        .unwrap_or(DEFAULT_FOG_STRENGTH)
}

pub(crate) fn save_fog_strength_preference(strength: f32) {
    let mut values = load_preferences();
    values.insert(
        "fog_strength".to_string(),
        format!("{:.2}", clamp_fog_strength(strength)),
    );
    save_preferences(&values);
}

pub(crate) fn load_lod_audit_small_threshold_preference() -> f32 {
    load_preferences()
        .get("lod_audit_small_threshold")
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(1.0, 500.0))
        .unwrap_or(20.0)
}

pub(crate) fn save_lod_audit_small_threshold_preference(threshold: f32) -> std::io::Result<()> {
    let threshold = if threshold.is_finite() {
        threshold.clamp(1.0, 500.0)
    } else {
        20.0
    };
    let mut values = load_preferences();
    values.insert(
        "lod_audit_small_threshold".to_string(),
        format!("{threshold:.1}"),
    );
    save_preferences_checked(&values)
}

fn lod_audit_ignored_coverage_path(project_root: &Path) -> PathBuf {
    app_config_dir()
        .join("lod_audit_ignored")
        .join(format!("{}.txt", project_state_key(project_root)))
}

pub(crate) fn load_lod_audit_ignored_coverage(project_root: &Path) -> HashSet<String> {
    let Ok(text) = fs::read_to_string(lod_audit_ignored_coverage_path(project_root)) else {
        return HashSet::new();
    };
    text.lines()
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub(crate) fn save_lod_audit_ignored_coverage(
    project_root: &Path,
    ignored: &HashSet<String>,
) -> std::io::Result<()> {
    let path = lod_audit_ignored_coverage_path(project_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut keys = ignored.iter().collect::<Vec<_>>();
    keys.sort_unstable();
    let mut out = String::new();
    for key in keys {
        out.push_str(key);
        out.push('\n');
    }
    fs::write(path, out)
}

pub(crate) fn load_bake_settings_preference() -> BakeSettings {
    let values = load_preferences();
    let mut settings = default_bake_settings();
    if let Some(preset) = values.get("bake_quality") {
        settings.preset = parse_bake_quality_preset(preset);
    }
    if let Some(backend) = values.get("bake_backend") {
        settings.backend = parse_bake_backend(backend);
    }
    if let Some(mode) = values.get("bake_light_mode") {
        settings.light_mode = parse_bake_light_mode(mode);
    }
    if let Some(scope) = values.get("bake_scope") {
        settings.scope = parse_bake_scope(scope);
    }
    if let Some(enabled) = values
        .get("bake_face_emitters_enabled")
        .and_then(|value| value.parse::<bool>().ok())
    {
        settings.face_emitters_enabled = enabled;
    }
    if let Some(samples) = values
        .get("bake_shadow_samples")
        .and_then(|value| value.parse::<usize>().ok())
    {
        settings.shadow_samples = samples;
    }
    if let Some(chunks) = values
        .get("bake_shadow_chunks")
        .and_then(|value| value.parse::<usize>().ok())
    {
        settings.shadow_chunks = chunks;
    }
    if let Some(bounces) = values
        .get("bake_bounces")
        .and_then(|value| value.parse::<usize>().ok())
    {
        settings.bounces = bounces;
    }
    if let Some(strength) = values
        .get("bake_bounce_strength")
        .and_then(|value| value.parse::<f32>().ok())
    {
        settings.bounce_strength = strength;
    }
    if let Some(maximum) = values
        .get("bake_bounce_maximum")
        .and_then(|value| value.parse::<f32>().ok())
    {
        settings.bounce_maximum = maximum;
    }
    if let Some(exposure) = values
        .get("bake_exposure")
        .and_then(|value| value.parse::<f32>().ok())
    {
        settings.exposure = exposure;
    }
    if let Some(ambient_bump) = values
        .get("bake_ambient_bump")
        .and_then(|value| value.parse::<f32>().ok())
    {
        settings.ambient_bump = ambient_bump;
    }
    if let Some(softness) = values
        .get("bake_shadow_softness")
        .and_then(|value| value.parse::<f32>().ok())
    {
        settings.shadow_softness = softness;
    }
    if let Some(samples) = values
        .get("bake_ao_samples")
        .and_then(|value| value.parse::<usize>().ok())
    {
        settings.ao_samples = samples;
    }
    if let Some(radius) = values
        .get("bake_ao_radius")
        .and_then(|value| value.parse::<f32>().ok())
    {
        settings.ao_radius = radius;
    }
    if let Some(strength) = values
        .get("bake_ao_strength")
        .and_then(|value| value.parse::<f32>().ok())
    {
        settings.ao_strength = strength;
    }
    if let Some(tolerance) = values
        .get("day_night_merge_position_tolerance")
        .and_then(|value| value.parse::<f32>().ok())
    {
        settings.day_night_merge_tolerance =
            migrate_day_night_merge_tolerance_preference(&values, tolerance);
    }
    clamp_bake_settings(settings)
}

fn migrate_day_night_merge_tolerance_preference(
    values: &BTreeMap<String, String>,
    tolerance: f32,
) -> f32 {
    let version = values
        .get(DAY_NIGHT_MERGE_TOLERANCE_VERSION_KEY)
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or_default();
    if version < DAY_NIGHT_MERGE_TOLERANCE_PREFERENCE_VERSION
        && (tolerance - LEGACY_DAY_NIGHT_MERGE_TOLERANCE).abs() <= f32::EPSILON
    {
        DEFAULT_DAY_NIGHT_MERGE_TOLERANCE
    } else {
        tolerance
    }
}

pub(crate) fn save_bake_settings_preference(settings: BakeSettings) {
    let settings = clamp_bake_settings(settings);
    let mut values = load_preferences();
    values.insert(
        "bake_quality".to_string(),
        bake_quality_label(settings.preset).to_ascii_lowercase(),
    );
    values.insert(
        "bake_backend".to_string(),
        bake_backend_label(settings.backend).to_ascii_lowercase(),
    );
    values.insert(
        "bake_light_mode".to_string(),
        bake_light_mode_label(settings.light_mode).to_ascii_lowercase(),
    );
    values.insert(
        "bake_scope".to_string(),
        bake_scope_label(settings.scope).to_ascii_lowercase(),
    );
    values.insert(
        "bake_face_emitters_enabled".to_string(),
        settings.face_emitters_enabled.to_string(),
    );
    values.insert(
        "bake_shadow_samples".to_string(),
        settings.shadow_samples.to_string(),
    );
    values.insert(
        "bake_shadow_chunks".to_string(),
        settings.shadow_chunks.to_string(),
    );
    values.insert("bake_bounces".to_string(), settings.bounces.to_string());
    values.insert(
        "bake_bounce_strength".to_string(),
        format!("{:.3}", settings.bounce_strength),
    );
    values.insert(
        "bake_bounce_maximum".to_string(),
        format!("{:.3}", settings.bounce_maximum),
    );
    values.insert(
        "bake_exposure".to_string(),
        format!("{:.3}", settings.exposure),
    );
    values.insert(
        "bake_ambient_bump".to_string(),
        format!("{:.3}", settings.ambient_bump),
    );
    values.insert(
        "bake_shadow_softness".to_string(),
        format!("{:.1}", settings.shadow_softness),
    );
    values.insert(
        "bake_ao_samples".to_string(),
        settings.ao_samples.to_string(),
    );
    values.insert(
        "bake_ao_radius".to_string(),
        format!("{:.1}", settings.ao_radius),
    );
    values.insert(
        "bake_ao_strength".to_string(),
        format!("{:.3}", settings.ao_strength),
    );
    values.insert(
        "day_night_merge_position_tolerance".to_string(),
        format!("{:.6}", settings.day_night_merge_tolerance),
    );
    values.insert(
        DAY_NIGHT_MERGE_TOLERANCE_VERSION_KEY.to_string(),
        DAY_NIGHT_MERGE_TOLERANCE_PREFERENCE_VERSION.to_string(),
    );
    save_preferences(&values);
}

#[cfg(test)]
mod bake_settings_preference_tests {
    use super::*;

    fn tolerance_preferences(tolerance: f32, version: Option<u32>) -> BTreeMap<String, String> {
        let mut values = BTreeMap::new();
        values.insert(
            "day_night_merge_position_tolerance".to_string(),
            tolerance.to_string(),
        );
        if let Some(version) = version {
            values.insert(
                DAY_NIGHT_MERGE_TOLERANCE_VERSION_KEY.to_string(),
                version.to_string(),
            );
        }
        values
    }

    #[test]
    fn default_day_night_tolerance_covers_common_vc_z_offset() {
        assert!(default_bake_settings().day_night_merge_tolerance >= 0.015);
    }

    #[test]
    fn legacy_default_day_night_tolerance_is_migrated() {
        let values = tolerance_preferences(LEGACY_DAY_NIGHT_MERGE_TOLERANCE, None);

        assert_eq!(
            migrate_day_night_merge_tolerance_preference(&values, LEGACY_DAY_NIGHT_MERGE_TOLERANCE),
            DEFAULT_DAY_NIGHT_MERGE_TOLERANCE
        );
    }

    #[test]
    fn custom_and_versioned_day_night_tolerances_are_preserved() {
        let custom = tolerance_preferences(0.004, None);
        assert_eq!(
            migrate_day_night_merge_tolerance_preference(&custom, 0.004),
            0.004
        );

        let versioned = tolerance_preferences(
            LEGACY_DAY_NIGHT_MERGE_TOLERANCE,
            Some(DAY_NIGHT_MERGE_TOLERANCE_PREFERENCE_VERSION),
        );
        assert_eq!(
            migrate_day_night_merge_tolerance_preference(
                &versioned,
                LEGACY_DAY_NIGHT_MERGE_TOLERANCE
            ),
            LEGACY_DAY_NIGHT_MERGE_TOLERANCE
        );
    }
}

pub(crate) fn load_vertex_paint_settings_preference() -> VertexPaintSettings {
    let values = load_preferences();
    let mut settings = default_vertex_paint_settings();
    if let Some(tool) = values.get("vertex_paint_tool") {
        settings.tool = parse_vertex_light_tool(tool);
    }
    if let Some(red) = values
        .get("vertex_paint_r")
        .and_then(|value| value.parse::<f32>().ok())
    {
        settings.color.x = red;
    }
    if let Some(green) = values
        .get("vertex_paint_g")
        .and_then(|value| value.parse::<f32>().ok())
    {
        settings.color.y = green;
    }
    if let Some(blue) = values
        .get("vertex_paint_b")
        .and_then(|value| value.parse::<f32>().ok())
    {
        settings.color.z = blue;
    }
    if let Some(temperature) = values
        .get("vertex_paint_temperature")
        .and_then(|value| value.parse::<f32>().ok())
    {
        settings.temperature = temperature;
    }
    if let Some(radius) = values
        .get("vertex_paint_radius")
        .and_then(|value| value.parse::<f32>().ok())
    {
        settings.radius = radius;
    }
    if let Some(strength) = values
        .get("vertex_paint_strength")
        .and_then(|value| value.parse::<f32>().ok())
    {
        settings.strength = strength;
    }
    clamp_vertex_paint_settings(settings)
}

pub(crate) fn save_vertex_paint_settings_preference(settings: VertexPaintSettings) {
    let settings = clamp_vertex_paint_settings(settings);
    let mut values = load_preferences();
    values.insert(
        "vertex_paint_tool".to_string(),
        vertex_light_tool_label(settings.tool).to_ascii_lowercase(),
    );
    values.insert(
        "vertex_paint_r".to_string(),
        format!("{:.3}", settings.color.x),
    );
    values.insert(
        "vertex_paint_g".to_string(),
        format!("{:.3}", settings.color.y),
    );
    values.insert(
        "vertex_paint_b".to_string(),
        format!("{:.3}", settings.color.z),
    );
    values.insert(
        "vertex_paint_temperature".to_string(),
        format!("{:.0}", settings.temperature),
    );
    values.insert(
        "vertex_paint_radius".to_string(),
        format!("{:.1}", settings.radius),
    );
    values.insert(
        "vertex_paint_strength".to_string(),
        format!("{:.3}", settings.strength),
    );
    save_preferences(&values);
}

pub(crate) fn is_eagle_resource(path: &Path) -> bool {
    path.is_dir()
        && ((path.join("eagleZones.txt").is_file() && path.join("zones").is_dir())
            || path.join("meta.xml").is_file()
            || path.join(crate::resource::mta_maps::REGISTRY).is_file())
}

/// Empty working folder used when the editor is launched with no project (the
/// "Open Editor" launcher option). It lives under the app config dir so the
/// editor always has a valid, writable root, but it is not an Eagle resource,
/// so it never shows up in Recent Projects and the scene starts empty.
pub(crate) fn editor_scratch_root() -> PathBuf {
    let root = preferences_path()
        .parent()
        .map(|dir| dir.join("editor_scratch"))
        .unwrap_or_else(|| PathBuf::from("mta_sa_eagle_edit_editor_scratch"));
    let _ = fs::create_dir_all(&root);
    root
}

pub(crate) fn load_recent_projects() -> Vec<PathBuf> {
    let values = load_preferences();
    let mut recents: Vec<PathBuf> = Vec::new();
    for idx in 0..8 {
        let Some(value) = values.get(&format!("recent_project_{idx}")) else {
            continue;
        };
        let path = PathBuf::from(value);
        if is_eagle_resource(&path)
            && !recents
                .iter()
                .any(|existing| same_resource_path(existing, &path))
        {
            recents.push(path);
        }
    }
    if recents.is_empty() {
        for candidate in [
            PathBuf::from(BROWSE_ROOT).join("Liberty_City"),
            PathBuf::from(BROWSE_ROOT).join("out_mafia2"),
        ] {
            if is_eagle_resource(&candidate) {
                recents.push(candidate);
            }
        }
    }
    recents
}

/// Folders the project launcher searches for Eagle resources. These are kept
/// separately from recents so a project can be discovered before it has ever
/// been opened in the editor.
pub(crate) fn load_project_roots() -> Vec<PathBuf> {
    let values = load_preferences();
    let mut roots: Vec<PathBuf> = Vec::new();
    for idx in 0..8 {
        let Some(value) = values.get(&format!("project_root_{idx}")) else {
            continue;
        };
        let path = PathBuf::from(value);
        if path.is_dir()
            && !roots
                .iter()
                .any(|existing| same_resource_path(existing, &path))
        {
            roots.push(path);
        }
    }
    roots
}

pub(crate) fn save_project_roots(roots: &[PathBuf]) {
    let mut values = load_preferences();
    for idx in 0..8 {
        values.remove(&format!("project_root_{idx}"));
    }
    for (idx, root) in roots.iter().take(8).enumerate() {
        values.insert(
            format!("project_root_{idx}"),
            root.to_string_lossy().to_string(),
        );
    }
    save_preferences(&values);
}

/// Parent folder used by the new-project wizard most recently. Keep this
/// separate from the discovery roots: creating a project somewhere should not
/// implicitly opt that entire folder into recursive project scanning.
pub(crate) fn load_last_new_project_root() -> Option<PathBuf> {
    load_preferences()
        .get("last_new_project_root")
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
}

pub(crate) fn save_last_new_project_root(path: &Path) {
    let mut values = load_preferences();
    values.insert(
        "last_new_project_root".to_string(),
        path.to_string_lossy().to_string(),
    );
    save_preferences(&values);
}

pub(crate) fn save_recent_project(path: &Path) {
    if !is_eagle_resource(path) {
        return;
    }
    let mut recents = load_recent_projects();
    recents.retain(|existing| !same_resource_path(existing, path));
    recents.insert(0, path.to_path_buf());
    recents.truncate(8);
    let mut values = load_preferences();
    for idx in 0..8 {
        values.remove(&format!("recent_project_{idx}"));
    }
    for (idx, recent) in recents.iter().enumerate() {
        values.insert(
            format!("recent_project_{idx}"),
            recent.to_string_lossy().to_string(),
        );
    }
    save_preferences(&values);
}

pub(crate) fn load_last_dff_export_dir() -> PathBuf {
    load_preferences()
        .get("last_dff_export_dir")
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(BROWSE_ROOT)))
}

pub(crate) fn save_last_dff_export_dir(path: &Path) {
    let mut values = load_preferences();
    values.insert(
        "last_dff_export_dir".to_string(),
        path.to_string_lossy().to_string(),
    );
    save_preferences(&values);
}
