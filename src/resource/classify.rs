use super::super::*;

/// Element size, in world units, at or below which an element is classified as
/// an `object` instead of a `building`.
pub(crate) const DEFAULT_CLASSIFY_OBJECT_MAX_SIZE: f32 = 20.0;
const CLASSIFY_SIZE_RANGE: std::ops::RangeInclusive<f32> = 0.1..=10_000.0;

/// Attributes that mark a placement as physics driven. MTA only honours these
/// on `object` elements, so any of them forces the object classification no
/// matter how large the model is.
pub(crate) const CLASSIFY_PHYSICS_ATTRS: [&str; 22] = [
    "physicsRoot",
    "physicsRootModel",
    "physics_root",
    "physics_root_model",
    "physicalPropsRoot",
    "simulated",
    "mass",
    "turnMass",
    "turn_mass",
    "airResistance",
    "air_resistance",
    "elasticity",
    "buoyancy",
    "centerOfMassX",
    "centerOfMassY",
    "centerOfMassZ",
    "breakable",
    "frozen",
    "respawn",
    "dynamic",
    "forceObject",
    "force_object",
];

/// Object-only attributes that are not physics. They never force the object
/// classification, but an element that uses one cannot become a building
/// without losing the feature.
const CLASSIFY_OBJECT_ONLY_ATTRS: [&str; 4] = ["scale", "streamable", "alpha", "doublesided"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ClassifyReason {
    /// The element carries physics attributes, so it must stay an object.
    Physics,
    /// The element is at or below the object size threshold.
    Small,
    /// The element is above the object size threshold.
    Large,
}

impl ClassifyReason {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Physics => "physics",
            Self::Small => "size",
            Self::Large => "size",
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ClassifyCandidate {
    pub(crate) index: usize,
    pub(crate) id: String,
    pub(crate) current_tag: String,
    pub(crate) target_tag: &'static str,
    /// Largest world-axis extent of the placed model, when a model is loaded.
    pub(crate) size: Option<f32>,
    pub(crate) reason: ClassifyReason,
    pub(crate) selected: bool,
    /// Set when the element matches a rule but cannot be retagged safely.
    pub(crate) blocked_reason: Option<String>,
}

pub(crate) struct ClassifyElementsDialog {
    pub(crate) size: String,
    pub(crate) size_cursor: usize,
    pub(crate) size_selection_anchor: Option<usize>,
    pub(crate) candidates: Vec<ClassifyCandidate>,
    /// Live, classifiable elements considered by the last review.
    pub(crate) scanned: usize,
    pub(crate) scroll: f32,
    pub(crate) error: Option<String>,
    /// Candidate discovery is spread across frames so large maps never stall
    /// the editor while their transformed bounds are inspected.
    pub(crate) scanning: bool,
    pub(crate) scan_cursor: usize,
    pub(crate) scan_max_object_size: f32,
}

fn attr_present(attrs: &BTreeMap<String, String>, keys: &[&str]) -> bool {
    keys.iter().any(|key| {
        attrs
            .get(*key)
            .is_some_and(|value| !value.trim().is_empty())
    })
}

fn attr_number(attrs: &BTreeMap<String, String>, key: &str) -> f32 {
    attrs
        .get(key)
        .and_then(|value| value.trim().parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(0.0)
}

/// Whether a placement (or the definition it instances) is physics driven.
pub(crate) fn placement_is_physics_element(
    placement: &Placement,
    definitions: &HashMap<String, Definition>,
) -> bool {
    attr_present(&placement.attrs, &CLASSIFY_PHYSICS_ATTRS)
        || definitions
            .get(&placement.id)
            .is_some_and(|definition| attr_present(&definition.attrs, &CLASSIFY_PHYSICS_ATTRS))
}

/// Why an element cannot be turned into a building, if it cannot.
fn building_blocker(
    placement: &Placement,
    definitions: &HashMap<String, Definition>,
) -> Option<String> {
    let definition_attrs = definitions.get(&placement.id).map(|def| &def.attrs);
    let uses_object_only = attr_present(&placement.attrs, &CLASSIFY_OBJECT_ONLY_ATTRS)
        || definition_attrs.is_some_and(|attrs| attr_present(attrs, &CLASSIFY_OBJECT_ONLY_ATTRS));
    if uses_object_only {
        return Some("uses object-only attributes".to_string());
    }
    if attr_number(&placement.attrs, "dimension") != 0.0 {
        return Some("placed in a custom dimension".to_string());
    }
    if attr_number(&placement.attrs, "interior") != 0.0 {
        return Some("placed in an interior".to_string());
    }
    None
}

fn parse_classify_size(dialog: &ClassifyElementsDialog) -> Result<f32, String> {
    match dialog.size.trim().parse::<f32>() {
        Ok(value) if value.is_finite() && CLASSIFY_SIZE_RANGE.contains(&value) => Ok(value),
        _ => Err("Object size must be a number from 0.1 to 10,000".to_string()),
    }
}

/// Largest world-axis extent of a placed element, or `None` when its model is
/// not loaded and the size rule cannot be evaluated.
fn placement_world_size(app: &AppState, placement: &Placement) -> Option<f32> {
    let mesh = element_mesh(app, placement)?;
    let bounds = transformed_bounds(mesh.bounds, &placement_matrix(placement).to_cols_array());
    let extents = bounds.max - bounds.min;
    Some(extents.x.max(extents.y).max(extents.z))
}

/// Element type an element should carry. Physics elements are always objects,
/// whatever their size; everything else is decided by the size threshold, and
/// stays unclassified while its model is missing.
fn classification(
    is_physics: bool,
    size: Option<f32>,
    max_object_size: f32,
) -> Option<(&'static str, ClassifyReason)> {
    if is_physics {
        return Some(("object", ClassifyReason::Physics));
    }
    match size? {
        size if size <= max_object_size => Some(("object", ClassifyReason::Small)),
        _ => Some(("building", ClassifyReason::Large)),
    }
}

fn classify_placement(
    app: &AppState,
    placement: &Placement,
    max_object_size: f32,
) -> Option<(&'static str, ClassifyReason, Option<f32>)> {
    let size = placement_world_size(app, placement);
    let is_physics = placement_is_physics_element(placement, &app.definitions);
    classification(is_physics, size, max_object_size)
        .map(|(target_tag, reason)| (target_tag, reason, size))
}

/// Starts rebuilding the review list from the dialog's current size threshold.
/// The actual scan is advanced by `update_classify_scan` over subsequent frames.
pub(crate) fn refresh_classify_candidates(app: &mut AppState) -> bool {
    let Some(dialog) = app.classify_dialog.as_ref() else {
        return false;
    };
    let max_object_size = match parse_classify_size(dialog) {
        Ok(size) => size,
        Err(error) => {
            if let Some(dialog) = app.classify_dialog.as_mut() {
                dialog.error = Some(error);
            }
            return false;
        }
    };
    if let Some(dialog) = app.classify_dialog.as_mut() {
        dialog.candidates.clear();
        dialog.scanned = 0;
        dialog.scroll = 0.0;
        dialog.error = None;
        dialog.scanning = true;
        dialog.scan_cursor = 0;
        dialog.scan_max_object_size = max_object_size;
    }
    app.status_message = "Reviewing element classifications...".to_string();
    true
}

pub(crate) fn update_classify_scan(app: &mut AppState) {
    const PLACEMENTS_PER_FRAME: usize = 256;
    let Some(dialog) = app.classify_dialog.as_ref() else {
        return;
    };
    if !dialog.scanning {
        return;
    }
    let start = dialog.scan_cursor;
    let end = (start + PLACEMENTS_PER_FRAME).min(app.placements.len());
    let max_object_size = dialog.scan_max_object_size;
    let mut candidates = Vec::new();
    let mut scanned = 0usize;
    for index in start..end {
        if !is_live_element(app, index) {
            continue;
        }
        let placement = &app.placements[index];
        // Only the two MTA world element types take part. Scenery keeps its
        // authored type, and an LOD's type must follow the element it covers.
        if !(placement.tag.eq_ignore_ascii_case("object")
            || placement.tag.eq_ignore_ascii_case("building"))
            || placement_is_app_lod(app, placement)
        {
            continue;
        }
        scanned += 1;
        let Some((target_tag, reason, size)) = classify_placement(app, placement, max_object_size)
        else {
            continue;
        };
        if placement.tag.eq_ignore_ascii_case(target_tag) {
            continue;
        }
        let blocked_reason = (target_tag == "building")
            .then(|| building_blocker(placement, &app.definitions))
            .flatten();
        candidates.push(ClassifyCandidate {
            index,
            id: placement.id.clone(),
            current_tag: placement.tag.clone(),
            target_tag,
            size,
            reason,
            selected: blocked_reason.is_none(),
            blocked_reason,
        });
    }
    let complete = end >= app.placements.len();
    if let Some(dialog) = app.classify_dialog.as_mut() {
        dialog.candidates.extend(candidates);
        dialog.scanned += scanned;
        dialog.scan_cursor = end;
        if complete {
            dialog.scanning = false;
        }
    }
    if complete {
        app.classify_object_max_size = max_object_size;
        app.status_message = "Element classification review ready".to_string();
    }
}

pub(crate) fn open_classify_dialog(app: &mut AppState) {
    let size = format!("{:.1}", app.classify_object_max_size);
    app.classify_dialog = Some(ClassifyElementsDialog {
        size_cursor: size.len(),
        size_selection_anchor: Some(0),
        size,
        candidates: Vec::new(),
        scanned: 0,
        scroll: 0.0,
        error: None,
        scanning: false,
        scan_cursor: 0,
        scan_max_object_size: app.classify_object_max_size,
    });
    refresh_classify_candidates(app);
}

/// Applies the selected reclassifications as a single undo step.
pub(crate) fn apply_classify_candidates(app: &mut AppState) -> bool {
    let Some(dialog) = app.classify_dialog.as_ref() else {
        return false;
    };
    if dialog.scanning {
        if let Some(dialog) = app.classify_dialog.as_mut() {
            dialog.error = Some("Wait for the classification review to finish".to_string());
        }
        return false;
    }
    let size = match parse_classify_size(dialog) {
        Ok(size) => size,
        Err(error) => {
            if let Some(dialog) = app.classify_dialog.as_mut() {
                dialog.error = Some(error);
            }
            return false;
        }
    };
    // The list is only meaningful for the size it was built with, so a typed
    // change is reviewed again instead of applied against stale rows.
    if size != app.classify_object_max_size {
        refresh_classify_candidates(app);
        if let Some(dialog) = app.classify_dialog.as_mut() {
            dialog.error = Some("Size changed: review the updated list, then apply".to_string());
        }
        return false;
    }
    let applied = app
        .classify_dialog
        .as_ref()
        .map(|dialog| {
            dialog
                .candidates
                .iter()
                .filter(|candidate| candidate.selected && candidate.blocked_reason.is_none())
                .map(|candidate| (candidate.index, candidate.target_tag))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if applied.is_empty() {
        if let Some(dialog) = app.classify_dialog.as_mut() {
            dialog.error = Some("Select at least one element to reclassify".to_string());
        }
        return false;
    }
    let before = world_history_snapshot(app);
    let mut objects = 0usize;
    let mut buildings = 0usize;
    for (index, target_tag) in applied {
        let Some(placement) = app.placements.get_mut(index) else {
            continue;
        };
        if placement.tag == target_tag {
            continue;
        }
        placement.tag = target_tag.to_string();
        if target_tag == "building" {
            buildings += 1;
        } else {
            objects += 1;
        }
    }
    let changed = objects + buildings;
    if changed > 0 {
        invalidate_outliner_labels(app);
        rebuild_outliner_filter(app);
        rebuild_render_cells(app);
        commit_world_history(app, "Classify Elements", before);
    }
    app.classify_dialog = None;
    app.status_message = format!(
        "Classified {changed} element(s): {objects} object(s), {buildings} building(s) at {:.1} units.",
        app.classify_object_max_size
    );
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placement_with(tag: &str, attrs: &[(&str, &str)]) -> Placement {
        Placement {
            id: "test".to_string(),
            dff: "test.dff".to_string(),
            zone: "SA".to_string(),
            tag: tag.to_string(),
            attrs: attrs
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect(),
            pos: V3::default(),
            rot: V3::default(),
        }
    }

    #[test]
    fn physics_attributes_on_placement_or_definition_mark_physics_elements() {
        let definitions = HashMap::new();
        assert!(placement_is_physics_element(
            &placement_with("building", &[("mass", "500")]),
            &definitions
        ));
        assert!(placement_is_physics_element(
            &placement_with("building", &[("physicsRoot", "1337")]),
            &definitions
        ));
        assert!(!placement_is_physics_element(
            &placement_with("building", &[("mass", "  ")]),
            &definitions
        ));

        let mut definitions = HashMap::new();
        definitions.insert(
            "test".to_string(),
            Definition {
                id: "test".to_string(),
                zone: "SA".to_string(),
                attrs: [("simulated".to_string(), "true".to_string())]
                    .into_iter()
                    .collect(),
            },
        );
        assert!(placement_is_physics_element(
            &placement_with("building", &[]),
            &definitions
        ));
    }

    #[test]
    fn object_only_attributes_block_building_promotion() {
        let definitions = HashMap::new();
        assert_eq!(
            building_blocker(&placement_with("object", &[("scale", "2")]), &definitions),
            Some("uses object-only attributes".to_string())
        );
        assert_eq!(
            building_blocker(
                &placement_with("object", &[("dimension", "3")]),
                &definitions
            ),
            Some("placed in a custom dimension".to_string())
        );
        assert_eq!(
            building_blocker(
                &placement_with("object", &[("dimension", "0"), ("interior", "0")]),
                &definitions
            ),
            None
        );
    }

    #[test]
    fn physics_elements_stay_objects_at_any_size() {
        assert_eq!(
            classification(true, Some(400.0), 20.0),
            Some(("object", ClassifyReason::Physics))
        );
        assert_eq!(
            classification(true, None, 20.0),
            Some(("object", ClassifyReason::Physics))
        );
    }

    #[test]
    fn size_threshold_splits_objects_from_buildings() {
        assert_eq!(
            classification(false, Some(19.9), 20.0),
            Some(("object", ClassifyReason::Small))
        );
        assert_eq!(
            classification(false, Some(20.0), 20.0),
            Some(("object", ClassifyReason::Small))
        );
        assert_eq!(
            classification(false, Some(20.1), 20.0),
            Some(("building", ClassifyReason::Large))
        );
        // An unloaded model has no measurable size, so it is left alone.
        assert_eq!(classification(false, None, 20.0), None);
    }

    #[test]
    fn size_threshold_is_validated() {
        let dialog = |value: &str| ClassifyElementsDialog {
            size: value.to_string(),
            size_cursor: 0,
            size_selection_anchor: None,
            candidates: Vec::new(),
            scanned: 0,
            scroll: 0.0,
            error: None,
            scanning: false,
            scan_cursor: 0,
            scan_max_object_size: DEFAULT_CLASSIFY_OBJECT_MAX_SIZE,
        };

        assert_eq!(parse_classify_size(&dialog(" 24 ")), Ok(24.0));
        assert!(parse_classify_size(&dialog("0")).is_err());
        assert!(parse_classify_size(&dialog("20000")).is_err());
        assert!(parse_classify_size(&dialog("big")).is_err());
    }
}
