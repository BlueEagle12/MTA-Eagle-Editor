use super::super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ColLoadIssueSeverity {
    Error,
    Warning,
}

#[derive(Clone)]
pub(crate) struct ColLoadIssue {
    pub(crate) severity: ColLoadIssueSeverity,
    pub(crate) model: String,
    pub(crate) message: String,
}

impl ColLoadIssue {
    pub(crate) fn label(&self, entry_name: &str) -> String {
        let severity = match self.severity {
            ColLoadIssueSeverity::Error => "ERROR",
            ColLoadIssueSeverity::Warning => "WARN",
        };
        if self.model.trim().is_empty() {
            format!("{severity} {entry_name}: {}", self.message)
        } else {
            format!("{severity} {entry_name}:{}: {}", self.model, self.message)
        }
    }
}

#[derive(Default)]
struct ColModelHeader {
    magic: [u8; 4],
    model_start: usize,
    model_end: usize,
    header_len: usize,
    name: String,
    sphere_count: usize,
    box_count: usize,
    face_count: usize,
    line_count: usize,
    flags: u32,
    spheres_offset: u32,
    boxes_offset: u32,
    lines_offset: u32,
    vertices_offset: u32,
    faces_offset: u32,
    planes_offset: u32,
    shadow_face_count: usize,
    shadow_vertices_offset: u32,
    shadow_faces_offset: u32,
}

fn push_issue(
    issues: &mut Vec<ColLoadIssue>,
    severity: ColLoadIssueSeverity,
    model: &str,
    message: impl Into<String>,
) {
    issues.push(ColLoadIssue {
        severity,
        model: model.to_string(),
        message: message.into(),
    });
}

fn col_magic(bytes: &[u8], start: usize) -> Option<[u8; 4]> {
    Some(bytes.get(start..start + 4)?.try_into().ok()?)
}

fn col_version_header_len(magic: &[u8; 4]) -> Option<usize> {
    match magic {
        b"COL2" => Some(108),
        b"COL3" => Some(120),
        b"COL4" => Some(124),
        _ => None,
    }
}

fn col_model_name(bytes: &[u8], start: usize) -> String {
    let raw = bytes.get(start + 8..start + 30).unwrap_or(&[]);
    let end = raw
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(raw.len());
    String::from_utf8_lossy(&raw[..end]).trim().to_string()
}

fn parse_header(bytes: &[u8], model_start: usize) -> Result<ColModelHeader, String> {
    let magic =
        col_magic(bytes, model_start).ok_or_else(|| "model header is truncated".to_string())?;
    let header_len = col_version_header_len(&magic)
        .ok_or_else(|| "not a SA COL2/COL3/COL4 model".to_string())?;
    if model_start + 8 > bytes.len() {
        return Err("model size field is truncated".to_string());
    }
    let file_size = rd32(bytes, model_start + 4) as usize;
    let model_end = model_start
        .checked_add(file_size)
        .and_then(|value| value.checked_add(8))
        .ok_or_else(|| "model size overflows".to_string())?;
    if model_end <= model_start + 8 {
        return Err("model declares an empty or negative size".to_string());
    }
    if model_end > bytes.len() {
        return Err(format!(
            "model declares {} bytes but only {} are available",
            model_end - model_start,
            bytes.len().saturating_sub(model_start)
        ));
    }
    if model_start + header_len > model_end {
        return Err(format!(
            "model is too short for {} header ({} < {header_len})",
            String::from_utf8_lossy(&magic),
            model_end - model_start
        ));
    }
    let header = model_start + 72;
    let mut out = ColModelHeader {
        magic,
        model_start,
        model_end,
        header_len,
        name: col_model_name(bytes, model_start),
        sphere_count: rd16(bytes, header) as usize,
        box_count: rd16(bytes, header + 2) as usize,
        face_count: rd16(bytes, header + 4) as usize,
        line_count: bytes.get(header + 6).copied().unwrap_or(0) as usize,
        flags: rd32(bytes, header + 8),
        spheres_offset: rd32(bytes, header + 12),
        boxes_offset: rd32(bytes, header + 16),
        lines_offset: rd32(bytes, header + 20),
        vertices_offset: rd32(bytes, header + 24),
        faces_offset: rd32(bytes, header + 28),
        planes_offset: rd32(bytes, header + 32),
        ..Default::default()
    };
    if matches!(&out.magic, b"COL3" | b"COL4") {
        out.shadow_face_count = rd32(bytes, header + 36) as usize;
        out.shadow_vertices_offset = rd32(bytes, header + 40);
        out.shadow_faces_offset = rd32(bytes, header + 44);
    }
    Ok(out)
}

fn section_abs(header: &ColModelHeader, offset: u32) -> Option<usize> {
    (offset != 0).then_some(header.model_start + offset as usize + 4)
}

fn validate_section(
    issues: &mut Vec<ColLoadIssue>,
    h: &ColModelHeader,
    name: &str,
    count: usize,
    offset: u32,
    stride: usize,
) -> Option<usize> {
    if count == 0 {
        if offset != 0 {
            push_issue(
                issues,
                ColLoadIssueSeverity::Warning,
                &h.name,
                format!("{name} offset is set even though {name} count is zero"),
            );
        }
        return None;
    }
    let Some(abs) = section_abs(h, offset) else {
        push_issue(
            issues,
            ColLoadIssueSeverity::Error,
            &h.name,
            format!("{name} count is {count} but offset is zero"),
        );
        return None;
    };
    if abs < h.model_start + h.header_len {
        push_issue(
            issues,
            ColLoadIssueSeverity::Error,
            &h.name,
            format!("{name} offset points inside the header"),
        );
        return None;
    }
    let Some(end) = abs.checked_add(count.saturating_mul(stride)) else {
        push_issue(
            issues,
            ColLoadIssueSeverity::Error,
            &h.name,
            format!("{name} range overflows"),
        );
        return None;
    };
    if end > h.model_end {
        push_issue(
            issues,
            ColLoadIssueSeverity::Error,
            &h.name,
            format!("{name} range extends past the COL model"),
        );
        return None;
    }
    Some(abs)
}

fn read_vertex(bytes: &[u8], abs: usize, idx: usize) -> Option<V3> {
    let o = abs + idx * 6;
    Some(V3 {
        x: rdi16(bytes, o) as f32 / 128.0,
        y: rdi16(bytes, o + 2) as f32 / 128.0,
        z: rdi16(bytes, o + 4) as f32 / 128.0,
    })
}

fn validate_bounds(bytes: &[u8], issues: &mut Vec<ColLoadIssue>, h: &ColModelHeader) {
    if h.model_start + 72 > bytes.len() {
        return;
    }
    let min = vec3(
        rdf32(bytes, h.model_start + 32),
        rdf32(bytes, h.model_start + 36),
        rdf32(bytes, h.model_start + 40),
    );
    let max = vec3(
        rdf32(bytes, h.model_start + 44),
        rdf32(bytes, h.model_start + 48),
        rdf32(bytes, h.model_start + 52),
    );
    let center = vec3(
        rdf32(bytes, h.model_start + 56),
        rdf32(bytes, h.model_start + 60),
        rdf32(bytes, h.model_start + 64),
    );
    let radius = rdf32(bytes, h.model_start + 68);
    let valid_numbers = [
        min.x, min.y, min.z, max.x, max.y, max.z, center.x, center.y, center.z, radius,
    ]
    .into_iter()
    .all(f32::is_finite);
    if !valid_numbers {
        push_issue(
            issues,
            ColLoadIssueSeverity::Error,
            &h.name,
            "bounds contain NaN or infinity",
        );
        return;
    }
    if min.x > max.x || min.y > max.y || min.z > max.z || radius < 0.0 {
        push_issue(
            issues,
            ColLoadIssueSeverity::Error,
            &h.name,
            "bounds header is invalid; game broad-phase may skip or over-hit this model",
        );
    }
    let expected_center = (min + max) * 0.5;
    if (center - expected_center).length() > 0.25 {
        push_issue(
            issues,
            ColLoadIssueSeverity::Warning,
            &h.name,
            "bounds center does not match min/max; editor recomputes this but game does not",
        );
    }
}

fn validate_col_model_for_game_load(
    bytes: &[u8],
    issues: &mut Vec<ColLoadIssue>,
    h: &ColModelHeader,
) {
    if h.name.is_empty() {
        push_issue(
            issues,
            ColLoadIssueSeverity::Error,
            &h.name,
            "internal COL model name is empty, so the game cannot bind it to a model",
        );
    }
    validate_bounds(bytes, issues, h);
    let allowed_flags = 1 | 2 | 8 | 16;
    if h.flags & !allowed_flags != 0 {
        push_issue(
            issues,
            ColLoadIssueSeverity::Warning,
            &h.name,
            format!("unknown COL flags set: 0x{:X}", h.flags & !allowed_flags),
        );
    }
    let has_shapes =
        h.sphere_count != 0 || h.box_count != 0 || h.face_count != 0 || h.line_count != 0;
    if has_shapes && h.flags & 2 == 0 {
        push_issue(
            issues,
            ColLoadIssueSeverity::Error,
            &h.name,
            "collision shapes exist but the not-empty flag is clear; game marks the model collisionless",
        );
    }
    if !has_shapes && h.flags & 2 != 0 {
        push_issue(
            issues,
            ColLoadIssueSeverity::Warning,
            &h.name,
            "not-empty flag is set but the model has no collision primitives",
        );
    }
    if h.flags & 1 != 0 || h.line_count != 0 {
        push_issue(
            issues,
            ColLoadIssueSeverity::Warning,
            &h.name,
            "line/cone/disc collision data is present; the editor does not preserve or preview it",
        );
    }

    let _ = validate_section(issues, h, "spheres", h.sphere_count, h.spheres_offset, 20);
    let _ = validate_section(issues, h, "boxes", h.box_count, h.boxes_offset, 28);
    let _ = validate_section(issues, h, "lines", h.line_count, h.lines_offset, 16);
    let faces_abs = validate_section(issues, h, "faces", h.face_count, h.faces_offset, 8);
    let vertices_abs = if h.face_count == 0 {
        None
    } else {
        validate_section(issues, h, "vertices", 1, h.vertices_offset, 6)
    };

    if h.planes_offset != 0 {
        push_issue(
            issues,
            ColLoadIssueSeverity::Warning,
            &h.name,
            "triangle plane offset is set in file; game recalculates planes and the editor does not preserve this section",
        );
    }

    if h.flags & 8 != 0 {
        if h.face_count == 0 || h.faces_offset == 0 {
            push_issue(
                issues,
                ColLoadIssueSeverity::Error,
                &h.name,
                "face-group flag is set without triangle faces",
            );
        } else {
            let count_pos = h.model_start + h.faces_offset as usize;
            if count_pos + 4 > h.model_end {
                push_issue(
                    issues,
                    ColLoadIssueSeverity::Error,
                    &h.name,
                    "face-group count lies outside the COL model",
                );
            } else {
                let group_count = rd32(bytes, count_pos) as usize;
                let group_bytes = group_count.saturating_mul(28);
                let groups_start = count_pos.saturating_sub(group_bytes);
                if group_count == 0 {
                    push_issue(
                        issues,
                        ColLoadIssueSeverity::Error,
                        &h.name,
                        "face-group flag is set but group count is zero",
                    );
                } else if group_count > h.face_count || groups_start < h.model_start + h.header_len
                {
                    push_issue(
                        issues,
                        ColLoadIssueSeverity::Error,
                        &h.name,
                        "face-group section is not valid before the triangle data",
                    );
                } else {
                    for group in 0..group_count {
                        let o = groups_start + group * 28;
                        let first = rd16(bytes, o + 24) as usize;
                        let last = rd16(bytes, o + 26) as usize;
                        if first > last || last >= h.face_count {
                            push_issue(
                                issues,
                                ColLoadIssueSeverity::Error,
                                &h.name,
                                format!(
                                    "face group {group} references faces outside the triangle array"
                                ),
                            );
                            break;
                        }
                    }
                }
            }
        }
    }

    if let (Some(vertices_abs), Some(faces_abs)) = (vertices_abs, faces_abs) {
        let mut highest = 0usize;
        let mut degenerate = 0usize;
        for face in 0..h.face_count {
            let o = faces_abs + face * 8;
            let a = rd16(bytes, o) as usize;
            let b = rd16(bytes, o + 2) as usize;
            let c = rd16(bytes, o + 4) as usize;
            highest = highest.max(a).max(b).max(c);
            if a == b || b == c || c == a {
                degenerate += 1;
            }
        }
        let vertex_count = highest + 1;
        if vertices_abs + vertex_count * 6 > h.model_end {
            push_issue(
                issues,
                ColLoadIssueSeverity::Error,
                &h.name,
                "triangle indices require vertices beyond the COL model body",
            );
        }
        if vertices_abs >= faces_abs {
            push_issue(
                issues,
                ColLoadIssueSeverity::Error,
                &h.name,
                "vertex section starts at or after triangle section",
            );
        }
        if degenerate != 0 {
            push_issue(
                issues,
                ColLoadIssueSeverity::Warning,
                &h.name,
                format!("{degenerate} degenerate triangle(s) will not collide correctly"),
            );
        }
        let available_vertices = faces_abs.saturating_sub(vertices_abs) / 6;
        let check_count = vertex_count.min(available_vertices);
        let mut non_finite = false;
        for idx in 0..check_count {
            if let Some(v) = read_vertex(bytes, vertices_abs, idx) {
                if !v.x.is_finite() || !v.y.is_finite() || !v.z.is_finite() {
                    non_finite = true;
                    break;
                }
            }
        }
        if non_finite {
            push_issue(
                issues,
                ColLoadIssueSeverity::Error,
                &h.name,
                "vertex section contains non-finite coordinates",
            );
        }
    }

    if h.flags & 16 != 0 {
        if !matches!(&h.magic, b"COL3" | b"COL4") {
            push_issue(
                issues,
                ColLoadIssueSeverity::Error,
                &h.name,
                "shadow mesh flag is set on a non-COL3/COL4 model",
            );
        }
        if h.shadow_face_count == 0 || h.shadow_vertices_offset == 0 || h.shadow_faces_offset == 0 {
            push_issue(
                issues,
                ColLoadIssueSeverity::Error,
                &h.name,
                "shadow mesh flag is set but shadow counts/offsets are incomplete",
            );
        } else {
            let _ = validate_section(
                issues,
                h,
                "shadow faces",
                h.shadow_face_count,
                h.shadow_faces_offset,
                8,
            );
            let _ = validate_section(issues, h, "shadow vertices", 1, h.shadow_vertices_offset, 6);
        }
    } else if h.shadow_face_count != 0
        || h.shadow_vertices_offset != 0
        || h.shadow_faces_offset != 0
    {
        push_issue(
            issues,
            ColLoadIssueSeverity::Warning,
            &h.name,
            "shadow mesh fields are populated but the shadow flag is clear",
        );
    }
}

pub(crate) fn validate_col_for_game_load(entry_name: &str, bytes: &[u8]) -> Vec<ColLoadIssue> {
    let mut issues = Vec::new();
    let mut model_start = 0usize;
    let mut model_count = 0usize;
    while model_start + 8 <= bytes.len() {
        let Some(magic) = col_magic(bytes, model_start) else {
            break;
        };
        if col_version_header_len(&magic).is_none() {
            break;
        }
        match parse_header(bytes, model_start) {
            Ok(header) => {
                validate_col_model_for_game_load(bytes, &mut issues, &header);
                model_start = header.model_end;
                model_count += 1;
            }
            Err(err) => {
                push_issue(
                    &mut issues,
                    ColLoadIssueSeverity::Error,
                    "",
                    format!("{} at model offset {model_start}", err),
                );
                return issues;
            }
        }
    }
    if model_count == 0 {
        push_issue(
            &mut issues,
            ColLoadIssueSeverity::Error,
            "",
            format!("{entry_name} does not contain a loadable SA COL2/COL3/COL4 model"),
        );
    } else if bytes
        .get(model_start..)
        .is_some_and(|tail| tail.iter().any(|value| *value != 0))
    {
        push_issue(
            &mut issues,
            ColLoadIssueSeverity::Warning,
            "",
            "non-zero trailing bytes after last COL model",
        );
    }
    issues
}

pub(crate) fn col_validation_has_errors(issues: &[ColLoadIssue]) -> bool {
    issues
        .iter()
        .any(|issue| issue.severity == ColLoadIssueSeverity::Error)
}

/// Clears section pointers that cannot be used because their corresponding
/// element count is zero. Some exporters point empty sections at the end of
/// the model; the game generally ignores those pointers, but canonical SA COL
/// headers use zero and the validator reports the inconsistency.
pub(crate) fn repair_zero_count_col_offsets(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut out = bytes.to_vec();
    let mut changed = false;
    let mut model_start = 0usize;
    while model_start + 8 <= bytes.len() {
        let header = parse_header(bytes, model_start).ok()?;
        let common = model_start + 72;
        for (count, offset, field) in [
            (header.sphere_count, header.spheres_offset, common + 12),
            (header.box_count, header.boxes_offset, common + 16),
            (header.line_count, header.lines_offset, common + 20),
            (header.face_count, header.faces_offset, common + 28),
        ] {
            if count == 0 && offset != 0 {
                out[field..field + 4].fill(0);
                changed = true;
            }
        }
        if header.face_count == 0 && header.vertices_offset != 0 {
            out[common + 24..common + 28].fill(0);
            changed = true;
        }
        if matches!(&header.magic, b"COL3" | b"COL4") && header.shadow_face_count == 0 {
            if header.shadow_vertices_offset != 0 {
                out[common + 40..common + 44].fill(0);
                changed = true;
            }
            if header.shadow_faces_offset != 0 {
                out[common + 44..common + 48].fill(0);
                changed = true;
            }
        }
        model_start = header.model_end;
    }
    changed.then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_col3() -> Vec<u8> {
        let mut bytes = vec![0u8; 120];
        bytes[0..4].copy_from_slice(b"COL3");
        bytes[8..12].copy_from_slice(b"test");
        for (offset, value) in [
            (32usize, -1.0f32),
            (36, -1.0),
            (40, -1.0),
            (44, 1.0),
            (48, 1.0),
            (52, 1.0),
            (56, 0.0),
            (60, 0.0),
            (64, 0.0),
            (68, 1.7321),
        ] {
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        bytes[76..78].copy_from_slice(&1u16.to_le_bytes());
        bytes[80..84].copy_from_slice(&2u32.to_le_bytes());
        bytes[96..100].copy_from_slice(&116u32.to_le_bytes());
        bytes[100..104].copy_from_slice(&134u32.to_le_bytes());
        for value in [0i16, 0, 0, 128, 0, 0, 0, 128, 0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.push(0);
        bytes.push(0);
        let size = (bytes.len() - 8) as u32;
        bytes[4..8].copy_from_slice(&size.to_le_bytes());
        bytes
    }

    #[test]
    fn game_load_validation_accepts_plain_col3() {
        let issues = validate_col_for_game_load("test.col", &test_col3());
        assert!(
            !col_validation_has_errors(&issues),
            "{}",
            issues
                .iter()
                .map(|issue| issue.label("test.col"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    #[test]
    fn zero_count_offset_repair_canonicalizes_empty_col3_header() {
        let mut bytes = test_col3();
        bytes.truncate(120);
        bytes[4..8].copy_from_slice(&112u32.to_le_bytes());
        bytes[76..78].fill(0);
        bytes[80..84].fill(0);
        bytes[96..100].copy_from_slice(&116u32.to_le_bytes());
        bytes[100..104].copy_from_slice(&116u32.to_le_bytes());

        let before = validate_col_for_game_load("empty.col", &bytes);
        assert!(before.iter().any(|issue| {
            issue
                .message
                .contains("faces offset is set even though faces count is zero")
        }));

        let repaired = repair_zero_count_col_offsets(&bytes).unwrap();

        assert_eq!(rd32(&repaired, 96), 0);
        assert_eq!(rd32(&repaired, 100), 0);
        assert!(validate_col_for_game_load("empty.col", &repaired).is_empty());
        assert!(repair_zero_count_col_offsets(&repaired).is_none());
    }

    #[test]
    fn game_load_validation_rejects_false_face_group_flag() {
        let mut bytes = test_col3();
        bytes[80..84].copy_from_slice(&10u32.to_le_bytes());

        let issues = validate_col_for_game_load("test.col", &bytes);

        assert!(col_validation_has_errors(&issues));
        assert!(
            issues
                .iter()
                .any(|issue| issue.message.contains("face-group section"))
        );
    }

    #[test]
    fn game_load_validation_rejects_coll_v1_bounds_order_in_col3() {
        let mut bytes = test_col3();
        bytes[32..36].copy_from_slice(&1.7321f32.to_le_bytes());
        bytes[36..40].copy_from_slice(&0.0f32.to_le_bytes());
        bytes[40..44].copy_from_slice(&0.0f32.to_le_bytes());
        bytes[44..48].copy_from_slice(&0.0f32.to_le_bytes());
        bytes[48..52].copy_from_slice(&(-1.0f32).to_le_bytes());
        bytes[52..56].copy_from_slice(&(-1.0f32).to_le_bytes());
        bytes[56..60].copy_from_slice(&(-1.0f32).to_le_bytes());
        bytes[60..64].copy_from_slice(&1.0f32.to_le_bytes());
        bytes[64..68].copy_from_slice(&1.0f32.to_le_bytes());
        bytes[68..72].copy_from_slice(&1.0f32.to_le_bytes());

        let issues = validate_col_for_game_load("test.col", &bytes);

        assert!(col_validation_has_errors(&issues));
        assert!(
            issues
                .iter()
                .any(|issue| issue.message.contains("bounds header"))
        );
    }
}
