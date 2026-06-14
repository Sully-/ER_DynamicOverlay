use std::collections::{HashMap, HashSet};

use er_game_state::{
    bosses_in_region, good_by_key, group_names, group_progress, group_size, item_owned,
    region_label_for_subregion, region_names, GameStateSource,
};
use er_overlay_common::{
    BossPanelScope, ChallengeSnapshot, GameStateDiagnostics, GameTime, TrackKind,
};

#[derive(Debug, Clone)]
pub struct TrackedEntryRow {
    pub name: String,
    pub kind: TrackKind,
    pub icon_key: String,
    /// Optional display cap for a countable good (e.g. scadutree → "N/50").
    pub max: Option<u32>,
    /// Present when the layout requests equipped tracking for this good key.
    pub equipped: Option<bool>,
}

/// Owned / total members of an aggregate group (e.g. great runes).
#[derive(Debug, Clone, Copy)]
pub struct GroupValue {
    pub owned: Option<u32>,
    pub total: u32,
}

#[derive(Debug, Clone)]
pub struct BossPanelRow {
    pub name: String,
    pub place: Option<String>,
    pub killed: Option<bool>,
    pub dlc: bool,
}

#[derive(Debug, Clone)]
pub struct BossPanelSection {
    pub region: String,
    pub is_current: bool,
    pub bosses: Vec<BossPanelRow>,
    pub killed: u32,
    pub total: u32,
}

#[derive(Debug, Clone)]
pub struct OverlayViewModel {
    pub igt: Option<GameTime>,
    pub bosses_killed: Option<u32>,
    pub bosses_total: u32,
    pub deaths: Option<u32>,
    pub ng_cycle: Option<u32>,
    pub scadutree_blessing: Option<u32>,
    pub groups: HashMap<String, GroupValue>,
    pub tracked_by_key: HashMap<String, TrackedEntryRow>,
    pub diagnostics: GameStateDiagnostics,
    pub current_subregion_id: Option<u32>,
    pub current_region: Option<String>,
    pub boss_panel_scope: BossPanelScope,
    pub boss_panel_sections: Vec<BossPanelSection>,
    pub boss_panel_killed: u32,
    pub boss_panel_total: u32,
    pub challenge: ChallengeSnapshot,
}

impl OverlayViewModel {
    pub fn tracked(&self, key: &str) -> Option<&TrackedEntryRow> {
        self.tracked_by_key.get(key)
    }

    pub fn group(&self, name: &str) -> Option<&GroupValue> {
        self.groups.get(name)
    }
}

fn boss_rows_for_region(
    source: &dyn GameStateSource,
    region: &str,
) -> (Vec<BossPanelRow>, u32, u32) {
    let mut rows = Vec::new();
    let mut killed = 0u32;
    for boss in bosses_in_region(region) {
        let is_killed = source.get_flag(boss.flag_id);
        if is_killed == Some(true) {
            killed += 1;
        }
        rows.push(BossPanelRow {
            name: boss.name.clone(),
            place: boss.place.clone(),
            killed: is_killed,
            dlc: boss.dlc,
        });
    }
    let total = rows.len() as u32;
    (rows, killed, total)
}

fn build_boss_panel_sections(
    source: &dyn GameStateSource,
    scope: BossPanelScope,
    current_region: Option<&str>,
) -> (Vec<BossPanelSection>, u32, u32) {
    match scope {
        BossPanelScope::CurrentRegion => {
            if let Some(region) = current_region {
                let (bosses, killed, total) = boss_rows_for_region(source, region);
                let section = BossPanelSection {
                    region: region.to_string(),
                    is_current: true,
                    bosses,
                    killed,
                    total,
                };
                (vec![section], killed, total)
            } else {
                // Unknown map id — fall back to the full checklist instead of an empty panel.
                build_all_region_sections(source, current_region)
            }
        }
        BossPanelScope::AllRegions => build_all_region_sections(source, current_region),
    }
}

fn build_all_region_sections(
    source: &dyn GameStateSource,
    current_region: Option<&str>,
) -> (Vec<BossPanelSection>, u32, u32) {
    let mut sections = Vec::new();
    let mut killed_total = 0u32;
    let mut boss_total = 0u32;

    for region in region_names() {
        let (bosses, killed, total) = boss_rows_for_region(source, &region);
        if bosses.is_empty() {
            continue;
        }
        killed_total += killed;
        boss_total += total;
        sections.push(BossPanelSection {
            region: region.clone(),
            is_current: matches!(current_region, Some(r) if r == region),
            bosses,
            killed,
            total,
        });
    }

    (sections, killed_total, boss_total)
}

pub fn empty_view_model(boss_panel_scope: BossPanelScope) -> OverlayViewModel {
    let bosses_total = er_game_state::bosses_total_count() as u32;
    OverlayViewModel {
        igt: None,
        bosses_killed: None,
        bosses_total,
        deaths: None,
        ng_cycle: None,
        scadutree_blessing: None,
        groups: HashMap::new(),
        tracked_by_key: HashMap::new(),
        diagnostics: GameStateDiagnostics::default(),
        current_subregion_id: None,
        current_region: None,
        boss_panel_scope,
        boss_panel_sections: Vec::new(),
        boss_panel_killed: 0,
        boss_panel_total: bosses_total,
        challenge: ChallengeSnapshot::default(),
    }
}

/// Builds the view model. `referenced_keys` are the good keys / metric refs used by the active
/// layout (from `LayoutConfig::collect_data_refs`); only those goods are resolved against the
/// inventory. Aggregate groups are always resolved (there are few, with few members).
pub fn build_view_model(
    source: &dyn GameStateSource,
    referenced_keys: &[String],
    equipped_keys: &HashSet<String>,
    boss_panel_scope: BossPanelScope,
    challenge: ChallengeSnapshot,
) -> OverlayViewModel {
    let mut tracked_by_key = HashMap::new();
    for key in referenced_keys {
        let Some(good) = good_by_key(key) else {
            continue;
        };
        let kind = if good.countable {
            TrackKind::Countable {
                count: source.get_goods_quantity(good.item_id),
            }
        } else {
            TrackKind::Unique {
                acquired: item_owned(source, good.item_id, good.category, good.pickup_flag),
            }
        };
        let equipped = if equipped_keys.contains(key) {
            source.is_item_equipped(good.item_id, good.category)
        } else {
            None
        };
        tracked_by_key.insert(
            good.key.clone(),
            TrackedEntryRow {
                name: good.name,
                kind,
                icon_key: good.key,
                max: good.max,
                equipped,
            },
        );
    }

    let mut groups = HashMap::new();
    for name in group_names() {
        let total = group_size(&name);
        let owned = group_progress(source, &name).map(|(owned, _)| owned);
        groups.insert(name, GroupValue { owned, total });
    }

    let current_subregion_id = source.get_current_subregion_id();
    let current_region = current_subregion_id.and_then(region_label_for_subregion);

    let (boss_panel_sections, boss_panel_killed, boss_panel_total) =
        build_boss_panel_sections(source, boss_panel_scope, current_region.as_deref());

    OverlayViewModel {
        igt: source.get_igt(),
        bosses_killed: source.get_killed_boss_count(),
        bosses_total: source.bosses_total(),
        deaths: source.get_death_count(),
        ng_cycle: source.get_ng_cycle(),
        scadutree_blessing: source.get_scadutree_blessing(),
        groups,
        tracked_by_key,
        diagnostics: source.get_status(),
        current_subregion_id,
        current_region,
        boss_panel_scope,
        boss_panel_sections,
        boss_panel_killed,
        boss_panel_total,
        challenge,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use er_game_state::mock::MockGameState;

    #[test]
    fn current_region_scope_filters_bosses() {
        let mock = MockGameState::default();
        let vm = build_view_model(
            &mock,
            &[],
            &HashSet::new(),
            BossPanelScope::CurrentRegion,
            ChallengeSnapshot::default(),
        );
        assert_eq!(vm.boss_panel_sections.len(), 1);
        assert_eq!(vm.boss_panel_sections[0].region, "Limgrave");
    }

    #[test]
    fn all_regions_scope_lists_every_region() {
        let mock = MockGameState::default();
        let vm = build_view_model(
            &mock,
            &[],
            &HashSet::new(),
            BossPanelScope::AllRegions,
            ChallengeSnapshot::default(),
        );
        assert!(vm.boss_panel_sections.len() > 1);
        assert_eq!(
            vm.boss_panel_total,
            er_game_state::bosses_total_count() as u32
        );
        assert!(vm.boss_panel_sections.iter().any(|s| s.is_current));
    }
}
