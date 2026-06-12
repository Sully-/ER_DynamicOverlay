use std::collections::HashMap;

use er_game_state::{
    good_by_key, group_names, group_progress, group_size, item_owned, GameStateSource,
};
use er_overlay_common::{GameStateDiagnostics, GameTime, TrackKind};

#[derive(Debug, Clone)]
pub struct TrackedEntryRow {
    pub name: String,
    pub kind: TrackKind,
    pub icon_key: String,
    /// Optional display cap for a countable good (e.g. scadutree → "N/50").
    pub max: Option<u32>,
}

/// Owned / total members of an aggregate group (e.g. great runes).
#[derive(Debug, Clone, Copy)]
pub struct GroupValue {
    pub owned: Option<u32>,
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
}

impl OverlayViewModel {
    pub fn tracked(&self, key: &str) -> Option<&TrackedEntryRow> {
        self.tracked_by_key.get(key)
    }

    pub fn group(&self, name: &str) -> Option<&GroupValue> {
        self.groups.get(name)
    }
}

/// Builds the view model. `referenced_keys` are the good keys / metric refs used by the active
/// layout (from `LayoutConfig::collect_data_refs`); only those goods are resolved against the
/// inventory. Aggregate groups are always resolved (there are few, with few members).
pub fn build_view_model(
    source: &dyn GameStateSource,
    referenced_keys: &[String],
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
        tracked_by_key.insert(
            good.key.clone(),
            TrackedEntryRow {
                name: good.name,
                kind,
                icon_key: good.key,
                max: good.max,
            },
        );
    }

    let mut groups = HashMap::new();
    for name in group_names() {
        let total = group_size(&name);
        let owned = group_progress(source, &name).map(|(owned, _)| owned);
        groups.insert(name, GroupValue { owned, total });
    }

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
    }
}
