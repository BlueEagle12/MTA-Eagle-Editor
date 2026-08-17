use super::super::*;

pub(crate) fn upsert_replacement_col(
    root: &Path,
    col_name: &str,
    bytes: &[u8],
) -> Result<(), String> {
    upsert_replacement_asset(root, col_name, bytes)
}

/// Rewrites the internal model-name field of every COL model in `bytes` to
/// `model_name`. GTA/MTA bind a collision to a model by this internal name (the
/// 22-byte field at offset +8 of each COL2/COL3/COL4/COLL model), NOT by the IMG
/// entry filename. Replacement COLs were written verbatim, so their stale internal
/// name matched no model in-game and the collision never attached — unlike DFFs,
/// whose internal frame name is normalized to the target on replace. Returns the
/// number of COL models that were renamed.
pub(crate) fn set_col_model_names(bytes: &mut [u8], model_name: &str) -> usize {
    const NAME_OFFSET: usize = 8;
    const NAME_LEN: usize = 22;
    let mut model_start = 0usize;
    let mut renamed = 0usize;
    while model_start + NAME_OFFSET + NAME_LEN <= bytes.len() {
        let magic = &bytes[model_start..model_start + 4];
        if !matches!(magic, b"COL2" | b"COL3" | b"COL4" | b"COLL") {
            break;
        }
        let file_size = rd32(bytes, model_start + 4) as usize;
        let model_end = model_start.saturating_add(file_size).saturating_add(8);
        if model_end <= model_start + 8 || model_end > bytes.len() {
            break;
        }
        let name_field =
            &mut bytes[model_start + NAME_OFFSET..model_start + NAME_OFFSET + NAME_LEN];
        name_field.fill(0);
        let src = model_name.as_bytes();
        // Leave room for at least one terminating NUL within the fixed-size field.
        let count = src.len().min(NAME_LEN - 1);
        name_field[..count].copy_from_slice(&src[..count]);
        renamed += 1;
        model_start = model_end;
    }
    renamed
}

pub(crate) fn set_col_model_names_from_entry(bytes: &mut [u8], entry_name: &str) -> usize {
    let model_name = Path::new(entry_name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(entry_name);
    set_col_model_names(bytes, model_name)
}

fn write_i16_fixed(out: &mut Vec<u8>, value: f32) -> Result<(), String> {
    let scaled = (value * 128.0).round();
    if scaled < i16::MIN as f32 || scaled > i16::MAX as f32 {
        return Err("COL vertex is outside the signed 16-bit fixed-point range".to_string());
    }
    out.extend_from_slice(&(scaled as i16).to_le_bytes());
    Ok(())
}

fn write_surface(out: &mut Vec<u8>, surface: &CollisionSurface) {
    out.push(surface.material);
    out.push(surface.flags);
    out.push(surface.brightness);
    out.push(surface.light);
}

const FACE_GROUP_MIN_FACES: usize = 64;
const FACE_GROUP_SIZE: usize = 32;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ColSpatialGroupStats {
    pub(crate) groups_generated: usize,
    pub(crate) faces_grouped: usize,
    pub(crate) faces_reordered: usize,
}

#[derive(Clone, Copy)]
struct ColFaceGroup {
    min: V3,
    max: V3,
    first: u16,
    last: u16,
}

fn quantized_component(value: f32) -> i32 {
    (value * 128.0).round() as i32
}

fn spread_bits_10(mut value: u32) -> u32 {
    value &= 0x0000_03ff;
    value = (value | (value << 16)) & 0x0300_00ff;
    value = (value | (value << 8)) & 0x0300_f00f;
    value = (value | (value << 4)) & 0x030c_30c3;
    value = (value | (value << 2)) & 0x0924_9249;
    value
}

fn morton_3d(x: u32, y: u32, z: u32) -> u32 {
    spread_bits_10(x) | (spread_bits_10(y) << 1) | (spread_bits_10(z) << 2)
}

fn normalized_morton_component(value: i64, min: i64, max: i64) -> u32 {
    if min >= max {
        return 0;
    }
    (((value - min) as i128 * 1023) / (max - min) as i128) as u32
}

fn spatially_grouped_faces(mesh: &CollisionMesh) -> (Vec<CollisionFace>, Vec<ColFaceGroup>) {
    if mesh.faces.len() < FACE_GROUP_MIN_FACES {
        return (mesh.faces.clone(), Vec::new());
    }
    let centroids = mesh
        .faces
        .iter()
        .map(|face| {
            let vertices = [face.a, face.b, face.c];
            let mut sum = [0i64; 3];
            for index in vertices {
                let vertex = mesh.vertices[index as usize];
                sum[0] += quantized_component(vertex.x) as i64;
                sum[1] += quantized_component(vertex.y) as i64;
                sum[2] += quantized_component(vertex.z) as i64;
            }
            sum
        })
        .collect::<Vec<_>>();
    let mut min = [i64::MAX; 3];
    let mut max = [i64::MIN; 3];
    for centroid in &centroids {
        for axis in 0..3 {
            min[axis] = min[axis].min(centroid[axis]);
            max[axis] = max[axis].max(centroid[axis]);
        }
    }
    let mut order = (0..mesh.faces.len()).collect::<Vec<_>>();
    order.sort_by_key(|index| {
        let centroid = centroids[*index];
        let face = &mesh.faces[*index];
        (
            morton_3d(
                normalized_morton_component(centroid[0], min[0], max[0]),
                normalized_morton_component(centroid[1], min[1], max[1]),
                normalized_morton_component(centroid[2], min[2], max[2]),
            ),
            face.material,
            face.light,
            face.a,
            face.b,
            face.c,
        )
    });
    let faces = order
        .iter()
        .map(|index| mesh.faces[*index].clone())
        .collect::<Vec<_>>();
    let mut groups = Vec::with_capacity(faces.len().div_ceil(FACE_GROUP_SIZE));
    for first in (0..faces.len()).step_by(FACE_GROUP_SIZE) {
        let last = (first + FACE_GROUP_SIZE).min(faces.len()) - 1;
        let mut group_min = V3 {
            x: f32::INFINITY,
            y: f32::INFINITY,
            z: f32::INFINITY,
        };
        let mut group_max = V3 {
            x: f32::NEG_INFINITY,
            y: f32::NEG_INFINITY,
            z: f32::NEG_INFINITY,
        };
        for face in &faces[first..=last] {
            for index in [face.a, face.b, face.c] {
                let vertex = mesh.vertices[index as usize];
                // Group bounds describe the fixed-point vertices that are
                // actually written, not higher-precision editor coordinates.
                // This keeps a write -> parse -> write cycle byte-stable.
                let vertex = V3 {
                    x: quantized_component(vertex.x) as f32 / 128.0,
                    y: quantized_component(vertex.y) as f32 / 128.0,
                    z: quantized_component(vertex.z) as f32 / 128.0,
                };
                group_min.x = group_min.x.min(vertex.x);
                group_min.y = group_min.y.min(vertex.y);
                group_min.z = group_min.z.min(vertex.z);
                group_max.x = group_max.x.max(vertex.x);
                group_max.y = group_max.y.max(vertex.y);
                group_max.z = group_max.z.max(vertex.z);
            }
        }
        groups.push(ColFaceGroup {
            min: group_min,
            max: group_max,
            first: first as u16,
            last: last as u16,
        });
    }
    (faces, groups)
}

pub(crate) fn col_spatial_group_stats(mesh: &CollisionMesh) -> ColSpatialGroupStats {
    let (faces, groups) = spatially_grouped_faces(mesh);
    let faces_reordered = faces
        .iter()
        .zip(&mesh.faces)
        .filter(|(left, right)| {
            left.a != right.a
                || left.b != right.b
                || left.c != right.c
                || left.material != right.material
                || left.light != right.light
        })
        .count();
    ColSpatialGroupStats {
        groups_generated: groups.len(),
        faces_grouped: usize::from(!groups.is_empty()) * faces.len(),
        faces_reordered,
    }
}

fn source_face_matches(source: &[u8], offset: usize, expected: &CollisionFace) -> bool {
    offset + 8 <= source.len()
        && rd16(source, offset) == expected.a
        && rd16(source, offset + 2) == expected.b
        && rd16(source, offset + 4) == expected.c
        && source[offset + 6] == expected.material
        && source[offset + 7] == expected.light
}

fn source_group_matches(source: &[u8], offset: usize, expected: ColFaceGroup) -> bool {
    let values = [
        expected.min.x,
        expected.min.y,
        expected.min.z,
        expected.max.x,
        expected.max.y,
        expected.max.z,
    ];
    offset + 28 <= source.len()
        && values
            .iter()
            .enumerate()
            .all(|(idx, value)| rd32(source, offset + idx * 4) == value.to_bits())
        && rd16(source, offset + 24) == expected.first
        && rd16(source, offset + 26) == expected.last
}

/// Return true when serializing `mesh` would add, remove, or deterministically
/// rebuild its face-group acceleration data. This gives callers a cheap,
/// explicit rewrite trigger even when collision geometry itself did not need
/// repair.
pub(crate) fn col_spatial_face_groups_need_rebuild(source: &[u8], mesh: &CollisionMesh) -> bool {
    let Some((model_start, model_end)) = first_col_model_range(source) else {
        return true;
    };
    let (faces, groups) = spatially_grouped_faces(mesh);
    let header = model_start + 72;
    let has_groups = rd32(source, header + 8) & 8 != 0;
    if groups.is_empty() {
        return has_groups;
    }
    if !has_groups
        || rd16(source, header + 4) as usize != faces.len()
        || rd32(source, header + 28) == 0
    {
        return true;
    }
    let faces_abs = model_start + rd32(source, header + 28) as usize + 4;
    let Some(count_pos) = faces_abs.checked_sub(4) else {
        return true;
    };
    let Some(groups_bytes) = groups.len().checked_mul(28) else {
        return true;
    };
    let Some(groups_start) = count_pos.checked_sub(groups_bytes) else {
        return true;
    };
    let header_len = match &source[model_start..model_start + 4] {
        b"COL4" => 124,
        b"COL3" => 120,
        _ => 116,
    };
    if faces_abs + faces.len() * 8 > model_end
        || count_pos + 4 > model_end
        || groups_start < model_start + header_len
        || rd32(source, count_pos) as usize != groups.len()
    {
        return true;
    }
    groups
        .iter()
        .enumerate()
        .any(|(idx, group)| !source_group_matches(source, groups_start + idx * 28, *group))
        || faces
            .iter()
            .enumerate()
            .any(|(idx, face)| !source_face_matches(source, faces_abs + idx * 8, face))
}

fn write_face_group(out: &mut Vec<u8>, group: ColFaceGroup) {
    for value in [
        group.min.x,
        group.min.y,
        group.min.z,
        group.max.x,
        group.max.y,
        group.max.z,
    ] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.extend_from_slice(&group.first.to_le_bytes());
    out.extend_from_slice(&group.last.to_le_bytes());
}

fn first_col_model_range(bytes: &[u8]) -> Option<(usize, usize)> {
    if bytes.len() < 8 || !matches!(&bytes[0..4], b"COL2" | b"COL3" | b"COL4") {
        return None;
    }
    let file_size = rd32(bytes, 4) as usize;
    let end = file_size.checked_add(8)?;
    let min_len = match &bytes[0..4] {
        b"COL4" => 124,
        b"COL3" => 120,
        _ => 116,
    };
    (end <= bytes.len() && end >= min_len).then_some((0, end))
}

/// Build a bare COL2 model header (116 bytes) usable as a serializer template.
///
/// `write_col_mesh_from_template` only needs the template for its magic, name,
/// model id and header bytes it does not overwrite; the geometry is taken from
/// the mesh. A zeroed COL2 header therefore round-trips any mesh into a valid
/// COL2 model.
pub(crate) fn minimal_col2_template(name: &str) -> Vec<u8> {
    let mut model = vec![0u8; 116];
    let file_size = (model.len() - 8) as u32;
    model[0..4].copy_from_slice(b"COL2");
    // file_size: everything after this dword. The serializer overwrites this
    // with the real size, but first_col_model_range validates it first and
    // requires the model to be at least a full COL2 header.
    model[4..8].copy_from_slice(&file_size.to_le_bytes());
    let stem = Path::new(name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(name);
    let raw = stem.as_bytes();
    let len = raw.len().min(21);
    model[8..8 + len].copy_from_slice(&raw[..len]);
    model
}

/// Return a serializer template for `source`. COL2/3/4 sources are used as-is
/// so their version and header details are preserved; anything else (notably
/// COL version 1 "COLL") is upgraded to a minimal COL2 template.
pub(crate) fn col_write_template(source: &[u8], name: &str) -> Vec<u8> {
    if first_col_model_range(source).is_some() {
        source.to_vec()
    } else {
        minimal_col2_template(name)
    }
}

/// Return a one-model template for full collision regeneration.
///
/// Regular editing preserves trailing COL models in a packed entry. Global
/// regeneration deliberately drops them so no stale referenced collision model
/// can survive behind the newly generated model.
pub(crate) fn col_regeneration_template(source: &[u8], name: &str) -> Vec<u8> {
    let mut template = col_write_template(source, name);
    if let Some((_, model_end)) = first_col_model_range(&template) {
        template.truncate(model_end);
    }
    template
}

pub(crate) fn write_col_mesh_from_template(
    template: &[u8],
    mesh: &CollisionMesh,
) -> Result<Vec<u8>, String> {
    let bounds_only = mesh.vertices.is_empty()
        && mesh.faces.is_empty()
        && mesh.spheres.is_empty()
        && mesh.boxes.is_empty();
    write_col_mesh_from_template_with_bounds(template, mesh, bounds_only.then_some(mesh.bounds))
}

/// Replace exactly one model in a packed COL entry.
///
/// The identity is captured when the editor opens the source. Both its ordinal
/// and original name/version must still match, otherwise staging fails instead
/// of risking an edit to a different internal model. COL1 targets are upgraded
/// in place to COL2 while all leading and trailing models remain byte-identical.
pub(crate) fn write_col_mesh_replacing_model(
    source: &[u8],
    identity: &ColModelIdentity,
    mesh: &CollisionMesh,
) -> Result<Vec<u8>, String> {
    let ranges = col_model_ranges(source);
    let range = ranges.get(identity.index).ok_or_else(|| {
        format!(
            "COL model {} ({}) no longer exists in the packed entry",
            identity.index + 1,
            identity.name
        )
    })?;
    if range.identity.name != identity.name || range.identity.magic != identity.magic {
        return Err(format!(
            "COL model {} no longer matches the opened model {}",
            identity.index + 1,
            identity.name
        ));
    }

    let template = if &identity.magic == b"COLL" {
        minimal_col2_template(&identity.name)
    } else {
        source[range.start..range.end].to_vec()
    };
    let replacement = write_col_mesh_from_template(&template, mesh)?;
    let mut out = Vec::with_capacity(
        source
            .len()
            .saturating_sub(range.end - range.start)
            .saturating_add(replacement.len()),
    );
    out.extend_from_slice(&source[..range.start]);
    out.extend_from_slice(&replacement);
    out.extend_from_slice(&source[range.end..]);
    Ok(out)
}

/// Serialize `mesh`, using `header_bounds` verbatim for the COL broad-phase
/// header when supplied. Automatic generation passes the source DFF bounds.
pub(crate) fn write_col_mesh_from_template_with_bounds(
    template: &[u8],
    mesh: &CollisionMesh,
    header_bounds: Option<Bounds>,
) -> Result<Vec<u8>, String> {
    let (model_start, model_end) = first_col_model_range(template)
        .ok_or_else(|| "COL template is not COL2/COL3/COL4".to_string())?;
    if mesh.faces.is_empty() && mesh.vertices.is_empty() {
        // Primitive-only and intentionally empty COLs are valid. Empty models
        // are used when every source DFF material is flagged No Collision.
    } else if mesh.faces.is_empty() {
        return Err(format!("COL {} has vertices but no faces", mesh.name));
    }
    if mesh.vertices.len() > u16::MAX as usize {
        return Err(format!(
            "COL {} has {} vertices, exceeding the triangle-index limit of {}",
            mesh.name,
            mesh.vertices.len(),
            u16::MAX
        ));
    }
    if mesh.faces.len() > u16::MAX as usize {
        return Err(format!(
            "COL {} has {} faces, exceeding the COL2/COL3/COL4 limit of {}",
            mesh.name,
            mesh.faces.len(),
            u16::MAX
        ));
    }
    for (idx, face) in mesh.faces.iter().enumerate() {
        let max_idx = mesh.vertices.len().saturating_sub(1);
        if face.a as usize > max_idx || face.b as usize > max_idx || face.c as usize > max_idx {
            return Err(format!("COL face {idx} references a missing vertex"));
        }
        if face.a == face.b || face.b == face.c || face.c == face.a {
            return Err(format!("COL face {idx} is degenerate"));
        }
    }
    if mesh.shadow_faces.is_empty() != mesh.shadow_vertices.is_empty() {
        return Err(format!(
            "COL {} shadow mesh must contain both vertices and faces",
            mesh.name
        ));
    }
    if mesh.shadow_vertices.len() > u16::MAX as usize {
        return Err(format!(
            "COL {} shadow mesh has {} vertices, exceeding the triangle-index limit of {}",
            mesh.name,
            mesh.shadow_vertices.len(),
            u16::MAX
        ));
    }
    if mesh.shadow_faces.len() > u32::MAX as usize {
        return Err(format!("COL {} shadow mesh has too many faces", mesh.name));
    }
    for (idx, face) in mesh.shadow_faces.iter().enumerate() {
        let max_idx = mesh.shadow_vertices.len().saturating_sub(1);
        if face.a as usize > max_idx || face.b as usize > max_idx || face.c as usize > max_idx {
            return Err(format!("COL shadow face {idx} references a missing vertex"));
        }
        if face.a == face.b || face.b == face.c || face.c == face.a {
            return Err(format!("COL shadow face {idx} is degenerate"));
        }
    }
    let (faces, face_groups) = spatially_grouped_faces(mesh);

    let source_magic = &template[model_start..model_start + 4];
    let col_has_shadow_header =
        matches!(source_magic, b"COL3" | b"COL4") || !mesh.shadow_faces.is_empty();
    let header_len = match &template[model_start..model_start + 4] {
        b"COL4" => 124usize,
        b"COL3" => 120usize,
        _ if col_has_shadow_header => 120usize,
        _ => 116usize,
    };
    let source_header_len = match source_magic {
        b"COL4" => 124usize,
        b"COL3" => 120usize,
        _ => 116usize,
    };
    let mut model = template[model_start..model_start + source_header_len].to_vec();
    if header_len > source_header_len {
        model.resize(header_len, 0);
        model[0..4].copy_from_slice(b"COL3");
    }
    let header = 72usize;
    let spheres_abs = header_len;
    let boxes_abs = spheres_abs + mesh.spheres.len() * 20;
    let vertices_abs = boxes_abs + mesh.boxes.len() * 28;
    let face_groups_abs = vertices_abs + mesh.vertices.len() * 6;
    let faces_abs =
        face_groups_abs + face_groups.len() * 28 + usize::from(!face_groups.is_empty()) * 4;
    let spheres_offset = if mesh.spheres.is_empty() {
        0
    } else {
        spheres_abs.saturating_sub(4) as u32
    };
    let boxes_offset = if mesh.boxes.is_empty() {
        0
    } else {
        boxes_abs.saturating_sub(4) as u32
    };
    let vertices_offset = if mesh.vertices.is_empty() {
        0
    } else {
        vertices_abs.saturating_sub(4) as u32
    };
    let faces_offset = if mesh.faces.is_empty() {
        0
    } else {
        faces_abs.saturating_sub(4) as u32
    };
    model[header..header + 2].copy_from_slice(&(mesh.spheres.len() as u16).to_le_bytes());
    model[header + 2..header + 4].copy_from_slice(&(mesh.boxes.len() as u16).to_le_bytes());
    model[header + 4..header + 6].copy_from_slice(&(mesh.faces.len() as u16).to_le_bytes());
    let mut flags = rd32(&model, header + 8);
    if !mesh.shadow_faces.is_empty() {
        flags |= 16;
    } else {
        flags &= !16;
    }
    if face_groups.is_empty() {
        flags &= !8;
    } else {
        flags |= 8;
    }
    if !mesh.spheres.is_empty() || !mesh.boxes.is_empty() || !mesh.faces.is_empty() {
        flags |= 2;
    } else {
        flags &= !2;
    }
    model[header + 8..header + 12].copy_from_slice(&flags.to_le_bytes());
    model[header + 12..header + 16].copy_from_slice(&spheres_offset.to_le_bytes());
    model[header + 16..header + 20].copy_from_slice(&boxes_offset.to_le_bytes());
    model[header + 24..header + 28].copy_from_slice(&vertices_offset.to_le_bytes());
    model[header + 28..header + 32].copy_from_slice(&faces_offset.to_le_bytes());
    if col_has_shadow_header {
        model[header + 36..header + 48].fill(0);
    }

    let bounds = header_bounds
        .unwrap_or_else(|| collision_mesh_bounds(&mesh.vertices, &mesh.spheres, &mesh.boxes));
    let center = (bounds.min + bounds.max) * 0.5;
    let bounds_points = [
        bounds.min,
        vec3(bounds.max.x, bounds.min.y, bounds.min.z),
        vec3(bounds.min.x, bounds.max.y, bounds.min.z),
        vec3(bounds.max.x, bounds.max.y, bounds.min.z),
        vec3(bounds.min.x, bounds.min.y, bounds.max.z),
        vec3(bounds.max.x, bounds.min.y, bounds.max.z),
        vec3(bounds.min.x, bounds.max.y, bounds.max.z),
        bounds.max,
    ];
    let radius = bounds_points
        .iter()
        .map(|vertex| (*vertex - center).length())
        .fold(0.0f32, f32::max);
    // COL2/COL3/COL4 TBounds layout is: min (TVector), max (TVector),
    // center (TVector), radius (float). This differs from version 1 (COLL),
    // which is radius, center, min, max. Writing the wrong order produces a
    // garbage bounding volume in-game: the broad-phase spatial query never
    // matches the model, so collisions silently fail even though the mesh is
    // intact. The editor is unaffected because parse_col_mesh recomputes bounds
    // from geometry and ignores these header fields.
    for (offset, value) in [
        (32usize, bounds.min.x),
        (36, bounds.min.y),
        (40, bounds.min.z),
        (44, bounds.max.x),
        (48, bounds.max.y),
        (52, bounds.max.z),
        (56, center.x),
        (60, center.y),
        (64, center.z),
        (68, radius),
    ] {
        if offset + 4 <= model.len() {
            model[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
    }

    for sphere in &mesh.spheres {
        model.extend_from_slice(&sphere.center.x.to_le_bytes());
        model.extend_from_slice(&sphere.center.y.to_le_bytes());
        model.extend_from_slice(&sphere.center.z.to_le_bytes());
        model.extend_from_slice(&sphere.radius.abs().to_le_bytes());
        write_surface(&mut model, &sphere.surface);
    }
    for col_box in &mesh.boxes {
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
        for value in [min.x, min.y, min.z, max.x, max.y, max.z] {
            model.extend_from_slice(&value.to_le_bytes());
        }
        write_surface(&mut model, &col_box.surface);
    }
    for vertex in &mesh.vertices {
        write_i16_fixed(&mut model, vertex.x)?;
        write_i16_fixed(&mut model, vertex.y)?;
        write_i16_fixed(&mut model, vertex.z)?;
    }
    for group in &face_groups {
        write_face_group(&mut model, *group);
    }
    if !face_groups.is_empty() {
        model.extend_from_slice(&(face_groups.len() as u32).to_le_bytes());
    }
    for face in &faces {
        model.extend_from_slice(&face.a.to_le_bytes());
        model.extend_from_slice(&face.b.to_le_bytes());
        model.extend_from_slice(&face.c.to_le_bytes());
        model.push(face.material);
        model.push(face.light);
    }
    if !mesh.shadow_faces.is_empty() {
        let shadow_vertices_abs = model.len();
        for vertex in &mesh.shadow_vertices {
            write_i16_fixed(&mut model, vertex.x)?;
            write_i16_fixed(&mut model, vertex.y)?;
            write_i16_fixed(&mut model, vertex.z)?;
        }
        let shadow_faces_abs = model.len();
        for face in &mesh.shadow_faces {
            model.extend_from_slice(&face.a.to_le_bytes());
            model.extend_from_slice(&face.b.to_le_bytes());
            model.extend_from_slice(&face.c.to_le_bytes());
            model.push(face.material);
            model.push(face.light);
        }
        model[header + 36..header + 40]
            .copy_from_slice(&(mesh.shadow_faces.len() as u32).to_le_bytes());
        model[header + 40..header + 44]
            .copy_from_slice(&((shadow_vertices_abs - 4) as u32).to_le_bytes());
        model[header + 44..header + 48]
            .copy_from_slice(&((shadow_faces_abs - 4) as u32).to_le_bytes());
    }
    let file_size = model
        .len()
        .checked_sub(8)
        .ok_or_else(|| "COL model is too short".to_string())?;
    model[4..8].copy_from_slice(&(file_size as u32).to_le_bytes());
    let mut out = model;
    out.extend_from_slice(&template[model_end..]);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_col_template(magic: &[u8; 4]) -> Vec<u8> {
        let header_len = match magic {
            b"COL4" => 124,
            b"COL3" => 120,
            _ => 116,
        };
        let mut bytes = vec![0u8; header_len];
        bytes[0..4].copy_from_slice(magic);
        bytes[8..12].copy_from_slice(b"test");
        bytes[76..78].copy_from_slice(&1u16.to_le_bytes());
        bytes[96..100].copy_from_slice(&((header_len - 4) as u32).to_le_bytes());
        bytes[100..104].copy_from_slice(&((header_len + 18 - 4) as u32).to_le_bytes());
        if header_len >= 120 {
            bytes[116..120].copy_from_slice(&0u32.to_le_bytes());
        }
        for value in [0i16, 0, 0, 128, 0, 0, 0, 128, 0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.push(3);
        bytes.push(4);
        let size = (bytes.len() - 8) as u32;
        bytes[4..8].copy_from_slice(&size.to_le_bytes());
        bytes
    }

    fn named_test_col_template(magic: &[u8; 4], name: &str) -> Vec<u8> {
        let mut bytes = test_col_template(magic);
        bytes[8..30].fill(0);
        let raw = name.as_bytes();
        let len = raw.len().min(21);
        bytes[8..8 + len].copy_from_slice(&raw[..len]);
        bytes
    }

    fn assert_col_mesh_serializer_round_trips_parser_subset(magic: &[u8; 4]) {
        let template = test_col_template(magic);
        let entry = ImgEntry {
            img_path: PathBuf::from("test.col"),
            name: "test.col".to_string(),
            offset: 0,
            size: template.len() as u32,
        };
        let mut mesh = parse_col_mesh(&template, &entry).unwrap();
        mesh.vertices[1].x = 2.0;
        mesh.faces[0].material = 9;
        mesh.faces.push(CollisionFace {
            a: 0,
            b: 2,
            c: 1,
            material: 7,
            light: 8,
            img_path: PathBuf::from("test.col"),
            material_file_offset: 0,
            light_file_offset: 0,
        });

        let bytes = write_col_mesh_from_template(&template, &mesh).unwrap();
        let reparsed = parse_col_mesh(&bytes, &entry).unwrap();

        assert_eq!(reparsed.vertices.len(), 3);
        assert_eq!(reparsed.faces.len(), 2);
        assert!((reparsed.vertices[1].x - 2.0).abs() < 0.001);
        assert!((reparsed.bounds.max.x - 2.0).abs() < 0.001);
        assert_eq!(reparsed.faces[0].material, 9);
        assert_eq!(reparsed.faces[1].light, 8);
    }

    #[test]
    fn regeneration_template_drops_stale_trailing_models() {
        let first = test_col_template(b"COL3");
        let mut bundled = first.clone();
        bundled.extend_from_slice(&test_col_template(b"COL3"));

        let template = col_regeneration_template(&bundled, "target.col");
        assert_eq!(template.len(), first.len());

        let mesh = CollisionMesh {
            name: "target".to_string(),
            spheres: Vec::new(),
            boxes: Vec::new(),
            vertices: Vec::new(),
            faces: Vec::new(),
            bounds: Bounds {
                min: Vec3::ZERO,
                max: Vec3::ZERO,
            },
            shadow_vertices: Vec::new(),
            shadow_faces: Vec::new(),
        };
        let regenerated = write_col_mesh_from_template(&template, &mesh).unwrap();
        assert_eq!(collect_col_model_names(&regenerated).len(), 1);
    }

    #[test]
    fn packed_replacement_changes_only_the_selected_internal_model() {
        let first = named_test_col_template(b"COL2", "first");
        let second = named_test_col_template(b"COL3", "target");
        let third = named_test_col_template(b"COL4", "last");
        let mut packed = first.clone();
        packed.extend_from_slice(&second);
        packed.extend_from_slice(&third);
        let ranges = col_model_ranges(&packed);
        let target = ranges[1].identity.clone();
        let entry = ImgEntry {
            img_path: PathBuf::from("bundle.col"),
            name: "bundle.col".to_string(),
            offset: 0,
            size: packed.len() as u32,
        };
        let mut mesh = parse_col_mesh_for_identity(&packed, &entry, &target).unwrap();
        mesh.faces[0].material = 77;

        let updated = write_col_mesh_replacing_model(&packed, &target, &mesh).unwrap();
        let updated_ranges = col_model_ranges(&updated);

        assert_eq!(updated_ranges.len(), 3);
        assert_eq!(
            &updated[updated_ranges[0].start..updated_ranges[0].end],
            first.as_slice()
        );
        assert_eq!(
            &updated[updated_ranges[2].start..updated_ranges[2].end],
            third.as_slice()
        );
        assert_eq!(
            collect_col_model_names(&updated),
            vec![
                "first.col".to_string(),
                "target.col".to_string(),
                "last.col".to_string()
            ]
        );
        let reparsed =
            parse_col_mesh_for_identity(&updated, &entry, &updated_ranges[1].identity).unwrap();
        assert_eq!(reparsed.faces[0].material, 77);
    }

    #[test]
    fn packed_col1_upgrade_preserves_trailing_models() {
        let mut col1 = vec![0u8; 72];
        col1[0..4].copy_from_slice(b"COLL");
        col1[4..8].copy_from_slice(&64u32.to_le_bytes());
        col1[8..14].copy_from_slice(b"legacy");
        let trailing = named_test_col_template(b"COL3", "untouched");
        let mut packed = col1;
        packed.extend_from_slice(&trailing);
        let target = col_model_ranges(&packed)[0].identity.clone();
        let mesh = CollisionMesh {
            name: "legacy".to_string(),
            spheres: Vec::new(),
            boxes: Vec::new(),
            vertices: Vec::new(),
            faces: Vec::new(),
            bounds: Bounds {
                min: Vec3::ZERO,
                max: Vec3::ZERO,
            },
            shadow_vertices: Vec::new(),
            shadow_faces: Vec::new(),
        };

        let updated = write_col_mesh_replacing_model(&packed, &target, &mesh).unwrap();
        let ranges = col_model_ranges(&updated);

        assert_eq!(ranges.len(), 2);
        assert_eq!(&ranges[0].identity.magic, b"COL2");
        assert_eq!(ranges[0].identity.name, "legacy");
        assert_eq!(
            &updated[ranges[1].start..ranges[1].end],
            trailing.as_slice()
        );
        assert_eq!(ranges[1].identity.name, "untouched");
    }

    #[test]
    fn packed_replacement_rejects_a_stale_model_identity() {
        let source = named_test_col_template(b"COL3", "target");
        let mut identity = col_model_ranges(&source)[0].identity.clone();
        identity.name = "some_other_model".to_string();
        let entry = ImgEntry {
            img_path: PathBuf::from("bundle.col"),
            name: "bundle.col".to_string(),
            offset: 0,
            size: source.len() as u32,
        };
        let mesh = parse_col_mesh(&source, &entry).unwrap();

        let error = write_col_mesh_replacing_model(&source, &identity, &mesh).unwrap_err();
        assert!(error.contains("no longer matches"));
    }

    #[test]
    fn col3_serializer_round_trips_spheres_and_boxes() {
        let template = test_col_template(b"COL3");
        let entry = ImgEntry {
            img_path: PathBuf::from("test.col"),
            name: "test.col".to_string(),
            offset: 0,
            size: template.len() as u32,
        };
        let mut mesh = parse_col_mesh(&template, &entry).unwrap();
        mesh.spheres.push(CollisionSphere {
            center: V3 {
                x: 4.0,
                y: 5.0,
                z: 6.0,
            },
            radius: 2.5,
            surface: CollisionSurface {
                material: 2,
                flags: 3,
                brightness: 4,
                light: 5,
            },
        });
        mesh.boxes.push(CollisionBox {
            min: V3 {
                x: -3.0,
                y: -2.0,
                z: -1.0,
            },
            max: V3 {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            surface: CollisionSurface {
                material: 8,
                flags: 7,
                brightness: 6,
                light: 5,
            },
        });

        let bytes = write_col_mesh_from_template(&template, &mesh).unwrap();
        let reparsed = parse_col_mesh(&bytes, &entry).unwrap();

        assert_eq!(reparsed.spheres.len(), 1);
        assert_eq!(reparsed.boxes.len(), 1);
        assert!((reparsed.spheres[0].center.x - 4.0).abs() < 0.001);
        assert!((reparsed.spheres[0].radius - 2.5).abs() < 0.001);
        assert_eq!(reparsed.spheres[0].surface.material, 2);
        assert_eq!(reparsed.boxes[0].surface.material, 8);
        assert!((reparsed.bounds.min.x + 3.0).abs() < 0.001);
        assert!((reparsed.bounds.max.x - 6.5).abs() < 0.001);
    }

    #[test]
    fn serializer_bounds_use_exact_visible_dff_geometry() {
        let template = test_col_template(b"COL2");
        let entry = ImgEntry {
            img_path: PathBuf::from("test.col"),
            name: "test.col".to_string(),
            offset: 0,
            size: template.len() as u32,
        };
        let mesh = parse_col_mesh(&template, &entry).unwrap();
        // Deliberately smaller than the triangle geometry. DFF bounds are the
        // authority and must not be unioned with generated/existing COL bounds.
        let visible = Bounds {
            min: vec3(-0.25, -0.5, -0.75),
            max: vec3(0.5, 0.625, 0.875),
        };

        let bytes =
            write_col_mesh_from_template_with_bounds(&template, &mesh, Some(visible)).unwrap();

        assert_eq!(rdf32(&bytes, 32), -0.25);
        assert_eq!(rdf32(&bytes, 36), -0.5);
        assert_eq!(rdf32(&bytes, 40), -0.75);
        assert_eq!(rdf32(&bytes, 44), 0.5);
        assert_eq!(rdf32(&bytes, 48), 0.625);
        assert_eq!(rdf32(&bytes, 52), 0.875);
    }

    #[test]
    fn serializer_writes_col_bounds_header() {
        let template = test_col_template(b"COL3");
        let entry = ImgEntry {
            img_path: PathBuf::from("test.col"),
            name: "test.col".to_string(),
            offset: 0,
            size: template.len() as u32,
        };
        let mut mesh = parse_col_mesh(&template, &entry).unwrap();
        mesh.vertices[0] = V3 {
            x: -4.0,
            y: -2.0,
            z: -1.0,
        };
        mesh.vertices[1] = V3 {
            x: 8.0,
            y: 5.0,
            z: 3.0,
        };
        mesh.vertices[2] = V3 {
            x: 2.0,
            y: 9.0,
            z: 7.0,
        };

        let bytes = write_col_mesh_from_template(&template, &mesh).unwrap();

        let bounds = collision_mesh_bounds(&mesh.vertices, &mesh.spheres, &mesh.boxes);
        let center = (bounds.min + bounds.max) * 0.5;
        let radius = [
            bounds.min,
            vec3(bounds.max.x, bounds.min.y, bounds.min.z),
            vec3(bounds.min.x, bounds.max.y, bounds.min.z),
            vec3(bounds.max.x, bounds.max.y, bounds.min.z),
            vec3(bounds.min.x, bounds.min.y, bounds.max.z),
            vec3(bounds.max.x, bounds.min.y, bounds.max.z),
            vec3(bounds.min.x, bounds.max.y, bounds.max.z),
            bounds.max,
        ]
        .into_iter()
        .map(|vertex| (vertex - center).length())
        .fold(0.0f32, f32::max);
        for (offset, expected) in [
            (32usize, bounds.min.x),
            (36, bounds.min.y),
            (40, bounds.min.z),
            (44, bounds.max.x),
            (48, bounds.max.y),
            (52, bounds.max.z),
            (56, center.x),
            (60, center.y),
            (64, center.z),
            (68, radius),
        ] {
            assert!(
                (rdf32(&bytes, offset) - expected).abs() < 0.001,
                "offset {offset}"
            );
        }
        assert_eq!(rd16(&bytes, 72), 0);
        assert_eq!(rd16(&bytes, 74), 0);
        assert_eq!(rd16(&bytes, 76), 1);
    }

    #[test]
    fn serializer_clears_face_group_flag_when_groups_are_not_written() {
        let mut template = test_col_template(b"COL3");
        template[80..84].copy_from_slice(&10u32.to_le_bytes());
        let entry = ImgEntry {
            img_path: PathBuf::from("test.col"),
            name: "test.col".to_string(),
            offset: 0,
            size: template.len() as u32,
        };
        let mesh = parse_col_mesh(&template, &entry).unwrap();

        let bytes = write_col_mesh_from_template(&template, &mesh).unwrap();

        assert_eq!(rd32(&bytes, 80), 2);
    }

    fn grid_mesh(size: usize) -> CollisionMesh {
        let mut vertices = Vec::new();
        for y in 0..=size {
            for x in 0..=size {
                vertices.push(V3 {
                    x: x as f32,
                    y: y as f32,
                    z: 0.0,
                });
            }
        }
        let mut faces = Vec::new();
        for y in 0..size {
            for x in 0..size {
                let row = size + 1;
                let a = (y * row + x) as u16;
                let b = (y * row + x + 1) as u16;
                let c = ((y + 1) * row + x + 1) as u16;
                let d = ((y + 1) * row + x) as u16;
                for [a, b, c] in [[a, b, c], [a, c, d]] {
                    faces.push(CollisionFace {
                        a,
                        b,
                        c,
                        material: ((x + y) % 3) as u8,
                        light: 4,
                        img_path: PathBuf::from("grid.col"),
                        material_file_offset: 0,
                        light_file_offset: 0,
                    });
                }
            }
        }
        let bounds = collision_mesh_bounds(&vertices, &[], &[]);
        CollisionMesh {
            name: "grid".to_string(),
            spheres: Vec::new(),
            boxes: Vec::new(),
            vertices,
            faces,
            bounds,
            shadow_vertices: Vec::new(),
            shadow_faces: Vec::new(),
        }
    }

    #[test]
    fn serializer_generates_valid_deterministic_face_groups() {
        let template = test_col_template(b"COL3");
        let mesh = grid_mesh(8);
        let entry = ImgEntry {
            img_path: PathBuf::from("grid.col"),
            name: "grid.col".to_string(),
            offset: 0,
            size: template.len() as u32,
        };

        assert!(col_spatial_face_groups_need_rebuild(&template, &mesh));
        let stats = col_spatial_group_stats(&mesh);
        assert_eq!(stats.groups_generated, 4);
        assert_eq!(stats.faces_grouped, 128);
        assert!(stats.faces_reordered > 0);
        let first = write_col_mesh_from_template(&template, &mesh).unwrap();
        let issues = validate_col_for_game_load("grid.col", &first);
        assert!(
            !col_validation_has_errors(&issues),
            "{}",
            issues
                .iter()
                .map(|issue| issue.label("grid.col"))
                .collect::<Vec<_>>()
                .join("\n")
        );
        assert_ne!(rd32(&first, 80) & 8, 0);
        let faces_offset = rd32(&first, 100) as usize;
        assert_eq!(rd32(&first, faces_offset), 4);

        let reparsed = parse_col_mesh(&first, &entry).unwrap();
        assert_eq!(reparsed.faces.len(), mesh.faces.len());
        assert!(!col_spatial_face_groups_need_rebuild(&first, &reparsed));
        let second = write_col_mesh_from_template(&first, &reparsed).unwrap();
        assert_eq!(second, first);
    }

    #[test]
    fn serializer_preserves_col3_shadow_mesh() {
        let mut template = test_col_template(b"COL3");
        let shadow_vertices_abs = template.len();
        for value in [0i16, 0, 0, 128, 0, 0, 0, 128, 0] {
            template.extend_from_slice(&value.to_le_bytes());
        }
        let shadow_faces_abs = template.len();
        template.extend_from_slice(&0u16.to_le_bytes());
        template.extend_from_slice(&1u16.to_le_bytes());
        template.extend_from_slice(&2u16.to_le_bytes());
        template.extend_from_slice(&[7, 8]);
        let flags = rd32(&template, 80) | 16;
        template[80..84].copy_from_slice(&flags.to_le_bytes());
        template[108..112].copy_from_slice(&1u32.to_le_bytes());
        template[112..116].copy_from_slice(&((shadow_vertices_abs - 4) as u32).to_le_bytes());
        template[116..120].copy_from_slice(&((shadow_faces_abs - 4) as u32).to_le_bytes());
        let file_size = (template.len() - 8) as u32;
        template[4..8].copy_from_slice(&file_size.to_le_bytes());
        let entry = ImgEntry {
            img_path: PathBuf::from("test.col"),
            name: "test.col".to_string(),
            offset: 0,
            size: template.len() as u32,
        };
        let mesh = parse_col_mesh(&template, &entry).unwrap();
        assert_eq!(mesh.shadow_faces.len(), 1);
        assert_eq!(mesh.shadow_vertices.len(), 3);

        let bytes = write_col_mesh_from_template(&template, &mesh).unwrap();
        let reparsed = parse_col_mesh(&bytes, &entry).unwrap();

        assert_eq!(reparsed.shadow_faces.len(), 1);
        assert_eq!(reparsed.shadow_vertices.len(), 3);
        assert_ne!(rd32(&bytes, 80) & 16, 0);
        let shadow_faces_abs = rd32(&bytes, 116) as usize + 4;
        assert_eq!(
            &bytes[shadow_faces_abs..shadow_faces_abs + 8],
            &[0, 0, 1, 0, 2, 0, 7, 8]
        );
        let issues = validate_col_for_game_load("test.col", &bytes);
        assert!(!col_validation_has_errors(&issues));
    }

    #[test]
    fn serializer_writes_edited_shadow_mesh_and_upgrades_col2() {
        let template = test_col_template(b"COL2");
        let mut mesh = grid_mesh(1);
        mesh.shadow_vertices = vec![
            V3 {
                x: -1.0,
                y: -2.0,
                z: -3.0,
            },
            V3 {
                x: 2.0,
                y: -2.0,
                z: -3.0,
            },
            V3 {
                x: -1.0,
                y: 4.0,
                z: -3.0,
            },
        ];
        mesh.shadow_faces = vec![CollisionFace {
            a: 0,
            b: 1,
            c: 2,
            material: 11,
            light: 22,
            img_path: PathBuf::new(),
            material_file_offset: 0,
            light_file_offset: 0,
        }];

        let bytes = write_col_mesh_from_template(&template, &mesh).unwrap();
        assert_eq!(&bytes[0..4], b"COL3");
        assert_ne!(rd32(&bytes, 80) & 16, 0);
        let entry = ImgEntry {
            img_path: PathBuf::from("grid.col"),
            name: "grid.col".to_string(),
            offset: 0,
            size: bytes.len() as u32,
        };
        let reparsed = parse_col_mesh(&bytes, &entry).unwrap();
        assert_eq!(reparsed.shadow_vertices, mesh.shadow_vertices);
        assert_eq!(reparsed.shadow_faces.len(), 1);
        assert_eq!(reparsed.shadow_faces[0].material, 11);
        assert_eq!(reparsed.shadow_faces[0].light, 22);
        assert!(!col_validation_has_errors(&validate_col_for_game_load(
            "grid.col", &bytes
        )));
    }

    #[test]
    fn set_col_model_names_from_entry_renames_coll_models() {
        let mut bytes = vec![0u8; 40];
        bytes[0..4].copy_from_slice(b"COLL");
        bytes[4..8].copy_from_slice(&(32u32).to_le_bytes());
        bytes[8..14].copy_from_slice(b"source");

        let renamed = set_col_model_names_from_entry(&mut bytes, "clnm_stadium.col");

        assert_eq!(renamed, 1);
        let name = String::from_utf8_lossy(&bytes[8..20]);
        assert!(name.starts_with("clnm_stadium"));
    }

    #[test]
    fn col2_mesh_serializer_round_trips_parser_subset() {
        assert_col_mesh_serializer_round_trips_parser_subset(b"COL2");
    }

    #[test]
    fn col3_mesh_serializer_round_trips_parser_subset() {
        assert_col_mesh_serializer_round_trips_parser_subset(b"COL3");
    }
}
