use super::super::*;

pub(crate) fn dff_chunk_len(bytes: &[u8]) -> usize {
    let mut o = 0usize;
    let mut last_valid = bytes.len();
    while o + 12 <= bytes.len() {
        let id = rd32(bytes, o);
        let size = rd32(bytes, o + 4) as usize;
        if id == 0 && size == 0 {
            break;
        }
        let end = o.saturating_add(12).saturating_add(size);
        if end > bytes.len() {
            break;
        }
        last_valid = end;
        o = end;
        if o == bytes.len() {
            break;
        }
    }
    last_valid
}

/// Rewrite only the first RenderWare string inside each material's texture
/// chunk. The replacement is always shorter, so this preserves every chunk,
/// plugin, and byte offset in the original DFF.
pub(crate) fn rename_dff_material_textures(
    bytes: &mut [u8],
    renames: &HashMap<String, String>,
) -> usize {
    fn patch_texture(
        bytes: &mut [u8],
        start: usize,
        end: usize,
        renames: &HashMap<String, String>,
    ) -> bool {
        let mut chunk = start;
        while chunk + 12 <= end {
            let id = rd32(bytes, chunk);
            let size = rd32(bytes, chunk + 4) as usize;
            let data = chunk + 12;
            let chunk_end = data.saturating_add(size);
            if chunk_end > end || chunk_end > bytes.len() {
                break;
            }
            if id == 0x02 {
                let value_end = bytes[data..chunk_end]
                    .iter()
                    .position(|byte| *byte == 0)
                    .map(|offset| data + offset)
                    .unwrap_or(chunk_end);
                let old = lower(&String::from_utf8_lossy(&bytes[data..value_end]));
                if let Some(new_name) = renames.get(&old) {
                    if new_name.len() <= size {
                        bytes[data..chunk_end].fill(0);
                        bytes[data..data + new_name.len()].copy_from_slice(new_name.as_bytes());
                        return true;
                    }
                }
                return false; // The second string is the mask name.
            }
            chunk = chunk_end;
        }
        false
    }

    fn scan(
        bytes: &mut [u8],
        start: usize,
        end: usize,
        renames: &HashMap<String, String>,
    ) -> usize {
        let mut changed = 0usize;
        let mut chunk = start;
        while chunk + 12 <= end {
            let id = rd32(bytes, chunk);
            let size = rd32(bytes, chunk + 4) as usize;
            let data = chunk + 12;
            let chunk_end = data.saturating_add(size);
            if chunk_end > end || chunk_end > bytes.len() {
                break;
            }
            if id == 0x07 {
                let mut child = data;
                while child + 12 <= chunk_end {
                    let child_id = rd32(bytes, child);
                    let child_size = rd32(bytes, child + 4) as usize;
                    let child_data = child + 12;
                    let child_end = child_data.saturating_add(child_size);
                    if child_end > chunk_end {
                        break;
                    }
                    if child_id == 0x06 && patch_texture(bytes, child_data, child_end, renames) {
                        changed += 1;
                    }
                    child = child_end;
                }
            } else if id != 0x01 {
                changed += scan(bytes, data, chunk_end, renames);
            }
            chunk = chunk_end;
        }
        changed
    }

    let len = dff_chunk_len(bytes);
    scan(bytes, 0, len, renames)
}

/// Rebuild the enclosing RenderWare chunk sizes while renaming material
/// texture strings. Unlike the in-place cleanup helper above, this supports a
/// replacement name that is longer than the original string allocation while
/// preserving every unrelated DFF chunk and plugin byte-for-byte.
pub(crate) fn rewrite_dff_material_textures(
    bytes: &[u8],
    renames: &HashMap<String, String>,
) -> Result<(Vec<u8>, usize), String> {
    fn chunk_end(bytes: &[u8], start: usize, limit: usize) -> Option<usize> {
        (start + 12 <= limit).then_some(())?;
        start
            .checked_add(12)?
            .checked_add(rd32(bytes, start + 4) as usize)
            .filter(|end| *end <= limit && *end <= bytes.len())
    }

    fn rebuild_chunk(header: &[u8], payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(12 + payload.len());
        out.extend_from_slice(&header[..12]);
        out[4..8].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        out.extend_from_slice(payload);
        out
    }

    fn rewrite_texture(
        bytes: &[u8],
        start: usize,
        end: usize,
        renames: &HashMap<String, String>,
    ) -> (Vec<u8>, usize) {
        let mut out = Vec::with_capacity(end.saturating_sub(start));
        let mut chunk = start;
        let mut changed = 0usize;
        let mut saw_string = false;
        while let Some(child_end) = chunk_end(bytes, chunk, end) {
            let id = rd32(bytes, chunk);
            if id == 0x02 && !saw_string {
                saw_string = true;
                let data = chunk + 12;
                let value_end = bytes[data..child_end]
                    .iter()
                    .position(|byte| *byte == 0)
                    .map(|offset| data + offset)
                    .unwrap_or(child_end);
                let old = lower(&String::from_utf8_lossy(&bytes[data..value_end]));
                if let Some(new_name) = renames.get(&old) {
                    let padded = (new_name.len() + 1 + 3) & !3;
                    let mut payload = vec![0u8; padded];
                    payload[..new_name.len()].copy_from_slice(new_name.as_bytes());
                    out.extend_from_slice(&rebuild_chunk(&bytes[chunk..chunk + 12], &payload));
                    changed += 1;
                } else {
                    out.extend_from_slice(&bytes[chunk..child_end]);
                }
            } else {
                out.extend_from_slice(&bytes[chunk..child_end]);
            }
            chunk = child_end;
            if chunk == end {
                break;
            }
        }
        if chunk != end {
            return (bytes[start..end].to_vec(), 0);
        }
        (out, changed)
    }

    fn rewrite_sequence(
        bytes: &[u8],
        start: usize,
        end: usize,
        renames: &HashMap<String, String>,
    ) -> (Vec<u8>, usize) {
        let mut out = Vec::with_capacity(end.saturating_sub(start));
        let mut chunk = start;
        let mut changed = 0usize;
        while let Some(current_end) = chunk_end(bytes, chunk, end) {
            let id = rd32(bytes, chunk);
            let data = chunk + 12;
            let (payload, child_changed) = if id == BREAKABLE_PLUGIN_ID {
                match parse_breakable_plugin(&bytes[data..current_end]) {
                    Ok(Some(mut breakable)) => {
                        let mut count = 0usize;
                        for group in &mut breakable.groups {
                            if let Some(name) = renames.get(&lower(&group.texture)) {
                                group.texture = name.clone();
                                count += 1;
                            }
                            if let Some(name) = renames.get(&lower(&group.mask)) {
                                group.mask = name.clone();
                                count += 1;
                            }
                        }
                        if count > 0 {
                            match encode_breakable_plugin(&breakable) {
                                Ok(payload) => (payload, count),
                                Err(_) => (bytes[data..current_end].to_vec(), 0),
                            }
                        } else {
                            (bytes[data..current_end].to_vec(), 0)
                        }
                    }
                    _ => (bytes[data..current_end].to_vec(), 0),
                }
            } else if id == 0x07 {
                let mut material_out = Vec::with_capacity(current_end - data);
                let mut child = data;
                let mut material_changed = 0usize;
                while let Some(child_end) = chunk_end(bytes, child, current_end) {
                    if rd32(bytes, child) == 0x06 {
                        let (rewritten, count) =
                            rewrite_texture(bytes, child + 12, child_end, renames);
                        material_out.extend_from_slice(&rebuild_chunk(
                            &bytes[child..child + 12],
                            &rewritten,
                        ));
                        material_changed += count;
                    } else {
                        material_out.extend_from_slice(&bytes[child..child_end]);
                    }
                    child = child_end;
                    if child == current_end {
                        break;
                    }
                }
                if child == current_end {
                    (material_out, material_changed)
                } else {
                    (bytes[data..current_end].to_vec(), 0)
                }
            } else if id != 0x01 {
                rewrite_sequence(bytes, data, current_end, renames)
            } else {
                (bytes[data..current_end].to_vec(), 0)
            };
            out.extend_from_slice(&rebuild_chunk(&bytes[chunk..chunk + 12], &payload));
            changed += child_changed;
            chunk = current_end;
            if chunk == end {
                break;
            }
        }
        if chunk != end {
            return (bytes[start..end].to_vec(), 0);
        }
        (out, changed)
    }

    let len = dff_chunk_len(bytes);
    if len == 0 || len > bytes.len() {
        return Err("DFF has no valid RenderWare chunks".to_string());
    }
    let (mut out, changed) = rewrite_sequence(bytes, 0, len, renames);
    out.extend_from_slice(&bytes[len..]);
    Ok((out, changed))
}

/// Fill a missing day or night prelight stream without re-exporting the DFF.
///
/// This deliberately rebuilds only the enclosing RenderWare chunk sizes. Every
/// frame, atomic, material, 2DFX record, pipeline marker, and unknown plug-in is
/// copied byte-for-byte. A full normalized export is not safe for arbitrary GTA
/// clumps because it cannot represent every game and RenderWare extension.
pub(crate) fn repair_dff_missing_prelight_streams(bytes: &[u8]) -> Result<Option<Vec<u8>>, String> {
    fn chunk_end(bytes: &[u8], start: usize, limit: usize) -> Option<usize> {
        (start + 12 <= limit).then_some(())?;
        start
            .checked_add(12)?
            .checked_add(rd32(bytes, start + 4) as usize)
            .filter(|end| *end <= limit && *end <= bytes.len())
    }

    fn rebuild_chunk(header: &[u8], payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(12 + payload.len());
        out.extend_from_slice(&header[..12]);
        out[4..8].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        out.extend_from_slice(payload);
        out
    }

    fn night_colors(
        bytes: &[u8],
        start: usize,
        end: usize,
        vertex_count: usize,
    ) -> Result<Option<Vec<u8>>, String> {
        let required = vertex_count
            .checked_mul(4)
            .ok_or_else(|| "DFF prelight vertex count overflowed".to_string())?;
        let mut chunk = start;
        while let Some(current_end) = chunk_end(bytes, chunk, end) {
            if rd32(bytes, chunk) == 0x0253_f2f9 {
                let data = chunk + 12;
                let size = current_end - data;
                if size >= 4 && rd32(bytes, data) != 0 {
                    if size < 4 + required {
                        return Err(
                            "DFF night prelight plug-in is shorter than its vertex stream"
                                .to_string(),
                        );
                    }
                    return Ok(Some(bytes[data + 4..data + 4 + required].to_vec()));
                }
            }
            chunk = current_end;
            if chunk == end {
                break;
            }
        }
        if chunk != end {
            return Err("DFF geometry extension has invalid chunk boundaries".to_string());
        }
        Ok(None)
    }

    fn add_or_replace_night_stream(
        bytes: &[u8],
        start: usize,
        end: usize,
        colors: &[u8],
        fallback_version: u32,
    ) -> Result<Vec<u8>, String> {
        let mut out = Vec::with_capacity(end.saturating_sub(start) + colors.len() + 16);
        let mut chunk = start;
        let mut replaced = false;
        while let Some(current_end) = chunk_end(bytes, chunk, end) {
            if rd32(bytes, chunk) == 0x0253_f2f9 && !replaced {
                let data = chunk + 12;
                let mut payload = if current_end - data >= 4 + colors.len() {
                    bytes[data..current_end].to_vec()
                } else {
                    vec![0u8; 4 + colors.len()]
                };
                payload[0..4].copy_from_slice(&1u32.to_le_bytes());
                payload[4..4 + colors.len()].copy_from_slice(colors);
                out.extend_from_slice(&rebuild_chunk(&bytes[chunk..chunk + 12], &payload));
                replaced = true;
            } else {
                out.extend_from_slice(&bytes[chunk..current_end]);
            }
            chunk = current_end;
            if chunk == end {
                break;
            }
        }
        if chunk != end {
            return Err("DFF geometry extension has invalid chunk boundaries".to_string());
        }
        if !replaced {
            let mut payload = Vec::with_capacity(4 + colors.len());
            payload.extend_from_slice(&1u32.to_le_bytes());
            payload.extend_from_slice(colors);
            out.extend_from_slice(&rw_chunk_with_version(
                0x0253_f2f9,
                fallback_version,
                payload,
            ));
        }
        Ok(out)
    }

    fn repair_geometry(
        bytes: &[u8],
        start: usize,
        end: usize,
        geometry_version: u32,
    ) -> Result<Option<Vec<u8>>, String> {
        let mut chunks = Vec::<(usize, usize)>::new();
        let mut chunk = start;
        while let Some(current_end) = chunk_end(bytes, chunk, end) {
            chunks.push((chunk, current_end));
            chunk = current_end;
            if chunk == end {
                break;
            }
        }
        if chunk != end {
            return Err("DFF geometry has invalid child chunk boundaries".to_string());
        }

        let Some(&(struct_start, struct_end)) =
            chunks.iter().find(|(child, _)| rd32(bytes, *child) == 0x01)
        else {
            return Ok(None);
        };
        let struct_data = struct_start + 12;
        if struct_end - struct_data < 16 {
            return Err("DFF geometry struct is too short for prelighting".to_string());
        }
        let flags = rd32(bytes, struct_data);
        let vertex_count = rd32(bytes, struct_data + 8) as usize;
        let color_len = vertex_count
            .checked_mul(4)
            .ok_or_else(|| "DFF prelight vertex count overflowed".to_string())?;
        let has_day = flags & 0x08 != 0;
        let day_colors = if has_day {
            let colors_end = struct_data
                .checked_add(16)
                .and_then(|start| start.checked_add(color_len))
                .ok_or_else(|| "DFF day prelight stream overflowed".to_string())?;
            if colors_end > struct_end {
                return Err("DFF day prelight stream is shorter than its vertex count".to_string());
            }
            Some(bytes[struct_data + 16..colors_end].to_vec())
        } else {
            None
        };

        let extension = chunks
            .iter()
            .find(|(child, _)| rd32(bytes, *child) == 0x03)
            .copied();
        let night = if let Some((extension_start, extension_end)) = extension {
            night_colors(bytes, extension_start + 12, extension_end, vertex_count)?
        } else {
            None
        };
        if has_day == night.is_some() {
            return Ok(None);
        }

        let mut out = Vec::with_capacity(end.saturating_sub(start) + color_len + 28);
        for (child, child_end) in chunks {
            let id = rd32(bytes, child);
            if id == 0x01 && !has_day {
                let colors = night.as_ref().expect("night stream checked above");
                let mut payload = Vec::with_capacity(struct_end - struct_data + colors.len());
                payload.extend_from_slice(&bytes[struct_data..struct_data + 16]);
                payload[0..4].copy_from_slice(&(flags | 0x08).to_le_bytes());
                payload.extend_from_slice(colors);
                payload.extend_from_slice(&bytes[struct_data + 16..struct_end]);
                out.extend_from_slice(&rebuild_chunk(&bytes[child..child + 12], &payload));
            } else if id == 0x03 && night.is_none() {
                let colors = day_colors.as_ref().expect("day stream checked above");
                let payload = add_or_replace_night_stream(
                    bytes,
                    child + 12,
                    child_end,
                    colors,
                    rd32(bytes, child + 8),
                )?;
                out.extend_from_slice(&rebuild_chunk(&bytes[child..child + 12], &payload));
            } else {
                out.extend_from_slice(&bytes[child..child_end]);
            }
        }
        if night.is_none() && extension.is_none() {
            let colors = day_colors.as_ref().expect("day stream checked above");
            let payload = add_or_replace_night_stream(bytes, 0, 0, colors, geometry_version)?;
            out.extend_from_slice(&rw_chunk_with_version(0x03, geometry_version, payload));
        }
        Ok(Some(out))
    }

    fn rewrite_range(bytes: &[u8], start: usize, end: usize) -> Result<Option<Vec<u8>>, String> {
        let mut out = Vec::with_capacity(end.saturating_sub(start));
        let mut chunk = start;
        let mut changed = false;
        while let Some(current_end) = chunk_end(bytes, chunk, end) {
            let id = rd32(bytes, chunk);
            let data = chunk + 12;
            let repaired = if id == 0x0f {
                repair_geometry(bytes, data, current_end, rd32(bytes, chunk + 8))?
            } else if matches!(id, 0x10 | 0x0e | 0x1a) {
                rewrite_range(bytes, data, current_end)?
            } else {
                None
            };
            if let Some(payload) = repaired {
                out.extend_from_slice(&rebuild_chunk(&bytes[chunk..chunk + 12], &payload));
                changed = true;
            } else {
                out.extend_from_slice(&bytes[chunk..current_end]);
            }
            chunk = current_end;
            if chunk == end {
                break;
            }
        }
        if chunk != end {
            return Err("DFF has invalid RenderWare chunk boundaries".to_string());
        }
        Ok(changed.then_some(out))
    }

    let len = dff_chunk_len(bytes);
    if len == 0 || len > bytes.len() {
        return Err("DFF has no valid RenderWare chunks".to_string());
    }
    let Some(mut repaired) = rewrite_range(bytes, 0, len)? else {
        return Ok(None);
    };
    repaired.extend_from_slice(&bytes[len..]);
    Ok(Some(repaired))
}

pub(crate) fn repair_dff_bounds_spheres(bytes: &mut [u8]) -> usize {
    fn is_container_chunk(id: u32) -> bool {
        matches!(id, 0x10 | 0x0e | 0x1a | 0x0f | 0x08 | 0x07 | 0x06 | 0x03)
    }

    fn patch_geometry_struct(bytes: &mut [u8], start: usize, end: usize) -> bool {
        if start + 16 > end || end > bytes.len() {
            return false;
        }
        let flags = rd32(bytes, start);
        let tri_count = rd32(bytes, start + 4) as usize;
        let vert_count = rd32(bytes, start + 8) as usize;
        let uv_count = {
            let explicit = ((flags >> 16) & 0xff) as usize;
            if explicit != 0 {
                explicit
            } else if flags & 0x80 != 0 {
                2
            } else if flags & 0x04 != 0 {
                1
            } else {
                0
            }
        };
        let mut p = start + 16;
        if flags & 0x08 != 0 {
            p = p.saturating_add(vert_count.saturating_mul(4));
        }
        p = p.saturating_add(uv_count.saturating_mul(vert_count).saturating_mul(8));
        p = p.saturating_add(tri_count.saturating_mul(8));
        let sphere_pos = p;
        p = p.saturating_add(16);
        if p + 8 > end {
            return false;
        }
        let has_verts = rd32(bytes, p);
        p += 4;
        let has_normals = rd32(bytes, p);
        p += 4;
        if has_verts == 0 || p + vert_count.saturating_mul(12) > end {
            return false;
        }

        let mut min = V3 {
            x: f32::INFINITY,
            y: f32::INFINITY,
            z: f32::INFINITY,
        };
        let mut max = V3 {
            x: f32::NEG_INFINITY,
            y: f32::NEG_INFINITY,
            z: f32::NEG_INFINITY,
        };
        for i in 0..vert_count {
            let o = p + i * 12;
            let vertex = V3 {
                x: rdf32(bytes, o),
                y: rdf32(bytes, o + 4),
                z: rdf32(bytes, o + 8),
            };
            if !vertex.x.is_finite() || !vertex.y.is_finite() || !vertex.z.is_finite() {
                return false;
            }
            min.x = min.x.min(vertex.x);
            min.y = min.y.min(vertex.y);
            min.z = min.z.min(vertex.z);
            max.x = max.x.max(vertex.x);
            max.y = max.y.max(vertex.y);
            max.z = max.z.max(vertex.z);
        }
        let vertex_start = p;
        if has_normals != 0 {
            p = p.saturating_add(vert_count.saturating_mul(12));
            if p > end {
                return false;
            }
        }

        let center = V3 {
            x: (min.x + max.x) * 0.5,
            y: (min.y + max.y) * 0.5,
            z: (min.z + max.z) * 0.5,
        };
        let center_mq = to_mq(center);
        let mut radius = 0.0f32;
        for i in 0..vert_count {
            let o = vertex_start + i * 12;
            let vertex = V3 {
                x: rdf32(bytes, o),
                y: rdf32(bytes, o + 4),
                z: rdf32(bytes, o + 8),
            };
            radius = radius.max((to_mq(vertex) - center_mq).length());
        }

        let current_center = V3 {
            x: rdf32(bytes, sphere_pos),
            y: rdf32(bytes, sphere_pos + 4),
            z: rdf32(bytes, sphere_pos + 8),
        };
        let current_radius = rdf32(bytes, sphere_pos + 12);
        let center_delta = (to_mq(current_center) - center_mq).length();
        if current_radius.is_finite() && current_radius + 0.01 >= radius && center_delta <= 0.01 {
            return false;
        }
        for (offset, value) in [
            (sphere_pos, center.x),
            (sphere_pos + 4, center.y),
            (sphere_pos + 8, center.z),
            (sphere_pos + 12, radius),
        ] {
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        true
    }

    fn scan(bytes: &mut [u8], start: usize, end: usize) -> usize {
        let mut repaired = 0usize;
        let mut o = start;
        while o + 12 <= end && o + 12 <= bytes.len() {
            let id = rd32(bytes, o);
            let size = rd32(bytes, o + 4) as usize;
            let cs = o + 12;
            let ce = cs.saturating_add(size);
            if ce > end || ce > bytes.len() {
                break;
            }
            if id == 0x0f {
                let mut child = cs;
                while child + 12 <= ce {
                    let child_id = rd32(bytes, child);
                    let child_size = rd32(bytes, child + 4) as usize;
                    let child_start = child + 12;
                    let child_end = child_start.saturating_add(child_size);
                    if child_end > ce || child_end > bytes.len() {
                        break;
                    }
                    if child_id == 0x01 && patch_geometry_struct(bytes, child_start, child_end) {
                        repaired += 1;
                    }
                    child = child_end;
                }
            }
            if id != 0x0f && is_container_chunk(id) {
                repaired += scan(bytes, cs, ce);
            }
            o = ce;
        }
        repaired
    }

    let len = dff_chunk_len(bytes);
    scan(bytes, 0, len)
}

/// Rewrites only BinMesh draw batches to match a reordered `RawMesh` while
/// retaining the original DFF's frames, atomics, plugins, and hierarchy.
///
/// This is intentionally limited to a pure triangle permutation. It lets the
/// DFF editor save alpha-face ordering for multi-frame models that cannot be
/// safely passed through the normalized writer.
pub(crate) fn rewrite_dff_bin_mesh_face_order(
    bytes: &[u8],
    source: &RawMesh,
    reordered: &RawMesh,
) -> Result<Option<Vec<u8>>, String> {
    fn chunk_end(bytes: &[u8], start: usize, end: usize) -> Option<usize> {
        (start + 12 <= end).then_some(())?;
        start
            .checked_add(12)?
            .checked_add(rd32(bytes, start + 4) as usize)
            .filter(|chunk_end| *chunk_end <= end && *chunk_end <= bytes.len())
    }

    fn bin_materials(payload: &[u8]) -> Result<Vec<u16>, String> {
        if payload.len() < 12 {
            return Err("DFF BinMesh is truncated".to_string());
        }
        let triangle_strip = rd32(payload, 0) & 1 != 0;
        let mesh_count = rd32(payload, 4) as usize;
        let mut out = Vec::new();
        let mut offset = 12usize;
        for _ in 0..mesh_count {
            if offset + 8 > payload.len() {
                return Err("DFF BinMesh group is truncated".to_string());
            }
            let index_count = rd32(payload, offset) as usize;
            let material = rd32(payload, offset + 4).min(u16::MAX as u32) as u16;
            offset += 8;
            if (!triangle_strip && index_count % 3 != 0)
                || (triangle_strip && index_count < 3)
                || offset + index_count * 4 > payload.len()
            {
                return Err("DFF BinMesh indices are invalid".to_string());
            }
            let triangle_count = if triangle_strip {
                index_count - 2
            } else {
                index_count / 3
            };
            out.extend(std::iter::repeat_n(material, triangle_count));
            offset += index_count * 4;
        }
        Ok(out)
    }

    fn collect_bin_mesh_payloads(
        bytes: &[u8],
        start: usize,
        end: usize,
        payloads: &mut Vec<Vec<u8>>,
    ) -> Result<(), String> {
        let mut chunk = start;
        while chunk < end {
            let Some(current_end) = chunk_end(bytes, chunk, end) else {
                return Err("DFF has invalid RenderWare chunk boundaries".to_string());
            };
            let id = rd32(bytes, chunk);
            let data = chunk + 12;
            if id == 0x050e {
                payloads.push(bytes[data..current_end].to_vec());
            } else if matches!(id, 0x10 | 0x0e | 0x1a | 0x0f | 0x03) {
                collect_bin_mesh_payloads(bytes, data, current_end, payloads)?;
            }
            chunk = current_end;
        }
        Ok(())
    }

    fn rebuild(
        bytes: &[u8],
        start: usize,
        end: usize,
        replacements: &[Vec<u8>],
        replacement_index: &mut usize,
    ) -> Result<Vec<u8>, String> {
        let mut out = Vec::with_capacity(end.saturating_sub(start));
        let mut chunk = start;
        while chunk < end {
            let Some(current_end) = chunk_end(bytes, chunk, end) else {
                return Err("DFF has invalid RenderWare chunk boundaries".to_string());
            };
            let id = rd32(bytes, chunk);
            let data = chunk + 12;
            let payload = if id == 0x050e {
                let replacement = replacements
                    .get(*replacement_index)
                    .ok_or_else(|| "DFF BinMesh count changed while rewriting".to_string())?;
                *replacement_index += 1;
                replacement.clone()
            } else if matches!(id, 0x10 | 0x0e | 0x1a | 0x0f | 0x03) {
                rebuild(bytes, data, current_end, replacements, replacement_index)?
            } else {
                out.extend_from_slice(&bytes[chunk..current_end]);
                chunk = current_end;
                continue;
            };
            out.extend_from_slice(&rw_chunk_with_version(id, rd32(bytes, chunk + 8), payload));
            chunk = current_end;
        }
        Ok(out)
    }

    if source.vertices != reordered.vertices
        || source.materials != reordered.materials
        || source.material_textures != reordered.material_textures
        || source.components.len() != reordered.components.len()
    {
        return Ok(None);
    }
    let len = dff_chunk_len(bytes);
    let mut payloads = Vec::new();
    collect_bin_mesh_payloads(bytes, 0, len, &mut payloads)?;
    if payloads.len() != source.components.len() {
        return Ok(None);
    }
    let mut replacements = Vec::with_capacity(payloads.len());
    for ((component, updated_component), payload) in source
        .components
        .iter()
        .zip(&reordered.components)
        .zip(payloads)
    {
        let original = &source.triangles[component.tri_start..component.tri_end];
        let updated = &reordered.triangles[updated_component.tri_start..updated_component.tri_end];
        let local_materials = bin_materials(&payload)?;
        if original.len() != updated.len() || original.len() != local_materials.len() {
            return Ok(None);
        }
        let mut material_map = HashMap::<u16, u16>::new();
        for (triangle, local_material) in original.iter().zip(local_materials) {
            if material_map
                .insert(triangle.material, local_material)
                .is_some_and(|known| known != local_material)
            {
                return Ok(None);
            }
        }
        let mut groups = Vec::<(u16, Vec<u32>)>::new();
        for triangle in updated {
            let material = *material_map.get(&triangle.material).ok_or_else(|| {
                "DFF face reorder introduced a material not present in the source BinMesh"
                    .to_string()
            })?;
            let indices = [triangle.b, triangle.a, triangle.c];
            if indices.iter().any(|index| {
                let index = *index as usize;
                index < updated_component.vertex_start || index >= updated_component.vertex_end
            }) {
                return Ok(None);
            }
            if groups.last().is_none_or(|(known, _)| *known != material) {
                groups.push((material, Vec::new()));
            }
            groups.last_mut().expect("BinMesh group exists").1.extend(
                indices
                    .into_iter()
                    .map(|index| index - updated_component.vertex_start as u32),
            );
        }
        let total_indices = groups
            .iter()
            .map(|(_, indices)| indices.len())
            .sum::<usize>();
        let mut replacement = Vec::with_capacity(12 + total_indices * 4 + groups.len() * 8);
        replacement.extend_from_slice(&0u32.to_le_bytes());
        replacement.extend_from_slice(&(groups.len() as u32).to_le_bytes());
        replacement.extend_from_slice(&(total_indices as u32).to_le_bytes());
        for (material, indices) in groups {
            replacement.extend_from_slice(&(indices.len() as u32).to_le_bytes());
            replacement.extend_from_slice(&(material as u32).to_le_bytes());
            for index in indices {
                replacement.extend_from_slice(&index.to_le_bytes());
            }
        }
        replacements.push(replacement);
    }
    let mut replacement_index = 0usize;
    let mut output = rebuild(bytes, 0, len, &replacements, &mut replacement_index)?;
    if replacement_index != replacements.len() {
        return Err("DFF BinMesh rewrite did not consume every geometry".to_string());
    }
    output.extend_from_slice(&bytes[len..]);
    Ok((output != bytes).then_some(output))
}

/// Splits a BinMesh payload into its draw batches.
///
/// Returns the payload flags, one `(material, raw index bytes)` pair per batch,
/// and any trailing bytes the batch table did not consume so callers rebuilding
/// the payload can keep them.
fn bin_mesh_batches(payload: &[u8]) -> Result<(u32, Vec<(u32, &[u8])>, &[u8]), String> {
    if payload.len() < 12 {
        return Err("DFF BinMesh is truncated".to_string());
    }
    let flags = rd32(payload, 0);
    let triangle_strip = flags & 1 != 0;
    let batch_count = rd32(payload, 4) as usize;
    let mut batches = Vec::new();
    let mut offset = 12usize;
    for _ in 0..batch_count {
        if offset + 8 > payload.len() {
            return Err("DFF BinMesh group is truncated".to_string());
        }
        let index_count = rd32(payload, offset) as usize;
        let material = rd32(payload, offset + 4);
        offset += 8;
        let next = index_count
            .checked_mul(4)
            .and_then(|size| offset.checked_add(size))
            .filter(|next| *next <= payload.len())
            .ok_or_else(|| "DFF BinMesh indices are invalid".to_string())?;
        if (!triangle_strip && index_count % 3 != 0) || (triangle_strip && index_count < 3) {
            return Err("DFF BinMesh indices are invalid".to_string());
        }
        batches.push((material, &payload[offset..next]));
        offset = next;
    }
    Ok((flags, batches, &payload[offset..]))
}

/// Rebuilds a BinMesh payload with neighbouring same-material batches merged.
///
/// Returns `Some((payload, removed_batches))` when at least one batch was
/// merged away, otherwise `None`.
fn canonical_bin_mesh_payload(payload: &[u8]) -> Result<Option<(Vec<u8>, usize)>, String> {
    let (flags, batches, trailing) = bin_mesh_batches(payload)?;
    if flags & 1 != 0 {
        // Concatenating two triangle strips welds them into a single strip and
        // invents triangles across the seam, so strips are left untouched.
        return Ok(None);
    }
    let mut merged = Vec::<(u32, Vec<u8>)>::new();
    for (material, indices) in batches.iter().copied() {
        match merged.last_mut() {
            Some((known, data)) if *known == material => data.extend_from_slice(indices),
            _ => merged.push((material, indices.to_vec())),
        }
    }
    let removed = batches.len() - merged.len();
    if removed == 0 {
        return Ok(None);
    }
    let total_indices = merged.iter().map(|(_, data)| data.len() / 4).sum::<usize>();
    let mut out = Vec::with_capacity(payload.len());
    out.extend_from_slice(&flags.to_le_bytes());
    out.extend_from_slice(&(merged.len() as u32).to_le_bytes());
    out.extend_from_slice(&(total_indices as u32).to_le_bytes());
    for (material, indices) in merged {
        out.extend_from_slice(&((indices.len() / 4) as u32).to_le_bytes());
        out.extend_from_slice(&material.to_le_bytes());
        out.extend_from_slice(&indices);
    }
    out.extend_from_slice(trailing);
    Ok(Some((out, removed)))
}

/// Collapses consecutive BinMesh draw batches that already share a material.
///
/// Every batch is its own draw call in-game, and exporters that emit one batch
/// per triangle multiply that cost without changing what is drawn. Merging only
/// adjacent batches keeps the face order intact, which alpha-blended materials
/// depend on; batches separated by a different material are left alone.
///
/// Returns `Some((fixed_bytes, removed_batches))` when a merge was applied.
pub(crate) fn canonicalize_dff_bin_mesh_batches(
    bytes: &[u8],
) -> Result<Option<(Vec<u8>, usize)>, String> {
    fn chunk_end(bytes: &[u8], start: usize, limit: usize) -> Option<usize> {
        (start + 12 <= limit).then_some(())?;
        start
            .checked_add(12)?
            .checked_add(rd32(bytes, start + 4) as usize)
            .filter(|end| *end <= limit && *end <= bytes.len())
    }

    fn rewrite_range(
        bytes: &[u8],
        start: usize,
        end: usize,
        removed: &mut usize,
    ) -> Result<Option<Vec<u8>>, String> {
        let mut out = Vec::with_capacity(end.saturating_sub(start));
        let mut chunk = start;
        let mut changed = false;
        while let Some(current_end) = chunk_end(bytes, chunk, end) {
            let id = rd32(bytes, chunk);
            let data = chunk + 12;
            let rewritten = if id == 0x050e {
                canonical_bin_mesh_payload(&bytes[data..current_end])?.map(|(payload, batches)| {
                    *removed += batches;
                    payload
                })
            } else if matches!(id, 0x10 | 0x0e | 0x1a | 0x0f | 0x03) {
                rewrite_range(bytes, data, current_end, removed)?
            } else {
                None
            };
            if let Some(payload) = rewritten {
                out.extend_from_slice(&rw_chunk_with_version(id, rd32(bytes, chunk + 8), payload));
                changed = true;
            } else {
                out.extend_from_slice(&bytes[chunk..current_end]);
            }
            chunk = current_end;
            if chunk == end {
                break;
            }
        }
        if chunk != end {
            return Err("DFF has invalid RenderWare chunk boundaries".to_string());
        }
        Ok(changed.then_some(out))
    }

    let len = dff_chunk_len(bytes);
    if len == 0 || len > bytes.len() {
        return Err("DFF has no valid RenderWare chunks".to_string());
    }
    let mut removed = 0usize;
    let Some(mut canonical) = rewrite_range(bytes, 0, len, &mut removed)? else {
        return Ok(None);
    };
    canonical.extend_from_slice(&bytes[len..]);
    Ok(Some((canonical, removed)))
}

/// Counts BinMesh batches that `canonicalize_dff_bin_mesh_batches` would merge
/// away, so validation can report the problem without rewriting the file.
pub(crate) fn dff_redundant_bin_mesh_batch_count(bytes: &[u8]) -> Result<usize, String> {
    fn scan(bytes: &[u8], start: usize, end: usize, redundant: &mut usize) -> Result<(), String> {
        let mut chunk = start;
        while chunk < end {
            let data = chunk + 12;
            let current_end = (chunk + 12 <= end)
                .then(|| data.checked_add(rd32(bytes, chunk + 4) as usize))
                .flatten()
                .filter(|current_end| *current_end <= end && *current_end <= bytes.len())
                .ok_or_else(|| "DFF has invalid RenderWare chunk boundaries".to_string())?;
            let id = rd32(bytes, chunk);
            if id == 0x050e {
                let (flags, batches, _) = bin_mesh_batches(&bytes[data..current_end])?;
                if flags & 1 == 0 {
                    *redundant += batches
                        .windows(2)
                        .filter(|pair| pair[0].0 == pair[1].0)
                        .count();
                }
            } else if matches!(id, 0x10 | 0x0e | 0x1a | 0x0f | 0x03) {
                scan(bytes, data, current_end, redundant)?;
            }
            chunk = current_end;
        }
        Ok(())
    }

    let len = dff_chunk_len(bytes);
    if len == 0 || len > bytes.len() {
        return Err("DFF has no valid RenderWare chunks".to_string());
    }
    let mut redundant = 0usize;
    scan(bytes, 0, len, &mut redundant)?;
    Ok(redundant)
}

/// Byte ranges of every geometry `Struct` payload (chunk 0x01 inside a 0x0f
/// geometry) in a DFF.
fn dff_geometry_struct_spans(bytes: &[u8]) -> Result<Vec<(usize, usize)>, String> {
    fn scan(
        bytes: &[u8],
        start: usize,
        end: usize,
        spans: &mut Vec<(usize, usize)>,
    ) -> Result<(), String> {
        let mut chunk = start;
        while chunk < end {
            let data = chunk + 12;
            let current_end = (chunk + 12 <= end)
                .then(|| data.checked_add(rd32(bytes, chunk + 4) as usize))
                .flatten()
                .filter(|current_end| *current_end <= end && *current_end <= bytes.len())
                .ok_or_else(|| "DFF has invalid RenderWare chunk boundaries".to_string())?;
            let id = rd32(bytes, chunk);
            if id == 0x0f {
                let mut child = data;
                while child + 12 <= current_end {
                    let child_data = child + 12;
                    let child_end = child_data
                        .checked_add(rd32(bytes, child + 4) as usize)
                        .filter(|child_end| *child_end <= current_end)
                        .ok_or_else(|| "DFF has invalid RenderWare chunk boundaries".to_string())?;
                    if rd32(bytes, child) == 0x01 {
                        spans.push((child_data, child_end));
                    }
                    child = child_end;
                }
            } else if matches!(id, 0x10 | 0x0e | 0x1a) {
                scan(bytes, data, current_end, spans)?;
            }
            chunk = current_end;
        }
        Ok(())
    }

    let len = dff_chunk_len(bytes);
    if len == 0 || len > bytes.len() {
        return Err("DFF has no valid RenderWare chunks".to_string());
    }
    let mut spans = Vec::new();
    scan(bytes, 0, len, &mut spans)?;
    Ok(spans)
}

/// A geometry that requests RenderWare lighting (`rpGEOMETRYLIGHT`) without
/// carrying the vertex normals (`rpGEOMETRYNORMALS`) that lighting reads.
fn geometry_flags_are_normal_less_lit(flags: u32) -> bool {
    flags & 0x20 != 0 && flags & 0x10 == 0
}

/// Counts geometry sections asking to be lit without shipping vertex normals.
pub(crate) fn dff_normal_less_lit_geometry_count(bytes: &[u8]) -> Result<usize, String> {
    Ok(dff_geometry_struct_spans(bytes)?
        .into_iter()
        .filter(|(start, end)| {
            *end >= start + 16 && geometry_flags_are_normal_less_lit(rd32(bytes, *start))
        })
        .count())
}

/// Clears `rpGEOMETRYLIGHT` on geometry that has no vertex normals.
///
/// GTA:SA lights such a geometry from an undefined normal stream, which shows
/// up as black or violently flickering faces. The flag lives in the geometry
/// struct header, so this patches in place without moving any chunk boundary.
///
/// Returns the number of geometry sections corrected.
pub(crate) fn repair_dff_normal_less_lighting_flags(bytes: &mut [u8]) -> Result<usize, String> {
    let mut repaired = 0usize;
    for (start, end) in dff_geometry_struct_spans(bytes)? {
        if end < start + 16 {
            continue;
        }
        let flags = rd32(bytes, start);
        if !geometry_flags_are_normal_less_lit(flags) {
            continue;
        }
        bytes[start..start + 4].copy_from_slice(&(flags & !0x20).to_le_bytes());
        repaired += 1;
    }
    Ok(repaired)
}

/// Detects a top-level UV Animation Dictionary (chunk 0x2b) positioned after the
/// Clump (chunk 0x10) and reorders it to appear before the clump.
///
/// GTA:SA's loader reads the dictionary first and registers each animation by
/// name, then resolves every material's UV Animation PLG reference against that
/// registry while reading the Clump. A dictionary streamed after the Clump is
/// silently ignored in-game (our own importer scans the whole file top-level, so
/// the editor preview still plays it either way). This repair rewrites such DFFs
/// so the animation plays in-game as well.
///
/// Returns `Some(fixed_bytes)` when a reorder was applied, otherwise `None`.
pub(crate) fn repair_dff_uv_anim_dictionary_order(bytes: &[u8]) -> Option<Vec<u8>> {
    let len = dff_chunk_len(bytes);
    let mut spans: Vec<(u32, usize, usize)> = Vec::new();
    let mut o = 0usize;
    while o + 12 <= len {
        let id = rd32(bytes, o);
        let size = rd32(bytes, o + 4) as usize;
        let end = o.saturating_add(12).saturating_add(size);
        if end > len {
            break;
        }
        spans.push((id, o, end));
        o = end;
    }

    let first_clump = spans.iter().position(|(id, _, _)| *id == 0x10)?;
    let has_late_dictionary = spans
        .iter()
        .skip(first_clump + 1)
        .any(|(id, _, _)| *id == 0x2b);
    if !has_late_dictionary {
        return None;
    }

    let mut out = Vec::with_capacity(bytes.len());
    // Chunks before the first clump stay put (may include a correct dictionary).
    for &(_, s, e) in &spans[..first_clump] {
        out.extend_from_slice(&bytes[s..e]);
    }
    // Move every dictionary that trailed the clump up to just before it.
    for &(id, s, e) in &spans[first_clump + 1..] {
        if id == 0x2b {
            out.extend_from_slice(&bytes[s..e]);
        }
    }
    // Then the clump and everything after it, minus the moved dictionaries.
    for &(id, s, e) in &spans[first_clump..] {
        if id == 0x2b {
            continue;
        }
        out.extend_from_slice(&bytes[s..e]);
    }
    // Preserve any trailing bytes beyond the parsed chunk stream.
    if len < bytes.len() {
        out.extend_from_slice(&bytes[len..]);
    }
    Some(out)
}

fn rw_chunk_with_version(id: u32, version: u32, data: Vec<u8>) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + 12);
    out.extend_from_slice(&id.to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&version.to_le_bytes());
    out.extend_from_slice(&data);
    out
}

fn contains_rw_chunk(bytes: &[u8], start: usize, end: usize, target: u32) -> bool {
    let mut o = start;
    while o + 12 <= end {
        let id = rd32(bytes, o);
        let size = rd32(bytes, o + 4) as usize;
        let cs = o + 12;
        let ce = cs.saturating_add(size);
        if ce > end || ce > bytes.len() {
            break;
        }
        if id == target || contains_rw_chunk(bytes, cs, ce, target) {
            return true;
        }
        o = ce;
    }
    false
}

fn right_to_render_chunk() -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&0x0116u32.to_le_bytes());
    data.extend_from_slice(&1u32.to_le_bytes());
    rw_chunk(0x1f, data)
}

fn repair_geometry_right_to_render(bytes: &[u8], start: usize, end: usize) -> Option<Vec<u8>> {
    let mut o = start;
    let mut out = Vec::with_capacity(end.saturating_sub(start) + 20);
    let mut changed = false;
    let mut saw_extension = false;
    while o + 12 <= end {
        let id = rd32(bytes, o);
        let size = rd32(bytes, o + 4) as usize;
        let version = rd32(bytes, o + 8);
        let cs = o + 12;
        let ce = cs.saturating_add(size);
        if ce > end || ce > bytes.len() {
            return None;
        }
        if id == 0x03 {
            saw_extension = true;
            let has_right_to_render = (cs..ce)
                .scan(cs, |pos, _| {
                    if *pos + 12 > ce {
                        return None;
                    }
                    let child_id = rd32(bytes, *pos);
                    let child_size = rd32(bytes, *pos + 4) as usize;
                    let child_end = (*pos).saturating_add(12).saturating_add(child_size);
                    if child_end > ce || child_end > bytes.len() {
                        return None;
                    }
                    *pos = child_end;
                    Some(child_id)
                })
                .any(|child_id| child_id == 0x1f);
            if has_right_to_render {
                out.extend_from_slice(&bytes[o..ce]);
            } else {
                let mut extension = bytes[cs..ce].to_vec();
                extension.extend_from_slice(&right_to_render_chunk());
                out.extend_from_slice(&rw_chunk_with_version(id, version, extension));
                changed = true;
            }
        } else {
            out.extend_from_slice(&bytes[o..ce]);
        }
        o = ce;
    }
    if o != end {
        return None;
    }
    if !saw_extension {
        out.extend_from_slice(&rw_chunk(0x03, right_to_render_chunk()));
        changed = true;
    }
    changed.then_some(out)
}

fn repair_uv_anim_right_to_render_range(bytes: &[u8], start: usize, end: usize) -> Option<Vec<u8>> {
    let mut o = start;
    let mut out = Vec::with_capacity(end.saturating_sub(start) + 20);
    let mut changed = false;
    while o + 12 <= end {
        let id = rd32(bytes, o);
        let size = rd32(bytes, o + 4) as usize;
        let version = rd32(bytes, o + 8);
        let cs = o + 12;
        let ce = cs.saturating_add(size);
        if ce > end || ce > bytes.len() {
            return None;
        }
        let repaired = if id == 0x0f {
            repair_geometry_right_to_render(bytes, cs, ce)
        } else if matches!(id, 0x10 | 0x0e | 0x1a) {
            repair_uv_anim_right_to_render_range(bytes, cs, ce)
        } else {
            None
        };
        if let Some(payload) = repaired {
            out.extend_from_slice(&rw_chunk_with_version(id, version, payload));
            changed = true;
        } else {
            out.extend_from_slice(&bytes[o..ce]);
        }
        o = ce;
    }
    if o != end {
        return None;
    }
    changed.then_some(out)
}

pub(crate) fn repair_dff_uv_anim_right_to_render(bytes: &[u8]) -> Option<Vec<u8>> {
    let len = dff_chunk_len(bytes);
    if !contains_rw_chunk(bytes, 0, len, 0x0135) {
        return None;
    }
    let mut out = repair_uv_anim_right_to_render_range(bytes, 0, len)?;
    if len < bytes.len() {
        out.extend_from_slice(&bytes[len..]);
    }
    Some(out)
}

pub(crate) fn normalized_normals(raw: &RawMesh) -> Vec<V3> {
    if raw.normals.len() == raw.vertices.len() {
        return raw.normals.clone();
    }

    fn pos_key(v: V3) -> (i32, i32, i32) {
        const SCALE: f32 = 1000.0;
        (
            (v.x * SCALE).round() as i32,
            (v.y * SCALE).round() as i32,
            (v.z * SCALE).round() as i32,
        )
    }

    let mut grouped = HashMap::<(i32, i32, i32), Vec3>::new();
    let mut grouped_source = HashMap::<(i32, i32, i32), Vec3>::new();
    let mut vertex_fallback = vec![Vec3::ZERO; raw.vertices.len()];
    for (idx, normal) in raw.normals.iter().enumerate() {
        let Some(vertex) = raw.vertices.get(idx) else {
            continue;
        };
        *grouped_source.entry(pos_key(*vertex)).or_insert(Vec3::ZERO) += to_mq(*normal);
    }
    for tri in &raw.triangles {
        let (a, b, c) = (tri.a as usize, tri.b as usize, tri.c as usize);
        if a >= raw.vertices.len() || b >= raw.vertices.len() || c >= raw.vertices.len() {
            continue;
        }
        let pa = to_mq(raw.vertices[a]);
        let pb = to_mq(raw.vertices[b]);
        let pc = to_mq(raw.vertices[c]);
        // RawMesh triangles use the RenderWare clockwise front-face order.
        // Reconstruct the visible-side normal rather than the CCW back-face
        // normal when a DFF has no complete authored normal stream.
        let face = (pc - pa).cross(pb - pa);
        if face.length_squared() < 0.000001 {
            continue;
        }
        let face_normal = face.normalize();
        for (idx, p, p1, p2) in [(a, pa, pb, pc), (b, pb, pc, pa), (c, pc, pa, pb)] {
            let v1 = (p1 - p).normalize_or_zero();
            let v2 = (p2 - p).normalize_or_zero();
            let angle = v1.dot(v2).clamp(-1.0, 1.0).acos();
            let weighted = face_normal * angle.max(0.0001);
            vertex_fallback[idx] += weighted;
            *grouped
                .entry(pos_key(raw.vertices[idx]))
                .or_insert(Vec3::ZERO) += weighted;
        }
    }
    raw.vertices
        .iter()
        .enumerate()
        .map(|(idx, vertex)| {
            let n = grouped
                .get(&pos_key(*vertex))
                .copied()
                .unwrap_or(vertex_fallback[idx])
                .normalize_or_zero();
            if n.length_squared() < 0.0001 {
                raw.normals.get(idx).copied().unwrap_or(V3 {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                })
            } else {
                let aligned = if let Some(source) = grouped_source.get(&pos_key(*vertex)) {
                    let source = source.normalize_or_zero();
                    if source.length_squared() > 0.0001 && n.dot(source) < 0.0 {
                        -n
                    } else {
                        n
                    }
                } else {
                    n
                };
                from_mq(aligned)
            }
        })
        .collect()
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct DffLosslessCompactionStats {
    pub(crate) vertices_removed: usize,
    pub(crate) unreferenced_vertices_removed: usize,
    pub(crate) invalid_vertices_removed: usize,
    pub(crate) vertices_welded: usize,
    pub(crate) invalid_triangles_removed: usize,
    pub(crate) degenerate_triangles_removed: usize,
    pub(crate) duplicate_triangles_removed: usize,
    pub(crate) materials_removed: usize,
    pub(crate) triangles_reordered: usize,
}

fn raw_vertex_is_finite(raw: &RawMesh, source_normals: &[V3], index: usize) -> bool {
    fn v2_is_finite(value: V2) -> bool {
        value.u.is_finite() && value.v.is_finite()
    }
    fn v3_is_finite(value: V3) -> bool {
        value.x.is_finite() && value.y.is_finite() && value.z.is_finite()
    }

    raw.vertices.get(index).copied().is_some_and(v3_is_finite)
        && source_normals.get(index).copied().is_some_and(v3_is_finite)
        && (raw.uvs.len() != raw.vertices.len() || v2_is_finite(raw.uvs[index]))
        && raw
            .secondary_uvs
            .iter()
            .all(|uvs| uvs.len() != raw.vertices.len() || v2_is_finite(uvs[index]))
        && (raw.prelit_colors.len() != raw.vertices.len() || v3_is_finite(raw.prelit_colors[index]))
        && (raw.prelit_alphas.len() != raw.vertices.len() || raw.prelit_alphas[index].is_finite())
        && (raw.night_prelit_colors.len() != raw.vertices.len()
            || v3_is_finite(raw.night_prelit_colors[index]))
        && (raw.night_prelit_alphas.len() != raw.vertices.len()
            || raw.night_prelit_alphas[index].is_finite())
}

fn raw_vertex_lossless_key(
    raw: &RawMesh,
    source_normals: &[V3],
    index: usize,
    owner: usize,
) -> Vec<u32> {
    fn push_v2(key: &mut Vec<u32>, value: V2) {
        key.extend([value.u.to_bits(), value.v.to_bits()]);
    }
    fn push_v3(key: &mut Vec<u32>, value: V3) {
        key.extend([value.x.to_bits(), value.y.to_bits(), value.z.to_bits()]);
    }

    let mut key = Vec::with_capacity(17);
    key.extend([(owner >> 32) as u32, owner as u32]);
    push_v3(&mut key, raw.vertices[index]);
    push_v3(&mut key, source_normals[index]);
    if raw.uvs.len() == raw.vertices.len() {
        push_v2(&mut key, raw.uvs[index]);
    }
    for uvs in &raw.secondary_uvs {
        if uvs.len() == raw.vertices.len() {
            push_v2(&mut key, uvs[index]);
        }
    }
    if raw.prelit_colors.len() == raw.vertices.len() {
        push_v3(&mut key, raw.prelit_colors[index]);
    }
    if raw.prelit_alphas.len() == raw.vertices.len() {
        key.push(raw.prelit_alphas[index].to_bits());
    }
    if raw.night_prelit_colors.len() == raw.vertices.len() {
        push_v3(&mut key, raw.night_prelit_colors[index]);
    }
    if raw.night_prelit_alphas.len() == raw.vertices.len() {
        key.push(raw.night_prelit_alphas[index].to_bits());
    }
    if raw.light_flags.len() == raw.vertices.len() {
        key.push(raw.light_flags[index] as u32);
    }
    key
}

fn triangle_is_exactly_degenerate(raw: &RawMesh, tri: Tri) -> bool {
    if tri.a == tri.b || tri.b == tri.c || tri.a == tri.c {
        return true;
    }
    let a = raw.vertices[tri.a as usize];
    let b = raw.vertices[tri.b as usize];
    let c = raw.vertices[tri.c as usize];
    let ab = [
        b.x as f64 - a.x as f64,
        b.y as f64 - a.y as f64,
        b.z as f64 - a.z as f64,
    ];
    let ac = [
        c.x as f64 - a.x as f64,
        c.y as f64 - a.y as f64,
        c.z as f64 - a.z as f64,
    ];
    let cross = [
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    ];
    cross == [0.0, 0.0, 0.0]
}

fn cyclic_triangle_key(tri: Tri, owner: usize) -> (usize, u16, [u32; 3]) {
    let rotations = [
        [tri.a, tri.b, tri.c],
        [tri.b, tri.c, tri.a],
        [tri.c, tri.a, tri.b],
    ];
    (owner, tri.material, *rotations.iter().min().unwrap())
}

fn reorder_triangle_run_for_cache(run: &mut [Tri]) -> usize {
    const CACHE_SIZE: usize = 32;
    if run.len() < 3 {
        return 0;
    }

    let original = run.to_vec();
    let mut vertex_triangles = HashMap::<u32, Vec<usize>>::new();
    for (index, tri) in original.iter().enumerate() {
        for vertex in [tri.a, tri.b, tri.c] {
            vertex_triangles.entry(vertex).or_default().push(index);
        }
    }
    let mut pending = BTreeSet::<usize>::from_iter(0..original.len());
    let mut cache = Vec::<u32>::with_capacity(CACHE_SIZE);
    let mut ordered = Vec::with_capacity(original.len());
    while !pending.is_empty() {
        let mut candidates = BTreeSet::<usize>::new();
        for vertex in &cache {
            if let Some(indices) = vertex_triangles.get(vertex) {
                candidates.extend(
                    indices
                        .iter()
                        .copied()
                        .filter(|index| pending.contains(index)),
                );
            }
        }
        let candidate_indices: Vec<_> = if candidates.is_empty() {
            vec![*pending.first().unwrap()]
        } else {
            candidates.into_iter().collect()
        };
        let next = candidate_indices
            .into_iter()
            .max_by_key(|index| {
                let tri = original[*index];
                let hits = [tri.a, tri.b, tri.c]
                    .into_iter()
                    .filter(|vertex| cache.contains(vertex))
                    .count();
                (hits, usize::MAX - *index)
            })
            .unwrap();
        pending.remove(&next);
        let tri = original[next];
        ordered.push(tri);
        for vertex in [tri.a, tri.b, tri.c] {
            if let Some(position) = cache.iter().position(|cached| *cached == vertex) {
                cache.remove(position);
            }
            cache.insert(0, vertex);
        }
        cache.truncate(CACHE_SIZE);
    }

    let changed = original
        .iter()
        .zip(&ordered)
        .filter(|(before, after)| before != after)
        .count();
    run.copy_from_slice(&ordered);
    changed
}

/// Deterministically removes data that cannot affect a valid exported DFF.
///
/// `opaque_materials` contains the source material indices whose triangles may
/// be reordered for post-transform vertex-cache locality. Reordering never
/// crosses a component or a contiguous material run, and never changes winding.
pub(crate) fn compact_raw_mesh_lossless(
    raw: &mut RawMesh,
    opaque_materials: &BTreeSet<u16>,
) -> DffLosslessCompactionStats {
    let original_vertex_count = raw.vertices.len();
    let original_triangle_count = raw.triangles.len();
    let source_normals = normalized_normals(raw);
    let has_normals = source_normals.len() == original_vertex_count;
    let has_uvs = raw.uvs.len() == original_vertex_count;
    let secondary_uv_sets = raw
        .secondary_uvs
        .iter()
        .filter(|uvs| uvs.len() == original_vertex_count)
        .cloned()
        .collect::<Vec<_>>();
    let has_prelit = raw.prelit_colors.len() == original_vertex_count;
    let has_prelit_alpha = raw.prelit_alphas.len() == original_vertex_count;
    let has_night_prelit = raw.night_prelit_colors.len() == original_vertex_count;
    let has_night_prelit_alpha = raw.night_prelit_alphas.len() == original_vertex_count;
    let has_light_flags = raw.light_flags.len() == original_vertex_count;
    let original_triangles = raw.triangles.clone();

    let mut triangle_owner = vec![usize::MAX; original_triangle_count];
    for (component_index, component) in raw.components.iter().enumerate() {
        let start = component.tri_start.min(original_triangle_count);
        let end = component.tri_end.min(original_triangle_count).max(start);
        for owner in &mut triangle_owner[start..end] {
            if *owner == usize::MAX {
                *owner = component_index;
            }
        }
    }

    let mut stats = DffLosslessCompactionStats::default();
    let mut vertices = Vec::<V3>::new();
    let mut normals = Vec::<V3>::new();
    let mut uvs = Vec::<V2>::new();
    let mut secondary_uvs = vec![Vec::<V2>::new(); secondary_uv_sets.len()];
    let mut prelit_colors = Vec::<V3>::new();
    let mut prelit_alphas = Vec::<f32>::new();
    let mut night_prelit_colors = Vec::<V3>::new();
    let mut night_prelit_alphas = Vec::<f32>::new();
    let mut light_flags = Vec::<bool>::new();
    let mut vertex_map = BTreeMap::<Vec<u32>, u32>::new();
    let mut used_source_vertices = BTreeSet::<usize>::new();
    let mut invalid_source_vertices = BTreeSet::<usize>::new();
    let mut triangles = Vec::<Tri>::new();
    let mut owners = Vec::<usize>::new();
    let mut duplicate_keys = BTreeSet::<(usize, u16, [u32; 3])>::new();
    let mut kept_prefix = vec![0usize; original_triangle_count + 1];

    for (triangle_index, tri) in original_triangles.iter().copied().enumerate() {
        let source_indices = [tri.a as usize, tri.b as usize, tri.c as usize];
        if source_indices
            .iter()
            .any(|index| *index >= original_vertex_count)
        {
            stats.invalid_triangles_removed += 1;
            kept_prefix[triangle_index + 1] = triangles.len();
            continue;
        }
        if source_indices
            .iter()
            .any(|index| !raw_vertex_is_finite(raw, &source_normals, *index))
        {
            invalid_source_vertices.extend(
                source_indices
                    .iter()
                    .copied()
                    .filter(|index| !raw_vertex_is_finite(raw, &source_normals, *index)),
            );
            stats.invalid_triangles_removed += 1;
            kept_prefix[triangle_index + 1] = triangles.len();
            continue;
        }
        if triangle_is_exactly_degenerate(raw, tri) {
            stats.degenerate_triangles_removed += 1;
            kept_prefix[triangle_index + 1] = triangles.len();
            continue;
        }

        let owner = triangle_owner[triangle_index];
        let mut mapped = [0u32; 3];
        for (corner, source_index) in source_indices.into_iter().enumerate() {
            let key = raw_vertex_lossless_key(raw, &source_normals, source_index, owner);
            mapped[corner] = if let Some(mapped) = vertex_map.get(&key).copied() {
                mapped
            } else {
                let mapped = vertices.len() as u32;
                vertex_map.insert(key, mapped);
                vertices.push(raw.vertices[source_index]);
                if has_normals {
                    normals.push(source_normals[source_index]);
                }
                if has_uvs {
                    uvs.push(raw.uvs[source_index]);
                }
                for (out, set) in secondary_uvs.iter_mut().zip(&secondary_uv_sets) {
                    out.push(set[source_index]);
                }
                if has_prelit {
                    prelit_colors.push(raw.prelit_colors[source_index]);
                }
                if has_prelit_alpha {
                    prelit_alphas.push(raw.prelit_alphas[source_index]);
                }
                if has_night_prelit {
                    night_prelit_colors.push(raw.night_prelit_colors[source_index]);
                }
                if has_night_prelit_alpha {
                    night_prelit_alphas.push(raw.night_prelit_alphas[source_index]);
                }
                if has_light_flags {
                    light_flags.push(raw.light_flags[source_index]);
                }
                mapped
            };
        }
        let mapped_tri = Tri {
            a: mapped[0],
            b: mapped[1],
            c: mapped[2],
            material: tri.material,
        };
        if !duplicate_keys.insert(cyclic_triangle_key(mapped_tri, owner)) {
            stats.duplicate_triangles_removed += 1;
            kept_prefix[triangle_index + 1] = triangles.len();
            continue;
        }
        used_source_vertices.extend(source_indices);
        triangles.push(mapped_tri);
        owners.push(owner);
        kept_prefix[triangle_index + 1] = triangles.len();
    }

    let source_material_count = raw
        .material_textures
        .len()
        .max(raw.materials.len())
        .max(raw.material_animations.len())
        .max(
            original_triangles
                .iter()
                .map(|triangle| triangle.material as usize + 1)
                .max()
                .unwrap_or(0),
        );
    let source_texture_names = material_texture_names(raw);
    let used_materials = triangles
        .iter()
        .map(|triangle| triangle.material)
        .collect::<BTreeSet<_>>();
    let material_remap = used_materials
        .iter()
        .enumerate()
        .map(|(new, old)| (*old, new as u16))
        .collect::<BTreeMap<_, _>>();
    for triangle in &mut triangles {
        triangle.material = material_remap[&triangle.material];
    }
    let remapped_opaque = opaque_materials
        .iter()
        .filter_map(|material| material_remap.get(material).copied())
        .collect::<BTreeSet<_>>();
    raw.material_textures = used_materials
        .iter()
        .map(|index| {
            source_texture_names
                .get(*index as usize)
                .cloned()
                .unwrap_or_else(|| format!("material_{index:02}"))
        })
        .collect();
    raw.materials = used_materials
        .iter()
        .map(|index| {
            raw.materials
                .get(*index as usize)
                .copied()
                .unwrap_or_default()
        })
        .collect();
    raw.material_animations = used_materials
        .iter()
        .map(|index| {
            raw.material_animations
                .get(*index as usize)
                .cloned()
                .unwrap_or_default()
        })
        .collect();
    stats.materials_removed = source_material_count.saturating_sub(used_materials.len());

    let mut run_start = 0usize;
    while run_start < triangles.len() {
        let owner = owners[run_start];
        let material = triangles[run_start].material;
        let mut run_end = run_start + 1;
        while run_end < triangles.len()
            && owners[run_end] == owner
            && triangles[run_end].material == material
        {
            run_end += 1;
        }
        if remapped_opaque.contains(&material) {
            stats.triangles_reordered +=
                reorder_triangle_run_for_cache(&mut triangles[run_start..run_end]);
        }
        run_start = run_end;
    }

    for (component_index, component) in raw.components.iter_mut().enumerate() {
        let old_start = component.tri_start.min(original_triangle_count);
        let old_end = component
            .tri_end
            .min(original_triangle_count)
            .max(old_start);
        component.tri_start = kept_prefix[old_start];
        component.tri_end = kept_prefix[old_end];
        let mut component_vertices = BTreeSet::new();
        for (triangle, owner) in triangles.iter().zip(&owners) {
            if *owner == component_index {
                component_vertices.extend([
                    triangle.a as usize,
                    triangle.b as usize,
                    triangle.c as usize,
                ]);
            }
        }
        component.vertex_start = component_vertices.first().copied().unwrap_or(0);
        component.vertex_end = component_vertices
            .last()
            .map(|index| index + 1)
            .unwrap_or(component.vertex_start);
    }

    stats.invalid_vertices_removed = invalid_source_vertices.len();
    stats.unreferenced_vertices_removed = original_vertex_count
        .saturating_sub(used_source_vertices.len() + invalid_source_vertices.len());
    stats.vertices_welded = used_source_vertices.len().saturating_sub(vertices.len());
    stats.vertices_removed = original_vertex_count.saturating_sub(vertices.len());
    raw.vertices = vertices;
    raw.normals = normals;
    raw.uvs = uvs;
    raw.secondary_uvs = secondary_uvs;
    raw.prelit_colors = prelit_colors;
    raw.prelit_alphas = prelit_alphas;
    raw.night_prelit_colors = night_prelit_colors;
    raw.night_prelit_alphas = night_prelit_alphas;
    raw.light_flags = light_flags;
    raw.triangles = triangles;
    stats
}

fn identity_frame(frame: &RawMeshFrame) -> bool {
    frame.parent < 0
        && frame.right
            == (V3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            })
        && frame.up
            == (V3 {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            })
        && frame.at
            == (V3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            })
        && frame.pos == V3::default()
}

/// Returns true only when the normalized writer can serialize `raw` without
/// flattening a meaningful frame/component hierarchy.
pub(crate) fn raw_mesh_is_safe_for_normalized_rewrite(raw: &RawMesh, frame_name: &str) -> bool {
    let component_safe = match raw.components.as_slice() {
        [] => true,
        [component] => {
            component.vertex_start == 0
                && component.vertex_end == raw.vertices.len()
                && component.tri_start == 0
                && component.tri_end == raw.triangles.len()
                && (component.name.trim().is_empty()
                    || component.name.eq_ignore_ascii_case(frame_name))
        }
        _ => false,
    };
    let frame_safe = match raw.frames.as_slice() {
        [] => true,
        [frame] => {
            identity_frame(frame)
                && (frame.name.trim().is_empty() || frame.name.eq_ignore_ascii_case(frame_name))
        }
        _ => false,
    };
    component_safe && frame_safe
}

/// Returns true when the normalized writer can retain a DFF's frame list and
/// atomic-to-frame bindings.  Unlike `raw_mesh_is_safe_for_normalized_rewrite`,
/// this deliberately accepts real multi-frame models.
pub(crate) fn raw_mesh_is_safe_for_hierarchy_rewrite(raw: &RawMesh, frame_name: &str) -> bool {
    if raw.components.is_empty() {
        return raw.frames.is_empty()
            || (raw.frames.len() == 1
                && (raw.frames[0].name.trim().is_empty()
                    || raw.frames[0].name.eq_ignore_ascii_case(frame_name)));
    }
    if raw.components.iter().any(|component| {
        component.vertex_start > component.vertex_end
            || component.vertex_end > raw.vertices.len()
            || component.tri_start > component.tri_end
            || component.tri_end > raw.triangles.len()
            || component.tri_start == component.tri_end
            || component
                .frame_index
                .is_some_and(|frame| frame >= raw.frames.len())
    }) {
        return false;
    }
    raw.frames.iter().enumerate().all(|(index, frame)| {
        if frame.parent >= 0
            && ((frame.parent as usize) >= raw.frames.len() || frame.parent as usize == index)
        {
            return false;
        }
        let mut parent = frame.parent;
        let mut traversed = 0usize;
        while parent >= 0 {
            if traversed == raw.frames.len() {
                return false;
            }
            parent = raw.frames[parent as usize].parent;
            traversed += 1;
        }
        true
    })
}

fn raw_mesh_bounds_sphere(raw: &RawMesh) -> (V3, f32) {
    if raw.vertices.is_empty() {
        return (V3::default(), 0.0);
    }
    let bounds = bounds_from_vertices(&raw.vertices);
    let center = V3 {
        x: (bounds.min.x + bounds.max.x) * 0.5,
        y: (bounds.min.y + bounds.max.y) * 0.5,
        z: (bounds.min.z + bounds.max.z) * 0.5,
    };
    let center_mq = to_mq(center);
    let radius = raw
        .vertices
        .iter()
        .map(|vertex| (to_mq(*vertex) - center_mq).length())
        .fold(0.0f32, f32::max);
    (center, radius)
}

fn material_texture_names(raw: &RawMesh) -> Vec<String> {
    let max_mat = raw
        .triangles
        .iter()
        .map(|tri| tri.material as usize)
        .max()
        .unwrap_or(0);
    let count = raw.material_textures.len().max(max_mat + 1).max(1);
    (0..count)
        .map(|idx| {
            raw.material_textures
                .get(idx)
                .cloned()
                .unwrap_or_else(|| format!("material_{idx:02}"))
        })
        .collect()
}

fn material_animation_infos(raw: &RawMesh, count: usize) -> Vec<DffMaterialAnim> {
    (0..count)
        .map(|idx| {
            raw.material_animations
                .get(idx)
                .cloned()
                .unwrap_or_default()
        })
        .collect()
}

fn raw_has_material_animations(raw: &RawMesh) -> bool {
    raw.material_animations
        .iter()
        .any(|animation| animation.names.iter().any(|name| !name.trim().is_empty()))
}

fn material_animations_have_names(material_animations: &[DffMaterialAnim]) -> bool {
    material_animations
        .iter()
        .any(|animation| animation.names.iter().any(|name| !name.trim().is_empty()))
}

struct ExportMesh {
    vertices: Vec<V3>,
    uvs: Vec<V2>,
    secondary_uvs: Vec<Vec<V2>>,
    prelit_colors: Vec<V3>,
    prelit_alphas: Vec<f32>,
    night_prelit_colors: Vec<V3>,
    night_prelit_alphas: Vec<f32>,
    normals: Vec<V3>,
    triangles: Vec<Tri>,
    has_uvs: bool,
    has_prelit: bool,
    has_night_prelit: bool,
    include_normals: bool,
}

struct HierarchyExportMesh {
    mesh: ExportMesh,
    frame_index: usize,
    component_index: Option<usize>,
}

fn dot(a: V3, b: V3) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

fn normalized_frame_vector(value: V3) -> V3 {
    let length = (value.x * value.x + value.y * value.y + value.z * value.z).sqrt();
    if length > 0.0001 {
        V3 {
            x: value.x / length,
            y: value.y / length,
            z: value.z / length,
        }
    } else {
        value
    }
}

fn inverse_frame_point(frame: &RawMeshFrame, point: V3) -> V3 {
    let offset = V3 {
        x: point.x - frame.pos.x,
        y: point.y - frame.pos.y,
        z: point.z - frame.pos.z,
    };
    V3 {
        x: dot(offset, frame.right),
        y: dot(offset, frame.up),
        z: dot(offset, frame.at),
    }
}

fn inverse_frame_vector(frame: &RawMeshFrame, vector: V3) -> V3 {
    V3 {
        x: dot(vector, frame.right),
        y: dot(vector, frame.up),
        z: dot(vector, frame.at),
    }
}

fn component_frame_index(raw: &RawMesh, component: &RawMeshComponent) -> usize {
    component
        .frame_index
        .filter(|index| *index < raw.frames.len())
        .or_else(|| {
            (!component.name.trim().is_empty())
                .then(|| {
                    raw.frames
                        .iter()
                        .position(|frame| frame.name.eq_ignore_ascii_case(&component.name))
                })
                .flatten()
        })
        .unwrap_or(0)
}

fn build_hierarchy_export_meshes(raw: &RawMesh, include_normals: bool) -> Vec<HierarchyExportMesh> {
    if raw.components.is_empty() {
        return build_export_meshes(raw, include_normals)
            .into_iter()
            .map(|mesh| HierarchyExportMesh {
                mesh,
                frame_index: 0,
                component_index: None,
            })
            .collect();
    }

    let mut exports = Vec::new();
    for (component_index, component) in raw.components.iter().enumerate() {
        let tri_start = component.tri_start.min(raw.triangles.len());
        let tri_end = component.tri_end.min(raw.triangles.len());
        if tri_start >= tri_end {
            continue;
        }
        let frame_index = component_frame_index(raw, component);
        let mut component_raw = raw.clone();
        component_raw.triangles = raw.triangles[tri_start..tri_end].to_vec();
        if let Some(frame) = raw.frames.get(frame_index) {
            for vertex in &mut component_raw.vertices {
                *vertex = inverse_frame_point(frame, *vertex);
            }
            for normal in &mut component_raw.normals {
                *normal = normalized_frame_vector(inverse_frame_vector(frame, *normal));
            }
        }
        exports.extend(
            build_export_meshes(&component_raw, include_normals)
                .into_iter()
                .map(|mesh| HierarchyExportMesh {
                    mesh,
                    frame_index,
                    component_index: Some(component_index),
                }),
        );
    }
    exports
}

fn local_frame_transform(frames: &[RawMeshFrame], index: usize) -> RawMeshFrame {
    let frame = &frames[index];
    let Some(parent) = (frame.parent >= 0)
        .then_some(frame.parent as usize)
        .filter(|parent| *parent < frames.len())
        .and_then(|parent| frames.get(parent))
    else {
        return frame.clone();
    };
    RawMeshFrame {
        name: frame.name.clone(),
        parent: frame.parent,
        right: inverse_frame_vector(parent, frame.right),
        up: inverse_frame_vector(parent, frame.up),
        at: inverse_frame_vector(parent, frame.at),
        pos: inverse_frame_point(parent, frame.pos),
    }
}

fn build_export_meshes(raw: &RawMesh, include_normals: bool) -> Vec<ExportMesh> {
    const MAX_GEOMETRY_VERTICES: usize = u16::MAX as usize;

    fn q(value: f32, scale: f32) -> i32 {
        (value * scale).round() as i32
    }

    let has_uvs = raw.uvs.len() == raw.vertices.len();
    let secondary_uv_count = raw
        .secondary_uvs
        .iter()
        .filter(|uvs| uvs.len() == raw.vertices.len())
        .count();
    let has_prelit = raw.prelit_colors.len() == raw.vertices.len();
    let has_night_prelit = raw.night_prelit_colors.len() == raw.vertices.len();
    let source_normals = normalized_normals(raw);
    let mut meshes = Vec::new();
    let mut remap = BTreeMap::<ExportVertexKey, u32>::new();
    let mut out = new_export_mesh(
        has_uvs,
        secondary_uv_count,
        has_prelit,
        has_night_prelit,
        include_normals,
    );

    for tri in &raw.triangles {
        let Some(a_data) = export_vertex_data(
            raw,
            &source_normals,
            tri.a as usize,
            has_uvs,
            has_prelit,
            has_night_prelit,
        ) else {
            continue;
        };
        let Some(b_data) = export_vertex_data(
            raw,
            &source_normals,
            tri.b as usize,
            has_uvs,
            has_prelit,
            has_night_prelit,
        ) else {
            continue;
        };
        let Some(c_data) = export_vertex_data(
            raw,
            &source_normals,
            tri.c as usize,
            has_uvs,
            has_prelit,
            has_night_prelit,
        ) else {
            continue;
        };
        let mut new_keys = Vec::new();
        for data in [&a_data, &b_data, &c_data] {
            let key = export_vertex_key(data, q, include_normals);
            if !remap.contains_key(&key) && !new_keys.iter().any(|existing| existing == &key) {
                new_keys.push(key);
            }
        }
        if out.vertices.len() + new_keys.len() > MAX_GEOMETRY_VERTICES && !out.triangles.is_empty()
        {
            meshes.push(out);
            remap.clear();
            out = new_export_mesh(
                has_uvs,
                secondary_uv_count,
                has_prelit,
                has_night_prelit,
                include_normals,
            );
        }
        let Some(a) = map_export_vertex(&mut out, &mut remap, &a_data, q, include_normals) else {
            continue;
        };
        let Some(b) = map_export_vertex(&mut out, &mut remap, &b_data, q, include_normals) else {
            continue;
        };
        let Some(c) = map_export_vertex(&mut out, &mut remap, &c_data, q, include_normals) else {
            continue;
        };
        if a == b || b == c || a == c {
            continue;
        }
        out.triangles.push(Tri {
            a,
            b,
            c,
            material: tri.material,
        });
    }
    if !out.triangles.is_empty() {
        meshes.push(out);
    }
    meshes
}

/// Produces the exact vertex/triangle streams the normalized writer will feed
/// to RenderWare serialization. This is used by post-write verification so
/// intentional vertex welding and collapsed-triangle removal are not mistaken
/// for data loss.
pub(crate) fn raw_mesh_after_normalized_export(raw: &RawMesh, include_normals: bool) -> RawMesh {
    let mut out = normalize_export_prelight_streams(raw);
    let exports = build_export_meshes(&out, include_normals);
    out.vertices.clear();
    out.normals.clear();
    out.uvs.clear();
    out.secondary_uvs = vec![
        Vec::new();
        exports
            .first()
            .map(|mesh| mesh.secondary_uvs.len())
            .unwrap_or_default()
    ];
    out.prelit_colors.clear();
    out.prelit_alphas.clear();
    out.night_prelit_colors.clear();
    out.night_prelit_alphas.clear();
    out.light_flags.clear();
    out.triangles.clear();

    for export in exports {
        let vertex_offset = out.vertices.len() as u32;
        out.vertices.extend(export.vertices);
        if export.include_normals {
            out.normals.extend(export.normals);
        }
        if export.has_uvs {
            out.uvs.extend(export.uvs);
        }
        for (target, source) in out.secondary_uvs.iter_mut().zip(export.secondary_uvs) {
            target.extend(source);
        }
        if export.has_prelit {
            out.prelit_colors.extend(export.prelit_colors);
            out.prelit_alphas.extend(export.prelit_alphas);
        }
        if export.has_night_prelit {
            out.night_prelit_colors.extend(export.night_prelit_colors);
            out.night_prelit_alphas.extend(export.night_prelit_alphas);
        }
        out.triangles
            .extend(export.triangles.into_iter().map(|triangle| Tri {
                a: triangle.a + vertex_offset,
                b: triangle.b + vertex_offset,
                c: triangle.c + vertex_offset,
                material: triangle.material,
            }));
    }
    out
}

fn new_export_mesh(
    has_uvs: bool,
    secondary_uv_count: usize,
    has_prelit: bool,
    has_night_prelit: bool,
    include_normals: bool,
) -> ExportMesh {
    ExportMesh {
        vertices: Vec::new(),
        uvs: Vec::new(),
        secondary_uvs: vec![Vec::new(); secondary_uv_count],
        prelit_colors: Vec::new(),
        prelit_alphas: Vec::new(),
        night_prelit_colors: Vec::new(),
        night_prelit_alphas: Vec::new(),
        normals: Vec::new(),
        triangles: Vec::new(),
        has_uvs,
        has_prelit,
        has_night_prelit,
        include_normals,
    }
}

#[derive(Clone)]
struct ExportVertexData {
    vertex: V3,
    uv: V2,
    secondary_uvs: Vec<V2>,
    prelit: V3,
    prelit_alpha: f32,
    night_prelit: V3,
    night_prelit_alpha: f32,
    normal: V3,
}

type ExportVertexKey = Vec<i32>;

fn export_vertex_data(
    raw: &RawMesh,
    source_normals: &[V3],
    idx: usize,
    has_uvs: bool,
    has_prelit: bool,
    has_night_prelit: bool,
) -> Option<ExportVertexData> {
    let prelit = if has_prelit {
        raw.prelit_colors[idx]
    } else {
        V3::default()
    };
    Some(ExportVertexData {
        vertex: *raw.vertices.get(idx)?,
        uv: if has_uvs { raw.uvs[idx] } else { V2::default() },
        secondary_uvs: raw
            .secondary_uvs
            .iter()
            .filter(|uvs| uvs.len() == raw.vertices.len())
            .map(|uvs| uvs[idx])
            .collect(),
        prelit,
        prelit_alpha: if has_prelit && raw.prelit_alphas.len() == raw.vertices.len() {
            raw.prelit_alphas[idx]
        } else {
            1.0
        },
        night_prelit: if has_night_prelit {
            raw.night_prelit_colors[idx]
        } else {
            prelit
        },
        night_prelit_alpha: if has_night_prelit
            && raw.night_prelit_alphas.len() == raw.vertices.len()
        {
            raw.night_prelit_alphas[idx]
        } else if has_prelit && raw.prelit_alphas.len() == raw.vertices.len() {
            raw.prelit_alphas[idx]
        } else {
            1.0
        },
        normal: source_normals.get(idx).copied().unwrap_or(V3 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        }),
    })
}

fn export_vertex_key(
    data: &ExportVertexData,
    q: fn(f32, f32) -> i32,
    include_normals: bool,
) -> ExportVertexKey {
    let normal = if include_normals {
        (
            q(data.normal.x, 1000.0),
            q(data.normal.y, 1000.0),
            q(data.normal.z, 1000.0),
        )
    } else {
        (0, 0, 0)
    };
    let mut key = vec![
        q(data.vertex.x, 1000.0),
        q(data.vertex.y, 1000.0),
        q(data.vertex.z, 1000.0),
        q(data.uv.u, 100000.0),
        q(data.uv.v, 100000.0),
        q(data.prelit.x, 255.0),
        q(data.prelit.y, 255.0),
        q(data.prelit.z, 255.0),
        q(data.night_prelit.x, 255.0),
        q(data.night_prelit.y, 255.0),
        q(data.night_prelit.z, 255.0),
        normal.0,
        normal.1,
        normal.2,
    ];
    for uv in &data.secondary_uvs {
        key.extend([q(uv.u, 100000.0), q(uv.v, 100000.0)]);
    }
    key.extend([
        q(data.prelit_alpha, 255.0),
        q(data.night_prelit_alpha, 255.0),
    ]);
    key
}

fn map_export_vertex(
    out: &mut ExportMesh,
    remap: &mut BTreeMap<ExportVertexKey, u32>,
    data: &ExportVertexData,
    q: fn(f32, f32) -> i32,
    include_normals: bool,
) -> Option<u32> {
    let key = export_vertex_key(data, q, include_normals);
    if let Some(mapped) = remap.get(&key).copied() {
        return Some(mapped);
    }
    if out.vertices.len() >= u16::MAX as usize {
        return None;
    }
    let mapped = out.vertices.len() as u32;
    remap.insert(key, mapped);
    out.vertices.push(data.vertex);
    if out.has_uvs {
        out.uvs.push(data.uv);
    }
    for (out_uvs, uv) in out.secondary_uvs.iter_mut().zip(&data.secondary_uvs) {
        out_uvs.push(*uv);
    }
    if out.has_prelit {
        out.prelit_colors.push(data.prelit);
        out.prelit_alphas.push(data.prelit_alpha);
    }
    if out.has_night_prelit {
        out.night_prelit_colors.push(data.night_prelit);
        out.night_prelit_alphas.push(data.night_prelit_alpha);
    }
    if out.include_normals {
        out.normals.push(data.normal);
    }
    Some(mapped)
}

#[allow(dead_code)]
pub(crate) fn write_normalized_dff(raw: &RawMesh, frame_name: &str) -> Result<Vec<u8>, String> {
    write_normalized_dff_with_options(raw, frame_name, DffWriteOptions::default())
}

#[derive(Clone, Copy)]
pub(crate) struct DffWriteOptions {
    pub(crate) include_normals: bool,
    pub(crate) include_bin_mesh: bool,
}

impl Default for DffWriteOptions {
    fn default() -> Self {
        Self {
            include_normals: true,
            include_bin_mesh: true,
        }
    }
}

fn normalize_export_prelight_streams(raw: &RawMesh) -> RawMesh {
    let mut out = raw.clone();
    let vertex_count = out.vertices.len();
    let has_day = out.prelit_colors.len() == vertex_count;
    let has_night = out.night_prelit_colors.len() == vertex_count;
    match (has_day, has_night) {
        (true, false) => {
            out.night_prelit_colors = out.prelit_colors.clone();
            out.night_prelit_alphas = if out.prelit_alphas.len() == vertex_count {
                out.prelit_alphas.clone()
            } else {
                vec![1.0; vertex_count]
            };
        }
        (false, true) => {
            out.prelit_colors = out.night_prelit_colors.clone();
            out.prelit_alphas = if out.night_prelit_alphas.len() == vertex_count {
                out.night_prelit_alphas.clone()
            } else {
                vec![1.0; vertex_count]
            };
        }
        _ => {}
    }
    if out.prelit_colors.len() == vertex_count && out.prelit_alphas.len() != vertex_count {
        out.prelit_alphas = vec![1.0; vertex_count];
    }
    if out.night_prelit_colors.len() == vertex_count
        && out.night_prelit_alphas.len() != vertex_count
    {
        out.night_prelit_alphas = vec![1.0; vertex_count];
    }
    out
}

pub(crate) fn write_normalized_dff_with_options(
    raw: &RawMesh,
    frame_name: &str,
    options: DffWriteOptions,
) -> Result<Vec<u8>, String> {
    if raw.vertices.is_empty() || raw.triangles.is_empty() {
        return Err("DFF has no readable geometry".to_string());
    }
    let populated_secondary_uvs = raw
        .secondary_uvs
        .iter()
        .filter(|uvs| !uvs.is_empty())
        .collect::<Vec<_>>();
    if !populated_secondary_uvs.is_empty() && raw.uvs.len() != raw.vertices.len() {
        return Err(
            "DFF secondary UV streams require a vertex-aligned primary UV stream".to_string(),
        );
    }
    if let Some((index, uvs)) = raw
        .secondary_uvs
        .iter()
        .enumerate()
        .find(|(_, uvs)| !uvs.is_empty() && uvs.len() != raw.vertices.len())
    {
        return Err(format!(
            "DFF UV set {} has {} coordinates for {} vertices",
            index + 2,
            uvs.len(),
            raw.vertices.len()
        ));
    }
    if populated_secondary_uvs.len() > 7 {
        return Err(format!(
            "DFF has {} UV sets; RenderWare supports at most 8",
            populated_secondary_uvs.len() + 1
        ));
    }
    let raw = normalize_export_prelight_streams(raw);
    let exports = build_hierarchy_export_meshes(&raw, options.include_normals);
    if exports.is_empty() {
        return Err("DFF normalization produced no geometry".to_string());
    }
    let materials = material_texture_names(&raw);
    let material_animations = material_animation_infos(&raw, materials.len());
    let has_material_animations = raw_has_material_animations(&raw);

    let output_frames = if raw.frames.is_empty() {
        vec![RawMeshFrame {
            name: frame_name.to_string(),
            parent: -1,
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
        }]
    } else {
        (0..raw.frames.len())
            .map(|index| local_frame_transform(&raw.frames, index))
            .collect::<Vec<_>>()
    };
    let mut frame_struct = Vec::new();
    for frame in &output_frames {
        for value in [
            frame.right.x,
            frame.right.y,
            frame.right.z,
            frame.up.x,
            frame.up.y,
            frame.up.z,
            frame.at.x,
            frame.at.y,
            frame.at.z,
            frame.pos.x,
            frame.pos.y,
            frame.pos.z,
        ] {
            frame_struct.extend_from_slice(&value.to_le_bytes());
        }
        let parent = (frame.parent >= 0 && (frame.parent as usize) < output_frames.len())
            .then_some(frame.parent)
            .unwrap_or(-1);
        frame_struct.extend_from_slice(&parent.to_le_bytes());
        frame_struct.extend_from_slice(&0u32.to_le_bytes());
    }
    let mut frame_list = rw_chunk(0x01, {
        let mut data = Vec::new();
        data.extend_from_slice(&(output_frames.len() as u32).to_le_bytes());
        data.extend_from_slice(&frame_struct);
        data
    });
    for frame in &output_frames {
        let name = if frame.name.trim().is_empty() {
            frame_name
        } else {
            &frame.name
        };
        frame_list.extend_from_slice(&rw_chunk(0x03, rw_chunk(0x0253f2fe, rw_string(name))));
    }

    let mut geometry_list = rw_chunk(0x01, (exports.len() as u32).to_le_bytes().to_vec());
    for (export_idx, export) in exports.iter().enumerate() {
        let effects_2dfx = (export_idx + 1 == exports.len()).then_some(raw.effects_2dfx.as_slice());
        let breakable = export
            .component_index
            .and_then(|component| raw.components.get(component))
            .and_then(|component| component.breakable.as_ref());
        geometry_list.extend_from_slice(&write_geometry(
            &export.mesh,
            &materials,
            &raw.materials,
            &material_animations,
            effects_2dfx,
            breakable,
            options,
        )?);
    }

    let mut clump = rw_chunk(0x01, {
        let mut data = Vec::new();
        for value in [exports.len() as u32, 0, 0] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data
    });
    clump.extend_from_slice(&rw_chunk(0x0e, frame_list));
    clump.extend_from_slice(&rw_chunk(0x1a, geometry_list));
    for (geometry_idx, export) in exports.iter().enumerate() {
        clump.extend_from_slice(&write_atomic(
            export
                .frame_index
                .min(output_frames.len().saturating_sub(1)) as u32,
            geometry_idx as u32,
            has_material_animations,
        ));
    }
    clump.extend_from_slice(&rw_chunk(0x03, Vec::new()));
    // The UV Animation Dictionary MUST be streamed before the Clump: GTA:SA's
    // loader reads the dictionary first and registers each animation by name,
    // then reads the Clump and resolves every material's UV Animation PLG
    // reference against that registry. A dictionary placed after the Clump is
    // silently ignored in-game (our own importer scans the whole file, so the
    // editor preview still plays it either way).
    let mut out = Vec::new();
    if raw.uv_animations.is_empty() {
        for dictionary in &raw.uv_anim_dictionaries {
            if dictionary.len() >= 12 && rd32(dictionary, 0) == 0x2b {
                out.extend_from_slice(dictionary);
            }
        }
    } else {
        out.extend_from_slice(&write_uv_animation_dictionary(&raw.uv_animations));
    }
    out.extend_from_slice(&rw_chunk(0x10, clump));
    Ok(out)
}

fn write_geometry(
    export: &ExportMesh,
    material_textures: &[String],
    material_properties: &[RawMaterial],
    material_animations: &[DffMaterialAnim],
    effects_2dfx: Option<&[Dff2dEffect]>,
    breakable: Option<&BreakableGeometry>,
    options: DffWriteOptions,
) -> Result<Vec<u8>, String> {
    let has_uvs = export.has_uvs;
    let uv_count = usize::from(has_uvs) + export.secondary_uvs.len();
    let has_prelit = export.has_prelit;
    let has_normals = options.include_normals && export.normals.len() == export.vertices.len();

    let mut geometry_struct = Vec::new();
    let mut flags = 0x02u32 | 0x40;
    if has_normals {
        // rpGEOMETRYLIGHT only travels with rpGEOMETRYNORMALS: asking GTA:SA to
        // light a geometry that ships no normal stream leaves it reading
        // undefined normals, which renders as black or flickering faces.
        flags |= 0x10 | 0x20;
    }
    if has_uvs {
        flags |= 0x04 | ((uv_count as u32) << 16);
    }
    if has_prelit {
        flags |= 0x08;
    }
    geometry_struct.extend_from_slice(&flags.to_le_bytes());
    geometry_struct.extend_from_slice(&(export.triangles.len() as u32).to_le_bytes());
    geometry_struct.extend_from_slice(&(export.vertices.len() as u32).to_le_bytes());
    geometry_struct.extend_from_slice(&1u32.to_le_bytes());
    if has_prelit {
        for (index, color) in export.prelit_colors.iter().enumerate() {
            for channel in [color.x, color.y, color.z] {
                geometry_struct.push((channel.clamp(0.0, 1.0) * 255.0).round() as u8);
            }
            let alpha = export.prelit_alphas.get(index).copied().unwrap_or(1.0);
            geometry_struct.push((alpha.clamp(0.0, 1.0) * 255.0).round() as u8);
        }
    }
    if has_uvs {
        for uv in &export.uvs {
            geometry_struct.extend_from_slice(&uv.u.to_le_bytes());
            geometry_struct.extend_from_slice(&uv.v.to_le_bytes());
        }
        for uv_set in &export.secondary_uvs {
            for uv in uv_set {
                geometry_struct.extend_from_slice(&uv.u.to_le_bytes());
                geometry_struct.extend_from_slice(&uv.v.to_le_bytes());
            }
        }
    }
    for tri in &export.triangles {
        for value in [tri.b as u16, tri.a as u16, tri.material, tri.c as u16] {
            geometry_struct.extend_from_slice(&value.to_le_bytes());
        }
    }
    let bounds_raw = RawMesh {
        vertices: export.vertices.clone(),
        ..RawMesh::default()
    };
    let (center, radius) = raw_mesh_bounds_sphere(&bounds_raw);
    for value in [center.x, center.y, center.z, radius] {
        geometry_struct.extend_from_slice(&value.to_le_bytes());
    }
    geometry_struct.extend_from_slice(&1u32.to_le_bytes());
    geometry_struct.extend_from_slice(&(has_normals as u32).to_le_bytes());
    for vertex in &export.vertices {
        for value in [vertex.x, vertex.y, vertex.z] {
            geometry_struct.extend_from_slice(&value.to_le_bytes());
        }
    }
    if has_normals {
        for normal in &export.normals {
            for value in [normal.x, normal.y, normal.z] {
                geometry_struct.extend_from_slice(&value.to_le_bytes());
            }
        }
    }

    let mut material_list_struct = Vec::new();
    material_list_struct.extend_from_slice(&(material_textures.len() as u32).to_le_bytes());
    for _ in material_textures {
        material_list_struct.extend_from_slice(&(-1i32).to_le_bytes());
    }
    let mut material_list = rw_chunk(0x01, material_list_struct);
    for (material_idx, texture_name) in material_textures.iter().enumerate() {
        let properties = material_properties
            .get(material_idx)
            .copied()
            .unwrap_or(RawMaterial {
                color: V3 {
                    x: 1.0,
                    y: 1.0,
                    z: 1.0,
                },
                alpha: 1.0,
                ambient: 1.0,
                specular: 0.0,
                diffuse: 1.0,
            });
        let mut material_struct = Vec::new();
        material_struct.extend_from_slice(&0u32.to_le_bytes());
        for channel in [properties.color.x, properties.color.y, properties.color.z] {
            material_struct.push((channel.clamp(0.0, 1.0) * 255.0).round() as u8);
        }
        material_struct.push((properties.alpha.clamp(0.0, 1.0) * 255.0).round() as u8);
        material_struct.extend_from_slice(&1u32.to_le_bytes());
        material_struct.extend_from_slice(&(!texture_name.trim().is_empty() as u32).to_le_bytes());
        for value in [properties.ambient, properties.specular, properties.diffuse] {
            material_struct.extend_from_slice(&value.to_le_bytes());
        }
        let mut material = rw_chunk(0x01, material_struct);
        if !texture_name.trim().is_empty() {
            let mut texture_struct = Vec::new();
            texture_struct.extend_from_slice(&[6, 1, 0, 0]);
            let mut texture = rw_chunk(0x01, texture_struct);
            texture.extend_from_slice(&rw_chunk(0x02, rw_string(texture_name.trim())));
            texture.extend_from_slice(&rw_chunk(0x02, rw_string("")));
            texture.extend_from_slice(&rw_chunk(0x03, Vec::new()));
            material.extend_from_slice(&rw_chunk(0x06, texture));
        }
        let material_extension = material_animations
            .get(material_idx)
            .map(write_material_animation_extension)
            .unwrap_or_default();
        material.extend_from_slice(&rw_chunk(0x03, material_extension));
        material_list.extend_from_slice(&rw_chunk(0x07, material));
    }

    let mut bin_mesh = Vec::new();
    bin_mesh.extend_from_slice(&0u32.to_le_bytes());
    // A BinMesh may contain several batches with the same material. Keep
    // consecutive material runs instead of collecting every material into one
    // sorted map: the runtime consumes BinMesh order, so merging runs would
    // undo editor face-order optimizations (notably opaque-before-alpha).
    let mut groups = Vec::<(u16, Vec<u32>)>::new();
    for tri in &export.triangles {
        if groups
            .last()
            .is_none_or(|(material, _)| *material != tri.material)
        {
            groups.push((tri.material, Vec::new()));
        }
        groups
            .last_mut()
            .expect("a BinMesh group was just created")
            .1
            .extend([tri.b, tri.a, tri.c]);
    }
    let total_indices: usize = groups.iter().map(|(_, indices)| indices.len()).sum();
    bin_mesh.extend_from_slice(&(groups.len() as u32).to_le_bytes());
    bin_mesh.extend_from_slice(&(total_indices as u32).to_le_bytes());
    for (material, indices) in groups {
        bin_mesh.extend_from_slice(&(indices.len() as u32).to_le_bytes());
        bin_mesh.extend_from_slice(&(material as u32).to_le_bytes());
        for index in indices {
            bin_mesh.extend_from_slice(&index.to_le_bytes());
        }
    }

    let mut geometry = rw_chunk(0x01, geometry_struct);
    geometry.extend_from_slice(&rw_chunk(0x08, material_list));
    let mut effects_chunk = Vec::new();
    if let Some(effects) = effects_2dfx.filter(|effects| !effects.is_empty()) {
        let mut data = Vec::new();
        data.extend_from_slice(&(effects.len() as u32).to_le_bytes());
        for effect in effects {
            for value in [effect.position.x, effect.position.y, effect.position.z] {
                data.extend_from_slice(&value.to_le_bytes());
            }
            data.extend_from_slice(&effect.effect_id.to_le_bytes());
            data.extend_from_slice(&(effect.payload.len() as u32).to_le_bytes());
            data.extend_from_slice(&effect.payload);
        }
        effects_chunk = rw_chunk(0x0253_f2f8, data);
    }
    let breakable_chunk = breakable
        .map(encode_breakable_plugin)
        .transpose()?
        .map(|data| rw_chunk(BREAKABLE_PLUGIN_ID, data))
        .unwrap_or_default();

    let has_material_animations = material_animations_have_names(material_animations);
    let right_to_render = has_material_animations.then(|| {
        let mut data = Vec::new();
        data.extend_from_slice(&0x0116u32.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        rw_chunk(0x1f, data)
    });

    let extension = if options.include_bin_mesh {
        let mut extension = rw_chunk(0x050e, bin_mesh);
        if let Some(right_to_render) = right_to_render.as_ref() {
            extension.extend_from_slice(right_to_render);
        }
        if export.has_night_prelit && export.night_prelit_colors.len() == export.vertices.len() {
            let mut night_colors = 1u32.to_le_bytes().to_vec();
            for (index, color) in export.night_prelit_colors.iter().enumerate() {
                for channel in [color.x, color.y, color.z] {
                    night_colors.push((channel.clamp(0.0, 1.0) * 255.0).round() as u8);
                }
                let alpha = export
                    .night_prelit_alphas
                    .get(index)
                    .copied()
                    .unwrap_or(1.0);
                night_colors.push((alpha.clamp(0.0, 1.0) * 255.0).round() as u8);
            }
            extension.extend_from_slice(&rw_chunk(0x0253_f2f9, night_colors));
        }
        extension.extend_from_slice(&effects_chunk);
        extension.extend_from_slice(&breakable_chunk);
        extension
    } else {
        let mut extension = Vec::new();
        if let Some(right_to_render) = right_to_render.as_ref() {
            extension.extend_from_slice(right_to_render);
        }
        if export.has_night_prelit && export.night_prelit_colors.len() == export.vertices.len() {
            let mut night_colors = 1u32.to_le_bytes().to_vec();
            for (index, color) in export.night_prelit_colors.iter().enumerate() {
                for channel in [color.x, color.y, color.z] {
                    night_colors.push((channel.clamp(0.0, 1.0) * 255.0).round() as u8);
                }
                let alpha = export
                    .night_prelit_alphas
                    .get(index)
                    .copied()
                    .unwrap_or(1.0);
                night_colors.push((alpha.clamp(0.0, 1.0) * 255.0).round() as u8);
            }
            extension.extend_from_slice(&rw_chunk(0x0253_f2f9, night_colors));
        }
        extension.extend_from_slice(&effects_chunk);
        extension.extend_from_slice(&breakable_chunk);
        extension
    };
    geometry.extend_from_slice(&rw_chunk(0x03, extension));
    let geometry = rw_chunk(0x0f, geometry);
    Ok(geometry)
}

fn write_material_animation_extension(animation: &DffMaterialAnim) -> Vec<u8> {
    let names = animation
        .names
        .iter()
        .map(|name| name.trim())
        .filter(|name| !name.is_empty())
        .take(8)
        .collect::<Vec<_>>();
    if names.is_empty() {
        return Vec::new();
    }

    let mut matfx = Vec::new();
    matfx.extend_from_slice(&5u32.to_le_bytes());
    matfx.extend_from_slice(&5u32.to_le_bytes());
    matfx.extend_from_slice(&0u32.to_le_bytes());

    let mut uv_anim = Vec::new();
    uv_anim.extend_from_slice(&((1u32 << names.len()) - 1).to_le_bytes());
    for name in names {
        let mut fixed = [0u8; 32];
        let bytes = name.as_bytes();
        let len = bytes.len().min(31);
        fixed[..len].copy_from_slice(&bytes[..len]);
        uv_anim.extend_from_slice(&fixed);
    }

    let mut out = rw_chunk(0x0120, matfx);
    out.extend_from_slice(&rw_chunk(0x0135, rw_chunk(0x01, uv_anim)));
    out
}

fn write_uv_animation_dictionary(animations: &[DffUvAnimation]) -> Vec<u8> {
    let animations = animations
        .iter()
        .filter(|animation| !animation.name.trim().is_empty())
        .collect::<Vec<_>>();
    let mut payload = rw_chunk(0x01, (animations.len() as u32).to_le_bytes().to_vec());
    for animation in animations {
        payload.extend_from_slice(&write_uv_animation_anim(animation));
    }
    rw_chunk(0x2b, payload)
}

fn write_uv_animation_anim(animation: &DffUvAnimation) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&0x100i32.to_le_bytes());
    data.extend_from_slice(&animation.type_id.to_le_bytes());
    data.extend_from_slice(&(animation.frames.len() as i32).to_le_bytes());
    data.extend_from_slice(&animation.flags.to_le_bytes());
    data.extend_from_slice(&animation.duration.max(0.0).to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    let mut fixed = [0u8; 32];
    let bytes = animation.name.trim().as_bytes();
    let len = bytes.len().min(31);
    fixed[..len].copy_from_slice(&bytes[..len]);
    data.extend_from_slice(&fixed);
    for value in animation.node_to_uv {
        data.extend_from_slice(&value.to_le_bytes());
    }
    for frame in &animation.frames {
        data.extend_from_slice(&frame.time.to_le_bytes());
        for value in frame.uv {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(&frame.prev.to_le_bytes());
    }
    rw_chunk(0x1b, data)
}

fn write_atomic(frame_idx: u32, geometry_idx: u32, has_material_animations: bool) -> Vec<u8> {
    rw_chunk(0x14, {
        let mut atomic = rw_chunk(0x01, {
            let mut data = Vec::new();
            for value in [frame_idx, geometry_idx, 5, 0] {
                data.extend_from_slice(&value.to_le_bytes());
            }
            data
        });
        let extension = if has_material_animations {
            rw_chunk(0x0120, 1u32.to_le_bytes().to_vec())
        } else {
            Vec::new()
        };
        atomic.extend_from_slice(&rw_chunk(0x03, extension));
        atomic
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn color(r: u8, g: u8, b: u8) -> V3 {
        V3 {
            x: r as f32 / 255.0,
            y: g as f32 / 255.0,
            z: b as f32 / 255.0,
        }
    }

    fn test_raw_mesh() -> RawMesh {
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
            normals: vec![
                V3 {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                };
                3
            ],
            uvs: vec![
                V2 { u: 0.0, v: 0.0 },
                V2 { u: 1.0, v: 0.0 },
                V2 { u: 0.0, v: 1.0 },
            ],
            prelit_colors: vec![
                V3 {
                    x: 1.0,
                    y: 1.0,
                    z: 1.0,
                };
                3
            ],
            material_textures: vec!["test_texture".to_string()],
            triangles: vec![Tri {
                a: 0,
                b: 1,
                c: 2,
                material: 0,
            }],
            ..RawMesh::default()
        }
    }

    #[test]
    fn missing_normals_are_reconstructed_for_vertical_faces() {
        let raw = RawMesh {
            vertices: vec![
                V3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                V3 {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
                V3 {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                },
            ],
            triangles: vec![Tri {
                a: 0,
                b: 1,
                c: 2,
                material: 0,
            }],
            ..RawMesh::default()
        };

        let normals = normalized_normals(&raw);

        assert_eq!(normals.len(), 3);
        assert!(normals.iter().all(|normal| normal.x < -0.99));
        assert!(normals.iter().all(|normal| normal.z.abs() < 0.001));
    }

    #[test]
    fn cleanup_rewrites_only_dff_material_texture_references() {
        let old = "abcdefghijklmnopqrstuvwxyz123456";
        let new = "abcdefghijklmnopqrstuvwxyz123_1";
        let mut raw = test_raw_mesh();
        raw.material_textures = vec![old.to_string()];
        let mut dff = write_normalized_dff(&raw, old).unwrap();
        let renames = HashMap::from([(old.to_string(), new.to_string())]);

        assert_eq!(rename_dff_material_textures(&mut dff, &renames), 1);
        let parsed = parse_dff_mesh(&dff);
        assert_eq!(parsed.material_textures, vec![new.to_string()]);
        // The frame name is also an RW string, but it is not a material texture.
        assert!(
            dff.windows(old.len())
                .any(|window| window == old.as_bytes())
        );
    }

    #[test]
    fn rewrite_supports_a_longer_material_texture_name() {
        let old = "road";
        let new = "city_road_markings_01";
        let mut raw = test_raw_mesh();
        raw.material_textures = vec![old.to_string()];
        let dff = write_normalized_dff(&raw, "test_frame").unwrap();
        let renames = HashMap::from([(old.to_string(), new.to_string())]);

        let (updated, changed) = rewrite_dff_material_textures(&dff, &renames).unwrap();

        assert_eq!(changed, 1);
        assert_eq!(parse_dff_mesh(&updated).material_textures, vec![new]);
    }

    #[test]
    fn missing_normals_follow_clockwise_renderware_front_face() {
        let raw = RawMesh {
            // Clockwise when viewed from above, so the visible normal is +Z.
            vertices: vec![
                V3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                V3 {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
                V3 {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
            ],
            triangles: vec![Tri {
                a: 0,
                b: 1,
                c: 2,
                material: 0,
            }],
            ..RawMesh::default()
        };

        let normals = normalized_normals(&raw);

        assert!(normals.iter().all(|normal| normal.z > 0.99));
    }

    fn first_geometry_flags_and_has_normals(bytes: &[u8]) -> Option<(u32, u32)> {
        fn scan(bytes: &[u8], start: usize, end: usize) -> Option<(u32, u32)> {
            let mut o = start;
            while o + 12 <= end {
                let id = rd32(bytes, o);
                let size = rd32(bytes, o + 4) as usize;
                let cs = o + 12;
                let ce = cs.saturating_add(size);
                if ce > end || ce > bytes.len() {
                    return None;
                }
                if id == 0x0f {
                    let mut child = cs;
                    while child + 12 <= ce {
                        let child_id = rd32(bytes, child);
                        let child_size = rd32(bytes, child + 4) as usize;
                        let child_start = child + 12;
                        let child_end = child_start.saturating_add(child_size);
                        if child_end > ce || child_end > bytes.len() {
                            return None;
                        }
                        if child_id == 0x01 {
                            let flags = rd32(bytes, child_start);
                            let tri_count = rd32(bytes, child_start + 4) as usize;
                            let vert_count = rd32(bytes, child_start + 8) as usize;
                            let uv_count = ((flags >> 16) & 0xff) as usize;
                            let mut p = child_start + 16;
                            if flags & 0x08 != 0 {
                                p += vert_count * 4;
                            }
                            p += uv_count * vert_count * 8;
                            p += tri_count * 8;
                            p += 16;
                            p += 4;
                            return Some((flags, rd32(bytes, p)));
                        }
                        child = child_end;
                    }
                } else if matches!(id, 0x10 | 0x0e | 0x1a) {
                    if let Some(found) = scan(bytes, cs, ce) {
                        return Some(found);
                    }
                }
                o = ce;
            }
            None
        }
        scan(bytes, 0, dff_chunk_len(bytes))
    }

    fn contains_chunk(bytes: &[u8], target: u32) -> bool {
        fn scan(bytes: &[u8], start: usize, end: usize, target: u32) -> bool {
            let mut o = start;
            while o + 12 <= end {
                let id = rd32(bytes, o);
                let size = rd32(bytes, o + 4) as usize;
                let cs = o + 12;
                let ce = cs.saturating_add(size);
                if ce > end || ce > bytes.len() {
                    return false;
                }
                if id == target || scan(bytes, cs, ce, target) {
                    return true;
                }
                o = ce;
            }
            false
        }
        scan(bytes, 0, dff_chunk_len(bytes), target)
    }

    fn count_top_level_chunks(bytes: &[u8], target: u32) -> usize {
        let mut count = 0usize;
        let mut o = 0usize;
        while o + 12 <= bytes.len() {
            let id = rd32(bytes, o);
            let size = rd32(bytes, o + 4) as usize;
            let end = o.saturating_add(12).saturating_add(size);
            if end > bytes.len() {
                break;
            }
            if id == target {
                count += 1;
            }
            o = end;
        }
        count
    }

    #[test]
    fn write_options_can_omit_normals_for_building_dffs() {
        let bytes = write_normalized_dff_with_options(
            &test_raw_mesh(),
            "building",
            DffWriteOptions {
                include_normals: false,
                include_bin_mesh: true,
            },
        )
        .unwrap();

        let (flags, has_normals) = first_geometry_flags_and_has_normals(&bytes).unwrap();
        assert_eq!(flags & 0x10, 0);
        assert_eq!(has_normals, 0);
        assert!(contains_chunk(&bytes, 0x050e));
    }

    #[test]
    fn default_write_options_keep_normals_and_bin_mesh() {
        let bytes = write_normalized_dff(&test_raw_mesh(), "object").unwrap();

        let (flags, has_normals) = first_geometry_flags_and_has_normals(&bytes).unwrap();
        assert_ne!(flags & 0x10, 0);
        assert_eq!(has_normals, 1);
        assert!(contains_chunk(&bytes, 0x050e));
    }

    #[test]
    fn bin_mesh_round_trip_preserves_cross_material_triangle_order() {
        let mut raw = test_raw_mesh();
        raw.vertices.extend([
            V3 {
                x: 2.0,
                y: 0.0,
                z: 0.0,
            },
            V3 {
                x: 3.0,
                y: 0.0,
                z: 0.0,
            },
            V3 {
                x: 2.0,
                y: 1.0,
                z: 0.0,
            },
        ]);
        raw.normals.extend(
            [V3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            }; 3],
        );
        raw.uvs.extend([V2 { u: 0.0, v: 0.0 }; 3]);
        raw.materials.push(RawMaterial::default());
        raw.material_textures.push("alpha".to_string());
        raw.triangles = vec![
            Tri {
                a: 0,
                b: 1,
                c: 2,
                material: 0,
            },
            Tri {
                a: 3,
                b: 4,
                c: 5,
                material: 1,
            },
            Tri {
                a: 2,
                b: 1,
                c: 0,
                material: 0,
            },
        ];

        let bytes = write_normalized_dff(&raw, "ordered").unwrap();
        let reparsed = parse_dff_mesh_preserving_topology(&bytes);

        assert_eq!(
            reparsed
                .triangles
                .iter()
                .map(|triangle| triangle.material)
                .collect::<Vec<_>>(),
            vec![0, 1, 0],
            "the runtime BinMesh must retain the staged face ordering"
        );
    }

    #[test]
    fn export_preserves_distinct_night_prelight_when_day_vertices_match() {
        let mut raw = test_raw_mesh();
        raw.vertices.push(raw.vertices[0]);
        raw.normals.push(raw.normals[0]);
        raw.uvs.push(raw.uvs[0]);
        raw.prelit_colors.push(raw.prelit_colors[0]);
        raw.night_prelit_colors = vec![
            color(8, 16, 24),
            color(32, 40, 48),
            color(56, 64, 72),
            color(200, 208, 216),
        ];
        raw.triangles.push(Tri {
            a: 3,
            b: 2,
            c: 1,
            material: 0,
        });

        let bytes = write_normalized_dff(&raw, "object").unwrap();
        let reparsed = parse_dff_mesh(&bytes);

        assert!(contains_chunk(&bytes, 0x0253_f2f9));
        assert_eq!(reparsed.vertices.len(), 4);
        assert_eq!(reparsed.night_prelit_colors.len(), 4);
        assert!(
            reparsed
                .night_prelit_colors
                .iter()
                .any(|value| *value == color(8, 16, 24))
        );
        assert!(
            reparsed
                .night_prelit_colors
                .iter()
                .any(|value| *value == color(200, 208, 216))
        );
    }

    #[test]
    fn normalized_round_trip_preserves_secondary_uvs_and_prelight_alpha() {
        let mut raw = test_raw_mesh();
        raw.secondary_uvs = vec![
            vec![
                V2 { u: 0.25, v: 0.75 },
                V2 { u: 0.5, v: 0.25 },
                V2 { u: 0.75, v: 0.5 },
            ],
            vec![
                V2 { u: -1.0, v: 2.0 },
                V2 { u: 3.0, v: -4.0 },
                V2 { u: 5.0, v: 6.0 },
            ],
        ];
        raw.prelit_alphas = vec![32.0 / 255.0, 128.0 / 255.0, 224.0 / 255.0];
        raw.night_prelit_colors = vec![color(16, 32, 48), color(64, 80, 96), color(112, 128, 144)];
        raw.night_prelit_alphas = vec![48.0 / 255.0, 144.0 / 255.0, 240.0 / 255.0];

        let bytes = write_normalized_dff(&raw, "stream_round_trip").unwrap();
        let reparsed = parse_dff_mesh(&bytes);

        assert_eq!(reparsed.uvs, raw.uvs);
        assert_eq!(reparsed.secondary_uvs, raw.secondary_uvs);
        assert_eq!(reparsed.prelit_alphas, raw.prelit_alphas);
        assert_eq!(reparsed.night_prelit_alphas, raw.night_prelit_alphas);
    }

    #[test]
    fn normalized_export_rejects_misaligned_secondary_uv_stream() {
        let mut raw = test_raw_mesh();
        raw.secondary_uvs = vec![vec![V2 { u: 0.5, v: 0.5 }]];

        let error = write_normalized_dff(&raw, "bad_uv_stream").unwrap_err();

        assert!(error.contains("UV set 2"));
        assert!(error.contains("1 coordinates for 3 vertices"));
    }

    #[test]
    fn export_round_trips_material_uv_animation_refs_and_dictionary() {
        let mut raw = test_raw_mesh();
        raw.material_animations = vec![DffMaterialAnim {
            names: vec!["scroll_sign".to_string()],
        }];
        raw.uv_animations.push(DffUvAnimation {
            name: "scroll_sign".to_string(),
            type_id: 0x1c1,
            flags: 0,
            duration: 1.0,
            node_to_uv: [0, 1, 0, 0, 0, 0, 0, 0],
            frames: vec![
                DffUvAnimFrame {
                    time: 0.0,
                    uv: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
                    prev: -1,
                },
                DffUvAnimFrame {
                    time: 1.0,
                    uv: [1.0, 0.0, 0.25, 0.0, 1.0, 0.0],
                    prev: 0,
                },
            ],
        });

        let bytes = write_normalized_dff(&raw, "animated").unwrap();
        let reparsed = parse_dff_mesh(&bytes);

        assert!(contains_chunk(&bytes, 0x0120));
        assert!(contains_chunk(&bytes, 0x0135));
        assert!(contains_chunk(&bytes, 0x1f));
        assert_eq!(count_top_level_chunks(&bytes, 0x2b), 1);
        assert_eq!(
            reparsed
                .material_animations
                .first()
                .map(|animation| animation.names.as_slice()),
            Some(&["scroll_sign".to_string()][..])
        );
        assert_eq!(reparsed.uv_anim_dictionaries.len(), 1);
        assert_eq!(reparsed.uv_animations.len(), 1);
        assert_eq!(reparsed.uv_animations[0].name, "scroll_sign");
        assert_eq!(reparsed.uv_animations[0].frames.len(), 2);
        assert_eq!(reparsed.uv_animations[0].frames[1].uv[2], 0.25);
    }

    #[test]
    fn repair_moves_uv_anim_dictionary_before_clump() {
        // A DFF written with the dictionary trailing the clump (the in-game bug):
        // GTA:SA ignores a dictionary that appears after the Clump.
        let dict = rw_chunk(0x2b, vec![0xaa, 0xbb, 0xcc, 0xdd]);
        let clump = rw_chunk(0x10, vec![0x01, 0x02, 0x03, 0x04]);
        let mut broken = clump.clone();
        broken.extend_from_slice(&dict);

        let fixed = repair_dff_uv_anim_dictionary_order(&broken)
            .expect("dictionary trailing the clump should be reordered");

        // Dictionary now leads, clump follows, and nothing was lost.
        assert_eq!(rd32(&fixed, 0), 0x2b);
        assert_eq!(count_top_level_chunks(&fixed, 0x2b), 1);
        assert_eq!(count_top_level_chunks(&fixed, 0x10), 1);
        assert_eq!(fixed.len(), broken.len());

        let mut expected = dict.clone();
        expected.extend_from_slice(&clump);
        assert_eq!(fixed, expected);
    }

    #[test]
    fn repair_leaves_correctly_ordered_uv_anim_dictionary_untouched() {
        let dict = rw_chunk(0x2b, vec![0xaa, 0xbb, 0xcc, 0xdd]);
        let clump = rw_chunk(0x10, vec![0x01, 0x02, 0x03, 0x04]);
        let mut ordered = dict;
        ordered.extend_from_slice(&clump);

        assert!(repair_dff_uv_anim_dictionary_order(&ordered).is_none());
    }

    #[test]
    fn repair_written_dff_with_broken_dictionary_order() {
        let mut raw = test_raw_mesh();
        raw.material_animations = vec![DffMaterialAnim {
            names: vec!["scroll_sign".to_string()],
        }];
        raw.uv_animations.push(DffUvAnimation {
            name: "scroll_sign".to_string(),
            type_id: 0x1c1,
            flags: 0,
            duration: 1.0,
            node_to_uv: [0, 1, 0, 0, 0, 0, 0, 0],
            frames: vec![DffUvAnimFrame {
                time: 0.0,
                uv: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
                prev: -1,
            }],
        });

        // Correct export already streams the dictionary first.
        let good = write_normalized_dff(&raw, "animated").unwrap();
        // Manufacture the broken layout: strip the leading dictionary and re-append it.
        let dict_size = 12 + rd32(&good, 4) as usize;
        assert_eq!(rd32(&good, 0), 0x2b);
        let mut broken = good[dict_size..].to_vec();
        broken.extend_from_slice(&good[..dict_size]);
        assert_eq!(rd32(&broken, 0), 0x10);

        let fixed =
            repair_dff_uv_anim_dictionary_order(&broken).expect("broken export should be repaired");
        // After repair the parse recovers the animation exactly as the good export.
        let reparsed = parse_dff_mesh(&fixed);
        assert_eq!(rd32(&fixed, 0), 0x2b);
        assert_eq!(reparsed.uv_animations.len(), 1);
        assert_eq!(reparsed.uv_animations[0].name, "scroll_sign");
    }

    #[test]
    fn export_round_trips_dff_2dfx_entries() {
        let mut raw = test_raw_mesh();
        raw.effects_2dfx.push(Dff2dEffect {
            position: V3 {
                x: 1.5,
                y: 2.5,
                z: 3.5,
            },
            effect_id: 1,
            payload: b"steam\0".to_vec(),
        });

        let bytes = write_normalized_dff(&raw, "effected").unwrap();
        let reparsed = parse_dff_mesh(&bytes);

        assert!(contains_chunk(&bytes, 0x0253_f2f8));
        assert_eq!(reparsed.effects_2dfx.len(), 1);
        assert_eq!(reparsed.effects_2dfx[0].effect_id, 1);
        assert_eq!(reparsed.effects_2dfx[0].position.x, 1.5);
        assert_eq!(reparsed.effects_2dfx[0].payload, b"steam\0");
    }

    #[test]
    fn lossless_compaction_removes_junk_and_welds_complete_attribute_matches() {
        let mut raw = RawMesh {
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
                V3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                V3 {
                    x: 1.0,
                    y: 1.0,
                    z: 0.0,
                },
                V3 {
                    x: 9.0,
                    y: 9.0,
                    z: 9.0,
                },
            ],
            normals: vec![
                V3 {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0
                };
                6
            ],
            uvs: vec![
                V2 { u: 0.0, v: 0.0 },
                V2 { u: 1.0, v: 0.0 },
                V2 { u: 0.0, v: 1.0 },
                V2 { u: 0.0, v: 0.0 },
                V2 { u: 1.0, v: 1.0 },
                V2 { u: 0.5, v: 0.5 },
            ],
            secondary_uvs: vec![vec![
                V2 { u: 0.25, v: 0.75 },
                V2 { u: 0.5, v: 0.75 },
                V2 { u: 0.25, v: 0.5 },
                V2 { u: 0.25, v: 0.75 },
                V2 { u: 0.75, v: 0.25 },
                V2 { u: 0.5, v: 0.5 },
            ]],
            prelit_colors: vec![neutral_vertex_color(); 6],
            prelit_alphas: vec![0.25, 0.5, 0.75, 0.25, 1.0, 0.125],
            night_prelit_colors: vec![neutral_vertex_color(); 6],
            night_prelit_alphas: vec![0.125, 0.375, 0.625, 0.125, 0.875, 1.0],
            triangles: vec![
                Tri {
                    a: 0,
                    b: 1,
                    c: 2,
                    material: 2,
                },
                Tri {
                    a: 3,
                    b: 2,
                    c: 4,
                    material: 2,
                },
                Tri {
                    a: 1,
                    b: 2,
                    c: 0,
                    material: 2,
                },
                Tri {
                    a: 0,
                    b: 0,
                    c: 1,
                    material: 2,
                },
                Tri {
                    a: 0,
                    b: 1,
                    c: 99,
                    material: 2,
                },
            ],
            material_textures: vec!["unused_0".into(), "unused_1".into(), "used".into()],
            ..RawMesh::default()
        };

        let stats = compact_raw_mesh_lossless(&mut raw, &BTreeSet::new());

        assert_eq!(raw.vertices.len(), 4);
        assert_eq!(raw.secondary_uvs[0].len(), 4);
        assert_eq!(raw.prelit_alphas, vec![0.25, 0.5, 0.75, 1.0]);
        assert_eq!(raw.night_prelit_alphas, vec![0.125, 0.375, 0.625, 0.875]);
        assert_eq!(raw.triangles.len(), 2);
        assert_eq!(raw.material_textures, vec!["used"]);
        assert!(raw.triangles.iter().all(|triangle| triangle.material == 0));
        assert_eq!(stats.vertices_removed, 2);
        assert_eq!(stats.unreferenced_vertices_removed, 1);
        assert_eq!(stats.vertices_welded, 1);
        assert_eq!(stats.duplicate_triangles_removed, 1);
        assert_eq!(stats.degenerate_triangles_removed, 1);
        assert_eq!(stats.invalid_triangles_removed, 1);
        assert_eq!(stats.materials_removed, 2);
    }

    #[test]
    fn lossless_compaction_remaps_material_parallel_arrays() {
        let mut raw = RawMesh {
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
                V3 {
                    x: 2.0,
                    y: 0.0,
                    z: 0.0,
                },
            ],
            triangles: vec![
                Tri {
                    a: 0,
                    b: 1,
                    c: 2,
                    material: 3,
                },
                Tri {
                    a: 1,
                    b: 3,
                    c: 2,
                    material: 1,
                },
            ],
            material_textures: vec!["zero".into(), "one".into(), "two".into(), "three".into()],
            materials: (0..4)
                .map(|index| RawMaterial {
                    ambient: index as f32,
                    ..RawMaterial::default()
                })
                .collect(),
            material_animations: (0..4)
                .map(|index| DffMaterialAnim {
                    names: vec![format!("anim_{index}")],
                })
                .collect(),
            ..RawMesh::default()
        };

        compact_raw_mesh_lossless(&mut raw, &BTreeSet::new());

        assert_eq!(raw.material_textures, vec!["one", "three"]);
        assert_eq!(
            raw.materials
                .iter()
                .map(|material| material.ambient)
                .collect::<Vec<_>>(),
            vec![1.0, 3.0]
        );
        assert_eq!(raw.material_animations[0].names, vec!["anim_1"]);
        assert_eq!(raw.material_animations[1].names, vec!["anim_3"]);
        assert_eq!(raw.triangles[0].material, 1);
        assert_eq!(raw.triangles[1].material, 0);
    }

    #[test]
    fn lossless_compaction_is_idempotent() {
        let mut raw = test_raw_mesh();
        let first_stats = compact_raw_mesh_lossless(&mut raw, &BTreeSet::from([0]));
        let once = raw.clone();
        let second_stats = compact_raw_mesh_lossless(&mut raw, &BTreeSet::from([0]));

        assert!(raw == once);
        assert_eq!(first_stats, DffLosslessCompactionStats::default());
        assert_eq!(second_stats, DffLosslessCompactionStats::default());
    }

    #[test]
    fn opaque_cache_ordering_is_deterministic_and_preserves_winding() {
        let mut raw = RawMesh {
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
                V3 {
                    x: 1.0,
                    y: 1.0,
                    z: 0.0,
                },
                V3 {
                    x: 10.0,
                    y: 0.0,
                    z: 0.0,
                },
                V3 {
                    x: 11.0,
                    y: 0.0,
                    z: 0.0,
                },
                V3 {
                    x: 10.0,
                    y: 1.0,
                    z: 0.0,
                },
            ],
            triangles: vec![
                Tri {
                    a: 0,
                    b: 1,
                    c: 2,
                    material: 0,
                },
                Tri {
                    a: 4,
                    b: 5,
                    c: 6,
                    material: 0,
                },
                Tri {
                    a: 1,
                    b: 3,
                    c: 2,
                    material: 0,
                },
            ],
            material_textures: vec!["opaque".into()],
            ..RawMesh::default()
        };
        let mut repeat = raw.clone();

        let stats = compact_raw_mesh_lossless(&mut raw, &BTreeSet::from([0]));
        compact_raw_mesh_lossless(&mut repeat, &BTreeSet::from([0]));

        assert!(raw == repeat);
        assert_eq!(stats.triangles_reordered, 2);
        assert_eq!(
            [raw.triangles[0].a, raw.triangles[0].b, raw.triangles[0].c],
            [0, 1, 2]
        );
        assert_eq!(
            [raw.triangles[1].a, raw.triangles[1].b, raw.triangles[1].c],
            [1, 6, 2]
        );
        assert_eq!(
            [raw.triangles[2].a, raw.triangles[2].b, raw.triangles[2].c],
            [3, 4, 5]
        );
    }

    #[test]
    fn normalized_writer_preserves_material_surface_properties() {
        let mut raw = test_raw_mesh();
        raw.materials = vec![RawMaterial {
            color: V3 {
                x: 64.0 / 255.0,
                y: 128.0 / 255.0,
                z: 192.0 / 255.0,
            },
            alpha: 128.0 / 255.0,
            ambient: 0.75,
            specular: 0.25,
            diffuse: 0.5,
        }];

        let bytes = write_normalized_dff(&raw, "material_test").unwrap();
        let reparsed = parse_dff_mesh(&bytes);

        assert!(reparsed.materials[0] == raw.materials[0]);
    }

    #[test]
    fn compaction_preserves_multi_component_identity_and_guard_rejects_flattening() {
        let identity = |name: &str, parent| RawMeshFrame {
            name: name.into(),
            parent,
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
        };
        let mut raw = RawMesh {
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
            triangles: vec![
                Tri {
                    a: 0,
                    b: 1,
                    c: 2,
                    material: 0,
                },
                Tri {
                    a: 3,
                    b: 4,
                    c: 5,
                    material: 0,
                },
            ],
            material_textures: vec!["shared".into()],
            components: vec![
                RawMeshComponent {
                    name: "chassis".into(),
                    frame_index: Some(0),
                    vertex_start: 0,
                    vertex_end: 3,
                    tri_start: 0,
                    tri_end: 1,
                    breakable: None,
                },
                RawMeshComponent {
                    name: "door_lf_dummy".into(),
                    frame_index: Some(1),
                    vertex_start: 3,
                    vertex_end: 6,
                    tri_start: 1,
                    tri_end: 2,
                    breakable: None,
                },
            ],
            frames: vec![identity("chassis", -1), identity("door_lf_dummy", 0)],
            ..RawMesh::default()
        };
        let components = raw.components.clone();
        let frames = raw.frames.clone();

        compact_raw_mesh_lossless(&mut raw, &BTreeSet::new());

        assert!(raw.components == components);
        assert!(raw.frames == frames);
        assert_eq!(raw.vertices.len(), 6);
        assert!(!raw_mesh_is_safe_for_normalized_rewrite(&raw, "chassis"));
    }

    #[test]
    fn normalized_writer_round_trips_multi_frame_component_hierarchy() {
        let identity = |name: &str, parent| RawMeshFrame {
            name: name.to_string(),
            parent,
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
        };
        let mut child = identity("door_lf_dummy", 0);
        child.pos.x = 10.0;
        let raw = RawMesh {
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
                V3 {
                    x: 10.0,
                    y: 0.0,
                    z: 0.0,
                },
                V3 {
                    x: 11.0,
                    y: 0.0,
                    z: 0.0,
                },
                V3 {
                    x: 10.0,
                    y: 1.0,
                    z: 0.0,
                },
            ],
            normals: vec![
                V3 {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0
                };
                6
            ],
            triangles: vec![
                Tri {
                    a: 0,
                    b: 1,
                    c: 2,
                    material: 0,
                },
                Tri {
                    a: 3,
                    b: 4,
                    c: 5,
                    material: 0,
                },
            ],
            material_textures: vec!["shared".into()],
            components: vec![
                RawMeshComponent {
                    name: "chassis".into(),
                    frame_index: Some(0),
                    vertex_start: 0,
                    vertex_end: 3,
                    tri_start: 0,
                    tri_end: 1,
                    breakable: None,
                },
                RawMeshComponent {
                    name: "door_lf_dummy".into(),
                    frame_index: Some(1),
                    vertex_start: 3,
                    vertex_end: 6,
                    tri_start: 1,
                    tri_end: 2,
                    breakable: None,
                },
            ],
            frames: vec![identity("chassis", -1), child],
            ..RawMesh::default()
        };

        assert!(raw_mesh_is_safe_for_hierarchy_rewrite(&raw, "chassis"));
        let bytes = write_normalized_dff(&raw, "model").unwrap();
        let parsed = crate::dff::import::parse_dff_mesh(&bytes);

        assert_eq!(parsed.frames.len(), 2);
        assert_eq!(parsed.frames[1].parent, 0);
        assert_eq!(parsed.components.len(), 2);
        assert_eq!(parsed.components[0].frame_index, Some(0));
        assert_eq!(parsed.components[1].frame_index, Some(1));
        assert!(
            parsed
                .vertices
                .iter()
                .any(|vertex| (vertex.x - 10.0).abs() < 0.001)
        );
    }
}
