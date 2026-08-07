use super::*;

const BLENDER_IMPORT_SCRIPT: &str = include_str!("../assets/import_blend.py");
const BLENDER_DFF_IMG: &str = "blender_dff.img";
const BLENDER_COL_IMG: &str = "blender_col.img";

#[derive(Clone, Debug)]
pub(crate) struct BlenderImportSummary {
    pub(crate) zones: Vec<String>,
    pub(crate) dffs: usize,
    pub(crate) cols: usize,
    pub(crate) txds: usize,
    pub(crate) custom_collisions: usize,
    pub(crate) warnings: Vec<String>,
}

pub(crate) fn open_blender_import_dialog(app: &mut AppState) {
    if app.blender_import_rx.is_some() {
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
    if app.blender_import_rx.is_some() {
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
    let root = app.root.clone();
    let (tx, rx) = mpsc::channel();
    app.blender_import_rx = Some(rx);
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
        let _ = tx.send(run_blender_import(&source, &root));
    });
}

pub(crate) fn poll_blender_import(app: &mut AppState) {
    let Some(rx) = app.blender_import_rx.as_ref() else {
        return;
    };
    let result = match rx.try_recv() {
        Ok(result) => result,
        Err(mpsc::TryRecvError::Empty) => return,
        Err(mpsc::TryRecvError::Disconnected) => {
            app.blender_import_rx = None;
            app.status_message = "Blender import stopped unexpectedly".to_string();
            return;
        }
    };
    app.blender_import_rx = None;
    match result {
        Ok(summary) => {
            let root = app.root.clone();
            let meta_result =
                update_save_as_meta(app, &root).and_then(|_| ensure_blender_meta_entries(&root));
            if let Err(err) = meta_result {
                app.status_message = format!("Blender assets imported, but meta.xml failed: {err}");
                return;
            }
            app.status_message = format!(
                "Imported {} zone(s): {} DFF, {} COL, {} TXD, {} custom COL; {} warning(s). Reloading...",
                summary.zones.len(),
                summary.dffs,
                summary.cols,
                summary.txds,
                summary.custom_collisions,
                summary.warnings.len(),
            );
            app.pending_load_source = LoadSceneSource::Saved;
            app.pending_load_root = Some(root);
        }
        Err(err) => {
            app.status_message = format!("Blender import failed: {}", ellipsize(&err, 100));
        }
    }
}

fn run_blender_import(source: &Path, root: &Path) -> Result<BlenderImportSummary, String> {
    let blender = find_blender_executable()?;
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
    let log_path = temp_dir.join(format!("eagle_import_blend_{nonce}.log"));
    fs::write(&script_path, BLENDER_IMPORT_SCRIPT)
        .map_err(|err| format!("Could not stage Blender import script: {err}"))?;

    let log_file = fs::File::create(&log_path)
        .map_err(|err| format!("Could not create Blender import log: {err}"))?;
    let log_stderr = log_file
        .try_clone()
        .map_err(|err| format!("Could not open Blender import log: {err}"))?;
    let status = Command::new(&blender)
        .arg("--background")
        .arg(source)
        .arg("--python")
        .arg(&script_path)
        .arg("--")
        .arg(root)
        .arg(&result_path)
        .stdout(std::process::Stdio::from(log_file))
        .stderr(std::process::Stdio::from(log_stderr))
        .status()
        .map_err(|err| format!("Could not launch {}: {err}", blender.display()));

    let _ = fs::remove_file(&script_path);
    let status = status?;
    let manifest = fs::read_to_string(&result_path).unwrap_or_default();
    let _ = fs::remove_file(&result_path);
    let manifest_json: serde_json::Value = serde_json::from_str(&manifest).unwrap_or_default();
    if !status.success() {
        let manifest_error = manifest_json["error"].as_str().unwrap_or_default();
        let logs = read_log_end(&log_path, 64 * 1024).unwrap_or_default();
        let tail = log_tail(&logs, 12);
        let _ = fs::remove_file(&log_path);
        return Err(if manifest_error.is_empty() {
            format!("Blender exited with {status}: {tail}")
        } else {
            format!("{manifest_error}: {tail}")
        });
    }
    let _ = fs::remove_file(&log_path);

    let zones: Vec<String> = manifest_json["zones"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().map(ToOwned::to_owned))
        .collect();
    if zones.is_empty() {
        return Err("Blender completed without producing any collection zones".to_string());
    }
    let mut warnings: Vec<String> = manifest_json["warnings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().map(ToOwned::to_owned))
        .collect();

    let dff_entries = collect_imported_assets(root, &zones, "dff")?;
    let col_entries = collect_imported_assets(root, &zones, "col")?;
    if dff_entries.is_empty() {
        return Err("RRW:MTA did not generate any DFF files".to_string());
    }
    let imgs = root.join("imgs");
    fs::create_dir_all(&imgs).map_err(|err| format!("{}: {err}", imgs.display()))?;
    write_img_archive(&imgs.join(BLENDER_DFF_IMG), &dff_entries)?;
    write_img_archive(&imgs.join(BLENDER_COL_IMG), &col_entries)?;
    let txds = build_imported_txds(root, &zones, &mut warnings)?;

    Ok(BlenderImportSummary {
        zones,
        dffs: dff_entries.len(),
        cols: col_entries.len(),
        txds,
        custom_collisions: manifest_json["custom_collisions"].as_u64().unwrap_or(0) as usize,
        warnings,
    })
}

fn find_blender_executable() -> Result<PathBuf, String> {
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
    let mut candidates = vec![
        PathBuf::from("/usr/bin/blender"),
        PathBuf::from("/usr/local/bin/blender"),
        PathBuf::from("/opt/blender/blender"),
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
                {
                    if let Ok(versions) = fs::read_dir(path) {
                        for version in versions.filter_map(Result::ok) {
                            candidates.push(version.path().join("blender.exe"));
                        }
                    }
                }
            }
        }
    }
    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        for base in [
            home.join("Utilities"),
            home.join("Applications"),
            home.join(".local/bin"),
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
                            path.join("blender")
                        } else {
                            path
                        });
                    }
                }
            }
        }
    }
    candidates.sort();
    candidates.reverse();
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .ok_or_else(|| {
            "Blender was not found. Install Blender or set BLENDER_PATH to its executable."
                .to_string()
        })
}

fn collect_imported_assets(
    root: &Path,
    zones: &[String],
    extension: &str,
) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut entries = BTreeMap::<String, Vec<u8>>::new();
    for zone in zones {
        let dir = root.join("zones").join(zone).join(extension);
        if !dir.is_dir() {
            continue;
        }
        for item in WalkDir::new(&dir).into_iter().filter_map(Result::ok) {
            if !item.file_type().is_file()
                || !item
                    .path()
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case(extension))
            {
                continue;
            }
            let name = item
                .path()
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or_else(|| format!("Invalid asset filename: {}", item.path().display()))?
                .to_string();
            if name.as_bytes().len() > IMG_RUNTIME_SAFE_ENTRY_NAME_BYTES {
                return Err(format!(
                    "{name}: IMG asset names must leave room for a terminator in the 24-byte directory field"
                ));
            }
            let bytes =
                fs::read(item.path()).map_err(|err| format!("{}: {err}", item.path().display()))?;
            let key = name.to_ascii_lowercase();
            if let Some(existing) = entries.get(&key) {
                if existing != &bytes {
                    return Err(format!(
                        "Conflicting {name} assets were generated in multiple collection zones"
                    ));
                }
            } else {
                entries.insert(key, bytes);
            }
        }
    }
    Ok(entries
        .into_iter()
        .map(|(name, bytes)| (name, bytes))
        .collect())
}

fn empty_txd() -> Vec<u8> {
    rw_chunk(0x16, rw_chunk(0x01, vec![0, 0, 0, 0]))
}

fn build_imported_txds(
    root: &Path,
    zones: &[String],
    warnings: &mut Vec<String>,
) -> Result<usize, String> {
    let build_root = root.join("txd_build");
    let mut groups = BTreeMap::<String, Vec<PathBuf>>::new();
    for zone in zones {
        groups.entry(zone.clone()).or_default();
    }
    if build_root.is_dir() {
        for entry in fs::read_dir(&build_root)
            .map_err(|err| format!("{}: {err}", build_root.display()))?
            .filter_map(Result::ok)
        {
            let dir = entry.path();
            if !dir.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            let images = groups.entry(name).or_default();
            for image in WalkDir::new(&dir).into_iter().filter_map(Result::ok) {
                if image.file_type().is_file()
                    && image
                        .path()
                        .extension()
                        .and_then(|value| value.to_str())
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
                {
                    images.push(image.path().to_path_buf());
                }
            }
        }
    }
    let texture_dir = root.join("textures");
    fs::create_dir_all(&texture_dir).map_err(|err| format!("{}: {err}", texture_dir.display()))?;
    for (txd_name, images) in &mut groups {
        images.sort();
        let mut txd = empty_txd();
        let mut used = HashSet::new();
        for path in images.iter() {
            let texture_name = path
                .file_stem()
                .and_then(|value| value.to_str())
                .map(sanitize_texture_name)
                .unwrap_or_else(|| "texture".to_string());
            if !used.insert(texture_name.to_ascii_lowercase()) {
                warnings.push(format!(
                    "{txd_name}.txd: duplicate texture name {texture_name}; kept the first"
                ));
                continue;
            }
            let native = imported_image_texture_native(path, &texture_name)?;
            txd = append_texture_native_to_txd(txd, &native, &texture_name)?;
        }
        let clean_name = txd_name.trim_end_matches(".txd");
        fs::write(texture_dir.join(format!("{clean_name}.txd")), txd)
            .map_err(|err| format!("Could not write {clean_name}.txd: {err}"))?;
    }
    Ok(groups.len())
}

fn ensure_blender_meta_entries(root: &Path) -> Result<(), String> {
    let path = root.join("meta.xml");
    let mut text = fs::read_to_string(&path).unwrap_or_else(|_| "<meta>\n</meta>\n".to_string());
    text = text
        .lines()
        .filter(|line| !line.contains("src=\"textures/textures.txd\""))
        .collect::<Vec<_>>()
        .join("\n");
    if !text.contains("src=\"textures/*.txd\"") {
        let entry = "    <file src=\"textures/*.txd\" type=\"client\" />\n";
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

fn read_log_end(path: &Path, max_bytes: u64) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(|err| format!("{}: {err}", path.display()))?;
    let len = file.metadata().map(|meta| meta.len()).unwrap_or(0);
    file.seek(SeekFrom::Start(len.saturating_sub(max_bytes)))
        .map_err(|err| format!("{}: {err}", path.display()))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|err| format!("{}: {err}", path.display()))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
