use super::super::*;

pub(crate) fn add_texture_alias(files: &mut HashMap<String, PathBuf>, key: &str, path: &Path) {
    let key = lower(key);
    if !key.is_empty() {
        files
            .entry(key.clone())
            .or_insert_with(|| path.to_path_buf());
    }
    let without_seq = key.strip_suffix(".001").unwrap_or(&key);
    files
        .entry(without_seq.to_string())
        .or_insert_with(|| path.to_path_buf());
    for ext in [".bmp", ".png", ".tga", ".jpg"] {
        if let Some(base) = without_seq.strip_suffix(ext) {
            if !base.is_empty() {
                files
                    .entry(base.to_string())
                    .or_insert_with(|| path.to_path_buf());
            }
        }
    }
}

pub(crate) fn collect_texture_files(root: &Path) -> HashMap<String, PathBuf> {
    collect_texture_files_from_roots([root.to_path_buf(), wip_root_path(root)])
}

pub(crate) fn collect_scene_texture_files(
    root: &Path,
    source: LoadSceneSource,
) -> HashMap<String, PathBuf> {
    let overlay_root = match source {
        LoadSceneSource::Saved => wip_root_path(root),
        LoadSceneSource::Autosave => autosave_root_path(root),
    };
    collect_texture_files_from_roots([root.to_path_buf(), overlay_root])
}

pub(crate) fn collect_texture_files_from_roots<const N: usize>(
    roots: [PathBuf; N],
) -> HashMap<String, PathBuf> {
    let mut files = HashMap::new();
    for root in roots {
        let texture_root = root.join("txd_build");
        if !texture_root.exists() {
            continue;
        }
        for entry in WalkDir::new(texture_root)
            .into_iter()
            .filter_map(Result::ok)
        {
            if entry.file_type().is_file()
                && lower(
                    entry
                        .path()
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or(""),
                ) == "png"
            {
                if let Some(stem) = entry.path().file_stem().and_then(|s| s.to_str()) {
                    add_texture_alias(&mut files, stem, entry.path());
                }
            }
        }
    }
    files
}
