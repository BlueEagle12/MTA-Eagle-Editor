use macroquad::prelude::*;
use regex::Regex;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque, hash_map::DefaultHasher},
    env,
    ffi::{CString, c_void},
    fs,
    hash::{Hash, Hasher},
    io::{BufRead, BufReader, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicBool, Ordering},
    sync::{Arc, Mutex, OnceLock, mpsc},
    thread,
    time::{Duration, Instant, SystemTime},
};
use walkdir::WalkDir;

#[allow(
    non_snake_case,
    non_upper_case_globals,
    dead_code,
    unsafe_op_in_unsafe_fn
)]
mod gl {
    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));
}

#[cfg(target_os = "linux")]
#[link(name = "GL")]
unsafe extern "C" {
    fn glXGetProcAddress(proc_name: *const u8) -> *const c_void;
}

#[cfg(windows)]
#[link(name = "opengl32")]
unsafe extern "system" {
    fn wglGetProcAddress(proc_name: *const u8) -> *const c_void;
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryA(name: *const u8) -> *mut c_void;
    fn GetProcAddress(module: *mut c_void, name: *const u8) -> *const c_void;
}

const BROWSE_ROOT: &str = ".";
const PANEL_W: f32 = 384.0;
const RIGHT_PANEL_W: f32 = 420.0;
/// Keep the 3D workspace usable on laptop-sized displays. The inspector stays
/// available because it is where most edits are committed; the outliner is
/// automatically tucked away below this threshold instead of squeezing the
/// viewport into an unusable strip.
const LEFT_SIDEBAR_MIN_SCREEN_W: f32 = 920.0;
// Header rows: project/file controls, workspace navigation, then editing actions.
const TOP_H: f32 = 154.0;
const STATUS_H: f32 = 34.0;
const EDITOR_GROUP_ATTR: &str = "editorGroup";
const DEFAULT_BOX_SELECT_DISTANCE: f32 = 6000.0;
const FAR_DRAW: f32 = 9000.0;
const DEFAULT_DRAW: f32 = 2500.0;
const VIEWPORT_FAR_CLIP: f32 = 16000.0;
const DEFAULT_VERTEX_BUDGET: usize = 6_000_000;
const NO_SELECTION: usize = usize::MAX;
const SETTLE_SECONDS: f64 = 0.35;
const AUTOSAVE_INTERVAL_SECONDS: f64 = 60.0;
const SETTLE_VERTEX_BUDGET: usize = 1_200_000;
const LIVE_RESOURCE_REFRESH_BATCH_LIMIT: usize = 256;
const LIVE_RESOURCE_REFRESH_FRAME_BUDGET: Duration = Duration::from_millis(5);
const DEFAULT_CAMERA_SPEED: f32 = 50.0;
const MIN_CAMERA_SPEED: f32 = 20.0;
const MAX_CAMERA_SPEED: f32 = 5000.0;
const DEFAULT_DETAIL_CAMERA_SPEED: f32 = 20.0;
const MIN_DETAIL_CAMERA_SPEED: f32 = 2.0;
const MIN_EDITING_CAMERA_SPEED: f32 = 2.0;
/// Multiplier applied to every transform gimbal/gizmo's world-space size. The
/// gimbals are sized from camera distance, which reads far too small on small
/// or low-resolution displays, so this is user tunable.
const DEFAULT_GIZMO_SCALE: f32 = 1.0;
const MIN_GIZMO_SCALE: f32 = 0.25;
const MAX_GIZMO_SCALE: f32 = 10.0;
/// Multiplier on the base mouse-look sensitivity. Defaults tuned on one mouse
/// feel wildly different on other hardware, so this is user tunable.
const DEFAULT_CAMERA_ROTATION_SPEED: f32 = 1.0;
const MIN_CAMERA_ROTATION_SPEED: f32 = 0.1;
const MAX_CAMERA_ROTATION_SPEED: f32 = 4.0;
/// Radians of camera rotation per pixel of mouse movement, before the user's
/// rotation speed multiplier is applied.
const CAMERA_LOOK_SENSITIVITY: f32 = 0.003;
/// Factor used by the Preferences dialog's speed steppers.
const CAMERA_SPEED_STEP_FACTOR: f32 = 1.25;
const CAMERA_SPEED_WHEEL_FACTOR: f32 = 1.15;
const DETAIL_CAMERA_SPEED_WHEEL_FACTOR: f32 = 1.05;
const DEFAULT_FOG_STRENGTH: f32 = 1.0;
const MIN_FOG_STRENGTH: f32 = 0.0;
const MAX_FOG_STRENGTH: f32 = 3.0;
const DEFAULT_MSAA_SAMPLES: i32 = 1;
const PREVIEW_AMBIENT: [f32; 4] = [0.58, 0.62, 0.68, 1.0];
const PREVIEW_DIFFUSE: [f32; 4] = [0.78, 0.72, 0.62, 1.0];
const PREVIEW_SPECULAR: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const BUNDLED_TIMECYC_JSON: &str = include_str!("../assets/timecyc_sa.json");
const TIMECYC_HOURS: [&str; 8] = [
    "Midnight", "5AM", "6AM", "7AM", "Midday", "7PM", "8PM", "10PM",
];
const TIMECYC_PHASES: [(&str, &str); 5] = [
    ("Night", "Midnight"),
    ("Sunrise", "6AM"),
    ("Afternoon", "Midday"),
    ("Sunset", "7PM"),
    ("Evening", "10PM"),
];
const GPU_SHADOW_MAP_SIZE: i32 = 2048;
const GPU_POINT_SHADOW_MAP_SIZE: i32 = 1024;
const ASSET_DIR_NAME: &str = "assets";
#[cfg(not(windows))]
const DEFAULT_GTA_SA_DIR: &str = ".";
#[cfg(windows)]
const DEFAULT_GTA_SA_DIR: &str = "C:\\Program Files (x86)\\Rockstar Games\\GTA San Andreas";
const SIM_PLAYER_DFF: &str = "__sim_player.dff";
const SIM_VEHICLE_DFF: &str = "__sim_vehicle.dff";
const REPLACEMENT_IMG: &str = "light_mapper_replacements.img";
// VER2 reserves 24 bytes for an entry name, but MTA's IMG enumeration expects
// a trailing NUL. Editor-authored names therefore use at most 23 bytes.
const IMG_RUNTIME_SAFE_ENTRY_NAME_BYTES: usize = 23;
// GTA:SA's atomic render path stores one alpha byte per geometry material in a
// fixed 152-byte stack buffer. A 153rd material corrupts the caller's stack.
const GTA_DFF_MATERIAL_LIMIT: usize = 152;
const EAGLE_ELEMENT_TYPES: [&str; 3] = ["building", "object", "scenery"];
static PHYSICS_ROOT_SPECS: [PhysicsRootSpec; 18] = [
    PhysicsRootSpec::new(
        "Traffic Light",
        1352,
        "CJ_TRAFFIC_LIGHT3",
        PhysicsRootProperties::new(700.0, 700.0, 0.99, 0.05, 50.0).with_breakable_behavior(
            240.0,
            1,
            100.0,
            V3 {
                x: 0.0,
                y: 0.0,
                z: 0.1,
            },
            0.07,
            0,
            false,
        ),
    ),
    PhysicsRootSpec::new(
        "Traffic Cone",
        1238,
        "trafficcone",
        PhysicsRootProperties::new(25.0, 50.0, 0.99, 0.03, 50.0),
    ),
    PhysicsRootSpec::new(
        "Trash Can",
        1331,
        "BinNt01_LA",
        PhysicsRootProperties::new(100.0, 100.0, 0.99, 0.1, 50.0),
    ),
    PhysicsRootSpec::new(
        "Traffic Lamp / Streetlight",
        1231,
        "Streetlamp2",
        PhysicsRootProperties::new(600.0, 4000.0, 0.99, 0.05, 50.0).with_breakable_behavior(
            240.0,
            1,
            100.0,
            V3 {
                x: 0.0,
                y: 0.0,
                z: 0.1,
            },
            0.07,
            0,
            true,
        ),
    ),
    PhysicsRootSpec::new(
        "Roadwork Barrier",
        1228,
        "roadworkbarrier1",
        PhysicsRootProperties::new(30.0, 50.0, 0.99, 0.04, 50.0).with_breakable_behavior(
            0.0,
            0,
            115.0,
            V3 {
                x: 0.0,
                y: 0.0,
                z: 0.1,
            },
            0.07,
            1,
            false,
        ),
    ),
    PhysicsRootSpec::new(
        "Cardboard Box",
        1230,
        "cardboardbox",
        PhysicsRootProperties::new(10.0, 10.0, 0.99, 0.03, 50.0),
    ),
    PhysicsRootSpec::new(
        "Wooden Crate",
        1224,
        "woodenbox",
        PhysicsRootProperties::new(250.0, 250.0, 0.99, 0.04, 50.0),
    ),
    PhysicsRootSpec::new(
        "Barrel",
        1218,
        "barrel1",
        PhysicsRootProperties::new(50.0, 50.0, 0.99, 0.05, 50.0),
    ),
    PhysicsRootSpec::new(
        "Fuel Pump (Explosive)",
        1676,
        "washgaspump",
        PhysicsRootProperties::new(99999.0, 500.0, 0.99, 0.05, 50.0).with_damage_behavior(
            120.0,
            20,
            0,
            150.0,
            V3 {
                x: 0.0,
                y: 0.0,
                z: 0.1,
            },
            0.07,
            1,
            false,
        ),
    ),
    PhysicsRootSpec::new(
        "Fire Hydrant",
        1211,
        "fire_hydrant",
        PhysicsRootProperties::new(150.0, 50.0, 0.99, 0.03, 50.0),
    ),
    PhysicsRootSpec::new(
        "Bollard",
        1214,
        "bollard",
        PhysicsRootProperties::new(150.0, 50.0, 0.99, 0.03, 50.0),
    ),
    PhysicsRootSpec::new(
        "Metal Fence",
        1419,
        "DYN_F_IRON_1",
        PhysicsRootProperties::new(50000.0, 50000.0, 0.99, 0.05, 50.0).with_breakable_behavior(
            9999.0,
            4,
            5000.0,
            V3 {
                x: 0.01,
                y: 0.01,
                z: 0.01,
            },
            0.07,
            1,
            false,
        ),
    ),
    PhysicsRootSpec::new(
        "Metal Fence Gate",
        1553,
        "vegasmashfnce_Gate",
        PhysicsRootProperties::new(99999.0, 99999.0, 0.99, 0.05, 50.0).with_breakable_behavior(
            0.0,
            0,
            115.0,
            V3 {
                x: 0.0,
                y: 0.0,
                z: 0.1,
            },
            0.07,
            1,
            false,
        ),
    ),
    PhysicsRootSpec::new(
        "Wood Fence",
        1408,
        "DYN_F_WOOD_2",
        PhysicsRootProperties::new(50000.0, 50000.0, 0.99, 0.05, 50.0).with_breakable_behavior(
            9999.0,
            4,
            5000.0,
            V3 {
                x: 0.01,
                y: 0.01,
                z: 0.01,
            },
            0.07,
            1,
            false,
        ),
    ),
    PhysicsRootSpec::new(
        "Door (Swinging)",
        1491,
        "Gen_doorINT01",
        PhysicsRootProperties::new(5.0, 5.0, 0.98, 0.1, 50.0),
    ),
    PhysicsRootSpec::new(
        "Mailbox",
        3407,
        "CE_mailbox1",
        PhysicsRootProperties::new(30.0, 50.0, 0.99, 0.05, 50.0),
    ),
    PhysicsRootSpec::new(
        "Dumpster",
        1343,
        "CJ_Dumpster3",
        PhysicsRootProperties::new(100.0, 100.0, 0.99, 0.1, 50.0),
    ),
    PhysicsRootSpec::new(
        "Small Street Sign",
        1233,
        "noparkingsign1",
        PhysicsRootProperties::new(30.0, 50.0, 0.99, 0.05, 50.0),
    ),
];
const EAGLE_DEFINITION_FLAGS: [&str; 16] = [
    "is_road",
    "draw_last",
    "additive",
    "no_zbuffer_write",
    "dont_receive_shadows",
    "is_glass_type_1",
    "is_glass_type_2",
    "is_garage_door",
    "is_damagable",
    "is_tree",
    "is_palm",
    "does_not_collide_with_flyer",
    "is_tag",
    "disable_backface_culling",
    "is_breakable_statue",
    "disable_collisions",
];
const EAGLE_PLACEMENT_OVERRIDE_FLAGS: [&str; 6] = [
    "double_sided",
    "disable_collisions",
    "breakable",
    "unbreakable",
    "frozen",
    "no_stream",
];

mod app_icon {
    include!(concat!(env!("OUT_DIR"), "/app_icon.rs"));
}

mod app;
mod assets;
mod blender_import;
mod blender_native;
mod col;
mod dff;
mod editor;
mod lighting;
mod render;
mod resource;
mod ui;

use col::{editing::*, export::*, generation::*, import::*, optimizer::*, validation::*};
use dff::{breakable::*, export::*, import::*, lod::*, optimize::*};

use app::preferences::*;
use assets::{join::*, replacement::*, textures::*, txd::*};
use blender_import::*;
use editor::{history::*, input::*};
use lighting::bake::*;
use lighting::vertex::*;
use render::{radar::*, runtime::*, scene::*};
use resource::{
    archive_rebalance::*, classify::*, collision_safety::*, cull::*, files::*, loading::*,
    lod_audit::*, material_classes::*, missing_texture_review::*, race::*, save::*, validation::*,
    water::*,
};
use ui::*;

#[derive(Clone)]
struct Options {
    root: PathBuf,
    root_from_cli: bool,
    launch_mode: LaunchMode,
    textures: bool,
    meshes: bool,
    render: bool,
    ui: bool,
    text: bool,
    fast_vbo: bool,
    vbo_selected_only: bool,
    vbo_immediate: bool,
    monitor: Option<u32>,
    msaa_samples: i32,
    draw_distance_percent: u16,
    draw_radius: f32,
    part_budget: usize,
    vertex_budget: usize,
    lod_mode: LodMode,
    lod_radius: f32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LaunchMode {
    Project,
    Editor,
}

fn launch_initial_tab(mode: LaunchMode) -> AppTab {
    match mode {
        LaunchMode::Project => AppTab::Preview,
        LaunchMode::Editor => AppTab::Editing,
    }
}

/// How LOD models are shown in the viewport.
/// - `Swap`: detail meshes up close, their LOD counterparts in the distance (GTA-style).
/// - `DetailOnly`: only full-detail meshes; LODs never drawn.
/// - `ShowAll`: draw both detail and LOD everywhere (debug).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum LodMode {
    Swap,
    DetailOnly,
    ShowAll,
}

impl LodMode {
    fn label(self) -> &'static str {
        match self {
            LodMode::Swap => "swap",
            LodMode::DetailOnly => "detail-only",
            LodMode::ShowAll => "show-all",
        }
    }
    fn next(self) -> LodMode {
        match self {
            LodMode::Swap => LodMode::DetailOnly,
            LodMode::DetailOnly => LodMode::ShowAll,
            LodMode::ShowAll => LodMode::Swap,
        }
    }
}

/// True when a placement names itself as an LOD model. Detail objects point at
/// their LOD via the `lodParent` attribute, so the set of all `lodParent` values
/// is the authoritative LOD-id set; this name-prefix check is a fallback for LOD
/// objects that nothing references.
fn is_lod_name(id: &str, dff: &str) -> bool {
    let id_l = id.trim();
    let dff_l = dff.trim();
    id_l.len() >= 3 && id_l[..3].eq_ignore_ascii_case("lod")
        || dff_l.len() >= 3 && dff_l[..3].eq_ignore_ascii_case("lod")
}

/// Collect the set of LOD object ids (lowercased) referenced by any placement's
/// `lodParent` attribute.
fn collect_lod_ids(placements: &[Placement]) -> std::collections::HashSet<String> {
    let mut ids = std::collections::HashSet::new();
    for p in placements {
        if let Some(parent) = p.attrs.get("lodParent") {
            let parent = parent.trim();
            // `self` is a special MTA marker, not the id of an LOD placement.
            if !parent.is_empty() && !parent.eq_ignore_ascii_case("self") {
                ids.insert(parent.to_ascii_lowercase());
            }
        }
    }
    ids
}

/// Whether a placement is an LOD object (referenced as someone's `lodParent`, or
/// named like an LOD as a fallback).
fn placement_is_lod(p: &Placement, lod_ids: &std::collections::HashSet<String>) -> bool {
    lod_ids.contains(&p.id.to_ascii_lowercase()) || is_lod_name(&p.id, &p.dff)
}

fn placement_is_app_lod(app: &AppState, placement: &Placement) -> bool {
    placement_is_lod(placement, &app.lod_ids)
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
struct V3 {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
struct EagleZoneOffsets {
    offset: Option<V3>,
    water_offset: Option<V3>,
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
struct V2 {
    u: f32,
    v: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Tri {
    a: u32,
    b: u32,
    c: u32,
    material: u16,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MissingAssetKind {
    Dff,
    Col,
    Txd,
    Definition,
}

#[derive(Clone)]
struct ValidationItem {
    kind: MissingAssetKind,
    key: String,
    label: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum ValidationActionCategory {
    #[default]
    Review,
    Repair,
    Optimize,
    Collision,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct DffMaterialAnim {
    names: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct DffUvAnimFrame {
    time: f32,
    uv: [f32; 6],
    prev: i32,
}

#[derive(Clone, Debug, PartialEq)]
struct DffUvAnimation {
    name: String,
    type_id: i32,
    flags: i32,
    duration: f32,
    node_to_uv: [u32; 8],
    frames: Vec<DffUvAnimFrame>,
}

#[derive(Clone, Default, PartialEq)]
struct Dff2dEffect {
    position: V3,
    effect_id: u32,
    payload: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum BreakableOrigin {
    Object,
    #[default]
    Collision,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct BreakableVertex {
    position: V3,
    uv: V2,
    color: [u8; 4],
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct BreakableTriangle {
    vertices: [u16; 3],
    group: u16,
    /// Editor-only intact-mesh face mapping used for manual fracture zones.
    /// Imported debris can intentionally differ, so this may be absent.
    source_face: Option<usize>,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct BreakableGroup {
    /// Editor-only label. The native plug-in stores groups by index.
    name: String,
    texture: String,
    mask: String,
    ambient: V3,
}

/// San Andreas Breakable PLG (`0x0253F2FD`) attached to one RenderWare
/// Geometry. This is deliberately independent from the intact mesh: the game
/// streams this second mesh only when the object fractures.
#[derive(Clone, Debug, Default, PartialEq)]
struct BreakableGeometry {
    origin: BreakableOrigin,
    vertices: Vec<BreakableVertex>,
    triangles: Vec<BreakableTriangle>,
    groups: Vec<BreakableGroup>,
    /// Editor state only. Topology-changing tools set this until the user
    /// regenerates or manually reviews the fracture zones.
    stale: bool,
}

const FRACTURE_PREVIEW_DURATION_SECONDS: f64 = 2.8;

/// One DFF geometry (atomic) committed into a flattened `RawMesh`, named after
/// the frame its atomic is attached to (e.g. "chassis", "door_lf_dummy").
#[derive(Clone, Default, PartialEq)]
struct RawMeshComponent {
    name: String,
    /// The RenderWare frame this geometry's atomic is attached to.  The
    /// imported mesh stores vertices in world space, so retaining this index
    /// lets the writer restore them to the matching frame-local space.
    frame_index: Option<usize>,
    vertex_start: usize,
    vertex_end: usize,
    tri_start: usize,
    tri_end: usize,
    breakable: Option<BreakableGeometry>,
}

#[derive(Clone, Default, PartialEq)]
struct RawMeshFrame {
    name: String,
    parent: i32,
    right: V3,
    up: V3,
    at: V3,
    pos: V3,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct RawMaterial {
    color: V3,
    alpha: f32,
    ambient: f32,
    specular: f32,
    diffuse: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum MaterialEmitterCastMode {
    Face,
    Point,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct MaterialEmitter {
    enabled: bool,
    emit_inversed: bool,
    cast_mode: MaterialEmitterCastMode,
    /// Whether Face-mode area samples test scene visibility while baking.
    casts_shadow: bool,
    max_grouping_size: f32,
    point_up_strength: f32,
    point_down_strength: f32,
    point_sides_strength: f32,
    use_material_color: bool,
    use_temperature: bool,
    color: V3,
    temperature: f32,
    strength: f32,
    falloff_distance: f32,
    day: bool,
    night: bool,
}

impl Default for MaterialEmitter {
    fn default() -> Self {
        Self {
            enabled: false,
            emit_inversed: false,
            cast_mode: MaterialEmitterCastMode::Face,
            casts_shadow: true,
            max_grouping_size: 2.0,
            point_up_strength: 1.0,
            point_down_strength: 1.0,
            point_sides_strength: 1.0,
            use_material_color: true,
            use_temperature: false,
            color: neutral_vertex_color(),
            temperature: 6500.0,
            strength: 1.0,
            falloff_distance: 512.0,
            day: true,
            night: true,
        }
    }
}

#[derive(Clone, Copy)]
struct VehicleMaterialPreview {
    body_a: V3,
    body_b: V3,
    lights_on: bool,
}

#[derive(Clone, Default, PartialEq)]
struct RawMesh {
    vertices: Vec<V3>,
    normals: Vec<V3>,
    /// Primary RenderWare texture-coordinate stream used by editor UV tools.
    uvs: Vec<V2>,
    /// Additional RenderWare texture-coordinate streams, in file order after
    /// `uvs`. Each preserved stream is either empty or vertex-aligned.
    secondary_uvs: Vec<Vec<V2>>,
    prelit_colors: Vec<V3>,
    /// Alpha channel paired with `prelit_colors`.
    prelit_alphas: Vec<f32>,
    night_prelit_colors: Vec<V3>,
    /// Alpha channel paired with `night_prelit_colors`.
    night_prelit_alphas: Vec<f32>,
    light_flags: Vec<bool>,
    triangles: Vec<Tri>,
    material_textures: Vec<String>,
    materials: Vec<RawMaterial>,
    material_animations: Vec<DffMaterialAnim>,
    uv_anim_dictionaries: Vec<Vec<u8>>,
    uv_animations: Vec<DffUvAnimation>,
    effects_2dfx: Vec<Dff2dEffect>,
    components: Vec<RawMeshComponent>,
    frames: Vec<RawMeshFrame>,
}

#[derive(Clone, PartialEq)]
struct Placement {
    id: String,
    dff: String,
    zone: String,
    tag: String,
    attrs: BTreeMap<String, String>,
    pos: V3,
    rot: V3,
}

#[derive(Clone, PartialEq)]
struct Definition {
    id: String,
    zone: String,
    attrs: BTreeMap<String, String>,
}

#[derive(Clone, Copy, Default, PartialEq)]
struct ElementState {
    deleted: bool,
    hidden: bool,
}

#[derive(Clone)]
struct ImgEntry {
    img_path: PathBuf,
    name: String,
    offset: u32,
    size: u32,
}

#[derive(Clone)]
struct RenderMesh {
    parts: Vec<RenderPart>,
    bounds: Bounds,
    material_animations: Vec<DffMaterialAnim>,
    uv_animations: Vec<DffUvAnimation>,
    effects_2dfx: Vec<Dff2dEffect>,
    /// Display names of the DFF components (frames/atomics), indexed by
    /// `RenderPart::component`.
    components: Vec<String>,
    /// World-space dummy/frame pivot for each component when the source DFF
    /// exposes one. Falls back to the component bounds center in UI code.
    component_pivots: Vec<Option<Vec3>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum TransparencyMode {
    Opaque,
    Cutout,
    Blend,
}

#[derive(Clone, PartialEq)]
struct CollisionMesh {
    name: String,
    spheres: Vec<CollisionSphere>,
    boxes: Vec<CollisionBox>,
    vertices: Vec<V3>,
    faces: Vec<CollisionFace>,
    bounds: Bounds,
    shadow_vertices: Vec<V3>,
    shadow_faces: Vec<CollisionFace>,
}

#[derive(Clone, Debug, PartialEq)]
struct CollisionSurface {
    material: u8,
    flags: u8,
    brightness: u8,
    light: u8,
}

#[derive(Clone, PartialEq)]
struct CollisionSphere {
    center: V3,
    radius: f32,
    surface: CollisionSurface,
}

#[derive(Clone, PartialEq)]
struct CollisionBox {
    min: V3,
    max: V3,
    surface: CollisionSurface,
}

/// Editor-authored capsule. GTA COL has no native capsule primitive, so the
/// editor materializes this as two native spheres plus a triangle cylinder.
/// The ranges keep that generated representation tied to the editable source.
#[derive(Clone, Debug, PartialEq)]
struct CollisionCapsule {
    start: V3,
    end: V3,
    radius: f32,
    round_edges: bool,
    surface: CollisionSurface,
    sphere_indices: [usize; 2],
    vertex_start: usize,
    vertex_count: usize,
    face_start: usize,
    face_count: usize,
}

/// Editable oriented box. Native COL boxes are axis-aligned only, so a
/// non-zero rotation is materialized as eight vertices and twelve triangles
/// forming six closed planes.
#[derive(Clone, Debug, PartialEq)]
struct CollisionCuboid {
    center: V3,
    half_extents: V3,
    rotation: V3,
    surface: CollisionSurface,
    vertex_start: usize,
    vertex_count: usize,
    face_start: usize,
    face_count: usize,
}

struct CollisionCuboidAuditResult {
    mesh: CollisionMesh,
    capsules: Vec<CollisionCapsule>,
    cuboids: Vec<CollisionCuboid>,
    native_boxes: usize,
    rotated_boxes: usize,
}

pub(crate) struct CollisionCuboidAuditJob {
    rx: mpsc::Receiver<Result<CollisionCuboidAuditResult, String>>,
    target_name: String,
}

#[derive(Clone, PartialEq)]
struct CollisionFace {
    a: u16,
    b: u16,
    c: u16,
    material: u8,
    light: u8,
    img_path: PathBuf,
    material_file_offset: u64,
    light_file_offset: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct SelectedCollisionFace {
    placement: usize,
    face: usize,
}

#[derive(Clone, Copy, PartialEq)]
struct Bounds {
    min: Vec3,
    max: Vec3,
}

#[derive(Clone, Copy)]
struct Vertex {
    pos: V3,
    normal: V3,
    uv: V2,
    color: V3,
    day_color: V3,
    night_color: V3,
    base_day_color: V3,
    base_night_color: V3,
    day_alpha: f32,
    night_alpha: f32,
    alpha: f32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum VehicleMaterialRole {
    BodyA,
    BodyB,
    /// Head/tail light material. In the game files these are tagged with a
    /// yellow/blue marker colour so the engine knows which texture is which
    /// light; in the editor they render with a white material colour (natural
    /// texture) and glow bright when the lights are switched on.
    Light,
}

#[derive(Clone)]
struct RenderPart {
    list: u32,
    /// Interleaved fixed-function vertex buffer used by editable, preview,
    /// vehicle, and simulation draws. Display lists remain as a compatibility
    /// fallback and for the legacy precompiled-cell path.
    vbo: u32,
    vertices: usize,
    material_index: usize,
    /// Index into `RenderMesh::components`.
    component: usize,
    texture: u32,
    texture_width: u16,
    texture_height: u16,
    texture_name: String,
    texture_fingerprint: Option<TextureContentFingerprint>,
    texture_missing: bool,
    transparency: TransparencyMode,
    alpha: f32,
    material_color: V3,
    material_ambient: f32,
    use_lighting: bool,
    vehicle_material_role: Option<VehicleMaterialRole>,
    /// Render this part at full brightness, unlit — used for vehicle lights
    /// when the "Lights" toggle is on so head/tail lights actually glow.
    emissive: bool,
    cpu_vertices: Vec<Vertex>,
    /// Original DFF triangle index for each triangle in `cpu_vertices`.
    face_indices: Vec<usize>,
}

#[derive(Clone)]
struct PrelightSampleGeometry {
    pos: V3,
    normal: Option<V3>,
    uv: Option<V2>,
    triangle: Option<[V3; 3]>,
    material: Option<usize>,
}

#[derive(Clone)]
struct PrelightClipboardSample {
    geometry: PrelightSampleGeometry,
    color: V3,
}

#[derive(Clone)]
struct PrelightClipboard {
    /// Channel selected when the copy was made.
    mode: BakeLightMode,
    /// Selected copied channel: day for Day/Both, night for Night.
    samples: Vec<PrelightClipboardSample>,
    /// Distinct night samples when copied in Both mode.
    secondary_samples: Vec<PrelightClipboardSample>,
    /// Lighting copied from the LOD placements assigned to the copied detail
    /// objects. Kept separate so detail geometry can never consume LOD colors.
    lod_samples: Vec<PrelightClipboardSample>,
    /// Distinct assigned-LOD night samples when copied in Both mode.
    lod_secondary_samples: Vec<PrelightClipboardSample>,
}

#[derive(Clone, Copy)]
struct BakedVertexAccum {
    color: Vec3,
    samples: usize,
    /// Night-channel result when a light bake targets both channels.
    secondary_color: Vec3,
    secondary_samples: usize,
}

struct ShadowTri {
    a: Vec3,
    b: Vec3,
    c: Vec3,
    min: Vec3,
    max: Vec3,
}

struct ShadowGrid {
    cell_size: f32,
    cells: HashMap<(i32, i32), Vec<usize>>,
    tris: Vec<ShadowTri>,
}

struct BakeLightIndexNode {
    min: Vec3,
    max: Vec3,
    left: Option<usize>,
    right: Option<usize>,
    lights: Vec<usize>,
}

#[derive(Default)]
struct BakeLightIndex {
    nodes: Vec<BakeLightIndexNode>,
    root: Option<usize>,
    global_lights: Vec<usize>,
}

struct GpuBakeChannel {
    mode: BakeLightMode,
    batches: Vec<Vec<EditorLight>>,
    next_batch: usize,
    output: Vec<f32>,
}

struct GpuBakeState {
    input: Vec<f32>,
    positions: Vec<Vec3>,
    triangles: Vec<[Vec3; 3]>,
    mapping: Vec<(String, usize, usize)>,
    channels: Vec<GpuBakeChannel>,
    current_channel: usize,
    completed_batches: usize,
    total_batches: usize,
    started: bool,
}

struct GpuLightmapPipeline {
    supported: bool,
    gl_version: String,
    glsl_version: String,
    vertex_buffer: u32,
    light_buffer: u32,
    output_buffer: u32,
    transform_feedback: u32,
    shadow_fbo: u32,
    point_shadow_fbo: u32,
    shadow_depth_texture: u32,
    point_shadow_depth_texture: u32,
    program: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BakePassKind {
    Light,
    AmbientOcclusion,
    AmbientBump,
}

struct BakeJob {
    pass: BakePassKind,
    settings: BakeSettings,
    scope: BakeScope,
    target_indices: Option<HashSet<usize>>,
    lights: Vec<EditorLight>,
    light_index: BakeLightIndex,
    material_face_emitters: HashMap<(String, usize), MaterialEmitter>,
    gpu_state: Option<GpuBakeState>,
    shadow_grid: ShadowGrid,
    accumulators: HashMap<String, Vec<Vec<BakedVertexAccum>>>,
    placement_index: usize,
    part_index: usize,
    vertex_index: usize,
    total_placements: usize,
    vertices: usize,
    lit_vertices: usize,
    total_light: Vec3,
    started_at: f64,
}

#[derive(Clone)]
struct TimecycSample {
    label: String,
    values: Vec<f32>,
    ambient: V3,
    ambient_object: V3,
    directional: V3,
    sky_top: V3,
    sky_bottom: V3,
    sun_core: V3,
    sun_corona: V3,
    postfx1: [f32; 4],
    postfx2: [f32; 4],
    far_clip: f32,
    fog_start: f32,
    light_on_ground: f32,
}

#[derive(Clone)]
struct TimecycWeather {
    name: String,
    samples: Vec<TimecycSample>,
}

#[derive(Clone)]
struct TimecycData {
    weathers: Vec<TimecycWeather>,
    source: String,
}

#[derive(Clone)]
struct TimecycState {
    data: TimecycData,
    weather_index: usize,
    hour_index: usize,
}

fn neutral_vertex_color() -> V3 {
    V3 {
        x: 1.0,
        y: 1.0,
        z: 1.0,
    }
}

fn v3_clamp01(value: V3) -> V3 {
    V3 {
        x: value.x.clamp(0.0, 1.0),
        y: value.y.clamp(0.0, 1.0),
        z: value.z.clamp(0.0, 1.0),
    }
}

fn vertex_bake_color(vertex: &Vertex, mode: BakeLightMode) -> V3 {
    match mode {
        BakeLightMode::Day => vertex.day_color,
        BakeLightMode::Night => vertex.night_color,
        BakeLightMode::Both => vertex.day_color,
    }
}

fn set_vertex_bake_color(vertex: &mut Vertex, mode: BakeLightMode, color: V3) {
    match mode {
        BakeLightMode::Day => vertex.day_color = color,
        BakeLightMode::Night => vertex.night_color = color,
        BakeLightMode::Both => {
            vertex.day_color = color;
            vertex.night_color = color;
        }
    }
}

fn apply_vertex_bake_display(vertex: &mut Vertex, mode: BakeLightMode) {
    vertex.color = vertex_bake_color(vertex, mode);
    vertex.alpha = match mode {
        BakeLightMode::Day | BakeLightMode::Both => vertex.day_alpha,
        BakeLightMode::Night => vertex.night_alpha,
    };
}

fn display_vertex_color(color: V3, ambient: V3, material_color: V3, material_ambient: f32) -> V3 {
    // RenderWare's PC lit/prelit equation is
    // (prelight + ambient * surfaceAmbient) * materialColor. Runtime vertex
    // colors already contain the material tint, so tint only the ambient term.
    let ambient_scale = material_ambient.max(0.0);
    v3_clamp01(V3 {
        x: color.x + ambient.x * ambient_scale * material_color.x,
        y: color.y + ambient.y * ambient_scale * material_color.y,
        z: color.z + ambient.z * ambient_scale * material_color.z,
    })
}

fn rebuild_render_part_list_with_lift(part: &mut RenderPart, ambient_lift: V3) {
    let mut packed = Vec::<f32>::with_capacity(part.cpu_vertices.len() * 12);
    for vertex in &part.cpu_vertices {
        let alpha = part.alpha * vertex.alpha;
        let color = display_vertex_color(
            vertex.color,
            ambient_lift,
            part.material_color,
            part.material_ambient,
        );
        packed.extend_from_slice(&[
            vertex.uv.u,
            vertex.uv.v,
            vertex.normal.x,
            vertex.normal.y,
            vertex.normal.z,
            color.x,
            color.y,
            color.z,
            alpha,
            vertex.pos.x,
            vertex.pos.y,
            vertex.pos.z,
        ]);
    }
    unsafe {
        if part.vbo == 0 {
            gl::GenBuffers(1, &mut part.vbo);
        }
        if part.vbo != 0 {
            gl::BindBuffer(gl::ARRAY_BUFFER, part.vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                std::mem::size_of_val(packed.as_slice()) as isize,
                packed.as_ptr().cast(),
                gl::DYNAMIC_DRAW,
            );
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        }
        if part.list != 0 {
            gl::DeleteLists(part.list, 1);
        }
        part.list = gl::GenLists(1);
        if part.list == 0 {
            return;
        }
        gl::NewList(part.list, gl::COMPILE);
        if part.texture != 0 {
            gl::Enable(gl::TEXTURE_2D);
            gl::BindTexture(gl::TEXTURE_2D, part.texture);
        } else {
            gl::Disable(gl::TEXTURE_2D);
        }
        gl::Begin(gl::TRIANGLES);
        for vertex in &part.cpu_vertices {
            let alpha = part.alpha * vertex.alpha;
            let color = display_vertex_color(
                vertex.color,
                ambient_lift,
                part.material_color,
                part.material_ambient,
            );
            gl::Color4f(color.x, color.y, color.z, alpha);
            gl::Normal3f(vertex.normal.x, vertex.normal.y, vertex.normal.z);
            gl::TexCoord2f(vertex.uv.u, vertex.uv.v);
            gl::Vertex3f(vertex.pos.x, vertex.pos.y, vertex.pos.z);
        }
        gl::End();
        gl::BindTexture(gl::TEXTURE_2D, 0);
        gl::Disable(gl::TEXTURE_2D);
        gl::EndList();
    }
}

fn rebuild_mesh_part_lists(meshes: &mut HashMap<String, RenderMesh>) {
    rebuild_mesh_part_lists_with_lift(meshes, V3::default());
}

fn rebuild_mesh_part_lists_with_lift(meshes: &mut HashMap<String, RenderMesh>, ambient_lift: V3) {
    for mesh in meshes.values_mut() {
        for part in &mut mesh.parts {
            rebuild_render_part_list_with_lift(part, ambient_lift);
        }
    }
}

fn delete_render_mesh_gpu(mesh: &RenderMesh) {
    unsafe {
        for part in &mesh.parts {
            if part.list != 0 {
                gl::DeleteLists(part.list, 1);
            }
            if part.vbo != 0 {
                gl::DeleteBuffers(1, &part.vbo);
            }
        }
    }
}

fn replace_render_mesh(meshes: &mut HashMap<String, RenderMesh>, key: String, mesh: RenderMesh) {
    if let Some(previous) = meshes.insert(key, mesh) {
        delete_render_mesh_gpu(&previous);
    }
}

fn apply_bake_light_mode_to_mesh(mesh: &mut RenderMesh, mode: BakeLightMode) {
    for part in &mut mesh.parts {
        for vertex in &mut part.cpu_vertices {
            apply_vertex_bake_display(vertex, mode);
        }
    }
}

fn apply_bake_light_mode_to_meshes(meshes: &mut HashMap<String, RenderMesh>, mode: BakeLightMode) {
    for mesh in meshes.values_mut() {
        apply_bake_light_mode_to_mesh(mesh, mode);
    }
}

fn gl_string(name: u32) -> String {
    unsafe {
        let ptr = gl::GetString(name);
        if ptr.is_null() {
            String::new()
        } else {
            let cstr = std::ffi::CStr::from_ptr(ptr.cast());
            cstr.to_string_lossy().into_owned()
        }
    }
}

fn shader_info_log(shader: u32) -> String {
    unsafe {
        let mut len = 0;
        gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
        if len <= 1 {
            return String::new();
        }
        let mut buf = vec![0u8; len as usize];
        gl::GetShaderInfoLog(shader, len, std::ptr::null_mut(), buf.as_mut_ptr().cast());
        String::from_utf8_lossy(&buf)
            .trim_end_matches('\0')
            .to_string()
    }
}

fn program_info_log(program: u32) -> String {
    unsafe {
        let mut len = 0;
        gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
        if len <= 1 {
            return String::new();
        }
        let mut buf = vec![0u8; len as usize];
        gl::GetProgramInfoLog(program, len, std::ptr::null_mut(), buf.as_mut_ptr().cast());
        String::from_utf8_lossy(&buf)
            .trim_end_matches('\0')
            .to_string()
    }
}

fn compile_shader(kind: u32, source: &str) -> Result<u32, String> {
    unsafe {
        let shader = gl::CreateShader(kind);
        if shader == 0 {
            return Err("glCreateShader returned 0".to_string());
        }
        let c_source = CString::new(source).map_err(|err| err.to_string())?;
        let ptr = c_source.as_ptr();
        gl::ShaderSource(shader, 1, &ptr, std::ptr::null());
        gl::CompileShader(shader);
        let mut ok = 0;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut ok);
        if ok == 0 {
            let log = shader_info_log(shader);
            gl::DeleteShader(shader);
            return Err(log);
        }
        Ok(shader)
    }
}

fn create_gpu_lightmap_program() -> Result<u32, String> {
    const SOURCE: &str = r#"#version 130
attribute vec3 in_pos;
attribute vec3 in_normal;
attribute vec3 in_tri_a;
attribute vec3 in_tri_b;
attribute vec3 in_tri_c;
uniform int light_count;
uniform int light_kind[32];
uniform int light_point_lobe[32];
uniform vec4 light_pos_radius[32];
uniform vec4 light_dir_intensity[32];
uniform vec4 light_color[32];
uniform float bounce_scale;
uniform int shadow_enabled;
uniform int shadow_light_index;
uniform mat4 shadow_matrix;
uniform sampler2D shadow_map;
uniform samplerCube point_shadow_map;
uniform float shadow_bias;
uniform float shadow_normal_offset;
uniform float shadow_filter_radius;
uniform int shadow_sample_count;
uniform int shadow_kind;
uniform vec3 point_shadow_pos;
uniform float point_shadow_near;
uniform float point_shadow_far;
// Occluder distance window for the directional (ortho) shadow map, in world
// units. When shadow_occluder_range > 0, only occluders between min and max
// distance count (AO pass); 0 disables the window (light bake).
uniform float shadow_occluder_range;
uniform float shadow_occluder_min;
uniform float shadow_occluder_max;
varying vec3 baked_color;

vec3 closest_point_on_triangle(vec3 p, vec3 a, vec3 b, vec3 c) {
    vec3 ab = b - a;
    vec3 ac = c - a;
    vec3 ap = p - a;
    float d1 = dot(ab, ap);
    float d2 = dot(ac, ap);
    if (d1 <= 0.0 && d2 <= 0.0) return a;
    vec3 bp = p - b;
    float d3 = dot(ab, bp);
    float d4 = dot(ac, bp);
    if (d3 >= 0.0 && d4 <= d3) return b;
    float vc = d1 * d4 - d3 * d2;
    if (vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0) return a + ab * (d1 / (d1 - d3));
    vec3 cp = p - c;
    float d5 = dot(ab, cp);
    float d6 = dot(ac, cp);
    if (d6 >= 0.0 && d5 <= d6) return c;
    float vb = d5 * d2 - d1 * d6;
    if (vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0) return a + ac * (d2 / (d2 - d6));
    float va = d3 * d6 - d5 * d4;
    if (va <= 0.0 && d4 - d3 >= 0.0 && d5 - d6 >= 0.0) {
        return b + (c - b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6)));
    }
    float denom = 1.0 / (va + vb + vc);
    return a + ab * (vb * denom) + ac * (vc * denom);
}

bool local_light_has_geometric_influence(int i, vec3 p) {
    vec3 from_light = p - light_pos_radius[i].xyz;
    float dist = length(from_light);
    if (dist > max(light_pos_radius[i].w, 0.001)) return false;
    if (light_kind[i] == 3) {
        if (dist <= 0.001) return false;
        vec3 forward = normalize(light_dir_intensity[i].xyz);
        return dot(forward, from_light / dist) > cos(radians(36.0));
    }
    if (light_kind[i] == 2 && light_point_lobe[i] != 0) {
        if (dist <= 0.001) return false;
        vec3 direction = from_light / dist;
        if (light_point_lobe[i] == 1) return direction.z > 0.0;
        if (light_point_lobe[i] == 2) return direction.z < 0.0;
        return length(direction.xy) > 0.0;
    }
    return true;
}

float point_shadow_depth(float dist) {
    float near_plane = max(point_shadow_near, 0.001);
    float far_plane = max(point_shadow_far, near_plane + 1.0);
    float ndc = (far_plane + near_plane) / (far_plane - near_plane)
        - (2.0 * far_plane * near_plane) / ((far_plane - near_plane) * max(dist, near_plane));
    return ndc * 0.5 + 0.5;
}

float shadow_visibility(vec3 world_pos, vec3 normal) {
    if (shadow_enabled == 0) {
        return 1.0;
    }
    if (shadow_kind == 2) {
        // Point/area cubemaps need a small receiver lift to avoid sampling the
        // emitting triangle. Do not apply this to directional shadows: the
        // pre-area sun path projected the exact world position, and shifting
        // it by arbitrary imported vertex normals changes which sun-map texel
        // and depth are tested.
        world_pos += normal * shadow_normal_offset;
        vec3 to_fragment = world_pos - point_shadow_pos;
        float dist = length(to_fragment);
        if (dist <= 0.001 || dist > point_shadow_far) {
            return 1.0;
        }
        float closest = textureCube(point_shadow_map, to_fragment).r;
        // Each cubemap face was rendered with a regular 90-degree perspective
        // projection. Its depth coordinate is based on view-space Z, which is
        // the dominant absolute component of this direction, not Euclidean
        // distance. Comparing different spaces makes point/area shadows leak
        // or disappear away from a cube face's center.
        float projected_dist = max(max(abs(to_fragment.x), abs(to_fragment.y)), abs(to_fragment.z));
        float current = point_shadow_depth(projected_dist);
        return (current - shadow_bias <= closest) ? 1.0 : 0.0;
    }
    vec4 shadow_pos = shadow_matrix * vec4(world_pos, 1.0);
    if (abs(shadow_pos.w) <= 0.00001) {
        return 1.0;
    }
    vec3 uvw = shadow_pos.xyz / shadow_pos.w;
    uvw = uvw * 0.5 + 0.5;
    if (uvw.x < 0.0 || uvw.x > 1.0 || uvw.y < 0.0 || uvw.y > 1.0 || uvw.z < 0.0 || uvw.z > 1.0) {
        // Outside the fitted shadow map there is no valid depth comparison.
        // Treat it as visible instead of turning a precision/frustum miss into
        // a large artificial shadow.
        return 1.0;
    }
    float closest = texture2D(shadow_map, uvw.xy).r;
    if (shadow_occluder_range > 0.0) {
        // Ortho depth is linear, so the depth delta converts directly to a
        // world-space distance to the occluder along the light direction.
        float dist = (uvw.z - closest) * shadow_occluder_range;
        if (dist <= shadow_occluder_min) {
            return 1.0;
        }
        if (shadow_occluder_max > 0.0) {
            if (dist > shadow_occluder_max) {
                return 1.0;
            }
            // Linear falloff: near occluders darken fully, distant ones fade.
            return clamp(dist / shadow_occluder_max, 0.0, 1.0);
        }
        return 0.0;
    }
    int samples = clamp(shadow_sample_count, 1, 128);
    if (shadow_filter_radius <= 0.01 || samples <= 1) {
        return (uvw.z - shadow_bias <= closest) ? 1.0 : 0.0;
    }
    // The receiver's own texel is always the first sample. Previously even a
    // one-sample bake was displaced several texels along a Vogel disk, so it
    // was not testing the receiver at all.
    float visibility = (uvw.z - shadow_bias <= closest) ? 1.0 : 0.0;
    float texel = 1.0 / 2048.0;
    // Vogel disk: dense, even coverage without the large unsampled gaps from
    // the old fixed 3x3 kernel. The configured Shadow Samples now genuinely
    // controls GPU shadow quality.
    for (int i = 1; i < 128; ++i) {
        if (i >= samples) {
            break;
        }
        float fi = float(i) - 0.5;
        float radius = sqrt(fi / float(samples - 1)) * shadow_filter_radius;
        float angle = fi * 2.39996323;
        vec2 offset = vec2(cos(angle), sin(angle)) * radius * texel;
        float sample_depth = texture2D(shadow_map, clamp(uvw.xy + offset, 0.0, 1.0)).r;
        visibility += (uvw.z - shadow_bias <= sample_depth) ? 1.0 : 0.0;
    }
    return visibility / float(samples);
}

void main() {
    vec3 n = normalize(in_normal);
    vec3 accum = vec3(0.0);
    for (int i = 0; i < 32; ++i) {
        if (i >= light_count) {
            break;
        }
        vec3 receiver_pos = in_pos;
        if (light_kind[i] >= 2 && light_kind[i] <= 4) {
            if (!local_light_has_geometric_influence(i, receiver_pos)) {
                vec3 surface_pos = closest_point_on_triangle(
                    light_pos_radius[i].xyz, in_tri_a, in_tri_b, in_tri_c);
                if (local_light_has_geometric_influence(i, surface_pos)) {
                    receiver_pos = surface_pos;
                }
            }
        }
        float visibility = (i == shadow_light_index) ? shadow_visibility(receiver_pos, n) : 1.0;
        vec3 color = light_color[i].rgb * light_dir_intensity[i].w;
        if (light_kind[i] == 0) {
            accum += color;
        } else if (light_kind[i] == 1) {
            vec3 dir = normalize(light_dir_intensity[i].xyz);
            // Stored direction is the forward vector along which light
            // travels; surface-to-source is therefore the inverse.
            float amount = max(dot(n, -dir), 0.0);
            accum += color * amount * visibility;
            // Bounces approximate diffuse sky/world fill, so unlike the
            // direct term they are not gated by N.L or direct visibility.
            accum += color * bounce_scale;
        } else if (light_kind[i] == 4) {
            // A quadrature point on an emitting triangle. Its intensity is
            // radiance times the patch area it represents, so applying both
            // cosine terms and inverse-square distance integrates the actual
            // emitting surface instead of approximating it with a spotlight.
            vec3 to_light = light_pos_radius[i].xyz - receiver_pos;
            float dist = length(to_light);
            float radius = max(light_pos_radius[i].w, 0.001);
            if (dist > 0.001 && dist <= radius) {
                vec3 receiver_to_light = to_light / dist;
                vec3 emitter_normal = normalize(light_dir_intensity[i].xyz);
                float emitter_cosine = max(dot(emitter_normal, -receiver_to_light), 0.0);
                if (emitter_cosine > 0.0) {
                    float t = clamp(dist / radius, 0.0, 1.0);
                    float attenuation = 1.0 - t * t * (3.0 - 2.0 * t);
                    float geometry = emitter_cosine / max(dist * dist, 1.0);
                    accum += color * geometry * attenuation * bounce_scale;
                    float receiver_cosine = max(dot(n, receiver_to_light), 0.0);
                    accum += color * geometry * receiver_cosine * attenuation * visibility;
                }
            }
        } else {
            vec3 to_light = light_pos_radius[i].xyz - receiver_pos;
            float dist = length(to_light);
            float radius = max(light_pos_radius[i].w, 0.001);
            if (dist <= radius) {
                float t = clamp(dist / radius, 0.0, 1.0);
                // Smooth fade with zero slope at both ends; reaches zero at
                // the configured falloff distance without a visible cutoff.
                float attenuation = 1.0 - t * t * (3.0 - 2.0 * t);
                if (light_kind[i] == 3) {
                    // Match the 72-degree spotlight shadow projection and
                    // feather the outer 20 percent of the cone.
                    if (dist <= 0.001) {
                        attenuation = 0.0;
                    } else {
                        vec3 forward = normalize(light_dir_intensity[i].xyz);
                        float cone_cos = dot(forward, normalize(-to_light));
                        float outer_cos = cos(radians(36.0));
                        float inner_cos = cos(radians(28.8));
                        attenuation *= smoothstep(outer_cos, inner_cos, cone_cos);
                    }
                } else if (light_kind[i] == 2 && dist > 0.001) {
                    // Material Point emitters are split into independently
                    // weighted vertical and horizontal lobes. Authored point
                    // lights use lobe 0 and remain fully omnidirectional.
                    vec3 from_light = normalize(-to_light);
                    float horizontal = length(from_light.xy);
                    float lobe_weight_sum = max(abs(from_light.z) + horizontal, 0.0001);
                    if (light_point_lobe[i] == 1) {
                        attenuation *= max(from_light.z, 0.0) / lobe_weight_sum;
                    } else if (light_point_lobe[i] == 2) {
                        attenuation *= max(-from_light.z, 0.0) / lobe_weight_sum;
                    } else if (light_point_lobe[i] == 3) {
                        attenuation *= horizontal / lobe_weight_sum;
                    }
                }
                accum += color * attenuation * bounce_scale;
                if (dist > 0.001) {
                    float amount = max(dot(n, normalize(to_light)), 0.0);
                    accum += color * amount * attenuation * visibility;
                }
            }
        }
    }
    // Preserve values above 1.0 until the shared CPU/GPU finalization step,
    // where bake exposure is applied before conversion to vertex colors.
    baked_color = max(accum, vec3(0.0));
    gl_Position = vec4(0.0, 0.0, 0.0, 1.0);
}
"#;
    unsafe {
        let shader = compile_shader(gl::VERTEX_SHADER, SOURCE)?;
        let program = gl::CreateProgram();
        if program == 0 {
            gl::DeleteShader(shader);
            return Err("glCreateProgram returned 0".to_string());
        }
        gl::AttachShader(program, shader);
        let in_pos = CString::new("in_pos").unwrap();
        let in_normal = CString::new("in_normal").unwrap();
        let in_tri_a = CString::new("in_tri_a").unwrap();
        let in_tri_b = CString::new("in_tri_b").unwrap();
        let in_tri_c = CString::new("in_tri_c").unwrap();
        gl::BindAttribLocation(program, 0, in_pos.as_ptr());
        gl::BindAttribLocation(program, 1, in_normal.as_ptr());
        gl::BindAttribLocation(program, 2, in_tri_a.as_ptr());
        gl::BindAttribLocation(program, 3, in_tri_b.as_ptr());
        gl::BindAttribLocation(program, 4, in_tri_c.as_ptr());
        let varying = CString::new("baked_color").unwrap();
        let varyings = [varying.as_ptr()];
        gl::TransformFeedbackVaryings(program, 1, varyings.as_ptr(), gl::INTERLEAVED_ATTRIBS);
        gl::LinkProgram(program);
        gl::DetachShader(program, shader);
        gl::DeleteShader(shader);
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

fn init_gpu_lightmap_pipeline() -> GpuLightmapPipeline {
    let gl_version = gl_string(gl::VERSION);
    let glsl_version = gl_string(gl::SHADING_LANGUAGE_VERSION);
    let has_tf = gl::BeginTransformFeedback::is_loaded()
        && gl::EndTransformFeedback::is_loaded()
        && gl::TransformFeedbackVaryings::is_loaded()
        && gl::BindBufferBase::is_loaded()
        && gl::BindTransformFeedback::is_loaded()
        && gl::GetBufferSubData::is_loaded();
    let has_shadow_map = gl::GenFramebuffers::is_loaded()
        && gl::BindFramebuffer::is_loaded()
        && gl::FramebufferTexture2D::is_loaded()
        && gl::CheckFramebufferStatus::is_loaded()
        && gl::DrawBuffer::is_loaded()
        && gl::ReadBuffer::is_loaded();
    let mut buffers = [0u32; 3];
    let mut transform_feedback = 0u32;
    let mut shadow_fbo = 0u32;
    let mut point_shadow_fbo = 0u32;
    let mut shadow_depth_texture = 0u32;
    let mut point_shadow_depth_texture = 0u32;
    unsafe {
        if has_tf {
            gl::GenBuffers(buffers.len() as i32, buffers.as_mut_ptr());
            gl::GenTransformFeedbacks(1, &mut transform_feedback);
        }
        if has_shadow_map {
            gl::GenFramebuffers(1, &mut shadow_fbo);
            gl::GenFramebuffers(1, &mut point_shadow_fbo);
            gl::GenTextures(1, &mut shadow_depth_texture);
            gl::GenTextures(1, &mut point_shadow_depth_texture);
            gl::BindTexture(gl::TEXTURE_2D, shadow_depth_texture);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::DEPTH_COMPONENT24 as i32,
                GPU_SHADOW_MAP_SIZE,
                GPU_SHADOW_MAP_SIZE,
                0,
                gl::DEPTH_COMPONENT,
                gl::FLOAT,
                std::ptr::null(),
            );
            // Depth is compared manually in the bake shader. Linear filtering
            // invents depths across triangle silhouettes and leaks sunlight;
            // smoothing is handled explicitly by PCF instead.
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_COMPARE_MODE, gl::NONE as i32);
            gl::BindFramebuffer(gl::FRAMEBUFFER, shadow_fbo);
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::DEPTH_ATTACHMENT,
                gl::TEXTURE_2D,
                shadow_depth_texture,
                0,
            );
            gl::DrawBuffer(gl::NONE);
            gl::ReadBuffer(gl::NONE);
            if gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE {
                eprintln!("GPU shadow framebuffer is incomplete");
                shadow_fbo = 0;
                shadow_depth_texture = 0;
            }
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            gl::BindTexture(gl::TEXTURE_2D, 0);
            gl::BindTexture(gl::TEXTURE_CUBE_MAP, point_shadow_depth_texture);
            for face in 0..6 {
                gl::TexImage2D(
                    gl::TEXTURE_CUBE_MAP_POSITIVE_X + face,
                    0,
                    gl::DEPTH_COMPONENT24 as i32,
                    GPU_POINT_SHADOW_MAP_SIZE,
                    GPU_POINT_SHADOW_MAP_SIZE,
                    0,
                    gl::DEPTH_COMPONENT,
                    gl::FLOAT,
                    std::ptr::null(),
                );
            }
            gl::TexParameteri(
                gl::TEXTURE_CUBE_MAP,
                gl::TEXTURE_MIN_FILTER,
                gl::NEAREST as i32,
            );
            gl::TexParameteri(
                gl::TEXTURE_CUBE_MAP,
                gl::TEXTURE_MAG_FILTER,
                gl::NEAREST as i32,
            );
            gl::TexParameteri(
                gl::TEXTURE_CUBE_MAP,
                gl::TEXTURE_WRAP_S,
                gl::CLAMP_TO_EDGE as i32,
            );
            gl::TexParameteri(
                gl::TEXTURE_CUBE_MAP,
                gl::TEXTURE_WRAP_T,
                gl::CLAMP_TO_EDGE as i32,
            );
            gl::TexParameteri(
                gl::TEXTURE_CUBE_MAP,
                gl::TEXTURE_WRAP_R,
                gl::CLAMP_TO_EDGE as i32,
            );
            gl::TexParameteri(
                gl::TEXTURE_CUBE_MAP,
                gl::TEXTURE_COMPARE_MODE,
                gl::NONE as i32,
            );
            gl::BindFramebuffer(gl::FRAMEBUFFER, point_shadow_fbo);
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::DEPTH_ATTACHMENT,
                gl::TEXTURE_CUBE_MAP_POSITIVE_X,
                point_shadow_depth_texture,
                0,
            );
            gl::DrawBuffer(gl::NONE);
            gl::ReadBuffer(gl::NONE);
            if gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE {
                eprintln!("GPU point shadow framebuffer is incomplete");
                point_shadow_fbo = 0;
                point_shadow_depth_texture = 0;
            }
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            gl::BindTexture(gl::TEXTURE_CUBE_MAP, 0);
        }
    }
    let program = if has_tf {
        match create_gpu_lightmap_program() {
            Ok(program) => program,
            Err(err) => {
                eprintln!("GPU bake shader unavailable: {err}");
                0
            }
        }
    } else {
        0
    };
    let supported = has_tf
        && buffers.iter().all(|buffer| *buffer != 0)
        && transform_feedback != 0
        && program != 0;
    GpuLightmapPipeline {
        supported,
        gl_version,
        glsl_version,
        vertex_buffer: buffers[0],
        light_buffer: buffers[1],
        output_buffer: buffers[2],
        transform_feedback,
        shadow_fbo,
        point_shadow_fbo,
        shadow_depth_texture,
        point_shadow_depth_texture,
        program,
    }
}

fn gpu_lightmap_label(pipeline: &GpuLightmapPipeline) -> &'static str {
    if pipeline.supported
        && pipeline.vertex_buffer != 0
        && pipeline.light_buffer != 0
        && pipeline.output_buffer != 0
        && pipeline.transform_feedback != 0
        && pipeline.shadow_fbo != 0
        && pipeline.point_shadow_fbo != 0
        && pipeline.shadow_depth_texture != 0
        && pipeline.point_shadow_depth_texture != 0
        && pipeline.program != 0
    {
        "GPU"
    } else {
        "CPU"
    }
}

struct SceneCell {
    list: u32,
    min: Vec3,
    max: Vec3,
    center: Vec3,
    radius: f32,
    placements: usize,
    parts: usize,
    vertices: usize,
}

/// Fixed-function-compatible vertex used by the pre-batched world-cell VBOs.
/// Colors use normalized bytes to reduce resident memory and vertex bandwidth
/// without sacrificing geometric precision.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
struct WorldVertex {
    uv: [f32; 2],
    normal: [f32; 3],
    shaded_rgba: [u8; 4],
    unshaded_rgba: [u8; 4],
    position: [f32; 3],
}

const _: () = assert!(std::mem::size_of::<WorldVertex>() == 40);
const _: () = assert!(std::mem::align_of::<WorldVertex>() == 4);
const _: () = assert!(std::mem::offset_of!(WorldVertex, uv) == 0);
const _: () = assert!(std::mem::offset_of!(WorldVertex, normal) == 8);
const _: () = assert!(std::mem::offset_of!(WorldVertex, shaded_rgba) == 20);
const _: () = assert!(std::mem::offset_of!(WorldVertex, unshaded_rgba) == 24);
const _: () = assert!(std::mem::offset_of!(WorldVertex, position) == 28);

struct WorldBatch {
    texture: u32,
    use_lighting: bool,
    double_sided: bool,
    transparency: TransparencyMode,
    first_vertex: usize,
    vertices: usize,
    data: Vec<WorldVertex>,
    /// Per-part ranges retain the material lookup identity used by the
    /// collision-classification viewport. The normal renderer can still batch
    /// those parts by render state, while the classification renderer assigns
    /// the correct color without walking the original placements.
    classification_segments: Vec<WorldClassificationSegment>,
    /// Translucent geometry is packed by render state into one VBO. The
    /// opaque-texel pass can therefore draw the whole batch at once, while the
    /// color pass retains exact per-part ranges for back-to-front sorting.
    blend_segments: Vec<WorldBlendSegment>,
}

struct WorldClassificationSegment {
    txd_name: Option<String>,
    texture_name: String,
    texture_fingerprint: Option<TextureContentFingerprint>,
    first_vertex: usize,
    vertices: usize,
}

struct WorldBlendSegment {
    center: Vec3,
    first_vertex: usize,
    vertices: usize,
}

const WORLD_CELL_SIZE: f32 = 256.0;
const DEFAULT_SLICER_CHUNK_SIZE: f32 = 200.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct WorldCellKey {
    x: i32,
    y: i32,
}

/// Conservative upper-bound buckets for authored element draw distances.
///
/// Geometry is split by tier inside each spatial cell so short-range objects do
/// not keep an entire 256 m cell resident out to the viewport's global cap.
/// Every finite tier rounds upward, so bucketing can only draw an element later
/// than its authored distance, never make it disappear early.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum WorldDrawDistanceTier {
    D128,
    D256,
    D512,
    D1024,
    D2048,
    D4096,
    D8192,
    Infinite,
}

impl WorldDrawDistanceTier {
    fn distance(self) -> f32 {
        match self {
            Self::D128 => 128.0,
            Self::D256 => 256.0,
            Self::D512 => 512.0,
            Self::D1024 => 1024.0,
            Self::D2048 => 2048.0,
            Self::D4096 => 4096.0,
            Self::D8192 => 8192.0,
            Self::Infinite => f32::INFINITY,
        }
    }
}

fn world_cell_key(position: V3) -> WorldCellKey {
    WorldCellKey {
        x: (position.x / WORLD_CELL_SIZE).floor() as i32,
        y: (position.y / WORLD_CELL_SIZE).floor() as i32,
    }
}

fn world_cell_key_is_selected(position: V3, selected_keys: Option<&HashSet<WorldCellKey>>) -> bool {
    selected_keys.is_none_or(|keys| keys.contains(&world_cell_key(position)))
}

struct WorldCell {
    key: WorldCellKey,
    draw_distance_tier: WorldDrawDistanceTier,
    min: Vec3,
    max: Vec3,
    center: Vec3,
    radius: f32,
    placements: usize,
    parts: usize,
    vertices: usize,
    /// Bytes occupied by the packed vertex payload when this cell is uploaded.
    gpu_bytes: usize,
    /// Last time this cell fell inside the residency preload band.
    resident_last_required_at: f64,
    vbo: u32,
    batches: Vec<WorldBatch>,
}

type WorldBatchKey = (u32, bool, bool, TransparencyMode);

struct WorldBatchBuild {
    data: Vec<WorldVertex>,
    classification_segments: Vec<WorldClassificationSegment>,
}

struct WorldBlendData {
    key: WorldBatchKey,
    data: Vec<WorldVertex>,
    center: Vec3,
    classification: WorldClassificationSegment,
}

struct WorldCellBuild {
    min: Vec3,
    max: Vec3,
    placements: usize,
    parts: usize,
    vertices: usize,
    batch_data: HashMap<WorldBatchKey, WorldBatchBuild>,
    blend_data: Vec<WorldBlendData>,
}

struct CollisionRenderCache {
    list: u32,
}

#[derive(Clone, Copy)]
struct CameraState {
    pos: Vec3,
    yaw: f32,
    pitch: f32,
    last_mouse: Vec2,
    looking: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CameraMode {
    Freeroam,
    Focus,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TransformMode {
    Select,
    Move,
    Rotate,
    Scale,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TransformSpace {
    World,
    Local,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AppTab {
    Preview,
    LodAudit,
    TextureReview,
    Scene,
    Vehicles,
    Validation,
    Editing,
    Collisions,
    Lights,
    Bake,
    Water,
    Cull,
    Race,
    Simulate,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum ViewportRenderMode {
    #[default]
    ShadedTextured,
    UnshadedTextured,
    CollisionClassification,
}

impl ViewportRenderMode {
    const ALL: [Self; 3] = [
        Self::ShadedTextured,
        Self::UnshadedTextured,
        Self::CollisionClassification,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::ShadedTextured => "Shaded + Textures",
            Self::UnshadedTextured => "Unshaded + Textures",
            Self::CollisionClassification => "Collision Classification",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PropertiesTab {
    Element,
    Settings,
    History,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum PhysicsScope {
    Global,
    #[default]
    PerObject,
}

#[derive(Clone, Debug, PartialEq)]
struct PhysicsRootProperties {
    mass: f32,
    turn_mass: f32,
    air_resistance: f32,
    elasticity: f32,
    buoyancy: f32,
    uproot_limit: f32,
    collision_damage_multiplier: f32,
    collision_damage_effect: i32,
    special_collision_response: i32,
    camera_avoid: bool,
    causes_explosion: bool,
    fx_type: i32,
    fx_offset: V3,
    fx_name: String,
    smash_multiplier: f32,
    break_velocity: V3,
    break_velocity_randomness: f32,
    gun_break_mode: i32,
    sparks_on_impact: bool,
}

impl PhysicsRootProperties {
    const fn new(
        mass: f32,
        turn_mass: f32,
        air_resistance: f32,
        elasticity: f32,
        buoyancy: f32,
    ) -> Self {
        Self {
            mass,
            turn_mass,
            air_resistance,
            elasticity,
            buoyancy,
            uproot_limit: 0.0,
            collision_damage_multiplier: 0.0,
            collision_damage_effect: 0,
            special_collision_response: 0,
            camera_avoid: false,
            causes_explosion: false,
            fx_type: 0,
            fx_offset: V3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            fx_name: String::new(),
            smash_multiplier: 0.0,
            break_velocity: V3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            break_velocity_randomness: 0.0,
            gun_break_mode: 0,
            sparks_on_impact: false,
        }
    }

    fn is_breakable(&self) -> bool {
        matches!(self.collision_damage_effect, 200 | 202)
    }

    const fn with_breakable_behavior(
        mut self,
        uproot_limit: f32,
        special_collision_response: i32,
        smash_multiplier: f32,
        break_velocity: V3,
        break_velocity_randomness: f32,
        gun_break_mode: i32,
        sparks_on_impact: bool,
    ) -> Self {
        self.uproot_limit = uproot_limit;
        self.collision_damage_multiplier = 1.0;
        self.collision_damage_effect = 200;
        self.special_collision_response = special_collision_response;
        self.smash_multiplier = smash_multiplier;
        self.break_velocity = break_velocity;
        self.break_velocity_randomness = break_velocity_randomness;
        self.gun_break_mode = gun_break_mode;
        self.sparks_on_impact = sparks_on_impact;
        self
    }

    const fn with_damage_behavior(
        mut self,
        uproot_limit: f32,
        collision_damage_effect: i32,
        special_collision_response: i32,
        smash_multiplier: f32,
        break_velocity: V3,
        break_velocity_randomness: f32,
        gun_break_mode: i32,
        sparks_on_impact: bool,
    ) -> Self {
        self.uproot_limit = uproot_limit;
        self.collision_damage_multiplier = 1.0;
        self.collision_damage_effect = collision_damage_effect;
        self.special_collision_response = special_collision_response;
        self.smash_multiplier = smash_multiplier;
        self.break_velocity = break_velocity;
        self.break_velocity_randomness = break_velocity_randomness;
        self.gun_break_mode = gun_break_mode;
        self.sparks_on_impact = sparks_on_impact;
        self
    }
}

#[derive(Clone)]
struct PhysicsRootSpec {
    label: &'static str,
    model_id: u16,
    object_name: &'static str,
    fallback: PhysicsRootProperties,
}

impl PhysicsRootSpec {
    const fn new(
        label: &'static str,
        model_id: u16,
        object_name: &'static str,
        fallback: PhysicsRootProperties,
    ) -> Self {
        Self {
            label,
            model_id,
            object_name,
            fallback,
        }
    }
}

impl PhysicsScope {
    fn label(self) -> &'static str {
        match self {
            Self::Global => "Global",
            Self::PerObject => "Per Object",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GizmoAxis {
    X,
    Y,
    Z,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GizmoPlane {
    XY,
    XZ,
    YZ,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GizmoTarget {
    Element,
    CullZone,
    Light,
    CollisionVertex,
    CollisionPrimitive,
    DffVertex,
    DffPivot,
    DffBooleanBox,
    Dff2dEffect,
    RacePoint,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct DffFreeformPivot {
    position: V3,
    rotation: V3,
}

struct GizmoDrag {
    target: GizmoTarget,
    axis: GizmoAxis,
    start_mouse: Vec2,
    start_pos: V3,
    start_rot: V3,
    element_start_positions: Vec<(usize, V3)>,
    element_start_rots: Vec<(usize, V3)>,
    scale_plane: Option<GizmoPlane>,
    dff_start_vertices: Vec<(usize, V3)>,
    before: ScopedHistorySnapshot,
    label: String,
}

struct DffScaleInput {
    axis: Option<GizmoAxis>,
    numeric_input: String,
    pivot: V3,
    start_vertices: Vec<(usize, V3)>,
    before: EditingHistorySnapshot,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WaterEdge {
    North,
    South,
    East,
    West,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WaterSplitAxis {
    X,
    Y,
}

struct WaterEdgeDrag {
    plane: usize,
    edge: WaterEdge,
    start_value: f32,
    before: WaterHistorySnapshot,
}

#[derive(Clone, Debug, PartialEq)]
struct CullZone {
    id: String,
    center: V3,
    size: V3,
}

// Drag-resize one face of the selected axis-aligned Cull zone while keeping
// the opposite face fixed.
struct CullFacePick {
    axis: usize,
    side_is_max: bool,
    center: Vec3,
    half_extents: Vec3,
    axis_dir: Vec3,
    face_origin: Vec3,
}

struct CullFaceDrag {
    zone: usize,
    axis: usize,
    side_is_max: bool,
    center: Vec3,
    half_extents: Vec3,
    axis_dir: Vec3,
    face_origin: Vec3,
    start_mouse: Vec2,
    before: CullHistorySnapshot,
}

// Drag-resize a single face of a selected collision box (Editing tab).
// axis: 0=x, 1=y, 2=z in macroquad space. side_is_max: true if dragging the
// high-coordinate face on that axis, false for the low-coordinate face.
struct ColBoxFacePick {
    primitive: CollisionPrimitiveSelection,
    axis: usize,
    side_is_max: bool,
    center: Vec3,
    half_extents: Vec3,
    axis_dir: Vec3,
    face_origin: Vec3,
}

struct ColBoxFaceDrag {
    primitive: CollisionPrimitiveSelection,
    axis: usize,
    side_is_max: bool,
    center: Vec3,
    half_extents: Vec3,
    axis_dir: Vec3,
    face_origin: Vec3,
    start_mouse: Vec2,
    before: EditingHistorySnapshot,
}

#[derive(Clone, Copy)]
enum Race2dPointDrag {
    Overlay {
        track: usize,
        point: usize,
        before: V3,
    },
    Checkpoint {
        track: usize,
        point: usize,
        before: RaceCheckpoint,
    },
}

#[derive(Clone)]
struct SavedContentSnapshot {
    placements: Vec<Placement>,
    definitions: HashMap<String, Definition>,
    readonly_definition_ids: HashSet<String>,
    element_states: Vec<ElementState>,
    lights: Vec<EditorLight>,
    water_planes: Vec<WaterPlane>,
    cull_zones: Vec<CullZone>,
    race_tracks: Vec<RaceTrack>,
    race_radar_path: String,
    race_world_size: f32,
    race_world_center_x: f32,
    race_world_center_y: f32,
    collisions: HashMap<String, CollisionMesh>,
    editing_asset: Option<EditingAsset>,
    editing_modified_entries: BTreeMap<String, Vec<u8>>,
    editing_deleted_entries: BTreeSet<String>,
    editing_added_entries: BTreeSet<String>,
}

#[derive(Clone)]
struct AutosaveDirtySnapshot {
    content: SavedContentSnapshot,
    material_classes: TextureMaterialClasses,
    material_classes_dirty: bool,
    safe_collisions: SafeCollisions,
    safe_collisions_dirty: bool,
    pending_replacement_assets: BTreeMap<String, (String, Vec<u8>)>,
    pending_col_writes: HashMap<(PathBuf, u64), u8>,
    pending_txd_writes: HashSet<String>,
    pending_asset_deletes: HashSet<String>,
    pending_vertex_light_meshes: HashSet<String>,
}

#[derive(Clone, PartialEq)]
struct WaterHistorySnapshot {
    planes: Vec<WaterPlane>,
    selected: usize,
    selected_planes: BTreeSet<usize>,
}

#[derive(Clone, PartialEq)]
struct CullHistorySnapshot {
    zones: Vec<CullZone>,
    selected: usize,
}

#[derive(Clone, PartialEq)]
struct LightHistorySnapshot {
    lights: Vec<EditorLight>,
    selected: usize,
}

#[derive(Clone, PartialEq)]
struct RaceHistorySnapshot {
    tracks: Vec<RaceTrack>,
    selected_track: usize,
    place_mode: RacePlaceMode,
    selected_point: Option<usize>,
    place_z: f32,
    default_radius: f32,
    radar_path: String,
    world_size: f32,
    world_center_x: f32,
    world_center_y: f32,
}

#[derive(Clone, PartialEq)]
struct WorldHistorySnapshot {
    placements: Vec<Placement>,
    definitions: HashMap<String, Definition>,
    readonly_definition_ids: HashSet<String>,
    element_states: Vec<ElementState>,
    selected: usize,
    selected_elements: BTreeSet<usize>,
    selected_element_order: Vec<usize>,
    selected_col_face: Option<SelectedCollisionFace>,
    selected_col_vertex: usize,
}

/// The placement subset touched by an in-place world transform.
///
/// Unlike `WorldHistorySnapshot`, this deliberately excludes definitions and
/// unrelated placements so frequent nudges and gizmo drags have history cost
/// proportional to the selection size.
#[derive(Clone, PartialEq)]
struct PlacementTransformHistorySnapshot {
    placements: Vec<(usize, Placement)>,
}

#[derive(Clone, PartialEq)]
struct SelectionHistorySnapshot {
    selected: usize,
    selected_elements: BTreeSet<usize>,
    selected_element_order: Vec<usize>,
    selected_group: Option<String>,
}

#[derive(Clone, PartialEq)]
struct CollisionHistorySnapshot {
    collisions: HashMap<String, CollisionMesh>,
    selected_col_face: Option<SelectedCollisionFace>,
    selected_col_vertex: usize,
    pending_writes: HashMap<(PathBuf, u64), u8>,
    pending_replacements: BTreeMap<String, (String, Vec<u8>)>,
}

#[derive(Clone)]
struct EditingHistorySnapshot {
    asset: Option<EditingAsset>,
    modified_entries: BTreeMap<String, Vec<u8>>,
    deleted_entries: BTreeSet<String>,
    added_entries: BTreeSet<String>,
    material_emitters: Option<HashMap<String, MaterialEmitter>>,
    shadow_casting: Option<HashMap<String, bool>>,
}

#[derive(Clone)]
struct GlobalTransformHistorySnapshot {
    world: WorldHistorySnapshot,
    lights: LightHistorySnapshot,
    race: RaceHistorySnapshot,
    water: WaterHistorySnapshot,
}

#[derive(Clone)]
struct WorldEditingHistorySnapshot {
    world: WorldHistorySnapshot,
    editing: EditingHistorySnapshot,
}

#[derive(Clone, PartialEq)]
struct WorldRaceHistorySnapshot {
    world: WorldHistorySnapshot,
    race: RaceHistorySnapshot,
}

#[derive(Clone)]
struct LodGenerationHistoryArtifact {
    assets: Vec<(String, String, Vec<u8>)>,
    mesh_key: String,
    mesh: Option<RenderMesh>,
    collision_key: String,
    collision: CollisionMesh,
}

#[derive(Clone)]
struct LodGenerationHistorySnapshot {
    world: WorldHistorySnapshot,
    artifacts: Arc<Vec<LodGenerationHistoryArtifact>>,
    generated_present: bool,
    /// Shared LOD dictionaries the batch wrote to, keyed by asset key, holding
    /// the bytes for this side of the edit. `None` means the dictionary did not
    /// exist yet.
    txd_assets: BTreeMap<String, (String, Option<Vec<u8>>)>,
}

enum UndoState {
    Water(WaterHistorySnapshot),
    Cull(CullHistorySnapshot),
    Lights(LightHistorySnapshot),
    Race(RaceHistorySnapshot),
    World(WorldHistorySnapshot),
    PlacementTransforms(PlacementTransformHistorySnapshot),
    Selection(SelectionHistorySnapshot),
    Collision(CollisionHistorySnapshot),
    Editing(EditingHistorySnapshot),
    GlobalTransform(GlobalTransformHistorySnapshot),
    VertexColors(VertexColorHistorySnapshot),
    WorldEditing(WorldEditingHistorySnapshot),
    WorldRace(WorldRaceHistorySnapshot),
    LodGeneration(LodGenerationHistorySnapshot),
}

enum ScopedHistorySnapshot {
    None,
    Water(WaterHistorySnapshot),
    Cull(CullHistorySnapshot),
    Lights(LightHistorySnapshot),
    Race(RaceHistorySnapshot),
    World(WorldHistorySnapshot),
    PlacementTransforms(PlacementTransformHistorySnapshot),
    Collision(CollisionHistorySnapshot),
    Editing(EditingHistorySnapshot),
    WorldEditing(WorldEditingHistorySnapshot),
    WorldRace(WorldRaceHistorySnapshot),
}

#[derive(Clone, Copy)]
struct GlobalTransformState {
    offset: V3,
    rotation: V3,
    preview: bool,
    transform_elements: bool,
    transform_water: bool,
}

impl Default for GlobalTransformState {
    fn default() -> Self {
        Self {
            offset: V3::default(),
            rotation: V3::default(),
            preview: false,
            transform_elements: true,
            transform_water: true,
        }
    }
}

// Packed day+night prelight colors (RGB8 each) per mesh part vertex.
#[derive(Clone, PartialEq)]
struct VertexColorSnapshot {
    meshes: HashMap<String, Vec<Vec<[u8; 6]>>>,
}

#[derive(Clone, PartialEq)]
struct VertexColorHistorySnapshot {
    colors: VertexColorSnapshot,
}

fn color_channel_to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn color_channel_from_u8(value: u8) -> f32 {
    value as f32 / 255.0
}

fn pack_vertex_prelight(vertex: &Vertex) -> [u8; 6] {
    [
        color_channel_to_u8(vertex.day_color.x),
        color_channel_to_u8(vertex.day_color.y),
        color_channel_to_u8(vertex.day_color.z),
        color_channel_to_u8(vertex.night_color.x),
        color_channel_to_u8(vertex.night_color.y),
        color_channel_to_u8(vertex.night_color.z),
    ]
}

fn unpack_vertex_prelight(vertex: &mut Vertex, packed: [u8; 6]) {
    vertex.day_color = V3 {
        x: color_channel_from_u8(packed[0]),
        y: color_channel_from_u8(packed[1]),
        z: color_channel_from_u8(packed[2]),
    };
    vertex.night_color = V3 {
        x: color_channel_from_u8(packed[3]),
        y: color_channel_from_u8(packed[4]),
        z: color_channel_from_u8(packed[5]),
    };
}

struct UndoEntry {
    label: String,
    before: UndoState,
    after: UndoState,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum InspectorField {
    ElementId,
    ElementPosX,
    ElementPosY,
    ElementPosZ,
    ElementRotX,
    ElementRotY,
    ElementRotZ,
    ElementDimension,
    ElementInterior,
    ElementLodParent,
    ElementUniqueId,
    ElementAlpha,
    ElementScale,
    PhysicsSimulated,
    PhysicsMass,
    PhysicsTurnMass,
    PhysicsAirResistance,
    PhysicsElasticity,
    PhysicsBuoyancy,
    PhysicsCenterOfMassX,
    PhysicsCenterOfMassY,
    PhysicsCenterOfMassZ,
    DefinitionDff,
    DefinitionNativeModel,
    DefinitionTxd,
    DefinitionCol,
    DefinitionLod,
    DefinitionTimeIn,
    DefinitionTimeOut,
    CollisionFaceMaterial,
    CollisionFaceLight,
    CollisionVertexX,
    CollisionVertexY,
    CollisionVertexZ,
    CollisionPrimitiveSizeX,
    CollisionPrimitiveSizeY,
    CollisionPrimitiveSizeZ,
    CollisionPrimitiveRotX,
    CollisionPrimitiveRotY,
    CollisionPrimitiveRotZ,
    #[allow(dead_code)] // Material RGBA is edited with drag bars, not text fields.
    DffMaterialRed,
    #[allow(dead_code)]
    DffMaterialGreen,
    #[allow(dead_code)]
    DffMaterialBlue,
    #[allow(dead_code)]
    DffMaterialAlpha,
    DffMaterialAmbient,
    DffMaterialDiffuse,
    DffMaterialSpecular,
    DffEmitterStrength,
    DffEmitterFalloff,
    DffEmitterMaxGroupingSize,
    DffEmitterPointUpStrength,
    DffEmitterPointDownStrength,
    DffEmitterPointSidesStrength,
    DffEmitterTemperature,
    BakeShadowSamples,
    BakeShadowChunks,
    BakeBounces,
    BakeBounceStrength,
    BakeBounceMaximum,
    BakeExposure,
    BakeAmbientBump,
    BakeShadowSoftness,
    BakeAoSamples,
    BakeAoRadius,
    BakeAoStrength,
    DayNightMergeTolerance,
    VertexPaintTemperature,
    VertexPaintRadius,
    VertexPaintStrength,
    SnapMove,
    SnapRotate,
    GlobalOffsetX,
    GlobalOffsetY,
    GlobalOffsetZ,
    EagleOffsetX,
    EagleOffsetY,
    EagleOffsetZ,
    EagleWaterOffsetX,
    EagleWaterOffsetY,
    EagleWaterOffsetZ,
    GlobalRotationX,
    GlobalRotationY,
    GlobalRotationZ,
    LightName,
    LightKind,
    LightProfile,
    LightPosition,
    LightDirection,
    LightTemperature,
    LightIntensity,
    LightRadius,
    WaterMinX,
    WaterMinY,
    WaterMaxX,
    WaterMaxY,
    WaterHeight,
    WaterType,
    CullPosX,
    CullPosY,
    CullPosZ,
    CullSizeX,
    CullSizeY,
    CullSizeZ,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LightKind {
    Ambient,
    Directional,
    Point,
    Spot,
    /// Internal quadrature sample representing a measured patch of an
    /// emitting triangle. This is not exposed as a placeable editor light.
    Area,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PointLightLobe {
    Omni,
    Up,
    Down,
    Sides,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LightProfile {
    Day,
    Night,
    Both,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BakeLightMode {
    Day,
    Night,
    Both,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BakeQualityPreset {
    Preview,
    Balanced,
    Final,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BakeBackend {
    Cpu,
    Gpu,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BakeScope {
    WholeScene,
    Selected,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum VertexLightTool {
    Paint,
    Sample,
}

#[derive(Clone, Copy)]
struct VertexPaintSettings {
    enabled: bool,
    tool: VertexLightTool,
    color: V3,
    temperature: f32,
    radius: f32,
    strength: f32,
}

#[derive(Clone, Copy)]
struct BakeSettings {
    preset: BakeQualityPreset,
    backend: BakeBackend,
    light_mode: BakeLightMode,
    scope: BakeScope,
    face_emitters_enabled: bool,
    shadow_samples: usize,
    shadow_chunks: usize,
    bounces: usize,
    bounce_strength: f32,
    bounce_maximum: f32,
    exposure: f32,
    ambient_bump: f32,
    shadow_softness: f32,
    ao_samples: usize,
    ao_radius: f32,
    ao_strength: f32,
    day_night_merge_tolerance: f32,
}

#[derive(Clone, PartialEq)]
struct EditorLight {
    name: String,
    /// Model id this light is attached to. Attached light transforms are stored
    /// in model-local space and expanded over every live instance at runtime.
    attached_to: Option<String>,
    kind: LightKind,
    profile: LightProfile,
    position: V3,
    direction: V3,
    color: V3,
    temperature: f32,
    use_temperature: bool,
    intensity: f32,
    radius: f32,
    casts_shadow: bool,
    point_lobe: PointLightLobe,
}

const DEFAULT_EDITOR_LIGHT_RADIUS: f32 = 50.0;

#[derive(Clone, Copy, PartialEq)]
struct WaterCorner {
    pos: V3,
    wave_x: f32,
    wave_y: f32,
    speed: f32,
    unknown: f32,
}

#[derive(Clone, PartialEq)]
struct WaterPlane {
    corners: [WaterCorner; 4],
    kind: i32,
}

struct InspectorEdit {
    field: InspectorField,
    buffer: String,
    cursor: usize,
    selection_anchor: Option<usize>,
    before: ScopedHistorySnapshot,
}

struct GroupRenameEdit {
    original: String,
    buffer: String,
    cursor: usize,
    selection_anchor: Option<usize>,
    before: WorldHistorySnapshot,
}

/// What a race-name text edit targets.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RaceNameTarget {
    Track,
    Subtrack,
}

/// Inline text edit for renaming the selected race track or its active subtrack.
struct RaceNameEdit {
    target: RaceNameTarget,
    buffer: String,
    cursor: usize,
    selection_anchor: Option<usize>,
    before: RaceHistorySnapshot,
}

struct OutlinerScrollDrag {
    grab_offset: f32,
}

#[derive(Clone, PartialEq, Eq)]
enum OutlinerEntry {
    Group(String),
    GroupChild(usize),
    Element(usize),
}

#[derive(Clone, Copy)]
struct BoxSelectDrag {
    start: Vec2,
    current: Vec2,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BoxSelectMode {
    Add,
    Subtract,
    Toggle,
}

impl BoxSelectMode {
    fn next(self) -> Self {
        match self {
            Self::Add => Self::Subtract,
            Self::Subtract => Self::Toggle,
            Self::Toggle => Self::Add,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Add => "Add",
            Self::Subtract => "Subtract",
            Self::Toggle => "Toggle",
        }
    }

    fn verb(self) -> &'static str {
        match self {
            Self::Add => "added",
            Self::Subtract => "removed",
            Self::Toggle => "toggled",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ContextAction {
    ViewAssetPreview,
    AddLight,
    AddLightToInstance,
    CopyId,
    CopyDff,
    ExportDff,
    ExportCol,
    ReplaceDff,
    ReplaceCol,
    BlenderPosition,
    CopyPosition,
    CopyMapTag,
    SnapTo,
    HideUnhide,
    HideEverythingBut,
    UnhideAll,
    Join,
    Duplicate,
    DeleteRestore,
    Deselect,
    SelectMatchingTextureName,
    SelectMatchingTextureContent,
    AddTextureToWater,
    LodAuditGenerate,
    LodAuditToggleIgnore,
    ViewDffMaterialTexture,
    ExportDffMaterialTexture,
    ReplaceDffMaterialTexture,
    SelectAllDffMaterialFaces,
    MarkDffTextureElementsDoubleSided,
    ToggleDffMaterialEmitter,
    ClearDffFaceLighting,
    CategorizeTexture,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum InspectorCopyAction {
    Field(InspectorField),
    ElementPosition,
    ElementRotation,
    LightPosition,
    LightRotation,
}

struct ContextMenu {
    pos: Vec2,
    target: ContextMenuTarget,
}

#[derive(Clone)]
enum ContextMenuTarget {
    Selection,
    Scene {
        position: V3,
    },
    LodAuditIssue(usize),
    PreviewTexture {
        placement: usize,
        material: usize,
    },
    EditingDffMaterial {
        dff_name: String,
        material: usize,
    },
    EditingDffFaceLighting {
        dff_name: String,
        emitter_key: String,
    },
    EditingTexture {
        txd_name: String,
        texture_name: String,
    },
    AssetBrowser {
        entry_id: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TextureMatchMode {
    Name,
    Content,
}

#[derive(Clone)]
struct TextureMatchCandidate {
    placement: usize,
    textures: Vec<(String, Option<TextureContentFingerprint>)>,
}

struct TextureMatchSelectionJob {
    mode: TextureMatchMode,
    target_name: String,
    target_fingerprint: Option<TextureContentFingerprint>,
    snapshot_index: usize,
    candidates: Vec<TextureMatchCandidate>,
    rx: Option<mpsc::Receiver<Result<Vec<usize>, String>>>,
    matches: Option<Vec<usize>>,
    apply_index: usize,
    newly_selected: usize,
    selection_before: Option<SelectionHistorySnapshot>,
}

#[derive(Clone)]
enum WaterTextureDffSource {
    Raw(Box<RawMesh>),
    Bytes(Vec<u8>),
    Entry(ImgEntry),
    Archive {
        root: PathBuf,
        gta_sa_dir: PathBuf,
        dff_name: String,
    },
}

#[derive(Clone)]
struct PreviewWorldUvWorkerTarget {
    dff_name: String,
    placement: Placement,
    materials: BTreeSet<usize>,
    source: WaterTextureDffSource,
    write_options: DffWriteOptions,
}

struct PreviewWorldUvRewrite {
    dff_name: String,
    dff_key: String,
    raw: RawMesh,
    bytes: Vec<u8>,
    materials: usize,
    uvs: usize,
}

struct PreviewWorldUvWorkerResult {
    rewrites: VecDeque<PreviewWorldUvRewrite>,
    skipped_assets: usize,
}

struct PreviewWorldUvJob {
    texture_name: String,
    multiplier: f32,
    variation: bool,
    all_dffs: bool,
    snapshot_index: usize,
    targets_by_dff: BTreeMap<String, (Placement, BTreeSet<usize>)>,
    targets: Vec<(String, Placement, BTreeSet<usize>)>,
    source_index: usize,
    worker_targets: Vec<PreviewWorldUvWorkerTarget>,
    skipped_readonly: usize,
    rx: Option<mpsc::Receiver<Result<PreviewWorldUvWorkerResult, String>>>,
    result: Option<PreviewWorldUvWorkerResult>,
    staged_dffs: usize,
    staged_materials: usize,
    staged_uvs: usize,
}

struct PreviewWorldUvVisual {
    dff_name: String,
    placement_index: usize,
    material: usize,
    original_raw: RawMesh,
}

struct WaterTextureConversionResult {
    dff_name: String,
    raw: RawMesh,
    dff_bytes: Vec<u8>,
    water_planes: Vec<WaterPlane>,
    removed_faces: usize,
    texture_name: String,
}

struct WaterTextureConversionJob {
    dff_name: String,
    dff_key: String,
    material: usize,
    texture_name: String,
    source: Option<WaterTextureDffSource>,
    write_options: DffWriteOptions,
    snapshot_index: usize,
    placement_matrices: Vec<[f32; 16]>,
    txd_scopes: BTreeSet<Option<String>>,
    rx: Option<mpsc::Receiver<Result<WaterTextureConversionResult, String>>>,
    result: Option<WaterTextureConversionResult>,
    apply_phase: u8,
    refresh_scopes: Vec<Option<String>>,
    refresh_index: usize,
    refreshed_any: bool,
    started_at: Instant,
}

struct SaveAsDialog {
    path: String,
    cursor: usize,
    selection_anchor: Option<usize>,
}

struct LoadDialog {
    path: String,
    cursor: usize,
    selection_anchor: Option<usize>,
}

struct ImportAssetDialog {
    dff_path: PathBuf,
    texture_dir: PathBuf,
    id: String,
    cursor: usize,
}

struct PreferencesDialog {
    gta_sa_dir: String,
    /// Pending viewport values. They are staged here rather than applied live so
    /// Cancel discards them along with the path edit.
    gizmo_scale: f32,
    camera_speed: f32,
    vehicle_camera_speed: f32,
    editing_camera_speed: f32,
    camera_rotation_speed: f32,
    msaa_samples: i32,
    draw_distance_percent: u16,
    /// MSAA as it was when the dialog opened, so "needs a restart" can be
    /// detected without re-reading preferences.txt every frame.
    msaa_samples_saved: i32,
    cursor: usize,
    selection_anchor: Option<usize>,
}

#[derive(Clone, PartialEq, Eq)]
enum DffPickerKind {
    ImportNewAssetDff,
    ImportNewAssetTextures,
    ImportBlender,
    GenerateTxdFolder,
    GenerateTxdBuild {
        txd_names: Vec<String>,
        destination: TxdGenerationDestination,
    },
    Export,
    ExportCol,
    ExportNamedDff {
        dff_name: String,
        vehicle_txd: String,
    },
    ExportLooseFile {
        source_path: PathBuf,
        asset_name: String,
        vehicle_txd: String,
    },
    ExportTexturePng {
        texture_name: String,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    },
    ExportTexturePngs {
        textures: Vec<(String, u32, u32, Vec<u8>)>,
    },
    VehicleTextureReplace(VehicleTextureReplaceRequest),
    Replace,
    ReplaceCol,
    ImportPrelight,
    TextureAdd {
        definition_id: String,
        txd_name: String,
    },
    TextureReplace {
        definition_id: String,
        txd_name: String,
        texture_name: String,
    },
    EditingOpenFile,
    EditingOpenImg,
    EditingCreateDff,
    /// Pick a loose `.txd` to pair with the DFF open in the editor.
    EditingPairTxd,
    EditingMergeImg,
    EditingAddEntry,
    EditingReplaceEntry {
        entry_name: String,
    },
    EditingExtractEntry {
        entry_name: String,
    },
    EditingTextureReplace {
        entry_name: String,
        texture_name: String,
    },
    EditingTextureExportAll {
        entry_name: String,
    },
    EditingFaceTextureImport {
        txd_name: String,
    },
    EditingMaterialTextureImport {
        txd_name: String,
        dff_name: String,
        material: usize,
    },
    EditingMaterialTextureReplace {
        txd_name: String,
        dff_name: String,
        material: usize,
        texture_name: String,
    },
    EditingGifAnimImport {
        txd_name: String,
    },
    RaceRadar,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TxdGenerationDestination {
    Textures,
    Img,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ReplacementAssetKind {
    Dff,
    Col,
}

struct DffReplaceChoiceDialog {
    path: PathBuf,
    kind: ReplacementAssetKind,
}

struct DffPrelightImportDialog {
    img_path: PathBuf,
    entries: Vec<ImgEntry>,
    selected: usize,
    scroll: f32,
}

struct DffMergeChoiceDialog {
    selected_count: usize,
}

/// Toggle sheet shown by the DFF editor's "Optimize / Repair DFF" button.
///
/// The dialog only edits `options`; nothing is applied until the user presses
/// Run, so the toggles can be reviewed first.
struct DffOptimizeDialog {
    /// Name of the DFF the dialog was opened for, so a run is rejected if the
    /// editor moved on to a different asset while the sheet was open.
    dff_name: String,
    options: DffOptimizeOptions,
}

#[derive(Clone)]
struct OversizedChunkCandidate {
    dff_name: String,
    extents: V3,
    placement_count: usize,
    selected: bool,
    blocked_reason: Option<String>,
}

struct OversizedChunkDialog {
    chunk_size: String,
    chunk_size_cursor: usize,
    chunk_size_selection_anchor: Option<usize>,
    candidates: Vec<OversizedChunkCandidate>,
    scroll: f32,
    error: Option<String>,
}

struct OversizedChunkResult {
    scanned: usize,
    requested: usize,
    chunked: usize,
    assets: Vec<(String, Vec<u8>)>,
    definitions: Vec<(String, Definition)>,
    added_placements: Vec<(Placement, ElementState)>,
    errors: Vec<String>,
}

pub(crate) struct OversizedChunkJob {
    rx: mpsc::Receiver<OversizedChunkResult>,
    result: Option<OversizedChunkResult>,
    definitions_applied: bool,
    added_placement_index: usize,
    asset_index: usize,
    scene_rebuilt: bool,
    started_at: Instant,
}

/// Prompt shown when an opened DFF references textures that no loaded TXD
/// provides, offering to pair an external `.txd` with it.
struct DffTxdPairDialog {
    dff_name: String,
    /// Distinct texture names the DFF asks for and the editor cannot resolve.
    missing_textures: Vec<String>,
    /// Total material slots, for the "n of m" line.
    material_count: usize,
    scroll: f32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DffTextureNameAction {
    Duplicate,
    RenameDff,
    RenameTxd,
    SeparateGeometry,
}

struct DffTextureDuplicateDialog {
    action: DffTextureNameAction,
    material: usize,
    source_texture: String,
    buffer: String,
    cursor: usize,
    selection_anchor: Option<usize>,
}

struct DffTextureViewDialog {
    texture_name: String,
    txd_name: String,
    width: u16,
    height: u16,
    texture: Option<Texture2D>,
    raw_texture: u32,
    rgba: Option<Vec<u8>>,
}

struct ElementIdRenameDialog {
    selected_idx: usize,
    selected_indices: Vec<usize>,
    old_id: String,
    new_id: String,
    before: ScopedHistorySnapshot,
}

struct ElementReplaceWithDialog {
    source_indices: Vec<usize>,
    search: String,
    cursor: usize,
    selection_anchor: Option<usize>,
    scroll: f32,
    selected_target: Option<String>,
    picking_scene: bool,
}

#[derive(Clone)]
struct MissingTextureChoice {
    definition_id: String,
    texture_name: String,
    candidates: Vec<MissingTextureCandidate>,
}

#[derive(Clone)]
struct MissingTextureCandidate {
    txd_name: String,
    source_texture_name: String,
}

struct MissingTextureDialog {
    choice: MissingTextureChoice,
    candidates: Vec<MissingTextureCandidatePreview>,
    remaining: Vec<MissingTextureChoice>,
    applied: usize,
}

struct MissingTextureCandidatePreview {
    txd_name: String,
    source_texture_name: String,
    width: u16,
    height: u16,
    format: TxFormat,
    thumbnail: Option<Texture2D>,
    duplicate_count: usize,
}

struct TextureArchiveDialog {
    definition_id: String,
    txd_name: String,
    textures: Vec<TextureArchiveEntry>,
    selected: usize,
    scroll: f32,
    preview_texture: Option<Texture2D>,
}

#[derive(Clone)]
struct TextureArchiveEntry {
    name: String,
    width: u16,
    height: u16,
    format: TxFormat,
    fingerprint: Option<TextureContentFingerprint>,
    thumbnail: Option<Texture2D>,
}

#[derive(Clone)]
struct EditingImgRow {
    entry: ImgEntry,
    logical_size: usize,
}

#[derive(Clone)]
struct EditingImgMergeEntry {
    name: String,
    bytes: Vec<u8>,
    matches_existing: bool,
}

struct EditingImgMergePlan {
    source_path: PathBuf,
    entries: Vec<EditingImgMergeEntry>,
    matching_entries: usize,
}

struct EditingImgMergeApplyJob {
    source_path: PathBuf,
    entries: VecDeque<EditingImgMergeEntry>,
    overwrite_matching: bool,
    total_entries: usize,
    added: usize,
    replaced: usize,
    skipped: usize,
}

#[derive(Clone)]
struct EditingDffOpenModel {
    name: String,
    placement_index: usize,
    /// Converts this DFF's local coordinates into the first opened placement's
    /// local coordinate system. Keeping every open model in that common space
    /// makes viewport selection and cross-model chunk moves deterministic.
    to_workspace: Mat4,
    raw: RawMesh,
    preview_mesh: Option<RenderMesh>,
    txd_context: Option<String>,
    txd_source_label: String,
    dirty: bool,
    selected_face: Option<usize>,
    selected_faces: BTreeSet<usize>,
    selected_edges: BTreeSet<(usize, usize)>,
    selected_vertex: Option<usize>,
    selected_vertices: BTreeSet<usize>,
}

#[derive(Clone)]
struct EditingDffState {
    name: String,
    /// Preview-only GTA:SA assets can be inspected but never staged or saved.
    read_only: bool,
    raw: RawMesh,
    preview_mesh: Option<RenderMesh>,
    txd_context: Option<String>,
    txd_source_label: String,
    material_thumbnails: Vec<Option<Texture2D>>,
    uv_editor: DffUvEditorState,
    selected_material: usize,
    selected_breakable_group: usize,
    /// Main-thread-only timestamp for the visual fracture simulation. The
    /// preview never mutates or serializes the authored debris mesh.
    fracture_preview_started_at: Option<f64>,
    selected_face: Option<usize>,
    selected_faces: BTreeSet<usize>,
    /// When faces are selected, chooses whether the Material-tab emitter
    /// controls edit those faces or the selected material/texture.
    emitter_targets_faces: bool,
    selected_edges: BTreeSet<(usize, usize)>,
    select_mode: EditingSelectMode,
    selected_vertex: Option<usize>,
    selected_vertices: BTreeSet<usize>,
    selected_2dfx: Option<usize>,
    hovered_face: Option<usize>,
    hovered_vertex: Option<usize>,
    hovered_edge: Option<(usize, usize)>,
    show_normals: bool,
    boolean_box: Option<DffBooleanBox>,
    /// Interactive, preview-only pivot transform. Applying it rewrites the DFF
    /// coordinate system and compensates every referencing placement.
    freeform_pivot: Option<DffFreeformPivot>,
    dirty: bool,
    normalized_warning: bool,
    normalized_rewrite_confirmed: bool,
    material_scroll: f32,
    collision_material_picker_open: bool,
    collision_material_picker_search: String,
    collision_material_picker_scroll: f32,
    collision_material_picker_scope: CollisionMaterialAssignmentScope,
    texture_picker_open: bool,
    texture_picker_edits_material: bool,
    texture_picker_scroll: f32,
    texture_picker_search: String,
    texture_picker_category: String,
    texture_picker_entries: Vec<TextureArchiveEntry>,
    uv_anim_picker_open: bool,
    uv_anim_picker_search: String,
    uv_anim_picker_scroll: f32,
    dff_2dfx_type_picker_open: bool,
    dff_2dfx_type_picker_search: String,
    dff_2dfx_type_picker_scroll: f32,
    dff_2dfx_corona_preset_picker_open: bool,
    dff_2dfx_payload_editor_open: bool,
    dff_2dfx_payload_hex: String,
    dff_2dfx_payload_fields: Vec<String>,
    dff_2dfx_payload_active_field: Option<usize>,
    dff_2dfx_payload_field_scroll: f32,
    dff_2dfx_particle_picker_scroll: f32,
    /// The currently visible DFF editor category (Material, UV, Mesh, etc.).
    /// Kept separately from section collapse state so switching tabs does not
    /// discard the user's expanded sections.
    panel_tab: usize,
    panel_scroll: f32,
    panel_collapsed: [bool; DFF_SECTION_COUNT],
    /// Scene-launched DFFs that share this editing workspace. `raw` mirrors
    /// `open_models[active_open_model].raw`; the other entries are contextual
    /// models rendered at their placement-relative offsets.
    open_models: Vec<EditingDffOpenModel>,
    active_open_model: usize,
    multi_select: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DffUvTransformMode {
    Grab,
    Scale,
    Rotate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DffUvAxis {
    Free,
    U,
    V,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DffUvMenu {
    Select,
    View,
    Snap,
    Align,
    Display,
}

#[derive(Clone)]
struct DffUvTransform {
    mode: DffUvTransformMode,
    axis: DffUvAxis,
    start_mouse: Vec2,
    start_uvs: Vec<(usize, V2)>,
    /// Typed scale multiplier while a Scale transform is active.
    numeric_input: String,
}

#[derive(Clone)]
struct DffUvEditorState {
    open: bool,
    /// Which half of the split workspace owns WASDEQ keyboard input.
    viewport_keyboard_focus: bool,
    selected: BTreeSet<usize>,
    hovered: Option<usize>,
    zoom: f32,
    pan: Vec2,
    panning: bool,
    pan_last: Vec2,
    /// Height of the UV split as a fraction of the center editing viewport.
    split_fraction: f32,
    resizing_split: bool,
    box_start: Option<Vec2>,
    transform: Option<DffUvTransform>,
    menu: Option<DffUvMenu>,
    snap_grid: bool,
    snap_vertices: bool,
    repeat_texture: bool,
    /// Show non-selected material/mesh edges while working in the UV editor.
    show_material_outlines: bool,
    /// Show the yellow outline around the active material in the 3D preview.
    show_material_selection_outlines: bool,
    texture_aspect: f32,
    grid_step: f32,
    before_uvs: Option<Vec<V2>>,
    before_dirty: bool,
}

impl Default for DffUvEditorState {
    fn default() -> Self {
        Self {
            open: false,
            viewport_keyboard_focus: false,
            selected: BTreeSet::new(),
            hovered: None,
            zoom: 1.0,
            pan: Vec2::ZERO,
            panning: false,
            pan_last: Vec2::ZERO,
            split_fraction: 0.58,
            resizing_split: false,
            box_start: None,
            transform: None,
            menu: None,
            snap_grid: false,
            snap_vertices: false,
            repeat_texture: false,
            show_material_outlines: true,
            show_material_selection_outlines: true,
            texture_aspect: 1.0,
            grid_step: 0.125,
            before_uvs: None,
            before_dirty: false,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
struct DffBooleanBox {
    center: V3,
    half_extents: V3,
}

#[derive(Clone)]
struct EditingTxdState {
    name: String,
    textures: Vec<TextureArchiveEntry>,
    selected: usize,
    scroll: f32,
    search: String,
    search_cursor: usize,
    search_anchor: Option<usize>,
    search_active: bool,
    category_filter: String,
    preview_texture: Option<Texture2D>,
    preview_zoom: f32,
    preview_pan: Vec2,
    preview_dragging: bool,
    preview_drag_last: Vec2,
    material_picker_open: bool,
    material_picker_search: String,
    material_picker_scroll: f32,
    material_picker_scope: CollisionMaterialAssignmentScope,
}

#[derive(Clone)]
struct TextureCategoryMenu {
    txd_name: String,
    texture_name: String,
    position: Vec2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CollisionMaterialAssignmentScope {
    ExactTxd,
    IdenticalContent,
    GlobalName,
}

impl CollisionMaterialAssignmentScope {
    const ALL: [Self; 3] = [Self::ExactTxd, Self::IdenticalContent, Self::GlobalName];

    fn label(self) -> &'static str {
        match self {
            Self::ExactTxd => "Exact TXD",
            Self::IdenticalContent => "Identical Content",
            Self::GlobalName => "Global Name",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EditingSelectMode {
    Vertex,
    Edge,
    Face,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CollisionPrimitiveKind {
    Sphere,
    Box,
    Cuboid,
    Capsule,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct CollisionPrimitiveSelection {
    kind: CollisionPrimitiveKind,
    index: usize,
}

#[derive(Clone)]
struct EditingColState {
    name: String,
    mesh: CollisionMesh,
    bytes: Vec<u8>,
    /// Exact internal model selected when this packed COL entry was opened.
    /// Staging revalidates this identity before replacing any bytes.
    source_model: ColModelIdentity,
    /// This COL came from the Rockstar collision plug-in embedded in a vehicle
    /// DFF. Staging it rewrites that plug-in instead of creating a `.col` entry.
    embedded_vehicle_dff: bool,
    /// Original/current owner DFF bytes for an embedded vehicle collision.
    /// This is bound at open time so changing archive selection cannot redirect
    /// a later stage or autosave to an unrelated DFF.
    embedded_source_dff_bytes: Option<Vec<u8>>,
    dff_overlay: Option<RenderMesh>,
    dff_overlay_name: Option<String>,
    dff_overlay_visible: bool,
    /// The existing COL editing tools always operate on `mesh.vertices` and
    /// `mesh.faces`. While this is true, those fields contain the shadow mesh
    /// and the regular collision triangles are parked in the corresponding
    /// `shadow_*` fields. `normalized_editing_col_mesh` swaps them back before
    /// serialization or use as a generation source.
    editing_shadow: bool,
    selected_face: usize,
    selected_faces: BTreeSet<usize>,
    selected_edges: BTreeSet<(usize, usize)>,
    select_mode: EditingSelectMode,
    face_scroll: f32,
    primitive_scroll: f32,
    selected_vertex: usize,
    selected_primitive: Option<CollisionPrimitiveSelection>,
    /// Logical capsule sources for the generated native COL representation.
    /// These definitions are persisted in EagleScene's collisionCapsules section.
    capsules: Vec<CollisionCapsule>,
    /// Rotated boxes materialized as closed six-plane triangle cuboids.
    cuboids: Vec<CollisionCuboid>,
    box_pick_enabled: bool,
    selected_vertices: BTreeSet<usize>,
    hovered_face: Option<usize>,
    hovered_vertex: Option<usize>,
    dirty: bool,
    panel_scroll: f32,
    panel_collapsed: [bool; COL_SECTION_COUNT],
}

#[derive(Clone)]
enum EditingAsset {
    Txd(EditingTxdState),
    Dff(EditingDffState),
    Col(EditingColState),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EditingNestedScrollFocus {
    DffMaterials,
    ColPrimitives,
    ColFaces,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EditingScrollbarDrag {
    ImgArchive,
    DffMaterials,
    DffPanel,
    ColPrimitives,
    ColFaces,
    ColPanel,
    TxdTextures,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditingLinkedAssetKind {
    Dff,
    Col,
}

struct EditingLinkedSelectionJob {
    kind: EditingLinkedAssetKind,
    asset_name: String,
    seed_face: usize,
    deselect: bool,
    face_count: usize,
    snapshot_index: usize,
    face_positions: Vec<[[i32; 3]; 3]>,
    rx: Option<mpsc::Receiver<Result<Vec<usize>, String>>>,
    result: Option<Vec<usize>>,
    apply_index: usize,
    apply_started: bool,
    clear_existing: bool,
    changed: usize,
    started_at: Instant,
}

struct EditingState {
    img_paths: Vec<PathBuf>,
    selected_img_path: usize,
    archive_picker_open: bool,
    img_path: Option<PathBuf>,
    rows: Vec<EditingImgRow>,
    selected_row: usize,
    scroll: f32,
    search: String,
    search_cursor: usize,
    search_anchor: Option<usize>,
    search_active: bool,
    modified_entries: BTreeMap<String, Vec<u8>>,
    deleted_entries: BTreeSet<String>,
    added_entries: BTreeSet<String>,
    asset: Option<EditingAsset>,
    camera: Option<CameraState>,
    camera_mode: Option<CameraMode>,
    camera_focus: Option<Option<Vec3>>,
    message: String,
    save_rx: Option<mpsc::Receiver<Result<EditingImgSaveOutcome, String>>>,
    merge_rx: Option<mpsc::Receiver<Result<EditingImgMergePlan, String>>>,
    merge_apply_job: Option<EditingImgMergeApplyJob>,
    txd_import_rx: Option<mpsc::Receiver<EditingTxdImportResult>>,
    dff_texture_replace_rx: Option<mpsc::Receiver<EditingDffTextureReplaceResult>>,
    txd_source_picker_rx: Option<mpsc::Receiver<EditingTxdSourcePickerResult>>,
    txd_refresh_job: Option<EditingTxdRefreshJob>,
    linked_selection_job: Option<EditingLinkedSelectionJob>,
    /// Child lists must be clicked before their wheel input takes precedence
    /// over their containing editor panel.
    nested_scroll_focus: Option<EditingNestedScrollFocus>,
    scrollbar_drag: Option<EditingScrollbarDrag>,
    scrollbar_drag_grab_offset_y: f32,
    texture_category_menu: Option<TextureCategoryMenu>,
}

struct EditingTxdImportResult {
    entry_name: String,
    target_texture: String,
    imported_count: usize,
    linked_definition_ids: Vec<String>,
    result: Result<EditingTxdImportOutput, String>,
}

struct EditingDffTextureReplaceResult {
    txd_name: String,
    dff_name: String,
    material: usize,
    texture_name: String,
    source_path: PathBuf,
    linked_definition_ids: Vec<String>,
    result: Result<EditingDffTextureReplaceOutput, String>,
}

struct EditingDffTextureReplaceOutput {
    indexed_textures: Vec<(String, Vec<TxdTexture>)>,
    replacement_txd_names: HashSet<String>,
}

#[derive(Clone, Copy)]
enum EditingTxdSourcePickerKind {
    Files,
    Folder,
}

struct EditingTxdSourcePickerResult {
    entry_name: String,
    kind: EditingTxdSourcePickerKind,
    result: Result<Option<Vec<PathBuf>>, String>,
}

struct EditingTxdImportOutput {
    updated: Vec<u8>,
    indexed_textures: Vec<(String, Vec<TxdTexture>)>,
    replacement_txd_names: HashSet<String>,
}

struct EditingTxdRefreshJob {
    txd_name: String,
    texture_name: String,
    indexed_textures: VecDeque<(String, Vec<TxdTexture>)>,
    replacement_txd_names: HashSet<String>,
    linked_definition_ids: VecDeque<String>,
    recompiled: usize,
    index_complete: bool,
}

struct EditingImgSaveOutcome {
    path: PathBuf,
    backup: PathBuf,
    rows: Vec<EditingImgRow>,
    /// Fresh TXD offsets for the rewritten IMG. IMG members can move after any
    /// entry changes size, so the previous global index is no longer valid.
    txd_index: TxdTextureIndex,
    camera: CameraState,
    selected_row: usize,
    selected_name: Option<String>,
    active_asset_name: Option<String>,
    saved_modified_entries: BTreeMap<String, Vec<u8>>,
    saved_deleted_entries: BTreeSet<String>,
    saved_added_entries: BTreeSet<String>,
}

impl Default for EditingState {
    fn default() -> Self {
        Self {
            img_paths: Vec::new(),
            selected_img_path: 0,
            archive_picker_open: false,
            img_path: None,
            rows: Vec::new(),
            selected_row: 0,
            scroll: 0.0,
            search: String::new(),
            search_cursor: 0,
            search_anchor: None,
            search_active: false,
            modified_entries: BTreeMap::new(),
            deleted_entries: BTreeSet::new(),
            added_entries: BTreeSet::new(),
            asset: None,
            camera: None,
            camera_mode: None,
            camera_focus: None,
            message: "Open an IMG archive to begin editing.".to_string(),
            save_rx: None,
            merge_rx: None,
            merge_apply_job: None,
            txd_import_rx: None,
            dff_texture_replace_rx: None,
            txd_source_picker_rx: None,
            txd_refresh_job: None,
            linked_selection_job: None,
            nested_scroll_focus: None,
            scrollbar_drag: None,
            scrollbar_drag_grab_offset_y: 0.0,
            texture_category_menu: None,
        }
    }
}

struct AssetBrowserState {
    expanded: bool,
    height: f32,
    grid_scale: f32,
    resizing: bool,
    show_sa_assets: bool,
    sort_by_zone: bool,
    hide_lods: bool,
    show_buildings: bool,
    show_objects: bool,
    show_scenery: bool,
    category_dropdown_open: bool,
    hidden_categories: HashSet<String>,
    search: String,
    cursor: usize,
    selection_anchor: Option<usize>,
    search_active: bool,
    scroll: f32,
    thumbnails: HashMap<String, Texture2D>,
    thumbnail_placeholder: Option<Texture2D>,
    thumbnail_generation_available: bool,
    entries_cache: std::sync::Arc<Vec<AssetBrowserEntry>>,
    entries_cache_fingerprint: Option<u64>,
    dff_sources: Option<HashMap<String, ImgEntry>>,
    drag: Option<AssetBrowserDrag>,
}

struct AssetBrowserDrag {
    entry_id: String,
    start_mouse: Vec2,
    position: Option<V3>,
    dragging: bool,
}

impl Default for AssetBrowserState {
    fn default() -> Self {
        Self {
            expanded: false,
            height: 244.0,
            grid_scale: 1.0,
            resizing: false,
            show_sa_assets: false,
            sort_by_zone: true,
            hide_lods: true,
            show_buildings: true,
            show_objects: true,
            show_scenery: true,
            category_dropdown_open: false,
            hidden_categories: HashSet::new(),
            search: String::new(),
            cursor: 0,
            selection_anchor: None,
            search_active: false,
            scroll: 0.0,
            thumbnails: HashMap::new(),
            thumbnail_placeholder: None,
            thumbnail_generation_available: false,
            entries_cache: std::sync::Arc::new(Vec::new()),
            entries_cache_fingerprint: None,
            dff_sources: None,
            drag: None,
        }
    }
}

#[derive(Clone)]
struct VehicleAsset {
    id: String,
    model_id: Option<u32>,
    handling_id: Option<String>,
    handling: HashMap<String, String>,
    dff: String,
    txd: String,
    col: String,
    wheel_front: Option<f32>,
    wheel_rear: Option<f32>,
    source: String,
    readonly: bool,
    loose_dff_path: Option<PathBuf>,
    loose_txd_path: Option<PathBuf>,
}

#[derive(Clone, Copy)]
struct VehicleGizmoDrag {
    component: usize,
    axis: GizmoAxis,
    start_mouse: Vec2,
    start_rotation: V3,
}

struct VehicleReloadResult {
    identity: String,
    vehicle_id: String,
    txd_name: String,
    raw: Result<RawMesh, String>,
    embedded_collision: Option<CollisionMesh>,
    txd_index: TxdTextureIndex,
    txd_error: Option<String>,
}

enum VehicleTextureExportUpdate {
    Progress { completed: usize, total: usize },
    Complete(VehicleTextureExportResult),
}

struct VehicleTextureExportResult {
    base_message: String,
    folder: PathBuf,
    exported: usize,
    generic_exported: usize,
    failed: usize,
    error: Option<String>,
}

struct VehicleTextureReplaceResult {
    vehicle_identity: String,
    vehicle_id: String,
    txd_name: String,
    texture_name: String,
    result: Result<(TxdTextureIndex, RawMesh, Option<PathBuf>), String>,
}

#[derive(Clone)]
struct VehicleCollisionCopyDialog {
    target: VehicleAsset,
    search: String,
    cursor: usize,
    selection_anchor: Option<usize>,
    selected_source: Option<usize>,
    scroll: f32,
}

struct VehicleCollisionCopyResult {
    target_identity: String,
    target_id: String,
    target_dff: String,
    source_id: String,
    result: Result<VehicleCollisionCopyOutput, String>,
}

struct VehicleCollisionCopyOutput {
    mesh: CollisionMesh,
    collision_bytes: Vec<u8>,
    dff_bytes: Vec<u8>,
    backup: Option<PathBuf>,
}

#[derive(Clone, PartialEq, Eq)]
struct VehicleTextureReplaceRequest {
    vehicle_identity: String,
    vehicle_id: String,
    dff_path: PathBuf,
    txd_path: PathBuf,
    txd_name: String,
    texture_name: String,
}

enum VehicleDictionaryScanUpdate {
    Progress { scanned: usize, total: usize },
    Complete(Vec<VehicleAsset>),
}

#[derive(Clone)]
struct VehicleBuildDialog {
    name: String,
    category: String,
    categories: Vec<String>,
    category_used: HashSet<String>,
    category_dropdown_open: bool,
    category_scroll: f32,
    category_manager_open: bool,
    category_manager_scroll: f32,
    category_new: String,
    category_new_cursor: usize,
    category_new_anchor: Option<usize>,
    base_model: String,
    base_search: String,
    base_dropdown_open: bool,
    base_scroll: f32,
    wheel_front: String,
    wheel_rear: String,
    wheel_width: String,
    new_vehicle: bool,
    active_field: Option<usize>,
    cursor: usize,
    selection_anchor: Option<usize>,
    handling: Vec<(&'static str, String)>,
    handling_dropdown: Option<usize>,
}

struct VehicleBuildResult {
    message: String,
    loader: VehicleLoaderScanResult,
}

struct VehicleLoaderScanResult {
    membership: HashSet<String>,
    categories: Vec<String>,
    used_categories: HashSet<String>,
}

struct VehicleBrowserState {
    search: String,
    cursor: usize,
    selection_anchor: Option<usize>,
    search_active: bool,
    selected: usize,
    scroll: f32,
    manage_dictionaries: bool,
    dictionary_scroll: f32,
    dictionary_scroll_drag: bool,
    list_scroll_drag: bool,
    list_scroll_grab_offset_y: f32,
    build_dialog: Option<VehicleBuildDialog>,
    preview_mesh: Option<RenderMesh>,
    embedded_collision: Option<CollisionMesh>,
    preview_key: String,
    show_default: bool,
    show_custom: bool,
    show_body: bool,
    show_collision_mesh: bool,
    show_collision_volumes: bool,
    show_textures: bool,
    lights_on: bool,
    hide_damaged: bool,
    hide_vlo: bool,
    body_color_a: V3,
    body_color_b: V3,
    body_color_picker: Option<usize>,
    /// Full-width presentation view for screenshots of the selected vehicle.
    photo_mode: bool,
    /// Regular vehicle-preview camera restored when photo mode closes.
    photo_mode_camera: Option<CameraState>,
    photo_mode_camera_mode: Option<CameraMode>,
    photo_mode_focus: Option<Option<Vec3>>,
    /// Lazily compiled clear-coat/environment pass used only by photo mode.
    photo_reflection_program: u32,
    photo_reflection_failed: bool,
    hidden_parts: HashSet<usize>,
    hidden_components: HashSet<usize>,
    /// Components explicitly shown from the list despite an active name-based
    /// visibility filter, such as "Hide Damaged".
    forced_visible_components: HashSet<usize>,
    collapsed_components: HashSet<usize>,
    selected_component: Option<usize>,
    component_rotations: HashMap<usize, V3>,
    hovered_gizmo: Option<GizmoAxis>,
    gizmo_drag: Option<VehicleGizmoDrag>,
    selected_part: Option<usize>,
    texture_previews: HashMap<String, Option<(Texture2D, u32, u32)>>,
    component_scroll: f32,
    component_scroll_drag: bool,
    component_scroll_grab_offset_y: f32,
    /// Camera used for the vehicle preview viewport. Kept separate from the main
    /// `AppState.camera` so orbiting the preview doesn't move the world camera.
    camera: Option<CameraState>,
    camera_mode: Option<CameraMode>,
    camera_focus: Option<Option<Vec3>>,
    /// DFF parsing and TXD indexing happen off the render thread. The completed
    /// CPU-side assets are installed (and uploaded to GL) from the main loop.
    reload_rx: Option<mpsc::Receiver<VehicleReloadResult>>,
    /// PNG encoding and filesystem writes for DFF companion textures run away
    /// from the render thread and report progress back through this channel.
    texture_export_rx: Option<mpsc::Receiver<VehicleTextureExportUpdate>>,
    /// Image conversion and custom vehicle TXD replacement run off the render
    /// thread. The completed texture index and preview mesh are installed from
    /// the main loop.
    texture_replace_rx: Option<mpsc::Receiver<VehicleTextureReplaceResult>>,
    /// Reading, validating, and atomically replacing an embedded custom-vehicle
    /// collision runs off the render thread.
    collision_copy_rx: Option<mpsc::Receiver<VehicleCollisionCopyResult>>,
    collision_copy_dialog: Option<VehicleCollisionCopyDialog>,
}

impl Default for VehicleBrowserState {
    fn default() -> Self {
        Self {
            search: String::new(),
            cursor: 0,
            selection_anchor: None,
            search_active: false,
            selected: 0,
            scroll: 0.0,
            manage_dictionaries: false,
            dictionary_scroll: 0.0,
            dictionary_scroll_drag: false,
            list_scroll_drag: false,
            list_scroll_grab_offset_y: 0.0,
            build_dialog: None,
            preview_mesh: None,
            embedded_collision: None,
            preview_key: String::new(),
            show_default: true,
            show_custom: true,
            show_body: true,
            show_collision_mesh: false,
            show_collision_volumes: false,
            show_textures: true,
            lights_on: false,
            hide_damaged: true,
            hide_vlo: true,
            body_color_a: V3 {
                x: 0.075,
                y: 0.255,
                z: 0.640,
            },
            body_color_b: V3 {
                x: 0.035,
                y: 0.135,
                z: 0.360,
            },
            body_color_picker: None,
            photo_mode: false,
            photo_mode_camera: None,
            photo_mode_camera_mode: None,
            photo_mode_focus: None,
            photo_reflection_program: 0,
            photo_reflection_failed: false,
            hidden_parts: HashSet::new(),
            hidden_components: HashSet::new(),
            forced_visible_components: HashSet::new(),
            collapsed_components: HashSet::new(),
            selected_component: None,
            component_rotations: HashMap::new(),
            hovered_gizmo: None,
            gizmo_drag: None,
            selected_part: None,
            texture_previews: HashMap::new(),
            component_scroll: 0.0,
            component_scroll_drag: false,
            component_scroll_grab_offset_y: 0.0,
            camera: None,
            camera_mode: None,
            camera_focus: None,
            reload_rx: None,
            texture_export_rx: None,
            texture_replace_rx: None,
            collision_copy_rx: None,
            collision_copy_dialog: None,
        }
    }
}

struct ProjectPicker {
    options: Options,
    ui_font: Font,
    icons: IconSet,
    projects: Vec<PathBuf>,
    project_roots: Vec<PathBuf>,
    thumbnails: HashMap<PathBuf, Option<Texture2D>>,
    project_discovery_rx: Option<mpsc::Receiver<ProjectDiscoveryUpdate>>,
    picker_rx: Option<mpsc::Receiver<Result<Option<PathBuf>, String>>>,
    project_root_picker_rx: Option<mpsc::Receiver<Result<Option<PathBuf>, String>>>,
    new_root_picker_rx: Option<mpsc::Receiver<Result<Option<PathBuf>, String>>>,
    new_project_dialog: Option<NewProjectDialog>,
    project_roots_dialog: bool,
    project_scroll_row: usize,
    project_scroll_drag: bool,
    status: String,
}

enum ProjectDiscoveryUpdate {
    Scanning {
        index: usize,
        total: usize,
        root: PathBuf,
    },
    Finished(Vec<PathBuf>),
}

struct NewProjectDialog {
    root: String,
    name: String,
    cursor: usize,
    selection_anchor: Option<usize>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SimPlaceTool {
    Player,
    Vehicle,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SimObjectKind {
    Player,
    Vehicle,
}

struct SimObject {
    kind: SimObjectKind,
    pos: Vec3,
    rot_z: f32,
}

struct SimState {
    place_tool: SimPlaceTool,
    objects: Vec<SimObject>,
    playing: bool,
    player_index: Option<usize>,
    vehicle_index: Option<usize>,
    controlling_vehicle: Option<usize>,
    velocity: Vec3,
    vehicle_speed: f32,
    on_ground: bool,
}

impl Default for SimState {
    fn default() -> Self {
        Self {
            place_tool: SimPlaceTool::Player,
            objects: Vec::new(),
            playing: false,
            player_index: None,
            vehicle_index: None,
            controlling_vehicle: None,
            velocity: Vec3::ZERO,
            vehicle_speed: 0.0,
            on_ground: true,
        }
    }
}

enum RuntimeState {
    ProjectPicker(ProjectPicker),
    Loading(Option<LoadJob>),
    Editor(AppState),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LoadSceneSource {
    Saved,
    Autosave,
}

struct MissingColDialog {
    target: PathBuf,
    missing: usize,
}

#[derive(Clone)]
struct LodBatchCandidate {
    placement_index: usize,
    id: String,
    dff: String,
    size: f32,
    existing_lod: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LodBatchMode {
    GenerateSelection,
    GenerateSceneMissing,
    RegenerateScene,
}

struct LodBatchDialog {
    mode: LodBatchMode,
    candidates: Vec<LodBatchCandidate>,
    minimum_size: String,
    cursor: usize,
    selection_anchor: Option<usize>,
    scroll: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AssetOptimizationScope {
    textures: bool,
    dffs: bool,
    cols: bool,
}

impl Default for AssetOptimizationScope {
    fn default() -> Self {
        Self {
            textures: true,
            dffs: true,
            cols: true,
        }
    }
}

impl AssetOptimizationScope {
    fn any(self) -> bool {
        self.textures || self.dffs || self.cols
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DffRepairScope {
    dff_issues: bool,
    prelighting: bool,
    texture_names: bool,
}

impl Default for DffRepairScope {
    fn default() -> Self {
        Self {
            dff_issues: true,
            prelighting: true,
            texture_names: true,
        }
    }
}

impl DffRepairScope {
    fn any(self) -> bool {
        self.dff_issues || self.prelighting || self.texture_names
    }
}

struct TxdCleanupTarget {
    txd_name: String,
    source_fingerprint: [u64; 2],
    prepared_bytes: Vec<u8>,
    dff_names: Vec<String>,
    purge_count: usize,
    purge_bytes: usize,
    format_fix_count: usize,
    format_fixes: Vec<String>,
    profile_reports: Vec<TxdTextureOptimizationReport>,
    profile_bytes_saved: i64,
    texture_renames: HashMap<String, String>,
    warnings: Vec<String>,
}

struct TxdCleanupPlan {
    targets: Vec<TxdCleanupTarget>,
    consolidations: Vec<TxdConsolidation>,
    deep_optimize: bool,
    scope: AssetOptimizationScope,
    profile: TxdOptimizationProfile,
    profile_bytes_saved: i64,
    purge_count: usize,
    purge_bytes: usize,
    format_fix_count: usize,
    format_fixes: Vec<String>,
    texture_rename_count: usize,
    skipped: usize,
    errors: Vec<String>,
    warnings: Vec<String>,
}

#[derive(Clone)]
struct TxdConsolidation {
    retained: String,
    donors: Vec<String>,
    exact: bool,
    source_fingerprints: BTreeMap<String, [u64; 2]>,
}

enum TxdCleanupPhase {
    DiscoverArchives,
    IndexArchives,
    ScanDffs,
    ScanTxds,
}

struct TxdCleanupDefinition {
    dff_name: String,
    txd_name: String,
    /// The live in-memory DFF, when it has not been promoted to the project
    /// archive yet. Generated LODs land here before Save, so cleanup must scan
    /// these bytes instead of an older (or not-yet-existing) archive entry.
    dff_bytes: Option<Arc<Vec<u8>>>,
}

struct TxdCleanupJob {
    root: PathBuf,
    wip_root: PathBuf,
    gta_sa_dir: PathBuf,
    img_files: Vec<PathBuf>,
    img_index: usize,
    dff_map: HashMap<String, ImgEntry>,
    txd_map: HashMap<String, ImgEntry>,
    definitions: Vec<TxdCleanupDefinition>,
    definition_index: usize,
    used_by_txd: HashMap<String, HashSet<String>>,
    dffs_by_txd: HashMap<String, HashSet<String>>,
    txd_names: Vec<String>,
    txd_index: usize,
    targets: Vec<TxdCleanupTarget>,
    prepared_txds: BTreeMap<String, Vec<TxdTextureContent>>,
    source_fingerprints: BTreeMap<String, [u64; 2]>,
    excluded_assets: HashSet<String>,
    deep_optimize: bool,
    scope: AssetOptimizationScope,
    profile: TxdOptimizationProfile,
    skipped: usize,
    errors: Vec<String>,
    phase: TxdCleanupPhase,
    status: String,
    started_at: Instant,
}

struct DffRepairResult {
    scope: DffRepairScope,
    scanned: usize,
    repaired: usize,
    prelighting_fixed: usize,
    sanitized_texture_names: usize,
    sanitized_texture_references: usize,
    normalized: usize,
    bin_mesh_batches_removed: usize,
    lighting_flags_fixed: usize,
    bounds_fixed: usize,
    uv_anim_reordered: usize,
    uv_anim_pipeline_fixed: usize,
    uv_anim_legacy_slots_fixed: usize,
    skipped: usize,
    repaired_dff_names: Vec<String>,
    repaired_entries: Vec<String>,
    errors: Vec<String>,
    elapsed: f32,
}

struct DffRepairRefresh {
    result: DffRepairResult,
    definition_ids: Vec<String>,
    next_definition: usize,
    refreshed: usize,
}

enum ConfirmAction {
    Load(PathBuf),
    OpenEditingAsset(EditingImgRow),
    OpenEditingFile(PathBuf),
    OpenEditingImgEntry(ImgEntry),
    OpenSceneDffsInEditing {
        models: Vec<(usize, Placement, ImgEntry)>,
        camera: Option<CameraState>,
    },
    OpenEditingStagedAsset(String),
    ApplyEditingImgMerge {
        plan: Arc<EditingImgMergePlan>,
        overwrite_matching: bool,
    },
    SaveEditingImgWithDuplicateCleanup,
    OpenSelectedVehicleCollision,
    RestoreAutosave(PathBuf),
    DismissAutosave,
    CleanupAutosaves,
    DismissWarning,
    TeleportCamera(Vec3),
    Quit,
    DeleteElements {
        indices: Vec<usize>,
        lod_indices: Vec<usize>,
        delete_lods: bool,
    },
    StartInstanceLodRemoval(String),
    ReviewPurgeUnused,
    PurgeUnused,
    RebalanceImgArchives,
    FixLods,
    RegenerateLods(Vec<usize>),
    ClearAllLods,
    TxdCleanup(TxdCleanupPlan),
    RequestTxdCleanup,
    ChooseAssetOptimizationProfile,
    StartAssetOptimization(TxdOptimizationProfile),
    StartBake(BakeScope),
    ClearBake,
    GenerateTxd {
        source_dir: PathBuf,
        destination: TxdGenerationDestination,
    },
    DefinitionOverrideEdit {
        ids: Vec<String>,
        field: InspectorField,
        value: String,
        before: ScopedHistorySnapshot,
    },
    DefinitionOverrideFlag {
        ids: Vec<String>,
        flag: &'static str,
        target_enabled: bool,
        before: ScopedHistorySnapshot,
    },
    MarkTextureElementsDoubleSided {
        indices: Vec<usize>,
        texture_name: String,
    },
    ApplyPhysicsEdit {
        edit: InspectorEdit,
        conversion_indices: Vec<usize>,
        writable_definition_ids: Vec<String>,
    },
    SetPhysicsRoot {
        model_id: u16,
        conversion_indices: Vec<usize>,
        before: WorldHistorySnapshot,
    },
}

struct ConfirmDialog {
    action: ConfirmAction,
    title: String,
    body: String,
    detail: String,
    primary_label: String,
    secondary_label: Option<String>,
    secondary_action: Option<ConfirmAction>,
}

#[derive(Clone)]
struct IconSet {
    select: Texture2D,
    move_tool: Texture2D,
    rotate: Texture2D,
    scale: Texture2D,
    duplicate: Texture2D,
    delete: Texture2D,
    face: Texture2D,
    edge: Texture2D,
    vertex: Texture2D,
    sphere: Texture2D,
    cube: Texture2D,
    save: Texture2D,
    undo: Texture2D,
    redo: Texture2D,
    texture: Texture2D,
    tool: Texture2D,
}

#[derive(Clone, Debug, Default)]
struct ParticleSizeCurve {
    size_x: Vec<(f32, f32)>,
    size_y: Vec<(f32, f32)>,
}

#[derive(Clone, Debug, Default)]
struct ParticleEffectDef {
    name: String,
    textures: Vec<String>,
    length: f32,
    play_mode: i32,
    cull_distance: f32,
    primitive_count: usize,
    size_curves: Vec<ParticleSizeCurve>,
}

struct AppState {
    options: Options,
    root: PathBuf,
    placements: Vec<Placement>,
    definitions: HashMap<String, Definition>,
    readonly_definition_ids: HashSet<String>,
    zones: Vec<String>,
    eagle_zone_offsets: EagleZoneOffsets,
    meshes: HashMap<String, RenderMesh>,
    collisions: HashMap<String, CollisionMesh>,
    collision_render_cache: HashMap<String, CollisionRenderCache>,
    lod_ids: HashSet<String>,
    scene_cells: Vec<SceneCell>,
    world_cells: Vec<WorldCell>,
    lod_scene_cells: Vec<SceneCell>,
    lod_world_cells: Vec<WorldCell>,
    /// Placements whose currently resolved mesh contains 2DFX data.
    placement_2dfx_indices: Vec<usize>,
    placement_2dfx_index_placement_count: usize,
    textures: HashMap<String, u32>,
    // Retained post-load so textures can be re-decoded at runtime (e.g. toggling
    // the texture option) without a full reload. Not read on the current paths.
    #[allow(dead_code)]
    txd_textures: TxdTextureIndex,
    texture_files: HashMap<String, PathBuf>,
    texture_alias_count: usize,
    texture_overrides: HashMap<(String, String), String>,
    pending_replacement_assets: BTreeMap<String, (String, Vec<u8>)>,
    pending_txd_writes: HashSet<String>,
    pending_asset_deletes: HashSet<String>,
    pending_vertex_light_meshes: HashSet<String>,
    material_emitters: HashMap<String, MaterialEmitter>,
    material_emitters_dirty: bool,
    material_classes: TextureMaterialClasses,
    material_classes_dirty: bool,
    safe_collisions: SafeCollisions,
    safe_collisions_dirty: bool,
    shadow_casting: HashMap<String, bool>,
    selected: usize,
    selected_elements: BTreeSet<usize>,
    selected_element_order: Vec<usize>,
    hovered: Option<usize>,
    outliner_labels: Vec<Option<(String, String)>>,
    outliner_filter: Vec<OutlinerEntry>,
    outliner_search: String,
    outliner_search_cursor: usize,
    outliner_search_anchor: Option<usize>,
    outliner_search_active: bool,
    outliner_show_objects: bool,
    outliner_show_buildings: bool,
    outliner_show_lods: bool,
    asset_browser: AssetBrowserState,
    vehicles: Vec<VehicleAsset>,
    vehicle_browser: VehicleBrowserState,
    custom_vehicle_dictionaries: Vec<PathBuf>,
    vehicle_folder_picker_rx: Option<mpsc::Receiver<Result<Option<PathBuf>, String>>>,
    vehicle_dictionary_scan_rx: Option<mpsc::Receiver<VehicleDictionaryScanUpdate>>,
    vehicle_loader_resource: Option<PathBuf>,
    vehicle_loader_membership: HashSet<String>,
    vehicle_loader_categories: Vec<String>,
    vehicle_loader_used_categories: HashSet<String>,
    vehicle_loader_picker_rx: Option<mpsc::Receiver<Result<Option<PathBuf>, String>>>,
    vehicle_loader_scan_rx: Option<mpsc::Receiver<Result<VehicleLoaderScanResult, String>>>,
    vehicle_build_rx: Option<mpsc::Receiver<Result<VehicleBuildResult, String>>>,
    validation_cache: Option<ValidationSummary>,
    validation_action_category: ValidationActionCategory,
    duplicate_placement_scan_rx: Option<mpsc::Receiver<DuplicatePlacementScanUpdate>>,
    lod_audit: LodAuditState,
    missing_texture_review: MissingTextureReviewState,
    element_states: Vec<ElementState>,
    transform_mode: TransformMode,
    transform_space: TransformSpace,
    snap_enabled: bool,
    show_selected_lod_local: bool,
    lod_selectable: bool,
    snap_move: f32,
    snap_rotate: f32,
    active_tab: AppTab,
    viewport_render_mode: ViewportRenderMode,
    editing: EditingState,
    properties_tab: PropertiesTab,
    properties_scroll: f32,
    validation_list_scroll: [f32; 3],
    validation_list_scroll_drag: Option<usize>,
    validation_list_scroll_grab_offset_y: f32,
    validation_collision_material_dropdown_open: bool,
    validation_collision_material_search: String,
    validation_collision_material_scroll: f32,
    element_panel_collapsed: [bool; ELEM_SECTION_COUNT],
    preview_selected_material: Option<(usize, usize)>,
    preview_texture_advanced: bool,
    preview_world_uv_all_dffs: bool,
    preview_world_uv_scale: f32,
    preview_world_uv_variation: bool,
    preview_world_uv_job: Option<PreviewWorldUvJob>,
    preview_world_uv_visual: Option<PreviewWorldUvVisual>,
    texture_match_selection_job: Option<TextureMatchSelectionJob>,
    water_texture_conversion_job: Option<WaterTextureConversionJob>,
    physics_scope: PhysicsScope,
    physics_root_properties: HashMap<u16, PhysicsRootProperties>,
    physics_root_dropdown_open: bool,
    native_model_dropdown_open: bool,
    native_model_dropdown_search: String,
    native_model_dropdown_scroll: f32,
    txd_dropdown_open: bool,
    txd_dropdown_search: String,
    txd_dropdown_scroll: f32,
    txd_dropdown_create_mode: bool,
    settings_panel_collapsed: [bool; SETTINGS_SECTION_COUNT],
    global_transform: GlobalTransformState,
    collision_edit_mode: bool,
    selected_col_face: Option<SelectedCollisionFace>,
    selected_col_vertex: usize,
    hovered_col_face: Option<SelectedCollisionFace>,
    hovered_col_vertex: Option<usize>,
    col_material_dropdown_open: bool,
    col_material_dropdown_scroll: f32,
    pending_col_writes: HashMap<(PathBuf, u64), u8>,
    hovered_gizmo: Option<GizmoAxis>,
    hovered_gizmo_plane: Option<GizmoPlane>,
    gizmo_drag: Option<GizmoDrag>,
    dff_scale_input: Option<DffScaleInput>,
    cull_face_drag: Option<CullFaceDrag>,
    cull_hovered_face: Option<(usize, bool)>,
    col_box_face_drag: Option<ColBoxFaceDrag>,
    col_box_hovered_face: Option<(CollisionPrimitiveSelection, usize, bool)>,
    inspector_edit: Option<InspectorEdit>,
    group_rename: Option<GroupRenameEdit>,
    race_name_edit: Option<RaceNameEdit>,
    undo_stack: Vec<UndoEntry>,
    redo_stack: Vec<UndoEntry>,
    scroll: f32,
    scroll_interaction_until: f64,
    outliner_scroll_drag: Option<OutlinerScrollDrag>,
    scrollbar_pointer_captured: bool,
    inspector_scroll_drag: bool,
    water_list_scroll_drag: bool,
    light_list_scroll_drag: bool,
    box_select_drag: Option<BoxSelectDrag>,
    box_select_distance: f32,
    box_select_mode: BoxSelectMode,
    selected_group: Option<String>,
    expanded_groups: BTreeSet<String>,
    context_menu: Option<ContextMenu>,
    load_dialog: Option<LoadDialog>,
    import_asset_dialog: Option<ImportAssetDialog>,
    preferences_dialog: Option<PreferencesDialog>,
    load_picker_rx: Option<mpsc::Receiver<Result<Option<PathBuf>, String>>>,
    dff_picker_rx: Option<mpsc::Receiver<(DffPickerKind, Result<Option<PathBuf>, String>)>>,
    blender_import_setup: Option<BlenderImportSetup>,
    oversized_chunk_dialog: Option<OversizedChunkDialog>,
    oversized_chunk_job: Option<OversizedChunkJob>,
    classify_dialog: Option<ClassifyElementsDialog>,
    /// Largest world-axis extent still classified as an `object` element.
    classify_object_max_size: f32,
    blender_import_rx: Option<mpsc::Receiver<BlenderImportUpdate>>,
    blender_import_dialog_open: bool,
    blender_import_progress: f32,
    blender_import_phase: String,
    blender_import_log: Vec<String>,
    blender_import_log_scroll: f32,
    blender_import_log_follow_tail: bool,
    blender_import_finished: bool,
    dff_repair_rx: Option<mpsc::Receiver<DffRepairResult>>,
    dff_repair_refresh: Option<DffRepairRefresh>,
    dff_repair_scope: DffRepairScope,
    dff_repair_menu_open: bool,
    pending_load_root: Option<PathBuf>,
    pending_load_source: LoadSceneSource,
    load_job: Option<LoadJob>,
    save_as_dialog: Option<SaveAsDialog>,
    dff_replace_choice_dialog: Option<DffReplaceChoiceDialog>,
    dff_prelight_import_dialog: Option<DffPrelightImportDialog>,
    dff_merge_choice_dialog: Option<DffMergeChoiceDialog>,
    dff_optimize_dialog: Option<DffOptimizeDialog>,
    dff_txd_pair_dialog: Option<DffTxdPairDialog>,
    /// Options carried between openings of the optimize dialog.
    dff_optimize_options: DffOptimizeOptions,
    dff_texture_duplicate_dialog: Option<DffTextureDuplicateDialog>,
    dff_texture_view_dialog: Option<DffTextureViewDialog>,
    element_id_rename_dialog: Option<ElementIdRenameDialog>,
    element_replace_with_dialog: Option<ElementReplaceWithDialog>,
    missing_texture_dialog: Option<MissingTextureDialog>,
    texture_archive_dialog: Option<TextureArchiveDialog>,
    missing_col_dialog: Option<MissingColDialog>,
    lod_batch_dialog: Option<LodBatchDialog>,
    confirm_dialog: Option<ConfirmDialog>,
    txd_cleanup_job: Option<TxdCleanupJob>,
    asset_optimization_scan_rx: Option<mpsc::Receiver<Result<TxdCleanupPlan, String>>>,
    asset_optimization_job: Option<AssetOptimizationJob>,
    asset_optimization_scope: AssetOptimizationScope,
    asset_optimization_menu_open: bool,
    navigation_menu_open: bool,
    purge_unused_job: Option<PurgeUnusedJob>,
    img_archive_rebalance_job: Option<ImgArchiveRebalanceJob>,
    object_bounds_fix_job: Option<ObjectBoundsFixJob>,
    corona_generation_job: Option<CoronaGenerationJob>,
    day_night_merge_job: Option<DayNightMergeJob>,
    day_night_merge_override: Option<DayNightMergeOverrideKey>,
    light_lod_job: Option<LightLodJob>,
    fracture_generation_job: Option<FractureGenerationJob>,
    dff_geometry_job: Option<DffGeometryJob>,
    dff_material_limit_repair_job: Option<DffMaterialLimitRepairJob>,
    collision_generation_job: Option<CollisionGenerationJob>,
    shadow_mesh_generation_job: Option<ShadowMeshGenerationJob>,
    collision_cuboid_audit_job: Option<CollisionCuboidAuditJob>,
    lod_generation_job: Option<LodGenerationJob>,
    instance_lod_removal_job: Option<InstanceLodRemovalJob>,
    collision_generation_preset: CollisionGenerationPreset,
    collision_generation_fallback_material: u8,
    camera: CameraState,
    /// Camera owned by the gameworld tabs. Editing and Vehicles keep their
    /// cameras in their respective tab state instead.
    gameworld_camera: Option<CameraState>,
    gameworld_camera_mode: Option<CameraMode>,
    gameworld_camera_focus: Option<Option<Vec3>>,
    camera_mode: CameraMode,
    camera_focus: Option<Vec3>,
    loaded_message: String,
    load_seconds: f32,
    textured_parts: usize,
    last_drawn_placements: usize,
    last_drawn_parts: usize,
    last_drawn_vertices: usize,
    last_log: Instant,
    render_settle_until: f64,
    fps_last_frame: Instant,
    fps_display: i32,
    ui_font: Font,
    icons: IconSet,
    status_message: String,
    activity_log: Vec<String>,
    activity_last_status: String,
    activity_started_at: Instant,
    save_log: Vec<String>,
    save_log_open: bool,
    save_log_scroll: f32,
    save_log_follow_tail: bool,
    camera_speed: f32,
    vehicle_camera_speed: f32,
    editing_camera_speed: f32,
    camera_rotation_speed: f32,
    gizmo_scale: f32,
    gta_sa_dir: PathBuf,
    particle_effects: Vec<ParticleEffectDef>,
    bake_settings: BakeSettings,
    vertex_paint: VertexPaintSettings,
    prelight_clipboard: Option<PrelightClipboard>,
    vertex_paint_dirty_meshes: HashSet<String>,
    vertex_paint_next_rebuild_at: f64,
    bake_job: Option<BakeJob>,
    gpu_lightmap: GpuLightmapPipeline,
    postfx_preview_texture: u32,
    postfx_preview_size: (i32, i32),
    dff_pointlight_preview_program: u32,
    dff_pointlight_preview_failed: bool,
    lights: Vec<EditorLight>,
    selected_light: usize,
    light_list_scroll: f32,
    light_kind_dropdown_open: bool,
    light_profile_dropdown_open: bool,
    light_color_drag_before: Option<LightHistorySnapshot>,
    light_temperature_drag_before: Option<LightHistorySnapshot>,
    water_planes: Vec<WaterPlane>,
    selected_water: usize,
    selected_water_planes: BTreeSet<usize>,
    hovered_water: Option<usize>,
    hovered_water_edge: Option<(usize, WaterEdge)>,
    water_edge_drag: Option<WaterEdgeDrag>,
    water_edge_snap_enabled: bool,
    water_scroll: f32,
    cull_zones: Vec<CullZone>,
    selected_cull: usize,
    cull_scroll: f32,
    race: RaceEditorState,
    timecyc: TimecycState,
    fog_strength: f32,
    sim: SimState,
    saved_snapshot: Option<SavedContentSnapshot>,
    manual_save_job: Option<ManualSaveJob>,
    pending_after_manual_save: Option<ConfirmAction>,
    autosave_next_at: f64,
    autosave_rx: Option<mpsc::Receiver<AutosaveResult>>,
    autosave_dirty_snapshot: Option<AutosaveDirtySnapshot>,
    autosave_cleanup_rx: Option<mpsc::Receiver<Result<bool, String>>>,
    autosave_restore_prompted: bool,
    loaded_autosave: bool,
    loaded_wip: bool,
    quit_after_persist: bool,
    last_camera_persist_at: f64,
    sim_editor_camera: Option<CameraState>,
    pending_camera_restore: Option<(CameraState, f64)>,
}

// ===================== Race Editor data model =====================
//
// A map may define one or more race tracks. Each track is stored in the map's
// own `tracks/tracks.xml` (see resource/race.rs) so the race system can locate
// per-map data. Gameplay checkpoints and the dense preview/radar overlay path
// are stored SEPARATELY: checkpoints drive the actual race logic, while the
// overlay/path point lists are only used for drawing the cropped preview image
// and the live radar/progress display, and can be much denser.

#[derive(Clone, Copy, PartialEq)]
struct RaceCheckpoint {
    pos: V3,
    r: f32,
}

// A single named variant ("subtrack") of a race track. Each subtrack has its
// own route (checkpoints), dense preview overlay, driving path, and generated
// preview images. The M_Race loader shows these as selectable variants.
#[derive(Clone, PartialEq)]
struct RaceSubtrack {
    id: String,
    name: String,
    /// Relative (to the map resource root) path of the generated preview PNG.
    image: String,
    /// Relative path of the generated transparent overlay PNG.
    overlay_image: String,
    /// Gameplay checkpoints — control the race route and checkpoint logic.
    checkpoints: Vec<RaceCheckpoint>,
    /// Dense outline points — used only for preview + live radar drawing.
    overlay: Vec<V3>,
    /// Full track path across the map (driving line reference).
    path: Vec<V3>,
}

impl RaceSubtrack {
    fn new(index: usize) -> Self {
        RaceSubtrack {
            id: format!("sub{}", index + 1),
            name: format!("Sub {}", index + 1),
            image: String::new(),
            overlay_image: String::new(),
            checkpoints: Vec::new(),
            overlay: Vec::new(),
            path: Vec::new(),
        }
    }
}

// A race track. Track-level fields (id/name/laps/radius/start/find) are shared
// by all its subtracks. The per-route fields below (checkpoints/overlay/path/
// image/subtrack_name) are a live "scratch" copy of the *currently selected*
// subtrack, so all the existing point-editing code keeps operating on them
// unchanged. `subtracks` stores every variant; `active_subtrack` is the index
// the scratch fields mirror. See commit_active_subtrack / load_subtrack.
#[derive(Clone, PartialEq)]
struct RaceTrack {
    id: String,
    name: String,
    laps: i32,
    radius: f32,
    start: V3,
    find: V3,
    /// Live name of the active subtrack (scratch; mirrors subtracks[active].name).
    subtrack_name: String,
    /// Relative (to the map resource root) path of the generated preview PNG.
    image: String,
    /// Relative path of the generated transparent overlay PNG.
    overlay_image: String,
    /// Gameplay checkpoints — control the race route and checkpoint logic.
    checkpoints: Vec<RaceCheckpoint>,
    /// Dense outline points — used only for preview + live radar drawing.
    overlay: Vec<V3>,
    /// Full track path across the map (driving line reference).
    path: Vec<V3>,
    /// All subtrack variants of this track.
    subtracks: Vec<RaceSubtrack>,
    /// Which subtrack the live scratch fields currently mirror.
    active_subtrack: usize,
}

impl RaceTrack {
    fn new(index: usize) -> Self {
        let id = format!("track{}", index + 1);
        let sub = RaceSubtrack::new(0);
        RaceTrack {
            name: format!("Track {}", index + 1),
            subtrack_name: sub.name.clone(),
            image: sub.image.clone(),
            overlay_image: sub.overlay_image.clone(),
            checkpoints: sub.checkpoints.clone(),
            overlay: sub.overlay.clone(),
            path: sub.path.clone(),
            subtracks: vec![sub],
            active_subtrack: 0,
            id,
            laps: 2,
            radius: 40.0,
            start: V3::default(),
            find: V3::default(),
        }
    }

    /// Copies the live scratch fields into the active subtrack slot. Call before
    /// switching subtracks so in-progress edits aren't lost.
    fn commit_active_subtrack(&mut self) {
        if self.subtracks.is_empty() {
            self.subtracks.push(RaceSubtrack::new(0));
            self.active_subtrack = 0;
        }
        let idx = self.active_subtrack.min(self.subtracks.len() - 1);
        self.active_subtrack = idx;
        let sub = &mut self.subtracks[idx];
        sub.name = self.subtrack_name.clone();
        sub.image = self.image.clone();
        sub.overlay_image = self.overlay_image.clone();
        sub.checkpoints = self.checkpoints.clone();
        sub.overlay = self.overlay.clone();
        sub.path = self.path.clone();
    }

    /// Loads the given subtrack slot into the live scratch fields.
    fn load_subtrack(&mut self, idx: usize) {
        let Some(sub) = self.subtracks.get(idx).cloned() else {
            return;
        };
        self.active_subtrack = idx;
        self.subtrack_name = sub.name;
        self.image = sub.image;
        self.overlay_image = sub.overlay_image;
        self.checkpoints = sub.checkpoints;
        self.overlay = sub.overlay;
        self.path = sub.path;
    }

    /// The subtrack list with the active slot refreshed from the live scratch
    /// fields — always authoritative for serialization/preview, so callers don't
    /// have to remember to commit first.
    fn effective_subtracks(&self) -> Vec<RaceSubtrack> {
        let mut subs = self.subtracks.clone();
        if subs.is_empty() {
            subs.push(RaceSubtrack {
                id: "sub1".to_string(),
                name: if self.subtrack_name.is_empty() {
                    self.name.clone()
                } else {
                    self.subtrack_name.clone()
                },
                image: self.image.clone(),
                overlay_image: self.overlay_image.clone(),
                checkpoints: self.checkpoints.clone(),
                overlay: self.overlay.clone(),
                path: self.path.clone(),
            });
        } else {
            let idx = self.active_subtrack.min(subs.len() - 1);
            let s = &mut subs[idx];
            s.name = self.subtrack_name.clone();
            s.image = self.image.clone();
            s.overlay_image = self.overlay_image.clone();
            s.checkpoints = self.checkpoints.clone();
            s.overlay = self.overlay.clone();
            s.path = self.path.clone();
        }
        subs
    }
}

/// Which kind of point a viewport click currently places / edits.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RacePlaceMode {
    None,
    Start,
    Checkpoint,
    Overlay,
    Path,
}

/// The compact control group currently shown in the Race tab's Track panel.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RacePanelSection {
    Track,
    Points,
    Output,
}

struct RaceEditorState {
    tracks: Vec<RaceTrack>,
    selected_track: usize,
    place_mode: RacePlaceMode,
    panel_section: RacePanelSection,
    /// Selected point index within the list matching `place_mode`.
    selected_point: Option<usize>,
    /// Z height used for the ground plane when placing points by clicking.
    place_z: f32,
    default_radius: f32,
    /// Backdrop radar image used to generate cropped previews (absolute or
    /// relative to the map root). Empty means "auto-detect".
    radar_path: String,
    /// World units spanned by the radar backdrop (square, centered on center).
    world_size: f32,
    world_center_x: f32,
    world_center_y: f32,
    list_scroll: f32,
    /// Scroll offset for the left-side track ("Races") list.
    track_scroll: f32,
    /// True once we've attempted to load existing per-map tracks for this map.
    loaded: bool,
    /// When true, the Preview tab shows a radar minimap overlay with a marker at
    /// the current camera position.
    preview_overlay: bool,
    /// Time and screen position of the last click inside the Preview minimap,
    /// used to recognize an intentional double-click without affecting scene
    /// selection behind the overlay.
    preview_overlay_last_click_at: f64,
    preview_overlay_last_click_pos: Vec2,
    /// Lazily-loaded radar texture for the minimap overlay, plus the path key it
    /// was built from (so we reload if the configured radar changes).
    radar_tex: Option<Texture2D>,
    radar_tex_key: String,
    /// When true, the Race tab's central viewport becomes a 2D top-down radar
    /// editor where points are placed/moved directly on the radar image.
    radar_2d: bool,
    /// True while dragging a point in the 2D radar editor.
    radar_2d_dragging: bool,
    /// Point position captured when a 2D radar point drag begins.
    radar_2d_drag_before: Option<Race2dPointDrag>,
    /// Zoom factor for the 2D radar editor (1.0 = whole radar visible).
    radar_2d_zoom: f32,
    /// Top-left UV coordinate of the visible window in the 2D radar editor.
    radar_2d_pan_u: f32,
    radar_2d_pan_v: f32,
    /// True while panning the 2D radar editor with the middle mouse button.
    radar_2d_panning: bool,
    /// Last cursor position captured while middle-mouse panning the 2D editor.
    radar_2d_pan_last: Vec2,
}

impl Default for RaceEditorState {
    fn default() -> Self {
        RaceEditorState {
            tracks: Vec::new(),
            selected_track: NO_SELECTION,
            place_mode: RacePlaceMode::None,
            panel_section: RacePanelSection::Track,
            selected_point: None,
            place_z: 0.0,
            default_radius: 8.0,
            radar_path: String::new(),
            world_size: 6000.0,
            world_center_x: 0.0,
            world_center_y: 0.0,
            list_scroll: 0.0,
            track_scroll: 0.0,
            loaded: false,
            preview_overlay: false,
            preview_overlay_last_click_at: -1.0,
            preview_overlay_last_click_pos: Vec2::ZERO,
            radar_tex: None,
            radar_tex_key: String::new(),
            radar_2d: false,
            radar_2d_dragging: false,
            radar_2d_drag_before: None,
            radar_2d_zoom: 1.0,
            radar_2d_pan_u: 0.0,
            radar_2d_pan_v: 0.0,
            radar_2d_panning: false,
            radar_2d_pan_last: Vec2::ZERO,
        }
    }
}

fn parse_options() -> Options {
    let draw_distance_percent = load_draw_distance_percent_preference();
    let mut options = Options {
        root: PathBuf::new(),
        root_from_cli: false,
        launch_mode: LaunchMode::Project,
        textures: true,
        meshes: true,
        render: true,
        ui: true,
        text: true,
        fast_vbo: true,
        vbo_selected_only: false,
        vbo_immediate: false,
        monitor: None,
        msaa_samples: requested_msaa_samples(),
        draw_distance_percent,
        draw_radius: draw_radius_for_percent(draw_distance_percent),
        part_budget: usize::MAX,
        vertex_budget: DEFAULT_VERTEX_BUDGET,
        lod_mode: LodMode::Swap,
        lod_radius: FAR_DRAW,
    };
    let mut args = env::args().skip(1).peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--no-textures" => options.textures = false,
            "--no-meshes" => options.meshes = false,
            "--no-render" => options.render = false,
            "--no-ui" => options.ui = false,
            "--no-text" => options.text = false,
            "--fast-vbo" => options.fast_vbo = true,
            "--no-fast-vbo" => options.fast_vbo = false,
            "--show-lod" => options.lod_mode = LodMode::ShowAll,
            "--lod-mode" => {
                if let Some(value) = args.next() {
                    options.lod_mode = match value.to_ascii_lowercase().as_str() {
                        "swap" => LodMode::Swap,
                        "detail" | "detail-only" | "none" | "off" => LodMode::DetailOnly,
                        "all" | "show-all" | "both" => LodMode::ShowAll,
                        _ => options.lod_mode,
                    };
                }
            }
            "--lod-radius" => {
                if let Some(value) = args.next() {
                    if let Ok(radius) = value.parse::<f32>() {
                        options.lod_radius = radius.max(100.0);
                    }
                }
            }
            "--vbo-selected-only" => options.vbo_selected_only = true,
            "--vbo-immediate" => options.vbo_immediate = true,
            "--monitor1" => options.monitor = Some(1),
            "--monitor2" => options.monitor = Some(2),
            "--monitor" => {
                if let Some(value) = args.next() {
                    if let Ok(monitor) = value.parse::<u32>() {
                        options.monitor = Some(monitor);
                    }
                }
            }
            "--no-msaa" => options.msaa_samples = 1,
            "--msaa" => {
                if let Some(value) = args.next() {
                    if let Ok(samples) = value.parse::<i32>() {
                        options.msaa_samples = clamp_msaa_samples(samples);
                    }
                }
            }
            "--draw-radius" => {
                if let Some(value) = args.next() {
                    if let Ok(radius) = value.parse::<f32>() {
                        options.draw_radius = radius.max(100.0);
                    }
                }
            }
            "--part-budget" => {
                if let Some(value) = args.next() {
                    if let Ok(budget) = value.parse::<usize>() {
                        options.part_budget = budget.max(1);
                    }
                }
            }
            "--vertex-budget" => {
                if let Some(value) = args.next() {
                    if let Ok(budget) = value.parse::<usize>() {
                        options.vertex_budget = budget.max(1_000);
                    }
                }
            }
            "--help" | "-h" => {
                println!(
                    "Usage: eagle-editor [resource_path] [--no-textures] [--no-meshes] [--no-render] [--no-ui] [--no-text] [--fast-vbo] [--no-fast-vbo] [--monitor N|--monitor1|--monitor2] [--msaa 2|4] [--no-msaa] [--vbo-selected-only] [--vbo-immediate] [--draw-radius N] [--part-budget N] [--vertex-budget N] [--lod-mode swap|detail|all] [--lod-radius N]"
                );
                std::process::exit(0);
            }
            value if !value.starts_with('-') => {
                options.root = PathBuf::from(value);
                options.root_from_cli = true;
            }
            _ => eprintln!("Unknown option: {arg}"),
        }
    }
    options
}

fn monitor_origin(monitor: u32) -> Option<(u32, u32)> {
    let output = Command::new("kscreen-doctor")
        .arg("--json")
        .output()
        .ok()
        .filter(|output| output.status.success())?;
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    let outputs = json.get("outputs")?.as_array()?;
    for output in outputs {
        if output.get("id").and_then(|id| id.as_u64()) != Some(monitor as u64) {
            continue;
        }
        let pos = output.get("pos")?;
        let x = pos.get("x")?.as_i64()?.max(0) as u32;
        let y = pos.get("y")?.as_i64()?.max(0) as u32;
        return Some((x, y));
    }
    match monitor {
        1 => Some((0, 0)),
        2 => Some((2560, 0)),
        _ => None,
    }
}

#[macroquad::main(conf)]
async fn main() {
    gl_set_drawcall_buffer_capacity(220_000, 360_000);
    init_raw_gl();
    prevent_quit();
    // Embedded fonts: pre-rasterized into texture atlases at startup so text is
    // crisp and renders reliably alongside the raw-GL 3D pipeline. Rasterized at
    // a high resolution and scaled down per draw for clean small text.
    if let Some(font) = build_font(include_bytes!("../assets/fonts/NotoSans-Regular.ttf"), 48.0) {
        let _ = FONT_REGULAR.set(font);
    } else {
        eprintln!("Warning: failed to build regular font");
    }
    if let Some(font) = build_font(include_bytes!("../assets/fonts/NotoSans-Bold.ttf"), 48.0) {
        let _ = FONT_BOLD.set(font);
    } else {
        eprintln!("Warning: failed to build bold font");
    }
    let ui_font = Font::default();
    if let Err(error) = validate_legacy_gl_context() {
        eprintln!("{error}");
        show_graphics_startup_error(&ui_font, &error).await;
        return;
    }
    let options = parse_options();
    UI_TEXT_ENABLED.store(options.text, Ordering::Relaxed);
    if !ensure_gta_sa_dir_configured(&ui_font).await {
        return;
    }
    if let Some(monitor) = options.monitor {
        if let Some((x, y)) = monitor_origin(monitor) {
            macroquad::miniquad::window::set_window_position(x, y);
            eprintln!("Monitor target: #{monitor} at {x},{y}");
        } else {
            eprintln!("Warning: monitor #{monitor} was not found");
        }
    }
    if options.root_from_cli {
        eprintln!(
            "Options: root={}, textures={}, meshes={}, render={}, ui={}, text={}, fast_vbo={}, msaa={}x, draw_radius={}, part_budget={}, vertex_budget={}",
            options.root.display(),
            options.textures,
            options.meshes,
            options.render,
            options.ui,
            options.text,
            options.fast_vbo,
            options.msaa_samples,
            options.draw_radius,
            budget_label(options.part_budget),
            budget_label(options.vertex_budget)
        );
    } else {
        eprintln!("Options: project picker, no resource auto-loaded");
    }
    let mut runtime = if options.root_from_cli {
        let mut app = load_app(options, ui_font).await;
        maybe_prompt_autosave_restore(&mut app);
        RuntimeState::Editor(app)
    } else {
        let icons = load_icons().await;
        RuntimeState::ProjectPicker(new_project_picker(options, ui_font, icons))
    };

    loop {
        if is_quit_requested() {
            match &mut runtime {
                RuntimeState::Editor(app) => {
                    if !app.quit_after_persist {
                        if has_unsaved_changes(app) {
                            if app.confirm_dialog.is_none() {
                                app.confirm_dialog = Some(ConfirmDialog {
                                    action: ConfirmAction::Quit,
                                    title: "Unsaved Changes".to_string(),
                                    body: "Save a WIP snapshot before exiting?".to_string(),
                                    detail:
                                        "Save WIP keeps map/light XML outside the resource; COL byte edits need resource Save."
                                            .to_string(),
                                    primary_label: "Save WIP".to_string(),
                                    secondary_label: Some("Discard".to_string()),
                                    secondary_action: None,
                                });
                            }
                        } else {
                            app.quit_after_persist = true;
                        }
                    }
                }
                _ => macroquad::miniquad::window::quit(),
            }
        }
        match &mut runtime {
            RuntimeState::ProjectPicker(picker) => {
                if let Some(job) = update_project_picker(picker) {
                    draw_loading_resource(&job.root, "Reading archives");
                    runtime = RuntimeState::Loading(Some(job));
                } else {
                    draw_project_picker(picker);
                }
            }
            RuntimeState::Loading(job_slot) => {
                if let Some(mut job) = job_slot.take() {
                    let done = job.step();
                    draw_loading_resource(&job.root, job.progress());
                    if done {
                        let root = job.root.clone();
                        let source = job.source;
                        let mut app = job.finish();
                        save_recent_project(&root);
                        if source == LoadSceneSource::Saved {
                            maybe_prompt_autosave_restore(&mut app);
                        }
                        runtime = RuntimeState::Editor(app);
                    } else {
                        *job_slot = Some(job);
                    }
                } else {
                    draw_loading_resource(Path::new(""), "No load job is active.");
                }
            }
            RuntimeState::Editor(app) => {
                let viewport = editor_viewport_rect();
                {
                    thread_local!(static LAST_RENDER_SZ: std::cell::Cell<(f32, f32)> =
                        const { std::cell::Cell::new((0.0, 0.0)) });
                    let cur = (screen_width(), screen_height());
                    let prev = LAST_RENDER_SZ.with(|c| c.get());
                    let resized = prev.0 > 0.0
                        && ((cur.0 - prev.0).abs() > 0.5 || (cur.1 - prev.1).abs() > 0.5);
                    if resized {
                        app.render_settle_until = get_time() + SETTLE_SECONDS;
                        app.camera.looking = false;
                        set_cursor_grab(false);
                        show_mouse(true);
                    }
                    LAST_RENDER_SZ.with(|c| c.set(cur));
                }
                apply_pending_project_camera_restore(app);
                poll_load_picker(app);
                poll_vehicle_folder_picker(app);
                poll_custom_vehicle_dictionary_scan(app);
                poll_vehicle_loader_picker(app);
                poll_vehicle_loader_scan(app);
                poll_vehicle_build(app);
                poll_vehicle_reload(app);
                poll_vehicle_texture_export(app);
                poll_vehicle_texture_replace(app);
                poll_vehicle_collision_copy(app);
                poll_blender_import(app);
                update_editing_txd_import(app);
                update_editing_dff_texture_replace(app);
                poll_editing_img_save(app);
                update_editing_img_merge(app);
                poll_manual_save(app);
                if let Some(mut job) = app.load_job.take() {
                    let done = job.step();
                    draw_loading_resource(&job.root, job.progress());
                    if done {
                        let root = job.root.clone();
                        let source = job.source;
                        let mut app_done = job.finish();
                        save_recent_project(&root);
                        if source == LoadSceneSource::Saved {
                            maybe_prompt_autosave_restore(&mut app_done);
                        }
                        *app = app_done;
                    } else {
                        app.load_job = Some(job);
                    }
                    next_frame().await;
                    continue;
                }
                update_txd_cleanup_job(app);
                update_purge_unused_assets(app);
                update_img_archive_rebalance(app);
                update_asset_optimization_scan(app);
                update_asset_optimization_job(app);
                update_object_bounds_fix(app);
                update_corona_generation_job(app);
                update_day_night_merge_job(app);
                update_light_lod_job(app);
                update_fracture_generation_job(app);
                update_dff_geometry_job(app);
                update_dff_material_limit_repair_job(app);
                update_oversized_chunk_job(app);
                update_classify_scan(app);
                update_collision_generation_job(app);
                update_shadow_mesh_generation_job(app);
                update_collision_cuboid_audit_job(app);
                update_lod_generation_job(app);
                update_instance_lod_removal_job(app);
                update_dff_repair_job(app);
                poll_duplicate_placement_scan(app);
                update_texture_match_selection_job(app);
                update_water_texture_conversion_job(app);
                update_preview_world_uv_job(app);
                update_editing_linked_selection_job(app);
                poll_lod_audit(app);
                poll_missing_texture_review(app);
                update_autosave(app);
                record_activity_status(app);
                // Native file drops are consumed and cleared once per frame.
                // Poll them outside the regular input gate so background jobs
                // and modal UI can never make the OS payload disappear.
                let handled_file_drop = update_dropped_resource_files(app);
                let background_blocks_editor_input = app.blender_import_rx.is_some()
                    || app.manual_save_job.is_some()
                    || app.autosave_cleanup_rx.is_some()
                    || app.asset_optimization_scan_rx.is_some()
                    || app.asset_optimization_job.is_some()
                    || app.purge_unused_job.is_some()
                    || app.img_archive_rebalance_job.is_some()
                    || app.object_bounds_fix_job.is_some()
                    || app.dff_repair_refresh.is_some()
                    || app.corona_generation_job.is_some()
                    || app.day_night_merge_job.is_some()
                    || app.light_lod_job.is_some()
                    || app.fracture_generation_job.is_some()
                    || app.dff_geometry_job.is_some()
                    || app.dff_material_limit_repair_job.is_some()
                    || app.oversized_chunk_job.is_some()
                    || app.collision_generation_job.is_some()
                    || app.shadow_mesh_generation_job.is_some()
                    || app.lod_generation_job.is_some()
                    || app.instance_lod_removal_job.is_some()
                    || app.water_texture_conversion_job.is_some()
                    || app.editing.txd_import_rx.is_some()
                    || app.editing.dff_texture_replace_rx.is_some()
                    || app.editing.txd_refresh_job.is_some()
                    || app.editing.save_rx.is_some()
                    || app.editing.merge_rx.is_some()
                    || app.editing.merge_apply_job.is_some()
                    || app.vehicle_browser.collision_copy_rx.is_some()
                    || app.editing.linked_selection_job.is_some();
                if !background_blocks_editor_input && !handled_file_drop {
                    update_editor_input(app, viewport);
                } else {
                    // Background asset work blocks scene mutations, but the
                    // status strip and activity console must remain interactive.
                    let mouse: Vec2 = mouse_position().into();
                    if !update_blender_import_dialog_input(app, mouse) {
                        let _ = update_save_log_input(app, mouse);
                    }
                }
                update_simulation(app);
                if let Some(root) = app.pending_load_root.take() {
                    let source = app.pending_load_source;
                    app.pending_load_source = LoadSceneSource::Saved;
                    persist_project_session(app, viewport);
                    let mut options = app.options.clone();
                    options.root = root.clone();
                    let icons = app.icons.clone();
                    release_loaded_resource(app);
                    app.load_job = Some(LoadJob::new_with_source(
                        options,
                        Font::default(),
                        icons,
                        source,
                    ));
                    draw_loading_resource(&root, "Reading archives");
                    next_frame().await;
                    continue;
                }
                update_bake_job(app);
                record_activity_status(app);
                if !editor_text_input_active(app)
                    && app.load_dialog.is_none()
                    && app.import_asset_dialog.is_none()
                    && app.preferences_dialog.is_none()
                    && app.save_as_dialog.is_none()
                    && app.dff_picker_rx.is_none()
                    && app.blender_import_setup.is_none()
                    && app.oversized_chunk_dialog.is_none()
                    && app.classify_dialog.is_none()
                    && app.blender_import_rx.is_none()
                    && !app.blender_import_dialog_open
                    && app.dff_replace_choice_dialog.is_none()
                    && app.dff_merge_choice_dialog.is_none()
                    && app.dff_optimize_dialog.is_none()
                    && app.dff_txd_pair_dialog.is_none()
                    && app.dff_texture_duplicate_dialog.is_none()
                    && app.dff_texture_view_dialog.is_none()
                    && app.element_id_rename_dialog.is_none()
                    && app.element_replace_with_dialog.is_none()
                    && app.missing_texture_dialog.is_none()
                    && app.texture_archive_dialog.is_none()
                    && app.dff_prelight_import_dialog.is_none()
                    && app.missing_col_dialog.is_none()
                    && app.lod_batch_dialog.is_none()
                    && app.confirm_dialog.is_none()
                    && !app.vehicle_browser.manage_dictionaries
                    && app.vehicle_browser.build_dialog.is_none()
                    && !app.save_log_open
                    && app.txd_cleanup_job.is_none()
                    && app.asset_optimization_scan_rx.is_none()
                    && app.asset_optimization_job.is_none()
                    && app.purge_unused_job.is_none()
                    && app.img_archive_rebalance_job.is_none()
                    && app.dff_repair_rx.is_none()
                    && app.dff_repair_refresh.is_none()
                    && app.manual_save_job.is_none()
                    && !editing_material_picker_is_open(app)
                    && !app.quit_after_persist
                    && app.pending_camera_restore.is_none()
                    && !(app.active_tab == AppTab::Simulate && app.sim.playing)
                {
                    update_camera(app, viewport);
                }
                update_display_fps(app);
                let t_scene = Instant::now();
                draw_scene(app, viewport);
                let scene_ms = t_scene.elapsed().as_secs_f32() * 1000.0;
                let t_ui = Instant::now();
                if app.options.ui {
                    draw_panel(app, viewport);
                    draw_load_dialog(app);
                    draw_import_asset_dialog(app);
                    draw_preferences_dialog(app);
                    draw_save_as_dialog(app);
                    draw_dff_replace_choice_dialog(app);
                    draw_dff_merge_choice_dialog(app);
                    draw_dff_optimize_dialog(app);
                    draw_dff_txd_pair_dialog(app);
                    draw_dff_texture_duplicate_dialog(app);
                    draw_dff_texture_view_dialog(app);
                    draw_element_id_rename_dialog(app);
                    draw_element_replace_with_dialog(app);
                    draw_missing_texture_dialog(app);
                    draw_texture_archive_dialog(app);
                    draw_dff_prelight_import_dialog(app);
                    draw_missing_col_dialog(app);
                    draw_lod_batch_dialog(app);
                    draw_confirm_dialog(app);
                    draw_save_log_dialog(app);
                    draw_blender_import_dialog(app);
                    draw_blender_import_setup_dialog(app);
                    draw_oversized_chunk_dialog(app);
                    draw_classify_dialog(app);
                    draw_pending_ui_tooltip(&app.ui_font);
                }
                let ui_ms = t_ui.elapsed().as_secs_f32() * 1000.0;
                // Resize / slow-frame diagnostics: log whenever the window size
                // changes or a frame's draw work spikes, so a resize freeze can
                // be attributed to the scene render vs. the UI pass.
                {
                    thread_local!(static LAST_SZ: std::cell::Cell<(f32, f32)> =
                        const { std::cell::Cell::new((0.0, 0.0)) });
                    let cur = (screen_width(), screen_height());
                    let prev = LAST_SZ.with(|c| c.get());
                    let resized = (cur.0 - prev.0).abs() > 0.5 || (cur.1 - prev.1).abs() > 0.5;
                    if resized {
                        LAST_SZ.with(|c| c.set(cur));
                    }
                    if resized || scene_ms + ui_ms > 80.0 {
                        eprintln!(
                            "frame timing: {}x{} resized={} scene={:.1}ms ui={:.1}ms drawn_verts={}",
                            cur.0 as i32,
                            cur.1 as i32,
                            resized,
                            scene_ms,
                            ui_ms,
                            app.last_drawn_vertices
                        );
                    }
                }
                if app.quit_after_persist {
                    persist_project_session(app, viewport);
                    macroquad::miniquad::window::quit();
                }
            }
        }
        macroquad::miniquad::window::schedule_update();
        next_frame().await;
    }
}
