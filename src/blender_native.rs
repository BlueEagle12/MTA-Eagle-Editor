use super::*;
use std::io::{Cursor, Read};

const NATIVE_MESH_MAGIC: &[u8; 8] = b"EAGMSH02";

#[derive(Clone, Debug)]
struct NativeVertex {
    position: V3,
    normal: V3,
    uvs: Vec<V2>,
    day: [u8; 4],
    night: [u8; 4],
}

#[derive(Clone, Debug)]
struct NativeTriangle {
    vertices: [u32; 3],
    material: u16,
}

#[derive(Clone, Debug)]
struct NativeObject {
    zone: String,
    name: String,
    txd: String,
    tag: String,
    lod_parent: String,
    lod_distance: f32,
    dimension: i32,
    interior: i32,
    origin: V3,
    rotation: V3,
    time_in: i32,
    time_out: i32,
    definition_flags: u64,
    has_day: bool,
    has_night: bool,
    textures: Vec<String>,
    materials: Vec<RawMaterial>,
    vertices: Vec<NativeVertex>,
    triangles: Vec<NativeTriangle>,
}

#[derive(Default)]
pub(crate) struct NativeBlenderBuild {
    pub(crate) zones: Vec<String>,
    pub(crate) dff_entries: Vec<(String, Vec<u8>)>,
    pub(crate) warnings: Vec<String>,
}

fn read_exact<const N: usize>(reader: &mut Cursor<Vec<u8>>) -> Result<[u8; N], String> {
    let mut bytes = [0u8; N];
    reader
        .read_exact(&mut bytes)
        .map_err(|error| format!("Native Blender mesh stream is truncated: {error}"))?;
    Ok(bytes)
}

fn read_u8(reader: &mut Cursor<Vec<u8>>) -> Result<u8, String> {
    Ok(read_exact::<1>(reader)?[0])
}

fn read_u16(reader: &mut Cursor<Vec<u8>>) -> Result<u16, String> {
    Ok(u16::from_le_bytes(read_exact(reader)?))
}

fn read_u32(reader: &mut Cursor<Vec<u8>>) -> Result<u32, String> {
    Ok(u32::from_le_bytes(read_exact(reader)?))
}

fn read_u64(reader: &mut Cursor<Vec<u8>>) -> Result<u64, String> {
    Ok(u64::from_le_bytes(read_exact(reader)?))
}

fn read_i32(reader: &mut Cursor<Vec<u8>>) -> Result<i32, String> {
    Ok(i32::from_le_bytes(read_exact(reader)?))
}

fn read_f32(reader: &mut Cursor<Vec<u8>>) -> Result<f32, String> {
    Ok(f32::from_le_bytes(read_exact(reader)?))
}

fn read_string(reader: &mut Cursor<Vec<u8>>) -> Result<String, String> {
    let length = read_u16(reader)? as usize;
    let mut bytes = vec![0; length];
    reader
        .read_exact(&mut bytes)
        .map_err(|error| format!("Native Blender string is truncated: {error}"))?;
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

fn read_v3(reader: &mut Cursor<Vec<u8>>) -> Result<V3, String> {
    Ok(V3 {
        x: read_f32(reader)?,
        y: read_f32(reader)?,
        z: read_f32(reader)?,
    })
}

fn read_native_objects(path: &Path) -> Result<Vec<NativeObject>, String> {
    let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut reader = Cursor::new(bytes);
    if read_exact::<8>(&mut reader)? != *NATIVE_MESH_MAGIC {
        return Err(format!(
            "{} is not an Eagle native Blender mesh stream",
            path.display()
        ));
    }
    let count = read_u32(&mut reader)? as usize;
    if count > 2_000_000 {
        return Err(format!(
            "Native Blender stream declares an unsafe object count: {count}"
        ));
    }
    let mut objects = Vec::with_capacity(count);
    for _ in 0..count {
        let zone = read_string(&mut reader)?;
        let name = read_string(&mut reader)?;
        let txd = read_string(&mut reader)?;
        let tag = read_string(&mut reader)?;
        let lod_parent = read_string(&mut reader)?;
        let lod_distance = read_f32(&mut reader)?;
        let dimension = read_i32(&mut reader)?;
        let interior = read_i32(&mut reader)?;
        let origin = read_v3(&mut reader)?;
        let rotation = read_v3(&mut reader)?;
        let time_in = read_i32(&mut reader)?;
        let time_out = read_i32(&mut reader)?;
        let definition_flags = read_u64(&mut reader)?;
        let uv_count = read_u8(&mut reader)? as usize;
        let color_flags = read_u8(&mut reader)?;
        let material_count = read_u16(&mut reader)? as usize;
        let vertex_count = read_u32(&mut reader)? as usize;
        let triangle_count = read_u32(&mut reader)? as usize;
        if uv_count > 8 || material_count > u16::MAX as usize {
            return Err(format!("{name}: invalid UV/material stream count"));
        }
        if vertex_count > 100_000_000 || triangle_count > 100_000_000 {
            return Err(format!("{name}: unsafe geometry count"));
        }
        let mut textures = Vec::with_capacity(material_count);
        let mut materials = Vec::with_capacity(material_count);
        for _ in 0..material_count {
            let texture = read_string(&mut reader)?;
            textures.push(if texture.trim().is_empty() {
                String::new()
            } else {
                sanitize_texture_name(&texture)
            });
            let red = read_f32(&mut reader)?;
            let green = read_f32(&mut reader)?;
            let blue = read_f32(&mut reader)?;
            let alpha = read_f32(&mut reader)?;
            materials.push(RawMaterial {
                color: V3 {
                    x: red,
                    y: green,
                    z: blue,
                },
                alpha,
                ambient: 1.0,
                specular: 1.0,
                diffuse: 1.0,
            });
        }
        let mut vertices = Vec::with_capacity(vertex_count);
        for _ in 0..vertex_count {
            let position = read_v3(&mut reader)?;
            let normal = read_v3(&mut reader)?;
            let mut uvs = Vec::with_capacity(uv_count);
            for _ in 0..uv_count {
                uvs.push(V2 {
                    u: read_f32(&mut reader)?,
                    v: read_f32(&mut reader)?,
                });
            }
            let day = read_exact::<4>(&mut reader)?;
            let night = read_exact::<4>(&mut reader)?;
            vertices.push(NativeVertex {
                position,
                normal,
                uvs,
                day,
                night,
            });
        }
        let mut triangles = Vec::with_capacity(triangle_count);
        for _ in 0..triangle_count {
            let vertices = [
                read_u32(&mut reader)?,
                read_u32(&mut reader)?,
                read_u32(&mut reader)?,
            ];
            let material = read_u16(&mut reader)?;
            if vertices.iter().any(|index| *index as usize >= vertex_count) {
                return Err(format!("{name}: triangle references a missing vertex"));
            }
            triangles.push(NativeTriangle { vertices, material });
        }
        objects.push(NativeObject {
            zone,
            name,
            txd,
            tag,
            lod_parent,
            lod_distance,
            dimension,
            interior,
            origin,
            rotation,
            time_in,
            time_out,
            definition_flags,
            has_day: color_flags & 1 != 0,
            has_night: color_flags & 2 != 0,
            textures,
            materials,
            vertices,
            triangles,
        });
    }
    Ok(objects)
}

fn axis_value(vertex: &NativeVertex, axis: usize) -> f32 {
    match axis {
        0 => vertex.position.x,
        1 => vertex.position.y,
        _ => vertex.position.z,
    }
}

fn interpolate(a: &NativeVertex, b: &NativeVertex, amount: f32) -> NativeVertex {
    let mix = |left: f32, right: f32| left + (right - left) * amount;
    let mix_byte =
        |left: u8, right: u8| mix(left as f32, right as f32).round().clamp(0.0, 255.0) as u8;
    let mut normal = V3 {
        x: mix(a.normal.x, b.normal.x),
        y: mix(a.normal.y, b.normal.y),
        z: mix(a.normal.z, b.normal.z),
    };
    let length = (normal.x * normal.x + normal.y * normal.y + normal.z * normal.z).sqrt();
    if length > 0.000001 {
        normal.x /= length;
        normal.y /= length;
        normal.z /= length;
    }
    NativeVertex {
        position: V3 {
            x: mix(a.position.x, b.position.x),
            y: mix(a.position.y, b.position.y),
            z: mix(a.position.z, b.position.z),
        },
        normal,
        uvs: a
            .uvs
            .iter()
            .zip(&b.uvs)
            .map(|(a, b)| V2 {
                u: mix(a.u, b.u),
                v: mix(a.v, b.v),
            })
            .collect(),
        day: std::array::from_fn(|index| mix_byte(a.day[index], b.day[index])),
        night: std::array::from_fn(|index| mix_byte(a.night[index], b.night[index])),
    }
}

fn clip_plane(
    polygon: Vec<NativeVertex>,
    axis: usize,
    plane: f32,
    keep_above: bool,
) -> Vec<NativeVertex> {
    if polygon.is_empty() {
        return polygon;
    }
    let inside = |vertex: &NativeVertex| {
        let value = axis_value(vertex, axis);
        if keep_above {
            value >= plane - 0.0001
        } else {
            value <= plane + 0.0001
        }
    };
    let mut output = Vec::new();
    let mut previous = polygon.last().expect("non-empty polygon");
    let mut previous_inside = inside(previous);
    for current in &polygon {
        let current_inside = inside(current);
        if current_inside != previous_inside {
            let previous_value = axis_value(previous, axis);
            let denominator = axis_value(current, axis) - previous_value;
            if denominator.abs() > f32::EPSILON {
                output.push(interpolate(
                    previous,
                    current,
                    (plane - previous_value) / denominator,
                ));
            }
        }
        if current_inside {
            output.push(current.clone());
        }
        previous = current;
        previous_inside = current_inside;
    }
    output
}

fn clip_triangle_to_cell(
    vertices: [&NativeVertex; 3],
    cell: [i32; 3],
    sizes: [f32; 3],
) -> Vec<NativeVertex> {
    let mut polygon = vertices.into_iter().cloned().collect::<Vec<_>>();
    for (axis, coordinate) in cell.into_iter().enumerate() {
        let size = sizes[axis];
        let minimum = coordinate as f32 * size;
        polygon = clip_plane(polygon, axis, minimum, true);
        polygon = clip_plane(polygon, axis, minimum + size, false);
    }
    polygon
}

fn object_bounds(object: &NativeObject) -> Option<(V3, V3)> {
    let first = object.vertices.first()?.position;
    let mut minimum = first;
    let mut maximum = first;
    for vertex in &object.vertices[1..] {
        minimum.x = minimum.x.min(vertex.position.x);
        minimum.y = minimum.y.min(vertex.position.y);
        minimum.z = minimum.z.min(vertex.position.z);
        maximum.x = maximum.x.max(vertex.position.x);
        maximum.y = maximum.y.max(vertex.position.y);
        maximum.z = maximum.z.max(vertex.position.z);
    }
    Some((minimum, maximum))
}

fn triangle_has_area(a: &NativeVertex, b: &NativeVertex, c: &NativeVertex) -> bool {
    let ab = V3 {
        x: b.position.x - a.position.x,
        y: b.position.y - a.position.y,
        z: b.position.z - a.position.z,
    };
    let ac = V3 {
        x: c.position.x - a.position.x,
        y: c.position.y - a.position.y,
        z: c.position.z - a.position.z,
    };
    let cross = V3 {
        x: ab.y * ac.z - ab.z * ac.y,
        y: ab.z * ac.x - ab.x * ac.z,
        z: ab.x * ac.y - ab.y * ac.x,
    };
    cross.x * cross.x + cross.y * cross.y + cross.z * cross.z > 1.0e-12
}

fn renderware_winding_matches_normals(
    a: &NativeVertex,
    b: &NativeVertex,
    c: &NativeVertex,
) -> bool {
    // RawMesh uses clockwise front faces. Its visible-side normal is therefore
    // (C-A)x(B-A), the reverse of Blender's counter-clockwise convention.
    let ca = V3 {
        x: c.position.x - a.position.x,
        y: c.position.y - a.position.y,
        z: c.position.z - a.position.z,
    };
    let ba = V3 {
        x: b.position.x - a.position.x,
        y: b.position.y - a.position.y,
        z: b.position.z - a.position.z,
    };
    let face = V3 {
        x: ca.y * ba.z - ca.z * ba.y,
        y: ca.z * ba.x - ca.x * ba.z,
        z: ca.x * ba.y - ca.y * ba.x,
    };
    let normal = V3 {
        x: a.normal.x + b.normal.x + c.normal.x,
        y: a.normal.y + b.normal.y + c.normal.y,
        z: a.normal.z + b.normal.z + c.normal.z,
    };
    face.x * normal.x + face.y * normal.y + face.z * normal.z >= 0.0
}

fn blender_uv_to_renderware(uv: V2) -> V2 {
    // Blender's V axis starts at the bottom of an image; RenderWare DFF UVs
    // use the opposite image-space direction. This matches DragonFF export.
    V2 {
        u: uv.u,
        v: 1.0 - uv.v,
    }
}

fn split_object(
    object: &NativeObject,
    size: f32,
    enabled: bool,
) -> Vec<Vec<(NativeVertex, NativeVertex, NativeVertex, u16)>> {
    split_object_sizes(object, [size; 3], enabled)
}

fn split_object_sizes(
    object: &NativeObject,
    sizes: [f32; 3],
    enabled: bool,
) -> Vec<Vec<(NativeVertex, NativeVertex, NativeVertex, u16)>> {
    let Some((minimum, maximum)) = object_bounds(object) else {
        return Vec::new();
    };
    let extents = [
        maximum.x - minimum.x,
        maximum.y - minimum.y,
        maximum.z - minimum.z,
    ];
    if !enabled
        || extents
            .iter()
            .zip(sizes)
            .all(|(extent, size)| *extent <= size)
    {
        return vec![
            object
                .triangles
                .iter()
                .map(|triangle| {
                    (
                        object.vertices[triangle.vertices[0] as usize].clone(),
                        object.vertices[triangle.vertices[1] as usize].clone(),
                        object.vertices[triangle.vertices[2] as usize].clone(),
                        triangle.material,
                    )
                })
                .collect(),
        ];
    }
    let mut chunks =
        BTreeMap::<[i32; 3], Vec<(NativeVertex, NativeVertex, NativeVertex, u16)>>::new();
    for triangle in &object.triangles {
        let vertices = [
            &object.vertices[triangle.vertices[0] as usize],
            &object.vertices[triangle.vertices[1] as usize],
            &object.vertices[triangle.vertices[2] as usize],
        ];
        let tri_min = [0, 1, 2].map(|axis| {
            vertices
                .iter()
                .map(|vertex| axis_value(vertex, axis))
                .fold(f32::INFINITY, f32::min)
        });
        let tri_max = [0, 1, 2].map(|axis| {
            vertices
                .iter()
                .map(|vertex| axis_value(vertex, axis))
                .fold(f32::NEG_INFINITY, f32::max)
        });
        let cell_min: [i32; 3] =
            std::array::from_fn(|axis| (tri_min[axis] / sizes[axis]).floor() as i32);
        let cell_max: [i32; 3] =
            std::array::from_fn(|axis| (tri_max[axis] / sizes[axis]).floor() as i32);
        for x in cell_min[0]..=cell_max[0] {
            for y in cell_min[1]..=cell_max[1] {
                for z in cell_min[2]..=cell_max[2] {
                    let polygon = clip_triangle_to_cell(vertices, [x, y, z], sizes);
                    if polygon.len() < 3 {
                        continue;
                    }
                    for index in 1..polygon.len() - 1 {
                        if !triangle_has_area(&polygon[0], &polygon[index], &polygon[index + 1]) {
                            continue;
                        }
                        chunks.entry([x, y, z]).or_default().push((
                            polygon[0].clone(),
                            polygon[index].clone(),
                            polygon[index + 1].clone(),
                            triangle.material,
                        ));
                    }
                }
            }
        }
    }
    chunks
        .into_values()
        .filter(|triangles| !triangles.is_empty())
        .collect()
}

type NativeTriangleVertices = (NativeVertex, NativeVertex, NativeVertex, u16);

fn split_chunk_by_material_limit(
    triangles: Vec<NativeTriangleVertices>,
) -> Vec<Vec<NativeTriangleVertices>> {
    let materials = triangles
        .iter()
        .map(|triangle| triangle.3)
        .collect::<BTreeSet<_>>();
    if materials.len() <= GTA_DFF_MATERIAL_LIMIT {
        return vec![triangles];
    }

    // Assign materials in index order so the result is stable across imports.
    // Every face using a material stays in the same output element.
    let material_groups = materials
        .into_iter()
        .enumerate()
        .map(|(index, material)| (material, index / GTA_DFF_MATERIAL_LIMIT))
        .collect::<BTreeMap<_, _>>();
    let group_count = material_groups.values().copied().max().unwrap_or(0) + 1;
    let mut groups = vec![Vec::new(); group_count];
    for triangle in triangles {
        let group = material_groups[&triangle.3];
        groups[group].push(triangle);
    }
    groups
}

fn bytes_to_color(value: [u8; 4]) -> (V3, f32) {
    (
        V3 {
            x: value[0] as f32 / 255.0,
            y: value[1] as f32 / 255.0,
            z: value[2] as f32 / 255.0,
        },
        value[3] as f32 / 255.0,
    )
}

fn chunk_to_raw(
    object: &NativeObject,
    triangles: &[(NativeVertex, NativeVertex, NativeVertex, u16)],
    center: V3,
) -> RawMesh {
    let uv_count = triangles
        .first()
        .map(|triangle| triangle.0.uvs.len())
        .unwrap_or(0);
    // A spatial chunk usually references only a subset of the source object's
    // materials. Keep just that subset and make its triangle indices dense.
    let used_materials = triangles
        .iter()
        .map(|triangle| triangle.3)
        .collect::<BTreeSet<_>>();
    let material_remap = used_materials
        .iter()
        .enumerate()
        .map(|(new_index, old_index)| (*old_index, new_index as u16))
        .collect::<BTreeMap<_, _>>();
    let material_textures = used_materials
        .iter()
        .map(|index| {
            object
                .textures
                .get(*index as usize)
                .cloned()
                .unwrap_or_default()
        })
        .collect::<Vec<_>>();
    let materials = used_materials
        .iter()
        .map(|index| {
            object
                .materials
                .get(*index as usize)
                .copied()
                .unwrap_or_default()
        })
        .collect::<Vec<_>>();
    let mut raw = RawMesh {
        secondary_uvs: vec![Vec::new(); uv_count.saturating_sub(1)],
        material_animations: vec![DffMaterialAnim::default(); material_textures.len()],
        material_textures,
        materials,
        ..RawMesh::default()
    };
    for (a, b, c, material) in triangles {
        let (a, b, c) = if renderware_winding_matches_normals(a, b, c) {
            (a, b, c)
        } else {
            (a, c, b)
        };
        let start = raw.vertices.len() as u32;
        for vertex in [a, b, c] {
            raw.vertices.push(V3 {
                x: vertex.position.x - center.x,
                y: vertex.position.y - center.y,
                z: vertex.position.z - center.z,
            });
            raw.normals.push(vertex.normal);
            if let Some(primary) = vertex.uvs.first() {
                raw.uvs.push(blender_uv_to_renderware(*primary));
            }
            for (index, uv) in vertex.uvs.iter().skip(1).enumerate() {
                raw.secondary_uvs[index].push(blender_uv_to_renderware(*uv));
            }
            if object.has_day {
                let (color, alpha) = bytes_to_color(vertex.day);
                raw.prelit_colors.push(color);
                raw.prelit_alphas.push(alpha);
            }
            if object.has_night {
                let (color, alpha) = bytes_to_color(vertex.night);
                raw.night_prelit_colors.push(color);
                raw.night_prelit_alphas.push(alpha);
            }
        }
        raw.triangles.push(Tri {
            a: start,
            b: start + 1,
            c: start + 2,
            material: material_remap[material],
        });
    }
    raw
}

fn triangle_bounds(
    triangles: &[(NativeVertex, NativeVertex, NativeVertex, u16)],
) -> Option<(V3, V3)> {
    let first = triangles.first()?.0.position;
    let mut minimum = first;
    let mut maximum = first;
    for (a, b, c, _) in triangles {
        for vertex in [a, b, c] {
            minimum.x = minimum.x.min(vertex.position.x);
            minimum.y = minimum.y.min(vertex.position.y);
            minimum.z = minimum.z.min(vertex.position.z);
            maximum.x = maximum.x.max(vertex.position.x);
            maximum.y = maximum.y.max(vertex.position.y);
            maximum.z = maximum.z.max(vertex.position.z);
        }
    }
    Some((minimum, maximum))
}

#[derive(Clone)]
pub(crate) struct RawSpatialChunk {
    pub(crate) raw: RawMesh,
    /// Offset from the source model origin to this chunk's centered origin.
    pub(crate) center: V3,
}

fn raw_color_bytes(color: V3, alpha: f32) -> [u8; 4] {
    let byte = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    [byte(color.x), byte(color.y), byte(color.z), byte(alpha)]
}

/// Spatially clips a simple normalized DFF mesh using the same boundary-safe
/// slicer as Blender import. Complex sidecar data is rejected so validation
/// never silently drops authored plug-ins while turning one model into many.
pub(crate) fn split_raw_mesh_spatial(
    source: &RawMesh,
    sizes: V3,
) -> Result<Vec<RawSpatialChunk>, String> {
    let dimensions = [sizes.x, sizes.y, sizes.z];
    if dimensions
        .iter()
        .any(|value| !value.is_finite() || *value < 1.0)
    {
        return Err("chunk dimensions must be finite and at least 1 unit".to_string());
    }
    if source.vertices.is_empty() || source.triangles.is_empty() {
        return Err("DFF has no readable geometry".to_string());
    }
    if source.components.len() > 1 {
        return Err("multi-geometry DFFs require manual review".to_string());
    }
    if source
        .components
        .iter()
        .any(|component| component.breakable.is_some())
    {
        return Err("breakable geometry cannot be spatially chunked safely".to_string());
    }
    if !source.effects_2dfx.is_empty()
        || !source.uv_anim_dictionaries.is_empty()
        || !source.uv_animations.is_empty()
        || source
            .material_animations
            .iter()
            .any(|animation| !animation.names.is_empty())
    {
        return Err("DFF contains 2DFX or UV-animation data that needs manual review".to_string());
    }
    if source.normals.len() != source.vertices.len() {
        return Err("DFF does not have a complete normalized vertex stream".to_string());
    }

    let has_primary_uv = source.uvs.len() == source.vertices.len();
    if !source.uvs.is_empty() && !has_primary_uv {
        return Err("DFF has an incomplete primary UV stream".to_string());
    }
    if source
        .secondary_uvs
        .iter()
        .any(|set| set.len() != source.vertices.len())
    {
        return Err("DFF has an incomplete secondary UV stream".to_string());
    }
    let has_day = source.prelit_colors.len() == source.vertices.len();
    let has_night = source.night_prelit_colors.len() == source.vertices.len();
    let day_alpha = source.prelit_alphas.len() == source.vertices.len();
    let night_alpha = source.night_prelit_alphas.len() == source.vertices.len();
    let mut vertices = Vec::with_capacity(source.vertices.len());
    for index in 0..source.vertices.len() {
        let mut uvs = Vec::with_capacity(usize::from(has_primary_uv) + source.secondary_uvs.len());
        if has_primary_uv {
            let uv = source.uvs[index];
            // Native Blender vertices use bottom-origin V; chunk_to_raw flips
            // this back, making this conversion lossless for DFF UVs.
            uvs.push(V2 {
                u: uv.u,
                v: 1.0 - uv.v,
            });
        }
        for set in &source.secondary_uvs {
            let uv = set[index];
            uvs.push(V2 {
                u: uv.u,
                v: 1.0 - uv.v,
            });
        }
        vertices.push(NativeVertex {
            position: source.vertices[index],
            normal: source.normals[index],
            uvs,
            day: if has_day {
                raw_color_bytes(
                    source.prelit_colors[index],
                    if day_alpha {
                        source.prelit_alphas[index]
                    } else {
                        1.0
                    },
                )
            } else {
                [255; 4]
            },
            night: if has_night {
                raw_color_bytes(
                    source.night_prelit_colors[index],
                    if night_alpha {
                        source.night_prelit_alphas[index]
                    } else {
                        1.0
                    },
                )
            } else {
                [255; 4]
            },
        });
    }
    let triangles = source
        .triangles
        .iter()
        .map(|triangle| NativeTriangle {
            vertices: [triangle.a, triangle.b, triangle.c],
            material: triangle.material,
        })
        .collect();
    let object = NativeObject {
        zone: String::new(),
        name: "validation_chunk".to_string(),
        txd: String::new(),
        tag: "object".to_string(),
        lod_parent: String::new(),
        lod_distance: 0.0,
        dimension: 0,
        interior: 0,
        origin: V3::default(),
        rotation: V3::default(),
        time_in: 0,
        time_out: 24,
        definition_flags: 0,
        has_day,
        has_night,
        textures: source.material_textures.clone(),
        materials: source.materials.clone(),
        vertices,
        triangles,
    };
    let chunks = split_object_sizes(&object, dimensions, true);
    if chunks.len() <= 1 {
        return Ok(Vec::new());
    }
    chunks
        .into_iter()
        .map(|triangles| {
            let (minimum, maximum) = triangle_bounds(&triangles)
                .ok_or_else(|| "spatial chunk has no readable bounds".to_string())?;
            let center = V3 {
                x: (minimum.x + maximum.x) * 0.5,
                y: (minimum.y + maximum.y) * 0.5,
                z: (minimum.z + maximum.z) * 0.5,
            };
            Ok(RawSpatialChunk {
                raw: chunk_to_raw(&object, &triangles, center),
                center,
            })
        })
        .collect()
}

fn base36(mut value: usize) -> String {
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    if value == 0 {
        return "0".to_string();
    }
    let mut encoded = Vec::new();
    while value > 0 {
        encoded.push(DIGITS[value % 36] as char);
        value /= 36;
    }
    encoded.iter().rev().collect()
}

fn model_stem(name: &str, source_index: usize, chunk_index: usize) -> String {
    let mut base = name
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
        .collect::<String>();
    if base.is_empty() {
        base = "object".to_string();
    }
    // The former 16-bit hash could collide when large maps contained objects
    // sharing the same nine-character prefix. Encode the source and chunk
    // indices directly so every generated model in an import is unique by
    // construction while still fitting the IMG directory field.
    let suffix = format!("_{}_{}", base36(source_index), base36(chunk_index));
    let max_stem_len = IMG_RUNTIME_SAFE_ENTRY_NAME_BYTES - ".dff".len();
    let keep = max_stem_len.saturating_sub(suffix.len());
    base.truncate(keep);
    format!("{base}{suffix}")
}

#[derive(Clone)]
struct InstanceModel {
    raw: RawMesh,
    stem: String,
    txd: String,
    include_normals: bool,
    lod_distance: f32,
    time_in: i32,
    time_out: i32,
    definition_flags: u64,
}

fn instance_signature(name: &str, object: &NativeObject, raw: &RawMesh) -> u64 {
    let mut state = DefaultHasher::new();
    name.to_ascii_lowercase().hash(&mut state);
    object.txd.to_ascii_lowercase().hash(&mut state);
    (!object.tag.eq_ignore_ascii_case("building")).hash(&mut state);
    object.lod_distance.to_bits().hash(&mut state);
    object.time_in.hash(&mut state);
    object.time_out.hash(&mut state);
    object.definition_flags.hash(&mut state);
    raw.vertices.len().hash(&mut state);
    raw.normals.len().hash(&mut state);
    raw.uvs.len().hash(&mut state);
    raw.secondary_uvs.len().hash(&mut state);
    raw.prelit_colors.len().hash(&mut state);
    raw.night_prelit_colors.len().hash(&mut state);
    raw.triangles.len().hash(&mut state);
    for set in &raw.secondary_uvs {
        set.len().hash(&mut state);
    }
    for triangle in &raw.triangles {
        triangle.a.hash(&mut state);
        triangle.b.hash(&mut state);
        triangle.c.hash(&mut state);
        triangle.material.hash(&mut state);
    }
    for texture in &raw.material_textures {
        texture.to_ascii_lowercase().hash(&mut state);
    }
    raw.materials.len().hash(&mut state);
    state.finish()
}

fn close_f32(a: f32, b: f32, tolerance: f32) -> bool {
    (a - b).abs() <= tolerance
}

fn close_v3(a: V3, b: V3, tolerance: f32) -> bool {
    close_f32(a.x, b.x, tolerance)
        && close_f32(a.y, b.y, tolerance)
        && close_f32(a.z, b.z, tolerance)
}

fn close_v2(a: V2, b: V2, tolerance: f32) -> bool {
    close_f32(a.u, b.u, tolerance) && close_f32(a.v, b.v, tolerance)
}

fn close_slice<T>(left: &[T], right: &[T], mut matches: impl FnMut(&T, &T) -> bool) -> bool {
    left.len() == right.len() && left.iter().zip(right).all(|(a, b)| matches(a, b))
}

fn same_raw_instance(left: &RawMesh, right: &RawMesh) -> bool {
    const GEOMETRY_TOLERANCE: f32 = 0.001;
    const NORMAL_TOLERANCE: f32 = 0.002;
    const ATTRIBUTE_TOLERANCE: f32 = 0.001;
    close_slice(&left.vertices, &right.vertices, |a, b| {
        close_v3(*a, *b, GEOMETRY_TOLERANCE)
    }) && close_slice(&left.normals, &right.normals, |a, b| {
        close_v3(*a, *b, NORMAL_TOLERANCE)
    }) && close_slice(&left.uvs, &right.uvs, |a, b| {
        close_v2(*a, *b, ATTRIBUTE_TOLERANCE)
    }) && close_slice(&left.secondary_uvs, &right.secondary_uvs, |a, b| {
        close_slice(a, b, |a, b| close_v2(*a, *b, ATTRIBUTE_TOLERANCE))
    }) && close_slice(&left.prelit_colors, &right.prelit_colors, |a, b| {
        close_v3(*a, *b, ATTRIBUTE_TOLERANCE)
    }) && close_slice(&left.prelit_alphas, &right.prelit_alphas, |a, b| {
        close_f32(*a, *b, ATTRIBUTE_TOLERANCE)
    }) && close_slice(
        &left.night_prelit_colors,
        &right.night_prelit_colors,
        |a, b| close_v3(*a, *b, ATTRIBUTE_TOLERANCE),
    ) && close_slice(
        &left.night_prelit_alphas,
        &right.night_prelit_alphas,
        |a, b| close_f32(*a, *b, ATTRIBUTE_TOLERANCE),
    ) && left.triangles == right.triangles
        && close_slice(&left.material_textures, &right.material_textures, |a, b| {
            a.eq_ignore_ascii_case(b)
        })
        && close_slice(&left.materials, &right.materials, |a, b| {
            close_v3(a.color, b.color, ATTRIBUTE_TOLERANCE)
                && close_f32(a.alpha, b.alpha, ATTRIBUTE_TOLERANCE)
                && close_f32(a.ambient, b.ambient, ATTRIBUTE_TOLERANCE)
                && close_f32(a.specular, b.specular, ATTRIBUTE_TOLERANCE)
                && close_f32(a.diffuse, b.diffuse, ATTRIBUTE_TOLERANCE)
        })
}

fn same_instance_model(model: &InstanceModel, object: &NativeObject, raw: &RawMesh) -> bool {
    same_raw_instance(&model.raw, raw)
        && model.txd.eq_ignore_ascii_case(&object.txd)
        && model.include_normals == !object.tag.eq_ignore_ascii_case("building")
        && model.lod_distance.to_bits() == object.lod_distance.to_bits()
        && model.time_in == object.time_in
        && model.time_out == object.time_out
        && model.definition_flags == object.definition_flags
}

pub(crate) fn rotate_placement(value: V3, rotation: V3) -> V3 {
    let (sx, cx) = rotation.x.to_radians().sin_cos();
    let (sy, cy) = rotation.y.to_radians().sin_cos();
    let (sz, cz) = rotation.z.to_radians().sin_cos();
    let after_x = V3 {
        x: value.x,
        y: cx * value.y - sx * value.z,
        z: sx * value.y + cx * value.z,
    };
    let after_y = V3 {
        x: cy * after_x.x + sy * after_x.z,
        y: after_x.y,
        z: -sy * after_x.x + cy * after_x.z,
    };
    V3 {
        x: cz * after_y.x - sz * after_y.y,
        y: sz * after_y.x + cz * after_y.y,
        z: after_y.z,
    }
}

pub(crate) fn build_native_blender_assets(
    geometry_path: &Path,
    root: &Path,
    options: &BlenderImportOptions,
    tx: &mpsc::Sender<BlenderImportUpdate>,
) -> Result<NativeBlenderBuild, String> {
    let objects = read_native_objects(geometry_path)?;
    let mut build = NativeBlenderBuild::default();
    let mut zone_maps = BTreeMap::<String, String>::new();
    let mut zone_defs = BTreeMap::<String, String>::new();
    let mut instance_models = Vec::<InstanceModel>::new();
    let mut instance_candidates = HashMap::<u64, Vec<usize>>::new();
    let mut defined_models = BTreeSet::<(String, String)>::new();
    let mut reused_instances = 0usize;
    for object in &objects {
        if !build.zones.contains(&object.zone) {
            build.zones.push(object.zone.clone());
        }
        zone_maps.entry(object.zone.clone()).or_insert_with(|| "<map>\n    <info name=\"Eagle Blender Import\" author=\"Eagle Editor\" version=\"1.0\"></info>\n".to_string());
        zone_defs
            .entry(object.zone.clone())
            .or_insert_with(|| "<zoneDefinitions>\n".to_string());
    }
    let mut first_named_source = HashMap::<String, usize>::new();
    for (index, object) in objects.iter().enumerate() {
        first_named_source
            .entry(object.name.to_ascii_lowercase())
            .or_insert(index);
    }
    let lod_targets = first_named_source
        .into_iter()
        .map(|(name, index)| (name.clone(), model_stem(&objects[index].name, index, 0)))
        .collect::<HashMap<_, _>>();
    for (source_index, object) in objects.iter().enumerate() {
        let spatial_chunks =
            split_object(object, options.chunk_size.max(1.0), options.chunk_meshes);
        let spatial_chunk_count = spatial_chunks.len();
        let chunks = spatial_chunks
            .into_iter()
            .flat_map(split_chunk_by_material_limit)
            .collect::<Vec<_>>();
        if chunks.len() > spatial_chunk_count {
            build.warnings.push(format!(
                "Split {} into {} elements so every DFF uses at most {} materials",
                object.name,
                chunks.len(),
                GTA_DFF_MATERIAL_LIMIT
            ));
        }
        for (chunk_index, triangles) in chunks.iter().enumerate() {
            let Some((minimum, maximum)) = triangle_bounds(triangles) else {
                continue;
            };
            let center = if options.center_origins || chunks.len() > 1 {
                V3 {
                    x: (minimum.x + maximum.x) * 0.5,
                    y: (minimum.y + maximum.y) * 0.5,
                    z: (minimum.z + maximum.z) * 0.5,
                }
            } else {
                V3::default()
            };
            let raw = chunk_to_raw(object, triangles, center);
            let include_normals = !object.tag.eq_ignore_ascii_case("building");
            debug_assert!(raw.material_textures.len() <= GTA_DFF_MATERIAL_LIMIT);
            let signature = instance_signature(&object.name, object, &raw);
            let existing = instance_candidates.get(&signature).and_then(|candidates| {
                candidates
                    .iter()
                    .copied()
                    .find(|index| same_instance_model(&instance_models[*index], object, &raw))
            });
            let stem = if let Some(index) = existing {
                reused_instances += 1;
                instance_models[index].stem.clone()
            } else {
                let stem = model_stem(&object.name, source_index, chunk_index);
                let dff = write_normalized_dff_with_options(
                    &raw,
                    &stem,
                    DffWriteOptions {
                        include_normals,
                        include_bin_mesh: true,
                    },
                )
                .map_err(|error| format!("{}: {error}", object.name))?;
                let filename = format!("{stem}.dff");
                // Keep generated models in memory until blender_import packs
                // them into imgs/dff.img. Loose zone copies duplicate the
                // archive, bloat resources, and can become stale independently.
                build.dff_entries.push((filename, dff));
                let index = instance_models.len();
                instance_models.push(InstanceModel {
                    raw,
                    stem: stem.clone(),
                    txd: object.txd.clone(),
                    include_normals,
                    lod_distance: object.lod_distance,
                    time_in: object.time_in,
                    time_out: object.time_out,
                    definition_flags: object.definition_flags,
                });
                instance_candidates
                    .entry(signature)
                    .or_default()
                    .push(index);
                stem
            };

            if defined_models.insert((object.zone.clone(), stem.clone())) {
                let mut definition = BTreeMap::new();
                definition.insert("id".to_string(), stem.clone());
                definition.insert("dff".to_string(), stem.clone());
                definition.insert("txd".to_string(), object.txd.clone());
                definition.insert(
                    "lodDistance".to_string(),
                    format!("{:.0}", object.lod_distance),
                );
                if object.definition_flags != 0 {
                    definition.insert("flags".to_string(), object.definition_flags.to_string());
                }
                if object.time_in != 0 || object.time_out != 24 {
                    definition.insert("timeIn".to_string(), object.time_in.to_string());
                    definition.insert("timeOut".to_string(), object.time_out.to_string());
                }
                zone_defs
                    .get_mut(&object.zone)
                    .expect("zone definition")
                    .push_str(&write_tag("definition", &definition));
            }

            let rotated_center = rotate_placement(center, object.rotation);
            let position = V3 {
                x: object.origin.x + rotated_center.x,
                y: object.origin.y + rotated_center.y,
                z: object.origin.z + rotated_center.z,
            };
            let mut placement = BTreeMap::new();
            placement.insert("id".to_string(), stem);
            placement.insert("posX".to_string(), format!("{:.6}", position.x));
            placement.insert("posY".to_string(), format!("{:.6}", position.y));
            placement.insert("posZ".to_string(), format!("{:.6}", position.z));
            placement.insert("rotX".to_string(), format!("{:.6}", object.rotation.x));
            placement.insert("rotY".to_string(), format!("{:.6}", object.rotation.y));
            placement.insert("rotZ".to_string(), format!("{:.6}", object.rotation.z));
            placement.insert("dimension".to_string(), object.dimension.to_string());
            placement.insert("interior".to_string(), object.interior.to_string());
            if !object.lod_parent.trim().is_empty() {
                let resolved = if object.lod_parent.eq_ignore_ascii_case("self") {
                    "self".to_string()
                } else {
                    lod_targets
                        .get(&object.lod_parent.to_ascii_lowercase())
                        .cloned()
                        .unwrap_or_else(|| object.lod_parent.clone())
                };
                placement.insert("lodParent".to_string(), resolved);
            }
            zone_maps
                .get_mut(&object.zone)
                .expect("zone map")
                .push_str(&write_tag(&object.tag, &placement));
        }
        if source_index == 0 || (source_index + 1) % 100 == 0 || source_index + 1 == objects.len() {
            send_blender_progress(
                tx,
                0.82 + 0.10 * (source_index + 1) as f32 / objects.len().max(1) as f32,
                format!(
                    "Native slicing/DFF build {}/{}",
                    source_index + 1,
                    objects.len()
                ),
            );
        }
    }
    if reused_instances > 0 {
        build.warnings.push(format!(
            "Reused {reused_instances} evaluated Blender instance(s) across {} unique DFF model(s)",
            build.dff_entries.len()
        ));
    }
    for zone in &build.zones {
        let zone_dir = root.join("zones").join(zone);
        fs::create_dir_all(&zone_dir)
            .map_err(|error| format!("{}: {error}", zone_dir.display()))?;
        let mut map = zone_maps
            .remove(zone)
            .unwrap_or_else(|| "<map>\n".to_string());
        map.push_str("</map>\n");
        fs::write(zone_dir.join(format!("{zone}.map")), map)
            .map_err(|error| format!("{zone}.map: {error}"))?;
        let mut definitions = zone_defs
            .remove(zone)
            .unwrap_or_else(|| "<zoneDefinitions>\n".to_string());
        definitions.push_str("</zoneDefinitions>\n");
        fs::write(zone_dir.join(format!("{zone}.definition")), definitions)
            .map_err(|error| format!("{zone}.definition: {error}"))?;
    }
    let mut zone_list = build.zones.join("\n");
    if !zone_list.is_empty() {
        zone_list.push('\n');
    }
    fs::write(root.join("eagleZones.txt"), zone_list)
        .map_err(|error| format!("eagleZones.txt: {error}"))?;
    Ok(build)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn push_string(bytes: &mut Vec<u8>, value: &str) {
        bytes.extend_from_slice(&(value.len() as u16).to_le_bytes());
        bytes.extend_from_slice(value.as_bytes());
    }

    fn vertex(x: f32, y: f32, z: f32) -> NativeVertex {
        NativeVertex {
            position: V3 { x, y, z },
            normal: V3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
            uvs: vec![V2 { u: x, v: y }],
            day: [255; 4],
            night: [255; 4],
        }
    }

    fn push_fixture_object(
        bytes: &mut Vec<u8>,
        name: &str,
        origin: V3,
        rotation: V3,
        model_scale: f32,
    ) {
        push_fixture_object_with_tag(bytes, name, "building", origin, rotation, model_scale);
    }

    fn push_fixture_object_with_tag(
        bytes: &mut Vec<u8>,
        name: &str,
        tag: &str,
        origin: V3,
        rotation: V3,
        model_scale: f32,
    ) {
        for value in ["TestZone", name, "roads", tag, ""] {
            push_string(bytes, value);
        }
        bytes.extend_from_slice(&700.0f32.to_le_bytes());
        bytes.extend_from_slice(&0i32.to_le_bytes());
        bytes.extend_from_slice(&0i32.to_le_bytes());
        for value in [
            origin.x, origin.y, origin.z, rotation.x, rotation.y, rotation.z,
        ] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&0i32.to_le_bytes());
        bytes.extend_from_slice(&24i32.to_le_bytes());
        bytes.extend_from_slice(&0u64.to_le_bytes());
        bytes.push(1);
        bytes.push(0);
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&3u32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        push_string(bytes, "");
        for value in [1.0f32, 1.0, 1.0, 1.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for (x, y) in [(0.0f32, 0.0f32), (model_scale, 0.0), (0.0, model_scale)] {
            for value in [x, y, 0.0, 0.0, 0.0, 1.0, x, y] {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            bytes.extend_from_slice(&[255; 8]);
        }
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&2u32.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
    }

    #[test]
    fn clipping_interpolates_triangle_across_chunk_boundary() {
        let polygon = clip_triangle_to_cell(
            [
                &vertex(-1.0, 0.0, 0.0),
                &vertex(1.0, 0.0, 0.0),
                &vertex(1.0, 1.0, 0.0),
            ],
            [0, 0, 0],
            [1.0; 3],
        );
        assert!(polygon.len() >= 3);
        assert!(
            polygon
                .iter()
                .all(|value| value.position.x >= -0.0001 && value.position.x <= 1.0001)
        );
    }

    #[test]
    fn raw_mesh_spatial_split_clips_every_chunk_to_requested_size() {
        let mut raw = RawMesh {
            material_textures: vec!["road".to_string()],
            materials: vec![RawMaterial::default()],
            ..RawMesh::default()
        };
        for position in [
            V3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            V3 {
                x: 512.0,
                y: 0.0,
                z: 0.0,
            },
            V3 {
                x: 512.0,
                y: 512.0,
                z: 0.0,
            },
            V3 {
                x: 0.0,
                y: 512.0,
                z: 0.0,
            },
        ] {
            raw.vertices.push(position);
            raw.normals.push(V3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            });
            raw.uvs.push(V2 {
                u: position.x / 512.0,
                v: position.y / 512.0,
            });
        }
        raw.triangles = vec![
            Tri {
                a: 0,
                b: 1,
                c: 2,
                material: 0,
            },
            Tri {
                a: 0,
                b: 2,
                c: 3,
                material: 0,
            },
        ];

        let chunks = split_raw_mesh_spatial(
            &raw,
            V3 {
                x: 256.0,
                y: 256.0,
                z: 256.0,
            },
        )
        .expect("simple raw mesh should split");

        assert_eq!(chunks.len(), 4);
        for chunk in chunks {
            let bounds = bounds_from_vertices(&chunk.raw.vertices);
            assert!(bounds.max.x - bounds.min.x <= 256.001);
            assert!(bounds.max.y - bounds.min.y <= 256.001);
            assert!(bounds.max.z - bounds.min.z <= 256.001);
            assert!(!chunk.raw.triangles.is_empty());
        }
    }

    #[test]
    fn raw_mesh_spatial_split_leaves_small_mesh_unchanged() {
        let raw = RawMesh {
            vertices: vec![
                V3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                V3 {
                    x: 10.0,
                    y: 0.0,
                    z: 0.0,
                },
                V3 {
                    x: 0.0,
                    y: 10.0,
                    z: 0.0,
                },
            ],
            normals: vec![
                V3 {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0
                };
                3
            ],
            triangles: vec![Tri {
                a: 0,
                b: 1,
                c: 2,
                material: 0,
            }],
            ..RawMesh::default()
        };

        assert!(
            split_raw_mesh_spatial(
                &raw,
                V3 {
                    x: 256.0,
                    y: 256.0,
                    z: 256.0
                },
            )
            .expect("small mesh should be valid")
            .is_empty()
        );
    }

    #[test]
    fn generated_model_stems_fit_img_directory_field() {
        let stem = model_stem("An extremely long Blender object name", 42, 999);
        assert!(format!("{stem}.dff").len() <= IMG_RUNTIME_SAFE_ENTRY_NAME_BYTES);
    }

    #[test]
    fn generated_model_stems_are_unique_for_large_same_prefix_maps() {
        let mut stems = HashSet::new();
        for source_index in 0..10_000 {
            for chunk_index in [0, 1, 999, 100_000] {
                let stem = model_stem(
                    "4smrk01c2b0_objects_with_the_same_long_prefix",
                    source_index,
                    chunk_index,
                );
                assert!(stems.insert(lower(&stem)), "duplicate stem: {stem}");
                assert!(format!("{stem}.dff").len() <= IMG_RUNTIME_SAFE_ENTRY_NAME_BYTES);
            }
        }
    }

    #[test]
    fn blender_counter_clockwise_face_is_reoriented_for_renderware() {
        let a = vertex(0.0, 0.0, 0.0);
        let b = vertex(1.0, 0.0, 0.0);
        let c = vertex(0.0, 1.0, 0.0);
        assert!(!renderware_winding_matches_normals(&a, &b, &c));
        assert!(renderware_winding_matches_normals(&a, &c, &b));
    }

    #[test]
    fn blender_uv_v_axis_is_converted_for_renderware() {
        assert_eq!(
            blender_uv_to_renderware(V2 { u: 0.25, v: 0.75 }),
            V2 { u: 0.25, v: 0.25 }
        );
        assert_eq!(
            blender_uv_to_renderware(V2 { u: -2.0, v: -0.5 }),
            V2 { u: -2.0, v: 1.5 }
        );
    }

    #[test]
    fn material_heavy_chunks_are_split_and_compacted_to_gta_limit() {
        let materials = GTA_DFF_MATERIAL_LIMIT * 2 + 1;
        let triangles = (0..materials)
            .map(|material| {
                (
                    vertex(0.0, 0.0, 0.0),
                    vertex(1.0, 0.0, 0.0),
                    vertex(0.0, 1.0, 0.0),
                    material as u16,
                )
            })
            .collect::<Vec<_>>();
        let object = NativeObject {
            zone: "zone".into(),
            name: "material_heavy".into(),
            txd: "textures".into(),
            tag: "object".into(),
            lod_parent: String::new(),
            lod_distance: 300.0,
            dimension: 0,
            interior: 0,
            origin: V3::default(),
            rotation: V3::default(),
            time_in: 0,
            time_out: 24,
            definition_flags: 0,
            has_day: false,
            has_night: false,
            textures: (0..materials)
                .map(|index| format!("texture_{index}"))
                .collect(),
            materials: vec![RawMaterial::default(); materials],
            vertices: Vec::new(),
            triangles: Vec::new(),
        };

        let chunks = split_chunk_by_material_limit(triangles);
        assert_eq!(
            chunks.iter().map(Vec::len).collect::<Vec<_>>(),
            [152, 152, 1]
        );
        assert_eq!(chunks.iter().map(Vec::len).sum::<usize>(), materials);
        for chunk in &chunks {
            let raw = chunk_to_raw(&object, chunk, V3::default());
            assert!(raw.material_textures.len() <= GTA_DFF_MATERIAL_LIMIT);
            assert_eq!(raw.material_textures.len(), chunk.len());
            assert!(
                raw.triangles
                    .iter()
                    .all(|triangle| (triangle.material as usize) < raw.material_textures.len())
            );
        }
        assert_eq!(
            chunk_to_raw(&object, &chunks[1], V3::default()).material_textures[0],
            "texture_152"
        );
    }

    #[test]
    fn unused_source_materials_do_not_force_an_element_split() {
        let triangles = vec![(
            vertex(0.0, 0.0, 0.0),
            vertex(1.0, 0.0, 0.0),
            vertex(0.0, 1.0, 0.0),
            199,
        )];
        let chunks = split_chunk_by_material_limit(triangles);
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn instance_verification_accepts_source_noise_but_rejects_model_changes() {
        let object = NativeObject {
            zone: "zone".to_string(),
            name: "lamp".to_string(),
            txd: "roads".to_string(),
            tag: "object".to_string(),
            lod_parent: String::new(),
            lod_distance: 300.0,
            dimension: 0,
            interior: 0,
            origin: V3::default(),
            rotation: V3::default(),
            time_in: 0,
            time_out: 24,
            definition_flags: 0,
            has_day: false,
            has_night: false,
            textures: vec!["lamp".to_string()],
            materials: vec![RawMaterial::default()],
            vertices: vec![
                vertex(0.0, 0.0, 0.0),
                vertex(1.0, 0.0, 0.0),
                vertex(0.0, 1.0, 0.0),
            ],
            triangles: vec![NativeTriangle {
                vertices: [0, 1, 2],
                material: 0,
            }],
        };
        let triangles = split_object(&object, 512.0, false).remove(0);
        let base = chunk_to_raw(&object, &triangles, V3::default());
        let mut source_noise = base.clone();
        source_noise.vertices[0].x += 0.0005;
        assert!(same_raw_instance(&base, &source_noise));
        source_noise.vertices[0].x += 0.002;
        assert!(!same_raw_instance(&base, &source_noise));
    }

    #[test]
    fn identical_evaluated_models_instance_but_scaled_geometry_does_not() {
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let root = env::temp_dir().join(format!("eagle_native_instances_{nonce}"));
        fs::create_dir_all(&root).unwrap();
        let stream = root.join("instances.eagmesh");
        let mut bytes = NATIVE_MESH_MAGIC.to_vec();
        bytes.extend_from_slice(&3u32.to_le_bytes());
        push_fixture_object(&mut bytes, "lamp02oo", V3::default(), V3::default(), 1.0);
        push_fixture_object(
            &mut bytes,
            "lamp02oo",
            V3 {
                x: 10.0,
                y: 20.0,
                z: 0.0,
            },
            V3 {
                x: 0.0,
                y: 0.0,
                z: 90.0,
            },
            1.0,
        );
        push_fixture_object(
            &mut bytes,
            "lamp02oo",
            V3 {
                x: 30.0,
                y: 0.0,
                z: 0.0,
            },
            V3::default(),
            2.0,
        );
        fs::write(&stream, bytes).unwrap();
        let (tx, _rx) = mpsc::channel();
        let build = build_native_blender_assets(
            &stream,
            &root,
            &BlenderImportOptions {
                chunk_size: 512.0,
                chunk_meshes: false,
                center_origins: true,
            },
            &tx,
        )
        .unwrap();

        assert_eq!(build.dff_entries.len(), 2);
        assert_ne!(
            lower(&build.dff_entries[0].0),
            lower(&build.dff_entries[1].0)
        );
        assert!(
            build
                .warnings
                .iter()
                .any(|warning| warning.contains("Reused 1"))
        );
        let definition =
            fs::read_to_string(root.join("zones/TestZone/TestZone.definition")).unwrap();
        assert_eq!(definition.matches("<definition ").count(), 2);
        let map = fs::read_to_string(root.join("zones/TestZone/TestZone.map")).unwrap();
        assert_eq!(map.matches("<building ").count(), 3);
        assert!(map.contains("rotZ=\"90.000000\""));
        assert!(!root.join("zones/TestZone/dff").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn building_and_object_instances_use_distinct_normal_policies() {
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let root = env::temp_dir().join(format!("eagle_native_normal_policy_{nonce}"));
        fs::create_dir_all(&root).unwrap();
        let stream = root.join("normal_policy.eagmesh");
        let mut bytes = NATIVE_MESH_MAGIC.to_vec();
        bytes.extend_from_slice(&2u32.to_le_bytes());
        push_fixture_object_with_tag(
            &mut bytes,
            "shared_mesh",
            "building",
            V3::default(),
            V3::default(),
            1.0,
        );
        push_fixture_object_with_tag(
            &mut bytes,
            "shared_mesh",
            "object",
            V3 {
                x: 10.0,
                y: 0.0,
                z: 0.0,
            },
            V3::default(),
            1.0,
        );
        fs::write(&stream, bytes).unwrap();
        let (tx, _rx) = mpsc::channel();
        let build = build_native_blender_assets(
            &stream,
            &root,
            &BlenderImportOptions {
                chunk_size: 512.0,
                chunk_meshes: false,
                center_origins: true,
            },
            &tx,
        )
        .unwrap();

        assert_eq!(build.dff_entries.len(), 2);
        let building = &build.dff_entries[0].1;
        let object = &build.dff_entries[1].1;
        assert!(!dff_has_geometry_normals(building));
        assert!(dff_has_geometry_normals(object));
        for dff in [building, object] {
            assert_eq!(dff_normal_less_lit_geometry_count(dff), Ok(0));
            assert_eq!(dff_redundant_bin_mesh_batch_count(dff), Ok(0));
        }
        let map = fs::read_to_string(root.join("zones/TestZone/TestZone.map")).unwrap();
        assert_eq!(map.matches("<building ").count(), 1);
        assert_eq!(map.matches("<object ").count(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn native_stream_builds_dff_definition_and_map() {
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let root = env::temp_dir().join(format!("eagle_native_build_{nonce}"));
        fs::create_dir_all(&root).unwrap();
        let stream = root.join("fixture.eagmesh");
        let mut bytes = NATIVE_MESH_MAGIC.to_vec();
        bytes.extend_from_slice(&1u32.to_le_bytes());
        for value in ["TestZone", "Plane", "roads", "building", ""] {
            push_string(&mut bytes, value);
        }
        bytes.extend_from_slice(&700.0f32.to_le_bytes());
        bytes.extend_from_slice(&0i32.to_le_bytes());
        bytes.extend_from_slice(&0i32.to_le_bytes());
        for value in [0.0f32, 0.0, 0.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in [0.0f32, 0.0, 0.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&0i32.to_le_bytes());
        bytes.extend_from_slice(&24i32.to_le_bytes());
        bytes.extend_from_slice(&0u64.to_le_bytes());
        bytes.push(1); // primary UV set
        bytes.push(0); // no vertex colors
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&3u32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        push_string(&mut bytes, "");
        for value in [1.0f32, 1.0, 1.0, 1.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for (x, y) in [(0.0f32, 0.0f32), (2.0, 0.0), (0.0, 2.0)] {
            for value in [x, y, 0.0, 0.0, 0.0, 1.0, x, y] {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            bytes.extend_from_slice(&[255; 8]);
        }
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&2u32.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        fs::write(&stream, bytes).unwrap();
        let (tx, _rx) = mpsc::channel();
        let build = build_native_blender_assets(
            &stream,
            &root,
            &BlenderImportOptions {
                chunk_size: 1.0,
                chunk_meshes: true,
                center_origins: true,
            },
            &tx,
        )
        .unwrap();
        assert_eq!(build.zones, vec!["TestZone"]);
        assert!(build.dff_entries.len() > 1);
        assert!(
            build
                .dff_entries
                .iter()
                .all(|(_, dff)| !parse_dff_mesh(dff).triangles.is_empty())
        );
        assert!(build.dff_entries.iter().all(|(_, dff)| {
            !dff_has_geometry_normals(dff)
                && dff_normal_less_lit_geometry_count(dff) == Ok(0)
                && dff_redundant_bin_mesh_batch_count(dff) == Ok(0)
        }));
        let definition =
            fs::read_to_string(root.join("zones/TestZone/TestZone.definition")).unwrap();
        assert!(definition.contains("txd=\"roads\""));
        let map = fs::read_to_string(root.join("zones/TestZone/TestZone.map")).unwrap();
        assert!(map.contains("<building"));
        assert_eq!(
            fs::read_to_string(root.join("eagleZones.txt")).unwrap(),
            "TestZone\n"
        );
        assert!(!root.join("zones/TestZone/dff").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
