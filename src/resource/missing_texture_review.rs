use super::super::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum MissingTextureReviewFilter {
    #[default]
    All,
    Detail,
    Lod,
}

impl MissingTextureReviewFilter {
    pub(crate) const ALL: [Self; 3] = [Self::All, Self::Detail, Self::Lod];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Detail => "Detail",
            Self::Lod => "LODs",
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct MissingTextureReviewItem {
    pub(crate) placement_index: usize,
    pub(crate) id: String,
    pub(crate) dff: String,
    pub(crate) txd: String,
    pub(crate) textures: Vec<String>,
    pub(crate) is_lod: bool,
    pub(crate) position: V3,
    pub(crate) min: V3,
    pub(crate) max: V3,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct MissingTextureReviewResult {
    pub(crate) items: Vec<MissingTextureReviewItem>,
    pub(crate) scanned_models: usize,
}

pub(crate) struct MissingTextureReviewState {
    pub(crate) result: Option<MissingTextureReviewResult>,
    pub(crate) rx: Option<mpsc::Receiver<MissingTextureReviewUpdate>>,
    pub(crate) revision: u64,
    pub(crate) stale: bool,
    pub(crate) progress: (usize, usize),
    pub(crate) filter: MissingTextureReviewFilter,
    pub(crate) selected_item: Option<usize>,
    pub(crate) list_scroll: f32,
    pub(crate) texture_scroll: f32,
}

impl Default for MissingTextureReviewState {
    fn default() -> Self {
        Self {
            result: None,
            rx: None,
            revision: 0,
            stale: true,
            progress: (0, 0),
            filter: MissingTextureReviewFilter::All,
            selected_item: None,
            list_scroll: 0.0,
            texture_scroll: 0.0,
        }
    }
}

#[derive(Clone, Debug)]
struct MissingTextureReference {
    name: String,
    txd_scope: Option<String>,
}

#[derive(Clone, Debug)]
struct MissingTextureReviewSnapshot {
    placement_index: usize,
    id: String,
    dff: String,
    txd: String,
    textures: Vec<MissingTextureReference>,
    is_lod: bool,
    position: V3,
    min: V3,
    max: V3,
    live: bool,
}

fn definition_keeps_default_sa_dff(definition: &Definition) -> bool {
    let from_sa = definition
        .attrs
        .get("source")
        .is_some_and(|source| source.trim().eq_ignore_ascii_case("GTA:SA"));
    if !from_sa {
        return false;
    }
    !definition
        .attrs
        .get("__overrideAttrs")
        .is_some_and(|attrs| {
            attrs
                .split(',')
                .any(|key| key.trim().eq_ignore_ascii_case("dff"))
        })
}

fn default_sa_dff_keys(app: &AppState) -> HashSet<String> {
    app.definitions
        .values()
        .filter(|definition| {
            app.readonly_definition_ids.contains(&definition.id)
                || definition_keeps_default_sa_dff(definition)
        })
        .map(|definition| asset_key_opt(definition.attrs.get("dff"), &definition.id, ".dff"))
        .collect()
}

fn placement_uses_default_sa_model(
    app: &AppState,
    placement: &Placement,
    default_sa_dffs: &HashSet<String>,
) -> bool {
    app.readonly_definition_ids.contains(&placement.id)
        || default_sa_dffs.contains(&asset_key(&placement.dff, ".dff"))
}

#[derive(Debug)]
pub(crate) enum MissingTextureReviewUpdate {
    Progress {
        revision: u64,
        scanned: usize,
        total: usize,
    },
    Complete {
        revision: u64,
        result: MissingTextureReviewResult,
    },
    Failed {
        revision: u64,
        message: String,
    },
}

fn snapshot_missing_texture_review(
    app: &AppState,
) -> (
    Vec<MissingTextureReviewSnapshot>,
    HashSet<String>,
    HashSet<(String, String)>,
) {
    let mut available_any = HashSet::new();
    let mut available_scoped = HashSet::new();
    available_any.extend(app.texture_files.keys().map(|name| lower(name)));
    for (name, entries) in &app.txd_textures {
        let name = lower(name);
        let mut available_in_txd = false;
        for entry in entries {
            let txd = asset_key(&entry.txd_name, ".txd");
            if !app.pending_asset_deletes.contains(&txd) {
                available_in_txd = true;
                available_scoped.insert((txd, name.clone()));
            }
        }
        if available_in_txd {
            available_any.insert(name);
        }
    }

    let default_sa_dffs = default_sa_dff_keys(app);

    let snapshots = app
        .placements
        .iter()
        .enumerate()
        .filter(|(_, placement)| !placement_uses_default_sa_model(app, placement, &default_sa_dffs))
        .map(|(placement_index, placement)| {
            let txd = definition_txd_name(&app.definitions, &placement.id)
                .map(|name| asset_key(name, ".txd"))
                .unwrap_or_default();
            let mut seen = HashSet::new();
            let textures = element_mesh(app, placement)
                .map(|mesh| {
                    mesh.parts
                        .iter()
                        .filter_map(|part| {
                            let name = lower(part.texture_name.trim());
                            if name.is_empty() {
                                return None;
                            }
                            let txd_scope = app
                                .texture_overrides
                                .get(&(placement.id.clone(), name.clone()))
                                .map(|scope| asset_key(scope, ".txd"))
                                .or_else(|| (!txd.is_empty()).then(|| txd.clone()));
                            if !seen.insert((name.clone(), txd_scope.clone())) {
                                return None;
                            }
                            Some(MissingTextureReference { name, txd_scope })
                        })
                        .collect()
                })
                .unwrap_or_default();
            let bounds = element_mesh(app, placement)
                .map(|mesh| {
                    transformed_bounds(mesh.bounds, &placement_matrix(placement).to_cols_array())
                })
                .unwrap_or(Bounds {
                    min: to_mq(placement.pos),
                    max: to_mq(placement.pos),
                });
            MissingTextureReviewSnapshot {
                placement_index,
                id: placement.id.trim().to_string(),
                dff: asset_key(&placement.dff, ".dff"),
                txd,
                textures,
                is_lod: placement_is_lod(placement, &app.lod_ids),
                position: from_mq((bounds.min + bounds.max) * 0.5),
                min: from_mq(bounds.min),
                max: from_mq(bounds.max),
                live: is_live_element(app, placement_index),
            }
        })
        .collect();
    (snapshots, available_any, available_scoped)
}

fn analyze_missing_textures(
    snapshots: Vec<MissingTextureReviewSnapshot>,
    available_any: HashSet<String>,
    available_scoped: HashSet<(String, String)>,
    revision: u64,
    progress_tx: &mpsc::Sender<MissingTextureReviewUpdate>,
) -> MissingTextureReviewResult {
    let total = snapshots.len();
    let mut items = Vec::new();
    for (scanned, snapshot) in snapshots.into_iter().enumerate() {
        if snapshot.live {
            let mut missing = snapshot
                .textures
                .iter()
                .filter(|texture| match texture.txd_scope.as_ref() {
                    Some(scope) => {
                        !available_scoped.contains(&(scope.clone(), texture.name.clone()))
                    }
                    None => !available_any.contains(&texture.name),
                })
                .map(|texture| texture.name.clone())
                .collect::<Vec<_>>();
            missing.sort();
            missing.dedup();
            if !missing.is_empty() {
                items.push(MissingTextureReviewItem {
                    placement_index: snapshot.placement_index,
                    id: snapshot.id,
                    dff: snapshot.dff,
                    txd: snapshot.txd,
                    textures: missing,
                    is_lod: snapshot.is_lod,
                    position: snapshot.position,
                    min: snapshot.min,
                    max: snapshot.max,
                });
            }
        }
        if scanned % 128 == 127 || scanned + 1 == total {
            let _ = progress_tx.send(MissingTextureReviewUpdate::Progress {
                revision,
                scanned: scanned + 1,
                total,
            });
        }
    }
    items.sort_by(|a, b| {
        (
            a.is_lod,
            a.id.to_ascii_lowercase(),
            &a.dff,
            a.placement_index,
        )
            .cmp(&(
                b.is_lod,
                b.id.to_ascii_lowercase(),
                &b.dff,
                b.placement_index,
            ))
    });
    MissingTextureReviewResult {
        items,
        scanned_models: total,
    }
}

pub(crate) fn invalidate_missing_texture_review(app: &mut AppState) {
    app.missing_texture_review.revision = app.missing_texture_review.revision.wrapping_add(1);
    app.missing_texture_review.stale = true;
    app.missing_texture_review.selected_item = None;
    app.missing_texture_review.texture_scroll = 0.0;
}

pub(crate) fn request_missing_texture_review(app: &mut AppState) {
    if app.missing_texture_review.rx.is_some() {
        app.status_message = "Missing-texture review is already running.".to_string();
        return;
    }
    let (snapshots, available_any, available_scoped) = snapshot_missing_texture_review(app);
    let total = snapshots.len();
    let revision = app.missing_texture_review.revision;
    let (tx, rx) = mpsc::channel();
    let worker_tx = tx.clone();
    let spawn = thread::Builder::new()
        .name("missing-texture-review".to_string())
        .spawn(move || {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                analyze_missing_textures(
                    snapshots,
                    available_any,
                    available_scoped,
                    revision,
                    &worker_tx,
                )
            }));
            let update = match outcome {
                Ok(result) => MissingTextureReviewUpdate::Complete { revision, result },
                Err(_) => MissingTextureReviewUpdate::Failed {
                    revision,
                    message: "Missing-texture review worker crashed".to_string(),
                },
            };
            let _ = worker_tx.send(update);
        });
    match spawn {
        Ok(_) => {
            app.missing_texture_review.rx = Some(rx);
            app.missing_texture_review.progress = (0, total);
            app.status_message = format!("Reviewing textures for {total} scene models...");
        }
        Err(error) => {
            app.status_message = format!("Could not start missing-texture review: {error}");
        }
    }
}

pub(crate) fn poll_missing_texture_review(app: &mut AppState) {
    let mut updates = Vec::new();
    let mut disconnected = false;
    if let Some(rx) = app.missing_texture_review.rx.as_ref() {
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
            MissingTextureReviewUpdate::Progress {
                revision,
                scanned,
                total,
            } if revision == app.missing_texture_review.revision => {
                app.missing_texture_review.progress = (scanned, total);
                app.status_message =
                    format!("Missing-texture review: {scanned}/{total} scene models...");
            }
            MissingTextureReviewUpdate::Complete { revision, result }
                if revision == app.missing_texture_review.revision =>
            {
                let affected = result.items.len();
                let lods = result.items.iter().filter(|item| item.is_lod).count();
                app.missing_texture_review.result = Some(result);
                app.missing_texture_review.stale = false;
                app.missing_texture_review.selected_item = None;
                app.missing_texture_review.list_scroll = 0.0;
                app.missing_texture_review.texture_scroll = 0.0;
                app.status_message = format!(
                    "Missing-texture review complete: {affected} model(s) flagged, including {lods} LOD(s)."
                );
                finished = true;
            }
            MissingTextureReviewUpdate::Failed { revision, message }
                if revision == app.missing_texture_review.revision =>
            {
                app.status_message = message;
                finished = true;
            }
            MissingTextureReviewUpdate::Progress { .. }
            | MissingTextureReviewUpdate::Complete { .. }
            | MissingTextureReviewUpdate::Failed { .. } => {
                // Scene or texture state changed while the worker was using its
                // snapshot. Discard the result instead of publishing stale flags.
                finished = true;
            }
        }
    }
    if disconnected {
        finished = true;
    }
    if finished {
        app.missing_texture_review.rx = None;
    }
}

pub(crate) fn filtered_missing_texture_review_indices(app: &AppState) -> Vec<usize> {
    app.missing_texture_review
        .result
        .as_ref()
        .map(|result| {
            result
                .items
                .iter()
                .enumerate()
                .filter_map(|(index, item)| {
                    let visible = match app.missing_texture_review.filter {
                        MissingTextureReviewFilter::All => true,
                        MissingTextureReviewFilter::Detail => !item.is_lod,
                        MissingTextureReviewFilter::Lod => item.is_lod,
                    };
                    visible.then_some(index)
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analysis_reports_each_placement_and_includes_lods() {
        let snapshot = |placement_index, is_lod, texture: &str, scope: Option<&str>| {
            MissingTextureReviewSnapshot {
                placement_index,
                id: format!("model_{placement_index}"),
                dff: format!("model_{placement_index}.dff"),
                txd: scope.unwrap_or_default().to_string(),
                textures: vec![MissingTextureReference {
                    name: texture.to_string(),
                    txd_scope: scope.map(str::to_string),
                }],
                is_lod,
                position: V3::default(),
                min: V3::default(),
                max: V3::default(),
                live: true,
            }
        };
        let (tx, _rx) = mpsc::channel();
        let result = analyze_missing_textures(
            vec![
                snapshot(0, false, "missing_detail", Some("detail.txd")),
                snapshot(1, true, "missing_lod", Some("lod.txd")),
                snapshot(2, false, "present", Some("detail.txd")),
            ],
            HashSet::new(),
            HashSet::from([("detail.txd".to_string(), "present".to_string())]),
            0,
            &tx,
        );
        assert_eq!(result.scanned_models, 3);
        assert_eq!(result.items.len(), 2);
        assert!(result.items.iter().any(|item| item.placement_index == 0));
        assert!(
            result
                .items
                .iter()
                .any(|item| item.placement_index == 1 && item.is_lod)
        );
    }

    #[test]
    fn gta_definition_is_default_until_its_dff_is_explicitly_replaced() {
        let definition = |override_attrs: Option<&str>| {
            let mut attrs = BTreeMap::from([
                ("source".to_string(), "GTA:SA".to_string()),
                ("dff".to_string(), "stock_model".to_string()),
            ]);
            if let Some(override_attrs) = override_attrs {
                attrs.insert("__overrideAttrs".to_string(), override_attrs.to_string());
            }
            Definition {
                id: "123".to_string(),
                zone: "project".to_string(),
                attrs,
            }
        };

        assert!(definition_keeps_default_sa_dff(&definition(None)));
        assert!(definition_keeps_default_sa_dff(&definition(Some(
            "lodDistance,txd"
        ))));
        assert!(!definition_keeps_default_sa_dff(&definition(Some(
            "lodDistance,dff"
        ))));
    }
}
