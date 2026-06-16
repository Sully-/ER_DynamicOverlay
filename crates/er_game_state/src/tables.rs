use std::collections::HashMap;
use std::sync::LazyLock;

use serde::Deserialize;

use crate::boss_table::boss_table;
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
        .map(|row| ParsedGood {
            key: row.key.clone(),
            item_id: row.item_id,
            name: row.name,
            file: row.file.unwrap_or_else(|| format!("{}.png", row.key)),
            category: row.category,
            pickup_flag: row.pickup_flag,
            max: row.max,
            countable: row.count,
        })
        .collect()
});

pub(crate) fn boss_entries() -> std::sync::Arc<crate::boss_table::BossTableData> {
    boss_table()
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

    #[test]
    fn boss_table_non_empty() {
        assert!(!boss_entries().bosses.is_empty());
    }

    #[test]
    fn boss_table_unique_flags() {
        let all = &boss_entries().bosses;
        let mut ids: Vec<u32> = all.iter().map(|b| b.flag_id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(
            ids.len(),
            all.len(),
            "duplicate boss flag_id in bosses.toml"
        );
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
