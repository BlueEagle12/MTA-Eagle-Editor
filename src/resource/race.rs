use super::super::*;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

// ============================================================================
// Per-map race track storage.
//
// Each map keeps its own race data under `<map>/tracks/`:
//   tracks/tracks.xml          - track definitions (see schema below)
//   tracks/<id>.png            - generated cropped preview image
//   tracks/<id>_overlay.png    - generated transparent outline overlay
//   tracks/radar.png           - (optional) radar backdrop used to generate
//                                the previews
//
// The XML stays backwards compatible with the existing M_Race loader: it reads
// <track>/<start>/<find>/<subtracks>/<subtrack>/<checkpoints>/<checkpoint>.
// The extra <overlay> and <path> children (dense preview + radar points) and
// the `overlayImage` attribute are simply ignored by the old loader.
// ============================================================================

pub(crate) fn race_dir(root: &Path) -> PathBuf {
    root.join("tracks")
}

pub(crate) fn race_tracks_xml_path(root: &Path) -> PathBuf {
    race_dir(root).join("tracks.xml")
}

/// Resolves the radar backdrop image for preview generation.
pub(crate) fn resolve_radar_path(root: &Path, configured: &str) -> Option<PathBuf> {
    if !configured.trim().is_empty() {
        let p = Path::new(configured);
        let full = if p.is_absolute() {
            p.to_path_buf()
        } else {
            root.join(p)
        };
        if full.is_file() {
            return Some(full);
        }
    }
    for candidate in [
        race_dir(root).join("radar.png"),
        root.join("radar.png"),
        race_dir(root).join("radar_old.png"),
    ] {
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

// ----------------------------------------------------------------------------
// Serialization
// ----------------------------------------------------------------------------

fn fmt_coord(v: f32) -> String {
    // Trim to 2 decimals, matching the existing tracks.xml style.
    let s = format!("{:.2}", v);
    s
}

/// Turns a track/subtrack name into a filesystem- and MTA-safe path segment.
/// MUST stay byte-for-byte identical to the sanitizer in the M_Race Lua loader
/// (server.lua / track_c.lua) so the previews it looks up line up with the files
/// the editor writes: keep ASCII alphanumerics and '-', map everything else to
/// '_'. Empty results fall back to the provided default.
pub(crate) fn sanitize_race_name(name: &str, fallback: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() {
        fallback.to_string()
    } else {
        cleaned
    }
}

/// Preview + overlay PNG paths (relative to the map root) for one subtrack,
/// following the M_Race naming convention:
///   * a single subtrack   -> tracks/<trackname>.png
///   * multiple subtracks   -> tracks/<trackname>/<subtrackname>.png
pub(crate) fn subtrack_preview_rel(
    track_name: &str,
    sub_name: &str,
    sub_index: usize,
    sub_count: usize,
) -> (String, String) {
    let t = sanitize_race_name(track_name, "track");
    if sub_count > 1 {
        let s = sanitize_race_name(sub_name, &format!("sub{}", sub_index + 1));
        (
            format!("tracks/{}/{}.png", t, s),
            format!("tracks/{}/{}_overlay.png", t, s),
        )
    } else {
        (
            format!("tracks/{}.png", t),
            format!("tracks/{}_overlay.png", t),
        )
    }
}

pub(crate) fn race_tracks_xml_text(
    tracks: &[RaceTrack],
    radar: &str,
    world_size: f32,
    center_x: f32,
    center_y: f32,
) -> String {
    let mut out = String::new();
    out.push_str("<tracks>\n");
    // Editor-only settings (radar backdrop + world mapping). Ignored by the
    // gameplay loader, but let us remember the configured radar per map.
    out.push_str(&format!(
        "    <settings radar=\"{}\" worldSize=\"{}\" centerX=\"{}\" centerY=\"{}\"/>\n",
        xml_escape(radar),
        fmt_coord(world_size),
        fmt_coord(center_x),
        fmt_coord(center_y),
    ));
    for t in tracks {
        out.push_str(&format!(
            "    <track id=\"{}\" name=\"{}\" laps=\"{}\" radius=\"{}\">\n",
            xml_escape(&t.id),
            xml_escape(&t.name),
            t.laps,
            fmt_coord(t.radius),
        ));
        out.push_str(&format!(
            "        <start x=\"{}\" y=\"{}\" z=\"{}\"/>\n",
            fmt_coord(t.start.x),
            fmt_coord(t.start.y),
            fmt_coord(t.start.z),
        ));
        let find = if t.find == V3::default() {
            t.start
        } else {
            t.find
        };
        out.push_str(&format!(
            "        <find x=\"{}\" y=\"{}\" z=\"{}\"/>\n",
            fmt_coord(find.x),
            fmt_coord(find.y),
            fmt_coord(find.z),
        ));
        out.push_str("        <subtracks>\n");
        let subs = t.effective_subtracks();
        let sub_count = subs.len();
        for (si, sub) in subs.iter().enumerate() {
            // Preview paths always follow the naming convention so they match
            // what the M_Race pack loader looks for.
            let (image_rel, overlay_rel) = subtrack_preview_rel(&t.name, &sub.name, si, sub_count);
            out.push_str(&format!(
                "            <subtrack id=\"{}\" name=\"{}\" image=\"{}\" overlayImage=\"{}\">\n",
                xml_escape(&sub.id),
                xml_escape(&sub.name),
                xml_escape(&image_rel),
                xml_escape(&overlay_rel),
            ));
            out.push_str("                <checkpoints>\n");
            for cp in &sub.checkpoints {
                out.push_str(&format!(
                    "                    <checkpoint x=\"{}\" y=\"{}\" z=\"{}\" r=\"{}\"/>\n",
                    fmt_coord(cp.pos.x),
                    fmt_coord(cp.pos.y),
                    fmt_coord(cp.pos.z),
                    fmt_coord(cp.r),
                ));
            }
            out.push_str("                </checkpoints>\n");
            // Dense preview / radar outline points (ignored by the gameplay loader).
            out.push_str("                <overlay>\n");
            for p in &sub.overlay {
                out.push_str(&format!(
                    "                    <point x=\"{}\" y=\"{}\" z=\"{}\"/>\n",
                    fmt_coord(p.x),
                    fmt_coord(p.y),
                    fmt_coord(p.z),
                ));
            }
            out.push_str("                </overlay>\n");
            // Full driving path reference.
            out.push_str("                <path>\n");
            for p in &sub.path {
                out.push_str(&format!(
                    "                    <point x=\"{}\" y=\"{}\" z=\"{}\"/>\n",
                    fmt_coord(p.x),
                    fmt_coord(p.y),
                    fmt_coord(p.z),
                ));
            }
            out.push_str("                </path>\n");
            out.push_str("            </subtrack>\n");
        }
        out.push_str("        </subtracks>\n");
        out.push_str("    </track>\n");
    }
    out.push_str("</tracks>\n");
    out
}

pub(crate) fn save_race_tracks(
    root: &Path,
    tracks: &[RaceTrack],
    radar: &str,
    world_size: f32,
    center_x: f32,
    center_y: f32,
) -> Result<(), String> {
    let dir = race_dir(root);
    fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = race_tracks_xml_path(root);
    fs::write(
        &path,
        race_tracks_xml_text(tracks, radar, world_size, center_x, center_y),
    )
    .map_err(|e| format!("{}: {e}", path.display()))
}

/// Editor-only race settings persisted in tracks.xml (`<settings>` element).
pub(crate) struct RaceSettings {
    pub radar: String,
    pub world_size: f32,
    pub center_x: f32,
    pub center_y: f32,
}

/// Reads the `<settings>` element from a map's tracks.xml, if present.
pub(crate) fn load_race_settings(root: &Path) -> Option<RaceSettings> {
    let path = race_tracks_xml_path(root);
    let text = fs::read_to_string(&path).ok()?;
    let tag = Regex::new(r"<settings\b[^>]*/?>")
        .ok()?
        .find(&text)?
        .as_str()
        .to_string();
    Some(RaceSettings {
        radar: attr_str(&tag, "radar").unwrap_or_default(),
        world_size: attr_f32(&tag, "worldSize").unwrap_or(6000.0),
        center_x: attr_f32(&tag, "centerX").unwrap_or(0.0),
        center_y: attr_f32(&tag, "centerY").unwrap_or(0.0),
    })
}

/// Ensures the map's meta.xml references the per-map track files so the race
/// system can locate and load them during gameplay.
pub(crate) fn patch_map_meta_for_tracks(root: &Path) -> Result<bool, String> {
    let meta_path = root.join("meta.xml");
    let Ok(text) = fs::read_to_string(&meta_path) else {
        // No meta.xml at the map root — nothing to patch.
        return Ok(false);
    };
    let mut text = text;
    let mut changed = false;

    // 1) Tag the map's <info> so the M_Race pack loader knows this resource ships
    //    tracks. Add trackpack="true" if <info> exists but lacks it.
    if !text.contains("trackpack") {
        if let Some(info_re) = Regex::new(r"<info\b[^>]*>").ok() {
            if let Some(m) = info_re.find(&text) {
                let tag = m.as_str();
                // Insert trackpack="true" right after `<info`.
                let new_tag = tag.replacen("<info", "<info trackpack=\"true\"", 1);
                let (s, e) = (m.start(), m.end());
                let mut nt = String::with_capacity(text.len() + 20);
                nt.push_str(&text[..s]);
                nt.push_str(&new_tag);
                nt.push_str(&text[e..]);
                text = nt;
                changed = true;
            }
        }
    }

    // 2) Reference the per-map track files so the client can download them.
    let needs_xml = !text.contains("tracks/tracks.xml");
    let needs_png = !text.contains("tracks/*.png");
    let needs_png_nested = !text.contains("tracks/**/*.png");
    if needs_xml || needs_png || needs_png_nested {
        let Some(close_idx) = text.rfind("</meta>") else {
            return Err("meta.xml has no </meta> tag".to_string());
        };
        let mut insert = String::new();
        if needs_xml {
            insert.push_str("    <file src=\"tracks/tracks.xml\" type=\"client\" />\n");
        }
        if needs_png {
            insert.push_str("    <file src=\"tracks/*.png\" type=\"client\" />\n");
        }
        if needs_png_nested {
            insert.push_str("    <file src=\"tracks/**/*.png\" type=\"client\" />\n");
        }
        let mut new_text = String::with_capacity(text.len() + insert.len());
        new_text.push_str(&text[..close_idx]);
        new_text.push_str(&insert);
        new_text.push_str(&text[close_idx..]);
        text = new_text;
        changed = true;
    }

    if changed {
        fs::write(&meta_path, &text).map_err(|e| format!("{}: {e}", meta_path.display()))?;
    }
    Ok(changed)
}

// ----------------------------------------------------------------------------
// Loading (round-trip existing per-map tracks back into the editor)
// ----------------------------------------------------------------------------

fn attr_str(tag: &str, name: &str) -> Option<String> {
    let re = Regex::new(&format!(r#"{}\s*=\s*"([^"]*)""#, regex::escape(name))).ok()?;
    re.captures(tag)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

fn attr_f32(tag: &str, name: &str) -> Option<f32> {
    attr_str(tag, name).and_then(|value| {
        value
            .trim()
            .parse::<f32>()
            .ok()
            .filter(|value| value.is_finite())
    })
}

fn parse_point(tag: &str) -> Option<V3> {
    Some(V3 {
        x: attr_f32(tag, "x")?,
        y: attr_f32(tag, "y")?,
        z: attr_f32(tag, "z")?,
    })
}

fn parse_points_block(block: &str, tag: &str) -> Vec<V3> {
    let re = Regex::new(&format!(r"<{}\b[^>]*/?>", tag)).unwrap();
    re.find_iter(block)
        .filter_map(|m| parse_point(m.as_str()))
        .collect()
}

fn first_block<'a>(text: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    let start = text.find(&open)?;
    // Move to end of the opening tag.
    let after_open = start + text[start..].find('>')? + 1;
    let end = text[after_open..].find(&close)? + after_open;
    Some(&text[after_open..end])
}

/// Parses one `<subtrack>...</subtrack>` block into a RaceSubtrack.
fn parse_subtrack_block(block: &str, index: usize, track_id: &str) -> RaceSubtrack {
    let open = Regex::new(r"<subtrack\b[^>]*>")
        .unwrap()
        .find(block)
        .map(|m| m.as_str().to_string())
        .unwrap_or_default();
    let id = attr_str(&open, "id").unwrap_or_else(|| format!("sub{}", index + 1));
    let name = attr_str(&open, "name").unwrap_or_else(|| id.clone());
    let image = attr_str(&open, "image").unwrap_or_else(|| format!("tracks/{}.png", track_id));
    let overlay_image = attr_str(&open, "overlayImage")
        .unwrap_or_else(|| format!("tracks/{}_overlay.png", track_id));

    let mut checkpoints = Vec::new();
    if let Some(cp_block) = first_block(block, "checkpoints") {
        let cp_re = Regex::new(r"<checkpoint\b[^>]*/?>").unwrap();
        for m in cp_re.find_iter(cp_block) {
            if let Some(pos) = parse_point(m.as_str()) {
                let r = attr_f32(m.as_str(), "r").unwrap_or(8.0);
                checkpoints.push(RaceCheckpoint { pos, r });
            }
        }
    }
    let overlay = first_block(block, "overlay")
        .map(|b| parse_points_block(b, "point"))
        .unwrap_or_default();
    let path_pts = first_block(block, "path")
        .map(|b| parse_points_block(b, "point"))
        .unwrap_or_default();

    RaceSubtrack {
        id,
        name,
        image,
        overlay_image,
        checkpoints,
        overlay,
        path: path_pts,
    }
}

pub(crate) fn load_race_tracks(root: &Path) -> Vec<RaceTrack> {
    let path = race_tracks_xml_path(root);
    let Ok(text) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    let track_re = Regex::new(r"(?s)<track\b.*?</track>").unwrap();
    let open_re = Regex::new(r"<track\b[^>]*>").unwrap();
    let subtrack_re = Regex::new(r"(?s)<subtrack\b.*?</subtrack>").unwrap();
    let mut tracks = Vec::new();
    for block in track_re.find_iter(&text) {
        let block = block.as_str();
        let open = open_re.find(block).map(|m| m.as_str()).unwrap_or("<track>");
        let id = attr_str(open, "id").unwrap_or_else(|| format!("track{}", tracks.len() + 1));
        let name = attr_str(open, "name").unwrap_or_else(|| id.clone());
        let laps = attr_f32(open, "laps").unwrap_or(2.0) as i32;
        let radius = attr_f32(open, "radius").unwrap_or(40.0);

        let start = Regex::new(r"<start\b[^>]*/?>")
            .unwrap()
            .find(block)
            .and_then(|m| parse_point(m.as_str()))
            .unwrap_or_default();
        let find = Regex::new(r"<find\b[^>]*/?>")
            .unwrap()
            .find(block)
            .and_then(|m| parse_point(m.as_str()))
            .unwrap_or(start);

        // Parse every <subtrack> variant. Older files without full <subtrack>
        // wrappers still work: parse_subtrack_block reads whatever checkpoints/
        // overlay/path it can find in the block.
        let mut subtracks: Vec<RaceSubtrack> = subtrack_re
            .find_iter(block)
            .enumerate()
            .map(|(si, m)| parse_subtrack_block(m.as_str(), si, &id))
            .collect();
        if subtracks.is_empty() {
            // No <subtrack> element at all — treat the track body as a single
            // implicit subtrack so legacy data round-trips.
            subtracks.push(parse_subtrack_block(block, 0, &id));
        }

        let mut track = RaceTrack {
            id,
            name,
            laps,
            radius,
            start,
            find,
            subtrack_name: String::new(),
            image: String::new(),
            overlay_image: String::new(),
            checkpoints: Vec::new(),
            overlay: Vec::new(),
            path: Vec::new(),
            subtracks,
            active_subtrack: 0,
        };
        track.load_subtrack(0);
        tracks.push(track);
    }
    tracks
}

// ----------------------------------------------------------------------------
// Preview + overlay image generation
// ----------------------------------------------------------------------------

type Img = image::RgbaImage;

/// Loads the resolved radar backdrop as raw RGBA8 pixels (for the live minimap
/// overlay). Returns (width, height, rgba).
pub(crate) fn load_radar_rgba(root: &Path, configured: &str) -> Option<(u32, u32, Vec<u8>)> {
    let path = resolve_radar_path(root, configured)?;
    let bytes = fs::read(&path).ok()?;
    let img = image::load_from_memory(&bytes).ok()?.to_rgba8();
    Some((img.width(), img.height(), img.into_raw()))
}

/// Maps a world (x, y) position onto normalized radar UVs in 0..1, where u=0 is
/// west, u=1 is east, v=0 is north (top) and v=1 is south (bottom).
pub(crate) fn race_world_to_radar_uv(
    wx: f32,
    wy: f32,
    world_size: f32,
    cx: f32,
    cy: f32,
) -> (f32, f32) {
    let left = cx - world_size * 0.5;
    let top = cy + world_size * 0.5;
    let u = (wx - left) / world_size;
    let v = (top - wy) / world_size;
    (u, v)
}

fn world_to_radar_px(
    wx: f32,
    wy: f32,
    world_size: f32,
    cx: f32,
    cy: f32,
    img_w: u32,
    img_h: u32,
) -> (f32, f32) {
    let left = cx - world_size * 0.5;
    let top = cy + world_size * 0.5; // north (+y) maps to the top row
    let px = (wx - left) / world_size * img_w as f32;
    let py = (top - wy) / world_size * img_h as f32;
    (px, py)
}

fn catmull_rom(points: &[(f32, f32)], subdiv: usize, closed: bool) -> Vec<(f32, f32)> {
    let n = points.len();
    if n < 3 {
        return points.to_vec();
    }
    let get = |i: isize| -> (f32, f32) {
        if closed {
            points[i.rem_euclid(n as isize) as usize]
        } else {
            let idx = i.clamp(0, n as isize - 1) as usize;
            points[idx]
        }
    };
    let mut out = Vec::new();
    let segs = if closed { n } else { n - 1 };
    for i in 0..segs {
        let p0 = get(i as isize - 1);
        let p1 = get(i as isize);
        let p2 = get(i as isize + 1);
        let p3 = get(i as isize + 2);
        for s in 0..subdiv {
            let t = s as f32 / subdiv as f32;
            let t2 = t * t;
            let t3 = t2 * t;
            let x = 0.5
                * ((2.0 * p1.0)
                    + (-p0.0 + p2.0) * t
                    + (2.0 * p0.0 - 5.0 * p1.0 + 4.0 * p2.0 - p3.0) * t2
                    + (-p0.0 + 3.0 * p1.0 - 3.0 * p2.0 + p3.0) * t3);
            let y = 0.5
                * ((2.0 * p1.1)
                    + (-p0.1 + p2.1) * t
                    + (2.0 * p0.1 - 5.0 * p1.1 + 4.0 * p2.1 - p3.1) * t2
                    + (-p0.1 + 3.0 * p1.1 - 3.0 * p2.1 + p3.1) * t3);
            out.push((x, y));
        }
    }
    if !closed {
        out.push(points[n - 1]);
    }
    out
}

fn stamp_disc(img: &mut Img, cx: f32, cy: f32, radius: f32, color: image::Rgba<u8>) {
    let w = img.width() as i32;
    let h = img.height() as i32;
    let r = radius.max(0.5);
    let r2 = r * r;
    let min_x = ((cx - r).floor() as i32).max(0);
    let max_x = ((cx + r).ceil() as i32).min(w - 1);
    let min_y = ((cy - r).floor() as i32).max(0);
    let max_y = ((cy + r).ceil() as i32).min(h - 1);
    for py in min_y..=max_y {
        for px in min_x..=max_x {
            let dx = px as f32 + 0.5 - cx;
            let dy = py as f32 + 0.5 - cy;
            if dx * dx + dy * dy <= r2 {
                img.put_pixel(px as u32, py as u32, color);
            }
        }
    }
}

fn draw_thick_polyline(
    img: &mut Img,
    pts: &[(f32, f32)],
    thickness: f32,
    color: image::Rgba<u8>,
    closed: bool,
) {
    if pts.len() < 2 {
        if let Some(p) = pts.first() {
            stamp_disc(img, p.0, p.1, thickness * 0.5, color);
        }
        return;
    }
    let radius = thickness * 0.5;
    let mut draw_seg = |a: (f32, f32), b: (f32, f32)| {
        let dx = b.0 - a.0;
        let dy = b.1 - a.1;
        let len = (dx * dx + dy * dy).sqrt().max(0.001);
        let steps = (len / (radius * 0.5).max(1.0)).ceil() as usize + 1;
        for s in 0..=steps {
            let t = s as f32 / steps as f32;
            stamp_disc(img, a.0 + dx * t, a.1 + dy * t, radius, color);
        }
    };
    for w in pts.windows(2) {
        draw_seg(w[0], w[1]);
    }
    if closed {
        draw_seg(pts[pts.len() - 1], pts[0]);
    }
}

pub(crate) struct RacePreviewResult {
    pub image_rel: String,
    pub overlay_rel: String,
}

/// Generates the cropped preview PNG and the transparent overlay PNG for a
/// single subtrack route, writing both into `<root>/tracks/` at the given
/// relative paths. Returns the same paths (echoed) on success.
///
/// This is point-based rather than track-based so it can be driven per subtrack:
/// the caller passes the route points (checkpoints/overlay/path) plus the start
/// used for framing, and the `image_rel`/`overlay_rel` output paths (typically
/// computed via `subtrack_preview_rel`).
pub(crate) fn generate_race_preview(
    root: &Path,
    checkpoints: &[RaceCheckpoint],
    overlay_pts: &[V3],
    path_pts: &[V3],
    start: V3,
    image_rel: &str,
    overlay_rel: &str,
    radar_path_cfg: &str,
    world_size: f32,
    center_x: f32,
    center_y: f32,
) -> Result<RacePreviewResult, String> {
    // Collect the world points that define the route bounds (checkpoints first,
    // per the requirement, but include overlay/path/start so nothing is clipped).
    let mut bound_pts: Vec<(f32, f32)> = Vec::new();
    for cp in checkpoints {
        bound_pts.push((cp.pos.x, cp.pos.y));
    }
    for p in overlay_pts {
        bound_pts.push((p.x, p.y));
    }
    for p in path_pts {
        bound_pts.push((p.x, p.y));
    }
    if start != V3::default() {
        bound_pts.push((start.x, start.y));
    }
    if bound_pts.len() < 2 {
        return Err("Need at least 2 checkpoints/points to generate a preview".to_string());
    }

    let radar_path = resolve_radar_path(root, radar_path_cfg).ok_or_else(|| {
        "No radar backdrop found (place radar.png in the map's tracks/ folder or set a path)"
            .to_string()
    })?;
    let bytes = fs::read(&radar_path).map_err(|e| format!("{}: {e}", radar_path.display()))?;
    let radar = image::load_from_memory(&bytes)
        .map_err(|e| format!("decode {}: {e}", radar_path.display()))?
        .to_rgba8();
    let (rw, rh) = (radar.width(), radar.height());

    // Route bounds in world space + padding.
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for (x, y) in &bound_pts {
        min_x = min_x.min(*x);
        min_y = min_y.min(*y);
        max_x = max_x.max(*x);
        max_y = max_y.max(*y);
    }
    let range = (max_x - min_x).max(max_y - min_y).max(1.0);
    let pad = (range * 0.18).max(25.0);
    min_x -= pad;
    max_x += pad;
    min_y -= pad;
    max_y += pad;

    // Corresponding radar-pixel crop rectangle. Note +y (north) -> smaller py.
    let (px0, py_top) = world_to_radar_px(min_x, max_y, world_size, center_x, center_y, rw, rh);
    let (px1, py_bot) = world_to_radar_px(max_x, min_y, world_size, center_x, center_y, rw, rh);
    let cx0 = px0.min(px1).floor().max(0.0) as u32;
    let cy0 = py_top.min(py_bot).floor().max(0.0) as u32;
    let cx1 = px0.max(px1).ceil().min(rw as f32) as u32;
    let cy1 = py_top.max(py_bot).ceil().min(rh as f32) as u32;
    if cx1 <= cx0 + 2 || cy1 <= cy0 + 2 {
        return Err(
            "Route falls outside the radar backdrop — check world size/center settings".to_string(),
        );
    }
    let crop_w = cx1 - cx0;
    let crop_h = cy1 - cy0;
    let cropped = image::imageops::crop_imm(&radar, cx0, cy0, crop_w, crop_h).to_image();

    // Resize so the largest side is ~900px (keeps the source aspect ratio).
    let max_dim = 900.0f32;
    let scale = (max_dim / crop_w.max(crop_h) as f32).min(1.0);
    let out_w = ((crop_w as f32 * scale).round() as u32).max(2);
    let out_h = ((crop_h as f32 * scale).round() as u32).max(2);
    let mut preview = image::imageops::resize(
        &cropped,
        out_w,
        out_h,
        image::imageops::FilterType::Triangle,
    );

    // Map a world point into the final output-image pixel space.
    let to_out = |wx: f32, wy: f32| -> (f32, f32) {
        let (px, py) = world_to_radar_px(wx, wy, world_size, center_x, center_y, rw, rh);
        ((px - cx0 as f32) * scale, (py - cy0 as f32) * scale)
    };

    // Outline source: prefer the dense overlay path, fall back to checkpoints.
    let outline_world: Vec<(f32, f32)> = if overlay_pts.len() >= 2 {
        overlay_pts.iter().map(|p| (p.x, p.y)).collect()
    } else {
        checkpoints.iter().map(|c| (c.pos.x, c.pos.y)).collect()
    };
    let outline_px: Vec<(f32, f32)> = outline_world.iter().map(|(x, y)| to_out(*x, *y)).collect();
    let smooth = catmull_rom(&outline_px, 16, true);

    let thickness = (out_w.max(out_h) as f32 * 0.012).clamp(4.0, 14.0);
    let white = image::Rgba([255, 255, 255, 255]);
    let dark = image::Rgba([20, 20, 24, 255]);

    // Preview: dark halo then white core, drawn over the radar crop.
    draw_thick_polyline(&mut preview, &smooth, thickness + 4.0, dark, true);
    draw_thick_polyline(&mut preview, &smooth, thickness, white, true);

    // Overlay: full-radar-sized transparent canvas so the in-game code can draw
    // it with the exact same section coords as the radar map texture (no per-point
    // math needed at runtime). Outline is drawn in true radar-pixel space.
    let outline_full: Vec<(f32, f32)> = outline_world
        .iter()
        .map(|(x, y)| world_to_radar_px(*x, *y, world_size, center_x, center_y, rw, rh))
        .collect();
    let smooth_full = catmull_rom(&outline_full, 16, true);
    let overlay_thickness = (rw.max(rh) as f32 * 0.003).clamp(4.0, 12.0);
    let mut overlay = image::RgbaImage::from_pixel(rw, rh, image::Rgba([0, 0, 0, 0]));
    draw_thick_polyline(
        &mut overlay,
        &smooth_full,
        overlay_thickness + 4.0,
        dark,
        true,
    );
    draw_thick_polyline(&mut overlay, &smooth_full, overlay_thickness, white, true);

    // Ensure the (possibly nested, e.g. tracks/<track>/<sub>.png) output dirs.
    if let Some(parent) = root.join(image_rel).parent() {
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    if let Some(parent) = root.join(overlay_rel).parent() {
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    preview
        .save(root.join(image_rel))
        .map_err(|e| format!("save preview: {e}"))?;
    overlay
        .save(root.join(overlay_rel))
        .map_err(|e| format!("save overlay: {e}"))?;

    Ok(RacePreviewResult {
        image_rel: image_rel.to_string(),
        overlay_rel: overlay_rel.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("eagle_race_{name}_{nonce}"))
    }

    #[test]
    fn race_tracks_round_trip_overlay_and_path_points() {
        let root = temp_root("round_trip");
        fs::create_dir_all(&root).unwrap();
        let mut track = RaceTrack::new(0);
        track.start = V3 {
            x: 10.0,
            y: 20.0,
            z: 3.0,
        };
        track.checkpoints = vec![RaceCheckpoint {
            pos: V3 {
                x: 11.0,
                y: 21.0,
                z: 3.5,
            },
            r: 8.0,
        }];
        track.overlay = vec![
            V3 {
                x: 100.0,
                y: 200.0,
                z: 4.0,
            },
            V3 {
                x: 110.0,
                y: 210.0,
                z: 4.5,
            },
        ];
        track.path = vec![V3 {
            x: 300.0,
            y: 400.0,
            z: 5.0,
        }];

        save_race_tracks(
            &root,
            &[track.clone()],
            "tracks/radar.png",
            6000.0,
            1.0,
            2.0,
        )
        .unwrap();
        let loaded = load_race_tracks(&root);

        assert_eq!(loaded.len(), 1);
        assert!(loaded[0].checkpoints == track.checkpoints);
        assert!(loaded[0].overlay == track.overlay);
        assert!(loaded[0].path == track.path);

        let xml = fs::read_to_string(race_tracks_xml_path(&root)).unwrap();
        assert!(xml.contains("<overlay>"));
        assert!(xml.contains("<path>"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn race_meta_patch_exposes_tracks_xml_to_game() {
        let root = temp_root("meta");
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("meta.xml"),
            "<meta>\n    <info author=\"x\" type=\"map\" />\n</meta>\n",
        )
        .unwrap();

        assert!(patch_map_meta_for_tracks(&root).unwrap());
        let meta = fs::read_to_string(root.join("meta.xml")).unwrap();
        assert!(meta.contains(r#"trackpack="true""#));
        assert!(meta.contains(r#"src="tracks/tracks.xml""#));
        assert!(meta.contains(r#"src="tracks/*.png""#));
        assert!(meta.contains(r#"src="tracks/**/*.png""#));

        // Idempotent: a second patch makes no further changes.
        assert!(!patch_map_meta_for_tracks(&root).unwrap());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn race_multi_subtrack_round_trip() {
        let root = temp_root("multisub");
        fs::create_dir_all(&root).unwrap();
        let mut track = RaceTrack::new(0);
        track.name = "Cool Track".to_string();
        // Active (first) subtrack.
        track.subtrack_name = "Main".to_string();
        track.checkpoints = vec![RaceCheckpoint {
            pos: V3 {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            r: 8.0,
        }];
        track.commit_active_subtrack();
        // Add a second subtrack.
        let mut sub2 = RaceSubtrack::new(1);
        sub2.name = "Reverse".to_string();
        sub2.checkpoints = vec![RaceCheckpoint {
            pos: V3 {
                x: 4.0,
                y: 5.0,
                z: 6.0,
            },
            r: 9.0,
        }];
        track.subtracks.push(sub2);

        save_race_tracks(&root, &[track.clone()], "", 6000.0, 0.0, 0.0).unwrap();
        let xml = fs::read_to_string(race_tracks_xml_path(&root)).unwrap();
        // Multiple subtracks -> nested tracks/<track>/<sub>.png convention.
        assert!(xml.contains("tracks/Cool_Track/Main.png"));
        assert!(xml.contains("tracks/Cool_Track/Reverse.png"));

        let loaded = load_race_tracks(&root);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].subtracks.len(), 2);
        assert_eq!(loaded[0].subtracks[0].name, "Main");
        assert_eq!(loaded[0].subtracks[1].name, "Reverse");
        // Active scratch mirrors subtrack 0.
        assert_eq!(loaded[0].active_subtrack, 0);
        assert_eq!(loaded[0].subtrack_name, "Main");

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn single_subtrack_uses_flat_png_path() {
        let root = temp_root("singlesub");
        fs::create_dir_all(&root).unwrap();
        let mut track = RaceTrack::new(0);
        track.name = "Solo".to_string();
        track.checkpoints = vec![RaceCheckpoint {
            pos: V3 {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            r: 8.0,
        }];
        save_race_tracks(&root, &[track.clone()], "", 6000.0, 0.0, 0.0).unwrap();
        let xml = fs::read_to_string(race_tracks_xml_path(&root)).unwrap();
        assert!(xml.contains("tracks/Solo.png"));
        assert!(!xml.contains("tracks/Solo/"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn race_numeric_attributes_reject_non_finite_values() {
        assert_eq!(attr_f32(r#"<point x="12.5" />"#, "x"), Some(12.5));
        assert_eq!(attr_f32(r#"<point x="NaN" />"#, "x"), None);
        assert_eq!(attr_f32(r#"<point x="inf" />"#, "x"), None);
        assert!(parse_point(r#"<point x="1" y="-inf" z="3" />"#).is_none());
    }
}
