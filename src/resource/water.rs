use super::super::*;

pub(crate) fn water_dat_path(root: &Path) -> PathBuf {
    root.join("water.dat")
}

pub(crate) fn wip_water_dat_path(root: &Path) -> PathBuf {
    wip_root_path(root).join("water.dat")
}

pub(crate) fn autosave_water_dat_path(root: &Path) -> PathBuf {
    autosave_root_path(root).join("water.dat")
}

pub(crate) fn load_water_dat_for_source(root: &Path, source: LoadSceneSource) -> Vec<WaterPlane> {
    if source == LoadSceneSource::Autosave {
        let autosave_path = autosave_water_dat_path(root);
        if autosave_path.is_file() {
            return load_water_dat(&autosave_path);
        }
    }
    let wip_path = wip_water_dat_path(root);
    if source == LoadSceneSource::Saved && wip_path.is_file() {
        return load_water_dat(&wip_path);
    }
    load_water_dat(&water_dat_path(root))
}

pub(crate) fn initial_water_selection(planes: &[WaterPlane]) -> BTreeSet<usize> {
    if planes.is_empty() {
        BTreeSet::new()
    } else {
        BTreeSet::from([0])
    }
}

pub(crate) fn default_water_corner(x: f32, y: f32, z: f32) -> WaterCorner {
    WaterCorner {
        pos: V3 { x, y, z },
        wave_x: 0.0,
        wave_y: 0.0,
        speed: 1.0,
        unknown: 0.0,
    }
}

pub(crate) fn new_water_plane(center: Vec3) -> WaterPlane {
    let half = 64.0;
    WaterPlane {
        corners: [
            default_water_corner(center.x - half, center.y - half, center.z),
            default_water_corner(center.x + half, center.y - half, center.z),
            default_water_corner(center.x - half, center.y + half, center.z),
            default_water_corner(center.x + half, center.y + half, center.z),
        ],
        kind: 1,
    }
}

pub(crate) fn water_plane_bounds(plane: &WaterPlane) -> (f32, f32, f32, f32, f32) {
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    let mut z = 0.0;
    for corner in &plane.corners {
        min_x = min_x.min(corner.pos.x);
        min_y = min_y.min(corner.pos.y);
        max_x = max_x.max(corner.pos.x);
        max_y = max_y.max(corner.pos.y);
        z += corner.pos.z;
    }
    (min_x, min_y, max_x, max_y, z / 4.0)
}

pub(crate) fn set_water_plane_rect(
    plane: &mut WaterPlane,
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
    z: f32,
) {
    plane.corners[0].pos = V3 {
        x: min_x,
        y: min_y,
        z,
    };
    plane.corners[1].pos = V3 {
        x: max_x,
        y: min_y,
        z,
    };
    plane.corners[2].pos = V3 {
        x: min_x,
        y: max_y,
        z,
    };
    plane.corners[3].pos = V3 {
        x: max_x,
        y: max_y,
        z,
    };
}

fn midpoint_water_corner(a: WaterCorner, b: WaterCorner) -> WaterCorner {
    WaterCorner {
        pos: V3 {
            x: (a.pos.x + b.pos.x) * 0.5,
            y: (a.pos.y + b.pos.y) * 0.5,
            z: (a.pos.z + b.pos.z) * 0.5,
        },
        wave_x: (a.wave_x + b.wave_x) * 0.5,
        wave_y: (a.wave_y + b.wave_y) * 0.5,
        speed: (a.speed + b.speed) * 0.5,
        unknown: (a.unknown + b.unknown) * 0.5,
    }
}

pub(crate) fn split_water_plane(
    plane: &WaterPlane,
    axis: WaterSplitAxis,
) -> (WaterPlane, WaterPlane) {
    let [south_west, south_east, north_west, north_east] = plane.corners;
    match axis {
        WaterSplitAxis::X => {
            let south_mid = midpoint_water_corner(south_west, south_east);
            let north_mid = midpoint_water_corner(north_west, north_east);
            (
                WaterPlane {
                    corners: [south_west, south_mid, north_west, north_mid],
                    kind: plane.kind,
                },
                WaterPlane {
                    corners: [south_mid, south_east, north_mid, north_east],
                    kind: plane.kind,
                },
            )
        }
        WaterSplitAxis::Y => {
            let west_mid = midpoint_water_corner(south_west, north_west);
            let east_mid = midpoint_water_corner(south_east, north_east);
            (
                WaterPlane {
                    corners: [south_west, south_east, west_mid, east_mid],
                    kind: plane.kind,
                },
                WaterPlane {
                    corners: [west_mid, east_mid, north_west, north_east],
                    kind: plane.kind,
                },
            )
        }
    }
}

fn parse_water_line(line: &str) -> Option<WaterPlane> {
    let values: Vec<f32> = line
        .split_whitespace()
        .map(|part| part.parse::<f32>().ok().filter(|value| value.is_finite()))
        .collect::<Option<_>>()?;
    if values.len() != 29 {
        return None;
    }
    let mut corners = [default_water_corner(0.0, 0.0, 0.0); 4];
    for (idx, corner) in corners.iter_mut().enumerate() {
        let base = idx * 7;
        *corner = WaterCorner {
            pos: V3 {
                x: values[base],
                y: values[base + 1],
                z: values[base + 2],
            },
            wave_x: values[base + 3],
            wave_y: values[base + 4],
            speed: values[base + 5],
            unknown: values[base + 6],
        };
    }
    Some(WaterPlane {
        corners,
        kind: values[28].round() as i32,
    })
}

pub(crate) fn load_water_dat(path: &Path) -> Vec<WaterPlane> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .map(str::trim)
        .filter(|line| {
            !line.is_empty() && !line.starts_with('#') && !line.eq_ignore_ascii_case("processed")
        })
        .filter_map(parse_water_line)
        .collect()
}

fn fmt_water_value(value: f32) -> String {
    if (value.round() - value).abs() < 0.0001 {
        format!("{:.1}", value)
    } else {
        format!("{:.3}", value)
    }
}

pub(crate) fn water_dat_text(planes: &[WaterPlane]) -> String {
    let mut out = String::from("processed\n");
    for plane in planes {
        for (idx, corner) in plane.corners.iter().enumerate() {
            if idx > 0 {
                out.push('\t');
            }
            for value in [
                corner.pos.x,
                corner.pos.y,
                corner.pos.z,
                corner.wave_x,
                corner.wave_y,
                corner.speed,
                corner.unknown,
            ] {
                out.push_str(&fmt_water_value(value));
                out.push(' ');
            }
        }
        out.push(' ');
        out.push_str(&plane.kind.to_string());
        out.push('\n');
    }
    out
}

pub(crate) fn save_water_dat(path: &Path, planes: &[WaterPlane]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("{}: {err}", parent.display()))?;
    }
    fs::write(path, water_dat_text(planes)).map_err(|err| format!("{}: {err}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_plane() -> WaterPlane {
        let mut plane = new_water_plane(vec3(10.0, 20.0, 30.0));
        for (idx, corner) in plane.corners.iter_mut().enumerate() {
            corner.wave_x = idx as f32;
            corner.wave_y = idx as f32 * 2.0;
            corner.speed = idx as f32 + 1.0;
            corner.unknown = idx as f32 * 3.0;
        }
        plane.kind = 7;
        plane
    }

    #[test]
    fn split_x_preserves_bounds_and_interpolates_seam() {
        let plane = test_plane();
        let (west, east) = split_water_plane(&plane, WaterSplitAxis::X);
        let original = water_plane_bounds(&plane);
        let west_bounds = water_plane_bounds(&west);
        let east_bounds = water_plane_bounds(&east);

        assert_eq!(west.kind, plane.kind);
        assert_eq!(east.kind, plane.kind);
        assert_eq!(west_bounds.0, original.0);
        assert_eq!(east_bounds.2, original.2);
        assert_eq!(west_bounds.2, east_bounds.0);
        assert!(west.corners[1] == east.corners[0]);
        assert!(west.corners[3] == east.corners[2]);
        assert_eq!(west.corners[1].wave_x, 0.5);
        assert_eq!(west.corners[3].wave_x, 2.5);
    }

    #[test]
    fn split_y_preserves_bounds_and_interpolates_seam() {
        let plane = test_plane();
        let (south, north) = split_water_plane(&plane, WaterSplitAxis::Y);
        let original = water_plane_bounds(&plane);
        let south_bounds = water_plane_bounds(&south);
        let north_bounds = water_plane_bounds(&north);

        assert_eq!(south.kind, plane.kind);
        assert_eq!(north.kind, plane.kind);
        assert_eq!(south_bounds.1, original.1);
        assert_eq!(north_bounds.3, original.3);
        assert_eq!(south_bounds.3, north_bounds.1);
        assert!(south.corners[2] == north.corners[0]);
        assert!(south.corners[3] == north.corners[1]);
        assert_eq!(south.corners[2].wave_x, 1.0);
        assert_eq!(south.corners[3].wave_x, 2.0);
    }

    #[test]
    fn water_parser_rejects_non_finite_values() {
        let valid = (0..29)
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(parse_water_line(&valid).is_some());

        for invalid in ["NaN", "inf", "-inf"] {
            let mut values = (0..29).map(|value| value.to_string()).collect::<Vec<_>>();
            values[7] = invalid.to_string();
            assert!(parse_water_line(&values.join(" ")).is_none());
        }
    }
}
