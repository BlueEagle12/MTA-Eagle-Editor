use serde_json::{Map, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

pub(crate) const EAGLE_SCENE_FILE: &str = "EagleScene.eaglescne";
pub(crate) const EAGLE_SCENE_FORMAT: &str = "EagleScene";
pub(crate) const EAGLE_SCENE_VERSION: u64 = 1;

pub(crate) const SECTION_MATERIAL_EMITTERS: &str = "materialEmitters";
pub(crate) const SECTION_SHADOW_CASTERS: &str = "shadowCasters";
pub(crate) const SECTION_COLLISION_CAPSULES: &str = "collisionCapsules";
pub(crate) const SECTION_COLLISION_CUBOIDS: &str = "collisionCuboids";
pub(crate) const SECTION_MATERIAL_CLASSES: &str = "materialClasses";
pub(crate) const SECTION_SAFE_COLLISIONS: &str = "safeCollisions";
pub(crate) const SECTION_LIGHTS: &str = "lights";

const LEGACY_MATERIAL_EMITTERS: &str = "Light_Emitters.json";
const LEGACY_SHADOW_CASTERS: &str = "Shadow_Casters.json";
const LEGACY_COLLISION_CAPSULES: &str = "Collision_Capsules.json";
const LEGACY_COLLISION_CUBOIDS: &str = "Collision_Cuboids.json";
const LEGACY_MATERIAL_CLASSES: &str = "eagleMaterialClasses.json";
const LEGACY_SAFE_COLLISIONS: &str = "eagleSafeCollisions.json";
const LEGACY_LIGHTS: &str = "Light_List.xml";
const LEGACY_PROJECT_SESSION: &str = ".light_mapper_session.json";
const LEGACY_MATERIAL_BACKUP: &str = "eagleMaterialClasses.name-scope-backup.json";

static SCENE_IO_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static TEMP_NONCE: AtomicU64 = AtomicU64::new(0);

fn scene_io_lock() -> &'static Mutex<()> {
    SCENE_IO_LOCK.get_or_init(|| Mutex::new(()))
}

fn lock_scene_io() -> Result<std::sync::MutexGuard<'static, ()>, String> {
    scene_io_lock()
        .lock()
        .map_err(|_| "EagleScene process-wide I/O lock was poisoned".to_string())
}

pub(crate) fn eagle_scene_path(root: &Path) -> PathBuf {
    root.join(EAGLE_SCENE_FILE)
}

#[derive(Clone, Debug)]
pub(crate) struct EagleSceneSectionUpdate {
    section: String,
    value: Value,
}

impl EagleSceneSectionUpdate {
    pub(crate) fn new(section: impl Into<String>, value: Value) -> Result<Self, String> {
        let section = section.into();
        if section.trim().is_empty() {
            return Err("EagleScene section name cannot be empty".to_string());
        }
        if !value.is_object() {
            return Err(format!(
                "EagleScene section `{section}` must be a JSON object"
            ));
        }
        Ok(Self { section, value })
    }
}

#[derive(Clone, Debug)]
pub(crate) struct LegacySceneImport {
    pub(crate) file_name: String,
    pub(crate) value: Value,
}

impl LegacySceneImport {
    pub(crate) fn new(file_name: impl Into<String>, value: Value) -> Result<Self, String> {
        let file_name = file_name.into();
        let Some(section) = legacy_section_for_file(&file_name) else {
            return Err(format!(
                "`{file_name}` is not an allowlisted EagleScene legacy source"
            ));
        };
        if section.is_none() {
            return Err(format!(
                "`{file_name}` is archive-only and cannot supply an EagleScene section"
            ));
        }
        if !value.is_object() {
            return Err(format!(
                "Converted legacy section from `{file_name}` must be a JSON object"
            ));
        }
        Ok(Self { file_name, value })
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PreparedEagleSceneMigration {
    root: PathBuf,
    archive_dir: PathBuf,
    originals: Vec<PreparedLegacyOriginal>,
    migrated_sections: usize,
}

#[derive(Clone, Debug)]
struct PreparedLegacyOriginal {
    file_name: String,
    bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EagleSceneMigrationOutcome {
    pub(crate) archive_dir: PathBuf,
    pub(crate) migrated_sections: usize,
    pub(crate) archived_files: usize,
}

fn legacy_specs() -> [(&'static str, Option<&'static str>); 9] {
    [
        (LEGACY_MATERIAL_EMITTERS, Some(SECTION_MATERIAL_EMITTERS)),
        (LEGACY_SHADOW_CASTERS, Some(SECTION_SHADOW_CASTERS)),
        (LEGACY_COLLISION_CAPSULES, Some(SECTION_COLLISION_CAPSULES)),
        (LEGACY_COLLISION_CUBOIDS, Some(SECTION_COLLISION_CUBOIDS)),
        (LEGACY_MATERIAL_CLASSES, Some(SECTION_MATERIAL_CLASSES)),
        (LEGACY_SAFE_COLLISIONS, Some(SECTION_SAFE_COLLISIONS)),
        (LEGACY_LIGHTS, Some(SECTION_LIGHTS)),
        (LEGACY_PROJECT_SESSION, None),
        (LEGACY_MATERIAL_BACKUP, None),
    ]
}

fn legacy_section_for_file(file_name: &str) -> Option<Option<&'static str>> {
    legacy_specs()
        .into_iter()
        .find(|(candidate, _)| *candidate == file_name)
        .map(|(_, section)| section)
}

fn new_document() -> Value {
    let mut root = Map::new();
    root.insert(
        "format".to_string(),
        Value::String(EAGLE_SCENE_FORMAT.to_string()),
    );
    root.insert(
        "version".to_string(),
        Value::Number(EAGLE_SCENE_VERSION.into()),
    );
    root.insert("revision".to_string(), Value::Number(0u64.into()));
    root.insert("sections".to_string(), Value::Object(Map::new()));
    Value::Object(root)
}

fn validate_document(value: &Value, path: &Path) -> Result<(), String> {
    let Some(root) = value.as_object() else {
        return Err(format!("{} must contain a JSON object", path.display()));
    };
    match root.get("format").and_then(Value::as_str) {
        Some(EAGLE_SCENE_FORMAT) => {}
        Some(format) => {
            return Err(format!(
                "{} uses unsupported format `{format}`",
                path.display()
            ));
        }
        None => {
            return Err(format!("{} has no valid `format` field", path.display()));
        }
    }
    match root.get("version").and_then(Value::as_u64) {
        Some(EAGLE_SCENE_VERSION) => {}
        Some(version) => {
            return Err(format!(
                "{} uses unsupported EagleScene version {version}; this editor supports only version {EAGLE_SCENE_VERSION}",
                path.display()
            ));
        }
        None => {
            return Err(format!("{} has no valid numeric `version`", path.display()));
        }
    }
    if root.get("revision").and_then(Value::as_u64).is_none() {
        return Err(format!(
            "{} has no valid numeric `revision`",
            path.display()
        ));
    }
    let Some(sections) = root.get("sections").and_then(Value::as_object) else {
        return Err(format!("{} has no valid `sections` object", path.display()));
    };
    if let Some((section, _)) = sections.iter().find(|(_, value)| !value.is_object()) {
        return Err(format!(
            "{} section `{section}` must be a JSON object",
            path.display()
        ));
    }
    Ok(())
}

fn read_document_unlocked(root: &Path) -> Result<Option<Value>, String> {
    let path = eagle_scene_path(root);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(format!("Could not read {}: {err}", path.display())),
    };
    let value = serde_json::from_slice::<Value>(&bytes)
        .map_err(|err| format!("Invalid {}: {err}", path.display()))?;
    validate_document(&value, &path)?;
    Ok(Some(value))
}

pub(crate) fn read_eagle_scene(root: &Path) -> Result<Option<Value>, String> {
    let _guard = lock_scene_io()?;
    read_document_unlocked(root)
}

pub(crate) fn read_eagle_scene_section(
    root: &Path,
    section: &str,
) -> Result<Option<Value>, String> {
    let _guard = lock_scene_io()?;
    let Some(document) = read_document_unlocked(root)? else {
        return Ok(None);
    };
    Ok(document
        .get("sections")
        .and_then(|sections| sections.get(section))
        .cloned())
}

fn next_temp_path(root: &Path) -> PathBuf {
    let nonce = TEMP_NONCE.fetch_add(1, Ordering::Relaxed);
    root.join(format!(
        ".{EAGLE_SCENE_FILE}.tmp-{}-{nonce}",
        std::process::id()
    ))
}

fn write_document_unlocked(root: &Path, document: &Value) -> Result<(), String> {
    fs::create_dir_all(root)
        .map_err(|err| format!("Could not create {}: {err}", root.display()))?;
    let path = eagle_scene_path(root);
    validate_document(document, &path)?;
    let mut bytes = serde_json::to_vec_pretty(document)
        .map_err(|err| format!("Could not serialize {}: {err}", path.display()))?;
    bytes.push(b'\n');
    // serde_json may normalize the final decimal digit of an in-memory f64
    // while formatting it (for example 0.9599999785423279 becomes
    // 0.959999978542328). Verify against the semantic value represented by
    // the exact staged bytes, not the pre-serialization Number internals.
    let canonical_document = serde_json::from_slice::<Value>(&bytes)
        .map_err(|err| format!("Could not verify serialized {}: {err}", path.display()))?;

    let (temporary, mut file) = loop {
        let temporary = next_temp_path(root);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
        {
            Ok(file) => break (temporary, file),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => {
                return Err(format!(
                    "Could not create temporary EagleScene file {}: {err}",
                    temporary.display()
                ));
            }
        }
    };
    let staged = file
        .write_all(&bytes)
        .and_then(|_| file.flush())
        .and_then(|_| file.sync_all());
    if let Err(err) = staged {
        let _ = fs::remove_file(&temporary);
        return Err(format!("Could not stage {}: {err}", temporary.display()));
    }
    drop(file);
    if let Err(err) = fs::rename(&temporary, &path) {
        let _ = fs::remove_file(&temporary);
        return Err(format!(
            "Could not atomically replace {}: {err}",
            path.display()
        ));
    }
    if let Ok(directory) = File::open(root) {
        let _ = directory.sync_all();
    }
    let verified = read_document_unlocked(root)?
        .ok_or_else(|| format!("{} disappeared after writing", path.display()))?;
    if verified != canonical_document {
        let difference = first_semantic_difference(&canonical_document, &verified, "$")
            .unwrap_or_else(|| "unknown value mismatch".to_string());
        return Err(format!(
            "{} failed semantic verification after writing: {difference}",
            path.display(),
        ));
    }
    Ok(())
}

fn first_semantic_difference(expected: &Value, actual: &Value, path: &str) -> Option<String> {
    if expected == actual {
        return None;
    }
    match (expected, actual) {
        (Value::Object(expected), Value::Object(actual)) => {
            let keys = expected
                .keys()
                .chain(actual.keys())
                .map(String::as_str)
                .collect::<BTreeSet<_>>();
            for key in keys {
                let child = format!("{path}.{key}");
                match (expected.get(key), actual.get(key)) {
                    (Some(expected), Some(actual)) => {
                        if let Some(difference) =
                            first_semantic_difference(expected, actual, &child)
                        {
                            return Some(difference);
                        }
                    }
                    (Some(_), None) => return Some(format!("{child} is missing after writing")),
                    (None, Some(_)) => return Some(format!("{child} appeared after writing")),
                    (None, None) => {}
                }
            }
            None
        }
        (Value::Array(expected), Value::Array(actual)) => {
            if expected.len() != actual.len() {
                return Some(format!(
                    "{path} array length changed from {} to {}",
                    expected.len(),
                    actual.len()
                ));
            }
            for (index, (expected, actual)) in expected.iter().zip(actual).enumerate() {
                if let Some(difference) =
                    first_semantic_difference(expected, actual, &format!("{path}[{index}]"))
                {
                    return Some(difference);
                }
            }
            None
        }
        _ => Some(format!(
            "{path} changed from {} to {}",
            compact_json_value(expected),
            compact_json_value(actual)
        )),
    }
}

fn compact_json_value(value: &Value) -> String {
    let text = serde_json::to_string(value).unwrap_or_else(|_| format!("{value:?}"));
    if text.len() <= 160 {
        text
    } else {
        format!("{}...", &text[..160])
    }
}

fn merge_section(document: &mut Value, update: &EagleSceneSectionUpdate) -> Result<(), String> {
    let sections = document
        .get_mut("sections")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "EagleScene document has no sections object".to_string())?;
    let incoming = update
        .value
        .as_object()
        .ok_or_else(|| format!("EagleScene section `{}` must be an object", update.section))?;
    match sections.get_mut(&update.section) {
        Some(Value::Object(existing)) => {
            for (key, value) in incoming {
                existing.insert(key.clone(), value.clone());
            }
        }
        Some(_) => {
            return Err(format!(
                "Existing EagleScene section `{}` is not an object",
                update.section
            ));
        }
        None => {
            sections.insert(update.section.clone(), update.value.clone());
        }
    }
    Ok(())
}

fn increment_revision(document: &mut Value) -> Result<(), String> {
    let revision = document
        .get("revision")
        .and_then(Value::as_u64)
        .ok_or_else(|| "EagleScene revision is invalid".to_string())?;
    let revision = revision
        .checked_add(1)
        .ok_or_else(|| "EagleScene revision overflowed".to_string())?;
    document["revision"] = Value::Number(revision.into());
    Ok(())
}

fn update_sections_unlocked(
    root: &Path,
    updates: &[EagleSceneSectionUpdate],
) -> Result<(), String> {
    if updates.is_empty() {
        return Ok(());
    }
    let mut names = BTreeSet::new();
    for update in updates {
        if !names.insert(update.section.as_str()) {
            return Err(format!(
                "Duplicate EagleScene batch update for section `{}`",
                update.section
            ));
        }
    }
    let mut document = read_document_unlocked(root)?.unwrap_or_else(new_document);
    for update in updates {
        merge_section(&mut document, update)?;
    }
    increment_revision(&mut document)?;
    write_document_unlocked(root, &document)
}

pub(crate) fn update_eagle_scene_section(
    root: &Path,
    section: &str,
    value: Value,
) -> Result<(), String> {
    let update = EagleSceneSectionUpdate::new(section, value)?;
    update_eagle_scene_sections(root, &[update])
}

pub(crate) fn update_eagle_scene_sections(
    root: &Path,
    updates: &[EagleSceneSectionUpdate],
) -> Result<(), String> {
    let _guard = lock_scene_io()?;
    update_sections_unlocked(root, updates)
}

fn unique_archive_dir(root: &Path) -> Result<PathBuf, String> {
    let legacy = root.join("legacy");
    fs::create_dir_all(&legacy)
        .map_err(|err| format!("Could not create {}: {err}", legacy.display()))?;
    for _ in 0..1000 {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let nonce = TEMP_NONCE.fetch_add(1, Ordering::Relaxed);
        let path = legacy.join(format!(
            "eaglescne-migration-{timestamp}-{}-{nonce}",
            std::process::id()
        ));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(format!("Could not create {}: {err}", path.display())),
        }
    }
    Err("Could not allocate a unique EagleScene migration archive".to_string())
}

fn write_exact_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|err| format!("Could not create {}: {err}", path.display()))?;
    file.write_all(bytes)
        .and_then(|_| file.flush())
        .and_then(|_| file.sync_all())
        .map_err(|err| format!("Could not write {}: {err}", path.display()))?;
    let copied =
        fs::read(path).map_err(|err| format!("Could not verify {}: {err}", path.display()))?;
    if copied != bytes {
        return Err(format!("Byte verification failed for {}", path.display()));
    }
    Ok(())
}

fn write_migration_note(
    archive_dir: &Path,
    originals: &[PreparedLegacyOriginal],
    finalized: bool,
) -> Result<(), String> {
    let mut note = String::from(
        "Eagle Editor legacy sidecar migration\n\
         Canonical scene document: EagleScene.eaglescne\n",
    );
    note.push_str(if finalized {
        "Status: finalized; verified source sidecars were removed from the project root.\n"
    } else {
        "Status: prepared; exact source copies verified, originals retained pending Resource Save success.\n"
    });
    note.push_str("Files:\n");
    for original in originals {
        note.push_str(&format!(
            "- {} ({} bytes)\n",
            original.file_name,
            original.bytes.len()
        ));
    }
    let path = archive_dir.join("MIGRATION.txt");
    if path.exists() {
        fs::remove_file(&path)
            .map_err(|err| format!("Could not replace {}: {err}", path.display()))?;
    }
    write_exact_file(&path, note.as_bytes())
}

pub(crate) fn prepare_eagle_scene_migration(
    root: &Path,
    imports: &[LegacySceneImport],
) -> Result<Option<PreparedEagleSceneMigration>, String> {
    let _guard = lock_scene_io()?;
    let mut imports_by_file = BTreeMap::new();
    for import in imports {
        if imports_by_file
            .insert(import.file_name.as_str(), import)
            .is_some()
        {
            return Err(format!(
                "Duplicate EagleScene legacy import `{}`",
                import.file_name
            ));
        }
    }

    let mut originals = Vec::new();
    let mut updates = Vec::new();
    for (file_name, section) in legacy_specs() {
        let path = root.join(file_name);
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                if imports_by_file.contains_key(file_name) {
                    return Err(format!(
                        "Legacy import `{file_name}` was supplied but the source file is missing"
                    ));
                }
                continue;
            }
            Err(err) => return Err(format!("Could not read {}: {err}", path.display())),
        };
        if let Some(section) = section {
            let import = imports_by_file.get(file_name).ok_or_else(|| {
                format!("No strict parsed import was supplied for legacy file `{file_name}`")
            })?;
            updates.push(EagleSceneSectionUpdate::new(section, import.value.clone())?);
        }
        originals.push(PreparedLegacyOriginal {
            file_name: file_name.to_string(),
            bytes,
        });
    }
    for file_name in imports_by_file.keys() {
        if !originals
            .iter()
            .any(|original| original.file_name == **file_name)
        {
            return Err(format!(
                "Legacy import `{file_name}` does not have an allowlisted source file"
            ));
        }
    }
    if originals.is_empty() {
        return Ok(None);
    }

    // Validate the existing document before creating an archive. Unsupported
    // or corrupt documents must never make the legacy originals look migrated.
    // A canonical section always wins over a leftover legacy copy.
    let existing = read_document_unlocked(root)?;
    if let Some(document) = existing.as_ref() {
        let sections = document["sections"]
            .as_object()
            .expect("validated EagleScene sections");
        updates.retain(|update| !sections.contains_key(&update.section));
    }
    let migrated_sections = updates.len();
    let archive_dir = unique_archive_dir(root)?;
    for original in &originals {
        write_exact_file(&archive_dir.join(&original.file_name), &original.bytes)?;
    }
    write_migration_note(&archive_dir, &originals, false)?;
    if updates.is_empty() {
        if read_document_unlocked(root)?.is_none() {
            let mut document = new_document();
            increment_revision(&mut document)?;
            write_document_unlocked(root, &document)?;
        }
    } else {
        update_sections_unlocked(root, &updates)?;
    }
    Ok(Some(PreparedEagleSceneMigration {
        root: root.to_path_buf(),
        archive_dir,
        originals,
        migrated_sections,
    }))
}

pub(crate) fn finalize_eagle_scene_migration(
    prepared: PreparedEagleSceneMigration,
) -> Result<EagleSceneMigrationOutcome, String> {
    let _guard = lock_scene_io()?;
    let document = read_document_unlocked(&prepared.root)?.ok_or_else(|| {
        format!(
            "{} is missing; refusing to remove legacy originals",
            eagle_scene_path(&prepared.root).display()
        )
    })?;
    validate_document(&document, &eagle_scene_path(&prepared.root))?;
    for original in &prepared.originals {
        let source = prepared.root.join(&original.file_name);
        let current = fs::read(&source)
            .map_err(|err| format!("Could not re-verify {}: {err}", source.display()))?;
        if current != original.bytes {
            return Err(format!(
                "{} changed after migration was prepared; no remaining originals were removed",
                source.display()
            ));
        }
        let archived = fs::read(prepared.archive_dir.join(&original.file_name)).map_err(|err| {
            format!(
                "Could not re-verify archived `{}`: {err}",
                original.file_name
            )
        })?;
        if archived != original.bytes {
            return Err(format!(
                "Archived `{}` no longer matches its source; originals were retained",
                original.file_name
            ));
        }
    }
    // Every byte comparison is complete before the first removal.
    for original in &prepared.originals {
        fs::remove_file(prepared.root.join(&original.file_name)).map_err(|err| {
            format!(
                "Could not remove migrated legacy file `{}`: {err}",
                original.file_name
            )
        })?;
    }
    write_migration_note(&prepared.archive_dir, &prepared.originals, true)?;
    Ok(EagleSceneMigrationOutcome {
        archive_dir: prepared.archive_dir,
        migrated_sections: prepared.migrated_sections,
        archived_files: prepared.originals.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "eagle_scene_{label}_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn write_document_for_test(root: &Path, value: &Value) {
        fs::create_dir_all(root).unwrap();
        fs::write(
            eagle_scene_path(root),
            serde_json::to_vec_pretty(value).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn atomic_updates_round_trip_and_increment_revision() {
        let root = temp_root("atomic");
        update_eagle_scene_section(
            &root,
            SECTION_LIGHTS,
            serde_json::json!({
                "zVendor": true,
                "items": [{"name": "Sun"}],
                "aVendor": true
            }),
        )
        .unwrap();
        update_eagle_scene_sections(
            &root,
            &[
                EagleSceneSectionUpdate::new(
                    SECTION_SAFE_COLLISIONS,
                    serde_json::json!({"safeCollisions": ["tower.col"]}),
                )
                .unwrap(),
                EagleSceneSectionUpdate::new(
                    SECTION_COLLISION_CUBOIDS,
                    serde_json::json!({"assets": {}}),
                )
                .unwrap(),
            ],
        )
        .unwrap();
        let document = read_eagle_scene(&root).unwrap().unwrap();
        assert_eq!(document["format"], EAGLE_SCENE_FORMAT);
        assert_eq!(document["version"], EAGLE_SCENE_VERSION);
        assert_eq!(document["revision"], 2);
        assert_eq!(
            read_eagle_scene_section(&root, SECTION_LIGHTS)
                .unwrap()
                .unwrap()["items"][0]["name"],
            "Sun"
        );
        assert!(
            fs::read_dir(&root)
                .unwrap()
                .filter_map(Result::ok)
                .all(|entry| !entry.file_name().to_string_lossy().contains(".tmp-"))
        );
        let text = fs::read_to_string(eagle_scene_path(&root)).unwrap();
        assert!(text.ends_with('\n'));
        assert!(text.find("\"aVendor\"").unwrap() < text.find("\"zVendor\"").unwrap());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn serializer_decimal_normalization_is_semantically_verified() {
        let root = temp_root("float_normalization");
        update_eagle_scene_section(
            &root,
            SECTION_LIGHTS,
            serde_json::json!({
                "lights": [{
                    "color": [1.0, 0.9599999785423279f64, 0.5]
                }]
            }),
        )
        .unwrap();
        let value = read_eagle_scene_section(&root, SECTION_LIGHTS)
            .unwrap()
            .unwrap();
        assert_eq!(
            value["lights"][0]["color"][1].as_f64(),
            Some(0.959999978542328)
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn updates_preserve_unknown_document_and_section_fields() {
        let root = temp_root("unknown");
        write_document_for_test(
            &root,
            &serde_json::json!({
                "format": EAGLE_SCENE_FORMAT,
                "version": EAGLE_SCENE_VERSION,
                "revision": 7,
                "vendorTop": {"keep": true},
                "sections": {
                    "materialEmitters": {
                        "version": 2,
                        "emitters": [],
                        "vendorSection": [1, 2, 3]
                    },
                    "futureSection": {"payload": 9}
                }
            }),
        );
        update_eagle_scene_section(
            &root,
            SECTION_MATERIAL_EMITTERS,
            serde_json::json!({"version": 2, "emitters": [{"enabled": true}]}),
        )
        .unwrap();
        let document = read_eagle_scene(&root).unwrap().unwrap();
        assert_eq!(document["revision"], 8);
        assert_eq!(document["vendorTop"]["keep"], true);
        assert_eq!(
            document["sections"]["materialEmitters"]["vendorSection"],
            serde_json::json!([1, 2, 3])
        );
        assert_eq!(document["sections"]["futureSection"]["payload"], 9);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_and_newer_documents_are_rejected_without_rewrite() {
        let root = temp_root("invalid");
        fs::create_dir_all(&root).unwrap();
        fs::write(eagle_scene_path(&root), b"{nope").unwrap();
        assert!(read_eagle_scene(&root).unwrap_err().contains("Invalid"));
        let newer = serde_json::json!({
            "format": EAGLE_SCENE_FORMAT,
            "version": EAGLE_SCENE_VERSION + 1,
            "revision": 1,
            "sections": {}
        });
        write_document_for_test(&root, &newer);
        assert!(read_eagle_scene(&root).unwrap_err().contains("unsupported"));
        let before = fs::read(eagle_scene_path(&root)).unwrap();
        assert!(
            update_eagle_scene_section(&root, SECTION_LIGHTS, serde_json::json!({"items":[]}))
                .is_err()
        );
        assert_eq!(fs::read(eagle_scene_path(&root)).unwrap(), before);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn migration_archives_exact_bytes_then_finalizes_and_is_idempotent() {
        let root = temp_root("migration");
        fs::create_dir_all(&root).unwrap();
        let emitters = b"{\n  \"version\": 2,\n  \"emitters\": []\n}\n";
        let session = b"{\"camera\":{\"yaw\":1}}\n";
        fs::write(root.join(LEGACY_MATERIAL_EMITTERS), emitters).unwrap();
        fs::write(root.join(LEGACY_PROJECT_SESSION), session).unwrap();
        let imports = vec![
            LegacySceneImport::new(
                LEGACY_MATERIAL_EMITTERS,
                serde_json::json!({"version": 2, "emitters": []}),
            )
            .unwrap(),
        ];
        let prepared = prepare_eagle_scene_migration(&root, &imports)
            .unwrap()
            .unwrap();
        assert!(root.join(LEGACY_MATERIAL_EMITTERS).is_file());
        assert!(root.join(LEGACY_PROJECT_SESSION).is_file());
        assert_eq!(
            fs::read(prepared.archive_dir.join(LEGACY_MATERIAL_EMITTERS)).unwrap(),
            emitters
        );
        assert_eq!(
            fs::read(prepared.archive_dir.join(LEGACY_PROJECT_SESSION)).unwrap(),
            session
        );
        let outcome = finalize_eagle_scene_migration(prepared).unwrap();
        assert_eq!(outcome.migrated_sections, 1);
        assert_eq!(outcome.archived_files, 2);
        assert!(!root.join(LEGACY_MATERIAL_EMITTERS).exists());
        assert!(!root.join(LEGACY_PROJECT_SESSION).exists());
        assert!(outcome.archive_dir.join("MIGRATION.txt").is_file());
        assert!(prepare_eagle_scene_migration(&root, &[]).unwrap().is_none());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn migration_failure_leaves_all_originals() {
        let root = temp_root("migration_failure");
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join(LEGACY_SAFE_COLLISIONS),
            b"{\"version\":1,\"safeCollisions\":[\"tower.col\"]}\n",
        )
        .unwrap();
        write_document_for_test(
            &root,
            &serde_json::json!({
                "format": EAGLE_SCENE_FORMAT,
                "version": EAGLE_SCENE_VERSION + 1,
                "revision": 1,
                "sections": {}
            }),
        );
        let imports = vec![
            LegacySceneImport::new(
                LEGACY_SAFE_COLLISIONS,
                serde_json::json!({"version": 1, "safeCollisions": ["tower.col"]}),
            )
            .unwrap(),
        ];
        assert!(prepare_eagle_scene_migration(&root, &imports).is_err());
        assert!(root.join(LEGACY_SAFE_COLLISIONS).is_file());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn existing_unified_section_wins_while_legacy_is_archived() {
        let root = temp_root("unified_precedence");
        fs::create_dir_all(&root).unwrap();
        update_eagle_scene_section(
            &root,
            SECTION_LIGHTS,
            serde_json::json!({"version": 1, "lights": [{"name": "Unified"}]}),
        )
        .unwrap();
        let revision = read_eagle_scene(&root).unwrap().unwrap()["revision"]
            .as_u64()
            .unwrap();
        fs::write(
            root.join(LEGACY_LIGHTS),
            b"<Light_List><light name=\"Legacy\" /></Light_List>\n",
        )
        .unwrap();
        let imports = vec![
            LegacySceneImport::new(
                LEGACY_LIGHTS,
                serde_json::json!({"version": 1, "lights": [{"name": "Legacy"}]}),
            )
            .unwrap(),
        ];
        let prepared = prepare_eagle_scene_migration(&root, &imports)
            .unwrap()
            .unwrap();
        let outcome = finalize_eagle_scene_migration(prepared).unwrap();
        assert_eq!(outcome.migrated_sections, 0);
        assert_eq!(outcome.archived_files, 1);
        let document = read_eagle_scene(&root).unwrap().unwrap();
        assert_eq!(document["revision"], revision);
        assert_eq!(
            document["sections"][SECTION_LIGHTS]["lights"][0]["name"],
            "Unified"
        );
        assert!(!root.join(LEGACY_LIGHTS).exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn finalize_refuses_changed_original_without_removing_any_file() {
        let root = temp_root("changed");
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join(LEGACY_SAFE_COLLISIONS),
            b"{\"version\":1,\"safeCollisions\":[]}\n",
        )
        .unwrap();
        fs::write(root.join(LEGACY_PROJECT_SESSION), b"{}\n").unwrap();
        let imports = vec![
            LegacySceneImport::new(
                LEGACY_SAFE_COLLISIONS,
                serde_json::json!({"version": 1, "safeCollisions": []}),
            )
            .unwrap(),
        ];
        let prepared = prepare_eagle_scene_migration(&root, &imports)
            .unwrap()
            .unwrap();
        fs::write(root.join(LEGACY_PROJECT_SESSION), b"{\"changed\":true}\n").unwrap();
        assert!(finalize_eagle_scene_migration(prepared).is_err());
        assert!(root.join(LEGACY_SAFE_COLLISIONS).is_file());
        assert!(root.join(LEGACY_PROJECT_SESSION).is_file());
        fs::remove_dir_all(root).unwrap();
    }
}
