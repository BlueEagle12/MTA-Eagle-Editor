use super::super::*;

fn collect_texture_strings(b: &[u8], start: usize, end: usize, names: &mut Vec<String>) {
    let mut o = start;
    while o + 12 <= end {
        let id = rd32(b, o);
        let size = rd32(b, o + 4) as usize;
        let cs = o + 12;
        let ce = cs.saturating_add(size);
        if ce > end || ce > b.len() {
            break;
        }
        if id == 0x02 {
            let bytes = &b[cs..ce];
            let end = bytes.iter().position(|v| *v == 0).unwrap_or(bytes.len());
            let value = String::from_utf8_lossy(&bytes[..end]).to_string();
            if !value.is_empty() {
                names.push(value);
            }
        } else if id != 0x01 {
            collect_texture_strings(b, cs, ce, names);
        }
        o = ce;
    }
}

fn parse_uv_anim_plugin_names(b: &[u8], start: usize, end: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut o = start;
    while o + 12 <= end {
        let id = rd32(b, o);
        let size = rd32(b, o + 4) as usize;
        let cs = o + 12;
        let ce = cs.saturating_add(size);
        if ce > end || ce > b.len() {
            break;
        }
        if id == 0x01 && size >= 4 {
            let mask = rd32(b, cs);
            let anim_count = (mask & 0xff).count_ones() as usize;
            let mut p = cs + 4;
            for _ in 0..anim_count {
                if p + 32 > ce {
                    break;
                }
                let bytes = &b[p..p + 32];
                let end = bytes.iter().position(|v| *v == 0).unwrap_or(bytes.len());
                let value = String::from_utf8_lossy(&bytes[..end]).to_string();
                if !value.is_empty() {
                    out.push(value);
                }
                p += 32;
            }
            break;
        }
        o = ce;
    }
    out
}

fn collect_uv_anim_plugin_names(b: &[u8], start: usize, end: usize, names: &mut Vec<String>) {
    let mut o = start;
    while o + 12 <= end {
        let id = rd32(b, o);
        let size = rd32(b, o + 4) as usize;
        let cs = o + 12;
        let ce = cs.saturating_add(size);
        if ce > end || ce > b.len() {
            break;
        }
        if id == 0x0135 {
            names.extend(parse_uv_anim_plugin_names(b, cs, ce));
        } else if id != 0x01 {
            collect_uv_anim_plugin_names(b, cs, ce, names);
        }
        o = ce;
    }
}

fn parse_material_chunk(
    b: &[u8],
    start: usize,
    end: usize,
) -> (String, RawMaterial, DffMaterialAnim) {
    let mut texture_names = Vec::new();
    let mut anim_names = Vec::new();
    let mut material = RawMaterial {
        color: neutral_vertex_color(),
        alpha: 1.0,
        ambient: 1.0,
        specular: 1.0,
        diffuse: 1.0,
    };
    let mut o = start;
    while o + 12 <= end {
        let id = rd32(b, o);
        let size = rd32(b, o + 4) as usize;
        let cs = o + 12;
        let ce = cs.saturating_add(size);
        if ce > end || ce > b.len() {
            break;
        }
        if id == 0x01 && size >= 28 {
            material = RawMaterial {
                color: V3 {
                    x: b[cs + 4] as f32 / 255.0,
                    y: b[cs + 5] as f32 / 255.0,
                    z: b[cs + 6] as f32 / 255.0,
                },
                alpha: b[cs + 7] as f32 / 255.0,
                ambient: rdf32(b, cs + 16),
                specular: rdf32(b, cs + 20),
                diffuse: rdf32(b, cs + 24),
            };
        } else if id == 0x06 {
            collect_texture_strings(b, cs, ce, &mut texture_names);
        } else if id == 0x03 {
            collect_uv_anim_plugin_names(b, cs, ce, &mut anim_names);
        }
        o = ce;
    }
    (
        texture_names.first().cloned().unwrap_or_default(),
        material,
        DffMaterialAnim { names: anim_names },
    )
}

fn parse_material_textures(
    b: &[u8],
    start: usize,
    end: usize,
    names: &mut Vec<String>,
    materials: &mut Vec<RawMaterial>,
    animations: &mut Vec<DffMaterialAnim>,
) {
    let mut o = start;
    while o + 12 <= end {
        let id = rd32(b, o);
        let size = rd32(b, o + 4) as usize;
        let cs = o + 12;
        let ce = cs.saturating_add(size);
        if ce > end || ce > b.len() {
            break;
        }
        if id == 0x07 {
            let (texture, material, animation) = parse_material_chunk(b, cs, ce);
            names.push(texture);
            materials.push(material);
            animations.push(animation);
        } else if id != 0x01 {
            parse_material_textures(b, cs, ce, names, materials, animations);
        }
        o = ce;
    }
}

fn parse_fixed_name(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).trim().to_string()
}

fn parse_uv_animation_anim(b: &[u8], start: usize, end: usize) -> Option<DffUvAnimation> {
    if start + 88 > end || end > b.len() {
        return None;
    }
    let type_id = rd32(b, start + 4) as i32;
    let frame_count = rd32(b, start + 8) as usize;
    let flags = rd32(b, start + 12) as i32;
    let duration = rdf32(b, start + 16).max(0.0);
    let name = parse_fixed_name(&b[start + 24..start + 56]);
    if name.is_empty() {
        return None;
    }
    let mut node_to_uv = [0u32; 8];
    for (idx, value) in node_to_uv.iter_mut().enumerate() {
        *value = rd32(b, start + 56 + idx * 4);
    }
    let mut frames = Vec::new();
    let mut p = start + 88;
    for frame_idx in 0..frame_count {
        if p + 32 > end {
            break;
        }
        let mut uv = [0.0f32; 6];
        for (idx, value) in uv.iter_mut().enumerate() {
            *value = rdf32(b, p + 4 + idx * 4);
        }
        frames.push(DffUvAnimFrame {
            time: rdf32(b, p),
            uv,
            prev: rd32(b, p + 28) as i32,
        });
        p += 32;
        if frame_idx > 512 {
            break;
        }
    }
    Some(DffUvAnimation {
        name,
        type_id,
        flags,
        duration,
        node_to_uv,
        frames,
    })
}

fn collect_uv_animations_from_dictionary_payload(
    b: &[u8],
    start: usize,
    end: usize,
    out: &mut Vec<DffUvAnimation>,
) {
    let mut o = start;
    while o + 12 <= end {
        let id = rd32(b, o);
        let size = rd32(b, o + 4) as usize;
        let cs = o + 12;
        let ce = cs.saturating_add(size);
        if ce > end || ce > b.len() {
            break;
        }
        if id == 0x1b {
            if let Some(anim) = parse_uv_animation_anim(b, cs, ce) {
                out.push(anim);
            }
        } else if id != 0x01 {
            collect_uv_animations_from_dictionary_payload(b, cs, ce, out);
        }
        o = ce;
    }
}

fn push_top_level_uv_anim_dictionaries(b: &[u8], mesh: &mut RawMesh) {
    let mut o = 0usize;
    while o + 12 <= b.len() {
        let id = rd32(b, o);
        let size = rd32(b, o + 4) as usize;
        let ce = o.saturating_add(12).saturating_add(size);
        if ce > b.len() {
            break;
        }
        if id == 0x2b {
            mesh.uv_anim_dictionaries.push(b[o..ce].to_vec());
            collect_uv_animations_from_dictionary_payload(b, o + 12, ce, &mut mesh.uv_animations);
        }
        o = ce;
    }
}

fn push_bin_mesh_triangle(
    out: &mut Vec<Tri>,
    a: u32,
    b: u32,
    c: u32,
    material: u32,
    vert_count: usize,
) {
    if a == b || b == c || a == c {
        return;
    }
    if a as usize >= vert_count || b as usize >= vert_count || c as usize >= vert_count {
        return;
    }
    out.push(Tri {
        a,
        b: c,
        c: b,
        material: material.min(u16::MAX as u32) as u16,
    });
}

fn parse_bin_mesh_plugin(
    b: &[u8],
    start: usize,
    end: usize,
    vert_count: usize,
) -> Option<Vec<Tri>> {
    if start + 12 > end || end > b.len() {
        return None;
    }
    let flags = rd32(b, start);
    let mesh_count = rd32(b, start + 4) as usize;
    let total_indices = rd32(b, start + 8) as usize;
    if mesh_count == 0 || total_indices == 0 {
        return None;
    }

    let is_tri_strip = flags & 0x01 != 0;
    let mut p = start + 12;
    let mut triangles = Vec::with_capacity(total_indices / 3);
    for _ in 0..mesh_count {
        if p + 8 > end {
            return None;
        }
        let index_count = rd32(b, p) as usize;
        let material = rd32(b, p + 4);
        p += 8;
        if index_count == 0 || p + index_count * 4 > end {
            return None;
        }
        let mut indices = Vec::with_capacity(index_count);
        for i in 0..index_count {
            indices.push(rd32(b, p + i * 4));
        }
        p += index_count * 4;

        if is_tri_strip {
            for i in 2..indices.len() {
                let (a, b, c) = if i & 1 == 0 {
                    (indices[i - 2], indices[i - 1], indices[i])
                } else {
                    (indices[i - 1], indices[i - 2], indices[i])
                };
                push_bin_mesh_triangle(&mut triangles, a, b, c, material, vert_count);
            }
        } else {
            for tri in indices.chunks_exact(3) {
                push_bin_mesh_triangle(
                    &mut triangles,
                    tri[0],
                    tri[1],
                    tri[2],
                    material,
                    vert_count,
                );
            }
        }
    }

    if triangles.is_empty() {
        None
    } else {
        Some(triangles)
    }
}

fn parse_geometry_extension_bin_mesh(
    b: &[u8],
    start: usize,
    end: usize,
    vert_count: usize,
) -> Option<Vec<Tri>> {
    let mut o = start;
    while o + 12 <= end {
        let id = rd32(b, o);
        let size = rd32(b, o + 4) as usize;
        let cs = o + 12;
        let ce = cs.saturating_add(size);
        if ce > end || ce > b.len() {
            break;
        }
        if id == 0x50E {
            if let Some(tris) = parse_bin_mesh_plugin(b, cs, ce, vert_count) {
                return Some(tris);
            }
        }
        o = ce;
    }
    None
}

fn parse_geometry_extension_night_colors(
    b: &[u8],
    start: usize,
    end: usize,
    vert_count: usize,
) -> Option<(Vec<V3>, Vec<f32>)> {
    let mut o = start;
    while o + 12 <= end {
        let id = rd32(b, o);
        let size = rd32(b, o + 4) as usize;
        let cs = o + 12;
        let ce = cs.saturating_add(size);
        if ce > end || ce > b.len() {
            break;
        }
        if id == 0x0253_f2f9 && size >= 4 {
            let enabled = rd32(b, cs) != 0;
            let colors_start = cs + 4;
            if enabled && colors_start + vert_count * 4 <= ce {
                let mut colors = Vec::with_capacity(vert_count);
                let mut alphas = Vec::with_capacity(vert_count);
                for i in 0..vert_count {
                    let color_p = colors_start + i * 4;
                    colors.push(V3 {
                        x: b[color_p] as f32 / 255.0,
                        y: b[color_p + 1] as f32 / 255.0,
                        z: b[color_p + 2] as f32 / 255.0,
                    });
                    alphas.push(b[color_p + 3] as f32 / 255.0);
                }
                return Some((colors, alphas));
            }
        }
        o = ce;
    }
    None
}

fn parse_geometry_extension_2dfx(b: &[u8], start: usize, end: usize, mesh: &mut RawMesh) {
    if start + 4 > end || end > b.len() {
        return;
    }
    let count = rd32(b, start) as usize;
    let mut p = start + 4;
    for _ in 0..count {
        if p + 20 > end {
            break;
        }
        let position = V3 {
            x: rdf32(b, p),
            y: rdf32(b, p + 4),
            z: rdf32(b, p + 8),
        };
        let effect_id = rd32(b, p + 12);
        let payload_size = rd32(b, p + 16) as usize;
        p += 20;
        if p + payload_size > end {
            break;
        }
        mesh.effects_2dfx.push(Dff2dEffect {
            position,
            effect_id,
            payload: b[p..p + payload_size].to_vec(),
        });
        p += payload_size;
    }
}

#[derive(Default)]
struct GeometryMaterials {
    names: Vec<String>,
    materials: Vec<RawMaterial>,
    animations: Vec<DffMaterialAnim>,
}

/// Fold one geometry's material list into the model-wide table and return the
/// base every triangle in that geometry rebases onto.
///
/// A RenderWare geometry owns its own material list, so a clump whose atomics
/// share materials repeats them. Appending unconditionally meant each rewrite
/// multiplied the table by the atomic count and moved the material index that
/// identifies a texture, which made per-material tools (the world-scale UV
/// unwrap in particular) act on the wrong faces the second time they ran.
/// Reusing an identical run keeps the table canonical and the indices stable
/// across any number of write/read cycles.
fn merge_geometry_materials(mesh: &mut RawMesh, geometry: GeometryMaterials) -> u32 {
    let count = geometry.names.len();
    let aligned = mesh.materials.len() == mesh.material_textures.len()
        && mesh.material_animations.len() == mesh.material_textures.len()
        && geometry.materials.len() == count
        && geometry.animations.len() == count;
    if count > 0 && aligned && mesh.material_textures.len() >= count {
        for base in 0..=(mesh.material_textures.len() - count) {
            if mesh.material_textures[base..base + count] == geometry.names[..]
                && mesh.materials[base..base + count] == geometry.materials[..]
                && mesh.material_animations[base..base + count] == geometry.animations[..]
            {
                return base as u32;
            }
        }
    }
    let base = mesh.material_textures.len() as u32;
    mesh.material_textures.extend(geometry.names);
    mesh.materials.extend(geometry.materials);
    mesh.material_animations.extend(geometry.animations);
    base
}

fn parse_geometry_chunk(b: &[u8], start: usize, end: usize, mesh: &mut RawMesh) -> bool {
    let mut o = start;
    let vertex_base = mesh.vertices.len() as u32;
    let mut geometry_materials = GeometryMaterials::default();
    let mut parsed_geometry: Option<(
        usize,
        Vec<Vec<V2>>,
        Vec<V3>,
        Vec<f32>,
        Vec<Tri>,
        Vec<V3>,
        Vec<V3>,
        bool,
    )> = None;
    let mut bin_mesh_tris: Option<Vec<Tri>> = None;
    let mut night_colors: Option<(Vec<V3>, Vec<f32>)> = None;
    let mut breakable: Option<BreakableGeometry> = None;
    while o + 12 <= end {
        let id = rd32(b, o);
        let size = rd32(b, o + 4) as usize;
        let cs = o + 12;
        let ce = cs.saturating_add(size);
        if ce > end || ce > b.len() {
            return false;
        }
        if id == 0x08 {
            parse_material_textures(
                b,
                cs,
                ce,
                &mut geometry_materials.names,
                &mut geometry_materials.materials,
                &mut geometry_materials.animations,
            );
        }
        if id == 0x03 {
            if let Some((vert_count, _, _, _, _, _, _, _)) = parsed_geometry.as_ref() {
                bin_mesh_tris = parse_geometry_extension_bin_mesh(b, cs, ce, *vert_count);
                night_colors = parse_geometry_extension_night_colors(b, cs, ce, *vert_count);
            }
            let mut ext = cs;
            while ext + 12 <= ce {
                let ext_id = rd32(b, ext);
                let ext_size = rd32(b, ext + 4) as usize;
                let ext_start = ext + 12;
                let ext_end = ext_start.saturating_add(ext_size);
                if ext_end > ce || ext_end > b.len() {
                    break;
                }
                if ext_id == 0x0253_f2f8 {
                    parse_geometry_extension_2dfx(b, ext_start, ext_end, mesh);
                } else if ext_id == BREAKABLE_PLUGIN_ID {
                    if let Ok(parsed) = parse_breakable_plugin(&b[ext_start..ext_end]) {
                        breakable = parsed;
                    }
                }
                ext = ext_end;
            }
        }
        if id == 0x01 && size >= 16 {
            let flags = rd16(b, cs);
            let geometry_lit = flags & 0x20 != 0;
            let mut uv_count = b[cs + 2];
            let tri_count = rd32(b, cs + 4) as usize;
            let vert_count = rd32(b, cs + 8) as usize;
            if uv_count == 0 {
                uv_count = if flags & 0x80 != 0 {
                    2
                } else if flags & 0x04 != 0 {
                    1
                } else {
                    0
                };
            }
            // Parse the geometry into local buffers and only commit to the shared
            // mesh once the vertex block is confirmed present and in-bounds.
            // Otherwise a truncated/native geometry chunk could leave triangles
            // that reference vertices which were never appended, causing an
            // out-of-bounds panic later in compile_render_mesh.
            let mut p = cs + 16;
            let mut local_colors = vec![neutral_vertex_color(); vert_count];
            let mut local_alphas = vec![1.0; vert_count];
            if flags & 0x08 != 0 {
                if p + vert_count * 4 > ce {
                    return false;
                }
                for (i, color) in local_colors.iter_mut().enumerate() {
                    let color_p = p + i * 4;
                    color.x = b[color_p] as f32 / 255.0;
                    color.y = b[color_p + 1] as f32 / 255.0;
                    color.z = b[color_p + 2] as f32 / 255.0;
                    local_alphas[i] = b[color_p + 3] as f32 / 255.0;
                }
                p += vert_count * 4;
            }
            let mut local_uv_sets = Vec::with_capacity(uv_count as usize);
            if uv_count > 0 && p + uv_count as usize * vert_count * 8 <= ce {
                for set_index in 0..uv_count as usize {
                    let set_start = p + set_index * vert_count * 8;
                    let mut local_uvs = Vec::with_capacity(vert_count);
                    for i in 0..vert_count {
                        local_uvs.push(V2 {
                            u: rdf32(b, set_start + i * 8),
                            v: rdf32(b, set_start + i * 8 + 4),
                        });
                    }
                    local_uv_sets.push(local_uvs);
                }
            }
            p += uv_count as usize * vert_count * 8;
            if p + tri_count * 8 > ce {
                return false;
            }
            let mut local_tris: Vec<Tri> = Vec::with_capacity(tri_count);
            for i in 0..tri_count {
                let bidx = rd16(b, p + i * 8);
                let a = rd16(b, p + i * 8 + 2);
                let material = rd16(b, p + i * 8 + 4);
                let c = rd16(b, p + i * 8 + 6);
                if (a as usize) < vert_count
                    && (bidx as usize) < vert_count
                    && (c as usize) < vert_count
                {
                    local_tris.push(Tri {
                        a: a as u32,
                        b: bidx as u32,
                        c: c as u32,
                        material,
                    });
                }
            }
            p += tri_count * 8;
            if p + 24 > ce {
                return false;
            }
            p += 16;
            let has_verts = rd32(b, p);
            p += 4;
            let has_normals = rd32(b, p);
            p += 4;
            if has_verts == 0 || p + vert_count * 12 > ce {
                return false;
            }
            let mut local_verts: Vec<V3> = Vec::with_capacity(vert_count);
            for i in 0..vert_count {
                local_verts.push(V3 {
                    x: rdf32(b, p + i * 12),
                    y: rdf32(b, p + i * 12 + 4),
                    z: rdf32(b, p + i * 12 + 8),
                });
            }
            p += vert_count * 12;
            let mut local_normals: Vec<V3> = Vec::new();
            if has_normals != 0 && p + vert_count * 12 <= ce {
                local_normals.reserve(vert_count);
                for i in 0..vert_count {
                    let mut n = V3 {
                        x: rdf32(b, p + i * 12),
                        y: rdf32(b, p + i * 12 + 4),
                        z: rdf32(b, p + i * 12 + 8),
                    };
                    let len = (n.x * n.x + n.y * n.y + n.z * n.z).sqrt();
                    if len > 0.0001 {
                        n.x /= len;
                        n.y /= len;
                        n.z /= len;
                    }
                    local_normals.push(n);
                }
            }
            parsed_geometry = Some((
                vert_count,
                local_uv_sets,
                local_colors,
                local_alphas,
                local_tris,
                local_verts,
                local_normals,
                geometry_lit,
            ));
        }
        o = ce;
    }
    if let Some((
        _vert_count,
        mut local_uv_sets,
        mut local_colors,
        mut local_alphas,
        mut local_tris,
        mut local_verts,
        mut local_normals,
        geometry_lit,
    )) = parsed_geometry
    {
        if let Some(tris) = bin_mesh_tris {
            local_tris = tris;
        }
        let tri_start = mesh.triangles.len();
        if let Some(breakable) = breakable.as_mut() {
            map_breakable_source_faces(breakable, &local_verts, &local_tris, tri_start);
        }
        let material_base = merge_geometry_materials(mesh, geometry_materials);
        for tri in &mut local_tris {
            tri.a = tri.a.saturating_add(vertex_base);
            tri.b = tri.b.saturating_add(vertex_base);
            tri.c = tri.c.saturating_add(vertex_base);
            tri.material = (tri.material as u32)
                .saturating_add(material_base)
                .min(u16::MAX as u32) as u16;
        }
        // Vertex block validated: commit everything together.
        mesh.components.push(RawMeshComponent {
            name: String::new(),
            frame_index: None,
            vertex_start: vertex_base as usize,
            vertex_end: vertex_base as usize + _vert_count,
            tri_start,
            tri_end: tri_start + local_tris.len(),
            breakable,
        });
        let mut local_primary_uvs = if local_uv_sets.is_empty() {
            Vec::new()
        } else {
            local_uv_sets.remove(0)
        };
        if mesh.uvs.len() == vertex_base as usize {
            if local_primary_uvs.len() == _vert_count {
                mesh.uvs.append(&mut local_primary_uvs);
            } else if !mesh.uvs.is_empty() {
                mesh.uvs
                    .extend(std::iter::repeat(V2::default()).take(_vert_count));
            }
        } else if local_primary_uvs.len() == _vert_count {
            mesh.uvs.resize(vertex_base as usize, V2::default());
            mesh.uvs.append(&mut local_primary_uvs);
        }
        let secondary_set_count = mesh.secondary_uvs.len().max(local_uv_sets.len());
        mesh.secondary_uvs.resize_with(secondary_set_count, || {
            vec![V2::default(); vertex_base as usize]
        });
        for set_index in 0..secondary_set_count {
            if let Some(local_set) = local_uv_sets.get_mut(set_index) {
                mesh.secondary_uvs[set_index].append(local_set);
            } else {
                mesh.secondary_uvs[set_index]
                    .extend(std::iter::repeat(V2::default()).take(_vert_count));
            }
        }
        if let Some((mut night_colors, mut night_alphas)) = night_colors {
            mesh.night_prelit_colors.append(&mut night_colors);
            mesh.night_prelit_alphas.append(&mut night_alphas);
        } else {
            mesh.night_prelit_colors
                .extend(local_colors.iter().copied());
            mesh.night_prelit_alphas
                .extend(local_alphas.iter().copied());
        }
        mesh.prelit_colors.append(&mut local_colors);
        mesh.prelit_alphas.append(&mut local_alphas);
        mesh.triangles.append(&mut local_tris);
        mesh.vertices.append(&mut local_verts);
        mesh.normals.append(&mut local_normals);
        mesh.light_flags
            .extend(std::iter::repeat(geometry_lit).take(_vert_count));
        true
    } else {
        false
    }
}

fn scan_dff_chunks(b: &[u8], start: usize, end: usize, mesh: &mut RawMesh) {
    let mut o = start;
    while o + 12 <= end {
        let id = rd32(b, o);
        let size = rd32(b, o + 4) as usize;
        let cs = o + 12;
        let ce = cs.saturating_add(size);
        if ce > end || ce > b.len() {
            return;
        }
        if id == 0x0f {
            parse_geometry_chunk(b, cs, ce, mesh);
        }
        if id != 0x01 {
            scan_dff_chunks(b, cs, ce, mesh);
        }
        o = ce;
    }
}

fn geometry_material_count(b: &[u8], start: usize, end: usize) -> Result<usize, String> {
    let mut o = start;
    while o + 12 <= end {
        let id = rd32(b, o);
        let size = rd32(b, o + 4) as usize;
        let cs = o + 12;
        let ce = cs
            .checked_add(size)
            .filter(|ce| *ce <= end && *ce <= b.len())
            .ok_or_else(|| "DFF contains a truncated RenderWare chunk".to_string())?;
        if id == 0x08 {
            let mut material_chunk = cs;
            while material_chunk + 12 <= ce {
                let child_id = rd32(b, material_chunk);
                let child_size = rd32(b, material_chunk + 4) as usize;
                let child_start = material_chunk + 12;
                let child_end = child_start
                    .checked_add(child_size)
                    .filter(|child_end| *child_end <= ce)
                    .ok_or_else(|| "DFF contains a truncated material list".to_string())?;
                if child_id == 0x01 {
                    if child_size < 4 {
                        return Err("DFF material-list struct is truncated".to_string());
                    }
                    return Ok(rd32(b, child_start) as usize);
                }
                material_chunk = child_end;
            }
        }
        o = ce;
    }
    Ok(0)
}

fn scan_dff_material_counts(
    b: &[u8],
    start: usize,
    end: usize,
    maximum: &mut usize,
) -> Result<(), String> {
    let mut o = start;
    while o + 12 <= end {
        let id = rd32(b, o);
        let size = rd32(b, o + 4) as usize;
        let cs = o + 12;
        let ce = cs
            .checked_add(size)
            .filter(|ce| *ce <= end && *ce <= b.len())
            .ok_or_else(|| "DFF contains a truncated RenderWare chunk".to_string())?;
        if id == 0x0f {
            *maximum = (*maximum).max(geometry_material_count(b, cs, ce)?);
        } else if matches!(id, 0x10 | 0x1a) {
            // Only Clump and Geometry List are containers on the path to a
            // Geometry. Extension/plugin payloads are arbitrary binary data,
            // not necessarily nested RenderWare chunks. Recursing into them
            // made valid DragonFF files look truncated and silently excluded
            // them from the material-limit scan.
            scan_dff_material_counts(b, cs, ce, maximum)?;
        }
        o = ce;
    }
    Ok(())
}

pub(crate) fn max_dff_geometry_material_count(bytes: &[u8]) -> Result<usize, String> {
    if bytes.len() < 12 {
        return Err("DFF is too short to contain a RenderWare chunk".to_string());
    }
    let mut maximum = 0;
    scan_dff_material_counts(bytes, 0, bytes.len(), &mut maximum)?;
    Ok(maximum)
}

#[derive(Clone, Copy)]
struct DffFrameTransform {
    right: V3,
    up: V3,
    at: V3,
    pos: V3,
}

impl DffFrameTransform {
    fn identity() -> Self {
        Self {
            right: V3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            up: V3 {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            at: V3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
            pos: V3::default(),
        }
    }

    fn transform_point(self, point: V3) -> V3 {
        V3 {
            x: self.right.x * point.x + self.up.x * point.y + self.at.x * point.z + self.pos.x,
            y: self.right.y * point.x + self.up.y * point.y + self.at.y * point.z + self.pos.y,
            z: self.right.z * point.x + self.up.z * point.y + self.at.z * point.z + self.pos.z,
        }
    }

    fn transform_vector(self, vector: V3) -> V3 {
        V3 {
            x: self.right.x * vector.x + self.up.x * vector.y + self.at.x * vector.z,
            y: self.right.y * vector.x + self.up.y * vector.y + self.at.y * vector.z,
            z: self.right.z * vector.x + self.up.z * vector.y + self.at.z * vector.z,
        }
    }

    fn then(self, child: Self) -> Self {
        Self {
            right: self.transform_vector(child.right),
            up: self.transform_vector(child.up),
            at: self.transform_vector(child.at),
            pos: self.transform_point(child.pos),
        }
    }
}

#[derive(Clone)]
struct DffFrameInfo {
    name: String,
    parent: i32,
    local: DffFrameTransform,
}

fn normalized_v3(value: V3) -> V3 {
    let len = (value.x * value.x + value.y * value.y + value.z * value.z).sqrt();
    if len > 0.0001 {
        V3 {
            x: value.x / len,
            y: value.y / len,
            z: value.z / len,
        }
    } else {
        value
    }
}

fn parse_frame_list_frames(b: &[u8], start: usize, end: usize, frames: &mut Vec<DffFrameInfo>) {
    let mut parsed_frames = Vec::<DffFrameInfo>::new();
    let mut names = Vec::<String>::new();
    let mut o = start;
    while o + 12 <= end {
        let id = rd32(b, o);
        let size = rd32(b, o + 4) as usize;
        let cs = o + 12;
        let ce = cs.saturating_add(size);
        if ce > end || ce > b.len() {
            return;
        }
        if id == 0x01 && size >= 4 {
            let frame_count = rd32(b, cs) as usize;
            let mut p = cs + 4;
            for _ in 0..frame_count {
                if p + 56 > ce {
                    break;
                }
                parsed_frames.push(DffFrameInfo {
                    name: String::new(),
                    parent: rd32(b, p + 48) as i32,
                    local: DffFrameTransform {
                        right: V3 {
                            x: rdf32(b, p),
                            y: rdf32(b, p + 4),
                            z: rdf32(b, p + 8),
                        },
                        up: V3 {
                            x: rdf32(b, p + 12),
                            y: rdf32(b, p + 16),
                            z: rdf32(b, p + 20),
                        },
                        at: V3 {
                            x: rdf32(b, p + 24),
                            y: rdf32(b, p + 28),
                            z: rdf32(b, p + 32),
                        },
                        pos: V3 {
                            x: rdf32(b, p + 36),
                            y: rdf32(b, p + 40),
                            z: rdf32(b, p + 44),
                        },
                    },
                });
                p += 56;
            }
        } else if id == 0x03 {
            let mut name = String::new();
            let mut ext = cs;
            while ext + 12 <= ce {
                let ext_id = rd32(b, ext);
                let ext_size = rd32(b, ext + 4) as usize;
                let ext_start = ext + 12;
                let ext_end = ext_start.saturating_add(ext_size);
                if ext_end > ce || ext_end > b.len() {
                    break;
                }
                if ext_id == 0x0253_f2fe {
                    name = parse_fixed_name(&b[ext_start..ext_end]);
                }
                ext = ext_end;
            }
            names.push(name);
        }
        o = ce;
    }
    for (idx, name) in names.into_iter().enumerate() {
        if let Some(frame) = parsed_frames.get_mut(idx) {
            frame.name = name;
        }
    }
    frames.extend(parsed_frames);
}

/// Walk the DFF and collect frame names plus atomic (frame, geometry) pairs so
/// flattened geometries can be labelled with the component they came from.
fn scan_dff_structure(
    b: &[u8],
    start: usize,
    end: usize,
    frames: &mut Vec<DffFrameInfo>,
    atomics: &mut Vec<(u32, u32)>,
) {
    let mut o = start;
    while o + 12 <= end {
        let id = rd32(b, o);
        let size = rd32(b, o + 4) as usize;
        let cs = o + 12;
        let ce = cs.saturating_add(size);
        if ce > end || ce > b.len() {
            return;
        }
        if id == 0x0e {
            parse_frame_list_frames(b, cs, ce, frames);
        } else if id == 0x14 {
            // Atomic struct: frame index, geometry index, flags, unused.
            let mut child = cs;
            while child + 12 <= ce {
                let child_id = rd32(b, child);
                let child_size = rd32(b, child + 4) as usize;
                let child_start = child + 12;
                let child_end = child_start.saturating_add(child_size);
                if child_end > ce || child_end > b.len() {
                    break;
                }
                if child_id == 0x01 && child_size >= 8 {
                    atomics.push((rd32(b, child_start), rd32(b, child_start + 4)));
                    break;
                }
                child = child_end;
            }
            scan_dff_structure(b, cs, ce, frames, atomics);
        } else if id != 0x01 {
            scan_dff_structure(b, cs, ce, frames, atomics);
        }
        o = ce;
    }
}

/// Resolve a frame into the model entity's coordinate space.
///
/// GTA installs the entity matrix on the clump's root frame when it creates a
/// world object, replacing the root matrix stored in the DFF. Child matrices
/// remain relative to that root and must still be accumulated. Including an
/// authored root translation here made Eagle move static geometry away from
/// its placement pivot even though GTA renders it at the pivot.
fn entity_frame_transform(
    frames: &[DffFrameInfo],
    frame_idx: usize,
    visiting: &mut Vec<bool>,
    cached: &mut Vec<Option<DffFrameTransform>>,
) -> Option<DffFrameTransform> {
    if frame_idx >= frames.len() {
        return None;
    }
    if let Some(transform) = cached[frame_idx] {
        return Some(transform);
    }
    if visiting[frame_idx] {
        return Some(frames[frame_idx].local);
    }
    visiting[frame_idx] = true;
    let local = frames[frame_idx].local;
    let world = if frames[frame_idx].parent >= 0 {
        entity_frame_transform(frames, frames[frame_idx].parent as usize, visiting, cached)
            .map(|parent| parent.then(local))
            .unwrap_or(local)
    } else {
        DffFrameTransform::identity()
    };
    visiting[frame_idx] = false;
    cached[frame_idx] = Some(world);
    Some(world)
}

fn assign_component_names_and_transforms(bytes: &[u8], mesh: &mut RawMesh) {
    if mesh.components.is_empty() {
        return;
    }
    let mut frames = Vec::new();
    let mut atomics = Vec::new();
    scan_dff_structure(bytes, 0, bytes.len(), &mut frames, &mut atomics);
    let mut visiting = vec![false; frames.len()];
    let mut cached = vec![None; frames.len()];
    mesh.frames = (0..frames.len())
        .filter_map(|frame_idx| {
            let transform = entity_frame_transform(&frames, frame_idx, &mut visiting, &mut cached)?;
            Some(RawMeshFrame {
                name: frames[frame_idx].name.clone(),
                parent: frames[frame_idx].parent,
                right: transform.right,
                up: transform.up,
                at: transform.at,
                pos: transform.pos,
            })
        })
        .collect();
    for (geometry_idx, component) in mesh.components.iter_mut().enumerate() {
        // Prefer the atomic that references this geometry index; fall back to
        // pairing atomics with geometries in encounter order (embedded
        // geometry DFFs have no geometry list indices).
        let frame_idx = atomics
            .iter()
            .find(|(_, geometry)| *geometry as usize == geometry_idx)
            .map(|(frame, _)| *frame)
            .or_else(|| atomics.get(geometry_idx).map(|(frame, _)| *frame));
        if let Some(frame_idx) = frame_idx {
            let frame_idx = frame_idx as usize;
            if let Some(frame) = frames.get(frame_idx) {
                component.name = frame.name.clone();
                component.frame_index = Some(frame_idx);
            }
            if let Some(transform) =
                entity_frame_transform(&frames, frame_idx, &mut visiting, &mut cached)
            {
                for vertex in component.vertex_start..component.vertex_end.min(mesh.vertices.len())
                {
                    mesh.vertices[vertex] = transform.transform_point(mesh.vertices[vertex]);
                }
                for normal in component.vertex_start..component.vertex_end.min(mesh.normals.len()) {
                    mesh.normals[normal] =
                        normalized_v3(transform.transform_vector(mesh.normals[normal]));
                }
            }
        }
    }
}

/// Parses a DFF without welding or otherwise changing its source vertex
/// indices. Repair operations that map geometry-local sidecar streams must use
/// this form so their per-geometry vertex ranges still match the file.
pub(crate) fn parse_dff_mesh_preserving_topology(bytes: &[u8]) -> RawMesh {
    let mut mesh = RawMesh::default();
    push_top_level_uv_anim_dictionaries(bytes, &mut mesh);
    scan_dff_chunks(bytes, 0, bytes.len(), &mut mesh);
    if mesh.material_animations.len() < mesh.material_textures.len() {
        mesh.material_animations
            .resize_with(mesh.material_textures.len(), DffMaterialAnim::default);
    }
    if mesh.materials.len() < mesh.material_textures.len() {
        mesh.materials
            .resize_with(mesh.material_textures.len(), || RawMaterial {
                color: neutral_vertex_color(),
                alpha: 1.0,
                ambient: 1.0,
                specular: 1.0,
                diffuse: 1.0,
            });
    }
    assign_component_names_and_transforms(bytes, &mut mesh);
    mesh
}

pub(crate) fn parse_dff_mesh(bytes: &[u8]) -> RawMesh {
    let mut mesh = parse_dff_mesh_preserving_topology(bytes);
    normalize_imported_mesh(&mut mesh);
    mesh
}

fn mesh_pos_key(v: V3) -> (i32, i32, i32) {
    const SCALE: f32 = 1000.0;
    (
        (v.x * SCALE).round() as i32,
        (v.y * SCALE).round() as i32,
        (v.z * SCALE).round() as i32,
    )
}

fn normalize_imported_mesh(mesh: &mut RawMesh) {
    // A complete DFF normal stream is authored asset data. In particular,
    // DragonFF represents Blender's per-loop/split normals by duplicating DFF
    // vertices at discontinuities. Reconstructing normals here discarded that
    // information and made sharp edges and weighted/custom normals appear
    // smoothed in the editor (and in any later normalized rewrite).
    if mesh.normals.len() != mesh.vertices.len() {
        smooth_imported_normals(mesh);
    }
    weld_matching_import_vertices(mesh);
}

fn smooth_imported_normals(mesh: &mut RawMesh) {
    if mesh.vertices.is_empty() || mesh.triangles.is_empty() {
        return;
    }
    const SHARP_DOT: f32 = 0.25;
    let source_normals = (mesh.normals.len() == mesh.vertices.len()).then(|| mesh.normals.clone());
    let mut corners_by_pos = HashMap::<(i32, i32, i32), Vec<(usize, Vec3, u16)>>::new();
    for tri in &mesh.triangles {
        let indices = [tri.a as usize, tri.b as usize, tri.c as usize];
        if indices.iter().any(|idx| *idx >= mesh.vertices.len()) {
            continue;
        }
        let pa = to_mq(mesh.vertices[indices[0]]);
        let pb = to_mq(mesh.vertices[indices[1]]);
        let pc = to_mq(mesh.vertices[indices[2]]);
        // DFF triangles are converted to the editor's RenderWare/CW order
        // during parsing (and rendered with GL_CW as the front face). The
        // usual (B-A)x(C-A) formula is for CCW triangles and points through
        // the back of these faces. Reverse the operands so reconstructed
        // normals point through the visible RenderWare face.
        let face = (pc - pa).cross(pb - pa);
        if face.length_squared() < 0.000001 {
            continue;
        }
        let face_normal = face.normalize();
        for (slot, idx) in indices.into_iter().enumerate() {
            let p = to_mq(mesh.vertices[idx]);
            let p1 = to_mq(mesh.vertices[indices[(slot + 1) % 3]]);
            let p2 = to_mq(mesh.vertices[indices[(slot + 2) % 3]]);
            let angle = (p1 - p)
                .normalize_or_zero()
                .dot((p2 - p).normalize_or_zero())
                .clamp(-1.0, 1.0)
                .acos()
                .max(0.0001);
            corners_by_pos
                .entry(mesh_pos_key(mesh.vertices[idx]))
                .or_default()
                .push((idx, face_normal * angle, tri.material));
        }
    }

    let mut normal_sums = vec![Vec3::ZERO; mesh.vertices.len()];
    for corners in corners_by_pos.values() {
        let face_normals: Vec<Vec3> = corners
            .iter()
            .map(|(_, weighted, _)| weighted.normalize_or_zero())
            .collect();
        let mut assigned = vec![false; corners.len()];
        for start in 0..corners.len() {
            if assigned[start] || face_normals[start].length_squared() < 0.0001 {
                continue;
            }
            let material = corners[start].2;
            let mut cluster_indices = Vec::new();
            let mut queue = vec![start];
            assigned[start] = true;
            while let Some(current) = queue.pop() {
                cluster_indices.push(current);
                let current_normal = face_normals[current];
                for next in 0..corners.len() {
                    if assigned[next]
                        || corners[next].2 != material
                        || face_normals[next].length_squared() < 0.0001
                    {
                        continue;
                    }
                    if current_normal.dot(face_normals[next]) >= SHARP_DOT {
                        assigned[next] = true;
                        queue.push(next);
                    }
                }
            }
            let cluster = cluster_indices
                .iter()
                .fold(Vec3::ZERO, |sum, corner_idx| sum + corners[*corner_idx].1);
            for corner_idx in cluster_indices {
                normal_sums[corners[corner_idx].0] += cluster;
            }
        }
        for (corner_idx, (idx, weighted, _)) in corners.iter().enumerate() {
            if !assigned[corner_idx] {
                normal_sums[*idx] += *weighted;
            }
        }
    }

    mesh.normals = normal_sums
        .into_iter()
        .enumerate()
        .map(|(idx, sum)| {
            let mut n = sum.normalize_or_zero();
            if n.length_squared() < 0.0001 {
                return source_normals
                    .as_ref()
                    .and_then(|normals| normals.get(idx))
                    .copied()
                    .unwrap_or(V3 {
                        x: 0.0,
                        y: 0.0,
                        z: 1.0,
                    });
            }
            if let Some(source) = source_normals
                .as_ref()
                .and_then(|normals| normals.get(idx))
                .map(|normal| to_mq(*normal).normalize_or_zero())
            {
                if source.length_squared() > 0.0001 && n.dot(source) < 0.0 {
                    n = -n;
                }
            }
            from_mq(n)
        })
        .collect();
}

fn weld_matching_import_vertices(mesh: &mut RawMesh) {
    if mesh.vertices.is_empty() || mesh.triangles.is_empty() {
        return;
    }
    let has_normals = mesh.normals.len() == mesh.vertices.len();
    let has_uvs = mesh.uvs.len() == mesh.vertices.len();
    let secondary_uv_sets = mesh
        .secondary_uvs
        .iter()
        .filter(|uvs| uvs.len() == mesh.vertices.len())
        .cloned()
        .collect::<Vec<_>>();
    let has_prelit = mesh.prelit_colors.len() == mesh.vertices.len();
    let has_prelit_alpha = mesh.prelit_alphas.len() == mesh.vertices.len();
    let has_night_prelit = mesh.night_prelit_colors.len() == mesh.vertices.len();
    let has_night_prelit_alpha = mesh.night_prelit_alphas.len() == mesh.vertices.len();
    let has_light_flags = mesh.light_flags.len() == mesh.vertices.len();
    let mut remap = BTreeMap::<Vec<i32>, u32>::new();
    let mut index_map = vec![0u32; mesh.vertices.len()];
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut secondary_uvs = vec![Vec::new(); secondary_uv_sets.len()];
    let mut prelit_colors = Vec::new();
    let mut prelit_alphas = Vec::new();
    let mut night_prelit_colors = Vec::new();
    let mut night_prelit_alphas = Vec::new();
    let mut light_flags = Vec::new();

    for idx in 0..mesh.vertices.len() {
        let v = mesh.vertices[idx];
        let uv = if has_uvs {
            mesh.uvs[idx]
        } else {
            V2::default()
        };
        let color = if has_prelit {
            mesh.prelit_colors[idx]
        } else {
            V3::default()
        };
        let night_color = if has_night_prelit {
            mesh.night_prelit_colors[idx]
        } else {
            color
        };
        let normal = if has_normals {
            mesh.normals[idx]
        } else {
            V3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            }
        };
        let mut key = vec![
            (v.x * 1000.0).round() as i32,
            (v.y * 1000.0).round() as i32,
            (v.z * 1000.0).round() as i32,
            (uv.u * 100000.0).round() as i32,
            (uv.v * 100000.0).round() as i32,
            (color.x * 255.0).round() as i32,
            (color.y * 255.0).round() as i32,
            (color.z * 255.0).round() as i32,
            (night_color.x * 255.0).round() as i32,
            (night_color.y * 255.0).round() as i32,
            (night_color.z * 255.0).round() as i32,
            (normal.x * 1000.0).round() as i32,
            (normal.y * 1000.0).round() as i32,
            (normal.z * 1000.0).round() as i32,
        ];
        for set in &secondary_uv_sets {
            key.extend([
                (set[idx].u * 100000.0).round() as i32,
                (set[idx].v * 100000.0).round() as i32,
            ]);
        }
        if has_prelit_alpha {
            key.push((mesh.prelit_alphas[idx] * 255.0).round() as i32);
        }
        if has_night_prelit_alpha {
            key.push((mesh.night_prelit_alphas[idx] * 255.0).round() as i32);
        }
        if let Some(mapped) = remap.get(&key).copied() {
            index_map[idx] = mapped;
            continue;
        }
        let mapped = vertices.len() as u32;
        remap.insert(key, mapped);
        index_map[idx] = mapped;
        vertices.push(v);
        if has_normals {
            normals.push(mesh.normals[idx]);
        }
        if has_uvs {
            uvs.push(uv);
        }
        for (out, set) in secondary_uvs.iter_mut().zip(&secondary_uv_sets) {
            out.push(set[idx]);
        }
        if has_prelit {
            prelit_colors.push(color);
        }
        if has_prelit_alpha {
            prelit_alphas.push(mesh.prelit_alphas[idx]);
        }
        if has_night_prelit {
            night_prelit_colors.push(night_color);
        }
        if has_night_prelit_alpha {
            night_prelit_alphas.push(mesh.night_prelit_alphas[idx]);
        }
        if has_light_flags {
            light_flags.push(mesh.light_flags[idx]);
        }
    }

    for tri in &mut mesh.triangles {
        tri.a = index_map.get(tri.a as usize).copied().unwrap_or(tri.a);
        tri.b = index_map.get(tri.b as usize).copied().unwrap_or(tri.b);
        tri.c = index_map.get(tri.c as usize).copied().unwrap_or(tri.c);
    }
    // Drop triangles made degenerate by the weld while keeping the recorded
    // component triangle ranges in sync with the compacted list.
    let tri_count = mesh.triangles.len();
    let mut removed_before = vec![0usize; tri_count + 1];
    let mut kept = Vec::with_capacity(tri_count);
    let mut removed = 0usize;
    for (idx, tri) in mesh.triangles.iter().enumerate() {
        removed_before[idx] = removed;
        if tri.a != tri.b && tri.b != tri.c && tri.a != tri.c {
            kept.push(*tri);
        } else {
            removed += 1;
        }
    }
    removed_before[tri_count] = removed;
    for component in &mut mesh.components {
        let mapped_vertices = (component.vertex_start..component.vertex_end.min(index_map.len()))
            .filter_map(|idx| index_map.get(idx).copied().map(|mapped| mapped as usize))
            .collect::<Vec<_>>();
        if let (Some(min), Some(max)) = (mapped_vertices.iter().min(), mapped_vertices.iter().max())
        {
            component.vertex_start = *min;
            component.vertex_end = max.saturating_add(1);
        } else {
            component.vertex_start = 0;
            component.vertex_end = 0;
        }
        component.tri_start = component
            .tri_start
            .min(tri_count)
            .saturating_sub(removed_before[component.tri_start.min(tri_count)]);
        component.tri_end = component
            .tri_end
            .min(tri_count)
            .saturating_sub(removed_before[component.tri_end.min(tri_count)]);
    }
    mesh.triangles = kept;
    mesh.vertices = vertices;
    if has_normals {
        mesh.normals = normals;
    }
    if has_uvs {
        mesh.uvs = uvs;
    }
    mesh.secondary_uvs = secondary_uvs;
    if has_prelit {
        mesh.prelit_colors = prelit_colors;
    }
    if has_prelit_alpha {
        mesh.prelit_alphas = prelit_alphas;
    }
    if has_night_prelit {
        mesh.night_prelit_colors = night_prelit_colors;
    }
    if has_night_prelit_alpha {
        mesh.night_prelit_alphas = night_prelit_alphas;
    }
    if has_light_flags {
        mesh.light_flags = light_flags;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(id: u32, data: Vec<u8>) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&id.to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&0x1803_ffffu32.to_le_bytes());
        out.extend_from_slice(&data);
        out
    }

    fn push_f32(out: &mut Vec<u8>, value: f32) {
        out.extend_from_slice(&value.to_le_bytes());
    }

    fn color(r: u8, g: u8, b: u8) -> V3 {
        V3 {
            x: r as f32 / 255.0,
            y: g as f32 / 255.0,
            z: b as f32 / 255.0,
        }
    }

    fn dff_with_material_count(count: usize) -> Vec<u8> {
        let mut material_list_struct = (count as u32).to_le_bytes().to_vec();
        for _ in 0..count {
            material_list_struct.extend_from_slice(&(-1i32).to_le_bytes());
        }
        let material_list = chunk(0x08, chunk(0x01, material_list_struct));
        chunk(0x10, chunk(0x0f, material_list))
    }

    #[test]
    fn reads_maximum_material_count_per_dff_geometry() {
        assert_eq!(
            max_dff_geometry_material_count(&dff_with_material_count(152)).unwrap(),
            152
        );
        assert_eq!(
            max_dff_geometry_material_count(&dff_with_material_count(153)).unwrap(),
            153
        );
    }

    #[test]
    fn material_count_ignores_non_chunk_plugin_payloads() {
        let mut clump = chunk(0x03, vec![0xff; 31]);
        clump.extend_from_slice(&chunk(
            0x1a,
            chunk(
                0x0f,
                chunk(0x08, chunk(0x01, 153u32.to_le_bytes().to_vec())),
            ),
        ));

        assert_eq!(
            max_dff_geometry_material_count(&chunk(0x10, clump)),
            Ok(153)
        );
    }

    #[test]
    fn material_surface_properties_are_parsed_for_game_preview() {
        let mut data = vec![0u8; 16];
        data[4..8].copy_from_slice(&[128, 192, 255, 224]);
        for value in [0.75f32, 0.25, 0.5] {
            push_f32(&mut data, value);
        }
        let bytes = chunk(0x01, data);

        let (_, material, _) = parse_material_chunk(&bytes, 0, bytes.len());

        assert!((material.color.x - 128.0 / 255.0).abs() < 0.0001);
        assert!((material.alpha - 224.0 / 255.0).abs() < 0.0001);
        assert!((material.ambient - 0.75).abs() < 0.0001);
        assert!((material.specular - 0.25).abs() < 0.0001);
        assert!((material.diffuse - 0.5).abs() < 0.0001);
    }

    #[test]
    fn normalize_imported_mesh_preserves_complete_authored_normals() {
        let authored = V3 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        };
        let mut mesh = RawMesh {
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
            normals: vec![authored; 3],
            triangles: vec![Tri {
                a: 0,
                b: 1,
                c: 2,
                material: 0,
            }],
            ..RawMesh::default()
        };

        normalize_imported_mesh(&mut mesh);

        assert_eq!(mesh.normals, vec![authored; 3]);
    }

    #[test]
    fn normalize_imported_mesh_reconstructs_an_incomplete_normal_stream() {
        let mut mesh = RawMesh {
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
            normals: vec![V3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            }],
            triangles: vec![Tri {
                a: 0,
                b: 1,
                c: 2,
                material: 0,
            }],
            ..RawMesh::default()
        };

        normalize_imported_mesh(&mut mesh);

        assert_eq!(mesh.normals.len(), mesh.vertices.len());
        assert!(mesh.normals.iter().all(|normal| normal.z.abs() > 0.999));
    }

    #[test]
    fn parse_dff_mesh_assigns_day_and_night_prelight_streams() {
        let day = [(255, 0, 0), (0, 255, 0), (0, 0, 255)];
        let day_alpha = [32u8, 128, 224];
        let night = [(16, 32, 48), (64, 80, 96), (112, 128, 144)];
        let night_alpha = [48u8, 144, 240];

        let mut geometry_struct = Vec::new();
        geometry_struct.extend_from_slice(&0x68u32.to_le_bytes());
        geometry_struct.extend_from_slice(&1u32.to_le_bytes());
        geometry_struct.extend_from_slice(&3u32.to_le_bytes());
        geometry_struct.extend_from_slice(&1u32.to_le_bytes());
        for ((r, g, b), alpha) in day.into_iter().zip(day_alpha) {
            geometry_struct.extend_from_slice(&[r, g, b, alpha]);
        }
        for value in [1u16, 0, 0, 2] {
            geometry_struct.extend_from_slice(&value.to_le_bytes());
        }
        for value in [0.0f32, 0.0, 0.0, 1.0] {
            push_f32(&mut geometry_struct, value);
        }
        geometry_struct.extend_from_slice(&1u32.to_le_bytes());
        geometry_struct.extend_from_slice(&0u32.to_le_bytes());
        for vertex in [
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
        ] {
            for value in [vertex.x, vertex.y, vertex.z] {
                push_f32(&mut geometry_struct, value);
            }
        }

        let mut night_data = 1u32.to_le_bytes().to_vec();
        for ((r, g, b), alpha) in night.into_iter().zip(night_alpha) {
            night_data.extend_from_slice(&[r, g, b, alpha]);
        }
        let geometry = [
            chunk(0x01, geometry_struct),
            chunk(0x03, chunk(0x0253_f2f9, night_data)),
        ]
        .concat();

        let raw = parse_dff_mesh(&chunk(0x0f, geometry));

        assert_eq!(raw.vertices.len(), 3);
        assert_eq!(raw.prelit_colors, day.map(|(r, g, b)| color(r, g, b)));
        assert_eq!(
            raw.prelit_alphas,
            day_alpha.map(|alpha| alpha as f32 / 255.0)
        );
        assert_eq!(
            raw.night_prelit_colors,
            night.map(|(r, g, b)| color(r, g, b))
        );
        assert_eq!(
            raw.night_prelit_alphas,
            night_alpha.map(|alpha| alpha as f32 / 255.0)
        );
    }

    fn identity_frame(parent: i32, pos: V3) -> Vec<u8> {
        let mut out = Vec::new();
        for value in [1.0f32, 0.0, 0.0] {
            push_f32(&mut out, value);
        }
        for value in [0.0f32, 1.0, 0.0] {
            push_f32(&mut out, value);
        }
        for value in [0.0f32, 0.0, 1.0] {
            push_f32(&mut out, value);
        }
        for value in [pos.x, pos.y, pos.z] {
            push_f32(&mut out, value);
        }
        out.extend_from_slice(&parent.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out
    }

    fn frame_name_extension(name: &str) -> Vec<u8> {
        chunk(0x03, chunk(0x0253_f2fe, name.as_bytes().to_vec()))
    }

    fn simple_triangle_geometry() -> Vec<u8> {
        let mut geometry_struct = Vec::new();
        geometry_struct.extend_from_slice(&0u32.to_le_bytes());
        geometry_struct.extend_from_slice(&1u32.to_le_bytes());
        geometry_struct.extend_from_slice(&3u32.to_le_bytes());
        geometry_struct.extend_from_slice(&1u32.to_le_bytes());
        for value in [1u16, 0, 0, 2] {
            geometry_struct.extend_from_slice(&value.to_le_bytes());
        }
        for value in [0.0f32, 0.0, 0.0, 1.0] {
            push_f32(&mut geometry_struct, value);
        }
        geometry_struct.extend_from_slice(&1u32.to_le_bytes());
        geometry_struct.extend_from_slice(&0u32.to_le_bytes());
        for vertex in [
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
        ] {
            for value in [vertex.x, vertex.y, vertex.z] {
                push_f32(&mut geometry_struct, value);
            }
        }
        chunk(0x0f, chunk(0x01, geometry_struct))
    }

    #[test]
    fn parse_dff_mesh_ignores_entity_root_but_applies_atomic_child_offset() {
        let ignored_root_pos = V3 {
            x: 20.0,
            y: 30.0,
            z: 40.0,
        };
        let wheel_pos = V3 {
            x: 2.0,
            y: 3.0,
            z: 4.0,
        };
        let mut frame_struct = 2u32.to_le_bytes().to_vec();
        frame_struct.extend_from_slice(&identity_frame(-1, ignored_root_pos));
        frame_struct.extend_from_slice(&identity_frame(0, wheel_pos));
        let frame_list = chunk(
            0x0e,
            [
                chunk(0x01, frame_struct),
                frame_name_extension("chassis_dummy"),
                frame_name_extension("wheel_lf_dummy"),
            ]
            .concat(),
        );

        let geometry_list = chunk(
            0x1a,
            [
                chunk(0x01, 1u32.to_le_bytes().to_vec()),
                simple_triangle_geometry(),
            ]
            .concat(),
        );
        let atomic = chunk(
            0x14,
            chunk(
                0x01,
                [
                    1u32.to_le_bytes(),
                    0u32.to_le_bytes(),
                    5u32.to_le_bytes(),
                    0u32.to_le_bytes(),
                ]
                .concat(),
            ),
        );
        let clump = chunk(
            0x10,
            [
                chunk(
                    0x01,
                    [1u32.to_le_bytes(), 1u32.to_le_bytes(), 0u32.to_le_bytes()].concat(),
                ),
                frame_list,
                geometry_list,
                atomic,
            ]
            .concat(),
        );

        let raw = parse_dff_mesh(&clump);

        assert_eq!(
            raw.components
                .first()
                .map(|component| component.name.as_str()),
            Some("wheel_lf_dummy")
        );
        assert!(raw.vertices.iter().any(|vertex| {
            (vertex.x - wheel_pos.x).abs() < 0.001
                && (vertex.y - wheel_pos.y).abs() < 0.001
                && (vertex.z - wheel_pos.z).abs() < 0.001
        }));
        assert_eq!(
            raw.frames.first().map(|frame| frame.pos),
            Some(V3::default())
        );
        assert_eq!(raw.frames.get(1).map(|frame| frame.pos), Some(wheel_pos));
    }

    #[test]
    fn parse_dff_mesh_ignores_root_offset_for_static_atomic() {
        let ignored_root_pos = V3 {
            x: -0.0583408,
            y: -0.00422143,
            z: -1.2912693,
        };
        let mut frame_struct = 1u32.to_le_bytes().to_vec();
        frame_struct.extend_from_slice(&identity_frame(-1, ignored_root_pos));
        let frame_list = chunk(
            0x0e,
            [
                chunk(0x01, frame_struct),
                frame_name_extension("box493_c2q_0"),
            ]
            .concat(),
        );
        let geometry_list = chunk(
            0x1a,
            [
                chunk(0x01, 1u32.to_le_bytes().to_vec()),
                simple_triangle_geometry(),
            ]
            .concat(),
        );
        let atomic = chunk(
            0x14,
            chunk(
                0x01,
                [
                    0u32.to_le_bytes(),
                    0u32.to_le_bytes(),
                    5u32.to_le_bytes(),
                    0u32.to_le_bytes(),
                ]
                .concat(),
            ),
        );
        let clump = chunk(
            0x10,
            [
                chunk(
                    0x01,
                    [1u32.to_le_bytes(), 1u32.to_le_bytes(), 0u32.to_le_bytes()].concat(),
                ),
                frame_list,
                geometry_list,
                atomic,
            ]
            .concat(),
        );

        let raw = parse_dff_mesh(&clump);

        assert!(raw.vertices.iter().any(|vertex| {
            vertex.x.abs() < 0.001 && vertex.y.abs() < 0.001 && vertex.z.abs() < 0.001
        }));
        assert_eq!(
            raw.frames.first().map(|frame| frame.pos),
            Some(V3::default())
        );
    }
}

#[cfg(test)]
mod component_probe_tests {
    use super::*;

    #[test]
    fn vehicle_dff_components_get_frame_names() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/player_vehicle/vehicle.dff"
        );
        let Ok(bytes) = fs::read(path) else {
            return;
        };
        let mesh = parse_dff_mesh(&bytes);
        eprintln!("components: {}", mesh.components.len());
        for c in &mesh.components {
            eprintln!("  '{}' tris {}..{}", c.name, c.tri_start, c.tri_end);
        }
        assert!(!mesh.components.is_empty());
        assert_eq!(
            mesh.components.last().map(|c| c.tri_end),
            Some(mesh.triangles.len())
        );
    }
}
