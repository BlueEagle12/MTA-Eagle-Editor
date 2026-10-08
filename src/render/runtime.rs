use super::super::*;

pub(crate) fn to_mq(v: V3) -> Vec3 {
    vec3(v.x, v.y, v.z)
}

pub(crate) fn from_mq(v: Vec3) -> V3 {
    V3 {
        x: v.x,
        y: v.y,
        z: v.z,
    }
}

pub(crate) fn fmt_f32(value: f32, decimals: usize) -> String {
    format!("{value:.decimals$}")
}

/// Return the equivalent Euler angle in the range used by MTA map rotations.
/// Keeping this at the placement/attribute boundary means every editor path
/// that changes an object's transform produces game-safe positive rotations.
fn positive_rotation_degrees(value: f32) -> f32 {
    if !value.is_finite() {
        return value;
    }

    let normalized = value.rem_euclid(360.0);
    // Very small negative values can round up to exactly 360 in f32. Both that
    // and negative zero should be written as the canonical zero rotation.
    if normalized >= 360.0 || normalized == 0.0 {
        0.0
    } else {
        normalized
    }
}

fn normalize_placement_rotation(placement: &mut Placement) {
    placement.rot.x = positive_rotation_degrees(placement.rot.x);
    placement.rot.y = positive_rotation_degrees(placement.rot.y);
    placement.rot.z = positive_rotation_degrees(placement.rot.z);
}

pub(crate) fn sync_placement_attrs(placement: &mut Placement) {
    normalize_placement_rotation(placement);
    if crate::resource::mta_maps::map_path(&placement.zone).is_some() {
        placement.attrs.insert("model".into(), placement.id.clone());
    } else {
        placement.attrs.insert("id".into(), placement.id.clone());
    }
    placement
        .attrs
        .insert("posX".to_string(), fmt_f32(placement.pos.x, 6));
    placement
        .attrs
        .insert("posY".to_string(), fmt_f32(placement.pos.y, 6));
    placement
        .attrs
        .insert("posZ".to_string(), fmt_f32(placement.pos.z, 6));
    placement
        .attrs
        .insert("rotX".to_string(), fmt_f32(placement.rot.x, 3));
    placement
        .attrs
        .insert("rotY".to_string(), fmt_f32(placement.rot.y, 3));
    placement
        .attrs
        .insert("rotZ".to_string(), fmt_f32(placement.rot.z, 3));
}

pub(crate) fn bounds_from_vertices(vertices: &[V3]) -> Bounds {
    if vertices.is_empty() {
        return Bounds {
            min: Vec3::ZERO,
            max: Vec3::ZERO,
        };
    }
    let mut min = to_mq(vertices[0]);
    let mut max = min;
    for v in vertices.iter().skip(1) {
        let p = to_mq(*v);
        min = min.min(p);
        max = max.max(p);
    }
    Bounds { min, max }
}

pub(crate) fn collision_mesh_bounds(
    vertices: &[V3],
    spheres: &[CollisionSphere],
    boxes: &[CollisionBox],
) -> Bounds {
    let mut points = Vec::with_capacity(vertices.len() + spheres.len() * 2 + boxes.len() * 2);
    points.extend_from_slice(vertices);
    for sphere in spheres {
        let r = sphere.radius.abs();
        points.push(V3 {
            x: sphere.center.x - r,
            y: sphere.center.y - r,
            z: sphere.center.z - r,
        });
        points.push(V3 {
            x: sphere.center.x + r,
            y: sphere.center.y + r,
            z: sphere.center.z + r,
        });
    }
    for col_box in boxes {
        points.push(V3 {
            x: col_box.min.x.min(col_box.max.x),
            y: col_box.min.y.min(col_box.max.y),
            z: col_box.min.z.min(col_box.max.z),
        });
        points.push(V3 {
            x: col_box.min.x.max(col_box.max.x),
            y: col_box.min.y.max(col_box.max.y),
            z: col_box.min.z.max(col_box.max.z),
        });
    }
    bounds_from_vertices(&points)
}

pub(crate) fn init_raw_gl() -> u32 {
    gl::load_with(|name| {
        let Ok(c_name) = CString::new(name) else {
            return std::ptr::null();
        };
        raw_gl_proc_address(c_name.as_ptr().cast())
    });
    // Do not mutate shared GL state here. Startup, the project picker, and
    // loading screens use Macroquad's 2D renderer before the first 3D frame.
    // In particular, leaving DEPTH_TEST enabled causes equal-depth UI quads to
    // disappear on Windows drivers. Each raw-GL render pass configures the
    // fixed-function state it needs and calls reset_gl_for_ui() afterward.
    0
}

fn current_gl_string(name: u32) -> String {
    let value = unsafe { gl::GetString(name) };
    if value.is_null() {
        return "unknown".to_string();
    }
    unsafe { std::ffi::CStr::from_ptr(value.cast()) }
        .to_string_lossy()
        .into_owned()
}

pub(crate) fn validate_legacy_gl_context() -> Result<(), String> {
    let version = current_gl_string(gl::VERSION);
    let renderer = current_gl_string(gl::RENDERER);
    let vendor = current_gl_string(gl::VENDOR);

    unsafe {
        for _ in 0..16 {
            if gl::GetError() == gl::NO_ERROR {
                break;
            }
        }
        // MatrixMode is removed from core profiles and is a harmless, direct
        // capability probe for the fixed-function API Eagle actually uses.
        gl::MatrixMode(gl::MODELVIEW);
        let error = gl::GetError();
        if error != gl::NO_ERROR {
            return Err(format!(
                "Eagle Editor requires an OpenGL compatibility context, but the driver supplied an incompatible context (OpenGL {version}, renderer {renderer}, vendor {vendor}, glMatrixMode error 0x{error:04x})."
            ));
        }
    }

    eprintln!("OpenGL compatibility context: {version} | {renderer} | {vendor}");
    Ok(())
}

#[cfg(target_os = "linux")]
fn raw_gl_proc_address(name: *const u8) -> *const c_void {
    unsafe { glXGetProcAddress(name) }
}

#[cfg(windows)]
fn raw_gl_proc_address(name: *const u8) -> *const c_void {
    unsafe {
        let extension = wglGetProcAddress(name);
        // WGL uses these sentinel values when a symbol is not an extension.
        if !extension.is_null() && (extension as usize) > 3 && extension as isize != -1 {
            return extension;
        }
        static OPENGL32: OnceLock<usize> = OnceLock::new();
        let module = *OPENGL32
            .get_or_init(|| LoadLibraryA(c"opengl32.dll".as_ptr().cast()) as usize)
            as *mut c_void;
        if module.is_null() {
            std::ptr::null()
        } else {
            GetProcAddress(module, name)
        }
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
fn raw_gl_proc_address(_name: *const u8) -> *const c_void {
    std::ptr::null()
}

pub(crate) fn reset_gl_for_ui() {
    reset_material_preview();
    set_lightmap_texture(0);
    let ui_viewport = (
        0,
        0,
        screen_width().max(1.0) as i32,
        screen_height().max(1.0) as i32,
    );
    unsafe {
        gl::UseProgram(0);
        gl::ActiveTexture(gl::TEXTURE0);
        gl::ClientActiveTexture(gl::TEXTURE1);
        gl::DisableClientState(gl::TEXTURE_COORD_ARRAY);
        gl::ClientActiveTexture(gl::TEXTURE0);
        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, 0);
        gl::BindTexture(gl::TEXTURE_2D, 0);
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::COLOR_MATERIAL);
        gl::Disable(gl::DEPTH_TEST);
        gl::Disable(gl::MULTISAMPLE);
        gl::Disable(gl::ALPHA_TEST);
        gl::Disable(gl::FOG);
        gl::Disable(gl::SCISSOR_TEST);
        gl::Disable(gl::CULL_FACE);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::DisableClientState(gl::VERTEX_ARRAY);
        gl::DisableClientState(gl::NORMAL_ARRAY);
        gl::DisableClientState(gl::TEXTURE_COORD_ARRAY);
        gl::DisableClientState(gl::COLOR_ARRAY);
        for index in 0..16 {
            gl::DisableVertexAttribArray(index);
        }
        gl::Color4f(1.0, 1.0, 1.0, 1.0);
        gl::Viewport(ui_viewport.0, ui_viewport.1, ui_viewport.2, ui_viewport.3);
        gl::MatrixMode(gl::MODELVIEW);
        gl::LoadIdentity();
        gl::MatrixMode(gl::PROJECTION);
        gl::LoadIdentity();
    }
    let mut gl = unsafe { get_internal_gl() };
    gl.flush();
    gl.quad_gl.texture(None);
    gl.quad_gl.draw_mode(DrawMode::Triangles);
    gl.quad_gl.viewport(Some(ui_viewport));
    gl.quad_gl.scissor(None);
    drop(gl);
    set_default_camera();
}

pub(crate) fn outline_offset_for_mesh(mesh: &RenderMesh) -> f32 {
    let diag = (mesh.bounds.max - mesh.bounds.min).length();
    (diag * 0.006).clamp(0.65, 8.0)
}

pub(crate) fn draw_outline_wire(mesh: &RenderMesh, color: [f32; 4], width: f32, offset: f32) {
    if color[3] <= 0.001 || width <= 0.001 {
        return;
    }
    unsafe {
        gl::Enable(gl::CULL_FACE);
        gl::FrontFace(gl::CW);
        gl::CullFace(gl::BACK);
        gl::Enable(gl::DEPTH_TEST);
        gl::DepthFunc(gl::LEQUAL);
        gl::DepthMask(gl::FALSE);
        gl::LineWidth(width);
        gl::Color4f(color[0], color[1], color[2], color[3]);
        gl::Begin(gl::LINES);
        for part in &mesh.parts {
            for tri in part.cpu_vertices.chunks_exact(3) {
                let mut points = [Vec3::ZERO; 3];
                for (slot, vertex) in points.iter_mut().zip(tri.iter()) {
                    let pos = to_mq(vertex.pos);
                    let normal = to_mq(vertex.normal);
                    let normal = if normal.length_squared() > 0.0001 {
                        normal.normalize()
                    } else {
                        Vec3::Z
                    };
                    *slot = pos + normal * offset;
                }
                for (a, b) in [(0usize, 1usize), (1, 2), (2, 0)] {
                    gl::Vertex3f(points[a].x, points[a].y, points[a].z);
                    gl::Vertex3f(points[b].x, points[b].y, points[b].z);
                }
            }
        }
        gl::End();
    }
}

fn bounds_outline_corners(bounds: Bounds, padding: f32) -> [Vec3; 8] {
    let padding = Vec3::splat(padding.max(0.0));
    let min = bounds.min - padding;
    let max = bounds.max + padding;
    [
        vec3(min.x, min.y, min.z),
        vec3(max.x, min.y, min.z),
        vec3(max.x, max.y, min.z),
        vec3(min.x, max.y, min.z),
        vec3(min.x, min.y, max.z),
        vec3(max.x, min.y, max.z),
        vec3(max.x, max.y, max.z),
        vec3(min.x, max.y, max.z),
    ]
}

fn draw_bounds_outline(bounds: Bounds, color: [f32; 4], width: f32, padding: f32) {
    const EDGES: [(usize, usize); 12] = [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 0),
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 4),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ];
    if color[3] <= 0.001 || width <= 0.001 {
        return;
    }
    let corners = bounds_outline_corners(bounds, padding);
    unsafe {
        gl::Enable(gl::DEPTH_TEST);
        gl::DepthFunc(gl::LEQUAL);
        gl::DepthMask(gl::FALSE);
        gl::LineWidth(width);
        gl::Color4f(color[0], color[1], color[2], color[3]);
        gl::Begin(gl::LINES);
        for (a, b) in EDGES {
            for point in [corners[a], corners[b]] {
                gl::Vertex3f(point.x, point.y, point.z);
            }
        }
        gl::End();
    }
}

pub(crate) fn draw_element_outline(app: &AppState, index: usize, selected: bool) {
    if index >= app.placements.len()
        || app
            .element_states
            .get(index)
            .is_some_and(|state| state.deleted || state.hidden)
    {
        return;
    }
    let placement = &app.placements[index];
    let Some(mesh) = element_mesh(app, placement) else {
        return;
    };
    let model = placement_matrix(placement);
    let offset = outline_offset_for_mesh(mesh);
    let vertex_lighting_edit = app.active_tab == AppTab::Bake;
    let wire = if selected && vertex_lighting_edit {
        [1.0, 0.82, 0.18, 0.14]
    } else if selected {
        [1.0, 0.82, 0.18, 0.72]
    } else {
        [0.42, 0.92, 1.0, 0.42]
    };
    let width = if selected && vertex_lighting_edit {
        0.55
    } else if selected {
        1.6
    } else {
        1.2
    };
    unsafe {
        gl::PushAttrib(
            gl::ENABLE_BIT
                | gl::POLYGON_BIT
                | gl::LINE_BIT
                | gl::CURRENT_BIT
                | gl::DEPTH_BUFFER_BIT,
        );
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::ALPHA_TEST);
        gl::Disable(gl::CULL_FACE);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::PushMatrix();
        gl::MultMatrixf(model.to_cols_array().as_ptr());
        if app.active_tab == AppTab::Collisions {
            // Collision geometry already supplies the detailed surface. A
            // second CPU walk over every visual-mesh triangle just to outline
            // the selection is disproportionately expensive on large maps.
            draw_bounds_outline(mesh.bounds, wire, width, offset * 0.08);
        } else {
            draw_outline_wire(mesh, wire, width, offset * 0.08);
        }
        gl::PopMatrix();
        gl::PopAttrib();
    }
}

fn draw_lod_batch_preview_outline(app: &AppState, index: usize) {
    if index >= app.placements.len()
        || app
            .element_states
            .get(index)
            .is_some_and(|state| state.deleted || state.hidden)
    {
        return;
    }
    let placement = &app.placements[index];
    let Some(mesh) = element_mesh(app, placement) else {
        return;
    };
    let model = placement_matrix(placement);
    let offset = outline_offset_for_mesh(mesh);
    unsafe {
        gl::PushAttrib(
            gl::ENABLE_BIT
                | gl::POLYGON_BIT
                | gl::LINE_BIT
                | gl::CURRENT_BIT
                | gl::DEPTH_BUFFER_BIT,
        );
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::ALPHA_TEST);
        gl::Disable(gl::CULL_FACE);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::PushMatrix();
        gl::MultMatrixf(model.to_cols_array().as_ptr());
        // Scene-wide previews can cover thousands of dense models. Bounds
        // mark every candidate clearly without walking every source triangle.
        draw_bounds_outline(mesh.bounds, [0.20, 0.62, 1.0, 0.86], 1.5, offset * 0.08);
        gl::PopMatrix();
        gl::PopAttrib();
    }
}

pub(crate) fn draw_editor_outlines(app: &AppState) {
    draw_preview_selected_material(app);
    if let Some(dialog) = app.lod_batch_dialog.as_ref()
        && dialog.mode != LodBatchMode::GenerateSelection
        && let Some(minimum_size) = lod_batch_minimum_size(dialog)
    {
        for candidate in &dialog.candidates {
            if lod_batch_candidate_included(candidate, minimum_size, dialog.mode) {
                draw_lod_batch_preview_outline(app, candidate.placement_index);
            }
        }
    }
    if let Some(hovered) = app.hovered {
        if !app.selected_elements.contains(&hovered) {
            draw_element_outline(app, hovered, false);
        }
    }
    for idx in selected_indices(app) {
        draw_element_outline(app, idx, true);
    }
    draw_center_of_mass_marker(app);
    draw_scene_lights(app);
    draw_transform_gizmo(app);
    draw_asset_browser_drag_preview(app);
}

pub(crate) fn draw_asset_browser_drag_preview(app: &AppState) {
    let Some(drag) = app.asset_browser.drag.as_ref() else {
        return;
    };
    let Some(position) = drag.position else {
        return;
    };
    let Some(definition) = app.definitions.get(&drag.entry_id) else {
        return;
    };
    let dff = definition
        .attrs
        .get("dff")
        .cloned()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| drag.entry_id.clone());
    let mut placement = Placement {
        id: drag.entry_id.clone(),
        dff,
        zone: definition.zone.clone(),
        tag: "object".to_string(),
        attrs: BTreeMap::new(),
        pos: position,
        rot: V3::default(),
    };
    placement
        .attrs
        .insert("alpha".to_string(), "112".to_string());
    let Some(mesh) = app
        .meshes
        .get(&placement_mesh_key(&placement, &app.definitions))
    else {
        return;
    };
    let ambient_lift = scene_ambient_lift_from_timecyc(&app.timecyc);
    let double_sided = placement_disable_backface_culling(&placement, &app.definitions);
    draw_placement_render_mesh(&placement, mesh, double_sided, ambient_lift);
    unsafe {
        gl::PushAttrib(
            gl::ENABLE_BIT
                | gl::POLYGON_BIT
                | gl::LINE_BIT
                | gl::CURRENT_BIT
                | gl::DEPTH_BUFFER_BIT,
        );
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::ALPHA_TEST);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::PushMatrix();
        gl::MultMatrixf(placement_matrix(&placement).to_cols_array().as_ptr());
        draw_outline_wire(
            mesh,
            [0.30, 0.84, 1.0, 0.92],
            1.8,
            outline_offset_for_mesh(mesh) * 0.08,
        );
        gl::PopMatrix();
        gl::PopAttrib();
    }
}

fn parse_center_of_mass_component(attrs: &BTreeMap<String, String>, key: &str) -> Option<f32> {
    attrs
        .get(key)?
        .trim()
        .parse::<f32>()
        .ok()
        .filter(|value| value.is_finite())
}

fn effective_center_of_mass(app: &AppState, placement: &Placement) -> Option<Vec3> {
    let definition_attrs = app
        .definitions
        .get(&placement.id)
        .map(|definition| &definition.attrs);
    let mut any_value = false;
    let mut component = |key: &str| {
        let value = parse_center_of_mass_component(&placement.attrs, key).or_else(|| {
            definition_attrs.and_then(|attrs| parse_center_of_mass_component(attrs, key))
        });
        any_value |= value.is_some();
        value.unwrap_or(0.0)
    };
    let center = vec3(
        component("centerOfMassX"),
        component("centerOfMassY"),
        component("centerOfMassZ"),
    );
    any_value.then_some(center)
}

fn draw_center_of_mass_marker(app: &AppState) {
    if app.active_tab != AppTab::Preview {
        return;
    }
    let Some(placement) = app.placements.get(app.selected) else {
        return;
    };
    if app
        .element_states
        .get(app.selected)
        .is_some_and(|state| state.deleted || state.hidden)
    {
        return;
    }
    let Some(local_center) = effective_center_of_mass(app, placement) else {
        return;
    };
    let center = placement_matrix(placement).transform_point3(local_center);
    let scale = gizmo_scale(app);
    let radius = (gizmo_visual_length(app, center) * 0.62).clamp(2.0 * scale, 6.0 * scale);

    unsafe {
        gl::PushAttrib(
            gl::ENABLE_BIT | gl::LINE_BIT | gl::POINT_BIT | gl::CURRENT_BIT | gl::DEPTH_BUFFER_BIT,
        );
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::ALPHA_TEST);
        gl::Disable(gl::CULL_FACE);
        gl::Disable(gl::DEPTH_TEST);
        gl::DepthMask(gl::FALSE);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

        gl::LineWidth(2.2);
        gl::Color4f(1.0, 0.32, 0.74, 0.92);
        for (a, b) in [(Vec3::X, Vec3::Y), (Vec3::X, Vec3::Z), (Vec3::Y, Vec3::Z)] {
            gl::Begin(gl::LINE_LOOP);
            for step in 0..40 {
                let angle = step as f32 / 40.0 * std::f32::consts::TAU;
                let point = center + (a * angle.cos() + b * angle.sin()) * radius;
                gl::Vertex3f(point.x, point.y, point.z);
            }
            gl::End();
        }

        gl::LineWidth(1.5);
        gl::Color4f(1.0, 0.82, 0.94, 0.88);
        gl::Begin(gl::LINES);
        for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
            let start = center - axis * radius * 1.45;
            let end = center + axis * radius * 1.45;
            gl::Vertex3f(start.x, start.y, start.z);
            gl::Vertex3f(end.x, end.y, end.z);
        }
        gl::End();

        gl::PointSize(9.0);
        gl::Color4f(1.0, 0.95, 0.35, 1.0);
        gl::Begin(gl::POINTS);
        gl::Vertex3f(center.x, center.y, center.z);
        gl::End();
        gl::PopAttrib();
    }
}

pub(crate) fn draw_scene_lights(app: &AppState) {
    // Expanding definition-backed lights walks the whole scene. These handles
    // are only interactive and useful in the dedicated Lights workspace.
    if app.active_tab != AppTab::Lights {
        return;
    }
    unsafe {
        gl::PushAttrib(
            gl::ENABLE_BIT | gl::LINE_BIT | gl::POINT_BIT | gl::CURRENT_BIT | gl::DEPTH_BUFFER_BIT,
        );
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::ALPHA_TEST);
        gl::Disable(gl::DEPTH_TEST);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        for (idx, _, light) in expanded_scene_lights(app) {
            let pos = vec3(light.position.x, light.position.y, light.position.z);
            let light_color = light_effective_color(&light);
            let selected = idx == app.selected_light;
            gl::PointSize(if selected { 14.0 } else { 8.0 });
            gl::Color4f(
                light_color.x,
                light_color.y,
                light_color.z,
                if selected { 1.0 } else { 0.72 },
            );
            gl::Begin(gl::POINTS);
            gl::Vertex3f(pos.x, pos.y, pos.z);
            gl::End();

            if selected {
                gl::LineWidth(2.0);
                gl::Color4f(1.0, 0.92, 0.18, 0.9);
                gl::Begin(gl::LINE_LOOP);
                let radius = match light.kind {
                    LightKind::Point | LightKind::Spot | LightKind::Area => {
                        light.radius.max(24.0).min(600.0)
                    }
                    _ => 34.0,
                };
                for i in 0..64 {
                    let t = i as f32 / 64.0 * std::f32::consts::TAU;
                    gl::Vertex3f(pos.x + t.cos() * radius, pos.y + t.sin() * radius, pos.z);
                }
                gl::End();
            }

            if matches!(
                light.kind,
                LightKind::Directional | LightKind::Spot | LightKind::Area
            ) {
                let dir = vec3(light.direction.x, light.direction.y, light.direction.z);
                if dir.length_squared() > 0.0001 {
                    let camera_distance = (pos - app.camera.pos).length().max(80.0);
                    let length = if selected {
                        (camera_distance * 0.16).clamp(140.0, 640.0)
                    } else {
                        (camera_distance * 0.09).clamp(80.0, 360.0)
                    };
                    gl::Color4f(
                        light_color.x,
                        light_color.y,
                        light_color.z,
                        if selected { 0.95 } else { 0.48 },
                    );
                    draw_light_direction_arrow(pos, dir.normalize(), length, selected);
                }
            }
        }
        gl::PopAttrib();
    }
}

/// Draws a camera-independent wire arrow whose tip is the exact direction in
/// which a directional or spot light travels.
fn draw_light_direction_arrow(origin: Vec3, direction: Vec3, length: f32, selected: bool) {
    let tip = origin + direction * length;
    let head_length = length * if selected { 0.24 } else { 0.20 };
    let head_radius = length * if selected { 0.085 } else { 0.07 };
    let base = tip - direction * head_length;
    let tangent = if direction.z.abs() < 0.9 {
        direction.cross(Vec3::Z).normalize_or_zero()
    } else {
        direction.cross(Vec3::X).normalize_or_zero()
    };
    let bitangent = direction.cross(tangent).normalize_or_zero();
    if tangent.length_squared() < 0.0001 || bitangent.length_squared() < 0.0001 {
        return;
    }
    unsafe {
        gl::LineWidth(if selected { 4.0 } else { 2.0 });
        gl::Begin(gl::LINES);
        gl::Vertex3f(origin.x, origin.y, origin.z);
        gl::Vertex3f(tip.x, tip.y, tip.z);
        for i in 0..8 {
            let angle = i as f32 / 8.0 * std::f32::consts::TAU;
            let rim =
                base + tangent * angle.cos() * head_radius + bitangent * angle.sin() * head_radius;
            gl::Vertex3f(tip.x, tip.y, tip.z);
            gl::Vertex3f(rim.x, rim.y, rim.z);
        }
        gl::End();
        gl::LineWidth(if selected { 2.5 } else { 1.5 });
        gl::Begin(gl::LINE_LOOP);
        for i in 0..16 {
            let angle = i as f32 / 16.0 * std::f32::consts::TAU;
            let rim =
                base + tangent * angle.cos() * head_radius + bitangent * angle.sin() * head_radius;
            gl::Vertex3f(rim.x, rim.y, rim.z);
        }
        gl::End();
    }
}

pub(crate) fn draw_axis_line(origin: Vec3, dir: Vec3, length: f32, color: [f32; 4], hovered: bool) {
    unsafe {
        if hovered {
            gl::LineWidth(4.0);
            let glow = glow_color(color);
            gl::Color4f(glow[0], glow[1], glow[2], 0.30);
            gl::Begin(gl::LINES);
            gl::Vertex3f(origin.x, origin.y, origin.z);
            let end = origin + dir * length;
            gl::Vertex3f(end.x, end.y, end.z);
            gl::End();
        }
        gl::LineWidth(if hovered { 2.5 } else { 1.5 });
        let alpha = if hovered { 0.90 } else { 0.64 };
        gl::Color4f(color[0], color[1], color[2], color[3] * alpha);
        gl::Begin(gl::LINES);
        gl::Vertex3f(origin.x, origin.y, origin.z);
        let end = origin + dir * length;
        gl::Vertex3f(end.x, end.y, end.z);
        gl::End();
        gl::PointSize(if hovered { 7.0 } else { 4.0 });
        gl::Begin(gl::POINTS);
        gl::Vertex3f(end.x, end.y, end.z);
        gl::End();
    }
}

pub(crate) fn draw_rotation_ring(
    origin: Vec3,
    a: Vec3,
    b: Vec3,
    radius: f32,
    color: [f32; 4],
    hovered: bool,
) {
    unsafe {
        if hovered {
            gl::LineWidth(4.0);
            let glow = glow_color(color);
            gl::Color4f(glow[0], glow[1], glow[2], 0.28);
            gl::Begin(gl::LINE_LOOP);
            for i in 0..96 {
                let t = i as f32 / 96.0 * std::f32::consts::TAU;
                let p = origin + a * t.cos() * radius + b * t.sin() * radius;
                gl::Vertex3f(p.x, p.y, p.z);
            }
            gl::End();
        }
        gl::LineWidth(if hovered { 2.5 } else { 1.5 });
        let alpha = if hovered { 0.84 } else { 0.54 };
        gl::Color4f(color[0], color[1], color[2], color[3] * alpha);
        gl::Begin(gl::LINE_LOOP);
        for i in 0..96 {
            let t = i as f32 / 96.0 * std::f32::consts::TAU;
            let p = origin + a * t.cos() * radius + b * t.sin() * radius;
            gl::Vertex3f(p.x, p.y, p.z);
        }
        gl::End();
    }
}

fn draw_gizmo_center_ring(origin: Vec3, camera_pos: Vec3, length: f32, scale: f32) {
    let view_dir = (camera_pos - origin).normalize_or_zero();
    let view_dir = if view_dir.length_squared() > 0.0001 {
        view_dir
    } else {
        Vec3::Z
    };
    let tangent = if view_dir.z.abs() < 0.9 {
        view_dir.cross(Vec3::Z).normalize_or_zero()
    } else {
        view_dir.cross(Vec3::X).normalize_or_zero()
    };
    let bitangent = view_dir.cross(tangent).normalize_or_zero();
    let radius = (length * 0.12).clamp(3.0 * scale, 13.0 * scale);
    unsafe {
        gl::LineWidth(1.5);
        gl::Color4f(0.88, 0.88, 0.88, 0.72);
        gl::Begin(gl::LINE_LOOP);
        for i in 0..40 {
            let t = i as f32 / 40.0 * std::f32::consts::TAU;
            let p = origin + tangent * t.cos() * radius + bitangent * t.sin() * radius;
            gl::Vertex3f(p.x, p.y, p.z);
        }
        gl::End();
    }
}

fn draw_scale_plane(app: &AppState, origin: Vec3, length: f32, plane: GizmoPlane) {
    let (first, second) = gizmo_plane_axes(plane);
    let a = selected_axis_vector(app, first);
    let b = selected_axis_vector(app, second);
    let low = length * 0.18;
    let high = length * 0.38;
    let hovered = app.hovered_gizmo_plane == Some(plane);
    let first_color = axis_color(first);
    let second_color = axis_color(second);
    let color = [
        (first_color[0] + second_color[0]) * 0.5,
        (first_color[1] + second_color[1]) * 0.5,
        (first_color[2] + second_color[2]) * 0.5,
        if hovered { 0.72 } else { 0.38 },
    ];
    let points = [
        origin + a * low + b * low,
        origin + a * high + b * low,
        origin + a * high + b * high,
        origin + a * low + b * high,
    ];
    unsafe {
        gl::Color4f(color[0], color[1], color[2], color[3]);
        gl::Begin(gl::QUADS);
        for point in points {
            gl::Vertex3f(point.x, point.y, point.z);
        }
        gl::End();
        gl::LineWidth(if hovered { 3.0 } else { 1.5 });
        gl::Color4f(color[0], color[1], color[2], 0.95);
        gl::Begin(gl::LINE_LOOP);
        for point in points {
            gl::Vertex3f(point.x, point.y, point.z);
        }
        gl::End();
    }
}

pub(crate) fn draw_transform_gizmo(app: &AppState) {
    if app.transform_mode == TransformMode::Select {
        return;
    }
    let Some(origin) = selected_origin(app) else {
        return;
    };
    let length = gizmo_visual_length(app, origin);
    unsafe {
        gl::PushAttrib(
            gl::ENABLE_BIT | gl::LINE_BIT | gl::POINT_BIT | gl::CURRENT_BIT | gl::DEPTH_BUFFER_BIT,
        );
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::ALPHA_TEST);
        gl::Disable(gl::DEPTH_TEST);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        draw_gizmo_center_ring(origin, app.camera.pos, length, gizmo_scale(app));
        match app.transform_mode {
            TransformMode::Select => {}
            TransformMode::Move | TransformMode::Scale => {
                for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
                    draw_axis_line(
                        origin,
                        selected_axis_vector(app, axis),
                        length,
                        axis_color(axis),
                        app.hovered_gizmo == Some(axis),
                    );
                }
                if app.transform_mode == TransformMode::Scale {
                    for plane in [GizmoPlane::XY, GizmoPlane::XZ, GizmoPlane::YZ] {
                        draw_scale_plane(app, origin, length, plane);
                    }
                }
            }
            TransformMode::Rotate => {
                for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
                    let (a, b) = selected_ring_basis(app, axis);
                    draw_rotation_ring(
                        origin,
                        a,
                        b,
                        length * 0.75,
                        axis_color(axis),
                        app.hovered_gizmo == Some(axis),
                    );
                }
            }
        }
        gl::PopAttrib();
    }
}

// --- Text rendering -------------------------------------------------------
//
// macroquad's own `draw_text` is unreliable in this app because we interleave
// raw OpenGL (the 3D scene) with macroquad's 2D batcher, which corrupts its
// lazily-built glyph atlas. A single packed atlas drawn with `source` sub-rects
// also renders as garbage here. Instead we give every glyph its OWN small
// texture and draw it with `draw_texture_ex` and NO source rect -- byte for
// byte the same code path as the toolbar icons, which render correctly.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placement_rotations_use_equivalent_positive_angles() {
        assert_eq!(positive_rotation_degrees(-30.0), 330.0);
        assert_eq!(positive_rotation_degrees(-390.0), 330.0);
        assert_eq!(positive_rotation_degrees(390.0), 30.0);
        assert_eq!(positive_rotation_degrees(-360.0), 0.0);
        assert_eq!(positive_rotation_degrees(360.0), 0.0);
        assert_eq!(positive_rotation_degrees(-0.0).to_bits(), 0.0_f32.to_bits());

        let mut placement = Placement {
            id: "rotation_test".to_string(),
            dff: "rotation_test".to_string(),
            zone: "test".to_string(),
            tag: "object".to_string(),
            attrs: BTreeMap::new(),
            pos: V3::default(),
            rot: V3 {
                x: -30.0,
                y: 390.0,
                z: -720.0,
            },
        };

        sync_placement_attrs(&mut placement);

        assert_eq!(
            placement.rot,
            V3 {
                x: 330.0,
                y: 30.0,
                z: 0.0
            }
        );
        assert_eq!(
            placement.attrs.get("rotX").map(String::as_str),
            Some("330.000")
        );
        assert_eq!(
            placement.attrs.get("rotY").map(String::as_str),
            Some("30.000")
        );
        assert_eq!(
            placement.attrs.get("rotZ").map(String::as_str),
            Some("0.000")
        );
    }

    #[test]
    fn bounds_outline_corners_expand_all_axes() {
        let corners = bounds_outline_corners(
            Bounds {
                min: vec3(-1.0, -2.0, -3.0),
                max: vec3(4.0, 5.0, 6.0),
            },
            0.5,
        );

        assert_eq!(corners[0], vec3(-1.5, -2.5, -3.5));
        assert_eq!(corners[2], vec3(4.5, 5.5, -3.5));
        assert_eq!(corners[5], vec3(4.5, -2.5, 6.5));
        assert_eq!(corners[7], vec3(-1.5, 5.5, 6.5));
    }

    #[test]
    fn bounds_outline_corners_clamp_negative_padding() {
        let bounds = Bounds {
            min: vec3(-1.0, -2.0, -3.0),
            max: vec3(4.0, 5.0, 6.0),
        };

        assert_eq!(
            bounds_outline_corners(bounds, -10.0),
            bounds_outline_corners(bounds, 0.0)
        );
    }
}
