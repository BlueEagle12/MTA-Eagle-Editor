//! Definition ownership follows the distribution of live Eagle placements.
use super::super::*;
use crate::resource::mta_maps::map_path;

/// Standard MTA maps cannot own Eagle definitions. Hidden instances still count;
/// deleted instances do not. Prefer the current owner on ties, then zone name.
pub(crate) fn rehome_definitions(
    definitions: &mut HashMap<String, Definition>,
    readonly_ids: &HashSet<String>,
    placements: &[Placement],
    states: &[ElementState],
) -> bool {
    let mut counts: HashMap<&str, HashMap<&str, usize>> = HashMap::new();
    for (index, placement) in placements.iter().enumerate() {
        if readonly_ids.contains(&placement.id)
            || states.get(index).is_some_and(|s| s.deleted)
            || map_path(&placement.zone).is_some()
        {
            continue;
        }
        *counts
            .entry(&placement.id)
            .or_default()
            .entry(&placement.zone)
            .or_default() += 1;
    }
    let mut changed = false;
    for definition in definitions.values_mut() {
        if readonly_ids.contains(&definition.id) {
            continue;
        }
        let Some(zones) = counts.get(definition.id.as_str()) else {
            continue;
        };
        let most = zones.values().copied().max().unwrap_or(0);
        let owner = if zones.get(definition.zone.as_str()) == Some(&most) {
            definition.zone.as_str()
        } else {
            zones
                .iter()
                .filter(|(_, count)| **count == most)
                .map(|(zone, _)| *zone)
                .min()
                .unwrap()
        };
        if owner != definition.zone {
            definition.zone = owner.to_owned();
            definition
                .attrs
                .insert("zone".into(), definition.zone.clone());
            changed = true;
        }
    }
    changed
}

pub(crate) fn reconcile_definition_zones(app: &mut AppState) {
    if rehome_definitions(
        &mut app.definitions,
        &app.readonly_definition_ids,
        &app.placements,
        &app.element_states,
    ) {
        app.asset_browser.entries_cache_fingerprint = None;
        invalidate_validation_cache(app);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn definition(zone: &str) -> Definition {
        Definition {
            id: "model".into(),
            zone: zone.into(),
            attrs: BTreeMap::from([("zone".into(), zone.into())]),
        }
    }
    fn placement(zone: &str) -> Placement {
        Placement {
            id: "model".into(),
            dff: "model".into(),
            zone: zone.into(),
            tag: "object".into(),
            attrs: BTreeMap::new(),
            pos: V3::default(),
            rot: V3::default(),
        }
    }
    #[test]
    fn majority_counts_hidden_but_excludes_deleted_and_standard_maps() {
        let mut defs = HashMap::from([("model".into(), definition("a"))]);
        let placements = vec![
            placement("a"),
            placement("b"),
            placement("b"),
            placement("a"),
            placement("map:main.map"),
        ];
        let states = vec![
            ElementState::default(),
            ElementState::default(),
            ElementState {
                hidden: true,
                deleted: false,
            },
            ElementState {
                deleted: true,
                hidden: false,
            },
            ElementState::default(),
        ];
        assert!(rehome_definitions(
            &mut defs,
            &HashSet::new(),
            &placements,
            &states
        ));
        assert_eq!(defs["model"].zone, "b");
        assert_eq!(defs["model"].attrs["zone"], "b");
    }
    #[test]
    fn ties_keep_current_owner_or_choose_alphabetically() {
        let ps = vec![placement("b"), placement("a")];
        let mut defs = HashMap::from([("model".into(), definition("b"))]);
        assert!(!rehome_definitions(&mut defs, &HashSet::new(), &ps, &[]));
        defs.get_mut("model").unwrap().zone = "unused".into();
        rehome_definitions(&mut defs, &HashSet::new(), &ps, &[]);
        assert_eq!(defs["model"].zone, "a");
    }
    #[test]
    fn unused_and_readonly_definitions_keep_their_owner() {
        let mut defs = HashMap::from([("model".into(), definition("a"))]);
        assert!(!rehome_definitions(&mut defs, &HashSet::new(), &[], &[]));
        assert!(!rehome_definitions(
            &mut defs,
            &HashSet::from(["model".into()]),
            &[placement("b")],
            &[]
        ));
        assert_eq!(defs["model"].zone, "a");
    }
}
