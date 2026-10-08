use super::*;

const BLENDER_IMPORT_SCRIPT: &str = include_str!("../assets/export_blend_native.py");
const BLENDER_DFF_IMG: &str = "dff.img";
const BLENDER_TXD_IMG: &str = "txd.img";
const LEGACY_BLENDER_DFF_IMG: &str = "blender_dff.img";
const LEGACY_BLENDER_COL_IMG: &str = "blender_col.img";
const BLENDER_PROGRESS_PREFIX: &str = "EAGLE_IMPORT_PROGRESS|";
const MAX_BLENDER_LOG_LINES: usize = 2_000;

#[derive(Debug)]
pub(crate) enum BlenderImportUpdate {
    Progress { fraction: f32, message: String },
    Log(String),
    Finished(Result<BlenderImportSummary, String>),
}

#[derive(Clone, Debug)]
pub(crate) struct BlenderImportOptions {
    pub(crate) chunk_size: f32,
    pub(crate) chunk_meshes: bool,
    pub(crate) center_origins: bool,
}

impl Default for BlenderImportOptions {
    fn default() -> Self {
        Self {
            chunk_size: DEFAULT_SLICER_CHUNK_SIZE,
            chunk_meshes: true,
            center_origins: true,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct BlenderImportSetup {
    pub(crate) source: PathBuf,
    pub(crate) chunk_size: String,
    pub(crate) chunk_size_cursor: usize,
    pub(crate) chunk_size_selection_anchor: Option<usize>,
    pub(crate) options: BlenderImportOptions,
    pub(crate) error: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct BlenderImportSummary {
    pub(crate) zones: Vec<String>,
    pub(crate) dffs: usize,
    pub(crate) txds: usize,
    pub(crate) warnings: Vec<String>,
}

pub(crate) fn open_blender_import_dialog(app: &mut AppState) {
    if app.blender_import_rx.is_some() || app.blender_import_setup.is_some() {
        app.status_message = "A Blender import is already running".to_string();
        return;
    }
    if app.dff_picker_rx.is_some() {
        app.status_message = "An asset file browser is already open".to_string();
        return;
    }
    if has_unsaved_changes(app) {
        app.status_message =
            "Save or discard pending editor changes before importing Blender".to_string();
        return;
    }
    let start_dir = if app.root.is_dir() {
        app.root.clone()
    } else {
        PathBuf::from(BROWSE_ROOT)
    };
    let (tx, rx) = mpsc::channel();
    app.dff_picker_rx = Some(rx);
    app.status_message = "Choose a Blender scene to import...".to_string();
    thread::spawn(move || {
        let _ = tx.send((
            DffPickerKind::ImportBlender,
            choose_blend_scene_path(start_dir),
        ));
    });
}

pub(crate) fn start_blender_import(app: &mut AppState, source: PathBuf) {
    if app.blender_import_rx.is_some() || app.blender_import_setup.is_some() {
        app.status_message = "A Blender import is already running".to_string();
        return;
    }
    if !source.is_file()
        || !source
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("blend"))
    {
        app.status_message = "Import Blender requires a readable .blend file".to_string();
        return;
    }
    if has_unsaved_changes(app) {
        app.status_message =
            "Save or discard pending editor changes before importing Blender".to_string();
        return;
    }
    let options = BlenderImportOptions::default();
    let chunk_size = format!("{:.0}", options.chunk_size);
    app.blender_import_setup = Some(BlenderImportSetup {
        source,
        chunk_size_cursor: chunk_size.len(),
        chunk_size_selection_anchor: None,
        chunk_size,
        options,
        error: None,
    });
    app.status_message = "Configure Blender map generation, then start the import".to_string();
}

pub(crate) fn confirm_blender_import_setup(app: &mut AppState) {
    let Some(mut setup) = app.blender_import_setup.take() else {
        return;
    };
    let chunk_size = match setup.chunk_size.trim().parse::<f32>() {
        Ok(value) if value.is_finite() && (1.0..=10_000.0).contains(&value) => value,
        _ => {
            setup.error = Some("Chunk size must be a number from 1 to 10,000".to_string());
            app.blender_import_setup = Some(setup);
            return;
        }
    };
    setup.options.chunk_size = chunk_size;
    begin_blender_import(app, setup.source, setup.options);
}

fn begin_blender_import(app: &mut AppState, source: PathBuf, options: BlenderImportOptions) {
    let root = app.root.clone();
    let (tx, rx) = mpsc::channel();
    app.blender_import_rx = Some(rx);
    app.blender_import_dialog_open = true;
    app.blender_import_progress = 0.0;
    app.blender_import_phase = "Preparing Blender import".to_string();
    app.blender_import_log.clear();
    app.blender_import_log_scroll = 0.0;
    app.blender_import_log_follow_tail = true;
    app.blender_import_finished = false;
    push_blender_import_log(app, format!("Import source: {}", source.display()));
    push_blender_import_log(app, format!("Resource target: {}", root.display()));
    push_blender_import_log(
        app,
        format!(
            "Setup: chunk size {:.0}, split meshes {}, center origins {}, per-definition TXDs, collisions disabled",
            options.chunk_size, options.chunk_meshes, options.center_origins,
        ),
    );
    app.status_message = format!(
        "Importing {} with Blender (this may take a while)...",
        ellipsize(
            source
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("scene.blend"),
            42,
        )
    );
    thread::spawn(move || {
        let result = run_blender_import(&source, &root, &options, &tx);
        let _ = tx.send(BlenderImportUpdate::Finished(result));
    });
}

pub(crate) fn poll_blender_import(app: &mut AppState) {
    let Some(rx) = app.blender_import_rx.as_ref() else {
        return;
    };
    let mut updates = Vec::new();
    let mut disconnected = false;
    loop {
        match rx.try_recv() {
            Ok(update) => updates.push(update),
            Err(mpsc::TryRecvError::Empty) => break,
            Err(mpsc::TryRecvError::Disconnected) => {
                disconnected = true;
                break;
            }
        }
    }
    let mut finished = None;
    for update in updates {
        match update {
            BlenderImportUpdate::Progress { fraction, message } => {
                app.blender_import_progress =
                    app.blender_import_progress.max(fraction.clamp(0.0, 1.0));
                app.blender_import_phase = message;
            }
            BlenderImportUpdate::Log(line) => push_blender_import_log(app, line),
            BlenderImportUpdate::Finished(result) => finished = Some(result),
        }
    }
    let Some(result) = finished else {
        if disconnected {
            app.blender_import_rx = None;
            app.blender_import_finished = true;
            app.blender_import_phase = "Import stopped unexpectedly".to_string();
            push_blender_import_log(app, "ERROR: Blender import worker disconnected".to_string());
            app.status_message = "Blender import stopped unexpectedly".to_string();
        }
        return;
    };
    app.blender_import_rx = None;
    app.blender_import_finished = true;
    match result {
        Ok(summary) => {
            app.blender_import_progress = 1.0;
            app.blender_import_phase = "Import complete; reloading map".to_string();
            push_blender_import_log(
                app,
                format!(
                    "Complete: {} zone(s), {} DFF, {} TXD, {} warning(s)",
                    summary.zones.len(),
                    summary.dffs,
                    summary.txds,
                    summary.warnings.len(),
                ),
            );
            let root = app.root.clone();
            let meta_result =
                update_save_as_meta(app, &root).and_then(|_| ensure_blender_meta_entries(&root));
            if let Err(err) = meta_result {
                app.blender_import_phase =
                    "Imported assets, but project metadata failed".to_string();
                push_blender_import_log(app, format!("ERROR: Could not update meta.xml: {err}"));
                app.status_message = format!("Blender assets imported, but meta.xml failed: {err}");
                return;
            }
            app.status_message = format!(
                "Imported {} zone(s): {} DFF, {} TXD; {} warning(s). Reloading...",
                summary.zones.len(),
                summary.dffs,
                summary.txds,
                summary.warnings.len(),
            );
            app.pending_load_source = LoadSceneSource::Saved;
            app.pending_load_root = Some(root);
        }
        Err(err) => {
            app.blender_import_phase = "Import failed".to_string();
            push_blender_import_log(app, format!("ERROR: {err}"));
            app.status_message = format!("Blender import failed: {}", ellipsize(&err, 100));
        }
    }
}

fn push_blender_import_log(app: &mut AppState, line: String) {
    let line = line.trim_end_matches(['\r', '\n']).to_string();
    if line.is_empty() {
        return;
    }
    app.blender_import_log.push(line);
    if app.blender_import_log.len() > MAX_BLENDER_LOG_LINES {
        let remove = app.blender_import_log.len() - MAX_BLENDER_LOG_LINES;
        app.blender_import_log.drain(..remove);
    }
}

pub(crate) fn send_blender_progress(
    tx: &mpsc::Sender<BlenderImportUpdate>,
    fraction: f32,
    message: impl Into<String>,
) {
    let _ = tx.send(BlenderImportUpdate::Progress {
        fraction,
        message: message.into(),
    });
}

fn stream_blender_output<R: Read + Send + 'static>(
    stream: R,
    tx: mpsc::Sender<BlenderImportUpdate>,
    log_file: Arc<Mutex<fs::File>>,
) -> Vec<String> {
    let mut captured = Vec::new();
    for line in BufReader::new(stream).lines() {
        let Ok(line) = line else { continue };
        if let Ok(mut file) = log_file.lock() {
            let _ = writeln!(file, "{line}");
        }
        if let Some(payload) = line.strip_prefix(BLENDER_PROGRESS_PREFIX) {
            let mut parts = payload.splitn(2, '|');
            if let (Some(fraction), Some(message)) = (parts.next(), parts.next())
                && let Ok(fraction) = fraction.parse::<f32>()
            {
                send_blender_progress(&tx, fraction, message);
            }
            continue;
        }
        captured.push(line.clone());
        let _ = tx.send(BlenderImportUpdate::Log(line));
    }
    captured
}

fn run_blender_import(
    source: &Path,
    root: &Path,
    options: &BlenderImportOptions,
    tx: &mpsc::Sender<BlenderImportUpdate>,
) -> Result<BlenderImportSummary, String> {
    send_blender_progress(tx, 0.01, "Finding Blender");
    let blender = find_blender_executable()?;
    let _ = tx.send(BlenderImportUpdate::Log(format!(
        "Blender executable: {}",
        blender.display()
    )));
    fs::create_dir_all(root).map_err(|err| {
        format!(
            "Could not create resource directory {}: {err}",
            root.display()
        )
    })?;
    let temp_dir = env::temp_dir();
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let script_path = temp_dir.join(format!("eagle_import_blend_{nonce}.py"));
    let result_path = temp_dir.join(format!("eagle_import_blend_{nonce}.json"));
    let logs_dir = root.join("logs");
    fs::create_dir_all(&logs_dir)
        .map_err(|err| format!("Could not create import log directory: {err}"))?;
    let log_path = logs_dir.join("blender-import.log");
    fs::write(&script_path, BLENDER_IMPORT_SCRIPT)
        .map_err(|err| format!("Could not stage Blender import script: {err}"))?;

    let mut log_file = fs::File::create(&log_path)
        .map_err(|err| format!("Could not create Blender import log: {err}"))?;
    let _ = writeln!(log_file, "Eagle Editor Blender import");
    let _ = writeln!(log_file, "Source: {}", source.display());
    let _ = writeln!(log_file, "Target: {}", root.display());
    let _ = writeln!(log_file, "Blender: {}", blender.display());
    let _ = writeln!(log_file, "Chunk size: {}", options.chunk_size);
    let _ = writeln!(log_file, "Split meshes: {}", options.chunk_meshes);
    let _ = writeln!(log_file, "Center origins: {}", options.center_origins);
    let _ = writeln!(
        log_file,
        "Geometry build: Eagle Editor native slicer and DFF writer"
    );
    let _ = writeln!(log_file, "TXD assignment: Blender definition settings");
    let _ = writeln!(
        log_file,
        "Collision export: disabled (generated in Eagle Editor)"
    );
    let _ = writeln!(log_file);
    let log_file = Arc::new(Mutex::new(log_file));
    let _ = tx.send(BlenderImportUpdate::Log(format!(
        "Full log: {}",
        log_path.display()
    )));
    send_blender_progress(tx, 0.03, "Launching Blender");
    let import_options = serde_json::json!({
        "chunk_size": options.chunk_size,
        "chunk_meshes": options.chunk_meshes,
        "center_origins": options.center_origins,
    })
    .to_string();
    let mut child = Command::new(&blender)
        .arg("--background")
        .arg(source)
        .arg("--python")
        .arg(&script_path)
        .arg("--")
        .arg(root)
        .arg(&result_path)
        .arg(import_options)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|err| format!("Could not launch {}: {err}", blender.display()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Could not capture Blender output".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Could not capture Blender errors".to_string())?;
    let stdout_reader = {
        let tx = tx.clone();
        let log_file = Arc::clone(&log_file);
        thread::spawn(move || stream_blender_output(stdout, tx, log_file))
    };
    let stderr_reader = {
        let tx = tx.clone();
        let log_file = Arc::clone(&log_file);
        thread::spawn(move || stream_blender_output(stderr, tx, log_file))
    };
    let status = child
        .wait()
        .map_err(|err| format!("Blender process failed: {err}"));
    let mut captured = stdout_reader.join().unwrap_or_default();
    captured.extend(stderr_reader.join().unwrap_or_default());
    let _ = fs::remove_file(&script_path);
    let status = status?;
    let manifest = fs::read_to_string(&result_path).unwrap_or_default();
    let _ = fs::remove_file(&result_path);
    let manifest_json: serde_json::Value = serde_json::from_str(&manifest).unwrap_or_default();
    if !status.success() {
        let manifest_error = manifest_json["error"].as_str().unwrap_or_default();
        let tail = log_tail(&captured.join("\n"), 12);
        return Err(if manifest_error.is_empty() {
            format!("Blender exited with {status}: {tail}")
        } else {
            format!("{manifest_error}: {tail}")
        });
    }
    let geometry_path = manifest_json["native_geometry"]
        .as_str()
        .map(PathBuf::from)
        .ok_or_else(|| "Blender completed without a native geometry stream".to_string())?;
    send_blender_progress(
        tx,
        0.82,
        "Slicing evaluated meshes and building DFFs natively",
    );
    let native_build =
        crate::blender_native::build_native_blender_assets(&geometry_path, root, options, tx);
    let _ = fs::remove_file(&geometry_path);
    let native_build = native_build?;
    let zones = native_build.zones;
    let mut warnings: Vec<String> = manifest_json["warnings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().map(ToOwned::to_owned))
        .collect();
    warnings.extend(native_build.warnings);
    let txd_assignments = collect_imported_txd_assignments(root, &zones)?;
    let mut dff_entries = native_build.dff_entries;
    if dff_entries.is_empty() {
        return Err("Eagle's native Blender build did not generate any DFF files".to_string());
    }
    normalize_imported_dff_texture_references(&mut dff_entries)?;
    send_blender_progress(tx, 0.925, "Applying DFF structural repairs");
    apply_blender_dff_repairs(&mut dff_entries, &mut warnings)?;
    let txd_requirements = imported_txd_texture_requirements(&dff_entries, &txd_assignments)?;
    let texture_count = txd_requirements.values().map(BTreeSet::len).sum::<usize>();
    let txd_count = txd_requirements.len();
    let workers = thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .min(12)
        .min(texture_count.max(1));
    send_blender_progress(
        tx,
        0.93,
        format!(
            "Encoding {texture_count} texture(s) into {txd_count} TXD(s) with {workers} worker(s)"
        ),
    );
    let txd_started = Instant::now();
    let generated_txds =
        build_imported_txds(root, &txd_assignments, &txd_requirements, &mut warnings)?;
    let txd_elapsed = txd_started.elapsed().as_secs_f32();
    let txd_log = format!(
        "TXD build: encoded {texture_count} texture(s) into {txd_count} dictionary/dictionaries in {txd_elapsed:.2}s ({workers} worker(s))"
    );
    let _ = tx.send(BlenderImportUpdate::Log(txd_log.clone()));
    if let Ok(mut file) = log_file.lock() {
        let _ = writeln!(file, "{txd_log}");
    }
    validate_imported_dff_textures(
        &dff_entries,
        &txd_assignments,
        &generated_txds.texture_names,
    )?;
    let txds = generated_txds.entries.len();

    let imgs = root.join("imgs");
    fs::create_dir_all(&imgs).map_err(|err| format!("{}: {err}", imgs.display()))?;
    validate_blender_archive_names(&dff_entries, BLENDER_DFF_IMG)?;
    validate_blender_archive_names(&generated_txds.entries, BLENDER_TXD_IMG)?;
    send_blender_progress(tx, 0.96, "Building imgs/dff.img");
    write_img_archive(&imgs.join(BLENDER_DFF_IMG), &dff_entries)?;
    send_blender_progress(tx, 0.98, "Building imgs/txd.img");
    write_img_archive(&imgs.join(BLENDER_TXD_IMG), &generated_txds.entries)?;
    remove_imported_loose_txds(root, &txd_assignments)?;
    let legacy_dff_archive = imgs.join(LEGACY_BLENDER_DFF_IMG);
    if legacy_dff_archive.is_file() {
        fs::remove_file(&legacy_dff_archive).map_err(|err| {
            format!(
                "Could not remove obsolete Blender model archive {}: {err}",
                legacy_dff_archive.display()
            )
        })?;
    }
    let legacy_collision_archive = imgs.join(LEGACY_BLENDER_COL_IMG);
    if legacy_collision_archive.is_file() {
        fs::remove_file(&legacy_collision_archive).map_err(|err| {
            format!(
                "Could not remove obsolete Blender collision archive {}: {err}",
                legacy_collision_archive.display()
            )
        })?;
    }
    send_blender_progress(tx, 0.99, "Finalizing imported map");

    Ok(BlenderImportSummary {
        zones,
        dffs: dff_entries.len(),
        txds,
        warnings,
    })
}

fn validate_blender_archive_names(
    entries: &[(String, Vec<u8>)],
    archive_name: &str,
) -> Result<(), String> {
    let mut names = HashSet::new();
    for (name, _) in entries {
        if name.as_bytes().len() > IMG_RUNTIME_SAFE_ENTRY_NAME_BYTES {
            return Err(format!(
                "{archive_name}: generated entry {name} is too long for the IMG directory"
            ));
        }
        if !names.insert(lower(name)) {
            return Err(format!(
                "{archive_name}: Blender import generated duplicate entry name {name}"
            ));
        }
    }
    Ok(())
}

fn find_blender_executable() -> Result<PathBuf, String> {
    if let Some(install_dir) = load_blender_install_dir_preference()
        && let Some(executable) = blender_executable_in_install_dir(&install_dir)
    {
        return Ok(executable);
    }
    if let Some(path) = env::var_os("BLENDER_PATH").map(PathBuf::from) {
        if path.is_file() {
            return Ok(path);
        }
    }
    if Command::new("blender")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
    {
        return Ok(PathBuf::from("blender"));
    }
    common_blender_executable_candidates()
        .into_iter()
        .next()
        .ok_or_else(|| {
            "Blender was not found. Set its install directory in Preferences or set BLENDER_PATH to its executable.".to_string()
        })
}

fn empty_txd() -> Vec<u8> {
    rw_chunk(0x16, rw_chunk(0x01, vec![0, 0, 0, 0]))
}

fn normalized_txd_stem(value: &str) -> String {
    let trimmed = value.trim();
    if Path::new(trimmed)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("txd"))
    {
        Path::new(trimmed)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("texture")
            .to_string()
    } else if trimmed.is_empty() {
        "texture".to_string()
    } else {
        trimmed.to_string()
    }
}

fn collect_imported_txd_assignments(
    root: &Path,
    zones: &[String],
) -> Result<BTreeMap<String, String>, String> {
    let definition_tag = Regex::new(r#"<definition\b[^>]*>"#).expect("static definition regex");
    let dff_attr = Regex::new(r#"\bdff="([^"]+)""#).expect("static DFF attribute regex");
    let txd_attr = Regex::new(r#"\btxd="([^"]+)""#).expect("static TXD attribute regex");
    let mut assignments = BTreeMap::new();
    for zone in zones {
        let path = root
            .join("zones")
            .join(zone)
            .join(format!("{zone}.definition"));
        let text = fs::read_to_string(&path).map_err(|err| {
            format!(
                "Could not read TXD assignments from {}: {err}",
                path.display()
            )
        })?;
        for tag in definition_tag.find_iter(&text).map(|value| value.as_str()) {
            let Some(dff) = dff_attr
                .captures(tag)
                .and_then(|capture| capture.get(1))
                .map(|value| asset_key(value.as_str(), ".dff"))
            else {
                continue;
            };
            let txd = txd_attr
                .captures(tag)
                .and_then(|capture| capture.get(1))
                .map(|value| normalized_txd_stem(value.as_str()))
                .unwrap_or_else(|| "texture".to_string());
            if let Some(existing) = assignments.insert(dff.clone(), txd.clone())
                && !existing.eq_ignore_ascii_case(&txd)
            {
                return Err(format!(
                    "{dff} is assigned to both {existing}.txd and {txd}.txd"
                ));
            }
        }
    }
    if assignments.is_empty() {
        return Err("Blender import generated no DFF/TXD definition assignments".to_string());
    }
    Ok(assignments)
}

fn source_folder_has_png(source_dir: &Path) -> bool {
    source_dir.is_dir()
        && WalkDir::new(source_dir)
            .into_iter()
            .filter_map(Result::ok)
            .any(|entry| {
                entry.file_type().is_file()
                    && entry
                        .path()
                        .extension()
                        .and_then(|value| value.to_str())
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
            })
}

struct ImportedTxdBuild {
    texture_names: BTreeMap<String, HashSet<String>>,
    entries: Vec<(String, Vec<u8>)>,
}

fn imported_txd_texture_requirements(
    dff_entries: &[(String, Vec<u8>)],
    assignments: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, BTreeSet<String>>, String> {
    let mut requirements = BTreeMap::<String, BTreeSet<String>>::new();
    for (dff_name, bytes) in dff_entries {
        let dff_key = asset_key(dff_name, ".dff");
        let txd_name = assignments
            .get(&dff_key)
            .ok_or_else(|| format!("{dff_name}: imported DFF has no TXD assignment"))?;
        requirements
            .entry(txd_name.to_ascii_lowercase())
            .or_default()
            .extend(
                parse_dff_mesh(bytes)
                    .material_textures
                    .into_iter()
                    .filter(|name| !name.trim().is_empty())
                    .map(|name| name.to_ascii_lowercase()),
            );
    }
    Ok(requirements)
}

fn build_imported_txds(
    root: &Path,
    assignments: &BTreeMap<String, String>,
    requirements: &BTreeMap<String, BTreeSet<String>>,
    warnings: &mut Vec<String>,
) -> Result<ImportedTxdBuild, String> {
    let assigned = assignments
        .values()
        .map(|name| (name.to_ascii_lowercase(), name.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut prepared = Vec::new();
    for (key, txd_name) in assigned {
        let source_dir = root.join("txd_build").join(&txd_name);
        let required = requirements.get(&key).cloned().unwrap_or_default();
        let mut selected = if source_folder_has_png(&source_dir) {
            WalkDir::new(&source_dir)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|entry| entry.file_type().is_file())
                .map(|entry| entry.into_path())
                .filter(|path| {
                    path.extension()
                        .and_then(|ext| ext.to_str())
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
                        && required.contains(&texture_name_from_path(path).to_ascii_lowercase())
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        selected.sort();
        let selected_names = selected
            .iter()
            .map(|path| texture_name_from_path(path).to_ascii_lowercase())
            .collect::<HashSet<_>>();
        let missing_sources = required
            .iter()
            .filter(|name| !selected_names.contains(*name))
            .cloned()
            .collect::<Vec<_>>();
        if !missing_sources.is_empty() {
            return Err(format!(
                "{txd_name}.txd: Blender did not export PNG source(s) for DFF texture(s): {}",
                missing_sources.join(", ")
            ));
        }

        let bytes = if selected.is_empty() {
            warnings.push(format!(
                "{txd_name}.txd: no assigned DFF textures were exported; created an empty TXD"
            ));
            empty_txd()
        } else {
            build_txd_from_paths(&format!("{txd_name}.txd"), &source_dir, &selected)?.1
        };
        let names = parse_txd_texture_contents(&bytes)?
            .into_iter()
            .map(|texture| texture.name.to_ascii_lowercase())
            .collect::<HashSet<_>>();
        let missing_generated = required
            .iter()
            .filter(|name| !names.contains(*name))
            .cloned()
            .collect::<Vec<_>>();
        if !missing_generated.is_empty() {
            return Err(format!(
                "{txd_name}.txd: Eagle's generated TXD is missing DFF texture(s): {}",
                missing_generated.join(", ")
            ));
        }
        prepared.push((key, txd_name, bytes, names));
    }

    let mut texture_names = BTreeMap::new();
    let mut entries = Vec::new();
    for (key, txd_name, bytes, names) in prepared {
        texture_names.insert(key, names);
        entries.push((format!("{txd_name}.txd"), bytes));
    }
    Ok(ImportedTxdBuild {
        texture_names,
        entries,
    })
}

fn remove_imported_loose_txds(
    root: &Path,
    assignments: &BTreeMap<String, String>,
) -> Result<(), String> {
    let texture_dir = root.join("textures");
    for txd_name in assignments
        .values()
        .map(|name| normalized_txd_stem(name))
        .collect::<BTreeSet<_>>()
    {
        let path = texture_dir.join(format!("{txd_name}.txd"));
        if path.is_file() {
            fs::remove_file(&path).map_err(|err| {
                format!(
                    "Could not remove archived loose TXD {}: {err}",
                    path.display()
                )
            })?;
        }
    }
    Ok(())
}

fn normalize_imported_dff_texture_references(
    dff_entries: &mut [(String, Vec<u8>)],
) -> Result<(), String> {
    for (dff_name, bytes) in dff_entries {
        let mesh = parse_dff_mesh(bytes);
        let renames = mesh
            .material_textures
            .iter()
            .filter(|name| !name.trim().is_empty())
            .filter_map(|name| {
                let sanitized = sanitize_texture_name(name);
                (!name.eq_ignore_ascii_case(&sanitized))
                    .then(|| (name.to_ascii_lowercase(), sanitized))
            })
            .collect::<HashMap<_, _>>();
        if renames.is_empty() {
            continue;
        }
        let (updated, _) = rewrite_dff_material_textures(bytes, &renames)
            .map_err(|err| format!("{dff_name}: could not normalize texture references: {err}"))?;
        *bytes = updated;
    }
    Ok(())
}

/// Apply the same targeted, content-preserving structural repairs used by
/// Validation before Blender-generated DFFs reach the resource archive. The
/// native writer should already satisfy these invariants; this pass is both a
/// safety net and a final verification boundary for future importer changes.
fn apply_blender_dff_repairs(
    dff_entries: &mut [(String, Vec<u8>)],
    warnings: &mut Vec<String>,
) -> Result<(), String> {
    let mut prelight_streams = 0usize;
    let mut bin_mesh_batches = 0usize;
    let mut lighting_flags = 0usize;
    let mut bounds = 0usize;
    let mut uv_dictionaries = 0usize;
    let mut uv_pipelines = 0usize;
    for (dff_name, bytes) in dff_entries {
        if let Some(fixed) = repair_dff_missing_prelight_streams(bytes)
            .map_err(|error| format!("{dff_name}: prelight repair failed: {error}"))?
        {
            *bytes = fixed;
            prelight_streams += 1;
        }
        if let Some((fixed, removed)) = canonicalize_dff_bin_mesh_batches(bytes)
            .map_err(|error| format!("{dff_name}: BinMesh repair failed: {error}"))?
        {
            *bytes = fixed;
            bin_mesh_batches += removed;
        }
        lighting_flags += repair_dff_normal_less_lighting_flags(bytes)
            .map_err(|error| format!("{dff_name}: lighting-flag repair failed: {error}"))?;
        if let Some(fixed) = repair_dff_uv_anim_dictionary_order(bytes) {
            *bytes = fixed;
            uv_dictionaries += 1;
        }
        if let Some(fixed) = repair_dff_uv_anim_right_to_render(bytes) {
            *bytes = fixed;
            uv_pipelines += 1;
        }
        bounds += repair_dff_bounds_spheres(bytes);

        let material_count = max_dff_geometry_material_count(bytes)
            .map_err(|error| format!("{dff_name}: material audit failed: {error}"))?;
        if material_count > GTA_DFF_MATERIAL_LIMIT {
            return Err(format!(
                "{dff_name}: generated geometry has {material_count} materials; limit is {GTA_DFF_MATERIAL_LIMIT}"
            ));
        }
        let redundant = dff_redundant_bin_mesh_batch_count(bytes)
            .map_err(|error| format!("{dff_name}: BinMesh verification failed: {error}"))?;
        let invalid_lighting = dff_normal_less_lit_geometry_count(bytes)
            .map_err(|error| format!("{dff_name}: lighting verification failed: {error}"))?;
        let (invalid, issues) = dff_geometry_issue_summary(bytes);
        if invalid || redundant != 0 || invalid_lighting != 0 {
            let mut failures = issues;
            if redundant != 0 {
                failures.push(format!("{redundant} redundant BinMesh batch(es)"));
            }
            if invalid_lighting != 0 {
                failures.push(format!(
                    "{invalid_lighting} normal-less lit geometry section(s)"
                ));
            }
            return Err(format!(
                "{dff_name}: generated DFF failed post-repair validation ({})",
                failures.join(", ")
            ));
        }
    }
    let repaired = prelight_streams
        + bin_mesh_batches
        + lighting_flags
        + bounds
        + uv_dictionaries
        + uv_pipelines;
    if repaired != 0 {
        warnings.push(format!(
            "Final DFF repair pass filled {prelight_streams} prelight stream(s), removed {bin_mesh_batches} redundant BinMesh batch(es), corrected {lighting_flags} lighting flag(s), repaired {bounds} bounds sphere(s), reordered {uv_dictionaries} UV dictionary/dictionaries, and added {uv_pipelines} UV pipeline marker(s)"
        ));
    }
    Ok(())
}

fn validate_imported_dff_textures(
    dff_entries: &[(String, Vec<u8>)],
    assignments: &BTreeMap<String, String>,
    generated_txds: &BTreeMap<String, HashSet<String>>,
) -> Result<(), String> {
    for (dff_name, bytes) in dff_entries {
        let dff_key = asset_key(dff_name, ".dff");
        let txd_name = assignments
            .get(&dff_key)
            .ok_or_else(|| format!("{dff_name}: imported DFF has no TXD assignment"))?;
        let available = generated_txds
            .get(&txd_name.to_ascii_lowercase())
            .ok_or_else(|| format!("{dff_name}: assigned TXD {txd_name}.txd was not generated"))?;
        let missing = parse_dff_mesh(bytes)
            .material_textures
            .into_iter()
            .filter(|name| !name.trim().is_empty())
            .filter(|name| !available.contains(&name.to_ascii_lowercase()))
            .collect::<BTreeSet<_>>();
        if !missing.is_empty() {
            return Err(format!(
                "{dff_name}: {}.txd is missing referenced texture(s): {}",
                txd_name,
                missing.into_iter().collect::<Vec<_>>().join(", ")
            ));
        }
    }
    Ok(())
}

fn ensure_blender_meta_entries(root: &Path) -> Result<(), String> {
    let path = root.join("meta.xml");
    let mut text = fs::read_to_string(&path).unwrap_or_else(|_| "<meta>\n</meta>\n".to_string());
    let has_loose_txds = WalkDir::new(root.join("textures"))
        .into_iter()
        .filter_map(Result::ok)
        .any(|entry| {
            entry.file_type().is_file()
                && entry
                    .path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("txd"))
        });
    text = text
        .lines()
        .filter(|line| {
            !line.contains("src=\"textures/textures.txd\"")
                && (has_loose_txds || !line.contains("src=\"textures/*.txd\""))
        })
        .collect::<Vec<_>>()
        .join("\n");
    if !text.contains("src=\"imgs/*.img\"") {
        let entry = "    <file src=\"imgs/*.img\" type=\"client\" />\n";
        if let Some(at) = text.rfind("</meta>") {
            text.insert_str(at, entry);
        } else {
            text.push_str("\n<meta>\n");
            text.push_str(entry);
            text.push_str("</meta>\n");
        }
    }
    fs::write(&path, text).map_err(|err| format!("{}: {err}", path.display()))
}

fn log_tail(log: &str, lines: usize) -> String {
    let rows: Vec<_> = log.lines().filter(|line| !line.trim().is_empty()).collect();
    rows[rows.len().saturating_sub(lines)..].join(" | ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blender_slicer_defaults_to_200_units() {
        assert_eq!(BlenderImportOptions::default().chunk_size, 200.0);
    }

    fn assignment_test_root(label: &str) -> PathBuf {
        env::temp_dir().join(format!(
            "eagle_blender_assignment_{label}_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ))
    }

    #[test]
    fn empty_txd_accepts_texture_native_entries() {
        let rgba = vec![255u8; 4 * 4 * 4];
        let native = texture_native_from_rgba(&rgba, 4, 4, "brick");
        let txd = append_texture_native_to_txd(empty_txd(), &native, "brick").unwrap();
        assert!(txd_contains_texture_native(&txd, "brick"));
        assert_eq!(txd_texture_count(&txd, 12, txd.len()), Some(1));
    }

    #[test]
    fn log_tail_keeps_only_requested_lines() {
        assert_eq!(log_tail("one\ntwo\nthree\n", 2), "two | three");
    }

    #[test]
    fn blender_archives_use_editor_standard_names() {
        assert_eq!(BLENDER_DFF_IMG, "dff.img");
        assert_eq!(BLENDER_TXD_IMG, "txd.img");
    }

    #[test]
    fn blender_archive_validation_rejects_duplicate_generated_names() {
        let entries = vec![
            ("model.dff".to_string(), vec![1]),
            ("MODEL.DFF".to_string(), vec![2]),
        ];

        let error = validate_blender_archive_names(&entries, "dff.img").unwrap_err();

        assert!(error.contains("duplicate entry name MODEL.DFF"));
    }

    #[test]
    fn imported_txd_is_packed_without_a_loose_copy() {
        let root = assignment_test_root("archived_txd");
        fs::create_dir_all(root.join("imgs")).unwrap();
        let assignments = BTreeMap::from([("road.dff".to_string(), "roads".to_string())]);
        let requirements = BTreeMap::from([("roads".to_string(), BTreeSet::<String>::new())]);
        let mut warnings = Vec::new();
        let generated =
            build_imported_txds(&root, &assignments, &requirements, &mut warnings).unwrap();
        assert_eq!(generated.entries.len(), 1);
        assert_eq!(generated.entries[0].0, "roads.txd");
        assert!(!root.join("textures/roads.txd").exists());

        let archive = root.join("imgs/txd.img");
        write_img_archive(&archive, &generated.entries).unwrap();
        assert_eq!(
            parse_img(&archive)
                .into_iter()
                .map(|entry| entry.name)
                .collect::<Vec<_>>(),
            vec!["roads.txd"]
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn imported_definitions_preserve_assorted_txd_assignments() {
        let root = assignment_test_root("matching");
        let zone_dir = root.join("zones/city");
        fs::create_dir_all(&zone_dir).unwrap();
        fs::write(
            zone_dir.join("city.definition"),
            "<zoneDefinitions>\n<definition id=\"a\" dff=\"road\" txd=\"texture\"></definition>\n<definition id=\"b\" dff=\"tower\" txd=\"city_custom.txd\"></definition>\n<definition id=\"c\" dff=\"plain\"></definition>\n</zoneDefinitions>",
        )
        .unwrap();
        let assignments = collect_imported_txd_assignments(&root, &["city".to_string()]).unwrap();
        assert_eq!(assignments.get("road.dff").unwrap(), "texture");
        assert_eq!(assignments.get("tower.dff").unwrap(), "city_custom");
        assert_eq!(assignments.get("plain.dff").unwrap(), "texture");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn conflicting_imported_txd_assignments_are_rejected() {
        let root = assignment_test_root("mismatch");
        let zone_dir = root.join("zones/city");
        fs::create_dir_all(&zone_dir).unwrap();
        fs::write(
            zone_dir.join("city.definition"),
            "<zoneDefinitions><definition id=\"a\" dff=\"shared\" txd=\"first\"></definition><definition id=\"b\" dff=\"shared\" txd=\"second\"></definition></zoneDefinitions>",
        )
        .unwrap();
        let error = collect_imported_txd_assignments(&root, &["city".to_string()]).unwrap_err();
        assert!(error.contains("assigned to both first.txd and second.txd"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn blender_finalizer_applies_validation_dff_repairs() {
        fn add_lighting_flag(bytes: &mut [u8], start: usize, end: usize) -> bool {
            let mut chunk = start;
            while chunk + 12 <= end {
                let id = rd32(bytes, chunk);
                let size = rd32(bytes, chunk + 4) as usize;
                let data = chunk + 12;
                let chunk_end = data + size;
                if id == 0x0f {
                    let mut child = data;
                    while child + 12 <= chunk_end {
                        let child_id = rd32(bytes, child);
                        let child_size = rd32(bytes, child + 4) as usize;
                        let child_data = child + 12;
                        let child_end = child_data + child_size;
                        if child_id == 0x01 {
                            let flags = rd32(bytes, child_data);
                            bytes[child_data..child_data + 4]
                                .copy_from_slice(&(flags | 0x20).to_le_bytes());
                            return true;
                        }
                        child = child_end;
                    }
                } else if matches!(id, 0x10 | 0x0e | 0x1a)
                    && add_lighting_flag(bytes, data, chunk_end)
                {
                    return true;
                }
                chunk = chunk_end;
            }
            false
        }

        let raw = RawMesh {
            vertices: vec![
                V3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                V3 {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
                V3 {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
            ],
            normals: vec![
                V3 {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                };
                3
            ],
            triangles: vec![Tri {
                a: 0,
                b: 1,
                c: 2,
                material: 0,
            }],
            ..RawMesh::default()
        };
        let mut dff = write_normalized_dff_with_options(
            &raw,
            "building",
            DffWriteOptions {
                include_normals: false,
                include_bin_mesh: true,
            },
        )
        .unwrap();
        let end = dff_chunk_len(&dff);
        assert!(add_lighting_flag(&mut dff, 0, end));
        assert_eq!(dff_normal_less_lit_geometry_count(&dff), Ok(1));
        let mut entries = vec![("building.dff".to_string(), dff)];
        let mut warnings = Vec::new();

        apply_blender_dff_repairs(&mut entries, &mut warnings).unwrap();

        assert_eq!(dff_normal_less_lit_geometry_count(&entries[0].1), Ok(0));
        assert_eq!(dff_redundant_bin_mesh_batch_count(&entries[0].1), Ok(0));
        assert!(!dff_geometry_issue_summary(&entries[0].1).0);
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("corrected 1 lighting flag"))
        );
    }
}
