//! Shader-only material plugin ABI. Plugin code stays outside the editor binary.
use super::super::*;
use std::cell::RefCell;
use std::hash::{Hash, Hasher};

#[derive(Clone)]
struct MaterialPreview {
    program: u32,
    textures: Vec<(u32, u32)>,
    uniforms: Vec<(i32, [f32; 4])>,
    settings: [f32; 4],
    settings_location: i32,
    blend_mode: i32,
}
struct PluginProgram {
    id: u32,
    locations: HashMap<String, i32>,
}
#[derive(Default)]
struct PluginState {
    materials: HashMap<u32, MaterialPreview>,
    programs: HashMap<u64, PluginProgram>,
    camera_to_world: [f32; 16],
}
thread_local! { static PLUGINS: RefCell<PluginState> = RefCell::new(PluginState::default()); }

fn safe_file(root: &Path, file: &str) -> Option<PathBuf> {
    let path = Path::new(file);
    if path
        .components()
        .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return None;
    }
    let root = root.canonicalize().ok()?;
    let file = root.join(path).canonicalize().ok()?;
    (file.starts_with(root) && file.is_file()).then_some(file)
}

pub(crate) fn collect_material_plugins(root: &Path, files: &mut HashMap<String, PathBuf>) {
    let Ok(xml) = fs::read_to_string(root.join("eagleMaterials.xml")) else {
        return;
    };
    let attrs = Regex::new(r#"([A-Za-z_][A-Za-z0-9_]*)\s*=\s*"([^"]*)""#).unwrap();
    let bindings = Regex::new(r"<material\b([^>]*)/?>").unwrap();
    for cap in bindings.captures_iter(&xml) {
        let properties = parse_attrs(&cap[1], &attrs);
        let (Some(texture), Some(file)) = (properties.get("texture"), properties.get("file"))
        else {
            continue;
        };
        let Some(path) = safe_file(root, file) else {
            continue;
        };
        files.insert(format!("@material:{}", lower(texture)), path);
        if let Some(path) = properties
            .get("lightmap")
            .and_then(|file| safe_file(root, file))
        {
            files.insert(format!("@lightmap:{}", lower(texture)), path);
        }
    }
}

/// Search precedence matches material loading: user installs override bundled plugins.
pub(crate) fn material_plugin_roots() -> Vec<PathBuf> {
    let mut roots = vec![app_config_dir().join("plugins")];
    if let Ok(exe) = env::current_exe()
        && let Some(parent) = exe.parent()
    {
        roots.push(parent.join("plugins"));
    }
    roots.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("plugins"));
    roots
}

fn valid_plugin_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn plugin_folder(id: &str) -> Option<PathBuf> {
    if !valid_plugin_id(id) {
        return None;
    }
    material_plugin_roots()
        .into_iter()
        .map(|root| root.join(id))
        .find(|folder| folder.join("plugin.json").is_file())
}

pub(crate) struct MaterialPluginInfo {
    pub id: String,
    pub name: String,
    pub folder: PathBuf,
    pub details: Vec<String>,
    pub error: Option<String>,
    pub enabled: bool,
}

fn plugin_enabled_in(values: &BTreeMap<String, String>, id: &str) -> bool {
    values
        .get(&format!("plugin_{id}_enabled"))
        .is_none_or(|value| value != "false")
}

thread_local! {
    static DISABLED_PLUGINS: RefCell<HashSet<String>> = RefCell::new(
        load_preferences().iter().filter_map(|(key, value)| {
            (value == "false").then(|| key.strip_prefix("plugin_")?.strip_suffix("_enabled").map(str::to_owned)).flatten()
        }).collect()
    );
}

pub(crate) fn discover_material_plugins() -> Vec<MaterialPluginInfo> {
    discover_material_plugins_in(&material_plugin_roots(), &load_preferences())
}

fn discover_material_plugins_in(
    roots: &[PathBuf],
    values: &BTreeMap<String, String>,
) -> Vec<MaterialPluginInfo> {
    let mut found = BTreeMap::new();
    for root in roots {
        let Ok(entries) = fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            let folder = entry.path();
            let Some(id) = folder
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned)
            else {
                continue;
            };
            if !valid_plugin_id(&id)
                || !folder.join("plugin.json").is_file()
                || found.contains_key(&id)
            {
                continue;
            }
            let mut plugin = MaterialPluginInfo {
                enabled: plugin_enabled_in(values, &id),
                name: id.clone(),
                id: id.clone(),
                folder: folder.clone(),
                details: Vec::new(),
                error: None,
            };
            let manifest = fs::read_to_string(folder.join("plugin.json"))
                .map_err(|error| error.to_string())
                .and_then(|text| {
                    serde_json::from_str::<serde_json::Value>(&text)
                        .map_err(|error| error.to_string())
                });
            match manifest {
                Err(error) => plugin.error = Some(format!("Invalid manifest: {error}")),
                Ok(manifest) => {
                    plugin.name = manifest["name"].as_str().unwrap_or(&id).to_string();
                    for (key, label) in [
                        ("version", "Version"),
                        ("api", "API"),
                        ("author", "Author"),
                        ("description", "Description"),
                        ("descriptor", "Descriptor"),
                        ("vertex", "Vertex shader"),
                        ("fragment", "Fragment shader"),
                    ] {
                        if let Some(value) = manifest.get(key) {
                            plugin.details.push(format!(
                                "{label}: {}",
                                value
                                    .as_str()
                                    .map(str::to_owned)
                                    .unwrap_or_else(|| value.to_string())
                            ));
                        }
                    }
                    plugin.error = validate_material_plugin_manifest(&folder, &id, &manifest).err();
                    if let Some(samplers) = manifest["samplers"].as_array() {
                        plugin
                            .details
                            .push(format!("{} texture samplers", samplers.len()));
                    }
                    if let Some(uniforms) = manifest["uniforms"].as_array() {
                        plugin
                            .details
                            .push(format!("{} shader uniforms", uniforms.len()));
                    }
                }
            }
            found.insert(id, plugin);
        }
    }
    found.into_values().collect()
}

fn validate_material_plugin_manifest(
    folder: &Path,
    id: &str,
    manifest: &serde_json::Value,
) -> Result<(), String> {
    if manifest
        .get("id")
        .and_then(|value| value.as_str())
        .is_some_and(|declared| declared != id)
    {
        return Err("Manifest ID does not match the plugin folder".into());
    }
    if manifest["api"].as_str() != Some("eagle-material-v1") {
        return Err("Unsupported material plugin API".into());
    }
    for key in ["vertex", "fragment"] {
        if manifest[key]
            .as_str()
            .and_then(|file| safe_file(folder, file))
            .is_none()
        {
            return Err(format!("Missing or invalid {key} shader"));
        }
    }
    if !string_array(&manifest["uniforms"])
        .is_some_and(|names| names.iter().any(|name| name == "Settings"))
    {
        return Err("Missing Settings uniform declaration".into());
    }
    if string_array(&manifest["samplers"]).is_none()
        || string_array(&manifest["uniforms"]).is_none()
    {
        return Err("Invalid sampler or uniform declarations".into());
    }
    Ok(())
}

pub(crate) fn save_material_plugin_preferences(
    plugins: &[MaterialPluginInfo],
) -> std::io::Result<()> {
    let mut values = load_preferences();
    for plugin in plugins {
        values.insert(
            format!("plugin_{}_enabled", plugin.id),
            plugin.enabled.to_string(),
        );
    }
    save_preferences_checked(&values)?;
    DISABLED_PLUGINS.with(|disabled| {
        let mut disabled = disabled.borrow_mut();
        for plugin in plugins {
            if plugin.enabled {
                disabled.remove(&plugin.id);
            } else {
                disabled.insert(plugin.id.clone());
            }
        }
    });
    Ok(())
}

fn vec4(value: &serde_json::Value) -> Option<[f32; 4]> {
    let a = value.as_array()?;
    if a.len() != 4 {
        return None;
    }
    let mut out = [0.0; 4];
    for (i, v) in a.iter().enumerate() {
        out[i] = v.as_f64()? as f32;
        if !out[i].is_finite() {
            return None;
        }
    }
    Some(out)
}
fn string_array(value: &serde_json::Value) -> Option<Vec<String>> {
    value
        .as_array()?
        .iter()
        .map(|v| v.as_str().map(str::to_owned))
        .collect()
}
fn program_location(program: u32, name: &str) -> i32 {
    CString::new(name)
        .map(|s| unsafe { gl::GetUniformLocation(program, s.as_ptr()) })
        .unwrap_or(-1)
}
fn compile_program(vertex: &str, fragment: &str) -> Result<u32, String> {
    let vs = compile_shader(gl::VERTEX_SHADER, vertex)?;
    let fs = match compile_shader(gl::FRAGMENT_SHADER, fragment) {
        Ok(v) => v,
        Err(e) => {
            unsafe { gl::DeleteShader(vs) };
            return Err(e);
        }
    };
    unsafe {
        let program = gl::CreateProgram();
        gl::AttachShader(program, vs);
        gl::AttachShader(program, fs);
        gl::LinkProgram(program);
        gl::DeleteShader(vs);
        gl::DeleteShader(fs);
        let mut ok = 0;
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut ok);
        if ok == 0 {
            let log = program_info_log(program);
            gl::DeleteProgram(program);
            return Err(log);
        }
        Ok(program)
    }
}
fn upload_cube(faces: &[PathBuf]) -> Option<u32> {
    if faces.len() != 6 {
        return None;
    }
    let images: Vec<_> = faces
        .iter()
        .map(|p| image::open(p).map(|i| i.to_rgba8()))
        .collect::<Result<_, _>>()
        .ok()?;
    let size = images[0].dimensions();
    if size.0 != size.1 || images.iter().any(|i| i.dimensions() != size) {
        return None;
    }
    unsafe {
        let mut id = 0;
        gl::GenTextures(1, &mut id);
        gl::BindTexture(gl::TEXTURE_CUBE_MAP, id);
        for (index, image) in images.iter().enumerate() {
            gl::TexImage2D(
                gl::TEXTURE_CUBE_MAP_POSITIVE_X + index as u32,
                0,
                gl::RGBA8 as i32,
                size.0 as i32,
                size.1 as i32,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                image.as_raw().as_ptr().cast(),
            );
        }
        gl::GenerateMipmap(gl::TEXTURE_CUBE_MAP);
        gl::TexParameteri(
            gl::TEXTURE_CUBE_MAP,
            gl::TEXTURE_MIN_FILTER,
            gl::LINEAR_MIPMAP_LINEAR as i32,
        );
        gl::TexParameteri(
            gl::TEXTURE_CUBE_MAP,
            gl::TEXTURE_MAG_FILTER,
            gl::LINEAR as i32,
        );
        for parameter in [gl::TEXTURE_WRAP_S, gl::TEXTURE_WRAP_T, gl::TEXTURE_WRAP_R] {
            gl::TexParameteri(gl::TEXTURE_CUBE_MAP, parameter, gl::CLAMP_TO_EDGE as i32);
        }
        gl::BindTexture(gl::TEXTURE_CUBE_MAP, 0);
        Some(id)
    }
}
// Optional authored mip chain: automatic downsampling can destroy material masks/fades.
fn upload_material_mips(paths: &[PathBuf]) -> Option<u32> {
    if paths.is_empty() || paths.len() > 32 {
        return None;
    }
    let images: Vec<_> = paths
        .iter()
        .map(|p| image::open(p).map(|i| i.to_rgba8()))
        .collect::<Result<_, _>>()
        .ok()?;
    let base = images.first()?.dimensions();
    if images.iter().enumerate().any(|(level, image)| {
        image.dimensions() != ((base.0 >> level).max(1), (base.1 >> level).max(1))
    }) {
        return None;
    }
    unsafe {
        let mut id = 0;
        gl::GenTextures(1, &mut id);
        gl::BindTexture(gl::TEXTURE_2D, id);
        for (level, image) in images.iter().enumerate() {
            gl::TexImage2D(
                gl::TEXTURE_2D,
                level as i32,
                gl::RGBA8 as i32,
                image.width() as i32,
                image.height() as i32,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                image.as_raw().as_ptr().cast(),
            );
        }
        gl::TexParameteri(
            gl::TEXTURE_2D,
            gl::TEXTURE_MAX_LEVEL,
            images.len() as i32 - 1,
        );
        gl::TexParameteri(
            gl::TEXTURE_2D,
            gl::TEXTURE_MIN_FILTER,
            gl::LINEAR_MIPMAP_LINEAR as i32,
        );
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);
        gl::BindTexture(gl::TEXTURE_2D, 0);
        Some(id)
    }
}
fn material_sampler(
    root: &Path,
    asset: &serde_json::Value,
    textures: &mut HashMap<String, u32>,
) -> Option<(u32, u32)> {
    let cube = asset.get("type").and_then(|v| v.as_str()) == Some("cube");
    let file = safe_file(root, asset.get("file")?.as_str()?)?;
    let key = format!("@plugin:{}", file.display());
    let target = if cube {
        gl::TEXTURE_CUBE_MAP
    } else {
        gl::TEXTURE_2D
    };
    if let Some(id) = textures.get(&key) {
        return Some((target, *id));
    }
    let id = if cube {
        let faces = asset
            .get("faces")?
            .as_array()?
            .iter()
            .map(|v| safe_file(root, v.as_str()?))
            .collect::<Option<Vec<_>>>()?;
        upload_cube(&faces)?
    } else if let Some(mipmaps) = asset.get("mipmaps").and_then(|v| v.as_array()) {
        let paths = mipmaps
            .iter()
            .map(|v| safe_file(root, v.as_str()?))
            .collect::<Option<Vec<_>>>()?;
        upload_material_mips(&paths)?
    } else {
        let image = image::open(file).ok()?.to_rgba8();
        upload_rgba_texture(image.width(), image.height(), image.as_raw())
    };
    textures.insert(key, id);
    Some((target, id))
}
fn load_material_preview(
    texture: u32,
    key: &str,
    files: &HashMap<String, PathBuf>,
    textures: &mut HashMap<String, u32>,
) -> Result<Option<MaterialPreview>, String> {
    let Some(path) = files.get(&format!("@material:{}", lower(key))) else {
        return Ok(None);
    };
    let data: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let id = data
        .get("plugin")
        .and_then(|v| v.as_str())
        .ok_or("Missing plugin ID")?;
    if DISABLED_PLUGINS.with(|disabled| disabled.borrow().contains(id)) {
        return Ok(None);
    }
    let folder =
        plugin_folder(id).ok_or_else(|| format!("Material plugin '{id}' is not installed"))?;
    let manifest: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(folder.join("plugin.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    validate_material_plugin_manifest(&folder, id, &manifest)?;
    let vertex_path = safe_file(
        &folder,
        manifest["vertex"].as_str().ok_or("Missing vertex shader")?,
    )
    .ok_or("Invalid vertex shader path")?;
    let fragment_path = safe_file(
        &folder,
        manifest["fragment"]
            .as_str()
            .ok_or("Missing fragment shader")?,
    )
    .ok_or("Invalid fragment shader path")?;
    let vertex = fs::read_to_string(vertex_path).map_err(|e| e.to_string())?;
    let snippet = data
        .get("fragmentSnippet")
        .and_then(|v| v.as_str())
        .or_else(|| manifest.get("fragmentDefault").and_then(|v| v.as_str()))
        .unwrap_or("");
    let fragment = fs::read_to_string(fragment_path)
        .map_err(|e| e.to_string())?
        .replace("/*GENERIC_PROGRAM*/", snippet);
    let samplers = string_array(&manifest["samplers"]).ok_or("Invalid sampler declarations")?;
    let uniform_names =
        string_array(&manifest["uniforms"]).ok_or("Invalid uniform declarations")?;
    let mut max_units = 0;
    unsafe {
        gl::GetIntegerv(gl::MAX_TEXTURE_IMAGE_UNITS, &mut max_units);
    }
    if samplers.len() + 1 > max_units.max(0) as usize {
        return Err("Insufficient material texture units".into());
    }
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    vertex.hash(&mut hasher);
    fragment.hash(&mut hasher);
    samplers.hash(&mut hasher);
    uniform_names.hash(&mut hasher);
    let signature = hasher.finish();
    let program = PLUGINS.with(|s| -> Result<u32, String> {
        let mut s = s.borrow_mut();
        if let Some(p) = s.programs.get(&signature) {
            return Ok(p.id);
        }
        let p = compile_program(&vertex, &fragment)?;
        let locations = uniform_names
            .iter()
            .chain(samplers.iter())
            .map(|n| (n.clone(), program_location(p, n)))
            .collect();
        s.programs
            .insert(signature, PluginProgram { id: p, locations });
        eprintln!("Material plugin '{id}': compiled program {p}");
        Ok(p)
    })?;
    let root = path
        .parent()
        .and_then(Path::parent)
        .ok_or("Material JSON must be in resource/materials")?;
    let mut loaded = Vec::new();
    for name in &samplers {
        if name == "LightmapTexture" {
            loaded.push((gl::TEXTURE_2D, 0));
            continue;
        }
        if let Some(asset) = data["samplers"].get(name) {
            loaded.push(
                material_sampler(root, asset, textures).ok_or_else(|| {
                    format!("Invalid material sampler {name} in {}", path.display())
                })?,
            );
        } else {
            loaded.push((
                if name == "ReflectionTexture" {
                    gl::TEXTURE_CUBE_MAP
                } else {
                    gl::TEXTURE_2D
                },
                0,
            ));
        }
    }
    let locations = PLUGINS.with(|s| s.borrow().programs[&signature].locations.clone());
    unsafe {
        gl::UseProgram(program);
        for (unit, name) in samplers.iter().enumerate() {
            gl::Uniform1i(locations[name], if unit == 0 { 12 } else { unit as i32 });
        }
        gl::UseProgram(0);
    }
    let uniforms = uniform_names
        .iter()
        .map(|name| {
            (
                locations[name],
                data["uniforms"]
                    .get(name)
                    .and_then(vec4)
                    .unwrap_or([0.0; 4]),
            )
        })
        .collect();
    let settings = data["uniforms"]["Settings"]
        .as_array()
        .and_then(|_| vec4(&data["uniforms"]["Settings"]))
        .unwrap_or([0.0; 4]);
    let blend_mode = data["uniforms"]["RenderSettings"][0].as_i64().unwrap_or(0) as i32;
    let _ = texture;
    Ok(Some(MaterialPreview {
        program,
        textures: loaded,
        uniforms,
        settings,
        settings_location: locations["Settings"],
        blend_mode,
    }))
}

pub(crate) fn register_material_preview(
    texture: u32,
    key: &str,
    files: &HashMap<String, PathBuf>,
    textures: &mut HashMap<String, u32>,
) {
    if texture == 0 {
        return;
    }
    let exists = PLUGINS.with(|s| s.borrow().materials.contains_key(&texture));
    if exists {
        return;
    }
    match load_material_preview(texture, key, files, textures) {
        Ok(Some(material)) => {
            PLUGINS.with(|s| s.borrow_mut().materials.insert(texture, material));
        }
        Ok(None) => {}
        Err(error) => eprintln!("Material preview {key}: {error}"),
    }
}

fn material_blend_transparency(blend_mode: i32) -> Option<TransparencyMode> {
    // A material's framebuffer operation overrides the fallback bitmap's alpha histogram.
    // In particular an all-black additive texture must never become a solid cutout.
    (blend_mode != 0).then_some(TransparencyMode::Blend)
}
pub(crate) fn material_preview_transparency(texture: u32) -> Option<TransparencyMode> {
    PLUGINS.with(|s| {
        s.borrow()
            .materials
            .get(&texture)
            .and_then(|m| material_blend_transparency(m.blend_mode))
    })
}

pub(crate) fn has_material_previews() -> bool {
    PLUGINS.with(|s| !s.borrow().materials.is_empty())
}

pub(crate) fn set_material_preview_view(camera_to_world: Mat4) {
    PLUGINS.with(|s| s.borrow_mut().camera_to_world = camera_to_world.to_cols_array());
}
pub(crate) fn bind_material_preview(texture: u32, lightmap: u32) {
    PLUGINS.with(|s| {
        let s = s.borrow();
        unsafe {
            let mut current = 0;
            gl::GetIntegerv(gl::CURRENT_PROGRAM, &mut current);
            // Pointlight/bake/vehicle passes own their own shaders.
            if current != 0 && !s.programs.values().any(|p| p.id == current as u32) {
                return;
            }
            let Some(material) = s.materials.get(&texture) else {
                if current != 0 {
                    gl::UseProgram(0);
                }
                gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
                return;
            };
            gl::UseProgram(material.program);
            for (unit, (target, id)) in material.textures.iter().enumerate() {
                gl::ActiveTexture(gl::TEXTURE0 + if unit == 0 { 12 } else { unit as u32 });
                gl::BindTexture(*target, if unit == 1 { lightmap } else { *id });
            }
            for (location, value) in &material.uniforms {
                gl::Uniform4fv(*location, 1, value.as_ptr());
            }
            let mut settings = material.settings;
            settings[3] = if lightmap != 0 { 1.0 } else { 0.0 };
            gl::Uniform4fv(material.settings_location, 1, settings.as_ptr());
            gl::Uniform1f(
                program_location(material.program, "Time"),
                get_time() as f32,
            );
            gl::Uniform1i(
                program_location(material.program, "FogEnabled"),
                if gl::IsEnabled(gl::FOG) != 0 { 1 } else { 0 },
            );
            gl::UniformMatrix4fv(
                program_location(material.program, "CameraToWorld"),
                1,
                gl::FALSE,
                s.camera_to_world.as_ptr(),
            );
            match material.blend_mode {
                4 | 8 => gl::BlendFunc(gl::ONE, gl::ONE),
                2 => gl::BlendFunc(gl::ZERO, gl::SRC_COLOR),
                3 => gl::BlendFunc(gl::DST_COLOR, gl::SRC_COLOR),
                _ => gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA),
            }
            gl::ActiveTexture(gl::TEXTURE0);
        }
    });
}
pub(crate) fn reset_material_preview() {
    PLUGINS.with(|s| {
        let s = s.borrow();
        if s.programs.is_empty() {
            return;
        }
        unsafe {
            let mut current = 0;
            gl::GetIntegerv(gl::CURRENT_PROGRAM, &mut current);
            let owned = s.programs.values().any(|p| p.id == current as u32);
            if current != 0 && !owned {
                return;
            }
            if owned {
                gl::UseProgram(0);
            }
            // Preserve texture units 0/1 for the existing diffuse/lightmap renderer.
            for unit in 2..13 {
                gl::ActiveTexture(gl::TEXTURE0 + unit);
                gl::BindTexture(gl::TEXTURE_2D, 0);
                gl::BindTexture(gl::TEXTURE_CUBE_MAP, 0);
            }
            gl::ActiveTexture(gl::TEXTURE0);
        }
    });
}
pub(crate) fn forget_material_preview(texture: u32) {
    PLUGINS.with(|s| {
        s.borrow_mut().materials.remove(&texture);
    });
}
pub(crate) fn clear_material_plugins() {
    reset_material_preview();
    PLUGINS.with(|s| {
        let mut s = s.borrow_mut();
        s.materials.clear();
        unsafe {
            for p in s.programs.values() {
                gl::DeleteProgram(p.id);
            }
        }
        s.programs.clear();
    });
}
#[cfg(test)]
mod material_plugin_tests {
    use super::*;
    #[test]
    fn framebuffer_blend_overrides_opaque_and_cutout_fallbacks() {
        assert!(material_blend_transparency(0).is_none());
        for mode in 1..=8 {
            assert_eq!(
                material_blend_transparency(mode),
                Some(TransparencyMode::Blend)
            );
        }
    }

    fn write_test_plugin(root: &Path, id: &str, name: &str) {
        let folder = root.join(id);
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("material.vert"), "vertex shader").unwrap();
        fs::write(folder.join("material.frag"), "fragment shader").unwrap();
        fs::write(
            folder.join("plugin.json"),
            serde_json::json!({
                "id": id, "name": name, "version": 1, "api": "eagle-material-v1",
                "vertex": "material.vert", "fragment": "material.frag",
                "samplers": [], "uniforms": ["Settings"]
            })
            .to_string(),
        )
        .unwrap();
    }

    #[test]
    fn plugin_discovery_matches_loader_precedence_and_saved_states() {
        let root = env::temp_dir().join(format!("eagle-plugin-discovery-{}", std::process::id()));
        let user = root.join("user");
        let bundled = root.join("bundled");
        write_test_plugin(&user, "example-materials", "User Example");
        write_test_plugin(&bundled, "example-materials", "Bundled Example");
        write_test_plugin(&bundled, "other-plugin", "Other Plugin");
        let values = BTreeMap::from([(
            "plugin_example-materials_enabled".to_string(),
            "false".to_string(),
        )]);
        let plugins = discover_material_plugins_in(&[user.clone(), bundled], &values);
        assert_eq!(plugins.len(), 2);
        assert_eq!(plugins[0].name, "User Example");
        assert_eq!(plugins[0].folder, user.join("example-materials"));
        assert!(!plugins[0].enabled);
        assert!(plugins[1].enabled);
        assert!(plugins.iter().all(|plugin| plugin.error.is_none()));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn malformed_and_incompatible_plugins_remain_visible_with_details() {
        let root = env::temp_dir().join(format!("eagle-plugin-invalid-{}", std::process::id()));
        write_test_plugin(&root, "incompatible", "Incompatible");
        fs::write(
            root.join("incompatible/plugin.json"),
            r#"{"api":"future-api"}"#,
        )
        .unwrap();
        fs::create_dir_all(root.join("malformed")).unwrap();
        fs::write(root.join("malformed/plugin.json"), "not json").unwrap();
        let plugins = discover_material_plugins_in(&[root.clone()], &BTreeMap::new());
        assert_eq!(plugins.len(), 2);
        assert_eq!(
            plugins[0].error.as_deref(),
            Some("Unsupported material plugin API")
        );
        assert!(
            plugins[1]
                .error
                .as_ref()
                .unwrap()
                .starts_with("Invalid manifest:")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn manifest_validation_rejects_shader_escape_and_missing_settings() {
        let root = env::temp_dir().join(format!("eagle-plugin-manifest-{}", std::process::id()));
        write_test_plugin(&root, "example-materials", "Example");
        let folder = root.join("example-materials");
        let mut manifest: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(folder.join("plugin.json")).unwrap()).unwrap();
        assert!(validate_material_plugin_manifest(&folder, "example-materials", &manifest).is_ok());
        manifest["vertex"] = serde_json::json!("../example-materials/material.vert");
        assert_eq!(
            validate_material_plugin_manifest(&folder, "example-materials", &manifest).unwrap_err(),
            "Missing or invalid vertex shader"
        );
        manifest["vertex"] = serde_json::json!("material.vert");
        manifest["uniforms"] = serde_json::json!([]);
        assert_eq!(
            validate_material_plugin_manifest(&folder, "example-materials", &manifest).unwrap_err(),
            "Missing Settings uniform declaration"
        );
        assert!(!valid_plugin_id("../example-materials"));
        assert!(!valid_plugin_id(""));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn descriptor_rejects_escape_paths_and_accepts_map_local_materials() {
        let root = std::env::temp_dir().join(format!("eagle-materials-{}", std::process::id()));
        fs::create_dir_all(root.join("Map/materials")).unwrap();
        fs::write(root.join("Map/materials/m.json"), "{}").unwrap();
        fs::write(root.join("outside.json"), "{}").unwrap();
        fs::write(root.join("Map/eagleMaterials.xml"),r#"<eagleMaterials><material texture="ground" file="materials/m.json"/><material texture="escape" file="../outside.json"/></eagleMaterials>"#).unwrap();
        let mut files = HashMap::new();
        collect_material_plugins(&root.join("Map"), &mut files);
        assert_eq!(files.len(), 1);
        assert!(files.contains_key("@material:ground"));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn plugin_vector_validation_rejects_nan_and_wrong_arity() {
        assert!(vec4(&serde_json::json!([1, 2, 3])).is_none());
        assert_eq!(
            vec4(&serde_json::json!([1, 2, 3, 4])),
            Some([1.0, 2.0, 3.0, 4.0])
        );
    }
}
