use super::super::*;

pub(crate) fn shadow_sample_offset(sample: usize, samples: usize, softness: f32) -> Vec2 {
    if samples <= 1 || softness <= 0.0 {
        return Vec2::ZERO;
    }
    let angle = sample as f32 * 2.3999631;
    let radius = ((sample as f32 + 0.5) / samples as f32).sqrt() * softness;
    vec2(angle.cos() * radius, angle.sin() * radius)
}

pub(crate) fn light_ray_for_sample(
    light: &EditorLight,
    world_pos: Vec3,
    sample: usize,
    settings: BakeSettings,
) -> Option<(Vec3, f32)> {
    match light.kind {
        LightKind::Ambient => None,
        LightKind::Directional => {
            // Direction is the light's forward/travel vector (matching the
            // XML/Blender contract). A visibility ray leaves the receiver in
            // the opposite direction, back toward the source.
            let base = -vec3(light.direction.x, light.direction.y, light.direction.z);
            if base.length_squared() < 0.0001 {
                return None;
            }
            let dir = base.normalize();
            let tangent = if dir.z.abs() < 0.9 {
                dir.cross(Vec3::Z).normalize()
            } else {
                dir.cross(Vec3::X).normalize()
            };
            let bitangent = dir.cross(tangent).normalize();
            let offset = shadow_sample_offset(
                sample,
                settings.shadow_samples,
                settings.shadow_softness / 16000.0,
            );
            Some((
                (dir + tangent * offset.x + bitangent * offset.y).normalize(),
                16000.0,
            ))
        }
        LightKind::Point | LightKind::Spot => {
            let light_pos = vec3(light.position.x, light.position.y, light.position.z);
            let to_light = light_pos - world_pos;
            let dist = to_light.length();
            if dist <= 0.001 {
                return None;
            }
            let dir = to_light / dist;
            let tangent = if dir.z.abs() < 0.9 {
                dir.cross(Vec3::Z).normalize()
            } else {
                dir.cross(Vec3::X).normalize()
            };
            let bitangent = dir.cross(tangent).normalize();
            let offset =
                shadow_sample_offset(sample, settings.shadow_samples, settings.shadow_softness);
            let sample_pos = light_pos + tangent * offset.x + bitangent * offset.y;
            let sample_vec = sample_pos - world_pos;
            let sample_dist = sample_vec.length();
            if sample_dist <= 0.001 {
                None
            } else {
                Some((sample_vec / sample_dist, sample_dist))
            }
        }
        LightKind::Area => {
            // Surface quadrature already distributes samples across the real
            // emitter. Applying point-light softness again would move the
            // visibility ray far outside the face and erase its shadows.
            let light_pos = vec3(light.position.x, light.position.y, light.position.z);
            let to_light = light_pos - world_pos;
            let distance = to_light.length();
            (distance > 0.001).then_some((to_light / distance, distance))
        }
    }
}

pub(crate) fn shadow_grid_key(point: Vec3, cell_size: f32) -> (i32, i32) {
    (
        (point.x / cell_size).floor() as i32,
        (point.y / cell_size).floor() as i32,
    )
}

pub(crate) fn build_shadow_grid(app: &AppState) -> ShadowGrid {
    build_shadow_grid_in_region(app, None)
}

pub(crate) fn bounds_intersect(a: Bounds, b: Bounds) -> bool {
    a.min.x <= b.max.x
        && a.max.x >= b.min.x
        && a.min.y <= b.max.y
        && a.max.y >= b.min.y
        && a.min.z <= b.max.z
        && a.max.z >= b.min.z
}

fn dff_face_casts_shadow(
    app: &AppState,
    dff_name: &str,
    part: &RenderPart,
    part_face: usize,
) -> bool {
    resolve_shadow_casting(
        &app.shadow_casting,
        dff_name,
        part.material_index,
        &part.texture_name,
        part.face_indices.get(part_face).copied(),
    )
}

fn resolve_shadow_casting(
    overrides: &HashMap<String, bool>,
    dff_name: &str,
    material: usize,
    texture: &str,
    face: Option<usize>,
) -> bool {
    if let Some(casts_shadow) =
        face.and_then(|face| overrides.get(&material_emitter_face_key(dff_name, face)))
    {
        return *casts_shadow;
    }
    if let Some(casts_shadow) = overrides.get(&material_emitter_key(dff_name, material)) {
        return *casts_shadow;
    }
    let texture = texture.trim();
    texture.is_empty()
        || overrides
            .get(&material_emitter_texture_key(texture))
            .copied()
            .unwrap_or(true)
}

/// Builds the occluder grid. When `region` is set, only placements whose world
/// bounds intersect it contribute triangles (Selected-scope AO uses this so a
/// small selection doesn't pay for whole-scene occluder gathering).
pub(crate) fn build_shadow_grid_in_region(app: &AppState, region: Option<Bounds>) -> ShadowGrid {
    let cell_size = 768.0;
    let mut grid = ShadowGrid {
        cell_size,
        cells: HashMap::new(),
        tris: Vec::new(),
    };
    for (idx, placement) in app.placements.iter().enumerate() {
        if app
            .element_states
            .get(idx)
            .is_some_and(|state| state.deleted)
        {
            continue;
        }
        let Some(mesh) = element_mesh(app, placement) else {
            continue;
        };
        let mesh_key = placement_mesh_key(placement, &app.definitions);
        let dff_name = mesh_key.split('|').next().unwrap_or(&mesh_key);
        let model = placement_matrix(placement).to_cols_array();
        if let Some(region) = region {
            if !bounds_intersect(transformed_bounds(mesh.bounds, &model), region) {
                continue;
            }
        }
        for part in &mesh.parts {
            for (part_face, tri) in part.cpu_vertices.chunks_exact(3).enumerate() {
                if !dff_face_casts_shadow(app, dff_name, part, part_face) {
                    continue;
                }
                let a = transform_point_gl(&model, tri[0].pos);
                let b = transform_point_gl(&model, tri[1].pos);
                let c = transform_point_gl(&model, tri[2].pos);
                let min = a.min(b).min(c);
                let max = a.max(b).max(c);
                let tri_idx = grid.tris.len();
                grid.tris.push(ShadowTri { a, b, c, min, max });
                let (min_x, min_y) = shadow_grid_key(min, cell_size);
                let (max_x, max_y) = shadow_grid_key(max, cell_size);
                for cx in min_x..=max_x {
                    for cy in min_y..=max_y {
                        grid.cells.entry((cx, cy)).or_default().push(tri_idx);
                    }
                }
            }
        }
    }
    grid
}

pub(crate) fn ray_overlaps_aabb(
    origin: Vec3,
    dir: Vec3,
    max_dist: f32,
    min: Vec3,
    max: Vec3,
) -> bool {
    ray_aabb(origin, dir, min, max).is_some_and(|t| t < max_dist)
}

pub(crate) fn scene_shadow_occluded(
    grid: &ShadowGrid,
    origin: Vec3,
    dir: Vec3,
    max_dist: f32,
) -> bool {
    let ray_end = origin + dir * max_dist;
    let query_min = origin.min(ray_end);
    let query_max = origin.max(ray_end);
    let (min_x, min_y) = shadow_grid_key(query_min, grid.cell_size);
    let (max_x, max_y) = shadow_grid_key(query_max, grid.cell_size);
    for cx in min_x..=max_x {
        for cy in min_y..=max_y {
            let Some(tris) = grid.cells.get(&(cx, cy)) else {
                continue;
            };
            for tri_idx in tris {
                let Some(tri) = grid.tris.get(*tri_idx) else {
                    continue;
                };
                if !ray_overlaps_aabb(origin, dir, max_dist, tri.min, tri.max) {
                    continue;
                }
                if ray_triangle(origin, dir, tri.a, tri.b, tri.c).is_some_and(|t| t < max_dist) {
                    return true;
                }
            }
        }
    }
    false
}

/// Distance to the nearest occluder along the ray, if any within `max_dist`.
pub(crate) fn scene_shadow_nearest_hit(
    grid: &ShadowGrid,
    origin: Vec3,
    dir: Vec3,
    max_dist: f32,
) -> Option<f32> {
    let ray_end = origin + dir * max_dist;
    let query_min = origin.min(ray_end);
    let query_max = origin.max(ray_end);
    let (min_x, min_y) = shadow_grid_key(query_min, grid.cell_size);
    let (max_x, max_y) = shadow_grid_key(query_max, grid.cell_size);
    let mut nearest: Option<f32> = None;
    for cx in min_x..=max_x {
        for cy in min_y..=max_y {
            let Some(tris) = grid.cells.get(&(cx, cy)) else {
                continue;
            };
            for tri_idx in tris {
                let Some(tri) = grid.tris.get(*tri_idx) else {
                    continue;
                };
                let limit = nearest.unwrap_or(max_dist);
                if !ray_overlaps_aabb(origin, dir, limit, tri.min, tri.max) {
                    continue;
                }
                if let Some(t) = ray_triangle(origin, dir, tri.a, tri.b, tri.c) {
                    if t < limit {
                        nearest = Some(t);
                    }
                }
            }
        }
    }
    nearest
}

/// Cosine-weighted hemisphere direction (golden spiral) around `normal`.
pub(crate) fn ao_sample_direction(normal: Vec3, sample: usize, samples: usize) -> Vec3 {
    let t = (sample as f32 + 0.5) / samples.max(1) as f32;
    let cos_theta = (1.0 - t).sqrt();
    let sin_theta = t.sqrt();
    let phi = sample as f32 * 2.3999631;
    let tangent = if normal.z.abs() < 0.9 {
        normal.cross(Vec3::Z).normalize_or_zero()
    } else {
        normal.cross(Vec3::X).normalize_or_zero()
    };
    let bitangent = normal.cross(tangent);
    (tangent * (phi.cos() * sin_theta) + bitangent * (phi.sin() * sin_theta) + normal * cos_theta)
        .normalize_or_zero()
}

/// Hemisphere occlusion with linear distance falloff (0 = open, 1 = fully
/// enclosed). Rays start just off the surface so tight creases and edges
/// register; a hit at distance d contributes 1 - d/radius, which is what
/// gives the classic dark-in-the-corner, bright-on-the-flat edge AO look.
pub(crate) fn ao_occlusion_value(
    grid: &ShadowGrid,
    settings: BakeSettings,
    world_pos: Vec3,
    normal: Vec3,
) -> f32 {
    if grid.tris.is_empty() || settings.ao_samples == 0 {
        return 0.0;
    }
    let normal = normal.normalize_or_zero();
    if normal.length_squared() < 0.0001 {
        return 0.0;
    }
    let max_dist = settings.ao_radius.max(0.5);
    // Small lift off the surface: enough to dodge coplanar self-hits, small
    // enough to keep crevice detail. Scale with radius for large-radius AO.
    let offset = (max_dist * 0.02).clamp(0.05, 0.5);
    let origin = world_pos + normal * offset;
    let samples = settings.ao_samples;
    let mut occlusion = 0.0f32;
    for sample in 0..samples {
        let dir = ao_sample_direction(normal, sample, samples);
        if dir.length_squared() < 0.0001 {
            continue;
        }
        if let Some(t) = scene_shadow_nearest_hit(grid, origin, dir, max_dist) {
            occlusion += (1.0 - t / max_dist).clamp(0.0, 1.0);
        }
    }
    (occlusion / samples as f32).clamp(0.0, 1.0)
}

pub(crate) fn light_visibility(
    grid: &ShadowGrid,
    settings: BakeSettings,
    light: &EditorLight,
    world_pos: Vec3,
    normal: Vec3,
) -> f32 {
    if !light.casts_shadow || matches!(light.kind, LightKind::Ambient) || grid.tris.is_empty() {
        return 1.0;
    }
    if settings.shadow_samples == 0 {
        return 1.0;
    }
    // One exact ray per area quadrature point. Soft area-light shadows emerge
    // from the collection of surface points, not repeated jitter around each
    // individual point.
    let samples = if matches!(light.kind, LightKind::Area) {
        1
    } else {
        settings.shadow_samples
    };
    let origin = world_pos + normal.normalize_or_zero() * 2.0;
    let mut visible = 0usize;
    for sample in 0..samples {
        let Some((dir, max_dist)) = light_ray_for_sample(light, world_pos, sample, settings) else {
            visible += 1;
            continue;
        };
        if !scene_shadow_occluded(grid, origin, dir, max_dist) {
            visible += 1;
        }
    }
    visible as f32 / samples as f32
}

pub(crate) fn bake_light_value(
    grid: &ShadowGrid,
    settings: BakeSettings,
    light: &EditorLight,
    world_pos: Vec3,
    normal: Vec3,
) -> Vec3 {
    let light_color = light_effective_color(light);
    let color = vec3(light_color.x, light_color.y, light_color.z) * light.intensity.max(0.0);
    match light.kind {
        LightKind::Ambient => color,
        LightKind::Directional => {
            let dir = vec3(light.direction.x, light.direction.y, light.direction.z);
            if dir.length_squared() < 0.0001 {
                return Vec3::ZERO;
            }
            let amount = normal.normalize_or_zero().dot(-dir.normalize()).max(0.0);
            color * amount * light_visibility(grid, settings, light, world_pos, normal)
        }
        LightKind::Point | LightKind::Spot => {
            let light_pos = vec3(light.position.x, light.position.y, light.position.z);
            let to_light = light_pos - world_pos;
            let dist = to_light.length();
            if dist <= 0.001 || light.radius <= 0.001 || dist > light.radius {
                return Vec3::ZERO;
            }
            let mut attenuation = smooth_distance_attenuation(dist, light.radius);
            if matches!(light.kind, LightKind::Spot) {
                attenuation *= spot_cone_attenuation(light, world_pos);
            } else {
                attenuation *= point_lobe_attenuation(light, world_pos);
            }
            let amount = normal
                .normalize_or_zero()
                .dot(to_light.normalize())
                .max(0.0);
            color
                * amount
                * attenuation
                * light_visibility(grid, settings, light, world_pos, normal)
        }
        LightKind::Area => {
            let light_pos = vec3(light.position.x, light.position.y, light.position.z);
            let to_light = light_pos - world_pos;
            let distance_squared = to_light.length_squared();
            let distance = distance_squared.sqrt();
            if distance <= 0.001 || light.radius <= 0.001 || distance > light.radius {
                return Vec3::ZERO;
            }
            let receiver_to_light = to_light / distance;
            let emitter_normal =
                vec3(light.direction.x, light.direction.y, light.direction.z).normalize_or_zero();
            if emitter_normal.length_squared() < 0.0001 {
                return Vec3::ZERO;
            }
            // Differential area-light geometry term. `intensity` already
            // contains the surface area represented by this quadrature sample.
            let receiver_cosine = normal.normalize_or_zero().dot(receiver_to_light).max(0.0);
            let emitter_cosine = emitter_normal.dot(-receiver_to_light).max(0.0);
            if receiver_cosine <= 0.0 || emitter_cosine <= 0.0 {
                return Vec3::ZERO;
            }
            let geometry = receiver_cosine * emitter_cosine / distance_squared.max(1.0);
            color
                * geometry
                * smooth_distance_attenuation(distance, light.radius)
                * light_visibility(grid, settings, light, world_pos, normal)
        }
    }
}

/// Energy-bounded indirect fill used by the bounce approximation. It keeps
/// distance/cone falloff for local lights, but deliberately ignores the
/// receiver normal and direct shadow visibility: indirect light arrives from
/// the surrounding scene rather than along the original direct ray.
pub(crate) fn bake_indirect_light_value(light: &EditorLight, world_pos: Vec3) -> Vec3 {
    let light_color = light_effective_color(light);
    let color = vec3(light_color.x, light_color.y, light_color.z) * light.intensity.max(0.0);
    match light.kind {
        LightKind::Ambient => Vec3::ZERO,
        LightKind::Directional => color,
        LightKind::Point | LightKind::Spot => {
            let light_pos = vec3(light.position.x, light.position.y, light.position.z);
            let distance = (light_pos - world_pos).length();
            if light.radius <= 0.001 || distance > light.radius {
                return Vec3::ZERO;
            }
            let mut attenuation = smooth_distance_attenuation(distance, light.radius);
            if matches!(light.kind, LightKind::Spot) {
                attenuation *= spot_cone_attenuation(light, world_pos);
            } else {
                attenuation *= point_lobe_attenuation(light, world_pos);
            }
            color * attenuation
        }
        LightKind::Area => {
            let light_pos = vec3(light.position.x, light.position.y, light.position.z);
            let from_light = world_pos - light_pos;
            let distance_squared = from_light.length_squared();
            let distance = distance_squared.sqrt();
            if distance <= 0.001 || light.radius <= 0.001 || distance > light.radius {
                return Vec3::ZERO;
            }
            let emitter_normal =
                vec3(light.direction.x, light.direction.y, light.direction.z).normalize_or_zero();
            let emitter_cosine = emitter_normal.dot(from_light / distance).max(0.0);
            color * emitter_cosine * smooth_distance_attenuation(distance, light.radius)
                / distance_squared.max(1.0)
        }
    }
}

/// Directional response for internally generated Point emitter lobes. Authored
/// point lights use `Omni` and retain their existing isotropic behavior.
pub(crate) fn point_lobe_attenuation(light: &EditorLight, world_pos: Vec3) -> f32 {
    if light.point_lobe == PointLightLobe::Omni {
        return 1.0;
    }
    let light_pos = vec3(light.position.x, light.position.y, light.position.z);
    let from_light = (world_pos - light_pos).normalize_or_zero();
    if from_light.length_squared() < 0.0001 {
        return 0.0;
    }
    // Normalize the vertical/horizontal weights so Up + Down + Sides is
    // exactly one. Consequently the legacy 1/1/1 defaults reproduce an
    // ordinary omnidirectional point light while still blending smoothly.
    let horizontal = from_light.xy().length();
    let weight_sum = (from_light.z.abs() + horizontal).max(0.0001);
    match light.point_lobe {
        PointLightLobe::Omni => 1.0,
        PointLightLobe::Up => from_light.z.max(0.0) / weight_sum,
        PointLightLobe::Down => (-from_light.z).max(0.0) / weight_sum,
        PointLightLobe::Sides => horizontal / weight_sum,
    }
}

/// Spotlight cone matching the fixed 72-degree GPU shadow projection. The
/// outer 20 percent is feathered to avoid a hard ring on sparse vertex meshes.
pub(crate) fn spot_cone_attenuation(light: &EditorLight, world_pos: Vec3) -> f32 {
    const OUTER_HALF_ANGLE_DEGREES: f32 = 36.0;
    const INNER_HALF_ANGLE_DEGREES: f32 = OUTER_HALF_ANGLE_DEGREES * 0.8;
    let forward = vec3(light.direction.x, light.direction.y, light.direction.z).normalize_or_zero();
    let from_light = (world_pos - vec3(light.position.x, light.position.y, light.position.z))
        .normalize_or_zero();
    if forward.length_squared() < 0.0001 || from_light.length_squared() < 0.0001 {
        return 0.0;
    }
    let cone_cos = forward.dot(from_light);
    let outer_cos = OUTER_HALF_ANGLE_DEGREES.to_radians().cos();
    let inner_cos = INNER_HALF_ANGLE_DEGREES.to_radians().cos();
    let t = ((cone_cos - outer_cos) / (inner_cos - outer_cos)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub(crate) fn smooth_distance_attenuation(distance: f32, falloff_distance: f32) -> f32 {
    if !distance.is_finite()
        || !falloff_distance.is_finite()
        || falloff_distance <= 0.0
        || distance >= falloff_distance
    {
        return 0.0;
    }
    let t = (distance / falloff_distance).clamp(0.0, 1.0);
    1.0 - t * t * (3.0 - 2.0 * t)
}

pub(crate) fn uniform_location(program: u32, name: &str) -> i32 {
    let Ok(name) = CString::new(name) else {
        return -1;
    };
    unsafe { gl::GetUniformLocation(program, name.as_ptr()) }
}

pub(crate) fn upload_gpu_bake_lights(program: u32, settings: BakeSettings, lights: &[EditorLight]) {
    const MAX_GPU_LIGHTS: usize = 32;
    let count = lights.len().min(MAX_GPU_LIGHTS);
    let mut kinds = [0i32; MAX_GPU_LIGHTS];
    let mut point_lobes = [0i32; MAX_GPU_LIGHTS];
    let mut pos_radius = [0.0f32; MAX_GPU_LIGHTS * 4];
    let mut dir_intensity = [0.0f32; MAX_GPU_LIGHTS * 4];
    let mut color = [0.0f32; MAX_GPU_LIGHTS * 4];
    for (idx, light) in lights.iter().take(MAX_GPU_LIGHTS).enumerate() {
        kinds[idx] = match light.kind {
            LightKind::Ambient => 0,
            LightKind::Directional => 1,
            LightKind::Point => 2,
            LightKind::Spot => 3,
            LightKind::Area => 4,
        };
        point_lobes[idx] = match light.point_lobe {
            PointLightLobe::Omni => 0,
            PointLightLobe::Up => 1,
            PointLightLobe::Down => 2,
            PointLightLobe::Sides => 3,
        };
        let base = idx * 4;
        pos_radius[base] = light.position.x;
        pos_radius[base + 1] = light.position.y;
        pos_radius[base + 2] = light.position.z;
        pos_radius[base + 3] = light.radius;
        dir_intensity[base] = light.direction.x;
        dir_intensity[base + 1] = light.direction.y;
        dir_intensity[base + 2] = light.direction.z;
        dir_intensity[base + 3] = light.intensity.max(0.0);
        let light_color = light_effective_color(light);
        color[base] = light_color.x;
        color[base + 1] = light_color.y;
        color[base + 2] = light_color.z;
        color[base + 3] = 1.0;
    }
    let bounce_scale = effective_bounce_gain(
        settings.bounces,
        settings.bounce_strength,
        settings.bounce_maximum,
    );
    unsafe {
        gl::Uniform1i(uniform_location(program, "light_count"), count as i32);
        gl::Uniform1iv(
            uniform_location(program, "light_kind"),
            MAX_GPU_LIGHTS as i32,
            kinds.as_ptr(),
        );
        gl::Uniform1iv(
            uniform_location(program, "light_point_lobe"),
            MAX_GPU_LIGHTS as i32,
            point_lobes.as_ptr(),
        );
        gl::Uniform4fv(
            uniform_location(program, "light_pos_radius"),
            MAX_GPU_LIGHTS as i32,
            pos_radius.as_ptr(),
        );
        gl::Uniform4fv(
            uniform_location(program, "light_dir_intensity"),
            MAX_GPU_LIGHTS as i32,
            dir_intensity.as_ptr(),
        );
        gl::Uniform4fv(
            uniform_location(program, "light_color"),
            MAX_GPU_LIGHTS as i32,
            color.as_ptr(),
        );
        gl::Uniform1f(uniform_location(program, "bounce_scale"), bounce_scale);
        gl::Uniform1f(
            uniform_location(program, "shadow_filter_radius"),
            gpu_shadow_filter_radius(settings.shadow_softness),
        );
        gl::Uniform1i(
            uniform_location(program, "shadow_sample_count"),
            gpu_shadow_sample_count(settings.shadow_samples) as i32,
        );
    }
}

pub(crate) fn gpu_shadow_sample_count(samples: usize) -> usize {
    samples.clamp(1, 128)
}

pub(crate) fn gpu_shadow_filter_radius(softness: f32) -> f32 {
    if !softness.is_finite() {
        return 0.0;
    }
    // The former /32 mapping spread a sparse 3x3 kernel over a 20-texel
    // diameter at the Final preset. A denser, tighter disk preserves a soft
    // penumbra without destroying the underlying shadow-map detail.
    (softness / 64.0).clamp(0.0, 8.0)
}

/// The baker does not trace indirect light paths, so bounces are an energy-
/// bounded fill approximation. Multiplying direct light by `count * strength`
/// made Final-quality surfaces up to 6.2x brighter and caused isolated meshes
/// to appear emissive. Successive bounces now decay and the total fill is
/// capped by the user-configurable Bounce Maximum setting.
pub(crate) fn effective_bounce_gain(bounces: usize, strength: f32, maximum: f32) -> f32 {
    if bounces == 0
        || strength <= 0.0
        || maximum <= 0.0
        || !strength.is_finite()
        || !maximum.is_finite()
    {
        return 0.0;
    }
    let retained = 1.0 - strength.clamp(0.0, 0.95);
    (1.0 - retained.powi(bounces.min(64) as i32)) * maximum.clamp(0.0, 1.0)
}

pub(crate) fn bake_has_shadowed_lights(lights: &[EditorLight]) -> bool {
    lights
        .iter()
        .any(|light| light.casts_shadow && !matches!(light.kind, LightKind::Ambient))
}

pub(crate) fn light_active_for_bake(light: &EditorLight, mode: BakeLightMode) -> bool {
    matches!(
        (light.profile, mode),
        (LightProfile::Both, _)
            | (LightProfile::Day, BakeLightMode::Day)
            | (LightProfile::Night, BakeLightMode::Night)
            | (LightProfile::Day, BakeLightMode::Both)
            | (LightProfile::Night, BakeLightMode::Both)
    )
}

pub(crate) fn bake_output_modes(mode: BakeLightMode) -> (BakeLightMode, Option<BakeLightMode>) {
    match mode {
        BakeLightMode::Both => (BakeLightMode::Day, Some(BakeLightMode::Night)),
        mode => (mode, None),
    }
}

pub(crate) fn bake_lights_for_mode(
    lights: &[EditorLight],
    mode: BakeLightMode,
) -> Vec<EditorLight> {
    lights
        .iter()
        .filter(|light| light_active_for_bake(light, mode))
        .cloned()
        .collect()
}

fn material_emitter_active(emitter: MaterialEmitter, mode: BakeLightMode) -> bool {
    emitter.enabled
        && match mode {
            BakeLightMode::Day => emitter.day,
            BakeLightMode::Night => emitter.night,
            BakeLightMode::Both => emitter.day || emitter.night,
        }
}

fn material_emitter_for_part(
    app: &AppState,
    dff_name: &str,
    part: &RenderPart,
) -> Option<MaterialEmitter> {
    let local = material_emitter_key(dff_name, part.material_index);
    app.material_emitters.get(&local).copied().or_else(|| {
        let texture = part.texture_name.trim();
        (!texture.is_empty())
            .then(|| material_emitter_texture_key(texture))
            .and_then(|key| app.material_emitters.get(&key).copied())
    })
}

fn material_emitter_for_face(
    app: &AppState,
    face_emitters: &HashMap<(String, usize), MaterialEmitter>,
    dff_name: &str,
    part: &RenderPart,
    face_index: usize,
) -> Option<MaterialEmitter> {
    let dff_key = asset_key(dff_name, ".dff");
    app.material_emitters
        .get(&material_emitter_face_key(dff_name, face_index))
        .copied()
        .or_else(|| face_emitters.get(&(dff_key, face_index)).copied())
        .or_else(|| material_emitter_for_part(app, dff_name, part))
}

fn material_face_emitter_index(
    emitters: &HashMap<String, MaterialEmitter>,
) -> HashMap<(String, usize), MaterialEmitter> {
    let mut face_emitters = HashMap::new();
    for (key, emitter) in emitters {
        if let Some((dff, face)) = material_emitter_face_from_key(key) {
            face_emitters.insert((dff.to_string(), face), *emitter);
            continue;
        }
        if let Some((dff, faces)) = material_emitter_face_group_from_key(key) {
            for face in faces {
                face_emitters.insert((dff.to_string(), face), *emitter);
            }
        }
    }
    face_emitters
}

fn build_material_face_emitter_index(app: &AppState) -> HashMap<(String, usize), MaterialEmitter> {
    material_face_emitter_index(&app.material_emitters)
}

fn emitter_sample_limits(preset: BakeQualityPreset) -> (usize, f32) {
    match preset {
        BakeQualityPreset::Preview => (4, 128.0),
        BakeQualityPreset::Balanced => (16, 64.0),
        BakeQualityPreset::Final => (64, 32.0),
    }
}

// Material-emitter strength predates the physical area-light evaluator and is
// authored in legacy vertex-light units, not radiance per square world unit.
// This conversion keeps small practical emitters (lamp lenses, neon tubes)
// visible while area, angle, and distance still determine their distribution.
const MATERIAL_EMITTER_RADIANCE_SCALE: f32 = 256.0;

type MaterialEmitterTriangle = (Vec3, Vec3, Vec3, f32, Vec3);

#[derive(Clone, Copy, Debug)]
struct PointEmitterGroup {
    min: Vec3,
    max: Vec3,
    weighted_center: Vec3,
    total_area: f32,
    faces: usize,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct PointEmitterDebugGroup {
    pub(crate) center: Vec3,
    pub(crate) min: Vec3,
    pub(crate) max: Vec3,
    pub(crate) faces: usize,
    pub(crate) up_strength: f32,
    pub(crate) down_strength: f32,
    pub(crate) sides_strength: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PointEmitterCoronaSource {
    pub(crate) position: V3,
    pub(crate) color: V3,
    pub(crate) day: bool,
    pub(crate) night: bool,
    pub(crate) range: f32,
    pub(crate) height_above_model_base: f32,
}

fn point_emitter_groups(
    triangles: &[MaterialEmitterTriangle],
    max_grouping_size: f32,
) -> Vec<PointEmitterGroup> {
    let max_size = max_grouping_size.clamp(0.01, 100_000.0);
    let mut groups = Vec::<PointEmitterGroup>::new();
    for (a, b, c, area, _) in triangles.iter().copied() {
        let tri_min = a.min(b).min(c);
        let tri_max = a.max(b).max(c);
        let centroid = (a + b + c) / 3.0;
        let mut best: Option<(usize, f32)> = None;
        for (idx, group) in groups.iter().enumerate() {
            let combined_min = group.min.min(tri_min);
            let combined_max = group.max.max(tri_max);
            if (combined_max - combined_min).max_element() > max_size + 0.0001 {
                continue;
            }
            let center = group.weighted_center / group.total_area.max(0.0001);
            let distance_squared = center.distance_squared(centroid);
            if best.is_none_or(|(_, best_distance)| distance_squared < best_distance) {
                best = Some((idx, distance_squared));
            }
        }
        if let Some((idx, _)) = best {
            let group = &mut groups[idx];
            group.min = group.min.min(tri_min);
            group.max = group.max.max(tri_max);
            group.weighted_center += centroid * area;
            group.total_area += area;
            group.faces += 1;
        } else {
            groups.push(PointEmitterGroup {
                min: tri_min,
                max: tri_max,
                weighted_center: centroid * area,
                total_area: area,
                faces: 1,
            });
        }
    }
    groups
}

/// Recreates the selected Point emitter's grouping in DFF-local space for the
/// editing viewport. Face and material overrides use the same precedence as
/// the baker, so the markers reveal the faces that actually feed this emitter
/// rather than every face that merely shares its texture.
pub(crate) fn editing_dff_point_emitter_debug_groups(
    app: &AppState,
    dff: &EditingDffState,
) -> Vec<PointEmitterDebugGroup> {
    let Some(selected_key) = selected_material_emitter_key(app) else {
        return Vec::new();
    };
    let Some(emitter) = app.material_emitters.get(&selected_key).copied() else {
        return Vec::new();
    };
    if !emitter.enabled || emitter.cast_mode != MaterialEmitterCastMode::Point {
        return Vec::new();
    }

    let dff_key = asset_key(&dff.name, ".dff");
    let selected_face_set = material_emitter_face_group_from_key(&selected_key)
        .filter(|(name, _)| *name == dff_key)
        .map(|(_, faces)| faces.into_iter().collect::<HashSet<_>>())
        .or_else(|| {
            material_emitter_face_from_key(&selected_key)
                .filter(|(name, _)| *name == dff_key)
                .map(|(_, face)| HashSet::from([face]))
        });
    let selected_texture = material_emitter_texture_from_key(&selected_key);
    let selected_material = if selected_face_set.is_none() && selected_texture.is_none() {
        selected_key
            .rsplit_once('|')
            .filter(|(name, _)| *name == dff_key)
            .and_then(|(_, material)| material.parse::<usize>().ok())
    } else {
        None
    };

    let mut overridden_faces = HashSet::new();
    for key in app.material_emitters.keys() {
        if let Some((name, faces)) = material_emitter_face_group_from_key(key) {
            if name == dff_key {
                overridden_faces.extend(faces);
            }
        } else if let Some((name, face)) = material_emitter_face_from_key(key)
            && name == dff_key
        {
            overridden_faces.insert(face);
        }
    }

    let mut triangles_by_part = BTreeMap::<(usize, usize), Vec<MaterialEmitterTriangle>>::new();
    for (face_index, triangle) in dff.raw.triangles.iter().enumerate() {
        let material = triangle.material as usize;
        let included = if let Some(faces) = selected_face_set.as_ref() {
            faces.contains(&face_index)
        } else if overridden_faces.contains(&face_index) {
            false
        } else if let Some(selected_material) = selected_material {
            material == selected_material
        } else if let Some(selected_texture) = selected_texture {
            let local_key = material_emitter_key(&dff.name, material);
            !app.material_emitters.contains_key(&local_key)
                && dff
                    .raw
                    .material_textures
                    .get(material)
                    .is_some_and(|texture| {
                        material_emitter_texture_key(texture) == selected_key
                            && material_emitter_texture_from_key(&selected_key)
                                == Some(selected_texture)
                    })
        } else {
            false
        };
        if !included {
            continue;
        }
        let Some(a) = dff
            .raw
            .vertices
            .get(triangle.a as usize)
            .map(|value| to_mq(*value))
        else {
            continue;
        };
        let Some(b) = dff
            .raw
            .vertices
            .get(triangle.b as usize)
            .map(|value| to_mq(*value))
        else {
            continue;
        };
        let Some(c) = dff
            .raw
            .vertices
            .get(triangle.c as usize)
            .map(|value| to_mq(*value))
        else {
            continue;
        };
        let cross = (b - a).cross(c - a);
        let area = cross.length() * 0.5;
        if area <= 0.0001 {
            continue;
        }
        let component = dff
            .raw
            .components
            .iter()
            .position(|component| {
                face_index >= component.tri_start && face_index < component.tri_end
            })
            .unwrap_or(0);
        triangles_by_part
            .entry((component, material))
            .or_default()
            .push((a, b, c, area, cross.normalize_or_zero()));
    }

    triangles_by_part
        .into_values()
        .flat_map(|triangles| point_emitter_groups(&triangles, emitter.max_grouping_size))
        .map(|group| PointEmitterDebugGroup {
            center: group.weighted_center / group.total_area.max(0.0001),
            min: group.min,
            max: group.max,
            faces: group.faces,
            up_strength: emitter.point_up_strength.max(0.0),
            down_strength: emitter.point_down_strength.max(0.0),
            sides_strength: emitter.point_sides_strength.max(0.0),
        })
        .collect()
}

/// Builds one DFF-local corona source for every point-emitter group in the
/// open model. Face overrides take precedence over material and texture
/// emitters, matching the light baker.
pub(crate) fn dff_point_emitter_corona_sources(
    emitters: &HashMap<String, MaterialEmitter>,
    dff_name: &str,
    raw: &RawMesh,
) -> Vec<PointEmitterCoronaSource> {
    let dff_key = asset_key(dff_name, ".dff");
    let model_min_z = bounds_from_vertices(&raw.vertices).min.z;
    let face_emitters = material_face_emitter_index(emitters);
    let mut triangles_by_part =
        BTreeMap::<(usize, usize), Vec<(MaterialEmitter, Vec<MaterialEmitterTriangle>)>>::new();

    for (face_index, triangle) in raw.triangles.iter().enumerate() {
        let material = triangle.material as usize;
        let emitter = face_emitters
            .get(&(dff_key.clone(), face_index))
            .copied()
            .or_else(|| {
                emitters
                    .get(&material_emitter_key(dff_name, material))
                    .copied()
            })
            .or_else(|| {
                raw.material_textures
                    .get(material)
                    .map(|texture| texture.trim())
                    .filter(|texture| !texture.is_empty())
                    .map(material_emitter_texture_key)
                    .and_then(|key| emitters.get(&key).copied())
            });
        let Some(emitter) = emitter.filter(|emitter| {
            emitter.enabled
                && emitter.cast_mode == MaterialEmitterCastMode::Point
                && (emitter.day || emitter.night)
        }) else {
            continue;
        };
        let Some(a) = raw
            .vertices
            .get(triangle.a as usize)
            .map(|value| to_mq(*value))
        else {
            continue;
        };
        let Some(b) = raw
            .vertices
            .get(triangle.b as usize)
            .map(|value| to_mq(*value))
        else {
            continue;
        };
        let Some(c) = raw
            .vertices
            .get(triangle.c as usize)
            .map(|value| to_mq(*value))
        else {
            continue;
        };
        let cross = (b - a).cross(c - a);
        let area = cross.length() * 0.5;
        if area <= 0.0001 {
            continue;
        }
        let component = raw
            .components
            .iter()
            .position(|component| {
                face_index >= component.tri_start && face_index < component.tri_end
            })
            .unwrap_or(0);
        let groups = triangles_by_part.entry((component, material)).or_default();
        if let Some((_, triangles)) = groups
            .iter_mut()
            .find(|(candidate, _)| *candidate == emitter)
        {
            triangles.push((a, b, c, area, cross.normalize_or_zero()));
        } else {
            groups.push((emitter, vec![(a, b, c, area, cross.normalize_or_zero())]));
        }
    }

    let mut sources = Vec::new();
    for ((_, material), emitter_groups) in triangles_by_part {
        for (emitter, triangles) in emitter_groups {
            let color = if emitter.use_material_color {
                raw.materials
                    .get(material)
                    .map(|material| material.color)
                    .unwrap_or_else(neutral_vertex_color)
            } else if emitter.use_temperature {
                color_from_temperature(emitter.temperature)
            } else {
                emitter.color
            };
            for group in point_emitter_groups(&triangles, emitter.max_grouping_size) {
                sources.push(PointEmitterCoronaSource {
                    position: from_mq(group.weighted_center / group.total_area.max(0.0001)),
                    color: v3_clamp01(color),
                    day: emitter.day,
                    night: emitter.night,
                    range: emitter.falloff_distance.clamp(0.0, 100_000.0),
                    height_above_model_base: (group.weighted_center.z
                        / group.total_area.max(0.0001)
                        - model_min_z)
                        .max(0.0),
                });
            }
        }
    }
    sources
}

fn dff_emitter_direction(geometric_normal: Vec3, rendered_normal: Vec3) -> Vec3 {
    let mut surface_normal = geometric_normal.normalize_or_zero();
    let rendered_normal = rendered_normal.normalize_or_zero();
    // Emit along the rendered face normal. Geometric winding is used for the
    // face plane, then corrected to the normal side authored in the DFF.
    if rendered_normal.length_squared() > 0.0001 && surface_normal.dot(rendered_normal) < 0.0 {
        surface_normal = -surface_normal;
    }
    surface_normal
}

fn material_point_lobes(emitter: MaterialEmitter) -> Vec<(PointLightLobe, &'static str, f32)> {
    let base_strength = emitter.strength.max(0.0);
    [
        (PointLightLobe::Up, "up", emitter.point_up_strength),
        (PointLightLobe::Down, "down", emitter.point_down_strength),
        (PointLightLobe::Sides, "sides", emitter.point_sides_strength),
    ]
    .into_iter()
    .filter_map(|(lobe, name, multiplier)| {
        let intensity = base_strength * multiplier.max(0.0);
        (intensity > 0.0).then_some((lobe, name, intensity))
    })
    .collect()
}

fn material_emitter_lights(
    app: &AppState,
    face_emitters: &HashMap<(String, usize), MaterialEmitter>,
    mode: BakeLightMode,
) -> Vec<EditorLight> {
    if !app.bake_settings.face_emitters_enabled {
        return Vec::new();
    }
    let mut lights = Vec::new();
    for (placement_idx, placement) in app.placements.iter().enumerate() {
        if app
            .element_states
            .get(placement_idx)
            .is_some_and(|state| state.deleted)
        {
            continue;
        }
        let mesh_key = placement_mesh_key(placement, &app.definitions);
        let Some(mesh) = app.meshes.get(&mesh_key) else {
            continue;
        };
        let dff_name = mesh_key.split('|').next().unwrap_or(&mesh_key);
        let model = placement_matrix(placement).to_cols_array();
        for part in &mesh.parts {
            if part.cpu_vertices.is_empty() {
                continue;
            }
            let mut groups = Vec::<(MaterialEmitter, Vec<MaterialEmitterTriangle>)>::new();
            for (part_face, tri) in part.cpu_vertices.chunks_exact(3).enumerate() {
                let Some(face_index) = part.face_indices.get(part_face).copied() else {
                    continue;
                };
                let Some(emitter) =
                    material_emitter_for_face(app, face_emitters, dff_name, part, face_index)
                else {
                    continue;
                };
                if !material_emitter_active(emitter, mode) {
                    continue;
                }
                let a = transform_point_gl(&model, tri[0].pos);
                let b = transform_point_gl(&model, tri[1].pos);
                let c = transform_point_gl(&model, tri[2].pos);
                let cross = (b - a).cross(c - a);
                let area = cross.length() * 0.5;
                if area <= 0.0001 {
                    continue;
                }
                let geometric_normal = cross.normalize();
                // Imported DFF winding is not guaranteed to agree with its
                // rendered normal stream. Preserve the geometric face normal,
                // but orient it to the normals the user actually sees.
                let rendered_normal = (transform_normal_gl(&model, tri[0].normal)
                    + transform_normal_gl(&model, tri[1].normal)
                    + transform_normal_gl(&model, tri[2].normal))
                .normalize_or_zero();
                let mut normal = dff_emitter_direction(geometric_normal, rendered_normal);
                if emitter.emit_inversed {
                    normal = -normal;
                }
                let triangle = (a, b, c, area, normal);
                if let Some((_, triangles)) = groups
                    .iter_mut()
                    .find(|(candidate, _)| *candidate == emitter)
                {
                    triangles.push(triangle);
                } else {
                    groups.push((emitter, vec![triangle]));
                }
            }
            for (emitter, triangles) in groups {
                let color = if emitter.use_material_color {
                    part.material_color
                } else if emitter.use_temperature {
                    color_from_temperature(emitter.temperature)
                } else {
                    emitter.color
                };
                let profile = match (emitter.day, emitter.night) {
                    (true, true) => LightProfile::Both,
                    (true, false) => LightProfile::Day,
                    (false, true) => LightProfile::Night,
                    (false, false) => continue,
                };
                if emitter.cast_mode == MaterialEmitterCastMode::Point {
                    let lobes = material_point_lobes(emitter);
                    if lobes.is_empty() {
                        continue;
                    }
                    for (group_idx, group) in
                        point_emitter_groups(&triangles, emitter.max_grouping_size)
                            .into_iter()
                            .enumerate()
                    {
                        let point = group.weighted_center / group.total_area.max(0.0001);
                        for &(point_lobe, lobe_name, intensity) in &lobes {
                            lights.push(EditorLight {
                                name: format!(
                                    "Emitter {} material {} point {} {} ({} faces)",
                                    dff_name,
                                    part.material_index,
                                    group_idx + 1,
                                    lobe_name,
                                    group.faces,
                                ),
                                attached_to: None,
                                kind: LightKind::Point,
                                profile,
                                position: V3 {
                                    x: point.x,
                                    y: point.y,
                                    z: point.z,
                                },
                                direction: V3::default(),
                                color: v3_clamp01(color),
                                temperature: emitter.temperature,
                                use_temperature: emitter.use_temperature,
                                intensity,
                                radius: emitter.falloff_distance.clamp(1.0, 100_000.0),
                                // The point is commonly enclosed by the emissive
                                // faces that produced it (for example a six-face
                                // lamp cube). Those faces would otherwise trap all
                                // light at its source.
                                casts_shadow: false,
                                point_lobe,
                            });
                        }
                    }
                    continue;
                }

                let total_area: f32 = triangles.iter().map(|triangle| triangle.3).sum();
                let (max_samples, spacing) = emitter_sample_limits(app.bake_settings.preset);
                let area_samples = (total_area / (spacing * spacing)).ceil() as usize;
                let sample_count = area_samples
                    .max(triangles.len().min(max_samples))
                    .clamp(1, max_samples);
                let mut triangle_index = 0usize;
                let mut cumulative_area = triangles[0].3;
                for sample in 0..sample_count {
                    let target_area = (sample as f32 + 0.5) * total_area / sample_count as f32;
                    while triangle_index + 1 < triangles.len() && cumulative_area < target_area {
                        triangle_index += 1;
                        cumulative_area += triangles[triangle_index].3;
                    }
                    let (a, b, c, _, normal) = triangles[triangle_index];
                    let mut u = ((sample + 1) as f32 * 0.618_034).fract();
                    let mut v = ((sample + 1) as f32 * 0.414_214).fract();
                    if u + v > 1.0 {
                        u = 1.0 - u;
                        v = 1.0 - v;
                    }
                    let point = a + (b - a) * u + (c - a) * v + normal * 2.0;
                    lights.push(EditorLight {
                        name: format!(
                            "Emitter {} material {} sample {}",
                            dff_name,
                            part.material_index,
                            sample + 1
                        ),
                        attached_to: None,
                        kind: LightKind::Area,
                        profile,
                        position: V3 {
                            x: point.x,
                            y: point.y,
                            z: point.z,
                        },
                        direction: V3 {
                            x: normal.x,
                            y: normal.y,
                            z: normal.z,
                        },
                        color: v3_clamp01(color),
                        temperature: emitter.temperature,
                        use_temperature: emitter.use_temperature,
                        // Each quadrature point represents an equal portion of
                        // the emitting surface. The area-light evaluator adds
                        // the two cosine terms and inverse-square geometry.
                        intensity: emitter.strength.max(0.0)
                            * MATERIAL_EMITTER_RADIANCE_SCALE
                            * total_area
                            / (sample_count as f32 * std::f32::consts::PI),
                        radius: emitter.falloff_distance.clamp(1.0, 100_000.0),
                        casts_shadow: true,
                        point_lobe: PointLightLobe::Omni,
                    });
                }
            }
        }
    }
    lights
}

fn all_bake_lights_for_mode(
    app: &AppState,
    face_emitters: &HashMap<(String, usize), MaterialEmitter>,
    mode: BakeLightMode,
) -> Vec<EditorLight> {
    let expanded = expanded_scene_lights(app)
        .into_iter()
        .map(|(_, _, light)| light)
        .collect::<Vec<_>>();
    let mut lights = bake_lights_for_mode(&expanded, mode);
    lights.extend(material_emitter_lights(app, face_emitters, mode));
    lights
}

fn local_light_bounds(light: &EditorLight) -> Option<(Vec3, Vec3, Vec3)> {
    if !matches!(
        light.kind,
        LightKind::Point | LightKind::Spot | LightKind::Area
    ) {
        return None;
    }
    let center = vec3(light.position.x, light.position.y, light.position.z);
    let radius = light.radius.max(0.0);
    if !center.is_finite() || !radius.is_finite() || radius <= 0.0 {
        return None;
    }
    let extent = Vec3::splat(radius);
    Some((center, center - extent, center + extent))
}

fn build_bake_light_index_node(
    lights: &[EditorLight],
    nodes: &mut Vec<BakeLightIndexNode>,
    mut indices: Vec<usize>,
) -> usize {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    let mut center_min = Vec3::splat(f32::INFINITY);
    let mut center_max = Vec3::splat(f32::NEG_INFINITY);
    for idx in &indices {
        if let Some((center, light_min, light_max)) = local_light_bounds(&lights[*idx]) {
            min = min.min(light_min);
            max = max.max(light_max);
            center_min = center_min.min(center);
            center_max = center_max.max(center);
        }
    }

    let node_idx = nodes.len();
    nodes.push(BakeLightIndexNode {
        min,
        max,
        left: None,
        right: None,
        lights: Vec::new(),
    });
    const LEAF_LIGHTS: usize = 16;
    if indices.len() <= LEAF_LIGHTS {
        nodes[node_idx].lights = indices;
        return node_idx;
    }

    let spread = center_max - center_min;
    let axis = if spread.x >= spread.y && spread.x >= spread.z {
        0
    } else if spread.y >= spread.z {
        1
    } else {
        2
    };
    indices.sort_unstable_by(|a, b| {
        let a = local_light_bounds(&lights[*a])
            .map(|value| value.0[axis])
            .unwrap_or(0.0);
        let b = local_light_bounds(&lights[*b])
            .map(|value| value.0[axis])
            .unwrap_or(0.0);
        a.total_cmp(&b)
    });
    let right_indices = indices.split_off(indices.len() / 2);
    let left = build_bake_light_index_node(lights, nodes, indices);
    let right = build_bake_light_index_node(lights, nodes, right_indices);
    nodes[node_idx].left = Some(left);
    nodes[node_idx].right = Some(right);
    node_idx
}

fn build_bake_light_index(lights: &[EditorLight]) -> BakeLightIndex {
    let mut index = BakeLightIndex::default();
    let mut local_lights = Vec::new();
    for (light_idx, light) in lights.iter().enumerate() {
        if local_light_bounds(light).is_some() {
            local_lights.push(light_idx);
        } else {
            index.global_lights.push(light_idx);
        }
    }
    if !local_lights.is_empty() {
        index.root = Some(build_bake_light_index_node(
            lights,
            &mut index.nodes,
            local_lights,
        ));
    }
    index
}

fn light_index_node_contains(node: &BakeLightIndexNode, point: Vec3) -> bool {
    point.x >= node.min.x
        && point.x <= node.max.x
        && point.y >= node.min.y
        && point.y <= node.max.y
        && point.z >= node.min.z
        && point.z <= node.max.z
}

fn visit_bake_light_candidates(
    index: &BakeLightIndex,
    lights: &[EditorLight],
    point: Vec3,
    mut visit: impl FnMut(usize),
) {
    for light_idx in &index.global_lights {
        visit(*light_idx);
    }
    let mut nodes = Vec::with_capacity(32);
    if let Some(root) = index.root {
        nodes.push(root);
    }
    while let Some(node_idx) = nodes.pop() {
        let Some(node) = index.nodes.get(node_idx) else {
            continue;
        };
        if !light_index_node_contains(node, point) {
            continue;
        }
        if let Some(left) = node.left {
            nodes.push(left);
        }
        if let Some(right) = node.right {
            nodes.push(right);
        }
        for light_idx in &node.lights {
            let Some(light) = lights.get(*light_idx) else {
                continue;
            };
            let center = vec3(light.position.x, light.position.y, light.position.z);
            if (center - point).length_squared() <= light.radius.max(0.0).powi(2) {
                visit(*light_idx);
            }
        }
    }
}

fn material_self_emission(
    app: &AppState,
    face_emitters: &HashMap<(String, usize), MaterialEmitter>,
    mesh_key: &str,
    part_idx: usize,
    vertex_idx: usize,
    mode: BakeLightMode,
) -> Vec3 {
    if !app.bake_settings.face_emitters_enabled {
        return Vec3::ZERO;
    }
    let Some(mesh) = app.meshes.get(mesh_key) else {
        return Vec3::ZERO;
    };
    let Some(part) = mesh.parts.get(part_idx) else {
        return Vec3::ZERO;
    };
    let dff_name = mesh_key.split('|').next().unwrap_or(mesh_key);
    let Some(face_index) = part.face_indices.get(vertex_idx / 3).copied() else {
        return Vec3::ZERO;
    };
    let Some(emitter) = material_emitter_for_face(app, face_emitters, dff_name, part, face_index)
    else {
        return Vec3::ZERO;
    };
    if !material_emitter_active(emitter, mode) {
        return Vec3::ZERO;
    }
    material_emitter_surface_color(emitter, part.material_color)
}

/// The visible source surface is emissive independently of how much light it
/// casts. This lets a small fixture remain visibly on while Point strength is
/// tuned down to avoid over-lighting the surrounding scene.
fn material_emitter_surface_color(emitter: MaterialEmitter, material_color: V3) -> Vec3 {
    let color = if emitter.use_material_color {
        material_color
    } else if emitter.use_temperature {
        color_from_temperature(emitter.temperature)
    } else {
        emitter.color
    };
    vec3(color.x, color.y, color.z).clamp(Vec3::ZERO, Vec3::ONE)
}

#[derive(Clone)]
struct MaterialSelfEmissionUpdate {
    mesh_key: String,
    part_idx: usize,
    vertex_idx: usize,
    day: Option<V3>,
    night: Option<V3>,
}

fn collect_material_self_emission_updates(
    app: &AppState,
    face_emitters: &HashMap<(String, usize), MaterialEmitter>,
    mesh_keys: &[String],
    mode: BakeLightMode,
) -> Vec<MaterialSelfEmissionUpdate> {
    let mut updates = Vec::new();
    for mesh_key in mesh_keys {
        let Some(mesh) = app.meshes.get(mesh_key) else {
            continue;
        };
        for (part_idx, part) in mesh.parts.iter().enumerate() {
            for vertex_idx in 0..part.cpu_vertices.len() {
                let day = matches!(mode, BakeLightMode::Day | BakeLightMode::Both)
                    .then(|| {
                        material_self_emission(
                            app,
                            face_emitters,
                            mesh_key,
                            part_idx,
                            vertex_idx,
                            BakeLightMode::Day,
                        )
                    })
                    .filter(|color| color.max_element() > 0.0);
                let night = matches!(mode, BakeLightMode::Night | BakeLightMode::Both)
                    .then(|| {
                        material_self_emission(
                            app,
                            face_emitters,
                            mesh_key,
                            part_idx,
                            vertex_idx,
                            BakeLightMode::Night,
                        )
                    })
                    .filter(|color| color.max_element() > 0.0);
                if day.is_some() || night.is_some() {
                    updates.push(MaterialSelfEmissionUpdate {
                        mesh_key: mesh_key.clone(),
                        part_idx,
                        vertex_idx,
                        day: day.map(|color| V3 {
                            x: color.x,
                            y: color.y,
                            z: color.z,
                        }),
                        night: night.map(|color| V3 {
                            x: color.x,
                            y: color.y,
                            z: color.z,
                        }),
                    });
                }
            }
        }
    }
    updates
}

fn apply_material_self_emission_updates(
    app: &mut AppState,
    updates: Vec<MaterialSelfEmissionUpdate>,
) -> usize {
    let mut applied = 0usize;
    for update in updates {
        let Some(vertex) = app
            .meshes
            .get_mut(&update.mesh_key)
            .and_then(|mesh| mesh.parts.get_mut(update.part_idx))
            .and_then(|part| part.cpu_vertices.get_mut(update.vertex_idx))
        else {
            continue;
        };
        if let Some(day) = update.day {
            vertex.day_color.x = vertex.day_color.x.max(day.x);
            vertex.day_color.y = vertex.day_color.y.max(day.y);
            vertex.day_color.z = vertex.day_color.z.max(day.z);
        }
        if let Some(night) = update.night {
            vertex.night_color.x = vertex.night_color.x.max(night.x);
            vertex.night_color.y = vertex.night_color.y.max(night.y);
            vertex.night_color.z = vertex.night_color.z.max(night.z);
        }
        apply_vertex_bake_display(vertex, app.bake_settings.light_mode);
        applied += 1;
    }
    applied
}

pub(crate) fn first_gpu_shadow_light(lights: &[EditorLight]) -> Option<usize> {
    lights.iter().position(|light| {
        light.casts_shadow
            && matches!(
                light.kind,
                LightKind::Directional | LightKind::Point | LightKind::Spot | LightKind::Area
            )
    })
}

pub(crate) fn transformed_bounds_corners(bounds: Bounds, model: &[f32; 16]) -> [Vec3; 8] {
    let min = bounds.min;
    let max = bounds.max;
    [
        transform_point_gl(
            model,
            V3 {
                x: min.x,
                y: min.y,
                z: min.z,
            },
        ),
        transform_point_gl(
            model,
            V3 {
                x: max.x,
                y: min.y,
                z: min.z,
            },
        ),
        transform_point_gl(
            model,
            V3 {
                x: min.x,
                y: max.y,
                z: min.z,
            },
        ),
        transform_point_gl(
            model,
            V3 {
                x: max.x,
                y: max.y,
                z: min.z,
            },
        ),
        transform_point_gl(
            model,
            V3 {
                x: min.x,
                y: min.y,
                z: max.z,
            },
        ),
        transform_point_gl(
            model,
            V3 {
                x: max.x,
                y: min.y,
                z: max.z,
            },
        ),
        transform_point_gl(
            model,
            V3 {
                x: min.x,
                y: max.y,
                z: max.z,
            },
        ),
        transform_point_gl(
            model,
            V3 {
                x: max.x,
                y: max.y,
                z: max.z,
            },
        ),
    ]
}

pub(crate) fn transformed_bounds(bounds: Bounds, model: &[f32; 16]) -> Bounds {
    let corners = transformed_bounds_corners(bounds, model);
    let mut min = corners[0];
    let mut max = corners[0];
    for corner in corners.iter().skip(1) {
        min = min.min(*corner);
        max = max.max(*corner);
    }
    Bounds { min, max }
}

pub(crate) fn active_scene_bounds(app: &AppState) -> Option<Bounds> {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    let mut found = false;
    for (idx, placement) in app.placements.iter().enumerate() {
        if app
            .element_states
            .get(idx)
            .is_some_and(|state| state.deleted)
        {
            continue;
        }
        let Some(mesh) = element_mesh(app, placement) else {
            continue;
        };
        let model = placement_matrix(placement).to_cols_array();
        for corner in transformed_bounds_corners(mesh.bounds, &model) {
            min = min.min(corner);
            max = max.max(corner);
            found = true;
        }
    }
    found.then_some(Bounds { min, max })
}

pub(crate) fn gpu_shadow_matrices_for_receiver(
    receiver_bounds: Bounds,
    _scene_bounds: Bounds,
    light: &EditorLight,
) -> Option<(Mat4, Mat4)> {
    let center = (receiver_bounds.min + receiver_bounds.max) * 0.5;
    let radius = (receiver_bounds.max - receiver_bounds.min)
        .length()
        .max(512.0)
        * 0.55;
    match light.kind {
        LightKind::Directional => {
            let dir = vec3(light.direction.x, light.direction.y, light.direction.z);
            if dir.length_squared() < 0.0001 {
                return None;
            }
            let dir = dir.normalize();
            let caster_distance = directional_shadow_caster_distance(receiver_bounds, dir);
            // Direction is the direction light travels, so the source and
            // shadow camera are on the opposite side of the receiver.
            // Keeping the depth corridor local preserves useful precision.
            let eye = center - dir * (caster_distance + radius * 2.0);
            let up = if dir.z.abs() < 0.95 { Vec3::Z } else { Vec3::Y };
            let view = Mat4::look_at_rh(eye, center, up);

            let receiver_corners =
                transformed_bounds_corners(receiver_bounds, &Mat4::IDENTITY.to_cols_array());
            let mut min_x = f32::INFINITY;
            let mut max_x = f32::NEG_INFINITY;
            let mut min_y = f32::INFINITY;
            let mut max_y = f32::NEG_INFINITY;
            let mut min_z = f32::INFINITY;
            let mut max_z = f32::NEG_INFINITY;
            for corner in receiver_corners {
                let point = view.transform_point3(corner);
                min_x = min_x.min(point.x);
                max_x = max_x.max(point.x);
                min_y = min_y.min(point.y);
                max_y = max_y.max(point.y);
                min_z = min_z.min(point.z);
                max_z = max_z.max(point.z);
            }
            let xy_pad = ((max_x - min_x).max(max_y - min_y) * 0.02).max(4.0);
            let z_pad = ((max_z - min_z).abs() * 0.02).max(4.0);
            let near = (-max_z - caster_distance - z_pad).max(0.1);
            let far = (-min_z + z_pad).max(near + 1.0);
            let projection = Mat4::orthographic_rh_gl(
                min_x - xy_pad,
                max_x + xy_pad,
                min_y - xy_pad,
                max_y + xy_pad,
                near,
                far,
            );
            Some((projection, view))
        }
        LightKind::Spot => {
            let eye = vec3(light.position.x, light.position.y, light.position.z);
            let dir = vec3(light.direction.x, light.direction.y, light.direction.z);
            if dir.length_squared() < 0.0001 {
                return None;
            }
            let dir = dir.normalize();
            let far = light
                .radius
                .max((center - eye).length() + radius)
                .max(128.0);
            let up = if dir.z.abs() < 0.95 { Vec3::Z } else { Vec3::Y };
            let view = Mat4::look_at_rh(eye, eye + dir * far, up);
            let projection = Mat4::perspective_rh_gl(72.0_f32.to_radians(), 1.0, 2.0, far);
            Some((projection, view))
        }
        LightKind::Ambient | LightKind::Point | LightKind::Area => None,
    }
}

pub(crate) fn directional_shadow_caster_distance(bounds: Bounds, direction: Vec3) -> f32 {
    if direction.length_squared() < 0.0001 {
        return 256.0;
    }
    let size = (bounds.max - bounds.min).abs();
    let local_width = size.x.hypot(size.y).max(128.0);
    let plausible_caster_height = (local_width * 0.5).clamp(128.0, 1024.0);
    let elevation = direction.normalize().z.abs().max(0.08);
    (plausible_caster_height / elevation).clamp(256.0, 2048.0)
}

pub(crate) fn directional_shadow_depth_range(bounds: Bounds, direction: Vec3) -> f32 {
    if direction.length_squared() < 0.0001 {
        return 1.0;
    }
    let dir = direction.normalize().abs();
    let size = (bounds.max - bounds.min).abs();
    let caster_distance = directional_shadow_caster_distance(bounds, direction);
    (dir.dot(size) + caster_distance + (size.length() * 0.02).max(4.0) * 2.0).max(1.0)
}

pub(crate) fn shadow_chunk_grid(chunks: usize) -> (usize, usize) {
    let chunks = chunks.max(1);
    // The grid must contain exactly `chunks` cells. A ceil(sqrt) grid makes
    // eight chunks into a 3x3 grid; the ninth cell is then clamped onto chunk
    // seven even though chunk seven's shadow map covers a different cell.
    // Final quality uses eight chunks, so that mismatch leaves a substantial
    // part of the scene sampling an unrelated directional-shadow projection.
    let mut rows = (chunks as f32).sqrt().floor() as usize;
    while rows > 1 && !chunks.is_multiple_of(rows) {
        rows -= 1;
    }
    let cols = chunks / rows.max(1);
    (cols.max(1), rows.max(1))
}

pub(crate) fn shadow_chunk_index(pos: Vec3, bounds: Bounds, chunks: usize) -> usize {
    let (cols, rows) = shadow_chunk_grid(chunks);
    let width = (bounds.max.x - bounds.min.x).abs().max(1.0);
    let depth = (bounds.max.y - bounds.min.y).abs().max(1.0);
    let col = (((pos.x - bounds.min.x) / width) * cols as f32)
        .floor()
        .clamp(0.0, (cols - 1) as f32) as usize;
    let row = (((pos.y - bounds.min.y) / depth) * rows as f32)
        .floor()
        .clamp(0.0, (rows - 1) as f32) as usize;
    (row * cols + col).min(chunks.saturating_sub(1))
}

pub(crate) fn shadow_chunk_bounds(bounds: Bounds, chunk: usize, chunks: usize) -> Bounds {
    let (cols, rows) = shadow_chunk_grid(chunks);
    let col = chunk % cols;
    let row = chunk / cols;
    let width = (bounds.max.x - bounds.min.x) / cols as f32;
    let depth = (bounds.max.y - bounds.min.y) / rows as f32;
    let min_x = bounds.min.x + col as f32 * width;
    let min_y = bounds.min.y + row as f32 * depth;
    let mut max_x = min_x + width;
    let mut max_y = min_y + depth;
    if col == cols - 1 {
        max_x = bounds.max.x;
    }
    if row == rows - 1 {
        max_y = bounds.max.y;
    }
    let pad = width.abs().max(depth.abs()).max(512.0) * 0.2;
    Bounds {
        min: vec3(min_x - pad, min_y - pad, bounds.min.z),
        max: vec3(max_x + pad, max_y + pad, bounds.max.z),
    }
}

pub(crate) fn draw_gpu_shadow_depth_geometry(app: &AppState) -> usize {
    let mut vertices = 0usize;
    unsafe {
        gl::Begin(gl::TRIANGLES);
        for (idx, placement) in app.placements.iter().enumerate() {
            if app
                .element_states
                .get(idx)
                .is_some_and(|state| state.deleted)
            {
                continue;
            }
            let Some(mesh) = element_mesh(app, placement) else {
                continue;
            };
            let mesh_key = placement_mesh_key(placement, &app.definitions);
            let dff_name = mesh_key.split('|').next().unwrap_or(&mesh_key);
            let model = placement_matrix(placement).to_cols_array();
            for part in &mesh.parts {
                for (part_face, tri) in part.cpu_vertices.chunks_exact(3).enumerate() {
                    if !dff_face_casts_shadow(app, dff_name, part, part_face) {
                        continue;
                    }
                    for vertex in tri {
                        let pos = transform_point_gl(&model, vertex.pos);
                        gl::Vertex3f(pos.x, pos.y, pos.z);
                        vertices += 1;
                    }
                }
            }
        }
        gl::End();
    }
    vertices
}

pub(crate) fn render_gpu_shadow_map_for_bounds(
    app: &AppState,
    lights: &[EditorLight],
    light_index: usize,
    bounds: Bounds,
) -> Result<Mat4, String> {
    let pipeline = &app.gpu_lightmap;
    if pipeline.shadow_fbo == 0 || pipeline.shadow_depth_texture == 0 {
        return Err("GPU shadow framebuffer is unavailable".to_string());
    }
    let Some(light) = lights.get(light_index) else {
        return Err("Shadow light is missing".to_string());
    };
    let scene_bounds = active_scene_bounds(app).unwrap_or(bounds);
    let Some((projection, view)) = gpu_shadow_matrices_for_receiver(bounds, scene_bounds, light)
    else {
        return Err("Shadow light cannot be projected to a 2D shadow map".to_string());
    };
    unsafe {
        gl::UseProgram(0);
        gl::BindFramebuffer(gl::FRAMEBUFFER, pipeline.shadow_fbo);
        // Point/area lights reuse this FBO and replace its depth attachment
        // with individual cubemap faces. Always restore the directional 2D
        // depth texture before rendering a sun/moon/spot shadow pass.
        gl::FramebufferTexture2D(
            gl::FRAMEBUFFER,
            gl::DEPTH_ATTACHMENT,
            gl::TEXTURE_2D,
            pipeline.shadow_depth_texture,
            0,
        );
        if gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE {
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            return Err("GPU directional shadow framebuffer is incomplete".to_string());
        }
        gl::DrawBuffer(gl::NONE);
        gl::ReadBuffer(gl::NONE);
        gl::Viewport(0, 0, GPU_SHADOW_MAP_SIZE, GPU_SHADOW_MAP_SIZE);
        // The scene viewport uses scissoring. A bake can be requested while
        // that state is still live, and glClear obeys the scissor rectangle.
        // Reusing a partially-cleared depth texture across directional chunks
        // makes old chunks shadow unrelated parts of the scene.
        gl::Disable(gl::SCISSOR_TEST);
        gl::Disable(gl::MULTISAMPLE);
        gl::ColorMask(gl::FALSE, gl::FALSE, gl::FALSE, gl::FALSE);
        gl::DepthMask(gl::TRUE);
        gl::ClearDepth(1.0);
        gl::Clear(gl::DEPTH_BUFFER_BIT);
        gl::Disable(gl::RASTERIZER_DISCARD);
        gl::Enable(gl::DEPTH_TEST);
        gl::DepthFunc(gl::LEQUAL);
        gl::Disable(gl::BLEND);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::ALPHA_TEST);
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::CULL_FACE);
        gl::Enable(gl::POLYGON_OFFSET_FILL);
        gl::PolygonOffset(1.0, 2.0);
        gl::MatrixMode(gl::PROJECTION);
        gl::LoadMatrixf(projection.to_cols_array().as_ptr());
        gl::MatrixMode(gl::MODELVIEW);
        gl::LoadMatrixf(view.to_cols_array().as_ptr());
    }
    let rendered_vertices = draw_gpu_shadow_depth_geometry(app);
    unsafe {
        gl::Disable(gl::POLYGON_OFFSET_FILL);
        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
        gl::Viewport(0, 0, screen_width() as i32, screen_height() as i32);
        gl::BindTexture(gl::TEXTURE_2D, 0);
    }
    if rendered_vertices == 0 {
        return Err("GPU shadow map had no geometry to render".to_string());
    }
    Ok(projection * view)
}

pub(crate) fn render_gpu_shadow_map(
    app: &AppState,
    lights: &[EditorLight],
    light_index: usize,
) -> Result<Mat4, String> {
    let Some(bounds) = active_scene_bounds(app) else {
        return Err("GPU shadow map has no scene bounds".to_string());
    };
    render_gpu_shadow_map_for_bounds(app, lights, light_index, bounds)
}

pub(crate) fn point_shadow_face_view(position: Vec3, face: u32) -> Mat4 {
    match face {
        0 => Mat4::look_at_rh(position, position + Vec3::X, -Vec3::Y),
        1 => Mat4::look_at_rh(position, position - Vec3::X, -Vec3::Y),
        2 => Mat4::look_at_rh(position, position + Vec3::Y, Vec3::Z),
        3 => Mat4::look_at_rh(position, position - Vec3::Y, -Vec3::Z),
        4 => Mat4::look_at_rh(position, position + Vec3::Z, -Vec3::Y),
        _ => Mat4::look_at_rh(position, position - Vec3::Z, -Vec3::Y),
    }
}

pub(crate) fn render_gpu_point_shadow_map(
    app: &AppState,
    lights: &[EditorLight],
    light_index: usize,
) -> Result<(Vec3, f32), String> {
    let pipeline = &app.gpu_lightmap;
    if pipeline.point_shadow_fbo == 0 || pipeline.point_shadow_depth_texture == 0 {
        return Err("GPU point shadow framebuffer is unavailable".to_string());
    }
    let Some(light) = lights.get(light_index) else {
        return Err("Point shadow light is missing".to_string());
    };
    if !matches!(light.kind, LightKind::Point | LightKind::Area) {
        return Err("Point shadow map requested for a non-point light".to_string());
    }
    let position = vec3(light.position.x, light.position.y, light.position.z);
    let far = light.radius.max(32.0);
    let projection = Mat4::perspective_rh_gl(90.0_f32.to_radians(), 1.0, 2.0, far);
    let mut rendered_vertices = 0usize;
    unsafe {
        gl::UseProgram(0);
        gl::BindFramebuffer(gl::FRAMEBUFFER, pipeline.point_shadow_fbo);
        gl::DrawBuffer(gl::NONE);
        gl::ReadBuffer(gl::NONE);
        gl::Viewport(0, 0, GPU_POINT_SHADOW_MAP_SIZE, GPU_POINT_SHADOW_MAP_SIZE);
        gl::Disable(gl::SCISSOR_TEST);
        gl::Disable(gl::MULTISAMPLE);
        gl::ColorMask(gl::FALSE, gl::FALSE, gl::FALSE, gl::FALSE);
        gl::DepthMask(gl::TRUE);
        gl::ClearDepth(1.0);
        gl::Disable(gl::RASTERIZER_DISCARD);
        gl::Enable(gl::DEPTH_TEST);
        gl::DepthFunc(gl::LEQUAL);
        gl::Disable(gl::BLEND);
        gl::Disable(gl::LIGHTING);
        gl::Disable(gl::ALPHA_TEST);
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::CULL_FACE);
        gl::Enable(gl::POLYGON_OFFSET_FILL);
        gl::PolygonOffset(1.0, 2.0);
    }
    for face in 0..6 {
        let view = point_shadow_face_view(position, face);
        unsafe {
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::DEPTH_ATTACHMENT,
                gl::TEXTURE_CUBE_MAP_POSITIVE_X + face,
                pipeline.point_shadow_depth_texture,
                0,
            );
            if gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE {
                gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
                gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
                gl::Disable(gl::POLYGON_OFFSET_FILL);
                return Err("GPU point shadow framebuffer face is incomplete".to_string());
            }
            gl::Clear(gl::DEPTH_BUFFER_BIT);
            gl::MatrixMode(gl::PROJECTION);
            gl::LoadMatrixf(projection.to_cols_array().as_ptr());
            gl::MatrixMode(gl::MODELVIEW);
            gl::LoadMatrixf(view.to_cols_array().as_ptr());
        }
        rendered_vertices = rendered_vertices.max(draw_gpu_shadow_depth_geometry(app));
    }
    unsafe {
        gl::Disable(gl::POLYGON_OFFSET_FILL);
        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
        gl::Viewport(0, 0, screen_width() as i32, screen_height() as i32);
        gl::BindTexture(gl::TEXTURE_CUBE_MAP, 0);
    }
    if rendered_vertices == 0 {
        return Err("GPU point shadow map had no geometry to render".to_string());
    }
    Ok((position, far))
}

pub(crate) fn upload_gpu_shadow_uniforms(
    program: u32,
    pipeline: &GpuLightmapPipeline,
    shadow_light_index: Option<usize>,
    shadow_kind: i32,
    shadow_matrix: Option<Mat4>,
    point_shadow: Option<(Vec3, f32)>,
) {
    let shadow_enabled = shadow_light_index.is_some() && shadow_kind != 0;
    unsafe {
        gl::Uniform1i(
            uniform_location(program, "shadow_enabled"),
            if shadow_enabled { 1 } else { 0 },
        );
        gl::Uniform1i(uniform_location(program, "shadow_kind"), shadow_kind);
        gl::Uniform1i(
            uniform_location(program, "shadow_light_index"),
            shadow_light_index.map(|idx| idx as i32).unwrap_or(-1),
        );
        gl::Uniform1f(uniform_location(program, "shadow_bias"), 0.00075);
        // Keep the GPU shadow lookup off the emitting triangle, matching the
        // CPU path's world-space ray-origin lift and avoiding self-shadowing.
        gl::Uniform1f(uniform_location(program, "shadow_normal_offset"), 0.5);
        gl::ActiveTexture(gl::TEXTURE1);
        gl::BindTexture(
            gl::TEXTURE_2D,
            if shadow_kind == 1 {
                pipeline.shadow_depth_texture
            } else {
                0
            },
        );
        gl::Uniform1i(uniform_location(program, "shadow_map"), 1);
        if let Some(matrix) = shadow_matrix {
            gl::UniformMatrix4fv(
                uniform_location(program, "shadow_matrix"),
                1,
                gl::FALSE,
                matrix.to_cols_array().as_ptr(),
            );
        }
        gl::ActiveTexture(gl::TEXTURE2);
        gl::BindTexture(
            gl::TEXTURE_CUBE_MAP,
            if shadow_kind == 2 {
                pipeline.point_shadow_depth_texture
            } else {
                0
            },
        );
        gl::Uniform1i(uniform_location(program, "point_shadow_map"), 2);
        if let Some((position, far)) = point_shadow {
            gl::Uniform3f(
                uniform_location(program, "point_shadow_pos"),
                position.x,
                position.y,
                position.z,
            );
            gl::Uniform1f(uniform_location(program, "point_shadow_near"), 2.0);
            gl::Uniform1f(uniform_location(program, "point_shadow_far"), far);
        }
        gl::ActiveTexture(gl::TEXTURE0);
    }
}

/// Sets the occluder distance window for the directional shadow map (world
/// units). `range` is the shadow map's near→far depth span; 0 disables the
/// window (plain bias test, used by the light bake).
pub(crate) fn upload_gpu_shadow_occluder_window(
    program: u32,
    range: f32,
    min_dist: f32,
    max_dist: f32,
) {
    unsafe {
        gl::Uniform1f(uniform_location(program, "shadow_occluder_range"), range);
        gl::Uniform1f(uniform_location(program, "shadow_occluder_min"), min_dist);
        gl::Uniform1f(uniform_location(program, "shadow_occluder_max"), max_dist);
    }
}

pub(crate) fn run_gpu_transform_feedback(
    app: &AppState,
    input: &[f32],
    output: &mut [f32],
) -> Result<(), String> {
    if input.is_empty() {
        return Ok(());
    }
    let vertices = input.len() / 6;
    if output.len() != vertices * 3 {
        return Err("GPU bake output buffer has the wrong size".to_string());
    }
    unsafe {
        gl::BindBuffer(gl::ARRAY_BUFFER, app.gpu_lightmap.vertex_buffer);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            std::mem::size_of_val(input) as isize,
            input.as_ptr().cast(),
            gl::STREAM_DRAW,
        );
        gl::EnableVertexAttribArray(0);
        gl::EnableVertexAttribArray(1);
        let stride = (6 * std::mem::size_of::<f32>()) as i32;
        gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, stride, std::ptr::null());
        gl::VertexAttribPointer(
            1,
            3,
            gl::FLOAT,
            gl::FALSE,
            stride,
            (3 * std::mem::size_of::<f32>()) as *const c_void,
        );
        gl::BindBuffer(
            gl::TRANSFORM_FEEDBACK_BUFFER,
            app.gpu_lightmap.output_buffer,
        );
        gl::BufferData(
            gl::TRANSFORM_FEEDBACK_BUFFER,
            std::mem::size_of_val(output) as isize,
            std::ptr::null(),
            gl::STREAM_READ,
        );
        gl::BindTransformFeedback(gl::TRANSFORM_FEEDBACK, app.gpu_lightmap.transform_feedback);
        gl::BindBufferBase(
            gl::TRANSFORM_FEEDBACK_BUFFER,
            0,
            app.gpu_lightmap.output_buffer,
        );
        gl::Enable(gl::RASTERIZER_DISCARD);
        gl::BeginTransformFeedback(gl::POINTS);
        gl::DrawArrays(gl::POINTS, 0, vertices as i32);
        gl::EndTransformFeedback();
        gl::Disable(gl::RASTERIZER_DISCARD);
        gl::BindBuffer(
            gl::TRANSFORM_FEEDBACK_BUFFER,
            app.gpu_lightmap.output_buffer,
        );
        gl::GetBufferSubData(
            gl::TRANSFORM_FEEDBACK_BUFFER,
            0,
            std::mem::size_of_val(output) as isize,
            output.as_mut_ptr().cast(),
        );
        gl::BindBufferBase(gl::TRANSFORM_FEEDBACK_BUFFER, 0, 0);
        gl::BindTransformFeedback(gl::TRANSFORM_FEEDBACK, 0);
        gl::DisableVertexAttribArray(0);
        gl::DisableVertexAttribArray(1);
        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
    }
    if output.iter().any(|value| !value.is_finite()) {
        return Err("GPU bake returned non-finite vertex colors".to_string());
    }
    Ok(())
}

/// Runs one GPU light batch. Explicit shadowed lights are isolated because the
/// shadow map is bound per pass; face-emitter area samples are unshadowed GPU
/// batches and CPU baking remains available when their shadow visibility is
/// required.
fn run_gpu_light_batch(
    app: &mut AppState,
    lights: &[EditorLight],
    input: &[f32],
    positions: &[Vec3],
    output: &mut [f32],
) -> Result<(), String> {
    let shadow_light_index = (app.bake_settings.shadow_samples > 0)
        .then(|| lights.iter().position(gpu_light_needs_individual_shadow))
        .flatten();
    let mut shadow_kind = 0i32;
    let mut shadow_matrix = None;
    let mut point_shadow = None;
    if let Some(idx) = shadow_light_index {
        match lights.get(idx).map(|light| light.kind) {
            Some(LightKind::Directional | LightKind::Spot) => {
                shadow_kind = 1;
                if app.bake_settings.shadow_chunks <= 1 {
                    shadow_matrix = Some(render_gpu_shadow_map(app, lights, idx)?);
                }
            }
            Some(LightKind::Point | LightKind::Area) => {
                shadow_kind = 2;
                point_shadow = Some(render_gpu_point_shadow_map(app, lights, idx)?);
            }
            _ => {}
        }
    }
    let chunked_shadow = shadow_kind == 1
        && app.bake_settings.shadow_samples > 0
        && app.bake_settings.shadow_chunks > 1;
    if chunked_shadow {
        let Some(light_idx) = shadow_light_index else {
            return Err("Chunked GPU shadow batch has no shadow light".to_string());
        };
        let Some(scene_bounds) = active_scene_bounds(app) else {
            return Err("Chunked GPU shadow batch has no scene bounds".to_string());
        };
        let chunks = app.bake_settings.shadow_chunks;
        let mut buckets = vec![Vec::<usize>::new(); chunks];
        for (idx, position) in positions.iter().enumerate() {
            buckets[shadow_chunk_index(*position, scene_bounds, chunks)].push(idx);
        }
        for (chunk_idx, bucket) in buckets.iter().enumerate() {
            if bucket.is_empty() {
                continue;
            }
            let bounds = shadow_chunk_bounds(scene_bounds, chunk_idx, chunks);
            let matrix = render_gpu_shadow_map_for_bounds(app, lights, light_idx, bounds)?;
            unsafe {
                gl::UseProgram(app.gpu_lightmap.program);
            }
            upload_gpu_bake_lights(app.gpu_lightmap.program, app.bake_settings, lights);
            upload_gpu_shadow_uniforms(
                app.gpu_lightmap.program,
                &app.gpu_lightmap,
                shadow_light_index,
                shadow_kind,
                Some(matrix),
                None,
            );
            upload_gpu_shadow_occluder_window(app.gpu_lightmap.program, 0.0, 0.0, 0.0);
            let mut chunk_input = Vec::with_capacity(bucket.len() * 6);
            for idx in bucket {
                let base = idx * 6;
                chunk_input.extend_from_slice(&input[base..base + 6]);
            }
            let mut chunk_output = vec![0.0; bucket.len() * 3];
            run_gpu_transform_feedback(app, &chunk_input, &mut chunk_output)?;
            for (local_idx, global_idx) in bucket.iter().enumerate() {
                let src = local_idx * 3;
                let dst = global_idx * 3;
                output[dst..dst + 3].copy_from_slice(&chunk_output[src..src + 3]);
            }
        }
        Ok(())
    } else {
        unsafe {
            gl::UseProgram(app.gpu_lightmap.program);
        }
        upload_gpu_bake_lights(app.gpu_lightmap.program, app.bake_settings, lights);
        upload_gpu_shadow_uniforms(
            app.gpu_lightmap.program,
            &app.gpu_lightmap,
            shadow_light_index,
            shadow_kind,
            shadow_matrix,
            point_shadow,
        );
        upload_gpu_shadow_occluder_window(app.gpu_lightmap.program, 0.0, 0.0, 0.0);
        run_gpu_transform_feedback(app, input, output)
    }
}

fn bake_target_indices(app: &AppState, scope: BakeScope) -> Option<HashSet<usize>> {
    match scope {
        BakeScope::WholeScene => None,
        BakeScope::Selected => Some(selected_live_indices(app).into_iter().collect()),
    }
}

fn placement_in_bake_scope(target_indices: Option<&HashSet<usize>>, idx: usize) -> bool {
    target_indices.is_none_or(|indices| indices.contains(&idx))
}

fn vertex_lighting_exists(app: &AppState, scope: BakeScope) -> bool {
    let target_indices = bake_target_indices(app, scope);
    let mode = app.bake_settings.light_mode;
    let neutral = neutral_vertex_color();
    for (idx, placement) in app.placements.iter().enumerate() {
        if !placement_in_bake_scope(target_indices.as_ref(), idx)
            || app
                .element_states
                .get(idx)
                .is_some_and(|state| state.deleted)
        {
            continue;
        }
        let Some(mesh) = element_mesh(app, placement) else {
            continue;
        };
        for part in &mesh.parts {
            for vertex in &part.cpu_vertices {
                let color = vertex_bake_color(vertex, mode);
                if (color.x - neutral.x).abs() > 0.001
                    || (color.y - neutral.y).abs() > 0.001
                    || (color.z - neutral.z).abs() > 0.001
                {
                    return true;
                }
            }
        }
    }
    false
}

pub(crate) fn request_bake_pass(app: &mut AppState) {
    if app.bake_job.is_some() {
        return;
    }
    let scope = app.bake_settings.scope;
    if scope == BakeScope::Selected && selected_live_indices(app).is_empty() {
        app.status_message = BAKE_EMPTY_SELECTION_MESSAGE.to_string();
        return;
    }
    request_bake_overwrite(app, scope);
}

pub(crate) fn request_bake_overwrite(app: &mut AppState, scope: BakeScope) {
    if vertex_lighting_exists(app, scope) {
        app.confirm_dialog = Some(ConfirmDialog {
            action: ConfirmAction::StartBake(scope),
            title: "Overwrite Vertex Lighting?".to_string(),
            body: format!(
                "Existing {} vertex lighting was found for {}.",
                bake_light_mode_label(app.bake_settings.light_mode),
                bake_scope_label(scope)
            ),
            detail: "Baking will replace the current vertex lighting values for that target."
                .to_string(),
            primary_label: "Bake".to_string(),
            secondary_label: None,
            secondary_action: None,
        });
        return;
    }
    start_bake_pass(app, scope);
}

pub(crate) fn request_clear_bake(app: &mut AppState) {
    if !vertex_lighting_exists(app, BakeScope::WholeScene) {
        app.status_message = format!(
            "No {} vertex lighting to clear.",
            bake_light_mode_label(app.bake_settings.light_mode)
        );
        return;
    }
    app.confirm_dialog = Some(ConfirmDialog {
        action: ConfirmAction::ClearBake,
        title: "Clear Vertex Lighting?".to_string(),
        body: format!(
            "Clear all {} vertex lighting from loaded meshes?",
            bake_light_mode_label(app.bake_settings.light_mode)
        ),
        detail: "This resets the active lighting channel to white and rebuilds the scene preview."
            .to_string(),
        primary_label: "Clear".to_string(),
        secondary_label: None,
        secondary_action: None,
    });
}

/// On the Bake tab a plain left-click paints, so tell users how to actually
/// select an element instead of just complaining about the empty selection.
pub(crate) const BAKE_EMPTY_SELECTION_MESSAGE: &str = "No elements selected: Alt+Left-Click or Ctrl+Right-Click an element in the viewport (left-click paints on this tab), or switch bake scope to Whole Scene.";

pub(crate) fn request_ao_pass(app: &mut AppState) {
    if app.bake_job.is_some() {
        return;
    }
    let scope = app.bake_settings.scope;
    if scope == BakeScope::Selected && selected_live_indices(app).is_empty() {
        app.status_message = BAKE_EMPTY_SELECTION_MESSAGE.to_string();
        return;
    }
    start_ao_pass(app, scope);
}

pub(crate) fn request_ambient_bump_pass(app: &mut AppState) {
    if app.bake_job.is_some() {
        return;
    }
    app.bake_settings = clamp_bake_settings(app.bake_settings);
    let scope = app.bake_settings.scope;
    if scope == BakeScope::Selected && selected_live_indices(app).is_empty() {
        app.status_message = BAKE_EMPTY_SELECTION_MESSAGE.to_string();
        return;
    }
    if app.bake_settings.ambient_bump <= 0.0 {
        app.status_message = "Ambient Bump must be greater than zero".to_string();
        return;
    }
    let target_indices = bake_target_indices(app, scope);
    app.bake_job = Some(BakeJob {
        pass: BakePassKind::AmbientBump,
        settings: app.bake_settings,
        scope,
        target_indices,
        lights: Vec::new(),
        light_index: BakeLightIndex::default(),
        material_face_emitters: HashMap::new(),
        gpu_state: None,
        shadow_grid: ShadowGrid {
            cell_size: 768.0,
            cells: HashMap::new(),
            tris: Vec::new(),
        },
        accumulators: HashMap::new(),
        placement_index: 0,
        part_index: 0,
        vertex_index: 0,
        total_placements: app.placements.len().max(1),
        vertices: 0,
        lit_vertices: 0,
        total_light: Vec3::ZERO,
        started_at: get_time(),
    });
    app.status_message = format!(
        "Applying +{:.2} ambient bump to existing {} vertex lighting for {}...",
        app.bake_settings.ambient_bump,
        bake_light_mode_label(app.bake_settings.light_mode),
        bake_scope_label(scope),
    );
}

/// World AABB around the scoped placements, expanded by the AO radius. AO rays
/// only travel `ao_radius`, so occluders outside this region cannot matter.
fn scoped_ao_occluder_region(
    app: &AppState,
    target_indices: Option<&HashSet<usize>>,
    ao_radius: f32,
) -> Option<Bounds> {
    let indices = target_indices?;
    let mut region: Option<Bounds> = None;
    for idx in indices {
        if app
            .element_states
            .get(*idx)
            .is_some_and(|state| state.deleted)
        {
            continue;
        }
        let placement = app.placements.get(*idx)?;
        let Some(mesh) = element_mesh(app, placement) else {
            continue;
        };
        let model = placement_matrix(placement).to_cols_array();
        let bounds = transformed_bounds(mesh.bounds, &model);
        region = Some(match region {
            Some(current) => Bounds {
                min: current.min.min(bounds.min),
                max: current.max.max(bounds.max),
            },
            None => bounds,
        });
    }
    let pad = ao_radius.max(1.0) + 4.0;
    region.map(|bounds| Bounds {
        min: bounds.min - Vec3::splat(pad),
        max: bounds.max + Vec3::splat(pad),
    })
}

pub(crate) fn start_ao_pass(app: &mut AppState, scope: BakeScope) {
    app.bake_settings = clamp_bake_settings(app.bake_settings);
    let job_settings = app.bake_settings;
    if job_settings.backend == BakeBackend::Gpu {
        let started_at = get_time();
        match run_gpu_ao_pass(app, scope) {
            Ok(()) => {
                app.status_message = format!(
                    "GPU AO for {} ({} sky directions, strength {:.2}) completed in {:.1}s on {}.",
                    bake_scope_label(scope),
                    job_settings.ao_samples,
                    job_settings.ao_strength,
                    get_time() - started_at,
                    app.gpu_lightmap.gl_version
                );
                return;
            }
            Err(err) => {
                app.status_message = format!("GPU AO unavailable ({err}); using CPU.");
            }
        }
    }
    let target_indices = bake_target_indices(app, scope);
    // Selected scope only needs occluders within ao_radius of the selection.
    let occluder_region = if scope == BakeScope::Selected {
        scoped_ao_occluder_region(app, target_indices.as_ref(), job_settings.ao_radius)
    } else {
        None
    };
    let shadow_grid = build_shadow_grid_in_region(app, occluder_region);
    if shadow_grid.tris.is_empty() {
        app.status_message = "No scene geometry available for ambient occlusion.".to_string();
        return;
    }
    app.bake_job = Some(BakeJob {
        pass: BakePassKind::AmbientOcclusion,
        settings: job_settings,
        scope,
        target_indices,
        lights: Vec::new(),
        light_index: BakeLightIndex::default(),
        material_face_emitters: build_material_face_emitter_index(app),
        gpu_state: None,
        shadow_grid,
        accumulators: HashMap::new(),
        placement_index: 0,
        part_index: 0,
        vertex_index: 0,
        // Progress is measured by the placement cursor, which walks every
        // placement (skipping out-of-scope ones), so use the full count.
        total_placements: app.placements.len().max(1),
        vertices: 0,
        lit_vertices: 0,
        total_light: Vec3::ZERO,
        started_at: get_time(),
    });
    app.status_message = format!(
        "Ambient occlusion (CPU) for {}... samples={} radius={:.0} strength={:.2} shadow_tris={}",
        bake_scope_label(scope),
        job_settings.ao_samples,
        job_settings.ao_radius,
        job_settings.ao_strength,
        app.bake_job
            .as_ref()
            .map(|job| job.shadow_grid.tris.len())
            .unwrap_or(0)
    );
}

/// Uniformly distributed direction `sample` of `samples` on the unit sphere
/// (fibonacci spiral). Used as global sky directions for the GPU AO pass.
pub(crate) fn fibonacci_sphere_direction(sample: usize, samples: usize) -> Vec3 {
    let n = samples.max(1) as f32;
    let z = 1.0 - (2.0 * sample as f32 + 1.0) / n;
    let r = (1.0 - z * z).max(0.0).sqrt();
    let phi = sample as f32 * 2.3999631;
    vec3(phi.cos() * r, phi.sin() * r, z)
}

/// GPU ambient occlusion: renders one whole-scene shadow map per sky
/// direction and accumulates visibility through the transform-feedback bake
/// pipeline. Occlusion = 1 - visible/expected (cosine-weighted). Only
/// occluders within `ao_radius` count (distance window in the shader), so
/// results stay local like the CPU raycast path instead of darkening
/// everything that has some sky blocked.
pub(crate) fn run_gpu_ao_pass(app: &mut AppState, scope: BakeScope) -> Result<(), String> {
    if !app.gpu_lightmap.supported {
        return Err("GPU bake pipeline is unavailable".to_string());
    }
    let mut input = Vec::<f32>::new();
    let mut positions = Vec::<Vec3>::new();
    let mut normals = Vec::<Vec3>::new();
    let mut mapping = Vec::<(String, usize, usize)>::new();
    let target_indices = bake_target_indices(app, scope);
    for (idx, placement) in app.placements.iter().enumerate() {
        if !placement_in_bake_scope(target_indices.as_ref(), idx)
            || app
                .element_states
                .get(idx)
                .is_some_and(|state| state.deleted)
        {
            continue;
        }
        let mesh_key = placement_mesh_key(placement, &app.definitions);
        let Some(mesh) = app.meshes.get(&mesh_key) else {
            continue;
        };
        let model = placement_matrix(placement).to_cols_array();
        for (part_idx, part) in mesh.parts.iter().enumerate() {
            for (vertex_idx, vertex) in part.cpu_vertices.iter().enumerate() {
                let pos = transform_point_gl(&model, vertex.pos);
                let normal = transform_normal_gl(&model, vertex.normal).normalize_or_zero();
                input.extend_from_slice(&[pos.x, pos.y, pos.z, normal.x, normal.y, normal.z]);
                positions.push(pos);
                normals.push(normal);
                mapping.push((mesh_key.clone(), part_idx, vertex_idx));
            }
        }
    }
    if mapping.is_empty() {
        return Err("No vertices in scope for ambient occlusion".to_string());
    }
    let samples = app.bake_settings.ao_samples.max(1);
    // Zero out bounce so the shader emits pure N.L * visibility per direction.
    let mut gpu_settings = app.bake_settings;
    gpu_settings.bounces = 0;
    gpu_settings.shadow_softness = 0.0;
    let Some(scene_bounds) = active_scene_bounds(app) else {
        return Err("No scene bounds for ambient occlusion".to_string());
    };
    let shadow_radius = (scene_bounds.max - scene_bounds.min).length().max(512.0) * 0.55;
    // Ignore occluders closer than a couple of shadow-map texels (acne guard)
    // and farther than ao_radius, so the AO stays local instead of measuring
    // whole-sky visibility.
    let texel_size = shadow_radius * 2.0 / GPU_SHADOW_MAP_SIZE as f32;
    let min_occluder = (texel_size * 2.0).max(2.0);
    let max_occluder = app.bake_settings.ao_radius.max(min_occluder * 2.0);
    // Edge AO needs occluder distances the shadow map can actually resolve.
    // If the requested radius is below a few texels, the CPU raycaster will
    // do a far better job, so bail and let start_ao_pass fall back.
    if app.bake_settings.ao_radius < min_occluder * 2.0 {
        return Err(format!(
            "scene shadow map too coarse ({texel_size:.1}u/texel) for AO radius {:.1}u",
            app.bake_settings.ao_radius
        ));
    }
    let mut visible = vec![0.0f32; mapping.len()];
    let mut expected = vec![0.0f32; mapping.len()];
    let mut output = vec![0.0f32; mapping.len() * 3];
    let result = (|| -> Result<(), String> {
        for sample in 0..samples {
            let dir = fibonacci_sphere_direction(sample, samples);
            let sky_light = EditorLight {
                name: "AO Sky Sample".to_string(),
                attached_to: None,
                kind: LightKind::Directional,
                profile: LightProfile::Both,
                position: V3::default(),
                // The stored vector is the direction light travels. Negating
                // the desired receiver-to-sky sample preserves that contract.
                direction: V3 {
                    x: -dir.x,
                    y: -dir.y,
                    z: -dir.z,
                },
                color: V3 {
                    x: 1.0,
                    y: 1.0,
                    z: 1.0,
                },
                temperature: 6500.0,
                use_temperature: false,
                intensity: 1.0,
                radius: 0.0,
                casts_shadow: true,
                point_lobe: PointLightLobe::Omni,
            };
            let depth_range = directional_shadow_depth_range(
                scene_bounds,
                vec3(
                    sky_light.direction.x,
                    sky_light.direction.y,
                    sky_light.direction.z,
                ),
            );
            let lights = [sky_light];
            let matrix = render_gpu_shadow_map_for_bounds(app, &lights, 0, scene_bounds)?;
            unsafe {
                gl::UseProgram(app.gpu_lightmap.program);
            }
            upload_gpu_bake_lights(app.gpu_lightmap.program, gpu_settings, &lights);
            upload_gpu_shadow_uniforms(
                app.gpu_lightmap.program,
                &app.gpu_lightmap,
                Some(0),
                1,
                Some(matrix),
                None,
            );
            upload_gpu_shadow_occluder_window(
                app.gpu_lightmap.program,
                depth_range,
                min_occluder,
                max_occluder,
            );
            run_gpu_transform_feedback(app, &input, &mut output)?;
            for (idx, normal) in normals.iter().enumerate() {
                // Same lighting model as the shader, minus shadowing.
                expected[idx] += normal.dot(dir).max(0.0);
                visible[idx] += output[idx * 3];
            }
        }
        Ok(())
    })();
    unsafe {
        gl::ActiveTexture(gl::TEXTURE1);
        gl::BindTexture(gl::TEXTURE_2D, 0);
        gl::ActiveTexture(gl::TEXTURE2);
        gl::BindTexture(gl::TEXTURE_CUBE_MAP, 0);
        gl::ActiveTexture(gl::TEXTURE0);
        gl::UseProgram(0);
    }
    result?;

    let mut accumulators = HashMap::<String, Vec<Vec<BakedVertexAccum>>>::new();
    let mut occluded_vertices = 0usize;
    let mut total_occlusion = Vec3::ZERO;
    for (idx, (mesh_key, part_idx, vertex_idx)) in mapping.iter().enumerate() {
        let Some(mesh) = app.meshes.get(mesh_key) else {
            continue;
        };
        let mesh_accum = accumulators.entry(mesh_key.to_string()).or_insert_with(|| {
            mesh.parts
                .iter()
                .map(|part| {
                    vec![
                        BakedVertexAccum {
                            color: Vec3::ZERO,
                            samples: 0,
                            secondary_color: Vec3::ZERO,
                            secondary_samples: 0,
                        };
                        part.cpu_vertices.len()
                    ]
                })
                .collect()
        });
        let occlusion = if expected[idx] > 0.0001 {
            (1.0 - visible[idx] / expected[idx]).clamp(0.0, 1.0)
        } else {
            0.0
        };
        if let Some(vertex_accum) = mesh_accum
            .get_mut(*part_idx)
            .and_then(|part| part.get_mut(*vertex_idx))
        {
            vertex_accum.color += Vec3::splat(occlusion);
            vertex_accum.samples += 1;
        }
        if occlusion > 0.0001 {
            occluded_vertices += 1;
            total_occlusion += Vec3::splat(occlusion);
        }
    }
    let job = BakeJob {
        pass: BakePassKind::AmbientOcclusion,
        settings: app.bake_settings,
        scope,
        target_indices,
        lights: Vec::new(),
        light_index: BakeLightIndex::default(),
        material_face_emitters: build_material_face_emitter_index(app),
        gpu_state: None,
        shadow_grid: ShadowGrid {
            cell_size: 768.0,
            cells: HashMap::new(),
            tris: Vec::new(),
        },
        accumulators,
        placement_index: app.placements.len(),
        part_index: 0,
        vertex_index: 0,
        total_placements: mapping.len().max(1),
        vertices: mapping.len(),
        lit_vertices: occluded_vertices,
        total_light: total_occlusion,
        started_at: get_time(),
    };
    finish_bake_pass(app, job);
    Ok(())
}

fn gpu_light_needs_individual_shadow(light: &EditorLight) -> bool {
    light.casts_shadow && !matches!(light.kind, LightKind::Ambient | LightKind::Area)
}

fn gpu_light_batches(lights: &[EditorLight]) -> Vec<Vec<EditorLight>> {
    const MAX_GPU_LIGHTS: usize = 32;
    let mut batches = Vec::new();
    let unshadowed: Vec<EditorLight> = lights
        .iter()
        .filter(|light| !gpu_light_needs_individual_shadow(light))
        .cloned()
        .collect();
    batches.extend(
        unshadowed
            .chunks(MAX_GPU_LIGHTS)
            .map(|batch| batch.to_vec()),
    );
    batches.extend(
        lights
            .iter()
            .filter(|light| gpu_light_needs_individual_shadow(light))
            .cloned()
            .map(|light| vec![light]),
    );
    batches
}

fn prepare_gpu_bake_state(
    app: &AppState,
    scope: BakeScope,
    material_face_emitters: &HashMap<(String, usize), MaterialEmitter>,
) -> Result<(GpuBakeState, Option<HashSet<usize>>), String> {
    if !app.gpu_lightmap.supported {
        return Err("GPU bake pipeline is unavailable".to_string());
    }
    let requested_mode = app.bake_settings.light_mode;
    let mut input = Vec::<f32>::new();
    let mut positions = Vec::<Vec3>::new();
    let mut mapping = Vec::<(String, usize, usize)>::new();
    let target_indices = bake_target_indices(app, scope);
    for (idx, placement) in app.placements.iter().enumerate() {
        if !placement_in_bake_scope(target_indices.as_ref(), idx)
            || app
                .element_states
                .get(idx)
                .is_some_and(|state| state.deleted)
        {
            continue;
        }
        let mesh_key = placement_mesh_key(placement, &app.definitions);
        let Some(mesh) = app.meshes.get(&mesh_key) else {
            continue;
        };
        let model = placement_matrix(placement).to_cols_array();
        for (part_idx, part) in mesh.parts.iter().enumerate() {
            for (vertex_idx, vertex) in part.cpu_vertices.iter().enumerate() {
                let pos = transform_point_gl(&model, vertex.pos);
                let normal = transform_normal_gl(&model, vertex.normal).normalize_or_zero();
                input.extend_from_slice(&[pos.x, pos.y, pos.z, normal.x, normal.y, normal.z]);
                positions.push(pos);
                mapping.push((mesh_key.clone(), part_idx, vertex_idx));
            }
        }
    }
    if mapping.is_empty() {
        return Err("No vertices to bake".to_string());
    }
    let (primary_mode, secondary_mode) = bake_output_modes(requested_mode);
    let mut modes = vec![primary_mode];
    if let Some(secondary_mode) = secondary_mode {
        modes.push(secondary_mode);
    }
    let output_len = mapping.len() * 3;
    let channels: Vec<GpuBakeChannel> = modes
        .into_iter()
        .map(|mode| {
            let lights = all_bake_lights_for_mode(app, material_face_emitters, mode);
            GpuBakeChannel {
                mode,
                batches: gpu_light_batches(&lights),
                next_batch: 0,
                output: vec![0.0; output_len],
            }
        })
        .collect();
    let total_batches = channels.iter().map(|channel| channel.batches.len()).sum();
    Ok((
        GpuBakeState {
            input,
            positions,
            mapping,
            channels,
            current_channel: 0,
            completed_batches: 0,
            total_batches,
            started: false,
        },
        target_indices,
    ))
}

fn reset_gpu_bake_bindings() {
    unsafe {
        gl::ActiveTexture(gl::TEXTURE1);
        gl::BindTexture(gl::TEXTURE_2D, 0);
        gl::ActiveTexture(gl::TEXTURE2);
        gl::BindTexture(gl::TEXTURE_CUBE_MAP, 0);
        gl::ActiveTexture(gl::TEXTURE0);
        gl::UseProgram(0);
    }
}

fn finish_gpu_bake_pass(app: &mut AppState, mut job: BakeJob, state: GpuBakeState) {
    let mut accumulators = HashMap::<String, Vec<Vec<BakedVertexAccum>>>::new();
    let mut lit_vertices = 0usize;
    let mut total = Vec3::ZERO;
    for (idx, (mesh_key, part_idx, vertex_idx)) in state.mapping.iter().enumerate() {
        let Some(mesh) = app.meshes.get(mesh_key) else {
            continue;
        };
        let mesh_accum = accumulators.entry(mesh_key.clone()).or_insert_with(|| {
            mesh.parts
                .iter()
                .map(|part| {
                    vec![
                        BakedVertexAccum {
                            color: Vec3::ZERO,
                            samples: 0,
                            secondary_color: Vec3::ZERO,
                            secondary_samples: 0,
                        };
                        part.cpu_vertices.len()
                    ]
                })
                .collect()
        });
        let base = idx * 3;
        let primary = &state.channels[0];
        let color = vec3(
            primary.output[base],
            primary.output[base + 1],
            primary.output[base + 2],
        );
        let secondary_color = state.channels.get(1).map(|secondary| {
            vec3(
                secondary.output[base],
                secondary.output[base + 1],
                secondary.output[base + 2],
            )
        });
        if let Some(vertex_accum) = mesh_accum
            .get_mut(*part_idx)
            .and_then(|part| part.get_mut(*vertex_idx))
        {
            vertex_accum.color += color;
            vertex_accum.samples += 1;
            if let Some(secondary_color) = secondary_color {
                vertex_accum.secondary_color += secondary_color;
                vertex_accum.secondary_samples += 1;
            }
        }
        if color.length_squared() > 0.0001
            || secondary_color.is_some_and(|value| value.length_squared() > 0.0001)
        {
            lit_vertices += 1;
            total += color + secondary_color.unwrap_or(Vec3::ZERO);
        }
    }
    job.gpu_state = None;
    job.accumulators = accumulators;
    job.placement_index = app.placements.len();
    job.vertices = state.mapping.len();
    job.lit_vertices = lit_vertices;
    job.total_light = total;
    finish_bake_pass(app, job);
}

pub(crate) fn baked_vec3_to_v3(value: Vec3) -> V3 {
    V3 {
        x: value.x.clamp(0.0, 1.0),
        y: value.y.clamp(0.0, 1.0),
        z: value.z.clamp(0.0, 1.0),
    }
}

pub(crate) fn exposed_baked_vec3_to_v3(value: Vec3, exposure: f32) -> V3 {
    baked_vec3_to_v3(value * exposure.clamp(0.0, 4.0))
}

fn add_ambient_bump(value: V3, ambient_bump: f32) -> V3 {
    let bump = ambient_bump.clamp(0.0, 1.0);
    V3 {
        x: (value.x + bump).clamp(0.0, 1.0),
        y: (value.y + bump).clamp(0.0, 1.0),
        z: (value.z + bump).clamp(0.0, 1.0),
    }
}

pub(crate) fn start_bake_pass(app: &mut AppState, scope: BakeScope) {
    app.bake_settings = clamp_bake_settings(app.bake_settings);
    let mut job_settings = app.bake_settings;
    let material_face_emitters = build_material_face_emitter_index(app);
    let bake_lights =
        all_bake_lights_for_mode(app, &material_face_emitters, job_settings.light_mode);
    let has_shadowed_lights = bake_has_shadowed_lights(&bake_lights);
    let has_gpu_shadow_light = first_gpu_shadow_light(&bake_lights).is_some();
    let mut gpu_fallback_reason = None;
    let gpu_can_run_settings =
        app.bake_settings.shadow_samples == 0 || !has_shadowed_lights || has_gpu_shadow_light;
    if app.bake_settings.backend == BakeBackend::Gpu && gpu_can_run_settings {
        match prepare_gpu_bake_state(app, scope, &material_face_emitters) {
            Ok((gpu_state, target_indices)) => {
                let total_batches = gpu_state.total_batches;
                let vertex_count = gpu_state.mapping.len();
                app.bake_job = Some(BakeJob {
                    pass: BakePassKind::Light,
                    settings: job_settings,
                    scope,
                    target_indices,
                    light_index: build_bake_light_index(&bake_lights),
                    lights: bake_lights,
                    material_face_emitters,
                    gpu_state: Some(gpu_state),
                    shadow_grid: ShadowGrid {
                        cell_size: 768.0,
                        cells: HashMap::new(),
                        tris: Vec::new(),
                    },
                    accumulators: HashMap::new(),
                    placement_index: 0,
                    part_index: 0,
                    vertex_index: 0,
                    total_placements: app.placements.len().max(1),
                    vertices: 0,
                    lit_vertices: 0,
                    total_light: Vec3::ZERO,
                    started_at: get_time(),
                });
                app.status_message = format!(
                    "GPU bake queued for {}: {vertex_count} vertices, {total_batches} light batches.",
                    bake_scope_label(scope)
                );
                return;
            }
            Err(err) => {
                gpu_fallback_reason = Some(err);
                job_settings.backend = BakeBackend::Cpu;
            }
        }
    } else if app.bake_settings.backend == BakeBackend::Gpu {
        gpu_fallback_reason = Some("GPU shadow map unavailable".to_string());
        job_settings.backend = BakeBackend::Cpu;
    }
    let needs_shadow_grid = job_settings.shadow_samples > 0 && has_shadowed_lights;
    let shadow_grid = if needs_shadow_grid {
        build_shadow_grid(app)
    } else {
        ShadowGrid {
            cell_size: 768.0,
            cells: HashMap::new(),
            tris: Vec::new(),
        }
    };
    let target_indices = bake_target_indices(app, scope);
    app.bake_job = Some(BakeJob {
        pass: BakePassKind::Light,
        settings: job_settings,
        scope,
        target_indices,
        light_index: build_bake_light_index(&bake_lights),
        lights: bake_lights,
        material_face_emitters,
        gpu_state: None,
        shadow_grid,
        accumulators: HashMap::new(),
        placement_index: 0,
        part_index: 0,
        vertex_index: 0,
        // The placement cursor walks every placement (skipping out-of-scope
        // ones), so progress uses the full count.
        total_placements: app.placements.len().max(1),
        vertices: 0,
        lit_vertices: 0,
        total_light: Vec3::ZERO,
        started_at: get_time(),
    });
    let gpu_fallback_note = gpu_fallback_reason
        .as_deref()
        .map(|reason| format!("; GPU fallback: {reason}"))
        .unwrap_or_default();
    app.status_message = format!(
        "Baking {} for {}... backend={} gpu_status={} shadows={} chunks={} bounces={} shadow_tris={}{}",
        bake_quality_label(job_settings.preset),
        bake_scope_label(scope),
        bake_backend_label(job_settings.backend),
        gpu_lightmap_label(&app.gpu_lightmap),
        job_settings.shadow_samples,
        job_settings.shadow_chunks,
        job_settings.bounces,
        app.bake_job
            .as_ref()
            .map(|job| job.shadow_grid.tris.len())
            .unwrap_or(0),
        gpu_fallback_note,
    );
}

pub(crate) fn bake_progress(job: &BakeJob) -> f32 {
    if let Some(gpu) = job.gpu_state.as_ref() {
        return gpu_bake_progress(gpu);
    }
    (job.placement_index as f32 / job.total_placements.max(1) as f32).clamp(0.0, 1.0)
}

fn gpu_bake_progress(state: &GpuBakeState) -> f32 {
    if state.total_batches == 0 {
        return 0.0;
    }
    (state.completed_batches as f32 / state.total_batches as f32).clamp(0.0, 1.0)
}

fn scale_v3_clamped(value: V3, factor: f32) -> V3 {
    V3 {
        x: (value.x * factor).clamp(0.0, 1.0),
        y: (value.y * factor).clamp(0.0, 1.0),
        z: (value.z * factor).clamp(0.0, 1.0),
    }
}

pub(crate) fn finish_bake_pass(app: &mut AppState, job: BakeJob) {
    let changed_mesh_keys: Vec<String> = job.accumulators.keys().cloned().collect();
    let self_emission_updates = collect_material_self_emission_updates(
        app,
        &job.material_face_emitters,
        &changed_mesh_keys,
        job.settings.light_mode,
    );
    let before = snapshot_with_vertex_colors(app, changed_mesh_keys.iter());
    for (mesh_key, mesh_accum) in job.accumulators {
        if let Some(mesh) = app.meshes.get_mut(&mesh_key) {
            for (part, part_accum) in mesh.parts.iter_mut().zip(mesh_accum.iter()) {
                for (vertex, baked) in part.cpu_vertices.iter_mut().zip(part_accum.iter()) {
                    match job.pass {
                        BakePassKind::Light => {
                            let color = if baked.samples > 0 {
                                exposed_baked_vec3_to_v3(
                                    baked.color / baked.samples as f32,
                                    job.settings.exposure,
                                )
                            } else {
                                V3::default()
                            };
                            if job.settings.light_mode == BakeLightMode::Both {
                                let night_color = if baked.secondary_samples > 0 {
                                    exposed_baked_vec3_to_v3(
                                        baked.secondary_color / baked.secondary_samples as f32,
                                        job.settings.exposure,
                                    )
                                } else {
                                    V3::default()
                                };
                                vertex.day_color = color;
                                vertex.night_color = night_color;
                            } else {
                                set_vertex_bake_color(vertex, job.settings.light_mode, color);
                            }
                        }
                        BakePassKind::AmbientOcclusion => {
                            if baked.samples > 0 {
                                let occlusion =
                                    (baked.color.x / baked.samples as f32).clamp(0.0, 1.0);
                                let factor =
                                    (1.0 - job.settings.ao_strength * occlusion).clamp(0.0, 1.0);
                                // Darken-only: multiply existing channels independently so an
                                // AO pass in Both mode never stomps day/night with each other.
                                match job.settings.light_mode {
                                    BakeLightMode::Day => {
                                        vertex.day_color =
                                            scale_v3_clamped(vertex.day_color, factor)
                                    }
                                    BakeLightMode::Night => {
                                        vertex.night_color =
                                            scale_v3_clamped(vertex.night_color, factor)
                                    }
                                    BakeLightMode::Both => {
                                        vertex.day_color =
                                            scale_v3_clamped(vertex.day_color, factor);
                                        vertex.night_color =
                                            scale_v3_clamped(vertex.night_color, factor);
                                    }
                                }
                            }
                        }
                        BakePassKind::AmbientBump => {
                            if baked.samples > 0 {
                                match job.settings.light_mode {
                                    BakeLightMode::Day => {
                                        vertex.day_color = add_ambient_bump(
                                            vertex.day_color,
                                            job.settings.ambient_bump,
                                        );
                                    }
                                    BakeLightMode::Night => {
                                        vertex.night_color = add_ambient_bump(
                                            vertex.night_color,
                                            job.settings.ambient_bump,
                                        );
                                    }
                                    BakeLightMode::Both => {
                                        vertex.day_color = add_ambient_bump(
                                            vertex.day_color,
                                            job.settings.ambient_bump,
                                        );
                                        vertex.night_color = add_ambient_bump(
                                            vertex.night_color,
                                            job.settings.ambient_bump,
                                        );
                                    }
                                }
                            }
                        }
                    }
                    apply_vertex_bake_display(vertex, job.settings.light_mode);
                }
            }
        }
    }
    let reconciled_corners =
        if job.scope == BakeScope::WholeScene && job.pass != BakePassKind::AmbientBump {
            reconcile_baked_corners(app, job.settings.light_mode)
        } else {
            0
        };
    // Apply this after exposure and corner reconciliation. The emitter's own
    // face stays visibly emissive even with a tiny casting strength, and its
    // color cannot be averaged away into neighboring non-emissive corners.
    let glowing_vertices = apply_material_self_emission_updates(app, self_emission_updates);
    apply_bake_light_mode_to_meshes(&mut app.meshes, job.settings.light_mode);
    rebuild_mesh_part_lists(&mut app.meshes);
    rebuild_render_cells(app);
    let queued = queue_vertex_lighting_meshes(app, changed_mesh_keys.iter());
    let avg = if job.lit_vertices > 0 {
        job.total_light / job.lit_vertices as f32
    } else {
        Vec3::ZERO
    };
    app.status_message = match job.pass {
        BakePassKind::Light => format!(
            "Bake {} ({}) for {} applied to {} vertices with {} lights in {:.1}s; kept {glowing_vertices} emitter-face vertices glowing; blended {reconciled_corners} coincident corners; queued {queued} DFF(s) for Save/Save WIP; shadow_tris={}, lit={}, avg RGB {:.2}/{:.2}/{:.2}.",
            bake_quality_label(job.settings.preset),
            bake_backend_label(job.settings.backend),
            bake_scope_label(job.scope),
            job.vertices,
            job.lights.len(),
            get_time() - job.started_at,
            job.shadow_grid.tris.len(),
            job.lit_vertices,
            avg.x,
            avg.y,
            avg.z
        ),
        BakePassKind::AmbientOcclusion => format!(
            "AO pass for {} applied to {} vertices in {:.1}s (samples={} radius={:.0} strength={:.2}); blended {reconciled_corners} coincident corners; queued {queued} DFF(s) for Save/Save WIP; avg occlusion {:.2}.",
            bake_scope_label(job.scope),
            job.vertices,
            get_time() - job.started_at,
            job.settings.ao_samples,
            job.settings.ao_radius,
            job.settings.ao_strength,
            avg.x
        ),
        BakePassKind::AmbientBump => format!(
            "Added +{:.2} ambient bump to existing {} vertex lighting for {} across {} vertices in {:.1}s; blended {reconciled_corners} coincident corners; queued {queued} DFF(s) for Save/Save WIP.",
            job.settings.ambient_bump,
            bake_light_mode_label(job.settings.light_mode),
            bake_scope_label(job.scope),
            job.vertices,
            get_time() - job.started_at,
        ),
    };
    let label = match job.pass {
        BakePassKind::Light => "Bake Pass",
        BakePassKind::AmbientOcclusion => "AO Pass",
        BakePassKind::AmbientBump => "Ambient Bump",
    };
    commit_vertex_lighting_history(app, label, before, changed_mesh_keys.iter());
}

fn cpu_light_channel_value(
    _app: &AppState,
    job: &BakeJob,
    _mesh_key: &str,
    _part_index: usize,
    _vertex_index: usize,
    mode: BakeLightMode,
    pos: Vec3,
    normal: Vec3,
) -> Vec3 {
    let mut direct = Vec3::ZERO;
    let mut indirect = Vec3::ZERO;
    let bounce_gain = effective_bounce_gain(
        job.settings.bounces,
        job.settings.bounce_strength,
        job.settings.bounce_maximum,
    );
    let mut accumulate_light = |light_idx: usize| {
        let Some(light) = job.lights.get(light_idx) else {
            return;
        };
        if light_active_for_bake(light, mode) {
            direct += bake_light_value(&job.shadow_grid, job.settings, light, pos, normal);
            indirect += bake_indirect_light_value(light, pos) * bounce_gain;
        }
    };
    visit_bake_light_candidates(&job.light_index, &job.lights, pos, &mut accumulate_light);
    direct + indirect
}

fn update_gpu_bake_job(app: &mut AppState, mut job: BakeJob, mut state: GpuBakeState) {
    // Always present the queued 0% state for one frame before issuing GL work.
    // This guarantees that even a slow first shadow batch has visible feedback.
    if !state.started {
        state.started = true;
        job.gpu_state = Some(state);
        app.bake_job = Some(job);
        return;
    }

    while state.current_channel < state.channels.len()
        && state.channels[state.current_channel].next_batch
            >= state.channels[state.current_channel].batches.len()
    {
        state.current_channel += 1;
    }
    if state.current_channel >= state.channels.len() {
        reset_gpu_bake_bindings();
        finish_gpu_bake_pass(app, job, state);
        return;
    }

    let channel_idx = state.current_channel;
    let batch_idx = state.channels[channel_idx].next_batch;
    let batch = state.channels[channel_idx].batches[batch_idx].clone();
    let mut batch_output = vec![0.0f32; state.positions.len() * 3];
    if let Err(err) = run_gpu_light_batch(
        app,
        &batch,
        &state.input,
        &state.positions,
        &mut batch_output,
    ) {
        reset_gpu_bake_bindings();
        job.settings.backend = BakeBackend::Cpu;
        job.gpu_state = None;
        job.placement_index = 0;
        job.part_index = 0;
        job.vertex_index = 0;
        job.vertices = 0;
        job.lit_vertices = 0;
        job.total_light = Vec3::ZERO;
        job.accumulators.clear();
        job.shadow_grid =
            if job.settings.shadow_samples > 0 && bake_has_shadowed_lights(&job.lights) {
                build_shadow_grid(app)
            } else {
                ShadowGrid {
                    cell_size: 768.0,
                    cells: HashMap::new(),
                    tris: Vec::new(),
                }
            };
        app.status_message =
            format!("GPU bake failed ({err}); continuing with responsive CPU bake.");
        app.bake_job = Some(job);
        return;
    }
    reset_gpu_bake_bindings();

    for (total, value) in state.channels[channel_idx]
        .output
        .iter_mut()
        .zip(batch_output)
    {
        *total += value;
    }
    state.channels[channel_idx].next_batch += 1;
    state.completed_batches += 1;
    let progress = state.completed_batches as f32 / state.total_batches.max(1) as f32;
    job.vertices = (state.mapping.len() as f32 * progress)
        .round()
        .min(state.mapping.len() as f32) as usize;
    app.status_message = format!(
        "GPU baking {} for {}... {:.0}% (batch {}/{}, {} vertices)",
        bake_light_mode_label(state.channels[channel_idx].mode),
        bake_scope_label(job.scope),
        progress * 100.0,
        state.completed_batches,
        state.total_batches,
        state.mapping.len(),
    );
    job.gpu_state = Some(state);
    app.bake_job = Some(job);
}

pub(crate) fn update_bake_job(app: &mut AppState) {
    let Some(mut job) = app.bake_job.take() else {
        return;
    };
    if let Some(state) = job.gpu_state.take() {
        update_gpu_bake_job(app, job, state);
        return;
    }
    let frame_start = get_time();
    let mut processed = 0usize;
    const FRAME_SECONDS: f64 = 0.010;
    const CHECK_INTERVAL: usize = 32;
    const MAX_VERTICES_PER_FRAME: usize = 4096;

    loop {
        if job.placement_index >= app.placements.len() {
            finish_bake_pass(app, job);
            return;
        }
        if !placement_in_bake_scope(job.target_indices.as_ref(), job.placement_index)
            || app
                .element_states
                .get(job.placement_index)
                .is_some_and(|state| state.deleted)
        {
            job.placement_index += 1;
            job.part_index = 0;
            job.vertex_index = 0;
            continue;
        }
        let placement = &app.placements[job.placement_index];
        let mesh_key = placement_mesh_key(placement, &app.definitions);
        let Some(mesh) = app.meshes.get(&mesh_key) else {
            job.placement_index += 1;
            job.part_index = 0;
            job.vertex_index = 0;
            continue;
        };
        if job.part_index >= mesh.parts.len() {
            job.placement_index += 1;
            job.part_index = 0;
            job.vertex_index = 0;
            continue;
        }
        let part = &mesh.parts[job.part_index];
        if job.vertex_index >= part.cpu_vertices.len() {
            job.part_index += 1;
            job.vertex_index = 0;
            continue;
        }
        let vertex = &part.cpu_vertices[job.vertex_index];
        let model = placement_matrix(placement).to_cols_array();
        let pos = transform_point_gl(&model, vertex.pos);
        let normal = transform_normal_gl(&model, vertex.normal).normalize_or_zero();
        let (accum, secondary_accum) = match job.pass {
            BakePassKind::Light => {
                let (primary_mode, secondary_mode) = bake_output_modes(job.settings.light_mode);
                let primary = cpu_light_channel_value(
                    app,
                    &job,
                    &mesh_key,
                    job.part_index,
                    job.vertex_index,
                    primary_mode,
                    pos,
                    normal,
                );
                let secondary = secondary_mode.map(|mode| {
                    cpu_light_channel_value(
                        app,
                        &job,
                        &mesh_key,
                        job.part_index,
                        job.vertex_index,
                        mode,
                        pos,
                        normal,
                    )
                });
                (primary, secondary)
            }
            BakePassKind::AmbientOcclusion => (
                Vec3::splat(ao_occlusion_value(
                    &job.shadow_grid,
                    job.settings,
                    pos,
                    normal,
                )),
                None,
            ),
            BakePassKind::AmbientBump => (Vec3::ZERO, None),
        };
        let mesh_accum = job.accumulators.entry(mesh_key.clone()).or_insert_with(|| {
            mesh.parts
                .iter()
                .map(|part| {
                    vec![
                        BakedVertexAccum {
                            color: Vec3::ZERO,
                            samples: 0,
                            secondary_color: Vec3::ZERO,
                            secondary_samples: 0,
                        };
                        part.cpu_vertices.len()
                    ]
                })
                .collect()
        });
        if let Some(vertex_accum) = mesh_accum
            .get_mut(job.part_index)
            .and_then(|part_accum| part_accum.get_mut(job.vertex_index))
        {
            vertex_accum.color += accum;
            vertex_accum.samples += 1;
            if let Some(secondary) = secondary_accum {
                vertex_accum.secondary_color += secondary;
                vertex_accum.secondary_samples += 1;
            }
        }
        job.vertices += 1;
        if accum.length_squared() > 0.0001
            || secondary_accum.is_some_and(|value| value.length_squared() > 0.0001)
        {
            job.lit_vertices += 1;
            job.total_light += accum + secondary_accum.unwrap_or(Vec3::ZERO);
        }
        job.vertex_index += 1;
        processed += 1;
        if processed >= MAX_VERTICES_PER_FRAME
            || (processed % CHECK_INTERVAL == 0 && get_time() - frame_start >= FRAME_SECONDS)
        {
            app.status_message = match job.pass {
                BakePassKind::Light => format!(
                    "Baking {} ({}) for {}... {:.0}% ({} vertices)",
                    bake_quality_label(job.settings.preset),
                    bake_backend_label(job.settings.backend),
                    bake_scope_label(job.scope),
                    bake_progress(&job) * 100.0,
                    job.vertices
                ),
                BakePassKind::AmbientOcclusion => format!(
                    "Ambient occlusion for {}... {:.0}% ({} vertices)",
                    bake_scope_label(job.scope),
                    bake_progress(&job) * 100.0,
                    job.vertices
                ),
                BakePassKind::AmbientBump => format!(
                    "Applying ambient bump to existing {} lighting for {}... {:.0}% ({} vertices)",
                    bake_light_mode_label(job.settings.light_mode),
                    bake_scope_label(job.scope),
                    bake_progress(&job) * 100.0,
                    job.vertices
                ),
            };
            app.bake_job = Some(job);
            return;
        }
    }
}

pub(crate) fn clear_bake_pass(app: &mut AppState) {
    let mut vertices = 0usize;
    let mode = app.bake_settings.light_mode;
    let mesh_keys: Vec<String> = app.meshes.keys().cloned().collect();
    let before = snapshot_with_vertex_colors(app, mesh_keys.iter());
    for mesh in app.meshes.values_mut() {
        for part in &mut mesh.parts {
            for vertex in &mut part.cpu_vertices {
                set_vertex_bake_color(vertex, mode, neutral_vertex_color());
                apply_vertex_bake_display(vertex, mode);
                vertices += 1;
            }
        }
    }
    rebuild_mesh_part_lists(&mut app.meshes);
    rebuild_render_cells(app);
    let queued = queue_vertex_lighting_meshes(app, mesh_keys.iter());
    app.status_message = format!(
        "Cleared {} baked vertex lighting from {vertices} mesh vertices; queued {queued} DFF(s) for Save/Save WIP.",
        bake_light_mode_label(mode),
    );
    commit_vertex_lighting_history(app, "Clear Vertex Lighting", before, mesh_keys.iter());
}

#[cfg(test)]
mod emitter_tests {
    use super::*;

    #[test]
    fn material_emitter_day_and_night_are_independent() {
        let night_only = MaterialEmitter {
            enabled: true,
            day: false,
            night: true,
            ..MaterialEmitter::default()
        };
        assert!(!material_emitter_active(night_only, BakeLightMode::Day));
        assert!(material_emitter_active(night_only, BakeLightMode::Night));
        assert!(material_emitter_active(night_only, BakeLightMode::Both));

        let disabled = MaterialEmitter {
            enabled: false,
            ..night_only
        };
        assert!(!material_emitter_active(disabled, BakeLightMode::Night));
    }

    #[test]
    fn zero_strength_point_lobes_are_not_generated() {
        let emitter = MaterialEmitter {
            strength: 3.0,
            point_up_strength: 0.0,
            point_down_strength: 2.0,
            point_sides_strength: 0.5,
            ..MaterialEmitter::default()
        };
        let lobes = material_point_lobes(emitter);
        assert_eq!(lobes.len(), 2);
        assert_eq!(lobes[0].0, PointLightLobe::Down);
        assert!((lobes[0].2 - 6.0).abs() < 0.0001);
        assert_eq!(lobes[1].0, PointLightLobe::Sides);
        assert!((lobes[1].2 - 1.5).abs() < 0.0001);
    }

    #[test]
    fn emitter_surface_glow_is_independent_of_casting_strength() {
        let emitter = MaterialEmitter {
            use_material_color: false,
            use_temperature: false,
            color: V3 {
                x: 0.2,
                y: 0.4,
                z: 0.8,
            },
            strength: 0.001,
            ..MaterialEmitter::default()
        };
        assert_eq!(
            material_emitter_surface_color(emitter, neutral_vertex_color()),
            vec3(0.2, 0.4, 0.8)
        );
        assert_eq!(
            material_emitter_surface_color(
                MaterialEmitter {
                    strength: 0.0,
                    ..emitter
                },
                neutral_vertex_color(),
            ),
            vec3(0.2, 0.4, 0.8)
        );
    }

    fn cube_emitter_triangles(center: Vec3) -> Vec<MaterialEmitterTriangle> {
        let p = |x, y, z| center + vec3(x, y, z);
        vec![
            (
                p(-1.0, -0.5, -0.5),
                p(-1.0, 0.5, -0.5),
                p(-1.0, 0.0, 1.0),
                1.0,
                Vec3::NEG_X,
            ),
            (
                p(1.0, -0.5, -0.5),
                p(1.0, 0.0, 1.0),
                p(1.0, 0.5, -0.5),
                1.0,
                Vec3::X,
            ),
            (
                p(-0.5, -1.0, -0.5),
                p(0.0, -1.0, 1.0),
                p(0.5, -1.0, -0.5),
                1.0,
                Vec3::NEG_Y,
            ),
            (
                p(-0.5, 1.0, -0.5),
                p(0.5, 1.0, -0.5),
                p(0.0, 1.0, 1.0),
                1.0,
                Vec3::Y,
            ),
            (
                p(-0.5, -0.5, -1.0),
                p(0.5, -0.5, -1.0),
                p(0.0, 1.0, -1.0),
                1.0,
                Vec3::NEG_Z,
            ),
            (
                p(-0.5, -0.5, 1.0),
                p(0.0, 1.0, 1.0),
                p(0.5, -0.5, 1.0),
                1.0,
                Vec3::Z,
            ),
        ]
    }

    #[test]
    fn point_emitter_grouping_keeps_separate_lamps_apart() {
        let mut triangles = cube_emitter_triangles(Vec3::ZERO);
        triangles.extend(cube_emitter_triangles(vec3(10.0, 0.0, 0.0)));
        let groups = point_emitter_groups(&triangles, 2.0);
        assert_eq!(groups.len(), 2);
        assert!(groups.iter().all(|group| group.faces == 6));
        let mut centers: Vec<_> = groups
            .iter()
            .map(|group| group.weighted_center / group.total_area)
            .collect();
        centers.sort_by(|a, b| a.x.total_cmp(&b.x));
        assert!(centers[0].distance(Vec3::ZERO) < 0.0001);
        assert!(centers[1].distance(vec3(10.0, 0.0, 0.0)) < 0.0001);
    }

    #[test]
    fn face_is_the_default_material_emitter_casting_mode() {
        let emitter = MaterialEmitter::default();
        assert_eq!(emitter.cast_mode, MaterialEmitterCastMode::Face);
        assert!((emitter.max_grouping_size - 2.0).abs() < 0.0001);
    }

    #[test]
    fn face_shadow_override_precedes_material_and_global_texture() {
        let mut overrides = HashMap::new();
        overrides.insert(material_emitter_texture_key("glass"), false);
        assert!(!resolve_shadow_casting(
            &overrides,
            "tower.dff",
            2,
            "glass",
            Some(17)
        ));

        overrides.insert(material_emitter_key("tower.dff", 2), false);
        overrides.insert(material_emitter_face_key("tower.dff", 17), true);
        assert!(resolve_shadow_casting(
            &overrides,
            "tower.dff",
            2,
            "glass",
            Some(17)
        ));
        assert!(!resolve_shadow_casting(
            &overrides,
            "tower.dff",
            2,
            "glass",
            Some(18)
        ));
        assert!(resolve_shadow_casting(
            &HashMap::new(),
            "tower.dff",
            2,
            "glass",
            Some(17)
        ));
    }

    #[test]
    fn bounce_fill_is_energy_bounded() {
        assert_eq!(effective_bounce_gain(0, 0.65, 0.12), 0.0);
        assert_eq!(effective_bounce_gain(8, 0.65, 0.0), 0.0);
        assert!(effective_bounce_gain(4, 0.45, 0.12) <= 0.12);
        assert!(effective_bounce_gain(8, 0.65, 0.12) <= 0.12);
        assert!(effective_bounce_gain(8, 0.65, 0.12) > effective_bounce_gain(1, 0.65, 0.12));
        assert!(effective_bounce_gain(8, 0.65, 0.4) > effective_bounce_gain(8, 0.65, 0.12));
    }

    #[test]
    fn bake_exposure_scales_hdr_light_before_vertex_color_clamping() {
        let exposed = exposed_baked_vec3_to_v3(vec3(1.4, 0.8, 0.2), 0.5);
        assert!((exposed.x - 0.7).abs() < 0.0001);
        assert!((exposed.y - 0.4).abs() < 0.0001);
        assert!((exposed.z - 0.1).abs() < 0.0001);

        let neutral = exposed_baked_vec3_to_v3(vec3(1.4, 0.8, 0.2), 1.0);
        assert_eq!(neutral.x, 1.0);
        assert!((neutral.y - 0.8).abs() < 0.0001);
    }

    #[test]
    fn ambient_bump_adds_to_existing_color_and_clamps() {
        let lifted = add_ambient_bump(
            V3 {
                x: 0.2,
                y: 0.4,
                z: 0.95,
            },
            0.1,
        );
        assert!((lifted.x - 0.3).abs() < 0.0001);
        assert!((lifted.y - 0.5).abs() < 0.0001);
        assert_eq!(lifted.z, 1.0);

        let clamped = add_ambient_bump(
            V3 {
                x: 0.9,
                y: 0.1,
                z: 0.0,
            },
            0.2,
        );
        assert_eq!(clamped.x, 1.0);
        assert!((clamped.y - 0.3).abs() < 0.0001);
        assert!((clamped.z - 0.2).abs() < 0.0001);
    }

    #[test]
    fn gpu_shadow_quality_uses_configured_samples_and_tighter_softness() {
        assert_eq!(gpu_shadow_sample_count(1), 1);
        assert_eq!(gpu_shadow_sample_count(128), 128);
        assert_eq!(gpu_shadow_sample_count(1024), 128);
        assert!((gpu_shadow_filter_radius(320.0) - 5.0).abs() < 0.0001);
        assert_eq!(gpu_shadow_filter_radius(0.0), 0.0);
    }

    #[test]
    fn final_shadow_chunks_form_an_exact_grid() {
        assert_eq!(shadow_chunk_grid(8), (4, 2));
        for chunks in 1..=32 {
            let (cols, rows) = shadow_chunk_grid(chunks);
            assert_eq!(cols * rows, chunks);
        }
    }

    #[test]
    fn every_final_shadow_grid_cell_maps_to_its_own_bounds() {
        let bounds = Bounds {
            min: vec3(-400.0, -200.0, -50.0),
            max: vec3(400.0, 200.0, 150.0),
        };
        let (cols, rows) = shadow_chunk_grid(8);
        for row in 0..rows {
            for col in 0..cols {
                let expected = row * cols + col;
                let chunk_bounds = shadow_chunk_bounds(bounds, expected, 8);
                let center = (chunk_bounds.min + chunk_bounds.max) * 0.5;
                assert_eq!(shadow_chunk_index(center, bounds, 8), expected);
            }
        }
    }

    #[test]
    fn bounce_fill_reaches_surfaces_with_zero_direct_lambert_term() {
        let light = EditorLight {
            name: "Grazing sun".to_string(),
            attached_to: None,
            kind: LightKind::Directional,
            profile: LightProfile::Day,
            position: V3::default(),
            direction: V3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            color: neutral_vertex_color(),
            temperature: 6500.0,
            use_temperature: false,
            intensity: 1.0,
            radius: 0.0,
            casts_shadow: false,
            point_lobe: PointLightLobe::Omni,
        };
        let grid = ShadowGrid {
            cell_size: 1.0,
            cells: HashMap::new(),
            tris: Vec::new(),
        };
        let direct = bake_light_value(&grid, default_bake_settings(), &light, Vec3::ZERO, Vec3::Z);
        let fill =
            bake_indirect_light_value(&light, Vec3::ZERO) * effective_bounce_gain(8, 0.65, 0.12);
        assert_eq!(direct, Vec3::ZERO);
        assert!(fill.x > 0.1 && fill.x <= 0.12);
    }

    #[test]
    fn material_light_falloff_fades_smoothly_to_zero() {
        let radius = 100.0;
        let near = smooth_distance_attenuation(10.0, radius);
        let middle = smooth_distance_attenuation(50.0, radius);
        let edge = smooth_distance_attenuation(90.0, radius);
        assert!(near > middle && middle > edge);
        assert!((middle - 0.5).abs() < 0.0001);
        assert_eq!(smooth_distance_attenuation(radius, radius), 0.0);
        assert_eq!(smooth_distance_attenuation(radius + 1.0, radius), 0.0);
    }

    fn test_area_sample(area: f32) -> EditorLight {
        EditorLight {
            name: "Area sample".to_string(),
            attached_to: None,
            kind: LightKind::Area,
            profile: LightProfile::Both,
            position: V3::default(),
            direction: V3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
            color: neutral_vertex_color(),
            temperature: 6500.0,
            use_temperature: false,
            intensity: area / std::f32::consts::PI,
            radius: 1_000.0,
            casts_shadow: false,
            point_lobe: PointLightLobe::Omni,
        }
    }

    #[test]
    fn point_lobes_only_contribute_in_their_configured_directions() {
        let mut light = test_area_sample(1.0);
        light.kind = LightKind::Point;
        light.position = V3::default();

        light.point_lobe = PointLightLobe::Up;
        assert!(point_lobe_attenuation(&light, Vec3::Z) > 0.999);
        assert_eq!(point_lobe_attenuation(&light, Vec3::NEG_Z), 0.0);
        assert_eq!(point_lobe_attenuation(&light, Vec3::X), 0.0);

        light.point_lobe = PointLightLobe::Down;
        assert!(point_lobe_attenuation(&light, Vec3::NEG_Z) > 0.999);
        assert_eq!(point_lobe_attenuation(&light, Vec3::Z), 0.0);

        light.point_lobe = PointLightLobe::Sides;
        assert!(point_lobe_attenuation(&light, Vec3::X) > 0.999);
        assert_eq!(point_lobe_attenuation(&light, Vec3::Z), 0.0);
        assert_eq!(point_lobe_attenuation(&light, Vec3::NEG_Z), 0.0);
    }

    #[test]
    fn light_index_only_visits_local_lights_whose_falloff_reaches_the_vertex() {
        let mut lights = Vec::new();
        for idx in 0..256 {
            let mut light = test_area_sample(1.0);
            light.position.x = idx as f32 * 100.0;
            light.radius = 40.0;
            lights.push(light);
        }
        lights.push(EditorLight {
            name: "Global ambient".to_string(),
            attached_to: None,
            kind: LightKind::Ambient,
            profile: LightProfile::Both,
            position: V3::default(),
            direction: V3::default(),
            color: neutral_vertex_color(),
            temperature: 6500.0,
            use_temperature: false,
            intensity: 1.0,
            radius: 0.0,
            casts_shadow: false,
            point_lobe: PointLightLobe::Omni,
        });

        let index = build_bake_light_index(&lights);
        let mut candidates = Vec::new();
        visit_bake_light_candidates(&index, &lights, vec3(10_000.0, 0.0, 0.0), |idx| {
            candidates.push(idx)
        });
        candidates.sort_unstable();

        assert_eq!(candidates, vec![100, 256]);
    }

    #[test]
    fn gpu_batches_area_samples_without_per_sample_scene_shadow_maps() {
        let mut area = test_area_sample(1.0);
        area.casts_shadow = true;
        assert!(!gpu_light_needs_individual_shadow(&area));

        let mut point = area.clone();
        point.kind = LightKind::Point;
        assert!(gpu_light_needs_individual_shadow(&point));

        let mut lights = vec![area; 65];
        lights.push(point.clone());
        lights.push(point);
        let batches = gpu_light_batches(&lights);
        let batch_lengths: Vec<usize> = batches.iter().map(Vec::len).collect();
        assert_eq!(batch_lengths, vec![32, 32, 1, 1, 1]);
        assert!(
            batches[..3]
                .iter()
                .flatten()
                .all(|light| matches!(light.kind, LightKind::Area))
        );
        assert!(
            batches[3..]
                .iter()
                .flatten()
                .all(gpu_light_needs_individual_shadow)
        );
    }

    #[test]
    fn gpu_progress_tracks_completed_light_batches() {
        let mut state = GpuBakeState {
            input: Vec::new(),
            positions: Vec::new(),
            mapping: Vec::new(),
            channels: Vec::new(),
            current_channel: 0,
            completed_batches: 0,
            total_batches: 8,
            started: false,
        };
        assert_eq!(gpu_bake_progress(&state), 0.0);
        state.completed_batches = 3;
        assert!((gpu_bake_progress(&state) - 0.375).abs() < 0.0001);
        state.completed_batches = 8;
        assert_eq!(gpu_bake_progress(&state), 1.0);
    }

    #[test]
    fn area_emitter_is_one_sided_and_scales_with_surface_area() {
        let grid = ShadowGrid {
            cell_size: 1.0,
            cells: HashMap::new(),
            tris: Vec::new(),
        };
        let settings = default_bake_settings();
        let receiver = vec3(0.0, 0.0, 10.0);
        let small = bake_light_value(
            &grid,
            settings,
            &test_area_sample(4.0),
            receiver,
            Vec3::NEG_Z,
        );
        let large = bake_light_value(
            &grid,
            settings,
            &test_area_sample(16.0),
            receiver,
            Vec3::NEG_Z,
        );
        let behind = bake_light_value(&grid, settings, &test_area_sample(16.0), -receiver, Vec3::Z);

        assert!(small.x > 0.0);
        assert!((large.x / small.x - 4.0).abs() < 0.0001);
        assert_eq!(behind, Vec3::ZERO);
    }

    #[test]
    fn dff_emitter_direction_uses_rendered_normal_side() {
        let matching = dff_emitter_direction(Vec3::Z, Vec3::Z);
        let reversed_winding = dff_emitter_direction(Vec3::NEG_Z, Vec3::Z);

        assert!(matching.dot(Vec3::Z) > 0.9999);
        assert!(reversed_winding.dot(Vec3::Z) > 0.9999);
    }

    #[test]
    fn area_emitter_shadow_ray_is_not_blurred_like_a_point_light() {
        let blocker = ShadowTri {
            a: vec3(-10.0, -10.0, 5.0),
            b: vec3(10.0, -10.0, 5.0),
            c: vec3(0.0, 10.0, 5.0),
            min: vec3(-10.0, -10.0, 5.0),
            max: vec3(10.0, 10.0, 5.0),
        };
        let mut cells = HashMap::new();
        cells.insert((0, 0), vec![0]);
        let grid = ShadowGrid {
            cell_size: 100.0,
            cells,
            tris: vec![blocker],
        };
        let mut settings = default_bake_settings();
        settings.shadow_softness = 10_000.0;
        let mut light = test_area_sample(4.0);
        light.position.z = 10.0;
        light.direction.z = -1.0;
        light.casts_shadow = true;

        let value = bake_light_value(&grid, settings, &light, Vec3::ZERO, Vec3::Z);
        assert_eq!(value, Vec3::ZERO);
    }

    #[test]
    fn vertical_directional_shadow_camera_is_finite() {
        let bounds = Bounds {
            min: vec3(-100.0, -200.0, -10.0),
            max: vec3(100.0, 200.0, 300.0),
        };
        let light = EditorLight {
            name: "Vertical sun".to_string(),
            attached_to: None,
            kind: LightKind::Directional,
            profile: LightProfile::Day,
            position: V3::default(),
            direction: V3 {
                x: 0.0,
                y: 0.0,
                z: -1.0,
            },
            color: neutral_vertex_color(),
            temperature: 6500.0,
            use_temperature: false,
            intensity: 1.0,
            radius: 0.0,
            casts_shadow: true,
            point_lobe: PointLightLobe::Omni,
        };
        let (projection, view) = gpu_shadow_matrices_for_receiver(bounds, bounds, &light).unwrap();
        assert!(
            projection
                .to_cols_array()
                .iter()
                .all(|value| value.is_finite())
        );
        assert!(view.to_cols_array().iter().all(|value| value.is_finite()));
        assert!(directional_shadow_depth_range(bounds, Vec3::NEG_Z) > 300.0);
        let high_sun = directional_shadow_caster_distance(bounds, Vec3::Z);
        let low_sun = directional_shadow_caster_distance(bounds, vec3(1.0, 0.0, 0.1));
        assert!(high_sun < low_sun);
    }

    #[test]
    fn directional_shadow_camera_depth_points_toward_the_light_source() {
        let bounds = Bounds {
            min: vec3(-2_000.0, -2_000.0, -500.0),
            max: vec3(2_000.0, 2_000.0, 1_500.0),
        };
        let sun = default_lights()
            .into_iter()
            .find(|light| light.name == "Sun")
            .unwrap();
        let (projection, view) = gpu_shadow_matrices_for_receiver(bounds, bounds, &sun).unwrap();
        let matrix = projection * view;
        let center = (bounds.min + bounds.max) * 0.5;
        let travel = vec3(sun.direction.x, sun.direction.y, sun.direction.z).normalize();
        let receiver_depth = matrix.project_point3(center).z;
        let toward_source_depth = matrix.project_point3(center - travel * 10.0).z;

        assert!(toward_source_depth < receiver_depth);
    }

    #[test]
    fn vertical_spotlight_shadow_camera_is_finite() {
        let bounds = Bounds {
            min: vec3(-50.0, -50.0, 0.0),
            max: vec3(50.0, 50.0, 100.0),
        };
        let light = EditorLight {
            name: "Downlight".to_string(),
            attached_to: None,
            kind: LightKind::Spot,
            profile: LightProfile::Both,
            position: V3 {
                x: 0.0,
                y: 0.0,
                z: 200.0,
            },
            direction: V3 {
                x: 0.0,
                y: 0.0,
                z: -1.0,
            },
            color: neutral_vertex_color(),
            temperature: 6500.0,
            use_temperature: false,
            intensity: 1.0,
            radius: 500.0,
            casts_shadow: true,
            point_lobe: PointLightLobe::Omni,
        };
        let (projection, view) = gpu_shadow_matrices_for_receiver(bounds, bounds, &light).unwrap();
        assert!(
            projection
                .to_cols_array()
                .iter()
                .all(|value| value.is_finite())
        );
        assert!(view.to_cols_array().iter().all(|value| value.is_finite()));
    }

    #[test]
    fn directional_vector_is_the_direction_light_travels() {
        let light = EditorLight {
            name: "Sun".to_string(),
            attached_to: None,
            kind: LightKind::Directional,
            profile: LightProfile::Day,
            position: V3::default(),
            direction: V3 {
                x: 0.0,
                y: 0.0,
                z: -1.0,
            },
            color: neutral_vertex_color(),
            temperature: 6500.0,
            use_temperature: false,
            intensity: 1.0,
            radius: 0.0,
            casts_shadow: false,
            point_lobe: PointLightLobe::Omni,
        };
        let grid = ShadowGrid {
            cell_size: 1.0,
            cells: HashMap::new(),
            tris: Vec::new(),
        };
        let settings = default_bake_settings();
        let upward = bake_light_value(&grid, settings, &light, Vec3::ZERO, Vec3::Z);
        let downward = bake_light_value(&grid, settings, &light, Vec3::ZERO, Vec3::NEG_Z);
        assert!(upward.x > 0.99);
        assert_eq!(downward, Vec3::ZERO);
        let (ray, _) = light_ray_for_sample(&light, Vec3::ZERO, 0, settings).unwrap();
        assert!(ray.z > 0.99);
    }

    #[test]
    fn shipped_default_sun_lights_upward_world_geometry() {
        let sun = default_lights()
            .into_iter()
            .find(|light| light.name == "Sun")
            .unwrap();
        let grid = ShadowGrid {
            cell_size: 1.0,
            cells: HashMap::new(),
            tris: Vec::new(),
        };
        let baked = bake_light_value(&grid, default_bake_settings(), &sun, Vec3::ZERO, Vec3::Z);
        assert!(baked.x > 0.7, "default sun produced {baked:?}");
    }

    #[test]
    fn shipped_default_moon_is_occluded_by_scene_geometry() {
        let moon = default_lights()
            .into_iter()
            .find(|light| light.name == "Moon")
            .unwrap();
        assert!(moon.casts_shadow);

        let blocker = ShadowTri {
            a: vec3(-20.0, -20.0, 5.0),
            b: vec3(20.0, -20.0, 5.0),
            c: vec3(0.0, 20.0, 5.0),
            min: vec3(-20.0, -20.0, 5.0),
            max: vec3(20.0, 20.0, 5.0),
        };
        let mut cells = HashMap::new();
        cells.insert((0, 0), vec![0]);
        let grid = ShadowGrid {
            cell_size: 100.0,
            cells,
            tris: vec![blocker],
        };

        let baked = bake_light_value(&grid, default_bake_settings(), &moon, Vec3::ZERO, Vec3::Z);
        assert_eq!(baked, Vec3::ZERO);
    }

    #[test]
    fn both_mode_keeps_day_and_night_as_separate_outputs() {
        assert_eq!(
            bake_output_modes(BakeLightMode::Both),
            (BakeLightMode::Day, Some(BakeLightMode::Night))
        );
        assert_eq!(
            bake_output_modes(BakeLightMode::Day),
            (BakeLightMode::Day, None)
        );
    }

    #[test]
    fn spotlight_only_lights_inside_its_forward_cone() {
        let light = EditorLight {
            name: "Spot".to_string(),
            attached_to: None,
            kind: LightKind::Spot,
            profile: LightProfile::Both,
            position: V3::default(),
            direction: V3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            color: neutral_vertex_color(),
            temperature: 6500.0,
            use_temperature: false,
            intensity: 1.0,
            radius: 100.0,
            casts_shadow: false,
            point_lobe: PointLightLobe::Omni,
        };
        let grid = ShadowGrid {
            cell_size: 1.0,
            cells: HashMap::new(),
            tris: Vec::new(),
        };
        let settings = default_bake_settings();
        let forward = bake_light_value(&grid, settings, &light, vec3(25.0, 0.0, 0.0), Vec3::NEG_X);
        let behind = bake_light_value(&grid, settings, &light, vec3(-25.0, 0.0, 0.0), Vec3::X);
        let sideways = bake_light_value(&grid, settings, &light, vec3(0.0, 25.0, 0.0), Vec3::NEG_Y);
        assert!(forward.x > 0.8);
        assert_eq!(behind, Vec3::ZERO);
        assert_eq!(sideways, Vec3::ZERO);
    }
}
