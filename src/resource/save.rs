#![cfg_attr(windows, allow(unreachable_code))]

use super::super::*;
use crate::resource::eagle_scene::{
    EAGLE_SCENE_FILE, EagleSceneSectionUpdate, LegacySceneImport, PreparedEagleSceneMigration,
    SECTION_LIGHTS, SECTION_MATERIAL_CLASSES, SECTION_MATERIAL_EMITTERS, SECTION_SAFE_COLLISIONS,
    SECTION_SHADOW_CASTERS, eagle_scene_path, finalize_eagle_scene_migration,
    prepare_eagle_scene_migration, read_eagle_scene, read_eagle_scene_section,
    update_eagle_scene_sections,
};

pub(crate) fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub(crate) fn write_tag(tag: &str, attrs: &BTreeMap<String, String>) -> String {
    let mut out = format!("    <{tag}");
    for (key, value) in attrs {
        out.push(' ');
        out.push_str(key);
        out.push_str("=\"");
        out.push_str(&xml_escape(value));
        out.push('"');
    }
    if tag == "override" {
        out.push_str(" />\n");
        return out;
    }
    out.push_str(&format!("></{tag}>\n"));
    out
}

#[derive(Default)]
struct AssetCaseIndex {
    dff: HashMap<String, String>,
    col: HashMap<String, String>,
    txd: HashMap<String, String>,
}

impl AssetCaseIndex {
    fn build(root: &Path) -> Self {
        let mut index = Self::default();
        let mut img_files = collect_resource_img_files(root);
        img_files.sort();
        for path in img_files {
            for entry in parse_img(&path) {
                index.add_file_name(&entry.name);
            }
        }
        index.add_loose_assets(root);
        index.add_loose_assets(&wip_root_path(root));
        index
    }

    fn add_loose_assets(&mut self, root: &Path) {
        for dir in ["zones", "textures", "models"] {
            let path = root.join(dir);
            if !path.exists() {
                continue;
            }
            for entry in WalkDir::new(path).into_iter().filter_map(Result::ok) {
                if entry.file_type().is_file() {
                    self.add_file_name(entry.file_name().to_string_lossy().as_ref());
                }
            }
        }
    }

    fn add_file_name(&mut self, name: &str) {
        let Some((stem, ext)) = split_asset_name(name) else {
            return;
        };
        let key = lower(&format!("{stem}.{ext}"));
        match ext.to_ascii_lowercase().as_str() {
            "dff" => {
                self.dff.entry(key).or_insert(stem);
            }
            "col" => {
                self.col.entry(key).or_insert(stem);
            }
            "txd" => {
                self.txd.entry(key).or_insert(stem);
            }
            _ => {}
        }
    }

    fn canonical_stem(&self, name: &str, ext: &str) -> Option<&str> {
        let key = asset_key(name, ext);
        match ext {
            ".dff" => self.dff.get(&key),
            ".col" => self.col.get(&key),
            ".txd" => self.txd.get(&key),
            _ => None,
        }
        .map(String::as_str)
    }
}

fn split_asset_name(name: &str) -> Option<(String, String)> {
    let trimmed = name.trim();
    let dot = trimmed.rfind('.')?;
    let stem = trimmed[..dot].trim();
    let ext = trimmed[dot + 1..].trim();
    if stem.is_empty() || ext.is_empty() {
        return None;
    }
    Some((stem.to_string(), ext.to_string()))
}

pub(crate) fn strip_legacy_light_mapper_stem(stem: &str) -> Option<String> {
    let stripped = stem.strip_prefix("lm_")?;
    let base = stripped
        .strip_suffix("_r")
        .or_else(|| stripped.strip_suffix("_u"))?;
    (!base.trim().is_empty()).then(|| base.to_string())
}

pub(crate) fn normalize_legacy_light_mapper_asset_name(name: &str) -> String {
    let Some((stem, ext)) = split_asset_name(name) else {
        return strip_legacy_light_mapper_stem(name).unwrap_or_else(|| name.to_string());
    };
    let stem = strip_legacy_light_mapper_stem(&stem).unwrap_or(stem);
    format!("{stem}.{ext}")
}

pub(crate) fn normalize_legacy_light_mapper_asset_stem(name: &str) -> String {
    let Some((stem, _)) = split_asset_name(name) else {
        return strip_legacy_light_mapper_stem(name).unwrap_or_else(|| name.to_string());
    };
    strip_legacy_light_mapper_stem(&stem).unwrap_or(stem)
}

pub(crate) fn dff_override_stem(def_id: &str, dff: Option<&str>) -> Option<String> {
    let current = dff.map(str::trim).filter(|value| !value.is_empty())?;
    let normalized = normalize_legacy_light_mapper_asset_stem(current);
    (asset_key(&normalized, ".dff") != asset_key(def_id, ".dff")).then_some(normalized)
}

fn canonicalize_asset_attr(
    attrs: &mut BTreeMap<String, String>,
    key: &str,
    fallback: &str,
    ext: &str,
    index: &AssetCaseIndex,
) {
    let current = attrs
        .get(key)
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback);
    let normalized = normalize_legacy_light_mapper_asset_stem(current);
    let current = normalized.as_str();
    let Some(canonical) = index.canonical_stem(current, ext) else {
        if attrs.get(key).is_some_and(|value| value.trim() != current) {
            attrs.insert(key.to_string(), current.to_string());
        }
        return;
    };
    if !attrs.contains_key(key) || canonical != current {
        attrs.insert(key.to_string(), canonical.to_string());
    }
}

fn canonicalize_txd_attr(attrs: &mut BTreeMap<String, String>, index: &AssetCaseIndex) {
    let Some(txd) = attrs.get("txd").cloned() else {
        return;
    };
    let canonical: Vec<_> = txd
        .split(',')
        .map(|name| {
            let trimmed = name.trim();
            index
                .canonical_stem(trimmed, ".txd")
                .unwrap_or(trimmed)
                .to_string()
        })
        .collect();
    attrs.insert("txd".to_string(), canonical.join(","));
}

fn canonicalize_definition_asset_attrs(
    def: &Definition,
    attrs: &mut BTreeMap<String, String>,
    index: &AssetCaseIndex,
) {
    if let Some(dff) = dff_override_stem(&def.id, attrs.get("dff").map(String::as_str)) {
        let canonical = index.canonical_stem(&dff, ".dff").unwrap_or(&dff);
        attrs.insert("dff".to_string(), canonical.to_string());
    } else {
        attrs.remove("dff");
    }
    canonicalize_asset_attr(attrs, "col", &def.id, ".col", index);
    canonicalize_txd_attr(attrs, index);
}

fn write_scene_files_data(
    zones_source: &[String],
    eagle_zone_offsets: EagleZoneOffsets,
    placements: &[Placement],
    definitions: &HashMap<String, Definition>,
    readonly_definition_ids: &HashSet<String>,
    element_states: &[ElementState],
    root: &Path,
) -> Result<(), Vec<String>> {
    let mut zones = zones_source.to_vec();
    let mut known_zones: HashSet<String> = zones.iter().cloned().collect();
    for placement in placements {
        if known_zones.insert(placement.zone.clone()) {
            zones.push(placement.zone.clone());
        }
    }
    for def in definitions.values() {
        if readonly_definition_ids.contains(&def.id) {
            continue;
        }
        if known_zones.insert(def.zone.clone()) {
            zones.push(def.zone.clone());
        }
    }
    let invalid_zones = zones
        .iter()
        .filter(|zone| !is_safe_zone_name(zone))
        .cloned()
        .collect::<Vec<_>>();
    if !invalid_zones.is_empty() {
        return Err(invalid_zones
            .into_iter()
            .map(|zone| {
                format!(
                    "Refusing to save unsafe zone name {:?}; zone names must be one plain path component",
                    zone
                )
            })
            .collect());
    }
    let asset_case_index = AssetCaseIndex::build(root);

    // Build these indexes once. Monaco-sized resources can contain many zones;
    // scanning every placement and definition again for each zone made saves
    // quadratic in practice.
    let mut placements_by_zone: HashMap<&str, Vec<(usize, &Placement)>> = HashMap::new();
    for (idx, placement) in placements.iter().enumerate() {
        placements_by_zone
            .entry(placement.zone.as_str())
            .or_default()
            .push((idx, placement));
    }
    let mut definitions_by_zone: HashMap<&str, Vec<&Definition>> = HashMap::new();
    for def in definitions.values() {
        if !readonly_definition_ids.contains(&def.id) {
            definitions_by_zone
                .entry(def.zone.as_str())
                .or_default()
                .push(def);
        }
    }

    let mut errors = Vec::new();
    for zone in &zones {
        let zone_dir = root.join("zones").join(zone);
        if let Err(err) = fs::create_dir_all(&zone_dir) {
            errors.push(format!("{}: {err}", zone_dir.display()));
            continue;
        }

        let mut map = String::from("<map>\n");
        map.push_str("    <info name=\"MTA:SA Eagle Edit\" author=\"MTA:SA Eagle Edit\" version=\"1.0\"></info>\n");
        for &(idx, placement) in placements_by_zone
            .get(zone.as_str())
            .map(Vec::as_slice)
            .unwrap_or_default()
        {
            if element_states.get(idx).is_some_and(|state| state.deleted) {
                continue;
            }
            let mut attrs = placement.attrs.clone();
            attrs.insert("id".to_string(), placement.id.clone());
            map.push_str(&write_tag(&placement.tag, &attrs));
        }
        map.push_str("</map>\n");
        let map_path = zone_dir.join(format!("{zone}.map"));
        if let Err(err) = fs::write(&map_path, map) {
            errors.push(format!("{}: {err}", map_path.display()));
        }

        let mut defs = String::from("<zoneDefinitions>\n");
        let mut zone_defs = definitions_by_zone
            .get(zone.as_str())
            .cloned()
            .unwrap_or_default();
        zone_defs.sort_by(|a, b| a.id.cmp(&b.id));
        for def in zone_defs {
            let mut attrs = def.attrs.clone();
            attrs.insert("id".to_string(), def.id.clone());
            if is_override_definition(def) {
                let explicit_keys: Vec<String> = attrs
                    .get("__overrideAttrs")
                    .map(|value| {
                        value
                            .split(',')
                            .map(str::trim)
                            .filter(|key| !key.is_empty())
                            .map(ToOwned::to_owned)
                            .collect()
                    })
                    .unwrap_or_default();
                let mut override_attrs = BTreeMap::new();
                override_attrs.insert("id".to_string(), def.id.clone());
                for key in explicit_keys {
                    if let Some(value) = attrs.get(&key).cloned() {
                        override_attrs.insert(key, value);
                    }
                }
                defs.push_str(&write_tag("override", &override_attrs));
            } else {
                canonicalize_definition_asset_attrs(def, &mut attrs, &asset_case_index);
                attrs.remove("__override");
                attrs.remove("__overrideAttrs");
                defs.push_str(&write_tag("definition", &attrs));
            }
        }
        defs.push_str("</zoneDefinitions>\n");
        let def_path = zone_dir.join(format!("{zone}.definition"));
        if let Err(err) = fs::write(&def_path, defs) {
            errors.push(format!("{}: {err}", def_path.display()));
        }
    }
    let mut zones_text = String::new();
    if let Some(offset) = eagle_zone_offsets.offset {
        zones_text.push_str(&format!("#offset {},{},{}\n", offset.x, offset.y, offset.z));
    }
    if let Some(offset) = eagle_zone_offsets.water_offset {
        zones_text.push_str(&format!(
            "#waterOffset {},{},{}\n",
            offset.x, offset.y, offset.z
        ));
    }
    if !zones.is_empty() {
        zones_text.push_str(&zones.join("\n"));
        zones_text.push('\n');
    }
    let zones_path = root.join("eagleZones.txt");
    if let Err(err) = fs::write(&zones_path, zones_text) {
        errors.push(format!("{}: {err}", zones_path.display()));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub(crate) fn write_scene_files(app: &AppState, root: &Path) -> Result<(), Vec<String>> {
    write_scene_files_data(
        &app.zones,
        app.eagle_zone_offsets,
        &app.placements,
        &app.definitions,
        &app.readonly_definition_ids,
        &app.element_states,
        root,
    )
}

pub(crate) fn replacement_asset_root(app: &AppState) -> PathBuf {
    if app.loaded_wip {
        wip_root_path(&app.root)
    } else {
        app.root.clone()
    }
}

pub(crate) fn copy_replacement_archive(src_root: &Path, dst_root: &Path) -> Result<usize, String> {
    let src = src_root.join("imgs").join(REPLACEMENT_IMG);
    if !src.is_file() {
        return Ok(0);
    }
    let dst_dir = dst_root.join("imgs");
    fs::create_dir_all(&dst_dir).map_err(|err| format!("{}: {err}", dst_dir.display()))?;
    let dst = dst_dir.join(REPLACEMENT_IMG);
    if src == dst {
        return Ok(1);
    }
    fs::copy(&src, &dst)
        .map(|_| 1)
        .map_err(|err| format!("{} -> {}: {err}", src.display(), dst.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_resource_root(name: &str) -> PathBuf {
        let id = TEMP_COUNTER.fetch_add(1, AtomicOrdering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("eagle_editor_{name}_{}_{}", std::process::id(), id));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("zones").join("test")).unwrap();
        fs::write(root.join("eagleZones.txt"), "test\n").unwrap();
        fs::write(
            root.join("zones").join("test").join("test.map"),
            "<map>\n</map>\n",
        )
        .unwrap();
        root
    }

    fn empty_manual_save_snapshot(root: PathBuf) -> ManualSaveWriteSnapshot {
        ManualSaveWriteSnapshot {
            mode: ManualSaveMode::Resource,
            source_root: root,
            zones: vec!["test".to_string()],
            eagle_zone_offsets: EagleZoneOffsets::default(),
            placements: Vec::new(),
            definitions: HashMap::new(),
            readonly_definition_ids: HashSet::new(),
            element_states: Vec::new(),
            lights: Vec::new(),
            material_emitters: HashMap::new(),
            material_classes: TextureMaterialClasses::default(),
            safe_collisions: SafeCollisions::default(),
            shadow_casting: HashMap::new(),
            water_planes: Vec::new(),
            race_loaded: false,
            race_tracks: Vec::new(),
            race_radar_path: String::new(),
            race_world_size: 6000.0,
            race_world_center_x: 0.0,
            race_world_center_y: 0.0,
            replacement_assets: BTreeMap::new(),
            asset_deletes: HashSet::new(),
            vertex_meshes: Vec::new(),
            col_writes: HashMap::new(),
        }
    }

    #[test]
    fn manual_save_worker_executes_off_the_calling_thread() {
        let root = temp_resource_root("async_manual_save");
        let caller = thread::current().id();
        let rx = spawn_manual_save_worker(empty_manual_save_snapshot(root.clone()));

        let result = loop {
            match rx.recv_timeout(Duration::from_secs(10)).unwrap() {
                ManualSaveMessage::Progress(_) => continue,
                ManualSaveMessage::Finished(result) => break result,
            }
        };

        assert_ne!(result.worker_thread, caller);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert!(root.join(EAGLE_SCENE_FILE).is_file());
        assert!(
            read_eagle_scene_section(&root, SECTION_LIGHTS)
                .unwrap()
                .is_some()
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn manual_save_is_mutually_exclusive_with_other_save_writers() {
        assert_eq!(
            primary_save_writer_conflict(true, false, false),
            Some("manual save")
        );
        assert_eq!(
            primary_save_writer_conflict(false, true, false),
            Some("Editing IMG save")
        );
        assert_eq!(
            primary_save_writer_conflict(false, false, true),
            Some("autosave")
        );
        assert_eq!(primary_save_writer_conflict(false, false, false), None);
    }

    #[test]
    fn save_reconciliation_keeps_values_changed_after_capture() {
        let captured = HashMap::from([("saved".to_string(), 1u8), ("changed".to_string(), 1u8)]);
        let mut current = HashMap::from([
            ("saved".to_string(), 1u8),
            ("changed".to_string(), 2u8),
            ("new".to_string(), 3u8),
        ]);

        reconcile_hash_map(&mut current, &captured);

        assert!(!current.contains_key("saved"));
        assert_eq!(current.get("changed"), Some(&2));
        assert_eq!(current.get("new"), Some(&3));
    }

    #[test]
    fn resource_save_snapshot_does_not_mark_editing_deletions_persisted() {
        let previous_deleted = BTreeSet::from(["old.dff".to_string()]);
        let current_deleted =
            BTreeSet::from(["old.dff".to_string(), "newly_deleted.dff".to_string()]);
        let baseline = saved_editing_deleted_baseline(Some(&previous_deleted));

        assert_ne!(baseline, current_deleted);
        assert_eq!(baseline, previous_deleted);
    }

    #[test]
    fn scene_save_writes_eagle_zone_offsets_before_zone_names() {
        let root = temp_resource_root("zone_offsets");
        let zones = vec!["test".to_string(), "second".to_string()];
        let offsets = EagleZoneOffsets {
            offset: Some(V3 {
                x: 1.25,
                y: -2.0,
                z: 3.0,
            }),
            water_offset: Some(V3 {
                x: 4.0,
                y: 5.5,
                z: 6.0,
            }),
        };

        write_scene_files_data(
            &zones,
            offsets,
            &[],
            &HashMap::new(),
            &HashSet::new(),
            &[],
            &root,
        )
        .unwrap();

        assert_eq!(
            fs::read_to_string(root.join("eagleZones.txt")).unwrap(),
            "#offset 1.25,-2,3\n#waterOffset 4,5.5,6\ntest\nsecond\n"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn scene_save_rejects_zone_path_traversal_before_writing() {
        let root = temp_resource_root("zone_traversal");
        let escaped = root.join("escaped");
        let original_zone_list = fs::read_to_string(root.join("eagleZones.txt")).unwrap();

        let errors = write_scene_files_data(
            &["../escaped".to_string()],
            EagleZoneOffsets::default(),
            &[],
            &HashMap::new(),
            &HashSet::new(),
            &[],
            &root,
        )
        .unwrap_err();

        assert!(errors.iter().any(|error| error.contains("unsafe zone")));
        assert!(!escaped.exists());
        assert_eq!(
            fs::read_to_string(root.join("eagleZones.txt")).unwrap(),
            original_zone_list
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn scene_save_keeps_global_and_per_object_physics_in_separate_files() {
        let root = temp_resource_root("physics_scopes");
        let mut placement_attrs = BTreeMap::new();
        placement_attrs.insert("posX".to_string(), "1".to_string());
        placement_attrs.insert("posY".to_string(), "2".to_string());
        placement_attrs.insert("posZ".to_string(), "3".to_string());
        placement_attrs.insert("physicsRoot".to_string(), "1238".to_string());
        placement_attrs.insert("mass".to_string(), "25".to_string());
        placement_attrs.insert("elasticity".to_string(), "0.2".to_string());
        let placement = Placement {
            id: "physics_crate".to_string(),
            dff: "physics_crate".to_string(),
            zone: "test".to_string(),
            tag: "object".to_string(),
            attrs: placement_attrs,
            pos: V3 {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            rot: V3::default(),
        };

        let mut definition_attrs = BTreeMap::new();
        definition_attrs.insert("dff".to_string(), "physics_crate".to_string());
        definition_attrs.insert("physicsRoot".to_string(), "1224".to_string());
        definition_attrs.insert("simulated".to_string(), "true".to_string());
        definition_attrs.insert("mass".to_string(), "100".to_string());
        definition_attrs.insert("airResistance".to_string(), "0.98".to_string());
        let definition = Definition {
            id: "physics_crate".to_string(),
            zone: "test".to_string(),
            attrs: definition_attrs,
        };
        let definitions = HashMap::from([("physics_crate".to_string(), definition)]);

        write_scene_files_data(
            &["test".to_string()],
            EagleZoneOffsets::default(),
            &[placement],
            &definitions,
            &HashSet::new(),
            &[ElementState::default()],
            &root,
        )
        .unwrap();

        let map = fs::read_to_string(root.join("zones/test/test.map")).unwrap();
        assert!(map.contains("physicsRoot=\"1238\""));
        assert!(map.contains("mass=\"25\""));
        assert!(map.contains("elasticity=\"0.2\""));
        assert!(!map.contains("airResistance=\"0.98\""));

        let definitions = fs::read_to_string(root.join("zones/test/test.definition")).unwrap();
        assert!(definitions.contains("physicsRoot=\"1224\""));
        assert!(!definitions.contains("physicsRoot=\"1238\""));
        assert!(definitions.contains("simulated=\"true\""));
        assert!(definitions.contains("mass=\"100\""));
        assert!(definitions.contains("airResistance=\"0.98\""));
        assert!(!definitions.contains("elasticity=\"0.2\""));
        fs::remove_dir_all(root).unwrap();
    }

    fn test_col_bytes(payload: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"COL2");
        bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        bytes.extend_from_slice(payload);
        bytes
    }

    #[test]
    fn validation_requires_exact_dff_bounds_in_col_header() {
        let dff_bounds = Bounds {
            min: vec3(-1.0, -2.0, -3.0),
            max: vec3(4.0, 5.0, 6.0),
        };
        let mut bytes = vec![0u8; 72];
        bytes[0..4].copy_from_slice(b"COL2");
        for (offset, value) in [
            (32usize, -1.0f32),
            (36, -2.0),
            (40, -3.0),
            (44, 4.0),
            (48, 5.0),
            (52, 6.0),
        ] {
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        assert!(col_header_matches_bounds(&bytes, dff_bounds));

        bytes[44..48].copy_from_slice(&10.0f32.to_le_bytes());
        assert!(!col_header_matches_bounds(&bytes, dff_bounds));
    }

    fn bounds_fix_test_context(
        root: &Path,
        definitions: Vec<Definition>,
    ) -> GeometryOptimizationContext {
        GeometryOptimizationContext {
            root: root.to_path_buf(),
            gta_sa_dir: root.join("gta"),
            loaded_definition_ids: definitions
                .iter()
                .map(|definition| definition.id.clone())
                .collect(),
            definitions,
            building_dffs: HashSet::new(),
            opaque_materials_by_dff: BTreeMap::new(),
        }
    }

    #[test]
    fn live_collision_refresh_uses_bounded_batches() {
        let total = LIVE_RESOURCE_REFRESH_BATCH_LIMIT + 17;
        let bounds = Bounds {
            min: vec3(-1.0, -1.0, -1.0),
            max: vec3(1.0, 1.0, 1.0),
        };
        let mut pending = (0..total)
            .map(|index| {
                (
                    format!("batch_{index}.col"),
                    CollisionMesh {
                        name: format!("batch_{index}"),
                        spheres: Vec::new(),
                        boxes: Vec::new(),
                        vertices: Vec::new(),
                        faces: Vec::new(),
                        bounds,
                        shadow_vertices: Vec::new(),
                        shadow_faces: Vec::new(),
                    },
                )
            })
            .collect::<Vec<_>>();
        let mut collisions = HashMap::new();

        let first_batch = refresh_collision_mesh_batch(&mut collisions, &mut pending);

        assert!(first_batch > 0);
        assert!(first_batch <= LIVE_RESOURCE_REFRESH_BATCH_LIMIT);
        assert!(!pending.is_empty());
        while !pending.is_empty() {
            let refreshed = refresh_collision_mesh_batch(&mut collisions, &mut pending);
            assert!(refreshed > 0);
            assert!(refreshed <= LIVE_RESOURCE_REFRESH_BATCH_LIMIT);
        }
        assert_eq!(collisions.len(), total);
    }

    #[test]
    fn object_bounds_fix_generates_and_assigns_missing_bounds_only_col() {
        let root = temp_resource_root("fix_missing_object_bounds");
        let raw = test_raw_dff_mesh();
        let dff = write_normalized_dff(&raw, "bounds_model").unwrap();
        upsert_replacement_assets(
            &wip_root_path(&root),
            &[("bounds_model.dff".to_string(), dff)],
        )
        .unwrap();
        let definition = Definition {
            id: "bounds_definition".to_string(),
            zone: "test".to_string(),
            attrs: BTreeMap::from([("dff".to_string(), "bounds_model".to_string())]),
        };

        let result = run_object_bounds_fix(
            bounds_fix_test_context(&root, vec![definition]),
            HashSet::new(),
        );

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.generated_cols, 1);
        assert_eq!(
            result.assignments,
            vec![("bounds_definition".to_string(), "bounds_model".to_string())]
        );
        let (_, col) = result
            .staged_assets
            .iter()
            .find(|(name, _)| name == "bounds_model.col")
            .unwrap();
        assert!(col_header_matches_bounds(
            col,
            bounds_from_vertices(&raw.vertices)
        ));
        let (_, mesh) = result
            .collision_meshes
            .iter()
            .find(|(name, _)| name == "bounds_model.col")
            .unwrap();
        assert!(mesh.faces.is_empty());
        assert!(mesh.boxes.is_empty());
        assert!(mesh.spheres.is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn object_bounds_fix_unions_dff_and_existing_collision_geometry() {
        let root = temp_resource_root("fix_combined_object_bounds");
        let raw = test_raw_dff_mesh();
        let dff = write_normalized_dff(&raw, "combined_model").unwrap();
        let dff_bounds = bounds_from_vertices(&raw.vertices);
        let mut mesh = CollisionMesh {
            name: "combined_model".to_string(),
            spheres: Vec::new(),
            boxes: Vec::new(),
            vertices: Vec::new(),
            faces: Vec::new(),
            bounds: dff_bounds,
            shadow_vertices: Vec::new(),
            shadow_faces: Vec::new(),
        };
        mesh.boxes.push(CollisionBox {
            min: V3 {
                x: -4.0,
                y: -3.0,
                z: -2.0,
            },
            max: V3 {
                x: 0.5,
                y: 0.5,
                z: 0.5,
            },
            surface: CollisionSurface {
                material: 0,
                flags: 0,
                brightness: 0,
                light: 0,
            },
        });
        let template = col_regeneration_template(&[], "combined_model.col");
        let mut col =
            write_col_mesh_from_template_with_bounds(&template, &mesh, Some(dff_bounds)).unwrap();
        set_col_model_names_from_entry(&mut col, "combined_model.col");
        upsert_replacement_assets(
            &wip_root_path(&root),
            &[
                ("combined_model.dff".to_string(), dff),
                ("combined_model.col".to_string(), col),
            ],
        )
        .unwrap();
        let definition = Definition {
            id: "combined_definition".to_string(),
            zone: "test".to_string(),
            attrs: BTreeMap::from([
                ("dff".to_string(), "combined_model".to_string()),
                ("col".to_string(), "combined_model".to_string()),
            ]),
        };

        let result = run_object_bounds_fix(
            bounds_fix_test_context(&root, vec![definition]),
            HashSet::new(),
        );

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.repaired_cols, 1);
        let expected = union_bounds(
            dff_bounds,
            Bounds {
                min: vec3(-4.0, -3.0, -2.0),
                max: vec3(0.5, 0.5, 0.5),
            },
        );
        let (_, repaired) = result
            .staged_assets
            .iter()
            .find(|(name, _)| name == "combined_model.col")
            .unwrap();
        assert!(col_header_matches_bounds(repaired, expected));
        fs::remove_dir_all(root).unwrap();
    }

    fn test_raw_dff_mesh() -> RawMesh {
        RawMesh {
            vertices: vec![
                V3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                V3 {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                V3 {
                    x: 0.0,
                    y: 1.0,
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
            uvs: vec![
                V2 { u: 0.0, v: 0.0 },
                V2 { u: 1.0, v: 0.0 },
                V2 { u: 0.0, v: 1.0 },
            ],
            prelit_colors: vec![
                V3 {
                    x: 1.0,
                    y: 1.0,
                    z: 1.0,
                };
                3
            ],
            material_textures: vec!["test_texture".to_string()],
            triangles: vec![Tri {
                a: 0,
                b: 1,
                c: 2,
                material: 0,
            }],
            ..RawMesh::default()
        }
    }

    #[test]
    fn autosave_serializes_the_active_dirty_dff_snapshot() {
        let mut raw = test_raw_dff_mesh();
        raw.vertices[0].x = 37.5;
        let asset = AutosaveEditingAsset::Dff {
            name: "autosave_active.dff".to_string(),
            raw,
            boolean_box: None,
        };

        let (name, bytes) = serialize_autosave_editing_asset(asset).unwrap();
        let reparsed = parse_dff_mesh(&bytes);

        assert_eq!(name, "autosave_active.dff");
        assert!((reparsed.vertices[0].x - 37.5).abs() < 1.0e-6);
    }

    fn contains_dff_chunk(bytes: &[u8], target: u32) -> bool {
        fn scan(bytes: &[u8], start: usize, end: usize, target: u32) -> bool {
            let mut o = start;
            while o + 12 <= end {
                let id = rd32(bytes, o);
                let size = rd32(bytes, o + 4) as usize;
                let cs = o + 12;
                let ce = cs.saturating_add(size);
                if ce > end || ce > bytes.len() {
                    return false;
                }
                if id == target || scan(bytes, cs, ce, target) {
                    return true;
                }
                o = ce;
            }
            false
        }
        scan(bytes, 0, dff_chunk_len(bytes), target)
    }

    fn remove_dff_chunk(bytes: &[u8], target: u32) -> Vec<u8> {
        fn rebuild(bytes: &[u8], start: usize, end: usize, target: u32) -> Option<Vec<u8>> {
            let mut o = start;
            let mut out = Vec::with_capacity(end.saturating_sub(start));
            let mut changed = false;
            while o + 12 <= end {
                let id = rd32(bytes, o);
                let size = rd32(bytes, o + 4) as usize;
                let version = rd32(bytes, o + 8);
                let cs = o + 12;
                let ce = cs.saturating_add(size);
                if ce > end || ce > bytes.len() {
                    return None;
                }
                if id == target {
                    changed = true;
                } else if let Some(payload) = rebuild(bytes, cs, ce, target) {
                    out.extend_from_slice(&id.to_le_bytes());
                    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
                    out.extend_from_slice(&version.to_le_bytes());
                    out.extend_from_slice(&payload);
                    changed = true;
                } else {
                    out.extend_from_slice(&bytes[o..ce]);
                }
                o = ce;
            }
            if o != end {
                return None;
            }
            changed.then_some(out)
        }
        let len = dff_chunk_len(bytes);
        let mut out = rebuild(bytes, 0, len, target).unwrap_or_else(|| bytes[..len].to_vec());
        if len < bytes.len() {
            out.extend_from_slice(&bytes[len..]);
        }
        out
    }

    fn remove_day_prelight_stream(bytes: &[u8]) -> Vec<u8> {
        fn rebuild(bytes: &[u8], start: usize, end: usize) -> Option<Vec<u8>> {
            let mut o = start;
            let mut out = Vec::with_capacity(end.saturating_sub(start));
            let mut changed = false;
            while o + 12 <= end {
                let id = rd32(bytes, o);
                let size = rd32(bytes, o + 4) as usize;
                let version = rd32(bytes, o + 8);
                let cs = o + 12;
                let ce = cs.saturating_add(size);
                if ce > end || ce > bytes.len() {
                    return None;
                }
                if id == 0x01 && size >= 16 {
                    let flags = rd32(bytes, cs);
                    let vert_count = rd32(bytes, cs + 8) as usize;
                    let colors_start = cs + 16;
                    let colors_end = colors_start.saturating_add(vert_count * 4);
                    if flags & 0x08 != 0 && colors_end <= ce {
                        let mut payload = bytes[cs..ce].to_vec();
                        payload[0..4].copy_from_slice(&(flags & !0x08).to_le_bytes());
                        payload.drain(16..16 + vert_count * 4);
                        out.extend_from_slice(&id.to_le_bytes());
                        out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
                        out.extend_from_slice(&version.to_le_bytes());
                        out.extend_from_slice(&payload);
                        changed = true;
                    } else {
                        out.extend_from_slice(&bytes[o..ce]);
                    }
                } else if let Some(payload) = rebuild(bytes, cs, ce) {
                    out.extend_from_slice(&id.to_le_bytes());
                    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
                    out.extend_from_slice(&version.to_le_bytes());
                    out.extend_from_slice(&payload);
                    changed = true;
                } else {
                    out.extend_from_slice(&bytes[o..ce]);
                }
                o = ce;
            }
            if o != end {
                return None;
            }
            changed.then_some(out)
        }
        let len = dff_chunk_len(bytes);
        let mut out = rebuild(bytes, 0, len).unwrap_or_else(|| bytes[..len].to_vec());
        if len < bytes.len() {
            out.extend_from_slice(&bytes[len..]);
        }
        out
    }

    fn append_geometry_extension_chunk(bytes: &[u8], plugin: &[u8]) -> Vec<u8> {
        fn rebuild(
            bytes: &[u8],
            start: usize,
            end: usize,
            parent: u32,
            plugin: &[u8],
        ) -> Option<Vec<u8>> {
            let mut o = start;
            let mut out = Vec::with_capacity(end.saturating_sub(start) + plugin.len());
            let mut changed = false;
            while o + 12 <= end {
                let id = rd32(bytes, o);
                let size = rd32(bytes, o + 4) as usize;
                let version = rd32(bytes, o + 8);
                let cs = o + 12;
                let ce = cs.saturating_add(size);
                if ce > end || ce > bytes.len() {
                    return None;
                }
                if id == 0x03 && parent == 0x0f {
                    let mut payload = bytes[cs..ce].to_vec();
                    payload.extend_from_slice(plugin);
                    out.extend_from_slice(&id.to_le_bytes());
                    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
                    out.extend_from_slice(&version.to_le_bytes());
                    out.extend_from_slice(&payload);
                    changed = true;
                } else if matches!(id, 0x10 | 0x0e | 0x1a | 0x0f | 0x08 | 0x07 | 0x06 | 0x14)
                    && let Some(payload) = rebuild(bytes, cs, ce, id, plugin)
                {
                    out.extend_from_slice(&id.to_le_bytes());
                    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
                    out.extend_from_slice(&version.to_le_bytes());
                    out.extend_from_slice(&payload);
                    changed = true;
                } else {
                    out.extend_from_slice(&bytes[o..ce]);
                }
                o = ce;
            }
            if o != end {
                return None;
            }
            changed.then_some(out)
        }

        let len = dff_chunk_len(bytes);
        let mut out = rebuild(bytes, 0, len, 0, plugin).unwrap_or_else(|| bytes[..len].to_vec());
        out.extend_from_slice(&bytes[len..]);
        out
    }

    fn dff_chunk_payloads(bytes: &[u8], target: u32) -> Vec<Vec<u8>> {
        fn scan(bytes: &[u8], start: usize, end: usize, target: u32, out: &mut Vec<Vec<u8>>) {
            let mut o = start;
            while o + 12 <= end {
                let id = rd32(bytes, o);
                let size = rd32(bytes, o + 4) as usize;
                let cs = o + 12;
                let ce = cs.saturating_add(size);
                if ce > end || ce > bytes.len() {
                    return;
                }
                if id == target {
                    out.push(bytes[cs..ce].to_vec());
                } else if matches!(
                    id,
                    0x10 | 0x0e | 0x1a | 0x0f | 0x08 | 0x07 | 0x06 | 0x14 | 0x03
                ) {
                    scan(bytes, cs, ce, target, out);
                }
                o = ce;
            }
        }
        let mut out = Vec::new();
        scan(bytes, 0, dff_chunk_len(bytes), target, &mut out);
        out
    }

    #[test]
    fn autosave_restore_only_when_autosave_is_newer() {
        let root = temp_resource_root("autosave_newer");
        assert!(!autosave_is_newer_than_saved(&root));

        let autosave_root = autosave_root_path(&root);
        fs::create_dir_all(autosave_root.join("zones").join("test")).unwrap();
        fs::write(autosave_root.join("eagleZones.txt"), "test\n").unwrap();
        fs::write(
            autosave_root.join("zones").join("test").join("test.map"),
            "<map>\n</map>\n",
        )
        .unwrap();
        std::thread::sleep(Duration::from_millis(1200));
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        fs::write(
            autosave_meta_path(&root),
            format!("unix_millis={}\n", now.as_millis()),
        )
        .unwrap();
        assert!(autosave_is_newer_than_saved(&root));

        std::thread::sleep(Duration::from_millis(1200));
        fs::write(root.join("Light_List.xml"), "<lights />\n").unwrap();
        assert!(!autosave_is_newer_than_saved(&root));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn incomplete_autosave_without_commit_marker_is_not_restorable() {
        let root = temp_resource_root("autosave_incomplete");
        let autosave_root = autosave_root_path(&root);
        fs::create_dir_all(autosave_root.join("zones").join("test")).unwrap();
        fs::write(autosave_root.join("eagleZones.txt"), "test\n").unwrap();
        fs::write(
            autosave_root.join("zones").join("test").join("test.map"),
            "<map>\n</map>\n",
        )
        .unwrap();

        assert!(!autosave_meta_path(&root).exists());
        assert!(!autosave_is_newer_than_saved(&root));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn autosave_cleanup_removes_only_the_autosave_directory() {
        let root = temp_resource_root("autosave_cleanup");
        let autosave_root = autosave_root_path(&root);
        fs::create_dir_all(autosave_root.join("zones").join("test")).unwrap();
        fs::write(autosave_root.join("autosave.txt"), "snapshot\n").unwrap();
        let saved_map = root.join("zones").join("test").join("test.map");

        assert_eq!(cleanup_autosaves(&root), Ok(true));
        assert!(!root.join(".eagle_autosave").exists());
        assert!(saved_map.is_file());
        assert_eq!(cleanup_autosaves(&root), Ok(false));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn definition_asset_names_are_canonicalized_to_real_asset_case() {
        let mut index = AssetCaseIndex::default();
        index.add_file_name("com_landnew25.dff");
        index.add_file_name("com_landnew25.col");
        index.add_file_name("lcsageneric.txd");

        let def = Definition {
            id: "Com_Landnew25".to_string(),
            zone: "comnbtm".to_string(),
            attrs: BTreeMap::from([("txd".to_string(), "LCSAgeneric".to_string())]),
        };
        let mut attrs = def.attrs.clone();

        canonicalize_definition_asset_attrs(&def, &mut attrs, &index);

        assert_eq!(attrs.get("dff"), None);
        assert_eq!(attrs.get("col").map(String::as_str), Some("com_landnew25"));
        assert_eq!(attrs.get("txd").map(String::as_str), Some("lcsageneric"));
    }

    #[test]
    fn definition_dff_override_is_preserved_when_different_from_id() {
        let mut index = AssetCaseIndex::default();
        index.add_file_name("custom_mesh.dff");

        let def = Definition {
            id: "building_a".to_string(),
            zone: "comnbtm".to_string(),
            attrs: BTreeMap::from([
                ("dff".to_string(), "custom_mesh".to_string()),
                ("nativeModel".to_string(), "1352".to_string()),
            ]),
        };
        let mut attrs = def.attrs.clone();

        canonicalize_definition_asset_attrs(&def, &mut attrs, &index);

        assert_eq!(attrs.get("dff").map(String::as_str), Some("custom_mesh"));
        assert_eq!(attrs.get("nativeModel").map(String::as_str), Some("1352"));
    }

    #[test]
    fn replacement_archive_entry_can_be_renamed() {
        let root = std::env::temp_dir().join(format!(
            "eagle_editor_replacement_rename_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let img_dir = root.join("imgs");
        fs::create_dir_all(&img_dir).unwrap();
        let img_path = img_dir.join(REPLACEMENT_IMG);
        write_img_archive(
            &img_path,
            &[
                ("old_model.dff".to_string(), vec![1, 2, 3, 4]),
                ("other_model.col".to_string(), vec![5, 6, 7, 8]),
            ],
        )
        .unwrap();

        assert!(rename_replacement_archive_entry(&root, "old_model.dff", "new_model.dff").unwrap());

        let names: Vec<String> = parse_img(&img_path)
            .into_iter()
            .map(|entry| entry.name)
            .collect();
        assert_eq!(names, vec!["new_model.dff", "other_model.col"]);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn staged_col_replacement_is_pending() {
        let root = std::env::temp_dir().join(format!(
            "eagle_editor_replacement_pending_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let col_bytes = test_col_bytes(b"COL replacement bytes");
        upsert_replacement_asset(&root, "example.col", &col_bytes).unwrap();

        let entries = pending_replacement_entries_for_root(&root).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, "example.col");
        assert_eq!(entries[0].1, col_bytes);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn col_replacement_overwrites_existing_col_archive_entry() {
        let root = std::env::temp_dir().join(format!(
            "eagle_editor_col_merge_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let img_dir = root.join("imgs");
        fs::create_dir_all(&img_dir).unwrap();
        let col_img = img_dir.join("col.img");
        write_img_archive(
            &col_img,
            &[("example.col".to_string(), test_col_bytes(b"old col bytes"))],
        )
        .unwrap();
        let new_col_bytes = test_col_bytes(b"new col bytes");

        let destinations = ReplacementDestinationIndex::build(&root);
        assert_eq!(destinations.archive_path("example.col").unwrap(), col_img);
        assert_eq!(
            upsert_img_archive_entries_owned(
                &col_img,
                vec![("example.col".to_string(), new_col_bytes.clone())],
            )
            .unwrap(),
            1
        );

        let entry = parse_img(&col_img)
            .into_iter()
            .find(|entry| entry.name == "example.col")
            .unwrap();
        let mut bytes = read_img_entry(&entry);
        bytes.truncate(new_col_bytes.len());
        assert_eq!(bytes, new_col_bytes);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn repair_preserves_valid_normals_in_building_dffs() {
        let root = std::env::temp_dir().join(format!(
            "eagle_editor_building_repair_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let img_dir = root.join("imgs");
        fs::create_dir_all(&img_dir).unwrap();
        let dff_bytes = write_normalized_dff(&test_raw_dff_mesh(), "building").unwrap();
        assert!(dff_has_geometry_normals(&dff_bytes));
        let dff_img = img_dir.join("dff.img");
        write_img_archive(&dff_img, &[("building.dff".to_string(), dff_bytes)]).unwrap();

        let result = repair_dffs_in_root_with_buildings(
            &root,
            HashSet::from([asset_key("building.dff", ".dff")]),
        );

        assert_eq!(result.errors, Vec::<String>::new());
        assert_eq!(result.repaired, 0);
        let entry = parse_img(&dff_img)
            .into_iter()
            .find(|entry| entry.name == "building.dff")
            .unwrap();
        let mut repaired = read_img_entry(&entry);
        repaired.truncate(dff_chunk_len(&repaired));
        assert!(dff_has_geometry_normals(&repaired));
        assert!(contains_dff_chunk(&repaired, 0x050e));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn validation_dff_repair_sanitizes_unambiguous_txd_texture_mismatch() {
        let root = std::env::temp_dir().join(format!(
            "eagle_editor_dff_texture_name_repair_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let img_dir = root.join("imgs");
        fs::create_dir_all(&img_dir).unwrap();
        let mut raw = test_raw_dff_mesh();
        raw.material_textures = vec!["new road".to_string()];
        let dff_bytes = write_normalized_dff(&raw, "starisland_road1").unwrap();
        let dff_img = img_dir.join("dff.img");
        write_img_archive(&dff_img, &[("starisland_road1.dff".to_string(), dff_bytes)]).unwrap();

        let texture_context = DffTextureSanitizeContext {
            scopes_by_dff: HashMap::from([(
                "starisland_road1.dff".to_string(),
                BTreeSet::from(["starroad.txd".to_string()]),
            )]),
            textures_by_txd: HashMap::from([(
                "starroad.txd".to_string(),
                HashSet::from(["new_road".to_string()]),
            )]),
        };
        let result = repair_dffs_in_root_with_context(
            &root,
            HashSet::new(),
            texture_context,
            DffRepairScope {
                dff_issues: false,
                prelighting: false,
                texture_names: true,
            },
        );

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.sanitized_texture_names, 1);
        assert_eq!(result.sanitized_texture_references, 1);
        assert!(result.repaired_entries.iter().any(|line| {
            line.contains("starisland_road1.dff") && line.contains("new road -> new_road")
        }));
        let entry = parse_img(&dff_img)
            .into_iter()
            .find(|entry| entry.name == "starisland_road1.dff")
            .unwrap();
        let mut repaired = read_img_entry(&entry);
        repaired.truncate(dff_chunk_len(&repaired));
        assert_eq!(
            parse_dff_mesh(&repaired).material_textures,
            vec!["new_road".to_string()]
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn dff_repair_scope_can_skip_texture_name_changes() {
        let root = std::env::temp_dir().join(format!(
            "eagle_editor_dff_issue_only_repair_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let img_dir = root.join("imgs");
        fs::create_dir_all(&img_dir).unwrap();
        let mut raw = test_raw_dff_mesh();
        raw.material_textures = vec!["new road".to_string()];
        let dff_img = img_dir.join("dff.img");
        write_img_archive(
            &dff_img,
            &[(
                "starisland_road1.dff".to_string(),
                write_normalized_dff(&raw, "starisland_road1").unwrap(),
            )],
        )
        .unwrap();
        let texture_context = DffTextureSanitizeContext {
            scopes_by_dff: HashMap::from([(
                "starisland_road1.dff".to_string(),
                BTreeSet::from(["starroad.txd".to_string()]),
            )]),
            textures_by_txd: HashMap::from([(
                "starroad.txd".to_string(),
                HashSet::from(["new_road".to_string()]),
            )]),
        };

        let result = repair_dffs_in_root_with_context(
            &root,
            HashSet::new(),
            texture_context,
            DffRepairScope {
                dff_issues: true,
                prelighting: false,
                texture_names: false,
            },
        );

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.sanitized_texture_names, 0);
        assert_eq!(result.sanitized_texture_references, 0);
        let entry = parse_img(&dff_img)
            .into_iter()
            .find(|entry| entry.name == "starisland_road1.dff")
            .unwrap();
        let mut repaired = read_img_entry(&entry);
        repaired.truncate(dff_chunk_len(&repaired));
        assert_eq!(
            parse_dff_mesh(&repaired).material_textures,
            vec!["new road".to_string()]
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn dff_texture_name_repair_skips_ambiguous_shared_txd_scopes() {
        let mut raw = test_raw_dff_mesh();
        raw.material_textures = vec!["new road".to_string()];
        let dff = write_normalized_dff(&raw, "shared_road").unwrap();
        let context = DffTextureSanitizeContext {
            scopes_by_dff: HashMap::from([(
                "shared_road.dff".to_string(),
                BTreeSet::from(["starroad.txd".to_string(), "legacyroad.txd".to_string()]),
            )]),
            textures_by_txd: HashMap::from([
                (
                    "starroad.txd".to_string(),
                    HashSet::from(["new_road".to_string()]),
                ),
                (
                    "legacyroad.txd".to_string(),
                    HashSet::from(["new road".to_string()]),
                ),
            ]),
        };

        let repaired = sanitize_dff_texture_references("shared_road.dff", &dff, &context).unwrap();

        assert!(repaired.is_none());
    }

    #[test]
    fn repair_fills_missing_night_prelight_stream() {
        let root = std::env::temp_dir().join(format!(
            "eagle_editor_missing_night_prelight_repair_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let img_dir = root.join("imgs");
        fs::create_dir_all(&img_dir).unwrap();
        let mut raw = test_raw_dff_mesh();
        raw.prelit_colors = vec![
            V3 {
                x: 0.2,
                y: 0.4,
                z: 0.6,
            };
            3
        ];
        let dff_bytes = remove_dff_chunk(
            &write_normalized_dff(&raw, "day_only").unwrap(),
            0x0253_f2f9,
        );
        assert!(!contains_dff_chunk(&dff_bytes, 0x0253_f2f9));
        let dff_img = img_dir.join("dff.img");
        write_img_archive(&dff_img, &[("day_only.dff".to_string(), dff_bytes)]).unwrap();

        let result = repair_dffs_in_root_with_context(
            &root,
            HashSet::new(),
            DffTextureSanitizeContext::default(),
            DffRepairScope {
                dff_issues: false,
                prelighting: true,
                texture_names: false,
            },
        );

        assert_eq!(result.errors, Vec::<String>::new());
        assert_eq!(result.prelighting_fixed, 1);
        assert_eq!(result.normalized, 0);
        assert!(
            result
                .repaired_entries
                .iter()
                .any(|line| line.contains("day_only.dff")
                    && line.contains("filled missing day/night prelight stream"))
        );
        let entry = parse_img(&dff_img)
            .into_iter()
            .find(|entry| entry.name == "day_only.dff")
            .unwrap();
        let mut repaired = read_img_entry(&entry);
        repaired.truncate(dff_chunk_len(&repaired));
        let reparsed = parse_dff_mesh(&repaired);
        assert!(contains_dff_chunk(&repaired, 0x0253_f2f9));
        assert_eq!(reparsed.prelit_colors, reparsed.night_prelit_colors);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn surgical_prelight_repair_preserves_2dfx_and_unknown_plugins_byte_exactly() {
        let mut raw = test_raw_dff_mesh();
        raw.effects_2dfx.push(Dff2dEffect {
            position: V3 {
                x: 12.5,
                y: -3.25,
                z: 8.0,
            },
            effect_id: 0,
            payload: (0..80).map(|value| value as u8).collect(),
        });
        let unknown_id = 0x0bad_c0de;
        let unknown_payload = b"vice-city-extension-must-survive".to_vec();
        let with_unknown = append_geometry_extension_chunk(
            &write_normalized_dff(&raw, "vc_effects").unwrap(),
            &rw_chunk(unknown_id, unknown_payload.clone()),
        );
        let day_only = remove_dff_chunk(&with_unknown, 0x0253_f2f9);
        let effects_before = dff_chunk_payloads(&day_only, 0x0253_f2f8);
        let unknown_before = dff_chunk_payloads(&day_only, unknown_id);

        let repaired = repair_dff_missing_prelight_streams(&day_only)
            .unwrap()
            .expect("missing night stream should be inserted");

        assert_eq!(dff_chunk_payloads(&repaired, 0x0253_f2f8), effects_before);
        assert_eq!(dff_chunk_payloads(&repaired, unknown_id), unknown_before);
        assert_eq!(unknown_before, vec![unknown_payload]);
        assert!(contains_dff_chunk(&repaired, 0x0253_f2f9));
        assert!(parse_dff_mesh(&repaired).effects_2dfx == parse_dff_mesh(&day_only).effects_2dfx);
        let (invalid, issues) = dff_geometry_issue_summary(&repaired);
        assert!(!invalid, "{issues:?}");
    }

    #[test]
    fn normalized_structure_repair_refuses_to_discard_unknown_vc_plugins() {
        let unknown_id = 0x0bad_c0de;
        let with_unknown = append_geometry_extension_chunk(
            &write_normalized_dff(&test_raw_dff_mesh(), "vc_unknown").unwrap(),
            &rw_chunk(unknown_id, b"opaque-vc-data".to_vec()),
        );
        let missing_bin_mesh = remove_dff_chunk(&with_unknown, 0x050e);
        let (invalid, issues) = dff_geometry_issue_summary(&missing_bin_mesh);
        assert!(invalid);
        assert!(
            issues
                .iter()
                .any(|issue| issue == "missing bin mesh plugin")
        );

        let err = normalize_dff_entry("vc_unknown.dff", &missing_bin_mesh, true).unwrap_err();

        assert!(err.contains("would discard VC/RenderWare data"), "{err}");
        assert!(err.contains("0x0badc0de"), "{err}");
    }

    #[test]
    fn normalized_structure_repair_round_trips_supported_2dfx() {
        let mut raw = test_raw_dff_mesh();
        raw.effects_2dfx.push(Dff2dEffect {
            position: V3 {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            effect_id: 7,
            payload: vec![0xde, 0xad, 0xbe, 0xef],
        });
        let source = remove_dff_chunk(
            &write_normalized_dff(&raw, "supported_2dfx").unwrap(),
            0x050e,
        );

        let repaired = normalize_dff_entry("supported_2dfx.dff", &source, true).unwrap();

        assert!(parse_dff_mesh(&repaired).effects_2dfx == parse_dff_mesh(&source).effects_2dfx);
        let (invalid, issues) = dff_geometry_issue_summary(&repaired);
        assert!(!invalid, "{issues:?}");
    }

    /// Read-only integration audit for a real Vice City DFF archive.
    ///
    /// Run with:
    /// `EAGLE_VC_DFF_IMG=/path/to/dff.img cargo test vc_archive_dff_repair_audit -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn vc_archive_dff_repair_audit() {
        let path = PathBuf::from(
            std::env::var("EAGLE_VC_DFF_IMG").expect("EAGLE_VC_DFF_IMG must name a VC IMG"),
        );
        let entries = parse_img(&path);
        assert!(!entries.is_empty(), "{} has no IMG entries", path.display());
        let preserved_plugin_ids = [
            0x0105,
            0x0110,
            0x011e,
            0x0120,
            0x0135,
            0x0253_f2f6,
            0x0253_f2f8,
            0x0253_f2fc,
            0x0253_f2fd,
            0x0253_f2fe,
            0x001f,
        ];
        let mut scanned = 0usize;
        let mut prelight_candidates = 0usize;
        let mut structural_errors = Vec::new();
        for entry in entries
            .iter()
            .filter(|entry| entry.name.to_ascii_lowercase().ends_with(".dff"))
        {
            scanned += 1;
            let mut bytes = read_img_entry(entry);
            bytes.truncate(dff_chunk_len(&bytes));
            let (invalid, issues) = dff_geometry_issue_summary(&bytes);
            if invalid {
                structural_errors.push(format!("{}: {}", entry.name, issues.join(", ")));
            }
            if let Some(repaired) = repair_dff_missing_prelight_streams(&bytes)
                .unwrap_or_else(|err| panic!("{} failed prelight dry-run: {err}", entry.name))
            {
                prelight_candidates += 1;
                for id in preserved_plugin_ids {
                    assert_eq!(
                        dff_chunk_payloads(&repaired, id),
                        dff_chunk_payloads(&bytes, id),
                        "{} changed plugin 0x{id:08x}",
                        entry.name
                    );
                }
                let (invalid, issues) = dff_geometry_issue_summary(&repaired);
                assert!(!invalid, "{} after prelight repair: {issues:?}", entry.name);
            }
        }
        assert!(
            structural_errors.is_empty(),
            "VC structural errors:\n{}",
            structural_errors.join("\n")
        );
        eprintln!(
            "VC DFF audit: scanned {scanned}, structurally valid {scanned}, prelight repair candidates {prelight_candidates}, preserved all inventoried FX/plugin payloads"
        );
    }

    /// Full background-repair integration pass against a temporary copy of a
    /// real VC archive. The source archive is never modified.
    #[test]
    #[ignore]
    fn vc_archive_full_repair_copy_audit() {
        let source = PathBuf::from(
            std::env::var("EAGLE_VC_DFF_IMG").expect("EAGLE_VC_DFF_IMG must name a VC IMG"),
        );
        let root = std::env::temp_dir().join(format!(
            "eagle_editor_vc_full_dff_audit_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let imgs = root.join("imgs");
        fs::create_dir_all(&imgs).unwrap();
        let target = imgs.join("dff.img");
        fs::copy(&source, &target).unwrap();

        let source_entries = parse_img(&source)
            .into_iter()
            .filter(|entry| entry.name.to_ascii_lowercase().ends_with(".dff"))
            .map(|entry| (lower(&entry.name), entry))
            .collect::<HashMap<_, _>>();
        let expected_prelight_repairs = source_entries
            .values()
            .filter(|entry| {
                let mut bytes = read_img_entry(entry);
                bytes.truncate(dff_chunk_len(&bytes));
                repair_dff_missing_prelight_streams(&bytes)
                    .expect("source prelight audit should parse")
                    .is_some()
            })
            .count();
        let result = repair_dffs_in_root_with_context(
            &root,
            HashSet::new(),
            DffTextureSanitizeContext::default(),
            DffRepairScope {
                dff_issues: true,
                prelighting: true,
                texture_names: false,
            },
        );

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.scanned, source_entries.len());
        assert_eq!(result.prelighting_fixed, expected_prelight_repairs);
        assert_eq!(result.normalized, 0);

        let preserved_plugin_ids = [
            0x001f,
            0x0105,
            0x0110,
            0x011e,
            0x0120,
            0x0135,
            0x050e,
            0x0253_f2f6,
            0x0253_f2f8,
            0x0253_f2fc,
            0x0253_f2fd,
            0x0253_f2fe,
        ];
        for repaired_entry in parse_img(&target)
            .into_iter()
            .filter(|entry| entry.name.to_ascii_lowercase().ends_with(".dff"))
        {
            let source_entry = &source_entries[&lower(&repaired_entry.name)];
            let mut before = read_img_entry(source_entry);
            before.truncate(dff_chunk_len(&before));
            let mut after = read_img_entry(&repaired_entry);
            after.truncate(dff_chunk_len(&after));
            for id in preserved_plugin_ids {
                let mut remaining = dff_chunk_payloads(&after, id);
                for payload in dff_chunk_payloads(&before, id) {
                    let Some(index) = remaining.iter().position(|candidate| *candidate == payload)
                    else {
                        panic!("{} lost or changed plugin 0x{id:08x}", repaired_entry.name);
                    };
                    remaining.remove(index);
                }
            }
        }
        eprintln!(
            "VC full repair copy: scanned {}, repaired {}, prelight {}, bounds {}, UV pipeline {}, normalized {}; all original FX/plugin payloads survived",
            result.scanned,
            result.repaired,
            result.prelighting_fixed,
            result.bounds_fixed,
            result.uv_anim_pipeline_fixed,
            result.normalized
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn repair_matches_prelight_ranges_before_normal_seams_are_preserved() {
        let root = std::env::temp_dir().join(format!(
            "eagle_editor_prelight_before_weld_repair_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let img_dir = root.join("imgs");
        fs::create_dir_all(&img_dir).unwrap();
        let mut raw = test_raw_dff_mesh();
        raw.vertices.push(raw.vertices[0]);
        raw.normals.push(V3 {
            x: 0.01,
            y: 0.0,
            z: 0.99995,
        });
        raw.uvs.push(raw.uvs[0]);
        raw.prelit_colors.push(raw.prelit_colors[0]);
        raw.triangles.push(Tri {
            a: 3,
            b: 2,
            c: 1,
            material: 0,
        });
        let day_only = remove_dff_chunk(
            &write_normalized_dff(&raw, "duplicate_vertices").unwrap(),
            0x0253_f2f9,
        );
        assert_eq!(
            parse_dff_mesh_preserving_topology(&day_only).vertices.len(),
            4
        );
        // Coincident positions with different authored normals form a real
        // shading seam and are intentionally not welded by the editor parser.
        assert_eq!(parse_dff_mesh(&day_only).vertices.len(), 4);
        let dff_img = img_dir.join("dff.img");
        write_img_archive(
            &dff_img,
            &[("duplicate_vertices.dff".to_string(), day_only)],
        )
        .unwrap();

        let result = repair_dffs_in_root_with_context(
            &root,
            HashSet::new(),
            DffTextureSanitizeContext::default(),
            DffRepairScope {
                dff_issues: false,
                prelighting: true,
                texture_names: false,
            },
        );

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.prelighting_fixed, 1);
        let entry = parse_img(&dff_img)
            .into_iter()
            .find(|entry| entry.name == "duplicate_vertices.dff")
            .unwrap();
        let mut repaired = read_img_entry(&entry);
        repaired.truncate(dff_chunk_len(&repaired));
        let reparsed = parse_dff_mesh(&repaired);
        assert!(contains_dff_chunk(&repaired, 0x0253_f2f9));
        assert_eq!(reparsed.prelit_colors, reparsed.night_prelit_colors);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn dff_repair_scope_can_skip_missing_prelighting() {
        let root = std::env::temp_dir().join(format!(
            "eagle_editor_skip_missing_prelighting_repair_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let img_dir = root.join("imgs");
        fs::create_dir_all(&img_dir).unwrap();
        let day_only = remove_dff_chunk(
            &write_normalized_dff(&test_raw_dff_mesh(), "day_only").unwrap(),
            0x0253_f2f9,
        );
        let dff_img = img_dir.join("dff.img");
        write_img_archive(&dff_img, &[("day_only.dff".to_string(), day_only)]).unwrap();

        let result = repair_dffs_in_root_with_context(
            &root,
            HashSet::new(),
            DffTextureSanitizeContext::default(),
            DffRepairScope {
                dff_issues: false,
                prelighting: false,
                texture_names: true,
            },
        );

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.prelighting_fixed, 0);
        assert_eq!(result.repaired, 0);
        let entry = parse_img(&dff_img)
            .into_iter()
            .find(|entry| entry.name == "day_only.dff")
            .unwrap();
        let mut repaired = read_img_entry(&entry);
        repaired.truncate(dff_chunk_len(&repaired));
        assert!(!contains_dff_chunk(&repaired, 0x0253_f2f9));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn repair_fills_missing_day_prelight_stream_from_night() {
        let root = std::env::temp_dir().join(format!(
            "eagle_editor_missing_day_prelight_repair_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let img_dir = root.join("imgs");
        fs::create_dir_all(&img_dir).unwrap();
        let mut raw = test_raw_dff_mesh();
        raw.prelit_colors = vec![
            V3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            };
            3
        ];
        raw.night_prelit_colors = vec![
            V3 {
                x: 0.1,
                y: 0.2,
                z: 0.3,
            };
            3
        ];
        let night_only =
            remove_day_prelight_stream(&write_normalized_dff(&raw, "night_only").unwrap());
        let dff_img = img_dir.join("dff.img");
        write_img_archive(&dff_img, &[("night_only.dff".to_string(), night_only)]).unwrap();

        let result = repair_dffs_in_root(&root);

        assert_eq!(result.errors, Vec::<String>::new());
        assert_eq!(result.prelighting_fixed, 1);
        assert_eq!(result.normalized, 0);
        let entry = parse_img(&dff_img)
            .into_iter()
            .find(|entry| entry.name == "night_only.dff")
            .unwrap();
        let mut repaired = read_img_entry(&entry);
        repaired.truncate(dff_chunk_len(&repaired));
        let reparsed = parse_dff_mesh(&repaired);
        assert_eq!(reparsed.prelit_colors, reparsed.night_prelit_colors);
        assert_ne!(reparsed.prelit_colors, raw.prelit_colors);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn repair_preserves_existing_distinct_day_and_night_prelight_streams() {
        let root = std::env::temp_dir().join(format!(
            "eagle_editor_distinct_prelight_preserve_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let img_dir = root.join("imgs");
        fs::create_dir_all(&img_dir).unwrap();
        let mut raw = test_raw_dff_mesh();
        raw.prelit_colors = vec![
            V3 {
                x: 0.9,
                y: 0.7,
                z: 0.5,
            };
            3
        ];
        raw.night_prelit_colors = vec![
            V3 {
                x: 0.1,
                y: 0.2,
                z: 0.3,
            };
            3
        ];
        let dff_bytes = write_normalized_dff(&raw, "painted").unwrap();
        let dff_img = img_dir.join("dff.img");
        write_img_archive(&dff_img, &[("painted.dff".to_string(), dff_bytes.clone())]).unwrap();

        let result = repair_dffs_in_root(&root);

        assert_eq!(result.errors, Vec::<String>::new());
        assert_eq!(result.repaired, 0);
        let entry = parse_img(&dff_img)
            .into_iter()
            .find(|entry| entry.name == "painted.dff")
            .unwrap();
        let mut actual = read_img_entry(&entry);
        actual.truncate(dff_chunk_len(&actual));
        assert_eq!(actual, dff_bytes);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn repair_missing_prelight_preserves_uv_animations_and_2dfx() {
        let root = std::env::temp_dir().join(format!(
            "eagle_editor_prelight_repair_sidecar_preserve_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let img_dir = root.join("imgs");
        fs::create_dir_all(&img_dir).unwrap();
        let mut raw = test_raw_dff_mesh();
        raw.material_animations = vec![DffMaterialAnim {
            names: vec!["scroll_sign".to_string()],
        }];
        raw.uv_animations.push(DffUvAnimation {
            name: "scroll_sign".to_string(),
            type_id: 0x1c1,
            flags: 0,
            duration: 1.0,
            node_to_uv: [0, 1, 0, 0, 0, 0, 0, 0],
            frames: vec![
                DffUvAnimFrame {
                    time: 0.0,
                    uv: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
                    prev: -1,
                },
                DffUvAnimFrame {
                    time: 1.0,
                    uv: [1.0, 0.0, 0.25, 0.0, 1.0, 0.0],
                    prev: 0,
                },
            ],
        });
        raw.effects_2dfx.push(Dff2dEffect {
            position: V3 {
                x: 1.5,
                y: 2.5,
                z: 3.5,
            },
            effect_id: 1,
            payload: b"steam\0".to_vec(),
        });
        let day_only = remove_dff_chunk(
            &write_normalized_dff(&raw, "animated_effected").unwrap(),
            0x0253_f2f9,
        );
        assert!(contains_dff_chunk(&day_only, 0x0135));
        assert!(contains_dff_chunk(&day_only, 0x1f));
        assert!(contains_dff_chunk(&day_only, 0x0253_f2f8));
        let dff_img = img_dir.join("dff.img");
        write_img_archive(&dff_img, &[("animated_effected.dff".to_string(), day_only)]).unwrap();

        let result = repair_dffs_in_root(&root);

        assert_eq!(result.errors, Vec::<String>::new());
        assert_eq!(result.normalized, 0);
        let entry = parse_img(&dff_img)
            .into_iter()
            .find(|entry| entry.name == "animated_effected.dff")
            .unwrap();
        let mut repaired = read_img_entry(&entry);
        repaired.truncate(dff_chunk_len(&repaired));
        let reparsed = parse_dff_mesh(&repaired);
        assert!(contains_dff_chunk(&repaired, 0x0135));
        assert!(contains_dff_chunk(&repaired, 0x1f));
        assert!(contains_dff_chunk(&repaired, 0x0253_f2f8));
        assert!(contains_dff_chunk(&repaired, 0x0253_f2f9));
        assert_eq!(
            reparsed
                .material_animations
                .first()
                .map(|animation| animation.names.as_slice()),
            Some(&["scroll_sign".to_string()][..])
        );
        assert_eq!(reparsed.uv_animations.len(), 1);
        assert_eq!(reparsed.uv_animations[0].name, "scroll_sign");
        assert_eq!(reparsed.uv_animations[0].frames.len(), 2);
        assert_eq!(reparsed.uv_animations[0].frames[1].uv[2], 0.25);
        assert_eq!(reparsed.effects_2dfx.len(), 1);
        assert_eq!(reparsed.effects_2dfx[0].effect_id, 1);
        assert_eq!(reparsed.effects_2dfx[0].position.x, 1.5);
        assert_eq!(reparsed.effects_2dfx[0].payload, b"steam\0");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn repair_adds_uv_anim_right_to_render_pipeline() {
        let root = std::env::temp_dir().join(format!(
            "eagle_editor_uv_anim_pipeline_repair_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let img_dir = root.join("imgs");
        fs::create_dir_all(&img_dir).unwrap();
        let mut raw = test_raw_dff_mesh();
        raw.material_animations = vec![DffMaterialAnim {
            names: vec!["scroll_sign".to_string()],
        }];
        raw.uv_animations.push(DffUvAnimation {
            name: "scroll_sign".to_string(),
            type_id: 0x1c1,
            flags: 0,
            duration: 1.0,
            node_to_uv: [0, 1, 0, 0, 0, 0, 0, 0],
            frames: vec![DffUvAnimFrame {
                time: 0.0,
                uv: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                prev: -1,
            }],
        });
        let dff_bytes = write_normalized_dff(&raw, "animated").unwrap();
        assert!(contains_dff_chunk(&dff_bytes, 0x0135));
        assert!(contains_dff_chunk(&dff_bytes, 0x1f));
        let broken = remove_dff_chunk(&dff_bytes, 0x1f);
        assert!(contains_dff_chunk(&broken, 0x0135));
        assert!(!contains_dff_chunk(&broken, 0x1f));
        let dff_img = img_dir.join("dff.img");
        write_img_archive(&dff_img, &[("animated.dff".to_string(), broken)]).unwrap();

        let result = repair_dffs_in_root(&root);

        assert_eq!(result.errors, Vec::<String>::new());
        assert_eq!(result.uv_anim_pipeline_fixed, 1);
        assert!(
            result
                .repaired_entries
                .iter()
                .any(|line| line.contains("animated.dff")
                    && line.contains("added UV animation Right to Render pipeline marker"))
        );
        let entry = parse_img(&dff_img)
            .into_iter()
            .find(|entry| entry.name == "animated.dff")
            .unwrap();
        let mut repaired = read_img_entry(&entry);
        repaired.truncate(dff_chunk_len(&repaired));
        assert!(contains_dff_chunk(&repaired, 0x0135));
        assert!(contains_dff_chunk(&repaired, 0x1f));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn repair_scan_accepts_uv_anim_dictionary_before_clump() {
        let mut raw = test_raw_dff_mesh();
        raw.material_animations = vec![DffMaterialAnim {
            names: vec!["scroll_sign".to_string()],
        }];
        raw.uv_animations.push(DffUvAnimation {
            name: "scroll_sign".to_string(),
            type_id: 0x1c1,
            flags: 0,
            duration: 1.0,
            node_to_uv: [0, 1, 0, 0, 0, 0, 0, 0],
            frames: vec![DffUvAnimFrame {
                time: 0.0,
                uv: [0.0, 1.0, 1.0, 0.0, 0.0, 0.0],
                prev: -1,
            }],
        });
        let dff_bytes = write_normalized_dff(&raw, "animated").unwrap();

        let (needs_normalize, issues) = dff_geometry_issue_summary(&dff_bytes);

        assert!(
            !issues.iter().any(|issue| issue == "not a RenderWare clump"),
            "{issues:?}"
        );
        assert!(!needs_normalize, "{issues:?}");
    }

    #[test]
    fn repair_does_not_log_failed_normalize_as_repaired() {
        let root = std::env::temp_dir().join(format!(
            "eagle_editor_failed_dff_repair_log_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let img_dir = root.join("imgs");
        fs::create_dir_all(&img_dir).unwrap();
        let dff_img = img_dir.join("dff.img");
        let broken = b"not a renderware clump".to_vec();
        write_img_archive(&dff_img, &[("broken.dff".to_string(), broken.clone())]).unwrap();

        let result = repair_dffs_in_root(&root);

        assert_eq!(result.repaired, 0);
        assert_eq!(result.skipped, 1);
        assert!(result.repaired_entries.is_empty());
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("broken.dff"));
        assert!(result.errors[0].contains("invalid RenderWare chunk boundaries"));
        let entry = parse_img(&dff_img)
            .into_iter()
            .find(|entry| entry.name == "broken.dff")
            .unwrap();
        let mut actual = read_img_entry(&entry);
        actual.truncate(broken.len());
        assert_eq!(actual, broken);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn repair_converts_legacy_uv_anim_basis_frames() {
        let root = std::env::temp_dir().join(format!(
            "eagle_editor_uv_anim_legacy_slots_repair_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let img_dir = root.join("imgs");
        fs::create_dir_all(&img_dir).unwrap();
        let mut raw = test_raw_dff_mesh();
        raw.material_animations = vec![DffMaterialAnim {
            names: vec!["atlas_gif".to_string()],
        }];
        raw.uv_animations.push(DffUvAnimation {
            name: "atlas_gif".to_string(),
            type_id: 0x1c1,
            flags: 0,
            duration: 0.2,
            node_to_uv: [0, 1, 0, 0, 0, 0, 0, 0],
            frames: vec![
                DffUvAnimFrame {
                    time: 0.0,
                    uv: [0.5, 0.0, 0.0, 0.5, -0.0, -0.0],
                    prev: -1,
                },
                DffUvAnimFrame {
                    time: 0.1,
                    uv: [0.5, 0.0, 0.0, 0.5, -0.5, -0.0],
                    prev: 0,
                },
            ],
        });
        let dff_img = img_dir.join("dff.img");
        write_img_archive(
            &dff_img,
            &[(
                "animated.dff".to_string(),
                write_normalized_dff(&raw, "animated").unwrap(),
            )],
        )
        .unwrap();

        let result = repair_dffs_in_root(&root);

        assert_eq!(result.errors, Vec::<String>::new());
        assert_eq!(result.uv_anim_legacy_slots_fixed, 1);
        assert!(
            result
                .repaired_entries
                .iter()
                .any(|line| line.contains("animated.dff")
                    && line.contains("converted legacy Eagle UV animation slots"))
        );
        let entry = parse_img(&dff_img)
            .into_iter()
            .find(|entry| entry.name == "animated.dff")
            .unwrap();
        let mut repaired = read_img_entry(&entry);
        repaired.truncate(dff_chunk_len(&repaired));
        let parsed = parse_dff_mesh(&repaired);
        for expected in &raw.uvs {
            assert!(
                parsed
                    .uvs
                    .iter()
                    .any(|uv| (uv.u - expected.u).abs() < 0.0001
                        && (uv.v - expected.v).abs() < 0.0001)
            );
        }
        let animation = parsed
            .uv_animations
            .iter()
            .find(|animation| animation.name == "atlas_gif")
            .unwrap();
        assert_eq!(animation.frames[0].uv, [0.0, 0.5, 0.5, 0.0, 0.0, 0.5]);
        assert_eq!(animation.frames[1].uv, [0.0, 0.5, 0.5, 0.0, 0.5, 0.5]);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn generated_txd_can_create_and_append_to_dedicated_img() {
        let root = temp_resource_root("generated_txd_img");
        let first = root.join("sources").join("roads");
        fs::create_dir_all(&first).unwrap();
        image::RgbaImage::from_pixel(4, 4, image::Rgba([100, 110, 120, 255]))
            .save(first.join("asphalt.png"))
            .unwrap();

        let created = generate_txds_into_project_img(&root, &[first]).unwrap();
        assert_eq!(created, root.join("imgs/txd.img"));
        assert_eq!(
            parse_img(&created)
                .into_iter()
                .map(|entry| entry.name)
                .collect::<Vec<_>>(),
            vec!["roads.txd"]
        );

        let uppercase = root.join("imgs/TXD.IMG");
        fs::rename(&created, &uppercase).unwrap();
        let second = root.join("sources").join("windows");
        fs::create_dir_all(&second).unwrap();
        image::RgbaImage::from_pixel(4, 4, image::Rgba([40, 80, 120, 128]))
            .save(second.join("glass.png"))
            .unwrap();

        let appended = generate_txds_into_project_img(&root, &[second]).unwrap();
        assert_eq!(appended, uppercase);
        let entries = parse_img(&appended);
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            vec!["roads.txd", "windows.txd"]
        );
        let roads = entries
            .iter()
            .find(|entry| entry.name == "roads.txd")
            .unwrap();
        assert!(txd_contains_texture_native(
            &read_img_entry(roads),
            "asphalt"
        ));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn txd_build_container_uses_each_child_folder_as_a_txd_source() {
        let root = temp_resource_root("txd_build_sources");
        let build = root.join("txd_build");
        for (folder, texture) in [("roads", "asphalt"), ("windows", "glass")] {
            let source = build.join(folder);
            fs::create_dir_all(&source).unwrap();
            image::RgbaImage::from_pixel(4, 4, image::Rgba([80, 100, 120, 255]))
                .save(source.join(format!("{texture}.png")))
                .unwrap();
        }

        let sources = txd_source_folders(&build).unwrap();
        let names = sources
            .iter()
            .map(|source| txd_name_from_folder(source).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["roads.txd", "windows.txd"]);
        let output = generate_txds_into_textures(&root, &sources).unwrap();
        for name in names {
            assert!(output.join(name).is_file());
        }

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn loaded_mesh_reference_check_uses_dff_part_of_composite_key() {
        let referenced = BTreeSet::from(["building.dff".to_string()]);

        assert!(loaded_mesh_is_required(
            "building.dff|city.txd",
            &referenced
        ));
        assert!(!loaded_mesh_is_required("unused.dff|city.txd", &referenced));
    }

    #[test]
    fn loaded_mesh_reference_check_preserves_simulation_meshes() {
        let referenced = BTreeSet::new();

        assert!(loaded_mesh_is_required(SIM_PLAYER_DFF, &referenced));
        assert!(loaded_mesh_is_required(SIM_VEHICLE_DFF, &referenced));
    }

    #[test]
    fn asset_optimization_scan_can_run_off_render_thread() {
        let root = temp_resource_root("async_asset_scan");
        let job = TxdCleanupJob {
            root: root.clone(),
            wip_root: wip_root_path(&root),
            gta_sa_dir: root.join("gta"),
            img_files: Vec::new(),
            img_index: 0,
            dff_map: HashMap::new(),
            txd_map: HashMap::new(),
            definitions: Vec::new(),
            definition_index: 0,
            used_by_txd: HashMap::new(),
            dffs_by_txd: HashMap::new(),
            txd_names: Vec::new(),
            txd_index: 0,
            targets: Vec::new(),
            prepared_txds: BTreeMap::new(),
            source_fingerprints: BTreeMap::new(),
            excluded_assets: HashSet::new(),
            deep_optimize: true,
            scope: AssetOptimizationScope::default(),
            profile: TxdOptimizationProfile::lossless(),
            skipped: 0,
            errors: Vec::new(),
            phase: TxdCleanupPhase::DiscoverArchives,
            status: String::new(),
            started_at: Instant::now(),
        };

        let plan = std::thread::spawn(move || job.run_to_completion())
            .join()
            .expect("background TXD scan must not call render-thread-only APIs");

        assert!(plan.targets.is_empty());
        assert!(plan.errors.is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn asset_optimization_apply_runs_off_thread_and_streams_progress() {
        let root = temp_resource_root("async_asset_apply");
        let context = GeometryOptimizationContext {
            root: root.clone(),
            gta_sa_dir: root.join("gta"),
            definitions: Vec::new(),
            loaded_definition_ids: HashSet::new(),
            building_dffs: HashSet::new(),
            opaque_materials_by_dff: BTreeMap::new(),
        };
        let plan = TxdCleanupPlan {
            targets: Vec::new(),
            consolidations: Vec::new(),
            deep_optimize: true,
            scope: AssetOptimizationScope::default(),
            profile: TxdOptimizationProfile::lossless(),
            profile_bytes_saved: 0,
            purge_count: 0,
            purge_bytes: 0,
            format_fix_count: 0,
            format_fixes: Vec::new(),
            texture_rename_count: 0,
            skipped: 0,
            errors: Vec::new(),
            warnings: Vec::new(),
        };
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = cancelled.clone();
        let (progress_tx, progress_rx) = mpsc::channel();

        let result = std::thread::spawn(move || {
            run_asset_optimization(
                context,
                HashSet::new(),
                plan,
                &worker_cancelled,
                &progress_tx,
            )
        })
        .join()
        .expect("asset apply worker must not call render-thread-only APIs")
        .expect("empty asset optimization should complete");

        assert!(result.geometry.errors.is_empty());
        let messages = progress_rx.try_iter().collect::<Vec<_>>();
        assert!(
            messages
                .iter()
                .any(|message| message.starts_with("TXD phase:"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.starts_with("Geometry phase:"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.starts_with("Archive phase:"))
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn txd_cleanup_apply_runs_off_thread_without_geometry_optimization() {
        let root = temp_resource_root("async_txd_cleanup_apply");
        let context = GeometryOptimizationContext {
            root: root.clone(),
            gta_sa_dir: root.join("gta"),
            definitions: Vec::new(),
            loaded_definition_ids: HashSet::new(),
            building_dffs: HashSet::new(),
            opaque_materials_by_dff: BTreeMap::new(),
        };
        let plan = TxdCleanupPlan {
            targets: Vec::new(),
            consolidations: Vec::new(),
            deep_optimize: false,
            scope: AssetOptimizationScope {
                textures: true,
                dffs: false,
                cols: false,
            },
            profile: TxdOptimizationProfile::lossless(),
            profile_bytes_saved: 0,
            purge_count: 0,
            purge_bytes: 0,
            format_fix_count: 0,
            format_fixes: Vec::new(),
            texture_rename_count: 0,
            skipped: 0,
            errors: Vec::new(),
            warnings: Vec::new(),
        };
        let cancelled = AtomicBool::new(false);
        let (progress_tx, progress_rx) = mpsc::channel();

        let result = std::thread::spawn(move || {
            run_asset_optimization(context, HashSet::new(), plan, &cancelled, &progress_tx)
        })
        .join()
        .expect("TXD cleanup worker should not panic")
        .expect("empty TXD cleanup should complete");

        assert_eq!(result.geometry.dffs_scanned, 0);
        assert_eq!(result.geometry.cols_scanned, 0);
        assert!(result.geometry.errors.is_empty());
        assert!(
            progress_rx
                .try_iter()
                .all(|message| !message.starts_with("Geometry phase:"))
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn asset_optimization_scope_can_run_col_only() {
        let root = temp_resource_root("col_only_asset_optimization");
        let raw = test_raw_dff_mesh();
        let dff_bounds = bounds_from_vertices(&raw.vertices);
        let dff = write_normalized_dff(&raw, "scope_model").unwrap();
        let mut mesh = CollisionMesh {
            name: "scope_model".to_string(),
            spheres: Vec::new(),
            boxes: vec![CollisionBox {
                min: V3 {
                    x: dff_bounds.min.x,
                    y: dff_bounds.min.y,
                    z: dff_bounds.min.z,
                },
                max: V3 {
                    x: dff_bounds.max.x,
                    y: dff_bounds.max.y,
                    z: dff_bounds.max.z,
                },
                surface: CollisionSurface {
                    material: 0,
                    flags: 0,
                    brightness: 0,
                    light: 0,
                },
            }],
            vertices: Vec::new(),
            faces: Vec::new(),
            bounds: dff_bounds,
            shadow_vertices: Vec::new(),
            shadow_faces: Vec::new(),
        };
        mesh.bounds = collision_mesh_bounds(&mesh.vertices, &mesh.spheres, &mesh.boxes);
        let template = col_regeneration_template(&[], "scope_model.col");
        let mut col =
            write_col_mesh_from_template_with_bounds(&template, &mesh, Some(dff_bounds)).unwrap();
        set_col_model_names_from_entry(&mut col, "scope_model.col");
        upsert_replacement_assets(
            &wip_root_path(&root),
            &[
                ("scope_model.dff".to_string(), dff),
                ("scope_model.col".to_string(), col),
            ],
        )
        .unwrap();
        let definition = Definition {
            id: "scope_definition".to_string(),
            zone: "test".to_string(),
            attrs: BTreeMap::from([
                ("dff".to_string(), "scope_model".to_string()),
                ("col".to_string(), "scope_model".to_string()),
            ]),
        };
        let plan = TxdCleanupPlan {
            targets: Vec::new(),
            consolidations: Vec::new(),
            deep_optimize: true,
            scope: AssetOptimizationScope {
                textures: false,
                dffs: false,
                cols: true,
            },
            profile: TxdOptimizationProfile::lossless(),
            profile_bytes_saved: 0,
            purge_count: 0,
            purge_bytes: 0,
            format_fix_count: 0,
            format_fixes: Vec::new(),
            texture_rename_count: 0,
            skipped: 0,
            errors: vec!["texture scan result must be ignored".to_string()],
            warnings: vec!["texture warning must be ignored".to_string()],
        };
        let cancelled = AtomicBool::new(false);
        let (progress_tx, _) = mpsc::channel();

        let result = run_asset_optimization(
            bounds_fix_test_context(&root, vec![definition]),
            HashSet::new(),
            plan,
            &cancelled,
            &progress_tx,
        )
        .unwrap();

        assert_eq!(result.geometry.dffs_scanned, 0);
        assert_eq!(result.geometry.cols_scanned, 1);
        assert!(result.txd.errors.is_empty());
        assert!(result.txd.warnings.is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn col_optimization_keeps_collision_geometry_inside_header_bounds() {
        let root = temp_resource_root("optimize_oversized_col_bounds");
        let raw = test_raw_dff_mesh();
        let dff_bounds = bounds_from_vertices(&raw.vertices);
        let dff = write_normalized_dff(&raw, "oversized_road").unwrap();
        let mesh = CollisionMesh {
            name: "oversized_road".to_string(),
            spheres: Vec::new(),
            boxes: vec![CollisionBox {
                min: V3 {
                    x: -8.0,
                    y: -6.0,
                    z: -1.0,
                },
                max: V3 {
                    x: 8.0,
                    y: 6.0,
                    z: 1.0,
                },
                surface: CollisionSurface {
                    material: 0,
                    flags: 0,
                    brightness: 0,
                    light: 0,
                },
            }],
            vertices: Vec::new(),
            faces: Vec::new(),
            bounds: dff_bounds,
            shadow_vertices: Vec::new(),
            shadow_faces: Vec::new(),
        };
        let expected_bounds = union_bounds(
            dff_bounds,
            collision_mesh_bounds(&mesh.vertices, &mesh.spheres, &mesh.boxes),
        );
        let template = col_regeneration_template(&[], "oversized_road.col");
        // Reproduce the bad state emitted by the old optimizer: the collision
        // extends outside the DFF, but the broad-phase header contains only the
        // DFF bounds.
        let mut col =
            write_col_mesh_from_template_with_bounds(&template, &mesh, Some(dff_bounds)).unwrap();
        set_col_model_names_from_entry(&mut col, "oversized_road.col");
        upsert_replacement_assets(
            &wip_root_path(&root),
            &[
                ("oversized_road.dff".to_string(), dff),
                ("oversized_road.col".to_string(), col),
            ],
        )
        .unwrap();
        let definition = Definition {
            id: "oversized_road_definition".to_string(),
            zone: "test".to_string(),
            attrs: BTreeMap::from([
                ("dff".to_string(), "oversized_road".to_string()),
                ("col".to_string(), "oversized_road".to_string()),
            ]),
        };

        let result = apply_geometry_cleanup_filtered(
            &bounds_fix_test_context(&root, vec![definition]),
            &BTreeSet::new(),
            &BTreeSet::from(["oversized_road.col".to_string()]),
            &AtomicBool::new(false),
            &|_| {},
        );

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let (_, repaired) = result
            .staged_assets
            .iter()
            .find(|(name, _)| name == "oversized_road.col")
            .expect("optimizer should repair the undersized broad-phase header");
        assert!(col_header_matches_bounds(repaired, expected_bounds));
        fs::remove_dir_all(root).unwrap();
    }

    fn stage_bounds_only_optimization_pair(
        root: &Path,
        stem: &str,
        noncanonical_empty_offsets: bool,
    ) -> Definition {
        let raw = test_raw_dff_mesh();
        let bounds = bounds_from_vertices(&raw.vertices);
        let dff = write_normalized_dff(&raw, stem).unwrap();
        let mesh = CollisionMesh {
            name: stem.to_string(),
            spheres: Vec::new(),
            boxes: Vec::new(),
            vertices: Vec::new(),
            faces: Vec::new(),
            bounds,
            shadow_vertices: Vec::new(),
            shadow_faces: Vec::new(),
        };
        let col_name = format!("{stem}.col");
        let template = col_regeneration_template(&[], &col_name);
        let mut col =
            write_col_mesh_from_template_with_bounds(&template, &mesh, Some(bounds)).unwrap();
        set_col_model_names_from_entry(&mut col, &col_name);
        if noncanonical_empty_offsets {
            assert!(col.len() >= 104);
            col[96..100].copy_from_slice(&1u32.to_le_bytes());
            col[100..104].copy_from_slice(&1u32.to_le_bytes());
        }
        upsert_replacement_assets(
            &wip_root_path(root),
            &[(format!("{stem}.dff"), dff), (col_name, col)],
        )
        .unwrap();
        Definition {
            id: format!("{stem}_definition"),
            zone: "test".to_string(),
            attrs: BTreeMap::from([
                ("dff".to_string(), stem.to_string()),
                ("col".to_string(), stem.to_string()),
            ]),
        }
    }

    #[test]
    fn col_optimization_accepts_canonical_bounds_only_col() {
        let root = temp_resource_root("optimize_bounds_only_col");
        let definition = stage_bounds_only_optimization_pair(&root, "empty_lod", false);
        let progress = std::cell::RefCell::new(Vec::new());

        let result = apply_geometry_cleanup_filtered(
            &bounds_fix_test_context(&root, vec![definition]),
            &BTreeSet::new(),
            &BTreeSet::from(["empty_lod.col".to_string()]),
            &AtomicBool::new(false),
            &|message| progress.borrow_mut().push(message),
        );

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.cols_scanned, 1);
        assert_eq!(result.cols_repaired, 0);
        assert!(result.staged_assets.is_empty());
        assert!(
            progress
                .borrow()
                .iter()
                .any(|message| message.contains("valid; no editable collision geometry"))
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn col_optimization_canonicalizes_repairable_bounds_only_col() {
        let root = temp_resource_root("repair_bounds_only_col_offsets");
        let definition = stage_bounds_only_optimization_pair(&root, "repairable_lod", true);

        let result = apply_geometry_cleanup_filtered(
            &bounds_fix_test_context(&root, vec![definition]),
            &BTreeSet::new(),
            &BTreeSet::from(["repairable_lod.col".to_string()]),
            &AtomicBool::new(false),
            &|_| {},
        );

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.cols_scanned, 1);
        assert_eq!(result.cols_repaired, 1);
        let (_, repaired) = result
            .staged_assets
            .iter()
            .find(|(name, _)| name == "repairable_lod.col")
            .unwrap();
        assert_eq!(rd32(repaired, 96), 0);
        assert_eq!(rd32(repaired, 100), 0);
        assert!(!col_validation_has_errors(&validate_col_for_game_load(
            "repairable_lod.col",
            repaired
        )));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn col_optimization_still_rejects_invalid_empty_col_with_reason() {
        let root = temp_resource_root("reject_invalid_empty_col");
        let definition = stage_bounds_only_optimization_pair(&root, "invalid_lod", false);
        let entry = replacement_img_entry(&wip_root_path(&root), "invalid_lod.col").unwrap();
        let mut invalid = read_img_entry(&entry);
        let declared_size = invalid.len() as u32 + 256;
        invalid[4..8].copy_from_slice(&declared_size.to_le_bytes());
        upsert_replacement_assets(
            &wip_root_path(&root),
            &[("invalid_lod.col".to_string(), invalid)],
        )
        .unwrap();

        let result = apply_geometry_cleanup_filtered(
            &bounds_fix_test_context(&root, vec![definition]),
            &BTreeSet::new(),
            &BTreeSet::from(["invalid_lod.col".to_string()]),
            &AtomicBool::new(false),
            &|_| {},
        );

        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("failed game-load validation"));
        assert!(result.errors[0].contains("declares"));
        fs::remove_dir_all(root).unwrap();
    }

    fn consolidation_test_txd(textures: &[(&str, [u8; 4])]) -> Vec<u8> {
        let mut children = vec![rw_chunk(
            0x01,
            (textures.len() as u16).to_le_bytes().to_vec(),
        )];
        for (name, color) in textures {
            let rgba = color.repeat(16);
            children.push(texture_native_from_rgba(&rgba, 4, 4, name));
        }
        rw_chunk(0x16, children.concat())
    }

    #[test]
    fn txd_consolidation_is_deterministic_and_rejects_name_conflicts() {
        let exact_a =
            consolidation_test_txd(&[("brick", [10, 20, 30, 255]), ("roof", [40, 50, 60, 255])]);
        let exact_b = exact_a.clone();
        let conflicting =
            consolidation_test_txd(&[("brick", [200, 20, 30, 255]), ("roof", [40, 50, 60, 255])]);
        let prepared = BTreeMap::from([
            (
                "a.txd".to_string(),
                parse_txd_texture_contents(&exact_a).unwrap(),
            ),
            (
                "b.txd".to_string(),
                parse_txd_texture_contents(&exact_b).unwrap(),
            ),
            (
                "conflict.txd".to_string(),
                parse_txd_texture_contents(&conflicting).unwrap(),
            ),
        ]);
        let referenced = prepared.keys().cloned().collect::<HashSet<_>>();
        let mut warnings = Vec::new();

        let fingerprints = prepared
            .keys()
            .map(|name| (name.clone(), [0, 0]))
            .collect::<BTreeMap<_, _>>();
        let plans = plan_txd_consolidations(&prepared, &fingerprints, &referenced, &mut warnings);

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].retained, "a.txd");
        assert_eq!(plans[0].donors, vec!["b.txd"]);
        assert!(plans[0].exact);
        assert_eq!(
            plans[0]
                .source_fingerprints
                .keys()
                .cloned()
                .collect::<Vec<_>>(),
            vec!["a.txd", "b.txd"]
        );
        assert!(warnings.is_empty());
    }

    #[test]
    fn txd_consolidation_preserves_native_alpha_semantics() {
        let rgba = [30, 80, 120, 96].repeat(16);
        let mut opaque_native = texture_native_from_rgba(&rgba, 4, 4, "road");
        let transparent_native = opaque_native.clone();
        // D3D texture-native struct byte 87 is the RenderWare raster flag:
        // 8 is opaque and 9 enables the stored alpha channel.
        opaque_native[24 + 87] = 8;
        let dictionary = |native: Vec<u8>| {
            rw_chunk(
                0x16,
                [rw_chunk(0x01, 1u16.to_le_bytes().to_vec()), native].concat(),
            )
        };
        let prepared = BTreeMap::from([
            (
                "opaque.txd".to_string(),
                parse_txd_texture_contents(&dictionary(opaque_native)).unwrap(),
            ),
            (
                "transparent.txd".to_string(),
                parse_txd_texture_contents(&dictionary(transparent_native)).unwrap(),
            ),
        ]);
        let referenced = prepared.keys().cloned().collect::<HashSet<_>>();
        let mut warnings = Vec::new();

        let fingerprints = prepared
            .keys()
            .map(|name| (name.clone(), [0, 0]))
            .collect::<BTreeMap<_, _>>();
        let plans = plan_txd_consolidations(&prepared, &fingerprints, &referenced, &mut warnings);

        assert!(plans.is_empty());
    }

    #[test]
    fn txd_merge_adds_unique_textures_and_culls_same_image_aliases() {
        let retained = consolidation_test_txd(&[
            ("a", [10, 10, 10, 255]),
            ("b", [20, 20, 20, 255]),
            ("c", [30, 30, 30, 255]),
            ("d", [40, 40, 40, 255]),
        ]);
        let donor = consolidation_test_txd(&[
            ("a", [10, 10, 10, 255]),
            ("b", [20, 20, 20, 255]),
            ("c_alias", [30, 30, 30, 255]),
            ("unique", [90, 90, 90, 255]),
        ]);

        let (merged, renames, added) = merge_txd_bytes(retained, &donor).unwrap();
        let names = parse_txd_texture_contents(&merged)
            .unwrap()
            .into_iter()
            .map(|texture| texture.name)
            .collect::<Vec<_>>();

        assert_eq!(added, 1);
        assert_eq!(renames.get("c_alias").map(String::as_str), Some("c"));
        assert_eq!(names, vec!["a", "b", "c", "d", "unique"]);
    }

    #[test]
    fn txd_scan_keeps_compact_fingerprints_until_merge_is_applied() {
        let txd = consolidation_test_txd(&[("large", [10, 20, 30, 255])]);

        let scanned = parse_txd_texture_contents(&txd).unwrap();
        let applying = parse_txd_texture_contents_with_natives(&txd).unwrap();

        assert!(scanned[0].native.is_empty());
        assert!(!applying[0].native.is_empty());
        assert_eq!(scanned[0].fingerprint, applying[0].fingerprint);
    }

    #[test]
    fn valid_dff_optimization_is_idempotent() {
        let bytes = write_normalized_dff(&test_raw_dff_mesh(), "valid").unwrap();

        let first = optimize_dff_bytes("valid.dff", &bytes, false, &BTreeSet::new()).unwrap();
        let second =
            optimize_dff_bytes("valid.dff", &first.bytes, false, &BTreeSet::new()).unwrap();

        assert_eq!(first.bytes, bytes);
        assert_eq!(second.bytes, first.bytes);
        assert!(first.reasons.is_empty());
        assert!(second.reasons.is_empty());
    }

    #[test]
    fn col_alignment_uses_geometry_and_cleanup_is_repeatable() {
        let dff_vertices = vec![
            V3 {
                x: -1.0,
                y: -2.0,
                z: 0.0,
            },
            V3 {
                x: 1.0,
                y: -2.0,
                z: 0.0,
            },
            V3 {
                x: 1.0,
                y: 2.0,
                z: 0.0,
            },
            V3 {
                x: -1.0,
                y: 2.0,
                z: 0.0,
            },
        ];
        let col_vertices = dff_vertices
            .iter()
            .map(|vertex| V3 {
                x: vertex.y * 3.0 + 100.0,
                y: -vertex.x * 3.0 - 50.0,
                z: vertex.z * 3.0 + 25.0,
            })
            .collect::<Vec<_>>();
        let face = |a, b, c| CollisionFace {
            a,
            b,
            c,
            material: 1,
            light: 2,
            img_path: PathBuf::new(),
            material_file_offset: 0,
            light_file_offset: 0,
        };
        let mut mesh = CollisionMesh {
            name: "aligned".to_string(),
            spheres: Vec::new(),
            boxes: Vec::new(),
            vertices: col_vertices,
            faces: vec![face(0, 1, 2), face(2, 1, 0), face(0, 0, 1)],
            bounds: Bounds {
                min: Vec3::ZERO,
                max: Vec3::ZERO,
            },
            shadow_vertices: Vec::new(),
            shadow_faces: Vec::new(),
        };

        let alignment = detect_col_alignment(&mesh, &dff_vertices).unwrap();
        apply_col_alignment(&mut mesh, alignment);
        let first = clean_collision_mesh(&mut mesh);
        let second = clean_collision_mesh(&mut mesh);

        assert_eq!(first.0, 2);
        assert_eq!(second, (0, 0));
        let bounds = bounds_from_vertices(&mesh.vertices);
        let expected = bounds_from_vertices(&dff_vertices);
        assert!((bounds.min - expected.min).length() < 0.001);
        assert!((bounds.max - expected.max).length() < 0.001);
    }
}

pub(crate) fn remove_replacement_archive_entries(
    root: &Path,
    keys: &HashSet<String>,
) -> Result<usize, String> {
    if keys.is_empty() {
        return Ok(0);
    }
    let img_path = root.join("imgs").join(REPLACEMENT_IMG);
    if !img_path.is_file() {
        return Ok(0);
    }
    let mut entries = Vec::<(String, Vec<u8>)>::new();
    let mut removed = 0usize;
    for entry in parse_img(&img_path) {
        let mut data = read_img_entry(&entry);
        let len = replacement_entry_len(&entry.name, &data);
        data.truncate(len);
        if keys.contains(&lower(&entry.name)) {
            removed += 1;
        } else {
            entries.push((entry.name, data));
        }
    }
    write_img_archive(&img_path, &entries)?;
    Ok(removed)
}

pub(crate) fn rename_replacement_archive_entry(
    root: &Path,
    old_name: &str,
    new_name: &str,
) -> Result<bool, String> {
    let img_path = root.join("imgs").join(REPLACEMENT_IMG);
    if !img_path.is_file() {
        return Ok(false);
    }
    let old_name = normalize_legacy_light_mapper_asset_name(old_name);
    let new_name = normalize_legacy_light_mapper_asset_name(new_name);
    if lower(&old_name) == lower(&new_name) {
        return Ok(false);
    }
    let old_key = lower(&old_name);
    let new_key = lower(&new_name);
    let mut entries = Vec::<(String, Vec<u8>)>::new();
    let mut renamed = false;
    for entry in parse_img(&img_path) {
        let mut data = read_img_entry(&entry);
        let len = replacement_entry_len(&entry.name, &data);
        data.truncate(len);
        let entry_name = normalize_legacy_light_mapper_asset_name(&entry.name);
        let key = lower(&entry_name);
        if key == old_key {
            entries.push((new_name.clone(), data));
            renamed = true;
        } else if key != new_key {
            entries.push((entry_name, data));
        }
    }
    if renamed {
        write_img_archive(&img_path, &entries)?;
    }
    Ok(renamed)
}

pub(crate) fn copy_replacement_assets_to_wip(
    app: &AppState,
    wip_root: &Path,
) -> Result<usize, String> {
    let entries = pending_replacement_entries(app)?;
    if entries.is_empty() {
        return Ok(0);
    }
    let img_dir = wip_root.join("imgs");
    fs::create_dir_all(&img_dir).map_err(|err| format!("{}: {err}", img_dir.display()))?;
    write_img_archive(&img_dir.join(REPLACEMENT_IMG), &entries)?;
    Ok(entries.len())
}

pub(crate) fn replacement_archive_entries(root: &Path) -> Result<Vec<(String, Vec<u8>)>, String> {
    let img_path = root.join("imgs").join(REPLACEMENT_IMG);
    if !img_path.is_file() {
        return Ok(Vec::new());
    }
    let parsed = parse_img(&img_path);
    let mut file =
        fs::File::open(&img_path).map_err(|err| format!("{}: {err}", img_path.display()))?;
    let mut entries = Vec::new();
    for entry in parsed {
        let mut data = read_img_entry_from(&mut file, &entry);
        let len = replacement_entry_len(&entry.name, &data);
        data.truncate(len);
        entries.push((entry.name, data));
    }
    Ok(entries)
}

fn pending_replacement_entries_for_root(root: &Path) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut by_key = BTreeMap::<String, (String, Vec<u8>)>::new();
    let wip_root = wip_root_path(root);
    for root in [root, wip_root.as_path()] {
        for (name, bytes) in replacement_archive_entries(root)? {
            let normalized = normalize_legacy_light_mapper_asset_name(&name);
            by_key.insert(lower(&normalized), (normalized, bytes));
        }
    }
    Ok(by_key.into_values().collect())
}

pub(crate) fn pending_replacement_entries(
    app: &AppState,
) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut by_key = pending_replacement_entries_for_root(&app.root)?
        .into_iter()
        .map(|(name, bytes)| (lower(&name), (name, bytes)))
        .collect::<BTreeMap<_, _>>();
    by_key.extend(app.pending_replacement_assets.clone());
    for (key, bytes) in &app.editing.modified_entries {
        if app.editing.deleted_entries.contains(key) || asset_ext(key).is_none() {
            continue;
        }
        let name = app
            .editing
            .rows
            .iter()
            .find(|row| lower(&row.entry.name) == *key)
            .map(|row| row.entry.name.clone())
            .unwrap_or_else(|| key.clone());
        let normalized = normalize_legacy_light_mapper_asset_name(&name);
        by_key.insert(lower(&normalized), (normalized, bytes.clone()));
    }
    Ok(by_key.into_values().collect())
}

fn pending_replacement_entries_from_parts(
    root: &Path,
    staged: &BTreeMap<String, (String, Vec<u8>)>,
) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut by_key = pending_replacement_entries_for_root(root)?
        .into_iter()
        .map(|(name, bytes)| (lower(&name), (name, bytes)))
        .collect::<BTreeMap<_, _>>();
    by_key.extend(staged.clone());
    Ok(by_key.into_values().collect())
}

fn clear_saved_editing_replacements(app: &mut AppState) {
    app.editing
        .modified_entries
        .retain(|key, _| asset_ext(key).is_none());
    app.editing
        .added_entries
        .retain(|key| asset_ext(key).is_none());
}

pub(crate) fn pending_replacement_asset_count(app: &AppState) -> usize {
    let mut keys = HashSet::new();
    let wip_root = wip_root_path(&app.root);
    for root in [&app.root, &wip_root] {
        let img_path = root.join("imgs").join(REPLACEMENT_IMG);
        for entry in parse_img(&img_path) {
            keys.insert(lower(&normalize_legacy_light_mapper_asset_name(
                &entry.name,
            )));
        }
    }
    keys.extend(
        app.pending_replacement_assets
            .values()
            .map(|(name, _)| lower(&normalize_legacy_light_mapper_asset_name(name))),
    );
    for key in app.editing.modified_entries.keys() {
        if !app.editing.deleted_entries.contains(key) && asset_ext(key).is_some() {
            keys.insert(lower(&normalize_legacy_light_mapper_asset_name(key)));
        }
    }
    keys.len()
}

fn project_img_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_img_files_from_dir(&root.join("imgs"), &mut files);
    files.retain(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_none_or(|name| !name.eq_ignore_ascii_case(REPLACEMENT_IMG))
    });
    files.sort();
    files
}

fn asset_ext(name: &str) -> Option<&'static str> {
    let key = lower(name);
    if key.ends_with(".dff") {
        Some("dff")
    } else if key.ends_with(".col") {
        Some("col")
    } else if key.ends_with(".txd") {
        Some("txd")
    } else {
        None
    }
}

struct ReplacementDestinationIndex {
    loose_by_name: HashMap<String, PathBuf>,
    archive_by_name: HashMap<String, PathBuf>,
    hinted_archive_by_ext: HashMap<&'static str, PathBuf>,
    populated_archive_by_ext: HashMap<&'static str, PathBuf>,
    img_dir: PathBuf,
}

impl ReplacementDestinationIndex {
    fn build(root: &Path) -> Self {
        let mut loose_by_name = HashMap::new();
        for dir in [
            "imgs",
            "Imgs",
            "models",
            "Models",
            "textures",
            "Textures",
            "txd_build",
            "TXD_Build",
        ] {
            let path = root.join(dir);
            if !path.exists() {
                continue;
            }
            for entry in WalkDir::new(path).into_iter().filter_map(Result::ok) {
                if !entry.file_type().is_file() {
                    continue;
                }
                let Some(name) = entry.path().file_name().and_then(|value| value.to_str()) else {
                    continue;
                };
                if name.eq_ignore_ascii_case(REPLACEMENT_IMG) {
                    continue;
                }
                loose_by_name
                    .entry(lower(name))
                    .or_insert_with(|| entry.path().to_path_buf());
            }
        }

        let mut archive_by_name = HashMap::new();
        let mut hinted_archive_by_ext = HashMap::new();
        let mut populated_archive_by_ext = HashMap::new();
        for path in project_img_files(root) {
            let stem = path
                .file_stem()
                .and_then(|value| value.to_str())
                .map(lower)
                .unwrap_or_default();
            for ext in ["dff", "col", "txd"] {
                let hinted = if ext == "txd" {
                    stem.contains("txd") || stem.contains("tex")
                } else {
                    stem.contains(ext)
                };
                if hinted {
                    hinted_archive_by_ext
                        .entry(ext)
                        .or_insert_with(|| path.clone());
                }
            }
            for entry in parse_img(&path) {
                archive_by_name
                    .entry(lower(&entry.name))
                    .or_insert_with(|| path.clone());
                if let Some(ext) = asset_ext(&entry.name) {
                    populated_archive_by_ext
                        .entry(ext)
                        .or_insert_with(|| path.clone());
                }
            }
        }

        Self {
            loose_by_name,
            archive_by_name,
            hinted_archive_by_ext,
            populated_archive_by_ext,
            img_dir: root.join("imgs"),
        }
    }

    fn loose_path(&self, asset_name: &str) -> Option<&Path> {
        self.loose_by_name
            .get(&lower(asset_name))
            .map(PathBuf::as_path)
    }

    fn archive_path(&self, asset_name: &str) -> Result<PathBuf, String> {
        let Some(ext) = asset_ext(asset_name) else {
            return Err(format!(
                "{asset_name}: unsupported replacement asset extension"
            ));
        };
        if let Some(path) = self.archive_by_name.get(&lower(asset_name)) {
            return Ok(path.clone());
        }
        if let Some(path) = self.hinted_archive_by_ext.get(ext) {
            return Ok(path.clone());
        }
        if let Some(path) = self.populated_archive_by_ext.get(ext) {
            return Ok(path.clone());
        }
        Ok(self.img_dir.join(format!("{ext}.img")))
    }
}

fn upsert_img_archive_entries_owned(
    archive_path: &Path,
    replacements: Vec<(String, Vec<u8>)>,
) -> Result<usize, String> {
    let mut entries = Vec::<(String, Vec<u8>)>::new();
    let replacement_keys = replacements
        .iter()
        .map(|(name, _)| lower(name))
        .collect::<HashSet<_>>();
    let replacement_count = replacement_keys.len();
    let mut remaining: BTreeMap<String, (String, Vec<u8>)> = replacements
        .into_iter()
        .map(|(name, bytes)| (lower(&name), (name, bytes)))
        .collect();

    if archive_path.is_file() {
        let parsed = parse_img(archive_path);
        let mut source_file = fs::File::open(archive_path)
            .map_err(|err| format!("{}: {err}", archive_path.display()))?;
        for entry in parsed {
            let key = lower(&entry.name);
            if let Some((name, bytes)) = remaining.remove(&key) {
                entries.push((name, bytes));
            } else {
                let mut data = read_img_entry_from(&mut source_file, &entry);
                let len = replacement_entry_len(&entry.name, &data);
                data.truncate(len);
                entries.push((entry.name, data));
            }
        }
    }

    entries.extend(remaining.into_values());
    for (name, bytes) in &mut entries {
        if asset_ext(name).is_some_and(|ext| ext.eq_ignore_ascii_case("dff")) {
            repair_dff_bounds_spheres(bytes);
        }
    }
    if let Some(parent) = archive_path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("{}: {err}", parent.display()))?;
    }
    write_img_archive(archive_path, &entries)?;
    let mut written_by_key = HashMap::new();
    for entry in parse_img(archive_path) {
        written_by_key.entry(lower(&entry.name)).or_insert(entry);
    }
    let mut expected_by_key = HashMap::new();
    for (name, bytes) in &entries {
        let key = lower(name);
        if replacement_keys.contains(&key) {
            expected_by_key.entry(key).or_insert((name, bytes));
        }
    }
    let mut verification = Vec::with_capacity(expected_by_key.len());
    for (key, (name, expected)) in expected_by_key {
        let Some(entry) = written_by_key.get(&key) else {
            return Err(format!(
                "{}: replacement {name} was not written to IMG",
                archive_path.display()
            ));
        };
        verification.push((entry, name, expected));
    }
    verification.sort_by_key(|(entry, _, _)| entry.offset);
    let mut written_file =
        fs::File::open(archive_path).map_err(|err| format!("{}: {err}", archive_path.display()))?;
    for (entry, name, expected) in verification {
        let expected_len = replacement_entry_len(name, expected);
        let expected = &expected[..expected_len.min(expected.len())];
        let mut actual = read_img_entry_from(&mut written_file, entry);
        let actual_len = replacement_entry_len(&entry.name, &actual);
        actual.truncate(actual_len.min(actual.len()));
        if actual != expected {
            return Err(format!(
                "{}: replacement {name} did not verify after IMG write (expected {} bytes, found {} bytes)",
                archive_path.display(),
                expected.len(),
                actual.len()
            ));
        }
    }
    Ok(replacement_count)
}

fn dff_geometry_issue_summary(bytes: &[u8]) -> (bool, Vec<String>) {
    fn scan_geometry_extension(
        bytes: &[u8],
        start: usize,
        end: usize,
        vertex_count: Option<usize>,
        issues: &mut Vec<String>,
    ) {
        let mut o = start;
        let mut has_bin_mesh = false;
        while o + 12 <= end {
            let id = rd32(bytes, o);
            let size = rd32(bytes, o + 4) as usize;
            let cs = o + 12;
            let ce = cs.saturating_add(size);
            if ce > end || ce > bytes.len() {
                issues.push("invalid geometry extension chunk".to_string());
                break;
            }
            if id == 0x050e {
                has_bin_mesh = true;
                if size < 12 {
                    issues.push("short bin mesh plugin".to_string());
                }
            } else if id == 0x0253_f2f8 {
                if size < 4 {
                    issues.push("short 2DFX plugin".to_string());
                } else {
                    let count = rd32(bytes, cs) as usize;
                    let mut effect = cs + 4;
                    for _ in 0..count {
                        if effect + 20 > ce {
                            issues.push("truncated 2DFX record".to_string());
                            break;
                        }
                        let payload_size = rd32(bytes, effect + 16) as usize;
                        effect = effect.saturating_add(20).saturating_add(payload_size);
                        if effect > ce {
                            issues.push("truncated 2DFX payload".to_string());
                            break;
                        }
                    }
                    if effect < ce {
                        issues.push("unexpected trailing 2DFX data".to_string());
                    }
                }
            } else if id == 0x0253_f2f9 {
                if size < 4 {
                    issues.push("short night prelight plugin".to_string());
                } else if rd32(bytes, cs) != 0
                    && vertex_count.is_some_and(|count| {
                        count
                            .checked_mul(4)
                            .and_then(|colors| colors.checked_add(4))
                            .is_none_or(|required| required > size)
                    })
                {
                    issues.push("truncated night prelight stream".to_string());
                }
            } else if id == BREAKABLE_PLUGIN_ID && parse_breakable_plugin(&bytes[cs..ce]).is_err() {
                issues.push("invalid breakable geometry plugin".to_string());
            }
            // Unknown RenderWare/GTA extension chunks are valid opaque data.
            // The repair pass must preserve them, not call them corruption and
            // trigger a normalized rewrite that silently drops their payloads.
            o = ce;
        }
        if o != end {
            issues.push("invalid geometry extension padding".to_string());
        }
        if !has_bin_mesh {
            issues.push("missing bin mesh plugin".to_string());
        }
    }

    fn scan_geometry(bytes: &[u8], start: usize, end: usize, issues: &mut Vec<String>) {
        let mut o = start;
        let mut saw_struct = false;
        let mut vertex_count = None;
        while o + 12 <= end {
            let id = rd32(bytes, o);
            let size = rd32(bytes, o + 4) as usize;
            let cs = o + 12;
            let ce = cs.saturating_add(size);
            if ce > end || ce > bytes.len() {
                issues.push("invalid geometry chunk".to_string());
                break;
            }
            if id == 0x01 {
                saw_struct = true;
                if size < 16 {
                    issues.push("short geometry struct".to_string());
                } else {
                    let flags = rd32(bytes, cs);
                    let tri_count = rd32(bytes, cs + 4);
                    let vert_count = rd32(bytes, cs + 8);
                    vertex_count = Some(vert_count as usize);
                    if tri_count == 0 || vert_count == 0 {
                        issues.push("empty geometry".to_string());
                    }
                    let _ = flags;
                }
            } else if id == 0x03 {
                scan_geometry_extension(bytes, cs, ce, vertex_count, issues);
            }
            o = ce;
        }
        if !saw_struct {
            issues.push("missing geometry struct".to_string());
        }
    }

    fn scan(bytes: &[u8], start: usize, end: usize, issues: &mut Vec<String>) -> (usize, usize) {
        let mut geometry_count = 0usize;
        let mut clump_count = 0usize;
        let mut o = start;
        while o + 12 <= end {
            let id = rd32(bytes, o);
            let size = rd32(bytes, o + 4) as usize;
            let cs = o + 12;
            let ce = cs.saturating_add(size);
            if ce > end || ce > bytes.len() {
                issues.push(format!("invalid chunk 0x{id:08x}"));
                break;
            }
            if id == 0x10 {
                clump_count += 1;
            }
            if id == 0x0f {
                geometry_count += 1;
                scan_geometry(bytes, cs, ce, issues);
            }
            if matches!(id, 0x10 | 0x0e | 0x1a | 0x0f | 0x08 | 0x07 | 0x06 | 0x03) {
                let (child_geometry_count, child_clump_count) = scan(bytes, cs, ce, issues);
                geometry_count += child_geometry_count;
                clump_count += child_clump_count;
            }
            o = ce;
        }
        (geometry_count, clump_count)
    }

    let len = dff_chunk_len(bytes);
    let mut issues = Vec::new();
    let (geometry_count, clump_count) = scan(bytes, 0, len, &mut issues);
    if clump_count == 0 {
        issues.push("not a RenderWare clump".to_string());
    }
    if geometry_count == 0 {
        issues.push("no geometry chunks".to_string());
    }
    issues.sort();
    issues.dedup();
    (!issues.is_empty(), issues)
}

/// Returns source data that the normalized writer cannot represent exactly.
/// Structural repair may only rebuild a DFF when this list is empty; targeted
/// repairs (bounds, texture strings, prelighting, UV chunk order) do not use
/// this gate because they preserve unrelated chunks byte-for-byte.
fn dff_normalized_rewrite_blockers(bytes: &[u8]) -> Vec<String> {
    fn plugin_supported(bytes: &[u8], parent: u32, id: u32, start: usize, end: usize) -> bool {
        match parent {
            // Geometry plug-ins represented by RawMesh and the normalized writer.
            0x0f => match id {
                0x050e | 0x1f | 0x0253_f2f8 | 0x0253_f2f9 => true,
                BREAKABLE_PLUGIN_ID => parse_breakable_plugin(&bytes[start..end]).is_ok(),
                _ => false,
            },
            // A single safe frame can retain its GTA frame name.
            0x0e => id == 0x0253_f2fe,
            // The writer represents UV animation material markers, but not
            // general MatFX/environment/specular/reflection material payloads.
            0x07 => {
                id == 0x0135
                    || (id == 0x0120
                        && end - start == 12
                        && rd32(bytes, start) == 5
                        && rd32(bytes, start + 4) == 5
                        && rd32(bytes, start + 8) == 0)
            }
            // Atomic MatFX enable marker emitted for UV-animated materials.
            0x14 => id == 0x0120 && end - start == 4 && rd32(bytes, start) == 1,
            _ => false,
        }
    }

    fn parent_label(id: u32) -> &'static str {
        match id {
            0x10 => "clump",
            0x0e => "frame",
            0x0f => "geometry",
            0x07 => "material",
            0x06 => "texture",
            0x14 => "atomic",
            _ => "RenderWare",
        }
    }

    fn scan_extension(
        bytes: &[u8],
        start: usize,
        end: usize,
        parent: u32,
        blockers: &mut Vec<String>,
    ) {
        let mut o = start;
        while o + 12 <= end {
            let id = rd32(bytes, o);
            let size = rd32(bytes, o + 4) as usize;
            let cs = o + 12;
            let ce = cs.saturating_add(size);
            if ce > end || ce > bytes.len() {
                blockers.push(format!(
                    "invalid {} extension 0x{id:08x}",
                    parent_label(parent)
                ));
                return;
            }
            if !plugin_supported(bytes, parent, id, cs, ce) {
                blockers.push(format!("{} extension 0x{id:08x}", parent_label(parent)));
            }
            o = ce;
        }
        if o != end {
            blockers.push(format!(
                "invalid {} extension padding",
                parent_label(parent)
            ));
        }
    }

    fn scan_range(bytes: &[u8], start: usize, end: usize, parent: u32, blockers: &mut Vec<String>) {
        let mut o = start;
        while o + 12 <= end {
            let id = rd32(bytes, o);
            let size = rd32(bytes, o + 4) as usize;
            let cs = o + 12;
            let ce = cs.saturating_add(size);
            if ce > end || ce > bytes.len() {
                blockers.push(format!("invalid RenderWare chunk 0x{id:08x}"));
                return;
            }
            if id == 0x03 {
                scan_extension(bytes, cs, ce, parent, blockers);
            } else if matches!(id, 0x10 | 0x0e | 0x1a | 0x0f | 0x08 | 0x07 | 0x06 | 0x14) {
                scan_range(bytes, cs, ce, id, blockers);
            }
            o = ce;
        }
        if o != end {
            blockers.push("invalid RenderWare chunk padding".to_string());
        }
    }

    let len = dff_chunk_len(bytes);
    let mut blockers = Vec::new();
    let mut top = 0usize;
    while top + 12 <= len {
        let id = rd32(bytes, top);
        let size = rd32(bytes, top + 4) as usize;
        let end = top.saturating_add(12).saturating_add(size);
        if end > len || end > bytes.len() {
            blockers.push(format!("invalid top-level RenderWare chunk 0x{id:08x}"));
            break;
        }
        if id == 0x10 {
            scan_range(bytes, top + 12, end, id, &mut blockers);
        } else if id != 0x2b {
            blockers.push(format!("top-level chunk 0x{id:08x}"));
        }
        top = end;
    }
    if top != len {
        blockers.push("invalid top-level RenderWare padding".to_string());
    }
    blockers.sort();
    blockers.dedup();
    blockers
}

fn dff_has_geometry_normals(bytes: &[u8]) -> bool {
    fn scan(bytes: &[u8], start: usize, end: usize) -> bool {
        let mut o = start;
        while o + 12 <= end {
            let id = rd32(bytes, o);
            let size = rd32(bytes, o + 4) as usize;
            let cs = o + 12;
            let ce = cs.saturating_add(size);
            if ce > end || ce > bytes.len() {
                return false;
            }
            if id == 0x0f {
                let mut child = cs;
                while child + 12 <= ce {
                    let child_id = rd32(bytes, child);
                    let child_size = rd32(bytes, child + 4) as usize;
                    let child_start = child + 12;
                    let child_end = child_start.saturating_add(child_size);
                    if child_end > ce || child_end > bytes.len() {
                        return false;
                    }
                    if child_id == 0x01 && child_size >= 16 && rd32(bytes, child_start) & 0x10 != 0
                    {
                        return true;
                    }
                    child = child_end;
                }
            } else if matches!(id, 0x10 | 0x0e | 0x1a) && scan(bytes, cs, ce) {
                return true;
            }
            o = ce;
        }
        false
    }
    scan(bytes, 0, dff_chunk_len(bytes))
}

fn normalize_dff_entry(name: &str, bytes: &[u8], include_normals: bool) -> Result<Vec<u8>, String> {
    let raw = parse_dff_mesh_preserving_topology(bytes);
    if raw.vertices.is_empty() || raw.triangles.is_empty() {
        return Err("DFF has no readable geometry".to_string());
    }
    let frame_name = Path::new(name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("dff_repair");
    if !raw_mesh_is_safe_for_normalized_rewrite(&raw, frame_name) {
        return Err(
            "normalized rewrite would flatten a meaningful frame/component hierarchy".to_string(),
        );
    }
    let blockers = dff_normalized_rewrite_blockers(bytes);
    if !blockers.is_empty() {
        return Err(format!(
            "normalized rewrite would discard VC/RenderWare data: {}",
            blockers.join(", ")
        ));
    }
    let candidate = write_normalized_dff_with_options(
        &raw,
        frame_name,
        DffWriteOptions {
            include_normals,
            include_bin_mesh: true,
        },
    )?;
    let reparsed = parse_dff_mesh_preserving_topology(&candidate);
    let mut mismatches = Vec::new();
    macro_rules! compare_field {
        ($field:ident) => {
            if raw.$field != reparsed.$field {
                mismatches.push(stringify!($field));
            }
        };
    }
    compare_field!(vertices);
    compare_field!(normals);
    compare_field!(uvs);
    compare_field!(secondary_uvs);
    compare_field!(prelit_colors);
    compare_field!(prelit_alphas);
    compare_field!(night_prelit_colors);
    compare_field!(night_prelit_alphas);
    compare_field!(light_flags);
    let triangles_match = raw.triangles.len() == reparsed.triangles.len()
        && raw
            .triangles
            .iter()
            .zip(&reparsed.triangles)
            .all(|(before, after)| {
                before.material == after.material
                    && ((before.a == after.a && before.b == after.b && before.c == after.c)
                        || (before.a == after.b && before.b == after.c && before.c == after.a)
                        || (before.a == after.c && before.b == after.a && before.c == after.b))
            });
    if !triangles_match {
        mismatches.push("triangles");
    }
    compare_field!(material_textures);
    compare_field!(materials);
    compare_field!(material_animations);
    compare_field!(uv_animations);
    compare_field!(effects_2dfx);
    compare_field!(components);
    compare_field!(frames);
    if !mismatches.is_empty() {
        return Err(format!(
            "normalized rewrite failed semantic round-trip validation ({})",
            mismatches.join(", ")
        ));
    }
    Ok(candidate)
}

fn uv_anim_legacy_basis_scale(animation: &DffUvAnimation) -> Option<(f32, f32)> {
    let first = animation.frames.first()?;
    let sx = first.uv[0];
    let sy = first.uv[3];
    if !sx.is_finite() || !sy.is_finite() || sx <= 0.0001 || sy <= 0.0001 {
        return None;
    }
    for frame in &animation.frames {
        if (frame.uv[0] - sx).abs() > 0.0001
            || frame.uv[1].abs() > 0.0001
            || frame.uv[2].abs() > 0.0001
            || (frame.uv[3] - sy).abs() > 0.0001
        {
            return None;
        }
    }
    Some((sx, sy))
}

fn repair_dff_uv_anim_legacy_basis(
    name: &str,
    bytes: &[u8],
    include_normals: bool,
) -> Result<Option<Vec<u8>>, String> {
    let mut raw = parse_dff_mesh_preserving_topology(bytes);
    if raw.vertices.is_empty() || raw.triangles.is_empty() {
        return Ok(None);
    }
    let mut animation_scales = HashMap::<String, (f32, f32)>::new();
    for animation in &raw.uv_animations {
        if let Some(scale) = uv_anim_legacy_basis_scale(animation) {
            animation_scales.insert(lower(&animation.name), scale);
        }
    }
    if animation_scales.is_empty() {
        return Ok(None);
    }

    for animation in &mut raw.uv_animations {
        if let Some((sx, sy)) = animation_scales.get(&lower(&animation.name)).copied() {
            for frame in &mut animation.frames {
                frame.uv[0] = 0.0;
                frame.uv[1] = sx;
                frame.uv[2] = sy;
                frame.uv[3] = 0.0;
                frame.uv[4] = -frame.uv[4];
                frame.uv[5] = 1.0 - (-frame.uv[5] + sy);
            }
        }
    }

    let frame_name = Path::new(name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("dff_repair");
    if !raw_mesh_is_safe_for_normalized_rewrite(&raw, frame_name) {
        return Err(
            "UV animation conversion would flatten a meaningful frame/component hierarchy"
                .to_string(),
        );
    }
    let blockers = dff_normalized_rewrite_blockers(bytes);
    if !blockers.is_empty() {
        return Err(format!(
            "UV animation conversion would discard VC/RenderWare data: {}",
            blockers.join(", ")
        ));
    }
    write_normalized_dff_with_options(
        &raw,
        frame_name,
        DffWriteOptions {
            include_normals,
            include_bin_mesh: true,
        },
    )
    .map(Some)
}

struct DffOptimizeOutput {
    bytes: Vec<u8>,
    reasons: Vec<String>,
    warnings: Vec<String>,
    compaction: DffLosslessCompactionStats,
}

fn optimize_dff_bytes(
    name: &str,
    source: &[u8],
    _is_building: bool,
    opaque_materials: &BTreeSet<u16>,
) -> Result<DffOptimizeOutput, String> {
    let mut bytes = source[..dff_chunk_len(source).min(source.len())].to_vec();
    let original = bytes.clone();
    let raw = parse_dff_mesh(&bytes);
    if raw.vertices.is_empty() || raw.triangles.is_empty() {
        return Err("DFF has no readable geometry".to_string());
    }
    let frame_name = Path::new(name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("dff_optimized");
    let include_normals = dff_has_geometry_normals(&bytes);
    let mut compacted = raw.clone();
    // Triangle/vertex remapping would invalidate the separate debris mesh and
    // its editor face mapping. Preserve authored fracture geometry exactly;
    // users can regenerate zones explicitly after topology edits.
    let compaction = if raw
        .components
        .iter()
        .any(|component| component.breakable.is_some())
    {
        DffLosslessCompactionStats::default()
    } else {
        compact_raw_mesh_lossless(&mut compacted, opaque_materials)
    };
    let compaction_changed = compaction.vertices_removed != 0
        || compaction.invalid_triangles_removed != 0
        || compaction.degenerate_triangles_removed != 0
        || compaction.duplicate_triangles_removed != 0
        || compaction.materials_removed != 0
        || compaction.triangles_reordered != 0;
    let mut warnings = Vec::new();
    let mut reasons = Vec::new();
    let mut applied_compaction = DffLosslessCompactionStats::default();
    if compaction_changed {
        if compacted.vertices.is_empty() || compacted.triangles.is_empty() {
            return Err("DFF compaction removed all usable geometry".to_string());
        }
        if raw_mesh_is_safe_for_normalized_rewrite(&raw, frame_name) {
            let candidate = write_normalized_dff_with_options(
                &compacted,
                frame_name,
                DffWriteOptions {
                    include_normals,
                    include_bin_mesh: true,
                },
            )?;
            let reparsed = parse_dff_mesh(&candidate);
            if reparsed.vertices.len() != compacted.vertices.len()
                || reparsed.triangles.len() != compacted.triangles.len()
                || reparsed.material_textures != compacted.material_textures
                || reparsed.materials != compacted.materials
                || reparsed.material_animations != compacted.material_animations
                || reparsed.uv_animations != compacted.uv_animations
                || reparsed.effects_2dfx != compacted.effects_2dfx
            {
                return Err(
                    "DFF compaction candidate failed semantic round-trip validation".to_string(),
                );
            }
            bytes = candidate;
            applied_compaction = compaction;
            reasons.push(format!(
                "lossless compaction removed {} vertex/vertices, {} triangle(s), and {} material slot(s); reordered {} opaque triangle position(s)",
                compaction.vertices_removed,
                compaction.invalid_triangles_removed
                    + compaction.degenerate_triangles_removed
                    + compaction.duplicate_triangles_removed,
                compaction.materials_removed,
                compaction.triangles_reordered
            ));
        } else {
            warnings.push(
                "lossless topology compaction skipped because the DFF has a meaningful multi-component/frame hierarchy"
                    .to_string(),
            );
        }
    }
    let bounds_vertices = if applied_compaction == DffLosslessCompactionStats::default() {
        raw.vertices
            .iter()
            .copied()
            .filter(|vertex| vertex.x.is_finite() && vertex.y.is_finite() && vertex.z.is_finite())
            .collect::<Vec<_>>()
    } else {
        compacted.vertices.clone()
    };
    if bounds_vertices.is_empty() {
        return Err("DFF has no finite geometry bounds".to_string());
    }
    let bounds = bounds_from_vertices(&bounds_vertices);
    let size = bounds.max - bounds.min;
    if size.max_element() > 100_000.0 {
        warnings.push(format!(
            "model bounds are unusually large ({:.1} x {:.1} x {:.1}); preserved for review",
            size.x, size.y, size.z
        ));
    }
    if raw.vertices.len() > 1_000_000 || raw.triangles.len() > 1_000_000 {
        warnings.push(format!(
            "model is unusually dense ({} vertices, {} triangles); preserved for review",
            raw.vertices.len(),
            raw.triangles.len()
        ));
    }

    if let Some(reordered) = repair_dff_uv_anim_dictionary_order(&bytes) {
        bytes = reordered;
        reasons.push("moved UV animation dictionary before clump".to_string());
    }
    if let Some(fixed) = repair_dff_uv_anim_right_to_render(&bytes) {
        bytes = fixed;
        reasons.push("added UV animation Right to Render pipeline marker".to_string());
    }
    let repaired_bounds = repair_dff_bounds_spheres(&mut bytes);
    if repaired_bounds != 0 {
        reasons.push(format!(
            "repaired {repaired_bounds} geometry bounds sphere(s)"
        ));
    }
    let (needs_normalize, issues) = dff_geometry_issue_summary(&bytes);
    // Normals affect visible lighting. Deep optimization preserves them even
    // for building-tagged models; the separate explicit DFF repair command can
    // still apply the legacy building-normal policy when requested.
    let include_normals = dff_has_geometry_normals(&bytes);
    if let Some(fixed) = repair_dff_missing_prelight_streams(&bytes)? {
        bytes = fixed;
        reasons.push("filled missing day/night prelight stream".to_string());
    }
    if let Some(fixed) = repair_dff_uv_anim_legacy_basis(name, &bytes, include_normals)? {
        bytes = fixed;
        reasons.push("converted legacy Eagle UV animation slots to DragonFF layout".to_string());
    }
    if needs_normalize {
        bytes = normalize_dff_entry(name, &bytes, include_normals)?;
        if needs_normalize {
            reasons.push(if issues.is_empty() {
                "normalized malformed DFF geometry".to_string()
            } else {
                format!("normalized DFF geometry ({})", issues.join(", "))
            });
        }
    }
    let (invalid, remaining) = dff_geometry_issue_summary(&bytes);
    if invalid {
        return Err(format!(
            "DFF remains structurally invalid after repair: {}",
            remaining.join(", ")
        ));
    }
    if bytes == original {
        reasons.clear();
    }
    Ok(DffOptimizeOutput {
        bytes,
        reasons,
        warnings,
        compaction: applied_compaction,
    })
}

#[derive(Default)]
struct DffTextureSanitizeContext {
    scopes_by_dff: HashMap<String, BTreeSet<String>>,
    textures_by_txd: HashMap<String, HashSet<String>>,
}

fn dff_texture_sanitize_context(app: &AppState) -> DffTextureSanitizeContext {
    let mut context = DffTextureSanitizeContext::default();
    for definition in app.definitions.values() {
        let Some(txd) = definition_txd_name_from_attrs(definition) else {
            continue;
        };
        let dff = asset_key_opt(definition.attrs.get("dff"), &definition.id, ".dff");
        context
            .scopes_by_dff
            .entry(dff)
            .or_default()
            .insert(asset_key(txd, ".txd"));
    }
    for (texture_name, entries) in &app.txd_textures {
        for entry in entries {
            context
                .textures_by_txd
                .entry(asset_key(&entry.txd_name, ".txd"))
                .or_default()
                .insert(lower(texture_name));
        }
    }
    context
}

fn sanitize_dff_texture_references(
    dff_name: &str,
    bytes: &[u8],
    context: &DffTextureSanitizeContext,
) -> Result<Option<(Vec<u8>, usize, usize, Vec<String>)>, String> {
    let Some(txd_scopes) = context.scopes_by_dff.get(&asset_key(dff_name, ".dff")) else {
        return Ok(None);
    };
    if txd_scopes.is_empty() {
        return Ok(None);
    }
    let raw = parse_dff_mesh(bytes);
    if raw.vertices.is_empty() || raw.triangles.is_empty() {
        return Ok(None);
    }
    let source_names = raw.material_textures.clone();

    let mut renames = HashMap::<String, String>::new();
    for source_name in source_names {
        let source = lower(source_name.trim());
        if source.is_empty() || renames.contains_key(&source) {
            continue;
        }
        let sanitized = sanitize_texture_name(&source);
        if sanitized == source {
            continue;
        }
        // Only repair an unambiguous missing-name mismatch. Every TXD scope
        // used by this DFF must contain the sanitized name and omit the legacy
        // name, otherwise a global DFF rewrite could change or break another
        // definition that shares the model.
        let unambiguous = txd_scopes.iter().all(|txd| {
            context.textures_by_txd.get(txd).is_some_and(|textures| {
                !textures.contains(&source) && textures.contains(&sanitized)
            })
        });
        if unambiguous {
            renames.insert(source, sanitized);
        }
    }
    if renames.is_empty() {
        return Ok(None);
    }

    let (updated, references) = rewrite_dff_material_textures(bytes, &renames)?;
    if references == 0 {
        return Err(
            "sanitized texture names were detected but no DFF references were rewritten"
                .to_string(),
        );
    }
    let reparsed = parse_dff_mesh(&updated);
    for (source, sanitized) in &renames {
        if reparsed
            .material_textures
            .iter()
            .any(|name| lower(name.trim()) == *source)
            || !reparsed
                .material_textures
                .iter()
                .any(|name| lower(name.trim()) == *sanitized)
        {
            return Err(format!(
                "texture reference '{source}' -> '{sanitized}' failed semantic verification"
            ));
        }
    }
    let mut descriptions = renames
        .iter()
        .map(|(source, sanitized)| format!("{source} -> {sanitized}"))
        .collect::<Vec<_>>();
    descriptions.sort();
    Ok(Some((updated, renames.len(), references, descriptions)))
}

fn repair_dff_img_archive(
    path: &Path,
    _building_dffs: &HashSet<String>,
    texture_context: &DffTextureSanitizeContext,
    repair_scope: DffRepairScope,
    result: &mut DffRepairResult,
) -> Result<(), String> {
    let img_entries = parse_img(path);
    if img_entries.is_empty() {
        return Ok(());
    }
    let mut changed = false;
    let mut packed = Vec::<(String, Vec<u8>)>::new();
    let mut written_entries = Vec::<(String, Vec<u8>)>::new();
    for entry in img_entries {
        let mut bytes = read_img_entry(&entry);
        let len = replacement_entry_len(&entry.name, &bytes);
        bytes.truncate(len.min(bytes.len()));
        let original_bytes = bytes.clone();
        if asset_ext(&entry.name).is_some_and(|ext| ext.eq_ignore_ascii_case("dff")) {
            result.scanned += 1;
            let mut prelighting_fixed = false;
            let mut uv_reordered = false;
            let mut uv_pipeline_fixed = false;
            let mut uv_legacy_slots_fixed = false;
            let mut normalized_entry = false;
            let mut reasons = Vec::<String>::new();
            let mut bounds_fixed = 0usize;
            if repair_scope.prelighting {
                // Run this independently before broader normalization. Otherwise a
                // structural rewrite can synthesize the absent stream first and hide
                // which DFF actually needed the dedicated prelighting repair.
                match repair_dff_missing_prelight_streams(&bytes) {
                    Ok(Some(fixed)) => {
                        bytes = fixed;
                        prelighting_fixed = true;
                        reasons.push("filled missing day/night prelight stream".to_string());
                    }
                    Ok(None) => {}
                    Err(err) => {
                        result.skipped += 1;
                        result
                            .errors
                            .push(format!("{}:{}: {}", path.display(), entry.name, err));
                        packed.push((entry.name, original_bytes));
                        continue;
                    }
                }
            }
            if repair_scope.dff_issues {
                if let Some(reordered) = repair_dff_uv_anim_dictionary_order(&bytes) {
                    bytes = reordered;
                    uv_reordered = true;
                    reasons.push("moved UV animation dictionary before clump".to_string());
                }
                if let Some(fixed) = repair_dff_uv_anim_right_to_render(&bytes) {
                    bytes = fixed;
                    uv_pipeline_fixed = true;
                    reasons.push("added UV animation Right to Render pipeline marker".to_string());
                }
                bounds_fixed = repair_dff_bounds_spheres(&mut bytes);
                if bounds_fixed != 0 {
                    reasons.push(format!("repaired {bounds_fixed} geometry bounds sphere(s)"));
                }
                let (needs_normalize, issues) = dff_geometry_issue_summary(&bytes);
                // Authored normals are valid RenderWare data in Vice City and
                // affect runtime lighting. Structural validation must not
                // remove them merely because an IDE entry is a building.
                let include_normals = dff_has_geometry_normals(&bytes);
                match repair_dff_uv_anim_legacy_basis(&entry.name, &bytes, include_normals) {
                    Ok(Some(fixed)) => {
                        bytes = fixed;
                        uv_legacy_slots_fixed = true;
                        normalized_entry = true;
                        reasons.push(
                            "converted legacy Eagle UV animation slots to DragonFF layout"
                                .to_string(),
                        );
                    }
                    Ok(None) => {}
                    Err(err) => {
                        result.skipped += 1;
                        result
                            .errors
                            .push(format!("{}:{}: {}", path.display(), entry.name, err));
                        packed.push((entry.name, original_bytes));
                        continue;
                    }
                }
                if needs_normalize {
                    match normalize_dff_entry(&entry.name, &bytes, include_normals) {
                        Ok(normalized) => {
                            bytes = normalized;
                            normalized_entry = true;
                            if needs_normalize && !issues.is_empty() {
                                reasons.push(format!(
                                    "normalized DFF geometry ({})",
                                    issues.join(", ")
                                ));
                            } else if needs_normalize {
                                reasons.push("normalized DFF geometry".to_string());
                            }
                        }
                        Err(err) => {
                            result.skipped += 1;
                            result.errors.push(format!(
                                "{}:{}: {} ({})",
                                path.display(),
                                entry.name,
                                err,
                                issues.join(", ")
                            ));
                            bytes = original_bytes;
                            packed.push((entry.name, bytes));
                            continue;
                        }
                    }
                }
            }
            if repair_scope.texture_names {
                match sanitize_dff_texture_references(&entry.name, &bytes, texture_context) {
                    Ok(Some((updated, names, references, renames))) => {
                        bytes = updated;
                        result.sanitized_texture_names += names;
                        result.sanitized_texture_references += references;
                        reasons.push(format!(
                            "sanitized {references} texture reference(s) ({})",
                            renames.join(", ")
                        ));
                    }
                    Ok(None) => {}
                    Err(err) => {
                        result.skipped += 1;
                        result.errors.push(format!(
                            "{}:{}: texture-name repair failed: {err}",
                            path.display(),
                            entry.name
                        ));
                        bytes = original_bytes;
                        packed.push((entry.name, bytes));
                        continue;
                    }
                }
            }
            if bytes != original_bytes {
                result.prelighting_fixed += usize::from(prelighting_fixed);
                result.uv_anim_reordered += usize::from(uv_reordered);
                result.uv_anim_pipeline_fixed += usize::from(uv_pipeline_fixed);
                result.uv_anim_legacy_slots_fixed += usize::from(uv_legacy_slots_fixed);
                result.normalized += usize::from(normalized_entry);
                result.bounds_fixed += bounds_fixed;
                result.repaired += 1;
                changed = true;
                result
                    .repaired_dff_names
                    .push(asset_key(&entry.name, ".dff"));
                result.repaired_entries.push(format!(
                    "{}:{} - {}",
                    path.display(),
                    entry.name,
                    reasons.join("; ")
                ));
                written_entries.push((entry.name.clone(), bytes.clone()));
            } else if !reasons.is_empty() {
                bytes = original_bytes;
            }
        }
        packed.push((entry.name, bytes));
    }
    if changed {
        write_img_archive(path, &packed)?;
        let verified_entries = parse_img(path);
        for (name, expected) in written_entries {
            let Some(entry) = verified_entries
                .iter()
                .find(|entry| entry.name.eq_ignore_ascii_case(&name))
            else {
                return Err(format!(
                    "{}:{name}: repair was not written to IMG",
                    path.display()
                ));
            };
            let mut actual = read_img_entry(entry);
            let expected_len = replacement_entry_len(&name, &expected);
            let actual_len = replacement_entry_len(&entry.name, &actual);
            let expected = &expected[..expected_len.min(expected.len())];
            actual.truncate(actual_len.min(actual.len()));
            if actual != expected {
                return Err(format!(
                    "{}:{name}: repair did not verify after IMG write (expected {} bytes, found {} bytes)",
                    path.display(),
                    expected.len(),
                    actual.len()
                ));
            }
        }
    }
    Ok(())
}

#[allow(dead_code)]
pub(crate) fn repair_dffs_in_root(root: &Path) -> DffRepairResult {
    repair_dffs_in_root_with_context(
        root,
        HashSet::new(),
        DffTextureSanitizeContext::default(),
        DffRepairScope::default(),
    )
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn repair_dffs_in_root_with_buildings(
    root: &Path,
    building_dffs: HashSet<String>,
) -> DffRepairResult {
    repair_dffs_in_root_with_context(
        root,
        building_dffs,
        DffTextureSanitizeContext::default(),
        DffRepairScope::default(),
    )
}

fn repair_dffs_in_root_with_context(
    root: &Path,
    building_dffs: HashSet<String>,
    texture_context: DffTextureSanitizeContext,
    repair_scope: DffRepairScope,
) -> DffRepairResult {
    let started = Instant::now();
    let mut result = DffRepairResult {
        scope: repair_scope,
        scanned: 0,
        repaired: 0,
        prelighting_fixed: 0,
        sanitized_texture_names: 0,
        sanitized_texture_references: 0,
        normalized: 0,
        bounds_fixed: 0,
        uv_anim_reordered: 0,
        uv_anim_pipeline_fixed: 0,
        uv_anim_legacy_slots_fixed: 0,
        skipped: 0,
        repaired_dff_names: Vec::new(),
        repaired_entries: Vec::new(),
        errors: Vec::new(),
        elapsed: 0.0,
    };
    for path in project_img_files(root) {
        if let Err(err) = repair_dff_img_archive(
            &path,
            &building_dffs,
            &texture_context,
            repair_scope,
            &mut result,
        ) {
            result.errors.push(format!("{}: {err}", path.display()));
        }
    }
    result.repaired_dff_names.sort();
    result.repaired_dff_names.dedup();
    result.elapsed = started.elapsed().as_secs_f32();
    result
}

pub(crate) fn building_dff_set(app: &AppState) -> HashSet<String> {
    app.placements
        .iter()
        .filter(|placement| placement.tag.eq_ignore_ascii_case("building"))
        .map(|placement| {
            let dff_ref = app
                .definitions
                .get(&placement.id)
                .and_then(|def| def.attrs.get("dff"))
                .filter(|value| !value.trim().is_empty())
                .cloned()
                .unwrap_or_else(|| placement.dff.clone());
            asset_key(&dff_ref, ".dff")
        })
        .collect()
}

fn dff_repair_scope_label(scope: DffRepairScope) -> String {
    let mut labels = Vec::new();
    if scope.dff_issues {
        labels.push("DFF issues");
    }
    if scope.prelighting {
        labels.push("missing prelighting");
    }
    if scope.texture_names {
        labels.push("texture names");
    }
    if labels.is_empty() {
        "nothing selected".to_string()
    } else {
        labels.join(" + ")
    }
}

pub(crate) fn request_dff_repair(app: &mut AppState) {
    if app.manual_save_job.is_some()
        || app.editing.save_rx.is_some()
        || app.autosave_rx.is_some()
        || app.autosave_cleanup_rx.is_some()
        || app.dff_picker_rx.is_some()
        || app.dff_repair_rx.is_some()
        || app.dff_repair_refresh.is_some()
        || app.txd_cleanup_job.is_some()
        || app.asset_optimization_scan_rx.is_some()
        || app.asset_optimization_job.is_some()
        || app.object_bounds_fix_job.is_some()
        || app.corona_generation_job.is_some()
        || app.day_night_merge_job.is_some()
        || app.light_lod_job.is_some()
        || app.fracture_generation_job.is_some()
        || app.dff_geometry_job.is_some()
        || app.collision_generation_job.is_some()
        || app.shadow_mesh_generation_job.is_some()
        || app.lod_generation_job.is_some()
    {
        app.status_message =
            "DFF repair cannot start while another asset writer is running.".to_string();
        return;
    }
    let root = app.root.clone();
    let building_dffs = building_dff_set(app);
    let repair_scope = app.dff_repair_scope;
    if !repair_scope.any() {
        app.status_message = "Select at least one Repair DFFs category first.".to_string();
        return;
    }
    let texture_context = if repair_scope.texture_names {
        dff_texture_sanitize_context(app)
    } else {
        DffTextureSanitizeContext::default()
    };
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = std::panic::catch_unwind(|| {
            repair_dffs_in_root_with_context(&root, building_dffs, texture_context, repair_scope)
        })
        .unwrap_or_else(|err| {
            let detail = err
                .downcast_ref::<&str>()
                .map(|value| (*value).to_string())
                .or_else(|| err.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "unknown panic".to_string());
            DffRepairResult {
                scope: repair_scope,
                scanned: 0,
                repaired: 0,
                prelighting_fixed: 0,
                sanitized_texture_names: 0,
                sanitized_texture_references: 0,
                normalized: 0,
                bounds_fixed: 0,
                uv_anim_reordered: 0,
                uv_anim_pipeline_fixed: 0,
                uv_anim_legacy_slots_fixed: 0,
                skipped: 0,
                repaired_dff_names: Vec::new(),
                repaired_entries: Vec::new(),
                errors: vec![format!("DFF repair worker crashed: {detail}")],
                elapsed: 0.0,
            }
        });
        let _ = tx.send(result);
    });
    app.dff_repair_rx = Some(rx);
    app.status_message = format!(
        "DFF repair ({}) scan started...",
        dff_repair_scope_label(repair_scope)
    );
}

pub(crate) fn update_dff_repair_job(app: &mut AppState) {
    if let Some(mut refresh) = app.dff_repair_refresh.take() {
        let frame_started = Instant::now();
        let mut applied = 0usize;
        while refresh.next_definition < refresh.definition_ids.len()
            && applied < 8
            && frame_started.elapsed() < Duration::from_millis(4)
        {
            let definition_id = &refresh.definition_ids[refresh.next_definition];
            if recompile_definition_mesh(app, definition_id) {
                refresh.refreshed += 1;
            }
            refresh.next_definition += 1;
            applied += 1;
        }
        if refresh.next_definition < refresh.definition_ids.len() {
            app.status_message = format!(
                "DFF repair: refreshing preview {}/{}...",
                refresh.next_definition,
                refresh.definition_ids.len()
            );
            app.dff_repair_refresh = Some(refresh);
        } else {
            if refresh.refreshed > 0 {
                rebuild_render_cells(app);
            }
            finish_dff_repair(app, refresh.result, refresh.refreshed);
        }
        return;
    }

    let Some(rx) = app.dff_repair_rx.take() else {
        return;
    };
    match rx.try_recv() {
        Ok(result) => {
            let repaired = result
                .repaired_dff_names
                .iter()
                .cloned()
                .collect::<HashSet<_>>();
            let mut definition_ids = app
                .definitions
                .values()
                .filter_map(|definition| {
                    let dff = asset_key_opt(definition.attrs.get("dff"), &definition.id, ".dff");
                    repaired.contains(&dff).then(|| definition.id.clone())
                })
                .collect::<Vec<_>>();
            definition_ids.sort();
            definition_ids.dedup();
            if definition_ids.is_empty() {
                finish_dff_repair(app, result, 0);
            } else {
                app.dff_repair_refresh = Some(DffRepairRefresh {
                    result,
                    definition_ids,
                    next_definition: 0,
                    refreshed: 0,
                });
                app.status_message = "DFF repair scan finished; refreshing preview...".to_string();
            }
        }
        Err(mpsc::TryRecvError::Empty) => {
            app.status_message = "DFF repair scan running...".to_string();
            app.dff_repair_rx = Some(rx);
        }
        Err(mpsc::TryRecvError::Disconnected) => {
            app.status_message = "DFF repair scan failed: worker disconnected".to_string();
        }
    }
}

fn finish_dff_repair(app: &mut AppState, result: DffRepairResult, refreshed: usize) {
    invalidate_validation_cache(app);
    app.status_message = if result.errors.is_empty() {
        format!(
            "DFF repair ({}) finished in {:.1}s: scanned {}, repaired {}, fixed missing prelighting in {} DFF(s), sanitized {} texture name(s) across {} reference(s), refreshed {} preview definition(s) (normalized {}, bounds {}, uv-anim reordered {}, uv-anim pipeline {}, uv-anim legacy slots {}).",
            dff_repair_scope_label(result.scope),
            result.elapsed,
            result.scanned,
            result.repaired,
            result.prelighting_fixed,
            result.sanitized_texture_names,
            result.sanitized_texture_references,
            refreshed,
            result.normalized,
            result.bounds_fixed,
            result.uv_anim_reordered,
            result.uv_anim_pipeline_fixed,
            result.uv_anim_legacy_slots_fixed
        )
    } else {
        format!(
            "DFF repair ({}) finished with {} error(s). Scanned {}, repaired {}, sanitized {} texture name(s), skipped {}. Click status bar to view log.",
            dff_repair_scope_label(result.scope),
            result.errors.len(),
            result.scanned,
            result.repaired,
            result.sanitized_texture_names,
            result.skipped
        )
    };
    let mut log = vec![app.status_message.clone()];
    if result.repaired_entries.is_empty() {
        log.push("No DFF entries needed repair.".to_string());
    } else {
        log.push("Repaired DFF entries:".to_string());
        log.extend(result.repaired_entries);
    }
    let open_log = log.len() > 2 || !result.errors.is_empty();
    log.extend(result.errors);
    set_save_log(app, "DFF Repair + Texture Names", log, open_log);
}

fn repair_dff_archive_bounds_in_root(root: &Path) -> Result<usize, String> {
    let mut repaired = 0usize;
    for img_path in collect_resource_img_files(root) {
        let entries: Vec<ImgEntry> = parse_img(&img_path)
            .into_iter()
            .filter(|entry| {
                asset_ext(&entry.name).is_some_and(|ext| ext.eq_ignore_ascii_case("dff"))
            })
            .collect();
        if entries.is_empty() {
            continue;
        }
        let mut bytes =
            fs::read(&img_path).map_err(|err| format!("{}: {err}", img_path.display()))?;
        let mut changed = false;
        for entry in entries {
            let start = entry.offset as usize;
            let end = start.saturating_add(entry.size as usize).min(bytes.len());
            if start >= end {
                continue;
            }
            let count = repair_dff_bounds_spheres(&mut bytes[start..end]);
            if count != 0 {
                repaired += count;
                changed = true;
            }
        }
        if changed {
            fs::write(&img_path, &bytes).map_err(|err| format!("{}: {err}", img_path.display()))?;
        }
    }
    Ok(repaired)
}

pub(crate) fn merge_replacement_entries_into_root(
    source_root: &Path,
    target_root: &Path,
    replacements: Vec<(String, Vec<u8>)>,
) -> Result<usize, String> {
    if replacements.is_empty() {
        return Ok(0);
    }

    let mut merged = 0usize;
    let mut archive_groups = BTreeMap::<PathBuf, Vec<(String, Vec<u8>)>>::new();
    let destinations = ReplacementDestinationIndex::build(target_root);
    for (name, bytes) in replacements {
        if let Some(path) = destinations.loose_path(&name) {
            fs::write(&path, &bytes).map_err(|err| format!("{}: {err}", path.display()))?;
            merged += 1;
            continue;
        }
        let archive = destinations.archive_path(&name)?;
        archive_groups
            .entry(archive)
            .or_default()
            .push((name, bytes));
    }

    for (archive, entries) in archive_groups {
        merged += upsert_img_archive_entries_owned(&archive, entries)?;
    }

    let source_wip_root = wip_root_path(source_root);
    let target_wip_root = wip_root_path(target_root);
    for root in [
        source_root,
        source_wip_root.as_path(),
        target_root,
        target_wip_root.as_path(),
    ] {
        let path = root.join("imgs").join(REPLACEMENT_IMG);
        if path.is_file() {
            fs::remove_file(&path).map_err(|err| format!("{}: {err}", path.display()))?;
        }
    }
    Ok(merged)
}

pub(crate) fn promote_wip_replacement_assets(app: &AppState) -> Result<usize, String> {
    promote_wip_replacement_assets_to_root(app, &app.root)
}

fn promote_wip_replacement_assets_to_root(
    app: &AppState,
    target_root: &Path,
) -> Result<usize, String> {
    let wip_root = wip_root_path(&app.root);
    let replacements = replacement_archive_entries(&wip_root)?;
    merge_replacement_entries_into_root(&app.root, target_root, replacements)
}

pub(crate) fn apply_pending_asset_deletes(app: &AppState, root: &Path) -> Result<usize, String> {
    let mut removed = 0usize;
    for key in &app.pending_asset_deletes {
        if remove_loose_asset_file(root, key) {
            removed += 1;
        }
    }
    removed += remove_packed_img_entries(root, &app.pending_asset_deletes)?;
    Ok(removed)
}

/// Rewrites every packed IMG archive under `<root>/imgs` to drop any entry whose
/// lowercased file name is staged for deletion. Loose files are handled separately
/// by `remove_loose_asset_file`; without this pass, "Purge Unused" never actually
/// removed assets that live inside dff.img / col.img / txd.img.
pub(crate) fn remove_packed_img_entries(
    root: &Path,
    keys: &HashSet<String>,
) -> Result<usize, String> {
    if keys.is_empty() {
        return Ok(0);
    }
    let mut img_paths = Vec::new();
    collect_img_files_from_dir(&root.join("imgs"), &mut img_paths);
    let mut removed = 0usize;
    for img_path in img_paths {
        let parsed = parse_img(&img_path);
        if parsed.is_empty() {
            continue;
        }
        let mut entries = Vec::<(String, Vec<u8>)>::new();
        let mut changed = false;
        for entry in parsed {
            if keys.contains(&lower(&entry.name)) {
                removed += 1;
                changed = true;
            } else {
                let mut data = read_img_entry(&entry);
                let len = replacement_entry_len(&entry.name, &data);
                data.truncate(len);
                entries.push((entry.name, data));
            }
        }
        if changed {
            write_img_archive(&img_path, &entries)?;
        }
    }
    Ok(removed)
}

const ACTIVITY_LOG_LIMIT: usize = 4_000;

pub(crate) fn append_activity_entry(app: &mut AppState, message: impl Into<String>) {
    let elapsed = app.activity_started_at.elapsed();
    let minutes = elapsed.as_secs() / 60;
    let seconds = elapsed.as_secs() % 60;
    let tenths = elapsed.subsec_millis() / 100;
    app.activity_log.push(format!(
        "[{minutes:02}:{seconds:02}.{tenths}] {}",
        message.into()
    ));
    if app.activity_log.len() > ACTIVITY_LOG_LIMIT {
        let excess = app.activity_log.len() - ACTIVITY_LOG_LIMIT;
        app.activity_log.drain(..excess);
    }
    if app.save_log_open && app.save_log_follow_tail {
        // update_save_log_input clamps this sentinel to the exact wrapped-row
        // maximum before drawing the console.
        app.save_log_scroll = f32::MAX;
    }
}

pub(crate) fn record_activity_status(app: &mut AppState) {
    let status = app.status_message.trim();
    if status.is_empty() || status == app.activity_last_status {
        return;
    }
    let status = status.to_string();
    append_activity_entry(app, status.clone());
    app.activity_last_status = status;
}

pub(crate) fn set_save_log(app: &mut AppState, title: &str, entries: Vec<String>, open: bool) {
    app.save_log.clear();
    app.save_log.push(title.to_string());
    app.save_log.extend(entries.iter().cloned());
    append_activity_entry(app, format!("── {title} ──"));
    for entry in entries {
        append_activity_entry(app, format!("  {entry}"));
    }
    if open {
        app.save_log_open = true;
        app.save_log_follow_tail = true;
        app.save_log_scroll = f32::MAX;
    }
}

fn record_txd_generation_failure(app: &mut AppState, error: String) {
    app.status_message = "Generate TXD failed. View log for details.".to_string();
    set_save_log(app, "TXD Generation failed", vec![error], true);
}

pub(crate) fn apply_pending_inspector_edit_for_save(app: &mut AppState) {
    if app.inspector_edit.is_some() {
        apply_inspector_edit(app);
    }
}

fn apply_pending_editing_dff_for_save(app: &mut AppState) -> Result<(), String> {
    let should_stage_col = matches!(
        app.editing.asset.as_ref(),
        Some(EditingAsset::Col(col)) if col.dirty
    );
    if should_stage_col {
        editing_stage_col_asset(app);
        if matches!(
            app.editing.asset.as_ref(),
            Some(EditingAsset::Col(col)) if col.dirty
        ) {
            return Err(app.status_message.clone());
        }
    }
    let traffic_dff_name = app.editing.asset.as_ref().and_then(|asset| match asset {
        EditingAsset::Dff(dff)
            if dff
                .raw
                .effects_2dfx
                .iter()
                .any(|effect| effect.effect_id == 0 && effect.payload.get(20) == Some(&7)) =>
        {
            Some(dff.name.clone())
        }
        _ => None,
    });
    if let Some(dff_name) = traffic_dff_name.as_deref() {
        ensure_traffic_native_model_for_dff(app, dff_name);
    }
    let should_stage = matches!(
        app.editing.asset.as_ref(),
        Some(EditingAsset::Dff(dff)) if dff.dirty
    );
    if !should_stage {
        return Ok(());
    }
    if editing_stage_dff_asset(app) {
        Ok(())
    } else {
        Err(app.status_message.clone())
    }
}

fn save_race_tracks_for_root(app: &mut AppState, root: &Path) -> Result<bool, String> {
    if !app.race.loaded && app.race.tracks.is_empty() {
        return Ok(false);
    }
    save_race_tracks(
        root,
        &app.race.tracks,
        &app.race.radar_path,
        app.race.world_size,
        app.race.world_center_x,
        app.race.world_center_y,
    )?;
    if !app.race.tracks.is_empty() {
        patch_map_meta_for_tracks(root)?;
    }
    Ok(true)
}

#[derive(Clone)]
enum ManualSaveMode {
    Resource,
    Wip,
    SaveAs(PathBuf),
}

impl ManualSaveMode {
    fn label(&self) -> &'static str {
        match self {
            Self::Resource => "Save",
            Self::Wip => "Save WIP",
            Self::SaveAs(_) => "Save As",
        }
    }

    fn writes_resource(&self) -> bool {
        !matches!(self, Self::Wip)
    }
}

struct ManualSaveWriteSnapshot {
    mode: ManualSaveMode,
    source_root: PathBuf,
    zones: Vec<String>,
    eagle_zone_offsets: EagleZoneOffsets,
    placements: Vec<Placement>,
    definitions: HashMap<String, Definition>,
    readonly_definition_ids: HashSet<String>,
    element_states: Vec<ElementState>,
    lights: Vec<EditorLight>,
    material_emitters: HashMap<String, MaterialEmitter>,
    material_classes: TextureMaterialClasses,
    safe_collisions: SafeCollisions,
    shadow_casting: HashMap<String, bool>,
    water_planes: Vec<WaterPlane>,
    race_loaded: bool,
    race_tracks: Vec<RaceTrack>,
    race_radar_path: String,
    race_world_size: f32,
    race_world_center_x: f32,
    race_world_center_y: f32,
    replacement_assets: BTreeMap<String, (String, Vec<u8>)>,
    asset_deletes: HashSet<String>,
    vertex_meshes: Vec<AutosaveVertexMesh>,
    col_writes: HashMap<(PathBuf, u64), u8>,
}

struct ManualSaveReconcile {
    mode: ManualSaveMode,
    saved_content: SavedContentSnapshot,
    pending_col_writes: HashMap<(PathBuf, u64), u8>,
    pending_replacement_assets: BTreeMap<String, (String, Vec<u8>)>,
    pending_txd_writes: HashSet<String>,
    pending_asset_deletes: HashSet<String>,
    pending_vertex_light_meshes: HashSet<String>,
    vertex_colors: VertexColorSnapshot,
    editing_modified_entries: BTreeMap<String, Vec<u8>>,
    editing_added_entries: BTreeSet<String>,
    material_emitters: HashMap<String, MaterialEmitter>,
    material_classes: TextureMaterialClasses,
    safe_collisions: SafeCollisions,
    shadow_casting: HashMap<String, bool>,
}

pub(crate) struct ManualSaveJob {
    rx: mpsc::Receiver<ManualSaveMessage>,
    reconcile: ManualSaveReconcile,
}

enum ManualSaveMessage {
    Progress(String),
    Finished(ManualSaveResult),
}

pub(crate) struct ManualSaveResult {
    mode: ManualSaveMode,
    errors: Vec<String>,
    warnings: Vec<String>,
    flushed_vertex_light_assets: usize,
    replacement_assets: usize,
    col_writes: usize,
    deleted_assets: usize,
    saved_race_tracks: bool,
    worker_thread: thread::ThreadId,
}

fn capture_staged_replacement_assets(app: &AppState) -> BTreeMap<String, (String, Vec<u8>)> {
    let mut replacement_assets = app.pending_replacement_assets.clone();
    for (key, bytes) in &app.editing.modified_entries {
        if app.editing.deleted_entries.contains(key) || asset_ext(key).is_none() {
            continue;
        }
        let name = app
            .editing
            .rows
            .iter()
            .find(|row| lower(&row.entry.name) == *key)
            .map(|row| row.entry.name.clone())
            .unwrap_or_else(|| key.clone());
        let normalized = normalize_legacy_light_mapper_asset_name(&name);
        replacement_assets.insert(lower(&normalized), (normalized, bytes.clone()));
    }
    replacement_assets
}

fn capture_save_vertex_meshes(app: &AppState) -> Vec<AutosaveVertexMesh> {
    app.pending_vertex_light_meshes
        .iter()
        .filter(|key| !is_simulation_mesh_key(key))
        .filter(|key| {
            !vertex_lighting_targets_deleted_asset(
                key,
                &app.pending_asset_deletes,
                &app.editing.deleted_entries,
            )
        })
        .filter_map(|key| {
            let mesh = app.meshes.get(key)?.clone();
            let dff_name = key.split('|').next().unwrap_or(key);
            Some(AutosaveVertexMesh {
                key: key.clone(),
                mesh,
                options: dff_write_options_for_asset(app, dff_name),
            })
        })
        .collect()
}

fn capture_manual_save(
    app: &AppState,
    mode: ManualSaveMode,
) -> (ManualSaveWriteSnapshot, ManualSaveReconcile) {
    let vertex_meshes = capture_save_vertex_meshes(app);
    let vertex_colors = capture_vertex_colors(app, app.pending_vertex_light_meshes.iter());
    let mut saved_content = saved_content_snapshot(app);
    if mode.writes_resource() {
        let previous = app.saved_snapshot.as_ref();
        saved_content.editing_modified_entries = previous
            .map(|saved| {
                saved
                    .editing_modified_entries
                    .iter()
                    .filter(|(key, _)| asset_ext(key).is_none())
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect()
            })
            .unwrap_or_default();
        saved_content.editing_added_entries = previous
            .map(|saved| {
                saved
                    .editing_added_entries
                    .iter()
                    .filter(|key| asset_ext(key).is_none())
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        // Editing-tab deletions are committed only by Save IMG. Resource Save
        // must not mark them clean unless they were separately staged through
        // the resource-wide asset-delete queue.
        saved_content.editing_deleted_entries =
            saved_editing_deleted_baseline(previous.map(|saved| &saved.editing_deleted_entries));
    }
    let write = ManualSaveWriteSnapshot {
        mode: mode.clone(),
        source_root: app.root.clone(),
        zones: app.zones.clone(),
        eagle_zone_offsets: app.eagle_zone_offsets,
        placements: app.placements.clone(),
        definitions: app.definitions.clone(),
        readonly_definition_ids: app.readonly_definition_ids.clone(),
        element_states: app.element_states.clone(),
        lights: app.lights.clone(),
        material_emitters: app.material_emitters.clone(),
        material_classes: app.material_classes.clone(),
        safe_collisions: app.safe_collisions.clone(),
        shadow_casting: app.shadow_casting.clone(),
        water_planes: app.water_planes.clone(),
        race_loaded: app.race.loaded,
        race_tracks: app.race.tracks.clone(),
        race_radar_path: app.race.radar_path.clone(),
        race_world_size: app.race.world_size,
        race_world_center_x: app.race.world_center_x,
        race_world_center_y: app.race.world_center_y,
        replacement_assets: capture_staged_replacement_assets(app),
        asset_deletes: app.pending_asset_deletes.clone(),
        vertex_meshes,
        col_writes: app.pending_col_writes.clone(),
    };
    let reconcile = ManualSaveReconcile {
        mode,
        saved_content,
        pending_col_writes: app.pending_col_writes.clone(),
        pending_replacement_assets: app.pending_replacement_assets.clone(),
        pending_txd_writes: app.pending_txd_writes.clone(),
        pending_asset_deletes: app.pending_asset_deletes.clone(),
        pending_vertex_light_meshes: app.pending_vertex_light_meshes.clone(),
        vertex_colors,
        editing_modified_entries: app.editing.modified_entries.clone(),
        editing_added_entries: app.editing.added_entries.clone(),
        material_emitters: app.material_emitters.clone(),
        material_classes: app.material_classes.clone(),
        safe_collisions: app.safe_collisions.clone(),
        shadow_casting: app.shadow_casting.clone(),
    };
    (write, reconcile)
}

fn saved_editing_deleted_baseline(previous: Option<&BTreeSet<String>>) -> BTreeSet<String> {
    previous.cloned().unwrap_or_default()
}

fn save_race_tracks_from_snapshot(
    snapshot: &ManualSaveWriteSnapshot,
    root: &Path,
) -> Result<bool, String> {
    if !snapshot.race_loaded && snapshot.race_tracks.is_empty() {
        return Ok(false);
    }
    save_race_tracks(
        root,
        &snapshot.race_tracks,
        &snapshot.race_radar_path,
        snapshot.race_world_size,
        snapshot.race_world_center_x,
        snapshot.race_world_center_y,
    )?;
    if !snapshot.race_tracks.is_empty() {
        patch_map_meta_for_tracks(root)?;
    }
    Ok(true)
}

fn apply_pending_asset_deletes_from_parts(
    deletes: &HashSet<String>,
    root: &Path,
) -> Result<usize, String> {
    let mut removed = 0usize;
    for key in deletes {
        if remove_loose_asset_file(root, key) {
            removed += 1;
        }
    }
    removed += remove_packed_img_entries(root, deletes)?;
    Ok(removed)
}

fn promote_wip_replacement_assets_from_roots(
    source_root: &Path,
    target_root: &Path,
) -> Result<usize, String> {
    let replacements = replacement_archive_entries(&wip_root_path(source_root))?;
    merge_replacement_entries_into_root(source_root, target_root, replacements)
}

fn collect_project_legacy_scene_imports(root: &Path) -> Result<Vec<LegacySceneImport>, String> {
    let attr_re = Regex::new(r#"([A-Za-z_][A-Za-z0-9_]*)="([^"]*)""#)
        .map_err(|err| format!("Could not build the legacy light parser: {err}"))?;
    let mut imports = collect_legacy_scene_section_imports(root, &attr_re)?;
    if let Some(value) = load_legacy_material_classes_section(root)? {
        imports.push(LegacySceneImport::new("eagleMaterialClasses.json", value)?);
    }
    if let Some(value) = load_legacy_safe_collisions_section(root)? {
        imports.push(LegacySceneImport::new("eagleSafeCollisions.json", value)?);
    }
    Ok(imports)
}

fn section_for_legacy_scene_file(file_name: &str) -> Option<&'static str> {
    match file_name {
        "Light_Emitters.json" => Some(SECTION_MATERIAL_EMITTERS),
        "Shadow_Casters.json" => Some(SECTION_SHADOW_CASTERS),
        "Collision_Capsules.json" => Some(crate::resource::eagle_scene::SECTION_COLLISION_CAPSULES),
        "Collision_Cuboids.json" => Some(crate::resource::eagle_scene::SECTION_COLLISION_CUBOIDS),
        "eagleMaterialClasses.json" => Some(SECTION_MATERIAL_CLASSES),
        "eagleSafeCollisions.json" => Some(SECTION_SAFE_COLLISIONS),
        "Light_List.xml" => Some(SECTION_LIGHTS),
        _ => None,
    }
}

/// Seeds a detached WIP/autosave scene document from the resource without
/// moving any legacy source files. Existing unified sections win over legacy
/// compatibility data.
fn seed_detached_eagle_scene(source_root: &Path, target_root: &Path) -> Result<(), String> {
    if source_root == target_root {
        return Ok(());
    }
    fs::create_dir_all(target_root)
        .map_err(|err| format!("Could not create {}: {err}", target_root.display()))?;
    if read_eagle_scene(source_root)?.is_some() {
        fs::copy(eagle_scene_path(source_root), eagle_scene_path(target_root)).map_err(|err| {
            format!(
                "{} -> {}: {err}",
                eagle_scene_path(source_root).display(),
                eagle_scene_path(target_root).display()
            )
        })?;
    } else if eagle_scene_path(target_root).is_file() {
        fs::remove_file(eagle_scene_path(target_root)).map_err(|err| {
            format!(
                "Could not clear stale detached {}: {err}",
                eagle_scene_path(target_root).display()
            )
        })?;
    }
    let imports = collect_project_legacy_scene_imports(source_root)?;
    let mut updates = Vec::new();
    for import in imports {
        let Some(section) = section_for_legacy_scene_file(&import.file_name) else {
            continue;
        };
        if read_eagle_scene_section(target_root, section)?.is_none() {
            updates.push(EagleSceneSectionUpdate::new(section, import.value)?);
        }
    }
    update_eagle_scene_sections(target_root, &updates)
}

fn live_scene_section_updates(
    lights: &[EditorLight],
    material_emitters: &HashMap<String, MaterialEmitter>,
    material_classes: &TextureMaterialClasses,
    safe_collisions: &SafeCollisions,
    shadow_casting: &HashMap<String, bool>,
) -> Result<Vec<EagleSceneSectionUpdate>, String> {
    Ok(vec![
        EagleSceneSectionUpdate::new(SECTION_LIGHTS, lights_section_value(lights))?,
        EagleSceneSectionUpdate::new(
            SECTION_MATERIAL_EMITTERS,
            material_emitters_section_value(material_emitters),
        )?,
        EagleSceneSectionUpdate::new(
            SECTION_MATERIAL_CLASSES,
            material_classes_section_value(material_classes),
        )?,
        EagleSceneSectionUpdate::new(
            SECTION_SAFE_COLLISIONS,
            safe_collisions_section_value(safe_collisions),
        )?,
        EagleSceneSectionUpdate::new(
            SECTION_SHADOW_CASTERS,
            shadow_casting_section_value(shadow_casting),
        )?,
    ])
}

fn write_manual_scene_documents(
    snapshot: &ManualSaveWriteSnapshot,
    root: &Path,
    include_resource_sidecars: bool,
    errors: &mut Vec<String>,
) -> bool {
    if let Err(mut scene_errors) = write_scene_files_data(
        &snapshot.zones,
        snapshot.eagle_zone_offsets,
        &snapshot.placements,
        &snapshot.definitions,
        &snapshot.readonly_definition_ids,
        &snapshot.element_states,
        root,
    ) {
        errors.append(&mut scene_errors);
    }
    if !include_resource_sidecars
        && let Err(err) = seed_detached_eagle_scene(&snapshot.source_root, root)
    {
        errors.push(err);
    }
    match live_scene_section_updates(
        &snapshot.lights,
        &snapshot.material_emitters,
        &snapshot.material_classes,
        &snapshot.safe_collisions,
        &snapshot.shadow_casting,
    )
    .and_then(|updates| update_eagle_scene_sections(root, &updates))
    {
        Ok(()) => {}
        Err(err) => errors.push(err),
    }
    if let Err(err) = save_water_dat(&water_dat_path(root), &snapshot.water_planes) {
        errors.push(err);
    }
    if include_resource_sidecars {
        match save_race_tracks_from_snapshot(snapshot, root) {
            Ok(saved) => saved,
            Err(err) => {
                errors.push(err);
                false
            }
        }
    } else {
        false
    }
}

fn write_manual_save_snapshot(snapshot: ManualSaveWriteSnapshot) -> ManualSaveResult {
    let worker_thread = thread::current().id();
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut flushed_vertex_light_assets = 0usize;
    let mut replacement_assets = 0usize;
    let mut col_writes = 0usize;
    let mut deleted_assets = 0usize;
    let mut prepared_migration: Option<PreparedEagleSceneMigration> = None;
    let saved_race_tracks;
    let source_root = snapshot.source_root.clone();
    let target_root = match &snapshot.mode {
        ManualSaveMode::Resource => source_root.clone(),
        ManualSaveMode::Wip => wip_root_path(&source_root),
        ManualSaveMode::SaveAs(target) => target.clone(),
    };

    if matches!(snapshot.mode, ManualSaveMode::SaveAs(_))
        && let Err(mut copy_errors) = copy_resource_shell(&source_root, &target_root)
    {
        errors.append(&mut copy_errors);
    }
    if errors.is_empty() && !matches!(snapshot.mode, ManualSaveMode::Wip) {
        match collect_project_legacy_scene_imports(&target_root)
            .and_then(|imports| prepare_eagle_scene_migration(&target_root, &imports))
        {
            Ok(prepared) => prepared_migration = prepared,
            Err(err) => errors.push(format!("Legacy EagleScene migration failed: {err}")),
        }
    }
    if !errors.is_empty() {
        return ManualSaveResult {
            mode: snapshot.mode,
            errors,
            warnings,
            flushed_vertex_light_assets,
            replacement_assets,
            col_writes,
            deleted_assets,
            saved_race_tracks: false,
            worker_thread,
        };
    }

    let staged_entries =
        match pending_replacement_entries_from_parts(&source_root, &snapshot.replacement_assets) {
            Ok(entries) => entries,
            Err(err) => {
                errors.push(err);
                Vec::new()
            }
        };
    if errors.is_empty() && !staged_entries.is_empty() {
        let wip_img_dir = wip_root_path(&source_root).join("imgs");
        match fs::create_dir_all(&wip_img_dir)
            .map_err(|err| format!("{}: {err}", wip_img_dir.display()))
            .and_then(|_| write_img_archive(&wip_img_dir.join(REPLACEMENT_IMG), &staged_entries))
        {
            Ok(()) => replacement_assets = staged_entries.len(),
            Err(err) => errors.push(err),
        }
    }
    if errors.is_empty() {
        match write_vertex_lighting_meshes_resilient(
            &source_root,
            &wip_root_path(&source_root),
            snapshot
                .vertex_meshes
                .iter()
                .map(|vertex| (vertex.key.as_str(), &vertex.mesh, vertex.options)),
        ) {
            Ok(outcome) => {
                flushed_vertex_light_assets = outcome.written;
                warnings.extend(outcome.warnings);
            }
            Err(err) => errors.push(err),
        }
    }

    match &snapshot.mode {
        ManualSaveMode::Wip => {
            saved_race_tracks = false;
            let _ = write_manual_scene_documents(&snapshot, &target_root, false, &mut errors);
            let readme = target_root.join("README.txt");
            if let Err(err) = fs::write(
                &readme,
                "Light Mapper WIP snapshot. Normal Save promotes this state to the resource.\nGenerated replacement DFF/TXD/COL archives are stored in imgs/.\nwater.dat is stored here when water planes are edited.\n",
            ) {
                errors.push(format!("{}: {err}", readme.display()));
            }
            if let Err(err) = save_wip_asset_deletes(&source_root, &snapshot.asset_deletes) {
                errors.push(err);
            }
        }
        ManualSaveMode::Resource | ManualSaveMode::SaveAs(_) => {
            saved_race_tracks =
                write_manual_scene_documents(&snapshot, &target_root, true, &mut errors);
            if matches!(snapshot.mode, ManualSaveMode::SaveAs(_))
                && let Err(err) = update_save_as_meta_root(&target_root)
            {
                errors.push(err);
            }
            if errors.is_empty() {
                match promote_wip_replacement_assets_from_roots(&source_root, &target_root) {
                    Ok(count) => replacement_assets = count,
                    Err(err) => errors.push(err),
                }
            }
            match apply_pending_col_writes_from_parts(
                &source_root,
                &target_root,
                &snapshot.col_writes,
            ) {
                Ok(count) => col_writes = count,
                Err(mut col_errors) => errors.append(&mut col_errors),
            }
            if errors.is_empty() {
                match apply_pending_asset_deletes_from_parts(&snapshot.asset_deletes, &target_root)
                {
                    Ok(count) => deleted_assets = count,
                    Err(err) => errors.push(err),
                }
            }
            if errors.is_empty()
                && let Some(prepared) = prepared_migration.take()
            {
                match finalize_eagle_scene_migration(prepared) {
                    Ok(outcome) => warnings.push(format!(
                        "Migrated {} legacy scene section(s) and moved {} verified source file(s) to {}.",
                        outcome.migrated_sections,
                        outcome.archived_files,
                        outcome.archive_dir.display()
                    )),
                    Err(err) => errors.push(format!(
                        "Could not finalize legacy EagleScene migration; source files were retained: {err}"
                    )),
                }
            }
            if errors.is_empty() {
                if let Err(err) = save_wip_asset_deletes(&source_root, &HashSet::new()) {
                    warnings.push(err);
                }
                for path in [wip_root_path(&source_root), wip_root_path(&target_root)] {
                    if path.is_dir()
                        && let Err(err) = fs::remove_dir_all(&path)
                    {
                        warnings.push(format!("Could not remove {}: {err}", path.display()));
                    }
                }
            }
        }
    }

    ManualSaveResult {
        mode: snapshot.mode,
        errors,
        warnings,
        flushed_vertex_light_assets,
        replacement_assets,
        col_writes,
        deleted_assets,
        saved_race_tracks,
        worker_thread,
    }
}

fn spawn_manual_save_worker(
    snapshot: ManualSaveWriteSnapshot,
) -> mpsc::Receiver<ManualSaveMessage> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mode = snapshot.mode.clone();
        let worker_thread = thread::current().id();
        let _ = tx.send(ManualSaveMessage::Progress(
            "Writing scene files and composing asset archives...".to_string(),
        ));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            write_manual_save_snapshot(snapshot)
        }))
        .unwrap_or_else(|panic| {
            let detail = panic
                .downcast_ref::<&str>()
                .map(|value| (*value).to_string())
                .or_else(|| panic.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "unknown panic".to_string());
            ManualSaveResult {
                mode,
                errors: vec![format!("Manual save worker crashed: {detail}")],
                warnings: Vec::new(),
                flushed_vertex_light_assets: 0,
                replacement_assets: 0,
                col_writes: 0,
                deleted_assets: 0,
                saved_race_tracks: false,
                worker_thread,
            }
        });
        let _ = tx.send(ManualSaveMessage::Finished(result));
    });
    rx
}

fn reconcile_hash_map<K, V>(current: &mut HashMap<K, V>, captured: &HashMap<K, V>)
where
    K: Eq + std::hash::Hash,
    V: PartialEq,
{
    current.retain(|key, value| captured.get(key) != Some(value));
}

fn reconcile_btree_map<K, V>(current: &mut BTreeMap<K, V>, captured: &BTreeMap<K, V>)
where
    K: Ord,
    V: PartialEq,
{
    current.retain(|key, value| captured.get(key) != Some(value));
}

fn reconcile_hash_set<T>(current: &mut HashSet<T>, captured: &HashSet<T>)
where
    T: Eq + std::hash::Hash,
{
    current.retain(|value| !captured.contains(value));
}

fn reconcile_btree_set<T>(current: &mut BTreeSet<T>, captured: &BTreeSet<T>)
where
    T: Ord,
{
    current.retain(|value| !captured.contains(value));
}

fn reconcile_manual_save_success(
    app: &mut AppState,
    reconcile: ManualSaveReconcile,
    result: &ManualSaveResult,
) {
    if let ManualSaveMode::SaveAs(target) = &result.mode {
        app.root = target.clone();
        save_recent_project(target);
    }

    let current_vertex_colors =
        capture_vertex_colors(app, reconcile.pending_vertex_light_meshes.iter());
    for key in &reconcile.pending_vertex_light_meshes {
        if reconcile.vertex_colors.meshes.get(key) == current_vertex_colors.meshes.get(key) {
            app.pending_vertex_light_meshes.remove(key);
        }
    }

    if reconcile.mode.writes_resource() {
        reconcile_hash_map(&mut app.pending_col_writes, &reconcile.pending_col_writes);
        reconcile_btree_map(
            &mut app.pending_replacement_assets,
            &reconcile.pending_replacement_assets,
        );
        reconcile_hash_set(&mut app.pending_txd_writes, &reconcile.pending_txd_writes);
        reconcile_hash_set(
            &mut app.pending_asset_deletes,
            &reconcile.pending_asset_deletes,
        );
        reconcile_btree_map(
            &mut app.editing.modified_entries,
            &reconcile.editing_modified_entries,
        );
        reconcile_btree_set(
            &mut app.editing.added_entries,
            &reconcile.editing_added_entries,
        );
        if app.material_emitters == reconcile.material_emitters {
            app.material_emitters_dirty = false;
        }
        if app.material_classes == reconcile.material_classes {
            app.material_classes_dirty = false;
        }
        if app.safe_collisions == reconcile.safe_collisions {
            app.safe_collisions_dirty = false;
        }
        let _shadow_casting_unchanged = app.shadow_casting == reconcile.shadow_casting;
        app.loaded_wip = false;
    }
    app.loaded_autosave = false;
    app.saved_snapshot = Some(reconcile.saved_content);
}

pub(crate) fn poll_manual_save(app: &mut AppState) {
    let Some(job) = app.manual_save_job.take() else {
        return;
    };
    match job.rx.try_recv() {
        Ok(ManualSaveMessage::Progress(status)) => {
            app.status_message = status;
            app.manual_save_job = Some(job);
        }
        Ok(ManualSaveMessage::Finished(result)) => {
            let label = result.mode.label();
            let _worker_thread = result.worker_thread;
            if result.errors.is_empty() {
                reconcile_manual_save_success(app, job.reconcile, &result);
                let warning_count = result.warnings.len();
                let race = if result.saved_race_tracks {
                    ", race tracks"
                } else {
                    ""
                };
                app.status_message = match &result.mode {
                    ManualSaveMode::Wip => format!(
                        "Saved WIP snapshot; wrote {} vertex-light DFF(s) and stored {} replacement asset(s).",
                        result.flushed_vertex_light_assets, result.replacement_assets
                    ),
                    ManualSaveMode::Resource => format!(
                        "Saved scene{race}; wrote {} vertex-light DFF(s), merged {} replacement asset(s), applied {} COL byte edit(s), and deleted {} staged asset(s).",
                        result.flushed_vertex_light_assets,
                        result.replacement_assets,
                        result.col_writes,
                        result.deleted_assets
                    ),
                    ManualSaveMode::SaveAs(target) => format!(
                        "Saved resource as {}{race}; wrote {} vertex-light DFF(s), merged {} replacement asset(s), applied {} COL byte edit(s), and deleted {} staged asset(s).",
                        ellipsize(target.to_string_lossy().as_ref(), 52),
                        result.flushed_vertex_light_assets,
                        result.replacement_assets,
                        result.col_writes,
                        result.deleted_assets
                    ),
                };
                if warning_count > 0 {
                    app.status_message.push_str(&format!(
                        " Completed with {warning_count} warning(s); click the status bar for details."
                    ));
                }
                let mut log = vec![app.status_message.clone()];
                log.extend(result.warnings);
                let title = if warning_count > 0 {
                    format!("{label} completed with warnings")
                } else {
                    format!("{label} completed")
                };
                set_save_log(app, &title, log, false);
                if let Some(action) = app.pending_after_manual_save.take() {
                    run_confirm_action(app, action);
                }
            } else {
                app.pending_after_manual_save = None;
                app.status_message = format!(
                    "{label} failed with {} error(s). Click status bar to view log.",
                    result.errors.len()
                );
                let mut errors = result.errors;
                errors.extend(result.warnings);
                set_save_log(app, &format!("{label} failed"), errors, true);
            }
        }
        Err(mpsc::TryRecvError::Empty) => {
            app.status_message = format!(
                "{} running in the background...",
                job.reconcile.mode.label()
            );
            app.manual_save_job = Some(job);
        }
        Err(mpsc::TryRecvError::Disconnected) => {
            app.pending_after_manual_save = None;
            app.status_message = "Manual save worker stopped unexpectedly.".to_string();
            set_save_log(
                app,
                "Manual save failed",
                vec![app.status_message.clone()],
                true,
            );
        }
    }
}

fn active_conflicting_save_job(app: &AppState) -> Option<&'static str> {
    if let Some(writer) = primary_save_writer_conflict(
        app.manual_save_job.is_some(),
        app.editing.save_rx.is_some(),
        app.autosave_rx.is_some(),
    ) {
        Some(writer)
    } else if app.autosave_cleanup_rx.is_some() {
        Some("autosave cleanup")
    } else if app.bake_job.is_some() {
        Some("vertex-light bake")
    } else if app.dff_repair_rx.is_some() || app.dff_repair_refresh.is_some() {
        Some("DFF repair")
    } else if app.txd_cleanup_job.is_some() {
        Some("TXD cleanup")
    } else if app.asset_optimization_scan_rx.is_some() {
        Some("asset optimization scan")
    } else if app.asset_optimization_job.is_some() {
        Some("asset optimization")
    } else if app.purge_unused_job.is_some() {
        Some("unused-asset purge")
    } else if app.img_archive_rebalance_job.is_some() {
        Some("IMG archive organization")
    } else if app.object_bounds_fix_job.is_some() {
        Some("object bounds repair")
    } else if app.water_texture_conversion_job.is_some() {
        Some("texture-to-water conversion")
    } else if app.vehicle_browser.collision_copy_rx.is_some() {
        Some("vehicle collision copy")
    } else if app.editing.txd_import_rx.is_some() || app.editing.txd_refresh_job.is_some() {
        Some("TXD texture import")
    } else if app.editing.merge_rx.is_some() || app.editing.merge_apply_job.is_some() {
        Some("Editing IMG merge")
    } else if app.dff_picker_rx.is_some() {
        // This receiver is shared by file pickers, exports, and the background
        // TXD generator. Conservatively serialize Save against it because the
        // receiver does not expose which variant is currently running.
        Some("asset file operation")
    } else {
        None
    }
}

fn primary_save_writer_conflict(
    manual_save: bool,
    editing_img_save: bool,
    autosave: bool,
) -> Option<&'static str> {
    if manual_save {
        Some("manual save")
    } else if editing_img_save {
        Some("Editing IMG save")
    } else if autosave {
        Some("autosave")
    } else {
        None
    }
}

fn start_manual_save(app: &mut AppState, mode: ManualSaveMode) -> bool {
    let label = mode.label();
    if let Some(job) = active_conflicting_save_job(app) {
        app.status_message = format!("Wait for the background {job} to finish before {label}.");
        return false;
    }
    let blocker = if app.instance_lod_removal_job.is_some() {
        Some("instance LOD removal")
    } else if app.lod_generation_job.is_some() {
        Some("LOD generation")
    } else if app.collision_generation_job.is_some() || app.shadow_mesh_generation_job.is_some() {
        Some("collision generation")
    } else if app.corona_generation_job.is_some() {
        Some("2DFX corona generation")
    } else if app.day_night_merge_job.is_some() {
        Some("day/night variant merge")
    } else if app.light_lod_job.is_some() {
        Some("LOD lighting")
    } else if app.fracture_generation_job.is_some() {
        Some("fracture generation")
    } else if app.dff_geometry_job.is_some() {
        Some("DFF geometry operation")
    } else {
        None
    };
    if let Some(blocker) = blocker {
        app.status_message = format!("Wait for the background {blocker} to finish before {label}.");
        return false;
    }

    apply_pending_inspector_edit_for_save(app);
    if let Err(err) = apply_pending_editing_dff_for_save(app) {
        set_save_log(app, &format!("{label} failed"), vec![err], true);
        return false;
    }
    if matches!(mode, ManualSaveMode::Wip) && !app.pending_col_writes.is_empty() {
        app.status_message =
            "Save WIP stores map/light XML only. Save the resource or discard pending COL byte edits."
                .to_string();
        return false;
    }

    let (snapshot, reconcile) = capture_manual_save(app, mode);
    let rx = spawn_manual_save_worker(snapshot);
    app.manual_save_job = Some(ManualSaveJob { rx, reconcile });
    app.status_message = format!("{label} running in the background...");
    true
}

pub(crate) fn save_scene(app: &mut AppState) {
    let _ = start_manual_save(app, ManualSaveMode::Resource);
}

#[allow(dead_code)]
fn save_scene_sync_legacy(app: &mut AppState) {
    if let Some(job) = active_conflicting_save_job(app) {
        app.status_message = format!("Wait for the background {job} to finish before saving.");
        return;
    }
    if app.instance_lod_removal_job.is_some() {
        app.status_message =
            "Wait for background instance LOD removal to finish before saving.".to_string();
        return;
    }
    if app.lod_generation_job.is_some() {
        app.status_message =
            "Wait for background LOD generation to finish before saving.".to_string();
        return;
    }
    if app.collision_generation_job.is_some() || app.shadow_mesh_generation_job.is_some() {
        app.status_message =
            "Wait for background collision generation to finish before saving.".to_string();
        return;
    }
    if app.corona_generation_job.is_some() {
        app.status_message =
            "Wait for background 2DFX corona generation to finish before saving.".to_string();
        return;
    }
    if app.day_night_merge_job.is_some() {
        app.status_message =
            "Wait for the day/night variant merge to finish before saving.".to_string();
        return;
    }
    if app.light_lod_job.is_some() {
        app.status_message = "Wait for Light LOD to finish before saving.".to_string();
        return;
    }
    if app.fracture_generation_job.is_some() {
        app.status_message =
            "Wait for background fracture generation to finish before saving.".to_string();
        return;
    }
    if app.dff_geometry_job.is_some() {
        app.status_message =
            "Wait for the background DFF geometry operation to finish before saving.".to_string();
        return;
    }
    apply_pending_inspector_edit_for_save(app);
    let mut errors = Vec::new();
    if let Err(err) = apply_pending_editing_dff_for_save(app) {
        errors.push(err);
    }
    let wip_root = wip_root_path(&app.root);
    // Compose every staged asset into one archive before vertex writeback.
    // Vertex lighting must be the final DFF mutation; otherwise an in-memory
    // Editing-tab replacement can overwrite the freshly written colors during
    // promotion.
    if errors.is_empty()
        && let Err(err) = copy_replacement_assets_to_wip(app, &wip_root)
    {
        errors.push(err);
    }
    let mut flushed_vertex_light_assets = 0usize;
    if errors.is_empty() {
        match flush_pending_vertex_lighting(app, &wip_root) {
            Ok(count) => flushed_vertex_light_assets = count,
            Err(err) => errors.push(err),
        }
    }
    let mut promoted_replacement_assets = 0usize;
    if errors.is_empty() {
        match promote_wip_replacement_assets(app) {
            Ok(count) => promoted_replacement_assets = count,
            Err(err) => errors.push(err),
        }
    }
    if let Err(mut err) = write_scene_files(app, &app.root) {
        errors.append(&mut err);
    }
    match live_scene_section_updates(
        &app.lights,
        &app.material_emitters,
        &app.material_classes,
        &app.safe_collisions,
        &app.shadow_casting,
    )
    .and_then(|updates| update_eagle_scene_sections(&app.root, &updates))
    {
        Ok(()) => {}
        Err(err) => errors.push(err),
    }
    if let Err(err) = save_water_dat(&water_dat_path(&app.root), &app.water_planes) {
        errors.push(err);
    }
    let root_for_race = app.root.clone();
    let mut saved_race_tracks = false;
    match save_race_tracks_for_root(app, &root_for_race) {
        Ok(saved) => saved_race_tracks = saved,
        Err(err) => errors.push(err),
    }
    let mut col_writes = 0usize;
    match apply_pending_col_writes(app, &app.root) {
        Ok(written) => col_writes = written,
        Err(mut err) => errors.append(&mut err),
    }
    let mut deleted_assets = 0usize;
    if errors.is_empty() {
        match apply_pending_asset_deletes(app, &app.root) {
            Ok(count) => deleted_assets = count,
            Err(err) => errors.push(err),
        }
    }
    if errors.is_empty() {
        app.pending_col_writes.clear();
        app.pending_replacement_assets.clear();
        app.pending_txd_writes.clear();
        app.pending_asset_deletes.clear();
        app.pending_vertex_light_meshes.clear();
        clear_saved_editing_replacements(app);
        if let Err(err) = save_wip_asset_deletes(&app.root, &app.pending_asset_deletes) {
            app.status_message = format!("Saved, but could not clear WIP purge list: {err}");
            set_save_log(
                app,
                "Save completed with a cleanup warning",
                vec![err],
                true,
            );
            return;
        }
        let _ = fs::remove_dir_all(wip_root_path(&app.root));
        app.loaded_autosave = false;
        app.loaded_wip = false;
        app.material_classes_dirty = false;
        app.safe_collisions_dirty = false;
        mark_saved_snapshot(app);
        app.status_message = if promoted_replacement_assets > 0
            || col_writes > 0
            || deleted_assets > 0
        {
            format!(
                "Saved scene, lights, water.dat{}; wrote {flushed_vertex_light_assets} vertex-light DFF(s), merged {promoted_replacement_assets} staged asset(s) into resource files, applied {col_writes} COL byte edit(s), and {deleted_assets} staged asset delete(s).",
                if saved_race_tracks {
                    ", and race tracks"
                } else {
                    ""
                }
            )
        } else {
            format!(
                "Saved scene maps/definitions, lights, water.dat{}. IMG unchanged: no DFF/TXD/COL mutations pending.",
                if saved_race_tracks {
                    ", and race tracks"
                } else {
                    ""
                }
            )
        };
        set_save_log(
            app,
            "Save completed",
            vec![app.status_message.clone()],
            false,
        );
    } else {
        app.status_message = format!(
            "Save finished with {} error(s). Click status bar to view log.",
            errors.len()
        );
        set_save_log(app, "Save failed", errors, true);
    }
}

pub(crate) fn save_wip_scene(app: &mut AppState) -> bool {
    start_manual_save(app, ManualSaveMode::Wip)
}

#[allow(dead_code)]
fn save_wip_scene_sync_legacy(app: &mut AppState) -> bool {
    if let Some(job) = active_conflicting_save_job(app) {
        app.status_message = format!("Wait for the background {job} to finish before saving WIP.");
        return false;
    }
    if app.instance_lod_removal_job.is_some() {
        app.status_message =
            "Wait for background instance LOD removal to finish before saving WIP.".to_string();
        return false;
    }
    if app.lod_generation_job.is_some() {
        app.status_message =
            "Wait for background LOD generation to finish before saving WIP.".to_string();
        return false;
    }
    if app.collision_generation_job.is_some() || app.shadow_mesh_generation_job.is_some() {
        app.status_message =
            "Wait for background collision generation to finish before saving WIP.".to_string();
        return false;
    }
    if app.corona_generation_job.is_some() {
        app.status_message =
            "Wait for background 2DFX corona generation to finish before saving WIP.".to_string();
        return false;
    }
    if app.day_night_merge_job.is_some() {
        app.status_message =
            "Wait for the day/night variant merge to finish before saving WIP.".to_string();
        return false;
    }
    if app.light_lod_job.is_some() {
        app.status_message = "Wait for Light LOD to finish before saving WIP.".to_string();
        return false;
    }
    if app.fracture_generation_job.is_some() {
        app.status_message =
            "Wait for background fracture generation to finish before saving WIP.".to_string();
        return false;
    }
    if app.dff_geometry_job.is_some() {
        app.status_message =
            "Wait for the background DFF geometry operation to finish before saving WIP."
                .to_string();
        return false;
    }
    apply_pending_inspector_edit_for_save(app);
    if let Err(err) = apply_pending_editing_dff_for_save(app) {
        set_save_log(app, "Save WIP failed", vec![err], true);
        return false;
    }
    if !app.pending_col_writes.is_empty() {
        app.status_message =
            "Save WIP stores map/light XML only. Save the resource or discard pending COL byte edits."
                .to_string();
        return false;
    }
    let wip_root = wip_root_path(&app.root);
    let mut errors = Vec::new();
    // Lay down staged DFF/TXD/COL replacements first. Vertex-light writeback
    // must read and update that newest DFF topology, and must be the final
    // writer for any DFF that appears in both sets.
    let copied_replacement_assets = match copy_replacement_assets_to_wip(app, &wip_root) {
        Ok(count) => count,
        Err(err) => {
            errors.push(err);
            0
        }
    };
    let mut flushed_vertex_light_assets = 0usize;
    if errors.is_empty() {
        match flush_pending_vertex_lighting(app, &wip_root) {
            Ok(count) => flushed_vertex_light_assets = count,
            Err(err) => errors.push(err),
        }
    }
    if let Err(mut err) = write_scene_files(app, &wip_root) {
        errors.append(&mut err);
    }
    if let Err(err) = seed_detached_eagle_scene(&app.root, &wip_root) {
        errors.push(err);
    }
    match live_scene_section_updates(
        &app.lights,
        &app.material_emitters,
        &app.material_classes,
        &app.safe_collisions,
        &app.shadow_casting,
    )
    .and_then(|updates| update_eagle_scene_sections(&wip_root, &updates))
    {
        Ok(()) => {}
        Err(err) => errors.push(err),
    }
    if let Err(err) = save_water_dat(&wip_water_dat_path(&app.root), &app.water_planes) {
        errors.push(err);
    }
    if let Err(err) = fs::write(
        wip_root.join("README.txt"),
        "Light Mapper WIP snapshot. Normal Save promotes this state to the resource.\nGenerated replacement DFF/TXD/COL archives are stored in imgs/.\nwater.dat is stored here when water planes are edited.\n",
    ) {
        errors.push(format!("{}: {err}", wip_root.join("README.txt").display()));
    }
    if let Err(err) = save_wip_asset_deletes(&app.root, &app.pending_asset_deletes) {
        errors.push(err);
    }
    if errors.is_empty() {
        app.pending_vertex_light_meshes.clear();
        app.loaded_autosave = false;
        mark_saved_snapshot(app);
        app.status_message = format!(
            "Saved WIP snapshot for {}; wrote water.dat, {flushed_vertex_light_assets} vertex-light DFF(s), and stored {copied_replacement_assets} replacement asset archive(s).",
            ellipsize(app.root.to_string_lossy().as_ref(), 42)
        );
        set_save_log(
            app,
            "Save WIP completed",
            vec![app.status_message.clone()],
            false,
        );
        true
    } else {
        app.status_message = format!(
            "WIP save finished with {} error(s). Click status bar to view log.",
            errors.len()
        );
        set_save_log(app, "Save WIP failed", errors, true);
        false
    }
}

struct AutosaveVertexMesh {
    key: String,
    mesh: RenderMesh,
    options: DffWriteOptions,
}

enum AutosaveEditingAsset {
    Dff {
        name: String,
        raw: RawMesh,
        boolean_box: Option<DffBooleanBox>,
    },
    Col {
        name: String,
        mesh: CollisionMesh,
        source_bytes: Vec<u8>,
        editing_shadow: bool,
        capsules: Vec<CollisionCapsule>,
        cuboids: Vec<CollisionCuboid>,
        embedded_source_dff_bytes: Option<Vec<u8>>,
    },
}

struct AutosaveSnapshot {
    root: PathBuf,
    captured_at: SystemTime,
    zones: Vec<String>,
    eagle_zone_offsets: EagleZoneOffsets,
    placements: Vec<Placement>,
    definitions: HashMap<String, Definition>,
    readonly_definition_ids: HashSet<String>,
    element_states: Vec<ElementState>,
    lights: Vec<EditorLight>,
    material_emitters: HashMap<String, MaterialEmitter>,
    material_classes: TextureMaterialClasses,
    safe_collisions: SafeCollisions,
    shadow_casting: HashMap<String, bool>,
    water_planes: Vec<WaterPlane>,
    replacement_assets: BTreeMap<String, (String, Vec<u8>)>,
    editing_asset: Option<AutosaveEditingAsset>,
    asset_deletes: HashSet<String>,
    vertex_meshes: Vec<AutosaveVertexMesh>,
}

pub(crate) struct AutosaveResult {
    errors: Vec<String>,
    warnings: Vec<String>,
    flushed_vertex_light_assets: usize,
    copied_replacement_assets: usize,
}

fn capture_autosave_snapshot(app: &AppState) -> AutosaveSnapshot {
    let mut replacement_assets = app.pending_replacement_assets.clone();
    for (key, bytes) in &app.editing.modified_entries {
        if app.editing.deleted_entries.contains(key) || asset_ext(key).is_none() {
            continue;
        }
        let name = app
            .editing
            .rows
            .iter()
            .find(|row| lower(&row.entry.name) == *key)
            .map(|row| row.entry.name.clone())
            .unwrap_or_else(|| key.clone());
        let normalized = normalize_legacy_light_mapper_asset_name(&name);
        replacement_assets.insert(lower(&normalized), (normalized, bytes.clone()));
    }
    let vertex_meshes = capture_save_vertex_meshes(app);
    let editing_asset = match app.editing.asset.as_ref() {
        Some(EditingAsset::Dff(dff)) if dff.dirty => Some(AutosaveEditingAsset::Dff {
            name: dff.name.clone(),
            raw: dff.raw.clone(),
            boolean_box: dff.boolean_box,
        }),
        Some(EditingAsset::Col(col)) if col.dirty => Some(AutosaveEditingAsset::Col {
            name: col.name.clone(),
            mesh: col.mesh.clone(),
            source_bytes: col.bytes.clone(),
            editing_shadow: col.editing_shadow,
            capsules: col.capsules.clone(),
            cuboids: col.cuboids.clone(),
            embedded_source_dff_bytes: col.embedded_source_dff_bytes.clone(),
        }),
        _ => None,
    };
    AutosaveSnapshot {
        root: app.root.clone(),
        captured_at: SystemTime::now(),
        zones: app.zones.clone(),
        eagle_zone_offsets: app.eagle_zone_offsets,
        placements: app.placements.clone(),
        definitions: app.definitions.clone(),
        readonly_definition_ids: app.readonly_definition_ids.clone(),
        element_states: app.element_states.clone(),
        lights: app.lights.clone(),
        material_emitters: app.material_emitters.clone(),
        material_classes: app.material_classes.clone(),
        safe_collisions: app.safe_collisions.clone(),
        shadow_casting: app.shadow_casting.clone(),
        water_planes: app.water_planes.clone(),
        replacement_assets,
        editing_asset,
        asset_deletes: app.pending_asset_deletes.clone(),
        vertex_meshes,
    }
}

fn serialize_autosave_editing_asset(
    asset: AutosaveEditingAsset,
) -> Result<(String, Vec<u8>), String> {
    match asset {
        AutosaveEditingAsset::Dff {
            name,
            mut raw,
            boolean_box,
        } => {
            let frame = Path::new(&name)
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("model");
            if !raw_mesh_is_safe_for_normalized_rewrite(&raw, frame) {
                return Err(format!(
                    "Could not autosave {name}: its frame/component hierarchy cannot be preserved by the normalized writer"
                ));
            }
            if let Some(cutter) = boolean_box {
                let removed = apply_dff_boolean_box(&mut raw, cutter);
                if removed > 0 {
                    for breakable in raw
                        .components
                        .iter_mut()
                        .filter_map(|component| component.breakable.as_mut())
                    {
                        breakable.stale = true;
                    }
                    compact_raw_vertices(&mut raw);
                    recalc_raw_normals(&mut raw);
                }
            }
            write_normalized_dff(&raw, frame)
                .map(|bytes| (name, bytes))
                .map_err(|err| format!("Could not autosave DFF: {err}"))
        }
        AutosaveEditingAsset::Col {
            name,
            mut mesh,
            source_bytes,
            editing_shadow,
            capsules,
            cuboids,
            embedded_source_dff_bytes,
        } => {
            if editing_shadow {
                std::mem::swap(&mut mesh.vertices, &mut mesh.shadow_vertices);
                std::mem::swap(&mut mesh.faces, &mut mesh.shadow_faces);
                mesh.bounds = bounds_from_vertices(&mesh.vertices);
            }
            let mut capsules = valid_capsules_for_mesh(&mut mesh, capsules);
            let mut cuboids = valid_cuboids_for_mesh(&mut mesh, cuboids);
            sync_generated_primitive_face_ranges(&mesh, &mut capsules, &mut cuboids)
                .map_err(|err| format!("Could not autosave COL {name}: {err}"))?;
            if !mesh.shadow_faces.is_empty() {
                snap_shadow_vertices_to_col_grid(&mut mesh.shadow_vertices)
                    .and_then(|_| {
                        orient_closed_shadow_components(
                            &mesh.shadow_vertices,
                            &mut mesh.shadow_faces,
                        )
                        .map(|_| ())
                    })
                    .map_err(|err| format!("Could not autosave COL {name}: {err}"))?;
            }
            let template = col_write_template(&source_bytes, &name);
            let mut col_bytes = write_col_mesh_from_template(&template, &mesh)
                .map_err(|err| format!("Could not autosave COL {name}: {err}"))?;
            set_col_model_names_from_entry(&mut col_bytes, &name);
            let bytes = if let Some(dff_bytes) = embedded_source_dff_bytes {
                replace_embedded_vehicle_collision(&dff_bytes, &col_bytes).map_err(|err| {
                    format!("Could not autosave embedded vehicle COL in {name}: {err}")
                })?
            } else {
                col_bytes
            };
            Ok((name, bytes))
        }
    }
}

fn write_autosave_snapshot(mut snapshot: AutosaveSnapshot) -> AutosaveResult {
    let autosave_root = autosave_root_path(&snapshot.root);
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    if let Some(asset) = snapshot.editing_asset.take() {
        match serialize_autosave_editing_asset(asset) {
            Ok((name, bytes)) => {
                let normalized = normalize_legacy_light_mapper_asset_name(&name);
                snapshot
                    .replacement_assets
                    .insert(lower(&normalized), (normalized, bytes));
            }
            Err(err) => errors.push(err),
        }
    }
    let meta_path = autosave_meta_path(&snapshot.root);
    // The metadata file is the snapshot commit marker. Remove the previous
    // marker before touching any payload so a crash or partial write cannot
    // advertise a mixed-generation autosave as complete.
    if meta_path.exists()
        && let Err(err) = fs::remove_file(&meta_path)
    {
        return AutosaveResult {
            errors: vec![format!("Could not clear {}: {err}", meta_path.display())],
            warnings,
            flushed_vertex_light_assets: 0,
            copied_replacement_assets: 0,
        };
    }
    if let Err(mut scene_errors) = write_scene_files_data(
        &snapshot.zones,
        snapshot.eagle_zone_offsets,
        &snapshot.placements,
        &snapshot.definitions,
        &snapshot.readonly_definition_ids,
        &snapshot.element_states,
        &autosave_root,
    ) {
        errors.append(&mut scene_errors);
    }
    if let Err(err) = seed_detached_eagle_scene(&snapshot.root, &autosave_root) {
        errors.push(err);
    }
    match live_scene_section_updates(
        &snapshot.lights,
        &snapshot.material_emitters,
        &snapshot.material_classes,
        &snapshot.safe_collisions,
        &snapshot.shadow_casting,
    )
    .and_then(|updates| update_eagle_scene_sections(&autosave_root, &updates))
    {
        Ok(()) => {}
        Err(err) => errors.push(err),
    }
    if let Err(err) = save_water_dat(
        &autosave_water_dat_path(&snapshot.root),
        &snapshot.water_planes,
    ) {
        errors.push(err);
    }
    let copied_replacement_assets = match pending_replacement_entries_from_parts(
        &snapshot.root,
        &snapshot.replacement_assets,
    ) {
        Ok(entries) => {
            let count = entries.len();
            let img_dir = autosave_root.join("imgs");
            match fs::create_dir_all(&img_dir)
                .map_err(|err| format!("{}: {err}", img_dir.display()))
                .and_then(|_| write_img_archive(&img_dir.join(REPLACEMENT_IMG), &entries))
            {
                // Writing an empty archive is intentional: it prevents stale
                // replacements from an older autosave from becoming the
                // topology source for this snapshot's vertex-light writeback.
                Ok(()) => count,
                Err(err) => {
                    errors.push(err);
                    0
                }
            }
        }
        Err(err) => {
            errors.push(err);
            0
        }
    };
    // The replacement snapshot is the source topology for dirty runtime
    // meshes. Apply vertex colors after writing it so writeback both reads the
    // newest DFF and remains the final update to the autosave archive.
    let flushed_vertex_light_assets = if errors.is_empty() {
        match write_vertex_lighting_meshes_resilient(
            &snapshot.root,
            &autosave_root,
            snapshot
                .vertex_meshes
                .iter()
                .map(|vertex| (vertex.key.as_str(), &vertex.mesh, vertex.options)),
        ) {
            Ok(outcome) => {
                warnings.extend(outcome.warnings);
                outcome.written
            }
            Err(err) => {
                errors.push(err);
                0
            }
        }
    } else {
        0
    };
    let readme_path = autosave_root.join("README.txt");
    if let Err(err) = fs::write(
        &readme_path,
        "Eagle Editor autosave snapshot. Restore prompts use this only when it is newer than the saved resource or WIP snapshot.\nNormal Save and Save WIP never write here.\nGenerated replacement DFF/TXD/COL archives are stored in imgs/.\n",
    ) {
        errors.push(format!("{}: {err}", readme_path.display()));
    }
    if let Err(err) = save_autosave_asset_deletes(&snapshot.root, &snapshot.asset_deletes) {
        errors.push(err);
    }
    let captured_since_epoch = snapshot
        .captured_at
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let meta = format!(
        "root={}\nunix_seconds={}\nunix_millis={}\n",
        snapshot.root.display(),
        captured_since_epoch.as_secs(),
        captured_since_epoch.as_millis()
    );
    if errors.is_empty() {
        let temporary = meta_path.with_extension(format!("tmp.{}", std::process::id()));
        match fs::write(&temporary, meta)
            .map_err(|err| format!("{}: {err}", temporary.display()))
            .and_then(|_| {
                fs::rename(&temporary, &meta_path)
                    .map_err(|err| format!("{}: {err}", meta_path.display()))
            }) {
            Ok(()) => {}
            Err(err) => {
                let _ = fs::remove_file(&temporary);
                errors.push(err);
            }
        }
    }
    AutosaveResult {
        errors,
        warnings,
        flushed_vertex_light_assets,
        copied_replacement_assets,
    }
}

fn poll_autosave(app: &mut AppState) -> bool {
    let Some(rx) = app.autosave_rx.take() else {
        return false;
    };
    match rx.try_recv() {
        Ok(result) => {
            app.autosave_next_at = get_time() + AUTOSAVE_INTERVAL_SECONDS;
            if result.errors.is_empty() {
                let warning_count = result.warnings.len();
                app.status_message = format!(
                    "Autosaved {}; wrote water.dat, {} vertex-light DFF(s), and stored {} replacement asset archive(s).",
                    ellipsize(app.root.to_string_lossy().as_ref(), 42),
                    result.flushed_vertex_light_assets,
                    result.copied_replacement_assets
                );
                if warning_count > 0 {
                    app.status_message.push_str(&format!(
                        " Completed with {warning_count} vertex-light warning(s); click the status bar for details."
                    ));
                    let mut log = vec![app.status_message.clone()];
                    log.extend(result.warnings);
                    set_save_log(app, "Autosave completed with warnings", log, false);
                }
            } else {
                // A failed snapshot did not make the captured dirty state
                // recoverable. Clear its baseline so the next interval retries.
                app.autosave_dirty_snapshot = None;
                app.status_message = format!(
                    "Autosave failed with {} error(s). Click status bar to view log.",
                    result.errors.len()
                );
                let mut log = result.errors;
                log.extend(result.warnings);
                set_save_log(app, "Autosave failed", log, true);
            }
            false
        }
        Err(mpsc::TryRecvError::Empty) => {
            app.autosave_rx = Some(rx);
            true
        }
        Err(mpsc::TryRecvError::Disconnected) => {
            app.autosave_dirty_snapshot = None;
            app.autosave_next_at = get_time() + AUTOSAVE_INTERVAL_SECONDS;
            app.status_message = "Autosave worker stopped unexpectedly.".to_string();
            false
        }
    }
}

fn cleanup_autosaves(root: &Path) -> Result<bool, String> {
    let autosave_dir = root.join(".eagle_autosave");
    if !autosave_dir.exists() {
        return Ok(false);
    }
    fs::remove_dir_all(&autosave_dir)
        .map(|_| true)
        .map_err(|err| format!("Could not remove {}: {err}", autosave_dir.display()))
}

fn poll_autosave_cleanup(app: &mut AppState) -> bool {
    let Some(rx) = app.autosave_cleanup_rx.take() else {
        return false;
    };
    match rx.try_recv() {
        Ok(Ok(removed)) => {
            // With the on-disk baseline gone, any still-dirty project state
            // must be eligible for a fresh autosave at a later interval.
            app.autosave_dirty_snapshot = None;
            app.autosave_next_at = get_time() + AUTOSAVE_INTERVAL_SECONDS;
            app.status_message = if removed {
                "Cleaned up autosaves for this project.".to_string()
            } else {
                "No autosaves were present for this project.".to_string()
            };
            false
        }
        Ok(Err(err)) => {
            app.status_message = err.clone();
            set_save_log(app, "Autosave cleanup failed", vec![err], true);
            false
        }
        Err(mpsc::TryRecvError::Empty) => {
            app.autosave_cleanup_rx = Some(rx);
            true
        }
        Err(mpsc::TryRecvError::Disconnected) => {
            app.status_message = "Autosave cleanup worker stopped unexpectedly.".to_string();
            false
        }
    }
}

pub(crate) fn request_autosave_cleanup(app: &mut AppState) {
    if app.autosave_rx.is_some()
        || app.autosave_cleanup_rx.is_some()
        || app.manual_save_job.is_some()
        || app.editing.save_rx.is_some()
        || app.editing.merge_rx.is_some()
        || app.editing.merge_apply_job.is_some()
        || app.vehicle_browser.collision_copy_rx.is_some()
    {
        app.status_message =
            "Wait for the active save or autosave job before cleaning autosaves.".to_string();
        return;
    }
    let root = app.root.clone();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = cleanup_autosaves(&root);
        let _ = tx.send(result);
    });
    app.autosave_cleanup_rx = Some(rx);
    app.status_message = "Cleaning up autosaves in the background...".to_string();
}

fn modified_time(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).and_then(|meta| meta.modified()).ok()
}

fn max_time(a: Option<SystemTime>, b: Option<SystemTime>) -> Option<SystemTime> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn latest_modified_under(path: &Path) -> Option<SystemTime> {
    if path.is_file() {
        return modified_time(path);
    }
    if !path.is_dir() {
        return None;
    }
    let mut latest = modified_time(path);
    for entry in WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        latest = max_time(latest, modified_time(entry.path()));
    }
    latest
}

pub(crate) fn latest_saved_scene_time(root: &Path) -> Option<SystemTime> {
    let mut latest = None;
    for path in [
        root.join("eagleZones.txt"),
        root.join("zones"),
        eagle_scene_path(root),
        // Compatibility until the first verified Resource Save migration.
        light_list_path(root),
        water_dat_path(root),
        wip_root_path(root),
    ] {
        latest = max_time(latest, latest_modified_under(&path));
    }
    latest
}

pub(crate) fn autosave_scene_time(root: &Path) -> Option<SystemTime> {
    let meta = autosave_meta_path(root);
    fs::read_to_string(&meta).ok().and_then(|text| {
        text.lines()
            .find_map(|line| line.strip_prefix("unix_millis="))
            .and_then(|value| value.parse::<u64>().ok())
            .map(|millis| SystemTime::UNIX_EPOCH + Duration::from_millis(millis))
    })
}

pub(crate) fn autosave_is_newer_than_saved(root: &Path) -> bool {
    let autosave_root = autosave_root_path(root);
    if !autosave_root.join("eagleZones.txt").is_file() || !autosave_root.join("zones").is_dir() {
        return false;
    }
    let Some(autosave_time) = autosave_scene_time(root) else {
        return false;
    };
    match latest_saved_scene_time(root) {
        Some(saved_time) => autosave_time > saved_time + Duration::from_secs(1),
        None => true,
    }
}

pub(crate) fn maybe_prompt_autosave_restore(app: &mut AppState) {
    if app.loaded_autosave || app.autosave_restore_prompted || app.confirm_dialog.is_some() {
        return;
    }
    app.autosave_restore_prompted = true;
    if !autosave_is_newer_than_saved(&app.root) {
        return;
    }
    app.confirm_dialog = Some(ConfirmDialog {
        action: ConfirmAction::RestoreAutosave(app.root.clone()),
        title: "Restore Autosave?".to_string(),
        body: "An autosave newer than the saved resource was found.".to_string(),
        detail: "Restore loads the autosave snapshot without modifying the main resource or WIP snapshot. Save or Save WIP afterward to keep it.".to_string(),
        primary_label: "Restore".to_string(),
        secondary_label: Some("Ignore".to_string()),
        secondary_action: Some(ConfirmAction::DismissAutosave),
    });
}

pub(crate) fn update_autosave(app: &mut AppState) {
    // Player and vehicle preview meshes are bundled runtime assets, not project
    // DFFs. Older bake paths could queue them and keep autosave permanently dirty.
    app.pending_vertex_light_meshes
        .retain(|key| !is_simulation_mesh_key(key));
    if poll_autosave_cleanup(app) {
        return;
    }
    if poll_autosave(app) {
        return;
    }
    if get_time() < app.autosave_next_at {
        return;
    }
    if !has_unautosaved_changes(app) {
        app.autosave_next_at = get_time() + AUTOSAVE_INTERVAL_SECONDS;
        return;
    }
    if app.load_job.is_some()
        || app.manual_save_job.is_some()
        || app.inspector_edit.is_some()
        || app.group_rename.is_some()
        || app.outliner_search_active
        || app.asset_browser.search_active
        || app.confirm_dialog.is_some()
        || app.save_as_dialog.is_some()
        || app.load_dialog.is_some()
        || app.preferences_dialog.is_some()
        || app.dff_picker_rx.is_some()
        || app.dff_repair_rx.is_some()
        || app.editing.save_rx.is_some()
        || app.editing.merge_rx.is_some()
        || app.editing.merge_apply_job.is_some()
        || app.editing.txd_import_rx.is_some()
        || app.editing.txd_refresh_job.is_some()
        || app.dff_prelight_import_dialog.is_some()
        || app.txd_cleanup_job.is_some()
        || app.asset_optimization_scan_rx.is_some()
        || app.asset_optimization_job.is_some()
        || app.purge_unused_job.is_some()
        || app.img_archive_rebalance_job.is_some()
        || app.object_bounds_fix_job.is_some()
        || app.corona_generation_job.is_some()
        || app.day_night_merge_job.is_some()
        || app.light_lod_job.is_some()
        || app.fracture_generation_job.is_some()
        || app.dff_geometry_job.is_some()
        || app.collision_generation_job.is_some()
        || app.shadow_mesh_generation_job.is_some()
        || app.lod_generation_job.is_some()
        || app.instance_lod_removal_job.is_some()
        || app.water_texture_conversion_job.is_some()
        || app.bake_job.is_some()
        || app.vehicle_browser.collision_copy_rx.is_some()
    {
        return;
    }
    let snapshot = capture_autosave_snapshot(app);
    // This is the autosave-clean baseline for the exact data handed to the
    // worker. Edits made after capture compare different and stay dirty.
    app.autosave_dirty_snapshot = Some(autosave_dirty_snapshot(app));
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = write_autosave_snapshot(snapshot);
        let _ = tx.send(result);
    });
    app.autosave_rx = Some(rx);
    app.autosave_next_at = get_time() + AUTOSAVE_INTERVAL_SECONDS;
    app.status_message = "Autosaving in the background...".to_string();
}

pub(crate) fn copy_dir_recursive(src: &Path, dst: &Path, errors: &mut Vec<String>) {
    if let Err(err) = fs::create_dir_all(dst) {
        errors.push(format!("{}: {err}", dst.display()));
        return;
    }
    let Ok(items) = fs::read_dir(src) else {
        errors.push(format!("{}: failed to read directory", src.display()));
        return;
    };
    for item in items.filter_map(Result::ok) {
        let src_path = item.path();
        let dst_path = dst.join(item.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path, errors);
        } else if let Err(err) = fs::copy(&src_path, &dst_path) {
            errors.push(format!(
                "{} -> {}: {err}",
                src_path.display(),
                dst_path.display()
            ));
        }
    }
}

pub(crate) fn copy_resource_shell(src_root: &Path, dst_root: &Path) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if let Err(err) = fs::create_dir_all(dst_root) {
        return Err(vec![format!("{}: {err}", dst_root.display())]);
    }
    for file in [
        "meta.xml",
        "eagleZones.txt",
        EAGLE_SCENE_FILE,
        "Light_Emitters.json",
        "Shadow_Casters.json",
        "Collision_Capsules.json",
        "Collision_Cuboids.json",
        "eagleMaterialClasses.json",
        "eagleSafeCollisions.json",
        "Light_List.xml",
        "water.dat",
    ] {
        let src = src_root.join(file);
        if src.exists() {
            let dst = dst_root.join(file);
            if let Err(err) = fs::copy(&src, &dst) {
                errors.push(format!("{} -> {}: {err}", src.display(), dst.display()));
            }
        }
    }
    if let Ok(entries) = fs::read_dir(src_root) {
        for entry in entries.filter_map(Result::ok) {
            let src = entry.path();
            if !src.is_file() {
                continue;
            }
            let Some(ext) = src.extension().and_then(|value| value.to_str()) else {
                continue;
            };
            if !ext.eq_ignore_ascii_case("map") && !ext.eq_ignore_ascii_case("definition") {
                continue;
            }
            let dst = dst_root.join(entry.file_name());
            if let Err(err) = fs::copy(&src, &dst) {
                errors.push(format!("{} -> {}: {err}", src.display(), dst.display()));
            }
        }
    }
    for dir in ["textures", "imgs", "zones"] {
        let src = src_root.join(dir);
        if src.is_dir() {
            copy_dir_recursive(&src, &dst_root.join(dir), &mut errors);
        }
    }
    if errors.is_empty() {
        if let Err(err) = repair_dff_archive_bounds_in_root(dst_root) {
            errors.push(err);
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn update_save_as_meta_root(root: &Path) -> Result<(), String> {
    let meta_path = root.join("meta.xml");
    let meta = fs::read_to_string(&meta_path).unwrap_or_default();
    let mut info_lines = Vec::new();
    let mut file_lines = Vec::new();
    let mut zone_wildcard_lines = Vec::new();
    for line in meta.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("<info") {
            info_lines.push(trimmed.to_string());
        } else if trimmed.starts_with("<file")
            && (trimmed.contains("src=\"zones/*/*.map\"")
                || trimmed.contains("src=\"zones/*/*.definition\""))
        {
            if !zone_wildcard_lines.iter().any(|line| line == trimmed) {
                zone_wildcard_lines.push(trimmed.to_string());
            }
        } else if trimmed.starts_with("<file") && !trimmed.contains("src=\"zones/") {
            file_lines.push(trimmed.to_string());
        }
    }
    if info_lines.is_empty() {
        info_lines.push(
            "<info type=\"script\" name=\"MTA:SA Eagle Edit\" author=\"MTA:SA Eagle Edit\" description=\"Map edited with MTA:SA Eagle Edit\" version=\"1.0\"/>"
                .to_string(),
        );
    }
    if file_lines.is_empty() {
        file_lines.push("<file src=\"textures/textures.txd\" type=\"client\" />".to_string());
        file_lines.push("<file src=\"eagleZones.txt\" type=\"client\" />".to_string());
        file_lines.push("<file src=\"water.dat\" type=\"client\" />".to_string());
        file_lines.push("<file src=\"imgs/*.img\" type=\"client\" />".to_string());
    }
    if !file_lines
        .iter()
        .any(|line| line.contains("src=\"water.dat\""))
    {
        file_lines.push("<file src=\"water.dat\" type=\"client\" />".to_string());
    }
    if !zone_wildcard_lines
        .iter()
        .any(|line| line.contains("src=\"zones/*/*.definition\""))
    {
        zone_wildcard_lines
            .push("<file src=\"zones/*/*.definition\" type=\"client\" />".to_string());
    }
    if !zone_wildcard_lines
        .iter()
        .any(|line| line.contains("src=\"zones/*/*.map\""))
    {
        zone_wildcard_lines.push("<file src=\"zones/*/*.map\" type=\"client\" />".to_string());
    }

    let mut out = String::from("<meta>\n");
    for line in info_lines {
        out.push_str("    ");
        out.push_str(&line);
        out.push('\n');
    }
    out.push('\n');
    for line in file_lines
        .iter()
        .filter(|line| !line.contains("imgs/*.img"))
    {
        out.push_str("    ");
        out.push_str(line);
        out.push('\n');
    }
    out.push('\n');
    for line in zone_wildcard_lines {
        out.push_str("    ");
        out.push_str(&line);
        out.push('\n');
    }
    out.push('\n');
    for line in file_lines.iter().filter(|line| line.contains("imgs/*.img")) {
        out.push_str("    ");
        out.push_str(line);
        out.push('\n');
    }
    out.push_str("\n</meta>\n");
    fs::write(&meta_path, out).map_err(|err| format!("{}: {err}", meta_path.display()))
}

#[allow(dead_code)]
pub(crate) fn update_save_as_meta(_app: &AppState, root: &Path) -> Result<(), String> {
    update_save_as_meta_root(root)
}

pub(crate) fn missing_col_count(app: &AppState) -> usize {
    app.definitions
        .values()
        .filter(|def| !app.readonly_definition_ids.contains(&def.id))
        .filter(|def| {
            def.attrs
                .get("col")
                .is_none_or(|value| value.trim().is_empty())
        })
        .count()
}

pub(crate) fn assign_missing_cols_from_dff(app: &mut AppState) -> usize {
    let mut assigned = 0;
    for def in app.definitions.values_mut() {
        if app.readonly_definition_ids.contains(&def.id) {
            continue;
        }
        if def
            .attrs
            .get("col")
            .is_none_or(|value| value.trim().is_empty())
        {
            let col = def
                .attrs
                .get("dff")
                .cloned()
                .filter(|value| !value.trim().is_empty())
                .map(|value| normalize_legacy_light_mapper_asset_stem(&value))
                .unwrap_or_else(|| def.id.clone());
            def.attrs.insert("col".to_string(), col);
            assigned += 1;
        }
    }
    assigned
}

pub(crate) fn remove_loose_asset_file(root: &Path, key: &str) -> bool {
    for dir in [
        "imgs",
        "Imgs",
        "models",
        "Models",
        "textures",
        "Textures",
        "txd_build",
        "TXD_Build",
    ] {
        let path = root.join(dir);
        if !path.exists() {
            continue;
        }
        for entry in WalkDir::new(&path).into_iter().filter_map(Result::ok) {
            if !entry.file_type().is_file() {
                continue;
            }
            let Some(name) = entry.path().file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            if lower(name) == key && fs::remove_file(entry.path()).is_ok() {
                return true;
            }
        }
    }
    false
}

struct PurgeUnusedFileResult {
    removed_wip_loose: usize,
    pruned_wip_archive: usize,
    errors: Vec<String>,
}

pub(crate) struct PurgeUnusedJob {
    rx: mpsc::Receiver<PurgeUnusedFileResult>,
    progress_rx: mpsc::Receiver<String>,
    unused_keys: Vec<String>,
    unused_definitions: Vec<String>,
    loose_count: usize,
    packed_count: usize,
    worker_result: Option<PurgeUnusedFileResult>,
    next_definition: usize,
    next_key: usize,
    removed_definitions: usize,
    started_at: Instant,
}

pub(crate) fn validation_operation_conflict(app: &AppState) -> Option<&'static str> {
    if let Some(conflict) = active_conflicting_save_job(app) {
        return Some(conflict);
    }
    if app.collision_generation_job.is_some() || app.shadow_mesh_generation_job.is_some() {
        Some("collision generation")
    } else if app.corona_generation_job.is_some() {
        Some("2DFX corona generation")
    } else if app.day_night_merge_job.is_some() {
        Some("day/night variant merge")
    } else if app.light_lod_job.is_some() || app.lod_generation_job.is_some() {
        Some("LOD generation")
    } else if app.fracture_generation_job.is_some() {
        Some("fracture generation")
    } else if app.dff_geometry_job.is_some() {
        Some("DFF geometry operation")
    } else if app.instance_lod_removal_job.is_some() {
        Some("instance LOD removal")
    } else {
        None
    }
}

pub(crate) fn request_purge_unused_assets(app: &mut AppState) {
    if app.purge_unused_job.is_some() {
        app.status_message = "Unused-asset purge is already running.".to_string();
        return;
    }
    if let Some(conflict) = validation_operation_conflict(app) {
        app.status_message =
            format!("Wait for the background {conflict} to finish before reviewing a purge.");
        return;
    }
    ensure_validation_cache(app);
    let Some(summary) = app.validation_cache.as_ref() else {
        return;
    };
    let assets = summary.unused_dffs.len() + summary.unused_cols.len() + summary.unused_txds.len();
    let definitions = summary.unused_definitions.len();
    if assets == 0 && definitions == 0 {
        app.status_message = "Nothing to purge: all definitions and assets are in use.".to_string();
        return;
    }
    let loose = summary
        .unused_dffs
        .iter()
        .chain(summary.unused_cols.iter())
        .chain(summary.unused_txds.iter())
        .filter(|key| {
            summary.loose_dffs.contains(*key)
                || summary.loose_cols.contains(*key)
                || summary.loose_txds.contains(*key)
        })
        .count();
    let packed = summary
        .unused_dffs
        .iter()
        .chain(summary.unused_cols.iter())
        .chain(summary.unused_txds.iter())
        .filter(|key| {
            summary.img_dffs.contains(*key)
                || summary.img_cols.contains(*key)
                || summary.img_txds.contains(*key)
        })
        .count();
    app.confirm_dialog = Some(ConfirmDialog {
        action: ConfirmAction::PurgeUnused,
        title: "Review Unused-Asset Purge".to_string(),
        body: format!(
            "Stage {assets} unused asset(s) and remove {definitions} unused definition(s)?"
        ),
        detail: format!(
            "{loose} loose and {packed} packed asset(s) are included. WIP loose files and WIP IMG entries are removed now; packed project IMG entries are removed on Resource Save. This external cleanup clears the entire undo and redo history."
        ),
        primary_label: "Purge and Clear History".to_string(),
        secondary_label: None,
        secondary_action: None,
    });
}

fn purge_wip_files(
    root: PathBuf,
    unused_keys: HashSet<String>,
    progress_tx: mpsc::Sender<String>,
) -> PurgeUnusedFileResult {
    let wip_root = wip_root_path(&root);
    let mut removed_wip_loose = 0usize;
    let mut errors = Vec::new();
    let _ = progress_tx.send("Scanning WIP asset folders...".to_string());
    for dir in [
        "imgs",
        "Imgs",
        "models",
        "Models",
        "textures",
        "Textures",
        "txd_build",
        "TXD_Build",
    ] {
        let path = wip_root.join(dir);
        if !path.exists() {
            continue;
        }
        for entry in WalkDir::new(&path).into_iter().filter_map(Result::ok) {
            if !entry.file_type().is_file() {
                continue;
            }
            let Some(name) = entry.path().file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if unused_keys.contains(&lower(name)) {
                match fs::remove_file(entry.path()) {
                    Ok(()) => removed_wip_loose += 1,
                    Err(err) => errors.push(format!("{}: {err}", entry.path().display())),
                }
            }
        }
    }

    let _ = progress_tx.send("Pruning unused entries from the WIP IMG...".to_string());
    let mut pruned_wip_archive = 0usize;
    if !unused_keys.is_empty() {
        if !wip_root.join("imgs").join(REPLACEMENT_IMG).is_file()
            && let Err(err) = copy_replacement_archive(&root, &wip_root)
        {
            errors.push(format!("Could not prepare WIP IMG: {err}"));
        }
        match remove_replacement_archive_entries(&wip_root, &unused_keys) {
            Ok(count) => pruned_wip_archive = count,
            Err(err) => errors.push(format!("Could not prune WIP IMG: {err}")),
        }
    }
    PurgeUnusedFileResult {
        removed_wip_loose,
        pruned_wip_archive,
        errors,
    }
}

pub(crate) fn start_purge_unused_assets(app: &mut AppState) {
    if app.purge_unused_job.is_some() {
        return;
    }
    if let Some(conflict) = validation_operation_conflict(app) {
        app.status_message =
            format!("Wait for the background {conflict} to finish before purging.");
        return;
    }
    ensure_validation_cache(app);
    let Some(summary) = app.validation_cache.as_ref() else {
        return;
    };
    let unused_keys = summary
        .unused_dffs
        .iter()
        .chain(summary.unused_cols.iter())
        .chain(summary.unused_txds.iter())
        .cloned()
        .collect::<HashSet<_>>();
    let loose_count = unused_keys
        .iter()
        .filter(|key| {
            summary.loose_dffs.contains(*key)
                || summary.loose_cols.contains(*key)
                || summary.loose_txds.contains(*key)
        })
        .count();
    let packed_count = unused_keys
        .iter()
        .filter(|key| {
            summary.img_dffs.contains(*key)
                || summary.img_cols.contains(*key)
                || summary.img_txds.contains(*key)
        })
        .count();
    let unused_definitions = summary.unused_definitions.clone();
    if unused_keys.is_empty() && unused_definitions.is_empty() {
        app.status_message = "Nothing to purge: validation results changed.".to_string();
        return;
    }

    let root = app.root.clone();
    let worker_keys = unused_keys.clone();
    let (tx, rx) = mpsc::channel();
    let (progress_tx, progress_rx) = mpsc::channel();
    thread::spawn(move || {
        let result = std::panic::catch_unwind(|| purge_wip_files(root, worker_keys, progress_tx))
            .unwrap_or_else(|_| PurgeUnusedFileResult {
                removed_wip_loose: 0,
                pruned_wip_archive: 0,
                errors: vec![
                    "Unused-asset purge worker panicked; review the WIP files before saving."
                        .to_string(),
                ],
            });
        let _ = tx.send(result);
    });
    app.purge_unused_job = Some(PurgeUnusedJob {
        rx,
        progress_rx,
        unused_keys: unused_keys.into_iter().collect(),
        unused_definitions,
        loose_count,
        packed_count,
        worker_result: None,
        next_definition: 0,
        next_key: 0,
        removed_definitions: 0,
        started_at: Instant::now(),
    });
    app.status_message = "Purging unused WIP assets in the background...".to_string();
}

pub(crate) fn update_purge_unused_assets(app: &mut AppState) {
    let Some(mut job) = app.purge_unused_job.take() else {
        return;
    };
    while let Ok(progress) = job.progress_rx.try_recv() {
        app.status_message = progress;
    }
    if job.worker_result.is_none() {
        match job.rx.try_recv() {
            Ok(result) => job.worker_result = Some(result),
            Err(mpsc::TryRecvError::Empty) => {
                app.purge_unused_job = Some(job);
                return;
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                app.status_message = "Unused-asset purge worker stopped unexpectedly.".to_string();
                set_save_log(
                    app,
                    "Unused-asset purge failed",
                    vec![app.status_message.clone()],
                    true,
                );
                return;
            }
        }
    }

    let frame_started = Instant::now();
    let mut applied = 0usize;
    while applied < LIVE_RESOURCE_REFRESH_BATCH_LIMIT
        && frame_started.elapsed() < LIVE_RESOURCE_REFRESH_FRAME_BUDGET
    {
        if let Some(id) = job.unused_definitions.get(job.next_definition) {
            job.removed_definitions += usize::from(app.definitions.remove(id).is_some());
            job.next_definition += 1;
            applied += 1;
            continue;
        }
        if let Some(key) = job.unused_keys.get(job.next_key) {
            app.pending_asset_deletes.insert(key.clone());
            job.next_key += 1;
            applied += 1;
            continue;
        }
        break;
    }
    if job.next_definition < job.unused_definitions.len() || job.next_key < job.unused_keys.len() {
        let total = job.unused_definitions.len() + job.unused_keys.len();
        let done = job.next_definition + job.next_key;
        app.status_message = format!("Applying purge results... {done}/{total}");
        app.purge_unused_job = Some(job);
        return;
    }

    let result = job.worker_result.take().unwrap();
    let changed = job.removed_definitions > 0 || !job.unused_keys.is_empty();
    if changed {
        clear_history_for_external_change(app);
    }
    if let Some(summary) = app.validation_cache.as_mut() {
        let keys = job.unused_keys.iter().cloned().collect::<HashSet<_>>();
        let definitions = job
            .unused_definitions
            .iter()
            .cloned()
            .collect::<HashSet<_>>();
        summary.unused_dffs.retain(|key| !keys.contains(key));
        summary.unused_cols.retain(|key| !keys.contains(key));
        summary.unused_txds.retain(|key| !keys.contains(key));
        summary
            .unused_definitions
            .retain(|id| !definitions.contains(id));
        summary.img_dffs.retain(|key| !keys.contains(key));
        summary.img_cols.retain(|key| !keys.contains(key));
        summary.img_txds.retain(|key| !keys.contains(key));
        summary.loose_dffs.retain(|key| !keys.contains(key));
        summary.loose_cols.retain(|key| !keys.contains(key));
        summary.loose_txds.retain(|key| !keys.contains(key));
        let max_scroll = validation_scroll_max_cached(summary);
        app.properties_scroll = app.properties_scroll.clamp(0.0, max_scroll);
    }
    let elapsed = job.started_at.elapsed().as_secs_f32();
    app.status_message = format!(
        "Purge staged in {elapsed:.1}s: removed {} definition(s), staged {} asset(s) ({} loose, {} packed), deleted {} WIP loose file(s), and pruned {} WIP IMG entry(s). Packed project IMG entries are removed on Resource Save.{}",
        job.removed_definitions,
        job.unused_keys.len(),
        job.loose_count,
        job.packed_count,
        result.removed_wip_loose,
        result.pruned_wip_archive,
        if changed {
            " Undo and redo history cleared."
        } else {
            ""
        }
    );
    let mut log = vec![app.status_message.clone()];
    log.extend(result.errors.iter().cloned());
    set_save_log(
        app,
        if result.errors.is_empty() {
            "Unused-asset purge completed"
        } else {
            "Unused-asset purge completed with warnings"
        },
        log,
        !result.errors.is_empty(),
    );
}

pub(crate) fn request_fix_lods(app: &mut AppState) {
    if let Some(conflict) = validation_operation_conflict(app) {
        app.status_message =
            format!("Wait for the background {conflict} to finish before repairing LODs.");
        return;
    }
    app.confirm_dialog = Some(ConfirmDialog {
        action: ConfirmAction::FixLods,
        title: "Review LOD Repairs".to_string(),
        body: "Apply all automatic LOD repairs to the loaded scene?".to_string(),
        detail: "This removes live orphan LOD placements, assigns matching uniqueID values to ambiguous repeated LOD pairs, and recalculates LOD/detail definition draw distances. The complete operation is recorded as one undo step.".to_string(),
        primary_label: "Apply LOD Repairs".to_string(),
        secondary_label: None,
        secondary_action: None,
    });
}

pub(crate) fn fix_lods_confirmed(app: &mut AppState) {
    let before = world_history_snapshot(app);
    let assigned = fix_repeated_lod_unique_ids(app);
    let removed = remove_orphan_lods(app);
    let (lod_distances, detail_distances) = fix_lod_distances(app);
    if removed > 0 {
        invalidate_outliner_labels(app);
        rebuild_outliner_filter(app);
    }
    if assigned > 0 || removed > 0 {
        rebuild_render_cells(app);
    }
    commit_world_history(app, "Fix LODs", before);
    app.status_message = format!(
        "LOD repairs complete: removed {removed} orphan LOD(s), paired {assigned} repeated LOD(s), and updated draw distance on {lod_distances} LOD and {detail_distances} detail definition(s)."
    );
}

fn texture_contents_equal(a: &TxdTextureContent, b: &TxdTextureContent) -> bool {
    a.width == b.width
        && a.height == b.height
        && a.sampler == b.sampler
        && a.has_alpha == b.has_alpha
        && a.rgba_len == b.rgba_len
        && a.fingerprint == b.fingerprint
}

fn txd_content_match(a: &[TxdTextureContent], b: &[TxdTextureContent]) -> Option<(bool, usize)> {
    if a.is_empty() || b.is_empty() {
        return None;
    }
    let by_name_b = b
        .iter()
        .map(|texture| (texture.name.as_str(), texture))
        .collect::<HashMap<_, _>>();
    for texture in a {
        if let Some(other) = by_name_b.get(texture.name.as_str())
            && !texture_contents_equal(texture, other)
        {
            // A same-name/different-image collision is never safe to merge.
            return None;
        }
    }
    let mut matched_b = HashSet::new();
    let mut matches = 0usize;
    for texture in a {
        if let Some((idx, _)) = b
            .iter()
            .enumerate()
            .find(|(idx, other)| !matched_b.contains(idx) && texture_contents_equal(texture, other))
        {
            matched_b.insert(idx);
            matches += 1;
        }
    }
    let exact = matches == a.len() && matches == b.len();
    let mostly_matching = matches >= 2
        && matches.saturating_mul(4) >= a.len().saturating_mul(3)
        && matches.saturating_mul(4) >= b.len().saturating_mul(3);
    (exact || mostly_matching).then_some((exact, matches))
}

fn merged_txd_content_set(
    retained: &[TxdTextureContent],
    donor: &[TxdTextureContent],
) -> Option<Vec<TxdTextureContent>> {
    txd_content_match(retained, donor)?;
    let mut merged = retained.to_vec();
    for texture in donor {
        if let Some(same_name) = merged.iter().find(|item| item.name == texture.name) {
            if !texture_contents_equal(same_name, texture) {
                return None;
            }
            continue;
        }
        if merged
            .iter()
            .any(|item| texture_contents_equal(item, texture))
        {
            continue;
        }
        merged.push(texture.clone());
    }
    merged.sort_by(|a, b| a.name.cmp(&b.name));
    Some(merged)
}

fn stable_bytes_fingerprint(bytes: &[u8]) -> [u64; 2] {
    let mut a = 0xcbf2_9ce4_8422_2325u64;
    let mut b = 0x9e37_79b9_7f4a_7c15u64;
    for (idx, value) in bytes.iter().copied().enumerate() {
        a ^= value as u64;
        a = a.wrapping_mul(0x1000_0000_01b3);
        b ^= (value as u64).wrapping_add((idx as u64).rotate_left(17));
        b = b.rotate_left(9).wrapping_mul(0x9ddf_ea08_eb38_2d69);
    }
    [a, b]
}

fn plan_txd_consolidations(
    prepared: &BTreeMap<String, Vec<TxdTextureContent>>,
    source_fingerprints: &BTreeMap<String, [u64; 2]>,
    referenced: &HashSet<String>,
    _warnings: &mut Vec<String>,
) -> Vec<TxdConsolidation> {
    let contents = prepared
        .iter()
        .filter(|(name, _)| referenced.contains(*name))
        .map(|(name, textures)| (name.clone(), textures.clone()))
        .collect::<BTreeMap<_, _>>();
    let names = contents.keys().cloned().collect::<Vec<_>>();
    let mut assigned = HashSet::<String>::new();
    let mut plans = Vec::new();
    for retained in &names {
        if assigned.contains(retained) {
            continue;
        }
        let Some(initial) = contents.get(retained) else {
            continue;
        };
        let mut merged = initial.clone();
        let mut donors = Vec::new();
        let mut all_exact = true;
        for donor in names.iter().filter(|name| *name > retained) {
            if assigned.contains(donor) {
                continue;
            }
            let Some(donor_contents) = contents.get(donor) else {
                continue;
            };
            let Some((exact, _)) = txd_content_match(&merged, donor_contents) else {
                continue;
            };
            let Some(next) = merged_txd_content_set(&merged, donor_contents) else {
                continue;
            };
            merged = next;
            all_exact &= exact;
            donors.push(donor.clone());
            assigned.insert(donor.clone());
        }
        if !donors.is_empty() {
            let group_names = std::iter::once(retained)
                .chain(donors.iter())
                .cloned()
                .collect::<Vec<_>>();
            plans.push(TxdConsolidation {
                retained: retained.clone(),
                donors,
                exact: all_exact,
                source_fingerprints: group_names
                    .into_iter()
                    .filter_map(|name| {
                        source_fingerprints
                            .get(&name)
                            .copied()
                            .map(|fingerprint| (name, fingerprint))
                    })
                    .collect(),
            });
        }
    }
    plans
}

fn merge_txd_bytes(
    mut retained_bytes: Vec<u8>,
    donor_bytes: &[u8],
) -> Result<(Vec<u8>, HashMap<String, String>, usize), String> {
    let mut retained = parse_txd_texture_contents(&retained_bytes)?;
    let donor = parse_txd_texture_contents_with_natives(donor_bytes)?;
    txd_content_match(&retained, &donor).ok_or_else(|| {
        "TXDs are no longer sufficiently similar or contain a name conflict".to_string()
    })?;
    let mut renames = HashMap::new();
    let mut added = 0usize;
    for texture in donor {
        if let Some(existing) = retained.iter().find(|item| item.name == texture.name) {
            if !texture_contents_equal(existing, &texture) {
                return Err(format!(
                    "{} has meaningfully different image data in both TXDs",
                    texture.name
                ));
            }
            continue;
        }
        if let Some(existing) = retained
            .iter()
            .find(|item| texture_contents_equal(item, &texture))
        {
            renames.insert(texture.name.clone(), existing.name.clone());
            continue;
        }
        retained_bytes =
            append_texture_native_to_txd(retained_bytes, &texture.native, &texture.name)?;
        retained.push(texture);
        added += 1;
    }
    Ok((retained_bytes, renames, added))
}

impl TxdCleanupJob {
    const FRAME_BUDGET: Duration = Duration::from_millis(6);

    fn new(app: &AppState, deep_optimize: bool, profile: TxdOptimizationProfile) -> Self {
        let mut definitions = Vec::new();
        for def in app.definitions.values() {
            let Some(txd_name) = definition_txd_name_from_attrs(def) else {
                continue;
            };
            definitions.push(TxdCleanupDefinition {
                dff_name: asset_key_opt(def.attrs.get("dff"), &def.id, ".dff"),
                txd_name: asset_key(txd_name, ".txd"),
            });
        }
        definitions.sort_by(|a, b| {
            a.txd_name
                .cmp(&b.txd_name)
                .then_with(|| a.dff_name.cmp(&b.dff_name))
        });
        definitions.dedup_by(|a, b| a.dff_name == b.dff_name && a.txd_name == b.txd_name);

        Self {
            root: app.root.clone(),
            wip_root: wip_root_path(&app.root),
            gta_sa_dir: app.gta_sa_dir.clone(),
            img_files: Vec::new(),
            img_index: 0,
            dff_map: HashMap::new(),
            txd_map: HashMap::new(),
            definitions,
            definition_index: 0,
            used_by_txd: HashMap::new(),
            dffs_by_txd: HashMap::new(),
            txd_names: Vec::new(),
            txd_index: 0,
            targets: Vec::new(),
            prepared_txds: BTreeMap::new(),
            source_fingerprints: BTreeMap::new(),
            excluded_assets: app.pending_asset_deletes.clone(),
            deep_optimize,
            scope: if deep_optimize {
                app.asset_optimization_scope
            } else {
                AssetOptimizationScope {
                    textures: true,
                    dffs: false,
                    cols: false,
                }
            },
            profile,
            skipped: 0,
            errors: Vec::new(),
            phase: TxdCleanupPhase::DiscoverArchives,
            status: "TXD cleanup: discovering archives".to_string(),
            started_at: Instant::now(),
        }
    }

    fn step(&mut self) -> Option<TxdCleanupPlan> {
        // Every caller runs this job on a worker. The small budget remains
        // useful for progress/status granularity and deterministic tests.
        let started = Instant::now();
        loop {
            let done = match self.phase {
                TxdCleanupPhase::DiscoverArchives => self.step_discover_archives(),
                TxdCleanupPhase::IndexArchives => self.step_index_archives(),
                TxdCleanupPhase::ScanDffs => self.step_scan_dffs(),
                TxdCleanupPhase::ScanTxds => self.step_scan_txds(),
            };
            if done {
                let mut purge_count = self.targets.iter().map(|target| target.purge_count).sum();
                let mut purge_bytes = self.targets.iter().map(|target| target.purge_bytes).sum();
                let mut format_fix_count = self
                    .targets
                    .iter()
                    .map(|target| target.format_fix_count)
                    .sum();
                let mut texture_rename_count = self
                    .targets
                    .iter()
                    .map(|target| target.texture_renames.len())
                    .sum();
                let mut profile_bytes_saved = self
                    .targets
                    .iter()
                    .map(|target| target.profile_bytes_saved)
                    .sum();
                let mut format_fixes: Vec<String> = self
                    .targets
                    .iter()
                    .flat_map(|target| {
                        target
                            .format_fixes
                            .iter()
                            .map(|fix| format!("{}: {fix}", target.txd_name))
                    })
                    .collect();
                let mut warnings: Vec<String> = self
                    .targets
                    .iter()
                    .flat_map(|target| {
                        target
                            .warnings
                            .iter()
                            .map(|warning| format!("{}: {warning}", target.txd_name))
                    })
                    .collect();
                let mut consolidations = if self.deep_optimize {
                    let referenced = self.used_by_txd.keys().cloned().collect::<HashSet<_>>();
                    plan_txd_consolidations(
                        &self.prepared_txds,
                        &self.source_fingerprints,
                        &referenced,
                        &mut warnings,
                    )
                } else {
                    Vec::new()
                };
                warnings.sort();
                warnings.dedup();
                let targets = if self.scope.textures {
                    std::mem::take(&mut self.targets)
                } else {
                    purge_count = 0;
                    purge_bytes = 0;
                    format_fix_count = 0;
                    texture_rename_count = 0;
                    profile_bytes_saved = 0;
                    format_fixes.clear();
                    consolidations.clear();
                    warnings.clear();
                    self.errors.clear();
                    Vec::new()
                };
                return Some(TxdCleanupPlan {
                    targets,
                    consolidations,
                    deep_optimize: self.deep_optimize,
                    scope: self.scope,
                    profile: self.profile,
                    profile_bytes_saved,
                    purge_count,
                    purge_bytes,
                    format_fix_count,
                    format_fixes,
                    texture_rename_count,
                    skipped: self.skipped,
                    errors: std::mem::take(&mut self.errors),
                    warnings,
                });
            }
            if started.elapsed() >= Self::FRAME_BUDGET {
                return None;
            }
        }
    }

    fn run_to_completion(mut self) -> TxdCleanupPlan {
        loop {
            if let Some(plan) = self.step() {
                return plan;
            }
        }
    }

    fn step_discover_archives(&mut self) -> bool {
        self.img_files = collect_resource_img_files(&self.root);
        self.img_files.extend(gta_sa_img_files(&self.gta_sa_dir));
        self.img_files.sort();
        self.img_files.dedup();
        self.phase = TxdCleanupPhase::IndexArchives;
        self.status = format!("TXD cleanup: indexing {} archive(s)", self.img_files.len());
        false
    }

    fn step_index_archives(&mut self) -> bool {
        if self.img_index < self.img_files.len() {
            let path = self.img_files[self.img_index].clone();
            self.img_index += 1;
            let is_project = path.starts_with(&self.root);
            let is_wip = path.starts_with(&self.wip_root);
            for entry in parse_img(&path) {
                let entry_name = lower(&entry.name);
                if self.excluded_assets.contains(&entry_name) {
                    continue;
                }
                if entry_name.ends_with(".dff") {
                    if is_project || !self.dff_map.contains_key(&entry_name) {
                        self.dff_map.insert(entry_name, entry);
                    }
                } else if entry_name.ends_with(".txd") {
                    if is_wip || (is_project && !self.txd_map.contains_key(&entry_name)) {
                        self.txd_map.insert(entry_name, entry);
                    }
                }
            }
            self.status = format!(
                "TXD cleanup: indexing archives ({}/{})",
                self.img_index,
                self.img_files.len()
            );
            return false;
        }
        for path in collect_resource_txd_files(&self.root) {
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let entry_name = asset_key(name, ".txd");
            if self.excluded_assets.contains(&entry_name) {
                continue;
            }
            let is_wip = path.starts_with(&self.wip_root);
            if is_wip || !self.txd_map.contains_key(&entry_name) {
                if let Some(entry) = loose_txd_entry(&path) {
                    self.txd_map.insert(entry_name, entry);
                }
            }
        }
        self.phase = TxdCleanupPhase::ScanDffs;
        self.status = "TXD cleanup: scanning DFF material textures".to_string();
        false
    }

    fn step_scan_dffs(&mut self) -> bool {
        if self.definition_index < self.definitions.len() {
            let def = &self.definitions[self.definition_index];
            self.definition_index += 1;
            let Some(entry) = self.dff_map.get(&def.dff_name) else {
                return false;
            };
            let raw = parse_dff_mesh(&read_img_entry(entry));
            let used = self.used_by_txd.entry(def.txd_name.clone()).or_default();
            self.dffs_by_txd
                .entry(def.txd_name.clone())
                .or_default()
                .insert(def.dff_name.clone());
            for texture in raw.material_textures {
                let texture = lower(texture.trim());
                if !texture.is_empty() {
                    used.insert(texture);
                }
            }
            self.status = format!(
                "TXD cleanup: scanning DFFs ({}/{})",
                self.definition_index,
                self.definitions.len()
            );
            return false;
        }
        self.txd_names = self.txd_map.keys().cloned().collect();
        self.txd_names.sort();
        self.phase = TxdCleanupPhase::ScanTxds;
        self.status = "TXD cleanup: comparing TXDs".to_string();
        false
    }

    fn step_scan_txds(&mut self) -> bool {
        if self.txd_index < self.txd_names.len() {
            let txd_name = self.txd_names[self.txd_index].clone();
            self.txd_index += 1;
            let Some(entry) = self.txd_map.get(&txd_name) else {
                self.skipped += 1;
                return false;
            };
            let bytes = read_txd_entry_bytes(entry);
            let source_fingerprint = stable_bytes_fingerprint(&bytes);
            self.source_fingerprints
                .insert(txd_name.clone(), source_fingerprint);
            let used_textures = self.used_by_txd.get(&txd_name).cloned();
            let purge_unused = used_textures.is_some();
            let used_textures = used_textures.unwrap_or_default();
            let purge_result = if purge_unused {
                purge_unused_texture_natives_from_txd(bytes, &used_textures)
            } else {
                Ok((bytes, 0, 0))
            };
            match purge_result {
                Ok((purged, removed, removed_bytes)) => {
                    let (name_fixed, texture_renames) =
                        match shorten_overlong_txd_texture_names(purged) {
                            Ok(result) => result,
                            Err(err) => {
                                self.errors.push(format!("{txd_name}: {err}"));
                                return false;
                            }
                        };
                    let optimized =
                        match optimize_txd_texture_formats_with_profile(name_fixed, self.profile) {
                            Ok(result) => result,
                            Err(err) => {
                                self.errors.push(format!("{txd_name}: {err}"));
                                return false;
                            }
                        };
                    let format_fixed = optimized.bytes;
                    let profile_reports = optimized.textures;
                    let warnings = optimized.warnings;
                    let profile_bytes_saved = profile_reports
                        .iter()
                        .map(|report| report.bytes_saved)
                        .sum();
                    let format_fixes = profile_reports
                        .iter()
                        .flat_map(|report| report.details.iter().cloned())
                        .collect::<Vec<_>>();
                    match parse_txd_texture_contents(&format_fixed) {
                        Ok(contents) => {
                            self.prepared_txds.insert(txd_name.clone(), contents);
                        }
                        Err(err) if self.deep_optimize => self
                            .errors
                            .push(format!("{txd_name}: consolidation scan skipped: {err}")),
                        Err(_) => {}
                    }
                    let format_fix_count = profile_reports.len();
                    if removed > 0
                        || format_fix_count > 0
                        || !texture_renames.is_empty()
                        || !warnings.is_empty()
                    {
                        let mut dff_names = self
                            .dffs_by_txd
                            .get(&txd_name)
                            .map(|names| names.iter().cloned().collect::<Vec<_>>())
                            .unwrap_or_default();
                        dff_names.sort();
                        self.targets.push(TxdCleanupTarget {
                            txd_name,
                            source_fingerprint,
                            prepared_bytes: format_fixed,
                            dff_names,
                            purge_count: removed,
                            purge_bytes: removed_bytes,
                            format_fix_count,
                            format_fixes,
                            profile_reports,
                            profile_bytes_saved,
                            texture_renames,
                            warnings,
                        });
                    }
                }
                Err(err) => self.errors.push(format!("{txd_name}: {err}")),
            }
            self.status = format!(
                "TXD cleanup: comparing TXDs ({}/{})",
                self.txd_index,
                self.txd_names.len()
            );
            return false;
        }
        true
    }
}

#[allow(dead_code)]
#[derive(Default)]
struct TxdConsolidationResult {
    groups: usize,
    removed_txds: usize,
    added_textures: usize,
    renamed_materials: usize,
    redirected_definitions: usize,
    rewritten_dffs: usize,
    errors: Vec<String>,
    warnings: Vec<String>,
}

#[derive(Default)]
struct GeometryCleanupResult {
    dffs_scanned: usize,
    dffs_repaired: usize,
    dff_vertices_removed: usize,
    dff_triangles_removed: usize,
    dff_materials_removed: usize,
    dff_triangles_reordered: usize,
    cols_scanned: usize,
    cols_repaired: usize,
    col_faces_removed: usize,
    col_vertices_removed: usize,
    col_coplanar_faces_removed: usize,
    col_winding_meshes_repaired: usize,
    col_faces_reoriented: usize,
    col_face_groups_generated: usize,
    col_faces_spatially_reordered: usize,
    col_alignments: usize,
    errors: Vec<String>,
    warnings: Vec<String>,
    staged_assets: Vec<(String, Vec<u8>)>,
    recompile_ids: BTreeSet<String>,
    collision_meshes: Vec<(String, CollisionMesh)>,
    referenced_dffs: BTreeSet<String>,
    referenced_cols: BTreeSet<String>,
}

#[derive(Clone)]
struct GeometryOptimizationContext {
    root: PathBuf,
    gta_sa_dir: PathBuf,
    definitions: Vec<Definition>,
    loaded_definition_ids: HashSet<String>,
    building_dffs: HashSet<String>,
    opaque_materials_by_dff: BTreeMap<String, BTreeSet<u16>>,
}

impl GeometryOptimizationContext {
    fn new(app: &AppState) -> Self {
        let loaded_definition_ids = app
            .placements
            .iter()
            .map(|placement| placement.id.clone())
            .collect::<HashSet<_>>();
        let mut opaque_materials_by_dff = BTreeMap::<String, BTreeSet<u16>>::new();
        let mut seen_mesh_scopes = HashSet::<String>::new();
        for placement in &app.placements {
            if !loaded_definition_ids.contains(&placement.id) {
                continue;
            }
            let dff_name = app
                .definitions
                .get(&placement.id)
                .map(|definition| {
                    asset_key_opt(definition.attrs.get("dff"), &placement.dff, ".dff")
                })
                .unwrap_or_else(|| asset_key(&placement.dff, ".dff"));
            let mesh_key = placement_mesh_key(placement, &app.definitions);
            if !seen_mesh_scopes.insert(mesh_key.clone()) {
                continue;
            }
            let Some(mesh) = app.meshes.get(&mesh_key) else {
                continue;
            };
            let opaque = mesh
                .parts
                .iter()
                .filter(|part| {
                    part.transparency == TransparencyMode::Opaque
                        && (part.texture_name.trim().is_empty()
                            || (app.options.textures && part.texture != 0 && !part.texture_missing))
                })
                .map(|part| part.material_index as u16)
                .collect::<BTreeSet<_>>();
            opaque_materials_by_dff
                .entry(dff_name)
                .and_modify(|known| known.retain(|material| opaque.contains(material)))
                .or_insert(opaque);
        }
        Self {
            root: app.root.clone(),
            gta_sa_dir: app.gta_sa_dir.clone(),
            definitions: app.definitions.values().cloned().collect(),
            loaded_definition_ids,
            building_dffs: building_dff_set(app),
            opaque_materials_by_dff,
        }
    }

    fn referenced_assets(&self) -> (BTreeSet<String>, BTreeSet<String>) {
        let mut dffs = BTreeSet::new();
        let mut cols = BTreeSet::new();
        for def in self
            .definitions
            .iter()
            .filter(|def| self.loaded_definition_ids.contains(&def.id))
        {
            dffs.insert(asset_key_opt(def.attrs.get("dff"), &def.id, ".dff"));
            let col = def
                .attrs
                .get("col")
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
                .map(|value| asset_key(value, ".col"))
                .unwrap_or_else(|| asset_key_opt(def.attrs.get("dff"), &def.id, ".col"));
            cols.insert(col);
        }
        (dffs, cols)
    }
}

fn find_dff_entry_for_geometry(
    context: &GeometryOptimizationContext,
    dff_name: &str,
) -> Option<ImgEntry> {
    let target = asset_key(dff_name, ".dff");
    if let Some(entry) = replacement_img_entry(&wip_root_path(&context.root), &target) {
        return Some(entry);
    }
    if let Some(entry) = replacement_img_entry(&context.root, &target) {
        return Some(entry);
    }
    for path in collect_resource_img_files(&context.root)
        .into_iter()
        .chain(gta_sa_img_files(&context.gta_sa_dir))
    {
        for entry in parse_img(&path) {
            if lower(&entry.name) == target {
                return Some(entry);
            }
        }
    }
    None
}

fn find_txd_entry_for_geometry(
    context: &GeometryOptimizationContext,
    txd_name: &str,
) -> Option<ImgEntry> {
    let target = asset_key(txd_name, ".txd");
    if let Some(entry) = replacement_img_entry(&wip_root_path(&context.root), &target) {
        return Some(entry);
    }
    if let Some(entry) = replacement_img_entry(&context.root, &target) {
        return Some(entry);
    }
    for path in collect_resource_txd_files(&context.root) {
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| asset_key(name, ".txd") == target)
        {
            return loose_txd_entry(&path);
        }
    }
    for path in collect_resource_img_files(&context.root)
        .into_iter()
        .chain(gta_sa_img_files(&context.gta_sa_dir))
    {
        for entry in parse_img(&path) {
            if lower(&entry.name) == target {
                return Some(entry);
            }
        }
    }
    None
}

fn refresh_collision_mesh_batch(
    collisions: &mut HashMap<String, CollisionMesh>,
    pending: &mut Vec<(String, CollisionMesh)>,
) -> usize {
    let started = Instant::now();
    let mut refreshed = 0usize;
    while refreshed < LIVE_RESOURCE_REFRESH_BATCH_LIMIT
        && (refreshed == 0 || started.elapsed() < LIVE_RESOURCE_REFRESH_FRAME_BUDGET)
    {
        let Some((name, mesh)) = pending.pop() else {
            break;
        };
        collisions.insert(name, mesh);
        refreshed += 1;
    }
    refreshed
}

#[derive(Default)]
struct ObjectBoundsFixResult {
    staged_assets: Vec<(String, Vec<u8>)>,
    collision_meshes: Vec<(String, CollisionMesh)>,
    assignments: Vec<(String, String)>,
    generated_cols: usize,
    repaired_cols: usize,
    unchanged_cols: usize,
    errors: Vec<String>,
}

pub(crate) struct ObjectBoundsFixJob {
    rx: mpsc::Receiver<ObjectBoundsFixResult>,
    result: Option<ObjectBoundsFixResult>,
    asset_index: usize,
    collision_index: usize,
    assignment_index: usize,
    started_at: Instant,
}

fn bounds_fix_col_value(col_name: &str) -> String {
    normalize_legacy_light_mapper_asset_stem(
        Path::new(col_name)
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or(col_name),
    )
}

fn run_object_bounds_fix(
    context: GeometryOptimizationContext,
    readonly_definition_ids: HashSet<String>,
) -> ObjectBoundsFixResult {
    let mut result = ObjectBoundsFixResult::default();
    let mut by_col = BTreeMap::<String, (BTreeSet<String>, Vec<String>)>::new();
    for definition in &context.definitions {
        if readonly_definition_ids.contains(&definition.id) {
            continue;
        }
        let dff_name = asset_key_opt(definition.attrs.get("dff"), &definition.id, ".dff");
        let col_name = definition
            .attrs
            .get("col")
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(|value| asset_key(value, ".col"))
            .unwrap_or_else(|| asset_key_opt(definition.attrs.get("dff"), &definition.id, ".col"));
        let group = by_col.entry(col_name).or_default();
        group.0.insert(dff_name);
        group.1.push(definition.id.clone());
    }

    let existing_cols = collect_resource_col_entries(&context.root);
    for (col_name, (dff_names, definition_ids)) in by_col {
        let mut combined_dff_bounds = None::<Bounds>;
        let mut missing_dff = false;
        for dff_name in &dff_names {
            let Some(entry) = find_dff_entry_for_geometry(&context, dff_name) else {
                result
                    .errors
                    .push(format!("{col_name}: paired DFF {dff_name} is unavailable"));
                missing_dff = true;
                continue;
            };
            let mut bytes = read_img_entry(&entry);
            bytes.truncate(dff_chunk_len(&bytes));
            let raw = parse_dff_mesh(&bytes);
            if raw.vertices.is_empty() {
                result
                    .errors
                    .push(format!("{col_name}: paired DFF {dff_name} has no geometry"));
                missing_dff = true;
                continue;
            }
            let bounds = bounds_from_vertices(&raw.vertices);
            combined_dff_bounds =
                Some(combined_dff_bounds.map_or(bounds, |current| union_bounds(current, bounds)));
        }
        let Some(dff_bounds) = combined_dff_bounds else {
            continue;
        };
        if missing_dff {
            result.errors.push(format!(
                "{col_name}: skipped because not every paired DFF could be measured"
            ));
            continue;
        }

        let existing = existing_cols.get(&col_name);
        let (mut mesh, source, generated) = if let Some(entry) = existing {
            let source = read_col_entry_bytes(entry);
            let model_names = collect_col_model_names(&source);
            if model_names.len() > 1 {
                result.errors.push(format!(
                    "{col_name}: contains multiple internal COL models and was left unchanged"
                ));
                continue;
            }
            let mesh = if let Some(mesh) = parse_col_mesh(&source, entry) {
                mesh
            } else if !col_validation_has_errors(&validate_col_for_game_load(&col_name, &source)) {
                CollisionMesh {
                    name: bounds_fix_col_value(&col_name),
                    spheres: Vec::new(),
                    boxes: Vec::new(),
                    vertices: Vec::new(),
                    faces: Vec::new(),
                    bounds: dff_bounds,
                    shadow_vertices: Vec::new(),
                    shadow_faces: Vec::new(),
                }
            } else {
                result
                    .errors
                    .push(format!("{col_name}: existing COL could not be parsed"));
                continue;
            };
            (mesh, source, false)
        } else {
            (
                CollisionMesh {
                    name: bounds_fix_col_value(&col_name),
                    spheres: Vec::new(),
                    boxes: Vec::new(),
                    vertices: Vec::new(),
                    faces: Vec::new(),
                    bounds: dff_bounds,
                    shadow_vertices: Vec::new(),
                    shadow_faces: Vec::new(),
                },
                col_regeneration_template(&[], &col_name),
                true,
            )
        };
        let has_collision_geometry =
            !mesh.vertices.is_empty() || !mesh.spheres.is_empty() || !mesh.boxes.is_empty();
        let header_bounds = if has_collision_geometry {
            union_bounds(
                dff_bounds,
                collision_mesh_bounds(&mesh.vertices, &mesh.spheres, &mesh.boxes),
            )
        } else {
            dff_bounds
        };
        mesh.bounds = header_bounds;
        let mut updated =
            match write_col_mesh_from_template_with_bounds(&source, &mesh, Some(header_bounds)) {
                Ok(bytes) => bytes,
                Err(error) => {
                    result
                        .errors
                        .push(format!("{col_name}: bounds rewrite failed: {error}"));
                    continue;
                }
            };
        if generated {
            set_col_model_names_from_entry(&mut updated, &col_name);
        }
        let issues = validate_col_for_game_load(&col_name, &updated);
        if col_validation_has_errors(&issues) {
            result.errors.push(format!(
                "{col_name}: bounds-fixed COL failed game-load validation"
            ));
            continue;
        }
        if generated || updated != source {
            result
                .staged_assets
                .push((col_name.clone(), updated.clone()));
            if generated {
                result.generated_cols += 1;
            } else {
                result.repaired_cols += 1;
            }
        } else {
            result.unchanged_cols += 1;
        }
        result.collision_meshes.push((col_name.clone(), mesh));
        let col_value = bounds_fix_col_value(&col_name);
        result
            .assignments
            .extend(definition_ids.into_iter().map(|id| (id, col_value.clone())));
    }

    result.staged_assets.sort_by(|a, b| a.0.cmp(&b.0));
    result.collision_meshes.sort_by(|a, b| a.0.cmp(&b.0));
    result.assignments.sort();
    if let Err(error) =
        upsert_replacement_assets(&wip_root_path(&context.root), &result.staged_assets)
    {
        result
            .errors
            .push(format!("Could not stage bounds-fixed COL assets: {error}"));
        result.staged_assets.clear();
        result.collision_meshes.clear();
        result.assignments.clear();
    }
    result
}

pub(crate) fn request_object_bounds_fix(app: &mut AppState) {
    if app.manual_save_job.is_some()
        || app.editing.save_rx.is_some()
        || app.autosave_rx.is_some()
        || app.autosave_cleanup_rx.is_some()
        || app.txd_cleanup_job.is_some()
        || app.asset_optimization_scan_rx.is_some()
        || app.asset_optimization_job.is_some()
        || app.object_bounds_fix_job.is_some()
        || app.dff_repair_rx.is_some()
        || app.collision_generation_job.is_some()
        || app.shadow_mesh_generation_job.is_some()
        || app.lod_generation_job.is_some()
    {
        app.status_message =
            "Object bounds repair cannot start while another asset writer is running.".to_string();
        return;
    }
    let context = GeometryOptimizationContext::new(app);
    let readonly = app.readonly_definition_ids.clone();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = std::panic::catch_unwind(|| run_object_bounds_fix(context, readonly))
            .unwrap_or_else(|panic| {
                let detail = panic
                    .downcast_ref::<&str>()
                    .map(|message| (*message).to_string())
                    .or_else(|| panic.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "unknown object bounds repair panic".to_string());
                ObjectBoundsFixResult {
                    errors: vec![format!("Object bounds worker crashed: {detail}")],
                    ..Default::default()
                }
            });
        let _ = tx.send(result);
    });
    app.object_bounds_fix_job = Some(ObjectBoundsFixJob {
        rx,
        result: None,
        asset_index: 0,
        collision_index: 0,
        assignment_index: 0,
        started_at: Instant::now(),
    });
    app.status_message =
        "Fix Object Bounds: checking definition DFF/COL pairings in background...".to_string();
}

pub(crate) fn update_object_bounds_fix(app: &mut AppState) {
    let Some(mut job) = app.object_bounds_fix_job.take() else {
        return;
    };
    if job.result.is_none() {
        match job.rx.try_recv() {
            Ok(result) => job.result = Some(result),
            Err(mpsc::TryRecvError::Empty) => {
                app.object_bounds_fix_job = Some(job);
                return;
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                app.status_message =
                    "Fix Object Bounds worker disconnected unexpectedly.".to_string();
                return;
            }
        }
    }
    let asset_total = job
        .result
        .as_ref()
        .expect("bounds fix result exists")
        .staged_assets
        .len();
    let asset_started = Instant::now();
    let mut assets_applied = 0usize;
    while job.asset_index < asset_total
        && assets_applied < LIVE_RESOURCE_REFRESH_BATCH_LIMIT
        && (assets_applied == 0 || asset_started.elapsed() < LIVE_RESOURCE_REFRESH_FRAME_BUDGET)
    {
        let (name, bytes) = job
            .result
            .as_ref()
            .and_then(|result| result.staged_assets.get(job.asset_index))
            .expect("bounds fix staged asset index is valid");
        app.pending_replacement_assets
            .insert(asset_key(name, ".col"), (name.clone(), bytes.clone()));
        job.asset_index += 1;
        assets_applied += 1;
    }
    if assets_applied > 0 {
        app.status_message = format!(
            "Fix Object Bounds: staging COL assets ({}/{})...",
            job.asset_index, asset_total
        );
        app.object_bounds_fix_job = Some(job);
        return;
    }

    let collision_total = job.collision_index
        + job
            .result
            .as_ref()
            .expect("bounds fix result exists")
            .collision_meshes
            .len();
    let collisions_refreshed = refresh_collision_mesh_batch(
        &mut app.collisions,
        &mut job
            .result
            .as_mut()
            .expect("bounds fix result exists")
            .collision_meshes,
    );
    if collisions_refreshed > 0 {
        job.collision_index += collisions_refreshed;
        app.status_message = format!(
            "Fix Object Bounds: refreshing collisions ({}/{})...",
            job.collision_index, collision_total
        );
        app.object_bounds_fix_job = Some(job);
        return;
    }

    let assignment_total = job
        .result
        .as_ref()
        .expect("bounds fix result exists")
        .assignments
        .len();
    let assignment_started = Instant::now();
    let mut assignments_applied = 0usize;
    while job.assignment_index < assignment_total
        && assignments_applied < LIVE_RESOURCE_REFRESH_BATCH_LIMIT
        && (assignments_applied == 0
            || assignment_started.elapsed() < LIVE_RESOURCE_REFRESH_FRAME_BUDGET)
    {
        let (id, col_value) = job
            .result
            .as_ref()
            .and_then(|result| result.assignments.get(job.assignment_index))
            .expect("bounds fix assignment index is valid");
        job.assignment_index += 1;
        if let Some(definition) = app.definitions.get_mut(id) {
            definition
                .attrs
                .insert("col".to_string(), col_value.clone());
            mark_definition_override_attr(definition, "col");
        }
        assignments_applied += 1;
    }
    if assignments_applied > 0 {
        app.status_message = format!(
            "Fix Object Bounds: assigning definition pairings ({}/{})...",
            job.assignment_index, assignment_total
        );
        app.object_bounds_fix_job = Some(job);
        return;
    }

    let result = job.result.as_ref().expect("bounds fix result exists");
    clear_history_for_external_change(app);
    invalidate_validation_cache(app);
    rebuild_render_cells(app);
    app.autosave_next_at = 0.0;
    let elapsed = job.started_at.elapsed().as_secs_f32();
    app.status_message = if result.errors.is_empty() {
        format!(
            "Fix Object Bounds finished in {elapsed:.1}s: generated {} missing COL(s), repaired {} COL bound header(s), verified {} unchanged COL(s), and assigned {} definition pairing(s). Save to apply. Undo history cleared.",
            result.generated_cols,
            result.repaired_cols,
            result.unchanged_cols,
            result.assignments.len()
        )
    } else {
        format!(
            "Fix Object Bounds finished in {elapsed:.1}s with {} issue(s): generated {}, repaired {}, verified {}, assigned {}. Click status for details.",
            result.errors.len(),
            result.generated_cols,
            result.repaired_cols,
            result.unchanged_cols,
            result.assignments.len()
        )
    };
    if !result.errors.is_empty() {
        let mut log = vec![app.status_message.clone()];
        log.extend(result.errors.iter().cloned());
        set_save_log(app, "Fix Object Bounds", log, true);
    }
}

#[derive(Default)]
struct BackgroundTxdCleanupResult {
    cleaned_txds: usize,
    removed_textures: usize,
    removed_bytes: usize,
    fixed_textures: usize,
    renamed_textures: usize,
    fixed_dffs: usize,
    consolidation: TxdConsolidationResult,
    definition_redirects: Vec<(String, String)>,
    pending_txd_writes: BTreeSet<String>,
    pending_asset_deletes: BTreeSet<String>,
    warnings: Vec<String>,
    errors: Vec<String>,
}

#[derive(Default)]
struct AssetOptimizationWorkerResult {
    txd: BackgroundTxdCleanupResult,
    geometry: GeometryCleanupResult,
    staged_txd_index: TxdTextureIndex,
}

fn merge_staged_txd_index(
    app: &mut AppState,
    staged: TxdTextureIndex,
    remove_txds: &BTreeSet<String>,
) {
    let mut staged_txds = staged
        .values()
        .flatten()
        .map(|texture| texture.txd_name.clone())
        .collect::<HashSet<_>>();
    staged_txds.extend(remove_txds.iter().cloned());
    app.txd_textures.retain(|_, entries| {
        entries.retain(|entry| !staged_txds.contains(&entry.txd_name));
        !entries.is_empty()
    });
    for (texture_name, mut entries) in staged {
        app.txd_textures
            .entry(texture_name)
            .or_default()
            .append(&mut entries);
    }
}

pub(crate) struct AssetOptimizationJob {
    rx: mpsc::Receiver<Result<AssetOptimizationWorkerResult, String>>,
    progress_rx: mpsc::Receiver<String>,
    cancelled: Arc<AtomicBool>,
    recompile_ids: Vec<String>,
    recompile_index: usize,
    collision_meshes: Vec<(String, CollisionMesh)>,
    collision_total: usize,
    result: Option<AssetOptimizationWorkerResult>,
    deep_optimize: bool,
    scope: AssetOptimizationScope,
    txd_profile: TxdOptimizationProfile,
    txd_bytes_saved: i64,
    started_at: Instant,
    progress_disconnected: bool,
    worker_succeeded: bool,
}

impl AssetOptimizationJob {
    fn new(app: &AppState, plan: TxdCleanupPlan) -> Self {
        let deep_optimize = plan.deep_optimize;
        let scope = plan.scope;
        let txd_profile = plan.profile;
        let txd_bytes_saved = plan.profile_bytes_saved;
        let context = GeometryOptimizationContext::new(app);
        let (tx, rx) = mpsc::channel();
        let (progress_tx, progress_rx) = mpsc::channel();
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = cancelled.clone();
        let readonly_definition_ids = app.readonly_definition_ids.clone();
        thread::spawn(move || {
            let result = std::panic::catch_unwind(|| {
                run_asset_optimization(
                    context,
                    readonly_definition_ids,
                    plan,
                    &worker_cancelled,
                    &progress_tx,
                )
            })
            .map_err(|panic| {
                panic
                    .downcast_ref::<&str>()
                    .map(|message| (*message).to_string())
                    .or_else(|| panic.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "unknown asset optimization panic".to_string())
            })
            .and_then(|result| result);
            let _ = tx.send(result);
        });
        Self {
            rx,
            progress_rx,
            cancelled,
            recompile_ids: Vec::new(),
            recompile_index: 0,
            collision_meshes: Vec::new(),
            collision_total: 0,
            result: None,
            deep_optimize,
            scope,
            txd_profile,
            txd_bytes_saved,
            started_at: Instant::now(),
            progress_disconnected: false,
            worker_succeeded: false,
        }
    }

    fn step(&mut self, app: &mut AppState) -> bool {
        let mut latest_progress = None;
        for _ in 0..64 {
            match self.progress_rx.try_recv() {
                Ok(message) => {
                    append_activity_entry(app, message.clone());
                    latest_progress = Some(message);
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.progress_disconnected = true;
                    break;
                }
            }
        }
        if let Some(message) = latest_progress.as_ref() {
            app.status_message = message.clone();
            app.activity_last_status = message.clone();
        }
        if self.result.is_none() {
            match self.rx.try_recv() {
                Ok(Ok(mut result)) => {
                    self.worker_succeeded = true;
                    for (id, retained_txd) in &result.txd.definition_redirects {
                        if let Some(definition) = app.definitions.get_mut(id) {
                            definition
                                .attrs
                                .insert("txd".to_string(), retained_txd.clone());
                        }
                    }
                    app.pending_txd_writes
                        .extend(result.txd.pending_txd_writes.iter().cloned());
                    for donor in &result.txd.pending_asset_deletes {
                        app.pending_txd_writes.remove(donor);
                        app.pending_asset_deletes.insert(donor.clone());
                    }
                    let remove_txds = result
                        .txd
                        .pending_txd_writes
                        .iter()
                        .chain(result.txd.pending_asset_deletes.iter())
                        .cloned()
                        .collect::<BTreeSet<_>>();
                    merge_staged_txd_index(
                        app,
                        std::mem::take(&mut result.staged_txd_index),
                        &remove_txds,
                    );
                    self.recompile_ids = result
                        .geometry
                        .recompile_ids
                        .iter()
                        .cloned()
                        .chain(
                            result
                                .txd
                                .definition_redirects
                                .iter()
                                .map(|(id, _)| id.clone()),
                        )
                        .collect::<BTreeSet<_>>()
                        .into_iter()
                        .collect();
                    self.collision_meshes = std::mem::take(&mut result.geometry.collision_meshes);
                    self.collision_total = self.collision_meshes.len();
                    self.result = Some(result);
                }
                Ok(Err(err)) => {
                    let mut result = AssetOptimizationWorkerResult::default();
                    result.geometry.errors.push(err);
                    self.result = Some(result);
                }
                Err(mpsc::TryRecvError::Empty) => {
                    if latest_progress.is_none() {
                        app.status_message = format!(
                            "{} ({}): worker running in background...",
                            if self.deep_optimize {
                                "Asset optimization"
                            } else {
                                "TXD cleanup"
                            },
                            txd_profile_label(self.txd_profile)
                        );
                    }
                    return false;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    let mut result = AssetOptimizationWorkerResult::default();
                    result
                        .geometry
                        .errors
                        .push("Geometry optimization worker disconnected".to_string());
                    self.result = Some(result);
                }
            }
        }
        let collisions_refreshed =
            refresh_collision_mesh_batch(&mut app.collisions, &mut self.collision_meshes);
        if collisions_refreshed > 0 {
            let completed = self.collision_total - self.collision_meshes.len();
            app.status_message = format!(
                "{} ({}): refreshing collisions ({}/{})",
                if self.deep_optimize {
                    "Asset optimization"
                } else {
                    "TXD cleanup"
                },
                txd_profile_label(self.txd_profile),
                completed,
                self.collision_total
            );
            return false;
        }
        let mesh_refresh_started = Instant::now();
        let mut meshes_refreshed = 0usize;
        while self.recompile_index < self.recompile_ids.len()
            && meshes_refreshed < LIVE_RESOURCE_REFRESH_BATCH_LIMIT
            && (meshes_refreshed == 0
                || mesh_refresh_started.elapsed() < LIVE_RESOURCE_REFRESH_FRAME_BUDGET)
        {
            let id = self.recompile_ids[self.recompile_index].clone();
            self.recompile_index += 1;
            let _ = recompile_definition_mesh(app, &id);
            meshes_refreshed += 1;
        }
        if meshes_refreshed > 0 {
            app.status_message = format!(
                "{} ({}): refreshing meshes ({}/{})",
                if self.deep_optimize {
                    "Asset optimization"
                } else {
                    "TXD cleanup"
                },
                txd_profile_label(self.txd_profile),
                self.recompile_index,
                self.recompile_ids.len()
            );
            return false;
        }
        if !self.progress_disconnected {
            return false;
        }
        let result = self.result.as_mut().expect("optimization result is set");
        if self.worker_succeeded && self.deep_optimize {
            let mut rebuild_needed = result.geometry.dffs_repaired > 0
                || result.geometry.cols_repaired > 0
                || !result.txd.definition_redirects.is_empty();
            let before_meshes = app.meshes.len();
            let before_collisions = app.collisions.len();
            if self.scope.dffs {
                app.meshes.retain(|key, _| {
                    loaded_mesh_is_required(key, &result.geometry.referenced_dffs)
                });
            }
            if self.scope.cols {
                app.collisions
                    .retain(|key, _| result.geometry.referenced_cols.contains(key));
            }
            if app.meshes.len() != before_meshes || app.collisions.len() != before_collisions {
                rebuild_needed = true;
            }
            if rebuild_needed {
                rebuild_render_cells(app);
            }
        }
        invalidate_validation_cache(app);
        result.geometry.warnings.sort();
        result.geometry.warnings.dedup();
        result.geometry.errors.sort();
        result.geometry.errors.dedup();
        let elapsed = self.started_at.elapsed().as_secs_f32();
        app.status_message = if !self.deep_optimize
            && result.geometry.errors.is_empty()
            && result.txd.errors.is_empty()
        {
            format!(
                "TXD cleanup ({}) finished in {elapsed:.1}s: staged {} TXD(s), removed {} unused texture(s) ({}), repaired {} texture payload(s), renamed {} texture(s), and updated {} DFF texture reference(s). Save promotes staged changes.",
                txd_profile_label(self.txd_profile),
                result.txd.cleaned_txds,
                result.txd.removed_textures,
                format_bytes(result.txd.removed_bytes),
                result.txd.fixed_textures,
                result.txd.renamed_textures,
                result.txd.fixed_dffs,
            )
        } else if result.geometry.errors.is_empty() && result.txd.errors.is_empty() {
            format!(
                "Asset optimization ({}) finished in {elapsed:.1}s: TXD policy {}, staged {} TXD(s), removed {} unused texture(s) ({}), consolidated {} group(s) and {} donor TXD(s), repaired {} texture payload(s), renamed {} texture(s), and updated {} texture-reference DFF(s); repaired {}/{} DFF(s), removing {} vertex/vertices, {} triangle(s), and {} material slot(s) and reordering {} opaque triangle position(s); repaired {}/{} COL(s), removed {} collision face(s) ({} by coplanar reduction) and {} redundant collision vertex/vertices, reoriented {} face(s) across {} inverted ground mesh(es), generated {} spatial face group(s) with {} face reorder(s), and corrected {} COL alignment(s). Save promotes staged changes.",
                txd_profile_label(self.txd_profile),
                signed_texture_savings(self.txd_bytes_saved),
                result.txd.cleaned_txds,
                result.txd.removed_textures,
                format_bytes(result.txd.removed_bytes),
                result.txd.consolidation.groups,
                result.txd.consolidation.removed_txds,
                result.txd.fixed_textures,
                result.txd.renamed_textures,
                result.txd.fixed_dffs,
                result.geometry.dffs_repaired,
                result.geometry.dffs_scanned,
                result.geometry.dff_vertices_removed,
                result.geometry.dff_triangles_removed,
                result.geometry.dff_materials_removed,
                result.geometry.dff_triangles_reordered,
                result.geometry.cols_repaired,
                result.geometry.cols_scanned,
                result.geometry.col_faces_removed,
                result.geometry.col_coplanar_faces_removed,
                result.geometry.col_vertices_removed,
                result.geometry.col_faces_reoriented,
                result.geometry.col_winding_meshes_repaired,
                result.geometry.col_face_groups_generated,
                result.geometry.col_faces_spatially_reordered,
                result.geometry.col_alignments,
            )
        } else if self.deep_optimize {
            format!(
                "Asset optimization ({}) finished in {elapsed:.1}s with {} issue(s); TXD texture policy {}; repaired {} DFF(s) and {} COL(s). Click status for details.",
                txd_profile_label(self.txd_profile),
                result.geometry.errors.len() + result.txd.errors.len(),
                signed_texture_savings(self.txd_bytes_saved),
                result.geometry.dffs_repaired,
                result.geometry.cols_repaired,
            )
        } else {
            format!(
                "TXD cleanup ({}) finished in {elapsed:.1}s with {} issue(s). Click status for details.",
                txd_profile_label(self.txd_profile),
                result.geometry.errors.len() + result.txd.errors.len(),
            )
        };
        append_activity_entry(app, app.status_message.clone());
        app.activity_last_status = app.status_message.clone();
        let mut log = vec![app.status_message.clone()];
        log.extend(result.txd.warnings.iter().take(100).cloned());
        log.extend(result.geometry.warnings.iter().take(100).cloned());
        log.extend(result.txd.errors.iter().cloned());
        log.extend(result.geometry.errors.iter().cloned());
        if log.len() > 1 {
            set_save_log(
                app,
                if self.deep_optimize {
                    "Asset Optimization"
                } else {
                    "TXD Cleanup"
                },
                log,
                !result.geometry.errors.is_empty() || !result.txd.errors.is_empty(),
            );
        }
        if self.worker_succeeded {
            // Persist matching XML definitions and staged assets together on
            // the next frame, minimizing the recovery window after TXD
            // reference redirects.
            app.autosave_next_at = 0.0;
        }
        true
    }
}

impl Drop for AssetOptimizationJob {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

fn run_asset_optimization(
    mut context: GeometryOptimizationContext,
    readonly_definition_ids: HashSet<String>,
    plan: TxdCleanupPlan,
    cancelled: &AtomicBool,
    progress_tx: &mpsc::Sender<String>,
) -> Result<AssetOptimizationWorkerResult, String> {
    let deep_optimize = plan.deep_optimize;
    let scope = plan.scope;
    fn missing_dff_texture_names(bytes: &[u8], renames: &HashMap<String, String>) -> Vec<String> {
        let present = parse_dff_mesh(bytes)
            .material_textures
            .into_iter()
            .map(|name| lower(name.trim()))
            .filter(|name| !name.is_empty())
            .collect::<HashSet<_>>();
        renames
            .keys()
            .filter(|name| !present.contains(lower(name).as_str()))
            .cloned()
            .collect()
    }

    let progress = |message: String| {
        let _ = progress_tx.send(message);
    };
    let mut txd_result = if scope.textures {
        BackgroundTxdCleanupResult {
            warnings: plan.warnings.clone(),
            errors: plan.errors.clone(),
            ..Default::default()
        }
    } else {
        BackgroundTxdCleanupResult::default()
    };
    let mut txd_updates = BTreeMap::<String, Vec<u8>>::new();
    let mut dff_renames = BTreeMap::<String, HashMap<String, String>>::new();
    let mut required_dff_renames = BTreeMap::<String, HashMap<String, String>>::new();
    let targets = if scope.textures {
        plan.targets
    } else {
        Vec::new()
    };
    let target_count = targets.len();

    progress(format!(
        "TXD phase: validating and preparing {target_count} archive(s)"
    ));
    for (index, target) in targets.into_iter().enumerate() {
        if cancelled.load(Ordering::Relaxed) {
            return Err("Asset optimization was cancelled".to_string());
        }
        let Some(entry) = find_txd_entry_for_geometry(&context, &target.txd_name) else {
            let message = format!("TXD {}: unavailable; skipped", target.txd_name);
            progress(message.clone());
            txd_result.errors.push(message);
            continue;
        };
        let current = read_txd_entry_bytes(&entry);
        if stable_bytes_fingerprint(&current) != target.source_fingerprint {
            let message = format!("TXD {}: changed since scan; skipped", target.txd_name);
            progress(message.clone());
            txd_result.errors.push(message);
            continue;
        }
        if target.purge_count == 0
            && target.format_fix_count == 0
            && target.texture_renames.is_empty()
        {
            progress(format!(
                "TXD {}/{} {}: already optimal",
                index + 1,
                target_count,
                target.txd_name
            ));
            continue;
        }
        txd_result.cleaned_txds += 1;
        txd_result.removed_textures += target.purge_count;
        txd_result.removed_bytes += target.purge_bytes;
        txd_result.fixed_textures += target.format_fix_count;
        txd_result.renamed_textures += target.texture_renames.len();
        txd_result
            .pending_txd_writes
            .insert(target.txd_name.clone());
        for dff_name in target.dff_names {
            dff_renames
                .entry(dff_name)
                .or_default()
                .extend(target.texture_renames.clone());
        }
        txd_updates.insert(target.txd_name.clone(), target.prepared_bytes);
        progress(format!(
            "TXD {}/{} {}: prepared ({} texture(s) removed, {} repaired, {})",
            index + 1,
            target_count,
            target.txd_name,
            target.purge_count,
            target.format_fix_count,
            format_bytes(target.purge_bytes)
        ));
    }

    let consolidations = if scope.textures {
        plan.consolidations
    } else {
        Vec::new()
    };
    let consolidation_count = consolidations.len();
    for (index, consolidation) in consolidations.into_iter().enumerate() {
        if cancelled.load(Ordering::Relaxed) {
            return Err("Asset optimization was cancelled".to_string());
        }
        let retained_name = consolidation.retained.clone();
        let expected_group_size = consolidation.donors.len() + 1;
        if consolidation.source_fingerprints.len() != expected_group_size {
            let error = format!(
                "{retained_name}: consolidation has incomplete source fingerprints; skipped"
            );
            progress(error.clone());
            txd_result.errors.push(error);
            continue;
        }
        let mut stale_source = None;
        for (name, expected) in &consolidation.source_fingerprints {
            let Some(entry) = find_txd_entry_for_geometry(&context, name) else {
                stale_source = Some(format!("{name}: consolidation source is unavailable"));
                break;
            };
            if stable_bytes_fingerprint(&read_txd_entry_bytes(&entry)) != *expected {
                stale_source = Some(format!("{name}: changed since consolidation scan"));
                break;
            }
        }
        if let Some(error) = stale_source {
            progress(format!(
                "TXD {retained_name}: consolidation skipped: {error}"
            ));
            txd_result.errors.push(error);
            continue;
        }
        let mut merged = if let Some(bytes) = txd_updates.get(&retained_name) {
            bytes.clone()
        } else {
            let Some(entry) = find_txd_entry_for_geometry(&context, &retained_name) else {
                let message = format!("TXD {retained_name}: consolidation source unavailable");
                progress(message.clone());
                txd_result.errors.push(message);
                continue;
            };
            read_txd_entry_bytes(&entry)
        };
        let mut donor_renames = BTreeMap::<String, HashMap<String, String>>::new();
        let mut added = 0usize;
        let mut group_error = None;
        for donor in &consolidation.donors {
            let donor_bytes = if let Some(bytes) = txd_updates.get(donor) {
                bytes.clone()
            } else {
                let Some(entry) = find_txd_entry_for_geometry(&context, donor) else {
                    group_error = Some(format!("{donor}: donor TXD is unavailable"));
                    break;
                };
                read_txd_entry_bytes(&entry)
            };
            match merge_txd_bytes(std::mem::take(&mut merged), &donor_bytes) {
                Ok((next, renames, count)) => {
                    merged = next;
                    donor_renames.insert(donor.clone(), renames);
                    added += count;
                }
                Err(err) => {
                    group_error = Some(format!(
                        "{retained_name} + {donor}: consolidation revalidation failed: {err}"
                    ));
                    break;
                }
            }
        }
        if let Some(error) = group_error {
            progress(format!(
                "TXD {retained_name}: consolidation skipped: {error}"
            ));
            txd_result.errors.push(error);
            continue;
        }
        if let Err(error) = parse_txd_texture_contents(&merged) {
            let error = format!("{retained_name}: merged TXD failed validation: {error}");
            progress(error.clone());
            txd_result.errors.push(error);
            continue;
        }

        let donor_set = consolidation.donors.iter().cloned().collect::<HashSet<_>>();
        let affected = context
            .definitions
            .iter()
            .filter_map(|definition| {
                let current = definition_txd_name_from_attrs(definition)?;
                let key = asset_key(current, ".txd");
                donor_set.contains(&key).then_some((
                    definition.id.clone(),
                    asset_key_opt(definition.attrs.get("dff"), &definition.id, ".dff"),
                    key,
                ))
            })
            .collect::<Vec<_>>();
        if affected
            .iter()
            .any(|(id, _, _)| readonly_definition_ids.contains(id))
        {
            let warning = format!(
                "{retained_name}: consolidation skipped because a donor is referenced by a read-only definition"
            );
            progress(warning.clone());
            txd_result.warnings.push(warning);
            continue;
        }
        let mut group_dff_renames = BTreeMap::<String, HashMap<String, String>>::new();
        let mut group_error = None;
        for (_, dff_name, donor) in &affected {
            let Some(renames) = donor_renames.get(donor) else {
                continue;
            };
            let target = group_dff_renames.entry(dff_name.clone()).or_default();
            for (old, new) in renames {
                let conflicts_with_group = target.get(old).is_some_and(|existing| existing != new);
                let conflicts_with_existing = dff_renames
                    .get(dff_name)
                    .and_then(|existing| existing.get(old))
                    .is_some_and(|existing| existing != new);
                if conflicts_with_group || conflicts_with_existing {
                    group_error = Some(format!(
                        "{dff_name}: texture {old} maps to multiple retained names"
                    ));
                    break;
                }
                target.insert(old.clone(), new.clone());
            }
        }
        if let Some(error) = group_error {
            progress(error.clone());
            txd_result.errors.push(error);
            continue;
        }

        // A consolidation is all-or-nothing: prove that every required DFF
        // texture reference can be rewritten before recording redirects or
        // donor deletions.
        for (dff_name, renames) in &group_dff_renames {
            let Some(entry) = find_dff_entry_for_geometry(&context, dff_name) else {
                group_error = Some(format!(
                    "{dff_name}: required DFF is unavailable; consolidation cancelled"
                ));
                break;
            };
            let mut bytes = read_img_entry(&entry);
            bytes.truncate(dff_chunk_len(&bytes));
            let missing = missing_dff_texture_names(&bytes, renames);
            if !missing.is_empty() {
                group_error = Some(format!(
                    "{dff_name}: required texture reference(s) {} are missing; consolidation cancelled",
                    missing.join(", ")
                ));
                break;
            }
        }
        if let Some(error) = group_error {
            progress(error.clone());
            txd_result.errors.push(error);
            continue;
        }

        for (dff_name, renames) in group_dff_renames {
            required_dff_renames
                .entry(dff_name.clone())
                .or_default()
                .extend(renames.clone());
            dff_renames.entry(dff_name).or_default().extend(renames);
        }
        let retained_stem = txd_stem(&retained_name);
        for (id, _, _) in &affected {
            if let Some(definition) = context
                .definitions
                .iter_mut()
                .find(|definition| definition.id == *id)
            {
                definition
                    .attrs
                    .insert("txd".to_string(), retained_stem.clone());
                txd_result
                    .definition_redirects
                    .push((id.clone(), retained_stem.clone()));
            }
        }
        txd_updates.insert(retained_name.clone(), merged);
        txd_result.pending_txd_writes.insert(retained_name.clone());
        for donor in &consolidation.donors {
            txd_updates.remove(donor);
            txd_result.pending_txd_writes.remove(donor);
            txd_result.pending_asset_deletes.insert(donor.clone());
        }
        txd_result.consolidation.groups += 1;
        txd_result.consolidation.removed_txds += consolidation.donors.len();
        txd_result.consolidation.added_textures += added;
        txd_result.consolidation.redirected_definitions += affected.len();
        if !consolidation.exact {
            txd_result.warnings.push(format!(
                "{retained_name}: merged unique textures from {} similar TXD(s)",
                consolidation.donors.len()
            ));
        }
        progress(format!(
            "TXD merge {}/{} {}: consolidated {} donor(s), added {} unique texture(s)",
            index + 1,
            consolidation_count,
            retained_name,
            consolidation.donors.len(),
            added
        ));
    }

    let mut geometry = if deep_optimize {
        if cancelled.load(Ordering::Relaxed) {
            return Err("Asset optimization was cancelled before geometry cleanup".to_string());
        }
        let (mut dffs, mut cols) = context.referenced_assets();
        if !scope.dffs {
            dffs.clear();
        }
        if !scope.cols {
            cols.clear();
        }
        progress(format!(
            "Geometry phase: optimizing {} DFF(s) and {} COL(s)",
            dffs.len(),
            cols.len()
        ));
        apply_geometry_cleanup_filtered(&context, &dffs, &cols, cancelled, &progress)
    } else {
        GeometryCleanupResult::default()
    };
    if cancelled.load(Ordering::Relaxed) {
        return Err("Asset optimization was cancelled before staging".to_string());
    }

    let mut staged_assets = geometry
        .staged_assets
        .drain(..)
        .collect::<BTreeMap<String, Vec<u8>>>();
    for (dff_name, renames) in dff_renames {
        if renames.is_empty() {
            continue;
        }
        let geometry_staged = staged_assets.contains_key(&dff_name);
        let mut bytes = if let Some(bytes) = staged_assets.remove(&dff_name) {
            bytes
        } else {
            let Some(entry) = find_dff_entry_for_geometry(&context, &dff_name) else {
                let error = format!("{dff_name}: DFF is unavailable for texture redirection");
                progress(error.clone());
                if required_dff_renames.contains_key(&dff_name) {
                    return Err(error);
                }
                txd_result.errors.push(error);
                continue;
            };
            let mut bytes = read_img_entry(&entry);
            bytes.truncate(dff_chunk_len(&bytes));
            bytes
        };
        if let Some(required) = required_dff_renames.get(&dff_name) {
            let missing = missing_dff_texture_names(&bytes, required);
            if !missing.is_empty() {
                return Err(format!(
                    "{dff_name}: required consolidation texture reference(s) {} disappeared during DFF cleanup",
                    missing.join(", ")
                ));
            }
        }
        let changed = rename_dff_material_textures(&mut bytes, &renames);
        if changed == 0 {
            txd_result.warnings.push(format!(
                "{dff_name}: no matching material reference found for renamed texture(s)"
            ));
        } else {
            txd_result.fixed_dffs += 1;
            txd_result.consolidation.renamed_materials += changed;
            for definition in &context.definitions {
                if context.loaded_definition_ids.contains(&definition.id)
                    && asset_key_opt(definition.attrs.get("dff"), &definition.id, ".dff")
                        == dff_name
                {
                    geometry.recompile_ids.insert(definition.id.clone());
                }
            }
            progress(format!(
                "DFF {dff_name}: redirected {changed} texture reference(s)"
            ));
        }
        if geometry_staged || changed > 0 {
            staged_assets.insert(dff_name, bytes);
        }
    }
    for (name, bytes) in txd_updates {
        staged_assets.insert(name, bytes);
    }

    progress(format!(
        "Archive phase: atomically staging {} optimized asset(s)",
        staged_assets.len()
    ));
    upsert_replacement_assets(
        &wip_root_path(&context.root),
        &staged_assets.into_iter().collect::<Vec<_>>(),
    )?;

    let mut staged_txd_index = TxdTextureIndex::new();
    let replacement_img = wip_root_path(&context.root)
        .join("imgs")
        .join(REPLACEMENT_IMG);
    if replacement_img.is_file() {
        index_txd_file(&replacement_img, &mut staged_txd_index);
    }
    txd_result.warnings.sort();
    txd_result.warnings.dedup();
    txd_result.errors.sort();
    txd_result.errors.dedup();
    progress("Archive phase: staged archive validated; applying editor state".to_string());
    Ok(AssetOptimizationWorkerResult {
        txd: txd_result,
        geometry,
        staged_txd_index,
    })
}

fn v3_component(value: V3, axis: usize) -> f32 {
    match axis {
        0 => value.x,
        1 => value.y,
        _ => value.z,
    }
}

fn oriented_v3(value: V3, permutation: [usize; 3], signs: [f32; 3]) -> V3 {
    V3 {
        x: v3_component(value, permutation[0]) * signs[0],
        y: v3_component(value, permutation[1]) * signs[1],
        z: v3_component(value, permutation[2]) * signs[2],
    }
}

fn transform_v3_uniform(
    value: V3,
    permutation: [usize; 3],
    signs: [f32; 3],
    scale: f32,
    offset: V3,
) -> V3 {
    let value = oriented_v3(value, permutation, signs);
    V3 {
        x: value.x * scale + offset.x,
        y: value.y * scale + offset.y,
        z: value.z * scale + offset.z,
    }
}

fn collision_geometry_points(mesh: &CollisionMesh) -> Vec<V3> {
    let mut out = mesh.vertices.clone();
    for sphere in &mesh.spheres {
        let r = sphere.radius.abs();
        out.extend([
            V3 {
                x: sphere.center.x - r,
                ..sphere.center
            },
            V3 {
                x: sphere.center.x + r,
                ..sphere.center
            },
            V3 {
                y: sphere.center.y - r,
                ..sphere.center
            },
            V3 {
                y: sphere.center.y + r,
                ..sphere.center
            },
            V3 {
                z: sphere.center.z - r,
                ..sphere.center
            },
            V3 {
                z: sphere.center.z + r,
                ..sphere.center
            },
        ]);
    }
    for col_box in &mesh.boxes {
        for x in [col_box.min.x, col_box.max.x] {
            for y in [col_box.min.y, col_box.max.y] {
                for z in [col_box.min.z, col_box.max.z] {
                    out.push(V3 { x, y, z });
                }
            }
        }
    }
    out
}

fn deterministic_sample(points: &[V3], limit: usize) -> Vec<V3> {
    if points.len() <= limit {
        return points.to_vec();
    }
    (0..limit)
        .map(|idx| points[idx.saturating_mul(points.len()) / limit])
        .collect()
}

fn point_set_score(source: &[V3], target: &[V3], normalizer: f32) -> f32 {
    if source.is_empty() || target.is_empty() || normalizer <= 0.0001 {
        return f32::INFINITY;
    }
    let sum = source
        .iter()
        .map(|point| {
            target
                .iter()
                .map(|other| {
                    let dx = point.x - other.x;
                    let dy = point.y - other.y;
                    let dz = point.z - other.z;
                    dx * dx + dy * dy + dz * dz
                })
                .fold(f32::INFINITY, f32::min)
                .sqrt()
        })
        .sum::<f32>();
    sum / source.len() as f32 / normalizer
}

#[derive(Clone, Copy)]
struct ColAlignment {
    permutation: [usize; 3],
    signs: [f32; 3],
    scale: f32,
    offset: V3,
    score: f32,
}

fn permutation_parity(permutation: [usize; 3]) -> f32 {
    let inversions = usize::from(permutation[0] > permutation[1])
        + usize::from(permutation[0] > permutation[2])
        + usize::from(permutation[1] > permutation[2]);
    if inversions % 2 == 0 { 1.0 } else { -1.0 }
}

fn detect_col_alignment(mesh: &CollisionMesh, dff_vertices: &[V3]) -> Option<ColAlignment> {
    let all_col_points = collision_geometry_points(mesh);
    if all_col_points.len() < 3 || dff_vertices.len() < 3 {
        return None;
    }
    let full_dff_bounds = bounds_from_vertices(dff_vertices);
    let full_col_bounds = bounds_from_vertices(&all_col_points);
    let full_dff_size = full_dff_bounds.max - full_dff_bounds.min;
    let full_dff_diag = full_dff_size.length();
    if full_dff_diag <= 0.001 {
        return None;
    }
    let center_delta = ((full_dff_bounds.min + full_dff_bounds.max)
        - (full_col_bounds.min + full_col_bounds.max))
        .length()
        * 0.5;
    let full_col_size = full_col_bounds.max - full_col_bounds.min;
    let axis_ratios = [
        full_col_size.x / full_dff_size.x.max(0.001),
        full_col_size.y / full_dff_size.y.max(0.001),
        full_col_size.z / full_dff_size.z.max(0.001),
    ];
    if center_delta <= full_dff_diag * 0.05
        && axis_ratios.iter().all(|ratio| (0.8..=1.25).contains(ratio))
    {
        return None;
    }
    // Candidate scoring is quadratic, so use a deterministic bounded sample.
    // The final transform and bounds still apply to the complete geometry.
    let col_points = deterministic_sample(&all_col_points, 64);
    let dff_points = deterministic_sample(dff_vertices, 64);
    if col_points.len() < 3 || dff_points.len() < 3 {
        return None;
    }
    let dff_bounds = bounds_from_vertices(&dff_points);
    let col_bounds = bounds_from_vertices(&col_points);
    let dff_size = dff_bounds.max - dff_bounds.min;
    let col_size = col_bounds.max - col_bounds.min;
    let dff_diag = dff_size.length();
    if dff_diag <= 0.001 {
        return None;
    }
    let dff_center = (dff_bounds.min + dff_bounds.max) * 0.5;
    let col_center = (col_bounds.min + col_bounds.max) * 0.5;
    let centered = (dff_center - col_center).length() <= dff_diag * 0.1;
    let ratios = [
        col_size.x / dff_size.x.max(0.001),
        col_size.y / dff_size.y.max(0.001),
        col_size.z / dff_size.z.max(0.001),
    ];
    // A centered but oversized collision is commonly intentional. Preserve it
    // rather than deriving a scale merely because its envelope is larger.
    if centered && ratios.iter().copied().fold(0.0f32, f32::max) > 2.5 {
        return None;
    }
    let identity_score = point_set_score(&col_points, &dff_points, dff_diag);
    let permutations = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];
    let mut best = None::<ColAlignment>;
    for permutation in permutations {
        for sx in [-1.0, 1.0] {
            for sy in [-1.0, 1.0] {
                for sz in [-1.0, 1.0] {
                    let signs = [sx, sy, sz];
                    if permutation_parity(permutation) * sx * sy * sz < 0.0 {
                        continue;
                    }
                    let oriented = col_points
                        .iter()
                        .map(|point| oriented_v3(*point, permutation, signs))
                        .collect::<Vec<_>>();
                    let oriented_bounds = bounds_from_vertices(&oriented);
                    let size = oriented_bounds.max - oriented_bounds.min;
                    let mut scales = [
                        dff_size.x / size.x.max(0.001),
                        dff_size.y / size.y.max(0.001),
                        dff_size.z / size.z.max(0.001),
                    ];
                    scales.sort_by(f32::total_cmp);
                    let scale = scales[1];
                    if !scale.is_finite() || !(0.01..=100.0).contains(&scale) {
                        continue;
                    }
                    let center = (oriented_bounds.min + oriented_bounds.max) * 0.5;
                    let offset_mq = dff_center - center * scale;
                    let offset = V3 {
                        x: offset_mq.x,
                        y: offset_mq.y,
                        z: offset_mq.z,
                    };
                    let transformed = oriented
                        .iter()
                        .map(|point| V3 {
                            x: point.x * scale + offset.x,
                            y: point.y * scale + offset.y,
                            z: point.z * scale + offset.z,
                        })
                        .collect::<Vec<_>>();
                    let score = point_set_score(&transformed, &dff_points, dff_diag);
                    if best.is_none_or(|current| score < current.score) {
                        best = Some(ColAlignment {
                            permutation,
                            signs,
                            scale,
                            offset,
                            score,
                        });
                    }
                }
            }
        }
    }
    let best = best?;
    let identity_transform = best.permutation == [0, 1, 2]
        && best.signs == [1.0, 1.0, 1.0]
        && (best.scale - 1.0).abs() < 0.01
        && to_mq(best.offset).length() < dff_diag * 0.01;
    (!identity_transform && best.score < 0.12 && best.score < identity_score * 0.35).then_some(best)
}

fn apply_col_alignment(mesh: &mut CollisionMesh, alignment: ColAlignment) {
    for vertex in &mut mesh.vertices {
        *vertex = transform_v3_uniform(
            *vertex,
            alignment.permutation,
            alignment.signs,
            alignment.scale,
            alignment.offset,
        );
    }
    for sphere in &mut mesh.spheres {
        sphere.center = transform_v3_uniform(
            sphere.center,
            alignment.permutation,
            alignment.signs,
            alignment.scale,
            alignment.offset,
        );
        sphere.radius = sphere.radius.abs() * alignment.scale.abs();
    }
    for col_box in &mut mesh.boxes {
        let mut corners = Vec::with_capacity(8);
        for x in [col_box.min.x, col_box.max.x] {
            for y in [col_box.min.y, col_box.max.y] {
                for z in [col_box.min.z, col_box.max.z] {
                    corners.push(transform_v3_uniform(
                        V3 { x, y, z },
                        alignment.permutation,
                        alignment.signs,
                        alignment.scale,
                        alignment.offset,
                    ));
                }
            }
        }
        let bounds = bounds_from_vertices(&corners);
        col_box.min = V3 {
            x: bounds.min.x,
            y: bounds.min.y,
            z: bounds.min.z,
        };
        col_box.max = V3 {
            x: bounds.max.x,
            y: bounds.max.y,
            z: bounds.max.z,
        };
    }
}

fn clean_collision_mesh(mesh: &mut CollisionMesh) -> (usize, usize) {
    let original_vertex_count = mesh.vertices.len();
    let mut seen_spheres = HashSet::new();
    mesh.spheres.retain(|sphere| {
        let valid = sphere.center.x.is_finite()
            && sphere.center.y.is_finite()
            && sphere.center.z.is_finite()
            && sphere.radius.is_finite()
            && sphere.radius.abs() > 0.0001;
        valid
            && seen_spheres.insert((
                [
                    sphere.center.x.to_bits(),
                    sphere.center.y.to_bits(),
                    sphere.center.z.to_bits(),
                    sphere.radius.abs().to_bits(),
                ],
                [
                    sphere.surface.material,
                    sphere.surface.flags,
                    sphere.surface.brightness,
                    sphere.surface.light,
                ],
            ))
    });
    for sphere in &mut mesh.spheres {
        sphere.radius = sphere.radius.abs();
    }
    let mut seen_boxes = HashSet::new();
    mesh.boxes.retain_mut(|col_box| {
        let values = [
            col_box.min.x,
            col_box.min.y,
            col_box.min.z,
            col_box.max.x,
            col_box.max.y,
            col_box.max.z,
        ];
        if !values.into_iter().all(f32::is_finite) {
            return false;
        }
        let min = V3 {
            x: col_box.min.x.min(col_box.max.x),
            y: col_box.min.y.min(col_box.max.y),
            z: col_box.min.z.min(col_box.max.z),
        };
        let max = V3 {
            x: col_box.min.x.max(col_box.max.x),
            y: col_box.min.y.max(col_box.max.y),
            z: col_box.min.z.max(col_box.max.z),
        };
        col_box.min = min;
        col_box.max = max;
        let valid = (max.x - min.x).abs() + (max.y - min.y).abs() + (max.z - min.z).abs() > 0.0001;
        valid
            && seen_boxes.insert((
                [
                    min.x.to_bits(),
                    min.y.to_bits(),
                    min.z.to_bits(),
                    max.x.to_bits(),
                    max.y.to_bits(),
                    max.z.to_bits(),
                ],
                [
                    col_box.surface.material,
                    col_box.surface.flags,
                    col_box.surface.brightness,
                    col_box.surface.light,
                ],
            ))
    });
    let mut unique_vertices = Vec::<V3>::new();
    let mut unique_by_position = HashMap::<[u32; 3], u16>::new();
    let mut vertex_remap = Vec::<Option<u16>>::with_capacity(mesh.vertices.len());
    for vertex in &mesh.vertices {
        if !vertex.x.is_finite() || !vertex.y.is_finite() || !vertex.z.is_finite() {
            vertex_remap.push(None);
            continue;
        }
        let key = [vertex.x.to_bits(), vertex.y.to_bits(), vertex.z.to_bits()];
        let next = unique_vertices.len() as u16;
        let index = *unique_by_position.entry(key).or_insert_with(|| {
            unique_vertices.push(*vertex);
            next
        });
        vertex_remap.push(Some(index));
    }
    for face in &mut mesh.faces {
        face.a = vertex_remap
            .get(face.a as usize)
            .and_then(|index| *index)
            .unwrap_or(u16::MAX);
        face.b = vertex_remap
            .get(face.b as usize)
            .and_then(|index| *index)
            .unwrap_or(u16::MAX);
        face.c = vertex_remap
            .get(face.c as usize)
            .and_then(|index| *index)
            .unwrap_or(u16::MAX);
    }
    mesh.vertices = unique_vertices;
    let before_faces = mesh.faces.len();
    let mut seen_faces = HashSet::new();
    mesh.faces.retain(|face| {
        if face.a == face.b || face.b == face.c || face.c == face.a {
            return false;
        }
        let Some((a, b, c)) = mesh
            .vertices
            .get(face.a as usize)
            .zip(mesh.vertices.get(face.b as usize))
            .zip(mesh.vertices.get(face.c as usize))
            .map(|((a, b), c)| (a, b, c))
        else {
            return false;
        };
        let area = (to_mq(*b) - to_mq(*a))
            .cross(to_mq(*c) - to_mq(*a))
            .length_squared();
        let mut indices = [face.a, face.b, face.c];
        indices.sort_unstable();
        area > 0.0000001 && seen_faces.insert((indices, face.material, face.light))
    });
    let mut used = BTreeSet::new();
    for face in &mesh.faces {
        used.extend([face.a as usize, face.b as usize, face.c as usize]);
    }
    let mut remap = HashMap::new();
    let mut vertices = Vec::with_capacity(used.len());
    for old in used {
        if let Some(vertex) = mesh.vertices.get(old).copied() {
            remap.insert(old, vertices.len() as u16);
            vertices.push(vertex);
        }
    }
    for face in &mut mesh.faces {
        face.a = remap[&(face.a as usize)];
        face.b = remap[&(face.b as usize)];
        face.c = remap[&(face.c as usize)];
    }
    mesh.vertices = vertices;
    mesh.bounds = collision_mesh_bounds(&mesh.vertices, &mesh.spheres, &mesh.boxes);
    (
        before_faces.saturating_sub(mesh.faces.len()),
        original_vertex_count.saturating_sub(mesh.vertices.len()),
    )
}

fn union_bounds(a: Bounds, b: Bounds) -> Bounds {
    Bounds {
        min: a.min.min(b.min),
        max: a.max.max(b.max),
    }
}

fn optimized_col_header_bounds(mesh: &CollisionMesh, dff_bounds: Option<Bounds>) -> Option<Bounds> {
    let has_collision_geometry =
        !mesh.vertices.is_empty() || !mesh.spheres.is_empty() || !mesh.boxes.is_empty();
    let collision_bounds = has_collision_geometry
        .then(|| collision_mesh_bounds(&mesh.vertices, &mesh.spheres, &mesh.boxes));
    match (dff_bounds, collision_bounds) {
        (Some(dff), Some(collision)) => Some(union_bounds(dff, collision)),
        (Some(dff), None) => Some(dff),
        (None, Some(collision)) => Some(collision),
        (None, None) => None,
    }
}

fn col_header_matches_bounds(bytes: &[u8], bounds: Bounds) -> bool {
    if bytes.len() < 72 || !matches!(&bytes[0..4], b"COL2" | b"COL3" | b"COL4") {
        return false;
    }
    let min = vec3(rdf32(bytes, 32), rdf32(bytes, 36), rdf32(bytes, 40));
    let max = vec3(rdf32(bytes, 44), rdf32(bytes, 48), rdf32(bytes, 52));
    [min.x, min.y, min.z, max.x, max.y, max.z]
        .into_iter()
        .all(f32::is_finite)
        && (min.x - bounds.min.x).abs() <= 0.001
        && (min.y - bounds.min.y).abs() <= 0.001
        && (min.z - bounds.min.z).abs() <= 0.001
        && (max.x - bounds.max.x).abs() <= 0.001
        && (max.y - bounds.max.y).abs() <= 0.001
        && (max.z - bounds.max.z).abs() <= 0.001
}

fn apply_geometry_cleanup_filtered(
    context: &GeometryOptimizationContext,
    dff_filter: &BTreeSet<String>,
    col_filter: &BTreeSet<String>,
    cancelled: &AtomicBool,
    progress: &impl Fn(String),
) -> GeometryCleanupResult {
    let mut result = GeometryCleanupResult::default();
    result.referenced_dffs = dff_filter.clone();
    result.referenced_cols = col_filter.clone();
    let wip_root = wip_root_path(&context.root);
    let mut staged_assets = Vec::<(String, Vec<u8>)>::new();
    let mut recompile_ids = BTreeSet::<String>::new();
    let mut staged_collision_meshes = Vec::<(String, CollisionMesh)>::new();
    let mut dff_to_definition_ids = BTreeMap::<String, Vec<String>>::new();
    for def in &context.definitions {
        if !context.loaded_definition_ids.contains(&def.id) {
            continue;
        }
        let dff_name = asset_key_opt(def.attrs.get("dff"), &def.id, ".dff");
        if !dff_filter.contains(&dff_name) {
            continue;
        }
        dff_to_definition_ids
            .entry(dff_name)
            .or_default()
            .push(def.id.clone());
    }
    let mut dff_meshes = BTreeMap::<String, RawMesh>::new();
    let dff_count = dff_to_definition_ids.len();
    for (dff_index, (dff_name, definition_ids)) in dff_to_definition_ids.iter().enumerate() {
        if cancelled.load(Ordering::Relaxed) {
            result.errors.push("DFF optimization cancelled".to_string());
            break;
        }
        let Some(entry) = find_dff_entry_for_geometry(context, dff_name) else {
            result
                .errors
                .push(format!("{dff_name}: referenced DFF is unavailable"));
            progress(format!(
                "DFF {}/{} {}: unavailable",
                dff_index + 1,
                dff_count,
                dff_name
            ));
            continue;
        };
        result.dffs_scanned += 1;
        let mut source = read_img_entry(&entry);
        source.truncate(dff_chunk_len(&source));
        let opaque_materials = context
            .opaque_materials_by_dff
            .get(dff_name)
            .cloned()
            .unwrap_or_default();
        match optimize_dff_bytes(
            dff_name,
            &source,
            context.building_dffs.contains(dff_name),
            &opaque_materials,
        ) {
            Ok(output) => {
                result.dff_vertices_removed += output.compaction.vertices_removed;
                result.dff_triangles_removed += output.compaction.invalid_triangles_removed
                    + output.compaction.degenerate_triangles_removed
                    + output.compaction.duplicate_triangles_removed;
                result.dff_materials_removed += output.compaction.materials_removed;
                result.dff_triangles_reordered += output.compaction.triangles_reordered;
                let raw = parse_dff_mesh(&output.bytes);
                if output.bytes != source {
                    staged_assets.push((dff_name.clone(), output.bytes.clone()));
                    result.dffs_repaired += 1;
                    if !output.reasons.is_empty() {
                        result
                            .warnings
                            .push(format!("{dff_name}: {}", output.reasons.join("; ")));
                    }
                    for id in definition_ids {
                        recompile_ids.insert(id.clone());
                    }
                    progress(format!(
                        "DFF {}/{} {}: optimized ({} vertices, {} triangles, {} materials removed)",
                        dff_index + 1,
                        dff_count,
                        dff_name,
                        output.compaction.vertices_removed,
                        output.compaction.invalid_triangles_removed
                            + output.compaction.degenerate_triangles_removed
                            + output.compaction.duplicate_triangles_removed,
                        output.compaction.materials_removed
                    ));
                } else {
                    progress(format!(
                        "DFF {}/{} {}: valid; no rewrite",
                        dff_index + 1,
                        dff_count,
                        dff_name
                    ));
                }
                result.warnings.extend(
                    output
                        .warnings
                        .into_iter()
                        .map(|warning| format!("{dff_name}: {warning}")),
                );
                dff_meshes.insert(dff_name.clone(), raw);
            }
            Err(err) => {
                progress(format!(
                    "DFF {}/{} {}: error: {}",
                    dff_index + 1,
                    dff_count,
                    dff_name,
                    err
                ));
                result.errors.push(format!("{dff_name}: {err}"));
            }
        }
    }

    let mut col_to_dffs = BTreeMap::<String, BTreeSet<String>>::new();
    for def in &context.definitions {
        if !context.loaded_definition_ids.contains(&def.id) {
            continue;
        }
        let dff_name = asset_key_opt(def.attrs.get("dff"), &def.id, ".dff");
        let col_name = def
            .attrs
            .get("col")
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(|value| asset_key(value, ".col"))
            .unwrap_or_else(|| asset_key_opt(def.attrs.get("dff"), &def.id, ".col"));
        if !col_filter.contains(&col_name) {
            continue;
        }
        col_to_dffs.entry(col_name).or_default().insert(dff_name);
    }
    let entries = collect_resource_col_entries(&context.root);
    let col_count = col_to_dffs.len();
    for (col_index, (col_name, dff_names)) in col_to_dffs.into_iter().enumerate() {
        if cancelled.load(Ordering::Relaxed) {
            result.errors.push("COL optimization cancelled".to_string());
            break;
        }
        let Some(entry) = entries.get(&col_name) else {
            result
                .errors
                .push(format!("{col_name}: referenced COL is unavailable"));
            progress(format!(
                "COL {}/{} {}: unavailable",
                col_index + 1,
                col_count,
                col_name
            ));
            continue;
        };
        result.cols_scanned += 1;
        let source = read_col_entry_bytes(entry);
        let before_issues = validate_col_for_game_load(&col_name, &source);
        let model_names = collect_col_model_names(&source);
        if model_names.len() > 1 {
            result.warnings.push(format!(
                "{col_name}: contains {} internal COL models; validated but left unchanged because a single definition-level alignment would be ambiguous",
                model_names.len()
            ));
            progress(format!(
                "COL {}/{} {}: multi-model; validated without rewrite",
                col_index + 1,
                col_count,
                col_name
            ));
            continue;
        }
        let Some(mut mesh) = parse_col_mesh(&source, entry) else {
            if let Some(updated) = repair_zero_count_col_offsets(&source) {
                let issues = validate_col_for_game_load(&col_name, &updated);
                if !col_validation_has_errors(&issues) {
                    staged_assets.push((col_name.clone(), updated));
                    result.cols_repaired += 1;
                    result
                        .warnings
                        .extend(issues.iter().map(|issue| issue.label(&col_name)));
                    progress(format!(
                        "COL {}/{} {}: canonicalized empty section offsets",
                        col_index + 1,
                        col_count,
                        col_name
                    ));
                    continue;
                }
            }
            if !col_validation_has_errors(&before_issues) {
                result
                    .warnings
                    .extend(before_issues.iter().map(|issue| issue.label(&col_name)));
                progress(format!(
                    "COL {}/{} {}: valid; no editable collision geometry",
                    col_index + 1,
                    col_count,
                    col_name
                ));
                continue;
            }
            let details = before_issues
                .iter()
                .filter(|issue| issue.severity == ColLoadIssueSeverity::Error)
                .take(4)
                .map(|issue| {
                    if issue.model.trim().is_empty() {
                        issue.message.clone()
                    } else {
                        format!("{}: {}", issue.model, issue.message)
                    }
                })
                .collect::<Vec<_>>()
                .join("; ");
            result.errors.push(format!(
                "{col_name}: COL failed game-load validation: {details}"
            ));
            progress(format!(
                "COL {}/{} {}: failed game-load validation",
                col_index + 1,
                col_count,
                col_name
            ));
            continue;
        };
        let mut dff_vertices = Vec::new();
        let mut dff_bounds = None::<Bounds>;
        for dff_name in &dff_names {
            let fallback_raw = if dff_meshes.contains_key(dff_name) {
                None
            } else {
                find_dff_entry_for_geometry(context, dff_name).map(|entry| {
                    let mut bytes = read_img_entry(&entry);
                    bytes.truncate(dff_chunk_len(&bytes));
                    parse_dff_mesh(&bytes)
                })
            };
            let Some(raw) = dff_meshes.get(dff_name).or(fallback_raw.as_ref()) else {
                result.warnings.push(format!(
                    "{col_name}: could not compare alignment with unavailable {dff_name}"
                ));
                continue;
            };
            if raw.vertices.is_empty() {
                continue;
            }
            let bounds = bounds_from_vertices(&raw.vertices);
            dff_bounds = Some(dff_bounds.map_or(bounds, |current| union_bounds(current, bounds)));
            dff_vertices.extend_from_slice(&raw.vertices);
        }
        let original_mesh = mesh.clone();
        let (removed_faces, removed_vertices) = clean_collision_mesh(&mut mesh);
        let winding = if dff_names
            .iter()
            .any(|dff_name| context.building_dffs.contains(dff_name))
        {
            repair_inverted_ground_winding(&mut mesh)
        } else {
            ColWindingRepairStats::default()
        };
        let coplanar = merge_connected_coplanar_faces(&mut mesh);
        let (post_merge_faces, post_merge_vertices) = clean_collision_mesh(&mut mesh);
        result.col_faces_removed += removed_faces + coplanar.faces_removed + post_merge_faces;
        result.col_vertices_removed += removed_vertices + post_merge_vertices;
        result.col_coplanar_faces_removed += coplanar.faces_removed;
        if winding.faces_reoriented != 0 {
            result.col_winding_meshes_repaired += 1;
            result.col_faces_reoriented += winding.faces_reoriented;
            result.warnings.push(format!(
                "{col_name}: reversed {} collision face(s) whose GTA plane normals pointed below the road",
                winding.faces_reoriented
            ));
        }

        let mut alignment_applied = false;
        if dff_names.len() == 1
            && let Some(alignment) = detect_col_alignment(&mesh, &dff_vertices)
        {
            apply_col_alignment(&mut mesh, alignment);
            alignment_applied = true;
            result.col_alignments += 1;
            result.warnings.push(format!(
                "{col_name}: corrected geometry-derived alignment (scale {:.5}, residual {:.4})",
                alignment.scale, alignment.score
            ));
        }
        if let Some(bounds) = dff_bounds {
            let col_bounds = collision_mesh_bounds(&mesh.vertices, &mesh.spheres, &mesh.boxes);
            let dff_size = bounds.max - bounds.min;
            let col_size = col_bounds.max - col_bounds.min;
            let dff_center = (bounds.min + bounds.max) * 0.5;
            let col_center = (col_bounds.min + col_bounds.max) * 0.5;
            let largest_ratio = [
                col_size.x / dff_size.x.max(0.001),
                col_size.y / dff_size.y.max(0.001),
                col_size.z / dff_size.z.max(0.001),
            ]
            .into_iter()
            .fold(0.0f32, f32::max);
            if largest_ratio > 2.5 && (col_center - dff_center).length() <= dff_size.length() * 0.1
            {
                result.warnings.push(format!(
                    "{col_name}: collision is intentionally-preserved oversized geometry ({largest_ratio:.2}x largest-axis ratio); review recommended"
                ));
            }
        }
        // The model header is the collision broad phase used by GTA/MTA. It
        // must enclose both the visual model and all collision primitives.
        // Using only the DFF bounds silently makes intentionally oversized COL
        // geometry unreachable even though its triangles remain intact.
        let header_bounds = optimized_col_header_bounds(&mesh, dff_bounds);
        let spatial_stats = col_spatial_group_stats(&mesh);
        let spatial_groups_need_rebuild = col_spatial_face_groups_need_rebuild(&source, &mesh);
        let needs_rewrite = !before_issues.is_empty()
            || mesh != original_mesh
            || alignment_applied
            || spatial_groups_need_rebuild
            || header_bounds.is_some_and(|bounds| !col_header_matches_bounds(&source, bounds));
        if !needs_rewrite {
            progress(format!(
                "COL {}/{} {}: valid; no rewrite",
                col_index + 1,
                col_count,
                col_name
            ));
            continue;
        }
        let template = col_write_template(&source, &col_name);
        let mut updated =
            match write_col_mesh_from_template_with_bounds(&template, &mesh, header_bounds) {
                Ok(bytes) => bytes,
                Err(err) => {
                    result.errors.push(format!("{col_name}: {err}"));
                    progress(format!(
                        "COL {}/{} {}: repair failed: {}",
                        col_index + 1,
                        col_count,
                        col_name,
                        err
                    ));
                    continue;
                }
            };
        set_col_model_names_from_entry(&mut updated, &col_name);
        let issues = validate_col_for_game_load(&col_name, &updated);
        if col_validation_has_errors(&issues) {
            result.errors.push(format!(
                "{col_name}: repaired COL still fails validation: {}",
                issues
                    .iter()
                    .map(|issue| issue.label(&col_name))
                    .collect::<Vec<_>>()
                    .join(" | ")
            ));
            continue;
        }
        result
            .warnings
            .extend(issues.iter().map(|issue| issue.label(&col_name)));
        let changed = updated != source;
        if changed {
            staged_assets.push((col_name.clone(), updated.clone()));
            result.cols_repaired += 1;
            if spatial_groups_need_rebuild {
                result.col_face_groups_generated += spatial_stats.groups_generated;
                result.col_faces_spatially_reordered += spatial_stats.faces_reordered;
            }
            let replacement_entry = ImgEntry {
                img_path: wip_root.join("imgs").join(REPLACEMENT_IMG),
                name: col_name.clone(),
                offset: 0,
                size: updated.len() as u32,
            };
            if let Some(reparsed) = parse_col_mesh(&updated, &replacement_entry) {
                staged_collision_meshes.push((col_name.clone(), reparsed));
            }
            progress(format!(
                "COL {}/{} {}: optimized ({} faces, {} vertices removed{}{})",
                col_index + 1,
                col_count,
                col_name,
                removed_faces + coplanar.faces_removed + post_merge_faces,
                removed_vertices + post_merge_vertices,
                if winding.faces_reoriented != 0 {
                    format!(", {} faces reoriented", winding.faces_reoriented)
                } else {
                    String::new()
                },
                if alignment_applied {
                    ", alignment corrected"
                } else {
                    ""
                }
            ));
        } else {
            progress(format!(
                "COL {}/{} {}: validated; encoded bytes unchanged",
                col_index + 1,
                col_count,
                col_name
            ));
        }
    }
    result.staged_assets = staged_assets;
    result.recompile_ids = recompile_ids;
    result.collision_meshes = staged_collision_meshes;
    result.warnings.sort();
    result.warnings.dedup();
    result.errors.sort();
    result.errors.dedup();
    result
}

fn txd_stem(name: &str) -> String {
    name.strip_suffix(".txd").unwrap_or(name).to_string()
}

#[allow(dead_code)]
fn apply_txd_consolidations(
    app: &mut AppState,
    plans: Vec<TxdConsolidation>,
) -> TxdConsolidationResult {
    let mut result = TxdConsolidationResult::default();
    let wip_root = wip_root_path(&app.root);
    for plan in plans {
        let Some(retained_entry) = find_txd_entry_for_app(app, &plan.retained) else {
            result.errors.push(format!(
                "{}: retained TXD is no longer available",
                plan.retained
            ));
            continue;
        };
        let mut merged = read_txd_entry_bytes(&retained_entry);
        let mut donor_renames = BTreeMap::<String, HashMap<String, String>>::new();
        let mut added = 0usize;
        let mut group_error = None;
        for donor in &plan.donors {
            let Some(entry) = find_txd_entry_for_app(app, donor) else {
                group_error = Some(format!("{donor}: donor TXD is no longer available"));
                break;
            };
            match merge_txd_bytes(std::mem::take(&mut merged), &read_txd_entry_bytes(&entry)) {
                Ok((next, renames, count)) => {
                    merged = next;
                    donor_renames.insert(donor.clone(), renames);
                    added += count;
                }
                Err(err) => {
                    group_error = Some(format!(
                        "{} + {donor}: consolidation revalidation failed: {err}",
                        plan.retained
                    ));
                    break;
                }
            }
        }
        if let Some(err) = group_error {
            result.errors.push(err);
            continue;
        }

        let donor_set = plan.donors.iter().cloned().collect::<HashSet<_>>();
        let affected = app
            .definitions
            .iter()
            .filter_map(|(id, def)| {
                let current = definition_txd_name_from_attrs(def)?;
                let key = asset_key(current, ".txd");
                donor_set.contains(&key).then_some((
                    id.clone(),
                    asset_key_opt(def.attrs.get("dff"), &def.id, ".dff"),
                    key,
                ))
            })
            .collect::<Vec<_>>();
        if affected
            .iter()
            .any(|(id, _, _)| app.readonly_definition_ids.contains(id))
        {
            result.warnings.push(format!(
                "{}: consolidation skipped because a donor is referenced by a read-only definition",
                plan.retained
            ));
            continue;
        }
        let mut dff_renames = BTreeMap::<String, HashMap<String, String>>::new();
        let mut rename_conflict = None;
        for (_, dff_name, donor) in &affected {
            let Some(renames) = donor_renames.get(donor) else {
                continue;
            };
            let target = dff_renames.entry(dff_name.clone()).or_default();
            for (old, new) in renames {
                if target.get(old).is_some_and(|existing| existing != new) {
                    rename_conflict = Some(format!(
                        "{dff_name}: texture {old} maps to multiple retained names"
                    ));
                    break;
                }
                target.insert(old.clone(), new.clone());
            }
        }
        if let Some(err) = rename_conflict {
            result.errors.push(err);
            continue;
        }

        let mut rewritten = Vec::<(String, Vec<u8>, usize)>::new();
        for (dff_name, renames) in dff_renames {
            if renames.is_empty() {
                continue;
            }
            let Some(entry) = find_dff_entry_for_app(app, &dff_name) else {
                group_error = Some(format!("{dff_name}: referenced DFF is unavailable"));
                break;
            };
            let mut bytes = read_img_entry(&entry);
            bytes.truncate(dff_chunk_len(&bytes));
            let changed = rename_dff_material_textures(&mut bytes, &renames);
            if changed < renames.len() {
                result.warnings.push(format!(
                    "{dff_name}: redirected {changed}/{} deduplicated texture name(s)",
                    renames.len()
                ));
            }
            rewritten.push((dff_name, bytes, changed));
        }
        if let Some(err) = group_error {
            result.errors.push(err);
            continue;
        }

        // Stage and verify the retained dictionary and every required DFF
        // rewrite before redirecting definitions or marking donors for delete.
        if let Err(err) = upsert_replacement_txd(&wip_root, &plan.retained, &merged) {
            result.errors.push(format!("{}: {err}", plan.retained));
            continue;
        }
        let mut write_failed = false;
        for (dff_name, bytes, _) in &rewritten {
            if let Err(err) = upsert_replacement_dff(&wip_root, dff_name, bytes) {
                result.errors.push(format!("{dff_name}: {err}"));
                write_failed = true;
            }
        }
        if write_failed {
            continue;
        }
        let Some(staged) = find_txd_entry_for_app(app, &plan.retained) else {
            result.errors.push(format!(
                "{}: staged consolidated TXD could not be re-indexed",
                plan.retained
            ));
            continue;
        };
        if let Err(err) = parse_txd_texture_contents(&read_txd_entry_bytes(&staged)) {
            result.errors.push(format!(
                "{}: staged consolidated TXD failed validation: {err}",
                plan.retained
            ));
            continue;
        }

        let definitions_before = app.definitions.clone();
        let retained_stem = txd_stem(&plan.retained);
        for (id, _, _) in &affected {
            if let Some(def) = app.definitions.get_mut(id) {
                def.attrs.insert("txd".to_string(), retained_stem.clone());
            }
        }
        let still_referenced = app.definitions.values().any(|def| {
            definition_txd_name_from_attrs(def)
                .map(|name| asset_key(name, ".txd"))
                .is_some_and(|name| donor_set.contains(&name))
        });
        if still_referenced {
            app.definitions = definitions_before;
            result.errors.push(format!(
                "{}: donor references remained after redirection; deletion cancelled",
                plan.retained
            ));
            continue;
        }
        app.pending_txd_writes.insert(plan.retained.clone());
        reindex_staged_txd(app, &plan.retained);
        for donor in &plan.donors {
            app.pending_txd_writes.remove(donor);
            app.pending_asset_deletes.insert(donor.clone());
        }
        let _ = recompile_definitions_using_txd(app, &plan.retained);
        clear_history_for_external_change(app);
        result.groups += 1;
        result.removed_txds += plan.donors.len();
        result.added_textures += added;
        result.redirected_definitions += affected.len();
        result.rewritten_dffs += rewritten.len();
        result.renamed_materials += rewritten.iter().map(|(_, _, count)| count).sum::<usize>();
        if !plan.exact {
            result.warnings.push(format!(
                "{}: merged unique textures from {} similar TXD(s)",
                plan.retained,
                plan.donors.len()
            ));
        }
    }
    result
}

fn txd_profile_label(profile: TxdOptimizationProfile) -> &'static str {
    if profile == TxdOptimizationProfile::lossless() {
        "Lossless"
    } else {
        "Balanced"
    }
}

fn signed_texture_savings(bytes_saved: i64) -> String {
    if bytes_saved >= 0 {
        format!("+{} saved", format_bytes(bytes_saved as usize))
    } else {
        format!(
            "-{} (size increase)",
            format_bytes(bytes_saved.unsigned_abs() as usize)
        )
    }
}

#[allow(dead_code)]
fn txd_profile_log(plan: &TxdCleanupPlan) -> Vec<String> {
    let mut log = vec![format!(
        "TXD profile: {} (texture payload {}, unused-texture purge saves {})",
        txd_profile_label(plan.profile),
        signed_texture_savings(plan.profile_bytes_saved),
        format_bytes(plan.purge_bytes)
    )];
    for target in &plan.targets {
        for report in &target.profile_reports {
            log.push(format!(
                "{} / {}: {}x{} {} ({} mip{}) -> {}x{} {} ({} mip{}); {}",
                target.txd_name,
                report.texture_name,
                report.old_dimensions.0,
                report.old_dimensions.1,
                report.old_format,
                report.old_mip_count,
                if report.old_mip_count == 1 { "" } else { "s" },
                report.new_dimensions.0,
                report.new_dimensions.1,
                report.new_format,
                report.new_mip_count,
                if report.new_mip_count == 1 { "" } else { "s" },
                signed_texture_savings(report.bytes_saved)
            ));
            log.extend(report.warnings.iter().map(|warning| {
                format!("{} / {}: {warning}", target.txd_name, report.texture_name)
            }));
        }
    }
    log
}

#[allow(dead_code)]
pub(crate) fn apply_txd_cleanup_plan(app: &mut AppState, plan: TxdCleanupPlan) -> Vec<String> {
    struct PreparedTxdUpdate {
        name: String,
        bytes: Vec<u8>,
        dff_names: Vec<String>,
        removed: usize,
        bytes_saved: usize,
        fixed_count: usize,
        fixes: Vec<String>,
        renames: HashMap<String, String>,
    }
    let mut cleaned_txds = 0usize;
    let mut removed_textures = 0usize;
    let mut removed_bytes = 0usize;
    let mut fixed_textures = 0usize;
    let mut renamed_textures = 0usize;
    let mut fixed_dffs = 0usize;
    let mut fixed_txds = Vec::<String>::new();
    let mut dff_renames = HashMap::<String, HashMap<String, String>>::new();
    let consolidations = plan.consolidations.clone();
    let deep_optimize = plan.deep_optimize;
    let profile = plan.profile;
    let profile_bytes_saved = plan.profile_bytes_saved;
    let mut apply_log = txd_profile_log(&plan);
    let skipped = plan.skipped;
    let mut errors = plan.errors;
    let mut warnings = plan.warnings;
    let mut prepared_updates = Vec::<PreparedTxdUpdate>::new();
    for target in plan.targets {
        let Some(entry) = find_txd_entry_for_app(app, &target.txd_name) else {
            errors.push(format!("{}: TXD is no longer available", target.txd_name));
            continue;
        };
        let current = read_txd_entry_bytes(&entry);
        if stable_bytes_fingerprint(&current) != target.source_fingerprint {
            errors.push(format!(
                "{}: TXD changed after the background scan; run optimization again",
                target.txd_name
            ));
            continue;
        }
        if target.purge_count == 0
            && target.format_fix_count == 0
            && target.texture_renames.is_empty()
        {
            continue;
        }
        prepared_updates.push(PreparedTxdUpdate {
            name: target.txd_name,
            bytes: target.prepared_bytes,
            dff_names: target.dff_names,
            removed: target.purge_count,
            bytes_saved: target.purge_bytes,
            fixed_count: target.format_fix_count,
            fixes: target.format_fixes,
            renames: target.texture_renames,
        });
    }
    let wip_root = wip_root_path(&app.root);
    if !prepared_updates.is_empty() {
        let assets = prepared_updates
            .iter()
            .map(|update| (update.name.clone(), update.bytes.clone()))
            .collect::<Vec<_>>();
        match upsert_replacement_assets(&wip_root, &assets) {
            Ok(()) => {
                for update in prepared_updates {
                    app.pending_txd_writes.insert(update.name.clone());
                    reindex_staged_txd(app, &update.name);
                    cleaned_txds += 1;
                    removed_textures += update.removed;
                    removed_bytes += update.bytes_saved;
                    fixed_textures += update.fixed_count;
                    renamed_textures += update.renames.len();
                    for dff_name in update.dff_names {
                        dff_renames
                            .entry(dff_name)
                            .or_default()
                            .extend(update.renames.clone());
                    }
                    fixed_txds.extend(
                        update
                            .fixes
                            .into_iter()
                            .map(|fix| format!("{}: {fix}", update.name)),
                    );
                }
            }
            Err(err) => errors.push(format!("Could not stage cleaned TXD batch: {err}")),
        }
    }
    let mut staged_dff_renames = Vec::<(String, Vec<u8>)>::new();
    for (dff_name, renames) in dff_renames {
        let Some(entry) = find_dff_entry_for_app(app, &dff_name) else {
            errors.push(format!("{dff_name}: DFF is no longer available"));
            continue;
        };
        let mut bytes = read_img_entry(&entry);
        let changed = rename_dff_material_textures(&mut bytes, &renames);
        if changed == 0 {
            warnings.push(format!(
                "{dff_name}: no matching material reference found for renamed texture(s)"
            ));
            continue;
        }
        staged_dff_renames.push((dff_name, bytes));
    }
    if !staged_dff_renames.is_empty() {
        match upsert_replacement_assets(&wip_root, &staged_dff_renames) {
            Ok(()) => fixed_dffs = staged_dff_renames.len(),
            Err(err) => errors.push(format!("Could not stage TXD-reference DFF batch: {err}")),
        }
    }
    let consolidation = apply_txd_consolidations(app, consolidations);
    errors.extend(consolidation.errors.clone());
    warnings.extend(consolidation.warnings.clone());
    let geometry = GeometryCleanupResult::default();
    warnings.sort();
    warnings.dedup();
    refresh_validation_cache(app);
    let fixed_msg = if fixed_textures > 0 {
        let shown = fixed_txds.iter().take(6).cloned().collect::<Vec<_>>();
        let more = fixed_txds.len().saturating_sub(shown.len());
        let suffix = if more > 0 {
            format!("; and {more} more")
        } else {
            String::new()
        };
        format!(
            " Fixed {fixed_textures} texture format/size issue(s): {}{}.",
            shown.join(" | "),
            suffix
        )
    } else {
        String::new()
    };
    let warning_msg = if warnings.is_empty() {
        String::new()
    } else {
        let shown = warnings.iter().take(5).cloned().collect::<Vec<_>>();
        let more = warnings.len().saturating_sub(shown.len());
        if more > 0 {
            format!(
                " Remaining warning(s) not fixed: {}; and {more} more.",
                shown.join(" | ")
            )
        } else {
            format!(" Remaining warning(s) not fixed: {}.", shown.join(" | "))
        }
    };
    if errors.is_empty() {
        app.status_message = format!(
            "{} ({}) staged {cleaned_txds} cleaned TXD(s) and {} consolidation group(s): texture policy {}, removed {removed_textures} unused texture(s), saving {}, added {} unique texture(s), redirected {} definition(s), staged {} duplicate TXD deletion(s), renamed {renamed_textures} overlong texture(s), repaired {}/{} DFF(s), repaired {}/{} COL(s) ({} faces and {} vertices culled, {} collision faces reoriented, {} alignment(s) corrected), and updated {} texture-reference DFF(s).{fixed_msg} Skipped {skipped} external or missing TXD(s). Save promotes staged cleanup.{warning_msg}",
            if deep_optimize {
                "Asset optimization"
            } else {
                "TXD cleanup"
            },
            txd_profile_label(profile),
            consolidation.groups,
            signed_texture_savings(profile_bytes_saved),
            format_bytes(removed_bytes),
            consolidation.added_textures,
            consolidation.redirected_definitions,
            consolidation.removed_txds,
            geometry.dffs_repaired,
            geometry.dffs_scanned,
            geometry.cols_repaired,
            geometry.cols_scanned,
            geometry.col_faces_removed,
            geometry.col_vertices_removed,
            geometry.col_faces_reoriented,
            geometry.col_alignments,
            fixed_dffs + consolidation.rewritten_dffs,
        );
    } else {
        app.status_message = format!(
            "TXD cleanup ({}) applied texture policy ({}), removed {removed_textures} texture(s), saving {}, fixed {fixed_textures} format/size issue(s), renamed {renamed_textures} overlong texture(s), updated {fixed_dffs} DFF(s), skipped {skipped}, and hit {} error(s): {}{warning_msg}",
            txd_profile_label(profile),
            signed_texture_savings(profile_bytes_saved),
            format_bytes(removed_bytes),
            errors.len(),
            errors.join(" | ")
        );
    }
    apply_log.push(app.status_message.clone());
    apply_log.extend(warnings);
    apply_log.extend(errors);
    apply_log
}

pub(crate) fn request_txd_cleanup(app: &mut AppState) {
    if app.manual_save_job.is_some()
        || app.editing.save_rx.is_some()
        || app.autosave_rx.is_some()
        || app.autosave_cleanup_rx.is_some()
        || app.dff_picker_rx.is_some()
        || app.txd_cleanup_job.is_some()
        || app.asset_optimization_scan_rx.is_some()
        || app.asset_optimization_job.is_some()
        || app.object_bounds_fix_job.is_some()
        || app.dff_repair_rx.is_some()
        || app.corona_generation_job.is_some()
        || app.day_night_merge_job.is_some()
        || app.light_lod_job.is_some()
        || app.fracture_generation_job.is_some()
        || app.collision_generation_job.is_some()
        || app.shadow_mesh_generation_job.is_some()
        || app.lod_generation_job.is_some()
    {
        app.status_message =
            "TXD cleanup cannot start while another asset writer is running.".to_string();
        return;
    }
    let job = TxdCleanupJob::new(app, false, TxdOptimizationProfile::lossless());
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = std::panic::catch_unwind(|| job.run_to_completion()).map_err(|panic| {
            panic
                .downcast_ref::<&str>()
                .map(|message| (*message).to_string())
                .or_else(|| panic.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "unknown TXD cleanup scan panic".to_string())
        });
        let _ = tx.send(result);
    });
    app.asset_optimization_scan_rx = Some(rx);
    app.status_message = "TXD cleanup scan running in background...".to_string();
}

pub(crate) fn start_loaded_asset_optimization(app: &mut AppState, plan: TxdCleanupPlan) {
    // Keep every CPU- and IO-heavy phase off the render thread. The worker
    // builds one deterministic replacement-archive transaction and streams
    // per-asset progress while the main loop remains responsive.
    let profile = plan.profile;
    let deep_optimize = plan.deep_optimize;
    let target_count = plan.targets.len();
    let consolidation_count = plan.consolidations.len();
    app.asset_optimization_job = Some(AssetOptimizationJob::new(app, plan));
    app.save_log_open = true;
    app.save_log_follow_tail = true;
    app.save_log_scroll = f32::MAX;
    app.status_message = format!(
        "{} ({}): background worker started for {} TXD update(s) and {} consolidation group(s)",
        if deep_optimize {
            "Asset optimization"
        } else {
            "TXD cleanup"
        },
        txd_profile_label(profile),
        target_count,
        consolidation_count
    );
    append_activity_entry(
        app,
        if deep_optimize {
            "── Asset Optimization ──"
        } else {
            "── TXD Cleanup ──"
        },
    );
    append_activity_entry(app, app.status_message.clone());
    app.activity_last_status = app.status_message.clone();
}

pub(crate) fn update_asset_optimization_job(app: &mut AppState) {
    let Some(mut job) = app.asset_optimization_job.take() else {
        return;
    };
    if !job.step(app) {
        app.asset_optimization_job = Some(job);
    }
}

pub(crate) fn show_asset_optimization_profile_choice(app: &mut AppState) {
    if app.manual_save_job.is_some()
        || app.editing.save_rx.is_some()
        || app.autosave_rx.is_some()
        || app.autosave_cleanup_rx.is_some()
        || app.dff_picker_rx.is_some()
        || app.txd_cleanup_job.is_some()
        || app.asset_optimization_scan_rx.is_some()
        || app.asset_optimization_job.is_some()
        || app.object_bounds_fix_job.is_some()
        || app.dff_repair_rx.is_some()
        || app.corona_generation_job.is_some()
        || app.day_night_merge_job.is_some()
        || app.light_lod_job.is_some()
        || app.fracture_generation_job.is_some()
        || app.collision_generation_job.is_some()
        || app.shadow_mesh_generation_job.is_some()
        || app.lod_generation_job.is_some()
    {
        app.status_message =
            "Asset optimization cannot start while another asset writer is running.".to_string();
        return;
    }
    if !app.asset_optimization_scope.any() {
        app.status_message =
            "Select at least one Optimize Loaded Assets category first.".to_string();
        return;
    }
    if !app.asset_optimization_scope.textures {
        request_loaded_asset_optimization(app, TxdOptimizationProfile::lossless());
        return;
    }
    let selected = [
        app.asset_optimization_scope.textures.then_some("textures"),
        app.asset_optimization_scope.dffs.then_some("DFF geometry"),
        app.asset_optimization_scope.cols.then_some("COL geometry"),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(", ");
    app.confirm_dialog = Some(ConfirmDialog {
        action: ConfirmAction::StartAssetOptimization(TxdOptimizationProfile::lossless()),
        title: "Choose Asset Optimization Profile".to_string(),
        body: format!(
            "Selected categories: {selected}. Lossless preserves existing valid compressed textures while cleaning invalid or wasteful data. Balanced can reduce file size further, but changes texture pixels."
        ),
        detail: "Balanced is lossy: it limits textures to 1024px, encodes opaque textures as DXT1 and alpha textures as DXT5, and generates full mip chains. Mipmaps improve runtime filtering and cache behavior, but may increase some small textures. Both profiles run in the background and stage changes for Save.".to_string(),
        primary_label: "Lossless".to_string(),
        secondary_label: Some("Balanced".to_string()),
        secondary_action: Some(ConfirmAction::StartAssetOptimization(
            TxdOptimizationProfile::balanced(1024, true),
        )),
    });
}

pub(crate) fn request_loaded_asset_optimization(
    app: &mut AppState,
    profile: TxdOptimizationProfile,
) {
    if app.manual_save_job.is_some()
        || app.editing.save_rx.is_some()
        || app.autosave_rx.is_some()
        || app.autosave_cleanup_rx.is_some()
        || app.dff_picker_rx.is_some()
        || app.txd_cleanup_job.is_some()
        || app.asset_optimization_scan_rx.is_some()
        || app.asset_optimization_job.is_some()
        || app.object_bounds_fix_job.is_some()
        || app.dff_repair_rx.is_some()
        || app.corona_generation_job.is_some()
        || app.fracture_generation_job.is_some()
        || app.collision_generation_job.is_some()
        || app.shadow_mesh_generation_job.is_some()
        || app.lod_generation_job.is_some()
    {
        app.status_message =
            "Asset optimization cannot start while another asset writer is running.".to_string();
        return;
    }
    let profile_label = txd_profile_label(profile);
    let job = TxdCleanupJob::new(app, true, profile);
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = std::panic::catch_unwind(|| job.run_to_completion()).map_err(|panic| {
            panic
                .downcast_ref::<&str>()
                .map(|message| (*message).to_string())
                .or_else(|| panic.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "unknown optimization scan panic".to_string())
        });
        let _ = tx.send(result);
    });
    app.asset_optimization_scan_rx = Some(rx);
    app.status_message =
        format!("Asset optimization ({profile_label}) scan running in background...");
}

pub(crate) fn update_asset_optimization_scan(app: &mut AppState) {
    let Some(rx) = app.asset_optimization_scan_rx.take() else {
        return;
    };
    match rx.try_recv() {
        Ok(Ok(plan)) => {
            let operation = if plan.deep_optimize {
                format!("Asset optimization ({})", txd_profile_label(plan.profile))
            } else {
                "TXD cleanup".to_string()
            };
            app.status_message = format!(
                "{operation} scan finished; texture policy {}.",
                signed_texture_savings(plan.profile_bytes_saved)
            );
            show_txd_cleanup_confirmation(app, plan);
        }
        Ok(Err(err)) => {
            app.status_message = format!("Asset scan failed: {err}");
        }
        Err(mpsc::TryRecvError::Empty) => {
            app.asset_optimization_scan_rx = Some(rx);
        }
        Err(mpsc::TryRecvError::Disconnected) => {
            app.status_message = "Asset scan worker disconnected.".to_string();
        }
    }
}

pub(crate) fn show_txd_cleanup_confirmation(app: &mut AppState, plan: TxdCleanupPlan) {
    if !plan.deep_optimize
        && plan.purge_count == 0
        && plan.format_fix_count == 0
        && plan.texture_rename_count == 0
        && plan.consolidations.is_empty()
    {
        app.status_message = if plan.errors.is_empty() && plan.warnings.is_empty() {
            format!(
                "TXD cleanup found no unused textures, overlong names, or format/size fixes; skipped {} external or missing TXD(s).",
                plan.skipped
            )
        } else {
            let warnings = if plan.warnings.is_empty() {
                String::new()
            } else {
                format!(
                    "; {} warning(s) not fixed: {}",
                    plan.warnings.len(),
                    plan.warnings
                        .iter()
                        .take(5)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(" | ")
                )
            };
            format!(
                "TXD cleanup found no fixable textures, skipped {}, and hit {} error(s): {}{}",
                plan.skipped,
                plan.errors.len(),
                plan.errors.join(" | "),
                warnings
            )
        };
        return;
    }
    let txd_count = plan
        .targets
        .iter()
        .filter(|target| {
            target.purge_count > 0
                || target.format_fix_count > 0
                || !target.texture_renames.is_empty()
        })
        .count();
    let mut body = format!(
        "{} will use the {} texture profile, purge {} unused texture(s), rename {} texture name(s) longer than {} characters, optimize {} texture(s) across {} TXD(s), and consolidate {} compatible TXD group(s). Texture policy {}; unused-texture purge saves about {}.",
        if plan.deep_optimize {
            "Asset optimization"
        } else {
            "TXD Cleanup"
        },
        txd_profile_label(plan.profile),
        plan.purge_count,
        plan.texture_rename_count,
        GTA_SA_TEXTURE_NAME_MAX,
        plan.format_fix_count,
        txd_count,
        plan.consolidations.len(),
        signed_texture_savings(plan.profile_bytes_saved),
        format_bytes(plan.purge_bytes)
    );
    if plan.skipped > 0 || !plan.errors.is_empty() || !plan.warnings.is_empty() {
        body.push_str(&format!(
            " Skipped {}; scan errors {}; warnings not fixed {}.",
            plan.skipped,
            plan.errors.len(),
            plan.warnings.len()
        ));
    }
    let mut fixed = plan
        .targets
        .iter()
        .filter(|target| target.purge_count > 0)
        .map(|target| format!("{}: purge {}", target.txd_name, target.purge_count))
        .collect::<Vec<_>>();
    fixed.extend(plan.targets.iter().flat_map(|target| {
        target.profile_reports.iter().map(|report| {
            format!(
                "{} / {}: {}x{} {} -> {}x{} {}, {}",
                target.txd_name,
                report.texture_name,
                report.old_dimensions.0,
                report.old_dimensions.1,
                report.old_format,
                report.new_dimensions.0,
                report.new_dimensions.1,
                report.new_format,
                signed_texture_savings(report.bytes_saved)
            )
        })
    }));
    fixed.extend(plan.format_fixes.iter().take(6).cloned());
    fixed.extend(plan.targets.iter().flat_map(|target| {
        target
            .texture_renames
            .iter()
            .map(|(old, new)| format!("{}: {old} -> {new}", target.txd_name))
    }));
    fixed.extend(
        plan.consolidations
            .iter()
            .map(|group| format!("{} <= {}", group.retained, group.donors.join(", "))),
    );
    fixed.truncate(6);
    let warnings = plan.warnings.iter().take(4).cloned().collect::<Vec<_>>();
    let mut detail = if plan.deep_optimize {
        if plan.profile == TxdOptimizationProfile::lossless() {
            String::from(
                "Lossless preserves existing valid DXT payloads. Confirm to stage TXD consolidation plus conservative validation and repair of every referenced DFF and COL. Donor TXDs are deleted on Save only after references are redirected and the retained assets validate.",
            )
        } else {
            String::from(
                "Balanced is lossy: textures above 1024px are downscaled, texture pixels may be recompressed, and full mip chains are generated. Mipmaps can increase some small textures. Confirm to stage these TXD changes plus conservative DFF/COL repair.",
            )
        }
    } else {
        String::from(
            "Confirm to stage the cleaned TXDs. Use Save to promote the staged TXD changes.",
        )
    };
    if !fixed.is_empty() {
        detail.push_str(&format!(" Planned fixes: {}.", fixed.join(" | ")));
    }
    if !warnings.is_empty() {
        detail.push_str(&format!(" Not fixed: {}.", warnings.join(" | ")));
    }
    let deep_optimize = plan.deep_optimize;
    app.confirm_dialog = Some(ConfirmDialog {
        action: ConfirmAction::TxdCleanup(plan),
        title: if deep_optimize {
            "Confirm Asset Optimization".to_string()
        } else {
            "Confirm TXD Cleanup".to_string()
        },
        body,
        detail,
        primary_label: if deep_optimize {
            "Optimize".to_string()
        } else {
            "Purge".to_string()
        },
        secondary_label: None,
        secondary_action: None,
    });
}

pub(crate) fn update_txd_cleanup_job(app: &mut AppState) {
    let Some(mut job) = app.txd_cleanup_job.take() else {
        return;
    };
    let deep_optimize = job.deep_optimize;
    match job.step() {
        Some(plan) => {
            let elapsed = job.started_at.elapsed().as_secs_f32();
            app.status_message = format!(
                "{} scan finished in {elapsed:.1}s.",
                if deep_optimize {
                    "Asset optimization"
                } else {
                    "TXD cleanup"
                }
            );
            show_txd_cleanup_confirmation(app, plan);
        }
        None => {
            app.status_message = job.status.clone();
            app.txd_cleanup_job = Some(job);
        }
    }
}

fn loaded_mesh_is_required(mesh_key: &str, referenced_dffs: &BTreeSet<String>) -> bool {
    let dff_key = mesh_key.split('|').next().unwrap_or(mesh_key);
    referenced_dffs.contains(dff_key) || is_simulation_mesh_key(mesh_key)
}

pub(crate) fn save_scene_as(app: &mut AppState, target: PathBuf) {
    let target = if target.is_absolute() {
        target
    } else {
        app.root
            .parent()
            .unwrap_or_else(|| Path::new(BROWSE_ROOT))
            .join(target)
    };
    let mode = if target == app.root {
        ManualSaveMode::Resource
    } else {
        ManualSaveMode::SaveAs(target)
    };
    let _ = start_manual_save(app, mode);
}

#[allow(dead_code)]
fn save_scene_as_sync_legacy(app: &mut AppState, target: PathBuf) {
    if let Some(job) = active_conflicting_save_job(app) {
        app.status_message = format!("Wait for the background {job} to finish before Save As.");
        return;
    }
    if app.instance_lod_removal_job.is_some() {
        app.status_message =
            "Wait for background instance LOD removal to finish before Save As.".to_string();
        return;
    }
    if app.lod_generation_job.is_some() {
        app.status_message =
            "Wait for background LOD generation to finish before Save As.".to_string();
        return;
    }
    if app.collision_generation_job.is_some() || app.shadow_mesh_generation_job.is_some() {
        app.status_message =
            "Wait for background collision generation to finish before Save As.".to_string();
        return;
    }
    if app.corona_generation_job.is_some() {
        app.status_message =
            "Wait for background 2DFX corona generation to finish before Save As.".to_string();
        return;
    }
    if app.day_night_merge_job.is_some() {
        app.status_message =
            "Wait for the day/night variant merge to finish before Save As.".to_string();
        return;
    }
    if app.light_lod_job.is_some() {
        app.status_message = "Wait for Light LOD to finish before Save As.".to_string();
        return;
    }
    if app.fracture_generation_job.is_some() {
        app.status_message =
            "Wait for background fracture generation to finish before Save As.".to_string();
        return;
    }
    if app.dff_geometry_job.is_some() {
        app.status_message =
            "Wait for the background DFF geometry operation to finish before Save As.".to_string();
        return;
    }
    apply_pending_inspector_edit_for_save(app);
    let mut errors = Vec::new();
    if let Err(err) = apply_pending_editing_dff_for_save(app) {
        errors.push(err);
    }
    let target = if target.is_absolute() {
        target
    } else {
        app.root
            .parent()
            .unwrap_or_else(|| Path::new(BROWSE_ROOT))
            .join(target)
    };
    if target == app.root {
        save_scene(app);
        return;
    }
    let old_root = app.root.clone();
    if let Err(mut err) = copy_resource_shell(&app.root, &target) {
        errors.append(&mut err);
    }
    if let Err(mut err) = write_scene_files(app, &target) {
        errors.append(&mut err);
    }
    match live_scene_section_updates(
        &app.lights,
        &app.material_emitters,
        &app.material_classes,
        &app.safe_collisions,
        &app.shadow_casting,
    )
    .and_then(|updates| update_eagle_scene_sections(&target, &updates))
    {
        Ok(()) => {}
        Err(err) => errors.push(err),
    }
    if let Err(err) = save_water_dat(&water_dat_path(&target), &app.water_planes) {
        errors.push(err);
    }
    let mut saved_race_tracks = false;
    match save_race_tracks_for_root(app, &target) {
        Ok(saved) => saved_race_tracks = saved,
        Err(err) => errors.push(err),
    }
    if let Err(err) = update_save_as_meta(app, &target) {
        errors.push(err);
    }
    let wip_root = wip_root_path(&app.root);
    if errors.is_empty()
        && let Err(err) = copy_replacement_assets_to_wip(app, &wip_root)
    {
        errors.push(err);
    }
    let mut flushed_vertex_light_assets = 0usize;
    if errors.is_empty() {
        match flush_pending_vertex_lighting(app, &wip_root) {
            Ok(count) => flushed_vertex_light_assets = count,
            Err(err) => errors.push(err),
        }
    }
    let mut merged_replacement_assets = 0usize;
    if errors.is_empty() {
        match promote_wip_replacement_assets_to_root(app, &target) {
            Ok(count) => merged_replacement_assets = count,
            Err(err) => errors.push(err),
        }
    }
    let mut col_writes = 0usize;
    match apply_pending_col_writes(app, &target) {
        Ok(written) => col_writes = written,
        Err(mut err) => errors.append(&mut err),
    }
    let mut deleted_assets = 0usize;
    if errors.is_empty() {
        match apply_pending_asset_deletes(app, &target) {
            Ok(count) => deleted_assets = count,
            Err(err) => errors.push(err),
        }
    }
    if errors.is_empty() {
        app.root = target.clone();
        app.pending_col_writes.clear();
        app.pending_replacement_assets.clear();
        app.pending_txd_writes.clear();
        app.pending_asset_deletes.clear();
        app.pending_vertex_light_meshes.clear();
        clear_saved_editing_replacements(app);
        app.loaded_autosave = false;
        app.loaded_wip = false;
        app.material_classes_dirty = false;
        app.safe_collisions_dirty = false;
        let _ = fs::remove_dir_all(wip_root_path(&old_root));
        let _ = fs::remove_dir_all(wip_root_path(&app.root));
        mark_saved_snapshot(app);
        let replacement_msg = if flushed_vertex_light_assets > 0 || merged_replacement_assets > 0 {
            format!(
                "{flushed_vertex_light_assets} vertex-light DFF(s) written, {merged_replacement_assets} staged asset(s) merged into resource files, "
            )
        } else {
            String::new()
        };
        app.status_message = format!(
            "Saved resource as {} with lights{}. Existing IMG files copied; {replacement_msg}{col_writes} COL byte edit(s) and {deleted_assets} staged asset delete(s) applied.",
            ellipsize(target.to_string_lossy().as_ref(), 52),
            if saved_race_tracks {
                " and race tracks"
            } else {
                ""
            }
        );
        set_save_log(
            app,
            "Save As completed",
            vec![app.status_message.clone()],
            false,
        );
    } else {
        app.status_message = format!(
            "Save As finished with {} error(s). Click status bar to view log.",
            errors.len()
        );
        set_save_log(app, "Save As failed", errors, true);
    }
}

pub(crate) fn default_save_as_path(app: &AppState) -> PathBuf {
    let parent = app.root.parent().unwrap_or_else(|| Path::new(BROWSE_ROOT));
    let name = app
        .root
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("eagle_resource");
    parent.join(format!("{name}_edited"))
}

pub(crate) fn open_save_as_dialog(app: &mut AppState) {
    drain_text_input();
    let path = default_save_as_path(app).to_string_lossy().to_string();
    app.save_as_dialog = Some(SaveAsDialog {
        cursor: path.len(),
        selection_anchor: None,
        path,
    });
}

pub(crate) fn open_load_dialog(app: &mut AppState) {
    drain_text_input();
    let path = if app.root.as_os_str().is_empty() {
        BROWSE_ROOT.to_string()
    } else {
        app.root.to_string_lossy().to_string()
    };
    app.load_dialog = Some(LoadDialog {
        cursor: path.len(),
        selection_anchor: None,
        path,
    });
}

pub(crate) fn open_preferences_dialog(app: &mut AppState) {
    drain_text_input();
    let gta_sa_dir = app.gta_sa_dir.to_string_lossy().to_string();
    app.preferences_dialog = Some(PreferencesDialog {
        cursor: gta_sa_dir.len(),
        selection_anchor: None,
        gta_sa_dir,
    });
}

pub(crate) fn run_folder_picker_command(mut command: Command) -> Result<Option<PathBuf>, String> {
    let output = command
        .output()
        .map_err(|err| format!("Could not open folder picker: {err}"))?;
    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if path.is_empty() {
            Ok(None)
        } else {
            Ok(Some(PathBuf::from(path)))
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            Ok(None)
        } else {
            Err(stderr)
        }
    }
}

#[cfg(windows)]
pub(crate) fn windows_pick_folder(title: &str, start: &Path) -> Result<Option<PathBuf>, String> {
    Ok(rfd::FileDialog::new()
        .set_title(title)
        .set_directory(start)
        .pick_folder())
}

#[cfg(windows)]
fn windows_pick_file(
    title: &str,
    start: &Path,
    filter_name: &str,
    extensions: &[&str],
) -> Result<Option<PathBuf>, String> {
    let directory = if start.is_dir() {
        start
    } else {
        start.parent().unwrap_or_else(|| Path::new("."))
    };
    Ok(rfd::FileDialog::new()
        .set_title(title)
        .set_directory(directory)
        .add_filter(filter_name, extensions)
        .pick_file())
}

#[cfg(windows)]
fn windows_save_file(
    title: &str,
    default_path: &Path,
    filter_name: &str,
    extensions: &[&str],
) -> Result<Option<PathBuf>, String> {
    let mut dialog = rfd::FileDialog::new()
        .set_title(title)
        .add_filter(filter_name, extensions);
    if let Some(parent) = default_path.parent() {
        dialog = dialog.set_directory(parent);
    }
    if let Some(name) = default_path.file_name() {
        dialog = dialog.set_file_name(name.to_string_lossy());
    }
    Ok(dialog.save_file())
}

pub(crate) fn choose_resource_folder() -> Result<Option<PathBuf>, String> {
    #[cfg(windows)]
    return windows_pick_folder("Load Eagle Resource", Path::new(BROWSE_ROOT));

    let mut kdialog = Command::new("kdialog");
    kdialog
        .arg("--title")
        .arg("Load Eagle Resource")
        .arg("--getexistingdirectory")
        .arg(BROWSE_ROOT);
    match run_folder_picker_command(kdialog) {
        Ok(result) => return Ok(result),
        Err(kdialog_err) => {
            let mut zenity = Command::new("zenity");
            zenity
                .arg("--file-selection")
                .arg("--directory")
                .arg("--title=Load Eagle Resource")
                .arg(format!("--filename={BROWSE_ROOT}/"));
            match run_folder_picker_command(zenity) {
                Ok(result) => Ok(result),
                Err(zenity_err) => Err(format!("{kdialog_err}; {zenity_err}")),
            }
        }
    }
}

pub(crate) fn choose_txd_source_folder(start_dir: PathBuf) -> Result<Option<PathBuf>, String> {
    #[cfg(windows)]
    return windows_pick_folder("Generate TXD from Folder", &start_dir);

    let start = if start_dir.is_dir() {
        start_dir.to_string_lossy().to_string()
    } else {
        BROWSE_ROOT.to_string()
    };
    let mut kdialog = Command::new("kdialog");
    kdialog
        .arg("--title")
        .arg("Generate TXD from Folder")
        .arg("--getexistingdirectory")
        .arg(&start);
    match run_folder_picker_command(kdialog) {
        Ok(result) => Ok(result),
        Err(kdialog_err) => {
            let mut zenity = Command::new("zenity");
            zenity
                .arg("--file-selection")
                .arg("--directory")
                .arg("--title=Generate TXD from Folder")
                .arg(format!("--filename={start}/"));
            match run_folder_picker_command(zenity) {
                Ok(result) => Ok(result),
                Err(zenity_err) => Err(format!("{kdialog_err}; {zenity_err}")),
            }
        }
    }
}

pub(crate) fn open_generate_txd_folder_picker(app: &mut AppState) {
    drain_text_input();
    if app.options.launch_mode != LaunchMode::Project {
        app.status_message = "Generate TXD from folder requires an open project".to_string();
        return;
    }
    if app.dff_picker_rx.is_some() {
        app.status_message = "An asset file browser is already open".to_string();
        return;
    }
    let txd_build = app.root.join("txd_build");
    let start_dir = if txd_build.is_dir() {
        txd_build
    } else if app.root.is_dir() {
        app.root.clone()
    } else {
        PathBuf::from(BROWSE_ROOT)
    };
    let (tx, rx) = mpsc::channel();
    app.dff_picker_rx = Some(rx);
    app.status_message = "Choose a folder of PNG textures...".to_string();
    thread::spawn(move || {
        let result = choose_txd_source_folder(start_dir);
        let _ = tx.send((DffPickerKind::GenerateTxdFolder, result));
    });
}

fn project_txd_img_path(root: &Path) -> Option<PathBuf> {
    let mut img_files = Vec::new();
    collect_img_files_from_dir(&root.join("imgs"), &mut img_files);
    img_files.into_iter().find(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("txd.img"))
    })
}

fn source_folder_has_direct_png(source_dir: &Path) -> bool {
    fs::read_dir(source_dir).is_ok_and(|entries| {
        entries.filter_map(Result::ok).any(|entry| {
            entry.file_type().is_ok_and(|kind| kind.is_file())
                && entry
                    .path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
        })
    })
}

fn source_folder_has_png(source_dir: &Path) -> bool {
    WalkDir::new(source_dir)
        .into_iter()
        .filter_map(Result::ok)
        .any(|entry| {
            entry.file_type().is_file()
                && entry
                    .path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
        })
}

/// A folder with PNGs directly in it is one TXD source. A container such as
/// `txd_build/`, with no direct PNGs, produces one TXD for each child folder.
fn txd_source_folders(source_dir: &Path) -> Result<Vec<PathBuf>, String> {
    if !source_dir.is_dir() {
        return Err(format!("{} is not a readable folder", source_dir.display()));
    }
    if source_folder_has_direct_png(source_dir) {
        return Ok(vec![source_dir.to_path_buf()]);
    }
    let mut folders = fs::read_dir(source_dir)
        .map_err(|err| format!("Could not read {}: {err}", source_dir.display()))?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            entry
                .file_type()
                .ok()
                .filter(|kind| kind.is_dir())
                .map(|_| entry.path())
        })
        .filter(|path| source_folder_has_png(path))
        .collect::<Vec<_>>();
    folders.sort();
    if folders.is_empty() {
        return Err(format!(
            "{} does not contain any PNG textures",
            source_dir.display()
        ));
    }
    let mut names = HashSet::new();
    for folder in &folders {
        let name = txd_name_from_folder(folder)?;
        if !names.insert(name.to_ascii_lowercase()) {
            return Err(format!(
                "Multiple source folders produce the TXD name '{name}'; rename one and try again"
            ));
        }
    }
    Ok(folders)
}

fn generate_txds_into_project_img(
    project_root: &Path,
    source_dirs: &[PathBuf],
) -> Result<PathBuf, String> {
    let mut generated = Vec::with_capacity(source_dirs.len());
    for source_dir in source_dirs {
        let (txd_name, txd_bytes) = build_txd_from_folder(source_dir)?;
        if txd_name.as_bytes().len() > IMG_RUNTIME_SAFE_ENTRY_NAME_BYTES {
            return Err(format!(
                "{txd_name}: TXD names must leave room for a terminator in the 24-byte IMG directory field"
            ));
        }
        generated.push((txd_name, txd_bytes));
    }
    let img_path = project_txd_img_path(project_root)
        .unwrap_or_else(|| project_root.join("imgs").join("txd.img"));
    let generated_names = generated
        .iter()
        .map(|(name, _)| name.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let mut entries = Vec::<(String, Vec<u8>)>::new();
    if img_path.is_file() {
        let header = fs::read(&img_path)
            .map_err(|err| format!("Could not read {}: {err}", img_path.display()))?;
        if header.len() < 8 || &header[..4] != b"VER2" {
            return Err(format!(
                "{} is not a supported VER2 IMG archive",
                img_path.display()
            ));
        }
        for entry in parse_img(&img_path) {
            if generated_names.contains(&entry.name.to_ascii_lowercase()) {
                continue;
            }
            let mut bytes = read_img_entry(&entry);
            bytes.truncate(replacement_entry_len(&entry.name, &bytes));
            entries.push((entry.name, bytes));
        }
    }
    entries.extend(generated);
    safe_write_img_archive(&img_path, &entries)?;
    Ok(img_path)
}

fn generate_txds_into_textures(
    project_root: &Path,
    source_dirs: &[PathBuf],
) -> Result<PathBuf, String> {
    if let [source_dir] = source_dirs {
        return generate_txd_from_folder(project_root, source_dir).map(|output| {
            output
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| project_root.join("textures"))
        });
    }
    let generated = source_dirs
        .iter()
        .map(|source_dir| build_txd_from_folder(source_dir))
        .collect::<Result<Vec<_>, _>>()?;
    let texture_dir = project_root.join("textures");
    fs::create_dir_all(&texture_dir)
        .map_err(|err| format!("Could not create {}: {err}", texture_dir.display()))?;
    for (txd_name, txd_bytes) in generated {
        let output = texture_dir.join(&txd_name);
        let temporary = texture_dir.join(format!(".{txd_name}.{}.tmp", std::process::id()));
        fs::write(&temporary, txd_bytes)
            .map_err(|err| format!("Could not write {}: {err}", temporary.display()))?;
        if let Err(err) = fs::rename(&temporary, &output) {
            let _ = fs::remove_file(&temporary);
            return Err(format!("Could not write {}: {err}", output.display()));
        }
    }
    Ok(texture_dir)
}

pub(crate) fn start_generate_txd_from_folder(
    app: &mut AppState,
    source_dir: PathBuf,
    destination: TxdGenerationDestination,
) {
    if app.manual_save_job.is_some()
        || app.editing.save_rx.is_some()
        || app.autosave_rx.is_some()
        || app.autosave_cleanup_rx.is_some()
        || app.dff_repair_rx.is_some()
        || app.txd_cleanup_job.is_some()
        || app.asset_optimization_scan_rx.is_some()
        || app.asset_optimization_job.is_some()
        || app.object_bounds_fix_job.is_some()
    {
        app.status_message =
            "TXD generation cannot start while another asset writer is running.".to_string();
        return;
    }
    if app.dff_picker_rx.is_some() {
        app.status_message = "An asset operation is already running".to_string();
        return;
    }
    let project_root = app.root.clone();
    let destination_label = match destination {
        TxdGenerationDestination::Textures => "textures/",
        TxdGenerationDestination::Img => "TXD.img",
    };
    let (tx, rx) = mpsc::channel();
    app.dff_picker_rx = Some(rx);
    app.status_message = format!(
        "Scanning {} and generating TXDs into {destination_label}...",
        source_dir.display()
    );
    thread::spawn(move || {
        let result = (|| {
            let source_dirs = txd_source_folders(&source_dir)?;
            let txd_names = source_dirs
                .iter()
                .map(|folder| txd_name_from_folder(folder))
                .collect::<Result<Vec<_>, _>>()?;
            let path = match destination {
                TxdGenerationDestination::Textures => {
                    generate_txds_into_textures(&project_root, &source_dirs)
                }
                TxdGenerationDestination::Img => {
                    generate_txds_into_project_img(&project_root, &source_dirs)
                }
            }?;
            Ok::<_, String>((txd_names, path))
        })();
        let message = match result {
            Ok((txd_names, path)) => (
                DffPickerKind::GenerateTxdBuild {
                    txd_names,
                    destination,
                },
                Ok(Some(path)),
            ),
            Err(err) => (
                DffPickerKind::GenerateTxdBuild {
                    txd_names: Vec::new(),
                    destination,
                },
                Err(err),
            ),
        };
        let _ = tx.send(message);
    });
}

fn prompt_txd_generation_destination(app: &mut AppState, source_dir: PathBuf) {
    let folder_name = source_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("selected folder");
    app.confirm_dialog = Some(ConfirmDialog {
        action: ConfirmAction::GenerateTxd {
            source_dir: source_dir.clone(),
            destination: TxdGenerationDestination::Img,
        },
        title: "Choose TXD Destination".to_string(),
        body: format!(
            "Where should TXDs generated from '{folder_name}' be stored? Folder discovery and PNG scanning run after confirmation."
        ),
        detail:
            "Eagle adds to the project TXD.img when present, or creates imgs/txd.img otherwise."
                .to_string(),
        primary_label: "Use TXD.img".to_string(),
        secondary_label: Some("Use textures/".to_string()),
        secondary_action: Some(ConfirmAction::GenerateTxd {
            source_dir,
            destination: TxdGenerationDestination::Textures,
        }),
    });
    app.status_message = "Choose TXD.img or textures/".to_string();
}

fn ensure_project_meta_file_entry(root: &Path, src: &str) -> Result<(), String> {
    let path = root.join("meta.xml");
    let mut text = fs::read_to_string(&path).unwrap_or_else(|_| "<meta>\n</meta>\n".to_string());
    let marker = format!("src=\"{src}\"");
    if text.contains(&marker) {
        return Ok(());
    }
    let entry = format!("    <file src=\"{src}\" type=\"client\" />\n");
    if let Some(at) = text.rfind("</meta>") {
        text.insert_str(at, &entry);
    } else {
        text.push_str("\n<meta>\n");
        text.push_str(&entry);
        text.push_str("</meta>\n");
    }
    fs::write(&path, text).map_err(|err| format!("{}: {err}", path.display()))
}

fn project_meta_has_file_entry(root: &Path, src: &str) -> bool {
    fs::read_to_string(root.join("meta.xml"))
        .is_ok_and(|text| text.contains(&format!("src=\"{src}\"")))
}

fn register_generated_txds(
    app: &mut AppState,
    output: PathBuf,
    txd_names: Vec<String>,
    destination: TxdGenerationDestination,
) {
    for txd_name in &txd_names {
        remove_txd_from_texture_index(app, txd_name);
        invalidate_cached_txd_textures(app, txd_name, None);
    }
    match destination {
        TxdGenerationDestination::Textures => {
            for txd_name in &txd_names {
                index_standalone_txd_file(&output.join(txd_name), &mut app.txd_textures);
            }
        }
        TxdGenerationDestination::Img => {
            app.txd_textures.retain(|_, entries| {
                entries.retain(|entry| entry.img_path != output);
                !entries.is_empty()
            });
            index_txd_file(&output, &mut app.txd_textures);
        }
    }
    let recompiled = txd_names
        .iter()
        .map(|txd_name| recompile_definitions_using_txd(app, txd_name))
        .sum::<usize>();
    app.validation_cache = None;
    let meta_result = match destination {
        TxdGenerationDestination::Textures => {
            if project_meta_has_file_entry(&app.root, "textures/*.txd") {
                Ok(())
            } else {
                ensure_project_meta_file_entry(&app.root, "textures/*.txd")
            }
        }
        TxdGenerationDestination::Img => {
            let exact = format!(
                "imgs/{}",
                output
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("txd.img")
            );
            if project_meta_has_file_entry(&app.root, "imgs/*.img")
                || project_meta_has_file_entry(&app.root, &exact)
            {
                Ok(())
            } else {
                ensure_project_meta_file_entry(&app.root, &exact)
            }
        }
    };
    let generated_label = match (destination, txd_names.len()) {
        (TxdGenerationDestination::Textures, 1) => {
            format!("Generated {}", output.join(&txd_names[0]).display())
        }
        (TxdGenerationDestination::Textures, count) => {
            format!("Generated {count} TXDs in {}", output.display())
        }
        (TxdGenerationDestination::Img, count) => {
            format!("Generated {count} TXD(s) in {}", output.display())
        }
    };
    app.status_message = match meta_result {
        Ok(()) => format!(
            "{generated_label}{}",
            if recompiled > 0 {
                format!(" and refreshed {recompiled} definition(s)")
            } else {
                String::new()
            }
        ),
        Err(err) => format!(
            "{generated_label}, but meta.xml could not be updated: {}",
            ellipsize(&err, 72)
        ),
    };
    let mut log = vec![app.status_message.clone()];
    log.extend(txd_names.into_iter().map(|name| format!("TXD: {name}")));
    set_save_log(app, "TXD Generation", log, false);
}

pub(crate) fn choose_export_dff_path(default_path: PathBuf) -> Result<Option<PathBuf>, String> {
    #[cfg(windows)]
    return windows_save_file("Export DFF", &default_path, "DFF files", &["dff"]);

    let mut kdialog = Command::new("kdialog");
    kdialog
        .arg("--title")
        .arg("Export DFF")
        .arg("--getsavefilename")
        .arg(default_path.to_string_lossy().to_string())
        .arg("*.dff|DFF files");
    match run_folder_picker_command(kdialog) {
        Ok(result) => return Ok(result),
        Err(kdialog_err) => {
            let mut zenity = Command::new("zenity");
            zenity
                .arg("--file-selection")
                .arg("--save")
                .arg("--confirm-overwrite")
                .arg("--title=Export DFF")
                .arg(format!("--filename={}", default_path.to_string_lossy()))
                .arg("--file-filter=DFF files | *.dff");
            match run_folder_picker_command(zenity) {
                Ok(result) => Ok(result),
                Err(zenity_err) => Err(format!("{kdialog_err}; {zenity_err}")),
            }
        }
    }
}

pub(crate) fn choose_export_col_path(default_path: PathBuf) -> Result<Option<PathBuf>, String> {
    #[cfg(windows)]
    return windows_save_file("Export COL", &default_path, "COL files", &["col"]);

    let mut kdialog = Command::new("kdialog");
    kdialog
        .arg("--title")
        .arg("Export COL")
        .arg("--getsavefilename")
        .arg(default_path.to_string_lossy().to_string())
        .arg("*.col|COL files");
    match run_folder_picker_command(kdialog) {
        Ok(result) => return Ok(result),
        Err(kdialog_err) => {
            let mut zenity = Command::new("zenity");
            zenity
                .arg("--file-selection")
                .arg("--save")
                .arg("--confirm-overwrite")
                .arg("--title=Export COL")
                .arg(format!("--filename={}", default_path.to_string_lossy()))
                .arg("--file-filter=COL files | *.col");
            match run_folder_picker_command(zenity) {
                Ok(result) => Ok(result),
                Err(zenity_err) => Err(format!("{kdialog_err}; {zenity_err}")),
            }
        }
    }
}

pub(crate) fn choose_replace_dff_path(start_dir: PathBuf) -> Result<Option<PathBuf>, String> {
    #[cfg(windows)]
    return windows_pick_file("Replace DFF", &start_dir, "DFF files", &["dff"]);

    let mut kdialog = Command::new("kdialog");
    kdialog
        .arg("--title")
        .arg("Replace DFF")
        .arg("--getopenfilename")
        .arg(start_dir.to_string_lossy().to_string())
        .arg("*.dff|DFF files");
    match run_folder_picker_command(kdialog) {
        Ok(result) => return Ok(result),
        Err(kdialog_err) => {
            let mut zenity = Command::new("zenity");
            let filename = if start_dir.is_dir() {
                format!("{}/", start_dir.to_string_lossy())
            } else {
                start_dir.to_string_lossy().to_string()
            };
            zenity
                .arg("--file-selection")
                .arg("--title=Replace DFF")
                .arg(format!("--filename={filename}"))
                .arg("--file-filter=DFF files | *.dff");
            match run_folder_picker_command(zenity) {
                Ok(result) => Ok(result),
                Err(zenity_err) => Err(format!("{kdialog_err}; {zenity_err}")),
            }
        }
    }
}

pub(crate) fn choose_import_prelight_path(start_dir: PathBuf) -> Result<Option<PathBuf>, String> {
    #[cfg(windows)]
    return windows_pick_file(
        "Import Lighting From DFF",
        &start_dir,
        "DFF files and IMG archives",
        &["dff", "img"],
    );

    let mut kdialog = Command::new("kdialog");
    kdialog
        .arg("--title")
        .arg("Import Lighting From DFF")
        .arg("--getopenfilename")
        .arg(start_dir.to_string_lossy().to_string())
        .arg("*.dff *.img|DFF files and IMG archives");
    match run_folder_picker_command(kdialog) {
        Ok(result) => return Ok(result),
        Err(kdialog_err) => {
            let mut zenity = Command::new("zenity");
            let filename = if start_dir.is_dir() {
                format!("{}/", start_dir.to_string_lossy())
            } else {
                start_dir.to_string_lossy().to_string()
            };
            zenity
                .arg("--file-selection")
                .arg("--title=Import Lighting From DFF")
                .arg(format!("--filename={filename}"))
                .arg("--file-filter=DFF files and IMG archives | *.dff *.img");
            match run_folder_picker_command(zenity) {
                Ok(result) => Ok(result),
                Err(zenity_err) => Err(format!("{kdialog_err}; {zenity_err}")),
            }
        }
    }
}

pub(crate) fn choose_blend_scene_path(start_dir: PathBuf) -> Result<Option<PathBuf>, String> {
    #[cfg(windows)]
    return windows_pick_file(
        "Import Blender Scene",
        &start_dir,
        "Blender scenes",
        &["blend"],
    );

    let start = if start_dir.is_dir() {
        start_dir
    } else {
        start_dir
            .parent()
            .unwrap_or(Path::new(BROWSE_ROOT))
            .to_path_buf()
    };
    let default_path = start.join("scene.blend");
    let mut kdialog = Command::new("kdialog");
    kdialog
        .arg("--title")
        .arg("Import Blender Scene")
        .arg("--getopenfilename")
        .arg(default_path.to_string_lossy().to_string())
        .arg("Blender scenes (*.blend)");
    match run_folder_picker_command(kdialog) {
        Ok(result) => Ok(result),
        Err(kdialog_err) => {
            let mut zenity = Command::new("zenity");
            zenity
                .arg("--file-selection")
                .arg("--title=Import Blender Scene")
                .arg("--file-filter=Blender scenes | *.blend")
                .arg(format!("--filename={}", default_path.to_string_lossy()));
            match run_folder_picker_command(zenity) {
                Ok(result) => Ok(result),
                Err(zenity_err) => Err(format!("{kdialog_err}; {zenity_err}")),
            }
        }
    }
}

pub(crate) fn choose_replace_col_path(start_dir: PathBuf) -> Result<Option<PathBuf>, String> {
    #[cfg(windows)]
    return windows_pick_file("Replace COL", &start_dir, "COL files", &["col"]);

    let mut kdialog = Command::new("kdialog");
    kdialog
        .arg("--title")
        .arg("Replace COL")
        .arg("--getopenfilename")
        .arg(start_dir.to_string_lossy().to_string())
        .arg("*.col|COL files");
    match run_folder_picker_command(kdialog) {
        Ok(result) => return Ok(result),
        Err(kdialog_err) => {
            let mut zenity = Command::new("zenity");
            let filename = if start_dir.is_dir() {
                format!("{}/", start_dir.to_string_lossy())
            } else {
                start_dir.to_string_lossy().to_string()
            };
            zenity
                .arg("--file-selection")
                .arg("--title=Replace COL")
                .arg(format!("--filename={filename}"))
                .arg("--file-filter=COL files | *.col");
            match run_folder_picker_command(zenity) {
                Ok(result) => Ok(result),
                Err(zenity_err) => Err(format!("{kdialog_err}; {zenity_err}")),
            }
        }
    }
}

pub(crate) fn choose_export_png_path(default_path: PathBuf) -> Result<Option<PathBuf>, String> {
    #[cfg(windows)]
    return windows_save_file("Export Texture", &default_path, "PNG images", &["png"]);

    let mut kdialog = Command::new("kdialog");
    kdialog
        .arg("--title")
        .arg("Export Texture")
        .arg("--getsavefilename")
        .arg(default_path.to_string_lossy().to_string())
        .arg("*.png|PNG images");
    match run_folder_picker_command(kdialog) {
        Ok(result) => return Ok(result),
        Err(kdialog_err) => {
            let mut zenity = Command::new("zenity");
            zenity
                .arg("--file-selection")
                .arg("--save")
                .arg("--confirm-overwrite")
                .arg("--title=Export Texture")
                .arg(format!("--filename={}", default_path.to_string_lossy()))
                .arg("--file-filter=PNG images | *.png");
            match run_folder_picker_command(zenity) {
                Ok(result) => Ok(result),
                Err(zenity_err) => Err(format!("{kdialog_err}; {zenity_err}")),
            }
        }
    }
}

/// Write a decoded RGBA texture out as a PNG (used by the vehicle texture
/// preview's Export button).
pub(crate) fn export_texture_png_to_path(
    app: &mut AppState,
    texture_name: String,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
    mut path: PathBuf,
) {
    if path
        .extension()
        .map(|ext| !ext.eq_ignore_ascii_case("png"))
        .unwrap_or(true)
    {
        path.set_extension("png");
    }
    if let Some(parent) = path.parent() {
        save_last_dff_export_dir(parent);
    }
    let expected = (width as usize) * (height as usize) * 4;
    if width == 0 || height == 0 || rgba.len() < expected {
        app.status_message = format!("Texture {texture_name} could not be exported (bad data)");
        return;
    }
    match image::save_buffer(
        &path,
        &rgba[..expected],
        width,
        height,
        image::ColorType::Rgba8,
    ) {
        Ok(()) => app.status_message = format!("Exported {texture_name} to {}", path.display()),
        Err(err) => app.status_message = format!("Texture export failed: {err}"),
    }
}

pub(crate) fn choose_texture_image_path(start_dir: PathBuf) -> Result<Option<PathBuf>, String> {
    #[cfg(windows)]
    return windows_pick_file("Choose Texture", &start_dir, "PNG images", &["png"]);

    let mut kdialog = Command::new("kdialog");
    kdialog
        .arg("--title")
        .arg("Choose Texture")
        .arg("--getopenfilename")
        .arg(start_dir.to_string_lossy().to_string())
        .arg("*.png|PNG images");
    match run_folder_picker_command(kdialog) {
        Ok(result) => return Ok(result),
        Err(kdialog_err) => {
            let mut zenity = Command::new("zenity");
            let filename = if start_dir.is_dir() {
                format!("{}/", start_dir.to_string_lossy())
            } else {
                start_dir.to_string_lossy().to_string()
            };
            zenity
                .arg("--file-selection")
                .arg("--title=Choose Texture")
                .arg(format!("--filename={filename}"))
                .arg("--file-filter=PNG images | *.png");
            match run_folder_picker_command(zenity) {
                Ok(result) => Ok(result),
                Err(zenity_err) => Err(format!("{kdialog_err}; {zenity_err}")),
            }
        }
    }
}

pub(crate) fn choose_gif_path(start_dir: PathBuf) -> Result<Option<PathBuf>, String> {
    #[cfg(windows)]
    return windows_pick_file("Choose GIF", &start_dir, "GIF animations", &["gif"]);

    let mut kdialog = Command::new("kdialog");
    kdialog
        .arg("--title")
        .arg("Choose GIF")
        .arg("--getopenfilename")
        .arg(start_dir.to_string_lossy().to_string())
        .arg("*.gif|GIF animations");
    match run_folder_picker_command(kdialog) {
        Ok(result) => return Ok(result),
        Err(kdialog_err) => {
            let mut zenity = Command::new("zenity");
            let filename = if start_dir.is_dir() {
                format!("{}/", start_dir.to_string_lossy())
            } else {
                start_dir.to_string_lossy().to_string()
            };
            zenity
                .arg("--file-selection")
                .arg("--title=Choose GIF")
                .arg(format!("--filename={filename}"))
                .arg("--file-filter=GIF animations | *.gif");
            match run_folder_picker_command(zenity) {
                Ok(result) => Ok(result),
                Err(zenity_err) => Err(format!("{kdialog_err}; {zenity_err}")),
            }
        }
    }
}

pub(crate) fn choose_editing_open_path(start_dir: PathBuf) -> Result<Option<PathBuf>, String> {
    #[cfg(windows)]
    return windows_pick_file(
        "Open IMG or RenderWare Asset",
        &start_dir,
        "IMG or RenderWare assets",
        &["img", "txd", "dff", "col"],
    );

    let mut kdialog = Command::new("kdialog");
    kdialog
        .arg("--title")
        .arg("Open IMG or RenderWare Asset")
        .arg("--getopenfilename")
        .arg(start_dir.to_string_lossy().to_string())
        .arg("*.img *.txd *.dff *.col *|IMG or RenderWare assets");
    match run_folder_picker_command(kdialog) {
        Ok(result) => return Ok(result),
        Err(kdialog_err) => {
            let mut zenity = Command::new("zenity");
            let filename = if start_dir.is_dir() {
                format!("{}/", start_dir.to_string_lossy())
            } else {
                start_dir.to_string_lossy().to_string()
            };
            zenity
                .arg("--file-selection")
                .arg("--title=Open IMG or RenderWare Asset")
                .arg(format!("--filename={filename}"))
                .arg("--file-filter=IMG or RenderWare assets | *.img *.txd *.dff *.col *");
            match run_folder_picker_command(zenity) {
                Ok(result) => Ok(result),
                Err(zenity_err) => Err(format!("{kdialog_err}; {zenity_err}")),
            }
        }
    }
}

pub(crate) fn choose_editing_merge_img_path(start_dir: PathBuf) -> Result<Option<PathBuf>, String> {
    #[cfg(windows)]
    return windows_pick_file("Import IMG to Merge", &start_dir, "IMG archives", &["img"]);

    let mut kdialog = Command::new("kdialog");
    kdialog
        .arg("--title")
        .arg("Import IMG to Merge")
        .arg("--getopenfilename")
        .arg(start_dir.to_string_lossy().to_string())
        .arg("*.img|IMG archives");
    match run_folder_picker_command(kdialog) {
        Ok(result) => return Ok(result),
        Err(kdialog_err) => {
            let mut zenity = Command::new("zenity");
            let filename = if start_dir.is_dir() {
                format!("{}/", start_dir.to_string_lossy())
            } else {
                start_dir.to_string_lossy().to_string()
            };
            zenity
                .arg("--file-selection")
                .arg("--title=Import IMG to Merge")
                .arg(format!("--filename={filename}"))
                .arg("--file-filter=IMG archives | *.img");
            match run_folder_picker_command(zenity) {
                Ok(result) => Ok(result),
                Err(zenity_err) => Err(format!("{kdialog_err}; {zenity_err}")),
            }
        }
    }
}

pub(crate) fn choose_editing_asset_path(start_dir: PathBuf) -> Result<Option<PathBuf>, String> {
    #[cfg(windows)]
    return windows_pick_file(
        "Choose IMG Entry File",
        &start_dir,
        "RenderWare assets",
        &["dff", "txd", "col"],
    );

    let mut kdialog = Command::new("kdialog");
    kdialog
        .arg("--title")
        .arg("Choose IMG Entry File")
        .arg("--getopenfilename")
        .arg(start_dir.to_string_lossy().to_string())
        .arg("*.dff *.txd *.col *|RenderWare assets");
    match run_folder_picker_command(kdialog) {
        Ok(result) => return Ok(result),
        Err(kdialog_err) => {
            let mut zenity = Command::new("zenity");
            let filename = if start_dir.is_dir() {
                format!("{}/", start_dir.to_string_lossy())
            } else {
                start_dir.to_string_lossy().to_string()
            };
            zenity
                .arg("--file-selection")
                .arg("--title=Choose IMG Entry File")
                .arg(format!("--filename={filename}"))
                .arg("--file-filter=RenderWare assets | *.dff *.txd *.col *");
            match run_folder_picker_command(zenity) {
                Ok(result) => Ok(result),
                Err(zenity_err) => Err(format!("{kdialog_err}; {zenity_err}")),
            }
        }
    }
}

pub(crate) fn choose_editing_export_textures_dir(
    start_dir: PathBuf,
) -> Result<Option<PathBuf>, String> {
    #[cfg(windows)]
    return windows_pick_folder("Export TXD Textures", &start_dir);

    let mut kdialog = Command::new("kdialog");
    kdialog
        .arg("--title")
        .arg("Export TXD Textures")
        .arg("--getexistingdirectory")
        .arg(start_dir.to_string_lossy().to_string());
    match run_folder_picker_command(kdialog) {
        Ok(result) => return Ok(result),
        Err(kdialog_err) => {
            let mut zenity = Command::new("zenity");
            let filename = if start_dir.is_dir() {
                format!("{}/", start_dir.to_string_lossy())
            } else {
                start_dir.to_string_lossy().to_string()
            };
            zenity
                .arg("--file-selection")
                .arg("--directory")
                .arg("--title=Export TXD Textures")
                .arg(format!("--filename={filename}"));
            match run_folder_picker_command(zenity) {
                Ok(result) => Ok(result),
                Err(zenity_err) => Err(format!("{kdialog_err}; {zenity_err}")),
            }
        }
    }
}

pub(crate) fn choose_editing_extract_path(
    default_path: PathBuf,
) -> Result<Option<PathBuf>, String> {
    #[cfg(windows)]
    return windows_save_file("Extract IMG Entry", &default_path, "All files", &["*"]);

    let mut kdialog = Command::new("kdialog");
    kdialog
        .arg("--title")
        .arg("Extract IMG Entry")
        .arg("--getsavefilename")
        .arg(default_path.to_string_lossy().to_string())
        .arg("*|All files");
    match run_folder_picker_command(kdialog) {
        Ok(result) => return Ok(result),
        Err(kdialog_err) => {
            let mut zenity = Command::new("zenity");
            zenity
                .arg("--file-selection")
                .arg("--save")
                .arg("--confirm-overwrite")
                .arg("--title=Extract IMG Entry")
                .arg(format!("--filename={}", default_path.to_string_lossy()));
            match run_folder_picker_command(zenity) {
                Ok(result) => Ok(result),
                Err(zenity_err) => Err(format!("{kdialog_err}; {zenity_err}")),
            }
        }
    }
}

pub(crate) fn open_load_picker(app: &mut AppState) {
    drain_text_input();
    if app.load_picker_rx.is_some() {
        app.status_message = "Folder browser is already open".to_string();
        return;
    }
    let (tx, rx) = mpsc::channel();
    app.load_picker_rx = Some(rx);
    app.status_message = "Opening folder browser...".to_string();
    thread::spawn(move || {
        let _ = tx.send(choose_resource_folder());
    });
}

pub(crate) fn poll_load_picker(app: &mut AppState) {
    let Some(rx) = app.load_picker_rx.as_ref() else {
        return;
    };
    let result = match rx.try_recv() {
        Ok(result) => result,
        Err(mpsc::TryRecvError::Empty) => return,
        Err(mpsc::TryRecvError::Disconnected) => {
            app.load_picker_rx = None;
            app.status_message = "Folder browser closed unexpectedly".to_string();
            return;
        }
    };
    app.load_picker_rx = None;
    match result {
        Ok(Some(path)) => start_load_resource(app, path),
        Ok(None) => app.status_message = "Load cancelled".to_string(),
        Err(err) => {
            app.status_message = format!("Folder browser failed: {}", ellipsize(&err, 72));
            open_load_dialog(app);
        }
    }
}

pub(crate) fn choose_vehicle_folder(start_dir: PathBuf) -> Result<Option<PathBuf>, String> {
    #[cfg(windows)]
    return windows_pick_folder("Add Custom Vehicle Dictionary", &start_dir);

    let start = if start_dir.is_dir() {
        start_dir.to_string_lossy().to_string()
    } else {
        BROWSE_ROOT.to_string()
    };
    let mut kdialog = Command::new("kdialog");
    kdialog
        .arg("--title")
        .arg("Add Custom Vehicle Dictionary")
        .arg("--getexistingdirectory")
        .arg(&start);
    match run_folder_picker_command(kdialog) {
        Ok(result) => Ok(result),
        Err(kdialog_err) => {
            let mut zenity = Command::new("zenity");
            zenity
                .arg("--file-selection")
                .arg("--directory")
                .arg("--title=Add Custom Vehicle Dictionary")
                .arg(format!("--filename={start}/"));
            match run_folder_picker_command(zenity) {
                Ok(result) => Ok(result),
                Err(zenity_err) => Err(format!("{kdialog_err}; {zenity_err}")),
            }
        }
    }
}

pub(crate) fn open_vehicle_folder_picker(app: &mut AppState) {
    drain_text_input();
    if app.vehicle_dictionary_scan_rx.is_some() {
        app.status_message =
            "Wait for the current custom vehicle dictionary scan to finish".to_string();
        return;
    }
    if app.vehicle_folder_picker_rx.is_some() {
        app.status_message = "Folder browser is already open".to_string();
        return;
    }
    let start_dir = app
        .custom_vehicle_dictionaries
        .last()
        .cloned()
        .unwrap_or_else(|| PathBuf::from(BROWSE_ROOT));
    let (tx, rx) = mpsc::channel();
    app.vehicle_folder_picker_rx = Some(rx);
    app.status_message = "Opening custom vehicle dictionary browser...".to_string();
    thread::spawn(move || {
        let _ = tx.send(choose_vehicle_folder(start_dir));
    });
}

pub(crate) fn start_custom_vehicle_dictionary_scan(app: &mut AppState) {
    if app.vehicle_dictionary_scan_rx.is_some() {
        app.status_message = "A custom vehicle dictionary scan is already running".to_string();
        return;
    }
    let root = app.root.clone();
    let gta_sa_dir = app.gta_sa_dir.clone();
    let dictionaries = app.custom_vehicle_dictionaries.clone();
    let dictionary_count = dictionaries.len();
    let (tx, rx) = mpsc::channel();
    app.vehicle_dictionary_scan_rx = Some(rx);
    app.status_message = if dictionary_count == 0 {
        "Refreshing the vehicle list...".to_string()
    } else {
        format!(
            "Scanning {dictionary_count} custom vehicle dictionar{}...",
            if dictionary_count == 1 { "y" } else { "ies" }
        )
    };
    thread::spawn(move || {
        let mut by_id = load_vehicle_assets(&root, &gta_sa_dir, &[])
            .into_iter()
            .map(|vehicle| (lower(&vehicle.id), vehicle))
            .collect::<BTreeMap<_, _>>();
        for (idx, dictionary) in dictionaries.iter().enumerate() {
            for vehicle in custom_vehicle_dictionary_assets(dictionary) {
                by_id.insert(lower(&vehicle.id), vehicle);
            }
            if tx
                .send(VehicleDictionaryScanUpdate::Progress {
                    scanned: idx + 1,
                    total: dictionary_count,
                })
                .is_err()
            {
                return;
            }
        }
        let _ = tx.send(VehicleDictionaryScanUpdate::Complete(
            by_id.into_values().collect(),
        ));
    });
}

pub(crate) fn add_custom_vehicle_dictionary(app: &mut AppState, dir: PathBuf) {
    if app
        .custom_vehicle_dictionaries
        .iter()
        .any(|existing| same_resource_path(existing, &dir))
    {
        app.status_message = "That custom vehicle dictionary has already been added".to_string();
        return;
    }
    app.custom_vehicle_dictionaries.push(dir);
    save_custom_vehicle_dictionary_preferences(&app.custom_vehicle_dictionaries);
    app.vehicle_browser.show_custom = true;
    start_custom_vehicle_dictionary_scan(app);
}

pub(crate) fn remove_custom_vehicle_dictionary(app: &mut AppState, index: usize) {
    if index >= app.custom_vehicle_dictionaries.len() {
        return;
    }
    app.custom_vehicle_dictionaries.remove(index);
    save_custom_vehicle_dictionary_preferences(&app.custom_vehicle_dictionaries);
    // Dropping the receiver invalidates any older scan. Its worker will stop
    // when the next progress send observes that the channel is disconnected.
    app.vehicle_dictionary_scan_rx = None;
    start_custom_vehicle_dictionary_scan(app);
}

pub(crate) fn poll_vehicle_folder_picker(app: &mut AppState) {
    let Some(rx) = app.vehicle_folder_picker_rx.as_ref() else {
        return;
    };
    let result = match rx.try_recv() {
        Ok(result) => result,
        Err(mpsc::TryRecvError::Empty) => return,
        Err(mpsc::TryRecvError::Disconnected) => {
            app.vehicle_folder_picker_rx = None;
            app.status_message = "Folder browser closed unexpectedly".to_string();
            return;
        }
    };
    app.vehicle_folder_picker_rx = None;
    match result {
        Ok(Some(path)) => add_custom_vehicle_dictionary(app, path),
        Ok(None) => app.status_message = "Add custom vehicle dictionary cancelled".to_string(),
        Err(err) => {
            app.status_message = format!("Folder browser failed: {}", ellipsize(&err, 72));
        }
    }
}

pub(crate) fn poll_custom_vehicle_dictionary_scan(app: &mut AppState) {
    let Some(rx) = app.vehicle_dictionary_scan_rx.as_ref() else {
        return;
    };
    let update = match rx.try_recv() {
        Ok(update) => update,
        Err(mpsc::TryRecvError::Empty) => return,
        Err(mpsc::TryRecvError::Disconnected) => {
            app.vehicle_dictionary_scan_rx = None;
            app.status_message = "Custom vehicle dictionary scan stopped unexpectedly".to_string();
            return;
        }
    };
    let vehicles = match update {
        VehicleDictionaryScanUpdate::Progress { scanned, total } => {
            app.status_message = format!(
                "Scanned {scanned} / {total} custom vehicle dictionar{}...",
                if total == 1 { "y" } else { "ies" }
            );
            return;
        }
        VehicleDictionaryScanUpdate::Complete(vehicles) => vehicles,
    };
    app.vehicle_dictionary_scan_rx = None;
    let selected_id = selected_vehicle(app).map(|vehicle| lower(&vehicle.id));
    clear_vehicle_preview_mesh(app);
    app.vehicles = vehicles;
    app.vehicle_browser.selected = selected_id
        .as_deref()
        .and_then(|id| {
            app.vehicles
                .iter()
                .position(|vehicle| lower(&vehicle.id) == id)
        })
        .unwrap_or(0);
    app.vehicle_browser.scroll = 0.0;
    app.vehicle_browser.preview_key.clear();
    let custom_count = app
        .vehicles
        .iter()
        .filter(|vehicle| vehicle.source == "custom dictionary")
        .count();
    let dictionary_count = app.custom_vehicle_dictionaries.len();
    app.status_message = format!(
        "Loaded {custom_count} matched DFF/TXD vehicle pair(s) from {dictionary_count} dictionar{}",
        if dictionary_count == 1 { "y" } else { "ies" }
    );
}

pub(crate) fn poll_dff_picker(app: &mut AppState) {
    let Some(rx) = app.dff_picker_rx.as_ref() else {
        return;
    };
    let (kind, result) = match rx.try_recv() {
        Ok(result) => result,
        Err(mpsc::TryRecvError::Empty) => return,
        Err(mpsc::TryRecvError::Disconnected) => {
            app.dff_picker_rx = None;
            app.status_message = "Asset file browser closed unexpectedly".to_string();
            return;
        }
    };
    app.dff_picker_rx = None;
    match (kind, result) {
        (DffPickerKind::ImportBlender, Ok(Some(path))) => start_blender_import(app, path),
        (DffPickerKind::GenerateTxdFolder, Ok(Some(path))) => {
            prompt_txd_generation_destination(app, path)
        }
        (
            DffPickerKind::GenerateTxdBuild {
                txd_names,
                destination,
            },
            Ok(Some(path)),
        ) => register_generated_txds(app, path, txd_names, destination),
        (DffPickerKind::Export, Ok(Some(path))) => start_export_dff(app, path),
        (DffPickerKind::ExportCol, Ok(Some(path))) => start_export_col(app, path),
        (
            DffPickerKind::ExportNamedDff {
                dff_name,
                vehicle_txd,
            },
            Ok(Some(path)),
        ) => start_export_named_dff(app, dff_name, vehicle_txd, path),
        (
            DffPickerKind::ExportLooseFile {
                source_path,
                asset_name,
                vehicle_txd,
            },
            Ok(Some(path)),
        ) => export_loose_file_to_path(app, source_path, asset_name, vehicle_txd, path),
        (
            DffPickerKind::ExportTexturePng {
                texture_name,
                width,
                height,
                rgba,
            },
            Ok(Some(path)),
        ) => export_texture_png_to_path(app, texture_name, width, height, rgba, path),
        (DffPickerKind::ExportTexturePng { .. }, Ok(None)) => {
            app.status_message = "Export texture cancelled".to_string()
        }
        (DffPickerKind::VehicleTextureReplace(request), Ok(Some(path))) => {
            start_vehicle_texture_replace(app, request, path)
        }
        (DffPickerKind::VehicleTextureReplace(_), Ok(None)) => {
            app.status_message = "Replace vehicle material cancelled".to_string()
        }
        (DffPickerKind::Replace, Ok(Some(path))) => {
            start_replace_asset_choice(app, path, ReplacementAssetKind::Dff)
        }
        (DffPickerKind::ReplaceCol, Ok(Some(path))) => {
            start_replace_asset_choice(app, path, ReplacementAssetKind::Col)
        }
        (DffPickerKind::ImportPrelight, Ok(Some(path))) => {
            start_import_prelight_from_path(app, path)
        }
        (
            DffPickerKind::TextureAdd {
                definition_id,
                txd_name,
            },
            Ok(Some(path)),
        ) => add_texture_to_archive(app, definition_id, txd_name, path),
        (
            DffPickerKind::TextureReplace {
                definition_id,
                txd_name,
                texture_name,
            },
            Ok(Some(path)),
        ) => replace_texture_in_archive(app, definition_id, txd_name, texture_name, path),
        (DffPickerKind::EditingOpenFile, Ok(Some(path))) => open_editing_file(app, path),
        (DffPickerKind::EditingMergeImg, Ok(Some(path))) => start_editing_img_merge_scan(app, path),
        (DffPickerKind::EditingAddEntry, Ok(Some(path))) => editing_add_entry_from_path(app, path),
        (DffPickerKind::EditingReplaceEntry { entry_name }, Ok(Some(path))) => {
            editing_replace_entry_from_path(app, entry_name, path)
        }
        (DffPickerKind::EditingExtractEntry { entry_name }, Ok(Some(path))) => {
            editing_extract_entry_to_path(app, entry_name, path)
        }
        (DffPickerKind::EditingTextureAdd { entry_name }, Ok(Some(path))) => {
            editing_import_texture_from_path(app, entry_name, None, path)
        }
        (
            DffPickerKind::EditingTextureReplace {
                entry_name,
                texture_name,
            },
            Ok(Some(path)),
        ) => editing_import_texture_from_path(app, entry_name, Some(texture_name), path),
        (DffPickerKind::EditingTextureExportAll { entry_name }, Ok(Some(path))) => {
            editing_export_all_txd_textures(app, entry_name, path)
        }
        (DffPickerKind::EditingFaceTextureImport { txd_name }, Ok(Some(path))) => {
            editing_import_face_texture_from_path(app, txd_name, path)
        }
        (DffPickerKind::EditingFaceTextureImport { .. }, Ok(None)) => {
            app.status_message = "Assign face texture cancelled".to_string()
        }
        (
            DffPickerKind::EditingMaterialTextureImport {
                txd_name,
                dff_name,
                material,
            },
            Ok(Some(path)),
        ) => editing_import_material_texture_from_path(app, txd_name, dff_name, material, path),
        (DffPickerKind::EditingMaterialTextureImport { .. }, Ok(None)) => {
            app.status_message = "Set material texture cancelled".to_string()
        }
        (DffPickerKind::EditingGifAnimImport { txd_name }, Ok(Some(path))) => {
            editing_import_gif_anim_from_path(app, txd_name, path)
        }
        (DffPickerKind::EditingGifAnimImport { .. }, Ok(None)) => {
            app.status_message = "GIF animation setup cancelled".to_string()
        }
        (DffPickerKind::Export, Ok(None)) => {
            app.status_message = "Export DFF cancelled".to_string()
        }
        (DffPickerKind::ExportCol, Ok(None)) => {
            app.status_message = "Export COL cancelled".to_string()
        }
        (DffPickerKind::ExportNamedDff { .. }, Ok(None)) => {
            app.status_message = "Export DFF cancelled".to_string()
        }
        (DffPickerKind::ExportLooseFile { .. }, Ok(None)) => {
            app.status_message = "Export cancelled".to_string()
        }
        (DffPickerKind::Replace, Ok(None)) => {
            app.status_message = "Replace DFF cancelled".to_string()
        }
        (DffPickerKind::ReplaceCol, Ok(None)) => {
            app.status_message = "Replace COL cancelled".to_string()
        }
        (DffPickerKind::ImportPrelight, Ok(None)) => {
            app.status_message = "Import lighting cancelled".to_string()
        }
        (DffPickerKind::ImportBlender, Ok(None)) => {
            app.status_message = "Import Blender cancelled".to_string()
        }
        (DffPickerKind::GenerateTxdFolder | DffPickerKind::GenerateTxdBuild { .. }, Ok(None)) => {
            app.status_message = "Generate TXD cancelled".to_string()
        }
        (DffPickerKind::TextureAdd { .. }, Ok(None)) => {
            app.status_message = "Add texture cancelled".to_string()
        }
        (DffPickerKind::TextureReplace { .. }, Ok(None)) => {
            app.status_message = "Replace texture cancelled".to_string()
        }
        (DffPickerKind::EditingOpenFile, Ok(None)) => {
            app.status_message = "Open file cancelled".to_string()
        }
        (DffPickerKind::EditingMergeImg, Ok(None)) => {
            app.status_message = "Merge IMG cancelled".to_string()
        }
        (DffPickerKind::EditingAddEntry, Ok(None)) => {
            app.status_message = "Add IMG entry cancelled".to_string()
        }
        (DffPickerKind::EditingReplaceEntry { .. }, Ok(None)) => {
            app.status_message = "Replace IMG entry cancelled".to_string()
        }
        (DffPickerKind::EditingExtractEntry { .. }, Ok(None)) => {
            app.status_message = "Extract IMG entry cancelled".to_string()
        }
        (DffPickerKind::EditingTextureAdd { .. }, Ok(None)) => {
            app.status_message = "Add texture cancelled".to_string()
        }
        (DffPickerKind::EditingTextureReplace { .. }, Ok(None)) => {
            app.status_message = "Replace texture cancelled".to_string()
        }
        (DffPickerKind::EditingTextureExportAll { .. }, Ok(None)) => {
            app.status_message = "Export textures cancelled".to_string()
        }
        (DffPickerKind::RaceRadar, Ok(Some(path))) => set_race_radar_from_path(app, path),
        (DffPickerKind::RaceRadar, Ok(None)) => {
            app.status_message = "Radar image selection cancelled".to_string()
        }
        (DffPickerKind::GenerateTxdFolder | DffPickerKind::GenerateTxdBuild { .. }, Err(err)) => {
            record_txd_generation_failure(app, err);
        }
        (_, Err(err)) => {
            app.status_message = format!("Asset file browser failed: {}", ellipsize(&err, 72));
        }
    }
}

pub(crate) fn normalize_resource_path(path: &str) -> PathBuf {
    let trimmed = path.trim();
    let path = PathBuf::from(trimmed);
    if path.is_absolute() {
        path
    } else {
        Path::new(BROWSE_ROOT).join(path)
    }
}

pub(crate) fn same_resource_path(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

pub(crate) fn start_load_resource(app: &mut AppState, path: PathBuf) {
    start_load_resource_from_source(app, path, LoadSceneSource::Saved, true);
}

pub(crate) fn start_load_resource_checked(
    app: &mut AppState,
    path: PathBuf,
    confirm_unsaved: bool,
) {
    start_load_resource_from_source(app, path, LoadSceneSource::Saved, confirm_unsaved);
}

pub(crate) fn start_load_resource_from_source(
    app: &mut AppState,
    path: PathBuf,
    source: LoadSceneSource,
    confirm_unsaved: bool,
) {
    if path.as_os_str().is_empty() {
        app.status_message = "Load needs a resource path".to_string();
        return;
    }
    if !path.is_dir() {
        app.status_message = format!(
            "Resource path is not a directory: {}",
            ellipsize(path.to_string_lossy().as_ref(), 44)
        );
        return;
    }
    if !path.join("eagleZones.txt").is_file() || !path.join("zones").is_dir() {
        app.status_message = format!(
            "Not an Eagle resource: {}",
            ellipsize(path.to_string_lossy().as_ref(), 48)
        );
        return;
    }
    if source == LoadSceneSource::Saved && same_resource_path(&path, &app.root) {
        app.status_message = format!(
            "Already loaded {}",
            ellipsize(path.to_string_lossy().as_ref(), 56)
        );
        return;
    }
    if confirm_unsaved && has_unsaved_changes(app) {
        app.confirm_dialog = Some(ConfirmDialog {
            action: ConfirmAction::Load(path),
            title: "Unsaved Changes".to_string(),
            body: "Save a WIP snapshot before loading another resource?".to_string(),
            detail:
                "Save WIP keeps map/light XML outside the resource; COL byte edits need resource Save."
                    .to_string(),
            primary_label: "Save WIP".to_string(),
            secondary_label: Some("Discard".to_string()),
            secondary_action: None,
        });
        return;
    }
    app.pending_load_source = source;
    app.pending_load_root = Some(path);
}

pub(crate) fn start_save_as_output(app: &mut AppState, target: PathBuf) {
    if target.as_os_str().is_empty() {
        app.status_message = "Save As needs a target resource path".to_string();
        return;
    }
    let missing = missing_col_count(app);
    if missing > 0 {
        app.missing_col_dialog = Some(MissingColDialog { target, missing });
    } else {
        save_scene_as(app, target);
    }
}

pub(crate) fn selected_dff_name(app: &AppState) -> Option<String> {
    let placement = selected_placement(app)?;
    Some(with_ext(&placement.dff, ".dff"))
}

pub(crate) fn dff_used_by_building(app: &AppState, dff_name: &str) -> bool {
    let target = asset_key(dff_name, ".dff");
    app.placements.iter().any(|placement| {
        placement.tag.eq_ignore_ascii_case("building") && {
            let dff_ref = app
                .definitions
                .get(&placement.id)
                .and_then(|def| def.attrs.get("dff"))
                .filter(|value| !value.trim().is_empty())
                .cloned()
                .unwrap_or_else(|| placement.dff.clone());
            asset_key(&dff_ref, ".dff") == target
        }
    })
}

pub(crate) fn dff_write_options_for_asset(app: &AppState, dff_name: &str) -> DffWriteOptions {
    DffWriteOptions {
        include_normals: !dff_used_by_building(app, dff_name),
        include_bin_mesh: true,
    }
}

pub(crate) fn selected_col_name(app: &AppState) -> Option<String> {
    let placement = selected_placement(app)?;
    let col_name = app
        .definitions
        .get(&placement.id)
        .and_then(|def| def.attrs.get("col"))
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| placement.dff.clone());
    Some(with_ext(&col_name, ".col"))
}

pub(crate) fn find_dff_entry(root: &Path, dff_name: &str) -> Option<ImgEntry> {
    let target = lower(with_ext(dff_name, ".dff"));
    let mut img_files = collect_resource_img_files(root);
    img_files.extend(gta_sa_img_files(&load_gta_sa_dir_preference()));
    for path in img_files {
        for img_entry in parse_img(&path) {
            if lower(&img_entry.name) == target {
                return Some(img_entry);
            }
        }
    }
    None
}

pub(crate) fn find_col_entry(root: &Path, col_name: &str) -> Option<ImgEntry> {
    let target = lower(with_ext(col_name, ".col"));
    if let Some(entry) = find_staged_replacement_entry(root, &target) {
        return Some(entry);
    }
    let mut img_files = collect_resource_img_files(root);
    img_files.extend(gta_sa_img_files(&load_gta_sa_dir_preference()));
    for path in img_files {
        for img_entry in parse_img(&path) {
            if lower(&img_entry.name) == target {
                return Some(img_entry);
            }
        }
    }
    None
}

fn find_staged_replacement_entry(root: &Path, target: &str) -> Option<ImgEntry> {
    for replacement_root in [wip_root_path(root), root.to_path_buf()] {
        let img_path = replacement_root.join("imgs").join(REPLACEMENT_IMG);
        for entry in parse_img(&img_path) {
            if lower(&entry.name) == target {
                return Some(entry);
            }
        }
    }
    None
}

pub(crate) fn img_safe_replacement_name(
    app: &AppState,
    selected_idx: usize,
    make_unique: bool,
    ext: &str,
) -> String {
    let id = app
        .placements
        .get(selected_idx)
        .map(|placement| placement.id.as_str())
        .unwrap_or("replacement");
    let mut base: String = id
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    if base.is_empty() {
        base = "replacement".to_string();
    }
    let suffix = if make_unique { "_unique" } else { "" };
    let max_base_len = IMG_RUNTIME_SAFE_ENTRY_NAME_BYTES.saturating_sub(suffix.len() + ext.len());
    base.truncate(max_base_len);
    format!("{base}{suffix}{ext}")
}

pub(crate) fn img_safe_col_name(app: &AppState, selected_idx: usize, make_unique: bool) -> String {
    img_safe_replacement_name(app, selected_idx, make_unique, ".col")
}

pub(crate) fn upsert_replacement_asset(
    root: &Path,
    asset_name: &str,
    bytes: &[u8],
) -> Result<(), String> {
    upsert_replacement_assets(root, &[(asset_name.to_string(), bytes.to_vec())])
}

/// Updates a replacement archive in one pass. Vertex-light saves can touch
/// hundreds of DFFs; reading and rewriting the complete archive for every DFF
/// made the operation quadratic and kept the UI thread blocked for minutes.
pub(crate) fn upsert_replacement_assets(
    root: &Path,
    replacements: &[(String, Vec<u8>)],
) -> Result<(), String> {
    if replacements.is_empty() {
        return Ok(());
    }
    let replacements = replacements
        .iter()
        .map(|(name, bytes)| {
            let name = normalize_legacy_light_mapper_asset_name(name);
            (lower(&name), name, bytes)
        })
        .collect::<Vec<_>>();
    let replacement_keys = replacements
        .iter()
        .map(|(key, _, _)| key.as_str())
        .collect::<HashSet<_>>();
    let img_dir = root.join("imgs");
    fs::create_dir_all(&img_dir).map_err(|err| format!("Could not create imgs folder: {err}"))?;
    let img_path = img_dir.join(REPLACEMENT_IMG);
    let mut entries = Vec::<(String, Vec<u8>)>::new();
    if img_path.is_file() {
        for entry in parse_img(&img_path) {
            let mut data = read_img_entry(&entry);
            let len = replacement_entry_len(&entry.name, &data);
            data.truncate(len);
            let entry_name = normalize_legacy_light_mapper_asset_name(&entry.name);
            if !replacement_keys.contains(lower(&entry_name).as_str()) {
                entries.push((entry_name, data));
            }
        }
    }
    entries.extend(
        replacements
            .into_iter()
            .map(|(_, name, bytes)| (name, bytes.clone())),
    );
    atomic_write_replacement_archive(&img_path, &entries)
}

fn atomic_write_replacement_archive(
    path: &Path,
    entries: &[(String, Vec<u8>)],
) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{} has no parent directory", path.display()))?;
    let stamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(REPLACEMENT_IMG);
    let temp_path = parent.join(format!(".{file_name}.tmp-{stamp}"));
    write_img_archive(&temp_path, entries)?;
    if parse_img(&temp_path).len() != entries.len() {
        let _ = fs::remove_file(&temp_path);
        return Err(format!(
            "Validation failed while writing temporary replacement IMG {}",
            temp_path.display()
        ));
    }
    if let Err(err) = fs::rename(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        return Err(format!(
            "Could not atomically replace {}: {err}",
            path.display()
        ));
    }
    Ok(())
}

pub(crate) fn upsert_replacement_dff(
    root: &Path,
    dff_name: &str,
    bytes: &[u8],
) -> Result<(), String> {
    upsert_replacement_asset(root, dff_name, bytes)
}

pub(crate) fn upsert_replacement_txd(
    root: &Path,
    txd_name: &str,
    bytes: &[u8],
) -> Result<(), String> {
    upsert_replacement_asset(root, txd_name, bytes)
}

pub(crate) fn write_img_archive(path: &Path, entries: &[(String, Vec<u8>)]) -> Result<(), String> {
    let dir_len = 8usize.saturating_add(entries.len().saturating_mul(32));
    let mut offset_sector = dir_len.div_ceil(2048).max(1) as u32;
    let mut header = Vec::with_capacity(offset_sector as usize * 2048);
    header.extend_from_slice(b"VER2");
    header.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    for (name, bytes) in entries {
        let sectors = bytes.len().div_ceil(2048).max(1);
        header.extend_from_slice(&offset_sector.to_le_bytes());
        header.extend_from_slice(&(sectors as u16).to_le_bytes());
        header.extend_from_slice(&0u16.to_le_bytes());
        let mut raw_name = [0u8; 24];
        let name_bytes = name.as_bytes();
        let count = name_bytes.len().min(raw_name.len());
        raw_name[..count].copy_from_slice(&name_bytes[..count]);
        header.extend_from_slice(&raw_name);
        offset_sector = offset_sector.saturating_add(sectors as u32);
    }
    header.resize((dir_len.div_ceil(2048).max(1)) * 2048, 0);
    let file =
        fs::File::create(path).map_err(|err| format!("Could not write replacement IMG: {err}"))?;
    let mut writer = std::io::BufWriter::new(file);
    writer
        .write_all(&header)
        .map_err(|err| format!("Could not write replacement IMG: {err}"))?;
    let padding = [0u8; 2048];
    for (_, bytes) in entries {
        let sectors = bytes.len().div_ceil(2048).max(1);
        let expected_len = sectors * 2048;
        if bytes.len() > expected_len {
            return Err("IMG entry size overflow".to_string());
        }
        writer
            .write_all(bytes)
            .map_err(|err| format!("Could not write replacement IMG: {err}"))?;
        writer
            .write_all(&padding[..expected_len - bytes.len()])
            .map_err(|err| format!("Could not write replacement IMG: {err}"))?;
    }
    writer
        .flush()
        .map_err(|err| format!("Could not write replacement IMG: {err}"))
}

pub(crate) fn safe_write_img_archive(
    path: &Path,
    entries: &[(String, Vec<u8>)],
) -> Result<PathBuf, String> {
    if entries.is_empty() {
        return Err("Refusing to save an empty IMG archive".to_string());
    }
    let mut seen = HashSet::new();
    for (name, bytes) in entries {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err("IMG entry names cannot be empty".to_string());
        }
        if trimmed.as_bytes().len() > IMG_RUNTIME_SAFE_ENTRY_NAME_BYTES {
            return Err(format!(
                "{trimmed}: IMG entry names must leave room for a terminator in the 24-byte RenderWare directory field"
            ));
        }
        if !seen.insert(lower(trimmed)) {
            return Err(format!("{trimmed}: duplicate IMG entry name"));
        }
        let sectors = bytes.len().div_ceil(2048).max(1);
        if sectors > u16::MAX as usize {
            return Err(format!(
                "{trimmed}: entry is too large for a VER2 IMG directory"
            ));
        }
    }
    let parent = path
        .parent()
        .ok_or_else(|| format!("{} has no parent directory", path.display()))?;
    fs::create_dir_all(parent).map_err(|err| format!("{}: {err}", parent.display()))?;
    let stem = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("archive.img");
    let stamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    let tmp_path = parent.join(format!("{stem}.eagle_tmp_{stamp}"));
    let backup_path = parent.join(format!("{stem}.eagle_backup_{stamp}"));
    if tmp_path.exists() {
        let _ = fs::remove_file(&tmp_path);
    }
    write_img_archive(&tmp_path, entries)?;
    let parsed = parse_img(&tmp_path);
    if parsed.len() != entries.len() {
        let _ = fs::remove_file(&tmp_path);
        return Err(format!(
            "Validation failed after temp write: expected {} entries, parsed {}",
            entries.len(),
            parsed.len()
        ));
    }
    for ((expected_name, expected_bytes), parsed_entry) in entries.iter().zip(parsed.iter()) {
        if !parsed_entry.name.eq_ignore_ascii_case(expected_name) {
            let _ = fs::remove_file(&tmp_path);
            return Err(format!(
                "Validation failed after temp write: expected {expected_name}, parsed {}",
                parsed_entry.name
            ));
        }
        let actual = read_img_entry(parsed_entry);
        let expected_len = replacement_entry_len(expected_name, expected_bytes);
        let actual_len = replacement_entry_len(expected_name, &actual);
        if actual_len != expected_len
            || actual.get(..actual_len) != expected_bytes.get(..expected_len)
        {
            let _ = fs::remove_file(&tmp_path);
            return Err(format!(
                "Validation failed after temp write: {expected_name} bytes differ"
            ));
        }
    }
    if path.exists() {
        fs::copy(path, &backup_path).map_err(|err| {
            format!(
                "Could not create IMG backup {}: {err}",
                backup_path.display()
            )
        })?;
    }
    if let Err(err) = fs::rename(&tmp_path, path) {
        let _ = fs::remove_file(&tmp_path);
        if backup_path.exists() {
            let _ = fs::copy(&backup_path, path);
        }
        return Err(format!("Could not replace {}: {err}", path.display()));
    }
    let reparsed = parse_img(path);
    if reparsed.len() != entries.len() {
        if backup_path.exists() {
            let _ = fs::copy(&backup_path, path);
        }
        return Err(
            "Validation failed after archive replacement; backup restored if possible".to_string(),
        );
    }
    Ok(backup_path)
}

pub(crate) const RW_VERSION: u32 = 0x1803ffff;

pub(crate) fn rw_chunk(id: u32, data: Vec<u8>) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + 12);
    out.extend_from_slice(&id.to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&RW_VERSION.to_le_bytes());
    out.extend_from_slice(&data);
    out
}

pub(crate) fn rw_string(value: &str) -> Vec<u8> {
    let mut bytes = value.as_bytes().to_vec();
    bytes.push(0);
    while bytes.len() % 4 != 0 {
        bytes.push(0);
    }
    bytes
}
