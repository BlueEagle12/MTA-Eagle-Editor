use super::super::*;

pub(crate) fn compile_render_mesh(
    raw: RawMesh,
    txd_scope: Option<&str>,
    texture_overrides: Option<&HashMap<String, String>>,
    vehicle_material_preview: Option<VehicleMaterialPreview>,
    texture_files: &HashMap<String, PathBuf>,
    txd_textures: &TxdTextureIndex,
    textures: &mut HashMap<String, u32>,
    textured_parts: &mut usize,
    texture_enabled: bool,
    ambient_lift: V3,
) -> Option<RenderMesh> {
    if raw.vertices.is_empty() || raw.triangles.is_empty() {
        return None;
    }
    let bounds = bounds_from_vertices(&raw.vertices);
    let effects_2dfx = raw.effects_2dfx.clone();
    let material_animations = raw.material_animations.clone();
    let uv_animations = raw.uv_animations.clone();
    let material_count = raw.material_textures.len().max(1);
    let vert_count = raw.vertices.len();
    // Component (frame/atomic) names, indexed by RenderPart::component.
    let components: Vec<String> = if raw.components.is_empty() {
        vec![String::new()]
    } else {
        raw.components
            .iter()
            .enumerate()
            .map(|(idx, component)| {
                let name = component.name.trim();
                if name.is_empty() {
                    format!("geometry {idx}")
                } else {
                    name.to_string()
                }
            })
            .collect()
    };
    let component_pivots = if raw.components.is_empty() {
        vec![None]
    } else {
        raw.components
            .iter()
            .map(|component| raw_component_pivot(&raw, component))
            .collect()
    };
    // Triangle ranges are recorded in commit order and welding/smoothing never
    // reorders triangles, so a forward cursor maps each triangle to its
    // originating component.
    let component_ranges: Vec<(usize, usize)> = raw
        .components
        .iter()
        .map(|component| (component.tri_start, component.tri_end))
        .collect();
    let mut component_cursor = 0usize;
    // Split by (component, material) so parts can be toggled per component.
    let mut tris_by_key = BTreeMap::<(usize, usize), Vec<(usize, Tri)>>::new();
    for (tri_idx, tri) in raw.triangles.iter().enumerate() {
        while component_cursor + 1 < component_ranges.len()
            && tri_idx >= component_ranges[component_cursor].1
        {
            component_cursor += 1;
        }
        // Defensive: never let a malformed mesh (triangle indices past the end
        // of the vertex array) reach the indexing below and panic.
        if (tri.a as usize) >= vert_count
            || (tri.b as usize) >= vert_count
            || (tri.c as usize) >= vert_count
        {
            continue;
        }
        let component = if component_ranges.is_empty() {
            0
        } else {
            component_cursor
        };
        let mat = (tri.material as usize).min(material_count - 1);
        tris_by_key
            .entry((component, mat))
            .or_default()
            .push((tri_idx, *tri));
    }

    // GTA building DFFs commonly omit normals because the game primarily
    // uses prelight. The vertex baker still needs real surface orientation,
    // so reconstruct angle-weighted normals from the mesh instead of treating
    // every missing normal as world-up.
    let effective_normals = normalized_normals(&raw);
    let has_uvs = raw.uvs.len() == raw.vertices.len();
    let has_prelit_colors = raw.prelit_colors.len() == raw.vertices.len();
    let has_prelit_alphas = raw.prelit_alphas.len() == raw.vertices.len();
    let has_night_prelit_colors = raw.night_prelit_colors.len() == raw.vertices.len();
    let has_night_prelit_alphas = raw.night_prelit_alphas.len() == raw.vertices.len();
    let has_light_flags = raw.light_flags.len() == raw.vertices.len();
    let mut parts = Vec::new();
    for ((component, mat), tris) in tris_by_key {
        if tris.is_empty() {
            continue;
        }
        let tex_name = raw.material_textures.get(mat).cloned().unwrap_or_default();
        let material = raw.materials.get(mat).copied().unwrap_or(RawMaterial {
            color: neutral_vertex_color(),
            alpha: 1.0,
            ambient: 1.0,
            specular: 1.0,
            diffuse: 1.0,
        });
        let material_alpha = if material.alpha <= 0.05 {
            1.0
        } else {
            material.alpha
        };
        let vehicle_material_role = vehicle_material_role(material);
        let marker_color = vehicle_marker_preview_color(material, vehicle_material_preview);
        // Light materials glow at full brightness while the "Lights" toggle is on.
        let lights_on = vehicle_material_preview.is_some_and(|preview| preview.lights_on);
        let emissive = lights_on && vehicle_material_role == Some(VehicleMaterialRole::Light);
        let material_color_affects_part = marker_color.is_some() || tex_name.trim().is_empty();
        let material_color = if let Some(marker_color) = marker_color {
            marker_color
        } else if material_color_affects_part {
            material.color
        } else {
            neutral_vertex_color()
        };
        let override_scope = texture_overrides
            .and_then(|overrides| overrides.get(&lower(tex_name.trim())))
            .map(String::as_str);
        let texture_scope = override_scope.or(txd_scope);
        let texture_missing = texture_enabled
            && !tex_name.trim().is_empty()
            && !texture_source_exists(&tex_name, texture_scope, texture_files, txd_textures);
        let texture = load_texture_cached(
            &tex_name,
            texture_scope,
            texture_files,
            txd_textures,
            textures,
            texture_enabled,
        );
        let (texture_width, texture_height) = if texture == 0 {
            (0, 0)
        } else {
            let mut width = 0i32;
            let mut height = 0i32;
            unsafe {
                gl::BindTexture(gl::TEXTURE_2D, texture);
                gl::GetTexLevelParameteriv(gl::TEXTURE_2D, 0, gl::TEXTURE_WIDTH, &mut width);
                gl::GetTexLevelParameteriv(gl::TEXTURE_2D, 0, gl::TEXTURE_HEIGHT, &mut height);
                gl::BindTexture(gl::TEXTURE_2D, 0);
            }
            (
                width.clamp(0, u16::MAX as i32) as u16,
                height.clamp(0, u16::MAX as i32) as u16,
            )
        };
        let texture_fingerprint =
            texture_content_fingerprint(txd_textures, &tex_name, texture_scope);
        let texture_transparency = loaded_texture_transparency_mode(
            texture,
            &tex_name,
            texture_scope,
            texture_files,
            txd_textures,
            texture_enabled,
        );
        let has_vertex_alpha = (has_prelit_alphas
            && tris.iter().any(|(_, tri)| {
                [tri.a, tri.b, tri.c].into_iter().any(|index| {
                    raw.prelit_alphas
                        .get(index as usize)
                        .is_some_and(|alpha| *alpha < 0.98)
                })
            }))
            || (has_night_prelit_alphas
                && tris.iter().any(|(_, tri)| {
                    [tri.a, tri.b, tri.c].into_iter().any(|index| {
                        raw.night_prelit_alphas
                            .get(index as usize)
                            .is_some_and(|alpha| *alpha < 0.98)
                    })
                }));
        let transparency = if material_alpha < 0.98 || has_vertex_alpha {
            TransparencyMode::Blend
        } else {
            texture_transparency
        };
        if texture != 0 {
            *textured_parts += 1;
        }
        let mut cpu_vertices = Vec::with_capacity(tris.len() * 3);
        let use_lighting = !has_prelit_colors
            && !has_night_prelit_colors
            && tris.iter().any(|(_, tri)| {
                [tri.a, tri.b, tri.c].into_iter().any(|idx| {
                    has_light_flags && raw.light_flags.get(idx as usize).copied().unwrap_or(false)
                })
            });
        for (_, tri) in &tris {
            for idx in [tri.a, tri.b, tri.c] {
                let i = idx as usize;
                let p = raw.vertices[i];
                let uv = if has_uvs { raw.uvs[i] } else { V2::default() };
                let n = effective_normals[i];
                let prelit_color = if has_prelit_colors {
                    raw.prelit_colors[i]
                } else if has_night_prelit_colors {
                    raw.night_prelit_colors[i]
                } else {
                    neutral_vertex_color()
                };
                let night_prelit_color = if has_night_prelit_colors {
                    raw.night_prelit_colors[i]
                } else {
                    prelit_color
                };
                let day_alpha = if has_prelit_alphas {
                    raw.prelit_alphas[i]
                } else if has_night_prelit_alphas {
                    raw.night_prelit_alphas[i]
                } else {
                    1.0
                };
                let night_alpha = if has_night_prelit_alphas {
                    raw.night_prelit_alphas[i]
                } else {
                    day_alpha
                };
                let day_color = V3 {
                    x: prelit_color.x * material_color.x,
                    y: prelit_color.y * material_color.y,
                    z: prelit_color.z * material_color.z,
                };
                let night_color = V3 {
                    x: night_prelit_color.x * material_color.x,
                    y: night_prelit_color.y * material_color.y,
                    z: night_prelit_color.z * material_color.z,
                };
                cpu_vertices.push(Vertex {
                    pos: p,
                    normal: n,
                    uv,
                    color: day_color,
                    day_color,
                    night_color,
                    base_day_color: prelit_color,
                    base_night_color: night_prelit_color,
                    day_alpha,
                    night_alpha,
                    alpha: day_alpha,
                });
            }
        }
        let mut part = RenderPart {
            list: 0,
            vbo: 0,
            vertices: tris.len() * 3,
            material_index: mat,
            component,
            texture,
            texture_width,
            texture_height,
            texture_name: tex_name,
            texture_fingerprint,
            texture_missing,
            transparency,
            alpha: material_alpha,
            material_color: material.color,
            material_ambient: material.ambient,
            use_lighting,
            vehicle_material_role,
            emissive,
            cpu_vertices,
            face_indices: tris.iter().map(|(face_idx, _)| *face_idx).collect(),
        };
        rebuild_render_part_list_with_lift(&mut part, ambient_lift);
        if part.list != 0 || part.vbo != 0 {
            parts.push(part);
        }
    }
    Some(RenderMesh {
        parts,
        bounds,
        material_animations,
        uv_animations,
        effects_2dfx,
        components,
        component_pivots,
    })
}

fn raw_component_pivot(raw: &RawMesh, component: &RawMeshComponent) -> Option<Vec3> {
    let name = lower(component.name.trim());
    if name.is_empty() {
        return None;
    }
    raw.frames
        .iter()
        .find(|frame| lower(frame.name.trim()) == name)
        .or_else(|| {
            let mut candidates = Vec::new();
            if !name.ends_with("_dummy") {
                candidates.push(format!("{name}_dummy"));
            }
            for suffix in ["_ok", "_dam"] {
                if let Some(base) = name.strip_suffix(suffix) {
                    candidates.push(base.to_string());
                    candidates.push(format!("{base}_dummy"));
                }
            }
            raw.frames.iter().find(|frame| {
                let frame_name = lower(frame.name.trim());
                candidates.iter().any(|candidate| *candidate == frame_name)
            })
        })
        .map(|frame| to_mq(frame.pos))
}

fn color_marker_eq(color: V3, r: u8, g: u8, b: u8) -> bool {
    let cr = (color.x.clamp(0.0, 1.0) * 255.0).round() as u8;
    let cg = (color.y.clamp(0.0, 1.0) * 255.0).round() as u8;
    let cb = (color.z.clamp(0.0, 1.0) * 255.0).round() as u8;
    cr == r && cg == g && cb == b
}

/// Head/tail light materials are tagged in the DFF with one of a handful of
/// marker colours (warm yellows for head lights, cool blues / reds for tail and
/// indicator lights). GTA:SA uses these purely to identify which texture is
/// which light — they are not meant to tint the surface.
fn is_vehicle_light_marker(color: V3) -> bool {
    if color_marker_eq(color, 0, 255, 255) || color_marker_eq(color, 255, 0, 255) {
        return true;
    }
    let warm_left = [
        (255, 175, 0),
        (185, 255, 0),
        (184, 255, 0),
        (183, 255, 0),
        (182, 255, 0),
        (181, 255, 0),
        (255, 173, 0),
        (255, 174, 0),
    ];
    if warm_left
        .iter()
        .any(|(r, g, b)| color_marker_eq(color, *r, *g, *b))
    {
        return true;
    }
    let cool_right = [
        (0, 255, 200),
        (255, 60, 0),
        (255, 59, 0),
        (255, 58, 0),
        (255, 57, 0),
        (255, 56, 0),
        (0, 255, 198),
        (0, 255, 199),
    ];
    if cool_right
        .iter()
        .any(|(r, g, b)| color_marker_eq(color, *r, *g, *b))
    {
        return true;
    }
    if (1..=8).any(|slot| color_marker_eq(color, 255, 199, slot))
        || (1..=7).any(|slot| color_marker_eq(color, 255, 200, slot))
    {
        return true;
    }
    color_marker_eq(color, 0, 18, 255)
        || color_marker_eq(color, 0, 17, 255)
        || color_marker_eq(color, 0, 16, 255)
}

fn vehicle_material_role(material: RawMaterial) -> Option<VehicleMaterialRole> {
    let color = material.color;
    if color_marker_eq(color, 60, 255, 0) {
        Some(VehicleMaterialRole::BodyA)
    } else if color_marker_eq(color, 255, 0, 175) {
        Some(VehicleMaterialRole::BodyB)
    } else if is_vehicle_light_marker(color) {
        Some(VehicleMaterialRole::Light)
    } else {
        None
    }
}

fn vehicle_marker_preview_color(
    material: RawMaterial,
    preview: Option<VehicleMaterialPreview>,
) -> Option<V3> {
    match vehicle_material_role(material) {
        Some(VehicleMaterialRole::BodyA) => preview.map(|preview| preview.body_a),
        Some(VehicleMaterialRole::BodyB) => preview.map(|preview| preview.body_b),
        // Light markers render with a white material colour so the actual light
        // texture shows through naturally; brightness when the lights are
        // switched on is handled by the emissive draw path, not by tinting here.
        Some(VehicleMaterialRole::Light) => Some(V3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        }),
        None => None,
    }
}

pub(crate) fn load_simulation_assets(
    meshes: &mut HashMap<String, RenderMesh>,
    textures: &mut HashMap<String, u32>,
    textured_parts: &mut usize,
    texture_enabled: bool,
) {
    let asset_root = asset_path("player_vehicle");
    let texture_files = HashMap::new();
    let mut txd_textures = TxdTextureIndex::new();
    for txd in ["player_1.txd", "player_2.txd", "vehicle.txd"] {
        index_standalone_txd_file(&asset_root.join(txd), &mut txd_textures);
    }
    for (key, dff) in [
        (SIM_PLAYER_DFF, "player_1.dff"),
        (SIM_VEHICLE_DFF, "vehicle.dff"),
    ] {
        let path = asset_root.join(dff);
        let Ok(bytes) = fs::read(&path) else {
            continue;
        };
        let raw = parse_dff_mesh(&bytes);
        if let Some(mesh) = compile_render_mesh(
            raw,
            None,
            None,
            None,
            &texture_files,
            &txd_textures,
            textures,
            textured_parts,
            texture_enabled,
            V3::default(),
        ) {
            meshes.insert(lower(key), mesh);
        }
    }
}

const LOAD_MESH_BATCH: usize = 32;
const LOAD_COLLISION_BATCH: usize = 128;
const LOAD_WORKER_QUEUE_DEPTH: usize = 2;

type PreparedMeshAsset = (String, RawMesh, Option<String>);
type PreparedCollisionAsset = (String, CollisionMesh);

enum LoadAssetBatch {
    Indexed {
        txd_textures: TxdTextureIndex,
        mesh_total: usize,
        collision_total: usize,
    },
    Meshes(Vec<PreparedMeshAsset>),
    Collisions(Vec<PreparedCollisionAsset>),
    Finished,
}

fn read_load_worker_entry(entry: &ImgEntry, files: &mut HashMap<PathBuf, fs::File>) -> Vec<u8> {
    if !files.contains_key(&entry.img_path) {
        let Ok(file) = fs::File::open(&entry.img_path) else {
            return Vec::new();
        };
        files.insert(entry.img_path.clone(), file);
    }
    files
        .get_mut(&entry.img_path)
        .map(|file| read_img_entry_from(file, entry))
        .unwrap_or_default()
}

fn prepare_batch_parallel<T, R, F>(inputs: &[T], prepare: F) -> Vec<R>
where
    T: Sync,
    R: Send,
    F: Fn(&T) -> R + Sync,
{
    if inputs.is_empty() {
        return Vec::new();
    }
    let available = thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1);
    // Keep one logical CPU available for the render/input thread while asset
    // preparation runs ahead of GPU installation.
    let workers = available.saturating_sub(1).max(1).min(inputs.len());
    if workers == 1 {
        return inputs.iter().map(prepare).collect();
    }

    let chunk_size = inputs.len().div_ceil(workers);
    let (tx, rx) = mpsc::channel::<(usize, Vec<R>)>();
    thread::scope(|scope| {
        for (chunk_index, chunk) in inputs.chunks(chunk_size).enumerate() {
            let tx = tx.clone();
            let prepare = &prepare;
            scope.spawn(move || {
                let prepared = chunk.iter().map(prepare).collect();
                let _ = tx.send((chunk_index, prepared));
            });
        }
    });
    drop(tx);

    // Scoped workers may finish out of order; restore archive/definition order
    // before handing the batch to the main thread.
    let mut chunks = rx.into_iter().collect::<BTreeMap<_, _>>();
    let mut output = Vec::with_capacity(inputs.len());
    for chunk in chunks.values_mut() {
        output.append(chunk);
    }
    output
}

fn run_load_asset_worker(
    root: PathBuf,
    source: LoadSceneSource,
    gta_sa_dir: PathBuf,
    placements: Vec<Placement>,
    defs: HashMap<String, Definition>,
    load_meshes: bool,
    load_textures: bool,
    batch_tx: mpsc::SyncSender<LoadAssetBatch>,
    progress_tx: mpsc::Sender<String>,
) {
    let mut img_files = collect_scene_img_files(&root, source);
    img_files.extend(gta_sa_img_files(&gta_sa_dir));
    let overlay_root = match source {
        LoadSceneSource::Saved => wip_root_path(&root),
        LoadSceneSource::Autosave => autosave_root_path(&root),
    };

    let mut dff_map = HashMap::<String, ImgEntry>::new();
    let mut col_map = HashMap::<String, ImgEntry>::new();
    let mut txd_textures = TxdTextureIndex::new();
    let archive_total = img_files.len();

    for (index, path) in img_files.into_iter().enumerate() {
        let archive_entries = parse_img(&path);
        if load_textures {
            if path.starts_with(&overlay_root) {
                remove_img_txd_dictionaries_from_index(&mut txd_textures, &archive_entries);
            }
            index_txd_entries(&path, &archive_entries, &mut txd_textures);
        }
        for entry in archive_entries {
            let entry_name = lower(&entry.name);
            if entry_name.ends_with(".dff") {
                if path.starts_with(&root) || !dff_map.contains_key(&entry_name) {
                    dff_map.insert(entry_name, entry);
                }
            } else if entry_name.ends_with(".col")
                && (path.starts_with(&root) || !col_map.contains_key(&entry_name))
            {
                col_map.insert(entry_name, entry);
            }
        }
        let _ = progress_tx.send(format!("Reading archives ({}/{archive_total})", index + 1));
    }

    if load_textures {
        let standalone_txds = collect_scene_txd_files(&root, source);
        let gta_standalone_txds = [
            gta_sa_dir.join("models").join("particle.txd"),
            gta_sa_dir.join("models").join("effectsPC.txd"),
            gta_sa_dir
                .join("models")
                .join("generic")
                .join("vehicle.txd"),
        ];
        let standalone_total = standalone_txds.len() + gta_standalone_txds.len();
        for (index, path) in standalone_txds.into_iter().enumerate() {
            if path.starts_with(&overlay_root)
                && let Some(name) = path.file_name().and_then(|name| name.to_str())
            {
                remove_txd_dictionary_from_index(&mut txd_textures, name);
            }
            index_standalone_txd_file(&path, &mut txd_textures);
            let _ = progress_tx.send(format!(
                "Indexing texture dictionaries ({}/{standalone_total})",
                index + 1
            ));
        }
        for (index, path) in gta_standalone_txds.into_iter().enumerate() {
            if path.is_file() {
                index_standalone_txd_file(&path, &mut txd_textures);
            }
            let _ = progress_tx.send(format!(
                "Indexing texture dictionaries ({}/{standalone_total})",
                standalone_total.saturating_sub(1) + index
            ));
        }
    }

    let dff_queue = if load_meshes {
        build_referenced_dff_queue(&placements, &defs, &dff_map)
    } else {
        Vec::new()
    };
    let col_queue: Vec<_> = col_map.into_iter().collect();
    let mesh_total = dff_queue.len();
    let collision_total = col_queue.len();
    if batch_tx
        .send(LoadAssetBatch::Indexed {
            txd_textures,
            mesh_total,
            collision_total,
        })
        .is_err()
    {
        return;
    }

    // Reuse one file handle per IMG for the whole job. Opening the same large
    // archive once per DFF/COL was a significant part of map load time.
    let mut files = HashMap::<PathBuf, fs::File>::new();
    for (batch_index, batch) in dff_queue.chunks(LOAD_MESH_BATCH).enumerate() {
        let mut input = Vec::with_capacity(batch.len());
        for (mesh_key, entry, txd_name) in batch {
            input.push((
                mesh_key.clone(),
                read_load_worker_entry(entry, &mut files),
                txd_name.clone(),
            ));
        }
        let prepared = prepare_batch_parallel(&input, |(mesh_key, bytes, txd_name)| {
            (mesh_key.clone(), parse_dff_mesh(bytes), txd_name.clone())
        });
        if batch_tx.send(LoadAssetBatch::Meshes(prepared)).is_err() {
            return;
        }
        let prepared_count = ((batch_index + 1) * LOAD_MESH_BATCH).min(mesh_total);
        let _ = progress_tx.send(format!(
            "Preparing mesh batches ({prepared_count}/{mesh_total})"
        ));
    }

    for (batch_index, batch) in col_queue.chunks(LOAD_COLLISION_BATCH).enumerate() {
        let mut input = Vec::with_capacity(batch.len());
        for (name, entry) in batch {
            input.push((
                name.clone(),
                entry.clone(),
                read_load_worker_entry(entry, &mut files),
            ));
        }
        let prepared = prepare_batch_parallel(&input, |(name, entry, bytes)| {
            parse_col_mesh(bytes, entry).map(|mesh| (name.clone(), mesh))
        })
        .into_iter()
        .flatten()
        .collect();
        if batch_tx.send(LoadAssetBatch::Collisions(prepared)).is_err() {
            return;
        }
        let prepared_count = ((batch_index + 1) * LOAD_COLLISION_BATCH).min(collision_total);
        let _ = progress_tx.send(format!(
            "Preparing collision batches ({prepared_count}/{collision_total})"
        ));
    }

    let _ = batch_tx.send(LoadAssetBatch::Finished);
}

// Phases of the incremental resource load. Filesystem reads and CPU parsing
// happen on the load worker; GPU-backed mesh/texture creation is installed in
// bounded batches here so the render thread keeps pumping frames.
pub(crate) enum LoadPhase {
    Preparing,
    Meshes,
    Collisions,
    Cells,
}

fn scene_metadata_root(root: &Path, source: LoadSceneSource, loaded_wip: bool) -> PathBuf {
    match source {
        LoadSceneSource::Autosave => {
            let autosave = autosave_root_path(root);
            if autosave.join("EagleScene.eaglescne").is_file() {
                autosave
            } else {
                root.to_path_buf()
            }
        }
        LoadSceneSource::Saved if loaded_wip => {
            let wip = wip_root_path(root);
            if wip.join("EagleScene.eaglescne").is_file() {
                wip
            } else {
                root.to_path_buf()
            }
        }
        LoadSceneSource::Saved => root.to_path_buf(),
    }
}

// State machine that loads a resource across many frames. Cheap scene metadata
// setup happens up front in `new`; archive/texture indexing and DFF/COL parsing
// run on the worker, while GPU uploads and scene assembly stay on the main
// thread where the graphics context is valid.
pub(crate) struct LoadJob {
    pub(crate) options: Options,
    pub(crate) source: LoadSceneSource,
    pub(crate) root: PathBuf,
    pub(crate) t0: Instant,
    pub(crate) ui_font: Font,
    pub(crate) icons: IconSet,
    // Cheap setup results.
    pub(crate) zones: Vec<String>,
    pub(crate) defs: HashMap<String, Definition>,
    pub(crate) readonly_definition_ids: HashSet<String>,
    pub(crate) placements: Vec<Placement>,
    pub(crate) eagle_zone_offsets: EagleZoneOffsets,
    pub(crate) texture_files: HashMap<String, PathBuf>,
    pub(crate) txd_textures: TxdTextureIndex,
    pub(crate) camera_speed: f32,
    pub(crate) gta_sa_dir: PathBuf,
    pub(crate) physics_root_properties: HashMap<u16, PhysicsRootProperties>,
    pub(crate) bake_settings: BakeSettings,
    pub(crate) vertex_paint: VertexPaintSettings,
    pub(crate) lights: Vec<EditorLight>,
    pub(crate) water_planes: Vec<WaterPlane>,
    pub(crate) timecyc: TimecycState,
    pub(crate) vehicles: Vec<VehicleAsset>,
    pub(crate) custom_vehicle_dictionaries: Vec<PathBuf>,
    pub(crate) loaded_wip: bool,
    // Background asset preparation and bounded main-thread installation.
    asset_batch_rx: mpsc::Receiver<LoadAssetBatch>,
    asset_progress_rx: mpsc::Receiver<String>,
    pub(crate) mesh_total: usize,
    pub(crate) collision_total: usize,
    pub(crate) meshes_installed: usize,
    pub(crate) collisions_installed: usize,
    pub(crate) worker_finished: bool,
    pub(crate) meshes: HashMap<String, RenderMesh>,
    pub(crate) collisions: HashMap<String, CollisionMesh>,
    pub(crate) textures: HashMap<String, u32>,
    pub(crate) textured_parts: usize,
    pub(crate) vertices: usize,
    pub(crate) triangles: usize,
    pub(crate) collision_faces: usize,
    pub(crate) phase: LoadPhase,
    pub(crate) status: String,
}

impl LoadJob {
    pub(crate) fn new(options: Options, ui_font: Font, icons: IconSet) -> LoadJob {
        LoadJob::new_with_source(options, ui_font, icons, LoadSceneSource::Saved)
    }

    pub(crate) fn new_with_source(
        options: Options,
        ui_font: Font,
        icons: IconSet,
        source: LoadSceneSource,
    ) -> LoadJob {
        let t0 = Instant::now();
        let root = options.root.clone();
        let attr_re = Regex::new(r#"([A-Za-z_][A-Za-z0-9_]*)="([^"]*)""#).unwrap();
        let (
            zones,
            defs,
            readonly_definition_ids,
            placements,
            lights,
            eagle_zone_offsets,
            loaded_wip,
        ) = load_scene_source_with_source(&root, &attr_re, source);
        let texture_files = collect_scene_texture_files(&root, source);
        // Translation speed belongs to this window and is intentionally not
        // loaded from the shared preferences file.
        let camera_speed = DEFAULT_CAMERA_SPEED;
        let gta_sa_dir = load_gta_sa_dir_preference();
        let physics_root_properties = load_physics_root_properties(&gta_sa_dir);
        let bake_settings = load_bake_settings_preference();
        let vertex_paint = load_vertex_paint_settings_preference();
        let water_planes = load_water_dat_for_source(&root, source);
        let timecyc = load_timecyc_state();
        let custom_vehicle_dictionaries = load_custom_vehicle_dictionary_preferences();
        let vehicles = load_vehicle_assets(&root, &gta_sa_dir, &[]);

        let (asset_batch_tx, asset_batch_rx) = mpsc::sync_channel(LOAD_WORKER_QUEUE_DEPTH);
        let (asset_progress_tx, asset_progress_rx) = mpsc::channel();
        let worker_root = root.clone();
        let worker_gta_sa_dir = gta_sa_dir.clone();
        let worker_placements = placements.clone();
        let worker_defs = defs.clone();
        let load_meshes = options.meshes;
        let load_textures = options.textures;
        thread::Builder::new()
            .name("eagle-map-assets".to_string())
            .spawn(move || {
                run_load_asset_worker(
                    worker_root,
                    source,
                    worker_gta_sa_dir,
                    worker_placements,
                    worker_defs,
                    load_meshes,
                    load_textures,
                    asset_batch_tx,
                    asset_progress_tx,
                );
            })
            .expect("failed to start map asset loading worker");

        LoadJob {
            options,
            source,
            root,
            t0,
            ui_font,
            icons,
            zones,
            defs,
            readonly_definition_ids,
            placements,
            eagle_zone_offsets,
            texture_files,
            txd_textures: HashMap::new(),
            camera_speed,
            gta_sa_dir,
            physics_root_properties,
            bake_settings,
            vertex_paint,
            lights,
            water_planes,
            timecyc,
            vehicles,
            custom_vehicle_dictionaries,
            loaded_wip,
            asset_batch_rx,
            asset_progress_rx,
            mesh_total: 0,
            collision_total: 0,
            meshes_installed: 0,
            collisions_installed: 0,
            worker_finished: false,
            meshes: HashMap::new(),
            collisions: HashMap::new(),
            textures: HashMap::new(),
            textured_parts: 0,
            vertices: 0,
            triangles: 0,
            collision_faces: 0,
            phase: LoadPhase::Preparing,
            status: "Reading archives".to_string(),
        }
    }

    // Advance the load by one bounded chunk. Returns true once the job is done
    // and `finish` can be called to assemble the AppState.
    pub(crate) fn step(&mut self) -> bool {
        let mut latest_progress = None;
        while let Ok(progress) = self.asset_progress_rx.try_recv() {
            latest_progress = Some(progress);
        }
        if matches!(self.phase, LoadPhase::Preparing) {
            if let Some(progress) = latest_progress {
                self.status = progress;
            }
        }

        match self.asset_batch_rx.try_recv() {
            Ok(LoadAssetBatch::Indexed {
                txd_textures,
                mesh_total,
                collision_total,
            }) => {
                self.txd_textures = txd_textures;
                self.mesh_total = mesh_total;
                self.collision_total = collision_total;
                self.phase = if mesh_total > 0 {
                    LoadPhase::Meshes
                } else {
                    LoadPhase::Collisions
                };
                self.status = if mesh_total > 0 {
                    format!("Compiling mesh batches (0/{mesh_total})")
                } else {
                    format!("Installing collision batches (0/{collision_total})")
                };
                false
            }
            Ok(LoadAssetBatch::Meshes(batch)) => {
                self.phase = LoadPhase::Meshes;
                let ambient_lift = scene_ambient_lift_from_timecyc(&self.timecyc);
                for (mesh_key, raw, txd_name) in batch {
                    self.vertices += raw.vertices.len();
                    self.triangles += raw.triangles.len();
                    if let Some(mesh) = compile_render_mesh(
                        raw,
                        txd_name.as_deref(),
                        None,
                        None,
                        &self.texture_files,
                        &self.txd_textures,
                        &mut self.textures,
                        &mut self.textured_parts,
                        self.options.textures,
                        ambient_lift,
                    ) {
                        self.meshes.insert(mesh_key, mesh);
                    }
                    self.meshes_installed += 1;
                }
                self.status = format!(
                    "Compiling mesh batches ({}/{})",
                    self.meshes_installed, self.mesh_total
                );
                false
            }
            Ok(LoadAssetBatch::Collisions(batch)) => {
                self.phase = LoadPhase::Collisions;
                for (name, mesh) in batch {
                    self.collision_faces += mesh.faces.len();
                    self.collisions.insert(name, mesh);
                }
                self.collisions_installed =
                    (self.collisions_installed + LOAD_COLLISION_BATCH).min(self.collision_total);
                self.status = format!(
                    "Installing collision batches ({}/{})",
                    self.collisions_installed, self.collision_total
                );
                false
            }
            Ok(LoadAssetBatch::Finished) => {
                self.worker_finished = true;
                self.phase = LoadPhase::Cells;
                self.status = "Building scene".to_string();
                true
            }
            Err(mpsc::TryRecvError::Empty) => false,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.phase = LoadPhase::Cells;
                self.status = if self.worker_finished {
                    "Building scene".to_string()
                } else {
                    "Asset worker stopped; building the assets that completed".to_string()
                };
                true
            }
        }
    }

    pub(crate) fn progress(&self) -> &str {
        &self.status
    }

    // Assemble the finished AppState. Builds the scene/world cells (which issue
    // GPU work) here in a single final step, then mirrors load_app's struct.
    pub(crate) fn finish(self) -> AppState {
        let LoadJob {
            options,
            source,
            root,
            t0,
            ui_font,
            icons,
            zones,
            defs,
            readonly_definition_ids,
            placements,
            eagle_zone_offsets,
            texture_files,
            txd_textures,
            camera_speed,
            gta_sa_dir,
            physics_root_properties,
            bake_settings,
            vertex_paint,
            lights,
            water_planes,
            timecyc,
            vehicles,
            custom_vehicle_dictionaries,
            loaded_wip,
            mut meshes,
            collisions,
            mut textures,
            mut textured_parts,
            vertices,
            triangles,
            collision_faces,
            ..
        } = self;

        load_simulation_assets(
            &mut meshes,
            &mut textures,
            &mut textured_parts,
            options.textures,
        );
        let element_states = vec![ElementState::default(); placements.len()];
        let active_placements = active_placements(&placements, &element_states);
        let world_source = if options.vbo_selected_only && !active_placements.is_empty() {
            &active_placements[0..1]
        } else {
            active_placements.as_slice()
        };
        let lod_ids = collect_lod_ids(world_source);
        let all_lod_ids = collect_lod_ids(&placements);
        let ambient_lift = scene_ambient_lift_from_timecyc(&timecyc);
        let particle_effects = load_gta_sa_particle_effects(&gta_sa_dir);
        // Only build the cell set the active render path will actually draw.
        // fast_vbo (default) uses world_cells; the display-list scene_cells are
        // only consulted when fast_vbo is off. Building both doubled end-of-load
        // GPU work for no benefit.
        let (scene_cells, world_cells, lod_scene_cells, lod_world_cells) = if options.fast_vbo {
            (
                Vec::new(),
                build_world_cells(
                    world_source,
                    &defs,
                    &meshes,
                    options.vbo_immediate,
                    &lod_ids,
                    false,
                    ambient_lift,
                ),
                Vec::new(),
                build_world_cells(
                    world_source,
                    &defs,
                    &meshes,
                    options.vbo_immediate,
                    &lod_ids,
                    true,
                    ambient_lift,
                ),
            )
        } else {
            (
                build_scene_cells(world_source, &defs, &meshes, &lod_ids, false, ambient_lift),
                Vec::new(),
                build_scene_cells(world_source, &defs, &meshes, &lod_ids, true, ambient_lift),
                Vec::new(),
            )
        };

        let outliner_labels = build_outliner_label_slots(placements.len());
        let expanded_groups = BTreeSet::new();
        let outliner_filter = build_outliner_filter(
            &placements,
            "",
            &expanded_groups,
            &all_lod_ids,
            true,
            true,
            true,
        );
        let gpu_lightmap = init_gpu_lightmap_pipeline();
        let selected_water_planes = initial_water_selection(&water_planes);
        let metadata_root = scene_metadata_root(&root, source, loaded_wip);
        let MaterialClassesLoad {
            classes: material_classes,
            diagnostics: material_class_diagnostics,
        } = load_material_classes(&metadata_root);

        let active_tab = launch_initial_tab(options.launch_mode);
        let mut app = AppState {
            options,
            root: root.clone(),
            placements,
            definitions: defs,
            readonly_definition_ids,
            zones,
            eagle_zone_offsets,
            meshes,
            collisions,
            collision_render_cache: HashMap::new(),
            lod_ids: all_lod_ids,
            scene_cells,
            world_cells,
            lod_scene_cells,
            lod_world_cells,
            textures,
            txd_textures,
            texture_alias_count: texture_files.len(),
            texture_files,
            texture_overrides: HashMap::new(),
            pending_replacement_assets: BTreeMap::new(),
            pending_txd_writes: HashSet::new(),
            pending_asset_deletes: match source {
                LoadSceneSource::Saved => load_wip_asset_deletes(&root),
                LoadSceneSource::Autosave => load_autosave_asset_deletes(&root),
            },
            pending_vertex_light_meshes: HashSet::new(),
            material_emitters: load_material_emitters(&metadata_root),
            material_emitters_dirty: false,
            material_classes,
            material_classes_dirty: false,
            safe_collisions: load_safe_collisions(&metadata_root),
            safe_collisions_dirty: false,
            shadow_casting: load_shadow_casting(&metadata_root),
            selected: NO_SELECTION,
            selected_elements: BTreeSet::new(),
            selected_element_order: Vec::new(),
            hovered: None,
            outliner_labels,
            outliner_filter,
            outliner_search: String::new(),
            outliner_search_cursor: 0,
            outliner_search_anchor: None,
            outliner_search_active: false,
            outliner_show_objects: true,
            outliner_show_buildings: true,
            outliner_show_lods: true,
            asset_browser: AssetBrowserState::default(),
            vehicles,
            vehicle_browser: VehicleBrowserState::default(),
            custom_vehicle_dictionaries,
            vehicle_folder_picker_rx: None,
            vehicle_dictionary_scan_rx: None,
            vehicle_loader_resource: load_vehicle_loader_resource_preference(),
            vehicle_loader_membership: HashSet::new(),
            vehicle_loader_categories: Vec::new(),
            vehicle_loader_used_categories: HashSet::new(),
            vehicle_loader_picker_rx: None,
            vehicle_loader_scan_rx: None,
            vehicle_build_rx: None,
            validation_cache: None,
            validation_action_category: ValidationActionCategory::default(),
            duplicate_placement_scan_rx: None,
            lod_audit: lod_audit_state_from_preferences(&root),
            missing_texture_review: MissingTextureReviewState::default(),
            element_states,
            transform_mode: TransformMode::Move,
            transform_space: TransformSpace::World,
            snap_enabled: false,
            show_selected_lod_local: false,
            lod_selectable: true,
            snap_move: 32.0,
            snap_rotate: 15.0,
            active_tab,
            viewport_render_mode: ViewportRenderMode::default(),
            race: RaceEditorState::default(),
            editing: EditingState::default(),
            properties_tab: PropertiesTab::Element,
            properties_scroll: 0.0,
            validation_list_scroll: [0.0; 3],
            validation_list_scroll_drag: None,
            validation_list_scroll_grab_offset_y: 0.0,
            element_panel_collapsed: element_default_collapsed(),
            preview_selected_material: None,
            texture_match_selection_job: None,
            water_texture_conversion_job: None,
            physics_scope: PhysicsScope::default(),
            physics_root_properties,
            physics_root_dropdown_open: false,
            settings_panel_collapsed: settings_default_collapsed(),
            global_transform: GlobalTransformState::default(),
            collision_edit_mode: false,
            selected_col_face: None,
            selected_col_vertex: 0,
            hovered_col_face: None,
            hovered_col_vertex: None,
            col_material_dropdown_open: false,
            col_material_dropdown_scroll: 0.0,
            pending_col_writes: HashMap::new(),
            hovered_gizmo: None,
            gizmo_drag: None,
            col_box_face_drag: None,
            col_box_hovered_face: None,
            inspector_edit: None,
            group_rename: None,
            race_name_edit: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            scroll: 0.0,
            scroll_interaction_until: 0.0,
            outliner_scroll_drag: None,
            scrollbar_pointer_captured: false,
            inspector_scroll_drag: false,
            water_list_scroll_drag: false,
            light_list_scroll_drag: false,
            box_select_drag: None,
            box_select_distance: DEFAULT_BOX_SELECT_DISTANCE,
            box_select_mode: BoxSelectMode::Add,
            selected_group: None,
            expanded_groups,
            context_menu: None,
            load_dialog: None,
            preferences_dialog: None,
            load_picker_rx: None,
            dff_picker_rx: None,
            blender_import_rx: None,
            dff_repair_rx: None,
            dff_repair_refresh: None,
            dff_repair_scope: DffRepairScope::default(),
            dff_repair_menu_open: false,
            pending_load_root: None,
            pending_load_source: LoadSceneSource::Saved,
            load_job: None,
            save_as_dialog: None,
            dff_replace_choice_dialog: None,
            dff_prelight_import_dialog: None,
            dff_merge_choice_dialog: None,
            dff_optimize_dialog: None,
            dff_txd_pair_dialog: None,
            dff_optimize_options: DffOptimizeOptions::default(),
            dff_texture_duplicate_dialog: None,
            dff_texture_view_dialog: None,
            element_id_rename_dialog: None,
            missing_texture_dialog: None,
            texture_archive_dialog: None,
            missing_col_dialog: None,
            lod_batch_dialog: None,
            confirm_dialog: None,
            txd_cleanup_job: None,
            asset_optimization_scan_rx: None,
            asset_optimization_job: None,
            asset_optimization_scope: AssetOptimizationScope::default(),
            asset_optimization_menu_open: false,
            navigation_menu_open: false,
            purge_unused_job: None,
            img_archive_rebalance_job: None,
            object_bounds_fix_job: None,
            corona_generation_job: None,
            day_night_merge_job: None,
            day_night_merge_override: None,
            light_lod_job: None,
            fracture_generation_job: None,
            dff_geometry_job: None,
            collision_generation_job: None,
            shadow_mesh_generation_job: None,
            collision_cuboid_audit_job: None,
            lod_generation_job: None,
            instance_lod_removal_job: None,
            collision_generation_preset: CollisionGenerationPreset::Auto,
            collision_generation_fallback_material: 0,
            camera: CameraState {
                pos: vec3(0.0, -160.0, 80.0),
                yaw: 42.0_f32.to_radians(),
                pitch: -24.0_f32.to_radians(),
                last_mouse: mouse_position().into(),
                looking: false,
            },
            gameworld_camera: None,
            gameworld_camera_mode: None,
            gameworld_camera_focus: None,
            camera_mode: CameraMode::Freeroam,
            camera_focus: None,
            loaded_message: String::new(),
            load_seconds: t0.elapsed().as_secs_f32(),
            textured_parts,
            last_drawn_placements: 0,
            last_drawn_parts: 0,
            last_drawn_vertices: 0,
            last_log: Instant::now(),
            render_settle_until: get_time() + SETTLE_SECONDS,
            fps_last_frame: Instant::now(),
            fps_display: 0,
            ui_font,
            icons,
            status_message: "Ready".to_string(),
            activity_log: Vec::new(),
            activity_last_status: String::new(),
            activity_started_at: Instant::now(),
            save_log: Vec::new(),
            save_log_open: false,
            save_log_scroll: 0.0,
            save_log_follow_tail: true,
            camera_speed,
            vehicle_camera_speed: DEFAULT_DETAIL_CAMERA_SPEED,
            editing_camera_speed: DEFAULT_DETAIL_CAMERA_SPEED,
            camera_rotation_speed: load_camera_rotation_speed_preference(),
            gizmo_scale: load_gizmo_scale_preference(),
            gta_sa_dir,
            particle_effects,
            bake_settings,
            vertex_paint,
            prelight_clipboard: None,
            vertex_paint_dirty_meshes: HashSet::new(),
            vertex_paint_next_rebuild_at: 0.0,
            bake_job: None,
            gpu_lightmap,
            postfx_preview_texture: 0,
            postfx_preview_size: (0, 0),
            dff_pointlight_preview_program: 0,
            dff_pointlight_preview_failed: false,
            lights,
            water_planes,
            selected_water: 0,
            selected_water_planes,
            hovered_water: None,
            hovered_water_edge: None,
            water_edge_drag: None,
            water_edge_snap_enabled: true,
            water_scroll: 0.0,
            selected_light: 0,
            light_list_scroll: 0.0,
            light_kind_dropdown_open: false,
            light_profile_dropdown_open: false,
            light_color_drag_before: None,
            light_temperature_drag_before: None,
            timecyc,
            fog_strength: load_fog_strength_preference(),
            sim: SimState::default(),
            saved_snapshot: None,
            manual_save_job: None,
            pending_after_manual_save: None,
            autosave_next_at: get_time() + AUTOSAVE_INTERVAL_SECONDS,
            autosave_rx: None,
            autosave_dirty_snapshot: None,
            autosave_cleanup_rx: None,
            autosave_restore_prompted: false,
            loaded_autosave: source == LoadSceneSource::Autosave,
            loaded_wip,
            quit_after_persist: false,
            last_camera_persist_at: 0.0,
            sim_editor_camera: None,
            pending_camera_restore: None,
        };
        app.gameworld_camera = Some(app.camera);
        app.gameworld_camera_mode = Some(app.camera_mode);
        app.gameworld_camera_focus = Some(app.camera_focus);
        schedule_project_camera_restore(&mut app);
        restore_project_editing_session(&mut app);
        for diagnostic in material_class_diagnostics {
            eprintln!("{diagnostic}");
            app.activity_log.push(diagnostic);
        }
        app.saved_snapshot = Some(saved_content_snapshot(&app));
        app.autosave_dirty_snapshot = Some(autosave_dirty_snapshot(&app));
        if source == LoadSceneSource::Autosave {
            app.status_message =
                "Recovery copy restored. Save writes it into the resource; Save WIP keeps a separate working copy."
                    .to_string();
        } else if loaded_wip {
            app.status_message =
                "Loaded saved WIP snapshot. Save writes it back to the resource.".to_string();
        }
        app.loaded_message = format!(
            "Loaded {} DFF meshes, {} COL meshes ({} faces), {} placements, {} cells, {} batches, {vertices} vertices, {triangles} triangles, {} textures, {} textured parts in {:.2}s",
            app.meshes.len(),
            app.collisions.len(),
            collision_faces,
            app.placements.len(),
            app.world_cells.len(),
            app.world_cells
                .iter()
                .map(|cell| cell.batches.len())
                .sum::<usize>(),
            app.textures.len(),
            app.textured_parts,
            app.load_seconds
        );
        if !app.placements.is_empty() {
            snap_to(&mut app, 0);
        }
        if !app.custom_vehicle_dictionaries.is_empty() {
            start_custom_vehicle_dictionary_scan(&mut app);
        }
        start_vehicle_loader_scan(&mut app);
        println!("{}", app.loaded_message);
        app
    }
}

pub(crate) async fn load_app(options: Options, ui_font: Font) -> AppState {
    load_app_with_source(options, ui_font, LoadSceneSource::Saved).await
}

pub(crate) async fn load_app_with_source(
    options: Options,
    ui_font: Font,
    source: LoadSceneSource,
) -> AppState {
    let t0 = Instant::now();
    let root = options.root.clone();
    let icons = load_icons().await;
    let attr_re = Regex::new(r#"([A-Za-z_][A-Za-z0-9_]*)="([^"]*)""#).unwrap();
    let (zones, defs, readonly_definition_ids, placements, lights, eagle_zone_offsets, loaded_wip) =
        load_scene_source_with_source(&root, &attr_re, source);
    let texture_files = collect_scene_texture_files(&root, source);
    let timecyc = load_timecyc_state();
    let gta_sa_dir = load_gta_sa_dir_preference();
    let physics_root_properties = load_physics_root_properties(&gta_sa_dir);
    let particle_effects = load_gta_sa_particle_effects(&gta_sa_dir);

    let mut dff_entries = HashMap::<String, ImgEntry>::new();
    let mut col_entries = HashMap::<String, ImgEntry>::new();
    let mut txd_textures = TxdTextureIndex::new();
    let mut img_files = collect_scene_img_files(&root, source);
    img_files.extend(gta_sa_img_files(&gta_sa_dir));
    let overlay_root = match source {
        LoadSceneSource::Saved => wip_root_path(&root),
        LoadSceneSource::Autosave => autosave_root_path(&root),
    };
    for path in img_files {
        let archive_entries = parse_img(&path);
        for entry in &archive_entries {
            let entry_name = lower(&entry.name);
            if entry_name.ends_with(".dff") {
                if path.starts_with(&root) || !dff_entries.contains_key(&entry_name) {
                    dff_entries.insert(entry_name, entry.clone());
                }
            } else if entry_name.ends_with(".col")
                && (path.starts_with(&root) || !col_entries.contains_key(&entry_name))
            {
                col_entries.insert(entry_name, entry.clone());
            }
        }
        if options.textures {
            if path.starts_with(&overlay_root) {
                remove_img_txd_dictionaries_from_index(&mut txd_textures, &archive_entries);
            }
            index_txd_entries(&path, &archive_entries, &mut txd_textures);
        }
    }
    if options.textures {
        for path in collect_scene_txd_files(&root, source) {
            if path.starts_with(&overlay_root)
                && let Some(name) = path.file_name().and_then(|name| name.to_str())
            {
                remove_txd_dictionary_from_index(&mut txd_textures, name);
            }
            index_standalone_txd_file(&path, &mut txd_textures);
        }
        for path in [
            gta_sa_dir.join("models").join("particle.txd"),
            gta_sa_dir.join("models").join("effectsPC.txd"),
            gta_sa_dir
                .join("models")
                .join("generic")
                .join("vehicle.txd"),
        ] {
            if path.is_file() {
                index_standalone_txd_file(&path, &mut txd_textures);
            }
        }
    }

    let mut textures = HashMap::new();
    let mut meshes = HashMap::new();
    let mut textured_parts = 0;
    let mut vertices = 0usize;
    let mut triangles = 0usize;
    let ambient_lift = scene_ambient_lift_from_timecyc(&timecyc);
    if options.meshes {
        for (mesh_key, entry, txd_name) in
            build_referenced_dff_queue(&placements, &defs, &dff_entries)
        {
            let raw = parse_dff_mesh(&read_img_entry(&entry));
            vertices += raw.vertices.len();
            triangles += raw.triangles.len();
            if let Some(mesh) = compile_render_mesh(
                raw,
                txd_name.as_deref(),
                None,
                None,
                &texture_files,
                &txd_textures,
                &mut textures,
                &mut textured_parts,
                options.textures,
                ambient_lift,
            ) {
                meshes.insert(mesh_key, mesh);
            }
        }
    }
    let mut collisions = HashMap::new();
    let mut collision_faces = 0usize;
    for (name, entry) in col_entries {
        if let Some(mesh) = parse_col_mesh(&read_img_entry(&entry), &entry) {
            collision_faces += mesh.faces.len();
            collisions.insert(name, mesh);
        }
    }
    load_simulation_assets(
        &mut meshes,
        &mut textures,
        &mut textured_parts,
        options.textures,
    );
    let element_states = vec![ElementState::default(); placements.len()];
    let active_placements = active_placements(&placements, &element_states);
    let world_source = if options.vbo_selected_only && !active_placements.is_empty() {
        &active_placements[0..1]
    } else {
        active_placements.as_slice()
    };
    let lod_ids = collect_lod_ids(world_source);
    let all_lod_ids = collect_lod_ids(&placements);
    // Only build the cell set the active render path will draw (see finish()).
    let (scene_cells, world_cells, lod_scene_cells, lod_world_cells) = if options.fast_vbo {
        (
            Vec::new(),
            build_world_cells(
                world_source,
                &defs,
                &meshes,
                options.vbo_immediate,
                &lod_ids,
                false,
                ambient_lift,
            ),
            Vec::new(),
            build_world_cells(
                world_source,
                &defs,
                &meshes,
                options.vbo_immediate,
                &lod_ids,
                true,
                ambient_lift,
            ),
        )
    } else {
        (
            build_scene_cells(world_source, &defs, &meshes, &lod_ids, false, ambient_lift),
            Vec::new(),
            build_scene_cells(world_source, &defs, &meshes, &lod_ids, true, ambient_lift),
            Vec::new(),
        )
    };

    let outliner_labels = build_outliner_label_slots(placements.len());
    let expanded_groups = BTreeSet::new();
    let outliner_filter = build_outliner_filter(
        &placements,
        "",
        &expanded_groups,
        &all_lod_ids,
        true,
        true,
        true,
    );
    // Translation speed belongs to this window and is intentionally not
    // loaded from the shared preferences file.
    let camera_speed = DEFAULT_CAMERA_SPEED;
    let bake_settings = load_bake_settings_preference();
    let vertex_paint = load_vertex_paint_settings_preference();
    let water_planes = load_water_dat_for_source(&root, source);
    let custom_vehicle_dictionaries = load_custom_vehicle_dictionary_preferences();
    let vehicles = load_vehicle_assets(&root, &gta_sa_dir, &[]);
    let gpu_lightmap = init_gpu_lightmap_pipeline();
    let selected_water_planes = initial_water_selection(&water_planes);
    let metadata_root = scene_metadata_root(&root, source, loaded_wip);
    let MaterialClassesLoad {
        classes: material_classes,
        diagnostics: material_class_diagnostics,
    } = load_material_classes(&metadata_root);
    let active_tab = launch_initial_tab(options.launch_mode);
    let mut app = AppState {
        options,
        root: root.clone(),
        placements,
        definitions: defs,
        readonly_definition_ids,
        zones,
        eagle_zone_offsets,
        meshes,
        collisions,
        collision_render_cache: HashMap::new(),
        lod_ids: all_lod_ids,
        scene_cells,
        world_cells,
        lod_scene_cells,
        lod_world_cells,
        textures,
        txd_textures,
        texture_alias_count: texture_files.len(),
        texture_files,
        texture_overrides: HashMap::new(),
        pending_replacement_assets: BTreeMap::new(),
        pending_txd_writes: HashSet::new(),
        pending_asset_deletes: match source {
            LoadSceneSource::Saved => load_wip_asset_deletes(&root),
            LoadSceneSource::Autosave => load_autosave_asset_deletes(&root),
        },
        pending_vertex_light_meshes: HashSet::new(),
        material_emitters: load_material_emitters(&metadata_root),
        material_emitters_dirty: false,
        material_classes,
        material_classes_dirty: false,
        safe_collisions: load_safe_collisions(&metadata_root),
        safe_collisions_dirty: false,
        shadow_casting: load_shadow_casting(&metadata_root),
        selected: NO_SELECTION,
        selected_elements: BTreeSet::new(),
        selected_element_order: Vec::new(),
        hovered: None,
        outliner_labels,
        outliner_filter,
        outliner_search: String::new(),
        outliner_search_cursor: 0,
        outliner_search_anchor: None,
        outliner_search_active: false,
        outliner_show_objects: true,
        outliner_show_buildings: true,
        outliner_show_lods: true,
        asset_browser: AssetBrowserState::default(),
        vehicles,
        vehicle_browser: VehicleBrowserState::default(),
        custom_vehicle_dictionaries,
        vehicle_folder_picker_rx: None,
        vehicle_dictionary_scan_rx: None,
        vehicle_loader_resource: load_vehicle_loader_resource_preference(),
        vehicle_loader_membership: HashSet::new(),
        vehicle_loader_categories: Vec::new(),
        vehicle_loader_used_categories: HashSet::new(),
        vehicle_loader_picker_rx: None,
        vehicle_loader_scan_rx: None,
        vehicle_build_rx: None,
        validation_cache: None,
        validation_action_category: ValidationActionCategory::default(),
        duplicate_placement_scan_rx: None,
        lod_audit: lod_audit_state_from_preferences(&root),
        missing_texture_review: MissingTextureReviewState::default(),
        element_states,
        transform_mode: TransformMode::Move,
        transform_space: TransformSpace::World,
        snap_enabled: false,
        show_selected_lod_local: false,
        lod_selectable: true,
        snap_move: 32.0,
        snap_rotate: 15.0,
        active_tab,
        viewport_render_mode: ViewportRenderMode::default(),
        race: RaceEditorState::default(),
        editing: EditingState::default(),
        properties_tab: PropertiesTab::Element,
        properties_scroll: 0.0,
        validation_list_scroll: [0.0; 3],
        validation_list_scroll_drag: None,
        validation_list_scroll_grab_offset_y: 0.0,
        element_panel_collapsed: element_default_collapsed(),
        preview_selected_material: None,
        texture_match_selection_job: None,
        water_texture_conversion_job: None,
        physics_scope: PhysicsScope::default(),
        physics_root_properties,
        physics_root_dropdown_open: false,
        settings_panel_collapsed: settings_default_collapsed(),
        global_transform: GlobalTransformState::default(),
        collision_edit_mode: false,
        selected_col_face: None,
        selected_col_vertex: 0,
        hovered_col_face: None,
        hovered_col_vertex: None,
        col_material_dropdown_open: false,
        col_material_dropdown_scroll: 0.0,
        pending_col_writes: HashMap::new(),
        hovered_gizmo: None,
        gizmo_drag: None,
        col_box_face_drag: None,
        col_box_hovered_face: None,
        inspector_edit: None,
        group_rename: None,
        race_name_edit: None,
        undo_stack: Vec::new(),
        redo_stack: Vec::new(),
        scroll: 0.0,
        scroll_interaction_until: 0.0,
        outliner_scroll_drag: None,
        scrollbar_pointer_captured: false,
        inspector_scroll_drag: false,
        water_list_scroll_drag: false,
        light_list_scroll_drag: false,
        box_select_drag: None,
        box_select_distance: DEFAULT_BOX_SELECT_DISTANCE,
        box_select_mode: BoxSelectMode::Add,
        selected_group: None,
        expanded_groups,
        context_menu: None,
        load_dialog: None,
        preferences_dialog: None,
        load_picker_rx: None,
        dff_picker_rx: None,
        blender_import_rx: None,
        dff_repair_rx: None,
        dff_repair_refresh: None,
        dff_repair_scope: DffRepairScope::default(),
        dff_repair_menu_open: false,
        pending_load_root: None,
        pending_load_source: LoadSceneSource::Saved,
        load_job: None,
        save_as_dialog: None,
        dff_replace_choice_dialog: None,
        dff_prelight_import_dialog: None,
        dff_merge_choice_dialog: None,
        dff_optimize_dialog: None,
        dff_txd_pair_dialog: None,
        dff_optimize_options: DffOptimizeOptions::default(),
        dff_texture_duplicate_dialog: None,
        dff_texture_view_dialog: None,
        element_id_rename_dialog: None,
        missing_texture_dialog: None,
        texture_archive_dialog: None,
        missing_col_dialog: None,
        lod_batch_dialog: None,
        confirm_dialog: None,
        txd_cleanup_job: None,
        asset_optimization_scan_rx: None,
        asset_optimization_job: None,
        asset_optimization_scope: AssetOptimizationScope::default(),
        asset_optimization_menu_open: false,
        navigation_menu_open: false,
        purge_unused_job: None,
        img_archive_rebalance_job: None,
        object_bounds_fix_job: None,
        corona_generation_job: None,
        day_night_merge_job: None,
        day_night_merge_override: None,
        light_lod_job: None,
        fracture_generation_job: None,
        dff_geometry_job: None,
        collision_generation_job: None,
        shadow_mesh_generation_job: None,
        collision_cuboid_audit_job: None,
        lod_generation_job: None,
        instance_lod_removal_job: None,
        collision_generation_preset: CollisionGenerationPreset::Auto,
        collision_generation_fallback_material: 0,
        camera: CameraState {
            pos: vec3(0.0, -160.0, 80.0),
            yaw: 42.0_f32.to_radians(),
            pitch: -24.0_f32.to_radians(),
            last_mouse: mouse_position().into(),
            looking: false,
        },
        gameworld_camera: None,
        gameworld_camera_mode: None,
        gameworld_camera_focus: None,
        camera_mode: CameraMode::Freeroam,
        camera_focus: None,
        loaded_message: String::new(),
        load_seconds: t0.elapsed().as_secs_f32(),
        textured_parts,
        last_drawn_placements: 0,
        last_drawn_parts: 0,
        last_drawn_vertices: 0,
        last_log: Instant::now(),
        render_settle_until: get_time() + SETTLE_SECONDS,
        fps_last_frame: Instant::now(),
        fps_display: 0,
        ui_font,
        icons,
        status_message: "Ready".to_string(),
        activity_log: Vec::new(),
        activity_last_status: String::new(),
        activity_started_at: Instant::now(),
        save_log: Vec::new(),
        save_log_open: false,
        save_log_scroll: 0.0,
        save_log_follow_tail: true,
        camera_speed,
        vehicle_camera_speed: DEFAULT_DETAIL_CAMERA_SPEED,
        editing_camera_speed: DEFAULT_DETAIL_CAMERA_SPEED,
        camera_rotation_speed: load_camera_rotation_speed_preference(),
        gizmo_scale: load_gizmo_scale_preference(),
        gta_sa_dir,
        particle_effects,
        bake_settings,
        vertex_paint,
        prelight_clipboard: None,
        vertex_paint_dirty_meshes: HashSet::new(),
        vertex_paint_next_rebuild_at: 0.0,
        bake_job: None,
        gpu_lightmap,
        postfx_preview_texture: 0,
        postfx_preview_size: (0, 0),
        dff_pointlight_preview_program: 0,
        dff_pointlight_preview_failed: false,
        lights,
        water_planes,
        selected_water: 0,
        selected_water_planes,
        hovered_water: None,
        hovered_water_edge: None,
        water_edge_drag: None,
        water_edge_snap_enabled: true,
        water_scroll: 0.0,
        selected_light: 0,
        light_list_scroll: 0.0,
        light_kind_dropdown_open: false,
        light_profile_dropdown_open: false,
        light_color_drag_before: None,
        light_temperature_drag_before: None,
        timecyc,
        fog_strength: load_fog_strength_preference(),
        sim: SimState::default(),
        saved_snapshot: None,
        manual_save_job: None,
        pending_after_manual_save: None,
        autosave_next_at: get_time() + AUTOSAVE_INTERVAL_SECONDS,
        autosave_rx: None,
        autosave_dirty_snapshot: None,
        autosave_cleanup_rx: None,
        autosave_restore_prompted: false,
        loaded_autosave: source == LoadSceneSource::Autosave,
        loaded_wip,
        quit_after_persist: false,
        last_camera_persist_at: 0.0,
        sim_editor_camera: None,
        pending_camera_restore: None,
    };
    app.gameworld_camera = Some(app.camera);
    app.gameworld_camera_mode = Some(app.camera_mode);
    app.gameworld_camera_focus = Some(app.camera_focus);
    schedule_project_camera_restore(&mut app);
    restore_project_editing_session(&mut app);
    for diagnostic in material_class_diagnostics {
        eprintln!("{diagnostic}");
        app.activity_log.push(diagnostic);
    }
    app.saved_snapshot = Some(saved_content_snapshot(&app));
    app.autosave_dirty_snapshot = Some(autosave_dirty_snapshot(&app));
    if source == LoadSceneSource::Autosave {
        app.status_message =
            "Recovery copy restored. Save writes it into the resource; Save WIP keeps a separate working copy."
                .to_string();
    } else if loaded_wip {
        app.status_message =
            "Loaded saved WIP snapshot. Save writes it back to the resource.".to_string();
    }
    app.loaded_message = format!(
        "Loaded {} DFF meshes, {} COL meshes ({} faces), {} placements, {} cells, {} batches, {vertices} vertices, {triangles} triangles, {} textures, {} textured parts in {:.2}s",
        app.meshes.len(),
        app.collisions.len(),
        collision_faces,
        app.placements.len(),
        app.world_cells.len(),
        app.world_cells
            .iter()
            .map(|cell| cell.batches.len())
            .sum::<usize>(),
        app.textures.len(),
        app.textured_parts,
        app.load_seconds
    );
    if !app.placements.is_empty() {
        snap_to(&mut app, 0);
    }
    if !app.custom_vehicle_dictionaries.is_empty() {
        start_custom_vehicle_dictionary_scan(&mut app);
    }
    start_vehicle_loader_scan(&mut app);
    println!("{}", app.loaded_message);
    app
}

pub(crate) async fn load_icon(path: String) -> Texture2D {
    let texture = load_texture(&path).await.unwrap_or_else(|err| {
        eprintln!("Failed to load icon {path}: {err}");
        Texture2D::from_image(&Image::gen_image_color(24, 24, WHITE))
    });
    texture.set_filter(FilterMode::Linear);
    texture
}

pub(crate) async fn load_icons() -> IconSet {
    IconSet {
        select: load_icon(
            asset_path("icons/select.png")
                .to_string_lossy()
                .into_owned(),
        )
        .await,
        move_tool: load_icon(asset_path("icons/move.png").to_string_lossy().into_owned()).await,
        rotate: load_icon(
            asset_path("icons/rotate.png")
                .to_string_lossy()
                .into_owned(),
        )
        .await,
        duplicate: load_icon(
            asset_path("icons/duplicate.png")
                .to_string_lossy()
                .into_owned(),
        )
        .await,
        delete: load_icon(
            asset_path("icons/delete.png")
                .to_string_lossy()
                .into_owned(),
        )
        .await,
        face: load_icon(asset_path("icons/face.png").to_string_lossy().into_owned()).await,
        edge: load_icon(asset_path("icons/edge.png").to_string_lossy().into_owned()).await,
        vertex: load_icon(
            asset_path("icons/vertex.png")
                .to_string_lossy()
                .into_owned(),
        )
        .await,
        sphere: load_icon(
            asset_path("icons/sphere.png")
                .to_string_lossy()
                .into_owned(),
        )
        .await,
        cube: load_icon(asset_path("icons/cube.png").to_string_lossy().into_owned()).await,
        save: load_icon(asset_path("icons/save.png").to_string_lossy().into_owned()).await,
        undo: load_icon(asset_path("icons/undo.png").to_string_lossy().into_owned()).await,
        redo: load_icon(asset_path("icons/redo.png").to_string_lossy().into_owned()).await,
        texture: load_icon(
            asset_path("icons/texture.png")
                .to_string_lossy()
                .into_owned(),
        )
        .await,
        tool: load_icon(asset_path("icons/tool.png").to_string_lossy().into_owned()).await,
    }
}
