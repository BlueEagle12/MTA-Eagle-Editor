use super::super::*;
use crate::dff::export::dff_chunk_len;

pub(crate) fn col_chunk_len(bytes: &[u8]) -> usize {
    let mut model_start = 0usize;
    let mut last_valid = 0usize;
    while model_start + 8 <= bytes.len() {
        let magic = &bytes[model_start..model_start + 4];
        if !matches!(magic, b"COLL" | b"COL2" | b"COL3" | b"COL4") {
            break;
        }
        let file_size = rd32(bytes, model_start + 4) as usize;
        let model_end = model_start.saturating_add(file_size).saturating_add(8);
        if model_end <= model_start + 8 || model_end > bytes.len() {
            break;
        }
        last_valid = model_end;
        model_start = model_end;
    }
    if last_valid == 0 {
        bytes.len()
    } else {
        last_valid
    }
}

pub(crate) fn replacement_entry_len(name: &str, bytes: &[u8]) -> usize {
    let key = lower(name);
    if key.ends_with(".dff") {
        dff_chunk_len(bytes)
    } else if key.ends_with(".txd") {
        txd_chunk_len(bytes)
    } else if key.ends_with(".col") {
        col_chunk_len(bytes)
    } else {
        bytes.len()
    }
}

fn parse_col_name(bytes: &[u8], start: usize) -> String {
    let name_start = start + 8;
    let name_end = (name_start + 22).min(bytes.len());
    let raw = &bytes[name_start..name_end];
    let end = raw
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(raw.len());
    String::from_utf8_lossy(&raw[..end]).to_string()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ColModelIdentity {
    pub(crate) index: usize,
    pub(crate) name: String,
    pub(crate) magic: [u8; 4],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ColModelRange {
    pub(crate) identity: ColModelIdentity,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

/// Enumerate the complete COL models packed into an entry.
///
/// IMG sector padding after the final model is deliberately ignored. A
/// truncated or malformed model terminates the valid prefix, matching the
/// parser's existing behavior.
pub(crate) fn col_model_ranges(bytes: &[u8]) -> Vec<ColModelRange> {
    let mut ranges = Vec::new();
    let mut model_start = 0usize;
    while model_start + 8 <= bytes.len() {
        let magic: [u8; 4] = bytes[model_start..model_start + 4]
            .try_into()
            .expect("four-byte COL magic slice");
        if !matches!(&magic, b"COLL" | b"COL2" | b"COL3" | b"COL4") {
            break;
        }
        let file_size = rd32(bytes, model_start + 4) as usize;
        let Some(model_end) = model_start
            .checked_add(file_size)
            .and_then(|end| end.checked_add(8))
        else {
            break;
        };
        if model_end <= model_start + 8 || model_end > bytes.len() || model_start + 30 > model_end {
            break;
        }
        ranges.push(ColModelRange {
            identity: ColModelIdentity {
                index: ranges.len(),
                name: parse_col_name(bytes, model_start),
                magic,
            },
            start: model_start,
            end: model_end,
        });
        model_start = model_end;
    }
    ranges
}

fn col_abs(model_start: usize, offset: u32) -> usize {
    model_start + offset as usize + 4
}

fn read_col_surface(bytes: &[u8], o: usize) -> CollisionSurface {
    CollisionSurface {
        material: bytes.get(o).copied().unwrap_or(0),
        flags: bytes.get(o + 1).copied().unwrap_or(0),
        brightness: bytes.get(o + 2).copied().unwrap_or(0),
        light: bytes.get(o + 3).copied().unwrap_or(0),
    }
}

fn finite_ordered_bounds(min: Vec3, max: Vec3) -> Option<Bounds> {
    [min.x, min.y, min.z, max.x, max.y, max.z]
        .into_iter()
        .all(f32::is_finite)
        .then_some(())?;
    (min.x <= max.x && min.y <= max.y && min.z <= max.z).then_some(Bounds { min, max })
}

fn col_model_header_bounds(bytes: &[u8], model_start: usize, magic: &[u8]) -> Option<Bounds> {
    if model_start + 72 > bytes.len() {
        return None;
    }
    let (min_offset, max_offset) = if magic == b"COLL" {
        // COL1 TBounds: radius, center, min, max.
        (48usize, 60usize)
    } else {
        // COL2/3/4 TBounds: min, max, center, radius.
        (32usize, 44usize)
    };
    let read_v3 = |offset: usize| {
        Vec3::new(
            rdf32(bytes, model_start + offset),
            rdf32(bytes, model_start + offset + 4),
            rdf32(bytes, model_start + offset + 8),
        )
    };
    finite_ordered_bounds(read_v3(min_offset), read_v3(max_offset))
}

pub(crate) fn collect_col_model_names(bytes: &[u8]) -> Vec<String> {
    let mut names = Vec::new();
    let mut model_start = 0usize;
    while model_start + 8 <= bytes.len() {
        let magic = &bytes[model_start..model_start + 4];
        if !matches!(magic, b"COLL" | b"COL2" | b"COL3" | b"COL4") {
            break;
        }
        let file_size = rd32(bytes, model_start + 4) as usize;
        let model_end = model_start.saturating_add(file_size).saturating_add(8);
        if model_end <= model_start + 8 || model_start + 30 > bytes.len() {
            break;
        }
        let name = parse_col_name(bytes, model_start);
        if !name.trim().is_empty() {
            names.push(asset_key(&name, ".col"));
        }
        model_start = model_end;
    }
    names
}

pub(crate) fn parse_col_mesh(bytes: &[u8], entry: &ImgEntry) -> Option<CollisionMesh> {
    parse_col_mesh_matching(bytes, entry, None)
}

/// Parse the first usable model and retain the exact packed-model identity
/// required to replace it safely later.
pub(crate) fn parse_col_mesh_with_identity(
    bytes: &[u8],
    entry: &ImgEntry,
) -> Option<(CollisionMesh, ColModelIdentity)> {
    for range in col_model_ranges(bytes) {
        let mut model_entry = entry.clone();
        model_entry.offset = model_entry
            .offset
            .saturating_add(range.start.min(u32::MAX as usize) as u32);
        model_entry.size = (range.end - range.start).min(u32::MAX as usize) as u32;
        if let Some(mesh) = parse_col_mesh(&bytes[range.start..range.end], &model_entry) {
            return Some((mesh, range.identity));
        }
    }
    None
}

pub(crate) fn parse_col_mesh_for_identity(
    bytes: &[u8],
    entry: &ImgEntry,
    identity: &ColModelIdentity,
) -> Option<CollisionMesh> {
    let range = col_model_ranges(bytes).into_iter().nth(identity.index)?;
    if range.identity.name != identity.name || range.identity.magic != identity.magic {
        return None;
    }
    let mut model_entry = entry.clone();
    model_entry.offset = model_entry
        .offset
        .saturating_add(range.start.min(u32::MAX as usize) as u32);
    model_entry.size = (range.end - range.start).min(u32::MAX as usize) as u32;
    parse_col_mesh(&bytes[range.start..range.end], &model_entry)
}

pub(crate) fn parse_col_mesh_matching(
    bytes: &[u8],
    entry: &ImgEntry,
    target_name: Option<String>,
) -> Option<CollisionMesh> {
    let mut model_start = 0usize;
    while model_start + 8 <= bytes.len() {
        let magic = &bytes[model_start..model_start + 4];
        if matches!(magic, b"COLL") {
            // COL version 1 (e.g. Blender/MTA v1 exports). Different on-disk
            // layout from COL2/3/4: no offset table, float vertices, 4-byte
            // face surfaces. Parse it into the shared CollisionMesh so it can
            // be previewed and edited; edits are upgraded to COL2 on save.
            let file_size = rd32(bytes, model_start + 4) as usize;
            let model_end = (model_start + file_size + 8).min(bytes.len());
            if model_end <= model_start + 8 {
                break;
            }
            let name = parse_col_name(bytes, model_start);
            let matches_target = target_name
                .as_ref()
                .is_none_or(|target| asset_key(&name, ".col") == *target);
            if matches_target {
                if let Some(mesh) = parse_col1_mesh_model(bytes, model_start, model_end, entry) {
                    return Some(mesh);
                }
            }
            model_start += file_size + 8;
            continue;
        }
        if !matches!(magic, b"COL2" | b"COL3" | b"COL4") {
            break;
        }
        if model_start + 116 > bytes.len() {
            break;
        }
        let file_size = rd32(bytes, model_start + 4) as usize;
        let model_end = (model_start + file_size + 8).min(bytes.len());
        // A bounds-only COL2 is exactly the 116-byte header. There is no
        // geometry payload to require beyond it, so accept that boundary.
        if model_end < model_start + 116 {
            break;
        }
        let name = parse_col_name(bytes, model_start);
        if target_name
            .as_ref()
            .is_some_and(|target| asset_key(&name, ".col") != *target)
        {
            model_start += file_size + 8;
            continue;
        }
        let header = model_start + 72;
        let sphere_count = rd16(bytes, header) as usize;
        let box_count = rd16(bytes, header + 2) as usize;
        let face_count = rd16(bytes, header + 4) as usize;
        let flags = rd32(bytes, header + 8);
        let spheres_offset = rd32(bytes, header + 12);
        let boxes_offset = rd32(bytes, header + 16);
        let vertices_offset = rd32(bytes, header + 24);
        let faces_offset = rd32(bytes, header + 28);
        let mut spheres = Vec::with_capacity(sphere_count);
        if sphere_count > 0 {
            let spheres_abs = col_abs(model_start, spheres_offset);
            if spheres_offset == 0 || spheres_abs + sphere_count * 20 > model_end {
                model_start += file_size + 8;
                continue;
            }
            for idx in 0..sphere_count {
                let o = spheres_abs + idx * 20;
                spheres.push(CollisionSphere {
                    center: V3 {
                        x: rdf32(bytes, o),
                        y: rdf32(bytes, o + 4),
                        z: rdf32(bytes, o + 8),
                    },
                    radius: rdf32(bytes, o + 12).abs(),
                    surface: read_col_surface(bytes, o + 16),
                });
            }
        }

        let mut boxes = Vec::with_capacity(box_count);
        if box_count > 0 {
            let boxes_abs = col_abs(model_start, boxes_offset);
            if boxes_offset == 0 || boxes_abs + box_count * 28 > model_end {
                model_start += file_size + 8;
                continue;
            }
            for idx in 0..box_count {
                let o = boxes_abs + idx * 28;
                boxes.push(CollisionBox {
                    min: V3 {
                        x: rdf32(bytes, o),
                        y: rdf32(bytes, o + 4),
                        z: rdf32(bytes, o + 8),
                    },
                    max: V3 {
                        x: rdf32(bytes, o + 12),
                        y: rdf32(bytes, o + 16),
                        z: rdf32(bytes, o + 20),
                    },
                    surface: read_col_surface(bytes, o + 24),
                });
            }
        }

        let vertices_abs = col_abs(model_start, vertices_offset);
        let faces_abs = col_abs(model_start, faces_offset);
        if face_count > 0
            && (vertices_offset == 0
                || faces_offset == 0
                || faces_abs + face_count * 8 > model_end
                || vertices_abs >= faces_abs)
        {
            model_start += file_size + 8;
            continue;
        }

        let mut faces = Vec::with_capacity(face_count);
        let mut highest_vertex = 0usize;
        for face_idx in 0..face_count {
            let o = faces_abs + face_idx * 8;
            let a = rd16(bytes, o);
            let b = rd16(bytes, o + 2);
            let c = rd16(bytes, o + 4);
            highest_vertex = highest_vertex
                .max(a as usize)
                .max(b as usize)
                .max(c as usize);
            faces.push(CollisionFace {
                a,
                b,
                c,
                material: bytes.get(o + 6).copied().unwrap_or(0),
                light: bytes.get(o + 7).copied().unwrap_or(0),
                img_path: entry.img_path.clone(),
                material_file_offset: entry.offset as u64 + o as u64 + 6,
                light_file_offset: entry.offset as u64 + o as u64 + 7,
            });
        }

        let mut vertices = Vec::new();
        if face_count > 0 {
            let max_vertex_count = (faces_abs - vertices_abs) / 6;
            let vertex_count = (highest_vertex + 1).min(max_vertex_count);
            if vertex_count == 0 || vertex_count > 1_000_000 {
                model_start += file_size + 8;
                continue;
            }
            vertices.reserve(vertex_count);
            for idx in 0..vertex_count {
                let o = vertices_abs + idx * 6;
                vertices.push(V3 {
                    x: rdi16(bytes, o) as f32 / 128.0,
                    y: rdi16(bytes, o + 2) as f32 / 128.0,
                    z: rdi16(bytes, o + 4) as f32 / 128.0,
                });
            }
        }

        let shadow_face_count = if matches!(magic, b"COL3" | b"COL4") {
            // COL3/COL4 extend the common header with shadow face count,
            // shadow vertex offset, and shadow face offset at +36/+40/+44.
            rd32(bytes, header + 36) as usize
        } else {
            0
        };
        let mut shadow_faces = Vec::new();
        let mut shadow_vertices = Vec::new();
        if flags & 16 != 0 && shadow_face_count > 0 {
            let shadow_vertices_abs = col_abs(model_start, rd32(bytes, header + 40));
            let shadow_faces_abs = col_abs(model_start, rd32(bytes, header + 44));
            let shadow_faces_end = shadow_faces_abs.checked_add(shadow_face_count * 8);
            if shadow_vertices_abs < shadow_faces_abs
                && shadow_faces_end.is_some_and(|end| end <= model_end && end <= bytes.len())
            {
                let mut highest_shadow_vertex = 0usize;
                shadow_faces.reserve(shadow_face_count);
                for face_idx in 0..shadow_face_count {
                    let o = shadow_faces_abs + face_idx * 8;
                    let a = rd16(bytes, o);
                    let b = rd16(bytes, o + 2);
                    let c = rd16(bytes, o + 4);
                    highest_shadow_vertex = highest_shadow_vertex
                        .max(a as usize)
                        .max(b as usize)
                        .max(c as usize);
                    shadow_faces.push(CollisionFace {
                        a,
                        b,
                        c,
                        material: bytes.get(o + 6).copied().unwrap_or(0),
                        light: bytes.get(o + 7).copied().unwrap_or(0),
                        img_path: entry.img_path.clone(),
                        material_file_offset: entry.offset as u64 + o as u64 + 6,
                        light_file_offset: entry.offset as u64 + o as u64 + 7,
                    });
                }
                let shadow_vertex_count = highest_shadow_vertex + 1;
                let shadow_vertices_end = shadow_vertices_abs.checked_add(shadow_vertex_count * 6);
                if shadow_vertices_end.is_some_and(|end| end <= shadow_faces_abs) {
                    shadow_vertices.reserve(shadow_vertex_count);
                    for idx in 0..shadow_vertex_count {
                        let o = shadow_vertices_abs + idx * 6;
                        shadow_vertices.push(V3 {
                            x: rdi16(bytes, o) as f32 / 128.0,
                            y: rdi16(bytes, o + 2) as f32 / 128.0,
                            z: rdi16(bytes, o + 4) as f32 / 128.0,
                        });
                    }
                } else {
                    shadow_faces.clear();
                }
            }
        }
        let bounds = if vertices.is_empty() && spheres.is_empty() && boxes.is_empty() {
            // Bounds-only models are deliberately emitted for LODs and for
            // definitions whose collision geometry is disabled. They are
            // valid COL models even though there is nothing to draw yet.
            col_model_header_bounds(bytes, model_start, magic)?
        } else {
            collision_mesh_bounds(&vertices, &spheres, &boxes)
        };
        return Some(CollisionMesh {
            name,
            spheres,
            boxes,
            vertices,
            faces,
            bounds,
            shadow_vertices,
            shadow_faces,
        });
    }
    None
}

/// Parse a single COL version 1 ("COLL") model into a `CollisionMesh`.
///
/// Layout (little-endian), relative to `model_start`:
/// ```text
///   +0   char[4]  "COLL"
///   +4   u32      file_size (bytes after this field)
///   +8   char[22] name
///   +30  u16      model_id
///   +32  TBounds  radius(f32), center(3f), min(3f), max(3f)   // 40 bytes
///   +72  u32      num_spheres; TSphere[20]  radius(f32), center(3f), surface(4)
///        u32      num_lines (unused, always 0)
///        u32      num_boxes;   TBox[28]     min(3f), max(3f), surface(4)
///        u32      num_vertices;TVertex[12]  x,y,z (f32)
///        u32      num_faces;   TFace[16]    a,b,c (u32), surface(4)
/// ```
/// Surface bytes are material, flag, brightness, light; only material and
/// light are kept to match the COL2/3/4 face model.
fn parse_col1_mesh_model(
    bytes: &[u8],
    model_start: usize,
    model_end: usize,
    entry: &ImgEntry,
) -> Option<CollisionMesh> {
    let name = parse_col_name(bytes, model_start);
    // Body begins after the 32-byte header and the 40-byte v1 TBounds.
    let mut o = model_start + 72;

    let read_count = |offset: usize| -> Option<usize> {
        if offset + 4 > model_end {
            None
        } else {
            Some(rd32(bytes, offset) as usize)
        }
    };

    // Spheres.
    let num_spheres = read_count(o)?;
    o += 4;
    if o + num_spheres.checked_mul(20)? > model_end {
        return None;
    }
    let mut spheres = Vec::with_capacity(num_spheres);
    for _ in 0..num_spheres {
        spheres.push(CollisionSphere {
            center: V3 {
                x: rdf32(bytes, o + 4),
                y: rdf32(bytes, o + 8),
                z: rdf32(bytes, o + 12),
            },
            radius: rdf32(bytes, o).abs(),
            surface: read_col_surface(bytes, o + 16),
        });
        o += 20;
    }

    // Lines/unknown count (always 0 in practice); skip.
    let _num_lines = read_count(o)?;
    o += 4;

    // Boxes.
    let num_boxes = read_count(o)?;
    o += 4;
    if o + num_boxes.checked_mul(28)? > model_end {
        return None;
    }
    let mut boxes = Vec::with_capacity(num_boxes);
    for _ in 0..num_boxes {
        boxes.push(CollisionBox {
            min: V3 {
                x: rdf32(bytes, o),
                y: rdf32(bytes, o + 4),
                z: rdf32(bytes, o + 8),
            },
            max: V3 {
                x: rdf32(bytes, o + 12),
                y: rdf32(bytes, o + 16),
                z: rdf32(bytes, o + 20),
            },
            surface: read_col_surface(bytes, o + 24),
        });
        o += 28;
    }

    // Vertices (float triplets). CollisionFace stores u16 indices, so a mesh
    // with more than u16::MAX vertices cannot be represented here.
    let num_vertices = read_count(o)?;
    o += 4;
    if num_vertices > u16::MAX as usize || o + num_vertices.checked_mul(12)? > model_end {
        return None;
    }
    let mut vertices = Vec::with_capacity(num_vertices);
    for _ in 0..num_vertices {
        vertices.push(V3 {
            x: rdf32(bytes, o),
            y: rdf32(bytes, o + 4),
            z: rdf32(bytes, o + 8),
        });
        o += 12;
    }

    // Faces.
    let num_faces = read_count(o)?;
    o += 4;
    if o + num_faces.checked_mul(16)? > model_end {
        return None;
    }
    let faces_abs = o;
    let mut faces = Vec::with_capacity(num_faces);
    for face_idx in 0..num_faces {
        let fo = faces_abs + face_idx * 16;
        let a = rd32(bytes, fo) as usize;
        let b = rd32(bytes, fo + 4) as usize;
        let c = rd32(bytes, fo + 8) as usize;
        if a > u16::MAX as usize || b > u16::MAX as usize || c > u16::MAX as usize {
            return None;
        }
        faces.push(CollisionFace {
            a: a as u16,
            b: b as u16,
            c: c as u16,
            material: bytes.get(fo + 12).copied().unwrap_or(0),
            light: bytes.get(fo + 15).copied().unwrap_or(0),
            img_path: entry.img_path.clone(),
            material_file_offset: entry.offset as u64 + fo as u64 + 12,
            light_file_offset: entry.offset as u64 + fo as u64 + 15,
        });
    }

    let bounds = if vertices.is_empty() && spheres.is_empty() && boxes.is_empty() {
        col_model_header_bounds(bytes, model_start, b"COLL")?
    } else {
        collision_mesh_bounds(&vertices, &spheres, &boxes)
    };
    Some(CollisionMesh {
        name,
        spheres,
        boxes,
        vertices,
        faces,
        bounds,
        shadow_vertices: Vec::new(),
        shadow_faces: Vec::new(),
    })
}

#[cfg(test)]
mod col1_tests {
    use crate::*;

    /// Build a minimal COL version 1 ("COLL") model: three vertices, one face,
    /// no spheres/boxes/lines. Mirrors the layout observed in real Blender v1
    /// exports (radius, center, min, max bounds; float verts; u32 face indices
    /// plus a 4-byte surface).
    fn make_col1(name: &str, verts: &[[f32; 3]], faces: &[[u32; 3]]) -> Vec<u8> {
        let mut body = Vec::new();
        // name (22 bytes) + model id (2 bytes).
        let mut name_field = [0u8; 22];
        let raw = name.as_bytes();
        let n = raw.len().min(22);
        name_field[..n].copy_from_slice(&raw[..n]);
        body.extend_from_slice(&name_field);
        body.extend_from_slice(&0u16.to_le_bytes());
        // TBounds v1: radius, center(3), min(3), max(3).
        for value in [1.0f32, 0.0, 0.0, 0.0, -1.0, -1.0, -1.0, 1.0, 1.0, 1.0] {
            body.extend_from_slice(&value.to_le_bytes());
        }
        // num_spheres, num_lines, num_boxes = 0.
        body.extend_from_slice(&0u32.to_le_bytes());
        body.extend_from_slice(&0u32.to_le_bytes());
        body.extend_from_slice(&0u32.to_le_bytes());
        // vertices.
        body.extend_from_slice(&(verts.len() as u32).to_le_bytes());
        for v in verts {
            for c in v {
                body.extend_from_slice(&c.to_le_bytes());
            }
        }
        // faces (a,b,c u32 + material, flag, brightness, light).
        body.extend_from_slice(&(faces.len() as u32).to_le_bytes());
        for f in faces {
            for idx in f {
                body.extend_from_slice(&idx.to_le_bytes());
            }
            body.extend_from_slice(&[7u8, 0, 0, 3]); // material 7, light 3
        }

        let mut out = Vec::new();
        out.extend_from_slice(b"COLL");
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(&body);
        out
    }

    fn entry() -> ImgEntry {
        ImgEntry {
            img_path: std::path::PathBuf::from("test.col"),
            name: "test.col".to_string(),
            offset: 0,
            size: 0,
        }
    }

    #[test]
    fn parses_col_version_1_model() {
        let verts = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let faces = [[0u32, 1, 2]];
        let bytes = make_col1("dam_wall", &verts, &faces);
        let mesh = parse_col_mesh(&bytes, &entry()).expect("v1 COL should parse");
        assert_eq!(mesh.vertices.len(), 3);
        assert_eq!(mesh.faces.len(), 1);
        assert_eq!(mesh.faces[0].material, 7);
        assert_eq!(mesh.faces[0].light, 3);
        assert!((mesh.vertices[1].x - 1.0).abs() < 1e-6);
        assert!((mesh.vertices[2].y - 1.0).abs() < 1e-6);
    }

    #[test]
    fn version_1_upgrades_to_col2_on_write() {
        let verts = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let faces = [[0u32, 1, 2]];
        let bytes = make_col1("dam_wall", &verts, &faces);
        let mesh = parse_col_mesh(&bytes, &entry()).expect("v1 COL should parse");

        // The raw v1 bytes are not a valid serializer template, so the write
        // path must substitute a COL2 template.
        let template = col_write_template(&bytes, "dam_wall.col");
        assert_eq!(&template[0..4], b"COL2");

        let written = write_col_mesh_from_template(&template, &mesh)
            .expect("mesh should serialize into COL2");
        assert_eq!(&written[0..4], b"COL2");

        let reparsed = parse_col_mesh(&written, &entry()).expect("COL2 output should parse");
        assert_eq!(reparsed.vertices.len(), 3);
        assert_eq!(reparsed.faces.len(), 1);
    }
}

#[cfg(test)]
mod bounds_only_tests {
    use crate::*;

    #[test]
    fn bounds_only_col_opens_and_round_trips_without_losing_its_volume() {
        let bounds = Bounds {
            min: Vec3::new(-12.0, -4.0, -1.5),
            max: Vec3::new(18.0, 9.0, 7.25),
        };
        let mesh = CollisionMesh {
            name: "lod_bounds".to_string(),
            spheres: Vec::new(),
            boxes: Vec::new(),
            vertices: Vec::new(),
            faces: Vec::new(),
            bounds,
            shadow_vertices: Vec::new(),
            shadow_faces: Vec::new(),
        };
        let template = col_regeneration_template(&[], "lod_bounds.col");
        let bytes = write_col_mesh_from_template(&template, &mesh).unwrap();
        let entry = ImgEntry {
            img_path: PathBuf::from("lod_bounds.col"),
            name: "lod_bounds.col".to_string(),
            offset: 0,
            size: bytes.len() as u32,
        };

        let (parsed, identity) = parse_col_mesh_with_identity(&bytes, &entry).unwrap();

        assert!(parsed.vertices.is_empty());
        assert!(parsed.faces.is_empty());
        assert!(parsed.spheres.is_empty());
        assert!(parsed.boxes.is_empty());
        assert_eq!(parsed.bounds.min, bounds.min);
        assert_eq!(parsed.bounds.max, bounds.max);

        let rewritten = write_col_mesh_replacing_model(&bytes, &identity, &parsed).unwrap();
        let reparsed = parse_col_mesh_for_identity(&rewritten, &entry, &identity).unwrap();
        assert_eq!(reparsed.bounds.min, bounds.min);
        assert_eq!(reparsed.bounds.max, bounds.max);
        assert!(!col_validation_has_errors(&validate_col_for_game_load(
            &entry.name,
            &rewritten,
        )));
    }
}
