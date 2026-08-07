use super::super::*;

const EDIT_ROW_H: f32 = 30.0;
const EDIT_VERTEX_PICK_RADIUS: f32 = 7.0;
const COLLISION_TAB_VERTEX_PICK_RADIUS: f32 = 7.0;
const VERTEX_OCCLUSION_TOLERANCE: f32 = 6.0;
const MERGE_BY_DISTANCE_DEFAULT: f32 = 0.0001;

pub(crate) fn editing_panel_rect() -> Rect {
    Rect::new(14.0, TOP_H + 10.0, screen_width() - 28.0, 36.0)
}

pub(crate) fn editing_archive_rect() -> Rect {
    Rect::new(
        14.0,
        TOP_H + 54.0,
        390.0_f32.min(screen_width() * 0.28),
        screen_height() - TOP_H - STATUS_H - 66.0,
    )
}

pub(crate) fn editing_asset_rect() -> Rect {
    let w = 520.0_f32.min(screen_width() * 0.36).max(400.0);
    Rect::new(
        screen_width() - w - 14.0,
        TOP_H + 54.0,
        w,
        screen_height() - TOP_H - STATUS_H - 66.0,
    )
}

pub(crate) fn editing_center_rect() -> Rect {
    let left = editing_archive_rect();
    let right = editing_asset_rect();
    Rect::new(
        left.x + left.w + 18.0,
        TOP_H + 54.0,
        (right.x - left.x - left.w - 36.0).max(1.0),
        screen_height() - TOP_H - STATUS_H - 66.0,
    )
}

pub(crate) fn editing_img_prev_rect() -> Rect {
    let panel = editing_panel_rect();
    Rect::new(panel.x + 92.0, panel.y + 4.0, 34.0, 28.0)
}

pub(crate) fn editing_img_next_rect() -> Rect {
    let panel = editing_panel_rect();
    Rect::new(panel.x + 132.0, panel.y + 4.0, 34.0, 28.0)
}

pub(crate) fn editing_img_open_rect() -> Rect {
    let panel = editing_panel_rect();
    Rect::new(panel.x + 176.0, panel.y + 4.0, 118.0, 28.0)
}

pub(crate) fn editing_img_choose_rect() -> Rect {
    let panel = editing_panel_rect();
    Rect::new(panel.x + 304.0, panel.y + 4.0, 132.0, 28.0)
}

pub(crate) fn editing_img_save_rect() -> Rect {
    let panel = editing_panel_rect();
    Rect::new(panel.x + panel.w - 128.0, panel.y + 4.0, 112.0, 28.0)
}

pub(crate) fn editing_box_select_mode_rect() -> Rect {
    let face = editing_select_face_mode_rect();
    Rect::new(face.x + face.w + 52.0, face.y, 88.0, 28.0)
}

pub(crate) fn editing_box_select_minus_rect() -> Rect {
    let mode = editing_box_select_mode_rect();
    Rect::new(mode.x + mode.w + 8.0, mode.y, 28.0, 28.0)
}

pub(crate) fn editing_box_select_plus_rect() -> Rect {
    let minus = editing_box_select_minus_rect();
    Rect::new(minus.x + minus.w + 6.0, minus.y, 28.0, 28.0)
}

pub(crate) fn editing_search_rect() -> Rect {
    let left = editing_archive_rect();
    Rect::new(left.x + 12.0, left.y + 44.0, left.w - 24.0, 28.0)
}

pub(crate) fn editing_merge_img_rect() -> Rect {
    let left = editing_archive_rect();
    Rect::new(left.x + left.w - 122.0, left.y + 7.0, 110.0, 28.0)
}

pub(crate) fn editing_row_rect(row: usize) -> Rect {
    let left = editing_archive_rect();
    Rect::new(
        left.x + 10.0,
        left.y + 108.0 + row as f32 * EDIT_ROW_H,
        left.w - 20.0,
        EDIT_ROW_H - 3.0,
    )
}

pub(crate) fn editing_open_entry_rect() -> Rect {
    let left = editing_archive_rect();
    Rect::new(left.x + 10.0, left.y + left.h - 36.0, 30.0, 28.0)
}

pub(crate) fn editing_add_entry_rect() -> Rect {
    let left = editing_archive_rect();
    Rect::new(left.x + 48.0, left.y + left.h - 36.0, 30.0, 28.0)
}

pub(crate) fn editing_replace_entry_rect() -> Rect {
    let left = editing_archive_rect();
    Rect::new(left.x + 86.0, left.y + left.h - 36.0, 30.0, 28.0)
}

pub(crate) fn editing_delete_entry_rect() -> Rect {
    let left = editing_archive_rect();
    Rect::new(left.x + 124.0, left.y + left.h - 36.0, 30.0, 28.0)
}

pub(crate) fn editing_extract_entry_rect() -> Rect {
    let left = editing_archive_rect();
    Rect::new(left.x + 162.0, left.y + left.h - 36.0, 30.0, 28.0)
}

pub(crate) fn editing_txd_add_rect() -> Rect {
    let right = editing_asset_rect();
    let w = (right.w - 46.0) / 4.0;
    Rect::new(right.x + 14.0, right.y + right.h - 38.0, w, 28.0)
}

pub(crate) fn editing_txd_replace_rect() -> Rect {
    let right = editing_asset_rect();
    let w = (right.w - 46.0) / 4.0;
    Rect::new(right.x + 20.0 + w, right.y + right.h - 38.0, w, 28.0)
}

pub(crate) fn editing_txd_rename_rect() -> Rect {
    let right = editing_asset_rect();
    let w = (right.w - 46.0) / 4.0;
    Rect::new(right.x + 26.0 + w * 2.0, right.y + right.h - 38.0, w, 28.0)
}

pub(crate) fn editing_txd_export_all_rect() -> Rect {
    let right = editing_asset_rect();
    let w = (right.w - 46.0) / 4.0;
    Rect::new(right.x + 32.0 + w * 3.0, right.y + right.h - 38.0, w, 28.0)
}

pub(crate) fn editing_txd_search_rect() -> Rect {
    let right = editing_asset_rect();
    Rect::new(right.x + 14.0, right.y + 74.0, right.w - 28.0, 28.0)
}

pub(crate) fn editing_txd_list_rect() -> Rect {
    let right = editing_asset_rect();
    Rect::new(
        right.x + 14.0,
        right.y + 110.0,
        right.w - 28.0,
        (right.h - 158.0).max(32.0),
    )
}

fn editing_txd_material_button_rect() -> Rect {
    let center = editing_center_rect();
    Rect::new(
        center.x + 22.0,
        center.y + center.h - 48.0,
        center.w - 44.0,
        30.0,
    )
}

fn editing_txd_material_picker_rect() -> Rect {
    let center = editing_center_rect();
    let width = (center.w - 64.0).min(520.0);
    let height = (center.h - 86.0).min(420.0);
    Rect::new(
        center.x + (center.w - width) * 0.5,
        center.y + (center.h - height) * 0.5,
        width,
        height,
    )
}

fn editing_txd_material_scope_rect(scope: CollisionMaterialAssignmentScope) -> Rect {
    let popup = editing_txd_material_picker_rect();
    let w = (popup.w - 56.0) / 3.0;
    let slot = match scope {
        CollisionMaterialAssignmentScope::ExactTxd => 0,
        CollisionMaterialAssignmentScope::IdenticalContent => 1,
        CollisionMaterialAssignmentScope::GlobalName => 2,
    };
    Rect::new(
        popup.x + 18.0 + slot as f32 * (w + 10.0),
        popup.y + 48.0,
        w,
        28.0,
    )
}

fn editing_txd_material_search_rect() -> Rect {
    let popup = editing_txd_material_picker_rect();
    Rect::new(popup.x + 18.0, popup.y + 86.0, popup.w - 36.0, 28.0)
}

fn editing_txd_material_list_rect() -> Rect {
    let popup = editing_txd_material_picker_rect();
    Rect::new(
        popup.x + 18.0,
        popup.y + 124.0,
        popup.w - 36.0,
        popup.h - 170.0,
    )
}

fn editing_txd_material_scrollbar_rect() -> Rect {
    let list = editing_txd_material_list_rect();
    Rect::new(list.x + list.w - 8.0, list.y, 8.0, list.h)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CollisionMaterialPickerValue {
    NoCollision,
    Material(u8),
}

fn editing_dff_material_texture_fingerprint(
    txd_textures: &TxdTextureIndex,
    dff: &EditingDffState,
    material_index: usize,
) -> Option<TextureContentFingerprint> {
    dff.preview_mesh
        .as_ref()
        .and_then(|mesh| {
            mesh.parts
                .iter()
                .find(|part| part.material_index == material_index)
                .and_then(|part| part.texture_fingerprint)
        })
        .or_else(|| {
            let texture = dff.raw.material_textures.get(material_index)?;
            texture_content_fingerprint(txd_textures, texture, dff.txd_context.as_deref())
        })
}

fn editing_dff_texture_fingerprint(
    txd_textures: &TxdTextureIndex,
    dff: &EditingDffState,
) -> Option<TextureContentFingerprint> {
    editing_dff_material_texture_fingerprint(txd_textures, dff, dff.selected_material)
}

fn collision_material_scope_available(
    scope: CollisionMaterialAssignmentScope,
    txd_name: Option<&str>,
    fingerprint: Option<TextureContentFingerprint>,
) -> bool {
    match scope {
        CollisionMaterialAssignmentScope::ExactTxd => txd_name.is_some(),
        CollisionMaterialAssignmentScope::IdenticalContent => fingerprint.is_some(),
        CollisionMaterialAssignmentScope::GlobalName => true,
    }
}

fn editing_txd_material_filtered(
    search: &str,
) -> Vec<(CollisionMaterialPickerValue, &'static str)> {
    let needle = search.trim().to_ascii_lowercase();
    let mut matches = Vec::new();
    if needle.is_empty()
        || "no collision".contains(&needle)
        || "none".contains(&needle)
        || "exclude".contains(&needle)
        || "disabled".contains(&needle)
    {
        matches.push((CollisionMaterialPickerValue::NoCollision, "No Collision"));
    }
    matches.extend(
        GTA_SA_COL_MATERIALS
            .iter()
            .copied()
            .filter(|(id, name)| {
                needle.is_empty()
                    || name.to_ascii_lowercase().contains(&needle)
                    || id.to_string().contains(&needle)
            })
            .map(|(id, name)| (CollisionMaterialPickerValue::Material(id), name)),
    );
    matches
}

fn editing_txd_material_scroll_from_mouse(search: &str, mouse: Vec2) -> Option<f32> {
    let track = editing_txd_material_scrollbar_rect();
    if !track.contains(mouse) {
        return None;
    }
    let list = editing_txd_material_list_rect();
    let row_h = 26.0;
    let visible = (list.h / row_h).floor().max(1.0) as usize;
    let count = editing_txd_material_filtered(search).len();
    let max_scroll = count.saturating_sub(visible) as f32;
    if max_scroll <= 0.0 {
        return Some(0.0);
    }
    let thumb_h = (track.h * visible as f32 / count as f32).clamp(24.0, track.h);
    let travel = (track.h - thumb_h).max(1.0);
    let position = (mouse.y - track.y - thumb_h * 0.5).clamp(0.0, travel);
    Some(position / travel * max_scroll)
}

// Shared DFF control geometry: two-column layout anchored to the panel bottom.
fn dff_ctl_x0(right: Rect) -> f32 {
    right.x + 14.0
}
fn dff_ctl_fullw(right: Rect) -> f32 {
    right.w - 28.0
}
fn dff_ctl_halfw(right: Rect) -> f32 {
    (dff_ctl_fullw(right) - 10.0) * 0.5
}
fn dff_ctl_x2(right: Rect) -> f32 {
    dff_ctl_x0(right) + dff_ctl_halfw(right) + 10.0
}

// ---------------------------------------------------------------------------
// DFF editing panel: collapsible-section layout.
//
// The panel is split into a fixed header (asset info), a scrollable stack of
// collapsible sections, and a pinned footer (Flip Normals / Stage DFF).
// `dff_panel_layout` computes every control rect once per frame; both the
// draw pass and the click/wheel handlers consume the same layout so hit
// testing always matches what is on screen.
// ---------------------------------------------------------------------------

pub(crate) const DFF_SECTION_COUNT: usize = 10;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum DffSection {
    Effects = 0,
    Lighting = 1,
    Fractures = 2,
    Materials = 3,
    FaceTexture = 4,
    MaterialAnim = 5,
    UvTools = 6,
    Mesh = 7,
    Cutter = 8,
    Lod = 9,
}

pub(crate) const DFF_SECTIONS: [DffSection; DFF_SECTION_COUNT] = [
    DffSection::Effects,
    DffSection::Lighting,
    DffSection::Fractures,
    DffSection::Materials,
    DffSection::FaceTexture,
    DffSection::MaterialAnim,
    DffSection::UvTools,
    DffSection::Mesh,
    DffSection::Cutter,
    DffSection::Lod,
];

impl DffSection {
    pub(crate) fn title(self) -> &'static str {
        match self {
            DffSection::Effects => "2DFX Effects",
            DffSection::Lighting => "Face Lighting",
            DffSection::Fractures => "Breakable Fracture Zones",
            DffSection::Materials => "Materials",
            DffSection::FaceTexture => "Face Texture",
            DffSection::MaterialAnim => "Material Animation",
            DffSection::UvTools => "UV Tools",
            DffSection::Mesh => "Mesh Tools",
            DffSection::Cutter => "Boolean Cutter",
            DffSection::Lod => "LOD Generator",
        }
    }
}

pub(crate) fn dff_default_collapsed() -> [bool; DFF_SECTION_COUNT] {
    let mut collapsed = [false; DFF_SECTION_COUNT];
    collapsed[DffSection::MaterialAnim as usize] = true;
    collapsed[DffSection::UvTools as usize] = true;
    collapsed[DffSection::Cutter as usize] = true;
    collapsed
}

pub(crate) const DFF_BTN_H: f32 = 34.0;
pub(crate) const DFF_SMALL_BTN_H: f32 = 30.0;
pub(crate) const DFF_SEC_HEADER_H: f32 = 32.0;
pub(crate) const DFF_MAT_ROW_H: f32 = 40.0;
pub(crate) const DFF_2DFX_ROW_H: f32 = 28.0;
pub(crate) const DFF_LIGHT_ROW_H: f32 = 32.0;
pub(crate) const DFF_MAT_VISIBLE_MAX: usize = 7;
pub(crate) const DFF_2DFX_VISIBLE_MAX: usize = 8;

pub(crate) struct DffPanelLayout {
    pub(crate) content: Rect,
    pub(crate) content_height: f32,
    pub(crate) headers: [Rect; DFF_SECTION_COUNT],
    pub(crate) rows_2dfx: Vec<Rect>,
    pub(crate) lighting_rows: Vec<Rect>,
    pub(crate) fracture_rows: Vec<Rect>,
    pub(crate) add_2dfx: Option<Rect>,
    pub(crate) delete_2dfx: Option<Rect>,
    pub(crate) type_2dfx: Option<Rect>,
    pub(crate) payload_2dfx: Option<Rect>,
    pub(crate) add_2dfx_corona_preset: Option<Rect>,
    pub(crate) regenerate_2dfx_coronas: Option<Rect>,
    pub(crate) generate_fractures: Option<Rect>,
    pub(crate) manual_fracture_zone: Option<Rect>,
    pub(crate) fracture_origin: Option<Rect>,
    pub(crate) clear_fractures: Option<Rect>,
    pub(crate) simulate_fractures: Option<Rect>,
    pub(crate) material_list: Option<Rect>,
    pub(crate) material_visible: usize,
    pub(crate) view_texture: Option<Rect>,
    pub(crate) rename_texture: Option<Rect>,
    pub(crate) duplicate_texture: Option<Rect>,
    pub(crate) set_material_texture_from_txd: Option<Rect>,
    pub(crate) set_material_texture_browse: Option<Rect>,
    pub(crate) new_material_for_faces: Option<Rect>,
    pub(crate) assign_material_to_faces: Option<Rect>,
    pub(crate) delete_unused_material: Option<Rect>,
    pub(crate) material_color: Option<[Rect; 4]>,
    pub(crate) material_color_presets: Option<[Rect; 9]>,
    pub(crate) material_alpha_presets: Option<[Rect; 5]>,
    pub(crate) material_color_swatch: Option<Rect>,
    pub(crate) material_surface: Option<[Rect; 3]>,
    pub(crate) collision_material: Option<Rect>,
    pub(crate) shadow_casting_toggle: Option<Rect>,
    pub(crate) shadow_casting_scope: Option<Rect>,
    pub(crate) emitter_toggle: Option<Rect>,
    pub(crate) emitter_source: Option<Rect>,
    pub(crate) emitter_scope: Option<Rect>,
    pub(crate) emitter_cast_mode: Option<Rect>,
    pub(crate) emitter_max_grouping_size: Option<Rect>,
    pub(crate) emitter_point_up_strength: Option<Rect>,
    pub(crate) emitter_point_down_strength: Option<Rect>,
    pub(crate) emitter_point_sides_strength: Option<Rect>,
    pub(crate) emitter_day: Option<Rect>,
    pub(crate) emitter_night: Option<Rect>,
    pub(crate) emitter_inversed: Option<Rect>,
    pub(crate) emitter_strength: Option<Rect>,
    pub(crate) emitter_falloff: Option<Rect>,
    pub(crate) emitter_temperature: Option<Rect>,
    pub(crate) emitter_color: Option<[Rect; 3]>,
    pub(crate) tex_from_txd: Option<Rect>,
    pub(crate) tex_browse: Option<Rect>,
    pub(crate) anim_assign: Option<Rect>,
    pub(crate) anim_clear: Option<Rect>,
    pub(crate) anim_motion: Option<[Rect; 4]>,
    pub(crate) uv_nudge: Option<[Rect; 4]>,
    pub(crate) uv_scale: Option<[Rect; 2]>,
    pub(crate) uv_rotate: Option<[Rect; 2]>,
    pub(crate) uv_unwrap_face: Option<Rect>,
    pub(crate) uv_unwrap_material: Option<Rect>,
    pub(crate) make_face: Option<Rect>,
    pub(crate) delete_face: Option<Rect>,
    pub(crate) delete_vertex: Option<Rect>,
    pub(crate) delete_material_faces: Option<Rect>,
    pub(crate) extrude_selection: Option<Rect>,
    pub(crate) merge_selected: Option<Rect>,
    pub(crate) merge_distance: Option<Rect>,
    pub(crate) subdivide: Option<Rect>,
    pub(crate) duplicate_faces: Option<Rect>,
    pub(crate) duplicate_material: Option<Rect>,
    pub(crate) separate_faces: Option<Rect>,
    pub(crate) pivot_to_selection: Option<Rect>,
    pub(crate) pivot_to_bounds: Option<Rect>,
    pub(crate) cutter_add: Option<Rect>,
    pub(crate) cutter_clear: Option<Rect>,
    pub(crate) cutter_resize: Option<[Rect; 6]>,
    pub(crate) generate_lod: Option<Rect>,
    pub(crate) generate_collision: Rect,
    pub(crate) flip_normals: Rect,
    pub(crate) stage: Rect,
}

pub(crate) fn dff_panel_header_height(dff: &EditingDffState) -> f32 {
    if dff.normalized_warning { 184.0 } else { 144.0 }
}

pub(crate) fn dff_panel_layout(
    dff: &EditingDffState,
    emitter: MaterialEmitter,
    lighting_entries: usize,
) -> DffPanelLayout {
    let panel = editing_asset_rect();
    let x0 = dff_ctl_x0(panel);
    let fullw = dff_ctl_fullw(panel);
    let halfw = dff_ctl_halfw(panel);
    let x2 = dff_ctl_x2(panel);
    let thirdw = (fullw - 20.0) / 3.0;
    let header_h = dff_panel_header_height(dff);
    let footer_h = DFF_BTN_H + 24.0;
    let content = Rect::new(
        panel.x + 1.0,
        panel.y + header_h,
        panel.w - 2.0,
        (panel.h - header_h - footer_h).max(40.0),
    );
    let gap = 10.0;
    let zero = Rect::new(0.0, 0.0, 0.0, 0.0);
    let mut layout = DffPanelLayout {
        content,
        content_height: 0.0,
        headers: [zero; DFF_SECTION_COUNT],
        rows_2dfx: Vec::new(),
        lighting_rows: Vec::new(),
        fracture_rows: Vec::new(),
        add_2dfx: None,
        delete_2dfx: None,
        type_2dfx: None,
        payload_2dfx: None,
        add_2dfx_corona_preset: None,
        regenerate_2dfx_coronas: None,
        generate_fractures: None,
        manual_fracture_zone: None,
        fracture_origin: None,
        clear_fractures: None,
        simulate_fractures: None,
        material_list: None,
        material_visible: 0,
        view_texture: None,
        rename_texture: None,
        duplicate_texture: None,
        set_material_texture_from_txd: None,
        set_material_texture_browse: None,
        new_material_for_faces: None,
        assign_material_to_faces: None,
        delete_unused_material: None,
        material_color: None,
        material_color_presets: None,
        material_alpha_presets: None,
        material_color_swatch: None,
        material_surface: None,
        collision_material: None,
        shadow_casting_toggle: None,
        shadow_casting_scope: None,
        emitter_toggle: None,
        emitter_source: None,
        emitter_scope: None,
        emitter_cast_mode: None,
        emitter_max_grouping_size: None,
        emitter_point_up_strength: None,
        emitter_point_down_strength: None,
        emitter_point_sides_strength: None,
        emitter_day: None,
        emitter_night: None,
        emitter_inversed: None,
        emitter_strength: None,
        emitter_falloff: None,
        emitter_temperature: None,
        emitter_color: None,
        tex_from_txd: None,
        tex_browse: None,
        anim_assign: None,
        anim_clear: None,
        anim_motion: None,
        uv_nudge: None,
        uv_scale: None,
        uv_rotate: None,
        uv_unwrap_face: None,
        uv_unwrap_material: None,
        make_face: None,
        delete_face: None,
        delete_vertex: None,
        delete_material_faces: None,
        extrude_selection: None,
        merge_selected: None,
        merge_distance: None,
        subdivide: None,
        duplicate_faces: None,
        duplicate_material: None,
        separate_faces: None,
        pivot_to_selection: None,
        pivot_to_bounds: None,
        cutter_add: None,
        cutter_clear: None,
        cutter_resize: None,
        generate_lod: None,
        generate_collision: Rect::new(x0, panel.y + panel.h - DFF_BTN_H - 12.0, thirdw, DFF_BTN_H),
        flip_normals: Rect::new(
            x0 + thirdw + 10.0,
            panel.y + panel.h - DFF_BTN_H - 12.0,
            thirdw,
            DFF_BTN_H,
        ),
        stage: Rect::new(
            x0 + (thirdw + 10.0) * 2.0,
            panel.y + panel.h - DFF_BTN_H - 12.0,
            thirdw,
            DFF_BTN_H,
        ),
    };
    let mut y = content.y + 4.0 - dff.panel_scroll;
    for section in DFF_SECTIONS {
        let idx = section as usize;
        layout.headers[idx] = Rect::new(x0, y, fullw, DFF_SEC_HEADER_H);
        y += DFF_SEC_HEADER_H + 6.0;
        if dff.panel_collapsed[idx] {
            y += gap - 6.0;
            continue;
        }
        match section {
            DffSection::Effects => {
                let count = dff.raw.effects_2dfx.len().min(DFF_2DFX_VISIBLE_MAX);
                for _ in 0..count {
                    layout
                        .rows_2dfx
                        .push(Rect::new(x0, y, fullw, DFF_2DFX_ROW_H - 3.0));
                    y += DFF_2DFX_ROW_H;
                }
                if count > 0 {
                    y += 4.0;
                }
                layout.add_2dfx = Some(Rect::new(x0, y, halfw, DFF_BTN_H));
                if dff.selected_2dfx.is_some() {
                    layout.delete_2dfx = Some(Rect::new(x2, y, halfw, DFF_BTN_H));
                    y += DFF_BTN_H + 6.0;
                    layout.type_2dfx = Some(Rect::new(x0, y, halfw, DFF_BTN_H));
                    layout.payload_2dfx = Some(Rect::new(x2, y, halfw, DFF_BTN_H));
                }
                y += DFF_BTN_H + 6.0;
                layout.add_2dfx_corona_preset = Some(Rect::new(x0, y, fullw, DFF_BTN_H));
                y += DFF_BTN_H + 6.0;
                layout.regenerate_2dfx_coronas = Some(Rect::new(x0, y, fullw, DFF_BTN_H));
                y += DFF_BTN_H + gap;
            }
            DffSection::Lighting => {
                for _ in 0..lighting_entries {
                    layout
                        .lighting_rows
                        .push(Rect::new(x0, y, fullw, DFF_LIGHT_ROW_H - 3.0));
                    y += DFF_LIGHT_ROW_H;
                }
                if lighting_entries == 0 {
                    y += 24.0;
                } else {
                    y += 4.0;
                }
            }
            DffSection::Fractures => {
                let zone_count = fracture_component_for_dff(dff)
                    .ok()
                    .and_then(|index| dff.raw.components.get(index))
                    .and_then(|component| component.breakable.as_ref())
                    .map(|breakable| breakable.groups.len())
                    .unwrap_or(0);
                for _ in 0..zone_count {
                    layout
                        .fracture_rows
                        .push(Rect::new(x0, y, fullw, DFF_LIGHT_ROW_H - 3.0));
                    y += DFF_LIGHT_ROW_H;
                }
                if zone_count > 0 {
                    y += 4.0;
                } else {
                    y += 24.0;
                }
                layout.generate_fractures = Some(Rect::new(x0, y, halfw, DFF_BTN_H));
                layout.manual_fracture_zone = Some(Rect::new(x2, y, halfw, DFF_BTN_H));
                y += DFF_BTN_H + 6.0;
                layout.fracture_origin = Some(Rect::new(x0, y, halfw, DFF_BTN_H));
                layout.clear_fractures = Some(Rect::new(x2, y, halfw, DFF_BTN_H));
                y += DFF_BTN_H + 6.0;
                layout.simulate_fractures = Some(Rect::new(x0, y, fullw, DFF_BTN_H));
                y += DFF_BTN_H + gap;
            }
            DffSection::Materials => {
                let total = dff_material_slot_count(&dff.raw);
                let visible = total.min(DFF_MAT_VISIBLE_MAX);
                layout.material_visible = visible;
                layout.material_list =
                    Some(Rect::new(x0, y, fullw, visible as f32 * DFF_MAT_ROW_H));
                y += visible as f32 * DFF_MAT_ROW_H + 6.0;
                layout.view_texture = Some(Rect::new(x0, y, halfw, DFF_BTN_H));
                layout.rename_texture = Some(Rect::new(x2, y, halfw, DFF_BTN_H));
                y += DFF_BTN_H + 6.0;
                layout.duplicate_texture = Some(Rect::new(x0, y, fullw, DFF_BTN_H));
                y += DFF_BTN_H + 6.0;
                layout.set_material_texture_from_txd = Some(Rect::new(x0, y, halfw, DFF_BTN_H));
                layout.set_material_texture_browse = Some(Rect::new(x2, y, halfw, DFF_BTN_H));
                y += DFF_BTN_H + 6.0;
                layout.new_material_for_faces = Some(Rect::new(x0, y, halfw, DFF_BTN_H));
                layout.assign_material_to_faces = Some(Rect::new(x2, y, halfw, DFF_BTN_H));
                y += DFF_BTN_H + 6.0;
                layout.delete_unused_material = Some(Rect::new(x0, y, fullw, DFF_BTN_H));
                y += DFF_BTN_H + 12.0;
                let mut color = [zero; 4];
                for rect in &mut color {
                    *rect = Rect::new(x0, y, fullw, 30.0);
                    y += 42.0;
                }
                layout.material_color = Some(color);
                layout.material_color_swatch = Some(Rect::new(x0, y, 44.0, 58.0));
                let preset_x = x0 + 54.0;
                let preset_w = (fullw - 54.0 - 8.0 * 4.0) / 9.0;
                let mut color_presets = [zero; 9];
                for (idx, rect) in color_presets.iter_mut().enumerate() {
                    *rect = Rect::new(preset_x + idx as f32 * (preset_w + 4.0), y, preset_w, 24.0);
                }
                layout.material_color_presets = Some(color_presets);
                let alpha_w = (fullw - 54.0 - 4.0 * 4.0) / 5.0;
                let mut alpha_presets = [zero; 5];
                for (idx, rect) in alpha_presets.iter_mut().enumerate() {
                    *rect = Rect::new(
                        preset_x + idx as f32 * (alpha_w + 4.0),
                        y + 34.0,
                        alpha_w,
                        24.0,
                    );
                }
                layout.material_alpha_presets = Some(alpha_presets);
                y += 70.0;
                let mut surface = [zero; 3];
                for rect in &mut surface {
                    *rect = Rect::new(x0, y, fullw, 30.0);
                    y += 42.0;
                }
                layout.material_surface = Some(surface);
                layout.collision_material = Some(Rect::new(x0, y, fullw, DFF_BTN_H));
                y += DFF_BTN_H + 6.0;
                layout.shadow_casting_toggle = Some(Rect::new(x0, y, fullw, DFF_BTN_H));
                y += DFF_BTN_H + 6.0;
                if !selected_dff_faces_are_emitter_target(dff) {
                    layout.shadow_casting_scope = Some(Rect::new(x0, y, fullw, DFF_BTN_H));
                    y += DFF_BTN_H + 6.0;
                }
                layout.emitter_toggle = Some(Rect::new(x0, y, fullw, DFF_BTN_H));
                y += DFF_BTN_H + gap;
                if emitter.enabled {
                    layout.emitter_source = Some(Rect::new(x0, y, fullw, DFF_BTN_H));
                    y += DFF_BTN_H + 6.0;
                    if !selected_dff_faces_are_emitter_target(dff) {
                        layout.emitter_scope = Some(Rect::new(x0, y, fullw, DFF_BTN_H));
                        y += DFF_BTN_H + 6.0;
                    }
                    layout.emitter_cast_mode = Some(Rect::new(x0, y, fullw, DFF_BTN_H));
                    y += DFF_BTN_H + 6.0;
                    if emitter.cast_mode == MaterialEmitterCastMode::Point {
                        layout.emitter_max_grouping_size = Some(Rect::new(x0, y, fullw, 30.0));
                        y += 30.0 + 12.0;
                        layout.emitter_point_up_strength = Some(Rect::new(x0, y, fullw, 30.0));
                        y += 30.0 + 12.0;
                        layout.emitter_point_down_strength = Some(Rect::new(x0, y, fullw, 30.0));
                        y += 30.0 + 12.0;
                        layout.emitter_point_sides_strength = Some(Rect::new(x0, y, fullw, 30.0));
                        y += 30.0 + 12.0;
                    }
                    layout.emitter_day = Some(Rect::new(x0, y, halfw, DFF_BTN_H));
                    layout.emitter_night = Some(Rect::new(x2, y, halfw, DFF_BTN_H));
                    y += DFF_BTN_H + 6.0;
                    layout.emitter_inversed = Some(Rect::new(x0, y, fullw, DFF_BTN_H));
                    y += DFF_BTN_H + 18.0;
                    layout.emitter_strength = Some(Rect::new(x0, y, fullw, 30.0));
                    y += 30.0 + 12.0;
                    layout.emitter_falloff = Some(Rect::new(x0, y, fullw, 30.0));
                    y += 30.0 + 12.0;
                    if emitter.use_temperature {
                        layout.emitter_temperature = Some(Rect::new(x0, y, fullw, 30.0));
                        y += 30.0 + 12.0;
                    } else if !emitter.use_material_color {
                        let mut color = [zero; 3];
                        for rect in &mut color {
                            *rect = Rect::new(x0 + 28.0, y, fullw - 28.0, 24.0);
                            y += 31.0;
                        }
                        layout.emitter_color = Some(color);
                        y += 2.0;
                    }
                }
            }
            DffSection::FaceTexture => {
                layout.tex_from_txd = Some(Rect::new(x0, y, halfw, DFF_BTN_H));
                layout.tex_browse = Some(Rect::new(x2, y, halfw, DFF_BTN_H));
                y += DFF_BTN_H + gap;
            }
            DffSection::MaterialAnim => {
                layout.anim_assign = Some(Rect::new(x0, y, halfw, DFF_BTN_H));
                layout.anim_clear = Some(Rect::new(x2, y, halfw, DFF_BTN_H));
                y += DFF_BTN_H + 6.0;
                let bw = ((fullw - 18.0) / 4.0).min(64.0);
                let mut motion = [zero; 4];
                for (i, slot) in motion.iter_mut().enumerate() {
                    *slot = Rect::new(x0 + i as f32 * (bw + 6.0), y, bw, DFF_SMALL_BTN_H);
                }
                layout.anim_motion = Some(motion);
                y += DFF_SMALL_BTN_H + gap;
            }
            DffSection::UvTools => {
                let bw = ((fullw - 18.0) / 4.0).min(64.0);
                let mut nudge = [zero; 4];
                for (i, slot) in nudge.iter_mut().enumerate() {
                    *slot = Rect::new(x0 + i as f32 * (bw + 6.0), y, bw, DFF_SMALL_BTN_H);
                }
                layout.uv_nudge = Some(nudge);
                y += DFF_SMALL_BTN_H + 6.0;
                layout.uv_scale = Some([
                    Rect::new(x0, y, halfw, DFF_BTN_H),
                    Rect::new(x2, y, halfw, DFF_BTN_H),
                ]);
                y += DFF_BTN_H + 6.0;
                layout.uv_rotate = Some([
                    Rect::new(x0, y, halfw, DFF_BTN_H),
                    Rect::new(x2, y, halfw, DFF_BTN_H),
                ]);
                y += DFF_BTN_H + 6.0;
                layout.uv_unwrap_face = Some(Rect::new(x0, y, halfw, DFF_BTN_H));
                layout.uv_unwrap_material = Some(Rect::new(x2, y, halfw, DFF_BTN_H));
                y += DFF_BTN_H + gap;
            }
            DffSection::Mesh => {
                layout.make_face = Some(Rect::new(x0, y, halfw, DFF_BTN_H));
                layout.delete_face = Some(Rect::new(x2, y, halfw, DFF_BTN_H));
                y += DFF_BTN_H + 6.0;
                layout.delete_vertex = Some(Rect::new(x0, y, halfw, DFF_BTN_H));
                layout.delete_material_faces = Some(Rect::new(x2, y, halfw, DFF_BTN_H));
                y += DFF_BTN_H + 6.0;
                layout.extrude_selection = Some(Rect::new(x0, y, fullw, DFF_BTN_H));
                y += DFF_BTN_H + 6.0;
                layout.merge_selected = Some(Rect::new(x0, y, halfw, DFF_BTN_H));
                layout.merge_distance = Some(Rect::new(x2, y, halfw, DFF_BTN_H));
                y += DFF_BTN_H + 6.0;
                layout.subdivide = Some(Rect::new(x0, y, halfw, DFF_BTN_H));
                layout.duplicate_faces = Some(Rect::new(x2, y, halfw, DFF_BTN_H));
                y += DFF_BTN_H + 6.0;
                layout.duplicate_material = Some(Rect::new(x0, y, fullw, DFF_BTN_H));
                y += DFF_BTN_H + 6.0;
                layout.separate_faces = Some(Rect::new(x0, y, fullw, DFF_BTN_H));
                y += DFF_BTN_H + 6.0;
                layout.pivot_to_selection = Some(Rect::new(x0, y, halfw, DFF_BTN_H));
                layout.pivot_to_bounds = Some(Rect::new(x2, y, halfw, DFF_BTN_H));
                y += DFF_BTN_H + gap;
            }
            DffSection::Cutter => {
                layout.cutter_add = Some(Rect::new(x0, y, halfw, DFF_BTN_H));
                layout.cutter_clear = Some(Rect::new(x2, y, halfw, DFF_BTN_H));
                y += DFF_BTN_H + 6.0;
                let bw = (fullw - 30.0) / 6.0;
                let mut resize = [zero; 6];
                for (i, slot) in resize.iter_mut().enumerate() {
                    *slot = Rect::new(x0 + i as f32 * (bw + 6.0), y, bw, DFF_SMALL_BTN_H);
                }
                layout.cutter_resize = Some(resize);
                y += DFF_SMALL_BTN_H + gap;
            }
            DffSection::Lod => {
                layout.generate_lod = Some(Rect::new(x0, y, fullw, DFF_BTN_H));
                y += DFF_BTN_H + gap;
            }
        }
    }
    layout.content_height = (y + dff.panel_scroll) - (content.y + 4.0) + 8.0;
    layout
}

pub(crate) fn dff_panel_max_scroll(layout: &DffPanelLayout) -> f32 {
    (layout.content_height - layout.content.h).max(0.0)
}

pub(crate) fn editing_dff_2dfx_type_picker_rect() -> Rect {
    let right = editing_asset_rect();
    Rect::new(right.x + 18.0, right.y + 148.0, right.w - 36.0, 318.0)
}

pub(crate) fn editing_dff_2dfx_type_picker_search_rect() -> Rect {
    let popup = editing_dff_2dfx_type_picker_rect();
    Rect::new(popup.x + 12.0, popup.y + 48.0, popup.w - 24.0, 30.0)
}

pub(crate) fn editing_dff_2dfx_type_picker_close_rect() -> Rect {
    let popup = editing_dff_2dfx_type_picker_rect();
    Rect::new(popup.x + popup.w - 82.0, popup.y + 12.0, 68.0, 28.0)
}

pub(crate) fn editing_dff_2dfx_type_picker_list_rect() -> Rect {
    let popup = editing_dff_2dfx_type_picker_rect();
    Rect::new(
        popup.x + 12.0,
        popup.y + 88.0,
        popup.w - 24.0,
        popup.h - 100.0,
    )
}

pub(crate) fn editing_dff_2dfx_corona_preset_picker_rect() -> Rect {
    let right = editing_asset_rect();
    Rect::new(right.x + 18.0, right.y + 104.0, right.w - 36.0, 500.0)
}

pub(crate) fn editing_dff_2dfx_corona_preset_close_rect() -> Rect {
    let popup = editing_dff_2dfx_corona_preset_picker_rect();
    Rect::new(popup.x + popup.w - 82.0, popup.y + 12.0, 68.0, 28.0)
}

pub(crate) fn editing_dff_2dfx_corona_preset_list_rect() -> Rect {
    let popup = editing_dff_2dfx_corona_preset_picker_rect();
    Rect::new(
        popup.x + 12.0,
        popup.y + 58.0,
        popup.w - 24.0,
        popup.h - 70.0,
    )
}

pub(crate) fn editing_dff_2dfx_payload_editor_rect() -> Rect {
    let right = editing_asset_rect();
    Rect::new(right.x + 18.0, right.y + 120.0, right.w - 36.0, 430.0)
}

pub(crate) fn editing_dff_2dfx_payload_close_rect() -> Rect {
    let popup = editing_dff_2dfx_payload_editor_rect();
    Rect::new(popup.x + popup.w - 82.0, popup.y + 12.0, 68.0, 28.0)
}

pub(crate) fn editing_dff_2dfx_payload_apply_rect() -> Rect {
    let popup = editing_dff_2dfx_payload_editor_rect();
    Rect::new(popup.x + 12.0, popup.y + popup.h - 40.0, 116.0, 28.0)
}

pub(crate) fn editing_dff_2dfx_payload_text_rect() -> Rect {
    let popup = editing_dff_2dfx_payload_editor_rect();
    Rect::new(
        popup.x + 12.0,
        popup.y + 132.0,
        popup.w - 24.0,
        popup.h - 178.0,
    )
}

pub(crate) fn editing_dff_2dfx_light_color_bar_rect(channel: usize) -> Rect {
    let popup = editing_dff_2dfx_payload_editor_rect();
    Rect::new(
        popup.x + 58.0,
        popup.y + 62.0 + channel as f32 * 22.0,
        popup.w - 164.0,
        15.0,
    )
}

pub(crate) fn editing_dff_2dfx_light_color_swatch_rect() -> Rect {
    let popup = editing_dff_2dfx_payload_editor_rect();
    Rect::new(popup.x + popup.w - 82.0, popup.y + 62.0, 46.0, 59.0)
}

pub(crate) fn editing_dff_2dfx_particle_picker_rect() -> Rect {
    let popup = editing_dff_2dfx_payload_editor_rect();
    Rect::new(
        popup.x + 12.0,
        popup.y + popup.h - 180.0,
        popup.w - 24.0,
        134.0,
    )
}

pub(crate) fn editing_select_vertex_mode_rect() -> Rect {
    let panel = editing_panel_rect();
    let btn_w = 92.0;
    let gap = 10.0;
    let total = btn_w * 3.0 + gap * 2.0;
    let start_x = panel.x + (panel.w - total) * 0.5;
    Rect::new(start_x, panel.y + 4.0, btn_w, 28.0)
}

pub(crate) fn editing_select_edge_mode_rect() -> Rect {
    let vertex = editing_select_vertex_mode_rect();
    Rect::new(vertex.x + vertex.w + 10.0, vertex.y, vertex.w, vertex.h)
}

pub(crate) fn editing_select_face_mode_rect() -> Rect {
    let edge = editing_select_edge_mode_rect();
    Rect::new(edge.x + edge.w + 10.0, edge.y, edge.w, edge.h)
}

pub(crate) fn editing_dff_uv_anim_picker_rect() -> Rect {
    let right = editing_asset_rect();
    Rect::new(right.x + 18.0, right.y + 148.0, right.w - 36.0, 320.0)
}

pub(crate) fn editing_dff_uv_anim_picker_search_rect() -> Rect {
    let popup = editing_dff_uv_anim_picker_rect();
    Rect::new(popup.x + 12.0, popup.y + 48.0, popup.w - 24.0, 30.0)
}

pub(crate) fn editing_dff_uv_anim_picker_close_rect() -> Rect {
    let popup = editing_dff_uv_anim_picker_rect();
    Rect::new(popup.x + popup.w - 82.0, popup.y + 12.0, 68.0, 28.0)
}

pub(crate) fn editing_dff_uv_anim_picker_apply_rect() -> Rect {
    let popup = editing_dff_uv_anim_picker_rect();
    Rect::new(popup.x + 12.0, popup.y + popup.h - 40.0, 130.0, 28.0)
}

pub(crate) fn editing_dff_uv_anim_picker_gif_rect() -> Rect {
    let popup = editing_dff_uv_anim_picker_rect();
    Rect::new(popup.x + 150.0, popup.y + popup.h - 40.0, 150.0, 28.0)
}

pub(crate) fn editing_dff_uv_anim_picker_list_rect() -> Rect {
    let popup = editing_dff_uv_anim_picker_rect();
    Rect::new(
        popup.x + 12.0,
        popup.y + 88.0,
        popup.w - 24.0,
        popup.h - 140.0,
    )
}

pub(crate) fn editing_dff_texture_picker_rect() -> Rect {
    let right = editing_asset_rect();
    let w = right.w - 28.0;
    let h = (right.h - 120.0).max(160.0);
    Rect::new(right.x + 14.0, right.y + 60.0, w, h)
}

pub(crate) fn editing_dff_texture_picker_close_rect() -> Rect {
    let popup = editing_dff_texture_picker_rect();
    Rect::new(popup.x + popup.w - 84.0, popup.y + 8.0, 76.0, 26.0)
}

pub(crate) const DFF_TEXTURE_PICKER_ROW_H: f32 = 30.0;

pub(crate) fn editing_dff_texture_picker_list_rect() -> Rect {
    let popup = editing_dff_texture_picker_rect();
    Rect::new(
        popup.x + 8.0,
        popup.y + 44.0,
        popup.w - 16.0,
        popup.h - 52.0,
    )
}

pub(crate) const COL_SECTION_COUNT: usize = 5;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ColSection {
    Primitives = 0,
    Faces = 1,
    Surface = 2,
    Vertex = 3,
    Mesh = 4,
}

pub(crate) const COL_SECTIONS: [ColSection; COL_SECTION_COUNT] = [
    ColSection::Primitives,
    ColSection::Faces,
    ColSection::Surface,
    ColSection::Vertex,
    ColSection::Mesh,
];

impl ColSection {
    pub(crate) fn title(self) -> &'static str {
        match self {
            ColSection::Primitives => "Primitives",
            ColSection::Faces => "Faces",
            ColSection::Surface => "Surface",
            ColSection::Vertex => "Vertex",
            ColSection::Mesh => "Mesh Tools",
        }
    }
}

pub(crate) fn col_default_collapsed() -> [bool; COL_SECTION_COUNT] {
    [false; COL_SECTION_COUNT]
}

pub(crate) const COL_FACE_ROW_H: f32 = 28.0;
pub(crate) const COL_PRIM_ROW_H: f32 = 28.0;
pub(crate) const COL_FACE_VISIBLE_MAX: usize = 12;
pub(crate) const COL_PRIM_VISIBLE_MAX: usize = 6;

pub(crate) struct ColPanelLayout {
    pub(crate) content: Rect,
    pub(crate) content_height: f32,
    pub(crate) headers: [Rect; COL_SECTION_COUNT],
    pub(crate) primitive_list: Option<Rect>,
    pub(crate) primitive_rows: Vec<Rect>,
    pub(crate) primitive_visible: usize,
    pub(crate) add_sphere: Option<Rect>,
    pub(crate) add_box: Option<Rect>,
    pub(crate) add_capsule: Option<Rect>,
    pub(crate) duplicate_primitive: Option<Rect>,
    pub(crate) box_pick_toggle: Option<Rect>,
    pub(crate) capsule_edges_toggle: Option<Rect>,
    pub(crate) face_list: Option<Rect>,
    pub(crate) face_visible: usize,
    pub(crate) material: Option<Rect>,
    pub(crate) light: Option<Rect>,
    pub(crate) select_same_material: Option<Rect>,
    pub(crate) prim_size: Option<[Rect; 3]>,
    pub(crate) prim_rotation: Option<[Rect; 3]>,
    pub(crate) vertex: Option<[Rect; 3]>,
    pub(crate) delete_face: Option<Rect>,
    pub(crate) make_face: Option<Rect>,
    pub(crate) flip_face: Option<Rect>,
    pub(crate) merge_distance: Option<Rect>,
    pub(crate) optimize: Option<Rect>,
    pub(crate) cleanup: Option<Rect>,
    pub(crate) validate: Option<Rect>,
    pub(crate) stage: Rect,
}

pub(crate) const COL_PANEL_HEADER_H: f32 = 226.0;

fn editing_col_header_action_rect(slot: usize) -> Rect {
    let right = editing_asset_rect();
    let gap = 7.0;
    let width = (right.w - 32.0 - gap * 4.0) / 5.0;
    Rect::new(
        right.x + 16.0 + slot as f32 * (width + gap),
        right.y + 150.0,
        width,
        26.0,
    )
}

pub(crate) fn editing_col_overlay_toggle_rect() -> Rect {
    editing_col_header_action_rect(0)
}

pub(crate) fn editing_col_overlay_pick_rect() -> Rect {
    editing_col_header_action_rect(1)
}

pub(crate) fn editing_col_overlay_match_rect() -> Rect {
    editing_col_header_action_rect(2)
}

pub(crate) fn editing_col_overlay_clear_rect() -> Rect {
    editing_col_header_action_rect(3)
}

pub(crate) fn editing_col_safe_rect() -> Rect {
    editing_col_header_action_rect(4)
}

pub(crate) fn editing_col_generation_preset_rect() -> Rect {
    let right = editing_asset_rect();
    let gap = 7.0;
    let width = (right.w - 32.0 - gap * 2.0) / 3.0;
    Rect::new(right.x + 16.0, right.y + 182.0, width, 26.0)
}

pub(crate) fn editing_col_shadow_layer_rect() -> Rect {
    let right = editing_asset_rect();
    let gap = 7.0;
    let width = (right.w - 32.0 - gap * 2.0) / 3.0;
    Rect::new(right.x + 16.0 + width + gap, right.y + 182.0, width, 26.0)
}

pub(crate) fn editing_col_generate_shadow_rect() -> Rect {
    let right = editing_asset_rect();
    let layer = editing_col_shadow_layer_rect();
    Rect::new(
        layer.x + layer.w + 7.0,
        layer.y,
        right.x + right.w - 16.0 - layer.x - layer.w - 7.0,
        layer.h,
    )
}

fn col_primitive_selections(col: &EditingColState) -> Vec<CollisionPrimitiveSelection> {
    let capsule_spheres = col
        .capsules
        .iter()
        .flat_map(|capsule| capsule.sphere_indices)
        .collect::<BTreeSet<_>>();
    let mut selections = col
        .mesh
        .spheres
        .iter()
        .enumerate()
        .filter_map(|(index, _)| {
            (!capsule_spheres.contains(&index)).then_some(CollisionPrimitiveSelection {
                kind: CollisionPrimitiveKind::Sphere,
                index,
            })
        })
        .collect::<Vec<_>>();
    selections.extend(col.mesh.boxes.iter().enumerate().map(|(index, _)| {
        CollisionPrimitiveSelection {
            kind: CollisionPrimitiveKind::Box,
            index,
        }
    }));
    selections.extend(col.cuboids.iter().enumerate().map(|(index, _)| {
        CollisionPrimitiveSelection {
            kind: CollisionPrimitiveKind::Cuboid,
            index,
        }
    }));
    selections.extend(col.capsules.iter().enumerate().map(|(index, _)| {
        CollisionPrimitiveSelection {
            kind: CollisionPrimitiveKind::Capsule,
            index,
        }
    }));
    selections
}

fn col_generated_primitive_for_face(
    col: &EditingColState,
    face: usize,
) -> Option<CollisionPrimitiveSelection> {
    if col.editing_shadow {
        return None;
    }
    if let Some(index) = col.capsules.iter().position(|capsule| {
        face >= capsule.face_start && face < capsule.face_start + capsule.face_count
    }) {
        return Some(CollisionPrimitiveSelection {
            kind: CollisionPrimitiveKind::Capsule,
            index,
        });
    }
    col.cuboids
        .iter()
        .position(|cuboid| {
            face >= cuboid.face_start && face < cuboid.face_start + cuboid.face_count
        })
        .map(|index| CollisionPrimitiveSelection {
            kind: CollisionPrimitiveKind::Cuboid,
            index,
        })
}

fn col_generated_primitive_for_vertex(
    col: &EditingColState,
    vertex: usize,
) -> Option<CollisionPrimitiveSelection> {
    if col.editing_shadow {
        return None;
    }
    if let Some(index) = col.capsules.iter().position(|capsule| {
        vertex >= capsule.vertex_start && vertex < capsule.vertex_start + capsule.vertex_count
    }) {
        return Some(CollisionPrimitiveSelection {
            kind: CollisionPrimitiveKind::Capsule,
            index,
        });
    }
    col.cuboids
        .iter()
        .position(|cuboid| {
            vertex >= cuboid.vertex_start && vertex < cuboid.vertex_start + cuboid.vertex_count
        })
        .map(|index| CollisionPrimitiveSelection {
            kind: CollisionPrimitiveKind::Cuboid,
            index,
        })
}

fn col_vertices_touch_generated_primitive(
    col: &EditingColState,
    vertices: impl IntoIterator<Item = usize>,
) -> bool {
    vertices
        .into_iter()
        .any(|vertex| col_generated_primitive_for_vertex(col, vertex).is_some())
}

fn col_faces_touch_generated_primitive(
    col: &EditingColState,
    faces: impl IntoIterator<Item = usize>,
) -> bool {
    faces
        .into_iter()
        .any(|face| col_generated_primitive_for_face(col, face).is_some())
}

fn col_regular_topology_has_editable_primitives(col: &EditingColState) -> bool {
    !col.editing_shadow && (!col.capsules.is_empty() || !col.cuboids.is_empty())
}

fn refresh_editing_col_bounds(col: &mut EditingColState) {
    col.mesh.bounds = if col.editing_shadow {
        bounds_from_vertices(&col.mesh.vertices)
    } else {
        collision_mesh_bounds(&col.mesh.vertices, &col.mesh.spheres, &col.mesh.boxes)
    };
}

pub(crate) fn col_panel_layout(col: &EditingColState) -> ColPanelLayout {
    let panel = editing_asset_rect();
    let x0 = dff_ctl_x0(panel);
    let fullw = dff_ctl_fullw(panel);
    let halfw = dff_ctl_halfw(panel);
    let x2 = dff_ctl_x2(panel);
    let footer_h = DFF_BTN_H + 24.0;
    let content = Rect::new(
        panel.x + 1.0,
        panel.y + COL_PANEL_HEADER_H,
        panel.w - 2.0,
        (panel.h - COL_PANEL_HEADER_H - footer_h).max(40.0),
    );
    let gap = 10.0;
    let label_gap = 22.0;
    let input_h = 30.0;
    let zero = Rect::new(0.0, 0.0, 0.0, 0.0);
    let mut layout = ColPanelLayout {
        content,
        content_height: 0.0,
        headers: [zero; COL_SECTION_COUNT],
        primitive_list: None,
        primitive_rows: Vec::new(),
        primitive_visible: 0,
        add_sphere: None,
        add_box: None,
        add_capsule: None,
        duplicate_primitive: None,
        box_pick_toggle: None,
        capsule_edges_toggle: None,
        face_list: None,
        face_visible: 0,
        material: None,
        light: None,
        select_same_material: None,
        prim_size: None,
        prim_rotation: None,
        vertex: None,
        delete_face: None,
        make_face: None,
        flip_face: None,
        merge_distance: None,
        optimize: None,
        cleanup: None,
        validate: None,
        stage: Rect::new(x0, panel.y + panel.h - DFF_BTN_H - 12.0, fullw, DFF_BTN_H),
    };
    let mut y = content.y + 4.0 - col.panel_scroll;
    for section in COL_SECTIONS {
        let idx = section as usize;
        layout.headers[idx] = Rect::new(x0, y, fullw, DFF_SEC_HEADER_H);
        y += DFF_SEC_HEADER_H + 6.0;
        if col.panel_collapsed[idx] {
            y += gap - 6.0;
            continue;
        }
        match section {
            ColSection::Primitives => {
                if col.editing_shadow {
                    y += gap;
                    continue;
                }
                let total = col_primitive_selections(col).len();
                let count = total.min(COL_PRIM_VISIBLE_MAX);
                layout.primitive_visible = count;
                if count > 0 {
                    layout.primitive_list =
                        Some(Rect::new(x0, y, fullw, count as f32 * COL_PRIM_ROW_H));
                }
                for _ in 0..count {
                    layout
                        .primitive_rows
                        .push(Rect::new(x0, y, fullw, COL_PRIM_ROW_H - 3.0));
                    y += COL_PRIM_ROW_H;
                }
                if count > 0 {
                    y += 4.0;
                }
                layout.add_sphere = Some(Rect::new(x0, y, halfw, DFF_BTN_H));
                layout.add_box = Some(Rect::new(x2, y, halfw, DFF_BTN_H));
                y += DFF_BTN_H + 6.0;
                layout.add_capsule = Some(Rect::new(x0, y, fullw, DFF_BTN_H));
                y += DFF_BTN_H + 6.0;
                layout.duplicate_primitive = Some(Rect::new(x0, y, halfw, DFF_BTN_H));
                if col
                    .selected_primitive
                    .is_some_and(|selected| selected.kind == CollisionPrimitiveKind::Capsule)
                {
                    layout.capsule_edges_toggle = Some(Rect::new(x2, y, halfw, DFF_BTN_H));
                } else {
                    layout.box_pick_toggle = Some(Rect::new(x2, y, halfw, DFF_BTN_H));
                }
                y += DFF_BTN_H + gap;
            }
            ColSection::Faces => {
                let total = col.mesh.faces.len().max(1);
                let visible = total.min(COL_FACE_VISIBLE_MAX);
                layout.face_visible = visible;
                layout.face_list = Some(Rect::new(x0, y, fullw, visible as f32 * COL_FACE_ROW_H));
                y += visible as f32 * COL_FACE_ROW_H + gap;
            }
            ColSection::Surface => {
                y += label_gap;
                layout.material = Some(Rect::new(x0, y, halfw, input_h));
                layout.light = Some(Rect::new(x2, y, halfw, input_h));
                y += input_h + 6.0;
                if col.selected_primitive.is_none()
                    && col.selected_face < col.mesh.faces.len()
                    && matches!(
                        col.select_mode,
                        EditingSelectMode::Vertex | EditingSelectMode::Face
                    )
                {
                    layout.select_same_material = Some(Rect::new(x0, y, fullw, DFF_BTN_H));
                    y += DFF_BTN_H + 6.0;
                }
                if col.selected_primitive.is_some() {
                    y += label_gap;
                    let w = (fullw - 20.0) / 3.0;
                    let mut size = [zero; 3];
                    for (i, slot) in size.iter_mut().enumerate() {
                        *slot = Rect::new(x0 + i as f32 * (w + 10.0), y, w, input_h);
                    }
                    layout.prim_size = Some(size);
                    y += input_h + 6.0;
                    if col.selected_primitive.is_some_and(|selected| {
                        matches!(
                            selected.kind,
                            CollisionPrimitiveKind::Box | CollisionPrimitiveKind::Cuboid
                        )
                    }) {
                        y += label_gap;
                        let mut rotation = [zero; 3];
                        for (i, slot) in rotation.iter_mut().enumerate() {
                            *slot = Rect::new(x0 + i as f32 * (w + 10.0), y, w, input_h);
                        }
                        layout.prim_rotation = Some(rotation);
                        y += input_h + 6.0;
                    }
                }
                y += gap - 6.0;
            }
            ColSection::Vertex => {
                y += label_gap;
                let w = (fullw - 20.0) / 3.0;
                let mut vertex = [zero; 3];
                for (i, slot) in vertex.iter_mut().enumerate() {
                    *slot = Rect::new(x0 + i as f32 * (w + 10.0), y, w, input_h);
                }
                layout.vertex = Some(vertex);
                y += input_h + gap;
            }
            ColSection::Mesh => {
                layout.delete_face = Some(Rect::new(x0, y, halfw, DFF_BTN_H));
                layout.make_face = Some(Rect::new(x2, y, halfw, DFF_BTN_H));
                y += DFF_BTN_H + 6.0;
                layout.flip_face = Some(Rect::new(x0, y, halfw, DFF_BTN_H));
                layout.merge_distance = Some(Rect::new(x2, y, halfw, DFF_BTN_H));
                y += DFF_BTN_H + 6.0;
                let w = (fullw - 12.0) / 3.0;
                layout.optimize = Some(Rect::new(x0, y, w, DFF_BTN_H));
                layout.cleanup = Some(Rect::new(x0 + w + 6.0, y, w, DFF_BTN_H));
                layout.validate = Some(Rect::new(x0 + (w + 6.0) * 2.0, y, w, DFF_BTN_H));
                y += DFF_BTN_H + gap;
            }
        }
    }
    layout.content_height = (y + col.panel_scroll) - (content.y + 4.0) + 8.0;
    // Input fields are edited through global inspector hit-testing that is not
    // clipped to the panel, so hide any field row that scrolled out of view.
    let fully_visible =
        |rect: &Rect| rect.y >= content.y && rect.y + rect.h <= content.y + content.h;
    if layout.material.as_ref().is_some_and(|r| !fully_visible(r)) {
        layout.material = None;
        layout.light = None;
    }
    if layout
        .prim_size
        .as_ref()
        .is_some_and(|rects| !fully_visible(&rects[0]))
    {
        layout.prim_size = None;
    }
    if layout
        .prim_rotation
        .as_ref()
        .is_some_and(|rects| !fully_visible(&rects[0]))
    {
        layout.prim_rotation = None;
    }
    if layout
        .vertex
        .as_ref()
        .is_some_and(|rects| !fully_visible(&rects[0]))
    {
        layout.vertex = None;
    }
    layout
}

pub(crate) fn col_panel_max_scroll(layout: &ColPanelLayout) -> f32 {
    (layout.content_height - layout.content.h).max(0.0)
}

pub(crate) fn scroll_editing_col_faces(col: &mut EditingColState, wheel_y: f32) -> bool {
    let visible = col.mesh.faces.len().max(1).min(COL_FACE_VISIBLE_MAX);
    let max_scroll = col.mesh.faces.len().saturating_sub(visible) as f32;
    let next = (col.face_scroll - wheel_y * 3.0).clamp(0.0, max_scroll);
    let changed = (next - col.face_scroll).abs() > f32::EPSILON;
    col.face_scroll = next;
    changed
}

pub(crate) fn scroll_editing_col_primitives(col: &mut EditingColState, wheel_y: f32) -> bool {
    let total = col_primitive_selections(col).len();
    let visible = total.min(COL_PRIM_VISIBLE_MAX);
    let max_scroll = total.saturating_sub(visible) as f32;
    let next = (col.primitive_scroll - wheel_y * 3.0).clamp(0.0, max_scroll);
    let changed = (next - col.primitive_scroll).abs() > f32::EPSILON;
    col.primitive_scroll = next;
    changed
}

fn editing_key(name: &str) -> String {
    lower(name)
}

pub(crate) fn frame_editing_camera(app: &mut AppState, bounds: Bounds) {
    let center = (bounds.min + bounds.max) * 0.5;
    let radius = (bounds.max - bounds.min).length().max(8.0);
    app.camera.pos = center + vec3(-radius * 1.55, -radius * 2.15, radius * 1.15);
    let dir = (center - app.camera.pos).normalize_or_zero();
    if dir.length_squared() > 0.0001 {
        app.camera.yaw = dir.y.atan2(dir.x);
        app.camera.pitch = dir.z.asin();
    }
    app.camera.looking = false;
    set_cursor_grab(false);
    show_mouse(true);
}

fn editing_entry_type(name: &str) -> &'static str {
    let key = lower(name);
    if key.ends_with(".txd") {
        "TXD"
    } else if key.ends_with(".dff") {
        "DFF"
    } else if key.ends_with(".col") {
        "COL"
    } else {
        "FILE"
    }
}

pub(crate) fn editing_dirty(app: &AppState) -> bool {
    !app.editing.modified_entries.is_empty()
        || !app.editing.deleted_entries.is_empty()
        || matches!(app.editing.asset.as_ref(), Some(EditingAsset::Dff(dff)) if dff.dirty)
        || matches!(app.editing.asset.as_ref(), Some(EditingAsset::Col(col)) if col.dirty)
}

fn editing_filtered_indices(app: &AppState) -> Vec<usize> {
    let query = lower(app.editing.search.trim());
    app.editing
        .rows
        .iter()
        .enumerate()
        .filter(|(_, row)| {
            !app.editing
                .deleted_entries
                .contains(&editing_key(&row.entry.name))
                && (query.is_empty() || lower(&row.entry.name).contains(&query))
        })
        .map(|(idx, _)| idx)
        .collect()
}

fn editing_txd_filtered_indices(txd: &EditingTxdState) -> Vec<usize> {
    let query = lower(txd.search.trim());
    txd.textures
        .iter()
        .enumerate()
        .filter(|(_, texture)| query.is_empty() || lower(&texture.name).contains(&query))
        .map(|(index, _)| index)
        .collect()
}

fn editing_txd_scroll_value(
    current: f32,
    filtered_len: usize,
    list_height: f32,
    wheel_y: f32,
) -> f32 {
    let visible = (list_height / 32.0).floor().max(1.0) as usize;
    let max_scroll = filtered_len.saturating_sub(visible) as f32;
    (current - wheel_y * 3.0).clamp(0.0, max_scroll)
}

pub(crate) fn scroll_editing_txd_list(app: &mut AppState, mouse: Vec2, wheel_y: f32) -> bool {
    if wheel_y.abs() <= f32::EPSILON || !editing_txd_list_rect().contains(mouse) {
        return false;
    }
    let Some(EditingAsset::Txd(txd)) = app.editing.asset.as_mut() else {
        return false;
    };
    let filtered_len = editing_txd_filtered_indices(txd).len();
    txd.scroll =
        editing_txd_scroll_value(txd.scroll, filtered_len, editing_txd_list_rect().h, wheel_y);
    true
}

fn editing_selected_index(app: &AppState) -> Option<usize> {
    let indices = editing_filtered_indices(app);
    indices
        .get(
            app.editing
                .selected_row
                .min(indices.len().saturating_sub(1)),
        )
        .copied()
}

fn editing_selected_row(app: &AppState) -> Option<&EditingImgRow> {
    editing_selected_index(app).and_then(|idx| app.editing.rows.get(idx))
}

fn checked_editing_entry_bytes(row: &EditingImgRow, bytes: Vec<u8>) -> Result<Vec<u8>, String> {
    let declared = row.entry.size as usize;
    if bytes.len() != declared {
        return Err(format!(
            "Could not read IMG entry {} completely: expected {declared} bytes, got {}",
            row.entry.name,
            bytes.len()
        ));
    }
    bytes
        .get(..row.logical_size.min(bytes.len()))
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("IMG entry {} has invalid bounds", row.entry.name))
}

fn editing_entry_bytes(app: &AppState, row: &EditingImgRow) -> Result<Vec<u8>, String> {
    let key = editing_key(&row.entry.name);
    if let Some(bytes) = app.editing.modified_entries.get(&key) {
        return Ok(bytes.clone());
    }
    checked_editing_entry_bytes(row, read_img_entry(&row.entry))
}

fn txd_entries_from_bytes(name: &str, bytes: &[u8]) -> Vec<TextureArchiveEntry> {
    let mut index = TxdTextureIndex::new();
    let txd_name = asset_key(name, ".txd");
    index_one_txd(
        bytes,
        0,
        bytes.len(),
        Path::new("<editing>"),
        &txd_name,
        &mut index,
    );
    let mut entries = Vec::new();
    for (texture_name, textures) in &index {
        for texture in textures {
            if !texture.txd_name.eq_ignore_ascii_case(&txd_name) {
                continue;
            }
            entries.push(TextureArchiveEntry {
                name: texture_name.clone(),
                width: texture.width,
                height: texture.height,
                format: texture.format,
                fingerprint: Some(texture.content_fingerprint),
                thumbnail: txd_texture_thumbnail_from_bytes(texture, bytes),
            });
        }
    }
    entries.sort_by(|a, b| lower(&a.name).cmp(&lower(&b.name)));
    entries
}

fn txd_texture_thumbnail_from_bytes(texture: &TxdTexture, bytes: &[u8]) -> Option<Texture2D> {
    let (width, height, rgba) = decode_txd_texture_from_bytes(texture, bytes)?;
    let thumb = Texture2D::from_rgba8(width as u16, height as u16, &rgba);
    thumb.set_filter(FilterMode::Nearest);
    Some(thumb)
}

fn txd_assignment_key(value: &str) -> String {
    let trimmed = value.trim();
    let lowered = lower(trimmed);
    if let Some(stem) = lowered
        .strip_suffix(".txd")
        .or_else(|| lowered.strip_suffix(".dff"))
    {
        asset_key(stem, ".txd")
    } else {
        asset_key(trimmed, ".txd")
    }
}

fn resolve_editing_dff_txd_context(app: &AppState, dff_name: &str) -> (Option<String>, String) {
    let dff_key = asset_key(dff_name, ".dff");
    for def in app.definitions.values() {
        let def_dff = def
            .attrs
            .get("dff")
            .or_else(|| def.attrs.get("model"))
            .map(String::as_str)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(&def.id);
        if asset_key(def_dff, ".dff") != dff_key && asset_key(&def.id, ".dff") != dff_key {
            continue;
        }
        if let Some(txd) = definition_txd_name_from_attrs(def) {
            let txd_key = txd_assignment_key(txd);
            return (Some(txd_key.clone()), format!("Definition TXD: {txd_key}"));
        }
    }

    let same_name = asset_key(dff_name, ".txd");
    if app
        .editing
        .rows
        .iter()
        .any(|row| asset_key(&row.entry.name, ".txd") == same_name)
    {
        return (
            Some(same_name.clone()),
            format!("Same-name TXD: {same_name}"),
        );
    }

    (None, "Texture pool fallback".to_string())
}

fn resolve_editing_col_dff_overlay_name(app: &AppState, col_name: &str) -> Option<String> {
    let col_key = asset_key(col_name, ".col");
    for def in app.definitions.values() {
        let def_col = def
            .attrs
            .get("col")
            .map(String::as_str)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(&def.id);
        if asset_key(def_col, ".col") != col_key {
            continue;
        }
        let def_dff = def
            .attrs
            .get("dff")
            .or_else(|| def.attrs.get("model"))
            .map(String::as_str)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(&def.id);
        return Some(asset_key(def_dff, ".dff"));
    }
    let dff_key = asset_key(col_name, ".dff");
    find_dff_entry_for_app(app, &dff_key).map(|entry| asset_key(&entry.name, ".dff"))
}

fn editing_dff_material_thumbnails(
    app: &AppState,
    raw: &RawMesh,
    txd_context: Option<&str>,
) -> Vec<Option<Texture2D>> {
    let txd_key = txd_context.map(|txd| asset_key(txd, ".txd"));
    raw.material_textures
        .iter()
        .map(|texture| {
            let entries = app.txd_textures.get(&lower(texture.trim()))?;
            let chosen = if let Some(txd_key) = txd_key.as_ref() {
                entries
                    .iter()
                    .find(|entry| entry.txd_name.eq_ignore_ascii_case(txd_key))
            } else {
                entries.first()
            }?;
            txd_texture_thumbnail(chosen)
        })
        .collect()
}

fn build_editing_dff_preview(
    app: &mut AppState,
    raw: &RawMesh,
    txd_context: Option<&str>,
) -> Option<RenderMesh> {
    let ambient_lift = scene_ambient_lift_from_timecyc(&app.timecyc);
    let mut mesh = compile_render_mesh(
        raw.clone(),
        txd_context,
        None,
        None,
        &app.texture_files,
        &app.txd_textures,
        &mut app.textures,
        &mut app.textured_parts,
        app.options.textures,
        ambient_lift,
    )?;
    // The world renderer intentionally neutralizes tint on many textured
    // materials. Editing preview must instead show the authored material RGBA,
    // including fully transparent values, so changes are visible before save.
    for part in &mut mesh.parts {
        let material = raw
            .materials
            .get(part.material_index)
            .copied()
            .unwrap_or_else(default_dff_material);
        part.alpha = material.alpha.clamp(0.0, 1.0);
        part.material_color = material.color;
        part.material_ambient = material.ambient;
        let diffuse = material.diffuse.clamp(0.0, 4.0);
        if part.alpha < 0.98 {
            part.transparency = TransparencyMode::Blend;
        }
        for vertex in &mut part.cpu_vertices {
            vertex.day_color = V3 {
                x: vertex.base_day_color.x * material.color.x * diffuse,
                y: vertex.base_day_color.y * material.color.y * diffuse,
                z: vertex.base_day_color.z * material.color.z * diffuse,
            };
            vertex.night_color = V3 {
                x: vertex.base_night_color.x * material.color.x * diffuse,
                y: vertex.base_night_color.y * material.color.y * diffuse,
                z: vertex.base_night_color.z * material.color.z * diffuse,
            };
            vertex.color = vertex.day_color;
        }
        rebuild_render_part_list_with_lift(part, ambient_lift);
    }
    Some(mesh)
}

fn build_editing_dff_overlay_from_entry(
    app: &mut AppState,
    entry: &ImgEntry,
) -> Option<RenderMesh> {
    let bytes = read_img_entry(entry);
    let raw = parse_dff_mesh(&bytes);
    if raw.vertices.is_empty() {
        return None;
    }
    let (txd_context, _) = resolve_editing_dff_txd_context(app, &entry.name);
    build_editing_dff_preview(app, &raw, txd_context.as_deref())
}

fn editing_set_col_dff_overlay_from_entry(app: &mut AppState, entry: ImgEntry) -> bool {
    let name = asset_key(&entry.name, ".dff");
    let Some(mesh) = build_editing_dff_overlay_from_entry(app, &entry) else {
        app.status_message = format!("{} did not parse as a usable DFF overlay", entry.name);
        return false;
    };
    let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() else {
        return false;
    };
    col.dff_overlay = Some(mesh);
    col.dff_overlay_name = Some(name.clone());
    col.dff_overlay_visible = true;
    app.status_message = format!("Using {name} as COL DFF overlay");
    true
}

fn refresh_editing_col_dff_overlay(app: &mut AppState) {
    let Some(col_name) = app.editing.asset.as_ref().and_then(|asset| match asset {
        EditingAsset::Col(col) => Some(col.name.clone()),
        _ => None,
    }) else {
        return;
    };
    let Some(dff_name) = resolve_editing_col_dff_overlay_name(app, &col_name) else {
        app.status_message =
            "No matching DFF overlay found. Select a DFF row and click Pick.".to_string();
        return;
    };
    let Some(entry) = find_dff_entry_for_app(app, &dff_name) else {
        app.status_message =
            format!("No DFF overlay entry found for {dff_name}. Select a DFF row and click Pick.");
        return;
    };
    let _ = editing_set_col_dff_overlay_from_entry(app, entry);
}

fn pick_selected_row_as_col_dff_overlay(app: &mut AppState) {
    let Some(row) = editing_selected_row(app).cloned() else {
        app.status_message = "Select a DFF row to use as the COL overlay.".to_string();
        return;
    };
    if !lower(&row.entry.name).ends_with(".dff") {
        app.status_message = "Select a DFF row to use as the COL overlay.".to_string();
        return;
    }
    let _ = editing_set_col_dff_overlay_from_entry(app, row.entry);
}

pub(crate) fn refresh_editing_dff_preview(app: &mut AppState) {
    let Some((raw, txd_context)) = app.editing.asset.as_ref().and_then(|asset| match asset {
        EditingAsset::Dff(dff) => Some((dff.raw.clone(), dff.txd_context.clone())),
        _ => None,
    }) else {
        return;
    };
    let thumbnails = editing_dff_material_thumbnails(app, &raw, txd_context.as_deref());
    let preview_mesh = build_editing_dff_preview(app, &raw, txd_context.as_deref());
    if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
        dff.preview_mesh = preview_mesh;
        dff.material_thumbnails = thumbnails;
    }
}

pub(crate) fn refresh_editing_img_paths(app: &mut AppState) {
    let selected = app
        .editing
        .img_paths
        .get(app.editing.selected_img_path)
        .cloned();
    app.editing.img_paths = collect_resource_img_files(&app.root);
    app.editing.img_paths.sort();
    if let Some(selected) = selected {
        if let Some(idx) = app
            .editing
            .img_paths
            .iter()
            .position(|path| *path == selected)
        {
            app.editing.selected_img_path = idx;
        }
    }
    app.editing.selected_img_path = app
        .editing
        .selected_img_path
        .min(app.editing.img_paths.len().saturating_sub(1));
}

fn load_editing_img_rows(path: &Path) -> Result<Vec<EditingImgRow>, String> {
    let rows: Vec<EditingImgRow> = parse_img(path)
        .into_iter()
        .map(|entry| {
            let bytes = read_img_entry(&entry);
            let logical_size = replacement_entry_len(&entry.name, &bytes);
            EditingImgRow {
                entry,
                logical_size,
            }
        })
        .collect();
    if rows.is_empty()
        && fs::read(path)
            .map(|bytes| !bytes.starts_with(b"VER2"))
            .unwrap_or(true)
    {
        Err(format!(
            "{} is not a supported VER2 IMG archive",
            path.display()
        ))
    } else {
        Ok(rows)
    }
}

pub(crate) fn request_discard_editing_context(
    app: &mut AppState,
    action: ConfirmAction,
    target: &str,
) -> bool {
    if !editing_dirty(app) {
        return false;
    }
    app.confirm_dialog = Some(ConfirmDialog {
        action,
        title: "Unsaved Editing Changes".to_string(),
        body: format!("Discard unsaved Editing changes before opening {target}?"),
        detail:
            "This includes unstaged mesh edits and staged archive add, replace, or delete operations."
                .to_string(),
        primary_label: "Discard and Open".to_string(),
        secondary_label: None,
        secondary_action: None,
    });
    true
}

fn request_discard_active_editing_asset(
    app: &mut AppState,
    action: ConfirmAction,
    target: &str,
) -> bool {
    let active_name = match app.editing.asset.as_ref() {
        Some(EditingAsset::Dff(dff)) if dff.dirty => Some(dff.name.as_str()),
        Some(EditingAsset::Col(col)) if col.dirty => Some(col.name.as_str()),
        _ => None,
    };
    let Some(active_name) = active_name else {
        return false;
    };
    app.confirm_dialog = Some(ConfirmDialog {
        action,
        title: "Unstaged Editing Changes".to_string(),
        body: format!("Discard unstaged changes to {active_name} and open {target}?"),
        detail: "Other staged archive add, replace, and delete operations will be retained."
            .to_string(),
        primary_label: "Discard and Open".to_string(),
        secondary_label: None,
        secondary_action: None,
    });
    true
}

pub(crate) fn open_editing_img(app: &mut AppState, path: PathBuf) {
    let target = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("another IMG")
        .to_string();
    if request_discard_editing_context(app, ConfirmAction::OpenEditingFile(path.clone()), &target) {
        return;
    }
    open_editing_img_unchecked(app, path);
}

pub(crate) fn open_editing_img_unchecked(app: &mut AppState, path: PathBuf) {
    let rows = match load_editing_img_rows(&path) {
        Ok(rows) => rows,
        Err(err) => {
            app.editing.message = err;
            app.status_message = app.editing.message.clone();
            return;
        }
    };
    app.editing.img_path = Some(path.clone());
    app.editing.rows = rows;
    app.editing.selected_row = 0;
    app.editing.scroll = 0.0;
    app.editing.modified_entries.clear();
    app.editing.deleted_entries.clear();
    app.editing.added_entries.clear();
    app.editing.asset = None;
    clear_editing_history(app);
    app.editing.message = format!("Opened {}", path.display());
    app.status_message = app.editing.message.clone();
}

pub(crate) fn open_editing_file(app: &mut AppState, path: PathBuf) {
    let target = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("another asset")
        .to_string();
    if request_discard_editing_context(app, ConfirmAction::OpenEditingFile(path.clone()), &target) {
        return;
    }
    open_editing_file_unchecked(app, path);
}

pub(crate) fn open_editing_file_unchecked(app: &mut AppState, path: PathBuf) {
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .unwrap_or_default();
    if ext == "img" {
        open_editing_img_unchecked(app, path);
        return;
    }
    if !matches!(ext.as_str(), "txd" | "dff" | "col") {
        app.status_message = format!("{} is not an IMG, TXD, DFF, or COL file", path.display());
        return;
    }
    let Some(name) = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
    else {
        app.status_message = "Selected file has no usable name".to_string();
        return;
    };
    let Ok(bytes) = fs::read(&path) else {
        app.status_message = format!("Could not read {}", path.display());
        return;
    };
    let logical_size = replacement_entry_len(&name, &bytes);
    let entry = ImgEntry {
        img_path: path.clone(),
        name: name.clone(),
        offset: 0,
        size: bytes.len().min(u32::MAX as usize) as u32,
    };
    app.editing.img_path = None;
    app.editing.rows = vec![EditingImgRow {
        entry,
        logical_size,
    }];
    app.editing.selected_row = 0;
    app.editing.scroll = 0.0;
    app.editing.modified_entries.clear();
    app.editing.deleted_entries.clear();
    app.editing.added_entries.clear();
    app.editing
        .modified_entries
        .insert(editing_key(&name), bytes);
    app.editing.asset = None;
    clear_editing_history(app);
    app.editing.message = format!("Opened loose asset {}", path.display());
    app.status_message = app.editing.message.clone();
    editing_open_selected_asset(app);
}

pub(crate) fn open_selected_editing_img(app: &mut AppState) {
    refresh_editing_img_paths(app);
    if let Some(path) = app
        .editing
        .img_paths
        .get(app.editing.selected_img_path)
        .cloned()
    {
        open_editing_img(app, path);
    } else {
        app.status_message = "No IMG archives found under this resource".to_string();
    }
}

pub(crate) fn editing_open_selected_asset(app: &mut AppState) {
    let Some(row) = editing_selected_row(app).cloned() else {
        app.status_message = "Select an IMG entry first".to_string();
        return;
    };
    let active_asset_dirty = matches!(
        app.editing.asset.as_ref(),
        Some(EditingAsset::Dff(dff)) if dff.dirty
    ) || matches!(
        app.editing.asset.as_ref(),
        Some(EditingAsset::Col(col)) if col.dirty
    );
    if active_asset_dirty {
        let current = match app.editing.asset.as_ref() {
            Some(EditingAsset::Dff(dff)) => dff.name.as_str(),
            Some(EditingAsset::Col(col)) => col.name.as_str(),
            Some(EditingAsset::Txd(txd)) => txd.name.as_str(),
            None => "asset",
        };
        app.confirm_dialog = Some(ConfirmDialog {
            action: ConfirmAction::OpenEditingAsset(row),
            title: "Unstaged Editing Changes".to_string(),
            body: format!("Discard unstaged changes to {current}?"),
            detail: "Stage the current DFF/COL first to keep these edits.".to_string(),
            primary_label: "Discard and Open".to_string(),
            secondary_label: None,
            secondary_action: None,
        });
        return;
    }
    editing_open_asset_row(app, row);
}

pub(crate) fn editing_open_asset_row(app: &mut AppState, row: EditingImgRow) {
    let bytes = match editing_entry_bytes(app, &row) {
        Ok(bytes) => bytes,
        Err(err) => {
            app.status_message = err;
            return;
        }
    };
    let key = lower(&row.entry.name);
    if key.ends_with(".txd") {
        let textures = txd_entries_from_bytes(&row.entry.name, &bytes);
        clear_editing_history(app);
        app.editing.asset = Some(EditingAsset::Txd(EditingTxdState {
            name: row.entry.name.clone(),
            textures,
            selected: 0,
            scroll: 0.0,
            search: String::new(),
            search_cursor: 0,
            search_anchor: None,
            search_active: false,
            preview_texture: None,
            material_picker_open: false,
            material_picker_search: String::new(),
            material_picker_scroll: 0.0,
            material_picker_scope: CollisionMaterialAssignmentScope::ExactTxd,
        }));
        editing_update_txd_preview(app);
    } else if key.ends_with(".dff") {
        let raw = parse_dff_mesh(&bytes);
        if raw.vertices.is_empty() {
            app.status_message = format!("{} did not parse as a usable DFF", row.entry.name);
            return;
        }
        let bounds = bounds_from_vertices(&raw.vertices);
        let (txd_context, txd_source_label) = resolve_editing_dff_txd_context(app, &row.entry.name);
        let material_thumbnails =
            editing_dff_material_thumbnails(app, &raw, txd_context.as_deref());
        let preview_mesh = build_editing_dff_preview(app, &raw, txd_context.as_deref());
        let collision_material_picker_scope = if txd_context.is_some() {
            CollisionMaterialAssignmentScope::ExactTxd
        } else {
            CollisionMaterialAssignmentScope::GlobalName
        };
        clear_editing_history(app);
        app.editing.asset = Some(EditingAsset::Dff(EditingDffState {
            name: row.entry.name.clone(),
            raw,
            preview_mesh,
            txd_context,
            txd_source_label,
            material_thumbnails,
            selected_material: 0,
            selected_breakable_group: 0,
            fracture_preview_started_at: None,
            selected_face: None,
            selected_faces: BTreeSet::new(),
            selected_edges: BTreeSet::new(),
            select_mode: EditingSelectMode::Vertex,
            selected_vertex: None,
            selected_vertices: BTreeSet::new(),
            selected_2dfx: None,
            hovered_face: None,
            hovered_vertex: None,
            boolean_box: None,
            dirty: false,
            normalized_warning: true,
            normalized_rewrite_confirmed: false,
            material_scroll: 0.0,
            collision_material_picker_open: false,
            collision_material_picker_search: String::new(),
            collision_material_picker_scroll: 0.0,
            collision_material_picker_scope,
            texture_picker_open: false,
            texture_picker_edits_material: false,
            texture_picker_scroll: 0.0,
            uv_anim_picker_open: false,
            uv_anim_picker_search: String::new(),
            uv_anim_picker_scroll: 0.0,
            dff_2dfx_type_picker_open: false,
            dff_2dfx_type_picker_search: String::new(),
            dff_2dfx_type_picker_scroll: 0.0,
            dff_2dfx_corona_preset_picker_open: false,
            dff_2dfx_payload_editor_open: false,
            dff_2dfx_payload_hex: String::new(),
            dff_2dfx_payload_fields: Vec::new(),
            dff_2dfx_payload_active_field: None,
            dff_2dfx_payload_field_scroll: 0.0,
            dff_2dfx_particle_picker_scroll: 0.0,
            panel_scroll: 0.0,
            panel_collapsed: dff_default_collapsed(),
        }));
        frame_editing_camera(app, bounds);
    } else if key.ends_with(".col") {
        let mut local = row.entry.clone();
        local.offset = 0;
        local.size = bytes.len().min(u32::MAX as usize) as u32;
        if let Some((mut mesh, source_model)) = parse_col_mesh_with_identity(&bytes, &local) {
            let mut capsules = valid_capsules_for_mesh(
                &mut mesh,
                load_collision_capsules(&app.root, &row.entry.name),
            );
            let mut cuboids = valid_cuboids_for_mesh(
                &mut mesh,
                load_collision_cuboids(&app.root, &row.entry.name),
            );
            let _ = sync_generated_primitive_face_ranges(&mesh, &mut capsules, &mut cuboids);
            clear_editing_history(app);
            app.editing.asset = Some(EditingAsset::Col(EditingColState {
                name: row.entry.name.clone(),
                mesh,
                bytes,
                source_model,
                embedded_vehicle_dff: false,
                embedded_source_dff_bytes: None,
                dff_overlay: None,
                dff_overlay_name: None,
                dff_overlay_visible: true,
                editing_shadow: false,
                selected_face: 0,
                selected_faces: BTreeSet::new(),
                selected_edges: BTreeSet::new(),
                select_mode: EditingSelectMode::Vertex,
                face_scroll: 0.0,
                primitive_scroll: 0.0,
                selected_vertex: 0,
                selected_primitive: None,
                capsules,
                cuboids,
                box_pick_enabled: true,
                selected_vertices: BTreeSet::new(),
                hovered_face: None,
                hovered_vertex: None,
                dirty: false,
                panel_scroll: 0.0,
                panel_collapsed: col_default_collapsed(),
            }));
            refresh_editing_col_dff_overlay(app);
            if let Some(EditingAsset::Col(col)) = app.editing.asset.as_ref() {
                frame_editing_camera(app, col.mesh.bounds);
            }
            start_editing_col_cuboid_audit(app);
        } else {
            app.status_message = format!("{} did not parse as a supported COL", row.entry.name);
        }
    } else {
        app.status_message = format!("{} is not a supported editor type", row.entry.name);
    }
}

pub(crate) fn editing_update_txd_preview(app: &mut AppState) {
    let Some(EditingAsset::Txd(txd)) = app.editing.asset.as_mut() else {
        return;
    };
    txd.preview_texture = txd
        .textures
        .get(txd.selected)
        .and_then(|entry| entry.thumbnail.clone());
}

fn material_face_count(raw: &RawMesh, material: usize) -> usize {
    raw.triangles
        .iter()
        .filter(|tri| tri.material as usize == material)
        .count()
}

pub(crate) fn recalc_raw_normals(raw: &mut RawMesh) {
    let mut sums = vec![Vec3::ZERO; raw.vertices.len()];
    for tri in &raw.triangles {
        let a = tri.a as usize;
        let b = tri.b as usize;
        let c = tri.c as usize;
        if a >= raw.vertices.len() || b >= raw.vertices.len() || c >= raw.vertices.len() {
            continue;
        }
        let pa = to_mq(raw.vertices[a]);
        let pb = to_mq(raw.vertices[b]);
        let pc = to_mq(raw.vertices[c]);
        // RawMesh triangles use RenderWare's clockwise front-face order.
        // Match the importer and normalized writer so topology edits do not
        // regenerate every authored normal toward the back face.
        let normal = (pc - pa).cross(pb - pa).normalize_or_zero();
        sums[a] += normal;
        sums[b] += normal;
        sums[c] += normal;
    }
    raw.normals = sums
        .into_iter()
        .map(|normal| {
            let n = normal.normalize_or_zero();
            V3 {
                x: n.x,
                y: n.y,
                z: n.z,
            }
        })
        .collect();
    // Fracture debris is an independent mesh whose source-face mapping and
    // positions no longer describe the intact mesh after a geometry edit.
    for breakable in raw
        .components
        .iter_mut()
        .filter_map(|component| component.breakable.as_mut())
    {
        breakable.stale = true;
    }
}

fn ensure_raw_uvs(raw: &mut RawMesh) {
    if raw.uvs.len() == raw.vertices.len() {
        return;
    }
    if raw.vertices.is_empty() {
        raw.uvs.clear();
        return;
    }
    let bounds = bounds_from_vertices(&raw.vertices);
    let size = bounds.max - bounds.min;
    let sx = size.x.abs().max(0.001);
    let sy = size.y.abs().max(0.001);
    raw.uvs = raw
        .vertices
        .iter()
        .map(|vertex| V2 {
            u: (vertex.x - bounds.min.x) / sx,
            v: (vertex.y - bounds.min.y) / sy,
        })
        .collect();
}

#[allow(dead_code)]
fn planar_unwrap_raw_vertices(raw: &mut RawMesh, vertices: &BTreeSet<usize>) -> usize {
    if vertices.is_empty() {
        return 0;
    }
    ensure_raw_uvs(raw);
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
    for idx in vertices {
        let Some(vertex) = raw.vertices.get(*idx) else {
            continue;
        };
        min.x = min.x.min(vertex.x);
        min.y = min.y.min(vertex.y);
        min.z = min.z.min(vertex.z);
        max.x = max.x.max(vertex.x);
        max.y = max.y.max(vertex.y);
        max.z = max.z.max(vertex.z);
    }
    let extent = V3 {
        x: max.x - min.x,
        y: max.y - min.y,
        z: max.z - min.z,
    };
    let (u_axis, v_axis) = if extent.x <= extent.y && extent.x <= extent.z {
        (1usize, 2usize)
    } else if extent.y <= extent.x && extent.y <= extent.z {
        (0usize, 2usize)
    } else {
        (0usize, 1usize)
    };
    let axis_value = |vertex: V3, axis: usize| match axis {
        0 => vertex.x,
        1 => vertex.y,
        _ => vertex.z,
    };
    let axis_min = |axis: usize| match axis {
        0 => min.x,
        1 => min.y,
        _ => min.z,
    };
    let axis_extent = |axis: usize| match axis {
        0 => extent.x.abs().max(0.001),
        1 => extent.y.abs().max(0.001),
        _ => extent.z.abs().max(0.001),
    };
    let mut changed = 0usize;
    for idx in vertices {
        let Some(vertex) = raw.vertices.get(*idx).copied() else {
            continue;
        };
        if let Some(uv) = raw.uvs.get_mut(*idx) {
            uv.u = (axis_value(vertex, u_axis) - axis_min(u_axis)) / axis_extent(u_axis);
            uv.v = (axis_value(vertex, v_axis) - axis_min(v_axis)) / axis_extent(v_axis);
            changed += 1;
        }
    }
    changed
}

fn raw_triangle_indices(raw: &RawMesh, face_idx: usize) -> Option<[usize; 3]> {
    let tri = raw.triangles.get(face_idx)?;
    let indices = [tri.a as usize, tri.b as usize, tri.c as usize];
    indices
        .iter()
        .all(|idx| *idx < raw.vertices.len())
        .then_some(indices)
}

fn dff_selected_vertex_index(dff: &EditingDffState) -> Option<usize> {
    if let Some(vertex) = dff.selected_vertex {
        return (vertex < dff.raw.vertices.len()).then_some(vertex);
    }
    if let Some(vertex) = dff
        .selected_vertices
        .iter()
        .copied()
        .find(|idx| *idx < dff.raw.vertices.len())
    {
        return Some(vertex);
    }
    if let Some((a, _)) = dff_selected_edge_set(dff).iter().next().copied() {
        return Some(a);
    }
    let face = dff.selected_face?;
    raw_triangle_indices(&dff.raw, face).map(|indices| indices[0])
}

fn dff_selected_vertex_set(dff: &EditingDffState) -> BTreeSet<usize> {
    let mut selected = dff
        .selected_vertices
        .iter()
        .copied()
        .filter(|idx| *idx < dff.raw.vertices.len())
        .collect::<BTreeSet<_>>();
    if let Some(vertex) = dff
        .selected_vertex
        .filter(|idx| *idx < dff.raw.vertices.len())
    {
        selected.insert(vertex);
    }
    if selected.is_empty() && dff.select_mode == EditingSelectMode::Face {
        for face_idx in dff_selected_face_set(dff) {
            if let Some(indices) = raw_triangle_indices(&dff.raw, face_idx) {
                selected.extend(indices);
            }
        }
    }
    if selected.is_empty() {
        for (a, b) in dff_selected_edge_set(dff) {
            selected.insert(a);
            selected.insert(b);
        }
    }
    if selected.is_empty() {
        if let Some(vertex) = dff_selected_vertex_index(dff) {
            selected.insert(vertex);
        }
    }
    selected
}

fn dff_explicit_selected_vertex_set(dff: &EditingDffState) -> BTreeSet<usize> {
    let mut selected = dff
        .selected_vertices
        .iter()
        .copied()
        .filter(|idx| *idx < dff.raw.vertices.len())
        .collect::<BTreeSet<_>>();
    if let Some(vertex) = dff
        .selected_vertex
        .filter(|idx| *idx < dff.raw.vertices.len())
    {
        selected.insert(vertex);
    }
    selected
}

pub(crate) fn dff_selected_face_set(dff: &EditingDffState) -> BTreeSet<usize> {
    let mut selected = dff
        .selected_faces
        .iter()
        .copied()
        .filter(|idx| *idx < dff.raw.triangles.len())
        .collect::<BTreeSet<_>>();
    if let Some(face) = dff
        .selected_face
        .filter(|idx| *idx < dff.raw.triangles.len())
    {
        selected.insert(face);
    }
    selected
}

fn editing_edge_key(a: usize, b: usize) -> (usize, usize) {
    if a <= b { (a, b) } else { (b, a) }
}

fn dff_triangle_edge_vertices(tri: &Tri) -> [(usize, usize); 3] {
    [
        editing_edge_key(tri.a as usize, tri.b as usize),
        editing_edge_key(tri.b as usize, tri.c as usize),
        editing_edge_key(tri.c as usize, tri.a as usize),
    ]
}

fn col_face_edge_vertices(face: &CollisionFace) -> [(usize, usize); 3] {
    [
        editing_edge_key(face.a as usize, face.b as usize),
        editing_edge_key(face.b as usize, face.c as usize),
        editing_edge_key(face.c as usize, face.a as usize),
    ]
}

fn dff_selected_edge_set(dff: &EditingDffState) -> BTreeSet<(usize, usize)> {
    dff.selected_edges
        .iter()
        .copied()
        .filter(|(a, b)| *a < dff.raw.vertices.len() && *b < dff.raw.vertices.len() && a != b)
        .collect()
}

fn col_selected_edge_set(col: &EditingColState) -> BTreeSet<(usize, usize)> {
    col.selected_edges
        .iter()
        .copied()
        .filter(|(a, b)| *a < col.mesh.vertices.len() && *b < col.mesh.vertices.len() && a != b)
        .collect()
}

fn toggle_edge_selection(edges: &mut BTreeSet<(usize, usize)>, edge: (usize, usize)) {
    let edge = editing_edge_key(edge.0, edge.1);
    if !edges.insert(edge) {
        edges.remove(&edge);
    }
}

fn bridge_edge_indices(
    camera_pos: Vec3,
    edge_a: (usize, usize),
    edge_b: (usize, usize),
    point_at: impl Fn(usize) -> Option<Vec3>,
) -> Option<[usize; 4]> {
    let a0 = point_at(edge_a.0)?;
    let a1 = point_at(edge_a.1)?;
    let b0 = point_at(edge_b.0)?;
    let b1 = point_at(edge_b.1)?;
    let direct = a0.distance_squared(b0) + a1.distance_squared(b1);
    let crossed = a0.distance_squared(b1) + a1.distance_squared(b0);
    let mut indices = if direct <= crossed {
        [edge_a.0, edge_a.1, edge_b.1, edge_b.0]
    } else {
        [edge_a.0, edge_a.1, edge_b.0, edge_b.1]
    };
    let p0 = point_at(indices[0])?;
    let p1 = point_at(indices[1])?;
    let p2 = point_at(indices[2])?;
    let centroid = (p0 + p1 + p2 + point_at(indices[3])?) * 0.25;
    let normal = (p1 - p0).cross(p2 - p0).normalize_or_zero();
    let to_camera = (camera_pos - centroid).normalize_or_zero();
    if normal.dot(to_camera) > 0.0 {
        indices.swap(1, 3);
    }
    Some(indices)
}

#[derive(Clone, Copy)]
enum BridgeFace {
    Tri([usize; 3]),
    Quad([usize; 4]),
}

impl BridgeFace {
    fn triangle_count(self) -> usize {
        match self {
            BridgeFace::Tri(_) => 1,
            BridgeFace::Quad(_) => 2,
        }
    }
}

fn bridge_triangle_indices(
    camera_pos: Vec3,
    mut indices: [usize; 3],
    point_at: impl Fn(usize) -> Option<Vec3>,
) -> Option<[usize; 3]> {
    let p0 = point_at(indices[0])?;
    let p1 = point_at(indices[1])?;
    let p2 = point_at(indices[2])?;
    let centroid = (p0 + p1 + p2) / 3.0;
    let normal = (p1 - p0).cross(p2 - p0).normalize_or_zero();
    let to_camera = (camera_pos - centroid).normalize_or_zero();
    if normal.dot(to_camera) > 0.0 {
        indices.swap(1, 2);
    }
    Some(indices)
}

fn bridge_chain_parameters(
    chain: &[usize],
    point_at: impl Fn(usize) -> Option<Vec3> + Copy,
) -> Option<Vec<f32>> {
    if chain.len() < 2 {
        return None;
    }
    let mut distances = vec![0.0f32; chain.len()];
    for i in 1..chain.len() {
        distances[i] = distances[i - 1] + point_at(chain[i - 1])?.distance(point_at(chain[i])?);
    }
    let total = *distances.last()?;
    if total > 0.0001 {
        for distance in &mut distances {
            *distance /= total;
        }
    } else {
        let denom = (chain.len() - 1) as f32;
        for (idx, distance) in distances.iter_mut().enumerate() {
            *distance = idx as f32 / denom;
        }
    }
    Some(distances)
}

fn bridge_vertex_loop_indices(
    camera_pos: Vec3,
    indices: &[usize],
    point_at: impl Fn(usize) -> Option<Vec3> + Copy,
) -> Option<Vec<[usize; 4]>> {
    if indices.len() < 6 || indices.len() % 2 != 0 {
        return None;
    }
    let mut points = Vec::<(usize, Vec3)>::new();
    for idx in indices {
        points.push((*idx, point_at(*idx)?));
    }
    let min = points
        .iter()
        .fold(Vec3::splat(f32::MAX), |acc, (_, p)| acc.min(*p));
    let max = points
        .iter()
        .fold(Vec3::splat(f32::MIN), |acc, (_, p)| acc.max(*p));
    let span = max - min;
    let axis = if span.x >= span.y && span.x >= span.z {
        Vec3::X
    } else if span.y >= span.z {
        Vec3::Y
    } else {
        Vec3::Z
    };
    points.sort_by(|(_, a), (_, b)| {
        a.dot(axis)
            .partial_cmp(&b.dot(axis))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let half = points.len() / 2;
    let first = points[..half]
        .iter()
        .map(|(idx, _)| *idx)
        .collect::<Vec<_>>();
    let second = points[half..]
        .iter()
        .map(|(idx, _)| *idx)
        .collect::<Vec<_>>();
    let first = camera_facing_vertex_order(camera_pos, &first, point_at)?;
    let second = camera_facing_vertex_order(camera_pos, &second, point_at)?;
    let mut quads = Vec::new();
    for i in 0..half {
        let a0 = first[i];
        let a1 = first[(i + 1) % half];
        let b0 = second[i];
        let b1 = second[(i + 1) % half];
        quads.push(bridge_edge_indices(
            camera_pos,
            (a0, a1),
            (b0, b1),
            point_at,
        )?);
    }
    Some(quads)
}

fn ordered_selected_edge_component(
    component_edges: &BTreeSet<(usize, usize)>,
) -> Option<(Vec<usize>, bool)> {
    let mut adjacency = BTreeMap::<usize, Vec<usize>>::new();
    for (a, b) in component_edges {
        adjacency.entry(*a).or_default().push(*b);
        adjacency.entry(*b).or_default().push(*a);
    }
    if adjacency.values().any(|neighbors| neighbors.len() > 2) {
        return None;
    }
    let endpoints = adjacency
        .iter()
        .filter_map(|(idx, neighbors)| (neighbors.len() == 1).then_some(*idx))
        .collect::<Vec<_>>();
    let closed = endpoints.is_empty();
    if !closed && endpoints.len() != 2 {
        return None;
    }
    let start = if closed {
        *adjacency.keys().next()?
    } else {
        endpoints[0]
    };
    let mut ordered = Vec::new();
    let mut previous = None;
    let mut current = start;
    loop {
        ordered.push(current);
        let neighbors = adjacency.get(&current)?;
        let next = neighbors.iter().copied().find(|neighbor| {
            Some(*neighbor) != previous
                && (closed || !ordered.contains(neighbor))
                && (closed || adjacency.get(neighbor).is_some_and(|list| !list.is_empty()))
        });
        let Some(next) = next else {
            break;
        };
        if closed && next == start {
            break;
        }
        previous = Some(current);
        current = next;
        if ordered.len() > adjacency.len() {
            return None;
        }
    }
    if ordered.len() != adjacency.len() {
        return None;
    }
    Some((ordered, closed))
}

fn selected_edge_components(edges: &BTreeSet<(usize, usize)>) -> Option<Vec<(Vec<usize>, bool)>> {
    let mut remaining = edges.clone();
    let mut components = Vec::new();
    while let Some(first) = remaining.iter().next().copied() {
        let mut stack = vec![first.0, first.1];
        let mut vertices = BTreeSet::new();
        while let Some(vertex) = stack.pop() {
            if !vertices.insert(vertex) {
                continue;
            }
            for (a, b) in edges {
                if *a == vertex && !vertices.contains(b) {
                    stack.push(*b);
                }
                if *b == vertex && !vertices.contains(a) {
                    stack.push(*a);
                }
            }
        }
        let component_edges = remaining
            .iter()
            .copied()
            .filter(|(a, b)| vertices.contains(a) && vertices.contains(b))
            .collect::<BTreeSet<_>>();
        for edge in &component_edges {
            remaining.remove(edge);
        }
        components.push(ordered_selected_edge_component(&component_edges)?);
    }
    Some(components)
}

fn align_bridge_loop(
    first: &[usize],
    second: &[usize],
    closed: bool,
    point_at: impl Fn(usize) -> Option<Vec3> + Copy,
) -> Option<Vec<usize>> {
    if first.len() < 2 || second.len() < 2 {
        return None;
    }
    if !closed {
        let first_start = point_at(*first.first()?)?;
        let first_end = point_at(*first.last()?)?;
        let second_start = point_at(*second.first()?)?;
        let second_end = point_at(*second.last()?)?;
        let direct =
            first_start.distance_squared(second_start) + first_end.distance_squared(second_end);
        let crossed =
            first_start.distance_squared(second_end) + first_end.distance_squared(second_start);
        let mut aligned = second.to_vec();
        if crossed < direct {
            aligned.reverse();
        }
        return Some(aligned);
    }
    let mut best = None::<(Vec<usize>, f32)>;
    for reversed in [false, true] {
        let mut candidate = second.to_vec();
        if reversed {
            candidate.reverse();
        }
        let rotations = if closed { candidate.len() } else { 1 };
        for rotate in 0..rotations {
            let mut rotated = candidate.clone();
            rotated.rotate_left(rotate);
            let mut score = 0.0;
            for (a, b) in first.iter().zip(rotated.iter()) {
                score += point_at(*a)?.distance_squared(point_at(*b)?);
            }
            if best
                .as_ref()
                .is_none_or(|(_, best_score)| score < *best_score)
            {
                best = Some((rotated, score));
            }
        }
    }
    best.map(|(indices, _)| indices)
}

fn bridge_selected_edge_loops(
    camera_pos: Vec3,
    edges: &BTreeSet<(usize, usize)>,
    point_at: impl Fn(usize) -> Option<Vec3> + Copy,
) -> Option<Vec<BridgeFace>> {
    let components = selected_edge_components(edges)?;
    if components.len() != 2 {
        return None;
    }
    let (first, first_closed) = &components[0];
    let (second, second_closed) = &components[1];
    if first_closed != second_closed {
        return None;
    }
    let second = align_bridge_loop(first, second, *first_closed, point_at)?;
    if !*first_closed && first.len() != second.len() {
        let first_t = bridge_chain_parameters(first, point_at)?;
        let second_t = bridge_chain_parameters(&second, point_at)?;
        let mut faces = Vec::new();
        let mut i = 0usize;
        let mut j = 0usize;
        const PARAM_EPSILON: f32 = 0.0001;
        while i + 1 < first.len() || j + 1 < second.len() {
            let next_first = first_t.get(i + 1).copied().unwrap_or(f32::INFINITY);
            let next_second = second_t.get(j + 1).copied().unwrap_or(f32::INFINITY);
            if i + 1 < first.len()
                && j + 1 < second.len()
                && (next_first - next_second).abs() <= PARAM_EPSILON
            {
                faces.push(BridgeFace::Quad(bridge_edge_indices(
                    camera_pos,
                    (first[i], first[i + 1]),
                    (second[j], second[j + 1]),
                    point_at,
                )?));
                i += 1;
                j += 1;
            } else if i + 1 < first.len() && (j + 1 >= second.len() || next_first < next_second) {
                faces.push(BridgeFace::Tri(bridge_triangle_indices(
                    camera_pos,
                    [first[i], first[i + 1], second[j]],
                    point_at,
                )?));
                i += 1;
            } else if j + 1 < second.len() {
                faces.push(BridgeFace::Tri(bridge_triangle_indices(
                    camera_pos,
                    [first[i], second[j + 1], second[j]],
                    point_at,
                )?));
                j += 1;
            } else {
                break;
            }
        }
        return Some(faces);
    }
    if first.len() != second.len() {
        return None;
    }
    let segment_count = if *first_closed {
        first.len()
    } else {
        first.len().saturating_sub(1)
    };
    let mut faces = Vec::new();
    for i in 0..segment_count {
        let next = (i + 1) % first.len();
        faces.push(BridgeFace::Quad(bridge_edge_indices(
            camera_pos,
            (first[i], first[next]),
            (second[i], second[next]),
            point_at,
        )?));
    }
    Some(faces)
}

fn bridge_face_triangle_count(faces: &[BridgeFace]) -> usize {
    faces.iter().copied().map(BridgeFace::triangle_count).sum()
}

fn push_dff_bridge_faces(raw: &mut RawMesh, faces: &[BridgeFace], material: u16) {
    for face in faces {
        match *face {
            BridgeFace::Tri(indices) => raw.triangles.push(Tri {
                a: indices[0] as u32,
                b: indices[1] as u32,
                c: indices[2] as u32,
                material,
            }),
            BridgeFace::Quad(indices) => {
                raw.triangles.push(Tri {
                    a: indices[0] as u32,
                    b: indices[1] as u32,
                    c: indices[2] as u32,
                    material,
                });
                raw.triangles.push(Tri {
                    a: indices[0] as u32,
                    b: indices[2] as u32,
                    c: indices[3] as u32,
                    material,
                });
            }
        }
    }
}

fn bridge_faces_fit_col(faces: &[BridgeFace]) -> bool {
    faces.iter().all(|face| match *face {
        BridgeFace::Tri(indices) => indices.iter().all(|idx| *idx <= u16::MAX as usize),
        BridgeFace::Quad(indices) => indices.iter().all(|idx| *idx <= u16::MAX as usize),
    })
}

fn push_col_bridge_faces(
    mesh: &mut CollisionMesh,
    faces: &[BridgeFace],
    material: u8,
    light: u8,
    img_path: &Path,
) {
    for face in faces {
        let mut push = |a: usize, b: usize, c: usize| {
            mesh.faces.push(CollisionFace {
                a: a as u16,
                b: b as u16,
                c: c as u16,
                material,
                light,
                img_path: img_path.to_path_buf(),
                material_file_offset: 0,
                light_file_offset: 0,
            });
        };
        match *face {
            BridgeFace::Tri(indices) => push(indices[0], indices[1], indices[2]),
            BridgeFace::Quad(indices) => {
                push(indices[0], indices[1], indices[2]);
                push(indices[0], indices[2], indices[3]);
            }
        }
    }
}

fn dff_set_single_face_selection(dff: &mut EditingDffState, face: usize) {
    dff.selected_face = Some(face);
    dff.selected_faces.clear();
    dff.selected_faces.insert(face);
}

fn dff_toggle_face_selection(dff: &mut EditingDffState, face: usize) {
    if face >= dff.raw.triangles.len() {
        return;
    }
    dff.selected_face = Some(face);
    if !dff.selected_faces.insert(face) {
        dff.selected_faces.remove(&face);
        if dff.selected_faces.is_empty() {
            dff.selected_faces.insert(face);
        }
    }
}

fn dff_boolean_box_for_raw(raw: &RawMesh) -> DffBooleanBox {
    let bounds = bounds_from_vertices(&raw.vertices);
    let center = (bounds.min + bounds.max) * 0.5;
    let size = (bounds.max - bounds.min).length().max(16.0) * 0.08;
    DffBooleanBox {
        center: from_mq(center),
        half_extents: V3 {
            x: size,
            y: size,
            z: size,
        },
    }
}

fn dff_boolean_contains(cutter: DffBooleanBox, p: V3) -> bool {
    (p.x - cutter.center.x).abs() <= cutter.half_extents.x
        && (p.y - cutter.center.y).abs() <= cutter.half_extents.y
        && (p.z - cutter.center.z).abs() <= cutter.half_extents.z
}

pub(crate) fn apply_dff_boolean_box(raw: &mut RawMesh, cutter: DffBooleanBox) -> usize {
    let before = raw.triangles.len();
    raw.triangles.retain(|tri| {
        let indices = [tri.a as usize, tri.b as usize, tri.c as usize];
        if indices.iter().any(|idx| *idx >= raw.vertices.len()) {
            return true;
        }
        let a = raw.vertices[indices[0]];
        let b = raw.vertices[indices[1]];
        let c = raw.vertices[indices[2]];
        let centroid = V3 {
            x: (a.x + b.x + c.x) / 3.0,
            y: (a.y + b.y + c.y) / 3.0,
            z: (a.z + b.z + c.z) / 3.0,
        };
        !dff_boolean_contains(cutter, centroid)
    });
    before.saturating_sub(raw.triangles.len())
}

pub(crate) fn compact_raw_vertices(raw: &mut RawMesh) {
    let mut used = vec![false; raw.vertices.len()];
    for tri in &raw.triangles {
        for idx in [tri.a, tri.b, tri.c] {
            if let Some(slot) = used.get_mut(idx as usize) {
                *slot = true;
            }
        }
    }
    let mut remap = vec![0usize; raw.vertices.len()];
    let mut next = 0usize;
    for (idx, is_used) in used.iter().copied().enumerate() {
        if is_used {
            remap[idx] = next;
            next += 1;
        }
    }
    let take_used_v3 = |values: &mut Vec<V3>| {
        if values.len() == used.len() {
            *values = values
                .iter()
                .copied()
                .enumerate()
                .filter_map(|(idx, value)| used[idx].then_some(value))
                .collect();
        }
    };
    take_used_v3(&mut raw.vertices);
    take_used_v3(&mut raw.normals);
    take_used_v3(&mut raw.prelit_colors);
    take_used_v3(&mut raw.night_prelit_colors);
    let take_used_f32 = |values: &mut Vec<f32>| {
        if values.len() == used.len() {
            *values = values
                .iter()
                .copied()
                .enumerate()
                .filter_map(|(idx, value)| used[idx].then_some(value))
                .collect();
        }
    };
    take_used_f32(&mut raw.prelit_alphas);
    take_used_f32(&mut raw.night_prelit_alphas);
    if raw.uvs.len() == used.len() {
        raw.uvs = raw
            .uvs
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(idx, value)| used[idx].then_some(value))
            .collect();
    }
    for uvs in &mut raw.secondary_uvs {
        if uvs.len() == used.len() {
            *uvs = uvs
                .iter()
                .copied()
                .enumerate()
                .filter_map(|(idx, value)| used[idx].then_some(value))
                .collect();
        }
    }
    if raw.light_flags.len() == used.len() {
        raw.light_flags = raw
            .light_flags
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(idx, value)| used[idx].then_some(value))
            .collect();
    }
    for tri in &mut raw.triangles {
        tri.a = remap[tri.a as usize] as u32;
        tri.b = remap[tri.b as usize] as u32;
        tri.c = remap[tri.c as usize] as u32;
    }
}

fn compact_col_mesh_vertices(mesh: &mut CollisionMesh) {
    let mut used = vec![false; mesh.vertices.len()];
    for face in &mesh.faces {
        for idx in [face.a, face.b, face.c] {
            if let Some(slot) = used.get_mut(idx as usize) {
                *slot = true;
            }
        }
    }
    let mut remap = vec![0usize; mesh.vertices.len()];
    let mut next = 0usize;
    for (idx, is_used) in used.iter().copied().enumerate() {
        if is_used {
            remap[idx] = next;
            next += 1;
        }
    }
    mesh.vertices = mesh
        .vertices
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(idx, value)| used[idx].then_some(value))
        .collect();
    for face in &mut mesh.faces {
        face.a = remap[face.a as usize] as u16;
        face.b = remap[face.b as usize] as u16;
        face.c = remap[face.c as usize] as u16;
    }
    mesh.bounds = collision_mesh_bounds(&mesh.vertices, &mesh.spheres, &mesh.boxes);
}

fn remove_raw_vertices(raw: &mut RawMesh, selected: &BTreeSet<usize>) -> (usize, usize) {
    if selected.is_empty() {
        return (0, 0);
    }
    let original_len = raw.vertices.len();
    let mut keep = vec![true; original_len];
    for idx in selected {
        if let Some(slot) = keep.get_mut(*idx) {
            *slot = false;
        }
    }
    let mut remap = vec![usize::MAX; original_len];
    let mut next = 0usize;
    for (idx, should_keep) in keep.iter().copied().enumerate() {
        if should_keep {
            remap[idx] = next;
            next += 1;
        }
    }
    let before_faces = raw.triangles.len();
    raw.triangles.retain_mut(|tri| {
        let indices = [tri.a as usize, tri.b as usize, tri.c as usize];
        if indices
            .iter()
            .any(|idx| *idx >= original_len || !keep[*idx])
        {
            return false;
        }
        tri.a = remap[tri.a as usize] as u32;
        tri.b = remap[tri.b as usize] as u32;
        tri.c = remap[tri.c as usize] as u32;
        true
    });
    let take_kept_v3 = |values: &mut Vec<V3>| {
        if values.len() == original_len {
            *values = values
                .iter()
                .copied()
                .enumerate()
                .filter_map(|(idx, value)| keep[idx].then_some(value))
                .collect();
        }
    };
    take_kept_v3(&mut raw.vertices);
    take_kept_v3(&mut raw.normals);
    take_kept_v3(&mut raw.prelit_colors);
    take_kept_v3(&mut raw.night_prelit_colors);
    let take_kept_f32 = |values: &mut Vec<f32>| {
        if values.len() == original_len {
            *values = values
                .iter()
                .copied()
                .enumerate()
                .filter_map(|(idx, value)| keep[idx].then_some(value))
                .collect();
        }
    };
    take_kept_f32(&mut raw.prelit_alphas);
    take_kept_f32(&mut raw.night_prelit_alphas);
    if raw.uvs.len() == original_len {
        raw.uvs = raw
            .uvs
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(idx, value)| keep[idx].then_some(value))
            .collect();
    }
    for uvs in &mut raw.secondary_uvs {
        if uvs.len() == original_len {
            *uvs = uvs
                .iter()
                .copied()
                .enumerate()
                .filter_map(|(idx, value)| keep[idx].then_some(value))
                .collect();
        }
    }
    if raw.light_flags.len() == original_len {
        raw.light_flags = raw
            .light_flags
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(idx, value)| keep[idx].then_some(value))
            .collect();
    }
    let removed_vertices = original_len.saturating_sub(raw.vertices.len());
    let removed_faces = before_faces.saturating_sub(raw.triangles.len());
    (removed_vertices, removed_faces)
}

fn remove_col_vertices(mesh: &mut CollisionMesh, selected: &BTreeSet<usize>) -> (usize, usize) {
    if selected.is_empty() {
        return (0, 0);
    }
    let original_len = mesh.vertices.len();
    let mut keep = vec![true; original_len];
    for idx in selected {
        if let Some(slot) = keep.get_mut(*idx) {
            *slot = false;
        }
    }
    let mut remap = vec![usize::MAX; original_len];
    let mut next = 0usize;
    for (idx, should_keep) in keep.iter().copied().enumerate() {
        if should_keep {
            remap[idx] = next;
            next += 1;
        }
    }
    let before_faces = mesh.faces.len();
    mesh.faces.retain_mut(|face| {
        let indices = [face.a as usize, face.b as usize, face.c as usize];
        if indices
            .iter()
            .any(|idx| *idx >= original_len || !keep[*idx])
        {
            return false;
        }
        face.a = remap[face.a as usize] as u16;
        face.b = remap[face.b as usize] as u16;
        face.c = remap[face.c as usize] as u16;
        true
    });
    mesh.vertices = mesh
        .vertices
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(idx, value)| keep[idx].then_some(value))
        .collect();
    mesh.bounds = collision_mesh_bounds(&mesh.vertices, &mesh.spheres, &mesh.boxes);
    let removed_vertices = original_len.saturating_sub(mesh.vertices.len());
    let removed_faces = before_faces.saturating_sub(mesh.faces.len());
    (removed_vertices, removed_faces)
}

/// Max UV distance for two co-located vertices to be considered weldable.
/// Vertices on opposite sides of a texture seam carry different UVs and must
/// stay separate, otherwise the texture smears across the seam.
const MERGE_UV_EPSILON: f32 = 1.0 / 1024.0;

fn merge_points_by_distance(
    vertices: &[V3],
    uv_sets: &[&[V2]],
    distance: f32,
) -> (Vec<usize>, usize) {
    let threshold = distance.max(0.0);
    if threshold <= 0.0 {
        return ((0..vertices.len()).collect(), 0);
    }
    let threshold2 = threshold * threshold;
    let uv_eps2 = MERGE_UV_EPSILON * MERGE_UV_EPSILON;
    let cell = threshold;
    let mut remap = vec![usize::MAX; vertices.len()];
    let mut representatives = Vec::<usize>::new();
    let mut grid = HashMap::<[i32; 3], Vec<usize>>::new();
    for (idx, vertex) in vertices.iter().enumerate() {
        let key = [
            (vertex.x / cell).floor() as i32,
            (vertex.y / cell).floor() as i32,
            (vertex.z / cell).floor() as i32,
        ];
        let mut found = None;
        'search: for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let neighbor = [key[0] + dx, key[1] + dy, key[2] + dz];
                    let Some(candidates) = grid.get(&neighbor) else {
                        continue;
                    };
                    for rep_slot in candidates {
                        let rep_idx = representatives[*rep_slot];
                        let rep = vertices[rep_idx];
                        let d2 = (vertex.x - rep.x).powi(2)
                            + (vertex.y - rep.y).powi(2)
                            + (vertex.z - rep.z).powi(2);
                        if d2 > threshold2 {
                            continue;
                        }
                        let mut uv_compatible = true;
                        for uvs in uv_sets {
                            let (a, b) = (uvs[idx], uvs[rep_idx]);
                            let uv_d2 = (a.u - b.u).powi(2) + (a.v - b.v).powi(2);
                            if uv_d2 > uv_eps2 {
                                uv_compatible = false;
                                break;
                            }
                        }
                        if !uv_compatible {
                            continue;
                        }
                        found = Some(*rep_slot);
                        break 'search;
                    }
                }
            }
        }
        let rep_slot = if let Some(rep_slot) = found {
            rep_slot
        } else {
            let rep_slot = representatives.len();
            representatives.push(idx);
            grid.entry(key).or_default().push(rep_slot);
            rep_slot
        };
        remap[idx] = rep_slot;
    }
    let merged = vertices.len().saturating_sub(representatives.len());
    (remap, merged)
}

fn take_representatives_v3(values: &mut Vec<V3>, representatives: &[usize], original_len: usize) {
    if values.len() == original_len {
        *values = representatives
            .iter()
            .filter_map(|idx| values.get(*idx).copied())
            .collect();
    }
}

fn take_representatives_v2(values: &mut Vec<V2>, representatives: &[usize], original_len: usize) {
    if values.len() == original_len {
        *values = representatives
            .iter()
            .filter_map(|idx| values.get(*idx).copied())
            .collect();
    }
}

fn take_representatives_f32(values: &mut Vec<f32>, representatives: &[usize], original_len: usize) {
    if values.len() == original_len {
        *values = representatives
            .iter()
            .filter_map(|idx| values.get(*idx).copied())
            .collect();
    }
}

fn merge_raw_vertices_by_distance(raw: &mut RawMesh, distance: f32) -> usize {
    let original_len = raw.vertices.len();
    let mut uv_sets = Vec::new();
    if raw.uvs.len() == original_len {
        uv_sets.push(raw.uvs.as_slice());
    }
    uv_sets.extend(
        raw.secondary_uvs
            .iter()
            .filter(|uvs| uvs.len() == original_len)
            .map(Vec::as_slice),
    );
    let (remap, merged) = merge_points_by_distance(&raw.vertices, &uv_sets, distance);
    if merged == 0 {
        return 0;
    }
    let mut representatives = vec![usize::MAX; original_len - merged];
    for (idx, rep) in remap.iter().copied().enumerate() {
        representatives[rep] = representatives[rep].min(idx);
    }
    raw.vertices = representatives
        .iter()
        .filter_map(|idx| raw.vertices.get(*idx).copied())
        .collect();
    take_representatives_v3(&mut raw.normals, &representatives, original_len);
    take_representatives_v3(&mut raw.prelit_colors, &representatives, original_len);
    take_representatives_f32(&mut raw.prelit_alphas, &representatives, original_len);
    take_representatives_v3(&mut raw.night_prelit_colors, &representatives, original_len);
    take_representatives_f32(&mut raw.night_prelit_alphas, &representatives, original_len);
    take_representatives_v2(&mut raw.uvs, &representatives, original_len);
    for uvs in &mut raw.secondary_uvs {
        take_representatives_v2(uvs, &representatives, original_len);
    }
    if raw.light_flags.len() == original_len {
        raw.light_flags = representatives
            .iter()
            .filter_map(|idx| raw.light_flags.get(*idx).copied())
            .collect();
    }
    raw.triangles.retain_mut(|tri| {
        tri.a = remap[tri.a as usize] as u32;
        tri.b = remap[tri.b as usize] as u32;
        tri.c = remap[tri.c as usize] as u32;
        tri.a != tri.b && tri.b != tri.c && tri.c != tri.a
    });
    recalc_raw_normals(raw);
    merged
}

fn raw_vertices_have_compatible_uvs(raw: &RawMesh, a: usize, b: usize) -> bool {
    let uv_eps2 = MERGE_UV_EPSILON * MERGE_UV_EPSILON;
    std::iter::once(&raw.uvs)
        .chain(raw.secondary_uvs.iter())
        .filter(|uvs| uvs.len() == raw.vertices.len())
        .all(|uvs| {
            let (Some(a), Some(b)) = (uvs.get(a), uvs.get(b)) else {
                return true;
            };
            (a.u - b.u).powi(2) + (a.v - b.v).powi(2) <= uv_eps2
        })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DffMergeTarget {
    Center,
    First,
    Last,
}

fn merge_target_position(
    vertices: &[V3],
    selected: &[usize],
    active: Option<usize>,
    target: DffMergeTarget,
) -> Option<V3> {
    match target {
        DffMergeTarget::Center => {
            let mut centroid = V3::default();
            for idx in selected {
                let vertex = *vertices.get(*idx)?;
                centroid.x += vertex.x;
                centroid.y += vertex.y;
                centroid.z += vertex.z;
            }
            let count = selected.len() as f32;
            centroid.x /= count;
            centroid.y /= count;
            centroid.z /= count;
            Some(centroid)
        }
        DffMergeTarget::First => selected.first().and_then(|idx| vertices.get(*idx)).copied(),
        DffMergeTarget::Last => active
            .filter(|idx| selected.contains(idx))
            .or_else(|| selected.last().copied())
            .and_then(|idx| vertices.get(idx))
            .copied(),
    }
}

fn merge_selected_raw_vertices_preserving_uvs(
    raw: &mut RawMesh,
    selected: &BTreeSet<usize>,
    active: Option<usize>,
    target: DffMergeTarget,
) -> Option<(V3, usize)> {
    let selected = selected
        .iter()
        .copied()
        .filter(|idx| *idx < raw.vertices.len())
        .collect::<Vec<_>>();
    if selected.len() < 2 {
        return None;
    }

    let position = merge_target_position(&raw.vertices, &selected, active, target)?;

    let mut groups = Vec::<Vec<usize>>::new();
    for idx in selected {
        if let Some(group) = groups.iter_mut().find(|group| {
            group
                .first()
                .is_some_and(|rep| raw_vertices_have_compatible_uvs(raw, *rep, idx))
        }) {
            group.push(idx);
        } else {
            groups.push(vec![idx]);
        }
    }

    let mut remap = (0..raw.vertices.len()).collect::<Vec<_>>();
    for group in &groups {
        let Some((&target, rest)) = group.split_first() else {
            continue;
        };
        if let Some(vertex) = raw.vertices.get_mut(target) {
            *vertex = position;
        }
        for idx in rest {
            remap[*idx] = target;
        }
    }

    raw.triangles.retain_mut(|tri| {
        for slot in [&mut tri.a, &mut tri.b, &mut tri.c] {
            if let Some(target) = remap.get(*slot as usize) {
                *slot = *target as u32;
            }
        }
        tri.a != tri.b && tri.b != tri.c && tri.c != tri.a
    });

    Some((position, groups.len()))
}

fn merge_col_vertices_by_distance(mesh: &mut CollisionMesh, distance: f32) -> usize {
    let original_len = mesh.vertices.len();
    let (remap, merged) = merge_points_by_distance(&mesh.vertices, &[], distance);
    if merged == 0 {
        return 0;
    }
    let mut representatives = vec![usize::MAX; original_len - merged];
    for (idx, rep) in remap.iter().copied().enumerate() {
        representatives[rep] = representatives[rep].min(idx);
    }
    mesh.vertices = representatives
        .iter()
        .filter_map(|idx| mesh.vertices.get(*idx).copied())
        .collect();
    mesh.faces.retain_mut(|face| {
        face.a = remap[face.a as usize] as u16;
        face.b = remap[face.b as usize] as u16;
        face.c = remap[face.c as usize] as u16;
        face.a != face.b && face.b != face.c && face.c != face.a
    });
    mesh.bounds = collision_mesh_bounds(&mesh.vertices, &mesh.spheres, &mesh.boxes);
    merged
}

pub(crate) fn editing_delete_selected_material(app: &mut AppState) {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return;
    };
    let material = dff.selected_material;
    let before = dff.raw.triangles.len();
    dff.raw
        .triangles
        .retain(|tri| tri.material as usize != material);
    let removed = before.saturating_sub(dff.raw.triangles.len());
    if removed > 0 {
        recalc_raw_normals(&mut dff.raw);
        dff.dirty = true;
        dff.selected_face = None;
        dff.selected_faces.clear();
        dff.selected_edges.clear();
        dff.selected_vertex = None;
        dff.selected_vertices.clear();
        app.status_message = format!("Deleted {removed} face(s) using material {material}");
        refresh_editing_dff_preview(app);
    } else {
        app.status_message = "Selected material has no faces to delete".to_string();
    }
}

pub(crate) fn editing_delete_selected_dff_face(app: &mut AppState) {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return;
    };
    let selected = dff_selected_face_set(dff);
    if selected.is_empty() {
        app.status_message = "Pick a DFF face first".to_string();
        return;
    }
    dff.raw.triangles = dff
        .raw
        .triangles
        .iter()
        .enumerate()
        .filter(|(idx, _)| !selected.contains(idx))
        .map(|(_, tri)| *tri)
        .collect();
    compact_raw_vertices(&mut dff.raw);
    recalc_raw_normals(&mut dff.raw);
    dff.selected_face = None;
    dff.selected_faces.clear();
    dff.selected_edges.clear();
    dff.selected_vertex = None;
    dff.selected_vertices.clear();
    dff.dirty = true;
    app.status_message = format!("Deleted {} DFF face(s)", selected.len());
    refresh_editing_dff_preview(app);
}

pub(crate) fn editing_delete_selected_dff_vertex(app: &mut AppState) {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return;
    };
    let selected = dff_selected_vertex_set(dff);
    if selected.is_empty() {
        app.status_message = "Pick a DFF vertex first".to_string();
        return;
    }
    let (removed_vertices, removed_faces) = remove_raw_vertices(&mut dff.raw, &selected);
    if removed_vertices == 0 {
        app.status_message = "Selected DFF vertex no longer exists".to_string();
        return;
    }
    recalc_raw_normals(&mut dff.raw);
    dff.selected_face = None;
    dff.selected_faces.clear();
    dff.selected_edges.clear();
    dff.selected_vertex = None;
    dff.selected_vertices.clear();
    dff.dirty = true;
    app.status_message =
        format!("Deleted {removed_vertices} DFF vertex(es) and {removed_faces} attached face(s)");
    refresh_editing_dff_preview(app);
}

pub(crate) fn editing_delete_selected_dff_edge(app: &mut AppState) {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return;
    };
    let selected = dff_selected_edge_set(dff);
    if selected.is_empty() {
        app.status_message = "Select DFF edge(s) first".to_string();
        return;
    }
    let before = dff.raw.triangles.len();
    dff.raw.triangles.retain(|tri| {
        !dff_triangle_edge_vertices(tri)
            .iter()
            .any(|edge| selected.contains(edge))
    });
    let removed = before.saturating_sub(dff.raw.triangles.len());
    if removed == 0 {
        app.status_message = "Selected DFF edge(s) have no attached faces".to_string();
        return;
    }
    compact_raw_vertices(&mut dff.raw);
    recalc_raw_normals(&mut dff.raw);
    dff.selected_face = None;
    dff.selected_faces.clear();
    dff.selected_edges.clear();
    dff.selected_vertex = None;
    dff.selected_vertices.clear();
    dff.dirty = true;
    app.status_message = format!(
        "Deleted {} DFF edge selection(s) and {removed} attached face(s)",
        selected.len()
    );
    refresh_editing_dff_preview(app);
}

pub(crate) fn selected_editing_dff_vertex_position(app: &AppState) -> Option<Vec3> {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_ref() else {
        return None;
    };
    let selected = dff_selected_vertex_set(dff);
    if selected.is_empty() {
        return None;
    }
    let mut sum = Vec3::ZERO;
    let mut count = 0.0f32;
    for idx in selected {
        if let Some(vertex) = dff.raw.vertices.get(idx) {
            sum += to_mq(*vertex);
            count += 1.0;
        }
    }
    (count > 0.0).then_some(sum / count)
}

pub(crate) fn selected_editing_dff_boolean_box_position(app: &AppState) -> Option<Vec3> {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_ref() else {
        return None;
    };
    dff.boolean_box.map(|cutter| to_mq(cutter.center))
}

pub(crate) fn selected_editing_dff_2dfx_position(app: &AppState) -> Option<Vec3> {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_ref() else {
        return None;
    };
    let idx = dff.selected_2dfx?;
    dff.raw
        .effects_2dfx
        .get(idx)
        .map(|effect| to_mq(effect.position))
}

pub(crate) fn set_selected_editing_dff_vertex_position(
    app: &mut AppState,
    position: V3,
) -> Result<(), String> {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return Err("Open a DFF in Editing first".to_string());
    };
    let selected = dff_selected_vertex_set(dff);
    if selected.is_empty() {
        return Err("Pick a DFF vertex or edge first".to_string());
    }
    let mut start = Vec3::ZERO;
    let mut count = 0.0f32;
    for idx in &selected {
        if let Some(vertex) = dff.raw.vertices.get(*idx) {
            start += to_mq(*vertex);
            count += 1.0;
        }
    }
    if count <= 0.0 {
        return Err("Selected DFF vertex no longer exists".to_string());
    }
    let start = from_mq(start / count);
    let delta = V3 {
        x: position.x - start.x,
        y: position.y - start.y,
        z: position.z - start.z,
    };
    for idx in selected {
        if let Some(vertex) = dff.raw.vertices.get_mut(idx) {
            vertex.x += delta.x;
            vertex.y += delta.y;
            vertex.z += delta.z;
        }
    }
    recalc_raw_normals(&mut dff.raw);
    dff.dirty = true;
    let count = dff_selected_vertex_set(dff).len().max(1);
    app.status_message = format!("Moved {count} DFF vertex(es)");
    refresh_editing_dff_preview(app);
    Ok(())
}

pub(crate) fn open_dff_merge_choice_dialog(app: &mut AppState) -> bool {
    let selected_count = match app.editing.asset.as_ref() {
        Some(EditingAsset::Dff(dff)) => dff_selected_vertex_set(dff).len(),
        Some(EditingAsset::Col(col)) => col_selected_vertex_set(col).len(),
        _ => return false,
    };
    if selected_count < 2 {
        app.status_message = "Select at least two vertices to merge".to_string();
        return true;
    }
    app.dff_merge_choice_dialog = Some(DffMergeChoiceDialog { selected_count });
    true
}

pub(crate) fn editing_merge_selected_vertices(app: &mut AppState, target: DffMergeTarget) -> bool {
    match app.editing.asset.as_mut() {
        Some(EditingAsset::Dff(dff)) => {
            let selected = dff_selected_vertex_set(dff);
            if selected.len() < 2 {
                app.status_message = "Select at least two DFF vertices to merge".to_string();
                return true;
            }
            let active = dff.selected_vertex;
            let Some((merge_position, uv_groups)) =
                merge_selected_raw_vertices_preserving_uvs(&mut dff.raw, &selected, active, target)
            else {
                app.status_message = "Selected DFF vertex no longer exists".to_string();
                return true;
            };
            compact_raw_vertices(&mut dff.raw);
            recalc_raw_normals(&mut dff.raw);
            let merged_indices = dff
                .raw
                .vertices
                .iter()
                .enumerate()
                .filter_map(|(idx, vertex)| {
                    let distance2 = to_mq(*vertex).distance_squared(to_mq(merge_position));
                    (distance2 <= 0.000001).then_some(idx)
                })
                .collect::<Vec<_>>();
            dff.selected_face = None;
            dff.selected_vertex = merged_indices.first().copied();
            dff.selected_vertices.clear();
            for vertex in merged_indices {
                dff.selected_vertices.insert(vertex);
            }
            dff.dirty = true;
            app.status_message = if uv_groups > 1 {
                format!(
                    "Merged {} DFF vertices into {uv_groups} UV seam groups",
                    selected.len()
                )
            } else {
                format!("Merged {} DFF vertices", selected.len())
            };
            refresh_editing_dff_preview(app);
            true
        }
        Some(EditingAsset::Col(col)) => {
            if col_regular_topology_has_editable_primitives(col) {
                app.status_message =
                    "Merge is disabled on the collision layer while editable capsules or rotated boxes are present"
                        .to_string();
                return false;
            }
            let selected = col_selected_vertex_set(col);
            if selected.len() < 2 {
                app.status_message = "Select at least two COL vertices to merge".to_string();
                return true;
            }
            let selected_vec = selected.iter().copied().collect::<Vec<_>>();
            let active = col
                .selected_vertices
                .iter()
                .next_back()
                .copied()
                .or_else(|| selected_vec.last().copied());
            let Some(merge_position) =
                merge_target_position(&col.mesh.vertices, &selected_vec, active, target)
            else {
                app.status_message = "Selected COL vertex no longer exists".to_string();
                return true;
            };
            let target = *selected.iter().next().unwrap();
            if let Some(vertex) = col.mesh.vertices.get_mut(target) {
                *vertex = merge_position;
            }
            col.mesh.faces.retain_mut(|face| {
                for slot in [&mut face.a, &mut face.b, &mut face.c] {
                    if selected.contains(&(*slot as usize)) {
                        *slot = target as u16;
                    }
                }
                face.a != face.b && face.b != face.c && face.c != face.a
            });
            compact_col_mesh_vertices(&mut col.mesh);
            let merged_idx = col
                .mesh
                .vertices
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    let da = to_mq(**a).distance_squared(to_mq(merge_position));
                    let db = to_mq(**b).distance_squared(to_mq(merge_position));
                    da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(idx, _)| idx);
            col.selected_face = col
                .selected_face
                .min(col.mesh.faces.len().saturating_sub(1));
            col.selected_vertex = 0;
            col.selected_vertices.clear();
            if let Some(vertex) = merged_idx {
                col.selected_vertices.insert(vertex);
            }
            refresh_editing_col_bounds(col);
            col.dirty = true;
            app.status_message = format!("Merged {} COL vertices", selected.len());
            true
        }
        _ => false,
    }
}

pub(crate) fn editing_merge_vertices_by_distance(app: &mut AppState, distance: f32) -> bool {
    let mut refresh_dff = false;
    let mut status = None;
    let changed = match app.editing.asset.as_mut() {
        Some(EditingAsset::Dff(dff)) => {
            let before_vertices = dff.raw.vertices.len();
            let before_faces = dff.raw.triangles.len();
            let merged = merge_raw_vertices_by_distance(&mut dff.raw, distance);
            if merged == 0 {
                status = Some(format!(
                    "No DFF vertices within merge distance {:.6}",
                    distance
                ));
                false
            } else {
                compact_raw_vertices(&mut dff.raw);
                let after_vertices = dff.raw.vertices.len();
                let after_faces = dff.raw.triangles.len();
                dff.selected_face = None;
                dff.selected_vertex = None;
                dff.selected_vertices.clear();
                dff.dirty = true;
                refresh_dff = true;
                status = Some(format!(
                    "Merged {merged} DFF vertex(es) by distance {:.6}; vertices {} -> {}, faces {} -> {}",
                    distance, before_vertices, after_vertices, before_faces, after_faces
                ));
                true
            }
        }
        Some(EditingAsset::Col(col)) => {
            if col_regular_topology_has_editable_primitives(col) {
                status = Some(
                    "Merge by Distance is disabled while editable capsules or rotated boxes are present"
                        .to_string(),
                );
                false
            } else {
                let before_vertices = col.mesh.vertices.len();
                let before_faces = col.mesh.faces.len();
                let merged = merge_col_vertices_by_distance(&mut col.mesh, distance);
                if merged == 0 {
                    status = Some(format!(
                        "No COL vertices within merge distance {:.6}",
                        distance
                    ));
                    false
                } else {
                    compact_col_mesh_vertices(&mut col.mesh);
                    col.selected_face = col
                        .selected_face
                        .min(col.mesh.faces.len().saturating_sub(1));
                    col.selected_faces.clear();
                    col.selected_vertices.clear();
                    col.selected_vertex = 0;
                    refresh_editing_col_bounds(col);
                    col.dirty = true;
                    status = Some(format!(
                        "Merged {merged} COL vertex(es) by distance {:.6}; vertices {} -> {}, faces {} -> {}",
                        distance,
                        before_vertices,
                        col.mesh.vertices.len(),
                        before_faces,
                        col.mesh.faces.len()
                    ));
                    true
                }
            }
        }
        _ => false,
    };
    if refresh_dff {
        refresh_editing_dff_preview(app);
    }
    if let Some(status) = status {
        app.status_message = status;
    }
    changed
}

pub(crate) fn editing_dff_picker_texture_names(app: &AppState) -> Vec<String> {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_ref() else {
        return Vec::new();
    };
    let Some(txd_name) = dff.txd_context.as_ref() else {
        return Vec::new();
    };
    let txd_key = asset_key(txd_name, ".txd");
    let mut names = Vec::new();
    for (name, textures) in &app.txd_textures {
        if textures
            .iter()
            .any(|texture| texture.txd_name.eq_ignore_ascii_case(&txd_key))
        {
            names.push(name.clone());
        }
    }
    names.sort();
    names
}

pub(crate) fn default_dff_material() -> RawMaterial {
    RawMaterial {
        color: neutral_vertex_color(),
        alpha: 1.0,
        ambient: 1.0,
        specular: 0.0,
        diffuse: 1.0,
    }
}

const DFF_MATERIAL_COLOR_PRESETS: [V3; 9] = [
    V3 {
        x: 1.0,
        y: 1.0,
        z: 1.0,
    },
    V3 {
        x: 0.5,
        y: 0.5,
        z: 0.5,
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
    V3 {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    },
    V3 {
        x: 1.0,
        y: 1.0,
        z: 0.0,
    },
    V3 {
        x: 0.0,
        y: 1.0,
        z: 1.0,
    },
    V3 {
        x: 1.0,
        y: 0.0,
        z: 1.0,
    },
];
const DFF_MATERIAL_ALPHA_PRESETS: [f32; 5] = [1.0, 0.75, 0.5, 0.25, 0.0];

pub(crate) fn dff_material_slot_count(raw: &RawMesh) -> usize {
    let referenced = raw
        .triangles
        .iter()
        .map(|triangle| triangle.material as usize + 1)
        .max()
        .unwrap_or(0);
    raw.material_textures
        .len()
        .max(raw.materials.len())
        .max(raw.material_animations.len())
        .max(referenced)
        .max(1)
}

pub(crate) fn ensure_dff_material_slots(raw: &mut RawMesh, count: usize) {
    raw.material_textures.resize(count, String::new());
    raw.materials.resize(count, default_dff_material());
    raw.material_animations
        .resize(count, DffMaterialAnim::default());
}

fn apply_dff_material_preset(
    properties: &mut RawMaterial,
    color: Option<V3>,
    alpha: Option<f32>,
) -> bool {
    let next_color = color.unwrap_or(properties.color);
    let next_alpha = alpha.unwrap_or(properties.alpha).clamp(0.0, 1.0);
    if properties.color == next_color && properties.alpha == next_alpha {
        return false;
    }
    properties.color = next_color;
    properties.alpha = next_alpha;
    true
}

fn editing_apply_dff_material_preset(
    app: &mut AppState,
    color: Option<V3>,
    alpha: Option<f32>,
) -> bool {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return false;
    };
    let material = dff.selected_material;
    let count = dff_material_slot_count(&dff.raw).max(material + 1);
    ensure_dff_material_slots(&mut dff.raw, count);
    let properties = &mut dff.raw.materials[material];
    if !apply_dff_material_preset(properties, color, alpha) {
        return false;
    }
    dff.dirty = true;
    refresh_editing_dff_preview(app);
    app.status_message = format!("Updated DFF material #{material:02} color");
    true
}

fn set_dff_material_texture(
    raw: &mut RawMesh,
    material: usize,
    texture_name: &str,
) -> Result<bool, String> {
    let texture_name = texture_name.trim();
    if texture_name.is_empty() {
        return Err("Texture name is empty".to_string());
    }
    let count = dff_material_slot_count(raw);
    if material >= count {
        return Err("Selected DFF material no longer exists".to_string());
    }
    if raw
        .material_textures
        .get(material)
        .is_some_and(|current| current.eq_ignore_ascii_case(texture_name))
    {
        return Ok(false);
    }
    ensure_dff_material_slots(raw, count);
    raw.material_textures[material] = texture_name.to_string();
    Ok(true)
}

fn editing_set_dff_material_texture(
    app: &mut AppState,
    expected_dff: Option<&str>,
    material: usize,
    texture_name: &str,
) -> bool {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        app.status_message = "Open a DFF in Editing first".to_string();
        return false;
    };
    if expected_dff.is_some_and(|name| !dff.name.eq_ignore_ascii_case(name)) {
        app.status_message = "The DFF changed while the texture browser was open".to_string();
        return false;
    }
    match set_dff_material_texture(&mut dff.raw, material, texture_name) {
        Ok(true) => {
            dff.selected_material = material;
            dff.texture_picker_open = false;
            dff.dirty = true;
            app.status_message = format!(
                "Set material #{material:02} texture to '{}'",
                texture_name.trim()
            );
            refresh_editing_dff_preview(app);
            true
        }
        Ok(false) => {
            dff.texture_picker_open = false;
            app.status_message = "Selected material already uses that texture".to_string();
            false
        }
        Err(error) => {
            app.status_message = error;
            false
        }
    }
}

pub(crate) fn editing_set_selected_dff_material_texture(
    app: &mut AppState,
    texture_name: &str,
) -> bool {
    let material = match app.editing.asset.as_ref() {
        Some(EditingAsset::Dff(dff)) => dff.selected_material,
        _ => return false,
    };
    editing_set_dff_material_texture(app, None, material, texture_name)
}

fn append_dff_material_slot(
    raw: &mut RawMesh,
    source: usize,
    texture: Option<&str>,
    copy_animation: bool,
) -> Result<usize, String> {
    let count = dff_material_slot_count(raw);
    if count > u16::MAX as usize {
        return Err("Too many DFF materials to add another".to_string());
    }
    ensure_dff_material_slots(raw, count);
    let source = source.min(count.saturating_sub(1));
    let texture = texture
        .map(str::to_string)
        .unwrap_or_else(|| raw.material_textures[source].clone());
    let properties = raw.materials[source];
    let animation = if copy_animation {
        raw.material_animations[source].clone()
    } else {
        DffMaterialAnim::default()
    };
    raw.material_textures.push(texture);
    raw.materials.push(properties);
    raw.material_animations.push(animation);
    Ok(count)
}

fn assign_dff_faces_to_material(
    raw: &mut RawMesh,
    faces: &BTreeSet<usize>,
    material: usize,
) -> usize {
    let material = material as u16;
    let mut changed = 0;
    for face in faces {
        if let Some(triangle) = raw.triangles.get_mut(*face)
            && triangle.material != material
        {
            triangle.material = material;
            changed += 1;
        }
    }
    changed
}

#[derive(Debug, PartialEq, Eq)]
struct DffMaterialRemap {
    old_to_new: Vec<Option<usize>>,
    removed: usize,
}

fn compact_unused_dff_material_slots(raw: &mut RawMesh) -> DffMaterialRemap {
    let count = dff_material_slot_count(raw);
    ensure_dff_material_slots(raw, count);
    let mut used = vec![false; count];
    for triangle in &raw.triangles {
        used[triangle.material as usize] = true;
    }
    if !used.iter().any(|value| *value) {
        used[0] = true;
    }
    let mut old_to_new = vec![None; count];
    let mut textures = Vec::new();
    let mut materials = Vec::new();
    let mut animations = Vec::new();
    for (old, keep) in used.into_iter().enumerate() {
        if keep {
            let new = textures.len();
            old_to_new[old] = Some(new);
            textures.push(raw.material_textures[old].clone());
            materials.push(raw.materials[old]);
            animations.push(raw.material_animations[old].clone());
        }
    }
    for triangle in &mut raw.triangles {
        triangle.material = old_to_new[triangle.material as usize]
            .expect("referenced DFF material must survive compaction")
            as u16;
    }
    let removed = count - textures.len();
    raw.material_textures = textures;
    raw.materials = materials;
    raw.material_animations = animations;
    DffMaterialRemap {
        old_to_new,
        removed,
    }
}

fn remap_dff_material_sidecar_keys<T>(
    values: &mut HashMap<String, T>,
    dff_name: &str,
    remap: &[Option<usize>],
) {
    let mut moved = Vec::new();
    for old in 0..remap.len() {
        if let Some(value) = values.remove(&material_emitter_key(dff_name, old))
            && let Some(new) = remap[old]
        {
            moved.push((material_emitter_key(dff_name, new), value));
        }
    }
    values.extend(moved);
}

pub(crate) fn editing_remove_unused_dff_materials(app: &mut AppState) -> bool {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return false;
    };
    let dff_name = dff.name.clone();
    let selected = dff.selected_material;
    let count = dff_material_slot_count(&dff.raw);
    let slots_were_aligned = dff.raw.material_textures.len() == count
        && dff.raw.materials.len() == count
        && dff.raw.material_animations.len() == count;
    let remap = compact_unused_dff_material_slots(&mut dff.raw);
    if remap.removed == 0 {
        if !slots_were_aligned {
            dff.dirty = true;
            refresh_editing_dff_preview(app);
            app.status_message = "Repaired sparse DFF material slots".to_string();
            return true;
        }
        app.status_message = "This DFF has no unused material slots".to_string();
        return false;
    }
    dff.selected_material = remap
        .old_to_new
        .get(selected)
        .and_then(|material| *material)
        .or_else(|| {
            remap
                .old_to_new
                .iter()
                .take(selected + 1)
                .rev()
                .find_map(|material| *material)
        })
        .unwrap_or(0);
    dff.material_scroll = dff.selected_material as f32;
    dff.dirty = true;
    remap_dff_material_sidecar_keys(&mut app.material_emitters, &dff_name, &remap.old_to_new);
    remap_dff_material_sidecar_keys(&mut app.shadow_casting, &dff_name, &remap.old_to_new);
    refresh_editing_dff_preview(app);
    app.status_message = format!(
        "Removed {} unused DFF material slot(s). Save to keep the remapped EagleScene metadata.",
        remap.removed
    );
    true
}

fn selected_dff_material_source(dff: &EditingDffState, faces: &BTreeSet<usize>) -> usize {
    faces
        .iter()
        .find_map(|face| {
            dff.raw
                .triangles
                .get(*face)
                .map(|triangle| triangle.material as usize)
        })
        .unwrap_or(dff.selected_material)
}

pub(crate) fn editing_create_material_for_selected_faces(app: &mut AppState) -> bool {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return false;
    };
    let faces = dff_selected_face_set(dff);
    if !faces.iter().any(|face| *face < dff.raw.triangles.len()) {
        app.status_message = "Select one or more DFF faces first".to_string();
        return false;
    }
    let source = selected_dff_material_source(dff, &faces);
    let new_material = match append_dff_material_slot(&mut dff.raw, source, None, false) {
        Ok(material) => material,
        Err(error) => {
            app.status_message = error;
            return false;
        }
    };
    assign_dff_faces_to_material(&mut dff.raw, &faces, new_material);
    dff.selected_material = new_material;
    dff.material_scroll = new_material as f32;
    dff.dirty = true;
    dff.texture_picker_open = false;
    app.status_message = format!(
        "Created material #{new_material:02} for {} selected face(s)",
        faces.len()
    );
    refresh_editing_dff_preview(app);
    true
}

pub(crate) fn editing_assign_selected_material_to_faces(app: &mut AppState) -> bool {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return false;
    };
    let faces = dff_selected_face_set(dff);
    if !faces.iter().any(|face| *face < dff.raw.triangles.len()) {
        app.status_message = "Select one or more DFF faces first".to_string();
        return false;
    }
    let material = dff.selected_material;
    let count = dff_material_slot_count(&dff.raw);
    if material >= count {
        app.status_message = "Select a valid DFF material first".to_string();
        return false;
    }
    let changed = faces
        .iter()
        .filter_map(|face| dff.raw.triangles.get(*face))
        .filter(|triangle| triangle.material as usize != material)
        .count();
    if changed == 0 {
        app.status_message = "Selected faces already use this material".to_string();
        return false;
    }
    ensure_dff_material_slots(&mut dff.raw, count);
    assign_dff_faces_to_material(&mut dff.raw, &faces, material);
    dff.texture_picker_open = false;
    dff.dirty = true;
    app.status_message = format!("Assigned material #{material:02} to {changed} selected face(s)");
    refresh_editing_dff_preview(app);
    true
}

pub(crate) fn reassign_selected_dff_faces_texture(app: &mut AppState, texture_name: &str) -> bool {
    let texture_name = texture_name.trim();
    if texture_name.is_empty() {
        app.status_message = "Texture name is empty".to_string();
        return false;
    }
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return false;
    };
    let faces = dff_selected_face_set(dff);
    if faces.is_empty() {
        app.status_message = "Select one or more DFF faces first".to_string();
        return false;
    }
    if !faces.iter().any(|face| *face < dff.raw.triangles.len()) {
        app.status_message = "Selected DFF faces no longer exist".to_string();
        return false;
    }
    // A texture name is not material identity: create a distinct slot so the
    // selected faces can carry independent surface values and animation.
    let source = selected_dff_material_source(dff, &faces);
    let material_index =
        match append_dff_material_slot(&mut dff.raw, source, Some(texture_name), false) {
            Ok(material) => material,
            Err(error) => {
                app.status_message = error;
                return false;
            }
        };
    assign_dff_faces_to_material(&mut dff.raw, &faces, material_index);
    dff.selected_material = material_index;
    dff.material_scroll = material_index as f32;
    dff.dirty = true;
    dff.texture_picker_open = false;
    app.status_message = format!(
        "Created material #{material_index:02} with texture '{}' for {} face(s)",
        texture_name,
        faces.len()
    );
    refresh_editing_dff_preview(app);
    true
}

pub(crate) fn editing_dff_uv_anim_options(dff: &EditingDffState) -> Vec<String> {
    let mut names = BTreeSet::new();
    for animation in &dff.raw.uv_animations {
        let name = animation.name.trim();
        if !name.is_empty() {
            names.insert(name.to_string());
        }
    }
    for animation in &dff.raw.material_animations {
        for name in &animation.names {
            let name = name.trim();
            if !name.is_empty() {
                names.insert(name.to_string());
            }
        }
    }
    for dictionary in &dff.raw.uv_anim_dictionaries {
        let mut start = None;
        for (idx, byte) in dictionary.iter().copied().enumerate() {
            if (32..=126).contains(&byte) {
                start.get_or_insert(idx);
            } else if let Some(s) = start.take() {
                let len = idx.saturating_sub(s);
                if (2..=32).contains(&len) {
                    if let Ok(name) = std::str::from_utf8(&dictionary[s..idx]) {
                        let name = name.trim();
                        if !name.is_empty() {
                            names.insert(name.to_string());
                        }
                    }
                }
            }
        }
    }
    names.into_iter().collect()
}

fn selected_material_emitter_keys(app: &AppState) -> Option<(String, Option<String>)> {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_ref() else {
        return None;
    };
    let local = material_emitter_key(&dff.name, dff.selected_material);
    let global = dff
        .raw
        .material_textures
        .get(dff.selected_material)
        .map(|texture| texture.trim())
        .filter(|texture| !texture.is_empty())
        .map(material_emitter_texture_key);
    Some((local, global))
}

fn dff_face_casts_shadow_from_parts(
    overrides: &HashMap<String, bool>,
    dff_name: &str,
    face: usize,
    material: usize,
    texture: &str,
) -> bool {
    if let Some(casts_shadow) = overrides.get(&material_emitter_face_key(dff_name, face)) {
        return *casts_shadow;
    }
    if let Some(casts_shadow) = overrides.get(&material_emitter_key(dff_name, material)) {
        return *casts_shadow;
    }
    if texture.trim().is_empty() {
        true
    } else {
        overrides
            .get(&material_emitter_texture_key(texture))
            .copied()
            .unwrap_or(true)
    }
}

fn editing_dff_face_casts_shadow(app: &AppState, dff: &EditingDffState, face: usize) -> bool {
    let material = dff
        .raw
        .triangles
        .get(face)
        .map(|triangle| triangle.material as usize)
        .unwrap_or(dff.selected_material);
    let texture = dff
        .raw
        .material_textures
        .get(material)
        .map(|texture| texture.trim())
        .unwrap_or("");
    dff_face_casts_shadow_from_parts(&app.shadow_casting, &dff.name, face, material, texture)
}

fn selected_shadow_casting(app: &AppState) -> bool {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_ref() else {
        return true;
    };
    if selected_dff_faces_are_emitter_target(dff) {
        let faces = dff_selected_face_set(dff);
        return faces
            .iter()
            .all(|face| editing_dff_face_casts_shadow(app, dff, *face));
    }
    let local = material_emitter_key(&dff.name, dff.selected_material);
    if let Some(casts_shadow) = app.shadow_casting.get(&local) {
        return *casts_shadow;
    }
    let texture = dff
        .raw
        .material_textures
        .get(dff.selected_material)
        .map(|texture| texture.trim())
        .unwrap_or("");
    if texture.is_empty() {
        true
    } else {
        app.shadow_casting
            .get(&material_emitter_texture_key(texture))
            .copied()
            .unwrap_or(true)
    }
}

fn selected_shadow_casting_is_global(app: &AppState) -> bool {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_ref() else {
        return false;
    };
    if selected_dff_faces_are_emitter_target(dff) {
        return false;
    }
    let local = material_emitter_key(&dff.name, dff.selected_material);
    if app.shadow_casting.contains_key(&local) {
        return false;
    }
    dff.raw
        .material_textures
        .get(dff.selected_material)
        .map(|texture| material_emitter_texture_key(texture))
        .is_some_and(|key| app.shadow_casting.contains_key(&key))
}

fn persist_shadow_casting_edit(app: &mut AppState, message: &str) {
    app.status_message = format!("{message}. Save to keep this EagleScene change.");
}

fn toggle_selected_shadow_casting(app: &mut AppState) {
    let casts_shadow = !selected_shadow_casting(app);
    let face_target = match app.editing.asset.as_ref() {
        Some(EditingAsset::Dff(dff)) if selected_dff_faces_are_emitter_target(dff) => Some((
            dff.name.clone(),
            dff_selected_face_set(dff).into_iter().collect::<Vec<_>>(),
        )),
        _ => None,
    };
    if let Some((dff_name, faces)) = face_target {
        for face in &faces {
            app.shadow_casting
                .insert(material_emitter_face_key(&dff_name, *face), casts_shadow);
        }
        persist_shadow_casting_edit(
            app,
            &format!(
                "{} shadow casting for {} selected face(s)",
                if casts_shadow { "Enabled" } else { "Disabled" },
                faces.len()
            ),
        );
        return;
    }
    let Some((local, global)) = selected_material_emitter_keys(app) else {
        return;
    };
    let key = if app.shadow_casting.contains_key(&local) {
        local
    } else if let Some(global) = global.filter(|key| app.shadow_casting.contains_key(key)) {
        global
    } else {
        local
    };
    app.shadow_casting.insert(key, casts_shadow);
    persist_shadow_casting_edit(
        app,
        if casts_shadow {
            "Material now casts shadows"
        } else {
            "Material no longer casts shadows"
        },
    );
}

fn toggle_selected_shadow_casting_scope(app: &mut AppState) {
    let Some((local, global)) = selected_material_emitter_keys(app) else {
        return;
    };
    let Some(global) = global else {
        app.status_message =
            "This material has no texture, so shadow scope must stay on this DFF".to_string();
        return;
    };
    let (current, target, global_scope) = if app.shadow_casting.contains_key(&local) {
        (local, global, true)
    } else if app.shadow_casting.contains_key(&global) {
        (global, local, false)
    } else {
        (local, global, true)
    };
    let casts_shadow = app
        .shadow_casting
        .remove(&current)
        .unwrap_or_else(|| selected_shadow_casting(app));
    app.shadow_casting.insert(target, casts_shadow);
    persist_shadow_casting_edit(
        app,
        if global_scope {
            "Shadow setting now applies to every material using this texture"
        } else {
            "Shadow setting now applies only to this DFF material"
        },
    );
}

fn selected_dff_faces_are_emitter_target(dff: &EditingDffState) -> bool {
    dff.select_mode == EditingSelectMode::Face && !dff_selected_face_set(dff).is_empty()
}

fn dff_face_emitter_entries(
    app: &AppState,
    dff_name: &str,
) -> Vec<(String, Vec<usize>, MaterialEmitter)> {
    let dff_key = asset_key(dff_name, ".dff");
    let mut entries: Vec<_> = app
        .material_emitters
        .iter()
        .filter_map(|(key, emitter)| {
            if let Some((dff, faces)) = material_emitter_face_group_from_key(key) {
                return (dff == dff_key).then(|| (key.clone(), faces, *emitter));
            }
            let (dff, face) = material_emitter_face_from_key(key)?;
            (dff == dff_key).then(|| (key.clone(), vec![face], *emitter))
        })
        .collect();
    entries.sort_by(|a, b| a.1.cmp(&b.1));
    entries
}

pub(crate) fn dff_face_emitter_entry_count(app: &AppState, dff_name: &str) -> usize {
    dff_face_emitter_entries(app, dff_name).len()
}

fn selected_face_emitter_keys(app: &AppState) -> Option<Vec<String>> {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_ref() else {
        return None;
    };
    if !selected_dff_faces_are_emitter_target(dff) {
        return None;
    }
    let faces: Vec<_> = dff_selected_face_set(dff).into_iter().collect();
    let group_key = material_emitter_face_group_key(&dff.name, &faces);
    if faces.len() > 1 || app.material_emitters.contains_key(&group_key) {
        return Some(vec![group_key]);
    }
    let face_key = material_emitter_face_key(&dff.name, faces[0]);
    if app.material_emitters.contains_key(&face_key) {
        return Some(vec![face_key]);
    }
    dff_face_emitter_entries(app, &dff.name)
        .into_iter()
        .find(|(_, group_faces, _)| group_faces.contains(&faces[0]))
        .map(|(key, _, _)| vec![key])
        .or(Some(vec![face_key]))
}

fn selected_emitter_keys(app: &AppState) -> Option<Vec<String>> {
    if let Some(keys) = selected_face_emitter_keys(app) {
        return Some(keys);
    }
    selected_material_emitter_key_for_material(app).map(|key| vec![key])
}

fn selected_material_emitter_key_for_material(app: &AppState) -> Option<String> {
    let (local, global) = selected_material_emitter_keys(app)?;
    if app.material_emitters.contains_key(&local) {
        Some(local)
    } else if let Some(global) = global {
        Some(global)
    } else {
        Some(local)
    }
}

pub(crate) fn selected_material_emitter_key(app: &AppState) -> Option<String> {
    selected_emitter_keys(app)?.into_iter().next_back()
}

pub(crate) fn selected_material_emitter(app: &AppState) -> MaterialEmitter {
    selected_material_emitter_key(app)
        .and_then(|key| app.material_emitters.get(&key).copied())
        .unwrap_or_default()
}

pub(crate) fn selected_material_emitter_is_global(app: &AppState) -> bool {
    selected_material_emitter_key(app)
        .as_deref()
        .and_then(material_emitter_texture_from_key)
        .is_some()
}

pub(crate) fn update_selected_emitters(
    app: &mut AppState,
    update: impl FnOnce(&mut MaterialEmitter),
) -> bool {
    let Some(keys) = selected_emitter_keys(app) else {
        return false;
    };
    let mut emitter = selected_material_emitter(app);
    let face_target = match app.editing.asset.as_ref() {
        Some(EditingAsset::Dff(dff)) if selected_dff_faces_are_emitter_target(dff) => Some((
            dff.name.clone(),
            dff_selected_face_set(dff).into_iter().collect::<Vec<_>>(),
        )),
        _ => None,
    };
    update(&mut emitter);
    if let Some((dff_name, faces)) = face_target {
        let exact_group = faces.len() > 1
            && keys
                .first()
                .is_some_and(|key| app.material_emitters.contains_key(key));
        let individual = faces.len() == 1
            && app
                .material_emitters
                .contains_key(&material_emitter_face_key(&dff_name, faces[0]));
        if exact_group || individual {
            app.material_emitters.insert(keys[0].clone(), emitter);
            return true;
        }

        // Editing part of a group first detaches those faces. The remainder
        // keeps the old settings, while this selection receives its own entry.
        let selected: BTreeSet<_> = faces.iter().copied().collect();
        let groups = dff_face_emitter_entries(app, &dff_name);
        for (key, group_faces, group_emitter) in groups {
            if material_emitter_face_group_from_key(&key).is_none()
                || !group_faces.iter().any(|face| selected.contains(face))
            {
                continue;
            }
            app.material_emitters.remove(&key);
            let remaining: Vec<_> = group_faces
                .into_iter()
                .filter(|face| !selected.contains(face))
                .collect();
            match remaining.as_slice() {
                [] => {}
                [face] => {
                    app.material_emitters
                        .insert(material_emitter_face_key(&dff_name, *face), group_emitter);
                }
                _ => {
                    app.material_emitters.insert(
                        material_emitter_face_group_key(&dff_name, &remaining),
                        group_emitter,
                    );
                }
            }
        }
        if faces.len() > 1 {
            for face in &faces {
                app.material_emitters
                    .remove(&material_emitter_face_key(&dff_name, *face));
            }
            app.material_emitters
                .insert(material_emitter_face_group_key(&dff_name, &faces), emitter);
        } else {
            app.material_emitters
                .insert(material_emitter_face_key(&dff_name, faces[0]), emitter);
        }
    } else {
        for key in keys {
            app.material_emitters.insert(key, emitter);
        }
    }
    true
}

fn toggle_selected_material_emitter_scope(app: &mut AppState) {
    if app
        .editing
        .asset
        .as_ref()
        .is_some_and(|asset| matches!(asset, EditingAsset::Dff(dff) if selected_dff_faces_are_emitter_target(dff)))
    {
        app.status_message = "Face emitters always apply only to the selected DFF faces".to_string();
        return;
    }
    let Some((local, global)) = selected_material_emitter_keys(app) else {
        return;
    };
    let Some(global) = global else {
        app.status_message =
            "This material has no texture, so it can only use DFF scope".to_string();
        return;
    };
    let current = if app.material_emitters.contains_key(&local) {
        local.clone()
    } else {
        global.clone()
    };
    let target = if material_emitter_texture_from_key(&current).is_some() {
        local
    } else {
        global
    };
    let emitter = app.material_emitters.remove(&current).unwrap_or_default();
    let global_scope = material_emitter_texture_from_key(&target).is_some();
    app.material_emitters.insert(target, emitter);
    persist_material_emitter_edit(
        app,
        if global_scope {
            "Emitter now applies to every material using this texture"
        } else {
            "Emitter now applies only to this DFF material"
        },
    );
}

pub(crate) fn persist_material_emitter_edit(app: &mut AppState, message: &str) {
    app.material_emitters_dirty = false;
    app.status_message = format!("{message}. Save to keep this EagleScene change.");
}

pub(crate) fn editing_dff_filtered_uv_anim_options(dff: &EditingDffState) -> Vec<String> {
    let query = lower(dff.uv_anim_picker_search.trim());
    editing_dff_uv_anim_options(dff)
        .into_iter()
        .filter(|name| query.is_empty() || lower(name).contains(&query))
        .collect()
}

fn default_dff_uv_animation(name: &str) -> DffUvAnimation {
    DffUvAnimation {
        name: name.to_string(),
        type_id: 0x1c1,
        flags: 0,
        duration: 1.0,
        node_to_uv: [0, 1, 0, 0, 0, 0, 0, 0],
        // Frame layout follows DragonFF/Kam's UV animation convention:
        // rotation_z, scale_x, scale_y, unused, position_x, flipped_position_y.
        frames: vec![
            DffUvAnimFrame {
                time: 0.0,
                uv: [0.0, 1.0, 1.0, 0.0, 0.0, 0.0],
                prev: -1,
            },
            DffUvAnimFrame {
                time: 1.0,
                uv: [0.0, 1.0, 1.0, 0.0, -0.25, 0.0],
                prev: 0,
            },
        ],
    }
}

fn ensure_dff_uv_animation(raw: &mut RawMesh, name: &str) -> usize {
    if let Some(idx) = raw
        .uv_animations
        .iter()
        .position(|animation| animation.name.eq_ignore_ascii_case(name))
    {
        return idx;
    }
    raw.uv_animations.push(default_dff_uv_animation(name));
    raw.uv_animations.len() - 1
}

fn selected_dff_uv_animation_name(dff: &EditingDffState) -> Option<String> {
    dff.raw
        .material_animations
        .get(dff.selected_material)
        .and_then(|animation| animation.names.first())
        .map(|name| name.trim())
        .filter(|name| !name.is_empty())
        .map(ToString::to_string)
}

pub(crate) fn adjust_selected_material_uv_animation_motion(
    app: &mut AppState,
    axis: usize,
    positive: bool,
) -> bool {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return false;
    };
    let Some(name) = selected_dff_uv_animation_name(dff) else {
        app.status_message = "Assign a UV animation first".to_string();
        return false;
    };
    let idx = ensure_dff_uv_animation(&mut dff.raw, &name);
    let animation = &mut dff.raw.uv_animations[idx];
    if animation.frames.len() < 2 {
        *animation = default_dff_uv_animation(&name);
    }
    animation.duration = animation.duration.max(0.05);
    let frame = animation
        .frames
        .last_mut()
        .expect("default animation has frames");
    let component = if axis == 0 { 4 } else { 5 };
    let delta = if positive { -0.25 } else { 0.25 };
    frame.uv[component] = (frame.uv[component] + delta).clamp(-16.0, 16.0);
    dff.dirty = true;
    app.status_message = format!(
        "UV animation '{}' motion: U {:.2}, V {:.2}",
        animation.name, frame.uv[4], frame.uv[5]
    );
    refresh_editing_dff_preview(app);
    true
}

pub(crate) fn assign_selected_material_uv_animation(app: &mut AppState, name: &str) -> bool {
    let name = name.trim();
    if name.is_empty() {
        app.status_message = "UV animation name is empty".to_string();
        return false;
    }
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return false;
    };
    let material = dff.selected_material;
    if material >= dff.raw.material_textures.len().max(1) {
        app.status_message = "Selected material no longer exists".to_string();
        return false;
    }
    if dff.raw.material_animations.len() <= material {
        dff.raw
            .material_animations
            .resize_with(material + 1, DffMaterialAnim::default);
    }
    dff.raw.material_animations[material].names = vec![name.to_string()];
    ensure_dff_uv_animation(&mut dff.raw, name);
    dff.uv_anim_picker_open = false;
    dff.uv_anim_picker_search.clear();
    dff.uv_anim_picker_scroll = 0.0;
    dff.dirty = true;
    app.status_message = format!("Assigned UV animation '{name}' to material {material}");
    refresh_editing_dff_preview(app);
    true
}

pub(crate) fn clear_selected_material_uv_animation(app: &mut AppState) -> bool {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return false;
    };
    let material = dff.selected_material;
    if dff.raw.material_animations.len() <= material
        || dff.raw.material_animations[material].names.is_empty()
    {
        app.status_message = "Selected material has no UV animation".to_string();
        return false;
    }
    dff.raw.material_animations[material].names.clear();
    dff.dirty = true;
    app.status_message = format!("Cleared UV animation from material {material}");
    true
}

pub(crate) fn update_dff_uv_anim_picker_text_input(app: &mut AppState) -> bool {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return false;
    };
    if !dff.uv_anim_picker_open
        && !dff.dff_2dfx_type_picker_open
        && !dff.dff_2dfx_corona_preset_picker_open
        && !dff.dff_2dfx_payload_editor_open
    {
        return false;
    }
    let mut changed = false;
    if is_key_pressed(KeyCode::Escape) {
        dff.uv_anim_picker_open = false;
        dff.dff_2dfx_type_picker_open = false;
        dff.dff_2dfx_corona_preset_picker_open = false;
        dff.dff_2dfx_payload_editor_open = false;
        return true;
    }
    if dff.dff_2dfx_corona_preset_picker_open {
        drain_text_input();
        return true;
    }
    if dff.dff_2dfx_payload_editor_open {
        if selected_dff_2dfx_effect_id(dff) == Some(0) && is_mouse_button_down(MouseButton::Left) {
            let mouse: Vec2 = mouse_position().into();
            for channel in 0..3 {
                let rect = editing_dff_2dfx_light_color_bar_rect(channel);
                if rect.contains(mouse) {
                    let value =
                        (((mouse.x - rect.x) / rect.w).clamp(0.0, 1.0) * 255.0).round() as u8;
                    set_dff_2dfx_light_color_value(dff, channel, value);
                    return true;
                }
            }
        }
        if let Some(active) = dff.dff_2dfx_payload_active_field {
            let active_particle_name = dff_2dfx_active_payload_is_particle_name(dff);
            if let Some(value) = dff.dff_2dfx_payload_fields.get_mut(active) {
                if is_key_pressed(KeyCode::Backspace) && !value.is_empty() {
                    value.pop();
                    changed = true;
                    if active_particle_name {
                        dff.dff_2dfx_particle_picker_scroll = 0.0;
                    }
                }
                while let Some(ch) = get_char_pressed() {
                    if !ch.is_control() {
                        value.push(ch);
                        changed = true;
                        if active_particle_name {
                            dff.dff_2dfx_particle_picker_scroll = 0.0;
                        }
                    }
                }
            } else {
                dff.dff_2dfx_payload_active_field = None;
                drain_text_input();
            }
        } else {
            drain_text_input();
        }
        return changed;
    }
    if dff.dff_2dfx_type_picker_open {
        if is_key_pressed(KeyCode::Backspace) && !dff.dff_2dfx_type_picker_search.is_empty() {
            dff.dff_2dfx_type_picker_search.pop();
            dff.dff_2dfx_type_picker_scroll = 0.0;
            changed = true;
        }
        while let Some(ch) = get_char_pressed() {
            if !ch.is_control() {
                dff.dff_2dfx_type_picker_search.push(ch);
                dff.dff_2dfx_type_picker_scroll = 0.0;
                changed = true;
            }
        }
        return changed;
    }
    if is_key_pressed(KeyCode::Backspace) && !dff.uv_anim_picker_search.is_empty() {
        dff.uv_anim_picker_search.pop();
        dff.uv_anim_picker_scroll = 0.0;
        changed = true;
    }
    while let Some(ch) = get_char_pressed() {
        if !ch.is_control() {
            dff.uv_anim_picker_search.push(ch);
            dff.uv_anim_picker_scroll = 0.0;
            changed = true;
        }
    }
    changed
}

pub(crate) fn editing_make_face_from_selected_vertices(app: &mut AppState) -> bool {
    match app.editing.asset.as_mut() {
        Some(EditingAsset::Dff(dff)) => {
            let selected_edges = dff_selected_edge_set(dff);
            if selected_edges.len() > 2 {
                let Some(faces) =
                    bridge_selected_edge_loops(app.camera.pos, &selected_edges, |idx| {
                        dff.raw.vertices.get(idx).map(|vertex| to_mq(*vertex))
                    })
                else {
                    app.status_message =
                        "Select two matching DFF edge loops or chains to bridge".to_string();
                    return false;
                };
                let material = dff
                    .selected_material
                    .min(dff.raw.material_textures.len().saturating_sub(1))
                    as u16;
                let first = dff.raw.triangles.len();
                let face_count = bridge_face_triangle_count(&faces);
                push_dff_bridge_faces(&mut dff.raw, &faces, material);
                recalc_raw_normals(&mut dff.raw);
                dff.selected_face = Some(first);
                dff.selected_faces = (first..dff.raw.triangles.len()).collect();
                dff.dirty = true;
                app.status_message = format!("Bridged DFF edge loops into {face_count} face(s)");
                refresh_editing_dff_preview(app);
                return true;
            }
            if selected_edges.len() == 2 {
                let edges = selected_edges.iter().copied().collect::<Vec<_>>();
                let Some(indices) =
                    bridge_edge_indices(app.camera.pos, edges[0], edges[1], |idx| {
                        dff.raw.vertices.get(idx).map(|vertex| to_mq(*vertex))
                    })
                else {
                    app.status_message = "Could not bridge selected DFF edges".to_string();
                    return false;
                };
                let material = dff
                    .selected_material
                    .min(dff.raw.material_textures.len().saturating_sub(1))
                    as u16;
                let first = dff.raw.triangles.len();
                dff.raw.triangles.push(Tri {
                    a: indices[0] as u32,
                    b: indices[1] as u32,
                    c: indices[2] as u32,
                    material,
                });
                dff.raw.triangles.push(Tri {
                    a: indices[0] as u32,
                    b: indices[2] as u32,
                    c: indices[3] as u32,
                    material,
                });
                recalc_raw_normals(&mut dff.raw);
                dff.selected_face = Some(first);
                dff.selected_faces.clear();
                dff.selected_faces.insert(first);
                dff.selected_faces.insert(first + 1);
                dff.dirty = true;
                app.status_message = "Bridged 2 DFF edges into 2 face(s)".to_string();
                refresh_editing_dff_preview(app);
                return true;
            }
            let selected = dff_selected_vertex_set(dff);
            if selected.len() >= 6 && selected.len() % 2 == 0 {
                let selected_indices = selected.iter().copied().collect::<Vec<_>>();
                let Some(quads) =
                    bridge_vertex_loop_indices(app.camera.pos, &selected_indices, |idx| {
                        dff.raw.vertices.get(idx).map(|vertex| to_mq(*vertex))
                    })
                else {
                    app.status_message =
                        "Could not split selected DFF vertices into two bridge loops".to_string();
                    return false;
                };
                let material = dff
                    .selected_material
                    .min(dff.raw.material_textures.len().saturating_sub(1))
                    as u16;
                let first = dff.raw.triangles.len();
                for indices in &quads {
                    dff.raw.triangles.push(Tri {
                        a: indices[0] as u32,
                        b: indices[1] as u32,
                        c: indices[2] as u32,
                        material,
                    });
                    dff.raw.triangles.push(Tri {
                        a: indices[0] as u32,
                        b: indices[2] as u32,
                        c: indices[3] as u32,
                        material,
                    });
                }
                recalc_raw_normals(&mut dff.raw);
                dff.selected_face = Some(first);
                dff.selected_faces = (first..dff.raw.triangles.len()).collect();
                dff.dirty = true;
                app.status_message = format!(
                    "Bridged {} DFF vertices into {} face(s)",
                    selected.len(),
                    quads.len() * 2
                );
                refresh_editing_dff_preview(app);
                return true;
            }
            if selected.len() < 3 || selected.len() > 4 {
                app.status_message =
                    "Select 3/4 DFF vertices, an even bridge loop, or 2 DFF edges".to_string();
                return false;
            }
            if selected.iter().any(|idx| *idx >= dff.raw.vertices.len()) {
                app.status_message = "Selected DFF vertex no longer exists".to_string();
                return false;
            }
            let selected_indices = selected.iter().copied().collect::<Vec<_>>();
            let Some(indices) =
                camera_facing_vertex_order(app.camera.pos, &selected_indices, |idx| {
                    dff.raw.vertices.get(idx).map(|vertex| to_mq(*vertex))
                })
            else {
                app.status_message = "Could not order selected DFF vertices".to_string();
                return false;
            };
            let material =
                dff.selected_material
                    .min(dff.raw.material_textures.len().saturating_sub(1)) as u16;
            dff.raw.triangles.push(Tri {
                a: indices[0] as u32,
                b: indices[1] as u32,
                c: indices[2] as u32,
                material,
            });
            if indices.len() == 4 {
                dff.raw.triangles.push(Tri {
                    a: indices[0] as u32,
                    b: indices[2] as u32,
                    c: indices[3] as u32,
                    material,
                });
            }
            recalc_raw_normals(&mut dff.raw);
            dff.selected_face =
                dff.raw
                    .triangles
                    .len()
                    .checked_sub(if indices.len() == 4 { 2 } else { 1 });
            dff.dirty = true;
            app.status_message = format!(
                "Created {} DFF face(s) from {} selected vertices",
                if indices.len() == 4 { 2 } else { 1 },
                indices.len()
            );
            refresh_editing_dff_preview(app);
            true
        }
        Some(EditingAsset::Col(col)) => {
            let selected_vertices = col_selected_vertex_set(col);
            if col_vertices_touch_generated_primitive(col, selected_vertices.iter().copied()) {
                app.status_message =
                    "Generated capsule and rotated-box vertices cannot be used to create faces; select the primitive instead"
                        .to_string();
                return false;
            }
            let selected_edges = col_selected_edge_set(col);
            if selected_edges.len() > 2 {
                let Some(faces) =
                    bridge_selected_edge_loops(app.camera.pos, &selected_edges, |idx| {
                        col.mesh.vertices.get(idx).map(|vertex| to_mq(*vertex))
                    })
                else {
                    app.status_message =
                        "Select two matching COL edge loops or chains to bridge".to_string();
                    return false;
                };
                if !bridge_faces_fit_col(&faces) {
                    app.status_message = "Selected COL edge loop cannot be written".to_string();
                    return false;
                }
                let template = col.mesh.faces.get(col.selected_face).cloned();
                let material = template.as_ref().map(|face| face.material).unwrap_or(0);
                let light = template.as_ref().map(|face| face.light).unwrap_or(255);
                let img_path = template
                    .as_ref()
                    .map(|face| face.img_path.clone())
                    .unwrap_or_default();
                let first = col.mesh.faces.len();
                let face_count = bridge_face_triangle_count(&faces);
                push_col_bridge_faces(&mut col.mesh, &faces, material, light, &img_path);
                col.selected_face = first;
                col.selected_faces = (first..col.mesh.faces.len()).collect();
                col.selected_vertex = 0;
                col.dirty = true;
                app.status_message = format!("Bridged COL edge loops into {face_count} face(s)");
                return true;
            }
            if selected_edges.len() == 2 {
                let edges = selected_edges.iter().copied().collect::<Vec<_>>();
                let Some(indices) =
                    bridge_edge_indices(app.camera.pos, edges[0], edges[1], |idx| {
                        col.mesh.vertices.get(idx).map(|vertex| to_mq(*vertex))
                    })
                else {
                    app.status_message = "Could not bridge selected COL edges".to_string();
                    return false;
                };
                if indices.iter().any(|idx| *idx > u16::MAX as usize) {
                    app.status_message = "Selected COL edge cannot be written".to_string();
                    return false;
                }
                let template = col.mesh.faces.get(col.selected_face).cloned();
                let material = template.as_ref().map(|face| face.material).unwrap_or(0);
                let light = template.as_ref().map(|face| face.light).unwrap_or(255);
                let img_path = template
                    .as_ref()
                    .map(|face| face.img_path.clone())
                    .unwrap_or_default();
                let first = col.mesh.faces.len();
                for (a, b, c) in [
                    (indices[0], indices[1], indices[2]),
                    (indices[0], indices[2], indices[3]),
                ] {
                    col.mesh.faces.push(CollisionFace {
                        a: a as u16,
                        b: b as u16,
                        c: c as u16,
                        material,
                        light,
                        img_path: img_path.clone(),
                        material_file_offset: 0,
                        light_file_offset: 0,
                    });
                }
                col.selected_face = first;
                col.selected_faces.clear();
                col.selected_faces.insert(first);
                col.selected_faces.insert(first + 1);
                col.selected_vertex = 0;
                col.dirty = true;
                app.status_message = "Bridged 2 COL edges into 2 face(s)".to_string();
                return true;
            }
            let selected = col_selected_vertex_set(col);
            if selected.len() >= 6 && selected.len() % 2 == 0 {
                let selected_indices = selected.iter().copied().collect::<Vec<_>>();
                let Some(quads) =
                    bridge_vertex_loop_indices(app.camera.pos, &selected_indices, |idx| {
                        col.mesh.vertices.get(idx).map(|vertex| to_mq(*vertex))
                    })
                else {
                    app.status_message =
                        "Could not split selected COL vertices into two bridge loops".to_string();
                    return false;
                };
                if selected.iter().any(|idx| *idx > u16::MAX as usize) {
                    app.status_message = "Selected COL vertex cannot be written".to_string();
                    return false;
                }
                let template = col.mesh.faces.get(col.selected_face).cloned();
                let material = template.as_ref().map(|face| face.material).unwrap_or(0);
                let light = template.as_ref().map(|face| face.light).unwrap_or(255);
                let img_path = template
                    .as_ref()
                    .map(|face| face.img_path.clone())
                    .unwrap_or_default();
                let first = col.mesh.faces.len();
                for indices in &quads {
                    for (a, b, c) in [
                        (indices[0], indices[1], indices[2]),
                        (indices[0], indices[2], indices[3]),
                    ] {
                        col.mesh.faces.push(CollisionFace {
                            a: a as u16,
                            b: b as u16,
                            c: c as u16,
                            material,
                            light,
                            img_path: img_path.clone(),
                            material_file_offset: 0,
                            light_file_offset: 0,
                        });
                    }
                }
                col.selected_face = first;
                col.selected_faces = (first..col.mesh.faces.len()).collect();
                col.selected_vertex = 0;
                col.dirty = true;
                app.status_message = format!(
                    "Bridged {} COL vertices into {} face(s)",
                    selected.len(),
                    quads.len() * 2
                );
                return true;
            }
            if selected.len() < 3 || selected.len() > 4 {
                app.status_message =
                    "Select 3/4 COL vertices, an even bridge loop, or 2 COL edges".to_string();
                return false;
            }
            if selected
                .iter()
                .any(|idx| *idx >= col.mesh.vertices.len() || *idx > u16::MAX as usize)
            {
                app.status_message = "Selected COL vertex cannot be written".to_string();
                return false;
            }
            let selected_indices = selected.iter().copied().collect::<Vec<_>>();
            let Some(indices) =
                camera_facing_vertex_order(app.camera.pos, &selected_indices, |idx| {
                    col.mesh.vertices.get(idx).map(|vertex| to_mq(*vertex))
                })
            else {
                app.status_message = "Could not order selected COL vertices".to_string();
                return false;
            };
            let template = col.mesh.faces.get(col.selected_face).cloned();
            let material = template.as_ref().map(|face| face.material).unwrap_or(0);
            let light = template.as_ref().map(|face| face.light).unwrap_or(255);
            let img_path = template
                .as_ref()
                .map(|face| face.img_path.clone())
                .unwrap_or_default();
            let push_face = |mesh: &mut CollisionMesh, a: usize, b: usize, c: usize| {
                mesh.faces.push(CollisionFace {
                    a: a as u16,
                    b: b as u16,
                    c: c as u16,
                    material,
                    light,
                    img_path: img_path.clone(),
                    material_file_offset: 0,
                    light_file_offset: 0,
                });
            };
            push_face(&mut col.mesh, indices[0], indices[1], indices[2]);
            if indices.len() == 4 {
                push_face(&mut col.mesh, indices[0], indices[2], indices[3]);
            }
            col.selected_face =
                col.mesh
                    .faces
                    .len()
                    .saturating_sub(if indices.len() == 4 { 2 } else { 1 });
            col.selected_faces.clear();
            col.selected_faces.insert(col.selected_face);
            if indices.len() == 4 {
                col.selected_faces.insert(col.selected_face + 1);
            }
            col.selected_vertex = 0;
            col.dirty = true;
            app.status_message = format!(
                "Created {} COL face(s) from {} selected vertices",
                if indices.len() == 4 { 2 } else { 1 },
                indices.len()
            );
            true
        }
        _ => false,
    }
}

pub(crate) fn editing_extrude_selected(app: &mut AppState) -> bool {
    match app.editing.asset.as_ref() {
        Some(EditingAsset::Dff(dff))
            if dff.select_mode == EditingSelectMode::Edge
                && !dff_selected_edge_set(dff).is_empty() =>
        {
            editing_extrude_selected_edges(app)
        }
        Some(EditingAsset::Col(col))
            if col.select_mode == EditingSelectMode::Edge
                && !col_selected_edge_set(col).is_empty() =>
        {
            editing_extrude_selected_edges(app)
        }
        Some(EditingAsset::Dff(dff))
            if dff.select_mode == EditingSelectMode::Vertex
                && !dff_explicit_selected_vertex_set(dff).is_empty() =>
        {
            editing_extrude_selected_vertices(app)
        }
        Some(EditingAsset::Col(col))
            if col.select_mode == EditingSelectMode::Vertex
                && !col_explicit_selected_vertex_set(col).is_empty() =>
        {
            editing_extrude_selected_vertices(app)
        }
        Some(EditingAsset::Dff(dff)) if !dff_selected_edge_set(dff).is_empty() => {
            editing_extrude_selected_edges(app)
        }
        Some(EditingAsset::Col(col)) if !col_selected_edge_set(col).is_empty() => {
            editing_extrude_selected_edges(app)
        }
        _ => {
            app.status_message = "Select vertex(es) or edge(s) before extruding".to_string();
            false
        }
    }
}

fn editing_extrude_selected_vertices(app: &mut AppState) -> bool {
    match app.editing.asset.as_mut() {
        Some(EditingAsset::Dff(dff)) => {
            let selected = dff_explicit_selected_vertex_set(dff);
            if selected.is_empty() {
                app.status_message = "Select DFF vertex(es) before extruding".to_string();
                return false;
            }
            let mut new_vertices = BTreeSet::new();
            let mut new_edges = BTreeSet::new();
            for old in selected {
                let Some(vertex) = dff.raw.vertices.get(old).copied() else {
                    continue;
                };
                let new_idx = dff.raw.vertices.len();
                dff.raw.vertices.push(vertex);
                if dff.raw.normals.len() == new_idx {
                    dff.raw
                        .normals
                        .push(dff.raw.normals.get(old).copied().unwrap_or_default());
                }
                if dff.raw.uvs.len() == new_idx {
                    dff.raw
                        .uvs
                        .push(dff.raw.uvs.get(old).copied().unwrap_or_default());
                }
                for uvs in &mut dff.raw.secondary_uvs {
                    if uvs.len() == new_idx {
                        uvs.push(uvs.get(old).copied().unwrap_or_default());
                    }
                }
                if dff.raw.prelit_colors.len() == new_idx {
                    dff.raw
                        .prelit_colors
                        .push(dff.raw.prelit_colors.get(old).copied().unwrap_or_default());
                }
                if dff.raw.prelit_alphas.len() == new_idx {
                    dff.raw
                        .prelit_alphas
                        .push(dff.raw.prelit_alphas.get(old).copied().unwrap_or(1.0));
                }
                if dff.raw.night_prelit_colors.len() == new_idx {
                    dff.raw.night_prelit_colors.push(
                        dff.raw
                            .night_prelit_colors
                            .get(old)
                            .copied()
                            .unwrap_or_default(),
                    );
                }
                if dff.raw.night_prelit_alphas.len() == new_idx {
                    dff.raw
                        .night_prelit_alphas
                        .push(dff.raw.night_prelit_alphas.get(old).copied().unwrap_or(1.0));
                }
                if dff.raw.light_flags.len() == new_idx {
                    dff.raw
                        .light_flags
                        .push(dff.raw.light_flags.get(old).copied().unwrap_or(false));
                }
                new_vertices.insert(new_idx);
                new_edges.insert(editing_edge_key(old, new_idx));
            }
            if new_vertices.is_empty() {
                app.status_message = "Could not extrude selected DFF vertex(es)".to_string();
                return false;
            }
            dff.selected_vertices = new_vertices;
            dff.selected_vertex = dff.selected_vertices.iter().next_back().copied();
            dff.selected_edges = new_edges;
            dff.selected_faces.clear();
            dff.selected_face = None;
            dff.dirty = true;
            app.transform_mode = TransformMode::Move;
            app.status_message = format!("Extruded {} DFF vertex(es)", dff.selected_vertices.len());
            refresh_editing_dff_preview(app);
            true
        }
        Some(EditingAsset::Col(col)) => {
            let selected = col_explicit_selected_vertex_set(col);
            if selected.is_empty() {
                app.status_message = "Select COL vertex(es) before extruding".to_string();
                return false;
            }
            if col_vertices_touch_generated_primitive(col, selected.iter().copied()) {
                app.status_message =
                    "Generated capsule and rotated-box vertices cannot be extruded; select the primitive instead"
                        .to_string();
                return false;
            }
            let mut new_vertices = BTreeSet::new();
            let mut new_edges = BTreeSet::new();
            for old in selected {
                let Some(vertex) = col.mesh.vertices.get(old).copied() else {
                    continue;
                };
                let new_idx = col.mesh.vertices.len();
                if new_idx > u16::MAX as usize {
                    app.status_message = "COL has too many vertices to extrude".to_string();
                    return false;
                }
                col.mesh.vertices.push(vertex);
                new_vertices.insert(new_idx);
                new_edges.insert(editing_edge_key(old, new_idx));
            }
            if new_vertices.is_empty() {
                app.status_message = "Could not extrude selected COL vertex(es)".to_string();
                return false;
            }
            col.selected_vertices = new_vertices;
            col.selected_edges = new_edges;
            col.selected_faces.clear();
            col.selected_face = usize::MAX;
            col.selected_vertex = 0;
            col.selected_primitive = None;
            refresh_editing_col_bounds(col);
            col.dirty = true;
            app.transform_mode = TransformMode::Move;
            app.status_message = format!("Extruded {} COL vertex(es)", col.selected_vertices.len());
            true
        }
        _ => false,
    }
}

pub(crate) fn editing_extrude_selected_edges(app: &mut AppState) -> bool {
    match app.editing.asset.as_mut() {
        Some(EditingAsset::Dff(dff)) => {
            let selected_edges = dff_selected_edge_set(dff);
            if selected_edges.is_empty() {
                app.status_message = "Select DFF edge(s) before extruding".to_string();
                return false;
            }
            let mut remap = BTreeMap::<usize, usize>::new();
            for (a, b) in &selected_edges {
                for old in [*a, *b] {
                    if remap.contains_key(&old) {
                        continue;
                    }
                    let Some(vertex) = dff.raw.vertices.get(old).copied() else {
                        continue;
                    };
                    let new_idx = dff.raw.vertices.len();
                    dff.raw.vertices.push(vertex);
                    if dff.raw.normals.len() == new_idx {
                        dff.raw
                            .normals
                            .push(dff.raw.normals.get(old).copied().unwrap_or_default());
                    }
                    if dff.raw.uvs.len() == new_idx {
                        dff.raw
                            .uvs
                            .push(dff.raw.uvs.get(old).copied().unwrap_or_default());
                    }
                    for uvs in &mut dff.raw.secondary_uvs {
                        if uvs.len() == new_idx {
                            uvs.push(uvs.get(old).copied().unwrap_or_default());
                        }
                    }
                    if dff.raw.prelit_colors.len() == new_idx {
                        dff.raw
                            .prelit_colors
                            .push(dff.raw.prelit_colors.get(old).copied().unwrap_or_default());
                    }
                    if dff.raw.prelit_alphas.len() == new_idx {
                        dff.raw
                            .prelit_alphas
                            .push(dff.raw.prelit_alphas.get(old).copied().unwrap_or(1.0));
                    }
                    if dff.raw.night_prelit_colors.len() == new_idx {
                        dff.raw.night_prelit_colors.push(
                            dff.raw
                                .night_prelit_colors
                                .get(old)
                                .copied()
                                .unwrap_or_default(),
                        );
                    }
                    if dff.raw.night_prelit_alphas.len() == new_idx {
                        dff.raw
                            .night_prelit_alphas
                            .push(dff.raw.night_prelit_alphas.get(old).copied().unwrap_or(1.0));
                    }
                    if dff.raw.light_flags.len() == new_idx {
                        dff.raw
                            .light_flags
                            .push(dff.raw.light_flags.get(old).copied().unwrap_or(false));
                    }
                    remap.insert(old, new_idx);
                }
            }
            let material =
                dff.selected_material
                    .min(dff.raw.material_textures.len().saturating_sub(1)) as u16;
            let first_face = dff.raw.triangles.len();
            let mut new_edges = BTreeSet::new();
            for (a, b) in selected_edges {
                let (Some(na), Some(nb)) = (remap.get(&a).copied(), remap.get(&b).copied()) else {
                    continue;
                };
                dff.raw.triangles.push(Tri {
                    a: a as u32,
                    b: b as u32,
                    c: nb as u32,
                    material,
                });
                dff.raw.triangles.push(Tri {
                    a: a as u32,
                    b: nb as u32,
                    c: na as u32,
                    material,
                });
                new_edges.insert(editing_edge_key(na, nb));
            }
            if new_edges.is_empty() {
                app.status_message = "Could not extrude selected DFF edge(s)".to_string();
                return false;
            }
            recalc_raw_normals(&mut dff.raw);
            dff.selected_edges = new_edges;
            dff.selected_vertices.clear();
            dff.selected_vertex = None;
            dff.selected_face = Some(first_face);
            dff.selected_faces = (first_face..dff.raw.triangles.len()).collect();
            dff.dirty = true;
            app.transform_mode = TransformMode::Move;
            app.status_message = format!("Extruded {} DFF edge(s)", dff.selected_edges.len());
            refresh_editing_dff_preview(app);
            true
        }
        Some(EditingAsset::Col(col)) => {
            let selected_edges = col_selected_edge_set(col);
            if selected_edges.is_empty() {
                app.status_message = "Select COL edge(s) before extruding".to_string();
                return false;
            }
            if col_vertices_touch_generated_primitive(
                col,
                selected_edges.iter().flat_map(|(a, b)| [*a, *b]),
            ) {
                app.status_message =
                    "Generated capsule and rotated-box edges cannot be extruded; select the primitive instead"
                        .to_string();
                return false;
            }
            let mut remap = BTreeMap::<usize, usize>::new();
            for (a, b) in &selected_edges {
                for old in [*a, *b] {
                    if remap.contains_key(&old) {
                        continue;
                    }
                    let Some(vertex) = col.mesh.vertices.get(old).copied() else {
                        continue;
                    };
                    let new_idx = col.mesh.vertices.len();
                    if new_idx > u16::MAX as usize {
                        app.status_message = "COL has too many vertices to extrude".to_string();
                        return false;
                    }
                    col.mesh.vertices.push(vertex);
                    remap.insert(old, new_idx);
                }
            }
            let template = col.mesh.faces.get(col.selected_face).cloned();
            let material = template.as_ref().map(|face| face.material).unwrap_or(0);
            let light = template.as_ref().map(|face| face.light).unwrap_or(255);
            let img_path = template
                .as_ref()
                .map(|face| face.img_path.clone())
                .unwrap_or_default();
            let first_face = col.mesh.faces.len();
            let mut new_edges = BTreeSet::new();
            for (a, b) in selected_edges {
                let (Some(na), Some(nb)) = (remap.get(&a).copied(), remap.get(&b).copied()) else {
                    continue;
                };
                for (fa, fb, fc) in [(a, b, nb), (a, nb, na)] {
                    col.mesh.faces.push(CollisionFace {
                        a: fa as u16,
                        b: fb as u16,
                        c: fc as u16,
                        material,
                        light,
                        img_path: img_path.clone(),
                        material_file_offset: 0,
                        light_file_offset: 0,
                    });
                }
                new_edges.insert(editing_edge_key(na, nb));
            }
            if new_edges.is_empty() {
                app.status_message = "Could not extrude selected COL edge(s)".to_string();
                return false;
            }
            refresh_editing_col_bounds(col);
            col.selected_edges = new_edges;
            col.selected_faces = (first_face..col.mesh.faces.len()).collect();
            col.selected_face = first_face;
            col.selected_vertices.clear();
            col.selected_vertex = 0;
            col.selected_primitive = None;
            col.dirty = true;
            app.transform_mode = TransformMode::Move;
            app.status_message = format!("Extruded {} COL edge(s)", col.selected_edges.len());
            true
        }
        _ => false,
    }
}

#[allow(dead_code)]
pub(crate) fn editing_unwrap_selected_dff_uvs(app: &mut AppState, material_scope: bool) -> bool {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return false;
    };
    let mut vertices = BTreeSet::<usize>::new();
    if material_scope {
        for tri in &dff.raw.triangles {
            if tri.material as usize == dff.selected_material {
                vertices.insert(tri.a as usize);
                vertices.insert(tri.b as usize);
                vertices.insert(tri.c as usize);
            }
        }
    } else if let Some(face_idx) = dff.selected_face {
        if let Some(indices) = raw_triangle_indices(&dff.raw, face_idx) {
            vertices.extend(indices);
        }
    } else {
        vertices = dff_selected_vertex_set(dff);
    }
    let changed = planar_unwrap_raw_vertices(&mut dff.raw, &vertices);
    if changed == 0 {
        app.status_message = "Select a DFF face, material, or vertices to unwrap".to_string();
        return false;
    }
    dff.dirty = true;
    app.status_message = if material_scope {
        format!("Planar unwrapped {changed} DFF material vertex UV(s)")
    } else {
        format!("Planar unwrapped {changed} selected DFF UV(s)")
    };
    refresh_editing_dff_preview(app);
    true
}

pub(crate) fn editing_nudge_selected_dff_uvs(app: &mut AppState, du: f32, dv: f32) -> bool {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return false;
    };
    let selected = dff_selected_vertex_set(dff);
    if selected.is_empty() {
        app.status_message = "Select DFF vertices before nudging UVs".to_string();
        return false;
    }
    ensure_raw_uvs(&mut dff.raw);
    let mut changed = 0usize;
    for idx in selected {
        if let Some(uv) = dff.raw.uvs.get_mut(idx) {
            uv.u += du;
            uv.v += dv;
            changed += 1;
        }
    }
    dff.dirty = true;
    app.status_message = format!("Nudged {changed} selected DFF UV(s)");
    refresh_editing_dff_preview(app);
    true
}

/// Append a new vertex whose attributes are the average of `indices`.
/// Attribute arrays are only extended when they are in sync with `vertices`.
fn raw_append_vertex_average(raw: &mut RawMesh, indices: &[usize]) -> usize {
    let n = raw.vertices.len();
    let count = indices.len().max(1) as f32;
    if raw.normals.len() == n {
        let mut acc = V3::default();
        for &idx in indices {
            let value = raw.normals[idx];
            acc.x += value.x;
            acc.y += value.y;
            acc.z += value.z;
        }
        raw.normals.push(V3 {
            x: acc.x / count,
            y: acc.y / count,
            z: acc.z / count,
        });
    }
    if raw.uvs.len() == n {
        let mut u = 0.0f32;
        let mut v = 0.0f32;
        for &idx in indices {
            u += raw.uvs[idx].u;
            v += raw.uvs[idx].v;
        }
        raw.uvs.push(V2 {
            u: u / count,
            v: v / count,
        });
    }
    for uvs in &mut raw.secondary_uvs {
        if uvs.len() == n {
            let mut u = 0.0f32;
            let mut v = 0.0f32;
            for &idx in indices {
                u += uvs[idx].u;
                v += uvs[idx].v;
            }
            uvs.push(V2 {
                u: u / count,
                v: v / count,
            });
        }
    }
    if raw.prelit_colors.len() == n {
        let mut acc = V3::default();
        for &idx in indices {
            let value = raw.prelit_colors[idx];
            acc.x += value.x;
            acc.y += value.y;
            acc.z += value.z;
        }
        raw.prelit_colors.push(V3 {
            x: acc.x / count,
            y: acc.y / count,
            z: acc.z / count,
        });
    }
    if raw.prelit_alphas.len() == n {
        let value = indices
            .iter()
            .map(|&idx| raw.prelit_alphas[idx])
            .sum::<f32>()
            / count;
        raw.prelit_alphas.push(value);
    }
    if raw.night_prelit_colors.len() == n {
        let mut acc = V3::default();
        for &idx in indices {
            let value = raw.night_prelit_colors[idx];
            acc.x += value.x;
            acc.y += value.y;
            acc.z += value.z;
        }
        raw.night_prelit_colors.push(V3 {
            x: acc.x / count,
            y: acc.y / count,
            z: acc.z / count,
        });
    }
    if raw.night_prelit_alphas.len() == n {
        let value = indices
            .iter()
            .map(|&idx| raw.night_prelit_alphas[idx])
            .sum::<f32>()
            / count;
        raw.night_prelit_alphas.push(value);
    }
    if raw.light_flags.len() == n {
        let flag = indices.iter().any(|&idx| raw.light_flags[idx]);
        raw.light_flags.push(flag);
    }
    let mut pos = V3::default();
    for &idx in indices {
        let value = raw.vertices[idx];
        pos.x += value.x;
        pos.y += value.y;
        pos.z += value.z;
    }
    raw.vertices.push(V3 {
        x: pos.x / count,
        y: pos.y / count,
        z: pos.z / count,
    });
    raw.vertices.len() - 1
}

/// Append an exact copy of vertex `src`, keeping attribute arrays in sync.
fn raw_append_vertex_copy(raw: &mut RawMesh, src: usize) -> usize {
    let n = raw.vertices.len();
    if raw.normals.len() == n {
        let value = raw.normals[src];
        raw.normals.push(value);
    }
    if raw.uvs.len() == n {
        let value = raw.uvs[src];
        raw.uvs.push(value);
    }
    for uvs in &mut raw.secondary_uvs {
        if uvs.len() == n {
            uvs.push(uvs[src]);
        }
    }
    if raw.prelit_colors.len() == n {
        let value = raw.prelit_colors[src];
        raw.prelit_colors.push(value);
    }
    if raw.prelit_alphas.len() == n {
        raw.prelit_alphas.push(raw.prelit_alphas[src]);
    }
    if raw.night_prelit_colors.len() == n {
        let value = raw.night_prelit_colors[src];
        raw.night_prelit_colors.push(value);
    }
    if raw.night_prelit_alphas.len() == n {
        raw.night_prelit_alphas.push(raw.night_prelit_alphas[src]);
    }
    if raw.light_flags.len() == n {
        let value = raw.light_flags[src];
        raw.light_flags.push(value);
    }
    let value = raw.vertices[src];
    raw.vertices.push(value);
    raw.vertices.len() - 1
}

/// Split each selected face into three by inserting its centroid.
pub(crate) fn editing_subdivide_selected_dff_faces(app: &mut AppState) -> bool {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return false;
    };
    let selected = dff_selected_face_set(dff);
    if selected.is_empty() {
        app.status_message = "Pick DFF face(s) to subdivide".to_string();
        return false;
    }
    let source = dff.raw.triangles.clone();
    let mut new_triangles = Vec::with_capacity(source.len() + selected.len() * 2);
    let mut new_faces = BTreeSet::new();
    let mut subdivided = 0usize;
    for (idx, tri) in source.into_iter().enumerate() {
        let indices = [tri.a as usize, tri.b as usize, tri.c as usize];
        if !selected.contains(&idx) || indices.iter().any(|&i| i >= dff.raw.vertices.len()) {
            new_triangles.push(tri);
            continue;
        }
        let centroid = raw_append_vertex_average(&mut dff.raw, &indices) as u32;
        for (a, b) in [(tri.a, tri.b), (tri.b, tri.c), (tri.c, tri.a)] {
            new_faces.insert(new_triangles.len());
            new_triangles.push(Tri {
                a,
                b,
                c: centroid,
                material: tri.material,
            });
        }
        subdivided += 1;
    }
    if subdivided == 0 {
        app.status_message = "Selected DFF face(s) no longer exist".to_string();
        return false;
    }
    dff.raw.triangles = new_triangles;
    recalc_raw_normals(&mut dff.raw);
    dff.selected_face = None;
    dff.selected_faces = new_faces;
    dff.selected_edges.clear();
    dff.selected_vertex = None;
    dff.selected_vertices.clear();
    dff.dirty = true;
    app.status_message = format!(
        "Subdivided {subdivided} DFF face(s) into {}",
        subdivided * 3
    );
    refresh_editing_dff_preview(app);
    true
}

/// Scale and/or rotate the UVs of the current selection around its UV centroid.
/// Scope falls back from selected vertices, to selected faces, to the selected
/// material.
pub(crate) fn editing_transform_selected_dff_uvs(
    app: &mut AppState,
    scale: f32,
    rotate_deg: f32,
) -> bool {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return false;
    };
    let mut vertices = dff_selected_vertex_set(dff);
    if vertices.is_empty() {
        for idx in dff_selected_face_set(dff) {
            if let Some(indices) = raw_triangle_indices(&dff.raw, idx) {
                vertices.extend(indices);
            }
        }
    }
    if vertices.is_empty() {
        for tri in &dff.raw.triangles {
            if tri.material as usize == dff.selected_material {
                vertices.insert(tri.a as usize);
                vertices.insert(tri.b as usize);
                vertices.insert(tri.c as usize);
            }
        }
    }
    if vertices.is_empty() {
        app.status_message = "Select DFF vertices, faces, or a material first".to_string();
        return false;
    }
    ensure_raw_uvs(&mut dff.raw);
    let mut cu = 0.0f32;
    let mut cv = 0.0f32;
    let mut count = 0.0f32;
    for &idx in &vertices {
        if let Some(uv) = dff.raw.uvs.get(idx) {
            cu += uv.u;
            cv += uv.v;
            count += 1.0;
        }
    }
    if count <= 0.0 {
        app.status_message = "Selected DFF vertices have no UVs".to_string();
        return false;
    }
    cu /= count;
    cv /= count;
    let (sin, cos) = rotate_deg.to_radians().sin_cos();
    let mut changed = 0usize;
    for idx in vertices {
        if let Some(uv) = dff.raw.uvs.get_mut(idx) {
            let du = uv.u - cu;
            let dv = uv.v - cv;
            uv.u = cu + (du * cos - dv * sin) * scale;
            uv.v = cv + (du * sin + dv * cos) * scale;
            changed += 1;
        }
    }
    dff.dirty = true;
    app.status_message = if rotate_deg.abs() > f32::EPSILON {
        format!("Rotated {changed} DFF UV(s) by {rotate_deg:.0} deg")
    } else {
        format!("Scaled {changed} DFF UV(s) x{scale:.2}")
    };
    refresh_editing_dff_preview(app);
    true
}

/// Duplicate the selected faces with copied vertices so the copy can be moved
/// independently. The duplicates become the new selection.
pub(crate) fn editing_duplicate_selected_dff_faces(app: &mut AppState) -> bool {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return false;
    };
    let selected = dff_selected_face_set(dff);
    if selected.is_empty() {
        app.status_message = "Pick DFF face(s) to duplicate".to_string();
        return false;
    }
    let tris = selected
        .iter()
        .filter_map(|&idx| dff.raw.triangles.get(idx).copied())
        .collect::<Vec<_>>();
    let mut remap = BTreeMap::<usize, usize>::new();
    let mut new_faces = BTreeSet::new();
    for tri in tris {
        let src = [tri.a as usize, tri.b as usize, tri.c as usize];
        if src.iter().any(|&idx| idx >= dff.raw.vertices.len()) {
            continue;
        }
        let mut dst = [0u32; 3];
        for (slot, source) in dst.iter_mut().zip(src) {
            let raw = &mut dff.raw;
            let new_idx = *remap
                .entry(source)
                .or_insert_with(|| raw_append_vertex_copy(raw, source));
            *slot = new_idx as u32;
        }
        new_faces.insert(dff.raw.triangles.len());
        dff.raw.triangles.push(Tri {
            a: dst[0],
            b: dst[1],
            c: dst[2],
            material: tri.material,
        });
    }
    if new_faces.is_empty() {
        app.status_message = "Selected DFF face(s) no longer exist".to_string();
        return false;
    }
    let duplicated = new_faces.len();
    dff.selected_face = None;
    dff.selected_faces = new_faces;
    dff.selected_edges.clear();
    dff.selected_vertex = None;
    dff.selected_vertices = remap.values().copied().collect();
    dff.dirty = true;
    app.status_message = format!("Duplicated {duplicated} DFF face(s)");
    refresh_editing_dff_preview(app);
    true
}

const DFF_GEOMETRY_PLACEMENT_BATCH: usize = 256;

#[derive(Clone, Copy, PartialEq, Eq)]
enum DffGeometryOperationKind {
    Separate,
    Pivot,
}

#[derive(Clone, Copy)]
enum FractureGenerationMode {
    Automatic,
    ManualZone,
}

struct FractureGenerationResult {
    asset_name: String,
    component_index: usize,
    breakable: BreakableGeometry,
    mode: FractureGenerationMode,
    selected_faces: usize,
}

pub(crate) struct FractureGenerationJob {
    rx: mpsc::Receiver<Result<FractureGenerationResult, String>>,
    before: EditingHistorySnapshot,
    started_at: Instant,
}

fn fracture_component_for_faces(
    raw: &RawMesh,
    selected_faces: &BTreeSet<usize>,
) -> Result<usize, String> {
    if raw.components.is_empty() {
        return if raw.triangles.is_empty() {
            Err("the DFF has no geometry".to_string())
        } else {
            Ok(0)
        };
    }
    let mut owners = selected_faces
        .iter()
        .filter_map(|face| {
            raw.components.iter().position(|component| {
                *face >= component.tri_start.min(raw.triangles.len())
                    && *face < component.tri_end.min(raw.triangles.len())
            })
        })
        .collect::<BTreeSet<_>>();
    if owners.len() > 1 {
        return Err("select faces from only one DFF geometry".to_string());
    }
    if let Some(owner) = owners.pop_first() {
        return Ok(owner);
    }
    Ok(0)
}

fn fracture_component_for_dff(dff: &EditingDffState) -> Result<usize, String> {
    fracture_component_for_faces(&dff.raw, &dff_selected_face_set(dff))
}

fn start_fracture_generation(app: &mut AppState, mode: FractureGenerationMode) -> bool {
    if app.fracture_generation_job.is_some()
        || app.dff_geometry_job.is_some()
        || other_dff_asset_writer_active(app)
    {
        app.status_message =
            "Wait for the current background asset operation to finish".to_string();
        return false;
    }
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_ref() else {
        return false;
    };
    let frame = dff_name_stem(&dff.name);
    if !raw_mesh_is_safe_for_normalized_rewrite(&dff.raw, frame) {
        app.status_message =
            "Fracture authoring currently requires a single static DFF geometry; this asset has a frame/component hierarchy"
                .to_string();
        return false;
    }
    let selected_faces = dff_selected_face_set(dff);
    if matches!(mode, FractureGenerationMode::ManualZone) && selected_faces.is_empty() {
        app.status_message =
            "Select one or more faces, then create a manual fracture zone".to_string();
        return false;
    }
    let component_index = match fracture_component_for_faces(&dff.raw, &selected_faces) {
        Ok(component) => component,
        Err(err) => {
            app.status_message = err;
            return false;
        }
    };
    let raw = dff.raw.clone();
    let asset_name = dff.name.clone();
    let before = editing_history_snapshot(app);
    let face_count = selected_faces.len();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = std::panic::catch_unwind(move || {
            let breakable = match mode {
                FractureGenerationMode::Automatic => {
                    generate_breakable_geometry(&raw, component_index)?
                }
                FractureGenerationMode::ManualZone => {
                    assign_faces_to_new_fracture_zone(&raw, component_index, &selected_faces)?
                }
            };
            Ok(FractureGenerationResult {
                asset_name,
                component_index,
                breakable,
                mode,
                selected_faces: face_count,
            })
        })
        .unwrap_or_else(|_| Err("fracture generation worker crashed".to_string()));
        let _ = tx.send(result);
    });
    app.fracture_generation_job = Some(FractureGenerationJob {
        rx,
        before,
        started_at: Instant::now(),
    });
    app.status_message = match mode {
        FractureGenerationMode::Automatic => {
            "Generating connected fracture zones in the background...".to_string()
        }
        FractureGenerationMode::ManualZone => {
            format!("Creating a manual fracture zone from {face_count} selected face(s)...")
        }
    };
    true
}

pub(crate) fn request_automatic_fracture_generation(app: &mut AppState) -> bool {
    start_fracture_generation(app, FractureGenerationMode::Automatic)
}

pub(crate) fn request_manual_fracture_zone(app: &mut AppState) -> bool {
    start_fracture_generation(app, FractureGenerationMode::ManualZone)
}

pub(crate) fn editing_dff_fracture_preview_physics(
    app: &AppState,
    dff_name: &str,
) -> (Vec3, f32, Option<u16>) {
    let selected_definition = app
        .placements
        .get(app.selected)
        .and_then(|placement| app.definitions.get(&placement.id))
        .filter(|definition| definition_uses_dff(definition, dff_name));
    let definition = selected_definition.or_else(|| {
        app.definitions
            .values()
            .find(|definition| definition_uses_dff(definition, dff_name))
    });
    let root = definition.and_then(|definition| physics_root_value_from_attrs(&definition.attrs));
    let properties = root.and_then(|root| {
        app.physics_root_properties
            .get(&root)
            .or_else(|| physics_root_spec(root).map(|spec| &spec.fallback))
    });
    if let Some(properties) = properties.filter(|properties| properties.is_breakable()) {
        return (
            to_mq(properties.break_velocity),
            properties.break_velocity_randomness.max(0.0),
            root,
        );
    }

    // This matches the common SA lamp, gate, and road-barrier entries. Wood
    // fences differ only by a tiny 0.01 velocity on each axis.
    (vec3(0.0, 0.0, 0.1), 0.07, None)
}

pub(crate) fn toggle_editing_dff_fracture_preview(app: &mut AppState) -> bool {
    let now = get_time();
    let dff_name = match app.editing.asset.as_ref() {
        Some(EditingAsset::Dff(dff)) => dff.name.clone(),
        _ => return false,
    };
    let (_, _, physics_root) = editing_dff_fracture_preview_physics(app, &dff_name);
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return false;
    };

    if let Some(started_at) = dff.fracture_preview_started_at {
        if now - started_at >= FRACTURE_PREVIEW_DURATION_SECONDS {
            dff.fracture_preview_started_at = Some(now);
            app.status_message = "Replaying the breakable-prop preview".to_string();
        } else {
            dff.fracture_preview_started_at = None;
            app.status_message = "Reset the breakable-prop preview".to_string();
        }
        return true;
    }

    let mut zone_count = 0usize;
    for breakable in dff
        .raw
        .components
        .iter()
        .filter_map(|component| component.breakable.as_ref())
    {
        if breakable.stale {
            app.status_message =
                "Fracture data is stale. Regenerate or clear it before simulating.".to_string();
            return false;
        }
        let errors = validate_breakable_geometry(breakable);
        if !errors.is_empty() {
            app.status_message = format!("Cannot simulate invalid fracture data: {}", errors[0]);
            return false;
        }
        zone_count += breakable.groups.len();
    }
    if zone_count == 0 {
        app.status_message =
            "Generate fracture zones before simulating the prop breaking apart".to_string();
        return false;
    }

    dff.fracture_preview_started_at = Some(now);
    let source = physics_root
        .map(|root| format!(" using {}", physics_root_label(Some(root))))
        .unwrap_or_else(|| " using SA's common break velocity".to_string());
    app.status_message = format!(
        "Simulating {zone_count} falling fracture zone(s){source}. Use Reset Preview to restore the intact prop."
    );
    true
}

pub(crate) fn update_fracture_generation_job(app: &mut AppState) {
    let Some(job) = app.fracture_generation_job.take() else {
        return;
    };
    match job.rx.try_recv() {
        Ok(Ok(result)) => {
            let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
                app.status_message =
                    "Fracture generation finished, but the DFF editor was closed".to_string();
                return;
            };
            if asset_key(&dff.name, ".dff") != asset_key(&result.asset_name, ".dff") {
                app.status_message =
                    "Fracture generation finished for a DFF that is no longer open".to_string();
                return;
            }
            let Some(component) = dff.raw.components.get_mut(result.component_index) else {
                app.status_message =
                    "Fracture generation finished after the target geometry changed".to_string();
                return;
            };
            let zone_count = result.breakable.groups.len();
            component.breakable = Some(result.breakable);
            dff.selected_breakable_group = zone_count.saturating_sub(1);
            dff.dirty = true;
            commit_editing_history(
                app,
                match result.mode {
                    FractureGenerationMode::Automatic => "Generate Fracture Zones",
                    FractureGenerationMode::ManualZone => "Create Manual Fracture Zone",
                },
                job.before,
            );
            let elapsed = job.started_at.elapsed().as_secs_f32();
            app.status_message = match result.mode {
                FractureGenerationMode::Automatic => format!(
                    "Generated {zone_count} fracture zone(s) in {elapsed:.2}s. Stage DFF, then Save to apply."
                ),
                FractureGenerationMode::ManualZone => format!(
                    "Defined a manual fracture zone from {} face(s); {zone_count} total zone(s) in {elapsed:.2}s.",
                    result.selected_faces
                ),
            };
        }
        Ok(Err(err)) => {
            app.status_message = format!("Fracture generation failed: {err}");
        }
        Err(mpsc::TryRecvError::Empty) => {
            app.fracture_generation_job = Some(job);
        }
        Err(mpsc::TryRecvError::Disconnected) => {
            app.status_message = "Fracture generation worker disconnected".to_string();
        }
    }
}

pub(crate) fn clear_editing_dff_fractures(app: &mut AppState) -> bool {
    let component_index = match app.editing.asset.as_ref() {
        Some(EditingAsset::Dff(dff)) => match fracture_component_for_dff(dff) {
            Ok(index) => index,
            Err(err) => {
                app.status_message = err;
                return false;
            }
        },
        _ => return false,
    };
    let before = editing_history_snapshot(app);
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return false;
    };
    let Some(component) = dff.raw.components.get_mut(component_index) else {
        return false;
    };
    if component.breakable.take().is_none() {
        app.status_message = "This geometry has no fracture zones".to_string();
        return false;
    }
    dff.selected_breakable_group = 0;
    dff.dirty = true;
    commit_editing_history(app, "Clear Fracture Zones", before);
    app.status_message = "Cleared fracture zones from this geometry".to_string();
    true
}

pub(crate) fn toggle_editing_dff_fracture_origin(app: &mut AppState) -> bool {
    let component_index = match app.editing.asset.as_ref() {
        Some(EditingAsset::Dff(dff)) => match fracture_component_for_dff(dff) {
            Ok(index) => index,
            Err(err) => {
                app.status_message = err;
                return false;
            }
        },
        _ => return false,
    };
    let before = editing_history_snapshot(app);
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return false;
    };
    let Some(breakable) = dff
        .raw
        .components
        .get_mut(component_index)
        .and_then(|component| component.breakable.as_mut())
    else {
        app.status_message = "Generate fracture zones before changing their origin".to_string();
        return false;
    };
    breakable.origin = match breakable.origin {
        BreakableOrigin::Object => BreakableOrigin::Collision,
        BreakableOrigin::Collision => BreakableOrigin::Object,
    };
    let label = match breakable.origin {
        BreakableOrigin::Object => "Object",
        BreakableOrigin::Collision => "Collision",
    };
    dff.dirty = true;
    commit_editing_history(app, "Change Fracture Origin", before);
    app.status_message = format!("Fracture origin: {label}");
    true
}

pub(crate) fn select_editing_dff_fracture_zone(app: &mut AppState, group: usize) -> bool {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return false;
    };
    let component_index = match fracture_component_for_dff(dff) {
        Ok(index) => index,
        Err(err) => {
            app.status_message = err;
            return false;
        }
    };
    let Some(breakable) = dff
        .raw
        .components
        .get(component_index)
        .and_then(|component| component.breakable.as_ref())
    else {
        return false;
    };
    if group >= breakable.groups.len() {
        return false;
    }
    let faces = breakable
        .triangles
        .iter()
        .filter(|triangle| triangle.group as usize == group)
        .filter_map(|triangle| triangle.source_face)
        .collect::<BTreeSet<_>>();
    dff.selected_breakable_group = group;
    if faces.is_empty() {
        app.status_message =
            "This imported zone does not map to intact faces; regenerate to edit it".to_string();
        return true;
    }
    dff.select_mode = EditingSelectMode::Face;
    dff.selected_faces = faces;
    dff.selected_face = dff.selected_faces.iter().next_back().copied();
    dff.selected_edges.clear();
    dff.selected_vertex = None;
    dff.selected_vertices.clear();
    app.status_message = format!(
        "Selected fracture zone {} ({} intact face(s))",
        group + 1,
        dff.selected_faces.len()
    );
    true
}

enum DffGeometryPlacementChange {
    Add {
        placement: Placement,
        state: ElementState,
    },
    Move {
        index: usize,
        position: V3,
    },
}

struct DffGeometryResult {
    kind: DffGeometryOperationKind,
    source_name: String,
    source_raw: RawMesh,
    source_bytes: Vec<u8>,
    new_asset: Option<(String, RawMesh, Vec<u8>)>,
    definitions: Vec<(String, Definition)>,
    placement_changes: Vec<DffGeometryPlacementChange>,
    face_count: usize,
    pivot: Option<V3>,
}

pub(crate) struct DffGeometryJob {
    rx: mpsc::Receiver<Result<DffGeometryResult, String>>,
    result: Option<DffGeometryResult>,
    base_applied: bool,
    placement_index: usize,
    refresh_index: usize,
    scene_rebuilt: bool,
    started_at: Instant,
}

fn selected_dff_geometry_vertices(dff: &EditingDffState) -> BTreeSet<usize> {
    let mut selected = dff_selected_vertex_set(dff);
    for face in dff_selected_face_set(dff) {
        if let Some(tri) = dff.raw.triangles.get(face) {
            selected.extend([tri.a as usize, tri.b as usize, tri.c as usize]);
        }
    }
    for (a, b) in dff_selected_edge_set(dff) {
        selected.extend([a, b]);
    }
    selected
}

fn selected_dff_geometry_pivot(dff: &EditingDffState) -> Option<V3> {
    let selected = selected_dff_geometry_vertices(dff);
    let mut sum = Vec3::ZERO;
    let mut count = 0.0f32;
    for index in selected {
        if let Some(vertex) = dff.raw.vertices.get(index) {
            sum += to_mq(*vertex);
            count += 1.0;
        }
    }
    (count > 0.0).then(|| from_mq(sum / count))
}

fn dff_bounds_pivot(dff: &EditingDffState) -> Option<V3> {
    (!dff.raw.vertices.is_empty()).then(|| {
        let bounds = bounds_from_vertices(&dff.raw.vertices);
        from_mq((bounds.min + bounds.max) * 0.5)
    })
}

fn dff_name_stem(name: &str) -> &str {
    Path::new(name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(name)
}

fn validate_separated_dff_name(name: &str) -> Result<(String, String), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Enter a name for the separated object".to_string());
    }
    if trimmed.contains('/') || trimmed.contains('\\') {
        return Err("Object name cannot contain a path".to_string());
    }
    let stem = dff_name_stem(trimmed).trim();
    if stem.is_empty()
        || !stem
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
    {
        return Err("Use only letters, numbers, underscores, and hyphens".to_string());
    }
    let asset_name = format!("{stem}.dff");
    if asset_name.as_bytes().len() > IMG_RUNTIME_SAFE_ENTRY_NAME_BYTES {
        return Err(
            "DFF name must leave room for a terminator in the 24-byte IMG directory field"
                .to_string(),
        );
    }
    Ok((asset_name, stem.to_string()))
}

fn unique_separated_definition_id(
    reserved: &mut HashSet<String>,
    requested: &str,
    source_id: &str,
    first: bool,
) -> String {
    let base = if first {
        requested.to_string()
    } else {
        format!("{requested}_{source_id}")
    };
    if reserved.insert(base.clone()) {
        return base;
    }
    for suffix in 2usize.. {
        let candidate = format!("{base}_{suffix}");
        if reserved.insert(candidate.clone()) {
            return candidate;
        }
    }
    unreachable!()
}

fn shift_raw_mesh_pivot(raw: &mut RawMesh, pivot: V3) {
    for vertex in &mut raw.vertices {
        vertex.x -= pivot.x;
        vertex.y -= pivot.y;
        vertex.z -= pivot.z;
    }
    for effect in &mut raw.effects_2dfx {
        effect.position.x -= pivot.x;
        effect.position.y -= pivot.y;
        effect.position.z -= pivot.z;
    }
    for breakable in raw
        .components
        .iter_mut()
        .filter_map(|component| component.breakable.as_mut())
    {
        for vertex in &mut breakable.vertices {
            vertex.position.x -= pivot.x;
            vertex.position.y -= pivot.y;
            vertex.position.z -= pivot.z;
        }
    }
}

fn split_raw_mesh_faces(
    source: &RawMesh,
    selected_faces: &BTreeSet<usize>,
) -> Result<(RawMesh, RawMesh), String> {
    if selected_faces.is_empty() {
        return Err("Select one or more DFF faces to separate".to_string());
    }
    if selected_faces.len() >= source.triangles.len() {
        return Err("Leave at least one face in the original object".to_string());
    }
    if selected_faces
        .iter()
        .any(|index| *index >= source.triangles.len())
    {
        return Err("The selected DFF faces changed before separation".to_string());
    }

    let mut separated = source.clone();
    separated.triangles = source
        .triangles
        .iter()
        .enumerate()
        .filter(|(index, _)| selected_faces.contains(index))
        .map(|(_, triangle)| *triangle)
        .collect();
    // 2DFX effects belong to the source object's authored frame rather than
    // to an individual triangle, so do not silently duplicate them.
    separated.effects_2dfx.clear();
    compact_raw_vertices(&mut separated);
    recalc_raw_normals(&mut separated);

    let mut remaining = source.clone();
    remaining.triangles = source
        .triangles
        .iter()
        .enumerate()
        .filter(|(index, _)| !selected_faces.contains(index))
        .map(|(_, triangle)| *triangle)
        .collect();
    compact_raw_vertices(&mut remaining);
    recalc_raw_normals(&mut remaining);
    Ok((remaining, separated))
}

fn other_dff_asset_writer_active(app: &AppState) -> bool {
    app.editing.save_rx.is_some()
        || app.dff_repair_rx.is_some()
        || app.txd_cleanup_job.is_some()
        || app.asset_optimization_scan_rx.is_some()
        || app.asset_optimization_job.is_some()
        || app.object_bounds_fix_job.is_some()
        || app.corona_generation_job.is_some()
        || app.fracture_generation_job.is_some()
        || app.collision_generation_job.is_some()
        || app.shadow_mesh_generation_job.is_some()
        || app.lod_generation_job.is_some()
        || app.instance_lod_removal_job.is_some()
}

fn start_dff_separation_worker(app: &mut AppState, new_name: String, new_stem: String) -> bool {
    if app.dff_geometry_job.is_some() || other_dff_asset_writer_active(app) {
        app.status_message =
            "Wait for the current background asset operation to finish".to_string();
        return false;
    }
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_ref() else {
        return false;
    };
    let selected_faces = dff_selected_face_set(dff);
    if selected_faces.is_empty() {
        app.status_message = "Select one or more DFF faces to separate".to_string();
        return false;
    }
    if selected_faces.len() >= dff.raw.triangles.len() {
        app.status_message =
            "Leave at least one face in the original object when separating".to_string();
        return false;
    }
    let source_name = asset_key(&dff.name, ".dff");
    let source_key = asset_key(&source_name, ".dff");
    let source_raw = dff.raw.clone();
    let options = dff_write_options_for_asset(app, &source_name);
    let root = app.root.clone();
    let placements = app.placements.clone();
    let element_states = app.element_states.clone();
    let definitions = app.definitions.clone();
    let reserved_asset_names = app
        .pending_replacement_assets
        .keys()
        .chain(app.editing.modified_entries.keys())
        .chain(app.editing.rows.iter().map(|row| &row.entry.name))
        .map(|name| asset_key(name, ".dff"))
        .collect::<HashSet<_>>();
    let status_name = new_name.clone();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = std::panic::catch_unwind(|| {
            let new_key = asset_key(&new_name, ".dff");
            if new_key == source_key
                || reserved_asset_names.contains(&new_key)
                || find_dff_entry(&root, &new_name).is_some()
            {
                return Err(format!("A DFF named {new_name} already exists"));
            }

            let (remaining, separated) = split_raw_mesh_faces(&source_raw, &selected_faces)?;

            let source_bytes = write_normalized_dff_with_options(
                &remaining,
                dff_name_stem(&source_name),
                options,
            )?;
            let new_bytes = write_normalized_dff_with_options(&separated, &new_stem, options)?;

            let mut reserved_ids = definitions.keys().cloned().collect::<HashSet<_>>();
            reserved_ids.extend(placements.iter().map(|placement| placement.id.clone()));
            let mut definition_ids = HashMap::<String, String>::new();
            let mut new_definitions = Vec::new();
            let mut placement_changes = Vec::new();
            for (index, placement) in placements.iter().enumerate() {
                if asset_key(&placement.dff, ".dff") != source_key {
                    continue;
                }
                let next_id = if let Some(existing) = definition_ids.get(&placement.id) {
                    existing.clone()
                } else {
                    let id = unique_separated_definition_id(
                        &mut reserved_ids,
                        &new_stem,
                        &placement.id,
                        definition_ids.is_empty(),
                    );
                    let mut definition =
                        definitions
                            .get(&placement.id)
                            .cloned()
                            .unwrap_or_else(|| Definition {
                                id: placement.id.clone(),
                                zone: placement.zone.clone(),
                                attrs: BTreeMap::new(),
                            });
                    definition.id = id.clone();
                    definition.attrs.insert("id".to_string(), id.clone());
                    definition.attrs.insert("dff".to_string(), new_stem.clone());
                    // The source COL describes the unsplit object. Reusing it on
                    // the separated object would duplicate the full collision.
                    definition.attrs.remove("col");
                    new_definitions.push((id.clone(), definition));
                    definition_ids.insert(placement.id.clone(), id.clone());
                    id
                };
                let mut clone = placement.clone();
                clone.id = next_id;
                clone.dff = new_stem.clone();
                sync_placement_attrs(&mut clone);
                placement_changes.push(DffGeometryPlacementChange::Add {
                    placement: clone,
                    state: element_states.get(index).copied().unwrap_or_default(),
                });
            }
            if placement_changes.is_empty() {
                return Err(format!(
                    "No map placements reference {}; open the DFF from a placed object",
                    source_name
                ));
            }
            Ok(DffGeometryResult {
                kind: DffGeometryOperationKind::Separate,
                source_name,
                source_raw: remaining,
                source_bytes,
                new_asset: Some((new_name, separated, new_bytes)),
                definitions: new_definitions,
                placement_changes,
                face_count: selected_faces.len(),
                pivot: None,
            })
        })
        .unwrap_or_else(|_| Err("DFF separation worker crashed".to_string()));
        let _ = tx.send(result);
    });
    app.dff_geometry_job = Some(DffGeometryJob {
        rx,
        result: None,
        base_applied: false,
        placement_index: 0,
        refresh_index: 0,
        scene_rebuilt: false,
        started_at: Instant::now(),
    });
    app.status_message = format!("Separating selected geometry into {status_name}...");
    true
}

fn start_dff_pivot_worker(app: &mut AppState, pivot: V3) -> bool {
    if app.dff_geometry_job.is_some() || other_dff_asset_writer_active(app) {
        app.status_message =
            "Wait for the current background asset operation to finish".to_string();
        return false;
    }
    if !pivot.x.is_finite() || !pivot.y.is_finite() || !pivot.z.is_finite() {
        app.status_message = "Pivot coordinates must be finite".to_string();
        return false;
    }
    if pivot.x.abs() + pivot.y.abs() + pivot.z.abs() <= 0.000001 {
        app.status_message = "The chosen pivot is already at the object origin".to_string();
        return false;
    }
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_ref() else {
        return false;
    };
    let source_name = asset_key(&dff.name, ".dff");
    let source_key = asset_key(&source_name, ".dff");
    let mut source_raw = dff.raw.clone();
    let options = dff_write_options_for_asset(app, &source_name);
    let placements = app.placements.clone();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = std::panic::catch_unwind(move || {
            shift_raw_mesh_pivot(&mut source_raw, pivot);
            let source_bytes = write_normalized_dff_with_options(
                &source_raw,
                dff_name_stem(&source_name),
                options,
            )?;
            let mut placement_changes = Vec::new();
            for (index, placement) in placements.iter().enumerate() {
                if asset_key(&placement.dff, ".dff") != source_key {
                    continue;
                }
                let offset = placement_matrix(placement).transform_vector3(to_mq(pivot));
                placement_changes.push(DffGeometryPlacementChange::Move {
                    index,
                    position: from_mq(to_mq(placement.pos) + offset),
                });
            }
            if placement_changes.is_empty() {
                return Err(format!(
                    "No map placements reference {}; open the DFF from a placed object",
                    source_name
                ));
            }
            Ok(DffGeometryResult {
                kind: DffGeometryOperationKind::Pivot,
                source_name,
                source_raw,
                source_bytes,
                new_asset: None,
                definitions: Vec::new(),
                placement_changes,
                face_count: 0,
                pivot: Some(pivot),
            })
        })
        .unwrap_or_else(|_| Err("DFF pivot worker crashed".to_string()));
        let _ = tx.send(result);
    });
    app.dff_geometry_job = Some(DffGeometryJob {
        rx,
        result: None,
        base_applied: false,
        placement_index: 0,
        refresh_index: 0,
        scene_rebuilt: false,
        started_at: Instant::now(),
    });
    app.status_message = format!(
        "Adjusting DFF pivot to {:.3}, {:.3}, {:.3}...",
        pivot.x, pivot.y, pivot.z
    );
    true
}

pub(crate) fn open_dff_separate_dialog(app: &mut AppState) -> bool {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_ref() else {
        return false;
    };
    let selected = dff_selected_face_set(dff);
    if selected.is_empty() {
        app.status_message = "Select one or more DFF faces to separate".to_string();
        return false;
    }
    if selected.len() >= dff.raw.triangles.len() {
        app.status_message =
            "Leave at least one face in the original object when separating".to_string();
        return false;
    }
    let stem = dff_name_stem(&dff.name);
    let suffix = "_part";
    let keep = 24usize.saturating_sub(suffix.len() + ".dff".len());
    let buffer = format!("{}{}", &stem[..stem.len().min(keep)], suffix);
    let source_name = dff.name.clone();
    drain_text_input();
    app.dff_texture_duplicate_dialog = Some(DffTextureDuplicateDialog {
        action: DffTextureNameAction::SeparateGeometry,
        material: 0,
        source_texture: source_name,
        cursor: buffer.len(),
        selection_anchor: Some(0),
        buffer,
    });
    true
}

pub(crate) fn start_dff_separation(app: &mut AppState, typed_name: &str) -> bool {
    let (new_name, new_stem) = match validate_separated_dff_name(typed_name) {
        Ok(name) => name,
        Err(err) => {
            app.status_message = err;
            return false;
        }
    };
    start_dff_separation_worker(app, new_name, new_stem)
}

pub(crate) fn editing_pivot_to_selection(app: &mut AppState) -> bool {
    let pivot = match app.editing.asset.as_ref() {
        Some(EditingAsset::Dff(dff)) => selected_dff_geometry_pivot(dff),
        _ => None,
    };
    let Some(pivot) = pivot else {
        app.status_message = "Select DFF vertices, edges, or faces for the new pivot".to_string();
        return false;
    };
    start_dff_pivot_worker(app, pivot)
}

pub(crate) fn editing_pivot_to_bounds(app: &mut AppState) -> bool {
    let pivot = match app.editing.asset.as_ref() {
        Some(EditingAsset::Dff(dff)) => dff_bounds_pivot(dff),
        _ => None,
    };
    let Some(pivot) = pivot else {
        app.status_message = "The DFF has no geometry for a bounds pivot".to_string();
        return false;
    };
    start_dff_pivot_worker(app, pivot)
}

impl DffGeometryJob {
    fn refresh_entry(&self, index: usize) -> Option<(String, Vec<u8>)> {
        let Some(result) = self.result.as_ref() else {
            return None;
        };
        if index == 0 {
            return Some((result.source_name.clone(), result.source_bytes.clone()));
        }
        (index == 1)
            .then(|| result.new_asset.as_ref())
            .flatten()
            .map(|(name, _, bytes)| (name.clone(), bytes.clone()))
    }

    fn refresh_count(&self) -> usize {
        self.result
            .as_ref()
            .map(|result| 1 + usize::from(result.new_asset.is_some()))
            .unwrap_or(0)
    }

    fn step(&mut self, app: &mut AppState) -> bool {
        if self.result.is_none() {
            match self.rx.try_recv() {
                Ok(Ok(result)) => self.result = Some(result),
                Ok(Err(err)) => {
                    app.status_message = format!("DFF geometry operation failed: {err}");
                    return true;
                }
                Err(mpsc::TryRecvError::Empty) => return false,
                Err(mpsc::TryRecvError::Disconnected) => {
                    app.status_message = "DFF geometry worker disconnected".to_string();
                    return true;
                }
            }
        }
        let result = self
            .result
            .as_ref()
            .expect("DFF geometry result must be available");
        if !self.base_applied {
            let source_key = editing_key(&result.source_name);
            app.editing
                .modified_entries
                .insert(source_key, result.source_bytes.clone());
            if let Some((name, _, bytes)) = result.new_asset.as_ref() {
                let key = editing_key(name);
                app.editing
                    .modified_entries
                    .insert(key.clone(), bytes.clone());
                app.editing.added_entries.insert(key);
                if let Some(img_path) = app.editing.img_path.clone()
                    && !app
                        .editing
                        .rows
                        .iter()
                        .any(|row| row.entry.name.eq_ignore_ascii_case(name))
                {
                    app.editing.rows.push(EditingImgRow {
                        entry: ImgEntry {
                            img_path,
                            name: name.clone(),
                            offset: 0,
                            size: bytes.len().min(u32::MAX as usize) as u32,
                        },
                        logical_size: replacement_entry_len(name, bytes),
                    });
                }
            }
            for (id, definition) in &result.definitions {
                app.definitions.insert(id.clone(), definition.clone());
            }
            if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut()
                && asset_key(&dff.name, ".dff") == asset_key(&result.source_name, ".dff")
            {
                dff.raw = result.source_raw.clone();
                dff.selected_face = None;
                dff.selected_faces.clear();
                dff.selected_edges.clear();
                dff.selected_vertex = None;
                dff.selected_vertices.clear();
                dff.dirty = false;
                dff.normalized_warning = false;
                dff.normalized_rewrite_confirmed = true;
            }
            self.base_applied = true;
            app.status_message = "Applying DFF placement updates...".to_string();
            return false;
        }

        if self.placement_index < result.placement_changes.len() {
            let end = (self.placement_index + DFF_GEOMETRY_PLACEMENT_BATCH)
                .min(result.placement_changes.len());
            for change in &result.placement_changes[self.placement_index..end] {
                match change {
                    DffGeometryPlacementChange::Add { placement, state } => {
                        app.placements.push(placement.clone());
                        app.element_states.push(*state);
                        app.outliner_labels.push(None);
                    }
                    DffGeometryPlacementChange::Move { index, position } => {
                        if let Some(placement) = app.placements.get_mut(*index) {
                            placement.pos = *position;
                            sync_placement_attrs(placement);
                            invalidate_outliner_label(app, *index);
                        }
                    }
                }
            }
            self.placement_index = end;
            app.status_message = format!(
                "Applying DFF placement updates: {}/{}",
                self.placement_index,
                result.placement_changes.len()
            );
            return false;
        }

        if !self.scene_rebuilt {
            rebuild_outliner_filter(app);
            invalidate_validation_cache(app);
            self.scene_rebuilt = true;
            return false;
        }

        let refresh_count = self.refresh_count();
        if let Some((name, bytes)) = self.refresh_entry(self.refresh_index) {
            refresh_live_asset_from_editing_entry(app, &name, &bytes);
            if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_ref()
                && asset_key(&dff.name, ".dff") == asset_key(&name, ".dff")
            {
                let mesh_key = mesh_key_from_dff_txd(&name, dff.txd_context.as_deref());
                let preview = app.meshes.get(&mesh_key).cloned();
                if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
                    dff.preview_mesh = preview;
                }
            }
            self.refresh_index += 1;
            app.status_message = format!("Refreshing DFF {}/{}", self.refresh_index, refresh_count);
            return false;
        }

        clear_history_for_external_change(app);
        let elapsed = self.started_at.elapsed().as_secs_f32();
        app.status_message = match result.kind {
            DffGeometryOperationKind::Separate => {
                let name = result
                    .new_asset
                    .as_ref()
                    .map(|(name, _, _)| name.as_str())
                    .unwrap_or("new DFF");
                format!(
                    "Separated {} face(s) into {name}; created {} placement(s) in {elapsed:.1}s. Save to apply; undo history cleared.",
                    result.face_count,
                    result.placement_changes.len()
                )
            }
            DffGeometryOperationKind::Pivot => {
                let pivot = result.pivot.unwrap_or_default();
                format!(
                    "Moved pivot to {:.3}, {:.3}, {:.3} and compensated {} placement(s) in {elapsed:.1}s. Save to apply; undo history cleared.",
                    pivot.x,
                    pivot.y,
                    pivot.z,
                    result.placement_changes.len()
                )
            }
        };
        true
    }
}

pub(crate) fn update_dff_geometry_job(app: &mut AppState) {
    let Some(mut job) = app.dff_geometry_job.take() else {
        return;
    };
    if !job.step(app) {
        app.dff_geometry_job = Some(job);
    }
}

pub(crate) fn open_dff_texture_duplicate_dialog(app: &mut AppState) -> bool {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_ref() else {
        return false;
    };
    let material = dff.selected_material;
    let Some(source_texture) = dff.raw.material_textures.get(material).cloned() else {
        app.status_message = "Select a DFF material first".to_string();
        return false;
    };
    if source_texture.trim().is_empty() {
        app.status_message = "The selected material has no texture to duplicate".to_string();
        return false;
    }
    let Some(txd_name) = dff.txd_context.clone() else {
        app.status_message =
            "The selected DFF has no resolved TXD for duplicating its texture".to_string();
        return false;
    };
    let txd_key = asset_key(&txd_name, ".txd");
    let existing = app
        .txd_textures
        .iter()
        .filter(|(_, entries)| {
            entries
                .iter()
                .any(|entry| entry.txd_name.eq_ignore_ascii_case(&txd_key))
        })
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    let buffer = suggested_duplicate_texture_name(&existing, source_texture.trim());
    drain_text_input();
    app.dff_texture_duplicate_dialog = Some(DffTextureDuplicateDialog {
        action: DffTextureNameAction::Duplicate,
        material,
        source_texture,
        cursor: buffer.len(),
        selection_anchor: Some(0),
        buffer,
    });
    true
}

pub(crate) fn open_dff_texture_rename_dialog(app: &mut AppState) -> bool {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_ref() else {
        return false;
    };
    let material = dff.selected_material;
    let Some(source_texture) = dff.raw.material_textures.get(material).cloned() else {
        app.status_message = "Select a DFF material first".to_string();
        return false;
    };
    if source_texture.trim().is_empty() {
        app.status_message = "The selected material has no texture to rename".to_string();
        return false;
    }
    if dff.txd_context.is_none() {
        app.status_message = "The selected DFF has no resolved TXD".to_string();
        return false;
    }
    let buffer = source_texture.clone();
    drain_text_input();
    app.dff_texture_duplicate_dialog = Some(DffTextureDuplicateDialog {
        action: DffTextureNameAction::RenameDff,
        material,
        source_texture,
        cursor: buffer.len(),
        selection_anchor: Some(0),
        buffer,
    });
    true
}

pub(crate) fn open_txd_texture_rename_dialog(app: &mut AppState) -> bool {
    let Some(EditingAsset::Txd(txd)) = app.editing.asset.as_ref() else {
        return false;
    };
    let Some(source_texture) = txd
        .textures
        .get(txd.selected)
        .map(|entry| entry.name.clone())
    else {
        app.status_message = "Select a TXD texture first".to_string();
        return false;
    };
    let buffer = source_texture.clone();
    drain_text_input();
    app.dff_texture_duplicate_dialog = Some(DffTextureDuplicateDialog {
        action: DffTextureNameAction::RenameTxd,
        material: txd.selected,
        source_texture,
        cursor: buffer.len(),
        selection_anchor: Some(0),
        buffer,
    });
    true
}

pub(crate) fn open_dff_texture_view_dialog(app: &mut AppState) -> bool {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_ref() else {
        return false;
    };
    let Some(texture_name) = dff
        .raw
        .material_textures
        .get(dff.selected_material)
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
    else {
        app.status_message = "The selected material has no texture to view".to_string();
        return false;
    };
    let txd_key = dff.txd_context.as_ref().map(|txd| asset_key(txd, ".txd"));
    let source = app
        .txd_textures
        .get(&lower(&texture_name))
        .and_then(|entries| {
            if let Some(txd_key) = txd_key.as_ref() {
                entries
                    .iter()
                    .find(|entry| entry.txd_name.eq_ignore_ascii_case(txd_key))
            } else {
                entries.first()
            }
        });
    let decoded = source
        .and_then(|source| {
            decode_txd_texture(source)
                .map(|(width, height, rgba)| (width, height, rgba, source.txd_name.clone()))
        })
        .or_else(|| {
            let path = app.texture_files.get(&lower(&texture_name))?;
            let bytes = fs::read(path).ok()?;
            let image = image::load_from_memory(&bytes).ok()?.to_rgba8();
            let (width, height) = image.dimensions();
            Some((width, height, image.into_raw(), "Loose PNG".to_string()))
        });
    let Some((width, height, rgba, source_label)) = decoded else {
        app.status_message = if let Some(txd_key) = txd_key {
            format!("Could not decode texture '{texture_name}' from {txd_key}")
        } else {
            format!("Could not decode texture '{texture_name}'")
        };
        return false;
    };
    let Ok(width_u16) = u16::try_from(width) else {
        app.status_message = format!("Texture '{texture_name}' is too wide to preview");
        return false;
    };
    let Ok(height_u16) = u16::try_from(height) else {
        app.status_message = format!("Texture '{texture_name}' is too tall to preview");
        return false;
    };
    let texture = Texture2D::from_rgba8(width_u16, height_u16, &rgba);
    texture.set_filter(FilterMode::Nearest);
    app.dff_texture_view_dialog = Some(DffTextureViewDialog {
        texture_name,
        txd_name: source_label,
        width: width_u16,
        height: height_u16,
        texture: Some(texture),
        raw_texture: 0,
    });
    true
}

fn suggested_duplicate_texture_name(existing: &[String], source: &str) -> String {
    let stem = sanitize_texture_name(source);
    for number in 1usize.. {
        let suffix = if number == 1 {
            "_copy".to_string()
        } else {
            format!("_copy{number}")
        };
        let keep = GTA_SA_TEXTURE_NAME_MAX.saturating_sub(suffix.len());
        let candidate = format!("{}{}", &stem[..stem.len().min(keep)], suffix);
        if !existing
            .iter()
            .any(|name| name.eq_ignore_ascii_case(&candidate))
        {
            return candidate;
        }
    }
    unreachable!()
}

pub(crate) fn editing_duplicate_selected_dff_texture(
    app: &mut AppState,
    material: usize,
    new_texture_name: &str,
) -> bool {
    let new_texture_name = sanitize_texture_name(new_texture_name);
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_ref() else {
        return false;
    };
    if material >= dff.raw.material_textures.len() {
        app.status_message = "The selected DFF material no longer exists".to_string();
        return false;
    }
    let source_texture = dff.raw.material_textures[material].clone();
    if new_texture_name.eq_ignore_ascii_case(&source_texture) {
        app.status_message =
            format!("Texture name '{new_texture_name}' must differ from the source");
        return false;
    }
    let Some(txd_name) = dff.txd_context.clone() else {
        app.status_message = "The selected DFF has no resolved TXD".to_string();
        return false;
    };
    let txd_key = asset_key(&txd_name, ".txd");
    if app
        .txd_textures
        .get(&lower(&new_texture_name))
        .is_some_and(|entries| {
            entries
                .iter()
                .any(|entry| entry.txd_name.eq_ignore_ascii_case(&txd_key))
        })
    {
        app.status_message = format!("Texture '{new_texture_name}' already exists in {txd_key}");
        return false;
    }
    let native = match source_texture_native(app, &source_texture, &txd_name) {
        Ok(native) => native,
        Err(err) => {
            app.status_message = format!("Could not duplicate texture: {err}");
            return false;
        }
    };
    if let Err(err) = stage_texture_native_into_txd(app, &txd_name, &new_texture_name, &native) {
        app.status_message = format!("Could not duplicate texture: {err}");
        return false;
    }
    app.status_message = format!(
        "Duplicated texture '{source_texture}' as '{new_texture_name}' in {txd_key}; materials unchanged"
    );
    true
}

fn dff_names_using_txd(app: &AppState, txd_name: &str) -> BTreeSet<String> {
    let txd_key = asset_key(txd_name, ".txd");
    let mut names = app
        .definitions
        .values()
        .filter(|definition| {
            definition_txd_name_from_attrs(definition)
                .is_some_and(|txd| asset_key(txd, ".txd") == txd_key)
        })
        .map(|definition| {
            let dff = definition
                .attrs
                .get("dff")
                .or_else(|| definition.attrs.get("model"))
                .map(String::as_str)
                .filter(|name| !name.trim().is_empty())
                .unwrap_or(&definition.id);
            asset_key(dff, ".dff")
        })
        .collect::<BTreeSet<_>>();
    for placement in &app.placements {
        if definition_txd_name(&app.definitions, &placement.id)
            .is_some_and(|txd| asset_key(txd, ".txd") == txd_key)
        {
            names.insert(asset_key(&placement.dff, ".dff"));
        }
    }
    let same_name_dff = asset_key(txd_name, ".dff");
    if app
        .editing
        .rows
        .iter()
        .any(|row| asset_key(&row.entry.name, ".dff") == same_name_dff)
        || find_dff_entry_for_app(app, &same_name_dff).is_some()
    {
        names.insert(same_name_dff);
    }
    if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_ref()
        && dff
            .txd_context
            .as_ref()
            .is_some_and(|txd| asset_key(txd, ".txd") == txd_key)
    {
        names.insert(asset_key(&dff.name, ".dff"));
    }
    names
}

pub(crate) fn editing_rename_selected_dff_texture(
    app: &mut AppState,
    material: usize,
    new_texture_name: &str,
) -> bool {
    let new_texture_name = sanitize_texture_name(new_texture_name);
    let Some((source_texture, txd_name, current_dff_name)) =
        app.editing.asset.as_ref().and_then(|asset| match asset {
            EditingAsset::Dff(dff) => {
                dff.raw
                    .material_textures
                    .get(material)
                    .cloned()
                    .and_then(|source| {
                        dff.txd_context
                            .clone()
                            .map(|txd| (source, txd, Some(asset_key(&dff.name, ".dff"))))
                    })
            }
            EditingAsset::Txd(txd) => txd
                .textures
                .get(material)
                .map(|texture| (texture.name.clone(), txd.name.clone(), None)),
            EditingAsset::Col(_) => None,
        })
    else {
        app.status_message = "The selected texture no longer exists".to_string();
        return false;
    };
    if new_texture_name.eq_ignore_ascii_case(&source_texture) {
        app.status_message = format!("Texture is already named '{new_texture_name}'");
        return false;
    }
    let txd_key = asset_key(&txd_name, ".txd");
    let txd_bytes = app
        .editing
        .modified_entries
        .get(&editing_key(&txd_key))
        .cloned()
        .or_else(|| {
            find_txd_entry_for_app(app, &txd_key).map(|entry| read_txd_entry_bytes(&entry))
        });
    let Some(txd_bytes) = txd_bytes else {
        app.status_message = format!("Could not locate {txd_key}");
        return false;
    };
    let updated_txd =
        match rename_texture_native_in_txd(txd_bytes, &source_texture, &new_texture_name) {
            Ok(bytes) => bytes,
            Err(err) => {
                app.status_message = format!("Could not rename texture: {err}");
                return false;
            }
        };

    let mut dff_names = dff_names_using_txd(app, &txd_key);
    if let Some(name) = current_dff_name.as_ref() {
        dff_names.insert(name.clone());
    }
    let rename = HashMap::from([(lower(&source_texture), new_texture_name.clone())]);
    let mut dff_updates = Vec::<(String, Vec<u8>, usize)>::new();
    for dff_name in dff_names {
        let result = if current_dff_name
            .as_ref()
            .is_some_and(|current| dff_name == *current)
        {
            let Some(EditingAsset::Dff(current)) = app.editing.asset.as_ref() else {
                continue;
            };
            let mut raw = current.raw.clone();
            let mut changed = raw
                .material_textures
                .iter_mut()
                .filter(|name| name.eq_ignore_ascii_case(&source_texture))
                .map(|name| {
                    *name = new_texture_name.clone();
                })
                .count();
            for group in raw
                .components
                .iter_mut()
                .filter_map(|component| component.breakable.as_mut())
                .flat_map(|breakable| breakable.groups.iter_mut())
            {
                if group.texture.eq_ignore_ascii_case(&source_texture) {
                    group.texture = new_texture_name.clone();
                    changed += 1;
                }
                if group.mask.eq_ignore_ascii_case(&source_texture) {
                    group.mask = new_texture_name.clone();
                    changed += 1;
                }
            }
            if changed == 0 {
                continue;
            }
            let frame = Path::new(&current.name)
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("model");
            write_normalized_dff(&raw, frame).map(|bytes| (bytes, changed))
        } else {
            let source = app
                .editing
                .modified_entries
                .get(&editing_key(&dff_name))
                .cloned()
                .or_else(|| {
                    app.pending_replacement_assets
                        .get(&editing_key(&dff_name))
                        .map(|(_, bytes)| bytes.clone())
                })
                .or_else(|| {
                    find_dff_entry_for_app(app, &dff_name).map(|entry| {
                        let mut bytes = read_img_entry(&entry);
                        bytes.truncate(dff_chunk_len(&bytes));
                        bytes
                    })
                })
                .ok_or_else(|| format!("Could not locate referenced DFF {dff_name}"));
            source.and_then(|bytes| rewrite_dff_material_textures(&bytes, &rename))
        };
        match result {
            Ok((bytes, changed)) if changed > 0 => {
                dff_updates.push((dff_name, bytes, changed));
            }
            Ok(_) => {}
            Err(err) => {
                app.status_message =
                    format!("Texture rename was not staged because {dff_name} failed: {err}");
                return false;
            }
        }
    }

    let mut replacements = Vec::with_capacity(dff_updates.len() + 1);
    replacements.push((txd_key.clone(), updated_txd.clone()));
    replacements.extend(
        dff_updates
            .iter()
            .map(|(name, bytes, _)| (name.clone(), bytes.clone())),
    );
    let wip_root = wip_root_path(&app.root);
    if let Err(err) = upsert_replacement_assets(&wip_root, &replacements) {
        app.status_message = format!("Could not stage texture rename: {err}");
        return false;
    }

    editing_stage_modified_entry(app, &txd_key, updated_txd.clone());
    for (name, bytes, _) in &dff_updates {
        editing_stage_modified_entry(app, name, bytes.clone());
    }
    if let Some(EditingAsset::Dff(current)) = app.editing.asset.as_mut() {
        for name in &mut current.raw.material_textures {
            if name.eq_ignore_ascii_case(&source_texture) {
                *name = new_texture_name.clone();
            }
        }
        for group in current
            .raw
            .components
            .iter_mut()
            .filter_map(|component| component.breakable.as_mut())
            .flat_map(|breakable| breakable.groups.iter_mut())
        {
            if group.texture.eq_ignore_ascii_case(&source_texture) {
                group.texture = new_texture_name.clone();
            }
            if group.mask.eq_ignore_ascii_case(&source_texture) {
                group.mask = new_texture_name.clone();
            }
        }
        current.dirty = false;
    }
    if let Some(EditingAsset::Txd(txd)) = app.editing.asset.as_mut()
        && asset_key(&txd.name, ".txd") == txd_key
    {
        let textures = txd_entries_from_bytes(&txd.name, &updated_txd);
        txd.selected = textures
            .iter()
            .position(|entry| entry.name.eq_ignore_ascii_case(&new_texture_name))
            .unwrap_or(0);
        txd.textures = textures;
        txd.scroll = 0.0;
        if !editing_txd_filtered_indices(txd).contains(&txd.selected) {
            txd.search.clear();
            txd.search_cursor = 0;
            txd.search_anchor = None;
        }
    }
    app.pending_txd_writes.insert(txd_key.clone());
    app.loaded_wip = true;
    reindex_staged_txd(app, &txd_key);
    invalidate_cached_txd_textures(app, &txd_key, None);
    let recompiled = recompile_definitions_using_txd(app, &txd_key);
    invalidate_validation_cache(app);
    editing_update_txd_preview(app);
    refresh_editing_dff_preview(app);

    let material_refs = dff_updates
        .iter()
        .map(|(_, _, changed)| *changed)
        .sum::<usize>();
    app.status_message = format!(
        "Renamed '{source_texture}' to '{new_texture_name}' in {txd_key}; updated {} DFF(s), {material_refs} material reference(s), and recompiled {recompiled} definition(s)",
        dff_updates.len()
    );
    true
}

pub(crate) fn editing_rename_selected_txd_texture(
    app: &mut AppState,
    texture: usize,
    new_texture_name: &str,
) -> bool {
    editing_rename_selected_dff_texture(app, texture, new_texture_name)
}

/// Duplicate the selected material (texture, surface properties, and
/// animation) together with all of its faces, assigning the copies to the new
/// material slot.
pub(crate) fn editing_duplicate_selected_dff_material(app: &mut AppState) -> bool {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return false;
    };
    let material = dff.selected_material;
    let dff_name = dff.name.clone();
    let material_count = dff_material_slot_count(&dff.raw);
    if material >= material_count {
        app.status_message = "Select a DFF material first".to_string();
        return false;
    }
    let new_material = match append_dff_material_slot(&mut dff.raw, material, None, true) {
        Ok(material) => material,
        Err(error) => {
            app.status_message = error;
            return false;
        }
    };
    let tris = dff
        .raw
        .triangles
        .iter()
        .copied()
        .filter(|tri| tri.material as usize == material)
        .collect::<Vec<_>>();
    let mut remap = BTreeMap::<usize, usize>::new();
    let mut duplicated = 0usize;
    for tri in tris {
        let src = [tri.a as usize, tri.b as usize, tri.c as usize];
        if src.iter().any(|&idx| idx >= dff.raw.vertices.len()) {
            continue;
        }
        let mut dst = [0u32; 3];
        for (slot, source) in dst.iter_mut().zip(src) {
            let raw = &mut dff.raw;
            let new_idx = *remap
                .entry(source)
                .or_insert_with(|| raw_append_vertex_copy(raw, source));
            *slot = new_idx as u32;
        }
        dff.raw.triangles.push(Tri {
            a: dst[0],
            b: dst[1],
            c: dst[2],
            material: new_material as u16,
        });
        duplicated += 1;
    }
    dff.selected_material = new_material;
    dff.selected_2dfx = None;
    dff.material_scroll = new_material as f32;
    dff.dirty = true;
    let source_key = material_emitter_key(&dff_name, material);
    if let Some(emitter) = app.material_emitters.get(&source_key).copied() {
        let target_key = material_emitter_key(&dff_name, new_material);
        app.material_emitters.insert(target_key, emitter);
    }
    if let Some(casts_shadow) = app.shadow_casting.get(&source_key).copied() {
        let target_key = material_emitter_key(&dff_name, new_material);
        app.shadow_casting.insert(target_key, casts_shadow);
    }
    app.status_message = format!(
        "Duplicated material #{material:02} to #{new_material:02} with {duplicated} face(s)"
    );
    refresh_editing_dff_preview(app);
    true
}

pub(crate) fn set_selected_editing_dff_boolean_box_position(
    app: &mut AppState,
    position: V3,
) -> Result<(), String> {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return Err("Open a DFF in Editing first".to_string());
    };
    let Some(cutter) = dff.boolean_box.as_mut() else {
        return Err("Add a boolean cutter first".to_string());
    };
    cutter.center = position;
    dff.dirty = true;
    app.status_message = "Moved DFF boolean cutter".to_string();
    Ok(())
}

pub(crate) fn set_selected_editing_dff_2dfx_position(
    app: &mut AppState,
    position: V3,
) -> Result<(), String> {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return Err("Open a DFF in Editing first".to_string());
    };
    let Some(idx) = dff.selected_2dfx else {
        return Err("Select a DFF 2DFX entry first".to_string());
    };
    let Some(effect) = dff.raw.effects_2dfx.get_mut(idx) else {
        return Err("Selected DFF 2DFX entry no longer exists".to_string());
    };
    effect.position = position;
    dff.dirty = true;
    app.status_message = format!("Moved DFF 2DFX {}", idx + 1);
    Ok(())
}

fn dff_2dfx_default_light_payload() -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(&[255, 240, 190, 255]);
    for value in [60.0f32, 12.0, 1.5, 0.0] {
        payload.extend_from_slice(&value.to_le_bytes());
    }
    payload.extend_from_slice(&[0, 1, 0, 0, 0x40]);
    let mut corona = [0u8; 24];
    let corona_name = b"coronastar";
    corona[..corona_name.len()].copy_from_slice(corona_name);
    payload.extend_from_slice(&corona);
    payload.extend_from_slice(&[0u8; 24]);
    payload.extend_from_slice(&[0, 0]);
    payload.push(0);
    payload
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct DffCoronaPreset {
    label: &'static str,
    detail: &'static str,
    color: [u8; 4],
    far_clip: f32,
    pointlight_range: f32,
    corona_size: f32,
    shadow_size: f32,
    show_mode: u8,
    reflection: u8,
    flare_type: u8,
    shadow_multiplier: u8,
    flags1: u8,
    shadow_z_distance: u8,
    flags2: u8,
    look_direction: [u8; 3],
}

const DFF_CORONA_PRESETS: [DffCoronaPreset; 10] = [
    DffCoronaPreset {
        label: "Traffic Light — Red",
        detail: "Stock traffic state; pair with native traffic-light behavior",
        color: [255, 0, 0, 200],
        far_clip: 100.0,
        pointlight_range: 18.0,
        corona_size: 1.0,
        shadow_size: 8.0,
        show_mode: 7,
        reflection: 1,
        flare_type: 0,
        shadow_multiplier: 40,
        flags1: 0x42,
        shadow_z_distance: 0,
        flags2: 0x04,
        look_direction: [0, 0, 100],
    },
    DffCoronaPreset {
        label: "Traffic Light — Amber",
        detail: "Stock traffic state; pair with native traffic-light behavior",
        color: [255, 210, 52, 200],
        far_clip: 100.0,
        pointlight_range: 18.0,
        corona_size: 1.0,
        shadow_size: 8.0,
        show_mode: 7,
        reflection: 1,
        flare_type: 0,
        shadow_multiplier: 40,
        flags1: 0x42,
        shadow_z_distance: 0,
        flags2: 0x04,
        look_direction: [0, 0, 100],
    },
    DffCoronaPreset {
        label: "Traffic Light — Green",
        detail: "Stock traffic state; pair with native traffic-light behavior",
        color: [0, 255, 0, 200],
        far_clip: 100.0,
        pointlight_range: 18.0,
        corona_size: 1.0,
        shadow_size: 8.0,
        show_mode: 7,
        reflection: 1,
        flare_type: 0,
        shadow_multiplier: 40,
        flags1: 0x42,
        shadow_z_distance: 0,
        flags2: 0x04,
        look_direction: [0, 0, 100],
    },
    DffCoronaPreset {
        label: "Street Lamp — Warm",
        detail: "Orange night lamp with point light and ground shadow",
        color: [249, 145, 34, 200],
        far_clip: 100.0,
        pointlight_range: 12.0,
        corona_size: 2.5,
        shadow_size: 8.0,
        show_mode: 0,
        reflection: 1,
        flare_type: 0,
        shadow_multiplier: 40,
        flags1: 0x42,
        shadow_z_distance: 0,
        flags2: 0x04,
        look_direction: [0, 0, 100],
    },
    DffCoronaPreset {
        label: "Street Lamp — White",
        detail: "White night lamp with point light and ground shadow",
        color: [255, 255, 255, 200],
        far_clip: 100.0,
        pointlight_range: 12.0,
        corona_size: 1.0,
        shadow_size: 8.0,
        show_mode: 0,
        reflection: 1,
        flare_type: 0,
        shadow_multiplier: 40,
        flags1: 0x42,
        shadow_z_distance: 0,
        flags2: 0x04,
        look_direction: [0, 0, 100],
    },
    DffCoronaPreset {
        label: "Bollard / Path Light",
        detail: "Compact white night light for low fixtures",
        color: [255, 255, 255, 200],
        far_clip: 180.0,
        pointlight_range: 12.0,
        corona_size: 0.75,
        shadow_size: 6.0,
        show_mode: 0,
        reflection: 1,
        flare_type: 0,
        shadow_multiplier: 40,
        flags1: 0x42,
        shadow_z_distance: 0,
        flags2: 0x04,
        look_direction: [0, 0, 100],
    },
    DffCoronaPreset {
        label: "Warning Flasher — Amber",
        detail: "Alternating roadblock warning-light behavior",
        color: [255, 65, 28, 200],
        far_clip: 100.0,
        pointlight_range: 12.0,
        corona_size: 0.6,
        shadow_size: 0.0,
        show_mode: 6,
        reflection: 0,
        flare_type: 0,
        shadow_multiplier: 40,
        flags1: 0x43,
        shadow_z_distance: 0,
        flags2: 0,
        look_direction: [0, 0, 100],
    },
    DffCoronaPreset {
        label: "Rail Crossing — Red",
        detail: "Train-crossing controlled red warning light",
        color: [255, 0, 0, 200],
        far_clip: 100.0,
        pointlight_range: 18.0,
        corona_size: 1.0,
        shadow_size: 8.0,
        show_mode: 8,
        reflection: 0,
        flare_type: 0,
        shadow_multiplier: 40,
        flags1: 0x60,
        shadow_z_distance: 0,
        flags2: 0,
        look_direction: [0, 0, 100],
    },
    DffCoronaPreset {
        label: "Airport Sequence — Red",
        detail: "Fast sequential flash for runway and obstruction lights",
        color: [148, 0, 0, 200],
        far_clip: 100.0,
        pointlight_range: 1.5,
        corona_size: 1.0,
        shadow_size: 8.0,
        show_mode: 3,
        reflection: 0,
        flare_type: 0,
        shadow_multiplier: 40,
        flags1: 0x42,
        shadow_z_distance: 0,
        flags2: 0,
        look_direction: [0, 0, 100],
    },
    DffCoronaPreset {
        label: "Decorative Lamp — Random",
        detail: "Soft decorative light with randomized flashing",
        color: [225, 185, 149, 200],
        far_clip: 62.0,
        pointlight_range: 5.0,
        corona_size: 1.0,
        shadow_size: 8.0,
        show_mode: 1,
        reflection: 0,
        flare_type: 0,
        shadow_multiplier: 40,
        flags1: 0x42,
        shadow_z_distance: 0,
        flags2: 0,
        look_direction: [0, 0, 100],
    },
];

fn dff_2dfx_corona_preset_payload(preset: DffCoronaPreset) -> Vec<u8> {
    let mut payload = vec![0u8; 80];
    payload[0..4].copy_from_slice(&preset.color);
    for (offset, value) in [
        (4, preset.far_clip),
        (8, preset.pointlight_range),
        (12, preset.corona_size),
        (16, preset.shadow_size),
    ] {
        payload[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    payload[20] = preset.show_mode;
    payload[21] = preset.reflection;
    payload[22] = preset.flare_type;
    payload[23] = preset.shadow_multiplier;
    payload[24] = preset.flags1;
    write_fixed_string(&mut payload, 25, 24, "coronastar");
    write_fixed_string(&mut payload, 49, 24, "shad_exp");
    payload[73] = preset.shadow_z_distance;
    payload[74] = preset.flags2;
    payload[75..78].copy_from_slice(&preset.look_direction);
    payload
}

fn dff_2dfx_corona_preset(effect: &Dff2dEffect) -> Option<DffCoronaPreset> {
    (effect.effect_id == 0).then_some(())?;
    DFF_CORONA_PRESETS
        .iter()
        .copied()
        .find(|preset| effect.payload == dff_2dfx_corona_preset_payload(*preset))
}

fn dff_2dfx_traffic_color_label(effect: &Dff2dEffect) -> Option<&'static str> {
    if effect.effect_id != 0 || effect.payload.get(20).copied() != Some(7) {
        return None;
    }
    let red = effect.payload.first().copied().unwrap_or(0);
    let green = effect.payload.get(1).copied().unwrap_or(0);
    Some(if red > 200 {
        if green > 100 { "Amber" } else { "Red" }
    } else {
        "Green"
    })
}

fn dff_2dfx_light_color_label(effect: &Dff2dEffect) -> String {
    let [red, green, blue] = [
        effect.payload.first().copied().unwrap_or(0),
        effect.payload.get(1).copied().unwrap_or(0),
        effect.payload.get(2).copied().unwrap_or(0),
    ];
    if red >= 235 && green >= 225 && blue >= 215 {
        "White".to_string()
    } else if red >= 210 && green >= 90 && green > blue.saturating_add(30) {
        "Warm".to_string()
    } else if red >= 180 && red > green.saturating_add(70) {
        "Red".to_string()
    } else if green >= 180 && green > red.saturating_add(40) {
        "Green".to_string()
    } else if blue >= 180 && blue > red.saturating_add(40) {
        "Blue".to_string()
    } else {
        format!("RGB {red}/{green}/{blue}")
    }
}

fn dff_2dfx_traffic_groups(effects: &[Dff2dEffect]) -> Vec<Vec<usize>> {
    let mut groups = Vec::<Vec<usize>>::new();
    let mut current = Vec::<usize>::new();
    let mut seen = [false; 3];

    for (index, effect) in effects.iter().enumerate() {
        let Some(color) = dff_2dfx_traffic_color_label(effect) else {
            continue;
        };
        let color_index = match color {
            "Red" => 0,
            "Amber" => 1,
            _ => 2,
        };
        if seen[color_index] && !current.is_empty() {
            groups.push(std::mem::take(&mut current));
            seen = [false; 3];
        }
        current.push(index);
        seen[color_index] = true;
        if seen.into_iter().all(|present| present) {
            groups.push(std::mem::take(&mut current));
            seen = [false; 3];
        }
    }
    if !current.is_empty() {
        groups.push(current);
    }
    groups
}

fn dff_2dfx_traffic_head_label(effects: &[Dff2dEffect], effect_index: usize) -> Option<String> {
    dff_2dfx_traffic_color_label(effects.get(effect_index)?)?;
    let groups = dff_2dfx_traffic_groups(effects);
    let target_group = groups
        .iter()
        .position(|group| group.contains(&effect_index))?;
    if groups.len() == 1 {
        return None;
    }

    let mut ordered = groups
        .iter()
        .enumerate()
        .map(|(group_index, group)| {
            let average_z = group
                .iter()
                .filter_map(|index| effects.get(*index))
                .map(|effect| effect.position.z)
                .sum::<f32>()
                / group.len().max(1) as f32;
            (group_index, average_z)
        })
        .collect::<Vec<_>>();
    ordered.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| left.0.cmp(&right.0))
    });
    let vertical_range = ordered
        .first()
        .zip(ordered.last())
        .map(|(top, bottom)| top.1 - bottom.1)
        .unwrap_or(0.0);
    let vertical_rank = ordered
        .iter()
        .position(|(group_index, _)| *group_index == target_group)
        .unwrap_or(target_group);

    if vertical_range >= 0.25 {
        return Some(match (groups.len(), vertical_rank) {
            (2, 0) => "Upper".to_string(),
            (2, _) => "Lower".to_string(),
            (3, 0) => "Upper".to_string(),
            (3, 1) => "Middle".to_string(),
            (3, _) => "Lower".to_string(),
            _ => format!("Head {}", vertical_rank + 1),
        });
    }
    Some(format!("Head {}", target_group + 1))
}

fn dff_2dfx_effect_display_name(effects: &[Dff2dEffect], effect_index: usize) -> String {
    let Some(effect) = effects.get(effect_index) else {
        return "Unknown".to_string();
    };
    if effect.effect_id == 1 {
        let name = fixed_string(&effect.payload, 0, 24);
        return if name.is_empty() {
            "Particle".to_string()
        } else {
            format!("Particle — {name}")
        };
    }
    if let Some(color) = dff_2dfx_traffic_color_label(effect) {
        return match dff_2dfx_traffic_head_label(effects, effect_index) {
            Some(head) => format!("{head} Traffic Light — {color}"),
            None => format!("Traffic Light — {color}"),
        };
    }
    if let Some(preset) = dff_2dfx_corona_preset(effect) {
        return preset.label.to_string();
    }
    if effect.effect_id == 0 {
        if dff_2dfx_is_generated_corona(effect) {
            return format!("Generated Corona — {}", dff_2dfx_light_color_label(effect));
        }
        let kind = match effect.payload.get(20).copied().unwrap_or(0) {
            0 => "Street / Path Light",
            1 => "Decorative Lamp",
            3 => "Airport Sequence",
            6 => "Warning Flasher",
            8 => "Rail Crossing",
            _ => "Corona",
        };
        return format!("{kind} — {}", dff_2dfx_light_color_label(effect));
    }
    dff_2dfx_label(effect.effect_id).to_string()
}

pub(crate) fn add_dff_2dfx_corona_preset(app: &mut AppState, preset_index: usize) -> bool {
    let Some(preset) = DFF_CORONA_PRESETS.get(preset_index).copied() else {
        return false;
    };
    let (dff_name, used_selected_face) = {
        let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
            return false;
        };
        let position = dff_2dfx_default_position(dff);
        let used_selected_face = dff.selected_face.is_some();
        dff.raw.effects_2dfx.push(Dff2dEffect {
            position,
            effect_id: 0,
            payload: dff_2dfx_corona_preset_payload(preset),
        });
        dff.selected_2dfx = dff.raw.effects_2dfx.len().checked_sub(1);
        dff.dff_2dfx_corona_preset_picker_open = false;
        dff.dirty = true;
        (dff.name.clone(), used_selected_face)
    };
    let placement = if used_selected_face {
        "selected face"
    } else {
        "model center"
    };
    app.status_message = if preset.show_mode == 7 {
        let binding = ensure_traffic_native_model_for_dff(app, &dff_name);
        format!("Added {} at {placement}; {binding}", preset.label)
    } else {
        format!("Added {} at {placement}", preset.label)
    };
    true
}

const DFF_GENERATED_CORONA_MARKER: &[u8] = b"eagle.generated.corona";
const DFF_GENERATED_CORONA_LOOK_MARKER: &[u8; 3] = b"EGL";
const DFF_GENERATED_CORONA_TEXTURE_TAIL_MARKER: &[u8; 6] = b"EGLGEN";
const DFF_GENERATED_CORONA_FLAGS2_MARKER: u8 = 0xA0;
const DFF_GENERATED_CORONA_FLAGS2_MARKER_MASK: u8 = 0xE0;
const DFF_GENERATED_GAME_COLOR_LIFT: f32 = 0.72;
const DFF_GENERATED_CORONA_ALPHA: u8 = 255;
const DFF_GENERATED_CORONA_SIZE: f32 = 0.60;
const DFF_GENERATED_SHADOW_SIZE: f32 = 14.0;
const DFF_GENERATED_POINTLIGHT_HEIGHT_MULTIPLIER: f32 = 2.75;
const DFF_GENERATED_POINTLIGHT_RANGE_ADDITIVE: f32 = 8.0;
const DFF_GENERATED_POINTLIGHT_RANGE_MAX: f32 = 192.0;
pub(crate) const DFF_GENERATED_EDITOR_CORONA_SCALE: f32 = 0.12;
pub(crate) const DFF_GENERATED_EDITOR_POINTLIGHT_SCALE: f32 = 0.12;
const DFF_2DFX_LIGHT_FLAGS_1_OFFSET: usize = 24;
const DFF_2DFX_LIGHT_SHADOW_TEXTURE_OFFSET: usize = 49;
const DFF_2DFX_LIGHT_TEXTURE_NAME_LEN: usize = 24;
const DFF_2DFX_LIGHT_SHADOW_TEXTURE_TAIL_MARKER_OFFSET: usize =
    DFF_2DFX_LIGHT_SHADOW_TEXTURE_OFFSET + 9;
const DFF_2DFX_LIGHT_FLAGS_2_OFFSET: usize = 74;
const DFF_2DFX_LIGHT_LOOK_DIRECTION_OFFSET: usize = 75;

pub(crate) fn dff_2dfx_is_generated_corona(effect: &Dff2dEffect) -> bool {
    effect.effect_id == 0
        && (effect
            .payload
            .get(
                DFF_2DFX_LIGHT_SHADOW_TEXTURE_TAIL_MARKER_OFFSET
                    ..DFF_2DFX_LIGHT_SHADOW_TEXTURE_TAIL_MARKER_OFFSET
                        + DFF_GENERATED_CORONA_TEXTURE_TAIL_MARKER.len(),
            )
            .is_some_and(|value| value == DFF_GENERATED_CORONA_TEXTURE_TAIL_MARKER)
            // Backward compatibility: versions that tagged Flags2 used bits
            // that SA interprets as blinking flags. Recognize those entries so
            // Regenerate removes the bad flags and rewrites a stock-safe light.
            || (effect
                .payload
                .get(DFF_2DFX_LIGHT_FLAGS_2_OFFSET)
                .copied()
                .unwrap_or(0)
                & DFF_GENERATED_CORONA_FLAGS2_MARKER_MASK)
                == DFF_GENERATED_CORONA_FLAGS2_MARKER
            || effect
            .payload
            .get(
                DFF_2DFX_LIGHT_LOOK_DIRECTION_OFFSET
                    ..DFF_2DFX_LIGHT_LOOK_DIRECTION_OFFSET
                        + DFF_GENERATED_CORONA_LOOK_MARKER.len(),
            )
            .is_some_and(|value| value == DFF_GENERATED_CORONA_LOOK_MARKER)
            // Backward compatibility: replace coronas generated by versions
            // that stored the marker in the shadow texture slot.
            || effect
                .payload
                .get(
                    DFF_2DFX_LIGHT_SHADOW_TEXTURE_OFFSET
                        ..DFF_2DFX_LIGHT_SHADOW_TEXTURE_OFFSET
                            + DFF_GENERATED_CORONA_MARKER.len(),
                )
                .is_some_and(|value| value == DFF_GENERATED_CORONA_MARKER))
}

fn dff_2dfx_generated_corona(source: PointEmitterCoronaSource) -> Dff2dEffect {
    let mut payload = dff_2dfx_default_light_payload();
    for (channel, value) in [source.color.x, source.color.y, source.color.z]
        .into_iter()
        .enumerate()
    {
        // The game stores 2DFX color in only eight bits per channel, so a
        // conventional exposure multiplier has almost no effect on warm
        // streetlights whose red channel is already saturated. Lift every
        // channel toward white instead, preserving the authored hue while
        // giving both the corona and the point-light projection more energy.
        let value = value.clamp(0.0, 1.0);
        let lifted = value + (1.0 - value) * DFF_GENERATED_GAME_COLOR_LIFT;
        payload[channel] = (lifted * 255.0).round() as u8;
    }
    payload[3] = DFF_GENERATED_CORONA_ALPHA;
    payload[4..8].copy_from_slice(&100.0f32.to_le_bytes());
    payload[12..16].copy_from_slice(&DFF_GENERATED_CORONA_SIZE.to_le_bytes());
    // Match stock SA streetlights: the exponential shadow texture produces
    // the visible pool on roads while the pointlight range illuminates nearby
    // dynamic entities such as vehicles.
    payload[16..20].copy_from_slice(&DFF_GENERATED_SHADOW_SIZE.to_le_bytes());
    payload[23] = 80;
    // Editor emitter falloff is authored for the vertex-bake radius. SA keeps
    // full point-light strength only through the inner half of the runtime
    // radius, so infer a ground-reaching radius from the emitter's height over
    // the DFF base and add a small footprint margin. Vertex-bake falloff is a
    // separate authored distance and does not determine the compact 2DFX pool.
    let pointlight_range = if source.range <= 0.0 {
        0.0
    } else {
        let ground_reaching = source.height_above_model_base
            * DFF_GENERATED_POINTLIGHT_HEIGHT_MULTIPLIER
            + DFF_GENERATED_POINTLIGHT_RANGE_ADDITIVE;
        ground_reaching.clamp(4.0, DFF_GENERATED_POINTLIGHT_RANGE_MAX)
    };
    payload[8..12].copy_from_slice(&pointlight_range.to_le_bytes());
    payload[DFF_2DFX_LIGHT_FLAGS_1_OFFSET] =
        0x02 | if source.day { 0x20 } else { 0 } | if source.night { 0x40 } else { 0 };
    let marker_field = &mut payload[DFF_2DFX_LIGHT_SHADOW_TEXTURE_OFFSET
        ..DFF_2DFX_LIGHT_SHADOW_TEXTURE_OFFSET + DFF_2DFX_LIGHT_TEXTURE_NAME_LEN];
    marker_field.fill(0);
    marker_field[..8].copy_from_slice(b"shad_exp");
    // Keep the generated marker after shad_exp's null terminator. RenderWare
    // reads this field as a C string, so these tail bytes are ignored by SA.
    // Flags2 must stay exactly 0x04: 0x20 is the Blinking3 bit and causes the
    // point light to be disabled for 63 of every 64 frames.
    marker_field[9..9 + DFF_GENERATED_CORONA_TEXTURE_TAIL_MARKER.len()]
        .copy_from_slice(DFF_GENERATED_CORONA_TEXTURE_TAIL_MARKER);
    payload[DFF_2DFX_LIGHT_FLAGS_2_OFFSET] = 0x04;
    payload.resize(80, 0);
    payload[DFF_2DFX_LIGHT_LOOK_DIRECTION_OFFSET] = 0;
    payload[DFF_2DFX_LIGHT_LOOK_DIRECTION_OFFSET + 1] = 0;
    payload[DFF_2DFX_LIGHT_LOOK_DIRECTION_OFFSET + 2] = 100;
    Dff2dEffect {
        position: source.position,
        effect_id: 0,
        payload,
    }
}

fn generated_coronas_for_raw(
    emitters: &HashMap<String, MaterialEmitter>,
    dff_name: &str,
    raw: &RawMesh,
) -> Vec<Dff2dEffect> {
    dff_point_emitter_corona_sources(emitters, dff_name, raw)
        .into_iter()
        .map(dff_2dfx_generated_corona)
        .collect()
}

fn replace_generated_coronas(raw: &mut RawMesh, generated: Vec<Dff2dEffect>) -> (usize, usize) {
    let previous = raw
        .effects_2dfx
        .iter()
        .filter(|effect| dff_2dfx_is_generated_corona(effect))
        .count();
    raw.effects_2dfx
        .retain(|effect| !dff_2dfx_is_generated_corona(effect));
    let created = generated.len();
    raw.effects_2dfx.extend(generated);
    (previous, created)
}

pub(crate) fn regenerate_dff_2dfx_coronas(app: &mut AppState) -> bool {
    let generated = match app.editing.asset.as_ref() {
        Some(EditingAsset::Dff(dff)) => {
            generated_coronas_for_raw(&app.material_emitters, &dff.name, &dff.raw)
        }
        _ => return false,
    };
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return false;
    };
    let selected_manual = dff
        .selected_2dfx
        .and_then(|idx| dff.raw.effects_2dfx.get(idx))
        .filter(|effect| !dff_2dfx_is_generated_corona(effect))
        .cloned();
    let (previous, created) = replace_generated_coronas(&mut dff.raw, generated);
    dff.selected_2dfx = selected_manual.as_ref().and_then(|selected| {
        dff.raw
            .effects_2dfx
            .iter()
            .position(|effect| effect == selected)
    });
    let changed = previous > 0 || created > 0;
    if changed {
        dff.dirty = true;
        app.status_message = format!(
            "Regenerated {created} 2DFX corona(s); replaced {previous} generated corona(s)"
        );
    } else {
        app.status_message = "No enabled point-light emitter groups found for this DFF".to_string();
    }
    changed
}

struct CoronaGenerationContext {
    root: PathBuf,
    emitters: HashMap<String, MaterialEmitter>,
    byte_overrides: BTreeMap<String, (String, Vec<u8>)>,
    raw_overrides: BTreeMap<String, (String, RawMesh)>,
    vertex_mesh_overrides: BTreeMap<String, Vec<(String, RenderMesh)>>,
    deleted_assets: HashSet<String>,
    building_dffs: HashSet<String>,
}

#[derive(Clone)]
struct CoronaGenerationReplacement {
    name: String,
    bytes: Vec<u8>,
    raw: RawMesh,
    preserved_vertex_mesh_keys: Vec<String>,
}

struct CoronaGenerationResult {
    replacements: Vec<CoronaGenerationReplacement>,
    texture_files: HashMap<String, PathBuf>,
    removed: usize,
    created: usize,
    scanned: usize,
}

fn collect_global_corona_dff_sources(
    context: &CoronaGenerationContext,
) -> Result<BTreeMap<String, (String, Vec<u8>)>, String> {
    let mut sources = BTreeMap::<String, (String, Vec<u8>)>::new();
    for img_path in collect_resource_img_files(&context.root) {
        if img_path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case(REPLACEMENT_IMG))
        {
            continue;
        }
        for entry in parse_img(&img_path) {
            if !lower(&entry.name).ends_with(".dff") {
                continue;
            }
            let key = asset_key(&entry.name, ".dff");
            if context.deleted_assets.contains(&key) {
                continue;
            }
            let mut bytes = read_img_entry(&entry);
            bytes.truncate(replacement_entry_len(&entry.name, &bytes));
            sources.entry(key).or_insert((entry.name, bytes));
        }
    }
    for dir in ["models", "Models", "imgs", "Imgs"] {
        let path = context.root.join(dir);
        if !path.exists() {
            continue;
        }
        for entry in WalkDir::new(path).into_iter().filter_map(Result::ok) {
            if !entry.file_type().is_file()
                || !entry
                    .path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("dff"))
            {
                continue;
            }
            let Some(name) = entry.path().file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let key = asset_key(name, ".dff");
            if context.deleted_assets.contains(&key) {
                continue;
            }
            let bytes = fs::read(entry.path())
                .map_err(|err| format!("Could not read {}: {err}", entry.path().display()))?;
            sources.insert(key, (name.to_string(), bytes));
        }
    }
    for replacement_root in [&context.root, &wip_root_path(&context.root)] {
        let path = replacement_root.join("imgs").join(REPLACEMENT_IMG);
        for entry in parse_img(&path) {
            if !lower(&entry.name).ends_with(".dff") {
                continue;
            }
            let key = asset_key(&entry.name, ".dff");
            if context.deleted_assets.contains(&key) {
                continue;
            }
            let mut bytes = read_img_entry(&entry);
            bytes.truncate(replacement_entry_len(&entry.name, &bytes));
            sources.insert(key, (entry.name, bytes));
        }
    }
    for (key, (name, bytes)) in &context.byte_overrides {
        if lower(name).ends_with(".dff") && !context.deleted_assets.contains(key) {
            sources.insert(key.clone(), (name.clone(), bytes.clone()));
        }
    }
    Ok(sources)
}

fn generate_all_dff_2dfx_coronas(
    context: CoronaGenerationContext,
    progress: &mpsc::Sender<String>,
) -> Result<CoronaGenerationResult, String> {
    // Texture discovery is shared by every live DFF refresh. Do it once on the
    // worker instead of rescanning the resource tree for each replacement on
    // the render thread.
    let _ = progress.send("2DFX corona generation: indexing textures".to_string());
    let texture_files = collect_texture_files(&context.root);
    let mut sources = collect_global_corona_dff_sources(&context)?;
    for key in context.raw_overrides.keys() {
        sources.remove(key);
    }
    let total = sources.len() + context.raw_overrides.len();
    let mut replacements = Vec::<CoronaGenerationReplacement>::new();
    let mut removed = 0usize;
    let mut created = 0usize;
    let mut scanned = 0usize;

    let mut process_raw = |name: String, mut raw: RawMesh| -> Result<(), String> {
        scanned += 1;
        if scanned == 1 || scanned % 16 == 0 || scanned == total {
            let _ = progress.send(format!(
                "2DFX corona generation: processing DFF {scanned}/{total}"
            ));
        }
        let key = asset_key(&name, ".dff");
        let mut preserved_vertex_mesh_keys = Vec::new();
        if let Some(runtime_meshes) = context.vertex_mesh_overrides.get(&key) {
            let mesh_refs = runtime_meshes
                .iter()
                .map(|(_, mesh)| mesh)
                .collect::<Vec<_>>();
            apply_runtime_vertex_meshes_to_raw(&mesh_refs, &mut raw).map_err(|err| {
                format!("Could not preserve unsaved vertex lighting for {name}: {err}")
            })?;
            preserved_vertex_mesh_keys
                .extend(runtime_meshes.iter().map(|(mesh_key, _)| mesh_key.clone()));
        }
        let generated = generated_coronas_for_raw(&context.emitters, &name, &raw);
        let (old_count, new_count) = replace_generated_coronas(&mut raw, generated);
        if old_count == 0 && new_count == 0 {
            return Ok(());
        }
        let frame = Path::new(&name)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("model");
        let options = DffWriteOptions {
            include_normals: !context.building_dffs.contains(&asset_key(&name, ".dff")),
            include_bin_mesh: true,
        };
        let bytes = write_normalized_dff_with_options(&raw, frame, options)
            .map_err(|err| format!("Could not generate 2DFX coronas for {name}: {err}"))?;
        // Keep the exact normalized mesh represented by the staged bytes so
        // the batched live preview matches what Save will write.
        let raw = parse_dff_mesh(&bytes);
        removed += old_count;
        created += new_count;
        replacements.push(CoronaGenerationReplacement {
            name,
            bytes,
            raw,
            preserved_vertex_mesh_keys,
        });
        Ok(())
    };
    for (_, (name, bytes)) in sources {
        process_raw(name, parse_dff_mesh(&bytes))?;
    }
    for (_, (name, raw)) in context.raw_overrides {
        process_raw(name, raw)?;
    }
    Ok(CoronaGenerationResult {
        replacements,
        texture_files,
        removed,
        created,
        scanned,
    })
}

pub(crate) struct CoronaGenerationJob {
    rx: mpsc::Receiver<Result<CoronaGenerationResult, String>>,
    progress_rx: mpsc::Receiver<String>,
    result: Option<CoronaGenerationResult>,
    refresh_index: usize,
    started_at: Instant,
}

const CORONA_REFRESH_BATCH_MIN: usize = 2;
const CORONA_REFRESH_BATCH_LIMIT: usize = 8;
const CORONA_REFRESH_FRAME_BUDGET: Duration = Duration::from_millis(6);

impl CoronaGenerationJob {
    fn new(context: CoronaGenerationContext) -> Self {
        let (tx, rx) = mpsc::channel();
        let (progress_tx, progress_rx) = mpsc::channel();
        thread::spawn(move || {
            let result =
                std::panic::catch_unwind(|| generate_all_dff_2dfx_coronas(context, &progress_tx))
                    .map_err(|panic| {
                        panic
                            .downcast_ref::<&str>()
                            .map(|message| (*message).to_string())
                            .or_else(|| panic.downcast_ref::<String>().cloned())
                            .unwrap_or_else(|| "unknown 2DFX corona generation panic".to_string())
                    })
                    .and_then(|result| result);
            let _ = tx.send(result);
        });
        Self {
            rx,
            progress_rx,
            result: None,
            refresh_index: 0,
            started_at: Instant::now(),
        }
    }

    fn step(&mut self, app: &mut AppState) -> bool {
        if let Some(message) = self.progress_rx.try_iter().last() {
            app.status_message = message;
        }
        if self.result.is_none() {
            match self.rx.try_recv() {
                Ok(Ok(result)) => {
                    self.result = Some(result);
                }
                Ok(Err(err)) => {
                    app.status_message = format!("2DFX corona generation failed: {err}");
                    return true;
                }
                Err(mpsc::TryRecvError::Empty) => return false,
                Err(mpsc::TryRecvError::Disconnected) => {
                    app.status_message = "2DFX corona generation worker disconnected".to_string();
                    return true;
                }
            }
        }
        let result = self
            .result
            .as_ref()
            .expect("corona generation result is set");
        if self.refresh_index < result.replacements.len() {
            let refresh_started = Instant::now();
            let mut refreshed_this_frame = 0usize;
            let mut rebuilt_mesh = false;
            while let Some(replacement) = result.replacements.get(self.refresh_index) {
                let key = editing_key(&replacement.name);
                let open_dirty = matches!(
                    app.editing.asset.as_ref(),
                    Some(EditingAsset::Dff(dff))
                        if editing_key(&dff.name) == key && dff.dirty
                );
                if open_dirty || app.editing.modified_entries.contains_key(&key) {
                    app.editing
                        .modified_entries
                        .insert(key.clone(), replacement.bytes.clone());
                } else {
                    app.pending_replacement_assets.insert(
                        key.clone(),
                        (replacement.name.clone(), replacement.bytes.clone()),
                    );
                }
                if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut()
                    && editing_key(&dff.name) == key
                {
                    dff.raw.effects_2dfx = replacement.raw.effects_2dfx.clone();
                    dff.raw.prelit_colors = replacement.raw.prelit_colors.clone();
                    dff.raw.night_prelit_colors = replacement.raw.night_prelit_colors.clone();
                    dff.selected_2dfx = None;
                    dff.dirty = false;
                }
                for mesh_key in &replacement.preserved_vertex_mesh_keys {
                    app.pending_vertex_light_meshes.remove(mesh_key);
                    app.vertex_paint_dirty_meshes.remove(mesh_key);
                }
                rebuilt_mesh |= refresh_live_dff_from_raw(
                    app,
                    &replacement.name,
                    &replacement.raw,
                    &result.texture_files,
                );
                self.refresh_index += 1;
                refreshed_this_frame += 1;
                if refreshed_this_frame >= CORONA_REFRESH_BATCH_LIMIT
                    || (refreshed_this_frame >= CORONA_REFRESH_BATCH_MIN
                        && refresh_started.elapsed() >= CORONA_REFRESH_FRAME_BUDGET)
                {
                    break;
                }
            }
            if rebuilt_mesh {
                // Rebuilding the spatial/render batches once per DFF dominated
                // large 2DFX jobs. All meshes refreshed above become visible in
                // a single scene rebuild for this bounded frame batch.
                rebuild_render_cells(app);
            }
            app.status_message = format!(
                "2DFX corona generation: refreshing DFFs {}/{}",
                self.refresh_index,
                result.replacements.len()
            );
            return false;
        }
        let elapsed = self.started_at.elapsed().as_secs_f32();
        if result.replacements.is_empty() {
            app.status_message = format!(
                "2DFX corona generation scanned {} DFF(s) in {elapsed:.1}s; no enabled point-light emitter groups found",
                result.scanned
            );
        } else {
            clear_history_for_external_change(app);
            app.status_message = format!(
                "Regenerated {} 2DFX corona light(s) across {} DFF(s) in {elapsed:.1}s; replaced {} generated light(s). Save to apply. Undo history cleared.",
                result.created,
                result.replacements.len(),
                result.removed
            );
        }
        true
    }
}

pub(crate) fn regenerate_all_dff_2dfx_coronas(app: &mut AppState) -> bool {
    if app.corona_generation_job.is_some()
        || app.txd_cleanup_job.is_some()
        || app.asset_optimization_scan_rx.is_some()
        || app.asset_optimization_job.is_some()
        || app.object_bounds_fix_job.is_some()
        || app.dff_repair_rx.is_some()
        || app.collision_generation_job.is_some()
        || app.shadow_mesh_generation_job.is_some()
        || app.lod_generation_job.is_some()
    {
        app.status_message =
            "2DFX corona generation cannot start while another asset writer is running."
                .to_string();
        return false;
    }
    let mut byte_overrides = app.pending_replacement_assets.clone();
    for (key, bytes) in &app.editing.modified_entries {
        if app.editing.deleted_entries.contains(key) || !key.ends_with(".dff") {
            continue;
        }
        let name = app
            .editing
            .rows
            .iter()
            .find(|row| lower(&row.entry.name) == *key)
            .map(|row| row.entry.name.clone())
            .unwrap_or_else(|| key.clone());
        byte_overrides.insert(key.clone(), (name, bytes.clone()));
    }
    let mut raw_overrides = BTreeMap::new();
    if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_ref()
        && dff.dirty
    {
        raw_overrides.insert(editing_key(&dff.name), (dff.name.clone(), dff.raw.clone()));
    }
    let mut vertex_mesh_overrides = BTreeMap::<String, Vec<(String, RenderMesh)>>::new();
    for mesh_key in &app.pending_vertex_light_meshes {
        let Some(mesh) = app.meshes.get(mesh_key) else {
            continue;
        };
        let dff_name = mesh_key.split('|').next().unwrap_or(mesh_key);
        vertex_mesh_overrides
            .entry(asset_key(dff_name, ".dff"))
            .or_default()
            .push((mesh_key.clone(), mesh.clone()));
    }
    let context = CoronaGenerationContext {
        root: app.root.clone(),
        emitters: app.material_emitters.clone(),
        byte_overrides,
        raw_overrides,
        vertex_mesh_overrides,
        deleted_assets: app.pending_asset_deletes.clone(),
        building_dffs: building_dff_set(app),
    };
    app.corona_generation_job = Some(CoronaGenerationJob::new(context));
    app.status_message = "2DFX corona generation started in background...".to_string();
    true
}

pub(crate) fn update_corona_generation_job(app: &mut AppState) {
    let Some(mut job) = app.corona_generation_job.take() else {
        return;
    };
    if !job.step(app) {
        app.corona_generation_job = Some(job);
    }
}

#[derive(Clone)]
struct CollisionGenerationSource {
    dff_name: String,
    col_name: String,
    txd_name: Option<String>,
    raw_override: Option<RawMesh>,
    lod_only: bool,
    definition_ids_to_assign: Vec<String>,
}

#[derive(Clone)]
struct CollisionGenerationContext {
    root: PathBuf,
    sources: Vec<CollisionGenerationSource>,
    byte_overrides: BTreeMap<String, (String, Vec<u8>)>,
    material_classes: TextureMaterialClasses,
    txd_textures: TxdTextureIndex,
    preset: CollisionGenerationPreset,
    fallback_material: u8,
}

struct GeneratedCollisionAsset {
    col_name: String,
    mesh: CollisionMesh,
    bytes: Vec<u8>,
    definition_ids_to_assign: Vec<String>,
}

struct CollisionGenerationBatchResult {
    generated: Vec<GeneratedCollisionAsset>,
    delete_keys: Vec<String>,
    discarded_backed_flat_components: usize,
    errors: Vec<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CollisionGenerationScope {
    Editing,
    Global,
}

pub(crate) fn collision_generation_preset_label(preset: CollisionGenerationPreset) -> &'static str {
    match preset {
        CollisionGenerationPreset::Auto => "Auto",
        CollisionGenerationPreset::Architecture => "Architecture",
        CollisionGenerationPreset::Prop => "Prop",
        CollisionGenerationPreset::Shapes => "Shapes",
        CollisionGenerationPreset::Surface => "Surface",
    }
}

pub(crate) fn cycle_collision_generation_preset(app: &mut AppState) {
    app.collision_generation_preset = match app.collision_generation_preset {
        CollisionGenerationPreset::Auto => CollisionGenerationPreset::Architecture,
        CollisionGenerationPreset::Architecture => CollisionGenerationPreset::Prop,
        CollisionGenerationPreset::Prop => CollisionGenerationPreset::Shapes,
        CollisionGenerationPreset::Shapes => CollisionGenerationPreset::Surface,
        CollisionGenerationPreset::Surface => CollisionGenerationPreset::Auto,
    };
    app.status_message = format!(
        "Collision generation preset: {}",
        collision_generation_preset_label(app.collision_generation_preset)
    );
}

pub(crate) struct CollisionGenerationJob {
    rx: mpsc::Receiver<CollisionGenerationBatchResult>,
    progress_rx: mpsc::Receiver<String>,
    result: Option<CollisionGenerationBatchResult>,
    apply_index: usize,
    generated_total: usize,
    delete_total: usize,
    updated_definition_total: usize,
    scope: CollisionGenerationScope,
    safe_skipped: usize,
    global_reset_done: bool,
    started_at: Instant,
}

struct GeneratedShadowMesh {
    vertices: Vec<V3>,
    faces: Vec<CollisionFace>,
    welded_vertices: usize,
    closed_borders: usize,
    closed_components: usize,
}

enum ShadowMeshSourceSnapshot {
    Collision(CollisionMesh),
    Dff {
        name: String,
        parts: Vec<Vec<Vertex>>,
    },
}

pub(crate) struct ShadowMeshGenerationJob {
    rx: mpsc::Receiver<Result<GeneratedShadowMesh, String>>,
    target_name: String,
    source_label: String,
    started_at: Instant,
}

fn clear_staged_collision_replacement(app: &mut AppState, col_key: &str) {
    app.pending_replacement_assets.remove(col_key);
    app.pending_asset_deletes.remove(col_key);
    app.editing.modified_entries.remove(col_key);
    app.editing.deleted_entries.remove(col_key);
    app.editing.added_entries.remove(col_key);
}

fn collision_generation_writer_active(app: &AppState) -> bool {
    app.editing.save_rx.is_some()
        || app.collision_generation_job.is_some()
        || app.shadow_mesh_generation_job.is_some()
        || app.corona_generation_job.is_some()
        || app.fracture_generation_job.is_some()
        || app.dff_geometry_job.is_some()
        || app.txd_cleanup_job.is_some()
        || app.asset_optimization_scan_rx.is_some()
        || app.asset_optimization_job.is_some()
        || app.object_bounds_fix_job.is_some()
        || app.dff_repair_rx.is_some()
        || app.lod_generation_job.is_some()
}

fn generated_shadow_face(a: usize, b: usize, c: usize) -> Result<CollisionFace, String> {
    Ok(CollisionFace {
        a: u16::try_from(a)
            .map_err(|_| "Shadow mesh exceeds the 16-bit vertex-index limit".to_string())?,
        b: u16::try_from(b)
            .map_err(|_| "Shadow mesh exceeds the 16-bit vertex-index limit".to_string())?,
        c: u16::try_from(c)
            .map_err(|_| "Shadow mesh exceeds the 16-bit vertex-index limit".to_string())?,
        material: 0,
        light: 255,
        img_path: PathBuf::new(),
        material_file_offset: 0,
        light_file_offset: 0,
    })
}

pub(crate) fn snap_shadow_vertices_to_col_grid(vertices: &mut [V3]) -> Result<(), String> {
    let snap = |value: f32| {
        let scaled = (value * 128.0).round();
        if !scaled.is_finite() || scaled < i16::MIN as f32 || scaled > i16::MAX as f32 {
            Err("Shadow vertex is outside the COL fixed-point coordinate range".to_string())
        } else {
            Ok(scaled / 128.0)
        }
    };
    for vertex in vertices {
        vertex.x = snap(vertex.x)?;
        vertex.y = snap(vertex.y)?;
        vertex.z = snap(vertex.z)?;
    }
    Ok(())
}

/// Makes every connected closed shell consistently clockwise, matching the
/// winding convention used by GTA's stock shadow meshes.
pub(crate) fn orient_closed_shadow_components(
    vertices: &[V3],
    faces: &mut [CollisionFace],
) -> Result<usize, String> {
    type EdgeUse = (usize, u16, u16);
    let mut edges = HashMap::<(u16, u16), Vec<EdgeUse>>::new();
    for (face_index, face) in faces.iter().enumerate() {
        let indices = [face.a, face.b, face.c];
        if indices
            .iter()
            .any(|index| *index as usize >= vertices.len())
        {
            return Err(format!(
                "Shadow face {} references a missing vertex",
                face_index + 1
            ));
        }
        if face.a == face.b || face.b == face.c || face.c == face.a {
            return Err(format!(
                "Shadow face {} collapses to a line after COL quantization",
                face_index + 1
            ));
        }
        let a = to_mq(vertices[face.a as usize]);
        let b = to_mq(vertices[face.b as usize]);
        let c = to_mq(vertices[face.c as usize]);
        // Snapped COL coordinates are exact binary fractions. Test exact zero
        // here: f32::EPSILON is much larger than the squared area of the
        // smallest valid fixed-point triangles.
        if (b - a).cross(c - a).length_squared() == 0.0 {
            return Err(format!(
                "Shadow face {} has zero area after COL quantization",
                face_index + 1
            ));
        }
        for (from, to) in [(face.a, face.b), (face.b, face.c), (face.c, face.a)] {
            let key = if from < to { (from, to) } else { (to, from) };
            edges.entry(key).or_default().push((face_index, from, to));
        }
    }

    let mut adjacency = vec![Vec::<(usize, bool)>::new(); faces.len()];
    for (edge, uses) in edges {
        if uses.len() != 2 {
            return Err(format!(
                "Shadow mesh is not a closed skin at edge {}-{} (used by {} faces)",
                edge.0,
                edge.1,
                uses.len()
            ));
        }
        let (left_face, left_from, left_to) = uses[0];
        let (right_face, right_from, right_to) = uses[1];
        let same_direction = left_from == right_from && left_to == right_to;
        adjacency[left_face].push((right_face, same_direction));
        adjacency[right_face].push((left_face, same_direction));
    }

    let mut flip = vec![None; faces.len()];
    let mut components = Vec::<Vec<usize>>::new();
    for seed in 0..faces.len() {
        if flip[seed].is_some() {
            continue;
        }
        flip[seed] = Some(false);
        let mut queue = std::collections::VecDeque::from([seed]);
        let mut component = Vec::new();
        while let Some(face_index) = queue.pop_front() {
            component.push(face_index);
            let face_flip = flip[face_index].unwrap_or(false);
            for &(neighbor, same_direction) in &adjacency[face_index] {
                let neighbor_flip = face_flip ^ same_direction;
                match flip[neighbor] {
                    Some(existing) if existing != neighbor_flip => {
                        return Err(
                            "Shadow mesh contains a non-orientable closed component".to_string()
                        );
                    }
                    Some(_) => {}
                    None => {
                        flip[neighbor] = Some(neighbor_flip);
                        queue.push_back(neighbor);
                    }
                }
            }
        }
        components.push(component);
    }

    for (face, should_flip) in faces.iter_mut().zip(flip) {
        if should_flip.unwrap_or(false) {
            std::mem::swap(&mut face.b, &mut face.c);
        }
    }
    for component in &components {
        let signed_volume = component.iter().fold(0.0_f64, |volume, face_index| {
            let face = &faces[*face_index];
            let a = vertices[face.a as usize];
            let b = vertices[face.b as usize];
            let c = vertices[face.c as usize];
            volume
                + (a.x as f64 * (b.y as f64 * c.z as f64 - b.z as f64 * c.y as f64)
                    + a.y as f64 * (b.z as f64 * c.x as f64 - b.x as f64 * c.z as f64)
                    + a.z as f64 * (b.x as f64 * c.y as f64 - b.y as f64 * c.x as f64))
                    / 6.0
        });
        if signed_volume.abs() <= 1.0e-12 {
            return Err("Shadow mesh contains a closed component with zero volume".to_string());
        }
        if signed_volume > 0.0 {
            for face_index in component {
                std::mem::swap(&mut faces[*face_index].b, &mut faces[*face_index].c);
            }
        }
    }
    Ok(components.len())
}

fn finish_generated_shadow_mesh(
    mut vertices: Vec<V3>,
    mut faces: Vec<CollisionFace>,
    welded_vertices: usize,
    closed_borders: usize,
) -> Result<GeneratedShadowMesh, String> {
    snap_shadow_vertices_to_col_grid(&mut vertices)?;
    let closed_components = orient_closed_shadow_components(&vertices, &mut faces)?;
    for face in &mut faces {
        face.material = 0;
        face.light = 255;
    }
    Ok(GeneratedShadowMesh {
        vertices,
        faces,
        welded_vertices,
        closed_borders,
        closed_components,
    })
}

fn shadow_triangle_has_area(vertices: &[V3], a: usize, b: usize, c: usize) -> bool {
    let (Some(a), Some(b), Some(c)) = (vertices.get(a), vertices.get(b), vertices.get(c)) else {
        return false;
    };
    (to_mq(*b) - to_mq(*a))
        .cross(to_mq(*c) - to_mq(*a))
        .length_squared()
        > 0.0
}

fn append_closed_shadow_border_faces(
    vertices: &[V3],
    faces: &mut Vec<CollisionFace>,
    a: usize,
    b: usize,
    inner_offset: usize,
) -> Result<(), String> {
    let a_inner = a + inner_offset;
    let b_inner = b + inner_offset;
    let primary = [(a, b, b_inner), (a, b_inner, a_inner)];
    let alternate = [(a, b, a_inner), (b, b_inner, a_inner)];
    let triangles = if primary
        .iter()
        .all(|(a, b, c)| shadow_triangle_has_area(vertices, *a, *b, *c))
    {
        primary
    } else if alternate
        .iter()
        .all(|(a, b, c)| shadow_triangle_has_area(vertices, *a, *b, *c))
    {
        alternate
    } else {
        return Err(format!(
            "Shadow border edge {a}-{b} collapses after COL quantization"
        ));
    };
    for (a, b, c) in triangles {
        faces.push(generated_shadow_face(a, b, c)?);
    }
    Ok(())
}

fn build_closed_shadow_faces(
    vertices: &[V3],
    source_faces: &[CollisionFace],
    boundary_edges: &[(u16, u16)],
    vertex_count: usize,
) -> Result<Vec<CollisionFace>, String> {
    let mut faces =
        Vec::with_capacity(source_faces.len() * 2 + boundary_edges.len().saturating_mul(2));
    for face in source_faces {
        faces.push(generated_shadow_face(
            face.a as usize,
            face.b as usize,
            face.c as usize,
        )?);
        faces.push(generated_shadow_face(
            face.c as usize + vertex_count,
            face.b as usize + vertex_count,
            face.a as usize + vertex_count,
        )?);
    }
    for &(a, b) in boundary_edges {
        append_closed_shadow_border_faces(
            vertices,
            &mut faces,
            a as usize,
            b as usize,
            vertex_count,
        )?;
    }
    Ok(faces)
}

fn grid_safe_shadow_offset_vertices(
    source_vertices: &[V3],
    boundary_edges: &[(u16, u16)],
) -> Result<Vec<V3>, String> {
    let mut selected = None;
    'magnitude: for magnitude in 1i32..=4 {
        for x in -magnitude..=magnitude {
            for y in -magnitude..=magnitude {
                for z in -magnitude..=magnitude {
                    if x.abs().max(y.abs()).max(z.abs()) != magnitude {
                        continue;
                    }
                    let offset = Vec3::new(x as f32, y as f32, z as f32);
                    if boundary_edges.iter().any(|&(a, b)| {
                        let edge =
                            to_mq(source_vertices[b as usize]) - to_mq(source_vertices[a as usize]);
                        edge.cross(offset).length_squared() == 0.0
                    }) {
                        continue;
                    }
                    let fits = source_vertices.iter().all(|vertex| {
                        [vertex.x, vertex.y, vertex.z]
                            .into_iter()
                            .zip([x, y, z])
                            .all(|(value, delta)| {
                                let fixed = (value * 128.0).round() as i32;
                                fixed + delta >= i16::MIN as i32
                                    && fixed + delta <= i16::MAX as i32
                                    && fixed - delta >= i16::MIN as i32
                                    && fixed - delta <= i16::MAX as i32
                            })
                    });
                    if fits {
                        selected = Some(V3 {
                            x: x as f32 / 128.0,
                            y: y as f32 / 128.0,
                            z: z as f32 / 128.0,
                        });
                        break 'magnitude;
                    }
                }
            }
        }
    }
    let offset = selected.ok_or_else(|| {
        "Could not find a COL-grid extrusion direction for all shadow border edges".to_string()
    })?;
    let mut vertices = Vec::with_capacity(source_vertices.len() * 2);
    for vertex in source_vertices {
        vertices.push(V3 {
            x: vertex.x + offset.x,
            y: vertex.y + offset.y,
            z: vertex.z + offset.z,
        });
    }
    for vertex in source_vertices {
        vertices.push(V3 {
            x: vertex.x - offset.x,
            y: vertex.y - offset.y,
            z: vertex.z - offset.z,
        });
    }
    Ok(vertices)
}

fn sanitize_shadow_source_for_col(mesh: &mut CollisionMesh) -> Result<usize, String> {
    snap_shadow_vertices_to_col_grid(&mut mesh.vertices)?;
    // After snapping, distinct source points can occupy the same fixed-point
    // coordinate even when their original distance was just over the weld
    // threshold. A half-grid threshold merges only exact snapped matches.
    let merged = merge_col_vertices_by_distance(mesh, 1.0 / 256.0);
    let mut seen = HashSet::<[u16; 3]>::new();
    mesh.faces.retain(|face| {
        let mut canonical = [face.a, face.b, face.c];
        canonical.sort_unstable();
        (face.a as usize) < mesh.vertices.len()
            && (face.b as usize) < mesh.vertices.len()
            && (face.c as usize) < mesh.vertices.len()
            && shadow_triangle_has_area(
                &mesh.vertices,
                face.a as usize,
                face.b as usize,
                face.c as usize,
            )
            && seen.insert(canonical)
    });
    compact_col_mesh_vertices(mesh);
    if mesh.faces.is_empty() {
        return Err("Shadow source contained only triangles lost to COL quantization".to_string());
    }
    Ok(merged)
}

fn append_shadow_source_box(
    vertices: &mut Vec<V3>,
    faces: &mut Vec<CollisionFace>,
    col_box: &CollisionBox,
) -> Result<(), String> {
    let min = V3 {
        x: col_box.min.x.min(col_box.max.x),
        y: col_box.min.y.min(col_box.max.y),
        z: col_box.min.z.min(col_box.max.z),
    };
    let max = V3 {
        x: col_box.min.x.max(col_box.max.x),
        y: col_box.min.y.max(col_box.max.y),
        z: col_box.min.z.max(col_box.max.z),
    };
    let first = vertices.len();
    vertices.extend([
        V3 {
            x: min.x,
            y: min.y,
            z: min.z,
        },
        V3 {
            x: max.x,
            y: min.y,
            z: min.z,
        },
        V3 {
            x: max.x,
            y: max.y,
            z: min.z,
        },
        V3 {
            x: min.x,
            y: max.y,
            z: min.z,
        },
        V3 {
            x: min.x,
            y: min.y,
            z: max.z,
        },
        V3 {
            x: max.x,
            y: min.y,
            z: max.z,
        },
        V3 {
            x: max.x,
            y: max.y,
            z: max.z,
        },
        V3 {
            x: min.x,
            y: max.y,
            z: max.z,
        },
    ]);
    for [a, b, c] in [
        [0, 2, 1],
        [0, 3, 2],
        [4, 5, 6],
        [4, 6, 7],
        [0, 1, 5],
        [0, 5, 4],
        [1, 2, 6],
        [1, 6, 5],
        [2, 3, 7],
        [2, 7, 6],
        [3, 0, 4],
        [3, 4, 7],
    ] {
        faces.push(generated_shadow_face(first + a, first + b, first + c)?);
    }
    Ok(())
}

fn append_shadow_source_sphere(
    vertices: &mut Vec<V3>,
    faces: &mut Vec<CollisionFace>,
    sphere: &CollisionSphere,
) -> Result<(), String> {
    const RINGS: usize = 8;
    const SIDES: usize = 12;
    let radius = sphere.radius.abs();
    if radius <= 0.00001 {
        return Ok(());
    }
    let first = vertices.len();
    vertices.push(V3 {
        x: sphere.center.x,
        y: sphere.center.y,
        z: sphere.center.z + radius,
    });
    for ring in 1..RINGS {
        let phi = std::f32::consts::PI * ring as f32 / RINGS as f32;
        let ring_radius = radius * phi.sin();
        let z = sphere.center.z + radius * phi.cos();
        for side in 0..SIDES {
            let theta = std::f32::consts::TAU * side as f32 / SIDES as f32;
            vertices.push(V3 {
                x: sphere.center.x + ring_radius * theta.cos(),
                y: sphere.center.y + ring_radius * theta.sin(),
                z,
            });
        }
    }
    let bottom = vertices.len();
    vertices.push(V3 {
        x: sphere.center.x,
        y: sphere.center.y,
        z: sphere.center.z - radius,
    });
    let ring_index = |ring: usize, side: usize| first + 1 + ring * SIDES + side % SIDES;
    for side in 0..SIDES {
        faces.push(generated_shadow_face(
            first,
            ring_index(0, side + 1),
            ring_index(0, side),
        )?);
    }
    for ring in 0..RINGS - 2 {
        for side in 0..SIDES {
            let a = ring_index(ring, side);
            let b = ring_index(ring, side + 1);
            let c = ring_index(ring + 1, side + 1);
            let d = ring_index(ring + 1, side);
            faces.push(generated_shadow_face(a, b, c)?);
            faces.push(generated_shadow_face(a, c, d)?);
        }
    }
    for side in 0..SIDES {
        faces.push(generated_shadow_face(
            ring_index(RINGS - 2, side),
            ring_index(RINGS - 2, side + 1),
            bottom,
        )?);
    }
    Ok(())
}

fn append_shadow_source_primitives(mesh: &mut CollisionMesh) -> Result<(), String> {
    for col_box in &mesh.boxes {
        append_shadow_source_box(&mut mesh.vertices, &mut mesh.faces, col_box)?;
    }
    for sphere in &mesh.spheres {
        append_shadow_source_sphere(&mut mesh.vertices, &mut mesh.faces, sphere)?;
    }
    Ok(())
}

fn generate_closed_shadow_skin(mut source: CollisionMesh) -> Result<GeneratedShadowMesh, String> {
    append_shadow_source_primitives(&mut source)?;
    if source.faces.is_empty() {
        return Err("The DFF overlay and collision mesh contain no triangles or primitives".into());
    }
    // DFF previews duplicate vertices per triangle. COL coordinates quantize
    // to 1/128 units, so welding at that scale closes those representational
    // seams without erasing meaningful model detail.
    let mut welded_vertices = merge_col_vertices_by_distance(&mut source, 1.0 / 128.0);
    welded_vertices += sanitize_shadow_source_for_col(&mut source)?;
    if source.vertices.len() > u16::MAX as usize {
        return Err(format!(
            "Shadow source has {} welded vertices; the COL limit is {}",
            source.vertices.len(),
            u16::MAX
        ));
    }
    if source.faces.is_empty() {
        return Err("Shadow source contained only degenerate triangles".into());
    }

    let mut edges = HashMap::<(u16, u16), Vec<(u16, u16)>>::new();
    for face in &source.faces {
        for (a, b) in [(face.a, face.b), (face.b, face.c), (face.c, face.a)] {
            let key = if a < b { (a, b) } else { (b, a) };
            edges.entry(key).or_default().push((a, b));
        }
    }
    let boundary_edges = edges
        .values()
        .filter_map(|uses| (uses.len() == 1).then_some(uses[0]))
        .collect::<Vec<_>>();
    if boundary_edges.is_empty() {
        return finish_generated_shadow_mesh(source.vertices, source.faces, welded_vertices, 0);
    }

    let mut normals = vec![Vec3::ZERO; source.vertices.len()];
    for face in &source.faces {
        let (a, b, c) = (
            source.vertices[face.a as usize],
            source.vertices[face.b as usize],
            source.vertices[face.c as usize],
        );
        let normal = (to_mq(b) - to_mq(a)).cross(to_mq(c) - to_mq(a));
        normals[face.a as usize] += normal;
        normals[face.b as usize] += normal;
        normals[face.c as usize] += normal;
    }
    let bounds = bounds_from_vertices(&source.vertices);
    let extent = bounds.max - bounds.min;
    let thickness = (extent.x.max(extent.y).max(extent.z) * 0.0025).clamp(1.0 / 64.0, 0.5);
    let center = (bounds.min + bounds.max) * 0.5;
    let vertex_count = source.vertices.len();
    if vertex_count.saturating_mul(2) > u16::MAX as usize {
        return Err(format!(
            "Closing the shadow skin needs {} vertices; the COL limit is {}",
            vertex_count * 2,
            u16::MAX
        ));
    }
    let mut vertices = Vec::with_capacity(vertex_count * 2);
    for (idx, vertex) in source.vertices.iter().enumerate() {
        let point = to_mq(*vertex);
        let fallback = (point - center).normalize_or_zero();
        let normal = if normals[idx].length_squared() > 0.000001 {
            normals[idx].normalize()
        } else if fallback.length_squared() > 0.000001 {
            fallback
        } else {
            Vec3::Z
        };
        vertices.push(from_mq(point + normal * (thickness * 0.5)));
    }
    for (idx, vertex) in source.vertices.iter().enumerate() {
        let point = to_mq(*vertex);
        let fallback = (point - center).normalize_or_zero();
        let normal = if normals[idx].length_squared() > 0.000001 {
            normals[idx].normalize()
        } else if fallback.length_squared() > 0.000001 {
            fallback
        } else {
            Vec3::Z
        };
        vertices.push(from_mq(point - normal * (thickness * 0.5)));
    }
    // Choose border triangulation using the exact fixed-point coordinates that
    // will be written. A valid quad can have one diagonal produce a collinear
    // triangle after COL quantization even though the other diagonal is safe.
    snap_shadow_vertices_to_col_grid(&mut vertices)?;
    let faces =
        match build_closed_shadow_faces(&vertices, &source.faces, &boundary_edges, vertex_count) {
            Ok(faces) => faces,
            Err(_) => {
                // Averaged vertex normals can become parallel to a particular
                // boundary edge. Fall back to a small integer-grid translation
                // selected to be non-parallel to every boundary edge.
                vertices = grid_safe_shadow_offset_vertices(&source.vertices, &boundary_edges)?;
                build_closed_shadow_faces(&vertices, &source.faces, &boundary_edges, vertex_count)?
            }
        };
    finish_generated_shadow_mesh(vertices, faces, welded_vertices, boundary_edges.len())
}

fn collision_mesh_from_dff_snapshot(name: &str, parts: Vec<Vec<Vertex>>) -> CollisionMesh {
    let mut vertices = Vec::new();
    let mut faces = Vec::new();
    for part in parts {
        for triangle in part.chunks_exact(3) {
            let first = vertices.len();
            vertices.extend([triangle[0].pos, triangle[1].pos, triangle[2].pos]);
            if let Ok(face) = generated_shadow_face(first, first + 1, first + 2) {
                faces.push(face);
            }
        }
    }
    let bounds = bounds_from_vertices(&vertices);
    CollisionMesh {
        name: name.to_string(),
        spheres: Vec::new(),
        boxes: Vec::new(),
        vertices,
        faces,
        bounds,
        shadow_vertices: Vec::new(),
        shadow_faces: Vec::new(),
    }
}

pub(crate) fn request_editing_shadow_mesh_generation(app: &mut AppState) {
    if collision_generation_writer_active(app) {
        app.status_message =
            "Shadow generation cannot start while another asset writer is running.".to_string();
        return;
    }
    let Some(EditingAsset::Col(col)) = app.editing.asset.as_ref() else {
        return;
    };
    let target_name = col.name.clone();
    let (source, source_label) = if let Some(overlay) = col.dff_overlay.as_ref() {
        let overlay_name = col
            .dff_overlay_name
            .as_deref()
            .unwrap_or(&target_name)
            .to_string();
        let parts = overlay
            .parts
            .iter()
            .map(|part| {
                let mut included = Vec::with_capacity(part.cpu_vertices.len());
                for (triangle_index, triangle) in part.cpu_vertices.chunks_exact(3).enumerate() {
                    let face = part
                        .face_indices
                        .get(triangle_index)
                        .copied()
                        .unwrap_or(triangle_index);
                    if dff_face_casts_shadow_from_parts(
                        &app.shadow_casting,
                        &overlay_name,
                        face,
                        part.material_index,
                        &part.texture_name,
                    ) {
                        included.extend_from_slice(triangle);
                    }
                }
                included
            })
            .filter(|part| !part.is_empty())
            .collect();
        (
            ShadowMeshSourceSnapshot::Dff {
                name: target_name.clone(),
                // Snapshot only thread-safe CPU data. Index reconstruction,
                // welding and skin closure all happen on the worker.
                parts,
            },
            col.dff_overlay_name
                .as_deref()
                .map(|name| format!("DFF overlay {name}"))
                .unwrap_or_else(|| "DFF overlay".to_string()),
        )
    } else {
        (
            ShadowMeshSourceSnapshot::Collision(normalized_editing_col_mesh(col)),
            "regular collision mesh".to_string(),
        )
    };
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = std::panic::catch_unwind(|| {
            let source = match source {
                ShadowMeshSourceSnapshot::Collision(mesh) => mesh,
                ShadowMeshSourceSnapshot::Dff { name, parts } => {
                    collision_mesh_from_dff_snapshot(&name, parts)
                }
            };
            generate_closed_shadow_skin(source)
        })
        .unwrap_or_else(|_| Err("Shadow mesh generation worker crashed".to_string()));
        let _ = tx.send(result);
    });
    app.shadow_mesh_generation_job = Some(ShadowMeshGenerationJob {
        rx,
        target_name,
        source_label: source_label.clone(),
        started_at: Instant::now(),
    });
    app.status_message = format!("Generating closed shadow skin from {source_label}...");
}

pub(crate) fn update_shadow_mesh_generation_job(app: &mut AppState) {
    let Some(job) = app.shadow_mesh_generation_job.take() else {
        return;
    };
    let result = match job.rx.try_recv() {
        Ok(result) => result,
        Err(mpsc::TryRecvError::Empty) => {
            app.shadow_mesh_generation_job = Some(job);
            return;
        }
        Err(mpsc::TryRecvError::Disconnected) => {
            app.status_message = "Shadow mesh generation worker disconnected".to_string();
            return;
        }
    };
    let elapsed = job.started_at.elapsed().as_secs_f32();
    let generated = match result {
        Ok(generated) => generated,
        Err(err) => {
            app.status_message = format!("Could not generate shadow mesh: {err}");
            return;
        }
    };
    let editing_target = matches!(
        app.editing.asset.as_ref(),
        Some(EditingAsset::Col(col)) if col.name.eq_ignore_ascii_case(&job.target_name)
    );
    if !editing_target {
        app.status_message = format!(
            "Generated shadow mesh for {}, but that COL is no longer open",
            job.target_name
        );
        return;
    }
    let before = editing_history_snapshot(app);
    let welded_vertices = generated.welded_vertices;
    let closed_borders = generated.closed_borders;
    let closed_components = generated.closed_components;
    let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() else {
        return;
    };
    if col.editing_shadow {
        col.mesh.vertices = generated.vertices;
        col.mesh.faces = generated.faces;
        col.mesh.bounds = bounds_from_vertices(&col.mesh.vertices);
    } else {
        col.mesh.shadow_vertices = generated.vertices;
        col.mesh.shadow_faces = generated.faces;
        swap_editing_col_mesh_layer(col);
    }
    col.dirty = true;
    col.selected_face = 0;
    col_clear_selection(col);
    let vertex_count = col.mesh.vertices.len();
    let face_count = col.mesh.faces.len();
    commit_editing_history(app, "Generate Shadow Mesh", before);
    app.status_message = format!(
        "Generated shadow skin from {} in {:.2}s: {} vertices, {} faces; welded {}, closed {} border edges, oriented {} closed shells",
        job.source_label,
        elapsed,
        vertex_count,
        face_count,
        welded_vertices,
        closed_borders,
        closed_components
    );
}

fn collision_generation_source_materials(
    classes: &TextureMaterialClasses,
    txd_textures: &TxdTextureIndex,
    txd_name: Option<&str>,
    raw: &RawMesh,
    fallback: u8,
) -> (Vec<Option<u8>>, BTreeSet<u16>) {
    let mut source_materials = Vec::with_capacity(raw.material_textures.len());
    let mut excluded = BTreeSet::new();
    for (index, texture) in raw.material_textures.iter().enumerate() {
        let fingerprint = texture_content_fingerprint(txd_textures, texture, txd_name);
        let resolved = classes.resolve_with_content(txd_name, texture, fingerprint, fallback);
        source_materials.push(Some(resolved.material));
        if resolved.no_collision
            && let Ok(index) = u16::try_from(index)
        {
            excluded.insert(index);
        }
    }
    (source_materials, excluded)
}

fn empty_collision_mesh(name: &str, bounds: Bounds) -> CollisionMesh {
    CollisionMesh {
        name: name.to_string(),
        spheres: Vec::new(),
        boxes: Vec::new(),
        vertices: Vec::new(),
        faces: Vec::new(),
        bounds,
        shadow_vertices: Vec::new(),
        shadow_faces: Vec::new(),
    }
}

fn serialize_generated_collision(
    root: &Path,
    byte_overrides: &BTreeMap<String, (String, Vec<u8>)>,
    source: &CollisionGenerationSource,
    mesh: CollisionMesh,
) -> Result<GeneratedCollisionAsset, String> {
    let col_key = asset_key(&source.col_name, ".col");
    let template_bytes = byte_overrides
        .get(&col_key)
        .map(|(_, bytes)| bytes.clone())
        .or_else(|| find_col_entry(root, &source.col_name).map(|entry| read_img_entry(&entry)))
        .unwrap_or_default();
    let template = col_regeneration_template(&template_bytes, &source.col_name);
    let header_bounds = mesh.bounds;
    let mut bytes = write_col_mesh_from_template_with_bounds(&template, &mesh, Some(header_bounds))
        .map_err(|err| {
            format!(
                "{}: could not serialize regenerated COL: {err}",
                source.col_name
            )
        })?;
    set_col_model_names_from_entry(&mut bytes, &source.col_name);
    let issues = validate_col_for_game_load(&source.col_name, &bytes);
    if col_validation_has_errors(&issues) {
        return Err(format!(
            "{}: regenerated COL failed game-load validation",
            source.col_name
        ));
    }
    Ok(GeneratedCollisionAsset {
        col_name: source.col_name.clone(),
        mesh,
        bytes,
        definition_ids_to_assign: source.definition_ids_to_assign.clone(),
    })
}

fn run_collision_generation(
    context: CollisionGenerationContext,
    progress: &mpsc::Sender<String>,
) -> CollisionGenerationBatchResult {
    let mut target_keys = context
        .sources
        .iter()
        .map(|source| asset_key(&source.col_name, ".col"))
        .collect::<Vec<_>>();
    target_keys.sort();
    target_keys.dedup();
    let total = context.sources.len();
    let mut generated = Vec::new();
    let mut discarded_backed_flat_components = 0usize;
    let mut errors = Vec::new();
    for (index, source) in context.sources.into_iter().enumerate() {
        let _ = progress.send(format!(
            "Collision generation: processing {} ({}/{total})",
            source.dff_name,
            index + 1
        ));
        let raw = match source.raw_override.as_ref() {
            Some(raw) => Some(raw.clone()),
            None => {
                let key = asset_key(&source.dff_name, ".dff");
                let bytes = context
                    .byte_overrides
                    .get(&key)
                    .map(|(_, bytes)| bytes.clone())
                    .or_else(|| {
                        find_dff_entry(&context.root, &source.dff_name)
                            .map(|entry| read_img_entry(&entry))
                    });
                if bytes.is_none() {
                    errors.push(format!("{}: DFF source was not found", source.dff_name));
                }
                bytes.map(|bytes| parse_dff_mesh(&bytes))
            }
        };
        let mesh = match raw {
            Some(raw) if !raw.vertices.is_empty() && !raw.triangles.is_empty() => {
                let (source_materials, excluded_source_materials) =
                    collision_generation_source_materials(
                        &context.material_classes,
                        &context.txd_textures,
                        source.txd_name.as_deref(),
                        &raw,
                        context.fallback_material,
                    );
                let settings = CollisionGenerationSettings::for_preset(
                    context.preset,
                    context.fallback_material,
                    source_materials,
                )
                .with_excluded_source_materials(excluded_source_materials)
                .with_empty_collision(source.lod_only);
                match generate_collision(&raw, &source.col_name, &settings) {
                    Ok(result) => {
                        discarded_backed_flat_components +=
                            result.stats.discarded_backed_flat_components;
                        result.mesh
                    }
                    Err(err) => {
                        errors.push(format!(
                            "{}: {err}; old referenced collision was cleared",
                            source.dff_name
                        ));
                        empty_collision_mesh(&source.col_name, bounds_from_vertices(&raw.vertices))
                    }
                }
            }
            Some(raw) => {
                errors.push(format!(
                    "{}: DFF has no usable geometry; old referenced collision was cleared",
                    source.dff_name
                ));
                empty_collision_mesh(&source.col_name, bounds_from_vertices(&raw.vertices))
            }
            None => empty_collision_mesh(
                &source.col_name,
                Bounds {
                    min: Vec3::ZERO,
                    max: Vec3::ZERO,
                },
            ),
        };
        let was_empty = mesh.faces.is_empty() && mesh.boxes.is_empty() && mesh.spheres.is_empty();
        let dff_bounds = mesh.bounds;
        match serialize_generated_collision(&context.root, &context.byte_overrides, &source, mesh) {
            Ok(asset) => generated.push(asset),
            Err(err) => {
                errors.push(err);
                if !was_empty {
                    match serialize_generated_collision(
                        &context.root,
                        &context.byte_overrides,
                        &source,
                        empty_collision_mesh(&source.col_name, dff_bounds),
                    ) {
                        Ok(asset) => {
                            errors.push(format!(
                                "{}: old referenced collision was cleared after regeneration failed",
                                source.col_name
                            ));
                            generated.push(asset);
                        }
                        Err(empty_err) => errors.push(empty_err),
                    }
                }
            }
        }
    }
    generated.sort_by(|a, b| lower(&a.col_name).cmp(&lower(&b.col_name)));
    let generated_keys = generated
        .iter()
        .map(|asset| asset_key(&asset.col_name, ".col"))
        .collect::<HashSet<_>>();
    let delete_keys = target_keys
        .into_iter()
        .filter(|key| !generated_keys.contains(key))
        .collect();
    CollisionGenerationBatchResult {
        generated,
        delete_keys,
        discarded_backed_flat_components,
        errors,
    }
}

impl CollisionGenerationJob {
    fn new(
        context: CollisionGenerationContext,
        scope: CollisionGenerationScope,
        safe_skipped: usize,
    ) -> Self {
        let (tx, rx) = mpsc::channel();
        let (progress_tx, progress_rx) = mpsc::channel();
        thread::spawn(move || {
            let result =
                std::panic::catch_unwind(|| run_collision_generation(context, &progress_tx))
                    .unwrap_or_else(|panic| {
                        let detail = panic
                            .downcast_ref::<&str>()
                            .map(|value| (*value).to_string())
                            .or_else(|| panic.downcast_ref::<String>().cloned())
                            .unwrap_or_else(|| "unknown collision generation panic".to_string());
                        CollisionGenerationBatchResult {
                            generated: Vec::new(),
                            delete_keys: Vec::new(),
                            discarded_backed_flat_components: 0,
                            errors: vec![format!("Collision generation worker crashed: {detail}")],
                        }
                    });
            let _ = tx.send(result);
        });
        Self {
            rx,
            progress_rx,
            result: None,
            apply_index: 0,
            generated_total: 0,
            delete_total: 0,
            updated_definition_total: 0,
            scope,
            safe_skipped,
            global_reset_done: false,
            started_at: Instant::now(),
        }
    }

    fn step(&mut self, app: &mut AppState) -> bool {
        if let Some(message) = self.progress_rx.try_iter().last() {
            app.status_message = message;
        }
        if self.result.is_none() {
            match self.rx.try_recv() {
                Ok(result) => {
                    self.generated_total = result.generated.len();
                    self.delete_total = result.delete_keys.len();
                    self.result = Some(result);
                }
                Err(mpsc::TryRecvError::Empty) => return false,
                Err(mpsc::TryRecvError::Disconnected) => {
                    app.status_message = "Collision generation worker disconnected".to_string();
                    return true;
                }
            }
        }
        if self.scope == CollisionGenerationScope::Global && !self.global_reset_done {
            // These byte-offset edits point into the old COL payloads and are
            // invalid after full regeneration.
            app.pending_col_writes.clear();
            app.selected_col_face = None;
            app.selected_col_vertex = 0;
            app.hovered_col_face = None;
            app.hovered_col_vertex = None;
            self.global_reset_done = true;
        }
        let refresh_started = Instant::now();
        let mut refreshed_this_frame = 0usize;
        loop {
            let asset = self
                .result
                .as_mut()
                .and_then(|result| result.generated.pop());
            if let Some(asset) = asset {
                let col_key = asset_key(&asset.col_name, ".col");
                self.updated_definition_total += assign_generated_col_to_definitions(
                    app,
                    &asset.definition_ids_to_assign,
                    &asset.col_name,
                );
                if self.scope == CollisionGenerationScope::Global {
                    clear_staged_collision_replacement(app, &col_key);
                }
                let editing_this_col = matches!(
                    app.editing.asset.as_ref(),
                    Some(EditingAsset::Col(col)) if editing_key(&col.name) == col_key
                );
                if self.scope == CollisionGenerationScope::Editing {
                    if editing_this_col {
                        if let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() {
                            let current = normalized_editing_col_mesh(col);
                            let mut replacement = asset.mesh.clone();
                            replacement.shadow_vertices = current.shadow_vertices;
                            replacement.shadow_faces = current.shadow_faces;
                            col.mesh = replacement;
                            col.editing_shadow = false;
                            col_clear_selection(col);
                            col.selected_face = 0;
                            col.face_scroll = 0.0;
                            col.primitive_scroll = 0.0;
                            col.dirty = true;
                        }
                    } else {
                        let fallback_img_path = app.root.join("imgs").join(REPLACEMENT_IMG);
                        editing_stage_added_entry(
                            &mut app.editing,
                            fallback_img_path,
                            &asset.col_name,
                            asset.bytes,
                        );
                        if asset.mesh.faces.is_empty()
                            && asset.mesh.boxes.is_empty()
                            && asset.mesh.spheres.is_empty()
                        {
                            app.collisions.remove(&col_key);
                        } else {
                            app.collisions.insert(col_key.clone(), asset.mesh);
                        }
                    }
                } else {
                    if editing_this_col {
                        if let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() {
                            col.mesh = asset.mesh.clone();
                            col.bytes = asset.bytes.clone();
                            col.editing_shadow = false;
                            col_clear_selection(col);
                            col.dirty = false;
                        }
                    }
                    if asset.mesh.faces.is_empty()
                        && asset.mesh.boxes.is_empty()
                        && asset.mesh.spheres.is_empty()
                    {
                        app.collisions.remove(&col_key);
                    } else {
                        app.collisions.insert(col_key.clone(), asset.mesh);
                    }
                    app.pending_replacement_assets
                        .insert(col_key, (asset.col_name, asset.bytes));
                }
            } else if self.scope == CollisionGenerationScope::Global
                && let Some(col_key) = self
                    .result
                    .as_mut()
                    .and_then(|result| result.delete_keys.pop())
            {
                clear_staged_collision_replacement(app, &col_key);
                app.collisions.remove(&col_key);
                app.pending_asset_deletes.insert(col_key);
            } else {
                break;
            }
            self.apply_index += 1;
            refreshed_this_frame += 1;
            app.loaded_wip = true;
            if self.scope == CollisionGenerationScope::Editing
                || refreshed_this_frame >= LIVE_RESOURCE_REFRESH_BATCH_LIMIT
                || refresh_started.elapsed() >= LIVE_RESOURCE_REFRESH_FRAME_BUDGET
            {
                break;
            }
        }
        if refreshed_this_frame > 0 {
            let refresh_total = self.generated_total + self.delete_total;
            app.status_message = format!("Refreshing COLs: {}/{}", self.apply_index, refresh_total);
            if self.apply_index < refresh_total {
                return false;
            }
        }
        invalidate_validation_cache(app);
        let changed_anything = self.generated_total + self.delete_total > 0;
        if changed_anything {
            clear_history_for_external_change(app);
        }
        let elapsed = self.started_at.elapsed().as_secs_f32();
        let safe_suffix = if self.safe_skipped > 0 {
            format!("; skipped {} Safe target(s)", self.safe_skipped)
        } else {
            String::new()
        };
        let definition_suffix = if self.updated_definition_total > 0 {
            format!("; updated {} definition(s)", self.updated_definition_total)
        } else {
            String::new()
        };
        let result = self
            .result
            .as_ref()
            .expect("collision generation result is set");
        let flat_detail_suffix = if result.discarded_backed_flat_components > 0 {
            format!(
                "; removed {} backed flat detail component(s)",
                result.discarded_backed_flat_components
            )
        } else {
            String::new()
        };
        app.status_message = if result.errors.is_empty() {
            format!(
                "Regenerated {} collision asset(s) in {elapsed:.1}s{flat_detail_suffix}{safe_suffix}{definition_suffix}. Save to apply.{}",
                self.generated_total,
                if changed_anything {
                    " Undo history cleared."
                } else {
                    ""
                }
            )
        } else {
            format!(
                "Regenerated {} collision asset(s) in {elapsed:.1}s with {} issue(s){flat_detail_suffix}{safe_suffix}{definition_suffix}. Click status for details.{}",
                self.generated_total,
                result.errors.len(),
                if changed_anything {
                    " Undo history cleared."
                } else {
                    ""
                }
            )
        };
        if !result.errors.is_empty() {
            let mut log = vec![app.status_message.clone()];
            log.extend(result.errors.clone());
            app.save_log = log;
            app.save_log_open = true;
            app.save_log_follow_tail = true;
            app.save_log_scroll = f32::MAX;
        }
        true
    }
}

fn collision_generation_byte_overrides(app: &AppState) -> BTreeMap<String, (String, Vec<u8>)> {
    let mut overrides = app.pending_replacement_assets.clone();
    for (key, bytes) in &app.editing.modified_entries {
        if app.editing.deleted_entries.contains(key) {
            continue;
        }
        let name = app
            .editing
            .rows
            .iter()
            .find(|row| lower(&row.entry.name) == *key)
            .map(|row| row.entry.name.clone())
            .unwrap_or_else(|| key.clone());
        overrides.insert(key.clone(), (name, bytes.clone()));
    }
    overrides
}

fn definition_uses_dff(definition: &Definition, dff_name: &str) -> bool {
    let definition_dff = definition
        .attrs
        .get("dff")
        .or_else(|| definition.attrs.get("model"))
        .map(String::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(&definition.id);
    asset_key(definition_dff, ".dff") == asset_key(dff_name, ".dff")
}

pub(crate) fn ensure_traffic_native_model_for_dff(app: &mut AppState, dff_name: &str) -> String {
    let matching_ids = app
        .definitions
        .values()
        .filter(|definition| definition_uses_dff(definition, dff_name))
        .map(|definition| definition.id.clone())
        .collect::<Vec<_>>();
    let [definition_id] = matching_ids.as_slice() else {
        return if matching_ids.is_empty() {
            "no matching definition was found; set Native Behavior Model to 1352 manually"
                .to_string()
        } else {
            "multiple definitions share this DFF; set one Native Behavior Model to 1352 manually"
                .to_string()
        };
    };
    if app.readonly_definition_ids.contains(definition_id) {
        return format!(
            "definition {definition_id} is read-only; create an override with Native Behavior Model 1352"
        );
    }
    let Some(definition) = app.definitions.get_mut(definition_id) else {
        return "set Native Behavior Model to 1352 manually".to_string();
    };
    if let Some(existing) = definition
        .attrs
        .get("nativeModel")
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return format!("Native Behavior Model {existing} is already assigned");
    }
    definition
        .attrs
        .insert("nativeModel".to_string(), "1352".to_string());
    mark_definition_override_attr(definition, "nativeModel");
    "assigned Native Behavior Model 1352; save and reload the map".to_string()
}

fn collision_name_from_dff(dff_name: &str) -> String {
    let trimmed = dff_name.trim();
    let stem = if lower(trimmed).ends_with(".dff") {
        &trimmed[..trimmed.len() - ".dff".len()]
    } else {
        trimmed
    };
    with_ext(stem, ".col")
}

fn collision_name_from_reference(col_name: &str) -> String {
    let trimmed = col_name.trim();
    let lowered = lower(trimmed);
    if lowered.ends_with(".dff.col") {
        with_ext(&trimmed[..trimmed.len() - ".dff.col".len()], ".col")
    } else if lowered.ends_with(".dff") {
        collision_name_from_dff(trimmed)
    } else {
        with_ext(trimmed, ".col")
    }
}

fn collision_definition_ids_for_dff(app: &AppState, dff_name: &str) -> Vec<String> {
    let mut ids = app
        .definitions
        .values()
        .filter(|definition| definition_uses_dff(definition, dff_name))
        .map(|definition| definition.id.clone())
        .collect::<Vec<_>>();
    ids.sort_by_key(|id| lower(id));
    ids.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    ids
}

fn generated_col_matches_definition_dff(definition: &Definition, col_name: &str) -> bool {
    let dff_name = definition
        .attrs
        .get("dff")
        .or_else(|| definition.attrs.get("model"))
        .map(String::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(&definition.id);
    lower(&normalize_legacy_light_mapper_asset_stem(dff_name))
        == lower(&normalize_legacy_light_mapper_asset_stem(col_name))
}

fn clear_redundant_definition_col(definition: &mut Definition) -> bool {
    let removed_value = definition.attrs.remove("col").is_some();
    if !is_override_definition(definition) {
        return removed_value;
    }
    let mut explicit_attrs = definition
        .attrs
        .get("__overrideAttrs")
        .map(|attrs| {
            attrs
                .split(',')
                .map(str::trim)
                .filter(|key| !key.is_empty())
                .map(ToOwned::to_owned)
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    let removed_marker = explicit_attrs.remove("col");
    if removed_marker {
        definition.attrs.insert(
            "__overrideAttrs".to_string(),
            explicit_attrs.into_iter().collect::<Vec<_>>().join(","),
        );
    }
    removed_value || removed_marker
}

fn assign_generated_col_to_definition(definition: &mut Definition, col_value: &str) -> bool {
    let value_changed = definition.attrs.get("col").map(String::as_str) != Some(col_value);
    let needs_override_marker = is_override_definition(definition)
        && !definition
            .attrs
            .get("__overrideAttrs")
            .is_some_and(|attrs| attrs.split(',').any(|key| key.trim() == "col"));
    if !value_changed && !needs_override_marker {
        return false;
    }
    if value_changed {
        definition
            .attrs
            .insert("col".to_string(), col_value.to_string());
    }
    mark_definition_override_attr(definition, "col");
    true
}

fn assign_generated_col_to_definitions(
    app: &mut AppState,
    definition_ids: &[String],
    col_name: &str,
) -> usize {
    let col_value = normalize_legacy_light_mapper_asset_stem(col_name);
    let mut updated = 0usize;
    for id in definition_ids {
        let Some(names_match) = app
            .definitions
            .get(id)
            .map(|definition| generated_col_matches_definition_dff(definition, &col_value))
        else {
            continue;
        };
        if names_match {
            if app.readonly_definition_ids.contains(id) {
                continue;
            }
            let Some(definition) = app.definitions.get_mut(id) else {
                continue;
            };
            updated += usize::from(clear_redundant_definition_col(definition));
            continue;
        }
        if app.readonly_definition_ids.contains(id) {
            let Some(zone) = app
                .definitions
                .get(id)
                .map(|definition| definition.zone.clone())
            else {
                continue;
            };
            make_definition_override_writable(app, id, zone);
        }
        let Some(definition) = app.definitions.get_mut(id) else {
            continue;
        };
        updated += usize::from(assign_generated_col_to_definition(definition, &col_value));
    }
    updated
}

fn collision_targets_for_dff(app: &AppState, dff_name: &str) -> BTreeSet<String> {
    collision_definition_ids_for_dff(app, dff_name)
        .into_iter()
        .filter_map(|id| {
            app.definitions.get(&id).map(|definition| {
                definition
                    .attrs
                    .get("col")
                    .map(String::as_str)
                    .filter(|value| !value.trim().is_empty())
                    .map(|value| asset_key(&collision_name_from_reference(value), ".col"))
                    .unwrap_or_else(|| asset_key(&collision_name_from_dff(dff_name), ".col"))
            })
        })
        .collect()
}

fn collision_generation_placement_dff_name(app: &AppState, placement: &Placement) -> String {
    app.definitions
        .get(&placement.id)
        .and_then(|definition| {
            definition
                .attrs
                .get("dff")
                .or_else(|| definition.attrs.get("model"))
        })
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| placement.dff.clone())
}

/// A shared DFF remains full collision if any live detail placement uses it.
/// This avoids degrading a detail object when a resource intentionally reuses
/// one visual for both roles.
fn collision_generation_dff_is_lod_only_in_scene(
    dff_name: &str,
    placements: &[Placement],
    definitions: &HashMap<String, Definition>,
    lod_ids: &HashSet<String>,
    element_states: &[ElementState],
) -> bool {
    let target = asset_key(dff_name, ".dff");
    let mut matched = false;
    let mut all_lod = true;
    for (index, placement) in placements.iter().enumerate() {
        if element_states.get(index).is_some_and(|state| state.deleted) {
            continue;
        }
        let placement_dff = definitions
            .get(&placement.id)
            .and_then(|definition| {
                definition
                    .attrs
                    .get("dff")
                    .or_else(|| definition.attrs.get("model"))
            })
            .filter(|value| !value.trim().is_empty())
            .map(String::as_str)
            .unwrap_or(&placement.dff);
        if asset_key(placement_dff, ".dff") != target {
            continue;
        }
        matched = true;
        all_lod &= placement_is_lod(placement, lod_ids);
    }
    if matched {
        all_lod
    } else {
        is_lod_name(dff_name, dff_name)
    }
}

fn collision_generation_dff_is_lod_only(app: &AppState, dff_name: &str) -> bool {
    collision_generation_dff_is_lod_only_in_scene(
        dff_name,
        &app.placements,
        &app.definitions,
        &app.lod_ids,
        &app.element_states,
    )
}

pub(crate) fn request_editing_collision_generation(
    app: &mut AppState,
    preset: CollisionGenerationPreset,
    fallback_material: u8,
) -> bool {
    if collision_generation_writer_active(app) {
        app.status_message =
            "Collision generation cannot start while another asset writer is running.".to_string();
        return false;
    }
    let source = match app.editing.asset.as_ref() {
        Some(EditingAsset::Dff(dff)) => {
            let targets = collision_targets_for_dff(app, &dff.name);
            let definition_ids_to_assign = collision_definition_ids_for_dff(app, &dff.name);
            if targets.len() > 1 {
                app.status_message = format!(
                    "{} maps to multiple COL assets. Open the intended COL and generate from its DFF overlay.",
                    dff.name
                );
                return false;
            }
            CollisionGenerationSource {
                dff_name: dff.name.clone(),
                col_name: targets
                    .into_iter()
                    .next()
                    .unwrap_or_else(|| collision_name_from_dff(&dff.name)),
                txd_name: dff.txd_context.clone(),
                raw_override: Some(dff.raw.clone()),
                lod_only: collision_generation_dff_is_lod_only(app, &dff.name),
                definition_ids_to_assign,
            }
        }
        Some(EditingAsset::Col(col)) => {
            let dff_name = col
                .dff_overlay_name
                .clone()
                .or_else(|| resolve_editing_col_dff_overlay_name(app, &col.name));
            let Some(dff_name) = dff_name else {
                app.status_message = "No matching DFF found. Pick a DFF overlay first.".to_string();
                return false;
            };
            let (txd_name, _) = resolve_editing_dff_txd_context(app, &dff_name);
            let lod_only = collision_generation_dff_is_lod_only(app, &dff_name);
            CollisionGenerationSource {
                dff_name,
                col_name: col.name.clone(),
                txd_name,
                raw_override: None,
                lod_only,
                definition_ids_to_assign: Vec::new(),
            }
        }
        _ => {
            app.status_message = "Open a DFF or COL before generating collision.".to_string();
            return false;
        }
    };
    if app.safe_collisions.is_safe(&source.col_name) {
        app.status_message = format!(
            "{} is marked Safe and was not regenerated",
            asset_key(&source.col_name, ".col")
        );
        return false;
    }
    let context = CollisionGenerationContext {
        root: app.root.clone(),
        sources: vec![source],
        byte_overrides: collision_generation_byte_overrides(app),
        material_classes: app.material_classes.clone(),
        txd_textures: app.txd_textures.clone(),
        preset,
        fallback_material,
    };
    app.collision_generation_job = Some(CollisionGenerationJob::new(
        context,
        CollisionGenerationScope::Editing,
        0,
    ));
    app.status_message = "Collision generation started in background...".to_string();
    true
}

pub(crate) fn request_global_collision_generation(
    app: &mut AppState,
    preset: CollisionGenerationPreset,
    fallback_material: u8,
) -> bool {
    if collision_generation_writer_active(app) {
        app.status_message =
            "Collision generation cannot start while another asset writer is running.".to_string();
        return false;
    }
    // A COL is the actual output unit. Keying by target means a DFF that is
    // intentionally referenced through multiple COL names gets every target,
    // while a shared COL is generated only once.
    let mut sources = BTreeMap::<String, CollisionGenerationSource>::new();
    for (index, placement) in app.placements.iter().enumerate() {
        if !is_live_element(app, index) {
            continue;
        }
        let lod_only = placement_is_app_lod(app, placement);
        let definition = app.definitions.get(&placement.id);
        let dff_name = collision_generation_placement_dff_name(app, placement);
        let dff_key = asset_key(&dff_name, ".dff");
        let col_name = definition
            .and_then(|def| def.attrs.get("col"))
            .filter(|value| !value.trim().is_empty())
            .map(|value| collision_name_from_reference(value))
            .unwrap_or_else(|| collision_name_from_dff(&dff_name));
        let txd_name = definition
            .and_then(definition_txd_name_from_attrs)
            .map(|value| asset_key(value, ".txd"));
        let col_key = asset_key(&col_name, ".col");
        match sources.entry(col_key) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(CollisionGenerationSource {
                    dff_name,
                    col_name,
                    txd_name,
                    raw_override: None,
                    lod_only,
                    definition_ids_to_assign: Vec::new(),
                });
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing_dff_key = asset_key(&entry.get().dff_name, ".dff");
                let existing_lod_only = entry.get().lod_only;
                // A shared output referenced through multiple TXDs or DFFs
                // cannot safely use an exact-TXD material override.
                if existing_dff_key != dff_key || entry.get().txd_name != txd_name {
                    entry.get_mut().txd_name = None;
                }
                // If different visuals share one COL target, choose the
                // lexicographically first DFF deterministically rather than
                // generating the same output twice and relying on last-write.
                let prefer_current = existing_lod_only && !lod_only
                    || existing_lod_only == lod_only && dff_key < existing_dff_key;
                if prefer_current {
                    let source = entry.get_mut();
                    source.dff_name = dff_name;
                    source.col_name = col_name;
                    source.raw_override = None;
                }
                // A COL target used by any detail placement must remain full
                // collision even if it is also referenced by an LOD.
                entry.get_mut().lod_only = existing_lod_only && lod_only;
            }
        }
    }
    let safe_skipped = remove_safe_collision_sources(&mut sources, &app.safe_collisions);
    if sources.is_empty() {
        app.status_message = if safe_skipped > 0 {
            format!(
                "No collision targets regenerated; all {safe_skipped} referenced target(s) are marked Safe."
            )
        } else {
            "No referenced DFFs are eligible for collision regeneration.".to_string()
        };
        return false;
    }
    if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_ref()
        && dff.dirty
    {
        let dff_key = asset_key(&dff.name, ".dff");
        for source in sources
            .values_mut()
            .filter(|source| asset_key(&source.dff_name, ".dff") == dff_key)
        {
            source.raw_override = Some(dff.raw.clone());
        }
    }
    let source_count = sources.len();
    let context = CollisionGenerationContext {
        root: app.root.clone(),
        sources: sources.into_values().collect(),
        byte_overrides: collision_generation_byte_overrides(app),
        material_classes: app.material_classes.clone(),
        txd_textures: app.txd_textures.clone(),
        preset,
        fallback_material,
    };
    app.collision_generation_job = Some(CollisionGenerationJob::new(
        context,
        CollisionGenerationScope::Global,
        safe_skipped,
    ));
    app.status_message = format!(
        "Global collision regeneration started for {source_count} COL target(s); skipped {safe_skipped} Safe target(s); old referenced collision geometry will be replaced..."
    );
    true
}

fn remove_safe_collision_sources(
    sources: &mut BTreeMap<String, CollisionGenerationSource>,
    safe_collisions: &SafeCollisions,
) -> usize {
    let before = sources.len();
    sources.retain(|col_key, _| !safe_collisions.is_safe(col_key));
    before - sources.len()
}

pub(crate) fn update_collision_generation_job(app: &mut AppState) {
    let Some(mut job) = app.collision_generation_job.take() else {
        return;
    };
    if !job.step(app) {
        app.collision_generation_job = Some(job);
    }
}

const DFF_2DFX_TYPES: [(u32, &str); 9] = [
    (0, "Light"),
    (1, "Particle / Smoke"),
    (3, "Ped Attractor"),
    (4, "Sun Glare"),
    (6, "Enter/Exit"),
    (7, "Road Sign"),
    (8, "Trigger"),
    (9, "Cover"),
    (10, "Escalator"),
];

pub(crate) fn dff_2dfx_label(effect_id: u32) -> &'static str {
    match effect_id {
        0 => "Light",
        1 => "Particle",
        3 => "Ped Attractor",
        4 => "Sun Glare",
        6 => "Enter/Exit",
        7 => "Road Sign",
        8 => "Trigger",
        9 => "Cover",
        10 => "Escalator",
        _ => "Unknown",
    }
}

fn dff_2dfx_default_payload(effect_id: u32) -> Vec<u8> {
    match effect_id {
        0 => dff_2dfx_default_light_payload(),
        1 => {
            let mut payload = [0u8; 24];
            payload[..5].copy_from_slice(b"smoke");
            payload.to_vec()
        }
        3 => {
            let mut payload = Vec::new();
            payload.extend_from_slice(&0u32.to_le_bytes());
            for _ in 0..9 {
                payload.extend_from_slice(&0.0f32.to_le_bytes());
            }
            let mut script = [0u8; 8];
            script[..4].copy_from_slice(b"none");
            payload.extend_from_slice(&script);
            payload.extend_from_slice(&0u32.to_le_bytes());
            payload.extend_from_slice(&0u32.to_le_bytes());
            payload
        }
        4 => Vec::new(),
        6 => {
            let mut payload = Vec::new();
            for value in [0.0f32, 2.0, 2.0, 0.0, 0.0, 0.0, 0.0] {
                payload.extend_from_slice(&value.to_le_bytes());
            }
            payload.extend_from_slice(&0u16.to_le_bytes());
            payload.extend_from_slice(&[0, 0]);
            payload.extend_from_slice(&[0; 8]);
            payload.extend_from_slice(&[0, 24, 0, 0]);
            payload
        }
        7 => {
            let mut payload = Vec::new();
            for value in [1.0f32, 1.0, 0.0, 0.0, 0.0] {
                payload.extend_from_slice(&value.to_le_bytes());
            }
            payload.extend_from_slice(&0u16.to_le_bytes());
            payload.extend_from_slice(&[0; 66]);
            payload
        }
        8 => 0u32.to_le_bytes().to_vec(),
        9 => {
            let mut payload = Vec::new();
            payload.extend_from_slice(&0.0f32.to_le_bytes());
            payload.extend_from_slice(&0.0f32.to_le_bytes());
            payload.extend_from_slice(&0u32.to_le_bytes());
            payload
        }
        10 => {
            let mut payload = Vec::new();
            for _ in 0..9 {
                payload.extend_from_slice(&0.0f32.to_le_bytes());
            }
            payload.extend_from_slice(&0u32.to_le_bytes());
            payload
        }
        _ => Vec::new(),
    }
}

pub(crate) fn dff_2dfx_filtered_types(query: &str) -> Vec<(u32, &'static str)> {
    let query = lower(query.trim());
    DFF_2DFX_TYPES
        .into_iter()
        .filter(|(id, label)| {
            query.is_empty()
                || lower(label).contains(&query)
                || id.to_string().contains(query.as_str())
        })
        .collect()
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_hex_bytes(value: &str) -> Result<Vec<u8>, String> {
    let compact = value
        .chars()
        .filter(|ch| !ch.is_whitespace() && *ch != '_' && *ch != ',')
        .collect::<String>();
    if compact.len() % 2 != 0 {
        return Err("Hex payload needs an even number of digits".to_string());
    }
    let mut out = Vec::with_capacity(compact.len() / 2);
    for idx in (0..compact.len()).step_by(2) {
        let byte = u8::from_str_radix(&compact[idx..idx + 2], 16)
            .map_err(|_| "Hex payload contains a non-hex digit".to_string())?;
        out.push(byte);
    }
    Ok(out)
}

#[derive(Clone, Copy)]
enum Dff2dfxPropKind {
    PosX,
    PosY,
    PosZ,
    F32(usize),
    U8(usize),
    U16(usize),
    U32(usize),
    I16(usize),
    I32(usize),
    Str(usize, usize),
    RawHex,
}

#[derive(Clone, Copy)]
struct Dff2dfxPropSpec {
    label: &'static str,
    kind: Dff2dfxPropKind,
}

fn dff_2dfx_prop_specs(effect_id: u32) -> Vec<Dff2dfxPropSpec> {
    let mut specs = vec![
        Dff2dfxPropSpec {
            label: "Position X",
            kind: Dff2dfxPropKind::PosX,
        },
        Dff2dfxPropSpec {
            label: "Position Y",
            kind: Dff2dfxPropKind::PosY,
        },
        Dff2dfxPropSpec {
            label: "Position Z",
            kind: Dff2dfxPropKind::PosZ,
        },
    ];
    match effect_id {
        0 => specs.extend([
            Dff2dfxPropSpec {
                label: "Color R",
                kind: Dff2dfxPropKind::U8(0),
            },
            Dff2dfxPropSpec {
                label: "Color G",
                kind: Dff2dfxPropKind::U8(1),
            },
            Dff2dfxPropSpec {
                label: "Color B",
                kind: Dff2dfxPropKind::U8(2),
            },
            Dff2dfxPropSpec {
                label: "Color A",
                kind: Dff2dfxPropKind::U8(3),
            },
            Dff2dfxPropSpec {
                label: "Corona Far Clip",
                kind: Dff2dfxPropKind::F32(4),
            },
            Dff2dfxPropSpec {
                label: "Pointlight Range",
                kind: Dff2dfxPropKind::F32(8),
            },
            Dff2dfxPropSpec {
                label: "Corona Size",
                kind: Dff2dfxPropKind::F32(12),
            },
            Dff2dfxPropSpec {
                label: "Shadow Size",
                kind: Dff2dfxPropKind::F32(16),
            },
            Dff2dfxPropSpec {
                label: "Corona Show Mode",
                kind: Dff2dfxPropKind::U8(20),
            },
            Dff2dfxPropSpec {
                label: "Reflection",
                kind: Dff2dfxPropKind::U8(21),
            },
            Dff2dfxPropSpec {
                label: "Flare Type",
                kind: Dff2dfxPropKind::U8(22),
            },
            Dff2dfxPropSpec {
                label: "Shadow Mult",
                kind: Dff2dfxPropKind::U8(23),
            },
            Dff2dfxPropSpec {
                label: "Flags 1",
                kind: Dff2dfxPropKind::U8(24),
            },
            Dff2dfxPropSpec {
                label: "Corona Texture",
                kind: Dff2dfxPropKind::Str(25, 24),
            },
            Dff2dfxPropSpec {
                label: "Shadow Texture",
                kind: Dff2dfxPropKind::Str(49, 24),
            },
            Dff2dfxPropSpec {
                label: "Shadow Z Dist",
                kind: Dff2dfxPropKind::U8(73),
            },
            Dff2dfxPropSpec {
                label: "Flags 2",
                kind: Dff2dfxPropKind::U8(74),
            },
            Dff2dfxPropSpec {
                label: "Look Dir X",
                kind: Dff2dfxPropKind::U8(75),
            },
            Dff2dfxPropSpec {
                label: "Look Dir Y",
                kind: Dff2dfxPropKind::U8(76),
            },
            Dff2dfxPropSpec {
                label: "Look Dir Z",
                kind: Dff2dfxPropKind::U8(77),
            },
        ]),
        1 => specs.push(Dff2dfxPropSpec {
            label: "Particle Name",
            kind: Dff2dfxPropKind::Str(0, 24),
        }),
        3 => specs.extend([
            Dff2dfxPropSpec {
                label: "Attractor Type",
                kind: Dff2dfxPropKind::I32(0),
            },
            Dff2dfxPropSpec {
                label: "Queue Dir X",
                kind: Dff2dfxPropKind::F32(4),
            },
            Dff2dfxPropSpec {
                label: "Queue Dir Y",
                kind: Dff2dfxPropKind::F32(8),
            },
            Dff2dfxPropSpec {
                label: "Queue Dir Z",
                kind: Dff2dfxPropKind::F32(12),
            },
            Dff2dfxPropSpec {
                label: "Use Dir X",
                kind: Dff2dfxPropKind::F32(16),
            },
            Dff2dfxPropSpec {
                label: "Use Dir Y",
                kind: Dff2dfxPropKind::F32(20),
            },
            Dff2dfxPropSpec {
                label: "Use Dir Z",
                kind: Dff2dfxPropKind::F32(24),
            },
            Dff2dfxPropSpec {
                label: "Forward Dir X",
                kind: Dff2dfxPropKind::F32(28),
            },
            Dff2dfxPropSpec {
                label: "Forward Dir Y",
                kind: Dff2dfxPropKind::F32(32),
            },
            Dff2dfxPropSpec {
                label: "Forward Dir Z",
                kind: Dff2dfxPropKind::F32(36),
            },
            Dff2dfxPropSpec {
                label: "Script Name",
                kind: Dff2dfxPropKind::Str(40, 8),
            },
            Dff2dfxPropSpec {
                label: "Ped Probability",
                kind: Dff2dfxPropKind::I32(48),
            },
            Dff2dfxPropSpec {
                label: "Unknown 1",
                kind: Dff2dfxPropKind::U8(52),
            },
            Dff2dfxPropSpec {
                label: "Not Used 1",
                kind: Dff2dfxPropKind::U8(53),
            },
            Dff2dfxPropSpec {
                label: "Unknown 2",
                kind: Dff2dfxPropKind::U8(54),
            },
            Dff2dfxPropSpec {
                label: "Not Used 2",
                kind: Dff2dfxPropKind::U8(55),
            },
        ]),
        4 => {}
        6 => specs.extend([
            Dff2dfxPropSpec {
                label: "Enter Rotation",
                kind: Dff2dfxPropKind::F32(0),
            },
            Dff2dfxPropSpec {
                label: "Radius X",
                kind: Dff2dfxPropKind::F32(4),
            },
            Dff2dfxPropSpec {
                label: "Radius Y",
                kind: Dff2dfxPropKind::F32(8),
            },
            Dff2dfxPropSpec {
                label: "Exit Offset X",
                kind: Dff2dfxPropKind::F32(12),
            },
            Dff2dfxPropSpec {
                label: "Exit Offset Y",
                kind: Dff2dfxPropKind::F32(16),
            },
            Dff2dfxPropSpec {
                label: "Exit Offset Z",
                kind: Dff2dfxPropKind::F32(20),
            },
            Dff2dfxPropSpec {
                label: "Exit Rotation",
                kind: Dff2dfxPropKind::F32(24),
            },
            Dff2dfxPropSpec {
                label: "Interior",
                kind: Dff2dfxPropKind::I16(28),
            },
            Dff2dfxPropSpec {
                label: "Flags",
                kind: Dff2dfxPropKind::I16(30),
            },
            Dff2dfxPropSpec {
                label: "Interior Name",
                kind: Dff2dfxPropKind::Str(32, 8),
            },
            Dff2dfxPropSpec {
                label: "Time On",
                kind: Dff2dfxPropKind::U8(40),
            },
            Dff2dfxPropSpec {
                label: "Time Off",
                kind: Dff2dfxPropKind::U8(41),
            },
            Dff2dfxPropSpec {
                label: "Sky Color",
                kind: Dff2dfxPropKind::U8(42),
            },
            Dff2dfxPropSpec {
                label: "Unknown",
                kind: Dff2dfxPropKind::U8(43),
            },
        ]),
        7 => {
            specs.extend([
                Dff2dfxPropSpec {
                    label: "Size X",
                    kind: Dff2dfxPropKind::F32(0),
                },
                Dff2dfxPropSpec {
                    label: "Size Y",
                    kind: Dff2dfxPropKind::F32(4),
                },
                Dff2dfxPropSpec {
                    label: "Rotation X",
                    kind: Dff2dfxPropKind::F32(8),
                },
                Dff2dfxPropSpec {
                    label: "Rotation Y",
                    kind: Dff2dfxPropKind::F32(12),
                },
                Dff2dfxPropSpec {
                    label: "Rotation Z",
                    kind: Dff2dfxPropKind::F32(16),
                },
                Dff2dfxPropSpec {
                    label: "Flags",
                    kind: Dff2dfxPropKind::U16(20),
                },
            ]);
            for (idx, offset) in [22, 38, 54, 70].into_iter().enumerate() {
                specs.push(Dff2dfxPropSpec {
                    label: match idx {
                        0 => "Text Line 1",
                        1 => "Text Line 2",
                        2 => "Text Line 3",
                        _ => "Text Line 4",
                    },
                    kind: Dff2dfxPropKind::Str(offset, 16),
                });
            }
        }
        8 => specs.push(Dff2dfxPropSpec {
            label: "Point ID",
            kind: Dff2dfxPropKind::I32(0),
        }),
        9 => specs.extend([
            Dff2dfxPropSpec {
                label: "Direction X",
                kind: Dff2dfxPropKind::F32(0),
            },
            Dff2dfxPropSpec {
                label: "Direction Y",
                kind: Dff2dfxPropKind::F32(4),
            },
            Dff2dfxPropSpec {
                label: "Cover Type",
                kind: Dff2dfxPropKind::U32(8),
            },
        ]),
        10 => specs.extend([
            Dff2dfxPropSpec {
                label: "Bottom X",
                kind: Dff2dfxPropKind::F32(0),
            },
            Dff2dfxPropSpec {
                label: "Bottom Y",
                kind: Dff2dfxPropKind::F32(4),
            },
            Dff2dfxPropSpec {
                label: "Bottom Z",
                kind: Dff2dfxPropKind::F32(8),
            },
            Dff2dfxPropSpec {
                label: "Top X",
                kind: Dff2dfxPropKind::F32(12),
            },
            Dff2dfxPropSpec {
                label: "Top Y",
                kind: Dff2dfxPropKind::F32(16),
            },
            Dff2dfxPropSpec {
                label: "Top Z",
                kind: Dff2dfxPropKind::F32(20),
            },
            Dff2dfxPropSpec {
                label: "End X",
                kind: Dff2dfxPropKind::F32(24),
            },
            Dff2dfxPropSpec {
                label: "End Y",
                kind: Dff2dfxPropKind::F32(28),
            },
            Dff2dfxPropSpec {
                label: "End Z",
                kind: Dff2dfxPropKind::F32(32),
            },
            Dff2dfxPropSpec {
                label: "Direction",
                kind: Dff2dfxPropKind::U32(36),
            },
        ]),
        _ => specs.push(Dff2dfxPropSpec {
            label: "Payload Hex",
            kind: Dff2dfxPropKind::RawHex,
        }),
    }
    specs
}

fn read_payload<const N: usize>(payload: &[u8], offset: usize) -> [u8; N] {
    let mut bytes = [0u8; N];
    if let Some(slice) = payload.get(offset..offset + N) {
        bytes.copy_from_slice(slice);
    }
    bytes
}

fn fixed_string(payload: &[u8], offset: usize, len: usize) -> String {
    payload
        .get(offset..offset + len)
        .unwrap_or(&[])
        .iter()
        .copied()
        .take_while(|byte| *byte != 0)
        .map(char::from)
        .collect()
}

fn dff_2dfx_prop_value(effect: &Dff2dEffect, kind: Dff2dfxPropKind) -> String {
    match kind {
        Dff2dfxPropKind::PosX => format!("{:.4}", effect.position.x),
        Dff2dfxPropKind::PosY => format!("{:.4}", effect.position.y),
        Dff2dfxPropKind::PosZ => format!("{:.4}", effect.position.z),
        Dff2dfxPropKind::F32(offset) => {
            format!(
                "{:.4}",
                f32::from_le_bytes(read_payload(&effect.payload, offset))
            )
        }
        Dff2dfxPropKind::U8(offset) => effect.payload.get(offset).copied().unwrap_or(0).to_string(),
        Dff2dfxPropKind::U16(offset) => {
            u16::from_le_bytes(read_payload(&effect.payload, offset)).to_string()
        }
        Dff2dfxPropKind::U32(offset) => {
            u32::from_le_bytes(read_payload(&effect.payload, offset)).to_string()
        }
        Dff2dfxPropKind::I16(offset) => {
            i16::from_le_bytes(read_payload(&effect.payload, offset)).to_string()
        }
        Dff2dfxPropKind::I32(offset) => {
            i32::from_le_bytes(read_payload(&effect.payload, offset)).to_string()
        }
        Dff2dfxPropKind::Str(offset, len) => fixed_string(&effect.payload, offset, len),
        Dff2dfxPropKind::RawHex => bytes_to_hex(&effect.payload),
    }
}

pub(crate) fn dff_2dfx_active_payload_is_particle_name(dff: &EditingDffState) -> bool {
    let Some(idx) = dff.selected_2dfx else {
        return false;
    };
    let Some(effect) = dff.raw.effects_2dfx.get(idx) else {
        return false;
    };
    if effect.effect_id != 1 {
        return false;
    }
    let Some(active) = dff.dff_2dfx_payload_active_field else {
        return false;
    };
    matches!(
        dff_2dfx_prop_specs(effect.effect_id)
            .get(active)
            .map(|spec| spec.kind),
        Some(Dff2dfxPropKind::Str(0, 24))
    )
}

pub(crate) fn dff_2dfx_particle_name_options(app: &AppState, dff: &EditingDffState) -> Vec<String> {
    let query = dff
        .dff_2dfx_payload_active_field
        .and_then(|idx| dff.dff_2dfx_payload_fields.get(idx))
        .map(|value| lower(value.trim()))
        .unwrap_or_default();
    app.particle_effects
        .iter()
        .filter(|effect| {
            query.is_empty()
                || lower(&effect.name).contains(&query)
                || effect
                    .textures
                    .iter()
                    .any(|texture| lower(texture).contains(&query))
        })
        .map(|effect| effect.name.clone())
        .collect()
}

fn selected_dff_2dfx_effect_id(dff: &EditingDffState) -> Option<u32> {
    dff.selected_2dfx
        .and_then(|idx| dff.raw.effects_2dfx.get(idx))
        .map(|effect| effect.effect_id)
}

fn dff_2dfx_payload_field_index(dff: &EditingDffState, label: &str) -> Option<usize> {
    let effect_id = selected_dff_2dfx_effect_id(dff)?;
    dff_2dfx_prop_specs(effect_id)
        .iter()
        .position(|spec| spec.label == label)
}

fn dff_2dfx_light_color_value(dff: &EditingDffState, channel: usize) -> u8 {
    let label = match channel {
        0 => "Color R",
        1 => "Color G",
        2 => "Color B",
        _ => return 0,
    };
    dff_2dfx_payload_field_index(dff, label)
        .and_then(|idx| dff.dff_2dfx_payload_fields.get(idx))
        .and_then(|value| value.trim().parse::<u8>().ok())
        .unwrap_or(0)
}

fn set_dff_2dfx_light_color_value(dff: &mut EditingDffState, channel: usize, value: u8) {
    let label = match channel {
        0 => "Color R",
        1 => "Color G",
        2 => "Color B",
        _ => return,
    };
    if let Some(idx) = dff_2dfx_payload_field_index(dff, label) {
        if let Some(field) = dff.dff_2dfx_payload_fields.get_mut(idx) {
            *field = value.to_string();
            dff.dff_2dfx_payload_active_field = Some(idx);
        }
    }
}

fn dff_2dfx_payload_len(effect_id: u32, current_len: usize) -> usize {
    match effect_id {
        0 => current_len.max(80),
        1 => 24,
        3 => 56,
        4 => 0,
        6 => 44,
        7 => 88,
        8 => 4,
        9 => 12,
        10 => 40,
        _ => current_len,
    }
}

fn parse_intish<T>(value: &str, label: &str) -> Result<T, String>
where
    T: TryFrom<i64>,
{
    let trimmed = value.trim();
    let parsed = if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        i64::from_str_radix(hex, 16)
    } else {
        trimmed.parse::<i64>()
    }
    .map_err(|_| format!("{label} must be a whole number"))?;
    T::try_from(parsed).map_err(|_| format!("{label} is out of range"))
}

fn write_fixed_string(payload: &mut [u8], offset: usize, len: usize, value: &str) {
    if let Some(slice) = payload.get_mut(offset..offset + len) {
        slice.fill(0);
        for (dst, src) in slice.iter_mut().zip(value.as_bytes().iter().copied()) {
            *dst = src;
        }
    }
}

fn apply_dff_2dfx_field_value(
    effect_id: u32,
    payload: &mut Vec<u8>,
    position: &mut V3,
    spec: Dff2dfxPropSpec,
    value: &str,
) -> Result<(), String> {
    match spec.kind {
        Dff2dfxPropKind::PosX => {
            position.x = value
                .trim()
                .parse()
                .map_err(|_| "Position X must be a number".to_string())?;
        }
        Dff2dfxPropKind::PosY => {
            position.y = value
                .trim()
                .parse()
                .map_err(|_| "Position Y must be a number".to_string())?;
        }
        Dff2dfxPropKind::PosZ => {
            position.z = value
                .trim()
                .parse()
                .map_err(|_| "Position Z must be a number".to_string())?;
        }
        Dff2dfxPropKind::F32(offset) => {
            let parsed: f32 = value
                .trim()
                .parse()
                .map_err(|_| format!("{} must be a number", spec.label))?;
            if let Some(slice) = payload.get_mut(offset..offset + 4) {
                slice.copy_from_slice(&parsed.to_le_bytes());
            }
        }
        Dff2dfxPropKind::U8(offset) => {
            let parsed: u8 = parse_intish(value, spec.label)?;
            if let Some(byte) = payload.get_mut(offset) {
                *byte = parsed;
            }
        }
        Dff2dfxPropKind::U16(offset) => {
            let parsed: u16 = parse_intish(value, spec.label)?;
            if let Some(slice) = payload.get_mut(offset..offset + 2) {
                slice.copy_from_slice(&parsed.to_le_bytes());
            }
        }
        Dff2dfxPropKind::U32(offset) => {
            let parsed: u32 = parse_intish(value, spec.label)?;
            if let Some(slice) = payload.get_mut(offset..offset + 4) {
                slice.copy_from_slice(&parsed.to_le_bytes());
            }
        }
        Dff2dfxPropKind::I16(offset) => {
            let parsed: i16 = parse_intish(value, spec.label)?;
            if let Some(slice) = payload.get_mut(offset..offset + 2) {
                slice.copy_from_slice(&parsed.to_le_bytes());
            }
        }
        Dff2dfxPropKind::I32(offset) => {
            let parsed: i32 = parse_intish(value, spec.label)?;
            if let Some(slice) = payload.get_mut(offset..offset + 4) {
                slice.copy_from_slice(&parsed.to_le_bytes());
            }
        }
        Dff2dfxPropKind::Str(offset, len) => write_fixed_string(payload, offset, len, value),
        Dff2dfxPropKind::RawHex => {
            *payload = parse_hex_bytes(value)?;
            if effect_id != 4 && payload.is_empty() {
                return Err("Payload cannot be empty for this unknown type".to_string());
            }
        }
    }
    Ok(())
}

pub(crate) fn set_selected_dff_2dfx_type(app: &mut AppState, effect_id: u32) -> bool {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return false;
    };
    let Some(idx) = dff.selected_2dfx else {
        app.status_message = "Select a DFF 2DFX entry first".to_string();
        return false;
    };
    let Some(effect) = dff.raw.effects_2dfx.get_mut(idx) else {
        app.status_message = "Selected DFF 2DFX entry no longer exists".to_string();
        return false;
    };
    effect.effect_id = effect_id;
    effect.payload = dff_2dfx_default_payload(effect_id);
    dff.dff_2dfx_type_picker_open = false;
    dff.dff_2dfx_corona_preset_picker_open = false;
    dff.dff_2dfx_type_picker_search.clear();
    dff.dff_2dfx_type_picker_scroll = 0.0;
    dff.dff_2dfx_payload_hex = bytes_to_hex(&effect.payload);
    dff.dirty = true;
    app.status_message = format!("Changed DFF 2DFX to {}", dff_2dfx_label(effect_id));
    true
}

fn dff_2dfx_default_position(dff: &EditingDffState) -> V3 {
    dff.selected_face
        .and_then(|face| raw_triangle_indices(&dff.raw, face))
        .map(|indices| {
            let mut center = V3::default();
            for idx in indices {
                if let Some(vertex) = dff.raw.vertices.get(idx) {
                    center.x += vertex.x;
                    center.y += vertex.y;
                    center.z += vertex.z;
                }
            }
            center.x /= 3.0;
            center.y /= 3.0;
            center.z /= 3.0;
            center
        })
        .unwrap_or_else(|| {
            let bounds = bounds_from_vertices(&dff.raw.vertices);
            V3 {
                x: (bounds.min.x + bounds.max.x) * 0.5,
                y: (bounds.min.y + bounds.max.y) * 0.5,
                z: (bounds.min.z + bounds.max.z) * 0.5,
            }
        })
}

pub(crate) fn add_dff_2dfx_of_type(app: &mut AppState, effect_id: u32) -> bool {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return false;
    };
    let position = dff_2dfx_default_position(dff);
    dff.raw.effects_2dfx.push(Dff2dEffect {
        position,
        effect_id,
        payload: dff_2dfx_default_payload(effect_id),
    });
    dff.selected_2dfx = dff.raw.effects_2dfx.len().checked_sub(1);
    dff.dff_2dfx_type_picker_open = false;
    dff.dff_2dfx_corona_preset_picker_open = false;
    dff.dff_2dfx_type_picker_search.clear();
    dff.dff_2dfx_type_picker_scroll = 0.0;
    dff.dirty = true;
    app.status_message = format!("Added DFF 2DFX {}", dff_2dfx_label(effect_id));
    true
}

pub(crate) fn open_selected_dff_2dfx_payload_editor(app: &mut AppState) -> bool {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return false;
    };
    let Some(idx) = dff.selected_2dfx else {
        app.status_message = "Select a DFF 2DFX entry first".to_string();
        return false;
    };
    let Some(effect) = dff.raw.effects_2dfx.get(idx) else {
        app.status_message = "Selected DFF 2DFX entry no longer exists".to_string();
        return false;
    };
    dff.dff_2dfx_payload_hex = bytes_to_hex(&effect.payload);
    dff.dff_2dfx_payload_fields = dff_2dfx_prop_specs(effect.effect_id)
        .into_iter()
        .map(|spec| dff_2dfx_prop_value(effect, spec.kind))
        .collect();
    dff.dff_2dfx_payload_active_field = None;
    dff.dff_2dfx_payload_field_scroll = 0.0;
    dff.dff_2dfx_payload_editor_open = true;
    dff.dff_2dfx_type_picker_open = false;
    dff.dff_2dfx_corona_preset_picker_open = false;
    true
}

pub(crate) fn apply_selected_dff_2dfx_payload_hex(app: &mut AppState) -> bool {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return false;
    };
    let Some(idx) = dff.selected_2dfx else {
        app.status_message = "Select a DFF 2DFX entry first".to_string();
        return false;
    };
    let Some(effect) = dff.raw.effects_2dfx.get(idx) else {
        app.status_message = "Selected DFF 2DFX entry no longer exists".to_string();
        return false;
    };
    let specs = dff_2dfx_prop_specs(effect.effect_id);
    let mut payload = if matches!(
        specs.last().map(|spec| spec.kind),
        Some(Dff2dfxPropKind::RawHex)
    ) {
        effect.payload.clone()
    } else {
        let mut payload = dff_2dfx_default_payload(effect.effect_id);
        payload.resize(
            dff_2dfx_payload_len(effect.effect_id, effect.payload.len()),
            0,
        );
        payload
    };
    let mut position = effect.position;
    let values = dff.dff_2dfx_payload_fields.clone();
    for (spec, value) in specs.iter().copied().zip(values.iter()) {
        if let Err(err) =
            apply_dff_2dfx_field_value(effect.effect_id, &mut payload, &mut position, spec, value)
        {
            app.status_message = err;
            return false;
        }
    }
    let Some(effect) = dff.raw.effects_2dfx.get_mut(idx) else {
        app.status_message = "Selected DFF 2DFX entry no longer exists".to_string();
        return false;
    };
    effect.position = position;
    effect.payload = payload;
    dff.dff_2dfx_payload_hex = bytes_to_hex(&effect.payload);
    dff.dff_2dfx_payload_editor_open = false;
    dff.dff_2dfx_payload_active_field = None;
    dff.dirty = true;
    app.status_message = format!("Updated DFF 2DFX properties for entry {}", idx + 1);
    true
}

pub(crate) fn editing_delete_selected_dff_2dfx(app: &mut AppState) {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return;
    };
    let Some(idx) = dff.selected_2dfx else {
        app.status_message = "Select a DFF 2DFX entry first".to_string();
        return;
    };
    if idx >= dff.raw.effects_2dfx.len() {
        dff.selected_2dfx = None;
        app.status_message = "Selected DFF 2DFX entry no longer exists".to_string();
        return;
    }
    dff.raw.effects_2dfx.remove(idx);
    dff.selected_2dfx = if dff.raw.effects_2dfx.is_empty() {
        None
    } else {
        Some(idx.min(dff.raw.effects_2dfx.len() - 1))
    };
    dff.dirty = true;
    app.status_message = "Deleted DFF 2DFX entry".to_string();
}

pub(crate) fn editing_add_dff_boolean_box(app: &mut AppState) {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return;
    };
    dff.boolean_box = Some(dff_boolean_box_for_raw(&dff.raw));
    dff.selected_face = None;
    dff.selected_vertex = None;
    dff.dirty = true;
    app.status_message = "Added non-destructive DFF boolean cutter".to_string();
}

pub(crate) fn editing_clear_dff_boolean_box(app: &mut AppState) {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return;
    };
    dff.boolean_box = None;
    dff.dirty = true;
    app.status_message = "Cleared DFF boolean cutter".to_string();
}

pub(crate) fn editing_resize_dff_boolean_box(app: &mut AppState, axis: usize, delta: f32) {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return;
    };
    let Some(cutter) = dff.boolean_box.as_mut() else {
        app.status_message = "Add a boolean cutter first".to_string();
        return;
    };
    let value = match axis {
        0 => &mut cutter.half_extents.x,
        1 => &mut cutter.half_extents.y,
        _ => &mut cutter.half_extents.z,
    };
    *value = (*value + delta).max(0.05);
    dff.dirty = true;
    app.status_message = format!(
        "Boolean cutter size {:.2}, {:.2}, {:.2}",
        cutter.half_extents.x * 2.0,
        cutter.half_extents.y * 2.0,
        cutter.half_extents.z * 2.0
    );
}

pub(crate) fn editing_flip_dff_normals(app: &mut AppState) {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return;
    };
    for normal in &mut dff.raw.normals {
        normal.x = -normal.x;
        normal.y = -normal.y;
        normal.z = -normal.z;
    }
    for tri in &mut dff.raw.triangles {
        std::mem::swap(&mut tri.b, &mut tri.c);
    }
    dff.dirty = true;
    app.status_message = "Flipped DFF normals and triangle winding".to_string();
    refresh_editing_dff_preview(app);
}

fn validate_normalized_dff_stage(raw: &RawMesh, frame: &str) -> Result<(), String> {
    if raw_mesh_is_safe_for_normalized_rewrite(raw, frame) {
        Ok(())
    } else {
        Err(
            "the model has a multi-frame/component hierarchy that the normalized writer would flatten"
                .to_string(),
        )
    }
}

fn editing_dff_needs_rewrite_confirmation(app: &AppState) -> bool {
    matches!(
        app.editing.asset.as_ref(),
        Some(EditingAsset::Dff(dff))
            if dff.normalized_warning && !dff.normalized_rewrite_confirmed
    )
}

fn editing_confirm_normalized_rewrite_and_stage(app: &mut AppState) -> bool {
    if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut()
        && dff.normalized_warning
        && !dff.normalized_rewrite_confirmed
    {
        dff.normalized_rewrite_confirmed = true;
    }
    editing_stage_dff_asset(app)
}

pub(crate) fn editing_stage_dff_asset(app: &mut AppState) -> bool {
    let traffic_dff_name = app.editing.asset.as_ref().and_then(|asset| match asset {
        EditingAsset::Dff(dff)
            if dff
                .raw
                .effects_2dfx
                .iter()
                .any(|effect| effect.effect_id == 0 && effect.payload.get(20) == Some(&7)) =>
        {
            Some(dff.name.clone())
        }
        _ => None,
    });
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return false;
    };
    let frame = Path::new(&dff.name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("model");
    if let Err(error) = validate_normalized_dff_stage(&dff.raw, frame) {
        app.status_message = format!("Could not write DFF {}: {error}", dff.name);
        return false;
    }
    let mut raw = dff.raw.clone();
    let removed_by_boolean = dff
        .boolean_box
        .map(|cutter| apply_dff_boolean_box(&mut raw, cutter))
        .unwrap_or(0);
    if removed_by_boolean > 0 {
        for breakable in raw
            .components
            .iter_mut()
            .filter_map(|component| component.breakable.as_mut())
        {
            breakable.stale = true;
        }
        compact_raw_vertices(&mut raw);
        recalc_raw_normals(&mut raw);
    }
    match write_normalized_dff(&raw, frame) {
        Ok(bytes) => {
            let refresh_name = dff.name.clone();
            let refresh_bytes = bytes.clone();
            app.editing
                .modified_entries
                .insert(editing_key(&refresh_name), bytes);
            dff.dirty = false;
            dff.normalized_warning = false;
            dff.normalized_rewrite_confirmed = true;
            if removed_by_boolean > 0 {
                dff.raw = raw;
                dff.boolean_box = None;
                dff.selected_face = None;
                dff.selected_vertex = None;
                app.status_message = format!(
                    "Staged normalized DFF {}; boolean removed {removed_by_boolean} face(s)",
                    dff.name
                );
                refresh_editing_dff_preview(app);
            } else {
                app.status_message = format!("Staged normalized DFF {}", dff.name);
            }
            refresh_live_asset_from_editing_entry(app, &refresh_name, &refresh_bytes);
            if let Some(dff_name) = traffic_dff_name.as_deref() {
                let binding = ensure_traffic_native_model_for_dff(app, dff_name);
                app.status_message.push_str("; ");
                app.status_message.push_str(&binding);
            }
            true
        }
        Err(err) => {
            app.status_message = format!("Could not write DFF: {err}");
            false
        }
    }
}

pub(crate) fn editing_stage_col_asset(app: &mut AppState) {
    let Some(EditingAsset::Col(col)) = app.editing.asset.as_ref() else {
        return;
    };
    let editing_shadow = col.editing_shadow;
    let mut mesh = normalized_editing_col_mesh(col);
    let mut capsules = valid_capsules_for_mesh(&mut mesh, col.capsules.clone());
    if capsules.len() != col.capsules.len() {
        app.status_message =
            "Could not stage COL: capsule metadata no longer matches generated geometry"
                .to_string();
        return;
    }
    let mut cuboids = valid_cuboids_for_mesh(&mut mesh, col.cuboids.clone());
    if cuboids.len() != col.cuboids.len() {
        app.status_message =
            "Could not stage COL: rotated-box metadata no longer matches generated geometry"
                .to_string();
        return;
    }
    if let Err(error) = sync_generated_primitive_face_ranges(&mesh, &mut capsules, &mut cuboids) {
        app.status_message = format!("Could not stage COL: {error}");
        return;
    }
    let source_bytes = col.bytes.clone();
    let source_model = col.source_model.clone();
    let name = col.name.clone();
    let resource_root = app.root.clone();
    let embedded_vehicle_dff = col.embedded_vehicle_dff;
    if editing_shadow && !mesh.shadow_faces.is_empty() {
        if let Err(err) =
            snap_shadow_vertices_to_col_grid(&mut mesh.shadow_vertices).and_then(|_| {
                orient_closed_shadow_components(&mesh.shadow_vertices, &mut mesh.shadow_faces)
                    .map(|_| ())
            })
        {
            app.status_message = format!("Could not stage shadow mesh for {name}: {err}");
            return;
        }
    }
    let staged = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        write_col_mesh_replacing_model(&source_bytes, &source_model, &mesh)
    }));
    match staged {
        Ok(Ok(bytes)) => {
            let Some(staged_model) = col_model_ranges(&bytes)
                .into_iter()
                .nth(source_model.index)
                .map(|range| range.identity)
            else {
                app.status_message =
                    format!("Could not stage COL {name}: rewritten model disappeared");
                return;
            };
            let entry = ImgEntry {
                img_path: PathBuf::from(&name),
                name: name.clone(),
                offset: 0,
                size: bytes.len().min(u32::MAX as usize) as u32,
            };
            if parse_col_mesh_for_identity(&bytes, &entry, &staged_model).is_none() {
                app.status_message =
                    format!("Could not stage COL {name}: rewritten bytes failed validation");
                return;
            }
            let load_issues = validate_col_for_game_load(&name, &bytes);
            if col_validation_has_errors(&load_issues) {
                let reason = load_issues
                    .iter()
                    .find(|issue| issue.severity == ColLoadIssueSeverity::Error)
                    .map(|issue| issue.message.as_str())
                    .unwrap_or("game-load validation failed");
                app.status_message = format!("Could not stage COL {name}: {reason}");
                return;
            }
            let entry_bytes = if embedded_vehicle_dff {
                let Some(dff_bytes) = app.editing.asset.as_ref().and_then(|asset| match asset {
                    EditingAsset::Col(col) => col.embedded_source_dff_bytes.clone(),
                    _ => None,
                }) else {
                    app.status_message =
                        format!("Could not stage embedded COL {name}: source DFF is unavailable");
                    return;
                };
                match replace_embedded_vehicle_collision(&dff_bytes, &bytes) {
                    Ok(dff_bytes) => dff_bytes,
                    Err(err) => {
                        app.status_message = format!("Could not stage embedded COL {name}: {err}");
                        return;
                    }
                }
            } else {
                bytes.clone()
            };
            if let Err(err) = save_collision_capsules(&resource_root, &name, &capsules) {
                app.status_message = format!("Could not stage capsule metadata for {name}: {err}");
                return;
            }
            if let Err(err) = save_collision_cuboids(&resource_root, &name, &cuboids) {
                app.status_message =
                    format!("Could not stage rotated-box metadata for {name}: {err}");
                return;
            }
            let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() else {
                return;
            };
            col.bytes = bytes.clone();
            col.source_model = staged_model;
            if embedded_vehicle_dff {
                col.embedded_source_dff_bytes = Some(entry_bytes.clone());
            }
            col.mesh = mesh;
            col.capsules = capsules;
            col.cuboids = cuboids;
            if col.editing_shadow {
                // Keep the editor on the same layer after staging while the
                // stored/serialized mesh remains in canonical collision-first
                // order.
                std::mem::swap(&mut col.mesh.vertices, &mut col.mesh.shadow_vertices);
                std::mem::swap(&mut col.mesh.faces, &mut col.mesh.shadow_faces);
                col.mesh.bounds = bounds_from_vertices(&col.mesh.vertices);
            }
            app.editing
                .modified_entries
                .insert(editing_key(&name), entry_bytes.clone());
            col.dirty = false;
            if embedded_vehicle_dff {
                app.vehicle_browser.preview_key.clear();
                app.vehicle_browser.embedded_collision = None;
            }
            refresh_live_asset_from_editing_entry(app, &name, &entry_bytes);
            let warnings = load_issues.len();
            app.status_message = if embedded_vehicle_dff && warnings == 0 {
                format!("Staged rewritten embedded COL in {name}")
            } else if embedded_vehicle_dff {
                format!(
                    "Staged rewritten embedded COL in {name} with {warnings} game-load warning(s)"
                )
            } else if warnings == 0 {
                format!("Staged rewritten COL {name}")
            } else {
                format!("Staged rewritten COL {name} with {warnings} game-load warning(s)")
            };
        }
        Ok(Err(err)) => app.status_message = format!("Could not write COL {name}: {err}"),
        Err(_) => app.status_message = format!("Could not write COL {name}: serializer crashed"),
    }
}

pub(crate) fn collision_selected_vertex_index_from_face(
    face: &CollisionFace,
    selected_vertex: usize,
) -> usize {
    match selected_vertex % 3 {
        0 => face.a as usize,
        1 => face.b as usize,
        _ => face.c as usize,
    }
}

pub(crate) fn selected_collision_tab_vertex_position(app: &AppState) -> Option<Vec3> {
    let selected = app.selected_col_face?;
    let placement = app.placements.get(selected.placement)?;
    let key = element_collision_key(app, placement);
    let mesh = app.collisions.get(&key)?;
    let face = mesh.faces.get(selected.face)?;
    let vertex_idx = collision_selected_vertex_index_from_face(face, app.selected_col_vertex);
    let vertex = mesh.vertices.get(vertex_idx)?;
    Some(placement_matrix(placement).transform_point3(to_mq(*vertex)))
}

pub(crate) fn nearest_collision_tab_face_vertex(
    app: &AppState,
    viewport: Rect,
    mouse: Vec2,
    selected: SelectedCollisionFace,
) -> usize {
    let Some(placement) = app.placements.get(selected.placement) else {
        return 0;
    };
    let key = element_collision_key(app, placement);
    let Some(mesh) = app.collisions.get(&key) else {
        return 0;
    };
    let Some(face) = mesh.faces.get(selected.face) else {
        return 0;
    };
    let model = placement_matrix(placement);
    [face.a, face.b, face.c]
        .into_iter()
        .enumerate()
        .filter_map(|(slot, idx)| {
            let vertex = mesh.vertices.get(idx as usize)?;
            let screen = world_to_screen(app, viewport, model.transform_point3(to_mq(*vertex)))?;
            Some((slot, screen.distance(mouse)))
        })
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(slot, _)| slot)
        .unwrap_or(0)
}

fn collision_tab_candidate_indices(app: &AppState) -> Vec<usize> {
    let mut candidates = selected_live_indices(app);
    if let Some(selected) = app.selected_col_face {
        candidates.push(selected.placement);
    }
    candidates.sort_unstable();
    candidates.dedup();
    candidates
}

pub(crate) fn nearest_selected_collision_tab_screen_vertex(
    app: &AppState,
    viewport: Rect,
    mouse: Vec2,
) -> Option<(SelectedCollisionFace, usize)> {
    let candidates = collision_tab_candidate_indices(app);
    if candidates.is_empty() {
        return None;
    }
    nearest_collision_tab_screen_vertex_in(app, viewport, mouse, &candidates)
}

fn nearest_collision_tab_screen_vertex_in(
    app: &AppState,
    viewport: Rect,
    mouse: Vec2,
    candidates: &[usize],
) -> Option<(SelectedCollisionFace, usize)> {
    let ray = viewport_ray(app, viewport, mouse);
    let mut best: Option<(SelectedCollisionFace, usize, f32)> = None;
    for placement_idx in candidates.iter().copied() {
        if app
            .element_states
            .get(placement_idx)
            .is_some_and(|state| state.deleted || state.hidden)
        {
            continue;
        }
        let Some(placement) = app.placements.get(placement_idx) else {
            continue;
        };
        if !app.lod_selectable && placement_is_app_lod(app, placement) {
            continue;
        }
        let Some(mesh) = element_collision_mesh(app, placement) else {
            continue;
        };
        let model = placement_matrix(placement);
        let local_ray = ray.map(|(origin, dir)| {
            let inv = model.inverse().to_cols_array();
            let local_origin = transform_point_gl(&inv, from_mq(origin));
            let local_far = transform_point_gl(&inv, from_mq(origin + dir));
            let local_dir = (local_far - local_origin).normalize_or_zero();
            (local_origin, local_dir)
        });
        for (face_idx, face) in mesh.faces.iter().enumerate() {
            for (slot, vertex_idx) in [face.a, face.b, face.c].into_iter().enumerate() {
                let Some(vertex) = mesh.vertices.get(vertex_idx as usize) else {
                    continue;
                };
                let world = model.transform_point3(to_mq(*vertex));
                let Some(screen) = world_to_screen(app, viewport, world) else {
                    continue;
                };
                let distance = screen.distance(mouse);
                if distance > COLLISION_TAB_VERTEX_PICK_RADIUS {
                    continue;
                }
                if let Some((origin, dir)) = local_ray {
                    if dir.length_squared() > 0.0001 {
                        let vertex_depth = ray_depth_to_point(origin, dir, to_mq(*vertex));
                        if vertex_depth > 0.0
                            && col_mesh_occludes_vertex(
                                mesh,
                                origin,
                                dir,
                                vertex_idx as usize,
                                vertex_depth,
                            )
                        {
                            continue;
                        }
                    }
                }
                if match best {
                    Some((_, _, best_distance)) => distance < best_distance,
                    None => true,
                } {
                    best = Some((
                        SelectedCollisionFace {
                            placement: placement_idx,
                            face: face_idx,
                        },
                        slot,
                        distance,
                    ));
                }
            }
        }
    }
    best.map(|(face, slot, _)| (face, slot))
}

pub(crate) fn pick_selected_collision_face(
    app: &AppState,
    viewport: Rect,
    mouse: Vec2,
) -> Option<SelectedCollisionFace> {
    let candidates = collision_tab_candidate_indices(app);
    if candidates.is_empty() {
        return None;
    }
    let (origin, dir) = viewport_ray(app, viewport, mouse)?;
    let mut best = None;
    let mut best_t = f32::MAX;
    for idx in candidates {
        if app
            .element_states
            .get(idx)
            .is_some_and(|state| state.deleted || state.hidden)
        {
            continue;
        }
        let Some(placement) = app.placements.get(idx) else {
            continue;
        };
        let Some(mesh) = element_collision_mesh(app, placement) else {
            continue;
        };
        let inv = placement_matrix(placement).inverse().to_cols_array();
        let local_origin = transform_point_gl(&inv, from_mq(origin));
        let local_far = transform_point_gl(&inv, from_mq(origin + dir));
        let local_dir = local_far - local_origin;
        if local_dir.length_squared() < 0.0001 {
            continue;
        }
        let local_dir = local_dir.normalize();
        if ray_aabb(local_origin, local_dir, mesh.bounds.min, mesh.bounds.max).is_none() {
            continue;
        }
        if let Some((face, t)) = ray_collision_triangles(local_origin, local_dir, mesh) {
            if t < best_t {
                best_t = t;
                best = Some(SelectedCollisionFace {
                    placement: idx,
                    face,
                });
            }
        }
    }
    best
}

pub(crate) fn stage_collision_tab_col_replacement(
    app: &mut AppState,
    col_key: &str,
) -> Result<(), String> {
    let mesh = app
        .collisions
        .get(col_key)
        .ok_or_else(|| format!("{col_key}: COL mesh is not loaded"))?;
    let entry = find_col_entry(&app.root, col_key)
        .ok_or_else(|| format!("{col_key}: source COL entry was not found"))?;
    let bytes = read_img_entry(&entry);
    if bytes.is_empty() {
        return Err(format!("{col_key}: source COL bytes could not be read"));
    }
    let mut updated = write_col_mesh_from_template(&bytes, mesh)?;
    set_col_model_names_from_entry(&mut updated, col_key);
    let normalized = normalize_legacy_light_mapper_asset_name(col_key);
    app.pending_replacement_assets
        .insert(lower(&normalized), (normalized, updated));
    app.loaded_wip = true;
    invalidate_validation_cache(app);
    Ok(())
}

pub(crate) fn set_selected_collision_tab_material(
    app: &mut AppState,
    material: u8,
) -> Result<(), String> {
    let selected = app
        .selected_col_face
        .ok_or_else(|| "Pick a collision face first".to_string())?;
    let placement = app
        .placements
        .get(selected.placement)
        .ok_or_else(|| "Selected collision placement no longer exists".to_string())?
        .clone();
    let key = element_collision_key(app, &placement);
    let mesh = app
        .collisions
        .get_mut(&key)
        .ok_or_else(|| "Selected element has no parsed COL mesh".to_string())?;
    let face = mesh
        .faces
        .get_mut(selected.face)
        .ok_or_else(|| "Selected collision face no longer exists".to_string())?;
    face.material = material;
    stage_collision_tab_col_replacement(app, &key)?;
    app.status_message = format!(
        "COL face material set to {}; replacement staged",
        col_material_label(material)
    );
    Ok(())
}

fn selected_collision_tab_col_key(app: &AppState) -> Result<String, String> {
    let selected = app
        .selected_col_face
        .ok_or_else(|| "Pick a collision face first".to_string())?;
    let placement = app
        .placements
        .get(selected.placement)
        .ok_or_else(|| "Selected collision placement no longer exists".to_string())?;
    Ok(element_collision_key(app, placement))
}

pub(crate) fn stage_selected_collision_tab_col(app: &mut AppState) -> Result<(), String> {
    let key = selected_collision_tab_col_key(app)?;
    stage_collision_tab_col_replacement(app, &key)?;
    app.status_message = format!("Staged COL replacement for {key}");
    Ok(())
}

pub(crate) fn set_selected_collision_tab_vertex_position_live(
    app: &mut AppState,
    world_position: Vec3,
) -> Result<(), String> {
    let selected = app
        .selected_col_face
        .ok_or_else(|| "Pick a collision face first".to_string())?;
    let placement = app
        .placements
        .get(selected.placement)
        .ok_or_else(|| "Selected collision placement no longer exists".to_string())?
        .clone();
    let key = element_collision_key(app, &placement);
    let inv = placement_matrix(&placement).inverse();
    let local = from_mq(inv.transform_point3(world_position));
    let mesh = app
        .collisions
        .get_mut(&key)
        .ok_or_else(|| "Selected element has no parsed COL mesh".to_string())?;
    let face = mesh
        .faces
        .get(selected.face)
        .ok_or_else(|| "Selected collision face no longer exists".to_string())?;
    let vertex_idx = collision_selected_vertex_index_from_face(face, app.selected_col_vertex);
    let vertex = mesh
        .vertices
        .get_mut(vertex_idx)
        .ok_or_else(|| "Selected collision vertex no longer exists".to_string())?;
    *vertex = local;
    mesh.bounds = bounds_from_vertices(&mesh.vertices);
    app.status_message = format!("Moved COL vertex {vertex_idx}");
    Ok(())
}

pub(crate) fn set_selected_collision_tab_vertex_position(
    app: &mut AppState,
    world_position: Vec3,
) -> Result<(), String> {
    set_selected_collision_tab_vertex_position_live(app, world_position)?;
    stage_selected_collision_tab_col(app)?;
    Ok(())
}

fn selected_col_vertex_index(col: &EditingColState) -> Option<usize> {
    if col.selected_primitive.is_some() {
        return None;
    }
    let face = col.mesh.faces.get(col.selected_face)?;
    let vertex = match col.selected_vertex % 3 {
        0 => face.a,
        1 => face.b,
        _ => face.c,
    };
    Some(vertex as usize).filter(|idx| *idx < col.mesh.vertices.len())
}

fn col_selected_vertex_set(col: &EditingColState) -> BTreeSet<usize> {
    let mut selected = col
        .selected_vertices
        .iter()
        .copied()
        .filter(|idx| *idx < col.mesh.vertices.len())
        .collect::<BTreeSet<_>>();
    if selected.is_empty() && col.select_mode == EditingSelectMode::Face {
        for face_idx in &col.selected_faces {
            if let Some(face) = col.mesh.faces.get(*face_idx) {
                selected.insert(face.a as usize);
                selected.insert(face.b as usize);
                selected.insert(face.c as usize);
            }
        }
    }
    if selected.is_empty() {
        for (a, b) in col_selected_edge_set(col) {
            selected.insert(a);
            selected.insert(b);
        }
    }
    if selected.is_empty() {
        if let Some(vertex) = selected_col_vertex_index(col) {
            selected.insert(vertex);
        }
    }
    selected
}

fn col_explicit_selected_vertex_set(col: &EditingColState) -> BTreeSet<usize> {
    col.selected_vertices
        .iter()
        .copied()
        .filter(|idx| *idx < col.mesh.vertices.len())
        .collect()
}

fn col_selected_face_set(col: &EditingColState) -> BTreeSet<usize> {
    let mut selected = col
        .selected_faces
        .iter()
        .copied()
        .filter(|idx| *idx < col.mesh.faces.len())
        .collect::<BTreeSet<_>>();
    if col.selected_primitive.is_none() && col.selected_face < col.mesh.faces.len() {
        selected.insert(col.selected_face);
    }
    selected
}

fn col_face_indices_with_material(mesh: &CollisionMesh, material: u8) -> BTreeSet<usize> {
    mesh.faces
        .iter()
        .enumerate()
        .filter_map(|(idx, face)| (face.material == material).then_some(idx))
        .collect()
}

fn col_vertex_indices_with_material(mesh: &CollisionMesh, material: u8) -> BTreeSet<usize> {
    mesh.faces
        .iter()
        .filter(|face| face.material == material)
        .flat_map(|face| [face.a as usize, face.b as usize, face.c as usize])
        .filter(|idx| *idx < mesh.vertices.len())
        .collect()
}

pub(crate) fn editing_select_all_with_selected_col_material(app: &mut AppState) -> bool {
    let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() else {
        return false;
    };
    if col.selected_primitive.is_some() {
        return false;
    }
    let Some(material) = col
        .mesh
        .faces
        .get(col.selected_face)
        .map(|face| face.material)
    else {
        return false;
    };

    col.selected_primitive = None;
    col.selected_edges.clear();
    match col.select_mode {
        EditingSelectMode::Face => {
            col.selected_faces = col_face_indices_with_material(&col.mesh, material)
                .into_iter()
                .filter(|face| col_generated_primitive_for_face(col, *face).is_none())
                .collect();
            col.selected_vertices.clear();
            let count = col.selected_faces.len();
            app.status_message = format!(
                "Selected {count} COL face(s) with material {}",
                col_material_label(material)
            );
        }
        EditingSelectMode::Vertex => {
            col.selected_vertices = col_vertex_indices_with_material(&col.mesh, material)
                .into_iter()
                .filter(|vertex| col_generated_primitive_for_vertex(col, *vertex).is_none())
                .collect();
            col.selected_faces.clear();
            let count = col.selected_vertices.len();
            app.status_message = format!(
                "Selected {count} COL vertices on faces with material {}",
                col_material_label(material)
            );
        }
        EditingSelectMode::Edge => return false,
    }
    true
}

fn col_clear_selection(col: &mut EditingColState) {
    col.selected_face = usize::MAX;
    col.selected_vertex = 0;
    col.selected_faces.clear();
    col.selected_edges.clear();
    col.selected_vertices.clear();
    col.selected_primitive = None;
    col.hovered_face = None;
    col.hovered_vertex = None;
}

fn swap_editing_col_mesh_layer(col: &mut EditingColState) {
    std::mem::swap(&mut col.mesh.vertices, &mut col.mesh.shadow_vertices);
    std::mem::swap(&mut col.mesh.faces, &mut col.mesh.shadow_faces);
    col.editing_shadow = !col.editing_shadow;
    col.mesh.bounds = if col.editing_shadow {
        bounds_from_vertices(&col.mesh.vertices)
    } else {
        collision_mesh_bounds(&col.mesh.vertices, &col.mesh.spheres, &col.mesh.boxes)
    };
    col.face_scroll = 0.0;
    col.primitive_scroll = 0.0;
    col_clear_selection(col);
    col.selected_face = 0;
}

fn normalized_editing_col_mesh(col: &EditingColState) -> CollisionMesh {
    let mut mesh = col.mesh.clone();
    if col.editing_shadow {
        std::mem::swap(&mut mesh.vertices, &mut mesh.shadow_vertices);
        std::mem::swap(&mut mesh.faces, &mut mesh.shadow_faces);
    }
    mesh.bounds = collision_mesh_bounds(&mesh.vertices, &mesh.spheres, &mesh.boxes);
    mesh
}

pub(crate) fn toggle_editing_col_shadow_layer(app: &mut AppState) {
    let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() else {
        return;
    };
    swap_editing_col_mesh_layer(col);
    app.col_box_face_drag = None;
    app.col_box_hovered_face = None;
    app.status_message = if col.editing_shadow {
        if col.mesh.faces.is_empty() {
            "Editing shadow mesh (empty); generate a closed skin or create faces manually"
                .to_string()
        } else {
            format!(
                "Editing shadow mesh: {} vertices, {} faces",
                col.mesh.vertices.len(),
                col.mesh.faces.len()
            )
        }
    } else {
        "Editing regular collision mesh".to_string()
    };
}

fn col_set_single_face_selection(col: &mut EditingColState, face: usize) {
    col.selected_face = face.min(col.mesh.faces.len().saturating_sub(1));
    col.selected_faces.clear();
    if col.selected_face < col.mesh.faces.len() {
        col.selected_faces.insert(col.selected_face);
    }
    col.selected_primitive = None;
}

fn col_toggle_face_selection(col: &mut EditingColState, face: usize) {
    if face >= col.mesh.faces.len() {
        return;
    }
    col.selected_face = face;
    col.selected_primitive = None;
    if !col.selected_faces.insert(face) {
        col.selected_faces.remove(&face);
        if col.selected_faces.is_empty() {
            col.selected_faces.insert(face);
        }
    }
}

pub(crate) fn set_selected_editing_col_vertex_position(
    app: &mut AppState,
    position: V3,
) -> Result<(), String> {
    let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() else {
        return Err("Open a COL in Editing first".to_string());
    };
    let selected = col_selected_vertex_set(col);
    if selected.is_empty() {
        return Err("Pick a COL vertex or edge first".to_string());
    }
    if col_vertices_touch_generated_primitive(col, selected.iter().copied()) {
        return Err(
            "Select the capsule or rotated box primitive to transform its generated vertices"
                .to_string(),
        );
    }
    let mut start = V3::default();
    let mut count = 0.0f32;
    for idx in &selected {
        if let Some(vertex) = col.mesh.vertices.get(*idx) {
            start.x += vertex.x;
            start.y += vertex.y;
            start.z += vertex.z;
            count += 1.0;
        }
    }
    if count <= 0.0 {
        return Err("Selected COL vertex no longer exists".to_string());
    }
    start.x /= count;
    start.y /= count;
    start.z /= count;
    let delta = V3 {
        x: position.x - start.x,
        y: position.y - start.y,
        z: position.z - start.z,
    };
    for idx in selected {
        if let Some(vertex) = col.mesh.vertices.get_mut(idx) {
            vertex.x += delta.x;
            vertex.y += delta.y;
            vertex.z += delta.z;
        }
    }
    refresh_editing_col_bounds(col);
    col.dirty = true;
    Ok(())
}

pub(crate) fn selected_editing_col_vertex_position(app: &AppState) -> Option<Vec3> {
    let Some(EditingAsset::Col(col)) = app.editing.asset.as_ref() else {
        return None;
    };
    let selected = col_selected_vertex_set(col);
    if selected.is_empty() {
        return None;
    }
    let mut sum = Vec3::ZERO;
    let mut count = 0.0f32;
    for idx in selected {
        if let Some(vertex) = col.mesh.vertices.get(idx) {
            sum += to_mq(*vertex);
            count += 1.0;
        }
    }
    (count > 0.0).then_some(sum / count)
}

const COL_CAPSULE_SIDES: usize = 16;
const COL_CAPSULE_VERTEX_COUNT: usize = COL_CAPSULE_SIDES * 2 + 2;
const COL_CAPSULE_FACE_COUNT: usize = COL_CAPSULE_SIDES * 4;
const COL_CUBOID_VERTEX_COUNT: usize = 8;
const COL_CUBOID_FACE_COUNT: usize = 12;

fn cuboid_rotation_matrix(rotation: V3) -> Mat4 {
    Mat4::from_rotation_z(rotation.z.to_radians())
        * Mat4::from_rotation_y(rotation.y.to_radians())
        * Mat4::from_rotation_x(rotation.x.to_radians())
}

fn cuboid_local_corners(half_extents: V3) -> [Vec3; COL_CUBOID_VERTEX_COUNT] {
    let half = to_mq(half_extents);
    [
        vec3(-half.x, -half.y, -half.z),
        vec3(half.x, -half.y, -half.z),
        vec3(half.x, half.y, -half.z),
        vec3(-half.x, half.y, -half.z),
        vec3(-half.x, -half.y, half.z),
        vec3(half.x, -half.y, half.z),
        vec3(half.x, half.y, half.z),
        vec3(-half.x, half.y, half.z),
    ]
}

fn cuboid_triangle_indices() -> [[usize; 3]; COL_CUBOID_FACE_COUNT] {
    [
        [0, 2, 1],
        [0, 3, 2],
        [4, 5, 6],
        [4, 6, 7],
        [0, 1, 5],
        [0, 5, 4],
        [1, 2, 6],
        [1, 6, 5],
        [2, 3, 7],
        [2, 7, 6],
        [3, 0, 4],
        [3, 4, 7],
    ]
}

fn write_cuboid_artifacts(
    mesh: &mut CollisionMesh,
    cuboid: &mut CollisionCuboid,
) -> Result<(), String> {
    if cuboid.vertex_start + cuboid.vertex_count > mesh.vertices.len()
        || cuboid.face_start + cuboid.face_count > mesh.faces.len()
        || cuboid.vertex_count != COL_CUBOID_VERTEX_COUNT
        || cuboid.face_count != COL_CUBOID_FACE_COUNT
    {
        return Err("Rotated box mesh representation is missing".to_string());
    }
    cuboid.half_extents.x = cuboid.half_extents.x.abs().max(0.005);
    cuboid.half_extents.y = cuboid.half_extents.y.abs().max(0.005);
    cuboid.half_extents.z = cuboid.half_extents.z.abs().max(0.005);
    let center = to_mq(cuboid.center);
    let rotation = cuboid_rotation_matrix(cuboid.rotation);
    for (slot, local) in cuboid_local_corners(cuboid.half_extents)
        .into_iter()
        .enumerate()
    {
        mesh.vertices[cuboid.vertex_start + slot] =
            from_mq(center + rotation.transform_vector3(local));
    }
    for (slot, indices) in cuboid_triangle_indices().into_iter().enumerate() {
        mesh.faces[cuboid.face_start + slot] = capsule_generated_face(
            cuboid.vertex_start + indices[0],
            cuboid.vertex_start + indices[1],
            cuboid.vertex_start + indices[2],
            &cuboid.surface,
        );
    }
    Ok(())
}

fn append_cuboid_artifacts(
    mesh: &mut CollisionMesh,
    center: V3,
    half_extents: V3,
    rotation: V3,
    surface: CollisionSurface,
) -> Result<CollisionCuboid, String> {
    if mesh.vertices.len() + COL_CUBOID_VERTEX_COUNT > u16::MAX as usize {
        return Err("Rotated box would exceed the COL 16-bit vertex limit".to_string());
    }
    if mesh.faces.len() + COL_CUBOID_FACE_COUNT > u16::MAX as usize {
        return Err("Rotated box would exceed the COL 16-bit face limit".to_string());
    }
    let vertex_start = mesh.vertices.len();
    mesh.vertices
        .resize(vertex_start + COL_CUBOID_VERTEX_COUNT, V3::default());
    let face_start = mesh.faces.len();
    mesh.faces
        .resize_with(face_start + COL_CUBOID_FACE_COUNT, || {
            capsule_generated_face(0, 0, 0, &surface)
        });
    let mut cuboid = CollisionCuboid {
        center,
        half_extents,
        rotation,
        surface,
        vertex_start,
        vertex_count: COL_CUBOID_VERTEX_COUNT,
        face_start,
        face_count: COL_CUBOID_FACE_COUNT,
    };
    write_cuboid_artifacts(mesh, &mut cuboid)?;
    Ok(cuboid)
}

pub(crate) fn valid_capsules_for_mesh(
    mesh: &mut CollisionMesh,
    capsules: Vec<CollisionCapsule>,
) -> Vec<CollisionCapsule> {
    let mut used_spheres = BTreeSet::new();
    let mut used_vertices = BTreeSet::new();
    let mut capsules = capsules
        .into_iter()
        .filter(|capsule| {
            let structurally_valid = capsule.vertex_count == COL_CAPSULE_VERTEX_COUNT
                && capsule.face_count == COL_CAPSULE_FACE_COUNT
                && capsule
                    .sphere_indices
                    .iter()
                    .all(|index| *index < mesh.spheres.len() && !used_spheres.contains(index))
                && (capsule.vertex_start..capsule.vertex_start + capsule.vertex_count)
                    .all(|index| index < mesh.vertices.len() && !used_vertices.contains(&index));
            if structurally_valid {
                used_spheres.extend(capsule.sphere_indices);
                used_vertices
                    .extend(capsule.vertex_start..capsule.vertex_start + capsule.vertex_count);
            }
            structurally_valid
        })
        .collect::<Vec<_>>();

    // COL face-group serialization may reorder triangle faces. Recover each
    // capsule cylinder by its dedicated vertex range, then append those faces
    // contiguously again so subsequent in-place edits remain bounded.
    let mut capsule_faces = vec![Vec::<CollisionFace>::new(); capsules.len()];
    let mut ordinary_faces = Vec::with_capacity(mesh.faces.len());
    for face in mesh.faces.drain(..) {
        let indices = [face.a as usize, face.b as usize, face.c as usize];
        let owner = capsules.iter().position(|capsule| {
            indices.iter().all(|index| {
                *index >= capsule.vertex_start
                    && *index < capsule.vertex_start + capsule.vertex_count
            })
        });
        if let Some(owner) = owner {
            capsule_faces[owner].push(face);
        } else {
            ordinary_faces.push(face);
        }
    }
    let mut valid = Vec::with_capacity(capsules.len());
    mesh.faces = ordinary_faces;
    for (mut capsule, faces) in capsules.drain(..).zip(capsule_faces) {
        if faces.len() != COL_CAPSULE_FACE_COUNT {
            mesh.faces.extend(faces);
            continue;
        }
        capsule.face_start = mesh.faces.len();
        capsule.face_count = faces.len();
        mesh.faces.extend(faces);
        valid.push(capsule);
    }
    valid
}

pub(crate) fn valid_cuboids_for_mesh(
    mesh: &mut CollisionMesh,
    cuboids: Vec<CollisionCuboid>,
) -> Vec<CollisionCuboid> {
    let mut used_vertices = BTreeSet::new();
    let mut cuboids = cuboids
        .into_iter()
        .filter(|cuboid| {
            let valid = cuboid.vertex_count == COL_CUBOID_VERTEX_COUNT
                && cuboid.face_count == COL_CUBOID_FACE_COUNT
                && (cuboid.vertex_start..cuboid.vertex_start + cuboid.vertex_count)
                    .all(|index| index < mesh.vertices.len() && !used_vertices.contains(&index));
            if valid {
                used_vertices
                    .extend(cuboid.vertex_start..cuboid.vertex_start + cuboid.vertex_count);
            }
            valid
        })
        .collect::<Vec<_>>();
    let mut cuboid_faces = vec![Vec::<CollisionFace>::new(); cuboids.len()];
    let mut ordinary_faces = Vec::with_capacity(mesh.faces.len());
    for face in mesh.faces.drain(..) {
        let indices = [face.a as usize, face.b as usize, face.c as usize];
        let owner = cuboids.iter().position(|cuboid| {
            indices.iter().all(|index| {
                *index >= cuboid.vertex_start && *index < cuboid.vertex_start + cuboid.vertex_count
            })
        });
        if let Some(owner) = owner {
            cuboid_faces[owner].push(face);
        } else {
            ordinary_faces.push(face);
        }
    }
    mesh.faces = ordinary_faces;
    let mut valid = Vec::with_capacity(cuboids.len());
    for (mut cuboid, faces) in cuboids.drain(..).zip(cuboid_faces) {
        if faces.len() != COL_CUBOID_FACE_COUNT {
            mesh.faces.extend(faces);
            continue;
        }
        cuboid.face_start = mesh.faces.len();
        cuboid.face_count = faces.len();
        mesh.faces.extend(faces);
        valid.push(cuboid);
    }
    valid
}

pub(crate) fn sync_generated_primitive_face_ranges(
    mesh: &CollisionMesh,
    capsules: &mut [CollisionCapsule],
    cuboids: &mut [CollisionCuboid],
) -> Result<(), String> {
    let sync_range = |vertex_start: usize,
                      vertex_count: usize,
                      expected_faces: usize|
     -> Result<usize, String> {
        let indices = mesh
            .faces
            .iter()
            .enumerate()
            .filter_map(|(face_index, face)| {
                [face.a as usize, face.b as usize, face.c as usize]
                    .iter()
                    .all(|index| *index >= vertex_start && *index < vertex_start + vertex_count)
                    .then_some(face_index)
            })
            .collect::<Vec<_>>();
        if indices.len() != expected_faces
            || indices
                .windows(2)
                .any(|pair| pair[1] != pair[0].saturating_add(1))
        {
            return Err("Generated primitive faces are missing or non-contiguous".to_string());
        }
        indices
            .first()
            .copied()
            .ok_or_else(|| "Generated primitive has no faces".to_string())
    };
    for capsule in capsules {
        capsule.face_start = sync_range(
            capsule.vertex_start,
            capsule.vertex_count,
            capsule.face_count,
        )?;
    }
    for cuboid in cuboids {
        cuboid.face_start =
            sync_range(cuboid.vertex_start, cuboid.vertex_count, cuboid.face_count)?;
    }
    Ok(())
}

#[derive(Clone)]
struct AuditedCuboid {
    faces: Vec<usize>,
    vertices: Vec<usize>,
    center: V3,
    half_extents: V3,
    rotation: V3,
    surface: CollisionSurface,
    axis_aligned: bool,
}

fn canonical_plane_axis(mut axis: Vec3) -> Vec3 {
    let largest = if axis.x.abs() >= axis.y.abs() && axis.x.abs() >= axis.z.abs() {
        axis.x
    } else if axis.y.abs() >= axis.z.abs() {
        axis.y
    } else {
        axis.z
    };
    if largest < 0.0 {
        axis = -axis;
    }
    axis
}

fn cuboid_euler_from_axes(x_axis: Vec3, y_axis: Vec3, z_axis: Vec3) -> V3 {
    let sy = (-x_axis.z).clamp(-1.0, 1.0);
    let y = sy.asin();
    let cy = y.cos();
    let (x, z) = if cy.abs() > 1.0e-5 {
        (y_axis.z.atan2(z_axis.z), x_axis.y.atan2(x_axis.x))
    } else {
        (0.0, (-y_axis.x).atan2(y_axis.y))
    };
    normalize_cuboid_rotation(V3 {
        x: x.to_degrees(),
        y: y.to_degrees(),
        z: z.to_degrees(),
    })
}

fn audit_cuboid_component(
    mesh: &CollisionMesh,
    component: &[usize],
    vertex_faces: &[Vec<usize>],
) -> Option<AuditedCuboid> {
    if component.len() != COL_CUBOID_FACE_COUNT {
        return None;
    }
    let component_set = component.iter().copied().collect::<HashSet<_>>();
    let mut vertex_set = BTreeSet::new();
    let first = mesh.faces.get(component[0])?;
    for &face_index in component {
        let face = mesh.faces.get(face_index)?;
        if face.material != first.material || face.light != first.light {
            return None;
        }
        vertex_set.extend([face.a as usize, face.b as usize, face.c as usize]);
    }
    if vertex_set.len() != COL_CUBOID_VERTEX_COUNT
        || vertex_set.iter().any(|vertex| {
            vertex_faces
                .get(*vertex)
                .is_none_or(|faces| faces.iter().any(|face| !component_set.contains(face)))
        })
    {
        return None;
    }

    let mut component_edges = HashMap::<(u16, u16), usize>::new();
    let mut normal_groups = Vec::<(Vec3, usize)>::new();
    for &face_index in component {
        let face = &mesh.faces[face_index];
        for (a, b) in [(face.a, face.b), (face.b, face.c), (face.c, face.a)] {
            *component_edges.entry((a.min(b), a.max(b))).or_default() += 1;
        }
        let a = to_mq(mesh.vertices[face.a as usize]);
        let b = to_mq(mesh.vertices[face.b as usize]);
        let c = to_mq(mesh.vertices[face.c as usize]);
        let normal = (b - a).cross(c - a).normalize_or_zero();
        if normal.length_squared() < 0.99 {
            return None;
        }
        let normal = canonical_plane_axis(normal);
        if let Some((_, count)) = normal_groups
            .iter_mut()
            .find(|(axis, _)| axis.dot(normal) > 0.999)
        {
            *count += 1;
        } else {
            normal_groups.push((normal, 1));
        }
    }
    if component_edges.values().any(|count| *count != 2)
        || normal_groups.len() != 3
        || normal_groups.iter().any(|(_, count)| *count != 4)
    {
        return None;
    }
    let normals = [normal_groups[0].0, normal_groups[1].0, normal_groups[2].0];
    if normals[0].dot(normals[1]).abs() > 0.002
        || normals[0].dot(normals[2]).abs() > 0.002
        || normals[1].dot(normals[2]).abs() > 0.002
    {
        return None;
    }

    let world = [Vec3::X, Vec3::Y, Vec3::Z];
    let permutations = [
        [0usize, 1usize, 2usize],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];
    let assignment = permutations.into_iter().max_by(|a, b| {
        let score = |permutation: &[usize; 3]| {
            (0..3)
                .map(|axis| normals[permutation[axis]].dot(world[axis]).abs())
                .sum::<f32>()
        };
        score(a).total_cmp(&score(b))
    })?;
    let mut axes = [
        normals[assignment[0]],
        normals[assignment[1]],
        normals[assignment[2]],
    ];
    for axis in 0..3 {
        if axes[axis].dot(world[axis]) < 0.0 {
            axes[axis] = -axes[axis];
        }
    }
    if axes[0].cross(axes[1]).dot(axes[2]) < 0.0 {
        axes[2] = -axes[2];
    }

    let vertices = vertex_set.into_iter().collect::<Vec<_>>();
    let center = vertices
        .iter()
        .map(|index| to_mq(mesh.vertices[*index]))
        .fold(Vec3::ZERO, |sum, point| sum + point)
        / vertices.len() as f32;
    let mut half = Vec3::ZERO;
    for index in &vertices {
        let point = to_mq(mesh.vertices[*index]) - center;
        half.x = half.x.max(point.dot(axes[0]).abs());
        half.y = half.y.max(point.dot(axes[1]).abs());
        half.z = half.z.max(point.dot(axes[2]).abs());
    }
    if half.min_element() <= 0.0001 {
        return None;
    }
    let tolerance = half.max_element() * 0.002 + 0.0002;
    for index in &vertices {
        let point = to_mq(mesh.vertices[*index]) - center;
        for axis in 0..3 {
            if (point.dot(axes[axis]).abs() - half[axis]).abs() > tolerance {
                return None;
            }
        }
    }
    let rotation = cuboid_euler_from_axes(axes[0], axes[1], axes[2]);
    let axis_aligned =
        rotation.x.abs() < 0.01 && rotation.y.abs() < 0.01 && rotation.z.abs() < 0.01;
    Some(AuditedCuboid {
        faces: component.to_vec(),
        vertices,
        center: from_mq(center),
        half_extents: from_mq(half),
        rotation,
        surface: CollisionSurface {
            material: first.material,
            flags: 0,
            brightness: 0,
            light: first.light,
        },
        axis_aligned,
    })
}

fn audit_collision_cuboids(
    mut mesh: CollisionMesh,
    mut capsules: Vec<CollisionCapsule>,
    mut cuboids: Vec<CollisionCuboid>,
) -> Result<CollisionCuboidAuditResult, String> {
    let excluded_ranges = capsules
        .iter()
        .map(|capsule| capsule.vertex_start..capsule.vertex_start + capsule.vertex_count)
        .chain(
            cuboids
                .iter()
                .map(|cuboid| cuboid.vertex_start..cuboid.vertex_start + cuboid.vertex_count),
        )
        .collect::<Vec<_>>();
    let eligible = mesh
        .faces
        .iter()
        .map(|face| {
            let indices = [face.a as usize, face.b as usize, face.c as usize];
            !excluded_ranges
                .iter()
                .any(|range| indices.iter().all(|index| range.contains(index)))
        })
        .collect::<Vec<_>>();
    let mut edge_faces = HashMap::<(u16, u16), Vec<usize>>::new();
    let mut vertex_faces = vec![Vec::<usize>::new(); mesh.vertices.len()];
    for (face_index, face) in mesh.faces.iter().enumerate() {
        for index in [face.a as usize, face.b as usize, face.c as usize] {
            if let Some(faces) = vertex_faces.get_mut(index) {
                faces.push(face_index);
            }
        }
        if !eligible[face_index] {
            continue;
        }
        for (a, b) in [(face.a, face.b), (face.b, face.c), (face.c, face.a)] {
            edge_faces
                .entry((a.min(b), a.max(b)))
                .or_default()
                .push(face_index);
        }
    }
    let mut seen = vec![false; mesh.faces.len()];
    let mut audited = Vec::new();
    for start in 0..mesh.faces.len() {
        if !eligible[start] || seen[start] {
            continue;
        }
        let mut stack = vec![start];
        let mut component = Vec::new();
        seen[start] = true;
        while let Some(face_index) = stack.pop() {
            component.push(face_index);
            let face = &mesh.faces[face_index];
            for (a, b) in [(face.a, face.b), (face.b, face.c), (face.c, face.a)] {
                if let Some(neighbors) = edge_faces.get(&(a.min(b), a.max(b))) {
                    for &neighbor in neighbors {
                        if eligible[neighbor] && !seen[neighbor] {
                            seen[neighbor] = true;
                            stack.push(neighbor);
                        }
                    }
                }
            }
        }
        if let Some(candidate) = audit_cuboid_component(&mesh, &component, &vertex_faces) {
            audited.push(candidate);
        }
    }
    if audited.is_empty() {
        return Ok(CollisionCuboidAuditResult {
            mesh,
            capsules,
            cuboids,
            native_boxes: 0,
            rotated_boxes: 0,
        });
    }

    let removed_faces = audited
        .iter()
        .flat_map(|candidate| candidate.faces.iter().copied())
        .collect::<HashSet<_>>();
    let removed_vertices = audited
        .iter()
        .flat_map(|candidate| candidate.vertices.iter().copied())
        .collect::<HashSet<_>>();
    let mut vertex_map = vec![None; mesh.vertices.len()];
    let mut vertices = Vec::with_capacity(mesh.vertices.len() - removed_vertices.len());
    for (old, vertex) in mesh.vertices.drain(..).enumerate() {
        if !removed_vertices.contains(&old) {
            vertex_map[old] = Some(vertices.len());
            vertices.push(vertex);
        }
    }
    let mut faces = Vec::with_capacity(mesh.faces.len() - removed_faces.len());
    for (face_index, mut face) in mesh.faces.drain(..).enumerate() {
        if removed_faces.contains(&face_index) {
            continue;
        }
        face.a = u16::try_from(
            vertex_map[face.a as usize]
                .ok_or("Cuboid audit encountered a retained face using a removed vertex")?,
        )
        .map_err(|_| "Cuboid audit vertex remap exceeded 16-bit indices")?;
        face.b = u16::try_from(
            vertex_map[face.b as usize]
                .ok_or("Cuboid audit encountered a retained face using a removed vertex")?,
        )
        .map_err(|_| "Cuboid audit vertex remap exceeded 16-bit indices")?;
        face.c = u16::try_from(
            vertex_map[face.c as usize]
                .ok_or("Cuboid audit encountered a retained face using a removed vertex")?,
        )
        .map_err(|_| "Cuboid audit vertex remap exceeded 16-bit indices")?;
        faces.push(face);
    }
    mesh.vertices = vertices;
    mesh.faces = faces;
    for capsule in &mut capsules {
        capsule.vertex_start = vertex_map
            .get(capsule.vertex_start)
            .and_then(|value| *value)
            .ok_or("Cuboid audit invalidated capsule vertex metadata")?;
    }
    for cuboid in &mut cuboids {
        cuboid.vertex_start = vertex_map
            .get(cuboid.vertex_start)
            .and_then(|value| *value)
            .ok_or("Cuboid audit invalidated rotated-box vertex metadata")?;
    }
    capsules = valid_capsules_for_mesh(&mut mesh, capsules);
    cuboids = valid_cuboids_for_mesh(&mut mesh, cuboids);
    sync_generated_primitive_face_ranges(&mesh, &mut capsules, &mut cuboids)?;

    let mut native_boxes = 0;
    let mut rotated_boxes = 0;
    for candidate in audited {
        if candidate.axis_aligned {
            let center = to_mq(candidate.center);
            let half = to_mq(candidate.half_extents);
            mesh.boxes.push(CollisionBox {
                min: from_mq(center - half),
                max: from_mq(center + half),
                surface: candidate.surface,
            });
            native_boxes += 1;
        } else {
            cuboids.push(append_cuboid_artifacts(
                &mut mesh,
                candidate.center,
                candidate.half_extents,
                candidate.rotation,
                candidate.surface,
            )?);
            rotated_boxes += 1;
        }
    }
    Ok(CollisionCuboidAuditResult {
        mesh,
        capsules,
        cuboids,
        native_boxes,
        rotated_boxes,
    })
}

pub(crate) fn start_editing_col_cuboid_audit(app: &mut AppState) {
    let Some(EditingAsset::Col(col)) = app.editing.asset.as_ref() else {
        return;
    };
    if col.editing_shadow {
        return;
    }
    let target_name = col.name.clone();
    let mesh = col.mesh.clone();
    let capsules = col.capsules.clone();
    let cuboids = col.cuboids.clone();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = std::panic::catch_unwind(|| audit_collision_cuboids(mesh, capsules, cuboids))
            .unwrap_or_else(|_| Err("Collision cuboid audit worker crashed".to_string()));
        let _ = tx.send(result);
    });
    app.collision_cuboid_audit_job = Some(CollisionCuboidAuditJob { rx, target_name });
}

pub(crate) fn update_collision_cuboid_audit_job(app: &mut AppState) {
    let Some(job) = app.collision_cuboid_audit_job.take() else {
        return;
    };
    let result = match job.rx.try_recv() {
        Ok(result) => result,
        Err(mpsc::TryRecvError::Empty) => {
            app.collision_cuboid_audit_job = Some(job);
            return;
        }
        Err(mpsc::TryRecvError::Disconnected) => {
            app.status_message = "Collision cuboid audit worker disconnected".to_string();
            return;
        }
    };
    let result = match result {
        Ok(result) => result,
        Err(error) => {
            app.status_message = format!("Could not audit collision cuboids: {error}");
            return;
        }
    };
    let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() else {
        return;
    };
    if !col.name.eq_ignore_ascii_case(&job.target_name) || col.dirty {
        return;
    }
    if result.native_boxes == 0 && result.rotated_boxes == 0 {
        return;
    }
    col.mesh = result.mesh;
    col.capsules = result.capsules;
    col.cuboids = result.cuboids;
    col.selected_primitive = None;
    col.selected_faces.clear();
    col.selected_edges.clear();
    col.selected_vertices.clear();
    col.dirty = true;
    app.status_message = format!(
        "Cuboid audit recovered {} native and {} rotated editable box(es)",
        result.native_boxes, result.rotated_boxes
    );
}

fn capsule_axis(capsule: &CollisionCapsule) -> Vec3 {
    let delta = to_mq(capsule.end) - to_mq(capsule.start);
    if delta.length_squared() > 1.0e-8 {
        delta.normalize()
    } else {
        Vec3::Z
    }
}

fn capsule_generated_face(
    a: usize,
    b: usize,
    c: usize,
    surface: &CollisionSurface,
) -> CollisionFace {
    CollisionFace {
        a: a as u16,
        b: b as u16,
        c: c as u16,
        material: surface.material,
        light: surface.light,
        img_path: PathBuf::new(),
        material_file_offset: 0,
        light_file_offset: 0,
    }
}

fn write_capsule_artifacts(
    mesh: &mut CollisionMesh,
    capsule: &mut CollisionCapsule,
) -> Result<(), String> {
    if capsule
        .sphere_indices
        .iter()
        .any(|index| *index >= mesh.spheres.len())
    {
        return Err("Capsule sphere representation is missing".to_string());
    }
    if capsule.vertex_start + capsule.vertex_count > mesh.vertices.len()
        || capsule.face_start + capsule.face_count > mesh.faces.len()
        || capsule.vertex_count != COL_CAPSULE_VERTEX_COUNT
        || capsule.face_count != COL_CAPSULE_FACE_COUNT
    {
        return Err("Capsule cylinder representation is missing".to_string());
    }
    capsule.radius = capsule.radius.abs().max(0.001);
    for (sphere_index, center) in capsule
        .sphere_indices
        .into_iter()
        .zip([capsule.start, capsule.end])
    {
        mesh.spheres[sphere_index] = CollisionSphere {
            center,
            radius: capsule.radius,
            surface: capsule.surface.clone(),
        };
    }

    let axis = capsule_axis(capsule);
    let helper = if axis.z.abs() < 0.9 { Vec3::Z } else { Vec3::Y };
    let right = axis.cross(helper).normalize_or_zero();
    let up = right.cross(axis).normalize_or_zero();
    let radius = capsule.radius;
    let (ring_start, ring_end) = if capsule.round_edges {
        (to_mq(capsule.start), to_mq(capsule.end))
    } else {
        (
            to_mq(capsule.start) - axis * radius,
            to_mq(capsule.end) + axis * radius,
        )
    };
    for side in 0..COL_CAPSULE_SIDES {
        let angle = side as f32 / COL_CAPSULE_SIDES as f32 * std::f32::consts::TAU;
        let radial = (right * angle.cos() + up * angle.sin()) * radius;
        mesh.vertices[capsule.vertex_start + side] = from_mq(ring_start + radial);
        mesh.vertices[capsule.vertex_start + COL_CAPSULE_SIDES + side] = from_mq(ring_end + radial);
    }
    mesh.vertices[capsule.vertex_start + COL_CAPSULE_SIDES * 2] = from_mq(ring_start);
    mesh.vertices[capsule.vertex_start + COL_CAPSULE_SIDES * 2 + 1] = from_mq(ring_end);

    let mut output = capsule.face_start;
    let vertex = capsule.vertex_start;
    let start_center = vertex + COL_CAPSULE_SIDES * 2;
    let end_center = start_center + 1;
    for side in 0..COL_CAPSULE_SIDES {
        let next = (side + 1) % COL_CAPSULE_SIDES;
        let a = vertex + side;
        let b = vertex + next;
        let c = vertex + COL_CAPSULE_SIDES + next;
        let d = vertex + COL_CAPSULE_SIDES + side;
        for (fa, fb, fc) in [
            (a, c, b),
            (a, d, c),
            (start_center, b, a),
            (end_center, d, c),
        ] {
            mesh.faces[output] = capsule_generated_face(fa, fb, fc, &capsule.surface);
            output += 1;
        }
    }
    Ok(())
}

fn append_capsule_artifacts(
    mesh: &mut CollisionMesh,
    start: V3,
    end: V3,
    radius: f32,
    round_edges: bool,
    surface: CollisionSurface,
) -> Result<CollisionCapsule, String> {
    if mesh.vertices.len() + COL_CAPSULE_VERTEX_COUNT > u16::MAX as usize {
        return Err("Capsule would exceed the COL 16-bit vertex limit".to_string());
    }
    if mesh.faces.len() + COL_CAPSULE_FACE_COUNT > u16::MAX as usize {
        return Err("Capsule would exceed the COL 16-bit face limit".to_string());
    }
    let sphere_start = mesh.spheres.len();
    mesh.spheres.extend([
        CollisionSphere {
            center: start,
            radius,
            surface: surface.clone(),
        },
        CollisionSphere {
            center: end,
            radius,
            surface: surface.clone(),
        },
    ]);
    let vertex_start = mesh.vertices.len();
    mesh.vertices
        .resize(vertex_start + COL_CAPSULE_VERTEX_COUNT, V3::default());
    let face_start = mesh.faces.len();
    mesh.faces
        .resize_with(face_start + COL_CAPSULE_FACE_COUNT, || {
            capsule_generated_face(0, 0, 0, &surface)
        });
    let mut capsule = CollisionCapsule {
        start,
        end,
        radius,
        round_edges,
        surface,
        sphere_indices: [sphere_start, sphere_start + 1],
        vertex_start,
        vertex_count: COL_CAPSULE_VERTEX_COUNT,
        face_start,
        face_count: COL_CAPSULE_FACE_COUNT,
    };
    write_capsule_artifacts(mesh, &mut capsule)?;
    Ok(capsule)
}

fn refresh_capsule_artifacts(
    col: &mut EditingColState,
    capsule_index: usize,
) -> Result<(), String> {
    let capsules = valid_capsules_for_mesh(&mut col.mesh, col.capsules.clone());
    if capsules.len() != col.capsules.len() {
        return Err("Capsule metadata no longer matches its generated COL geometry".to_string());
    }
    col.capsules = capsules;
    sync_generated_primitive_face_ranges(&col.mesh, &mut col.capsules, &mut col.cuboids)?;
    let capsule = col
        .capsules
        .get_mut(capsule_index)
        .ok_or_else(|| "Selected COL capsule no longer exists".to_string())?;
    write_capsule_artifacts(&mut col.mesh, capsule)?;
    refresh_editing_col_bounds(col);
    Ok(())
}

fn refresh_cuboid_artifacts(col: &mut EditingColState, cuboid_index: usize) -> Result<(), String> {
    let cuboids = valid_cuboids_for_mesh(&mut col.mesh, col.cuboids.clone());
    if cuboids.len() != col.cuboids.len() {
        return Err(
            "Rotated box metadata no longer matches its six-plane COL geometry".to_string(),
        );
    }
    col.cuboids = cuboids;
    sync_generated_primitive_face_ranges(&col.mesh, &mut col.capsules, &mut col.cuboids)?;
    let cuboid = col
        .cuboids
        .get_mut(cuboid_index)
        .ok_or_else(|| "Selected rotated COL box no longer exists".to_string())?;
    write_cuboid_artifacts(&mut col.mesh, cuboid)?;
    refresh_editing_col_bounds(col);
    Ok(())
}

pub(crate) fn selected_editing_col_primitive_position(app: &AppState) -> Option<Vec3> {
    let Some(EditingAsset::Col(col)) = app.editing.asset.as_ref() else {
        return None;
    };
    let selected = col.selected_primitive?;
    match selected.kind {
        CollisionPrimitiveKind::Sphere => col
            .mesh
            .spheres
            .get(selected.index)
            .map(|sphere| to_mq(sphere.center)),
        CollisionPrimitiveKind::Box => col.mesh.boxes.get(selected.index).map(|col_box| {
            to_mq(V3 {
                x: (col_box.min.x + col_box.max.x) * 0.5,
                y: (col_box.min.y + col_box.max.y) * 0.5,
                z: (col_box.min.z + col_box.max.z) * 0.5,
            })
        }),
        CollisionPrimitiveKind::Cuboid => col
            .cuboids
            .get(selected.index)
            .map(|cuboid| to_mq(cuboid.center)),
        CollisionPrimitiveKind::Capsule => col
            .capsules
            .get(selected.index)
            .map(|capsule| (to_mq(capsule.start) + to_mq(capsule.end)) * 0.5),
    }
}

pub(crate) fn set_selected_editing_col_primitive_position(
    app: &mut AppState,
    position: V3,
) -> Result<(), String> {
    let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() else {
        return Err("Open a COL in Editing first".to_string());
    };
    let selected = col
        .selected_primitive
        .ok_or_else(|| "Select a COL sphere, box, or capsule first".to_string())?;
    match selected.kind {
        CollisionPrimitiveKind::Sphere => {
            let sphere = col
                .mesh
                .spheres
                .get_mut(selected.index)
                .ok_or_else(|| "Selected COL sphere no longer exists".to_string())?;
            sphere.center = position;
        }
        CollisionPrimitiveKind::Box => {
            let col_box = col
                .mesh
                .boxes
                .get_mut(selected.index)
                .ok_or_else(|| "Selected COL box no longer exists".to_string())?;
            let center = V3 {
                x: (col_box.min.x + col_box.max.x) * 0.5,
                y: (col_box.min.y + col_box.max.y) * 0.5,
                z: (col_box.min.z + col_box.max.z) * 0.5,
            };
            let delta = V3 {
                x: position.x - center.x,
                y: position.y - center.y,
                z: position.z - center.z,
            };
            col_box.min.x += delta.x;
            col_box.min.y += delta.y;
            col_box.min.z += delta.z;
            col_box.max.x += delta.x;
            col_box.max.y += delta.y;
            col_box.max.z += delta.z;
        }
        CollisionPrimitiveKind::Cuboid => {
            let cuboid = col
                .cuboids
                .get_mut(selected.index)
                .ok_or_else(|| "Selected rotated COL box no longer exists".to_string())?;
            cuboid.center = position;
            refresh_cuboid_artifacts(col, selected.index)?;
        }
        CollisionPrimitiveKind::Capsule => {
            let capsule = col
                .capsules
                .get_mut(selected.index)
                .ok_or_else(|| "Selected COL capsule no longer exists".to_string())?;
            let center = (to_mq(capsule.start) + to_mq(capsule.end)) * 0.5;
            let delta = to_mq(position) - center;
            capsule.start = from_mq(to_mq(capsule.start) + delta);
            capsule.end = from_mq(to_mq(capsule.end) + delta);
            refresh_capsule_artifacts(col, selected.index)?;
        }
    }
    col.mesh.bounds = collision_mesh_bounds(&col.mesh.vertices, &col.mesh.spheres, &col.mesh.boxes);
    col.dirty = true;
    Ok(())
}

pub(crate) fn set_selected_editing_col_capsule_orientation(
    app: &mut AppState,
    center: V3,
    half_axis: V3,
) -> Result<(), String> {
    let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() else {
        return Err("Open a COL in Editing first".to_string());
    };
    let selected = col
        .selected_primitive
        .filter(|selected| selected.kind == CollisionPrimitiveKind::Capsule)
        .ok_or_else(|| "Select a COL capsule first".to_string())?;
    let capsule = col
        .capsules
        .get_mut(selected.index)
        .ok_or_else(|| "Selected COL capsule no longer exists".to_string())?;
    let half_axis = to_mq(half_axis);
    if half_axis.length_squared() <= 1.0e-8 {
        return Err("Capsule length must be greater than zero".to_string());
    }
    let center = to_mq(center);
    capsule.start = from_mq(center - half_axis);
    capsule.end = from_mq(center + half_axis);
    refresh_capsule_artifacts(col, selected.index)?;
    col.dirty = true;
    Ok(())
}

fn normalize_cuboid_rotation(mut rotation: V3) -> V3 {
    let normalize = |value: f32| {
        let mut value = (value + 180.0).rem_euclid(360.0) - 180.0;
        if value.abs() < 0.0005 {
            value = 0.0;
        }
        value
    };
    rotation.x = normalize(rotation.x);
    rotation.y = normalize(rotation.y);
    rotation.z = normalize(rotation.z);
    rotation
}

fn remove_generated_mesh_range(
    mesh: &mut CollisionMesh,
    vertex_start: usize,
    vertex_count: usize,
    face_start: usize,
    face_count: usize,
) {
    let vertex_end = vertex_start.saturating_add(vertex_count);
    let face_end = face_start.saturating_add(face_count);
    let mut remap = vec![usize::MAX; mesh.vertices.len()];
    let mut vertices = Vec::with_capacity(mesh.vertices.len().saturating_sub(vertex_count));
    for (old, vertex) in mesh.vertices.iter().copied().enumerate() {
        if old < vertex_start || old >= vertex_end {
            remap[old] = vertices.len();
            vertices.push(vertex);
        }
    }
    let mut faces = Vec::with_capacity(mesh.faces.len().saturating_sub(face_count));
    for (index, face) in mesh.faces.iter().enumerate() {
        if index >= face_start && index < face_end {
            continue;
        }
        let mapped = [
            remap.get(face.a as usize).copied().unwrap_or(usize::MAX),
            remap.get(face.b as usize).copied().unwrap_or(usize::MAX),
            remap.get(face.c as usize).copied().unwrap_or(usize::MAX),
        ];
        if mapped.iter().any(|index| *index == usize::MAX) {
            continue;
        }
        let mut face = face.clone();
        face.a = mapped[0] as u16;
        face.b = mapped[1] as u16;
        face.c = mapped[2] as u16;
        faces.push(face);
    }
    mesh.vertices = vertices;
    mesh.faces = faces;
}

pub(crate) fn selected_editing_col_primitive_rotation(app: &AppState) -> Option<V3> {
    let Some(EditingAsset::Col(col)) = app.editing.asset.as_ref() else {
        return None;
    };
    let selected = col.selected_primitive?;
    match selected.kind {
        CollisionPrimitiveKind::Box => Some(V3::default()),
        CollisionPrimitiveKind::Cuboid => col
            .cuboids
            .get(selected.index)
            .map(|cuboid| cuboid.rotation),
        _ => None,
    }
}

pub(crate) fn set_selected_editing_col_box_rotation(
    app: &mut AppState,
    rotation: V3,
) -> Result<(), String> {
    let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() else {
        return Err("Open a COL in Editing first".to_string());
    };
    let selected = col
        .selected_primitive
        .ok_or_else(|| "Select a COL box first".to_string())?;
    let rotation = normalize_cuboid_rotation(rotation);
    let zero = rotation == V3::default();
    match selected.kind {
        CollisionPrimitiveKind::Box => {
            if zero {
                return Ok(());
            }
            let col_box = col
                .mesh
                .boxes
                .get(selected.index)
                .cloned()
                .ok_or_else(|| "Selected COL box no longer exists".to_string())?;
            let center = V3 {
                x: (col_box.min.x + col_box.max.x) * 0.5,
                y: (col_box.min.y + col_box.max.y) * 0.5,
                z: (col_box.min.z + col_box.max.z) * 0.5,
            };
            let half_extents = V3 {
                x: (col_box.max.x - col_box.min.x).abs() * 0.5,
                y: (col_box.max.y - col_box.min.y).abs() * 0.5,
                z: (col_box.max.z - col_box.min.z).abs() * 0.5,
            };
            let cuboid = append_cuboid_artifacts(
                &mut col.mesh,
                center,
                half_extents,
                rotation,
                col_box.surface,
            )?;
            col.mesh.boxes.remove(selected.index);
            col.cuboids.push(cuboid);
            col.selected_primitive = Some(CollisionPrimitiveSelection {
                kind: CollisionPrimitiveKind::Cuboid,
                index: col.cuboids.len() - 1,
            });
        }
        CollisionPrimitiveKind::Cuboid => {
            let cuboids = valid_cuboids_for_mesh(&mut col.mesh, col.cuboids.clone());
            if cuboids.len() != col.cuboids.len() {
                return Err(
                    "Rotated box metadata no longer matches its six-plane geometry".to_string(),
                );
            }
            col.cuboids = cuboids;
            sync_generated_primitive_face_ranges(&col.mesh, &mut col.capsules, &mut col.cuboids)?;
            if zero {
                let cuboid = col
                    .cuboids
                    .get(selected.index)
                    .cloned()
                    .ok_or_else(|| "Selected rotated COL box no longer exists".to_string())?;
                remove_generated_mesh_range(
                    &mut col.mesh,
                    cuboid.vertex_start,
                    cuboid.vertex_count,
                    cuboid.face_start,
                    cuboid.face_count,
                );
                col.cuboids.remove(selected.index);
                for other in &mut col.cuboids {
                    if other.vertex_start > cuboid.vertex_start {
                        other.vertex_start -= cuboid.vertex_count;
                    }
                }
                for capsule in &mut col.capsules {
                    if capsule.vertex_start > cuboid.vertex_start {
                        capsule.vertex_start -= cuboid.vertex_count;
                    }
                }
                sync_generated_primitive_face_ranges(
                    &col.mesh,
                    &mut col.capsules,
                    &mut col.cuboids,
                )?;
                let half = cuboid.half_extents;
                col.mesh.boxes.push(CollisionBox {
                    min: V3 {
                        x: cuboid.center.x - half.x,
                        y: cuboid.center.y - half.y,
                        z: cuboid.center.z - half.z,
                    },
                    max: V3 {
                        x: cuboid.center.x + half.x,
                        y: cuboid.center.y + half.y,
                        z: cuboid.center.z + half.z,
                    },
                    surface: cuboid.surface,
                });
                col.selected_primitive = Some(CollisionPrimitiveSelection {
                    kind: CollisionPrimitiveKind::Box,
                    index: col.mesh.boxes.len() - 1,
                });
            } else {
                let cuboid = col
                    .cuboids
                    .get_mut(selected.index)
                    .ok_or_else(|| "Selected rotated COL box no longer exists".to_string())?;
                cuboid.rotation = rotation;
                refresh_cuboid_artifacts(col, selected.index)?;
            }
        }
        _ => return Err("Select a COL box first".to_string()),
    }
    col.mesh.bounds = collision_mesh_bounds(&col.mesh.vertices, &col.mesh.spheres, &col.mesh.boxes);
    col.dirty = true;
    Ok(())
}

pub(crate) fn resize_editing_col_box_primitive(
    app: &mut AppState,
    primitive: CollisionPrimitiveSelection,
    center: V3,
    half_extents: V3,
) -> Result<(), String> {
    let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() else {
        return Err("Open a COL in Editing first".to_string());
    };
    match primitive.kind {
        CollisionPrimitiveKind::Box => {
            let col_box = col
                .mesh
                .boxes
                .get_mut(primitive.index)
                .ok_or_else(|| "Selected COL box no longer exists".to_string())?;
            col_box.min = V3 {
                x: center.x - half_extents.x,
                y: center.y - half_extents.y,
                z: center.z - half_extents.z,
            };
            col_box.max = V3 {
                x: center.x + half_extents.x,
                y: center.y + half_extents.y,
                z: center.z + half_extents.z,
            };
        }
        CollisionPrimitiveKind::Cuboid => {
            let cuboid = col
                .cuboids
                .get_mut(primitive.index)
                .ok_or_else(|| "Selected rotated COL box no longer exists".to_string())?;
            cuboid.center = center;
            cuboid.half_extents = half_extents;
            refresh_cuboid_artifacts(col, primitive.index)?;
        }
        _ => return Err("Select a COL box face first".to_string()),
    }
    col.mesh.bounds = collision_mesh_bounds(&col.mesh.vertices, &col.mesh.spheres, &col.mesh.boxes);
    col.dirty = true;
    Ok(())
}

pub(crate) fn selected_editing_col_primitive_size(app: &AppState) -> Option<V3> {
    let Some(EditingAsset::Col(col)) = app.editing.asset.as_ref() else {
        return None;
    };
    let selected = col.selected_primitive?;
    match selected.kind {
        CollisionPrimitiveKind::Sphere => col.mesh.spheres.get(selected.index).map(|sphere| V3 {
            x: sphere.radius,
            y: sphere.radius,
            z: sphere.radius,
        }),
        CollisionPrimitiveKind::Box => col.mesh.boxes.get(selected.index).map(|col_box| V3 {
            x: (col_box.max.x - col_box.min.x).abs(),
            y: (col_box.max.y - col_box.min.y).abs(),
            z: (col_box.max.z - col_box.min.z).abs(),
        }),
        CollisionPrimitiveKind::Cuboid => col.cuboids.get(selected.index).map(|cuboid| V3 {
            x: cuboid.half_extents.x * 2.0,
            y: cuboid.half_extents.y * 2.0,
            z: cuboid.half_extents.z * 2.0,
        }),
        CollisionPrimitiveKind::Capsule => col.capsules.get(selected.index).map(|capsule| V3 {
            x: capsule.radius,
            y: capsule.radius,
            z: (to_mq(capsule.end) - to_mq(capsule.start)).length(),
        }),
    }
}

pub(crate) fn set_selected_editing_col_primitive_size(
    app: &mut AppState,
    size: V3,
) -> Result<(), String> {
    let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() else {
        return Err("Open a COL in Editing first".to_string());
    };
    let selected = col
        .selected_primitive
        .ok_or_else(|| "Select a COL sphere, box, or capsule first".to_string())?;
    match selected.kind {
        CollisionPrimitiveKind::Sphere => {
            let sphere = col
                .mesh
                .spheres
                .get_mut(selected.index)
                .ok_or_else(|| "Selected COL sphere no longer exists".to_string())?;
            sphere.radius = size.x.abs().max(0.001);
        }
        CollisionPrimitiveKind::Box => {
            let col_box = col
                .mesh
                .boxes
                .get_mut(selected.index)
                .ok_or_else(|| "Selected COL box no longer exists".to_string())?;
            let center = V3 {
                x: (col_box.min.x + col_box.max.x) * 0.5,
                y: (col_box.min.y + col_box.max.y) * 0.5,
                z: (col_box.min.z + col_box.max.z) * 0.5,
            };
            let half = V3 {
                x: size.x.abs().max(0.001) * 0.5,
                y: size.y.abs().max(0.001) * 0.5,
                z: size.z.abs().max(0.001) * 0.5,
            };
            col_box.min = V3 {
                x: center.x - half.x,
                y: center.y - half.y,
                z: center.z - half.z,
            };
            col_box.max = V3 {
                x: center.x + half.x,
                y: center.y + half.y,
                z: center.z + half.z,
            };
        }
        CollisionPrimitiveKind::Cuboid => {
            let cuboid = col
                .cuboids
                .get_mut(selected.index)
                .ok_or_else(|| "Selected rotated COL box no longer exists".to_string())?;
            cuboid.half_extents = V3 {
                x: size.x.abs().max(0.01) * 0.5,
                y: size.y.abs().max(0.01) * 0.5,
                z: size.z.abs().max(0.01) * 0.5,
            };
            refresh_cuboid_artifacts(col, selected.index)?;
        }
        CollisionPrimitiveKind::Capsule => {
            let capsule = col
                .capsules
                .get_mut(selected.index)
                .ok_or_else(|| "Selected COL capsule no longer exists".to_string())?;
            let center = (to_mq(capsule.start) + to_mq(capsule.end)) * 0.5;
            let axis = capsule_axis(capsule);
            let half_length = size.z.abs().max(0.001) * 0.5;
            capsule.start = from_mq(center - axis * half_length);
            capsule.end = from_mq(center + axis * half_length);
            capsule.radius = size.x.abs().max(0.001);
            refresh_capsule_artifacts(col, selected.index)?;
        }
    }
    col.mesh.bounds = collision_mesh_bounds(&col.mesh.vertices, &col.mesh.spheres, &col.mesh.boxes);
    col.dirty = true;
    Ok(())
}

fn selected_col_primitive_surface_mut(col: &mut EditingColState) -> Option<&mut CollisionSurface> {
    let selected = col.selected_primitive?;
    match selected.kind {
        CollisionPrimitiveKind::Sphere => col
            .mesh
            .spheres
            .get_mut(selected.index)
            .map(|sphere| &mut sphere.surface),
        CollisionPrimitiveKind::Box => col
            .mesh
            .boxes
            .get_mut(selected.index)
            .map(|col_box| &mut col_box.surface),
        CollisionPrimitiveKind::Cuboid => col
            .cuboids
            .get_mut(selected.index)
            .map(|cuboid| &mut cuboid.surface),
        CollisionPrimitiveKind::Capsule => col
            .capsules
            .get_mut(selected.index)
            .map(|capsule| &mut capsule.surface),
    }
}

pub(crate) fn set_selected_editing_col_material(app: &mut AppState, material: u8) -> bool {
    let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() else {
        return false;
    };
    let capsule_index = col
        .selected_primitive
        .filter(|selected| selected.kind == CollisionPrimitiveKind::Capsule)
        .map(|selected| selected.index);
    let cuboid_index = col
        .selected_primitive
        .filter(|selected| selected.kind == CollisionPrimitiveKind::Cuboid)
        .map(|selected| selected.index);
    if let Some(surface) = selected_col_primitive_surface_mut(col) {
        surface.material = material;
        if let Some(index) = capsule_index
            && let Err(err) = refresh_capsule_artifacts(col, index)
        {
            app.status_message = err;
            return false;
        }
        if let Some(index) = cuboid_index
            && let Err(err) = refresh_cuboid_artifacts(col, index)
        {
            app.status_message = err;
            return false;
        }
        col.dirty = true;
        app.status_message = format!(
            "Editing COL primitive material set to {}",
            col_material_label(material)
        );
        return true;
    }
    let selected = col_selected_face_set(col);
    if selected.is_empty() {
        return false;
    }
    if col_faces_touch_generated_primitive(col, selected.iter().copied()) {
        app.status_message =
            "Select the capsule or rotated box primitive to edit its surface".to_string();
        return false;
    }
    for face_idx in &selected {
        if let Some(face) = col.mesh.faces.get_mut(*face_idx) {
            face.material = material;
        }
    }
    col.dirty = true;
    app.status_message = format!(
        "Editing COL material set to {} on {} face(s)",
        col_material_label(material),
        selected.len()
    );
    true
}

pub(crate) fn set_selected_editing_col_light(app: &mut AppState, light: u8) -> bool {
    let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() else {
        return false;
    };
    let capsule_index = col
        .selected_primitive
        .filter(|selected| selected.kind == CollisionPrimitiveKind::Capsule)
        .map(|selected| selected.index);
    let cuboid_index = col
        .selected_primitive
        .filter(|selected| selected.kind == CollisionPrimitiveKind::Cuboid)
        .map(|selected| selected.index);
    if let Some(surface) = selected_col_primitive_surface_mut(col) {
        surface.light = light;
        if let Some(index) = capsule_index
            && let Err(err) = refresh_capsule_artifacts(col, index)
        {
            app.status_message = err;
            return false;
        }
        if let Some(index) = cuboid_index
            && let Err(err) = refresh_cuboid_artifacts(col, index)
        {
            app.status_message = err;
            return false;
        }
        col.dirty = true;
        app.status_message = format!("Editing COL primitive light set to {light}");
        return true;
    }
    let selected = col_selected_face_set(col);
    if selected.is_empty() {
        return false;
    }
    if col_faces_touch_generated_primitive(col, selected.iter().copied()) {
        app.status_message =
            "Select the capsule or rotated box primitive to edit its surface".to_string();
        return false;
    }
    for face_idx in &selected {
        if let Some(face) = col.mesh.faces.get_mut(*face_idx) {
            face.light = light;
        }
    }
    col.dirty = true;
    app.status_message = format!(
        "Editing COL light set to {light} on {} face(s)",
        selected.len()
    );
    true
}

fn selected_col_surface(col: &EditingColState) -> CollisionSurface {
    col.mesh
        .faces
        .get(col.selected_face)
        .map(|face| CollisionSurface {
            material: face.material,
            flags: 0,
            brightness: 0,
            light: face.light,
        })
        .or_else(|| {
            col.mesh
                .spheres
                .first()
                .map(|sphere| sphere.surface.clone())
        })
        .or_else(|| {
            col.mesh
                .boxes
                .first()
                .map(|col_box| col_box.surface.clone())
        })
        .unwrap_or(CollisionSurface {
            material: 0,
            flags: 0,
            brightness: 0,
            light: 255,
        })
}

fn selected_col_insert_center(col: &EditingColState) -> V3 {
    if let Some(face) = col.mesh.faces.get(col.selected_face) {
        let mut sum = Vec3::ZERO;
        let mut count = 0.0f32;
        for idx in [face.a, face.b, face.c] {
            if let Some(vertex) = col.mesh.vertices.get(idx as usize) {
                sum += to_mq(*vertex);
                count += 1.0;
            }
        }
        if count > 0.0 {
            return from_mq(sum / count);
        }
    }
    from_mq((col.mesh.bounds.min + col.mesh.bounds.max) * 0.5)
}

pub(crate) fn editing_add_col_sphere(app: &mut AppState) -> bool {
    let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() else {
        return false;
    };
    let center = selected_col_insert_center(col);
    let radius = ((col.mesh.bounds.max - col.mesh.bounds.min).length() * 0.05).clamp(0.25, 8.0);
    col.mesh.spheres.push(CollisionSphere {
        center,
        radius,
        surface: selected_col_surface(col),
    });
    col.selected_primitive = Some(CollisionPrimitiveSelection {
        kind: CollisionPrimitiveKind::Sphere,
        index: col.mesh.spheres.len() - 1,
    });
    col.selected_vertices.clear();
    col.selected_faces.clear();
    col.selected_edges.clear();
    col.mesh.bounds = collision_mesh_bounds(&col.mesh.vertices, &col.mesh.spheres, &col.mesh.boxes);
    col.dirty = true;
    app.status_message = format!("Added COL sphere {}", col.mesh.spheres.len());
    true
}

pub(crate) fn editing_add_col_box(app: &mut AppState) -> bool {
    let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() else {
        return false;
    };
    let center = selected_col_insert_center(col);
    let size = ((col.mesh.bounds.max - col.mesh.bounds.min).length() * 0.04).clamp(0.25, 8.0);
    let half = V3 {
        x: size,
        y: size,
        z: size,
    };
    col.mesh.boxes.push(CollisionBox {
        min: V3 {
            x: center.x - half.x,
            y: center.y - half.y,
            z: center.z - half.z,
        },
        max: V3 {
            x: center.x + half.x,
            y: center.y + half.y,
            z: center.z + half.z,
        },
        surface: selected_col_surface(col),
    });
    col.selected_primitive = Some(CollisionPrimitiveSelection {
        kind: CollisionPrimitiveKind::Box,
        index: col.mesh.boxes.len() - 1,
    });
    col.selected_vertices.clear();
    col.selected_faces.clear();
    col.selected_edges.clear();
    col.mesh.bounds = collision_mesh_bounds(&col.mesh.vertices, &col.mesh.spheres, &col.mesh.boxes);
    col.dirty = true;
    app.status_message = format!("Added COL box {}", col.mesh.boxes.len());
    true
}

pub(crate) fn editing_add_col_capsule(app: &mut AppState) -> bool {
    let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() else {
        return false;
    };
    let center = selected_col_insert_center(col);
    let scale = ((col.mesh.bounds.max - col.mesh.bounds.min).length() * 0.04).clamp(0.25, 8.0);
    let start = V3 {
        x: center.x,
        y: center.y,
        z: center.z - scale,
    };
    let end = V3 {
        x: center.x,
        y: center.y,
        z: center.z + scale,
    };
    let surface = selected_col_surface(col);
    let capsule = match append_capsule_artifacts(&mut col.mesh, start, end, scale, true, surface) {
        Ok(capsule) => capsule,
        Err(err) => {
            app.status_message = err;
            return false;
        }
    };
    col.capsules.push(capsule);
    col.selected_primitive = Some(CollisionPrimitiveSelection {
        kind: CollisionPrimitiveKind::Capsule,
        index: col.capsules.len() - 1,
    });
    col.selected_vertices.clear();
    col.selected_faces.clear();
    col.selected_edges.clear();
    col.mesh.bounds = collision_mesh_bounds(&col.mesh.vertices, &col.mesh.spheres, &col.mesh.boxes);
    col.dirty = true;
    app.status_message = format!("Added editable COL capsule {}", col.capsules.len());
    true
}

pub(crate) fn editing_toggle_selected_col_capsule_edges(app: &mut AppState) -> bool {
    let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() else {
        return false;
    };
    let Some(selected) = col.selected_primitive else {
        return false;
    };
    if selected.kind != CollisionPrimitiveKind::Capsule {
        return false;
    }
    let round_edges = {
        let Some(capsule) = col.capsules.get_mut(selected.index) else {
            return false;
        };
        capsule.round_edges = !capsule.round_edges;
        capsule.round_edges
    };
    if let Err(err) = refresh_capsule_artifacts(col, selected.index) {
        app.status_message = err;
        return false;
    }
    col.dirty = true;
    app.status_message = if round_edges {
        "Capsule edges set to Round; cylinder ends at the sphere centers".to_string()
    } else {
        "Capsule edges set to Flat; cylinder extends to the outer sphere ends".to_string()
    };
    true
}

pub(crate) fn editing_duplicate_selected_col_primitive(app: &mut AppState) -> bool {
    let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() else {
        return false;
    };
    let Some(selected) = col.selected_primitive else {
        app.status_message =
            "Select a COL sphere, box, rotated box, or capsule to duplicate".to_string();
        return false;
    };
    match selected.kind {
        CollisionPrimitiveKind::Sphere => {
            let Some(sphere) = col.mesh.spheres.get(selected.index).cloned() else {
                app.status_message = "Selected COL sphere no longer exists".to_string();
                return false;
            };
            col.mesh.spheres.push(sphere);
            col.selected_primitive = Some(CollisionPrimitiveSelection {
                kind: CollisionPrimitiveKind::Sphere,
                index: col.mesh.spheres.len() - 1,
            });
            app.status_message = format!("Duplicated COL sphere {}", col.mesh.spheres.len());
        }
        CollisionPrimitiveKind::Box => {
            let Some(col_box) = col.mesh.boxes.get(selected.index).cloned() else {
                app.status_message = "Selected COL box no longer exists".to_string();
                return false;
            };
            col.mesh.boxes.push(col_box);
            col.selected_primitive = Some(CollisionPrimitiveSelection {
                kind: CollisionPrimitiveKind::Box,
                index: col.mesh.boxes.len() - 1,
            });
            app.status_message = format!("Duplicated COL box {}", col.mesh.boxes.len());
        }
        CollisionPrimitiveKind::Cuboid => {
            let Some(source) = col.cuboids.get(selected.index).cloned() else {
                app.status_message = "Selected rotated COL box no longer exists".to_string();
                return false;
            };
            let cuboid = match append_cuboid_artifacts(
                &mut col.mesh,
                source.center,
                source.half_extents,
                source.rotation,
                source.surface,
            ) {
                Ok(cuboid) => cuboid,
                Err(err) => {
                    app.status_message = err;
                    return false;
                }
            };
            col.cuboids.push(cuboid);
            col.selected_primitive = Some(CollisionPrimitiveSelection {
                kind: CollisionPrimitiveKind::Cuboid,
                index: col.cuboids.len() - 1,
            });
            app.status_message = format!("Duplicated rotated COL box {}", col.cuboids.len());
        }
        CollisionPrimitiveKind::Capsule => {
            let Some(source) = col.capsules.get(selected.index).cloned() else {
                app.status_message = "Selected COL capsule no longer exists".to_string();
                return false;
            };
            let capsule = match append_capsule_artifacts(
                &mut col.mesh,
                source.start,
                source.end,
                source.radius,
                source.round_edges,
                source.surface,
            ) {
                Ok(capsule) => capsule,
                Err(err) => {
                    app.status_message = err;
                    return false;
                }
            };
            col.capsules.push(capsule);
            col.selected_primitive = Some(CollisionPrimitiveSelection {
                kind: CollisionPrimitiveKind::Capsule,
                index: col.capsules.len() - 1,
            });
            app.status_message = format!("Duplicated COL capsule {}", col.capsules.len());
        }
    }
    col.selected_vertices.clear();
    col.selected_faces.clear();
    col.selected_edges.clear();
    col.mesh.bounds = collision_mesh_bounds(&col.mesh.vertices, &col.mesh.spheres, &col.mesh.boxes);
    col.dirty = true;
    true
}

pub(crate) fn editing_delete_selected_col_primitive(app: &mut AppState) -> bool {
    let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() else {
        return false;
    };
    let Some(selected) = col.selected_primitive else {
        app.status_message =
            "Select a COL sphere, box, rotated box, or capsule to delete".to_string();
        return false;
    };
    let deleted = match selected.kind {
        CollisionPrimitiveKind::Sphere => {
            if selected.index >= col.mesh.spheres.len() {
                app.status_message = "Selected COL sphere no longer exists".to_string();
                return false;
            }
            col.mesh.spheres.remove(selected.index);
            for capsule in &mut col.capsules {
                for sphere_index in &mut capsule.sphere_indices {
                    if *sphere_index > selected.index {
                        *sphere_index -= 1;
                    }
                }
            }
            "sphere"
        }
        CollisionPrimitiveKind::Box => {
            if selected.index >= col.mesh.boxes.len() {
                app.status_message = "Selected COL box no longer exists".to_string();
                return false;
            }
            col.mesh.boxes.remove(selected.index);
            "box"
        }
        CollisionPrimitiveKind::Cuboid => {
            let cuboids = valid_cuboids_for_mesh(&mut col.mesh, col.cuboids.clone());
            if cuboids.len() != col.cuboids.len() {
                app.status_message =
                    "Rotated box metadata no longer matches its six-plane geometry".to_string();
                return false;
            }
            col.cuboids = cuboids;
            if let Err(error) =
                sync_generated_primitive_face_ranges(&col.mesh, &mut col.capsules, &mut col.cuboids)
            {
                app.status_message = error;
                return false;
            }
            let Some(cuboid) = col.cuboids.get(selected.index).cloned() else {
                app.status_message = "Selected rotated COL box no longer exists".to_string();
                return false;
            };
            remove_generated_mesh_range(
                &mut col.mesh,
                cuboid.vertex_start,
                cuboid.vertex_count,
                cuboid.face_start,
                cuboid.face_count,
            );
            col.cuboids.remove(selected.index);
            for other in &mut col.cuboids {
                if other.vertex_start > cuboid.vertex_start {
                    other.vertex_start -= cuboid.vertex_count;
                }
            }
            for capsule in &mut col.capsules {
                if capsule.vertex_start > cuboid.vertex_start {
                    capsule.vertex_start -= cuboid.vertex_count;
                }
            }
            "rotated box"
        }
        CollisionPrimitiveKind::Capsule => {
            if selected.index >= col.capsules.len() {
                app.status_message = "Selected COL capsule no longer exists".to_string();
                return false;
            }
            let capsules = valid_capsules_for_mesh(&mut col.mesh, col.capsules.clone());
            if capsules.len() != col.capsules.len() {
                app.status_message =
                    "Capsule metadata no longer matches its generated COL geometry".to_string();
                return false;
            }
            col.capsules = capsules;
            // Rebuild all capsule artifacts after removing one so every stored
            // range remains contiguous and valid.
            let mut remaining = col.capsules.clone();
            remaining.remove(selected.index);
            let owned_spheres = col
                .capsules
                .iter()
                .flat_map(|capsule| capsule.sphere_indices)
                .collect::<BTreeSet<_>>();
            col.mesh.spheres = col
                .mesh
                .spheres
                .iter()
                .enumerate()
                .filter(|(index, _)| !owned_spheres.contains(index))
                .map(|(_, sphere)| sphere.clone())
                .collect();
            let owned_vertices = col
                .capsules
                .iter()
                .flat_map(|capsule| {
                    capsule.vertex_start..capsule.vertex_start + capsule.vertex_count
                })
                .collect::<BTreeSet<_>>();
            let owned_faces = col
                .capsules
                .iter()
                .flat_map(|capsule| capsule.face_start..capsule.face_start + capsule.face_count)
                .collect::<BTreeSet<_>>();
            let mut remap = vec![usize::MAX; col.mesh.vertices.len()];
            let mut vertices =
                Vec::with_capacity(col.mesh.vertices.len().saturating_sub(owned_vertices.len()));
            for (old, vertex) in col.mesh.vertices.iter().copied().enumerate() {
                if !owned_vertices.contains(&old) {
                    remap[old] = vertices.len();
                    vertices.push(vertex);
                }
            }
            let mut faces =
                Vec::with_capacity(col.mesh.faces.len().saturating_sub(owned_faces.len()));
            for (face_index, face) in col.mesh.faces.iter().enumerate() {
                if owned_faces.contains(&face_index) {
                    continue;
                }
                let mapped = [
                    remap.get(face.a as usize).copied().unwrap_or(usize::MAX),
                    remap.get(face.b as usize).copied().unwrap_or(usize::MAX),
                    remap.get(face.c as usize).copied().unwrap_or(usize::MAX),
                ];
                if mapped.iter().any(|index| *index == usize::MAX) {
                    continue;
                }
                let mut face = face.clone();
                face.a = mapped[0] as u16;
                face.b = mapped[1] as u16;
                face.c = mapped[2] as u16;
                faces.push(face);
            }
            col.mesh.vertices = vertices;
            col.mesh.faces = faces;
            for cuboid in &mut col.cuboids {
                let removed_before = owned_vertices.range(..cuboid.vertex_start).count();
                cuboid.vertex_start = cuboid.vertex_start.saturating_sub(removed_before);
            }
            col.capsules.clear();
            for source in remaining {
                let rebuilt = match append_capsule_artifacts(
                    &mut col.mesh,
                    source.start,
                    source.end,
                    source.radius,
                    source.round_edges,
                    source.surface,
                ) {
                    Ok(capsule) => capsule,
                    Err(err) => {
                        app.status_message = err;
                        return false;
                    }
                };
                col.capsules.push(rebuilt);
            }
            "capsule"
        }
    };
    if let Err(error) =
        sync_generated_primitive_face_ranges(&col.mesh, &mut col.capsules, &mut col.cuboids)
    {
        app.status_message = error;
        return false;
    }
    col.selected_primitive = None;
    col.selected_vertices.clear();
    col.selected_faces.clear();
    col.selected_edges.clear();
    col.mesh.bounds = collision_mesh_bounds(&col.mesh.vertices, &col.mesh.spheres, &col.mesh.boxes);
    let total = col_primitive_selections(col).len();
    let visible = total.min(COL_PRIM_VISIBLE_MAX);
    col.primitive_scroll = col
        .primitive_scroll
        .min(total.saturating_sub(visible) as f32);
    col.dirty = true;
    app.status_message = format!("Deleted COL {deleted}");
    true
}

pub(crate) fn editing_delete_selected_col_face(app: &mut AppState) {
    let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() else {
        return;
    };
    if col_regular_topology_has_editable_primitives(col) {
        app.status_message =
            "COL topology deletion is disabled while editable capsules or rotated boxes are present"
                .to_string();
        return;
    }
    let selected =
        if col.select_mode == EditingSelectMode::Vertex && !col.selected_vertices.is_empty() {
            let selected_vertices = col_selected_vertex_set(col);
            let faces = col
                .mesh
                .faces
                .iter()
                .enumerate()
                .filter_map(|(idx, face)| {
                    (selected_vertices.contains(&(face.a as usize))
                        || selected_vertices.contains(&(face.b as usize))
                        || selected_vertices.contains(&(face.c as usize)))
                    .then_some(idx)
                })
                .collect::<BTreeSet<_>>();
            if faces.is_empty() {
                app.status_message = "Selected COL vertices have no attached faces".to_string();
                return;
            }
            faces
        } else {
            col_selected_face_set(col)
        };
    if selected.is_empty() {
        return;
    }
    col.mesh.faces = col
        .mesh
        .faces
        .iter()
        .enumerate()
        .filter(|(idx, _)| !selected.contains(idx))
        .map(|(_, face)| face.clone())
        .collect();
    col.dirty = true;
    app.status_message = format!("Deleted {} selected COL face(s)", selected.len());
    editing_optimize_col_mesh(app);
    if let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() {
        col_clear_selection(col);
    }
}

pub(crate) fn editing_delete_selected_col_vertex(app: &mut AppState) {
    let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() else {
        return;
    };
    if col_regular_topology_has_editable_primitives(col) {
        app.status_message =
            "COL topology deletion is disabled while editable capsules or rotated boxes are present"
                .to_string();
        return;
    }
    let selected = col_selected_vertex_set(col);
    if selected.is_empty() {
        app.status_message = "Pick a COL vertex first".to_string();
        return;
    }
    let (removed_vertices, removed_faces) = remove_col_vertices(&mut col.mesh, &selected);
    if removed_vertices == 0 {
        app.status_message = "Selected COL vertex no longer exists".to_string();
        return;
    }
    refresh_editing_col_bounds(col);
    col_clear_selection(col);
    col.dirty = true;
    app.status_message =
        format!("Deleted {removed_vertices} COL vertex(es) and {removed_faces} attached face(s)");
}

pub(crate) fn editing_delete_selected_col_edge(app: &mut AppState) {
    let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() else {
        return;
    };
    if col_regular_topology_has_editable_primitives(col) {
        app.status_message =
            "COL topology deletion is disabled while editable capsules or rotated boxes are present"
                .to_string();
        return;
    }
    let selected = col_selected_edge_set(col);
    if selected.is_empty() {
        app.status_message = "Select COL edge(s) first".to_string();
        return;
    }
    let before = col.mesh.faces.len();
    col.mesh.faces.retain(|face| {
        !col_face_edge_vertices(face)
            .iter()
            .any(|edge| selected.contains(edge))
    });
    let removed = before.saturating_sub(col.mesh.faces.len());
    if removed == 0 {
        app.status_message = "Selected COL edge(s) have no attached faces".to_string();
        return;
    }
    col.dirty = true;
    app.status_message = format!(
        "Deleted {} COL edge selection(s) and {removed} attached face(s)",
        selected.len()
    );
    editing_optimize_col_mesh(app);
    if let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() {
        col_clear_selection(col);
    }
}

pub(crate) fn editing_flip_selected_col_face(app: &mut AppState) -> bool {
    let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() else {
        return false;
    };
    let selected = col_selected_face_set(col);
    if selected.is_empty() {
        app.status_message = "Select a COL face before flipping normals".to_string();
        return false;
    }
    if col_faces_touch_generated_primitive(col, selected.iter().copied()) {
        app.status_message =
            "Generated capsule and rotated-box faces cannot be flipped independently".to_string();
        return false;
    }
    for face_idx in &selected {
        if let Some(face) = col.mesh.faces.get_mut(*face_idx) {
            std::mem::swap(&mut face.b, &mut face.c);
        }
    }
    col.selected_vertex = col.selected_vertex.min(2);
    col.dirty = true;
    app.status_message = format!("Flipped {} COL face normal(s)", selected.len());
    true
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct ColPlaneKey {
    material: u8,
    light: u8,
    nx: i32,
    ny: i32,
    nz: i32,
    d: i32,
}

fn col_face_plane_key(mesh: &CollisionMesh, face: &CollisionFace) -> Option<(ColPlaneKey, Vec3)> {
    let (a, b, c) = collision_face_points(mesh, face)?;
    let mut normal = (b - a).cross(c - a).normalize_or_zero();
    if normal.length_squared() < 0.0001 {
        return None;
    }
    let mut d = normal.dot(a);
    let flip = normal.x < -0.0001
        || (normal.x.abs() <= 0.0001 && normal.y < -0.0001)
        || (normal.x.abs() <= 0.0001 && normal.y.abs() <= 0.0001 && normal.z < -0.0001);
    if flip {
        normal = -normal;
        d = -d;
    }
    Some((
        ColPlaneKey {
            material: face.material,
            light: face.light,
            nx: (normal.x * 96.0).round() as i32,
            ny: (normal.y * 96.0).round() as i32,
            nz: (normal.z * 96.0).round() as i32,
            d: (d * 16.0).round() as i32,
        },
        normal,
    ))
}

fn col_edge_key(a: u16, b: u16) -> (u16, u16) {
    if a <= b { (a, b) } else { (b, a) }
}

fn col_project_for_normal(point: Vec3, normal: Vec3) -> Vec2 {
    let ax = normal.x.abs();
    let ay = normal.y.abs();
    let az = normal.z.abs();
    if az >= ax && az >= ay {
        vec2(point.x, point.y)
    } else if ay >= ax {
        vec2(point.x, point.z)
    } else {
        vec2(point.y, point.z)
    }
}

fn col_polygon_area(points: &[u16], vertices: &[V3], normal: Vec3) -> Option<f32> {
    if points.len() < 3 {
        return None;
    }
    let mut area = 0.0f32;
    for i in 0..points.len() {
        let a = col_project_for_normal(to_mq(*vertices.get(points[i] as usize)?), normal);
        let b = col_project_for_normal(
            to_mq(*vertices.get(points[(i + 1) % points.len()] as usize)?),
            normal,
        );
        area += a.x * b.y - b.x * a.y;
    }
    Some(area * 0.5)
}

fn col_projected_area(points: &[Vec2]) -> f32 {
    if points.len() < 3 {
        return 0.0;
    }
    let mut area = 0.0f32;
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        area += a.x * b.y - b.x * a.y;
    }
    area * 0.5
}

fn col_convex_hull_projected(mut points: Vec<(Vec2, u16)>) -> Vec<u16> {
    const HULL_EPSILON: f32 = 0.0005;
    points.sort_by(|(a, _), (b, _)| {
        a.x.partial_cmp(&b.x)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal))
    });
    points.dedup_by(|(a, _), (b, _)| a.distance_squared(*b) <= HULL_EPSILON * HULL_EPSILON);
    if points.len() <= 2 {
        return points.into_iter().map(|(_, idx)| idx).collect();
    }
    fn cross(o: Vec2, a: Vec2, b: Vec2) -> f32 {
        let oa = a - o;
        let ob = b - o;
        oa.x * ob.y - oa.y * ob.x
    }
    let mut lower = Vec::<(Vec2, u16)>::new();
    for point in &points {
        while lower.len() >= 2
            && cross(lower[lower.len() - 2].0, lower[lower.len() - 1].0, point.0) <= HULL_EPSILON
        {
            lower.pop();
        }
        lower.push(*point);
    }
    let mut upper = Vec::<(Vec2, u16)>::new();
    for point in points.iter().rev() {
        while upper.len() >= 2
            && cross(upper[upper.len() - 2].0, upper[upper.len() - 1].0, point.0) <= HULL_EPSILON
        {
            upper.pop();
        }
        upper.push(*point);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower.into_iter().map(|(_, idx)| idx).collect()
}

fn col_component_projected_area(mesh: &CollisionMesh, component: &[usize], normal: Vec3) -> f32 {
    let mut area = 0.0f32;
    for face_idx in component {
        let Some(face) = mesh.faces.get(*face_idx) else {
            continue;
        };
        let Some(a) = mesh
            .vertices
            .get(face.a as usize)
            .map(|v| col_project_for_normal(to_mq(*v), normal))
        else {
            continue;
        };
        let Some(b) = mesh
            .vertices
            .get(face.b as usize)
            .map(|v| col_project_for_normal(to_mq(*v), normal))
        else {
            continue;
        };
        let Some(c) = mesh
            .vertices
            .get(face.c as usize)
            .map(|v| col_project_for_normal(to_mq(*v), normal))
        else {
            continue;
        };
        area += col_projected_area(&[a, b, c]).abs();
    }
    area
}

fn col_projected_face_points(
    mesh: &CollisionMesh,
    face_idx: usize,
    normal: Vec3,
) -> Option<[Vec2; 3]> {
    let face = mesh.faces.get(face_idx)?;
    Some([
        col_project_for_normal(to_mq(*mesh.vertices.get(face.a as usize)?), normal),
        col_project_for_normal(to_mq(*mesh.vertices.get(face.b as usize)?), normal),
        col_project_for_normal(to_mq(*mesh.vertices.get(face.c as usize)?), normal),
    ])
}

fn col_projected_bounds(points: &[Vec2; 3]) -> (Vec2, Vec2) {
    let mut min = points[0];
    let mut max = points[0];
    for point in &points[1..] {
        min = min.min(*point);
        max = max.max(*point);
    }
    (min, max)
}

fn col_projected_bounds_touch(a: &[Vec2; 3], b: &[Vec2; 3], epsilon: f32) -> bool {
    let (amin, amax) = col_projected_bounds(a);
    let (bmin, bmax) = col_projected_bounds(b);
    amin.x <= bmax.x + epsilon
        && amax.x + epsilon >= bmin.x
        && amin.y <= bmax.y + epsilon
        && amax.y + epsilon >= bmin.y
}

fn col_projected_orient(a: Vec2, b: Vec2, c: Vec2) -> f32 {
    let ab = b - a;
    let ac = c - a;
    ab.x * ac.y - ab.y * ac.x
}

fn col_point_in_projected_tri(point: Vec2, tri: &[Vec2; 3], epsilon: f32) -> bool {
    let a = col_projected_orient(tri[0], tri[1], point);
    let b = col_projected_orient(tri[1], tri[2], point);
    let c = col_projected_orient(tri[2], tri[0], point);
    (a >= -epsilon && b >= -epsilon && c >= -epsilon)
        || (a <= epsilon && b <= epsilon && c <= epsilon)
}

fn col_projected_segments_touch(a0: Vec2, a1: Vec2, b0: Vec2, b1: Vec2, epsilon: f32) -> bool {
    let oa = col_projected_orient(a0, a1, b0);
    let ob = col_projected_orient(a0, a1, b1);
    let oc = col_projected_orient(b0, b1, a0);
    let od = col_projected_orient(b0, b1, a1);
    if oa.abs() <= epsilon && ob.abs() <= epsilon && oc.abs() <= epsilon && od.abs() <= epsilon {
        let amin = a0.min(a1);
        let amax = a0.max(a1);
        let bmin = b0.min(b1);
        let bmax = b0.max(b1);
        return amin.x <= bmax.x + epsilon
            && amax.x + epsilon >= bmin.x
            && amin.y <= bmax.y + epsilon
            && amax.y + epsilon >= bmin.y;
    }
    oa * ob <= epsilon && oc * od <= epsilon
}

fn col_projected_tris_touch(a: &[Vec2; 3], b: &[Vec2; 3]) -> bool {
    const TOUCH_EPSILON: f32 = 0.0025;
    if !col_projected_bounds_touch(a, b, TOUCH_EPSILON) {
        return false;
    }
    for point in a {
        if col_point_in_projected_tri(*point, b, TOUCH_EPSILON) {
            return true;
        }
    }
    for point in b {
        if col_point_in_projected_tri(*point, a, TOUCH_EPSILON) {
            return true;
        }
    }
    for i in 0..3 {
        let a0 = a[i];
        let a1 = a[(i + 1) % 3];
        for j in 0..3 {
            if col_projected_segments_touch(a0, a1, b[j], b[(j + 1) % 3], TOUCH_EPSILON) {
                return true;
            }
        }
    }
    false
}

fn col_remove_collinear_boundary_vertices(points: &mut Vec<u16>, vertices: &[V3], normal: Vec3) {
    let eps = 0.0005f32;
    let mut changed = true;
    while changed && points.len() > 3 {
        changed = false;
        let len = points.len();
        for i in 0..len {
            let prev = points[(i + len - 1) % len];
            let cur = points[i];
            let next = points[(i + 1) % len];
            let Some(prev_p) = vertices
                .get(prev as usize)
                .map(|v| col_project_for_normal(to_mq(*v), normal))
            else {
                continue;
            };
            let Some(cur_p) = vertices
                .get(cur as usize)
                .map(|v| col_project_for_normal(to_mq(*v), normal))
            else {
                continue;
            };
            let Some(next_p) = vertices
                .get(next as usize)
                .map(|v| col_project_for_normal(to_mq(*v), normal))
            else {
                continue;
            };
            let a = cur_p - prev_p;
            let b = next_p - cur_p;
            let cross = a.x * b.y - a.y * b.x;
            if cross.abs() <= eps && a.dot(b) >= -eps {
                points.remove(i);
                changed = true;
                break;
            }
        }
    }
}

fn col_boundary_is_convex(points: &[u16], vertices: &[V3], normal: Vec3) -> bool {
    if points.len() < 3 {
        return false;
    }
    let mut sign = 0.0f32;
    let eps = 0.0005f32;
    for i in 0..points.len() {
        let Some(a) = vertices
            .get(points[i] as usize)
            .map(|v| col_project_for_normal(to_mq(*v), normal))
        else {
            return false;
        };
        let Some(b) = vertices
            .get(points[(i + 1) % points.len()] as usize)
            .map(|v| col_project_for_normal(to_mq(*v), normal))
        else {
            return false;
        };
        let Some(c) = vertices
            .get(points[(i + 2) % points.len()] as usize)
            .map(|v| col_project_for_normal(to_mq(*v), normal))
        else {
            return false;
        };
        let ab = b - a;
        let bc = c - b;
        let cross = ab.x * bc.y - ab.y * bc.x;
        if cross.abs() <= eps {
            continue;
        }
        if sign == 0.0 {
            sign = cross.signum();
        } else if cross.signum() != sign {
            return false;
        }
    }
    sign != 0.0
}

fn col_order_boundary_loop(boundary_edges: &[(u16, u16)]) -> Option<Vec<u16>> {
    let mut adjacency = HashMap::<u16, Vec<u16>>::new();
    for (a, b) in boundary_edges {
        adjacency.entry(*a).or_default().push(*b);
        adjacency.entry(*b).or_default().push(*a);
    }
    if adjacency.values().any(|neighbors| neighbors.len() != 2) {
        return None;
    }
    let start = *adjacency.keys().min()?;
    let mut ordered = vec![start];
    let mut prev = None;
    let mut current = start;
    for _ in 0..adjacency.len() {
        let neighbors = adjacency.get(&current)?;
        let next = if Some(neighbors[0]) == prev {
            neighbors[1]
        } else {
            neighbors[0]
        };
        if next == start {
            return (ordered.len() == adjacency.len()).then_some(ordered);
        }
        if ordered.contains(&next) {
            return None;
        }
        ordered.push(next);
        prev = Some(current);
        current = next;
    }
    None
}

fn col_component_replacement_faces(
    mesh: &CollisionMesh,
    component: &[usize],
) -> Option<Vec<CollisionFace>> {
    if component.len() <= 2 {
        return None;
    }
    let first = mesh.faces.get(component[0])?.clone();
    let (_, target_normal) = col_face_plane_key(mesh, &first)?;
    let mut edge_counts = HashMap::<(u16, u16), usize>::new();
    for face_idx in component {
        let face = mesh.faces.get(*face_idx)?;
        for (a, b) in [(face.a, face.b), (face.b, face.c), (face.c, face.a)] {
            *edge_counts.entry(col_edge_key(a, b)).or_default() += 1;
        }
    }
    let has_non_manifold_edges = edge_counts.values().any(|count| *count > 2);
    let boundary = edge_counts
        .iter()
        .filter_map(|(edge, count)| (*count == 1).then_some(*edge))
        .collect::<Vec<_>>();
    let mut loop_vertices = if !has_non_manifold_edges {
        col_order_boundary_loop(&boundary)
    } else {
        None
    };
    let mut loop_vertices = if let Some(loop_vertices) = loop_vertices.take() {
        loop_vertices
    } else {
        let mut points = Vec::<(Vec2, u16)>::new();
        for face_idx in component {
            let face = mesh.faces.get(*face_idx)?;
            for idx in [face.a, face.b, face.c] {
                let point =
                    col_project_for_normal(to_mq(*mesh.vertices.get(idx as usize)?), target_normal);
                points.push((point, idx));
            }
        }
        let hull = col_convex_hull_projected(points);
        if hull.len() < 3 {
            return None;
        }
        let hull_points = hull
            .iter()
            .filter_map(|idx| {
                mesh.vertices
                    .get(*idx as usize)
                    .map(|vertex| col_project_for_normal(to_mq(*vertex), target_normal))
            })
            .collect::<Vec<_>>();
        let hull_area = col_projected_area(&hull_points).abs();
        let component_area = col_component_projected_area(mesh, component, target_normal);
        if hull_area <= 0.0001 {
            return None;
        }
        let filled_extra = (hull_area - component_area).max(0.0);
        if filled_extra > hull_area * 0.025 {
            return None;
        }
        hull
    };
    col_remove_collinear_boundary_vertices(&mut loop_vertices, &mesh.vertices, target_normal);
    if loop_vertices.len() < 3
        || !col_boundary_is_convex(&loop_vertices, &mesh.vertices, target_normal)
    {
        return None;
    }
    let area = col_polygon_area(&loop_vertices, &mesh.vertices, target_normal)?;
    if area.abs() <= 0.0001 {
        return None;
    }
    let p0 = to_mq(*mesh.vertices.get(loop_vertices[0] as usize)?);
    let p1 = to_mq(*mesh.vertices.get(loop_vertices[1] as usize)?);
    let p2 = to_mq(*mesh.vertices.get(loop_vertices[2] as usize)?);
    if (p1 - p0)
        .cross(p2 - p0)
        .normalize_or_zero()
        .dot(target_normal)
        < 0.0
    {
        loop_vertices.reverse();
    }
    let replacement_count = loop_vertices.len().saturating_sub(2);
    if replacement_count >= component.len() {
        return None;
    }
    let mut faces = Vec::with_capacity(replacement_count);
    for i in 1..loop_vertices.len() - 1 {
        let mut face = first.clone();
        face.a = loop_vertices[0];
        face.b = loop_vertices[i];
        face.c = loop_vertices[i + 1];
        faces.push(face);
    }
    Some(faces)
}

fn simplify_col_coplanar_faces(mesh: &mut CollisionMesh) -> usize {
    let mut groups = HashMap::<ColPlaneKey, Vec<usize>>::new();
    for (idx, face) in mesh.faces.iter().enumerate() {
        if let Some((key, _)) = col_face_plane_key(mesh, face) {
            groups.entry(key).or_default().push(idx);
        }
    }
    let mut replacements = HashMap::<usize, Vec<CollisionFace>>::new();
    let mut remove = BTreeSet::<usize>::new();
    for face_indices in groups.values() {
        if face_indices.len() <= 2 {
            continue;
        }
        let face_set = face_indices.iter().copied().collect::<HashSet<_>>();
        let target_normal = face_indices
            .iter()
            .find_map(|idx| {
                mesh.faces
                    .get(*idx)
                    .and_then(|face| col_face_plane_key(mesh, face).map(|(_, normal)| normal))
            })
            .unwrap_or(Vec3::Z);
        let mut edge_to_faces = HashMap::<(u16, u16), Vec<usize>>::new();
        for idx in face_indices {
            let Some(face) = mesh.faces.get(*idx) else {
                continue;
            };
            for (a, b) in [(face.a, face.b), (face.b, face.c), (face.c, face.a)] {
                edge_to_faces
                    .entry(col_edge_key(a, b))
                    .or_default()
                    .push(*idx);
            }
        }
        let mut adjacency = HashMap::<usize, Vec<usize>>::new();
        for sharing in edge_to_faces.values() {
            if sharing.len() == 2
                && face_set.contains(&sharing[0])
                && face_set.contains(&sharing[1])
            {
                adjacency.entry(sharing[0]).or_default().push(sharing[1]);
                adjacency.entry(sharing[1]).or_default().push(sharing[0]);
            }
        }
        let projected = face_indices
            .iter()
            .filter_map(|idx| {
                col_projected_face_points(mesh, *idx, target_normal).map(|tri| (*idx, tri))
            })
            .collect::<Vec<_>>();
        for i in 0..projected.len() {
            for j in i + 1..projected.len() {
                if col_projected_tris_touch(&projected[i].1, &projected[j].1) {
                    adjacency
                        .entry(projected[i].0)
                        .or_default()
                        .push(projected[j].0);
                    adjacency
                        .entry(projected[j].0)
                        .or_default()
                        .push(projected[i].0);
                }
            }
        }
        let mut visited = HashSet::<usize>::new();
        for start in face_indices {
            if !visited.insert(*start) {
                continue;
            }
            let mut stack = vec![*start];
            let mut component = Vec::new();
            while let Some(idx) = stack.pop() {
                component.push(idx);
                if let Some(next) = adjacency.get(&idx) {
                    for other in next {
                        if visited.insert(*other) {
                            stack.push(*other);
                        }
                    }
                }
            }
            component.sort_unstable();
            let Some(new_faces) = col_component_replacement_faces(mesh, &component) else {
                continue;
            };
            if new_faces.len() >= component.len() {
                continue;
            }
            let anchor = component[0];
            replacements.insert(anchor, new_faces);
            for idx in component {
                remove.insert(idx);
            }
        }
    }
    if replacements.is_empty() {
        return 0;
    }
    let old_len = mesh.faces.len();
    let mut faces = Vec::with_capacity(mesh.faces.len());
    for (idx, face) in mesh.faces.iter().enumerate() {
        if let Some(replacement) = replacements.get(&idx) {
            faces.extend(replacement.iter().cloned());
        } else if !remove.contains(&idx) {
            faces.push(face.clone());
        }
    }
    mesh.faces = faces;
    old_len.saturating_sub(mesh.faces.len())
}

fn simplify_selected_col_faces(mesh: &mut CollisionMesh, selected: &BTreeSet<usize>) -> usize {
    let component = selected
        .iter()
        .copied()
        .filter(|idx| *idx < mesh.faces.len())
        .collect::<Vec<_>>();
    if component.len() <= 2 {
        return 0;
    }
    let Some(first_face) = mesh.faces.get(component[0]) else {
        return 0;
    };
    let Some((first_key, _)) = col_face_plane_key(mesh, first_face) else {
        return 0;
    };
    if component.iter().any(|idx| {
        mesh.faces
            .get(*idx)
            .and_then(|face| col_face_plane_key(mesh, face).map(|(key, _)| key))
            != Some(first_key)
    }) {
        return 0;
    }
    let Some(new_faces) = col_component_replacement_faces(mesh, &component) else {
        return 0;
    };
    if new_faces.len() >= component.len() {
        return 0;
    }
    let old_len = mesh.faces.len();
    let selected = component.into_iter().collect::<BTreeSet<_>>();
    let mut faces = Vec::with_capacity(mesh.faces.len() - selected.len() + new_faces.len());
    let mut inserted = false;
    for (idx, face) in mesh.faces.iter().enumerate() {
        if selected.contains(&idx) {
            if !inserted {
                faces.extend(new_faces.iter().cloned());
                inserted = true;
            }
        } else {
            faces.push(face.clone());
        }
    }
    mesh.faces = faces;
    old_len.saturating_sub(mesh.faces.len())
}

pub(crate) fn editing_cleanup_col_mesh(app: &mut AppState) {
    let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() else {
        return;
    };
    if col_regular_topology_has_editable_primitives(col) {
        app.status_message =
            "Cleanup is disabled while editable capsules or rotated boxes are present".to_string();
        return;
    }
    let original_vertices = col.mesh.vertices.len();
    let original_faces = col.mesh.faces.len();
    let welded = merge_col_vertices_by_distance(&mut col.mesh, MERGE_BY_DISTANCE_DEFAULT);
    let mut total_simplified = 0usize;
    let selected = col_selected_face_set(col);
    if selected.len() > 2 {
        total_simplified += simplify_selected_col_faces(&mut col.mesh, &selected);
    }
    let mut passes = 0usize;
    loop {
        let before = col.mesh.faces.len();
        let simplified = simplify_col_coplanar_faces(&mut col.mesh);
        total_simplified += simplified;
        passes += 1;
        if simplified == 0 || col.mesh.faces.len() == before || passes >= 8 {
            break;
        }
    }
    compact_col_mesh_vertices(&mut col.mesh);
    refresh_editing_col_bounds(col);
    col.selected_face = col
        .selected_face
        .min(col.mesh.faces.len().saturating_sub(1));
    col.selected_faces = col
        .selected_faces
        .iter()
        .copied()
        .filter(|idx| *idx < col.mesh.faces.len())
        .collect();
    if col.selected_faces.is_empty() && col.selected_face < col.mesh.faces.len() {
        col.selected_faces.insert(col.selected_face);
    }
    col.selected_edges.clear();
    col.selected_vertices.clear();
    col.face_scroll = col
        .face_scroll
        .min(col.mesh.faces.len().saturating_sub(1) as f32);
    col.dirty = true;
    app.status_message = format!(
        "Cleaned COL planes: vertices {} -> {}, faces {} -> {} (welded {}, removed {} coplanar face(s))",
        original_vertices,
        col.mesh.vertices.len(),
        original_faces,
        col.mesh.faces.len(),
        welded,
        total_simplified
    );
}

pub(crate) fn editing_optimize_col_mesh(app: &mut AppState) {
    let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() else {
        return;
    };
    if col_regular_topology_has_editable_primitives(col) {
        app.status_message =
            "Optimize is disabled while editable capsules or rotated boxes are present".to_string();
        return;
    }
    let original_vertices = col.mesh.vertices.len();
    let original_faces = col.mesh.faces.len();
    let mut remap = HashMap::<[i32; 3], u16>::new();
    let mut vertices = Vec::<V3>::new();
    let mut face_remap = HashMap::<u16, u16>::new();
    for (idx, vertex) in col.mesh.vertices.iter().enumerate() {
        let key = [
            (vertex.x * 128.0).round() as i32,
            (vertex.y * 128.0).round() as i32,
            (vertex.z * 128.0).round() as i32,
        ];
        let mapped = *remap.entry(key).or_insert_with(|| {
            let next = vertices.len().min(u16::MAX as usize) as u16;
            vertices.push(*vertex);
            next
        });
        face_remap.insert(idx.min(u16::MAX as usize) as u16, mapped);
    }
    let mut faces = Vec::with_capacity(col.mesh.faces.len());
    for face in &col.mesh.faces {
        let Some(a) = face_remap.get(&face.a).copied() else {
            continue;
        };
        let Some(b) = face_remap.get(&face.b).copied() else {
            continue;
        };
        let Some(c) = face_remap.get(&face.c).copied() else {
            continue;
        };
        if a == b || b == c || c == a {
            continue;
        }
        faces.push(CollisionFace {
            a,
            b,
            c,
            material: face.material,
            light: face.light,
            img_path: face.img_path.clone(),
            material_file_offset: face.material_file_offset,
            light_file_offset: face.light_file_offset,
        });
    }
    col.mesh.vertices = vertices;
    col.mesh.faces = faces;
    let simplified_faces = simplify_col_coplanar_faces(&mut col.mesh);
    compact_col_mesh_vertices(&mut col.mesh);
    refresh_editing_col_bounds(col);
    col.selected_face = col
        .selected_face
        .min(col.mesh.faces.len().saturating_sub(1));
    col.selected_faces = col
        .selected_faces
        .iter()
        .copied()
        .filter(|idx| *idx < col.mesh.faces.len())
        .collect();
    if col.selected_faces.is_empty() && col.selected_face < col.mesh.faces.len() {
        col.selected_faces.insert(col.selected_face);
    }
    col.face_scroll = col
        .face_scroll
        .min(col.mesh.faces.len().saturating_sub(1) as f32);
    col.dirty = true;
    app.status_message = format!(
        "Optimized COL: vertices {} -> {}, faces {} -> {} (merged {} coplanar face(s))",
        original_vertices,
        col.mesh.vertices.len(),
        original_faces,
        col.mesh.faces.len(),
        simplified_faces
    );
}

fn current_editing_col_bytes(col: &EditingColState) -> Result<Vec<u8>, String> {
    let mut mesh = normalized_editing_col_mesh(col);
    let mut capsules = valid_capsules_for_mesh(&mut mesh, col.capsules.clone());
    if capsules.len() != col.capsules.len() {
        return Err("capsule metadata no longer matches generated geometry".to_string());
    }
    let mut cuboids = valid_cuboids_for_mesh(&mut mesh, col.cuboids.clone());
    if cuboids.len() != col.cuboids.len() {
        return Err("rotated-box metadata no longer matches generated geometry".to_string());
    }
    sync_generated_primitive_face_ranges(&mesh, &mut capsules, &mut cuboids)?;
    if col.editing_shadow && !mesh.shadow_faces.is_empty() {
        snap_shadow_vertices_to_col_grid(&mut mesh.shadow_vertices)?;
        orient_closed_shadow_components(&mesh.shadow_vertices, &mut mesh.shadow_faces)?;
    }
    write_col_mesh_replacing_model(&col.bytes, &col.source_model, &mesh)
}

pub(crate) fn editing_validate_col_mesh(app: &mut AppState) {
    let Some(EditingAsset::Col(col)) = app.editing.asset.as_ref() else {
        return;
    };
    let mesh = normalized_editing_col_mesh(col);
    let mut invalid = 0usize;
    let mut degenerate = 0usize;
    let mut used = HashSet::<u16>::new();
    for face in &mesh.faces {
        let max = mesh.vertices.len();
        if face.a as usize >= max || face.b as usize >= max || face.c as usize >= max {
            invalid += 1;
            continue;
        }
        if face.a == face.b || face.b == face.c || face.c == face.a {
            degenerate += 1;
        }
        used.insert(face.a);
        used.insert(face.b);
        used.insert(face.c);
    }
    let orphan = mesh.vertices.len().saturating_sub(used.len());
    let bytes = match current_editing_col_bytes(col) {
        Ok(bytes) => bytes,
        Err(error) => {
            app.status_message = format!(
                "COL issues: current edited mesh cannot be staged: {error}; {invalid} invalid face(s), {degenerate} degenerate face(s), {orphan} orphan vertex/vertices"
            );
            return;
        }
    };
    let entry = ImgEntry {
        img_path: PathBuf::from(&col.name),
        name: col.name.clone(),
        offset: 0,
        size: bytes.len().min(u32::MAX as usize) as u32,
    };
    if parse_col_mesh(&bytes, &entry).is_none() {
        app.status_message =
            "COL issues: current edited mesh serialized but failed parser validation".to_string();
        return;
    }
    let load_issues = validate_col_for_game_load(&col.name, &bytes);
    let load_errors = load_issues
        .iter()
        .filter(|issue| issue.severity == ColLoadIssueSeverity::Error)
        .count();
    let load_warnings = load_issues.len().saturating_sub(load_errors);
    app.status_message = if invalid == 0 && degenerate == 0 && load_errors == 0 {
        format!(
            "COL valid: {} face(s), {} orphan vertex/vertices, {load_warnings} game-load warning(s)",
            mesh.faces.len(),
            orphan
        )
    } else {
        let first_load = load_issues
            .first()
            .map(|issue| format!("; {}", issue.message))
            .unwrap_or_default();
        format!(
            "COL issues: {invalid} invalid face(s), {degenerate} degenerate face(s), {orphan} orphan vertex/vertices, {load_errors} game-load error(s), {load_warnings} warning(s){first_load}"
        )
    };
}

pub(crate) fn editing_stage_modified_entry(app: &mut AppState, name: &str, bytes: Vec<u8>) {
    app.editing
        .modified_entries
        .insert(editing_key(name), bytes);
    if let Some(row) = app
        .editing
        .rows
        .iter_mut()
        .find(|row| row.entry.name.eq_ignore_ascii_case(name))
    {
        row.logical_size = app
            .editing
            .modified_entries
            .get(&editing_key(name))
            .map(|bytes| replacement_entry_len(name, bytes))
            .unwrap_or(row.logical_size);
    }
    if let Some(bytes) = app
        .editing
        .modified_entries
        .get(&editing_key(name))
        .cloned()
    {
        refresh_live_asset_from_editing_entry(app, name, &bytes);
    }
}

fn editing_stage_added_entry(
    editing: &mut EditingState,
    fallback_img_path: PathBuf,
    name: &str,
    bytes: Vec<u8>,
) {
    let key = editing_key(name);
    let logical_size = replacement_entry_len(name, &bytes);
    let byte_size = bytes.len().min(u32::MAX as usize) as u32;
    editing.modified_entries.insert(key.clone(), bytes);
    editing.deleted_entries.remove(&key);
    if let Some(row) = editing
        .rows
        .iter_mut()
        .find(|row| editing_key(&row.entry.name) == key)
    {
        row.logical_size = logical_size;
        return;
    }
    let img_path = editing
        .img_path
        .clone()
        .or_else(|| editing.rows.first().map(|row| row.entry.img_path.clone()))
        .unwrap_or(fallback_img_path);
    editing.rows.push(EditingImgRow {
        entry: ImgEntry {
            img_path,
            name: name.to_string(),
            offset: 0,
            size: byte_size,
        },
        logical_size,
    });
    editing.added_entries.insert(key);
}

fn refresh_live_asset_from_editing_entry(app: &mut AppState, name: &str, bytes: &[u8]) {
    let key = lower(name.trim());
    if key.ends_with(".col") {
        let entry = ImgEntry {
            img_path: app
                .editing
                .img_path
                .clone()
                .unwrap_or_else(|| PathBuf::from(name)),
            name: with_ext(name, ".col"),
            offset: 0,
            size: bytes.len().min(u32::MAX as usize) as u32,
        };
        if let Some(mesh) = parse_col_mesh(bytes, &entry) {
            app.collisions.insert(lower(&entry.name), mesh);
        } else if !col_validation_has_errors(&validate_col_for_game_load(&entry.name, bytes)) {
            app.collisions.remove(&lower(&entry.name));
        }
        return;
    }
    if !key.ends_with(".dff") {
        return;
    }
    let raw = parse_dff_mesh(bytes);
    if raw.vertices.is_empty() {
        return;
    }
    let texture_files = collect_texture_files(&app.root);
    if refresh_live_dff_from_raw(app, name, &raw, &texture_files) {
        rebuild_render_cells(app);
    }
}

pub(crate) fn refresh_live_dff_from_raw(
    app: &mut AppState,
    name: &str,
    raw: &RawMesh,
    texture_files: &HashMap<String, PathBuf>,
) -> bool {
    let dff_key = asset_key(name, ".dff");
    let mut txd_scopes = BTreeSet::<Option<String>>::new();
    for placement in &app.placements {
        if asset_key(&placement.dff, ".dff") == dff_key {
            txd_scopes
                .insert(definition_txd_name(&app.definitions, &placement.id).map(str::to_owned));
        }
    }
    if txd_scopes.is_empty() {
        txd_scopes.insert(None);
    }
    let mut refreshed = false;
    for txd_scope in txd_scopes {
        refreshed |=
            refresh_live_dff_scope_from_raw(app, name, raw, texture_files, txd_scope.as_deref());
    }
    refreshed
}

pub(crate) fn refresh_live_dff_scope_from_raw(
    app: &mut AppState,
    name: &str,
    raw: &RawMesh,
    texture_files: &HashMap<String, PathBuf>,
    txd_scope: Option<&str>,
) -> bool {
    let ambient_lift = scene_ambient_lift_from_timecyc(&app.timecyc);
    let dff_key = asset_key(name, ".dff");
    let mesh_key = mesh_key_from_dff_txd(&dff_key, txd_scope);
    if let Some(mut mesh) = compile_render_mesh(
        raw.clone(),
        txd_scope,
        None,
        None,
        texture_files,
        &app.txd_textures,
        &mut app.textures,
        &mut app.textured_parts,
        app.options.textures,
        ambient_lift,
    ) {
        // Compiled DFFs initialize their display color from the day prelight
        // stream. Preserve the user's current Bake-tab preview mode.
        apply_bake_light_mode_to_mesh(&mut mesh, app.bake_settings.light_mode);
        for part in &mut mesh.parts {
            rebuild_render_part_list_with_lift(part, ambient_lift);
        }
        replace_render_mesh(&mut app.meshes, mesh_key, mesh);
        true
    } else {
        false
    }
}

pub(crate) fn refresh_loaded_dff_scope_from_raw(
    app: &mut AppState,
    name: &str,
    raw: &RawMesh,
    txd_scope: Option<&str>,
) -> bool {
    let ambient_lift = scene_ambient_lift_from_timecyc(&app.timecyc);
    let mesh_key = mesh_key_from_dff_txd(&asset_key(name, ".dff"), txd_scope);
    let Some(mut mesh) = compile_render_mesh(
        raw.clone(),
        txd_scope,
        None,
        None,
        &app.texture_files,
        &app.txd_textures,
        &mut app.textures,
        &mut app.textured_parts,
        app.options.textures,
        ambient_lift,
    ) else {
        return false;
    };
    apply_bake_light_mode_to_mesh(&mut mesh, app.bake_settings.light_mode);
    for part in &mut mesh.parts {
        rebuild_render_part_list_with_lift(part, ambient_lift);
    }
    replace_render_mesh(&mut app.meshes, mesh_key, mesh);
    true
}

pub(crate) fn open_editing_img_picker(app: &mut AppState) {
    drain_text_input();
    if app.dff_picker_rx.is_some() {
        app.status_message = "File browser is already open".to_string();
        return;
    }
    let start_dir = app.root.join("imgs");
    let (tx, rx) = mpsc::channel();
    app.dff_picker_rx = Some(rx);
    thread::spawn(move || {
        let _ = tx.send((
            DffPickerKind::EditingOpenFile,
            choose_editing_open_path(start_dir),
        ));
    });
}

pub(crate) fn open_editing_merge_img_picker(app: &mut AppState) {
    if app.editing.img_path.is_none() {
        app.status_message = "Open the destination IMG before merging another archive".to_string();
        return;
    }
    drain_text_input();
    if app.dff_picker_rx.is_some()
        || app.editing.merge_rx.is_some()
        || app.editing.merge_apply_job.is_some()
        || app.editing.save_rx.is_some()
    {
        app.status_message = "An IMG file operation is already running".to_string();
        return;
    }
    let start_dir = app.root.join("imgs");
    let (tx, rx) = mpsc::channel();
    app.dff_picker_rx = Some(rx);
    app.status_message = "Choose an external IMG to merge...".to_string();
    thread::spawn(move || {
        let _ = tx.send((
            DffPickerKind::EditingMergeImg,
            choose_editing_merge_img_path(start_dir),
        ));
    });
}

fn load_editing_img_merge_plan(
    source_path: PathBuf,
    existing_keys: BTreeSet<String>,
) -> Result<EditingImgMergePlan, String> {
    let mut file = fs::File::open(&source_path)
        .map_err(|err| format!("Could not open {}: {err}", source_path.display()))?;
    let mut header = [0u8; 8];
    if file.read_exact(&mut header).is_err() || &header[..4] != b"VER2" {
        return Err(format!(
            "{} is not a supported VER2 IMG archive",
            source_path.display()
        ));
    }
    let parsed = parse_img(&source_path);
    if parsed.is_empty() {
        return Err(format!(
            "{} contains no readable IMG entries",
            source_path.display()
        ));
    }
    let mut seen = BTreeSet::new();
    let mut entries = Vec::with_capacity(parsed.len());
    let mut matching_entries = 0usize;
    for entry in parsed {
        let key = editing_key(&entry.name);
        if !seen.insert(key.clone()) {
            return Err(format!(
                "{} contains the duplicate entry name {}",
                source_path.display(),
                entry.name
            ));
        }
        let mut bytes = read_img_entry_from(&mut file, &entry);
        if bytes.len() != entry.size as usize {
            return Err(format!(
                "Could not read source IMG entry {} completely: expected {} bytes, got {}",
                entry.name,
                entry.size,
                bytes.len()
            ));
        }
        bytes.truncate(replacement_entry_len(&entry.name, &bytes));
        let matches_existing = existing_keys.contains(&key);
        matching_entries += usize::from(matches_existing);
        entries.push(EditingImgMergeEntry {
            name: entry.name,
            bytes,
            matches_existing,
        });
    }
    Ok(EditingImgMergePlan {
        source_path,
        entries,
        matching_entries,
    })
}

pub(crate) fn start_editing_img_merge_scan(app: &mut AppState, source_path: PathBuf) {
    let Some(target_path) = app.editing.img_path.as_ref() else {
        app.status_message = "Open the destination IMG before merging another archive".to_string();
        return;
    };
    if same_resource_path(target_path, &source_path) {
        app.status_message =
            "Choose a different IMG; an archive cannot be merged into itself".to_string();
        return;
    }
    if app.editing.merge_rx.is_some()
        || app.editing.merge_apply_job.is_some()
        || app.editing.save_rx.is_some()
    {
        app.status_message = "An IMG file operation is already running".to_string();
        return;
    }
    let existing_keys = app
        .editing
        .rows
        .iter()
        .map(|row| editing_key(&row.entry.name))
        .filter(|key| !app.editing.deleted_entries.contains(key))
        .collect::<BTreeSet<_>>();
    let (tx, rx) = mpsc::channel();
    app.editing.merge_rx = Some(rx);
    app.status_message = format!("Reading {} in the background...", source_path.display());
    thread::spawn(move || {
        let result = load_editing_img_merge_plan(source_path, existing_keys);
        let _ = tx.send(result);
    });
}

pub(crate) fn start_editing_img_merge_apply(
    app: &mut AppState,
    plan: Arc<EditingImgMergePlan>,
    overwrite_matching: bool,
) {
    let source_path = plan.source_path.clone();
    let total_entries = plan.entries.len();
    let entries = match Arc::try_unwrap(plan) {
        Ok(plan) => plan.entries.into(),
        Err(plan) => plan.entries.iter().cloned().collect(),
    };
    app.editing.merge_apply_job = Some(EditingImgMergeApplyJob {
        source_path,
        entries,
        overwrite_matching,
        total_entries,
        added: 0,
        replaced: 0,
        skipped: 0,
    });
    app.status_message = format!("Merging {total_entries} IMG entries...");
}

pub(crate) fn update_editing_img_merge(app: &mut AppState) {
    if let Some(rx) = app.editing.merge_rx.as_ref() {
        let result = match rx.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Err("IMG merge worker stopped unexpectedly".to_string()))
            }
        };
        if let Some(result) = result {
            app.editing.merge_rx = None;
            match result {
                Ok(plan) if plan.matching_entries > 0 => {
                    let matching = plan.matching_entries;
                    let new_entries = plan.entries.len().saturating_sub(matching);
                    let source_name = plan
                        .source_path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("the imported IMG")
                        .to_string();
                    let plan = Arc::new(plan);
                    app.confirm_dialog = Some(ConfirmDialog {
                        action: ConfirmAction::ApplyEditingImgMerge {
                            plan: Arc::clone(&plan),
                            overwrite_matching: true,
                        },
                        title: "Matching IMG Entries".to_string(),
                        body: format!(
                            "{source_name} has {matching} matching entr{} and {new_entries} new entr{}. Overwrite the matching items?",
                            if matching == 1 { "y" } else { "ies" },
                            if new_entries == 1 { "y" } else { "ies" },
                        ),
                        detail: "Overwrite Matches replaces same-name entries case-insensitively, including staged changes. Keep Existing imports only new names. Nothing is written until you use Write IMG.".to_string(),
                        primary_label: "Overwrite Matches".to_string(),
                        secondary_label: Some("Keep Existing".to_string()),
                        secondary_action: Some(ConfirmAction::ApplyEditingImgMerge {
                            plan,
                            overwrite_matching: false,
                        }),
                    });
                    app.status_message = "Choose how to handle matching IMG entries".to_string();
                }
                Ok(plan) => start_editing_img_merge_apply(app, Arc::new(plan), false),
                Err(err) => {
                    app.editing.message = format!("IMG merge failed: {err}");
                    app.status_message = app.editing.message.clone();
                }
            }
        }
    }

    let Some(mut job) = app.editing.merge_apply_job.take() else {
        return;
    };
    let frame_started = Instant::now();
    let mut applied = 0usize;
    while applied < LIVE_RESOURCE_REFRESH_BATCH_LIMIT
        && (applied == 0 || frame_started.elapsed() < LIVE_RESOURCE_REFRESH_FRAME_BUDGET)
    {
        let Some(entry) = job.entries.pop_front() else {
            break;
        };
        applied += 1;
        if entry.matches_existing && !job.overwrite_matching {
            job.skipped += 1;
            continue;
        }
        if entry.matches_existing {
            let active_matches = app.editing.asset.as_ref().is_some_and(|asset| match asset {
                EditingAsset::Txd(txd) => txd.name.eq_ignore_ascii_case(&entry.name),
                EditingAsset::Dff(dff) => dff.name.eq_ignore_ascii_case(&entry.name),
                EditingAsset::Col(col) => col.name.eq_ignore_ascii_case(&entry.name),
            });
            if active_matches {
                app.editing.asset = None;
            }
            job.replaced += 1;
        } else {
            job.added += 1;
        }
        editing_stage_added_entry(
            &mut app.editing,
            job.source_path.clone(),
            &entry.name,
            entry.bytes,
        );
    }
    let processed = job
        .added
        .saturating_add(job.replaced)
        .saturating_add(job.skipped);
    if job.entries.is_empty() {
        let source_name = job
            .source_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("IMG");
        app.editing.message = format!(
            "Merged {source_name}: added {}, replaced {}, skipped {}. Use Write IMG to save.",
            job.added, job.replaced, job.skipped
        );
        app.status_message = app.editing.message.clone();
        invalidate_validation_cache(app);
    } else {
        app.status_message = format!("Merging IMG entries: {processed}/{}...", job.total_entries);
        app.editing.merge_apply_job = Some(job);
    }
}

pub(crate) fn open_editing_add_entry_picker(app: &mut AppState) {
    drain_text_input();
    if app.dff_picker_rx.is_some() {
        app.status_message = "File browser is already open".to_string();
        return;
    }
    let start_dir = app.root.join("imgs");
    let (tx, rx) = mpsc::channel();
    app.dff_picker_rx = Some(rx);
    thread::spawn(move || {
        let _ = tx.send((
            DffPickerKind::EditingAddEntry,
            choose_editing_asset_path(start_dir),
        ));
    });
}

pub(crate) fn open_editing_replace_entry_picker(app: &mut AppState) {
    let Some(row) = editing_selected_row(app) else {
        app.status_message = "Select an IMG entry before replacing".to_string();
        return;
    };
    drain_text_input();
    if app.dff_picker_rx.is_some() {
        app.status_message = "File browser is already open".to_string();
        return;
    }
    let entry_name = row.entry.name.clone();
    let start_dir = app.root.join("imgs");
    let (tx, rx) = mpsc::channel();
    app.dff_picker_rx = Some(rx);
    thread::spawn(move || {
        let _ = tx.send((
            DffPickerKind::EditingReplaceEntry { entry_name },
            choose_editing_asset_path(start_dir),
        ));
    });
}

pub(crate) fn open_editing_extract_entry_picker(app: &mut AppState) {
    let Some(row) = editing_selected_row(app) else {
        app.status_message = "Select an IMG entry before extracting".to_string();
        return;
    };
    drain_text_input();
    if app.dff_picker_rx.is_some() {
        app.status_message = "File browser is already open".to_string();
        return;
    }
    let entry_name = row.entry.name.clone();
    let default_path = app.root.join(&entry_name);
    let (tx, rx) = mpsc::channel();
    app.dff_picker_rx = Some(rx);
    thread::spawn(move || {
        let _ = tx.send((
            DffPickerKind::EditingExtractEntry { entry_name },
            choose_editing_extract_path(default_path),
        ));
    });
}

pub(crate) fn open_editing_txd_texture_picker(app: &mut AppState, replace: bool) {
    let Some(EditingAsset::Txd(txd)) = app.editing.asset.as_ref() else {
        return;
    };
    let entry_name = txd.name.clone();
    let texture_name = txd
        .textures
        .get(txd.selected)
        .map(|entry| entry.name.clone());
    if replace && texture_name.is_none() {
        app.status_message = "Select a texture before replacing".to_string();
        return;
    }
    drain_text_input();
    if app.dff_picker_rx.is_some() {
        app.status_message = "File browser is already open".to_string();
        return;
    }
    let start_dir = app.root.join("txd_build");
    let kind = if replace {
        DffPickerKind::EditingTextureReplace {
            entry_name,
            texture_name: texture_name.unwrap(),
        }
    } else {
        DffPickerKind::EditingTextureAdd { entry_name }
    };
    let (tx, rx) = mpsc::channel();
    app.dff_picker_rx = Some(rx);
    thread::spawn(move || {
        let _ = tx.send((kind, choose_texture_image_path(start_dir)));
    });
}

pub(crate) fn open_editing_txd_export_all_picker(app: &mut AppState) {
    let Some(EditingAsset::Txd(txd)) = app.editing.asset.as_ref() else {
        return;
    };
    drain_text_input();
    if app.dff_picker_rx.is_some() {
        app.status_message = "File browser is already open".to_string();
        return;
    }
    let entry_name = txd.name.clone();
    let start_dir = app.root.join("txd_export");
    let (tx, rx) = mpsc::channel();
    app.dff_picker_rx = Some(rx);
    thread::spawn(move || {
        let _ = tx.send((
            DffPickerKind::EditingTextureExportAll { entry_name },
            choose_editing_export_textures_dir(start_dir),
        ));
    });
}

pub(crate) fn open_dff_face_texture_picker(app: &mut AppState) {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return;
    };
    if dff.txd_context.is_none() {
        app.status_message = "This model has no resolved TXD to pick textures from".to_string();
        return;
    }
    if dff_selected_face_set(dff).is_empty() {
        app.status_message = "Select one or more DFF faces first".to_string();
        return;
    }
    dff.texture_picker_edits_material = false;
    dff.texture_picker_open = true;
    dff.texture_picker_scroll = 0.0;
}

pub(crate) fn open_dff_material_texture_picker(app: &mut AppState) {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
        return;
    };
    if dff.txd_context.is_none() {
        app.status_message = "This model has no resolved TXD to pick textures from".to_string();
        return;
    }
    if dff.selected_material >= dff_material_slot_count(&dff.raw) {
        app.status_message = "Select a valid DFF material first".to_string();
        return;
    }
    dff.texture_picker_edits_material = true;
    dff.texture_picker_open = true;
    dff.texture_picker_scroll = 0.0;
}

pub(crate) fn start_dff_face_texture_browse(app: &mut AppState) {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_ref() else {
        return;
    };
    let Some(txd_name) = dff.txd_context.clone() else {
        app.status_message = "This model has no resolved TXD to import textures into".to_string();
        return;
    };
    if dff_selected_face_set(dff).is_empty() {
        app.status_message = "Select one or more DFF faces first".to_string();
        return;
    }
    drain_text_input();
    if app.dff_picker_rx.is_some() {
        app.status_message = "File browser is already open".to_string();
        return;
    }
    let start_dir = app.root.join("txd_build");
    let kind = DffPickerKind::EditingFaceTextureImport { txd_name };
    let (tx, rx) = mpsc::channel();
    app.dff_picker_rx = Some(rx);
    thread::spawn(move || {
        let _ = tx.send((kind, choose_texture_image_path(start_dir)));
    });
}

pub(crate) fn start_dff_material_texture_browse(app: &mut AppState) {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_ref() else {
        return;
    };
    let Some(txd_name) = dff.txd_context.clone() else {
        app.status_message = "This model has no resolved TXD to import textures into".to_string();
        return;
    };
    let material = dff.selected_material;
    if material >= dff_material_slot_count(&dff.raw) {
        app.status_message = "Select a valid DFF material first".to_string();
        return;
    }
    let dff_name = dff.name.clone();
    drain_text_input();
    if app.dff_picker_rx.is_some() {
        app.status_message = "File browser is already open".to_string();
        return;
    }
    let start_dir = app.root.join("txd_build");
    let kind = DffPickerKind::EditingMaterialTextureImport {
        txd_name,
        dff_name,
        material,
    };
    let (tx, rx) = mpsc::channel();
    app.dff_picker_rx = Some(rx);
    thread::spawn(move || {
        let _ = tx.send((kind, choose_texture_image_path(start_dir)));
    });
}

pub(crate) fn editing_add_entry_from_path(app: &mut AppState, path: PathBuf) {
    if app.editing.img_path.is_none() {
        app.status_message = "Open an IMG archive before adding entries".to_string();
        return;
    }
    let Some(name) = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
    else {
        app.status_message = "Selected file has no usable name".to_string();
        return;
    };
    let Ok(mut bytes) = fs::read(&path) else {
        app.status_message = format!("Could not read {}", path.display());
        return;
    };
    if name.to_ascii_lowercase().ends_with(".col") {
        set_col_model_names_from_entry(&mut bytes, &name);
    }
    if app
        .editing
        .rows
        .iter()
        .any(|row| row.entry.name.eq_ignore_ascii_case(&name))
    {
        app.status_message = format!("{name} already exists; use Replace instead");
        return;
    }
    let Some(img_path) = app.editing.img_path.clone() else {
        return;
    };
    let logical_size = replacement_entry_len(&name, &bytes);
    let entry = ImgEntry {
        img_path,
        name: name.clone(),
        offset: 0,
        size: bytes.len().min(u32::MAX as usize) as u32,
    };
    app.editing.rows.push(EditingImgRow {
        entry,
        logical_size,
    });
    app.editing
        .modified_entries
        .insert(editing_key(&name), bytes);
    app.editing.added_entries.insert(editing_key(&name));
    app.editing.message = format!("Staged add for {name}");
    app.status_message = app.editing.message.clone();
}

pub(crate) fn editing_replace_entry_from_path(
    app: &mut AppState,
    entry_name: String,
    path: PathBuf,
) {
    let Ok(mut bytes) = fs::read(&path) else {
        app.status_message = format!("Could not read {}", path.display());
        return;
    };
    if entry_name.to_ascii_lowercase().ends_with(".col") {
        set_col_model_names_from_entry(&mut bytes, &entry_name);
    }
    editing_stage_modified_entry(app, &entry_name, bytes);
    app.editing
        .deleted_entries
        .remove(&editing_key(&entry_name));
    app.editing.message = format!("Staged replacement for {entry_name}");
    app.status_message = app.editing.message.clone();
    if app.editing.asset.as_ref().is_some_and(|asset| match asset {
        EditingAsset::Txd(txd) => txd.name.eq_ignore_ascii_case(&entry_name),
        EditingAsset::Dff(dff) => dff.name.eq_ignore_ascii_case(&entry_name),
        EditingAsset::Col(col) => col.name.eq_ignore_ascii_case(&entry_name),
    }) {
        editing_open_selected_asset(app);
    }
}

pub(crate) fn editing_extract_entry_to_path(app: &mut AppState, entry_name: String, path: PathBuf) {
    let Some(row) = app
        .editing
        .rows
        .iter()
        .find(|row| row.entry.name.eq_ignore_ascii_case(&entry_name))
        .cloned()
    else {
        app.status_message = format!("{entry_name} is no longer in the open IMG");
        return;
    };
    let bytes = match editing_entry_bytes(app, &row) {
        Ok(bytes) => bytes,
        Err(err) => {
            app.status_message = err;
            return;
        }
    };
    if let Some(parent) = path.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            app.status_message = format!("Could not create {}: {err}", parent.display());
            return;
        }
    }
    match fs::write(&path, bytes) {
        Ok(()) => app.status_message = format!("Extracted {entry_name} to {}", path.display()),
        Err(err) => app.status_message = format!("Could not extract {entry_name}: {err}"),
    }
}

pub(crate) fn editing_export_all_txd_textures(
    app: &mut AppState,
    entry_name: String,
    dir: PathBuf,
) {
    let Some(row) = app
        .editing
        .rows
        .iter()
        .find(|row| row.entry.name.eq_ignore_ascii_case(&entry_name))
        .cloned()
    else {
        app.status_message = format!("{entry_name} is no longer in the open IMG");
        return;
    };
    let bytes = match editing_entry_bytes(app, &row) {
        Ok(bytes) => bytes,
        Err(err) => {
            app.status_message = err;
            return;
        }
    };
    if let Err(err) = fs::create_dir_all(&dir) {
        app.status_message = format!("Could not create {}: {err}", dir.display());
        return;
    }

    let txd_name = asset_key(&entry_name, ".txd");
    let mut index = TxdTextureIndex::new();
    index_one_txd(
        &bytes,
        0,
        bytes.len(),
        Path::new("<editing>"),
        &txd_name,
        &mut index,
    );
    let mut exported = 0usize;
    let mut failed = 0usize;
    let mut used_names = HashSet::new();
    for (texture_name, textures) in &index {
        for texture in textures {
            if !texture.txd_name.eq_ignore_ascii_case(&txd_name) {
                continue;
            }
            let Some((width, height, rgba)) = decode_txd_texture_from_bytes(texture, &bytes) else {
                failed += 1;
                continue;
            };
            let path = unique_export_texture_path(&dir, texture_name, &mut used_names);
            match image::save_buffer(&path, &rgba, width, height, image::ColorType::Rgba8) {
                Ok(()) => exported += 1,
                Err(_) => failed += 1,
            }
        }
    }
    app.status_message = if failed == 0 {
        format!(
            "Exported {exported} texture(s) from {entry_name} to {}",
            dir.display()
        )
    } else {
        format!("Exported {exported} texture(s) from {entry_name}; {failed} failed")
    };
}

pub(crate) fn unique_export_texture_path(
    dir: &Path,
    texture_name: &str,
    used_names: &mut HashSet<String>,
) -> PathBuf {
    let file_name = export_texture_png_name(texture_name);
    let stem = Path::new(&file_name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("texture");
    let ext = Path::new(&file_name)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("PNG");
    let mut index = 0usize;
    loop {
        let candidate = if index == 0 {
            file_name.clone()
        } else {
            format!("{stem}_{index}.{ext}")
        };
        if used_names.insert(candidate.to_ascii_lowercase()) {
            return dir.join(candidate);
        }
        index += 1;
    }
}

pub(crate) fn editing_import_texture_from_path(
    app: &mut AppState,
    entry_name: String,
    texture_name: Option<String>,
    path: PathBuf,
) {
    let target_texture = texture_name.unwrap_or_else(|| {
        path.file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("texture")
            .to_string()
    });
    if app.editing.txd_import_rx.is_some() || app.editing.txd_refresh_job.is_some() {
        app.status_message = "Wait for the current TXD import to finish".to_string();
        return;
    }
    if app.manual_save_job.is_some()
        || app.editing.save_rx.is_some()
        || app.autosave_rx.is_some()
        || app.autosave_cleanup_rx.is_some()
        || app.txd_cleanup_job.is_some()
        || app.asset_optimization_scan_rx.is_some()
        || app.asset_optimization_job.is_some()
    {
        app.status_message =
            "Texture import cannot start while another asset writer is running".to_string();
        return;
    }
    let Some(row) = app
        .editing
        .rows
        .iter()
        .find(|row| row.entry.name.eq_ignore_ascii_case(&entry_name))
        .cloned()
    else {
        app.status_message = format!("{entry_name} is no longer in the open IMG");
        return;
    };
    let bytes = match editing_entry_bytes(app, &row) {
        Ok(bytes) => bytes,
        Err(err) => {
            app.status_message = err;
            return;
        }
    };
    let txd_key = asset_key(&entry_name, ".txd");
    let linked_definition_ids = app
        .definitions
        .iter()
        .filter_map(|(id, definition)| {
            definition_txd_name_from_attrs(definition)
                .is_some_and(|name| asset_key(name, ".txd") == txd_key)
                .then(|| id.clone())
        })
        .collect::<Vec<_>>();
    let root = app.root.clone();
    let worker_entry_name = entry_name.clone();
    let worker_texture_name = target_texture.clone();
    let (tx, rx) = mpsc::channel();
    app.editing.txd_import_rx = Some(rx);
    app.status_message =
        format!("Importing {target_texture} into {entry_name} in the background...");
    thread::spawn(move || {
        let result = (|| {
            let native = imported_image_texture_native(&path, &worker_texture_name)?;
            let updated =
                replace_or_append_texture_native_in_txd(bytes, &native, &worker_texture_name)?;
            let wip_root = wip_root_path(&root);
            upsert_replacement_txd(&wip_root, &worker_entry_name, &updated)?;

            // Index the rewritten overlay archive on the worker. Rewriting one
            // member can move every other member, so all of its TXD offsets must
            // be refreshed together before linked meshes are recompiled.
            let replacement_img = wip_root.join("imgs").join(REPLACEMENT_IMG);
            let replacement_entries = parse_img(&replacement_img);
            let replacement_txd_names = replacement_entries
                .iter()
                .filter(|entry| lower(&entry.name).ends_with(".txd"))
                .map(|entry| asset_key(&entry.name, ".txd"))
                .collect::<HashSet<_>>();
            let mut indexed = TxdTextureIndex::new();
            index_txd_entries(&replacement_img, &replacement_entries, &mut indexed);
            let mut indexed_textures = indexed.into_iter().collect::<Vec<_>>();
            indexed_textures.sort_by(|a, b| a.0.cmp(&b.0));
            Ok(EditingTxdImportOutput {
                updated,
                indexed_textures,
                replacement_txd_names,
            })
        })();
        let _ = tx.send(EditingTxdImportResult {
            entry_name: worker_entry_name,
            target_texture: worker_texture_name,
            linked_definition_ids,
            result,
        });
    });
}

pub(crate) fn update_editing_txd_import(app: &mut AppState) {
    if let Some(rx) = app.editing.txd_import_rx.as_ref() {
        let received = match rx.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                app.editing.txd_import_rx = None;
                app.status_message = "TXD import worker stopped unexpectedly".to_string();
                return;
            }
        };
        if let Some(result) = received {
            app.editing.txd_import_rx = None;
            match result.result {
                Ok(output) => {
                    editing_stage_modified_entry(app, &result.entry_name, output.updated.clone());
                    let textures = txd_entries_from_bytes(&result.entry_name, &output.updated);
                    let selected = textures
                        .iter()
                        .position(|entry| entry.name.eq_ignore_ascii_case(&result.target_texture))
                        .unwrap_or(0);
                    app.editing.asset = Some(EditingAsset::Txd(EditingTxdState {
                        name: result.entry_name.clone(),
                        textures,
                        selected,
                        scroll: selected.saturating_sub(4) as f32,
                        search: String::new(),
                        search_cursor: 0,
                        search_anchor: None,
                        search_active: false,
                        preview_texture: None,
                        material_picker_open: false,
                        material_picker_search: String::new(),
                        material_picker_scroll: 0.0,
                        material_picker_scope: CollisionMaterialAssignmentScope::ExactTxd,
                    }));
                    editing_update_txd_preview(app);
                    let txd_name = asset_key(&result.entry_name, ".txd");
                    app.pending_txd_writes.insert(txd_name.clone());
                    app.loaded_wip = true;
                    app.editing.txd_refresh_job = Some(EditingTxdRefreshJob {
                        txd_name,
                        texture_name: result.target_texture,
                        indexed_textures: output.indexed_textures.into(),
                        replacement_txd_names: output.replacement_txd_names,
                        linked_definition_ids: result.linked_definition_ids.into(),
                        recompiled: 0,
                        index_complete: false,
                    });
                    invalidate_validation_cache(app);
                    app.status_message = format!(
                        "Staged texture in {}; refreshing linked DFFs...",
                        result.entry_name
                    );
                }
                Err(err) => app.status_message = format!("Could not update TXD: {err}"),
            }
        }
    }

    let Some(mut job) = app.editing.txd_refresh_job.take() else {
        return;
    };
    let frame_started = Instant::now();
    let mut applied = 0usize;
    while applied < LIVE_RESOURCE_REFRESH_BATCH_LIMIT
        && frame_started.elapsed() < LIVE_RESOURCE_REFRESH_FRAME_BUDGET
    {
        let Some((texture_name, entries)) = job.indexed_textures.pop_front() else {
            break;
        };
        let indexed = app.txd_textures.entry(texture_name).or_default();
        indexed.retain(|entry| {
            !job.replacement_txd_names
                .contains(&asset_key(&entry.txd_name, ".txd"))
        });
        indexed.extend(entries);
        applied += 1;
    }
    if !job.indexed_textures.is_empty() {
        app.status_message = format!(
            "Refreshing TXD index ({} texture group(s) remaining)...",
            job.indexed_textures.len()
        );
        app.editing.txd_refresh_job = Some(job);
        return;
    }
    if !job.index_complete {
        invalidate_cached_txd_textures(app, &job.txd_name, Some(&job.texture_name));
        job.index_complete = true;
    }

    let mut recompiled_this_frame = 0usize;
    while recompiled_this_frame < LIVE_RESOURCE_REFRESH_BATCH_LIMIT
        && frame_started.elapsed() < LIVE_RESOURCE_REFRESH_FRAME_BUDGET
    {
        let Some(definition_id) = job.linked_definition_ids.pop_front() else {
            break;
        };
        if recompile_definition_mesh(app, &definition_id) {
            job.recompiled += 1;
        }
        recompiled_this_frame += 1;
    }
    if !job.linked_definition_ids.is_empty() {
        app.status_message = format!(
            "Refreshing linked DFFs ({} definition(s) remaining)...",
            job.linked_definition_ids.len()
        );
        app.editing.txd_refresh_job = Some(job);
        return;
    }

    if job.recompiled > 0 {
        rebuild_render_cells(app);
    }
    refresh_editing_dff_preview(app);
    app.status_message = format!(
        "Staged texture '{}' in {}; refreshed {} linked definition(s)",
        job.texture_name, job.txd_name, job.recompiled
    );
}

pub(crate) fn editing_import_face_texture_from_path(
    app: &mut AppState,
    txd_name: String,
    path: PathBuf,
) {
    let texture_name = texture_name_from_path(&path);
    if texture_name.is_empty() {
        app.status_message = "Could not derive a texture name from the file".to_string();
        return;
    }
    let native = match imported_image_texture_native(&path, &texture_name) {
        Ok(native) => native,
        Err(err) => {
            app.status_message = format!("Could not import texture: {err}");
            return;
        }
    };
    let before = editing_history_snapshot(app);
    if let Err(err) = stage_texture_native_into_txd(app, &txd_name, &texture_name, &native) {
        app.status_message = format!("Could not stage texture into {txd_name}: {err}");
        return;
    }
    if reassign_selected_dff_faces_texture(app, &texture_name) {
        commit_editing_history(app, "Create DFF Material", before);
        app.status_message = format!(
            "Imported '{texture_name}' into {txd_name} and created a material for selected face(s)"
        );
    }
}

pub(crate) fn editing_import_material_texture_from_path(
    app: &mut AppState,
    txd_name: String,
    dff_name: String,
    material: usize,
    path: PathBuf,
) {
    let still_valid = matches!(
        app.editing.asset.as_ref(),
        Some(EditingAsset::Dff(dff))
            if dff.name.eq_ignore_ascii_case(&dff_name)
                && dff.txd_context.as_ref().is_some_and(|current| current.eq_ignore_ascii_case(&txd_name))
                && material < dff_material_slot_count(&dff.raw)
    );
    if !still_valid {
        app.status_message =
            "The DFF or material changed while the texture browser was open".to_string();
        return;
    }
    let texture_name = texture_name_from_path(&path);
    if texture_name.is_empty() {
        app.status_message = "Could not derive a texture name from the file".to_string();
        return;
    }
    let native = match imported_image_texture_native(&path, &texture_name) {
        Ok(native) => native,
        Err(error) => {
            app.status_message = format!("Could not import texture: {error}");
            return;
        }
    };
    let before = editing_history_snapshot(app);
    if let Err(error) = stage_texture_native_into_txd(app, &txd_name, &texture_name, &native) {
        app.status_message = format!("Could not stage texture into {txd_name}: {error}");
        return;
    }
    let material_changed =
        editing_set_dff_material_texture(app, Some(&dff_name), material, &texture_name);
    commit_editing_history(app, "Set DFF Material Texture", before);
    app.status_message = if material_changed {
        format!("Imported '{texture_name}' into {txd_name} and set material #{material:02}")
    } else {
        format!("Imported updated texture '{texture_name}' into {txd_name}")
    };
}

fn build_gif_uv_animation(name: &str, fb: &GifFlipbook) -> DffUvAnimation {
    let inv_cols = 1.0 / fb.cols as f32;
    let inv_rows = 1.0 / fb.rows as f32;
    // A tiny hold-off so each cell holds steady, then snaps to the next.
    let eps = (fb.duration / (fb.frame_count.max(1) as f32 * 4.0)).clamp(0.0005, 0.02);
    let mut frames: Vec<DffUvAnimFrame> = Vec::with_capacity(fb.frame_count * 2);
    for i in 0..fb.frame_count {
        let col = (i % fb.cols) as f32;
        let row = (i / fb.cols) as f32;
        let matrix = [
            0.0,
            inv_cols,
            inv_rows,
            0.0,
            col * inv_cols,
            1.0 - (row * inv_rows + inv_rows),
        ];
        let start = fb.frame_starts[i];
        let end = if i + 1 < fb.frame_count {
            fb.frame_starts[i + 1]
        } else {
            fb.duration
        };
        let prev = frames.len() as i32 - 1;
        frames.push(DffUvAnimFrame {
            time: start,
            uv: matrix,
            prev,
        });
        let hold = (end - eps).max(start);
        let prev = frames.len() as i32 - 1;
        frames.push(DffUvAnimFrame {
            time: hold,
            uv: matrix,
            prev,
        });
    }
    DffUvAnimation {
        name: name.to_string(),
        type_id: 0x1c1,
        flags: 0,
        duration: fb.duration,
        node_to_uv: [0, 1, 0, 0, 0, 0, 0, 0],
        frames,
    }
}

pub(crate) fn start_dff_gif_anim_browse(app: &mut AppState) {
    let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_ref() else {
        return;
    };
    let Some(txd_name) = dff.txd_context.clone() else {
        app.status_message =
            "This model has no resolved TXD to import the GIF texture into".to_string();
        return;
    };
    if dff.raw.material_textures.is_empty() {
        app.status_message = "This model has no materials to animate".to_string();
        return;
    }
    drain_text_input();
    if app.dff_picker_rx.is_some() {
        app.status_message = "File browser is already open".to_string();
        return;
    }
    let start_dir = app.root.join("txd_build");
    let kind = DffPickerKind::EditingGifAnimImport { txd_name };
    let (tx, rx) = mpsc::channel();
    app.dff_picker_rx = Some(rx);
    thread::spawn(move || {
        let _ = tx.send((kind, choose_gif_path(start_dir)));
    });
}

pub(crate) fn editing_import_gif_anim_from_path(
    app: &mut AppState,
    txd_name: String,
    path: PathBuf,
) {
    let texture_name = texture_name_from_path(&path);
    if texture_name.is_empty() {
        app.status_message = "Could not derive a texture name from the GIF".to_string();
        return;
    }
    let anim_name = sanitize_texture_name(&format!("{texture_name}_gif"));
    let fb = match bake_gif_flipbook(&path) {
        Ok(fb) => fb,
        Err(err) => {
            app.status_message = format!("Could not read GIF: {err}");
            return;
        }
    };
    let native = texture_native_from_rgba(&fb.rgba, fb.width, fb.height, &texture_name);
    let before = editing_history_snapshot(app);
    if let Err(err) = stage_texture_native_into_txd(app, &txd_name, &texture_name, &native) {
        app.status_message = format!("Could not stage GIF texture into {txd_name}: {err}");
        return;
    }

    let animation = build_gif_uv_animation(&anim_name, &fb);
    let mut assigned = false;
    if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
        let material = dff
            .selected_material
            .min(dff.raw.material_textures.len() - 1);
        dff.raw.material_textures[material] = texture_name.clone();
        if let Some(idx) = dff
            .raw
            .uv_animations
            .iter()
            .position(|existing| existing.name.eq_ignore_ascii_case(&anim_name))
        {
            dff.raw.uv_animations[idx] = animation;
        } else {
            dff.raw.uv_animations.push(animation);
        }
        if dff.raw.material_animations.len() <= material {
            dff.raw
                .material_animations
                .resize_with(material + 1, DffMaterialAnim::default);
        }
        dff.raw.material_animations[material].names = vec![anim_name.clone()];
        dff.selected_material = material;
        dff.dirty = true;
        assigned = true;
        app.status_message = format!(
            "Imported GIF '{texture_name}' ({} frames, {}x{} grid) and set up animation on material {material}",
            fb.frame_count, fb.cols, fb.rows
        );
    }
    if assigned {
        commit_editing_history(app, "Setup GIF Animation", before);
        refresh_editing_dff_preview(app);
    }
}

pub(crate) fn open_img_entry_in_editing(app: &mut AppState, entry: ImgEntry) {
    let target = entry.name.clone();
    let same_archive = app
        .editing
        .img_path
        .as_ref()
        .is_some_and(|path| path == &entry.img_path);
    let blocked = if same_archive {
        request_discard_active_editing_asset(
            app,
            ConfirmAction::OpenEditingImgEntry(entry.clone()),
            &target,
        )
    } else {
        request_discard_editing_context(
            app,
            ConfirmAction::OpenEditingImgEntry(entry.clone()),
            &target,
        )
    };
    if blocked {
        return;
    }
    open_img_entry_in_editing_unchecked(app, entry);
}

pub(crate) fn open_img_entry_in_editing_unchecked(app: &mut AppState, entry: ImgEntry) {
    let name = entry.name.clone();
    remember_world_camera_before_editing(app);
    let same_archive = app
        .editing
        .img_path
        .as_ref()
        .is_some_and(|path| path == &entry.img_path);
    if !same_archive {
        open_editing_img_unchecked(app, entry.img_path.clone());
    }
    app.editing.search.clear();
    app.editing.search_cursor = 0;
    if let Some(row_idx) = app
        .editing
        .rows
        .iter()
        .position(|row| row.entry.name.eq_ignore_ascii_case(&name))
    {
        app.editing.selected_row = row_idx;
        app.editing.scroll = row_idx.saturating_sub(4) as f32;
        editing_open_selected_asset(app);
        app.editing.camera = Some(app.camera);
        app.active_tab = AppTab::Editing;
    }
}

pub(crate) fn open_selected_dff_in_editing(app: &mut AppState) {
    let Some(placement) = selected_placement(app).cloned() else {
        app.status_message = "Select an element before opening the DFF editor".to_string();
        return;
    };
    if let Some(entry) = find_dff_entry_for_app(app, &placement.dff) {
        open_img_entry_in_editing(app, entry);
    } else {
        app.status_message = format!("Could not resolve DFF {}", placement.dff);
    }
}

pub(crate) fn open_selected_txd_in_editing(app: &mut AppState) {
    let Some((_definition_id, txd_name)) = selected_definition_id_and_txd(app) else {
        app.status_message = "Selected element has no TXD definition".to_string();
        return;
    };
    if let Some(entry) = find_txd_entry_for_app(app, &txd_name) {
        open_img_entry_in_editing(app, entry);
    } else {
        app.status_message = format!("Could not resolve TXD {txd_name}");
    }
}

fn open_staged_asset_in_editing(app: &mut AppState, name: &str) -> bool {
    if request_discard_active_editing_asset(
        app,
        ConfirmAction::OpenEditingStagedAsset(name.to_string()),
        name,
    ) {
        return true;
    }
    open_staged_asset_in_editing_unchecked(app, name)
}

pub(crate) fn open_staged_asset_in_editing_unchecked(app: &mut AppState, name: &str) -> bool {
    let key = editing_key(name);
    let Some(bytes) = app.editing.modified_entries.get(&key).cloned() else {
        return false;
    };
    let fallback_img_path = app.root.join("imgs").join(REPLACEMENT_IMG);
    editing_stage_added_entry(&mut app.editing, fallback_img_path, name, bytes);
    let Some(row_index) = app
        .editing
        .rows
        .iter()
        .position(|row| editing_key(&row.entry.name) == key)
    else {
        return false;
    };
    remember_world_camera_before_editing(app);
    app.editing.search.clear();
    app.editing.search_cursor = 0;
    app.editing.search_anchor = None;
    app.editing.selected_row = row_index;
    app.editing.scroll = row_index.saturating_sub(4) as f32;
    editing_open_selected_asset(app);
    app.editing.camera = Some(app.camera);
    app.active_tab = AppTab::Editing;
    true
}

pub(crate) fn open_selected_col_in_editing(app: &mut AppState) {
    let Some(placement) = selected_placement(app).cloned() else {
        app.status_message = "Select an element before opening the COL editor".to_string();
        return;
    };
    let col_name = element_collision_key(app, &placement);
    if open_staged_asset_in_editing(app, &col_name) {
        return;
    }
    if let Some(entry) = find_col_entry(&app.root, &col_name) {
        open_img_entry_in_editing(app, entry);
    } else {
        app.status_message = format!("Could not resolve COL {col_name}");
    }
}

fn update_editing_search_input(app: &mut AppState, mouse: Vec2) -> bool {
    let rect = editing_search_rect();
    if is_mouse_button_pressed(MouseButton::Left) {
        app.editing.search_active = rect.contains(mouse);
        if app.editing.search_active {
            app.editing.search_cursor = app.editing.search.len();
            app.editing.search_anchor = None;
            drain_text_input();
            return true;
        }
    }
    if !app.editing.search_active {
        return false;
    }
    let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
    let mut changed = false;
    if is_key_pressed(KeyCode::Escape) {
        app.editing.search_active = false;
        return true;
    }
    if is_key_pressed(KeyCode::Backspace) {
        if delete_text_selection(
            &mut app.editing.search,
            &mut app.editing.search_cursor,
            &mut app.editing.search_anchor,
        ) {
            changed = true;
        } else if app.editing.search_cursor > 0 {
            let prev = if ctrl {
                prev_word_boundary(&app.editing.search, app.editing.search_cursor)
            } else {
                prev_char_boundary(&app.editing.search, app.editing.search_cursor)
            };
            app.editing
                .search
                .replace_range(prev..app.editing.search_cursor, "");
            app.editing.search_cursor = prev;
            changed = true;
        }
    }
    if is_key_pressed(KeyCode::Delete) {
        if delete_text_selection(
            &mut app.editing.search,
            &mut app.editing.search_cursor,
            &mut app.editing.search_anchor,
        ) {
            changed = true;
        } else if app.editing.search_cursor < app.editing.search.len() {
            let next = next_char_boundary(&app.editing.search, app.editing.search_cursor);
            app.editing
                .search
                .replace_range(app.editing.search_cursor..next, "");
            changed = true;
        }
    }
    while let Some(ch) = get_char_pressed() {
        if !ch.is_control() {
            insert_text_at_cursor(
                &mut app.editing.search,
                &mut app.editing.search_cursor,
                &mut app.editing.search_anchor,
                &ch.to_string(),
            );
            changed = true;
        }
    }
    if changed {
        app.editing.selected_row = 0;
        app.editing.scroll = 0.0;
    }
    changed
}

fn update_editing_txd_search_input(app: &mut AppState, mouse: Vec2) -> bool {
    let rect = editing_txd_search_rect();
    let Some(EditingAsset::Txd(txd)) = app.editing.asset.as_mut() else {
        return false;
    };
    if is_mouse_button_pressed(MouseButton::Left) {
        txd.search_active = rect.contains(mouse);
        if txd.search_active {
            txd.search_cursor = txd.search.len();
            txd.search_anchor = None;
            drain_text_input();
            return true;
        }
    }
    if !txd.search_active {
        return false;
    }
    if is_key_pressed(KeyCode::Escape) {
        txd.search_active = false;
        return true;
    }
    let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
    let mut changed = false;
    if is_key_pressed(KeyCode::Backspace) {
        if delete_text_selection(
            &mut txd.search,
            &mut txd.search_cursor,
            &mut txd.search_anchor,
        ) {
            changed = true;
        } else if txd.search_cursor > 0 {
            let previous = if ctrl {
                prev_word_boundary(&txd.search, txd.search_cursor)
            } else {
                prev_char_boundary(&txd.search, txd.search_cursor)
            };
            txd.search.replace_range(previous..txd.search_cursor, "");
            txd.search_cursor = previous;
            changed = true;
        }
    }
    if is_key_pressed(KeyCode::Delete) {
        if delete_text_selection(
            &mut txd.search,
            &mut txd.search_cursor,
            &mut txd.search_anchor,
        ) {
            changed = true;
        } else if txd.search_cursor < txd.search.len() {
            let next = next_char_boundary(&txd.search, txd.search_cursor);
            txd.search.replace_range(txd.search_cursor..next, "");
            changed = true;
        }
    }
    while let Some(ch) = get_char_pressed() {
        if !ch.is_control() {
            insert_text_at_cursor(
                &mut txd.search,
                &mut txd.search_cursor,
                &mut txd.search_anchor,
                &ch.to_string(),
            );
            changed = true;
        }
    }
    if changed {
        txd.scroll = 0.0;
        txd.selected = editing_txd_filtered_indices(txd)
            .first()
            .copied()
            .unwrap_or(0);
        editing_update_txd_preview(app);
    }
    changed
}

fn editing_nearest_col_vertex_screen(
    app: &AppState,
    col: &EditingColState,
    viewport: Rect,
    mouse: Vec2,
) -> Option<(usize, usize)> {
    let ray = viewport_ray(app, viewport, mouse);
    let mut best: Option<(usize, usize, f32)> = None;
    for (face_idx, face) in col.mesh.faces.iter().enumerate() {
        for (slot, idx) in [face.a, face.b, face.c].into_iter().enumerate() {
            let Some(vertex) = col.mesh.vertices.get(idx as usize) else {
                continue;
            };
            let Some(screen) = world_to_screen(app, viewport, to_mq(*vertex)) else {
                continue;
            };
            let distance = screen.distance(mouse);
            if distance > EDIT_VERTEX_PICK_RADIUS {
                continue;
            }
            if let Some((origin, dir)) = ray {
                let vertex_depth = ray_depth_to_point(origin, dir, to_mq(*vertex));
                if vertex_depth > 0.0
                    && col_mesh_occludes_vertex(&col.mesh, origin, dir, idx as usize, vertex_depth)
                {
                    continue;
                }
            }
            if match best {
                Some((_, _, best_distance)) => distance < best_distance,
                None => true,
            } {
                best = Some((face_idx, slot, distance));
            }
        }
    }
    best.map(|(face, slot, _)| (face, slot))
}

fn editing_hover_col_face_vertex(
    app: &AppState,
    col: &EditingColState,
    viewport: Rect,
    mouse: Vec2,
) -> Option<(usize, usize)> {
    editing_nearest_col_vertex_screen(app, col, viewport, mouse)
}

fn editing_pick_col_face(
    app: &AppState,
    col: &EditingColState,
    viewport: Rect,
    mouse: Vec2,
) -> Option<usize> {
    let (origin, dir) = viewport_ray(app, viewport, mouse)?;
    ray_collision_triangles(origin, dir, &col.mesh).map(|(face, _)| face)
}

// Ray-pick the front-most native or rotated COL box face.
pub(crate) fn editing_pick_col_box_face(
    app: &AppState,
    viewport: Rect,
    mouse: Vec2,
) -> Option<ColBoxFacePick> {
    let Some(EditingAsset::Col(col)) = app.editing.asset.as_ref() else {
        return None;
    };
    let (ro, rd) = viewport_ray(app, viewport, mouse)?;
    let mut best: Option<(f32, ColBoxFacePick)> = None;
    let mut test = |primitive: CollisionPrimitiveSelection,
                    center: Vec3,
                    half_extents: Vec3,
                    rotation_value: V3| {
        let rotation = cuboid_rotation_matrix(rotation_value);
        let inverse = rotation.transpose();
        let local_ro = inverse.transform_vector3(ro - center);
        let local_rd = inverse.transform_vector3(rd);
        for axis in 0..3 {
            for side_is_max in [false, true] {
                let side = if side_is_max { 1.0 } else { -1.0 };
                let denom = local_rd[axis];
                if denom.abs() < 1.0e-6 {
                    continue;
                }
                let t = (side * half_extents[axis] - local_ro[axis]) / denom;
                if t <= 0.0 {
                    continue;
                }
                let hit = local_ro + local_rd * t;
                let inside = (0..3).filter(|other| *other != axis).all(|other| {
                    let pad = half_extents[other] * 0.02 + 0.05;
                    hit[other] >= -half_extents[other] - pad
                        && hit[other] <= half_extents[other] + pad
                });
                if !inside || best.as_ref().is_some_and(|(best_t, _)| t >= *best_t) {
                    continue;
                }
                let mut local_axis = Vec3::ZERO;
                local_axis[axis] = 1.0;
                let axis_dir = rotation.transform_vector3(local_axis).normalize_or_zero();
                best = Some((
                    t,
                    ColBoxFacePick {
                        primitive,
                        axis,
                        side_is_max,
                        center,
                        half_extents,
                        axis_dir,
                        face_origin: center + axis_dir * (side * half_extents[axis]),
                    },
                ));
            }
        }
    };
    for (box_index, col_box) in col.mesh.boxes.iter().enumerate() {
        let a = to_mq(col_box.min);
        let b = to_mq(col_box.max);
        let lo = vec3(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z));
        let hi = vec3(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z));
        test(
            CollisionPrimitiveSelection {
                kind: CollisionPrimitiveKind::Box,
                index: box_index,
            },
            (lo + hi) * 0.5,
            (hi - lo) * 0.5,
            V3::default(),
        );
    }
    for (cuboid_index, cuboid) in col.cuboids.iter().enumerate() {
        test(
            CollisionPrimitiveSelection {
                kind: CollisionPrimitiveKind::Cuboid,
                index: cuboid_index,
            },
            to_mq(cuboid.center),
            to_mq(cuboid.half_extents),
            cuboid.rotation,
        );
    }
    best.map(|(_, pick)| pick)
}

fn editing_pick_col_edge(
    app: &AppState,
    col: &EditingColState,
    viewport: Rect,
    mouse: Vec2,
) -> Option<(usize, usize)> {
    let mut best: Option<((usize, usize), f32)> = None;
    for face in &col.mesh.faces {
        for edge in col_face_edge_vertices(face) {
            let (Some(a), Some(b)) = (col.mesh.vertices.get(edge.0), col.mesh.vertices.get(edge.1))
            else {
                continue;
            };
            let (Some(sa), Some(sb)) = (
                world_to_screen(app, viewport, to_mq(*a)),
                world_to_screen(app, viewport, to_mq(*b)),
            ) else {
                continue;
            };
            let dist = dist_to_segment(mouse, sa, sb);
            if dist <= EDIT_VERTEX_PICK_RADIUS && best.is_none_or(|(_, best_dist)| dist < best_dist)
            {
                best = Some((edge, dist));
            }
        }
    }
    best.map(|(edge, _)| edge)
}

fn editing_nearest_dff_vertex_screen(
    app: &AppState,
    dff: &EditingDffState,
    viewport: Rect,
    mouse: Vec2,
) -> Option<(usize, usize)> {
    let ray = viewport_ray(app, viewport, mouse);
    let mut best: Option<(usize, usize, f32)> = None;
    for (face_idx, tri) in dff.raw.triangles.iter().enumerate() {
        for vertex_idx in [tri.a as usize, tri.b as usize, tri.c as usize] {
            let Some(vertex) = dff.raw.vertices.get(vertex_idx) else {
                continue;
            };
            let Some(screen) = world_to_screen(app, viewport, to_mq(*vertex)) else {
                continue;
            };
            let distance = screen.distance(mouse);
            if distance > EDIT_VERTEX_PICK_RADIUS {
                continue;
            }
            if let Some((origin, dir)) = ray {
                let vertex_depth = ray_depth_to_point(origin, dir, to_mq(*vertex));
                if vertex_depth > 0.0
                    && raw_mesh_occludes_vertex(&dff.raw, origin, dir, vertex_idx, vertex_depth)
                {
                    continue;
                }
            }
            if match best {
                Some((_, _, best_distance)) => distance < best_distance,
                None => true,
            } {
                best = Some((face_idx, vertex_idx, distance));
            }
        }
    }
    best.map(|(face, vertex, _)| (face, vertex))
}

fn editing_hover_dff_face_vertex(
    app: &AppState,
    dff: &EditingDffState,
    viewport: Rect,
    mouse: Vec2,
) -> Option<(usize, usize)> {
    editing_nearest_dff_vertex_screen(app, dff, viewport, mouse)
}

fn editing_pick_dff_face(
    app: &AppState,
    dff: &EditingDffState,
    viewport: Rect,
    mouse: Vec2,
) -> Option<usize> {
    let (origin, dir) = viewport_ray(app, viewport, mouse)?;
    let mut best = None;
    let mut best_t = f32::MAX;
    for (face_idx, tri) in dff.raw.triangles.iter().enumerate() {
        let (a, b, c) = (tri.a as usize, tri.b as usize, tri.c as usize);
        let (Some(a), Some(b), Some(c)) = (
            dff.raw.vertices.get(a).map(|v| to_mq(*v)),
            dff.raw.vertices.get(b).map(|v| to_mq(*v)),
            dff.raw.vertices.get(c).map(|v| to_mq(*v)),
        ) else {
            continue;
        };
        if let Some(t) = ray_triangle(origin, dir, a, b, c) {
            if t < best_t {
                best_t = t;
                best = Some(face_idx);
            }
        }
    }
    best
}

fn editing_pick_dff_edge(
    app: &AppState,
    dff: &EditingDffState,
    viewport: Rect,
    mouse: Vec2,
) -> Option<(usize, usize)> {
    let mut best: Option<((usize, usize), f32)> = None;
    for tri in &dff.raw.triangles {
        for edge in dff_triangle_edge_vertices(tri) {
            let (Some(a), Some(b)) = (dff.raw.vertices.get(edge.0), dff.raw.vertices.get(edge.1))
            else {
                continue;
            };
            let (Some(sa), Some(sb)) = (
                world_to_screen(app, viewport, to_mq(*a)),
                world_to_screen(app, viewport, to_mq(*b)),
            ) else {
                continue;
            };
            let dist = dist_to_segment(mouse, sa, sb);
            if dist <= EDIT_VERTEX_PICK_RADIUS && best.is_none_or(|(_, best_dist)| dist < best_dist)
            {
                best = Some((edge, dist));
            }
        }
    }
    best.map(|(edge, _)| edge)
}

fn dff_face_for_vertex(dff: &EditingDffState, vertex: usize) -> Option<usize> {
    dff.raw.triangles.iter().position(|tri| {
        tri.a as usize == vertex || tri.b as usize == vertex || tri.c as usize == vertex
    })
}

fn col_face_slot_for_vertex(col: &EditingColState, vertex: usize) -> Option<(usize, usize)> {
    col.mesh
        .faces
        .iter()
        .enumerate()
        .find_map(|(face_idx, face)| {
            [face.a, face.b, face.c]
                .into_iter()
                .position(|idx| idx as usize == vertex)
                .map(|slot| (face_idx, slot))
        })
}

fn ray_depth_to_point(origin: Vec3, dir: Vec3, point: Vec3) -> f32 {
    (point - origin).dot(dir)
}

fn raw_mesh_occludes_vertex(
    raw: &RawMesh,
    origin: Vec3,
    dir: Vec3,
    vertex_idx: usize,
    vertex_depth: f32,
) -> bool {
    let mut best = f32::MAX;
    for tri in &raw.triangles {
        if tri.a as usize == vertex_idx
            || tri.b as usize == vertex_idx
            || tri.c as usize == vertex_idx
        {
            continue;
        }
        let (a, b, c) = (tri.a as usize, tri.b as usize, tri.c as usize);
        let (Some(a), Some(b), Some(c)) = (
            raw.vertices.get(a).map(|v| to_mq(*v)),
            raw.vertices.get(b).map(|v| to_mq(*v)),
            raw.vertices.get(c).map(|v| to_mq(*v)),
        ) else {
            continue;
        };
        if let Some(t) = ray_triangle(origin, dir, a, b, c) {
            best = best.min(t);
        }
    }
    best + VERTEX_OCCLUSION_TOLERANCE < vertex_depth
}

fn col_mesh_occludes_vertex(
    mesh: &CollisionMesh,
    origin: Vec3,
    dir: Vec3,
    vertex_idx: usize,
    vertex_depth: f32,
) -> bool {
    let mut best = f32::MAX;
    for face in &mesh.faces {
        if face.a as usize == vertex_idx
            || face.b as usize == vertex_idx
            || face.c as usize == vertex_idx
        {
            continue;
        }
        let Some((a, b, c)) = collision_face_points(mesh, face) else {
            continue;
        };
        if let Some(t) = ray_triangle(origin, dir, a, b, c) {
            best = best.min(t);
        }
    }
    best + VERTEX_OCCLUSION_TOLERANCE < vertex_depth
}

fn camera_facing_vertex_order(
    camera_pos: Vec3,
    indices: &[usize],
    point_at: impl Fn(usize) -> Option<Vec3>,
) -> Option<Vec<usize>> {
    if indices.len() < 3 {
        return None;
    }
    let mut points = Vec::<(usize, Vec3)>::new();
    for idx in indices {
        points.push((*idx, point_at(*idx)?));
    }
    let centroid = points
        .iter()
        .fold(Vec3::ZERO, |sum, (_, point)| sum + *point)
        / points.len() as f32;
    let view_dir = (centroid - camera_pos).normalize_or_zero();
    let mut right = view_dir.cross(Vec3::Z).normalize_or_zero();
    if right.length_squared() < 0.0001 {
        right = Vec3::X;
    }
    let up = right.cross(view_dir).normalize_or_zero();
    points.sort_by(|(_, a), (_, b)| {
        let da = *a - centroid;
        let db = *b - centroid;
        let aa = da.dot(up).atan2(da.dot(right));
        let ab = db.dot(up).atan2(db.dot(right));
        aa.partial_cmp(&ab).unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut ordered = points.iter().map(|(idx, _)| *idx).collect::<Vec<_>>();
    let p0 = point_at(ordered[0])?;
    let p1 = point_at(ordered[1])?;
    let p2 = point_at(ordered[2])?;
    let normal = (p1 - p0).cross(p2 - p0).normalize_or_zero();
    let to_camera = (camera_pos - centroid).normalize_or_zero();
    if normal.dot(to_camera) > 0.0 {
        ordered[1..].reverse();
    }
    Some(ordered)
}

fn apply_box_selection_set<T: Ord + Copy>(
    current: &mut BTreeSet<T>,
    incoming: BTreeSet<T>,
    mode: BoxSelectMode,
) {
    match mode {
        BoxSelectMode::Add => current.extend(incoming),
        BoxSelectMode::Subtract => {
            for item in incoming {
                current.remove(&item);
            }
        }
        BoxSelectMode::Toggle => {
            for item in incoming {
                if !current.insert(item) {
                    current.remove(&item);
                }
            }
        }
    }
}

pub(crate) fn box_select_editing_vertices(app: &mut AppState, start: Vec2, end: Vec2) -> bool {
    if app.active_tab != AppTab::Editing {
        return false;
    }
    let viewport = editing_center_rect();
    let rect = normalized_screen_rect(start, end);
    if rect.w < 4.0 || rect.h < 4.0 {
        return false;
    }
    enum BoxSelection {
        Dff(BTreeSet<usize>),
        DffFaces(BTreeSet<usize>),
        DffEdges(BTreeSet<(usize, usize)>),
        Col(BTreeSet<usize>),
        ColFaces(BTreeSet<usize>),
        ColEdges(BTreeSet<(usize, usize)>),
        Unsupported,
    }
    let selection = match app.editing.asset.as_ref() {
        Some(EditingAsset::Dff(dff)) if dff.select_mode == EditingSelectMode::Edge => {
            BoxSelection::DffEdges(
                dff.raw
                    .triangles
                    .iter()
                    .flat_map(dff_triangle_edge_vertices)
                    .filter_map(|edge| {
                        let a = dff.raw.vertices.get(edge.0)?;
                        let b = dff.raw.vertices.get(edge.1)?;
                        let mid = to_mq(V3 {
                            x: (a.x + b.x) * 0.5,
                            y: (a.y + b.y) * 0.5,
                            z: (a.z + b.z) * 0.5,
                        });
                        let screen = world_to_screen(app, viewport, mid)?;
                        rect.contains(screen).then_some(edge)
                    })
                    .collect(),
            )
        }
        Some(EditingAsset::Dff(dff)) if dff.select_mode == EditingSelectMode::Face => {
            BoxSelection::DffFaces(
                dff.raw
                    .triangles
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, tri)| {
                        let a = dff.raw.vertices.get(tri.a as usize)?;
                        let b = dff.raw.vertices.get(tri.b as usize)?;
                        let c = dff.raw.vertices.get(tri.c as usize)?;
                        let centroid = to_mq(V3 {
                            x: (a.x + b.x + c.x) / 3.0,
                            y: (a.y + b.y + c.y) / 3.0,
                            z: (a.z + b.z + c.z) / 3.0,
                        });
                        let screen = world_to_screen(app, viewport, centroid)?;
                        rect.contains(screen).then_some(idx)
                    })
                    .collect(),
            )
        }
        Some(EditingAsset::Dff(dff)) => BoxSelection::Dff(
            dff.raw
                .vertices
                .iter()
                .enumerate()
                .filter_map(|(idx, vertex)| {
                    let screen = world_to_screen(app, viewport, to_mq(*vertex))?;
                    rect.contains(screen).then_some(idx)
                })
                .collect(),
        ),
        Some(EditingAsset::Col(col)) if col.select_mode == EditingSelectMode::Face => {
            BoxSelection::ColFaces(
                col.mesh
                    .faces
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, face)| {
                        if col_generated_primitive_for_face(col, idx).is_some() {
                            return None;
                        }
                        let a = col.mesh.vertices.get(face.a as usize)?;
                        let b = col.mesh.vertices.get(face.b as usize)?;
                        let c = col.mesh.vertices.get(face.c as usize)?;
                        let screen = world_to_screen(
                            app,
                            viewport,
                            to_mq(V3 {
                                x: (a.x + b.x + c.x) / 3.0,
                                y: (a.y + b.y + c.y) / 3.0,
                                z: (a.z + b.z + c.z) / 3.0,
                            }),
                        )?;
                        rect.contains(screen).then_some(idx)
                    })
                    .collect(),
            )
        }
        Some(EditingAsset::Col(col)) if col.select_mode == EditingSelectMode::Edge => {
            BoxSelection::ColEdges(
                col.mesh
                    .faces
                    .iter()
                    .enumerate()
                    .filter(|(idx, _)| col_generated_primitive_for_face(col, *idx).is_none())
                    .flat_map(|(_, face)| col_face_edge_vertices(face))
                    .filter_map(|edge| {
                        if col_vertices_touch_generated_primitive(col, [edge.0, edge.1]) {
                            return None;
                        }
                        let a = col.mesh.vertices.get(edge.0)?;
                        let b = col.mesh.vertices.get(edge.1)?;
                        let mid = to_mq(V3 {
                            x: (a.x + b.x) * 0.5,
                            y: (a.y + b.y) * 0.5,
                            z: (a.z + b.z) * 0.5,
                        });
                        let screen = world_to_screen(app, viewport, mid)?;
                        rect.contains(screen).then_some(edge)
                    })
                    .collect(),
            )
        }
        Some(EditingAsset::Col(col)) => BoxSelection::Col(
            col.mesh
                .vertices
                .iter()
                .enumerate()
                .filter_map(|(idx, vertex)| {
                    if col_generated_primitive_for_vertex(col, idx).is_some() {
                        return None;
                    }
                    let screen = world_to_screen(app, viewport, to_mq(*vertex))?;
                    rect.contains(screen).then_some(idx)
                })
                .collect(),
        ),
        _ => BoxSelection::Unsupported,
    };
    let mode = app.box_select_mode;
    match (selection, app.editing.asset.as_mut()) {
        (BoxSelection::Dff(selected), Some(EditingAsset::Dff(dff))) => {
            if selected.is_empty() {
                app.status_message = "Box selected 0 DFF vertices".to_string();
                return true;
            }
            let active = selected.iter().next_back().copied();
            apply_box_selection_set(&mut dff.selected_vertices, selected, mode);
            dff.selected_faces.clear();
            dff.selected_vertex = dff.selected_vertices.iter().next_back().copied().or(active);
            dff.selected_face = dff
                .selected_vertex
                .and_then(|vertex| dff_face_for_vertex(dff, vertex));
            if let Some(face) = dff.selected_face.and_then(|idx| dff.raw.triangles.get(idx)) {
                dff.selected_material = face.material as usize;
            }
            app.status_message = format!(
                "Box {} DFF vertices; {} selected",
                mode.verb(),
                dff.selected_vertices.len()
            );
            true
        }
        (BoxSelection::DffFaces(selected), Some(EditingAsset::Dff(dff))) => {
            if selected.is_empty() {
                app.status_message = "Box selected 0 DFF faces".to_string();
                return true;
            }
            apply_box_selection_set(&mut dff.selected_faces, selected, mode);
            dff.selected_face = dff.selected_faces.iter().next_back().copied();
            dff.selected_vertex = None;
            dff.selected_vertices.clear();
            if let Some(face) = dff.selected_face.and_then(|idx| dff.raw.triangles.get(idx)) {
                dff.selected_material = face.material as usize;
            }
            app.status_message = format!(
                "Box {} DFF faces; {} selected",
                mode.verb(),
                dff.selected_faces.len()
            );
            true
        }
        (BoxSelection::DffEdges(selected), Some(EditingAsset::Dff(dff))) => {
            if selected.is_empty() {
                app.status_message = "Box selected 0 DFF edges".to_string();
                return true;
            }
            apply_box_selection_set(&mut dff.selected_edges, selected, mode);
            dff.selected_faces.clear();
            dff.selected_face = None;
            dff.selected_vertices.clear();
            dff.selected_vertex = None;
            app.status_message = format!(
                "Box {} DFF edges; {} selected",
                mode.verb(),
                dff.selected_edges.len()
            );
            true
        }
        (BoxSelection::Col(selected), Some(EditingAsset::Col(col))) => {
            if selected.is_empty() {
                app.status_message = "Box selected 0 COL vertices".to_string();
                return true;
            }
            let active = selected.iter().next_back().copied();
            apply_box_selection_set(&mut col.selected_vertices, selected, mode);
            col.selected_faces.clear();
            if let Some((face, slot)) = col
                .selected_vertices
                .iter()
                .next_back()
                .copied()
                .or(active)
                .and_then(|vertex| col_face_slot_for_vertex(col, vertex))
            {
                col.selected_face = face;
                col.selected_vertex = slot;
            }
            app.status_message = format!(
                "Box {} COL vertices; {} selected",
                mode.verb(),
                col.selected_vertices.len()
            );
            true
        }
        (BoxSelection::ColFaces(selected), Some(EditingAsset::Col(col))) => {
            if selected.is_empty() {
                app.status_message = "Box selected 0 COL faces".to_string();
                return true;
            }
            apply_box_selection_set(&mut col.selected_faces, selected, mode);
            col.selected_face = col
                .selected_faces
                .iter()
                .next_back()
                .copied()
                .unwrap_or(usize::MAX);
            col.selected_vertex = 0;
            col.selected_vertices.clear();
            col.selected_primitive = None;
            app.status_message = format!(
                "Box {} COL faces; {} selected",
                mode.verb(),
                col.selected_faces.len()
            );
            true
        }
        (BoxSelection::ColEdges(selected), Some(EditingAsset::Col(col))) => {
            if selected.is_empty() {
                app.status_message = "Box selected 0 COL edges".to_string();
                return true;
            }
            apply_box_selection_set(&mut col.selected_edges, selected, mode);
            col.selected_faces.clear();
            col.selected_vertices.clear();
            col.selected_primitive = None;
            app.status_message = format!(
                "Box {} COL edges; {} selected",
                mode.verb(),
                col.selected_edges.len()
            );
            true
        }
        _ => false,
    }
}

const LINKED_SELECTION_POSITION_SCALE: f32 = 1000.0;

fn linked_selection_position_key(vertex: V3) -> Result<[i32; 3], String> {
    if !vertex.x.is_finite() || !vertex.y.is_finite() || !vertex.z.is_finite() {
        return Err("mesh contains a non-finite vertex".to_string());
    }
    let scaled = [
        vertex.x * LINKED_SELECTION_POSITION_SCALE,
        vertex.y * LINKED_SELECTION_POSITION_SCALE,
        vertex.z * LINKED_SELECTION_POSITION_SCALE,
    ];
    if scaled
        .iter()
        .any(|value| *value < i32::MIN as f32 || *value > i32::MAX as f32)
    {
        return Err("mesh vertex exceeds linked-selection coordinate limits".to_string());
    }
    Ok(scaled.map(|value| value.round() as i32))
}

fn linked_face_component(
    face_positions: &[[[i32; 3]; 3]],
    seed_face: usize,
) -> Result<Vec<usize>, String> {
    if seed_face >= face_positions.len() {
        return Err("the hovered face no longer exists".to_string());
    }
    let mut faces_by_position = HashMap::<[i32; 3], Vec<usize>>::new();
    for (face_index, positions) in face_positions.iter().enumerate() {
        for (slot, position) in positions.iter().copied().enumerate() {
            if positions[..slot].contains(&position) {
                continue;
            }
            faces_by_position
                .entry(position)
                .or_default()
                .push(face_index);
        }
    }

    let mut visited = vec![false; face_positions.len()];
    let mut stack = vec![seed_face];
    visited[seed_face] = true;
    let mut linked = Vec::new();
    while let Some(face_index) = stack.pop() {
        linked.push(face_index);
        for position in face_positions[face_index] {
            let Some(neighbors) = faces_by_position.get(&position) else {
                continue;
            };
            for neighbor in neighbors.iter().copied() {
                if !visited[neighbor] {
                    visited[neighbor] = true;
                    stack.push(neighbor);
                }
            }
        }
    }
    linked.sort_unstable();
    Ok(linked)
}

fn request_editing_linked_selection(app: &mut AppState) {
    if app.editing.linked_selection_job.is_some() {
        app.status_message = "Linked selection is already running".to_string();
        return;
    }
    let (kind, asset_name, seed_face, face_count, clear_existing) = match app.editing.asset.as_ref()
    {
        Some(EditingAsset::Dff(dff)) => {
            if dff.fracture_preview_started_at.is_some() {
                app.status_message =
                    "Reset the fracture preview before selecting linked geometry".to_string();
                return;
            }
            let Some(seed_face) = dff.hovered_face else {
                app.status_message = "Hover a DFF face before pressing L".to_string();
                return;
            };
            (
                EditingLinkedAssetKind::Dff,
                dff.name.clone(),
                seed_face,
                dff.raw.triangles.len(),
                dff.select_mode != EditingSelectMode::Face,
            )
        }
        Some(EditingAsset::Col(col)) => {
            let Some(seed_face) = col.hovered_face else {
                app.status_message = "Hover a COL face before pressing L".to_string();
                return;
            };
            if col_generated_primitive_for_face(col, seed_face).is_some() {
                app.status_message =
                    "Linked selection does not edit generated primitive faces; select the capsule or rotated box instead"
                        .to_string();
                return;
            }
            (
                EditingLinkedAssetKind::Col,
                col.name.clone(),
                seed_face,
                col.mesh.faces.len(),
                col.select_mode != EditingSelectMode::Face,
            )
        }
        _ => return,
    };
    if seed_face >= face_count {
        app.status_message = "The hovered face no longer exists".to_string();
        return;
    }
    let deselect = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
    app.editing.linked_selection_job = Some(EditingLinkedSelectionJob {
        kind,
        asset_name,
        seed_face,
        deselect,
        face_count,
        snapshot_index: 0,
        face_positions: Vec::new(),
        rx: None,
        result: None,
        apply_index: 0,
        apply_started: false,
        clear_existing,
        changed: 0,
        started_at: Instant::now(),
    });
    app.status_message = if deselect {
        "Preparing linked-island deselection...".to_string()
    } else {
        "Preparing linked-island selection...".to_string()
    };
}

fn linked_selection_asset_is_current(app: &AppState, job: &EditingLinkedSelectionJob) -> bool {
    match (&app.editing.asset, job.kind) {
        (Some(EditingAsset::Dff(dff)), EditingLinkedAssetKind::Dff) => {
            editing_key(&dff.name) == editing_key(&job.asset_name)
                && dff.raw.triangles.len() == job.face_count
        }
        (Some(EditingAsset::Col(col)), EditingLinkedAssetKind::Col) => {
            editing_key(&col.name) == editing_key(&job.asset_name)
                && col.mesh.faces.len() == job.face_count
        }
        _ => false,
    }
}

fn linked_selection_face_positions(
    app: &AppState,
    kind: EditingLinkedAssetKind,
    face_index: usize,
) -> Result<[[i32; 3]; 3], String> {
    let (vertices, indices): (&[V3], [usize; 3]) =
        match (&app.editing.asset, kind) {
            (Some(EditingAsset::Dff(dff)), EditingLinkedAssetKind::Dff) => {
                let tri =
                    dff.raw.triangles.get(face_index).ok_or_else(|| {
                        "DFF topology changed during linked selection".to_string()
                    })?;
                (
                    &dff.raw.vertices,
                    [tri.a as usize, tri.b as usize, tri.c as usize],
                )
            }
            (Some(EditingAsset::Col(col)), EditingLinkedAssetKind::Col) => {
                let face =
                    col.mesh.faces.get(face_index).ok_or_else(|| {
                        "COL topology changed during linked selection".to_string()
                    })?;
                (
                    &col.mesh.vertices,
                    [face.a as usize, face.b as usize, face.c as usize],
                )
            }
            _ => return Err("The edited asset changed during linked selection".to_string()),
        };
    let mut positions = [[0i32; 3]; 3];
    for (slot, vertex_index) in indices.into_iter().enumerate() {
        let vertex = vertices
            .get(vertex_index)
            .copied()
            .ok_or_else(|| format!("face {face_index} references an invalid vertex"))?;
        positions[slot] = linked_selection_position_key(vertex)?;
    }
    Ok(positions)
}

pub(crate) fn update_editing_linked_selection_job(app: &mut AppState) {
    const SNAPSHOT_BATCH_LIMIT: usize = 2048;
    const APPLY_BATCH_LIMIT: usize = 2048;
    const FRAME_BUDGET: Duration = Duration::from_millis(4);

    let Some(mut job) = app.editing.linked_selection_job.take() else {
        return;
    };
    if !linked_selection_asset_is_current(app, &job) {
        app.status_message =
            "Linked selection cancelled because the edited mesh changed".to_string();
        return;
    }

    if job.rx.is_none() && job.result.is_none() {
        let started = Instant::now();
        let mut processed = 0usize;
        while job.snapshot_index < job.face_count
            && processed < SNAPSHOT_BATCH_LIMIT
            && started.elapsed() < FRAME_BUDGET
        {
            match linked_selection_face_positions(app, job.kind, job.snapshot_index) {
                Ok(positions) => job.face_positions.push(positions),
                Err(error) => {
                    app.status_message = format!("Linked selection failed: {error}");
                    return;
                }
            }
            job.snapshot_index += 1;
            processed += 1;
        }
        if job.snapshot_index < job.face_count {
            app.status_message = format!(
                "Preparing linked selection... {}/{} faces",
                job.snapshot_index, job.face_count
            );
            app.editing.linked_selection_job = Some(job);
            return;
        }
        let face_positions = std::mem::take(&mut job.face_positions);
        let seed_face = job.seed_face;
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let result =
                std::panic::catch_unwind(|| linked_face_component(&face_positions, seed_face))
                    .map_err(|panic| {
                        panic
                            .downcast_ref::<&str>()
                            .map(|message| (*message).to_string())
                            .or_else(|| panic.downcast_ref::<String>().cloned())
                            .unwrap_or_else(|| "unknown linked-selection worker panic".to_string())
                    })
                    .and_then(|result| result);
            let _ = tx.send(result);
        });
        job.rx = Some(rx);
        app.status_message = "Finding connected geometry in the background...".to_string();
        app.editing.linked_selection_job = Some(job);
        return;
    }

    if job.result.is_none() {
        let Some(rx) = job.rx.as_ref() else {
            app.status_message = "Linked-selection worker failed to start".to_string();
            return;
        };
        match rx.try_recv() {
            Ok(Ok(linked)) => job.result = Some(linked),
            Ok(Err(error)) => {
                app.status_message = format!("Linked selection failed: {error}");
                return;
            }
            Err(mpsc::TryRecvError::Empty) => {
                app.editing.linked_selection_job = Some(job);
                return;
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                app.status_message =
                    "Linked-selection worker disconnected unexpectedly".to_string();
                return;
            }
        }
    }

    if !job.apply_started {
        match app.editing.asset.as_mut() {
            Some(EditingAsset::Dff(dff)) => {
                if job.clear_existing && !job.deselect {
                    dff.selected_faces.clear();
                    dff.selected_face = None;
                }
                dff.select_mode = EditingSelectMode::Face;
                dff.selected_edges.clear();
                dff.selected_vertex = None;
                dff.selected_vertices.clear();
            }
            Some(EditingAsset::Col(col)) => {
                if job.clear_existing && !job.deselect {
                    col.selected_faces.clear();
                    col.selected_face = usize::MAX;
                }
                col.select_mode = EditingSelectMode::Face;
                col.selected_edges.clear();
                col.selected_vertex = 0;
                col.selected_vertices.clear();
                col.selected_primitive = None;
            }
            _ => unreachable!("linked-selection asset was checked before applying"),
        }
        job.apply_started = true;
    }

    let linked = job
        .result
        .as_ref()
        .expect("linked-selection result exists while applying");
    let started = Instant::now();
    let mut processed = 0usize;
    while job.apply_index < linked.len()
        && processed < APPLY_BATCH_LIMIT
        && started.elapsed() < FRAME_BUDGET
    {
        let face = linked[job.apply_index];
        let changed = match app.editing.asset.as_mut() {
            Some(EditingAsset::Dff(dff)) => {
                if job.deselect {
                    dff.selected_faces.remove(&face)
                } else {
                    dff.selected_faces.insert(face)
                }
            }
            Some(EditingAsset::Col(col)) => {
                if job.deselect {
                    col.selected_faces.remove(&face)
                } else {
                    col.selected_faces.insert(face)
                }
            }
            _ => false,
        };
        job.changed += usize::from(changed);
        job.apply_index += 1;
        processed += 1;
    }
    if job.apply_index < linked.len() {
        app.status_message = format!(
            "Applying linked selection... {}/{} faces",
            job.apply_index,
            linked.len()
        );
        app.editing.linked_selection_job = Some(job);
        return;
    }

    match app.editing.asset.as_mut() {
        Some(EditingAsset::Dff(dff)) => {
            dff.selected_face = if job.deselect {
                dff.selected_faces.iter().next_back().copied()
            } else {
                if let Some(tri) = dff.raw.triangles.get(job.seed_face) {
                    dff.selected_material = tri.material as usize;
                }
                Some(job.seed_face)
            };
        }
        Some(EditingAsset::Col(col)) => {
            col.selected_face = if job.deselect {
                col.selected_faces
                    .iter()
                    .next_back()
                    .copied()
                    .unwrap_or(usize::MAX)
            } else {
                job.seed_face
            };
        }
        _ => unreachable!("linked-selection asset was checked before finalizing"),
    }
    let action = if job.deselect {
        "Deselected"
    } else {
        "Selected"
    };
    app.status_message = format!(
        "{action} linked {} island: {} face{} changed ({:.1}s)",
        match job.kind {
            EditingLinkedAssetKind::Dff => "DFF",
            EditingLinkedAssetKind::Col => "COL",
        },
        job.changed,
        if job.changed == 1 { "" } else { "s" },
        job.started_at.elapsed().as_secs_f32()
    );
}

pub(crate) fn handle_editing_click(app: &mut AppState, mouse: Vec2) -> bool {
    if app.active_tab != AppTab::Editing {
        return false;
    }
    if handle_editing_txd_material_picker(app, mouse) {
        return true;
    }
    if handle_editing_dff_material_picker(app, mouse) {
        return true;
    }
    if is_mouse_button_released(MouseButton::Left) && app.material_emitters_dirty {
        persist_material_emitter_edit(app, "Updated emitter RGB color");
    }
    refresh_editing_img_paths(app);
    if update_editing_search_input(app, mouse) {
        return true;
    }
    if update_editing_txd_search_input(app, mouse) {
        return true;
    }
    let left = editing_archive_rect();
    if left.contains(mouse) {
        let (_x, wheel_y) = mouse_wheel();
        if wheel_y.abs() > 0.0 {
            let visible = ((left.h - 154.0) / EDIT_ROW_H).floor().max(1.0) as usize;
            let max_scroll = editing_filtered_indices(app).len().saturating_sub(visible) as f32;
            app.editing.scroll = (app.editing.scroll - wheel_y * 3.0).clamp(0.0, max_scroll);
            return true;
        }
    }
    if let Some(EditingAsset::Txd(txd)) = app.editing.asset.as_mut() {
        let list = editing_txd_list_rect();
        let filtered = editing_txd_filtered_indices(txd);
        let visible = (list.h / 32.0).floor().max(1.0) as usize;
        if editing_asset_rect().contains(mouse) && is_key_pressed(KeyCode::Up) {
            let position = filtered
                .iter()
                .position(|index| *index == txd.selected)
                .unwrap_or(0)
                .saturating_sub(1);
            if let Some(index) = filtered.get(position) {
                txd.selected = *index;
                txd.scroll = txd.scroll.min(position as f32);
            }
            editing_update_txd_preview(app);
            return true;
        }
        if editing_asset_rect().contains(mouse) && is_key_pressed(KeyCode::Down) {
            let position = filtered
                .iter()
                .position(|index| *index == txd.selected)
                .unwrap_or(0);
            let next = (position + 1).min(filtered.len().saturating_sub(1));
            if let Some(index) = filtered.get(next) {
                txd.selected = *index;
                if next >= txd.scroll.floor() as usize + visible {
                    txd.scroll = (next + 1).saturating_sub(visible) as f32;
                }
            }
            editing_update_txd_preview(app);
            return true;
        }
    }
    if is_key_pressed(KeyCode::Up) {
        app.editing.selected_row = app.editing.selected_row.saturating_sub(1);
        app.editing.scroll = app.editing.scroll.min(app.editing.selected_row as f32);
        return true;
    }
    if is_key_pressed(KeyCode::Down) {
        let filtered_len = editing_filtered_indices(app).len();
        app.editing.selected_row =
            (app.editing.selected_row + 1).min(filtered_len.saturating_sub(1));
        let visible = ((left.h - 154.0) / EDIT_ROW_H).floor().max(1.0) as usize;
        if app.editing.selected_row >= app.editing.scroll as usize + visible {
            app.editing.scroll = (app.editing.selected_row + 1).saturating_sub(visible) as f32;
        }
        return true;
    }
    if matches!(app.editing.asset, Some(EditingAsset::Col(_))) {
        let center = editing_center_rect();
        let (hovered_face, hovered_vertex) =
            app.editing
                .asset
                .as_ref()
                .map_or((None, None), |asset| match asset {
                    EditingAsset::Col(col) if center.contains(mouse) => {
                        let vertex_hover = editing_hover_col_face_vertex(app, col, center, mouse);
                        let face_hover = vertex_hover
                            .map(|(face, _)| face)
                            .or_else(|| editing_pick_col_face(app, col, center, mouse));
                        (face_hover, vertex_hover.map(|(_, vertex)| vertex))
                    }
                    _ => (None, None),
                });
        if let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() {
            col.hovered_face = hovered_face;
            col.hovered_vertex = hovered_vertex;
        }
    }
    if let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() {
        let (_, wheel_y) = mouse_wheel();
        if wheel_y.abs() > 0.0 {
            let layout = col_panel_layout(col);
            if layout.face_list.is_some_and(|list| list.contains(mouse)) {
                scroll_editing_col_faces(col, wheel_y);
                return true;
            }
            if layout.content.contains(mouse) {
                col.panel_scroll =
                    (col.panel_scroll - wheel_y * 24.0).clamp(0.0, col_panel_max_scroll(&layout));
                return true;
            }
        }
    }
    if matches!(app.editing.asset, Some(EditingAsset::Dff(_))) {
        let center = editing_center_rect();
        let (hovered_face, hovered_vertex) =
            app.editing
                .asset
                .as_ref()
                .map_or((None, None), |asset| match asset {
                    EditingAsset::Dff(dff)
                        if center.contains(mouse) && dff.fracture_preview_started_at.is_none() =>
                    {
                        let vertex_hover = editing_hover_dff_face_vertex(app, dff, center, mouse);
                        let face_hover = vertex_hover
                            .map(|(face, _)| face)
                            .or_else(|| editing_pick_dff_face(app, dff, center, mouse));
                        (face_hover, vertex_hover.map(|(_, vertex)| vertex))
                    }
                    _ => (None, None),
                });
        if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
            dff.hovered_face = hovered_face;
            dff.hovered_vertex = hovered_vertex;
        }
    }
    if is_key_pressed(KeyCode::L)
        && !is_key_down(KeyCode::LeftControl)
        && !is_key_down(KeyCode::RightControl)
        && !is_key_down(KeyCode::LeftAlt)
        && !is_key_down(KeyCode::RightAlt)
        && editing_center_rect().contains(mouse)
        && matches!(
            app.editing.asset,
            Some(EditingAsset::Dff(_) | EditingAsset::Col(_))
        )
    {
        request_editing_linked_selection(app);
        return true;
    }
    if is_mouse_button_down(MouseButton::Left)
        && let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_ref()
    {
        let emitter = selected_material_emitter(app);
        let layout = dff_panel_layout(dff, emitter, dff_face_emitter_entries(app, &dff.name).len());
        if let Some(bars) = layout.emitter_color {
            for (channel, rect) in bars.iter().enumerate() {
                if rect.contains(mouse) {
                    let value = ((mouse.x - rect.x) / rect.w).clamp(0.0, 1.0);
                    let mut updated = selected_material_emitter(app);
                    if update_selected_emitters(app, |emitter| {
                        emitter.use_material_color = false;
                        emitter.use_temperature = false;
                        match channel {
                            0 => emitter.color.x = value,
                            1 => emitter.color.y = value,
                            _ => emitter.color.z = value,
                        }
                        updated = *emitter;
                    }) {
                        app.material_emitters_dirty = true;
                        app.status_message = format!(
                            "Emitter RGB {:.2}/{:.2}/{:.2}",
                            updated.color.x, updated.color.y, updated.color.z
                        );
                    }
                    return true;
                }
            }
        }
    }
    if !is_mouse_button_pressed(MouseButton::Left) {
        return false;
    }
    let ctrl_down = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
    let shift_down = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
    if editing_box_select_mode_rect().contains(mouse) {
        app.box_select_mode = app.box_select_mode.next();
        app.status_message = format!("Box select mode: {}", app.box_select_mode.label());
        return true;
    }
    if editing_box_select_minus_rect().contains(mouse) {
        app.box_select_distance = (app.box_select_distance * 0.5).max(250.0);
        app.status_message = format!("Box select distance {:.0}", app.box_select_distance);
        return true;
    }
    if editing_box_select_plus_rect().contains(mouse) {
        app.box_select_distance = (app.box_select_distance * 2.0).min(64000.0);
        app.status_message = format!("Box select distance {:.0}", app.box_select_distance);
        return true;
    }
    if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_ref() {
        let center = editing_center_rect();
        if center.contains(mouse) {
            if dff.fracture_preview_started_at.is_some() {
                app.status_message =
                    "Reset the fracture preview before editing the intact mesh".to_string();
                return true;
            }
            if ctrl_down {
                return false;
            }
            if app.transform_mode != TransformMode::Select
                && gizmo_axis_direct_at(app, center, mouse).is_some()
            {
                return false;
            }
            let picked = (dff.select_mode == EditingSelectMode::Vertex)
                .then(|| editing_hover_dff_face_vertex(app, dff, center, mouse))
                .flatten();
            if let Some((face, vertex)) = picked {
                if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
                    dff_set_single_face_selection(dff, face);
                    dff.selected_edges.clear();
                    dff.selected_vertex = Some(vertex);
                    if shift_down {
                        if !dff.selected_vertices.insert(vertex) {
                            dff.selected_vertices.remove(&vertex);
                        }
                    } else {
                        dff.selected_vertices.clear();
                        dff.selected_vertices.insert(vertex);
                    }
                    if let Some(tri) = dff.raw.triangles.get(face) {
                        dff.selected_material = tri.material as usize;
                    }
                }
                app.status_message = format!("Selected DFF face {face}, vertex {vertex}");
            } else if dff.select_mode == EditingSelectMode::Vertex {
                if editing_pick_dff_face(app, dff, center, mouse).is_none() && !shift_down {
                    if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
                        dff.selected_face = None;
                        dff.selected_faces.clear();
                        dff.selected_edges.clear();
                        dff.selected_vertex = None;
                        dff.selected_vertices.clear();
                    }
                    app.status_message = "Deselected DFF selection".to_string();
                }
            } else if dff.select_mode == EditingSelectMode::Face {
                let picked_face = editing_pick_dff_face(app, dff, center, mouse);
                if let Some(face) = picked_face {
                    if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
                        if shift_down {
                            dff_toggle_face_selection(dff, face);
                        } else {
                            dff_set_single_face_selection(dff, face);
                        }
                        dff.selected_edges.clear();
                        dff.selected_vertex = None;
                        if let Some(tri) = dff.raw.triangles.get(face) {
                            dff.selected_material = tri.material as usize;
                        }
                        if !shift_down {
                            dff.selected_vertices.clear();
                        }
                    }
                    app.status_message = format!("Selected DFF face {face}");
                } else if !shift_down {
                    if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
                        dff.selected_face = None;
                        dff.selected_faces.clear();
                        dff.selected_vertex = None;
                        dff.selected_vertices.clear();
                    }
                    app.status_message = "Deselected DFF selection".to_string();
                }
            } else if dff.select_mode == EditingSelectMode::Edge {
                if let Some(edge) = editing_pick_dff_edge(app, dff, center, mouse) {
                    if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
                        if shift_down {
                            toggle_edge_selection(&mut dff.selected_edges, edge);
                        } else {
                            dff.selected_edges.clear();
                            dff.selected_edges.insert(edge);
                        }
                        dff.selected_vertex = None;
                        dff.selected_vertices.clear();
                        dff.selected_face = None;
                        dff.selected_faces.clear();
                    }
                    app.status_message = format!("Selected DFF edge v{}-v{}", edge.0, edge.1);
                } else if editing_pick_dff_face(app, dff, center, mouse).is_none() && !shift_down {
                    if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
                        dff.selected_edges.clear();
                    }
                    app.status_message = "Deselected DFF edge selection".to_string();
                }
            }
            return true;
        }
    }
    if let Some(EditingAsset::Col(_)) = app.editing.asset.as_ref() {
        if editing_col_generation_preset_rect().contains(mouse) {
            cycle_collision_generation_preset(app);
            return true;
        }
        if editing_col_shadow_layer_rect().contains(mouse) {
            toggle_editing_col_shadow_layer(app);
            return true;
        }
        if editing_col_generate_shadow_rect().contains(mouse) {
            request_editing_shadow_mesh_generation(app);
            return true;
        }
        if editing_col_overlay_toggle_rect().contains(mouse) {
            let has_overlay = app.editing.asset.as_ref().is_some_and(
                |asset| matches!(asset, EditingAsset::Col(col) if col.dff_overlay.is_some()),
            );
            if !has_overlay {
                refresh_editing_col_dff_overlay(app);
                if let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() {
                    col.dff_overlay_visible = col.dff_overlay.is_some();
                }
            } else if let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() {
                col.dff_overlay_visible = !col.dff_overlay_visible;
                app.status_message = if col.dff_overlay_visible {
                    "DFF overlay visible".to_string()
                } else {
                    "DFF overlay hidden".to_string()
                };
            }
            return true;
        }
        if editing_col_overlay_pick_rect().contains(mouse) {
            pick_selected_row_as_col_dff_overlay(app);
            return true;
        }
        if editing_col_overlay_match_rect().contains(mouse) {
            request_editing_collision_generation(
                app,
                app.collision_generation_preset,
                app.collision_generation_fallback_material,
            );
            return true;
        }
        if editing_col_overlay_clear_rect().contains(mouse) {
            if let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() {
                col.dff_overlay = None;
                col.dff_overlay_name = None;
                col.dff_overlay_visible = false;
            }
            app.status_message = "Cleared COL DFF overlay".to_string();
            return true;
        }
        if editing_col_safe_rect().contains(mouse) {
            let name = app.editing.asset.as_ref().and_then(|asset| {
                let EditingAsset::Col(col) = asset else {
                    return None;
                };
                Some(col.name.clone())
            });
            if let Some(name) = name {
                let safe = !app.safe_collisions.is_safe(&name);
                if app.safe_collisions.set_safe(&name, safe) {
                    app.safe_collisions_dirty = true;
                }
                app.status_message = if safe {
                    format!("{name} marked Safe; whole-scene regeneration will skip it")
                } else {
                    format!("{name} is no longer Safe")
                };
            }
            return true;
        }
        let center = editing_center_rect();
        if center.contains(mouse)
            && !ctrl_down
            && !(app.transform_mode != TransformMode::Select
                && gizmo_axis_direct_at(app, center, mouse).is_some())
        {
            // COL box face handles: clicking a box selects it; clicking a face of the
            // already-selected box starts a drag-resize of that face.
            let box_pick_enabled = app.editing.asset.as_ref().is_some_and(
                |asset| matches!(asset, EditingAsset::Col(col) if col.box_pick_enabled && !col.editing_shadow),
            );
            if let Some(pick) = box_pick_enabled
                .then(|| editing_pick_col_box_face(app, center, mouse))
                .flatten()
            {
                let already = matches!(
                    app.editing.asset.as_ref(),
                    Some(EditingAsset::Col(col))
                        if col.selected_primitive == Some(pick.primitive)
                );
                if already {
                    let before = editing_history_snapshot(app);
                    app.col_box_face_drag = Some(ColBoxFaceDrag {
                        primitive: pick.primitive,
                        axis: pick.axis,
                        side_is_max: pick.side_is_max,
                        center: pick.center,
                        half_extents: pick.half_extents,
                        axis_dir: pick.axis_dir,
                        face_origin: pick.face_origin,
                        start_mouse: mouse,
                        before,
                    });
                    app.camera.looking = false;
                    set_cursor_grab(false);
                    show_mouse(true);
                } else if let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() {
                    col.selected_primitive = Some(pick.primitive);
                    app.status_message = if pick.primitive.kind == CollisionPrimitiveKind::Cuboid {
                        format!("Selected rotated COL box {}", pick.primitive.index + 1)
                    } else {
                        format!("Selected COL box {}", pick.primitive.index + 1)
                    };
                }
                return true;
            }
        }
    }
    if let Some(EditingAsset::Col(col)) = app.editing.asset.as_ref() {
        let center = editing_center_rect();
        if center.contains(mouse) {
            if ctrl_down {
                return false;
            }
            if app.transform_mode != TransformMode::Select
                && gizmo_axis_direct_at(app, center, mouse).is_some()
            {
                return false;
            }
            if let Some(face) = editing_pick_col_face(app, col, center, mouse)
                && let Some(primitive) = col_generated_primitive_for_face(col, face)
            {
                if let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() {
                    col.selected_primitive = Some(primitive);
                    col.selected_faces.clear();
                    col.selected_edges.clear();
                    col.selected_vertices.clear();
                }
                app.status_message = match primitive.kind {
                    CollisionPrimitiveKind::Capsule => {
                        format!("Selected COL capsule {}", primitive.index + 1)
                    }
                    CollisionPrimitiveKind::Cuboid => {
                        format!("Selected rotated COL box {}", primitive.index + 1)
                    }
                    _ => unreachable!(),
                };
                return true;
            }
            let picked = (col.select_mode == EditingSelectMode::Vertex)
                .then(|| editing_hover_col_face_vertex(app, col, center, mouse))
                .flatten();
            if let Some((face, vertex)) = picked {
                if let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() {
                    col.selected_face = face;
                    col.selected_vertex = vertex;
                    col.selected_edges.clear();
                    col.selected_primitive = None;
                    if let Some(face_ref) = col.mesh.faces.get(face) {
                        let vertex_idx =
                            collision_selected_vertex_index_from_face(face_ref, vertex);
                        if shift_down {
                            if !col.selected_vertices.insert(vertex_idx) {
                                col.selected_vertices.remove(&vertex_idx);
                            }
                        } else {
                            col.selected_faces.clear();
                            col.selected_vertices.clear();
                            col.selected_vertices.insert(vertex_idx);
                        }
                    }
                }
                app.status_message = format!("Selected COL face {face}, vertex {}", vertex + 1);
            } else if col.select_mode == EditingSelectMode::Vertex {
                if editing_pick_col_face(app, col, center, mouse).is_none() && !shift_down {
                    if let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() {
                        col_clear_selection(col);
                    }
                    app.status_message = "Deselected COL selection".to_string();
                }
            } else if col.select_mode == EditingSelectMode::Face {
                let picked_face = editing_pick_col_face(app, col, center, mouse);
                if let Some(face) = picked_face {
                    if let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() {
                        if shift_down {
                            col_toggle_face_selection(col, face);
                        } else {
                            col_set_single_face_selection(col, face);
                        }
                        col.selected_edges.clear();
                        col.selected_vertex = 0;
                        if !shift_down {
                            col.selected_vertices.clear();
                        }
                    }
                    app.status_message = format!("Selected COL face {face}");
                } else if !shift_down {
                    if let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() {
                        col_clear_selection(col);
                    }
                    app.status_message = "Deselected COL selection".to_string();
                }
            } else if col.select_mode == EditingSelectMode::Edge {
                if let Some(edge) = editing_pick_col_edge(app, col, center, mouse) {
                    if let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() {
                        if shift_down {
                            toggle_edge_selection(&mut col.selected_edges, edge);
                        } else {
                            col.selected_edges.clear();
                            col.selected_edges.insert(edge);
                        }
                        col.selected_vertices.clear();
                        col.selected_faces.clear();
                        col.selected_primitive = None;
                    }
                    app.status_message = format!("Selected COL edge v{}-v{}", edge.0, edge.1);
                } else if editing_pick_col_face(app, col, center, mouse).is_none() && !shift_down {
                    if let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() {
                        col.selected_edges.clear();
                    }
                    app.status_message = "Deselected COL edge selection".to_string();
                }
            }
            return true;
        }
    }
    if editing_img_prev_rect().contains(mouse) {
        app.editing.selected_img_path = app.editing.selected_img_path.saturating_sub(1);
        return true;
    }
    if editing_img_next_rect().contains(mouse) {
        app.editing.selected_img_path =
            (app.editing.selected_img_path + 1).min(app.editing.img_paths.len().saturating_sub(1));
        return true;
    }
    if editing_img_open_rect().contains(mouse) {
        open_selected_editing_img(app);
        return true;
    }
    if editing_img_choose_rect().contains(mouse) {
        open_editing_img_picker(app);
        return true;
    }
    if editing_img_save_rect().contains(mouse) {
        editing_save_img(app);
        return true;
    }
    if editing_merge_img_rect().contains(mouse) {
        open_editing_merge_img_picker(app);
        return true;
    }
    if editing_select_vertex_mode_rect().contains(mouse) {
        match app.editing.asset.as_mut() {
            Some(EditingAsset::Dff(dff)) => dff.select_mode = EditingSelectMode::Vertex,
            Some(EditingAsset::Col(col)) => col.select_mode = EditingSelectMode::Vertex,
            _ => return false,
        }
        app.status_message = "Editing selection mode: Vertex".to_string();
        return true;
    }
    if editing_select_edge_mode_rect().contains(mouse) {
        match app.editing.asset.as_mut() {
            Some(EditingAsset::Dff(dff)) => dff.select_mode = EditingSelectMode::Edge,
            Some(EditingAsset::Col(col)) => col.select_mode = EditingSelectMode::Edge,
            _ => return false,
        }
        app.status_message = "Editing selection mode: Edge".to_string();
        return true;
    }
    if editing_select_face_mode_rect().contains(mouse) {
        match app.editing.asset.as_mut() {
            Some(EditingAsset::Dff(dff)) => dff.select_mode = EditingSelectMode::Face,
            Some(EditingAsset::Col(col)) => col.select_mode = EditingSelectMode::Face,
            _ => return false,
        }
        app.status_message = "Editing selection mode: Face".to_string();
        return true;
    }
    let filtered = editing_filtered_indices(app);
    let visible = ((left.h - 154.0) / EDIT_ROW_H).floor().max(1.0) as usize;
    for row in 0..visible {
        if editing_row_rect(row).contains(mouse) {
            let idx = app.editing.scroll.floor() as usize + row;
            if idx < filtered.len() {
                app.editing.selected_row = idx;
            }
            return true;
        }
    }
    if editing_open_entry_rect().contains(mouse) {
        editing_open_selected_asset(app);
        return true;
    }
    if editing_add_entry_rect().contains(mouse) {
        open_editing_add_entry_picker(app);
        return true;
    }
    if editing_replace_entry_rect().contains(mouse) {
        open_editing_replace_entry_picker(app);
        return true;
    }
    if editing_delete_entry_rect().contains(mouse) {
        editing_delete_selected_entry(app);
        return true;
    }
    if editing_extract_entry_rect().contains(mouse) {
        open_editing_extract_entry_picker(app);
        return true;
    }
    if let Some(EditingAsset::Txd(txd)) = app.editing.asset.as_mut() {
        let list = editing_txd_list_rect();
        let filtered = editing_txd_filtered_indices(txd);
        let visible = (list.h / 32.0).floor().max(1.0) as usize;
        let start = (txd.scroll.floor() as usize).min(filtered.len().saturating_sub(visible));
        for row in 0..visible {
            let rect = Rect::new(list.x, list.y + row as f32 * 32.0, list.w - 10.0, 28.0);
            if rect.contains(mouse) {
                if let Some(index) = filtered.get(start + row) {
                    txd.selected = *index;
                    editing_update_txd_preview(app);
                }
                return true;
            }
        }
        if editing_txd_add_rect().contains(mouse) {
            open_editing_txd_texture_picker(app, false);
            return true;
        }
        if editing_txd_replace_rect().contains(mouse) {
            open_editing_txd_texture_picker(app, true);
            return true;
        }
        if editing_txd_rename_rect().contains(mouse) {
            open_txd_texture_rename_dialog(app);
            return true;
        }
        if editing_txd_export_all_rect().contains(mouse) {
            open_editing_txd_export_all_picker(app);
            return true;
        }
    }
    let picker_open = matches!(
        app.editing.asset.as_ref(),
        Some(EditingAsset::Dff(dff)) if dff.texture_picker_open
    );
    let uv_anim_picker_open = matches!(
        app.editing.asset.as_ref(),
        Some(EditingAsset::Dff(dff)) if dff.uv_anim_picker_open
    );
    let dff_2dfx_type_picker_open = matches!(
        app.editing.asset.as_ref(),
        Some(EditingAsset::Dff(dff)) if dff.dff_2dfx_type_picker_open
    );
    let dff_2dfx_corona_preset_picker_open = matches!(
        app.editing.asset.as_ref(),
        Some(EditingAsset::Dff(dff)) if dff.dff_2dfx_corona_preset_picker_open
    );
    if dff_2dfx_corona_preset_picker_open {
        let popup = editing_dff_2dfx_corona_preset_picker_rect();
        if editing_dff_2dfx_corona_preset_close_rect().contains(mouse) {
            if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
                dff.dff_2dfx_corona_preset_picker_open = false;
            }
            return true;
        }
        let list = editing_dff_2dfx_corona_preset_list_rect();
        if list.contains(mouse) {
            let row_h = 42.0;
            let row = ((mouse.y - list.y) / row_h).floor() as usize;
            if row < DFF_CORONA_PRESETS.len() {
                let before = editing_history_snapshot(app);
                if add_dff_2dfx_corona_preset(app, row) {
                    commit_editing_history(app, "Add DFF Corona Preset", before);
                }
            }
            return true;
        }
        if !popup.contains(mouse)
            && let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut()
        {
            dff.dff_2dfx_corona_preset_picker_open = false;
        }
        return true;
    }
    if dff_2dfx_type_picker_open {
        let popup = editing_dff_2dfx_type_picker_rect();
        if editing_dff_2dfx_type_picker_close_rect().contains(mouse) {
            if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
                dff.dff_2dfx_type_picker_open = false;
            }
            return true;
        }
        let list = editing_dff_2dfx_type_picker_list_rect();
        if list.contains(mouse) {
            let picked = match app.editing.asset.as_ref() {
                Some(EditingAsset::Dff(dff)) => {
                    let types = dff_2dfx_filtered_types(&dff.dff_2dfx_type_picker_search);
                    let row_h = 28.0;
                    let visible = (list.h / row_h).floor().max(1.0) as usize;
                    let max_start = types.len().saturating_sub(visible) as f32;
                    let start = dff
                        .dff_2dfx_type_picker_scroll
                        .floor()
                        .max(0.0)
                        .min(max_start) as usize;
                    let row = ((mouse.y - list.y) / row_h).floor() as usize;
                    types.get(start + row).map(|(id, _)| *id)
                }
                _ => None,
            };
            if let Some(effect_id) = picked {
                let edits_selected_effect = matches!(
                    app.editing.asset.as_ref(),
                    Some(EditingAsset::Dff(dff)) if dff.selected_2dfx.is_some()
                );
                let before = editing_history_snapshot(app);
                let changed = if edits_selected_effect {
                    set_selected_dff_2dfx_type(app, effect_id)
                } else {
                    add_dff_2dfx_of_type(app, effect_id)
                };
                if changed {
                    commit_editing_history(
                        app,
                        if edits_selected_effect {
                            "Set DFF 2DFX Type"
                        } else {
                            "Add DFF 2DFX"
                        },
                        before,
                    );
                }
            }
            return true;
        }
        if !popup.contains(mouse) {
            if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
                dff.dff_2dfx_type_picker_open = false;
            }
        }
        return true;
    }
    let dff_2dfx_payload_editor_open = matches!(
        app.editing.asset.as_ref(),
        Some(EditingAsset::Dff(dff)) if dff.dff_2dfx_payload_editor_open
    );
    if dff_2dfx_payload_editor_open {
        let popup = editing_dff_2dfx_payload_editor_rect();
        if editing_dff_2dfx_payload_close_rect().contains(mouse) {
            if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
                dff.dff_2dfx_payload_editor_open = false;
            }
            return true;
        }
        if editing_dff_2dfx_payload_apply_rect().contains(mouse) {
            let before = editing_history_snapshot(app);
            if apply_selected_dff_2dfx_payload_hex(app) {
                commit_editing_history(app, "Edit DFF 2DFX Payload", before);
            }
            return true;
        }
        let particle_picker_open = matches!(
            app.editing.asset.as_ref(),
            Some(EditingAsset::Dff(dff)) if dff_2dfx_active_payload_is_particle_name(dff)
        );
        if particle_picker_open {
            let picker = editing_dff_2dfx_particle_picker_rect();
            if picker.contains(mouse) {
                let picked = match app.editing.asset.as_ref() {
                    Some(EditingAsset::Dff(dff)) => {
                        let names = dff_2dfx_particle_name_options(app, dff);
                        let row_h = 26.0;
                        let visible = (picker.h / row_h).floor().max(1.0) as usize;
                        let max_start = names.len().saturating_sub(visible) as f32;
                        let start = dff
                            .dff_2dfx_particle_picker_scroll
                            .floor()
                            .max(0.0)
                            .min(max_start) as usize;
                        let row = ((mouse.y - picker.y) / row_h).floor() as usize;
                        names.get(start + row).cloned()
                    }
                    _ => None,
                };
                if let Some(name) = picked {
                    if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
                        if let Some(active) = dff.dff_2dfx_payload_active_field {
                            if let Some(value) = dff.dff_2dfx_payload_fields.get_mut(active) {
                                *value = name;
                            }
                        }
                    }
                }
                return true;
            }
        }
        let list = editing_dff_2dfx_payload_text_rect();
        if list.contains(mouse) {
            if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
                let row_h = 34.0;
                let visible = (list.h / row_h).floor().max(1.0) as usize;
                let max_start = dff.dff_2dfx_payload_fields.len().saturating_sub(visible) as f32;
                let start = dff
                    .dff_2dfx_payload_field_scroll
                    .floor()
                    .max(0.0)
                    .min(max_start) as usize;
                let row = ((mouse.y - list.y) / row_h).floor() as usize;
                let idx = start + row;
                if idx < dff.dff_2dfx_payload_fields.len() {
                    dff.dff_2dfx_payload_active_field = Some(idx);
                    dff.dff_2dfx_particle_picker_scroll = 0.0;
                }
            }
            return true;
        }
        if !popup.contains(mouse) {
            if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
                dff.dff_2dfx_payload_editor_open = false;
            }
        }
        return true;
    }
    if uv_anim_picker_open {
        let popup = editing_dff_uv_anim_picker_rect();
        if editing_dff_uv_anim_picker_close_rect().contains(mouse) {
            if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
                dff.uv_anim_picker_open = false;
            }
            return true;
        }
        if editing_dff_uv_anim_picker_apply_rect().contains(mouse) {
            let typed = match app.editing.asset.as_ref() {
                Some(EditingAsset::Dff(dff)) => dff.uv_anim_picker_search.clone(),
                _ => String::new(),
            };
            let before = editing_history_snapshot(app);
            if assign_selected_material_uv_animation(app, &typed) {
                commit_editing_history(app, "Assign UV Animation", before);
            }
            return true;
        }
        if editing_dff_uv_anim_picker_gif_rect().contains(mouse) {
            if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
                dff.uv_anim_picker_open = false;
            }
            start_dff_gif_anim_browse(app);
            return true;
        }
        let list = editing_dff_uv_anim_picker_list_rect();
        if list.contains(mouse) {
            let picked = match app.editing.asset.as_ref() {
                Some(EditingAsset::Dff(dff)) => {
                    let names = editing_dff_filtered_uv_anim_options(dff);
                    let row_h = 26.0;
                    let visible = (list.h / row_h).floor().max(1.0) as usize;
                    let max_start = names.len().saturating_sub(visible) as f32;
                    let start = dff.uv_anim_picker_scroll.floor().max(0.0).min(max_start) as usize;
                    let row = ((mouse.y - list.y) / row_h).floor() as usize;
                    names.get(start + row).cloned()
                }
                _ => None,
            };
            if let Some(name) = picked {
                let before = editing_history_snapshot(app);
                if assign_selected_material_uv_animation(app, &name) {
                    commit_editing_history(app, "Assign UV Animation", before);
                }
            }
            return true;
        }
        if !popup.contains(mouse) {
            if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
                dff.uv_anim_picker_open = false;
            }
        }
        return true;
    }
    if picker_open {
        let popup = editing_dff_texture_picker_rect();
        if editing_dff_texture_picker_close_rect().contains(mouse) {
            if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
                dff.texture_picker_open = false;
            }
            return true;
        }
        let list = editing_dff_texture_picker_list_rect();
        if list.contains(mouse) {
            let scroll = match app.editing.asset.as_ref() {
                Some(EditingAsset::Dff(dff)) => dff.texture_picker_scroll,
                _ => 0.0,
            };
            let edits_material = matches!(
                app.editing.asset.as_ref(),
                Some(EditingAsset::Dff(dff)) if dff.texture_picker_edits_material
            );
            let names = editing_dff_picker_texture_names(app);
            let visible = (list.h / DFF_TEXTURE_PICKER_ROW_H).floor().max(1.0) as usize;
            let max_start = names.len().saturating_sub(visible) as f32;
            let start = scroll.floor().max(0.0).min(max_start) as usize;
            let row = ((mouse.y - list.y) / DFF_TEXTURE_PICKER_ROW_H).floor() as usize;
            if let Some(name) = names.get(start + row).cloned() {
                let before = editing_history_snapshot(app);
                let changed = if edits_material {
                    editing_set_selected_dff_material_texture(app, &name)
                } else {
                    reassign_selected_dff_faces_texture(app, &name)
                };
                if changed {
                    commit_editing_history(
                        app,
                        if edits_material {
                            "Set DFF Material Texture"
                        } else {
                            "Create DFF Material"
                        },
                        before,
                    );
                }
            }
            return true;
        }
        if !popup.contains(mouse) {
            if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
                dff.texture_picker_open = false;
            }
        }
        return true;
    }
    let selected_emitter = selected_material_emitter(app);
    let lighting_entry_count = match app.editing.asset.as_ref() {
        Some(EditingAsset::Dff(dff)) => dff_face_emitter_entries(app, &dff.name).len(),
        _ => 0,
    };
    if let Some(layout) = match app.editing.asset.as_ref() {
        Some(EditingAsset::Dff(dff)) => Some(dff_panel_layout(
            dff,
            selected_emitter,
            lighting_entry_count,
        )),
        _ => None,
    } {
        // Pinned footer buttons.
        if layout.generate_collision.contains(mouse) {
            request_editing_collision_generation(
                app,
                app.collision_generation_preset,
                app.collision_generation_fallback_material,
            );
            return true;
        }
        if layout.flip_normals.contains(mouse) {
            editing_flip_dff_normals(app);
            return true;
        }
        if layout.stage.contains(mouse) {
            editing_confirm_normalized_rewrite_and_stage(app);
            return true;
        }
        if layout.content.contains(mouse) {
            // Section headers toggle collapse.
            for section in DFF_SECTIONS {
                let idx = section as usize;
                if layout.headers[idx].contains(mouse) {
                    if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
                        dff.panel_collapsed[idx] = !dff.panel_collapsed[idx];
                        let max_scroll = dff_panel_max_scroll(&dff_panel_layout(
                            dff,
                            selected_emitter,
                            lighting_entry_count,
                        ));
                        dff.panel_scroll = dff.panel_scroll.min(max_scroll);
                    }
                    return true;
                }
            }
            // 2DFX rows.
            for (idx, rect) in layout.rows_2dfx.iter().enumerate() {
                if rect.contains(mouse) {
                    if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
                        dff.selected_2dfx = Some(idx);
                        dff.selected_face = None;
                        dff.selected_faces.clear();
                        dff.selected_edges.clear();
                        dff.selected_vertex = None;
                        dff.selected_vertices.clear();
                    }
                    return true;
                }
            }
            if layout.add_2dfx.is_some_and(|rect| rect.contains(mouse)) {
                if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
                    dff.selected_2dfx = None;
                    dff.dff_2dfx_type_picker_open = true;
                    dff.dff_2dfx_corona_preset_picker_open = false;
                    dff.dff_2dfx_type_picker_search.clear();
                    dff.dff_2dfx_type_picker_scroll = 0.0;
                    dff.dff_2dfx_payload_editor_open = false;
                }
                return true;
            }
            if layout
                .add_2dfx_corona_preset
                .is_some_and(|rect| rect.contains(mouse))
            {
                if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
                    dff.dff_2dfx_corona_preset_picker_open = true;
                    dff.dff_2dfx_type_picker_open = false;
                    dff.dff_2dfx_payload_editor_open = false;
                }
                return true;
            }
            if layout.delete_2dfx.is_some_and(|rect| rect.contains(mouse)) {
                let before = editing_history_snapshot(app);
                editing_delete_selected_dff_2dfx(app);
                commit_editing_history(app, "Delete DFF 2DFX", before);
                return true;
            }
            if layout.type_2dfx.is_some_and(|rect| rect.contains(mouse)) {
                if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
                    dff.dff_2dfx_type_picker_open = true;
                    dff.dff_2dfx_corona_preset_picker_open = false;
                    dff.dff_2dfx_type_picker_search.clear();
                    dff.dff_2dfx_type_picker_scroll = 0.0;
                    dff.dff_2dfx_payload_editor_open = false;
                }
                return true;
            }
            if layout.payload_2dfx.is_some_and(|rect| rect.contains(mouse)) {
                open_selected_dff_2dfx_payload_editor(app);
                return true;
            }
            if layout
                .regenerate_2dfx_coronas
                .is_some_and(|rect| rect.contains(mouse))
            {
                let before = editing_history_snapshot(app);
                if regenerate_dff_2dfx_coronas(app) {
                    commit_editing_history(app, "Regenerate DFF 2DFX Coronas", before);
                }
                return true;
            }
            for (group, rect) in layout.fracture_rows.iter().enumerate() {
                if rect.contains(mouse) {
                    select_editing_dff_fracture_zone(app, group);
                    return true;
                }
            }
            if layout
                .generate_fractures
                .is_some_and(|rect| rect.contains(mouse))
            {
                request_automatic_fracture_generation(app);
                return true;
            }
            if layout
                .manual_fracture_zone
                .is_some_and(|rect| rect.contains(mouse))
            {
                request_manual_fracture_zone(app);
                return true;
            }
            if layout
                .fracture_origin
                .is_some_and(|rect| rect.contains(mouse))
            {
                toggle_editing_dff_fracture_origin(app);
                return true;
            }
            if layout
                .clear_fractures
                .is_some_and(|rect| rect.contains(mouse))
            {
                clear_editing_dff_fractures(app);
                return true;
            }
            if layout
                .simulate_fractures
                .is_some_and(|rect| rect.contains(mouse))
            {
                toggle_editing_dff_fracture_preview(app);
                return true;
            }
            if layout.generate_lod.is_some_and(|rect| rect.contains(mouse)) {
                request_editing_dff_lod(app);
                return true;
            }
            // Face-lighting rows select the complete linked face set.
            for (row, rect) in layout.lighting_rows.iter().enumerate() {
                if !rect.contains(mouse) {
                    continue;
                }
                let entry = match app.editing.asset.as_ref() {
                    Some(EditingAsset::Dff(dff)) => {
                        dff_face_emitter_entries(app, &dff.name).get(row).cloned()
                    }
                    _ => None,
                };
                if let Some((_, faces, _)) = entry
                    && let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut()
                {
                    dff.select_mode = EditingSelectMode::Face;
                    dff.selected_faces = faces.iter().copied().collect();
                    dff.selected_face = faces.last().copied();
                    dff.selected_edges.clear();
                    dff.selected_vertex = None;
                    dff.selected_vertices.clear();
                    dff.selected_2dfx = None;
                    if let Some(material) = dff
                        .selected_face
                        .and_then(|face| dff.raw.triangles.get(face))
                        .map(|triangle| triangle.material as usize)
                    {
                        dff.selected_material = material;
                    }
                    app.status_message = format!(
                        "Selected face lighting {} ({} face{})",
                        row + 1,
                        faces.len(),
                        if faces.len() == 1 { "" } else { "s" }
                    );
                }
                return true;
            }
            // Material rows.
            if let Some(list) = layout.material_list {
                if list.contains(mouse) {
                    if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
                        let total = dff_material_slot_count(&dff.raw);
                        let visible = layout.material_visible.max(1);
                        let max_start = total.saturating_sub(visible);
                        let start = (dff.material_scroll.floor().max(0.0) as usize).min(max_start);
                        let row = ((mouse.y - list.y) / DFF_MAT_ROW_H).floor().max(0.0) as usize;
                        let material = start + row;
                        if material < total {
                            dff.selected_material = material;
                            dff.selected_2dfx = None;
                        }
                    }
                    return true;
                }
            }
            if layout.view_texture.is_some_and(|rect| rect.contains(mouse)) {
                open_dff_texture_view_dialog(app);
                return true;
            }
            if layout
                .rename_texture
                .is_some_and(|rect| rect.contains(mouse))
            {
                open_dff_texture_rename_dialog(app);
                return true;
            }
            if layout
                .duplicate_texture
                .is_some_and(|rect| rect.contains(mouse))
            {
                open_dff_texture_duplicate_dialog(app);
                return true;
            }
            if layout
                .set_material_texture_from_txd
                .is_some_and(|rect| rect.contains(mouse))
            {
                open_dff_material_texture_picker(app);
                return true;
            }
            if layout
                .set_material_texture_browse
                .is_some_and(|rect| rect.contains(mouse))
            {
                start_dff_material_texture_browse(app);
                return true;
            }
            if layout
                .new_material_for_faces
                .is_some_and(|rect| rect.contains(mouse))
            {
                let before = editing_history_snapshot(app);
                if editing_create_material_for_selected_faces(app) {
                    commit_editing_history(app, "Create DFF Material", before);
                }
                return true;
            }
            if layout
                .assign_material_to_faces
                .is_some_and(|rect| rect.contains(mouse))
            {
                let before = editing_history_snapshot(app);
                if editing_assign_selected_material_to_faces(app) {
                    commit_editing_history(app, "Assign DFF Material", before);
                }
                return true;
            }
            if layout
                .delete_unused_material
                .is_some_and(|rect| rect.contains(mouse))
            {
                let before = editing_history_snapshot_with_material_sidecars(app);
                if editing_remove_unused_dff_materials(app) {
                    commit_editing_history(app, "Remove Unused DFF Materials", before);
                }
                return true;
            }
            if let Some(presets) = layout.material_color_presets {
                for (idx, rect) in presets.iter().enumerate() {
                    if rect.contains(mouse) {
                        let before = editing_history_snapshot(app);
                        if editing_apply_dff_material_preset(
                            app,
                            Some(DFF_MATERIAL_COLOR_PRESETS[idx]),
                            None,
                        ) {
                            commit_editing_history(app, "Set DFF Material Color", before);
                        }
                        return true;
                    }
                }
            }
            if let Some(presets) = layout.material_alpha_presets {
                for (idx, rect) in presets.iter().enumerate() {
                    if rect.contains(mouse) {
                        let before = editing_history_snapshot(app);
                        if editing_apply_dff_material_preset(
                            app,
                            None,
                            Some(DFF_MATERIAL_ALPHA_PRESETS[idx]),
                        ) {
                            commit_editing_history(app, "Set DFF Material Alpha", before);
                        }
                        return true;
                    }
                }
            }
            if layout
                .collision_material
                .is_some_and(|rect| rect.contains(mouse))
            {
                if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
                    let texture = dff
                        .raw
                        .material_textures
                        .get(dff.selected_material)
                        .map(String::as_str)
                        .unwrap_or("");
                    if texture.trim().is_empty() {
                        app.status_message =
                            "The selected DFF material has no texture to classify.".to_string();
                    } else {
                        dff.collision_material_picker_open = true;
                        dff.collision_material_picker_search.clear();
                        dff.collision_material_picker_scroll = 0.0;
                        if dff.txd_context.is_none() {
                            dff.collision_material_picker_scope =
                                CollisionMaterialAssignmentScope::GlobalName;
                        }
                        drain_text_input();
                    }
                }
                return true;
            }
            if layout
                .shadow_casting_toggle
                .is_some_and(|rect| rect.contains(mouse))
            {
                toggle_selected_shadow_casting(app);
                return true;
            }
            if layout
                .shadow_casting_scope
                .is_some_and(|rect| rect.contains(mouse))
            {
                toggle_selected_shadow_casting_scope(app);
                return true;
            }
            if layout
                .emitter_toggle
                .is_some_and(|rect| rect.contains(mouse))
            {
                let enabled = !selected_material_emitter(app).enabled;
                if update_selected_emitters(app, |emitter| emitter.enabled = enabled) {
                    persist_material_emitter_edit(app, "Updated material light emitter");
                }
                return true;
            }
            if layout
                .emitter_source
                .is_some_and(|rect| rect.contains(mouse))
            {
                if update_selected_emitters(app, |emitter| {
                    if emitter.use_material_color {
                        emitter.use_material_color = false;
                        emitter.use_temperature = false;
                    } else if !emitter.use_temperature {
                        emitter.use_temperature = true;
                    } else {
                        emitter.use_temperature = false;
                        emitter.use_material_color = true;
                    }
                }) {
                    persist_material_emitter_edit(app, "Updated emitter color source");
                }
                return true;
            }
            if layout
                .emitter_scope
                .is_some_and(|rect| rect.contains(mouse))
            {
                toggle_selected_material_emitter_scope(app);
                return true;
            }
            if layout
                .emitter_cast_mode
                .is_some_and(|rect| rect.contains(mouse))
            {
                let mode = match selected_material_emitter(app).cast_mode {
                    MaterialEmitterCastMode::Face => MaterialEmitterCastMode::Point,
                    MaterialEmitterCastMode::Point => MaterialEmitterCastMode::Face,
                };
                if update_selected_emitters(app, |emitter| emitter.cast_mode = mode) {
                    persist_material_emitter_edit(
                        app,
                        match mode {
                            MaterialEmitterCastMode::Face => "Emitter casting mode changed to Face",
                            MaterialEmitterCastMode::Point => {
                                "Emitter casting mode changed to Point"
                            }
                        },
                    );
                }
                return true;
            }
            if layout.emitter_day.is_some_and(|rect| rect.contains(mouse)) {
                let enabled = !selected_material_emitter(app).day;
                if update_selected_emitters(app, |emitter| emitter.day = enabled) {
                    persist_material_emitter_edit(app, "Updated emitter day mode");
                }
                return true;
            }
            if layout
                .emitter_night
                .is_some_and(|rect| rect.contains(mouse))
            {
                let enabled = !selected_material_emitter(app).night;
                if update_selected_emitters(app, |emitter| emitter.night = enabled) {
                    persist_material_emitter_edit(app, "Updated emitter night mode");
                }
                return true;
            }
            if layout
                .emitter_inversed
                .is_some_and(|rect| rect.contains(mouse))
            {
                let inversed = !selected_material_emitter(app).emit_inversed;
                if update_selected_emitters(app, |emitter| emitter.emit_inversed = inversed) {
                    persist_material_emitter_edit(app, "Updated inversed emission");
                }
                return true;
            }
            // Face texture.
            if layout.tex_from_txd.is_some_and(|rect| rect.contains(mouse)) {
                open_dff_face_texture_picker(app);
                return true;
            }
            if layout.tex_browse.is_some_and(|rect| rect.contains(mouse)) {
                start_dff_face_texture_browse(app);
                return true;
            }
            // Material animation.
            if layout.anim_assign.is_some_and(|rect| rect.contains(mouse)) {
                if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
                    dff.uv_anim_picker_open = true;
                    dff.uv_anim_picker_search.clear();
                    dff.uv_anim_picker_scroll = 0.0;
                    dff.texture_picker_open = false;
                }
                return true;
            }
            if layout.anim_clear.is_some_and(|rect| rect.contains(mouse)) {
                let before = editing_history_snapshot(app);
                if clear_selected_material_uv_animation(app) {
                    commit_editing_history(app, "Clear UV Animation", before);
                }
                return true;
            }
            if let Some(motion) = layout.anim_motion {
                for (idx, rect) in motion.iter().enumerate() {
                    if rect.contains(mouse) {
                        let before = editing_history_snapshot(app);
                        if adjust_selected_material_uv_animation_motion(app, idx / 2, idx % 2 == 1)
                        {
                            commit_editing_history(app, "Adjust UV Animation", before);
                        }
                        return true;
                    }
                }
            }
            // UV tools.
            if let Some(nudge) = layout.uv_nudge {
                for (idx, rect) in nudge.iter().enumerate() {
                    if rect.contains(mouse) {
                        let sign = if idx % 2 == 1 { 0.05 } else { -0.05 };
                        let delta = if idx / 2 == 0 {
                            (sign, 0.0)
                        } else {
                            (0.0, sign)
                        };
                        let before = editing_history_snapshot(app);
                        if editing_nudge_selected_dff_uvs(app, delta.0, delta.1) {
                            commit_editing_history(app, "Nudge DFF UVs", before);
                        }
                        return true;
                    }
                }
            }
            if let Some(scale) = layout.uv_scale {
                for (idx, rect) in scale.iter().enumerate() {
                    if rect.contains(mouse) {
                        let factor = if idx == 0 { 1.0 / 1.1 } else { 1.1 };
                        let before = editing_history_snapshot(app);
                        if editing_transform_selected_dff_uvs(app, factor, 0.0) {
                            commit_editing_history(app, "Scale DFF UVs", before);
                        }
                        return true;
                    }
                }
            }
            if let Some(rotate) = layout.uv_rotate {
                for (idx, rect) in rotate.iter().enumerate() {
                    if rect.contains(mouse) {
                        let degrees = if idx == 0 { -15.0 } else { 15.0 };
                        let before = editing_history_snapshot(app);
                        if editing_transform_selected_dff_uvs(app, 1.0, degrees) {
                            commit_editing_history(app, "Rotate DFF UVs", before);
                        }
                        return true;
                    }
                }
            }
            if layout
                .uv_unwrap_face
                .is_some_and(|rect| rect.contains(mouse))
            {
                let before = editing_history_snapshot(app);
                if editing_unwrap_selected_dff_uvs(app, false) {
                    commit_editing_history(app, "Unwrap DFF UVs", before);
                }
                return true;
            }
            if layout
                .uv_unwrap_material
                .is_some_and(|rect| rect.contains(mouse))
            {
                let before = editing_history_snapshot(app);
                if editing_unwrap_selected_dff_uvs(app, true) {
                    commit_editing_history(app, "Unwrap DFF Material UVs", before);
                }
                return true;
            }
            // Mesh tools.
            if layout.make_face.is_some_and(|rect| rect.contains(mouse)) {
                let before = editing_history_snapshot(app);
                if editing_make_face_from_selected_vertices(app) {
                    commit_editing_history(app, "Make DFF Face", before);
                }
                return true;
            }
            if layout.delete_face.is_some_and(|rect| rect.contains(mouse)) {
                let before = editing_history_snapshot(app);
                editing_delete_selected_dff_face(app);
                commit_editing_history(app, "Delete DFF Face", before);
                return true;
            }
            if layout
                .delete_vertex
                .is_some_and(|rect| rect.contains(mouse))
            {
                let before = editing_history_snapshot(app);
                editing_delete_selected_dff_vertex(app);
                commit_editing_history(app, "Delete DFF Vertex", before);
                return true;
            }
            if layout
                .delete_material_faces
                .is_some_and(|rect| rect.contains(mouse))
            {
                editing_delete_selected_material(app);
                return true;
            }
            if layout
                .extrude_selection
                .is_some_and(|rect| rect.contains(mouse))
            {
                let before = editing_history_snapshot(app);
                if editing_extrude_selected(app) {
                    commit_editing_history(app, "Extrude DFF Selection", before);
                }
                return true;
            }
            if layout
                .merge_selected
                .is_some_and(|rect| rect.contains(mouse))
            {
                open_dff_merge_choice_dialog(app);
                return true;
            }
            if layout
                .merge_distance
                .is_some_and(|rect| rect.contains(mouse))
            {
                let before = editing_history_snapshot(app);
                if editing_merge_vertices_by_distance(app, MERGE_BY_DISTANCE_DEFAULT) {
                    commit_editing_history(app, "Merge DFF By Distance", before);
                }
                return true;
            }
            if layout.subdivide.is_some_and(|rect| rect.contains(mouse)) {
                let before = editing_history_snapshot(app);
                if editing_subdivide_selected_dff_faces(app) {
                    commit_editing_history(app, "Subdivide DFF Faces", before);
                }
                return true;
            }
            if layout
                .duplicate_faces
                .is_some_and(|rect| rect.contains(mouse))
            {
                let before = editing_history_snapshot(app);
                if editing_duplicate_selected_dff_faces(app) {
                    commit_editing_history(app, "Duplicate DFF Faces", before);
                }
                return true;
            }
            if layout
                .duplicate_material
                .is_some_and(|rect| rect.contains(mouse))
            {
                let before = editing_history_snapshot_with_material_sidecars(app);
                if editing_duplicate_selected_dff_material(app) {
                    commit_editing_history(app, "Duplicate DFF Material", before);
                }
                return true;
            }
            if layout
                .separate_faces
                .is_some_and(|rect| rect.contains(mouse))
            {
                open_dff_separate_dialog(app);
                return true;
            }
            if layout
                .pivot_to_selection
                .is_some_and(|rect| rect.contains(mouse))
            {
                editing_pivot_to_selection(app);
                return true;
            }
            if layout
                .pivot_to_bounds
                .is_some_and(|rect| rect.contains(mouse))
            {
                editing_pivot_to_bounds(app);
                return true;
            }
            // Boolean cutter.
            if layout.cutter_add.is_some_and(|rect| rect.contains(mouse)) {
                editing_add_dff_boolean_box(app);
                return true;
            }
            if layout.cutter_clear.is_some_and(|rect| rect.contains(mouse)) {
                editing_clear_dff_boolean_box(app);
                return true;
            }
            if let Some(resize) = layout.cutter_resize {
                for (idx, rect) in resize.iter().enumerate() {
                    if rect.contains(mouse) {
                        let delta = if idx % 2 == 1 { 0.25 } else { -0.25 };
                        editing_resize_dff_boolean_box(app, idx / 2, delta);
                        return true;
                    }
                }
            }
        }
    }
    if let Some(layout) = match app.editing.asset.as_ref() {
        Some(EditingAsset::Col(col)) => Some(col_panel_layout(col)),
        _ => None,
    } {
        // Pinned footer button.
        if layout.stage.contains(mouse) {
            editing_stage_col_asset(app);
            return true;
        }
        if layout.content.contains(mouse) {
            // Section headers toggle collapse.
            for section in COL_SECTIONS {
                let idx = section as usize;
                if layout.headers[idx].contains(mouse) {
                    if let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() {
                        col.panel_collapsed[idx] = !col.panel_collapsed[idx];
                        let max_scroll = col_panel_max_scroll(&col_panel_layout(col));
                        col.panel_scroll = col.panel_scroll.min(max_scroll);
                    }
                    return true;
                }
            }
            // Primitive rows.
            for (row, rect) in layout.primitive_rows.iter().enumerate() {
                if rect.contains(mouse) {
                    if let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() {
                        let visible = layout.primitive_visible.max(1);
                        let primitives = col_primitive_selections(col);
                        let total = primitives.len();
                        let start = col
                            .primitive_scroll
                            .floor()
                            .max(0.0)
                            .min(total.saturating_sub(visible) as f32)
                            as usize;
                        let primitive_idx = start + row;
                        col.selected_primitive = primitives.get(primitive_idx).copied();
                        col.selected_vertices.clear();
                        col.selected_faces.clear();
                        app.status_message = "Selected COL primitive".to_string();
                    }
                    return true;
                }
            }
            // Face rows.
            if let Some(list) = layout.face_list {
                if list.contains(mouse) {
                    if let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() {
                        let visible = layout.face_visible.max(1);
                        let start = col
                            .face_scroll
                            .floor()
                            .max(0.0)
                            .min(col.mesh.faces.len().saturating_sub(visible) as f32)
                            as usize;
                        let row = ((mouse.y - list.y) / COL_FACE_ROW_H).floor().max(0.0) as usize;
                        let face_idx = start + row;
                        if face_idx < col.mesh.faces.len() {
                            if let Some(primitive) = col_generated_primitive_for_face(col, face_idx)
                            {
                                col.selected_primitive = Some(primitive);
                                col.selected_faces.clear();
                                col.selected_edges.clear();
                                col.selected_vertices.clear();
                            } else {
                                if shift_down {
                                    col_toggle_face_selection(col, face_idx);
                                } else {
                                    col_set_single_face_selection(col, face_idx);
                                }
                                col.selected_vertex = 0;
                                if !shift_down {
                                    col.selected_vertices.clear();
                                }
                            }
                        }
                    }
                    return true;
                }
            }
            if layout.add_sphere.is_some_and(|rect| rect.contains(mouse)) {
                let before = editing_history_snapshot(app);
                if editing_add_col_sphere(app) {
                    commit_editing_history(app, "Add COL Sphere", before);
                }
                return true;
            }
            if layout.add_box.is_some_and(|rect| rect.contains(mouse)) {
                let before = editing_history_snapshot(app);
                if editing_add_col_box(app) {
                    commit_editing_history(app, "Add COL Box", before);
                }
                return true;
            }
            if layout.add_capsule.is_some_and(|rect| rect.contains(mouse)) {
                let before = editing_history_snapshot(app);
                if editing_add_col_capsule(app) {
                    commit_editing_history(app, "Add COL Capsule", before);
                }
                return true;
            }
            if layout
                .duplicate_primitive
                .is_some_and(|rect| rect.contains(mouse))
            {
                let before = editing_history_snapshot(app);
                if editing_duplicate_selected_col_primitive(app) {
                    commit_editing_history(app, "Duplicate COL Primitive", before);
                }
                return true;
            }
            if layout
                .select_same_material
                .is_some_and(|rect| rect.contains(mouse))
            {
                editing_select_all_with_selected_col_material(app);
                return true;
            }
            if layout
                .box_pick_toggle
                .is_some_and(|rect| rect.contains(mouse))
            {
                if let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() {
                    col.box_pick_enabled = !col.box_pick_enabled;
                    if !col.box_pick_enabled {
                        app.col_box_hovered_face = None;
                        app.col_box_face_drag = None;
                    }
                    app.status_message = if col.box_pick_enabled {
                        "COL box viewport picking enabled".to_string()
                    } else {
                        "COL box viewport picking disabled".to_string()
                    };
                }
                return true;
            }
            if layout
                .capsule_edges_toggle
                .is_some_and(|rect| rect.contains(mouse))
            {
                let before = editing_history_snapshot(app);
                if editing_toggle_selected_col_capsule_edges(app) {
                    commit_editing_history(app, "Toggle COL Capsule Edges", before);
                }
                return true;
            }
            if layout.delete_face.is_some_and(|rect| rect.contains(mouse)) {
                let before = editing_history_snapshot(app);
                match app.editing.asset.as_ref() {
                    Some(EditingAsset::Col(col)) if col.selected_primitive.is_some() => {
                        if editing_delete_selected_col_primitive(app) {
                            commit_editing_history(app, "Delete COL Primitive", before);
                        }
                    }
                    Some(EditingAsset::Col(col))
                        if col.select_mode == EditingSelectMode::Vertex =>
                    {
                        editing_delete_selected_col_vertex(app);
                        commit_editing_history(app, "Delete COL Vertex", before);
                    }
                    Some(EditingAsset::Col(col)) if col.select_mode == EditingSelectMode::Edge => {
                        editing_delete_selected_col_edge(app);
                        commit_editing_history(app, "Delete COL Edge", before);
                    }
                    _ => {
                        editing_delete_selected_col_face(app);
                        commit_editing_history(app, "Delete COL Face", before);
                    }
                }
                return true;
            }
            if layout.make_face.is_some_and(|rect| rect.contains(mouse)) {
                let before = editing_history_snapshot(app);
                if editing_make_face_from_selected_vertices(app) {
                    commit_editing_history(app, "Make COL Face", before);
                }
                return true;
            }
            if layout.flip_face.is_some_and(|rect| rect.contains(mouse)) {
                let before = editing_history_snapshot(app);
                if editing_flip_selected_col_face(app) {
                    commit_editing_history(app, "Flip COL Face Normal", before);
                }
                return true;
            }
            if layout
                .merge_distance
                .is_some_and(|rect| rect.contains(mouse))
            {
                let before = editing_history_snapshot(app);
                if editing_merge_vertices_by_distance(app, MERGE_BY_DISTANCE_DEFAULT) {
                    commit_editing_history(app, "Merge COL By Distance", before);
                }
                return true;
            }
            if layout.optimize.is_some_and(|rect| rect.contains(mouse)) {
                let before = editing_history_snapshot(app);
                editing_optimize_col_mesh(app);
                commit_editing_history(app, "Optimize COL", before);
                return true;
            }
            if layout.cleanup.is_some_and(|rect| rect.contains(mouse)) {
                let before = editing_history_snapshot(app);
                editing_cleanup_col_mesh(app);
                commit_editing_history(app, "Clean Up COL Planes", before);
                return true;
            }
            if layout.validate.is_some_and(|rect| rect.contains(mouse)) {
                editing_validate_col_mesh(app);
                return true;
            }
        }
    }
    editing_panel_rect().contains(mouse)
}

fn handle_editing_txd_material_picker(app: &mut AppState, mouse: Vec2) -> bool {
    let is_txd = matches!(app.editing.asset, Some(EditingAsset::Txd(_)));
    if !is_txd {
        return false;
    }
    let picker_open = matches!(
        app.editing.asset.as_ref(),
        Some(EditingAsset::Txd(txd)) if txd.material_picker_open
    );
    if !picker_open {
        if is_mouse_button_pressed(MouseButton::Left)
            && editing_txd_material_button_rect().contains(mouse)
        {
            if let Some(EditingAsset::Txd(txd)) = app.editing.asset.as_mut() {
                if txd.textures.get(txd.selected).is_none() {
                    app.status_message = "Select a TXD texture first".to_string();
                } else {
                    txd.material_picker_open = true;
                    txd.material_picker_search.clear();
                    txd.material_picker_scroll = 0.0;
                    drain_text_input();
                }
            }
            return true;
        }
        return false;
    }
    if is_key_pressed(KeyCode::Escape) {
        if let Some(EditingAsset::Txd(txd)) = app.editing.asset.as_mut() {
            txd.material_picker_open = false;
        }
        return true;
    }
    if let Some(EditingAsset::Txd(txd)) = app.editing.asset.as_mut() {
        let mut changed_search = false;
        while let Some(ch) = get_char_pressed() {
            if !ch.is_control() {
                txd.material_picker_search.push(ch);
                changed_search = true;
            }
        }
        if is_key_pressed(KeyCode::Backspace) {
            txd.material_picker_search.pop();
            changed_search = true;
        }
        if changed_search {
            txd.material_picker_scroll = 0.0;
        }
        if is_mouse_button_down(MouseButton::Left)
            && let Some(scroll) =
                editing_txd_material_scroll_from_mouse(&txd.material_picker_search, mouse)
        {
            txd.material_picker_scroll = scroll;
            return true;
        }
        let (_, wheel) = mouse_wheel();
        if wheel.abs() > 0.0 && editing_txd_material_list_rect().contains(mouse) {
            let visible = (editing_txd_material_list_rect().h / 26.0).floor().max(1.0) as usize;
            let max_scroll = editing_txd_material_filtered(&txd.material_picker_search)
                .len()
                .saturating_sub(visible) as f32;
            txd.material_picker_scroll =
                (txd.material_picker_scroll - wheel * 3.0).clamp(0.0, max_scroll);
            return true;
        }
        let page = (editing_txd_material_list_rect().h / 26.0).floor().max(1.0);
        let max_scroll = editing_txd_material_filtered(&txd.material_picker_search)
            .len()
            .saturating_sub(page as usize) as f32;
        if is_key_pressed(KeyCode::Down) {
            txd.material_picker_scroll = (txd.material_picker_scroll + 1.0).min(max_scroll);
            return true;
        }
        if is_key_pressed(KeyCode::Up) {
            txd.material_picker_scroll = (txd.material_picker_scroll - 1.0).max(0.0);
            return true;
        }
        if is_key_pressed(KeyCode::PageDown) {
            txd.material_picker_scroll = (txd.material_picker_scroll + page).min(max_scroll);
            return true;
        }
        if is_key_pressed(KeyCode::PageUp) {
            txd.material_picker_scroll = (txd.material_picker_scroll - page).max(0.0);
            return true;
        }
    }
    if !is_mouse_button_pressed(MouseButton::Left) {
        return true;
    }
    for scope in CollisionMaterialAssignmentScope::ALL {
        if editing_txd_material_scope_rect(scope).contains(mouse) {
            if let Some(EditingAsset::Txd(txd)) = app.editing.asset.as_mut() {
                let fingerprint = txd
                    .textures
                    .get(txd.selected)
                    .and_then(|entry| entry.fingerprint);
                if collision_material_scope_available(scope, Some(&txd.name), fingerprint) {
                    txd.material_picker_scope = scope;
                }
            }
            return true;
        }
    }
    let picked = app.editing.asset.as_ref().and_then(|asset| {
        let EditingAsset::Txd(txd) = asset else {
            return None;
        };
        let entry = txd.textures.get(txd.selected)?;
        let texture = entry.name.clone();
        let options = editing_txd_material_filtered(&txd.material_picker_search);
        let list = editing_txd_material_list_rect();
        if !list.contains(mouse) {
            return None;
        }
        let visible = (list.h / 26.0).floor().max(1.0) as usize;
        let start = txd
            .material_picker_scroll
            .floor()
            .max(0.0)
            .min(options.len().saturating_sub(visible) as f32) as usize;
        let row = ((mouse.y - list.y) / 26.0).floor().max(0.0) as usize;
        options.get(start + row).map(|(material, _)| {
            (
                txd.name.clone(),
                texture,
                entry.fingerprint,
                txd.material_picker_scope,
                *material,
            )
        })
    });
    if let Some((txd_name, texture, fingerprint, scope, material)) = picked {
        apply_texture_collision_material_class(
            app,
            Some(&txd_name),
            &texture,
            fingerprint,
            scope,
            material,
        );
        return true;
    }
    if !editing_txd_material_picker_rect().contains(mouse) {
        if let Some(EditingAsset::Txd(txd)) = app.editing.asset.as_mut() {
            txd.material_picker_open = false;
        }
    }
    true
}

fn apply_texture_collision_material_class(
    app: &mut AppState,
    txd_name: Option<&str>,
    texture: &str,
    fingerprint: Option<TextureContentFingerprint>,
    scope: CollisionMaterialAssignmentScope,
    choice: CollisionMaterialPickerValue,
) {
    if choice == CollisionMaterialPickerValue::NoCollision {
        let previous = match scope {
            CollisionMaterialAssignmentScope::ExactTxd => {
                txd_name.is_some_and(|txd| app.material_classes.txd_is_no_collision(txd, texture))
            }
            CollisionMaterialAssignmentScope::IdenticalContent => {
                fingerprint.is_some_and(|fingerprint| {
                    app.material_classes.content_is_no_collision(fingerprint)
                })
            }
            CollisionMaterialAssignmentScope::GlobalName => {
                app.material_classes.global_is_no_collision(texture)
            }
        };
        let enabled = !previous;
        let cleared_stale = enabled
            && clear_stale_texture_collision_material_scopes(
                &mut app.material_classes,
                &app.txd_textures,
                txd_name,
                texture,
                fingerprint,
                scope,
            );
        let changed = match scope {
            CollisionMaterialAssignmentScope::ExactTxd => {
                let Some(txd) = txd_name else {
                    app.status_message =
                        "This DFF has no resolved TXD; choose another assignment scope."
                            .to_string();
                    return;
                };
                app.material_classes
                    .set_txd_no_collision(txd, texture, enabled)
            }
            CollisionMaterialAssignmentScope::IdenticalContent => {
                let Some(fingerprint) = fingerprint else {
                    app.status_message =
                        "The selected texture could not be fingerprinted.".to_string();
                    return;
                };
                app.material_classes
                    .set_content_no_collision(fingerprint, enabled)
            }
            CollisionMaterialAssignmentScope::GlobalName => app
                .material_classes
                .set_global_no_collision(texture, enabled),
        };
        if changed || cleared_stale {
            app.material_classes_dirty = true;
        }
        app.status_message = if enabled {
            format!(
                "Excluded {texture} from generated collision ({})",
                scope.label()
            )
        } else {
            format!("Cleared {} no-collision flag for {texture}", scope.label())
        };
        return;
    }
    let CollisionMaterialPickerValue::Material(material) = choice else {
        unreachable!();
    };
    let previous = match scope {
        CollisionMaterialAssignmentScope::ExactTxd => {
            txd_name.and_then(|txd| app.material_classes.txd_material(txd, texture))
        }
        CollisionMaterialAssignmentScope::IdenticalContent => {
            fingerprint.and_then(|fingerprint| app.material_classes.content_material(fingerprint))
        }
        CollisionMaterialAssignmentScope::GlobalName => {
            app.material_classes.global_material(texture)
        }
    };
    let next = (previous != Some(material)).then_some(material);
    let cleared_stale = next.is_some()
        && clear_stale_texture_collision_material_scopes(
            &mut app.material_classes,
            &app.txd_textures,
            txd_name,
            texture,
            fingerprint,
            scope,
        );
    let changed = match scope {
        CollisionMaterialAssignmentScope::ExactTxd => {
            let Some(txd) = txd_name else {
                app.status_message =
                    "This DFF has no resolved TXD; choose another assignment scope.".to_string();
                return;
            };
            app.material_classes.set_txd(txd, texture, next)
        }
        CollisionMaterialAssignmentScope::IdenticalContent => {
            let Some(fingerprint) = fingerprint else {
                app.status_message = "The selected texture could not be fingerprinted.".to_string();
                return;
            };
            app.material_classes.set_content(fingerprint, next)
        }
        CollisionMaterialAssignmentScope::GlobalName => {
            app.material_classes.set_global(texture, next)
        }
    };
    if changed || cleared_stale {
        app.material_classes_dirty = true;
    }
    app.status_message = match next {
        Some(material) => format!(
            "Classified {texture} as {} ({})",
            col_material_label(material),
            scope.label()
        ),
        None => format!("Cleared {} collision material for {texture}", scope.label()),
    };
}

fn clear_stale_texture_collision_material_scopes(
    classes: &mut TextureMaterialClasses,
    txd_textures: &TxdTextureIndex,
    txd_name: Option<&str>,
    texture: &str,
    fingerprint: Option<TextureContentFingerprint>,
    scope: CollisionMaterialAssignmentScope,
) -> bool {
    match scope {
        CollisionMaterialAssignmentScope::ExactTxd => false,
        CollisionMaterialAssignmentScope::IdenticalContent => {
            let Some(fingerprint) = fingerprint else {
                return false;
            };
            let mut exact_keys = classes
                .txd_entries()
                .map(|(txd, texture, _)| (txd.to_string(), texture.to_string()))
                .chain(
                    classes
                        .txd_no_collision_entries()
                        .map(|(txd, texture)| (txd.to_string(), texture.to_string())),
                )
                .filter(|(txd, texture)| {
                    texture_content_fingerprint(txd_textures, texture, Some(txd))
                        == Some(fingerprint)
                })
                .collect::<BTreeSet<_>>();
            if let Some(txd) = txd_name {
                exact_keys.insert((txd.to_string(), texture.to_string()));
            }
            let cleared_exact = exact_keys
                .into_iter()
                .fold(false, |changed, (txd, texture)| {
                    classes.clear_txd(&txd, &texture) || changed
                });
            classes.clear_global(texture) || cleared_exact
        }
        CollisionMaterialAssignmentScope::GlobalName => {
            let exact_keys = classes
                .txd_entries()
                .map(|(txd, assigned_texture, _)| (txd.to_string(), assigned_texture.to_string()))
                .chain(
                    classes
                        .txd_no_collision_entries()
                        .map(|(txd, assigned_texture)| {
                            (txd.to_string(), assigned_texture.to_string())
                        }),
                )
                .filter(|(_, assigned_texture)| assigned_texture.eq_ignore_ascii_case(texture))
                .collect::<BTreeSet<_>>();
            let cleared_exact = exact_keys
                .into_iter()
                .fold(false, |changed, (txd, texture)| {
                    classes.clear_txd(&txd, &texture) || changed
                });
            let mut fingerprints = BTreeSet::new();
            if let Some(fingerprint) = fingerprint {
                fingerprints.insert(fingerprint);
            }
            let texture_key = texture.trim().to_ascii_lowercase();
            if let Some(entries) = txd_textures.get(&texture_key) {
                fingerprints.extend(entries.iter().map(|entry| entry.content_fingerprint));
            }
            fingerprints
                .into_iter()
                .fold(cleared_exact, |changed, fingerprint| {
                    classes.clear_content(fingerprint) || changed
                })
        }
    }
}

fn handle_editing_dff_material_picker(app: &mut AppState, mouse: Vec2) -> bool {
    let picker_open = matches!(
        app.editing.asset.as_ref(),
        Some(EditingAsset::Dff(dff)) if dff.collision_material_picker_open
    );
    if !picker_open {
        return false;
    }
    if is_key_pressed(KeyCode::Escape) {
        if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
            dff.collision_material_picker_open = false;
        }
        return true;
    }
    if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
        let mut changed_search = false;
        while let Some(ch) = get_char_pressed() {
            if !ch.is_control() {
                dff.collision_material_picker_search.push(ch);
                changed_search = true;
            }
        }
        if is_key_pressed(KeyCode::Backspace) {
            dff.collision_material_picker_search.pop();
            changed_search = true;
        }
        if changed_search {
            dff.collision_material_picker_scroll = 0.0;
        }
        if is_mouse_button_down(MouseButton::Left)
            && let Some(scroll) =
                editing_txd_material_scroll_from_mouse(&dff.collision_material_picker_search, mouse)
        {
            dff.collision_material_picker_scroll = scroll;
            return true;
        }
        let (_, wheel) = mouse_wheel();
        if wheel.abs() > 0.0 && editing_txd_material_list_rect().contains(mouse) {
            let visible = (editing_txd_material_list_rect().h / 26.0).floor().max(1.0) as usize;
            let max_scroll = editing_txd_material_filtered(&dff.collision_material_picker_search)
                .len()
                .saturating_sub(visible) as f32;
            dff.collision_material_picker_scroll =
                (dff.collision_material_picker_scroll - wheel * 3.0).clamp(0.0, max_scroll);
            return true;
        }
        let page = (editing_txd_material_list_rect().h / 26.0).floor().max(1.0);
        let max_scroll = editing_txd_material_filtered(&dff.collision_material_picker_search)
            .len()
            .saturating_sub(page as usize) as f32;
        if is_key_pressed(KeyCode::Down) {
            dff.collision_material_picker_scroll =
                (dff.collision_material_picker_scroll + 1.0).min(max_scroll);
            return true;
        }
        if is_key_pressed(KeyCode::Up) {
            dff.collision_material_picker_scroll =
                (dff.collision_material_picker_scroll - 1.0).max(0.0);
            return true;
        }
        if is_key_pressed(KeyCode::PageDown) {
            dff.collision_material_picker_scroll =
                (dff.collision_material_picker_scroll + page).min(max_scroll);
            return true;
        }
        if is_key_pressed(KeyCode::PageUp) {
            dff.collision_material_picker_scroll =
                (dff.collision_material_picker_scroll - page).max(0.0);
            return true;
        }
    }
    if !is_mouse_button_pressed(MouseButton::Left) {
        return true;
    }
    for scope in CollisionMaterialAssignmentScope::ALL {
        if editing_txd_material_scope_rect(scope).contains(mouse) {
            let availability = app.editing.asset.as_ref().and_then(|asset| {
                let EditingAsset::Dff(dff) = asset else {
                    return None;
                };
                Some((
                    dff.txd_context.is_some(),
                    editing_dff_texture_fingerprint(&app.txd_textures, dff).is_some(),
                ))
            });
            let available = availability.is_some_and(|(has_txd, has_fingerprint)| match scope {
                CollisionMaterialAssignmentScope::ExactTxd => has_txd,
                CollisionMaterialAssignmentScope::IdenticalContent => has_fingerprint,
                CollisionMaterialAssignmentScope::GlobalName => true,
            });
            if available {
                if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
                    dff.collision_material_picker_scope = scope;
                }
            } else {
                app.status_message = match scope {
                    CollisionMaterialAssignmentScope::ExactTxd => {
                        "This DFF has no resolved TXD; choose another assignment scope.".to_string()
                    }
                    CollisionMaterialAssignmentScope::IdenticalContent => {
                        "The selected texture could not be fingerprinted.".to_string()
                    }
                    CollisionMaterialAssignmentScope::GlobalName => String::new(),
                };
            }
            return true;
        }
    }
    let picked = app.editing.asset.as_ref().and_then(|asset| {
        let EditingAsset::Dff(dff) = asset else {
            return None;
        };
        let texture = dff
            .raw
            .material_textures
            .get(dff.selected_material)?
            .clone();
        let fingerprint = editing_dff_texture_fingerprint(&app.txd_textures, dff);
        let options = editing_txd_material_filtered(&dff.collision_material_picker_search);
        let list = editing_txd_material_list_rect();
        if !list.contains(mouse) {
            return None;
        }
        let visible = (list.h / 26.0).floor().max(1.0) as usize;
        let start = dff
            .collision_material_picker_scroll
            .floor()
            .max(0.0)
            .min(options.len().saturating_sub(visible) as f32) as usize;
        let row = ((mouse.y - list.y) / 26.0).floor().max(0.0) as usize;
        options.get(start + row).map(|(material, _)| {
            (
                dff.txd_context.clone(),
                texture,
                fingerprint,
                dff.collision_material_picker_scope,
                *material,
            )
        })
    });
    if let Some((txd_name, texture, fingerprint, scope, material)) = picked {
        apply_texture_collision_material_class(
            app,
            txd_name.as_deref(),
            &texture,
            fingerprint,
            scope,
            material,
        );
        return true;
    }
    if !editing_txd_material_picker_rect().contains(mouse) {
        if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
            dff.collision_material_picker_open = false;
        }
    }
    true
}

pub(crate) fn handle_open_editing_material_picker_input(app: &mut AppState, mouse: Vec2) -> bool {
    handle_editing_txd_material_picker(app, mouse) || handle_editing_dff_material_picker(app, mouse)
}

pub(crate) fn editing_material_picker_is_open(app: &AppState) -> bool {
    matches!(
        app.editing.asset.as_ref(),
        Some(EditingAsset::Txd(txd)) if txd.material_picker_open
    ) || matches!(
        app.editing.asset.as_ref(),
        Some(EditingAsset::Dff(dff)) if dff.collision_material_picker_open
    )
}

pub(crate) fn editing_delete_selected_entry(app: &mut AppState) {
    let Some(entry_name) = editing_selected_row(app).map(|row| row.entry.name.clone()) else {
        return;
    };
    let key = editing_key(&entry_name);
    app.editing.deleted_entries.insert(key.clone());
    app.editing.modified_entries.remove(&key);
    app.editing.message = format!("Staged delete for {entry_name}");
}

fn editing_img_save_entries(
    rows: Vec<EditingImgRow>,
    modified_entries: &BTreeMap<String, Vec<u8>>,
    deleted_entries: &BTreeSet<String>,
) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut entries = Vec::with_capacity(rows.len());
    for row in rows {
        let key = editing_key(&row.entry.name);
        if deleted_entries.contains(&key) {
            continue;
        }
        let bytes = if let Some(bytes) = modified_entries.get(&key) {
            bytes.clone()
        } else {
            let bytes = read_img_entry(&row.entry);
            if bytes.len() != row.entry.size as usize {
                return Err(format!(
                    "Could not read source IMG entry {} completely; save aborted",
                    row.entry.name
                ));
            }
            bytes[..row.logical_size.min(bytes.len())].to_vec()
        };
        entries.push((row.entry.name, bytes));
    }
    Ok(entries)
}

fn reconcile_saved_global_asset_staging(
    pending_replacements: &mut BTreeMap<String, (String, Vec<u8>)>,
    pending_txd_writes: &mut HashSet<String>,
    key: &str,
    saved_bytes: &[u8],
) {
    if pending_replacements
        .get(key)
        .is_some_and(|(_, pending_bytes)| pending_bytes == saved_bytes)
    {
        pending_replacements.remove(key);
        pending_txd_writes.remove(key);
    }
}

pub(crate) fn editing_save_img(app: &mut AppState) {
    if app.manual_save_job.is_some()
        || app.editing.save_rx.is_some()
        || app.editing.merge_rx.is_some()
        || app.editing.merge_apply_job.is_some()
        || app.editing.txd_import_rx.is_some()
        || app.editing.txd_refresh_job.is_some()
        || app.autosave_rx.is_some()
        || app.autosave_cleanup_rx.is_some()
        || app.dff_repair_rx.is_some()
        || app.txd_cleanup_job.is_some()
        || app.asset_optimization_scan_rx.is_some()
        || app.asset_optimization_job.is_some()
        || app.object_bounds_fix_job.is_some()
        || app.corona_generation_job.is_some()
        || app.fracture_generation_job.is_some()
        || app.dff_geometry_job.is_some()
        || app.collision_generation_job.is_some()
        || app.shadow_mesh_generation_job.is_some()
        || app.lod_generation_job.is_some()
        || app.instance_lod_removal_job.is_some()
    {
        app.status_message =
            "IMG save cannot start while another asset writer is running".to_string();
        return;
    }
    if app.editing.img_path.is_none() {
        editing_save_loose_asset(app);
        return;
    }
    let Some(path) = app.editing.img_path.clone() else {
        app.status_message = "Open an IMG archive before saving".to_string();
        return;
    };
    if !editing_stage_active_asset_for_save(app) {
        return;
    }
    let camera = app.camera;
    let selected_row = app.editing.selected_row;
    let selected_name = editing_selected_row(app).map(|row| row.entry.name.clone());
    let active_asset_name = app.editing.asset.as_ref().map(|asset| match asset {
        EditingAsset::Txd(txd) => txd.name.clone(),
        EditingAsset::Dff(dff) => dff.name.clone(),
        EditingAsset::Col(col) => col.name.clone(),
    });
    let rows = app.editing.rows.clone();
    let modified_entries = app.editing.modified_entries.clone();
    let deleted_entries = app.editing.deleted_entries.clone();
    let added_entries = app.editing.added_entries.clone();
    let (tx, rx) = mpsc::channel();
    app.editing.save_rx = Some(rx);
    app.status_message = format!("Saving {} in the background...", path.display());
    thread::spawn(move || {
        let result = (|| {
            let entries = editing_img_save_entries(rows, &modified_entries, &deleted_entries)?;
            let backup = safe_write_img_archive(&path, &entries)?;
            let refreshed_rows = load_editing_img_rows(&path)?;
            Ok(EditingImgSaveOutcome {
                path,
                backup,
                rows: refreshed_rows,
                camera,
                selected_row,
                selected_name,
                active_asset_name,
                saved_modified_entries: modified_entries,
                saved_deleted_entries: deleted_entries,
                saved_added_entries: added_entries,
            })
        })();
        let _ = tx.send(result);
    });
}

pub(crate) fn poll_editing_img_save(app: &mut AppState) {
    let Some(rx) = app.editing.save_rx.as_ref() else {
        return;
    };
    let result = match rx.try_recv() {
        Ok(result) => result,
        Err(mpsc::TryRecvError::Empty) => return,
        Err(mpsc::TryRecvError::Disconnected) => {
            app.editing.save_rx = None;
            app.status_message = "IMG save worker stopped unexpectedly".to_string();
            return;
        }
    };
    app.editing.save_rx = None;
    match result {
        Ok(outcome) => {
            let modified_entry_keys = outcome
                .saved_modified_entries
                .keys()
                .cloned()
                .collect::<Vec<_>>();
            for (key, saved_bytes) in &outcome.saved_modified_entries {
                if app.editing.modified_entries.get(key) == Some(saved_bytes) {
                    app.editing.modified_entries.remove(key);
                }
                reconcile_saved_global_asset_staging(
                    &mut app.pending_replacement_assets,
                    &mut app.pending_txd_writes,
                    key,
                    saved_bytes,
                );
            }
            for key in &outcome.saved_deleted_entries {
                app.editing.deleted_entries.remove(key);
                app.pending_asset_deletes.remove(key);
            }
            // Once a DFF deletion is committed to the IMG there is no source
            // asset left for queued prelight writeback. Keeping its old live
            // mesh in these queues makes the next Save resolve an unrelated
            // fallback DFF (or fail topology validation) and can block the
            // entire transaction.
            app.pending_vertex_light_meshes.retain(|mesh_key| {
                let dff_name = mesh_key.split('|').next().unwrap_or(mesh_key);
                !outcome
                    .saved_deleted_entries
                    .contains(&asset_key(dff_name, ".dff"))
            });
            app.vertex_paint_dirty_meshes.retain(|mesh_key| {
                let dff_name = mesh_key.split('|').next().unwrap_or(mesh_key);
                !outcome
                    .saved_deleted_entries
                    .contains(&asset_key(dff_name, ".dff"))
            });
            for key in &outcome.saved_added_entries {
                app.editing.added_entries.remove(key);
            }
            app.editing.img_path = Some(outcome.path);
            app.editing.rows = outcome.rows;
            app.camera = outcome.camera;
            if let Some(name) = outcome.selected_name {
                if let Some(row_idx) = app
                    .editing
                    .rows
                    .iter()
                    .position(|row| row.entry.name.eq_ignore_ascii_case(&name))
                {
                    app.editing.selected_row = row_idx;
                    app.editing.scroll = row_idx.saturating_sub(4) as f32;
                }
            } else {
                app.editing.selected_row = outcome
                    .selected_row
                    .min(app.editing.rows.len().saturating_sub(1));
            }
            if outcome.active_asset_name.is_some_and(|name| {
                !app.editing
                    .rows
                    .iter()
                    .any(|row| row.entry.name.eq_ignore_ascii_case(&name))
            }) {
                app.editing.asset = None;
            }
            if app.editing.modified_entries.is_empty()
                && app.editing.deleted_entries.is_empty()
                && app.editing.added_entries.is_empty()
            {
                mark_editing_img_saved(app, &modified_entry_keys);
            }
            app.status_message = format!("Saved IMG safely. Backup: {}", outcome.backup.display());
        }
        Err(err) => app.status_message = format!("IMG save failed: {err}"),
    }
}

fn editing_stage_active_asset_for_save(app: &mut AppState) -> bool {
    let active = app.editing.asset.as_ref().map(|asset| match asset {
        EditingAsset::Txd(txd) => (txd.name.clone(), false),
        EditingAsset::Dff(dff) => (dff.name.clone(), dff.dirty),
        EditingAsset::Col(col) => (col.name.clone(), col.dirty),
    });
    let Some((name, dirty)) = active else {
        return true;
    };
    if !dirty {
        return true;
    }
    if editing_dff_needs_rewrite_confirmation(app) {
        app.status_message =
            "Review the DFF rewrite warning, then use Confirm Rewrite & Stage before writing"
                .to_string();
        return false;
    }
    match app.editing.asset.as_ref() {
        Some(EditingAsset::Dff(_)) => {
            if !editing_stage_dff_asset(app) {
                return false;
            }
        }
        Some(EditingAsset::Col(_)) => {
            editing_stage_col_asset(app);
            if matches!(app.editing.asset.as_ref(), Some(EditingAsset::Col(col)) if col.dirty) {
                return false;
            }
        }
        _ => {}
    }
    app.editing
        .modified_entries
        .contains_key(&editing_key(&name))
}

fn editing_save_loose_asset(app: &mut AppState) {
    if !editing_stage_active_asset_for_save(app) {
        return;
    }
    let Some(row) = editing_selected_row(app).cloned() else {
        app.status_message = "Open an asset before saving".to_string();
        return;
    };
    let key = editing_key(&row.entry.name);
    let bytes = match app.editing.modified_entries.get(&key).cloned() {
        Some(bytes) => bytes,
        None => match editing_entry_bytes(app, &row) {
            Ok(bytes) => bytes,
            Err(err) => {
                app.status_message = err;
                return;
            }
        },
    };
    let path = row.entry.img_path.clone();
    let backup = path.with_extension(format!(
        "{}bak",
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| format!("{ext}."))
            .unwrap_or_default()
    ));
    if path.exists() {
        let _ = fs::copy(&path, &backup);
    }
    match fs::write(&path, &bytes) {
        Ok(()) => {
            app.editing.modified_entries.remove(&key);
            app.editing.deleted_entries.remove(&key);
            if let Some(row) = app
                .editing
                .rows
                .iter_mut()
                .find(|row| row.entry.name.eq_ignore_ascii_case(&key))
            {
                row.entry.size = bytes.len().min(u32::MAX as usize) as u32;
                row.logical_size = replacement_entry_len(&row.entry.name, &bytes);
            }
            app.status_message = if backup.exists() {
                format!("Saved asset. Backup: {}", backup.display())
            } else {
                format!("Saved asset {}", path.display())
            };
        }
        Err(err) => app.status_message = format!("Asset save failed: {err}"),
    }
}

pub(crate) fn draw_editing_panel(app: &AppState) {
    let panel = editing_panel_rect();
    draw_rrect_bordered(
        panel.x,
        panel.y,
        panel.w,
        panel.h,
        7.0,
        1.0,
        Color::new(0.055, 0.065, 0.078, 0.94),
        ui_border(),
    );
    let dirty = editing_dirty(app);
    ui_text_bold("Editing", panel.x + 14.0, panel.y + 24.0, 18, WHITE);
    let archive_label = app
        .editing
        .img_path
        .as_ref()
        .map(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| path.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| {
            app.editing
                .rows
                .first()
                .map(|row| row.entry.img_path.to_string_lossy().into_owned())
                .unwrap_or_else(|| "No archive open".to_string())
        });
    let label_x = editing_img_choose_rect().x + editing_img_choose_rect().w + 16.0;
    let label_w = (editing_select_vertex_mode_rect().x - label_x - 16.0).max(80.0);
    ui_text(
        &app.ui_font,
        &ellipsize_width(&archive_label, 16, label_w),
        label_x,
        panel.y + 24.0,
        if dirty { YELLOW } else { LIGHTGRAY },
    );
    text_button(&app.ui_font, editing_img_prev_rect(), "<", false);
    text_button(&app.ui_font, editing_img_next_rect(), ">", false);
    text_button(&app.ui_font, editing_img_open_rect(), "Open IMG", false);
    text_button(&app.ui_font, editing_img_choose_rect(), "Open Asset", false);
    text_button(
        &app.ui_font,
        editing_img_save_rect(),
        if app.editing.img_path.is_some() {
            if dirty { "Write IMG *" } else { "Write IMG" }
        } else if dirty {
            "Write Asset *"
        } else {
            "Write Asset"
        },
        dirty,
    );
    draw_editing_box_select_controls(app);

    draw_editing_center_overlay(app);
    draw_archive_browser(app);
    draw_active_asset_editor(app);
}

fn draw_editing_center_overlay(app: &AppState) {
    let center = editing_center_rect();
    match app.editing.asset.as_ref() {
        Some(EditingAsset::Txd(txd)) => {
            draw_rrect_bordered(
                center.x,
                center.y,
                center.w,
                center.h,
                8.0,
                1.0,
                Color::new(0.035, 0.040, 0.050, 0.76),
                ui_border(),
            );
            ui_text_bold(&txd.name, center.x + 16.0, center.y + 30.0, 18, WHITE);
            if let Some(texture) = txd.preview_texture.as_ref() {
                let preview = Rect::new(
                    center.x + 22.0,
                    center.y + 52.0,
                    center.w - 44.0,
                    center.h - 82.0,
                );
                let size = texture.size();
                let scale = (preview.w / size.x).min(preview.h / size.y).min(1.0);
                let w = (size.x * scale).max(1.0);
                let h = (size.y * scale).max(1.0);
                draw_texture_ex(
                    texture,
                    preview.x + (preview.w - w) * 0.5,
                    preview.y + (preview.h - h) * 0.5,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(vec2(w, h)),
                        ..Default::default()
                    },
                );
            } else {
                ui_text(
                    &app.ui_font,
                    "Select a texture to preview it here.",
                    center.x + 22.0,
                    center.y + 70.0,
                    ui_muted(),
                );
            }
            let selected_texture = txd.textures.get(txd.selected);
            let classification = selected_texture.map(|entry| {
                app.material_classes.resolve_with_content(
                    Some(&txd.name),
                    &entry.name,
                    entry.fingerprint,
                    app.collision_generation_fallback_material,
                )
            });
            text_button(
                &app.ui_font,
                editing_txd_material_button_rect(),
                &classification
                    .map(|resolved| {
                        if resolved.no_collision {
                            "Collision material: No Collision".to_string()
                        } else {
                            format!(
                                "Collision material: {}",
                                col_material_label(resolved.material)
                            )
                        }
                    })
                    .unwrap_or_else(|| "Collision material: select a texture".to_string()),
                txd.material_picker_open,
            );
            draw_editing_txd_material_picker(app, txd);
        }
        Some(EditingAsset::Dff(dff)) => {
            ui_text_bold(
                &format!("DFF Preview: {}", ellipsize(&dff.name, 42)),
                center.x + 14.0,
                center.y + 24.0,
                18,
                WHITE,
            );
            ui_text(
                &app.ui_font,
                "Selected material is highlighted in yellow.",
                center.x + 14.0,
                center.y + 48.0,
                ui_dim(),
            );
        }
        Some(EditingAsset::Col(col)) => {
            ui_text_bold(
                &format!("COL Preview: {}", ellipsize(&col.name, 42)),
                center.x + 14.0,
                center.y + 24.0,
                18,
                WHITE,
            );
            ui_text(
                &app.ui_font,
                "Selected face is highlighted in yellow.",
                center.x + 14.0,
                center.y + 48.0,
                ui_dim(),
            );
        }
        None => {
            draw_rrect_bordered(
                center.x,
                center.y,
                center.w,
                center.h,
                8.0,
                1.0,
                Color::new(0.035, 0.040, 0.050, 0.46),
                ui_border(),
            );
            ui_text_bold("No Asset Open", center.x + 20.0, center.y + 34.0, 20, WHITE);
            ui_text(
                &app.ui_font,
                "Open a DFF, COL, or TXD from the archive list.",
                center.x + 20.0,
                center.y + 64.0,
                ui_dim(),
            );
        }
    }
}

fn draw_editing_txd_material_picker(app: &AppState, txd: &EditingTxdState) {
    if !txd.material_picker_open {
        return;
    }
    let entry = txd.textures.get(txd.selected);
    let texture = entry.map(|entry| entry.name.as_str()).unwrap_or("");
    let fingerprint = entry.and_then(|entry| entry.fingerprint);
    draw_collision_material_picker(
        app,
        texture,
        Some(&txd.name),
        fingerprint,
        txd.material_picker_scope,
        &txd.material_picker_search,
        txd.material_picker_scroll,
    );
}

fn draw_dff_collision_material_picker(app: &AppState, dff: &EditingDffState) {
    if !dff.collision_material_picker_open {
        return;
    }
    let texture = dff
        .raw
        .material_textures
        .get(dff.selected_material)
        .map(String::as_str)
        .unwrap_or("");
    let fingerprint = editing_dff_texture_fingerprint(&app.txd_textures, dff);
    draw_collision_material_picker(
        app,
        texture,
        dff.txd_context.as_deref(),
        fingerprint,
        dff.collision_material_picker_scope,
        &dff.collision_material_picker_search,
        dff.collision_material_picker_scroll,
    );
}

fn draw_collision_material_picker(
    app: &AppState,
    texture: &str,
    txd_name: Option<&str>,
    fingerprint: Option<TextureContentFingerprint>,
    scope: CollisionMaterialAssignmentScope,
    search_text: &str,
    scroll: f32,
) {
    let popup = editing_txd_material_picker_rect();
    draw_rrect_bordered(
        popup.x,
        popup.y,
        popup.w,
        popup.h,
        8.0,
        1.0,
        Color::new(0.035, 0.043, 0.054, 0.99),
        ui_accent(),
    );
    ui_text_bold(
        &format!("Collision Material: {}", ellipsize(texture, 34)),
        popup.x + 18.0,
        popup.y + 28.0,
        17,
        WHITE,
    );
    let identical_count = fingerprint.map(|fingerprint| {
        app.txd_textures
            .values()
            .flatten()
            .filter(|entry| entry.content_fingerprint == fingerprint)
            .count()
    });
    for option in CollisionMaterialAssignmentScope::ALL {
        let available = collision_material_scope_available(option, txd_name, fingerprint);
        let label = match option {
            CollisionMaterialAssignmentScope::ExactTxd => txd_name
                .map(|txd| format!("Exact: {}", ellipsize(txd, 12)))
                .unwrap_or_else(|| "Exact unavailable".to_string()),
            CollisionMaterialAssignmentScope::IdenticalContent => identical_count
                .map(|count| format!("Identical ({count})"))
                .unwrap_or_else(|| "Identical unavailable".to_string()),
            CollisionMaterialAssignmentScope::GlobalName => "Global name".to_string(),
        };
        text_button(
            &app.ui_font,
            editing_txd_material_scope_rect(option),
            &label,
            scope == option && available,
        );
    }
    let search = editing_txd_material_search_rect();
    draw_rrect_bordered(
        search.x,
        search.y,
        search.w,
        search.h,
        6.0,
        1.0,
        Color::new(0.055, 0.064, 0.078, 1.0),
        ui_accent(),
    );
    ui_text(
        &app.ui_font,
        if search_text.is_empty() {
            "Search material name or ID..."
        } else {
            search_text
        },
        search.x + 9.0,
        search.y + 20.0,
        if search_text.is_empty() {
            ui_muted()
        } else {
            WHITE
        },
    );
    let list = editing_txd_material_list_rect();
    let options = editing_txd_material_filtered(search_text);
    let row_h = 26.0;
    let visible = (list.h / row_h).floor().max(1.0) as usize;
    let start = scroll
        .floor()
        .max(0.0)
        .min(options.len().saturating_sub(visible) as f32) as usize;
    let assigned = match scope {
        CollisionMaterialAssignmentScope::ExactTxd => txd_name.and_then(|txd| {
            if app.material_classes.txd_is_no_collision(txd, texture) {
                Some(CollisionMaterialPickerValue::NoCollision)
            } else {
                app.material_classes
                    .txd_material(txd, texture)
                    .map(CollisionMaterialPickerValue::Material)
            }
        }),
        CollisionMaterialAssignmentScope::IdenticalContent => fingerprint.and_then(|fingerprint| {
            if app.material_classes.content_is_no_collision(fingerprint) {
                Some(CollisionMaterialPickerValue::NoCollision)
            } else {
                app.material_classes
                    .content_material(fingerprint)
                    .map(CollisionMaterialPickerValue::Material)
            }
        }),
        CollisionMaterialAssignmentScope::GlobalName => {
            if app.material_classes.global_is_no_collision(texture) {
                Some(CollisionMaterialPickerValue::NoCollision)
            } else {
                app.material_classes
                    .global_material(texture)
                    .map(CollisionMaterialPickerValue::Material)
            }
        }
    };
    begin_ui_clip(list);
    for row in 0..visible {
        let Some((choice, name)) = options.get(start + row).copied() else {
            break;
        };
        let rect = Rect::new(list.x, list.y + row as f32 * row_h, list.w, row_h - 2.0);
        let selected = assigned == Some(choice);
        if selected || rect.contains(mouse_position().into()) {
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
        let swatch = match choice {
            CollisionMaterialPickerValue::NoCollision => [0.70, 0.12, 0.12, 1.0],
            CollisionMaterialPickerValue::Material(id) => collision_material_color(id, 1.0),
        };
        draw_rrect(
            rect.x + 6.0,
            rect.y + 5.0,
            14.0,
            14.0,
            3.0,
            Color::new(swatch[0], swatch[1], swatch[2], 1.0),
        );
        ui_text_size(
            &app.ui_font,
            &match choice {
                CollisionMaterialPickerValue::NoCollision => {
                    "No Collision — exclude these faces".to_string()
                }
                CollisionMaterialPickerValue::Material(id) => {
                    format!("{name} ({id})")
                }
            },
            rect.x + 28.0,
            rect.y + 18.0,
            14,
            if selected { ui_accent() } else { LIGHTGRAY },
        );
    }
    end_ui_clip();
    let track = editing_txd_material_scrollbar_rect();
    let max_scroll = options.len().saturating_sub(visible) as f32;
    if max_scroll > 0.0 {
        draw_rrect(
            track.x,
            track.y,
            track.w,
            track.h,
            4.0,
            Color::new(0.08, 0.10, 0.13, 0.92),
        );
        let thumb_h = (track.h * visible as f32 / options.len() as f32).clamp(24.0, track.h);
        let thumb_y = track.y + (track.h - thumb_h) * (start as f32 / max_scroll);
        draw_rrect(
            track.x,
            thumb_y,
            track.w,
            thumb_h,
            4.0,
            if track.contains(mouse_position().into()) {
                ui_accent()
            } else {
                Color::new(0.36, 0.43, 0.52, 1.0)
            },
        );
    }
    ui_text_size(
        &app.ui_font,
        "Click the selected material again to clear it. Esc closes.",
        popup.x + 18.0,
        popup.y + popup.h - 16.0,
        13,
        ui_muted(),
    );
}

fn draw_archive_browser(app: &AppState) {
    let left = editing_archive_rect();
    draw_panel_rect(&app.ui_font, left, Some("IMG Browser"));
    text_button(
        &app.ui_font,
        editing_merge_img_rect(),
        "Merge IMG",
        app.editing.merge_rx.is_some() || app.editing.merge_apply_job.is_some(),
    );
    let search = editing_search_rect();
    draw_rrect_bordered(
        search.x,
        search.y,
        search.w,
        search.h,
        5.0,
        1.0,
        Color::new(0.045, 0.052, 0.064, 1.0),
        if app.editing.search_active {
            ui_accent()
        } else {
            ui_border()
        },
    );
    ui_text(
        &app.ui_font,
        if app.editing.search.is_empty() {
            "Filter entries..."
        } else {
            &app.editing.search
        },
        search.x + 9.0,
        search.y + 20.0,
        if app.editing.search.is_empty() {
            ui_muted()
        } else {
            WHITE
        },
    );

    ui_text(
        &app.ui_font,
        "Name",
        left.x + 14.0,
        left.y + 94.0,
        ui_muted(),
    );
    ui_text(
        &app.ui_font,
        "Type",
        left.x + left.w - 164.0,
        left.y + 94.0,
        ui_muted(),
    );
    ui_text(
        &app.ui_font,
        "Size",
        left.x + left.w - 88.0,
        left.y + 94.0,
        ui_muted(),
    );
    let filtered = editing_filtered_indices(app);
    let visible = ((left.h - 154.0) / EDIT_ROW_H).floor().max(1.0) as usize;
    let start = app.editing.scroll.floor() as usize;
    for row_slot in 0..visible {
        let Some(idx) = filtered.get(start + row_slot).copied() else {
            break;
        };
        let Some(row) = app.editing.rows.get(idx) else {
            continue;
        };
        let row_rect = editing_row_rect(row_slot);
        let selected = app.editing.selected_row == start + row_slot;
        let key = editing_key(&row.entry.name);
        let staged = app.editing.modified_entries.contains_key(&key);
        if selected || row_rect.contains(mouse_position().into()) {
            draw_rrect(
                row_rect.x,
                row_rect.y,
                row_rect.w,
                row_rect.h,
                5.0,
                if selected {
                    ui_surface_active()
                } else {
                    ui_surface_hover()
                },
            );
        }
        ui_text(
            &app.ui_font,
            &ellipsize_width(&row.entry.name, 16, row_rect.w - 185.0),
            row_rect.x + 8.0,
            row_rect.y + 20.0,
            if selected { ui_accent() } else { WHITE },
        );
        ui_text(
            &app.ui_font,
            editing_entry_type(&row.entry.name),
            left.x + left.w - 164.0,
            row_rect.y + 20.0,
            if staged { YELLOW } else { ui_dim() },
        );
        ui_text(
            &app.ui_font,
            &format_bytes(row.logical_size),
            left.x + left.w - 88.0,
            row_rect.y + 20.0,
            ui_muted(),
        );
    }
    icon_button(
        &app.ui_font,
        editing_open_entry_rect(),
        &app.icons.select,
        false,
        true,
        "Open selected asset",
    );
    icon_button(
        &app.ui_font,
        editing_add_entry_rect(),
        &app.icons.duplicate,
        false,
        true,
        "Add file to IMG",
    );
    icon_button(
        &app.ui_font,
        editing_replace_entry_rect(),
        &app.icons.redo,
        false,
        true,
        "Replace selected entry",
    );
    icon_button(
        &app.ui_font,
        editing_delete_entry_rect(),
        &app.icons.delete,
        false,
        true,
        "Delete selected entry",
    );
    icon_button(
        &app.ui_font,
        editing_extract_entry_rect(),
        &app.icons.save,
        false,
        true,
        "Extract selected entry",
    );
}

fn draw_active_asset_editor(app: &AppState) {
    let right = editing_asset_rect();
    draw_panel_rect(&app.ui_font, right, None);
    match app.editing.asset.as_ref() {
        Some(EditingAsset::Txd(txd)) => draw_txd_asset(app, right, txd),
        Some(EditingAsset::Dff(dff)) => draw_dff_asset(app, right, dff),
        Some(EditingAsset::Col(col)) => draw_col_asset(app, right, col),
        None => {
            ui_text(
                &app.ui_font,
                "Open a .txd, .dff, or .col entry from the archive list.",
                right.x + 16.0,
                right.y + 62.0,
                ui_dim(),
            );
            ui_text(
                &app.ui_font,
                &app.editing.message,
                right.x + 16.0,
                right.y + 90.0,
                ui_muted(),
            );
        }
    }
}

fn draw_txd_asset(app: &AppState, right: Rect, txd: &EditingTxdState) {
    ui_text_bold(&txd.name, right.x + 16.0, right.y + 30.0, 18, WHITE);
    let filtered = editing_txd_filtered_indices(txd);
    ui_text(
        &app.ui_font,
        &if txd.search.trim().is_empty() {
            format!("{} texture(s)", txd.textures.len())
        } else {
            format!("{} of {} texture(s)", filtered.len(), txd.textures.len())
        },
        right.x + 16.0,
        right.y + 60.0,
        ui_dim(),
    );

    let search = editing_txd_search_rect();
    draw_rrect_bordered(
        search.x,
        search.y,
        search.w,
        search.h,
        5.0,
        1.0,
        Color::new(0.045, 0.052, 0.064, 1.0),
        if txd.search_active {
            ui_accent()
        } else {
            ui_border()
        },
    );
    let search_text = if txd.search.is_empty() {
        "Search textures...".to_string()
    } else {
        ellipsize_width(&txd.search, 16, search.w - 20.0)
    };
    ui_text(
        &app.ui_font,
        &search_text,
        search.x + 9.0,
        search.y + 20.0,
        if txd.search.is_empty() {
            ui_muted()
        } else {
            WHITE
        },
    );
    if txd.search_active && (get_time() * 2.0) as i32 % 2 == 0 {
        let caret_x =
            (search.x + 9.0 + ui_text_width(&search_text, 16)).min(search.x + search.w - 8.0);
        draw_line(
            caret_x,
            search.y + 6.0,
            caret_x,
            search.y + 22.0,
            1.0,
            WHITE,
        );
    }

    let list = editing_txd_list_rect();
    let visible = (list.h / 32.0).floor().max(1.0) as usize;
    let max_start = filtered.len().saturating_sub(visible);
    let start = (txd.scroll.floor() as usize).min(max_start);
    for row in 0..visible {
        let Some(idx) = filtered.get(start + row).copied() else {
            break;
        };
        let Some(entry) = txd.textures.get(idx) else {
            break;
        };
        let rect = Rect::new(list.x, list.y + row as f32 * 32.0, list.w - 10.0, 28.0);
        let selected = idx == txd.selected;
        if selected || rect.contains(mouse_position().into()) {
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
        let thumb_rect = Rect::new(rect.x + 5.0, rect.y + 3.0, 22.0, 22.0);
        draw_rrect_bordered(
            thumb_rect.x,
            thumb_rect.y,
            thumb_rect.w,
            thumb_rect.h,
            4.0,
            1.0,
            Color::new(0.035, 0.040, 0.050, 1.0),
            ui_border(),
        );
        if let Some(thumbnail) = entry.thumbnail.as_ref() {
            draw_texture_ex(
                thumbnail,
                thumb_rect.x + 2.0,
                thumb_rect.y + 2.0,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(thumb_rect.w - 4.0, thumb_rect.h - 4.0)),
                    ..Default::default()
                },
            );
        }
        ui_text(
            &app.ui_font,
            &ellipsize_width(&entry.name, 16, rect.w - 158.0),
            rect.x + 34.0,
            rect.y + 20.0,
            if selected { ui_accent() } else { WHITE },
        );
        ui_text(
            &app.ui_font,
            &format!(
                "{}x{} {}",
                entry.width,
                entry.height,
                tx_format_label(entry.format)
            ),
            rect.x + rect.w - 124.0,
            rect.y + 20.0,
            ui_muted(),
        );
    }
    if filtered.is_empty() {
        ui_text(
            &app.ui_font,
            "No matching textures",
            list.x + 8.0,
            list.y + 22.0,
            ui_muted(),
        );
    } else if max_start > 0 {
        let track = Rect::new(
            list.x + list.w - 5.0,
            list.y,
            3.0,
            visible as f32 * 32.0 - 4.0,
        );
        let thumb_h = (track.h * visible as f32 / filtered.len() as f32).max(24.0);
        let thumb_y = track.y + (track.h - thumb_h) * (start as f32 / max_start as f32);
        draw_rrect(
            track.x,
            track.y,
            track.w,
            track.h,
            2.0,
            Color::new(0.12, 0.14, 0.17, 1.0),
        );
        draw_rrect(track.x, thumb_y, track.w, thumb_h, 2.0, ui_dim());
    }
    text_button(&app.ui_font, editing_txd_add_rect(), "Add Texture", false);
    text_button(&app.ui_font, editing_txd_replace_rect(), "Replace", false);
    text_button(&app.ui_font, editing_txd_rename_rect(), "Rename", false);
    text_button(
        &app.ui_font,
        editing_txd_export_all_rect(),
        "Export All",
        false,
    );
}

fn editing_action_button(
    font: &Font,
    rect: Rect,
    icon: &Texture2D,
    label: &str,
    active: bool,
    danger: bool,
) {
    let mouse: Vec2 = mouse_position().into();
    let hovered = rect.contains(mouse);
    let bg = if active {
        ui_surface_active()
    } else if danger && hovered {
        Color::new(0.30, 0.12, 0.13, 1.0)
    } else if hovered {
        ui_surface_hover()
    } else {
        ui_surface()
    };
    let border = if active {
        ui_accent()
    } else if danger && hovered {
        Color::new(0.82, 0.34, 0.34, 1.0)
    } else if hovered {
        Color::new(0.34, 0.36, 0.40, 1.0)
    } else {
        ui_border()
    };
    draw_rrect(
        rect.x + 1.0,
        rect.y + 2.0,
        rect.w,
        rect.h,
        8.0,
        Color::new(0.0, 0.0, 0.0, 0.20),
    );
    draw_rrect_bordered(rect.x, rect.y, rect.w, rect.h, 8.0, 1.0, bg, border);
    draw_texture_ex(
        icon,
        rect.x + 7.0,
        rect.y + (rect.h - 18.0) * 0.5,
        if danger {
            Color::new(1.0, 0.72, 0.72, 1.0)
        } else {
            WHITE
        },
        DrawTextureParams {
            dest_size: Some(vec2(18.0, 18.0)),
            ..Default::default()
        },
    );
    let text_x = rect.x + 31.0;
    let visible = ellipsize_width(label, 16, rect.w - 38.0);
    ui_text(font, &visible, text_x, rect.y + rect.h * 0.5 + 6.0, WHITE);
    if hovered && visible != label {
        let tip_w = (label.len() as f32 * 7.5 + 18.0).max(42.0);
        draw_rrect_bordered(
            rect.x,
            rect.y - 30.0,
            tip_w,
            24.0,
            6.0,
            1.0,
            Color::new(0.025, 0.035, 0.050, 0.98),
            Color::new(0.25, 0.27, 0.30, 1.0),
        );
        ui_text(font, label, rect.x + 9.0, rect.y - 14.0, LIGHTGRAY);
    }
}

fn draw_editing_select_mode_toggle(app: &AppState, mode: EditingSelectMode) {
    editing_action_button(
        &app.ui_font,
        editing_select_vertex_mode_rect(),
        &app.icons.vertex,
        "Vertex",
        mode == EditingSelectMode::Vertex,
        false,
    );
    editing_action_button(
        &app.ui_font,
        editing_select_edge_mode_rect(),
        &app.icons.edge,
        "Edge",
        mode == EditingSelectMode::Edge,
        false,
    );
    editing_action_button(
        &app.ui_font,
        editing_select_face_mode_rect(),
        &app.icons.face,
        "Face",
        mode == EditingSelectMode::Face,
        false,
    );
}

fn draw_editing_box_select_controls(app: &AppState) {
    let mode = editing_box_select_mode_rect();
    let minus = editing_box_select_minus_rect();
    let plus = editing_box_select_plus_rect();
    let label_w = ui_text_width("Box", 14);
    ui_text(
        &app.ui_font,
        "Box",
        mode.x - label_w - 8.0,
        mode.y + 19.0,
        ui_dim(),
    );
    text_button(
        &app.ui_font,
        mode,
        app.box_select_mode.label(),
        app.box_select_mode != BoxSelectMode::Add,
    );
    text_button(&app.ui_font, minus, "-", false);
    text_button(&app.ui_font, plus, "+", false);
    ui_text(
        &app.ui_font,
        &format!("{:.0}", app.box_select_distance),
        plus.x + plus.w + 8.0,
        plus.y + 19.0,
        LIGHTGRAY,
    );
}

pub(crate) fn dff_section_header_button(
    font: &Font,
    rect: Rect,
    expanded: bool,
    title: &str,
    hint: &str,
) {
    let mouse: Vec2 = mouse_position().into();
    let hovered = rect.contains(mouse);
    let bg = if hovered {
        ui_surface_hover()
    } else {
        ui_surface()
    };
    let border = if hovered {
        Color::new(0.34, 0.36, 0.40, 1.0)
    } else {
        ui_border()
    };
    draw_rrect_bordered(rect.x, rect.y, rect.w, rect.h, 8.0, 1.0, bg, border);
    ui_text_size(
        font,
        if expanded { "v" } else { ">" },
        rect.x + 10.0,
        rect.y + 21.0,
        14,
        ui_accent(),
    );
    ui_text_bold(
        title,
        rect.x + 26.0,
        rect.y + 22.0,
        16,
        if expanded { WHITE } else { LIGHTGRAY },
    );
    if !hint.is_empty() {
        let hint_w = ui_text_width(hint, 14);
        ui_text_size(
            font,
            hint,
            rect.x + rect.w - hint_w - 10.0,
            rect.y + 21.0,
            14,
            ui_dim(),
        );
    }
}

fn draw_dff_asset(app: &AppState, right: Rect, dff: &EditingDffState) {
    draw_editing_select_mode_toggle(app, dff.select_mode);
    let mouse: Vec2 = mouse_position().into();
    let emitter = selected_material_emitter(app);
    let layout = dff_panel_layout(dff, emitter, dff_face_emitter_entries(app, &dff.name).len());
    let active_breakable = fracture_component_for_dff(dff)
        .ok()
        .and_then(|index| dff.raw.components.get(index))
        .and_then(|component| component.breakable.as_ref());

    // Fixed header.
    ui_text_bold(
        &dff.name,
        right.x + 16.0,
        right.y + 30.0,
        18,
        if dff.dirty { YELLOW } else { WHITE },
    );
    let animated_materials = dff
        .raw
        .material_animations
        .iter()
        .filter(|animation| animation.names.iter().any(|name| !name.trim().is_empty()))
        .count();
    let asset_counts = if animated_materials > 0 || !dff.raw.uv_anim_dictionaries.is_empty() {
        format!(
            "Vertices {}   Faces {}   Materials {}   Animated {}   Dictionaries {}",
            dff.raw.vertices.len(),
            dff.raw.triangles.len(),
            dff.raw.material_textures.len(),
            animated_materials,
            dff.raw.uv_anim_dictionaries.len()
        )
    } else {
        format!(
            "Vertices {}   Faces {}   Materials {}",
            dff.raw.vertices.len(),
            dff.raw.triangles.len(),
            dff.raw.material_textures.len()
        )
    };
    ui_text_size(
        &app.ui_font,
        &ellipsize_width(&asset_counts, 15, right.w - 32.0),
        right.x + 16.0,
        right.y + 56.0,
        15,
        ui_dim(),
    );
    ui_text(
        &app.ui_font,
        &ellipsize_width(&dff.txd_source_label, 16, right.w - 32.0),
        right.x + 16.0,
        right.y + 78.0,
        if dff.txd_context.is_some() {
            ui_accent()
        } else {
            YELLOW
        },
    );
    let selected_label = if let Some(idx) = dff.selected_2dfx {
        if dff.raw.effects_2dfx.get(idx).is_some() {
            format!(
                "Selected 2DFX {} ({})",
                idx + 1,
                dff_2dfx_effect_display_name(&dff.raw.effects_2dfx, idx)
            )
        } else {
            "No face selected".to_string()
        }
    } else {
        match (dff.selected_face, dff_selected_vertex_index(dff)) {
            (Some(face), Some(vertex)) => format!(
                "Selected face {face}, vertex {vertex}   {} selected",
                dff_selected_vertex_set(dff).len()
            ),
            (Some(face), None) => format!("Selected face {face}"),
            _ => "No face selected".to_string(),
        }
    };
    ui_text(
        &app.ui_font,
        &ellipsize_width(&selected_label, 16, right.w - 32.0),
        right.x + 16.0,
        right.y + 100.0,
        LIGHTGRAY,
    );
    let mut hover_line = String::new();
    if let Some(vertex) = dff.hovered_vertex.and_then(|idx| dff.raw.vertices.get(idx)) {
        hover_line = format!(
            "Hover vertex {}   {:.3}, {:.3}, {:.3}",
            dff.hovered_vertex.unwrap_or(0),
            vertex.x,
            vertex.y,
            vertex.z
        );
    }
    if let Some(cutter) = dff.boolean_box {
        if !hover_line.is_empty() {
            hover_line.push_str("   ");
        }
        hover_line.push_str(&format!(
            "Cutter {:.1} x {:.1} x {:.1}",
            cutter.half_extents.x * 2.0,
            cutter.half_extents.y * 2.0,
            cutter.half_extents.z * 2.0
        ));
    }
    if !hover_line.is_empty() {
        ui_text_size(
            &app.ui_font,
            &ellipsize_width(&hover_line, 14, right.w - 32.0),
            right.x + 16.0,
            right.y + 122.0,
            14,
            ui_accent(),
        );
    }
    if dff.normalized_warning {
        ui_text_size(
            &app.ui_font,
            "Staging rewrites this DFF with Eagle's normalized mesh writer.",
            right.x + 16.0,
            right.y + 144.0,
            14,
            YELLOW,
        );
        ui_text_size(
            &app.ui_font,
            "Unknown RenderWare extensions may not be preserved.",
            right.x + 16.0,
            right.y + 163.0,
            14,
            YELLOW,
        );
    }

    // Scrollable section stack.
    begin_ui_clip(layout.content);
    for section in DFF_SECTIONS {
        let idx = section as usize;
        let hint = match section {
            DffSection::Effects => format!("{}", dff.raw.effects_2dfx.len()),
            DffSection::Lighting => format!("{}", layout.lighting_rows.len()),
            DffSection::Fractures => active_breakable
                .map(|breakable| {
                    if breakable.stale {
                        format!("{} · STALE", breakable.groups.len())
                    } else {
                        format!("{}", breakable.groups.len())
                    }
                })
                .unwrap_or_default(),
            DffSection::Materials => format!("{}", dff.raw.material_textures.len()),
            DffSection::MaterialAnim => {
                if animated_materials > 0 {
                    format!("{animated_materials} animated")
                } else {
                    String::new()
                }
            }
            DffSection::Cutter => {
                if dff.boolean_box.is_some() {
                    "active".to_string()
                } else {
                    String::new()
                }
            }
            _ => String::new(),
        };
        dff_section_header_button(
            &app.ui_font,
            layout.headers[idx],
            !dff.panel_collapsed[idx],
            section.title(),
            &hint,
        );
    }

    // 2DFX rows.
    for (idx, rect) in layout.rows_2dfx.iter().enumerate() {
        let Some(effect) = dff.raw.effects_2dfx.get(idx) else {
            break;
        };
        let selected = Some(idx) == dff.selected_2dfx;
        if selected || rect.contains(mouse) {
            draw_rrect(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                5.0,
                if selected {
                    Color::new(0.22, 0.18, 0.10, 1.0)
                } else {
                    Color::new(0.10, 0.13, 0.16, 1.0)
                },
            );
        }
        ui_text(
            &app.ui_font,
            &ellipsize_width(
                &format!(
                    "#{:02} {}",
                    idx + 1,
                    dff_2dfx_effect_display_name(&dff.raw.effects_2dfx, idx)
                ),
                16,
                rect.w - 168.0,
            ),
            rect.x + 8.0,
            rect.y + 18.0,
            if selected { YELLOW } else { WHITE },
        );
        let coords = format!(
            "{:.1}, {:.1}, {:.1}",
            effect.position.x, effect.position.y, effect.position.z
        );
        let coords_w = ui_text_width(&coords, 14);
        ui_text_size(
            &app.ui_font,
            &coords,
            rect.x + rect.w - coords_w - 8.0,
            rect.y + 18.0,
            14,
            ui_muted(),
        );
    }
    if let Some(rect) = layout.add_2dfx {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.duplicate,
            "Add 2DFX",
            false,
            false,
        );
    }
    if let Some(rect) = layout.delete_2dfx {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.delete,
            "Delete 2DFX",
            false,
            true,
        );
    }
    if let Some(rect) = layout.type_2dfx {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.select,
            "2DFX Type",
            dff.dff_2dfx_type_picker_open,
            false,
        );
    }
    if let Some(rect) = layout.payload_2dfx {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.duplicate,
            "Payload Hex",
            dff.dff_2dfx_payload_editor_open,
            false,
        );
    }
    if let Some(rect) = layout.add_2dfx_corona_preset {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.duplicate,
            "Add Corona Preset  v",
            dff.dff_2dfx_corona_preset_picker_open,
            false,
        );
    }
    if let Some(rect) = layout.regenerate_2dfx_coronas {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.duplicate,
            "Regenerate 2DFX Coronas",
            false,
            false,
        );
    }
    if let Some(rect) = layout.generate_lod {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.duplicate,
            if app.lod_generation_job.is_some() {
                "Generating LOD..."
            } else {
                "Generate LOD"
            },
            false,
            false,
        );
    }

    // Breakable PLG fracture zones. Selecting a row maps the native debris
    // group back onto intact faces, giving an immediate colored/highlighted
    // manual editing workflow without pretending the debris is render mesh.
    if let Some(breakable) = active_breakable {
        for (idx, rect) in layout.fracture_rows.iter().enumerate() {
            let Some(group) = breakable.groups.get(idx) else {
                break;
            };
            let selected = idx == dff.selected_breakable_group;
            if selected || rect.contains(mouse) {
                draw_rrect(
                    rect.x,
                    rect.y,
                    rect.w,
                    rect.h,
                    5.0,
                    if selected {
                        Color::new(0.23, 0.15, 0.32, 1.0)
                    } else {
                        Color::new(0.10, 0.13, 0.16, 1.0)
                    },
                );
            }
            let color = Color::new(
                0.35 + ((idx * 47 % 100) as f32 / 250.0),
                0.28 + ((idx * 71 % 100) as f32 / 300.0),
                0.45 + ((idx * 29 % 100) as f32 / 250.0),
                1.0,
            );
            draw_rrect(rect.x + 7.0, rect.y + 6.0, 14.0, 14.0, 3.0, color);
            let face_count = breakable
                .triangles
                .iter()
                .filter(|triangle| triangle.group as usize == idx)
                .count();
            ui_text(
                &app.ui_font,
                &ellipsize_width(
                    &format!(
                        "Zone {} · {} face(s) · {}",
                        idx + 1,
                        face_count,
                        group.texture
                    ),
                    15,
                    rect.w - 38.0,
                ),
                rect.x + 29.0,
                rect.y + 20.0,
                if selected {
                    Color::new(0.90, 0.72, 1.0, 1.0)
                } else {
                    WHITE
                },
            );
        }
    } else if let Some(header) = layout.headers.get(DffSection::Fractures as usize)
        && !dff.panel_collapsed[DffSection::Fractures as usize]
    {
        ui_text_size(
            &app.ui_font,
            "No fragment mesh. Generate zones or select faces for a manual zone.",
            header.x + 10.0,
            header.y + DFF_SEC_HEADER_H + 20.0,
            14,
            ui_muted(),
        );
    }
    if let Some(rect) = layout.generate_fractures {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.duplicate,
            if app.fracture_generation_job.is_some() {
                "Generating..."
            } else {
                "Auto Generate"
            },
            false,
            false,
        );
    }
    if let Some(rect) = layout.manual_fracture_zone {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.face,
            "Zone from Faces",
            false,
            false,
        );
    }
    if let Some(rect) = layout.fracture_origin {
        let origin = active_breakable
            .map(|breakable| match breakable.origin {
                BreakableOrigin::Object => "Origin: Object",
                BreakableOrigin::Collision => "Origin: Collision",
            })
            .unwrap_or("Origin: Collision");
        editing_action_button(&app.ui_font, rect, &app.icons.select, origin, false, false);
    }
    if let Some(rect) = layout.clear_fractures {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.delete,
            "Clear Zones",
            false,
            true,
        );
    }
    if let Some(rect) = layout.simulate_fractures {
        let preview_label = match dff.fracture_preview_started_at {
            Some(started_at) if get_time() - started_at >= FRACTURE_PREVIEW_DURATION_SECONDS => {
                "Replay Break"
            }
            Some(_) => "Reset Preview",
            None => "Simulate Break",
        };
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.duplicate,
            preview_label,
            dff.fracture_preview_started_at.is_some(),
            false,
        );
    }

    // Linked face-lighting entries. Clicking one selects every face in it.
    let lighting_entries = dff_face_emitter_entries(app, &dff.name);
    let selected_faces = dff_selected_face_set(dff);
    for (idx, rect) in layout.lighting_rows.iter().enumerate() {
        let Some((_, faces, entry)) = lighting_entries.get(idx) else {
            break;
        };
        let selected = selected_faces.len() == faces.len()
            && faces.iter().all(|face| selected_faces.contains(face));
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
        let label = if faces.len() == 1 {
            format!("Face {}", faces[0])
        } else {
            format!("Group {}  ·  {} faces", idx + 1, faces.len())
        };
        ui_text(
            &app.ui_font,
            &ellipsize_width(&label, 16, rect.w - 100.0),
            rect.x + 9.0,
            rect.y + 20.0,
            if selected { ui_accent() } else { WHITE },
        );
        let status = if entry.enabled { "Enabled" } else { "Disabled" };
        let status_w = ui_text_width(status, 14);
        ui_text_size(
            &app.ui_font,
            status,
            rect.x + rect.w - status_w - 9.0,
            rect.y + 20.0,
            14,
            if entry.enabled { GREEN } else { ui_muted() },
        );
    }
    if layout.lighting_rows.is_empty()
        && let Some(header) = layout.headers.get(DffSection::Lighting as usize)
        && !dff.panel_collapsed[DffSection::Lighting as usize]
    {
        ui_text_size(
            &app.ui_font,
            "No face lighting assigned",
            header.x + 10.0,
            header.y + DFF_SEC_HEADER_H + 20.0,
            14,
            ui_muted(),
        );
    }

    // Material rows.
    if let Some(list) = layout.material_list {
        let total = dff_material_slot_count(&dff.raw);
        let visible = layout.material_visible.max(1);
        let max_start = total.saturating_sub(visible);
        let start = (dff.material_scroll.floor().max(0.0) as usize).min(max_start);
        for row in 0..visible.min(total) {
            let material = start + row;
            if material >= total {
                break;
            }
            let rect = Rect::new(
                list.x,
                list.y + row as f32 * DFF_MAT_ROW_H,
                list.w,
                DFF_MAT_ROW_H - 4.0,
            );
            let selected = material == dff.selected_material;
            let face_count = material_face_count(&dff.raw, material);
            let texture = dff
                .raw
                .material_textures
                .get(material)
                .map(String::as_str)
                .unwrap_or("");
            let anim_label = dff
                .raw
                .material_animations
                .get(material)
                .and_then(|animation| {
                    let names = animation
                        .names
                        .iter()
                        .filter(|name| !name.trim().is_empty())
                        .map(|name| name.trim())
                        .collect::<Vec<_>>();
                    (!names.is_empty()).then(|| format!("Anim {}", names.join(", ")))
                });
            let thumbnail = dff
                .material_thumbnails
                .get(material)
                .and_then(Option::as_ref);
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
            let thumb_rect = Rect::new(rect.x + 4.0, rect.y + 2.0, 32.0, 32.0);
            draw_rrect_bordered(
                thumb_rect.x,
                thumb_rect.y,
                thumb_rect.w,
                thumb_rect.h,
                4.0,
                1.0,
                Color::new(0.035, 0.040, 0.050, 1.0),
                if thumbnail.is_some() {
                    ui_border()
                } else {
                    Color::new(0.48, 0.16, 0.16, 1.0)
                },
            );
            if let Some(thumbnail) = thumbnail {
                draw_texture_ex(
                    thumbnail,
                    thumb_rect.x + 2.0,
                    thumb_rect.y + 2.0,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(vec2(thumb_rect.w - 4.0, thumb_rect.h - 4.0)),
                        ..Default::default()
                    },
                );
            }
            ui_text(
                &app.ui_font,
                &format!("#{material:02}"),
                rect.x + 44.0,
                rect.y + 23.0,
                if selected { ui_accent() } else { ui_dim() },
            );
            let fingerprint =
                editing_dff_material_texture_fingerprint(&app.txd_textures, dff, material);
            let collision_material = app.material_classes.resolve_with_content(
                dff.txd_context.as_deref(),
                texture,
                fingerprint,
                app.collision_generation_fallback_material,
            );
            let face_label = if collision_material.no_collision {
                format!("{face_count} face(s) · NO COL")
            } else {
                format!("{face_count} face(s) · COL {}", collision_material.material)
            };
            let face_w = ui_text_width(&face_label, 14);
            ui_text(
                &app.ui_font,
                &ellipsize_width(
                    if texture.is_empty() {
                        "<empty material>"
                    } else {
                        texture
                    },
                    16,
                    rect.w - 190.0,
                ),
                rect.x + 84.0,
                if anim_label.is_some() {
                    rect.y + 17.0
                } else {
                    rect.y + 23.0
                },
                if texture.is_empty() || face_count == 0 || thumbnail.is_none() {
                    RED
                } else {
                    WHITE
                },
            );
            if let Some(anim_label) = anim_label {
                ui_text_size(
                    &app.ui_font,
                    &ellipsize_width(&anim_label, 13, rect.w - 190.0),
                    rect.x + 84.0,
                    rect.y + 31.0,
                    13,
                    GREEN,
                );
            }
            ui_text_size(
                &app.ui_font,
                &face_label,
                rect.x + rect.w - face_w - 14.0,
                rect.y + 23.0,
                14,
                ui_muted(),
            );
        }
        if total > visible {
            let track_x = list.x + list.w - 4.0;
            let track_h = list.h - 6.0;
            draw_rrect(track_x, list.y + 2.0, 3.0, track_h, 1.5, ui_border());
            let thumb_h = (track_h * visible as f32 / total as f32).max(16.0);
            let frac = if max_start > 0 {
                start as f32 / max_start as f32
            } else {
                0.0
            };
            draw_rrect(
                track_x,
                list.y + 2.0 + frac * (track_h - thumb_h),
                3.0,
                thumb_h,
                1.5,
                ui_accent(),
            );
        }
    }

    if let Some(rect) = layout.view_texture {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.select,
            "View Full Size",
            false,
            false,
        );
    }
    if let Some(rect) = layout.rename_texture {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.duplicate,
            "Rename Texture",
            false,
            false,
        );
    }
    if let Some(rect) = layout.duplicate_texture {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.duplicate,
            "Duplicate Texture",
            false,
            false,
        );
    }
    if let Some(rect) = layout.set_material_texture_from_txd {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.select,
            "Set from TXD",
            dff.texture_picker_open && dff.texture_picker_edits_material,
            false,
        );
    }
    if let Some(rect) = layout.set_material_texture_browse {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.duplicate,
            "Import + Set",
            false,
            false,
        );
    }
    if let Some(rect) = layout.new_material_for_faces {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.duplicate,
            "New for Faces",
            false,
            false,
        );
    }
    if let Some(rect) = layout.assign_material_to_faces {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.face,
            "Assign to Faces",
            false,
            false,
        );
    }
    if let Some(rect) = layout.delete_unused_material {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.delete,
            "Remove Unused Materials",
            false,
            true,
        );
    }
    if layout.material_color.is_some() {
        draw_input_box(app, InspectorField::DffMaterialRed, "Color R (0-255)");
        draw_input_box(app, InspectorField::DffMaterialGreen, "Color G (0-255)");
        draw_input_box(app, InspectorField::DffMaterialBlue, "Color B (0-255)");
        draw_input_box(app, InspectorField::DffMaterialAlpha, "Alpha (0-255)");
    }
    let material_properties = dff
        .raw
        .materials
        .get(dff.selected_material)
        .copied()
        .unwrap_or_else(default_dff_material);
    if let Some(swatch) = layout.material_color_swatch {
        let cell = swatch.w / 4.0;
        for y in 0..6 {
            for x in 0..4 {
                let shade = if (x + y) % 2 == 0 { 0.72 } else { 0.38 };
                draw_rectangle(
                    swatch.x + x as f32 * cell,
                    swatch.y + y as f32 * (swatch.h / 6.0),
                    cell,
                    swatch.h / 6.0,
                    Color::new(shade, shade, shade, 1.0),
                );
            }
        }
        draw_rrect_bordered(
            swatch.x,
            swatch.y,
            swatch.w,
            swatch.h,
            5.0,
            1.0,
            Color::new(
                material_properties.color.x,
                material_properties.color.y,
                material_properties.color.z,
                material_properties.alpha,
            ),
            ui_border(),
        );
    }
    if let Some(presets) = layout.material_color_presets {
        for (rect, color) in presets.into_iter().zip(DFF_MATERIAL_COLOR_PRESETS) {
            draw_rrect_bordered(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                4.0,
                1.0,
                Color::new(color.x, color.y, color.z, 1.0),
                ui_border(),
            );
        }
    }
    if let Some(presets) = layout.material_alpha_presets {
        for (idx, rect) in presets.into_iter().enumerate() {
            let alpha = DFF_MATERIAL_ALPHA_PRESETS[idx];
            text_button(
                &app.ui_font,
                rect,
                &format!("{:.0}%", alpha * 100.0),
                (material_properties.alpha - alpha).abs() < 0.01,
            );
        }
    }
    if layout.material_surface.is_some() {
        draw_input_box(app, InspectorField::DffMaterialAmbient, "Ambient");
        draw_input_box(app, InspectorField::DffMaterialDiffuse, "Diffuse");
        draw_input_box(app, InspectorField::DffMaterialSpecular, "Specular");
    }
    if let Some(rect) = layout.collision_material {
        let texture = dff
            .raw
            .material_textures
            .get(dff.selected_material)
            .map(String::as_str)
            .unwrap_or("");
        let fingerprint = editing_dff_texture_fingerprint(&app.txd_textures, dff);
        let resolved = app.material_classes.resolve_with_content(
            dff.txd_context.as_deref(),
            texture,
            fingerprint,
            app.collision_generation_fallback_material,
        );
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.cube,
            &if resolved.no_collision {
                "Collision Material: No Collision".to_string()
            } else if resolved.source == MaterialClassSource::Fallback {
                format!(
                    "Collision Material: Fallback ({})",
                    col_material_label(resolved.material)
                )
            } else {
                format!(
                    "Collision Material: {}",
                    col_material_label(resolved.material)
                )
            },
            dff.collision_material_picker_open,
            false,
        );
    }

    let casts_shadow = selected_shadow_casting(app);
    let shadow_scope_global = selected_shadow_casting_is_global(app);
    let face_shadow_target = selected_dff_faces_are_emitter_target(dff);
    let shadow_face_count = if face_shadow_target {
        dff_selected_face_set(dff).len()
    } else {
        0
    };
    let shadow_faces_mixed = if face_shadow_target {
        let states: Vec<_> = dff_selected_face_set(dff)
            .into_iter()
            .map(|face| editing_dff_face_casts_shadow(app, dff, face))
            .collect();
        states.iter().any(|state| *state) && states.iter().any(|state| !*state)
    } else {
        false
    };
    if let Some(rect) = layout.shadow_casting_toggle {
        text_button(
            &app.ui_font,
            rect,
            if shadow_faces_mixed {
                "Collision Shadow: Mixed Face Inclusion"
            } else if face_shadow_target && shadow_face_count > 1 {
                if casts_shadow {
                    "Collision Shadow: Include Selected Faces"
                } else {
                    "Collision Shadow: Exclude Selected Faces"
                }
            } else if face_shadow_target {
                if casts_shadow {
                    "Collision Shadow: Include Selected Face"
                } else {
                    "Collision Shadow: Exclude Selected Face"
                }
            } else if casts_shadow {
                "Shadow Casting: Enabled"
            } else {
                "Shadow Casting: Disabled"
            },
            !casts_shadow,
        );
    }
    if let Some(rect) = layout.shadow_casting_scope {
        text_button(
            &app.ui_font,
            rect,
            if shadow_scope_global {
                "Shadow Scope: Global Texture"
            } else {
                "Shadow Scope: This DFF Material"
            },
            shadow_scope_global,
        );
    }

    let emitter = selected_material_emitter(app);
    let emitter_is_global = selected_material_emitter_is_global(app);
    let face_emitter = selected_dff_faces_are_emitter_target(dff);
    let face_count = if face_emitter {
        dff_selected_face_set(dff).len()
    } else {
        0
    };
    if let Some(rect) = layout.emitter_toggle {
        text_button(
            &app.ui_font,
            rect,
            if emitter.enabled && face_emitter {
                "Selected Face Emitter: Enabled"
            } else if emitter.enabled {
                "Light Emitter: Enabled"
            } else if face_emitter && face_count > 1 {
                "Mark Selected Faces as Light Emitters"
            } else if face_emitter {
                "Mark Selected Face as Light Emitter"
            } else {
                "Mark as Light Emitter"
            },
            emitter.enabled,
        );
    }
    if let Some(rect) = layout.emitter_source {
        text_button(
            &app.ui_font,
            rect,
            if emitter.use_temperature {
                "Color Source: Temperature"
            } else if emitter.use_material_color {
                "Color Source: Base Material"
            } else {
                "Color Source: Custom"
            },
            !emitter.use_material_color || emitter.use_temperature,
        );
        let swatch = Rect::new(rect.x + rect.w - 38.0, rect.y + 6.0, 28.0, rect.h - 12.0);
        let resolved_material = if face_emitter {
            dff.selected_face
                .and_then(|face| dff.raw.triangles.get(face))
                .map(|triangle| triangle.material as usize)
                .unwrap_or(dff.selected_material)
        } else {
            dff.selected_material
        };
        let resolved = if emitter.use_temperature {
            color_from_temperature(emitter.temperature)
        } else if emitter.use_material_color {
            dff.raw
                .materials
                .get(resolved_material)
                .map(|material| material.color)
                .unwrap_or_else(neutral_vertex_color)
        } else {
            emitter.color
        };
        draw_rrect_bordered(
            swatch.x,
            swatch.y,
            swatch.w,
            swatch.h,
            5.0,
            1.0,
            Color::new(resolved.x, resolved.y, resolved.z, 1.0),
            WHITE,
        );
    }
    if let Some(rect) = layout.emitter_scope {
        text_button(
            &app.ui_font,
            rect,
            if emitter_is_global {
                "Scope: Global Texture"
            } else {
                "Scope: This DFF Material"
            },
            emitter_is_global,
        );
    }
    if let Some(rect) = layout.emitter_cast_mode {
        text_button(
            &app.ui_font,
            rect,
            match emitter.cast_mode {
                MaterialEmitterCastMode::Face => "Casting Mode: Face",
                MaterialEmitterCastMode::Point => "Casting Mode: Point",
            },
            emitter.cast_mode == MaterialEmitterCastMode::Point,
        );
    }
    if layout.emitter_max_grouping_size.is_some() {
        draw_input_box(
            app,
            InspectorField::DffEmitterMaxGroupingSize,
            "Maximum Grouping Size",
        );
    }
    if layout.emitter_point_up_strength.is_some() {
        draw_input_box(
            app,
            InspectorField::DffEmitterPointUpStrength,
            "Up Strength",
        );
    }
    if layout.emitter_point_down_strength.is_some() {
        draw_input_box(
            app,
            InspectorField::DffEmitterPointDownStrength,
            "Down Strength",
        );
    }
    if layout.emitter_point_sides_strength.is_some() {
        draw_input_box(
            app,
            InspectorField::DffEmitterPointSidesStrength,
            "Sides Strength",
        );
    }
    if let Some(rect) = layout.emitter_day {
        text_button(&app.ui_font, rect, "Day", emitter.day);
    }
    if let Some(rect) = layout.emitter_night {
        text_button(&app.ui_font, rect, "Night", emitter.night);
    }
    if let Some(rect) = layout.emitter_inversed {
        text_button(&app.ui_font, rect, "Emit inversed", emitter.emit_inversed);
    }
    if let Some(strength) = layout.emitter_strength {
        let _ = strength;
        draw_input_box(app, InspectorField::DffEmitterStrength, "Brightness");
    }
    if let Some(falloff) = layout.emitter_falloff {
        let _ = falloff;
        draw_input_box(app, InspectorField::DffEmitterFalloff, "Falloff Distance");
    }
    if let Some(temperature) = layout.emitter_temperature {
        let _ = temperature;
        draw_input_box(
            app,
            InspectorField::DffEmitterTemperature,
            "Temperature (K)",
        );
    }
    if let Some(color) = layout.emitter_color {
        for (idx, rect) in color.iter().enumerate() {
            let (channel, tint, value) = match idx {
                0 => ("R", RED, emitter.color.x),
                1 => ("G", GREEN, emitter.color.y),
                _ => ("B", BLUE, emitter.color.z),
            };
            ui_text(&app.ui_font, channel, rect.x - 25.0, rect.y + 17.0, tint);
            draw_rrect_bordered(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                6.0,
                1.0,
                Color::new(0.04, 0.05, 0.06, 1.0),
                ui_border(),
            );
            const STEPS: usize = 48;
            for step in 0..STEPS {
                let t = step as f32 / (STEPS - 1) as f32;
                let x = rect.x + rect.w * step as f32 / STEPS as f32;
                draw_rectangle(
                    x,
                    rect.y + 2.0,
                    rect.w / STEPS as f32 + 1.0,
                    rect.h - 4.0,
                    Color::new(t * tint.r, t * tint.g, t * tint.b, 1.0),
                );
            }
            let marker_x = rect.x + rect.w * value.clamp(0.0, 1.0);
            draw_line(
                marker_x,
                rect.y - 2.0,
                marker_x,
                rect.y + rect.h + 2.0,
                2.0,
                WHITE,
            );
            ui_text_size(
                &app.ui_font,
                &format!("{value:.2}"),
                rect.x + rect.w - 39.0,
                rect.y + 17.0,
                13,
                WHITE,
            );
        }
    }

    // Face texture.
    if let Some(rect) = layout.tex_from_txd {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.select,
            "New from TXD",
            dff.texture_picker_open && !dff.texture_picker_edits_material,
            false,
        );
    }
    if let Some(rect) = layout.tex_browse {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.duplicate,
            "New from File",
            false,
            false,
        );
    }

    // Material animation.
    if let Some(rect) = layout.anim_assign {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.select,
            "Assign Anim",
            dff.uv_anim_picker_open,
            false,
        );
    }
    if let Some(rect) = layout.anim_clear {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.delete,
            "Clear Anim",
            false,
            true,
        );
    }
    if let Some(motion) = layout.anim_motion {
        for (idx, rect) in motion.iter().enumerate() {
            let axis = if idx / 2 == 0 { "U" } else { "V" };
            let sign = if idx % 2 == 1 { "+" } else { "-" };
            text_button(&app.ui_font, *rect, &format!("{axis}{sign}"), false);
        }
    }

    // UV tools.
    if let Some(nudge) = layout.uv_nudge {
        for (idx, rect) in nudge.iter().enumerate() {
            let axis = if idx / 2 == 0 { "U" } else { "V" };
            let sign = if idx % 2 == 1 { "+" } else { "-" };
            text_button(&app.ui_font, *rect, &format!("{axis}{sign}"), false);
        }
    }
    if let Some(scale) = layout.uv_scale {
        text_button(&app.ui_font, scale[0], "Scale -", false);
        text_button(&app.ui_font, scale[1], "Scale +", false);
    }
    if let Some(rotate) = layout.uv_rotate {
        text_button(&app.ui_font, rotate[0], "Rotate -15", false);
        text_button(&app.ui_font, rotate[1], "Rotate +15", false);
    }
    if let Some(rect) = layout.uv_unwrap_face {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.face,
            "Unwrap Face",
            false,
            false,
        );
    }
    if let Some(rect) = layout.uv_unwrap_material {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.select,
            "Unwrap Material",
            false,
            false,
        );
    }

    // Mesh tools.
    if let Some(rect) = layout.make_face {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.duplicate,
            "Make Face",
            matches!(dff_selected_vertex_set(dff).len(), 3 | 4)
                || dff_selected_edge_set(dff).len() == 2,
            false,
        );
    }
    if let Some(rect) = layout.delete_face {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.delete,
            "Delete Face",
            false,
            true,
        );
    }
    if let Some(rect) = layout.delete_vertex {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.delete,
            "Delete Vertex",
            false,
            true,
        );
    }
    if let Some(rect) = layout.delete_material_faces {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.delete,
            "Delete Mat Faces",
            false,
            true,
        );
    }
    if let Some(rect) = layout.extrude_selection {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.duplicate,
            "Extrude Selection",
            !dff_selected_vertex_set(dff).is_empty()
                || !dff_selected_edge_set(dff).is_empty()
                || !dff_selected_face_set(dff).is_empty(),
            false,
        );
    }
    if let Some(rect) = layout.merge_selected {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.vertex,
            "Merge Selected",
            dff_selected_vertex_set(dff).len() >= 2,
            false,
        );
    }
    if let Some(rect) = layout.merge_distance {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.vertex,
            "Merge by Distance",
            false,
            false,
        );
    }
    if let Some(rect) = layout.subdivide {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.face,
            "Subdivide",
            !dff_selected_face_set(dff).is_empty(),
            false,
        );
    }
    if let Some(rect) = layout.duplicate_faces {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.duplicate,
            "Duplicate Faces",
            false,
            false,
        );
    }
    if let Some(rect) = layout.duplicate_material {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.duplicate,
            "Duplicate Material",
            false,
            false,
        );
    }
    if let Some(rect) = layout.separate_faces {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.duplicate,
            "Separate from Object",
            !dff_selected_face_set(dff).is_empty(),
            false,
        );
    }
    if let Some(rect) = layout.pivot_to_selection {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.vertex,
            "Pivot to Selection",
            selected_dff_geometry_pivot(dff).is_some(),
            false,
        );
    }
    if let Some(rect) = layout.pivot_to_bounds {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.cube,
            "Pivot to Bounds",
            !dff.raw.vertices.is_empty(),
            false,
        );
    }

    // Boolean cutter.
    if let Some(rect) = layout.cutter_add {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.cube,
            "Add Cutter",
            dff.boolean_box.is_some(),
            false,
        );
    }
    if let Some(rect) = layout.cutter_clear {
        editing_action_button(&app.ui_font, rect, &app.icons.delete, "Clear", false, true);
    }
    if let Some(resize) = layout.cutter_resize {
        for (idx, rect) in resize.iter().enumerate() {
            let axis = ["X", "Y", "Z"][idx / 2];
            let sign = if idx % 2 == 1 { "+" } else { "-" };
            text_button(&app.ui_font, *rect, &format!("{axis}{sign}"), false);
        }
    }
    end_ui_clip();

    // Panel scrollbar.
    let max_scroll = dff_panel_max_scroll(&layout);
    if max_scroll > 0.0 {
        let track_x = layout.content.x + layout.content.w - 5.0;
        let track_y = layout.content.y + 2.0;
        let track_h = layout.content.h - 4.0;
        draw_rrect(track_x, track_y, 3.0, track_h, 1.5, ui_border());
        let thumb_h = (track_h * layout.content.h / layout.content_height.max(1.0)).max(24.0);
        let frac = (dff.panel_scroll / max_scroll).clamp(0.0, 1.0);
        draw_rrect(
            track_x,
            track_y + frac * (track_h - thumb_h),
            3.0,
            thumb_h,
            1.5,
            ui_accent(),
        );
    }

    // Pinned footer.
    draw_rrect(
        right.x + 1.0,
        layout.flip_normals.y - 8.0,
        right.w - 2.0,
        1.0,
        0.5,
        ui_border(),
    );
    editing_action_button(
        &app.ui_font,
        layout.generate_collision,
        &app.icons.cube,
        &format!(
            "COL {}",
            collision_generation_preset_label(app.collision_generation_preset)
        ),
        app.collision_generation_job.is_some() || app.shadow_mesh_generation_job.is_some(),
        false,
    );
    editing_action_button(
        &app.ui_font,
        layout.flip_normals,
        &app.icons.rotate,
        "Flip Normals",
        false,
        false,
    );
    editing_action_button(
        &app.ui_font,
        layout.stage,
        &app.icons.save,
        if dff.normalized_warning && !dff.normalized_rewrite_confirmed {
            "Confirm Rewrite & Stage"
        } else if dff.dirty {
            "Stage for Project Save *"
        } else {
            "Stage for Project Save"
        },
        dff.dirty,
        false,
    );
    draw_dff_texture_picker_popup(app, dff);
    draw_dff_uv_anim_picker_popup(app, dff);
    draw_dff_2dfx_type_picker_popup(app, dff);
    draw_dff_2dfx_corona_preset_popup(app, dff);
    draw_dff_2dfx_payload_editor_popup(app, dff);
    draw_dff_collision_material_picker(app, dff);
}

fn draw_dff_2dfx_corona_preset_popup(app: &AppState, dff: &EditingDffState) {
    if !dff.dff_2dfx_corona_preset_picker_open {
        return;
    }
    let popup = editing_dff_2dfx_corona_preset_picker_rect();
    draw_rrect_bordered(
        popup.x,
        popup.y,
        popup.w,
        popup.h,
        7.0,
        1.0,
        Color::new(0.04, 0.047, 0.058, 1.0),
        ui_accent(),
    );
    ui_text_bold(
        "Add corona preset",
        popup.x + 12.0,
        popup.y + 26.0,
        18,
        WHITE,
    );
    text_button(
        &app.ui_font,
        editing_dff_2dfx_corona_preset_close_rect(),
        "Close",
        false,
    );
    ui_text_size(
        &app.ui_font,
        "Uses stock GTA:SA values, textures, show mode, and flags",
        popup.x + 12.0,
        popup.y + 48.0,
        13,
        ui_muted(),
    );
    let list = editing_dff_2dfx_corona_preset_list_rect();
    let row_h = 42.0;
    let mouse: Vec2 = mouse_position().into();
    for (row, preset) in DFF_CORONA_PRESETS.iter().enumerate() {
        let rect = Rect::new(list.x, list.y + row as f32 * row_h, list.w, row_h - 4.0);
        let hovered = rect.contains(mouse);
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
                Color::new(0.055, 0.064, 0.078, 1.0)
            },
            ui_border(),
        );
        let swatch = Rect::new(rect.x + 8.0, rect.y + 7.0, 24.0, 24.0);
        draw_rrect_bordered(
            swatch.x,
            swatch.y,
            swatch.w,
            swatch.h,
            5.0,
            1.0,
            Color::new(
                preset.color[0] as f32 / 255.0,
                preset.color[1] as f32 / 255.0,
                preset.color[2] as f32 / 255.0,
                1.0,
            ),
            ui_border(),
        );
        ui_text_size(
            &app.ui_font,
            preset.label,
            rect.x + 40.0,
            rect.y + 16.0,
            15,
            WHITE,
        );
        ui_text_size(
            &app.ui_font,
            &ellipsize_width(preset.detail, 12, rect.w - 52.0),
            rect.x + 40.0,
            rect.y + 31.0,
            12,
            ui_muted(),
        );
    }
}

fn draw_dff_2dfx_type_picker_popup(app: &AppState, dff: &EditingDffState) {
    if !dff.dff_2dfx_type_picker_open {
        return;
    }
    let popup = editing_dff_2dfx_type_picker_rect();
    draw_rrect_bordered(
        popup.x,
        popup.y,
        popup.w,
        popup.h,
        7.0,
        1.0,
        Color::new(0.04, 0.047, 0.058, 1.0),
        ui_accent(),
    );
    ui_text_bold(
        "Choose 2DFX type",
        popup.x + 12.0,
        popup.y + 26.0,
        18,
        WHITE,
    );
    text_button(
        &app.ui_font,
        editing_dff_2dfx_type_picker_close_rect(),
        "Close",
        false,
    );
    let search = editing_dff_2dfx_type_picker_search_rect();
    draw_rrect_bordered(
        search.x,
        search.y,
        search.w,
        search.h,
        5.0,
        1.0,
        Color::new(0.025, 0.030, 0.038, 1.0),
        ui_border(),
    );
    ui_text(
        &app.ui_font,
        if dff.dff_2dfx_type_picker_search.is_empty() {
            "Search type or id"
        } else {
            &dff.dff_2dfx_type_picker_search
        },
        search.x + 10.0,
        search.y + 20.0,
        if dff.dff_2dfx_type_picker_search.is_empty() {
            ui_muted()
        } else {
            WHITE
        },
    );
    let types = dff_2dfx_filtered_types(&dff.dff_2dfx_type_picker_search);
    let list = editing_dff_2dfx_type_picker_list_rect();
    let row_h = 28.0;
    let visible = (list.h / row_h).floor().max(1.0) as usize;
    let max_start = types.len().saturating_sub(visible) as f32;
    let start = dff
        .dff_2dfx_type_picker_scroll
        .floor()
        .max(0.0)
        .min(max_start) as usize;
    let current = dff
        .selected_2dfx
        .and_then(|idx| dff.raw.effects_2dfx.get(idx))
        .map(|effect| effect.effect_id);
    for row in 0..visible {
        let Some((id, label)) = types.get(start + row).copied() else {
            break;
        };
        let rect = Rect::new(list.x, list.y + row as f32 * row_h, list.w, row_h - 4.0);
        let selected = current == Some(id);
        let hovered = rect.contains(mouse_position().into());
        draw_rrect_bordered(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            5.0,
            1.0,
            if selected {
                ui_surface_active()
            } else if hovered {
                ui_surface_hover()
            } else {
                Color::new(0.055, 0.064, 0.078, 1.0)
            },
            ui_border(),
        );
        ui_text(
            &app.ui_font,
            &format!("{id:>2}  {label}"),
            rect.x + 10.0,
            rect.y + 19.0,
            if selected { ui_accent() } else { WHITE },
        );
    }
}

fn draw_dff_2dfx_payload_editor_popup(app: &AppState, dff: &EditingDffState) {
    if !dff.dff_2dfx_payload_editor_open {
        return;
    }
    let popup = editing_dff_2dfx_payload_editor_rect();
    draw_rrect_bordered(
        popup.x,
        popup.y,
        popup.w,
        popup.h,
        7.0,
        1.0,
        Color::new(0.04, 0.047, 0.058, 1.0),
        ui_accent(),
    );
    let title = dff
        .selected_2dfx
        .filter(|idx| dff.raw.effects_2dfx.get(*idx).is_some())
        .map(|idx| dff_2dfx_effect_display_name(&dff.raw.effects_2dfx, idx))
        .unwrap_or_else(|| "2DFX properties".to_string());
    ui_text_bold(&title, popup.x + 12.0, popup.y + 26.0, 18, WHITE);
    text_button(
        &app.ui_font,
        editing_dff_2dfx_payload_close_rect(),
        "Close",
        false,
    );
    if let Some(effect) = dff
        .selected_2dfx
        .and_then(|idx| dff.raw.effects_2dfx.get(idx))
        .filter(|effect| effect.effect_id == 1)
    {
        let name = fixed_string(&effect.payload, 0, 24);
        let detail = app
            .particle_effects
            .iter()
            .find(|def| def.name.eq_ignore_ascii_case(&name))
            .map(|def| {
                let textures = if def.textures.is_empty() {
                    "no texture refs".to_string()
                } else {
                    def.textures.join(", ")
                };
                format!(
                    "FXP: {} prim(s), cull {:.1}, textures {}",
                    def.primitive_count, def.cull_distance, textures
                )
            })
            .unwrap_or_else(|| format!("FXP: {name} not found in effects.fxp"));
        ui_text(
            &app.ui_font,
            &ellipsize_width(&detail, 13, popup.w - 120.0),
            popup.x + 12.0,
            popup.y + 44.0,
            ui_muted(),
        );
    }
    if selected_dff_2dfx_effect_id(dff) == Some(0) {
        let r = dff_2dfx_light_color_value(dff, 0) as f32 / 255.0;
        let g = dff_2dfx_light_color_value(dff, 1) as f32 / 255.0;
        let b = dff_2dfx_light_color_value(dff, 2) as f32 / 255.0;
        let swatch = editing_dff_2dfx_light_color_swatch_rect();
        draw_rrect_bordered(
            swatch.x,
            swatch.y,
            swatch.w,
            swatch.h,
            6.0,
            1.0,
            Color::new(r, g, b, 1.0),
            ui_border(),
        );
        let channels = [("R", RED, r), ("G", GREEN, g), ("B", BLUE, b)];
        for (idx, (label, color, value)) in channels.into_iter().enumerate() {
            let rect = editing_dff_2dfx_light_color_bar_rect(idx);
            ui_text(&app.ui_font, label, rect.x - 28.0, rect.y + 13.0, LIGHTGRAY);
            draw_rrect_bordered(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                6.0,
                1.0,
                Color::new(0.050, 0.058, 0.070, 1.0),
                ui_border(),
            );
            draw_rrect(
                rect.x,
                rect.y,
                rect.w * value.clamp(0.0, 1.0),
                rect.h,
                6.0,
                color,
            );
            let marker_x = rect.x + rect.w * value.clamp(0.0, 1.0);
            draw_line(
                marker_x,
                rect.y - 2.0,
                marker_x,
                rect.y + rect.h + 2.0,
                2.0,
                WHITE,
            );
        }
    }
    let text = editing_dff_2dfx_payload_text_rect();
    let specs = dff
        .selected_2dfx
        .and_then(|idx| dff.raw.effects_2dfx.get(idx))
        .map(|effect| dff_2dfx_prop_specs(effect.effect_id))
        .unwrap_or_default();
    let row_h = 34.0;
    let visible = (text.h / row_h).floor().max(1.0) as usize;
    let max_start = dff.dff_2dfx_payload_fields.len().saturating_sub(visible) as f32;
    let start = dff
        .dff_2dfx_payload_field_scroll
        .floor()
        .max(0.0)
        .min(max_start) as usize;
    for row in 0..visible {
        let idx = start + row;
        let Some(value) = dff.dff_2dfx_payload_fields.get(idx) else {
            break;
        };
        let spec_label = specs.get(idx).map(|spec| spec.label).unwrap_or("Property");
        let rect = Rect::new(text.x, text.y + row as f32 * row_h, text.w, row_h - 4.0);
        let active = dff.dff_2dfx_payload_active_field == Some(idx);
        let hovered = rect.contains(mouse_position().into());
        draw_rrect_bordered(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            5.0,
            1.0,
            if active {
                ui_surface_active()
            } else if hovered {
                ui_surface_hover()
            } else {
                Color::new(0.055, 0.064, 0.078, 1.0)
            },
            ui_border(),
        );
        ui_text(
            &app.ui_font,
            &ellipsize_width(spec_label, 14, 132.0),
            rect.x + 10.0,
            rect.y + 20.0,
            ui_muted(),
        );
        ui_text(
            &app.ui_font,
            &ellipsize_width(value, 15, rect.w - 158.0),
            rect.x + 146.0,
            rect.y + 20.0,
            if active { WHITE } else { LIGHTGRAY },
        );
        if active && (get_time() * 2.0) as i32 % 2 == 0 {
            let caret_x = rect.x + 146.0 + ui_text_width(value, 15).min(rect.w - 164.0);
            draw_line(
                caret_x,
                rect.y + 7.0,
                caret_x,
                rect.y + rect.h - 7.0,
                1.0,
                ui_accent(),
            );
        }
    }
    if dff.dff_2dfx_payload_fields.len() > visible {
        let track_x = text.x + text.w - 4.0;
        draw_rrect(track_x, text.y, 3.0, text.h, 1.5, ui_border());
        let thumb_h =
            (text.h * visible as f32 / dff.dff_2dfx_payload_fields.len() as f32).max(16.0);
        let frac = if max_start > 0.0 {
            start as f32 / max_start
        } else {
            0.0
        };
        let thumb_y = text.y + frac * (text.h - thumb_h);
        draw_rrect(track_x, thumb_y, 3.0, thumb_h, 1.5, ui_accent());
    }
    if dff_2dfx_active_payload_is_particle_name(dff) {
        let picker = editing_dff_2dfx_particle_picker_rect();
        draw_rrect_bordered(
            picker.x,
            picker.y,
            picker.w,
            picker.h,
            6.0,
            1.0,
            Color::new(0.030, 0.036, 0.046, 0.98),
            ui_accent(),
        );
        let names = dff_2dfx_particle_name_options(app, dff);
        let row_h = 26.0;
        let visible = (picker.h / row_h).floor().max(1.0) as usize;
        let max_start = names.len().saturating_sub(visible) as f32;
        let start = dff
            .dff_2dfx_particle_picker_scroll
            .floor()
            .max(0.0)
            .min(max_start) as usize;
        if names.is_empty() {
            ui_text(
                &app.ui_font,
                "No parsed effects.fxp particle names match",
                picker.x + 10.0,
                picker.y + 22.0,
                ui_muted(),
            );
        }
        for row in 0..visible {
            let Some(name) = names.get(start + row) else {
                break;
            };
            let rect = Rect::new(
                picker.x + 4.0,
                picker.y + 4.0 + row as f32 * row_h,
                picker.w - 8.0,
                row_h - 4.0,
            );
            let hovered = rect.contains(mouse_position().into());
            if hovered {
                draw_rrect(rect.x, rect.y, rect.w, rect.h, 4.0, ui_surface_hover());
            }
            let detail = app
                .particle_effects
                .iter()
                .find(|effect| effect.name.eq_ignore_ascii_case(name))
                .map(|effect| {
                    if effect.textures.is_empty() {
                        format!(
                            "{} prim(s), cull {:.0}",
                            effect.primitive_count, effect.cull_distance
                        )
                    } else {
                        format!(
                            "{} prim(s), cull {:.0}, {}",
                            effect.primitive_count,
                            effect.cull_distance,
                            effect.textures.join(", ")
                        )
                    }
                })
                .unwrap_or_default();
            ui_text(
                &app.ui_font,
                &ellipsize_width(name, 15, 130.0),
                rect.x + 8.0,
                rect.y + 17.0,
                WHITE,
            );
            ui_text(
                &app.ui_font,
                &ellipsize_width(&detail, 12, rect.w - 150.0),
                rect.x + 146.0,
                rect.y + 17.0,
                ui_muted(),
            );
        }
    }
    text_button(
        &app.ui_font,
        editing_dff_2dfx_payload_apply_rect(),
        "Apply",
        dff.dff_2dfx_payload_fields.is_empty(),
    );
}

fn draw_dff_uv_anim_picker_popup(app: &AppState, dff: &EditingDffState) {
    if !dff.uv_anim_picker_open {
        return;
    }
    let popup = editing_dff_uv_anim_picker_rect();
    draw_rrect_bordered(
        popup.x,
        popup.y,
        popup.w,
        popup.h,
        7.0,
        1.0,
        Color::new(0.04, 0.047, 0.058, 1.0),
        ui_accent(),
    );
    ui_text_bold(
        "Assign material animation",
        popup.x + 12.0,
        popup.y + 26.0,
        18,
        WHITE,
    );
    text_button(
        &app.ui_font,
        editing_dff_uv_anim_picker_close_rect(),
        "Close",
        false,
    );

    let search = editing_dff_uv_anim_picker_search_rect();
    draw_rrect_bordered(
        search.x,
        search.y,
        search.w,
        search.h,
        5.0,
        1.0,
        Color::new(0.025, 0.030, 0.038, 1.0),
        ui_border(),
    );
    if dff.uv_anim_picker_search.is_empty() {
        ui_text(
            &app.ui_font,
            "Search or type animation name",
            search.x + 10.0,
            search.y + 20.0,
            ui_muted(),
        );
    } else {
        ui_text(
            &app.ui_font,
            &ellipsize_width(&dff.uv_anim_picker_search, 16, search.w - 24.0),
            search.x + 10.0,
            search.y + 20.0,
            WHITE,
        );
    }
    if (get_time() * 2.0) as i32 % 2 == 0 {
        let caret_x =
            search.x + 10.0 + ui_text_width(&dff.uv_anim_picker_search, 16).min(search.w - 28.0);
        draw_line(
            caret_x,
            search.y + 7.0,
            caret_x,
            search.y + search.h - 7.0,
            1.0,
            ui_accent(),
        );
    }

    let names = editing_dff_filtered_uv_anim_options(dff);
    let list = editing_dff_uv_anim_picker_list_rect();
    let row_h = 26.0;
    let visible = (list.h / row_h).floor().max(1.0) as usize;
    let max_start = names.len().saturating_sub(visible) as f32;
    let start = dff.uv_anim_picker_scroll.floor().max(0.0).min(max_start) as usize;
    let current = dff
        .raw
        .material_animations
        .get(dff.selected_material)
        .and_then(|animation| animation.names.first())
        .cloned()
        .unwrap_or_default();
    if names.is_empty() {
        ui_text(
            &app.ui_font,
            "No matching UV animation refs; type a name and apply it.",
            list.x + 4.0,
            list.y + 22.0,
            ui_muted(),
        );
    }
    for row in 0..visible {
        let Some(name) = names.get(start + row) else {
            break;
        };
        let rect = Rect::new(list.x, list.y + row as f32 * row_h, list.w, row_h - 3.0);
        let hovered = rect.contains(mouse_position().into());
        let selected = name.eq_ignore_ascii_case(&current);
        draw_rrect_bordered(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            5.0,
            1.0,
            if selected {
                ui_surface_active()
            } else if hovered {
                ui_surface_hover()
            } else {
                Color::new(0.055, 0.064, 0.078, 1.0)
            },
            ui_border(),
        );
        ui_text(
            &app.ui_font,
            &ellipsize_width(name, 16, rect.w - 20.0),
            rect.x + 10.0,
            rect.y + 18.0,
            if selected { ui_accent() } else { WHITE },
        );
    }
    if names.len() > visible {
        let track_x = list.x + list.w - 4.0;
        draw_rrect(track_x, list.y, 3.0, list.h, 1.5, ui_border());
        let thumb_h = (list.h * visible as f32 / names.len() as f32).max(14.0);
        let frac = if max_start > 0.0 {
            start as f32 / max_start
        } else {
            0.0
        };
        let thumb_y = list.y + frac * (list.h - thumb_h);
        draw_rrect(track_x, thumb_y, 3.0, thumb_h, 1.5, ui_accent());
    }
    text_button(
        &app.ui_font,
        editing_dff_uv_anim_picker_apply_rect(),
        "Apply Typed",
        dff.uv_anim_picker_search.trim().is_empty(),
    );
    text_button(
        &app.ui_font,
        editing_dff_uv_anim_picker_gif_rect(),
        "From GIF...",
        false,
    );
}

fn draw_dff_texture_picker_popup(app: &AppState, dff: &EditingDffState) {
    if !dff.texture_picker_open {
        return;
    }
    let popup = editing_dff_texture_picker_rect();
    draw_rrect_bordered(
        popup.x,
        popup.y,
        popup.w,
        popup.h,
        7.0,
        1.0,
        Color::new(0.04, 0.047, 0.058, 1.0),
        ui_accent(),
    );
    ui_text_bold(
        if dff.texture_picker_edits_material {
            "Set selected material texture"
        } else {
            "Create material from TXD texture"
        },
        popup.x + 12.0,
        popup.y + 26.0,
        18,
        WHITE,
    );
    let close = editing_dff_texture_picker_close_rect();
    text_button(&app.ui_font, close, "Close", false);

    let names = editing_dff_picker_texture_names(app);
    let list = editing_dff_texture_picker_list_rect();
    let visible = (list.h / DFF_TEXTURE_PICKER_ROW_H).floor().max(1.0) as usize;
    let max_start = names.len().saturating_sub(visible) as f32;
    let start = dff.texture_picker_scroll.floor().max(0.0).min(max_start) as usize;
    if names.is_empty() {
        ui_text(
            &app.ui_font,
            "No textures found in this model's TXD",
            list.x + 4.0,
            list.y + 22.0,
            ui_muted(),
        );
    }
    let current = dff
        .raw
        .material_textures
        .get(dff.selected_material)
        .cloned()
        .unwrap_or_default();
    for row in 0..visible {
        let Some(name) = names.get(start + row) else {
            break;
        };
        let rect = Rect::new(
            list.x,
            list.y + row as f32 * DFF_TEXTURE_PICKER_ROW_H,
            list.w,
            DFF_TEXTURE_PICKER_ROW_H - 4.0,
        );
        let hovered = rect.contains(mouse_position().into());
        let selected = name.eq_ignore_ascii_case(&current);
        draw_rrect_bordered(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            5.0,
            1.0,
            if selected {
                ui_surface_active()
            } else if hovered {
                ui_surface_hover()
            } else {
                Color::new(0.055, 0.064, 0.078, 1.0)
            },
            ui_border(),
        );
        ui_text(
            &app.ui_font,
            &ellipsize_width(name, 16, rect.w - 20.0),
            rect.x + 10.0,
            rect.y + 18.0,
            if selected { ui_accent() } else { WHITE },
        );
    }
    if names.len() > visible {
        let track_x = list.x + list.w - 4.0;
        draw_rrect(track_x, list.y, 3.0, list.h, 1.5, ui_border());
        let thumb_h = (list.h * visible as f32 / names.len() as f32).max(14.0);
        let frac = if max_start > 0.0 {
            start as f32 / max_start
        } else {
            0.0
        };
        let thumb_y = list.y + frac * (list.h - thumb_h);
        draw_rrect(track_x, thumb_y, 3.0, thumb_h, 1.5, ui_accent());
    }
}

fn draw_col_asset(app: &AppState, right: Rect, col: &EditingColState) {
    draw_editing_select_mode_toggle(app, col.select_mode);
    let mouse: Vec2 = mouse_position().into();
    let layout = col_panel_layout(col);
    let (collision_vertices, collision_faces, shadow_vertices, shadow_faces) = if col.editing_shadow
    {
        (
            col.mesh.shadow_vertices.len(),
            col.mesh.shadow_faces.len(),
            col.mesh.vertices.len(),
            col.mesh.faces.len(),
        )
    } else {
        (
            col.mesh.vertices.len(),
            col.mesh.faces.len(),
            col.mesh.shadow_vertices.len(),
            col.mesh.shadow_faces.len(),
        )
    };

    // Fixed header.
    ui_text_bold(
        &col.name,
        right.x + 16.0,
        right.y + 30.0,
        18,
        if col.dirty { YELLOW } else { WHITE },
    );
    ui_text_size(
        &app.ui_font,
        &ellipsize_width(
            &format!(
                "Collision v{} f{}   Shadow v{} f{}   Spheres {}   Boxes {}   Rotated {}   Capsules {}",
                collision_vertices,
                collision_faces,
                shadow_vertices,
                shadow_faces,
                col.mesh.spheres.len(),
                col.mesh.boxes.len(),
                col.cuboids.len(),
                col.capsules.len()
            ),
            15,
            right.w - 32.0,
        ),
        right.x + 16.0,
        right.y + 56.0,
        15,
        ui_dim(),
    );
    let selected_line = if let Some(selected) = col.selected_primitive {
        match selected.kind {
            CollisionPrimitiveKind::Sphere => col.mesh.spheres.get(selected.index).map(|sphere| {
                format!(
                    "Sphere {:03}   center {:.3}, {:.3}, {:.3}   radius {:.3}",
                    selected.index,
                    sphere.center.x,
                    sphere.center.y,
                    sphere.center.z,
                    sphere.radius
                )
            }),
            CollisionPrimitiveKind::Box => col.mesh.boxes.get(selected.index).map(|col_box| {
                let center = V3 {
                    x: (col_box.min.x + col_box.max.x) * 0.5,
                    y: (col_box.min.y + col_box.max.y) * 0.5,
                    z: (col_box.min.z + col_box.max.z) * 0.5,
                };
                format!(
                    "Box {:03}   center {:.3}, {:.3}, {:.3}",
                    selected.index, center.x, center.y, center.z
                )
            }),
            CollisionPrimitiveKind::Cuboid => col.cuboids.get(selected.index).map(|cuboid| {
                format!(
                    "Rotated Box {:03}   center {:.3}, {:.3}, {:.3}   rotation {:.1}, {:.1}, {:.1}",
                    selected.index,
                    cuboid.center.x,
                    cuboid.center.y,
                    cuboid.center.z,
                    cuboid.rotation.x,
                    cuboid.rotation.y,
                    cuboid.rotation.z
                )
            }),
            CollisionPrimitiveKind::Capsule => col.capsules.get(selected.index).map(|capsule| {
                let center = (to_mq(capsule.start) + to_mq(capsule.end)) * 0.5;
                format!(
                    "Capsule {:03}   center {:.3}, {:.3}, {:.3}   radius {:.3}   length {:.3}",
                    selected.index,
                    center.x,
                    center.y,
                    center.z,
                    capsule.radius,
                    (to_mq(capsule.end) - to_mq(capsule.start)).length()
                )
            }),
        }
    } else {
        selected_col_vertex_index(col).and_then(|vertex_idx| {
            col.mesh.vertices.get(vertex_idx).map(|vertex| {
                format!(
                    "Face {:04}   Vertex {}   {:.3}, {:.3}, {:.3}",
                    col.selected_face, vertex_idx, vertex.x, vertex.y, vertex.z
                )
            })
        })
    };
    ui_text(
        &app.ui_font,
        &ellipsize_width(
            selected_line
                .as_deref()
                .unwrap_or("Click a vertex or face in the preview to edit it."),
            16,
            right.w - 32.0,
        ),
        right.x + 16.0,
        right.y + 78.0,
        if selected_line.is_some() {
            LIGHTGRAY
        } else {
            ui_muted()
        },
    );
    ui_text(
        &app.ui_font,
        &format!(
            "{} selected face(s), {} selected vertices",
            col_selected_face_set(col).len(),
            col_selected_vertex_set(col).len()
        ),
        right.x + 16.0,
        right.y + 100.0,
        ui_muted(),
    );
    let hover_vertex_idx = col
        .hovered_face
        .and_then(|face_idx| col.mesh.faces.get(face_idx).map(|face| (face_idx, face)))
        .map(|(face_idx, face)| {
            (
                face_idx,
                collision_selected_vertex_index_from_face(face, col.hovered_vertex.unwrap_or(0)),
            )
        });
    if let Some((face, vertex_idx)) = hover_vertex_idx {
        if let Some(vertex) = col.mesh.vertices.get(vertex_idx) {
            ui_text_size(
                &app.ui_font,
                &format!(
                    "Hover face {face:04}   vertex {vertex_idx}   {:.3}, {:.3}, {:.3}",
                    vertex.x, vertex.y, vertex.z
                ),
                right.x + 16.0,
                right.y + 122.0,
                14,
                ui_accent(),
            );
        }
    }
    let overlay_label = col
        .dff_overlay_name
        .as_deref()
        .map(|name| format!("DFF overlay: {}", ellipsize(name, 26)))
        .unwrap_or_else(|| "DFF overlay: none".to_string());
    ui_text_size(
        &app.ui_font,
        &overlay_label,
        right.x + 16.0,
        right.y + 142.0,
        14,
        if col.dff_overlay.is_some() {
            LIGHTGRAY
        } else {
            ui_muted()
        },
    );
    text_button(
        &app.ui_font,
        editing_col_overlay_toggle_rect(),
        if col.dff_overlay_visible {
            "Hide"
        } else {
            "Show"
        },
        col.dff_overlay_visible,
    );
    text_button(
        &app.ui_font,
        editing_col_overlay_pick_rect(),
        "Pick DFF",
        false,
    );
    text_button(
        &app.ui_font,
        editing_col_overlay_match_rect(),
        &format!(
            "Regen {}",
            collision_generation_preset_label(app.collision_generation_preset)
        ),
        app.collision_generation_job.is_some() || app.shadow_mesh_generation_job.is_some(),
    );
    text_button(
        &app.ui_font,
        editing_col_overlay_clear_rect(),
        "Clear",
        false,
    );
    text_button(
        &app.ui_font,
        editing_col_safe_rect(),
        if app.safe_collisions.is_safe(&col.name) {
            "Safe"
        } else {
            "Unsafe"
        },
        app.safe_collisions.is_safe(&col.name),
    );
    text_button(
        &app.ui_font,
        editing_col_generation_preset_rect(),
        &format!(
            "Mode: {}",
            collision_generation_preset_label(app.collision_generation_preset)
        ),
        matches!(
            app.collision_generation_preset,
            CollisionGenerationPreset::Shapes
        ),
    );
    text_button(
        &app.ui_font,
        editing_col_shadow_layer_rect(),
        if col.editing_shadow {
            "Editing: Shadow"
        } else {
            "Editing: Collision"
        },
        col.editing_shadow,
    );
    text_button(
        &app.ui_font,
        editing_col_generate_shadow_rect(),
        if app.shadow_mesh_generation_job.is_some() {
            "Generating Shadow..."
        } else {
            "Generate Closed Shadow Skin"
        },
        app.shadow_mesh_generation_job.is_some(),
    );

    // Scrollable section stack.
    begin_ui_clip(layout.content);
    for section in COL_SECTIONS {
        let idx = section as usize;
        let hint = match section {
            ColSection::Primitives => {
                if col.editing_shadow {
                    "collision layer only".to_string()
                } else {
                    format!("{}", col_primitive_selections(col).len())
                }
            }
            ColSection::Faces => format!("{}", col.mesh.faces.len()),
            ColSection::Surface => if col.selected_primitive.is_some() {
                "primitive"
            } else {
                "face"
            }
            .to_string(),
            _ => String::new(),
        };
        dff_section_header_button(
            &app.ui_font,
            layout.headers[idx],
            !col.panel_collapsed[idx],
            section.title(),
            &hint,
        );
    }

    // Primitive rows.
    let primitive_visible = layout.primitive_visible.max(1);
    let primitives = col_primitive_selections(col);
    let primitive_total = primitives.len();
    let primitive_start =
        col.primitive_scroll
            .floor()
            .max(0.0)
            .min(primitive_total.saturating_sub(primitive_visible) as f32) as usize;
    for (row, rect) in layout.primitive_rows.iter().enumerate() {
        let primitive_idx = primitive_start + row;
        let Some(primitive) = primitives.get(primitive_idx).copied() else {
            break;
        };
        let (kind, index, label, material, light) = match primitive.kind {
            CollisionPrimitiveKind::Sphere => {
                let sphere = &col.mesh.spheres[primitive.index];
                (
                    primitive.kind,
                    primitive.index,
                    format!("Sphere {:03}  r {:.2}", primitive.index, sphere.radius),
                    sphere.surface.material,
                    sphere.surface.light,
                )
            }
            CollisionPrimitiveKind::Box => {
                let col_box = &col.mesh.boxes[primitive.index];
                (
                    primitive.kind,
                    primitive.index,
                    format!("Box {:03}", primitive.index),
                    col_box.surface.material,
                    col_box.surface.light,
                )
            }
            CollisionPrimitiveKind::Cuboid => {
                let cuboid = &col.cuboids[primitive.index];
                (
                    primitive.kind,
                    primitive.index,
                    format!("Box {:03}  Rotated", primitive.index),
                    cuboid.surface.material,
                    cuboid.surface.light,
                )
            }
            CollisionPrimitiveKind::Capsule => {
                let capsule = &col.capsules[primitive.index];
                (
                    primitive.kind,
                    primitive.index,
                    format!(
                        "Capsule {:03}  r {:.2}  {}",
                        primitive.index,
                        capsule.radius,
                        if capsule.round_edges { "Round" } else { "Flat" }
                    ),
                    capsule.surface.material,
                    capsule.surface.light,
                )
            }
        };
        let selected = col
            .selected_primitive
            .is_some_and(|value| value.kind == kind && value.index == index);
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
        let swatch = collision_material_color(material, 1.0);
        draw_rrect_bordered(
            rect.x + 8.0,
            rect.y + 6.0,
            14.0,
            14.0,
            3.0,
            1.0,
            Color::new(swatch[0], swatch[1], swatch[2], 1.0),
            ui_border(),
        );
        ui_text(
            &app.ui_font,
            &label,
            rect.x + 30.0,
            rect.y + 18.0,
            if selected { ui_accent() } else { WHITE },
        );
        let surface_label = format!("{}  light {}", col_material_label(material), light);
        let surface_w = ui_text_width(&surface_label, 14).min(180.0);
        ui_text_size(
            &app.ui_font,
            &ellipsize_width(&surface_label, 14, 180.0),
            rect.x + rect.w - surface_w - 10.0,
            rect.y + 18.0,
            14,
            ui_muted(),
        );
    }
    if let Some(list) = layout.primitive_list {
        if primitive_total > primitive_visible {
            let track_x = list.x + list.w - 4.0;
            let track_h = list.h - 6.0;
            draw_rrect(track_x, list.y + 2.0, 3.0, track_h, 1.5, ui_border());
            let max_scroll = primitive_total.saturating_sub(primitive_visible).max(1) as f32;
            let thumb_h =
                (track_h * primitive_visible as f32 / primitive_total.max(1) as f32).max(16.0);
            let frac = (col.primitive_scroll / max_scroll).clamp(0.0, 1.0);
            draw_rrect(
                track_x,
                list.y + 2.0 + frac * (track_h - thumb_h),
                3.0,
                thumb_h,
                1.5,
                ui_accent(),
            );
        }
    }
    if let Some(rect) = layout.add_sphere {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.sphere,
            "Add Sphere",
            false,
            false,
        );
    }
    if let Some(rect) = layout.add_box {
        editing_action_button(&app.ui_font, rect, &app.icons.cube, "Add Box", false, false);
    }
    if let Some(rect) = layout.add_capsule {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.sphere,
            "Add Capsule (2 Spheres + Cylinder)",
            false,
            false,
        );
    }
    if let Some(rect) = layout.duplicate_primitive {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.duplicate,
            "Duplicate",
            col.selected_primitive.is_some(),
            false,
        );
    }
    if let Some(rect) = layout.box_pick_toggle {
        text_button(
            &app.ui_font,
            rect,
            if col.box_pick_enabled {
                "Pick Boxes"
            } else {
                "Ignore Boxes"
            },
            col.box_pick_enabled,
        );
    }
    if let Some(rect) = layout.capsule_edges_toggle {
        let round = col
            .selected_primitive
            .filter(|selected| selected.kind == CollisionPrimitiveKind::Capsule)
            .and_then(|selected| col.capsules.get(selected.index))
            .is_some_and(|capsule| capsule.round_edges);
        text_button(
            &app.ui_font,
            rect,
            if round { "Round Edges" } else { "Flat Edges" },
            round,
        );
    }

    // Face rows.
    if let Some(list) = layout.face_list {
        let visible = layout.face_visible.max(1);
        let start = col
            .face_scroll
            .floor()
            .max(0.0)
            .min(col.mesh.faces.len().saturating_sub(visible) as f32) as usize;
        for row in 0..visible {
            let idx = start + row;
            let Some(face) = col.mesh.faces.get(idx) else {
                break;
            };
            let rect = Rect::new(
                list.x,
                list.y + row as f32 * COL_FACE_ROW_H,
                list.w,
                COL_FACE_ROW_H - 3.0,
            );
            let selected = idx == col.selected_face || col.selected_faces.contains(&idx);
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
            let swatch = collision_material_color(face.material, 1.0);
            draw_rrect_bordered(
                rect.x + 8.0,
                rect.y + 6.0,
                14.0,
                14.0,
                3.0,
                1.0,
                Color::new(swatch[0], swatch[1], swatch[2], 1.0),
                ui_border(),
            );
            ui_text(
                &app.ui_font,
                &ellipsize_width(
                    &format!("Face {idx:04}  v{} v{} v{}", face.a, face.b, face.c),
                    16,
                    rect.w - 220.0,
                ),
                rect.x + 30.0,
                rect.y + 18.0,
                if selected { ui_accent() } else { WHITE },
            );
            let surface_label = format!(
                "{}  light {}",
                col_material_label(face.material),
                face.light
            );
            let surface_w = ui_text_width(&surface_label, 14).min(180.0);
            ui_text_size(
                &app.ui_font,
                &ellipsize_width(&surface_label, 14, 180.0),
                rect.x + rect.w - surface_w - 14.0,
                rect.y + 18.0,
                14,
                ui_muted(),
            );
        }
        if col.mesh.faces.len() > visible {
            let track_x = list.x + list.w - 4.0;
            let track_h = list.h - 6.0;
            draw_rrect(track_x, list.y + 2.0, 3.0, track_h, 1.5, ui_border());
            let max_scroll = col.mesh.faces.len().saturating_sub(visible).max(1) as f32;
            let thumb_h = (track_h * visible as f32 / col.mesh.faces.len().max(1) as f32).max(16.0);
            let frac = (col.face_scroll / max_scroll).clamp(0.0, 1.0);
            draw_rrect(
                track_x,
                list.y + 2.0 + frac * (track_h - thumb_h),
                3.0,
                thumb_h,
                1.5,
                ui_accent(),
            );
        }
    }

    // Surface inputs.
    if layout.material.is_some() {
        draw_col_material_dropdown(app);
        draw_input_box(app, InspectorField::CollisionFaceLight, "Light");
    }
    if let Some(rect) = layout.select_same_material {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.select,
            "Select all with selected material",
            false,
            false,
        );
    }
    if layout.prim_size.is_some() {
        if let Some(selected) = col.selected_primitive {
            match selected.kind {
                CollisionPrimitiveKind::Sphere => {
                    draw_input_box(app, InspectorField::CollisionPrimitiveSizeX, "Radius");
                }
                CollisionPrimitiveKind::Box | CollisionPrimitiveKind::Cuboid => {
                    draw_input_box(app, InspectorField::CollisionPrimitiveSizeX, "Size X");
                    draw_input_box(app, InspectorField::CollisionPrimitiveSizeY, "Size Y");
                    draw_input_box(app, InspectorField::CollisionPrimitiveSizeZ, "Size Z");
                }
                CollisionPrimitiveKind::Capsule => {
                    draw_input_box(app, InspectorField::CollisionPrimitiveSizeX, "Radius");
                    draw_input_box(app, InspectorField::CollisionPrimitiveSizeZ, "Length");
                }
            }
        }
    }
    if layout.prim_rotation.is_some() {
        draw_input_box(app, InspectorField::CollisionPrimitiveRotX, "Rot X");
        draw_input_box(app, InspectorField::CollisionPrimitiveRotY, "Rot Y");
        draw_input_box(app, InspectorField::CollisionPrimitiveRotZ, "Rot Z");
    }

    // Vertex inputs.
    if layout.vertex.is_some() {
        draw_input_box(app, InspectorField::CollisionVertexX, "Vertex X");
        draw_input_box(app, InspectorField::CollisionVertexY, "Vertex Y");
        draw_input_box(app, InspectorField::CollisionVertexZ, "Vertex Z");
    }

    // Mesh tools.
    if let Some(rect) = layout.delete_face {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.delete,
            match col.select_mode {
                EditingSelectMode::Vertex | EditingSelectMode::Edge | EditingSelectMode::Face => {
                    "Delete"
                }
            },
            false,
            true,
        );
    }
    if let Some(rect) = layout.make_face {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.duplicate,
            "Make Face",
            matches!(col_selected_vertex_set(col).len(), 3 | 4)
                || col_selected_edge_set(col).len() == 2,
            false,
        );
    }
    if let Some(rect) = layout.flip_face {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.rotate,
            "Flip Face",
            false,
            false,
        );
    }
    if let Some(rect) = layout.merge_distance {
        editing_action_button(
            &app.ui_font,
            rect,
            &app.icons.vertex,
            "Merge by Distance",
            false,
            false,
        );
    }
    if let Some(rect) = layout.optimize {
        text_button(&app.ui_font, rect, "Optimize", false);
    }
    if let Some(rect) = layout.cleanup {
        text_button(&app.ui_font, rect, "Cleanup", false);
    }
    if let Some(rect) = layout.validate {
        text_button(&app.ui_font, rect, "Validate", false);
    }
    end_ui_clip();

    // Panel scrollbar.
    let max_scroll = col_panel_max_scroll(&layout);
    if max_scroll > 0.0 {
        let track_x = layout.content.x + layout.content.w - 5.0;
        let track_y = layout.content.y + 2.0;
        let track_h = layout.content.h - 4.0;
        draw_rrect(track_x, track_y, 3.0, track_h, 1.5, ui_border());
        let thumb_h = (track_h * layout.content.h / layout.content_height.max(1.0)).max(24.0);
        let frac = (col.panel_scroll / max_scroll).clamp(0.0, 1.0);
        draw_rrect(
            track_x,
            track_y + frac * (track_h - thumb_h),
            3.0,
            thumb_h,
            1.5,
            ui_accent(),
        );
    }

    // Pinned footer.
    draw_rrect(
        right.x + 1.0,
        layout.stage.y - 8.0,
        right.w - 2.0,
        1.0,
        0.5,
        ui_border(),
    );
    editing_action_button(
        &app.ui_font,
        layout.stage,
        &app.icons.save,
        if col.dirty {
            "Stage COL *"
        } else {
            "Stage COL"
        },
        col.dirty,
        false,
    );
    // Draw the material dropdown's expanded list last so it overlays the buttons above.
    draw_col_material_dropdown_popup(app);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_test_img_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "eagle_{label}_{}_{}.img",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn img_merge_scan_reads_entries_and_finds_case_insensitive_matches() {
        let path = unique_test_img_path("merge_scan");
        let mut col = Vec::from(&b"COLL"[..]);
        col.extend_from_slice(&8u32.to_le_bytes());
        col.extend_from_slice(&[7; 8]);
        write_img_archive(
            &path,
            &[
                ("MATCH.COL".to_string(), col.clone()),
                ("new.dat".to_string(), vec![3; 9]),
            ],
        )
        .unwrap();

        let plan =
            load_editing_img_merge_plan(path.clone(), BTreeSet::from(["match.col".to_string()]))
                .unwrap();

        assert_eq!(plan.matching_entries, 1);
        assert_eq!(plan.entries.len(), 2);
        assert!(plan.entries[0].matches_existing);
        assert_eq!(plan.entries[0].bytes, col);
        assert!(!plan.entries[1].matches_existing);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn img_merge_staging_replaces_matches_and_marks_only_new_rows_added() {
        let target = PathBuf::from("target.img");
        let mut editing = EditingState {
            img_path: Some(target.clone()),
            rows: vec![EditingImgRow {
                entry: ImgEntry {
                    img_path: target.clone(),
                    name: "keep.dff".to_string(),
                    offset: 2048,
                    size: 2048,
                },
                logical_size: 12,
            }],
            ..EditingState::default()
        };

        editing_stage_added_entry(&mut editing, target.clone(), "KEEP.DFF", vec![9; 17]);
        editing_stage_added_entry(&mut editing, target, "new.txd", vec![4; 23]);

        assert_eq!(editing.rows.len(), 2);
        assert_eq!(editing.modified_entries["keep.dff"], vec![9; 17]);
        assert!(!editing.added_entries.contains("keep.dff"));
        assert!(editing.added_entries.contains("new.txd"));
    }

    #[test]
    fn truncated_img_entry_read_returns_an_error_instead_of_slicing_past_input() {
        let row = EditingImgRow {
            entry: ImgEntry {
                img_path: PathBuf::from("truncated.img"),
                name: "broken.dff".to_string(),
                offset: 0,
                size: 16,
            },
            logical_size: 12,
        };

        let error = checked_editing_entry_bytes(&row, vec![0; 7]).unwrap_err();

        assert!(error.contains("expected 16 bytes, got 7"));
    }

    #[test]
    fn complete_img_entry_read_is_trimmed_to_its_logical_size() {
        let row = EditingImgRow {
            entry: ImgEntry {
                img_path: PathBuf::from("complete.img"),
                name: "model.dff".to_string(),
                offset: 0,
                size: 16,
            },
            logical_size: 9,
        };

        let bytes = checked_editing_entry_bytes(&row, (0u8..16).collect()).unwrap();

        assert_eq!(bytes, (0u8..9).collect::<Vec<_>>());
    }

    fn test_clockwise_triangle_raw() -> RawMesh {
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
    fn recalculated_normals_match_normalized_writer_clockwise_winding() {
        let mut raw = test_clockwise_triangle_raw();
        let expected = normalized_normals(&raw);

        recalc_raw_normals(&mut raw);

        assert_eq!(raw.normals.len(), expected.len());
        for (actual, expected) in raw.normals.iter().zip(expected) {
            assert!((actual.x - expected.x).abs() < 1.0e-6);
            assert!((actual.y - expected.y).abs() < 1.0e-6);
            assert!((actual.z - expected.z).abs() < 1.0e-6);
        }
    }

    #[test]
    fn geometry_recalculation_marks_breakable_data_stale() {
        let mut raw = test_clockwise_triangle_raw();
        raw.components.push(RawMeshComponent {
            name: "model".to_string(),
            vertex_start: 0,
            vertex_end: raw.vertices.len(),
            tri_start: 0,
            tri_end: raw.triangles.len(),
            breakable: Some(BreakableGeometry::default()),
        });

        recalc_raw_normals(&mut raw);

        assert!(
            raw.components[0]
                .breakable
                .as_ref()
                .is_some_and(|breakable| breakable.stale)
        );
    }

    #[test]
    fn normalized_stage_rejects_hierarchy_without_requiring_fractures() {
        let mut raw = test_clockwise_triangle_raw();
        raw.components = vec![
            RawMeshComponent {
                name: "body".to_string(),
                vertex_start: 0,
                vertex_end: raw.vertices.len(),
                tri_start: 0,
                tri_end: raw.triangles.len(),
                breakable: None,
            },
            RawMeshComponent {
                name: "dummy".to_string(),
                ..RawMeshComponent::default()
            },
        ];

        let error = validate_normalized_dff_stage(&raw, "model").unwrap_err();

        assert!(error.contains("multi-frame/component hierarchy"));
    }

    #[test]
    fn normalized_stage_accepts_a_single_static_geometry() {
        let raw = test_clockwise_triangle_raw();
        assert!(validate_normalized_dff_stage(&raw, "model").is_ok());
    }

    fn empty_test_collision_mesh() -> CollisionMesh {
        CollisionMesh {
            name: "test".to_string(),
            spheres: Vec::new(),
            boxes: Vec::new(),
            vertices: Vec::new(),
            faces: Vec::new(),
            bounds: Bounds {
                min: Vec3::ZERO,
                max: Vec3::ZERO,
            },
            shadow_vertices: Vec::new(),
            shadow_faces: Vec::new(),
        }
    }

    #[test]
    fn cuboid_audit_recovers_a_rotated_closed_six_plane_box() {
        let mut mesh = empty_test_collision_mesh();
        append_cuboid_artifacts(
            &mut mesh,
            V3 {
                x: 2.0,
                y: -3.0,
                z: 4.0,
            },
            V3 {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            V3 {
                x: 17.0,
                y: -21.0,
                z: 38.0,
            },
            CollisionSurface {
                material: 7,
                flags: 0,
                brightness: 0,
                light: 123,
            },
        )
        .unwrap();

        let result = audit_collision_cuboids(mesh, Vec::new(), Vec::new()).unwrap();

        assert_eq!(result.native_boxes, 0);
        assert_eq!(result.rotated_boxes, 1);
        assert_eq!(result.cuboids.len(), 1);
        assert_eq!(result.mesh.vertices.len(), COL_CUBOID_VERTEX_COUNT);
        assert_eq!(result.mesh.faces.len(), COL_CUBOID_FACE_COUNT);
        assert_eq!(result.cuboids[0].surface.material, 7);
        assert!((result.cuboids[0].center.x - 2.0).abs() < 0.001);
        assert!((result.cuboids[0].center.y + 3.0).abs() < 0.001);
        assert!((result.cuboids[0].center.z - 4.0).abs() < 0.001);
    }

    #[test]
    fn cuboid_audit_reinstates_axis_aligned_mesh_as_native_box() {
        let mut mesh = empty_test_collision_mesh();
        append_cuboid_artifacts(
            &mut mesh,
            V3 {
                x: 3.0,
                y: 4.0,
                z: 5.0,
            },
            V3 {
                x: 1.0,
                y: 2.0,
                z: 0.5,
            },
            V3::default(),
            CollisionSurface {
                material: 2,
                flags: 0,
                brightness: 0,
                light: 200,
            },
        )
        .unwrap();

        let result = audit_collision_cuboids(mesh, Vec::new(), Vec::new()).unwrap();

        assert_eq!(result.native_boxes, 1);
        assert_eq!(result.rotated_boxes, 0);
        assert!(result.cuboids.is_empty());
        assert!(result.mesh.vertices.is_empty());
        assert!(result.mesh.faces.is_empty());
        assert_eq!(result.mesh.boxes.len(), 1);
        assert_eq!(
            result.mesh.boxes[0].min,
            V3 {
                x: 2.0,
                y: 2.0,
                z: 4.5
            }
        );
        assert_eq!(
            result.mesh.boxes[0].max,
            V3 {
                x: 4.0,
                y: 6.0,
                z: 5.5
            }
        );
    }

    fn test_dff_material(seed: f32) -> RawMaterial {
        RawMaterial {
            color: V3 {
                x: seed,
                y: seed + 0.1,
                z: seed + 0.2,
            },
            alpha: seed + 0.3,
            ambient: seed + 1.0,
            diffuse: seed + 2.0,
            specular: seed + 3.0,
        }
    }

    #[test]
    fn create_material_keeps_slots_aligned_and_assigns_only_selected_faces() {
        let source = test_dff_material(0.1);
        let mut raw = RawMesh {
            triangles: vec![
                Tri {
                    a: 0,
                    b: 1,
                    c: 2,
                    material: 0,
                },
                Tri {
                    a: 2,
                    b: 3,
                    c: 0,
                    material: 0,
                },
                Tri {
                    a: 4,
                    b: 5,
                    c: 6,
                    material: 0,
                },
            ],
            material_textures: vec!["brick".to_string()],
            materials: vec![source],
            material_animations: vec![DffMaterialAnim {
                names: vec!["scroll".to_string()],
            }],
            ..RawMesh::default()
        };
        let selected = BTreeSet::from([0usize, 2]);
        let triangle_count = raw.triangles.len();
        let vertex_count = raw.vertices.len();

        let material = append_dff_material_slot(&mut raw, 0, Some("brick"), false).unwrap();
        let changed = assign_dff_faces_to_material(&mut raw, &selected, material);

        assert_eq!(material, 1);
        assert_eq!(changed, 2);
        assert_eq!(raw.triangles[0].material, 1);
        assert_eq!(raw.triangles[1].material, 0);
        assert_eq!(raw.triangles[2].material, 1);
        assert_eq!(raw.triangles.len(), triangle_count);
        assert_eq!(raw.vertices.len(), vertex_count);
        assert_eq!(raw.material_textures, ["brick", "brick"]);
        assert_eq!(raw.materials, [source, source]);
        assert!(raw.material_animations[1].names.is_empty());
        assert_eq!(raw.material_textures.len(), raw.materials.len());
        assert_eq!(raw.materials.len(), raw.material_animations.len());
    }

    #[test]
    fn assigning_existing_material_does_not_append_slots() {
        let mut raw = RawMesh {
            triangles: vec![
                Tri {
                    a: 0,
                    b: 1,
                    c: 2,
                    material: 0,
                },
                Tri {
                    a: 2,
                    b: 3,
                    c: 0,
                    material: 1,
                },
            ],
            material_textures: vec!["a".to_string(), "b".to_string()],
            materials: vec![test_dff_material(0.1), test_dff_material(0.2)],
            material_animations: vec![DffMaterialAnim::default(); 2],
            ..RawMesh::default()
        };
        let lengths = (
            raw.material_textures.len(),
            raw.materials.len(),
            raw.material_animations.len(),
        );

        let changed = assign_dff_faces_to_material(&mut raw, &BTreeSet::from([0, 1]), 1);

        assert_eq!(changed, 1);
        assert!(raw.triangles.iter().all(|triangle| triangle.material == 1));
        assert_eq!(
            lengths,
            (
                raw.material_textures.len(),
                raw.materials.len(),
                raw.material_animations.len()
            )
        );
    }

    #[test]
    fn appending_material_repairs_sparse_parallel_arrays() {
        let mut raw = RawMesh {
            triangles: vec![Tri {
                a: 0,
                b: 1,
                c: 2,
                material: 1,
            }],
            material_textures: vec!["a".to_string(), "b".to_string()],
            materials: vec![test_dff_material(0.1)],
            ..RawMesh::default()
        };

        let material = append_dff_material_slot(&mut raw, 1, Some("c"), false).unwrap();

        assert_eq!(material, 2);
        assert_eq!(raw.material_textures.len(), 3);
        assert_eq!(raw.materials.len(), 3);
        assert_eq!(raw.material_animations.len(), 3);
        assert_eq!(raw.materials[1], default_dff_material());
        assert_eq!(raw.materials[2], default_dff_material());
    }

    #[test]
    fn duplicate_material_slot_clones_surface_and_animation() {
        let source = test_dff_material(0.25);
        let animation = DffMaterialAnim {
            names: vec!["water_scroll".to_string()],
        };
        let mut raw = RawMesh {
            material_textures: vec!["water".to_string()],
            materials: vec![source],
            material_animations: vec![animation.clone()],
            ..RawMesh::default()
        };

        let material = append_dff_material_slot(&mut raw, 0, None, true).unwrap();

        assert_eq!(material, 1);
        assert_eq!(raw.material_textures[1], "water");
        assert_eq!(raw.materials[1], source);
        assert_eq!(raw.material_animations[1], animation);
    }

    #[test]
    fn set_material_texture_changes_only_the_target_slot() {
        let materials = vec![test_dff_material(0.1), test_dff_material(0.2)];
        let animations = vec![
            DffMaterialAnim {
                names: vec!["first".to_string()],
            },
            DffMaterialAnim {
                names: vec!["second".to_string()],
            },
        ];
        let triangles = vec![Tri {
            a: 0,
            b: 1,
            c: 2,
            material: 1,
        }];
        let mut raw = RawMesh {
            triangles: triangles.clone(),
            material_textures: vec!["brick".to_string(), "glass".to_string()],
            materials: materials.clone(),
            material_animations: animations.clone(),
            ..RawMesh::default()
        };

        assert_eq!(set_dff_material_texture(&mut raw, 1, "metal"), Ok(true));
        assert_eq!(raw.material_textures, ["brick", "metal"]);
        assert_eq!(raw.materials, materials);
        assert_eq!(raw.material_animations, animations);
        assert_eq!(raw.triangles, triangles);
        assert_eq!(set_dff_material_texture(&mut raw, 1, "METAL"), Ok(false));
        assert!(set_dff_material_texture(&mut raw, 2, "missing").is_err());
        assert!(set_dff_material_texture(&mut raw, 0, " ").is_err());
    }

    #[test]
    fn compact_unused_materials_stably_remaps_parallel_slots_and_faces() {
        let materials = (0..4)
            .map(|index| test_dff_material(index as f32 * 0.1))
            .collect::<Vec<_>>();
        let animations = (0..4)
            .map(|index| DffMaterialAnim {
                names: vec![format!("anim{index}")],
            })
            .collect::<Vec<_>>();
        let mut raw = RawMesh {
            triangles: vec![
                Tri {
                    a: 0,
                    b: 1,
                    c: 2,
                    material: 2,
                },
                Tri {
                    a: 2,
                    b: 3,
                    c: 0,
                    material: 0,
                },
                Tri {
                    a: 4,
                    b: 5,
                    c: 6,
                    material: 3,
                },
            ],
            material_textures: ["a", "unused", "c", "d"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            materials: materials.clone(),
            material_animations: animations.clone(),
            ..RawMesh::default()
        };
        let triangle_count = raw.triangles.len();

        let remap = compact_unused_dff_material_slots(&mut raw);

        assert_eq!(
            remap,
            DffMaterialRemap {
                old_to_new: vec![Some(0), None, Some(1), Some(2)],
                removed: 1,
            }
        );
        assert_eq!(raw.material_textures, ["a", "c", "d"]);
        assert_eq!(raw.materials, [materials[0], materials[2], materials[3]]);
        assert_eq!(
            raw.material_animations,
            [
                animations[0].clone(),
                animations[2].clone(),
                animations[3].clone()
            ]
        );
        assert_eq!(
            raw.triangles
                .iter()
                .map(|triangle| triangle.material)
                .collect::<Vec<_>>(),
            [1, 0, 2]
        );
        assert_eq!(raw.triangles.len(), triangle_count);
    }

    #[test]
    fn material_sidecar_remap_preserves_other_scopes_and_dffs() {
        let mut values = HashMap::from([
            (material_emitter_key("target.dff", 0), 10),
            (material_emitter_key("target.dff", 1), 11),
            (material_emitter_key("target.dff", 2), 12),
            (material_emitter_key("other.dff", 2), 20),
            (material_emitter_face_key("target.dff", 4), 30),
            ("face-group|target.dff|1,2".to_string(), 31),
            ("texture|brick".to_string(), 32),
        ]);

        remap_dff_material_sidecar_keys(&mut values, "target.dff", &[Some(0), None, Some(1)]);

        assert_eq!(
            values.get(&material_emitter_key("target.dff", 0)),
            Some(&10)
        );
        assert_eq!(
            values.get(&material_emitter_key("target.dff", 1)),
            Some(&12)
        );
        assert!(!values.contains_key(&material_emitter_key("target.dff", 2)));
        assert_eq!(values.get(&material_emitter_key("other.dff", 2)), Some(&20));
        assert_eq!(
            values.get(&material_emitter_face_key("target.dff", 4)),
            Some(&30)
        );
        assert_eq!(values.get("face-group|target.dff|1,2"), Some(&31));
        assert_eq!(values.get("texture|brick"), Some(&32));
    }

    #[test]
    fn material_presets_change_only_requested_rgba_components() {
        let mut material = test_dff_material(0.2);
        let original_alpha = material.alpha;
        let original_surface = (material.ambient, material.diffuse, material.specular);

        assert!(apply_dff_material_preset(
            &mut material,
            Some(DFF_MATERIAL_COLOR_PRESETS[3]),
            None
        ));
        assert_eq!(material.color, DFF_MATERIAL_COLOR_PRESETS[3]);
        assert_eq!(material.alpha, original_alpha);
        assert!(apply_dff_material_preset(&mut material, None, Some(-1.0)));
        assert_eq!(material.color, DFF_MATERIAL_COLOR_PRESETS[3]);
        assert_eq!(material.alpha, 0.0);
        assert_eq!(
            (material.ambient, material.diffuse, material.specular),
            original_surface
        );
    }

    #[test]
    fn linked_face_component_selects_only_the_seed_island() {
        let faces = [
            [[0, 0, 0], [1000, 0, 0], [0, 1000, 0]],
            [[1000, 0, 0], [1000, 1000, 0], [0, 1000, 0]],
            [[5000, 0, 0], [6000, 0, 0], [5000, 1000, 0]],
        ];

        assert_eq!(linked_face_component(&faces, 0).unwrap(), vec![0, 1]);
        assert_eq!(linked_face_component(&faces, 2).unwrap(), vec![2]);
    }

    #[test]
    fn linked_face_component_crosses_coincident_vertex_seams() {
        // DFF geometry can duplicate vertex indices at UV or material seams.
        // The linked-selection snapshot contains positions, so coincident seam
        // vertices still join the same physical island.
        let first = [
            linked_selection_position_key(V3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            })
            .unwrap(),
            linked_selection_position_key(V3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            })
            .unwrap(),
            linked_selection_position_key(V3 {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            })
            .unwrap(),
        ];
        let across_seam = [
            linked_selection_position_key(V3 {
                x: 1.0004,
                y: 0.0,
                z: 0.0,
            })
            .unwrap(),
            linked_selection_position_key(V3 {
                x: 2.0,
                y: 0.0,
                z: 0.0,
            })
            .unwrap(),
            linked_selection_position_key(V3 {
                x: 1.0,
                y: 1.0,
                z: 0.0,
            })
            .unwrap(),
        ];

        assert_eq!(
            linked_face_component(&[first, across_seam], 0).unwrap(),
            vec![0, 1]
        );
    }

    #[test]
    fn linked_face_component_rejects_a_stale_seed() {
        let faces = [[[0, 0, 0], [1000, 0, 0], [0, 1000, 0]]];

        assert!(linked_face_component(&faces, 1).is_err());
    }

    #[test]
    fn img_save_aborts_when_an_unchanged_source_entry_cannot_be_read() {
        let missing = std::env::temp_dir().join(format!(
            "eagle_missing_img_entry_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let rows = vec![EditingImgRow {
            entry: ImgEntry {
                img_path: missing,
                name: "keep.dff".to_string(),
                offset: 2048,
                size: 2048,
            },
            logical_size: 128,
        }];

        let result = editing_img_save_entries(rows, &BTreeMap::new(), &BTreeSet::new());

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("Could not read source IMG entry keep.dff completely")
        );
    }

    fn lod_test_placement(id: &str, dff: &str, lod_parent: Option<&str>) -> Placement {
        let mut attrs = BTreeMap::new();
        if let Some(lod_parent) = lod_parent {
            attrs.insert("lodParent".to_string(), lod_parent.to_string());
        }
        Placement {
            id: id.to_string(),
            dff: dff.to_string(),
            zone: "test".to_string(),
            tag: "object".to_string(),
            attrs,
            pos: V3::default(),
            rot: V3::default(),
        }
    }

    #[test]
    fn collision_generation_recognizes_assigned_lod_dff() {
        let placements = vec![
            lod_test_placement("detail_house", "house", Some("lod_house")),
            lod_test_placement("lod_house", "house_lod", None),
        ];
        let lod_ids = collect_lod_ids(&placements);
        let states = vec![ElementState::default(); placements.len()];

        assert!(collision_generation_dff_is_lod_only_in_scene(
            "house_lod.dff",
            &placements,
            &HashMap::new(),
            &lod_ids,
            &states,
        ));
        assert!(!collision_generation_dff_is_lod_only_in_scene(
            "house.dff",
            &placements,
            &HashMap::new(),
            &lod_ids,
            &states,
        ));
    }

    #[test]
    fn shared_detail_and_lod_dff_keeps_full_collision() {
        let placements = vec![
            lod_test_placement("detail_house", "shared_house", Some("lod_house")),
            lod_test_placement("lod_house", "shared_house", None),
        ];
        let lod_ids = collect_lod_ids(&placements);
        let states = vec![ElementState::default(); placements.len()];

        assert!(!collision_generation_dff_is_lod_only_in_scene(
            "shared_house.dff",
            &placements,
            &HashMap::new(),
            &lod_ids,
            &states,
        ));
    }

    #[test]
    fn generated_col_assignment_matches_dff_and_marks_definition_override() {
        let mut definition = Definition {
            id: "physics_prop".to_string(),
            zone: "test".to_string(),
            attrs: BTreeMap::from([
                ("dff".to_string(), "physics_visual.dff".to_string()),
                ("__override".to_string(), "true".to_string()),
                ("__overrideAttrs".to_string(), "dff".to_string()),
            ]),
        };

        assert!(definition_uses_dff(&definition, "PHYSICS_VISUAL"));
        assert!(assign_generated_col_to_definition(
            &mut definition,
            "physics_crate"
        ));
        assert_eq!(
            definition.attrs.get("col").map(String::as_str),
            Some("physics_crate")
        );
        assert_eq!(
            definition.attrs.get("__overrideAttrs").map(String::as_str),
            Some("col,dff")
        );
        assert!(!assign_generated_col_to_definition(
            &mut definition,
            "physics_crate"
        ));

        let mut inherited_override = definition.clone();
        inherited_override
            .attrs
            .insert("__overrideAttrs".to_string(), "dff".to_string());
        assert!(assign_generated_col_to_definition(
            &mut inherited_override,
            "physics_crate"
        ));
        assert_eq!(
            inherited_override
                .attrs
                .get("__overrideAttrs")
                .map(String::as_str),
            Some("col,dff")
        );
    }

    #[test]
    fn matching_dff_and_col_names_use_implicit_definition_collision() {
        let mut definition = Definition {
            id: "fence_definition".to_string(),
            zone: "test".to_string(),
            attrs: BTreeMap::from([
                ("dff".to_string(), "d3b0890001_fence_A.dff".to_string()),
                ("col".to_string(), "d3b0890001_fence_A".to_string()),
                ("__override".to_string(), "true".to_string()),
                ("__overrideAttrs".to_string(), "col,dff".to_string()),
            ]),
        };

        assert!(generated_col_matches_definition_dff(
            &definition,
            "D3B0890001_FENCE_A.col"
        ));
        assert!(clear_redundant_definition_col(&mut definition));
        assert!(!definition.attrs.contains_key("col"));
        assert_eq!(
            definition.attrs.get("__overrideAttrs").map(String::as_str),
            Some("dff")
        );
        assert!(!clear_redundant_definition_col(&mut definition));
    }

    #[test]
    fn generated_col_name_replaces_dff_extension() {
        assert_eq!(
            collision_name_from_dff("d3b0890001_fence_b.dff"),
            "d3b0890001_fence_b.col"
        );
        assert_eq!(collision_name_from_dff("CRATE.DFF"), "CRATE.col");
        assert_eq!(collision_name_from_dff("barrel"), "barrel.col");
        assert_eq!(
            collision_name_from_reference("d3b0890001_fence_b.dff"),
            "d3b0890001_fence_b.col"
        );
        assert_eq!(
            collision_name_from_reference("d3b0890001_fence_b.col"),
            "d3b0890001_fence_b.col"
        );
        assert_eq!(
            collision_name_from_reference("d3b0890001_fence_b.dff.col"),
            "d3b0890001_fence_b.col"
        );
    }

    #[test]
    fn generated_col_is_added_to_editor_archive_index() {
        let mut editing = EditingState::default();
        let bytes = vec![1, 2, 3, 4];

        editing_stage_added_entry(
            &mut editing,
            PathBuf::from("imgs/light_mapper_replacements.img"),
            "d3b0890001_fence_A.col",
            bytes.clone(),
        );

        assert_eq!(editing.rows.len(), 1);
        assert_eq!(editing.rows[0].entry.name, "d3b0890001_fence_A.col");
        assert_eq!(
            editing.modified_entries.get("d3b0890001_fence_a.col"),
            Some(&bytes)
        );
        assert!(editing.added_entries.contains("d3b0890001_fence_a.col"));
        assert!(!editing.deleted_entries.contains("d3b0890001_fence_a.col"));
    }

    #[test]
    fn global_regeneration_clears_old_target_when_dff_is_missing() {
        let context = CollisionGenerationContext {
            root: std::env::temp_dir().join(format!(
                "eagle_missing_collision_source_{}",
                std::process::id()
            )),
            sources: vec![CollisionGenerationSource {
                dff_name: "missing_model.dff".to_string(),
                col_name: "referenced_target.col".to_string(),
                txd_name: None,
                raw_override: None,
                lod_only: false,
                definition_ids_to_assign: Vec::new(),
            }],
            byte_overrides: BTreeMap::new(),
            material_classes: TextureMaterialClasses::default(),
            txd_textures: TxdTextureIndex::new(),
            preset: CollisionGenerationPreset::Auto,
            fallback_material: 0,
        };
        let (progress, _) = mpsc::channel();
        let result = run_collision_generation(context, &progress);

        assert_eq!(result.generated.len(), 1);
        assert_eq!(result.errors.len(), 1);
        let generated = &result.generated[0];
        assert!(generated.mesh.faces.is_empty());
        assert!(generated.mesh.boxes.is_empty());
        assert!(generated.mesh.spheres.is_empty());
        assert_eq!(
            collect_col_model_names(&generated.bytes),
            vec!["referenced_target.col".to_string()]
        );
    }

    #[test]
    fn empty_generated_col_serializes_dff_bounds() {
        let raw = RawMesh {
            vertices: vec![
                V3 {
                    x: -4.0,
                    y: -5.0,
                    z: -6.0,
                },
                V3 {
                    x: 7.0,
                    y: 8.0,
                    z: 9.0,
                },
            ],
            ..RawMesh::default()
        };
        let context = CollisionGenerationContext {
            root: PathBuf::new(),
            sources: vec![CollisionGenerationSource {
                dff_name: "vegetation.dff".to_string(),
                col_name: "vegetation.col".to_string(),
                txd_name: None,
                raw_override: Some(raw),
                lod_only: false,
                definition_ids_to_assign: Vec::new(),
            }],
            byte_overrides: BTreeMap::new(),
            material_classes: TextureMaterialClasses::default(),
            txd_textures: TxdTextureIndex::new(),
            preset: CollisionGenerationPreset::Auto,
            fallback_material: 0,
        };
        let (progress, _) = mpsc::channel();

        let result = run_collision_generation(context, &progress);

        assert_eq!(result.generated.len(), 1);
        let generated = &result.generated[0];
        assert!(generated.mesh.faces.is_empty());
        assert!(generated.mesh.boxes.is_empty());
        assert!(generated.mesh.spheres.is_empty());
        assert_eq!(rdf32(&generated.bytes, 32), -4.0);
        assert_eq!(rdf32(&generated.bytes, 36), -5.0);
        assert_eq!(rdf32(&generated.bytes, 40), -6.0);
        assert_eq!(rdf32(&generated.bytes, 44), 7.0);
        assert_eq!(rdf32(&generated.bytes, 48), 8.0);
        assert_eq!(rdf32(&generated.bytes, 52), 9.0);
    }

    #[test]
    fn safe_collision_targets_are_excluded_from_regeneration() {
        let source = |name: &str| CollisionGenerationSource {
            dff_name: format!("{name}.dff"),
            col_name: format!("{name}.col"),
            txd_name: None,
            raw_override: None,
            lod_only: false,
            definition_ids_to_assign: Vec::new(),
        };
        let mut sources = BTreeMap::from([
            ("protected.col".to_string(), source("protected")),
            ("replace_me.col".to_string(), source("replace_me")),
        ]);
        let mut safe = SafeCollisions::default();
        safe.set_safe("PROTECTED", true);

        assert_eq!(remove_safe_collision_sources(&mut sources, &safe), 1);
        assert_eq!(
            sources.keys().cloned().collect::<Vec<_>>(),
            vec!["replace_me.col".to_string()]
        );
    }

    #[test]
    fn collision_material_picker_filters_by_name_id_and_no_collision() {
        let grass = editing_txd_material_filtered("grass");
        assert!(!grass.is_empty());
        assert!(
            grass
                .iter()
                .all(|(_, name)| name.to_ascii_lowercase().contains("grass"))
        );

        let material_id = GTA_SA_COL_MATERIALS[7].0;
        let by_id = editing_txd_material_filtered(&material_id.to_string());
        assert!(
            by_id
                .iter()
                .any(|(choice, _)| *choice == CollisionMaterialPickerValue::Material(material_id))
        );

        assert_eq!(
            editing_txd_material_filtered("exclude"),
            vec![(CollisionMaterialPickerValue::NoCollision, "No Collision")]
        );
    }

    fn indexed_material_texture(
        txd_name: &str,
        fingerprint: TextureContentFingerprint,
    ) -> TxdTexture {
        TxdTexture {
            txd_name: txd_name.to_string(),
            img_path: PathBuf::new(),
            native_offset: 0,
            native_size: 0,
            data_offset: 0,
            data_size: 0,
            palette_offset: 0,
            width: 64,
            height: 64,
            format: TxFormat::Dxt1,
            has_alpha: false,
            content_fingerprint: fingerprint,
        }
    }

    #[test]
    fn global_material_scope_clears_older_content_assignments_for_texture() {
        let first = [0x1111, 0xaaaa];
        let second = [0x2222, 0xbbbb];
        let unrelated = [0x3333, 0xcccc];
        let mut index = TxdTextureIndex::new();
        index.insert(
            "road".to_string(),
            vec![
                indexed_material_texture("city.txd", first),
                indexed_material_texture("country.txd", second),
            ],
        );
        let mut classes = TextureMaterialClasses::default();
        classes.set_content(first, Some(4));
        classes.set_content_no_collision(second, true);
        classes.set_content(unrelated, Some(7));
        classes.set_txd("city", "road", Some(2));
        classes.set_txd_no_collision("country", "road", true);

        assert!(clear_stale_texture_collision_material_scopes(
            &mut classes,
            &index,
            Some("city"),
            "Road",
            Some(second),
            CollisionMaterialAssignmentScope::GlobalName,
        ));
        assert_eq!(classes.content_material(first), None);
        assert!(!classes.content_is_no_collision(second));
        assert_eq!(classes.content_material(unrelated), Some(7));
        assert_eq!(classes.txd_material("city", "road"), None);
        assert!(!classes.txd_is_no_collision("country", "road"));

        classes.set_global("road", Some(9));
        assert_eq!(
            classes
                .resolve_with_content(Some("city"), "road", Some(first), 0)
                .material,
            9
        );
        assert_eq!(
            classes
                .resolve_with_content(Some("country"), "road", Some(second), 0)
                .material,
            9
        );
    }

    #[test]
    fn identical_material_scope_clears_older_global_assignment_for_texture() {
        let fingerprint = [0x1111, 0xaaaa];
        let mut index = TxdTextureIndex::new();
        index.insert(
            "road".to_string(),
            vec![indexed_material_texture("city.txd", fingerprint)],
        );
        index.insert(
            "road_alias".to_string(),
            vec![indexed_material_texture("country.txd", fingerprint)],
        );
        let mut classes = TextureMaterialClasses::default();
        classes.set_global_no_collision("road", true);
        classes.set_txd("city", "road", Some(2));
        classes.set_txd_no_collision("country", "road_alias", true);

        assert!(clear_stale_texture_collision_material_scopes(
            &mut classes,
            &index,
            Some("city"),
            "ROAD",
            Some(fingerprint),
            CollisionMaterialAssignmentScope::IdenticalContent,
        ));
        assert!(!classes.global_is_no_collision("road"));
        assert_eq!(classes.txd_material("city", "road"), None);
        assert!(!classes.txd_is_no_collision("country", "road_alias"));

        classes.set_content(fingerprint, Some(4));
        assert_eq!(
            classes
                .resolve_with_content(Some("city"), "road", Some(fingerprint), 0)
                .material,
            4
        );
    }

    fn test_face(a: u16, b: u16, c: u16, material: u8, light: u8) -> CollisionFace {
        CollisionFace {
            a,
            b,
            c,
            material,
            light,
            img_path: PathBuf::from("test.col"),
            material_file_offset: 0,
            light_file_offset: 0,
        }
    }

    fn stacked_wall_mesh(materials: &[u8]) -> CollisionMesh {
        let mut vertices = Vec::new();
        for z in 0..=materials.len() {
            vertices.push(V3 {
                x: 0.0,
                y: 0.0,
                z: z as f32,
            });
            vertices.push(V3 {
                x: 10.0,
                y: 0.0,
                z: z as f32,
            });
        }
        let mut faces = Vec::new();
        for (band, material) in materials.iter().copied().enumerate() {
            let lb = (band * 2) as u16;
            let rb = lb + 1;
            let lt = lb + 2;
            let rt = lb + 3;
            faces.push(test_face(lb, rb, rt, material, 130));
            faces.push(test_face(lb, rt, lt, material, 130));
        }
        let bounds = bounds_from_vertices(&vertices);
        CollisionMesh {
            name: "wall".to_string(),
            spheres: Vec::new(),
            boxes: Vec::new(),
            vertices,
            faces,
            bounds,
            shadow_vertices: Vec::new(),
            shadow_faces: Vec::new(),
        }
    }

    #[test]
    fn generated_corona_payload_matches_source_color_and_time() {
        let effect = dff_2dfx_generated_corona(PointEmitterCoronaSource {
            position: V3 {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            color: V3 {
                x: 0.25,
                y: 0.5,
                z: 1.0,
            },
            day: true,
            night: false,
            range: 512.0,
            height_above_model_base: 3.0,
        });

        assert_eq!(&effect.payload[..4], &[201, 219, 255, 255]);
        assert_eq!(&effect.payload[4..8], &100.0f32.to_le_bytes());
        assert_eq!(effect.payload[DFF_2DFX_LIGHT_FLAGS_1_OFFSET], 0x02 | 0x20);
        let expected_range = 3.0 * DFF_GENERATED_POINTLIGHT_HEIGHT_MULTIPLIER
            + DFF_GENERATED_POINTLIGHT_RANGE_ADDITIVE;
        assert_eq!(&effect.payload[8..12], &expected_range.to_le_bytes());
        assert_eq!(
            &effect.payload[12..16],
            &DFF_GENERATED_CORONA_SIZE.to_le_bytes()
        );
        assert_eq!(
            &effect.payload[16..20],
            &DFF_GENERATED_SHADOW_SIZE.to_le_bytes()
        );
        assert_eq!(effect.payload[23], 80);
        assert_eq!(
            &effect.payload
                [DFF_2DFX_LIGHT_SHADOW_TEXTURE_OFFSET..DFF_2DFX_LIGHT_SHADOW_TEXTURE_OFFSET + 8],
            b"shad_exp"
        );
        assert_eq!(effect.payload[DFF_2DFX_LIGHT_FLAGS_2_OFFSET], 0x04);
        assert_eq!(
            &effect.payload[DFF_2DFX_LIGHT_SHADOW_TEXTURE_TAIL_MARKER_OFFSET
                ..DFF_2DFX_LIGHT_SHADOW_TEXTURE_TAIL_MARKER_OFFSET
                    + DFF_GENERATED_CORONA_TEXTURE_TAIL_MARKER.len()],
            DFF_GENERATED_CORONA_TEXTURE_TAIL_MARKER
        );
        assert_eq!(
            &effect.payload
                [DFF_2DFX_LIGHT_LOOK_DIRECTION_OFFSET..DFF_2DFX_LIGHT_LOOK_DIRECTION_OFFSET + 3],
            &[0, 0, 100]
        );
        assert!(dff_2dfx_is_generated_corona(&effect));
    }

    #[test]
    fn corona_presets_write_complete_stock_light_payloads() {
        for preset in DFF_CORONA_PRESETS {
            let payload = dff_2dfx_corona_preset_payload(preset);
            assert_eq!(payload.len(), 80, "{}", preset.label);
            assert_eq!(&payload[0..4], &preset.color, "{}", preset.label);
            assert_eq!(payload[20], preset.show_mode, "{}", preset.label);
            assert_eq!(payload[21], preset.reflection, "{}", preset.label);
            assert_eq!(payload[22], preset.flare_type, "{}", preset.label);
            assert_eq!(payload[23], preset.shadow_multiplier, "{}", preset.label);
            assert_eq!(payload[24], preset.flags1, "{}", preset.label);
            assert_ne!(
                payload[24] & 0x60,
                0,
                "{} must be enabled during day, night, or both",
                preset.label
            );
            assert_eq!(fixed_string(&payload, 25, 24), "coronastar");
            assert_eq!(fixed_string(&payload, 49, 24), "shad_exp");
            assert_eq!(payload[73], preset.shadow_z_distance);
            assert_eq!(payload[74], preset.flags2);
            assert_eq!(&payload[75..78], &preset.look_direction);
            assert_eq!(
                dff_2dfx_corona_preset(&Dff2dEffect {
                    position: V3::default(),
                    effect_id: 0,
                    payload,
                }),
                Some(preset)
            );
        }
    }

    #[test]
    fn traffic_corona_presets_have_all_three_native_states() {
        let traffic = &DFF_CORONA_PRESETS[..3];
        assert!(traffic.iter().all(|preset| preset.show_mode == 7));
        assert_eq!(traffic[0].color[..3], [255, 0, 0]);
        assert!(traffic[1].color[0] > 200 && traffic[1].color[1] > 100);
        assert!(traffic[2].color[0] <= 200);
        assert!(traffic.iter().all(|preset| preset.pointlight_range == 18.0));
    }

    #[test]
    fn traffic_coronas_are_labeled_by_head_and_state_after_payload_edits() {
        let positions = [
            V3 {
                x: 0.00603,
                y: 0.30,
                z: 3.38062,
            },
            V3 {
                x: 0.00603,
                y: 0.30,
                z: 3.09296,
            },
            V3 {
                x: 0.00603,
                y: 0.30,
                z: 2.79663,
            },
            V3 {
                x: 0.14476,
                y: 0.04,
                z: 1.39783,
            },
            V3 {
                x: 0.14476,
                y: 0.04,
                z: 1.19415,
            },
            V3 {
                x: 0.14476,
                y: 0.04,
                z: 0.99310,
            },
        ];
        let preset_indices = [0usize, 1, 2, 0, 1, 2];
        let mut effects = positions
            .into_iter()
            .zip(preset_indices)
            .map(|(position, preset_index)| Dff2dEffect {
                position,
                effect_id: 0,
                payload: dff_2dfx_corona_preset_payload(DFF_CORONA_PRESETS[preset_index]),
            })
            .collect::<Vec<_>>();

        // Changing a size/flag makes this no longer byte-identical to a preset,
        // but the semantic label must remain intact.
        effects[4].payload[12..16].copy_from_slice(&0.25f32.to_le_bytes());
        effects[4].payload[24] = 0x60;

        assert_eq!(
            dff_2dfx_effect_display_name(&effects, 0),
            "Upper Traffic Light — Red"
        );
        assert_eq!(
            dff_2dfx_effect_display_name(&effects, 1),
            "Upper Traffic Light — Amber"
        );
        assert_eq!(
            dff_2dfx_effect_display_name(&effects, 2),
            "Upper Traffic Light — Green"
        );
        assert_eq!(
            dff_2dfx_effect_display_name(&effects, 3),
            "Lower Traffic Light — Red"
        );
        assert_eq!(
            dff_2dfx_effect_display_name(&effects, 4),
            "Lower Traffic Light — Amber"
        );
        assert_eq!(
            dff_2dfx_effect_display_name(&effects, 5),
            "Lower Traffic Light — Green"
        );
    }

    #[test]
    fn edited_common_coronas_keep_semantic_labels() {
        let mut warm = dff_2dfx_corona_preset_payload(DFF_CORONA_PRESETS[3]);
        warm[12..16].copy_from_slice(&0.75f32.to_le_bytes());
        let effects = vec![Dff2dEffect {
            position: V3::default(),
            effect_id: 0,
            payload: warm,
        }];

        assert_eq!(
            dff_2dfx_effect_display_name(&effects, 0),
            "Street / Path Light — Warm"
        );
    }

    #[test]
    fn specialized_corona_presets_apply_stock_show_modes() {
        assert_eq!(DFF_CORONA_PRESETS[6].show_mode, 6);
        assert_eq!(DFF_CORONA_PRESETS[7].show_mode, 8);
        assert_eq!(DFF_CORONA_PRESETS[8].show_mode, 3);
        assert_eq!(DFF_CORONA_PRESETS[9].show_mode, 1);
    }

    #[test]
    fn generated_corona_pointlight_reaches_nice_street_level() {
        let effect = dff_2dfx_generated_corona(PointEmitterCoronaSource {
            position: V3 {
                x: 0.0,
                y: 0.0,
                z: 9.705,
            },
            color: neutral_vertex_color(),
            day: false,
            night: true,
            range: 64.0,
            height_above_model_base: 9.705,
        });

        let expected = 9.705 * DFF_GENERATED_POINTLIGHT_HEIGHT_MULTIPLIER
            + DFF_GENERATED_POINTLIGHT_RANGE_ADDITIVE;
        assert_eq!(&effect.payload[8..12], &expected.to_le_bytes());
    }

    #[test]
    fn only_marker_tagged_lights_are_generated_coronas() {
        let manual = Dff2dEffect {
            effect_id: 0,
            payload: dff_2dfx_default_light_payload(),
            ..Dff2dEffect::default()
        };
        assert!(!dff_2dfx_is_generated_corona(&manual));

        let generated = dff_2dfx_generated_corona(PointEmitterCoronaSource {
            position: V3::default(),
            color: neutral_vertex_color(),
            day: true,
            night: true,
            range: 16.0,
            height_above_model_base: 0.0,
        });
        assert_eq!(
            generated.payload[DFF_2DFX_LIGHT_FLAGS_1_OFFSET],
            0x02 | 0x20 | 0x40
        );
        assert!(dff_2dfx_is_generated_corona(&generated));

        let mut flags_marker_legacy = Dff2dEffect {
            effect_id: 0,
            payload: dff_2dfx_default_light_payload(),
            ..Dff2dEffect::default()
        };
        flags_marker_legacy.payload[DFF_2DFX_LIGHT_FLAGS_2_OFFSET] =
            0x04 | DFF_GENERATED_CORONA_FLAGS2_MARKER;
        assert!(dff_2dfx_is_generated_corona(&flags_marker_legacy));

        let mut legacy = Dff2dEffect {
            effect_id: 0,
            payload: dff_2dfx_default_light_payload(),
            ..Dff2dEffect::default()
        };
        legacy.payload[DFF_2DFX_LIGHT_SHADOW_TEXTURE_OFFSET
            ..DFF_2DFX_LIGHT_SHADOW_TEXTURE_OFFSET + DFF_GENERATED_CORONA_MARKER.len()]
            .copy_from_slice(DFF_GENERATED_CORONA_MARKER);
        assert!(dff_2dfx_is_generated_corona(&legacy));
    }

    #[test]
    fn regeneration_replaces_only_tagged_coronas() {
        let manual = Dff2dEffect {
            effect_id: 0,
            payload: dff_2dfx_default_light_payload(),
            ..Dff2dEffect::default()
        };
        let old_generated = dff_2dfx_generated_corona(PointEmitterCoronaSource {
            position: V3::default(),
            color: neutral_vertex_color(),
            day: false,
            night: true,
            range: 12.0,
            height_above_model_base: 0.0,
        });
        let new_generated = dff_2dfx_generated_corona(PointEmitterCoronaSource {
            position: V3 {
                x: 4.0,
                y: 5.0,
                z: 6.0,
            },
            color: V3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            day: true,
            night: false,
            range: 32.0,
            height_above_model_base: 6.0,
        });
        let mut raw = RawMesh {
            effects_2dfx: vec![manual.clone(), old_generated],
            ..RawMesh::default()
        };

        assert_eq!(
            replace_generated_coronas(&mut raw, vec![new_generated.clone()]),
            (1, 1)
        );
        assert!(raw.effects_2dfx == vec![manual, new_generated]);
    }

    #[test]
    fn global_corona_worker_processes_raw_dff_override() {
        let root = std::env::temp_dir().join(format!(
            "eagle_corona_worker_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let raw = RawMesh {
            vertices: vec![
                V3::default(),
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
            triangles: vec![Tri {
                a: 0,
                b: 1,
                c: 2,
                material: 0,
            }],
            material_textures: vec!["lamp".to_string()],
            materials: vec![RawMaterial {
                color: V3 {
                    x: 0.2,
                    y: 0.4,
                    z: 0.8,
                },
                ..RawMaterial::default()
            }],
            ..RawMesh::default()
        };
        let emitter = MaterialEmitter {
            enabled: true,
            cast_mode: MaterialEmitterCastMode::Point,
            falloff_distance: 36.0,
            day: false,
            night: true,
            use_material_color: false,
            ..MaterialEmitter::default()
        };
        let day_color = V3 {
            x: 0.2,
            y: 0.3,
            z: 0.4,
        };
        let night_color = V3 {
            x: 0.1,
            y: 0.15,
            z: 0.2,
        };
        let runtime_mesh = RenderMesh {
            parts: vec![RenderPart {
                list: 0,
                vbo: 0,
                vertices: 3,
                material_index: 0,
                component: 0,
                texture: 0,
                texture_width: 0,
                texture_height: 0,
                texture_name: "lamp".to_string(),
                texture_fingerprint: None,
                texture_missing: false,
                transparency: TransparencyMode::Opaque,
                alpha: 1.0,
                material_color: neutral_vertex_color(),
                material_ambient: 1.0,
                use_lighting: false,
                vehicle_material_role: None,
                emissive: false,
                cpu_vertices: [
                    V3::default(),
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
                ]
                .into_iter()
                .map(|pos| Vertex {
                    pos,
                    normal: V3 {
                        x: 0.0,
                        y: 0.0,
                        z: 1.0,
                    },
                    uv: V2::default(),
                    color: day_color,
                    day_color,
                    night_color,
                    base_day_color: neutral_vertex_color(),
                    base_night_color: neutral_vertex_color(),
                    day_alpha: 1.0,
                    night_alpha: 1.0,
                    alpha: 1.0,
                })
                .collect(),
                face_indices: vec![0],
            }],
            bounds: Bounds {
                min: Vec3::ZERO,
                max: Vec3::ONE,
            },
            material_animations: Vec::new(),
            uv_animations: Vec::new(),
            effects_2dfx: Vec::new(),
            components: Vec::new(),
            component_pivots: Vec::new(),
        };
        let dirty_mesh_key = "lamp.dff|test.txd".to_string();
        let context = CoronaGenerationContext {
            root,
            emitters: HashMap::from([(material_emitter_key("lamp.dff", 0), emitter)]),
            byte_overrides: BTreeMap::new(),
            raw_overrides: BTreeMap::from([(
                asset_key("lamp.dff", ".dff"),
                ("lamp.dff".to_string(), raw),
            )]),
            vertex_mesh_overrides: BTreeMap::from([(
                asset_key("lamp.dff", ".dff"),
                vec![(dirty_mesh_key.clone(), runtime_mesh)],
            )]),
            deleted_assets: HashSet::new(),
            building_dffs: HashSet::new(),
        };
        let (progress, _) = mpsc::channel();

        let result = generate_all_dff_2dfx_coronas(context, &progress).unwrap();

        assert_eq!(result.scanned, 1);
        assert_eq!(result.created, 1);
        assert_eq!(result.replacements.len(), 1);
        assert_eq!(
            result.replacements[0].preserved_vertex_mesh_keys,
            vec![dirty_mesh_key]
        );
        let reparsed = parse_dff_mesh(&result.replacements[0].bytes);
        assert_eq!(reparsed.effects_2dfx.len(), 1);
        assert!(
            reparsed
                .prelit_colors
                .iter()
                .all(|color| to_mq(*color).distance(to_mq(day_color)) < 0.01)
        );
        assert!(
            reparsed
                .night_prelit_colors
                .iter()
                .all(|color| to_mq(*color).distance(to_mq(night_color)) < 0.01)
        );
        assert_eq!(
            &reparsed.effects_2dfx[0].payload[8..12],
            &DFF_GENERATED_POINTLIGHT_RANGE_ADDITIVE.to_le_bytes()
        );
        assert_eq!(
            reparsed.effects_2dfx[0].payload[3],
            DFF_GENERATED_CORONA_ALPHA
        );
        assert_eq!(
            &reparsed.effects_2dfx[0].payload[12..16],
            &DFF_GENERATED_CORONA_SIZE.to_le_bytes()
        );
        assert_eq!(
            reparsed.effects_2dfx[0].payload[DFF_2DFX_LIGHT_FLAGS_1_OFFSET],
            0x02 | 0x40
        );
        assert_eq!(
            reparsed.effects_2dfx[0].payload[DFF_2DFX_LIGHT_FLAGS_2_OFFSET],
            0x04
        );
        assert!(dff_2dfx_is_generated_corona(&reparsed.effects_2dfx[0]));
    }

    #[test]
    fn collision_material_selection_finds_matching_faces_and_vertices() {
        let vertices = vec![V3::default(); 7];
        let bounds = bounds_from_vertices(&vertices);
        let mesh = CollisionMesh {
            name: "materials".to_string(),
            spheres: Vec::new(),
            boxes: Vec::new(),
            vertices,
            faces: vec![
                test_face(0, 1, 2, 4, 130),
                test_face(2, 3, 4, 9, 130),
                test_face(2, 5, 6, 4, 130),
            ],
            bounds,
            shadow_vertices: Vec::new(),
            shadow_faces: Vec::new(),
        };

        assert_eq!(
            col_face_indices_with_material(&mesh, 4),
            [0, 2].into_iter().collect()
        );
        assert_eq!(
            col_vertex_indices_with_material(&mesh, 4),
            [0, 1, 2, 5, 6].into_iter().collect()
        );
    }

    #[test]
    fn optimize_merges_coplanar_stacked_rectangles_when_material_matches() {
        let mut mesh = stacked_wall_mesh(&[1, 1, 1, 1]);

        let removed = simplify_col_coplanar_faces(&mut mesh);

        assert_eq!(removed, 6);
        assert_eq!(mesh.faces.len(), 2);
        assert!(mesh.faces.iter().all(|face| face.material == 1));
    }

    #[test]
    fn optimize_merges_coplanar_rectangles_with_mixed_winding() {
        let mut mesh = stacked_wall_mesh(&[1, 1, 1]);
        mesh.faces[2] = test_face(mesh.faces[2].a, mesh.faces[2].c, mesh.faces[2].b, 1, 130);

        let removed = simplify_col_coplanar_faces(&mut mesh);

        assert_eq!(removed, 4);
        assert_eq!(mesh.faces.len(), 2);
        assert!(mesh.faces.iter().all(|face| face.material == 1));
    }

    #[test]
    fn cleanup_welds_duplicate_seams_before_merging_planes() {
        let vertices = vec![
            V3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            V3 {
                x: 10.0,
                y: 0.0,
                z: 0.0,
            },
            V3 {
                x: 0.0,
                y: 0.0,
                z: 10.0,
            },
            V3 {
                x: 10.0,
                y: 0.0,
                z: 10.0,
            },
            V3 {
                x: 10.0,
                y: 0.0,
                z: 0.0,
            },
            V3 {
                x: 20.0,
                y: 0.0,
                z: 0.0,
            },
            V3 {
                x: 10.0,
                y: 0.0,
                z: 10.0,
            },
            V3 {
                x: 20.0,
                y: 0.0,
                z: 10.0,
            },
        ];
        let bounds = bounds_from_vertices(&vertices);
        let mut mesh = CollisionMesh {
            name: "seamed_plane".to_string(),
            spheres: Vec::new(),
            boxes: Vec::new(),
            vertices,
            faces: vec![
                test_face(0, 1, 3, 1, 130),
                test_face(0, 3, 2, 1, 130),
                test_face(4, 5, 7, 1, 130),
                test_face(4, 7, 6, 1, 130),
            ],
            bounds,
            shadow_vertices: Vec::new(),
            shadow_faces: Vec::new(),
        };

        let welded = merge_col_vertices_by_distance(&mut mesh, MERGE_BY_DISTANCE_DEFAULT);
        let removed = simplify_col_coplanar_faces(&mut mesh);

        assert_eq!(welded, 2);
        assert_eq!(removed, 2);
        assert_eq!(mesh.faces.len(), 2);
    }

    #[test]
    fn optimize_merges_coplanar_plane_with_t_junction_boundary() {
        let vertices = vec![
            V3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            V3 {
                x: 10.0,
                y: 0.0,
                z: 0.0,
            },
            V3 {
                x: 20.0,
                y: 0.0,
                z: 0.0,
            },
            V3 {
                x: 0.0,
                y: 0.0,
                z: 10.0,
            },
            V3 {
                x: 10.0,
                y: 0.0,
                z: 10.0,
            },
            V3 {
                x: 20.0,
                y: 0.0,
                z: 10.0,
            },
            V3 {
                x: 0.0,
                y: 0.0,
                z: 20.0,
            },
            V3 {
                x: 20.0,
                y: 0.0,
                z: 20.0,
            },
        ];
        let bounds = bounds_from_vertices(&vertices);
        let mut mesh = CollisionMesh {
            name: "t_junction_plane".to_string(),
            spheres: Vec::new(),
            boxes: Vec::new(),
            vertices,
            faces: vec![
                test_face(0, 1, 4, 1, 130),
                test_face(0, 4, 3, 1, 130),
                test_face(1, 2, 5, 1, 130),
                test_face(1, 5, 4, 1, 130),
                test_face(3, 4, 7, 1, 130),
                test_face(3, 7, 6, 1, 130),
                test_face(4, 5, 7, 1, 130),
            ],
            bounds,
            shadow_vertices: Vec::new(),
            shadow_faces: Vec::new(),
        };

        let removed = simplify_col_coplanar_faces(&mut mesh);

        assert_eq!(removed, 5);
        assert_eq!(mesh.faces.len(), 2);
    }

    #[test]
    fn selected_cleanup_merges_non_manifold_coplanar_patch() {
        let vertices = vec![
            V3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            V3 {
                x: 10.0,
                y: 0.0,
                z: 0.0,
            },
            V3 {
                x: 20.0,
                y: 0.0,
                z: 0.0,
            },
            V3 {
                x: 0.0,
                y: 0.0,
                z: 10.0,
            },
            V3 {
                x: 10.0,
                y: 0.0,
                z: 10.0,
            },
            V3 {
                x: 20.0,
                y: 0.0,
                z: 10.0,
            },
        ];
        let bounds = bounds_from_vertices(&vertices);
        let mut mesh = CollisionMesh {
            name: "non_manifold_patch".to_string(),
            spheres: Vec::new(),
            boxes: Vec::new(),
            vertices,
            faces: vec![
                test_face(0, 1, 4, 1, 130),
                test_face(0, 4, 3, 1, 130),
                test_face(1, 2, 5, 1, 130),
                test_face(1, 5, 4, 1, 130),
                test_face(1, 4, 5, 1, 130),
            ],
            bounds,
            shadow_vertices: Vec::new(),
            shadow_faces: Vec::new(),
        };
        let selected = (0..mesh.faces.len()).collect::<BTreeSet<_>>();

        let removed = simplify_selected_col_faces(&mut mesh, &selected);

        assert_eq!(removed, 3);
        assert_eq!(mesh.faces.len(), 2);
    }

    #[test]
    fn global_cleanup_finds_projected_touching_coplanar_patch() {
        let vertices = vec![
            V3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            V3 {
                x: 10.0,
                y: 0.0,
                z: 0.0,
            },
            V3 {
                x: 20.0,
                y: 0.0,
                z: 0.0,
            },
            V3 {
                x: 0.0,
                y: 0.0,
                z: 10.0,
            },
            V3 {
                x: 10.0,
                y: 0.0,
                z: 10.0,
            },
            V3 {
                x: 20.0,
                y: 0.0,
                z: 10.0,
            },
        ];
        let bounds = bounds_from_vertices(&vertices);
        let mut mesh = CollisionMesh {
            name: "projected_touch_patch".to_string(),
            spheres: Vec::new(),
            boxes: Vec::new(),
            vertices,
            faces: vec![
                test_face(0, 1, 4, 1, 130),
                test_face(0, 4, 3, 1, 130),
                test_face(1, 2, 5, 1, 130),
                test_face(1, 5, 4, 1, 130),
                test_face(1, 4, 5, 1, 130),
            ],
            bounds,
            shadow_vertices: Vec::new(),
            shadow_faces: Vec::new(),
        };

        let removed = simplify_col_coplanar_faces(&mut mesh);

        assert_eq!(removed, 3);
        assert_eq!(mesh.faces.len(), 2);
    }

    #[test]
    fn optimize_keeps_material_boundaries() {
        let mut mesh = stacked_wall_mesh(&[1, 1, 2, 2]);

        let removed = simplify_col_coplanar_faces(&mut mesh);

        assert_eq!(removed, 4);
        assert_eq!(mesh.faces.len(), 4);
        assert_eq!(
            mesh.faces.iter().filter(|face| face.material == 1).count(),
            2
        );
        assert_eq!(
            mesh.faces.iter().filter(|face| face.material == 2).count(),
            2
        );
    }

    #[test]
    fn merge_raw_vertices_by_distance_preserves_sidecar_streams() {
        let mut raw = RawMesh {
            vertices: vec![
                V3::default(),
                V3 {
                    x: 0.00005,
                    y: 0.0,
                    z: 0.0,
                },
                V3 {
                    x: 1.0,
                    y: 0.0,
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
                V2 { u: 0.0, v: 0.0 },
                V2 { u: 1.0, v: 0.0 },
            ],
            secondary_uvs: vec![vec![
                V2 { u: 0.25, v: 0.75 },
                V2 { u: 0.25, v: 0.75 },
                V2 { u: 0.75, v: 0.25 },
            ]],
            prelit_colors: vec![
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
                    z: 1.0,
                },
            ],
            prelit_alphas: vec![0.25, 0.5, 0.75],
            night_prelit_colors: Vec::new(),
            night_prelit_alphas: vec![0.125, 0.625, 0.875],
            light_flags: vec![true, false, true],
            triangles: vec![Tri {
                a: 0,
                b: 1,
                c: 2,
                material: 0,
            }],
            material_textures: Vec::new(),
            materials: Vec::new(),
            material_animations: Vec::new(),
            uv_anim_dictionaries: Vec::new(),
            uv_animations: Vec::new(),
            effects_2dfx: Vec::new(),
            components: Vec::new(),
            frames: Vec::new(),
        };

        let merged = merge_raw_vertices_by_distance(&mut raw, MERGE_BY_DISTANCE_DEFAULT);

        assert_eq!(merged, 1);
        assert_eq!(raw.vertices.len(), 2);
        assert_eq!(raw.uvs.len(), 2);
        assert_eq!(
            raw.secondary_uvs,
            vec![vec![V2 { u: 0.25, v: 0.75 }, V2 { u: 0.75, v: 0.25 },]]
        );
        assert_eq!(raw.prelit_colors[0].x, 1.0);
        assert_eq!(raw.prelit_alphas, vec![0.25, 0.75]);
        assert_eq!(raw.night_prelit_alphas, vec![0.125, 0.875]);
        assert_eq!(raw.light_flags, vec![true, true]);
        assert!(raw.triangles.is_empty());
    }

    #[test]
    fn merge_raw_vertices_by_distance_keeps_uv_seams() {
        // Two co-located vertices with different UVs (a texture seam) must not weld.
        let mut raw = RawMesh {
            vertices: vec![
                V3::default(),
                V3::default(),
                V3 {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
            ],
            normals: Vec::new(),
            uvs: vec![
                V2 { u: 0.0, v: 0.0 },
                V2 { u: 1.0, v: 0.0 },
                V2 { u: 0.5, v: 1.0 },
            ],
            secondary_uvs: Vec::new(),
            prelit_colors: Vec::new(),
            prelit_alphas: Vec::new(),
            night_prelit_colors: Vec::new(),
            night_prelit_alphas: Vec::new(),
            light_flags: Vec::new(),
            triangles: vec![Tri {
                a: 0,
                b: 1,
                c: 2,
                material: 0,
            }],
            material_textures: Vec::new(),
            materials: Vec::new(),
            material_animations: Vec::new(),
            uv_anim_dictionaries: Vec::new(),
            uv_animations: Vec::new(),
            effects_2dfx: Vec::new(),
            components: Vec::new(),
            frames: Vec::new(),
        };

        let merged = merge_raw_vertices_by_distance(&mut raw, MERGE_BY_DISTANCE_DEFAULT);

        assert_eq!(merged, 0);
        assert_eq!(raw.vertices.len(), 3);
        assert_eq!(raw.uvs.len(), 3);
        assert_eq!(raw.triangles.len(), 1);
    }

    #[test]
    fn merge_selected_dff_vertices_preserves_uv_seams() {
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
                V3 {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                V3 {
                    x: 2.0,
                    y: 1.0,
                    z: 0.0,
                },
            ],
            normals: Vec::new(),
            uvs: vec![
                V2 { u: 0.0, v: 0.0 },
                V2 { u: 1.0, v: 0.0 },
                V2 { u: 0.0, v: 1.0 },
                V2 { u: 1.0, v: 0.0 },
                V2 { u: 0.0, v: 0.0 },
                V2 { u: 0.0, v: 1.0 },
            ],
            secondary_uvs: Vec::new(),
            prelit_colors: Vec::new(),
            prelit_alphas: Vec::new(),
            night_prelit_colors: Vec::new(),
            night_prelit_alphas: Vec::new(),
            light_flags: Vec::new(),
            triangles: vec![
                Tri {
                    a: 0,
                    b: 1,
                    c: 2,
                    material: 0,
                },
                Tri {
                    a: 4,
                    b: 3,
                    c: 5,
                    material: 0,
                },
            ],
            material_textures: Vec::new(),
            materials: Vec::new(),
            material_animations: Vec::new(),
            uv_anim_dictionaries: Vec::new(),
            uv_animations: Vec::new(),
            effects_2dfx: Vec::new(),
            components: Vec::new(),
            frames: Vec::new(),
        };
        let selected = [1usize, 4usize].into_iter().collect::<BTreeSet<_>>();

        let (_, uv_groups) = merge_selected_raw_vertices_preserving_uvs(
            &mut raw,
            &selected,
            Some(4),
            DffMergeTarget::Center,
        )
        .unwrap();
        compact_raw_vertices(&mut raw);

        assert_eq!(uv_groups, 2);
        assert_eq!(raw.vertices.len(), 6);
        assert_eq!(raw.uvs.len(), 6);
        assert_eq!(raw.triangles.len(), 2);
        let merged = raw
            .vertices
            .iter()
            .enumerate()
            .filter_map(|(idx, vertex)| {
                ((vertex.x - 1.0).abs() < f32::EPSILON
                    && vertex.y.abs() < f32::EPSILON
                    && vertex.z.abs() < f32::EPSILON)
                    .then_some(raw.uvs[idx])
            })
            .collect::<Vec<_>>();
        assert!(merged.contains(&V2 { u: 1.0, v: 0.0 }));
        assert!(merged.contains(&V2 { u: 0.0, v: 0.0 }));
    }

    #[test]
    fn merge_selected_dff_vertices_can_use_last_position() {
        let mut raw = RawMesh {
            vertices: vec![
                V3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                V3 {
                    x: 4.0,
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
                    y: 2.0,
                    z: 0.0,
                },
                V3 {
                    x: 5.0,
                    y: 0.0,
                    z: 0.0,
                },
                V3 {
                    x: 5.0,
                    y: 1.0,
                    z: 0.0,
                },
            ],
            normals: Vec::new(),
            uvs: vec![
                V2 { u: 0.0, v: 0.0 },
                V2 { u: 0.0, v: 0.0 },
                V2 { u: 1.0, v: 0.0 },
                V2 { u: 1.0, v: 1.0 },
                V2 { u: 1.0, v: 0.0 },
                V2 { u: 1.0, v: 1.0 },
            ],
            secondary_uvs: Vec::new(),
            prelit_colors: Vec::new(),
            prelit_alphas: Vec::new(),
            night_prelit_colors: Vec::new(),
            night_prelit_alphas: Vec::new(),
            light_flags: Vec::new(),
            triangles: vec![
                Tri {
                    a: 0,
                    b: 2,
                    c: 3,
                    material: 0,
                },
                Tri {
                    a: 1,
                    b: 4,
                    c: 5,
                    material: 0,
                },
            ],
            material_textures: Vec::new(),
            materials: Vec::new(),
            material_animations: Vec::new(),
            uv_anim_dictionaries: Vec::new(),
            uv_animations: Vec::new(),
            effects_2dfx: Vec::new(),
            components: Vec::new(),
            frames: Vec::new(),
        };
        let selected = [0usize, 1usize].into_iter().collect::<BTreeSet<_>>();

        let (position, uv_groups) = merge_selected_raw_vertices_preserving_uvs(
            &mut raw,
            &selected,
            Some(1),
            DffMergeTarget::Last,
        )
        .unwrap();
        compact_raw_vertices(&mut raw);

        assert_eq!(position.x, 4.0);
        assert_eq!(uv_groups, 1);
        assert_eq!(raw.vertices[0].x, 4.0);
    }

    #[test]
    fn gif_flipbook_uses_dragonff_uv_anim_slots() {
        let fb = GifFlipbook {
            rgba: Vec::new(),
            width: 128,
            height: 64,
            cols: 2,
            rows: 2,
            frame_count: 4,
            frame_starts: vec![0.0, 0.1, 0.2, 0.3],
            duration: 0.4,
        };
        let animation = build_gif_uv_animation("atlas_gif", &fb);

        assert_eq!(animation.frames[0].uv, [0.0, 0.5, 0.5, 0.0, 0.0, 0.5]);
        assert_eq!(animation.frames[2].uv, [0.0, 0.5, 0.5, 0.0, 0.5, 0.5]);
        assert_eq!(animation.frames[4].uv, [0.0, 0.5, 0.5, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn merge_col_vertices_by_distance_remaps_faces() {
        let vertices = vec![
            V3::default(),
            V3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            V3 {
                x: 1.00005,
                y: 0.0,
                z: 0.0,
            },
            V3 {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
        ];
        let bounds = bounds_from_vertices(&vertices);
        let mut mesh = CollisionMesh {
            name: "merge".to_string(),
            spheres: Vec::new(),
            boxes: Vec::new(),
            vertices,
            faces: vec![test_face(0, 1, 3, 1, 130), test_face(0, 2, 3, 1, 130)],
            bounds,
            shadow_vertices: Vec::new(),
            shadow_faces: Vec::new(),
        };

        let merged = merge_col_vertices_by_distance(&mut mesh, MERGE_BY_DISTANCE_DEFAULT);

        assert_eq!(merged, 1);
        assert_eq!(mesh.vertices.len(), 3);
        assert_eq!(mesh.faces.len(), 2);
        assert_eq!(mesh.faces[1].b, mesh.faces[0].b);
    }

    fn assert_closed_shadow_edges(mesh: &GeneratedShadowMesh) {
        let mut edges = HashMap::<(u16, u16), usize>::new();
        for face in &mesh.faces {
            for (a, b) in [(face.a, face.b), (face.b, face.c), (face.c, face.a)] {
                let edge = if a < b { (a, b) } else { (b, a) };
                *edges.entry(edge).or_default() += 1;
            }
        }
        assert!(
            edges.values().all(|uses| *uses == 2),
            "generated skin contains non-closed edges: {edges:?}"
        );
    }

    fn signed_shadow_volume(mesh: &GeneratedShadowMesh, faces: &[usize]) -> f64 {
        faces.iter().fold(0.0, |volume, face_index| {
            let face = &mesh.faces[*face_index];
            let a = mesh.vertices[face.a as usize];
            let b = mesh.vertices[face.b as usize];
            let c = mesh.vertices[face.c as usize];
            volume
                + (a.x as f64 * (b.y as f64 * c.z as f64 - b.z as f64 * c.y as f64)
                    + a.y as f64 * (b.z as f64 * c.x as f64 - b.x as f64 * c.z as f64)
                    + a.z as f64 * (b.x as f64 * c.y as f64 - b.y as f64 * c.x as f64))
                    / 6.0
        })
    }

    #[test]
    fn shadow_skin_closes_an_open_triangle() {
        let vertices = vec![
            V3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            V3 {
                x: 10.0,
                y: 0.0,
                z: 0.0,
            },
            V3 {
                x: 0.0,
                y: 10.0,
                z: 0.0,
            },
        ];
        let source = CollisionMesh {
            name: "open".to_string(),
            spheres: Vec::new(),
            boxes: Vec::new(),
            bounds: bounds_from_vertices(&vertices),
            vertices,
            faces: vec![test_face(0, 1, 2, 0, 0)],
            shadow_vertices: Vec::new(),
            shadow_faces: Vec::new(),
        };

        let shadow = generate_closed_shadow_skin(source).unwrap();

        assert_eq!(shadow.vertices.len(), 6);
        assert_eq!(shadow.faces.len(), 8);
        assert_eq!(shadow.closed_borders, 3);
        assert_eq!(shadow.closed_components, 1);
        assert_closed_shadow_edges(&shadow);
        assert!(signed_shadow_volume(&shadow, &(0..shadow.faces.len()).collect::<Vec<_>>()) < 0.0);
        assert!(
            shadow
                .faces
                .iter()
                .all(|face| face.material == 0 && face.light == 255)
        );
    }

    #[test]
    fn shadow_skin_converts_collision_primitives_to_closed_triangles() {
        let source = CollisionMesh {
            name: "box".to_string(),
            spheres: Vec::new(),
            boxes: vec![CollisionBox {
                min: V3 {
                    x: -1.0,
                    y: -2.0,
                    z: -3.0,
                },
                max: V3 {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                },
                surface: CollisionSurface {
                    material: 0,
                    flags: 0,
                    brightness: 0,
                    light: 0,
                },
            }],
            vertices: Vec::new(),
            faces: Vec::new(),
            bounds: Bounds {
                min: Vec3::ZERO,
                max: Vec3::ZERO,
            },
            shadow_vertices: Vec::new(),
            shadow_faces: Vec::new(),
        };

        let shadow = generate_closed_shadow_skin(source).unwrap();

        assert_eq!(shadow.vertices.len(), 8);
        assert_eq!(shadow.faces.len(), 12);
        assert_eq!(shadow.closed_borders, 0);
        assert_eq!(shadow.closed_components, 1);
        assert_closed_shadow_edges(&shadow);
    }

    #[test]
    fn shadow_skin_orients_disconnected_closed_components_for_gta() {
        let vertices = vec![
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
                z: 1.0,
            },
            V3 {
                x: 3.0,
                y: 0.0,
                z: 0.0,
            },
            V3 {
                x: 4.0,
                y: 0.0,
                z: 0.0,
            },
            V3 {
                x: 3.0,
                y: 1.0,
                z: 0.0,
            },
            V3 {
                x: 3.0,
                y: 0.0,
                z: 1.0,
            },
        ];
        // The first tetrahedron has positive signed volume and the second has
        // negative signed volume, reproducing d3_h59's mixed component winding.
        let faces = vec![
            test_face(1, 2, 3, 7, 0),
            test_face(0, 3, 2, 7, 0),
            test_face(0, 1, 3, 7, 0),
            test_face(0, 2, 1, 7, 0),
            test_face(5, 7, 6, 9, 0),
            test_face(4, 6, 7, 9, 0),
            test_face(4, 7, 5, 9, 0),
            test_face(4, 5, 6, 9, 0),
        ];
        let source = CollisionMesh {
            name: "mixed_shells".to_string(),
            spheres: Vec::new(),
            boxes: Vec::new(),
            bounds: bounds_from_vertices(&vertices),
            vertices,
            faces,
            shadow_vertices: Vec::new(),
            shadow_faces: Vec::new(),
        };

        let shadow = generate_closed_shadow_skin(source).unwrap();

        assert_eq!(shadow.closed_components, 2);
        assert_eq!(shadow.closed_borders, 0);
        assert_closed_shadow_edges(&shadow);
        assert!(signed_shadow_volume(&shadow, &[0, 1, 2, 3]) < 0.0);
        assert!(signed_shadow_volume(&shadow, &[4, 5, 6, 7]) < 0.0);
        assert!(
            shadow
                .faces
                .iter()
                .all(|face| face.material == 0 && face.light == 255)
        );
    }

    #[test]
    fn shadow_border_uses_non_degenerate_diagonal_after_quantization() {
        let vertices = vec![
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
        ];
        let mut faces = Vec::new();

        append_closed_shadow_border_faces(&vertices, &mut faces, 0, 1, 2).unwrap();

        assert_eq!(faces.len(), 2);
        assert_eq!((faces[0].a, faces[0].b, faces[0].c), (0, 1, 2));
        assert_eq!((faces[1].a, faces[1].b, faces[1].c), (1, 3, 2));
        assert!(faces.iter().all(|face| shadow_triangle_has_area(
            &vertices,
            face.a as usize,
            face.b as usize,
            face.c as usize
        )));
    }

    #[test]
    fn shadow_border_grid_fallback_cannot_run_parallel_to_an_edge() {
        let source = vec![
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
        ];
        let boundary = vec![(0, 1)];

        let vertices = grid_safe_shadow_offset_vertices(&source, &boundary).unwrap();
        let mut faces = Vec::new();
        append_closed_shadow_border_faces(&vertices, &mut faces, 0, 1, source.len()).unwrap();

        assert_eq!(faces.len(), 2);
        assert!(faces.iter().all(|face| shadow_triangle_has_area(
            &vertices,
            face.a as usize,
            face.b as usize,
            face.c as usize
        )));
    }

    #[test]
    fn shadow_border_accepts_d3_h59_minimum_grid_triangle() {
        // Exact fixed-point coordinates captured from NICE's d3_h59 border.
        // Its valid area squared is no larger than f32::EPSILON, which must
        // never be treated as a geometric zero threshold.
        let vertices = vec![
            V3 {
                x: 6.0 / 128.0,
                y: -1.0 / 128.0,
                z: 168.0 / 128.0,
            },
            V3 {
                x: 6.0 / 128.0,
                y: -3.0 / 128.0,
                z: 166.0 / 128.0,
            },
            V3 {
                x: 8.0 / 128.0,
                y: 1.0 / 128.0,
                z: 170.0 / 128.0,
            },
            V3 {
                x: 8.0 / 128.0,
                y: -1.0 / 128.0,
                z: 168.0 / 128.0,
            },
        ];
        let area_squared = (to_mq(vertices[1]) - to_mq(vertices[0]))
            .cross(to_mq(vertices[3]) - to_mq(vertices[0]))
            .length_squared();
        assert!(area_squared > 0.0);
        assert!(area_squared <= f32::EPSILON);

        let mut faces = Vec::new();
        append_closed_shadow_border_faces(&vertices, &mut faces, 0, 1, 2).unwrap();

        assert_eq!(faces.len(), 2);
        assert!(faces.iter().all(|face| shadow_triangle_has_area(
            &vertices,
            face.a as usize,
            face.b as usize,
            face.c as usize
        )));
    }

    #[test]
    fn duplicate_texture_suggestion_keeps_suffix_with_long_texture_names() {
        let source = "abcdefghijklmnopqrstuvwxyz123456";
        let first = suggested_duplicate_texture_name(&[source.to_string()], source);
        assert_eq!(first.len(), GTA_SA_TEXTURE_NAME_MAX);
        assert!(first.ends_with("_copy"));
        let second = suggested_duplicate_texture_name(&[source.to_string(), first.clone()], source);
        assert_eq!(second.len(), GTA_SA_TEXTURE_NAME_MAX);
        assert!(second.ends_with("_copy2"));
        assert_ne!(first, second);
    }

    #[test]
    fn txd_texture_search_is_case_insensitive_and_preserves_source_indices() {
        let entry = |name: &str| TextureArchiveEntry {
            name: name.to_string(),
            width: 16,
            height: 16,
            format: TxFormat::Dxt1,
            fingerprint: None,
            thumbnail: None,
        };
        let txd = EditingTxdState {
            name: "city.txd".to_string(),
            textures: vec![entry("brick"), entry("Road_Main"), entry("road_markings")],
            selected: 0,
            scroll: 0.0,
            search: "ROAD".to_string(),
            search_cursor: 4,
            search_anchor: None,
            search_active: true,
            preview_texture: None,
            material_picker_open: false,
            material_picker_search: String::new(),
            material_picker_scroll: 0.0,
            material_picker_scope: CollisionMaterialAssignmentScope::ExactTxd,
        };

        assert_eq!(editing_txd_filtered_indices(&txd), vec![1, 2]);
    }

    #[test]
    fn txd_texture_wheel_scrolls_and_clamps_to_visible_rows() {
        assert_eq!(editing_txd_scroll_value(0.0, 38, 672.0, -1.0), 3.0);
        assert_eq!(editing_txd_scroll_value(15.0, 38, 672.0, -1.0), 17.0);
        assert_eq!(editing_txd_scroll_value(1.0, 38, 672.0, 2.0), 0.0);
    }

    #[test]
    fn txd_texture_wheel_does_not_scroll_when_every_row_is_visible() {
        assert_eq!(editing_txd_scroll_value(5.0, 8, 672.0, -4.0), 0.0);
    }

    #[test]
    fn separation_moves_only_selected_faces_and_compacts_vertex_streams() {
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
            uvs: vec![V2::default(); 4],
            prelit_colors: vec![neutral_vertex_color(); 4],
            triangles: vec![
                Tri {
                    a: 0,
                    b: 1,
                    c: 2,
                    material: 0,
                },
                Tri {
                    a: 1,
                    b: 3,
                    c: 2,
                    material: 1,
                },
            ],
            effects_2dfx: vec![Dff2dEffect::default()],
            ..RawMesh::default()
        };
        recalc_raw_normals(&mut raw);

        let (remaining, separated) = split_raw_mesh_faces(&raw, &BTreeSet::from([1usize])).unwrap();

        assert_eq!(remaining.triangles.len(), 1);
        assert_eq!(remaining.triangles[0].material, 0);
        assert_eq!(remaining.vertices.len(), 3);
        assert_eq!(remaining.uvs.len(), 3);
        assert_eq!(remaining.prelit_colors.len(), 3);
        assert_eq!(remaining.effects_2dfx.len(), 1);
        assert_eq!(separated.triangles.len(), 1);
        assert_eq!(separated.triangles[0].material, 1);
        assert_eq!(separated.vertices.len(), 3);
        assert_eq!(separated.uvs.len(), 3);
        assert_eq!(separated.prelit_colors.len(), 3);
        assert!(separated.effects_2dfx.is_empty());
    }

    #[test]
    fn pivot_shift_and_placement_compensation_preserve_world_position() {
        let pivot = V3 {
            x: 1.0,
            y: 0.5,
            z: -0.25,
        };
        let original_vertex = V3 {
            x: 4.0,
            y: 2.0,
            z: 1.0,
        };
        let mut raw = RawMesh {
            vertices: vec![original_vertex],
            effects_2dfx: vec![Dff2dEffect {
                position: original_vertex,
                ..Dff2dEffect::default()
            }],
            ..RawMesh::default()
        };
        let placement = Placement {
            id: "source".to_string(),
            dff: "source".to_string(),
            zone: "zone".to_string(),
            tag: "object".to_string(),
            attrs: BTreeMap::from([("scale".to_string(), "2".to_string())]),
            pos: V3 {
                x: 10.0,
                y: 20.0,
                z: 30.0,
            },
            rot: V3 {
                x: 10.0,
                y: -20.0,
                z: 90.0,
            },
        };
        let original_matrix = placement_matrix(&placement);
        let before = original_matrix.transform_point3(to_mq(original_vertex));

        shift_raw_mesh_pivot(&mut raw, pivot);
        let offset = original_matrix.transform_vector3(to_mq(pivot));
        let mut compensated = placement.clone();
        compensated.pos = from_mq(to_mq(compensated.pos) + offset);
        let after = placement_matrix(&compensated).transform_point3(to_mq(raw.vertices[0]));

        assert!(before.distance(after) < 0.0001);
        assert_eq!(
            raw.effects_2dfx[0].position,
            V3 {
                x: 3.0,
                y: 1.5,
                z: 1.25
            }
        );
    }

    #[test]
    fn separated_dff_names_enforce_img_constraints() {
        assert_eq!(
            validate_separated_dff_name("lamp_part").unwrap(),
            ("lamp_part.dff".to_string(), "lamp_part".to_string())
        );
        assert!(validate_separated_dff_name("../lamp").is_err());
        assert!(validate_separated_dff_name("lamp part").is_err());
        assert!(validate_separated_dff_name("1234567890123456789").is_ok());
        assert!(validate_separated_dff_name("12345678901234567890").is_err());
        assert!(validate_separated_dff_name("this_name_is_far_too_long_for_an_img_entry").is_err());
    }

    #[test]
    fn direct_img_save_clears_only_matching_global_staging() {
        let mut replacements = BTreeMap::from([
            (
                "saved.txd".to_string(),
                ("saved.txd".to_string(), vec![1, 2, 3]),
            ),
            (
                "newer.txd".to_string(),
                ("newer.txd".to_string(), vec![9, 9, 9]),
            ),
        ]);
        let mut txd_writes = HashSet::from(["saved.txd".to_string(), "newer.txd".to_string()]);

        reconcile_saved_global_asset_staging(
            &mut replacements,
            &mut txd_writes,
            "saved.txd",
            &[1, 2, 3],
        );
        reconcile_saved_global_asset_staging(
            &mut replacements,
            &mut txd_writes,
            "newer.txd",
            &[1, 2, 3],
        );

        assert!(!replacements.contains_key("saved.txd"));
        assert!(!txd_writes.contains("saved.txd"));
        assert_eq!(replacements["newer.txd"].1, vec![9, 9, 9]);
        assert!(txd_writes.contains("newer.txd"));
    }

    fn empty_capsule_test_mesh() -> CollisionMesh {
        CollisionMesh {
            name: "capsule_test".to_string(),
            spheres: Vec::new(),
            boxes: Vec::new(),
            vertices: Vec::new(),
            faces: Vec::new(),
            bounds: Bounds {
                min: Vec3::ZERO,
                max: Vec3::ZERO,
            },
            shadow_vertices: Vec::new(),
            shadow_faces: Vec::new(),
        }
    }

    fn test_editing_col_state(
        mesh: CollisionMesh,
        capsules: Vec<CollisionCapsule>,
        cuboids: Vec<CollisionCuboid>,
    ) -> EditingColState {
        EditingColState {
            name: "capsule_test.col".to_string(),
            mesh,
            bytes: minimal_col2_template("capsule_test"),
            source_model: ColModelIdentity {
                index: 0,
                name: "capsule_test".to_string(),
                magic: *b"COL2",
            },
            embedded_vehicle_dff: false,
            embedded_source_dff_bytes: None,
            dff_overlay: None,
            dff_overlay_name: None,
            dff_overlay_visible: false,
            editing_shadow: false,
            selected_face: 0,
            selected_faces: BTreeSet::new(),
            selected_edges: BTreeSet::new(),
            select_mode: EditingSelectMode::Vertex,
            face_scroll: 0.0,
            primitive_scroll: 0.0,
            selected_vertex: 0,
            selected_primitive: None,
            capsules,
            cuboids,
            box_pick_enabled: true,
            selected_vertices: BTreeSet::new(),
            hovered_face: None,
            hovered_vertex: None,
            dirty: false,
            panel_scroll: 0.0,
            panel_collapsed: col_default_collapsed(),
        }
    }

    #[test]
    fn generated_primitive_ownership_covers_capsule_and_cuboid_but_not_shadow_layer() {
        let mut mesh = empty_capsule_test_mesh();
        let surface = CollisionSurface {
            material: 7,
            flags: 1,
            brightness: 2,
            light: 200,
        };
        let capsule = append_capsule_artifacts(
            &mut mesh,
            V3::default(),
            V3 {
                x: 0.0,
                y: 0.0,
                z: 3.0,
            },
            0.5,
            true,
            surface.clone(),
        )
        .unwrap();
        let cuboid = append_cuboid_artifacts(
            &mut mesh,
            V3 {
                x: 4.0,
                y: 0.0,
                z: 0.0,
            },
            V3 {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            V3 {
                x: 0.0,
                y: 0.0,
                z: 30.0,
            },
            surface,
        )
        .unwrap();
        let capsule_face = capsule.face_start;
        let capsule_vertex = capsule.vertex_start;
        let cuboid_face = cuboid.face_start;
        let cuboid_vertex = cuboid.vertex_start;
        let mut col = test_editing_col_state(mesh, vec![capsule], vec![cuboid]);

        assert!(
            col_generated_primitive_for_face(&col, capsule_face).is_some_and(|selection| {
                selection.kind == CollisionPrimitiveKind::Capsule && selection.index == 0
            })
        );
        assert!(
            col_generated_primitive_for_vertex(&col, cuboid_vertex).is_some_and(|selection| {
                selection.kind == CollisionPrimitiveKind::Cuboid && selection.index == 0
            })
        );
        assert!(col_faces_touch_generated_primitive(
            &col,
            [capsule_face, cuboid_face]
        ));
        assert!(col_vertices_touch_generated_primitive(
            &col,
            [capsule_vertex, cuboid_vertex]
        ));
        assert!(col_regular_topology_has_editable_primitives(&col));

        col.editing_shadow = true;
        assert!(col_generated_primitive_for_face(&col, capsule_face).is_none());
        assert!(col_generated_primitive_for_vertex(&col, cuboid_vertex).is_none());
        assert!(!col_regular_topology_has_editable_primitives(&col));
    }

    #[test]
    fn current_col_validation_serializes_the_edited_mesh() {
        let mut mesh = empty_capsule_test_mesh();
        mesh.vertices = vec![
            V3 {
                x: 300.0,
                y: 0.0,
                z: 0.0,
            },
            V3 {
                x: 301.0,
                y: 0.0,
                z: 0.0,
            },
            V3 {
                x: 300.0,
                y: 1.0,
                z: 0.0,
            },
        ];
        mesh.faces.push(capsule_generated_face(
            0,
            1,
            2,
            &CollisionSurface {
                material: 0,
                flags: 0,
                brightness: 0,
                light: 255,
            },
        ));
        let col = test_editing_col_state(mesh, Vec::new(), Vec::new());

        let error = current_editing_col_bytes(&col).unwrap_err();
        assert!(error.contains("signed 16-bit fixed-point range"));
    }

    #[test]
    fn capsule_materializes_two_spheres_and_editable_cylinder_extents() {
        let mut mesh = empty_capsule_test_mesh();
        let surface = CollisionSurface {
            material: 7,
            flags: 1,
            brightness: 2,
            light: 200,
        };
        let mut capsule = append_capsule_artifacts(
            &mut mesh,
            V3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            V3 {
                x: 0.0,
                y: 0.0,
                z: 4.0,
            },
            1.0,
            true,
            surface,
        )
        .unwrap();

        assert_eq!(mesh.spheres.len(), 2);
        assert_eq!(mesh.vertices.len(), COL_CAPSULE_VERTEX_COUNT);
        assert_eq!(mesh.faces.len(), COL_CAPSULE_FACE_COUNT);
        assert!(
            mesh.vertices[..COL_CAPSULE_SIDES]
                .iter()
                .all(|vertex| vertex.z.abs() < 0.0001)
        );
        assert!(
            mesh.vertices[COL_CAPSULE_SIDES..COL_CAPSULE_SIDES * 2]
                .iter()
                .all(|vertex| (vertex.z - 4.0).abs() < 0.0001)
        );

        capsule.round_edges = false;
        write_capsule_artifacts(&mut mesh, &mut capsule).unwrap();
        assert!(
            mesh.vertices[..COL_CAPSULE_SIDES]
                .iter()
                .all(|vertex| (vertex.z + 1.0).abs() < 0.0001)
        );
        assert!(
            mesh.vertices[COL_CAPSULE_SIDES..COL_CAPSULE_SIDES * 2]
                .iter()
                .all(|vertex| (vertex.z - 5.0).abs() < 0.0001)
        );
        assert!(
            mesh.faces
                .iter()
                .all(|face| face.material == 7 && face.light == 200)
        );
    }

    #[test]
    fn capsule_face_ranges_recover_after_col_face_reordering() {
        let mut mesh = empty_capsule_test_mesh();
        mesh.vertices.extend([
            V3 {
                x: -2.0,
                y: 0.0,
                z: 0.0,
            },
            V3 {
                x: -2.0,
                y: 1.0,
                z: 0.0,
            },
            V3 {
                x: -2.0,
                y: 0.0,
                z: 1.0,
            },
        ]);
        mesh.faces.push(capsule_generated_face(
            0,
            1,
            2,
            &CollisionSurface {
                material: 3,
                flags: 0,
                brightness: 0,
                light: 255,
            },
        ));
        let capsule = append_capsule_artifacts(
            &mut mesh,
            V3::default(),
            V3 {
                x: 0.0,
                y: 0.0,
                z: 3.0,
            },
            0.5,
            true,
            CollisionSurface {
                material: 5,
                flags: 0,
                brightness: 0,
                light: 255,
            },
        )
        .unwrap();
        mesh.faces.rotate_left(17);

        let valid = valid_capsules_for_mesh(&mut mesh, vec![capsule]);
        assert_eq!(valid.len(), 1);
        assert_eq!(valid[0].face_start, 1);
        assert_eq!(mesh.faces.len(), 1 + COL_CAPSULE_FACE_COUNT);
        assert_eq!(mesh.faces[0].material, 3);
    }

    #[test]
    fn materialized_capsule_round_trips_as_game_loadable_native_col() {
        let mut mesh = empty_capsule_test_mesh();
        let capsule = append_capsule_artifacts(
            &mut mesh,
            V3 {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            V3 {
                x: 4.0,
                y: 6.0,
                z: 8.0,
            },
            0.75,
            true,
            CollisionSurface {
                material: 4,
                flags: 0,
                brightness: 0,
                light: 255,
            },
        )
        .unwrap();
        mesh.bounds = collision_mesh_bounds(&mesh.vertices, &mesh.spheres, &mesh.boxes);
        let source = CollisionGenerationSource {
            dff_name: "capsule_test.dff".to_string(),
            col_name: "capsule_test.col".to_string(),
            txd_name: None,
            raw_override: None,
            lod_only: false,
            definition_ids_to_assign: Vec::new(),
        };
        let generated = serialize_generated_collision(
            Path::new("/definitely/not/an/eagle/resource"),
            &BTreeMap::new(),
            &source,
            mesh,
        )
        .unwrap();
        assert!(!col_validation_has_errors(&validate_col_for_game_load(
            "capsule_test.col",
            &generated.bytes
        )));
        let entry = ImgEntry {
            img_path: PathBuf::from("capsule_test.col"),
            name: "capsule_test.col".to_string(),
            offset: 0,
            size: generated.bytes.len() as u32,
        };
        let mut reparsed = parse_col_mesh(&generated.bytes, &entry).unwrap();
        let recovered = valid_capsules_for_mesh(&mut reparsed, vec![capsule]);
        assert_eq!(reparsed.spheres.len(), 2);
        assert_eq!(reparsed.faces.len(), COL_CAPSULE_FACE_COUNT);
        assert_eq!(recovered.len(), 1);
    }

    #[test]
    fn collision_shadow_face_resolution_prefers_face_over_material_and_texture() {
        let mut overrides = HashMap::new();
        overrides.insert(material_emitter_texture_key("wall"), false);
        overrides.insert(material_emitter_key("tower.dff", 2), true);
        overrides.insert(material_emitter_face_key("tower.dff", 9), false);

        assert!(!dff_face_casts_shadow_from_parts(
            &overrides,
            "tower.dff",
            9,
            2,
            "wall"
        ));
        assert!(dff_face_casts_shadow_from_parts(
            &overrides,
            "tower.dff",
            8,
            2,
            "wall"
        ));
        assert!(!dff_face_casts_shadow_from_parts(
            &overrides,
            "other.dff",
            8,
            2,
            "wall"
        ));
    }
}
