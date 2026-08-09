use super::super::*;

#[derive(Clone)]
pub(crate) struct AssetBrowserEntry {
    pub(crate) id: String,
    pub(crate) dff: String,
    pub(crate) txd: String,
    pub(crate) zone: String,
    pub(crate) tag: String,
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
        PANEL_W + 12.0,
        screen_height() - STATUS_H - h - 8.0,
        (screen_width() - PANEL_W - RIGHT_PANEL_W - 24.0).max(280.0),
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

pub(crate) fn asset_browser_tag_filter_rect(app: &AppState, slot: usize) -> Rect {
    let rect = asset_browser_rect(app);
    Rect::new(
        rect.x + 556.0 + slot as f32 * 88.0,
        rect.y + 44.0,
        80.0,
        30.0,
    )
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
    ((grid.w + 10.0) / 190.0).floor().max(1.0) as usize
}

pub(crate) fn asset_browser_entries(app: &AppState) -> Vec<AssetBrowserEntry> {
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
                tag,
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

pub(crate) fn asset_mesh_thumbnail(
    mesh: &RenderMesh,
    accent: Color,
    texture_files: &HashMap<String, PathBuf>,
    txd_textures: &TxdTextureIndex,
) -> Image {
    let mut image = Image::gen_image_color(72, 72, Color::from_rgba(15, 18, 23, 255));
    let center = (mesh.bounds.min + mesh.bounds.max) * 0.5;
    let size = (mesh.bounds.max - mesh.bounds.min).length().max(1.0);
    let scale = 48.0 / size;
    for y in 0..72 {
        for x in 0..72 {
            if (x + y) % 2 == 0 {
                image.set_pixel(x, y, Color::from_rgba(18, 22, 28, 255));
            }
        }
    }
    let mut tris = Vec::<(f32, [(i32, i32); 3], Color)>::new();
    let mut sources = HashMap::<String, Option<ThumbnailSource>>::new();
    for part in &mesh.parts {
        let source_key = lower(part.texture_name.trim());
        if !source_key.is_empty() && !sources.contains_key(&source_key) {
            sources.insert(
                source_key.clone(),
                asset_decode_thumbnail_source(&source_key, texture_files, txd_textures),
            );
        }
        let source = sources.get(&source_key).and_then(|source| source.as_ref());
        for tri in part.cpu_vertices.chunks_exact(3).take(900) {
            let a = asset_project_vertex(tri[0].pos, center, scale);
            let b = asset_project_vertex(tri[1].pos, center, scale);
            let c = asset_project_vertex(tri[2].pos, center, scale);
            let normal = (to_mq(tri[1].pos) - to_mq(tri[0].pos))
                .cross(to_mq(tri[2].pos) - to_mq(tri[0].pos))
                .normalize_or_zero();
            let light = normal.dot(vec3(-0.35, -0.42, 0.84).normalize()).abs();
            let shade = (0.44 + light * 0.46).clamp(0.25, 0.95);
            tris.push((
                (a.2 + b.2 + c.2) / 3.0,
                [(a.0, a.1), (b.0, b.1), (c.0, c.1)],
                source
                    .and_then(|source| {
                        asset_sample_thumbnail_source(
                            source,
                            V2 {
                                u: (tri[0].uv.u + tri[1].uv.u + tri[2].uv.u) / 3.0,
                                v: (tri[0].uv.v + tri[1].uv.v + tri[2].uv.v) / 3.0,
                            },
                            shade,
                        )
                    })
                    .unwrap_or_else(|| {
                        Color::new(accent.r * shade, accent.g * shade, accent.b * shade, 1.0)
                    }),
            ));
        }
    }
    tris.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    for (_, pts, color) in tris {
        asset_thumbnail_triangle(&mut image, pts, color);
        asset_thumbnail_line(&mut image, pts[0].0, pts[0].1, pts[1].0, pts[1].1, color);
        asset_thumbnail_line(&mut image, pts[1].0, pts[1].1, pts[2].0, pts[2].1, color);
        asset_thumbnail_line(&mut image, pts[2].0, pts[2].1, pts[0].0, pts[0].1, color);
    }
    image
}

pub(crate) fn asset_browser_thumbnail(app: &mut AppState, entry: &AssetBrowserEntry) -> Texture2D {
    if let Some(texture) = app.asset_browser.thumbnails.get(&entry.id) {
        return texture.clone();
    }
    let mut hasher = DefaultHasher::new();
    entry.id.hash(&mut hasher);
    entry.zone.hash(&mut hasher);
    let hash = hasher.finish();
    let r = 52u8.saturating_add((hash & 0x5f) as u8);
    let g = 64u8.saturating_add(((hash >> 8) & 0x5f) as u8);
    let b = 74u8.saturating_add(((hash >> 16) & 0x5f) as u8);
    let accent = Color::from_rgba(r, g, b, 255);
    let mesh_key = lower(with_ext(&entry.dff, ".dff"));
    let image = app
        .meshes
        .get(&mesh_key)
        .map(|mesh| asset_mesh_thumbnail(mesh, accent, &app.texture_files, &app.txd_textures))
        .unwrap_or_else(|| Image::gen_image_color(72, 72, accent));
    let texture = Texture2D::from_image(&image);
    texture.set_filter(FilterMode::Nearest);
    app.asset_browser
        .thumbnails
        .insert(entry.id.clone(), texture.clone());
    texture
}

pub(crate) fn asset_place_position(app: &AppState) -> V3 {
    let (forward, _) = camera_vectors(&app.camera);
    let distance = if forward.z < -0.01 {
        ((0.0 - app.camera.pos.z) / forward.z).clamp(96.0, 1200.0)
    } else {
        360.0
    };
    let mut pos = app.camera.pos + forward * distance;
    pos.z = 0.0;
    from_mq(pos)
}

pub(crate) fn place_asset_from_browser(app: &mut AppState, entry: &AssetBrowserEntry) {
    let Some(def) = app.definitions.get(&entry.id).cloned() else {
        return;
    };
    let before = world_history_snapshot(app);
    let zone = app
        .placements
        .get(app.selected)
        .map(|placement| placement.zone.clone())
        .or_else(|| app.zones.first().cloned())
        .unwrap_or_else(|| {
            if entry.readonly {
                "SA".to_string()
            } else {
                entry.zone.clone()
            }
        });
    if entry.readonly {
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
    let pos = asset_place_position(app);
    let mut placement = Placement {
        id: entry.id.clone(),
        dff: entry.dff.clone(),
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
    rebuild_outliner_filter(app);
    rebuild_render_cells(app);
    commit_world_history(app, "Place Asset", before);
    app.status_message = format!("Placed {}", entry.id);
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
    let col = visible_idx % cols;
    let row = visible_idx / cols;
    Rect::new(
        grid.x + col as f32 * 190.0,
        grid.y + row as f32 * 92.0,
        180.0,
        82.0,
    )
}

pub(crate) fn update_asset_browser(app: &mut AppState, mouse: Vec2) -> bool {
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
            return true;
        }
        return rect.contains(mouse);
    }
    if !rect.contains(mouse) && !resize_rect.contains(mouse) {
        if app.asset_browser.search_active && is_mouse_button_pressed(MouseButton::Left) {
            app.asset_browser.search_active = false;
        }
        update_asset_browser_text_input(app);
        return false;
    }
    let entries = asset_browser_entries(app);
    let cols = asset_browser_columns(app);
    let visible_rows = ((asset_browser_grid_rect(app).h + 10.0) / 92.0)
        .floor()
        .max(1.0) as usize;
    let max_scroll = entries.len().div_ceil(cols).saturating_sub(visible_rows) as f32;
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
        for (slot, tag) in ["building", "object", "scenery"].iter().enumerate() {
            if asset_browser_tag_filter_rect(app, slot).contains(mouse) {
                match *tag {
                    "building" => {
                        app.asset_browser.show_buildings = !app.asset_browser.show_buildings
                    }
                    "object" => app.asset_browser.show_objects = !app.asset_browser.show_objects,
                    "scenery" => app.asset_browser.show_scenery = !app.asset_browser.show_scenery,
                    _ => {}
                }
                app.asset_browser.scroll = 0.0;
                return true;
            }
        }
        let start = app.asset_browser.scroll as usize * cols;
        let visible_slots = visible_rows * cols;
        for slot in 0..visible_slots {
            let idx = start + slot;
            let Some(entry) = entries.get(idx) else {
                break;
            };
            if asset_browser_card_rect(app, slot).contains(mouse) {
                place_asset_from_browser(app, entry);
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
    text_button(
        &app.ui_font,
        asset_browser_tag_filter_rect(app, 0),
        "Building",
        app.asset_browser.show_buildings,
    );
    text_button(
        &app.ui_font,
        asset_browser_tag_filter_rect(app, 1),
        "Object",
        app.asset_browser.show_objects,
    );
    text_button(
        &app.ui_font,
        asset_browser_tag_filter_rect(app, 2),
        "Scenery",
        app.asset_browser.show_scenery,
    );

    let grid = asset_browser_grid_rect(app);
    draw_rectangle(grid.x, grid.y, grid.w, grid.h, ui_canvas_bg());
    let cols = asset_browser_columns(app);
    let visible_rows = ((grid.h + 10.0) / 92.0).floor().max(1.0) as usize;
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
        draw_texture_ex(
            &thumb,
            card.x + 6.0,
            card.y + 6.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(70.0, 70.0)),
                ..Default::default()
            },
        );
        ui_text(
            &app.ui_font,
            &ellipsize_width(&entry.id, 16, card.w - 88.0),
            card.x + 84.0,
            card.y + 20.0,
            WHITE,
        );
        ui_text(
            &app.ui_font,
            &ellipsize_width(&entry.zone, 14, card.w - 88.0),
            card.x + 84.0,
            card.y + 39.0,
            if entry.readonly { ORANGE } else { ui_dim() },
        );
        ui_text_size(
            &app.ui_font,
            &ellipsize_width(&format!("{} / {}", entry.dff, entry.txd), 13, card.w - 88.0),
            card.x + 84.0,
            card.y + 56.0,
            13,
            ui_muted(),
        );
        ui_text_size(
            &app.ui_font,
            if entry.lod { "LOD" } else { &entry.tag },
            card.x + 84.0,
            card.y + 73.0,
            12,
            if entry.lod { ORANGE } else { ui_dim() },
        );
    }
}
