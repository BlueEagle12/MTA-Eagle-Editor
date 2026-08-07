use super::super::*;
use crate::resource::eagle_scene::{
    LegacySceneImport, SECTION_COLLISION_CAPSULES, SECTION_COLLISION_CUBOIDS, SECTION_LIGHTS,
    SECTION_MATERIAL_EMITTERS, SECTION_SHADOW_CASTERS, read_eagle_scene_section,
    update_eagle_scene_section,
};

pub(crate) fn rd16(b: &[u8], o: usize) -> u16 {
    if o + 2 > b.len() {
        0
    } else {
        u16::from_le_bytes([b[o], b[o + 1]])
    }
}

pub(crate) fn rdi16(b: &[u8], o: usize) -> i16 {
    if o + 2 > b.len() {
        0
    } else {
        i16::from_le_bytes([b[o], b[o + 1]])
    }
}

pub(crate) fn rd32(b: &[u8], o: usize) -> u32 {
    if o + 4 > b.len() {
        0
    } else {
        u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
    }
}

pub(crate) fn rdf32(b: &[u8], o: usize) -> f32 {
    f32::from_bits(rd32(b, o))
}

pub(crate) fn lower<S: AsRef<str>>(s: S) -> String {
    s.as_ref().to_ascii_lowercase()
}

pub(crate) fn format_bytes(bytes: usize) -> String {
    const KB: f32 = 1024.0;
    const MB: f32 = 1024.0 * 1024.0;
    if bytes as f32 >= MB {
        format!("{:.1} MB", bytes as f32 / MB)
    } else if bytes as f32 >= KB {
        format!("{:.0} KB", bytes as f32 / KB)
    } else {
        format!("{bytes} B")
    }
}

pub(crate) fn with_ext(name: &str, ext: &str) -> String {
    let mut out = name.to_string();
    if !lower(&out).ends_with(ext) {
        out.push_str(ext);
    }
    out
}

pub(crate) fn parse_attrs(input: &str, attr_re: &Regex) -> BTreeMap<String, String> {
    attr_re
        .captures_iter(input)
        .map(|c| (c[1].to_string(), xml_unescape(&c[2])))
        .collect()
}

pub(crate) fn xml_unescape(value: &str) -> String {
    // XML entities are decoded exactly once. Repeating this pass corrupts
    // literal entity text: `&amp;amp;` represents the value `&amp;`, not `&`.
    // Keep `&amp;` last so an escaped entity remains literal after this pass.
    value
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

fn parse_finite_f32_value(value: &str) -> Option<f32> {
    value
        .trim()
        .parse::<f32>()
        .ok()
        .filter(|value| value.is_finite())
}

pub(crate) fn attrf(attrs: &BTreeMap<String, String>, key: &str) -> f32 {
    attrs
        .get(key)
        .and_then(|value| parse_finite_f32_value(value))
        .unwrap_or(0.0)
}

pub(crate) fn light_kind_label(kind: LightKind) -> &'static str {
    match kind {
        LightKind::Ambient => "ambient",
        LightKind::Directional => "directional",
        LightKind::Point => "point",
        LightKind::Spot => "spot",
        LightKind::Area => "area",
    }
}

pub(crate) fn parse_light_kind(value: &str) -> LightKind {
    match lower(value).as_str() {
        "ambient" => LightKind::Ambient,
        "directional" | "sun" | "moon" => LightKind::Directional,
        "spot" => LightKind::Spot,
        "area" => LightKind::Area,
        _ => LightKind::Point,
    }
}

pub(crate) fn light_profile_label(profile: LightProfile) -> &'static str {
    match profile {
        LightProfile::Day => "day",
        LightProfile::Night => "night",
        LightProfile::Both => "both",
    }
}

pub(crate) fn parse_light_profile(value: &str) -> LightProfile {
    match lower(value).as_str() {
        "day" => LightProfile::Day,
        "night" => LightProfile::Night,
        _ => LightProfile::Both,
    }
}

fn point_light_lobe_label(lobe: PointLightLobe) -> &'static str {
    match lobe {
        PointLightLobe::Omni => "omni",
        PointLightLobe::Up => "up",
        PointLightLobe::Down => "down",
        PointLightLobe::Sides => "sides",
    }
}

fn parse_point_light_lobe(value: &str) -> PointLightLobe {
    match lower(value).as_str() {
        "up" => PointLightLobe::Up,
        "down" => PointLightLobe::Down,
        "sides" => PointLightLobe::Sides,
        _ => PointLightLobe::Omni,
    }
}

pub(crate) fn attrf_default(attrs: &BTreeMap<String, String>, key: &str, default: f32) -> f32 {
    attrs
        .get(key)
        .and_then(|value| parse_finite_f32_value(value))
        .unwrap_or(default)
}

pub(crate) fn attr_bool(attrs: &BTreeMap<String, String>, keys: &[&str], default: bool) -> bool {
    keys.iter()
        .find_map(|key| attrs.get(*key))
        .map(|value| matches!(lower(value).as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(default)
}

pub(crate) fn attr_vec3(attrs: &BTreeMap<String, String>, prefix: &str, default: V3) -> V3 {
    V3 {
        x: attrf_default(attrs, &format!("{prefix}X"), default.x),
        y: attrf_default(attrs, &format!("{prefix}Y"), default.y),
        z: attrf_default(attrs, &format!("{prefix}Z"), default.z),
    }
}

pub(crate) fn parse_vec3_text(value: &str, default: V3) -> V3 {
    let parts: Vec<_> = value
        .split(|ch| ch == ',' || ch == ' ')
        .filter(|part| !part.trim().is_empty())
        .collect();
    if parts.len() < 3 {
        return default;
    }
    let Some(x) = parse_finite_f32_value(parts[0]) else {
        return default;
    };
    let Some(y) = parse_finite_f32_value(parts[1]) else {
        return default;
    };
    let Some(z) = parse_finite_f32_value(parts[2]) else {
        return default;
    };
    V3 { x, y, z }
}

pub(crate) fn timecyc_color(values: &[f32], offset: usize) -> V3 {
    if values.len() < offset + 3 {
        return V3::default();
    }
    V3 {
        x: (values[offset] / 255.0).clamp(0.0, 1.0),
        y: (values[offset + 1] / 255.0).clamp(0.0, 1.0),
        z: (values[offset + 2] / 255.0).clamp(0.0, 1.0),
    }
}

pub(crate) fn timecyc_postfx_rgba(values: &[f32], offset: usize) -> [f32; 4] {
    if values.len() < offset + 4 {
        return [0.0; 4];
    }
    // SA stores these columns as A,R,G,B. The PC loader then doubles alpha
    // into an 8-bit field even though the shipped PC file already uses the
    // standard 0..255 alpha range. Preserve the resulting byte wrap exactly.
    let doubled_alpha_byte = ((values[offset] * 2.0) as i32).rem_euclid(256) as f32;
    [
        (values[offset + 1] / 255.0).clamp(0.0, 1.0),
        (values[offset + 2] / 255.0).clamp(0.0, 1.0),
        (values[offset + 3] / 255.0).clamp(0.0, 1.0),
        doubled_alpha_byte / 255.0,
    ]
}

pub(crate) fn timecyc_sample_from_values(label: String, values: Vec<f32>) -> Option<TimecycSample> {
    if values.len() < 35 {
        return None;
    }
    Some(TimecycSample {
        label,
        ambient: timecyc_color(&values, 0),
        ambient_object: timecyc_color(&values, 3),
        directional: timecyc_color(&values, 6),
        sky_top: timecyc_color(&values, 9),
        sky_bottom: timecyc_color(&values, 12),
        sun_core: timecyc_color(&values, 15),
        sun_corona: timecyc_color(&values, 18),
        postfx1: timecyc_postfx_rgba(&values, 40),
        postfx2: timecyc_postfx_rgba(&values, 44),
        far_clip: values.get(27).copied().unwrap_or(800.0),
        fog_start: values.get(28).copied().unwrap_or(100.0),
        light_on_ground: values.get(29).copied().unwrap_or(1.0),
        values,
    })
}

pub(crate) fn parse_timecyc_dat_text(text: &str) -> TimecycData {
    let mut weathers = Vec::<TimecycWeather>::new();
    let mut current: Option<TimecycWeather> = None;
    let mut pending_label: Option<String> = None;
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(name) = line.strip_prefix("////////////") {
            if let Some(weather) = current.take() {
                if !weather.samples.is_empty() {
                    weathers.push(weather);
                }
            }
            current = Some(TimecycWeather {
                name: name.trim().to_string(),
                samples: Vec::new(),
            });
            pending_label = None;
            continue;
        }
        if let Some(comment) = line.strip_prefix("//") {
            let label = comment.trim();
            if TIMECYC_HOURS
                .iter()
                .any(|hour| hour.eq_ignore_ascii_case(label))
            {
                pending_label = Some(label.to_string());
            }
            continue;
        }
        let Some(weather) = current.as_mut() else {
            continue;
        };
        let values: Vec<f32> = line
            .split_whitespace()
            .filter_map(|part| part.parse::<f32>().ok())
            .collect();
        if let Some(sample) = timecyc_sample_from_values(
            pending_label
                .take()
                .unwrap_or_else(|| format!("Sample {}", weather.samples.len() + 1)),
            values,
        ) {
            weather.samples.push(sample);
        }
    }
    if let Some(weather) = current.take() {
        if !weather.samples.is_empty() {
            weathers.push(weather);
        }
    }
    TimecycData {
        weathers,
        source: "built-in defaults".to_string(),
    }
}

pub(crate) fn vec3_json(value: V3) -> serde_json::Value {
    serde_json::json!([value.x, value.y, value.z])
}

pub(crate) fn save_timecyc_cache(data: &TimecycData) {
    let weathers: Vec<_> = data
        .weathers
        .iter()
        .map(|weather| {
            let samples: Vec<_> = weather
                .samples
                .iter()
                .map(|sample| {
                    serde_json::json!({
                        "label": sample.label,
                        "values": sample.values,
                        "ambient": vec3_json(sample.ambient),
                        "ambient_object": vec3_json(sample.ambient_object),
                        "directional": vec3_json(sample.directional),
                        "sky_top": vec3_json(sample.sky_top),
                        "sky_bottom": vec3_json(sample.sky_bottom),
                        "sun_core": vec3_json(sample.sun_core),
                        "sun_corona": vec3_json(sample.sun_corona),
                        "postfx1": sample.postfx1,
                        "postfx2": sample.postfx2,
                        "far_clip": sample.far_clip,
                        "fog_start": sample.fog_start,
                        "light_on_ground": sample.light_on_ground,
                    })
                })
                .collect();
            serde_json::json!({
                "name": weather.name,
                "samples": samples,
            })
        })
        .collect();
    let payload = serde_json::json!({
        "version": 1,
        "source": data.source,
        "hours": TIMECYC_HOURS,
        "weathers": weathers,
    });
    let cache_path = app_config_dir().join("timecyc_sa.json");
    if let Some(parent) = cache_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(text) = serde_json::to_string_pretty(&payload) {
        let _ = fs::write(cache_path, text);
    }
}

pub(crate) fn json_vec3(value: &serde_json::Value) -> V3 {
    let Some(array) = value.as_array() else {
        return V3::default();
    };
    V3 {
        x: array.first().and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
        y: array.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
        z: array.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
    }
}

pub(crate) fn load_timecyc_cache() -> Option<TimecycData> {
    let text = fs::read_to_string(app_config_dir().join("timecyc_sa.json"))
        .unwrap_or_else(|_| BUNDLED_TIMECYC_JSON.to_string());
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    let mut weathers = Vec::new();
    for weather_json in json.get("weathers")?.as_array()? {
        let name = weather_json.get("name")?.as_str()?.to_string();
        let mut samples = Vec::new();
        for sample_json in weather_json.get("samples")?.as_array()? {
            let values: Vec<f32> = sample_json
                .get("values")
                .and_then(|v| v.as_array())
                .map(|array| {
                    array
                        .iter()
                        .filter_map(|item| item.as_f64().map(|v| v as f32))
                        .collect()
                })
                .unwrap_or_default();
            let Some(mut sample) = timecyc_sample_from_values(
                sample_json
                    .get("label")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Sample")
                    .to_string(),
                values,
            ) else {
                continue;
            };
            sample.ambient = json_vec3(&sample_json["ambient"]);
            sample.ambient_object = json_vec3(&sample_json["ambient_object"]);
            sample.directional = json_vec3(&sample_json["directional"]);
            sample.sky_top = json_vec3(&sample_json["sky_top"]);
            sample.sky_bottom = json_vec3(&sample_json["sky_bottom"]);
            sample.sun_core = json_vec3(&sample_json["sun_core"]);
            sample.sun_corona = json_vec3(&sample_json["sun_corona"]);
            sample.far_clip = sample_json
                .get("far_clip")
                .and_then(|v| v.as_f64())
                .unwrap_or(sample.far_clip as f64) as f32;
            sample.fog_start = sample_json
                .get("fog_start")
                .and_then(|v| v.as_f64())
                .unwrap_or(sample.fog_start as f64) as f32;
            sample.light_on_ground = sample_json
                .get("light_on_ground")
                .and_then(|v| v.as_f64())
                .unwrap_or(sample.light_on_ground as f64)
                as f32;
            samples.push(sample);
        }
        if !samples.is_empty() {
            weathers.push(TimecycWeather { name, samples });
        }
    }
    if weathers.is_empty() {
        return None;
    }
    Some(TimecycData {
        weathers,
        source: json
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("bundled timecyc_sa.json")
            .to_string(),
    })
}

pub(crate) fn fallback_timecyc_data() -> TimecycData {
    TimecycData {
        source: "built-in fallback".to_string(),
        weathers: vec![TimecycWeather {
            name: "DEFAULT".to_string(),
            samples: vec![TimecycSample {
                label: "Midday".to_string(),
                values: Vec::new(),
                ambient: V3 {
                    x: PREVIEW_AMBIENT[0],
                    y: PREVIEW_AMBIENT[1],
                    z: PREVIEW_AMBIENT[2],
                },
                ambient_object: V3 {
                    x: 0.82,
                    y: 0.76,
                    z: 0.71,
                },
                directional: V3 {
                    x: PREVIEW_DIFFUSE[0],
                    y: PREVIEW_DIFFUSE[1],
                    z: PREVIEW_DIFFUSE[2],
                },
                sky_top: V3 {
                    x: 0.24,
                    y: 0.52,
                    z: 0.91,
                },
                sky_bottom: V3 {
                    x: 0.40,
                    y: 0.68,
                    z: 0.96,
                },
                sun_core: V3 {
                    x: 1.0,
                    y: 0.96,
                    z: 0.82,
                },
                sun_corona: V3 {
                    x: 1.0,
                    y: 0.72,
                    z: 0.42,
                },
                postfx1: [0.0; 4],
                postfx2: [0.0; 4],
                far_clip: 800.0,
                fog_start: 100.0,
                light_on_ground: 1.0,
            }],
        }],
    }
}

pub(crate) fn load_timecyc_state() -> TimecycState {
    let timecyc_path = load_gta_sa_dir_preference().join("data/timecyc.dat");
    let data = fs::read_to_string(&timecyc_path)
        .ok()
        .map(|text| {
            let mut data = parse_timecyc_dat_text(&text);
            if !data.weathers.is_empty() {
                save_timecyc_cache(&data);
            }
            data.source = timecyc_path.to_string_lossy().to_string();
            data
        })
        .filter(|data| !data.weathers.is_empty())
        .or_else(load_timecyc_cache)
        .unwrap_or_else(fallback_timecyc_data);
    let values = load_preferences();
    let weather_index = values
        .get("timecyc_weather")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0)
        .min(data.weathers.len().saturating_sub(1));
    let hour_count = data
        .weathers
        .get(weather_index)
        .map(|weather| weather.samples.len())
        .unwrap_or(1);
    let hour_index = values
        .get("timecyc_hour")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(4)
        .min(hour_count.saturating_sub(1));
    TimecycState {
        data,
        weather_index,
        hour_index,
    }
}

pub(crate) fn save_timecyc_preference(state: &TimecycState) {
    let mut values = load_preferences();
    values.insert(
        "timecyc_weather".to_string(),
        state.weather_index.to_string(),
    );
    values.insert("timecyc_hour".to_string(), state.hour_index.to_string());
    save_preferences(&values);
}

pub(crate) fn active_timecyc_sample(state: &TimecycState) -> &TimecycSample {
    let weather = &state.data.weathers[state.weather_index.min(state.data.weathers.len() - 1)];
    &weather.samples[state.hour_index.min(weather.samples.len() - 1)]
}

pub(crate) fn active_timecyc_weather_name(state: &TimecycState) -> &str {
    &state.data.weathers[state.weather_index.min(state.data.weathers.len() - 1)].name
}

pub(crate) fn vec3_rgba(value: V3, alpha: f32) -> [f32; 4] {
    [
        value.x.clamp(0.0, 1.0),
        value.y.clamp(0.0, 1.0),
        value.z.clamp(0.0, 1.0),
        alpha,
    ]
}

pub(crate) fn v3_mix(a: V3, b: V3, t: f32) -> V3 {
    let t = t.clamp(0.0, 1.0);
    V3 {
        x: a.x + (b.x - a.x) * t,
        y: a.y + (b.y - a.y) * t,
        z: a.z + (b.z - a.z) * t,
    }
}

pub(crate) fn v3_scale(value: V3, scale: f32) -> V3 {
    V3 {
        x: (value.x * scale).clamp(0.0, 1.0),
        y: (value.y * scale).clamp(0.0, 1.0),
        z: (value.z * scale).clamp(0.0, 1.0),
    }
}

pub(crate) fn v3_floor(value: V3, floor: f32) -> V3 {
    V3 {
        x: value.x.max(floor).clamp(0.0, 1.0),
        y: value.y.max(floor).clamp(0.0, 1.0),
        z: value.z.max(floor).clamp(0.0, 1.0),
    }
}

pub(crate) fn preview_ambient(sample: &TimecycSample) -> V3 {
    let sky_lift = v3_scale(v3_mix(sample.sky_bottom, sample.sky_top, 0.35), 0.35);
    let sa_lift = v3_mix(sample.ambient, sample.ambient_object, 0.42);
    let combined = V3 {
        x: (sa_lift.x + sky_lift.x).min(1.0),
        y: (sa_lift.y + sky_lift.y).min(1.0),
        z: (sa_lift.z + sky_lift.z).min(1.0),
    };
    v3_floor(combined, 0.24)
}

pub(crate) fn preview_material(sample: &TimecycSample) -> V3 {
    v3_floor(
        v3_mix(sample.ambient_object, preview_ambient(sample), 0.34),
        0.34,
    )
}

pub(crate) fn preview_diffuse(sample: &TimecycSample) -> V3 {
    v3_scale(v3_floor(sample.directional, 0.22), 0.38)
}

pub(crate) fn scene_ambient_lift_from_timecyc(timecyc: &TimecycState) -> V3 {
    // The custom building pipeline uses the world Ambient timecycle triplet.
    // Ambient_Obj is reserved for peds, vehicles, and ordinary objects.
    active_timecyc_sample(timecyc).ambient
}

pub(crate) fn vec3_csv(value: V3) -> String {
    format!("{:.3}, {:.3}, {:.3}", value.x, value.y, value.z)
}

pub(crate) fn color_from_temperature(kelvin: f32) -> V3 {
    let t = (kelvin.clamp(1000.0, 40000.0) / 100.0).max(10.0);
    let r = if t <= 66.0 {
        255.0
    } else {
        329.69873 * (t - 60.0).powf(-0.13320476)
    };
    let g = if t <= 66.0 {
        99.4708 * t.ln() - 161.11957
    } else {
        288.12216 * (t - 60.0).powf(-0.07551485)
    };
    let b = if t >= 66.0 {
        255.0
    } else if t <= 19.0 {
        0.0
    } else {
        138.51773 * (t - 10.0).ln() - 305.0448
    };
    V3 {
        x: (r / 255.0).clamp(0.0, 1.0),
        y: (g / 255.0).clamp(0.0, 1.0),
        z: (b / 255.0).clamp(0.0, 1.0),
    }
}

pub(crate) fn light_effective_color(light: &EditorLight) -> V3 {
    if light.use_temperature {
        color_from_temperature(light.temperature)
    } else {
        light.color
    }
}

pub(crate) fn default_lights() -> Vec<EditorLight> {
    vec![
        EditorLight {
            name: "Sun".to_string(),
            kind: LightKind::Directional,
            profile: LightProfile::Day,
            position: V3::default(),
            direction: V3 {
                x: 0.45,
                y: 0.35,
                z: -0.82,
            },
            color: V3 {
                x: 1.0,
                y: 0.96,
                z: 0.86,
            },
            temperature: 5600.0,
            use_temperature: false,
            intensity: 1.0,
            radius: 0.0,
            casts_shadow: true,
            point_lobe: PointLightLobe::Omni,
        },
        EditorLight {
            name: "Moon".to_string(),
            kind: LightKind::Directional,
            profile: LightProfile::Night,
            position: V3::default(),
            direction: V3 {
                x: -0.35,
                y: -0.25,
                z: -0.9,
            },
            color: V3 {
                x: 0.32,
                y: 0.48,
                z: 0.82,
            },
            temperature: 9000.0,
            use_temperature: false,
            intensity: 0.35,
            radius: 0.0,
            casts_shadow: true,
            point_lobe: PointLightLobe::Omni,
        },
    ]
}

pub(crate) fn light_list_path(root: &Path) -> PathBuf {
    root.join("Light_List.xml")
}

pub(crate) fn material_emitters_path(root: &Path) -> PathBuf {
    root.join("Light_Emitters.json")
}

pub(crate) fn shadow_casters_path(root: &Path) -> PathBuf {
    root.join("Shadow_Casters.json")
}

pub(crate) fn collision_capsules_path(root: &Path) -> PathBuf {
    root.join("Collision_Capsules.json")
}

pub(crate) fn collision_cuboids_path(root: &Path) -> PathBuf {
    root.join("Collision_Cuboids.json")
}

fn legacy_json_document(path: &Path) -> Result<Option<serde_json::Value>, String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(format!("Could not read {}: {err}", path.display())),
    };
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|err| format!("Invalid {}: {err}", path.display()))
}

fn section_or_legacy_json(
    root: &Path,
    section: &str,
    legacy_path: &Path,
) -> Result<Option<serde_json::Value>, String> {
    match read_eagle_scene_section(root, section)? {
        Some(value) => Ok(Some(value)),
        None => legacy_json_document(legacy_path),
    }
}

fn capsule_asset_key(name: &str) -> String {
    lower(name.trim())
}

fn json_v3(value: &serde_json::Value) -> Option<V3> {
    let values = value.as_array()?;
    if values.len() != 3 {
        return None;
    }
    let point = V3 {
        x: values[0].as_f64()? as f32,
        y: values[1].as_f64()? as f32,
        z: values[2].as_f64()? as f32,
    };
    (point.x.is_finite() && point.y.is_finite() && point.z.is_finite()).then_some(point)
}

pub(crate) fn load_collision_capsules(root: &Path, asset_name: &str) -> Vec<CollisionCapsule> {
    match read_eagle_scene_section(root, SECTION_COLLISION_CAPSULES) {
        Ok(Some(json)) => collision_capsules_from_json(&json, asset_name),
        Ok(None) => legacy_json_document(&collision_capsules_path(root))
            .ok()
            .flatten()
            .map(|json| collision_capsules_from_json(&json, asset_name))
            .unwrap_or_default(),
        Err(err) => {
            eprintln!("Could not load EagleScene collision capsules: {err}");
            Vec::new()
        }
    }
}

fn collision_capsules_from_json(
    json: &serde_json::Value,
    asset_name: &str,
) -> Vec<CollisionCapsule> {
    let Some(entries) = json
        .get("assets")
        .and_then(|assets| assets.get(capsule_asset_key(asset_name)))
        .and_then(serde_json::Value::as_array)
    else {
        return Vec::new();
    };
    entries
        .iter()
        .filter_map(collision_capsule_from_json)
        .collect()
}

fn collision_capsule_from_json(entry: &serde_json::Value) -> Option<CollisionCapsule> {
    let start = json_v3(entry.get("start")?)?;
    let end = json_v3(entry.get("end")?)?;
    let radius = entry.get("radius")?.as_f64()? as f32;
    if !radius.is_finite() || radius <= 0.0 {
        return None;
    }
    let sphere_indices = entry.get("sphereIndices")?.as_array()?;
    if sphere_indices.len() != 2 {
        return None;
    }
    let byte = |name: &str, default: u8| {
        entry
            .get(name)
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| u8::try_from(value).ok())
            .unwrap_or(default)
    };
    Some(CollisionCapsule {
        start,
        end,
        radius,
        round_edges: entry
            .get("roundEdges")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true),
        surface: CollisionSurface {
            material: byte("material", 0),
            flags: byte("flags", 0),
            brightness: byte("brightness", 0),
            light: byte("light", 255),
        },
        sphere_indices: [
            usize::try_from(sphere_indices[0].as_u64()?).ok()?,
            usize::try_from(sphere_indices[1].as_u64()?).ok()?,
        ],
        vertex_start: usize::try_from(entry.get("vertexStart")?.as_u64()?).ok()?,
        vertex_count: usize::try_from(entry.get("vertexCount")?.as_u64()?).ok()?,
        face_start: usize::try_from(entry.get("faceStart")?.as_u64()?).ok()?,
        face_count: usize::try_from(entry.get("faceCount")?.as_u64()?).ok()?,
    })
}

pub(crate) fn save_collision_capsules(
    root: &Path,
    asset_name: &str,
    capsules: &[CollisionCapsule],
) -> Result<(), String> {
    let mut json = section_or_legacy_json(
        root,
        SECTION_COLLISION_CAPSULES,
        &collision_capsules_path(root),
    )?
    .unwrap_or_else(|| serde_json::json!({"version": 1, "assets": {}}));
    validate_collision_capsules_json(&json)
        .map_err(|err| format!("Could not update EagleScene collision capsules safely: {err}"))?;
    if !json.get("assets").is_some_and(serde_json::Value::is_object) {
        json["assets"] = serde_json::json!({});
    }
    let key = capsule_asset_key(asset_name);
    if capsules.is_empty() {
        if let Some(assets) = json
            .get_mut("assets")
            .and_then(serde_json::Value::as_object_mut)
        {
            assets.remove(&key);
        }
    } else {
        let entries = capsules.iter().map(collision_capsule_to_json).collect();
        json["assets"][key] = serde_json::Value::Array(entries);
    }
    json["version"] = serde_json::json!(1);
    update_eagle_scene_section(root, SECTION_COLLISION_CAPSULES, json)
}

fn collision_capsule_to_json(capsule: &CollisionCapsule) -> serde_json::Value {
    serde_json::json!({
        "start": [capsule.start.x, capsule.start.y, capsule.start.z],
        "end": [capsule.end.x, capsule.end.y, capsule.end.z],
        "radius": capsule.radius,
        "roundEdges": capsule.round_edges,
        "material": capsule.surface.material,
        "flags": capsule.surface.flags,
        "brightness": capsule.surface.brightness,
        "light": capsule.surface.light,
        "sphereIndices": capsule.sphere_indices,
        "vertexStart": capsule.vertex_start,
        "vertexCount": capsule.vertex_count,
        "faceStart": capsule.face_start,
        "faceCount": capsule.face_count,
    })
}

fn validate_collision_capsules_json(json: &serde_json::Value) -> Result<(), String> {
    validate_scene_sidecar_version(json, 1)?;
    let assets = json
        .get("assets")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| "`assets` must be an object".to_string())?;
    for (asset, entries) in assets {
        if capsule_asset_key(asset) != *asset {
            return Err(format!("asset key `{asset}` is not normalized"));
        }
        let entries = entries
            .as_array()
            .ok_or_else(|| format!("assets.{asset} must be an array"))?;
        for (index, entry) in entries.iter().enumerate() {
            if collision_capsule_from_json(entry).is_none() {
                return Err(format!("assets.{asset}[{index}] is not a valid capsule"));
            }
        }
    }
    Ok(())
}

pub(crate) fn load_collision_cuboids(root: &Path, asset_name: &str) -> Vec<CollisionCuboid> {
    match read_eagle_scene_section(root, SECTION_COLLISION_CUBOIDS) {
        Ok(Some(json)) => collision_cuboids_from_json(&json, asset_name),
        Ok(None) => legacy_json_document(&collision_cuboids_path(root))
            .ok()
            .flatten()
            .map(|json| collision_cuboids_from_json(&json, asset_name))
            .unwrap_or_default(),
        Err(err) => {
            eprintln!("Could not load EagleScene collision cuboids: {err}");
            Vec::new()
        }
    }
}

fn collision_cuboids_from_json(json: &serde_json::Value, asset_name: &str) -> Vec<CollisionCuboid> {
    let Some(entries) = json
        .get("assets")
        .and_then(|assets| assets.get(capsule_asset_key(asset_name)))
        .and_then(serde_json::Value::as_array)
    else {
        return Vec::new();
    };
    entries
        .iter()
        .filter_map(collision_cuboid_from_json)
        .collect()
}

fn collision_cuboid_from_json(entry: &serde_json::Value) -> Option<CollisionCuboid> {
    let center = json_v3(entry.get("center")?)?;
    let half_extents = json_v3(entry.get("halfExtents")?)?;
    let rotation = json_v3(entry.get("rotation")?)?;
    if half_extents.x <= 0.0 || half_extents.y <= 0.0 || half_extents.z <= 0.0 {
        return None;
    }
    let byte = |name: &str, default: u8| {
        entry
            .get(name)
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| u8::try_from(value).ok())
            .unwrap_or(default)
    };
    Some(CollisionCuboid {
        center,
        half_extents,
        rotation,
        surface: CollisionSurface {
            material: byte("material", 0),
            flags: byte("flags", 0),
            brightness: byte("brightness", 0),
            light: byte("light", 255),
        },
        vertex_start: usize::try_from(entry.get("vertexStart")?.as_u64()?).ok()?,
        vertex_count: usize::try_from(entry.get("vertexCount")?.as_u64()?).ok()?,
        face_start: usize::try_from(entry.get("faceStart")?.as_u64()?).ok()?,
        face_count: usize::try_from(entry.get("faceCount")?.as_u64()?).ok()?,
    })
}

pub(crate) fn save_collision_cuboids(
    root: &Path,
    asset_name: &str,
    cuboids: &[CollisionCuboid],
) -> Result<(), String> {
    let mut json = section_or_legacy_json(
        root,
        SECTION_COLLISION_CUBOIDS,
        &collision_cuboids_path(root),
    )?
    .unwrap_or_else(|| serde_json::json!({"version": 1, "assets": {}}));
    validate_collision_cuboids_json(&json)
        .map_err(|err| format!("Could not update EagleScene collision cuboids safely: {err}"))?;
    if !json.get("assets").is_some_and(serde_json::Value::is_object) {
        json["assets"] = serde_json::json!({});
    }
    let key = capsule_asset_key(asset_name);
    if cuboids.is_empty() {
        if let Some(assets) = json
            .get_mut("assets")
            .and_then(serde_json::Value::as_object_mut)
        {
            assets.remove(&key);
        }
    } else {
        let entries = cuboids.iter().map(collision_cuboid_to_json).collect();
        json["assets"][key] = serde_json::Value::Array(entries);
    }
    json["version"] = serde_json::json!(1);
    update_eagle_scene_section(root, SECTION_COLLISION_CUBOIDS, json)
}

fn collision_cuboid_to_json(cuboid: &CollisionCuboid) -> serde_json::Value {
    serde_json::json!({
        "center": [cuboid.center.x, cuboid.center.y, cuboid.center.z],
        "halfExtents": [
            cuboid.half_extents.x,
            cuboid.half_extents.y,
            cuboid.half_extents.z
        ],
        "rotation": [cuboid.rotation.x, cuboid.rotation.y, cuboid.rotation.z],
        "material": cuboid.surface.material,
        "flags": cuboid.surface.flags,
        "brightness": cuboid.surface.brightness,
        "light": cuboid.surface.light,
        "vertexStart": cuboid.vertex_start,
        "vertexCount": cuboid.vertex_count,
        "faceStart": cuboid.face_start,
        "faceCount": cuboid.face_count,
    })
}

fn validate_collision_cuboids_json(json: &serde_json::Value) -> Result<(), String> {
    validate_scene_sidecar_version(json, 1)?;
    let assets = json
        .get("assets")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| "`assets` must be an object".to_string())?;
    for (asset, entries) in assets {
        if capsule_asset_key(asset) != *asset {
            return Err(format!("asset key `{asset}` is not normalized"));
        }
        let entries = entries
            .as_array()
            .ok_or_else(|| format!("assets.{asset} must be an array"))?;
        for (index, entry) in entries.iter().enumerate() {
            if collision_cuboid_from_json(entry).is_none() {
                return Err(format!("assets.{asset}[{index}] is not a valid cuboid"));
            }
        }
    }
    Ok(())
}

fn validate_scene_sidecar_version(json: &serde_json::Value, expected: u64) -> Result<(), String> {
    let object = json
        .as_object()
        .ok_or_else(|| "the document root must be an object".to_string())?;
    match object.get("version").and_then(serde_json::Value::as_u64) {
        Some(version) if version == expected => Ok(()),
        Some(version) => Err(format!(
            "unsupported version {version}; expected {expected}"
        )),
        None => Err("missing numeric `version`".to_string()),
    }
}

pub(crate) fn material_emitter_key(dff_name: &str, material: usize) -> String {
    format!("{}|{material}", asset_key(dff_name, ".dff"))
}

const MATERIAL_EMITTER_FACE_PREFIX: &str = "face|";
const MATERIAL_EMITTER_FACE_GROUP_PREFIX: &str = "face-group|";

pub(crate) fn material_emitter_face_key(dff_name: &str, face: usize) -> String {
    format!(
        "{MATERIAL_EMITTER_FACE_PREFIX}{}|{face}",
        asset_key(dff_name, ".dff")
    )
}

pub(crate) fn material_emitter_face_from_key(key: &str) -> Option<(&str, usize)> {
    let value = key.strip_prefix(MATERIAL_EMITTER_FACE_PREFIX)?;
    let (dff, face) = value.rsplit_once('|')?;
    Some((dff, face.parse().ok()?))
}

pub(crate) fn material_emitter_face_group_key(dff_name: &str, faces: &[usize]) -> String {
    let mut faces = faces.to_vec();
    faces.sort_unstable();
    faces.dedup();
    let faces = faces
        .into_iter()
        .map(|face| face.to_string())
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{MATERIAL_EMITTER_FACE_GROUP_PREFIX}{}|{faces}",
        asset_key(dff_name, ".dff")
    )
}

pub(crate) fn material_emitter_face_group_from_key(key: &str) -> Option<(&str, Vec<usize>)> {
    let value = key.strip_prefix(MATERIAL_EMITTER_FACE_GROUP_PREFIX)?;
    let (dff, faces) = value.rsplit_once('|')?;
    let faces = faces
        .split(',')
        .map(str::parse)
        .collect::<Result<Vec<usize>, _>>()
        .ok()?;
    (!faces.is_empty()).then_some((dff, faces))
}

const MATERIAL_EMITTER_TEXTURE_PREFIX: &str = "texture|";

pub(crate) fn material_emitter_texture_key(texture_name: &str) -> String {
    format!(
        "{MATERIAL_EMITTER_TEXTURE_PREFIX}{}",
        lower(texture_name.trim())
    )
}

pub(crate) fn material_emitter_texture_from_key(key: &str) -> Option<&str> {
    key.strip_prefix(MATERIAL_EMITTER_TEXTURE_PREFIX)
}

pub(crate) fn load_shadow_casting(root: &Path) -> HashMap<String, bool> {
    match read_eagle_scene_section(root, SECTION_SHADOW_CASTERS) {
        Ok(Some(json)) => shadow_casting_from_json(&json),
        Ok(None) => legacy_json_document(&shadow_casters_path(root))
            .ok()
            .flatten()
            .map(|json| shadow_casting_from_json(&json))
            .unwrap_or_default(),
        Err(err) => {
            eprintln!("Could not load EagleScene shadow casters: {err}");
            HashMap::new()
        }
    }
}

fn shadow_casting_from_json(json: &serde_json::Value) -> HashMap<String, bool> {
    let Some(entries) = json
        .get("overrides")
        .or_else(|| json.get("nonShadowCasters"))
        .and_then(|value| value.as_array())
    else {
        return HashMap::new();
    };
    entries
        .iter()
        .filter_map(|entry| {
            let scope = entry.get("scope").and_then(|value| value.as_str());
            if scope == Some("face") {
                let dff = entry.get("dff")?.as_str()?;
                let face = entry.get("face")?.as_u64()? as usize;
                Some((
                    material_emitter_face_key(dff, face),
                    entry
                        .get("castsShadow")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false),
                ))
            } else if scope == Some("texture") {
                let texture = entry.get("texture")?.as_str()?.trim();
                (!texture.is_empty()).then(|| {
                    (
                        material_emitter_texture_key(texture),
                        entry
                            .get("castsShadow")
                            .and_then(|value| value.as_bool())
                            .unwrap_or(false),
                    )
                })
            } else {
                let dff = entry.get("dff")?.as_str()?;
                let material = entry.get("material")?.as_u64()? as usize;
                Some((
                    material_emitter_key(dff, material),
                    entry
                        .get("castsShadow")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false),
                ))
            }
        })
        .collect()
}

#[allow(dead_code)]
pub(crate) fn save_shadow_casting(
    root: &Path,
    shadow_casting: &HashMap<String, bool>,
) -> Result<(), String> {
    update_eagle_scene_section(
        root,
        SECTION_SHADOW_CASTERS,
        shadow_casting_section_value(shadow_casting),
    )
}

pub(crate) fn shadow_casting_section_value(
    shadow_casting: &HashMap<String, bool>,
) -> serde_json::Value {
    let mut keys: Vec<_> = shadow_casting.keys().collect();
    keys.sort();
    let entries: Vec<_> = keys
        .into_iter()
        .filter_map(|key| {
            let casts_shadow = shadow_casting.get(key).copied()?;
            if let Some((dff, face)) = material_emitter_face_from_key(key) {
                Some(serde_json::json!({
                    "scope": "face",
                    "dff": dff,
                    "face": face,
                    "castsShadow": casts_shadow,
                }))
            } else if let Some(texture) = material_emitter_texture_from_key(key) {
                Some(serde_json::json!({
                    "scope": "texture",
                    "texture": texture,
                    "castsShadow": casts_shadow,
                }))
            } else {
                let (dff, material) = key.rsplit_once('|')?;
                Some(serde_json::json!({
                    "scope": "material",
                    "dff": dff,
                    "material": material.parse::<usize>().ok()?,
                    "castsShadow": casts_shadow,
                }))
            }
        })
        .collect();
    serde_json::json!({
        "version": 1,
        "overrides": entries,
    })
}

fn strict_shadow_casting_json(json: &serde_json::Value) -> Result<(), String> {
    validate_scene_sidecar_version(json, 1)?;
    let entries = json
        .get("overrides")
        .or_else(|| json.get("nonShadowCasters"))
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "`overrides` must be an array".to_string())?;
    let parsed = shadow_casting_from_json(json);
    if parsed.len() != entries.len() {
        return Err(
            "one or more shadow overrides are invalid or resolve to a duplicate key".to_string(),
        );
    }
    Ok(())
}

pub(crate) fn load_material_emitters(root: &Path) -> HashMap<String, MaterialEmitter> {
    match read_eagle_scene_section(root, SECTION_MATERIAL_EMITTERS) {
        Ok(Some(json)) => material_emitters_from_json(&json),
        Ok(None) => legacy_json_document(&material_emitters_path(root))
            .ok()
            .flatten()
            .map(|json| material_emitters_from_json(&json))
            .unwrap_or_default(),
        Err(err) => {
            eprintln!("Could not load EagleScene material emitters: {err}");
            HashMap::new()
        }
    }
}

fn material_emitters_from_json(json: &serde_json::Value) -> HashMap<String, MaterialEmitter> {
    let Some(entries) = json.get("emitters").and_then(|value| value.as_array()) else {
        return HashMap::new();
    };
    let mut result = HashMap::new();
    for entry in entries {
        let scope = entry.get("scope").and_then(|value| value.as_str());
        let key = if scope == Some("faceGroup") {
            let Some(dff) = entry.get("dff").and_then(|value| value.as_str()) else {
                continue;
            };
            let Some(faces) = entry.get("faces").and_then(|value| value.as_array()) else {
                continue;
            };
            let faces: Vec<_> = faces
                .iter()
                .filter_map(|face| face.as_u64().map(|face| face as usize))
                .collect();
            if faces.is_empty() {
                continue;
            }
            material_emitter_face_group_key(dff, &faces)
        } else if scope == Some("face") {
            let Some(dff) = entry.get("dff").and_then(|value| value.as_str()) else {
                continue;
            };
            let Some(face) = entry.get("face").and_then(|value| value.as_u64()) else {
                continue;
            };
            material_emitter_face_key(dff, face as usize)
        } else if let Some(texture) = entry.get("texture").and_then(|value| value.as_str()) {
            let texture = texture.trim();
            if texture.is_empty() {
                continue;
            }
            material_emitter_texture_key(texture)
        } else {
            let Some(dff) = entry.get("dff").and_then(|value| value.as_str()) else {
                continue;
            };
            let Some(material) = entry.get("material").and_then(|value| value.as_u64()) else {
                continue;
            };
            material_emitter_key(dff, material as usize)
        };
        let color = entry
            .get("color")
            .map(json_vec3)
            .unwrap_or_else(neutral_vertex_color);
        let emitter = MaterialEmitter {
            enabled: entry
                .get("enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            emit_inversed: entry
                .get("emitInversed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            cast_mode: if entry
                .get("castMode")
                .and_then(|v| v.as_str())
                .is_some_and(|mode| mode.eq_ignore_ascii_case("point"))
            {
                MaterialEmitterCastMode::Point
            } else {
                MaterialEmitterCastMode::Face
            },
            max_grouping_size: entry
                .get("maxGroupingSize")
                .and_then(|v| v.as_f64())
                .map(|value| (value as f32).clamp(0.01, 100_000.0))
                .unwrap_or(2.0),
            point_up_strength: entry
                .get("pointUpStrength")
                .and_then(|v| v.as_f64())
                .map(|value| (value as f32).clamp(0.0, 100.0))
                .unwrap_or(1.0),
            point_down_strength: entry
                .get("pointDownStrength")
                .and_then(|v| v.as_f64())
                .map(|value| (value as f32).clamp(0.0, 100.0))
                .unwrap_or(1.0),
            point_sides_strength: entry
                .get("pointSidesStrength")
                .and_then(|v| v.as_f64())
                .map(|value| (value as f32).clamp(0.0, 100.0))
                .unwrap_or(1.0),
            use_material_color: entry
                .get("colorSource")
                .and_then(|v| v.as_str())
                .is_none_or(|value| value == "material"),
            use_temperature: entry
                .get("colorSource")
                .and_then(|v| v.as_str())
                .is_some_and(|value| value == "temperature"),
            color: v3_clamp01(color),
            temperature: entry
                .get("temperature")
                .and_then(|v| v.as_f64())
                .map(|value| (value as f32).clamp(1000.0, 40000.0))
                .unwrap_or(6500.0),
            strength: entry
                .get("strength")
                .and_then(|v| v.as_f64())
                .map(|value| (value as f32).clamp(0.0, 100.0))
                .unwrap_or(1.0),
            falloff_distance: entry
                .get("falloffDistance")
                .and_then(|v| v.as_f64())
                .map(|value| (value as f32).clamp(1.0, 100_000.0))
                .unwrap_or(512.0),
            day: entry.get("day").and_then(|v| v.as_bool()).unwrap_or(true),
            night: entry.get("night").and_then(|v| v.as_bool()).unwrap_or(true),
        };
        result.insert(key, emitter);
    }
    result
}

#[allow(dead_code)]
pub(crate) fn save_material_emitters(
    root: &Path,
    emitters: &HashMap<String, MaterialEmitter>,
) -> Result<(), String> {
    update_eagle_scene_section(
        root,
        SECTION_MATERIAL_EMITTERS,
        material_emitters_section_value(emitters),
    )
}

pub(crate) fn material_emitters_section_value(
    emitters: &HashMap<String, MaterialEmitter>,
) -> serde_json::Value {
    let mut keys: Vec<_> = emitters.keys().collect();
    keys.sort();
    let entries: Vec<_> = keys
        .into_iter()
        .filter_map(|key| {
            let emitter = emitters.get(key)?;
            let mut entry = serde_json::json!({
                "enabled": emitter.enabled,
                "emitInversed": emitter.emit_inversed,
                "castMode": match emitter.cast_mode {
                    MaterialEmitterCastMode::Face => "face",
                    MaterialEmitterCastMode::Point => "point",
                },
                "maxGroupingSize": emitter.max_grouping_size,
                "pointUpStrength": emitter.point_up_strength,
                "pointDownStrength": emitter.point_down_strength,
                "pointSidesStrength": emitter.point_sides_strength,
                "colorSource": if emitter.use_temperature {
                    "temperature"
                } else if emitter.use_material_color {
                    "material"
                } else {
                    "custom"
                },
                "color": [emitter.color.x, emitter.color.y, emitter.color.z],
                "temperature": emitter.temperature,
                "strength": emitter.strength,
                "falloffDistance": emitter.falloff_distance,
                "day": emitter.day,
                "night": emitter.night,
            });
            if let Some((dff, faces)) = material_emitter_face_group_from_key(key) {
                entry["scope"] = serde_json::json!("faceGroup");
                entry["dff"] = serde_json::json!(dff);
                entry["faces"] = serde_json::json!(faces);
            } else if let Some((dff, face)) = material_emitter_face_from_key(key) {
                entry["scope"] = serde_json::json!("face");
                entry["dff"] = serde_json::json!(dff);
                entry["face"] = serde_json::json!(face);
            } else if let Some(texture) = material_emitter_texture_from_key(key) {
                entry["scope"] = serde_json::json!("texture");
                entry["texture"] = serde_json::json!(texture);
            } else {
                let (dff, material) = key.rsplit_once('|')?;
                entry["scope"] = serde_json::json!("material");
                entry["dff"] = serde_json::json!(dff);
                entry["material"] = serde_json::json!(material.parse::<usize>().ok()?);
            }
            Some(entry)
        })
        .collect();
    serde_json::json!({
        "version": 2,
        "emitters": entries,
    })
}

fn strict_material_emitters_json(json: &serde_json::Value) -> Result<(), String> {
    let object = json
        .as_object()
        .ok_or_else(|| "the document root must be an object".to_string())?;
    match object.get("version").and_then(serde_json::Value::as_u64) {
        Some(1 | 2) => {}
        Some(version) => {
            return Err(format!(
                "unsupported material-emitter version {version}; expected 1 or 2"
            ));
        }
        None => return Err("missing numeric `version`".to_string()),
    }
    let entries = json
        .get("emitters")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "`emitters` must be an array".to_string())?;
    for (index, entry) in entries.iter().enumerate() {
        if entry.get("scope").and_then(serde_json::Value::as_str) == Some("faceGroup") {
            let faces = entry
                .get("faces")
                .and_then(serde_json::Value::as_array)
                .ok_or_else(|| format!("emitters[{index}].faces must be an array"))?;
            if faces.is_empty() || faces.iter().any(|face| face.as_u64().is_none()) {
                return Err(format!(
                    "emitters[{index}].faces must contain unsigned face indices"
                ));
            }
        }
    }
    let parsed = material_emitters_from_json(json);
    if parsed.len() != entries.len() {
        return Err("one or more emitters are invalid or resolve to a duplicate key".to_string());
    }
    Ok(())
}

pub(crate) fn load_legacy_material_emitters_section(
    root: &Path,
) -> Result<Option<serde_json::Value>, String> {
    let path = material_emitters_path(root);
    let Some(json) = legacy_json_document(&path)? else {
        return Ok(None);
    };
    strict_material_emitters_json(&json)
        .map_err(|err| format!("{} cannot be migrated safely: {err}", path.display()))?;
    Ok(Some(json))
}

pub(crate) fn load_legacy_shadow_casters_section(
    root: &Path,
) -> Result<Option<serde_json::Value>, String> {
    let path = shadow_casters_path(root);
    let Some(json) = legacy_json_document(&path)? else {
        return Ok(None);
    };
    strict_shadow_casting_json(&json)
        .map_err(|err| format!("{} cannot be migrated safely: {err}", path.display()))?;
    Ok(Some(json))
}

pub(crate) fn load_legacy_collision_capsules_section(
    root: &Path,
) -> Result<Option<serde_json::Value>, String> {
    let path = collision_capsules_path(root);
    let Some(json) = legacy_json_document(&path)? else {
        return Ok(None);
    };
    validate_collision_capsules_json(&json)
        .map_err(|err| format!("{} cannot be migrated safely: {err}", path.display()))?;
    Ok(Some(json))
}

pub(crate) fn load_legacy_collision_cuboids_section(
    root: &Path,
) -> Result<Option<serde_json::Value>, String> {
    let path = collision_cuboids_path(root);
    let Some(json) = legacy_json_document(&path)? else {
        return Ok(None);
    };
    validate_collision_cuboids_json(&json)
        .map_err(|err| format!("{} cannot be migrated safely: {err}", path.display()))?;
    Ok(Some(json))
}

#[cfg(test)]
mod material_emitter_tests {
    use super::*;

    #[test]
    fn numeric_attribute_helpers_reject_non_finite_values() {
        let attrs = BTreeMap::from([
            ("finite".to_string(), "12.5".to_string()),
            ("nan".to_string(), "NaN".to_string()),
            ("infinite".to_string(), "inf".to_string()),
        ]);

        assert_eq!(attrf(&attrs, "finite"), 12.5);
        assert_eq!(attrf(&attrs, "nan"), 0.0);
        assert_eq!(attrf_default(&attrs, "infinite", 7.0), 7.0);
    }

    #[test]
    fn vec3_text_rejects_non_finite_components() {
        let default = V3 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };

        assert_eq!(
            parse_vec3_text("4 5 6", default),
            V3 {
                x: 4.0,
                y: 5.0,
                z: 6.0
            }
        );
        assert_eq!(parse_vec3_text("NaN 5 6", default), default);
        assert_eq!(parse_vec3_text("4 inf 6", default), default);
    }

    #[test]
    fn material_emitters_round_trip_day_night_and_color_source() {
        let root = std::env::temp_dir().join(format!(
            "eagle_emitters_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let mut emitters = HashMap::new();
        emitters.insert(
            material_emitter_key("Neon.DFF", 3),
            MaterialEmitter {
                enabled: true,
                emit_inversed: true,
                cast_mode: MaterialEmitterCastMode::Point,
                max_grouping_size: 3.5,
                point_up_strength: 0.0,
                point_down_strength: 2.0,
                point_sides_strength: 0.5,
                use_material_color: false,
                use_temperature: false,
                color: V3 {
                    x: 0.2,
                    y: 0.4,
                    z: 0.8,
                },
                temperature: 6500.0,
                strength: 2.75,
                falloff_distance: 900.0,
                day: false,
                night: true,
            },
        );
        emitters.insert(
            material_emitter_texture_key("Shop_Neon"),
            MaterialEmitter {
                enabled: true,
                emit_inversed: false,
                cast_mode: MaterialEmitterCastMode::Face,
                max_grouping_size: 2.0,
                point_up_strength: 1.0,
                point_down_strength: 1.0,
                point_sides_strength: 1.0,
                use_material_color: true,
                use_temperature: false,
                color: neutral_vertex_color(),
                temperature: 6500.0,
                strength: 4.0,
                falloff_distance: 320.0,
                day: true,
                night: true,
            },
        );
        emitters.insert(
            material_emitter_face_key("Lamp.DFF", 17),
            MaterialEmitter {
                enabled: true,
                emit_inversed: true,
                cast_mode: MaterialEmitterCastMode::Face,
                max_grouping_size: 2.0,
                point_up_strength: 1.0,
                point_down_strength: 1.0,
                point_sides_strength: 1.0,
                use_material_color: true,
                use_temperature: false,
                color: neutral_vertex_color(),
                temperature: 6500.0,
                strength: 3.0,
                falloff_distance: 256.0,
                day: false,
                night: true,
            },
        );
        emitters.insert(
            material_emitter_face_group_key("Sign.DFF", &[9, 3, 7]),
            MaterialEmitter {
                enabled: false,
                emit_inversed: false,
                cast_mode: MaterialEmitterCastMode::Point,
                max_grouping_size: 12.0,
                point_up_strength: 3.0,
                point_down_strength: 0.0,
                point_sides_strength: 0.25,
                use_material_color: false,
                use_temperature: true,
                color: V3 {
                    x: 1.0,
                    y: 0.25,
                    z: 0.1,
                },
                temperature: 3200.0,
                strength: 5.0,
                falloff_distance: 128.0,
                day: true,
                night: false,
            },
        );
        save_material_emitters(&root, &emitters).unwrap();
        assert!(!material_emitters_path(&root).exists());
        let loaded = load_material_emitters(&root);
        assert_eq!(loaded, emitters);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn shadow_casting_overrides_round_trip_all_scopes() {
        let root = std::env::temp_dir().join(format!(
            "eagle_shadow_casters_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let overrides = HashMap::from([
            (material_emitter_texture_key("glass_wall"), false),
            (material_emitter_key("tower.dff", 2), false),
            (material_emitter_face_key("tower.dff", 17), true),
        ]);
        save_shadow_casting(&root, &overrides).unwrap();
        assert!(!shadow_casters_path(&root).exists());
        assert_eq!(load_shadow_casting(&root), overrides);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn legacy_emitter_metadata_defaults_to_face_casting() {
        let root = std::env::temp_dir().join(format!(
            "eagle_legacy_emitter_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            material_emitters_path(&root),
            r#"{
  "version": 2,
  "emitters": [{
    "scope": "texture",
    "texture": "legacy_lamp",
    "enabled": true,
    "strength": 1.0,
    "falloffDistance": 512.0
  }]
}"#,
        )
        .unwrap();
        let loaded = load_material_emitters(&root);
        let emitter = loaded
            .get(&material_emitter_texture_key("legacy_lamp"))
            .unwrap();
        assert_eq!(emitter.cast_mode, MaterialEmitterCastMode::Face);
        assert!((emitter.max_grouping_size - 2.0).abs() < 0.0001);
        assert!((emitter.point_up_strength - 1.0).abs() < 0.0001);
        assert!((emitter.point_down_strength - 1.0).abs() < 0.0001);
        assert!((emitter.point_sides_strength - 1.0).abs() < 0.0001);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unified_emitter_section_takes_priority_over_legacy() {
        let root = std::env::temp_dir().join(format!(
            "eagle_emitter_priority_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            material_emitters_path(&root),
            r#"{"version":2,"emitters":[{"scope":"texture","texture":"legacy","enabled":true}]}"#,
        )
        .unwrap();
        update_eagle_scene_section(
            &root,
            SECTION_MATERIAL_EMITTERS,
            serde_json::json!({
                "version": 2,
                "emitters": [{"scope": "texture", "texture": "unified", "enabled": true}]
            }),
        )
        .unwrap();

        let loaded = load_material_emitters(&root);

        assert!(loaded.contains_key(&material_emitter_texture_key("unified")));
        assert!(!loaded.contains_key(&material_emitter_texture_key("legacy")));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn strict_legacy_collector_converts_all_owned_documents() {
        let root = std::env::temp_dir().join(format!(
            "eagle_legacy_scene_collect_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            material_emitters_path(&root),
            r#"{"version":2,"emitters":[]}"#,
        )
        .unwrap();
        fs::write(
            shadow_casters_path(&root),
            r#"{"version":1,"overrides":[]}"#,
        )
        .unwrap();
        fs::write(
            collision_capsules_path(&root),
            r#"{"version":1,"assets":{}}"#,
        )
        .unwrap();
        fs::write(
            collision_cuboids_path(&root),
            r#"{"version":1,"assets":{}}"#,
        )
        .unwrap();
        fs::write(
            light_list_path(&root),
            r#"<Light_List version="1">
<light name="Lamp" kind="point" profile="night" posX="1" posY="2" posZ="3" dirX="0" dirY="0" dirZ="-1" colorX="0.5" colorY="0.6" colorZ="0.7" temperature="4200" useTemperature="true" intensity="2.5" radius="64" castsShadow="true" pointLobe="sides" />
</Light_List>
"#,
        )
        .unwrap();
        let attrs = Regex::new(r#"([A-Za-z_][A-Za-z0-9_]*)="([^"]*)""#).unwrap();

        let imports = collect_legacy_scene_section_imports(&root, &attrs).unwrap();

        assert_eq!(imports.len(), 5);
        let lights = imports
            .iter()
            .find(|import| import.file_name == "Light_List.xml")
            .unwrap();
        assert_eq!(lights.value["version"], 1);
        assert_eq!(lights.value["lights"][0]["pointLobe"], "sides");
        assert_eq!(
            lights.value["lights"][0]["position"],
            serde_json::json!([1.0, 2.0, 3.0])
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn strict_legacy_collector_rejects_a_partially_invalid_document() {
        let root = std::env::temp_dir().join(format!(
            "eagle_legacy_scene_reject_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            material_emitters_path(&root),
            r#"{"version":2,"emitters":[{"scope":"faceGroup","dff":"lamp.dff","faces":[1,"bad"]}]}"#,
        )
        .unwrap();
        let attrs = Regex::new(r#"([A-Za-z_][A-Za-z0-9_]*)="([^"]*)""#).unwrap();

        let error = collect_legacy_scene_section_imports(&root, &attrs).unwrap_err();

        assert!(error.contains("cannot be migrated safely"));
        assert!(material_emitters_path(&root).is_file());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unified_lights_round_trip_every_editor_field() {
        let root = std::env::temp_dir().join(format!(
            "eagle_lights_round_trip_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let lights = vec![EditorLight {
            name: "Side Lamp".to_string(),
            kind: LightKind::Point,
            profile: LightProfile::Night,
            position: V3 {
                x: 1.25,
                y: -2.5,
                z: 3.75,
            },
            direction: V3 {
                x: 0.1,
                y: 0.2,
                z: -0.9,
            },
            color: V3 {
                x: 0.25,
                y: 0.5,
                z: 0.75,
            },
            temperature: 4200.0,
            use_temperature: true,
            intensity: 2.75,
            radius: 96.0,
            casts_shadow: true,
            point_lobe: PointLightLobe::Sides,
        }];
        let attrs = Regex::new(r#"([A-Za-z_][A-Za-z0-9_]*)="([^"]*)""#).unwrap();

        save_project_lights(&root, &lights).unwrap();
        let loaded = load_project_lights(&root, &light_list_path(&root), &attrs).unwrap();

        assert!(loaded == lights);
        assert!(!light_list_path(&root).exists());
        assert_eq!(
            read_eagle_scene_section(&root, SECTION_LIGHTS)
                .unwrap()
                .unwrap()["lights"][0]["pointLobe"],
            "sides"
        );
        fs::remove_dir_all(root).unwrap();
    }
}

pub(crate) fn wip_root_path(root: &Path) -> PathBuf {
    root.join(".light_mapper_wip").join("latest")
}

pub(crate) fn autosave_root_path(root: &Path) -> PathBuf {
    root.join(".eagle_autosave").join("latest")
}

pub(crate) fn wip_light_list_path(root: &Path) -> PathBuf {
    wip_root_path(root).join("Light_List.xml")
}

pub(crate) fn autosave_light_list_path(root: &Path) -> PathBuf {
    autosave_root_path(root).join("Light_List.xml")
}

pub(crate) fn autosave_meta_path(root: &Path) -> PathBuf {
    autosave_root_path(root).join("autosave.txt")
}

pub(crate) fn wip_asset_deletes_path(root: &Path) -> PathBuf {
    wip_root_path(root).join("purged_assets.txt")
}

pub(crate) fn autosave_asset_deletes_path(root: &Path) -> PathBuf {
    autosave_root_path(root).join("purged_assets.txt")
}

pub(crate) fn load_asset_deletes_path(path: &Path) -> HashSet<String> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(lower)
        .collect()
}

pub(crate) fn load_wip_asset_deletes(root: &Path) -> HashSet<String> {
    load_asset_deletes_path(&wip_asset_deletes_path(root))
}

pub(crate) fn load_autosave_asset_deletes(root: &Path) -> HashSet<String> {
    load_asset_deletes_path(&autosave_asset_deletes_path(root))
}

pub(crate) fn save_wip_asset_deletes(root: &Path, deletes: &HashSet<String>) -> Result<(), String> {
    save_asset_deletes_path(&wip_asset_deletes_path(root), deletes)
}

pub(crate) fn save_autosave_asset_deletes(
    root: &Path,
    deletes: &HashSet<String>,
) -> Result<(), String> {
    save_asset_deletes_path(&autosave_asset_deletes_path(root), deletes)
}

pub(crate) fn save_asset_deletes_path(
    path: &Path,
    deletes: &HashSet<String>,
) -> Result<(), String> {
    if deletes.is_empty() {
        if path.exists() {
            fs::remove_file(path).map_err(|err| format!("{}: {err}", path.display()))?;
        }
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("{}: {err}", parent.display()))?;
    }
    let mut lines: Vec<_> = deletes.iter().cloned().collect();
    lines.sort();
    fs::write(&path, lines.join("\n") + "\n").map_err(|err| format!("{}: {err}", path.display()))
}

pub(crate) fn collect_img_files_from_dir(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(items) = fs::read_dir(dir) {
        for item in items.filter_map(Result::ok) {
            let path = item.path();
            if path
                .extension()
                .and_then(|s| s.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("img"))
            {
                out.push(path);
            }
        }
    }
}

pub(crate) fn collect_resource_img_files(root: &Path) -> Vec<PathBuf> {
    let mut img_files = Vec::new();
    collect_img_files_from_dir(&root.join("imgs"), &mut img_files);
    collect_img_files_from_dir(&wip_root_path(root).join("imgs"), &mut img_files);
    img_files
}

pub(crate) fn collect_scene_img_files(root: &Path, source: LoadSceneSource) -> Vec<PathBuf> {
    let mut img_files = Vec::new();
    collect_img_files_from_dir(&root.join("imgs"), &mut img_files);
    match source {
        LoadSceneSource::Saved => {
            collect_img_files_from_dir(&wip_root_path(root).join("imgs"), &mut img_files)
        }
        LoadSceneSource::Autosave => {
            collect_img_files_from_dir(&autosave_root_path(root).join("imgs"), &mut img_files)
        }
    }
    img_files
}

pub(crate) fn collect_txd_files_from_dir(dir: &Path, out: &mut Vec<PathBuf>) {
    if !dir.exists() {
        return;
    }
    for entry in WalkDir::new(dir).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        if entry
            .path()
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("txd"))
        {
            out.push(entry.path().to_path_buf());
        }
    }
}

pub(crate) fn collect_resource_txd_files(root: &Path) -> Vec<PathBuf> {
    collect_scene_txd_files(root, LoadSceneSource::Saved)
}

pub(crate) fn collect_scene_txd_files(root: &Path, source: LoadSceneSource) -> Vec<PathBuf> {
    let overlay_root = match source {
        LoadSceneSource::Saved => wip_root_path(root),
        LoadSceneSource::Autosave => autosave_root_path(root),
    };
    let mut txd_files = Vec::new();
    for base in [root.to_path_buf(), overlay_root] {
        let mut files_for_root = Vec::new();
        for dir in [
            "textures",
            "Textures",
            "txd_build",
            "TXD_Build",
            "models",
            "Models",
            "imgs",
            "Imgs",
        ] {
            collect_txd_files_from_dir(&base.join(dir), &mut files_for_root);
        }
        files_for_root.sort();
        files_for_root.dedup();
        txd_files.extend(files_for_root);
    }
    txd_files.dedup();
    txd_files
}

pub(crate) fn load_scene_source_with_source(
    root: &Path,
    attr_re: &Regex,
    source: LoadSceneSource,
) -> (
    Vec<String>,
    HashMap<String, Definition>,
    HashSet<String>,
    Vec<Placement>,
    Vec<EditorLight>,
    EagleZoneOffsets,
    bool,
) {
    let wip_root = wip_root_path(root);
    let autosave_root = autosave_root_path(root);
    let scene_root = match source {
        LoadSceneSource::Autosave => {
            if autosave_root.join("eagleZones.txt").is_file()
                && autosave_root.join("zones").is_dir()
            {
                &autosave_root
            } else {
                root
            }
        }
        LoadSceneSource::Saved => {
            if wip_root.join("eagleZones.txt").is_file() && wip_root.join("zones").is_dir() {
                &wip_root
            } else {
                root
            }
        }
    };
    let loaded_wip = scene_root == wip_root.as_path();
    let (zones, eagle_zone_offsets) = parse_eagle_zones(scene_root);
    let mut defs = parse_definitions(scene_root, &zones, attr_re);
    let gta_defs = load_gta_sa_definitions(&load_gta_sa_dir_preference());
    let readonly_definition_ids = merge_gta_sa_definitions(&mut defs, gta_defs);
    let placements = parse_placements(scene_root, &zones, &defs, attr_re);
    let light_path = match source {
        LoadSceneSource::Autosave if autosave_light_list_path(root).is_file() => {
            autosave_light_list_path(root)
        }
        _ if loaded_wip && wip_light_list_path(root).is_file() => wip_light_list_path(root),
        _ => light_list_path(root),
    };
    let lights =
        load_project_lights(scene_root, &light_path, attr_re).unwrap_or_else(|_| default_lights());
    (
        zones,
        defs,
        readonly_definition_ids,
        placements,
        lights,
        eagle_zone_offsets,
        loaded_wip,
    )
}

pub(crate) fn definition_is_gta_sa(definition: &Definition) -> bool {
    definition
        .attrs
        .get("source")
        .is_some_and(|source| source.trim().eq_ignore_ascii_case("GTA:SA"))
}

fn merge_gta_sa_definitions(
    defs: &mut HashMap<String, Definition>,
    gta_defs: HashMap<String, Definition>,
) -> HashSet<String> {
    let mut readonly_definition_ids = HashSet::new();
    for (id, gta_def) in gta_defs {
        if defs.get(&id).is_some_and(is_override_definition) {
            if let Some(override_def) = defs.get_mut(&id) {
                let explicit = override_def
                    .attrs
                    .get("__overrideAttrs")
                    .cloned()
                    .unwrap_or_default();
                let zone = override_def.zone.clone();
                let mut attrs = gta_def.attrs.clone();
                for key in explicit.split(',').filter(|key| !key.trim().is_empty()) {
                    if let Some(value) = override_def.attrs.get(key).cloned() {
                        attrs.insert(key.to_string(), value);
                    } else {
                        attrs.remove(key);
                    }
                }
                attrs.insert("id".to_string(), id.clone());
                attrs.insert("zone".to_string(), zone.clone());
                attrs.insert("__override".to_string(), "true".to_string());
                attrs.insert("__overrideAttrs".to_string(), explicit);
                override_def.zone = zone;
                override_def.attrs = attrs;
            }
        } else if !defs.contains_key(&id) {
            readonly_definition_ids.insert(id.clone());
            defs.insert(id, gta_def);
        }
    }
    readonly_definition_ids
}

pub(crate) fn light_from_attrs(attrs: &BTreeMap<String, String>, index: usize) -> EditorLight {
    EditorLight {
        name: attrs
            .get("name")
            .cloned()
            .unwrap_or_else(|| format!("Light {}", index + 1)),
        kind: parse_light_kind(
            attrs
                .get("kind")
                .or_else(|| attrs.get("type"))
                .map(String::as_str)
                .unwrap_or("point"),
        ),
        profile: parse_light_profile(attrs.get("profile").map(String::as_str).unwrap_or("both")),
        position: attr_vec3(attrs, "pos", V3::default()),
        direction: attr_vec3(
            attrs,
            "dir",
            V3 {
                x: 0.0,
                y: 0.0,
                z: -1.0,
            },
        ),
        color: attr_vec3(
            attrs,
            "color",
            V3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
        ),
        temperature: attrs
            .get("temperature")
            .or_else(|| attrs.get("kelvin"))
            .and_then(|value| value.parse::<f32>().ok())
            .unwrap_or(6500.0),
        use_temperature: attr_bool(
            attrs,
            &["useTemperature", "use_temperature", "temperatureColor"],
            false,
        ),
        intensity: attrs
            .get("intensity")
            .or_else(|| attrs.get("energy"))
            .and_then(|value| value.parse::<f32>().ok())
            .unwrap_or(1.0),
        radius: attrs
            .get("radius")
            .or_else(|| attrs.get("range"))
            .and_then(|value| value.parse::<f32>().ok())
            .unwrap_or(50.0),
        casts_shadow: attr_bool(attrs, &["castsShadow", "casts_shadow", "shadow"], false),
        point_lobe: parse_point_light_lobe(
            attrs
                .get("pointLobe")
                .or_else(|| attrs.get("point_lobe"))
                .map(String::as_str)
                .unwrap_or("omni"),
        ),
    }
}

fn editor_light_to_json(light: &EditorLight) -> serde_json::Value {
    serde_json::json!({
        "name": light.name,
        "kind": light_kind_label(light.kind),
        "profile": light_profile_label(light.profile),
        "position": [light.position.x, light.position.y, light.position.z],
        "direction": [light.direction.x, light.direction.y, light.direction.z],
        "color": [light.color.x, light.color.y, light.color.z],
        "temperature": light.temperature,
        "useTemperature": light.use_temperature,
        "intensity": light.intensity,
        "radius": light.radius,
        "castsShadow": light.casts_shadow,
        "pointLobe": point_light_lobe_label(light.point_lobe),
    })
}

fn finite_json_f32(value: Option<&serde_json::Value>) -> Option<f32> {
    let value = value?.as_f64()? as f32;
    value.is_finite().then_some(value)
}

fn editor_light_from_json(value: &serde_json::Value, index: usize) -> Option<EditorLight> {
    let value = value.as_object()?;
    Some(EditorLight {
        name: value
            .get("name")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| format!("Light {}", index + 1)),
        kind: parse_light_kind(
            value
                .get("kind")
                .or_else(|| value.get("type"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("point"),
        ),
        profile: parse_light_profile(
            value
                .get("profile")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("both"),
        ),
        position: value.get("position").and_then(json_v3).unwrap_or_default(),
        direction: value.get("direction").and_then(json_v3).unwrap_or(V3 {
            x: 0.0,
            y: 0.0,
            z: -1.0,
        }),
        color: value.get("color").and_then(json_v3).unwrap_or(V3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        }),
        temperature: finite_json_f32(value.get("temperature")).unwrap_or(6500.0),
        use_temperature: value
            .get("useTemperature")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        intensity: finite_json_f32(value.get("intensity")).unwrap_or(1.0),
        radius: finite_json_f32(value.get("radius")).unwrap_or(50.0),
        casts_shadow: value
            .get("castsShadow")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        point_lobe: parse_point_light_lobe(
            value
                .get("pointLobe")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("omni"),
        ),
    })
}

fn lights_from_json(json: &serde_json::Value) -> Vec<EditorLight> {
    json.get("lights")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(index, value)| editor_light_from_json(value, index))
        .collect()
}

pub(crate) fn lights_section_value(lights: &[EditorLight]) -> serde_json::Value {
    serde_json::json!({
        "version": 1,
        "lights": lights.iter().map(editor_light_to_json).collect::<Vec<_>>(),
    })
}

fn parse_lights_json_strict(json: &serde_json::Value) -> Result<Vec<EditorLight>, String> {
    validate_scene_sidecar_version(json, 1)?;
    let entries = json
        .get("lights")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "`lights` must be an array".to_string())?;
    let lights = lights_from_json(json);
    if lights.len() != entries.len() {
        return Err("one or more light entries are not objects".to_string());
    }
    Ok(lights)
}

/// Loads project-authored lights from EagleScene, falling back to the legacy
/// interchange XML only when the unified section is absent.
pub(crate) fn load_project_lights(
    root: &Path,
    legacy_path: &Path,
    attr_re: &Regex,
) -> Result<Vec<EditorLight>, String> {
    match read_eagle_scene_section(root, SECTION_LIGHTS)? {
        Some(json) => parse_lights_json_strict(&json)
            .map_err(|err| format!("Invalid EagleScene lights section: {err}")),
        None => load_lights_xml(legacy_path, attr_re),
    }
}

/// Persists project-authored lights without updating the legacy interchange
/// XML. `save_lights_xml` remains available for explicit import/export.
#[allow(dead_code)]
pub(crate) fn save_project_lights(root: &Path, lights: &[EditorLight]) -> Result<(), String> {
    update_eagle_scene_section(root, SECTION_LIGHTS, lights_section_value(lights))
}

pub(crate) fn load_lights_xml(path: &Path, attr_re: &Regex) -> Result<Vec<EditorLight>, String> {
    let text = fs::read_to_string(path).map_err(|err| format!("{}: {err}", path.display()))?;
    let tag_re = Regex::new(r#"(?i)<light\s+([^>]*)/?>"#).unwrap();
    let mut lights = Vec::new();
    for cap in tag_re.captures_iter(&text) {
        let attrs = parse_attrs(&cap[1], attr_re);
        lights.push(light_from_attrs(&attrs, lights.len()));
    }
    if lights.is_empty() {
        Err(format!("{} contained no <light> entries", path.display()))
    } else {
        Ok(lights)
    }
}

pub(crate) fn load_legacy_lights_section(
    root: &Path,
    attr_re: &Regex,
) -> Result<Option<serde_json::Value>, String> {
    let path = light_list_path(root);
    if !path.is_file() {
        return Ok(None);
    }
    let lights = load_lights_xml(&path, attr_re)
        .map_err(|err| format!("{} cannot be migrated safely: {err}", path.display()))?;
    Ok(Some(lights_section_value(&lights)))
}

/// Collects only the exact legacy scene documents owned by this module.
///
/// Every present document is parsed strictly before it is returned so the
/// migration transaction can safely verify and archive the original files.
pub(crate) fn collect_legacy_scene_section_imports(
    root: &Path,
    attr_re: &Regex,
) -> Result<Vec<LegacySceneImport>, String> {
    let mut imports = Vec::new();
    if let Some(value) = load_legacy_material_emitters_section(root)? {
        imports.push(LegacySceneImport::new("Light_Emitters.json", value)?);
    }
    if let Some(value) = load_legacy_shadow_casters_section(root)? {
        imports.push(LegacySceneImport::new("Shadow_Casters.json", value)?);
    }
    if let Some(value) = load_legacy_collision_capsules_section(root)? {
        imports.push(LegacySceneImport::new("Collision_Capsules.json", value)?);
    }
    if let Some(value) = load_legacy_collision_cuboids_section(root)? {
        imports.push(LegacySceneImport::new("Collision_Cuboids.json", value)?);
    }
    if let Some(value) = load_legacy_lights_section(root, attr_re)? {
        imports.push(LegacySceneImport::new("Light_List.xml", value)?);
    }
    Ok(imports)
}

#[allow(dead_code)]
pub(crate) fn save_lights_xml(path: &Path, lights: &[EditorLight]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("{}: {err}", parent.display()))?;
    }
    let mut out = String::from("<Light_List version=\"1\">\n");
    for light in lights {
        out.push_str(&format!(
            "    <light name=\"{}\" kind=\"{}\" profile=\"{}\" posX=\"{:.3}\" posY=\"{:.3}\" posZ=\"{:.3}\" dirX=\"{:.5}\" dirY=\"{:.5}\" dirZ=\"{:.5}\" colorX=\"{:.4}\" colorY=\"{:.4}\" colorZ=\"{:.4}\" temperature=\"{:.0}\" useTemperature=\"{}\" intensity=\"{:.4}\" radius=\"{:.3}\" castsShadow=\"{}\" pointLobe=\"{}\" />\n",
            xml_escape(&light.name),
            light_kind_label(light.kind),
            light_profile_label(light.profile),
            light.position.x,
            light.position.y,
            light.position.z,
            light.direction.x,
            light.direction.y,
            light.direction.z,
            light.color.x,
            light.color.y,
            light.color.z,
            light.temperature,
            if light.use_temperature { "true" } else { "false" },
            light.intensity,
            light.radius,
            if light.casts_shadow { "true" } else { "false" },
            point_light_lobe_label(light.point_lobe),
        ));
    }
    out.push_str("</Light_List>\n");
    fs::write(path, out).map_err(|err| format!("{}: {err}", path.display()))
}

fn parse_eagle_zone_offset(line: &str, directive: &str) -> Option<V3> {
    let values = line.strip_prefix(directive)?;
    if !values.starts_with(char::is_whitespace) {
        return None;
    }
    let mut values = values.trim().split(',').map(str::trim);
    let offset = V3 {
        x: parse_finite_f32_value(values.next()?)?,
        y: parse_finite_f32_value(values.next()?)?,
        z: parse_finite_f32_value(values.next()?)?,
    };
    values.next().is_none().then_some(offset)
}

/// Zone names are used as both a directory and a file stem. Keep them to one
/// plain path component so a malformed resource cannot make loading or saving
/// escape `<resource>/zones`.
pub(crate) fn is_safe_zone_name(zone: &str) -> bool {
    !zone.is_empty()
        && zone != "."
        && zone != ".."
        && !zone.contains(['/', '\\'])
        && !zone.chars().any(char::is_control)
}

pub(crate) fn parse_eagle_zones(root: &Path) -> (Vec<String>, EagleZoneOffsets) {
    let text = fs::read_to_string(root.join("eagleZones.txt")).unwrap_or_default();
    let mut zones = Vec::new();
    let mut offsets = EagleZoneOffsets::default();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if let Some(offset) = parse_eagle_zone_offset(line, "#offset") {
            offsets.offset = Some(offset);
        } else if let Some(offset) = parse_eagle_zone_offset(line, "#waterOffset") {
            offsets.water_offset = Some(offset);
        } else if !line.starts_with('#') && is_safe_zone_name(line) {
            zones.push(line.to_string());
        }
    }
    (zones, offsets)
}

pub(crate) fn parse_definitions(
    root: &Path,
    zones: &[String],
    attr_re: &Regex,
) -> HashMap<String, Definition> {
    let tag_re = Regex::new(r#"(?i)<(definition|override)\s+([^>]*)/?>"#).unwrap();
    let mut defs = HashMap::new();
    for zone in zones {
        if !is_safe_zone_name(zone) {
            continue;
        }
        let text = fs::read_to_string(
            root.join("zones")
                .join(zone)
                .join(format!("{zone}.definition")),
        )
        .unwrap_or_default();
        for cap in tag_re.captures_iter(&text) {
            let tag = cap[1].to_ascii_lowercase();
            let mut attrs = parse_attrs(&cap[2], attr_re);
            if let Some(id) = attrs.get("id").cloned() {
                if tag == "override" {
                    let explicit_attrs = attrs
                        .keys()
                        .filter(|key| key.as_str() != "id" && key.as_str() != "zone")
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(",");
                    attrs.insert("__override".to_string(), "true".to_string());
                    attrs.insert("__overrideAttrs".to_string(), explicit_attrs);
                }
                attrs
                    .entry("zone".to_string())
                    .or_insert_with(|| zone.clone());
                defs.insert(
                    id.clone(),
                    Definition {
                        id,
                        zone: attrs.get("zone").cloned().unwrap_or_else(|| zone.clone()),
                        attrs,
                    },
                );
            }
        }
    }
    defs
}

pub(crate) fn is_override_definition(def: &Definition) -> bool {
    def.attrs
        .get("__override")
        .is_some_and(|value| value == "true")
}

pub(crate) fn parse_ide_section_name(line: &str) -> Option<&str> {
    let name = line.trim().to_ascii_lowercase();
    match name.as_str() {
        "objs" | "tobj" => Some(if name == "objs" { "objs" } else { "tobj" }),
        _ => None,
    }
}

pub(crate) fn parse_gta_sa_ide_line(line: &str, section: &str) -> Option<Definition> {
    let cleaned = line.split('#').next().unwrap_or("").trim();
    if cleaned.is_empty() {
        return None;
    }
    let parts: Vec<_> = cleaned
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    if parts.len() < 3 {
        return None;
    }
    let id = parts[0].parse::<i32>().ok()?.to_string();
    let model = parts[1].to_string();
    let txd = parts[2].to_string();
    let mut attrs = BTreeMap::new();
    attrs.insert("id".to_string(), id.clone());
    attrs.insert("dff".to_string(), model.clone());
    attrs.insert("txd".to_string(), txd);
    attrs.insert("source".to_string(), "GTA:SA".to_string());
    attrs.insert("zone".to_string(), "GTA:SA".to_string());
    if section == "tobj" {
        if parts.len() >= 7 {
            attrs.insert("timeIn".to_string(), parts[3].to_string());
            attrs.insert("timeOut".to_string(), parts[4].to_string());
            attrs.insert("drawDistance".to_string(), parts[5].to_string());
            attrs.insert("gtaFlags".to_string(), parts[6].to_string());
        }
    } else if parts.len() >= 5 {
        attrs.insert("drawDistance".to_string(), parts[3].to_string());
        attrs.insert("gtaFlags".to_string(), parts[4].to_string());
    }
    Some(Definition {
        id,
        zone: "GTA:SA".to_string(),
        attrs,
    })
}

pub(crate) fn load_gta_sa_definitions(gta_sa_dir: &Path) -> HashMap<String, Definition> {
    let maps_dir = gta_sa_dir.join("data").join("maps");
    if !maps_dir.is_dir() {
        return HashMap::new();
    }
    let mut defs = HashMap::new();
    for entry in WalkDir::new(maps_dir).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_file()
            || !path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("ide"))
        {
            continue;
        }
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        let mut section: Option<&str> = None;
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.eq_ignore_ascii_case("end") {
                section = None;
                continue;
            }
            if let Some(next) = parse_ide_section_name(trimmed) {
                section = Some(next);
                continue;
            }
            if let Some(section) = section {
                if let Some(def) = parse_gta_sa_ide_line(trimmed, section) {
                    let model_alias = def.attrs.get("dff").cloned().filter(|model| {
                        !model.trim().is_empty() && !model.eq_ignore_ascii_case(&def.id)
                    });
                    defs.entry(def.id.clone()).or_insert_with(|| def.clone());
                    if let Some(model) = model_alias {
                        let mut alias = def;
                        alias.id = model.clone();
                        alias.attrs.insert("id".to_string(), model.clone());
                        defs.entry(model).or_insert(alias);
                    }
                }
            }
        }
    }
    defs
}

pub(crate) fn parse_vehicle_ide_line(
    line: &str,
    source: &str,
    readonly: bool,
) -> Option<VehicleAsset> {
    let trimmed = line
        .split('#')
        .next()
        .unwrap_or(line)
        .split("//")
        .next()
        .unwrap_or(line)
        .split(';')
        .next()
        .unwrap_or(line)
        .trim();
    if trimmed.is_empty() || trimmed.starts_with("//") {
        return None;
    }
    let parts: Vec<_> = trimmed
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    if parts.len() < 3 {
        return None;
    }
    let id = parts.get(1)?.to_string();
    let model_id = parts.first().and_then(|value| value.parse::<u32>().ok());
    let handling_id = parts
        .get(4)
        .map(|value| value.to_string())
        .filter(|value| !value.is_empty());
    let txd = parts
        .get(2)
        .map(|value| (*value).to_string())
        .unwrap_or_default();
    Some(VehicleAsset {
        id: id.clone(),
        model_id,
        handling_id,
        handling: HashMap::new(),
        dff: id.clone(),
        txd,
        col: id.clone(),
        wheel_front: parts.get(12).and_then(|value| value.parse::<f32>().ok()),
        wheel_rear: parts.get(13).and_then(|value| value.parse::<f32>().ok()),
        source: source.to_string(),
        readonly,
        loose_dff_path: None,
        loose_txd_path: None,
    })
}

pub(crate) fn load_vehicle_ide_file(
    path: &Path,
    source: &str,
    readonly: bool,
) -> Vec<VehicleAsset> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut section: Option<&str> = None;
    let mut vehicles = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case("end") {
            section = None;
            continue;
        }
        let lower_section = trimmed.to_ascii_lowercase();
        if matches!(
            lower_section.as_str(),
            "cars" | "boats" | "trains" | "planes" | "heli" | "helis" | "bikes"
        ) {
            section = Some(match lower_section.as_str() {
                "cars" => "cars",
                "boats" => "boats",
                "trains" => "trains",
                "planes" => "planes",
                "heli" | "helis" => "helis",
                _ => "bikes",
            });
            continue;
        }
        if section.is_some() {
            if let Some(vehicle) = parse_vehicle_ide_line(trimmed, source, readonly) {
                vehicles.push(vehicle);
            }
        }
    }
    vehicles
}

fn parse_physics_root_object_dat(text: &str) -> HashMap<u16, PhysicsRootProperties> {
    let roots_by_name: HashMap<String, u16> = PHYSICS_ROOT_SPECS
        .iter()
        .map(|spec| (lower(spec.object_name), spec.model_id))
        .collect();
    let mut result = HashMap::new();

    for raw_line in text.lines() {
        let line = raw_line.split(';').next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        // The shipped file is only comma-ish: several rows omit commas
        // between numeric fields, so accept either commas or whitespace.
        let fields: Vec<&str> = line
            .split(|ch: char| ch == ',' || ch.is_whitespace())
            .filter(|field| !field.is_empty())
            .collect();
        if fields.len() < 6 {
            continue;
        }
        let Some(model_id) = roots_by_name.get(&lower(fields[0])).copied() else {
            continue;
        };
        let Ok(mass) = fields[1].parse::<f32>() else {
            continue;
        };
        let Ok(turn_mass) = fields[2].parse::<f32>() else {
            continue;
        };
        let Ok(air_resistance) = fields[3].parse::<f32>() else {
            continue;
        };
        let Ok(elasticity) = fields[4].parse::<f32>() else {
            continue;
        };
        let Ok(buoyancy) = fields[5].parse::<f32>() else {
            continue;
        };
        let values = [mass, turn_mass, air_resistance, elasticity, buoyancy];
        if values.iter().all(|value| value.is_finite()) {
            let mut properties =
                PhysicsRootProperties::new(mass, turn_mass, air_resistance, elasticity, buoyancy);
            if fields.len() >= 17
                && let (
                    Ok(uproot_limit),
                    Ok(collision_damage_multiplier),
                    Ok(collision_damage_effect),
                    Ok(special_collision_response),
                    Ok(camera_avoid),
                    Ok(causes_explosion),
                    Ok(fx_type),
                    Ok(fx_x),
                    Ok(fx_y),
                    Ok(fx_z),
                ) = (
                    fields[6].parse::<f32>(),
                    fields[7].parse::<f32>(),
                    fields[8].parse::<i32>(),
                    fields[9].parse::<i32>(),
                    fields[10].parse::<i32>(),
                    fields[11].parse::<i32>(),
                    fields[12].parse::<i32>(),
                    fields[13].parse::<f32>(),
                    fields[14].parse::<f32>(),
                    fields[15].parse::<f32>(),
                )
                && [uproot_limit, collision_damage_multiplier, fx_x, fx_y, fx_z]
                    .iter()
                    .all(|value| value.is_finite())
            {
                properties.uproot_limit = uproot_limit;
                properties.collision_damage_multiplier = collision_damage_multiplier;
                properties.collision_damage_effect = collision_damage_effect;
                properties.special_collision_response = special_collision_response;
                properties.camera_avoid = camera_avoid != 0;
                properties.causes_explosion = causes_explosion != 0;
                properties.fx_type = fx_type;
                properties.fx_offset = V3 {
                    x: fx_x,
                    y: fx_y,
                    z: fx_z,
                };
                properties.fx_name = fields[16].to_string();
            }
            if fields.len() >= 24 {
                let parsed = (
                    fields[6].parse::<f32>(),
                    fields[7].parse::<f32>(),
                    fields[8].parse::<i32>(),
                    fields[9].parse::<i32>(),
                    fields[10].parse::<i32>(),
                    fields[11].parse::<i32>(),
                    fields[12].parse::<i32>(),
                    fields[13].parse::<f32>(),
                    fields[14].parse::<f32>(),
                    fields[15].parse::<f32>(),
                    fields[17].parse::<f32>(),
                    fields[18].parse::<f32>(),
                    fields[19].parse::<f32>(),
                    fields[20].parse::<f32>(),
                    fields[21].parse::<f32>(),
                    fields[22].parse::<i32>(),
                    fields[23].parse::<i32>(),
                );
                if let (
                    Ok(uproot_limit),
                    Ok(collision_damage_multiplier),
                    Ok(collision_damage_effect),
                    Ok(special_collision_response),
                    Ok(camera_avoid),
                    Ok(causes_explosion),
                    Ok(fx_type),
                    Ok(fx_x),
                    Ok(fx_y),
                    Ok(fx_z),
                    Ok(smash_multiplier),
                    Ok(break_x),
                    Ok(break_y),
                    Ok(break_z),
                    Ok(break_velocity_randomness),
                    Ok(gun_break_mode),
                    Ok(sparks_on_impact),
                ) = parsed
                {
                    let extended_values = [
                        uproot_limit,
                        collision_damage_multiplier,
                        fx_x,
                        fx_y,
                        fx_z,
                        smash_multiplier,
                        break_x,
                        break_y,
                        break_z,
                        break_velocity_randomness,
                    ];
                    if extended_values.iter().all(|value| value.is_finite()) {
                        properties.uproot_limit = uproot_limit;
                        properties.collision_damage_multiplier = collision_damage_multiplier;
                        properties.collision_damage_effect = collision_damage_effect;
                        properties.special_collision_response = special_collision_response;
                        properties.camera_avoid = camera_avoid != 0;
                        properties.causes_explosion = causes_explosion != 0;
                        properties.fx_type = fx_type;
                        properties.fx_offset = V3 {
                            x: fx_x,
                            y: fx_y,
                            z: fx_z,
                        };
                        properties.fx_name = fields[16].to_string();
                        properties.smash_multiplier = smash_multiplier;
                        properties.break_velocity = V3 {
                            x: break_x,
                            y: break_y,
                            z: break_z,
                        };
                        properties.break_velocity_randomness = break_velocity_randomness;
                        properties.gun_break_mode = gun_break_mode;
                        properties.sparks_on_impact = sparks_on_impact != 0;
                    }
                }
            }
            result.insert(model_id, properties);
        }
    }

    result
}

pub(crate) fn load_physics_root_properties(
    gta_sa_dir: &Path,
) -> HashMap<u16, PhysicsRootProperties> {
    let mut result: HashMap<u16, PhysicsRootProperties> = PHYSICS_ROOT_SPECS
        .iter()
        .map(|spec| (spec.model_id, spec.fallback.clone()))
        .collect();
    let path = gta_sa_dir.join("data").join("object.dat");
    if let Ok(text) = fs::read_to_string(path) {
        result.extend(parse_physics_root_object_dat(&text));
    }
    result
}

const HANDLING_CFG_COLUMNS: &[(&str, usize)] = &[
    ("mass", 1),
    ("turnMass", 2),
    ("dragCoeff", 3),
    ("tractionMultiplier", 8),
    ("tractionLoss", 9),
    ("tractionBias", 10),
    ("numberOfGears", 11),
    ("maxVelocity", 12),
    ("engineAcceleration", 13),
    ("engineInertia", 14),
    ("driveType", 15),
    ("engineType", 16),
    ("brakeDeceleration", 17),
    ("brakeBias", 18),
    ("steeringLock", 20),
    ("suspensionForceLevel", 21),
    ("suspensionDamping", 22),
    ("suspensionUpperLimit", 24),
    ("suspensionLowerLimit", 25),
    ("suspensionFrontRearBias", 26),
    ("collisionDamageMultiplier", 29),
];

pub(crate) fn parse_vehicle_handling_cfg(text: &str) -> HashMap<String, HashMap<String, String>> {
    let mut result = HashMap::new();
    for line in text.lines() {
        let trimmed = line
            .split(';')
            .next()
            .unwrap_or(line)
            .split('#')
            .next()
            .unwrap_or(line)
            .trim();
        if trimmed.is_empty()
            || matches!(
                trimmed.as_bytes().first(),
                Some(b'!') | Some(b'$') | Some(b'%') | Some(b'^')
            )
        {
            continue;
        }
        let columns = trimmed.split_whitespace().collect::<Vec<_>>();
        if columns.len() <= 29 {
            continue;
        }
        let values = HANDLING_CFG_COLUMNS
            .iter()
            .filter_map(|(property, column)| {
                columns
                    .get(*column)
                    .map(|value| ((*property).to_string(), (*value).to_string()))
            })
            .collect::<HashMap<_, _>>();
        result.insert(columns[0].to_ascii_lowercase(), values);
    }
    result
}

pub(crate) fn loose_vehicle_assets(root: &Path) -> Vec<VehicleAsset> {
    let mut vehicles = Vec::new();
    for dir in [
        root.join("vehicles"),
        root.join("vehicle"),
        root.join("custom_vehicles"),
        root.join("models").join("vehicles"),
    ] {
        if !dir.is_dir() {
            continue;
        }
        for entry in WalkDir::new(&dir).into_iter().filter_map(Result::ok) {
            let path = entry.path();
            if !path.is_file()
                || !path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("dff"))
            {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
                continue;
            };
            vehicles.push(VehicleAsset {
                id: stem.to_string(),
                model_id: None,
                handling_id: None,
                handling: HashMap::new(),
                dff: stem.to_string(),
                txd: stem.to_string(),
                col: stem.to_string(),
                wheel_front: None,
                wheel_rear: None,
                source: "custom loose".to_string(),
                readonly: false,
                loose_dff_path: Some(path.to_path_buf()),
                loose_txd_path: None,
            });
        }
    }
    vehicles
}

/// Scan a custom vehicle dictionary recursively. A vehicle is added only when
/// its DFF has a same-named TXD in the same directory. This includes pairs in
/// the selected dictionary root as well as pairs in any nested vehicle folder.
pub(crate) fn custom_vehicle_dictionary_assets(dir: &Path) -> Vec<VehicleAsset> {
    let mut vehicles = Vec::new();
    if !dir.is_dir() {
        return vehicles;
    }
    let mut txds = HashMap::<(PathBuf, String), PathBuf>::new();
    let entries = WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();
    for path in &entries {
        if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("txd"))
            && let (Some(parent), Some(stem)) = (
                path.parent(),
                path.file_stem().and_then(|value| value.to_str()),
            )
        {
            txds.entry((parent.to_path_buf(), lower(stem)))
                .or_insert_with(|| path.clone());
        }
    }
    for path in entries {
        if !path.is_file()
            || !path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("dff"))
        {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        let Some(parent) = path.parent() else {
            continue;
        };
        let Some(txd_path) = txds.get(&(parent.to_path_buf(), lower(stem))) else {
            continue;
        };
        vehicles.push(VehicleAsset {
            id: stem.to_string(),
            model_id: None,
            handling_id: None,
            handling: HashMap::new(),
            dff: stem.to_string(),
            txd: stem.to_string(),
            col: stem.to_string(),
            wheel_front: None,
            wheel_rear: None,
            source: "custom dictionary".to_string(),
            readonly: false,
            loose_dff_path: Some(path),
            loose_txd_path: Some(txd_path.clone()),
        });
    }
    vehicles
}

pub(crate) fn load_vehicle_assets(
    root: &Path,
    gta_sa_dir: &Path,
    custom_dictionaries: &[PathBuf],
) -> Vec<VehicleAsset> {
    let handling = fs::read_to_string(gta_sa_dir.join("data").join("handling.cfg"))
        .map(|text| parse_vehicle_handling_cfg(&text))
        .unwrap_or_default();
    let mut by_id = BTreeMap::<String, VehicleAsset>::new();
    for name in ["vehicles.ide", "default.ide"] {
        let path = gta_sa_dir.join("data").join(name);
        for vehicle in load_vehicle_ide_file(&path, "GTA:SA", true) {
            by_id.entry(lower(&vehicle.id)).or_insert(vehicle);
        }
    }
    for path in [
        root.join("vehicles.ide"),
        root.join("data").join("vehicles.ide"),
        root.join("data").join("default.ide"),
    ] {
        for vehicle in load_vehicle_ide_file(&path, "custom IDE", false) {
            by_id.insert(lower(&vehicle.id), vehicle);
        }
    }
    for vehicle in loose_vehicle_assets(root) {
        by_id.insert(lower(&vehicle.id), vehicle);
    }
    for dir in custom_dictionaries {
        for vehicle in custom_vehicle_dictionary_assets(dir) {
            by_id.insert(lower(&vehicle.id), vehicle);
        }
    }
    by_id
        .into_values()
        .map(|mut vehicle| {
            if let Some(values) = vehicle
                .handling_id
                .as_ref()
                .and_then(|id| handling.get(&id.to_ascii_lowercase()))
            {
                vehicle.handling = values.clone();
            }
            vehicle
        })
        .collect()
}

pub(crate) fn parse_placements(
    root: &Path,
    zones: &[String],
    defs: &HashMap<String, Definition>,
    attr_re: &Regex,
) -> Vec<Placement> {
    let tag_re = Regex::new(r#"(?i)<(building|object|scenery)\s+([^>]*)>"#).unwrap();
    let mut placements = Vec::new();
    for zone in zones {
        if !is_safe_zone_name(zone) {
            continue;
        }
        let text = fs::read_to_string(root.join("zones").join(zone).join(format!("{zone}.map")))
            .unwrap_or_default();
        for cap in tag_re.captures_iter(&text) {
            let attrs = parse_attrs(&cap[2], attr_re);
            let Some(id) = attrs.get("id") else { continue };
            let dff = defs
                .get(id)
                .and_then(|def| def.attrs.get("dff"))
                .cloned()
                .unwrap_or_else(|| id.clone());
            placements.push(Placement {
                id: id.clone(),
                dff,
                zone: zone.clone(),
                tag: cap[1].to_ascii_lowercase(),
                attrs: attrs.clone(),
                pos: V3 {
                    x: attrf(&attrs, "posX"),
                    y: attrf(&attrs, "posY"),
                    z: attrf(&attrs, "posZ"),
                },
                rot: V3 {
                    x: attrf(&attrs, "rotX"),
                    y: attrf(&attrs, "rotY"),
                    z: attrf(&attrs, "rotZ"),
                },
            });
        }
    }
    placements
}

fn parse_img_directory(reader: &mut impl Read, path: &Path) -> Vec<ImgEntry> {
    // Even the largest normal GTA archives are far below this. Reject an
    // implausible untrusted count instead of allowing a corrupt header to
    // drive an effectively unbounded directory scan.
    const MAX_IMG_DIRECTORY_ENTRIES: usize = 1_000_000;

    let mut header = [0u8; 8];
    if reader.read_exact(&mut header).is_err() || &header[0..4] != b"VER2" {
        return Vec::new();
    }
    let count = rd32(&header, 4) as usize;
    if count > MAX_IMG_DIRECTORY_ENTRIES {
        return Vec::new();
    }
    let mut out = Vec::new();
    for _ in 0..count {
        let mut entry = [0u8; 32];
        if reader.read_exact(&mut entry).is_err() {
            break;
        }
        let off_sector = rd32(&entry, 0);
        let streaming = rd16(&entry, 4) as u32;
        let archive = rd16(&entry, 6) as u32;
        let raw_name = &entry[8..32];
        let end = raw_name
            .iter()
            .position(|b| *b == 0)
            .unwrap_or(raw_name.len());
        let name = String::from_utf8_lossy(&raw_name[..end]).to_string();
        let sectors = if streaming != 0 { streaming } else { archive };
        let Some(offset) = off_sector.checked_mul(2048) else {
            continue;
        };
        let Some(size) = sectors.checked_mul(2048) else {
            continue;
        };
        if !name.is_empty() && size != 0 {
            out.push(ImgEntry {
                img_path: path.to_path_buf(),
                name,
                offset,
                size,
            });
        }
    }
    out
}

pub(crate) fn parse_img(path: &Path) -> Vec<ImgEntry> {
    let Ok(mut file) = fs::File::open(path) else {
        return Vec::new();
    };
    let file_len = file.metadata().ok().map(|metadata| metadata.len());
    let mut entries = parse_img_directory(&mut file, path);
    if let Some(file_len) = file_len {
        entries.retain(|entry| {
            (entry.offset as u64)
                .checked_add(entry.size as u64)
                .is_some_and(|end| end <= file_len)
        });
    }
    entries
}

pub(crate) fn gta_sa_img_files(gta_sa_dir: &Path) -> Vec<PathBuf> {
    let models_dir = gta_sa_dir.join("models");
    let mut out = Vec::new();
    if let Ok(items) = fs::read_dir(models_dir) {
        for item in items.filter_map(Result::ok) {
            let path = item.path();
            if path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("img"))
            {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

pub(crate) fn load_gta_sa_particle_effects(gta_sa_dir: &Path) -> Vec<ParticleEffectDef> {
    let path = gta_sa_dir.join("models").join("effects.fxp");
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    parse_gta_sa_effects_fxp(&text)
}

pub(crate) fn parse_gta_sa_effects_fxp(text: &str) -> Vec<ParticleEffectDef> {
    let mut effects = Vec::new();
    let mut current: Option<ParticleEffectDef> = None;
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if line == "FX_SYSTEM_DATA:" {
            if let Some(effect) = current.take().filter(|effect| !effect.name.is_empty()) {
                effects.push(effect);
            }
            current = Some(ParticleEffectDef::default());
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let Some(effect) = current.as_mut() else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "NAME" if effect.name.is_empty() => effect.name = value.to_string(),
            "LENGTH" if effect.length == 0.0 => {
                effect.length = value.parse().unwrap_or(0.0);
            }
            "PLAYMODE" => effect.play_mode = value.parse().unwrap_or(0),
            "CULLDIST" => effect.cull_distance = value.parse().unwrap_or(0.0),
            "NUM_PRIMS" => effect.primitive_count = value.parse().unwrap_or(0),
            "TEXTURE" | "TEXTURE2" | "TEXTURE3" | "TEXTURE4" => {
                if !value.is_empty()
                    && !value.eq_ignore_ascii_case("NULL")
                    && !effect
                        .textures
                        .iter()
                        .any(|texture| texture.eq_ignore_ascii_case(value))
                {
                    effect.textures.push(value.to_string());
                }
            }
            _ => {}
        }
    }
    if let Some(effect) = current.take().filter(|effect| !effect.name.is_empty()) {
        effects.push(effect);
    }
    effects
}

pub(crate) fn read_img_entry_from(file: &mut fs::File, entry: &ImgEntry) -> Vec<u8> {
    let mut out = vec![0; entry.size as usize];
    if file.seek(SeekFrom::Start(entry.offset as u64)).is_err()
        || file.read_exact(&mut out).is_err()
    {
        return Vec::new();
    }
    out
}

pub(crate) fn read_img_entry(entry: &ImgEntry) -> Vec<u8> {
    let Ok(mut file) = fs::File::open(&entry.img_path) else {
        return Vec::new();
    };
    read_img_entry_from(&mut file, entry)
}

pub(crate) fn loose_txd_entry(path: &Path) -> Option<ImgEntry> {
    let size = fs::metadata(path).ok()?.len().min(u32::MAX as u64) as u32;
    Some(ImgEntry {
        img_path: path.to_path_buf(),
        name: path.file_name()?.to_str()?.to_string(),
        offset: 0,
        size,
    })
}

pub(crate) fn rw_single_chunk_len(bytes: &[u8], expected_id: u32) -> usize {
    if bytes.len() < 12 || rd32(bytes, 0) != expected_id {
        return bytes.len();
    }
    let len = 12usize.saturating_add(rd32(bytes, 4) as usize);
    len.min(bytes.len())
}

pub(crate) fn txd_chunk_len(bytes: &[u8]) -> usize {
    rw_single_chunk_len(bytes, 0x16)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_sa_definition(id: &str) -> Definition {
        Definition {
            id: id.to_string(),
            zone: "GTA:SA".to_string(),
            attrs: BTreeMap::from([
                ("id".to_string(), id.to_string()),
                ("dff".to_string(), id.to_string()),
                ("drawDistance".to_string(), "85".to_string()),
                ("source".to_string(), "GTA:SA".to_string()),
                ("zone".to_string(), "GTA:SA".to_string()),
            ]),
        }
    }

    fn test_override(id: &str, explicit: &str, attrs: &[(&str, &str)]) -> Definition {
        let mut definition_attrs = BTreeMap::from([
            ("id".to_string(), id.to_string()),
            ("zone".to_string(), "haiti".to_string()),
            ("__override".to_string(), "true".to_string()),
            ("__overrideAttrs".to_string(), explicit.to_string()),
        ]);
        for (key, value) in attrs {
            definition_attrs.insert((*key).to_string(), (*value).to_string());
        }
        Definition {
            id: id.to_string(),
            zone: "haiti".to_string(),
            attrs: definition_attrs,
        }
    }

    #[test]
    fn sa_lod_only_override_remains_editable() {
        let id = "MTraffic1";
        let mut defs = HashMap::from([(
            id.to_string(),
            test_override(id, "lodDistance", &[("lodDistance", "170")]),
        )]);
        let gta_defs = HashMap::from([(id.to_string(), test_sa_definition(id))]);

        let readonly = merge_gta_sa_definitions(&mut defs, gta_defs);

        let definition = &defs[id];
        assert!(!readonly.contains(id));
        assert!(definition_is_gta_sa(definition));
        assert!(is_override_definition(definition));
        assert_eq!(
            definition.attrs.get("lodDistance").map(String::as_str),
            Some("170")
        );
        assert_eq!(
            definition.attrs.get("__overrideAttrs").map(String::as_str),
            Some("lodDistance")
        );
    }

    #[test]
    fn sa_mixed_override_preserves_lod_distance() {
        let id = "MTraffic1";
        let mut defs = HashMap::from([(
            id.to_string(),
            test_override(
                id,
                "flags,lodDistance",
                &[("flags", "7"), ("lodDistance", "170")],
            ),
        )]);
        let gta_defs = HashMap::from([(id.to_string(), test_sa_definition(id))]);

        let readonly = merge_gta_sa_definitions(&mut defs, gta_defs);

        let definition = &defs[id];
        assert!(!readonly.contains(id));
        assert!(definition_is_gta_sa(definition));
        assert!(is_override_definition(definition));
        assert_eq!(
            definition.attrs.get("__overrideAttrs").map(String::as_str),
            Some("flags,lodDistance")
        );
        assert_eq!(definition.attrs.get("flags").map(String::as_str), Some("7"));
        assert_eq!(
            definition.attrs.get("lodDistance").map(String::as_str),
            Some("170")
        );
    }

    #[test]
    fn xml_unescape_decodes_entities_exactly_once() {
        assert_eq!(xml_unescape("plain &amp; value"), "plain & value");
        assert_eq!(
            xml_unescape("literal &amp;amp; value"),
            "literal &amp; value"
        );
        assert_eq!(
            xml_unescape("&amp;quot;quoted&amp;quot;"),
            "&quot;quoted&quot;"
        );
    }

    #[test]
    fn eagle_zones_parser_reads_optional_project_offsets() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = env::temp_dir().join(format!(
            "eagle_zone_offsets_{}_{}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("eagleZones.txt"),
            "#offset 1.25, -2, 3\n# ignored\nzone_a\n#waterOffset 4,5.5, 6\nzone_b\n",
        )
        .unwrap();

        let (zones, offsets) = parse_eagle_zones(&root);

        assert_eq!(zones, vec!["zone_a", "zone_b"]);
        assert_eq!(
            offsets.offset,
            Some(V3 {
                x: 1.25,
                y: -2.0,
                z: 3.0,
            })
        );
        assert_eq!(
            offsets.water_offset,
            Some(V3 {
                x: 4.0,
                y: 5.5,
                z: 6.0,
            })
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn eagle_zones_parser_rejects_non_finite_offsets() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = env::temp_dir().join(format!(
            "eagle_zone_non_finite_offsets_{}_{}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("eagleZones.txt"),
            "#offset NaN,2,3\n#waterOffset 1,inf,3\nzone_a\n",
        )
        .unwrap();

        let (zones, offsets) = parse_eagle_zones(&root);

        assert_eq!(zones, vec!["zone_a"]);
        assert_eq!(offsets, EagleZoneOffsets::default());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn eagle_zones_parser_rejects_path_components() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = env::temp_dir().join(format!(
            "eagle_zone_paths_{}_{}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("eagleZones.txt"),
            "valid\n../escape\nnested/zone\nnested\\zone\n.\n..\n",
        )
        .unwrap();

        let (zones, _) = parse_eagle_zones(&root);

        assert_eq!(zones, vec!["valid"]);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn custom_vehicle_dictionary_matches_root_and_nested_dff_txd_pairs() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = env::temp_dir().join(format!(
            "eagle_custom_vehicle_dictionary_{}_{}",
            std::process::id(),
            unique
        ));
        let nested = root.join("police_car");
        let root_txd = root.join("ROOTCAR.TXD");
        let nested_txd = nested.join("copcarsf.txd");
        fs::create_dir_all(&nested).unwrap();
        fs::write(root.join("rootcar.dff"), []).unwrap();
        fs::write(&root_txd, []).unwrap();
        fs::write(nested.join("CopCarSF.DFF"), []).unwrap();
        fs::write(&nested_txd, []).unwrap();
        fs::write(nested.join("unpaired.dff"), []).unwrap();

        let vehicles = custom_vehicle_dictionary_assets(&root);
        let by_id = vehicles
            .into_iter()
            .map(|vehicle| (lower(&vehicle.id), vehicle))
            .collect::<BTreeMap<_, _>>();

        assert_eq!(by_id.len(), 2);
        assert_eq!(
            by_id["rootcar"].loose_txd_path.as_deref(),
            Some(root_txd.as_path())
        );
        assert_eq!(
            by_id["copcarsf"].loose_txd_path.as_deref(),
            Some(nested_txd.as_path())
        );
        assert!(!by_id.contains_key("unpaired"));

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn img_parser_reads_only_the_directory_table() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"VER2");
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&2u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        let mut name = [0u8; 24];
        name[.."test.dff".len()].copy_from_slice(b"test.dff");
        bytes.extend_from_slice(&name);
        bytes.resize(1024 * 1024, 0x5a);
        let mut reader = Cursor::new(bytes);

        let entries = parse_img_directory(&mut reader, Path::new("large.img"));

        assert_eq!(reader.position(), 40);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "test.dff");
        assert_eq!(entries[0].offset, 4096);
        assert_eq!(entries[0].size, 2048);
    }

    #[test]
    fn img_parser_skips_sector_offsets_that_do_not_fit_the_entry_type() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"VER2");
        bytes.extend_from_slice(&2u32.to_le_bytes());
        for (sector, name) in [
            (u32::MAX, b"bad.dff".as_slice()),
            (2, b"good.dff".as_slice()),
        ] {
            bytes.extend_from_slice(&sector.to_le_bytes());
            bytes.extend_from_slice(&1u16.to_le_bytes());
            bytes.extend_from_slice(&0u16.to_le_bytes());
            let mut entry_name = [0u8; 24];
            entry_name[..name.len()].copy_from_slice(name);
            bytes.extend_from_slice(&entry_name);
        }

        let entries = parse_img_directory(&mut Cursor::new(bytes), Path::new("overflow.img"));

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "good.dff");
        assert_eq!(entries[0].offset, 4096);
    }

    #[test]
    fn img_parser_rejects_implausible_directory_counts() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"VER2");
        bytes.extend_from_slice(&1_000_001u32.to_le_bytes());
        let mut reader = Cursor::new(bytes);

        assert!(parse_img_directory(&mut reader, Path::new("count.img")).is_empty());
        assert_eq!(reader.position(), 8);
    }

    #[test]
    fn pc_timecyc_postfx_uses_argb_columns_and_doubled_byte_alpha() {
        let mut values = vec![0.0; 48];
        values[40..44].copy_from_slice(&[255.0, 124.0, 64.0, 32.0]);

        let pass = timecyc_postfx_rgba(&values, 40);

        assert!((pass[0] - 124.0 / 255.0).abs() < 0.0001);
        assert!((pass[1] - 64.0 / 255.0).abs() < 0.0001);
        assert!((pass[2] - 32.0 / 255.0).abs() < 0.0001);
        assert!((pass[3] - 254.0 / 255.0).abs() < 0.0001);
    }

    pub(crate) fn test_definition_with_flags(flags: &str) -> Definition {
        let mut attrs = BTreeMap::new();
        attrs.insert("flags".to_string(), flags.to_string());
        Definition {
            id: "treepatchcomtop1".to_string(),
            zone: "test".to_string(),
            attrs,
        }
    }

    #[test]
    pub(crate) fn eagle_loader_bit_flags_enable_named_flags() {
        let def = test_definition_with_flags("2,21");

        assert!(definition_flag_enabled(&def, "draw_last"));
        assert!(definition_flag_enabled(&def, "disable_backface_culling"));
        assert!(!definition_flag_enabled(&def, "additive"));
    }

    #[test]
    pub(crate) fn decimal_and_hex_flags_enable_same_named_flags() {
        let def = test_definition_with_flags("0x4,2097152");

        assert!(definition_flag_enabled(&def, "draw_last"));
        assert!(definition_flag_enabled(&def, "disable_backface_culling"));
    }

    #[test]
    pub(crate) fn appending_texture_native_updates_txd_dictionary() {
        let dict_struct = rw_chunk(0x01, vec![0, 0, 0, 0]);
        let txd = rw_chunk(0x16, dict_struct);
        let mut native_struct_data = vec![0u8; 88];
        native_struct_data[8..8 + "missing_tex".len()].copy_from_slice(b"missing_tex");
        let native = rw_chunk(0x15, rw_chunk(0x01, native_struct_data));

        let updated = append_texture_native_to_txd(txd, &native, "missing_tex").unwrap();

        assert_eq!(rd16(&updated, 24), 1);
        assert!(txd_contains_texture_native(&updated, "missing_tex"));
        assert_eq!(txd_chunk_len(&updated), updated.len());
    }

    #[test]
    pub(crate) fn gta_sa_internal_col_names_are_indexed_from_archives() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let gta_sa_dir = env::temp_dir().join(format!(
            "eagle_gta_archive_index_{}_{}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(gta_sa_dir.join("models")).unwrap();

        let expected = [
            "alpha_fence.col",
            "bistro_blok.col",
            "gen_roofbit1.col",
            "kmb_deadarm.col",
            "roofstuff13.col",
        ];
        let mut packed_col = Vec::new();
        for name in expected {
            let stem = name.trim_end_matches(".col").as_bytes();
            let mut model = vec![0u8; 32];
            model[0..4].copy_from_slice(b"COL2");
            model[4..8].copy_from_slice(&24u32.to_le_bytes());
            model[8..8 + stem.len()].copy_from_slice(stem);
            packed_col.extend(model);
        }
        write_img_archive(
            &gta_sa_dir.join(GTA_SA_MARKER_FILE),
            &[("generic.col".to_string(), packed_col)],
        )
        .unwrap();

        let mut dffs = BTreeSet::new();
        let mut cols = BTreeSet::new();
        let mut txds = BTreeSet::new();
        collect_archive_asset_names_from_files(
            gta_sa_img_files(&gta_sa_dir),
            &mut dffs,
            &mut cols,
            &mut txds,
        );
        for name in expected {
            assert!(cols.contains(name), "missing internal COL model {name}");
        }

        fs::remove_dir_all(gta_sa_dir).unwrap();
    }

    #[test]
    pub(crate) fn parse_effects_fxp_extracts_particle_system_metadata() {
        let effects = parse_gta_sa_effects_fxp(
            r#"
FX_PROJECT_DATA:

FX_SYSTEM_DATA:
2

NAME: smoke30m
LENGTH: 1.500
PLAYMODE: 2
CULLDIST: 80.000
NUM_PRIMS: 1
TEXTURE: cloud1
TEXTURE2: NULL

FX_SYSTEM_DATA:
NAME: fire
LENGTH: 0.750
PLAYMODE: 1
CULLDIST: 50.000
NUM_PRIMS: 2
TEXTURE: flame
TEXTURE2: flame_core
"#,
        );
        assert_eq!(effects.len(), 2);
        assert_eq!(effects[0].name, "smoke30m");
        assert_eq!(effects[0].textures, vec!["cloud1"]);
        assert_eq!(effects[0].play_mode, 2);
        assert_eq!(effects[0].primitive_count, 1);
        assert_eq!(effects[1].name, "fire");
        assert_eq!(effects[1].textures, vec!["flame", "flame_core"]);
    }

    #[test]
    pub(crate) fn appending_texture_native_renames_to_missing_slot() {
        let txd = rw_chunk(0x16, rw_chunk(0x01, 0u16.to_le_bytes().to_vec()));
        let native = imported_image_texture_native(Path::new("does-not-exist.png"), "source_name")
            .unwrap_or_else(|_| {
                let mut data = Vec::new();
                data.extend_from_slice(&9u32.to_le_bytes());
                data.extend_from_slice(&0x1101u32.to_le_bytes());
                let mut raw_name = [0u8; 32];
                raw_name[..11].copy_from_slice(b"source_name");
                data.extend_from_slice(&raw_name);
                data.extend_from_slice(&[0u8; 32]);
                data.extend_from_slice(&0x0500u32.to_le_bytes());
                data.extend_from_slice(&[0u8; 4]);
                data.extend_from_slice(&1u16.to_le_bytes());
                data.extend_from_slice(&1u16.to_le_bytes());
                data.push(32);
                data.push(1);
                data.push(4);
                data.push(0);
                data.extend_from_slice(&4u32.to_le_bytes());
                data.extend_from_slice(&[0, 0, 0, 255]);
                rw_chunk(0x15, rw_chunk(0x01, data))
            });
        let updated = append_texture_native_to_txd(txd, &native, "missing_slot").unwrap();
        assert!(txd_contains_texture_native(&updated, "missing_slot"));
        assert!(!txd_contains_texture_native(&updated, "source_name"));
    }

    #[test]
    fn vehicle_ide_parser_keeps_model_and_absolute_wheel_sizes() {
        let vehicle = parse_vehicle_ide_line(
            "541, bullet, bullet, car, BULLET, BULLET, null, normal, 10, 0, 1f10, -1, 0.82, 0.91, 0",
            "test",
            true,
        )
        .unwrap();
        assert_eq!(vehicle.model_id, Some(541));
        assert_eq!(vehicle.id, "bullet");
        assert_eq!(vehicle.handling_id.as_deref(), Some("BULLET"));
        assert_eq!(vehicle.wheel_front, Some(0.82));
        assert_eq!(vehicle.wheel_rear, Some(0.91));
    }

    #[test]
    fn handling_cfg_parser_maps_vehicle_override_fields() {
        let text = "BULLET 1500 3500 2.0 0 0 -0.1 75 0.80 0.90 0.48 5 230 25 10 R P 8.0 0.52 0 35 1.2 0.15 0 0.25 -0.12 0.50 0.1 0.2 0.45";

        let handling = parse_vehicle_handling_cfg(text);
        let bullet = &handling["bullet"];

        assert_eq!(bullet["mass"], "1500");
        assert_eq!(bullet["numberOfGears"], "5");
        assert_eq!(bullet["driveType"], "R");
        assert_eq!(bullet["engineType"], "P");
        assert_eq!(bullet["collisionDamageMultiplier"], "0.45");
    }

    #[test]
    fn physics_root_parser_reads_object_dat_values_and_keeps_fallbacks() {
        let parsed = parse_physics_root_object_dat(
            "trafficcone, 31.0, 62.0 0.95, 0.07, 44.0, 10.0\n\
             Streetlamp2 650.0 4200.0 0.98 0.06 55.0 240.0\n\
             DYN_F_IRON_1 51000.0 52000.0 0.97 0.08 49.0 9999.0\n\
             vegasmashfnce_Gate 91000.0 92000.0 0.96 0.09 48.0 0.0\n\
             DYN_F_WOOD_2 53000.0 54000.0 0.95 0.10 47.0 9999.0\n\
             Gen_doorINT01 6.0 7.0 0.94 0.11 46.0 0.0\n\
             washgaspump 99999.0 500.0 0.99 0.05 50.0 120.0 1.0 20 0 1 1 2 0.0 0.0 0.0 explosion_medium 150.0 0.0 0.0 0.1 0.07 1 0\n\
             unknown_prop, 1.0, 2.0, 0.9, 0.1, 50.0\n",
        );

        assert_eq!(
            parsed.get(&1238),
            Some(&PhysicsRootProperties::new(31.0, 62.0, 0.95, 0.07, 44.0))
        );
        assert_eq!(
            parsed.get(&1231),
            Some(&PhysicsRootProperties::new(650.0, 4200.0, 0.98, 0.06, 55.0))
        );
        assert_eq!(
            parsed.get(&1419),
            Some(&PhysicsRootProperties::new(
                51000.0, 52000.0, 0.97, 0.08, 49.0
            ))
        );
        assert_eq!(
            parsed.get(&1553),
            Some(&PhysicsRootProperties::new(
                91000.0, 92000.0, 0.96, 0.09, 48.0
            ))
        );
        assert_eq!(
            parsed.get(&1408),
            Some(&PhysicsRootProperties::new(
                53000.0, 54000.0, 0.95, 0.10, 47.0
            ))
        );
        assert_eq!(
            parsed.get(&1491),
            Some(&PhysicsRootProperties::new(6.0, 7.0, 0.94, 0.11, 46.0))
        );
        assert!(parsed.get(&1676).is_some_and(|root| {
            root.mass == 99999.0
                && root.turn_mass == 500.0
                && root.uproot_limit == 120.0
                && root.collision_damage_effect == 20
                && root.smash_multiplier == 150.0
                && root.gun_break_mode == 1
                && root.causes_explosion
        }));
        assert!(!parsed.contains_key(&1218));

        let missing_root = PathBuf::from("/definitely/not/a/gta/root");
        let with_fallbacks = load_physics_root_properties(&missing_root);
        assert_eq!(
            with_fallbacks.get(&1218),
            Some(&PhysicsRootProperties::new(50.0, 50.0, 0.99, 0.05, 50.0))
        );
        assert!(with_fallbacks.get(&1676).is_some_and(|root| {
            root.mass == 99999.0
                && root.turn_mass == 500.0
                && root.uproot_limit == 120.0
                && root.collision_damage_effect == 20
                && root.smash_multiplier == 150.0
                && root.gun_break_mode == 1
                && !root.is_breakable()
        }));
        assert!(with_fallbacks.get(&1419).is_some_and(|root| {
            root.mass == 50000.0 && root.special_collision_response == 4 && root.is_breakable()
        }));
        assert!(with_fallbacks.get(&1553).is_some_and(|root| {
            root.mass == 99999.0 && root.smash_multiplier == 115.0 && root.is_breakable()
        }));
        assert!(with_fallbacks.get(&1408).is_some_and(|root| {
            root.mass == 50000.0 && root.gun_break_mode == 1 && root.is_breakable()
        }));
        assert_eq!(
            with_fallbacks.get(&1491),
            Some(&PhysicsRootProperties::new(5.0, 5.0, 0.98, 0.1, 50.0))
        );
    }

    #[test]
    fn physics_root_parser_reads_complete_breakable_behavior() {
        let parsed = parse_physics_root_object_dat(
            "DYN_F_WOOD_2, 50000.0, 50000.0 0.99, 0.05, 50.0, 9999.0, 1.0, \
             200, 4, 1, 0, 0, 0.0, 0.0, 0.0, none, 5000.0, 0.01, 0.01, \
             0.01, 0.07, 1, 0",
        );
        let wood = parsed.get(&1408).unwrap();
        assert!(wood.is_breakable());
        assert_eq!(wood.uproot_limit, 9999.0);
        assert_eq!(wood.collision_damage_effect, 200);
        assert_eq!(wood.special_collision_response, 4);
        assert_eq!(wood.smash_multiplier, 5000.0);
        assert_eq!(
            wood.break_velocity,
            V3 {
                x: 0.01,
                y: 0.01,
                z: 0.01
            }
        );
        assert_eq!(wood.break_velocity_randomness, 0.07);
        assert_eq!(wood.gun_break_mode, 1);
        assert!(!wood.sparks_on_impact);
    }

    #[test]
    fn collision_capsule_sidecar_round_trips_editable_source_and_ranges() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = env::temp_dir().join(format!(
            "eagle_collision_capsules_{}_{}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(&root).unwrap();
        let capsules = vec![CollisionCapsule {
            start: V3 {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            end: V3 {
                x: 4.0,
                y: 5.0,
                z: 6.0,
            },
            radius: 1.25,
            round_edges: false,
            surface: CollisionSurface {
                material: 7,
                flags: 8,
                brightness: 9,
                light: 10,
            },
            sphere_indices: [2, 3],
            vertex_start: 12,
            vertex_count: 34,
            face_start: 20,
            face_count: 64,
        }];

        save_collision_capsules(&root, "Tower.COL", &capsules).unwrap();
        assert!(!collision_capsules_path(&root).exists());
        assert_eq!(load_collision_capsules(&root, "tower.col"), capsules);
        save_collision_capsules(&root, "tower.col", &[]).unwrap();
        assert!(load_collision_capsules(&root, "tower.col").is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn collision_cuboid_sidecar_round_trips_editable_source_and_ranges() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = env::temp_dir().join(format!(
            "eagle_collision_cuboids_{}_{}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(&root).unwrap();
        let cuboids = vec![CollisionCuboid {
            center: V3 {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            half_extents: V3 {
                x: 4.0,
                y: 5.0,
                z: 6.0,
            },
            rotation: V3 {
                x: 12.0,
                y: 23.0,
                z: 34.0,
            },
            surface: CollisionSurface {
                material: 7,
                flags: 8,
                brightness: 9,
                light: 10,
            },
            vertex_start: 12,
            vertex_count: 8,
            face_start: 20,
            face_count: 12,
        }];

        save_collision_cuboids(&root, "Tower.COL", &cuboids).unwrap();
        assert!(!collision_cuboids_path(&root).exists());
        assert_eq!(load_collision_cuboids(&root, "tower.col"), cuboids);
        save_collision_cuboids(&root, "tower.col", &[]).unwrap();
        assert!(load_collision_cuboids(&root, "tower.col").is_empty());
        let _ = fs::remove_dir_all(root);
    }
}
