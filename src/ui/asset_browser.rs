use super::super::*;

fn imported_dff_stem(dff_path: &Path) -> Option<String> {
    let mut stem = dff_path.file_stem()?.to_str()?.trim();
    // Some picker/export combinations hand back a doubly-qualified name such
    // as model.dff.dff. Never let that leftover DFF suffix leak into the TXD
    // name or the definition as model.dff.txd.
    while stem.len() > 4 && stem[stem.len() - 4..].eq_ignore_ascii_case(".dff") {
        stem = stem[..stem.len() - 4].trim_end();
    }
    (!stem.is_empty()).then(|| stem.to_string())
}

pub(crate) fn automatic_texture_folder(dff_path: &Path) -> Option<PathBuf> {
    let parent = dff_path.parent()?;
    let stem = imported_dff_stem(dff_path)?;
    [parent.join("textures"), parent.join(stem)]
        .into_iter()
        .find(|path| path.is_dir())
}

fn imported_texture_image_paths(folder: &Path) -> Vec<PathBuf> {
    let mut paths = WalkDir::new(folder)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| {
                    ["png", "jpg", "jpeg", "bmp", "tga"]
                        .iter()
                        .any(|supported| ext.eq_ignore_ascii_case(supported))
                })
        })
        .collect::<Vec<_>>();
    paths.sort_by(|a, b| lower(&a.to_string_lossy()).cmp(&lower(&b.to_string_lossy())));
    paths
}

fn same_named_txd(folder: &Path, stem: &str) -> Option<PathBuf> {
    let expected = format!("{stem}.txd");
    WalkDir::new(folder)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .find(|entry| {
            entry.file_type().is_file()
                && entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| name.eq_ignore_ascii_case(&expected))
        })
        .map(|entry| entry.into_path())
}

fn referenced_texture_names(raw: &RawMesh) -> BTreeSet<String> {
    raw.material_textures
        .iter()
        .map(|name| lower(name.trim()))
        .filter(|name| !name.is_empty())
        .collect()
}

fn persist_imported_asset_payloads(
    root: &Path,
    assets: &[(String, Vec<u8>)],
) -> Result<(), String> {
    // Imports are resource assets, not unsaved light-mapper edits. Write the
    // payloads directly to the level's DFF/TXD archives and remove any older
    // replacement overlay for the same names so it cannot keep shadowing the
    // imported bytes.
    upsert_imported_resource_assets(root, assets)?;
    let keys = assets
        .iter()
        .map(|(name, _)| lower(&normalize_legacy_light_mapper_asset_name(name)))
        .collect::<HashSet<_>>();
    remove_replacement_archive_entries(root, &keys)?;
    remove_replacement_archive_entries(&wip_root_path(root), &keys)?;
    Ok(())
}

pub(crate) fn open_import_asset_dialog(app: &mut AppState, dff_path: PathBuf) {
    let id = dff_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("new_asset")
        .to_string();
    let texture_dir = automatic_texture_folder(&dff_path)
        .or_else(|| dff_path.parent().map(Path::to_path_buf))
        .unwrap_or_default();
    app.import_asset_dialog = Some(ImportAssetDialog {
        dff_path,
        texture_dir,
        cursor: id.len(),
        id,
    });
    drain_text_input();
}

pub(crate) fn cancel_import_asset_dialog(app: &mut AppState) {
    app.import_asset_dialog = None;
    // The only picker launched by this modal is its texture-folder picker.
    // Dropping the receiver invalidates a late result after the modal closes.
    app.dff_picker_rx = None;
    app.status_message = "Import asset cancelled".to_string();
}

pub(crate) fn import_new_asset(
    app: &mut AppState,
    dff_path: &Path,
    texture_dir: Option<&Path>,
    id: &str,
    position: Option<V3>,
) -> Result<(), String> {
    let id = id.trim();
    if id.is_empty() {
        return Err("Asset ID cannot be empty".to_string());
    }
    if app.definitions.contains_key(id) {
        return Err(format!("Asset ID '{id}' already exists"));
    }
    let stem =
        imported_dff_stem(dff_path).ok_or_else(|| "The DFF has no usable file name".to_string())?;
    let dff_name = format!("{stem}.dff");
    if dff_name.as_bytes().len() >= 24 {
        return Err("DFF names must be shorter than 24 bytes for IMG archives".to_string());
    }
    let bytes = fs::read(dff_path)
        .map_err(|err| format!("Could not read {}: {err}", dff_path.display()))?;
    let raw = parse_dff_mesh(&bytes);
    if raw.vertices.is_empty() || raw.triangles.is_empty() {
        return Err("The DFF did not contain readable mesh geometry".to_string());
    }

    let required_textures = referenced_texture_names(&raw);
    let mut txd_name = None;
    let mut txd_bytes = None;
    if !required_textures.is_empty() {
        let folder = texture_dir
            .filter(|folder| folder.is_dir())
            .ok_or_else(|| {
                format!(
                    "The DFF references textures, but no readable texture folder was found ({})",
                    required_textures
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;
        let name = format!("{stem}.txd");
        if let Some(path) = same_named_txd(folder, &stem) {
            txd_bytes = Some(
                fs::read(&path)
                    .map_err(|err| format!("Could not read {}: {err}", path.display()))?,
            );
        } else {
            let images = imported_texture_image_paths(folder);
            let available = images
                .iter()
                .map(|path| lower(&texture_name_from_path(path)))
                .collect::<BTreeSet<_>>();
            let missing = required_textures
                .difference(&available)
                .cloned()
                .collect::<Vec<_>>();
            if !missing.is_empty() {
                return Err(format!(
                    "{} is missing texture image(s): {}",
                    folder.display(),
                    missing.join(", ")
                ));
            }
            let (_, built) = build_txd_from_paths(&name, folder, &images)?;
            txd_bytes = Some(built);
        }
        txd_name = Some(name);
    }

    if let (Some(name), Some(bytes)) = (txd_name.as_ref(), txd_bytes.as_ref()) {
        let mut candidate_index = TxdTextureIndex::new();
        index_one_txd(
            bytes,
            0,
            bytes.len(),
            Path::new("import-preview.txd"),
            name,
            &mut candidate_index,
        );
        let missing = required_textures
            .iter()
            .filter(|texture| !candidate_index.contains_key(*texture))
            .cloned()
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(format!(
                "The selected texture source does not contain: {}",
                missing.join(", ")
            ));
        }
    }

    // A newly imported level asset belongs to the resource itself, even when
    // the editor was opened from a detached WIP scene. Keep the original file
    // payload byte-exact in the resource replacement archive; WIP remains for
    // editable derivatives, lighting, and other pending work.
    let mut imported_assets = vec![(dff_name.clone(), bytes)];
    if let (Some(name), Some(bytes)) = (txd_name.as_ref(), txd_bytes) {
        imported_assets.push((name.clone(), bytes));
    }
    persist_imported_asset_payloads(&app.root, &imported_assets)?;
    for (name, _) in &imported_assets {
        let key = lower(&normalize_legacy_light_mapper_asset_name(name));
        app.pending_replacement_assets.remove(&key);
        app.pending_txd_writes.remove(&key);
    }
    app.asset_browser.dff_sources = None;
    if let Some(name) = txd_name.as_ref() {
        let destination = imported_resource_asset_path(&app.root, name)?;
        remove_txd_from_texture_index(app, name);
        if destination
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("txd"))
        {
            index_standalone_txd_file(&destination, &mut app.txd_textures);
        } else {
            // Import routing normally puts a new dictionary in txd.img, not in
            // eagleReplacement.img. Refresh its real archive so the generated
            // textures are available to the live preview without a reload.
            refresh_txd_archive_index(&destination, &mut app.txd_textures);
        }
        invalidate_cached_txd_textures(app, name, None);
    }

    let texture_files = collect_texture_files(&app.root);
    let ambient_lift = scene_ambient_lift_from_timecyc(&app.timecyc);
    let mesh = compile_render_mesh(
        raw,
        txd_name.as_deref(),
        None,
        None,
        &texture_files,
        &app.txd_textures,
        &mut app.textures,
        &mut app.textured_parts,
        app.options.textures,
        ambient_lift,
    )
    .ok_or_else(|| "The DFF could not be compiled into a render mesh".to_string())?;
    replace_render_mesh(
        &mut app.meshes,
        mesh_key_from_dff_txd(&dff_name, txd_name.as_deref()),
        mesh,
    );

    let zone = app
        .placement_destination
        .clone()
        .or_else(|| app.placements.get(app.selected).map(|p| p.zone.clone()))
        .or_else(|| app.zones.first().cloned())
        .unwrap_or_else(|| "default".to_string());
    let zone = if crate::resource::mta_maps::map_path(&zone).is_some() {
        "default".to_string()
    } else {
        zone
    };
    let mut attrs = BTreeMap::new();
    attrs.insert("id".to_string(), id.to_string());
    attrs.insert("dff".to_string(), stem.to_string());
    attrs.insert("zone".to_string(), zone.clone());
    if txd_name.is_some() {
        attrs.insert("txd".to_string(), stem.to_string());
    }
    app.definitions.insert(
        id.to_string(),
        Definition {
            id: id.to_string(),
            zone: zone.clone(),
            attrs,
        },
    );
    if !app.zones.contains(&zone) {
        app.zones.push(zone.clone());
    }
    app.asset_browser.entries_cache_fingerprint = None;
    invalidate_validation_cache(app);

    let staged_placement = if let Some(position) = position {
        let entry = AssetBrowserEntry {
            id: id.to_string(),
            dff: stem.to_string(),
            txd: txd_name
                .as_ref()
                .map(|_| stem.to_string())
                .unwrap_or_default(),
            zone,
            category: asset_category(id, &stem),
            readonly: false,
            lod: false,
        };
        let previous_destination = app.placement_destination.replace(entry.zone.clone());
        place_asset_from_browser_at(app, &entry, position);
        app.placement_destination = previous_destination;
        app.status_message = format!("Imported and placed {id}");
        app.placements.get(app.selected).cloned()
    } else {
        app.status_message = format!("Imported {id}; drag it from the Asset Browser to place it");
        None
    };
    // Register the definition (and a dropped placement) in the level's real
    // zone files immediately. This makes the asset part of the level asset
    // list instead of leaving its definition isolated in a WIP snapshot.
    let definition = app
        .definitions
        .get(id)
        .ok_or_else(|| "Imported definition disappeared before level staging".to_string())?;
    stage_imported_level_asset(&app.root, definition, staged_placement.as_ref()).map_err(
        |err| {
            format!("The DFF/TXD were staged, but the level asset list could not be updated: {err}")
        },
    )?;
    Ok(())
}

#[derive(Clone)]
pub(crate) struct AssetBrowserEntry {
    pub(crate) id: String,
    pub(crate) dff: String,
    pub(crate) txd: String,
    pub(crate) zone: String,
    pub(crate) category: String,
    pub(crate) readonly: bool,
    pub(crate) lod: bool,
}

pub(crate) fn asset_browser_height(app: &AppState) -> f32 {
    if app.asset_browser.expanded {
        app.asset_browser.height.clamp(
            170.0,
            (screen_height() - TOP_H - STATUS_H - 24.0).max(170.0),
        )
    } else {
        34.0
    }
}

pub(crate) fn asset_browser_rect(app: &AppState) -> Rect {
    let h = asset_browser_height(app);
    Rect::new(
        left_panel_width() + 12.0,
        screen_height() - STATUS_H - h - 8.0,
        (screen_width() - left_panel_width() - right_panel_width() - 24.0).max(280.0),
        h,
    )
}

pub(crate) fn asset_browser_search_rect(app: &AppState) -> Rect {
    let rect = asset_browser_rect(app);
    Rect::new(rect.x + 12.0, rect.y + 44.0, 220.0, 30.0)
}

pub(crate) fn asset_browser_show_sa_rect(app: &AppState) -> Rect {
    let rect = asset_browser_rect(app);
    Rect::new(rect.x + 244.0, rect.y + 44.0, 92.0, 30.0)
}

pub(crate) fn asset_browser_sort_rect(app: &AppState) -> Rect {
    let rect = asset_browser_rect(app);
    Rect::new(rect.x + 344.0, rect.y + 44.0, 104.0, 30.0)
}

pub(crate) fn asset_browser_lod_filter_rect(app: &AppState) -> Rect {
    let rect = asset_browser_rect(app);
    Rect::new(rect.x + 456.0, rect.y + 44.0, 92.0, 30.0)
}

pub(crate) fn asset_browser_category_rect(app: &AppState) -> Rect {
    let rect = asset_browser_rect(app);
    Rect::new(rect.x + 556.0, rect.y + 44.0, 154.0, 30.0)
}

const ASSET_GRID_SCALE_MIN: f32 = 0.7;
const ASSET_GRID_SCALE_MAX: f32 = 1.6;
const ASSET_GRID_SCALE_STEP: f32 = 0.1;

pub(crate) fn asset_browser_grid_smaller_rect(app: &AppState) -> Rect {
    let rect = asset_browser_rect(app);
    Rect::new(rect.x + rect.w - 150.0, rect.y + 44.0, 34.0, 30.0)
}

pub(crate) fn asset_browser_grid_size_rect(app: &AppState) -> Rect {
    let rect = asset_browser_rect(app);
    Rect::new(rect.x + rect.w - 112.0, rect.y + 44.0, 62.0, 30.0)
}

pub(crate) fn asset_browser_grid_larger_rect(app: &AppState) -> Rect {
    let rect = asset_browser_rect(app);
    Rect::new(rect.x + rect.w - 46.0, rect.y + 44.0, 34.0, 30.0)
}

fn asset_browser_grid_scale(app: &AppState) -> f32 {
    app.asset_browser
        .grid_scale
        .clamp(ASSET_GRID_SCALE_MIN, ASSET_GRID_SCALE_MAX)
}

fn asset_browser_cell_size(app: &AppState) -> Vec2 {
    let scale = asset_browser_grid_scale(app);
    vec2(190.0 * scale, 92.0 * scale)
}

const ASSET_CATEGORIES: [&str; 10] = [
    "Alpha / Non-collidable",
    "Beach and Sea",
    "Buildings",
    "Industrial",
    "Interior Objects",
    "Land Masses",
    "Miscellaneous",
    "Nature",
    "Structures",
    "Transportation",
];

pub(crate) fn asset_browser_category_menu_rect(app: &AppState) -> Rect {
    let button = asset_browser_category_rect(app);
    let height = ASSET_CATEGORIES.len() as f32 * 26.0 + 12.0;
    let y = (button.y + button.h + 4.0)
        .min(screen_height() - STATUS_H - height - 8.0)
        .max(TOP_H + 8.0);
    Rect::new(button.x, y, 238.0, height)
}

/// Mirrors the broad groups used by MTA:SA's stock object browser. GTA's IDE
/// data has no category field, so model names are classified by their stable
/// naming conventions and unmatched/custom assets remain under Miscellaneous.
pub(crate) fn asset_category(id: &str, dff: &str) -> String {
    let name = lower(&format!("{id} {dff}"));
    let category = if ["alpha", "shad", "shadow", "coll", "glass"]
        .iter()
        .any(|word| name.contains(word))
    {
        ASSET_CATEGORIES[0]
    } else if [
        "beach", "sea", "dock", "pier", "jetty", "buoy", "ship", "boat",
    ]
    .iter()
    .any(|word| name.contains(word))
    {
        ASSET_CATEGORIES[1]
    } else if [
        "road", "street", "bridge", "rail", "train", "airport", "runway",
    ]
    .iter()
    .any(|word| name.contains(word))
    {
        ASSET_CATEGORIES[9]
    } else if ["tree", "plant", "grass", "bush", "rock", "forest", "veg"]
        .iter()
        .any(|word| name.contains(word))
    {
        ASSET_CATEGORIES[7]
    } else if ["land", "ground", "hill", "mount", "dirt", "desert"]
        .iter()
        .any(|word| name.contains(word))
    {
        ASSET_CATEGORIES[5]
    } else if ["factory", "indust", "warehouse", "crane", "refin", "quarry"]
        .iter()
        .any(|word| name.contains(word))
    {
        ASSET_CATEGORIES[3]
    } else if [
        "int_", "interior", "room", "kitchen", "bed", "office", "shop",
    ]
    .iter()
    .any(|word| name.contains(word))
    {
        ASSET_CATEGORIES[4]
    } else if [
        "fence", "wall", "barrier", "gate", "tower", "stairs", "ramp",
    ]
    .iter()
    .any(|word| name.contains(word))
    {
        ASSET_CATEGORIES[8]
    } else if [
        "building", "house", "hotel", "block", "skyscr", "garage", "stadium",
    ]
    .iter()
    .any(|word| name.contains(word))
    {
        ASSET_CATEGORIES[2]
    } else {
        ASSET_CATEGORIES[6]
    };
    category.to_string()
}

pub(crate) fn asset_browser_resize_rect(app: &AppState) -> Rect {
    let rect = asset_browser_rect(app);
    Rect::new(rect.x, rect.y - 10.0, rect.w, 20.0)
}

pub(crate) fn asset_browser_grid_rect(app: &AppState) -> Rect {
    let rect = asset_browser_rect(app);
    Rect::new(rect.x + 12.0, rect.y + 86.0, rect.w - 24.0, rect.h - 98.0)
}

pub(crate) fn asset_browser_columns(app: &AppState) -> usize {
    let grid = asset_browser_grid_rect(app);
    let cell = asset_browser_cell_size(app);
    ((grid.w + 10.0 * asset_browser_grid_scale(app)) / cell.x)
        .floor()
        .max(1.0) as usize
}

pub(crate) fn asset_browser_visible_rows(app: &AppState) -> usize {
    let grid = asset_browser_grid_rect(app);
    let cell = asset_browser_cell_size(app);
    ((grid.h + 10.0 * asset_browser_grid_scale(app)) / cell.y)
        .floor()
        .max(1.0) as usize
}

pub(crate) fn asset_browser_entries_fingerprint(app: &AppState) -> u64 {
    let mut hasher = DefaultHasher::new();
    app.definitions.len().hash(&mut hasher);
    app.readonly_definition_ids.len().hash(&mut hasher);
    app.placements.len().hash(&mut hasher);
    app.asset_browser.search.hash(&mut hasher);
    app.asset_browser.show_sa_assets.hash(&mut hasher);
    app.asset_browser.sort_by_zone.hash(&mut hasher);
    app.asset_browser.hide_lods.hash(&mut hasher);
    app.asset_browser.show_buildings.hash(&mut hasher);
    app.asset_browser.show_objects.hash(&mut hasher);
    app.asset_browser.show_scenery.hash(&mut hasher);
    let mut hidden = app
        .asset_browser
        .hidden_categories
        .iter()
        .collect::<Vec<_>>();
    hidden.sort();
    for category in hidden {
        category.hash(&mut hasher);
    }
    hasher.finish()
}

pub(crate) fn build_asset_browser_entries(app: &AppState) -> Vec<AssetBrowserEntry> {
    let needle = lower(app.asset_browser.search.trim());
    let mut tag_by_id = HashMap::<String, String>::new();
    for placement in &app.placements {
        tag_by_id
            .entry(placement.id.clone())
            .or_insert_with(|| placement.tag.clone());
    }
    let mut entries = app
        .definitions
        .values()
        .filter_map(|def| {
            let readonly = app.readonly_definition_ids.contains(&def.id);
            if readonly && !app.asset_browser.show_sa_assets {
                return None;
            }
            let dff = def
                .attrs
                .get("dff")
                .cloned()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| def.id.clone());
            let txd = def.attrs.get("txd").cloned().unwrap_or_default();
            let zone = def
                .attrs
                .get("zone")
                .cloned()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| def.zone.clone());
            let tag = tag_by_id
                .get(&def.id)
                .cloned()
                .unwrap_or_else(|| "object".to_string());
            let lod = is_lod_name(&def.id, &dff);
            let category = asset_category(&def.id, &dff);
            if app.asset_browser.hidden_categories.contains(&category) {
                return None;
            }
            if app.asset_browser.hide_lods && lod {
                return None;
            }
            if (tag == "building" && !app.asset_browser.show_buildings)
                || (tag == "object" && !app.asset_browser.show_objects)
                || (tag == "scenery" && !app.asset_browser.show_scenery)
            {
                return None;
            }
            if !needle.is_empty() {
                let hay = format!(
                    "{} {} {} {} {}",
                    lower(&def.id),
                    lower(&dff),
                    lower(&txd),
                    lower(&zone),
                    lower(&tag)
                );
                if !hay.contains(&needle) {
                    return None;
                }
            }
            Some(AssetBrowserEntry {
                id: def.id.clone(),
                dff,
                txd,
                zone,
                category,
                readonly,
                lod,
            })
        })
        .collect::<Vec<_>>();
    if app.asset_browser.sort_by_zone {
        entries.sort_by(|a, b| {
            lower(&a.zone)
                .cmp(&lower(&b.zone))
                .then_with(|| lower(&a.id).cmp(&lower(&b.id)))
        });
    } else {
        entries.sort_by(|a, b| lower(&a.id).cmp(&lower(&b.id)));
    }
    entries
}

pub(crate) fn asset_browser_entries(app: &mut AppState) -> std::sync::Arc<Vec<AssetBrowserEntry>> {
    let fingerprint = asset_browser_entries_fingerprint(app);
    if app.asset_browser.entries_cache_fingerprint != Some(fingerprint) {
        app.asset_browser.entries_cache = std::sync::Arc::new(build_asset_browser_entries(app));
        app.asset_browser.entries_cache_fingerprint = Some(fingerprint);
    }
    app.asset_browser.entries_cache.clone()
}

pub(crate) fn asset_thumbnail_line(
    image: &mut Image,
    mut x0: i32,
    mut y0: i32,
    x1: i32,
    y1: i32,
    color: Color,
) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        if (0..72).contains(&x0) && (0..72).contains(&y0) {
            image.set_pixel(x0 as u32, y0 as u32, color);
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

pub(crate) fn asset_thumb_edge(a: (i32, i32), b: (i32, i32), p: (i32, i32)) -> i32 {
    (p.0 - a.0) * (b.1 - a.1) - (p.1 - a.1) * (b.0 - a.0)
}

pub(crate) fn asset_thumbnail_triangle(image: &mut Image, pts: [(i32, i32); 3], color: Color) {
    let min_x = pts.iter().map(|p| p.0).min().unwrap_or(0).clamp(0, 71);
    let max_x = pts.iter().map(|p| p.0).max().unwrap_or(0).clamp(0, 71);
    let min_y = pts.iter().map(|p| p.1).min().unwrap_or(0).clamp(0, 71);
    let max_y = pts.iter().map(|p| p.1).max().unwrap_or(0).clamp(0, 71);
    let area = asset_thumb_edge(pts[0], pts[1], pts[2]);
    if area == 0 {
        return;
    }
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let p = (x, y);
            let w0 = asset_thumb_edge(pts[1], pts[2], p);
            let w1 = asset_thumb_edge(pts[2], pts[0], p);
            let w2 = asset_thumb_edge(pts[0], pts[1], p);
            let inside = if area > 0 {
                w0 >= 0 && w1 >= 0 && w2 >= 0
            } else {
                w0 <= 0 && w1 <= 0 && w2 <= 0
            };
            if inside {
                image.set_pixel(x as u32, y as u32, color);
            }
        }
    }
}

pub(crate) fn asset_project_vertex(v: V3, center: Vec3, scale: f32) -> (i32, i32, f32) {
    let p = to_mq(v) - center;
    let yaw = 0.78f32;
    let pitch = -0.58f32;
    let cy = yaw.cos();
    let sy = yaw.sin();
    let cp = pitch.cos();
    let sp = pitch.sin();
    let x = p.x * cy - p.y * sy;
    let y0 = p.x * sy + p.y * cy;
    let y = y0 * cp - p.z * sp;
    let z = y0 * sp + p.z * cp;
    (
        (36.0 + x * scale).round() as i32,
        (38.0 - y * scale).round() as i32,
        z,
    )
}

#[derive(Clone)]
pub(crate) struct ThumbnailSource {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) rgba: Vec<u8>,
}

pub(crate) fn asset_decode_thumbnail_source(
    name: &str,
    texture_files: &HashMap<String, PathBuf>,
    txd_textures: &TxdTextureIndex,
) -> Option<ThumbnailSource> {
    let key = lower(name.trim());
    if key.is_empty() {
        return None;
    }
    if let Some(path) = texture_files.get(&key) {
        let bytes = fs::read(path).ok()?;
        let rgba = image::load_from_memory(&bytes).ok()?.to_rgba8();
        let (width, height) = rgba.dimensions();
        return Some(ThumbnailSource {
            width,
            height,
            rgba: rgba.into_raw(),
        });
    }
    if let Some(tex) = txd_textures.get(&key).and_then(|entries| entries.first()) {
        let (width, height, rgba) = decode_txd_texture(tex)?;
        return Some(ThumbnailSource {
            width,
            height,
            rgba,
        });
    }
    None
}

pub(crate) fn asset_sample_thumbnail_source(
    source: &ThumbnailSource,
    uv: V2,
    shade: f32,
) -> Option<Color> {
    if source.width == 0 || source.height == 0 || source.rgba.len() < 4 {
        return None;
    }
    let mut u = uv.u.fract();
    let mut v = uv.v.fract();
    if u < 0.0 {
        u += 1.0;
    }
    if v < 0.0 {
        v += 1.0;
    }
    let x = (u * (source.width.saturating_sub(1)) as f32).round() as u32;
    let y = ((1.0 - v) * (source.height.saturating_sub(1)) as f32).round() as u32;
    let idx = ((y * source.width + x) as usize) * 4;
    if idx + 3 >= source.rgba.len() || source.rgba[idx + 3] < 16 {
        return None;
    }
    Some(Color::new(
        (source.rgba[idx] as f32 / 255.0 * shade).clamp(0.0, 1.0),
        (source.rgba[idx + 1] as f32 / 255.0 * shade).clamp(0.0, 1.0),
        (source.rgba[idx + 2] as f32 / 255.0 * shade).clamp(0.0, 1.0),
        1.0,
    ))
}

pub(crate) fn asset_raw_mesh_thumbnail(
    raw: &RawMesh,
    accent: Color,
    texture_files: &HashMap<String, PathBuf>,
    txd_textures: &TxdTextureIndex,
) -> Image {
    let mut image = Image::gen_image_color(72, 72, Color::from_rgba(15, 18, 23, 255));
    if raw.vertices.is_empty() {
        return image;
    }
    let bounds = bounds_from_vertices(&raw.vertices);
    let center = (bounds.min + bounds.max) * 0.5;
    let scale = 48.0 / (bounds.max - bounds.min).length().max(1.0);
    for y in 0..72 {
        for x in 0..72 {
            if (x + y) % 2 == 0 {
                image.set_pixel(x, y, Color::from_rgba(18, 22, 28, 255));
            }
        }
    }
    let mut sources = HashMap::<String, Option<ThumbnailSource>>::new();
    let mut triangles = Vec::<(f32, [(i32, i32); 3], Color)>::new();
    for tri in raw.triangles.iter().take(1800) {
        let indices = [tri.a as usize, tri.b as usize, tri.c as usize];
        let (Some(a), Some(b), Some(c)) = (
            raw.vertices.get(indices[0]).copied(),
            raw.vertices.get(indices[1]).copied(),
            raw.vertices.get(indices[2]).copied(),
        ) else {
            continue;
        };
        let pa = asset_project_vertex(a, center, scale);
        let pb = asset_project_vertex(b, center, scale);
        let pc = asset_project_vertex(c, center, scale);
        let normal = (to_mq(b) - to_mq(a))
            .cross(to_mq(c) - to_mq(a))
            .normalize_or_zero();
        let light = normal.dot(vec3(-0.35, -0.42, 0.84).normalize()).abs();
        let shade = (0.44 + light * 0.46).clamp(0.25, 0.95);
        let material = tri.material as usize;
        let source_key = lower(
            raw.material_textures
                .get(material)
                .map(String::as_str)
                .unwrap_or_default(),
        );
        if !source_key.is_empty() && !sources.contains_key(&source_key) {
            sources.insert(
                source_key.clone(),
                asset_decode_thumbnail_source(&source_key, texture_files, txd_textures),
            );
        }
        let mut uv = V2::default();
        for index in indices {
            if let Some(vertex_uv) = raw.uvs.get(index) {
                uv.u += vertex_uv.u / 3.0;
                uv.v += vertex_uv.v / 3.0;
            }
        }
        let fallback = raw
            .materials
            .get(material)
            .map(|material| {
                Color::new(
                    material.color.x * shade,
                    material.color.y * shade,
                    material.color.z * shade,
                    1.0,
                )
            })
            .unwrap_or_else(|| {
                Color::new(accent.r * shade, accent.g * shade, accent.b * shade, 1.0)
            });
        let color = sources
            .get(&source_key)
            .and_then(|source| source.as_ref())
            .and_then(|source| asset_sample_thumbnail_source(source, uv, shade))
            .unwrap_or(fallback);
        triangles.push((
            (pa.2 + pb.2 + pc.2) / 3.0,
            [(pa.0, pa.1), (pb.0, pb.1), (pc.0, pc.1)],
            color,
        ));
    }
    triangles.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    for (_, points, color) in triangles {
        asset_thumbnail_triangle(&mut image, points, color);
        for (a, b) in [(0usize, 1usize), (1, 2), (2, 0)] {
            asset_thumbnail_line(
                &mut image,
                points[a].0,
                points[a].1,
                points[b].0,
                points[b].1,
                color,
            );
        }
    }
    image
}

pub(crate) fn asset_browser_thumbnail_placeholder(app: &mut AppState) -> Texture2D {
    if let Some(texture) = app.asset_browser.thumbnail_placeholder.as_ref() {
        return texture.clone();
    }
    let image = Image::gen_image_color(72, 72, Color::from_rgba(24, 28, 35, 255));
    let texture = Texture2D::from_image(&image);
    texture.set_filter(FilterMode::Nearest);
    app.asset_browser.thumbnail_placeholder = Some(texture.clone());
    texture
}

pub(crate) fn asset_browser_dff_source(app: &mut AppState, dff: &str) -> Option<ImgEntry> {
    let source_key = asset_key(dff, ".dff");
    if app.asset_browser.dff_sources.is_none() {
        let mut sources = HashMap::new();
        // Later sources override earlier ones: project WIP > project > GTA:SA.
        for path in gta_sa_img_files(&app.gta_sa_dir) {
            for source in parse_img(&path) {
                sources.insert(lower(&source.name), source);
            }
        }
        let mut project_imgs = Vec::new();
        collect_img_files_from_dir(&app.root.join("imgs"), &mut project_imgs);
        for path in project_imgs {
            for source in parse_img(&path) {
                sources.insert(lower(&source.name), source);
            }
        }
        let mut wip_imgs = Vec::new();
        collect_img_files_from_dir(&wip_root_path(&app.root).join("imgs"), &mut wip_imgs);
        for path in wip_imgs {
            for source in parse_img(&path) {
                sources.insert(lower(&source.name), source);
            }
        }
        app.asset_browser.dff_sources = Some(sources);
    }
    app.asset_browser
        .dff_sources
        .as_ref()
        .and_then(|sources| sources.get(&source_key))
        .cloned()
        .or_else(|| find_dff_entry_for_app(app, dff))
}

pub(crate) fn asset_browser_thumbnail(app: &mut AppState, entry: &AssetBrowserEntry) -> Texture2D {
    if let Some(texture) = app.asset_browser.thumbnails.get(&entry.id) {
        return texture.clone();
    }
    if !app.asset_browser.thumbnail_generation_available {
        return asset_browser_thumbnail_placeholder(app);
    }
    app.asset_browser.thumbnail_generation_available = false;
    let mut hasher = DefaultHasher::new();
    entry.id.hash(&mut hasher);
    entry.zone.hash(&mut hasher);
    let hash = hasher.finish();
    let r = 52u8.saturating_add((hash & 0x5f) as u8);
    let g = 64u8.saturating_add(((hash >> 8) & 0x5f) as u8);
    let b = 74u8.saturating_add(((hash >> 16) & 0x5f) as u8);
    let accent = Color::from_rgba(r, g, b, 255);
    let raw = asset_browser_dff_source(app, &entry.dff)
        .map(|source| parse_dff_mesh(&read_img_entry(&source)));
    let image = raw
        .as_ref()
        .map(|raw| asset_raw_mesh_thumbnail(raw, accent, &app.texture_files, &app.txd_textures))
        .unwrap_or_else(|| Image::gen_image_color(72, 72, accent));
    let texture = Texture2D::from_image(&image);
    texture.set_filter(FilterMode::Nearest);
    app.asset_browser
        .thumbnails
        .insert(entry.id.clone(), texture.clone());
    texture
}

fn asset_browser_mesh_identity(
    entry: &AssetBrowserEntry,
    definitions: &HashMap<String, Definition>,
) -> (String, Option<String>) {
    // Browser entries can outlive definition edits. Use the same identity as
    // placement_mesh_key, rather than uploading under a stale browser key.
    if let Some(definition) = definitions.get(&entry.id) {
        (
            asset_key_opt(definition.attrs.get("dff"), &definition.id, ".dff"),
            definition_txd_name_from_attrs(definition).map(ToOwned::to_owned),
        )
    } else {
        (
            asset_key(&entry.dff, ".dff"),
            (!entry.txd.trim().is_empty()).then(|| entry.txd.trim().to_string()),
        )
    }
}

pub(crate) fn ensure_asset_browser_mesh_loaded(
    app: &mut AppState,
    entry: &AssetBrowserEntry,
) -> bool {
    let (dff, txd) = asset_browser_mesh_identity(entry, &app.definitions);
    let txd_scope = txd.as_deref();
    let mesh_key = mesh_key_from_dff_txd(&dff, txd_scope);
    if app
        .meshes
        .get(&mesh_key)
        .is_some_and(|mesh| !mesh.parts.is_empty())
    {
        return true;
    }
    let source = asset_browser_dff_source(app, &dff);
    let Some(source) = source else {
        app.status_message = format!("Could not resolve DFF {dff}");
        return false;
    };
    let raw = parse_dff_mesh(&read_img_entry(&source));
    let ambient_lift = scene_ambient_lift_from_timecyc(&app.timecyc);
    let Some(mesh) = compile_render_mesh(
        raw,
        txd_scope,
        None,
        None,
        &app.texture_files,
        &app.txd_textures,
        &mut app.textures,
        &mut app.textured_parts,
        app.options.textures,
        ambient_lift,
    ) else {
        app.status_message = format!("DFF {dff} has no renderable geometry");
        return false;
    };
    if mesh.parts.is_empty() {
        app.status_message = format!("DFF {dff} could not create any render parts");
        return false;
    }
    replace_render_mesh(&mut app.meshes, mesh_key, mesh);
    true
}

pub(crate) fn asset_place_position(app: &AppState) -> V3 {
    let (forward, _) = camera_vectors(&app.camera);
    asset_surface_position(app, app.camera.pos, forward)
}

fn asset_surface_position(app: &AppState, origin: Vec3, direction: Vec3) -> V3 {
    from_mq(asset_surface_point(
        origin,
        direction,
        |origin, direction, range| {
            trace_scene_geometry(app, origin, direction, range).map(|(_, point)| point)
        },
    ))
}

fn asset_surface_point(
    origin: Vec3,
    direction: Vec3,
    mut trace: impl FnMut(Vec3, Vec3, f32) -> Option<Vec3>,
) -> Vec3 {
    const TRACE_DISTANCE: f32 = 16000.0;
    if let Some(point) = trace(origin, direction, TRACE_DISTANCE) {
        return point;
    }

    // Aim at the editor ground plane when it is in front of the camera.
    // Looking level or up instead uses a nearby point ahead, then traces down
    // so roofs and elevated terrain still take precedence over the grid.
    let ground_distance = if direction.z.abs() > 0.00001 {
        -origin.z / direction.z
    } else {
        -1.0
    };
    let distance = if ground_distance > 0.0 && ground_distance <= TRACE_DISTANCE {
        ground_distance
    } else {
        360.0
    };
    let ahead = origin + direction * distance;
    let down_origin = vec3(ahead.x, ahead.y, origin.z.max(ahead.z).max(0.0) + 1.0);
    trace(down_origin, -Vec3::Z, TRACE_DISTANCE).unwrap_or_else(|| vec3(ahead.x, ahead.y, 0.0))
}

pub(crate) fn place_asset_from_browser(app: &mut AppState, entry: &AssetBrowserEntry) {
    let pos = asset_place_position(app);
    place_asset_from_browser_at(app, entry, pos);
}

pub(crate) fn place_asset_from_browser_at(app: &mut AppState, entry: &AssetBrowserEntry, pos: V3) {
    if !ensure_asset_browser_mesh_loaded(app, entry) {
        return;
    }
    let Some(def) = app.definitions.get(&entry.id).cloned() else {
        return;
    };
    let before = local_world_history_snapshot(app, [], [entry.id.clone()]);
    let zone = app
        .placement_destination
        .clone()
        .or_else(|| app.placements.get(app.selected).map(|p| p.zone.clone()))
        .or_else(|| app.zones.first().cloned())
        .unwrap_or_else(|| {
            if entry.readonly {
                "SA".to_string()
            } else {
                entry.zone.clone()
            }
        });
    if crate::resource::mta_maps::map_path(&zone).is_some() && entry.id.parse::<u32>().is_err() {
        app.status_message = "Custom models require an Eagle zone destination".into();
        return;
    }
    if entry.readonly && crate::resource::mta_maps::map_path(&zone).is_none() {
        let mut local_def = def.clone();
        local_def.zone = zone.clone();
        local_def.attrs.insert("zone".to_string(), zone.clone());
        local_def.attrs.remove("source");
        app.definitions.insert(entry.id.clone(), local_def);
        app.readonly_definition_ids.remove(&entry.id);
    }
    if !app.zones.iter().any(|existing| existing == &zone) {
        app.zones.push(zone.clone());
    }
    let mut placement = Placement {
        id: entry.id.clone(),
        dff: asset_key_opt(def.attrs.get("dff"), &def.id, ".dff"),
        zone,
        tag: "object".to_string(),
        attrs: BTreeMap::new(),
        pos,
        rot: V3::default(),
    };
    sync_placement_attrs(&mut placement);
    app.placements.push(placement);
    app.element_states.push(ElementState::default());
    app.outliner_labels.push(None);
    let idx = app.placements.len() - 1;
    app.selected = idx;
    app.selected_elements.clear();
    app.selected_elements.insert(idx);
    app.selected_element_order.clear();
    app.selected_element_order.push(idx);
    commit_local_world_history(app, "Place Asset", before);
    app.status_message = format!("Placed {}", entry.id);
}

pub(crate) fn asset_browser_drag_position(
    app: &AppState,
    viewport: Rect,
    mouse: Vec2,
) -> Option<V3> {
    if !viewport.contains(mouse) || asset_browser_rect(app).contains(mouse) {
        return None;
    }
    let (origin, direction) = viewport_ray(app, viewport, mouse)?;
    Some(asset_surface_position(app, origin, direction))
}

pub(crate) fn update_asset_browser_drag(app: &mut AppState, mouse: Vec2) -> bool {
    let Some((entry_id, start_mouse, was_dragging)) = app
        .asset_browser
        .drag
        .as_ref()
        .map(|drag| (drag.entry_id.clone(), drag.start_mouse, drag.dragging))
    else {
        return false;
    };
    let viewport = editor_viewport_rect();
    if is_mouse_button_down(MouseButton::Left) {
        let dragging = was_dragging || mouse.distance(start_mouse) >= 6.0;
        if dragging && !was_dragging {
            let Some(entry) = asset_browser_entries(app)
                .iter()
                .find(|entry| entry.id == entry_id)
                .cloned()
            else {
                app.asset_browser.drag = None;
                return true;
            };
            if !ensure_asset_browser_mesh_loaded(app, &entry) {
                app.asset_browser.drag = None;
                return true;
            }
            app.status_message =
                format!("Dragging {} — release in the viewport to place", entry.id);
        }
        let position = dragging
            .then(|| asset_browser_drag_position(app, viewport, mouse))
            .flatten();
        if let Some(drag) = app.asset_browser.drag.as_mut() {
            drag.dragging = dragging;
            drag.position = position;
        }
        return true;
    }
    if is_mouse_button_released(MouseButton::Left) {
        let Some(drag) = app.asset_browser.drag.take() else {
            return true;
        };
        let Some(entry) = asset_browser_entries(app)
            .iter()
            .find(|entry| entry.id == drag.entry_id)
            .cloned()
        else {
            return true;
        };
        if drag.dragging {
            if let Some(position) = asset_browser_drag_position(app, viewport, mouse) {
                place_asset_from_browser_at(app, &entry, position);
            } else {
                app.status_message = format!("Cancelled placement of {}", entry.id);
            }
        } else {
            place_asset_from_browser(app, &entry);
        }
        return true;
    }
    app.asset_browser.drag = None;
    true
}

pub(crate) fn preview_asset_from_browser(app: &mut AppState, entry: &AssetBrowserEntry) {
    if editing_dirty(app) {
        app.status_message =
            "Save or discard the current DFF/IMG changes before opening an asset preview"
                .to_string();
        return;
    }
    let Some(source) = asset_browser_dff_source(app, &entry.dff) else {
        app.status_message = format!("Could not resolve DFF {}", entry.dff);
        return;
    };
    open_img_entry_in_editing(app, source);
    // Opening may be deferred behind the editor's discard-changes prompt.
    // Do not relabel the still-active asset while that confirmation is shown.
    if app.confirm_dialog.is_some() {
        return;
    }
    if entry.readonly {
        if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut()
            && asset_key(&dff.name, ".dff") == asset_key(&entry.dff, ".dff")
        {
            dff.read_only = true;
            dff.dirty = false;
            app.status_message = format!("Previewing GTA:SA model {} (read-only)", entry.id);
        }
    } else if matches!(
        app.editing.asset.as_ref(),
        Some(EditingAsset::Dff(dff))
            if asset_key(&dff.name, ".dff") == asset_key(&entry.dff, ".dff")
    ) {
        app.status_message = format!("Opened {} in the DFF viewer", entry.id);
    }
}

pub(crate) fn set_asset_browser_cursor_from_mouse(app: &mut AppState, mouse_x: f32) {
    let rect = asset_browser_search_rect(app);
    let text_x = rect.x + 10.0;
    let mut cursor = 0usize;
    let mut best = f32::MAX;
    for idx in app
        .asset_browser
        .search
        .char_indices()
        .map(|(idx, _)| idx)
        .chain(std::iter::once(app.asset_browser.search.len()))
    {
        let prefix = &app.asset_browser.search[..idx];
        let x = text_x + ui_text_width(prefix, 16);
        let dist = (mouse_x - x).abs();
        if dist < best {
            best = dist;
            cursor = idx;
        }
    }
    app.asset_browser.cursor = cursor;
    app.asset_browser.selection_anchor = None;
}

pub(crate) fn update_asset_browser_text_input(app: &mut AppState) {
    if !app.asset_browser.search_active {
        return;
    }
    app.asset_browser.cursor =
        clamp_char_boundary(&app.asset_browser.search, app.asset_browser.cursor);
    if handle_text_clipboard_shortcuts(
        &mut app.asset_browser.search,
        &mut app.asset_browser.cursor,
        &mut app.asset_browser.selection_anchor,
    ) {
        drain_text_input();
        app.asset_browser.scroll = 0.0;
        return;
    }
    if is_key_pressed(KeyCode::Home) {
        app.asset_browser.cursor = 0;
        app.asset_browser.selection_anchor = None;
    }
    if is_key_pressed(KeyCode::End) {
        app.asset_browser.cursor = app.asset_browser.search.len();
        app.asset_browser.selection_anchor = None;
    }
    if is_key_pressed(KeyCode::Left) {
        app.asset_browser.cursor =
            prev_char_boundary(&app.asset_browser.search, app.asset_browser.cursor);
        app.asset_browser.selection_anchor = None;
    }
    if is_key_pressed(KeyCode::Right) {
        app.asset_browser.cursor =
            next_char_boundary(&app.asset_browser.search, app.asset_browser.cursor);
        app.asset_browser.selection_anchor = None;
    }
    let ctrl_down = ctrl_down();
    if is_key_pressed(KeyCode::Backspace)
        && !delete_text_selection(
            &mut app.asset_browser.search,
            &mut app.asset_browser.cursor,
            &mut app.asset_browser.selection_anchor,
        )
        && app.asset_browser.cursor > 0
    {
        let prev = if ctrl_down {
            prev_word_boundary(&app.asset_browser.search, app.asset_browser.cursor)
        } else {
            prev_char_boundary(&app.asset_browser.search, app.asset_browser.cursor)
        };
        app.asset_browser
            .search
            .replace_range(prev..app.asset_browser.cursor, "");
        app.asset_browser.cursor = prev;
        app.asset_browser.scroll = 0.0;
    }
    if is_key_pressed(KeyCode::Delete)
        && !delete_text_selection(
            &mut app.asset_browser.search,
            &mut app.asset_browser.cursor,
            &mut app.asset_browser.selection_anchor,
        )
        && app.asset_browser.cursor < app.asset_browser.search.len()
    {
        let next = next_char_boundary(&app.asset_browser.search, app.asset_browser.cursor);
        app.asset_browser
            .search
            .replace_range(app.asset_browser.cursor..next, "");
        app.asset_browser.scroll = 0.0;
    }
    while let Some(ch) = get_char_pressed() {
        if handle_text_control_char(
            &mut app.asset_browser.search,
            &mut app.asset_browser.cursor,
            &mut app.asset_browser.selection_anchor,
            ch,
        ) {
            continue;
        }
        if !ch.is_control() {
            insert_text_at_cursor(
                &mut app.asset_browser.search,
                &mut app.asset_browser.cursor,
                &mut app.asset_browser.selection_anchor,
                &ch.to_string(),
            );
            app.asset_browser.scroll = 0.0;
        }
    }
}

pub(crate) fn asset_browser_card_rect(app: &AppState, visible_idx: usize) -> Rect {
    let grid = asset_browser_grid_rect(app);
    let cols = asset_browser_columns(app);
    let scale = asset_browser_grid_scale(app);
    let cell = asset_browser_cell_size(app);
    let col = visible_idx % cols;
    let row = visible_idx / cols;
    Rect::new(
        grid.x + col as f32 * cell.x,
        grid.y + row as f32 * cell.y,
        180.0 * scale,
        82.0 * scale,
    )
}

pub(crate) fn update_asset_browser(app: &mut AppState, mouse: Vec2) -> bool {
    if update_asset_browser_drag(app, mouse) {
        return true;
    }
    if app.asset_browser.resizing {
        if is_mouse_button_down(MouseButton::Left) {
            let max_h = (screen_height() - TOP_H - STATUS_H - 24.0).max(170.0);
            app.asset_browser.height =
                (screen_height() - STATUS_H - 8.0 - mouse.y).clamp(170.0, max_h);
            return true;
        }
        app.asset_browser.resizing = false;
        return true;
    }
    let rect = asset_browser_rect(app);
    let resize_rect = asset_browser_resize_rect(app);
    if !app.asset_browser.expanded {
        if rect.contains(mouse) && is_mouse_button_pressed(MouseButton::Left) {
            app.asset_browser.expanded = true;
            app.asset_browser.entries_cache_fingerprint = None;
            return true;
        }
        return rect.contains(mouse);
    }
    let category_menu_contains = app.asset_browser.category_dropdown_open
        && asset_browser_category_menu_rect(app).contains(mouse);
    if !rect.contains(mouse) && !resize_rect.contains(mouse) && !category_menu_contains {
        if app.asset_browser.category_dropdown_open && is_mouse_button_pressed(MouseButton::Left) {
            app.asset_browser.category_dropdown_open = false;
        }
        if app.asset_browser.search_active && is_mouse_button_pressed(MouseButton::Left) {
            app.asset_browser.search_active = false;
        }
        update_asset_browser_text_input(app);
        // Keyboard focus belongs exclusively to the search field even when
        // the pointer has moved back over the viewport.
        return app.asset_browser.search_active;
    }
    let entries = asset_browser_entries(app);
    let cols = asset_browser_columns(app);
    let visible_rows = asset_browser_visible_rows(app);
    let max_scroll = entries.len().div_ceil(cols).saturating_sub(visible_rows) as f32;
    if app.asset_browser.category_dropdown_open {
        let menu = asset_browser_category_menu_rect(app);
        if is_mouse_button_pressed(MouseButton::Left) {
            if asset_browser_category_rect(app).contains(mouse) {
                app.asset_browser.category_dropdown_open = false;
                return true;
            }
            if menu.contains(mouse) {
                let row = ((mouse.y - menu.y - 6.0) / 26.0).floor() as usize;
                if let Some(category) = ASSET_CATEGORIES.get(row) {
                    if !app.asset_browser.hidden_categories.remove(*category) {
                        app.asset_browser
                            .hidden_categories
                            .insert((*category).to_string());
                    }
                    app.asset_browser.scroll = 0.0;
                }
                return true;
            }
            app.asset_browser.category_dropdown_open = false;
        }
        if menu.contains(mouse) {
            return true;
        }
    }
    let (_, wheel) = safe_mouse_wheel();
    if wheel.abs() > 0.0 {
        app.asset_browser.scroll = (app.asset_browser.scroll - wheel).clamp(0.0, max_scroll);
        return true;
    }
    if is_mouse_button_pressed(MouseButton::Left) {
        if resize_rect.contains(mouse) {
            app.asset_browser.resizing = true;
            return true;
        }
        if Rect::new(rect.x, rect.y, rect.w, 34.0).contains(mouse) {
            app.asset_browser.expanded = false;
            app.asset_browser.search_active = false;
            return true;
        }
        if asset_browser_search_rect(app).contains(mouse) {
            app.asset_browser.search_active = true;
            set_asset_browser_cursor_from_mouse(app, mouse.x);
            return true;
        }
        app.asset_browser.search_active = false;
        if asset_browser_show_sa_rect(app).contains(mouse) {
            app.asset_browser.show_sa_assets = !app.asset_browser.show_sa_assets;
            app.asset_browser.scroll = 0.0;
            return true;
        }
        if asset_browser_sort_rect(app).contains(mouse) {
            app.asset_browser.sort_by_zone = !app.asset_browser.sort_by_zone;
            app.asset_browser.scroll = 0.0;
            return true;
        }
        if asset_browser_lod_filter_rect(app).contains(mouse) {
            app.asset_browser.hide_lods = !app.asset_browser.hide_lods;
            app.asset_browser.scroll = 0.0;
            return true;
        }
        if asset_browser_category_rect(app).contains(mouse) {
            app.asset_browser.category_dropdown_open = true;
            return true;
        }
        if asset_browser_grid_smaller_rect(app).contains(mouse) {
            app.asset_browser.grid_scale = (app.asset_browser.grid_scale - ASSET_GRID_SCALE_STEP)
                .clamp(ASSET_GRID_SCALE_MIN, ASSET_GRID_SCALE_MAX);
            app.asset_browser.scroll = 0.0;
            return true;
        }
        if asset_browser_grid_larger_rect(app).contains(mouse) {
            app.asset_browser.grid_scale = (app.asset_browser.grid_scale + ASSET_GRID_SCALE_STEP)
                .clamp(ASSET_GRID_SCALE_MIN, ASSET_GRID_SCALE_MAX);
            app.asset_browser.scroll = 0.0;
            return true;
        }
        let start = app.asset_browser.scroll as usize * cols;
        let visible_slots = visible_rows * cols;
        for slot in 0..visible_slots {
            let idx = start + slot;
            let Some(entry) = entries.get(idx) else {
                break;
            };
            if asset_browser_card_rect(app, slot).contains(mouse) {
                app.asset_browser.drag = Some(AssetBrowserDrag {
                    entry_id: entry.id.clone(),
                    start_mouse: mouse,
                    position: None,
                    dragging: false,
                });
                return true;
            }
        }
    }
    if is_mouse_button_pressed(MouseButton::Right) {
        let start = app.asset_browser.scroll as usize * cols;
        let visible_slots = visible_rows * cols;
        for slot in 0..visible_slots {
            let idx = start + slot;
            let Some(entry) = entries.get(idx) else {
                break;
            };
            if asset_browser_card_rect(app, slot).contains(mouse) {
                app.context_menu = Some(ContextMenu {
                    pos: mouse,
                    target: ContextMenuTarget::AssetBrowser {
                        entry_id: entry.id.clone(),
                    },
                });
                return true;
            }
        }
    }
    update_asset_browser_text_input(app);
    true
}

pub(crate) fn draw_asset_browser(app: &mut AppState) {
    let rect = asset_browser_rect(app);
    draw_rrect(
        rect.x + 1.0,
        rect.y + 3.0,
        rect.w,
        rect.h,
        11.0,
        ui_shadow(),
    );
    draw_rrect_bordered(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        11.0,
        1.0,
        ui_panel_bg(),
        ui_border(),
    );
    let entries = asset_browser_entries(app);
    let title = "Asset Browser";
    let title_x = rect.x + 12.0;
    draw_rrect(title_x, rect.y + 8.0, 3.0, 17.0, 1.5, ui_accent());
    ui_text_bold(title, title_x + 11.0, rect.y + 23.0, 16, WHITE);
    let count_label = format!("{} assets", entries.len());
    let count_w = ui_text_width(&count_label, 16);
    let count_x = rect.x + rect.w - count_w - 12.0;
    let hint_x = title_x + ui_text_width(title, 16) + 29.0;
    let hint_w = (count_x - hint_x - 14.0).max(1.0);
    let hint = if app.asset_browser.expanded {
        "click header to collapse"
    } else {
        "click to expand"
    };
    ui_text(
        &app.ui_font,
        &ellipsize_width(hint, 16, hint_w),
        hint_x,
        rect.y + 23.0,
        ui_muted(),
    );
    ui_text(&app.ui_font, &count_label, count_x, rect.y + 23.0, ui_dim());
    if !app.asset_browser.expanded {
        return;
    }
    // Parsing a DFF and rasterizing its shaded preview is deliberately
    // amortized so opening the browser cannot turn one frame into a bulk load.
    app.asset_browser.thumbnail_generation_available = true;
    draw_rrect(
        rect.x + 18.0,
        rect.y - 6.0,
        rect.w - 36.0,
        8.0,
        4.0,
        if app.asset_browser.resizing {
            ui_accent()
        } else {
            Color::new(0.20, 0.21, 0.23, 1.0)
        },
    );
    draw_rrect(
        rect.x + rect.w * 0.5 - 32.0,
        rect.y - 3.5,
        64.0,
        3.0,
        1.5,
        Color::new(0.46, 0.48, 0.52, 0.9),
    );

    let search = asset_browser_search_rect(app);
    draw_rrect_bordered(
        search.x,
        search.y,
        search.w,
        search.h,
        7.0,
        1.0,
        if app.asset_browser.search_active {
            Color::new(0.055, 0.090, 0.128, 1.0)
        } else {
            ui_input_bg()
        },
        if app.asset_browser.search_active {
            ui_accent()
        } else {
            ui_border()
        },
    );
    let text_x = search.x + 10.0;
    if app.asset_browser.search.is_empty() && !app.asset_browser.search_active {
        ui_text(
            &app.ui_font,
            "Search assets",
            text_x,
            search.y + 20.0,
            ui_muted(),
        );
    } else {
        let visible = ellipsize_width(&app.asset_browser.search, 16, search.w - 20.0);
        ui_text(&app.ui_font, &visible, text_x, search.y + 20.0, WHITE);
        if app.asset_browser.search_active && (get_time() * 2.0) as i32 % 2 == 0 {
            let cursor = clamp_char_boundary(&app.asset_browser.search, app.asset_browser.cursor);
            let prefix = ellipsize_width(&app.asset_browser.search[..cursor], 16, search.w - 20.0);
            let caret_x = (text_x + ui_text_width(&prefix, 16)).min(search.x + search.w - 8.0);
            draw_line(
                caret_x,
                search.y + 7.0,
                caret_x,
                search.y + search.h - 7.0,
                1.0,
                WHITE,
            );
        }
    }
    text_button(
        &app.ui_font,
        asset_browser_show_sa_rect(app),
        "SA Assets",
        app.asset_browser.show_sa_assets,
    );
    text_button(
        &app.ui_font,
        asset_browser_sort_rect(app),
        if app.asset_browser.sort_by_zone {
            "Zone Sort"
        } else {
            "Name Sort"
        },
        app.asset_browser.sort_by_zone,
    );
    text_button(
        &app.ui_font,
        asset_browser_lod_filter_rect(app),
        "Hide LOD",
        app.asset_browser.hide_lods,
    );
    let visible_categories = ASSET_CATEGORIES.len() - app.asset_browser.hidden_categories.len();
    text_button(
        &app.ui_font,
        asset_browser_category_rect(app),
        &format!("Categories {visible_categories}/{}", ASSET_CATEGORIES.len()),
        app.asset_browser.category_dropdown_open || !app.asset_browser.hidden_categories.is_empty(),
    );
    text_button(
        &app.ui_font,
        asset_browser_grid_smaller_rect(app),
        "-",
        false,
    );
    text_button(
        &app.ui_font,
        asset_browser_grid_size_rect(app),
        &format!(
            "{}%",
            (asset_browser_grid_scale(app) * 100.0).round() as i32
        ),
        false,
    );
    text_button(
        &app.ui_font,
        asset_browser_grid_larger_rect(app),
        "+",
        false,
    );

    let grid = asset_browser_grid_rect(app);
    draw_rectangle(grid.x, grid.y, grid.w, grid.h, ui_canvas_bg());
    let cols = asset_browser_columns(app);
    let visible_rows = asset_browser_visible_rows(app);
    let scale = asset_browser_grid_scale(app);
    let start = app.asset_browser.scroll as usize * cols;
    let visible_slots = visible_rows * cols;
    for slot in 0..visible_slots {
        let idx = start + slot;
        let Some(entry) = entries.get(idx).cloned() else {
            break;
        };
        let card = asset_browser_card_rect(app, slot);
        let hovered = card.contains(mouse_position().into());
        draw_rrect(card.x + 1.0, card.y + 2.0, card.w, card.h, 9.0, ui_shadow());
        draw_rrect_bordered(
            card.x,
            card.y,
            card.w,
            card.h,
            9.0,
            1.0,
            if hovered {
                ui_surface_hover()
            } else {
                ui_surface()
            },
            if hovered { ui_accent() } else { ui_border() },
        );
        let thumb = asset_browser_thumbnail(app, &entry);
        let thumb_size = 70.0 * scale;
        draw_texture_ex(
            &thumb,
            card.x + 6.0 * scale,
            card.y + 6.0 * scale,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(thumb_size, thumb_size)),
                ..Default::default()
            },
        );
        let text_x = card.x + 84.0 * scale;
        let text_width = (card.w - 88.0 * scale).max(24.0);
        let id_size = (16.0 * scale).round().clamp(11.0, 22.0) as u16;
        let zone_size = (14.0 * scale).round().clamp(10.0, 20.0) as u16;
        let detail_size = (13.0 * scale).round().clamp(9.0, 19.0) as u16;
        let category_size = (12.0 * scale).round().clamp(9.0, 18.0) as u16;
        ui_text_size(
            &app.ui_font,
            &ellipsize_width(&entry.id, id_size, text_width),
            text_x,
            card.y + 20.0 * scale,
            id_size,
            WHITE,
        );
        ui_text_size(
            &app.ui_font,
            &ellipsize_width(&entry.zone, zone_size, text_width),
            text_x,
            card.y + 39.0 * scale,
            zone_size,
            if entry.readonly { ORANGE } else { ui_dim() },
        );
        ui_text_size(
            &app.ui_font,
            &ellipsize_width(
                &format!("{} / {}", entry.dff, entry.txd),
                detail_size,
                text_width,
            ),
            text_x,
            card.y + 56.0 * scale,
            detail_size,
            ui_muted(),
        );
        let category_label = if entry.lod { "LOD" } else { &entry.category };
        ui_text_size(
            &app.ui_font,
            &ellipsize_width(category_label, category_size, text_width),
            text_x,
            card.y + 73.0 * scale,
            category_size,
            if entry.lod { ORANGE } else { ui_dim() },
        );
    }
    if app.asset_browser.category_dropdown_open {
        let menu = asset_browser_category_menu_rect(app);
        draw_rrect_bordered(
            menu.x,
            menu.y,
            menu.w,
            menu.h,
            8.0,
            1.0,
            Color::new(0.052, 0.055, 0.061, 0.99),
            ui_border(),
        );
        let mouse: Vec2 = mouse_position().into();
        for (row, category) in ASSET_CATEGORIES.iter().enumerate() {
            let row_rect = Rect::new(
                menu.x + 6.0,
                menu.y + 6.0 + row as f32 * 26.0,
                menu.w - 12.0,
                26.0,
            );
            let visible = !app.asset_browser.hidden_categories.contains(*category);
            if row_rect.contains(mouse) {
                draw_rrect(
                    row_rect.x,
                    row_rect.y + 2.0,
                    row_rect.w,
                    22.0,
                    5.0,
                    ui_accent_soft(),
                );
            }
            ui_text(
                &app.ui_font,
                if visible { "✓" } else { "  " },
                row_rect.x + 7.0,
                row_rect.y + 18.0,
                if visible { ui_accent() } else { ui_muted() },
            );
            ui_text(
                &app.ui_font,
                category,
                row_rect.x + 28.0,
                row_rect.y + 18.0,
                if visible { WHITE } else { ui_muted() },
            );
        }
    }
}

#[cfg(test)]
mod import_asset_tests {
    use super::*;

    #[test]
    fn asset_placement_hits_the_surface_in_front_of_the_camera() {
        let origin = vec3(0.0, 0.0, 20.0);
        let wall = [
            vec3(-50.0, 40.0, 0.0),
            vec3(50.0, 40.0, 0.0),
            vec3(0.0, 40.0, 100.0),
        ];
        let point = asset_surface_point(origin, Vec3::Y, |start, dir, range| {
            ray_triangle(start, dir, wall[0], wall[1], wall[2])
                .filter(|t| *t <= range)
                .map(|t| start + dir * t)
        });
        assert!(point.distance(vec3(0.0, 40.0, 20.0)) < 0.001);
    }

    #[test]
    fn asset_placement_traces_down_to_elevated_ground_when_looking_level() {
        let deck = [
            vec3(-1000.0, -1000.0, 12.0),
            vec3(1000.0, -1000.0, 12.0),
            vec3(0.0, 1000.0, 12.0),
        ];
        let point = asset_surface_point(vec3(0.0, 0.0, 20.0), Vec3::Y, |start, dir, range| {
            ray_triangle(start, dir, deck[0], deck[1], deck[2])
                .filter(|t| *t <= range)
                .map(|t| start + dir * t)
        });
        assert!(point.distance(vec3(0.0, 360.0, 12.0)) < 0.001);
    }

    #[test]
    fn asset_placement_empty_scene_uses_ground_without_clamping_near_hits() {
        let origin = vec3(0.0, 0.0, 10.0);
        let down = vec3(0.0, 1.0, -1.0).normalize();
        let point = asset_surface_point(origin, down, |_, _, _| None);
        assert!(point.distance(vec3(0.0, 10.0, 0.0)) < 0.001);
        let up = vec3(0.0, 1.0, 1.0).normalize();
        let point = asset_surface_point(origin, up, |_, _, _| None);
        assert_eq!(point.z, 0.0);
        assert!(point.y > 0.0);
    }

    #[test]
    fn browser_first_load_uses_current_sa_definition_for_render_lookup() {
        let entry = AssetBrowserEntry {
            id: "100".into(),
            dff: "old_model".into(),
            txd: "old_textures".into(),
            zone: "GTA:SA".into(),
            category: "Other".into(),
            readonly: true,
            lod: false,
        };
        for attrs in [
            BTreeMap::from([
                ("dff".into(), " fence ".into()),
                ("txd".into(), " generic ".into()),
            ]),
            BTreeMap::from([("dff".into(), " ".into()), ("txd".into(), " ".into())]),
        ] {
            let definitions = HashMap::from([(
                entry.id.clone(),
                Definition {
                    id: entry.id.clone(),
                    zone: entry.zone.clone(),
                    attrs,
                },
            )]);
            let placement = Placement {
                id: entry.id.clone(),
                dff: entry.dff.clone(),
                zone: "default".into(),
                tag: "object".into(),
                attrs: BTreeMap::new(),
                pos: V3::default(),
                rot: V3::default(),
            };
            let (dff, txd) = asset_browser_mesh_identity(&entry, &definitions);
            assert_eq!(
                mesh_key_from_dff_txd(&dff, txd.as_deref()),
                placement_mesh_key(&placement, &definitions),
            );
            assert_ne!(dff, "old_model.dff");
        }
    }

    #[test]
    fn automatic_texture_folder_prefers_textures_then_dff_name() {
        let root = std::env::temp_dir().join(format!(
            "eagle_asset_import_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("tower")).unwrap();
        let dff = root.join("tower.dff");
        fs::write(&dff, []).unwrap();
        assert_eq!(automatic_texture_folder(&dff), Some(root.join("tower")));
        fs::create_dir_all(root.join("textures")).unwrap();
        assert_eq!(automatic_texture_folder(&dff), Some(root.join("textures")));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn imported_stem_never_carries_a_dff_extension_into_txd_name() {
        assert_eq!(
            imported_dff_stem(Path::new("tower.dff")),
            Some("tower".to_string())
        );
        assert_eq!(
            imported_dff_stem(Path::new("tower.dff.dff")),
            Some("tower".to_string())
        );
        assert_eq!(
            imported_dff_stem(Path::new("tower.DFF.dff")),
            Some("tower".to_string())
        );
    }

    #[test]
    fn imported_texture_search_accepts_common_image_formats() {
        let root = std::env::temp_dir().join(format!(
            "eagle_asset_texture_search_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("nested")).unwrap();
        for name in ["one.png", "two.JPG", "three.bmp", "four.tga"] {
            fs::write(root.join("nested").join(name), []).unwrap();
        }
        fs::write(root.join("nested").join("ignored.txt"), []).unwrap();
        assert_eq!(imported_texture_image_paths(&root).len(), 4);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn imported_dff_is_persisted_to_resource_and_evicted_from_wip() {
        let root = std::env::temp_dir().join(format!(
            "eagle_asset_resource_stage_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let old = include_bytes!("../../assets/player_vehicle/player_1.dff");
        let imported = include_bytes!("../../assets/player_vehicle/player_2.dff");
        let img_dir = root.join("imgs");
        fs::create_dir_all(&img_dir).unwrap();
        let level_dff_img = img_dir.join("dff_1.img");
        write_img_archive(
            &level_dff_img,
            &[("existing.dff".to_string(), old.to_vec())],
        )
        .unwrap();
        upsert_replacement_dff(&wip_root_path(&root), "imported.dff", old).unwrap();

        persist_imported_asset_payloads(&root, &[("imported.dff".to_string(), imported.to_vec())])
            .unwrap();

        let imported_entry = parse_img(&level_dff_img)
            .into_iter()
            .find(|entry| entry.name == "imported.dff")
            .unwrap();
        let mut imported_bytes = read_img_entry(&imported_entry);
        imported_bytes.truncate(imported.len());
        assert_eq!(imported_bytes.as_slice(), imported);
        assert!(replacement_archive_entries(&root).unwrap().is_empty());
        assert!(
            replacement_archive_entries(&wip_root_path(&root))
                .unwrap()
                .is_empty()
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn imported_txd_destination_resolves_to_the_generated_resource_archive() {
        let root = std::env::temp_dir().join(format!(
            "eagle_asset_txd_destination_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let txd_name = "logo.txd";
        let txd_bytes = include_bytes!("../../assets/player_vehicle/player_1.txd");

        persist_imported_asset_payloads(&root, &[(txd_name.to_string(), txd_bytes.to_vec())])
            .unwrap();

        let destination = imported_resource_asset_path(&root, txd_name).unwrap();
        assert_eq!(destination, root.join("imgs").join("txd.img"));
        assert!(
            parse_img(&destination)
                .iter()
                .any(|entry| entry.name.eq_ignore_ascii_case(txd_name))
        );
        let mut index = TxdTextureIndex::new();
        refresh_txd_archive_index(&destination, &mut index);
        assert!(
            index
                .values()
                .flatten()
                .any(|texture| texture.txd_name.eq_ignore_ascii_case(txd_name))
        );
        fs::remove_dir_all(root).unwrap();
    }
}
