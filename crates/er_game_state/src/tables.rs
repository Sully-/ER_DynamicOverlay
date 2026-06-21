use std::collections::HashMap;
use std::sync::LazyLock;

use serde::Deserialize;

use crate::boss_table::boss_table;
use crate::lot_flags::{effective_good_flag, LotRef, LotTable};
use crate::GameStateSource;

#[derive(Debug, Clone)]
pub struct BossEntry {
    pub flag_id: u32,
    pub name: String,
    pub region: String,
    pub icon: String,
    pub place: Option<String>,
    pub dlc: bool,
}

/// Inventory category a tracked good lives in. Mirrors the relevant subset of
/// `eldenring::cs::ItemCategory`, but is kept independent so the data layer
/// compiles without the `game` feature (mock / tests).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    /// `EquipParamGoods` — consumables, runes, key items.
    #[default]
    Goods,
    /// `EquipParamAccessory` — talismans.
    Accessory,
}

#[derive(Debug, Clone)]
pub struct GoodEntry {
    pub key: String,
    pub item_id: u32,
    pub name: String,
    pub file: String,
    /// Inventory category the item belongs to (goods vs accessory/talisman).
    pub category: ItemKind,
    /// Optional event flag used to detect ownership (falls back to inventory presence).
    pub pickup_flag: Option<u32>,
    /// Optional vanilla lot metadata used by layout-driven historic tracking.
    pub historic_lot: Option<LotRef>,
    /// Optional display cap for a counter (e.g. scadutree → "N/50").
    pub max: Option<u32>,
    /// Stackable good: show inventory quantity (`true`) instead of owned / not-owned.
    pub countable: bool,
}

#[derive(Debug, Clone)]
struct ParsedGood {
    key: String,
    item_id: u32,
    name: String,
    file: String,
    category: ItemKind,
    pickup_flag: Option<u32>,
    historic_lot: Option<LotRef>,
    max: Option<u32>,
    countable: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct GroupDef {
    members: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GoodsTable {
    #[serde(default)]
    groups: HashMap<String, GroupDef>,
    good: Vec<GoodRow>,
}

#[derive(Debug, Deserialize)]
struct GoodRow {
    key: String,
    item_id: u32,
    #[serde(default)]
    name: String,
    file: Option<String>,
    #[serde(default)]
    category: ItemKind,
    pickup_flag: Option<u32>,
    #[serde(default)]
    historic_lot_table: Option<LotTable>,
    #[serde(default)]
    historic_lot_id: Option<u32>,
    #[serde(default)]
    historic_vanilla_flag: Option<u32>,
    max: Option<u32>,
    #[serde(default)]
    count: bool,
}

const GOODS_TOML: &str = include_str!("../tables/goods.toml");

/// `name -> member good keys` for every aggregate group in `goods.toml`.
static GROUPS: LazyLock<HashMap<String, Vec<String>>> = LazyLock::new(|| {
    let table: GoodsTable = toml::from_str(GOODS_TOML).expect("goods.toml must parse");
    table
        .groups
        .into_iter()
        .map(|(name, def)| (name, def.members))
        .collect()
});

static GOODS: LazyLock<Vec<ParsedGood>> = LazyLock::new(|| {
    let table: GoodsTable = toml::from_str(GOODS_TOML).expect("goods.toml must parse");
    table
        .good
        .into_iter()
        .map(|row| {
            let historic_lot = row.historic_lot();
            ParsedGood {
                key: row.key.clone(),
                item_id: row.item_id,
                name: row.name,
                file: row.file.unwrap_or_else(|| format!("{}.png", row.key)),
                category: row.category,
                pickup_flag: row.pickup_flag,
                historic_lot,
                max: row.max,
                countable: row.count,
            }
        })
        .collect()
});

pub(crate) fn boss_entries() -> std::sync::Arc<crate::boss_table::BossTableData> {
    boss_table()
}

impl GoodRow {
    fn historic_lot(&self) -> Option<LotRef> {
        Some(LotRef {
            table: self.historic_lot_table?,
            lot_id: self.historic_lot_id?,
            vanilla_flag: self.historic_vanilla_flag,
        })
    }
}

pub fn boss_entries_full() -> std::sync::Arc<crate::boss_table::BossTableData> {
    boss_table()
}

/// Resolves the bosses.toml region label for a live map id.
/// Keys match bosses.json: usually `map_id / 1000`, with a direct lookup fallback.
pub fn region_label_for_subregion(map_id: u32) -> Option<String> {
    let table = boss_table();
    let key = map_id / 1000;
    table
        .subregion_to_region
        .get(&key)
        .or_else(|| table.subregion_to_region.get(&map_id))
        .cloned()
}

/// Boss entries whose `region` label matches `region`.
pub fn bosses_in_region(region: &str) -> Vec<BossEntry> {
    boss_table()
        .bosses
        .iter()
        .filter(|b| b.region == region)
        .cloned()
        .collect()
}

/// Region labels in bosses.json display order.
pub fn region_names() -> Vec<String> {
    boss_table().region_names.clone()
}

fn good_entry(g: &ParsedGood) -> GoodEntry {
    GoodEntry {
        key: g.key.clone(),
        item_id: g.item_id,
        name: g.name.clone(),
        file: g.file.clone(),
        category: g.category,
        pickup_flag: g.pickup_flag,
        historic_lot: g.historic_lot,
        max: g.max,
        countable: g.countable,
    }
}

pub fn good_by_key(key: &str) -> Option<GoodEntry> {
    GOODS.iter().find(|g| g.key == key).map(good_entry)
}

/// Names of every aggregate group declared in `goods.toml`.
pub fn group_names() -> Vec<String> {
    GROUPS.keys().cloned().collect()
}

/// Goods that belong to an aggregate group (unknown member keys are skipped).
pub fn group_members(name: &str) -> Vec<GoodEntry> {
    GROUPS
        .get(name)
        .map(|keys| keys.iter().filter_map(|k| good_by_key(k)).collect())
        .unwrap_or_default()
}

/// Number of resolvable members in an aggregate group.
pub fn group_size(name: &str) -> u32 {
    group_members(name).len() as u32
}

/// Whether a good is owned: present in the inventory, or its pickup flag is set.
pub fn item_owned(
    source: &dyn GameStateSource,
    item_id: u32,
    category: ItemKind,
    pickup_flag: Option<u32>,
) -> Option<bool> {
    match source.has_item(item_id, category) {
        Some(true) => Some(true),
        has => match pickup_flag {
            Some(flag) => source.get_flag(flag).or(has),
            None => has,
        },
    }
}

/// Whether a good is historically owned when the active layout asks for historic tracking.
pub fn item_owned_historic(
    source: &dyn GameStateSource,
    key: &str,
    item_id: u32,
    category: ItemKind,
    pickup_flag: Option<u32>,
    historic_lot: Option<LotRef>,
) -> Option<bool> {
    let current = item_owned(source, item_id, category, pickup_flag);
    if current == Some(true) {
        return current;
    }

    match effective_good_flag(key, historic_lot) {
        Some(flag) => source.get_flag(flag).or(current),
        None => current,
    }
}

/// `(owned, total)` members of an aggregate group, or `None` while the data is incomplete.
pub fn group_progress(source: &dyn GameStateSource, name: &str) -> Option<(u32, u32)> {
    let members = group_members(name);
    if members.is_empty() {
        return None;
    }
    let total = members.len() as u32;
    let mut owned = 0u32;
    for m in &members {
        match item_owned(source, m.item_id, m.category, m.pickup_flag) {
            Some(true) => owned += 1,
            Some(false) => {}
            None => return None,
        }
    }
    Some((owned, total))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockGameState;
    use crate::GameStateSource;
    use er_overlay_common::{GameStateDiagnostics, GameTime};

    #[test]
    fn boss_table_non_empty() {
        assert!(!boss_entries().bosses.is_empty());
    }

    #[test]
    fn boss_total_counts_rows_not_unique_flags() {
        let all = &boss_entries().bosses;
        assert_eq!(crate::boss_table::bosses_total_count(), all.len());
    }

    #[test]
    fn goods_unique_keys() {
        let keys: Vec<_> = GOODS.iter().map(|g| g.key.as_str()).collect();
        let mut sorted = keys.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), keys.len(), "duplicate good key in goods.toml");
    }

    #[test]
    fn great_runes_group() {
        assert!(group_names().iter().any(|n| n == "great_runes"));
        assert_eq!(group_size("great_runes"), 7);
        let runes = group_members("great_runes");
        assert_eq!(runes.len(), 7);
        assert_eq!(runes[0].item_id, 191);
        assert_eq!(runes[0].pickup_flag, Some(171));
        assert!(!runes[0].countable, "runes are owned-checks, not counters");
    }

    #[test]
    fn runes_and_talismans_are_owned_checks() {
        assert!(!good_by_key("godrick_rune").unwrap().countable);
        assert!(!good_by_key("daedicar_s_woe").unwrap().countable);
    }

    #[test]
    fn good_categories_are_tagged() {
        // Runes / consumables are plain goods; talismans are accessories.
        assert_eq!(
            good_by_key("godrick_rune").unwrap().category,
            ItemKind::Goods
        );
        assert_eq!(good_by_key("scadutree").unwrap().category, ItemKind::Goods);
        assert_eq!(
            good_by_key("crimson_amber_medallion").unwrap().category,
            ItemKind::Accessory
        );
        // Every great-rune member resolves as a goods item.
        for m in group_members("great_runes") {
            assert_eq!(m.category, ItemKind::Goods, "{} should be goods", m.key);
        }
    }

    #[test]
    fn stackable_goods_are_counters() {
        assert!(good_by_key("scadutree").unwrap().countable);
        assert_eq!(good_by_key("scadutree").unwrap().max, Some(50));
        assert!(good_by_key("kindling").unwrap().countable);
        assert!(good_by_key("smithing_stone_1").unwrap().countable);
    }

    #[test]
    fn goods_table_parses_historic_lot() {
        let raw = r#"
[[good]]
key = "fire_scorpion_charm"
item_id = 1170
category = "accessory"
name = "Fire Scorpion Charm"
historic_lot_table = "map"
historic_lot_id = 123456
historic_vanilla_flag = 40001234
"#;
        let table: GoodsTable = toml::from_str(raw).unwrap();
        let lot = table.good[0].historic_lot().unwrap();
        assert_eq!(lot.table, crate::LotTable::Map);
        assert_eq!(lot.lot_id, 123456);
        assert_eq!(lot.vanilla_flag, Some(40001234));
    }

    #[test]
    fn historic_ownership_uses_lot_flag_when_inventory_is_empty() {
        struct Source;

        impl GameStateSource for Source {
            fn get_igt(&self) -> Option<GameTime> {
                None
            }
            fn get_death_count(&self) -> Option<u32> {
                None
            }
            fn get_ng_cycle(&self) -> Option<u32> {
                None
            }
            fn get_scadutree_blessing(&self) -> Option<u32> {
                None
            }
            fn get_killed_boss_count(&self) -> Option<u32> {
                None
            }
            fn get_goods_quantity(&self, _item_id: u32) -> Option<u32> {
                None
            }
            fn has_item(&self, _item_id: u32, _category: ItemKind) -> Option<bool> {
                Some(false)
            }
            fn is_item_equipped(&self, _item_id: u32, _category: ItemKind) -> Option<bool> {
                None
            }
            fn get_flag(&self, flag_id: u32) -> Option<bool> {
                Some(flag_id == 40001234)
            }
            fn get_current_subregion_id(&self) -> Option<u32> {
                None
            }
            fn get_status(&self) -> GameStateDiagnostics {
                GameStateDiagnostics::default()
            }
            fn bosses_total(&self) -> u32 {
                0
            }
        }

        crate::clear_lot_seed_flags();
        assert_eq!(
            item_owned_historic(
                &Source,
                "fire_scorpion_charm",
                1170,
                ItemKind::Accessory,
                None,
                Some(crate::LotRef {
                    table: crate::LotTable::Map,
                    lot_id: 123456,
                    vanilla_flag: Some(40001234),
                }),
            ),
            Some(true)
        );
    }

    #[test]
    fn group_progress_counts_owned_members() {
        let source = MockGameState::default();
        assert_eq!(group_progress(&source, "great_runes"), Some((0, 7)));
    }

    #[test]
    fn boss_entries_have_metadata() {
        let table = boss_entries_full();
        let b = table.bosses.first().unwrap();
        assert!(!b.name.is_empty());
        assert!(!b.region.is_empty());
        assert!(!b.icon.is_empty());
    }

    #[test]
    fn region_map_resolves_limgrave() {
        assert_eq!(
            region_label_for_subregion(6_100_000).as_deref(),
            Some("Limgrave")
        );
        assert_eq!(
            region_label_for_subregion(3_002_000).as_deref(),
            Some("Limgrave")
        );
        assert_eq!(
            region_label_for_subregion(6100).as_deref(),
            Some("Limgrave")
        );
        assert_eq!(
            region_label_for_subregion(1_001_001).as_deref(),
            Some("Liurnia of the Lakes")
        );
    }

    #[test]
    fn bosses_in_region_non_empty() {
        assert!(!bosses_in_region("Limgrave").is_empty());
    }

    #[test]
    fn region_names_non_empty() {
        assert!(!region_names().is_empty());
    }

    #[test]
    fn region_names_follow_bosses_json_order() {
        let names = region_names();
        assert_eq!(names.first().map(String::as_str), Some("Limgrave"));
        assert_eq!(names.get(1).map(String::as_str), Some("Weeping Peninsula"));
        let limgrave_pos = names.iter().position(|r| r == "Limgrave").unwrap();
        let liurnia_pos = names
            .iter()
            .position(|r| r == "Liurnia of the Lakes")
            .unwrap();
        assert!(limgrave_pos < liurnia_pos);
    }

    #[test]
    fn tree_sentinel_has_place() {
        let table = boss_entries_full();
        let sentinel = table
            .bosses
            .iter()
            .find(|b| b.flag_id == 1042360800)
            .expect("Tree Sentinel");
        assert_eq!(sentinel.place.as_deref(), Some("Church of Elleh"));
    }
}
