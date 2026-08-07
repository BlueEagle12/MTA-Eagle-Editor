use super::super::*;

pub(crate) const BREAKABLE_PLUGIN_ID: u32 = 0x0253_f2fd;
const BREAKABLE_INFO_SIZE: usize = 0x34;
const BREAKABLE_PREFIX_SIZE: usize = 4 + BREAKABLE_INFO_SIZE;
const FIXED_TEXTURE_NAME: usize = 32;

fn checked_payload_size(vertices: usize, triangles: usize, groups: usize) -> Option<usize> {
    BREAKABLE_PREFIX_SIZE
        .checked_add(vertices.checked_mul(24)?)
        .and_then(|size| size.checked_add(triangles.checked_mul(8)?))
        .and_then(|size| size.checked_add(groups.checked_mul(76)?))
}

fn fixed_name(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).trim().to_string()
}

fn push_fixed_name(out: &mut Vec<u8>, value: &str) -> Result<(), String> {
    let bytes = value.trim().as_bytes();
    if bytes.len() >= FIXED_TEXTURE_NAME {
        return Err(format!(
            "breakable texture or mask '{}' is {} bytes; the SA limit is 31",
            value,
            bytes.len()
        ));
    }
    let mut fixed = [0u8; FIXED_TEXTURE_NAME];
    fixed[..bytes.len()].copy_from_slice(bytes);
    out.extend_from_slice(&fixed);
    Ok(())
}

pub(crate) fn parse_breakable_plugin(payload: &[u8]) -> Result<Option<BreakableGeometry>, String> {
    if payload.len() < 4 {
        return Err("breakable plug-in is shorter than its section marker".to_string());
    }
    if rd32(payload, 0) == 0 {
        return Ok(None);
    }
    if payload.len() < BREAKABLE_PREFIX_SIZE {
        return Err(format!(
            "breakable plug-in is truncated ({} bytes, needs at least {})",
            payload.len(),
            BREAKABLE_PREFIX_SIZE
        ));
    }
    let origin = match rd32(payload, 4) {
        0 => BreakableOrigin::Object,
        1 => BreakableOrigin::Collision,
        value => return Err(format!("unsupported breakable origin rule {value}")),
    };
    let vertex_count = rd16(payload, 8) as usize;
    let triangle_count = rd16(payload, 24) as usize;
    let group_count = rd16(payload, 36) as usize;
    let expected = checked_payload_size(vertex_count, triangle_count, group_count)
        .ok_or_else(|| "breakable plug-in counts overflow its payload size".to_string())?;
    if payload.len() < expected {
        return Err(format!(
            "breakable plug-in is truncated ({} bytes, counts require {expected})",
            payload.len()
        ));
    }

    let mut cursor = BREAKABLE_PREFIX_SIZE;
    let mut positions = Vec::with_capacity(vertex_count);
    for _ in 0..vertex_count {
        positions.push(V3 {
            x: rdf32(payload, cursor),
            y: rdf32(payload, cursor + 4),
            z: rdf32(payload, cursor + 8),
        });
        cursor += 12;
    }
    let mut uvs = Vec::with_capacity(vertex_count);
    for _ in 0..vertex_count {
        uvs.push(V2 {
            u: rdf32(payload, cursor),
            v: rdf32(payload, cursor + 4),
        });
        cursor += 8;
    }
    let mut colors = Vec::with_capacity(vertex_count);
    for _ in 0..vertex_count {
        colors.push([
            payload[cursor],
            payload[cursor + 1],
            payload[cursor + 2],
            payload[cursor + 3],
        ]);
        cursor += 4;
    }
    let vertices = (0..vertex_count)
        .map(|idx| BreakableVertex {
            position: positions[idx],
            uv: uvs[idx],
            color: colors[idx],
        })
        .collect::<Vec<_>>();

    let mut triangle_vertices = Vec::with_capacity(triangle_count);
    for _ in 0..triangle_count {
        triangle_vertices.push([
            rd16(payload, cursor),
            rd16(payload, cursor + 2),
            rd16(payload, cursor + 4),
        ]);
        cursor += 6;
    }
    let mut triangles = Vec::with_capacity(triangle_count);
    for vertices in triangle_vertices {
        let group = rd16(payload, cursor);
        cursor += 2;
        if vertices.iter().any(|index| *index as usize >= vertex_count) {
            return Err("breakable triangle references a missing vertex".to_string());
        }
        if group as usize >= group_count {
            return Err("breakable triangle references a missing fracture group".to_string());
        }
        triangles.push(BreakableTriangle {
            vertices,
            group,
            source_face: None,
        });
    }

    let mut textures = Vec::with_capacity(group_count);
    for _ in 0..group_count {
        textures.push(fixed_name(&payload[cursor..cursor + FIXED_TEXTURE_NAME]));
        cursor += FIXED_TEXTURE_NAME;
    }
    let mut masks = Vec::with_capacity(group_count);
    for _ in 0..group_count {
        masks.push(fixed_name(&payload[cursor..cursor + FIXED_TEXTURE_NAME]));
        cursor += FIXED_TEXTURE_NAME;
    }
    let mut groups = Vec::with_capacity(group_count);
    for idx in 0..group_count {
        groups.push(BreakableGroup {
            name: format!("Zone {}", idx + 1),
            texture: textures[idx].clone(),
            mask: masks[idx].clone(),
            ambient: V3 {
                x: rdf32(payload, cursor),
                y: rdf32(payload, cursor + 4),
                z: rdf32(payload, cursor + 8),
            },
        });
        cursor += 12;
    }

    Ok(Some(BreakableGeometry {
        origin,
        vertices,
        triangles,
        groups,
        stale: false,
    }))
}

pub(crate) fn encode_breakable_plugin(breakable: &BreakableGeometry) -> Result<Vec<u8>, String> {
    let errors = validate_breakable_geometry(breakable);
    if !errors.is_empty() {
        return Err(errors.join("; "));
    }
    let vertex_count = u16::try_from(breakable.vertices.len())
        .map_err(|_| "breakable geometry has more than 65535 vertices".to_string())?;
    let triangle_count = u16::try_from(breakable.triangles.len())
        .map_err(|_| "breakable geometry has more than 65535 triangles".to_string())?;
    let group_count = u16::try_from(breakable.groups.len())
        .map_err(|_| "breakable geometry has more than 65535 groups".to_string())?;
    let capacity = checked_payload_size(
        breakable.vertices.len(),
        breakable.triangles.len(),
        breakable.groups.len(),
    )
    .ok_or_else(|| "breakable geometry is too large".to_string())?;
    let mut out = Vec::with_capacity(capacity);
    // The native loader treats any non-zero four-byte marker as "data follows".
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(
        &(match breakable.origin {
            BreakableOrigin::Object => 0u32,
            BreakableOrigin::Collision => 1u32,
        })
        .to_le_bytes(),
    );
    out.extend_from_slice(&vertex_count.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&[0u8; 12]); // streamed pointers
    out.extend_from_slice(&triangle_count.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&[0u8; 8]); // streamed pointers
    out.extend_from_slice(&group_count.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&[0u8; 16]); // streamed pointers
    debug_assert_eq!(out.len(), BREAKABLE_PREFIX_SIZE);

    for vertex in &breakable.vertices {
        for value in [vertex.position.x, vertex.position.y, vertex.position.z] {
            out.extend_from_slice(&value.to_le_bytes());
        }
    }
    for vertex in &breakable.vertices {
        out.extend_from_slice(&vertex.uv.u.to_le_bytes());
        out.extend_from_slice(&vertex.uv.v.to_le_bytes());
    }
    for vertex in &breakable.vertices {
        out.extend_from_slice(&vertex.color);
    }
    for triangle in &breakable.triangles {
        for index in triangle.vertices {
            out.extend_from_slice(&index.to_le_bytes());
        }
    }
    for triangle in &breakable.triangles {
        out.extend_from_slice(&triangle.group.to_le_bytes());
    }
    for group in &breakable.groups {
        push_fixed_name(&mut out, &group.texture)?;
    }
    for group in &breakable.groups {
        push_fixed_name(&mut out, &group.mask)?;
    }
    for group in &breakable.groups {
        for value in [group.ambient.x, group.ambient.y, group.ambient.z] {
            out.extend_from_slice(&value.to_le_bytes());
        }
    }
    debug_assert_eq!(out.len(), capacity);
    Ok(out)
}

fn position_key(position: V3) -> [i32; 3] {
    [
        (position.x * 100_000.0).round() as i32,
        (position.y * 100_000.0).round() as i32,
        (position.z * 100_000.0).round() as i32,
    ]
}

/// Best-effort mapping from imported debris triangles back to intact faces.
/// Vanilla assets normally use the same positions, but custom debris is
/// allowed to differ, in which case manual zones remain unavailable until the
/// user regenerates from the intact mesh.
pub(crate) fn map_breakable_source_faces(
    breakable: &mut BreakableGeometry,
    intact_vertices: &[V3],
    intact_triangles: &[Tri],
    global_face_start: usize,
) {
    let mut intact = BTreeMap::<[[i32; 3]; 3], Vec<usize>>::new();
    for (local_face, triangle) in intact_triangles.iter().enumerate() {
        let Some(mut key) = [
            intact_vertices.get(triangle.a as usize),
            intact_vertices.get(triangle.b as usize),
            intact_vertices.get(triangle.c as usize),
        ]
        .into_iter()
        .collect::<Option<Vec<_>>>() else {
            continue;
        };
        let mut positions = [
            position_key(*key.remove(0)),
            position_key(*key.remove(0)),
            position_key(*key.remove(0)),
        ];
        positions.sort();
        intact
            .entry(positions)
            .or_default()
            .push(global_face_start + local_face);
    }
    for triangle in &mut breakable.triangles {
        let Some(mut positions) = triangle
            .vertices
            .iter()
            .map(|index| {
                breakable
                    .vertices
                    .get(*index as usize)
                    .map(|vertex| position_key(vertex.position))
            })
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        positions.sort();
        let key = [positions[0], positions[1], positions[2]];
        triangle.source_face = intact.get_mut(&key).and_then(Vec::pop);
    }
}

fn component_bounds(raw: &RawMesh, component_index: usize) -> Result<(usize, usize), String> {
    if raw.components.is_empty() && component_index == 0 {
        return Ok((0, raw.triangles.len()));
    }
    let component = raw
        .components
        .get(component_index)
        .ok_or_else(|| "fracture component no longer exists".to_string())?;
    let start = component.tri_start.min(raw.triangles.len());
    let end = component.tri_end.min(raw.triangles.len()).max(start);
    Ok((start, end))
}

#[derive(Default)]
struct DisjointSet {
    parent: Vec<usize>,
}

impl DisjointSet {
    fn new(len: usize) -> Self {
        Self {
            parent: (0..len).collect(),
        }
    }

    fn root(&mut self, value: usize) -> usize {
        let parent = self.parent[value];
        if parent != value {
            let root = self.root(parent);
            self.parent[value] = root;
        }
        self.parent[value]
    }

    fn join(&mut self, a: usize, b: usize) {
        let a = self.root(a);
        let b = self.root(b);
        if a != b {
            self.parent[b] = a;
        }
    }
}

pub(crate) fn generate_breakable_geometry(
    raw: &RawMesh,
    component_index: usize,
) -> Result<BreakableGeometry, String> {
    let (face_start, face_end) = component_bounds(raw, component_index)?;
    if face_start == face_end {
        return Err("selected geometry has no faces".to_string());
    }
    let faces = &raw.triangles[face_start..face_end];
    let mut sets = DisjointSet::new(faces.len());
    let mut edge_owner = BTreeMap::<([i32; 3], [i32; 3], u16), usize>::new();
    for (local_face, triangle) in faces.iter().enumerate() {
        for (a, b) in [
            (triangle.a, triangle.b),
            (triangle.b, triangle.c),
            (triangle.c, triangle.a),
        ] {
            let Some(a) = raw.vertices.get(a as usize).copied() else {
                return Err("intact triangle references a missing vertex".to_string());
            };
            let Some(b) = raw.vertices.get(b as usize).copied() else {
                return Err("intact triangle references a missing vertex".to_string());
            };
            let a = position_key(a);
            let b = position_key(b);
            let edge = if a <= b { (a, b) } else { (b, a) };
            let key = (edge.0, edge.1, triangle.material);
            if let Some(other) = edge_owner.insert(key, local_face) {
                sets.join(local_face, other);
            }
        }
    }

    let mut root_to_group = BTreeMap::<usize, u16>::new();
    let mut groups = Vec::<BreakableGroup>::new();
    let mut vertex_map = BTreeMap::<u32, u16>::new();
    let mut vertices = Vec::<BreakableVertex>::new();
    let mut triangles = Vec::<BreakableTriangle>::with_capacity(faces.len());
    let has_uvs = raw.uvs.len() == raw.vertices.len();
    let has_colors = raw.prelit_colors.len() == raw.vertices.len();

    for (local_face, triangle) in faces.iter().enumerate() {
        let root = sets.root(local_face);
        let group = if let Some(group) = root_to_group.get(&root).copied() {
            group
        } else {
            let group = u16::try_from(groups.len())
                .map_err(|_| "automatic fracture produced more than 65535 zones".to_string())?;
            let texture = raw
                .material_textures
                .get(triangle.material as usize)
                .cloned()
                .unwrap_or_default();
            groups.push(BreakableGroup {
                name: format!("Zone {}", groups.len() + 1),
                texture,
                mask: String::new(),
                ambient: V3 {
                    x: 1.0,
                    y: 1.0,
                    z: 1.0,
                },
            });
            root_to_group.insert(root, group);
            group
        };

        let mut indices = [0u16; 3];
        for (corner, source_index) in [triangle.a, triangle.b, triangle.c].into_iter().enumerate() {
            indices[corner] = if let Some(index) = vertex_map.get(&source_index).copied() {
                index
            } else {
                let source = source_index as usize;
                let position = *raw
                    .vertices
                    .get(source)
                    .ok_or_else(|| "intact triangle references a missing vertex".to_string())?;
                let index = u16::try_from(vertices.len())
                    .map_err(|_| "fracture mesh has more than 65535 vertices".to_string())?;
                let color = raw
                    .prelit_colors
                    .get(source)
                    .filter(|_| has_colors)
                    .copied()
                    .unwrap_or(V3 {
                        x: 1.0,
                        y: 1.0,
                        z: 1.0,
                    });
                vertices.push(BreakableVertex {
                    position,
                    uv: raw
                        .uvs
                        .get(source)
                        .filter(|_| has_uvs)
                        .copied()
                        .unwrap_or_default(),
                    color: [
                        (color.x.clamp(0.0, 1.0) * 255.0).round() as u8,
                        (color.y.clamp(0.0, 1.0) * 255.0).round() as u8,
                        (color.z.clamp(0.0, 1.0) * 255.0).round() as u8,
                        255,
                    ],
                });
                vertex_map.insert(source_index, index);
                index
            };
        }
        triangles.push(BreakableTriangle {
            vertices: indices,
            group,
            source_face: Some(face_start + local_face),
        });
    }

    let breakable = BreakableGeometry {
        origin: BreakableOrigin::Collision,
        vertices,
        triangles,
        groups,
        stale: false,
    };
    let errors = validate_breakable_geometry(&breakable);
    if errors.is_empty() {
        Ok(breakable)
    } else {
        Err(errors.join("; "))
    }
}

pub(crate) fn assign_faces_to_new_fracture_zone(
    raw: &RawMesh,
    component_index: usize,
    selected_faces: &BTreeSet<usize>,
) -> Result<BreakableGeometry, String> {
    let (face_start, face_end) = component_bounds(raw, component_index)?;
    let selected = selected_faces
        .iter()
        .copied()
        .filter(|face| *face >= face_start && *face < face_end)
        .collect::<BTreeSet<_>>();
    if selected.is_empty() {
        return Err("select one or more faces in a single geometry first".to_string());
    }
    let materials = selected
        .iter()
        .filter_map(|face| raw.triangles.get(*face).map(|triangle| triangle.material))
        .collect::<BTreeSet<_>>();
    if materials.len() != 1 {
        return Err(
            "a fracture zone can use only one texture; select faces from one material".to_string(),
        );
    }

    let mut breakable = raw
        .components
        .get(component_index)
        .and_then(|component| component.breakable.clone())
        .filter(|breakable| {
            breakable
                .triangles
                .iter()
                .all(|triangle| triangle.source_face.is_some())
        })
        .unwrap_or(generate_breakable_geometry(raw, component_index)?);
    let material = *materials.first().unwrap();
    let new_group = u16::try_from(breakable.groups.len())
        .map_err(|_| "fracture mesh already has 65535 zones".to_string())?;
    let mut assigned = 0usize;
    for triangle in &mut breakable.triangles {
        if triangle
            .source_face
            .is_some_and(|face| selected.contains(&face))
        {
            triangle.group = new_group;
            assigned += 1;
        }
    }
    if assigned == 0 {
        return Err(
            "selected faces could not be mapped to the debris mesh; regenerate zones first"
                .to_string(),
        );
    }
    breakable.groups.push(BreakableGroup {
        name: format!("Zone {}", breakable.groups.len() + 1),
        texture: raw
            .material_textures
            .get(material as usize)
            .cloned()
            .unwrap_or_default(),
        mask: String::new(),
        ambient: V3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        },
    });
    remove_empty_breakable_groups(&mut breakable);
    breakable.stale = false;
    Ok(breakable)
}

pub(crate) fn remove_empty_breakable_groups(breakable: &mut BreakableGeometry) {
    let used = breakable
        .triangles
        .iter()
        .map(|triangle| triangle.group)
        .collect::<BTreeSet<_>>();
    let mut remap = BTreeMap::<u16, u16>::new();
    let mut groups = Vec::new();
    for (old, group) in breakable.groups.iter().cloned().enumerate() {
        let old = old as u16;
        if used.contains(&old) {
            remap.insert(old, groups.len() as u16);
            groups.push(group);
        }
    }
    for (idx, group) in groups.iter_mut().enumerate() {
        group.name = format!("Zone {}", idx + 1);
    }
    for triangle in &mut breakable.triangles {
        if let Some(group) = remap.get(&triangle.group).copied() {
            triangle.group = group;
        }
    }
    breakable.groups = groups;
}

pub(crate) fn validate_breakable_geometry(breakable: &BreakableGeometry) -> Vec<String> {
    let mut errors = Vec::new();
    if breakable.vertices.is_empty() {
        errors.push("breakable geometry has no vertices".to_string());
    }
    if breakable.triangles.is_empty() {
        errors.push("breakable geometry has no triangles".to_string());
    }
    if breakable.groups.is_empty() {
        errors.push("breakable geometry has no fracture zones".to_string());
    }
    if breakable.vertices.len() > u16::MAX as usize {
        errors.push("breakable geometry exceeds 65535 vertices".to_string());
    }
    if breakable.triangles.len() > u16::MAX as usize {
        errors.push("breakable geometry exceeds 65535 triangles".to_string());
    }
    if breakable.groups.len() > u16::MAX as usize {
        errors.push("breakable geometry exceeds 65535 zones".to_string());
    }
    for (idx, triangle) in breakable.triangles.iter().enumerate() {
        if triangle
            .vertices
            .iter()
            .any(|vertex| *vertex as usize >= breakable.vertices.len())
        {
            errors.push(format!(
                "fracture triangle {} has an invalid vertex",
                idx + 1
            ));
        }
        if triangle.group as usize >= breakable.groups.len() {
            errors.push(format!("fracture triangle {} has an invalid zone", idx + 1));
        }
    }
    let used = breakable
        .triangles
        .iter()
        .map(|triangle| triangle.group as usize)
        .collect::<BTreeSet<_>>();
    for (idx, group) in breakable.groups.iter().enumerate() {
        if !used.contains(&idx) {
            errors.push(format!("fracture zone {} has no triangles", idx + 1));
        }
        if group.texture.trim().is_empty() {
            errors.push(format!("fracture zone {} has no texture", idx + 1));
        }
        if group.texture.trim().as_bytes().len() >= FIXED_TEXTURE_NAME {
            errors.push(format!(
                "fracture zone {} texture exceeds the 31-byte SA limit",
                idx + 1
            ));
        }
        if group.mask.trim().as_bytes().len() >= FIXED_TEXTURE_NAME {
            errors.push(format!(
                "fracture zone {} mask exceeds the 31-byte SA limit",
                idx + 1
            ));
        }
    }
    if breakable.stale {
        errors.push("fracture zones are stale after an intact-mesh edit".to_string());
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triangle_mesh() -> RawMesh {
        RawMesh {
            vertices: vec![
                V3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                V3 {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                V3 {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
            ],
            uvs: vec![V2::default(); 3],
            prelit_colors: vec![
                V3 {
                    x: 1.0,
                    y: 1.0,
                    z: 1.0,
                };
                3
            ],
            triangles: vec![Tri {
                a: 0,
                b: 1,
                c: 2,
                material: 0,
            }],
            material_textures: vec!["wood".to_string()],
            components: vec![RawMeshComponent {
                name: "object".to_string(),
                vertex_start: 0,
                vertex_end: 3,
                tri_start: 0,
                tri_end: 1,
                breakable: None,
            }],
            ..RawMesh::default()
        }
    }

    #[test]
    fn breakable_codec_round_trips_generated_geometry() {
        let generated = generate_breakable_geometry(&triangle_mesh(), 0).unwrap();
        let bytes = encode_breakable_plugin(&generated).unwrap();
        let decoded = parse_breakable_plugin(&bytes).unwrap().unwrap();
        assert_eq!(decoded.origin, BreakableOrigin::Collision);
        assert_eq!(decoded.vertices, generated.vertices);
        assert_eq!(decoded.groups, generated.groups);
        assert_eq!(
            decoded
                .triangles
                .iter()
                .map(|triangle| (triangle.vertices, triangle.group))
                .collect::<Vec<_>>(),
            generated
                .triangles
                .iter()
                .map(|triangle| (triangle.vertices, triangle.group))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            bytes.len(),
            checked_payload_size(3, 1, 1).expect("valid size")
        );
    }

    #[test]
    fn breakable_parser_rejects_truncated_counts() {
        let generated = generate_breakable_geometry(&triangle_mesh(), 0).unwrap();
        let mut bytes = encode_breakable_plugin(&generated).unwrap();
        bytes.truncate(bytes.len() - 1);
        assert!(parse_breakable_plugin(&bytes).is_err());
    }

    #[test]
    fn manual_zone_uses_selected_faces() {
        let mut raw = triangle_mesh();
        raw.components[0].breakable = Some(generate_breakable_geometry(&raw, 0).unwrap());
        let selected = BTreeSet::from([0usize]);
        let reassigned = assign_faces_to_new_fracture_zone(&raw, 0, &selected).unwrap();
        assert_eq!(reassigned.groups.len(), 1);
        assert_eq!(reassigned.triangles[0].source_face, Some(0));
    }

    #[test]
    fn normalized_dff_round_trip_preserves_breakable_plugin() {
        let mut raw = triangle_mesh();
        raw.components[0].breakable = Some(generate_breakable_geometry(&raw, 0).unwrap());
        let dff = write_normalized_dff(&raw, "breakable_test").unwrap();
        let reparsed = parse_dff_mesh(&dff);
        let breakable = reparsed.components[0].breakable.as_ref().unwrap();
        assert_eq!(breakable.vertices.len(), 3);
        assert_eq!(breakable.triangles.len(), 1);
        assert_eq!(breakable.groups.len(), 1);
        assert_eq!(breakable.groups[0].texture, "wood");
        assert_eq!(breakable.triangles[0].source_face, Some(0));
    }

    #[test]
    fn texture_rewrite_updates_intact_and_fragment_references() {
        let mut raw = triangle_mesh();
        raw.components[0].breakable = Some(generate_breakable_geometry(&raw, 0).unwrap());
        let dff = write_normalized_dff(&raw, "breakable_test").unwrap();
        let renames = HashMap::from([("wood".to_string(), "new_wood".to_string())]);
        let (rewritten, changed) = rewrite_dff_material_textures(&dff, &renames).unwrap();
        assert_eq!(changed, 2);
        let reparsed = parse_dff_mesh(&rewritten);
        assert_eq!(reparsed.material_textures[0], "new_wood");
        assert_eq!(
            reparsed.components[0].breakable.as_ref().unwrap().groups[0].texture,
            "new_wood"
        );
    }
}
