use super::super::*;

const NORMAL_EPSILON: f32 = 1.0e-5;
const PLANE_EPSILON: f32 = 1.0e-4;
const GROUND_NORMAL_Z_MIN: f32 = 0.25;
const GROUND_WINDING_DOMINANCE: f64 = 4.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ColCoplanarMergeStats {
    pub(crate) regions_merged: usize,
    pub(crate) faces_before: usize,
    pub(crate) faces_after: usize,
    pub(crate) faces_removed: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ColWindingRepairStats {
    pub(crate) faces_reoriented: usize,
}

fn face_vertices(mesh: &CollisionMesh, face: &CollisionFace) -> Option<[V3; 3]> {
    Some([
        *mesh.vertices.get(face.a as usize)?,
        *mesh.vertices.get(face.b as usize)?,
        *mesh.vertices.get(face.c as usize)?,
    ])
}

fn face_normal(mesh: &CollisionMesh, face: &CollisionFace) -> Option<Vec3> {
    let [a, b, c] = face_vertices(mesh, face)?;
    let a = to_mq(a);
    let b = to_mq(b);
    let c = to_mq(c);
    let normal = (b - a).cross(c - a);
    let length = normal.length();
    (length > NORMAL_EPSILON && length.is_finite()).then_some(normal / length)
}

/// Repair a collision mesh exported with conventional counter-clockwise
/// winding when its ground surfaces make that inversion unambiguous.
///
/// GTA builds a collision plane with `(c - a) x (b - a)`, the reverse of the
/// conventional triangle cross product used by most exporters. Projected
/// vehicle-light shadows use those planes, so a road exported with a dominant
/// conventional upward winding has effective GTA normals pointing downward.
///
/// The repair deliberately requires a 4:1 dominance in both projected area
/// and face count. Once detected, every face is reversed because winding is a
/// mesh-wide convention and walls/undersides must stay consistent with roads.
/// A repaired mesh is a fixed point on subsequent optimization passes.
pub(crate) fn repair_inverted_ground_winding(mesh: &mut CollisionMesh) -> ColWindingRepairStats {
    let mut upward_projected_area = 0.0f64;
    let mut downward_projected_area = 0.0f64;
    let mut upward_faces = 0usize;
    let mut downward_faces = 0usize;

    for face in &mesh.faces {
        let Some([a, b, c]) = face_vertices(mesh, face) else {
            continue;
        };
        let normal = (to_mq(b) - to_mq(a)).cross(to_mq(c) - to_mq(a));
        let length = normal.length();
        if length <= NORMAL_EPSILON || !length.is_finite() {
            continue;
        }
        let normalized_z = normal.z / length;
        if normalized_z >= GROUND_NORMAL_Z_MIN {
            upward_projected_area += normal.z as f64;
            upward_faces += 1;
        } else if normalized_z <= -GROUND_NORMAL_Z_MIN {
            downward_projected_area += (-normal.z) as f64;
            downward_faces += 1;
        }
    }

    let area_is_dominant =
        upward_projected_area > downward_projected_area * GROUND_WINDING_DOMINANCE;
    let count_is_dominant =
        upward_faces > downward_faces.saturating_mul(GROUND_WINDING_DOMINANCE as usize);
    if upward_faces == 0 || !area_is_dominant || !count_is_dominant {
        return ColWindingRepairStats::default();
    }

    for face in &mut mesh.faces {
        std::mem::swap(&mut face.b, &mut face.c);
    }
    ColWindingRepairStats {
        faces_reoriented: mesh.faces.len(),
    }
}

fn same_plane_and_surface(
    mesh: &CollisionMesh,
    seed: &CollisionFace,
    seed_normal: Vec3,
    candidate: &CollisionFace,
) -> bool {
    if seed.material != candidate.material || seed.light != candidate.light {
        return false;
    }
    let Some(candidate_normal) = face_normal(mesh, candidate) else {
        return false;
    };
    if seed_normal.dot(candidate_normal) < 1.0 - NORMAL_EPSILON {
        return false;
    }
    let Some([origin, _, _]) = face_vertices(mesh, seed) else {
        return false;
    };
    face_vertices(mesh, candidate).is_some_and(|vertices| {
        vertices
            .iter()
            .all(|vertex| seed_normal.dot(to_mq(*vertex) - to_mq(origin)).abs() <= PLANE_EPSILON)
    })
}

fn undirected_edge(a: u16, b: u16) -> (u16, u16) {
    if a < b { (a, b) } else { (b, a) }
}

fn projected(vertex: V3, dominant_axis: usize) -> (f64, f64) {
    match dominant_axis {
        0 => (vertex.y as f64, vertex.z as f64),
        1 => (vertex.x as f64, vertex.z as f64),
        _ => (vertex.x as f64, vertex.y as f64),
    }
}

fn cross_2d(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> f64 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

fn remove_collinear_boundary_vertices(mesh: &CollisionMesh, boundary: &mut Vec<u16>) {
    loop {
        if boundary.len() <= 3 {
            return;
        }
        let mut removed = false;
        for idx in 0..boundary.len() {
            let prev =
                mesh.vertices[boundary[(idx + boundary.len() - 1) % boundary.len()] as usize];
            let current = mesh.vertices[boundary[idx] as usize];
            let next = mesh.vertices[boundary[(idx + 1) % boundary.len()] as usize];
            let lhs = to_mq(current) - to_mq(prev);
            let rhs = to_mq(next) - to_mq(current);
            let scale = lhs.length().max(rhs.length()).max(1.0);
            let collinear = lhs.cross(rhs).length() <= PLANE_EPSILON * scale;
            let between =
                (to_mq(prev) - to_mq(current)).dot(to_mq(next) - to_mq(current)) <= PLANE_EPSILON;
            if collinear && between {
                boundary.remove(idx);
                removed = true;
                break;
            }
        }
        if !removed {
            return;
        }
    }
}

fn point_in_triangle(
    point: (f64, f64),
    a: (f64, f64),
    b: (f64, f64),
    c: (f64, f64),
    orientation: f64,
) -> bool {
    const EPSILON: f64 = 1.0e-10;
    orientation * cross_2d(a, b, point) >= -EPSILON
        && orientation * cross_2d(b, c, point) >= -EPSILON
        && orientation * cross_2d(c, a, point) >= -EPSILON
}

fn triangulate_boundary(
    mesh: &CollisionMesh,
    boundary: &[u16],
    normal: Vec3,
) -> Option<Vec<[u16; 3]>> {
    if boundary.len() < 3 {
        return None;
    }
    let dominant_axis = if normal.x.abs() >= normal.y.abs() && normal.x.abs() >= normal.z.abs() {
        0
    } else if normal.y.abs() >= normal.z.abs() {
        1
    } else {
        2
    };
    let points = boundary
        .iter()
        .map(|index| projected(mesh.vertices[*index as usize], dominant_axis))
        .collect::<Vec<_>>();
    let signed_area = points
        .iter()
        .enumerate()
        .map(|(idx, point)| {
            let next = points[(idx + 1) % points.len()];
            point.0 * next.1 - next.0 * point.1
        })
        .sum::<f64>();
    if signed_area.abs() <= 1.0e-10 {
        return None;
    }
    let orientation = signed_area.signum();
    let mut remaining = (0..boundary.len()).collect::<Vec<_>>();
    let mut triangles = Vec::with_capacity(boundary.len() - 2);
    while remaining.len() > 3 {
        let mut ear = None;
        for cursor in 0..remaining.len() {
            let prev = remaining[(cursor + remaining.len() - 1) % remaining.len()];
            let current = remaining[cursor];
            let next = remaining[(cursor + 1) % remaining.len()];
            if orientation * cross_2d(points[prev], points[current], points[next]) <= 1.0e-10 {
                continue;
            }
            let contains_other = remaining.iter().copied().any(|candidate| {
                candidate != prev
                    && candidate != current
                    && candidate != next
                    && point_in_triangle(
                        points[candidate],
                        points[prev],
                        points[current],
                        points[next],
                        orientation,
                    )
            });
            if !contains_other {
                ear = Some((cursor, [boundary[prev], boundary[current], boundary[next]]));
                break;
            }
        }
        let (cursor, triangle) = ear?;
        triangles.push(triangle);
        remaining.remove(cursor);
    }
    triangles.push([
        boundary[remaining[0]],
        boundary[remaining[1]],
        boundary[remaining[2]],
    ]);
    Some(triangles)
}

fn merged_component(
    mesh: &CollisionMesh,
    component: &[usize],
    normal: Vec3,
) -> Option<Vec<CollisionFace>> {
    if component.len() < 2 {
        return None;
    }
    let mut edges = std::collections::BTreeMap::<(u16, u16), Vec<(u16, u16)>>::new();
    for face_idx in component {
        let face = &mesh.faces[*face_idx];
        for (a, b) in [(face.a, face.b), (face.b, face.c), (face.c, face.a)] {
            edges.entry(undirected_edge(a, b)).or_default().push((a, b));
        }
    }
    if edges
        .values()
        .any(|uses| uses.len() > 2 || (uses.len() == 2 && uses[0] != (uses[1].1, uses[1].0)))
    {
        return None;
    }
    let boundary_edges = edges
        .values()
        .filter_map(|uses| (uses.len() == 1).then_some(uses[0]))
        .collect::<Vec<_>>();
    if boundary_edges.len() < 3 {
        return None;
    }
    let mut next = std::collections::BTreeMap::<u16, u16>::new();
    let mut incoming = std::collections::BTreeMap::<u16, usize>::new();
    for (a, b) in &boundary_edges {
        if next.insert(*a, *b).is_some() {
            return None;
        }
        *incoming.entry(*b).or_default() += 1;
    }
    if next.len() != boundary_edges.len()
        || incoming.len() != boundary_edges.len()
        || incoming.values().any(|count| *count != 1)
    {
        return None;
    }
    let start = *next.keys().next()?;
    let mut boundary = Vec::with_capacity(boundary_edges.len());
    let mut current = start;
    loop {
        if boundary.len() >= boundary_edges.len() {
            return None;
        }
        boundary.push(current);
        current = *next.get(&current)?;
        if current == start {
            break;
        }
    }
    if boundary.len() != boundary_edges.len() {
        // Multiple loops imply a hole or disconnected boundary.
        return None;
    }
    remove_collinear_boundary_vertices(mesh, &mut boundary);
    let triangles = triangulate_boundary(mesh, &boundary, normal)?;
    if triangles.len() >= component.len() {
        return None;
    }
    let source = &mesh.faces[component[0]];
    Some(
        triangles
            .into_iter()
            .map(|[a, b, c]| CollisionFace {
                a,
                b,
                c,
                material: source.material,
                light: source.light,
                img_path: source.img_path.clone(),
                material_file_offset: 0,
                light_file_offset: 0,
            })
            .collect(),
    )
}

/// Retriangulate same-surface connected planar patches only when doing so
/// reduces their triangle count without changing their single outer boundary.
///
/// Regions with holes, non-manifold edges, inconsistent winding, degenerate
/// faces, different material/light values, or a non-coplanar face are left
/// byte-for-byte equivalent at the mesh level. The operation is deterministic
/// and reaches a fixed point after one pass.
pub(crate) fn merge_connected_coplanar_faces(mesh: &mut CollisionMesh) -> ColCoplanarMergeStats {
    let before = mesh.faces.len();
    let mut edge_faces = std::collections::BTreeMap::<(u16, u16), Vec<usize>>::new();
    for (face_idx, face) in mesh.faces.iter().enumerate() {
        for (a, b) in [(face.a, face.b), (face.b, face.c), (face.c, face.a)] {
            edge_faces
                .entry(undirected_edge(a, b))
                .or_default()
                .push(face_idx);
        }
    }
    let mut neighbors = vec![Vec::new(); mesh.faces.len()];
    for uses in edge_faces.values() {
        if uses.len() == 2 {
            neighbors[uses[0]].push(uses[1]);
            neighbors[uses[1]].push(uses[0]);
        }
    }
    for values in &mut neighbors {
        values.sort_unstable();
    }

    let mut visited = vec![false; mesh.faces.len()];
    let mut replacements = std::collections::BTreeMap::<usize, Vec<CollisionFace>>::new();
    let mut removed = std::collections::BTreeSet::<usize>::new();
    for seed_idx in 0..mesh.faces.len() {
        if visited[seed_idx] {
            continue;
        }
        visited[seed_idx] = true;
        let Some(normal) = face_normal(mesh, &mesh.faces[seed_idx]) else {
            continue;
        };
        let mut component = Vec::new();
        let mut queue = std::collections::VecDeque::from([seed_idx]);
        while let Some(face_idx) = queue.pop_front() {
            component.push(face_idx);
            for neighbor in &neighbors[face_idx] {
                if !visited[*neighbor]
                    && same_plane_and_surface(
                        mesh,
                        &mesh.faces[seed_idx],
                        normal,
                        &mesh.faces[*neighbor],
                    )
                {
                    visited[*neighbor] = true;
                    queue.push_back(*neighbor);
                }
            }
        }
        component.sort_unstable();
        if let Some(merged) = merged_component(mesh, &component, normal) {
            replacements.insert(component[0], merged);
            removed.extend(component);
        }
    }

    let regions_merged = replacements.len();
    if regions_merged != 0 {
        let mut faces = Vec::with_capacity(mesh.faces.len());
        for (face_idx, face) in mesh.faces.iter().enumerate() {
            if let Some(replacement) = replacements.remove(&face_idx) {
                faces.extend(replacement);
            } else if !removed.contains(&face_idx) {
                faces.push(face.clone());
            }
        }
        mesh.faces = faces;
        mesh.bounds = collision_mesh_bounds(&mesh.vertices, &mesh.spheres, &mesh.boxes);
    }
    let after = mesh.faces.len();
    ColCoplanarMergeStats {
        regions_merged,
        faces_before: before,
        faces_after: after,
        faces_removed: before.saturating_sub(after),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn face(a: u16, b: u16, c: u16, material: u8) -> CollisionFace {
        CollisionFace {
            a,
            b,
            c,
            material,
            light: 4,
            img_path: PathBuf::from("test.col"),
            material_file_offset: 0,
            light_file_offset: 0,
        }
    }

    fn mesh(vertices: Vec<V3>, faces: Vec<CollisionFace>) -> CollisionMesh {
        let bounds = collision_mesh_bounds(&vertices, &[], &[]);
        CollisionMesh {
            name: "test".to_string(),
            spheres: Vec::new(),
            boxes: Vec::new(),
            vertices,
            faces,
            bounds,
            shadow_vertices: Vec::new(),
            shadow_faces: Vec::new(),
        }
    }

    fn v3(x: f32, y: f32, z: f32) -> V3 {
        V3 { x, y, z }
    }

    #[test]
    fn repairs_exporter_winding_on_ground_meshes_for_gta_planes() {
        let mut value = mesh(
            vec![
                v3(0.0, 0.0, 0.0),
                v3(2.0, 0.0, 0.0),
                v3(2.0, 2.0, 0.0),
                v3(0.0, 2.0, 0.0),
            ],
            vec![face(0, 1, 2, 1), face(0, 2, 3, 1)],
        );

        assert_eq!(
            repair_inverted_ground_winding(&mut value),
            ColWindingRepairStats {
                faces_reoriented: 2
            }
        );
        assert!(
            value
                .faces
                .iter()
                .all(|face| { face_normal(&value, face).is_some_and(|normal| normal.z < -0.99) })
        );

        let repaired = value.faces.clone();
        assert_eq!(
            repair_inverted_ground_winding(&mut value),
            ColWindingRepairStats::default()
        );
        assert!(value.faces == repaired);
    }

    #[test]
    fn preserves_existing_gta_ground_winding() {
        let mut value = mesh(
            vec![
                v3(0.0, 0.0, 0.0),
                v3(2.0, 0.0, 0.0),
                v3(2.0, 2.0, 0.0),
                v3(0.0, 2.0, 0.0),
            ],
            vec![face(0, 2, 1, 1), face(0, 3, 2, 1)],
        );
        let original = value.faces.clone();

        assert_eq!(
            repair_inverted_ground_winding(&mut value),
            ColWindingRepairStats::default()
        );
        assert!(value.faces == original);
    }

    #[test]
    fn preserves_ambiguous_mixed_winding() {
        let mut value = mesh(
            vec![
                v3(0.0, 0.0, 0.0),
                v3(2.0, 0.0, 0.0),
                v3(2.0, 2.0, 0.0),
                v3(0.0, 2.0, 0.0),
            ],
            vec![face(0, 1, 2, 1), face(0, 3, 2, 1)],
        );
        let original = value.faces.clone();

        assert_eq!(
            repair_inverted_ground_winding(&mut value),
            ColWindingRepairStats::default()
        );
        assert!(value.faces == original);
    }

    #[test]
    fn removes_an_interior_vertex_from_a_planar_patch() {
        let mut value = mesh(
            vec![
                v3(0.0, 0.0, 0.0),
                v3(2.0, 0.0, 0.0),
                v3(2.0, 2.0, 0.0),
                v3(0.0, 2.0, 0.0),
                v3(1.0, 1.0, 0.0),
            ],
            vec![
                face(0, 1, 4, 1),
                face(1, 2, 4, 1),
                face(2, 3, 4, 1),
                face(3, 0, 4, 1),
            ],
        );

        let stats = merge_connected_coplanar_faces(&mut value);

        assert_eq!(stats.regions_merged, 1);
        assert_eq!(stats.faces_removed, 2);
        assert_eq!(value.faces.len(), 2);
        let once = value.faces.clone();
        assert_eq!(
            merge_connected_coplanar_faces(&mut value),
            ColCoplanarMergeStats {
                faces_before: 2,
                faces_after: 2,
                ..Default::default()
            }
        );
        assert!(value.faces == once);
    }

    #[test]
    fn preserves_material_boundaries() {
        let mut value = mesh(
            vec![
                v3(0.0, 0.0, 0.0),
                v3(1.0, 0.0, 0.0),
                v3(1.0, 1.0, 0.0),
                v3(0.0, 1.0, 0.0),
            ],
            vec![face(0, 1, 2, 1), face(0, 2, 3, 2)],
        );
        let original = value.faces.clone();

        let stats = merge_connected_coplanar_faces(&mut value);

        assert_eq!(stats.faces_removed, 0);
        assert!(value.faces == original);
    }

    #[test]
    fn preserves_non_coplanar_regions() {
        let mut value = mesh(
            vec![
                v3(0.0, 0.0, 0.0),
                v3(1.0, 0.0, 0.0),
                v3(1.0, 1.0, 0.0),
                v3(0.0, 1.0, 1.0),
            ],
            vec![face(0, 1, 2, 1), face(0, 2, 3, 1)],
        );
        let original = value.faces.clone();

        let stats = merge_connected_coplanar_faces(&mut value);

        assert_eq!(stats.faces_removed, 0);
        assert!(value.faces == original);
    }
}
