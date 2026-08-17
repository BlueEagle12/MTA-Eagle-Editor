use super::super::*;

fn parse_finite_f32(value: &str) -> Option<f32> {
    value.parse::<f32>().ok().filter(|value| value.is_finite())
}

fn apply_dff_material_value(material: &mut RawMaterial, field: InspectorField, value: f32) -> bool {
    let before = *material;
    match field {
        InspectorField::DffMaterialRed => material.color.x = value.clamp(0.0, 255.0) / 255.0,
        InspectorField::DffMaterialGreen => material.color.y = value.clamp(0.0, 255.0) / 255.0,
        InspectorField::DffMaterialBlue => material.color.z = value.clamp(0.0, 255.0) / 255.0,
        InspectorField::DffMaterialAlpha => material.alpha = value.clamp(0.0, 255.0) / 255.0,
        InspectorField::DffMaterialAmbient => material.ambient = value.max(0.0),
        InspectorField::DffMaterialDiffuse => material.diffuse = value.max(0.0),
        InspectorField::DffMaterialSpecular => material.specular = value.max(0.0),
        _ => return false,
    }
    *material != before
}

#[derive(Clone, Copy)]
enum EditingSelectionCommand {
    SelectAll,
    DeselectAll,
    Invert,
}

fn editing_selection_mode_label(mode: EditingSelectMode) -> &'static str {
    match mode {
        EditingSelectMode::Vertex => "Vertex",
        EditingSelectMode::Edge => "Edge",
        EditingSelectMode::Face => "Face",
    }
}

fn input_edge_key(a: usize, b: usize) -> (usize, usize) {
    if a <= b { (a, b) } else { (b, a) }
}

fn topology_edges(
    faces: impl IntoIterator<Item = [usize; 3]>,
    vertex_count: usize,
) -> BTreeSet<(usize, usize)> {
    faces
        .into_iter()
        .flat_map(|[a, b, c]| {
            [
                input_edge_key(a, b),
                input_edge_key(b, c),
                input_edge_key(c, a),
            ]
        })
        .filter(|(a, b)| *a < vertex_count && *b < vertex_count && a != b)
        .collect()
}

fn selection_complement<T: Ord + Copy>(
    universe: &BTreeSet<T>,
    selected: &BTreeSet<T>,
) -> BTreeSet<T> {
    universe.difference(selected).copied().collect()
}

fn input_col_generated_face(col: &EditingColState, face: usize) -> bool {
    !col.editing_shadow
        && (col.capsules.iter().any(|capsule| {
            face >= capsule.face_start && face < capsule.face_start + capsule.face_count
        }) || col.cuboids.iter().any(|cuboid| {
            face >= cuboid.face_start && face < cuboid.face_start + cuboid.face_count
        }))
}

fn input_col_generated_vertex(col: &EditingColState, vertex: usize) -> bool {
    !col.editing_shadow
        && (col.capsules.iter().any(|capsule| {
            vertex >= capsule.vertex_start && vertex < capsule.vertex_start + capsule.vertex_count
        }) || col.cuboids.iter().any(|cuboid| {
            vertex >= cuboid.vertex_start && vertex < cuboid.vertex_start + cuboid.vertex_count
        }))
}

fn input_dff_edges(dff: &EditingDffState) -> BTreeSet<(usize, usize)> {
    topology_edges(
        dff.raw
            .triangles
            .iter()
            .map(|tri| [tri.a as usize, tri.b as usize, tri.c as usize]),
        dff.raw.vertices.len(),
    )
}

fn input_col_edges(col: &EditingColState) -> BTreeSet<(usize, usize)> {
    topology_edges(
        col.mesh
            .faces
            .iter()
            .enumerate()
            .filter(|(face, _)| !input_col_generated_face(col, *face))
            .map(|(_, face)| [face.a as usize, face.b as usize, face.c as usize]),
        col.mesh.vertices.len(),
    )
    .into_iter()
    .filter(|(a, b)| !input_col_generated_vertex(col, *a) && !input_col_generated_vertex(col, *b))
    .collect()
}

fn input_dff_face_for_vertex(dff: &EditingDffState, vertex: usize) -> Option<usize> {
    dff.raw
        .triangles
        .iter()
        .position(|tri| [tri.a as usize, tri.b as usize, tri.c as usize].contains(&vertex))
}

fn input_col_face_slot_for_vertex(col: &EditingColState, vertex: usize) -> Option<(usize, usize)> {
    col.mesh
        .faces
        .iter()
        .enumerate()
        .filter(|(face, _)| !input_col_generated_face(col, *face))
        .find_map(|(face_index, face)| {
            [face.a as usize, face.b as usize, face.c as usize]
                .iter()
                .position(|index| *index == vertex)
                .map(|slot| (face_index, slot))
        })
}

fn set_input_dff_vertex_selection(dff: &mut EditingDffState, selected: BTreeSet<usize>) {
    dff.selected_faces.clear();
    dff.selected_edges.clear();
    dff.selected_vertices = selected;
    dff.selected_vertex = dff.selected_vertices.iter().next_back().copied();
    dff.selected_face = dff
        .selected_vertex
        .and_then(|vertex| input_dff_face_for_vertex(dff, vertex));
    if let Some(material) = dff
        .selected_face
        .and_then(|face| dff.raw.triangles.get(face))
        .map(|tri| tri.material as usize)
    {
        dff.selected_material = material;
    }
}

fn set_input_dff_edge_selection(dff: &mut EditingDffState, selected: BTreeSet<(usize, usize)>) {
    dff.selected_face = None;
    dff.selected_faces.clear();
    dff.selected_edges = selected;
    dff.selected_vertex = None;
    dff.selected_vertices.clear();
}

fn set_input_dff_face_selection(dff: &mut EditingDffState, selected: BTreeSet<usize>) {
    dff.selected_faces = selected;
    dff.selected_face = dff.selected_faces.iter().next_back().copied();
    dff.selected_edges.clear();
    dff.selected_vertex = None;
    dff.selected_vertices.clear();
    if let Some(material) = dff
        .selected_face
        .and_then(|face| dff.raw.triangles.get(face))
        .map(|tri| tri.material as usize)
    {
        dff.selected_material = material;
    }
}

fn set_input_col_vertex_selection(col: &mut EditingColState, selected: BTreeSet<usize>) {
    col.selected_faces.clear();
    col.selected_edges.clear();
    col.selected_vertices = selected;
    col.selected_primitive = None;
    if let Some((face, slot)) = col
        .selected_vertices
        .iter()
        .rev()
        .find_map(|vertex| input_col_face_slot_for_vertex(col, *vertex))
    {
        col.selected_face = face;
        col.selected_vertex = slot;
    } else {
        col.selected_face = usize::MAX;
        col.selected_vertex = 0;
    }
}

fn set_input_col_edge_selection(col: &mut EditingColState, selected: BTreeSet<(usize, usize)>) {
    col.selected_face = usize::MAX;
    col.selected_vertex = 0;
    col.selected_faces.clear();
    col.selected_edges = selected;
    col.selected_vertices.clear();
    col.selected_primitive = None;
}

fn set_input_col_face_selection(col: &mut EditingColState, selected: BTreeSet<usize>) {
    col.selected_faces = selected;
    col.selected_face = col
        .selected_faces
        .iter()
        .next_back()
        .copied()
        .unwrap_or(usize::MAX);
    col.selected_vertex = 0;
    col.selected_edges.clear();
    col.selected_vertices.clear();
    col.selected_primitive = None;
}

fn set_editing_selection_mode(app: &mut AppState, mode: EditingSelectMode) -> bool {
    match app.editing.asset.as_mut() {
        Some(EditingAsset::Dff(dff)) => {
            let mode_was_active = dff.select_mode == mode;
            dff.select_mode = mode;
            match mode {
                EditingSelectMode::Vertex => {
                    let mut selected = if mode_was_active {
                        dff.selected_vertices
                            .iter()
                            .copied()
                            .filter(|vertex| *vertex < dff.raw.vertices.len())
                            .collect()
                    } else {
                        BTreeSet::new()
                    };
                    if mode_was_active
                        && let Some(vertex) = dff
                            .selected_vertex
                            .filter(|vertex| *vertex < dff.raw.vertices.len())
                    {
                        selected.insert(vertex);
                    }
                    set_input_dff_vertex_selection(dff, selected);
                }
                EditingSelectMode::Edge => {
                    let universe = input_dff_edges(dff);
                    let selected = if mode_was_active {
                        dff.selected_edges
                            .iter()
                            .copied()
                            .filter(|edge| universe.contains(edge))
                            .collect()
                    } else {
                        BTreeSet::new()
                    };
                    set_input_dff_edge_selection(dff, selected);
                }
                EditingSelectMode::Face => {
                    let mut selected = if mode_was_active {
                        dff.selected_faces
                            .iter()
                            .copied()
                            .filter(|face| *face < dff.raw.triangles.len())
                            .collect()
                    } else {
                        BTreeSet::new()
                    };
                    if mode_was_active
                        && let Some(face) = dff
                            .selected_face
                            .filter(|face| *face < dff.raw.triangles.len())
                    {
                        selected.insert(face);
                    }
                    set_input_dff_face_selection(dff, selected);
                }
            }
            sync_active_open_dff_model(dff);
        }
        Some(EditingAsset::Col(col)) => {
            let mode_was_active = col.select_mode == mode;
            col.select_mode = mode;
            match mode {
                EditingSelectMode::Vertex => {
                    let selected = if mode_was_active {
                        col.selected_vertices
                            .iter()
                            .copied()
                            .filter(|vertex| {
                                *vertex < col.mesh.vertices.len()
                                    && !input_col_generated_vertex(col, *vertex)
                            })
                            .collect()
                    } else {
                        BTreeSet::new()
                    };
                    set_input_col_vertex_selection(col, selected);
                }
                EditingSelectMode::Edge => {
                    let universe = input_col_edges(col);
                    let selected = if mode_was_active {
                        col.selected_edges
                            .iter()
                            .copied()
                            .filter(|edge| universe.contains(edge))
                            .collect()
                    } else {
                        BTreeSet::new()
                    };
                    set_input_col_edge_selection(col, selected);
                }
                EditingSelectMode::Face => {
                    let universe = (0..col.mesh.faces.len())
                        .filter(|face| !input_col_generated_face(col, *face))
                        .collect::<BTreeSet<_>>();
                    let mut selected = if mode_was_active {
                        col.selected_faces
                            .iter()
                            .copied()
                            .filter(|face| universe.contains(face))
                            .collect()
                    } else {
                        BTreeSet::new()
                    };
                    if mode_was_active && universe.contains(&col.selected_face) {
                        selected.insert(col.selected_face);
                    }
                    set_input_col_face_selection(col, selected);
                }
            }
        }
        _ => return false,
    }
    app.status_message = format!(
        "Editing selection mode: {}",
        editing_selection_mode_label(mode)
    );
    true
}

fn apply_editing_selection_command(app: &mut AppState, command: EditingSelectionCommand) -> bool {
    let (asset_label, mode, count) = match app.editing.asset.as_mut() {
        Some(EditingAsset::Dff(dff)) => {
            let mode = dff.select_mode;
            let count = match mode {
                EditingSelectMode::Vertex => {
                    let universe = (0..dff.raw.vertices.len()).collect::<BTreeSet<_>>();
                    let mut current = dff
                        .selected_vertices
                        .iter()
                        .copied()
                        .filter(|vertex| *vertex < dff.raw.vertices.len())
                        .collect::<BTreeSet<_>>();
                    if let Some(vertex) = dff
                        .selected_vertex
                        .filter(|vertex| *vertex < dff.raw.vertices.len())
                    {
                        current.insert(vertex);
                    }
                    let selected = match command {
                        EditingSelectionCommand::SelectAll => universe,
                        EditingSelectionCommand::DeselectAll => BTreeSet::new(),
                        EditingSelectionCommand::Invert => {
                            selection_complement(&universe, &current)
                        }
                    };
                    let count = selected.len();
                    set_input_dff_vertex_selection(dff, selected);
                    count
                }
                EditingSelectMode::Edge => {
                    let universe = input_dff_edges(dff);
                    let current = dff
                        .selected_edges
                        .iter()
                        .copied()
                        .filter(|edge| universe.contains(edge))
                        .collect::<BTreeSet<_>>();
                    let selected = match command {
                        EditingSelectionCommand::SelectAll => universe,
                        EditingSelectionCommand::DeselectAll => BTreeSet::new(),
                        EditingSelectionCommand::Invert => {
                            selection_complement(&universe, &current)
                        }
                    };
                    let count = selected.len();
                    set_input_dff_edge_selection(dff, selected);
                    count
                }
                EditingSelectMode::Face => {
                    let universe = (0..dff.raw.triangles.len()).collect::<BTreeSet<_>>();
                    let mut current = dff
                        .selected_faces
                        .iter()
                        .copied()
                        .filter(|face| *face < dff.raw.triangles.len())
                        .collect::<BTreeSet<_>>();
                    if let Some(face) = dff
                        .selected_face
                        .filter(|face| *face < dff.raw.triangles.len())
                    {
                        current.insert(face);
                    }
                    let selected = match command {
                        EditingSelectionCommand::SelectAll => universe,
                        EditingSelectionCommand::DeselectAll => BTreeSet::new(),
                        EditingSelectionCommand::Invert => {
                            selection_complement(&universe, &current)
                        }
                    };
                    let count = selected.len();
                    set_input_dff_face_selection(dff, selected);
                    count
                }
            };
            ("DFF", mode, count)
        }
        Some(EditingAsset::Col(col)) => {
            let mode = col.select_mode;
            let count = match mode {
                EditingSelectMode::Vertex => {
                    let universe = (0..col.mesh.vertices.len())
                        .filter(|vertex| !input_col_generated_vertex(col, *vertex))
                        .collect::<BTreeSet<_>>();
                    let current = col
                        .selected_vertices
                        .iter()
                        .copied()
                        .filter(|vertex| universe.contains(vertex))
                        .collect::<BTreeSet<_>>();
                    let selected = match command {
                        EditingSelectionCommand::SelectAll => universe,
                        EditingSelectionCommand::DeselectAll => BTreeSet::new(),
                        EditingSelectionCommand::Invert => {
                            selection_complement(&universe, &current)
                        }
                    };
                    let count = selected.len();
                    set_input_col_vertex_selection(col, selected);
                    count
                }
                EditingSelectMode::Edge => {
                    let universe = input_col_edges(col);
                    let current = col
                        .selected_edges
                        .iter()
                        .copied()
                        .filter(|edge| universe.contains(edge))
                        .collect::<BTreeSet<_>>();
                    let selected = match command {
                        EditingSelectionCommand::SelectAll => universe,
                        EditingSelectionCommand::DeselectAll => BTreeSet::new(),
                        EditingSelectionCommand::Invert => {
                            selection_complement(&universe, &current)
                        }
                    };
                    let count = selected.len();
                    set_input_col_edge_selection(col, selected);
                    count
                }
                EditingSelectMode::Face => {
                    let universe = (0..col.mesh.faces.len())
                        .filter(|face| !input_col_generated_face(col, *face))
                        .collect::<BTreeSet<_>>();
                    let mut current = col
                        .selected_faces
                        .iter()
                        .copied()
                        .filter(|face| universe.contains(face))
                        .collect::<BTreeSet<_>>();
                    if universe.contains(&col.selected_face) {
                        current.insert(col.selected_face);
                    }
                    let selected = match command {
                        EditingSelectionCommand::SelectAll => universe,
                        EditingSelectionCommand::DeselectAll => BTreeSet::new(),
                        EditingSelectionCommand::Invert => {
                            selection_complement(&universe, &current)
                        }
                    };
                    let count = selected.len();
                    set_input_col_face_selection(col, selected);
                    count
                }
            };
            ("COL", mode, count)
        }
        _ => return false,
    };
    let action = match command {
        EditingSelectionCommand::SelectAll => "Selected all",
        EditingSelectionCommand::DeselectAll => "Deselected all",
        EditingSelectionCommand::Invert => "Inverted",
    };
    app.status_message = format!(
        "{action} {asset_label} {} selection: {count} selected",
        editing_selection_mode_label(mode)
    );
    true
}

fn editing_mesh_shortcuts_blocked_by_text_input(app: &AppState) -> bool {
    if app.editing.search_active {
        return true;
    }
    matches!(
        app.editing.asset.as_ref(),
        Some(EditingAsset::Dff(dff))
            if dff.uv_editor.open
                || dff.texture_picker_open
                || dff.collision_material_picker_open
                || dff.uv_anim_picker_open
                || dff.dff_2dfx_type_picker_open
                || dff.dff_2dfx_corona_preset_picker_open
                || dff.dff_2dfx_payload_editor_open
    )
}

fn handle_editing_mesh_selection_shortcuts(
    app: &mut AppState,
    ctrl_down: bool,
    shift_down: bool,
    alt_down: bool,
) -> bool {
    if app.active_tab != AppTab::Editing
        || !matches!(
            app.editing.asset.as_ref(),
            Some(EditingAsset::Dff(_) | EditingAsset::Col(_))
        )
        || editing_mesh_shortcuts_blocked_by_text_input(app)
    {
        return false;
    }
    if alt_down && !ctrl_down && !shift_down && is_key_pressed(KeyCode::D) {
        if app.editing.linked_selection_job.is_some() {
            app.status_message =
                "Wait for linked-island selection to finish before duplicating".to_string();
            return true;
        }
        let before = editing_history_snapshot(app);
        if editing_duplicate_selected_mesh(app) {
            commit_editing_history(app, "Duplicate Mesh Selection", before);
        }
        return true;
    }
    let mode = if !ctrl_down && !shift_down && !alt_down && is_key_pressed(KeyCode::Key1) {
        Some(EditingSelectMode::Vertex)
    } else if !ctrl_down && !shift_down && !alt_down && is_key_pressed(KeyCode::Key2) {
        Some(EditingSelectMode::Edge)
    } else if !ctrl_down && !shift_down && !alt_down && is_key_pressed(KeyCode::Key3) {
        Some(EditingSelectMode::Face)
    } else {
        None
    };
    let command = if ctrl_down && shift_down && !alt_down && is_key_pressed(KeyCode::A) {
        Some(EditingSelectionCommand::DeselectAll)
    } else if ctrl_down && !shift_down && !alt_down && is_key_pressed(KeyCode::A) {
        Some(EditingSelectionCommand::SelectAll)
    } else if ctrl_down && !shift_down && !alt_down && is_key_pressed(KeyCode::I) {
        Some(EditingSelectionCommand::Invert)
    } else {
        None
    };
    if mode.is_none() && command.is_none() {
        return false;
    }
    if app.editing.linked_selection_job.is_some() {
        app.status_message =
            "Wait for linked-island selection to finish before changing selection".to_string();
        return true;
    }
    if let Some(mode) = mode {
        set_editing_selection_mode(app, mode)
    } else {
        apply_editing_selection_command(app, command.expect("command checked above"))
    }
}

// Apply an in-progress COL box face drag: move only the dragged face, keeping the
// opposite face fixed; respect Move Snap when Snap Mode is enabled.
fn apply_col_box_face_drag(app: &mut AppState, viewport: Rect, mouse: Vec2) {
    let Some(drag) = app.col_box_face_drag.as_ref() else {
        return;
    };
    let axis = drag.axis;
    let side_is_max = drag.side_is_max;
    let start_mouse = drag.start_mouse;
    let primitive = drag.primitive;
    let center = drag.center;
    let mut half_extents = drag.half_extents;
    let axis_dir = drag.axis_dir;
    let face_origin = drag.face_origin;

    let amount = axis_drag_amount(app, viewport, face_origin, axis_dir, start_mouse, mouse);
    let mut amount = if app.snap_enabled {
        snap_delta(amount, app.snap_move)
    } else {
        amount
    };

    let min_thickness = 0.01;
    if side_is_max {
        amount = amount.max(-half_extents[axis] * 2.0 + min_thickness);
    } else {
        amount = amount.min(half_extents[axis] * 2.0 - min_thickness);
    }
    let side_sign = if side_is_max { 1.0 } else { -1.0 };
    half_extents[axis] = (half_extents[axis] + side_sign * amount * 0.5).max(0.005);
    let center = center + axis_dir * (amount * 0.5);
    if let Err(err) =
        resize_editing_col_box_primitive(app, primitive, from_mq(center), from_mq(half_extents))
    {
        app.status_message = err;
    }
}

// COL inspector fields live inside the scrollable COL panel; fields that are
// collapsed or scrolled out of view report an off-screen rect so they can't be
// drawn or clicked.
fn col_layout_field_rect(app: &AppState, pick: impl Fn(&ColPanelLayout) -> Option<Rect>) -> Rect {
    let offscreen = Rect::new(-10000.0, -10000.0, 0.0, 0.0);
    match app.editing.asset.as_ref() {
        Some(EditingAsset::Col(col)) => pick(&col_panel_layout(col)).unwrap_or(offscreen),
        _ => offscreen,
    }
}

fn dff_layout_field_rect(app: &AppState, pick: impl Fn(&DffPanelLayout) -> Option<Rect>) -> Rect {
    let offscreen = Rect::new(-10000.0, -10000.0, 0.0, 0.0);
    let emitter = selected_material_emitter(app);
    match app.editing.asset.as_ref() {
        Some(EditingAsset::Dff(dff)) => pick(&dff_panel_layout(
            dff,
            emitter,
            dff_face_emitter_entry_count(app, &dff.name),
        ))
        .unwrap_or(offscreen),
        _ => offscreen,
    }
}

// Element-tab inspector fields live inside the scrollable properties panel;
// fields in collapsed sections report an off-screen rect so they can't be
// drawn or clicked.
fn element_layout_field_rect(
    app: &AppState,
    pick: impl Fn(&ElementPanelLayout) -> Option<Rect>,
) -> Rect {
    pick(&element_panel_layout(app)).unwrap_or(Rect::new(-10000.0, -10000.0, 0.0, 0.0))
}

fn settings_layout_field_rect(
    app: &AppState,
    pick: impl Fn(&SettingsPanelLayout) -> Option<Rect>,
) -> Rect {
    pick(&settings_panel_layout(app)).unwrap_or(Rect::new(-10000.0, -10000.0, 0.0, 0.0))
}

pub(crate) fn inspector_field_rect(app: &AppState, field: InspectorField) -> Rect {
    let x = screen_width() - RIGHT_PANEL_W + 18.0;
    match field {
        InspectorField::ElementId => element_layout_field_rect(app, |layout| layout.id),
        InspectorField::ElementPosX => {
            element_layout_field_rect(app, |layout| layout.pos.map(|rects| rects[0]))
        }
        InspectorField::ElementPosY => {
            element_layout_field_rect(app, |layout| layout.pos.map(|rects| rects[1]))
        }
        InspectorField::ElementPosZ => {
            element_layout_field_rect(app, |layout| layout.pos.map(|rects| rects[2]))
        }
        InspectorField::ElementRotX => {
            element_layout_field_rect(app, |layout| layout.rot.map(|rects| rects[0]))
        }
        InspectorField::ElementRotY => {
            element_layout_field_rect(app, |layout| layout.rot.map(|rects| rects[1]))
        }
        InspectorField::ElementRotZ => {
            element_layout_field_rect(app, |layout| layout.rot.map(|rects| rects[2]))
        }
        InspectorField::ElementDimension => element_layout_field_rect(app, |layout| layout.dim),
        InspectorField::ElementInterior => element_layout_field_rect(app, |layout| layout.interior),
        InspectorField::ElementLodParent => {
            element_layout_field_rect(app, |layout| layout.lod_parent)
        }
        InspectorField::ElementUniqueId => {
            element_layout_field_rect(app, |layout| layout.unique_id)
        }
        InspectorField::ElementAlpha => element_layout_field_rect(app, |layout| layout.alpha),
        InspectorField::ElementScale => element_layout_field_rect(app, |layout| layout.scale),
        InspectorField::PhysicsSimulated => {
            element_layout_field_rect(app, |layout| layout.physics_simulated)
        }
        InspectorField::PhysicsMass => element_layout_field_rect(app, |layout| layout.physics_mass),
        InspectorField::PhysicsTurnMass => {
            element_layout_field_rect(app, |layout| layout.physics_turn_mass)
        }
        InspectorField::PhysicsAirResistance => {
            element_layout_field_rect(app, |layout| layout.physics_air_resistance)
        }
        InspectorField::PhysicsElasticity => {
            element_layout_field_rect(app, |layout| layout.physics_elasticity)
        }
        InspectorField::PhysicsBuoyancy => {
            element_layout_field_rect(app, |layout| layout.physics_buoyancy)
        }
        InspectorField::PhysicsCenterOfMassX => element_layout_field_rect(app, |layout| {
            layout.physics_center_of_mass.map(|rects| rects[0])
        }),
        InspectorField::PhysicsCenterOfMassY => element_layout_field_rect(app, |layout| {
            layout.physics_center_of_mass.map(|rects| rects[1])
        }),
        InspectorField::PhysicsCenterOfMassZ => element_layout_field_rect(app, |layout| {
            layout.physics_center_of_mass.map(|rects| rects[2])
        }),
        InspectorField::DefinitionDff => element_layout_field_rect(app, |layout| layout.dff),
        InspectorField::DefinitionNativeModel => {
            element_layout_field_rect(app, |layout| layout.native_model)
        }
        InspectorField::DefinitionTxd => element_layout_field_rect(app, |layout| layout.txd),
        InspectorField::DefinitionCol => element_layout_field_rect(app, |layout| layout.col),
        InspectorField::DefinitionLod => element_layout_field_rect(app, |layout| layout.lod),
        InspectorField::DefinitionTimeIn => element_layout_field_rect(app, |layout| layout.time_in),
        InspectorField::DefinitionTimeOut => {
            element_layout_field_rect(app, |layout| layout.time_out)
        }
        InspectorField::SnapMove => settings_layout_field_rect(app, |layout| layout.snap_move),
        InspectorField::SnapRotate => settings_layout_field_rect(app, |layout| layout.snap_rotate),
        InspectorField::GlobalOffsetX => {
            settings_layout_field_rect(app, |l| l.global_offset.map(|r| r[0]))
        }
        InspectorField::GlobalOffsetY => {
            settings_layout_field_rect(app, |l| l.global_offset.map(|r| r[1]))
        }
        InspectorField::GlobalOffsetZ => {
            settings_layout_field_rect(app, |l| l.global_offset.map(|r| r[2]))
        }
        InspectorField::EagleOffsetX => {
            settings_layout_field_rect(app, |l| l.eagle_offset.map(|r| r[0]))
        }
        InspectorField::EagleOffsetY => {
            settings_layout_field_rect(app, |l| l.eagle_offset.map(|r| r[1]))
        }
        InspectorField::EagleOffsetZ => {
            settings_layout_field_rect(app, |l| l.eagle_offset.map(|r| r[2]))
        }
        InspectorField::EagleWaterOffsetX => {
            settings_layout_field_rect(app, |l| l.eagle_water_offset.map(|r| r[0]))
        }
        InspectorField::EagleWaterOffsetY => {
            settings_layout_field_rect(app, |l| l.eagle_water_offset.map(|r| r[1]))
        }
        InspectorField::EagleWaterOffsetZ => {
            settings_layout_field_rect(app, |l| l.eagle_water_offset.map(|r| r[2]))
        }
        InspectorField::GlobalRotationX => {
            settings_layout_field_rect(app, |l| l.global_rotation.map(|r| r[0]))
        }
        InspectorField::GlobalRotationY => {
            settings_layout_field_rect(app, |l| l.global_rotation.map(|r| r[1]))
        }
        InspectorField::GlobalRotationZ => {
            settings_layout_field_rect(app, |l| l.global_rotation.map(|r| r[2]))
        }
        InspectorField::CollisionFaceMaterial => {
            if app.active_tab == AppTab::Editing {
                col_layout_field_rect(app, |layout| layout.material)
            } else {
                Rect::new(x, TOP_H + 318.0 - app.properties_scroll, 150.0, 30.0)
            }
        }
        InspectorField::CollisionFaceLight => {
            if app.active_tab == AppTab::Editing {
                col_layout_field_rect(app, |layout| layout.light)
            } else {
                Rect::new(
                    x + 168.0,
                    TOP_H + 318.0 - app.properties_scroll,
                    150.0,
                    30.0,
                )
            }
        }
        InspectorField::CollisionVertexX => {
            if app.active_tab == AppTab::Editing {
                col_layout_field_rect(app, |layout| layout.vertex.map(|rects| rects[0]))
            } else {
                Rect::new(x, TOP_H + 384.0 - app.properties_scroll, 94.0, 30.0)
            }
        }
        InspectorField::CollisionVertexY => {
            if app.active_tab == AppTab::Editing {
                col_layout_field_rect(app, |layout| layout.vertex.map(|rects| rects[1]))
            } else {
                Rect::new(x + 112.0, TOP_H + 384.0 - app.properties_scroll, 94.0, 30.0)
            }
        }
        InspectorField::CollisionVertexZ => {
            if app.active_tab == AppTab::Editing {
                col_layout_field_rect(app, |layout| layout.vertex.map(|rects| rects[2]))
            } else {
                Rect::new(x + 224.0, TOP_H + 384.0 - app.properties_scroll, 94.0, 30.0)
            }
        }
        InspectorField::CollisionPrimitiveSizeX => {
            col_layout_field_rect(app, |layout| layout.prim_size.map(|rects| rects[0]))
        }
        InspectorField::CollisionPrimitiveSizeY => {
            col_layout_field_rect(app, |layout| layout.prim_size.map(|rects| rects[1]))
        }
        InspectorField::CollisionPrimitiveSizeZ => {
            col_layout_field_rect(app, |layout| layout.prim_size.map(|rects| rects[2]))
        }
        InspectorField::CollisionPrimitiveRotX => {
            col_layout_field_rect(app, |layout| layout.prim_rotation.map(|rects| rects[0]))
        }
        InspectorField::CollisionPrimitiveRotY => {
            col_layout_field_rect(app, |layout| layout.prim_rotation.map(|rects| rects[1]))
        }
        InspectorField::CollisionPrimitiveRotZ => {
            col_layout_field_rect(app, |layout| layout.prim_rotation.map(|rects| rects[2]))
        }
        InspectorField::DffMaterialRed => {
            dff_layout_field_rect(app, |layout| layout.material_color.map(|rects| rects[0]))
        }
        InspectorField::DffMaterialGreen => {
            dff_layout_field_rect(app, |layout| layout.material_color.map(|rects| rects[1]))
        }
        InspectorField::DffMaterialBlue => {
            dff_layout_field_rect(app, |layout| layout.material_color.map(|rects| rects[2]))
        }
        InspectorField::DffMaterialAlpha => {
            dff_layout_field_rect(app, |layout| layout.material_color.map(|rects| rects[3]))
        }
        InspectorField::DffMaterialAmbient => {
            dff_layout_field_rect(app, |layout| layout.material_surface.map(|rects| rects[0]))
        }
        InspectorField::DffMaterialDiffuse => {
            dff_layout_field_rect(app, |layout| layout.material_surface.map(|rects| rects[1]))
        }
        InspectorField::DffMaterialSpecular => {
            dff_layout_field_rect(app, |layout| layout.material_surface.map(|rects| rects[2]))
        }
        InspectorField::DffEmitterStrength => {
            dff_layout_field_rect(app, |layout| layout.emitter_strength)
        }
        InspectorField::DffEmitterFalloff => {
            dff_layout_field_rect(app, |layout| layout.emitter_falloff)
        }
        InspectorField::DffEmitterMaxGroupingSize => {
            dff_layout_field_rect(app, |layout| layout.emitter_max_grouping_size)
        }
        InspectorField::DffEmitterPointUpStrength => {
            dff_layout_field_rect(app, |layout| layout.emitter_point_up_strength)
        }
        InspectorField::DffEmitterPointDownStrength => {
            dff_layout_field_rect(app, |layout| layout.emitter_point_down_strength)
        }
        InspectorField::DffEmitterPointSidesStrength => {
            dff_layout_field_rect(app, |layout| layout.emitter_point_sides_strength)
        }
        InspectorField::DffEmitterTemperature => {
            dff_layout_field_rect(app, |layout| layout.emitter_temperature)
        }
        InspectorField::BakeShadowSamples => bake_panel_layout(app).shadow_samples,
        InspectorField::BakeShadowChunks => bake_panel_layout(app).shadow_chunks,
        InspectorField::BakeBounces => bake_panel_layout(app).bounces,
        InspectorField::BakeBounceStrength => bake_panel_layout(app).bounce_strength,
        InspectorField::BakeBounceMaximum => bake_panel_layout(app).bounce_maximum,
        InspectorField::BakeExposure => bake_panel_layout(app).exposure,
        InspectorField::BakeAmbientBump => bake_panel_layout(app).ambient_bump,
        InspectorField::BakeShadowSoftness => bake_panel_layout(app).shadow_softness,
        InspectorField::BakeAoSamples => bake_panel_layout(app).ao_samples,
        InspectorField::BakeAoRadius => bake_panel_layout(app).ao_radius,
        InspectorField::BakeAoStrength => bake_panel_layout(app).ao_strength,
        InspectorField::DayNightMergeTolerance => bake_panel_layout(app).variant_tolerance,
        InspectorField::VertexPaintTemperature => bake_panel_layout(app).paint_temperature,
        InspectorField::VertexPaintStrength => bake_panel_layout(app).paint_strength,
        InspectorField::VertexPaintRadius => bake_panel_layout(app).paint_radius,
        InspectorField::LightName => {
            Rect::new(x, TOP_H + 348.0 - app.properties_scroll, 150.0, 30.0)
        }
        InspectorField::LightKind => Rect::new(
            x + 168.0,
            TOP_H + 348.0 - app.properties_scroll,
            150.0,
            30.0,
        ),
        InspectorField::LightProfile => {
            Rect::new(x, TOP_H + 414.0 - app.properties_scroll, 150.0, 30.0)
        }
        InspectorField::LightIntensity => Rect::new(
            x + 168.0,
            TOP_H + 414.0 - app.properties_scroll,
            150.0,
            30.0,
        ),
        InspectorField::LightPosition => {
            Rect::new(x, TOP_H + 498.0 - app.properties_scroll, 318.0, 30.0)
        }
        InspectorField::LightDirection => {
            Rect::new(x, TOP_H + 580.0 - app.properties_scroll, 318.0, 30.0)
        }
        InspectorField::LightTemperature => {
            Rect::new(x, TOP_H + 662.0 - app.properties_scroll, 150.0, 30.0)
        }
        InspectorField::LightRadius => Rect::new(
            x + 168.0,
            TOP_H + 662.0 - app.properties_scroll,
            150.0,
            30.0,
        ),
        InspectorField::WaterMinX => {
            Rect::new(x, TOP_H + 600.0 - app.properties_scroll, 150.0, 30.0)
        }
        InspectorField::WaterMinY => Rect::new(
            x + 168.0,
            TOP_H + 600.0 - app.properties_scroll,
            150.0,
            30.0,
        ),
        InspectorField::WaterMaxX => {
            Rect::new(x, TOP_H + 666.0 - app.properties_scroll, 150.0, 30.0)
        }
        InspectorField::WaterMaxY => Rect::new(
            x + 168.0,
            TOP_H + 666.0 - app.properties_scroll,
            150.0,
            30.0,
        ),
        InspectorField::WaterHeight => {
            Rect::new(x, TOP_H + 732.0 - app.properties_scroll, 150.0, 30.0)
        }
        InspectorField::WaterType => Rect::new(
            x + 168.0,
            TOP_H + 732.0 - app.properties_scroll,
            150.0,
            30.0,
        ),
        InspectorField::CullPosX => {
            Rect::new(x, TOP_H + 510.0 - app.properties_scroll, 150.0, 30.0)
        }
        InspectorField::CullPosY => Rect::new(
            x + 168.0,
            TOP_H + 510.0 - app.properties_scroll,
            150.0,
            30.0,
        ),
        InspectorField::CullPosZ => {
            Rect::new(x, TOP_H + 576.0 - app.properties_scroll, 150.0, 30.0)
        }
        InspectorField::CullSizeX => Rect::new(
            x + 168.0,
            TOP_H + 576.0 - app.properties_scroll,
            150.0,
            30.0,
        ),
        InspectorField::CullSizeY => {
            Rect::new(x, TOP_H + 642.0 - app.properties_scroll, 150.0, 30.0)
        }
        InspectorField::CullSizeZ => Rect::new(
            x + 168.0,
            TOP_H + 642.0 - app.properties_scroll,
            150.0,
            30.0,
        ),
    }
}

pub(crate) fn inspector_copy_button_rect(app: &AppState, action: InspectorCopyAction) -> Rect {
    let x = screen_width() - RIGHT_PANEL_W + 18.0;
    match action {
        InspectorCopyAction::Field(field) => {
            let anchor = inspector_field_rect(app, field);
            Rect::new(anchor.x + anchor.w - 27.0, anchor.y + 4.0, 22.0, 22.0)
        }
        InspectorCopyAction::ElementPosition => {
            element_layout_field_rect(app, |layout| layout.copy_position)
        }
        InspectorCopyAction::ElementRotation => {
            element_layout_field_rect(app, |layout| layout.copy_rotation)
        }
        InspectorCopyAction::LightPosition => {
            let anchor = inspector_field_rect(app, InspectorField::LightPosition);
            Rect::new(x, anchor.y - 58.0, 150.0, 28.0)
        }
        InspectorCopyAction::LightRotation => {
            let anchor = inspector_field_rect(app, InspectorField::LightPosition);
            Rect::new(x + 168.0, anchor.y - 58.0, 150.0, 28.0)
        }
    }
}

pub(crate) fn inspector_copy_actions(app: &AppState) -> Vec<InspectorCopyAction> {
    if app.active_tab == AppTab::Lights {
        if app.lights.get(app.selected_light).is_some() {
            return vec![
                InspectorCopyAction::LightPosition,
                InspectorCopyAction::LightRotation,
            ];
        }
        return Vec::new();
    }
    if app.active_tab == AppTab::Preview
        && app.properties_tab == PropertiesTab::Element
        && app.placements.get(app.selected).is_some()
    {
        return vec![
            InspectorCopyAction::Field(InspectorField::ElementId),
            InspectorCopyAction::Field(InspectorField::ElementUniqueId),
            InspectorCopyAction::Field(InspectorField::DefinitionDff),
            InspectorCopyAction::Field(InspectorField::DefinitionNativeModel),
            InspectorCopyAction::Field(InspectorField::DefinitionTxd),
            InspectorCopyAction::Field(InspectorField::DefinitionCol),
            InspectorCopyAction::Field(InspectorField::DefinitionLod),
            InspectorCopyAction::ElementPosition,
            InspectorCopyAction::ElementRotation,
        ];
    }
    Vec::new()
}

pub(crate) fn clicked_inspector_copy_action(
    app: &AppState,
    mouse: Vec2,
) -> Option<InspectorCopyAction> {
    if app.active_tab == AppTab::Preview
        && app.properties_tab == PropertiesTab::Element
        && !properties_content_rect().contains(mouse)
    {
        return None;
    }
    inspector_copy_actions(app)
        .into_iter()
        .find(|action| inspector_copy_button_rect(app, *action).contains(mouse))
}

pub(crate) fn inspector_fields(app: &AppState) -> Vec<InspectorField> {
    if app.active_tab == AppTab::Collisions {
        return Vec::new();
    }
    if app.active_tab == AppTab::Editing
        && matches!(app.editing.asset.as_ref(), Some(EditingAsset::Dff(_)))
    {
        // Material RGBA is edited with drag bars rather than text boxes, so the
        // four colour fields are deliberately absent here: listing them would
        // let a click on a bar open a text editor on top of it.
        let mut fields = vec![
            InspectorField::DffMaterialAmbient,
            InspectorField::DffMaterialDiffuse,
            InspectorField::DffMaterialSpecular,
        ];
        if selected_material_emitter(app).enabled {
            fields.extend([
                InspectorField::DffEmitterStrength,
                InspectorField::DffEmitterFalloff,
            ]);
            if selected_material_emitter(app).cast_mode == MaterialEmitterCastMode::Point {
                fields.push(InspectorField::DffEmitterMaxGroupingSize);
                fields.push(InspectorField::DffEmitterPointUpStrength);
                fields.push(InspectorField::DffEmitterPointDownStrength);
                fields.push(InspectorField::DffEmitterPointSidesStrength);
            }
        }
        return fields;
    }
    if app.active_tab == AppTab::Editing
        && matches!(
            app.editing.asset.as_ref(),
            Some(EditingAsset::Col(EditingColState {
                box_pick_enabled: true,
                ..
            }))
        )
    {
        let mut fields = vec![
            InspectorField::CollisionFaceMaterial,
            InspectorField::CollisionFaceLight,
            InspectorField::CollisionVertexX,
            InspectorField::CollisionVertexY,
            InspectorField::CollisionVertexZ,
        ];
        if let Some(EditingAsset::Col(col)) = app.editing.asset.as_ref() {
            if let Some(selected) = col.selected_primitive {
                match selected.kind {
                    CollisionPrimitiveKind::Sphere => {
                        fields.push(InspectorField::CollisionPrimitiveSizeX);
                    }
                    CollisionPrimitiveKind::Box => {
                        fields.extend([
                            InspectorField::CollisionPrimitiveSizeX,
                            InspectorField::CollisionPrimitiveSizeY,
                            InspectorField::CollisionPrimitiveSizeZ,
                            InspectorField::CollisionPrimitiveRotX,
                            InspectorField::CollisionPrimitiveRotY,
                            InspectorField::CollisionPrimitiveRotZ,
                        ]);
                    }
                    CollisionPrimitiveKind::Cuboid => {
                        fields.extend([
                            InspectorField::CollisionPrimitiveSizeX,
                            InspectorField::CollisionPrimitiveSizeY,
                            InspectorField::CollisionPrimitiveSizeZ,
                            InspectorField::CollisionPrimitiveRotX,
                            InspectorField::CollisionPrimitiveRotY,
                            InspectorField::CollisionPrimitiveRotZ,
                        ]);
                    }
                    CollisionPrimitiveKind::Capsule => {
                        fields.extend([
                            InspectorField::CollisionPrimitiveSizeX,
                            InspectorField::CollisionPrimitiveSizeZ,
                        ]);
                    }
                }
            }
        }
        return fields;
    }
    if app.active_tab == AppTab::Lights {
        return vec![
            InspectorField::LightName,
            InspectorField::LightIntensity,
            InspectorField::LightPosition,
            InspectorField::LightDirection,
            InspectorField::LightRadius,
        ];
    }
    if app.active_tab == AppTab::Bake {
        return vec![
            InspectorField::BakeShadowSamples,
            InspectorField::BakeShadowChunks,
            InspectorField::BakeBounces,
            InspectorField::BakeBounceStrength,
            InspectorField::BakeBounceMaximum,
            InspectorField::BakeExposure,
            InspectorField::BakeAmbientBump,
            InspectorField::BakeShadowSoftness,
            InspectorField::BakeAoSamples,
            InspectorField::BakeAoRadius,
            InspectorField::BakeAoStrength,
            InspectorField::DayNightMergeTolerance,
            InspectorField::VertexPaintTemperature,
            InspectorField::VertexPaintRadius,
            InspectorField::VertexPaintStrength,
        ];
    }
    if app.active_tab == AppTab::Water {
        return vec![
            InspectorField::WaterMinX,
            InspectorField::WaterMinY,
            InspectorField::WaterMaxX,
            InspectorField::WaterMaxY,
            InspectorField::WaterHeight,
            InspectorField::WaterType,
        ];
    }
    if app.active_tab == AppTab::Cull {
        return vec![
            InspectorField::CullPosX,
            InspectorField::CullPosY,
            InspectorField::CullPosZ,
            InspectorField::CullSizeX,
            InspectorField::CullSizeY,
            InspectorField::CullSizeZ,
        ];
    }
    if app.properties_tab == PropertiesTab::Settings {
        return vec![
            InspectorField::SnapMove,
            InspectorField::SnapRotate,
            InspectorField::GlobalOffsetX,
            InspectorField::GlobalOffsetY,
            InspectorField::GlobalOffsetZ,
            InspectorField::EagleOffsetX,
            InspectorField::EagleOffsetY,
            InspectorField::EagleOffsetZ,
            InspectorField::EagleWaterOffsetX,
            InspectorField::EagleWaterOffsetY,
            InspectorField::EagleWaterOffsetZ,
            InspectorField::GlobalRotationX,
            InspectorField::GlobalRotationY,
            InspectorField::GlobalRotationZ,
        ];
    }
    if app.properties_tab == PropertiesTab::History {
        return Vec::new();
    }
    let fields = vec![
        InspectorField::ElementId,
        InspectorField::ElementPosX,
        InspectorField::ElementPosY,
        InspectorField::ElementPosZ,
        InspectorField::ElementRotX,
        InspectorField::ElementRotY,
        InspectorField::ElementRotZ,
        InspectorField::ElementDimension,
        InspectorField::ElementInterior,
        InspectorField::ElementUniqueId,
        InspectorField::ElementAlpha,
        InspectorField::ElementScale,
        InspectorField::PhysicsMass,
        InspectorField::PhysicsTurnMass,
        InspectorField::PhysicsAirResistance,
        InspectorField::PhysicsElasticity,
        InspectorField::PhysicsBuoyancy,
        InspectorField::PhysicsCenterOfMassX,
        InspectorField::PhysicsCenterOfMassY,
        InspectorField::PhysicsCenterOfMassZ,
        InspectorField::DefinitionDff,
        InspectorField::DefinitionNativeModel,
        InspectorField::DefinitionTxd,
        InspectorField::DefinitionCol,
        InspectorField::DefinitionLod,
        InspectorField::DefinitionTimeIn,
        InspectorField::DefinitionTimeOut,
    ];
    fields
}

pub(crate) fn clicked_inspector_field(app: &AppState, mouse: Vec2) -> Option<InspectorField> {
    if app.active_tab == AppTab::Preview
        && app.properties_tab == PropertiesTab::Element
        && !properties_content_rect().contains(mouse)
    {
        return None;
    }
    if matches!(
        app.active_tab,
        AppTab::Collisions | AppTab::Lights | AppTab::Bake | AppTab::Water | AppTab::Cull
    ) && !inspector_panel_content_rect().contains(mouse)
    {
        return None;
    }
    inspector_fields(app)
        .into_iter()
        .find(|field| inspector_field_rect(app, *field).contains(mouse))
}

pub(crate) fn inspector_field_value(app: &AppState, field: InspectorField) -> String {
    if field == InspectorField::SnapMove {
        return format!("{:.3}", app.snap_move);
    }
    if field == InspectorField::SnapRotate {
        return format!("{:.3}", app.snap_rotate);
    }
    match field {
        InspectorField::GlobalOffsetX => return format!("{:.3}", app.global_transform.offset.x),
        InspectorField::GlobalOffsetY => return format!("{:.3}", app.global_transform.offset.y),
        InspectorField::GlobalOffsetZ => return format!("{:.3}", app.global_transform.offset.z),
        InspectorField::EagleOffsetX => {
            return format!("{:.3}", app.eagle_zone_offsets.offset.unwrap_or_default().x);
        }
        InspectorField::EagleOffsetY => {
            return format!("{:.3}", app.eagle_zone_offsets.offset.unwrap_or_default().y);
        }
        InspectorField::EagleOffsetZ => {
            return format!("{:.3}", app.eagle_zone_offsets.offset.unwrap_or_default().z);
        }
        InspectorField::EagleWaterOffsetX => {
            return format!(
                "{:.3}",
                app.eagle_zone_offsets.water_offset.unwrap_or_default().x
            );
        }
        InspectorField::EagleWaterOffsetY => {
            return format!(
                "{:.3}",
                app.eagle_zone_offsets.water_offset.unwrap_or_default().y
            );
        }
        InspectorField::EagleWaterOffsetZ => {
            return format!(
                "{:.3}",
                app.eagle_zone_offsets.water_offset.unwrap_or_default().z
            );
        }
        InspectorField::GlobalRotationX => {
            return format!("{:.3}", app.global_transform.rotation.x);
        }
        InspectorField::GlobalRotationY => {
            return format!("{:.3}", app.global_transform.rotation.y);
        }
        InspectorField::GlobalRotationZ => {
            return format!("{:.3}", app.global_transform.rotation.z);
        }
        _ => {}
    }
    if let Some(zone) = selected_cull_zone(app) {
        match field {
            InspectorField::CullPosX => return format!("{:.3}", zone.center.x),
            InspectorField::CullPosY => return format!("{:.3}", zone.center.y),
            InspectorField::CullPosZ => return format!("{:.3}", zone.center.z),
            InspectorField::CullSizeX => return format!("{:.3}", zone.size.x),
            InspectorField::CullSizeY => return format!("{:.3}", zone.size.y),
            InspectorField::CullSizeZ => return format!("{:.3}", zone.size.z),
            _ => {}
        }
    }
    if matches!(
        field,
        InspectorField::DffMaterialRed
            | InspectorField::DffMaterialGreen
            | InspectorField::DffMaterialBlue
            | InspectorField::DffMaterialAlpha
            | InspectorField::DffMaterialAmbient
            | InspectorField::DffMaterialDiffuse
            | InspectorField::DffMaterialSpecular
    ) {
        let material = match app.editing.asset.as_ref() {
            Some(EditingAsset::Dff(dff)) => dff
                .raw
                .materials
                .get(dff.selected_material)
                .copied()
                .unwrap_or_else(default_dff_material),
            _ => default_dff_material(),
        };
        return match field {
            InspectorField::DffMaterialRed => {
                format!("{:.0}", (material.color.x.clamp(0.0, 1.0) * 255.0).round())
            }
            InspectorField::DffMaterialGreen => {
                format!("{:.0}", (material.color.y.clamp(0.0, 1.0) * 255.0).round())
            }
            InspectorField::DffMaterialBlue => {
                format!("{:.0}", (material.color.z.clamp(0.0, 1.0) * 255.0).round())
            }
            InspectorField::DffMaterialAlpha => {
                format!("{:.0}", (material.alpha.clamp(0.0, 1.0) * 255.0).round())
            }
            InspectorField::DffMaterialAmbient => format!("{:.3}", material.ambient),
            InspectorField::DffMaterialDiffuse => format!("{:.3}", material.diffuse),
            InspectorField::DffMaterialSpecular => format!("{:.3}", material.specular),
            _ => unreachable!(),
        };
    }
    if field == InspectorField::DffEmitterStrength {
        return format!("{:.3}", selected_material_emitter(app).strength);
    }
    if field == InspectorField::DffEmitterFalloff {
        return format!("{:.1}", selected_material_emitter(app).falloff_distance);
    }
    if field == InspectorField::DffEmitterMaxGroupingSize {
        return format!("{:.2}", selected_material_emitter(app).max_grouping_size);
    }
    if field == InspectorField::DffEmitterPointUpStrength {
        return format!("{:.3}", selected_material_emitter(app).point_up_strength);
    }
    if field == InspectorField::DffEmitterPointDownStrength {
        return format!("{:.3}", selected_material_emitter(app).point_down_strength);
    }
    if field == InspectorField::DffEmitterPointSidesStrength {
        return format!("{:.3}", selected_material_emitter(app).point_sides_strength);
    }
    if field == InspectorField::DffEmitterTemperature {
        return format!("{:.0}", selected_material_emitter(app).temperature);
    }
    match field {
        InspectorField::BakeShadowSamples => return app.bake_settings.shadow_samples.to_string(),
        InspectorField::BakeShadowChunks => return app.bake_settings.shadow_chunks.to_string(),
        InspectorField::BakeBounces => return app.bake_settings.bounces.to_string(),
        InspectorField::BakeBounceStrength => {
            return format!("{:.2}", app.bake_settings.bounce_strength);
        }
        InspectorField::BakeBounceMaximum => {
            return format!("{:.2}", app.bake_settings.bounce_maximum);
        }
        InspectorField::BakeExposure => return format!("{:.2}", app.bake_settings.exposure),
        InspectorField::BakeAmbientBump => {
            return format!("{:.2}", app.bake_settings.ambient_bump);
        }
        InspectorField::BakeShadowSoftness => {
            return format!("{:.0}", app.bake_settings.shadow_softness);
        }
        InspectorField::BakeAoSamples => return app.bake_settings.ao_samples.to_string(),
        InspectorField::BakeAoRadius => return format!("{:.0}", app.bake_settings.ao_radius),
        InspectorField::BakeAoStrength => {
            return format!("{:.2}", app.bake_settings.ao_strength);
        }
        InspectorField::DayNightMergeTolerance => {
            return format!("{:.4}", app.bake_settings.day_night_merge_tolerance);
        }
        InspectorField::VertexPaintTemperature => {
            return format!("{:.0}", app.vertex_paint.temperature);
        }
        InspectorField::VertexPaintRadius => return format!("{:.0}", app.vertex_paint.radius),
        InspectorField::VertexPaintStrength => return format!("{:.2}", app.vertex_paint.strength),
        _ => {}
    }
    if let Some(light) = app.lights.get(app.selected_light) {
        match field {
            InspectorField::LightName => return light.name.clone(),
            InspectorField::LightKind => return light_kind_label(light.kind).to_string(),
            InspectorField::LightProfile => return light_profile_label(light.profile).to_string(),
            InspectorField::LightPosition => return vec3_csv(light.position),
            InspectorField::LightDirection => return vec3_csv(light.direction),
            InspectorField::LightTemperature => return format!("{:.0}", light.temperature),
            InspectorField::LightIntensity => return format!("{:.3}", light.intensity),
            InspectorField::LightRadius => return format!("{:.3}", light.radius),
            _ => {}
        }
        if let Some(plane) = selected_water_plane(app) {
            let (min_x, min_y, max_x, max_y, z) = water_plane_bounds(plane);
            match field {
                InspectorField::WaterMinX => return format!("{min_x:.3}"),
                InspectorField::WaterMinY => return format!("{min_y:.3}"),
                InspectorField::WaterMaxX => return format!("{max_x:.3}"),
                InspectorField::WaterMaxY => return format!("{max_y:.3}"),
                InspectorField::WaterHeight => return format!("{z:.3}"),
                InspectorField::WaterType => return plane.kind.to_string(),
                _ => {}
            }
        }
    }
    let Some(placement) = app.placements.get(app.selected) else {
        return String::new();
    };
    match field {
        InspectorField::ElementId => placement.id.clone(),
        InspectorField::ElementPosX => format!("{:.3}", placement.pos.x),
        InspectorField::ElementPosY => format!("{:.3}", placement.pos.y),
        InspectorField::ElementPosZ => format!("{:.3}", placement.pos.z),
        InspectorField::ElementRotX => format!("{:.3}", placement.rot.x),
        InspectorField::ElementRotY => format!("{:.3}", placement.rot.y),
        InspectorField::ElementRotZ => format!("{:.3}", placement.rot.z),
        InspectorField::CollisionFaceMaterial | InspectorField::CollisionFaceLight => {
            if app.active_tab == AppTab::Editing {
                if let Some(EditingAsset::Col(col)) = app.editing.asset.as_ref() {
                    if let Some(selected) = col.selected_primitive {
                        let surface = match selected.kind {
                            CollisionPrimitiveKind::Sphere => col
                                .mesh
                                .spheres
                                .get(selected.index)
                                .map(|sphere| &sphere.surface),
                            CollisionPrimitiveKind::Box => col
                                .mesh
                                .boxes
                                .get(selected.index)
                                .map(|col_box| &col_box.surface),
                            CollisionPrimitiveKind::Cuboid => col
                                .cuboids
                                .get(selected.index)
                                .map(|cuboid| &cuboid.surface),
                            CollisionPrimitiveKind::Capsule => col
                                .capsules
                                .get(selected.index)
                                .map(|capsule| &capsule.surface),
                        };
                        if let Some(surface) = surface {
                            return match field {
                                InspectorField::CollisionFaceMaterial => {
                                    surface.material.to_string()
                                }
                                InspectorField::CollisionFaceLight => surface.light.to_string(),
                                _ => String::new(),
                            };
                        }
                    }
                    if let Some(face) = col.mesh.faces.get(col.selected_face) {
                        return match field {
                            InspectorField::CollisionFaceMaterial => face.material.to_string(),
                            InspectorField::CollisionFaceLight => face.light.to_string(),
                            _ => String::new(),
                        };
                    }
                }
            }
            if let Some(selected) = app.selected_col_face {
                if let Some(placement) = app.placements.get(selected.placement) {
                    if let Some(mesh) = element_collision_mesh(app, placement) {
                        if let Some(face) = mesh.faces.get(selected.face) {
                            return match field {
                                InspectorField::CollisionFaceMaterial => face.material.to_string(),
                                InspectorField::CollisionFaceLight => face.light.to_string(),
                                _ => String::new(),
                            };
                        }
                    }
                }
            }
            String::new()
        }
        InspectorField::CollisionVertexX
        | InspectorField::CollisionVertexY
        | InspectorField::CollisionVertexZ => {
            let pos = if app.active_tab == AppTab::Editing {
                if let Some(EditingAsset::Col(col)) = app.editing.asset.as_ref() {
                    if let Some(selected) = col.selected_primitive {
                        match selected.kind {
                            CollisionPrimitiveKind::Sphere => col
                                .mesh
                                .spheres
                                .get(selected.index)
                                .map(|sphere| sphere.center),
                            CollisionPrimitiveKind::Box => {
                                col.mesh.boxes.get(selected.index).map(|col_box| V3 {
                                    x: (col_box.min.x + col_box.max.x) * 0.5,
                                    y: (col_box.min.y + col_box.max.y) * 0.5,
                                    z: (col_box.min.z + col_box.max.z) * 0.5,
                                })
                            }
                            CollisionPrimitiveKind::Cuboid => {
                                col.cuboids.get(selected.index).map(|cuboid| cuboid.center)
                            }
                            CollisionPrimitiveKind::Capsule => {
                                col.capsules.get(selected.index).map(|capsule| {
                                    from_mq((to_mq(capsule.start) + to_mq(capsule.end)) * 0.5)
                                })
                            }
                        }
                    } else {
                        col.mesh.faces.get(col.selected_face).and_then(|face| {
                            let idx = collision_selected_vertex_index_from_face(
                                face,
                                col.selected_vertex,
                            );
                            col.mesh.vertices.get(idx).copied()
                        })
                    }
                } else {
                    None
                }
            } else {
                selected_collision_tab_vertex_position(app).map(from_mq)
            };
            if let Some(pos) = pos {
                return match field {
                    InspectorField::CollisionVertexX => format!("{:.3}", pos.x),
                    InspectorField::CollisionVertexY => format!("{:.3}", pos.y),
                    InspectorField::CollisionVertexZ => format!("{:.3}", pos.z),
                    _ => String::new(),
                };
            }
            String::new()
        }
        InspectorField::CollisionPrimitiveSizeX
        | InspectorField::CollisionPrimitiveSizeY
        | InspectorField::CollisionPrimitiveSizeZ => {
            if let Some(size) = selected_editing_col_primitive_size(app) {
                return match field {
                    InspectorField::CollisionPrimitiveSizeX => format!("{:.3}", size.x),
                    InspectorField::CollisionPrimitiveSizeY => format!("{:.3}", size.y),
                    InspectorField::CollisionPrimitiveSizeZ => format!("{:.3}", size.z),
                    _ => String::new(),
                };
            }
            String::new()
        }
        InspectorField::CollisionPrimitiveRotX
        | InspectorField::CollisionPrimitiveRotY
        | InspectorField::CollisionPrimitiveRotZ => {
            if let Some(rotation) = selected_editing_col_primitive_rotation(app) {
                return match field {
                    InspectorField::CollisionPrimitiveRotX => format!("{:.3}", rotation.x),
                    InspectorField::CollisionPrimitiveRotY => format!("{:.3}", rotation.y),
                    InspectorField::CollisionPrimitiveRotZ => format!("{:.3}", rotation.z),
                    _ => String::new(),
                };
            }
            String::new()
        }
        InspectorField::ElementDimension => placement
            .attrs
            .get("dimension")
            .cloned()
            .unwrap_or_else(|| "0".to_string()),
        InspectorField::ElementInterior => placement
            .attrs
            .get("interior")
            .cloned()
            .unwrap_or_else(|| "0".to_string()),
        InspectorField::ElementLodParent => placement
            .attrs
            .get("lodParent")
            .cloned()
            .unwrap_or_default(),
        InspectorField::ElementUniqueId => {
            placement.attrs.get("uniqueID").cloned().unwrap_or_default()
        }
        InspectorField::ElementAlpha => placement.attrs.get("alpha").cloned().unwrap_or_default(),
        InspectorField::ElementScale => placement.attrs.get("scale").cloned().unwrap_or_default(),
        InspectorField::PhysicsSimulated => physics_attr_value(app, "simulated"),
        InspectorField::PhysicsMass => physics_attr_value(app, "mass"),
        InspectorField::PhysicsTurnMass => physics_attr_value(app, "turnMass"),
        InspectorField::PhysicsAirResistance => physics_attr_value(app, "airResistance"),
        InspectorField::PhysicsElasticity => physics_attr_value(app, "elasticity"),
        InspectorField::PhysicsBuoyancy => physics_attr_value(app, "buoyancy"),
        InspectorField::PhysicsCenterOfMassX => physics_attr_value(app, "centerOfMassX"),
        InspectorField::PhysicsCenterOfMassY => physics_attr_value(app, "centerOfMassY"),
        InspectorField::PhysicsCenterOfMassZ => physics_attr_value(app, "centerOfMassZ"),
        InspectorField::DefinitionDff => selected_definition(app)
            .and_then(|def| dff_override_stem(&def.id, def.attrs.get("dff").map(String::as_str)))
            .unwrap_or_default(),
        InspectorField::DefinitionNativeModel => selected_definition(app)
            .and_then(|def| def.attrs.get("nativeModel"))
            .cloned()
            .unwrap_or_default(),
        InspectorField::DefinitionTxd => selected_definition(app)
            .and_then(|def| def.attrs.get("txd"))
            .cloned()
            .unwrap_or_default(),
        InspectorField::DefinitionCol => selected_definition(app)
            .and_then(|def| def.attrs.get("col"))
            .cloned()
            .unwrap_or_default(),
        InspectorField::DefinitionLod => selected_definition(app)
            .and_then(|def| {
                def.attrs
                    .get("lodDistance")
                    .or_else(|| def.attrs.get("drawDistance"))
            })
            .cloned()
            .unwrap_or_default(),
        InspectorField::DefinitionTimeIn => selected_definition(app)
            .and_then(|def| def.attrs.get("timeIn"))
            .cloned()
            .unwrap_or_default(),
        InspectorField::DefinitionTimeOut => selected_definition(app)
            .and_then(|def| def.attrs.get("timeOut"))
            .cloned()
            .unwrap_or_default(),
        _ => String::new(),
    }
}

fn inspector_history_snapshot(app: &AppState) -> ScopedHistorySnapshot {
    match app.active_tab {
        AppTab::Water => ScopedHistorySnapshot::Water(water_history_snapshot(app)),
        AppTab::Cull => ScopedHistorySnapshot::Cull(cull_history_snapshot(app)),
        AppTab::Lights => ScopedHistorySnapshot::Lights(light_history_snapshot(app)),
        AppTab::Race => ScopedHistorySnapshot::Race(race_history_snapshot(app)),
        AppTab::Collisions => ScopedHistorySnapshot::Collision(collision_history_snapshot(app)),
        AppTab::Editing => ScopedHistorySnapshot::Editing(editing_history_snapshot(app)),
        AppTab::Bake => ScopedHistorySnapshot::None,
        _ => ScopedHistorySnapshot::World(world_history_snapshot(app)),
    }
}

pub(crate) fn start_inspector_edit(app: &mut AppState, field: InspectorField) {
    drain_text_input();
    let buffer = inspector_field_value(app, field);
    let cursor = buffer.len();
    app.inspector_edit = Some(InspectorEdit {
        field,
        buffer,
        cursor,
        selection_anchor: None,
        before: inspector_history_snapshot(app),
    });
}

pub(crate) fn drain_text_input() {
    while get_char_pressed().is_some() {}
}

fn current_col_material(app: &AppState) -> Option<u8> {
    if app.active_tab == AppTab::Editing {
        if let Some(EditingAsset::Col(col)) = app.editing.asset.as_ref() {
            if let Some(selected) = col.selected_primitive {
                return match selected.kind {
                    CollisionPrimitiveKind::Sphere => col
                        .mesh
                        .spheres
                        .get(selected.index)
                        .map(|sphere| sphere.surface.material),
                    CollisionPrimitiveKind::Box => col
                        .mesh
                        .boxes
                        .get(selected.index)
                        .map(|col_box| col_box.surface.material),
                    CollisionPrimitiveKind::Cuboid => col
                        .cuboids
                        .get(selected.index)
                        .map(|cuboid| cuboid.surface.material),
                    CollisionPrimitiveKind::Capsule => col
                        .capsules
                        .get(selected.index)
                        .map(|capsule| capsule.surface.material),
                };
            }
            return col
                .mesh
                .faces
                .get(col.selected_face)
                .map(|face| face.material);
        }
    }
    let selected = app.selected_col_face?;
    let placement = app.placements.get(selected.placement)?;
    let mesh = element_collision_mesh(app, placement)?;
    mesh.faces.get(selected.face).map(|face| face.material)
}

fn center_col_material_dropdown_on_current(app: &mut AppState) {
    let Some(material) = current_col_material(app) else {
        app.col_material_dropdown_scroll = 0.0;
        return;
    };
    let selected_row = GTA_SA_COL_MATERIALS
        .iter()
        .position(|(id, _)| *id == material)
        .unwrap_or(0);
    let visible = COL_MATERIAL_DROPDOWN_VISIBLE.min(GTA_SA_COL_MATERIALS.len());
    let max_scroll = GTA_SA_COL_MATERIALS.len().saturating_sub(visible) as f32;
    app.col_material_dropdown_scroll =
        (selected_row.saturating_sub(visible / 2) as f32).min(max_scroll);
}

fn set_current_col_material(app: &mut AppState, material: u8) -> bool {
    if app.active_tab == AppTab::Editing {
        if current_col_material(app) == Some(material) {
            app.status_message = format!("COL material already {}", col_material_label(material));
            return false;
        }
        return set_selected_editing_col_material(app, material);
    }
    let before = current_col_material(app);
    if before == Some(material) {
        app.status_message = format!("COL material already {}", col_material_label(material));
        return false;
    }
    match set_selected_collision_tab_material(app, material) {
        Ok(()) => true,
        Err(err) => {
            app.status_message = err;
            false
        }
    }
}

fn handle_col_material_dropdown_click(app: &mut AppState, mouse: Vec2) -> bool {
    let rect = inspector_field_rect(app, InspectorField::CollisionFaceMaterial);
    let active_context = app.active_tab == AppTab::Editing
        && matches!(app.editing.asset, Some(EditingAsset::Col(_)));
    if !active_context {
        app.col_material_dropdown_open = false;
        return false;
    }
    if app.col_material_dropdown_open && is_mouse_button_down(MouseButton::Left) {
        let visible = COL_MATERIAL_DROPDOWN_VISIBLE.min(GTA_SA_COL_MATERIALS.len());
        let total = GTA_SA_COL_MATERIALS.len();
        let max_scroll = total.saturating_sub(visible) as f32;
        let track = col_material_dropdown_scrollbar_rect(app);
        let hit_area = Rect::new(track.x - 6.0, track.y, track.w + 12.0, track.h);
        if max_scroll > 0.0 && hit_area.contains(mouse) {
            let thumb_h = (track.h * visible as f32 / total as f32).max(14.0);
            let travel = (track.h - thumb_h).max(1.0);
            let position = (mouse.y - track.y - thumb_h * 0.5).clamp(0.0, travel);
            app.col_material_dropdown_scroll = position / travel * max_scroll;
            return true;
        }
    }
    if !is_mouse_button_pressed(MouseButton::Left) {
        return false;
    }
    if app.col_material_dropdown_open {
        let visible = COL_MATERIAL_DROPDOWN_VISIBLE.min(GTA_SA_COL_MATERIALS.len());
        let start = app
            .col_material_dropdown_scroll
            .floor()
            .max(0.0)
            .min(GTA_SA_COL_MATERIALS.len().saturating_sub(visible) as f32)
            as usize;
        for row in 0..visible {
            let Some((material, _)) = GTA_SA_COL_MATERIALS.get(start + row).copied() else {
                break;
            };
            if col_material_option_rect(app, row).contains(mouse) {
                let before = if app.active_tab == AppTab::Editing {
                    ScopedHistorySnapshot::Editing(editing_history_snapshot(app))
                } else {
                    ScopedHistorySnapshot::Collision(collision_history_snapshot(app))
                };
                if set_current_col_material(app, material) {
                    commit_scoped_history(app, "Set COL Material", before);
                }
                app.col_material_dropdown_open = false;
                return true;
            }
        }
    }
    if rect.contains(mouse) {
        if app.inspector_edit.is_some() {
            apply_inspector_edit(app);
        }
        app.col_material_dropdown_open = !app.col_material_dropdown_open;
        if app.col_material_dropdown_open {
            center_col_material_dropdown_on_current(app);
        }
        return true;
    }
    if app.col_material_dropdown_open {
        app.col_material_dropdown_open = false;
        return true;
    }
    false
}

pub(crate) fn assign_selected_definition_txd(app: &mut AppState, value: &str) -> bool {
    let value = value.trim();
    if value.is_empty() {
        return false;
    }
    let before = world_history_snapshot(app);
    let ids = ensure_selected_definition_overrides(app);
    if ids.is_empty() {
        app.status_message = "Select an element before choosing a TXD".to_string();
        return false;
    }
    let mut changed = 0usize;
    for id in ids {
        let Some(definition) = app.definitions.get_mut(&id) else {
            continue;
        };
        let old = definition.attrs.get("txd").map(String::as_str);
        if !old.is_some_and(|old| old.eq_ignore_ascii_case(value)) {
            definition
                .attrs
                .insert("txd".to_string(), value.to_string());
            mark_definition_override_attr(definition, "txd");
            changed += 1;
        }
    }
    if changed > 0 {
        commit_world_history(app, "Assign TXD", before);
        invalidate_validation_cache(app);
        app.status_message = format!("Assigned TXD {value} to {changed} definition(s)");
    }
    true
}

fn close_txd_dropdown(app: &mut AppState) {
    app.txd_dropdown_open = false;
    app.txd_dropdown_search.clear();
    app.txd_dropdown_scroll = 0.0;
    app.txd_dropdown_create_mode = false;
}

fn close_native_model_dropdown(app: &mut AppState) {
    app.native_model_dropdown_open = false;
    app.native_model_dropdown_search.clear();
    app.native_model_dropdown_scroll = 0.0;
}

fn apply_native_model_dropdown_value(app: &mut AppState, value: String) {
    let before = ScopedHistorySnapshot::World(world_history_snapshot(app));
    app.inspector_edit = Some(InspectorEdit {
        field: InspectorField::DefinitionNativeModel,
        cursor: value.len(),
        buffer: value,
        selection_anchor: None,
        before,
    });
    apply_inspector_edit(app);
}

fn handle_native_model_dropdown_input(app: &mut AppState, mouse: Vec2) -> bool {
    let active_context = app.active_tab == AppTab::Preview
        && app.properties_tab == PropertiesTab::Element
        && selected_placement(app).is_some();
    if !active_context {
        close_native_model_dropdown(app);
        return false;
    }

    let field = inspector_field_rect(app, InspectorField::DefinitionNativeModel);
    if !app.native_model_dropdown_open {
        if is_mouse_button_pressed(MouseButton::Left) && field.contains(mouse) {
            if app.inspector_edit.is_some() {
                apply_inspector_edit(app);
            }
            close_txd_dropdown(app);
            app.native_model_dropdown_open = true;
            app.native_model_dropdown_search.clear();
            app.native_model_dropdown_scroll = 0.0;
            drain_text_input();
            return true;
        }
        return false;
    }

    if is_key_pressed(KeyCode::Escape) {
        close_native_model_dropdown(app);
        return true;
    }
    if is_key_pressed(KeyCode::Backspace) {
        app.native_model_dropdown_search.pop();
        app.native_model_dropdown_scroll = 0.0;
    }
    while let Some(ch) = get_char_pressed() {
        if !ch.is_control() && app.native_model_dropdown_search.len() < 64 {
            app.native_model_dropdown_search.push(ch);
            app.native_model_dropdown_scroll = 0.0;
        }
    }

    let specs = filtered_native_model_specs(app);
    let visible = specs.len().min(NATIVE_MODEL_DROPDOWN_VISIBLE);
    let max_start = specs.len().saturating_sub(visible);
    if is_key_pressed(KeyCode::Down) {
        app.native_model_dropdown_scroll =
            (app.native_model_dropdown_scroll + 1.0).min(max_start as f32);
    }
    if is_key_pressed(KeyCode::Up) {
        app.native_model_dropdown_scroll = (app.native_model_dropdown_scroll - 1.0).max(0.0);
    }
    let first = native_model_dropdown_option_rect(app, 0);
    let custom = native_model_dropdown_custom_rect(app);
    let popup = Rect::new(
        first.x - 4.0,
        first.y - 4.0,
        first.w + 8.0,
        custom.y + custom.h - first.y + 8.0,
    );
    let wheel = mouse_wheel().1;
    if wheel.abs() > 0.0 && popup.contains(mouse) {
        app.native_model_dropdown_scroll =
            (app.native_model_dropdown_scroll - wheel * 2.0).clamp(0.0, max_start as f32);
        return true;
    }

    if is_key_pressed(KeyCode::Enter) {
        let value = specs
            .get(app.native_model_dropdown_scroll.floor().max(0.0) as usize)
            .map(|spec| spec.model_id.to_string())
            .unwrap_or_else(|| app.native_model_dropdown_search.trim().to_string());
        apply_native_model_dropdown_value(app, value);
        close_native_model_dropdown(app);
        return true;
    }

    if !is_mouse_button_pressed(MouseButton::Left) {
        return true;
    }
    let start = (app.native_model_dropdown_scroll.floor().max(0.0) as usize).min(max_start);
    for row in 0..visible {
        if native_model_dropdown_option_rect(app, row).contains(mouse) {
            if let Some(spec) = specs.get(start + row) {
                apply_native_model_dropdown_value(app, spec.model_id.to_string());
            }
            close_native_model_dropdown(app);
            return true;
        }
    }
    if custom.contains(mouse) {
        let value = app.native_model_dropdown_search.trim().to_string();
        apply_native_model_dropdown_value(app, value);
        close_native_model_dropdown(app);
        return true;
    }
    if field.contains(mouse) {
        return true;
    }
    close_native_model_dropdown(app);
    true
}

fn handle_txd_dropdown_input(app: &mut AppState, mouse: Vec2) -> bool {
    let active_context = app.active_tab == AppTab::Preview
        && app.properties_tab == PropertiesTab::Element
        && selected_placement(app).is_some();
    if !active_context {
        close_txd_dropdown(app);
        return false;
    }

    let field = inspector_field_rect(app, InspectorField::DefinitionTxd);
    if !app.txd_dropdown_open {
        if is_mouse_button_pressed(MouseButton::Left) && field.contains(mouse) {
            if app.inspector_edit.is_some() {
                apply_inspector_edit(app);
            }
            close_native_model_dropdown(app);
            app.txd_dropdown_open = true;
            app.txd_dropdown_search.clear();
            app.txd_dropdown_scroll = 0.0;
            app.txd_dropdown_create_mode = false;
            return true;
        }
        return false;
    }

    if is_key_pressed(KeyCode::Escape) {
        close_txd_dropdown(app);
        return true;
    }
    if is_key_pressed(KeyCode::Backspace) {
        app.txd_dropdown_search.pop();
        app.txd_dropdown_scroll = 0.0;
    }
    while let Some(ch) = get_char_pressed() {
        if !ch.is_control() && app.txd_dropdown_search.len() < 64 {
            app.txd_dropdown_search.push(ch);
            app.txd_dropdown_scroll = 0.0;
        }
    }

    let names = filtered_scene_txd_names(app);
    let visible = names.len().min(TXD_DROPDOWN_VISIBLE);
    let max_start = names.len().saturating_sub(visible);
    if is_key_pressed(KeyCode::Down) {
        app.txd_dropdown_scroll = (app.txd_dropdown_scroll + 1.0).min(max_start as f32);
    }
    if is_key_pressed(KeyCode::Up) {
        app.txd_dropdown_scroll = (app.txd_dropdown_scroll - 1.0).max(0.0);
    }
    let first = txd_dropdown_option_rect(app, 0);
    let create = txd_dropdown_create_rect(app);
    let popup = Rect::new(
        first.x - 4.0,
        first.y - 4.0,
        first.w + 8.0,
        create.y + create.h - first.y + 8.0,
    );
    let wheel = mouse_wheel().1;
    if wheel.abs() > 0.0 && popup.contains(mouse) {
        app.txd_dropdown_scroll =
            (app.txd_dropdown_scroll - wheel * 2.0).clamp(0.0, max_start as f32);
        return true;
    }

    if is_key_pressed(KeyCode::Enter) {
        if app.txd_dropdown_create_mode {
            let name = app.txd_dropdown_search.clone();
            if create_empty_txd_for_selected(app, &name) {
                close_txd_dropdown(app);
            }
        } else if let Some(name) = names
            .get(app.txd_dropdown_scroll.floor().max(0.0) as usize)
            .cloned()
        {
            assign_selected_definition_txd(app, &name);
            close_txd_dropdown(app);
        }
        return true;
    }

    if !is_mouse_button_pressed(MouseButton::Left) {
        return true;
    }
    let start = (app.txd_dropdown_scroll.floor().max(0.0) as usize).min(max_start);
    for row in 0..visible {
        if txd_dropdown_option_rect(app, row).contains(mouse) {
            if let Some(name) = names.get(start + row) {
                assign_selected_definition_txd(app, name);
            }
            close_txd_dropdown(app);
            return true;
        }
    }
    if create.contains(mouse) {
        if app.txd_dropdown_create_mode {
            let name = app.txd_dropdown_search.clone();
            if create_empty_txd_for_selected(app, &name) {
                close_txd_dropdown(app);
            }
        } else {
            app.txd_dropdown_create_mode = true;
            app.txd_dropdown_search.clear();
            app.txd_dropdown_scroll = 0.0;
        }
        return true;
    }
    if field.contains(mouse) {
        return true;
    }
    close_txd_dropdown(app);
    true
}

pub(crate) fn selected_definition_ids(app: &AppState) -> Vec<String> {
    let mut ids = Vec::new();
    let mut seen = BTreeSet::new();
    for idx in selected_live_indices(app) {
        if let Some(id) = app
            .placements
            .get(idx)
            .map(|placement| placement.id.clone())
        {
            if seen.insert(id.clone()) {
                ids.push(id);
            }
        }
    }
    ids
}

fn set_element_type_for_indices(
    placements: &mut [Placement],
    indices: &[usize],
    element_type: &str,
) -> usize {
    let mut changed = 0usize;
    for &idx in indices {
        let Some(placement) = placements.get_mut(idx) else {
            continue;
        };
        if placement.tag != element_type {
            placement.tag = element_type.to_string();
            changed += 1;
        }
    }
    changed
}

fn selected_definitions_include_readonly(app: &AppState) -> bool {
    selected_definition_ids(app)
        .iter()
        .any(|id| app.readonly_definition_ids.contains(id))
}

fn selected_zone_for_definition(app: &AppState, id: &str) -> String {
    for idx in selected_live_indices(app) {
        if let Some(placement) = app.placements.get(idx) {
            if placement.id == id {
                return placement.zone.clone();
            }
        }
    }
    app.placements
        .get(app.selected)
        .map(|placement| placement.zone.clone())
        .or_else(|| app.zones.first().cloned())
        .unwrap_or_else(|| "SA".to_string())
}

fn override_attr_for_definition_field(field: InspectorField) -> Option<&'static str> {
    match field {
        InspectorField::DefinitionDff => Some("dff"),
        InspectorField::DefinitionNativeModel => Some("nativeModel"),
        InspectorField::DefinitionTxd => Some("txd"),
        InspectorField::DefinitionCol => Some("col"),
        InspectorField::DefinitionLod => Some("lodDistance"),
        InspectorField::DefinitionTimeIn => Some("timeIn"),
        InspectorField::DefinitionTimeOut => Some("timeOut"),
        InspectorField::PhysicsSimulated => Some("simulated"),
        InspectorField::PhysicsMass => Some("mass"),
        InspectorField::PhysicsTurnMass => Some("turnMass"),
        InspectorField::PhysicsAirResistance => Some("airResistance"),
        InspectorField::PhysicsElasticity => Some("elasticity"),
        InspectorField::PhysicsBuoyancy => Some("buoyancy"),
        InspectorField::PhysicsCenterOfMassX => Some("centerOfMassX"),
        InspectorField::PhysicsCenterOfMassY => Some("centerOfMassY"),
        InspectorField::PhysicsCenterOfMassZ => Some("centerOfMassZ"),
        _ => None,
    }
}

fn physics_attr_for_inspector_field(field: InspectorField) -> Option<&'static str> {
    match field {
        InspectorField::PhysicsSimulated => Some("simulated"),
        InspectorField::PhysicsMass => Some("mass"),
        InspectorField::PhysicsTurnMass => Some("turnMass"),
        InspectorField::PhysicsAirResistance => Some("airResistance"),
        InspectorField::PhysicsElasticity => Some("elasticity"),
        InspectorField::PhysicsBuoyancy => Some("buoyancy"),
        InspectorField::PhysicsCenterOfMassX => Some("centerOfMassX"),
        InspectorField::PhysicsCenterOfMassY => Some("centerOfMassY"),
        InspectorField::PhysicsCenterOfMassZ => Some("centerOfMassZ"),
        _ => None,
    }
}

fn apply_physics_root_attrs(
    attrs: &mut BTreeMap<String, String>,
    model_id: Option<u16>,
    properties: Option<PhysicsRootProperties>,
) -> usize {
    let before = attrs.clone();
    for key in PHYSICS_ROOT_ATTR_KEYS {
        attrs.remove(key);
    }
    if let Some(model_id) = model_id {
        attrs.insert("physicsRoot".to_string(), model_id.to_string());
        attrs.insert("simulated".to_string(), "true".to_string());
        if let Some(properties) = properties {
            attrs.insert("mass".to_string(), fmt_f32(properties.mass, 6));
            attrs.insert("turnMass".to_string(), fmt_f32(properties.turn_mass, 6));
            attrs.insert(
                "airResistance".to_string(),
                fmt_f32(properties.air_resistance, 6),
            );
            attrs.insert("elasticity".to_string(), fmt_f32(properties.elasticity, 6));
            attrs.insert("buoyancy".to_string(), fmt_f32(properties.buoyancy, 6));
        }
        // GTA's object.dat roots do not define a center-of-mass override.
        for key in ["centerOfMassX", "centerOfMassY", "centerOfMassZ"] {
            attrs.remove(key);
        }
    }
    before
        .iter()
        .filter(|(key, value)| attrs.get(*key) != Some(*value))
        .count()
        + attrs
            .keys()
            .filter(|key| !before.contains_key(*key))
            .count()
}

fn physics_conversion_indices(app: &AppState) -> Vec<usize> {
    match app.physics_scope {
        PhysicsScope::PerObject => selected_live_indices(app)
            .into_iter()
            .filter(|index| {
                app.placements
                    .get(*index)
                    .is_some_and(|placement| placement.tag.eq_ignore_ascii_case("building"))
            })
            .collect(),
        PhysicsScope::Global => {
            let ids: BTreeSet<_> = selected_definition_ids(app).into_iter().collect();
            app.placements
                .iter()
                .enumerate()
                .filter(|(index, placement)| {
                    is_live_element(app, *index)
                        && ids.contains(&placement.id)
                        && !placement.tag.eq_ignore_ascii_case("object")
                })
                .map(|(index, _)| index)
                .collect()
        }
    }
}

fn convert_physics_elements_to_objects(app: &mut AppState, indices: &[usize]) -> usize {
    let mut converted = 0usize;
    for index in indices.iter().copied() {
        let Some(placement) = app.placements.get_mut(index) else {
            continue;
        };
        if !placement.tag.eq_ignore_ascii_case("object") {
            placement.tag = "object".to_string();
            converted += 1;
        }
    }
    if converted > 0 {
        invalidate_outliner_labels(app);
        invalidate_validation_cache(app);
        rebuild_outliner_filter(app);
        rebuild_render_cells(app);
    }
    converted
}

fn physics_conversion_detail(app: &AppState, count: usize) -> String {
    match app.physics_scope {
        PhysicsScope::PerObject => format!(
            "{count} selected building instance(s) will be permanently retagged as object elements."
        ),
        PhysicsScope::Global => format!(
            "Global physics applies by model: all {count} non-object instance(s) using the selected definition(s) will be retagged as object elements."
        ),
    }
}

fn set_selected_physics_root_with_before(
    app: &mut AppState,
    model_id: Option<u16>,
    before: WorldHistorySnapshot,
) {
    if app.physics_scope == PhysicsScope::PerObject && model_id.is_some() {
        app.status_message =
            "Physical roots are model-wide in MTA. Switch Physics Scope to Global; per-object mass and center-of-mass overrides remain supported."
                .to_string();
        return;
    }
    let properties = model_id.and_then(|model_id| {
        app.physics_root_properties
            .get(&model_id)
            .cloned()
            .or_else(|| physics_root_spec(model_id).map(|spec| spec.fallback.clone()))
    });
    let changed = match app.physics_scope {
        PhysicsScope::Global => {
            let ids = ensure_selected_definition_overrides(app);
            if ids.is_empty() {
                app.status_message = "Select an element first".to_string();
                return;
            }
            let mut changed = 0usize;
            for id in ids {
                let Some(def) = app.definitions.get_mut(&id) else {
                    continue;
                };
                changed += apply_physics_root_attrs(&mut def.attrs, model_id, properties.clone());
                for key in [
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
                ] {
                    mark_definition_override_attr(def, key);
                }
            }
            changed
        }
        PhysicsScope::PerObject => {
            let indices = selected_live_indices(app);
            if indices.is_empty() {
                app.status_message = "Select an element first".to_string();
                return;
            }
            let mut changed = 0usize;
            for idx in indices {
                let Some(placement) = app.placements.get_mut(idx) else {
                    continue;
                };
                changed +=
                    apply_physics_root_attrs(&mut placement.attrs, model_id, properties.clone());
            }
            changed
        }
    };
    let _ = changed;
    commit_world_history(app, "Set Physics Root", before);
    app.status_message = match model_id {
        Some(model_id) => format!(
            "{} physics root: {}",
            app.physics_scope.label(),
            physics_root_label(Some(model_id))
        ),
        None if app.physics_scope == PhysicsScope::PerObject => {
            "Per Object physics root cleared; Global root is inherited".to_string()
        }
        None => "Global physics root cleared; manual values kept".to_string(),
    };
}

fn request_set_selected_physics_root(app: &mut AppState, model_id: Option<u16>) {
    let before = world_history_snapshot(app);
    if app.physics_scope == PhysicsScope::PerObject && model_id.is_some() {
        set_selected_physics_root_with_before(app, model_id, before);
        return;
    }
    let conversion_indices = model_id.map_or_else(Vec::new, |_| physics_conversion_indices(app));
    if let Some(model_id) = model_id
        && !conversion_indices.is_empty()
    {
        app.confirm_dialog = Some(ConfirmDialog {
            action: ConfirmAction::SetPhysicsRoot {
                model_id,
                conversion_indices: conversion_indices.clone(),
                before,
            },
            title: "Convert Elements for Physics?".to_string(),
            body: "MTA physics requires object elements. Apply physics and convert the affected elements?"
                .to_string(),
            detail: physics_conversion_detail(app, conversion_indices.len()),
            primary_label: "Convert & Apply".to_string(),
            secondary_label: None,
            secondary_action: None,
        });
        app.status_message = "Confirm element conversion before applying physics".to_string();
        return;
    }
    set_selected_physics_root_with_before(app, model_id, before);
}

fn handle_physics_root_dropdown_click(app: &mut AppState, mouse: Vec2) -> bool {
    if !is_mouse_button_pressed(MouseButton::Left) {
        return false;
    }
    if app.physics_root_dropdown_open {
        let option_count = PHYSICS_ROOT_SPECS.len() + 1;
        for row in 0..option_count {
            if physics_root_option_rect(app, row).contains(mouse) {
                if app.inspector_edit.is_some() {
                    apply_inspector_edit(app);
                }
                let model_id = row
                    .checked_sub(1)
                    .map(|index| PHYSICS_ROOT_SPECS[index].model_id);
                request_set_selected_physics_root(app, model_id);
                app.physics_root_dropdown_open = false;
                return true;
            }
        }
    }
    if physics_root_rect(app).contains(mouse) {
        if app.inspector_edit.is_some() {
            apply_inspector_edit(app);
        }
        app.physics_root_dropdown_open = !app.physics_root_dropdown_open;
        return true;
    }
    if app.physics_root_dropdown_open {
        app.physics_root_dropdown_open = false;
        return true;
    }
    false
}

pub(crate) fn mark_definition_override_attr(def: &mut Definition, key: &str) {
    if !is_override_definition(def) {
        return;
    }
    let mut attrs: BTreeSet<String> = def
        .attrs
        .get("__overrideAttrs")
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default();
    attrs.insert(key.to_string());
    def.attrs.insert(
        "__overrideAttrs".to_string(),
        attrs.into_iter().collect::<Vec<_>>().join(","),
    );
}

pub(crate) fn make_definition_override_writable(app: &mut AppState, id: &str, zone: String) {
    if !app.readonly_definition_ids.remove(id)
        && app.definitions.get(id).is_some_and(is_override_definition)
    {
        return;
    }
    let Some(current) = app.definitions.get(id).cloned() else {
        return;
    };
    let mut attrs = current.attrs.clone();
    attrs.insert("id".to_string(), id.to_string());
    attrs.insert("zone".to_string(), zone.clone());
    attrs.remove("source");
    attrs.insert("__override".to_string(), "true".to_string());
    attrs.entry("__overrideAttrs".to_string()).or_default();
    app.definitions.insert(
        id.to_string(),
        Definition {
            id: id.to_string(),
            zone,
            attrs,
        },
    );
}

fn ensure_selected_definition_overrides(app: &mut AppState) -> Vec<String> {
    let ids = selected_definition_ids(app);
    for id in &ids {
        if app.readonly_definition_ids.contains(id) {
            let zone = selected_zone_for_definition(app, id);
            make_definition_override_writable(app, id, zone);
        }
    }
    ids
}

fn prompt_definition_override_edit(
    app: &mut AppState,
    field: InspectorField,
    value: String,
    before: ScopedHistorySnapshot,
) {
    let ids = selected_definition_ids(app);
    if ids.is_empty() {
        return;
    }
    let selected_count = ids.len();
    app.confirm_dialog = Some(ConfirmDialog {
        action: ConfirmAction::DefinitionOverrideEdit {
            ids,
            field,
            value,
            before,
        },
        title: "Override GTA:SA Model?".to_string(),
        body: if selected_count == 1 {
            "Create an override for the selected GTA:SA model?".to_string()
        } else {
            format!("Create overrides for {selected_count} selected model definitions?")
        },
        detail: "The edited value will be written as <override> data in each selected model's first selected zone, and the original GTA:SA model IDs will be kept.".to_string(),
        primary_label: "Override".to_string(),
        secondary_label: None,
        secondary_action: None,
    });
}

fn apply_definition_override_flag(
    app: &mut AppState,
    ids: Vec<String>,
    flag: &'static str,
    target_enabled: bool,
    before: ScopedHistorySnapshot,
) {
    let mut changed = 0usize;
    for id in ids {
        if app.readonly_definition_ids.contains(&id) {
            let zone = selected_zone_for_definition(app, &id);
            make_definition_override_writable(app, &id, zone);
        }
        if let Some(def) = app.definitions.get_mut(&id) {
            set_definition_flag(def, flag, target_enabled);
            mark_definition_override_attr(def, "flags");
            changed += 1;
        }
    }
    if changed > 0 {
        invalidate_outliner_labels(app);
        rebuild_outliner_filter(app);
        rebuild_render_cells(app);
        commit_scoped_history(app, "Definition Override Flag", before);
        app.status_message = format!("Updated {changed} override flag(s)");
    }
}

fn select_same_id_as_active(app: &mut AppState) {
    let Some(active) = app.placements.get(app.selected) else {
        app.status_message = "Select an element first".to_string();
        return;
    };
    let id = active.id.clone();
    let before = world_history_snapshot(app);
    app.selected_elements.clear();
    app.selected_element_order.clear();
    for (idx, placement) in app.placements.iter().enumerate() {
        if placement.id == id && is_live_element(app, idx) {
            app.selected_elements.insert(idx);
            app.selected_element_order.push(idx);
        }
    }
    if !app.selected_elements.contains(&app.selected) {
        app.selected = app
            .selected_element_order
            .last()
            .copied()
            .unwrap_or(NO_SELECTION);
    }
    app.selected_group = None;
    app.selected_col_face = None;
    app.status_message = format!(
        "Selected {} instance(s) of {id}",
        app.selected_elements.len()
    );
    commit_world_history(app, "Select Same ID", before);
}

pub(crate) fn element_replace_target_ids(app: &AppState, query: &str) -> Vec<String> {
    let needle = query.trim().to_ascii_lowercase();
    let mut ids = BTreeSet::new();
    ids.extend(app.definitions.keys().cloned());
    ids.extend(app.placements.iter().map(|placement| placement.id.clone()));
    ids.into_iter()
        .filter(|id| needle.is_empty() || id.to_ascii_lowercase().contains(&needle))
        .collect()
}

fn open_element_replace_with_dialog(app: &mut AppState) {
    let source_indices = selected_live_indices(app);
    if source_indices.is_empty() {
        app.status_message = "Select one or more live elements first".to_string();
        return;
    }
    app.element_replace_with_dialog = Some(ElementReplaceWithDialog {
        source_indices,
        search: String::new(),
        cursor: 0,
        selection_anchor: None,
        scroll: 0.0,
        selected_target: None,
        picking_scene: false,
    });
}

fn replace_elements_with_id(app: &mut AppState, target: &str, source_indices: &[usize]) {
    let Some(target_id) = element_replace_target_ids(app, "")
        .into_iter()
        .find(|id| id.eq_ignore_ascii_case(target))
    else {
        app.status_message = format!("Model ID {target} was not found");
        return;
    };
    let target_dff = app
        .placements
        .iter()
        .find(|placement| placement.id.eq_ignore_ascii_case(&target_id))
        .map(|placement| placement.dff.clone())
        .or_else(|| {
            app.definitions.get(&target_id).map(|definition| {
                dff_override_stem(
                    &definition.id,
                    definition.attrs.get("dff").map(String::as_str),
                )
                .unwrap_or_else(|| definition.id.clone())
            })
        })
        .unwrap_or_else(|| target_id.clone());
    let before = ScopedHistorySnapshot::World(world_history_snapshot(app));
    let mut old_ids = BTreeSet::new();
    let mut changed = 0usize;
    for index in source_indices.iter().copied() {
        let Some(placement) = app.placements.get_mut(index) else {
            continue;
        };
        if placement.id.eq_ignore_ascii_case(&target_id) {
            continue;
        }
        old_ids.insert(placement.id.clone());
        placement.id = target_id.clone();
        placement.dff = target_dff.clone();
        sync_placement_attrs(placement);
        changed += 1;
    }
    if changed == 0 {
        app.status_message = format!("Selected elements already use {target_id}");
        return;
    }

    let mut retired_ids = Vec::new();
    for old_id in old_ids {
        let still_used = app
            .placements
            .iter()
            .any(|placement| placement.id.eq_ignore_ascii_case(&old_id));
        if !still_used && !app.readonly_definition_ids.contains(&old_id) {
            app.definitions.remove(&old_id);
            retired_ids.push(old_id);
        }
    }
    for placement in &mut app.placements {
        if placement.attrs.get("lodParent").is_some_and(|parent| {
            retired_ids
                .iter()
                .any(|old_id| parent.eq_ignore_ascii_case(old_id))
        }) {
            placement
                .attrs
                .insert("lodParent".to_string(), target_id.clone());
        }
    }
    invalidate_outliner_labels(app);
    invalidate_validation_cache(app);
    rebuild_outliner_filter(app);
    rebuild_render_cells(app);
    commit_scoped_history(app, "Replace Elements With Model", before);
    app.status_message = if retired_ids.is_empty() {
        format!("Replaced {changed} element(s) with {target_id}")
    } else {
        format!(
            "Replaced {changed} element(s) with {target_id}; combined {} unused definition(s)",
            retired_ids.len()
        )
    };
}

pub(crate) fn apply_inspector_edit(app: &mut AppState) {
    let Some(edit) = app.inspector_edit.take() else {
        return;
    };
    let value = edit.buffer.trim().to_string();
    let mut rebuild = false;
    if selected_definitions_include_readonly(app)
        && definition_field_is_readonly(edit.field)
        && !inspector_field_is_physics(edit.field)
    {
        prompt_definition_override_edit(app, edit.field, value, edit.before);
        return;
    }
    match edit.field {
        InspectorField::ElementId => {
            request_element_id_rename(app, edit.before, value);
            return;
        }
        InspectorField::ElementPosX
        | InspectorField::ElementPosY
        | InspectorField::ElementPosZ
        | InspectorField::ElementRotX
        | InspectorField::ElementRotY
        | InspectorField::ElementRotZ => {
            let Some(parsed) = parse_finite_f32(&value) else {
                app.status_message = "Transform values must be finite numbers".to_string();
                return;
            };
            for idx in selected_live_indices(app) {
                let Some(p) = app.placements.get_mut(idx) else {
                    continue;
                };
                match edit.field {
                    InspectorField::ElementPosX => p.pos.x = parsed,
                    InspectorField::ElementPosY => p.pos.y = parsed,
                    InspectorField::ElementPosZ => p.pos.z = parsed,
                    InspectorField::ElementRotX => p.rot.x = parsed,
                    InspectorField::ElementRotY => p.rot.y = parsed,
                    InspectorField::ElementRotZ => p.rot.z = parsed,
                    _ => unreachable!(),
                }
                sync_placement_attrs(p);
                rebuild = true;
            }
        }
        InspectorField::ElementDimension => {
            let Ok(dimension) = value.parse::<u16>() else {
                app.status_message = "Dimension must be a whole number from 0 to 65535".to_string();
                return;
            };
            for idx in selected_live_indices(app) {
                let Some(p) = app.placements.get_mut(idx) else {
                    continue;
                };
                p.attrs
                    .insert("dimension".to_string(), dimension.to_string());
            }
        }
        InspectorField::ElementInterior => {
            let Ok(interior) = value.parse::<u8>() else {
                app.status_message = "Interior must be a whole number from 0 to 255".to_string();
                return;
            };
            for idx in selected_live_indices(app) {
                let Some(p) = app.placements.get_mut(idx) else {
                    continue;
                };
                p.attrs.insert("interior".to_string(), interior.to_string());
            }
        }
        InspectorField::ElementLodParent => {
            for idx in selected_live_indices(app) {
                let Some(p) = app.placements.get_mut(idx) else {
                    continue;
                };
                set_optional_attr(&mut p.attrs, "lodParent", value.clone());
                rebuild = true;
            }
        }
        InspectorField::ElementUniqueId => {
            for idx in selected_live_indices(app) {
                let Some(p) = app.placements.get_mut(idx) else {
                    continue;
                };
                set_optional_attr(&mut p.attrs, "uniqueID", value.clone());
            }
        }
        InspectorField::ElementAlpha => {
            if !value.is_empty() {
                let Ok(alpha) = value.parse::<u8>() else {
                    app.status_message = "Alpha must be 0-255".to_string();
                    return;
                };
                for idx in selected_live_indices(app) {
                    let Some(p) = app.placements.get_mut(idx) else {
                        continue;
                    };
                    p.attrs.insert("alpha".to_string(), alpha.to_string());
                    rebuild = true;
                }
            } else {
                for idx in selected_live_indices(app) {
                    let Some(p) = app.placements.get_mut(idx) else {
                        continue;
                    };
                    p.attrs.remove("alpha");
                    rebuild = true;
                }
            }
        }
        InspectorField::ElementScale => {
            if !value.is_empty() {
                let Some(scale) = parse_finite_f32(&value) else {
                    app.status_message = "Scale must be a finite number".to_string();
                    return;
                };
                if scale <= 0.0 {
                    app.status_message = "Scale must be greater than 0".to_string();
                    return;
                }
                for idx in selected_live_indices(app) {
                    let Some(p) = app.placements.get_mut(idx) else {
                        continue;
                    };
                    p.attrs.insert("scale".to_string(), fmt_f32(scale, 3));
                    rebuild = true;
                }
            } else {
                for idx in selected_live_indices(app) {
                    let Some(p) = app.placements.get_mut(idx) else {
                        continue;
                    };
                    p.attrs.remove("scale");
                    rebuild = true;
                }
            }
        }
        InspectorField::PhysicsSimulated
        | InspectorField::PhysicsMass
        | InspectorField::PhysicsTurnMass
        | InspectorField::PhysicsAirResistance
        | InspectorField::PhysicsElasticity
        | InspectorField::PhysicsBuoyancy
        | InspectorField::PhysicsCenterOfMassX
        | InspectorField::PhysicsCenterOfMassY
        | InspectorField::PhysicsCenterOfMassZ => {
            let key = physics_attr_for_inspector_field(edit.field)
                .expect("physics inspector field must have an attribute");
            let normalized = if edit.field == InspectorField::PhysicsSimulated {
                if value.is_empty() {
                    String::new()
                } else if value.eq_ignore_ascii_case("true") || value == "1" {
                    "true".to_string()
                } else if value.eq_ignore_ascii_case("false") || value == "0" {
                    "false".to_string()
                } else {
                    app.status_message = "Simulation must be true, false, or unset".to_string();
                    return;
                }
            } else if value.is_empty() {
                String::new()
            } else {
                let Ok(parsed) = value.parse::<f32>() else {
                    app.status_message = "Physics values must be numbers".to_string();
                    return;
                };
                if !parsed.is_finite() {
                    app.status_message = "Physics values must be finite".to_string();
                    return;
                }
                if matches!(
                    edit.field,
                    InspectorField::PhysicsMass | InspectorField::PhysicsTurnMass
                ) && parsed <= 0.0
                {
                    app.status_message = "Mass values must be greater than 0".to_string();
                    return;
                }
                if matches!(
                    edit.field,
                    InspectorField::PhysicsAirResistance
                        | InspectorField::PhysicsElasticity
                        | InspectorField::PhysicsBuoyancy
                ) && parsed < 0.0
                {
                    app.status_message =
                        "Resistance, elasticity, and buoyancy cannot be negative".to_string();
                    return;
                }
                fmt_f32(parsed, 6)
            };

            let conversion_indices = if normalized.is_empty() {
                Vec::new()
            } else {
                physics_conversion_indices(app)
            };
            if !conversion_indices.is_empty() {
                let writable_definition_ids = if app.physics_scope == PhysicsScope::Global
                    && selected_definitions_include_readonly(app)
                {
                    selected_definition_ids(app)
                        .into_iter()
                        .filter(|id| app.readonly_definition_ids.contains(id))
                        .collect()
                } else {
                    Vec::new()
                };
                let detail = physics_conversion_detail(app, conversion_indices.len());
                app.confirm_dialog = Some(ConfirmDialog {
                    action: ConfirmAction::ApplyPhysicsEdit {
                        edit: InspectorEdit {
                            buffer: normalized,
                            cursor: 0,
                            selection_anchor: None,
                            ..edit
                        },
                        conversion_indices,
                        writable_definition_ids,
                    },
                    title: "Convert Elements for Physics?".to_string(),
                    body: "MTA physics requires object elements. Apply physics and convert the affected elements?"
                        .to_string(),
                    detail,
                    primary_label: "Convert & Apply".to_string(),
                    secondary_label: None,
                    secondary_action: None,
                });
                app.status_message =
                    "Confirm element conversion before applying physics".to_string();
                return;
            }
            if app.physics_scope == PhysicsScope::Global
                && selected_definitions_include_readonly(app)
            {
                prompt_definition_override_edit(app, edit.field, normalized, edit.before);
                return;
            }

            match app.physics_scope {
                PhysicsScope::Global => {
                    let ids = ensure_selected_definition_overrides(app);
                    for id in ids {
                        let Some(def) = app.definitions.get_mut(&id) else {
                            continue;
                        };
                        set_optional_attr(&mut def.attrs, key, normalized.clone());
                        mark_definition_override_attr(def, key);
                    }
                }
                PhysicsScope::PerObject => {
                    for idx in selected_live_indices(app) {
                        let Some(placement) = app.placements.get_mut(idx) else {
                            continue;
                        };
                        set_optional_attr(&mut placement.attrs, key, normalized.clone());
                    }
                }
            }
            app.status_message =
                format!("Updated {} physics for {}", app.physics_scope.label(), key);
        }
        InspectorField::DefinitionDff => {
            for id in ensure_selected_definition_overrides(app) {
                let dff_override = dff_override_stem(&id, Some(&value));
                let dff_value = dff_override.clone().unwrap_or_else(|| id.clone());
                if let Some(def) = app.definitions.get_mut(&id) {
                    if let Some(dff_override) = dff_override {
                        def.attrs.insert("dff".to_string(), dff_override);
                    } else {
                        def.attrs.remove("dff");
                    }
                    mark_definition_override_attr(def, "dff");
                }
                for placement in &mut app.placements {
                    if placement.id == id {
                        placement.dff = dff_value.clone();
                    }
                }
                for idx in 0..app.placements.len() {
                    if app.placements[idx].id == id {
                        invalidate_outliner_label(app, idx);
                    }
                }
                rebuild = true;
            }
        }
        InspectorField::DefinitionNativeModel => {
            let ids = ensure_selected_definition_overrides(app);
            for id in ids {
                let Some(def) = app.definitions.get_mut(&id) else {
                    continue;
                };
                set_optional_attr(&mut def.attrs, "nativeModel", value.clone());
                mark_definition_override_attr(def, "nativeModel");
            }
            app.status_message =
                "Native behavior model will take effect after saving and reloading the map"
                    .to_string();
        }
        InspectorField::DefinitionTxd => {
            for id in ensure_selected_definition_overrides(app) {
                let Some(def) = app.definitions.get_mut(&id) else {
                    continue;
                };
                def.attrs.insert("txd".to_string(), value.clone());
                mark_definition_override_attr(def, "txd");
            }
        }
        InspectorField::DefinitionCol => {
            for id in ensure_selected_definition_overrides(app) {
                let Some(def) = app.definitions.get_mut(&id) else {
                    continue;
                };
                def.attrs.insert("col".to_string(), value.clone());
                mark_definition_override_attr(def, "col");
            }
        }
        InspectorField::DefinitionLod => {
            let lod_distance = if value.is_empty() {
                String::new()
            } else {
                let Some(distance) = parse_finite_f32(&value) else {
                    app.status_message = "LOD distance must be a finite number".to_string();
                    return;
                };
                if distance < 0.0 {
                    app.status_message = "LOD distance cannot be negative".to_string();
                    return;
                }
                distance.to_string()
            };
            for id in ensure_selected_definition_overrides(app) {
                let Some(def) = app.definitions.get_mut(&id) else {
                    continue;
                };
                set_optional_attr(&mut def.attrs, "lodDistance", lod_distance.clone());
                mark_definition_override_attr(def, "lodDistance");
            }
        }
        InspectorField::DefinitionTimeIn => {
            let time = if value.is_empty() {
                String::new()
            } else {
                let Ok(time) = value.parse::<u8>() else {
                    app.status_message = "Time In must be a whole hour from 0 to 24".to_string();
                    return;
                };
                if time > 24 {
                    app.status_message = "Time In must be a whole hour from 0 to 24".to_string();
                    return;
                }
                time.to_string()
            };
            for id in ensure_selected_definition_overrides(app) {
                let Some(def) = app.definitions.get_mut(&id) else {
                    continue;
                };
                set_optional_attr(&mut def.attrs, "timeIn", time.clone());
                mark_definition_override_attr(def, "timeIn");
            }
        }
        InspectorField::DefinitionTimeOut => {
            let time = if value.is_empty() {
                String::new()
            } else {
                let Ok(time) = value.parse::<u8>() else {
                    app.status_message = "Time Out must be a whole hour from 0 to 24".to_string();
                    return;
                };
                if time > 24 {
                    app.status_message = "Time Out must be a whole hour from 0 to 24".to_string();
                    return;
                }
                time.to_string()
            };
            for id in ensure_selected_definition_overrides(app) {
                let Some(def) = app.definitions.get_mut(&id) else {
                    continue;
                };
                set_optional_attr(&mut def.attrs, "timeOut", time.clone());
                mark_definition_override_attr(def, "timeOut");
            }
        }
        InspectorField::CollisionPrimitiveRotX
        | InspectorField::CollisionPrimitiveRotY
        | InspectorField::CollisionPrimitiveRotZ => {
            let Some(parsed) = parse_finite_f32(&value) else {
                app.status_message = "COL box rotation must be a finite number".to_string();
                return;
            };
            if app.active_tab != AppTab::Editing {
                return;
            }
            let Some(mut rotation) = selected_editing_col_primitive_rotation(app) else {
                app.status_message = "Select a COL box first".to_string();
                return;
            };
            match edit.field {
                InspectorField::CollisionPrimitiveRotX => rotation.x = parsed,
                InspectorField::CollisionPrimitiveRotY => rotation.y = parsed,
                InspectorField::CollisionPrimitiveRotZ => rotation.z = parsed,
                _ => unreachable!(),
            }
            if let Err(err) = set_selected_editing_col_box_rotation(app, rotation) {
                app.status_message = err;
            } else {
                app.status_message = if rotation == V3::default() {
                    "Reinstated native axis-aligned COL box".to_string()
                } else {
                    "Saved rotated COL box as closed six-plane geometry".to_string()
                };
                commit_scoped_history(app, "Rotate COL Box", edit.before);
            }
            return;
        }
        InspectorField::SnapMove => {
            if let Some(step) = parse_finite_f32(&value) {
                app.snap_move = step.abs().max(0.001);
                app.status_message = format!("Move snap {:.3}", app.snap_move);
            } else {
                app.status_message = "Move snap must be a finite number".to_string();
            }
            return;
        }
        InspectorField::SnapRotate => {
            if let Some(step) = parse_finite_f32(&value) {
                app.snap_rotate = step.abs().max(0.001);
                app.status_message = format!("Rotate snap {:.3}", app.snap_rotate);
            } else {
                app.status_message = "Rotate snap must be a finite number".to_string();
            }
            return;
        }
        InspectorField::EagleOffsetX
        | InspectorField::EagleOffsetY
        | InspectorField::EagleOffsetZ
        | InspectorField::EagleWaterOffsetX
        | InspectorField::EagleWaterOffsetY
        | InspectorField::EagleWaterOffsetZ => {
            let Some(parsed) = parse_finite_f32(&value) else {
                app.status_message = "eagleZones offsets must be finite numbers".to_string();
                return;
            };
            let offset = match edit.field {
                InspectorField::EagleOffsetX
                | InspectorField::EagleOffsetY
                | InspectorField::EagleOffsetZ => {
                    app.eagle_zone_offsets.offset.get_or_insert_default()
                }
                InspectorField::EagleWaterOffsetX
                | InspectorField::EagleWaterOffsetY
                | InspectorField::EagleWaterOffsetZ => {
                    app.eagle_zone_offsets.water_offset.get_or_insert_default()
                }
                _ => unreachable!(),
            };
            match edit.field {
                InspectorField::EagleOffsetX | InspectorField::EagleWaterOffsetX => {
                    offset.x = parsed;
                }
                InspectorField::EagleOffsetY | InspectorField::EagleWaterOffsetY => {
                    offset.y = parsed;
                }
                InspectorField::EagleOffsetZ | InspectorField::EagleWaterOffsetZ => {
                    offset.z = parsed;
                }
                _ => unreachable!(),
            }
            app.status_message = "eagleZones offset updated".to_string();
            return;
        }
        InspectorField::GlobalOffsetX
        | InspectorField::GlobalOffsetY
        | InspectorField::GlobalOffsetZ
        | InspectorField::GlobalRotationX
        | InspectorField::GlobalRotationY
        | InspectorField::GlobalRotationZ => {
            let Some(parsed) = parse_finite_f32(&value) else {
                app.status_message = "Global transform values must be finite numbers".to_string();
                return;
            };
            match edit.field {
                InspectorField::GlobalOffsetX => app.global_transform.offset.x = parsed,
                InspectorField::GlobalOffsetY => app.global_transform.offset.y = parsed,
                InspectorField::GlobalOffsetZ => app.global_transform.offset.z = parsed,
                InspectorField::GlobalRotationX => app.global_transform.rotation.x = parsed,
                InspectorField::GlobalRotationY => app.global_transform.rotation.y = parsed,
                InspectorField::GlobalRotationZ => app.global_transform.rotation.z = parsed,
                _ => unreachable!(),
            }
            app.status_message = "Global transform updated".to_string();
            return;
        }
        InspectorField::DffMaterialRed
        | InspectorField::DffMaterialGreen
        | InspectorField::DffMaterialBlue
        | InspectorField::DffMaterialAlpha
        | InspectorField::DffMaterialAmbient
        | InspectorField::DffMaterialDiffuse
        | InspectorField::DffMaterialSpecular => {
            let Some(parsed) = parse_finite_f32(&value) else {
                app.status_message = "Material values must be finite numbers".to_string();
                return;
            };
            let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() else {
                return;
            };
            let material_index = dff.selected_material;
            let material_count = dff_material_slot_count(&dff.raw).max(material_index + 1);
            let slots_were_aligned = dff.raw.material_textures.len() == material_count
                && dff.raw.materials.len() == material_count
                && dff.raw.material_animations.len() == material_count;
            ensure_dff_material_slots(&mut dff.raw, material_count);
            let material = &mut dff.raw.materials[material_index];
            if !apply_dff_material_value(material, edit.field, parsed) && slots_were_aligned {
                app.status_message = "DFF material value is unchanged".to_string();
                return;
            }
            dff.dirty = true;
            refresh_editing_dff_preview(app);
            app.status_message = format!("Updated DFF material #{material_index:02}");
            commit_scoped_history(app, "Edit DFF Material", edit.before);
            return;
        }
        InspectorField::DffEmitterStrength => {
            let Some(strength) = parse_finite_f32(&value) else {
                app.status_message = "Emitter brightness must be a finite number".to_string();
                return;
            };
            if update_selected_emitters(app, |emitter| {
                emitter.strength = strength.clamp(0.0, 100.0)
            }) {
                persist_material_emitter_edit(app, "Updated emitter brightness");
            }
            return;
        }
        InspectorField::DffEmitterFalloff => {
            let Some(distance) = parse_finite_f32(&value) else {
                app.status_message = "Emitter falloff distance must be a finite number".to_string();
                return;
            };
            if update_selected_emitters(app, |emitter| {
                emitter.falloff_distance = distance.clamp(1.0, 100_000.0)
            }) {
                persist_material_emitter_edit(app, "Updated emitter falloff distance");
            }
            return;
        }
        InspectorField::DffEmitterMaxGroupingSize => {
            let Some(size) = parse_finite_f32(&value) else {
                app.status_message = "Maximum grouping size must be a finite number".to_string();
                return;
            };
            if update_selected_emitters(app, |emitter| {
                emitter.max_grouping_size = size.clamp(0.01, 100_000.0)
            }) {
                persist_material_emitter_edit(app, "Updated emitter maximum grouping size");
            }
            return;
        }
        InspectorField::DffEmitterPointUpStrength => {
            let Some(strength) = parse_finite_f32(&value) else {
                app.status_message = "Point Up strength must be a finite number".to_string();
                return;
            };
            if update_selected_emitters(app, |emitter| {
                emitter.point_up_strength = strength.clamp(0.0, 100.0)
            }) {
                persist_material_emitter_edit(app, "Updated Point Up strength");
            }
            return;
        }
        InspectorField::DffEmitterPointDownStrength => {
            let Some(strength) = parse_finite_f32(&value) else {
                app.status_message = "Point Down strength must be a finite number".to_string();
                return;
            };
            if update_selected_emitters(app, |emitter| {
                emitter.point_down_strength = strength.clamp(0.0, 100.0)
            }) {
                persist_material_emitter_edit(app, "Updated Point Down strength");
            }
            return;
        }
        InspectorField::DffEmitterPointSidesStrength => {
            let Some(strength) = parse_finite_f32(&value) else {
                app.status_message = "Point Sides strength must be a finite number".to_string();
                return;
            };
            if update_selected_emitters(app, |emitter| {
                emitter.point_sides_strength = strength.clamp(0.0, 100.0)
            }) {
                persist_material_emitter_edit(app, "Updated Point Sides strength");
            }
            return;
        }
        InspectorField::DffEmitterTemperature => {
            let Some(temperature) = parse_finite_f32(&value) else {
                app.status_message = "Emitter temperature must be a finite number".to_string();
                return;
            };
            if update_selected_emitters(app, |emitter| {
                emitter.use_material_color = false;
                emitter.use_temperature = true;
                emitter.temperature = temperature.clamp(1000.0, 40000.0);
            }) {
                persist_material_emitter_edit(app, "Updated emitter temperature");
            }
            return;
        }
        InspectorField::CollisionFaceMaterial | InspectorField::CollisionFaceLight => {
            let Ok(parsed) = value.parse::<u8>() else {
                app.status_message = "Collision face values must be 0-255".to_string();
                return;
            };
            if app.active_tab == AppTab::Editing {
                let changed = match edit.field {
                    InspectorField::CollisionFaceMaterial => {
                        set_selected_editing_col_material(app, parsed)
                    }
                    InspectorField::CollisionFaceLight => {
                        set_selected_editing_col_light(app, parsed)
                    }
                    _ => unreachable!(),
                };
                if changed {
                    commit_scoped_history(app, "Edit COL Surface", edit.before);
                }
                return;
            }
            if edit.field == InspectorField::CollisionFaceMaterial {
                match set_selected_collision_tab_material(app, parsed) {
                    Ok(()) => commit_scoped_history(app, "Edit COL Material", edit.before),
                    Err(err) => app.status_message = err,
                }
                return;
            }
            let Some(selected) = app.selected_col_face else {
                app.status_message = "Pick a collision face first".to_string();
                return;
            };
            let Some(key) = app
                .placements
                .get(selected.placement)
                .map(|placement| element_collision_key(app, placement))
            else {
                return;
            };
            let Some(mesh) = app.collisions.get_mut(&key) else {
                app.status_message = "Selected element has no parsed COL mesh".to_string();
                return;
            };
            let Some(face) = mesh.faces.get_mut(selected.face) else {
                app.status_message = "Selected collision face no longer exists".to_string();
                return;
            };
            let (path, offset, label) = match edit.field {
                InspectorField::CollisionFaceMaterial => {
                    face.material = parsed;
                    (face.img_path.clone(), face.material_file_offset, "material")
                }
                InspectorField::CollisionFaceLight => {
                    face.light = parsed;
                    (face.img_path.clone(), face.light_file_offset, "light")
                }
                _ => unreachable!(),
            };
            invalidate_collision_render_cache(app, &key);
            app.pending_col_writes.insert((path, offset), parsed);
            app.status_message = if edit.field == InspectorField::CollisionFaceMaterial {
                format!(
                    "Collision face material set to {}",
                    col_material_label(parsed)
                )
            } else {
                format!("Collision face {label} set to {parsed}")
            };
            commit_scoped_history(app, "Edit COL Surface", edit.before);
            return;
        }
        InspectorField::CollisionVertexX
        | InspectorField::CollisionVertexY
        | InspectorField::CollisionVertexZ => {
            let Some(parsed) = parse_finite_f32(&value) else {
                app.status_message = "COL vertex position must be a finite number".to_string();
                return;
            };
            if app.active_tab == AppTab::Editing {
                let mut current = if let Some(pos) = selected_editing_col_primitive_position(app) {
                    from_mq(pos)
                } else {
                    let Some(EditingAsset::Col(col)) = app.editing.asset.as_ref() else {
                        app.status_message = "Open a COL in Editing first".to_string();
                        return;
                    };
                    let Some(face) = col.mesh.faces.get(col.selected_face) else {
                        app.status_message = "Selected COL face no longer exists".to_string();
                        return;
                    };
                    let vertex_idx =
                        collision_selected_vertex_index_from_face(face, col.selected_vertex);
                    let Some(vertex) = col.mesh.vertices.get(vertex_idx).copied() else {
                        app.status_message = "Selected COL vertex no longer exists".to_string();
                        return;
                    };
                    vertex
                };
                match edit.field {
                    InspectorField::CollisionVertexX => current.x = parsed,
                    InspectorField::CollisionVertexY => current.y = parsed,
                    InspectorField::CollisionVertexZ => current.z = parsed,
                    _ => unreachable!(),
                }
                let result = if selected_editing_col_primitive_position(app).is_some() {
                    set_selected_editing_col_primitive_position(app, current)
                } else {
                    set_selected_editing_col_vertex_position(app, current)
                };
                if let Err(err) = result {
                    app.status_message = err;
                } else {
                    app.status_message = "Editing COL position updated".to_string();
                    commit_scoped_history(app, "Edit COL Position", edit.before);
                }
                return;
            }
            let Some(mut world) = selected_collision_tab_vertex_position(app) else {
                app.status_message = "Pick a collision vertex first".to_string();
                return;
            };
            match edit.field {
                InspectorField::CollisionVertexX => world.x = parsed,
                InspectorField::CollisionVertexY => world.y = parsed,
                InspectorField::CollisionVertexZ => world.z = parsed,
                _ => unreachable!(),
            }
            if let Err(err) = set_selected_collision_tab_vertex_position(app, world) {
                app.status_message = err;
            } else {
                commit_scoped_history(app, "Edit COL Vertex", edit.before);
            }
            return;
        }
        InspectorField::CollisionPrimitiveSizeX
        | InspectorField::CollisionPrimitiveSizeY
        | InspectorField::CollisionPrimitiveSizeZ => {
            let Some(parsed) = parse_finite_f32(&value) else {
                app.status_message = "COL primitive size must be a finite number".to_string();
                return;
            };
            if app.active_tab != AppTab::Editing {
                return;
            }
            let Some(mut size) = selected_editing_col_primitive_size(app) else {
                app.status_message = "Select a COL sphere, box, or capsule first".to_string();
                return;
            };
            match edit.field {
                InspectorField::CollisionPrimitiveSizeX => size.x = parsed,
                InspectorField::CollisionPrimitiveSizeY => size.y = parsed,
                InspectorField::CollisionPrimitiveSizeZ => size.z = parsed,
                _ => unreachable!(),
            }
            if let Err(err) = set_selected_editing_col_primitive_size(app, size) {
                app.status_message = err;
            } else {
                app.status_message = "Editing COL primitive size updated".to_string();
                commit_scoped_history(app, "Resize COL Primitive", edit.before);
            }
            return;
        }
        InspectorField::BakeShadowSamples => {
            if let Ok(samples) = value.parse::<usize>() {
                app.bake_settings.shadow_samples = samples;
                app.bake_settings = clamp_bake_settings(app.bake_settings);
                save_bake_settings_preference(app.bake_settings);
            } else {
                app.status_message = "Shadow samples must be a whole number".to_string();
            }
            return;
        }
        InspectorField::BakeShadowChunks => {
            if let Ok(chunks) = value.parse::<usize>() {
                app.bake_settings.shadow_chunks = chunks;
                app.bake_settings = clamp_bake_settings(app.bake_settings);
                save_bake_settings_preference(app.bake_settings);
            } else {
                app.status_message = "Shadow chunks must be a whole number".to_string();
            }
            return;
        }
        InspectorField::BakeBounces => {
            if let Ok(bounces) = value.parse::<usize>() {
                app.bake_settings.bounces = bounces;
                app.bake_settings = clamp_bake_settings(app.bake_settings);
                save_bake_settings_preference(app.bake_settings);
            } else {
                app.status_message = "Bounces must be a whole number".to_string();
            }
            return;
        }
        InspectorField::BakeBounceStrength => {
            if let Some(strength) = parse_finite_f32(&value) {
                app.bake_settings.bounce_strength = strength;
                app.bake_settings = clamp_bake_settings(app.bake_settings);
                save_bake_settings_preference(app.bake_settings);
            } else {
                app.status_message = "Bounce strength must be a finite number".to_string();
            }
            return;
        }
        InspectorField::BakeBounceMaximum => {
            if let Some(maximum) = parse_finite_f32(&value) {
                app.bake_settings.bounce_maximum = maximum;
                app.bake_settings = clamp_bake_settings(app.bake_settings);
                save_bake_settings_preference(app.bake_settings);
            } else {
                app.status_message = "Bounce maximum must be a finite number".to_string();
            }
            return;
        }
        InspectorField::BakeExposure => {
            if let Some(exposure) = parse_finite_f32(&value) {
                app.bake_settings.exposure = exposure;
                app.bake_settings = clamp_bake_settings(app.bake_settings);
                save_bake_settings_preference(app.bake_settings);
            } else {
                app.status_message = "Exposure must be a finite number".to_string();
            }
            return;
        }
        InspectorField::BakeAmbientBump => {
            if let Some(ambient_bump) = parse_finite_f32(&value) {
                app.bake_settings.ambient_bump = ambient_bump;
                app.bake_settings = clamp_bake_settings(app.bake_settings);
                save_bake_settings_preference(app.bake_settings);
            } else {
                app.status_message = "Ambient bump must be a finite number".to_string();
            }
            return;
        }
        InspectorField::BakeShadowSoftness => {
            if let Some(softness) = parse_finite_f32(&value) {
                app.bake_settings.shadow_softness = softness;
                app.bake_settings = clamp_bake_settings(app.bake_settings);
                save_bake_settings_preference(app.bake_settings);
            } else {
                app.status_message = "Shadow softness must be a finite number".to_string();
            }
            return;
        }
        InspectorField::BakeAoSamples => {
            if let Ok(samples) = value.parse::<usize>() {
                app.bake_settings.ao_samples = samples;
                app.bake_settings = clamp_bake_settings(app.bake_settings);
                save_bake_settings_preference(app.bake_settings);
            } else {
                app.status_message = "AO samples must be a whole number".to_string();
            }
            return;
        }
        InspectorField::BakeAoRadius => {
            if let Some(radius) = parse_finite_f32(&value) {
                app.bake_settings.ao_radius = radius;
                app.bake_settings = clamp_bake_settings(app.bake_settings);
                save_bake_settings_preference(app.bake_settings);
            } else {
                app.status_message = "AO radius must be a finite number".to_string();
            }
            return;
        }
        InspectorField::BakeAoStrength => {
            if let Some(strength) = parse_finite_f32(&value) {
                app.bake_settings.ao_strength = strength;
                app.bake_settings = clamp_bake_settings(app.bake_settings);
                save_bake_settings_preference(app.bake_settings);
            } else {
                app.status_message = "AO strength must be a finite number".to_string();
            }
            return;
        }
        InspectorField::DayNightMergeTolerance => {
            if let Some(tolerance) = parse_finite_f32(&value) {
                app.bake_settings.day_night_merge_tolerance = tolerance;
                app.bake_settings = clamp_bake_settings(app.bake_settings);
                save_bake_settings_preference(app.bake_settings);
            } else {
                app.status_message =
                    "Variant position tolerance must be a finite number".to_string();
            }
            return;
        }
        InspectorField::VertexPaintTemperature => {
            if let Some(temperature) = parse_finite_f32(&value) {
                app.vertex_paint.temperature = temperature.clamp(1000.0, 40000.0);
                app.vertex_paint.color = color_from_temperature(app.vertex_paint.temperature);
                save_vertex_paint_settings_preference(app.vertex_paint);
            } else {
                app.status_message = "Paint temperature must be a finite number".to_string();
            }
            return;
        }
        InspectorField::VertexPaintRadius => {
            if let Some(radius) = parse_finite_f32(&value) {
                app.vertex_paint.radius = radius.clamp(1.0, 4096.0);
                save_vertex_paint_settings_preference(app.vertex_paint);
            } else {
                app.status_message = "Paint radius must be a finite number".to_string();
            }
            return;
        }
        InspectorField::VertexPaintStrength => {
            if let Some(strength) = parse_finite_f32(&value) {
                app.vertex_paint.strength = strength.clamp(0.0, 1.0);
                save_vertex_paint_settings_preference(app.vertex_paint);
            } else {
                app.status_message = "Paint strength must be a finite number".to_string();
            }
            return;
        }
        InspectorField::LightName => {
            if let Some(light) = app.lights.get_mut(app.selected_light) {
                light.name = value;
                mark_lights_changed(app);
            }
            commit_scoped_history(app, "Edit Light Name", edit.before);
            return;
        }
        InspectorField::LightKind => {
            if let Some(light) = app.lights.get_mut(app.selected_light) {
                light.kind = parse_light_kind(&value);
                mark_lights_changed(app);
            }
            commit_scoped_history(app, "Edit Light Kind", edit.before);
            return;
        }
        InspectorField::LightProfile => {
            if let Some(light) = app.lights.get_mut(app.selected_light) {
                light.profile = parse_light_profile(&value);
                mark_lights_changed(app);
            }
            commit_scoped_history(app, "Edit Light Profile", edit.before);
            return;
        }
        InspectorField::LightPosition => {
            if let Some(light) = app.lights.get_mut(app.selected_light) {
                light.position = parse_vec3_text(&value, light.position);
                mark_lights_changed(app);
            }
            commit_scoped_history(app, "Edit Light Position", edit.before);
            return;
        }
        InspectorField::LightDirection => {
            if let Some(light) = app.lights.get_mut(app.selected_light) {
                light.direction = parse_vec3_text(&value, light.direction);
                mark_lights_changed(app);
            }
            commit_scoped_history(app, "Edit Light Direction", edit.before);
            return;
        }
        InspectorField::LightTemperature => {
            if let Some(temperature) = parse_finite_f32(&value) {
                if let Some(light) = app.lights.get_mut(app.selected_light) {
                    light.temperature = temperature.clamp(1000.0, 40000.0);
                    mark_lights_changed(app);
                }
            } else {
                app.status_message = "Light temperature must be a finite number".to_string();
            }
            commit_scoped_history(app, "Edit Light Temperature", edit.before);
            return;
        }
        InspectorField::LightIntensity => {
            if let Some(intensity) = parse_finite_f32(&value) {
                if let Some(light) = app.lights.get_mut(app.selected_light) {
                    light.intensity = intensity.max(0.0);
                    mark_lights_changed(app);
                }
            } else {
                app.status_message = "Light intensity must be a finite number".to_string();
            }
            commit_scoped_history(app, "Edit Light Intensity", edit.before);
            return;
        }
        InspectorField::LightRadius => {
            if let Some(radius) = parse_finite_f32(&value) {
                if let Some(light) = app.lights.get_mut(app.selected_light) {
                    light.radius = radius.max(0.0);
                    mark_lights_changed(app);
                }
            } else {
                app.status_message = "Light radius must be a finite number".to_string();
            }
            commit_scoped_history(app, "Edit Light Radius", edit.before);
            return;
        }
        InspectorField::WaterMinX
        | InspectorField::WaterMinY
        | InspectorField::WaterMaxX
        | InspectorField::WaterMaxY
        | InspectorField::WaterHeight => {
            let Some(parsed) = parse_finite_f32(&value) else {
                app.status_message = "Water plane values must be finite numbers".to_string();
                return;
            };
            if let Some(plane) = selected_water_plane_mut(app) {
                let (mut min_x, mut min_y, mut max_x, mut max_y, mut z) = water_plane_bounds(plane);
                match edit.field {
                    InspectorField::WaterMinX => min_x = parsed,
                    InspectorField::WaterMinY => min_y = parsed,
                    InspectorField::WaterMaxX => max_x = parsed,
                    InspectorField::WaterMaxY => max_y = parsed,
                    InspectorField::WaterHeight => z = parsed,
                    _ => {}
                }
                if min_x > max_x {
                    std::mem::swap(&mut min_x, &mut max_x);
                }
                if min_y > max_y {
                    std::mem::swap(&mut min_y, &mut max_y);
                }
                set_water_plane_rect(plane, min_x, min_y, max_x, max_y, z);
                app.status_message = format!("Edited water plane {}", app.selected_water + 1);
                commit_scoped_history(app, "Edit water plane", edit.before);
            }
            return;
        }
        InspectorField::WaterType => {
            let Ok(kind) = value.parse::<i32>() else {
                app.status_message = "Water type must be a whole number".to_string();
                return;
            };
            if let Some(plane) = selected_water_plane_mut(app) {
                plane.kind = kind;
                app.status_message = format!("Edited water plane {}", app.selected_water + 1);
                commit_scoped_history(app, "Edit water plane", edit.before);
            }
            return;
        }
        InspectorField::CullPosX
        | InspectorField::CullPosY
        | InspectorField::CullPosZ
        | InspectorField::CullSizeX
        | InspectorField::CullSizeY
        | InspectorField::CullSizeZ => {
            let Some(parsed) = parse_finite_f32(&value) else {
                app.status_message = "Cull zone values must be finite numbers".to_string();
                return;
            };
            let selected = app.selected_cull;
            if let Some(zone) = app.cull_zones.get_mut(selected) {
                match edit.field {
                    InspectorField::CullPosX => zone.center.x = parsed,
                    InspectorField::CullPosY => zone.center.y = parsed,
                    InspectorField::CullPosZ => zone.center.z = parsed,
                    InspectorField::CullSizeX => zone.size.x = parsed.abs().max(0.1),
                    InspectorField::CullSizeY => zone.size.y = parsed.abs().max(0.1),
                    InspectorField::CullSizeZ => zone.size.z = parsed.abs().max(0.1),
                    _ => {}
                }
                app.status_message = format!("Edited water cull zone {}", selected + 1);
            }
            return;
        }
    }
    if rebuild {
        invalidate_outliner_labels(app);
        rebuild_outliner_filter(app);
        rebuild_render_cells(app);
    }
    commit_scoped_history(app, "Inspector Edit", edit.before);
}

pub(crate) fn clamp_char_boundary(value: &str, cursor: usize) -> usize {
    let mut cursor = cursor.min(value.len());
    while cursor > 0 && !value.is_char_boundary(cursor) {
        cursor -= 1;
    }
    cursor
}

pub(crate) fn prev_char_boundary(value: &str, cursor: usize) -> usize {
    let cursor = clamp_char_boundary(value, cursor);
    value[..cursor]
        .char_indices()
        .last()
        .map(|(idx, _)| idx)
        .unwrap_or(0)
}

pub(crate) fn next_char_boundary(value: &str, cursor: usize) -> usize {
    let cursor = clamp_char_boundary(value, cursor);
    if cursor >= value.len() {
        return value.len();
    }
    value[cursor..]
        .char_indices()
        .nth(1)
        .map(|(idx, _)| cursor + idx)
        .unwrap_or(value.len())
}

pub(crate) fn ctrl_down() -> bool {
    is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl)
}

pub(crate) fn selection_range(
    value: &str,
    cursor: usize,
    anchor: Option<usize>,
) -> Option<(usize, usize)> {
    let anchor = clamp_char_boundary(value, anchor?);
    let cursor = clamp_char_boundary(value, cursor);
    if anchor == cursor {
        None
    } else {
        Some((anchor.min(cursor), anchor.max(cursor)))
    }
}

pub(crate) fn request_element_id_rename(
    app: &mut AppState,
    before: ScopedHistorySnapshot,
    value: String,
) {
    let new_id = value.trim().to_string();
    let Some(selected_idx) = app.placements.get(app.selected).map(|_| app.selected) else {
        return;
    };
    let selected_indices = selected_live_indices(app);
    let selected_ids: BTreeSet<_> = selected_indices
        .iter()
        .filter_map(|idx| app.placements.get(*idx).map(|placement| &placement.id))
        .collect();
    if selected_ids.len() > 1 {
        app.status_message = "Model ID rename requires all selected elements to share the same ID; no elements were changed".to_string();
        return;
    }
    if new_id.is_empty() {
        app.status_message = "Element ID cannot be empty".to_string();
        return;
    }
    let old_id = app.placements[selected_idx].id.clone();
    if new_id == old_id {
        return;
    }
    if app.definitions.contains_key(&new_id)
        || app
            .placements
            .iter()
            .any(|placement| placement.id == new_id)
    {
        app.status_message = format!("ID {new_id} already exists");
        return;
    }
    app.element_id_rename_dialog = Some(ElementIdRenameDialog {
        selected_idx,
        selected_indices,
        old_id,
        new_id,
        before,
    });
}

#[derive(Clone, Copy)]
pub(crate) enum ElementIdRenameMode {
    RenameAssets,
    KeepOldAssets,
    MakeUnique,
}

pub(crate) fn apply_element_id_rename(app: &mut AppState, mode: ElementIdRenameMode) {
    let Some(dialog) = app.element_id_rename_dialog.take() else {
        return;
    };
    if dialog.selected_idx >= app.placements.len() {
        return;
    }
    let old_id = dialog.old_id;
    let new_id = dialog.new_id;
    let old_def = app
        .definitions
        .get(&old_id)
        .cloned()
        .unwrap_or_else(|| Definition {
            id: old_id.clone(),
            zone: app.placements[dialog.selected_idx].zone.clone(),
            attrs: BTreeMap::new(),
        });
    let mut new_def = old_def.clone();
    new_def.id = new_id.clone();
    new_def.attrs.insert("id".to_string(), new_id.clone());
    match mode {
        ElementIdRenameMode::RenameAssets => {
            let asset_root = replacement_asset_root(app);
            let _ = rename_replacement_archive_entry(
                &asset_root,
                &with_ext(&old_id, ".dff"),
                &with_ext(&new_id, ".dff"),
            );
            let _ = rename_replacement_archive_entry(
                &asset_root,
                &with_ext(&old_id, ".col"),
                &with_ext(&new_id, ".col"),
            );
            new_def.attrs.remove("dff");
            new_def.attrs.insert("col".to_string(), new_id.clone());
            app.definitions.remove(&old_id);
            app.definitions.insert(new_id.clone(), new_def);
            for placement in &mut app.placements {
                if placement.id == old_id {
                    placement.id = new_id.clone();
                    placement.dff = new_id.clone();
                    sync_placement_attrs(placement);
                }
            }
        }
        ElementIdRenameMode::KeepOldAssets => {
            new_def.attrs.insert("dff".to_string(), old_id.clone());
            new_def.attrs.insert("col".to_string(), old_id.clone());
            app.definitions.remove(&old_id);
            app.definitions.insert(new_id.clone(), new_def);
            for placement in &mut app.placements {
                if placement.id == old_id {
                    placement.id = new_id.clone();
                    placement.dff = old_id.clone();
                    sync_placement_attrs(placement);
                }
            }
        }
        ElementIdRenameMode::MakeUnique => {
            new_def.attrs.insert("dff".to_string(), old_id.clone());
            new_def.attrs.insert("col".to_string(), old_id.clone());
            app.definitions.insert(new_id.clone(), new_def);
            for idx in dialog.selected_indices.iter().copied() {
                let Some(placement) = app.placements.get_mut(idx) else {
                    continue;
                };
                placement.id = new_id.clone();
                placement.dff = old_id.clone();
                sync_placement_attrs(placement);
            }
        }
    }
    for placement in &mut app.placements {
        if placement
            .attrs
            .get("lodParent")
            .is_some_and(|lod_parent| lod_parent.eq_ignore_ascii_case(&old_id))
        {
            placement
                .attrs
                .insert("lodParent".to_string(), new_id.clone());
        }
    }
    for idx in 0..app.placements.len() {
        invalidate_outliner_label(app, idx);
    }
    invalidate_validation_cache(app);
    rebuild_render_cells(app);
    commit_scoped_history(app, "Rename Element ID", dialog.before);
    app.status_message = match mode {
        ElementIdRenameMode::RenameAssets => {
            format!("Renamed ID {old_id} to {new_id}; DFF/COL now follow the new ID")
        }
        ElementIdRenameMode::KeepOldAssets => {
            format!("Renamed ID {old_id} to {new_id}; kept DFF/COL on {old_id}")
        }
        ElementIdRenameMode::MakeUnique => {
            format!(
                "Made {} selected element(s) {new_id}; DFF/COL stay on {old_id}",
                dialog.selected_indices.len()
            )
        }
    };
}

pub(crate) fn delete_text_selection(
    value: &mut String,
    cursor: &mut usize,
    anchor: &mut Option<usize>,
) -> bool {
    *cursor = clamp_char_boundary(value, *cursor);
    let Some((start, end)) = selection_range(value, *cursor, *anchor) else {
        return false;
    };
    value.replace_range(start..end, "");
    *cursor = start;
    *anchor = None;
    true
}

const MAX_EDITABLE_TEXT_BYTES: usize = 16 * 1024;
const CLIPBOARD_PASTE_TIMEOUT: Duration = Duration::from_secs(6);

struct PendingClipboardPaste {
    target: usize,
    expected_value: String,
    expected_cursor: usize,
    expected_anchor: Option<usize>,
    started_at: Instant,
    rx: mpsc::Receiver<Result<String, String>>,
}

static PENDING_CLIPBOARD_PASTE: OnceLock<Mutex<Option<PendingClipboardPaste>>> = OnceLock::new();
static CLIPBOARD_READ_ACTIVE: AtomicBool = AtomicBool::new(false);

fn pending_clipboard_paste() -> &'static Mutex<Option<PendingClipboardPaste>> {
    PENDING_CLIPBOARD_PASTE.get_or_init(|| Mutex::new(None))
}

fn lock_pending_clipboard_paste() -> std::sync::MutexGuard<'static, Option<PendingClipboardPaste>> {
    pending_clipboard_paste()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn text_field_target(value: &mut String) -> usize {
    value as *mut String as usize
}

fn bounded_editable_text(text: &str, max_bytes: usize) -> String {
    let mut out = String::with_capacity(text.len().min(max_bytes));
    for ch in text.chars().filter(|ch| !ch.is_control()) {
        if out.len().saturating_add(ch.len_utf8()) > max_bytes {
            break;
        }
        out.push(ch);
    }
    out
}

fn request_clipboard_paste(value: &mut String, cursor: &mut usize, anchor: &mut Option<usize>) {
    let target = text_field_target(value);
    let mut pending = lock_pending_clipboard_paste();
    if pending.as_ref().is_some_and(|request| {
        request.target == target
            && request.expected_value == *value
            && request.expected_cursor == *cursor
            && request.expected_anchor == *anchor
    }) {
        return;
    }
    if CLIPBOARD_READ_ACTIVE.swap(true, Ordering::AcqRel) {
        return;
    }
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = std::panic::catch_unwind(|| {
            read_system_clipboard_text()
                .map(|text| bounded_editable_text(&text, MAX_EDITABLE_TEXT_BYTES))
        })
        .unwrap_or_else(|_| Err("Clipboard worker stopped unexpectedly".to_string()));
        let _ = tx.send(result);
        CLIPBOARD_READ_ACTIVE.store(false, Ordering::Release);
    });
    *pending = Some(PendingClipboardPaste {
        target,
        expected_value: value.clone(),
        expected_cursor: *cursor,
        expected_anchor: *anchor,
        started_at: Instant::now(),
        rx,
    });
}

fn poll_clipboard_paste(
    value: &mut String,
    cursor: &mut usize,
    anchor: &mut Option<usize>,
) -> bool {
    let target = text_field_target(value);
    let mut pending = lock_pending_clipboard_paste();
    let Some(request) = pending.as_ref() else {
        return false;
    };
    if request.started_at.elapsed() > CLIPBOARD_PASTE_TIMEOUT {
        *pending = None;
        return false;
    }
    if request.target != target {
        return false;
    }
    let result = match request.rx.try_recv() {
        Ok(result) => result,
        Err(mpsc::TryRecvError::Empty) => return false,
        Err(mpsc::TryRecvError::Disconnected) => {
            *pending = None;
            return false;
        }
    };
    let request = pending.take().expect("pending clipboard request exists");
    if request.expected_value != *value
        || request.expected_cursor != *cursor
        || request.expected_anchor != *anchor
    {
        return false;
    }
    result
        .ok()
        .is_some_and(|text| insert_text_at_cursor(value, cursor, anchor, &text))
}

pub(crate) fn insert_text_at_cursor(
    value: &mut String,
    cursor: &mut usize,
    anchor: &mut Option<usize>,
    text: &str,
) -> bool {
    *cursor = clamp_char_boundary(value, *cursor);
    let selected_bytes = selection_range(value, *cursor, *anchor)
        .map(|(start, end)| end - start)
        .unwrap_or(0);
    let retained_bytes = value.len().saturating_sub(selected_bytes);
    let remaining = MAX_EDITABLE_TEXT_BYTES.saturating_sub(retained_bytes);
    let text = bounded_editable_text(text, remaining);
    if text.is_empty() {
        return false;
    }
    delete_text_selection(value, cursor, anchor);
    *cursor = clamp_char_boundary(value, *cursor);
    value.insert_str(*cursor, &text);
    *cursor += text.len();
    true
}

pub(crate) fn copy_text_to_clipboard(value: &str, cursor: usize, anchor: Option<usize>) {
    if let Some((start, end)) = selection_range(value, cursor, anchor) {
        let _ = set_system_clipboard(&value[start..end]);
    } else if !value.is_empty() {
        let _ = set_system_clipboard(value);
    }
}

pub(crate) fn handle_text_clipboard_shortcuts(
    value: &mut String,
    cursor: &mut usize,
    anchor: &mut Option<usize>,
) -> bool {
    if poll_clipboard_paste(value, cursor, anchor) {
        return true;
    }
    if !ctrl_down() {
        return false;
    }
    if is_key_pressed(KeyCode::A) {
        *cursor = value.len();
        *anchor = Some(0);
        return true;
    }
    if is_key_pressed(KeyCode::C) {
        copy_text_to_clipboard(value, *cursor, *anchor);
        return true;
    }
    if is_key_pressed(KeyCode::V) {
        request_clipboard_paste(value, cursor, anchor);
        return true;
    }
    false
}

pub(crate) fn handle_text_control_char(
    value: &mut String,
    cursor: &mut usize,
    anchor: &mut Option<usize>,
    ch: char,
) -> bool {
    match ch {
        '\u{1}' => {
            *cursor = value.len();
            *anchor = Some(0);
            true
        }
        '\u{3}' => {
            copy_text_to_clipboard(value, *cursor, *anchor);
            true
        }
        '\u{16}' => {
            request_clipboard_paste(value, cursor, anchor);
            true
        }
        _ => false,
    }
}

pub(crate) fn draw_visible_text_selection(
    value: &str,
    cursor: usize,
    anchor: Option<usize>,
    start_char: usize,
    visible: &str,
    text_x: f32,
    rect: Rect,
) {
    let Some((start, end)) = selection_range(value, cursor, anchor) else {
        return;
    };
    let start_sel_char = value[..start].chars().count();
    let end_sel_char = value[..end].chars().count();
    let visible_chars = visible.chars().count();
    let visible_start = start_char;
    let visible_end = start_char + visible_chars;
    let sel_start = start_sel_char.max(visible_start).min(visible_end);
    let sel_end = end_sel_char.max(visible_start).min(visible_end);
    if sel_start >= sel_end {
        return;
    }
    let before: String = visible
        .chars()
        .take(sel_start.saturating_sub(visible_start))
        .collect();
    let selected: String = visible
        .chars()
        .skip(sel_start.saturating_sub(visible_start))
        .take(sel_end - sel_start)
        .collect();
    let x = text_x + ui_text_width(&before, 16).round();
    let w = ui_text_width(&selected, 16).round().max(2.0);
    draw_rectangle(
        x,
        rect.y + 6.0,
        w,
        (rect.h - 12.0).max(1.0),
        Color::new(0.20, 0.42, 0.72, 0.65),
    );
}

pub(crate) fn prev_word_boundary(value: &str, cursor: usize) -> usize {
    let mut idx = clamp_char_boundary(value, cursor);
    while idx > 0 {
        let prev = prev_char_boundary(value, idx);
        if !value[prev..idx].chars().all(char::is_whitespace) {
            break;
        }
        idx = prev;
    }
    while idx > 0 {
        let prev = prev_char_boundary(value, idx);
        if value[prev..idx].chars().all(char::is_whitespace) {
            break;
        }
        idx = prev;
    }
    idx
}

pub(crate) fn set_edit_cursor_from_mouse(edit: &mut InspectorEdit, mouse_x: f32, rect: Rect) {
    let local_x = (mouse_x - rect.x - 10.0).max(0.0);
    let mut best = edit.buffer.len();
    let mut best_dist = f32::MAX;
    let Some(font) = FONT_REGULAR.get() else {
        edit.cursor = edit.buffer.len();
        return;
    };
    let scale = 16.0 / font.raster_px;
    let mut width = 0.0;
    for (idx, ch) in edit
        .buffer
        .char_indices()
        .chain(std::iter::once((edit.buffer.len(), '\0')))
    {
        let dist = (width - local_x).abs();
        if dist < best_dist {
            best = idx;
            best_dist = dist;
        }
        if idx < edit.buffer.len() {
            width += glyph_advance(font, ch, scale);
        }
    }
    edit.cursor = best;
    edit.selection_anchor = None;
}

pub(crate) fn update_inspector_text_input(app: &mut AppState, mouse: Vec2) -> bool {
    if app.inspector_edit.is_none() {
        return false;
    }
    if is_mouse_button_pressed(MouseButton::Left) {
        if let Some(action) = clicked_inspector_copy_action(app, mouse) {
            apply_inspector_edit(app);
            copy_inspector_value(app, action);
            return true;
        }
        if let Some(field) = clicked_inspector_field(app, mouse) {
            let same_field = app
                .inspector_edit
                .as_ref()
                .is_some_and(|edit| edit.field == field);
            if same_field {
                let rect = inspector_field_rect(app, field);
                if let Some(edit) = app.inspector_edit.as_mut() {
                    set_edit_cursor_from_mouse(edit, mouse.x, rect);
                }
            } else {
                apply_inspector_edit(app);
                start_inspector_edit(app, field);
                let rect = inspector_field_rect(app, field);
                if let Some(edit) = app.inspector_edit.as_mut() {
                    set_edit_cursor_from_mouse(edit, mouse.x, rect);
                }
            }
            return true;
        }
        apply_inspector_edit(app);
        return true;
    }
    if is_key_pressed(KeyCode::Escape) {
        app.inspector_edit = None;
        return true;
    }
    if is_key_pressed(KeyCode::Enter) {
        apply_inspector_edit(app);
        return true;
    }
    if let Some(edit) = app.inspector_edit.as_mut() {
        edit.cursor = clamp_char_boundary(&edit.buffer, edit.cursor);
        if handle_text_clipboard_shortcuts(
            &mut edit.buffer,
            &mut edit.cursor,
            &mut edit.selection_anchor,
        ) {
            drain_text_input();
            return true;
        }
        let ctrl_down = ctrl_down();
        if is_key_pressed(KeyCode::Home) {
            edit.cursor = 0;
            edit.selection_anchor = None;
        }
        if is_key_pressed(KeyCode::End) {
            edit.cursor = edit.buffer.len();
            edit.selection_anchor = None;
        }
        if is_key_pressed(KeyCode::Left) {
            edit.cursor = prev_char_boundary(&edit.buffer, edit.cursor);
            edit.selection_anchor = None;
        }
        if is_key_pressed(KeyCode::Right) {
            edit.cursor = next_char_boundary(&edit.buffer, edit.cursor);
            edit.selection_anchor = None;
        }
        if is_key_pressed(KeyCode::Backspace)
            && !delete_text_selection(
                &mut edit.buffer,
                &mut edit.cursor,
                &mut edit.selection_anchor,
            )
            && edit.cursor > 0
        {
            let prev = if ctrl_down {
                prev_word_boundary(&edit.buffer, edit.cursor)
            } else {
                prev_char_boundary(&edit.buffer, edit.cursor)
            };
            edit.buffer.replace_range(prev..edit.cursor, "");
            edit.cursor = prev;
        }
        if is_key_pressed(KeyCode::Delete)
            && !delete_text_selection(
                &mut edit.buffer,
                &mut edit.cursor,
                &mut edit.selection_anchor,
            )
            && edit.cursor < edit.buffer.len()
        {
            let next = next_char_boundary(&edit.buffer, edit.cursor);
            edit.buffer.replace_range(edit.cursor..next, "");
        }
    }
    while let Some(ch) = get_char_pressed() {
        if let Some(edit) = app.inspector_edit.as_mut() {
            if handle_text_control_char(
                &mut edit.buffer,
                &mut edit.cursor,
                &mut edit.selection_anchor,
                ch,
            ) {
                continue;
            }
            if !ch.is_control() {
                insert_text_at_cursor(
                    &mut edit.buffer,
                    &mut edit.cursor,
                    &mut edit.selection_anchor,
                    &ch.to_string(),
                );
            }
        }
    }
    true
}

pub(crate) fn set_search_cursor_from_mouse(app: &mut AppState, mouse_x: f32) {
    let rect = outliner_search_rect();
    let local_x = (mouse_x - rect.x - 10.0).max(0.0);
    let mut best = app.outliner_search.len();
    let mut best_dist = f32::MAX;
    let Some(font) = FONT_REGULAR.get() else {
        app.outliner_search_cursor = app.outliner_search.len();
        return;
    };
    let scale = 16.0 / font.raster_px;
    let mut width = 0.0;
    for (idx, ch) in app
        .outliner_search
        .char_indices()
        .chain(std::iter::once((app.outliner_search.len(), '\0')))
    {
        let dist = (width - local_x).abs();
        if dist < best_dist {
            best = idx;
            best_dist = dist;
        }
        if idx < app.outliner_search.len() {
            width += glyph_advance(font, ch, scale);
        }
    }
    app.outliner_search_cursor = best;
}

pub(crate) fn update_outliner_type_filter_input(app: &mut AppState, mouse: Vec2) -> bool {
    if !is_mouse_button_pressed(MouseButton::Left) {
        return false;
    }
    let Some(slot) = (0..3).find(|slot| outliner_type_filter_rect(*slot).contains(mouse)) else {
        return false;
    };
    if app.outliner_search_active {
        app.outliner_search_active = false;
        app.outliner_search_anchor = None;
    }
    let active_count = [
        app.outliner_show_objects,
        app.outliner_show_buildings,
        app.outliner_show_lods,
    ]
    .into_iter()
    .filter(|enabled| *enabled)
    .count();
    let target = match slot {
        0 => &mut app.outliner_show_objects,
        1 => &mut app.outliner_show_buildings,
        _ => &mut app.outliner_show_lods,
    };
    if *target && active_count == 1 {
        app.status_message = "At least one outliner type filter must stay enabled".to_string();
        return true;
    }
    *target = !*target;
    rebuild_outliner_filter(app);
    app.status_message = format!(
        "Outliner filters: {}{}{}",
        if app.outliner_show_objects {
            "Objects "
        } else {
            ""
        },
        if app.outliner_show_buildings {
            "Buildings "
        } else {
            ""
        },
        if app.outliner_show_lods { "LODs" } else { "" },
    )
    .trim()
    .to_string();
    true
}

pub(crate) fn update_outliner_search_input(app: &mut AppState, mouse: Vec2) -> bool {
    let ctrl_down = ctrl_down();
    if ctrl_down && is_key_pressed(KeyCode::F) {
        if app.inspector_edit.is_some() {
            apply_inspector_edit(app);
        }
        drain_text_input();
        app.outliner_search_active = true;
        app.outliner_search_cursor = app.outliner_search.len();
        app.outliner_search_anchor = None;
        return true;
    }
    if is_mouse_button_pressed(MouseButton::Left) {
        if outliner_search_rect().contains(mouse) {
            if app.inspector_edit.is_some() {
                apply_inspector_edit(app);
            }
            drain_text_input();
            app.outliner_search_active = true;
            set_search_cursor_from_mouse(app, mouse.x);
            app.outliner_search_anchor = None;
            return true;
        }
        if app.outliner_search_active {
            app.outliner_search_active = false;
            app.outliner_search_anchor = None;
        }
    }
    if !app.outliner_search_active {
        return false;
    }
    if is_key_pressed(KeyCode::Escape) {
        app.outliner_search_active = false;
        app.outliner_search_anchor = None;
        return true;
    }
    let mut changed = false;
    app.outliner_search_cursor =
        clamp_char_boundary(&app.outliner_search, app.outliner_search_cursor);
    if handle_text_clipboard_shortcuts(
        &mut app.outliner_search,
        &mut app.outliner_search_cursor,
        &mut app.outliner_search_anchor,
    ) {
        drain_text_input();
        changed = true;
    }
    if is_key_pressed(KeyCode::Home) {
        app.outliner_search_cursor = 0;
        app.outliner_search_anchor = None;
    }
    if is_key_pressed(KeyCode::End) {
        app.outliner_search_cursor = app.outliner_search.len();
        app.outliner_search_anchor = None;
    }
    if is_key_pressed(KeyCode::Left) {
        app.outliner_search_cursor =
            prev_char_boundary(&app.outliner_search, app.outliner_search_cursor);
        app.outliner_search_anchor = None;
    }
    if is_key_pressed(KeyCode::Right) {
        app.outliner_search_cursor =
            next_char_boundary(&app.outliner_search, app.outliner_search_cursor);
        app.outliner_search_anchor = None;
    }
    if is_key_pressed(KeyCode::Backspace)
        && !delete_text_selection(
            &mut app.outliner_search,
            &mut app.outliner_search_cursor,
            &mut app.outliner_search_anchor,
        )
        && app.outliner_search_cursor > 0
    {
        let prev = if ctrl_down {
            prev_word_boundary(&app.outliner_search, app.outliner_search_cursor)
        } else {
            prev_char_boundary(&app.outliner_search, app.outliner_search_cursor)
        };
        app.outliner_search
            .replace_range(prev..app.outliner_search_cursor, "");
        app.outliner_search_cursor = prev;
        changed = true;
    }
    if is_key_pressed(KeyCode::Delete)
        && !delete_text_selection(
            &mut app.outliner_search,
            &mut app.outliner_search_cursor,
            &mut app.outliner_search_anchor,
        )
        && app.outliner_search_cursor < app.outliner_search.len()
    {
        let next = next_char_boundary(&app.outliner_search, app.outliner_search_cursor);
        app.outliner_search
            .replace_range(app.outliner_search_cursor..next, "");
        changed = true;
    }
    while let Some(ch) = get_char_pressed() {
        if handle_text_control_char(
            &mut app.outliner_search,
            &mut app.outliner_search_cursor,
            &mut app.outliner_search_anchor,
            ch,
        ) {
            changed = true;
            continue;
        }
        if !ch.is_control() {
            insert_text_at_cursor(
                &mut app.outliner_search,
                &mut app.outliner_search_cursor,
                &mut app.outliner_search_anchor,
                &ch.to_string(),
            );
            changed = true;
        }
    }
    if changed {
        app.scroll = 0.0;
        rebuild_outliner_filter(app);
    }
    true
}

pub(crate) fn update_group_rename_input(app: &mut AppState) -> bool {
    if app.group_rename.is_none() {
        return false;
    }
    if is_key_pressed(KeyCode::Escape) {
        cancel_group_rename(app);
        return true;
    }
    if is_key_pressed(KeyCode::Enter) {
        apply_group_rename(app);
        return true;
    }
    if let Some(edit) = app.group_rename.as_mut() {
        edit.cursor = clamp_char_boundary(&edit.buffer, edit.cursor);
        if handle_text_clipboard_shortcuts(
            &mut edit.buffer,
            &mut edit.cursor,
            &mut edit.selection_anchor,
        ) {
            drain_text_input();
            return true;
        }
        let ctrl_down = ctrl_down();
        if is_key_pressed(KeyCode::Home) {
            edit.cursor = 0;
            edit.selection_anchor = None;
        }
        if is_key_pressed(KeyCode::End) {
            edit.cursor = edit.buffer.len();
            edit.selection_anchor = None;
        }
        if is_key_pressed(KeyCode::Left) {
            edit.cursor = prev_char_boundary(&edit.buffer, edit.cursor);
            edit.selection_anchor = None;
        }
        if is_key_pressed(KeyCode::Right) {
            edit.cursor = next_char_boundary(&edit.buffer, edit.cursor);
            edit.selection_anchor = None;
        }
        if is_key_pressed(KeyCode::Backspace)
            && !delete_text_selection(
                &mut edit.buffer,
                &mut edit.cursor,
                &mut edit.selection_anchor,
            )
            && edit.cursor > 0
        {
            let prev = if ctrl_down {
                prev_word_boundary(&edit.buffer, edit.cursor)
            } else {
                prev_char_boundary(&edit.buffer, edit.cursor)
            };
            edit.buffer.replace_range(prev..edit.cursor, "");
            edit.cursor = prev;
        }
        if is_key_pressed(KeyCode::Delete)
            && !delete_text_selection(
                &mut edit.buffer,
                &mut edit.cursor,
                &mut edit.selection_anchor,
            )
            && edit.cursor < edit.buffer.len()
        {
            let next = next_char_boundary(&edit.buffer, edit.cursor);
            edit.buffer.replace_range(edit.cursor..next, "");
        }
    }
    while let Some(ch) = get_char_pressed() {
        if let Some(edit) = app.group_rename.as_mut() {
            if handle_text_control_char(
                &mut edit.buffer,
                &mut edit.cursor,
                &mut edit.selection_anchor,
                ch,
            ) {
                continue;
            }
            if !ch.is_control() && ch != '"' && ch != '<' && ch != '>' {
                insert_text_at_cursor(
                    &mut edit.buffer,
                    &mut edit.cursor,
                    &mut edit.selection_anchor,
                    &ch.to_string(),
                );
            }
        }
    }
    true
}

pub(crate) fn update_race_name_input(app: &mut AppState) -> bool {
    if app.race_name_edit.is_none() {
        return false;
    }
    if is_key_pressed(KeyCode::Escape) {
        cancel_race_name_edit(app);
        return true;
    }
    if is_key_pressed(KeyCode::Enter) {
        apply_race_name_edit(app);
        return true;
    }
    if let Some(edit) = app.race_name_edit.as_mut() {
        edit.cursor = clamp_char_boundary(&edit.buffer, edit.cursor);
        if handle_text_clipboard_shortcuts(
            &mut edit.buffer,
            &mut edit.cursor,
            &mut edit.selection_anchor,
        ) {
            drain_text_input();
            return true;
        }
        let ctrl_down = ctrl_down();
        if is_key_pressed(KeyCode::Home) {
            edit.cursor = 0;
            edit.selection_anchor = None;
        }
        if is_key_pressed(KeyCode::End) {
            edit.cursor = edit.buffer.len();
            edit.selection_anchor = None;
        }
        if is_key_pressed(KeyCode::Left) {
            edit.cursor = prev_char_boundary(&edit.buffer, edit.cursor);
            edit.selection_anchor = None;
        }
        if is_key_pressed(KeyCode::Right) {
            edit.cursor = next_char_boundary(&edit.buffer, edit.cursor);
            edit.selection_anchor = None;
        }
        if is_key_pressed(KeyCode::Backspace)
            && !delete_text_selection(
                &mut edit.buffer,
                &mut edit.cursor,
                &mut edit.selection_anchor,
            )
            && edit.cursor > 0
        {
            let prev = if ctrl_down {
                prev_word_boundary(&edit.buffer, edit.cursor)
            } else {
                prev_char_boundary(&edit.buffer, edit.cursor)
            };
            edit.buffer.replace_range(prev..edit.cursor, "");
            edit.cursor = prev;
        }
        if is_key_pressed(KeyCode::Delete)
            && !delete_text_selection(
                &mut edit.buffer,
                &mut edit.cursor,
                &mut edit.selection_anchor,
            )
            && edit.cursor < edit.buffer.len()
        {
            let next = next_char_boundary(&edit.buffer, edit.cursor);
            edit.buffer.replace_range(edit.cursor..next, "");
        }
    }
    while let Some(ch) = get_char_pressed() {
        if let Some(edit) = app.race_name_edit.as_mut() {
            if handle_text_control_char(
                &mut edit.buffer,
                &mut edit.cursor,
                &mut edit.selection_anchor,
                ch,
            ) {
                continue;
            }
            if !ch.is_control() && ch != '"' && ch != '<' && ch != '>' {
                insert_text_at_cursor(
                    &mut edit.buffer,
                    &mut edit.cursor,
                    &mut edit.selection_anchor,
                    &ch.to_string(),
                );
            }
        }
    }
    true
}

pub(crate) fn save_as_input_rect() -> Rect {
    let w = 760.0_f32.min(screen_width() - 80.0);
    Rect::new(
        (screen_width() - w) * 0.5 + 24.0,
        screen_height() * 0.5 - 6.0,
        w - 48.0,
        32.0,
    )
}

pub(crate) fn load_input_rect() -> Rect {
    save_as_input_rect()
}

pub(crate) fn save_as_dialog_rect() -> Rect {
    let w = 760.0_f32.min(screen_width() - 80.0);
    Rect::new(
        (screen_width() - w) * 0.5,
        screen_height() * 0.5 - 104.0,
        w,
        212.0,
    )
}

pub(crate) fn save_log_dialog_rect() -> Rect {
    let w = 860.0_f32.min(screen_width() - 80.0);
    let h = 520.0_f32.min(screen_height() - 80.0);
    Rect::new(
        (screen_width() - w) * 0.5,
        (screen_height() - h) * 0.5,
        w,
        h,
    )
}

pub(crate) fn blender_import_dialog_rect() -> Rect {
    let w = 880.0_f32.min(screen_width() - 64.0);
    let h = 560.0_f32.min(screen_height() - 64.0);
    Rect::new(
        (screen_width() - w) * 0.5,
        (screen_height() - h) * 0.5,
        w,
        h,
    )
}

pub(crate) fn blender_import_setup_dialog_rect() -> Rect {
    let w = 700.0_f32.min(screen_width() - 64.0);
    let h = 400.0_f32.min(screen_height() - 64.0);
    Rect::new(
        (screen_width() - w) * 0.5,
        (screen_height() - h) * 0.5,
        w,
        h,
    )
}

pub(crate) fn blender_import_setup_chunk_rect() -> Rect {
    let rect = blender_import_setup_dialog_rect();
    Rect::new(rect.x + rect.w - 184.0, rect.y + 125.0, 160.0, 32.0)
}

pub(crate) fn blender_import_setup_toggle_rect(row: usize) -> Rect {
    let rect = blender_import_setup_dialog_rect();
    Rect::new(
        rect.x + 24.0,
        rect.y + 182.0 + row as f32 * 52.0,
        rect.w - 48.0,
        38.0,
    )
}

pub(crate) fn blender_import_setup_start_rect() -> Rect {
    let rect = blender_import_setup_dialog_rect();
    Rect::new(rect.x + rect.w - 230.0, rect.y + rect.h - 50.0, 120.0, 32.0)
}

pub(crate) fn blender_import_setup_cancel_rect() -> Rect {
    let rect = blender_import_setup_dialog_rect();
    Rect::new(rect.x + rect.w - 98.0, rect.y + rect.h - 50.0, 74.0, 32.0)
}

pub(crate) fn blender_import_close_rect() -> Rect {
    let rect = blender_import_dialog_rect();
    Rect::new(rect.x + rect.w - 92.0, rect.y + rect.h - 48.0, 68.0, 30.0)
}

pub(crate) fn save_log_close_rect() -> Rect {
    let rect = save_log_dialog_rect();
    Rect::new(rect.x + rect.w - 92.0, rect.y + rect.h - 48.0, 68.0, 30.0)
}

pub(crate) fn activity_console_clear_rect() -> Rect {
    let rect = save_log_dialog_rect();
    Rect::new(rect.x + 24.0, rect.y + rect.h - 48.0, 68.0, 30.0)
}

pub(crate) fn status_log_rect() -> Rect {
    Rect::new(
        (PANEL_W + 360.0).min(screen_width() - 94.0).max(16.0),
        screen_height() - STATUS_H + 3.0,
        82.0,
        STATUS_H - 6.0,
    )
}

pub(crate) fn load_dialog_rect() -> Rect {
    save_as_dialog_rect()
}

pub(crate) const PREFERENCES_DIALOG_H: f32 = 646.0;
/// First row of the Viewport section, relative to the dialog's top edge.
const PREFERENCES_VIEWPORT_ROW_Y: f32 = 246.0;
const PREFERENCES_VIEWPORT_ROW_GAP: f32 = 44.0;
const PREFERENCES_STEP_W: f32 = 32.0;
const PREFERENCES_VALUE_W: f32 = 104.0;

pub(crate) fn preferences_dialog_rect() -> Rect {
    let w = 760.0_f32.min(screen_width() - 80.0);
    let h = PREFERENCES_DIALOG_H.min(screen_height() - 40.0);
    Rect::new(
        (screen_width() - w) * 0.5,
        ((screen_height() - h) * 0.5).max(20.0),
        w,
        h,
    )
}

pub(crate) fn preferences_input_rect() -> Rect {
    let rect = preferences_dialog_rect();
    Rect::new(rect.x + 24.0, rect.y + 98.0, rect.w - 48.0, 32.0)
}

/// Row `row` of the Viewport section: the label sits at the left, the control
/// group is right-aligned. Returns (label_x, row_y).
pub(crate) fn preferences_row_origin(row: usize) -> (f32, f32) {
    let rect = preferences_dialog_rect();
    (
        rect.x + 24.0,
        rect.y + PREFERENCES_VIEWPORT_ROW_Y + row as f32 * PREFERENCES_VIEWPORT_ROW_GAP,
    )
}

/// Stepper for row `row`: (minus, value, plus).
pub(crate) fn preferences_stepper_rects(row: usize) -> (Rect, Rect, Rect) {
    let rect = preferences_dialog_rect();
    let (_, y) = preferences_row_origin(row);
    let right = rect.x + rect.w - 24.0;
    let plus = Rect::new(right - PREFERENCES_STEP_W, y, PREFERENCES_STEP_W, 30.0);
    let value = Rect::new(
        plus.x - 6.0 - PREFERENCES_VALUE_W,
        y,
        PREFERENCES_VALUE_W,
        30.0,
    );
    let minus = Rect::new(
        value.x - 6.0 - PREFERENCES_STEP_W,
        y,
        PREFERENCES_STEP_W,
        30.0,
    );
    (minus, value, plus)
}

pub(crate) fn preferences_gizmo_scale_rects() -> (Rect, Rect, Rect) {
    preferences_stepper_rects(0)
}

pub(crate) fn preferences_camera_speed_rects() -> (Rect, Rect, Rect) {
    preferences_stepper_rects(1)
}

pub(crate) fn preferences_vehicle_camera_speed_rects() -> (Rect, Rect, Rect) {
    preferences_stepper_rects(2)
}

pub(crate) fn preferences_editing_camera_speed_rects() -> (Rect, Rect, Rect) {
    preferences_stepper_rects(3)
}

pub(crate) fn preferences_camera_rotation_speed_rects() -> (Rect, Rect, Rect) {
    preferences_stepper_rects(4)
}

pub(crate) const PREFERENCES_MSAA_ROW: usize = 5;
pub(crate) const PREFERENCES_DRAW_DISTANCE_ROW: usize = 6;

pub(crate) fn preferences_draw_distance_rects() -> (Rect, Rect, Rect) {
    preferences_stepper_rects(PREFERENCES_DRAW_DISTANCE_ROW)
}

pub(crate) fn preferences_msaa_rect() -> Rect {
    let (_, value, plus) = preferences_stepper_rects(PREFERENCES_MSAA_ROW);
    Rect::new(value.x, value.y, value.w + 6.0 + plus.w, value.h)
}

pub(crate) fn preferences_cleanup_autosaves_rect() -> Rect {
    let rect = preferences_dialog_rect();
    Rect::new(rect.x + 24.0, rect.y + rect.h - 50.0, 172.0, 32.0)
}

pub(crate) fn dff_replace_choice_dialog_rect() -> Rect {
    let w = 620.0_f32.min(screen_width() - 80.0);
    Rect::new(
        (screen_width() - w) * 0.5,
        screen_height() * 0.5 - 94.0,
        w,
        188.0,
    )
}

pub(crate) fn element_id_rename_dialog_rect() -> Rect {
    let w = 720.0_f32.min(screen_width() - 80.0);
    Rect::new(
        (screen_width() - w) * 0.5,
        screen_height() * 0.5 - 104.0,
        w,
        208.0,
    )
}

pub(crate) fn element_replace_with_dialog_rect() -> Rect {
    let w = 700.0_f32.min(screen_width() - 80.0);
    Rect::new(
        (screen_width() - w) * 0.5,
        (screen_height() - 500.0) * 0.5,
        w,
        500.0,
    )
}

pub(crate) fn element_replace_with_search_rect() -> Rect {
    let rect = element_replace_with_dialog_rect();
    Rect::new(rect.x + 24.0, rect.y + 88.0, rect.w - 48.0, 36.0)
}

pub(crate) fn element_replace_with_row_rect(row: usize) -> Rect {
    let rect = element_replace_with_dialog_rect();
    Rect::new(
        rect.x + 24.0,
        rect.y + 138.0 + row as f32 * 32.0,
        rect.w - 48.0,
        30.0,
    )
}

pub(crate) fn element_replace_with_pick_rect() -> Rect {
    let rect = element_replace_with_dialog_rect();
    Rect::new(rect.x + 24.0, rect.y + rect.h - 52.0, 150.0, 32.0)
}

pub(crate) fn element_replace_with_apply_rect() -> Rect {
    let rect = element_replace_with_dialog_rect();
    Rect::new(rect.x + rect.w - 210.0, rect.y + rect.h - 52.0, 100.0, 32.0)
}

pub(crate) fn element_replace_with_cancel_rect() -> Rect {
    let rect = element_replace_with_dialog_rect();
    Rect::new(rect.x + rect.w - 98.0, rect.y + rect.h - 52.0, 74.0, 32.0)
}

pub(crate) fn missing_texture_dialog_rect() -> Rect {
    let w = 720.0_f32.min(screen_width() - 80.0);
    Rect::new(
        (screen_width() - w) * 0.5,
        screen_height() * 0.5 - 214.0,
        w,
        428.0,
    )
}

pub(crate) fn missing_texture_candidate_rect(slot: usize) -> Rect {
    let rect = missing_texture_dialog_rect();
    Rect::new(
        rect.x + 24.0,
        rect.y + 112.0 + slot as f32 * 56.0,
        rect.w - 48.0,
        50.0,
    )
}

pub(crate) fn texture_archive_dialog_rect() -> Rect {
    let w = 720.0_f32.min(screen_width() - 80.0);
    let h = 520.0_f32.min(screen_height() - 100.0);
    Rect::new(
        (screen_width() - w) * 0.5,
        (screen_height() - h) * 0.5,
        w,
        h,
    )
}

pub(crate) fn texture_archive_row_rect(slot: usize) -> Rect {
    let rect = texture_archive_dialog_rect();
    Rect::new(
        rect.x + 24.0,
        rect.y + 112.0 + slot as f32 * 38.0,
        (rect.w - 300.0).max(280.0),
        34.0,
    )
}

pub(crate) fn texture_archive_preview_rect() -> Rect {
    let rect = texture_archive_dialog_rect();
    Rect::new(rect.x + rect.w - 244.0, rect.y + 112.0, 220.0, 220.0)
}

pub(crate) fn texture_archive_add_rect() -> Rect {
    let rect = texture_archive_dialog_rect();
    Rect::new(rect.x + 24.0, rect.y + rect.h - 50.0, 118.0, 32.0)
}

pub(crate) fn texture_archive_replace_rect() -> Rect {
    let rect = texture_archive_dialog_rect();
    Rect::new(rect.x + 154.0, rect.y + rect.h - 50.0, 154.0, 32.0)
}

pub(crate) fn texture_archive_close_rect() -> Rect {
    let rect = texture_archive_dialog_rect();
    Rect::new(rect.x + rect.w - 112.0, rect.y + rect.h - 50.0, 88.0, 32.0)
}

pub(crate) fn dff_prelight_import_dialog_rect() -> Rect {
    let w = 720.0_f32.min(screen_width() - 80.0);
    let h = 520.0_f32.min(screen_height() - 100.0);
    Rect::new(
        (screen_width() - w) * 0.5,
        (screen_height() - h) * 0.5,
        w,
        h,
    )
}

pub(crate) fn dff_prelight_import_row_rect(slot: usize) -> Rect {
    let rect = dff_prelight_import_dialog_rect();
    Rect::new(
        rect.x + 24.0,
        rect.y + 112.0 + slot as f32 * 34.0,
        rect.w - 48.0,
        30.0,
    )
}

pub(crate) fn dff_prelight_import_apply_rect() -> Rect {
    let rect = dff_prelight_import_dialog_rect();
    Rect::new(rect.x + 24.0, rect.y + rect.h - 50.0, 166.0, 32.0)
}

pub(crate) fn dff_prelight_import_close_rect() -> Rect {
    let rect = dff_prelight_import_dialog_rect();
    Rect::new(rect.x + rect.w - 112.0, rect.y + rect.h - 50.0, 88.0, 32.0)
}

pub(crate) fn missing_col_dialog_rect() -> Rect {
    let w = 620.0_f32.min(screen_width() - 80.0);
    Rect::new(
        (screen_width() - w) * 0.5,
        screen_height() * 0.5 - 94.0,
        w,
        188.0,
    )
}

pub(crate) fn lod_batch_dialog_rect() -> Rect {
    let w = 780.0_f32.min(screen_width() - 80.0);
    let h = 610.0_f32.min(screen_height() - 80.0);
    Rect::new(
        (screen_width() - w) * 0.5,
        (screen_height() - h) * 0.5,
        w,
        h,
    )
}

pub(crate) fn lod_batch_minimum_size_rect() -> Rect {
    let rect = lod_batch_dialog_rect();
    Rect::new(rect.x + 166.0, rect.y + 78.0, 150.0, 32.0)
}

pub(crate) fn lod_batch_list_rect() -> Rect {
    let rect = lod_batch_dialog_rect();
    Rect::new(rect.x + 24.0, rect.y + 176.0, rect.w - 48.0, rect.h - 246.0)
}

pub(crate) fn lod_batch_missing_only_rect() -> Rect {
    let rect = lod_batch_dialog_rect();
    Rect::new(rect.x + 166.0, rect.y + 122.0, 132.0, 30.0)
}

pub(crate) fn lod_batch_regenerate_all_rect() -> Rect {
    let rect = lod_batch_dialog_rect();
    Rect::new(rect.x + 306.0, rect.y + 122.0, 142.0, 30.0)
}

pub(crate) fn lod_batch_continue_rect() -> Rect {
    let rect = lod_batch_dialog_rect();
    Rect::new(rect.x + rect.w - 228.0, rect.y + rect.h - 50.0, 104.0, 32.0)
}

pub(crate) fn lod_batch_cancel_rect() -> Rect {
    let rect = lod_batch_dialog_rect();
    Rect::new(rect.x + rect.w - 112.0, rect.y + rect.h - 50.0, 88.0, 32.0)
}

pub(crate) fn set_save_as_cursor_from_mouse(dialog: &mut SaveAsDialog, mouse_x: f32) {
    let rect = save_as_input_rect();
    let local_x = (mouse_x - rect.x - 10.0).max(0.0);
    let mut best = dialog.path.len();
    let mut best_dist = f32::MAX;
    let Some(font) = FONT_REGULAR.get() else {
        dialog.cursor = dialog.path.len();
        return;
    };
    let scale = 16.0 / font.raster_px;
    let mut width = 0.0;
    for (idx, ch) in dialog
        .path
        .char_indices()
        .chain(std::iter::once((dialog.path.len(), '\0')))
    {
        let dist = (width - local_x).abs();
        if dist < best_dist {
            best = idx;
            best_dist = dist;
        }
        if idx < dialog.path.len() {
            width += glyph_advance(font, ch, scale);
        }
    }
    dialog.cursor = best;
    dialog.selection_anchor = None;
}

pub(crate) fn set_load_cursor_from_mouse(dialog: &mut LoadDialog, mouse_x: f32) {
    let rect = load_input_rect();
    let local_x = (mouse_x - rect.x - 10.0).max(0.0);
    let mut best = dialog.path.len();
    let mut best_dist = f32::MAX;
    let Some(font) = FONT_REGULAR.get() else {
        dialog.cursor = dialog.path.len();
        return;
    };
    let scale = 16.0 / font.raster_px;
    let mut width = 0.0;
    for (idx, ch) in dialog
        .path
        .char_indices()
        .chain(std::iter::once((dialog.path.len(), '\0')))
    {
        let dist = (width - local_x).abs();
        if dist < best_dist {
            best = idx;
            best_dist = dist;
        }
        if idx < dialog.path.len() {
            width += glyph_advance(font, ch, scale);
        }
    }
    dialog.cursor = best;
    dialog.selection_anchor = None;
}

pub(crate) fn set_preferences_cursor_from_mouse(dialog: &mut PreferencesDialog, mouse_x: f32) {
    let rect = preferences_input_rect();
    let local_x = (mouse_x - rect.x - 10.0).max(0.0);
    let mut best = dialog.gta_sa_dir.len();
    let mut best_dist = f32::MAX;
    let Some(font) = FONT_REGULAR.get() else {
        dialog.cursor = dialog.gta_sa_dir.len();
        return;
    };
    let scale = 16.0 / font.raster_px;
    let mut width = 0.0;
    for (idx, ch) in dialog
        .gta_sa_dir
        .char_indices()
        .chain(std::iter::once((dialog.gta_sa_dir.len(), '\0')))
    {
        let dist = (width - local_x).abs();
        if dist < best_dist {
            best = idx;
            best_dist = dist;
        }
        if idx < dialog.gta_sa_dir.len() {
            width += glyph_advance(font, ch, scale);
        }
    }
    dialog.cursor = best;
    dialog.selection_anchor = None;
}

pub(crate) fn save_preferences_dialog(app: &mut AppState, dialog: PreferencesDialog) {
    let path = PathBuf::from(dialog.gta_sa_dir.trim());
    if let Err(error) = validate_gta_sa_dir(&path) {
        app.status_message = error;
        app.preferences_dialog = Some(dialog);
        return;
    }
    app.gta_sa_dir = path.clone();
    app.physics_root_properties = load_physics_root_properties(&path);
    app.physics_root_dropdown_open = false;
    save_gta_sa_dir_preference(&path);
    invalidate_validation_cache(app);

    set_gizmo_scale(app, dialog.gizmo_scale);
    set_camera_speed_for_tab(app, AppTab::Preview, dialog.camera_speed);
    set_camera_speed_for_tab(app, AppTab::Vehicles, dialog.vehicle_camera_speed);
    set_camera_speed_for_tab(app, AppTab::Editing, dialog.editing_camera_speed);
    set_camera_rotation_speed(app, dialog.camera_rotation_speed);
    let msaa_changed =
        clamp_msaa_samples(dialog.msaa_samples) != clamp_msaa_samples(dialog.msaa_samples_saved);
    if msaa_changed {
        save_msaa_samples_preference(dialog.msaa_samples);
    }
    let draw_distance_percent = clamp_draw_distance_percent(dialog.draw_distance_percent);
    app.options.draw_distance_percent = draw_distance_percent;
    app.options.draw_radius = draw_radius_for_percent(draw_distance_percent);
    save_draw_distance_percent_preference(draw_distance_percent);

    app.status_message = if msaa_changed {
        "Preferences saved. Restart for the anti-aliasing change, and reload the resource to re-index GTA:SA assets.".to_string()
    } else {
        "Preferences saved. Reload the resource to re-index GTA:SA assets.".to_string()
    };
}

pub(crate) fn update_load_dialog_input(app: &mut AppState, mouse: Vec2) -> bool {
    if app.load_dialog.is_none() {
        return false;
    }
    let rect = load_dialog_rect();
    let load_rect = Rect::new(rect.x + rect.w - 216.0, rect.y + rect.h - 50.0, 88.0, 32.0);
    let cancel_rect = Rect::new(rect.x + rect.w - 116.0, rect.y + rect.h - 50.0, 88.0, 32.0);
    if is_mouse_button_pressed(MouseButton::Left) {
        if load_input_rect().contains(mouse) {
            if let Some(dialog) = app.load_dialog.as_mut() {
                set_load_cursor_from_mouse(dialog, mouse.x);
            }
            return true;
        }
        if load_rect.contains(mouse) {
            if let Some(dialog) = app.load_dialog.take() {
                start_load_resource(app, normalize_resource_path(&dialog.path));
            }
            return true;
        }
        if cancel_rect.contains(mouse) || !rect.contains(mouse) {
            app.load_dialog = None;
            return true;
        }
    }
    if is_key_pressed(KeyCode::Escape) {
        app.load_dialog = None;
        return true;
    }
    if is_key_pressed(KeyCode::Enter) {
        if let Some(dialog) = app.load_dialog.take() {
            start_load_resource(app, normalize_resource_path(&dialog.path));
        }
        return true;
    }
    if let Some(dialog) = app.load_dialog.as_mut() {
        dialog.cursor = clamp_char_boundary(&dialog.path, dialog.cursor);
        if handle_text_clipboard_shortcuts(
            &mut dialog.path,
            &mut dialog.cursor,
            &mut dialog.selection_anchor,
        ) {
            drain_text_input();
            return true;
        }
        if is_key_pressed(KeyCode::Home) {
            dialog.cursor = 0;
            dialog.selection_anchor = None;
        }
        if is_key_pressed(KeyCode::End) {
            dialog.cursor = dialog.path.len();
            dialog.selection_anchor = None;
        }
        if is_key_pressed(KeyCode::Left) {
            dialog.cursor = prev_char_boundary(&dialog.path, dialog.cursor);
            dialog.selection_anchor = None;
        }
        if is_key_pressed(KeyCode::Right) {
            dialog.cursor = next_char_boundary(&dialog.path, dialog.cursor);
            dialog.selection_anchor = None;
        }
        let ctrl_down = ctrl_down();
        if is_key_pressed(KeyCode::Backspace)
            && !delete_text_selection(
                &mut dialog.path,
                &mut dialog.cursor,
                &mut dialog.selection_anchor,
            )
            && dialog.cursor > 0
        {
            let prev = if ctrl_down {
                prev_word_boundary(&dialog.path, dialog.cursor)
            } else {
                prev_char_boundary(&dialog.path, dialog.cursor)
            };
            dialog.path.replace_range(prev..dialog.cursor, "");
            dialog.cursor = prev;
        }
        if is_key_pressed(KeyCode::Delete)
            && !delete_text_selection(
                &mut dialog.path,
                &mut dialog.cursor,
                &mut dialog.selection_anchor,
            )
            && dialog.cursor < dialog.path.len()
        {
            let next = next_char_boundary(&dialog.path, dialog.cursor);
            dialog.path.replace_range(dialog.cursor..next, "");
        }
        while let Some(ch) = get_char_pressed() {
            if handle_text_control_char(
                &mut dialog.path,
                &mut dialog.cursor,
                &mut dialog.selection_anchor,
                ch,
            ) {
                continue;
            }
            if !ch.is_control() {
                insert_text_at_cursor(
                    &mut dialog.path,
                    &mut dialog.cursor,
                    &mut dialog.selection_anchor,
                    &ch.to_string(),
                );
            }
        }
    }
    true
}

pub(crate) fn update_import_asset_dialog_input(app: &mut AppState, mouse: Vec2) -> bool {
    if app.import_asset_dialog.is_none() {
        return false;
    }
    let layout = import_asset_dialog_layout();
    if is_mouse_button_pressed(MouseButton::Left) {
        if layout.browse.contains(mouse) {
            open_import_asset_texture_folder_picker(app);
            return true;
        }
        if layout.cancel.contains(mouse) || !layout.rect.contains(mouse) {
            cancel_import_asset_dialog(app);
            return true;
        }
        if layout.import.contains(mouse) {
            if let Some(dialog) = app.import_asset_dialog.take() {
                if let Err(err) = import_new_asset(
                    app,
                    &dialog.dff_path,
                    Some(&dialog.texture_dir),
                    &dialog.id,
                    None,
                ) {
                    app.status_message = format!("Import failed: {err}");
                    app.import_asset_dialog = Some(dialog);
                }
            }
            return true;
        }
    }
    if is_key_pressed(KeyCode::Escape) {
        cancel_import_asset_dialog(app);
        return true;
    }
    if is_key_pressed(KeyCode::Enter) {
        if let Some(dialog) = app.import_asset_dialog.take() {
            if let Err(err) = import_new_asset(
                app,
                &dialog.dff_path,
                Some(&dialog.texture_dir),
                &dialog.id,
                None,
            ) {
                app.status_message = format!("Import failed: {err}");
                app.import_asset_dialog = Some(dialog);
            }
        }
        return true;
    }
    let Some(dialog) = app.import_asset_dialog.as_mut() else {
        return true;
    };
    dialog.cursor = clamp_char_boundary(&dialog.id, dialog.cursor);
    if is_key_pressed(KeyCode::Left) {
        dialog.cursor = prev_char_boundary(&dialog.id, dialog.cursor);
    }
    if is_key_pressed(KeyCode::Right) {
        dialog.cursor = next_char_boundary(&dialog.id, dialog.cursor);
    }
    if is_key_pressed(KeyCode::Backspace) && dialog.cursor > 0 {
        let previous = prev_char_boundary(&dialog.id, dialog.cursor);
        dialog.id.replace_range(previous..dialog.cursor, "");
        dialog.cursor = previous;
    }
    if is_key_pressed(KeyCode::Delete) && dialog.cursor < dialog.id.len() {
        let next = next_char_boundary(&dialog.id, dialog.cursor);
        dialog.id.replace_range(dialog.cursor..next, "");
    }
    while let Some(ch) = get_char_pressed() {
        if !ch.is_control() && dialog.id.len() < 96 {
            dialog.id.insert(dialog.cursor, ch);
            dialog.cursor += ch.len_utf8();
        }
    }
    true
}

pub(crate) fn update_preferences_dialog_input(app: &mut AppState, mouse: Vec2) -> bool {
    if app.preferences_dialog.is_none() {
        return false;
    }
    let rect = preferences_dialog_rect();
    let save_rect = Rect::new(rect.x + rect.w - 216.0, rect.y + rect.h - 50.0, 88.0, 32.0);
    let cancel_rect = Rect::new(rect.x + rect.w - 116.0, rect.y + rect.h - 50.0, 88.0, 32.0);
    if is_mouse_button_pressed(MouseButton::Left) {
        if preferences_input_rect().contains(mouse) {
            if let Some(dialog) = app.preferences_dialog.as_mut() {
                set_preferences_cursor_from_mouse(dialog, mouse.x);
            }
            return true;
        }
        let (gizmo_minus, _, gizmo_plus) = preferences_gizmo_scale_rects();
        let (speed_minus, _, speed_plus) = preferences_camera_speed_rects();
        let (vehicle_speed_minus, _, vehicle_speed_plus) = preferences_vehicle_camera_speed_rects();
        let (editing_speed_minus, _, editing_speed_plus) = preferences_editing_camera_speed_rects();
        let (spin_minus, _, spin_plus) = preferences_camera_rotation_speed_rects();
        let (draw_minus, _, draw_plus) = preferences_draw_distance_rects();
        if let Some(dialog) = app.preferences_dialog.as_mut() {
            if gizmo_minus.contains(mouse) {
                let step = gizmo_scale_step(dialog.gizmo_scale, false);
                dialog.gizmo_scale = clamp_gizmo_scale(dialog.gizmo_scale - step);
                return true;
            }
            if gizmo_plus.contains(mouse) {
                let step = gizmo_scale_step(dialog.gizmo_scale, true);
                dialog.gizmo_scale = clamp_gizmo_scale(dialog.gizmo_scale + step);
                return true;
            }
            if speed_minus.contains(mouse) {
                dialog.camera_speed =
                    clamp_camera_speed(dialog.camera_speed / CAMERA_SPEED_STEP_FACTOR);
                return true;
            }
            if speed_plus.contains(mouse) {
                dialog.camera_speed =
                    clamp_camera_speed(dialog.camera_speed * CAMERA_SPEED_STEP_FACTOR);
                return true;
            }
            if vehicle_speed_minus.contains(mouse) {
                dialog.vehicle_camera_speed = clamp_detail_camera_speed(
                    dialog.vehicle_camera_speed / CAMERA_SPEED_STEP_FACTOR,
                );
                return true;
            }
            if vehicle_speed_plus.contains(mouse) {
                dialog.vehicle_camera_speed = clamp_detail_camera_speed(
                    dialog.vehicle_camera_speed * CAMERA_SPEED_STEP_FACTOR,
                );
                return true;
            }
            if editing_speed_minus.contains(mouse) {
                dialog.editing_camera_speed = clamp_editing_camera_speed(
                    dialog.editing_camera_speed / CAMERA_SPEED_STEP_FACTOR,
                );
                return true;
            }
            if editing_speed_plus.contains(mouse) {
                dialog.editing_camera_speed = clamp_editing_camera_speed(
                    dialog.editing_camera_speed * CAMERA_SPEED_STEP_FACTOR,
                );
                return true;
            }
            if spin_minus.contains(mouse) {
                dialog.camera_rotation_speed = clamp_camera_rotation_speed(
                    dialog.camera_rotation_speed / CAMERA_SPEED_STEP_FACTOR,
                );
                return true;
            }
            if spin_plus.contains(mouse) {
                dialog.camera_rotation_speed = clamp_camera_rotation_speed(
                    dialog.camera_rotation_speed * CAMERA_SPEED_STEP_FACTOR,
                );
                return true;
            }
            if preferences_msaa_rect().contains(mouse) {
                dialog.msaa_samples = next_msaa_samples(dialog.msaa_samples);
                return true;
            }
            if draw_minus.contains(mouse) {
                dialog.draw_distance_percent =
                    clamp_draw_distance_percent(dialog.draw_distance_percent.saturating_sub(25));
                return true;
            }
            if draw_plus.contains(mouse) {
                dialog.draw_distance_percent =
                    clamp_draw_distance_percent(dialog.draw_distance_percent.saturating_add(25));
                return true;
            }
        }
        if preferences_cleanup_autosaves_rect().contains(mouse) {
            app.confirm_dialog = Some(ConfirmDialog {
                action: ConfirmAction::CleanupAutosaves,
                title: "Delete Recovery Copies?".to_string(),
                body: "Delete all recovery copies for this project?".to_string(),
                detail: "This does not change the resource currently open in the editor. New unsaved changes can create another recovery copy later.".to_string(),
                primary_label: "Clean Up".to_string(),
                secondary_label: None,
                secondary_action: None,
            });
            return true;
        }
        if save_rect.contains(mouse) {
            if let Some(dialog) = app.preferences_dialog.take() {
                save_preferences_dialog(app, dialog);
            }
            return true;
        }
        if cancel_rect.contains(mouse) || !rect.contains(mouse) {
            app.preferences_dialog = None;
            return true;
        }
    }
    if is_key_pressed(KeyCode::Escape) {
        app.preferences_dialog = None;
        return true;
    }
    if is_key_pressed(KeyCode::Enter) {
        if let Some(dialog) = app.preferences_dialog.take() {
            save_preferences_dialog(app, dialog);
        }
        return true;
    }
    if let Some(dialog) = app.preferences_dialog.as_mut() {
        dialog.cursor = clamp_char_boundary(&dialog.gta_sa_dir, dialog.cursor);
        if handle_text_clipboard_shortcuts(
            &mut dialog.gta_sa_dir,
            &mut dialog.cursor,
            &mut dialog.selection_anchor,
        ) {
            drain_text_input();
            return true;
        }
        if is_key_pressed(KeyCode::Home) {
            dialog.cursor = 0;
            dialog.selection_anchor = None;
        }
        if is_key_pressed(KeyCode::End) {
            dialog.cursor = dialog.gta_sa_dir.len();
            dialog.selection_anchor = None;
        }
        if is_key_pressed(KeyCode::Left) {
            dialog.cursor = prev_char_boundary(&dialog.gta_sa_dir, dialog.cursor);
            dialog.selection_anchor = None;
        }
        if is_key_pressed(KeyCode::Right) {
            dialog.cursor = next_char_boundary(&dialog.gta_sa_dir, dialog.cursor);
            dialog.selection_anchor = None;
        }
        let ctrl_down = ctrl_down();
        if is_key_pressed(KeyCode::Backspace)
            && !delete_text_selection(
                &mut dialog.gta_sa_dir,
                &mut dialog.cursor,
                &mut dialog.selection_anchor,
            )
            && dialog.cursor > 0
        {
            let prev = if ctrl_down {
                prev_word_boundary(&dialog.gta_sa_dir, dialog.cursor)
            } else {
                prev_char_boundary(&dialog.gta_sa_dir, dialog.cursor)
            };
            dialog.gta_sa_dir.replace_range(prev..dialog.cursor, "");
            dialog.cursor = prev;
        }
        if is_key_pressed(KeyCode::Delete)
            && !delete_text_selection(
                &mut dialog.gta_sa_dir,
                &mut dialog.cursor,
                &mut dialog.selection_anchor,
            )
            && dialog.cursor < dialog.gta_sa_dir.len()
        {
            let next = next_char_boundary(&dialog.gta_sa_dir, dialog.cursor);
            dialog.gta_sa_dir.replace_range(dialog.cursor..next, "");
        }
        while let Some(ch) = get_char_pressed() {
            if handle_text_control_char(
                &mut dialog.gta_sa_dir,
                &mut dialog.cursor,
                &mut dialog.selection_anchor,
                ch,
            ) {
                continue;
            }
            if !ch.is_control() {
                insert_text_at_cursor(
                    &mut dialog.gta_sa_dir,
                    &mut dialog.cursor,
                    &mut dialog.selection_anchor,
                    &ch.to_string(),
                );
            }
        }
    }
    true
}

pub(crate) fn update_save_as_dialog_input(app: &mut AppState, mouse: Vec2) -> bool {
    if app.save_as_dialog.is_none() {
        return false;
    }
    let rect = save_as_dialog_rect();
    let save_rect = Rect::new(rect.x + rect.w - 216.0, rect.y + rect.h - 50.0, 88.0, 32.0);
    let cancel_rect = Rect::new(rect.x + rect.w - 116.0, rect.y + rect.h - 50.0, 88.0, 32.0);
    if is_mouse_button_pressed(MouseButton::Left) {
        if save_as_input_rect().contains(mouse) {
            if let Some(dialog) = app.save_as_dialog.as_mut() {
                set_save_as_cursor_from_mouse(dialog, mouse.x);
            }
            return true;
        }
        if save_rect.contains(mouse) {
            if let Some(dialog) = app.save_as_dialog.take() {
                start_save_as_output(app, PathBuf::from(dialog.path.trim()));
            }
            return true;
        }
        if cancel_rect.contains(mouse) || !rect.contains(mouse) {
            app.save_as_dialog = None;
            return true;
        }
    }
    if is_key_pressed(KeyCode::Escape) {
        app.save_as_dialog = None;
        return true;
    }
    if is_key_pressed(KeyCode::Enter) {
        if let Some(dialog) = app.save_as_dialog.take() {
            start_save_as_output(app, PathBuf::from(dialog.path.trim()));
        }
        return true;
    }
    if let Some(dialog) = app.save_as_dialog.as_mut() {
        dialog.cursor = clamp_char_boundary(&dialog.path, dialog.cursor);
        if handle_text_clipboard_shortcuts(
            &mut dialog.path,
            &mut dialog.cursor,
            &mut dialog.selection_anchor,
        ) {
            drain_text_input();
            return true;
        }
        if is_key_pressed(KeyCode::Home) {
            dialog.cursor = 0;
            dialog.selection_anchor = None;
        }
        if is_key_pressed(KeyCode::End) {
            dialog.cursor = dialog.path.len();
            dialog.selection_anchor = None;
        }
        if is_key_pressed(KeyCode::Left) {
            dialog.cursor = prev_char_boundary(&dialog.path, dialog.cursor);
            dialog.selection_anchor = None;
        }
        if is_key_pressed(KeyCode::Right) {
            dialog.cursor = next_char_boundary(&dialog.path, dialog.cursor);
            dialog.selection_anchor = None;
        }
        let ctrl_down = ctrl_down();
        if is_key_pressed(KeyCode::Backspace)
            && !delete_text_selection(
                &mut dialog.path,
                &mut dialog.cursor,
                &mut dialog.selection_anchor,
            )
            && dialog.cursor > 0
        {
            let prev = if ctrl_down {
                prev_word_boundary(&dialog.path, dialog.cursor)
            } else {
                prev_char_boundary(&dialog.path, dialog.cursor)
            };
            dialog.path.replace_range(prev..dialog.cursor, "");
            dialog.cursor = prev;
        }
        if is_key_pressed(KeyCode::Delete)
            && !delete_text_selection(
                &mut dialog.path,
                &mut dialog.cursor,
                &mut dialog.selection_anchor,
            )
            && dialog.cursor < dialog.path.len()
        {
            let next = next_char_boundary(&dialog.path, dialog.cursor);
            dialog.path.replace_range(dialog.cursor..next, "");
        }
        while let Some(ch) = get_char_pressed() {
            if handle_text_control_char(
                &mut dialog.path,
                &mut dialog.cursor,
                &mut dialog.selection_anchor,
                ch,
            ) {
                continue;
            }
            if !ch.is_control() {
                insert_text_at_cursor(
                    &mut dialog.path,
                    &mut dialog.cursor,
                    &mut dialog.selection_anchor,
                    &ch.to_string(),
                );
            }
        }
    }
    true
}

pub(crate) fn update_save_log_input(app: &mut AppState, mouse: Vec2) -> bool {
    if app.save_log_open {
        let rect = save_log_dialog_rect();
        let list_width = rect.w - 48.0;
        let rows = save_log_display_rows(app, list_width).len();
        let visible_rows = ((rect.h - 142.0) / 22.0).max(1.0) as usize;
        let max_scroll = rows.saturating_sub(visible_rows) as f32;
        app.save_log_scroll = app.save_log_scroll.clamp(0.0, max_scroll);
        let list = Rect::new(rect.x + 24.0, rect.y + 78.0, rect.w - 48.0, rect.h - 142.0);
        let track = Rect::new(list.x + list.w - 10.0, list.y + 8.0, 4.0, list.h - 16.0);
        let hit_area = Rect::new(track.x - 6.0, track.y, track.w + 12.0, track.h);
        if is_mouse_button_down(MouseButton::Left) && max_scroll > 0.0 && hit_area.contains(mouse) {
            let thumb_h = (track.h * visible_rows as f32 / rows as f32).clamp(24.0, track.h);
            let travel = (track.h - thumb_h).max(1.0);
            app.save_log_scroll =
                ((mouse.y - track.y - thumb_h * 0.5).clamp(0.0, travel) / travel) * max_scroll;
            app.save_log_follow_tail = app.save_log_scroll >= max_scroll - 0.5;
            return true;
        }
        let (_, wheel_y) = safe_mouse_wheel();
        if rect.contains(mouse) && wheel_y.abs() > 0.01 {
            app.save_log_scroll = (app.save_log_scroll - wheel_y * 3.0).clamp(0.0, max_scroll);
            app.save_log_follow_tail = app.save_log_scroll >= max_scroll - 0.5;
            return true;
        }
        if is_key_pressed(KeyCode::End) {
            app.save_log_scroll = max_scroll;
            app.save_log_follow_tail = true;
            return true;
        }
        if is_key_pressed(KeyCode::Home) {
            app.save_log_scroll = 0.0;
            app.save_log_follow_tail = false;
            return true;
        }
        if is_key_pressed(KeyCode::Escape) {
            app.save_log_open = false;
            return true;
        }
        if is_mouse_button_pressed(MouseButton::Left) {
            if activity_console_clear_rect().contains(mouse) {
                app.activity_log.clear();
                app.activity_last_status.clear();
                app.save_log_scroll = 0.0;
                app.save_log_follow_tail = true;
            } else if save_log_close_rect().contains(mouse) || !rect.contains(mouse) {
                app.save_log_open = false;
            }
            return true;
        }
        return true;
    }

    if is_mouse_button_pressed(MouseButton::Left) && status_log_rect().contains(mouse) {
        app.save_log_open = true;
        app.save_log_follow_tail = true;
        let rect = save_log_dialog_rect();
        let rows = save_log_display_rows(app, rect.w - 48.0).len();
        let visible_rows = ((rect.h - 142.0) / 22.0).max(1.0) as usize;
        app.save_log_scroll = rows.saturating_sub(visible_rows) as f32;
        return true;
    }
    false
}

pub(crate) fn update_blender_import_dialog_input(app: &mut AppState, mouse: Vec2) -> bool {
    if !app.blender_import_dialog_open {
        return false;
    }
    let rect = blender_import_dialog_rect();
    let list = Rect::new(rect.x + 24.0, rect.y + 142.0, rect.w - 48.0, rect.h - 206.0);
    let rows = blender_import_display_rows(app, list.w).len();
    let visible_rows = (list.h / 20.0).max(1.0) as usize;
    let max_scroll = rows.saturating_sub(visible_rows) as f32;
    if app.blender_import_log_follow_tail {
        app.blender_import_log_scroll = max_scroll;
    } else {
        app.blender_import_log_scroll = app.blender_import_log_scroll.clamp(0.0, max_scroll);
    }

    let (_, wheel_y) = safe_mouse_wheel();
    if list.contains(mouse) && wheel_y.abs() > 0.01 {
        app.blender_import_log_scroll =
            (app.blender_import_log_scroll - wheel_y * 3.0).clamp(0.0, max_scroll);
        app.blender_import_log_follow_tail = app.blender_import_log_scroll >= max_scroll - 0.5;
        return true;
    }
    if is_key_pressed(KeyCode::End) {
        app.blender_import_log_scroll = max_scroll;
        app.blender_import_log_follow_tail = true;
        return true;
    }
    if is_key_pressed(KeyCode::Home) {
        app.blender_import_log_scroll = 0.0;
        app.blender_import_log_follow_tail = false;
        return true;
    }
    if app.blender_import_finished
        && (is_key_pressed(KeyCode::Escape)
            || (is_mouse_button_pressed(MouseButton::Left)
                && blender_import_close_rect().contains(mouse)))
    {
        app.blender_import_dialog_open = false;
        return true;
    }
    true
}

pub(crate) fn update_blender_import_setup_input(app: &mut AppState, mouse: Vec2) -> bool {
    if app.blender_import_setup.is_none() {
        return false;
    }
    if is_key_pressed(KeyCode::Escape) {
        app.blender_import_setup = None;
        app.status_message = "Blender import cancelled".to_string();
        return true;
    }
    if is_key_pressed(KeyCode::Enter) {
        confirm_blender_import_setup(app);
        return true;
    }
    if is_mouse_button_pressed(MouseButton::Left) {
        if blender_import_setup_start_rect().contains(mouse) {
            confirm_blender_import_setup(app);
            return true;
        }
        if blender_import_setup_cancel_rect().contains(mouse) {
            app.blender_import_setup = None;
            app.status_message = "Blender import cancelled".to_string();
            return true;
        }
        if blender_import_setup_toggle_rect(0).contains(mouse) {
            if let Some(setup) = app.blender_import_setup.as_mut() {
                setup.options.chunk_meshes = !setup.options.chunk_meshes;
            }
            return true;
        }
        if blender_import_setup_toggle_rect(1).contains(mouse) {
            if let Some(setup) = app.blender_import_setup.as_mut() {
                setup.options.center_origins = !setup.options.center_origins;
            }
            return true;
        }
        if blender_import_setup_chunk_rect().contains(mouse)
            && let Some(setup) = app.blender_import_setup.as_mut()
        {
            setup.chunk_size_cursor = setup.chunk_size.len();
            setup.chunk_size_selection_anchor = None;
            setup.error = None;
        }
    }

    let Some(setup) = app.blender_import_setup.as_mut() else {
        return true;
    };
    if is_key_pressed(KeyCode::Backspace) {
        if !delete_text_selection(
            &mut setup.chunk_size,
            &mut setup.chunk_size_cursor,
            &mut setup.chunk_size_selection_anchor,
        ) && setup.chunk_size_cursor > 0
        {
            let previous = prev_char_boundary(&setup.chunk_size, setup.chunk_size_cursor);
            setup
                .chunk_size
                .replace_range(previous..setup.chunk_size_cursor, "");
            setup.chunk_size_cursor = previous;
        }
        setup.error = None;
    }
    if is_key_pressed(KeyCode::Left) {
        setup.chunk_size_cursor = prev_char_boundary(&setup.chunk_size, setup.chunk_size_cursor);
        setup.chunk_size_selection_anchor = None;
    }
    if is_key_pressed(KeyCode::Right) {
        setup.chunk_size_cursor = next_char_boundary(&setup.chunk_size, setup.chunk_size_cursor);
        setup.chunk_size_selection_anchor = None;
    }
    while let Some(ch) = get_char_pressed() {
        if (ch.is_ascii_digit() || ch == '.') && setup.chunk_size.len() < 12 {
            insert_text_at_cursor(
                &mut setup.chunk_size,
                &mut setup.chunk_size_cursor,
                &mut setup.chunk_size_selection_anchor,
                &ch.to_string(),
            );
            setup.error = None;
        }
    }
    true
}

pub(crate) fn update_classify_dialog_input(app: &mut AppState, mouse: Vec2) -> bool {
    if app.classify_dialog.is_none() {
        return false;
    }
    if is_key_pressed(KeyCode::Escape) {
        app.classify_dialog = None;
        app.status_message = "Element classification cancelled".to_string();
        return true;
    }
    if is_key_pressed(KeyCode::Enter) {
        if !app
            .classify_dialog
            .as_ref()
            .is_some_and(|dialog| dialog.scanning)
        {
            refresh_classify_candidates(app);
        }
        return true;
    }
    let list = classify_list_rect();
    let wheel = safe_mouse_wheel().1;
    if list.contains(mouse) && wheel.abs() > f32::EPSILON {
        if let Some(dialog) = app.classify_dialog.as_mut() {
            let max_scroll = (dialog.candidates.len() as f32 * 58.0 + 12.0 - list.h).max(0.0);
            dialog.scroll = (dialog.scroll - wheel * 44.0).clamp(0.0, max_scroll);
        }
        return true;
    }
    if is_mouse_button_pressed(MouseButton::Left) {
        if classify_review_rect().contains(mouse) {
            if !app
                .classify_dialog
                .as_ref()
                .is_some_and(|dialog| dialog.scanning)
            {
                refresh_classify_candidates(app);
            }
            return true;
        }
        if classify_apply_rect().contains(mouse) {
            apply_classify_candidates(app);
            return true;
        }
        if classify_cancel_rect().contains(mouse) {
            app.classify_dialog = None;
            app.status_message = "Element classification cancelled".to_string();
            return true;
        }
        if list.contains(mouse) {
            let candidate_index = app.classify_dialog.as_ref().and_then(|dialog| {
                dialog
                    .candidates
                    .iter()
                    .enumerate()
                    .find(|(index, _)| classify_candidate_rect(app, *index).contains(mouse))
                    .map(|(index, _)| index)
            });
            if let Some(index) = candidate_index
                && let Some(candidate) = app
                    .classify_dialog
                    .as_mut()
                    .and_then(|dialog| dialog.candidates.get_mut(index))
                && candidate.blocked_reason.is_none()
            {
                candidate.selected = !candidate.selected;
            }
            return true;
        }
        if classify_size_rect().contains(mouse)
            && let Some(dialog) = app.classify_dialog.as_mut()
        {
            dialog.size_cursor = dialog.size.len();
            dialog.size_selection_anchor = Some(0);
            dialog.error = None;
        }
    }
    let Some(dialog) = app.classify_dialog.as_mut() else {
        return true;
    };
    if is_key_pressed(KeyCode::Backspace) {
        if !delete_text_selection(
            &mut dialog.size,
            &mut dialog.size_cursor,
            &mut dialog.size_selection_anchor,
        ) && dialog.size_cursor > 0
        {
            let previous = prev_char_boundary(&dialog.size, dialog.size_cursor);
            dialog.size.replace_range(previous..dialog.size_cursor, "");
            dialog.size_cursor = previous;
        }
        dialog.error = None;
    }
    if is_key_pressed(KeyCode::Left) {
        dialog.size_cursor = prev_char_boundary(&dialog.size, dialog.size_cursor);
        dialog.size_selection_anchor = None;
    }
    if is_key_pressed(KeyCode::Right) {
        dialog.size_cursor = next_char_boundary(&dialog.size, dialog.size_cursor);
        dialog.size_selection_anchor = None;
    }
    while let Some(ch) = get_char_pressed() {
        if (ch.is_ascii_digit() || ch == '.') && dialog.size.len() < 12 {
            insert_text_at_cursor(
                &mut dialog.size,
                &mut dialog.size_cursor,
                &mut dialog.size_selection_anchor,
                &ch.to_string(),
            );
            dialog.error = None;
        }
    }
    true
}

pub(crate) fn update_oversized_chunk_dialog_input(app: &mut AppState, mouse: Vec2) -> bool {
    if app.oversized_chunk_dialog.is_none() {
        return false;
    }
    if is_key_pressed(KeyCode::Escape) {
        app.oversized_chunk_dialog = None;
        app.status_message = "Oversized-element review cancelled".to_string();
        return true;
    }
    if is_key_pressed(KeyCode::Enter) {
        refresh_oversized_chunk_candidates(app);
        return true;
    }
    let list = oversized_chunk_list_rect();
    let wheel = safe_mouse_wheel().1;
    if list.contains(mouse) && wheel.abs() > f32::EPSILON {
        if let Some(dialog) = app.oversized_chunk_dialog.as_mut() {
            let max_scroll = (dialog.candidates.len() as f32 * 58.0 + 12.0 - list.h).max(0.0);
            dialog.scroll = (dialog.scroll - wheel * 44.0).clamp(0.0, max_scroll);
        }
        return true;
    }
    if is_mouse_button_pressed(MouseButton::Left) {
        if oversized_chunk_rescan_rect().contains(mouse) {
            refresh_oversized_chunk_candidates(app);
            return true;
        }
        if oversized_chunk_apply_rect().contains(mouse) {
            start_oversized_chunking(app);
            return true;
        }
        if oversized_chunk_cancel_rect().contains(mouse) {
            app.oversized_chunk_dialog = None;
            app.status_message = "Oversized-element review cancelled".to_string();
            return true;
        }
        if list.contains(mouse) {
            let candidate_index = app.oversized_chunk_dialog.as_ref().and_then(|dialog| {
                dialog
                    .candidates
                    .iter()
                    .enumerate()
                    .find(|(index, _)| oversized_chunk_candidate_rect(app, *index).contains(mouse))
                    .map(|(index, _)| index)
            });
            if let Some(index) = candidate_index
                && let Some(candidate) = app
                    .oversized_chunk_dialog
                    .as_mut()
                    .and_then(|dialog| dialog.candidates.get_mut(index))
                && candidate.blocked_reason.is_none()
            {
                candidate.selected = !candidate.selected;
            }
            return true;
        }
        if oversized_chunk_size_rect().contains(mouse)
            && let Some(dialog) = app.oversized_chunk_dialog.as_mut()
        {
            dialog.chunk_size_cursor = dialog.chunk_size.len();
            dialog.chunk_size_selection_anchor = Some(0);
            dialog.error = None;
        }
    }
    let Some(dialog) = app.oversized_chunk_dialog.as_mut() else {
        return true;
    };
    if is_key_pressed(KeyCode::Backspace) {
        if !delete_text_selection(
            &mut dialog.chunk_size,
            &mut dialog.chunk_size_cursor,
            &mut dialog.chunk_size_selection_anchor,
        ) && dialog.chunk_size_cursor > 0
        {
            let previous = prev_char_boundary(&dialog.chunk_size, dialog.chunk_size_cursor);
            dialog
                .chunk_size
                .replace_range(previous..dialog.chunk_size_cursor, "");
            dialog.chunk_size_cursor = previous;
        }
        dialog.error = None;
    }
    if is_key_pressed(KeyCode::Left) {
        dialog.chunk_size_cursor = prev_char_boundary(&dialog.chunk_size, dialog.chunk_size_cursor);
        dialog.chunk_size_selection_anchor = None;
    }
    if is_key_pressed(KeyCode::Right) {
        dialog.chunk_size_cursor = next_char_boundary(&dialog.chunk_size, dialog.chunk_size_cursor);
        dialog.chunk_size_selection_anchor = None;
    }
    while let Some(ch) = get_char_pressed() {
        if (ch.is_ascii_digit() || ch == '.') && dialog.chunk_size.len() < 12 {
            insert_text_at_cursor(
                &mut dialog.chunk_size,
                &mut dialog.chunk_size_cursor,
                &mut dialog.chunk_size_selection_anchor,
                &ch.to_string(),
            );
            dialog.error = None;
        }
    }
    true
}

pub(crate) fn update_dff_replace_choice_dialog_input(app: &mut AppState, mouse: Vec2) -> bool {
    if app.dff_replace_choice_dialog.is_none() {
        return false;
    }
    let rect = dff_replace_choice_dialog_rect();
    let instance_rect = Rect::new(rect.x + rect.w - 358.0, rect.y + rect.h - 50.0, 110.0, 32.0);
    let unique_rect = Rect::new(rect.x + rect.w - 236.0, rect.y + rect.h - 50.0, 120.0, 32.0);
    let cancel_rect = Rect::new(rect.x + rect.w - 104.0, rect.y + rect.h - 50.0, 76.0, 32.0);
    let mut choice = None;
    if is_mouse_button_pressed(MouseButton::Left) {
        if instance_rect.contains(mouse) {
            choice = Some(false);
        } else if unique_rect.contains(mouse) {
            choice = Some(true);
        } else if cancel_rect.contains(mouse) || !rect.contains(mouse) {
            app.dff_replace_choice_dialog = None;
            return true;
        }
    }
    if is_key_pressed(KeyCode::Escape) {
        app.dff_replace_choice_dialog = None;
        return true;
    }
    if is_key_pressed(KeyCode::Enter) {
        choice = Some(false);
    }
    if let Some(make_unique) = choice {
        if let Some(dialog) = app.dff_replace_choice_dialog.take() {
            match dialog.kind {
                ReplacementAssetKind::Dff => replace_selected_dff(app, dialog.path, make_unique),
                ReplacementAssetKind::Col => replace_selected_col(app, dialog.path, make_unique),
            }
        }
        return true;
    }
    true
}

pub(crate) fn update_element_id_rename_dialog_input(app: &mut AppState, mouse: Vec2) -> bool {
    if app.element_id_rename_dialog.is_none() {
        return false;
    }
    let rect = element_id_rename_dialog_rect();
    let rename_rect = Rect::new(rect.x + rect.w - 496.0, rect.y + rect.h - 50.0, 128.0, 32.0);
    let keep_rect = Rect::new(rect.x + rect.w - 356.0, rect.y + rect.h - 50.0, 128.0, 32.0);
    let unique_rect = Rect::new(rect.x + rect.w - 216.0, rect.y + rect.h - 50.0, 116.0, 32.0);
    let cancel_rect = Rect::new(rect.x + rect.w - 88.0, rect.y + rect.h - 50.0, 64.0, 32.0);
    let mut choice = None;
    if is_mouse_button_pressed(MouseButton::Left) {
        if rename_rect.contains(mouse) {
            choice = Some(ElementIdRenameMode::RenameAssets);
        } else if keep_rect.contains(mouse) {
            choice = Some(ElementIdRenameMode::KeepOldAssets);
        } else if unique_rect.contains(mouse) {
            choice = Some(ElementIdRenameMode::MakeUnique);
        } else if cancel_rect.contains(mouse) || !rect.contains(mouse) {
            app.element_id_rename_dialog = None;
            return true;
        }
    }
    if is_key_pressed(KeyCode::Escape) {
        app.element_id_rename_dialog = None;
        return true;
    }
    if is_key_pressed(KeyCode::Enter) {
        choice = Some(ElementIdRenameMode::KeepOldAssets);
    }
    if let Some(mode) = choice {
        apply_element_id_rename(app, mode);
        return true;
    }
    true
}

pub(crate) fn update_element_replace_with_dialog_input(
    app: &mut AppState,
    viewport: Rect,
    mouse: Vec2,
) -> bool {
    let Some(dialog) = app.element_replace_with_dialog.as_ref() else {
        return false;
    };
    if dialog.picking_scene {
        if is_key_pressed(KeyCode::Escape) || is_mouse_button_pressed(MouseButton::Right) {
            if let Some(dialog) = app.element_replace_with_dialog.as_mut() {
                dialog.picking_scene = false;
            }
            app.status_message = "Returned to Replace with search".to_string();
        } else if viewport.contains(mouse) && is_mouse_button_pressed(MouseButton::Left) {
            if let Some(index) = pick_scene_element(app, viewport, mouse) {
                let target = app.placements[index].id.clone();
                let sources = app
                    .element_replace_with_dialog
                    .as_ref()
                    .map(|dialog| dialog.source_indices.clone())
                    .unwrap_or_default();
                app.element_replace_with_dialog = None;
                replace_elements_with_id(app, &target, &sources);
            } else {
                app.status_message =
                    "No scene element under the pointer; click a model or press Esc".to_string();
            }
        }
        return true;
    }

    if is_key_pressed(KeyCode::Escape) {
        app.element_replace_with_dialog = None;
        return true;
    }
    let search_before = app
        .element_replace_with_dialog
        .as_ref()
        .map(|dialog| dialog.search.clone())
        .unwrap_or_default();
    if let Some(dialog) = app.element_replace_with_dialog.as_mut() {
        while let Some(ch) = get_char_pressed() {
            if !ch.is_control() {
                insert_text_at_cursor(
                    &mut dialog.search,
                    &mut dialog.cursor,
                    &mut dialog.selection_anchor,
                    &ch.to_string(),
                );
            }
        }
        if is_key_pressed(KeyCode::Backspace)
            && !delete_text_selection(
                &mut dialog.search,
                &mut dialog.cursor,
                &mut dialog.selection_anchor,
            )
            && dialog.cursor > 0
        {
            let previous = prev_char_boundary(&dialog.search, dialog.cursor);
            dialog.search.replace_range(previous..dialog.cursor, "");
            dialog.cursor = previous;
        }
        if dialog.search != search_before {
            dialog.scroll = 0.0;
            dialog.selected_target = None;
        }
    }
    let query = app
        .element_replace_with_dialog
        .as_ref()
        .map(|dialog| dialog.search.as_str())
        .unwrap_or_default();
    let options = element_replace_target_ids(app, query);
    let max_scroll = options.len().saturating_sub(8) as f32;
    let wheel = mouse_wheel().1;
    if wheel.abs() > 0.0 {
        if let Some(dialog) = app.element_replace_with_dialog.as_mut() {
            dialog.scroll = (dialog.scroll - wheel * 3.0).clamp(0.0, max_scroll);
        }
    }
    if is_mouse_button_pressed(MouseButton::Left) {
        if element_replace_with_pick_rect().contains(mouse) {
            if let Some(dialog) = app.element_replace_with_dialog.as_mut() {
                dialog.picking_scene = true;
            }
            app.status_message =
                "Click the replacement element in the scene; Esc returns".to_string();
            return true;
        }
        if element_replace_with_cancel_rect().contains(mouse) {
            app.element_replace_with_dialog = None;
            return true;
        }
        let start = app
            .element_replace_with_dialog
            .as_ref()
            .map(|dialog| dialog.scroll.floor() as usize)
            .unwrap_or(0);
        for row in 0..8 {
            if element_replace_with_row_rect(row).contains(mouse) {
                if let Some(target) = options.get(start + row).cloned() {
                    if let Some(dialog) = app.element_replace_with_dialog.as_mut() {
                        dialog.selected_target = Some(target);
                    }
                }
                return true;
            }
        }
        if element_replace_with_apply_rect().contains(mouse) {
            let selection = app.element_replace_with_dialog.as_ref().and_then(|dialog| {
                dialog.selected_target.clone().or_else(|| {
                    options
                        .iter()
                        .find(|id| id.eq_ignore_ascii_case(dialog.search.trim()))
                        .cloned()
                })
            });
            if let Some(target) = selection {
                let sources = app
                    .element_replace_with_dialog
                    .as_ref()
                    .map(|dialog| dialog.source_indices.clone())
                    .unwrap_or_default();
                app.element_replace_with_dialog = None;
                replace_elements_with_id(app, &target, &sources);
            } else {
                app.status_message = "Choose a replacement model ID first".to_string();
            }
            return true;
        }
    }
    true
}

pub(crate) fn update_missing_texture_dialog_input(app: &mut AppState, mouse: Vec2) -> bool {
    let Some(dialog) = app.missing_texture_dialog.as_ref() else {
        return false;
    };
    let rect = missing_texture_dialog_rect();
    let cancel_rect = Rect::new(rect.x + rect.w - 116.0, rect.y + rect.h - 50.0, 88.0, 32.0);
    let mut selected = None;
    if is_mouse_button_pressed(MouseButton::Left) {
        for (idx, candidate) in dialog.candidates.iter().take(5).enumerate() {
            if missing_texture_candidate_rect(idx).contains(mouse) {
                selected = Some(MissingTextureCandidate {
                    txd_name: candidate.txd_name.clone(),
                    source_texture_name: candidate.source_texture_name.clone(),
                });
                break;
            }
        }
        if cancel_rect.contains(mouse) || !rect.contains(mouse) {
            app.missing_texture_dialog = None;
            return true;
        }
    }
    if is_key_pressed(KeyCode::Escape) {
        app.missing_texture_dialog = None;
        return true;
    }
    if let Some(candidate) = selected {
        if let Some(dialog) = app.missing_texture_dialog.take() {
            let mut applied = dialog.applied;
            if apply_missing_texture_override(app, &dialog.choice, &candidate) {
                applied += 1;
            } else {
                return true;
            }
            continue_missing_texture_fixes(app, dialog.remaining, applied);
        }
        return true;
    }
    true
}

pub(crate) fn update_texture_archive_dialog_input(app: &mut AppState, mouse: Vec2) -> bool {
    let Some(dialog) = app.texture_archive_dialog.as_ref() else {
        return false;
    };
    let rect = texture_archive_dialog_rect();
    let visible_rows = ((rect.h - 180.0) / 38.0).floor().max(1.0) as usize;
    let max_scroll = dialog.textures.len().saturating_sub(visible_rows) as f32;
    if rect.contains(mouse) {
        let (_x, wheel_y) = safe_mouse_wheel();
        if wheel_y.abs() > 0.0 {
            if let Some(dialog) = app.texture_archive_dialog.as_mut() {
                dialog.scroll = (dialog.scroll - wheel_y * 3.0).clamp(0.0, max_scroll);
            }
            return true;
        }
    }
    if is_key_pressed(KeyCode::Escape) {
        app.texture_archive_dialog = None;
        return true;
    }
    if is_key_pressed(KeyCode::Up) {
        if let Some(dialog) = app.texture_archive_dialog.as_mut() {
            dialog.selected = dialog.selected.saturating_sub(1);
            dialog.scroll = dialog.scroll.min(dialog.selected as f32);
        }
        update_texture_archive_preview(app);
        return true;
    }
    if is_key_pressed(KeyCode::Down) {
        if let Some(dialog) = app.texture_archive_dialog.as_mut() {
            dialog.selected = (dialog.selected + 1).min(dialog.textures.len().saturating_sub(1));
            let bottom = dialog.scroll as usize + visible_rows;
            if dialog.selected >= bottom {
                dialog.scroll = (dialog.selected + 1).saturating_sub(visible_rows) as f32;
            }
        }
        update_texture_archive_preview(app);
        return true;
    }
    if is_mouse_button_pressed(MouseButton::Left) {
        let start = dialog.scroll.floor() as usize;
        for row in 0..visible_rows {
            let idx = start + row;
            if idx >= dialog.textures.len() {
                break;
            }
            if texture_archive_row_rect(row).contains(mouse) {
                if let Some(dialog) = app.texture_archive_dialog.as_mut() {
                    dialog.selected = idx;
                }
                update_texture_archive_preview(app);
                return true;
            }
        }
        if texture_archive_add_rect().contains(mouse) {
            let kind = DffPickerKind::TextureAdd {
                definition_id: dialog.definition_id.clone(),
                txd_name: dialog.txd_name.clone(),
            };
            open_texture_image_picker(app, kind);
            return true;
        }
        if texture_archive_replace_rect().contains(mouse) {
            if let Some(entry) = dialog.textures.get(dialog.selected) {
                let kind = DffPickerKind::TextureReplace {
                    definition_id: dialog.definition_id.clone(),
                    txd_name: dialog.txd_name.clone(),
                    texture_name: entry.name.clone(),
                };
                open_texture_image_picker(app, kind);
            } else {
                app.status_message = "Select a texture before replacing".to_string();
            }
            return true;
        }
        if texture_archive_close_rect().contains(mouse) || !rect.contains(mouse) {
            app.texture_archive_dialog = None;
            return true;
        }
    }
    true
}

pub(crate) fn update_dff_prelight_import_dialog_input(app: &mut AppState, mouse: Vec2) -> bool {
    let Some(dialog) = app.dff_prelight_import_dialog.as_ref() else {
        return false;
    };
    let rect = dff_prelight_import_dialog_rect();
    let list_rect = Rect::new(rect.x + 24.0, rect.y + 108.0, rect.w - 48.0, rect.h - 176.0);
    let visible_rows = ((list_rect.h - 8.0) / 34.0).max(1.0) as usize;
    let max_scroll = dialog.entries.len().saturating_sub(visible_rows) as f32;
    let (_, wheel_y) = safe_mouse_wheel();
    if rect.contains(mouse) && wheel_y.abs() > 0.01 {
        if let Some(dialog) = app.dff_prelight_import_dialog.as_mut() {
            dialog.scroll = (dialog.scroll - wheel_y * 3.0).clamp(0.0, max_scroll);
        }
        return true;
    }
    if is_key_pressed(KeyCode::Escape) {
        app.dff_prelight_import_dialog = None;
        app.status_message = "Import lighting cancelled".to_string();
        return true;
    }
    if is_key_pressed(KeyCode::Up) {
        if let Some(dialog) = app.dff_prelight_import_dialog.as_mut() {
            dialog.selected = dialog.selected.saturating_sub(1);
            dialog.scroll = dialog.scroll.min(dialog.selected as f32);
        }
        return true;
    }
    if is_key_pressed(KeyCode::Down) {
        if let Some(dialog) = app.dff_prelight_import_dialog.as_mut() {
            dialog.selected = (dialog.selected + 1).min(dialog.entries.len().saturating_sub(1));
            let bottom = dialog.scroll as usize + visible_rows;
            if dialog.selected >= bottom {
                dialog.scroll = (dialog.selected + 1).saturating_sub(visible_rows) as f32;
            }
        }
        return true;
    }
    if is_key_pressed(KeyCode::Enter) {
        if let Some(dialog) = app.dff_prelight_import_dialog.take() {
            if let Some(entry) = dialog.entries.get(dialog.selected).cloned() {
                import_selected_prelight_from_img_entry(app, entry);
            }
        }
        return true;
    }
    if is_mouse_button_pressed(MouseButton::Left) {
        let start = dialog.scroll.floor() as usize;
        for row in 0..visible_rows {
            let idx = start + row;
            if idx >= dialog.entries.len() {
                break;
            }
            if dff_prelight_import_row_rect(row).contains(mouse) {
                if let Some(dialog) = app.dff_prelight_import_dialog.as_mut() {
                    dialog.selected = idx;
                }
                return true;
            }
        }
        if dff_prelight_import_apply_rect().contains(mouse) {
            if let Some(dialog) = app.dff_prelight_import_dialog.take() {
                if let Some(entry) = dialog.entries.get(dialog.selected).cloned() {
                    import_selected_prelight_from_img_entry(app, entry);
                }
            }
            return true;
        }
        if dff_prelight_import_close_rect().contains(mouse) || !rect.contains(mouse) {
            app.dff_prelight_import_dialog = None;
            app.status_message = "Import lighting cancelled".to_string();
            return true;
        }
    }
    true
}

pub(crate) fn update_missing_col_dialog_input(app: &mut AppState, mouse: Vec2) -> bool {
    if app.missing_col_dialog.is_none() {
        return false;
    }
    let rect = missing_col_dialog_rect();
    let fill_rect = Rect::new(rect.x + rect.w - 294.0, rect.y + rect.h - 50.0, 164.0, 32.0);
    let cancel_rect = Rect::new(rect.x + rect.w - 116.0, rect.y + rect.h - 50.0, 88.0, 32.0);
    if is_mouse_button_pressed(MouseButton::Left) {
        if fill_rect.contains(mouse) {
            if let Some(dialog) = app.missing_col_dialog.take() {
                let assigned = assign_missing_cols_from_dff(app);
                app.status_message = format!("Assigned {assigned} missing COL value(s) from DFF");
                save_scene_as(app, dialog.target);
            }
            return true;
        }
        if cancel_rect.contains(mouse) || !rect.contains(mouse) {
            app.missing_col_dialog = None;
            return true;
        }
    }
    if is_key_pressed(KeyCode::Escape) {
        app.missing_col_dialog = None;
        return true;
    }
    if is_key_pressed(KeyCode::Enter) {
        if let Some(dialog) = app.missing_col_dialog.take() {
            let assigned = assign_missing_cols_from_dff(app);
            app.status_message = format!("Assigned {assigned} missing COL value(s) from DFF");
            save_scene_as(app, dialog.target);
        }
        return true;
    }
    true
}

pub(crate) fn update_lod_batch_dialog_input(app: &mut AppState, mouse: Vec2) -> bool {
    if app.lod_batch_dialog.is_none() {
        return false;
    }
    let rect = lod_batch_dialog_rect();
    let list = lod_batch_list_rect();
    let visible_rows = (list.h / 30.0).max(1.0) as usize;
    let (_, wheel_y) = safe_mouse_wheel();
    if wheel_y.abs() > f32::EPSILON && list.contains(mouse) {
        if let Some(dialog) = app.lod_batch_dialog.as_mut() {
            let max_scroll = dialog.candidates.len().saturating_sub(visible_rows) as f32;
            dialog.scroll = (dialog.scroll - wheel_y * 3.0).clamp(0.0, max_scroll);
        }
    }
    if is_mouse_button_pressed(MouseButton::Left) {
        if lod_batch_minimum_size_rect().contains(mouse) {
            if let Some(dialog) = app.lod_batch_dialog.as_mut() {
                dialog.cursor = dialog.minimum_size.len();
                dialog.selection_anchor = None;
            }
            return true;
        }
        if lod_batch_missing_only_rect().contains(mouse)
            && app
                .lod_batch_dialog
                .as_ref()
                .is_some_and(|dialog| dialog.mode != LodBatchMode::GenerateSelection)
        {
            if let Some(dialog) = app.lod_batch_dialog.as_mut() {
                dialog.mode = LodBatchMode::GenerateSceneMissing;
            }
            app.status_message =
                "Generate LODs will include only scene elements currently missing an LOD."
                    .to_string();
            return true;
        }
        if lod_batch_regenerate_all_rect().contains(mouse)
            && app
                .lod_batch_dialog
                .as_ref()
                .is_some_and(|dialog| dialog.mode != LodBatchMode::GenerateSelection)
        {
            if let Some(dialog) = app.lod_batch_dialog.as_mut() {
                dialog.mode = LodBatchMode::RegenerateScene;
            }
            app.status_message =
                "Generate LODs will replace existing LODs and generate missing ones.".to_string();
            return true;
        }
        if lod_batch_continue_rect().contains(mouse) {
            continue_lod_batch_dialog(app);
            return true;
        }
        if lod_batch_cancel_rect().contains(mouse) || !rect.contains(mouse) {
            app.lod_batch_dialog = None;
            app.status_message = "LOD generation cancelled.".to_string();
            return true;
        }
    }
    if is_key_pressed(KeyCode::Escape) {
        app.lod_batch_dialog = None;
        app.status_message = "LOD generation cancelled.".to_string();
        return true;
    }
    if is_key_pressed(KeyCode::Enter) {
        continue_lod_batch_dialog(app);
        return true;
    }
    if let Some(dialog) = app.lod_batch_dialog.as_mut() {
        dialog.cursor = clamp_char_boundary(&dialog.minimum_size, dialog.cursor);
        if handle_text_clipboard_shortcuts(
            &mut dialog.minimum_size,
            &mut dialog.cursor,
            &mut dialog.selection_anchor,
        ) {
            drain_text_input();
            return true;
        }
        if is_key_pressed(KeyCode::Home) {
            dialog.cursor = 0;
            dialog.selection_anchor = None;
        }
        if is_key_pressed(KeyCode::End) {
            dialog.cursor = dialog.minimum_size.len();
            dialog.selection_anchor = None;
        }
        if is_key_pressed(KeyCode::Left) {
            dialog.cursor = prev_char_boundary(&dialog.minimum_size, dialog.cursor);
            dialog.selection_anchor = None;
        }
        if is_key_pressed(KeyCode::Right) {
            dialog.cursor = next_char_boundary(&dialog.minimum_size, dialog.cursor);
            dialog.selection_anchor = None;
        }
        if is_key_pressed(KeyCode::Backspace)
            && !delete_text_selection(
                &mut dialog.minimum_size,
                &mut dialog.cursor,
                &mut dialog.selection_anchor,
            )
            && dialog.cursor > 0
        {
            let previous = prev_char_boundary(&dialog.minimum_size, dialog.cursor);
            dialog
                .minimum_size
                .replace_range(previous..dialog.cursor, "");
            dialog.cursor = previous;
        }
        if is_key_pressed(KeyCode::Delete)
            && !delete_text_selection(
                &mut dialog.minimum_size,
                &mut dialog.cursor,
                &mut dialog.selection_anchor,
            )
            && dialog.cursor < dialog.minimum_size.len()
        {
            let next = next_char_boundary(&dialog.minimum_size, dialog.cursor);
            dialog.minimum_size.replace_range(dialog.cursor..next, "");
        }
        while let Some(ch) = get_char_pressed() {
            if handle_text_control_char(
                &mut dialog.minimum_size,
                &mut dialog.cursor,
                &mut dialog.selection_anchor,
                ch,
            ) {
                continue;
            }
            if ch.is_ascii_digit() || ch == '.' {
                insert_text_at_cursor(
                    &mut dialog.minimum_size,
                    &mut dialog.cursor,
                    &mut dialog.selection_anchor,
                    &ch.to_string(),
                );
            }
        }
    }
    true
}

pub(crate) fn run_confirm_action(app: &mut AppState, action: ConfirmAction) {
    match action {
        ConfirmAction::Load(path) => start_load_resource_checked(app, path, false),
        ConfirmAction::OpenEditingAsset(row) => editing_open_asset_row(app, row),
        ConfirmAction::OpenEditingFile(path) => open_editing_file_unchecked(app, path),
        ConfirmAction::OpenEditingImgEntry(entry) => {
            open_img_entry_in_editing_unchecked(app, entry)
        }
        ConfirmAction::OpenSceneDffsInEditing { models, camera } => {
            open_scene_dffs_in_editing_unchecked(app, models, camera)
        }
        ConfirmAction::OpenEditingStagedAsset(name) => {
            let _ = open_staged_asset_in_editing_unchecked(app, &name);
        }
        ConfirmAction::ApplyEditingImgMerge {
            plan,
            overwrite_matching,
        } => start_editing_img_merge_apply(app, plan, overwrite_matching),
        ConfirmAction::SaveEditingImgWithDuplicateCleanup => {
            editing_save_img_after_duplicate_confirmation(app)
        }
        ConfirmAction::OpenSelectedVehicleCollision => {
            open_selected_vehicle_collision_editor_unchecked(app)
        }
        ConfirmAction::RestoreAutosave(path) => {
            start_load_resource_from_source(app, path, LoadSceneSource::Autosave, false)
        }
        ConfirmAction::DismissAutosave => {
            app.status_message =
                "Kept the saved resource. The recovery copy remains on disk and can be deleted in Preferences."
                    .to_string();
        }
        ConfirmAction::CleanupAutosaves => request_autosave_cleanup(app),
        ConfirmAction::DismissWarning => {}
        ConfirmAction::TeleportCamera(target) => {
            let delta = target - app.camera.pos;
            let shifted_focus = if app.camera_mode == CameraMode::Focus {
                current_camera_focus_target(app).map(|focus| focus + delta)
            } else {
                None
            };
            app.camera.pos = target;
            if app.camera_mode == CameraMode::Focus {
                app.camera_focus = shifted_focus;
            }
            app.camera.looking = false;
            app.camera.last_mouse = mouse_position().into();
            set_cursor_grab(false);
            show_mouse(true);
            save_project_camera_state(app);
            app.last_camera_persist_at = get_time();
            app.status_message = format!(
                "Camera teleported to {:.1}, {:.1}, {:.1}",
                target.x, target.y, target.z
            );
        }
        ConfirmAction::Quit => {
            app.quit_after_persist = true;
        }
        ConfirmAction::DeleteElements {
            indices,
            lod_indices,
            delete_lods,
        } => delete_elements_by_index(app, indices, lod_indices, delete_lods),
        ConfirmAction::StartInstanceLodRemoval(target_id) => {
            start_instance_lod_removal(app, target_id)
        }
        ConfirmAction::ReviewPurgeUnused => request_purge_unused_assets(app),
        ConfirmAction::PurgeUnused => start_purge_unused_assets(app),
        ConfirmAction::RebalanceImgArchives => start_img_archive_rebalance(app),
        ConfirmAction::FixLods => fix_lods_confirmed(app),
        ConfirmAction::RegenerateLods(indices) => regenerate_selected_element_lods(app, indices),
        ConfirmAction::ClearAllLods => clear_all_lods(app),
        ConfirmAction::TxdCleanup(plan) => {
            start_loaded_asset_optimization(app, plan);
        }
        ConfirmAction::RequestTxdCleanup => request_txd_cleanup(app),
        ConfirmAction::ChooseAssetOptimizationProfile => {
            show_asset_optimization_profile_choice(app)
        }
        ConfirmAction::StartAssetOptimization(profile) => {
            request_loaded_asset_optimization(app, profile);
        }
        ConfirmAction::StartBake(scope) => start_bake_pass(app, scope),
        ConfirmAction::ClearBake => {
            app.bake_job = None;
            clear_bake_pass(app);
        }
        ConfirmAction::GenerateTxd {
            source_dir,
            destination,
        } => start_generate_txd_from_folder(app, source_dir, destination),
        ConfirmAction::DefinitionOverrideEdit {
            ids,
            field,
            value,
            before,
        } => {
            if let Some(key) = override_attr_for_definition_field(field) {
                for id in ids {
                    let zone = selected_zone_for_definition(app, &id);
                    make_definition_override_writable(app, &id, zone);
                    if let Some(def) = app.definitions.get_mut(&id) {
                        mark_definition_override_attr(def, key);
                    }
                }
            }
            app.inspector_edit = Some(InspectorEdit {
                field,
                buffer: value,
                cursor: 0,
                selection_anchor: None,
                before,
            });
            apply_inspector_edit(app);
        }
        ConfirmAction::DefinitionOverrideFlag {
            ids,
            flag,
            target_enabled,
            before,
        } => apply_definition_override_flag(app, ids, flag, target_enabled, before),
        ConfirmAction::MarkTextureElementsDoubleSided {
            indices,
            texture_name,
        } => mark_texture_elements_double_sided(app, indices, &texture_name),
        ConfirmAction::ApplyPhysicsEdit {
            edit,
            conversion_indices,
            writable_definition_ids,
        } => {
            convert_physics_elements_to_objects(app, &conversion_indices);
            for id in writable_definition_ids {
                let zone = selected_zone_for_definition(app, &id);
                make_definition_override_writable(app, &id, zone);
            }
            app.inspector_edit = Some(edit);
            apply_inspector_edit(app);
        }
        ConfirmAction::SetPhysicsRoot {
            model_id,
            conversion_indices,
            before,
        } => {
            convert_physics_elements_to_objects(app, &conversion_indices);
            set_selected_physics_root_with_before(app, Some(model_id), before);
        }
    }
}

pub(crate) fn update_confirm_dialog_input(app: &mut AppState, mouse: Vec2) -> bool {
    let Some(dialog) = app.confirm_dialog.as_ref() else {
        return false;
    };
    let layout = confirm_dialog_layout(dialog);
    let mut selected: Option<&'static str> = None;
    if is_mouse_button_pressed(MouseButton::Left) {
        if layout.primary.contains(mouse) {
            selected = Some("primary");
        } else if layout.secondary.is_some_and(|rect| rect.contains(mouse)) {
            selected = Some("secondary");
        } else if layout.cancel.contains(mouse) || !layout.rect.contains(mouse) {
            selected = Some("cancel");
        }
    }
    if is_key_pressed(KeyCode::Escape) {
        selected = Some("cancel");
    }
    if is_key_pressed(KeyCode::Enter) {
        selected = Some("primary");
    }
    match selected {
        Some("primary") => {
            if let Some(mut dialog) = app.confirm_dialog.take() {
                if dialog.primary_label == "Save WIP" {
                    if save_wip_scene(app) {
                        app.pending_after_manual_save = Some(dialog.action);
                    } else {
                        app.confirm_dialog = Some(dialog);
                    }
                } else if dialog.primary_label == "Save" {
                    if save_scene_before_action(app) {
                        app.pending_after_manual_save = Some(dialog.action);
                    } else {
                        app.confirm_dialog = Some(dialog);
                    }
                } else {
                    // Release an unused secondary action before the selected
                    // action starts. Some confirmations share a large Arc-backed
                    // worker result between both choices.
                    dialog.secondary_action = None;
                    let action =
                        std::mem::replace(&mut dialog.action, ConfirmAction::DismissWarning);
                    drop(dialog);
                    run_confirm_action(app, action);
                }
            }
        }
        Some("secondary") => {
            if let Some(mut dialog) = app.confirm_dialog.take() {
                let action = dialog.secondary_action.take().unwrap_or_else(|| {
                    std::mem::replace(&mut dialog.action, ConfirmAction::DismissWarning)
                });
                dialog.action = ConfirmAction::DismissWarning;
                drop(dialog);
                run_confirm_action(app, action);
            }
        }
        Some("cancel") => {
            app.confirm_dialog = None;
            app.status_message = "Cancelled".to_string();
        }
        _ => {}
    }
    true
}

pub(crate) fn update_dff_merge_choice_dialog_input(app: &mut AppState, mouse: Vec2) -> bool {
    if app.dff_merge_choice_dialog.is_none() {
        return false;
    }
    let layout = dff_merge_choice_dialog_layout();
    let mut selected = None;
    if is_mouse_button_pressed(MouseButton::Left) {
        if layout.center.contains(mouse) {
            selected = Some(DffMergeTarget::Center);
        } else if layout.first.contains(mouse) {
            selected = Some(DffMergeTarget::First);
        } else if layout.last.contains(mouse) {
            selected = Some(DffMergeTarget::Last);
        } else if layout.cancel.contains(mouse) || !layout.rect.contains(mouse) {
            app.dff_merge_choice_dialog = None;
            app.status_message = "Cancelled".to_string();
            return true;
        }
    }
    if is_key_pressed(KeyCode::Escape) {
        app.dff_merge_choice_dialog = None;
        app.status_message = "Cancelled".to_string();
        return true;
    }
    if is_key_pressed(KeyCode::Enter) {
        selected = Some(DffMergeTarget::Center);
    }
    if let Some(target) = selected {
        app.dff_merge_choice_dialog = None;
        let before = editing_history_snapshot(app);
        if editing_merge_selected_vertices(app, target) {
            commit_editing_history(app, "Merge Vertices", before);
        }
    }
    true
}

pub(crate) fn update_dff_optimize_dialog_input(app: &mut AppState, mouse: Vec2) -> bool {
    if app.dff_optimize_dialog.is_none() {
        return false;
    }
    let layout = dff_optimize_dialog_layout();
    if is_key_pressed(KeyCode::Escape) {
        app.dff_optimize_dialog = None;
        app.status_message = "Cancelled".to_string();
        return true;
    }
    let run = is_key_pressed(KeyCode::Enter);
    if !is_mouse_button_pressed(MouseButton::Left) && !run {
        // Still swallow input so clicks do not fall through to the viewport.
        return true;
    }
    if !run {
        if layout.cancel.contains(mouse) || !layout.rect.contains(mouse) {
            app.dff_optimize_dialog = None;
            app.status_message = "Cancelled".to_string();
            return true;
        }
        let toggles = dff_optimize_toggles();
        if let Some(dialog) = app.dff_optimize_dialog.as_mut() {
            if layout.select_all.contains(mouse) {
                for toggle in &toggles {
                    (toggle.set)(&mut dialog.options, true);
                }
                return true;
            }
            if layout.select_none.contains(mouse) {
                for toggle in &toggles {
                    (toggle.set)(&mut dialog.options, false);
                }
                return true;
            }
            for (index, toggle) in toggles.iter().enumerate() {
                let Some(rect) = layout.rows.get(index) else {
                    break;
                };
                if !rect.contains(mouse) {
                    continue;
                }
                // The nested depth-sort row does nothing while its parent is off.
                if toggle.nested && !dialog.options.reorder_transparent_faces {
                    return true;
                }
                let value = !(toggle.get)(&dialog.options);
                (toggle.set)(&mut dialog.options, value);
                return true;
            }
        }
        if !layout.run.contains(mouse) {
            return true;
        }
    }
    let Some(dialog) = app.dff_optimize_dialog.take() else {
        return true;
    };
    app.dff_optimize_options = dialog.options;
    let still_open = matches!(
        app.editing.asset.as_ref(),
        Some(EditingAsset::Dff(dff)) if dff.name.eq_ignore_ascii_case(&dialog.dff_name)
    );
    if !still_open {
        app.status_message =
            "The DFF changed while the optimize dialog was open; nothing was applied".to_string();
        return true;
    }
    apply_dff_optimize(app, dialog.options);
    true
}

pub(crate) fn update_dff_txd_pair_dialog_input(app: &mut AppState, mouse: Vec2) -> bool {
    if app.dff_txd_pair_dialog.is_none() {
        return false;
    }
    let layout = dff_txd_pair_dialog_layout();
    let wheel = safe_mouse_wheel().1;
    if wheel != 0.0
        && layout.list.contains(mouse)
        && let Some(dialog) = app.dff_txd_pair_dialog.as_mut()
    {
        let max_start = dialog.missing_textures.len().saturating_sub(1) as f32;
        dialog.scroll = (dialog.scroll - wheel.signum() * 3.0).clamp(0.0, max_start);
        return true;
    }
    if is_key_pressed(KeyCode::Escape) {
        app.dff_txd_pair_dialog = None;
        app.status_message = "Continuing without a paired TXD".to_string();
        return true;
    }
    if !is_mouse_button_pressed(MouseButton::Left) {
        return true;
    }
    if layout.browse.contains(mouse) {
        start_editing_dff_txd_pair_browse(app);
        return true;
    }
    if layout.skip.contains(mouse) || !layout.rect.contains(mouse) {
        app.dff_txd_pair_dialog = None;
        app.status_message = "Continuing without a paired TXD".to_string();
    }
    true
}

fn set_dff_texture_duplicate_cursor_from_mouse(
    dialog: &mut DffTextureDuplicateDialog,
    mouse_x: f32,
    rect: Rect,
) {
    let local_x = (mouse_x - rect.x - 10.0).max(0.0);
    let mut best = dialog.buffer.len();
    let mut best_dist = f32::MAX;
    let Some(font) = FONT_REGULAR.get() else {
        dialog.cursor = dialog.buffer.len();
        return;
    };
    let scale = 16.0 / font.raster_px;
    let mut width = 0.0;
    for (idx, ch) in dialog
        .buffer
        .char_indices()
        .chain(std::iter::once((dialog.buffer.len(), '\0')))
    {
        let dist = (width - local_x).abs();
        if dist < best_dist {
            best = idx;
            best_dist = dist;
        }
        if idx < dialog.buffer.len() {
            width += glyph_advance(font, ch, scale);
        }
    }
    dialog.cursor = best;
    dialog.selection_anchor = None;
}

fn apply_dff_texture_duplicate_dialog(app: &mut AppState) {
    let Some(dialog) = app.dff_texture_duplicate_dialog.as_ref() else {
        return;
    };
    let typed = dialog.buffer.trim();
    if typed.is_empty() {
        app.status_message = if dialog.action == DffTextureNameAction::SeparateGeometry {
            "Enter a name for the separated object".to_string()
        } else {
            "Enter a new texture name".to_string()
        };
        return;
    }
    let name = if dialog.action == DffTextureNameAction::SeparateGeometry {
        typed.to_string()
    } else {
        sanitize_texture_name(typed)
    };
    let action = dialog.action;
    let material = dialog.material;
    let source_texture = dialog.source_texture.clone();
    let selection_matches = app
        .editing
        .asset
        .as_ref()
        .is_some_and(|asset| match (action, asset) {
            (
                DffTextureNameAction::Duplicate | DffTextureNameAction::RenameDff,
                EditingAsset::Dff(dff),
            ) => dff
                .raw
                .material_textures
                .get(material)
                .is_some_and(|current| current.eq_ignore_ascii_case(&source_texture)),
            (DffTextureNameAction::RenameTxd, EditingAsset::Txd(txd)) => txd
                .textures
                .get(material)
                .is_some_and(|current| current.name.eq_ignore_ascii_case(&source_texture)),
            (DffTextureNameAction::SeparateGeometry, EditingAsset::Dff(dff)) => {
                dff.name.eq_ignore_ascii_case(&source_texture)
                    && !dff_selected_face_set(dff).is_empty()
            }
            _ => false,
        });
    if !selection_matches {
        app.status_message = format!(
            "The selected texture changed; reopen {} Texture",
            if action != DffTextureNameAction::Duplicate {
                "Rename"
            } else {
                "Duplicate"
            }
        );
        app.dff_texture_duplicate_dialog = None;
        return;
    }
    let applied = match action {
        DffTextureNameAction::Duplicate => {
            editing_duplicate_selected_dff_texture(app, material, &name)
        }
        DffTextureNameAction::RenameDff => {
            editing_rename_selected_dff_texture(app, material, &name)
        }
        DffTextureNameAction::RenameTxd => {
            editing_rename_selected_txd_texture(app, material, &name)
        }
        DffTextureNameAction::SeparateGeometry => start_dff_separation(app, &name),
    };
    if applied {
        app.dff_texture_duplicate_dialog = None;
    }
}

pub(crate) fn update_dff_texture_duplicate_dialog_input(app: &mut AppState, mouse: Vec2) -> bool {
    if app.dff_texture_duplicate_dialog.is_none() {
        return false;
    }
    let layout = dff_texture_duplicate_dialog_layout();
    if is_key_pressed(KeyCode::Escape) {
        app.dff_texture_duplicate_dialog = None;
        app.status_message = "Cancelled".to_string();
        return true;
    }
    if is_key_pressed(KeyCode::Enter) {
        apply_dff_texture_duplicate_dialog(app);
        return true;
    }
    if is_mouse_button_pressed(MouseButton::Left) {
        if layout.duplicate.contains(mouse) {
            apply_dff_texture_duplicate_dialog(app);
            return true;
        }
        if layout.cancel.contains(mouse) || !layout.rect.contains(mouse) {
            app.dff_texture_duplicate_dialog = None;
            app.status_message = "Cancelled".to_string();
            return true;
        }
        if layout.input.contains(mouse) {
            if let Some(dialog) = app.dff_texture_duplicate_dialog.as_mut() {
                set_dff_texture_duplicate_cursor_from_mouse(dialog, mouse.x, layout.input);
            }
        }
    }
    if let Some(dialog) = app.dff_texture_duplicate_dialog.as_mut() {
        dialog.cursor = clamp_char_boundary(&dialog.buffer, dialog.cursor);
        if handle_text_clipboard_shortcuts(
            &mut dialog.buffer,
            &mut dialog.cursor,
            &mut dialog.selection_anchor,
        ) {
            drain_text_input();
            return true;
        }
        let ctrl = ctrl_down();
        if is_key_pressed(KeyCode::Home) {
            dialog.cursor = 0;
            dialog.selection_anchor = None;
        }
        if is_key_pressed(KeyCode::End) {
            dialog.cursor = dialog.buffer.len();
            dialog.selection_anchor = None;
        }
        if is_key_pressed(KeyCode::Left) {
            dialog.cursor = prev_char_boundary(&dialog.buffer, dialog.cursor);
            dialog.selection_anchor = None;
        }
        if is_key_pressed(KeyCode::Right) {
            dialog.cursor = next_char_boundary(&dialog.buffer, dialog.cursor);
            dialog.selection_anchor = None;
        }
        if is_key_pressed(KeyCode::Backspace)
            && !delete_text_selection(
                &mut dialog.buffer,
                &mut dialog.cursor,
                &mut dialog.selection_anchor,
            )
            && dialog.cursor > 0
        {
            let prev = if ctrl {
                prev_word_boundary(&dialog.buffer, dialog.cursor)
            } else {
                prev_char_boundary(&dialog.buffer, dialog.cursor)
            };
            dialog.buffer.replace_range(prev..dialog.cursor, "");
            dialog.cursor = prev;
        }
        if is_key_pressed(KeyCode::Delete)
            && !delete_text_selection(
                &mut dialog.buffer,
                &mut dialog.cursor,
                &mut dialog.selection_anchor,
            )
            && dialog.cursor < dialog.buffer.len()
        {
            let next = next_char_boundary(&dialog.buffer, dialog.cursor);
            dialog.buffer.replace_range(dialog.cursor..next, "");
        }
    }
    while let Some(ch) = get_char_pressed() {
        let Some(dialog) = app.dff_texture_duplicate_dialog.as_mut() else {
            break;
        };
        if handle_text_control_char(
            &mut dialog.buffer,
            &mut dialog.cursor,
            &mut dialog.selection_anchor,
            ch,
        ) {
            continue;
        }
        if !ch.is_control() {
            insert_text_at_cursor(
                &mut dialog.buffer,
                &mut dialog.cursor,
                &mut dialog.selection_anchor,
                &ch.to_string(),
            );
        }
    }
    true
}

pub(crate) fn update_dff_texture_view_dialog_input(app: &mut AppState, mouse: Vec2) -> bool {
    let Some(dialog) = app.dff_texture_view_dialog.as_ref() else {
        return false;
    };
    let layout = dff_texture_view_dialog_layout(dialog);
    if is_mouse_button_pressed(MouseButton::Left) && layout.export.contains(mouse) {
        let export = dialog.rgba.as_ref().map(|rgba| {
            (
                dialog.texture_name.clone(),
                dialog.width as u32,
                dialog.height as u32,
                rgba.clone(),
            )
        });
        if let Some((texture_name, width, height, rgba)) = export {
            if app.dff_picker_rx.is_some() {
                app.status_message = "File browser is already open".to_string();
            } else {
                let default_path =
                    load_last_dff_export_dir().join(format!("{}.png", lower(&texture_name)));
                let (tx, rx) = mpsc::channel();
                app.dff_picker_rx = Some(rx);
                app.status_message =
                    format!("Opening texture export browser for {texture_name}...");
                thread::spawn(move || {
                    let _ = tx.send((
                        DffPickerKind::ExportTexturePng {
                            texture_name,
                            width,
                            height,
                            rgba,
                        },
                        choose_export_png_path(default_path),
                    ));
                });
            }
        }
        return true;
    }
    if is_key_pressed(KeyCode::Escape)
        || is_key_pressed(KeyCode::Enter)
        || (is_mouse_button_pressed(MouseButton::Left)
            && (layout.close.contains(mouse) || !layout.rect.contains(mouse)))
    {
        app.dff_texture_view_dialog = None;
    }
    true
}

pub(crate) fn handle_inspector_click(app: &mut AppState, mouse: Vec2) -> bool {
    if app.active_tab == AppTab::Preview {
        for (slot, tab) in properties_tabs().into_iter().enumerate() {
            if properties_tab_rect(slot).contains(mouse) {
                if is_mouse_button_pressed(MouseButton::Left) {
                    app.properties_tab = tab;
                    app.properties_scroll = 0.0;
                    app.inspector_edit = None;
                }
                return true;
            }
        }
        if app.properties_tab == PropertiesTab::Settings {
            let layout = settings_panel_layout(app);
            for (idx, header) in layout.headers.iter().enumerate() {
                if header.contains(mouse) {
                    if is_mouse_button_pressed(MouseButton::Left) {
                        app.settings_panel_collapsed[idx] = !app.settings_panel_collapsed[idx];
                    }
                    return true;
                }
            }
        }
        if app.properties_tab == PropertiesTab::Settings
            && properties_snap_toggle_rect(app).contains(mouse)
        {
            if is_mouse_button_pressed(MouseButton::Left) {
                app.snap_enabled = !app.snap_enabled;
            }
            return true;
        }
        if app.properties_tab == PropertiesTab::Settings
            && properties_local_lod_toggle_rect(app).contains(mouse)
        {
            if is_mouse_button_pressed(MouseButton::Left) {
                app.show_selected_lod_local = !app.show_selected_lod_local;
                app.status_message = if app.show_selected_lod_local {
                    "Selected LOD local visibility on".to_string()
                } else {
                    "Selected LOD local visibility off".to_string()
                };
            }
            return true;
        }
        if app.properties_tab == PropertiesTab::Settings
            && properties_lod_selectable_toggle_rect(app).contains(mouse)
        {
            if is_mouse_button_pressed(MouseButton::Left) {
                app.lod_selectable = !app.lod_selectable;
                app.status_message = if app.lod_selectable {
                    "Viewport LOD picking on".to_string()
                } else {
                    "Viewport LOD picking off".to_string()
                };
            }
            return true;
        }
        if app.properties_tab == PropertiesTab::Settings
            && properties_camera_mode_toggle_rect(app).contains(mouse)
        {
            if is_mouse_button_pressed(MouseButton::Left) {
                toggle_camera_mode(app);
            }
            return true;
        }
        if app.properties_tab == PropertiesTab::Settings
            && properties_box_select_mode_rect(app).contains(mouse)
        {
            if is_mouse_button_pressed(MouseButton::Left) {
                app.box_select_mode = app.box_select_mode.next();
                app.status_message = format!("Box select mode: {}", app.box_select_mode.label());
            }
            return true;
        }
        if app.properties_tab == PropertiesTab::Settings
            && properties_box_select_minus_rect(app).contains(mouse)
        {
            if is_mouse_button_pressed(MouseButton::Left) {
                app.box_select_distance = (app.box_select_distance * 0.5).max(250.0);
                app.status_message = format!("Box select distance {:.0}", app.box_select_distance);
            }
            return true;
        }
        if app.properties_tab == PropertiesTab::Settings
            && properties_box_select_plus_rect(app).contains(mouse)
        {
            if is_mouse_button_pressed(MouseButton::Left) {
                app.box_select_distance = (app.box_select_distance * 2.0).min(64000.0);
                app.status_message = format!("Box select distance {:.0}", app.box_select_distance);
            }
            return true;
        }
        if app.properties_tab == PropertiesTab::Settings
            && properties_global_elements_rect(app).contains(mouse)
        {
            if is_mouse_button_pressed(MouseButton::Left) {
                app.global_transform.transform_elements = !app.global_transform.transform_elements;
                app.status_message = if app.global_transform.transform_elements {
                    "Global transform will include elements".to_string()
                } else {
                    "Global transform will leave elements unchanged".to_string()
                };
            }
            return true;
        }
        if app.properties_tab == PropertiesTab::Settings
            && properties_global_water_rect(app).contains(mouse)
        {
            if is_mouse_button_pressed(MouseButton::Left) {
                app.global_transform.transform_water = !app.global_transform.transform_water;
                app.status_message = if app.global_transform.transform_water {
                    "Global transform will include water".to_string()
                } else {
                    "Global transform will leave water unchanged".to_string()
                };
            }
            return true;
        }
        if app.properties_tab == PropertiesTab::Settings
            && properties_global_preview_rect(app).contains(mouse)
        {
            if is_mouse_button_pressed(MouseButton::Left) {
                app.global_transform.preview = !app.global_transform.preview;
                app.status_message = if app.global_transform.preview {
                    "Global transform preview enabled".to_string()
                } else {
                    "Global transform preview disabled".to_string()
                };
            }
            return true;
        }
        if app.properties_tab == PropertiesTab::Settings
            && properties_global_reset_rect(app).contains(mouse)
        {
            if is_mouse_button_pressed(MouseButton::Left) {
                app.global_transform = GlobalTransformState::default();
                app.status_message = "Global transform reset".to_string();
            }
            return true;
        }
        if app.properties_tab == PropertiesTab::Settings
            && properties_global_apply_rect(app).contains(mouse)
        {
            if is_mouse_button_pressed(MouseButton::Left) {
                apply_global_transform(app);
            }
            return true;
        }
    }
    if app.active_tab == AppTab::Collisions {
        if !inspector_panel_content_rect().contains(mouse) {
            return false;
        }
        if collision_open_col_editor_rect(app).contains(mouse) {
            if is_mouse_button_pressed(MouseButton::Left) {
                if selected_placement(app).is_some() {
                    open_selected_col_in_editing(app);
                } else {
                    app.status_message =
                        "Select a live element before opening COL editor".to_string();
                }
            }
            return true;
        }
    }
    if app.active_tab == AppTab::Preview && app.properties_tab == PropertiesTab::Element {
        if !properties_content_rect().contains(mouse) {
            return false;
        }
        {
            let layout = element_panel_layout(app);
            for (idx, header) in layout.headers.iter().enumerate() {
                if header.contains(mouse) {
                    if is_mouse_button_pressed(MouseButton::Left) {
                        app.element_panel_collapsed[idx] = !app.element_panel_collapsed[idx];
                        let max_scroll = element_panel_max_scroll(&element_panel_layout(app));
                        app.properties_scroll = app.properties_scroll.min(max_scroll);
                    }
                    return true;
                }
            }
            if let Some(rows) = layout.texture_rows.as_ref() {
                let entries = preview_material_entries(app, app.selected);
                for (entry, rect) in entries.iter().zip(rows) {
                    if !rect.contains(mouse) {
                        continue;
                    }
                    if is_mouse_button_pressed(MouseButton::Left) {
                        app.preview_selected_material = Some((app.selected, entry.material_index));
                        if entry.texture_id != 0 {
                            let placement_index = app.selected;
                            open_preview_texture_view_dialog(
                                app,
                                placement_index,
                                entry.material_index,
                            );
                        } else {
                            app.status_message = format!(
                                "Selected material #{} ({})",
                                entry.material_index,
                                if entry.texture_name.trim().is_empty() {
                                    "<empty>"
                                } else {
                                    entry.texture_name.as_str()
                                }
                            );
                        }
                    } else if is_mouse_button_pressed(MouseButton::Right) {
                        app.preview_selected_material = Some((app.selected, entry.material_index));
                        app.camera.looking = false;
                        set_cursor_grab(false);
                        show_mouse(true);
                        app.context_menu = Some(ContextMenu {
                            pos: mouse,
                            target: ContextMenuTarget::PreviewTexture {
                                placement: app.selected,
                                material: entry.material_index,
                            },
                        });
                    }
                    return true;
                }
            }
            if layout
                .texture_export_all
                .is_some_and(|rect| rect.contains(mouse))
            {
                if is_mouse_button_pressed(MouseButton::Left) {
                    open_preview_texture_export_all_picker(app);
                }
                return true;
            }
            if layout
                .texture_advanced
                .is_some_and(|rect| rect.contains(mouse))
            {
                if is_mouse_button_pressed(MouseButton::Left) {
                    app.preview_texture_advanced = !app.preview_texture_advanced;
                    let max_scroll = element_panel_max_scroll(&element_panel_layout(app));
                    app.properties_scroll = app.properties_scroll.min(max_scroll);
                }
                return true;
            }
            if layout
                .texture_variation
                .is_some_and(|rect| rect.contains(mouse))
            {
                if is_mouse_button_pressed(MouseButton::Left) {
                    app.preview_world_uv_variation = !app.preview_world_uv_variation;
                    refresh_preview_world_scale_uv_visual(app);
                    app.status_message = if app.preview_world_uv_variation {
                        "Multi-scale world UV variation enabled; seams remain matched".to_string()
                    } else {
                        "World UV variation disabled".to_string()
                    };
                }
                return true;
            }
            if layout
                .texture_uv_scale_minus
                .is_some_and(|rect| rect.contains(mouse))
            {
                if is_mouse_button_pressed(MouseButton::Left) {
                    app.preview_world_uv_scale =
                        (app.preview_world_uv_scale * 0.5).clamp(0.001, 1024.0);
                    refresh_preview_world_scale_uv_visual(app);
                    app.status_message = format!(
                        "World UV scale set to {}x",
                        fmt_f32(app.preview_world_uv_scale, 3)
                    );
                }
                return true;
            }
            if layout
                .texture_scope_object
                .is_some_and(|rect| rect.contains(mouse))
            {
                if is_mouse_button_pressed(MouseButton::Left) {
                    app.preview_world_uv_all_dffs = false;
                    app.status_message = "World-scale UV scope: object only".to_string();
                }
                return true;
            }
            if layout
                .texture_scope_world
                .is_some_and(|rect| rect.contains(mouse))
            {
                if is_mouse_button_pressed(MouseButton::Left) {
                    app.preview_world_uv_all_dffs = true;
                    app.status_message =
                        "World-scale UV scope: all DFFs using this texture".to_string();
                }
                return true;
            }
            if layout
                .texture_uv_scale_value
                .is_some_and(|rect| rect.contains(mouse))
            {
                if is_mouse_button_pressed(MouseButton::Left) {
                    app.preview_world_uv_scale = 1.0;
                    refresh_preview_world_scale_uv_visual(app);
                    app.status_message = "World UV scale reset to 1x".to_string();
                }
                return true;
            }
            if layout
                .texture_uv_scale_plus
                .is_some_and(|rect| rect.contains(mouse))
            {
                if is_mouse_button_pressed(MouseButton::Left) {
                    app.preview_world_uv_scale =
                        (app.preview_world_uv_scale * 2.0).clamp(0.001, 1024.0);
                    refresh_preview_world_scale_uv_visual(app);
                    app.status_message = format!(
                        "World UV scale set to {}x",
                        fmt_f32(app.preview_world_uv_scale, 3)
                    );
                }
                return true;
            }
            if layout
                .texture_world_preview
                .is_some_and(|rect| rect.contains(mouse))
            {
                if is_mouse_button_pressed(MouseButton::Left) {
                    toggle_preview_world_scale_uv_visual(app);
                }
                return true;
            }
            if layout
                .texture_world_unwrap
                .is_some_and(|rect| rect.contains(mouse))
            {
                if is_mouse_button_pressed(MouseButton::Left) {
                    apply_preview_world_scale_uv(app);
                }
                return true;
            }
        }
        for scope in [PhysicsScope::Global, PhysicsScope::PerObject] {
            if physics_scope_rect(app, scope).contains(mouse) {
                if is_mouse_button_pressed(MouseButton::Left) {
                    if app.inspector_edit.is_some() {
                        apply_inspector_edit(app);
                    }
                    app.physics_root_dropdown_open = false;
                    app.physics_scope = scope;
                    app.status_message = format!("Physics scope: {}", app.physics_scope.label());
                }
                return true;
            }
        }
        if physics_simulated_rect(app).contains(mouse) {
            if is_mouse_button_pressed(MouseButton::Left) {
                if app.inspector_edit.is_some() {
                    apply_inspector_edit(app);
                }
                let next_value = match physics_simulated_value(app) {
                    None => "true",
                    Some(true) => "false",
                    Some(false) => "",
                }
                .to_string();
                let before = ScopedHistorySnapshot::World(world_history_snapshot(app));
                if app.physics_scope == PhysicsScope::Global
                    && selected_definitions_include_readonly(app)
                {
                    prompt_definition_override_edit(
                        app,
                        InspectorField::PhysicsSimulated,
                        next_value,
                        before,
                    );
                } else {
                    app.inspector_edit = Some(InspectorEdit {
                        field: InspectorField::PhysicsSimulated,
                        cursor: next_value.len(),
                        buffer: next_value,
                        selection_anchor: None,
                        before,
                    });
                    apply_inspector_edit(app);
                }
            }
            return true;
        }
        if physics_clear_rect(app).contains(mouse) {
            if is_mouse_button_pressed(MouseButton::Left) {
                if app.inspector_edit.is_some() {
                    apply_inspector_edit(app);
                }
                if app.physics_scope == PhysicsScope::Global
                    && selected_definitions_include_readonly(app)
                {
                    app.status_message =
                        "This GTA:SA definition has no editable global physics override"
                            .to_string();
                    return true;
                }
                let before = world_history_snapshot(app);
                let mut changed = 0usize;
                match app.physics_scope {
                    PhysicsScope::Global => {
                        for id in selected_definition_ids(app) {
                            let Some(def) = app.definitions.get_mut(&id) else {
                                continue;
                            };
                            for key in PHYSICS_ATTR_KEYS {
                                changed += usize::from(def.attrs.remove(key).is_some());
                            }
                            for key in PHYSICS_ROOT_ATTR_KEYS {
                                changed += usize::from(def.attrs.remove(key).is_some());
                            }
                        }
                    }
                    PhysicsScope::PerObject => {
                        for idx in selected_live_indices(app) {
                            let Some(placement) = app.placements.get_mut(idx) else {
                                continue;
                            };
                            for key in PHYSICS_ATTR_KEYS {
                                changed += usize::from(placement.attrs.remove(key).is_some());
                            }
                            for key in PHYSICS_ROOT_ATTR_KEYS {
                                changed += usize::from(placement.attrs.remove(key).is_some());
                            }
                        }
                    }
                }
                if changed > 0 {
                    commit_world_history(app, "Clear Physics Values", before);
                    app.status_message = format!(
                        "Cleared {changed} {} physics value(s)",
                        app.physics_scope.label()
                    );
                } else {
                    app.status_message =
                        format!("No {} physics values to clear", app.physics_scope.label());
                }
            }
            return true;
        }
        let dff_actions_enabled = selected_placement(app).is_some()
            && !app
                .element_states
                .get(app.selected)
                .is_some_and(|state| state.deleted);
        let replace_dff_enabled = dff_actions_enabled && !selected_definition_is_readonly(app);
        let replace_col_enabled = replace_dff_enabled;
        let open_txd_enabled = selected_definition_id_and_txd(app).is_some();
        let find_missing_textures_enabled = selected_definition_has_missing_textures(app);
        let blender_position_enabled = selected_live_indices(app).len() >= 2;
        let assign_lod_enabled = selected_live_indices_in_selection_order(app).len() >= 2;
        if element_self_lod_rect(app).contains(mouse) {
            if is_mouse_button_pressed(MouseButton::Left) {
                if app.inspector_edit.is_some() {
                    apply_inspector_edit(app);
                }
                toggle_self_lod_for_selection(app);
            }
            return true;
        }
        if element_remove_instance_lods_rect(app).contains(mouse) {
            if is_mouse_button_pressed(MouseButton::Left) {
                if app.inspector_edit.is_some() {
                    apply_inspector_edit(app);
                }
                request_remove_lods_from_all_instances(app);
            }
            return true;
        }
        if element_lod_parent_select_rect(app).contains(mouse) {
            if is_mouse_button_pressed(MouseButton::Left) {
                if app.inspector_edit.is_some() {
                    apply_inspector_edit(app);
                }
                select_lod_parent_for_selected(app);
            }
            return true;
        }
        if element_generate_lod_rect(app).contains(mouse) {
            if is_mouse_button_pressed(MouseButton::Left) {
                if app.inspector_edit.is_some() {
                    apply_inspector_edit(app);
                }
                request_selected_element_lod(app);
            }
            return true;
        }
        if element_light_lod_rect(app).contains(mouse) {
            if is_mouse_button_pressed(MouseButton::Left) {
                if app.inspector_edit.is_some() {
                    apply_inspector_edit(app);
                }
                request_light_lod(app);
            }
            return true;
        }
        if element_export_dff_rect(app).contains(mouse) {
            if is_mouse_button_pressed(MouseButton::Left) {
                if dff_actions_enabled {
                    if app.inspector_edit.is_some() {
                        apply_inspector_edit(app);
                    }
                    open_export_dff_dialog(app);
                } else {
                    app.status_message = "Select a live element before exporting DFF".to_string();
                }
            }
            return true;
        }
        if element_open_dff_editor_rect(app).contains(mouse) {
            if is_mouse_button_pressed(MouseButton::Left) {
                if dff_actions_enabled {
                    if app.inspector_edit.is_some() {
                        apply_inspector_edit(app);
                    }
                    open_selected_dff_in_editing(app);
                } else {
                    app.status_message =
                        "Select a live element before opening DFF editor".to_string();
                }
            }
            return true;
        }
        if element_replace_dff_rect(app).contains(mouse) {
            if is_mouse_button_pressed(MouseButton::Left) {
                if replace_dff_enabled {
                    if app.inspector_edit.is_some() {
                        apply_inspector_edit(app);
                    }
                    open_replace_dff_dialog(app);
                } else if selected_definition_is_readonly(app) {
                    app.status_message = "GTA:SA fallback definitions are read-only".to_string();
                } else {
                    app.status_message = "Select a live element before replacing DFF".to_string();
                }
            }
            return true;
        }
        if element_export_col_rect(app).contains(mouse) {
            if is_mouse_button_pressed(MouseButton::Left) {
                if dff_actions_enabled {
                    if app.inspector_edit.is_some() {
                        apply_inspector_edit(app);
                    }
                    open_export_col_dialog(app);
                } else {
                    app.status_message = "Select a live element before exporting COL".to_string();
                }
            }
            return true;
        }
        if element_replace_col_rect(app).contains(mouse) {
            if is_mouse_button_pressed(MouseButton::Left) {
                if replace_col_enabled {
                    if app.inspector_edit.is_some() {
                        apply_inspector_edit(app);
                    }
                    open_replace_col_dialog(app);
                } else if selected_definition_is_readonly(app) {
                    app.status_message = "GTA:SA fallback definitions are read-only".to_string();
                } else {
                    app.status_message = "Select a live element before replacing COL".to_string();
                }
            }
            return true;
        }
        if element_assign_lod_rect(app).contains(mouse) {
            if is_mouse_button_pressed(MouseButton::Left) {
                if assign_lod_enabled {
                    if app.inspector_edit.is_some() {
                        apply_inspector_edit(app);
                    }
                    assign_lod_parent_from_selection(app);
                } else {
                    app.status_message =
                        "Select the LOD parent first, then at least one child".to_string();
                }
            }
            return true;
        }
        if element_select_same_id_rect(app).contains(mouse) {
            if is_mouse_button_pressed(MouseButton::Left) {
                if app.inspector_edit.is_some() {
                    apply_inspector_edit(app);
                }
                select_same_id_as_active(app);
            }
            return true;
        }
        if element_replace_with_rect(app).contains(mouse) {
            if is_mouse_button_pressed(MouseButton::Left) {
                if app.inspector_edit.is_some() {
                    apply_inspector_edit(app);
                }
                open_element_replace_with_dialog(app);
            }
            return true;
        }
        if element_open_col_editor_rect(app).contains(mouse) {
            if is_mouse_button_pressed(MouseButton::Left) {
                if dff_actions_enabled {
                    if app.inspector_edit.is_some() {
                        apply_inspector_edit(app);
                    }
                    open_selected_col_in_editing(app);
                } else {
                    app.status_message =
                        "Select a live element before opening COL editor".to_string();
                }
            }
            return true;
        }
        if element_merge_vertex_lighting_rect(app).contains(mouse) {
            if is_mouse_button_pressed(MouseButton::Left) {
                if app.inspector_edit.is_some() {
                    apply_inspector_edit(app);
                }
                request_day_night_variant_merge(app);
            }
            return true;
        }
        if element_open_txd_rect(app).contains(mouse) {
            if is_mouse_button_pressed(MouseButton::Left) {
                if open_txd_enabled {
                    if app.inspector_edit.is_some() {
                        apply_inspector_edit(app);
                    }
                    open_selected_txd_in_editing(app);
                } else {
                    app.status_message =
                        "Select an element with a TXD before opening textures".to_string();
                }
            }
            return true;
        }
        if find_missing_textures_enabled && element_find_missing_textures_rect(app).contains(mouse)
        {
            if is_mouse_button_pressed(MouseButton::Left) {
                if app.inspector_edit.is_some() {
                    apply_inspector_edit(app);
                }
                find_missing_textures_for_selected_definition(app);
            }
            return true;
        }
        if element_blender_position_rect(app).contains(mouse) {
            if is_mouse_button_pressed(MouseButton::Left) {
                if blender_position_enabled {
                    if app.inspector_edit.is_some() {
                        apply_inspector_edit(app);
                    }
                    write_blender_position_script(app);
                } else {
                    app.status_message =
                        "Select two or more live elements for Blender position".to_string();
                }
            }
            return true;
        }
        for (slot, element_type) in EAGLE_ELEMENT_TYPES.iter().enumerate() {
            if element_type_button_rect(app, slot).contains(mouse) {
                if is_mouse_button_pressed(MouseButton::Left) {
                    let before = world_history_snapshot(app);
                    let indices = selected_live_indices(app);
                    let changed =
                        set_element_type_for_indices(&mut app.placements, &indices, element_type);
                    if changed > 0 {
                        invalidate_outliner_labels(app);
                        rebuild_outliner_filter(app);
                        rebuild_render_cells(app);
                        commit_world_history(app, "Element Type", before);
                    }
                    app.status_message =
                        format!("Set {changed} selected element(s) to {}", element_type);
                }
                return true;
            }
        }
        for (slot, flag) in EAGLE_DEFINITION_FLAGS.iter().enumerate() {
            if definition_flag_rect(app, slot).contains(mouse) {
                if is_mouse_button_pressed(MouseButton::Left) {
                    let before = ScopedHistorySnapshot::World(world_history_snapshot(app));
                    let ids = selected_definition_ids(app);
                    let all_enabled = !ids.is_empty()
                        && ids.iter().all(|id| {
                            app.definitions
                                .get(id)
                                .is_some_and(|def| definition_flag_enabled(def, flag))
                        });
                    let target_enabled = !all_enabled;
                    if ids
                        .iter()
                        .any(|id| app.readonly_definition_ids.contains(id))
                    {
                        app.confirm_dialog = Some(ConfirmDialog {
                            action: ConfirmAction::DefinitionOverrideFlag {
                                ids,
                                flag,
                                target_enabled,
                                before,
                            },
                            title: "Override GTA:SA Model?".to_string(),
                            body: "Create override(s) for the selected GTA:SA model flag change?"
                                .to_string(),
                            detail: "The flag value will be written as <override> data in the first selected zone for each GTA:SA model.".to_string(),
                            primary_label: "Override".to_string(),
                            secondary_label: None,
                            secondary_action: None,
                        });
                        return true;
                    }
                    let mut changed = 0usize;
                    for id in ids {
                        if let Some(def) = app.definitions.get_mut(&id) {
                            set_definition_flag(def, flag, target_enabled);
                            changed += 1;
                        }
                    }
                    if changed > 0 {
                        rebuild_render_cells(app);
                        commit_scoped_history(app, "Definition Flag", before);
                        app.status_message = format!("Updated {changed} definition flag(s)");
                    }
                }
                return true;
            }
        }
        for (slot, flag) in EAGLE_PLACEMENT_OVERRIDE_FLAGS.iter().enumerate() {
            if placement_override_flag_rect(app, slot).contains(mouse) {
                if is_mouse_button_pressed(MouseButton::Left) {
                    let before = world_history_snapshot(app);
                    let indices = selected_live_indices(app);
                    let all_enabled = !indices.is_empty()
                        && indices.iter().all(|idx| {
                            app.placements.get(*idx).is_some_and(|placement| {
                                placement_override_flag_enabled(placement, flag)
                            })
                        });
                    let target_enabled = !all_enabled;
                    let mut changed = 0usize;
                    for idx in indices {
                        if let Some(placement) = app.placements.get_mut(idx) {
                            set_placement_override_flag(placement, flag, target_enabled);
                            changed += 1;
                        }
                    }
                    if changed > 0 {
                        rebuild_render_cells(app);
                        commit_world_history(app, "Placement Flag Override", before);
                        app.status_message = format!("Updated {changed} placement override(s)");
                    }
                }
                return true;
            }
        }
    }
    if let Some(action) = clicked_inspector_copy_action(app, mouse) {
        if is_mouse_button_pressed(MouseButton::Left) {
            copy_inspector_value(app, action);
        }
        return true;
    }
    if let Some(field) = clicked_inspector_field(app, mouse) {
        if is_mouse_button_pressed(MouseButton::Left) {
            start_inspector_edit(app, field);
            let rect = inspector_field_rect(app, field);
            if let Some(edit) = app.inspector_edit.as_mut() {
                set_edit_cursor_from_mouse(edit, mouse.x, rect);
            }
        }
        return true;
    }
    false
}

pub(crate) fn move_selected(app: &mut AppState, delta: Vec3) {
    let indices = selected_live_indices(app);
    if indices.is_empty() {
        return;
    }
    let before = placement_transform_history_snapshot(app, indices.iter().copied());
    for idx in indices {
        if let Some(placement) = app.placements.get_mut(idx) {
            placement.pos.x += delta.x;
            placement.pos.y += delta.y;
            placement.pos.z += delta.z;
            sync_placement_attrs(placement);
        }
    }
    rebuild_render_cells_for_placement_transforms(app, &before);
    commit_placement_transform_history(app, "Nudge", before);
}

pub(crate) fn move_selected_light(app: &mut AppState, delta: Vec3) {
    if app.selected_light >= app.lights.len() {
        return;
    }
    let before = light_history_snapshot(app);
    let reference = app
        .lights
        .get(app.selected_light)
        .and_then(|light| light_reference_placement(app, light))
        .map(|(_, placement)| placement_matrix(placement).inverse());
    if let Some(light) = app.lights.get_mut(app.selected_light) {
        let local_delta = reference
            .map(|inverse| inverse.transform_vector3(delta))
            .unwrap_or(delta);
        light.position = from_mq(to_mq(light.position) + local_delta);
    }
    mark_lights_changed(app);
    commit_light_history(app, "Nudge Light", before);
}

fn transform_interaction_viewport(app: &AppState, viewport: Rect) -> Rect {
    if app.active_tab == AppTab::Editing {
        editing_preview_rect(app)
    } else {
        viewport
    }
}

pub(crate) fn apply_gizmo_drag(app: &mut AppState, viewport: Rect, mouse: Vec2) {
    let Some(drag) = app.gizmo_drag.as_ref() else {
        return;
    };
    if drag.target == GizmoTarget::Element && drag.element_start_positions.is_empty() {
        return;
    }
    if drag.target == GizmoTarget::Light && app.selected_light >= app.lights.len() {
        return;
    }
    let axis_dir = selected_axis_vector(app, drag.axis);
    let amount = axis_drag_amount(
        app,
        viewport,
        to_mq(drag.start_pos),
        axis_dir,
        drag.start_mouse,
        mouse,
    );
    match app.transform_mode {
        TransformMode::Select => {}
        TransformMode::Move => {
            let mut world_units = amount;
            if app.snap_enabled {
                world_units = snap_delta(world_units, app.snap_move);
            }
            let delta = axis_dir * world_units;
            match drag.target {
                GizmoTarget::Element => {
                    for (idx, start_pos) in &drag.element_start_positions {
                        if let Some(placement) = app.placements.get_mut(*idx) {
                            placement.pos = from_mq(to_mq(*start_pos) + delta);
                            sync_placement_attrs(placement);
                        }
                    }
                }
                GizmoTarget::CullZone => {
                    if let Some(zone) = app.cull_zones.get_mut(app.selected_cull) {
                        zone.center = from_mq(to_mq(drag.start_pos) + delta);
                    }
                }
                GizmoTarget::Light => {
                    let inverse = app
                        .lights
                        .get(app.selected_light)
                        .and_then(|light| light_reference_placement(app, light))
                        .map(|(_, placement)| placement_matrix(placement).inverse());
                    if let Some(light) = app.lights.get_mut(app.selected_light) {
                        let world = to_mq(drag.start_pos) + delta;
                        light.position = from_mq(
                            inverse
                                .map(|matrix| matrix.transform_point3(world))
                                .unwrap_or(world),
                        );
                    }
                }
                GizmoTarget::CollisionVertex => {
                    let target = from_mq(to_mq(drag.start_pos) + delta);
                    let result = if app.active_tab == AppTab::Editing {
                        set_selected_editing_col_vertex_position(app, target)
                    } else {
                        set_selected_collision_tab_vertex_position_live(app, to_mq(target))
                    };
                    if let Err(err) = result {
                        app.status_message = err;
                    }
                }
                GizmoTarget::CollisionPrimitive => {
                    let target = from_mq(to_mq(drag.start_pos) + delta);
                    if let Err(err) = set_selected_editing_col_primitive_position(app, target) {
                        app.status_message = err;
                    }
                }
                GizmoTarget::DffVertex => {
                    let target = from_mq(to_mq(drag.start_pos) + delta);
                    if let Err(err) = set_selected_editing_dff_vertex_position(app, target) {
                        app.status_message = err;
                    }
                }
                GizmoTarget::DffPivot => {
                    set_editing_dff_freeform_pivot(
                        app,
                        DffFreeformPivot {
                            position: from_mq(to_mq(drag.start_pos) + delta),
                            rotation: drag.start_rot,
                        },
                    );
                }
                GizmoTarget::DffBooleanBox => {
                    let target = from_mq(to_mq(drag.start_pos) + delta);
                    if let Err(err) = set_selected_editing_dff_boolean_box_position(app, target) {
                        app.status_message = err;
                    }
                }
                GizmoTarget::Dff2dEffect => {
                    let target = from_mq(to_mq(drag.start_pos) + delta);
                    if let Err(err) = set_selected_editing_dff_2dfx_position(app, target) {
                        app.status_message = err;
                    }
                }
                GizmoTarget::RacePoint => {
                    set_selected_race_point_position(app, to_mq(drag.start_pos) + delta);
                }
            }
        }
        TransformMode::Rotate => {
            let mut degrees = ring_drag_degrees(app, viewport, drag.start_mouse, mouse);
            if drag.axis == GizmoAxis::Z {
                degrees = -degrees;
            }
            if app.snap_enabled {
                degrees = snap_delta(degrees, app.snap_rotate);
            }
            match drag.target {
                GizmoTarget::Element => {
                    let rot = Mat4::from_axis_angle(
                        selected_axis_vector(app, drag.axis),
                        degrees.to_radians(),
                    );
                    let pivot = to_mq(drag.start_pos);
                    for (idx, start_pos) in &drag.element_start_positions {
                        if let Some(placement) = app.placements.get_mut(*idx) {
                            let rotated_pos =
                                pivot + rot.transform_vector3(to_mq(*start_pos) - pivot);
                            placement.pos = from_mq(rotated_pos);
                            if let Some((_, start_rot)) = drag
                                .element_start_rots
                                .iter()
                                .find(|(rot_idx, _)| rot_idx == idx)
                            {
                                placement.rot = *start_rot;
                                match drag.axis {
                                    GizmoAxis::X => placement.rot.x += degrees,
                                    GizmoAxis::Y => placement.rot.y += degrees,
                                    GizmoAxis::Z => placement.rot.z += degrees,
                                }
                            }
                            sync_placement_attrs(placement);
                        }
                    }
                }
                GizmoTarget::Light => {
                    let inverse_rotation = app
                        .lights
                        .get(app.selected_light)
                        .and_then(|light| light_reference_placement(app, light))
                        .map(|(_, placement)| placement_rotation_matrix(placement).inverse());
                    if let Some(light) = app.lights.get_mut(app.selected_light) {
                        let mut direction =
                            vec3(drag.start_rot.x, drag.start_rot.y, drag.start_rot.z);
                        if direction.length_squared() < 0.0001 {
                            direction = Vec3::NEG_Z;
                        }
                        let rotation_axis = if app.transform_space == TransformSpace::Local {
                            light_local_axis(drag.start_rot, drag.axis)
                        } else {
                            axis_vector(drag.axis)
                        };
                        let rot = Mat4::from_axis_angle(rotation_axis, degrees.to_radians());
                        let direction = rot.transform_vector3(direction).normalize_or_zero();
                        light.direction = from_mq(
                            inverse_rotation
                                .map(|rotation| rotation.transform_vector3(direction))
                                .unwrap_or(direction)
                                .normalize_or_zero(),
                        );
                    }
                }
                GizmoTarget::CollisionPrimitive => {
                    let selected_kind = app.editing.asset.as_ref().and_then(|asset| match asset {
                        EditingAsset::Col(col) => {
                            col.selected_primitive.map(|selected| selected.kind)
                        }
                        _ => None,
                    });
                    match selected_kind {
                        Some(CollisionPrimitiveKind::Capsule) => {
                            let half_axis = to_mq(drag.start_rot);
                            if half_axis.length_squared() > 1.0e-8 {
                                let rot = Mat4::from_axis_angle(
                                    axis_vector(drag.axis),
                                    degrees.to_radians(),
                                );
                                let rotated = from_mq(rot.transform_vector3(half_axis));
                                if let Err(err) = set_selected_editing_col_capsule_orientation(
                                    app,
                                    drag.start_pos,
                                    rotated,
                                ) {
                                    app.status_message = err;
                                }
                            }
                        }
                        Some(CollisionPrimitiveKind::Box | CollisionPrimitiveKind::Cuboid) => {
                            let mut rotation = drag.start_rot;
                            match drag.axis {
                                GizmoAxis::X => rotation.x += degrees,
                                GizmoAxis::Y => rotation.y += degrees,
                                GizmoAxis::Z => rotation.z += degrees,
                            }
                            if let Err(err) = set_selected_editing_col_box_rotation(app, rotation) {
                                app.status_message = err;
                            }
                        }
                        _ => {}
                    }
                }
                GizmoTarget::CollisionVertex
                | GizmoTarget::CullZone
                | GizmoTarget::DffVertex
                | GizmoTarget::DffBooleanBox
                | GizmoTarget::RacePoint => {}
                GizmoTarget::DffPivot => {
                    let start = dff_pivot_rotation_matrix(drag.start_rot);
                    let incremental =
                        Mat4::from_axis_angle(axis_vector(drag.axis), degrees.to_radians());
                    // The rings are drawn in pivot-local space, so apply the
                    // delta in that exact same basis to keep visuals and math
                    // locked together throughout the drag.
                    let rotation = start * incremental;
                    set_editing_dff_freeform_pivot(
                        app,
                        DffFreeformPivot {
                            position: drag.start_pos,
                            rotation: matrix_rotation_degrees(rotation),
                        },
                    );
                }
                GizmoTarget::Dff2dEffect => {
                    let mut rotation = drag.start_rot;
                    match drag.axis {
                        GizmoAxis::X => rotation.x += degrees,
                        GizmoAxis::Y => rotation.y += degrees,
                        GizmoAxis::Z => rotation.z += degrees,
                    }
                    if let Err(err) = set_selected_editing_dff_2dfx_rotation(app, rotation) {
                        app.status_message = err;
                    }
                }
            }
        }
        TransformMode::Scale => {
            if drag.target != GizmoTarget::DffVertex {
                return;
            }
            let length = gizmo_visual_length(app, to_mq(drag.start_pos)).max(0.001);
            let factor = if let Some(plane) = drag.scale_plane {
                let (first, second) = gizmo_plane_axes(plane);
                let direction =
                    selected_axis_vector(app, first) + selected_axis_vector(app, second);
                let screen_direction = world_to_screen(
                    app,
                    viewport,
                    to_mq(drag.start_pos) + direction * length * 0.28,
                )
                .and_then(|end| {
                    world_to_screen(app, viewport, to_mq(drag.start_pos))
                        .map(|start| (end - start).normalize_or_zero())
                })
                .unwrap_or(Vec2::new(1.0, -1.0).normalize());
                ((mouse - drag.start_mouse).dot(screen_direction) / 90.0)
                    .exp()
                    .clamp(0.001, 1000.0)
            } else {
                (amount / length).exp().clamp(0.001, 1000.0)
            };
            let mut factors = Vec3::ONE;
            if let Some(plane) = drag.scale_plane {
                let (first, second) = gizmo_plane_axes(plane);
                for axis in [first, second] {
                    match axis {
                        GizmoAxis::X => factors.x = factor,
                        GizmoAxis::Y => factors.y = factor,
                        GizmoAxis::Z => factors.z = factor,
                    }
                }
            } else {
                match drag.axis {
                    GizmoAxis::X => factors.x = factor,
                    GizmoAxis::Y => factors.y = factor,
                    GizmoAxis::Z => factors.z = factor,
                }
            }
            let start_vertices = drag.dff_start_vertices.clone();
            let pivot = drag.start_pos;
            if let Err(err) =
                scale_selected_editing_dff_vertices_from(app, &start_vertices, pivot, factors)
            {
                app.status_message = err;
            }
        }
    }
}

fn begin_dff_scale_input(app: &mut AppState) -> bool {
    if app.active_tab != AppTab::Editing {
        return false;
    }
    let start_vertices = selected_editing_dff_vertices(app);
    let Some(pivot) = selected_editing_dff_vertex_position(app).map(from_mq) else {
        app.status_message = "Select DFF vertices, edges, or faces before scaling".to_string();
        return true;
    };
    if start_vertices.is_empty() {
        app.status_message = "Select DFF vertices, edges, or faces before scaling".to_string();
        return true;
    }
    drain_text_input();
    app.transform_mode = TransformMode::Scale;
    app.dff_scale_input = Some(DffScaleInput {
        axis: None,
        numeric_input: String::new(),
        pivot,
        start_vertices,
        before: editing_history_snapshot(app),
    });
    app.status_message =
        "Scale: type a factor, or X/Y/Z then a factor; Enter confirms, Esc cancels".to_string();
    true
}

fn update_dff_scale_input(app: &mut AppState) -> bool {
    let Some(mut input) = app.dff_scale_input.take() else {
        return false;
    };
    if is_key_pressed(KeyCode::Escape) {
        let _ = scale_selected_editing_dff_vertices_from(
            app,
            &input.start_vertices,
            input.pivot,
            Vec3::ONE,
        );
        app.status_message = "Cancelled DFF scaling".to_string();
        return true;
    }
    for (key, axis) in [
        (KeyCode::X, GizmoAxis::X),
        (KeyCode::Y, GizmoAxis::Y),
        (KeyCode::Z, GizmoAxis::Z),
    ] {
        if is_key_pressed(key) {
            input.axis = Some(axis);
        }
    }
    while let Some(ch) = get_char_pressed() {
        let decimal = ch == '.' && !input.numeric_input.contains('.');
        let sign = (ch == '-' || ch == '+') && input.numeric_input.is_empty();
        if ch.is_ascii_digit() || decimal || sign {
            input.numeric_input.push(ch);
        }
    }
    if is_key_pressed(KeyCode::Backspace) {
        input.numeric_input.pop();
    }
    let factor = input
        .numeric_input
        .parse::<f32>()
        .ok()
        .filter(|value| value.is_finite());
    if let Some(factor) = factor {
        let mut factors = Vec3::splat(factor);
        if let Some(axis) = input.axis {
            factors = Vec3::ONE;
            match axis {
                GizmoAxis::X => factors.x = factor,
                GizmoAxis::Y => factors.y = factor,
                GizmoAxis::Z => factors.z = factor,
            }
        }
        if let Err(err) = scale_selected_editing_dff_vertices_from(
            app,
            &input.start_vertices,
            input.pivot,
            factors,
        ) {
            app.status_message = err;
        } else {
            let axis = input.axis.map_or("XYZ", |axis| match axis {
                GizmoAxis::X => "X",
                GizmoAxis::Y => "Y",
                GizmoAxis::Z => "Z",
            });
            app.status_message = format!("Scale {axis}: {factor}; Enter confirms, Esc cancels");
        }
    }
    if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
        if factor.is_some() {
            commit_editing_history(app, "Scale DFF Mesh Selection", input.before);
        } else {
            let _ = scale_selected_editing_dff_vertices_from(
                app,
                &input.start_vertices,
                input.pivot,
                Vec3::ONE,
            );
            app.status_message = "Scale cancelled: enter a valid number first".to_string();
        }
        return true;
    }
    app.dff_scale_input = Some(input);
    true
}

pub(crate) fn update_editor_input(app: &mut AppState, viewport: Rect) {
    poll_dff_picker(app);
    update_race_minimap(app);
    let mouse: Vec2 = mouse_position().into();
    // Asset drags must retain pointer ownership after leaving the browser;
    // otherwise viewport handlers can consume the release before placement.
    if app.asset_browser.drag.is_some() && update_asset_browser_drag(app, mouse) {
        return;
    }
    // Context menus float above every panel. Handle them before the asset
    // browser so a menu opened over a card receives its own click, and any
    // click away dismisses it immediately.
    if app.context_menu.is_some() {
        set_ui_interaction_suppressed(true);
        if is_key_pressed(KeyCode::Escape) {
            app.context_menu = None;
            return;
        }
        if is_mouse_button_pressed(MouseButton::Left) {
            if let Some(action) = context_action_at(app, mouse) {
                run_context_action(app, action);
            }
            app.context_menu = None;
            return;
        }
        if is_mouse_button_pressed(MouseButton::Right) {
            app.context_menu = None;
            return;
        }
        return;
    }
    if update_classify_dialog_input(app, mouse) {
        return;
    }
    if update_oversized_chunk_dialog_input(app, mouse) {
        return;
    }
    if update_blender_import_setup_input(app, mouse) {
        return;
    }
    if update_blender_import_dialog_input(app, mouse) {
        return;
    }
    if app.camera.looking {
        // Freecam owns pointer input until the right button is released. Do
        // not let the cursor's virtual position hover or activate editor UI.
        // The wheel still belongs to the active viewport so speed can be
        // tuned without interrupting camera movement.
        let wheel = safe_mouse_wheel().1;
        if wheel.abs() > f32::EPSILON {
            if app.active_tab == AppTab::Editing && handle_dff_uv_editor_wheel(app, mouse, wheel) {
                return;
            }
            let active_tab = app.active_tab;
            let speed =
                camera_speed_after_wheel(camera_speed_for_tab(app, active_tab), wheel, active_tab);
            set_camera_speed(app, speed);
        }
        app.hovered = None;
        app.hovered_gizmo = None;
        app.cull_hovered_face = None;
        app.hovered_col_face = None;
        app.hovered_col_vertex = None;
        app.col_box_hovered_face = None;
        app.hovered_water = None;
        app.hovered_water_edge = None;
        set_ui_interaction_suppressed(true);
        return;
    }
    set_ui_interaction_suppressed(false);
    if app.scrollbar_pointer_captured {
        if !is_mouse_button_down(MouseButton::Left) {
            app.scrollbar_pointer_captured = false;
            app.editing.scrollbar_drag = None;
            app.vehicle_browser.list_scroll_drag = false;
            app.validation_list_scroll_drag = None;
            set_scrollbar_hover_suppressed(false);
            return;
        }
        match app.active_tab {
            AppTab::Editing => {
                let _ = handle_editing_click(app, mouse);
            }
            AppTab::Vehicles => {
                let _ = update_vehicle_browser(app, mouse);
            }
            AppTab::Validation => {
                let _ = handle_validation_scrollbar_drag(app, mouse);
            }
            _ => {}
        }
        return;
    }
    if update_save_log_input(app, mouse) {
        return;
    }
    if update_confirm_dialog_input(app, mouse) {
        return;
    }
    if update_dff_merge_choice_dialog_input(app, mouse) {
        return;
    }
    if update_dff_optimize_dialog_input(app, mouse) {
        return;
    }
    if update_dff_txd_pair_dialog_input(app, mouse) {
        return;
    }
    if update_dff_texture_view_dialog_input(app, mouse) {
        return;
    }
    if update_dff_texture_duplicate_dialog_input(app, mouse) {
        return;
    }
    if update_lod_batch_dialog_input(app, mouse) {
        return;
    }
    if update_missing_col_dialog_input(app, mouse) {
        return;
    }
    if update_import_asset_dialog_input(app, mouse) {
        return;
    }
    if update_load_dialog_input(app, mouse) {
        return;
    }
    if update_preferences_dialog_input(app, mouse) {
        return;
    }
    if update_save_as_dialog_input(app, mouse) {
        return;
    }
    if update_dff_replace_choice_dialog_input(app, mouse) {
        return;
    }
    if update_element_replace_with_dialog_input(app, viewport, mouse) {
        return;
    }
    if update_element_id_rename_dialog_input(app, mouse) {
        return;
    }
    if update_missing_texture_dialog_input(app, mouse) {
        return;
    }
    if update_texture_archive_dialog_input(app, mouse) {
        return;
    }
    if update_dff_prelight_import_dialog_input(app, mouse) {
        return;
    }
    // Modal material pickers must receive text and wheel input before the
    // generic editor path drains characters or turns the wheel into camera
    // speed changes.
    if handle_open_editing_material_picker_input(app, mouse) {
        return;
    }
    if handle_viewport_render_mode_click(app, viewport, mouse) {
        return;
    }
    if handle_race_2d(app, mouse) {
        return;
    }
    if update_light_temperature_slider(app, mouse) {
        return;
    }
    if handle_open_light_dropdown_click(app, mouse) {
        return;
    }
    if is_mouse_button_pressed(MouseButton::Left) {
        if let Some(action) = clicked_inspector_copy_action(app, mouse) {
            if app.inspector_edit.is_some() {
                apply_inspector_edit(app);
            }
            copy_inspector_value(app, action);
            return;
        }
    }
    if update_vehicle_browser(app, mouse) {
        return;
    }
    if app.active_tab != AppTab::Editing
        && app.active_tab != AppTab::Vehicles
        && update_asset_browser(app, mouse)
    {
        return;
    }
    if app.active_tab != AppTab::Editing && update_group_rename_input(app) {
        return;
    }
    if app.active_tab == AppTab::Race && update_race_name_input(app) {
        return;
    }
    if app.active_tab != AppTab::Editing
        && app.active_tab != AppTab::Race
        && left_sidebar_visible()
        && update_outliner_type_filter_input(app, mouse)
    {
        return;
    }
    if app.active_tab != AppTab::Editing
        && app.active_tab != AppTab::Race
        && left_sidebar_visible()
        && update_outliner_search_input(app, mouse)
    {
        return;
    }
    if app.active_tab == AppTab::Lights
        && app.inspector_edit.is_some()
        && is_mouse_button_pressed(MouseButton::Left)
        && (inspector_field_rect(app, InspectorField::LightKind).contains(mouse)
            || inspector_field_rect(app, InspectorField::LightProfile).contains(mouse))
    {
        apply_inspector_edit(app);
        let kind = inspector_field_rect(app, InspectorField::LightKind).contains(mouse);
        app.light_kind_dropdown_open = kind;
        app.light_profile_dropdown_open = !kind;
        return;
    }
    if handle_col_material_dropdown_click(app, mouse) {
        return;
    }
    if handle_native_model_dropdown_input(app, mouse) {
        return;
    }
    if handle_txd_dropdown_input(app, mouse) {
        return;
    }
    if app.active_tab == AppTab::Preview
        && app.properties_tab == PropertiesTab::Element
        && handle_physics_root_dropdown_click(app, mouse)
    {
        return;
    }
    if update_dff_scale_input(app) {
        return;
    }
    if update_inspector_text_input(app, mouse) {
        return;
    }
    if app.active_tab == AppTab::Editing && update_dff_uv_anim_picker_text_input(app) {
        return;
    }
    if app.active_tab == AppTab::Editing {
        capture_dff_uv_numeric_transform_input(app);
    }
    drain_text_input();
    if app.water_edge_drag.is_some() {
        update_water_edge_drag(app, viewport, mouse);
        if is_mouse_button_released(MouseButton::Left) {
            finish_water_edge_drag(app);
        }
        return;
    }
    if app.cull_face_drag.is_some() {
        let transform_viewport = transform_interaction_viewport(app, viewport);
        apply_cull_face_drag(app, transform_viewport, mouse);
        if is_mouse_button_released(MouseButton::Left)
            && let Some(drag) = app.cull_face_drag.take()
        {
            commit_cull_history(app, "Resize Cull Zone Face", drag.before);
            app.status_message = "Resized water cull zone".to_string();
        }
        return;
    }
    if app.gizmo_drag.is_some() {
        let transform_viewport = transform_interaction_viewport(app, viewport);
        apply_gizmo_drag(app, transform_viewport, mouse);
        if is_mouse_button_released(MouseButton::Left) {
            let Some(drag) = app.gizmo_drag.take() else {
                return;
            };
            match drag.target {
                GizmoTarget::Element => {
                    if let ScopedHistorySnapshot::PlacementTransforms(before) = &drag.before {
                        rebuild_render_cells_for_placement_transforms(app, before);
                    } else {
                        // Alt-drag duplicates placements and is therefore a
                        // structural edit rather than an in-place transform.
                        rebuild_render_cells(app);
                    }
                    commit_scoped_history(app, drag.label, drag.before);
                }
                GizmoTarget::CullZone => {
                    commit_scoped_history(app, drag.label.clone(), drag.before);
                    app.status_message = drag.label;
                }
                GizmoTarget::Light => {
                    mark_lights_changed(app);
                    commit_scoped_history(app, drag.label, drag.before);
                }
                GizmoTarget::CollisionVertex => {
                    if app.active_tab == AppTab::Collisions {
                        let staged = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            stage_selected_collision_tab_col(app)
                        }));
                        match staged {
                            Ok(Ok(())) => commit_scoped_history(app, drag.label, drag.before),
                            Ok(Err(err)) => {
                                app.status_message =
                                    format!("Moved COL vertex; staging failed: {err}");
                                commit_scoped_history(app, drag.label, drag.before);
                            }
                            Err(_) => {
                                app.status_message =
                                    "Moved COL vertex; staging crashed and was skipped".to_string();
                                commit_scoped_history(app, drag.label, drag.before);
                            }
                        }
                    } else {
                        commit_scoped_history(app, drag.label, drag.before);
                        app.status_message = "Moved COL vertex".to_string();
                    }
                }
                GizmoTarget::CollisionPrimitive
                | GizmoTarget::DffVertex
                | GizmoTarget::DffBooleanBox
                | GizmoTarget::Dff2dEffect => {
                    commit_scoped_history(app, drag.label.clone(), drag.before);
                    app.status_message = drag.label;
                }
                GizmoTarget::DffPivot => {
                    app.status_message = drag.label;
                }
                GizmoTarget::RacePoint => {
                    commit_scoped_history(app, drag.label.clone(), drag.before);
                    app.status_message = drag.label;
                }
            }
        }
        return;
    }
    if app.col_box_face_drag.is_some() {
        let transform_viewport = transform_interaction_viewport(app, viewport);
        apply_col_box_face_drag(app, transform_viewport, mouse);
        if is_mouse_button_released(MouseButton::Left) {
            if let Some(drag) = app.col_box_face_drag.take() {
                commit_editing_history(app, "Resize COL Box Face".to_string(), drag.before);
                app.status_message = "Resized COL box face".to_string();
            }
        }
        return;
    }
    let (_, wheel) = safe_mouse_wheel();
    // The UV workspace owns wheel input over its canvas. Route this before the
    // generic editing viewport handler so scrolling zooms UVs without also
    // changing the 3D camera speed underneath the editor.
    if app.active_tab == AppTab::Editing && handle_dff_uv_editor_wheel(app, mouse, wheel) {
        return;
    }
    if app.active_tab == AppTab::Editing && update_editing_txd_preview_input(app, mouse, wheel) {
        return;
    }
    if handle_validation_collision_material_dropdown_input(app, mouse, wheel) {
        return;
    }
    if handle_validation_scroll(app, mouse, wheel) {
        return;
    }
    if app.inspector_scroll_drag {
        if is_mouse_button_down(MouseButton::Left) {
            app.properties_scroll = inspector_scroll_from_pointer(app, mouse);
            return;
        }
        app.inspector_scroll_drag = false;
    }
    if is_mouse_button_down(MouseButton::Left)
        && let Some(scroll) = inspector_scroll_from_mouse(app, mouse)
    {
        app.properties_scroll = scroll;
        app.inspector_scroll_drag = true;
        return;
    }
    if is_mouse_button_down(MouseButton::Left) && app.active_tab == AppTab::Water {
        let list = water_list_rect(app);
        let visible = water_visible_rows();
        let max_scroll = app.water_planes.len().saturating_sub(visible) as f32;
        let track = Rect::new(list.x + list.w - 10.0, list.y, 10.0, list.h);
        if app.water_list_scroll_drag || (max_scroll > 0.0 && track.contains(mouse)) {
            let thumb_h =
                (list.h * visible as f32 / app.water_planes.len() as f32).clamp(28.0, list.h);
            let travel = (list.h - thumb_h).max(1.0);
            app.water_scroll =
                ((mouse.y - list.y - thumb_h * 0.5).clamp(0.0, travel) / travel) * max_scroll;
            app.water_list_scroll_drag = true;
            update_water_hover(app, mouse);
            return;
        }
    }
    if !is_mouse_button_down(MouseButton::Left) {
        app.water_list_scroll_drag = false;
    }
    if is_mouse_button_down(MouseButton::Left) && app.active_tab == AppTab::Lights {
        let list = light_list_rect(app);
        let max_scroll = app.lights.len().saturating_sub(LIGHT_LIST_VISIBLE_ROWS) as f32;
        let track = Rect::new(list.x + list.w - 10.0, list.y, 10.0, list.h);
        if app.light_list_scroll_drag || (max_scroll > 0.0 && track.contains(mouse)) {
            let thumb_h = (list.h * LIGHT_LIST_VISIBLE_ROWS as f32 / app.lights.len() as f32)
                .clamp(24.0, list.h);
            let travel = (list.h - thumb_h).max(1.0);
            app.light_list_scroll =
                ((mouse.y - list.y - thumb_h * 0.5).clamp(0.0, travel) / travel) * max_scroll;
            app.light_list_scroll_drag = true;
            return;
        }
    }
    if !is_mouse_button_down(MouseButton::Left) {
        app.light_list_scroll_drag = false;
    }
    if wheel.abs() > 0.0 && app.col_material_dropdown_open {
        let visible = COL_MATERIAL_DROPDOWN_VISIBLE.min(GTA_SA_COL_MATERIALS.len());
        let first = col_material_option_rect(app, 0);
        let last = col_material_option_rect(app, visible.saturating_sub(1));
        let popup = Rect::new(
            first.x - 4.0,
            first.y - 4.0,
            first.w + 8.0,
            last.y + last.h - first.y + 8.0,
        );
        if popup.contains(mouse) {
            let max_scroll = GTA_SA_COL_MATERIALS.len().saturating_sub(visible) as f32;
            app.col_material_dropdown_scroll =
                (app.col_material_dropdown_scroll - wheel * 3.0).clamp(0.0, max_scroll);
            return;
        }
    }
    if wheel.abs() > 0.0
        && app.active_tab == AppTab::Editing
        && matches!(
            app.editing.asset.as_ref(),
            Some(EditingAsset::Dff(dff)) if dff.uv_anim_picker_open
        )
    {
        let list = editing_dff_uv_anim_picker_list_rect();
        if list.contains(mouse) {
            let row_h = 26.0;
            let visible = (list.h / row_h).floor().max(1.0) as usize;
            let max_scroll = match app.editing.asset.as_ref() {
                Some(EditingAsset::Dff(dff)) => editing_dff_filtered_uv_anim_options(dff)
                    .len()
                    .saturating_sub(visible) as f32,
                _ => 0.0,
            };
            if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
                dff.uv_anim_picker_scroll =
                    (dff.uv_anim_picker_scroll - wheel * 2.0).clamp(0.0, max_scroll);
            }
        }
        return;
    }
    if wheel.abs() > 0.0
        && app.active_tab == AppTab::Editing
        && matches!(
            app.editing.asset.as_ref(),
            Some(EditingAsset::Dff(dff)) if dff.dff_2dfx_type_picker_open
        )
    {
        let list = editing_dff_2dfx_type_picker_list_rect();
        if list.contains(mouse) {
            let row_h = 28.0;
            let visible = (list.h / row_h).floor().max(1.0) as usize;
            let max_scroll = match app.editing.asset.as_ref() {
                Some(EditingAsset::Dff(dff)) => {
                    dff_2dfx_filtered_types(&dff.dff_2dfx_type_picker_search)
                        .len()
                        .saturating_sub(visible) as f32
                }
                _ => 0.0,
            };
            if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
                dff.dff_2dfx_type_picker_scroll =
                    (dff.dff_2dfx_type_picker_scroll - wheel * 2.0).clamp(0.0, max_scroll);
            }
        }
        return;
    }
    if wheel.abs() > 0.0
        && app.active_tab == AppTab::Editing
        && matches!(
            app.editing.asset.as_ref(),
            Some(EditingAsset::Dff(dff)) if dff.dff_2dfx_payload_editor_open
        )
    {
        if matches!(
            app.editing.asset.as_ref(),
            Some(EditingAsset::Dff(dff)) if dff_2dfx_active_payload_is_particle_name(dff)
        ) {
            let picker = editing_dff_2dfx_particle_picker_rect();
            if picker.contains(mouse) {
                let row_h = 26.0;
                let visible = (picker.h / row_h).floor().max(1.0) as usize;
                let max_scroll = match app.editing.asset.as_ref() {
                    Some(EditingAsset::Dff(dff)) => dff_2dfx_particle_name_options(app, dff)
                        .len()
                        .saturating_sub(visible)
                        as f32,
                    _ => 0.0,
                };
                if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
                    dff.dff_2dfx_particle_picker_scroll =
                        (dff.dff_2dfx_particle_picker_scroll - wheel * 2.0).clamp(0.0, max_scroll);
                }
                return;
            }
        }
        let list = editing_dff_2dfx_payload_text_rect();
        if list.contains(mouse) {
            let row_h = 34.0;
            let visible = (list.h / row_h).floor().max(1.0) as usize;
            let max_scroll = match app.editing.asset.as_ref() {
                Some(EditingAsset::Dff(dff)) => {
                    dff.dff_2dfx_payload_fields.len().saturating_sub(visible) as f32
                }
                _ => 0.0,
            };
            if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
                dff.dff_2dfx_payload_field_scroll =
                    (dff.dff_2dfx_payload_field_scroll - wheel * 2.0).clamp(0.0, max_scroll);
            }
        }
        return;
    }
    if wheel.abs() > 0.0
        && app.active_tab == AppTab::Editing
        && matches!(
            app.editing.asset.as_ref(),
            Some(EditingAsset::Dff(dff)) if dff.texture_picker_open
        )
    {
        let list = editing_dff_texture_picker_list_rect();
        let visible = (list.h / DFF_TEXTURE_PICKER_ROW_H).floor().max(1.0) as usize;
        let max_scroll = editing_dff_picker_texture_names(app)
            .len()
            .saturating_sub(visible) as f32;
        if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
            dff.texture_picker_scroll =
                (dff.texture_picker_scroll - wheel * 2.0).clamp(0.0, max_scroll);
        }
        return;
    }
    if wheel.abs() > 0.0 && app.active_tab == AppTab::Editing {
        let selected_emitter = selected_material_emitter(app);
        let lighting_entry_count = match app.editing.asset.as_ref() {
            Some(EditingAsset::Dff(dff)) => dff_face_emitter_entry_count(app, &dff.name),
            _ => 0,
        };
        if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_mut() {
            let layout = dff_panel_layout(dff, selected_emitter, lighting_entry_count);
            if layout
                .material_list
                .is_some_and(|list| list.contains(mouse))
                && app.editing.nested_scroll_focus == Some(EditingNestedScrollFocus::DffMaterials)
            {
                let max_scroll =
                    dff.raw
                        .material_textures
                        .len()
                        .saturating_sub(layout.material_visible.max(1)) as f32;
                dff.material_scroll = (dff.material_scroll - wheel * 2.0).clamp(0.0, max_scroll);
                return;
            }
            if layout.content.contains(mouse) {
                dff.panel_scroll =
                    (dff.panel_scroll - wheel * 24.0).clamp(0.0, dff_panel_max_scroll(&layout));
                return;
            }
        }
    }
    if wheel.abs() > 0.0 && app.active_tab == AppTab::Editing {
        if let Some(EditingAsset::Col(col)) = app.editing.asset.as_mut() {
            let layout = col_panel_layout(col);
            if layout
                .primitive_list
                .is_some_and(|list| list.contains(mouse))
                && app.editing.nested_scroll_focus == Some(EditingNestedScrollFocus::ColPrimitives)
            {
                scroll_editing_col_primitives(col, wheel);
                return;
            }
            if layout.face_list.is_some_and(|list| list.contains(mouse))
                && app.editing.nested_scroll_focus == Some(EditingNestedScrollFocus::ColFaces)
            {
                scroll_editing_col_faces(col, wheel);
                return;
            }
            if layout.content.contains(mouse) {
                col.panel_scroll =
                    (col.panel_scroll - wheel * 24.0).clamp(0.0, col_panel_max_scroll(&layout));
                return;
            }
        }
    }
    if wheel.abs() > 0.0 && app.active_tab == AppTab::Water && water_list_rect(app).contains(mouse)
    {
        scroll_water_list(app, wheel);
        update_water_hover(app, mouse);
        return;
    }
    if wheel.abs() > 0.0 && app.active_tab == AppTab::Cull && cull_list_rect(app).contains(mouse) {
        scroll_cull_list(app, wheel);
        return;
    }
    if wheel.abs() > 0.0 && app.active_tab == AppTab::Lights && light_list_rect(app).contains(mouse)
    {
        scroll_light_list(app, wheel);
        return;
    }
    if app.active_tab == AppTab::Race {
        ensure_race_loaded(app);
        // Wheel zooms the 2D radar editor when the cursor is over its view.
        if wheel.abs() > 0.0 && app.race.radar_2d && race_2d_view_rect().contains(mouse) {
            scroll_race_2d(app, wheel, mouse);
            return;
        }
        if wheel.abs() > 0.0 && mouse.x > screen_width() - RIGHT_PANEL_W {
            scroll_race_list(app, wheel);
            return;
        }
        if wheel.abs() > 0.0 && mouse.x < PANEL_W && mouse.y > TOP_H {
            scroll_race_tracks(app, wheel);
            return;
        }
    }
    if handle_missing_texture_review_scroll(app, mouse, wheel) {
        return;
    }
    // The TXD asset pane overlaps the generic right inspector. Route its wheel
    // input first or the inspector consumes it and the texture list never moves.
    if app.active_tab == AppTab::Editing && scroll_editing_txd_list(app, mouse, wheel) {
        return;
    }
    if wheel.abs() > 0.0 && right_panel_rect().contains(mouse) {
        app.properties_scroll = (app.properties_scroll - wheel * 36.0).max(0.0);
        clamp_properties_scroll(app);
        return;
    }
    if wheel.abs() > 0.0 && viewport.contains(mouse) {
        let speed = camera_speed_after_wheel(
            camera_speed_for_tab(app, app.active_tab),
            wheel,
            app.active_tab,
        );
        set_camera_speed(app, speed);
        return;
    }
    if app.active_tab != AppTab::Editing
        && wheel.abs() > 0.0
        && left_sidebar_visible()
        && mouse.x < PANEL_W
        && mouse.y > TOP_H
    {
        app.scroll = (app.scroll - wheel * 7.0).max(0.0);
        app.scroll_interaction_until = get_time() + 0.18;
    }
    if app.active_tab != AppTab::Editing
        && left_sidebar_visible()
        && update_outliner_scroll_from_mouse(app, mouse)
    {
        app.hovered = None;
        app.hovered_gizmo = None;
        return;
    }
    if app.active_tab != AppTab::Editing {
        let rows = outliner_rows();
        let max_scroll = app.outliner_filter.len().saturating_sub(rows) as f32;
        app.scroll = app.scroll.min(max_scroll);
    }

    app.hovered = None;
    app.hovered_gizmo = None;
    app.cull_hovered_face = None;
    app.col_box_hovered_face = None;
    if app.active_tab == AppTab::Cull && viewport.contains(mouse) {
        if let Some(pick) = pick_selected_cull_face(app, viewport, mouse) {
            app.cull_hovered_face = Some((pick.axis, pick.side_is_max));
        }
    }
    if app.active_tab == AppTab::Editing && matches!(app.editing.asset, Some(EditingAsset::Col(_)))
    {
        let face_viewport = transform_interaction_viewport(app, viewport);
        if face_viewport.contains(mouse) {
            if let Some(pick) = editing_pick_col_box_face(app, face_viewport, mouse) {
                app.col_box_hovered_face = Some((pick.primitive, pick.axis, pick.side_is_max));
            }
        }
    }
    update_water_hover(app, mouse);
    update_water_edge_hover(app, viewport, mouse);
    let suppress_hover_pick = get_time() < app.scroll_interaction_until;
    let ctrl_down = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
    let shift_down = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
    let alt_down = is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt);
    if alt_down && !ctrl_down && is_key_pressed(KeyCode::S) && begin_dff_scale_input(app) {
        return;
    }
    if app.active_tab == AppTab::Collisions && app.collision_edit_mode && viewport.contains(mouse) {
        if let Some((face, vertex)) =
            nearest_selected_collision_tab_screen_vertex(app, viewport, mouse)
        {
            app.hovered_col_face = Some(face);
            app.hovered_col_vertex = Some(vertex);
        } else {
            app.hovered_col_face = None;
            app.hovered_col_vertex = None;
        }
    } else if app.active_tab == AppTab::Collisions {
        app.hovered_col_face = None;
        app.hovered_col_vertex = None;
    }
    if app.box_select_drag.is_some() {
        if ctrl_down && is_mouse_button_down(MouseButton::Left) {
            if let Some(drag) = app.box_select_drag.as_mut() {
                drag.current = mouse;
            }
            return;
        }
        let Some(drag) = app.box_select_drag.take() else {
            return;
        };
        if app.active_tab == AppTab::Editing {
            box_select_editing_vertices(app, drag.start, drag.current);
            return;
        }
        let selected = box_select_elements(app, viewport, drag.start, drag.current);
        if selected.is_empty() {
            app.status_message = "Box selected 0 assets".to_string();
            return;
        }
        match app.box_select_mode {
            BoxSelectMode::Add => {
                for idx in selected {
                    if app.selected_elements.insert(idx) {
                        app.selected_element_order.push(idx);
                    }
                }
            }
            BoxSelectMode::Subtract => {
                for idx in selected {
                    app.selected_elements.remove(&idx);
                    app.selected_element_order.retain(|ordered| *ordered != idx);
                }
            }
            BoxSelectMode::Toggle => {
                for idx in selected {
                    if !app.selected_elements.insert(idx) {
                        app.selected_elements.remove(&idx);
                        app.selected_element_order.retain(|ordered| *ordered != idx);
                    } else {
                        app.selected_element_order.push(idx);
                    }
                }
            }
        }
        app.selected = app
            .selected_element_order
            .iter()
            .rev()
            .copied()
            .find(|idx| app.selected_elements.contains(idx))
            .unwrap_or(NO_SELECTION);
        app.selected_group = None;
        app.selected_col_face = None;
        app.status_message = format!(
            "Box {} {} asset(s) within {:.0} units",
            app.box_select_mode.verb(),
            app.selected_elements.len(),
            app.box_select_distance
        );
        return;
    }
    if handle_editing_mesh_selection_shortcuts(app, ctrl_down, shift_down, alt_down) {
        return;
    }
    if !ctrl_down && !alt_down && is_key_pressed(KeyCode::C) {
        toggle_camera_mode(app);
        return;
    }
    if !ctrl_down && !alt_down && is_key_pressed(KeyCode::Period) {
        focus_camera_on_selection(app);
        return;
    }
    if ctrl_down && is_key_pressed(KeyCode::Z) {
        if shift_down {
            redo(app);
        } else if cancel_texture_match_selection_for_undo(app) {
            return;
        } else {
            undo(app);
        }
        return;
    }
    if ctrl_down && is_key_pressed(KeyCode::Y) {
        redo(app);
        return;
    }
    let uv_editor_open = matches!(
        app.editing.asset.as_ref(),
        Some(EditingAsset::Dff(dff)) if dff.uv_editor.open
    );
    if !ctrl_down
        && app.active_tab == AppTab::Editing
        && !uv_editor_open
        && is_key_pressed(KeyCode::M)
    {
        open_dff_merge_choice_dialog(app);
        return;
    }
    if !ctrl_down && app.active_tab == AppTab::Editing && is_key_pressed(KeyCode::F) {
        let before = editing_history_snapshot(app);
        if editing_make_face_from_selected_vertices(app) {
            commit_editing_history(app, "Make Face", before);
            return;
        }
    }
    if ctrl_down && app.active_tab == AppTab::Editing && is_key_pressed(KeyCode::E) {
        let before = editing_history_snapshot(app);
        if editing_extrude_selected(app) {
            commit_editing_history(app, "Extrude", before);
            return;
        }
    }
    if ctrl_down && is_key_pressed(KeyCode::S) {
        let alt_down = is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt);
        if alt_down {
            save_wip_scene(app);
        } else if shift_down {
            open_save_as_dialog(app);
        } else {
            save_scene(app);
        }
        return;
    }
    if ctrl_down && is_key_pressed(KeyCode::G) {
        if shift_down {
            clear_selected_group(app);
        } else {
            assign_selected_to_group(app);
        }
        return;
    }
    if is_key_pressed(KeyCode::H)
        && app.active_tab != AppTab::Lights
        && app.active_tab != AppTab::Water
    {
        if ctrl_down {
            unhide_all(app);
        } else if alt_down {
            hide_everything_but_selected(app);
        } else if shift_down {
            unhide_selected(app);
        } else {
            hide_selected(app);
        }
        return;
    }
    if ctrl_down && is_key_pressed(KeyCode::LeftBracket) {
        app.box_select_distance = (app.box_select_distance * 0.5).max(250.0);
        app.status_message = format!("Box select distance {:.0}", app.box_select_distance);
        return;
    }
    if ctrl_down && is_key_pressed(KeyCode::RightBracket) {
        app.box_select_distance = (app.box_select_distance * 2.0).min(64000.0);
        app.status_message = format!("Box select distance {:.0}", app.box_select_distance);
        return;
    }
    if is_key_pressed(KeyCode::F2) {
        if let Some(group) = app.selected_group.clone() {
            start_group_rename(app, &group);
        }
        return;
    }
    // The overflow navigation menu is drawn over the viewport. Give it first
    // refusal while open so viewport-owned tools (notably vertex painting in
    // the Bake tab) cannot consume clicks intended for its Water/Race rows.
    if app.navigation_menu_open && handle_tab_click(app, mouse) {
        return;
    }
    if handle_lights_click(app, mouse) {
        return;
    }
    if handle_bake_click(app, mouse) {
        return;
    }
    if handle_timecyc_click(app, mouse) {
        return;
    }
    if handle_validation_click(app, mouse) {
        return;
    }
    if handle_lod_audit_click(app, mouse) {
        return;
    }
    if handle_missing_texture_review_click(app, mouse) {
        return;
    }
    if handle_editing_click(app, mouse) {
        return;
    }
    if handle_simulate_click(app, viewport, mouse) {
        return;
    }
    if handle_water_click(app, mouse) {
        return;
    }
    if handle_cull_click(app, mouse) {
        return;
    }
    if handle_race_overlay_toggle(app, mouse) {
        return;
    }
    if handle_race_minimap_click(app, mouse) {
        return;
    }
    if handle_race_click(app, mouse) {
        return;
    }
    if update_vertex_lighting_tool(app, viewport, mouse) {
        return;
    }
    if app.active_tab == AppTab::Bake
        && viewport.contains(mouse)
        && !alt_down
        && is_mouse_button_pressed(MouseButton::Left)
    {
        return;
    }
    if handle_inspector_click(app, mouse) {
        return;
    }
    if handle_tab_click(app, mouse) {
        return;
    }
    if app.active_tab == AppTab::Simulate && app.sim.playing {
        return;
    }
    if handle_toolbar_click(app, mouse) {
        return;
    }
    if app.active_tab == AppTab::Water {
        if start_water_edge_drag(app, viewport, mouse) {
            return;
        }
        if handle_water_viewport_click(app, viewport, mouse) {
            return;
        }
        if viewport.contains(mouse) && is_mouse_button_pressed(MouseButton::Right) {
            return;
        }
    }
    if app.active_tab == AppTab::Cull {
        if start_cull_face_drag(app, viewport, mouse) {
            return;
        }
        if handle_cull_viewport_click(app, viewport, mouse) {
            return;
        }
    }
    if app.active_tab == AppTab::Race {
        if handle_race_viewport_click(app, viewport, mouse) {
            return;
        }
    }
    if is_key_pressed(KeyCode::Key1) {
        app.transform_mode = TransformMode::Select;
    }
    if is_key_pressed(KeyCode::Key2) {
        app.transform_mode = TransformMode::Move;
    }
    if is_key_pressed(KeyCode::Key3) {
        app.transform_mode = TransformMode::Rotate;
    }
    if is_key_pressed(KeyCode::L) && !ctrl_down {
        app.options.lod_mode = app.options.lod_mode.next();
        eprintln!("LOD mode: {}", app.options.lod_mode.label());
        rebuild_render_cells(app);
    }
    if app.active_tab == AppTab::Lights
        && ctrl_down
        && viewport.contains(mouse)
        && is_mouse_button_pressed(MouseButton::Right)
    {
        if let Some((idx, placement)) = pick_scene_light(app, viewport, mouse) {
            app.selected_light = idx;
            if let Some(placement) = placement {
                app.selected = placement;
            }
            app.camera.looking = false;
            set_cursor_grab(false);
            show_mouse(true);
            app.context_menu = Some(ContextMenu {
                pos: mouse,
                target: ContextMenuTarget::Selection,
            });
            return;
        }
        let position = pick_scene_geometry_point(app, viewport, mouse)
            .map(|(_, position)| position)
            .or_else(|| {
                let (origin, direction) = viewport_ray(app, viewport, mouse)?;
                if direction.z.abs() < 0.0001 {
                    return None;
                }
                let t = -origin.z / direction.z;
                (t >= 0.0).then_some(origin + direction * t)
            })
            .unwrap_or_else(|| {
                let (forward, _) = camera_vectors(&app.camera);
                app.camera.pos + forward * 500.0
            });
        app.camera.looking = false;
        set_cursor_grab(false);
        show_mouse(true);
        app.context_menu = Some(ContextMenu {
            pos: mouse,
            target: ContextMenuTarget::Scene {
                position: from_mq(position),
            },
        });
        return;
    }
    if ctrl_down && viewport.contains(mouse) && is_mouse_button_pressed(MouseButton::Right) {
        let picked = if app.active_tab == AppTab::Collisions {
            pick_collision_face(app, viewport, mouse).map(|face| face.placement)
        } else {
            pick_scene_element(app, viewport, mouse)
        };
        if let Some(idx) = picked {
            if !app
                .element_states
                .get(idx)
                .is_some_and(|state| state.deleted)
            {
                select_element_with_mode(app, idx, shift_down);
            }
        }
        app.camera.looking = false;
        set_cursor_grab(false);
        show_mouse(true);
        app.context_menu = Some(ContextMenu {
            pos: mouse,
            target: ContextMenuTarget::Selection,
        });
        return;
    }
    let transform_viewport = transform_interaction_viewport(app, viewport);
    if !suppress_hover_pick
        && transform_viewport.contains(mouse)
        && app.transform_mode != TransformMode::Select
        && app.active_tab != AppTab::Bake
    {
        app.hovered_gizmo_plane = gizmo_scale_plane_at(app, transform_viewport, mouse);
        app.hovered_gizmo = if app.hovered_gizmo_plane.is_none() {
            gizmo_axis_at(app, transform_viewport, mouse)
        } else {
            None
        };
    }

    if !suppress_hover_pick
        && transform_viewport.contains(mouse)
        && app.transform_mode != TransformMode::Select
        && app.active_tab != AppTab::Bake
        && is_mouse_button_pressed(MouseButton::Left)
    {
        let hovered_plane = app.hovered_gizmo_plane;
        if let Some(axis) = app
            .hovered_gizmo
            .or_else(|| hovered_plane.map(|plane| gizmo_plane_axes(plane).0))
        {
            let alt_down = is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt);
            let before = if app.active_tab == AppTab::Cull {
                ScopedHistorySnapshot::Cull(cull_history_snapshot(app))
            } else if alt_down && app.active_tab != AppTab::Lights {
                match app.active_tab {
                    AppTab::Editing => {
                        ScopedHistorySnapshot::WorldEditing(world_editing_history_snapshot(app))
                    }
                    AppTab::Race => {
                        ScopedHistorySnapshot::WorldRace(world_race_history_snapshot(app))
                    }
                    _ => ScopedHistorySnapshot::World(world_history_snapshot(app)),
                }
            } else {
                match app.active_tab {
                    AppTab::Lights => ScopedHistorySnapshot::Lights(light_history_snapshot(app)),
                    AppTab::Collisions => {
                        ScopedHistorySnapshot::Collision(collision_history_snapshot(app))
                    }
                    AppTab::Editing => {
                        ScopedHistorySnapshot::Editing(editing_history_snapshot(app))
                    }
                    AppTab::Race => ScopedHistorySnapshot::Race(race_history_snapshot(app)),
                    _ => ScopedHistorySnapshot::PlacementTransforms(
                        placement_transform_history_snapshot(app, selected_live_indices(app)),
                    ),
                }
            };
            let mut label = match app.transform_mode {
                TransformMode::Move => "Move",
                TransformMode::Rotate => "Rotate",
                TransformMode::Scale => "Scale",
                TransformMode::Select => "Transform",
            }
            .to_string();
            if app.active_tab == AppTab::Lights {
                if alt_down {
                    if let Some(source) = app.lights.get(app.selected_light).cloned() {
                        let mut copy = source;
                        copy.name = format!("{} Copy", copy.name);
                        app.lights.push(copy);
                        app.selected_light = app.lights.len() - 1;
                        label = format!("Alt {label} Light Duplicate");
                    }
                }
                if let Some(light) = app.lights.get(app.selected_light) {
                    let (start_pos, start_direction) = light_world_transform(
                        light,
                        light_reference_placement(app, light).map(|(_, placement)| placement),
                    );
                    let drag_label = if alt_down {
                        label.clone()
                    } else {
                        format!("{label} Light")
                    };
                    app.gizmo_drag = Some(GizmoDrag {
                        target: GizmoTarget::Light,
                        axis,
                        start_mouse: mouse,
                        start_pos: from_mq(start_pos),
                        start_rot: from_mq(start_direction),
                        element_start_positions: Vec::new(),
                        element_start_rots: Vec::new(),
                        scale_plane: None,
                        dff_start_vertices: Vec::new(),
                        before,
                        label: drag_label,
                    });
                    return;
                }
            }
            if app.active_tab == AppTab::Collisions
                && app.collision_edit_mode
                && app.selected_col_face.is_some()
            {
                if let Some(origin) = selected_collision_tab_vertex_position(app) {
                    app.gizmo_drag = Some(GizmoDrag {
                        target: GizmoTarget::CollisionVertex,
                        axis,
                        start_mouse: mouse,
                        start_pos: from_mq(origin),
                        start_rot: V3::default(),
                        element_start_positions: Vec::new(),
                        element_start_rots: Vec::new(),
                        scale_plane: None,
                        dff_start_vertices: Vec::new(),
                        before,
                        label: "Move COL Vertex".to_string(),
                    });
                    return;
                }
            }
            if app.active_tab == AppTab::Editing
                && matches!(app.editing.asset, Some(EditingAsset::Col(_)))
            {
                let primitive_selected = app
                    .editing
                    .asset
                    .as_ref()
                    .and_then(|asset| match asset {
                        EditingAsset::Col(col) => col.selected_primitive,
                        _ => None,
                    })
                    .is_some();
                if alt_down && !editing_duplicate_selected_mesh(app) {
                    return;
                }
                if let Some(origin) = selected_origin(app) {
                    let target = if primitive_selected {
                        GizmoTarget::CollisionPrimitive
                    } else {
                        GizmoTarget::CollisionVertex
                    };
                    let (primitive_kind, primitive_rotation) = app
                        .editing
                        .asset
                        .as_ref()
                        .and_then(|asset| match asset {
                            EditingAsset::Col(col) => {
                                let selected = col.selected_primitive?;
                                let rotation = match selected.kind {
                                    CollisionPrimitiveKind::Capsule => col
                                        .capsules
                                        .get(selected.index)
                                        .map(|capsule| {
                                            from_mq(
                                                (to_mq(capsule.end) - to_mq(capsule.start)) * 0.5,
                                            )
                                        })
                                        .unwrap_or_default(),
                                    CollisionPrimitiveKind::Cuboid => col
                                        .cuboids
                                        .get(selected.index)
                                        .map(|cuboid| cuboid.rotation)
                                        .unwrap_or_default(),
                                    _ => V3::default(),
                                };
                                Some((selected.kind, rotation))
                            }
                            _ => None,
                        })
                        .unwrap_or((CollisionPrimitiveKind::Sphere, V3::default()));
                    let label = if target == GizmoTarget::CollisionPrimitive && alt_down {
                        "Alt Duplicate Editing COL Primitive"
                    } else if target == GizmoTarget::CollisionPrimitive
                        && app.transform_mode == TransformMode::Rotate
                        && primitive_kind == CollisionPrimitiveKind::Capsule
                    {
                        "Rotate Editing COL Capsule"
                    } else if target == GizmoTarget::CollisionPrimitive
                        && app.transform_mode == TransformMode::Rotate
                        && matches!(
                            primitive_kind,
                            CollisionPrimitiveKind::Box | CollisionPrimitiveKind::Cuboid
                        )
                    {
                        "Rotate Editing COL Box"
                    } else if target == GizmoTarget::CollisionPrimitive {
                        "Move Editing COL Primitive"
                    } else if alt_down {
                        "Alt Duplicate COL Mesh Selection"
                    } else {
                        "Move Editing COL Vertex"
                    };
                    app.gizmo_drag = Some(GizmoDrag {
                        target,
                        axis,
                        start_mouse: mouse,
                        start_pos: from_mq(origin),
                        start_rot: primitive_rotation,
                        element_start_positions: Vec::new(),
                        element_start_rots: Vec::new(),
                        scale_plane: None,
                        dff_start_vertices: Vec::new(),
                        before,
                        label: label.to_string(),
                    });
                    return;
                }
            }
            if app.active_tab == AppTab::Editing {
                if let Some(EditingAsset::Dff(dff)) = app.editing.asset.as_ref() {
                    // Freeform pivot editing owns the transform gimbal until it
                    // is applied or cancelled, regardless of mesh selection.
                    if let Some(pivot) = dff.freeform_pivot {
                        if app.dff_geometry_job.is_some() {
                            return;
                        }
                        app.gizmo_drag = Some(GizmoDrag {
                            target: GizmoTarget::DffPivot,
                            axis,
                            start_mouse: mouse,
                            start_pos: pivot.position,
                            start_rot: pivot.rotation,
                            element_start_positions: Vec::new(),
                            element_start_rots: Vec::new(),
                            scale_plane: None,
                            dff_start_vertices: Vec::new(),
                            before,
                            label: if app.transform_mode == TransformMode::Rotate {
                                "Rotated freeform DFF pivot".to_string()
                            } else {
                                "Moved freeform DFF pivot".to_string()
                            },
                        });
                        return;
                    }
                    // An active cutter owns the DFF transform gizmo. Prioritise
                    // it over mesh and 2DFX selections so restored selection
                    // state cannot make the cutter appear immovable.
                    if dff.boolean_box.is_some() && app.transform_mode != TransformMode::Scale {
                        if let Some(origin) = selected_editing_dff_boolean_box_position(app) {
                            app.gizmo_drag = Some(GizmoDrag {
                                target: GizmoTarget::DffBooleanBox,
                                axis,
                                start_mouse: mouse,
                                start_pos: from_mq(origin),
                                start_rot: V3::default(),
                                element_start_positions: Vec::new(),
                                element_start_rots: Vec::new(),
                                scale_plane: None,
                                dff_start_vertices: Vec::new(),
                                before,
                                label: "Moved DFF Boolean Cutter".to_string(),
                            });
                            return;
                        }
                    }
                    if app.transform_mode != TransformMode::Scale
                        && let Some(origin) = selected_editing_dff_2dfx_position(app)
                    {
                        let rotation = selected_editing_dff_2dfx_rotation(app);
                        if app.transform_mode == TransformMode::Rotate && rotation.is_none() {
                            return;
                        }
                        let rotating = app.transform_mode == TransformMode::Rotate;
                        app.gizmo_drag = Some(GizmoDrag {
                            target: GizmoTarget::Dff2dEffect,
                            axis,
                            start_mouse: mouse,
                            start_pos: from_mq(origin),
                            start_rot: rotation.unwrap_or_default(),
                            element_start_positions: Vec::new(),
                            element_start_rots: Vec::new(),
                            scale_plane: None,
                            dff_start_vertices: Vec::new(),
                            before,
                            label: if rotating {
                                "Rotated DFF Road Sign".to_string()
                            } else {
                                "Moved DFF 2DFX".to_string()
                            },
                        });
                        return;
                    }
                    if selected_editing_dff_vertex_position(app).is_some() {
                        if alt_down && !editing_duplicate_selected_mesh(app) {
                            return;
                        }
                        let Some(origin) = selected_editing_dff_vertex_position(app) else {
                            return;
                        };
                        app.gizmo_drag = Some(GizmoDrag {
                            target: GizmoTarget::DffVertex,
                            axis,
                            start_mouse: mouse,
                            start_pos: from_mq(origin),
                            start_rot: V3::default(),
                            element_start_positions: Vec::new(),
                            element_start_rots: Vec::new(),
                            scale_plane: hovered_plane,
                            dff_start_vertices: selected_editing_dff_vertices(app),
                            before,
                            label: if app.transform_mode == TransformMode::Scale {
                                "Scaled DFF Mesh Selection".to_string()
                            } else if alt_down {
                                "Alt Duplicate DFF Mesh Selection".to_string()
                            } else {
                                "Moved DFF Vertex".to_string()
                            },
                        });
                        return;
                    }
                }
            }
            if app.active_tab == AppTab::Race {
                if let Some(origin) = selected_race_point_position(app) {
                    app.gizmo_drag = Some(GizmoDrag {
                        target: GizmoTarget::RacePoint,
                        axis,
                        start_mouse: mouse,
                        start_pos: from_mq(origin),
                        start_rot: V3::default(),
                        element_start_positions: Vec::new(),
                        element_start_rots: Vec::new(),
                        scale_plane: None,
                        dff_start_vertices: Vec::new(),
                        before,
                        label: "Move Race Point".to_string(),
                    });
                    return;
                }
            }
            if app.active_tab == AppTab::Cull {
                if let Some(zone) = selected_cull_zone(app) {
                    app.gizmo_drag = Some(GizmoDrag {
                        target: GizmoTarget::CullZone,
                        axis,
                        start_mouse: mouse,
                        start_pos: zone.center,
                        start_rot: V3::default(),
                        element_start_positions: Vec::new(),
                        element_start_rots: Vec::new(),
                        scale_plane: None,
                        dff_start_vertices: Vec::new(),
                        before,
                        label: "Move water cull zone".to_string(),
                    });
                    return;
                }
            }
            // Alt-duplicate applies only when this drag will target world elements.
            // Editing and race gizmos have their own targets; duplicating the
            // current world selection before returning from those branches created
            // unrelated, invisible placement copies.
            if alt_down {
                let indices = selected_live_indices(app);
                let mut new_selection = BTreeSet::new();
                for idx in indices {
                    let Some(source) = app.placements.get(idx) else {
                        continue;
                    };
                    let mut copy = source.clone();
                    sync_placement_attrs(&mut copy);
                    app.placements.push(copy);
                    app.element_states.push(ElementState::default());
                    app.outliner_labels.push(None);
                    let new_idx = app.placements.len() - 1;
                    new_selection.insert(new_idx);
                    app.selected = new_idx;
                }
                if !new_selection.is_empty() {
                    app.selected_elements = new_selection;
                    app.selected_element_order = app.selected_elements.iter().copied().collect();
                    rebuild_outliner_filter(app);
                    label = format!("Alt {label} Duplicate");
                }
            }
            if let Some(origin) = selection_origin(app) {
                let element_start_positions = selected_live_indices(app)
                    .into_iter()
                    .filter_map(|idx| app.placements.get(idx).map(|p| (idx, p.pos)))
                    .collect::<Vec<_>>();
                let element_start_rots = selected_live_indices(app)
                    .into_iter()
                    .filter_map(|idx| app.placements.get(idx).map(|p| (idx, p.rot)))
                    .collect::<Vec<_>>();
                app.gizmo_drag = Some(GizmoDrag {
                    target: GizmoTarget::Element,
                    axis,
                    start_mouse: mouse,
                    start_pos: from_mq(origin),
                    start_rot: V3::default(),
                    element_start_positions,
                    element_start_rots,
                    scale_plane: None,
                    dff_start_vertices: Vec::new(),
                    before,
                    label,
                });
                return;
            }
        }
    }

    let box_select_viewport = transform_interaction_viewport(app, viewport);
    if ctrl_down
        && (app.active_tab != AppTab::Lights || alt_down)
        && box_select_viewport.contains(mouse)
        && is_mouse_button_pressed(MouseButton::Left)
    {
        app.box_select_drag = Some(BoxSelectDrag {
            start: mouse,
            current: mouse,
        });
        app.camera.looking = false;
        set_cursor_grab(false);
        show_mouse(true);
        return;
    }

    if !suppress_hover_pick && app.active_tab != AppTab::Race && left_sidebar_visible() {
        if let Some(row) = outliner_row_at(mouse) {
            if let Some(entry) = app.outliner_filter.get(app.scroll as usize + row).cloned() {
                match entry {
                    OutlinerEntry::Group(group) => {
                        if is_mouse_button_pressed(MouseButton::Left)
                            && (app.active_tab != AppTab::Lights || alt_down)
                        {
                            let disclosure = Rect::new(
                                20.0,
                                outliner_list_top() + row as f32 * OUTLINER_ROW_H,
                                22.0,
                                OUTLINER_ROW_H,
                            );
                            if disclosure.contains(mouse) {
                                toggle_asset_group_expanded(app, &group);
                            } else {
                                select_asset_group(app, &group);
                            }
                        }
                        if is_mouse_button_pressed(MouseButton::Right) {
                            select_asset_group(app, &group);
                            start_group_rename(app, &group);
                        }
                    }
                    OutlinerEntry::Element(idx) | OutlinerEntry::GroupChild(idx) => {
                        app.hovered = Some(idx);
                        if is_mouse_button_pressed(MouseButton::Left)
                            && (app.active_tab != AppTab::Lights || alt_down)
                        {
                            if shift_down {
                                select_outliner_range(app, idx);
                            } else {
                                select_element(app, idx);
                            }
                        }
                        if is_mouse_button_pressed(MouseButton::Right) {
                            snap_to(app, idx);
                        }
                    }
                }
            }
        } else if viewport.contains(mouse) && is_mouse_button_pressed(MouseButton::Left) {
            if app.active_tab == AppTab::Lights && !alt_down {
                if let Some((light, placement)) = pick_scene_light(app, viewport, mouse) {
                    app.selected_light = light;
                    if let Some(placement) = placement {
                        app.selected = placement;
                    }
                }
                return;
            }
            let collision_vertex_pick = (app.active_tab == AppTab::Collisions
                && app.collision_edit_mode)
                .then(|| nearest_selected_collision_tab_screen_vertex(app, viewport, mouse))
                .flatten();
            let collision_pick = if let Some((face, vertex)) = collision_vertex_pick {
                Some((face, vertex))
            } else {
                (app.active_tab == AppTab::Collisions && app.collision_edit_mode)
                    .then(|| pick_selected_collision_face(app, viewport, mouse))
                    .flatten()
                    .or_else(|| {
                        (app.active_tab == AppTab::Collisions && app.collision_edit_mode)
                            .then(|| pick_collision_face(app, viewport, mouse))
                            .flatten()
                    })
                    .map(|face| {
                        (
                            face,
                            nearest_collision_tab_face_vertex(app, viewport, mouse, face),
                        )
                    })
            };
            if let Some((face, vertex)) = collision_pick {
                app.selected_col_face = Some(face);
                app.selected_col_vertex = vertex;
                if !app
                    .element_states
                    .get(face.placement)
                    .is_some_and(|state| state.deleted)
                {
                    select_element_with_mode(app, face.placement, shift_down);
                    app.selected_col_face = Some(face);
                }
            } else if let Some(idx) = if app.active_tab == AppTab::Collisions {
                pick_collision_face(app, viewport, mouse).map(|face| face.placement)
            } else {
                pick_scene_element(app, viewport, mouse)
            } {
                if !app
                    .element_states
                    .get(idx)
                    .is_some_and(|state| state.deleted)
                {
                    select_element_with_mode(app, idx, shift_down);
                }
            } else if !shift_down {
                deselect_element(app);
            }
        }
    }

    if is_key_pressed(KeyCode::Delete) || is_key_pressed(KeyCode::Backspace) {
        if app.active_tab == AppTab::Lights {
            delete_selected_light(app);
        } else if app.active_tab == AppTab::Water {
            delete_water_plane(app);
        } else if app.active_tab == AppTab::Cull {
            delete_cull_zone(app);
        } else if app.active_tab == AppTab::Editing {
            match app.editing.asset.as_ref() {
                Some(EditingAsset::Dff(dff)) if dff.select_mode == EditingSelectMode::Vertex => {
                    let before = editing_history_snapshot(app);
                    editing_delete_selected_dff_vertex(app);
                    commit_editing_history(app, "Delete DFF Vertex", before);
                }
                Some(EditingAsset::Dff(dff)) if dff.select_mode == EditingSelectMode::Edge => {
                    let before = editing_history_snapshot(app);
                    editing_delete_selected_dff_edge(app);
                    commit_editing_history(app, "Delete DFF Edge", before);
                }
                Some(EditingAsset::Dff(_)) => {
                    let before = editing_history_snapshot(app);
                    editing_delete_selected_dff_face(app);
                    commit_editing_history(app, "Delete DFF Face", before);
                }
                Some(EditingAsset::Col(col)) if col.selected_primitive.is_some() => {
                    let before = editing_history_snapshot(app);
                    if editing_delete_selected_col_primitive(app) {
                        commit_editing_history(app, "Delete COL Primitive", before);
                    }
                }
                Some(EditingAsset::Col(col)) if col.select_mode == EditingSelectMode::Edge => {
                    let before = editing_history_snapshot(app);
                    editing_delete_selected_col_edge(app);
                    commit_editing_history(app, "Delete COL Edge", before);
                }
                Some(EditingAsset::Col(col)) if col.select_mode == EditingSelectMode::Vertex => {
                    let before = editing_history_snapshot(app);
                    editing_delete_selected_col_vertex(app);
                    commit_editing_history(app, "Delete COL Vertex", before);
                }
                Some(EditingAsset::Col(_)) => {
                    let before = editing_history_snapshot(app);
                    editing_delete_selected_col_face(app);
                    commit_editing_history(app, "Delete COL Face", before);
                }
                _ => {}
            }
        } else {
            delete_selected(app);
        }
    }
    if is_key_pressed(KeyCode::R)
        && app
            .element_states
            .get(app.selected)
            .is_some_and(|state| state.deleted)
    {
        restore_selected(app);
    } else if is_key_pressed(KeyCode::R) {
        app.transform_mode = TransformMode::Rotate;
    }
    if ctrl_down {
        if is_key_pressed(KeyCode::J)
            && app.active_tab != AppTab::Lights
            && app.active_tab != AppTab::Water
            && app.active_tab != AppTab::Editing
        {
            join_selected_elements(app);
            return;
        }
        if is_key_pressed(KeyCode::D) {
            if app.active_tab == AppTab::Lights {
                duplicate_selected_light(app);
            } else if app.active_tab == AppTab::Water {
                duplicate_water_plane(app);
            } else if app.active_tab == AppTab::Cull {
                duplicate_cull_zone(app);
            } else if app.active_tab == AppTab::Editing
                && matches!(app.editing.asset, Some(EditingAsset::Col(_)))
            {
                let before = editing_history_snapshot(app);
                if editing_duplicate_selected_col_primitive(app) {
                    commit_editing_history(app, "Duplicate COL Primitive", before);
                }
            } else {
                duplicate_selected(app);
            }
        }
    }

    let move_step = if shift_down {
        64.0
    } else if ctrl_down {
        4.0
    } else {
        16.0
    };
    let mut delta = Vec3::ZERO;
    if is_key_pressed(KeyCode::Left) {
        delta.x -= move_step;
    }
    if is_key_pressed(KeyCode::Right) {
        delta.x += move_step;
    }
    if is_key_pressed(KeyCode::Up) {
        delta.y += move_step;
    }
    if is_key_pressed(KeyCode::Down) {
        delta.y -= move_step;
    }
    if is_key_pressed(KeyCode::PageUp) {
        delta.z += move_step;
    }
    if is_key_pressed(KeyCode::PageDown) {
        delta.z -= move_step;
    }
    if delta.length_squared() > 0.0 {
        if app.active_tab == AppTab::Lights {
            move_selected_light(app, delta);
        } else if app.active_tab == AppTab::Water {
            move_selected_water(app, delta);
        } else if app.active_tab == AppTab::Cull {
            move_selected_cull(app, delta);
        } else {
            move_selected(app, delta);
        }
    }
}

fn camera_mode_label(mode: CameraMode) -> &'static str {
    match mode {
        CameraMode::Freeroam => "Freeroam",
        CameraMode::Focus => "Focus",
    }
}

fn bounds_focus_target(bounds: Bounds) -> Option<Vec3> {
    if bounds.min.is_finite() && bounds.max.is_finite() {
        Some((bounds.min + bounds.max) * 0.5)
    } else {
        None
    }
}

fn editing_asset_focus_target(app: &AppState) -> Option<Vec3> {
    if app.active_tab != AppTab::Editing {
        return None;
    }
    match app.editing.asset.as_ref()? {
        EditingAsset::Dff(dff) => dff
            .preview_mesh
            .as_ref()
            .and_then(|mesh| bounds_focus_target(mesh.bounds))
            .or_else(|| bounds_focus_target(bounds_from_vertices(&dff.raw.vertices))),
        EditingAsset::Col(col) => bounds_focus_target(col.mesh.bounds),
        EditingAsset::Txd(_) => None,
    }
}

fn scene_focus_target(app: &AppState) -> Option<Vec3> {
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    let mut any = false;
    for cell in app.world_cells.iter().chain(app.lod_world_cells.iter()) {
        min = min.min(cell.min);
        max = max.max(cell.max);
        any = true;
    }
    for cell in app.scene_cells.iter().chain(app.lod_scene_cells.iter()) {
        min = min.min(cell.min);
        max = max.max(cell.max);
        any = true;
    }
    any.then_some((min + max) * 0.5)
}

fn default_camera_focus_target(app: &AppState) -> Option<Vec3> {
    editing_asset_focus_target(app)
        .or_else(|| scene_focus_target(app))
        .or_else(|| selected_origin(app))
}

fn current_camera_focus_target(app: &AppState) -> Option<Vec3> {
    app.camera_focus
        .or_else(|| default_camera_focus_target(app))
}

fn focus_camera_on_selection(app: &mut AppState) {
    let selected = selected_origin(app);
    let Some(target) = selected.or_else(|| default_camera_focus_target(app)) else {
        app.status_message = "Nothing available to focus".to_string();
        return;
    };
    app.camera_mode = CameraMode::Focus;
    app.camera_focus = Some(target);
    app.status_message = if selected.is_some() {
        "Camera focus set to selection".to_string()
    } else {
        "Camera focus set to model".to_string()
    };
}

fn toggle_camera_mode(app: &mut AppState) {
    app.camera_mode = match app.camera_mode {
        CameraMode::Freeroam => {
            if app.camera_focus.is_none() {
                app.camera_focus = default_camera_focus_target(app);
            }
            CameraMode::Focus
        }
        CameraMode::Focus => CameraMode::Freeroam,
    };
    app.status_message = format!("Camera mode: {}", camera_mode_label(app.camera_mode));
}

/// Whether an editor text control currently owns keyboard input.
///
/// Macroquad exposes typed characters separately from key-down state, so
/// draining `get_char_pressed` is not enough to keep a held W/A/S/D/Q/E from
/// reaching the camera later in the same frame. Keep the focus test in one
/// place so every custom edit box follows the same rule.
pub(crate) fn editor_text_input_active(app: &AppState) -> bool {
    let world_outliner_active = app.active_tab != AppTab::Editing
        && app.active_tab != AppTab::Race
        && left_sidebar_visible()
        && app.outliner_search_active;
    let asset_search_active = app.active_tab != AppTab::Editing
        && app.active_tab != AppTab::Vehicles
        && app.asset_browser.expanded
        && app.asset_browser.search_active;
    let vehicle_text_active = app.active_tab == AppTab::Vehicles
        && (app.vehicle_browser.search_active
            || app.vehicle_browser.build_dialog.is_some()
            || app.vehicle_browser.collision_copy_dialog.is_some());
    let editing_text_active = app.active_tab == AppTab::Editing
        && (app.editing.search_active
            || match app.editing.asset.as_ref() {
                Some(EditingAsset::Txd(txd)) => txd.search_active || txd.material_picker_open,
                Some(EditingAsset::Dff(dff)) => {
                    dff.collision_material_picker_open
                        || dff.uv_anim_picker_open
                        || dff.dff_2dfx_type_picker_open
                        || dff.dff_2dfx_corona_preset_picker_open
                        || dff.dff_2dfx_payload_editor_open
                }
                _ => false,
            });

    app.inspector_edit.is_some()
        || (app.active_tab != AppTab::Editing && app.group_rename.is_some())
        || (app.active_tab == AppTab::Race && app.race_name_edit.is_some())
        || world_outliner_active
        || asset_search_active
        || vehicle_text_active
        || editing_text_active
        || app.native_model_dropdown_open
        || app.txd_dropdown_open
        || app.load_dialog.is_some()
        || app.preferences_dialog.is_some()
        || app.save_as_dialog.is_some()
        || app.blender_import_setup.is_some()
        || app.oversized_chunk_dialog.is_some()
        || app.classify_dialog.is_some()
        || app.lod_batch_dialog.is_some()
        || app.dff_texture_duplicate_dialog.is_some()
}

pub(crate) fn update_camera(app: &mut AppState, viewport: Rect) {
    let viewport = if app.active_tab == AppTab::Vehicles {
        vehicle_preview_viewport_rect(app)
    } else {
        viewport
    };
    if app.context_menu.is_some() {
        app.camera.looking = false;
        set_ui_interaction_suppressed(true);
        set_cursor_grab(false);
        show_mouse(true);
        return;
    }
    let start_pos = app.camera.pos;
    let start_yaw = app.camera.yaw;
    let start_pitch = app.camera.pitch;
    let mut stopped_looking = false;
    let mouse: Vec2 = mouse_position().into();
    let in_view = viewport.contains(mouse);
    let uv_editor_open = matches!(
        app.editing.asset.as_ref(),
        Some(EditingAsset::Dff(dff)) if app.active_tab == AppTab::Editing && dff.uv_editor.open
    );
    let right_click_owns_viewport =
        in_view && (!uv_editor_open || editing_preview_rect(app).contains(mouse));
    if is_mouse_button_pressed(MouseButton::Right) && right_click_owns_viewport {
        app.camera.looking = true;
        app.camera.last_mouse = mouse;
        set_cursor_grab(true);
        show_mouse(false);
    }
    if is_mouse_button_released(MouseButton::Right) {
        stopped_looking = app.camera.looking;
        app.camera.looking = false;
        set_cursor_grab(false);
        show_mouse(true);
    }
    set_ui_interaction_suppressed(app.camera.looking);
    // Hold middle mouse to pan the camera across the scene (grab-style drag).
    let mid_pan = !app.camera.looking
        && is_mouse_button_down(MouseButton::Middle)
        && in_view
        && !dff_uv_editor_owns_middle_mouse(app, mouse);
    if app.camera.looking {
        let delta = mouse - app.camera.last_mouse;
        app.camera.last_mouse = mouse;
        let focus_target = if app.camera_mode == CameraMode::Focus {
            current_camera_focus_target(app)
        } else {
            None
        };
        let orbit_radius = focus_target.map(|target| (app.camera.pos - target).length().max(1.0));
        let look = CAMERA_LOOK_SENSITIVITY * clamp_camera_rotation_speed(app.camera_rotation_speed);
        app.camera.yaw += delta.x * look;
        app.camera.pitch = (app.camera.pitch - delta.y * look).clamp(-1.54, 1.54);
        if let (Some(target), Some(radius)) = (focus_target, orbit_radius) {
            let (forward, _) = camera_vectors(&app.camera);
            app.camera_focus = Some(target);
            app.camera.pos = target - forward * radius;
        }
    } else if mid_pan {
        let delta = mouse - app.camera.last_mouse;
        app.camera.last_mouse = mouse;
        let (forward, right) = camera_vectors(&app.camera);
        let up = right.cross(forward).normalize_or_zero();
        let pan_scale = (camera_speed_for_tab(app, app.active_tab)
            * camera_translation_scale(app.active_tab)
            * 0.01)
            .max(0.01);
        let shift = (right * -delta.x + up * delta.y) * pan_scale;
        if shift.length_squared() > 0.0 {
            app.camera.pos += shift;
            if app.camera_mode == CameraMode::Focus {
                let target = current_camera_focus_target(app).unwrap_or(app.camera.pos);
                app.camera_focus = Some(target + shift);
            }
        }
    } else {
        app.camera.last_mouse = mouse;
    }

    let dt = get_frame_time();
    let active_camera_speed = camera_speed_for_tab(app, app.active_tab);
    let shift_down = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
    let alt_down = is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt);
    let speed = active_camera_speed
        * camera_translation_modifier(shift_down, alt_down)
        * camera_translation_scale(app.active_tab);
    let step = speed * dt;
    let (forward, right) = camera_vectors(&app.camera);
    let mut movement = Vec3::ZERO;
    // Keep keyboard focus stable while freecam captures/hides the pointer.
    // Either mouse button can focus the model preview; clicking the UV panel
    // hands WASDEQ back to the UV workspace.
    let camera_translation_focused = app.dff_scale_input.is_none()
        && app.editing.asset.as_ref().is_none_or(|asset| match asset {
            EditingAsset::Dff(dff) if app.active_tab == AppTab::Editing && dff.uv_editor.open => {
                dff.uv_editor.viewport_keyboard_focus
            }
            _ => true,
        });
    if camera_translation_focused {
        if is_key_down(KeyCode::W) {
            movement += forward * step;
        }
        if is_key_down(KeyCode::S) {
            movement -= forward * step;
        }
        if is_key_down(KeyCode::D) {
            movement += right * step;
        }
        if is_key_down(KeyCode::A) {
            movement -= right * step;
        }
        if is_key_down(KeyCode::Q) {
            movement.z -= step;
        }
    }
    let ctrl_down = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
    let editing_extrude_pressed =
        app.active_tab == AppTab::Editing && ctrl_down && is_key_pressed(KeyCode::E);
    if camera_translation_focused && is_key_down(KeyCode::E) && !editing_extrude_pressed {
        movement.z += step;
    }
    if movement.length_squared() > 0.0 {
        app.camera.pos += movement;
        if app.camera_mode == CameraMode::Focus {
            let target = current_camera_focus_target(app).unwrap_or(app.camera.pos);
            app.camera_focus = Some(target + movement);
        }
    }
    let moved = (app.camera.pos - start_pos).length_squared() > 0.0001
        || (app.camera.yaw - start_yaw).abs() > 0.00001
        || (app.camera.pitch - start_pitch).abs() > 0.00001;
    let now = get_time();
    if moved && (stopped_looking || now - app.last_camera_persist_at > 0.5) {
        save_project_camera_state(app);
        app.last_camera_persist_at = now;
    }
}

fn camera_translation_scale(active_tab: AppTab) -> f32 {
    // Vehicle models use a smaller working scale than the scene editor. Keep
    // their freecam translation at half of the selected camera speed without
    // changing the shared preference shown to the user.
    if active_tab == AppTab::Vehicles {
        0.5
    } else {
        1.0
    }
}

fn camera_translation_modifier(shift_down: bool, alt_down: bool) -> f32 {
    let shift_multiplier = if shift_down { 3.5 } else { 1.0 };
    let alt_multiplier = if alt_down { 0.5 } else { 1.0 };
    shift_multiplier * alt_multiplier
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vehicle_freecam_translation_runs_at_half_speed() {
        assert_eq!(camera_translation_scale(AppTab::Vehicles), 0.5);
        assert_eq!(camera_translation_scale(AppTab::Editing), 1.0);
    }

    #[test]
    fn alt_halves_freecam_translation_speed() {
        assert_eq!(camera_translation_modifier(false, false), 1.0);
        assert_eq!(camera_translation_modifier(false, true), 0.5);
        assert_eq!(camera_translation_modifier(true, false), 3.5);
        assert_eq!(camera_translation_modifier(true, true), 1.75);
    }

    #[test]
    fn inspector_float_parser_rejects_non_finite_values() {
        assert_eq!(parse_finite_f32("12.5"), Some(12.5));
        assert_eq!(parse_finite_f32("NaN"), None);
        assert_eq!(parse_finite_f32("inf"), None);
        assert_eq!(parse_finite_f32("-inf"), None);
        assert_eq!(parse_finite_f32("not a number"), None);
    }

    #[test]
    fn dff_material_values_clamp_channels_and_preserve_large_surface_coefficients() {
        let mut material = default_dff_material();

        assert!(apply_dff_material_value(
            &mut material,
            InspectorField::DffMaterialRed,
            127.5
        ));
        assert!((material.color.x - 0.5).abs() < 0.0001);
        assert!(apply_dff_material_value(
            &mut material,
            InspectorField::DffMaterialAlpha,
            -1.0
        ));
        assert!(apply_dff_material_value(
            &mut material,
            InspectorField::DffMaterialAlpha,
            300.0
        ));
        assert_eq!(material.alpha, 1.0);
        assert!(apply_dff_material_value(
            &mut material,
            InspectorField::DffMaterialDiffuse,
            8.0
        ));
        assert_eq!(material.diffuse, 8.0);
        assert!(apply_dff_material_value(
            &mut material,
            InspectorField::DffMaterialSpecular,
            2.0
        ));
        assert!(apply_dff_material_value(
            &mut material,
            InspectorField::DffMaterialSpecular,
            -2.0
        ));
        assert_eq!(material.specular, 0.0);
        assert_eq!(material.ambient, 1.0);
    }

    #[test]
    fn text_insert_repairs_non_boundary_cursor() {
        let mut value = "aé".to_string();
        let mut cursor = 2;
        let mut anchor = None;

        assert!(insert_text_at_cursor(
            &mut value,
            &mut cursor,
            &mut anchor,
            " pasted"
        ));

        assert_eq!(value, "a pastedé");
        assert!(value.is_char_boundary(cursor));
    }

    #[test]
    fn text_insert_is_bounded_and_does_not_delete_for_invalid_text() {
        let mut value = "keep".to_string();
        let mut cursor = value.len();
        let mut anchor = Some(0);

        assert!(!insert_text_at_cursor(
            &mut value,
            &mut cursor,
            &mut anchor,
            "\n\t\u{7}"
        ));
        assert_eq!(value, "keep");
        assert_eq!(anchor, Some(0));

        let oversized = "é".repeat(MAX_EDITABLE_TEXT_BYTES);
        assert!(insert_text_at_cursor(
            &mut value,
            &mut cursor,
            &mut anchor,
            &oversized
        ));
        assert_eq!(value.len(), MAX_EDITABLE_TEXT_BYTES);
        assert!(value.is_char_boundary(cursor));
    }

    #[test]
    fn completed_clipboard_paste_applies_only_to_unchanged_target() {
        *lock_pending_clipboard_paste() = None;
        let mut value = "12".to_string();
        let mut cursor = value.len();
        let mut anchor = None;
        let target = text_field_target(&mut value);
        let (tx, rx) = mpsc::channel();
        *lock_pending_clipboard_paste() = Some(PendingClipboardPaste {
            target,
            expected_value: value.clone(),
            expected_cursor: cursor,
            expected_anchor: anchor,
            started_at: Instant::now(),
            rx,
        });
        tx.send(Ok("34\n".to_string())).unwrap();

        assert!(poll_clipboard_paste(&mut value, &mut cursor, &mut anchor));
        assert_eq!(value, "1234");

        let target = text_field_target(&mut value);
        let (tx, rx) = mpsc::channel();
        *lock_pending_clipboard_paste() = Some(PendingClipboardPaste {
            target,
            expected_value: value.clone(),
            expected_cursor: cursor,
            expected_anchor: anchor,
            started_at: Instant::now(),
            rx,
        });
        value.push('5');
        tx.send(Ok("ignored".to_string())).unwrap();

        assert!(!poll_clipboard_paste(&mut value, &mut cursor, &mut anchor));
        assert_eq!(value, "12345");
        *lock_pending_clipboard_paste() = None;
    }

    #[test]
    fn selection_range_repairs_non_boundary_anchor() {
        let value = "éx";

        assert_eq!(selection_range(value, value.len(), Some(1)), Some((0, 3)));
    }

    #[test]
    fn topology_edges_are_canonical_unique_and_skip_invalid_edges() {
        let edges = topology_edges([[2, 0, 1], [2, 1, 3], [3, 3, 99]], 4);

        assert_eq!(
            edges,
            BTreeSet::from([(0, 1), (0, 2), (1, 2), (1, 3), (2, 3)])
        );
    }

    #[test]
    fn selection_complement_inverts_only_within_the_active_universe() {
        let universe = BTreeSet::from([0, 1, 2, 3]);
        let selected = BTreeSet::from([1, 3, 99]);

        assert_eq!(
            selection_complement(&universe, &selected),
            BTreeSet::from([0, 2])
        );
    }

    #[test]
    fn physics_root_selection_canonicalizes_and_prefills_properties() {
        let mut attrs = BTreeMap::from([
            ("physics_root_model".to_string(), "1352".to_string()),
            ("centerOfMassX".to_string(), "4".to_string()),
        ]);
        let cone = PhysicsRootProperties::new(25.0, 50.0, 0.99, 0.03, 50.0);

        assert!(apply_physics_root_attrs(&mut attrs, Some(1238), Some(cone)) > 0);
        assert_eq!(physics_root_value_from_attrs(&attrs), Some(1238));
        assert_eq!(attrs.get("physicsRoot").map(String::as_str), Some("1238"));
        assert_eq!(attrs.get("simulated").map(String::as_str), Some("true"));
        assert_eq!(attrs.get("mass").map(String::as_str), Some("25.000000"));
        assert_eq!(attrs.get("turnMass").map(String::as_str), Some("50.000000"));
        assert_eq!(
            attrs.get("airResistance").map(String::as_str),
            Some("0.990000")
        );
        assert!(!attrs.contains_key("physics_root_model"));
        assert!(!attrs.contains_key("centerOfMassX"));

        apply_physics_root_attrs(&mut attrs, None, None);
        assert_eq!(physics_root_value_from_attrs(&attrs), None);
        assert_eq!(attrs.get("mass").map(String::as_str), Some("25.000000"));
    }

    #[test]
    fn element_type_change_updates_every_selected_placement() {
        let placement = |tag: &str| Placement {
            id: "test".to_string(),
            dff: "test".to_string(),
            zone: "SA".to_string(),
            tag: tag.to_string(),
            attrs: BTreeMap::new(),
            pos: V3::default(),
            rot: V3::default(),
        };
        let mut placements = vec![
            placement("building"),
            placement("scenery"),
            placement("object"),
        ];

        let changed = set_element_type_for_indices(&mut placements, &[0, 2, 99], "object");

        assert_eq!(changed, 1);
        assert_eq!(placements[0].tag, "object");
        assert_eq!(placements[1].tag, "scenery");
        assert_eq!(placements[2].tag, "object");
    }
}
