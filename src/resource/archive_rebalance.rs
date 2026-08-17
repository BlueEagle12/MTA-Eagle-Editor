use super::super::*;

pub(crate) const IMG_ARCHIVE_LIMIT_BYTES: u64 = 90 * 1024 * 1024;
const IMG_ARCHIVE_SECTOR_BYTES: u64 = 2048;
const MAX_LOADER_ARCHIVES_PER_FAMILY: usize = 64;
const RESOURCE_IMG_CONFIG: &str = "eagleLoader-imgs.xml";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum ImgArchiveFamily {
    Dff,
    Col,
    Txd,
    Custom,
}

impl ImgArchiveFamily {
    const ALL: [Self; 4] = [Self::Dff, Self::Col, Self::Txd, Self::Custom];

    fn for_name(name: &str) -> Self {
        let name = name.to_ascii_lowercase();
        if name.ends_with(".dff") {
            Self::Dff
        } else if name.ends_with(".col") {
            Self::Col
        } else if name.ends_with(".txd") {
            Self::Txd
        } else {
            Self::Custom
        }
    }

    fn stem(self) -> &'static str {
        match self {
            Self::Dff => "dff",
            Self::Col => "col",
            Self::Txd => "txd",
            Self::Custom => "custom",
        }
    }
}

#[derive(Clone)]
struct RebalanceEntry {
    name: String,
    bytes: Vec<u8>,
}

impl RebalanceEntry {
    fn padded_bytes(&self) -> u64 {
        (self.bytes.len() as u64)
            .div_ceil(IMG_ARCHIVE_SECTOR_BYTES)
            .max(1)
            * IMG_ARCHIVE_SECTOR_BYTES
    }
}

#[derive(Default)]
struct RebalanceBin {
    entries: Vec<RebalanceEntry>,
}

impl RebalanceBin {
    fn archive_size_with(&self, entry: Option<&RebalanceEntry>) -> u64 {
        let count = self.entries.len() + usize::from(entry.is_some());
        let directory_bytes = 8u64.saturating_add((count as u64).saturating_mul(32));
        let header_bytes =
            directory_bytes.div_ceil(IMG_ARCHIVE_SECTOR_BYTES).max(1) * IMG_ARCHIVE_SECTOR_BYTES;
        header_bytes
            + self
                .entries
                .iter()
                .map(RebalanceEntry::padded_bytes)
                .sum::<u64>()
            + entry.map(RebalanceEntry::padded_bytes).unwrap_or_default()
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ImgArchiveFamilyResult {
    pub(crate) entries: usize,
    pub(crate) archives: usize,
    pub(crate) bytes: u64,
}

pub(crate) struct ImgArchiveRebalanceResult {
    pub(crate) families: BTreeMap<String, ImgArchiveFamilyResult>,
    pub(crate) duplicate_resolutions: Vec<String>,
    pub(crate) runtime_name_repairs: BTreeMap<String, String>,
    runtime_dff_repairs: BTreeSet<String>,
    pub(crate) repaired_definition_files: usize,
    pub(crate) resource_archive_txd_index: TxdTextureIndex,
    pub(crate) backup_dir: PathBuf,
    pub(crate) archive_paths: Vec<PathBuf>,
    pub(crate) elapsed: f32,
}

pub(crate) struct ImgArchiveRebalanceJob {
    rx: mpsc::Receiver<Result<ImgArchiveRebalanceResult, String>>,
    progress_rx: mpsc::Receiver<String>,
}

fn pack_family_entries(
    family: ImgArchiveFamily,
    mut entries: Vec<RebalanceEntry>,
    limit: u64,
) -> Result<Vec<RebalanceBin>, String> {
    entries.sort_by(|a, b| {
        b.padded_bytes()
            .cmp(&a.padded_bytes())
            .then_with(|| lower(&a.name).cmp(&lower(&b.name)))
    });
    let mut bins = Vec::<RebalanceBin>::new();
    for entry in entries {
        if RebalanceBin::default().archive_size_with(Some(&entry)) > limit {
            return Err(format!(
                "{} is too large to fit in a {} MiB {} archive",
                entry.name,
                limit / (1024 * 1024),
                family.stem().to_ascii_uppercase()
            ));
        }
        let best = bins
            .iter()
            .enumerate()
            .filter_map(|(index, bin)| {
                let size = bin.archive_size_with(Some(&entry));
                (size <= limit).then(|| (index, limit - size))
            })
            .min_by_key(|(_, remaining)| *remaining)
            .map(|(index, _)| index);
        if let Some(index) = best {
            bins[index].entries.push(entry);
        } else {
            bins.push(RebalanceBin {
                entries: vec![entry],
            });
        }
    }
    if bins.len() > MAX_LOADER_ARCHIVES_PER_FAMILY {
        return Err(format!(
            "{} assets need {} archives, but the canonical eagleLoader currently scans only {} numbered archives per family",
            family.stem().to_ascii_uppercase(),
            bins.len(),
            MAX_LOADER_ARCHIVES_PER_FAMILY
        ));
    }
    for bin in &mut bins {
        bin.entries
            .sort_by(|a, b| lower(&a.name).cmp(&lower(&b.name)));
    }
    Ok(bins)
}

fn archive_output_name(family: ImgArchiveFamily, index: usize, count: usize) -> String {
    if count == 1 {
        format!("{}.img", family.stem())
    } else {
        format!("{}_{}.img", family.stem(), index + 1)
    }
}

fn verify_staged_archive(
    path: &Path,
    expected: &[RebalanceEntry],
    limit: u64,
) -> Result<(), String> {
    let file_size = fs::metadata(path)
        .map_err(|err| format!("{}: {err}", path.display()))?
        .len();
    if file_size > limit {
        return Err(format!(
            "{} is {:.2} MiB after writing, above the {:.2} MiB limit",
            path.display(),
            file_size as f64 / (1024.0 * 1024.0),
            limit as f64 / (1024.0 * 1024.0)
        ));
    }
    let parsed = parse_img(path);
    if parsed.len() != expected.len() {
        return Err(format!(
            "{} failed validation: wrote {} entries but parsed {}",
            path.display(),
            expected.len(),
            parsed.len()
        ));
    }
    for (actual, expected) in parsed.iter().zip(expected) {
        if !actual.name.eq_ignore_ascii_case(&expected.name) {
            return Err(format!(
                "{} failed validation: expected {}, found {}",
                path.display(),
                expected.name,
                actual.name
            ));
        }
        let bytes = read_img_entry(actual);
        let actual_len = replacement_entry_len(&actual.name, &bytes);
        if actual_len != expected.bytes.len()
            || bytes.get(..actual_len) != Some(expected.bytes.as_slice())
        {
            return Err(format!(
                "{} failed byte validation for {}",
                path.display(),
                expected.name
            ));
        }
    }
    Ok(())
}

fn unique_transaction_dir(root: &Path, prefix: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let mut candidate = root.join(format!("{prefix}_{stamp}"));
    let mut suffix = 1usize;
    while candidate.exists() {
        candidate = root.join(format!("{prefix}_{stamp}_{suffix}"));
        suffix += 1;
    }
    candidate
}

fn img_entry_stem(name: &str) -> Option<&str> {
    let dot = name.rfind('.')?;
    (dot > 0).then_some(&name[..dot])
}

fn runtime_safe_img_stem(stem: &str, reserved: &mut BTreeSet<String>) -> String {
    let max_stem_bytes = IMG_RUNTIME_SAFE_ENTRY_NAME_BYTES - ".dff".len();
    let truncate = |value: &str, max_bytes: usize| {
        let mut output = String::new();
        for character in value.chars() {
            if output.len() + character.len_utf8() > max_bytes {
                break;
            }
            output.push(character);
        }
        output
    };
    let mut base = truncate(stem, max_stem_bytes);
    while base.ends_with('_') || base.ends_with('-') || base.ends_with('.') {
        base.pop();
    }
    if base.is_empty() {
        base = "asset".to_string();
    }

    let mut candidate = base.clone();
    for suffix in 1usize.. {
        if reserved.insert(lower(&candidate)) {
            return candidate;
        }
        let tail = format!("_{suffix}");
        let keep = max_stem_bytes.saturating_sub(tail.len());
        let prefix = truncate(&base, keep);
        candidate = format!("{prefix}{tail}");
    }
    unreachable!()
}

fn collect_runtime_name_repairs(
    gathered: &HashMap<String, (ImgArchiveFamily, RebalanceEntry, SystemTime, PathBuf)>,
) -> (BTreeMap<String, String>, BTreeSet<String>) {
    let mut reserved = gathered
        .values()
        .filter_map(|(_, entry, _, _)| img_entry_stem(&entry.name))
        .map(lower)
        .collect::<BTreeSet<_>>();
    let unsafe_stems = gathered
        .values()
        .filter(|(family, entry, _, _)| {
            *family != ImgArchiveFamily::Custom
                && entry.name.as_bytes().len() > IMG_RUNTIME_SAFE_ENTRY_NAME_BYTES
        })
        .filter_map(|(_, entry, _, _)| img_entry_stem(&entry.name))
        .map(lower)
        .collect::<BTreeSet<_>>();
    let mut repairs = BTreeMap::new();
    for old_stem in unsafe_stems {
        let new_stem = runtime_safe_img_stem(&old_stem, &mut reserved);
        repairs.insert(old_stem, new_stem);
    }
    let dff_repairs = gathered
        .values()
        .filter(|(family, _, _, _)| *family == ImgArchiveFamily::Dff)
        .filter_map(|(_, entry, _, _)| img_entry_stem(&entry.name))
        .map(lower)
        .filter(|stem| repairs.contains_key(stem))
        .collect();
    (repairs, dff_repairs)
}

fn renamed_img_entry(name: &str, repairs: &BTreeMap<String, String>) -> String {
    let Some(dot) = name.rfind('.') else {
        return name.to_string();
    };
    let Some(replacement) = repairs.get(&lower(&name[..dot])) else {
        return name.to_string();
    };
    format!("{replacement}{}", &name[dot..])
}

fn renamed_definition_asset(value: &str, repairs: &BTreeMap<String, String>) -> Option<String> {
    let trimmed = value.trim();
    let (stem, extension) = trimmed
        .rfind('.')
        .map(|dot| (&trimmed[..dot], Some(&trimmed[dot..])))
        .unwrap_or((trimmed, None));
    let replacement = repairs.get(&lower(stem))?;
    Some(format!("{replacement}{}", extension.unwrap_or_default()))
}

fn renamed_definition_asset_list(
    value: &str,
    repairs: &BTreeMap<String, String>,
) -> Option<String> {
    let mut changed = false;
    let renamed = value
        .split(',')
        .map(|part| {
            let replacement = renamed_definition_asset(part, repairs);
            changed |= replacement.is_some();
            replacement.unwrap_or_else(|| part.trim().to_string())
        })
        .collect::<Vec<_>>()
        .join(",");
    changed.then_some(renamed)
}

fn rewrite_definition_asset_references(
    text: &str,
    repairs: &BTreeMap<String, String>,
    dff_repairs: &BTreeSet<String>,
) -> (String, usize) {
    let tag_re = Regex::new(r#"(?i)<(?:definition|override)\b[^>]*>"#).unwrap();
    let attr_re = Regex::new(r#"(?i)([A-Za-z_][A-Za-z0-9_]*)\s*=\s*\"([^\"]*)\""#).unwrap();
    let mut changed_tags = 0usize;
    let rewritten = tag_re.replace_all(text, |tag_caps: &regex::Captures<'_>| {
        let original = tag_caps.get(0).unwrap().as_str();
        let attrs = attr_re
            .captures_iter(original)
            .map(|caps| (lower(&caps[1]), caps[2].to_string()))
            .collect::<HashMap<_, _>>();
        let mut tag = original.to_string();
        for field in ["dff", "col", "txd"] {
            let Some(value) = attrs.get(field) else {
                continue;
            };
            let Some(replacement) = renamed_definition_asset_list(value, repairs) else {
                continue;
            };
            let field_re = Regex::new(&format!(
                r#"(?i)(\b{}\s*=\s*\"){}(\")"#,
                regex::escape(field),
                regex::escape(value)
            ))
            .unwrap();
            tag = field_re
                .replace(&tag, format!("${{1}}{replacement}${{2}}"))
                .into_owned();
        }
        if !attrs.contains_key("dff")
            && let Some(id) = attrs.get("id")
            && dff_repairs.contains(&lower(id))
            && let Some(replacement) = repairs.get(&lower(id))
        {
            if let Some(position) = tag.rfind("/>").or_else(|| tag.rfind('>')) {
                tag.insert_str(position, &format!(" dff=\"{replacement}\""));
            }
        }
        if tag != original {
            changed_tags += 1;
        }
        tag
    });
    (rewritten.into_owned(), changed_tags)
}

fn stage_definition_name_repairs(
    root: &Path,
    staging_dir: &Path,
    repairs: &BTreeMap<String, String>,
    dff_repairs: &BTreeSet<String>,
) -> Result<(Vec<(PathBuf, PathBuf)>, usize), String> {
    if repairs.is_empty() {
        return Ok((Vec::new(), 0));
    }
    let mut staged = Vec::new();
    let mut repaired_files = 0usize;
    for entry in WalkDir::new(root.join("zones"))
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("definition"))
        })
    {
        let output_path = entry.path().to_path_buf();
        let text = fs::read_to_string(&output_path)
            .map_err(|err| format!("{}: {err}", output_path.display()))?;
        let (rewritten, changed_tags) =
            rewrite_definition_asset_references(&text, repairs, dff_repairs);
        if changed_tags == 0 {
            continue;
        }
        let relative = output_path
            .strip_prefix(root)
            .map_err(|_| format!("{} is outside the project root", output_path.display()))?;
        let staged_path = staging_dir.join(relative);
        if let Some(parent) = staged_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("Could not create {}: {err}", parent.display()))?;
        }
        fs::write(&staged_path, rewritten)
            .map_err(|err| format!("Could not stage {}: {err}", output_path.display()))?;
        staged.push((staged_path, output_path));
        repaired_files += 1;
    }
    Ok((staged, repaired_files))
}

fn repaired_asset_value(value: &str, repairs: &BTreeMap<String, String>) -> String {
    renamed_definition_asset_list(value, repairs).unwrap_or_else(|| value.to_string())
}

fn apply_runtime_name_repairs_to_app(
    app: &mut AppState,
    repairs: &BTreeMap<String, String>,
    dff_repairs: &BTreeSet<String>,
) {
    if repairs.is_empty() {
        return;
    }

    for definition in app.definitions.values_mut() {
        if app.readonly_definition_ids.contains(&definition.id) {
            continue;
        }
        for field in ["dff", "col", "txd"] {
            if let Some(value) = definition.attrs.get_mut(field) {
                *value = repaired_asset_value(value, repairs);
            }
        }
        if !definition.attrs.contains_key("dff")
            && dff_repairs.contains(&lower(&definition.id))
            && let Some(replacement) = repairs.get(&lower(&definition.id))
        {
            definition
                .attrs
                .insert("dff".to_string(), replacement.clone());
        }
    }

    for placement in &mut app.placements {
        if let Some(replacement) = renamed_definition_asset(&placement.dff, repairs) {
            placement.dff = replacement;
        }
        if let Some(dff) = app
            .definitions
            .get(&placement.id)
            .and_then(|definition| definition.attrs.get("dff"))
            .map(String::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            placement.dff = dff.to_string();
        }
    }

    let old_meshes = std::mem::take(&mut app.meshes);
    for (key, mesh) in old_meshes {
        let (dff, txd) = key.split_once('|').unwrap_or((&key, ""));
        let new_dff = renamed_definition_asset(dff, repairs).unwrap_or_else(|| dff.to_string());
        let new_txd = renamed_definition_asset(txd, repairs).unwrap_or_else(|| txd.to_string());
        app.meshes.insert(format!("{new_dff}|{new_txd}"), mesh);
    }

    clear_collision_render_cache(app);
    let old_collisions = std::mem::take(&mut app.collisions);
    for (key, collision) in old_collisions {
        let new_key = renamed_definition_asset(&key, repairs).unwrap_or(key);
        app.collisions
            .insert(asset_key(&new_key, ".col"), collision);
    }
    invalidate_validation_cache(app);
}

fn rollback_archive_install(installed: &[PathBuf], backups: &[(PathBuf, PathBuf)]) -> Vec<String> {
    let mut errors = Vec::new();
    for path in installed {
        if path.exists()
            && let Err(err) = fs::remove_file(path)
        {
            errors.push(format!(
                "Could not remove {} during rollback: {err}",
                path.display()
            ));
        }
    }
    for (original, backup) in backups.iter().rev() {
        if backup.exists()
            && let Err(err) = fs::rename(backup, original)
        {
            errors.push(format!(
                "Could not restore {} from {}: {err}",
                original.display(),
                backup.display()
            ));
        }
    }
    errors
}

fn index_resource_archive_txds(root: &Path) -> TxdTextureIndex {
    let mut index = TxdTextureIndex::new();
    let mut base_archives = Vec::new();
    collect_img_files_from_dir(&root.join("imgs"), &mut base_archives);
    base_archives.sort();
    for path in base_archives {
        index_txd_file(&path, &mut index);
    }

    let mut overlay_archives = Vec::new();
    collect_img_files_from_dir(&wip_root_path(root).join("imgs"), &mut overlay_archives);
    overlay_archives.sort();
    for path in overlay_archives {
        let entries = parse_img(&path);
        remove_img_txd_dictionaries_from_index(&mut index, &entries);
        index_txd_entries(&path, &entries, &mut index);
    }
    index
}

fn install_resource_archive_txd_index(app: &mut AppState, fresh: TxdTextureIndex) {
    app.txd_textures.retain(|_, entries| {
        entries.retain(|entry| {
            !(entry.img_path.starts_with(&app.root)
                && entry
                    .img_path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("img")))
        });
        !entries.is_empty()
    });
    for (texture_name, entries) in fresh {
        let destination = app.txd_textures.entry(texture_name).or_default();
        for entry in entries {
            if !destination
                .iter()
                .any(|existing| existing.txd_name.eq_ignore_ascii_case(&entry.txd_name))
            {
                destination.push(entry);
            }
        }
    }
}

fn rebalance_img_archives(
    root: &Path,
    limit: u64,
    progress_tx: &mpsc::Sender<String>,
) -> Result<ImgArchiveRebalanceResult, String> {
    let started_at = Instant::now();
    let imgs_dir = root.join("imgs");
    let mut source_paths = Vec::new();
    collect_img_files_from_dir(&imgs_dir, &mut source_paths);
    source_paths.retain(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_none_or(|name| !name.eq_ignore_ascii_case(REPLACEMENT_IMG))
    });
    source_paths.sort();
    if source_paths.is_empty() {
        return Err("No saved project IMG archives were found under imgs/.".to_string());
    }

    let _ = progress_tx.send(format!(
        "Organizing IMG archives: reading {} archive(s)...",
        source_paths.len()
    ));
    let mut gathered =
        HashMap::<String, (ImgArchiveFamily, RebalanceEntry, SystemTime, PathBuf)>::new();
    let mut duplicate_resolutions = Vec::new();
    for path in &source_paths {
        let parsed = parse_img(path);
        let valid_ver2 = fs::read(path)
            .map(|bytes| bytes.starts_with(b"VER2"))
            .unwrap_or(false);
        if !valid_ver2 || parsed.is_empty() {
            return Err(format!(
                "{} is empty or is not a valid VER2 IMG archive; no archives were changed",
                path.display()
            ));
        }
        let modified = fs::metadata(path)
            .and_then(|metadata| metadata.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let mut file = fs::File::open(path).map_err(|err| format!("{}: {err}", path.display()))?;
        for entry in parsed {
            let key = lower(&entry.name);
            let mut bytes = read_img_entry_from(&mut file, &entry);
            let logical_len = replacement_entry_len(&entry.name, &bytes).min(bytes.len());
            bytes.truncate(logical_len);
            let candidate = (
                ImgArchiveFamily::for_name(&entry.name),
                RebalanceEntry {
                    name: entry.name,
                    bytes,
                },
                modified,
                path.clone(),
            );
            if let Some(existing) = gathered.get_mut(&key) {
                let candidate_is_newer = candidate.2 > existing.2
                    || (candidate.2 == existing.2 && candidate.3 > existing.3);
                let (kept, discarded) = if candidate_is_newer {
                    let discarded = existing.3.clone();
                    *existing = candidate;
                    (existing.3.clone(), discarded)
                } else {
                    (existing.3.clone(), candidate.3)
                };
                duplicate_resolutions.push(format!(
                    "{}: kept newest copy from {}; discarded duplicate from {}",
                    existing.1.name,
                    kept.display(),
                    discarded.display()
                ));
            } else {
                gathered.insert(key, candidate);
            }
        }
    }

    let (runtime_name_repairs, runtime_dff_repairs) = collect_runtime_name_repairs(&gathered);
    if !runtime_name_repairs.is_empty() {
        let _ = progress_tx.send(format!(
            "Repairing {} IMG asset name(s) for MTA compatibility...",
            runtime_name_repairs.len()
        ));
        for (_, entry, _, _) in gathered.values_mut() {
            entry.name = renamed_img_entry(&entry.name, &runtime_name_repairs);
        }
    }

    let mut families = BTreeMap::<ImgArchiveFamily, Vec<RebalanceEntry>>::new();
    for (_, (family, entry, _, _)) in gathered {
        families.entry(family).or_default().push(entry);
    }
    duplicate_resolutions.sort();

    let _ = progress_tx.send("Organizing IMG archives: balancing entries...".to_string());
    let mut packed = BTreeMap::<ImgArchiveFamily, Vec<RebalanceBin>>::new();
    for family in ImgArchiveFamily::ALL {
        let entries = families.remove(&family).unwrap_or_default();
        packed.insert(family, pack_family_entries(family, entries, limit)?);
    }

    let staging_dir = unique_transaction_dir(root, ".eagle_img_rebalance_staging");
    fs::create_dir_all(&staging_dir)
        .map_err(|err| format!("Could not create {}: {err}", staging_dir.display()))?;
    let mut staged = Vec::<(PathBuf, PathBuf)>::new();
    let mut family_results = BTreeMap::<String, ImgArchiveFamilyResult>::new();
    let write_result = (|| -> Result<(), String> {
        for family in ImgArchiveFamily::ALL {
            let bins = packed.get(&family).map(Vec::as_slice).unwrap_or_default();
            let mut summary = ImgArchiveFamilyResult::default();
            summary.archives = bins.len();
            for (index, bin) in bins.iter().enumerate() {
                let name = archive_output_name(family, index, bins.len());
                let staged_path = staging_dir.join(&name);
                let output_path = imgs_dir.join(&name);
                let entries = bin
                    .entries
                    .iter()
                    .map(|entry| (entry.name.clone(), entry.bytes.clone()))
                    .collect::<Vec<_>>();
                let _ = progress_tx.send(format!(
                    "Organizing IMG archives: writing {} ({}/{})...",
                    name,
                    index + 1,
                    bins.len()
                ));
                write_img_archive(&staged_path, &entries)?;
                verify_staged_archive(&staged_path, &bin.entries, limit)?;
                summary.entries += bin.entries.len();
                summary.bytes += fs::metadata(&staged_path)
                    .map_err(|err| format!("{}: {err}", staged_path.display()))?
                    .len();
                staged.push((staged_path, output_path));
            }
            family_results.insert(family.stem().to_string(), summary);
        }
        Ok(())
    })();
    if let Err(err) = write_result {
        let _ = fs::remove_dir_all(&staging_dir);
        return Err(err);
    }

    let (staged_definitions, repaired_definition_files) = match stage_definition_name_repairs(
        root,
        &staging_dir,
        &runtime_name_repairs,
        &runtime_dff_repairs,
    ) {
        Ok(staged) => staged,
        Err(err) => {
            let _ = fs::remove_dir_all(&staging_dir);
            return Err(err);
        }
    };

    let resource_img_config = root.join(RESOURCE_IMG_CONFIG);
    let staged_img_config = resource_img_config.is_file().then(|| {
        let path = staging_dir.join(RESOURCE_IMG_CONFIG);
        (path, resource_img_config.clone())
    });
    if let Some((staged_path, _)) = &staged_img_config {
        let config = format!(
            "<config>\n    <img names=\"dff,col,txd,custom\" max=\"{}\" />\n</config>\n",
            MAX_LOADER_ARCHIVES_PER_FAMILY
        );
        fs::write(staged_path, config)
            .map_err(|err| format!("Could not stage {}: {err}", staged_path.display()))?;
    }
    let resource_meta = root.join("meta.xml");
    let staged_meta = resource_meta.is_file().then(|| {
        let path = staging_dir.join("meta.xml");
        (path, resource_meta.clone())
    });
    if let Some((staged_path, _)) = &staged_meta {
        let source = fs::read_to_string(&resource_meta)
            .map_err(|err| format!("Could not read {}: {err}", resource_meta.display()))?;
        let updated = ensure_img_archive_meta_wildcard(&source)?;
        fs::write(staged_path, updated)
            .map_err(|err| format!("Could not stage {}: {err}", staged_path.display()))?;
    }

    let backup_root = root.join(".eagle_backups");
    let backup_dir = unique_transaction_dir(&backup_root, "img_rebalance");
    fs::create_dir_all(&backup_dir)
        .map_err(|err| format!("Could not create {}: {err}", backup_dir.display()))?;
    let mut definition_backup_paths = Vec::with_capacity(staged_definitions.len());
    for (_, original) in &staged_definitions {
        let relative = original
            .strip_prefix(root)
            .map_err(|_| format!("{} is outside the project root", original.display()))?;
        let backup = backup_dir.join(relative);
        if let Some(parent) = backup.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("Could not create {}: {err}", parent.display()))?;
        }
        definition_backup_paths.push(backup);
    }
    let _ =
        progress_tx.send("Organizing IMG archives: installing verified archives...".to_string());
    let mut backups = Vec::<(PathBuf, PathBuf)>::new();
    for original in &source_paths {
        let file_name = original
            .file_name()
            .ok_or_else(|| format!("{} has no file name", original.display()))?;
        let backup = backup_dir.join(file_name);
        if let Err(err) = fs::rename(original, &backup) {
            let rollback_errors = rollback_archive_install(&[], &backups);
            return Err(format!(
                "Could not back up {}: {err}.{}",
                original.display(),
                if rollback_errors.is_empty() {
                    String::new()
                } else {
                    format!(" Rollback errors: {}", rollback_errors.join("; "))
                }
            ));
        }
        backups.push((original.clone(), backup));
    }
    if let Some((_, original)) = &staged_img_config {
        let backup = backup_dir.join(RESOURCE_IMG_CONFIG);
        if let Err(err) = fs::rename(original, &backup) {
            let rollback_errors = rollback_archive_install(&[], &backups);
            return Err(format!(
                "Could not back up {}: {err}.{}",
                original.display(),
                if rollback_errors.is_empty() {
                    String::new()
                } else {
                    format!(" Rollback errors: {}", rollback_errors.join("; "))
                }
            ));
        }
        backups.push((original.clone(), backup));
    }
    if let Some((_, original)) = &staged_meta {
        let backup = backup_dir.join("meta.xml");
        if let Err(err) = fs::rename(original, &backup) {
            let rollback_errors = rollback_archive_install(&[], &backups);
            return Err(format!(
                "Could not back up {}: {err}.{}",
                original.display(),
                if rollback_errors.is_empty() {
                    String::new()
                } else {
                    format!(" Rollback errors: {}", rollback_errors.join("; "))
                }
            ));
        }
        backups.push((original.clone(), backup));
    }
    for ((_, original), backup) in staged_definitions.iter().zip(definition_backup_paths) {
        if let Err(err) = fs::rename(original, &backup) {
            let rollback_errors = rollback_archive_install(&[], &backups);
            let _ = fs::remove_dir_all(&staging_dir);
            return Err(format!(
                "Could not back up {}: {err}.{}",
                original.display(),
                if rollback_errors.is_empty() {
                    String::new()
                } else {
                    format!(" Rollback errors: {}", rollback_errors.join("; "))
                }
            ));
        }
        backups.push((original.clone(), backup));
    }

    let mut installed = Vec::new();
    for (staged_path, output_path) in staged
        .iter()
        .chain(staged_img_config.iter())
        .chain(staged_meta.iter())
        .chain(staged_definitions.iter())
    {
        if let Err(err) = fs::rename(staged_path, output_path) {
            let rollback_errors = rollback_archive_install(&installed, &backups);
            let _ = fs::remove_dir_all(&staging_dir);
            return Err(format!(
                "Could not install {}: {err}.{}",
                output_path.display(),
                if rollback_errors.is_empty() {
                    String::new()
                } else {
                    format!(" Rollback errors: {}", rollback_errors.join("; "))
                }
            ));
        }
        installed.push(output_path.clone());
    }
    let _ = fs::remove_dir_all(&staging_dir);
    let mut archive_paths = staged
        .iter()
        .map(|(_, output)| output.clone())
        .collect::<Vec<_>>();
    archive_paths.sort();
    let resource_archive_txd_index = index_resource_archive_txds(root);

    Ok(ImgArchiveRebalanceResult {
        families: family_results,
        duplicate_resolutions,
        runtime_name_repairs,
        runtime_dff_repairs,
        repaired_definition_files,
        resource_archive_txd_index,
        backup_dir,
        archive_paths,
        elapsed: started_at.elapsed().as_secs_f32(),
    })
}

fn ensure_img_archive_meta_wildcard(source: &str) -> Result<String, String> {
    let mut out = Vec::new();
    let mut insertion = None;
    for line in source.lines() {
        let trimmed = line.trim();
        let is_img_file = trimmed.starts_with("<file")
            && trimmed.contains("src=\"imgs/")
            && trimmed.contains(".img");
        if is_img_file {
            if insertion.is_none() {
                let indent = &line[..line.len() - line.trim_start().len()];
                insertion = Some(format!(
                    "{indent}<file type=\"client\" src=\"imgs/*.img\" />"
                ));
            }
        } else {
            out.push(line.to_string());
        }
    }
    let wildcard =
        insertion.unwrap_or_else(|| "    <file type=\"client\" src=\"imgs/*.img\" />".to_string());
    let at = out
        .iter()
        .position(|line| line.trim() == "</meta>")
        .ok_or_else(|| "meta.xml has no </meta> tag".to_string())?;
    out.insert(at, wildcard);
    let mut text = out.join("\n");
    if source.ends_with('\n') {
        text.push('\n');
    }
    Ok(text)
}

pub(crate) fn request_img_archive_rebalance(app: &mut AppState) {
    if app.img_archive_rebalance_job.is_some() {
        app.status_message = "IMG archive organization is already running.".to_string();
        return;
    }
    if let Some(conflict) = validation_operation_conflict(app) {
        app.status_message =
            format!("Wait for the background {conflict} to finish before organizing IMG archives.");
        return;
    }
    if editing_dirty(app) {
        app.status_message =
            "Save or discard the current Editing changes before organizing IMG archives."
                .to_string();
        return;
    }
    if has_unsaved_changes(app) {
        app.status_message =
            "Save project changes before fixing or organizing IMG archives.".to_string();
        return;
    }
    app.confirm_dialog = Some(ConfirmDialog {
        action: ConfirmAction::RebalanceImgArchives,
        title: "Fix & Organize IMG Archives".to_string(),
        body: "Repair MTA-incompatible asset names and redistribute saved project IMG archives?"
            .to_string(),
        detail: "DFF, COL, and TXD entry names that occupy all 24 IMG directory bytes will be shortened consistently, and matching project definitions will receive updated asset references without changing their logical IDs or LOD links. Assets are then placed in balanced type-specific archives no larger than 90 MiB for GitHub compatibility. Original IMG and definition files are retained in a recovery backup, and editor undo/redo history is cleared after success.".to_string(),
        primary_label: "Fix & Organize".to_string(),
        secondary_label: None,
        secondary_action: None,
    });
}

fn saved_project_img_duplicates(root: &Path) -> Result<Vec<String>, String> {
    let mut paths = Vec::new();
    collect_img_files_from_dir(&root.join("imgs"), &mut paths);
    paths.retain(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_none_or(|name| !name.eq_ignore_ascii_case(REPLACEMENT_IMG))
    });
    paths.sort();
    let mut first = HashMap::<String, (String, PathBuf)>::new();
    let mut duplicates = Vec::new();
    for path in paths {
        let entries = parse_img(&path);
        let valid_ver2 = fs::read(&path)
            .map(|bytes| bytes.starts_with(b"VER2"))
            .unwrap_or(false);
        if !valid_ver2 {
            return Err(format!(
                "{} is not a supported VER2 IMG archive",
                path.display()
            ));
        }
        for entry in entries {
            let key = lower(&entry.name);
            if let Some((first_name, first_path)) = first.get(&key) {
                duplicates.push(format!(
                    "{} in {} duplicates {} in {}",
                    entry.name,
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("IMG"),
                    first_name,
                    first_path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("IMG"),
                ));
            } else {
                first.insert(key, (entry.name, path.clone()));
            }
        }
    }
    Ok(duplicates)
}

pub(crate) fn request_img_duplicate_fix(app: &mut AppState) {
    if app.img_archive_rebalance_job.is_some() {
        app.status_message = "IMG archive repair is already running.".to_string();
        return;
    }
    if let Some(conflict) = validation_operation_conflict(app) {
        app.status_message =
            format!("Wait for the background {conflict} to finish before fixing duplicates.");
        return;
    }
    if editing_dirty(app) {
        app.status_message =
            "Save or discard the current Editing changes before fixing duplicate IMG entries."
                .to_string();
        return;
    }
    if has_unsaved_changes(app) {
        app.status_message =
            "Save project changes before fixing duplicate IMG entries.".to_string();
        return;
    }
    let duplicates = match saved_project_img_duplicates(&app.root) {
        Ok(duplicates) => duplicates,
        Err(error) => {
            app.status_message = format!("Could not scan IMG duplicates: {error}");
            return;
        }
    };
    if duplicates.is_empty() {
        app.status_message = "No duplicate IMG entry names were found.".to_string();
        return;
    }
    let mut listed = duplicates.iter().take(8).cloned().collect::<Vec<_>>();
    if duplicates.len() > listed.len() {
        listed.push(format!("and {} more", duplicates.len() - listed.len()));
    }
    app.confirm_dialog = Some(ConfirmDialog {
        action: ConfirmAction::RebalanceImgArchives,
        title: "Fix Duplicate IMG Entries".to_string(),
        body: format!(
            "Found {} duplicate IMG entr{}. Fix them now?",
            duplicates.len(),
            if duplicates.len() == 1 { "y" } else { "ies" },
        ),
        detail: format!(
            "The verified repair keeps the newest copy across archives and the first copy when a name repeats inside one archive. It creates a recovery backup, repairs incompatible IMG names, and reorganizes entries into type-specific archives.\n{}",
            listed.join("; ")
        ),
        primary_label: "Fix Duplicates".to_string(),
        secondary_label: None,
        secondary_action: None,
    });
    app.status_message = format!(
        "Found {} duplicate IMG entries; confirmation required.",
        duplicates.len()
    );
}

pub(crate) fn start_img_archive_rebalance(app: &mut AppState) {
    if app.img_archive_rebalance_job.is_some() {
        return;
    }
    if let Some(conflict) = validation_operation_conflict(app) {
        app.status_message =
            format!("Wait for the background {conflict} to finish before organizing IMG archives.");
        return;
    }
    if editing_dirty(app) {
        app.status_message =
            "Save or discard the current Editing changes before organizing IMG archives."
                .to_string();
        return;
    }
    if has_unsaved_changes(app) {
        app.status_message =
            "Save project changes before fixing or organizing IMG archives.".to_string();
        return;
    }
    let root = app.root.clone();
    let (tx, rx) = mpsc::channel();
    let (progress_tx, progress_rx) = mpsc::channel();
    thread::spawn(move || {
        let result = std::panic::catch_unwind(|| {
            rebalance_img_archives(&root, IMG_ARCHIVE_LIMIT_BYTES, &progress_tx)
        })
        .unwrap_or_else(|_| Err("IMG archive organization worker panicked.".to_string()));
        let _ = tx.send(result);
    });
    app.img_archive_rebalance_job = Some(ImgArchiveRebalanceJob { rx, progress_rx });
    app.status_message = "Organizing IMG archives in the background...".to_string();
}

pub(crate) fn update_img_archive_rebalance(app: &mut AppState) {
    let Some(job) = app.img_archive_rebalance_job.take() else {
        return;
    };
    while let Ok(progress) = job.progress_rx.try_recv() {
        app.status_message = progress;
    }
    match job.rx.try_recv() {
        Ok(Ok(result)) => {
            install_resource_archive_txd_index(app, result.resource_archive_txd_index);
            apply_runtime_name_repairs_to_app(
                app,
                &result.runtime_name_repairs,
                &result.runtime_dff_repairs,
            );
            clear_history_for_external_change(app);
            let editing_camera = app.editing.camera.clone();
            let editing_camera_mode = app.editing.camera_mode;
            let editing_camera_focus = app.editing.camera_focus;
            app.editing = EditingState::default();
            app.editing.camera = editing_camera;
            app.editing.camera_mode = editing_camera_mode;
            app.editing.camera_focus = editing_camera_focus;
            app.editing.img_paths = result.archive_paths.clone();
            mark_saved_snapshot(app);
            let mut parts = Vec::new();
            for family in ["dff", "col", "txd", "custom"] {
                if let Some(summary) = result.families.get(family)
                    && summary.entries > 0
                {
                    parts.push(format!(
                        "{}: {} entries / {} archive(s)",
                        family.to_ascii_uppercase(),
                        summary.entries,
                        summary.archives
                    ));
                }
            }
            app.status_message = format!(
                "IMG archives fixed and organized in {:.1}s ({}). Repaired {} runtime-unsafe asset name(s) across {} definition file(s). Resolved {} duplicate(s) using the newest archive copy. Backup: {}. Undo and redo history cleared.",
                result.elapsed,
                parts.join(", "),
                result.runtime_name_repairs.len(),
                result.repaired_definition_files,
                result.duplicate_resolutions.len(),
                result.backup_dir.display()
            );
            let mut log = vec![app.status_message.clone()];
            if !result.runtime_name_repairs.is_empty() {
                log.push("MTA-compatible IMG name repairs:".to_string());
                log.extend(
                    result
                        .runtime_name_repairs
                        .iter()
                        .map(|(old, new)| format!("{old} -> {new}")),
                );
            }
            if !result.duplicate_resolutions.is_empty() {
                log.push("Duplicate resolutions:".to_string());
                log.extend(result.duplicate_resolutions);
            }
            set_save_log(app, "IMG Archives Fixed & Organized", log, false);
        }
        Ok(Err(err)) => {
            app.status_message =
                format!("IMG archive organization failed. Click the status bar for details.");
            set_save_log(app, "IMG Archive Organization Failed", vec![err], true);
        }
        Err(mpsc::TryRecvError::Empty) => {
            app.img_archive_rebalance_job = Some(job);
        }
        Err(mpsc::TryRecvError::Disconnected) => {
            app.status_message =
                "IMG archive organization failed: worker disconnected.".to_string();
            set_save_log(
                app,
                "IMG Archive Organization Failed",
                vec![app.status_message.clone()],
                true,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, sectors: usize) -> RebalanceEntry {
        RebalanceEntry {
            name: name.to_string(),
            bytes: vec![1; sectors * IMG_ARCHIVE_SECTOR_BYTES as usize],
        }
    }

    #[test]
    fn duplicate_scan_finds_repeated_names_within_and_across_archives() {
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let root = env::temp_dir().join(format!("eagle_img_duplicate_scan_{nonce}"));
        let imgs = root.join("imgs");
        fs::create_dir_all(&imgs).unwrap();
        write_img_archive(
            &imgs.join("a.img"),
            &[
                ("shared.dff".to_string(), vec![1]),
                ("inside.dff".to_string(), vec![2]),
                ("INSIDE.DFF".to_string(), vec![3]),
            ],
        )
        .unwrap();
        write_img_archive(&imgs.join("b.img"), &[("SHARED.DFF".to_string(), vec![4])]).unwrap();

        let duplicates = saved_project_img_duplicates(&root).unwrap();

        assert_eq!(duplicates.len(), 2);
        assert!(duplicates.iter().any(|item| item.contains("INSIDE.DFF")));
        assert!(duplicates.iter().any(|item| item.contains("SHARED.DFF")));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn archive_names_use_base_until_a_split_is_needed() {
        assert_eq!(archive_output_name(ImgArchiveFamily::Txd, 0, 1), "txd.img");
        assert_eq!(
            archive_output_name(ImgArchiveFamily::Txd, 0, 2),
            "txd_1.img"
        );
        assert_eq!(
            archive_output_name(ImgArchiveFamily::Txd, 1, 2),
            "txd_2.img"
        );
    }

    #[test]
    fn meta_img_entries_are_consolidated_to_runtime_wildcard() {
        let source = "<meta>\n    <file type=\"client\" src=\"imgs/dff.img\" />\n    <file type=\"client\" src=\"imgs/txd_1.img\" />\n    <file type=\"client\" src=\"water.dat\" />\n</meta>\n";
        let updated = ensure_img_archive_meta_wildcard(source).unwrap();
        assert_eq!(updated.matches("imgs/*.img").count(), 1);
        assert!(!updated.contains("imgs/dff.img"));
        assert!(!updated.contains("imgs/txd_1.img"));
        assert!(updated.contains("src=\"water.dat\""));
    }

    #[test]
    fn configured_archive_limit_is_90_mib() {
        assert_eq!(IMG_ARCHIVE_LIMIT_BYTES, 90 * 1024 * 1024);
    }

    #[test]
    fn best_fit_redistributes_entries_under_the_limit() {
        let limit = 100 * IMG_ARCHIVE_SECTOR_BYTES;
        let bins = pack_family_entries(
            ImgArchiveFamily::Txd,
            vec![entry("a.txd", 54), entry("b.txd", 43), entry("c.txd", 39)],
            limit,
        )
        .unwrap();
        assert_eq!(bins.len(), 2);
        assert!(bins.iter().all(|bin| bin.archive_size_with(None) <= limit));
        assert_eq!(bins.iter().map(|bin| bin.entries.len()).sum::<usize>(), 3);
    }

    #[test]
    fn entry_larger_than_an_archive_is_rejected() {
        let limit = 10 * IMG_ARCHIVE_SECTOR_BYTES;
        let error = pack_family_entries(
            ImgArchiveFamily::Dff,
            vec![entry("oversized.dff", 10)],
            limit,
        )
        .err()
        .unwrap();
        assert!(error.contains("oversized.dff"));
    }

    #[test]
    fn definition_repair_preserves_logical_id_and_adds_safe_asset_override() {
        let old = "lod_doontoon_newbi_2";
        let new = "lod_doontoon_newbi";
        let repairs = BTreeMap::from([(old.to_string(), new.to_string())]);
        let dff_repairs = BTreeSet::from([old.to_string()]);
        let source = format!(
            "<definition id=\"{old}\" col=\"{old}\" txd=\"{old}\" lodDistance=\"700\" />\n"
        );

        let (rewritten, changed) =
            rewrite_definition_asset_references(&source, &repairs, &dff_repairs);

        assert_eq!(changed, 1);
        assert!(rewritten.contains(&format!("id=\"{old}\"")));
        assert!(rewritten.contains(&format!("dff=\"{new}\"")));
        assert!(rewritten.contains(&format!("col=\"{new}\"")));
        assert!(rewritten.contains(&format!("txd=\"{new}\"")));
        assert!(rewritten.contains(&format!("dff=\"{new}\"/>")));
    }

    #[test]
    fn archive_transaction_repairs_full_width_mta_asset_names() {
        let root = unique_transaction_dir(&std::env::temp_dir(), "eagle_img_name_repair_test");
        let imgs = root.join("imgs");
        let zone_dir = root.join("zones/test");
        fs::create_dir_all(&imgs).unwrap();
        fs::create_dir_all(&zone_dir).unwrap();
        let old = "lod_doontoon_newbi_2";
        write_img_archive(
            &imgs.join("vice.img"),
            &[
                (format!("{old}.dff"), vec![1; 2048]),
                (format!("{old}.col"), vec![2; 2048]),
                (format!("{old}.txd"), vec![3; 2048]),
            ],
        )
        .unwrap();
        let definition_path = zone_dir.join("test.definition");
        let definition = format!(
            "<definition id=\"{old}\" col=\"{old}\" txd=\"{old}\" lodDistance=\"700\" />\n"
        );
        fs::write(&definition_path, &definition).unwrap();

        let (progress_tx, _progress_rx) = mpsc::channel();
        let result =
            rebalance_img_archives(&root, 100 * IMG_ARCHIVE_SECTOR_BYTES, &progress_tx).unwrap();

        let repaired = result.runtime_name_repairs.get(old).unwrap();
        assert!(format!("{repaired}.dff").len() <= IMG_RUNTIME_SAFE_ENTRY_NAME_BYTES);
        assert_eq!(result.repaired_definition_files, 1);
        for (archive, extension) in [("dff.img", "dff"), ("col.img", "col"), ("txd.img", "txd")] {
            let entries = parse_img(&imgs.join(archive));
            assert!(
                entries
                    .iter()
                    .any(|entry| entry.name == format!("{repaired}.{extension}"))
            );
        }
        let rewritten = fs::read_to_string(&definition_path).unwrap();
        assert!(rewritten.contains(&format!("id=\"{old}\"")));
        assert!(rewritten.contains(&format!("dff=\"{repaired}\"")));
        assert!(rewritten.contains(&format!("col=\"{repaired}\"")));
        assert!(rewritten.contains(&format!("txd=\"{repaired}\"")));
        assert_eq!(
            fs::read_to_string(result.backup_dir.join("zones/test/test.definition")).unwrap(),
            definition
        );
        assert!(result.backup_dir.join("vice.img").is_file());

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn archive_transaction_preserves_entries_and_uses_type_targets() {
        let root = unique_transaction_dir(&std::env::temp_dir(), "eagle_rebalance_test");
        let imgs = root.join("imgs");
        fs::create_dir_all(&imgs).unwrap();
        let old_meta = "<meta>\n    <file type=\"client\" src=\"imgs/vice_a.img\" />\n    <file type=\"client\" src=\"imgs/vice_b.img\" />\n</meta>\n";
        fs::write(root.join("meta.xml"), old_meta).unwrap();
        write_img_archive(
            &imgs.join("vice_a.img"),
            &[
                ("alpha.dff".to_string(), vec![1; 2048]),
                ("road.txd".to_string(), vec![2; 4096]),
            ],
        )
        .unwrap();
        thread::sleep(Duration::from_millis(20));
        let old_config = "<img name=\"vice_a\" max=\"2\" />\n";
        fs::write(root.join(RESOURCE_IMG_CONFIG), old_config).unwrap();
        write_img_archive(
            &imgs.join("vice_b.img"),
            &[
                ("alpha.dff".to_string(), vec![9; 2048]),
                ("alpha.col".to_string(), vec![3; 2048]),
                ("sign.txd".to_string(), vec![4; 4096]),
                ("notes.dat".to_string(), vec![5; 2048]),
            ],
        )
        .unwrap();

        let (progress_tx, _progress_rx) = mpsc::channel();
        let result =
            rebalance_img_archives(&root, 4 * IMG_ARCHIVE_SECTOR_BYTES, &progress_tx).unwrap();

        assert!(imgs.join("dff.img").is_file());
        assert!(imgs.join("col.img").is_file());
        assert!(imgs.join("txd_1.img").is_file());
        assert!(imgs.join("txd_2.img").is_file());
        assert!(imgs.join("custom.img").is_file());
        assert!(!imgs.join("vice_a.img").exists());
        assert!(!imgs.join("vice_b.img").exists());
        assert!(result.backup_dir.join("vice_a.img").is_file());
        assert!(result.backup_dir.join("vice_b.img").is_file());
        assert_eq!(
            fs::read_to_string(result.backup_dir.join("meta.xml")).unwrap(),
            old_meta
        );
        let updated_meta = fs::read_to_string(root.join("meta.xml")).unwrap();
        assert_eq!(updated_meta.matches("imgs/*.img").count(), 1);
        assert!(!updated_meta.contains("imgs/vice_a.img"));
        assert!(!updated_meta.contains("imgs/vice_b.img"));
        assert_eq!(
            fs::read_to_string(result.backup_dir.join(RESOURCE_IMG_CONFIG)).unwrap(),
            old_config
        );
        let updated_config = fs::read_to_string(root.join(RESOURCE_IMG_CONFIG)).unwrap();
        assert!(updated_config.contains("names=\"dff,col,txd,custom\""));
        assert!(updated_config.contains("max=\"64\""));
        assert_eq!(result.duplicate_resolutions.len(), 1);
        let chosen_dff = parse_img(&imgs.join("dff.img"))
            .into_iter()
            .find(|entry| entry.name.eq_ignore_ascii_case("alpha.dff"))
            .unwrap();
        assert_eq!(read_img_entry(&chosen_dff)[0], 9);
        assert!(
            result
                .archive_paths
                .iter()
                .all(|path| { fs::metadata(path).unwrap().len() <= 4 * IMG_ARCHIVE_SECTOR_BYTES })
        );

        fs::remove_dir_all(&root).unwrap();
    }
}
