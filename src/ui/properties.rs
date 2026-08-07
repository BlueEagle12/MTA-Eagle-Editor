use super::super::*;

pub(crate) fn properties_tab_label(tab: PropertiesTab) -> &'static str {
    match tab {
        PropertiesTab::Element => "Element",
        PropertiesTab::Settings => "Settings",
        PropertiesTab::History => "History",
    }
}

pub(crate) fn properties_tabs() -> [PropertiesTab; 3] {
    [
        PropertiesTab::Element,
        PropertiesTab::Settings,
        PropertiesTab::History,
    ]
}

pub(crate) fn properties_tab_rect(slot: usize) -> Rect {
    let x = screen_width() - RIGHT_PANEL_W + 26.0;
    let step = ((RIGHT_PANEL_W - 52.0) / 3.0).floor();
    Rect::new(x + slot as f32 * step, TOP_H + 56.0, step - 8.0, 28.0)
}

pub(crate) fn right_panel_rect() -> Rect {
    Rect::new(
        screen_width() - RIGHT_PANEL_W,
        TOP_H,
        RIGHT_PANEL_W,
        screen_height() - TOP_H - STATUS_H,
    )
}

pub(crate) fn editor_viewport_rect() -> Rect {
    Rect::new(
        PANEL_W,
        TOP_H,
        (screen_width() - PANEL_W - RIGHT_PANEL_W).max(16.0),
        (screen_height() - TOP_H - STATUS_H).max(16.0),
    )
}

pub(crate) fn properties_content_top() -> f32 {
    TOP_H + 100.0
}

pub(crate) fn properties_content_rect() -> Rect {
    let top = properties_content_top();
    let bottom = screen_height() - STATUS_H - 12.0;
    Rect::new(
        screen_width() - RIGHT_PANEL_W + 12.0,
        top,
        RIGHT_PANEL_W - 24.0,
        (bottom - top).max(1.0),
    )
}

pub(crate) fn inspector_panel_content_rect() -> Rect {
    let top = TOP_H + 52.0;
    let bottom = screen_height() - STATUS_H - 12.0;
    Rect::new(
        screen_width() - RIGHT_PANEL_W + 12.0,
        top,
        RIGHT_PANEL_W - 24.0,
        (bottom - top).max(1.0),
    )
}

pub(crate) fn panel_scroll_max_for_bottom(content_bottom: f32) -> f32 {
    let view = inspector_panel_content_rect();
    (content_bottom - (view.y + view.h)).max(0.0)
}

pub(crate) fn draw_inspector_scrollbar(app: &AppState, max_scroll: f32) {
    if max_scroll <= 0.0 {
        return;
    }
    let view = inspector_panel_content_rect();
    let track = Rect::new(view.x + view.w - 7.0, view.y + 4.0, 4.0, view.h - 8.0);
    let content_h = view.h + max_scroll;
    let thumb_h = (track.h * view.h / content_h).clamp(28.0, track.h);
    let thumb_y =
        track.y + (track.h - thumb_h) * (app.properties_scroll / max_scroll).clamp(0.0, 1.0);
    draw_rrect(
        track.x,
        thumb_y,
        track.w,
        thumb_h,
        3.0,
        Color::new(0.34, 0.36, 0.40, 0.9),
    );
}

pub(crate) fn properties_scroll_max(app: &AppState) -> f32 {
    if app.active_tab == AppTab::Validation {
        return validation_scroll_max(app);
    }
    if app.active_tab == AppTab::LodAudit {
        return lod_audit_scroll_max(app);
    }
    if app.active_tab == AppTab::TextureReview {
        return 0.0;
    }
    if app.active_tab == AppTab::Scene {
        return scene_panel_scroll_max();
    }
    if app.active_tab == AppTab::Collisions {
        return collision_panel_scroll_max();
    }
    if app.active_tab == AppTab::Lights {
        return lights_panel_scroll_max();
    }
    if app.active_tab == AppTab::Bake {
        return bake_panel_scroll_max(app);
    }
    if app.active_tab == AppTab::Water {
        return water_panel_scroll_max();
    }
    if app.active_tab != AppTab::Preview || app.properties_tab != PropertiesTab::Element {
        return 0.0;
    }
    element_panel_max_scroll(&element_panel_layout(app))
}

pub(crate) fn clamp_properties_scroll(app: &mut AppState) {
    let max_scroll = properties_scroll_max(app);
    app.properties_scroll = app.properties_scroll.clamp(0.0, max_scroll);
}

pub(crate) fn split_flags(value: &str) -> Vec<String> {
    value
        .split(|ch: char| ch == ',' || ch == ';' || ch == '|' || ch.is_whitespace())
        .map(str::trim)
        .filter(|flag| !flag.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub(crate) fn model_flag_bit_name(bit: u32) -> Option<&'static str> {
    match bit {
        0 => Some("is_road"),
        2 => Some("draw_last"),
        3 => Some("additive"),
        6 => Some("no_zbuffer_write"),
        7 => Some("dont_receive_shadows"),
        9 => Some("is_glass_type_1"),
        10 => Some("is_glass_type_2"),
        11 => Some("is_garage_door"),
        12 => Some("is_damagable"),
        13 => Some("is_tree"),
        14 => Some("is_palm"),
        15 => Some("does_not_collide_with_flyer"),
        20 => Some("is_tag"),
        21 => Some("disable_backface_culling"),
        22 => Some("is_breakable_statue"),
        50 => Some("disable_collisions"),
        _ => None,
    }
}

pub(crate) fn model_flag_decimal_name(decimal: u64) -> Option<&'static str> {
    match decimal {
        1 => Some("is_road"),
        4 => Some("draw_last"),
        8 => Some("additive"),
        64 => Some("no_zbuffer_write"),
        128 => Some("dont_receive_shadows"),
        512 => Some("is_glass_type_1"),
        1024 => Some("is_glass_type_2"),
        2048 => Some("is_garage_door"),
        4096 => Some("is_damagable"),
        8192 => Some("is_tree"),
        16384 => Some("is_palm"),
        32768 => Some("does_not_collide_with_flyer"),
        1_048_576 => Some("is_tag"),
        2_097_152 => Some("disable_backface_culling"),
        4_194_304 => Some("is_breakable_statue"),
        _ => None,
    }
}

pub(crate) fn parse_model_flag_number(token: &str) -> Option<u64> {
    let token = token.trim();
    if let Some(hex) = token
        .strip_prefix("0x")
        .or_else(|| token.strip_prefix("0X"))
    {
        u64::from_str_radix(hex, 16).ok()
    } else {
        token.parse::<u64>().ok()
    }
}

pub(crate) fn flag_token_matches(token: &str, flag: &str) -> bool {
    token == flag
        || parse_model_flag_number(token).is_some_and(|value| {
            model_flag_bit_name(value as u32).is_some_and(|name| name == flag)
                || model_flag_decimal_name(value).is_some_and(|name| name == flag)
        })
}

pub(crate) fn placement_disable_backface_culling(
    placement: &Placement,
    definitions: &HashMap<String, Definition>,
) -> bool {
    placement_override_flag_enabled(placement, "double_sided")
        || placement_definition_disable_backface_culling(placement, definitions)
}

pub(crate) fn placement_disable_collisions(
    placement: &Placement,
    definitions: &HashMap<String, Definition>,
) -> bool {
    placement_override_flag_enabled(placement, "disable_collisions")
        || definitions
            .get(&placement.id)
            .is_some_and(|def| definition_flag_enabled(def, "disable_collisions"))
}

pub(crate) fn placement_alpha(placement: &Placement) -> f32 {
    placement
        .attrs
        .get("alpha")
        .and_then(|value| value.trim().parse::<u8>().ok())
        .map(|alpha| alpha as f32 / 255.0)
        .unwrap_or(1.0)
        .clamp(0.0, 1.0)
}

pub(crate) fn placement_scale(placement: &Placement) -> f32 {
    placement
        .attrs
        .get("scale")
        .and_then(|value| value.trim().parse::<f32>().ok())
        .filter(|scale| scale.is_finite() && *scale > 0.0)
        .unwrap_or(1.0)
}

pub(crate) fn placement_definition_disable_backface_culling(
    placement: &Placement,
    definitions: &HashMap<String, Definition>,
) -> bool {
    definitions
        .get(&placement.id)
        .is_some_and(|def| definition_flag_enabled(def, "disable_backface_culling"))
}

pub(crate) fn definition_flag_enabled(def: &Definition, flag: &str) -> bool {
    def.attrs
        .get(flag)
        .is_some_and(|value| value != "false" && value != "0")
        || def.attrs.get("flags").is_some_and(|flags| {
            split_flags(flags)
                .iter()
                .any(|value| flag_token_matches(value, flag))
        })
        || def.attrs.get("gtaFlags").is_some_and(|flags| {
            split_flags(flags)
                .iter()
                .any(|value| flag_token_matches(value, flag))
        })
}

pub(crate) fn set_definition_flag(def: &mut Definition, flag: &str, enabled: bool) {
    let mut flags = def
        .attrs
        .get("flags")
        .map(|value| split_flags(value))
        .unwrap_or_default();
    flags.retain(|value| !flag_token_matches(value, flag));
    if enabled {
        flags.push(flag.to_string());
        def.attrs.insert(flag.to_string(), "true".to_string());
    } else {
        def.attrs.remove(flag);
    }
    def.attrs.insert("flags".to_string(), flags.join(","));
}

pub(crate) fn set_optional_attr(attrs: &mut BTreeMap<String, String>, key: &str, value: String) {
    if value.trim().is_empty() {
        attrs.remove(key);
    } else {
        attrs.insert(key.to_string(), value);
    }
}

// ---------------------------------------------------------------------------
// Element properties tab: collapsible-section layout.
//
// A single layout function computes every control rect from the current
// state; both the draw pass and the click handlers consume the same struct,
// so hit boxes always match pixels. Collapsed sections report off-screen
// rects for their controls so they can't be drawn or clicked.
// ---------------------------------------------------------------------------

pub(crate) const PHYSICS_ATTR_KEYS: [&str; 10] = [
    "physicsRoot",
    "simulated",
    "mass",
    "turnMass",
    "airResistance",
    "elasticity",
    "buoyancy",
    "centerOfMassX",
    "centerOfMassY",
    "centerOfMassZ",
];
pub(crate) const PHYSICS_ROOT_ATTR_KEYS: [&str; 5] = [
    "physicsRoot",
    "physicsRootModel",
    "physics_root",
    "physics_root_model",
    "physicalPropsRoot",
];

pub(crate) const ELEM_SECTION_COUNT: usize = 9;
pub(crate) const ELEM_SECTION_TITLES: [&str; ELEM_SECTION_COUNT] = [
    "Element",
    "LOD",
    "Transform",
    "Model",
    "Collision",
    "Flags",
    "Flag Overrides",
    "Physics",
    "Textures",
];

pub(crate) fn element_default_collapsed() -> [bool; ELEM_SECTION_COUNT] {
    let mut collapsed = [false; ELEM_SECTION_COUNT];
    collapsed[8] = true;
    collapsed
}

pub(crate) const SETTINGS_SECTION_COUNT: usize = 4;
pub(crate) const SETTINGS_SECTION_TITLES: [&str; SETTINGS_SECTION_COUNT] = [
    "Transform",
    "Camera",
    "Box Select",
    "Project / Global Transform",
];

pub(crate) fn settings_default_collapsed() -> [bool; SETTINGS_SECTION_COUNT] {
    [false; SETTINGS_SECTION_COUNT]
}

const ELEM_OFFSCREEN: Rect = Rect {
    x: -10000.0,
    y: -10000.0,
    w: 0.0,
    h: 0.0,
};

// Vertical space reserved above an input row for its floating label.
const ELEM_LABEL_GAP: f32 = 22.0;
const ELEM_FIELD_H: f32 = 30.0;
const ELEM_ROW_GAP: f32 = 8.0;
pub(crate) const PREVIEW_TEXTURE_ROW_H: f32 = 56.0;

#[derive(Clone)]
pub(crate) struct PreviewMaterialEntry {
    pub(crate) material_index: usize,
    pub(crate) texture_name: String,
    pub(crate) fingerprint: Option<TextureContentFingerprint>,
    pub(crate) face_count: usize,
    pub(crate) missing: bool,
    pub(crate) texture_id: u32,
    pub(crate) texture_width: u16,
    pub(crate) texture_height: u16,
}

pub(crate) fn preview_material_entries(
    app: &AppState,
    placement_index: usize,
) -> Vec<PreviewMaterialEntry> {
    let Some(placement) = app.placements.get(placement_index) else {
        return Vec::new();
    };
    let Some(mesh) = element_mesh(app, placement) else {
        return Vec::new();
    };
    let mut entries = BTreeMap::<usize, PreviewMaterialEntry>::new();
    for part in &mesh.parts {
        let entry = entries
            .entry(part.material_index)
            .or_insert_with(|| PreviewMaterialEntry {
                material_index: part.material_index,
                texture_name: part.texture_name.clone(),
                fingerprint: part.texture_fingerprint,
                face_count: 0,
                missing: false,
                texture_id: part.texture,
                texture_width: part.texture_width,
                texture_height: part.texture_height,
            });
        entry.face_count += part.face_indices.len();
        entry.missing |= part.texture_missing;
        if entry.fingerprint.is_none() {
            entry.fingerprint = part.texture_fingerprint;
        }
        if entry.texture_name.is_empty() && !part.texture_name.is_empty() {
            entry.texture_name = part.texture_name.clone();
        }
        if entry.texture_id == 0 && part.texture != 0 {
            entry.texture_id = part.texture;
            entry.texture_width = part.texture_width;
            entry.texture_height = part.texture_height;
        }
    }
    entries.into_values().collect()
}

pub(crate) fn preview_texture_thumbnail_rect(row: Rect) -> Rect {
    Rect::new(row.x + 4.0, row.y + 4.0, 44.0, 44.0)
}

pub(crate) fn preview_texture_view_rect(row: Rect) -> Rect {
    Rect::new(row.x + row.w - 54.0, row.y + 13.0, 48.0, 26.0)
}

pub(crate) fn open_preview_texture_view_dialog(
    app: &mut AppState,
    placement_index: usize,
    material_index: usize,
) -> bool {
    let Some(entry) = preview_material_entry(app, placement_index, material_index) else {
        app.status_message = "The selected texture material is no longer available.".to_string();
        return false;
    };
    if entry.texture_id == 0 {
        app.status_message = format!(
            "Texture '{}' is not resolved and cannot be previewed.",
            entry.texture_name
        );
        return false;
    }
    let width = entry.texture_width.max(1);
    let height = entry.texture_height.max(1);
    let txd_name = app
        .placements
        .get(placement_index)
        .and_then(|placement| definition_txd_name(&app.definitions, &placement.id))
        .map(|name| asset_key(name, ".txd"))
        .unwrap_or_else(|| "Resolved texture pool".to_string());
    app.dff_texture_view_dialog = Some(DffTextureViewDialog {
        texture_name: entry.texture_name,
        txd_name,
        width,
        height,
        texture: None,
        raw_texture: entry.texture_id,
    });
    true
}

pub(crate) fn preview_material_entry(
    app: &AppState,
    placement_index: usize,
    material_index: usize,
) -> Option<PreviewMaterialEntry> {
    preview_material_entries(app, placement_index)
        .into_iter()
        .find(|entry| entry.material_index == material_index)
}

pub(crate) struct ElementPanelLayout {
    pub(crate) content: Rect,
    pub(crate) content_height: f32,
    pub(crate) info_top: f32,
    pub(crate) headers: [Rect; ELEM_SECTION_COUNT],
    pub(crate) type_buttons: Option<[Rect; 3]>,
    pub(crate) id: Option<Rect>,
    pub(crate) lod_parent: Option<Rect>,
    pub(crate) lod_parent_select: Option<Rect>,
    pub(crate) self_lod: Option<Rect>,
    pub(crate) remove_instance_lods: Option<Rect>,
    pub(crate) generate_lod: Option<Rect>,
    pub(crate) light_lod: Option<Rect>,
    pub(crate) unique_id: Option<Rect>,
    pub(crate) select_same_id: Option<Rect>,
    pub(crate) copy_position: Option<Rect>,
    pub(crate) copy_rotation: Option<Rect>,
    pub(crate) pos: Option<[Rect; 3]>,
    pub(crate) rot: Option<[Rect; 3]>,
    pub(crate) dim: Option<Rect>,
    pub(crate) interior: Option<Rect>,
    pub(crate) dff: Option<Rect>,
    pub(crate) native_model: Option<Rect>,
    pub(crate) txd: Option<Rect>,
    pub(crate) readonly_note_y: Option<f32>,
    pub(crate) export_dff: Option<Rect>,
    pub(crate) replace_dff: Option<Rect>,
    pub(crate) dff_editor: Option<Rect>,
    pub(crate) txd_editor: Option<Rect>,
    pub(crate) blender_position: Option<Rect>,
    pub(crate) find_missing: Option<Rect>,
    pub(crate) col: Option<Rect>,
    pub(crate) lod: Option<Rect>,
    pub(crate) assign_lod: Option<Rect>,
    pub(crate) export_col: Option<Rect>,
    pub(crate) replace_col: Option<Rect>,
    pub(crate) col_editor: Option<Rect>,
    pub(crate) merge_vertex_lighting: Option<Rect>,
    pub(crate) time_in: Option<Rect>,
    pub(crate) time_out: Option<Rect>,
    pub(crate) flags: Option<Vec<Rect>>,
    pub(crate) override_flags: Option<Vec<Rect>>,
    pub(crate) alpha: Option<Rect>,
    pub(crate) scale: Option<Rect>,
    pub(crate) physics_scope: Option<[Rect; 2]>,
    pub(crate) physics_root: Option<Rect>,
    pub(crate) physics_dimensions: Option<[Rect; 3]>,
    pub(crate) physics_simulated: Option<Rect>,
    pub(crate) physics_mass: Option<Rect>,
    pub(crate) physics_turn_mass: Option<Rect>,
    pub(crate) physics_air_resistance: Option<Rect>,
    pub(crate) physics_elasticity: Option<Rect>,
    pub(crate) physics_buoyancy: Option<Rect>,
    pub(crate) physics_center_of_mass: Option<[Rect; 3]>,
    pub(crate) physics_clear: Option<Rect>,
    pub(crate) texture_rows: Option<Vec<Rect>>,
}

fn elem_full_w() -> f32 {
    properties_content_rect().w - 12.0
}

fn elem_full(x0: f32, y: f32, h: f32) -> Rect {
    Rect::new(x0, y, elem_full_w(), h)
}

fn elem_half(x0: f32, y: f32, slot: usize, h: f32) -> Rect {
    let w = (elem_full_w() - 18.0) * 0.5;
    Rect::new(x0 + slot as f32 * (w + 18.0), y, w, h)
}

fn elem_third(x0: f32, y: f32) -> [Rect; 3] {
    let w = (elem_full_w() - 36.0) / 3.0;
    [
        Rect::new(x0, y, w, ELEM_FIELD_H),
        Rect::new(x0 + w + 18.0, y, w, ELEM_FIELD_H),
        Rect::new(x0 + 2.0 * (w + 18.0), y, w, ELEM_FIELD_H),
    ]
}

pub(crate) fn element_panel_layout(app: &AppState) -> ElementPanelLayout {
    let content = properties_content_rect();
    let x0 = content.x + 6.0;
    let readonly = selected_definition_is_readonly(app);
    let missing_textures = selected_definition_has_missing_textures(app);
    let collapsed = app.element_panel_collapsed;

    let mut layout = ElementPanelLayout {
        content,
        content_height: 0.0,
        info_top: 0.0,
        headers: [ELEM_OFFSCREEN; ELEM_SECTION_COUNT],
        type_buttons: None,
        id: None,
        lod_parent: None,
        lod_parent_select: None,
        self_lod: None,
        remove_instance_lods: None,
        generate_lod: None,
        light_lod: None,
        unique_id: None,
        select_same_id: None,
        copy_position: None,
        copy_rotation: None,
        pos: None,
        rot: None,
        dim: None,
        interior: None,
        dff: None,
        native_model: None,
        txd: None,
        readonly_note_y: None,
        export_dff: None,
        replace_dff: None,
        dff_editor: None,
        txd_editor: None,
        blender_position: None,
        find_missing: None,
        col: None,
        lod: None,
        assign_lod: None,
        export_col: None,
        replace_col: None,
        col_editor: None,
        merge_vertex_lighting: None,
        time_in: None,
        time_out: None,
        flags: None,
        override_flags: None,
        alpha: None,
        scale: None,
        physics_scope: None,
        physics_root: None,
        physics_dimensions: None,
        physics_simulated: None,
        physics_mass: None,
        physics_turn_mass: None,
        physics_air_resistance: None,
        physics_elasticity: None,
        physics_buoyancy: None,
        physics_center_of_mass: None,
        physics_clear: None,
        texture_rows: None,
    };

    let mut y = content.y + 6.0 - app.properties_scroll;
    layout.info_top = y;
    y += 120.0;

    // Section 0: Element (type + identity).
    layout.headers[0] = elem_full(x0, y, DFF_SEC_HEADER_H);
    y += DFF_SEC_HEADER_H + 6.0;
    if !collapsed[0] {
        let mut buttons = [ELEM_OFFSCREEN; 3];
        let btn_w = (elem_full_w() - 18.0) / 3.0;
        for (slot, rect) in buttons.iter_mut().enumerate() {
            *rect = Rect::new(x0 + slot as f32 * (btn_w + 9.0), y, btn_w, DFF_BTN_H);
        }
        layout.type_buttons = Some(buttons);
        y += DFF_BTN_H + ELEM_ROW_GAP + 2.0;
        layout.id = Some(elem_full(x0, y + ELEM_LABEL_GAP, ELEM_FIELD_H));
        y += ELEM_LABEL_GAP + ELEM_FIELD_H + ELEM_ROW_GAP;
        layout.unique_id = Some(elem_full(x0, y + ELEM_LABEL_GAP, ELEM_FIELD_H));
        y += ELEM_LABEL_GAP + ELEM_FIELD_H + ELEM_ROW_GAP;
        y += 4.0;
    }

    // Section 1: LOD assignment and generation.
    layout.headers[1] = elem_full(x0, y, DFF_SEC_HEADER_H);
    y += DFF_SEC_HEADER_H + 6.0;
    if !collapsed[1] {
        layout.lod_parent = Some(elem_half(x0, y + ELEM_LABEL_GAP, 0, ELEM_FIELD_H));
        layout.lod_parent_select = layout
            .lod_parent
            .map(|rect| Rect::new(rect.x + rect.w - 28.0, rect.y + 5.0, 20.0, rect.h - 10.0));
        layout.lod = Some(elem_half(x0, y + ELEM_LABEL_GAP, 1, ELEM_FIELD_H));
        y += ELEM_LABEL_GAP + ELEM_FIELD_H + ELEM_ROW_GAP;
        layout.self_lod = Some(elem_full(x0, y, DFF_BTN_H));
        y += DFF_BTN_H + ELEM_ROW_GAP;
        layout.remove_instance_lods = Some(elem_full(x0, y, DFF_BTN_H));
        y += DFF_BTN_H + ELEM_ROW_GAP;
        layout.select_same_id = Some(elem_full(x0, y, DFF_BTN_H));
        y += DFF_BTN_H + ELEM_ROW_GAP;
        layout.assign_lod = Some(elem_full(x0, y, DFF_BTN_H));
        y += DFF_BTN_H + ELEM_ROW_GAP;
        layout.generate_lod = Some(elem_full(x0, y, DFF_BTN_H));
        y += DFF_BTN_H + ELEM_ROW_GAP;
        layout.light_lod = Some(elem_full(x0, y, DFF_BTN_H));
        y += DFF_BTN_H + ELEM_ROW_GAP;
        y += 4.0;
    }

    // Section 2: Transform.
    layout.headers[2] = elem_full(x0, y, DFF_SEC_HEADER_H);
    y += DFF_SEC_HEADER_H + 6.0;
    if !collapsed[2] {
        layout.copy_position = Some(elem_half(x0, y, 0, ELEM_FIELD_H));
        layout.copy_rotation = Some(elem_half(x0, y, 1, ELEM_FIELD_H));
        y += ELEM_FIELD_H + ELEM_ROW_GAP + 2.0;
        layout.blender_position = Some(elem_full(x0, y, DFF_BTN_H));
        y += DFF_BTN_H + ELEM_ROW_GAP;
        layout.pos = Some(elem_third(x0, y + ELEM_LABEL_GAP));
        y += ELEM_LABEL_GAP + ELEM_FIELD_H + ELEM_ROW_GAP;
        layout.rot = Some(elem_third(x0, y + ELEM_LABEL_GAP));
        y += ELEM_LABEL_GAP + ELEM_FIELD_H + ELEM_ROW_GAP;
        layout.dim = Some(elem_half(x0, y + ELEM_LABEL_GAP, 0, ELEM_FIELD_H));
        layout.interior = Some(elem_half(x0, y + ELEM_LABEL_GAP, 1, ELEM_FIELD_H));
        y += ELEM_LABEL_GAP + ELEM_FIELD_H + ELEM_ROW_GAP;
        y += 4.0;
    }

    // Section 3: Model (DFF/TXD assets).
    layout.headers[3] = elem_full(x0, y, DFF_SEC_HEADER_H);
    y += DFF_SEC_HEADER_H + 6.0;
    if !collapsed[3] {
        layout.dff = Some(elem_half(x0, y + ELEM_LABEL_GAP, 0, ELEM_FIELD_H));
        layout.txd = Some(elem_half(x0, y + ELEM_LABEL_GAP, 1, ELEM_FIELD_H));
        y += ELEM_LABEL_GAP + ELEM_FIELD_H + ELEM_ROW_GAP;
        layout.native_model = Some(elem_full(x0, y + ELEM_LABEL_GAP, ELEM_FIELD_H));
        y += ELEM_LABEL_GAP + ELEM_FIELD_H + ELEM_ROW_GAP;
        if readonly {
            layout.readonly_note_y = Some(y + 14.0);
            y += 24.0;
        }
        layout.export_dff = Some(elem_half(x0, y, 0, DFF_BTN_H));
        layout.replace_dff = Some(elem_half(x0, y, 1, DFF_BTN_H));
        y += DFF_BTN_H + ELEM_ROW_GAP;
        layout.dff_editor = Some(elem_half(x0, y, 0, DFF_BTN_H));
        layout.txd_editor = Some(elem_half(x0, y, 1, DFF_BTN_H));
        y += DFF_BTN_H + ELEM_ROW_GAP;
        if missing_textures {
            layout.find_missing = Some(elem_full(x0, y, DFF_BTN_H));
            y += DFF_BTN_H + ELEM_ROW_GAP;
        }
        y += 4.0;
    }

    // Section 4: Collision.
    layout.headers[4] = elem_full(x0, y, DFF_SEC_HEADER_H);
    y += DFF_SEC_HEADER_H + 6.0;
    if !collapsed[4] {
        layout.col = Some(elem_full(x0, y + ELEM_LABEL_GAP, ELEM_FIELD_H));
        y += ELEM_LABEL_GAP + ELEM_FIELD_H + ELEM_ROW_GAP;
        layout.export_col = Some(elem_half(x0, y, 0, DFF_BTN_H));
        layout.replace_col = Some(elem_half(x0, y, 1, DFF_BTN_H));
        y += DFF_BTN_H + ELEM_ROW_GAP;
        layout.col_editor = Some(elem_full(x0, y, DFF_BTN_H));
        y += DFF_BTN_H + ELEM_ROW_GAP;
        if selected_day_night_merge_available(app) {
            layout.merge_vertex_lighting = Some(elem_full(x0, y, DFF_BTN_H));
            y += DFF_BTN_H + ELEM_ROW_GAP;
        }
        y += 4.0;
    }

    // Section 5: Flags (time in/out + definition flags).
    layout.headers[5] = elem_full(x0, y, DFF_SEC_HEADER_H);
    y += DFF_SEC_HEADER_H + 6.0;
    if !collapsed[5] {
        layout.time_in = Some(elem_half(x0, y + ELEM_LABEL_GAP, 0, ELEM_FIELD_H));
        layout.time_out = Some(elem_half(x0, y + ELEM_LABEL_GAP, 1, ELEM_FIELD_H));
        y += ELEM_LABEL_GAP + ELEM_FIELD_H + ELEM_ROW_GAP + 2.0;
        let mut flags = Vec::with_capacity(EAGLE_DEFINITION_FLAGS.len());
        let flag_w = (elem_full_w() - 18.0) * 0.5;
        for slot in 0..EAGLE_DEFINITION_FLAGS.len() {
            let col = slot % 2;
            let row = slot / 2;
            flags.push(Rect::new(
                x0 + col as f32 * (flag_w + 18.0),
                y + row as f32 * 26.0,
                flag_w,
                22.0,
            ));
        }
        let rows = EAGLE_DEFINITION_FLAGS.len().div_ceil(2);
        y += rows as f32 * 26.0;
        layout.flags = Some(flags);
        y += 4.0;
    }

    // Section 6: Flag Overrides (placement-level .map attributes).
    layout.headers[6] = elem_full(x0, y, DFF_SEC_HEADER_H);
    y += DFF_SEC_HEADER_H + 6.0;
    if !collapsed[6] {
        let mut flags = Vec::with_capacity(EAGLE_PLACEMENT_OVERRIDE_FLAGS.len());
        let flag_w = (elem_full_w() - 18.0) * 0.5;
        for slot in 0..EAGLE_PLACEMENT_OVERRIDE_FLAGS.len() {
            let col = slot % 2;
            let row = slot / 2;
            flags.push(Rect::new(
                x0 + col as f32 * (flag_w + 18.0),
                y + row as f32 * 26.0,
                flag_w,
                22.0,
            ));
        }
        let rows = EAGLE_PLACEMENT_OVERRIDE_FLAGS.len().div_ceil(2);
        y += rows as f32 * 26.0 + ELEM_ROW_GAP;
        layout.override_flags = Some(flags);
        layout.alpha = Some(elem_half(x0, y + ELEM_LABEL_GAP, 0, ELEM_FIELD_H));
        layout.scale = Some(elem_half(x0, y + ELEM_LABEL_GAP, 1, ELEM_FIELD_H));
        y += ELEM_LABEL_GAP + ELEM_FIELD_H + ELEM_ROW_GAP;
        y += 4.0;
    }

    // Section 7: Physics. The scope selects whether these canonical
    // attributes are stored on the model definition or this map placement.
    layout.headers[7] = elem_full(x0, y, DFF_SEC_HEADER_H);
    y += DFF_SEC_HEADER_H + 6.0;
    if !collapsed[7] {
        layout.physics_scope = Some([
            elem_half(x0, y, 0, DFF_BTN_H),
            elem_half(x0, y, 1, DFF_BTN_H),
        ]);
        y += DFF_BTN_H + ELEM_ROW_GAP;
        layout.physics_root = Some(elem_full(x0, y + ELEM_LABEL_GAP, ELEM_FIELD_H));
        y += ELEM_LABEL_GAP + ELEM_FIELD_H + ELEM_ROW_GAP;
        layout.physics_dimensions = Some(elem_third(x0, y + ELEM_LABEL_GAP));
        y += ELEM_LABEL_GAP + ELEM_FIELD_H + ELEM_ROW_GAP;
        layout.physics_simulated = Some(elem_full(x0, y, DFF_BTN_H));
        y += DFF_BTN_H + ELEM_ROW_GAP;
        layout.physics_mass = Some(elem_half(x0, y + ELEM_LABEL_GAP, 0, ELEM_FIELD_H));
        layout.physics_turn_mass = Some(elem_half(x0, y + ELEM_LABEL_GAP, 1, ELEM_FIELD_H));
        y += ELEM_LABEL_GAP + ELEM_FIELD_H + ELEM_ROW_GAP;
        layout.physics_air_resistance = Some(elem_half(x0, y + ELEM_LABEL_GAP, 0, ELEM_FIELD_H));
        layout.physics_elasticity = Some(elem_half(x0, y + ELEM_LABEL_GAP, 1, ELEM_FIELD_H));
        y += ELEM_LABEL_GAP + ELEM_FIELD_H + ELEM_ROW_GAP;
        layout.physics_buoyancy = Some(elem_full(x0, y + ELEM_LABEL_GAP, ELEM_FIELD_H));
        y += ELEM_LABEL_GAP + ELEM_FIELD_H + ELEM_ROW_GAP;
        layout.physics_center_of_mass = Some(elem_third(x0, y + ELEM_LABEL_GAP));
        y += ELEM_LABEL_GAP + ELEM_FIELD_H + ELEM_ROW_GAP;
        layout.physics_clear = Some(elem_full(x0, y, DFF_BTN_H));
        y += DFF_BTN_H + ELEM_ROW_GAP + 4.0;
    }

    // Section 8: resolved DFF materials/textures. This is collapsed by
    // default because some map assets contain a long material list.
    layout.headers[8] = elem_full(x0, y, DFF_SEC_HEADER_H);
    y += DFF_SEC_HEADER_H + 6.0;
    if !collapsed[8] {
        let entries = preview_material_entries(app, app.selected);
        if entries.is_empty() {
            y += 28.0;
        } else {
            let mut rows = Vec::with_capacity(entries.len());
            for _ in entries {
                rows.push(elem_full(x0, y, PREVIEW_TEXTURE_ROW_H - 4.0));
                y += PREVIEW_TEXTURE_ROW_H;
            }
            layout.texture_rows = Some(rows);
        }
        y += 4.0;
    }

    layout.content_height = (y + app.properties_scroll) - content.y + 10.0;
    layout
}

pub(crate) fn element_panel_max_scroll(layout: &ElementPanelLayout) -> f32 {
    (layout.content_height - layout.content.h).max(0.0)
}

pub(crate) struct SettingsPanelLayout {
    pub(crate) headers: [Rect; SETTINGS_SECTION_COUNT],
    pub(crate) snap_toggle: Option<Rect>,
    pub(crate) local_lod: Option<Rect>,
    pub(crate) snap_move: Option<Rect>,
    pub(crate) snap_rotate: Option<Rect>,
    pub(crate) camera_speed: Option<Rect>,
    pub(crate) pick_lods: Option<Rect>,
    pub(crate) camera_mode: Option<Rect>,
    pub(crate) box_mode: Option<Rect>,
    pub(crate) box_distance_minus: Option<Rect>,
    pub(crate) box_distance_plus: Option<Rect>,
    pub(crate) box_distance_label: Option<Rect>,
    pub(crate) eagle_offset: Option<[Rect; 3]>,
    pub(crate) eagle_water_offset: Option<[Rect; 3]>,
    pub(crate) global_offset: Option<[Rect; 3]>,
    pub(crate) global_rotation: Option<[Rect; 3]>,
    pub(crate) global_elements: Option<Rect>,
    pub(crate) global_water: Option<Rect>,
    pub(crate) global_preview: Option<Rect>,
    pub(crate) global_reset: Option<Rect>,
    pub(crate) global_apply: Option<Rect>,
    pub(crate) hint_y: f32,
}

pub(crate) fn settings_panel_layout(app: &AppState) -> SettingsPanelLayout {
    let content = properties_content_rect();
    let x0 = content.x + 6.0;
    let collapsed = app.settings_panel_collapsed;
    let mut layout = SettingsPanelLayout {
        headers: [ELEM_OFFSCREEN; SETTINGS_SECTION_COUNT],
        snap_toggle: None,
        local_lod: None,
        snap_move: None,
        snap_rotate: None,
        camera_speed: None,
        pick_lods: None,
        camera_mode: None,
        box_mode: None,
        box_distance_minus: None,
        box_distance_plus: None,
        box_distance_label: None,
        eagle_offset: None,
        eagle_water_offset: None,
        global_offset: None,
        global_rotation: None,
        global_elements: None,
        global_water: None,
        global_preview: None,
        global_reset: None,
        global_apply: None,
        hint_y: 0.0,
    };
    let mut y = content.y + 6.0;

    // Keep the project-wide operation at the top so it remains reachable on
    // shorter windows even when every settings section is expanded.
    layout.headers[3] = elem_full(x0, y, DFF_SEC_HEADER_H);
    y += DFF_SEC_HEADER_H + 6.0;
    if !collapsed[3] {
        layout.eagle_offset = Some(elem_third(x0, y + ELEM_LABEL_GAP));
        y += ELEM_LABEL_GAP + ELEM_FIELD_H + ELEM_ROW_GAP;
        layout.eagle_water_offset = Some(elem_third(x0, y + ELEM_LABEL_GAP));
        y += ELEM_LABEL_GAP + ELEM_FIELD_H + ELEM_ROW_GAP;
        layout.global_offset = Some(elem_third(x0, y + ELEM_LABEL_GAP));
        y += ELEM_LABEL_GAP + ELEM_FIELD_H + ELEM_ROW_GAP;
        layout.global_rotation = Some(elem_third(x0, y + ELEM_LABEL_GAP));
        y += ELEM_LABEL_GAP + ELEM_FIELD_H + ELEM_ROW_GAP;
        layout.global_elements = Some(elem_half(x0, y, 0, DFF_BTN_H));
        layout.global_water = Some(elem_half(x0, y, 1, DFF_BTN_H));
        y += DFF_BTN_H + ELEM_ROW_GAP;
        layout.global_preview = Some(elem_half(x0, y, 0, DFF_BTN_H));
        layout.global_reset = Some(elem_half(x0, y, 1, DFF_BTN_H));
        y += DFF_BTN_H + ELEM_ROW_GAP;
        layout.global_apply = Some(elem_full(x0, y, DFF_BTN_H));
        y += DFF_BTN_H + ELEM_ROW_GAP + 4.0;
    }

    layout.headers[0] = elem_full(x0, y, DFF_SEC_HEADER_H);
    y += DFF_SEC_HEADER_H + 6.0;
    if !collapsed[0] {
        layout.snap_toggle = Some(elem_half(x0, y, 0, DFF_BTN_H));
        layout.local_lod = Some(elem_half(x0, y, 1, DFF_BTN_H));
        y += DFF_BTN_H + ELEM_ROW_GAP;
        layout.snap_move = Some(elem_half(x0, y + ELEM_LABEL_GAP, 0, ELEM_FIELD_H));
        layout.snap_rotate = Some(elem_half(x0, y + ELEM_LABEL_GAP, 1, ELEM_FIELD_H));
        y += ELEM_LABEL_GAP + ELEM_FIELD_H + ELEM_ROW_GAP;
        y += 4.0;
    }

    layout.headers[1] = elem_full(x0, y, DFF_SEC_HEADER_H);
    y += DFF_SEC_HEADER_H + 6.0;
    if !collapsed[1] {
        layout.camera_speed = Some(elem_half(x0, y + ELEM_LABEL_GAP, 0, ELEM_FIELD_H));
        layout.pick_lods = Some(elem_half(x0, y + ELEM_LABEL_GAP, 1, ELEM_FIELD_H));
        y += ELEM_LABEL_GAP + ELEM_FIELD_H + ELEM_ROW_GAP;
        layout.camera_mode = Some(elem_full(x0, y, DFF_BTN_H));
        y += DFF_BTN_H + ELEM_ROW_GAP;
        y += 4.0;
    }

    layout.headers[2] = elem_full(x0, y, DFF_SEC_HEADER_H);
    y += DFF_SEC_HEADER_H + 6.0;
    if !collapsed[2] {
        let mode = elem_half(x0, y + ELEM_LABEL_GAP, 0, ELEM_FIELD_H);
        layout.box_mode = Some(mode);
        let plus = Rect::new(
            x0 + elem_full_w() - ELEM_FIELD_H,
            y + ELEM_LABEL_GAP,
            ELEM_FIELD_H,
            ELEM_FIELD_H,
        );
        let minus = Rect::new(
            plus.x - 8.0 - ELEM_FIELD_H,
            plus.y,
            ELEM_FIELD_H,
            ELEM_FIELD_H,
        );
        let distance = Rect::new(
            mode.x + mode.w + 18.0,
            mode.y,
            (minus.x - 8.0 - (mode.x + mode.w + 18.0)).max(64.0),
            ELEM_FIELD_H,
        );
        layout.box_distance_label = Some(distance);
        layout.box_distance_minus = Some(minus);
        layout.box_distance_plus = Some(plus);
        y += ELEM_LABEL_GAP + ELEM_FIELD_H + ELEM_ROW_GAP;
        y += 4.0;
    }

    layout.hint_y = y + 18.0;
    layout
}

fn element_layout_rect(app: &AppState, pick: impl Fn(&ElementPanelLayout) -> Option<Rect>) -> Rect {
    pick(&element_panel_layout(app)).unwrap_or(ELEM_OFFSCREEN)
}

pub(crate) fn element_type_button_rect(app: &AppState, slot: usize) -> Rect {
    element_layout_rect(app, |layout| {
        layout
            .type_buttons
            .and_then(|rects| rects.get(slot).copied())
    })
}

pub(crate) fn element_export_dff_rect(app: &AppState) -> Rect {
    element_layout_rect(app, |layout| layout.export_dff)
}

pub(crate) fn element_replace_dff_rect(app: &AppState) -> Rect {
    element_layout_rect(app, |layout| layout.replace_dff)
}

pub(crate) fn element_blender_position_rect(app: &AppState) -> Rect {
    element_layout_rect(app, |layout| layout.blender_position)
}

pub(crate) fn element_open_dff_editor_rect(app: &AppState) -> Rect {
    element_layout_rect(app, |layout| layout.dff_editor)
}

pub(crate) fn element_open_txd_rect(app: &AppState) -> Rect {
    element_layout_rect(app, |layout| layout.txd_editor)
}

pub(crate) fn element_find_missing_textures_rect(app: &AppState) -> Rect {
    element_layout_rect(app, |layout| layout.find_missing)
}

pub(crate) fn element_replace_col_rect(app: &AppState) -> Rect {
    element_layout_rect(app, |layout| layout.replace_col)
}

pub(crate) fn element_assign_lod_rect(app: &AppState) -> Rect {
    element_layout_rect(app, |layout| layout.assign_lod)
}

pub(crate) fn element_select_same_id_rect(app: &AppState) -> Rect {
    element_layout_rect(app, |layout| layout.select_same_id)
}

pub(crate) fn element_lod_parent_select_rect(app: &AppState) -> Rect {
    element_layout_rect(app, |layout| layout.lod_parent_select)
}

pub(crate) fn element_self_lod_rect(app: &AppState) -> Rect {
    element_layout_rect(app, |layout| layout.self_lod)
}

pub(crate) fn element_remove_instance_lods_rect(app: &AppState) -> Rect {
    element_layout_rect(app, |layout| layout.remove_instance_lods)
}

pub(crate) fn element_generate_lod_rect(app: &AppState) -> Rect {
    element_layout_rect(app, |layout| layout.generate_lod)
}

pub(crate) fn element_light_lod_rect(app: &AppState) -> Rect {
    element_layout_rect(app, |layout| layout.light_lod)
}

pub(crate) fn element_export_col_rect(app: &AppState) -> Rect {
    element_layout_rect(app, |layout| layout.export_col)
}

pub(crate) fn element_open_col_editor_rect(app: &AppState) -> Rect {
    element_layout_rect(app, |layout| layout.col_editor)
}

pub(crate) fn element_merge_vertex_lighting_rect(app: &AppState) -> Rect {
    element_layout_rect(app, |layout| layout.merge_vertex_lighting)
}

pub(crate) fn properties_snap_toggle_rect(app: &AppState) -> Rect {
    settings_panel_layout(app)
        .snap_toggle
        .unwrap_or(ELEM_OFFSCREEN)
}

pub(crate) fn properties_local_lod_toggle_rect(app: &AppState) -> Rect {
    settings_panel_layout(app)
        .local_lod
        .unwrap_or(ELEM_OFFSCREEN)
}

pub(crate) fn properties_lod_selectable_toggle_rect(app: &AppState) -> Rect {
    settings_panel_layout(app)
        .pick_lods
        .unwrap_or(ELEM_OFFSCREEN)
}

pub(crate) fn properties_camera_mode_toggle_rect(app: &AppState) -> Rect {
    settings_panel_layout(app)
        .camera_mode
        .unwrap_or(ELEM_OFFSCREEN)
}

pub(crate) fn properties_box_select_mode_rect(app: &AppState) -> Rect {
    settings_panel_layout(app)
        .box_mode
        .unwrap_or(ELEM_OFFSCREEN)
}

pub(crate) fn properties_box_select_minus_rect(app: &AppState) -> Rect {
    settings_panel_layout(app)
        .box_distance_minus
        .unwrap_or(ELEM_OFFSCREEN)
}

pub(crate) fn properties_box_select_plus_rect(app: &AppState) -> Rect {
    settings_panel_layout(app)
        .box_distance_plus
        .unwrap_or(ELEM_OFFSCREEN)
}

pub(crate) fn properties_global_preview_rect(app: &AppState) -> Rect {
    settings_panel_layout(app)
        .global_preview
        .unwrap_or(ELEM_OFFSCREEN)
}

pub(crate) fn properties_global_elements_rect(app: &AppState) -> Rect {
    settings_panel_layout(app)
        .global_elements
        .unwrap_or(ELEM_OFFSCREEN)
}

pub(crate) fn properties_global_water_rect(app: &AppState) -> Rect {
    settings_panel_layout(app)
        .global_water
        .unwrap_or(ELEM_OFFSCREEN)
}

pub(crate) fn properties_global_reset_rect(app: &AppState) -> Rect {
    settings_panel_layout(app)
        .global_reset
        .unwrap_or(ELEM_OFFSCREEN)
}

pub(crate) fn properties_global_apply_rect(app: &AppState) -> Rect {
    settings_panel_layout(app)
        .global_apply
        .unwrap_or(ELEM_OFFSCREEN)
}

pub(crate) fn definition_flag_rect(app: &AppState, slot: usize) -> Rect {
    element_layout_rect(app, |layout| {
        layout
            .flags
            .as_ref()
            .and_then(|flags| flags.get(slot).copied())
    })
}

pub(crate) fn placement_override_flag_rect(app: &AppState, slot: usize) -> Rect {
    element_layout_rect(app, |layout| {
        layout
            .override_flags
            .as_ref()
            .and_then(|flags| flags.get(slot).copied())
    })
}

pub(crate) fn physics_scope_rect(app: &AppState, scope: PhysicsScope) -> Rect {
    element_layout_rect(app, |layout| {
        layout
            .physics_scope
            .map(|rects| rects[usize::from(scope == PhysicsScope::PerObject)])
    })
}

pub(crate) fn physics_simulated_rect(app: &AppState) -> Rect {
    element_layout_rect(app, |layout| layout.physics_simulated)
}

pub(crate) fn physics_root_rect(app: &AppState) -> Rect {
    element_layout_rect(app, |layout| layout.physics_root)
}

pub(crate) fn physics_clear_rect(app: &AppState) -> Rect {
    element_layout_rect(app, |layout| layout.physics_clear)
}

pub(crate) fn physics_attrs(app: &AppState) -> Option<&BTreeMap<String, String>> {
    let placement = app.placements.get(app.selected)?;
    match app.physics_scope {
        PhysicsScope::Global => app.definitions.get(&placement.id).map(|def| &def.attrs),
        PhysicsScope::PerObject => Some(&placement.attrs),
    }
}

pub(crate) fn physics_attr_value(app: &AppState, key: &str) -> String {
    let Some(attrs) = physics_attrs(app) else {
        return String::new();
    };
    if let Some(value) = attrs.get(key) {
        return value.clone();
    }
    let Some(root) = physics_root_value_from_attrs(attrs) else {
        return String::new();
    };
    let Some(properties) = app
        .physics_root_properties
        .get(&root)
        .cloned()
        .or_else(|| physics_root_spec(root).map(|spec| spec.fallback.clone()))
    else {
        return String::new();
    };
    match key {
        "mass" => fmt_f32(properties.mass, 6),
        "turnMass" => fmt_f32(properties.turn_mass, 6),
        "airResistance" => fmt_f32(properties.air_resistance, 6),
        "elasticity" => fmt_f32(properties.elasticity, 6),
        "buoyancy" => fmt_f32(properties.buoyancy, 6),
        _ => String::new(),
    }
}

pub(crate) fn physics_root_value_from_attrs(attrs: &BTreeMap<String, String>) -> Option<u16> {
    PHYSICS_ROOT_ATTR_KEYS.iter().find_map(|key| {
        attrs
            .get(*key)
            .and_then(|value| value.trim().parse::<u16>().ok())
    })
}

pub(crate) fn physics_root_value(app: &AppState) -> Option<u16> {
    physics_attrs(app).and_then(physics_root_value_from_attrs)
}

pub(crate) fn physics_root_spec(model_id: u16) -> Option<&'static PhysicsRootSpec> {
    PHYSICS_ROOT_SPECS
        .iter()
        .find(|spec| spec.model_id == model_id)
}

pub(crate) fn physics_root_label(model_id: Option<u16>) -> String {
    match model_id {
        None => "No Root (Manual Values)".to_string(),
        Some(model_id) => physics_root_spec(model_id)
            .map(|spec| format!("{} ({model_id})", spec.label))
            .unwrap_or_else(|| format!("Custom SA Model ({model_id})")),
    }
}

pub(crate) fn physics_root_display_label(app: &AppState, model_id: Option<u16>) -> String {
    if model_id.is_none() && app.physics_scope == PhysicsScope::PerObject {
        let inherited = app
            .placements
            .get(app.selected)
            .and_then(|placement| app.definitions.get(&placement.id))
            .and_then(|definition| physics_root_value_from_attrs(&definition.attrs));
        return inherited
            .map(|model_id| format!("Inherit: {}", physics_root_label(Some(model_id))))
            .unwrap_or_else(|| "Inherit Global Root".to_string());
    }
    let base = physics_root_label(model_id);
    let Some(model_id) = model_id else {
        return base;
    };
    let Some(properties) = app
        .physics_root_properties
        .get(&model_id)
        .or_else(|| physics_root_spec(model_id).map(|spec| &spec.fallback))
    else {
        return base;
    };
    let behavior = if properties.is_breakable() {
        match properties.special_collision_response {
            1 => "Breakable · Lamppost",
            4 => "Breakable · Fence",
            6 => "Breakable · Swinging door",
            7 => "Breakable · Locked door",
            _ => "Breakable",
        }
    } else {
        match properties.collision_damage_effect {
            20 => "Smash",
            21 => "Change then smash",
            1 => "Damage model",
            _ => "No fracture",
        }
    };
    format!("{base} · {behavior}")
}

pub(crate) fn physics_root_option_rect(app: &AppState, row: usize) -> Rect {
    let field = physics_root_rect(app);
    let content = properties_content_rect();
    let option_count = PHYSICS_ROOT_SPECS.len() + 1;
    let popup_height = option_count as f32 * 25.0;
    let below = field.y + field.h + 6.0;
    let above = field.y - 6.0 - popup_height;
    let popup_y = if below + popup_height <= content.y + content.h {
        below
    } else {
        above.max(content.y + 4.0)
    };
    Rect::new(field.x, popup_y + row as f32 * 25.0, field.w, 24.0)
}

pub(crate) fn physics_simulated_value(app: &AppState) -> Option<bool> {
    let value = physics_attrs(app)?.get("simulated")?.trim();
    if value.eq_ignore_ascii_case("true") || value == "1" {
        Some(true)
    } else if value.eq_ignore_ascii_case("false") || value == "0" {
        Some(false)
    } else {
        None
    }
}

pub(crate) fn physics_attr_count(app: &AppState) -> usize {
    physics_attrs(app)
        .map(|attrs| {
            usize::from(physics_root_value_from_attrs(attrs).is_some())
                + PHYSICS_ATTR_KEYS
                    .iter()
                    .filter(|key| **key != "physicsRoot")
                    .filter(|key| {
                        attrs
                            .get(**key)
                            .is_some_and(|value| !value.trim().is_empty())
                    })
                    .count()
        })
        .unwrap_or(0)
}

pub(crate) fn inspector_field_is_physics(field: InspectorField) -> bool {
    matches!(
        field,
        InspectorField::PhysicsSimulated
            | InspectorField::PhysicsMass
            | InspectorField::PhysicsTurnMass
            | InspectorField::PhysicsAirResistance
            | InspectorField::PhysicsElasticity
            | InspectorField::PhysicsBuoyancy
            | InspectorField::PhysicsCenterOfMassX
            | InspectorField::PhysicsCenterOfMassY
            | InspectorField::PhysicsCenterOfMassZ
    )
}

fn normalize_override_flag(token: &str) -> String {
    token.trim().to_ascii_lowercase().replace('-', "_")
}

fn placement_override_flag_aliases(flag: &str) -> &'static [&'static str] {
    match flag {
        "double_sided" => &["double_sided", "disable_backface_culling", "21"],
        "disable_collisions" => &[
            "disable_collisions",
            "collisions_disabled",
            "no_collisions",
            "50",
        ],
        "breakable" => &["breakable"],
        "unbreakable" => &["unbreakable"],
        "frozen" => &["frozen"],
        "no_stream" => &["no_stream"],
        _ => &[],
    }
}

fn placement_override_bool(placement: &Placement, keys: &[&str]) -> Option<bool> {
    keys.iter().find_map(|key| {
        placement.attrs.get(*key).and_then(|value| {
            match value.trim().to_ascii_lowercase().as_str() {
                "true" | "1" | "yes" | "on" | "enabled" => Some(true),
                "false" | "0" | "no" | "off" | "disabled" => Some(false),
                _ => None,
            }
        })
    })
}

fn placement_override_tokens(placement: &Placement) -> Vec<String> {
    let mut tokens = placement
        .attrs
        .get("flags")
        .map(|value| split_flags(value))
        .unwrap_or_default();
    if let Some(value) = ["overrideFlags", "overrideflags", "override_flags"]
        .iter()
        .find_map(|key| placement.attrs.get(*key))
    {
        tokens.extend(split_flags(value));
    }
    tokens
}

fn placement_override_has_token(placement: &Placement, flag: &str) -> bool {
    placement_override_tokens(placement).iter().any(|token| {
        let normalized = normalize_override_flag(token);
        placement_override_flag_aliases(flag)
            .iter()
            .any(|alias| normalized == *alias)
    })
}

fn placement_direct_override_keys(flag: &str) -> &'static [&'static str] {
    match flag {
        "double_sided" => &[
            "doubleSided",
            "double_sided",
            "doublesided",
            "double-sided",
            "disableBackfaceCulling",
            "disablebackfaceculling",
            "disable_backface_culling",
            "noBackfaceCulling",
            "nobackfaceculling",
            "no_backface_culling",
        ],
        "disable_collisions" => &[
            "collisions",
            "collision",
            "collisionsEnabled",
            "collisionsenabled",
            "collisions_enabled",
            "noCollisions",
            "nocollisions",
            "no_collisions",
            "collisionsDisabled",
            "collisionsdisabled",
            "collisions_disabled",
            "disableCollisions",
            "disablecollisions",
            "disable_collisions",
        ],
        "breakable" | "unbreakable" => &[
            "breakable",
            "objectBreakable",
            "objectbreakable",
            "object_breakable",
        ],
        "frozen" => &["frozen", "freeze"],
        "no_stream" => &[
            "streamable",
            "streamed",
            "no_stream",
            "noStream",
            "nostream",
        ],
        _ => &[],
    }
}

pub(crate) fn placement_override_flag_label(flag: &str) -> &str {
    match flag {
        "double_sided" => "Double Sided",
        "disable_collisions" => "No Collisions",
        "breakable" => "Breakable",
        "unbreakable" => "Unbreakable",
        "frozen" => "Frozen",
        "no_stream" => "No Stream",
        _ => flag,
    }
}

pub(crate) fn placement_override_flag_enabled(placement: &Placement, flag: &str) -> bool {
    match flag {
        "double_sided" => placement_override_bool(
            placement,
            &[
                "doubleSided",
                "double_sided",
                "doublesided",
                "double-sided",
                "disableBackfaceCulling",
                "disablebackfaceculling",
                "disable_backface_culling",
                "noBackfaceCulling",
                "nobackfaceculling",
                "no_backface_culling",
            ],
        )
        .unwrap_or_else(|| placement_override_has_token(placement, flag)),
        "disable_collisions" => {
            let collisions = placement_override_bool(
                placement,
                &[
                    "collisions",
                    "collision",
                    "collisionsEnabled",
                    "collisionsenabled",
                    "collisions_enabled",
                ],
            );
            if let Some(enabled) = collisions {
                !enabled
            } else if placement_override_bool(
                placement,
                &[
                    "noCollisions",
                    "nocollisions",
                    "no_collisions",
                    "collisionsDisabled",
                    "collisionsdisabled",
                    "collisions_disabled",
                    "disableCollisions",
                    "disablecollisions",
                    "disable_collisions",
                ],
            ) == Some(true)
            {
                true
            } else {
                placement_override_has_token(placement, flag)
            }
        }
        "breakable" | "unbreakable" => {
            if let Some(breakable) = placement_override_bool(
                placement,
                &[
                    "breakable",
                    "objectBreakable",
                    "objectbreakable",
                    "object_breakable",
                ],
            ) {
                breakable == (flag == "breakable")
            } else {
                placement_override_has_token(placement, flag)
            }
        }
        "frozen" => placement_override_bool(placement, &["frozen", "freeze"])
            .unwrap_or_else(|| placement_override_has_token(placement, flag)),
        "no_stream" => {
            if let Some(streamable) =
                placement_override_bool(placement, &["streamable", "streamed"])
            {
                !streamable
            } else if placement_override_bool(placement, &["no_stream", "noStream", "nostream"])
                == Some(true)
            {
                true
            } else {
                placement_override_has_token(placement, flag)
            }
        }
        _ => placement_override_has_token(placement, flag),
    }
}

pub(crate) fn set_placement_override_flag(placement: &mut Placement, flag: &str, enabled: bool) {
    let aliases = placement_override_flag_aliases(flag);
    let mut remove_aliases: Vec<&str> = aliases.to_vec();
    if flag == "breakable" {
        remove_aliases.extend_from_slice(placement_override_flag_aliases("unbreakable"));
    } else if flag == "unbreakable" {
        remove_aliases.extend_from_slice(placement_override_flag_aliases("breakable"));
    }
    let mut flags = placement
        .attrs
        .get("flags")
        .map(|value| split_flags(value))
        .unwrap_or_default();
    flags.retain(|token| {
        let normalized = normalize_override_flag(token);
        !remove_aliases.iter().any(|alias| normalized == *alias)
    });
    if flags.is_empty() {
        placement.attrs.remove("flags");
    } else {
        placement.attrs.insert("flags".to_string(), flags.join(","));
    }

    let mut override_flags = Vec::new();
    for key in ["overrideFlags", "overrideflags", "override_flags"] {
        if let Some(value) = placement.attrs.remove(key) {
            override_flags.extend(split_flags(&value));
        }
    }
    override_flags.retain(|token| {
        let normalized = normalize_override_flag(token);
        !remove_aliases.iter().any(|alias| normalized == *alias)
    });

    for key in placement_direct_override_keys(flag) {
        placement.attrs.remove(*key);
    }
    if enabled {
        override_flags.push(flag.to_string());
    }
    if !override_flags.is_empty() {
        placement
            .attrs
            .insert("overrideFlags".to_string(), override_flags.join(","));
    }
}

pub(crate) fn collision_edit_button_rect(app: &AppState) -> Rect {
    Rect::new(
        screen_width() - RIGHT_PANEL_W + 18.0,
        TOP_H + 148.0 - app.properties_scroll,
        140.0,
        30.0,
    )
}

pub(crate) fn collision_open_col_editor_rect(app: &AppState) -> Rect {
    Rect::new(
        screen_width() - RIGHT_PANEL_W + 176.0,
        TOP_H + 148.0 - app.properties_scroll,
        150.0,
        30.0,
    )
}

pub(crate) fn draw_checkbox(font: &Font, rect: Rect, label: &str, checked: bool) {
    let mouse: Vec2 = mouse_position().into();
    let hovered = rect.contains(mouse);
    if hovered {
        draw_rrect(
            rect.x - 4.0,
            rect.y - 2.0,
            rect.w + 8.0,
            rect.h + 4.0,
            7.0,
            ui_surface_hover(),
        );
    }
    let box_rect = Rect::new(rect.x, rect.y + 3.0, 16.0, 16.0);
    draw_rrect_bordered(
        box_rect.x,
        box_rect.y,
        box_rect.w,
        box_rect.h,
        4.0,
        1.0,
        if checked {
            ui_surface_active()
        } else {
            ui_input_bg()
        },
        if checked || hovered {
            ui_accent()
        } else {
            ui_border()
        },
    );
    if checked {
        draw_line(
            box_rect.x + 4.0,
            box_rect.y + 8.0,
            box_rect.x + 7.0,
            box_rect.y + 12.0,
            2.0,
            WHITE,
        );
        draw_line(
            box_rect.x + 7.0,
            box_rect.y + 12.0,
            box_rect.x + 13.0,
            box_rect.y + 5.0,
            2.0,
            WHITE,
        );
    }
    let visible = ellipsize_width(label, 16, rect.w - 26.0);
    ui_text(font, &visible, rect.x + 22.0, rect.y + 17.0, LIGHTGRAY);
}

pub(crate) fn draw_input_box(app: &AppState, field: InspectorField, label: &str) {
    let rect = inspector_field_rect(app, field);
    let copy_rect = inspector_copy_button_rect(app, InspectorCopyAction::Field(field));
    let has_copy_button = inspector_copy_actions(app)
        .into_iter()
        .any(|action| action == InspectorCopyAction::Field(field));
    let text_right_pad = if has_copy_button { 36.0 } else { 20.0 };
    let readonly = app.active_tab == AppTab::Preview
        && app.properties_tab == PropertiesTab::Element
        && selected_definition_is_readonly(app)
        && definition_field_is_readonly(field)
        && (!inspector_field_is_physics(field) || app.physics_scope == PhysicsScope::Global);
    let active = app
        .inspector_edit
        .as_ref()
        .is_some_and(|edit| edit.field == field)
        && !readonly;
    let (mut text, cursor, selection_anchor) = if active {
        app.inspector_edit
            .as_ref()
            .map(|edit| {
                (
                    edit.buffer.clone(),
                    clamp_char_boundary(&edit.buffer, edit.cursor),
                    edit.selection_anchor,
                )
            })
            .unwrap_or_default()
    } else {
        (inspector_field_value(app, field), 0, None)
    };
    let dff_placeholder = !active
        && field == InspectorField::DefinitionDff
        && text.trim().is_empty()
        && selected_definition(app).is_some();
    if dff_placeholder {
        if let Some(def) = selected_definition(app) {
            text = def.id.clone();
        }
    }
    ui_text_size(&app.ui_font, label, rect.x, rect.y - 7.0, 14, ui_dim());
    let bg = if active {
        Color::new(0.055, 0.090, 0.128, 1.0)
    } else if readonly {
        Color::new(0.026, 0.034, 0.046, 1.0)
    } else {
        ui_input_bg()
    };
    let border = if active {
        ui_accent()
    } else if readonly {
        Color::new(0.11, 0.13, 0.15, 1.0)
    } else {
        ui_border()
    };
    if active {
        draw_rrect(
            rect.x - 2.0,
            rect.y - 2.0,
            rect.w + 4.0,
            rect.h + 4.0,
            9.0,
            Color::new(0.62, 0.66, 0.72, 0.10),
        );
    }
    draw_rrect_bordered(rect.x, rect.y, rect.w, rect.h, 8.0, 1.0, bg, border);
    if active {
        let max_chars = ((rect.w - text_right_pad) / 8.5).max(1.0) as usize;
        let cursor = clamp_char_boundary(&text, cursor);
        let cursor_char = text[..cursor].chars().count();
        let total_chars = text.chars().count();
        let start_char = cursor_char.saturating_sub(max_chars.saturating_sub(1));
        let end_char = (start_char + max_chars).min(total_chars);
        let visible: String = text
            .chars()
            .skip(start_char)
            .take(end_char - start_char)
            .collect();
        let caret_prefix: String = visible
            .chars()
            .take(cursor_char.saturating_sub(start_char))
            .collect();
        let text_x = rect.x + 10.0;
        draw_visible_text_selection(
            &text,
            cursor,
            selection_anchor,
            start_char,
            &visible,
            text_x,
            rect,
        );
        ui_text(&app.ui_font, &visible, text_x, rect.y + 20.0, WHITE);
        if (get_time() * 2.0) as i32 % 2 == 0 {
            let caret_x = text_x + ui_text_width(&caret_prefix, 16).round();
            draw_line(
                caret_x,
                rect.y + 7.0,
                caret_x,
                rect.y + rect.h - 7.0,
                1.0,
                WHITE,
            );
        }
    } else {
        let visible = ellipsize_width(&text, 16, rect.w - text_right_pad);
        ui_text(
            &app.ui_font,
            &visible,
            rect.x + 10.0,
            rect.y + 20.0,
            if readonly || dff_placeholder {
                ui_muted()
            } else {
                WHITE
            },
        );
    }
    if has_copy_button {
        draw_field_copy_button(app, copy_rect);
    }
}

pub(crate) fn draw_lod_parent_box(app: &AppState) {
    let rect = inspector_field_rect(app, InspectorField::ElementLodParent);
    ui_text_size(
        &app.ui_font,
        "LOD Parent",
        rect.x,
        rect.y - 7.0,
        14,
        ui_dim(),
    );
    draw_rrect_bordered(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        7.0,
        1.0,
        ui_input_bg(),
        ui_border(),
    );
    let raw_parent = app
        .placements
        .get(app.selected)
        .and_then(|placement| placement.attrs.get("lodParent"))
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());
    let parent_idx = selected_lod_parent_index(app);
    let self_lod = raw_parent.is_some_and(|parent| parent.eq_ignore_ascii_case("self"));
    let (text, color) = if self_lod {
        ("(self)", WHITE)
    } else if let Some(idx) = parent_idx {
        (
            app.placements
                .get(idx)
                .map(|placement| placement.id.as_str())
                .unwrap_or("None"),
            WHITE,
        )
    } else if let Some(parent) = raw_parent {
        (parent, ORANGE)
    } else {
        ("None", ui_muted())
    };
    let button = element_lod_parent_select_rect(app);
    ui_text(
        &app.ui_font,
        &ellipsize_width(text, 16, rect.w - 46.0),
        rect.x + 10.0,
        rect.y + 20.0,
        color,
    );
    let hovered = button.contains(mouse_position().into());
    draw_rrect_bordered(
        button.x,
        button.y,
        button.w,
        button.h,
        5.0,
        1.0,
        if hovered {
            ui_surface_hover()
        } else {
            ui_surface()
        },
        if hovered { ui_accent() } else { ui_border() },
    );
    let icon_size = 13.0;
    draw_texture_ex(
        &app.icons.select,
        button.x + (button.w - icon_size) * 0.5,
        button.y + (button.h - icon_size) * 0.5,
        if parent_idx.is_some() && hovered {
            WHITE
        } else if parent_idx.is_some() {
            ui_dim()
        } else {
            Color::new(0.35, 0.39, 0.46, 1.0)
        },
        DrawTextureParams {
            dest_size: Some(vec2(icon_size, icon_size)),
            ..Default::default()
        },
    );
}

pub(crate) const GTA_SA_COL_MATERIALS: &[(u8, &str)] = &[
    (0, "Default"),
    (1, "Tarmac"),
    (2, "Tarmac (damaged)"),
    (3, "Tarmac (really damaged)"),
    (4, "Pavement"),
    (5, "Pavement (damaged)"),
    (6, "Gravel"),
    (7, "Concrete (damaged)"),
    (8, "Painted Ground"),
    (9, "Grass (short lush)"),
    (10, "Grass (medium lush)"),
    (11, "Grass (long lush)"),
    (12, "Grass (short dry)"),
    (13, "Grass (medium dry)"),
    (14, "Grass (long dry)"),
    (15, "Golf Grass (rough)"),
    (16, "Golf Grass (smooth)"),
    (17, "Steep Slidy Grass"),
    (18, "Steep Cliff"),
    (19, "Flower Bed"),
    (20, "Meadow"),
    (21, "Waste Ground"),
    (22, "Woodland Ground"),
    (23, "Vegetation"),
    (24, "Mud (wet)"),
    (25, "Mud (dry)"),
    (26, "Dirt"),
    (27, "Dirt Track"),
    (28, "Sand (deep)"),
    (29, "Sand (medium)"),
    (30, "Sand (compact)"),
    (31, "Sand (arid)"),
    (32, "Sand (more)"),
    (33, "Sand (beach)"),
    (34, "Concrete (beach)"),
    (35, "Rock (dry)"),
    (36, "Rock (wet)"),
    (37, "Rock (cliff)"),
    (38, "Water (riverbed)"),
    (39, "Water (shallow)"),
    (40, "Corn Field"),
    (41, "Hedge"),
    (42, "Wood (crates)"),
    (43, "Wood (solid)"),
    (44, "Wood (thin)"),
    (45, "Glass"),
    (46, "Glass Windows (large)"),
    (47, "Glass Windows (small)"),
    (48, "Empty1"),
    (49, "Empty2"),
    (50, "Garage Door"),
    (51, "Thick Metal Plate"),
    (52, "Scaffold Pole"),
    (53, "Lamp Post"),
    (54, "Metal Gate"),
    (55, "Metal Chain fence"),
    (56, "Girder"),
    (57, "Fire Hydrant"),
    (58, "Container"),
    (59, "News Vendor"),
    (60, "Wheelbase"),
    (61, "Cardboard Box"),
    (62, "Ped"),
    (63, "Car"),
    (64, "Car (panel)"),
    (65, "Car (moving component)"),
    (66, "Transparent Cloth"),
    (67, "Rubber"),
    (68, "Plastic"),
    (69, "Transparent Stone"),
    (70, "Wood (bench)"),
    (71, "Carpet"),
    (72, "Floorboard"),
    (73, "Stairs (wood)"),
    (74, "Sand"),
    (75, "Sand (dense)"),
    (76, "Sand (arid)"),
    (77, "Sand (compact)"),
    (78, "Sand (rocky)"),
    (79, "Sand (beach)"),
    (80, "Grass (short)"),
    (81, "Grass (meadow)"),
    (82, "Grass (dry)"),
    (83, "Woodland"),
    (84, "Wood Dense"),
    (85, "Roadside"),
    (86, "Roadside Des"),
    (87, "Flowerbed"),
    (88, "Waste Ground"),
    (89, "Concrete"),
    (90, "Office Desk"),
    (91, "711 Shelf 1"),
    (92, "711 Shelf 2"),
    (93, "711 Shelf 3"),
    (94, "Restuarant Table"),
    (95, "Bar Table"),
    (96, "Underwater (lush)"),
    (97, "Underwater (barren)"),
    (98, "Underwater (coral)"),
    (99, "Underwater (deep)"),
    (100, "Riverbed"),
    (101, "Rubble"),
    (102, "Bedroom Floor"),
    (103, "Kitchen Floor"),
    (104, "Livingroom Floor"),
    (105, "corridor Floor"),
    (106, "711 Floor"),
    (107, "Fast Food Floor"),
    (108, "Skanky Floor"),
    (109, "Mountain"),
    (110, "Marsh"),
    (111, "Bushy"),
    (112, "Bushy (mix)"),
    (113, "Bushy (dry)"),
    (114, "Bushy (mid)"),
    (115, "Grass (wee flowers)"),
    (116, "Grass (dry tall)"),
    (117, "Grass (lush tall)"),
    (118, "Grass (green mix)"),
    (119, "Grass (brown mix)"),
    (120, "Grass (low)"),
    (121, "Grass (rocky)"),
    (122, "Grass (small trees)"),
    (123, "Dirt (rocky)"),
    (124, "Dirt (weeds)"),
    (125, "Grass (weeds)"),
    (126, "River Edge"),
    (127, "Poolside"),
    (128, "Forest (stumps)"),
    (129, "Forest (sticks)"),
    (130, "Forest (leaves)"),
    (131, "Desert Rocks"),
    (132, "Forest (dry)"),
    (133, "Sparse Flowers"),
    (134, "Building Site"),
    (135, "Docklands"),
    (136, "Industrial"),
    (137, "Industrial Jetty"),
    (138, "Concrete (litter)"),
    (139, "Alley Rubbish"),
    (140, "Junkyard Piles"),
    (141, "Junkyard Ground"),
    (142, "Dump"),
    (143, "Cactus Dense"),
    (144, "Airport Ground"),
    (145, "Cornfield"),
    (146, "Grass (light)"),
    (147, "Grass (lighter)"),
    (148, "Grass (lighter 2)"),
    (149, "Grass (mid 1)"),
    (150, "Grass (mid 2)"),
    (151, "Grass (dark)"),
    (152, "Grass (dark 2)"),
    (153, "Grass (dirt mix)"),
    (154, "Riverbed (stone)"),
    (155, "Riverbed (shallow)"),
    (156, "Riverbed (weeds)"),
    (157, "Seaweed"),
    (158, "Door"),
    (159, "Plastic Barrier"),
    (160, "Park Grass"),
    (161, "Stairs (stone)"),
    (162, "Stairs (metal)"),
    (163, "Stairs (carpet)"),
    (164, "Floor (metal)"),
    (165, "Floor (concrete)"),
    (166, "Bin Bag"),
    (167, "Thin Metal Sheet"),
    (168, "Metal Barrel"),
    (169, "Plastic Cone"),
    (170, "Plastic Dumpster"),
    (171, "Metal Dumpster"),
    (172, "Wood Picket Fence"),
    (173, "Wood Slatted Fence"),
    (174, "Wood Ranch Fence"),
    (175, "Unbreakable Glass"),
    (176, "Hay Bale"),
    (177, "Gore"),
    (178, "Rail Track"),
];

pub(crate) fn col_material_name(id: u8) -> &'static str {
    GTA_SA_COL_MATERIALS
        .iter()
        .find_map(|(material_id, name)| (*material_id == id).then_some(*name))
        .unwrap_or("Unknown")
}

pub(crate) fn col_material_label(id: u8) -> String {
    format!("{id}: {}", col_material_name(id))
}

pub(crate) fn col_material_option_rect(app: &AppState, row: usize) -> Rect {
    let field = inspector_field_rect(app, InspectorField::CollisionFaceMaterial);
    Rect::new(
        field.x,
        field.y + field.h + 6.0 + row as f32 * 25.0,
        field.w,
        24.0,
    )
}

pub(crate) fn draw_col_material_dropdown(app: &AppState) {
    let rect = inspector_field_rect(app, InspectorField::CollisionFaceMaterial);
    let value = inspector_field_value(app, InspectorField::CollisionFaceMaterial)
        .parse::<u8>()
        .unwrap_or(0);
    ui_text(&app.ui_font, "Material", rect.x, rect.y - 7.0, ui_dim());
    let hovered = rect.contains(mouse_position().into());
    draw_rrect_bordered(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        7.0,
        1.0,
        Color::new(0.055, 0.064, 0.078, 1.0),
        if app.col_material_dropdown_open || hovered {
            ui_accent()
        } else {
            ui_border()
        },
    );
    ui_text(
        &app.ui_font,
        &ellipsize_width(&col_material_label(value), 16, rect.w - 54.0),
        rect.x + 30.0,
        rect.y + 20.0,
        WHITE,
    );
    let swatch = collision_material_color(value, 1.0);
    draw_rrect_bordered(
        rect.x + 8.0,
        rect.y + 8.0,
        14.0,
        14.0,
        3.0,
        1.0,
        Color::new(swatch[0], swatch[1], swatch[2], 1.0),
        ui_border(),
    );
    ui_text(
        &app.ui_font,
        if app.col_material_dropdown_open {
            "^"
        } else {
            "v"
        },
        rect.x + rect.w - 22.0,
        rect.y + 20.0,
        ui_dim(),
    );
}

// Number of option rows shown at once in the open material dropdown.
pub(crate) const COL_MATERIAL_DROPDOWN_VISIBLE: usize = 8;

// Draws the expanded material option list. Call this AFTER the surrounding panel
// buttons so the popup overlays them cleanly instead of being drawn underneath.
pub(crate) fn draw_col_material_dropdown_popup(app: &AppState) {
    if !app.col_material_dropdown_open {
        return;
    }
    let value = inspector_field_value(app, InspectorField::CollisionFaceMaterial)
        .parse::<u8>()
        .unwrap_or(0);
    let total = GTA_SA_COL_MATERIALS.len();
    let visible = COL_MATERIAL_DROPDOWN_VISIBLE.min(total);
    let start = app
        .col_material_dropdown_scroll
        .floor()
        .max(0.0)
        .min(total.saturating_sub(visible) as f32) as usize;

    // Opaque backing panel so underlying buttons don't bleed through.
    let first = col_material_option_rect(app, 0);
    let last = col_material_option_rect(app, visible.saturating_sub(1));
    let pad = 4.0;
    let panel_x = first.x - pad;
    let panel_y = first.y - pad;
    let panel_w = first.w + pad * 2.0;
    let panel_h = (last.y + last.h) - first.y + pad * 2.0;
    draw_rrect_bordered(
        panel_x,
        panel_y,
        panel_w,
        panel_h,
        7.0,
        1.0,
        Color::new(0.04, 0.047, 0.058, 1.0),
        ui_accent(),
    );

    for row in 0..visible {
        let Some((id, _)) = GTA_SA_COL_MATERIALS.get(start + row).copied() else {
            break;
        };
        let option = col_material_option_rect(app, row);
        let option_hovered = option.contains(mouse_position().into());
        let selected = id == value;
        draw_rrect_bordered(
            option.x,
            option.y,
            option.w,
            option.h,
            5.0,
            1.0,
            if selected {
                ui_surface_active()
            } else if option_hovered {
                ui_surface_hover()
            } else {
                ui_input_bg()
            },
            ui_border(),
        );
        let swatch = collision_material_color(id, 1.0);
        draw_rrect_bordered(
            option.x + 8.0,
            option.y + 5.0,
            14.0,
            14.0,
            3.0,
            1.0,
            Color::new(swatch[0], swatch[1], swatch[2], 1.0),
            ui_border(),
        );
        ui_text(
            &app.ui_font,
            &ellipsize_width(&col_material_label(id), 16, option.w - 38.0),
            option.x + 30.0,
            option.y + 18.0,
            if selected { ui_accent() } else { WHITE },
        );
    }

    // Scrollbar indicator on the right edge of the popup.
    if total > visible {
        let track_x = panel_x + panel_w - 5.0;
        let track_y = panel_y + 3.0;
        let track_h = panel_h - 6.0;
        draw_rrect(track_x, track_y, 3.0, track_h, 1.5, ui_border());
        let thumb_h = (track_h * visible as f32 / total as f32).max(14.0);
        let max_start = total.saturating_sub(visible) as f32;
        let frac = if max_start > 0.0 {
            start as f32 / max_start
        } else {
            0.0
        };
        let thumb_y = track_y + (track_h - thumb_h) * frac;
        draw_rrect(track_x, thumb_y, 3.0, thumb_h, 1.5, ui_accent());
    }
}

pub(crate) fn draw_field_copy_button(app: &AppState, rect: Rect) {
    let hovered = rect.contains(mouse_position().into());
    draw_rrect_bordered(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        5.0,
        1.0,
        if hovered {
            ui_surface_hover()
        } else {
            ui_surface()
        },
        if hovered { ui_accent() } else { ui_border() },
    );
    let icon_size = 14.0;
    draw_texture_ex(
        &app.icons.duplicate,
        rect.x + (rect.w - icon_size) * 0.5,
        rect.y + (rect.h - icon_size) * 0.5,
        if hovered { WHITE } else { ui_dim() },
        DrawTextureParams {
            dest_size: Some(vec2(icon_size, icon_size)),
            ..Default::default()
        },
    );
}

pub(crate) fn draw_inspector_copy_button(app: &AppState, action: InspectorCopyAction) {
    if matches!(action, InspectorCopyAction::Field(_)) {
        return;
    }
    let rect = inspector_copy_button_rect(app, action);
    let mouse: Vec2 = mouse_position().into();
    let hovered = rect.contains(mouse);
    let label = match action {
        InspectorCopyAction::Field(_) => "",
        InspectorCopyAction::ElementPosition | InspectorCopyAction::LightPosition => {
            "Copy Position"
        }
        InspectorCopyAction::ElementRotation | InspectorCopyAction::LightRotation => {
            "Copy Rotation"
        }
    };
    draw_rrect_bordered(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        6.0,
        1.0,
        if hovered {
            ui_surface_hover()
        } else {
            ui_surface()
        },
        if hovered { ui_accent() } else { ui_border() },
    );
    let text_w = ui_text_width(label, 14);
    ui_text_size(
        &app.ui_font,
        label,
        rect.x + (rect.w - text_w) * 0.5,
        rect.y + 17.0,
        14,
        WHITE,
    );
}

pub(crate) fn draw_physics_root_dropdown(app: &AppState) {
    let rect = physics_root_rect(app);
    let hovered = rect.contains(mouse_position().into());
    ui_text(&app.ui_font, "Physics Root", rect.x, rect.y - 7.0, ui_dim());
    draw_rrect_bordered(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        7.0,
        1.0,
        Color::new(0.055, 0.064, 0.078, 1.0),
        if app.physics_root_dropdown_open || hovered {
            ui_accent()
        } else {
            ui_border()
        },
    );
    ui_text(
        &app.ui_font,
        &ellipsize_width(
            &physics_root_display_label(app, physics_root_value(app)),
            15,
            rect.w - 42.0,
        ),
        rect.x + 10.0,
        rect.y + 20.0,
        WHITE,
    );
    ui_text(
        &app.ui_font,
        if app.physics_root_dropdown_open {
            "^"
        } else {
            "v"
        },
        rect.x + rect.w - 22.0,
        rect.y + 20.0,
        ui_dim(),
    );
}

fn draw_physics_dimensions(app: &AppState) {
    let Some(rects) = element_panel_layout(app).physics_dimensions else {
        return;
    };
    let dimensions = app
        .placements
        .get(app.selected)
        .and_then(|placement| element_mesh(app, placement))
        .map(|mesh| mesh.bounds.max - mesh.bounds.min)
        .filter(|size| size.is_finite());
    let values = dimensions
        .map(|size| {
            [
                fmt_f32(size.x.abs(), 3),
                fmt_f32(size.y.abs(), 3),
                fmt_f32(size.z.abs(), 3),
            ]
        })
        .unwrap_or_else(|| ["—".to_string(), "—".to_string(), "—".to_string()]);

    for ((rect, label), value) in rects
        .into_iter()
        .zip(["Dimension X", "Dimension Y", "Dimension Z"])
        .zip(values)
    {
        ui_text(&app.ui_font, label, rect.x, rect.y - 7.0, ui_dim());
        draw_rrect_bordered(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            7.0,
            1.0,
            Color::new(0.045, 0.050, 0.060, 1.0),
            Color::new(0.11, 0.13, 0.15, 1.0),
        );
        ui_text(
            &app.ui_font,
            &ellipsize_width(&value, 16, rect.w - 20.0),
            rect.x + 10.0,
            rect.y + 20.0,
            LIGHTGRAY,
        );
    }
}

pub(crate) fn draw_physics_root_dropdown_popup(app: &AppState) {
    if !app.physics_root_dropdown_open {
        return;
    }
    let selected = physics_root_value(app);
    let option_count = PHYSICS_ROOT_SPECS.len() + 1;
    let first = physics_root_option_rect(app, 0);
    let last = physics_root_option_rect(app, option_count - 1);
    let pad = 4.0;
    draw_rrect_bordered(
        first.x - pad,
        first.y - pad,
        first.w + pad * 2.0,
        (last.y + last.h) - first.y + pad * 2.0,
        7.0,
        1.0,
        Color::new(0.04, 0.047, 0.058, 1.0),
        ui_accent(),
    );

    for row in 0..option_count {
        let model_id = row
            .checked_sub(1)
            .map(|index| PHYSICS_ROOT_SPECS[index].model_id);
        let rect = physics_root_option_rect(app, row);
        let hovered = rect.contains(mouse_position().into());
        let is_selected = selected == model_id;
        draw_rrect_bordered(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            5.0,
            1.0,
            if is_selected {
                ui_surface_active()
            } else if hovered {
                ui_surface_hover()
            } else {
                ui_input_bg()
            },
            ui_border(),
        );
        ui_text_size(
            &app.ui_font,
            &ellipsize_width(
                &physics_root_display_label(app, model_id),
                14,
                rect.w - 18.0,
            ),
            rect.x + 9.0,
            rect.y + 17.0,
            14,
            WHITE,
        );
    }
}

pub(crate) fn draw_inspector(app: &AppState) {
    let x = screen_width() - RIGHT_PANEL_W + 12.0;
    let y = TOP_H + 12.0;
    let w = RIGHT_PANEL_W - 24.0;
    let rect = Rect::new(x, y, w, (screen_height() - STATUS_H - y - 12.0).max(480.0));
    draw_panel_rect(&app.ui_font, rect, Some("Properties"));
    begin_ui_clip(properties_content_rect());
    match app.properties_tab {
        PropertiesTab::Element => draw_element_properties(app, x, y),
        PropertiesTab::Settings => draw_properties_settings(app, x, y),
        PropertiesTab::History => draw_properties_history(app, x, y),
    }
    end_ui_clip();
    let right_x = screen_width() - RIGHT_PANEL_W;
    let panel_bottom = rect.y + rect.h;
    let status_top = screen_height() - STATUS_H;
    if panel_bottom < status_top {
        draw_rectangle(
            right_x,
            panel_bottom,
            RIGHT_PANEL_W,
            status_top - panel_bottom,
            ui_canvas_bg(),
        );
    }
    draw_rectangle(
        right_x,
        status_top,
        RIGHT_PANEL_W,
        STATUS_H,
        Color::new(0.025, 0.035, 0.049, 1.0),
    );
    draw_line(
        right_x,
        status_top,
        screen_width(),
        status_top,
        1.0,
        ui_border(),
    );
    draw_rectangle(
        rect.x + 1.0,
        rect.y + 1.0,
        rect.w - 2.0,
        88.0,
        ui_panel_bg(),
    );
    draw_rrect(rect.x + 14.0, rect.y + 11.0, 3.0, 17.0, 1.5, ui_accent());
    ui_text_bold("Properties", rect.x + 25.0, rect.y + 26.0, 16, WHITE);
    draw_line(
        rect.x + 14.0,
        rect.y + 36.0,
        rect.x + rect.w - 14.0,
        rect.y + 36.0,
        1.0,
        ui_border(),
    );
    for (slot, tab) in properties_tabs().into_iter().enumerate() {
        text_button(
            &app.ui_font,
            properties_tab_rect(slot),
            properties_tab_label(tab),
            app.properties_tab == tab,
        );
    }
}

pub(crate) fn draw_element_properties(app: &AppState, x: f32, _y: f32) {
    let layout = element_panel_layout(app);
    let info_y = layout.info_top;
    let x0 = layout.content.x + 6.0;
    let content_w = RIGHT_PANEL_W - 52.0;
    if let Some(p) = app.placements.get(app.selected) {
        ui_text(
            &app.ui_font,
            "Selected Element",
            x0,
            info_y + 14.0,
            ui_dim(),
        );
        ui_text_size(
            &app.ui_font,
            &ellipsize(&p.id, 28),
            x0,
            info_y + 40.0,
            18,
            WHITE,
        );
        if selected_definition_is_gta_sa(app) {
            let id_w = ui_text_width(&ellipsize(&p.id, 28), 18);
            let tag_x = (x0 + 8.0 + id_w).min(x + content_w - 38.0);
            let tag_rect = Rect::new(tag_x, info_y + 22.0, 36.0, 22.0);
            draw_rrect_bordered(
                tag_rect.x,
                tag_rect.y,
                tag_rect.w,
                tag_rect.h,
                6.0,
                1.0,
                Color::new(0.15, 0.16, 0.18, 1.0),
                Color::new(0.34, 0.36, 0.40, 1.0),
            );
            ui_text_size(
                &app.ui_font,
                "[SA]",
                tag_rect.x + 5.0,
                tag_rect.y + 16.0,
                13,
                ui_accent(),
            );
        }
        let zone_line = format!("Zone {}", p.zone);
        ui_text(
            &app.ui_font,
            &format!(
                "{}   DFF {}",
                zone_line,
                ellipsize_width(&p.dff, 16, content_w - ui_text_width(&zone_line, 16) - 52.0)
            ),
            x0,
            info_y + 64.0,
            LIGHTGRAY,
        );
        ui_text(
            &app.ui_font,
            &format!("Position  {:.2}, {:.2}, {:.2}", p.pos.x, p.pos.y, p.pos.z),
            x0,
            info_y + 86.0,
            ui_muted(),
        );
        if let Some(mesh) = element_mesh(app, p) {
            let vertices = mesh.parts.iter().map(|part| part.vertices).sum::<usize>();
            let faces = mesh
                .parts
                .iter()
                .map(|part| part.face_indices.len())
                .sum::<usize>();
            ui_text(
                &app.ui_font,
                &format!("Geometry  {vertices} vertices   {faces} faces"),
                x0,
                info_y + 108.0,
                ui_accent(),
            );
        }
    } else {
        ui_text(
            &app.ui_font,
            "No element selected",
            x0,
            info_y + 14.0,
            ui_dim(),
        );
    }
    // Collapsible section headers.
    let placement = app.placements.get(app.selected);
    let definition = selected_definition(app);
    let flag_count = definition
        .map(|def| {
            EAGLE_DEFINITION_FLAGS
                .iter()
                .filter(|flag| definition_flag_enabled(def, flag))
                .count()
        })
        .unwrap_or(0);
    let override_count = placement
        .map(|p| {
            EAGLE_PLACEMENT_OVERRIDE_FLAGS
                .iter()
                .filter(|flag| placement_override_flag_enabled(p, flag))
                .count()
                + usize::from(
                    p.attrs
                        .get("alpha")
                        .is_some_and(|value| !value.trim().is_empty()),
                )
                + usize::from(
                    p.attrs
                        .get("scale")
                        .is_some_and(|value| !value.trim().is_empty()),
                )
        })
        .unwrap_or(0);
    let hints: [String; ELEM_SECTION_COUNT] = [
        placement.map(|p| p.tag.clone()).unwrap_or_default(),
        placement
            .and_then(|p| p.attrs.get("lodParent"))
            .cloned()
            .unwrap_or_default(),
        placement
            .map(|p| format!("{:.1}, {:.1}, {:.1}", p.pos.x, p.pos.y, p.pos.z))
            .unwrap_or_default(),
        placement.map(|p| ellipsize(&p.dff, 16)).unwrap_or_default(),
        ellipsize(
            &inspector_field_value(app, InspectorField::DefinitionCol),
            16,
        ),
        if flag_count > 0 {
            format!("{flag_count} set")
        } else {
            String::new()
        },
        if override_count > 0 {
            format!("{override_count} set")
        } else {
            String::new()
        },
        {
            let count = physics_attr_count(app);
            if count > 0 {
                format!("{} | {count} set", app.physics_scope.label())
            } else {
                app.physics_scope.label().to_string()
            }
        },
        {
            let count = preview_material_entries(app, app.selected).len();
            if count == 0 {
                String::new()
            } else {
                format!("{count} material{}", if count == 1 { "" } else { "s" })
            }
        },
    ];
    for (idx, title) in ELEM_SECTION_TITLES.iter().enumerate() {
        dff_section_header_button(
            &app.ui_font,
            layout.headers[idx],
            !app.element_panel_collapsed[idx],
            title,
            &hints[idx],
        );
    }
    let texture_entries = preview_material_entries(app, app.selected);
    if let Some(rows) = layout.texture_rows.as_ref() {
        let mouse: Vec2 = mouse_position().into();
        let mut thumbnails = Vec::new();
        for (entry, rect) in texture_entries.iter().zip(rows) {
            if rect.y + rect.h < layout.content.y || rect.y > layout.content.y + layout.content.h {
                continue;
            }
            let selected =
                app.preview_selected_material == Some((app.selected, entry.material_index));
            if selected || rect.contains(mouse) {
                draw_rrect(
                    rect.x,
                    rect.y,
                    rect.w,
                    rect.h,
                    5.0,
                    if selected {
                        ui_surface_active()
                    } else {
                        ui_surface_hover()
                    },
                );
            }
            let thumbnail_rect = preview_texture_thumbnail_rect(*rect);
            draw_rrect_bordered(
                thumbnail_rect.x,
                thumbnail_rect.y,
                thumbnail_rect.w,
                thumbnail_rect.h,
                4.0,
                1.0,
                Color::new(0.035, 0.040, 0.050, 1.0),
                if selected { YELLOW } else { ui_border() },
            );
            if entry.texture_id != 0 {
                let available = thumbnail_rect.w - 4.0;
                let source_width = entry.texture_width.max(1) as f32;
                let source_height = entry.texture_height.max(1) as f32;
                let scale = (available / source_width).min(available / source_height);
                let width = (source_width * scale).max(1.0);
                let height = (source_height * scale).max(1.0);
                thumbnails.push((
                    entry.texture_id,
                    Rect::new(
                        thumbnail_rect.x + (thumbnail_rect.w - width) * 0.5,
                        thumbnail_rect.y + (thumbnail_rect.h - height) * 0.5,
                        width,
                        height,
                    ),
                ));
            } else {
                ui_text_size(
                    &app.ui_font,
                    "N/A",
                    thumbnail_rect.x + 10.0,
                    thumbnail_rect.y + 27.0,
                    12,
                    if entry.missing { RED } else { ui_muted() },
                );
            }
            let texture_label = if entry.texture_name.trim().is_empty() {
                "<empty material>"
            } else {
                entry.texture_name.as_str()
            };
            ui_text(
                &app.ui_font,
                &ellipsize_width(texture_label, 16, rect.w - 126.0),
                rect.x + 57.0,
                rect.y + 21.0,
                if selected { ui_accent() } else { WHITE },
            );
            let detail = if entry.missing {
                format!("Material #{}  ·  MISSING", entry.material_index)
            } else {
                format!(
                    "Material #{}  ·  {} face{}",
                    entry.material_index,
                    entry.face_count,
                    if entry.face_count == 1 { "" } else { "s" }
                )
            };
            ui_text_size(
                &app.ui_font,
                &ellipsize_width(&detail, 14, rect.w - 126.0),
                rect.x + 57.0,
                rect.y + 42.0,
                14,
                if entry.missing { RED } else { ui_muted() },
            );
            if entry.texture_id != 0 {
                text_button(
                    &app.ui_font,
                    preview_texture_view_rect(*rect),
                    "View",
                    false,
                );
            }
        }
        draw_raw_texture_quads(&thumbnails);
    } else if !app.element_panel_collapsed[8] && texture_entries.is_empty() {
        let header = layout.headers[8];
        ui_text_size(
            &app.ui_font,
            "No resolved texture materials",
            header.x + 10.0,
            header.y + DFF_SEC_HEADER_H + 20.0,
            14,
            ui_muted(),
        );
    }
    if let (Some(buttons), Some(_)) = (layout.type_buttons, placement) {
        let selected = selected_live_indices(app);
        for (slot, element_type) in EAGLE_ELEMENT_TYPES.iter().enumerate() {
            text_button(
                &app.ui_font,
                buttons[slot],
                match *element_type {
                    "building" => "Building",
                    "object" => "Object",
                    "scenery" => "Scenery",
                    _ => element_type,
                },
                !selected.is_empty()
                    && selected.iter().all(|idx| {
                        app.placements
                            .get(*idx)
                            .is_some_and(|placement| placement.tag == *element_type)
                    }),
            );
        }
    }
    draw_inspector_copy_button(app, InspectorCopyAction::ElementPosition);
    draw_inspector_copy_button(app, InspectorCopyAction::ElementRotation);
    draw_input_box(app, InspectorField::ElementId, "ID");
    draw_input_box(app, InspectorField::ElementPosX, "X");
    draw_input_box(app, InspectorField::ElementPosY, "Y");
    draw_input_box(app, InspectorField::ElementPosZ, "Z");
    draw_input_box(app, InspectorField::ElementRotX, "XR");
    draw_input_box(app, InspectorField::ElementRotY, "YR");
    draw_input_box(app, InspectorField::ElementRotZ, "ZR");
    draw_input_box(app, InspectorField::ElementDimension, "Dimension");
    draw_input_box(app, InspectorField::ElementInterior, "Interior");
    draw_input_box(app, InspectorField::ElementAlpha, "Alpha");
    draw_input_box(app, InspectorField::ElementScale, "Scale");
    for scope in [PhysicsScope::Global, PhysicsScope::PerObject] {
        text_button(
            &app.ui_font,
            physics_scope_rect(app, scope),
            scope.label(),
            app.physics_scope == scope,
        );
    }
    draw_physics_root_dropdown(app);
    draw_physics_dimensions(app);
    let simulation_label = match physics_simulated_value(app) {
        Some(true) => "Simulation: On",
        Some(false) => "Simulation: Off",
        None if app.physics_scope == PhysicsScope::PerObject => "Simulation: Inherit",
        None => "Simulation: Unset",
    };
    text_button(
        &app.ui_font,
        physics_simulated_rect(app),
        simulation_label,
        physics_simulated_value(app).is_some(),
    );
    draw_input_box(app, InspectorField::PhysicsMass, "Mass");
    draw_input_box(app, InspectorField::PhysicsTurnMass, "Turn Mass");
    draw_input_box(
        app,
        InspectorField::PhysicsAirResistance,
        "Damping / Air Resistance",
    );
    draw_input_box(app, InspectorField::PhysicsElasticity, "Elasticity");
    draw_input_box(app, InspectorField::PhysicsBuoyancy, "Buoyancy");
    draw_input_box(app, InspectorField::PhysicsCenterOfMassX, "Center X");
    draw_input_box(app, InspectorField::PhysicsCenterOfMassY, "Center Y");
    draw_input_box(app, InspectorField::PhysicsCenterOfMassZ, "Center Z");
    text_button(
        &app.ui_font,
        physics_clear_rect(app),
        "Clear Physics Values",
        false,
    );
    draw_lod_parent_box(app);
    draw_input_box(app, InspectorField::DefinitionLod, "Draw Distance");
    draw_input_box(app, InspectorField::ElementUniqueId, "Unique ID");
    text_button(
        &app.ui_font,
        element_self_lod_rect(app),
        "Self LOD",
        selected_live_elements_are_self_lod(app),
    );
    let has_live_element = selected_placement(app).is_some()
        && !app
            .element_states
            .get(app.selected)
            .is_some_and(|state| state.deleted);
    if app.instance_lod_removal_job.is_some() {
        text_button_busy(
            &app.ui_font,
            element_remove_instance_lods_rect(app),
            "Removing Instance LODs",
        );
    } else if has_live_element {
        text_button(
            &app.ui_font,
            element_remove_instance_lods_rect(app),
            "Remove LODs From All Instances",
            false,
        );
    } else {
        text_button_disabled(
            &app.ui_font,
            element_remove_instance_lods_rect(app),
            "Remove LODs From All Instances",
            "Select a live element first.",
        );
    }
    text_button(
        &app.ui_font,
        element_select_same_id_rect(app),
        "Select Same ID",
        false,
    );
    if selected_live_indices_in_selection_order(app).len() >= 2 {
        text_button(
            &app.ui_font,
            element_assign_lod_rect(app),
            "Assign LOD",
            false,
        );
    } else {
        text_button_disabled(
            &app.ui_font,
            element_assign_lod_rect(app),
            "Assign LOD",
            "Select a detail element and its LOD element.",
        );
    }
    if app.lod_generation_job.is_some() {
        text_button_busy(
            &app.ui_font,
            element_generate_lod_rect(app),
            "Generating LOD",
        );
    } else if has_live_element {
        text_button(
            &app.ui_font,
            element_generate_lod_rect(app),
            if selected_models_all_have_lods(app) {
                "Regenerate LODs"
            } else {
                "Generate LOD"
            },
            false,
        );
    } else {
        text_button_disabled(
            &app.ui_font,
            element_generate_lod_rect(app),
            "Generate LOD",
            "Select a live element first.",
        );
    }
    if app.light_lod_job.is_some() {
        text_button_busy(&app.ui_font, element_light_lod_rect(app), "Lighting LOD");
    } else if has_live_element {
        text_button(
            &app.ui_font,
            element_light_lod_rect(app),
            "Light LOD",
            false,
        );
    } else {
        text_button_disabled(
            &app.ui_font,
            element_light_lod_rect(app),
            "Light LOD",
            "Select a live element first.",
        );
    }
    draw_input_box(app, InspectorField::DefinitionDff, "DFF override");
    draw_input_box(
        app,
        InspectorField::DefinitionNativeModel,
        "Native Behavior Model",
    );
    draw_input_box(app, InspectorField::DefinitionTxd, "TXD");
    let open_txd_enabled = selected_definition_id_and_txd(app).is_some();
    let find_missing_textures_enabled = selected_definition_has_missing_textures(app);
    if has_live_element {
        text_button(
            &app.ui_font,
            element_open_dff_editor_rect(app),
            "DFF Editor",
            false,
        );
    } else {
        text_button_disabled(
            &app.ui_font,
            element_open_dff_editor_rect(app),
            "DFF Editor",
            "Select a live element first.",
        );
    }
    if open_txd_enabled {
        text_button(
            &app.ui_font,
            element_open_txd_rect(app),
            "TXD Editor",
            false,
        );
    } else {
        text_button_disabled(
            &app.ui_font,
            element_open_txd_rect(app),
            "TXD Editor",
            "The selected definition has no TXD.",
        );
    }
    if find_missing_textures_enabled {
        text_button(
            &app.ui_font,
            element_find_missing_textures_rect(app),
            "Find Missing Textures",
            false,
        );
    }
    if let Some(note_y) = layout.readonly_note_y {
        ui_text(
            &app.ui_font,
            "Definition from GTA:SA install (read-only)",
            x0,
            note_y,
            ui_muted(),
        );
    }
    let dff_actions_enabled = has_live_element;
    let replace_dff_enabled = dff_actions_enabled && !selected_definition_is_readonly(app);
    let replace_col_enabled = replace_dff_enabled;
    let blender_position_enabled = selected_live_indices(app).len() >= 2;
    if dff_actions_enabled {
        text_button(
            &app.ui_font,
            element_export_dff_rect(app),
            "Export DFF",
            false,
        );
    } else {
        text_button_disabled(
            &app.ui_font,
            element_export_dff_rect(app),
            "Export DFF",
            "Select a live element first.",
        );
    }
    if replace_dff_enabled {
        text_button(
            &app.ui_font,
            element_replace_dff_rect(app),
            "Replace DFF",
            false,
        );
    } else {
        text_button_disabled(
            &app.ui_font,
            element_replace_dff_rect(app),
            "Replace DFF",
            if dff_actions_enabled {
                "GTA:SA fallback definitions are read-only."
            } else {
                "Select a live element first."
            },
        );
    }
    if blender_position_enabled {
        text_button(
            &app.ui_font,
            element_blender_position_rect(app),
            "Blender Position",
            false,
        );
    } else {
        text_button_disabled(
            &app.ui_font,
            element_blender_position_rect(app),
            "Blender Position",
            "Select at least two live elements.",
        );
    }
    draw_input_box(app, InspectorField::DefinitionCol, "COL");
    if dff_actions_enabled {
        text_button(
            &app.ui_font,
            element_export_col_rect(app),
            "Export COL",
            false,
        );
        text_button(
            &app.ui_font,
            element_open_col_editor_rect(app),
            "COL Editor",
            false,
        );
    } else {
        text_button_disabled(
            &app.ui_font,
            element_export_col_rect(app),
            "Export COL",
            "Select a live element first.",
        );
        text_button_disabled(
            &app.ui_font,
            element_open_col_editor_rect(app),
            "COL Editor",
            "Select a live element first.",
        );
    }
    if replace_col_enabled {
        text_button(
            &app.ui_font,
            element_replace_col_rect(app),
            "Replace COL",
            false,
        );
    } else {
        text_button_disabled(
            &app.ui_font,
            element_replace_col_rect(app),
            "Replace COL",
            if dff_actions_enabled {
                "GTA:SA fallback definitions are read-only."
            } else {
                "Select a live element first."
            },
        );
    }
    if layout.merge_vertex_lighting.is_some() {
        if app.day_night_merge_job.is_some() {
            text_button_busy(
                &app.ui_font,
                element_merge_vertex_lighting_rect(app),
                "Merging Vertex Lighting",
            );
        } else {
            text_button(
                &app.ui_font,
                element_merge_vertex_lighting_rect(app),
                if selected_day_night_merge_override_ready(app) {
                    "Override Merge Warning"
                } else {
                    "Merge Vertex Lighting"
                },
                false,
            );
        }
    }
    draw_input_box(app, InspectorField::DefinitionTimeIn, "Time In");
    draw_input_box(app, InspectorField::DefinitionTimeOut, "Time Out");
    let selected_definition_ids = selected_definition_ids(app);
    if !selected_definition_ids.is_empty() {
        for (slot, flag) in EAGLE_DEFINITION_FLAGS.iter().enumerate() {
            draw_checkbox(
                &app.ui_font,
                definition_flag_rect(app, slot),
                flag,
                selected_definition_ids.iter().all(|id| {
                    app.definitions
                        .get(id)
                        .is_some_and(|def| definition_flag_enabled(def, flag))
                }),
            );
        }
    }
    let selected = selected_live_indices(app);
    if !selected.is_empty() {
        for (slot, flag) in EAGLE_PLACEMENT_OVERRIDE_FLAGS.iter().enumerate() {
            draw_checkbox(
                &app.ui_font,
                placement_override_flag_rect(app, slot),
                placement_override_flag_label(flag),
                selected.iter().all(|idx| {
                    app.placements
                        .get(*idx)
                        .is_some_and(|p| placement_override_flag_enabled(p, flag))
                }),
            );
        }
    }
    let max_scroll = element_panel_max_scroll(&layout);
    if max_scroll > 0.0 {
        let content = layout.content;
        let track = Rect::new(
            content.x + content.w - 5.0,
            content.y + 4.0,
            3.0,
            content.h - 8.0,
        );
        draw_rectangle(
            track.x,
            track.y,
            track.w,
            track.h,
            Color::new(1.0, 1.0, 1.0, 0.06),
        );
        let thumb_h = (track.h * content.h / layout.content_height).max(24.0);
        let frac = (app.properties_scroll / max_scroll).clamp(0.0, 1.0);
        let thumb_y = track.y + frac * (track.h - thumb_h);
        draw_rectangle(
            track.x,
            thumb_y,
            track.w,
            thumb_h,
            Color::new(1.0, 1.0, 1.0, 0.25),
        );
    }
    draw_physics_root_dropdown_popup(app);
}

pub(crate) fn draw_properties_settings(app: &AppState, _x: f32, _y: f32) {
    let layout = settings_panel_layout(app);
    let hints = [
        format!("snap {:.1} / {:.0} deg", app.snap_move, app.snap_rotate),
        if app.camera_mode == CameraMode::Focus {
            "Focus Cam".to_string()
        } else {
            "Free Cam".to_string()
        },
        format!(
            "{} / {:.0}",
            app.box_select_mode.label(),
            app.box_select_distance
        ),
        if app.global_transform.preview {
            "Previewing | bounds +/-3000".to_string()
        } else {
            "Not applied".to_string()
        },
    ];
    for (idx, title) in SETTINGS_SECTION_TITLES.iter().enumerate() {
        dff_section_header_button(
            &app.ui_font,
            layout.headers[idx],
            !app.settings_panel_collapsed[idx],
            title,
            &hints[idx],
        );
    }
    if !app.settings_panel_collapsed[0] {
        text_button(
            &app.ui_font,
            properties_snap_toggle_rect(app),
            "Snap Mode",
            app.snap_enabled,
        );
        text_button(
            &app.ui_font,
            properties_local_lod_toggle_rect(app),
            "Local LOD",
            app.show_selected_lod_local,
        );
        draw_input_box(app, InspectorField::SnapMove, "Move Snap");
        draw_input_box(app, InspectorField::SnapRotate, "Rotate Snap");
    }
    if !app.settings_panel_collapsed[1] {
        draw_input_box(app, InspectorField::CameraSpeed, "Camera Speed");
        text_button(
            &app.ui_font,
            properties_lod_selectable_toggle_rect(app),
            "Pick LODs",
            app.lod_selectable,
        );
        text_button(
            &app.ui_font,
            properties_camera_mode_toggle_rect(app),
            if app.camera_mode == CameraMode::Focus {
                "Focus Cam"
            } else {
                "Free Cam"
            },
            app.camera_mode == CameraMode::Focus,
        );
    }
    if !app.settings_panel_collapsed[2] {
        if let Some(rect) = layout.box_mode {
            ui_text(&app.ui_font, "Mode", rect.x, rect.y - 7.0, ui_dim());
            text_button(
                &app.ui_font,
                rect,
                app.box_select_mode.label(),
                app.box_select_mode != BoxSelectMode::Add,
            );
        }
        if let Some(rect) = layout.box_distance_label {
            ui_text(&app.ui_font, "Range", rect.x, rect.y - 7.0, ui_dim());
            draw_rectangle(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                Color::new(0.045, 0.052, 0.064, 1.0),
            );
            draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, ui_border());
            ui_text(
                &app.ui_font,
                &format!("{:.0}", app.box_select_distance),
                rect.x + 12.0,
                rect.y + 20.0,
                WHITE,
            );
        }
        text_button(
            &app.ui_font,
            properties_box_select_minus_rect(app),
            "-",
            false,
        );
        text_button(
            &app.ui_font,
            properties_box_select_plus_rect(app),
            "+",
            false,
        );
    }
    if !app.settings_panel_collapsed[3] {
        draw_input_box(app, InspectorField::EagleOffsetX, "#offset X");
        draw_input_box(app, InspectorField::EagleOffsetY, "#offset Y");
        draw_input_box(app, InspectorField::EagleOffsetZ, "#offset Z");
        draw_input_box(app, InspectorField::EagleWaterOffsetX, "#waterOffset X");
        draw_input_box(app, InspectorField::EagleWaterOffsetY, "#waterOffset Y");
        draw_input_box(app, InspectorField::EagleWaterOffsetZ, "#waterOffset Z");
        draw_input_box(app, InspectorField::GlobalOffsetX, "Offset X");
        draw_input_box(app, InspectorField::GlobalOffsetY, "Offset Y");
        draw_input_box(app, InspectorField::GlobalOffsetZ, "Offset Z");
        draw_input_box(app, InspectorField::GlobalRotationX, "Rotate X");
        draw_input_box(app, InspectorField::GlobalRotationY, "Rotate Y");
        draw_input_box(app, InspectorField::GlobalRotationZ, "Rotate Z");
        text_button(
            &app.ui_font,
            properties_global_elements_rect(app),
            "Elements",
            app.global_transform.transform_elements,
        );
        text_button(
            &app.ui_font,
            properties_global_water_rect(app),
            "Water",
            app.global_transform.transform_water,
        );
        text_button(
            &app.ui_font,
            properties_global_preview_rect(app),
            "Preview",
            app.global_transform.preview,
        );
        text_button(
            &app.ui_font,
            properties_global_reset_rect(app),
            "Reset",
            false,
        );
        text_button(
            &app.ui_font,
            properties_global_apply_rect(app),
            "Apply Global Transform",
            false,
        );
    }
    ui_text(
        &app.ui_font,
        "Enter applies   Esc cancels",
        layout.headers[0].x,
        layout.hint_y,
        ui_muted(),
    );
}

pub(crate) fn draw_properties_history(app: &AppState, x: f32, y: f32) {
    ui_text(&app.ui_font, "Undo Stack", x + 14.0, y + 116.0, ui_dim());
    for (row, entry) in app.undo_stack.iter().rev().take(12).enumerate() {
        let label = ellipsize(&entry.label, 28);
        ui_text(
            &app.ui_font,
            &label,
            x + 24.0,
            y + 142.0 + row as f32 * 23.0,
            if row == 0 { ui_accent() } else { LIGHTGRAY },
        );
    }
    ui_text(&app.ui_font, "Redo", x + 14.0, y + 444.0, ui_dim());
    for (row, entry) in app.redo_stack.iter().rev().take(6).enumerate() {
        let label = ellipsize(&entry.label, 28);
        ui_text(
            &app.ui_font,
            &label,
            x + 24.0,
            y + 470.0 + row as f32 * 23.0,
            LIGHTGRAY,
        );
    }
}

pub(crate) fn draw_collision_panel(app: &AppState) {
    let x = screen_width() - RIGHT_PANEL_W + 12.0;
    let panel_y = TOP_H + 12.0;
    let y = panel_y - app.properties_scroll;
    let w = RIGHT_PANEL_W - 24.0;
    draw_panel_rect(
        &app.ui_font,
        Rect::new(
            x,
            panel_y,
            w,
            (screen_height() - STATUS_H - panel_y - 12.0).max(1.0),
        ),
        Some("Collisions"),
    );
    begin_ui_clip(inspector_panel_content_rect());
    if let Some(p) = app.placements.get(app.selected) {
        ui_text(
            &app.ui_font,
            "Selected Element",
            x + 14.0,
            y + 56.0,
            ui_dim(),
        );
        ui_text_size(
            &app.ui_font,
            &ellipsize(&p.id, 28),
            x + 14.0,
            y + 80.0,
            18,
            WHITE,
        );
        let col_key = element_collision_key(app, p);
        if let Some(mesh) = element_collision_mesh(app, p) {
            ui_text(
                &app.ui_font,
                &format!("COL {}", ellipsize(&col_key, 28)),
                x + 14.0,
                y + 106.0,
                LIGHTGRAY,
            );
            ui_text(
                &app.ui_font,
                &format!(
                    "{} vertices   {} faces   {} primitives",
                    mesh.vertices.len(),
                    mesh.faces.len(),
                    mesh.spheres.len() + mesh.boxes.len()
                ),
                x + 14.0,
                y + 128.0,
                ui_accent(),
            );
        } else {
            ui_text(
                &app.ui_font,
                &format!("COL {} not parsed", ellipsize(&col_key, 24)),
                x + 14.0,
                y + 106.0,
                LIGHTGRAY,
            );
        }
    } else {
        ui_text(
            &app.ui_font,
            "No element selected",
            x + 14.0,
            y + 58.0,
            ui_dim(),
        );
    }
    text_button(
        &app.ui_font,
        collision_edit_button_rect(app),
        if app.collision_edit_mode {
            "Edit Faces"
        } else {
            "View Only"
        },
        app.collision_edit_mode,
    );
    text_button(
        &app.ui_font,
        collision_open_col_editor_rect(app),
        "COL Editor",
        false,
    );
    if selected_placement(app).is_none() {
        let rect = collision_open_col_editor_rect(app);
        draw_rectangle(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            Color::new(0.0, 0.0, 0.0, 0.36),
        );
    }
    ui_text(
        &app.ui_font,
        if app.collision_edit_mode {
            "Click a COL face to edit material/light."
        } else {
            "Enable Edit Faces to pick individual triangles."
        },
        x + 14.0,
        y + 190.0,
        ui_muted(),
    );
    if let Some(selected) = app.selected_col_face {
        ui_text(&app.ui_font, "Selected Face", x + 14.0, y + 226.0, ui_dim());
        ui_text(
            &app.ui_font,
            &format!("#{} on element {}", selected.face, selected.placement),
            x + 14.0,
            y + 248.0,
            WHITE,
        );
        draw_input_box(app, InspectorField::CollisionFaceLight, "Light");
        draw_input_box(app, InspectorField::CollisionVertexX, "Vertex X");
        draw_input_box(app, InspectorField::CollisionVertexY, "Vertex Y");
        draw_input_box(app, InspectorField::CollisionVertexZ, "Vertex Z");
        draw_col_material_dropdown(app);
        if let Some(p) = app.placements.get(selected.placement) {
            if let Some(mesh) = element_collision_mesh(app, p) {
                if let Some(face) = mesh.faces.get(selected.face) {
                    ui_text(
                        &app.ui_font,
                        &format!("Verts {}, {}, {}", face.a, face.b, face.c),
                        x + 14.0,
                        TOP_H + 446.0 - app.properties_scroll,
                        LIGHTGRAY,
                    );
                }
            }
        }
    } else {
        ui_text(
            &app.ui_font,
            "No face selected",
            x + 14.0,
            y + 226.0,
            ui_dim(),
        );
    }
    draw_input_box(app, InspectorField::CameraSpeed, "Camera Speed");
    ui_text(
        &app.ui_font,
        "Moving/rotating the element moves its base model and COL together.",
        x + 14.0,
        y + 550.0,
        ui_muted(),
    );
    ui_text_size(&app.ui_font, "History", x + 14.0, y + 590.0, 18, WHITE);
    draw_properties_history(app, x, y + 600.0);
    // Draw the material dropdown's expanded list last so it overlays content below.
    draw_col_material_dropdown_popup(app);
    end_ui_clip();
    draw_inspector_scrollbar(app, collision_panel_scroll_max());
}

pub(crate) fn collision_panel_scroll_max() -> f32 {
    panel_scroll_max_for_bottom(TOP_H + 12.0 + 1_220.0)
}

pub(crate) fn draw_metric_row(
    font: &Font,
    label: &str,
    value: impl std::fmt::Display,
    x: f32,
    y: f32,
) {
    ui_text(font, label, x, y, ui_dim());
    ui_text(font, &value.to_string(), x + 168.0, y, WHITE);
}

pub(crate) fn timecyc_weather_button_rect(app: &AppState, slot: usize) -> Rect {
    let x = screen_width() - RIGHT_PANEL_W + 18.0;
    let y = TOP_H + 650.0 - app.properties_scroll;
    Rect::new(x + slot as f32 * 168.0, y, 150.0, 28.0)
}

pub(crate) fn timecyc_phase_button_rect(app: &AppState, slot: usize) -> Rect {
    let x = screen_width() - RIGHT_PANEL_W + 18.0;
    let y = TOP_H + 708.0 - app.properties_scroll;
    let col = slot % 2;
    let row = slot / 2;
    Rect::new(x + col as f32 * 168.0, y + row as f32 * 36.0, 150.0, 28.0)
}

pub(crate) fn fog_strength_button_rect(app: &AppState, slot: usize) -> Rect {
    let x = screen_width() - RIGHT_PANEL_W + 18.0;
    let y = TOP_H + 814.0 - app.properties_scroll;
    Rect::new(x + slot as f32 * 168.0, y, 150.0, 28.0)
}

pub(crate) fn find_duplicate_placements_button_rect(app: &AppState) -> Rect {
    Rect::new(
        screen_width() - RIGHT_PANEL_W + 18.0,
        TOP_H + 421.0 - app.properties_scroll,
        RIGHT_PANEL_W - 36.0,
        28.0,
    )
}

pub(crate) fn set_timecyc_weather(app: &mut AppState, index: usize) {
    if app.timecyc.data.weathers.is_empty() {
        return;
    }
    app.timecyc.weather_index = index % app.timecyc.data.weathers.len();
    let hour_count = app.timecyc.data.weathers[app.timecyc.weather_index]
        .samples
        .len()
        .max(1);
    app.timecyc.hour_index = app.timecyc.hour_index.min(hour_count - 1);
    save_timecyc_preference(&app.timecyc);
    app.status_message = format!(
        "Timecycle weather {}",
        active_timecyc_weather_name(&app.timecyc)
    );
}

pub(crate) fn set_timecyc_hour(app: &mut AppState, index: usize) {
    let Some(weather) = app.timecyc.data.weathers.get(app.timecyc.weather_index) else {
        return;
    };
    if weather.samples.is_empty() {
        return;
    }
    app.timecyc.hour_index = index % weather.samples.len();
    save_timecyc_preference(&app.timecyc);
    app.status_message = format!(
        "Timecycle time {}",
        active_timecyc_sample(&app.timecyc).label
    );
}

pub(crate) fn set_fog_strength(app: &mut AppState, strength: f32) {
    app.fog_strength = clamp_fog_strength(strength);
    save_fog_strength_preference(app.fog_strength);
    app.status_message = if app.fog_strength <= 0.001 {
        "Fog disabled".to_string()
    } else {
        format!("Fog strength {:.2}", app.fog_strength)
    };
}

pub(crate) fn timecyc_phase_hour_index(weather: &TimecycWeather, hour_label: &str) -> usize {
    weather
        .samples
        .iter()
        .position(|sample| sample.label.eq_ignore_ascii_case(hour_label))
        .unwrap_or(0)
}

pub(crate) fn active_timecyc_phase_label(app: &AppState) -> &str {
    let sample = active_timecyc_sample(&app.timecyc);
    TIMECYC_PHASES
        .iter()
        .find_map(|(phase, hour)| sample.label.eq_ignore_ascii_case(hour).then_some(*phase))
        .unwrap_or(sample.label.as_str())
}

pub(crate) fn save_project_thumbnail(app: &mut AppState, viewport: Rect) {
    for path in project_thumbnail_paths(&app.root) {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
    }
    draw_scene(app, viewport);
    let mut internal = unsafe { get_internal_gl() };
    internal.flush();
    drop(internal);
    unsafe {
        gl::Finish();
    }
    let screen = get_screen_data();
    if screen.width == 0 || screen.height == 0 || screen.bytes.is_empty() {
        return;
    }
    const THUMB_W: u32 = 320;
    const THUMB_H: u32 = 100;
    let src_w = screen.width as usize;
    let src_h = screen.height as usize;
    let vx = viewport.x.max(0.0).min(screen.width as f32 - 1.0) as usize;
    let vy = viewport.y.max(0.0).min(screen.height as f32 - 1.0) as usize;
    let vw = viewport.w.max(1.0).min(screen.width as f32 - vx as f32) as usize;
    let vh = viewport.h.max(1.0).min(screen.height as f32 - vy as f32) as usize;
    let mut out = vec![0u8; THUMB_W as usize * THUMB_H as usize * 4];
    for y in 0..THUMB_H as usize {
        for x in 0..THUMB_W as usize {
            let sx = vx + x * vw / THUMB_W as usize;
            let sy_top = vy + y * vh / THUMB_H as usize;
            let sy = src_h.saturating_sub(1).saturating_sub(sy_top);
            let src = (sy.min(src_h - 1) * src_w + sx.min(src_w - 1)) * 4;
            let dst = (y * THUMB_W as usize + x) * 4;
            out[dst..dst + 4].copy_from_slice(&screen.bytes[src..src + 4]);
        }
    }
    for path in project_thumbnail_paths(&app.root) {
        let _ = image::save_buffer(path, &out, THUMB_W, THUMB_H, image::ColorType::Rgba8);
    }
}

pub(crate) fn persist_project_session(app: &mut AppState, viewport: Rect) {
    save_project_camera_state(app);
    save_project_thumbnail(app, viewport);
}

pub(crate) fn handle_timecyc_click(app: &mut AppState, mouse: Vec2) -> bool {
    if app.active_tab != AppTab::Scene {
        return false;
    }
    if !inspector_panel_content_rect().contains(mouse) {
        return false;
    }
    if find_duplicate_placements_button_rect(app).contains(mouse) {
        if is_mouse_button_pressed(MouseButton::Left) {
            request_duplicate_placement_scan(app);
        }
        return true;
    }
    for slot in 0..2 {
        if timecyc_weather_button_rect(app, slot).contains(mouse) {
            if is_mouse_button_pressed(MouseButton::Left) {
                match slot {
                    0 => {
                        let len = app.timecyc.data.weathers.len().max(1);
                        set_timecyc_weather(app, (app.timecyc.weather_index + len - 1) % len);
                    }
                    1 => set_timecyc_weather(app, app.timecyc.weather_index + 1),
                    _ => {}
                }
            }
            return true;
        }
    }
    for (slot, (_, hour_label)) in TIMECYC_PHASES.iter().enumerate() {
        if timecyc_phase_button_rect(app, slot).contains(mouse) {
            if is_mouse_button_pressed(MouseButton::Left) {
                if let Some(weather) = app.timecyc.data.weathers.get(app.timecyc.weather_index) {
                    set_timecyc_hour(app, timecyc_phase_hour_index(weather, hour_label));
                }
            }
            return true;
        }
    }
    for slot in 0..2 {
        if fog_strength_button_rect(app, slot).contains(mouse) {
            if is_mouse_button_pressed(MouseButton::Left) {
                let delta = if slot == 0 { -0.25 } else { 0.25 };
                set_fog_strength(app, app.fog_strength + delta);
            }
            return true;
        }
    }
    false
}

pub(crate) fn draw_scene_panel(app: &AppState) {
    let x = screen_width() - RIGHT_PANEL_W + 12.0;
    let panel_y = TOP_H + 12.0;
    let y = panel_y - app.properties_scroll;
    let w = RIGHT_PANEL_W - 24.0;
    draw_panel_rect(
        &app.ui_font,
        Rect::new(
            x,
            panel_y,
            w,
            (screen_height() - STATUS_H - panel_y - 12.0).max(1.0),
        ),
        Some("Scene"),
    );
    begin_ui_clip(inspector_panel_content_rect());
    let active = app
        .element_states
        .iter()
        .filter(|state| !state.deleted && !state.hidden)
        .count();
    let hidden = app
        .element_states
        .iter()
        .filter(|state| !state.deleted && state.hidden)
        .count();
    let deleted = app
        .element_states
        .iter()
        .filter(|state| state.deleted)
        .count();
    let buildings = app
        .placements
        .iter()
        .filter(|p| p.tag == "building")
        .count();
    let objects = app.placements.iter().filter(|p| p.tag == "object").count();
    let scenery = app.placements.iter().filter(|p| p.tag == "scenery").count();
    let missing_col = missing_col_count(app);
    ui_text_size(
        &app.ui_font,
        "Resource Overview",
        x + 14.0,
        y + 64.0,
        18,
        WHITE,
    );
    draw_metric_row(&app.ui_font, "Zones", app.zones.len(), x + 14.0, y + 100.0);
    draw_metric_row(
        &app.ui_font,
        "Placements",
        app.placements.len(),
        x + 14.0,
        y + 126.0,
    );
    draw_metric_row(&app.ui_font, "Active", active, x + 14.0, y + 152.0);
    draw_metric_row(&app.ui_font, "Hidden", hidden, x + 14.0, y + 178.0);
    draw_metric_row(&app.ui_font, "Deleted", deleted, x + 14.0, y + 204.0);
    draw_metric_row(
        &app.ui_font,
        "Definitions",
        app.definitions.len(),
        x + 14.0,
        y + 230.0,
    );
    draw_metric_row(
        &app.ui_font,
        "Missing COL",
        missing_col,
        x + 14.0,
        y + 256.0,
    );

    ui_text_size(
        &app.ui_font,
        "Element Types",
        x + 14.0,
        y + 306.0,
        18,
        WHITE,
    );
    draw_metric_row(&app.ui_font, "Building", buildings, x + 14.0, y + 342.0);
    draw_metric_row(&app.ui_font, "Object", objects, x + 14.0, y + 368.0);
    draw_metric_row(&app.ui_font, "Scenery", scenery, x + 14.0, y + 394.0);
    text_button(
        &app.ui_font,
        find_duplicate_placements_button_rect(app),
        if app.duplicate_placement_scan_rx.is_some() {
            "Finding Overlapping Duplicates..."
        } else {
            "Find Overlapping Duplicates"
        },
        app.duplicate_placement_scan_rx.is_some(),
    );

    ui_text_size(&app.ui_font, "Selection", x + 14.0, y + 448.0, 18, WHITE);
    if let Some(p) = app.placements.get(app.selected) {
        draw_metric_row(
            &app.ui_font,
            "ID",
            ellipsize(&p.id, 20),
            x + 14.0,
            y + 462.0,
        );
        draw_metric_row(&app.ui_font, "Type", &p.tag, x + 14.0, y + 488.0);
        draw_metric_row(
            &app.ui_font,
            "Zone",
            ellipsize(&p.zone, 20),
            x + 14.0,
            y + 514.0,
        );
        draw_metric_row(
            &app.ui_font,
            "DFF",
            ellipsize(&p.dff, 20),
            x + 14.0,
            y + 540.0,
        );
    } else {
        ui_text(
            &app.ui_font,
            "No element selected",
            x + 14.0,
            y + 462.0,
            ui_dim(),
        );
    }

    let sample = active_timecyc_sample(&app.timecyc);
    ui_text_size(&app.ui_font, "Timecycle", x + 14.0, y + 600.0, 18, WHITE);
    draw_metric_row(
        &app.ui_font,
        "Weather",
        ellipsize(active_timecyc_weather_name(&app.timecyc), 18),
        x + 14.0,
        y + 636.0,
    );
    draw_metric_row(
        &app.ui_font,
        "Phase",
        active_timecyc_phase_label(app),
        x + 14.0,
        y + 662.0,
    );
    text_button(
        &app.ui_font,
        timecyc_weather_button_rect(app, 0),
        "< Weather",
        false,
    );
    text_button(
        &app.ui_font,
        timecyc_weather_button_rect(app, 1),
        "Weather >",
        false,
    );
    for (slot, (phase, hour_label)) in TIMECYC_PHASES.iter().enumerate() {
        let active = sample.label.eq_ignore_ascii_case(hour_label);
        text_button(
            &app.ui_font,
            timecyc_phase_button_rect(app, slot),
            phase,
            active,
        );
    }
    text_button(
        &app.ui_font,
        fog_strength_button_rect(app, 0),
        "Fog -",
        false,
    );
    text_button(
        &app.ui_font,
        fog_strength_button_rect(app, 1),
        "Fog +",
        false,
    );
    ui_text(
        &app.ui_font,
        &format!(
            "Amb {:.2} {:.2} {:.2}   Fog x{:.2} {:.0}-{:.0}",
            preview_ambient(sample).x,
            preview_ambient(sample).y,
            preview_ambient(sample).z,
            app.fog_strength,
            sample.fog_start.max(0.0),
            sample.far_clip,
        ),
        x + 14.0,
        y + 874.0,
        ui_muted(),
    );
    ui_text_size(&app.ui_font, "History", x + 14.0, y + 930.0, 18, WHITE);
    draw_properties_history(app, x, y + 940.0);
    end_ui_clip();
    draw_inspector_scrollbar(app, scene_panel_scroll_max());
}

pub(crate) fn scene_panel_scroll_max() -> f32 {
    panel_scroll_max_for_bottom(TOP_H + 12.0 + 1_560.0)
}

#[allow(dead_code)]
pub(crate) fn draw_assets_panel(app: &AppState) {
    let x = screen_width() - RIGHT_PANEL_W + 12.0;
    let y = TOP_H + 12.0;
    let w = RIGHT_PANEL_W - 24.0;
    draw_panel_rect(
        &app.ui_font,
        Rect::new(x, y, w, (screen_height() - STATUS_H - y - 12.0).max(480.0)),
        Some("Assets"),
    );
    let textured_batches = app
        .world_cells
        .iter()
        .flat_map(|cell| cell.batches.iter())
        .filter(|batch| batch.texture != 0)
        .count();
    let col_faces: usize = app.collisions.values().map(|mesh| mesh.faces.len()).sum();
    let pending_col = app.pending_col_writes.len();
    ui_text_size(&app.ui_font, "Loaded Assets", x + 14.0, y + 64.0, 18, WHITE);
    draw_metric_row(
        &app.ui_font,
        "DFF Meshes",
        app.meshes.len(),
        x + 14.0,
        y + 100.0,
    );
    draw_metric_row(
        &app.ui_font,
        "COL Meshes",
        app.collisions.len(),
        x + 14.0,
        y + 126.0,
    );
    draw_metric_row(&app.ui_font, "COL Faces", col_faces, x + 14.0, y + 152.0);
    draw_metric_row(
        &app.ui_font,
        "Textures",
        app.textures.len(),
        x + 14.0,
        y + 178.0,
    );
    draw_metric_row(
        &app.ui_font,
        "Texture Aliases",
        app.texture_alias_count,
        x + 14.0,
        y + 204.0,
    );
    draw_metric_row(
        &app.ui_font,
        "Textured Batches",
        textured_batches,
        x + 14.0,
        y + 230.0,
    );
    draw_metric_row(
        &app.ui_font,
        "Pending COL Edits",
        pending_col,
        x + 14.0,
        y + 256.0,
    );

    ui_text_size(&app.ui_font, "Render Cache", x + 14.0, y + 310.0, 18, WHITE);
    draw_metric_row(
        &app.ui_font,
        "World Cells",
        app.world_cells.len(),
        x + 14.0,
        y + 346.0,
    );
    draw_metric_row(
        &app.ui_font,
        "Display Cells",
        app.scene_cells.len(),
        x + 14.0,
        y + 372.0,
    );
    draw_metric_row(
        &app.ui_font,
        "Draw Radius",
        format!("{:.0}", app.options.draw_radius),
        x + 14.0,
        y + 398.0,
    );
    draw_metric_row(
        &app.ui_font,
        "Fast VBO",
        app.options.fast_vbo,
        x + 14.0,
        y + 424.0,
    );
    draw_metric_row(
        &app.ui_font,
        "MSAA",
        format!("{}x", app.options.msaa_samples),
        x + 14.0,
        y + 450.0,
    );

    ui_text_size(&app.ui_font, "Paths", x + 14.0, y + 504.0, 18, WHITE);
    ui_text(
        &app.ui_font,
        &ellipsize(app.root.join("imgs").to_string_lossy().as_ref(), 38),
        x + 14.0,
        y + 540.0,
        LIGHTGRAY,
    );
    ui_text(
        &app.ui_font,
        &ellipsize(app.root.join("txd_build").to_string_lossy().as_ref(), 38),
        x + 14.0,
        y + 566.0,
        LIGHTGRAY,
    );
}

#[cfg(test)]
mod placement_override_tests {
    use super::*;

    fn placement_with_attrs(attrs: &[(&str, &str)]) -> Placement {
        Placement {
            id: "test".to_string(),
            dff: "test".to_string(),
            zone: "test".to_string(),
            tag: "object".to_string(),
            attrs: attrs
                .iter()
                .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
                .collect(),
            pos: V3::default(),
            rot: V3::default(),
        }
    }

    #[test]
    fn placement_override_flags_match_all_loader_sources_and_precedence() {
        let placement = placement_with_attrs(&[
            ("flags", "21,disable_collisions"),
            ("override_flags", "frozen,no_stream"),
        ]);
        assert!(placement_override_flag_enabled(&placement, "double_sided"));
        assert!(placement_override_flag_enabled(
            &placement,
            "disable_collisions"
        ));
        assert!(placement_override_flag_enabled(&placement, "frozen"));
        assert!(placement_override_flag_enabled(&placement, "no_stream"));

        let explicit_collision = placement_with_attrs(&[
            ("collisions", "true"),
            ("overrideFlags", "disable_collisions"),
        ]);
        assert!(!placement_override_flag_enabled(
            &explicit_collision,
            "disable_collisions"
        ));

        let explicit_breakable = placement_with_attrs(&[("breakable", "false")]);
        assert!(!placement_override_flag_enabled(
            &explicit_breakable,
            "breakable"
        ));
        assert!(placement_override_flag_enabled(
            &explicit_breakable,
            "unbreakable"
        ));
    }

    #[test]
    fn setting_placement_override_removes_conflicting_loader_aliases() {
        let mut placement = placement_with_attrs(&[
            ("flags", "50,is_road"),
            ("override_flags", "frozen,no_stream"),
            ("collisions", "true"),
            ("noCollisions", "true"),
        ]);

        set_placement_override_flag(&mut placement, "disable_collisions", true);

        assert_eq!(
            placement.attrs.get("flags").map(String::as_str),
            Some("is_road")
        );
        assert_eq!(
            placement.attrs.get("overrideFlags").map(String::as_str),
            Some("frozen,no_stream,disable_collisions")
        );
        assert!(!placement.attrs.contains_key("override_flags"));
        assert!(!placement.attrs.contains_key("collisions"));
        assert!(!placement.attrs.contains_key("noCollisions"));
        assert!(placement_override_flag_enabled(
            &placement,
            "disable_collisions"
        ));

        set_placement_override_flag(&mut placement, "disable_collisions", false);
        assert_eq!(
            placement.attrs.get("overrideFlags").map(String::as_str),
            Some("frozen,no_stream")
        );
        assert!(!placement_override_flag_enabled(
            &placement,
            "disable_collisions"
        ));
    }
}
