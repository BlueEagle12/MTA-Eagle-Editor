//! Standard MTA map documents remain separate from Eagle zone maps.
use super::super::*;
use quick_xml::{
    Reader,
    events::{BytesStart, Event},
};

pub(crate) const REGISTRY: &str = "eagleMaps.json";
const PREFIX: &str = "map:";
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct MapDocument {
    pub path: String,
    pub text: String,
}

pub(crate) fn map_path(zone: &str) -> Option<&str> {
    zone.strip_prefix(PREFIX)
}
pub(crate) fn map_zone(path: &str) -> String {
    format!("{PREFIX}{path}")
}
pub(crate) fn destination_label(zone: &str) -> String {
    map_path(zone)
        .map(|p| format!("Map {p}"))
        .unwrap_or_else(|| format!("Zone {zone}"))
}
pub(crate) fn safe_map_path(path: &str) -> bool {
    !path.contains(['\\', ':'])
        && !path.chars().any(char::is_control)
        && path
            .split('/')
            .all(|p| !p.is_empty() && p != "." && p != "..")
        && Path::new(path)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("map"))
}
fn checked_path(root: &Path, path: &str) -> Result<PathBuf, String> {
    if !safe_map_path(path) {
        return Err(format!("Invalid project map path: {path}"));
    }
    let base = root.canonicalize().map_err(|e| e.to_string())?;
    let target = root.join(path);
    let mut ancestor = target.as_path();
    while !ancestor.exists() {
        ancestor = ancestor.parent().ok_or("Invalid map path")?;
    }
    if !ancestor
        .canonicalize()
        .map_err(|e| e.to_string())?
        .starts_with(&base)
    {
        return Err(format!("Map path leaves the project: {path}"));
    }
    Ok(target)
}
fn attributes(
    tag: &BytesStart<'_>,
    reader: &Reader<&[u8]>,
) -> Result<BTreeMap<String, String>, String> {
    tag.attributes()
        .map(|a| {
            let a = a.map_err(|e| e.to_string())?;
            Ok((
                String::from_utf8_lossy(a.key.as_ref()).into_owned(),
                a.decode_and_unescape_value(reader.decoder())
                    .map_err(|e| e.to_string())?
                    .into_owned(),
            ))
        })
        .collect()
}
// Byte ranges let us preserve comments, unknown elements, and nested data exactly.
struct Node {
    start: usize,
    open_end: usize,
    close_start: usize,
    end: usize,
    tag: String,
    attrs: BTreeMap<String, String>,
}
fn nodes(text: &str, root_tag: &str) -> Result<(Vec<Node>, usize), String> {
    let mut reader = Reader::from_str(text);
    let mut depth = 0usize;
    let mut result = Vec::<Node>::new();
    let mut current = None;
    let mut close = None;
    let mut found_root = false;
    loop {
        let start = reader.buffer_position() as usize;
        match reader.read_event().map_err(|e| e.to_string())? {
            Event::Start(e) => {
                if depth == 0 {
                    if found_root || e.name().as_ref() != root_tag.as_bytes() {
                        return Err(format!("Expected <{root_tag}> document"));
                    }
                    found_root = true;
                } else if depth == 1 {
                    result.push(Node {
                        start,
                        open_end: reader.buffer_position() as usize,
                        close_start: 0,
                        end: 0,
                        tag: String::from_utf8_lossy(e.name().as_ref()).into_owned(),
                        attrs: attributes(&e, &reader)?,
                    });
                    current = Some(result.len() - 1);
                }
                depth += 1;
            }
            Event::Empty(e) if depth == 1 => result.push(Node {
                start,
                open_end: reader.buffer_position() as usize,
                close_start: reader.buffer_position() as usize,
                end: reader.buffer_position() as usize,
                tag: String::from_utf8_lossy(e.name().as_ref()).into_owned(),
                attrs: attributes(&e, &reader)?,
            }),
            Event::End(_) => {
                depth = depth.checked_sub(1).ok_or("Unexpected closing XML tag")?;
                if depth == 1 {
                    if let Some(i) = current.take() {
                        result[i].close_start = start;
                        result[i].end = reader.buffer_position() as usize;
                    }
                }
                if depth == 0 {
                    close = Some(start);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if depth != 0 {
        return Err("Unclosed XML element".into());
    }
    Ok((
        result,
        close.ok_or_else(|| format!("Missing </{root_tag}>"))?,
    ))
}
fn editable(node: &Node) -> bool {
    matches!(node.tag.as_str(), "object" | "building" | "scenery")
        && node
            .attrs
            .get("model")
            .is_some_and(|m| m.parse::<u32>().is_ok())
}
pub(crate) fn registered_paths(root: &Path) -> Vec<String> {
    if let Ok(text) = fs::read_to_string(root.join(REGISTRY)) {
        if let Ok(paths) = serde_json::from_str::<Vec<String>>(&text) {
            return paths.into_iter().filter(|p| safe_map_path(p)).collect();
        }
    }
    Vec::new()
}
pub(crate) fn discover_paths(root: &Path) -> Vec<String> {
    let mut paths = registered_paths(root);
    if let Ok(text) = fs::read_to_string(root.join("meta.xml")) {
        if let Ok((tags, _)) = nodes(&text, "meta") {
            for tag in tags.into_iter().filter(|t| t.tag == "map") {
                if let Some(path) = tag.attrs.get("src").filter(|p| safe_map_path(p)) {
                    if !paths.contains(path) {
                        paths.push(path.clone());
                    }
                }
            }
        }
    }
    paths
}
pub(crate) fn new_document(root: &Path, path: &str) -> Result<MapDocument, String> {
    checked_path(root, path)?;
    Ok(MapDocument {
        path: path.into(),
        text: "<map>\n</map>\n".into(),
    })
}
pub(crate) fn read_document(root: &Path, path: &str) -> Result<MapDocument, String> {
    let text = fs::read_to_string(checked_path(root, path)?).map_err(|e| format!("{path}: {e}"))?;
    let mut text = text;
    // A newly authored, empty MTA map may use the compact XML root form.
    let mut reader = Reader::from_str(&text);
    loop {
        let start = reader.buffer_position() as usize;
        match reader.read_event().map_err(|e| format!("{path}: {e}"))? {
            Event::Empty(e) if e.name().as_ref() == b"map" => {
                let end = reader.buffer_position() as usize;
                let expanded = format!("{}>\n</map>", &text[start..end - 2]);
                text.replace_range(start..end, &expanded);
                break;
            }
            Event::Start(_) | Event::Empty(_) | Event::Eof => break,
            _ => {}
        }
    }
    nodes(&text, "map").map_err(|e| format!("{path}: {e}"))?;
    Ok(MapDocument {
        path: path.into(),
        text,
    })
}
pub(crate) fn documents_for_source(root: &Path, source: LoadSceneSource) -> Vec<MapDocument> {
    let overlay = match source {
        LoadSceneSource::Saved => wip_root_path(root),
        LoadSceneSource::Autosave => autosave_root_path(root),
    };
    let scene_root = if overlay.join(REGISTRY).is_file() {
        overlay.as_path()
    } else {
        root
    };
    load_documents(root, scene_root)
}
pub(crate) fn load_documents(root: &Path, scene_root: &Path) -> Vec<MapDocument> {
    let paths = if root != scene_root && scene_root.join(REGISTRY).is_file() {
        registered_paths(scene_root)
    } else {
        discover_paths(root)
    };
    paths
        .into_iter()
        .filter_map(|path| {
            let base = if scene_root.join(&path).is_file() {
                scene_root
            } else {
                root
            };
            match read_document(base, &path) {
                Ok(doc) => Some(doc),
                Err(e) => {
                    eprintln!("MTA map: {e}");
                    None
                }
            }
        })
        .collect()
}
pub(crate) fn placements(doc: &MapDocument, defs: &HashMap<String, Definition>) -> Vec<Placement> {
    let Ok((tags, _)) = nodes(&doc.text, "map") else {
        return Vec::new();
    };
    tags.into_iter()
        .enumerate()
        .filter(|(_, n)| editable(n))
        .map(|(index, n)| {
            let children = if n.open_end < n.close_start {
                Some(doc.text[n.open_end..n.close_start].to_string())
            } else {
                None
            };
            let mut attrs = n.attrs;
            if let Some(children) = children {
                attrs.insert("__mapChildren".into(), children);
            }
            let id = attrs["model"].clone();
            attrs.insert("__mapNode".into(), index.to_string());
            attrs.insert("__mapSource".into(), doc.path.clone());
            Placement {
                dff: defs
                    .get(&id)
                    .and_then(|d| d.attrs.get("dff"))
                    .cloned()
                    .unwrap_or_else(|| id.clone()),
                id,
                zone: map_zone(&doc.path),
                tag: n.tag,
                pos: V3 {
                    x: attrf(&attrs, "posX"),
                    y: attrf(&attrs, "posY"),
                    z: attrf(&attrs, "posZ"),
                },
                rot: V3 {
                    x: attrf(&attrs, "rotX"),
                    y: attrf(&attrs, "rotY"),
                    z: attrf(&attrs, "rotZ"),
                },
                attrs,
            }
        })
        .collect()
}
fn opening(p: &Placement, empty: bool) -> Result<String, String> {
    if p.id.parse::<u32>().is_err() {
        return Err(format!("{} is not a GTA:SA model ID for {}", p.id, p.zone));
    }
    let mut copy = p.clone();
    sync_placement_attrs(&mut copy);
    copy.attrs.insert("model".into(), p.id.clone());
    let mut out = format!("<{}", p.tag);
    for (key, value) in copy
        .attrs
        .iter()
        .filter(|(k, _)| !matches!(k.as_str(), "__mapNode" | "__mapSource" | "__mapChildren"))
    {
        out.push_str(&format!(
            " {key}=\"{}\"",
            crate::resource::save::xml_escape(value)
        ));
    }
    out.push_str(if empty { "/>" } else { ">" });
    Ok(out)
}
pub(crate) fn render_document(
    doc: &MapDocument,
    placements: &[Placement],
    states: &[ElementState],
) -> Result<String, String> {
    let (tags, close) = nodes(&doc.text, "map")?;
    let zone = map_zone(&doc.path);
    let selected: Vec<_> = placements
        .iter()
        .enumerate()
        .filter(|(i, p)| p.zone == zone && !states.get(*i).is_some_and(|s| s.deleted))
        .map(|(_, p)| p)
        .collect();
    let mut used = HashSet::new();
    let mut out = String::new();
    let mut cursor = 0;
    let mut originals = HashMap::new();
    for (i, p) in selected.iter().enumerate() {
        if p.attrs.get("__mapSource") == Some(&doc.path) {
            if let Some(node) = p
                .attrs
                .get("__mapNode")
                .and_then(|n| n.parse::<usize>().ok())
            {
                originals.entry(node).or_insert(i);
            }
        }
    }
    for (index, node) in tags.iter().enumerate().filter(|(_, n)| editable(n)) {
        out.push_str(&doc.text[cursor..node.start]);
        if let Some(&i) = originals.get(&index) {
            let p = selected[i];
            used.insert(i);
            out.push_str(&opening(p, node.open_end == node.end)?);
            if node.open_end != node.end {
                out.push_str(&doc.text[node.open_end..node.close_start]);
                if p.tag == node.tag {
                    out.push_str(&doc.text[node.close_start..node.end]);
                } else {
                    out.push_str(&format!("</{}>", p.tag));
                }
            }
        }
        cursor = node.end;
    }
    out.push_str(&doc.text[cursor..close]);
    let mut ids: HashSet<String> = selected
        .iter()
        .enumerate()
        .filter(|(i, _)| used.contains(i))
        .filter_map(|(_, p)| p.attrs.get("id").cloned())
        .collect();
    for (_, p) in selected
        .iter()
        .enumerate()
        .filter(|(i, _)| !used.contains(i))
    {
        let mut copy = (*p).clone();
        if copy
            .attrs
            .get("id")
            .is_some_and(|id| !ids.insert(id.clone()))
        {
            copy.attrs.remove("id");
        }
        out.push_str("    ");
        if let Some(children) = copy.attrs.get("__mapChildren") {
            out.push_str(&opening(&copy, false)?);
            out.push_str(children);
            out.push_str(&format!("</{}>", copy.tag));
        } else {
            out.push_str(&opening(&copy, true)?);
        }
        out.push('\n');
    }
    out.push_str(&doc.text[close..]);
    Ok(out)
}
pub(crate) fn save_documents(
    root: &Path,
    docs: &[MapDocument],
    placements: &[Placement],
    states: &[ElementState],
) -> Result<(), Vec<String>> {
    let mut writes = Vec::new();
    for doc in docs {
        let path = checked_path(root, &doc.path).map_err(|e| vec![e])?;
        let text = render_document(doc, placements, states).map_err(|e| vec![e])?;
        writes.push((path, text));
    }
    for (path, text) in writes {
        fs::create_dir_all(path.parent().unwrap())
            .and_then(|_| fs::write(&path, text))
            .map_err(|e| vec![format!("{}: {e}", path.display())])?;
    }
    if docs.iter().any(|d| d.path == "maps/world_edits.map") {
        save_world_removal_runtime(root).map_err(|e| vec![e])?;
    }
    let paths: Vec<_> = docs.iter().map(|d| &d.path).collect();
    fs::write(
        root.join(REGISTRY),
        serde_json::to_string_pretty(&paths).unwrap(),
    )
    .map_err(|e| vec![e.to_string()])
}

#[cfg(test)]
mod tests {
    use super::*;
    fn root(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "eagle_mta_map_{name}_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }
    fn document() -> MapDocument {
        MapDocument { path: "maps/main.map".into(), text: "<?xml version=\"1.0\"?>\n<map edf:definitions=\"editor_main\">\n<!-- preserve -->\n<info name=\"A &amp; B\"/>\n<object id='unique &amp; one' model='1337' scale='2' posX='1' posY='2' posZ='3' rotZ='45'><data name='custom' value='untouched'/></object>\n<vehicle model='411' id='car'/>\n<object model=\"1337\" posX=\"4\"/>\n</map>\n".into() }
    }
    #[test]
    fn discovers_only_meta_maps_and_explicit_registration() {
        let r = root("discover");
        fs::write(r.join("meta.xml"),"<meta><!-- <map src='ignored.map'/> --><file src='alternative.map'/><map src = 'maps/main.map' dimension='7'/><map src='maps/main.map'/><map src='../escape.map'/></meta>").unwrap();
        fs::write(r.join(REGISTRY), r#"["alternative.map"]"#).unwrap();
        assert_eq!(discover_paths(&r), vec!["alternative.map", "maps/main.map"]);
        fs::remove_dir_all(r).unwrap();
    }
    #[test]
    fn model_ids_are_distinct_from_optional_element_ids() {
        let doc = document();
        let ps = placements(&doc, &HashMap::new());
        assert_eq!(ps.len(), 2);
        assert_eq!(ps[0].id, "1337");
        assert_eq!(ps[0].attrs["id"], "unique & one");
        assert_eq!(ps[0].zone, "map:maps/main.map");
        assert_eq!(ps[0].pos.x, 1.0);
        assert_eq!(ps[0].attrs["scale"], "2");
        let out = render_document(&doc, &ps, &[]).unwrap();
        let ps2 = placements(
            &MapDocument {
                text: out,
                path: doc.path.clone(),
            },
            &HashMap::new(),
        );
        assert_eq!(ps2[0].attrs["id"], "unique & one");
        assert!(!ps2[1].attrs.contains_key("id"));
    }
    #[test]
    fn saves_transforms_deletion_duplicates_and_unknown_xml() {
        let doc = document();
        let mut ps = placements(&doc, &HashMap::new());
        ps[0].pos.x = 99.0;
        ps[0].tag = "building".into();
        ps.push(ps[0].clone());
        let states = [
            ElementState::default(),
            ElementState {
                deleted: true,
                hidden: false,
            },
            ElementState::default(),
        ];
        let out = render_document(&doc, &ps, &states).unwrap();
        assert!(out.contains("<!-- preserve -->"));
        assert!(out.contains("<info name=\"A &amp; B\"/>"));
        assert!(out.contains("<vehicle model='411' id='car'/>"));
        assert!(out.contains("<data name='custom' value='untouched'/></building>"));
        assert!(!out.contains("__map"));
        assert_eq!(
            out.matches("<data name='custom' value='untouched'/>")
                .count(),
            2
        );
        let ps2 = placements(
            &MapDocument {
                text: out,
                path: doc.path.clone(),
            },
            &HashMap::new(),
        );
        assert_eq!(ps2.len(), 2);
        assert_eq!(ps2[0].pos.x, 99.0);
        assert_eq!(ps2[1].pos.x, 99.0);
        assert!(!ps2[1].attrs.contains_key("id"));
    }
    #[test]
    fn moves_objects_between_documents_without_resurrecting_sources() {
        let doc = document();
        let mut ps = placements(&doc, &HashMap::new());
        let second = MapDocument {
            path: "second.map".into(),
            text: "<map>\n</map>".into(),
        };
        ps[0].zone = map_zone(&second.path);
        assert_eq!(
            placements(
                &MapDocument {
                    text: render_document(&doc, &ps, &[]).unwrap(),
                    path: doc.path.clone()
                },
                &HashMap::new()
            )
            .len(),
            1
        );
        assert_eq!(
            placements(
                &MapDocument {
                    text: render_document(&second, &ps, &[]).unwrap(),
                    path: second.path.clone()
                },
                &HashMap::new()
            )
            .len(),
            1
        );
    }
    #[test]
    fn snapshots_preserve_map_membership_and_empty_maps() {
        let r = root("snapshot");
        let overlay = r.join("snapshot");
        fs::create_dir(&overlay).unwrap();
        let doc = document();
        fs::create_dir_all(r.join("maps")).unwrap();
        fs::write(r.join(&doc.path), &doc.text).unwrap();
        fs::write(
            r.join("meta.xml"),
            "<meta><map src='maps/main.map'/></meta>",
        )
        .unwrap();
        save_documents(&overlay, std::slice::from_ref(&doc), &[], &[]).unwrap();
        let loaded = load_documents(&r, &overlay);
        assert_eq!(loaded.len(), 1);
        assert!(placements(&loaded[0], &HashMap::new()).is_empty());
        assert!(loaded[0].text.contains("<vehicle"));
        fs::remove_dir_all(r).unwrap();
    }
    #[test]
    fn accepts_self_closing_empty_map_root() {
        let r = root("empty");
        fs::write(r.join("empty.map"), "<?xml version=\"1.0\"?><map />").unwrap();
        let doc = read_document(&r, "empty.map").unwrap();
        assert!(render_document(&doc, &[], &[]).unwrap().contains("</map>"));
        fs::remove_dir_all(r).unwrap();
    }
    #[test]
    fn rejects_unsafe_paths_and_invalid_documents() {
        for path in [
            "../x.map",
            "/x.map",
            "a/../x.map",
            "a\\x.map",
            "C:/x.map",
            "x.txt",
            "x.map\n",
        ] {
            assert!(!safe_map_path(path), "{path}");
        }
        assert!(nodes("<map><object></map>", "map").is_err());
        assert!(nodes("<other></other>", "map").is_err());
        #[cfg(unix)]
        {
            let r = root("symlink");
            let outside = root("outside");
            std::os::unix::fs::symlink(&outside, r.join("maps")).unwrap();
            assert!(new_document(&r, "maps/escape.map").is_err());
            fs::remove_dir_all(r).unwrap();
            fs::remove_dir_all(outside).unwrap();
        }
    }
}

// removeWorldObject is an editor map element, not a built-in world deletion.
// A resource script applies it during gameplay and restores it on shutdown.
fn save_world_removal_runtime(root: &Path) -> Result<(), String> {
    const SCRIPT: &str = "eagle_world_removals.lua";
    fs::write(
        checked_path_script(root, SCRIPT)?,
        r#"local removals = {}
addEventHandler("onResourceStart", resourceRoot, function()
    for _, element in ipairs(getElementsByType("removeWorldObject", resourceRoot)) do
        local model = tonumber(getElementData(element, "model"))
        local radius = tonumber(getElementData(element, "radius"))
        local x = tonumber(getElementData(element, "posX"))
        local y = tonumber(getElementData(element, "posY"))
        local z = tonumber(getElementData(element, "posZ"))
        local interior = tonumber(getElementData(element, "interior")) or -1
        if model and radius and x and y and z then
            local args = {model, radius, x, y, z, interior}
            removeWorldModel(unpack(args))
            table.insert(removals, args)
            local lod = tonumber(getElementData(element, "lodModel"))
            if lod and lod > 0 then
                local lodArgs = {lod, radius, x, y, z, interior}
                removeWorldModel(unpack(lodArgs))
                table.insert(removals, lodArgs)
            end
        end
    end
end)
addEventHandler("onResourceStop", resourceRoot, function()
    for _, args in ipairs(removals) do
        restoreWorldModel(unpack(args))
    end
end)
"#,
    )
    .map_err(|e| e.to_string())?;
    let path = root.join("meta.xml");
    let mut text = fs::read_to_string(&path).unwrap_or_else(|_| "<meta>\n</meta>\n".into());
    let (tags, close) = nodes(&text, "meta")?;
    let mut entries = String::new();
    for (tag, src, extra) in [
        ("map", "maps/world_edits.map", ""),
        ("script", SCRIPT, " type=\"server\""),
    ] {
        if !tags
            .iter()
            .any(|n| n.tag == tag && n.attrs.get("src").is_some_and(|s| s == src))
        {
            entries.push_str(&format!("    <{tag} src=\"{src}\"{extra}/>\n"));
        }
    }
    text.insert_str(close, &entries);
    fs::write(path, text).map_err(|e| e.to_string())
}
fn checked_path_script(root: &Path, file: &str) -> Result<PathBuf, String> {
    let path = root.join(file);
    if path.exists()
        && !path
            .canonicalize()
            .map_err(|e| e.to_string())?
            .starts_with(root.canonicalize().map_err(|e| e.to_string())?)
    {
        return Err("World removal script leaves the project".into());
    }
    Ok(path)
}

#[cfg(test)]
mod world_runtime_tests {
    use super::*;
    #[test]
    fn world_edits_round_trip_and_register_removal_runtime_without_losing_meta() {
        let root = std::env::temp_dir().join(format!(
            "eagle_world_runtime_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("meta.xml"),
            "<meta><info type='map' gamemodes='race'/><map src='other.map' dimension='7'/></meta>",
        )
        .unwrap();
        let doc = MapDocument { path: "maps/world_edits.map".into(), text: "<map><removeWorldObject model=\"1337\" lodModel=\"0\" radius=\"0.1\" interior=\"13\" posX=\"10\" posY=\"20\" posZ=\"30\"/><object model=\"1337\" posX=\"10\"/></map>".into() };
        let mut ps = placements(&doc, &HashMap::new());
        ps[0].pos.x = 99.0;
        save_documents(&root, std::slice::from_ref(&doc), &ps, &[]).unwrap();
        let saved = read_document(&root, &doc.path).unwrap();
        assert!(saved.text.contains("<removeWorldObject model=\"1337\""));
        assert_eq!(placements(&saved, &HashMap::new())[0].pos.x, 99.0);
        assert_eq!(placements(&saved, &HashMap::new()).len(), 1);
        let meta = fs::read_to_string(root.join("meta.xml")).unwrap();
        assert!(meta.contains("dimension='7'"));
        assert!(meta.contains("gamemodes='race'"));
        assert!(meta.contains("src=\"maps/world_edits.map\""));
        assert!(meta.contains("src=\"eagle_world_removals.lua\" type=\"server\""));
        let script = fs::read_to_string(root.join("eagle_world_removals.lua")).unwrap();
        assert!(script.contains("removeWorldModel(unpack(args))"));
        assert!(script.contains("restoreWorldModel(unpack(args))"));
        save_documents(
            &root,
            &[doc],
            &ps,
            &[ElementState {
                deleted: true,
                hidden: false,
            }],
        )
        .unwrap();
        let deleted = fs::read_to_string(root.join("maps/world_edits.map")).unwrap();
        assert!(deleted.contains("<removeWorldObject"));
        assert!(!deleted.contains("<object"));
        let meta = fs::read_to_string(root.join("meta.xml")).unwrap();
        assert_eq!(meta.matches("src=\"maps/world_edits.map\"").count(), 1);
        fs::remove_dir_all(root).unwrap();
    }
}
