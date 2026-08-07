use super::super::*;

const DEFAULT_MODEL_DISTANCE: f32 = 200.0;
const PROMOTED_MODEL_DISTANCE: f32 = 700.0;
const DENSITY_RADIUS: f32 = 768.0;
const DENSE_LOD_COUNT: usize = 24;
const DENSE_LOD_TRIANGLES: usize = 100_000;
const SMALL_LOD_MAX_EXTENT: f32 = 20.0;
const UNCOVERED_DETAIL_MIN_EXTENT: f32 = 80.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum LodAuditFilter {
    #[default]
    All,
    Missing,
    Dense,
    Small,
    Ignored,
}

impl LodAuditFilter {
    pub(crate) const ALL: [Self; 5] = [
        Self::All,
        Self::Missing,
        Self::Dense,
        Self::Small,
        Self::Ignored,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Missing => "Gaps",
            Self::Dense => "Dense",
            Self::Small => "Small",
            Self::Ignored => "Ignored",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LodAuditIssueKind {
    Missing,
    Dense,
    Small,
}

impl LodAuditIssueKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Missing => "COVERAGE",
            Self::Dense => "DENSE",
            Self::Small => "SMALL",
        }
    }

    pub(crate) fn color(self) -> Color {
        match self {
            Self::Missing => Color::new(0.96, 0.30, 0.25, 1.0),
            Self::Dense => Color::new(1.0, 0.62, 0.16, 1.0),
            Self::Small => Color::new(0.72, 0.42, 0.95, 1.0),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct LodAuditIssue {
    pub(crate) kind: LodAuditIssueKind,
    pub(crate) title: String,
    pub(crate) detail: String,
    pub(crate) position: V3,
    pub(crate) placement_indices: Vec<usize>,
    pub(crate) min: V3,
    pub(crate) max: V3,
    /// Maximum world-axis extent for size sorting. Dense issues use the
    /// aggregate hotspot extent.
    pub(crate) size: f32,
}

#[derive(Clone, Debug)]
pub(crate) enum LodAuditRole {
    Detail,
    Child { target: usize },
    LodTarget { children: Vec<usize> },
    SelfLod,
}

#[derive(Clone, Debug)]
pub(crate) struct LodAuditPlacementInfo {
    pub(crate) role: LodAuditRole,
    pub(crate) center: V3,
    pub(crate) min: V3,
    pub(crate) max: V3,
    pub(crate) radius: f32,
    /// The exact value passed by eagleLoader to engineSetModelLODDistance,
    /// including its default/promotion rule and drawDistanceMultiplier.
    pub(crate) model_distance: f32,
    pub(crate) low_distance: f32,
    pub(crate) triangles: usize,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct LodAuditResult {
    pub(crate) placements: Vec<LodAuditPlacementInfo>,
    pub(crate) issues: Vec<LodAuditIssue>,
    pub(crate) loader_distance_multiplier: f32,
    pub(crate) loader_config_found: bool,
}

pub(crate) struct LodAuditState {
    pub(crate) result: Option<LodAuditResult>,
    pub(crate) rx: Option<mpsc::Receiver<LodAuditUpdate>>,
    pub(crate) revision: u64,
    pub(crate) stale: bool,
    pub(crate) progress: (usize, usize),
    pub(crate) filter: LodAuditFilter,
    pub(crate) selected_issue: Option<usize>,
    /// MTA's Video-tab draw-distance setting. High-detail objects scale from
    /// 1x at 0% to approximately 2x at 100%; low-LOD objects ignore it.
    pub(crate) client_draw_percent: u8,
    pub(crate) small_threshold: f32,
    pub(crate) small_sort_ascending: bool,
    /// Stable, project-scoped coverage warning keys. These survive audit
    /// reruns and editor restarts.
    pub(crate) ignored_coverage: HashSet<String>,
}

impl Default for LodAuditState {
    fn default() -> Self {
        Self {
            result: None,
            rx: None,
            revision: 0,
            stale: true,
            progress: (0, 0),
            filter: LodAuditFilter::All,
            selected_issue: None,
            client_draw_percent: 0,
            small_threshold: SMALL_LOD_MAX_EXTENT,
            small_sort_ascending: true,
            ignored_coverage: HashSet::new(),
        }
    }
}

pub(crate) fn lod_audit_state_from_preferences(project_root: &Path) -> LodAuditState {
    LodAuditState {
        small_threshold: load_lod_audit_small_threshold_preference(),
        ignored_coverage: load_lod_audit_ignored_coverage(project_root),
        ..LodAuditState::default()
    }
}

fn stable_hash_bytes(bytes: &[u8]) -> u64 {
    // FNV-1a keeps persisted issue identities stable across Rust/toolchain
    // upgrades, unlike DefaultHasher whose algorithm is intentionally opaque.
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub(crate) fn lod_audit_coverage_ignore_key(issue: &LodAuditIssue) -> Option<String> {
    if issue.kind != LodAuditIssueKind::Missing {
        return None;
    }
    let identity = format!(
        "{}|{:08x}|{:08x}|{:08x}|{:08x}|{:08x}|{:08x}|{:08x}|{:08x}|{:08x}",
        issue.title,
        issue.position.x.to_bits(),
        issue.position.y.to_bits(),
        issue.position.z.to_bits(),
        issue.min.x.to_bits(),
        issue.min.y.to_bits(),
        issue.min.z.to_bits(),
        issue.max.x.to_bits(),
        issue.max.y.to_bits(),
        issue.max.z.to_bits(),
    );
    Some(format!("{:016x}", stable_hash_bytes(identity.as_bytes())))
}

pub(crate) fn lod_audit_issue_is_ignored(app: &AppState, issue: &LodAuditIssue) -> bool {
    lod_audit_coverage_ignore_key(issue)
        .is_some_and(|key| app.lod_audit.ignored_coverage.contains(&key))
}

pub(crate) fn lod_audit_issue_count(app: &AppState, kind: LodAuditIssueKind) -> usize {
    app.lod_audit
        .result
        .as_ref()
        .map(|result| {
            result
                .issues
                .iter()
                .filter(|issue| issue.kind == kind && !lod_audit_issue_is_ignored(app, issue))
                .count()
        })
        .unwrap_or(0)
}

pub(crate) fn result_ignored_count(app: &AppState) -> usize {
    app.lod_audit
        .result
        .as_ref()
        .map(|result| {
            result
                .issues
                .iter()
                .filter(|issue| lod_audit_issue_is_ignored(app, issue))
                .count()
        })
        .unwrap_or(0)
}

pub(crate) fn lod_audit_issue_generation_indices(app: &AppState, issue_index: usize) -> Vec<usize> {
    let Some(result) = app.lod_audit.result.as_ref() else {
        return Vec::new();
    };
    let Some(issue) = result.issues.get(issue_index) else {
        return Vec::new();
    };
    if issue.kind != LodAuditIssueKind::Missing {
        return Vec::new();
    }
    issue
        .placement_indices
        .iter()
        .copied()
        .filter(|index| {
            is_live_element(app, *index)
                && result
                    .placements
                    .get(*index)
                    .is_some_and(|info| matches!(info.role, LodAuditRole::Detail))
        })
        .collect()
}

pub(crate) fn toggle_lod_audit_coverage_ignore(app: &mut AppState, issue_index: usize) {
    let Some((key, title)) = app
        .lod_audit
        .result
        .as_ref()
        .and_then(|result| result.issues.get(issue_index))
        .and_then(|issue| {
            lod_audit_coverage_ignore_key(issue).map(|key| (key, issue.title.clone()))
        })
    else {
        app.status_message = "Only Coverage warnings can be ignored.".to_string();
        return;
    };
    let ignored = if app.lod_audit.ignored_coverage.remove(&key) {
        false
    } else {
        app.lod_audit.ignored_coverage.insert(key);
        true
    };
    app.lod_audit.selected_issue = None;
    app.properties_scroll = 0.0;
    let persistence = save_lod_audit_ignored_coverage(&app.root, &app.lod_audit.ignored_coverage);
    app.status_message = match (ignored, persistence) {
        (true, Ok(())) => format!("Ignored Coverage warning: {title}"),
        (false, Ok(())) => format!("Restored Coverage warning: {title}"),
        (true, Err(error)) => {
            format!("Ignored Coverage warning for this session, but could not persist it: {error}")
        }
        (false, Err(error)) => {
            format!("Restored Coverage warning for this session, but could not persist it: {error}")
        }
    };
}

#[derive(Clone, Debug)]
struct LodAuditSnapshot {
    index: usize,
    id: String,
    tag: String,
    lod_parent: Option<String>,
    unique_id: Option<String>,
    definition_lod_distance: Option<f32>,
    center: V3,
    min: V3,
    max: V3,
    radius: f32,
    triangles: usize,
    live: bool,
    can_promote_to_building: bool,
}

#[derive(Debug)]
pub(crate) enum LodAuditUpdate {
    Progress {
        revision: u64,
        scanned: usize,
        total: usize,
    },
    Complete {
        revision: u64,
        result: LodAuditResult,
    },
    Failed {
        revision: u64,
        message: String,
    },
}

fn finite_attr_f32(value: Option<&String>) -> Option<f32> {
    value?
        .trim()
        .parse::<f32>()
        .ok()
        .filter(|value| value.is_finite())
}

fn loader_distance(raw: Option<f32>, multiplier: f32) -> f32 {
    let distance = raw.unwrap_or(DEFAULT_MODEL_DISTANCE);
    let distance = if distance < 10.0 {
        PROMOTED_MODEL_DISTANCE
    } else {
        distance
    };
    (distance * multiplier).max(0.0)
}

struct CanonicalLoaderSettings {
    distance_multiplier: f32,
    found: bool,
    prefer_static_buildings: bool,
    force_object: bool,
}

fn xml_bool_attr(element: &str, name: &str, default: bool) -> bool {
    let Some(attr_start) = element.find(name) else {
        return default;
    };
    let value = &element[attr_start + name.len()..];
    let Some((_, quoted)) = value.split_once('"') else {
        return default;
    };
    let Some((value, _)) = quoted.split_once('"') else {
        return default;
    };
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn canonical_loader_settings() -> CanonicalLoaderSettings {
    let config_path = env::var_os("EAGLE_LOADER_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("eagleLoader/config.xml"));
    let Ok(xml) = fs::read_to_string(config_path) else {
        return CanonicalLoaderSettings {
            distance_multiplier: 1.0,
            found: false,
            prefer_static_buildings: true,
            force_object: false,
        };
    };
    let Some(streaming_start) = xml.find("<streaming") else {
        return CanonicalLoaderSettings {
            distance_multiplier: 1.0,
            found: true,
            prefer_static_buildings: true,
            force_object: false,
        };
    };
    let tail = &xml[streaming_start..];
    let element = tail
        .split_once('>')
        .map(|(element, _)| element)
        .unwrap_or(tail);
    let multiplier = element
        .find("drawDistanceMultiplier")
        .and_then(|attr_start| {
            element[attr_start + "drawDistanceMultiplier".len()..]
                .split_once('"')
                .and_then(|(_, quoted)| quoted.split_once('"'))
                .map(|(number, _)| number)
        })
        .and_then(|number| number.trim().parse::<f32>().ok())
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(1.0);
    let force_object = xml
        .find("<debug")
        .map(|start| &xml[start..])
        .and_then(|tail| tail.split_once('>').map(|(element, _)| element))
        .is_some_and(|debug| xml_bool_attr(debug, "forceObject", false));
    CanonicalLoaderSettings {
        distance_multiplier: multiplier,
        found: true,
        prefer_static_buildings: xml_bool_attr(element, "preferStaticBuildings", true),
        force_object,
    }
}

fn v3_distance_squared(a: V3, b: V3) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    dx * dx + dy * dy + dz * dz
}

fn v3_extent(min: V3, max: V3) -> V3 {
    V3 {
        x: (max.x - min.x).abs(),
        y: (max.y - min.y).abs(),
        z: (max.z - min.z).abs(),
    }
}

fn max_extent(min: V3, max: V3) -> f32 {
    let extent = v3_extent(min, max);
    extent.x.max(extent.y).max(extent.z)
}

fn union_snapshot_bounds<'a>(
    indices: impl IntoIterator<Item = &'a usize>,
    snapshots: &[LodAuditSnapshot],
) -> Option<(V3, V3)> {
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
    let mut any = false;
    for index in indices {
        let Some(snapshot) = snapshots.get(*index) else {
            continue;
        };
        min.x = min.x.min(snapshot.min.x);
        min.y = min.y.min(snapshot.min.y);
        min.z = min.z.min(snapshot.min.z);
        max.x = max.x.max(snapshot.max.x);
        max.y = max.y.max(snapshot.max.y);
        max.z = max.z.max(snapshot.max.z);
        any = true;
    }
    any.then_some((min, max))
}

fn position_from_bounds(min: V3, max: V3) -> V3 {
    V3 {
        x: (min.x + max.x) * 0.5,
        y: (min.y + max.y) * 0.5,
        z: (min.z + max.z) * 0.5,
    }
}

fn resolve_lod_target(
    child: &LodAuditSnapshot,
    candidates: &[usize],
    snapshots: &[LodAuditSnapshot],
) -> Option<usize> {
    match candidates {
        [] => return None,
        [only] => return Some(*only),
        _ => {}
    }
    if let Some(unique_id) = child.unique_id.as_deref() {
        if let Some(index) = candidates.iter().copied().find(|index| {
            snapshots
                .get(*index)
                .and_then(|snapshot| snapshot.unique_id.as_deref())
                .is_some_and(|candidate| candidate.eq_ignore_ascii_case(unique_id))
        }) {
            return Some(index);
        }
    }
    candidates.iter().copied().min_by(|a, b| {
        v3_distance_squared(snapshots[*a].center, child.center)
            .total_cmp(&v3_distance_squared(snapshots[*b].center, child.center))
    })
}

fn lod_audit_analysis(
    snapshots: Vec<LodAuditSnapshot>,
    revision: u64,
    small_threshold: f32,
    progress_tx: &mpsc::Sender<LodAuditUpdate>,
) -> LodAuditResult {
    let total = snapshots.len();
    let loader_settings = canonical_loader_settings();
    let distance_multiplier = loader_settings.distance_multiplier;
    let config_found = loader_settings.found;
    let referenced_ids = snapshots
        .iter()
        .filter_map(|snapshot| snapshot.lod_parent.as_deref())
        .filter(|parent| !parent.eq_ignore_ascii_case("self"))
        .map(|parent| parent.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let mut target_candidates = HashMap::<String, Vec<usize>>::new();
    for snapshot in &snapshots {
        let is_target = referenced_ids.contains(&snapshot.id.to_ascii_lowercase())
            || is_lod_name(&snapshot.id, "");
        if snapshot.live && is_target {
            target_candidates
                .entry(snapshot.id.to_ascii_lowercase())
                .or_default()
                .push(snapshot.index);
        }
    }

    let mut roles = vec![LodAuditRole::Detail; snapshots.len()];
    let mut children_by_target = vec![Vec::<usize>::new(); snapshots.len()];
    let mut issues = Vec::<LodAuditIssue>::new();
    for (scanned, snapshot) in snapshots.iter().enumerate() {
        if !snapshot.live {
            continue;
        }
        match snapshot.lod_parent.as_deref() {
            Some(parent) if parent.eq_ignore_ascii_case("self") => {
                roles[snapshot.index] = LodAuditRole::SelfLod;
            }
            Some(parent) => {
                let candidates = target_candidates
                    .get(&parent.to_ascii_lowercase())
                    .map(Vec::as_slice)
                    .unwrap_or_default();
                if let Some(target) = resolve_lod_target(snapshot, candidates, &snapshots) {
                    roles[snapshot.index] = LodAuditRole::Child { target };
                    children_by_target[target].push(snapshot.index);
                } else {
                    issues.push(LodAuditIssue {
                        kind: LodAuditIssueKind::Missing,
                        title: format!("{} has no LOD target", snapshot.id),
                        detail: format!(
                            "lodParent=\"{parent}\" does not resolve to a live placement."
                        ),
                        position: snapshot.center,
                        placement_indices: vec![snapshot.index],
                        min: snapshot.min,
                        max: snapshot.max,
                        size: max_extent(snapshot.min, snapshot.max),
                    });
                }
            }
            None => {}
        }
        if (scanned + 1) % 1024 == 0 {
            let _ = progress_tx.send(LodAuditUpdate::Progress {
                revision,
                scanned: scanned + 1,
                total,
            });
        }
    }
    for candidates in target_candidates.values() {
        for target in candidates {
            roles[*target] = LodAuditRole::LodTarget {
                children: std::mem::take(&mut children_by_target[*target]),
            };
        }
    }

    let mut placements = Vec::with_capacity(snapshots.len());
    for snapshot in &snapshots {
        let model_distance = loader_distance(snapshot.definition_lod_distance, distance_multiplier);
        let inside_building_bounds = snapshot.center.x > -3000.0
            && snapshot.center.x < 3000.0
            && snapshot.center.y > -3000.0
            && snapshot.center.y < 3000.0;
        let is_building = !loader_settings.force_object
            && inside_building_bounds
            && (snapshot.tag.eq_ignore_ascii_case("building")
                || (loader_settings.prefer_static_buildings && snapshot.can_promote_to_building));
        let low_distance = if is_building && model_distance <= 300.0 {
            // MTA documents low-LOD buildings at <=300 as always visible.
            f32::INFINITY
        } else if is_building {
            model_distance
        } else {
            model_distance * 5.0
        };
        placements.push(LodAuditPlacementInfo {
            role: roles[snapshot.index].clone(),
            center: snapshot.center,
            min: snapshot.min,
            max: snapshot.max,
            radius: snapshot.radius,
            model_distance,
            low_distance,
            triangles: snapshot.triangles,
        });
    }

    let mut lod_targets = Vec::<usize>::new();
    for snapshot in &snapshots {
        if !snapshot.live {
            continue;
        }
        match &placements[snapshot.index].role {
            LodAuditRole::LodTarget { children } => {
                lod_targets.push(snapshot.index);
                if placements[snapshot.index].low_distance.is_infinite() {
                    issues.push(LodAuditIssue {
                        kind: LodAuditIssueKind::Dense,
                        title: format!("{} low-LOD building is always visible", snapshot.id),
                        detail: format!(
                            "Its effective LOD distance is {:.0}; MTA requires low-LOD buildings to be above 300 for normal culling.",
                            placements[snapshot.index].model_distance
                        ),
                        position: snapshot.center,
                        placement_indices: vec![snapshot.index],
                        min: snapshot.min,
                        max: snapshot.max,
                        size: max_extent(snapshot.min, snapshot.max),
                    });
                }
                if children.is_empty() {
                    issues.push(LodAuditIssue {
                        kind: LodAuditIssueKind::Missing,
                        title: format!("{} is an orphan LOD", snapshot.id),
                        detail: "No live detail placement resolves to this LOD target.".to_string(),
                        position: snapshot.center,
                        placement_indices: vec![snapshot.index],
                        min: snapshot.min,
                        max: snapshot.max,
                        size: max_extent(snapshot.min, snapshot.max),
                    });
                    continue;
                }
                if let Some((group_min, group_max)) =
                    union_snapshot_bounds(children.iter(), &snapshots)
                {
                    let extent = max_extent(group_min, group_max);
                    if extent <= small_threshold {
                        let transition = children
                            .iter()
                            .map(|child| placements[*child].model_distance)
                            .fold(f32::INFINITY, f32::min)
                            .max(1.0);
                        let approx_pixels = extent / transition * 771.0;
                        let mut group = Vec::with_capacity(children.len() + 1);
                        group.push(snapshot.index);
                        group.extend(children.iter().copied());
                        issues.push(LodAuditIssue {
                            kind: LodAuditIssueKind::Small,
                            title: format!("{} may not need an LOD", snapshot.id),
                            detail: format!(
                                "Detail group is only {extent:.1} units across (~{approx_pixels:.0}px at its nearest transition). Review only; no assets are changed."
                            ),
                            position: position_from_bounds(group_min, group_max),
                            placement_indices: group,
                            min: group_min,
                            max: group_max,
                            size: extent,
                        });
                    }
                }
            }
            LodAuditRole::SelfLod => {
                let extent = max_extent(snapshot.min, snapshot.max);
                if extent <= small_threshold {
                    issues.push(LodAuditIssue {
                        kind: LodAuditIssueKind::Small,
                        title: format!("{} self-LOD may be unnecessary", snapshot.id),
                        detail: format!(
                            "Object is {extent:.1} units across and duplicates itself as a low LOD. Review only; no assets are changed."
                        ),
                        position: snapshot.center,
                        placement_indices: vec![snapshot.index],
                        min: snapshot.min,
                        max: snapshot.max,
                        size: extent,
                    });
                }
            }
            LodAuditRole::Detail => {
                let extent = max_extent(snapshot.min, snapshot.max);
                if extent >= UNCOVERED_DETAIL_MIN_EXTENT {
                    issues.push(LodAuditIssue {
                        kind: LodAuditIssueKind::Missing,
                        title: format!("{} has no LOD coverage", snapshot.id),
                        detail: format!(
                            "Large detail placement ({extent:.0} units) has no lodParent and disappears at {:.0} units.",
                            placements[snapshot.index].model_distance
                        ),
                        position: snapshot.center,
                        placement_indices: vec![snapshot.index],
                        min: snapshot.min,
                        max: snapshot.max,
                        size: extent,
                    });
                }
            }
            LodAuditRole::Child { .. } => {}
        }
    }

    // Sample each LOD neighborhood, then suppress heavily overlapping hot
    // samples. This measures concurrently nearby long-range chunks instead of
    // merely counting how many origins happen to land in one grid bin.
    let density_radius_sq = DENSITY_RADIUS * DENSITY_RADIUS;
    let mut hot_spots = Vec::<(usize, usize, V3, Vec<usize>)>::new();
    for center_index in &lod_targets {
        let center = snapshots[*center_index].center;
        let nearby = lod_targets
            .iter()
            .copied()
            .filter(|index| {
                let candidate = snapshots[*index].center;
                let dx = candidate.x - center.x;
                let dy = candidate.y - center.y;
                dx * dx + dy * dy <= density_radius_sq
            })
            .collect::<Vec<_>>();
        let triangles = nearby
            .iter()
            .map(|index| placements[*index].triangles)
            .sum::<usize>();
        if nearby.len() >= DENSE_LOD_COUNT || triangles >= DENSE_LOD_TRIANGLES {
            hot_spots.push((nearby.len(), triangles, center, nearby));
        }
    }
    hot_spots.sort_by(|a, b| (b.0, b.1).cmp(&(a.0, a.1)));
    let mut accepted = Vec::<V3>::new();
    for (count, triangles, center, nearby) in hot_spots {
        if accepted
            .iter()
            .any(|other| v3_distance_squared(*other, center) < (DENSITY_RADIUS * 0.75).powi(2))
        {
            continue;
        }
        accepted.push(center);
        let (min, max) =
            union_snapshot_bounds(nearby.iter(), &snapshots).unwrap_or((center, center));
        issues.push(LodAuditIssue {
            kind: LodAuditIssueKind::Dense,
            title: format!("{count} LOD chunks in one view neighborhood"),
            detail: format!(
                "{triangles} LOD triangles within {:.0} units. Review batching, grouping, and draw distances.",
                DENSITY_RADIUS
            ),
            position: center,
            placement_indices: nearby,
            min,
            max,
            size: max_extent(min, max),
        });
    }

    issues.sort_by(|a, b| {
        let rank = |kind| match kind {
            LodAuditIssueKind::Missing => 0,
            LodAuditIssueKind::Dense => 1,
            LodAuditIssueKind::Small => 2,
        };
        rank(a.kind)
            .cmp(&rank(b.kind))
            .then_with(|| a.title.cmp(&b.title))
    });

    let _ = progress_tx.send(LodAuditUpdate::Progress {
        revision,
        scanned: total,
        total,
    });
    LodAuditResult {
        placements,
        issues,
        loader_distance_multiplier: distance_multiplier,
        loader_config_found: config_found,
    }
}

fn snapshot_lod_audit(app: &AppState) -> Vec<LodAuditSnapshot> {
    const OBJECT_ONLY_ATTRS: &[&str] = &[
        "breakable",
        "frozen",
        "streamable",
        "scale",
        "simulated",
        "physicsRoot",
        "physics_root",
        "mass",
        "turnMass",
        "turn_mass",
        "airResistance",
        "air_resistance",
        "elasticity",
        "buoyancy",
        "respawn",
    ];
    app.placements
        .iter()
        .enumerate()
        .map(|(index, placement)| {
            let mesh = element_mesh(app, placement);
            let bounds = mesh
                .map(|mesh| {
                    transformed_bounds(mesh.bounds, &placement_matrix(placement).to_cols_array())
                })
                .unwrap_or(Bounds {
                    min: to_mq(placement.pos),
                    max: to_mq(placement.pos),
                });
            let center = (bounds.min + bounds.max) * 0.5;
            let radius = (bounds.max - center).length();
            let triangles = mesh
                .map(|mesh| {
                    mesh.parts
                        .iter()
                        .map(|part| part.cpu_vertices.len() / 3)
                        .sum()
                })
                .unwrap_or(0);
            let definition_lod_distance = app
                .definitions
                .get(&placement.id)
                .and_then(|definition| finite_attr_f32(definition.attrs.get("lodDistance")));
            let definition_has_object_physics =
                app.definitions
                    .get(&placement.id)
                    .is_some_and(|definition| {
                        OBJECT_ONLY_ATTRS.iter().any(|key| {
                            definition
                                .attrs
                                .get(*key)
                                .is_some_and(|value| !value.trim().is_empty())
                        })
                    });
            let placement_has_object_override = OBJECT_ONLY_ATTRS.iter().any(|key| {
                placement
                    .attrs
                    .get(*key)
                    .is_some_and(|value| !value.trim().is_empty())
            });
            let dynamic_or_forced = attr_bool(
                &placement.attrs,
                &["dynamic", "forceObject", "force_object"],
                false,
            );
            let dimension = finite_attr_f32(placement.attrs.get("dimension")).unwrap_or(0.0);
            let tram_attached = placement.id.eq_ignore_ascii_case("Tram")
                || placement
                    .attrs
                    .get("lodParent")
                    .is_some_and(|parent| parent.trim().eq_ignore_ascii_case("Tram"));
            LodAuditSnapshot {
                index,
                id: placement.id.trim().to_string(),
                tag: placement.tag.trim().to_string(),
                lod_parent: placement
                    .attrs
                    .get("lodParent")
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty()),
                unique_id: placement
                    .attrs
                    .get("uniqueID")
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty()),
                definition_lod_distance,
                center: from_mq(center),
                min: from_mq(bounds.min),
                max: from_mq(bounds.max),
                radius,
                triangles,
                live: is_live_element(app, index),
                can_promote_to_building: placement.tag.eq_ignore_ascii_case("object")
                    && dimension == 0.0
                    && !dynamic_or_forced
                    && !tram_attached
                    && !definition_has_object_physics
                    && !placement_has_object_override,
            }
        })
        .collect()
}

pub(crate) fn invalidate_lod_audit(app: &mut AppState) {
    app.lod_audit.revision = app.lod_audit.revision.wrapping_add(1);
    app.lod_audit.stale = true;
    app.lod_audit.selected_issue = None;
}

pub(crate) fn request_lod_audit(app: &mut AppState) {
    if app.lod_audit.rx.is_some() {
        app.status_message = "LOD audit is already running".to_string();
        return;
    }
    let snapshots = snapshot_lod_audit(app);
    let total = snapshots.len();
    let revision = app.lod_audit.revision;
    let small_threshold = app.lod_audit.small_threshold;
    let (tx, rx) = mpsc::channel();
    let worker_tx = tx.clone();
    let spawn = thread::Builder::new()
        .name("lod-audit".to_string())
        .spawn(move || {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                lod_audit_analysis(snapshots, revision, small_threshold, &worker_tx)
            }));
            let update = match outcome {
                Ok(result) => LodAuditUpdate::Complete { revision, result },
                Err(_) => LodAuditUpdate::Failed {
                    revision,
                    message: "LOD audit worker crashed".to_string(),
                },
            };
            let _ = worker_tx.send(update);
        });
    match spawn {
        Ok(_) => {
            app.lod_audit.rx = Some(rx);
            app.lod_audit.progress = (0, total);
            app.status_message = format!("Auditing {total} placements for LOD coverage...");
        }
        Err(error) => {
            app.status_message = format!("Could not start LOD audit: {error}");
        }
    }
}

pub(crate) fn poll_lod_audit(app: &mut AppState) {
    let mut updates = Vec::new();
    let mut disconnected = false;
    if let Some(rx) = app.lod_audit.rx.as_ref() {
        loop {
            match rx.try_recv() {
                Ok(update) => updates.push(update),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    disconnected = true;
                    break;
                }
            }
        }
    }
    let mut finished = false;
    for update in updates {
        match update {
            LodAuditUpdate::Progress {
                revision,
                scanned,
                total,
            } if revision == app.lod_audit.revision => {
                app.lod_audit.progress = (scanned, total);
                app.status_message = format!("LOD audit: {scanned}/{total} placements...");
            }
            LodAuditUpdate::Complete { revision, result } if revision == app.lod_audit.revision => {
                app.lod_audit.result = Some(result);
                let missing = lod_audit_issue_count(app, LodAuditIssueKind::Missing);
                let dense = lod_audit_issue_count(app, LodAuditIssueKind::Dense);
                let small = lod_audit_issue_count(app, LodAuditIssueKind::Small);
                app.lod_audit.stale = false;
                app.lod_audit.selected_issue = None;
                app.properties_scroll = 0.0;
                app.status_message = format!(
                    "LOD audit complete: {missing} coverage, {dense} dense, {small} small review item(s)"
                );
                finished = true;
            }
            LodAuditUpdate::Failed { revision, message } if revision == app.lod_audit.revision => {
                app.status_message = message;
                finished = true;
            }
            LodAuditUpdate::Progress { .. }
            | LodAuditUpdate::Complete { .. }
            | LodAuditUpdate::Failed { .. } => {
                // The scene changed while the worker was running. Discard its
                // old snapshot rather than publishing misleading results.
                finished = true;
            }
        }
    }
    if disconnected {
        finished = true;
    }
    if finished {
        app.lod_audit.rx = None;
    }
}

pub(crate) fn lod_audit_high_distance(app: &AppState, info: &LodAuditPlacementInfo) -> f32 {
    info.model_distance * (1.0 + app.lod_audit.client_draw_percent as f32 / 100.0)
}

fn detail_partially_visible(distance: f32, radius: f32, high_distance: f32) -> bool {
    distance - radius <= high_distance
}

fn detail_fully_visible(distance: f32, radius: f32, high_distance: f32) -> bool {
    distance + radius <= high_distance
}

pub(crate) fn lod_audit_placement_visible(
    app: &AppState,
    result: &LodAuditResult,
    index: usize,
    camera: Vec3,
) -> bool {
    let Some(info) = result.placements.get(index) else {
        return false;
    };
    let center = to_mq(info.center);
    let distance = (center - camera).length();
    match &info.role {
        LodAuditRole::Detail | LodAuditRole::Child { .. } => {
            detail_partially_visible(distance, info.radius, lod_audit_high_distance(app, info))
        }
        LodAuditRole::SelfLod => distance - info.radius <= info.low_distance,
        LodAuditRole::LodTarget { children } => {
            if info.low_distance.is_infinite() {
                return true;
            }
            let in_low_range = distance - info.radius <= info.low_distance;
            if !in_low_range {
                return false;
            }
            children.is_empty()
                || children.iter().any(|child| {
                    let Some(child_info) = result.placements.get(*child) else {
                        return true;
                    };
                    let child_drawable = app.placements.get(*child).is_some_and(|placement| {
                        is_visible_element(app, *child) && element_mesh(app, placement).is_some()
                    });
                    if !child_drawable {
                        // Never hide the only visible stand-in when its
                        // high-detail replacement cannot be rendered.
                        return true;
                    }
                    let child_distance = (to_mq(child_info.center) - camera).length();
                    !detail_fully_visible(
                        child_distance,
                        child_info.radius,
                        lod_audit_high_distance(app, child_info),
                    )
                })
        }
    }
}

pub(crate) fn filtered_lod_audit_issue_indices(app: &AppState) -> Vec<usize> {
    let Some(result) = app.lod_audit.result.as_ref() else {
        return Vec::new();
    };
    let mut indices = result
        .issues
        .iter()
        .enumerate()
        .filter_map(|(index, issue)| {
            let ignored = lod_audit_issue_is_ignored(app, issue);
            let matches = match app.lod_audit.filter {
                LodAuditFilter::All => !ignored,
                LodAuditFilter::Missing => issue.kind == LodAuditIssueKind::Missing && !ignored,
                LodAuditFilter::Dense => issue.kind == LodAuditIssueKind::Dense,
                LodAuditFilter::Small => issue.kind == LodAuditIssueKind::Small,
                LodAuditFilter::Ignored => ignored,
            };
            matches.then_some(index)
        })
        .collect::<Vec<_>>();
    if app.lod_audit.filter == LodAuditFilter::Small {
        indices.sort_by(|a, b| {
            let ordering = result.issues[*a].size.total_cmp(&result.issues[*b].size);
            let ordering = if app.lod_audit.small_sort_ascending {
                ordering
            } else {
                ordering.reverse()
            };
            ordering.then_with(|| result.issues[*a].title.cmp(&result.issues[*b].title))
        });
    }
    indices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loader_distance_matches_canonical_promotion_rule() {
        assert_eq!(loader_distance(None, 1.0), 200.0);
        assert_eq!(loader_distance(Some(9.0), 1.0), 700.0);
        assert_eq!(loader_distance(Some(10.0), 1.5), 15.0);
    }

    #[test]
    fn canonical_loader_settings_use_safe_defaults_without_a_config() {
        let settings = canonical_loader_settings();
        assert_eq!(settings.distance_multiplier, 1.0);
        assert!(settings.prefer_static_buildings);
        assert!(!settings.force_object);
    }

    #[test]
    fn detail_and_lod_handoff_has_no_distance_gap() {
        let high_distance = 200.0;
        let radius = 5.0;
        for step in 0..=500 {
            let distance = step as f32;
            let detail = detail_partially_visible(distance, radius, high_distance);
            let lod = !detail_fully_visible(distance, radius, high_distance);
            assert!(
                detail || lod,
                "handoff gap at distance {distance} for radius {radius}"
            );
        }
    }

    fn coverage_issue_at(x: f32) -> LodAuditIssue {
        LodAuditIssue {
            kind: LodAuditIssueKind::Missing,
            title: "tower has no LOD coverage".to_string(),
            detail: String::new(),
            position: V3 { x, y: 2.0, z: 3.0 },
            placement_indices: vec![0],
            min: V3 {
                x: x - 1.0,
                y: 1.0,
                z: 2.0,
            },
            max: V3 {
                x: x + 1.0,
                y: 3.0,
                z: 4.0,
            },
            size: 2.0,
        }
    }

    #[test]
    fn coverage_ignore_keys_are_stable_and_location_specific() {
        let original = coverage_issue_at(10.0);
        let rerun = original.clone();
        let other_instance = coverage_issue_at(11.0);
        assert_eq!(
            lod_audit_coverage_ignore_key(&original),
            lod_audit_coverage_ignore_key(&rerun)
        );
        assert_ne!(
            lod_audit_coverage_ignore_key(&original),
            lod_audit_coverage_ignore_key(&other_instance)
        );
    }

    #[test]
    fn only_coverage_issues_have_ignore_keys() {
        let mut issue = coverage_issue_at(10.0);
        issue.kind = LodAuditIssueKind::Small;
        assert_eq!(lod_audit_coverage_ignore_key(&issue), None);
    }
}
