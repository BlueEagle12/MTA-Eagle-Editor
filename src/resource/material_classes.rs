use crate::assets::txd::TextureContentFingerprint;
#[cfg(test)]
use crate::assets::txd::TxdTextureIndex;
use crate::resource::eagle_scene::{
    SECTION_MATERIAL_CLASSES, read_eagle_scene_section, update_eagle_scene_section,
};
use serde_json::{Map, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

pub(crate) const MATERIAL_CLASSES_FILE: &str = "eagleMaterialClasses.json";
const MATERIAL_CLASSES_VERSION: u64 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MaterialClassSource {
    TxdOverride,
    ContentFingerprint,
    Global,
    Fallback,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ResolvedCollisionMaterial {
    pub(crate) material: u8,
    pub(crate) no_collision: bool,
    pub(crate) source: MaterialClassSource,
}

/// Persistent texture-to-COL-material assignments and collision exclusions.
///
/// Names are normalized on every public operation so callers do not need to
/// match the case used by a DFF, TXD, or sidecar. A TXD-specific assignment
/// takes precedence over a global texture-name assignment.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct TextureMaterialClasses {
    global: BTreeMap<String, u8>,
    by_txd: BTreeMap<(String, String), u8>,
    by_content: BTreeMap<TextureContentFingerprint, u8>,
    global_no_collision: BTreeSet<String>,
    by_txd_no_collision: BTreeSet<(String, String)>,
    by_content_no_collision: BTreeSet<TextureContentFingerprint>,
    texture_categories: BTreeMap<(String, String), String>,
}

impl TextureMaterialClasses {
    pub(crate) fn len(&self) -> usize {
        self.global.len()
            + self.by_txd.len()
            + self.by_content.len()
            + self.global_no_collision.len()
            + self.by_txd_no_collision.len()
            + self.by_content_no_collision.len()
    }

    pub(crate) fn texture_category(&self, txd: &str, texture: &str) -> Option<&str> {
        let txd = normalize_txd_name(txd)?;
        let texture = normalize_texture_name(texture)?;
        self.texture_categories
            .get(&(txd, texture))
            .map(String::as_str)
    }

    /// Assigns an editor browsing category to one texture in one TXD.
    /// An empty category clears the assignment.
    pub(crate) fn set_texture_category(
        &mut self,
        txd: &str,
        texture: &str,
        category: Option<&str>,
    ) -> bool {
        let Some(txd) = normalize_txd_name(txd) else {
            return false;
        };
        let Some(texture) = normalize_texture_name(texture) else {
            return false;
        };
        let key = (txd, texture);
        let category = category.map(str::trim).filter(|value| !value.is_empty());
        match category {
            Some(category) => {
                let normalized = category.split_whitespace().collect::<Vec<_>>().join(" ");
                if self.texture_categories.get(&key) == Some(&normalized) {
                    false
                } else {
                    self.texture_categories.insert(key, normalized);
                    true
                }
            }
            None => self.texture_categories.remove(&key).is_some(),
        }
    }

    pub(crate) fn texture_category_names(&self) -> BTreeSet<String> {
        self.texture_categories.values().cloned().collect()
    }

    pub(crate) fn global_material(&self, texture: &str) -> Option<u8> {
        let texture = normalize_texture_name(texture)?;
        self.global.get(&texture).copied()
    }

    pub(crate) fn txd_material(&self, txd: &str, texture: &str) -> Option<u8> {
        let txd = normalize_txd_name(txd)?;
        let texture = normalize_texture_name(texture)?;
        self.by_txd.get(&(txd, texture)).copied()
    }

    pub(crate) fn global_is_no_collision(&self, texture: &str) -> bool {
        normalize_texture_name(texture)
            .is_some_and(|texture| self.global_no_collision.contains(&texture))
    }

    pub(crate) fn txd_is_no_collision(&self, txd: &str, texture: &str) -> bool {
        let Some(txd) = normalize_txd_name(txd) else {
            return false;
        };
        let Some(texture) = normalize_texture_name(texture) else {
            return false;
        };
        self.by_txd_no_collision.contains(&(txd, texture))
    }

    pub(crate) fn content_material(&self, fingerprint: TextureContentFingerprint) -> Option<u8> {
        self.by_content.get(&fingerprint).copied()
    }

    pub(crate) fn content_is_no_collision(&self, fingerprint: TextureContentFingerprint) -> bool {
        self.by_content_no_collision.contains(&fingerprint)
    }

    pub(crate) fn resolve_with_content(
        &self,
        txd: Option<&str>,
        texture: &str,
        fingerprint: Option<TextureContentFingerprint>,
        fallback: u8,
    ) -> ResolvedCollisionMaterial {
        if txd.is_some_and(|txd| self.txd_is_no_collision(txd, texture)) {
            return ResolvedCollisionMaterial {
                material: fallback,
                no_collision: true,
                source: MaterialClassSource::TxdOverride,
            };
        }
        if let Some(material) = txd.and_then(|txd| self.txd_material(txd, texture)) {
            return ResolvedCollisionMaterial {
                material,
                no_collision: false,
                source: MaterialClassSource::TxdOverride,
            };
        }
        if fingerprint.is_some_and(|fingerprint| self.content_is_no_collision(fingerprint)) {
            return ResolvedCollisionMaterial {
                material: fallback,
                no_collision: true,
                source: MaterialClassSource::ContentFingerprint,
            };
        }
        if let Some(material) =
            fingerprint.and_then(|fingerprint| self.content_material(fingerprint))
        {
            return ResolvedCollisionMaterial {
                material,
                no_collision: false,
                source: MaterialClassSource::ContentFingerprint,
            };
        }
        if self.global_is_no_collision(texture) {
            return ResolvedCollisionMaterial {
                material: fallback,
                no_collision: true,
                source: MaterialClassSource::Global,
            };
        }
        if let Some(material) = self.global_material(texture) {
            return ResolvedCollisionMaterial {
                material,
                no_collision: false,
                source: MaterialClassSource::Global,
            };
        }
        ResolvedCollisionMaterial {
            material: fallback,
            no_collision: false,
            source: MaterialClassSource::Fallback,
        }
    }

    /// Sets or removes a global texture-name assignment.
    ///
    /// Returns `true` only when the persisted state changed.
    pub(crate) fn set_global(&mut self, texture: &str, material: Option<u8>) -> bool {
        let Some(texture) = normalize_texture_name(texture) else {
            return false;
        };
        let removed_no_collision = material
            .is_some()
            .then(|| self.global_no_collision.remove(&texture))
            .unwrap_or(false);
        set_map_value(&mut self.global, texture, material) || removed_no_collision
    }

    /// Removes either kind of global assignment for a texture name.
    pub(crate) fn clear_global(&mut self, texture: &str) -> bool {
        let Some(texture) = normalize_texture_name(texture) else {
            return false;
        };
        let removed_material = self.global.remove(&texture).is_some();
        self.global_no_collision.remove(&texture) || removed_material
    }

    /// Sets or removes an exact TXD + texture assignment.
    ///
    /// Returns `true` only when the persisted state changed.
    pub(crate) fn set_txd(&mut self, txd: &str, texture: &str, material: Option<u8>) -> bool {
        let Some(txd) = normalize_txd_name(txd) else {
            return false;
        };
        let Some(texture) = normalize_texture_name(texture) else {
            return false;
        };
        let key = (txd, texture);
        let removed_no_collision = material
            .is_some()
            .then(|| self.by_txd_no_collision.remove(&key))
            .unwrap_or(false);
        set_map_value(&mut self.by_txd, key, material) || removed_no_collision
    }

    /// Removes either kind of exact TXD assignment for a texture.
    pub(crate) fn clear_txd(&mut self, txd: &str, texture: &str) -> bool {
        let Some(txd) = normalize_txd_name(txd) else {
            return false;
        };
        let Some(texture) = normalize_texture_name(texture) else {
            return false;
        };
        let key = (txd, texture);
        let removed_material = self.by_txd.remove(&key).is_some();
        self.by_txd_no_collision.remove(&key) || removed_material
    }

    pub(crate) fn set_content(
        &mut self,
        fingerprint: TextureContentFingerprint,
        material: Option<u8>,
    ) -> bool {
        let removed_no_collision = material
            .is_some()
            .then(|| self.by_content_no_collision.remove(&fingerprint))
            .unwrap_or(false);
        set_map_value(&mut self.by_content, fingerprint, material) || removed_no_collision
    }

    /// Removes either kind of identical-content assignment for a fingerprint.
    pub(crate) fn clear_content(&mut self, fingerprint: TextureContentFingerprint) -> bool {
        let removed_material = self.by_content.remove(&fingerprint).is_some();
        self.by_content_no_collision.remove(&fingerprint) || removed_material
    }

    pub(crate) fn set_global_no_collision(&mut self, texture: &str, enabled: bool) -> bool {
        let Some(texture) = normalize_texture_name(texture) else {
            return false;
        };
        if enabled {
            let removed_material = self.global.remove(&texture).is_some();
            self.global_no_collision.insert(texture) || removed_material
        } else {
            self.global_no_collision.remove(&texture)
        }
    }

    pub(crate) fn set_txd_no_collision(&mut self, txd: &str, texture: &str, enabled: bool) -> bool {
        let Some(txd) = normalize_txd_name(txd) else {
            return false;
        };
        let Some(texture) = normalize_texture_name(texture) else {
            return false;
        };
        let key = (txd, texture);
        if enabled {
            let removed_material = self.by_txd.remove(&key).is_some();
            self.by_txd_no_collision.insert(key) || removed_material
        } else {
            self.by_txd_no_collision.remove(&key)
        }
    }

    pub(crate) fn set_content_no_collision(
        &mut self,
        fingerprint: TextureContentFingerprint,
        enabled: bool,
    ) -> bool {
        if enabled {
            let removed_material = self.by_content.remove(&fingerprint).is_some();
            self.by_content_no_collision.insert(fingerprint) || removed_material
        } else {
            self.by_content_no_collision.remove(&fingerprint)
        }
    }

    pub(crate) fn global_entries(&self) -> impl Iterator<Item = (&str, u8)> {
        self.global
            .iter()
            .map(|(texture, material)| (texture.as_str(), *material))
    }

    pub(crate) fn txd_entries(&self) -> impl Iterator<Item = (&str, &str, u8)> {
        self.by_txd
            .iter()
            .map(|((txd, texture), material)| (txd.as_str(), texture.as_str(), *material))
    }

    pub(crate) fn content_entries(
        &self,
    ) -> impl Iterator<Item = (TextureContentFingerprint, u8)> + '_ {
        self.by_content
            .iter()
            .map(|(fingerprint, material)| (*fingerprint, *material))
    }

    pub(crate) fn global_no_collision_entries(&self) -> impl Iterator<Item = &str> {
        self.global_no_collision.iter().map(String::as_str)
    }

    pub(crate) fn txd_no_collision_entries(&self) -> impl Iterator<Item = (&str, &str)> {
        self.by_txd_no_collision
            .iter()
            .map(|(txd, texture)| (txd.as_str(), texture.as_str()))
    }

    pub(crate) fn content_no_collision_entries(
        &self,
    ) -> impl Iterator<Item = TextureContentFingerprint> + '_ {
        self.by_content_no_collision.iter().copied()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct MaterialClassesLoad {
    pub(crate) classes: TextureMaterialClasses,
    /// Non-fatal parse and validation messages. Invalid entries are skipped.
    pub(crate) diagnostics: Vec<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg(test)]
pub(crate) struct MaterialClassContentMigration {
    pub(crate) original_assignments: usize,
    pub(crate) content_assignments: usize,
    pub(crate) exact_exceptions: usize,
    pub(crate) propagated_textures: usize,
    pub(crate) unmatched_assignments: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg(test)]
enum MigrationValue {
    Material(u8),
    NoCollision,
}

/// Converts name-scoped assignments that match indexed textures into
/// content-scoped assignments. Exact TXD exceptions preserve conflicts where
/// identical payloads were intentionally classified differently.
#[cfg(test)]
pub(crate) fn migrate_name_assignments_to_content(
    classes: &TextureMaterialClasses,
    txd_textures: &TxdTextureIndex,
) -> (TextureMaterialClasses, MaterialClassContentMigration) {
    let mut migrated = TextureMaterialClasses::default();
    let mut groups =
        BTreeMap::<TextureContentFingerprint, Vec<(String, String, Option<MigrationValue>)>>::new();
    let mut matched_global = BTreeSet::<String>::new();
    let mut matched_exact = BTreeSet::<(String, String)>::new();

    for (texture, entries) in txd_textures {
        for entry in entries {
            let exact_key = (
                entry.txd_name.to_ascii_lowercase(),
                texture.to_ascii_lowercase(),
            );
            let value = if classes.txd_is_no_collision(&entry.txd_name, texture) {
                matched_exact.insert(exact_key);
                Some(MigrationValue::NoCollision)
            } else if let Some(material) = classes.txd_material(&entry.txd_name, texture) {
                matched_exact.insert(exact_key);
                Some(MigrationValue::Material(material))
            } else if classes.global_is_no_collision(texture) {
                matched_global.insert(texture.to_ascii_lowercase());
                Some(MigrationValue::NoCollision)
            } else if let Some(material) = classes.global_material(texture) {
                matched_global.insert(texture.to_ascii_lowercase());
                Some(MigrationValue::Material(material))
            } else {
                None
            };
            groups.entry(entry.content_fingerprint).or_default().push((
                entry.txd_name.clone(),
                texture.clone(),
                value,
            ));
        }
    }

    let mut stats = MaterialClassContentMigration {
        original_assignments: classes.len(),
        ..Default::default()
    };
    for (fingerprint, members) in groups {
        let mut counts = BTreeMap::<MigrationValue, usize>::new();
        for (_, _, value) in &members {
            if let Some(value) = value {
                *counts.entry(*value).or_default() += 1;
            }
        }
        let Some(baseline) = counts
            .into_iter()
            .max_by(|(left_value, left_count), (right_value, right_count)| {
                left_count
                    .cmp(right_count)
                    .then_with(|| right_value.cmp(left_value))
            })
            .map(|(value, _)| value)
        else {
            continue;
        };
        match baseline {
            MigrationValue::Material(material) => {
                migrated.set_content(fingerprint, Some(material));
            }
            MigrationValue::NoCollision => {
                migrated.set_content_no_collision(fingerprint, true);
            }
        }
        stats.content_assignments += 1;
        for (txd, texture, value) in members {
            match value {
                Some(value) if value != baseline => {
                    match value {
                        MigrationValue::Material(material) => {
                            migrated.set_txd(&txd, &texture, Some(material));
                        }
                        MigrationValue::NoCollision => {
                            migrated.set_txd_no_collision(&txd, &texture, true);
                        }
                    }
                    stats.exact_exceptions += 1;
                }
                None => stats.propagated_textures += 1,
                _ => {}
            }
        }
    }

    for (texture, material) in classes.global_entries() {
        if !matched_global.contains(texture) {
            migrated.set_global(texture, Some(material));
            stats.unmatched_assignments += 1;
        }
    }
    for texture in classes.global_no_collision_entries() {
        if !matched_global.contains(texture) {
            migrated.set_global_no_collision(texture, true);
            stats.unmatched_assignments += 1;
        }
    }
    for (txd, texture, material) in classes.txd_entries() {
        if !matched_exact.contains(&(txd.to_string(), texture.to_string())) {
            migrated.set_txd(txd, texture, Some(material));
            stats.unmatched_assignments += 1;
        }
    }
    for (txd, texture) in classes.txd_no_collision_entries() {
        if !matched_exact.contains(&(txd.to_string(), texture.to_string())) {
            migrated.set_txd_no_collision(txd, texture, true);
            stats.unmatched_assignments += 1;
        }
    }
    for (fingerprint, material) in classes.content_entries() {
        migrated.set_content(fingerprint, Some(material));
    }
    for fingerprint in classes.content_no_collision_entries() {
        migrated.set_content_no_collision(fingerprint, true);
    }
    migrated.texture_categories = classes.texture_categories.clone();
    (migrated, stats)
}

pub(crate) fn material_classes_path(root: &Path) -> PathBuf {
    root.join(MATERIAL_CLASSES_FILE)
}

pub(crate) fn load_material_classes(root: &Path) -> MaterialClassesLoad {
    match read_eagle_scene_section(root, SECTION_MATERIAL_CLASSES) {
        Ok(Some(json)) => return parse_material_classes_value(&json),
        Ok(None) => {}
        Err(err) => {
            return MaterialClassesLoad {
                diagnostics: vec![format!(
                    "Could not load the EagleScene material-classes section: {err}. Texture material classes were ignored."
                )],
                ..Default::default()
            };
        }
    }

    let path = material_classes_path(root);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return MaterialClassesLoad::default();
        }
        Err(err) => {
            return MaterialClassesLoad {
                diagnostics: vec![format!(
                    "Could not read {}: {err}. Texture material classes were ignored.",
                    path.display()
                )],
                ..Default::default()
            };
        }
    };
    parse_material_classes(&text)
}

#[allow(dead_code)]
pub(crate) fn save_material_classes(
    root: &Path,
    classes: &TextureMaterialClasses,
) -> Result<(), String> {
    update_eagle_scene_section(
        root,
        SECTION_MATERIAL_CLASSES,
        material_classes_section_value(classes),
    )
}

pub(crate) fn material_classes_section_value(classes: &TextureMaterialClasses) -> Value {
    let mut entries = Vec::with_capacity(classes.len());
    for (texture, material) in classes.global_entries() {
        entries.push(material_entry(None, texture, material));
    }
    for (txd, texture, material) in classes.txd_entries() {
        entries.push(material_entry(Some(txd), texture, material));
    }
    for (fingerprint, material) in classes.content_entries() {
        entries.push(content_material_entry(fingerprint, material));
    }
    for texture in classes.global_no_collision_entries() {
        entries.push(no_collision_entry(None, texture));
    }
    for (txd, texture) in classes.txd_no_collision_entries() {
        entries.push(no_collision_entry(Some(txd), texture));
    }
    for fingerprint in classes.content_no_collision_entries() {
        entries.push(content_no_collision_entry(fingerprint));
    }
    let categories = classes
        .texture_categories
        .iter()
        .map(|((txd, texture), category)| {
            serde_json::json!({
                "txd": txd,
                "texture": texture,
                "category": category,
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "materials": entries,
        "textureCategories": categories,
        "version": MATERIAL_CLASSES_VERSION,
    })
}

/// Strictly reads the legacy standalone document for verified migration.
///
/// Missing files are not errors. Any condition the compatibility parser would
/// diagnose is an error here so migration cannot silently archive skipped or
/// ambiguous records.
pub(crate) fn load_legacy_material_classes_section(root: &Path) -> Result<Option<Value>, String> {
    let path = material_classes_path(root);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(format!("Could not read {}: {err}", path.display())),
    };
    let json = serde_json::from_str::<Value>(&text)
        .map_err(|err| format!("Invalid {}: {err}", path.display()))?;
    let loaded = parse_material_classes_value(&json);
    if !loaded.diagnostics.is_empty() {
        return Err(format!(
            "{} cannot be migrated safely: {}",
            path.display(),
            loaded.diagnostics.join(" ")
        ));
    }
    Ok(Some(json))
}

fn parse_material_classes(text: &str) -> MaterialClassesLoad {
    let json = match serde_json::from_str::<Value>(text) {
        Ok(json) => json,
        Err(err) => {
            return MaterialClassesLoad {
                diagnostics: vec![format!(
                    "Invalid {MATERIAL_CLASSES_FILE}: {err}. Texture material classes were ignored."
                )],
                ..Default::default()
            };
        }
    };
    parse_material_classes_value(&json)
}

fn parse_material_classes_value(json: &Value) -> MaterialClassesLoad {
    let Some(object) = json.as_object() else {
        return MaterialClassesLoad {
            diagnostics: vec![format!(
                "Invalid {MATERIAL_CLASSES_FILE}: the root must be a JSON object."
            )],
            ..Default::default()
        };
    };

    let mut result = MaterialClassesLoad::default();
    match object.get("version").and_then(Value::as_u64) {
        Some(1 | 2 | MATERIAL_CLASSES_VERSION) => {}
        Some(version) => result.diagnostics.push(format!(
            "{MATERIAL_CLASSES_FILE} uses version {version}; reading compatible version-{MATERIAL_CLASSES_VERSION} fields."
        )),
        None => result.diagnostics.push(format!(
            "{MATERIAL_CLASSES_FILE} has no valid numeric version; reading compatible fields."
        )),
    }
    let Some(entries) = object.get("materials").and_then(Value::as_array) else {
        result.diagnostics.push(format!(
            "Invalid {MATERIAL_CLASSES_FILE}: `materials` must be an array."
        ));
        return result;
    };

    for (index, entry) in entries.iter().enumerate() {
        parse_material_entry(index, entry, &mut result);
    }
    if let Some(categories) = object.get("textureCategories").and_then(Value::as_array) {
        for (index, entry) in categories.iter().enumerate() {
            let Some(entry) = entry.as_object() else {
                result.diagnostics.push(format!(
                    "{MATERIAL_CLASSES_FILE} textureCategories[{index}] is not an object and was skipped."
                ));
                continue;
            };
            let Some(txd) = entry.get("txd").and_then(Value::as_str) else {
                result.diagnostics.push(format!(
                    "{MATERIAL_CLASSES_FILE} textureCategories[{index}] has no valid TXD and was skipped."
                ));
                continue;
            };
            let Some(texture) = entry.get("texture").and_then(Value::as_str) else {
                result.diagnostics.push(format!(
                    "{MATERIAL_CLASSES_FILE} textureCategories[{index}] has no valid texture and was skipped."
                ));
                continue;
            };
            let Some(category) = entry.get("category").and_then(Value::as_str) else {
                result.diagnostics.push(format!(
                    "{MATERIAL_CLASSES_FILE} textureCategories[{index}] has no valid category and was skipped."
                ));
                continue;
            };
            result
                .classes
                .set_texture_category(txd, texture, Some(category));
        }
    }
    result
}

fn parse_material_entry(index: usize, entry: &Value, result: &mut MaterialClassesLoad) {
    let Some(entry) = entry.as_object() else {
        result.diagnostics.push(format!(
            "{MATERIAL_CLASSES_FILE} materials[{index}] is not an object and was skipped."
        ));
        return;
    };
    let no_collision = entry
        .get("noCollision")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let material = entry
        .get("material")
        .and_then(Value::as_u64)
        .and_then(|value| u8::try_from(value).ok());
    if let Some(fingerprint_value) = entry.get("contentFingerprint") {
        let Some(fingerprint) = fingerprint_value
            .as_str()
            .and_then(parse_content_fingerprint)
        else {
            result.diagnostics.push(format!(
                "{MATERIAL_CLASSES_FILE} materials[{index}] has an invalid content fingerprint and was skipped."
            ));
            return;
        };
        if !no_collision && material.is_none() {
            result.diagnostics.push(format!(
                "{MATERIAL_CLASSES_FILE} materials[{index}] has neither `noCollision: true` nor a material ID in 0..=255 and was skipped."
            ));
            return;
        }
        let replaced = if no_collision {
            let removed_material = result.classes.by_content.remove(&fingerprint).is_some();
            let duplicate = !result.classes.by_content_no_collision.insert(fingerprint);
            removed_material || duplicate
        } else {
            result.classes.by_content_no_collision.remove(&fingerprint);
            result
                .classes
                .by_content
                .insert(fingerprint, material.unwrap())
                .is_some()
        };
        if replaced {
            result.diagnostics.push(format!(
                "{MATERIAL_CLASSES_FILE} materials[{index}] replaces an earlier duplicate content assignment."
            ));
        }
        return;
    }
    let Some(texture) = entry
        .get("texture")
        .and_then(Value::as_str)
        .and_then(normalize_texture_name)
    else {
        result.diagnostics.push(format!(
            "{MATERIAL_CLASSES_FILE} materials[{index}] has no valid texture name and was skipped."
        ));
        return;
    };
    if !no_collision && material.is_none() {
        result.diagnostics.push(format!(
            "{MATERIAL_CLASSES_FILE} materials[{index}] has neither `noCollision: true` nor a material ID in 0..=255 and was skipped."
        ));
        return;
    }

    if let Some(txd_value) = entry.get("txd") {
        let Some(txd) = txd_value.as_str().and_then(normalize_txd_name) else {
            result.diagnostics.push(format!(
                "{MATERIAL_CLASSES_FILE} materials[{index}] has an invalid TXD name and was skipped."
            ));
            return;
        };
        let key = (txd, texture);
        let replaced = if no_collision {
            let removed_material = result.classes.by_txd.remove(&key).is_some();
            let duplicate = !result.classes.by_txd_no_collision.insert(key);
            removed_material || duplicate
        } else {
            result.classes.by_txd_no_collision.remove(&key);
            result
                .classes
                .by_txd
                .insert(key, material.unwrap())
                .is_some()
        };
        if replaced {
            result.diagnostics.push(format!(
                "{MATERIAL_CLASSES_FILE} materials[{index}] replaces an earlier duplicate TXD texture assignment."
            ));
        }
    } else {
        let replaced = if no_collision {
            let removed_material = result.classes.global.remove(&texture).is_some();
            let duplicate = !result.classes.global_no_collision.insert(texture);
            removed_material || duplicate
        } else {
            result.classes.global_no_collision.remove(&texture);
            result
                .classes
                .global
                .insert(texture, material.unwrap())
                .is_some()
        };
        if replaced {
            result.diagnostics.push(format!(
                "{MATERIAL_CLASSES_FILE} materials[{index}] replaces an earlier duplicate global texture assignment."
            ));
        }
    }
}

fn material_entry(txd: Option<&str>, texture: &str, material: u8) -> Value {
    let mut entry = Map::new();
    if let Some(txd) = txd {
        entry.insert("txd".to_string(), Value::String(txd.to_string()));
    }
    entry.insert("texture".to_string(), Value::String(texture.to_string()));
    entry.insert(
        "material".to_string(),
        Value::Number(serde_json::Number::from(material)),
    );
    Value::Object(entry)
}

fn no_collision_entry(txd: Option<&str>, texture: &str) -> Value {
    let mut entry = Map::new();
    if let Some(txd) = txd {
        entry.insert("txd".to_string(), Value::String(txd.to_string()));
    }
    entry.insert("texture".to_string(), Value::String(texture.to_string()));
    entry.insert("noCollision".to_string(), Value::Bool(true));
    Value::Object(entry)
}

fn content_material_entry(fingerprint: TextureContentFingerprint, material: u8) -> Value {
    serde_json::json!({
        "contentFingerprint": format_content_fingerprint(fingerprint),
        "material": material,
    })
}

fn content_no_collision_entry(fingerprint: TextureContentFingerprint) -> Value {
    serde_json::json!({
        "contentFingerprint": format_content_fingerprint(fingerprint),
        "noCollision": true,
    })
}

pub(crate) fn format_content_fingerprint(fingerprint: TextureContentFingerprint) -> String {
    format!("{:016x}{:016x}", fingerprint[0], fingerprint[1])
}

fn parse_content_fingerprint(value: &str) -> Option<TextureContentFingerprint> {
    let value = value.trim();
    if value.len() != 32 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    Some([
        u64::from_str_radix(&value[..16], 16).ok()?,
        u64::from_str_radix(&value[16..], 16).ok()?,
    ])
}

fn normalize_texture_name(value: &str) -> Option<String> {
    normalize_name(value)
}

fn normalize_txd_name(value: &str) -> Option<String> {
    let mut value = normalize_name(value)?;
    if !value.ends_with(".txd") {
        value.push_str(".txd");
    }
    Some(value)
}

fn normalize_name(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_ascii_lowercase())
}

fn set_map_value<K: Ord>(map: &mut BTreeMap<K, u8>, key: K, value: Option<u8>) -> bool {
    match value {
        Some(value) if map.get(&key) == Some(&value) => false,
        Some(value) => {
            map.insert(key, value);
            true
        }
        None => map.remove(&key).is_some(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::txd::{TxFormat, TxdTexture};

    fn temp_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "eagle_material_classes_{label}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn indexed_texture(txd_name: &str, fingerprint: TextureContentFingerprint) -> TxdTexture {
        TxdTexture {
            txd_name: txd_name.to_string(),
            img_path: PathBuf::new(),
            native_offset: 0,
            native_size: 0,
            data_offset: 0,
            data_size: 0,
            palette_offset: 0,
            width: 64,
            height: 64,
            format: TxFormat::Dxt1,
            has_alpha: false,
            content_fingerprint: fingerprint,
        }
    }

    #[test]
    fn exact_txd_assignment_precedes_global_then_fallback() {
        let mut classes = TextureMaterialClasses::default();
        assert!(classes.set_global(" Grass_Main ", Some(9)));
        assert!(classes.set_txd("City", "GRASS_MAIN", Some(4)));

        assert_eq!(
            classes.resolve_with_content(Some("CITY.TXD"), "grass_main", None, 1),
            ResolvedCollisionMaterial {
                material: 4,
                no_collision: false,
                source: MaterialClassSource::TxdOverride,
            }
        );
        assert_eq!(
            classes.resolve_with_content(Some("country"), "GRASS_MAIN", None, 1),
            ResolvedCollisionMaterial {
                material: 9,
                no_collision: false,
                source: MaterialClassSource::Global,
            }
        );
        assert_eq!(
            classes.resolve_with_content(Some("city"), "unassigned", None, 1),
            ResolvedCollisionMaterial {
                material: 1,
                no_collision: false,
                source: MaterialClassSource::Fallback,
            }
        );
    }

    #[test]
    fn setters_normalize_remove_and_report_real_changes() {
        let mut classes = TextureMaterialClasses::default();
        assert!(!classes.set_global(" ", Some(9)));
        assert!(classes.set_global("Stone", Some(1)));
        assert!(!classes.set_global(" stone ", Some(1)));
        assert_eq!(classes.global_material("STONE"), Some(1));
        assert!(classes.set_global("STONE", None));
        assert!(!classes.set_global("stone", None));

        assert!(classes.set_global_no_collision("Glass", true));
        assert!(classes.clear_global("glass"));
        assert!(!classes.global_is_no_collision("glass"));
        assert!(!classes.clear_global("glass"));

        assert!(classes.set_txd("Props", "Metal", Some(5)));
        assert_eq!(classes.txd_material("props.txd", "metal"), Some(5));
        assert!(classes.clear_txd("PROPS.TXD", "METAL"));
        assert!(!classes.clear_txd("props", "metal"));

        let fingerprint = [0x1234, 0x5678];
        assert!(classes.set_content_no_collision(fingerprint, true));
        assert!(classes.clear_content(fingerprint));
        assert!(!classes.content_is_no_collision(fingerprint));
        assert!(!classes.clear_content(fingerprint));
        assert_eq!(classes.len(), 0);
    }

    #[test]
    fn no_collision_obeys_scope_precedence_and_replaces_material() {
        let mut classes = TextureMaterialClasses::default();
        assert!(classes.set_global("glass", Some(4)));
        assert!(classes.set_txd_no_collision("city", "glass", true));

        let exact = classes.resolve_with_content(Some("city.txd"), "GLASS", None, 1);
        assert!(exact.no_collision);
        assert_eq!(exact.source, MaterialClassSource::TxdOverride);

        let global = classes.resolve_with_content(Some("country"), "glass", None, 1);
        assert!(!global.no_collision);
        assert_eq!(global.material, 4);

        assert!(classes.set_txd("city", "glass", Some(9)));
        assert!(!classes.txd_is_no_collision("city", "glass"));
        assert_eq!(classes.txd_material("city", "glass"), Some(9));
    }

    #[test]
    fn content_assignment_sits_between_exact_and_global_precedence() {
        let fingerprint = [0x1234, 0x5678];
        let mut classes = TextureMaterialClasses::default();
        classes.set_global("road_a", Some(1));
        classes.set_content(fingerprint, Some(4));
        classes.set_txd("city", "road_a", Some(7));

        assert_eq!(
            classes
                .resolve_with_content(Some("city"), "road_a", Some(fingerprint), 0)
                .material,
            7
        );
        let content = classes.resolve_with_content(Some("country"), "road_b", Some(fingerprint), 0);
        assert_eq!(content.material, 4);
        assert_eq!(content.source, MaterialClassSource::ContentFingerprint);
        assert_eq!(
            classes
                .resolve_with_content(Some("country"), "road_a", None, 0)
                .material,
            1
        );
    }

    #[test]
    fn name_migration_targets_identical_textures_and_preserves_conflicts() {
        let shared = [0xaaaa, 0xbbbb];
        let mut index = TxdTextureIndex::new();
        index.insert(
            "grass_a".to_string(),
            vec![indexed_texture("nice.txd", shared)],
        );
        index.insert(
            "grass_b".to_string(),
            vec![indexed_texture("other.txd", shared)],
        );
        index.insert(
            "grass_unassigned".to_string(),
            vec![indexed_texture("third.txd", shared)],
        );

        let mut classes = TextureMaterialClasses::default();
        classes.set_global("grass_a", Some(9));
        classes.set_txd("other", "grass_b", Some(4));
        let (migrated, stats) = migrate_name_assignments_to_content(&classes, &index);

        assert_eq!(stats.content_assignments, 1);
        assert_eq!(stats.exact_exceptions, 1);
        assert_eq!(stats.propagated_textures, 1);
        assert_eq!(
            migrated
                .resolve_with_content(Some("nice"), "grass_a", Some(shared), 0)
                .material,
            9
        );
        assert_eq!(
            migrated
                .resolve_with_content(Some("other"), "grass_b", Some(shared), 0)
                .material,
            4
        );
        assert_eq!(
            migrated
                .resolve_with_content(Some("third"), "grass_unassigned", Some(shared), 0)
                .material,
            4
        );
    }

    #[test]
    fn load_missing_and_invalid_files_is_safe_and_diagnostic() {
        let root = temp_root("safe_load");
        fs::create_dir_all(&root).unwrap();
        let missing = load_material_classes(&root);
        assert_eq!(missing.classes.len(), 0);
        assert!(missing.diagnostics.is_empty());

        fs::write(material_classes_path(&root), "{nope").unwrap();
        let invalid = load_material_classes(&root);
        assert_eq!(invalid.classes.len(), 0);
        assert_eq!(invalid.diagnostics.len(), 1);
        assert!(invalid.diagnostics[0].contains("Invalid"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_entries_are_skipped_without_losing_valid_entries() {
        let loaded = parse_material_classes(
            r#"{
                "version": 1,
                "materials": [
                    {"texture": "Grass", "material": 9},
                    {"texture": "", "material": 4},
                    {"txd": "City", "texture": "Stone", "material": 300},
                    {"txd": "City", "texture": "Grass", "material": 4},
                    {"txd": "CITY.TXD", "texture": "GRASS", "material": 5}
                ]
            }"#,
        );
        assert_eq!(loaded.classes.global_material("grass"), Some(9));
        assert_eq!(loaded.classes.txd_material("city", "grass"), Some(5));
        assert_eq!(loaded.classes.len(), 2);
        assert_eq!(loaded.diagnostics.len(), 3);
    }

    #[test]
    fn serialization_is_deterministic_and_round_trips() {
        let root = temp_root("round_trip");
        fs::create_dir_all(&root).unwrap();
        let mut classes = TextureMaterialClasses::default();
        classes.set_txd("Zed", "Road", Some(1));
        classes.set_global("Stone", Some(4));
        classes.set_global("grass", Some(9));
        classes.set_txd("Alpha.txd", "Road", Some(7));
        classes.set_global_no_collision("glass", true);
        classes.set_txd_no_collision("Alpha.txd", "invisible", true);
        classes.set_content([0x1234, 0x5678], Some(6));
        classes.set_content_no_collision([0xabcd, 0xef01], true);

        save_material_classes(&root, &classes).unwrap();
        assert!(!material_classes_path(&root).exists());
        let first_value = read_eagle_scene_section(&root, SECTION_MATERIAL_CLASSES)
            .unwrap()
            .unwrap();
        let first = serde_json::to_string_pretty(&first_value).unwrap();
        save_material_classes(&root, &classes).unwrap();
        let second_value = read_eagle_scene_section(&root, SECTION_MATERIAL_CLASSES)
            .unwrap()
            .unwrap();
        let second = serde_json::to_string_pretty(&second_value).unwrap();
        assert_eq!(first, second);
        assert!(first.find("\"grass\"").unwrap() < first.find("\"stone\"").unwrap());
        assert!(first.find("\"alpha.txd\"").unwrap() < first.find("\"zed.txd\"").unwrap());

        let loaded = load_material_classes(&root);
        assert!(loaded.diagnostics.is_empty());
        assert_eq!(loaded.classes, classes);
        assert!(first.contains("\"noCollision\": true"));
        assert!(first.contains("\"contentFingerprint\": \"00000000000012340000000000005678\""));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn eagle_scene_material_classes_take_priority_over_legacy() {
        let root = temp_root("unified_priority");
        fs::create_dir_all(&root).unwrap();
        fs::write(
            material_classes_path(&root),
            r#"{"version":2,"materials":[{"texture":"road","material":1}]}"#,
        )
        .unwrap();
        update_eagle_scene_section(
            &root,
            SECTION_MATERIAL_CLASSES,
            serde_json::json!({
                "version": 2,
                "materials": [{"texture": "road", "material": 7}]
            }),
        )
        .unwrap();

        let loaded = load_material_classes(&root);
        assert!(loaded.diagnostics.is_empty());
        assert_eq!(loaded.classes.global_material("road"), Some(7));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn legacy_material_classes_are_a_missing_section_fallback() {
        let root = temp_root("legacy_fallback");
        fs::create_dir_all(&root).unwrap();
        fs::write(
            material_classes_path(&root),
            r#"{"version":2,"materials":[{"texture":"road","material":3}]}"#,
        )
        .unwrap();

        let loaded = load_material_classes(&root);
        assert!(loaded.diagnostics.is_empty());
        assert_eq!(loaded.classes.global_material("road"), Some(3));
        assert!(
            load_legacy_material_classes_section(&root)
                .unwrap()
                .is_some()
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn strict_legacy_material_import_rejects_skipped_records() {
        let root = temp_root("strict_legacy");
        fs::create_dir_all(&root).unwrap();
        fs::write(
            material_classes_path(&root),
            r#"{"version":2,"materials":[{"texture":"","material":1}]}"#,
        )
        .unwrap();

        assert!(load_legacy_material_classes_section(&root).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn texture_categories_round_trip_with_exact_txd_scope() {
        let mut classes = TextureMaterialClasses::default();
        assert!(classes.set_texture_category("CITY.TXD", "Grass_Main", Some("Nature")));
        assert!(classes.set_texture_category("country", "grass_main", Some("Ground")));
        assert_eq!(
            classes.texture_category("city", "GRASS_MAIN"),
            Some("Nature")
        );
        assert_eq!(
            classes.texture_category("country.txd", "grass_main"),
            Some("Ground")
        );

        let value = material_classes_section_value(&classes);
        let mut loaded = parse_material_classes_value(&value);
        assert!(loaded.diagnostics.is_empty());
        assert_eq!(loaded.classes, classes);
        assert!(
            loaded
                .classes
                .set_texture_category("city", "grass_main", None)
        );
        assert_eq!(loaded.classes.texture_category("city", "grass_main"), None);
    }

    /// Developer migration utility. Run with:
    /// `EAGLE_MATERIAL_MIGRATION_ROOT=/resource/path cargo test
    /// migrate_resource_material_classes_to_content -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn migrate_resource_material_classes_to_content() {
        let root = PathBuf::from(
            std::env::var("EAGLE_MATERIAL_MIGRATION_ROOT")
                .expect("EAGLE_MATERIAL_MIGRATION_ROOT must name a resource"),
        );
        let loaded = load_material_classes(&root);
        assert!(
            loaded.diagnostics.is_empty(),
            "material file has diagnostics: {:?}",
            loaded.diagnostics
        );
        let mut index = TxdTextureIndex::new();
        for path in crate::collect_scene_img_files(&root, crate::LoadSceneSource::Saved) {
            crate::index_txd_file(&path, &mut index);
        }
        for path in crate::collect_scene_txd_files(&root, crate::LoadSceneSource::Saved) {
            crate::index_standalone_txd_file(&path, &mut index);
        }
        assert!(!index.is_empty(), "resource contains no indexed textures");

        let (migrated, stats) = migrate_name_assignments_to_content(&loaded.classes, &index);
        for (texture, entries) in &index {
            for entry in entries {
                let old =
                    loaded
                        .classes
                        .resolve_with_content(Some(&entry.txd_name), texture, None, 0);
                if old.source == MaterialClassSource::Fallback {
                    continue;
                }
                let new = migrated.resolve_with_content(
                    Some(&entry.txd_name),
                    texture,
                    Some(entry.content_fingerprint),
                    0,
                );
                assert_eq!(
                    (new.material, new.no_collision),
                    (old.material, old.no_collision),
                    "migration changed {}/{}",
                    entry.txd_name,
                    texture
                );
            }
        }

        let source = material_classes_path(&root);
        let backup = root.join("eagleMaterialClasses.name-scope-backup.json");
        assert!(
            !backup.exists(),
            "refusing to overwrite existing backup {}",
            backup.display()
        );
        fs::copy(&source, &backup).expect("could not create migration backup");
        save_material_classes(&root, &migrated).expect("could not save migrated assignments");
        let reloaded = load_material_classes(&root);
        assert!(reloaded.diagnostics.is_empty());
        assert_eq!(reloaded.classes, migrated);
        println!(
            "migrated {}: {} original, {} content, {} exact exceptions, {} propagated textures, {} unmatched; backup {}",
            root.display(),
            stats.original_assignments,
            stats.content_assignments,
            stats.exact_exceptions,
            stats.propagated_textures,
            stats.unmatched_assignments,
            backup.display()
        );
    }
}
