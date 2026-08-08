use super::super::*;

pub(crate) fn placement_matrix(p: &Placement) -> Mat4 {
    let pos = to_mq(p.pos);
    Mat4::from_translation(pos)
        * Mat4::from_rotation_z(p.rot.z.to_radians())
        * Mat4::from_rotation_y(p.rot.y.to_radians())
        * Mat4::from_rotation_x(p.rot.x.to_radians())
        * Mat4::from_scale(Vec3::splat(placement_scale(p)))
}

pub(crate) fn transform_point_gl(m: &[f32; 16], p: V3) -> Vec3 {
    vec3(
        m[0] * p.x + m[4] * p.y + m[8] * p.z + m[12],
        m[1] * p.x + m[5] * p.y + m[9] * p.z + m[13],
        m[2] * p.x + m[6] * p.y + m[10] * p.z + m[14],
    )
}

pub(crate) fn transform_normal_gl(m: &[f32; 16], n: V3) -> Vec3 {
    let normal = vec3(
        m[0] * n.x + m[4] * n.y + m[8] * n.z,
        m[1] * n.x + m[5] * n.y + m[9] * n.z,
        m[2] * n.x + m[6] * n.y + m[10] * n.z,
    );
    if normal.length_squared() > 0.0001 {
        normal.normalize()
    } else {
        Vec3::Z
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Plane {
    pub(crate) normal: Vec3,
    pub(crate) d: f32,
}

pub(crate) fn make_plane(a: f32, b: f32, c: f32, d: f32) -> Plane {
    let normal = vec3(a, b, c);
    let len = normal.length();
    if len > 0.0001 {
        Plane {
            normal: normal / len,
            d: d / len,
        }
    } else {
        Plane { normal, d }
    }
}

pub(crate) fn frustum_planes(view_projection: Mat4) -> [Plane; 6] {
    let m = view_projection.to_cols_array();
    let r0 = [m[0], m[4], m[8], m[12]];
    let r1 = [m[1], m[5], m[9], m[13]];
    let r2 = [m[2], m[6], m[10], m[14]];
    let r3 = [m[3], m[7], m[11], m[15]];
    [
        make_plane(r3[0] + r0[0], r3[1] + r0[1], r3[2] + r0[2], r3[3] + r0[3]),
        make_plane(r3[0] - r0[0], r3[1] - r0[1], r3[2] - r0[2], r3[3] - r0[3]),
        make_plane(r3[0] + r1[0], r3[1] + r1[1], r3[2] + r1[2], r3[3] + r1[3]),
        make_plane(r3[0] - r1[0], r3[1] - r1[1], r3[2] - r1[2], r3[3] - r1[3]),
        make_plane(r3[0] + r2[0], r3[1] + r2[1], r3[2] + r2[2], r3[3] + r2[3]),
        make_plane(r3[0] - r2[0], r3[1] - r2[1], r3[2] - r2[2], r3[3] - r2[3]),
    ]
}

pub(crate) fn sphere_in_frustum(planes: &[Plane; 6], center: Vec3, radius: f32) -> bool {
    planes
        .iter()
        .all(|plane| plane.normal.dot(center) + plane.d >= -radius)
}

pub(crate) fn aabb_in_frustum(planes: &[Plane; 6], min: Vec3, max: Vec3) -> bool {
    planes.iter().all(|plane| {
        let p = vec3(
            if plane.normal.x >= 0.0 { max.x } else { min.x },
            if plane.normal.y >= 0.0 { max.y } else { min.y },
            if plane.normal.z >= 0.0 { max.z } else { min.z },
        );
        plane.normal.dot(p) + plane.d >= 0.0
    })
}

/// Draw a set of pre-batched world cells (fast VBO / immediate path) whose
/// distance from the camera falls in the band `[near, far]`. Cells are visited
/// nearest-first so a vertex budget drops the farthest geometry. Counters are
/// shared across calls so the budget spans the detail + LOD sets together.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_world_cell_set(
    cells: &[WorldCell],
    cam: Vec3,
    near: f32,
    far: f32,
    frustum: &[Plane; 6],
    immediate: bool,
    vertex_budget: usize,
    drawn_placements: &mut usize,
    drawn_parts: &mut usize,
    drawn_vertices: &mut usize,
) {
    let mut order: Vec<(f32, usize)> = cells
        .iter()
        .enumerate()
        .filter_map(|(i, cell)| {
            let distance_squared = (cell.center - cam).length_squared();
            let far_edge = far + cell.radius;
            if distance_squared > far_edge * far_edge {
                return None;
            }
            let near_edge = near - cell.radius;
            if near_edge > 0.0 && distance_squared < near_edge * near_edge {
                return None;
            }
            if !aabb_in_frustum(frustum, cell.min, cell.max) {
                return None;
            }
            Some((distance_squared, i))
        })
        .collect();
    order.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    // Populate the depth buffer with all solid/cutout geometry before drawing
    // any translucent surface.  Blended geometry cannot write depth (otherwise
    // its transparent pixels hide geometry behind it), but it must still be
    // depth-tested and drawn back-to-front to preserve the scene's depth order.
    let mut visible_blend_segments = Vec::new();
    let mut state = WorldDrawState::new(immediate);
    unsafe {
        if !immediate {
            enable_world_client_arrays();
        }
        for (_, idx) in order {
            let cell = &cells[idx];
            if *drawn_vertices + cell.vertices > vertex_budget {
                if *drawn_vertices > 0 {
                    break;
                }
                if cell.vertices > vertex_budget {
                    continue;
                }
            }
            *drawn_placements += cell.placements;
            *drawn_parts += cell.parts;
            *drawn_vertices += cell.vertices;
            for batch in &cell.batches {
                if batch.transparency != TransparencyMode::Blend {
                    state.set_pass(WorldPassState::Solid(batch.transparency));
                    state.draw_batch(batch, cell.vbo, None);
                } else {
                    // A texture may contain both solid wall pixels and a small
                    // translucent region. Populate depth for its fully opaque
                    // texels so that one alpha face cannot turn the rest of the
                    // DFF into depthless geometry.
                    state.set_pass(WorldPassState::BlendOpaque);
                    state.draw_batch(batch, cell.vbo, None);
                }
            }
            for (batch_idx, batch) in cell.batches.iter().enumerate() {
                if batch.transparency == TransparencyMode::Blend {
                    visible_blend_segments.extend(batch.blend_segments.iter().enumerate().map(
                        |(segment_idx, segment)| {
                            (
                                (segment.center - cam).length_squared(),
                                idx,
                                batch_idx,
                                segment_idx,
                            )
                        },
                    ));
                }
            }
        }

        // Alpha parts remain separate and are sorted by their own centers,
        // rather than inheriting the order of an entire world cell.
        visible_blend_segments
            .sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        state.set_pass(WorldPassState::BlendColor);
        for (_, cell_idx, batch_idx, segment_idx) in visible_blend_segments {
            let batch = &cells[cell_idx].batches[batch_idx];
            state.draw_batch(
                batch,
                cells[cell_idx].vbo,
                Some(&batch.blend_segments[segment_idx]),
            );
        }
        gl::Enable(gl::CULL_FACE);
        gl::LightModeli(gl::LIGHT_MODEL_TWO_SIDE, gl::FALSE as i32);
        gl::Disable(gl::BLEND);
        gl::Enable(gl::ALPHA_TEST);
        gl::DepthMask(gl::TRUE);
        gl::DepthFunc(gl::LEQUAL);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WorldPassState {
    Solid(TransparencyMode),
    BlendOpaque,
    BlendColor,
}

struct WorldDrawState {
    immediate: bool,
    pass: Option<WorldPassState>,
    double_sided: Option<bool>,
    use_lighting: Option<bool>,
    texture: Option<u32>,
    vbo: u32,
}

impl WorldDrawState {
    fn new(immediate: bool) -> Self {
        Self {
            immediate,
            pass: None,
            double_sided: None,
            use_lighting: None,
            texture: None,
            vbo: 0,
        }
    }

    fn set_pass(&mut self, pass: WorldPassState) {
        if self.pass == Some(pass) {
            return;
        }
        match pass {
            WorldPassState::Solid(transparency) => set_transparency_state(transparency),
            WorldPassState::BlendOpaque => set_blend_opaque_pass_state(),
            WorldPassState::BlendColor => set_blend_color_pass_state(),
        }
        self.pass = Some(pass);
    }

    fn draw_batch(&mut self, batch: &WorldBatch, vbo: u32, segment: Option<&WorldBlendSegment>) {
        unsafe {
            if self.double_sided != Some(batch.double_sided) {
                if batch.double_sided {
                    gl::Disable(gl::CULL_FACE);
                    gl::LightModeli(gl::LIGHT_MODEL_TWO_SIDE, gl::TRUE as i32);
                } else {
                    gl::Enable(gl::CULL_FACE);
                    gl::LightModeli(gl::LIGHT_MODEL_TWO_SIDE, gl::FALSE as i32);
                }
                self.double_sided = Some(batch.double_sided);
            }
            if self.use_lighting != Some(batch.use_lighting) {
                if batch.use_lighting {
                    gl::Enable(gl::LIGHTING);
                } else {
                    gl::Disable(gl::LIGHTING);
                }
                self.use_lighting = Some(batch.use_lighting);
            }
            if self.texture != Some(batch.texture) {
                if batch.texture != 0 {
                    gl::Enable(gl::TEXTURE_2D);
                    gl::BindTexture(gl::TEXTURE_2D, batch.texture);
                    gl::Color4f(1.0, 1.0, 1.0, 1.0);
                } else {
                    gl::Disable(gl::TEXTURE_2D);
                    gl::Color4f(0.78, 0.78, 0.74, 1.0);
                }
                self.texture = Some(batch.texture);
            }

            let (local_first_vertex, vertices) = segment
                .map(|segment| (segment.first_vertex, segment.vertices))
                .unwrap_or((0, batch.vertices));
            if self.immediate {
                draw_world_batch_immediate(batch, local_first_vertex, vertices);
            } else {
                if self.vbo != vbo {
                    bind_world_vbo(vbo);
                    self.vbo = vbo;
                }
                let first_vertex = batch
                    .first_vertex
                    .saturating_add(local_first_vertex)
                    .min(i32::MAX as usize);
                gl::DrawArrays(
                    gl::TRIANGLES,
                    first_vertex as i32,
                    vertices.min(i32::MAX as usize) as i32,
                );
            }
        }
    }
}

unsafe fn enable_world_client_arrays() {
    unsafe {
        gl::EnableClientState(gl::VERTEX_ARRAY);
        gl::EnableClientState(gl::NORMAL_ARRAY);
        gl::EnableClientState(gl::TEXTURE_COORD_ARRAY);
        gl::EnableClientState(gl::COLOR_ARRAY);
    }
}

unsafe fn bind_world_vbo(vbo: u32) {
    let stride = (12 * std::mem::size_of::<f32>()) as i32;
    unsafe {
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::TexCoordPointer(2, gl::FLOAT, stride, std::ptr::null());
        gl::NormalPointer(
            gl::FLOAT,
            stride,
            (2 * std::mem::size_of::<f32>()) as *const c_void,
        );
        gl::ColorPointer(
            4,
            gl::FLOAT,
            stride,
            (5 * std::mem::size_of::<f32>()) as *const c_void,
        );
        gl::VertexPointer(
            3,
            gl::FLOAT,
            stride,
            (9 * std::mem::size_of::<f32>()) as *const c_void,
        );
    }
}

unsafe fn draw_world_batch_immediate(batch: &WorldBatch, first_vertex: usize, vertices: usize) {
    let first = first_vertex.saturating_mul(12);
    let end = first
        .saturating_add(vertices.saturating_mul(12))
        .min(batch.data.len());
    unsafe {
        gl::Begin(gl::TRIANGLES);
        for v in batch.data[first..end].chunks_exact(12) {
            gl::TexCoord2f(v[0], v[1]);
            gl::Normal3f(v[2], v[3], v[4]);
            gl::Color4f(v[5], v[6], v[7], v[8]);
            gl::Vertex3f(v[9], v[10], v[11]);
        }
        gl::End();
    }
}

fn set_blend_opaque_pass_state() {
    unsafe {
        gl::Enable(gl::DEPTH_TEST);
        gl::Disable(gl::BLEND);
        gl::Enable(gl::ALPHA_TEST);
        gl::AlphaFunc(gl::GREATER, 0.98);
        gl::DepthFunc(gl::LEQUAL);
        gl::DepthMask(gl::TRUE);
    }
}

fn set_blend_color_pass_state() {
    unsafe {
        gl::Enable(gl::DEPTH_TEST);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::Enable(gl::ALPHA_TEST);
        gl::AlphaFunc(gl::GREATER, 0.01);
        // Opaque texels were already colored and wrote depth in the first pass.
        // LESS prevents drawing them twice while admitting translucent texels,
        // which did not participate in that pass.
        gl::DepthFunc(gl::LESS);
        gl::DepthMask(gl::FALSE);
    }
}

fn set_transparency_state(transparency: TransparencyMode) {
    unsafe {
        match transparency {
            TransparencyMode::Blend => {
                gl::Enable(gl::DEPTH_TEST);
                gl::Enable(gl::BLEND);
                gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
                gl::Disable(gl::ALPHA_TEST);
                gl::DepthMask(gl::FALSE);
            }
            TransparencyMode::Opaque | TransparencyMode::Cutout => {
                gl::Disable(gl::BLEND);
                gl::Enable(gl::ALPHA_TEST);
                gl::AlphaFunc(gl::GREATER, 0.08);
                gl::DepthMask(gl::TRUE);
            }
        }
    }
}

fn draw_render_part_buffer(part: &RenderPart) {
    unsafe {
        if part.vbo == 0 {
            gl::CallList(part.list);
            return;
        }
        if part.texture != 0 {
            gl::Enable(gl::TEXTURE_2D);
            gl::BindTexture(gl::TEXTURE_2D, part.texture);
        } else {
            gl::Disable(gl::TEXTURE_2D);
        }
        let stride = (12 * std::mem::size_of::<f32>()) as i32;
        gl::BindBuffer(gl::ARRAY_BUFFER, part.vbo);
        gl::EnableClientState(gl::VERTEX_ARRAY);
        gl::EnableClientState(gl::NORMAL_ARRAY);
        gl::EnableClientState(gl::TEXTURE_COORD_ARRAY);
        gl::EnableClientState(gl::COLOR_ARRAY);
        gl::TexCoordPointer(2, gl::FLOAT, stride, std::ptr::null());
        gl::NormalPointer(
            gl::FLOAT,
            stride,
            (2 * std::mem::size_of::<f32>()) as *const c_void,
        );
        gl::ColorPointer(
            4,
            gl::FLOAT,
            stride,
            (5 * std::mem::size_of::<f32>()) as *const c_void,
        );
        gl::VertexPointer(
            3,
            gl::FLOAT,
            stride,
            (9 * std::mem::size_of::<f32>()) as *const c_void,
        );
        gl::DrawArrays(gl::TRIANGLES, 0, part.vertices as i32);
        gl::DisableClientState(gl::COLOR_ARRAY);
        gl::DisableClientState(gl::TEXTURE_COORD_ARRAY);
        gl::DisableClientState(gl::NORMAL_ARRAY);
        gl::DisableClientState(gl::VERTEX_ARRAY);
        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        gl::BindTexture(gl::TEXTURE_2D, 0);
        gl::Disable(gl::TEXTURE_2D);
    }
}

/// Draw a set of pre-compiled scene cells (display-list path) in the band
/// `[near, far]`, honoring the part / vertex budgets. Cells are visited
/// nearest-first so budgets drop the farthest geometry.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_scene_cell_set(
    cells: &[SceneCell],
    cam: Vec3,
    near: f32,
    far: f32,
    frustum: &[Plane; 6],
    part_budget: usize,
    vertex_budget: usize,
    drawn_placements: &mut usize,
    drawn_parts: &mut usize,
    drawn_vertices: &mut usize,
) {
    let mut order: Vec<(f32, usize)> = cells
        .iter()
        .enumerate()
        .map(|(i, c)| ((c.center - cam).length(), i))
        .collect();
    order.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    for (dist, idx) in order {
        let cell = &cells[idx];
        if *drawn_parts >= part_budget || *drawn_vertices >= vertex_budget {
            break;
        }
        if dist > far + cell.radius || dist + cell.radius < near {
            continue;
        }
        if !aabb_in_frustum(frustum, cell.min, cell.max) {
            continue;
        }
        if *drawn_parts + cell.parts > part_budget
            || *drawn_vertices + cell.vertices > vertex_budget
        {
            continue;
        }
        unsafe { gl::CallList(cell.list) };
        *drawn_placements += cell.placements;
        *drawn_parts += cell.parts;
        *drawn_vertices += cell.vertices;
    }
}

pub(crate) fn draw_placement_render_mesh(
    placement: &Placement,
    mesh: &RenderMesh,
    double_sided: bool,
    ambient_lift: V3,
) -> (usize, usize) {
    let model = placement_matrix(placement);
    let model_cols = model.to_cols_array();
    let alpha = placement_alpha(placement);
    let mut parts = 0usize;
    let mut vertices = 0usize;
    unsafe {
        if double_sided {
            gl::Disable(gl::CULL_FACE);
            gl::LightModeli(gl::LIGHT_MODEL_TWO_SIDE, gl::TRUE as i32);
        }
        gl::PushMatrix();
        gl::MultMatrixf(model_cols.as_ptr());
        for part in &mesh.parts {
            let transparency = if alpha < 0.999 {
                TransparencyMode::Blend
            } else {
                part.transparency
            };
            set_transparency_state(transparency);
            if part.use_lighting {
                gl::Enable(gl::LIGHTING);
            } else {
                gl::Disable(gl::LIGHTING);
            }
            if alpha < 0.999 {
                draw_render_part_immediate(part, alpha, ambient_lift);
            } else {
                draw_render_part_buffer(part);
            }
            parts += 1;
            vertices += part.vertices;
        }
        gl::PopMatrix();
        if double_sided {
            gl::LightModeli(gl::LIGHT_MODEL_TWO_SIDE, gl::FALSE as i32);
            gl::Enable(gl::CULL_FACE);
        }
    }
    (parts, vertices)
}

pub(crate) fn draw_preview_selected_material(app: &AppState) {
    if app.active_tab != AppTab::Preview {
        return;
    }
    let Some((placement_index, material_index)) = app.preview_selected_material else {
        return;
    };
    if placement_index != app.selected || !is_visible_element(app, placement_index) {
        return;
    }
    let Some(placement) = app.placements.get(placement_index) else {
        return;
    };
    let Some(mesh) = element_mesh(app, placement) else {
        return;
    };
    let selected_parts = mesh
        .parts
        .iter()
        .filter(|part| part.material_index == material_index)
        .collect::<Vec<_>>();
    if selected_parts.is_empty() {
        return;
    }

    unsafe {
        gl::PushAttrib(gl::ALL_ATTRIB_BITS);
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::ALPHA_TEST);
        gl::Disable(gl::CULL_FACE);
        gl::Enable(gl::DEPTH_TEST);
        gl::DepthFunc(gl::LEQUAL);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::DepthMask(gl::FALSE);
        gl::Enable(gl::POLYGON_OFFSET_FILL);
        gl::PolygonOffset(-1.0, -1.0);
        gl::PushMatrix();
        gl::MultMatrixf(placement_matrix(placement).to_cols_array().as_ptr());

        gl::Color4f(1.0, 0.68, 0.06, 0.42);
        gl::Begin(gl::TRIANGLES);
        for part in &selected_parts {
            for vertex in &part.cpu_vertices {
                gl::Vertex3f(vertex.pos.x, vertex.pos.y, vertex.pos.z);
            }
        }
        gl::End();

        gl::Disable(gl::POLYGON_OFFSET_FILL);
        gl::LineWidth(2.0);
        gl::Color4f(1.0, 0.92, 0.20, 0.95);
        gl::Begin(gl::LINES);
        for part in selected_parts {
            for triangle in part.cpu_vertices.chunks_exact(3) {
                for (a, b) in [(0, 1), (1, 2), (2, 0)] {
                    let p = triangle[a].pos;
                    let q = triangle[b].pos;
                    gl::Vertex3f(p.x, p.y, p.z);
                    gl::Vertex3f(q.x, q.y, q.z);
                }
            }
        }
        gl::End();
        gl::PopMatrix();
        gl::PopAttrib();
    }
}

fn collision_classification_color(
    classes: &TextureMaterialClasses,
    txd_name: Option<&str>,
    texture_name: &str,
    fingerprint: Option<TextureContentFingerprint>,
    fallback_material: u8,
) -> [f32; 4] {
    let resolved =
        classes.resolve_with_content(txd_name, texture_name, fingerprint, fallback_material);
    if resolved.no_collision {
        // Intentionally outside the normal material palette so excluded faces
        // cannot be mistaken for a generated collision surface.
        [1.0, 0.08, 0.64, 1.0]
    } else {
        collision_material_color(resolved.material, 1.0)
    }
}

fn draw_render_part_view_mode(
    part: &RenderPart,
    placement_alpha: f32,
    mode: ViewportRenderMode,
    classification: [f32; 4],
) {
    unsafe {
        gl::Disable(gl::LIGHTING);
        match mode {
            ViewportRenderMode::ShadedTextured => unreachable!(),
            ViewportRenderMode::UnshadedTextured => {
                let transparency = if placement_alpha < 0.999 {
                    TransparencyMode::Blend
                } else {
                    part.transparency
                };
                set_transparency_state(transparency);
                if part.texture != 0 {
                    gl::Enable(gl::TEXTURE_2D);
                    gl::BindTexture(gl::TEXTURE_2D, part.texture);
                } else {
                    gl::Disable(gl::TEXTURE_2D);
                }
                gl::Begin(gl::TRIANGLES);
                for vertex in &part.cpu_vertices {
                    let alpha = placement_alpha * part.alpha * vertex.alpha;
                    gl::Color4f(1.0, 1.0, 1.0, alpha);
                    gl::TexCoord2f(vertex.uv.u, vertex.uv.v);
                    gl::Vertex3f(vertex.pos.x, vertex.pos.y, vertex.pos.z);
                }
                gl::End();
            }
            ViewportRenderMode::CollisionClassification => {
                gl::Disable(gl::TEXTURE_2D);
                gl::Disable(gl::BLEND);
                gl::Disable(gl::ALPHA_TEST);
                gl::DepthMask(gl::TRUE);
                gl::Color4f(
                    classification[0],
                    classification[1],
                    classification[2],
                    classification[3],
                );
                gl::Begin(gl::TRIANGLES);
                for vertex in &part.cpu_vertices {
                    gl::Vertex3f(vertex.pos.x, vertex.pos.y, vertex.pos.z);
                }
                gl::End();
            }
        }
    }
}

fn draw_placement_render_mesh_view_mode(
    placement: &Placement,
    mesh: &RenderMesh,
    double_sided: bool,
    mode: ViewportRenderMode,
    classes: &TextureMaterialClasses,
    txd_name: Option<&str>,
    fallback_material: u8,
) -> (usize, usize) {
    let model_cols = placement_matrix(placement).to_cols_array();
    let placement_alpha = placement_alpha(placement);
    unsafe {
        if double_sided {
            gl::Disable(gl::CULL_FACE);
        }
        gl::PushMatrix();
        gl::MultMatrixf(model_cols.as_ptr());
        for part in &mesh.parts {
            let classification = collision_classification_color(
                classes,
                txd_name,
                &part.texture_name,
                part.texture_fingerprint,
                fallback_material,
            );
            draw_render_part_view_mode(part, placement_alpha, mode, classification);
        }
        gl::PopMatrix();
        if double_sided {
            gl::Enable(gl::CULL_FACE);
        }
        gl::Disable(gl::BLEND);
        gl::Enable(gl::ALPHA_TEST);
        gl::DepthMask(gl::TRUE);
    }
    (
        mesh.parts.len(),
        mesh.parts.iter().map(|part| part.vertices).sum(),
    )
}

fn draw_preview_view_mode_scene(
    app: &AppState,
    frustum: &[Plane; 6],
    cam: Vec3,
    far: f32,
    lod_near: f32,
    lod_far: f32,
    part_budget: usize,
    vertex_budget: usize,
) -> (usize, usize, usize) {
    let mut visible = Vec::<(f32, usize)>::new();
    for (index, placement) in app.placements.iter().enumerate() {
        if app
            .element_states
            .get(index)
            .is_some_and(|state| state.deleted || state.hidden)
        {
            continue;
        }
        let Some(mesh) = element_mesh(app, placement) else {
            continue;
        };
        let model_cols = placement_matrix(placement).to_cols_array();
        let bounds = transformed_bounds(mesh.bounds, &model_cols);
        let center = (bounds.min + bounds.max) * 0.5;
        let radius = (bounds.max - center).length() + 64.0;
        let distance = (center - cam).length();
        let is_lod = placement_is_app_lod(app, placement);
        let in_distance = if is_lod {
            app.options.lod_mode != LodMode::DetailOnly
                && distance + radius >= lod_near
                && distance - radius <= lod_far
        } else {
            distance - radius <= far
        };
        if in_distance && aabb_in_frustum(frustum, bounds.min, bounds.max) {
            visible.push((distance, index));
        }
    }
    visible.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut placements = 0usize;
    let mut parts = 0usize;
    let mut vertices = 0usize;
    for (_, index) in visible {
        let placement = &app.placements[index];
        let Some(mesh) = element_mesh(app, placement) else {
            continue;
        };
        let mesh_parts = mesh.parts.len();
        let mesh_vertices = mesh.parts.iter().map(|part| part.vertices).sum::<usize>();
        if parts + mesh_parts > part_budget || vertices + mesh_vertices > vertex_budget {
            continue;
        }
        let double_sided = placement_disable_backface_culling(placement, &app.definitions);
        let txd_name = definition_txd_name(&app.definitions, &placement.id);
        let (drawn_parts, drawn_vertices) = draw_placement_render_mesh_view_mode(
            placement,
            mesh,
            double_sided,
            app.viewport_render_mode,
            &app.material_classes,
            txd_name,
            app.collision_generation_fallback_material,
        );
        placements += 1;
        parts += drawn_parts;
        vertices += drawn_vertices;
    }
    (placements, parts, vertices)
}

fn draw_lod_audit_wire_box(min: Vec3, max: Vec3, color: Color, width: f32) {
    let corners = [
        vec3(min.x, min.y, min.z),
        vec3(max.x, min.y, min.z),
        vec3(max.x, max.y, min.z),
        vec3(min.x, max.y, min.z),
        vec3(min.x, min.y, max.z),
        vec3(max.x, min.y, max.z),
        vec3(max.x, max.y, max.z),
        vec3(min.x, max.y, max.z),
    ];
    let edges = [
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
    unsafe {
        gl::LineWidth(width);
        gl::Color4f(color.r, color.g, color.b, color.a);
        gl::Begin(gl::LINES);
        for (a, b) in edges {
            gl::Vertex3f(corners[a].x, corners[a].y, corners[a].z);
            gl::Vertex3f(corners[b].x, corners[b].y, corners[b].z);
        }
        gl::End();
    }
}

fn draw_missing_texture_review_overlays(app: &AppState) {
    if app.active_tab != AppTab::TextureReview {
        return;
    }
    let Some(result) = app.missing_texture_review.result.as_ref() else {
        return;
    };
    let selected = app.missing_texture_review.selected_item;
    unsafe {
        gl::PushAttrib(gl::ALL_ATTRIB_BITS);
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::ALPHA_TEST);
        gl::Disable(gl::CULL_FACE);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::DepthMask(gl::FALSE);
        for item_index in filtered_missing_texture_review_indices(app)
            .into_iter()
            .filter(|index| Some(*index) != selected)
            .take(512)
        {
            let item = &result.items[item_index];
            if !is_live_element(app, item.placement_index) {
                continue;
            }
            let color = if item.is_lod {
                Color::new(1.0, 0.64, 0.12, 0.72)
            } else {
                Color::new(1.0, 0.20, 0.14, 0.68)
            };
            draw_lod_audit_wire_box(to_mq(item.min), to_mq(item.max), color, 2.25);
        }
        if let Some(item) = selected.and_then(|index| result.items.get(index))
            && is_live_element(app, item.placement_index)
        {
            draw_lod_audit_wire_box(
                to_mq(item.min),
                to_mq(item.max),
                Color::new(1.0, 0.92, 0.20, 1.0),
                4.0,
            );
        }
        gl::DepthMask(gl::TRUE);
        gl::LineWidth(1.0);
        gl::PopAttrib();
    }
}

fn draw_lod_audit_overlays(app: &AppState, result: &LodAuditResult, visible_indices: &[usize]) {
    unsafe {
        gl::PushAttrib(gl::ALL_ATTRIB_BITS);
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::ALPHA_TEST);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::DepthMask(gl::FALSE);
        let selected_dense_members = app
            .lod_audit
            .selected_issue
            .and_then(|index| result.issues.get(index))
            .filter(|issue| issue.kind == LodAuditIssueKind::Dense)
            .map(|issue| {
                issue
                    .placement_indices
                    .iter()
                    .copied()
                    .collect::<HashSet<_>>()
            });
        for index in visible_indices {
            if app.lod_audit.filter == LodAuditFilter::Dense
                || selected_dense_members
                    .as_ref()
                    .is_some_and(|members| members.contains(index))
            {
                continue;
            }
            let Some(info) = result.placements.get(*index) else {
                continue;
            };
            let (color, width) = match &info.role {
                LodAuditRole::LodTarget { .. } => (Color::new(1.0, 0.72, 0.12, 0.78), 2.0),
                LodAuditRole::Child { target } => {
                    if let Some(target_info) = result.placements.get(*target) {
                        let child_center = to_mq(info.center);
                        let target_center = to_mq(target_info.center);
                        gl::LineWidth(1.0);
                        gl::Color4f(0.18, 0.78, 1.0, 0.30);
                        gl::Begin(gl::LINES);
                        gl::Vertex3f(child_center.x, child_center.y, child_center.z);
                        gl::Vertex3f(target_center.x, target_center.y, target_center.z);
                        gl::End();
                    }
                    (Color::new(0.18, 0.78, 1.0, 0.46), 1.2)
                }
                LodAuditRole::SelfLod => (Color::new(0.18, 0.78, 1.0, 0.46), 1.2),
                LodAuditRole::Detail => continue,
            };
            draw_lod_audit_wire_box(to_mq(info.min), to_mq(info.max), color, width);
        }

        let selected_issue = app.lod_audit.selected_issue;
        for issue_index in filtered_lod_audit_issue_indices(app) {
            let Some(issue) = result.issues.get(issue_index) else {
                continue;
            };
            let selected = selected_issue == Some(issue_index);
            let mut color = issue.kind.color();
            color.a = if selected { 1.0 } else { 0.68 };
            if issue.kind == LodAuditIssueKind::Dense {
                let padding = if selected { 28.0 } else { 16.0 };
                let min = to_mq(issue.min) - Vec3::splat(padding);
                let max = to_mq(issue.max) + Vec3::splat(padding);
                draw_lod_audit_wire_box(min, max, color, if selected { 4.0 } else { 2.5 });
                continue;
            }
            let p = to_mq(issue.position);
            let size = if selected { 34.0 } else { 18.0 };
            gl::LineWidth(if selected { 3.0 } else { 1.5 });
            gl::Color4f(color.r, color.g, color.b, color.a);
            gl::Begin(gl::LINES);
            gl::Vertex3f(p.x - size, p.y, p.z);
            gl::Vertex3f(p.x + size, p.y, p.z);
            gl::Vertex3f(p.x, p.y - size, p.z);
            gl::Vertex3f(p.x, p.y + size, p.z);
            gl::Vertex3f(p.x, p.y, p.z - size);
            gl::Vertex3f(p.x, p.y, p.z + size);
            gl::End();
        }
        gl::PopAttrib();
    }
}

fn draw_lod_audit_scene(
    app: &AppState,
    frustum: &[Plane; 6],
    camera: Vec3,
    part_budget: usize,
    vertex_budget: usize,
) -> (usize, usize, usize) {
    let Some(result) = app.lod_audit.result.as_ref() else {
        return (0, 0, 0);
    };
    let mut visible = Vec::<(f32, usize)>::new();
    for (index, placement) in app.placements.iter().enumerate() {
        if !is_visible_element(app, index)
            || !lod_audit_placement_visible(app, result, index, camera)
        {
            continue;
        }
        let Some(info) = result.placements.get(index) else {
            continue;
        };
        let min = to_mq(info.min);
        let max = to_mq(info.max);
        if !aabb_in_frustum(frustum, min, max) {
            continue;
        }
        let distance = (to_mq(info.center) - camera).length();
        if element_mesh(app, placement).is_some() {
            visible.push((distance, index));
        }
    }
    visible.sort_by(|a, b| a.0.total_cmp(&b.0));

    let ambient_lift = scene_ambient_lift_from_timecyc(&app.timecyc);
    let mut placements = 0usize;
    let mut parts = 0usize;
    let mut vertices = 0usize;
    let mut visible_indices = Vec::new();
    for (_, index) in visible {
        let placement = &app.placements[index];
        let Some(mesh) = element_mesh(app, placement) else {
            continue;
        };
        let mesh_parts = mesh.parts.len();
        let mesh_vertices = mesh.parts.iter().map(|part| part.vertices).sum::<usize>();
        if parts + mesh_parts > part_budget || vertices + mesh_vertices > vertex_budget {
            continue;
        }
        let double_sided = placement_disable_backface_culling(placement, &app.definitions);
        let (drawn_parts, drawn_vertices) =
            draw_placement_render_mesh(placement, mesh, double_sided, ambient_lift);
        placements += 1;
        parts += drawn_parts;
        vertices += drawn_vertices;
        visible_indices.push(index);
    }
    draw_lod_audit_overlays(app, result, &visible_indices);
    (placements, parts, vertices)
}

fn draw_render_part_immediate(part: &RenderPart, alpha: f32, ambient_lift: V3) {
    unsafe {
        let transparency = if alpha < 0.999 {
            TransparencyMode::Blend
        } else {
            part.transparency
        };
        set_transparency_state(transparency);
        if part.texture != 0 {
            gl::Enable(gl::TEXTURE_2D);
            gl::BindTexture(gl::TEXTURE_2D, part.texture);
        } else {
            gl::Disable(gl::TEXTURE_2D);
        }
        gl::Begin(gl::TRIANGLES);
        for vertex in &part.cpu_vertices {
            let color = display_vertex_color(
                vertex.color,
                ambient_lift,
                part.material_color,
                part.material_ambient,
            );
            gl::Color4f(color.x, color.y, color.z, alpha * part.alpha * vertex.alpha);
            gl::Normal3f(vertex.normal.x, vertex.normal.y, vertex.normal.z);
            gl::TexCoord2f(vertex.uv.u, vertex.uv.v);
            gl::Vertex3f(vertex.pos.x, vertex.pos.y, vertex.pos.z);
        }
        gl::End();
    }
}

pub(crate) fn draw_simulation_objects(app: &mut AppState, frustum: &[Plane; 6]) {
    let objects: Vec<(SimObjectKind, Vec3, f32)> = app
        .sim
        .objects
        .iter()
        .map(|object| (object.kind, object.pos, object.rot_z))
        .collect();
    unsafe {
        gl::PushAttrib(gl::ENABLE_BIT | gl::CURRENT_BIT | gl::DEPTH_BUFFER_BIT);
        gl::Enable(gl::DEPTH_TEST);
        gl::Enable(gl::NORMALIZE);
    }
    for (kind, pos, rot_z) in objects {
        let Some(mesh) = sim_object_mesh(app, kind) else {
            continue;
        };
        let visual_scale = match kind {
            SimObjectKind::Player => SIM_PLAYER_VISUAL_SCALE,
            SimObjectKind::Vehicle => SIM_VEHICLE_VISUAL_SCALE,
        };
        let visual_rot = match kind {
            SimObjectKind::Player => rot_z + SIM_PLAYER_VISUAL_ROT_OFFSET,
            SimObjectKind::Vehicle => rot_z,
        };
        let upright_rot = match kind {
            SimObjectKind::Player => Mat4::from_rotation_x(SIM_PLAYER_VISUAL_UPRIGHT_ROT),
            SimObjectKind::Vehicle => Mat4::IDENTITY,
        };
        let world_min = pos + mesh.bounds.min * visual_scale;
        let world_max = pos + mesh.bounds.max * visual_scale;
        if !aabb_in_frustum(frustum, world_min, world_max) {
            continue;
        }
        let model = Mat4::from_translation(pos)
            * Mat4::from_rotation_z(visual_rot)
            * upright_rot
            * Mat4::from_scale(Vec3::splat(visual_scale));
        let mut vertices = 0usize;
        unsafe {
            gl::PushMatrix();
            gl::MultMatrixf(model.to_cols_array().as_ptr());
            for part in &mesh.parts {
                set_transparency_state(part.transparency);
                if part.use_lighting {
                    gl::Enable(gl::LIGHTING);
                } else {
                    gl::Disable(gl::LIGHTING);
                }
                draw_render_part_buffer(part);
                vertices += part.vertices;
            }
            gl::PopMatrix();
        }
        let parts = mesh.parts.len();
        app.last_drawn_placements += 1;
        app.last_drawn_parts += parts;
        app.last_drawn_vertices += vertices;
    }
    unsafe {
        gl::PopAttrib();
    }
}

pub(crate) fn draw_selected_local_lod(
    app: &AppState,
    frustum: &[Plane; 6],
    lod_near: f32,
) -> (usize, usize, usize) {
    if !app.show_selected_lod_local || app.options.lod_mode == LodMode::ShowAll {
        return (0, 0, 0);
    }
    let Some(placement) = app.placements.get(app.selected) else {
        return (0, 0, 0);
    };
    if app
        .element_states
        .get(app.selected)
        .is_some_and(|state| state.deleted || state.hidden)
        || !placement_is_app_lod(app, placement)
    {
        return (0, 0, 0);
    }
    let Some(mesh) = element_mesh(app, placement) else {
        return (0, 0, 0);
    };
    let model_cols = placement_matrix(placement).to_cols_array();
    let bounds = transformed_bounds(mesh.bounds, &model_cols);
    let center = (bounds.min + bounds.max) * 0.5;
    let radius = (bounds.max - center).length() + 64.0;
    let distance = (center - app.camera.pos).length();
    if distance + radius >= lod_near {
        return (0, 0, 0);
    }
    if !aabb_in_frustum(frustum, bounds.min, bounds.max) {
        return (0, 0, 0);
    }
    let double_sided = placement_disable_backface_culling(placement, &app.definitions);
    let ambient_lift = scene_ambient_lift_from_timecyc(&app.timecyc);
    let (parts, vertices) = draw_placement_render_mesh(placement, mesh, double_sided, ambient_lift);
    (1, parts, vertices)
}

pub(crate) fn build_scene_cells(
    placements: &[Placement],
    definitions: &HashMap<String, Definition>,
    meshes: &HashMap<String, RenderMesh>,
    lod_ids: &std::collections::HashSet<String>,
    want_lod: bool,
    ambient_lift: V3,
) -> Vec<SceneCell> {
    const CELL_SIZE: f32 = 384.0;
    // Bucket placements by cell first. A display list must be compiled
    // start-to-finish before another is opened (only one gl::NewList may be
    // open at a time), so we cannot interleave NewList calls across cells.
    let mut buckets = HashMap::<(i32, i32), Vec<usize>>::new();
    for (i, p) in placements.iter().enumerate() {
        if placement_is_lod(p, lod_ids) != want_lod {
            continue;
        }
        let key = placement_mesh_key(p, definitions);
        if !meshes.contains_key(&key) {
            continue;
        }
        let pos = to_mq(p.pos);
        let cell_key = (
            (pos.x / CELL_SIZE).floor() as i32,
            (pos.y / CELL_SIZE).floor() as i32,
        );
        buckets.entry(cell_key).or_default().push(i);
    }

    let mut cells = Vec::with_capacity(buckets.len());
    for (_, members) in buckets {
        if members.is_empty() {
            continue;
        }
        let list = unsafe { gl::GenLists(1) };
        if list == 0 {
            continue;
        }
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);
        let mut placements_n = 0usize;
        let mut parts = 0usize;
        let mut vertices = 0usize;
        unsafe { gl::NewList(list, gl::COMPILE) };
        for &i in &members {
            let p = &placements[i];
            let key = placement_mesh_key(p, definitions);
            let Some(mesh) = meshes.get(&key) else {
                continue;
            };
            placements_n += 1;
            let model = placement_matrix(p);
            let model_cols = model.to_cols_array();
            let bounds = transformed_bounds(mesh.bounds, &model_cols);
            min = min.min(bounds.min);
            max = max.max(bounds.max);
            let double_sided = placement_disable_backface_culling(p, definitions);
            let alpha = placement_alpha(p);
            unsafe {
                if double_sided {
                    gl::Disable(gl::CULL_FACE);
                    gl::LightModeli(gl::LIGHT_MODEL_TWO_SIDE, gl::TRUE as i32);
                }
                gl::PushMatrix();
                gl::MultMatrixf(model_cols.as_ptr());
                for part in &mesh.parts {
                    if part.use_lighting {
                        gl::Enable(gl::LIGHTING);
                    } else {
                        gl::Disable(gl::LIGHTING);
                    }
                    if alpha < 0.999 {
                        draw_render_part_immediate(part, alpha, ambient_lift);
                    } else {
                        set_transparency_state(part.transparency);
                        gl::CallList(part.list);
                    }
                    parts += 1;
                    vertices += part.vertices;
                }
                gl::PopMatrix();
                gl::Disable(gl::BLEND);
                gl::Enable(gl::ALPHA_TEST);
                gl::DepthMask(gl::TRUE);
                if double_sided {
                    gl::LightModeli(gl::LIGHT_MODEL_TWO_SIDE, gl::FALSE as i32);
                    gl::Enable(gl::CULL_FACE);
                }
            }
        }
        unsafe { gl::EndList() };
        if placements_n == 0 {
            continue;
        }
        let center = (min + max) * 0.5;
        let radius = (max - center).length() + 64.0;
        cells.push(SceneCell {
            list,
            min,
            max,
            center,
            radius,
            placements: placements_n,
            parts,
            vertices,
        });
    }
    cells.sort_by(|a, b| b.vertices.cmp(&a.vertices));
    cells
}

pub(crate) fn build_world_cells(
    placements: &[Placement],
    definitions: &HashMap<String, Definition>,
    meshes: &HashMap<String, RenderMesh>,
    keep_cpu_data: bool,
    lod_ids: &std::collections::HashSet<String>,
    want_lod: bool,
    ambient_lift: V3,
) -> Vec<WorldCell> {
    // Keep neighboring placements with the same render state in fewer VBOs
    // without making frustum culling as coarse as the larger map chunks.
    const CELL_SIZE: f32 = 512.0;
    let mut cells = HashMap::<(i32, i32), WorldCellBuild>::new();
    for p in placements {
        if placement_is_lod(p, lod_ids) != want_lod {
            continue;
        }
        let key = placement_mesh_key(p, definitions);
        let Some(mesh) = meshes.get(&key) else {
            continue;
        };
        let origin = to_mq(p.pos);
        let cell_key = (
            (origin.x / CELL_SIZE).floor() as i32,
            (origin.y / CELL_SIZE).floor() as i32,
        );
        let model = placement_matrix(p);
        let double_sided = placement_disable_backface_culling(p, definitions);
        let alpha = placement_alpha(p);
        let alpha_u8 = (alpha * 255.0).round().clamp(0.0, 255.0) as u8;
        let cell = cells.entry(cell_key).or_insert_with(|| WorldCellBuild {
            min: Vec3::splat(f32::MAX),
            max: Vec3::splat(f32::MIN),
            placements: 0,
            parts: 0,
            vertices: 0,
            batch_data: HashMap::new(),
            blend_data: Vec::new(),
        });
        cell.placements += 1;
        let model_cols = model.to_cols_array();
        let bounds = transformed_bounds(mesh.bounds, &model_cols);
        cell.min = cell.min.min(bounds.min);
        cell.max = cell.max.max(bounds.max);
        for part in &mesh.parts {
            cell.parts += 1;
            let transparency = if alpha_u8 < 255 {
                TransparencyMode::Blend
            } else {
                part.transparency
            };
            // Blend parts have an opaque-texel depth/color pass plus a sorted
            // translucent color pass. Count both submissions against the
            // vertex budget so alpha-heavy maps cannot silently do almost
            // twice the configured GPU work.
            let pass_count = if transparency == TransparencyMode::Blend {
                2
            } else {
                1
            };
            cell.vertices += part.cpu_vertices.len() * pass_count;
            let key = (part.texture, part.use_lighting, double_sided, transparency);
            let mut part_data = Vec::with_capacity(part.cpu_vertices.len() * 12);
            let mut part_min = Vec3::splat(f32::MAX);
            let mut part_max = Vec3::splat(f32::MIN);
            for v in &part.cpu_vertices {
                let pos = transform_point_gl(&model_cols, v.pos);
                part_min = part_min.min(pos);
                part_max = part_max.max(pos);
                let normal = transform_normal_gl(&model_cols, v.normal);
                let display_color = display_vertex_color(
                    v.color,
                    ambient_lift,
                    part.material_color,
                    part.material_ambient,
                );
                let vertex_alpha = alpha * part.alpha * v.alpha;
                part_data.extend_from_slice(&[
                    v.uv.u,
                    v.uv.v,
                    normal.x,
                    normal.y,
                    normal.z,
                    display_color.x,
                    display_color.y,
                    display_color.z,
                    vertex_alpha,
                    pos.x,
                    pos.y,
                    pos.z,
                ]);
            }
            if transparency == TransparencyMode::Blend {
                // Do not merge alpha parts by texture. Their individual centers
                // are required for stable back-to-front ordering as the camera
                // moves around the DFF.
                let center = if part_data.is_empty() {
                    origin
                } else {
                    (part_min + part_max) * 0.5
                };
                cell.blend_data.push((key, part_data, center));
            } else {
                cell.batch_data.entry(key).or_default().extend(part_data);
            }
        }
    }

    let mut out = Vec::with_capacity(cells.len());
    for (_, cell) in cells {
        let center = (cell.min + cell.max) * 0.5;
        let mut grouped_data = cell
            .batch_data
            .into_iter()
            .map(|(key, data)| (key, (data, Vec::new())))
            .collect::<HashMap<_, _>>();
        for (key, data, segment_center) in cell.blend_data {
            let (group_data, segments) = grouped_data
                .entry(key)
                .or_insert_with(|| (Vec::new(), Vec::new()));
            let first_vertex = group_data.len() / 12;
            let vertices = data.len() / 12;
            group_data.extend(data);
            segments.push(WorldBlendSegment {
                center: segment_center,
                first_vertex,
                vertices,
            });
        }
        let mut groups = grouped_data
            .into_iter()
            .filter(|(_, (data, _))| !data.is_empty())
            .collect::<Vec<_>>();
        groups.sort_by_key(|(key, _)| {
            let (texture, use_lighting, double_sided, transparency) = *key;
            (
                transparency == TransparencyMode::Blend,
                double_sided,
                use_lighting,
                texture,
            )
        });
        let combined_len = groups.iter().map(|(_, (data, _))| data.len()).sum();
        let mut combined_data = Vec::with_capacity(combined_len);
        let mut batches = Vec::with_capacity(groups.len());
        for ((texture, use_lighting, double_sided, transparency), (data, blend_segments)) in groups
        {
            let first_vertex = combined_data.len() / 12;
            let vertices = data.len() / 12;
            combined_data.extend_from_slice(&data);
            batches.push(WorldBatch {
                texture,
                use_lighting,
                double_sided,
                transparency,
                first_vertex,
                vertices,
                data: if keep_cpu_data { data } else { Vec::new() },
                blend_segments,
            });
        }
        let mut vbo = 0u32;
        unsafe {
            gl::GenBuffers(1, &mut vbo);
            if vbo != 0 {
                gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
                gl::BufferData(
                    gl::ARRAY_BUFFER,
                    (combined_data.len() * std::mem::size_of::<f32>()) as isize,
                    combined_data.as_ptr().cast(),
                    gl::STATIC_DRAW,
                );
                gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            }
        }
        if vbo == 0 {
            continue;
        }
        let radius = (cell.max - center).length() + 64.0;
        out.push(WorldCell {
            min: cell.min,
            max: cell.max,
            center,
            radius,
            placements: cell.placements,
            parts: cell.parts,
            vertices: cell.vertices,
            vbo,
            batches,
        });
    }
    out.sort_by(|a, b| b.vertices.cmp(&a.vertices));
    out
}

pub(crate) fn collision_material_color(material: u8, alpha: f32) -> [f32; 4] {
    let (r, g, b) = match material {
        1..=5 | 85..=86 | 144 | 178 => (0.30, 0.32, 0.35), // tarmac, pavement, roads, rails
        6 | 21 | 24..=27 | 88 | 123..=124 | 139..=142 => (0.43, 0.32, 0.22), // gravel, mud, dirt, rubbish
        7 | 34 | 89 | 101 | 134..=138 | 165 => (0.48, 0.50, 0.49), // concrete/rubble/building site
        8 | 18 | 35..=37 | 109 | 131 | 154 | 161 => (0.45, 0.42, 0.36), // rock/stone/cliff
        9..=17
        | 19..=20
        | 23
        | 40..=41
        | 80..=84
        | 87
        | 110..=122
        | 125
        | 128..=130
        | 132..=133
        | 143
        | 145..=153
        | 157
        | 160 => (0.23, 0.55, 0.22), // grass/vegetation/forest
        28..=33 | 74..=79 | 176 => (0.74, 0.64, 0.38),             // sand/hay
        38..=39 | 96..=100 | 126 | 155..=156 => (0.16, 0.48, 0.58), // water/riverbed/underwater
        42..=44 | 70 | 72..=73 | 172..=174 => (0.52, 0.35, 0.18),  // wood
        45..=47 | 66 | 69 | 175 => (0.38, 0.78, 0.92),             // glass/transparent
        50..=58 | 60 | 164 | 167..=168 | 171 => (0.45, 0.53, 0.60), // metal/doors/gates/scaffold
        61 | 90..=95 | 102..=108 | 158 => (0.62, 0.46, 0.33),      // interiors/furniture/doors
        62 => (0.90, 0.62, 0.46),                                  // ped
        63..=65 => (0.34, 0.42, 0.75),                             // vehicles
        67 => (0.05, 0.05, 0.05),                                  // rubber
        68 | 159 | 169..=170 => (0.86, 0.48, 0.18),                // plastic/barriers/cones
        71 | 163 => (0.50, 0.22, 0.44),                            // carpet
        127 => (0.36, 0.68, 0.76),                                 // poolside
        166 => (0.09, 0.10, 0.11),                                 // bin bag
        177 => (0.72, 0.05, 0.05),                                 // gore
        _ => (0.56, 0.56, 0.56),
    };
    [r, g, b, alpha]
}

pub(crate) fn collision_shaded_color(color: [f32; 4], normal: Vec3) -> [f32; 4] {
    let light_dir = vec3(-0.35, -0.55, 0.76).normalize();
    let normal = normal.normalize_or_zero();
    let diffuse = normal.dot(light_dir).abs();
    let upward = normal.z.abs();
    let shade = (0.42 + diffuse * 0.48 + upward * 0.10).clamp(0.35, 1.08);
    [
        (color[0] * shade).clamp(0.0, 1.0),
        (color[1] * shade).clamp(0.0, 1.0),
        (color[2] * shade).clamp(0.0, 1.0),
        color[3],
    ]
}

fn vertex_marker_alpha(camera_pos: Vec3, point: Vec3) -> f32 {
    let distance = camera_pos.distance(point);
    let near = 25.0;
    let far = 100.0;
    if distance >= far {
        0.0
    } else {
        let t = ((far - distance) / (far - near)).clamp(0.0, 1.0);
        t * t * 0.85
    }
}

fn vertex_marker_size(alpha: f32) -> f32 {
    1.0 + alpha * 3.0
}

fn draw_nearby_points(points: impl Iterator<Item = Vec3>, camera_pos: Vec3, color: [f32; 3]) {
    unsafe {
        gl::Enable(gl::DEPTH_TEST);
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        for point in points {
            let alpha = vertex_marker_alpha(camera_pos, point);
            if alpha <= 0.0 {
                continue;
            }
            gl::PointSize(vertex_marker_size(alpha));
            gl::Color4f(color[0], color[1], color[2], alpha);
            gl::Begin(gl::POINTS);
            gl::Vertex3f(point.x, point.y, point.z);
            gl::End();
        }
        gl::Disable(gl::BLEND);
    }
}

fn draw_nearby_raw_vertices(raw: &RawMesh, camera_pos: Vec3) {
    draw_nearby_points(
        raw.vertices.iter().copied().map(to_mq),
        camera_pos,
        [0.85, 0.94, 1.0],
    );
}

fn draw_nearby_collision_vertices(mesh: &CollisionMesh, model_cols: &[f32; 16], camera_pos: Vec3) {
    draw_nearby_points(
        mesh.vertices
            .iter()
            .copied()
            .map(|vertex| transform_point_gl(model_cols, vertex)),
        camera_pos,
        [0.85, 0.94, 1.0],
    );
}

fn draw_collision_sphere_primitive(sphere: &CollisionSphere) {
    let c = to_mq(sphere.center);
    let r = sphere.radius.abs().max(0.01);
    let base = collision_material_color(sphere.surface.material, 0.72);
    unsafe {
        gl::Begin(gl::TRIANGLES);
        let rings = 10;
        let segments = 20;
        for lat in 0..rings {
            let v0 = lat as f32 / rings as f32;
            let v1 = (lat + 1) as f32 / rings as f32;
            let phi0 = (v0 - 0.5) * std::f32::consts::PI;
            let phi1 = (v1 - 0.5) * std::f32::consts::PI;
            for lon in 0..segments {
                let u0 = lon as f32 / segments as f32;
                let u1 = (lon + 1) as f32 / segments as f32;
                let theta0 = u0 * std::f32::consts::TAU;
                let theta1 = u1 * std::f32::consts::TAU;
                let n00 = vec3(
                    phi0.cos() * theta0.cos(),
                    phi0.cos() * theta0.sin(),
                    phi0.sin(),
                );
                let n01 = vec3(
                    phi0.cos() * theta1.cos(),
                    phi0.cos() * theta1.sin(),
                    phi0.sin(),
                );
                let n10 = vec3(
                    phi1.cos() * theta0.cos(),
                    phi1.cos() * theta0.sin(),
                    phi1.sin(),
                );
                let n11 = vec3(
                    phi1.cos() * theta1.cos(),
                    phi1.cos() * theta1.sin(),
                    phi1.sin(),
                );
                for n in [n00, n10, n11, n00, n11, n01] {
                    let color = collision_shaded_color(base, n);
                    gl::Color4f(color[0], color[1], color[2], color[3]);
                    let p = c + n * r;
                    gl::Vertex3f(p.x, p.y, p.z);
                }
            }
        }
        gl::End();

        gl::Color4f(0.03, 0.03, 0.03, 0.55);
        gl::LineWidth(1.0);
        gl::Begin(gl::LINES);
        for i in 0..32 {
            let a = i as f32 / 32.0 * std::f32::consts::TAU;
            let b = (i + 1) as f32 / 32.0 * std::f32::consts::TAU;
            for axis in 0..3 {
                let (pa, pb) = match axis {
                    0 => (
                        vec3(c.x, c.y + a.cos() * r, c.z + a.sin() * r),
                        vec3(c.x, c.y + b.cos() * r, c.z + b.sin() * r),
                    ),
                    1 => (
                        vec3(c.x + a.cos() * r, c.y, c.z + a.sin() * r),
                        vec3(c.x + b.cos() * r, c.y, c.z + b.sin() * r),
                    ),
                    _ => (
                        vec3(c.x + a.cos() * r, c.y + a.sin() * r, c.z),
                        vec3(c.x + b.cos() * r, c.y + b.sin() * r, c.z),
                    ),
                };
                gl::Vertex3f(pa.x, pa.y, pa.z);
                gl::Vertex3f(pb.x, pb.y, pb.z);
            }
        }
        gl::End();
    }
}

fn draw_collision_box_primitive(col_box: &CollisionBox) {
    let min = to_mq(V3 {
        x: col_box.min.x.min(col_box.max.x),
        y: col_box.min.y.min(col_box.max.y),
        z: col_box.min.z.min(col_box.max.z),
    });
    let max = to_mq(V3 {
        x: col_box.min.x.max(col_box.max.x),
        y: col_box.min.y.max(col_box.max.y),
        z: col_box.min.z.max(col_box.max.z),
    });
    let p = [
        vec3(min.x, min.y, min.z),
        vec3(max.x, min.y, min.z),
        vec3(max.x, max.y, min.z),
        vec3(min.x, max.y, min.z),
        vec3(min.x, min.y, max.z),
        vec3(max.x, min.y, max.z),
        vec3(max.x, max.y, max.z),
        vec3(min.x, max.y, max.z),
    ];
    let base = collision_material_color(col_box.surface.material, 0.72);
    unsafe {
        gl::Begin(gl::TRIANGLES);
        for (indices, normal) in [
            ([0, 2, 1, 0, 3, 2], vec3(0.0, 0.0, -1.0)),
            ([4, 5, 6, 4, 6, 7], vec3(0.0, 0.0, 1.0)),
            ([0, 1, 5, 0, 5, 4], vec3(0.0, -1.0, 0.0)),
            ([1, 2, 6, 1, 6, 5], vec3(1.0, 0.0, 0.0)),
            ([2, 3, 7, 2, 7, 6], vec3(0.0, 1.0, 0.0)),
            ([3, 0, 4, 3, 4, 7], vec3(-1.0, 0.0, 0.0)),
        ] {
            let color = collision_shaded_color(base, normal);
            gl::Color4f(color[0], color[1], color[2], color[3]);
            for idx in indices {
                gl::Vertex3f(p[idx].x, p[idx].y, p[idx].z);
            }
        }
        gl::End();

        gl::Color4f(0.03, 0.03, 0.03, 0.58);
        gl::LineWidth(1.0);
        gl::Begin(gl::LINES);
        for (a, b) in [
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
        ] {
            gl::Vertex3f(p[a].x, p[a].y, p[a].z);
            gl::Vertex3f(p[b].x, p[b].y, p[b].z);
        }
        gl::End();
    }
}

// Highlight a single face of a COL box (used for the Editing-tab face-resize handles).
// axis is 0=x/1=y/2=z in macroquad space; active = currently being dragged.
pub(crate) fn draw_col_box_face_highlight(
    col_box: &CollisionBox,
    axis: usize,
    side_is_max: bool,
    active: bool,
) {
    let a = to_mq(col_box.min);
    let b = to_mq(col_box.max);
    let lo = vec3(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z));
    let hi = vec3(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z));
    draw_oriented_col_box_face_highlight(
        (lo + hi) * 0.5,
        (hi - lo) * 0.5,
        V3::default(),
        axis,
        side_is_max,
        active,
    );
}

pub(crate) fn draw_col_cuboid_face_highlight(
    cuboid: &CollisionCuboid,
    axis: usize,
    side_is_max: bool,
    active: bool,
) {
    draw_oriented_col_box_face_highlight(
        to_mq(cuboid.center),
        to_mq(cuboid.half_extents),
        cuboid.rotation,
        axis,
        side_is_max,
        active,
    );
}

fn draw_oriented_col_box_face_highlight(
    center: Vec3,
    half_extents: Vec3,
    rotation: V3,
    axis: usize,
    side_is_max: bool,
    active: bool,
) {
    let coord = if side_is_max {
        half_extents[axis]
    } else {
        -half_extents[axis]
    };
    let (u, v) = match axis {
        0 => (1usize, 2usize),
        1 => (0usize, 2usize),
        _ => (0usize, 1usize),
    };
    let rotation = Mat4::from_rotation_z(rotation.z.to_radians())
        * Mat4::from_rotation_y(rotation.y.to_radians())
        * Mat4::from_rotation_x(rotation.x.to_radians());
    let corner = |su: bool, sv: bool| {
        let mut p = Vec3::ZERO;
        p[axis] = coord;
        p[u] = if su {
            half_extents[u]
        } else {
            -half_extents[u]
        };
        p[v] = if sv {
            half_extents[v]
        } else {
            -half_extents[v]
        };
        center + rotation.transform_vector3(p)
    };
    let c0 = corner(false, false);
    let c1 = corner(true, false);
    let c2 = corner(true, true);
    let c3 = corner(false, true);
    let (fill, line_a) = if active {
        ([1.0f32, 0.55, 0.10, 0.45], 1.0f32)
    } else {
        ([1.0f32, 0.82, 0.18, 0.24], 0.85f32)
    };
    unsafe {
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::CULL_FACE);
        gl::Enable(gl::DEPTH_TEST);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::DepthMask(gl::FALSE);
        gl::Begin(gl::TRIANGLES);
        gl::Color4f(fill[0], fill[1], fill[2], fill[3]);
        for p in [c0, c1, c2, c0, c2, c3] {
            gl::Vertex3f(p.x, p.y, p.z);
        }
        gl::End();
        gl::DepthMask(gl::TRUE);
        gl::LineWidth(3.0);
        gl::Color4f(1.0, 0.85, 0.12, line_a);
        gl::Begin(gl::LINE_LOOP);
        for p in [c0, c1, c2, c3] {
            gl::Vertex3f(p.x, p.y, p.z);
        }
        gl::End();
        gl::Disable(gl::BLEND);
    }
}

pub(crate) fn draw_collision_primitives(mesh: &CollisionMesh) {
    unsafe {
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::CULL_FACE);
        gl::Enable(gl::DEPTH_TEST);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::DepthMask(gl::TRUE);
    }
    for sphere in &mesh.spheres {
        draw_collision_sphere_primitive(sphere);
    }
    for col_box in &mesh.boxes {
        draw_collision_box_primitive(col_box);
    }
    unsafe {
        gl::Enable(gl::DEPTH_TEST);
        gl::Disable(gl::BLEND);
    }
}

pub(crate) fn draw_selected_collision_face(
    mesh: &CollisionMesh,
    selected_face: Option<usize>,
    selected_vertex: Option<usize>,
    selected_vertices: &BTreeSet<usize>,
    camera_pos: Vec3,
    model_cols: &[f32; 16],
    show_vertices: bool,
    show_face_outline: bool,
) {
    let Some(selected_face) = selected_face else {
        return;
    };
    let Some(face) = mesh.faces.get(selected_face) else {
        return;
    };
    let Some((a, b, c)) = collision_face_points(mesh, face) else {
        return;
    };
    unsafe {
        gl::Enable(gl::DEPTH_TEST);
        if show_face_outline {
            gl::LineWidth(4.0);
            gl::Color4f(1.0, 0.88, 0.12, 1.0);
            gl::Begin(gl::LINE_LOOP);
            gl::Vertex3f(a.x, a.y, a.z);
            gl::Vertex3f(b.x, b.y, b.z);
            gl::Vertex3f(c.x, c.y, c.z);
            gl::End();
        }
        if show_vertices {
            gl::PointSize(4.0);
            gl::Begin(gl::POINTS);
            for (slot, point) in [a, b, c].into_iter().enumerate() {
                if selected_vertex == Some(slot) {
                    gl::Color4f(0.2, 0.85, 1.0, 1.0);
                } else {
                    gl::Color4f(1.0, 1.0, 1.0, 0.95);
                }
                gl::Vertex3f(point.x, point.y, point.z);
            }
            gl::End();
            if !selected_vertices.is_empty() {
                gl::PointSize(6.0);
                gl::Begin(gl::POINTS);
                for vertex_idx in selected_vertices {
                    if let Some(point) = mesh.vertices.get(*vertex_idx).map(|vertex| to_mq(*vertex))
                    {
                        let world_point = transform_point_gl(model_cols, from_mq(point));
                        let alpha = vertex_marker_alpha(camera_pos, world_point).max(0.72);
                        gl::Color4f(1.0, 0.18, 0.92, alpha);
                        gl::Vertex3f(point.x, point.y, point.z);
                    }
                }
                gl::End();
                gl::PointSize(3.0);
                gl::Begin(gl::POINTS);
                for vertex_idx in selected_vertices {
                    if let Some(point) = mesh.vertices.get(*vertex_idx).map(|vertex| to_mq(*vertex))
                    {
                        gl::Color4f(1.0, 1.0, 1.0, 0.98);
                        gl::Vertex3f(point.x, point.y, point.z);
                    }
                }
                gl::End();
            }
        }
    }
}

pub(crate) fn draw_hovered_collision_vertex(
    mesh: &CollisionMesh,
    hovered_face: Option<usize>,
    hovered_vertex: Option<usize>,
    camera_pos: Vec3,
    model_cols: &[f32; 16],
) {
    let Some(face_idx) = hovered_face else {
        return;
    };
    let Some(face) = mesh.faces.get(face_idx) else {
        return;
    };
    let vertex_idx = collision_selected_vertex_index_from_face(face, hovered_vertex.unwrap_or(0));
    let Some(point) = mesh.vertices.get(vertex_idx).map(|vertex| to_mq(*vertex)) else {
        return;
    };
    unsafe {
        gl::Enable(gl::DEPTH_TEST);
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::PointSize(6.0);
        let world_point = transform_point_gl(model_cols, from_mq(point));
        let alpha = vertex_marker_alpha(camera_pos, world_point).max(0.55);
        gl::Color4f(0.15, 0.95, 1.0, alpha);
        gl::Begin(gl::POINTS);
        gl::Vertex3f(point.x, point.y, point.z);
        gl::End();
    }
}

fn draw_collision_mesh_edges(mesh: &CollisionMesh) {
    unsafe {
        gl::Enable(gl::DEPTH_TEST);
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::CULL_FACE);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::LineWidth(1.0);
        gl::Color4f(0.02, 0.03, 0.04, 0.62);
        gl::Begin(gl::LINES);
        for face in &mesh.faces {
            let Some((a, b, c)) = collision_face_points(mesh, face) else {
                continue;
            };
            for (p, q) in [(a, b), (b, c), (c, a)] {
                gl::Vertex3f(p.x, p.y, p.z);
                gl::Vertex3f(q.x, q.y, q.z);
            }
        }
        gl::End();
        gl::Disable(gl::BLEND);
        gl::Enable(gl::CULL_FACE);
    }
}

fn draw_col_vertex_incident_edges(
    mesh: &CollisionMesh,
    active_vertex: Option<usize>,
    selected_vertices: &BTreeSet<usize>,
) {
    let mut vertices = selected_vertices.clone();
    if let Some(vertex) = active_vertex {
        vertices.insert(vertex);
    }
    if vertices.is_empty() {
        return;
    }
    unsafe {
        gl::Enable(gl::DEPTH_TEST);
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::LineWidth(1.25);
        gl::Begin(gl::LINES);
        for face in &mesh.faces {
            for (a, b) in [
                (face.a as usize, face.b as usize),
                (face.b as usize, face.c as usize),
                (face.c as usize, face.a as usize),
            ] {
                let (Some(pa), Some(pb)) = (mesh.vertices.get(a), mesh.vertices.get(b)) else {
                    continue;
                };
                let pa = to_mq(*pa);
                let pb = to_mq(*pb);
                match (vertices.contains(&a), vertices.contains(&b)) {
                    (true, true) => {
                        gl::Color4f(1.0, 0.72, 0.10, 0.28);
                        gl::Vertex3f(pa.x, pa.y, pa.z);
                        gl::Vertex3f(pb.x, pb.y, pb.z);
                    }
                    (true, false) => {
                        gl::Color4f(1.0, 0.72, 0.10, 0.28);
                        gl::Vertex3f(pa.x, pa.y, pa.z);
                        gl::Color4f(1.0, 0.72, 0.10, 0.035);
                        gl::Vertex3f(pb.x, pb.y, pb.z);
                    }
                    (false, true) => {
                        gl::Color4f(1.0, 0.72, 0.10, 0.035);
                        gl::Vertex3f(pa.x, pa.y, pa.z);
                        gl::Color4f(1.0, 0.72, 0.10, 0.28);
                        gl::Vertex3f(pb.x, pb.y, pb.z);
                    }
                    (false, false) => {}
                }
            }
        }
        gl::End();
        gl::Disable(gl::BLEND);
    }
}

pub(crate) fn draw_collision_mesh(
    mesh: &CollisionMesh,
    selected_face: Option<usize>,
    selected_faces: &BTreeSet<usize>,
    selected_edges: &BTreeSet<(usize, usize)>,
    selected_vertex: Option<usize>,
    selected_vertices: &BTreeSet<usize>,
    hovered_face: Option<usize>,
    hovered_vertex: Option<usize>,
    model_cols: &[f32; 16],
    camera_pos: Vec3,
    draw_vertex_cloud: bool,
    show_edit_vertices: bool,
    show_face_selection: bool,
) {
    draw_collision_mesh_with_options(
        mesh,
        selected_face,
        selected_faces,
        selected_edges,
        selected_vertex,
        selected_vertices,
        hovered_face,
        hovered_vertex,
        model_cols,
        camera_pos,
        draw_vertex_cloud,
        show_edit_vertices,
        show_face_selection,
        true,
    );
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_collision_mesh_with_options(
    mesh: &CollisionMesh,
    selected_face: Option<usize>,
    selected_faces: &BTreeSet<usize>,
    selected_edges: &BTreeSet<(usize, usize)>,
    selected_vertex: Option<usize>,
    selected_vertices: &BTreeSet<usize>,
    hovered_face: Option<usize>,
    hovered_vertex: Option<usize>,
    model_cols: &[f32; 16],
    camera_pos: Vec3,
    draw_vertex_cloud: bool,
    show_edit_vertices: bool,
    show_face_selection: bool,
    show_primitives: bool,
) {
    if show_primitives {
        draw_collision_primitives(mesh);
    }
    unsafe {
        gl::Begin(gl::TRIANGLES);
        for (idx, face) in mesh.faces.iter().enumerate() {
            let Some((a, b, c)) = collision_face_points(mesh, face) else {
                continue;
            };
            let color = if show_face_selection
                && (selected_face == Some(idx) || selected_faces.contains(&idx))
            {
                [1.0, 0.78, 0.14, 1.0]
            } else {
                collision_material_color(face.material, 1.0)
            };
            let local_normal = (b - a).cross(c - a);
            let normal = transform_normal_gl(
                model_cols,
                V3 {
                    x: local_normal.x,
                    y: local_normal.y,
                    z: local_normal.z,
                },
            );
            let color = collision_shaded_color(color, normal);
            gl::Color4f(color[0], color[1], color[2], color[3]);
            gl::Vertex3f(a.x, a.y, a.z);
            gl::Vertex3f(b.x, b.y, b.z);
            gl::Vertex3f(c.x, c.y, c.z);
        }
        gl::End();
    }
    draw_collision_mesh_edges(mesh);
    if show_edit_vertices {
        let active_vertex = selected_face
            .and_then(|face_idx| mesh.faces.get(face_idx))
            .map(|face| {
                collision_selected_vertex_index_from_face(face, selected_vertex.unwrap_or(0))
            });
        draw_col_vertex_incident_edges(mesh, active_vertex, selected_vertices);
    }
    if !selected_edges.is_empty() {
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::Disable(gl::TEXTURE_2D);
            gl::Disable(gl::LIGHTING);
            gl::LineWidth(4.0);
            gl::Color4f(1.0, 0.62, 0.14, 1.0);
            gl::Begin(gl::LINES);
            for (a, b) in selected_edges {
                let (Some(a), Some(b)) = (mesh.vertices.get(*a), mesh.vertices.get(*b)) else {
                    continue;
                };
                let a = to_mq(*a);
                let b = to_mq(*b);
                gl::Vertex3f(a.x, a.y, a.z);
                gl::Vertex3f(b.x, b.y, b.z);
            }
            gl::End();
        }
    }
    if draw_vertex_cloud {
        draw_nearby_collision_vertices(mesh, model_cols, camera_pos);
    }
    draw_selected_collision_face(
        mesh,
        selected_face,
        selected_vertex,
        selected_vertices,
        camera_pos,
        model_cols,
        show_edit_vertices,
        show_face_selection,
    );
    if show_edit_vertices {
        draw_hovered_collision_vertex(mesh, hovered_face, hovered_vertex, camera_pos, model_cols);
    }
}

pub(crate) fn draw_collision_world(app: &mut AppState, frustum: &[Plane; 6]) {
    let far = app.options.draw_radius;
    unsafe {
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::ALPHA_TEST);
        gl::Disable(gl::BLEND);
        gl::DepthMask(gl::TRUE);
        gl::Disable(gl::CULL_FACE);
        for (idx, placement) in app.placements.iter().enumerate() {
            if app
                .element_states
                .get(idx)
                .is_some_and(|state| state.deleted || state.hidden)
            {
                continue;
            }
            let Some(mesh) = element_collision_mesh(app, placement) else {
                continue;
            };
            let origin = to_mq(placement.pos);
            let radius = (mesh.bounds.max - mesh.bounds.min).length().max(32.0);
            if (origin - app.camera.pos).length() > far + radius {
                continue;
            }
            if !sphere_in_frustum(frustum, origin, radius + 256.0) {
                continue;
            }
            let face_count = mesh.faces.len();
            let selected_face = app
                .selected_col_face
                .filter(|face| face.placement == idx)
                .map(|face| face.face);
            let selected_vertex = selected_face.map(|_| app.selected_col_vertex % 3);
            let selected_vertices = BTreeSet::new();
            let hovered = app.hovered_col_face.filter(|face| face.placement == idx);
            let hovered_face = hovered.map(|face| face.face);
            let hovered_vertex = hovered.and(app.hovered_col_vertex);
            let model_cols = placement_matrix(placement).to_cols_array();
            gl::PushMatrix();
            gl::MultMatrixf(model_cols.as_ptr());
            draw_collision_mesh(
                mesh,
                selected_face,
                &BTreeSet::new(),
                &BTreeSet::new(),
                selected_vertex,
                &selected_vertices,
                hovered_face,
                hovered_vertex,
                &model_cols,
                app.camera.pos,
                false,
                false,
                false,
            );
            gl::PopMatrix();
            app.last_drawn_placements += 1;
            app.last_drawn_parts += 1;
            app.last_drawn_vertices += face_count * 3;
        }
        gl::DepthMask(gl::TRUE);
        gl::Disable(gl::BLEND);
    }
}

fn editing_material_color(material: usize, selected: bool) -> [f32; 4] {
    if selected {
        return [1.0, 0.78, 0.12, 1.0];
    }
    const COLORS: [[f32; 4]; 10] = [
        [0.56, 0.68, 0.82, 1.0],
        [0.68, 0.58, 0.42, 1.0],
        [0.48, 0.72, 0.58, 1.0],
        [0.72, 0.50, 0.50, 1.0],
        [0.62, 0.58, 0.78, 1.0],
        [0.78, 0.70, 0.48, 1.0],
        [0.46, 0.68, 0.72, 1.0],
        [0.70, 0.52, 0.68, 1.0],
        [0.64, 0.66, 0.54, 1.0],
        [0.50, 0.58, 0.72, 1.0],
    ];
    COLORS[material % COLORS.len()]
}

fn draw_editing_dff_preview(
    raw: &RawMesh,
    selected_material: usize,
    mode: ViewportRenderMode,
    classes: &TextureMaterialClasses,
    txd_textures: &TxdTextureIndex,
    txd_name: Option<&str>,
    fallback_material: u8,
) {
    let has_normals = raw.normals.len() == raw.vertices.len();
    unsafe {
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::CULL_FACE);
        gl::Begin(gl::TRIANGLES);
        for tri in &raw.triangles {
            let indices = [tri.a as usize, tri.b as usize, tri.c as usize];
            if indices.iter().any(|idx| *idx >= raw.vertices.len()) {
                continue;
            }
            let material_index = tri.material as usize;
            let color = match mode {
                ViewportRenderMode::ShadedTextured => {
                    editing_material_color(material_index, material_index == selected_material)
                }
                ViewportRenderMode::UnshadedTextured => [0.92, 0.92, 0.92, 1.0],
                ViewportRenderMode::CollisionClassification => {
                    let texture_name = raw
                        .material_textures
                        .get(material_index)
                        .map(String::as_str)
                        .unwrap_or_default();
                    let fingerprint =
                        texture_content_fingerprint(txd_textures, texture_name, txd_name);
                    collision_classification_color(
                        classes,
                        txd_name,
                        texture_name,
                        fingerprint,
                        fallback_material,
                    )
                }
            };
            gl::Color4f(color[0], color[1], color[2], color[3]);
            for idx in indices {
                let p = raw.vertices[idx];
                let n = if has_normals {
                    raw.normals[idx]
                } else {
                    V3 {
                        x: 0.0,
                        y: 0.0,
                        z: 1.0,
                    }
                };
                gl::Normal3f(n.x, n.y, n.z);
                gl::Vertex3f(p.x, p.y, p.z);
            }
        }
        gl::End();
        gl::LineWidth(2.0);
        gl::Color4f(1.0, 0.92, 0.20, 1.0);
        gl::Begin(gl::LINES);
        for tri in raw
            .triangles
            .iter()
            .filter(|tri| tri.material as usize == selected_material)
        {
            let indices = [tri.a as usize, tri.b as usize, tri.c as usize];
            if indices.iter().any(|idx| *idx >= raw.vertices.len()) {
                continue;
            }
            let a = raw.vertices[indices[0]];
            let b = raw.vertices[indices[1]];
            let c = raw.vertices[indices[2]];
            for (p, q) in [(a, b), (b, c), (c, a)] {
                gl::Vertex3f(p.x, p.y, p.z);
                gl::Vertex3f(q.x, q.y, q.z);
            }
        }
        gl::End();
        gl::Enable(gl::CULL_FACE);
    }
}

fn draw_raw_mesh_edges(raw: &RawMesh) {
    unsafe {
        gl::Enable(gl::DEPTH_TEST);
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::CULL_FACE);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::LineWidth(1.0);
        gl::Color4f(0.02, 0.03, 0.04, 0.62);
        gl::Begin(gl::LINES);
        for tri in &raw.triangles {
            let indices = [tri.a as usize, tri.b as usize, tri.c as usize];
            if indices.iter().any(|idx| *idx >= raw.vertices.len()) {
                continue;
            }
            for (a, b) in [
                (indices[0], indices[1]),
                (indices[1], indices[2]),
                (indices[2], indices[0]),
            ] {
                let p = raw.vertices[a];
                let q = raw.vertices[b];
                gl::Vertex3f(p.x, p.y, p.z);
                gl::Vertex3f(q.x, q.y, q.z);
            }
        }
        gl::End();
        gl::Disable(gl::BLEND);
        gl::Enable(gl::CULL_FACE);
    }
}

fn draw_dff_vertex_incident_edges(
    raw: &RawMesh,
    active_vertex: Option<usize>,
    selected_vertices: &BTreeSet<usize>,
) {
    let mut vertices = selected_vertices.clone();
    if let Some(vertex) = active_vertex {
        vertices.insert(vertex);
    }
    if vertices.is_empty() {
        return;
    }
    unsafe {
        gl::Enable(gl::DEPTH_TEST);
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::LineWidth(1.25);
        gl::Begin(gl::LINES);
        for tri in &raw.triangles {
            for (a, b) in [
                (tri.a as usize, tri.b as usize),
                (tri.b as usize, tri.c as usize),
                (tri.c as usize, tri.a as usize),
            ] {
                let (Some(pa), Some(pb)) = (raw.vertices.get(a), raw.vertices.get(b)) else {
                    continue;
                };
                match (vertices.contains(&a), vertices.contains(&b)) {
                    (true, true) => {
                        gl::Color4f(1.0, 0.72, 0.10, 0.28);
                        gl::Vertex3f(pa.x, pa.y, pa.z);
                        gl::Vertex3f(pb.x, pb.y, pb.z);
                    }
                    (true, false) => {
                        gl::Color4f(1.0, 0.72, 0.10, 0.28);
                        gl::Vertex3f(pa.x, pa.y, pa.z);
                        gl::Color4f(1.0, 0.72, 0.10, 0.035);
                        gl::Vertex3f(pb.x, pb.y, pb.z);
                    }
                    (false, true) => {
                        gl::Color4f(1.0, 0.72, 0.10, 0.035);
                        gl::Vertex3f(pa.x, pa.y, pa.z);
                        gl::Color4f(1.0, 0.72, 0.10, 0.28);
                        gl::Vertex3f(pb.x, pb.y, pb.z);
                    }
                    (false, false) => {}
                }
            }
        }
        gl::End();
        gl::Disable(gl::BLEND);
    }
}

fn draw_dff_face_vertex_overlay(
    raw: &RawMesh,
    selected_face: Option<usize>,
    selected_faces: &BTreeSet<usize>,
    selected_edges: &BTreeSet<(usize, usize)>,
    selected_vertex: Option<usize>,
    selected_vertices: &BTreeSet<usize>,
    hovered_vertex: Option<usize>,
    camera_pos: Vec3,
    show_vertices: bool,
    show_faces: bool,
) {
    draw_raw_mesh_edges(raw);
    if show_vertices {
        draw_nearby_raw_vertices(raw, camera_pos);
        draw_dff_vertex_incident_edges(raw, selected_vertex, selected_vertices);
    }
    unsafe {
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::CULL_FACE);
        let mut faces = selected_faces.clone();
        if let Some(face_idx) = selected_face {
            faces.insert(face_idx);
        }
        if show_faces && !faces.is_empty() {
            gl::Enable(gl::DEPTH_TEST);
            gl::LineWidth(4.0);
            gl::Color4f(0.2, 0.9, 1.0, 1.0);
            gl::Begin(gl::LINES);
            for face_idx in faces {
                let Some(tri) = raw.triangles.get(face_idx) else {
                    continue;
                };
                let indices = [tri.a as usize, tri.b as usize, tri.c as usize];
                if indices.iter().all(|idx| *idx < raw.vertices.len()) {
                    for (a, b) in [
                        (indices[0], indices[1]),
                        (indices[1], indices[2]),
                        (indices[2], indices[0]),
                    ] {
                        let p = raw.vertices[a];
                        let q = raw.vertices[b];
                        gl::Vertex3f(p.x, p.y, p.z);
                        gl::Vertex3f(q.x, q.y, q.z);
                    }
                }
            }
            gl::End();
        }
        if !selected_edges.is_empty() {
            gl::Enable(gl::DEPTH_TEST);
            gl::LineWidth(4.0);
            gl::Color4f(1.0, 0.62, 0.14, 1.0);
            gl::Begin(gl::LINES);
            for (a, b) in selected_edges {
                let (Some(a), Some(b)) = (raw.vertices.get(*a), raw.vertices.get(*b)) else {
                    continue;
                };
                gl::Vertex3f(a.x, a.y, a.z);
                gl::Vertex3f(b.x, b.y, b.z);
            }
            gl::End();
        }
        if show_vertices {
            if let Some(vertex_idx) = selected_vertex {
                if let Some(p) = raw.vertices.get(vertex_idx) {
                    gl::Enable(gl::DEPTH_TEST);
                    gl::PointSize(4.0);
                    gl::Color4f(1.0, 1.0, 1.0, 1.0);
                    gl::Begin(gl::POINTS);
                    gl::Vertex3f(p.x, p.y, p.z);
                    gl::End();
                }
            }
            if !selected_vertices.is_empty() {
                gl::Enable(gl::DEPTH_TEST);
                gl::PointSize(6.0);
                gl::Begin(gl::POINTS);
                for vertex_idx in selected_vertices {
                    if let Some(p) = raw.vertices.get(*vertex_idx) {
                        let point = to_mq(*p);
                        let alpha = vertex_marker_alpha(camera_pos, point).max(0.72);
                        gl::Color4f(1.0, 0.18, 0.92, alpha);
                        gl::Vertex3f(p.x, p.y, p.z);
                    }
                }
                gl::End();
                gl::PointSize(3.0);
                gl::Begin(gl::POINTS);
                for vertex_idx in selected_vertices {
                    if let Some(p) = raw.vertices.get(*vertex_idx) {
                        gl::Color4f(1.0, 1.0, 1.0, 0.98);
                        gl::Vertex3f(p.x, p.y, p.z);
                    }
                }
                gl::End();
            }
            if let Some(vertex_idx) = hovered_vertex {
                if selected_vertex != Some(vertex_idx) {
                    if let Some(p) = raw.vertices.get(vertex_idx) {
                        gl::Enable(gl::DEPTH_TEST);
                        gl::PointSize(6.0);
                        let alpha = vertex_marker_alpha(camera_pos, to_mq(*p)).max(0.55);
                        gl::Color4f(0.15, 0.95, 1.0, alpha);
                        gl::Begin(gl::POINTS);
                        gl::Vertex3f(p.x, p.y, p.z);
                        gl::End();
                    }
                }
            }
        }
        gl::Enable(gl::CULL_FACE);
    }
}

fn draw_dff_fracture_zone_overlay(raw: &RawMesh, selected_group: usize) {
    unsafe {
        gl::PushAttrib(gl::ENABLE_BIT | gl::LINE_BIT | gl::CURRENT_BIT);
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::CULL_FACE);
        gl::Enable(gl::DEPTH_TEST);
        gl::LineWidth(2.5);
        gl::Begin(gl::LINES);
        for breakable in raw
            .components
            .iter()
            .filter_map(|component| component.breakable.as_ref())
        {
            for triangle in &breakable.triangles {
                let Some(face) = triangle
                    .source_face
                    .and_then(|face| raw.triangles.get(face))
                else {
                    continue;
                };
                let group = triangle.group as usize;
                let selected = group == selected_group;
                let red = 0.35 + ((group * 47 % 100) as f32 / 250.0);
                let green = 0.28 + ((group * 71 % 100) as f32 / 300.0);
                let blue = 0.45 + ((group * 29 % 100) as f32 / 250.0);
                gl::Color4f(
                    if selected { (red + 0.25).min(1.0) } else { red },
                    if selected {
                        (green + 0.25).min(1.0)
                    } else {
                        green
                    },
                    if selected {
                        (blue + 0.25).min(1.0)
                    } else {
                        blue
                    },
                    1.0,
                );
                let indices = [face.a as usize, face.b as usize, face.c as usize];
                if indices.iter().all(|index| *index < raw.vertices.len()) {
                    for (a, b) in [
                        (indices[0], indices[1]),
                        (indices[1], indices[2]),
                        (indices[2], indices[0]),
                    ] {
                        let a = raw.vertices[a];
                        let b = raw.vertices[b];
                        gl::Vertex3f(a.x, a.y, a.z);
                        gl::Vertex3f(b.x, b.y, b.z);
                    }
                }
            }
        }
        gl::End();
        gl::PopAttrib();
    }
}

#[derive(Clone, Copy, Debug)]
struct FracturePreviewMotion {
    pivot: Vec3,
    translation: Vec3,
    spin_axis: Vec3,
    spin_degrees: f32,
    settle_rotation_degrees: Vec3,
}

fn fracture_preview_seed(group: usize, salt: u32) -> f32 {
    let mut value = (group as u32).wrapping_mul(0x9e37_79b9).wrapping_add(salt);
    value ^= value >> 16;
    value = value.wrapping_mul(0x7feb_352d);
    value ^= value >> 15;
    (value & 0xffff) as f32 / 65_535.0
}

fn fracture_preview_signed_seed(group: usize, salt: u32) -> f32 {
    fracture_preview_seed(group, salt) * 2.0 - 1.0
}

fn fracture_preview_motion(
    breakable: &BreakableGeometry,
    group_index: usize,
    elapsed: f32,
    base_velocity: Vec3,
    velocity_randomness: f32,
) -> Option<FracturePreviewMotion> {
    let mut group_vertices = BTreeSet::<usize>::new();
    for triangle in breakable
        .triangles
        .iter()
        .filter(|triangle| triangle.group as usize == group_index)
    {
        group_vertices.extend(triangle.vertices.into_iter().map(usize::from));
    }
    if group_vertices.is_empty() {
        return None;
    }

    let mut object_min = Vec3::splat(f32::INFINITY);
    for vertex in &breakable.vertices {
        let position = to_mq(vertex.position);
        object_min = object_min.min(position);
    }
    let mut pivot = Vec3::ZERO;
    let mut group_min = Vec3::splat(f32::INFINITY);
    let mut group_max = Vec3::splat(f32::NEG_INFINITY);
    let mut count = 0.0f32;
    for index in group_vertices {
        let Some(vertex) = breakable.vertices.get(index) else {
            continue;
        };
        let position = to_mq(vertex.position);
        pivot += position;
        group_min = group_min.min(position);
        group_max = group_max.max(position);
        count += 1.0;
    }
    if count <= 0.0 {
        return None;
    }
    pivot /= count;

    // BreakObject_c::SetGroupData starts every group at the object.dat break
    // velocity, then adds independent random values to X/Y/Z. It does not add
    // an outward-from-center impulse.
    let randomness = velocity_randomness.max(0.0);
    let mut velocity = base_velocity
        + vec3(
            fracture_preview_signed_seed(group_index, 0x1c69_b3f7) * randomness,
            fracture_preview_signed_seed(group_index, 0xa4d9_4a4f) * randomness,
            fracture_preview_signed_seed(group_index, 0x51f2_a931) * randomness,
        );
    let extents = (group_max - group_min).max(Vec3::ZERO);
    let (thin_axis, bounding_size) = if extents.x <= extents.y && extents.x <= extents.z {
        (0usize, extents.x * 0.5)
    } else if extents.y <= extents.x && extents.y <= extents.z {
        (1usize, extents.y * 0.5)
    } else {
        (2usize, extents.z * 0.5)
    };
    let ground_z = object_min.z;
    let landing_center_z = ground_z + bounding_size;

    // SA's time step is approximately 1.0 at 30 FPS. Replaying fixed frame
    // steps makes its velocity.z -= timeStep / 125 gravity rule directly
    // visible and gives the same characteristic small bounce on ground hit.
    let gta_frames = elapsed.clamp(0.0, FRACTURE_PREVIEW_DURATION_SECONDS as f32) * 30.0;
    let whole_frames = gta_frames.floor() as usize;
    let partial_frame = gta_frames.fract();
    let mut translation = Vec3::ZERO;
    let mut first_collision_frame = None::<f32>;
    let mut advance = |time_step: f32, frame: f32| {
        velocity.z -= time_step / 125.0;
        translation += velocity * time_step;
        if pivot.z + translation.z - bounding_size < ground_z {
            translation.z = landing_center_z - pivot.z;
            first_collision_frame.get_or_insert(frame);
            let impact_speed = velocity.length();
            // On a horizontal plane SA's 0.85 reflection followed by its 0.8
            // velocity scaling is approximately a 0.56 vertical restitution.
            velocity = vec3(velocity.x * 0.8, velocity.y * 0.8, -velocity.z * 0.56);
            if impact_speed < 0.05 {
                velocity = Vec3::ZERO;
            }
        }
    };
    for frame in 0..whole_frames {
        advance(1.0, frame as f32 + 1.0);
    }
    if partial_frame > 0.0 {
        advance(partial_frame, whole_frames as f32 + partial_frame);
    }

    let mut spin_axis = vec3(
        fracture_preview_signed_seed(group_index, 0x8d12_2a11),
        fracture_preview_signed_seed(group_index, 0xf018_5ca9),
        fracture_preview_signed_seed(group_index, 0x61bc_932d),
    )
    .normalize_or_zero();
    if spin_axis.length_squared() < 0.0001 {
        spin_axis = Vec3::X;
    }
    let spin_speed = 3.0 + fracture_preview_seed(group_index, 0x7c4a_7b11) * 3.0;
    let spin_frames = gta_frames
        .min(5.0)
        .min(first_collision_frame.unwrap_or(f32::INFINITY));
    let spin_degrees = spin_speed * spin_frames;

    // After five frames SA turns the group's thinnest bounding-box axis toward
    // the ground normal. This approximation preserves that visible settling
    // behavior while keeping the preview deterministic.
    let settle_target = match thin_axis {
        0 => vec3(0.0, -90.0, 0.0),
        1 => vec3(90.0, 0.0, 0.0),
        _ => Vec3::ZERO,
    };
    let settle_frames = (gta_frames - 5.0).max(0.0);
    let settle_factor = 1.0 - 0.95f32.powf(settle_frames);

    Some(FracturePreviewMotion {
        pivot,
        translation,
        spin_axis,
        spin_degrees,
        settle_rotation_degrees: settle_target * settle_factor,
    })
}

fn fracture_preview_texture(
    textures: &HashMap<String, u32>,
    txd_name: Option<&str>,
    texture_name: &str,
) -> u32 {
    let texture_name = texture_name.trim();
    if texture_name.is_empty() {
        return 0;
    }
    let texture_key = lower(texture_name);
    let cache_key = txd_name
        .map(|txd| format!("{}|{texture_key}", asset_key(txd, ".txd")))
        .unwrap_or(texture_key);
    textures.get(&cache_key).copied().unwrap_or(0)
}

fn draw_breakable_fracture_preview(
    raw: &RawMesh,
    textures: &HashMap<String, u32>,
    txd_name: Option<&str>,
    selected_group: usize,
    elapsed: f32,
    base_velocity: Vec3,
    velocity_randomness: f32,
) -> (usize, usize) {
    let mut group_offset = 0usize;
    let mut triangle_count = 0usize;
    let mut vertex_count = 0usize;
    unsafe {
        gl::PushAttrib(
            gl::ENABLE_BIT
                | gl::CURRENT_BIT
                | gl::COLOR_BUFFER_BIT
                | gl::DEPTH_BUFFER_BIT
                | gl::LINE_BIT
                | gl::TEXTURE_BIT,
        );
        gl::Enable(gl::DEPTH_TEST);
        gl::Enable(gl::ALPHA_TEST);
        gl::AlphaFunc(gl::GREATER, 0.05);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::DepthMask(gl::TRUE);
        gl::Disable(gl::LIGHTING);
        // Debris meshes commonly contain only the original exterior faces.
        // Double-sided previewing keeps thin fence and sign fragments legible.
        gl::Disable(gl::CULL_FACE);

        for breakable in raw
            .components
            .iter()
            .filter_map(|component| component.breakable.as_ref())
        {
            for (local_group, group) in breakable.groups.iter().enumerate() {
                let Some(motion) = fracture_preview_motion(
                    breakable,
                    local_group,
                    elapsed,
                    base_velocity,
                    velocity_randomness,
                ) else {
                    continue;
                };
                let display_group = group_offset + local_group;
                let texture_name = if group.texture.trim().is_empty() {
                    group.mask.as_str()
                } else {
                    group.texture.as_str()
                };
                let texture = fracture_preview_texture(textures, txd_name, texture_name);
                if texture != 0 {
                    gl::Enable(gl::TEXTURE_2D);
                    gl::BindTexture(gl::TEXTURE_2D, texture);
                } else {
                    gl::Disable(gl::TEXTURE_2D);
                }

                gl::PushMatrix();
                gl::Translatef(
                    motion.pivot.x + motion.translation.x,
                    motion.pivot.y + motion.translation.y,
                    motion.pivot.z + motion.translation.z,
                );
                gl::Rotatef(motion.settle_rotation_degrees.x, 1.0, 0.0, 0.0);
                gl::Rotatef(motion.settle_rotation_degrees.y, 0.0, 1.0, 0.0);
                gl::Rotatef(motion.settle_rotation_degrees.z, 0.0, 0.0, 1.0);
                gl::Rotatef(
                    motion.spin_degrees,
                    motion.spin_axis.x,
                    motion.spin_axis.y,
                    motion.spin_axis.z,
                );
                gl::Translatef(-motion.pivot.x, -motion.pivot.y, -motion.pivot.z);

                gl::Begin(gl::TRIANGLES);
                for triangle in breakable
                    .triangles
                    .iter()
                    .filter(|triangle| triangle.group as usize == local_group)
                {
                    let indices = triangle.vertices.map(usize::from);
                    let Some(a) = breakable.vertices.get(indices[0]) else {
                        continue;
                    };
                    let Some(b) = breakable.vertices.get(indices[1]) else {
                        continue;
                    };
                    let Some(c) = breakable.vertices.get(indices[2]) else {
                        continue;
                    };
                    let normal = (to_mq(b.position) - to_mq(a.position))
                        .cross(to_mq(c.position) - to_mq(a.position))
                        .normalize_or_zero();
                    gl::Normal3f(normal.x, normal.y, normal.z);
                    for vertex in [a, b, c] {
                        let fallback = editing_material_color(display_group, false);
                        let texture_tint = if texture != 0 {
                            [1.0, 1.0, 1.0]
                        } else {
                            [fallback[0], fallback[1], fallback[2]]
                        };
                        gl::Color4f(
                            vertex.color[0] as f32 / 255.0
                                * group.ambient.x.clamp(0.0, 1.0)
                                * texture_tint[0],
                            vertex.color[1] as f32 / 255.0
                                * group.ambient.y.clamp(0.0, 1.0)
                                * texture_tint[1],
                            vertex.color[2] as f32 / 255.0
                                * group.ambient.z.clamp(0.0, 1.0)
                                * texture_tint[2],
                            vertex.color[3] as f32 / 255.0,
                        );
                        gl::TexCoord2f(vertex.uv.u, vertex.uv.v);
                        gl::Vertex3f(vertex.position.x, vertex.position.y, vertex.position.z);
                    }
                    triangle_count += 1;
                    vertex_count += 3;
                }
                gl::End();

                gl::Disable(gl::TEXTURE_2D);
                let selected = display_group == selected_group;
                let edge = editing_material_color(display_group, selected);
                gl::LineWidth(if selected { 2.5 } else { 1.25 });
                gl::Color4f(edge[0], edge[1], edge[2], if selected { 1.0 } else { 0.72 });
                gl::Begin(gl::LINES);
                for triangle in breakable
                    .triangles
                    .iter()
                    .filter(|triangle| triangle.group as usize == local_group)
                {
                    let indices = triangle.vertices.map(usize::from);
                    if indices
                        .iter()
                        .any(|index| *index >= breakable.vertices.len())
                    {
                        continue;
                    }
                    for (a, b) in [(0, 1), (1, 2), (2, 0)] {
                        let a = breakable.vertices[indices[a]].position;
                        let b = breakable.vertices[indices[b]].position;
                        gl::Vertex3f(a.x, a.y, a.z);
                        gl::Vertex3f(b.x, b.y, b.z);
                    }
                }
                gl::End();
                gl::PopMatrix();
            }
            group_offset += breakable.groups.len();
        }
        gl::BindTexture(gl::TEXTURE_2D, 0);
        gl::PopAttrib();
    }
    (triangle_count, vertex_count)
}

fn draw_render_mesh_edges(mesh: &RenderMesh) {
    unsafe {
        gl::Enable(gl::DEPTH_TEST);
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::CULL_FACE);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::LineWidth(1.0);
        gl::Color4f(0.02, 0.03, 0.04, 0.62);
        gl::Begin(gl::LINES);
        for part in &mesh.parts {
            for tri in part.cpu_vertices.chunks_exact(3) {
                for (a, b) in [(0, 1), (1, 2), (2, 0)] {
                    let p = tri[a].pos;
                    let q = tri[b].pos;
                    gl::Vertex3f(p.x, p.y, p.z);
                    gl::Vertex3f(q.x, q.y, q.z);
                }
            }
        }
        gl::End();
        gl::Disable(gl::BLEND);
        gl::Enable(gl::CULL_FACE);
    }
}

fn draw_dff_boolean_box(cutter: DffBooleanBox) {
    let c = to_mq(cutter.center);
    let e = to_mq(cutter.half_extents);
    let corners = [
        vec3(c.x - e.x, c.y - e.y, c.z - e.z),
        vec3(c.x + e.x, c.y - e.y, c.z - e.z),
        vec3(c.x + e.x, c.y + e.y, c.z - e.z),
        vec3(c.x - e.x, c.y + e.y, c.z - e.z),
        vec3(c.x - e.x, c.y - e.y, c.z + e.z),
        vec3(c.x + e.x, c.y - e.y, c.z + e.z),
        vec3(c.x + e.x, c.y + e.y, c.z + e.z),
        vec3(c.x - e.x, c.y + e.y, c.z + e.z),
    ];
    unsafe {
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::CULL_FACE);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::Color4f(0.15, 0.75, 1.0, 0.16);
        gl::Begin(gl::QUADS);
        for face in [
            [0, 1, 2, 3],
            [4, 7, 6, 5],
            [0, 4, 5, 1],
            [1, 5, 6, 2],
            [2, 6, 7, 3],
            [3, 7, 4, 0],
        ] {
            for idx in face {
                let p = corners[idx];
                gl::Vertex3f(p.x, p.y, p.z);
            }
        }
        gl::End();
        gl::LineWidth(2.0);
        gl::Color4f(0.25, 0.9, 1.0, 0.95);
        gl::Begin(gl::LINES);
        for (a, b) in [
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
        ] {
            let p = corners[a];
            let q = corners[b];
            gl::Vertex3f(p.x, p.y, p.z);
            gl::Vertex3f(q.x, q.y, q.z);
        }
        gl::End();
        gl::Disable(gl::BLEND);
        gl::Enable(gl::CULL_FACE);
    }
}

fn particle_name_from_payload(payload: &[u8]) -> String {
    payload
        .iter()
        .copied()
        .take(24)
        .take_while(|byte| *byte != 0)
        .map(char::from)
        .collect()
}

fn payload_u8(payload: &[u8], offset: usize, default: u8) -> u8 {
    payload.get(offset).copied().unwrap_or(default)
}

fn payload_f32(payload: &[u8], offset: usize, default: f32) -> f32 {
    let Some(bytes) = payload.get(offset..offset + 4) else {
        return default;
    };
    f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn payload_string(payload: &[u8], offset: usize, len: usize) -> String {
    payload
        .get(offset..offset + len)
        .unwrap_or(&[])
        .iter()
        .copied()
        .take_while(|byte| *byte != 0)
        .map(char::from)
        .collect()
}

fn dff_2dfx_active_at_time(effect: &Dff2dEffect, hour_index: usize) -> bool {
    if effect.effect_id != 0 {
        return true;
    }
    let flags = payload_u8(&effect.payload, 24, 0);
    let day = flags & 0x20 != 0;
    let night = flags & 0x40 != 0;
    if !day && !night {
        return true;
    }
    let preview_is_night = matches!(hour_index % TIMECYC_HOURS.len(), 0 | 1 | 6 | 7);
    if preview_is_night { night } else { day }
}

const DFF_2DFX_CORONA_SHOW_MODE_OFFSET: usize = 20;
const DFF_2DFX_TRAFFIC_LIGHT_SHOW_MODE: u8 = 7;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DffTrafficLightState {
    Green,
    Yellow,
    Red,
}

fn dff_2dfx_traffic_light_state(effect: &Dff2dEffect) -> Option<DffTrafficLightState> {
    if effect.effect_id != 0
        || payload_u8(&effect.payload, DFF_2DFX_CORONA_SHOW_MODE_OFFSET, u8::MAX)
            != DFF_2DFX_TRAFFIC_LIGHT_SHOW_MODE
    {
        return None;
    }
    let red = payload_u8(&effect.payload, 0, 255);
    let green = payload_u8(&effect.payload, 1, 255);
    Some(if red > 200 {
        if green > 100 {
            DffTrafficLightState::Yellow
        } else {
            DffTrafficLightState::Red
        }
    } else {
        DffTrafficLightState::Green
    })
}

fn dff_traffic_light_state_for_phase(
    elapsed_seconds: f64,
    north_south: bool,
) -> DffTrafficLightState {
    // GTA halves its millisecond timer before masking it to 14 bits, making a
    // full traffic-light cycle 32.768 seconds.
    let phase = ((elapsed_seconds.max(0.0) * 500.0) as u64 & 16_383) as u32;
    if north_south {
        if phase < 5_000 {
            DffTrafficLightState::Green
        } else if phase < 6_000 {
            DffTrafficLightState::Yellow
        } else {
            DffTrafficLightState::Red
        }
    } else if phase < 6_000 {
        DffTrafficLightState::Red
    } else if phase < 11_000 {
        DffTrafficLightState::Green
    } else if phase < 12_000 {
        DffTrafficLightState::Yellow
    } else {
        DffTrafficLightState::Red
    }
}

fn dff_traffic_light_is_north_south(rotation_z_degrees: f32) -> bool {
    let heading = rotation_z_degrees.rem_euclid(360.0);
    !((heading <= 60.0 || heading >= 150.0) && (heading <= 240.0 || heading >= 330.0))
}

fn dff_2dfx_active_for_traffic_preview(effect: &Dff2dEffect, state: DffTrafficLightState) -> bool {
    match dff_2dfx_traffic_light_state(effect) {
        Some(effect_state) => effect_state == state,
        None => true,
    }
}

fn dff_2dfx_in_preview_range(effect: &Dff2dEffect, camera_pos: Vec3) -> bool {
    if effect.effect_id != 0 {
        return true;
    }
    let far_clip = payload_f32(&effect.payload, 4, 60.0);
    far_clip <= 0.0 || (to_mq(effect.position) - camera_pos).length() <= far_clip
}

const MAX_DFF_POINT_LIGHTS: usize = 32;

#[derive(Clone, Copy, Debug)]
struct DffPointLight {
    position: Vec3,
    radius: f32,
    color: Vec3,
}

fn dff_2dfx_point_light(effect: &Dff2dEffect) -> Option<DffPointLight> {
    if effect.effect_id != 0 {
        return None;
    }
    let radius = payload_f32(&effect.payload, 8, 0.0);
    if !radius.is_finite() || radius <= 0.0 {
        return None;
    }
    Some(DffPointLight {
        position: to_mq(effect.position),
        radius,
        color: vec3(
            payload_u8(&effect.payload, 0, 255) as f32 / 256.0,
            payload_u8(&effect.payload, 1, 255) as f32 / 256.0,
            payload_u8(&effect.payload, 2, 255) as f32 / 256.0,
        ),
    })
}

/// GTA:SA point lights remain at full strength through the inner half of their
/// radius, then fade linearly to zero through the outer half.
#[cfg(test)]
fn sa_2dfx_pointlight_attenuation(normalized_distance: f32) -> f32 {
    if normalized_distance <= 0.5 {
        1.0
    } else {
        (2.0 * (1.0 - normalized_distance)).clamp(0.0, 1.0)
    }
}

fn sphere_intersects_aabb(center: Vec3, radius: f32, min: Vec3, max: Vec3) -> bool {
    let nearest = center.clamp(min, max);
    (nearest - center).length_squared() <= radius * radius
}

fn select_dff_point_lights(
    effects: &[Dff2dEffect],
    camera_pos: Vec3,
    sprite_brightness: f32,
) -> Vec<DffPointLight> {
    let sprite_brightness = sprite_brightness.max(0.0);
    let mut candidates: Vec<(f32, DffPointLight)> = effects
        .iter()
        .filter_map(|effect| {
            dff_2dfx_point_light(effect).map(|light| {
                let preview_scale = if dff_2dfx_is_generated_corona(effect) {
                    DFF_GENERATED_EDITOR_POINTLIGHT_SCALE
                } else {
                    1.0
                };
                (light, preview_scale)
            })
        })
        .map(|(mut light, preview_scale)| {
            light.color *= sprite_brightness * preview_scale;
            ((light.position - camera_pos).length_squared(), light)
        })
        .collect();
    candidates.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut selected = Vec::<DffPointLight>::with_capacity(MAX_DFF_POINT_LIGHTS);
    for (_, light) in candidates {
        // Detail and LOD meshes can briefly coexist in the transition band.
        // Do not spend two of SA's 32 light slots on the same embedded 2DFX.
        let duplicate = selected.iter().any(|existing| {
            (existing.position - light.position).length_squared() < 0.25 * 0.25
                && (existing.radius - light.radius).abs() < 0.05
                && (existing.color - light.color).length_squared() < 0.01 * 0.01
        });
        if !duplicate {
            selected.push(light);
            if selected.len() == MAX_DFF_POINT_LIGHTS {
                break;
            }
        }
    }
    selected
}

fn create_dff_pointlight_preview_program() -> Result<u32, String> {
    const VERTEX_SOURCE: &str = r#"#version 120
uniform mat4 model_to_world;
varying vec3 world_position;
varying vec3 world_normal;
varying vec2 texture_uv;
varying float vertex_alpha;
varying float fog_distance;

void main() {
    vec4 world = model_to_world * gl_Vertex;
    world_position = world.xyz;
    world_normal = normalize(mat3(model_to_world) * gl_Normal);
    texture_uv = gl_MultiTexCoord0.xy;
    vertex_alpha = gl_Color.a;
    vec4 eye = gl_ModelViewMatrix * gl_Vertex;
    fog_distance = abs(eye.z);
    gl_Position = gl_ModelViewProjectionMatrix * gl_Vertex;
}
"#;
    const FRAGMENT_SOURCE: &str = r#"#version 120
const int MAX_POINT_LIGHTS = 32;
uniform int light_count;
uniform vec4 light_position_radius[MAX_POINT_LIGHTS];
uniform vec4 light_color[MAX_POINT_LIGHTS];
uniform sampler2D diffuse_texture;
uniform int has_texture;
uniform vec4 base_color;
uniform float alpha_cutoff;
uniform int fog_enabled;
uniform float fog_start;
uniform float fog_end;
varying vec3 world_position;
varying vec3 world_normal;
varying vec2 texture_uv;
varying float vertex_alpha;
varying float fog_distance;

void main() {
    vec4 texel = has_texture != 0 ? texture2D(diffuse_texture, texture_uv)
                                  : vec4(1.0);
    float alpha = texel.a * base_color.a * vertex_alpha;
    if (alpha <= alpha_cutoff) {
        discard;
    }

    vec3 normal = normalize(world_normal);
    vec3 point_light = vec3(0.0);
    for (int i = 0; i < MAX_POINT_LIGHTS; ++i) {
        if (i >= light_count) {
            break;
        }
        vec3 delta = light_position_radius[i].xyz - world_position;
        float radius = light_position_radius[i].w;
        float distance_to_light = length(delta);
        if (distance_to_light >= radius || radius <= 0.0) {
            continue;
        }
        float normalized_distance = distance_to_light / radius;
        float attenuation = normalized_distance <= 0.5
            ? 1.0
            : max(0.0, 2.0 * (1.0 - normalized_distance));
        float diffuse = max(dot(normal, delta / max(distance_to_light, 0.0001)), 0.0);
        point_light += light_color[i].rgb * attenuation * diffuse;
    }

    float fog = 1.0;
    if (fog_enabled != 0) {
        fog = clamp((fog_end - fog_distance) / max(fog_end - fog_start, 0.0001), 0.0, 1.0);
    }
    gl_FragColor = vec4(texel.rgb * base_color.rgb * point_light * fog, 0.0);
}
"#;

    let vertex = compile_shader(gl::VERTEX_SHADER, VERTEX_SOURCE)?;
    let fragment = match compile_shader(gl::FRAGMENT_SHADER, FRAGMENT_SOURCE) {
        Ok(shader) => shader,
        Err(err) => {
            unsafe { gl::DeleteShader(vertex) };
            return Err(err);
        }
    };
    unsafe {
        let program = gl::CreateProgram();
        if program == 0 {
            gl::DeleteShader(vertex);
            gl::DeleteShader(fragment);
            return Err("glCreateProgram returned 0".to_string());
        }
        gl::AttachShader(program, vertex);
        gl::AttachShader(program, fragment);
        gl::LinkProgram(program);
        gl::DeleteShader(vertex);
        gl::DeleteShader(fragment);
        let mut ok = 0;
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut ok);
        if ok == 0 {
            let log = program_info_log(program);
            gl::DeleteProgram(program);
            return Err(log);
        }
        Ok(program)
    }
}

fn ensure_dff_pointlight_preview_program(app: &mut AppState) -> u32 {
    if app.dff_pointlight_preview_program != 0 || app.dff_pointlight_preview_failed {
        return app.dff_pointlight_preview_program;
    }
    match create_dff_pointlight_preview_program() {
        Ok(program) => {
            app.dff_pointlight_preview_program = program;
            program
        }
        Err(err) => {
            app.dff_pointlight_preview_failed = true;
            eprintln!("2DFX per-pixel light preview unavailable: {err}");
            0
        }
    }
}

unsafe fn upload_dff_pointlight_uniforms(program: u32, lights: &[DffPointLight]) {
    let mut position_radius = [[0.0_f32; 4]; MAX_DFF_POINT_LIGHTS];
    let mut colors = [[0.0_f32; 4]; MAX_DFF_POINT_LIGHTS];
    for (index, light) in lights.iter().enumerate() {
        position_radius[index] = [
            light.position.x,
            light.position.y,
            light.position.z,
            light.radius,
        ];
        colors[index] = [light.color.x, light.color.y, light.color.z, 1.0];
    }
    unsafe {
        gl::Uniform1i(
            uniform_location(program, "light_count"),
            lights.len() as i32,
        );
        gl::Uniform4fv(
            uniform_location(program, "light_position_radius"),
            MAX_DFF_POINT_LIGHTS as i32,
            position_radius.as_ptr().cast(),
        );
        gl::Uniform4fv(
            uniform_location(program, "light_color"),
            MAX_DFF_POINT_LIGHTS as i32,
            colors.as_ptr().cast(),
        );
        gl::Uniform1i(uniform_location(program, "diffuse_texture"), 0);
        let mut fog_start = 0.0;
        let mut fog_end = 1.0;
        gl::GetFloatv(gl::FOG_START, &mut fog_start);
        gl::GetFloatv(gl::FOG_END, &mut fog_end);
        gl::Uniform1i(
            uniform_location(program, "fog_enabled"),
            if gl::IsEnabled(gl::FOG) == gl::TRUE {
                1
            } else {
                0
            },
        );
        gl::Uniform1f(uniform_location(program, "fog_start"), fog_start);
        gl::Uniform1f(uniform_location(program, "fog_end"), fog_end);
    }
}

unsafe fn set_dff_pointlight_material_uniforms(
    program: u32,
    texture: u32,
    color: Vec3,
    alpha: f32,
    transparency: TransparencyMode,
) {
    unsafe {
        gl::ActiveTexture(gl::TEXTURE0);
        gl::BindTexture(gl::TEXTURE_2D, texture);
        gl::Uniform1i(
            uniform_location(program, "has_texture"),
            if texture != 0 { 1 } else { 0 },
        );
        gl::Uniform4f(
            uniform_location(program, "base_color"),
            color.x,
            color.y,
            color.z,
            alpha,
        );
        let cutoff = match transparency {
            TransparencyMode::Blend => 0.98,
            TransparencyMode::Cutout => 0.08,
            TransparencyMode::Opaque => 0.0,
        };
        gl::Uniform1f(uniform_location(program, "alpha_cutoff"), cutoff);
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn draw_world_dff_pointlight_pass(
    cells: &[WorldCell],
    cam: Vec3,
    near: f32,
    far: f32,
    frustum: &[Plane; 6],
    program: u32,
    lights: &[DffPointLight],
) {
    let identity = Mat4::IDENTITY.to_cols_array();
    unsafe {
        gl::UniformMatrix4fv(
            uniform_location(program, "model_to_world"),
            1,
            gl::FALSE,
            identity.as_ptr(),
        );
        enable_world_client_arrays();
        for cell in cells {
            let distance_squared = (cell.center - cam).length_squared();
            let far_edge = far + cell.radius;
            let near_edge = near - cell.radius;
            if distance_squared > far_edge * far_edge
                || (near_edge > 0.0 && distance_squared < near_edge * near_edge)
                || !aabb_in_frustum(frustum, cell.min, cell.max)
                || !lights.iter().any(|light| {
                    sphere_intersects_aabb(light.position, light.radius, cell.min, cell.max)
                })
            {
                continue;
            }
            bind_world_vbo(cell.vbo);
            for batch in &cell.batches {
                if batch.double_sided {
                    gl::Disable(gl::CULL_FACE);
                } else {
                    gl::Enable(gl::CULL_FACE);
                }
                let color = if batch.texture == 0 {
                    vec3(0.78, 0.78, 0.74)
                } else {
                    Vec3::ONE
                };
                set_dff_pointlight_material_uniforms(
                    program,
                    batch.texture,
                    color,
                    1.0,
                    batch.transparency,
                );
                gl::DrawArrays(
                    gl::TRIANGLES,
                    batch.first_vertex.min(i32::MAX as usize) as i32,
                    batch.vertices.min(i32::MAX as usize) as i32,
                );
            }
        }
        gl::DisableClientState(gl::COLOR_ARRAY);
        gl::DisableClientState(gl::TEXTURE_COORD_ARRAY);
        gl::DisableClientState(gl::NORMAL_ARRAY);
        gl::DisableClientState(gl::VERTEX_ARRAY);
        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
    }
}

unsafe fn draw_simulation_dff_pointlight_pass(app: &AppState, frustum: &[Plane; 6], program: u32) {
    for object in &app.sim.objects {
        let Some(mesh) = sim_object_mesh(app, object.kind) else {
            continue;
        };
        let visual_scale = match object.kind {
            SimObjectKind::Player => SIM_PLAYER_VISUAL_SCALE,
            SimObjectKind::Vehicle => SIM_VEHICLE_VISUAL_SCALE,
        };
        let visual_rot = match object.kind {
            SimObjectKind::Player => object.rot_z + SIM_PLAYER_VISUAL_ROT_OFFSET,
            SimObjectKind::Vehicle => object.rot_z,
        };
        let upright_rot = match object.kind {
            SimObjectKind::Player => Mat4::from_rotation_x(SIM_PLAYER_VISUAL_UPRIGHT_ROT),
            SimObjectKind::Vehicle => Mat4::IDENTITY,
        };
        let world_min = object.pos + mesh.bounds.min * visual_scale;
        let world_max = object.pos + mesh.bounds.max * visual_scale;
        if !aabb_in_frustum(frustum, world_min, world_max) {
            continue;
        }
        let model = Mat4::from_translation(object.pos)
            * Mat4::from_rotation_z(visual_rot)
            * upright_rot
            * Mat4::from_scale(Vec3::splat(visual_scale));
        unsafe {
            gl::UniformMatrix4fv(
                uniform_location(program, "model_to_world"),
                1,
                gl::FALSE,
                model.to_cols_array().as_ptr(),
            );
            gl::PushMatrix();
            gl::MultMatrixf(model.to_cols_array().as_ptr());
            for part in &mesh.parts {
                set_dff_pointlight_material_uniforms(
                    program,
                    part.texture,
                    to_mq(part.material_color),
                    part.alpha,
                    part.transparency,
                );
                draw_render_part_buffer(part);
            }
            gl::PopMatrix();
        }
    }
}

fn draw_dff_pointlight_preview(
    app: &mut AppState,
    effects: &[Dff2dEffect],
    frustum: &[Plane; 6],
    cam: Vec3,
    far: f32,
    lod_near: f32,
    lod_far: f32,
    draw_lod: bool,
) {
    // SA stores sprite brightness in timecyc as a decimal, quantizes it to
    // tenths internally, then applies the reconstructed decimal to 2DFX RGB.
    let sprite_brightness = active_timecyc_sample(&app.timecyc)
        .values
        .get(23)
        .copied()
        .unwrap_or(1.0);
    let lights = select_dff_point_lights(effects, cam, sprite_brightness);
    if lights.is_empty() {
        return;
    }
    let program = ensure_dff_pointlight_preview_program(app);
    if program == 0 {
        return;
    }
    unsafe {
        gl::PushAttrib(gl::ALL_ATTRIB_BITS);
        gl::UseProgram(program);
        gl::Enable(gl::DEPTH_TEST);
        // Re-shade the visible surface. Exact equality is unreliable when the
        // base path used a display list and this pass uses the equivalent VBO.
        gl::DepthFunc(gl::LEQUAL);
        // The compatibility display-list and packed-VBO paths can differ by a
        // few depth bits even though they describe the same triangles. Pull
        // only this color-only pass forward enough to make the depth result
        // deterministic instead of producing a stippled/z-fighting light.
        gl::Enable(gl::POLYGON_OFFSET_FILL);
        gl::PolygonOffset(-1.0, -1.0);
        gl::DepthMask(gl::FALSE);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::ONE, gl::ONE);
        gl::Disable(gl::ALPHA_TEST);
        gl::Disable(gl::LIGHTING);
        gl::Enable(gl::CULL_FACE);
        upload_dff_pointlight_uniforms(program, &lights);
        draw_world_dff_pointlight_pass(&app.world_cells, cam, 0.0, far, frustum, program, &lights);
        if draw_lod {
            draw_world_dff_pointlight_pass(
                &app.lod_world_cells,
                cam,
                lod_near,
                lod_far,
                frustum,
                program,
                &lights,
            );
        }
        draw_simulation_dff_pointlight_pass(app, frustum, program);
        gl::UseProgram(0);
        gl::BindTexture(gl::TEXTURE_2D, 0);
        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        gl::PopAttrib();
    }
}

fn dff_2dfx_preview_texture_names(
    effects: &[Dff2dEffect],
    particle_effects: &[ParticleEffectDef],
) -> Vec<String> {
    let mut names = Vec::new();
    for effect in effects {
        match effect.effect_id {
            0 => {
                let corona = payload_string(&effect.payload, 25, 24);
                names.push(if corona.trim().is_empty() {
                    "coronastar".to_string()
                } else {
                    corona
                });
            }
            1 => {
                let particle_name = particle_name_from_payload(&effect.payload);
                if let Some(def) = particle_effects
                    .iter()
                    .find(|def| def.name.eq_ignore_ascii_case(&particle_name))
                {
                    names.extend(def.textures.iter().cloned());
                }
            }
            _ => {}
        }
    }
    names.sort_by_key(|name| lower(name));
    names.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    names
}

fn draw_textured_billboard(
    center: Vec3,
    right: Vec3,
    up: Vec3,
    size: f32,
    texture: u32,
    color: Color,
) {
    let right = right * size;
    let up = up * size;
    unsafe {
        if texture != 0 {
            gl::Enable(gl::TEXTURE_2D);
            gl::BindTexture(gl::TEXTURE_2D, texture);
        } else {
            gl::Disable(gl::TEXTURE_2D);
        }
        gl::Color4f(color.r, color.g, color.b, color.a);
        gl::Begin(gl::QUADS);
        gl::TexCoord2f(0.0, 0.0);
        gl::Vertex3f(
            center.x - right.x - up.x,
            center.y - right.y - up.y,
            center.z - right.z - up.z,
        );
        gl::TexCoord2f(1.0, 0.0);
        gl::Vertex3f(
            center.x + right.x - up.x,
            center.y + right.y - up.y,
            center.z + right.z - up.z,
        );
        gl::TexCoord2f(1.0, 1.0);
        gl::Vertex3f(
            center.x + right.x + up.x,
            center.y + right.y + up.y,
            center.z + right.z + up.z,
        );
        gl::TexCoord2f(0.0, 1.0);
        gl::Vertex3f(
            center.x - right.x + up.x,
            center.y - right.y + up.y,
            center.z - right.z + up.z,
        );
        gl::End();
        gl::BindTexture(gl::TEXTURE_2D, 0);
    }
}

fn transform_dff_2dfx_effect(effect: &Dff2dEffect, model: Mat4) -> Dff2dEffect {
    let mut out = effect.clone();
    let p = model.transform_point3(to_mq(effect.position));
    out.position = from_mq(p);
    out
}

unsafe fn draw_dff_pointlight_wire_sphere(effect: &Dff2dEffect) {
    let radius = payload_f32(&effect.payload, 8, 0.0);
    if effect.effect_id != 0 || radius <= 0.0 || !radius.is_finite() {
        return;
    }
    let p = to_mq(effect.position);
    unsafe {
        for plane in 0..3 {
            gl::Begin(gl::LINE_LOOP);
            for step in 0..48 {
                let angle = step as f32 / 48.0 * std::f32::consts::TAU;
                let (a, b) = (angle.cos() * radius, angle.sin() * radius);
                match plane {
                    0 => gl::Vertex3f(p.x + a, p.y + b, p.z),
                    1 => gl::Vertex3f(p.x + a, p.y, p.z + b),
                    _ => gl::Vertex3f(p.x, p.y + a, p.z + b),
                }
            }
            gl::End();
        }
    }
}

fn draw_dff_pointlight_volumes(effects: &[Dff2dEffect]) {
    unsafe {
        gl::PushAttrib(
            gl::ENABLE_BIT
                | gl::CURRENT_BIT
                | gl::LINE_BIT
                | gl::DEPTH_BUFFER_BIT
                | gl::COLOR_BUFFER_BIT,
        );
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::CULL_FACE);
        gl::Enable(gl::DEPTH_TEST);
        gl::DepthMask(gl::FALSE);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::LineWidth(2.5);
        gl::Color4f(0.12, 0.82, 1.0, 0.82);
        for effect in effects {
            draw_dff_pointlight_wire_sphere(effect);
        }
        gl::PopAttrib();
    }
}

fn draw_dff_corona_effect(
    effect: &Dff2dEffect,
    textures: &HashMap<String, u32>,
    right: Vec3,
    up: Vec3,
    sprite_size: f32,
    sprite_brightness: f32,
) {
    let corona = payload_string(&effect.payload, 25, 24);
    let texture_name = if corona.trim().is_empty() {
        "coronastar".to_string()
    } else {
        corona
    };
    let key = lower(&texture_name);
    let texture = textures
        .get(&key)
        .copied()
        .or_else(|| textures.get(&format!("particle.txd|{key}")).copied())
        .or_else(|| textures.get(&format!("effectspc.txd|{key}")).copied())
        .unwrap_or(0);
    let is_traffic_light = dff_2dfx_traffic_light_state(effect).is_some();
    let (color, size) = if is_traffic_light {
        // DisplayActualLight bypasses the ordinary 2DFX corona fields. It
        // renders one state with a much smaller timecyc-scaled sprite and only
        // 7% of sprite brightness.
        let brightness = sprite_brightness.max(0.0) * 0.07;
        (
            Color::new(
                payload_u8(&effect.payload, 0, 255) as f32 / 255.0 * brightness,
                payload_u8(&effect.payload, 1, 240) as f32 / 255.0 * brightness,
                payload_u8(&effect.payload, 2, 190) as f32 / 255.0 * brightness,
                1.0,
            ),
            sprite_size.max(0.1) * 0.175,
        )
    } else {
        let preview_alpha = payload_u8(&effect.payload, 3, 255) as f32 / 255.0
            * if dff_2dfx_is_generated_corona(effect) {
                DFF_GENERATED_EDITOR_CORONA_SCALE
            } else {
                1.0
            };
        (
            Color::new(
                payload_u8(&effect.payload, 0, 255) as f32 / 255.0,
                payload_u8(&effect.payload, 1, 240) as f32 / 255.0,
                payload_u8(&effect.payload, 2, 190) as f32 / 255.0,
                preview_alpha,
            ),
            payload_f32(&effect.payload, 12, 1.5).max(0.1) * 12.0,
        )
    };
    draw_textured_billboard(to_mq(effect.position), right, up, size, texture, color);
}

fn draw_dff_2dfx_effects(
    effects: &[Dff2dEffect],
    selected: Option<usize>,
    particle_effects: &[ParticleEffectDef],
    textures: &HashMap<String, u32>,
    camera: &CameraState,
    sprite_size: f32,
    sprite_brightness: f32,
    traffic_state: Option<DffTrafficLightState>,
) {
    if effects.is_empty() {
        return;
    }
    let (forward, right) = camera_vectors(camera);
    let up = right.cross(forward).normalize_or_zero();
    unsafe {
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::CULL_FACE);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE);
        gl::DepthMask(gl::FALSE);
    }
    for effect in effects {
        if traffic_state.is_some_and(|state| !dff_2dfx_active_for_traffic_preview(effect, state)) {
            continue;
        }
        let p = to_mq(effect.position);
        match effect.effect_id {
            0 => {
                draw_dff_corona_effect(effect, textures, right, up, sprite_size, sprite_brightness);
            }
            1 => {
                let particle_name = particle_name_from_payload(&effect.payload);
                if let Some(def) = particle_effects
                    .iter()
                    .find(|def| def.name.eq_ignore_ascii_case(&particle_name))
                {
                    let puff_count = if def.primitive_count > 1 { 6 } else { 4 };
                    let base_size = (def.cull_distance * 0.045).clamp(6.0, 28.0);
                    for puff in 0..puff_count {
                        let phase =
                            (get_time() as f32 * 0.45 + puff as f32 / puff_count as f32).fract();
                        let texture_name = if def.textures.is_empty() {
                            "smoke".to_string()
                        } else {
                            let frame = ((get_time() * 8.0) as usize + puff) % def.textures.len();
                            def.textures[frame].clone()
                        };
                        let key = lower(&texture_name);
                        let texture = textures
                            .get(&key)
                            .copied()
                            .or_else(|| textures.get(&format!("effectspc.txd|{key}")).copied())
                            .or_else(|| textures.get(&format!("particle.txd|{key}")).copied())
                            .unwrap_or(0);
                        let angle = puff as f32 * 2.3999631 + phase * 0.8;
                        let spread = base_size * (0.35 + phase * 0.85);
                        let lift = up * (phase * base_size * 1.2);
                        let drift =
                            right * angle.cos() * spread + forward * angle.sin() * spread * 0.35;
                        let center = p + drift + lift;
                        let size = base_size * (0.65 + phase * 1.15);
                        let alpha = (1.0 - phase).clamp(0.0, 1.0) * 0.62;
                        draw_textured_billboard(
                            center,
                            right,
                            up,
                            size,
                            texture,
                            Color::new(0.82, 0.88, 0.96, alpha),
                        );
                    }
                }
            }
            _ => {}
        }
    }
    unsafe {
        gl::DepthMask(gl::TRUE);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::Disable(gl::TEXTURE_2D);
        gl::LineWidth(2.0);
        gl::Begin(gl::LINES);
        for (idx, effect) in effects.iter().enumerate() {
            let p = to_mq(effect.position);
            let mut size: f32 = if Some(idx) == selected { 12.0 } else { 7.0 };
            if effect.effect_id == 1 {
                let particle_name = particle_name_from_payload(&effect.payload);
                if let Some(def) = particle_effects
                    .iter()
                    .find(|def| def.name.eq_ignore_ascii_case(&particle_name))
                {
                    size = size.max((def.cull_distance * 0.08).clamp(8.0, 30.0));
                } else {
                    size = size.max(12.0);
                }
            }
            if Some(idx) == selected {
                gl::Color4f(1.0, 0.78, 0.18, 1.0);
            } else if effect.effect_id == 1 {
                gl::Color4f(0.55, 0.72, 0.92, 0.78);
            } else if effect.effect_id == 0 {
                gl::Color4f(1.0, 0.86, 0.38, 0.85);
            } else {
                gl::Color4f(0.22, 0.85, 1.0, 0.85);
            }
            gl::Vertex3f(p.x - size, p.y, p.z);
            gl::Vertex3f(p.x + size, p.y, p.z);
            gl::Vertex3f(p.x, p.y - size, p.z);
            gl::Vertex3f(p.x, p.y + size, p.z);
            gl::Vertex3f(p.x, p.y, p.z - size);
            gl::Vertex3f(p.x, p.y, p.z + size);
        }
        gl::End();
        for effect in effects.iter().filter(|effect| effect.effect_id == 1) {
            let p = to_mq(effect.position);
            let particle_name = particle_name_from_payload(&effect.payload);
            let size = particle_effects
                .iter()
                .find(|def| def.name.eq_ignore_ascii_case(&particle_name))
                .map(|def| (def.cull_distance * 0.08).clamp(8.0, 30.0))
                .unwrap_or(12.0);
            gl::Color4f(0.55, 0.72, 0.92, 0.45);
            gl::Begin(gl::LINE_LOOP);
            for step in 0..24 {
                let angle = step as f32 / 24.0 * std::f32::consts::TAU;
                gl::Vertex3f(p.x + angle.cos() * size, p.y + angle.sin() * size, p.z);
            }
            gl::End();
        }
        if let Some(effect) = selected
            .and_then(|index| effects.get(index))
            .filter(|effect| effect.effect_id == 0)
        {
            gl::Color4f(1.0, 0.58, 0.12, 0.62);
            draw_dff_pointlight_wire_sphere(effect);
        }
        gl::Disable(gl::BLEND);
        gl::Enable(gl::CULL_FACE);
    }
}

fn preload_dff_2dfx_preview_textures(app: &mut AppState, effects: &[Dff2dEffect]) {
    for texture_name in dff_2dfx_preview_texture_names(effects, &app.particle_effects) {
        load_texture_cached(
            &texture_name,
            None,
            &app.texture_files,
            &app.txd_textures,
            &mut app.textures,
            app.options.textures,
        );
    }
}

pub(crate) fn draw_vehicle_2dfx_coronas(app: &mut AppState, effects: &[Dff2dEffect], model: Mat4) {
    let coronas = effects
        .iter()
        .filter(|effect| effect.effect_id == 0)
        // The explicit vehicle Lights toggle controls visibility, so a corona's
        // day/night and far-clip flags do not suppress a manually switched-on
        // headlight in the isolated vehicle preview.
        .map(|effect| {
            let mut effect = transform_dff_2dfx_effect(effect, model);
            if let Some(alpha) = effect.payload.get_mut(3) {
                *alpha = (*alpha).max(192);
            }
            effect
        })
        .collect::<Vec<_>>();
    if coronas.is_empty() {
        return;
    }

    preload_dff_2dfx_preview_textures(app, &coronas);
    let sample = active_timecyc_sample(&app.timecyc);
    let sprite_size = sample.values.get(22).copied().unwrap_or(1.0);
    let sprite_brightness = sample.values.get(23).copied().unwrap_or(1.0);
    let (forward, right) = camera_vectors(&app.camera);
    let up = right.cross(forward).normalize_or_zero();
    unsafe {
        gl::PushAttrib(gl::ALL_ATTRIB_BITS);
        // Vehicle corona origins commonly sit just behind the authored lens
        // face. The normal depth test hides the entire billboard in that case,
        // so draw this manually enabled showcase layer over the lens.
        gl::Disable(gl::DEPTH_TEST);
        gl::DepthMask(gl::FALSE);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::CULL_FACE);
        gl::Disable(gl::ALPHA_TEST);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE);
        for effect in &coronas {
            draw_dff_corona_effect(
                effect,
                &app.textures,
                right,
                up,
                sprite_size,
                sprite_brightness,
            );
        }
        gl::BindTexture(gl::TEXTURE_2D, 0);
        gl::PopAttrib();
    }
}

fn collect_placement_2dfx_effects(
    app: &AppState,
    frustum: &[Plane; 6],
    lod_near: f32,
    lod_far: f32,
) -> Vec<Dff2dEffect> {
    let far = app.options.draw_radius;
    let mode = app.options.lod_mode;
    let mut visible_effects = Vec::<Dff2dEffect>::new();
    for (idx, placement) in app.placements.iter().enumerate() {
        if app
            .element_states
            .get(idx)
            .is_some_and(|state| state.deleted || state.hidden)
        {
            continue;
        }
        let is_lod = placement_is_app_lod(app, placement);
        let draw_this = if is_lod {
            mode != LodMode::DetailOnly
                && (to_mq(placement.pos) - app.camera.pos).length() >= lod_near
                && (to_mq(placement.pos) - app.camera.pos).length() <= lod_far
        } else {
            (to_mq(placement.pos) - app.camera.pos).length() <= far
        };
        if !draw_this {
            continue;
        }
        let Some(mesh) = element_mesh(app, placement) else {
            continue;
        };
        if mesh.effects_2dfx.is_empty() {
            continue;
        }
        let origin = to_mq(placement.pos);
        let radius = (mesh.bounds.max - mesh.bounds.min).length().max(32.0);
        if !sphere_in_frustum(frustum, origin, radius + 256.0) {
            continue;
        }
        let model = placement_matrix(placement);
        let traffic_state = dff_traffic_light_state_for_phase(
            get_time(),
            dff_traffic_light_is_north_south(placement.rot.z),
        );
        visible_effects.extend(
            mesh.effects_2dfx
                .iter()
                .filter(|effect| dff_2dfx_active_at_time(effect, app.timecyc.hour_index))
                .filter(|effect| dff_2dfx_active_for_traffic_preview(effect, traffic_state))
                .map(|effect| transform_dff_2dfx_effect(effect, model))
                .filter(|effect| dff_2dfx_in_preview_range(effect, app.camera.pos)),
        );
    }
    visible_effects
}

fn draw_placement_2dfx_overlay(app: &mut AppState, visible_effects: &[Dff2dEffect]) {
    if visible_effects.is_empty() {
        return;
    }
    preload_dff_2dfx_preview_textures(app, visible_effects);
    let sample = active_timecyc_sample(&app.timecyc);
    let sprite_size = sample.values.get(22).copied().unwrap_or(1.0);
    let sprite_brightness = sample.values.get(23).copied().unwrap_or(1.0);
    draw_dff_2dfx_effects(
        visible_effects,
        None,
        &app.particle_effects,
        &app.textures,
        &app.camera,
        sprite_size,
        sprite_brightness,
        None,
    );
    let selected_effects = app
        .placements
        .get(app.selected)
        .and_then(|placement| {
            element_mesh(app, placement).map(|mesh| {
                let model = placement_matrix(placement);
                let traffic_state = dff_traffic_light_state_for_phase(
                    get_time(),
                    dff_traffic_light_is_north_south(placement.rot.z),
                );
                mesh.effects_2dfx
                    .iter()
                    .filter(|effect| dff_2dfx_active_at_time(effect, app.timecyc.hour_index))
                    .filter(|effect| dff_2dfx_active_for_traffic_preview(effect, traffic_state))
                    .map(|effect| transform_dff_2dfx_effect(effect, model))
                    .collect::<Vec<_>>()
            })
        })
        .unwrap_or_default();
    draw_dff_pointlight_volumes(&selected_effects);
}

fn editing_part_uv_anim<'a>(
    mesh: &'a RenderMesh,
    material_index: usize,
) -> Option<&'a DffUvAnimation> {
    let name = mesh
        .material_animations
        .get(material_index)
        .and_then(|animation| animation.names.first())
        .map(|name| name.trim())
        .filter(|name| !name.is_empty())?;
    mesh.uv_animations
        .iter()
        .find(|animation| animation.name.eq_ignore_ascii_case(name))
}

fn sample_uv_animation_transform(animation: &DffUvAnimation) -> Option<[f32; 6]> {
    if animation.frames.is_empty() {
        return None;
    }
    if animation.frames.len() == 1 {
        return Some(animation.frames[0].uv);
    }
    let duration = if animation.duration > 0.0 {
        animation.duration
    } else {
        animation
            .frames
            .last()
            .map(|frame| frame.time)
            .unwrap_or(1.0)
            .max(0.001)
    };
    let t = (get_time() as f32 % duration).max(0.0);
    let mut prev = &animation.frames[0];
    let mut next = animation.frames.last().unwrap_or(prev);
    for frame in &animation.frames {
        if frame.time <= t {
            prev = frame;
        }
        if frame.time >= t {
            next = frame;
            break;
        }
    }
    let span = (next.time - prev.time).abs();
    let frac = if span > 0.0001 {
        ((t - prev.time) / span).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let mut uv = [0.0; 6];
    for idx in 0..6 {
        uv[idx] = prev.uv[idx] + (next.uv[idx] - prev.uv[idx]) * frac;
    }
    Some(uv)
}

fn transform_uv(uv: V2, matrix: [f32; 6]) -> V2 {
    let angle = matrix[0];
    let (sin, cos) = angle.sin_cos();
    let u = uv.u * matrix[1];
    let v = uv.v * matrix[2];
    V2 {
        u: u * cos - v * sin + matrix[4],
        v: u * sin + v * cos + (1.0 - (matrix[5] + matrix[2])),
    }
}

fn draw_editing_part_dynamic(part: &RenderPart, matrix: [f32; 6]) {
    unsafe {
        gl::Begin(gl::TRIANGLES);
        for vertex in &part.cpu_vertices {
            let uv = transform_uv(vertex.uv, matrix);
            let alpha = part.alpha * vertex.alpha;
            if part.texture != 0 {
                gl::Color4f(vertex.color.x, vertex.color.y, vertex.color.z, alpha);
            } else {
                gl::Color4f(0.78, 0.78, 0.74, alpha);
            }
            gl::Normal3f(vertex.normal.x, vertex.normal.y, vertex.normal.z);
            gl::TexCoord2f(uv.u, uv.v);
            gl::Vertex3f(vertex.pos.x, vertex.pos.y, vertex.pos.z);
        }
        gl::End();
    }
}

fn draw_editing_material_specular_preview(mesh: &RenderMesh, raw: &RawMesh, camera_pos: Vec3) {
    let key_dir = Vec3::new(-0.28, -0.36, 0.89).normalize_or_zero();
    unsafe {
        gl::PushAttrib(
            gl::ENABLE_BIT | gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT | gl::CURRENT_BIT,
        );
        gl::Enable(gl::DEPTH_TEST);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE);
        gl::DepthFunc(gl::LEQUAL);
        gl::DepthMask(gl::FALSE);
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        for part in &mesh.parts {
            let material = raw
                .materials
                .get(part.material_index)
                .copied()
                .unwrap_or_else(default_dff_material);
            let strength = material.specular.clamp(0.0, 4.0) * material.alpha.clamp(0.0, 1.0);
            if strength <= 0.0001 {
                continue;
            }
            gl::Begin(gl::TRIANGLES);
            for vertex in &part.cpu_vertices {
                let position = to_mq(vertex.pos);
                let normal = to_mq(vertex.normal).normalize_or_zero();
                let view = (camera_pos - position).normalize_or_zero();
                let half_vector = (view + key_dir).normalize_or_zero();
                let highlight = normal.dot(half_vector).max(0.0).powf(24.0) * strength * 0.45;
                gl::Color4f(1.0, 1.0, 1.0, highlight.clamp(0.0, 1.0));
                gl::Normal3f(vertex.normal.x, vertex.normal.y, vertex.normal.z);
                gl::Vertex3f(vertex.pos.x, vertex.pos.y, vertex.pos.z);
            }
            gl::End();
        }
        gl::PopAttrib();
    }
}

/// Draw order for the editing preview.
///
/// `Sorted` is the forgiving editor view: every opaque batch first, alpha last
/// with depth writes off, so the model always looks right regardless of how the
/// faces are stored.
///
/// `StoredFaceOrder` reproduces what San Andreas actually does — batches are
/// drawn in the order the DFF stores them and alpha faces write depth. A model
/// whose transparent faces come before the geometry behind them will visibly
/// cull that geometry, which is the whole point: it makes a broken face order
/// obvious in the viewport instead of invisible until the model is in game.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum EditingPreviewFaceOrder {
    Sorted,
    StoredFaceOrder,
}

fn draw_editing_render_mesh_preview(
    mesh: &RenderMesh,
    raw: &RawMesh,
    camera_pos: Vec3,
    selected_material: usize,
    mode: ViewportRenderMode,
    classes: &TextureMaterialClasses,
    txd_name: Option<&str>,
    fallback_material: u8,
    face_order: EditingPreviewFaceOrder,
) {
    unsafe {
        gl::Enable(gl::CULL_FACE);
        gl::LightModeli(gl::LIGHT_MODEL_TWO_SIDE, gl::FALSE as i32);
        // In stored-face-order mode there is a single pass over every batch,
        // sequenced by the first DFF triangle each batch owns.
        let stored_order = face_order == EditingPreviewFaceOrder::StoredFaceOrder;
        let mut sequence: Vec<usize> = (0..mesh.parts.len()).collect();
        if stored_order {
            sequence.sort_by_key(|index| {
                mesh.parts[*index]
                    .face_indices
                    .iter()
                    .copied()
                    .min()
                    .unwrap_or(usize::MAX)
            });
        }
        let passes: Vec<TransparencyMode> = if stored_order {
            vec![TransparencyMode::Opaque]
        } else {
            vec![
                TransparencyMode::Opaque,
                TransparencyMode::Cutout,
                TransparencyMode::Blend,
            ]
        };
        for pass in passes.iter().copied() {
            if !stored_order {
                match pass {
                    TransparencyMode::Blend => {
                        gl::Enable(gl::BLEND);
                        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
                        gl::Disable(gl::ALPHA_TEST);
                        gl::DepthMask(gl::FALSE);
                    }
                    TransparencyMode::Opaque | TransparencyMode::Cutout => {
                        gl::Disable(gl::BLEND);
                        gl::Enable(gl::ALPHA_TEST);
                        gl::AlphaFunc(gl::GREATER, 0.08);
                        gl::DepthMask(gl::TRUE);
                    }
                }
            }
            for index in sequence.iter().copied() {
                let part = &mesh.parts[index];
                if !stored_order && part.transparency != pass {
                    continue;
                }
                if stored_order {
                    // Depth writes stay on for alpha too. That is precisely the
                    // behaviour that makes a badly ordered DFF cull whatever is
                    // behind its transparent faces.
                    match part.transparency {
                        TransparencyMode::Blend => {
                            gl::Enable(gl::BLEND);
                            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
                            gl::Enable(gl::ALPHA_TEST);
                            gl::AlphaFunc(gl::GREATER, 0.0);
                        }
                        TransparencyMode::Opaque | TransparencyMode::Cutout => {
                            gl::Disable(gl::BLEND);
                            gl::Enable(gl::ALPHA_TEST);
                            gl::AlphaFunc(gl::GREATER, 0.08);
                        }
                    }
                    gl::DepthMask(gl::TRUE);
                }
                if mode != ViewportRenderMode::ShadedTextured {
                    gl::Disable(gl::FOG);
                    let classification = collision_classification_color(
                        classes,
                        txd_name,
                        &part.texture_name,
                        part.texture_fingerprint,
                        fallback_material,
                    );
                    draw_render_part_view_mode(part, 1.0, mode, classification);
                    continue;
                }
                if part.use_lighting {
                    gl::Enable(gl::LIGHTING);
                } else {
                    gl::Disable(gl::LIGHTING);
                }
                if part.texture != 0 {
                    gl::Enable(gl::TEXTURE_2D);
                    gl::BindTexture(gl::TEXTURE_2D, part.texture);
                    gl::Color3f(1.0, 1.0, 1.0);
                } else {
                    gl::Disable(gl::TEXTURE_2D);
                    gl::Color3f(0.78, 0.78, 0.74);
                }
                if let Some(matrix) = editing_part_uv_anim(mesh, part.material_index)
                    .and_then(sample_uv_animation_transform)
                {
                    draw_editing_part_dynamic(part, matrix);
                } else {
                    draw_render_part_buffer(part);
                }
            }
        }
        if mode == ViewportRenderMode::ShadedTextured {
            draw_editing_material_specular_preview(mesh, raw, camera_pos);
        }
        gl::Disable(gl::BLEND);
        gl::Enable(gl::ALPHA_TEST);
        gl::DepthMask(gl::TRUE);
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::CULL_FACE);
        gl::LineWidth(2.0);
        gl::Color4f(1.0, 0.92, 0.20, 1.0);
        gl::Begin(gl::LINES);
        for part in mesh
            .parts
            .iter()
            .filter(|part| part.material_index == selected_material)
        {
            for tri in part.cpu_vertices.chunks_exact(3) {
                for (a, b) in [(0, 1), (1, 2), (2, 0)] {
                    let p = tri[a].pos;
                    let q = tri[b].pos;
                    gl::Vertex3f(p.x, p.y, p.z);
                    gl::Vertex3f(q.x, q.y, q.z);
                }
            }
        }
        gl::End();
        gl::Enable(gl::CULL_FACE);
    }
}

fn draw_editing_dff_overlay(mesh: &RenderMesh) {
    unsafe {
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::CULL_FACE);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::DepthMask(gl::FALSE);
        gl::Color4f(0.72, 0.76, 0.80, 0.20);
        gl::Begin(gl::TRIANGLES);
        for part in &mesh.parts {
            for vertex in &part.cpu_vertices {
                gl::Vertex3f(vertex.pos.x, vertex.pos.y, vertex.pos.z);
            }
        }
        gl::End();
        gl::Color4f(0.86, 0.90, 0.94, 0.34);
        gl::LineWidth(1.0);
        gl::Begin(gl::LINES);
        for part in &mesh.parts {
            for tri in part.cpu_vertices.chunks_exact(3) {
                for (a, b) in [(0, 1), (1, 2), (2, 0)] {
                    let p = tri[a].pos;
                    let q = tri[b].pos;
                    gl::Vertex3f(p.x, p.y, p.z);
                    gl::Vertex3f(q.x, q.y, q.z);
                }
            }
        }
        gl::End();
        gl::DepthMask(gl::TRUE);
        gl::Disable(gl::BLEND);
    }
}

fn draw_editing_point_emitter_markers(app: &AppState, dff: &EditingDffState) {
    let groups = editing_dff_point_emitter_debug_groups(app, dff);
    if groups.is_empty() {
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

        for group in groups {
            let scale = (app.camera.pos.distance(group.center) * 0.012).clamp(0.08, 8.0);

            // The cyan wire box is the exact face extent accepted by this
            // group. It makes an unexpectedly large or fragmented grouping
            // immediately visible.
            let corners = [
                vec3(group.min.x, group.min.y, group.min.z),
                vec3(group.max.x, group.min.y, group.min.z),
                vec3(group.max.x, group.max.y, group.min.z),
                vec3(group.min.x, group.max.y, group.min.z),
                vec3(group.min.x, group.min.y, group.max.z),
                vec3(group.max.x, group.min.y, group.max.z),
                vec3(group.max.x, group.max.y, group.max.z),
                vec3(group.min.x, group.max.y, group.max.z),
            ];
            gl::LineWidth(1.0);
            gl::Color4f(0.15, 0.88, 1.0, 0.55);
            gl::Begin(gl::LINES);
            for (a, b) in [
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
            ] {
                gl::Vertex3f(corners[a].x, corners[a].y, corners[a].z);
                gl::Vertex3f(corners[b].x, corners[b].y, corners[b].z);
            }
            gl::End();

            // Magenta center point plus a white three-axis cross identifies
            // the actual point-light origin independently of the group box.
            gl::PointSize((8.0 + (group.faces as f32).ln_1p()).clamp(8.0, 15.0));
            gl::Color4f(1.0, 0.15, 0.85, 1.0);
            gl::Begin(gl::POINTS);
            gl::Vertex3f(group.center.x, group.center.y, group.center.z);
            gl::End();
            gl::LineWidth(2.0);
            gl::Color4f(1.0, 1.0, 1.0, 0.9);
            gl::Begin(gl::LINES);
            for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
                let a = group.center - axis * scale * 0.45;
                let b = group.center + axis * scale * 0.45;
                gl::Vertex3f(a.x, a.y, a.z);
                gl::Vertex3f(b.x, b.y, b.z);
            }
            gl::End();

            // Direction colors: green Up, blue Down, orange Sides. A zero
            // direction draws no ray, matching the bake's skipped work.
            if group.up_strength > 0.0 {
                let end =
                    group.center + Vec3::Z * scale * (1.2 + group.up_strength.sqrt().min(3.0));
                gl::Color4f(0.25, 1.0, 0.25, 0.95);
                gl::Begin(gl::LINES);
                gl::Vertex3f(group.center.x, group.center.y, group.center.z);
                gl::Vertex3f(end.x, end.y, end.z);
                gl::End();
            }
            if group.down_strength > 0.0 {
                let end =
                    group.center - Vec3::Z * scale * (1.2 + group.down_strength.sqrt().min(3.0));
                gl::Color4f(0.25, 0.45, 1.0, 0.95);
                gl::Begin(gl::LINES);
                gl::Vertex3f(group.center.x, group.center.y, group.center.z);
                gl::Vertex3f(end.x, end.y, end.z);
                gl::End();
            }
            if group.sides_strength > 0.0 {
                let length = scale * (1.2 + group.sides_strength.sqrt().min(3.0));
                gl::Color4f(1.0, 0.58, 0.12, 0.95);
                gl::Begin(gl::LINES);
                for axis in [Vec3::X, Vec3::Y] {
                    for sign in [-1.0, 1.0] {
                        let end = group.center + axis * length * sign;
                        gl::Vertex3f(group.center.x, group.center.y, group.center.z);
                        gl::Vertex3f(end.x, end.y, end.z);
                    }
                }
                gl::End();
            }
        }
        gl::PopAttrib();
    }
}

pub(crate) fn draw_editing_preview(app: &mut AppState, _viewport: Rect) -> bool {
    if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_ref() {
        let particle_textures =
            dff_2dfx_preview_texture_names(&dff.raw.effects_2dfx, &app.particle_effects);
        let fracture_textures = dff
            .raw
            .components
            .iter()
            .filter_map(|component| component.breakable.as_ref())
            .flat_map(|breakable| {
                breakable.groups.iter().filter_map(|group| {
                    let name = if group.texture.trim().is_empty() {
                        group.mask.trim()
                    } else {
                        group.texture.trim()
                    };
                    (!name.is_empty()).then(|| name.to_string())
                })
            })
            .collect::<Vec<_>>();
        let txd_context = dff.txd_context.clone();
        for texture_name in particle_textures {
            load_texture_cached(
                &texture_name,
                None,
                &app.texture_files,
                &app.txd_textures,
                &mut app.textures,
                app.options.textures,
            );
        }
        for texture_name in fracture_textures {
            load_texture_cached(
                &texture_name,
                txd_context.as_deref(),
                &app.texture_files,
                &app.txd_textures,
                &mut app.textures,
                app.options.textures,
            );
        }
    }
    let Some(asset) = app.editing.asset.as_ref() else {
        return false;
    };
    match asset {
        EditingAsset::Dff(dff) => {
            if let Some(started_at) = dff.fracture_preview_started_at {
                let elapsed =
                    (get_time() - started_at).clamp(0.0, FRACTURE_PREVIEW_DURATION_SECONDS) as f32;
                let (break_velocity, break_velocity_randomness, _) =
                    editing_dff_fracture_preview_physics(app, &dff.name);
                let (triangles, vertices) = draw_breakable_fracture_preview(
                    &dff.raw,
                    &app.textures,
                    dff.txd_context.as_deref(),
                    dff.selected_breakable_group,
                    elapsed,
                    break_velocity,
                    break_velocity_randomness,
                );
                app.last_drawn_placements = 1;
                app.last_drawn_parts = dff
                    .raw
                    .components
                    .iter()
                    .filter_map(|component| component.breakable.as_ref())
                    .map(|breakable| breakable.groups.len())
                    .sum();
                app.last_drawn_vertices = vertices.max(triangles * 3);
                return true;
            }

            let show_edit_vertices = dff.select_mode == EditingSelectMode::Vertex;
            let show_face_selection = dff.select_mode == EditingSelectMode::Face;
            if let Some(mesh) = dff.preview_mesh.as_ref() {
                draw_editing_render_mesh_preview(
                    mesh,
                    &dff.raw,
                    app.camera.pos,
                    dff.selected_material,
                    app.viewport_render_mode,
                    &app.material_classes,
                    dff.txd_context.as_deref(),
                    app.collision_generation_fallback_material,
                    if app.dff_face_order_preview {
                        EditingPreviewFaceOrder::StoredFaceOrder
                    } else {
                        EditingPreviewFaceOrder::Sorted
                    },
                );
                draw_render_mesh_edges(mesh);
            } else {
                draw_editing_dff_preview(
                    &dff.raw,
                    dff.selected_material,
                    app.viewport_render_mode,
                    &app.material_classes,
                    &app.txd_textures,
                    dff.txd_context.as_deref(),
                    app.collision_generation_fallback_material,
                );
            }
            if !dff.panel_collapsed[DffSection::Fractures as usize] {
                draw_dff_fracture_zone_overlay(&dff.raw, dff.selected_breakable_group);
            }
            draw_dff_face_vertex_overlay(
                &dff.raw,
                dff.selected_face,
                &dff.selected_faces,
                &dff.selected_edges,
                dff.selected_vertex,
                &dff.selected_vertices,
                dff.hovered_vertex,
                app.camera.pos,
                show_edit_vertices,
                show_face_selection,
            );
            draw_editing_point_emitter_markers(app, dff);
            if let Some(cutter) = dff.boolean_box {
                draw_dff_boolean_box(cutter);
            }
            let sample = active_timecyc_sample(&app.timecyc);
            let sprite_size = sample.values.get(22).copied().unwrap_or(1.0);
            let sprite_brightness = sample.values.get(23).copied().unwrap_or(1.0);
            draw_dff_2dfx_effects(
                &dff.raw.effects_2dfx,
                dff.selected_2dfx,
                &app.particle_effects,
                &app.textures,
                &app.camera,
                sprite_size,
                sprite_brightness,
                Some(dff_traffic_light_state_for_phase(get_time(), true)),
            );
            app.last_drawn_placements = 1;
            app.last_drawn_parts = dff
                .preview_mesh
                .as_ref()
                .map(|mesh| mesh.parts.len())
                .unwrap_or_else(|| dff.raw.material_textures.len().max(1));
            app.last_drawn_vertices = dff
                .preview_mesh
                .as_ref()
                .map(|mesh| mesh.parts.iter().map(|part| part.vertices).sum())
                .unwrap_or_else(|| dff.raw.triangles.len() * 3);
            true
        }
        EditingAsset::Col(col) => {
            unsafe {
                gl::Disable(gl::LIGHTING);
                gl::Disable(gl::TEXTURE_2D);
            }
            if col.dff_overlay_visible
                && let Some(mesh) = col.dff_overlay.as_ref()
            {
                draw_editing_dff_overlay(mesh);
            }
            let model = Mat4::IDENTITY.to_cols_array();
            let show_edit_vertices = col.select_mode == EditingSelectMode::Vertex;
            let show_face_selection = col.select_mode == EditingSelectMode::Face;
            draw_collision_mesh_with_options(
                &col.mesh,
                Some(col.selected_face),
                &col.selected_faces,
                &col.selected_edges,
                Some(col.selected_vertex % 3),
                &col.selected_vertices,
                col.hovered_face,
                col.hovered_vertex,
                &model,
                app.camera.pos,
                show_edit_vertices,
                show_edit_vertices,
                show_face_selection,
                !col.editing_shadow,
            );
            let face_highlight = app
                .col_box_face_drag
                .as_ref()
                .map(|d| (d.primitive, d.axis, d.side_is_max, true))
                .or_else(|| {
                    app.col_box_hovered_face
                        .map(|(primitive, ax, side)| (primitive, ax, side, false))
                });
            if !col.editing_shadow
                && let Some((primitive, axis, side_is_max, active)) = face_highlight
            {
                match primitive.kind {
                    CollisionPrimitiveKind::Box => {
                        if let Some(col_box) = col.mesh.boxes.get(primitive.index) {
                            draw_col_box_face_highlight(col_box, axis, side_is_max, active);
                        }
                    }
                    CollisionPrimitiveKind::Cuboid => {
                        if let Some(cuboid) = col.cuboids.get(primitive.index) {
                            draw_col_cuboid_face_highlight(cuboid, axis, side_is_max, active);
                        }
                    }
                    _ => {}
                }
            }
            app.last_drawn_placements = 1;
            app.last_drawn_parts = 1;
            app.last_drawn_vertices = col.mesh.faces.len() * 3;
            true
        }
        EditingAsset::Txd(_) => false,
    }
}

/// Configures LIGHT0 for the viewport. On the Lights tab the selected editor
/// light replaces the generic timecyc key light, giving immediate feedback
/// without changing vertex colors or starting a bake.
pub(crate) unsafe fn configure_viewport_light(app: &AppState, timecyc_sample: &TimecycSample) {
    unsafe {
        let preview_amb = preview_ambient(timecyc_sample);
        let mut ambient = vec3_rgba(preview_amb, 1.0);
        let mut diffuse = vec3_rgba(preview_diffuse(timecyc_sample), 1.0);
        let mut light_ambient = [0.0f32, 0.0, 0.0, 1.0];
        let mut position = [-0.28f32, -0.36, 0.89, 0.0];

        // Always reset persistent fixed-function state when switching between
        // directional and local lights.
        gl::Lightf(gl::LIGHT0, gl::CONSTANT_ATTENUATION, 1.0);
        gl::Lightf(gl::LIGHT0, gl::LINEAR_ATTENUATION, 0.0);
        gl::Lightf(gl::LIGHT0, gl::QUADRATIC_ATTENUATION, 0.0);
        gl::Lightf(gl::LIGHT0, gl::SPOT_CUTOFF, 180.0);
        gl::Lightf(gl::LIGHT0, gl::SPOT_EXPONENT, 0.0);

        if app.active_tab == AppTab::Lights {
            if let Some(light) = app.lights.get(app.selected_light) {
                let effective = light_effective_color(light);
                let intensity = light.intensity.max(0.0);
                let rgb = [
                    effective.x * intensity,
                    effective.y * intensity,
                    effective.z * intensity,
                    1.0,
                ];
                // Keep a small neutral lift so unlit faces remain navigable, but
                // remove the normal timecyc key light from the comparison.
                ambient = [
                    preview_amb.x * 0.15,
                    preview_amb.y * 0.15,
                    preview_amb.z * 0.15,
                    1.0,
                ];
                diffuse = [0.0, 0.0, 0.0, 1.0];
                match light.kind {
                    LightKind::Ambient => {
                        light_ambient = rgb;
                    }
                    LightKind::Directional => {
                        let dir = vec3(light.direction.x, light.direction.y, light.direction.z)
                            .normalize_or_zero();
                        if dir.length_squared() > 0.0001 {
                            // OpenGL's w=0 position points from the scene back
                            // toward the source; stored direction is instead
                            // the forward vector along which light travels.
                            position = [-dir.x, -dir.y, -dir.z, 0.0];
                            diffuse = rgb;
                        }
                    }
                    LightKind::Point | LightKind::Spot | LightKind::Area => {
                        position = [light.position.x, light.position.y, light.position.z, 1.0];
                        diffuse = rgb;
                        let radius = light.radius.max(1.0);
                        // Fast approximation of the baker's smooth radius fade.
                        gl::Lightf(gl::LIGHT0, gl::LINEAR_ATTENUATION, 2.0 / radius);
                        gl::Lightf(
                            gl::LIGHT0,
                            gl::QUADRATIC_ATTENUATION,
                            4.0 / (radius * radius),
                        );
                        if matches!(light.kind, LightKind::Spot) {
                            let dir = vec3(light.direction.x, light.direction.y, light.direction.z)
                                .normalize_or_zero();
                            if dir.length_squared() > 0.0001 {
                                let spot_direction = [dir.x, dir.y, dir.z];
                                gl::Lightfv(
                                    gl::LIGHT0,
                                    gl::SPOT_DIRECTION,
                                    spot_direction.as_ptr(),
                                );
                                gl::Lightf(gl::LIGHT0, gl::SPOT_CUTOFF, 36.0);
                                gl::Lightf(gl::LIGHT0, gl::SPOT_EXPONENT, 1.0);
                            }
                        }
                    }
                }
            }
        }

        gl::Lightfv(gl::LIGHT0, gl::POSITION, position.as_ptr());
        gl::Lightfv(gl::LIGHT0, gl::AMBIENT, light_ambient.as_ptr());
        gl::Lightfv(gl::LIGHT0, gl::DIFFUSE, diffuse.as_ptr());
        gl::Lightfv(gl::LIGHT0, gl::SPECULAR, PREVIEW_SPECULAR.as_ptr());
        gl::LightModelfv(gl::LIGHT_MODEL_AMBIENT, ambient.as_ptr());
    }
}

/// Reproduces SA PC's two additive timecycle colour-filter passes. Both passes
/// sample the same saved, unfiltered framebuffer rather than feeding the first
/// result into the second pass.
pub(crate) unsafe fn apply_sa_timecyc_color_filter(
    app: &mut AppState,
    viewport: Rect,
    sample: &TimecycSample,
) {
    unsafe {
        let width = viewport.w.max(1.0) as i32;
        let height = viewport.h.max(1.0) as i32;
        let source_x = viewport.x as i32;
        let source_y = (screen_height() - viewport.y - viewport.h) as i32;
        if app.postfx_preview_texture == 0 {
            gl::GenTextures(1, &mut app.postfx_preview_texture);
        }
        if app.postfx_preview_texture == 0 {
            return;
        }

        gl::PushAttrib(
            gl::ENABLE_BIT
                | gl::COLOR_BUFFER_BIT
                | gl::DEPTH_BUFFER_BIT
                | gl::CURRENT_BIT
                | gl::TRANSFORM_BIT
                | gl::TEXTURE_BIT,
        );
        gl::BindTexture(gl::TEXTURE_2D, app.postfx_preview_texture);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
        if app.postfx_preview_size != (width, height) {
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA8 as i32,
                width,
                height,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                std::ptr::null(),
            );
            app.postfx_preview_size = (width, height);
        }
        gl::CopyTexSubImage2D(gl::TEXTURE_2D, 0, 0, 0, source_x, source_y, width, height);

        gl::Disable(gl::DEPTH_TEST);
        gl::Disable(gl::LIGHTING);
        gl::Enable(gl::TEXTURE_2D);
        gl::Disable(gl::FOG);
        gl::Disable(gl::ALPHA_TEST);
        gl::Disable(gl::CULL_FACE);
        gl::DepthMask(gl::FALSE);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE);

        gl::MatrixMode(gl::PROJECTION);
        gl::PushMatrix();
        gl::LoadIdentity();
        gl::MatrixMode(gl::MODELVIEW);
        gl::PushMatrix();
        gl::LoadIdentity();

        for pass in [sample.postfx1, sample.postfx2] {
            if pass[3] <= 0.0 {
                continue;
            }
            gl::Color4f(pass[0], pass[1], pass[2], pass[3]);
            gl::Begin(gl::QUADS);
            gl::TexCoord2f(0.0, 0.0);
            gl::Vertex2f(-1.0, -1.0);
            gl::TexCoord2f(1.0, 0.0);
            gl::Vertex2f(1.0, -1.0);
            gl::TexCoord2f(1.0, 1.0);
            gl::Vertex2f(1.0, 1.0);
            gl::TexCoord2f(0.0, 1.0);
            gl::Vertex2f(-1.0, 1.0);
            gl::End();
        }

        gl::PopMatrix();
        gl::MatrixMode(gl::PROJECTION);
        gl::PopMatrix();
        gl::MatrixMode(gl::MODELVIEW);
        gl::PopAttrib();
    }
}

pub(crate) fn draw_scene(app: &mut AppState, viewport: Rect) {
    let viewport = if app.active_tab == AppTab::Editing {
        editing_center_rect()
    } else if app.active_tab == AppTab::Vehicles {
        vehicle_preview_viewport_rect(app)
    } else {
        viewport
    };
    app.last_drawn_placements = 0;
    app.last_drawn_parts = 0;
    app.last_drawn_vertices = 0;
    let mut internal = unsafe { get_internal_gl() };
    internal.flush();
    drop(internal);

    let view_x = viewport.x as i32;
    let view_y = (screen_height() - viewport.y - viewport.h) as i32;
    let view_w = viewport.w.max(1.0) as i32;
    let view_h = viewport.h.max(1.0) as i32;
    let timecyc_sample = active_timecyc_sample(&app.timecyc).clone();
    let sky = timecyc_sample.sky_top;
    let fog_color = v3_mix(timecyc_sample.sky_bottom, timecyc_sample.sky_top, 0.55);
    unsafe {
        gl::UseProgram(0);
        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, 0);
        gl::ActiveTexture(gl::TEXTURE0);
        gl::Viewport(view_x, view_y, view_w, view_h);
        gl::Enable(gl::SCISSOR_TEST);
        gl::Scissor(view_x, view_y, view_w, view_h);
        gl::ClearColor(sky.x, sky.y, sky.z, 1.0);
        gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        if app.options.msaa_samples > 1 {
            gl::Enable(gl::MULTISAMPLE);
        } else {
            gl::Disable(gl::MULTISAMPLE);
        }
    }
    if !app.options.render {
        reset_gl_for_ui();
        return;
    }
    let (forward, _) = camera_vectors(&app.camera);
    // Small audit targets can fit entirely inside the regular one-unit near
    // plane as the reviewer approaches them, which looks like an LOD handoff
    // failure. Keep the normal world precision elsewhere and only pull the
    // audit near plane inward.
    let near_clip = if app.active_tab == AppTab::LodAudit {
        0.25
    } else {
        1.0
    };
    let projection = Mat4::perspective_rh_gl(
        70.0_f32.to_radians(),
        viewport.w / viewport.h.max(1.0),
        near_clip,
        16000.0,
    );
    let view = Mat4::look_at_rh(app.camera.pos, app.camera.pos + forward, Vec3::Z);
    let preview_transform = if app.active_tab == AppTab::Preview
        && app.global_transform.preview
        && app.global_transform.transform_elements
    {
        global_transform_matrix(app.global_transform)
    } else {
        Mat4::IDENTITY
    };
    let scene_view = view * preview_transform;
    let frustum = frustum_planes(projection * scene_view);
    let mut max_attribs = 0;
    unsafe {
        gl::Enable(gl::DEPTH_TEST);
        gl::DepthFunc(gl::LEQUAL);
        // Backface culling halves fragment work on the big LC scene. RenderWare
        // geometry is wound clockwise, so cull back faces with CW as front-facing.
        gl::Enable(gl::CULL_FACE);
        gl::CullFace(gl::BACK);
        gl::FrontFace(gl::CW);
        gl::Disable(gl::BLEND);
        gl::Enable(gl::ALPHA_TEST);
        gl::AlphaFunc(gl::GREATER, 0.08);
        gl::DepthMask(gl::TRUE);
        gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
        gl::Enable(gl::LIGHTING);
        gl::Enable(gl::LIGHT0);
        gl::Enable(gl::COLOR_MATERIAL);
        gl::ColorMaterial(gl::FRONT_AND_BACK, gl::AMBIENT_AND_DIFFUSE);
        if app.fog_strength <= 0.001 {
            gl::Disable(gl::FOG);
        } else {
            let fog_scale = (1.0 / app.fog_strength).clamp(0.25, 6.0);
            let base_start = timecyc_sample
                .fog_start
                .max(0.0)
                .min(timecyc_sample.far_clip * 0.55);
            let base_end = timecyc_sample
                .far_clip
                .max(app.options.draw_radius * 0.35)
                .min(app.options.draw_radius.max(600.0));
            let fog_start = base_start * fog_scale;
            let fog_end = (base_end * fog_scale).max(fog_start + 64.0);
            gl::Enable(gl::FOG);
            gl::Fogi(gl::FOG_MODE, gl::LINEAR as i32);
            let fog_rgba = vec3_rgba(fog_color, 1.0);
            gl::Fogfv(gl::FOG_COLOR, fog_rgba.as_ptr());
            gl::Fogf(gl::FOG_START, fog_start);
            gl::Fogf(gl::FOG_END, fog_end);
        }
        // Single-sided lighting: with culling on, back faces are gone, so the
        // two-sided lighting pass is wasted work.
        gl::LightModeli(gl::LIGHT_MODEL_TWO_SIDE, gl::FALSE as i32);
        let preview_amb = preview_ambient(&timecyc_sample);
        let material = vec3_rgba(preview_material(&timecyc_sample), 1.0);
        let emission = if app.active_tab == AppTab::Lights {
            [0.0, 0.0, 0.0, 1.0]
        } else {
            vec3_rgba(v3_scale(preview_amb, 0.10), 1.0)
        };
        gl::MatrixMode(gl::PROJECTION);
        gl::LoadMatrixf(projection.to_cols_array().as_ptr());
        gl::MatrixMode(gl::MODELVIEW);
        gl::LoadMatrixf(scene_view.to_cols_array().as_ptr());
        configure_viewport_light(app, &timecyc_sample);
        gl::Materialfv(
            gl::FRONT_AND_BACK,
            gl::AMBIENT_AND_DIFFUSE,
            material.as_ptr(),
        );
        gl::Materialfv(gl::FRONT_AND_BACK, gl::SPECULAR, PREVIEW_SPECULAR.as_ptr());
        gl::Materialfv(gl::FRONT_AND_BACK, gl::EMISSION, emission.as_ptr());
        gl::GetIntegerv(gl::MAX_VERTEX_ATTRIBS, &mut max_attribs);
        for index in 0..max_attribs.max(0).min(32) {
            gl::DisableVertexAttribArray(index as u32);
        }
        gl::DisableClientState(gl::COLOR_ARRAY);
        gl::DisableClientState(gl::VERTEX_ARRAY);
        gl::DisableClientState(gl::NORMAL_ARRAY);
        gl::DisableClientState(gl::TEXTURE_COORD_ARRAY);
    }

    if app.active_tab == AppTab::Collisions {
        draw_collision_world(app, &frustum);
        draw_transform_gizmo(app);
        draw_water_planes(app);
        draw_editor_outlines(app);
        reset_gl_for_ui();
        return;
    }

    if app.active_tab == AppTab::Editing {
        draw_editing_preview(app, viewport);
        draw_transform_gizmo(app);
        reset_gl_for_ui();
        return;
    }

    if app.active_tab == AppTab::Vehicles {
        draw_vehicle_preview(app);
        reset_gl_for_ui();
        return;
    }

    // Only the part budget forces us off the fast VBO path; the vertex budget is
    // applied inside both paths so it stays active by default.
    let part_budgeted = app.options.part_budget != usize::MAX;
    let mut vertex_budget = app.options.vertex_budget;
    let far = app.options.draw_radius;
    if get_time() < app.render_settle_until {
        vertex_budget = vertex_budget.min(SETTLE_VERTEX_BUDGET);
    }
    let lod_far = app.options.lod_radius.max(far);
    let mode = app.options.lod_mode;
    let cam = preview_transform.inverse().transform_point3(app.camera.pos);
    if app.active_tab == AppTab::LodAudit {
        let (placements, parts, vertices) =
            draw_lod_audit_scene(app, &frustum, cam, app.options.part_budget, vertex_budget);
        app.last_drawn_placements = placements;
        app.last_drawn_parts = parts;
        app.last_drawn_vertices = vertices;
        draw_editor_outlines(app);
        reset_gl_for_ui();
        return;
    }
    // LOD band: detail meshes draw within `far`; their LOD stand-ins fill the
    // gap from `far` out to `lod_far`. A small overlap avoids a visible seam.
    let lod_near = match mode {
        LodMode::Swap => (far - 256.0).max(0.0),
        LodMode::ShowAll => 0.0,
        LodMode::DetailOnly => f32::MAX, // never drawn
    };
    let draw_lod = mode != LodMode::DetailOnly && !app.lod_world_cells.is_empty()
        || mode != LodMode::DetailOnly && !app.lod_scene_cells.is_empty();

    if app.active_tab == AppTab::Preview
        && app.viewport_render_mode != ViewportRenderMode::ShadedTextured
    {
        unsafe {
            gl::Disable(gl::FOG);
        }
        let (placements, parts, vertices) = draw_preview_view_mode_scene(
            app,
            &frustum,
            cam,
            far,
            lod_near,
            lod_far,
            app.options.part_budget,
            vertex_budget,
        );
        app.last_drawn_placements = placements;
        app.last_drawn_parts = parts;
        app.last_drawn_vertices = vertices;
        draw_global_transform_bounds(app);
        draw_editor_outlines(app);
        reset_gl_for_ui();
        return;
    }

    if app.options.fast_vbo && !part_budgeted && !app.world_cells.is_empty() {
        let mut dp = 0usize;
        let mut dpa = 0usize;
        let mut dv = 0usize;
        let immediate = app.options.vbo_immediate;
        let vbud = vertex_budget;
        draw_world_cell_set(
            &app.world_cells,
            cam,
            0.0,
            far,
            &frustum,
            immediate,
            vbud,
            &mut dp,
            &mut dpa,
            &mut dv,
        );
        if draw_lod {
            draw_world_cell_set(
                &app.lod_world_cells,
                cam,
                lod_near,
                lod_far,
                &frustum,
                immediate,
                vbud,
                &mut dp,
                &mut dpa,
                &mut dv,
            );
        }
        app.last_drawn_placements += dp;
        app.last_drawn_parts += dpa;
        app.last_drawn_vertices += dv;
        unsafe {
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindTexture(gl::TEXTURE_2D, 0);
            gl::Disable(gl::TEXTURE_2D);
            gl::DisableClientState(gl::COLOR_ARRAY);
            gl::DisableClientState(gl::TEXTURE_COORD_ARRAY);
            gl::DisableClientState(gl::NORMAL_ARRAY);
            gl::DisableClientState(gl::VERTEX_ARRAY);
            let (local_dp, local_dpa, local_dv) = draw_selected_local_lod(app, &frustum, lod_near);
            app.last_drawn_placements += local_dp;
            app.last_drawn_parts += local_dpa;
            app.last_drawn_vertices += local_dv;
            let visible_2dfx = collect_placement_2dfx_effects(app, &frustum, lod_near, lod_far);
            draw_simulation_objects(app, &frustum);
            draw_dff_pointlight_preview(
                app,
                &visible_2dfx,
                &frustum,
                cam,
                far,
                lod_near,
                lod_far,
                draw_lod,
            );
            draw_water_planes(app);
            draw_placement_2dfx_overlay(app, &visible_2dfx);
            draw_global_transform_bounds(app);
            apply_sa_timecyc_color_filter(app, viewport, &timecyc_sample);
            draw_race_markers(app);
            draw_missing_texture_review_overlays(app);
            draw_editor_outlines(app);
            draw_vertex_paint_marker(app, viewport);
            for index in 0..max_attribs.max(0).min(4) {
                gl::EnableVertexAttribArray(index as u32);
            }
        }
        reset_gl_for_ui();
        return;
    }

    let mut dp = 0usize;
    let mut dpa = 0usize;
    let mut dv = 0usize;
    draw_scene_cell_set(
        &app.scene_cells,
        cam,
        0.0,
        far,
        &frustum,
        app.options.part_budget,
        vertex_budget,
        &mut dp,
        &mut dpa,
        &mut dv,
    );
    if draw_lod {
        draw_scene_cell_set(
            &app.lod_scene_cells,
            cam,
            lod_near,
            lod_far,
            &frustum,
            app.options.part_budget,
            vertex_budget,
            &mut dp,
            &mut dpa,
            &mut dv,
        );
    }
    app.last_drawn_placements += dp;
    app.last_drawn_parts += dpa;
    app.last_drawn_vertices += dv;
    let (local_dp, local_dpa, local_dv) = draw_selected_local_lod(app, &frustum, lod_near);
    app.last_drawn_placements += local_dp;
    app.last_drawn_parts += local_dpa;
    app.last_drawn_vertices += local_dv;

    let visible_2dfx = collect_placement_2dfx_effects(app, &frustum, lod_near, lod_far);
    draw_simulation_objects(app, &frustum);
    draw_dff_pointlight_preview(
        app,
        &visible_2dfx,
        &frustum,
        cam,
        far,
        lod_near,
        lod_far,
        draw_lod,
    );
    draw_water_planes(app);
    draw_placement_2dfx_overlay(app, &visible_2dfx);
    draw_global_transform_bounds(app);
    unsafe { apply_sa_timecyc_color_filter(app, viewport, &timecyc_sample) };
    draw_race_markers(app);
    draw_missing_texture_review_overlays(app);
    draw_editor_outlines(app);
    draw_vertex_paint_marker(app, viewport);
    reset_gl_for_ui();
}

/// Draws the currently selected race track in the 3D viewport: the start point,
/// gameplay checkpoints (as ground rings sized by their radius), the dense
/// preview overlay path, and the full track path.
pub(crate) fn draw_race_markers(app: &AppState) {
    if app.active_tab != AppTab::Race {
        return;
    }
    if app.race.tracks.is_empty() {
        return;
    }
    let selected_idx = app.race.selected_track;
    let sel = app.race.selected_point;
    let mode = app.race.place_mode;
    unsafe {
        gl::PushAttrib(gl::ENABLE_BIT | gl::CURRENT_BIT | gl::LINE_BIT | gl::DEPTH_BUFFER_BIT);
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::ALPHA_TEST);
        gl::Disable(gl::CULL_FACE);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::DepthMask(gl::FALSE);
        gl::Disable(gl::DEPTH_TEST);

        // Draw every track so the whole race layout is visible at once. The
        // non-selected tracks are dimmed and get no editable handles; the
        // selected track is drawn bright with handles + selection highlight.
        for (t_idx, track) in app.race.tracks.iter().enumerate() {
            let is_sel = t_idx == selected_idx;
            draw_race_track_gl(track, is_sel, mode, if is_sel { sel } else { None });
        }

        gl::PointSize(1.0);
        gl::LineWidth(1.0);
        gl::Enable(gl::DEPTH_TEST);
        gl::DepthMask(gl::TRUE);
        gl::PopAttrib();
    }
}

/// Draws one race track's 3D markers: connected checkpoint route, checkpoint
/// rings + pillars, dense overlay/path polylines, and a start flag icon.
/// Non-selected tracks are drawn dimmer (`is_sel == false`) with no handles.
fn draw_race_track_gl(track: &RaceTrack, is_sel: bool, mode: RacePlaceMode, sel: Option<usize>) {
    // Overall alpha multiplier so background tracks recede visually.
    let a = if is_sel { 1.0 } else { 0.35 };
    unsafe {
        // Full track path (faint blue polyline).
        if track.path.len() >= 2 {
            gl::LineWidth(1.5);
            gl::Color4f(0.30, 0.55, 1.0, 0.55 * a);
            gl::Begin(gl::LINE_STRIP);
            for p in &track.path {
                gl::Vertex3f(p.x, p.y, p.z + 0.3);
            }
            gl::End();
        }

        // Dense overlay outline (green polyline — this is what the radar draws).
        if track.overlay.len() >= 2 {
            gl::LineWidth(2.5);
            gl::Color4f(0.20, 1.0, 0.45, 0.85 * a);
            gl::Begin(gl::LINE_STRIP);
            for p in &track.overlay {
                gl::Vertex3f(p.x, p.y, p.z + 0.4);
            }
            gl::End();
        }

        // Overlay / path point handles (selected track only).
        if is_sel {
            let draw_handles = |pts: &[V3], base: (f32, f32, f32), active: bool| {
                gl::PointSize(if active { 8.0 } else { 5.0 });
                gl::Begin(gl::POINTS);
                for p in pts {
                    gl::Color4f(base.0, base.1, base.2, 1.0);
                    gl::Vertex3f(p.x, p.y, p.z + 0.5);
                }
                gl::End();
            };
            draw_handles(&track.path, (0.30, 0.55, 1.0), mode == RacePlaceMode::Path);
            draw_handles(
                &track.overlay,
                (0.20, 1.0, 0.45),
                mode == RacePlaceMode::Overlay,
            );
        }

        // Gameplay checkpoints: connect the dots (start -> cp0 -> cp1 -> ...).
        let start = track.start;
        let has_start = start != V3::default();
        if !track.checkpoints.is_empty() {
            let route_z = 3.0;
            gl::LineWidth(if is_sel { 5.0 } else { 3.0 });
            gl::Color4f(0.05, 0.02, 0.0, 0.75 * a);
            gl::Begin(gl::LINE_STRIP);
            if has_start {
                gl::Vertex3f(start.x, start.y, start.z + route_z - 0.15);
            }
            for cp in &track.checkpoints {
                gl::Vertex3f(cp.pos.x, cp.pos.y, cp.pos.z + route_z - 0.15);
            }
            gl::End();

            gl::LineWidth(if is_sel { 3.5 } else { 2.0 });
            gl::Color4f(1.0, 0.88, 0.12, 0.95 * a);
            gl::Begin(gl::LINE_STRIP);
            if has_start {
                gl::Vertex3f(start.x, start.y, start.z + route_z);
            }
            for cp in &track.checkpoints {
                gl::Vertex3f(cp.pos.x, cp.pos.y, cp.pos.z + route_z);
            }
            gl::End();
        }

        // Checkpoint icons: ground ring + vertical pillar so each is legible.
        for (idx, cp) in track.checkpoints.iter().enumerate() {
            let selected = is_sel && mode == RacePlaceMode::Checkpoint && sel == Some(idx);
            let ring_z = cp.pos.z + 1.0;
            let post_top = cp.pos.z + if selected { 24.0 } else { 16.0 };
            gl::LineWidth(if selected { 5.0 } else { 3.5 });
            gl::Color4f(0.0, 0.0, 0.0, 0.85 * a.max(0.75));
            gl::Begin(gl::LINE_LOOP);
            let segments = 28;
            for s in 0..segments {
                let ang = s as f32 / segments as f32 * std::f32::consts::TAU;
                gl::Vertex3f(
                    cp.pos.x + ang.cos() * cp.r,
                    cp.pos.y + ang.sin() * cp.r,
                    ring_z - 0.08,
                );
            }
            gl::End();
            gl::LineWidth(if selected { 3.5 } else { 2.25 });
            gl::Color4f(
                1.0,
                if selected { 1.0 } else { 0.68 },
                if selected { 0.25 } else { 0.0 },
                1.0 * a.max(0.7),
            );
            gl::Begin(gl::LINE_LOOP);
            for s in 0..segments {
                let ang = s as f32 / segments as f32 * std::f32::consts::TAU;
                gl::Vertex3f(
                    cp.pos.x + ang.cos() * cp.r,
                    cp.pos.y + ang.sin() * cp.r,
                    ring_z,
                );
            }
            gl::End();

            // Center dot and vertical pillar make checkpoints visible from
            // oblique camera angles, where ground rings can visually flatten.
            gl::PointSize(if selected { 14.0 } else { 10.0 });
            gl::Begin(gl::POINTS);
            gl::Color4f(1.0, 0.95, 0.15, 1.0 * a.max(0.7));
            gl::Vertex3f(cp.pos.x, cp.pos.y, post_top);
            gl::End();
            // Vertical pillar rising from the checkpoint centre (an icon post).
            gl::LineWidth(if selected { 4.0 } else { 2.5 });
            gl::Color4f(1.0, 0.82, 0.08, 0.95 * a.max(0.7));
            gl::Begin(gl::LINES);
            gl::Vertex3f(cp.pos.x, cp.pos.y, ring_z);
            gl::Vertex3f(cp.pos.x, cp.pos.y, post_top);
            gl::End();
        }

        // Start icon: magenta pole with a pennant flag near the top.
        if has_start || (is_sel && mode == RacePlaceMode::Start) {
            let s = start;
            gl::LineWidth(3.0);
            gl::Color4f(1.0, 0.15, 0.85, 1.0 * a.max(0.6));
            // Ground cross.
            gl::Begin(gl::LINES);
            gl::Vertex3f(s.x - 6.0, s.y, s.z + 0.4);
            gl::Vertex3f(s.x + 6.0, s.y, s.z + 0.4);
            gl::Vertex3f(s.x, s.y - 6.0, s.z + 0.4);
            gl::Vertex3f(s.x, s.y + 6.0, s.z + 0.4);
            // Vertical pole.
            gl::Vertex3f(s.x, s.y, s.z);
            gl::Vertex3f(s.x, s.y, s.z + 14.0);
            gl::End();
            // Pennant flag (filled triangle) hanging off the top of the pole.
            gl::Color4f(1.0, 0.30, 0.90, 0.85 * a.max(0.6));
            gl::Begin(gl::TRIANGLES);
            gl::Vertex3f(s.x, s.y, s.z + 14.0);
            gl::Vertex3f(s.x, s.y, s.z + 9.5);
            gl::Vertex3f(s.x + 7.0, s.y, s.z + 11.75);
            gl::End();
        }
    }
}

pub(crate) fn draw_water_planes(app: &AppState) {
    if app.water_planes.is_empty() {
        return;
    }
    unsafe {
        gl::MatrixMode(gl::MODELVIEW);
        gl::PushMatrix();
        if app.active_tab == AppTab::Preview && app.global_transform.preview {
            let transform = global_transform_matrix(app.global_transform);
            let current = if app.global_transform.transform_elements {
                transform
            } else {
                Mat4::IDENTITY
            };
            let desired = if app.global_transform.transform_water {
                transform
            } else {
                Mat4::IDENTITY
            };
            let adjustment = current.inverse() * desired;
            gl::MultMatrixf(adjustment.to_cols_array().as_ptr());
        }
        gl::PushAttrib(gl::ENABLE_BIT | gl::CURRENT_BIT | gl::LINE_BIT | gl::DEPTH_BUFFER_BIT);
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::ALPHA_TEST);
        gl::Disable(gl::CULL_FACE);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::DepthMask(gl::FALSE);
        gl::ShadeModel(gl::SMOOTH);
        let preview_water = app.active_tab == AppTab::Preview;
        let edit_water = app.active_tab == AppTab::Water;
        let time = get_time() as f32;
        for (idx, plane) in app.water_planes.iter().enumerate() {
            let selected = edit_water
                && (idx == app.selected_water || app.selected_water_planes.contains(&idx));
            let hovered = edit_water && app.hovered_water == Some(idx);
            gl::Begin(gl::TRIANGLE_STRIP);
            for corner in [0usize, 1, 2, 3] {
                let p = plane.corners[corner].pos;
                if preview_water {
                    let wave = ((p.x + p.y) * 0.012 + time * 1.7).sin() * 0.5 + 0.5;
                    gl::Color4f(
                        0.05 + wave * 0.04,
                        0.25 + wave * 0.08,
                        0.46 + wave * 0.14,
                        0.58,
                    );
                } else if selected {
                    gl::Color4f(0.18, 0.62, 1.0, 0.42);
                } else if hovered {
                    gl::Color4f(0.24, 0.78, 1.0, 0.34);
                } else {
                    gl::Color4f(0.10, 0.42, 0.84, 0.24);
                }
                gl::Vertex3f(p.x, p.y, p.z);
            }
            gl::End();
            if !edit_water {
                continue;
            }
            gl::LineWidth(if selected { 2.5 } else { 1.2 });
            if selected {
                gl::Color4f(0.45, 0.86, 1.0, 0.90);
            } else if hovered {
                gl::Color4f(0.55, 0.95, 1.0, 0.74);
            } else {
                gl::Color4f(0.34, 0.68, 1.0, 0.48);
            }
            gl::Begin(gl::LINE_LOOP);
            for corner in [0usize, 1, 3, 2] {
                let p = plane.corners[corner].pos;
                gl::Vertex3f(p.x, p.y, p.z + 0.2);
            }
            gl::End();
            if let Some((edge_idx, edge)) = app.hovered_water_edge {
                if edge_idx == idx {
                    let (a_idx, b_idx) = match edge {
                        WaterEdge::South => (0usize, 1usize),
                        WaterEdge::North => (2usize, 3usize),
                        WaterEdge::West => (0usize, 2usize),
                        WaterEdge::East => (1usize, 3usize),
                    };
                    let a = plane.corners[a_idx].pos;
                    let b = plane.corners[b_idx].pos;
                    gl::LineWidth(5.0);
                    gl::Color4f(0.98, 0.98, 0.42, 0.96);
                    gl::Begin(gl::LINES);
                    gl::Vertex3f(a.x, a.y, a.z + 0.45);
                    gl::Vertex3f(b.x, b.y, b.z + 0.45);
                    gl::End();
                }
            }
        }
        gl::DepthMask(gl::TRUE);
        gl::PopAttrib();
        gl::PopMatrix();
    }
}

/// Draw the fixed playable-map perimeter while previewing a global transform.
/// This deliberately cancels the element preview matrix so the guide remains
/// at the authoritative -3000..3000 world coordinates.
fn draw_global_transform_bounds(app: &AppState) {
    if app.active_tab != AppTab::Preview || !app.global_transform.preview {
        return;
    }
    unsafe {
        gl::MatrixMode(gl::MODELVIEW);
        gl::PushMatrix();
        if app.global_transform.transform_elements {
            let inverse = global_transform_matrix(app.global_transform).inverse();
            gl::MultMatrixf(inverse.to_cols_array().as_ptr());
        }
        gl::PushAttrib(gl::ENABLE_BIT | gl::CURRENT_BIT | gl::LINE_BIT | gl::DEPTH_BUFFER_BIT);
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::ALPHA_TEST);
        gl::Disable(gl::CULL_FACE);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::Disable(gl::DEPTH_TEST);
        gl::DepthMask(gl::FALSE);

        const LIMIT: f32 = 3000.0;
        const Z: f32 = 2.5;
        gl::LineWidth(5.0);
        gl::Color4f(1.0, 0.34, 0.05, 0.95);
        gl::Begin(gl::LINE_LOOP);
        gl::Vertex3f(-LIMIT, -LIMIT, Z);
        gl::Vertex3f(LIMIT, -LIMIT, Z);
        gl::Vertex3f(LIMIT, LIMIT, Z);
        gl::Vertex3f(-LIMIT, LIMIT, Z);
        gl::End();

        // Tall corner ticks keep the boundary legible from an oblique camera.
        gl::LineWidth(3.0);
        gl::Color4f(1.0, 0.72, 0.12, 0.9);
        gl::Begin(gl::LINES);
        for (x, y) in [
            (-LIMIT, -LIMIT),
            (LIMIT, -LIMIT),
            (LIMIT, LIMIT),
            (-LIMIT, LIMIT),
        ] {
            gl::Vertex3f(x, y, Z - 80.0);
            gl::Vertex3f(x, y, Z + 160.0);
        }
        gl::End();

        gl::DepthMask(gl::TRUE);
        gl::PopAttrib();
        gl::PopMatrix();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point_light_effect(position: V3, radius: f32, color: [u8; 3]) -> Dff2dEffect {
        let mut payload = vec![0u8; 12];
        payload[..3].copy_from_slice(&color);
        payload[8..12].copy_from_slice(&radius.to_le_bytes());
        Dff2dEffect {
            position,
            effect_id: 0,
            payload,
        }
    }

    fn traffic_light_effect(color: [u8; 3]) -> Dff2dEffect {
        let mut effect = point_light_effect(V3::default(), 18.0, color);
        effect
            .payload
            .resize(DFF_2DFX_CORONA_SHOW_MODE_OFFSET + 1, 0);
        effect.payload[DFF_2DFX_CORONA_SHOW_MODE_OFFSET] = DFF_2DFX_TRAFFIC_LIGHT_SHOW_MODE;
        effect
    }

    #[test]
    fn traffic_light_show_mode_classifies_stock_colors() {
        assert_eq!(
            dff_2dfx_traffic_light_state(&traffic_light_effect([255, 0, 0])),
            Some(DffTrafficLightState::Red)
        );
        assert_eq!(
            dff_2dfx_traffic_light_state(&traffic_light_effect([255, 142, 0])),
            Some(DffTrafficLightState::Yellow)
        );
        assert_eq!(
            dff_2dfx_traffic_light_state(&traffic_light_effect([0, 255, 0])),
            Some(DffTrafficLightState::Green)
        );
        assert_eq!(
            dff_2dfx_traffic_light_state(&point_light_effect(V3::default(), 18.0, [255, 0, 0])),
            None
        );
    }

    #[test]
    fn traffic_light_preview_matches_sa_phase_boundaries() {
        assert_eq!(
            dff_traffic_light_state_for_phase(0.0, true),
            DffTrafficLightState::Green
        );
        assert_eq!(
            dff_traffic_light_state_for_phase(10.0, true),
            DffTrafficLightState::Yellow
        );
        assert_eq!(
            dff_traffic_light_state_for_phase(12.0, true),
            DffTrafficLightState::Red
        );
        assert_eq!(
            dff_traffic_light_state_for_phase(0.0, false),
            DffTrafficLightState::Red
        );
        assert_eq!(
            dff_traffic_light_state_for_phase(12.0, false),
            DffTrafficLightState::Green
        );
        assert_eq!(
            dff_traffic_light_state_for_phase(22.0, false),
            DffTrafficLightState::Yellow
        );
        assert_eq!(
            dff_traffic_light_state_for_phase(24.0, false),
            DffTrafficLightState::Red
        );
    }

    #[test]
    fn traffic_light_direction_matches_sa_heading_bands() {
        assert!(!dff_traffic_light_is_north_south(0.0));
        assert!(dff_traffic_light_is_north_south(90.0));
        assert!(!dff_traffic_light_is_north_south(180.0));
        assert!(dff_traffic_light_is_north_south(270.0));
        assert!(!dff_traffic_light_is_north_south(360.0));
    }

    #[test]
    fn sa_pointlight_falloff_is_full_then_linear() {
        assert_eq!(sa_2dfx_pointlight_attenuation(0.0), 1.0);
        assert_eq!(sa_2dfx_pointlight_attenuation(0.5), 1.0);
        assert!((sa_2dfx_pointlight_attenuation(0.75) - 0.5).abs() < 0.0001);
        assert_eq!(sa_2dfx_pointlight_attenuation(1.0), 0.0);
        assert_eq!(sa_2dfx_pointlight_attenuation(2.0), 0.0);
    }

    #[test]
    fn pointlight_reads_dff_color_position_and_radius() {
        let effect = point_light_effect(
            V3 {
                x: 4.0,
                y: -2.0,
                z: 10.0,
            },
            16.0,
            [255, 128, 0],
        );
        let light = dff_2dfx_point_light(&effect).expect("valid point light");
        assert_eq!(light.position, vec3(4.0, -2.0, 10.0));
        assert_eq!(light.radius, 16.0);
        assert!((light.color.x - 255.0 / 256.0).abs() < 0.0001);
        assert!((light.color.y - 128.0 / 256.0).abs() < 0.0001);
        assert_eq!(light.color.z, 0.0);
    }

    #[test]
    fn pointlight_selection_deduplicates_lod_and_matches_sa_pool_limit() {
        let mut effects = Vec::new();
        for index in 0..40 {
            effects.push(point_light_effect(
                V3 {
                    x: index as f32 * 2.0,
                    y: 0.0,
                    z: 4.0,
                },
                16.0,
                [255, 220, 180],
            ));
        }
        effects.push(effects[0].clone());
        let selected = select_dff_point_lights(&effects, Vec3::ZERO, 0.3);
        assert_eq!(selected.len(), MAX_DFF_POINT_LIGHTS);
        assert_eq!(selected[0].position, vec3(0.0, 0.0, 4.0));
        assert!((selected[0].color.x - (255.0 / 256.0 * 0.3)).abs() < 0.0001);
    }

    #[test]
    fn pointlight_sphere_aabb_intersection_handles_edges() {
        let min = vec3(-1.0, -1.0, -1.0);
        let max = vec3(1.0, 1.0, 1.0);
        assert!(sphere_intersects_aabb(Vec3::ZERO, 0.1, min, max));
        assert!(sphere_intersects_aabb(vec3(2.0, 0.0, 0.0), 1.0, min, max));
        assert!(!sphere_intersects_aabb(vec3(2.1, 0.0, 0.0), 1.0, min, max));
    }

    #[test]
    fn collision_classification_color_uses_exact_txd_override() {
        let mut classes = TextureMaterialClasses::default();
        classes.set_global("ground", Some(7));
        classes.set_txd("city", "ground", Some(9));

        assert_eq!(
            collision_classification_color(&classes, Some("city"), "ground", None, 0),
            collision_material_color(9, 1.0)
        );
        assert_eq!(
            collision_classification_color(&classes, Some("country"), "ground", None, 0),
            collision_material_color(7, 1.0)
        );
    }

    #[test]
    fn no_collision_has_a_distinct_diagnostic_color() {
        let mut classes = TextureMaterialClasses::default();
        classes.set_global_no_collision("foliage", true);

        let excluded = collision_classification_color(&classes, None, "foliage", None, 0);
        assert_eq!(excluded, [1.0, 0.08, 0.64, 1.0]);
        assert_ne!(excluded, collision_material_color(0, 1.0));
    }

    #[test]
    fn fracture_preview_uses_sa_velocity_gravity_and_clamps_time() {
        let vertex = |x, y, z| BreakableVertex {
            position: V3 { x, y, z },
            uv: V2::default(),
            color: [255; 4],
        };
        let breakable = BreakableGeometry {
            origin: BreakableOrigin::Collision,
            vertices: vec![
                // An unreferenced low vertex establishes the intact object's
                // ground plane while both authored groups begin above it.
                vertex(0.0, 0.0, 0.0),
                vertex(-1.5, -1.0, 2.0),
                vertex(-1.5, 1.0, 2.0),
                vertex(-1.5, 0.0, 4.0),
                vertex(1.5, -1.0, 2.0),
                vertex(1.5, 1.0, 2.0),
                vertex(1.5, 0.0, 4.0),
            ],
            triangles: vec![
                BreakableTriangle {
                    vertices: [1, 2, 3],
                    group: 0,
                    source_face: None,
                },
                BreakableTriangle {
                    vertices: [4, 5, 6],
                    group: 1,
                    source_face: None,
                },
            ],
            groups: vec![BreakableGroup::default(), BreakableGroup::default()],
            stale: false,
        };

        let base_velocity = vec3(0.0, 0.0, 0.1);
        let initial = fracture_preview_motion(&breakable, 0, 0.0, base_velocity, 0.0).unwrap();
        assert!(initial.translation.length() < 0.0001);
        assert!(initial.spin_degrees.abs() < 0.0001);
        assert!(initial.settle_rotation_degrees.length() < 0.0001);

        let rising = fracture_preview_motion(&breakable, 0, 0.05, base_velocity, 0.0).unwrap();
        assert!(rising.translation.z > 0.0);

        let left = fracture_preview_motion(&breakable, 0, 1.0, base_velocity, 0.0).unwrap();
        let right = fracture_preview_motion(&breakable, 1, 1.0, base_velocity, 0.0).unwrap();
        assert!(left.translation.x.abs() < 0.0001);
        assert!(right.translation.x.abs() < 0.0001);
        assert!(left.translation.y.abs() < 0.0001);
        assert!(right.translation.y.abs() < 0.0001);
        assert!(left.translation.z < 0.0);
        assert!(left.spin_degrees > 0.0);
        assert!(left.settle_rotation_degrees.length() > 0.0);

        let clamped = fracture_preview_motion(&breakable, 0, 100.0, base_velocity, 0.0).unwrap();
        let duration = fracture_preview_motion(
            &breakable,
            0,
            FRACTURE_PREVIEW_DURATION_SECONDS as f32,
            base_velocity,
            0.0,
        )
        .unwrap();
        assert!((clamped.translation - duration.translation).length() < 0.0001);
        assert!((clamped.spin_degrees - duration.spin_degrees).abs() < 0.0001);
        assert!(
            (clamped.settle_rotation_degrees - duration.settle_rotation_degrees).length() < 0.0001
        );
    }
}
