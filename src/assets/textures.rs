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
    let mut files = collect_texture_files_from_roots([root.to_path_buf(), overlay_root]);
    collect_map_lightmaps(root, &mut files);
    collect_material_plugins(root, &mut files);
    files
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

// Resolve resource-owned UV1 atlases alongside the opened map. These aliases
// are separate from diffuse texture keys, so ordinary TXD lookup is unchanged.
fn collect_map_lightmaps(root: &Path, files: &mut HashMap<String, PathBuf>) {
    let Ok(xml) = fs::read_to_string(root.join("eagleLightMaps.xml")) else {
        return;
    };
    let attrs = Regex::new(r#"([A-Za-z_][A-Za-z0-9_]*)\s*=\s*"([^"]*)""#).unwrap();
    let header = Regex::new(r"<eagleLightMaps\b([^>]*)>").unwrap();
    let Some(cap) = header.captures(&xml) else {
        return;
    };
    let properties = parse_attrs(&cap[1], &attrs);
    if properties.get("uvSet").map(String::as_str) != Some("1") {
        return;
    }
    let Ok(resource_root) = root.canonicalize() else {
        return;
    };
    let bindings = Regex::new(r"<lightmap\b([^>]*)/?>").unwrap();
    for cap in bindings.captures_iter(&xml) {
        let properties = parse_attrs(&cap[1], &attrs);
        let (Some(texture), Some(file)) = (properties.get("texture"), properties.get("file"))
        else {
            continue;
        };
        let path = Path::new(file);
        if !path.starts_with("lightmaps")
            || path
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            continue;
        }
        let Ok(path) = resource_root.join(path).canonicalize() else {
            continue;
        };
        if !path.starts_with(&resource_root) || !path.is_file() {
            continue;
        }
        files.insert(format!("@lightmap:{}", lower(texture)), path);
    }
}

#[cfg(test)]
mod external_lightmap_tests {
    use super::*;
    #[test]
    fn lightmaps_resolve_map_resource_and_reject_escape_paths() {
        let root = std::env::temp_dir().join(format!("eagle-lightmaps-{}", std::process::id()));
        let map = root.join("Map");
        let atlas = map.join("lightmaps");
        fs::create_dir_all(&map).unwrap();
        fs::create_dir_all(&atlas).unwrap();
        fs::write(atlas.join("lm.png"), b"fixture").unwrap();
        fs::write(root.join("outside.png"), b"fixture").unwrap();
        fs::write(
            map.join("eagleLightMaps.xml"),
            r#"<eagleLightMaps uvSet="1">
            <lightmap texture="ground" file="lightmaps/lm.png"/>
            <lightmap texture="escape" file="lightmaps/../../outside.png"/>
            </eagleLightMaps>"#,
        )
        .unwrap();
        let mut files = HashMap::new();
        collect_map_lightmaps(&map, &mut files);
        assert_eq!(files.len(), 1);
        assert_eq!(
            files["@lightmap:ground"],
            atlas.join("lm.png").canonicalize().unwrap()
        );
        fs::remove_dir_all(root).unwrap();
    }
}
