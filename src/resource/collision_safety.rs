use crate::resource::eagle_scene::{
    SECTION_SAFE_COLLISIONS, read_eagle_scene_section, update_eagle_scene_section,
};
use serde_json::Value;
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

pub(crate) const SAFE_COLLISIONS_FILE: &str = "eagleSafeCollisions.json";
const SAFE_COLLISIONS_VERSION: u64 = 1;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SafeCollisions {
    names: BTreeSet<String>,
}

impl SafeCollisions {
    pub(crate) fn is_safe(&self, name: &str) -> bool {
        normalize_col_name(name).is_some_and(|name| self.names.contains(&name))
    }

    pub(crate) fn set_safe(&mut self, name: &str, safe: bool) -> bool {
        let Some(name) = normalize_col_name(name) else {
            return false;
        };
        if safe {
            self.names.insert(name)
        } else {
            self.names.remove(&name)
        }
    }

    pub(crate) fn entries(&self) -> impl Iterator<Item = &str> {
        self.names.iter().map(String::as_str)
    }
}

pub(crate) fn safe_collisions_path(root: &Path) -> PathBuf {
    root.join(SAFE_COLLISIONS_FILE)
}

pub(crate) fn load_safe_collisions(root: &Path) -> SafeCollisions {
    match read_eagle_scene_section(root, SECTION_SAFE_COLLISIONS) {
        Ok(Some(json)) => match parse_safe_collisions_value(&json, true) {
            Ok(safe) => return safe,
            Err(err) => {
                eprintln!("Invalid EagleScene safe-collisions section: {err}");
                return SafeCollisions::default();
            }
        },
        Ok(None) => {}
        Err(err) => {
            eprintln!("Could not load the EagleScene safe-collisions section: {err}");
            return SafeCollisions::default();
        }
    }

    let path = safe_collisions_path(root);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return SafeCollisions::default();
        }
        Err(err) => {
            eprintln!("Could not read {}: {err}", path.display());
            return SafeCollisions::default();
        }
    };
    let Ok(json) = serde_json::from_str::<Value>(&text) else {
        eprintln!("Invalid {SAFE_COLLISIONS_FILE}; safe collision flags were ignored.");
        return SafeCollisions::default();
    };
    match parse_safe_collisions_value(&json, false) {
        Ok(safe) => safe,
        Err(err) => {
            eprintln!("Invalid {SAFE_COLLISIONS_FILE}: {err}; flags were ignored.");
            SafeCollisions::default()
        }
    }
}

#[allow(dead_code)]
pub(crate) fn save_safe_collisions(root: &Path, safe: &SafeCollisions) -> Result<(), String> {
    update_eagle_scene_section(
        root,
        SECTION_SAFE_COLLISIONS,
        safe_collisions_section_value(safe),
    )
}

pub(crate) fn safe_collisions_section_value(safe: &SafeCollisions) -> Value {
    let entries = safe.entries().collect::<Vec<_>>();
    serde_json::json!({
        "safeCollisions": entries,
        "version": SAFE_COLLISIONS_VERSION,
    })
}

/// Strictly reads the legacy standalone document for verified migration.
pub(crate) fn load_legacy_safe_collisions_section(root: &Path) -> Result<Option<Value>, String> {
    let path = safe_collisions_path(root);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(format!("Could not read {}: {err}", path.display())),
    };
    let json = serde_json::from_str::<Value>(&text)
        .map_err(|err| format!("Invalid {}: {err}", path.display()))?;
    parse_safe_collisions_value(&json, true)
        .map_err(|err| format!("{} cannot be migrated safely: {err}", path.display()))?;
    Ok(Some(json))
}

fn parse_safe_collisions_value(json: &Value, strict: bool) -> Result<SafeCollisions, String> {
    let object = json
        .as_object()
        .ok_or_else(|| "the section root must be an object".to_string())?;
    if strict {
        match object.get("version").and_then(Value::as_u64) {
            Some(SAFE_COLLISIONS_VERSION) => {}
            Some(version) => {
                return Err(format!(
                    "unsupported version {version}; expected {SAFE_COLLISIONS_VERSION}"
                ));
            }
            None => return Err("missing numeric `version`".to_string()),
        }
    }
    let entries = object
        .get("safeCollisions")
        .and_then(Value::as_array)
        .ok_or_else(|| "`safeCollisions` must be an array".to_string())?;
    let mut safe = SafeCollisions::default();
    for (index, entry) in entries.iter().enumerate() {
        let Some(name) = entry.as_str() else {
            if strict {
                return Err(format!("safeCollisions[{index}] must be a string"));
            }
            continue;
        };
        if !safe.set_safe(name, true) {
            let normalized = normalize_col_name(name);
            if strict && normalized.is_none() {
                return Err(format!(
                    "safeCollisions[{index}] must contain a non-empty collision name"
                ));
            }
        }
    }
    Ok(safe)
}

fn normalize_col_name(name: &str) -> Option<String> {
    let mut name = name.trim().to_ascii_lowercase();
    if name.is_empty() {
        return None;
    }
    if !name.ends_with(".col") {
        name.push_str(".col");
    }
    Some(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_collision_names_are_normalized_and_round_trip() {
        let root = std::env::temp_dir().join(format!(
            "eagle_safe_collisions_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let mut safe = SafeCollisions::default();
        assert!(safe.set_safe(" Important_Building ", true));
        assert!(!safe.set_safe("important_building.COL", true));
        assert!(safe.is_safe("IMPORTANT_BUILDING"));

        save_safe_collisions(&root, &safe).unwrap();
        assert!(!safe_collisions_path(&root).exists());
        assert_eq!(load_safe_collisions(&root), safe);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn eagle_scene_safe_collisions_take_priority_over_legacy() {
        let root = std::env::temp_dir().join(format!(
            "eagle_safe_collisions_priority_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            safe_collisions_path(&root),
            r#"{"version":1,"safeCollisions":["legacy.col"]}"#,
        )
        .unwrap();
        update_eagle_scene_section(
            &root,
            SECTION_SAFE_COLLISIONS,
            serde_json::json!({
                "version": 1,
                "safeCollisions": ["unified.col"]
            }),
        )
        .unwrap();

        let loaded = load_safe_collisions(&root);
        assert!(loaded.is_safe("unified"));
        assert!(!loaded.is_safe("legacy"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn legacy_safe_collisions_are_a_missing_section_fallback() {
        let root = std::env::temp_dir().join(format!(
            "eagle_safe_collisions_fallback_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            safe_collisions_path(&root),
            r#"{"version":1,"safeCollisions":["legacy.col"]}"#,
        )
        .unwrap();

        assert!(load_safe_collisions(&root).is_safe("legacy"));
        assert!(
            load_legacy_safe_collisions_section(&root)
                .unwrap()
                .is_some()
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn strict_legacy_safe_collision_import_rejects_invalid_entries() {
        let root = std::env::temp_dir().join(format!(
            "eagle_safe_collisions_strict_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            safe_collisions_path(&root),
            r#"{"version":1,"safeCollisions":["valid.col",12]}"#,
        )
        .unwrap();

        assert!(load_legacy_safe_collisions_section(&root).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
