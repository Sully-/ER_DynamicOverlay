use er_game_state::SCADUTREE_BLESSING_MAX;
use er_overlay_common::{GameTime, TrackKind};

use crate::view_model::{OverlayViewModel, TrackedEntryRow};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetricValue {
    Time(GameTime),
    Count {
        current: Option<u32>,
        max: Option<u32>,
    },
    NgCycle(Option<u32>),
    Unavailable,
}

/// Resolves a metric ref. `metric` is a built-in id, an aggregate group name, or a good key.
pub fn resolve_metric(metric: &str, vm: &OverlayViewModel) -> MetricValue {
    match metric {
        "igt" => vm
            .igt
            .map(MetricValue::Time)
            .unwrap_or(MetricValue::Unavailable),
        "deaths" => MetricValue::Count {
            current: vm.deaths,
            max: None,
        },
        "ng_cycle" => MetricValue::NgCycle(vm.ng_cycle),
        "scadutree_blessing" => MetricValue::Count {
            current: vm.scadutree_blessing,
            max: Some(SCADUTREE_BLESSING_MAX),
        },
        "bosses" => MetricValue::Count {
            current: vm.bosses_killed,
            max: Some(vm.bosses_total),
        },
        other => {
            if let Some(group) = vm.group(other) {
                MetricValue::Count {
                    current: group.owned,
                    max: Some(group.total),
                }
            } else if let Some(row) = vm.tracked(other) {
                match row.kind {
                    TrackKind::Countable { count } => MetricValue::Count {
                        current: count,
                        max: row.max,
                    },
                    TrackKind::Unique { acquired } => MetricValue::Count {
                        current: acquired.map(u32::from),
                        max: Some(1),
                    },
                }
            } else {
                MetricValue::Unavailable
            }
        }
    }
}

pub fn resolve_tracked_key<'a>(key: &str, vm: &'a OverlayViewModel) -> Option<&'a TrackedEntryRow> {
    vm.tracked(key)
}

pub fn metric_is_complete(value: &MetricValue) -> bool {
    match value {
        MetricValue::Count {
            current: Some(c),
            max: Some(m),
        } => *m > 0 && *c >= *m,
        _ => false,
    }
}

pub fn format_metric_value(value: &MetricValue, show_max: bool) -> String {
    match value {
        MetricValue::Time(t) => t.format_hms(),
        MetricValue::Count {
            current: Some(c),
            max: Some(m),
        } if show_max => format!("{c}/{m}"),
        MetricValue::Count {
            current: Some(c),
            max: None,
        } => c.to_string(),
        MetricValue::Count {
            current: Some(c),
            max: Some(_),
        } => c.to_string(),
        MetricValue::Count { current: None, .. } | MetricValue::Unavailable => "---".to_string(),
        MetricValue::NgCycle(Some(n)) => format!("NG+{n}"),
        MetricValue::NgCycle(None) => "---".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use er_game_state::mock::MockGameState;

    use super::*;
    use crate::view_model::build_view_model;

    fn keys(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    fn equipped_keys(list: &[&str]) -> HashSet<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn active_metrics_resolve_from_mock() {
        let refs = keys(&[
            "godrick_rune",
            "smithing_stone_1",
            "somber_ancient_dragon_smithing_stone",
        ]);
        let vm = build_view_model(
            &MockGameState::default(),
            &refs,
            &HashSet::new(),
            er_overlay_common::BossPanelScope::CurrentRegion,
        );
        assert!(matches!(resolve_metric("igt", &vm), MetricValue::Time(_)));
        assert!(resolve_tracked_key("godrick_rune", &vm).is_some());
        assert!(resolve_tracked_key("smithing_stone_1", &vm).is_some());
        assert!(resolve_tracked_key("somber_ancient_dragon_smithing_stone", &vm).is_some());
    }

    #[test]
    fn great_runes_group_resolves_as_count() {
        let vm = build_view_model(
            &MockGameState::default(),
            &[],
            &HashSet::new(),
            er_overlay_common::BossPanelScope::CurrentRegion,
        );
        assert_eq!(
            resolve_metric("great_runes", &vm),
            MetricValue::Count {
                current: Some(0),
                max: Some(7),
            }
        );
    }

    #[test]
    fn scadutree_blessing_resolves_from_mock() {
        let vm = build_view_model(
            &MockGameState::default(),
            &[],
            &HashSet::new(),
            er_overlay_common::BossPanelScope::CurrentRegion,
        );
        assert_eq!(
            resolve_metric("scadutree_blessing", &vm),
            MetricValue::Count {
                current: Some(12),
                max: Some(SCADUTREE_BLESSING_MAX),
            }
        );
        assert_eq!(
            format_metric_value(
                &MetricValue::Count {
                    current: Some(12),
                    max: Some(SCADUTREE_BLESSING_MAX),
                },
                true
            ),
            "12/20"
        );
    }

    #[test]
    fn ng_cycle_formats_as_ng_plus() {
        let vm = build_view_model(
            &MockGameState::default(),
            &[],
            &HashSet::new(),
            er_overlay_common::BossPanelScope::CurrentRegion,
        );
        assert_eq!(
            resolve_metric("ng_cycle", &vm),
            MetricValue::NgCycle(Some(2))
        );
        assert_eq!(
            format_metric_value(&MetricValue::NgCycle(Some(0)), false),
            "NG+0"
        );
        assert_eq!(
            format_metric_value(&MetricValue::NgCycle(Some(2)), false),
            "NG+2"
        );
    }

    #[test]
    fn talisman_item_resolves_from_mock() {
        let refs = keys(&["daedicar_s_woe"]);
        let vm = build_view_model(
            &MockGameState::default(),
            &refs,
            &HashSet::new(),
            er_overlay_common::BossPanelScope::CurrentRegion,
        );
        assert!(resolve_tracked_key("daedicar_s_woe", &vm).is_some());
    }

    #[test]
    fn equipped_tracking_populates_row_when_requested() {
        let refs = keys(&["daedicar_s_woe"]);
        let equipped = equipped_keys(&["daedicar_s_woe"]);
        let vm = build_view_model(
            &MockGameState::default(),
            &refs,
            &equipped,
            er_overlay_common::BossPanelScope::CurrentRegion,
        );
        let row = resolve_tracked_key("daedicar_s_woe", &vm).unwrap();
        assert_eq!(row.equipped, Some(false));
    }
}
