use super::super::*;

pub(crate) type SaScene = (
    Vec<String>,
    HashMap<String, Definition>,
    HashSet<String>,
    Vec<Placement>,
    Vec<EditorLight>,
    EagleZoneOffsets,
    bool,
);

#[derive(Clone, Debug)]
struct Instance {
    model: String,
    interior: u32,
    pos: V3,
    quaternion: [f32; 4],
    lod: i32,
}

// Area 13 is visible in every area, including the exterior world. It is not
// an interior to discard. The upper bits of the IPL field are instance flags.
// Matches CEntity::IsInCurrentArea in GTA SA.
fn visible_in_exterior(area_flags: u32) -> bool {
    matches!(area_flags & 0xff, 0 | 13)
}

// IPL quaternions use the opposite rotation direction to the editor's XYZ
// Euler angles (the game conjugates the quaternion before applying it).
fn ipl_rotation(q: [f32; 4]) -> Option<V3> {
    let length = q.iter().map(|v| v * v).sum::<f32>().sqrt();
    if !length.is_finite() || length < 1e-6 {
        return None;
    }
    let [x, y, z, w] = [
        -q[0] / length,
        -q[1] / length,
        -q[2] / length,
        q[3] / length,
    ];
    let sy = (2.0 * (w * y - z * x)).clamp(-1.0, 1.0);
    let ry = sy.asin();
    let (rx, rz) = if ry.cos().abs() > 1e-5 {
        (
            (2.0 * (w * x + y * z)).atan2(1.0 - 2.0 * (x * x + y * y)),
            (2.0 * (w * z + x * y)).atan2(1.0 - 2.0 * (y * y + z * z)),
        )
    } else {
        (
            0.0,
            (2.0 * (w * z - x * y)).atan2(1.0 - 2.0 * (x * x + z * z)),
        )
    };
    Some(V3 {
        x: rx.to_degrees(),
        y: ry.to_degrees(),
        z: rz.to_degrees(),
    })
}

fn parse_text_instance(line: &str) -> Option<Instance> {
    let p: Vec<_> = line.split(',').map(str::trim).collect();
    if p.len() != 11 {
        return None;
    }
    Some(Instance {
        model: p[0].parse::<u32>().ok()?.to_string(),
        interior: p[2].parse().ok()?,
        pos: V3 {
            x: p[3].parse().ok()?,
            y: p[4].parse().ok()?,
            z: p[5].parse().ok()?,
        },
        quaternion: [
            p[6].parse().ok()?,
            p[7].parse().ok()?,
            p[8].parse().ok()?,
            p[9].parse().ok()?,
        ],
        lod: p[10].parse().ok()?,
    })
}

fn parse_ipl(bytes: &[u8]) -> Result<Vec<Instance>, String> {
    if bytes.starts_with(b"bnry") {
        if bytes.len() < 76 {
            return Err("Truncated binary IPL header".into());
        }
        let word = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        let count = word(4) as usize;
        let offset = word(28) as usize;
        if offset < 76 || offset > bytes.len() || count > (bytes.len() - offset) / 40 {
            return Err("Invalid binary IPL instance range".into());
        }
        return Ok(bytes[offset..offset + count * 40]
            .chunks_exact(40)
            .map(|row| {
                let int = |at: usize| u32::from_le_bytes(row[at..at + 4].try_into().unwrap());
                let float = |at: usize| f32::from_bits(int(at));
                Instance {
                    model: int(28).to_string(),
                    interior: int(32),
                    pos: V3 {
                        x: float(0),
                        y: float(4),
                        z: float(8),
                    },
                    quaternion: [float(12), float(16), float(20), float(24)],
                    lod: int(36) as i32,
                }
            })
            .collect());
    }
    let text = String::from_utf8_lossy(bytes);
    let mut in_instances = false;
    let mut instances = Vec::new();
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.eq_ignore_ascii_case("end") {
            in_instances = false;
        } else if line.eq_ignore_ascii_case("inst") {
            in_instances = true;
        } else if in_instances {
            // Fail instead of shifting the indices used by LOD references.
            instances.push(
                parse_text_instance(line).ok_or_else(|| format!("Invalid IPL instance: {line}"))?,
            );
        }
    }
    Ok(instances)
}

// gta.dat uses Windows casing and separators even on case-sensitive hosts.
fn game_path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let mut path = root.to_path_buf();
    for component in relative.split(['/', '\\']).filter(|s| !s.is_empty()) {
        if component == ".." || component.contains(':') {
            return Err("Invalid GTA data path".into());
        }
        let exact = path.join(component);
        if exact.exists() {
            path = exact;
            continue;
        }
        path = fs::read_dir(&path)
            .ok()
            .and_then(|entries| {
                entries
                    .filter_map(Result::ok)
                    .find(|entry| {
                        entry
                            .file_name()
                            .to_string_lossy()
                            .eq_ignore_ascii_case(component)
                    })
                    .map(|entry| entry.path())
            })
            .ok_or_else(|| format!("Missing SA map file: {}", exact.display()))?;
    }
    Ok(path)
}

pub(crate) fn load_default_sa_scene(root: &Path) -> Result<SaScene, String> {
    validate_gta_sa_dir(root)?;
    let manifest = game_path(root, "data/gta.dat")?;
    let text =
        fs::read_to_string(&manifest).map_err(|err| format!("Cannot read gta.dat: {err}"))?;
    let defs = load_gta_sa_definitions(root);
    if defs.is_empty() {
        return Err("No SA map definitions found in data/maps.".into());
    }
    let mut groups = BTreeMap::<String, Vec<Instance>>::new();
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        let Some((kind, relative)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        if !kind.eq_ignore_ascii_case("IPL") {
            continue;
        }
        let path = game_path(root, relative.trim())?;
        let bytes = fs::read(&path).map_err(|err| format!("{}: {err}", path.display()))?;
        let instances = parse_ipl(&bytes).map_err(|err| format!("{}: {err}", path.display()))?;
        let name = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_ascii_lowercase();
        groups.insert(name, instances);
    }
    let mut streams = BTreeMap::new();
    for archive in gta_sa_img_files(root) {
        for entry in parse_img(&archive) {
            let name = entry.name.to_ascii_lowercase();
            let Some(stem) = name.strip_suffix(".ipl") else {
                continue;
            };
            let Some((base, _)) = stem.rsplit_once("_stream") else {
                continue;
            };
            if !groups.contains_key(base) {
                continue;
            }
            let instances = parse_ipl(&read_img_entry(&entry))
                .map_err(|err| format!("{} / {}: {err}", archive.display(), entry.name))?;
            streams.insert(stem.to_string(), (base.to_string(), instances));
        }
    }
    let mut placements = Vec::new();
    let mut zones = Vec::new();
    for (zone, instances) in &groups {
        append_instances(zone, instances, instances, &defs, &mut placements);
        if !instances.is_empty() {
            zones.push(zone.clone());
        }
    }
    for (zone, (base, instances)) in &streams {
        // Streamed LOD indices point into their parent text IPL, not the stream.
        append_instances(zone, instances, &groups[base], &defs, &mut placements);
        if !instances.is_empty() {
            zones.push(zone.clone());
        }
    }
    if placements.is_empty() {
        return Err("No exterior SA map placements found.".into());
    }
    let readonly = defs.keys().cloned().collect();
    Ok((
        zones,
        defs,
        readonly,
        placements,
        default_lights(),
        EagleZoneOffsets::default(),
        false,
    ))
}

fn append_instances(
    zone: &str,
    instances: &[Instance],
    lods: &[Instance],
    defs: &HashMap<String, Definition>,
    out: &mut Vec<Placement>,
) {
    for (index, item) in instances.iter().enumerate() {
        if !visible_in_exterior(item.interior) {
            continue;
        }
        let Some(def) = defs.get(&item.model) else {
            continue;
        };
        let Some(rot) = ipl_rotation(item.quaternion) else {
            continue;
        };
        if ![item.pos.x, item.pos.y, item.pos.z]
            .iter()
            .all(|v| v.is_finite())
        {
            continue;
        }
        let mut attrs = BTreeMap::from([
            ("id".into(), item.model.clone()),
            ("interior".into(), "0".into()),
            ("saArea".into(), (item.interior & 0xff).to_string()),
            ("saIpl".into(), zone.into()),
            ("saInstance".into(), index.to_string()),
        ]);
        for (key, value) in [
            ("posX", item.pos.x),
            ("posY", item.pos.y),
            ("posZ", item.pos.z),
            ("rotX", rot.x),
            ("rotY", rot.y),
            ("rotZ", rot.z),
        ] {
            attrs.insert(key.into(), value.to_string());
        }
        if item.lod >= 0 {
            if let Some(lod) = lods
                .get(item.lod as usize)
                .filter(|lod| visible_in_exterior(lod.interior) && defs.contains_key(&lod.model))
            {
                attrs.insert("lodParent".into(), lod.model.clone());
                attrs.insert(
                    "saLodIpl".into(),
                    zone.split("_stream").next().unwrap_or(zone).into(),
                );
                attrs.insert("saLodInstance".into(), item.lod.to_string());
            }
        }
        out.push(Placement {
            id: item.model.clone(),
            dff: def.attrs.get("dff").cloned().unwrap_or_default(),
            zone: zone.into(),
            tag: "building".into(),
            attrs,
            pos: item.pos,
            rot,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_ipl_keeps_lod_indices_and_converts_rotation() {
        let rows = parse_ipl(b"# test\ninst\n100, test, 256, 1, 2, 3, 0, 0, -0.70710678, 0.70710678, 1\n101, lod, 0, 1, 2, 3, 0, 0, 0, 1, -1\nend\n").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].lod, 1);
        assert_eq!(rows[0].interior & 255, 0);
        assert!((ipl_rotation(rows[0].quaternion).unwrap().z - 90.0).abs() < 0.001);
        assert!(parse_ipl(b"inst\ninvalid\nend").is_err());
    }

    #[test]
    fn binary_ipl_uses_declared_offset_and_rejects_truncation() {
        let mut bytes = vec![0; 120];
        bytes[..4].copy_from_slice(b"bnry");
        bytes[4..8].copy_from_slice(&1u32.to_le_bytes());
        bytes[28..32].copy_from_slice(&80u32.to_le_bytes());
        bytes[80..84].copy_from_slice(&12.5f32.to_le_bytes());
        bytes[104..108].copy_from_slice(&1f32.to_le_bytes());
        bytes[108..112].copy_from_slice(&100u32.to_le_bytes());
        bytes[116..120].copy_from_slice(&(-1i32).to_le_bytes());
        let rows = parse_ipl(&bytes).unwrap();
        assert_eq!(rows[0].model, "100");
        assert_eq!(rows[0].pos.x, 12.5);
        assert_eq!(rows[0].lod, -1);
        assert!(parse_ipl(&bytes[..119]).is_err());
        bytes[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(parse_ipl(&bytes).is_err());
    }

    #[test]
    fn streamed_lod_uses_parent_and_filters_interiors() {
        let rows = parse_ipl(b"inst\n100, test, 256, 1, 2, 3, 0, 0, 0, 1, 0\n100, test, 1, 1, 2, 3, 0, 0, 0, 1, -1\nend").unwrap();
        let lods = parse_ipl(b"inst\n101, lod, 0, 1, 2, 3, 0, 0, 0, 1, -1\nend").unwrap();
        let defs = ["100, test, generic, 100, 0", "101, lod, generic, 300, 0"]
            .into_iter()
            .map(|line| {
                let def = parse_gta_sa_ide_line(line, "objs").unwrap();
                (def.id.clone(), def)
            })
            .collect();
        let mut placements = Vec::new();
        append_instances("test_stream0", &rows, &lods, &defs, &mut placements);
        assert_eq!(placements.len(), 1);
        assert_eq!(placements[0].attrs["lodParent"], "101");
    }

    #[test]
    fn exterior_includes_universal_area_with_instance_flags() {
        for area in [0, 13, 256, 256 | 13, 512 | 13] {
            assert!(visible_in_exterior(area), "exterior-visible area {area}");
        }
        for area in [1, 2, 10, 14, 17, 18, 256 | 1] {
            assert!(!visible_in_exterior(area), "interior area {area}");
        }
    }

    #[test]
    fn universal_area_preserves_detail_and_lod_placements() {
        let rows = parse_ipl(b"inst\n100, road, 269, 1, 2, 3, 0, 0, 0, 1, 1\n101, lod, 13, 1, 2, 3, 0, 0, 0, 1, -1\n100, road, 1, 1, 2, 3, 0, 0, 0, 1, -1\nend").unwrap();
        let defs = ["100, road, generic, 100, 0", "101, lod, generic, 300, 0"]
            .into_iter()
            .map(|line| {
                let def = parse_gta_sa_ide_line(line, "objs").unwrap();
                (def.id.clone(), def)
            })
            .collect();
        let mut placements = Vec::new();
        append_instances("test", &rows, &rows, &defs, &mut placements);
        assert_eq!(placements.len(), 2);
        assert_eq!(placements[0].attrs["lodParent"], "101");
        assert_eq!(placements[0].attrs["saArea"], "13");
        assert_eq!(placements[0].attrs["interior"], "0");
        let lod_ids = collect_lod_ids(&placements);
        assert!(!placement_is_lod(&placements[0], &lod_ids));
        assert!(placement_is_lod(&placements[1], &lod_ids));
    }

    #[test]
    #[ignore = "requires a local SA installation; set EAGLE_SA_TEST_ROOT"]
    fn loads_installed_sa_map() {
        let root = PathBuf::from(env::var("EAGLE_SA_TEST_ROOT").unwrap());
        let (zones, defs, _, placements, _, _, _) = load_default_sa_scene(&root).unwrap();
        assert!(placements.len() > 10_000);
        assert!(zones.iter().any(|zone| zone.contains("_stream")));
        assert!(placements.iter().all(|p| defs.contains_key(&p.id)));
        // Real combined and timed IDE rows must retain their native flags.
        let ivy = &defs["9812"];
        assert!(definition_flag_enabled(ivy, "disable_backface_culling"));
        assert!(definition_flag_enabled(ivy, "draw_last"));
        let night_lights = &defs["9885"];
        assert_eq!(night_lights.attrs["timeIn"], "21");
        assert_eq!(night_lights.attrs["timeOut"], "6");
        assert_eq!(night_lights.attrs["drawDistance"], "900");
        assert!(definition_flag_enabled(night_lights, "additive"));
        assert!(definition_flag_enabled(
            night_lights,
            "dont_receive_shadows"
        ));
        let double_sided_models = defs
            .values()
            .filter(|def| {
                def.id.parse::<u32>().is_ok()
                    && definition_flag_enabled(def, "disable_backface_culling")
            })
            .count();
        assert!(double_sided_models > 1000);
        eprintln!("Loaded {double_sided_models} double-sided native model definitions");
        // These real exterior roads/buildings use universal area 13. The old
        // area-zero-only filter dropped them and left holes in LA and SF.
        for (zone, model) in [("lan", "4156"), ("sfse_stream5", "11340")] {
            let placement = placements
                .iter()
                .find(|p| p.zone == zone && p.id == model)
                .unwrap_or_else(|| panic!("Missing exterior model {model} in {zone}"));
            assert_eq!(placement.attrs["saArea"], "13");
            let parent = &placement.attrs["lodParent"];
            assert!(placements.iter().any(|p| &p.id == parent));
        }
        eprintln!(
            "Loaded {} exterior placements across {} IPLs",
            placements.len(),
            zones.len()
        );
    }
}

// IPL provenance is only retained on immutable game-world instances. Editable
// copies deliberately lose it, so duplicating a copy never removes more world.
pub(crate) fn is_default_world_placement(p: &Placement) -> bool {
    p.attrs.contains_key("saIpl") && p.attrs.contains_key("saInstance")
}
pub(crate) fn selection_has_default_world(app: &AppState) -> bool {
    selected_indices(app)
        .iter()
        .any(|&i| is_default_world_placement(&app.placements[i]))
}
pub(crate) fn selected_editable_indices(app: &AppState) -> Vec<usize> {
    selected_live_indices(app)
        .into_iter()
        .filter(|&i| !is_default_world_placement(&app.placements[i]))
        .collect()
}
pub(crate) fn world_conversion_rect(app: &AppState) -> Rect {
    let l = element_panel_layout(app);
    Rect::new(
        l.content.x + 6.0,
        l.info_top + 362.0,
        right_panel_width() - 52.0,
        28.0,
    )
}
const WORLD_EDITS: &str = "maps/world_edits.map";

fn removal_tag(p: &Placement) -> String {
    // MTA removes instances by model, original position, radius and interior.
    // Keep the radius small to avoid deleting adjacent instances of that model.
    let attrs = BTreeMap::from([
        (
            "id".into(),
            format!("removed_{}_{}", p.attrs["saIpl"], p.attrs["saInstance"]),
        ),
        ("model".into(), p.id.clone()),
        ("lodModel".into(), "0".into()),
        ("radius".into(), "0.1".into()),
        (
            "interior".into(),
            p.attrs.get("saArea").cloned().unwrap_or_else(|| "0".into()),
        ),
        ("posX".into(), p.pos.x.to_string()),
        ("posY".into(), p.pos.y.to_string()),
        ("posZ".into(), p.pos.z.to_string()),
        ("rotX".into(), "0".into()),
        ("rotY".into(), "0".into()),
        ("rotZ".into(), "0".into()),
    ]);
    crate::resource::save::write_tag("removeWorldObject", &attrs)
}

pub(crate) fn remove_default_world_element(app: &mut AppState, index: usize) {
    let Some(p) = app
        .placements
        .get(index)
        .filter(|p| is_default_world_placement(p))
        .cloned()
    else {
        return;
    };
    if app.element_states.get(index).is_some_and(|s| s.deleted) {
        return;
    }
    if !app.map_documents.iter().any(|d| d.path == WORLD_EDITS) {
        app.map_documents
            .push(crate::resource::mta_maps::MapDocument {
                path: WORLD_EDITS.into(),
                text: "<map>\n</map>\n".into(),
            });
        let zone = crate::resource::mta_maps::map_zone(WORLD_EDITS);
        if !app.zones.contains(&zone) {
            app.zones.push(zone);
        }
    }
    let tag = removal_tag(&p);
    let doc = app
        .map_documents
        .iter_mut()
        .find(|d| d.path == WORLD_EDITS)
        .unwrap();
    let at = doc.text.rfind("</map>").unwrap();
    doc.text.insert_str(at, &tag);
    app.element_states[index].deleted = true;
    // A streamed IPL's LOD belongs to its parent text IPL. Resolve the
    // instance index, never just the model ID (which may occur many times).
    if let (Some(ipl), Some(instance)) = (p.attrs.get("saLodIpl"), p.attrs.get("saLodInstance")) {
        if let Some(lod) = app.placements.iter().position(|q| {
            q.attrs.get("saIpl") == Some(ipl) && q.attrs.get("saInstance") == Some(instance)
        }) {
            remove_default_world_element(app, lod);
        }
    }
}

pub(crate) fn turn_world_into_placement(app: &mut AppState) {
    let indices: Vec<_> = selected_live_indices(app)
        .into_iter()
        .filter(|&i| is_default_world_placement(&app.placements[i]))
        .collect();
    if indices.is_empty() {
        return;
    }
    let before = local_world_history_snapshot(app, [], []);
    let mut selection = BTreeSet::new();
    for i in indices {
        let mut copy = app.placements[i].clone();
        remove_default_world_element(app, i);
        copy.attrs
            .retain(|k, _| !k.starts_with("sa") && k != "id" && k != "lodParent");
        copy.attrs.insert("interior".into(), "0".into());
        copy.zone = crate::resource::mta_maps::map_zone(WORLD_EDITS);
        copy.tag = "object".into();
        sync_placement_attrs(&mut copy);
        let index = app.placements.len();
        app.placements.push(copy);
        app.element_states.push(ElementState::default());
        app.outliner_labels.push(None);
        selection.insert(index);
        app.selected = index;
    }
    app.selected_elements = selection;
    app.selected_element_order = app.selected_elements.iter().copied().collect();
    app.inspector_edit = None;
    commit_local_world_history(app, "Turn into placement", before);
    app.status_message = "World objects converted to editable placements".into();
}

pub(crate) fn restore_default_world_element(app: &mut AppState, index: usize) {
    let Some(p) = app
        .placements
        .get(index)
        .filter(|p| is_default_world_placement(p))
        .cloned()
    else {
        return;
    };
    if !app.element_states.get(index).is_some_and(|s| s.deleted) {
        return;
    }
    if let Some(doc) = app.map_documents.iter_mut().find(|d| d.path == WORLD_EDITS) {
        doc.text = doc.text.replace(&removal_tag(&p), "");
    }
    app.element_states[index].deleted = false;
    if let (Some(ipl), Some(instance)) = (p.attrs.get("saLodIpl"), p.attrs.get("saLodInstance")) {
        if let Some(lod) = app.placements.iter().position(|q| {
            q.attrs.get("saIpl") == Some(ipl) && q.attrs.get("saInstance") == Some(instance)
        }) {
            restore_default_world_element(app, lod);
        }
    }
}

#[cfg(test)]
mod world_edit_tests {
    use super::*;
    #[test]
    fn world_removal_retains_original_position_and_area() {
        let p = Placement {
            id: "1337".into(),
            dff: "bin".into(),
            zone: "test".into(),
            tag: "building".into(),
            attrs: BTreeMap::from([
                ("saIpl".into(), "test".into()),
                ("saInstance".into(), "4".into()),
                ("saArea".into(), "13".into()),
            ]),
            pos: V3 {
                x: 10.0,
                y: 20.0,
                z: 30.0,
            },
            rot: V3::default(),
        };
        assert!(is_default_world_placement(&p));
        let tag = removal_tag(&p);
        for attr in [
            "model=\"1337\"",
            "interior=\"13\"",
            "posX=\"10\"",
            "radius=\"0.1\"",
            "lodModel=\"0\"",
        ] {
            assert!(tag.contains(attr), "{tag}");
        }
        let mut copy = p.clone();
        copy.attrs.retain(|k, _| !k.starts_with("sa"));
        assert!(!is_default_world_placement(&copy));
        assert_eq!(p.pos.x, 10.0);
    }
}
