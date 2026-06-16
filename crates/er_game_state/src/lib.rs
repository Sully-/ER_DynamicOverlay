mod boss_table;
mod checks_table;
mod inventory;
mod tables;

#[cfg(feature = "game")]
mod field_area;

#[cfg(feature = "game")]
mod game_language;

#[cfg(feature = "game")]
mod reader;

#[cfg(feature = "game")]
mod screen_state;

pub use boss_table::{
    active_boss_locale, bosses_total_count, load_boss_table_from_path, normalize_locale_id,
    reload_boss_table_if_modified, resolve_boss_table_path, resolve_locale_id, BossTableData,
    DEFAULT_LOCALE_ID,
};
pub use checks_table::{
    active_checks_locale, checks_in_region, checks_region_label_for_subregion, checks_region_names,
    checks_seed_flags_loaded, checks_seed_regulation_hash, checks_total_count,
    clear_checks_seed_flags, effective_flag, load_checks_flags_from_path,
    load_checks_table_from_path, reload_checks_flags_if_modified, reload_checks_table_if_modified,
    resolve_checks_table_path, CheckEntry, ChecksFlagsData, ChecksTableData, LotParam,
};
pub use tables::{
    boss_entries_full, bosses_in_region, good_by_key, group_members, group_names, group_progress,
    group_size, item_owned, region_label_for_subregion, region_names, BossEntry, GoodEntry,
    ItemKind,
};

/// Maximum Scadutree Blessing level (fragments spent at Sites of Grace in the DLC).
pub const SCADUTREE_BLESSING_MAX: u32 = 20;

#[cfg(feature = "game")]
pub use reader::GameStateReader;

use er_overlay_common::{GameStateDiagnostics, GameTime};

/// Trait for reading game state (live or mock). Item- and group-level meaning is layered
/// on top of these primitives by the free functions in `tables` (`item_owned`, `group_progress`).
pub trait GameStateSource {
    fn get_igt(&self) -> Option<GameTime>;
    fn get_death_count(&self) -> Option<u32>;
    fn get_ng_cycle(&self) -> Option<u32>;
    /// Scadutree Blessing level (`PlayerGameData.scadutree_blessing`), not fragment inventory count.
    fn get_scadutree_blessing(&self) -> Option<u32>;
    fn get_killed_boss_count(&self) -> Option<u32>;
    /// Inventory quantity of a good (`ItemCategory::Goods`).
    fn get_goods_quantity(&self, item_id: u32) -> Option<u32>;
    /// Whether an item id is present in the inventory, scoped to its category
    /// (goods vs accessory/talisman) to avoid cross-category param-id collisions.
    fn has_item(&self, item_id: u32, category: ItemKind) -> Option<bool>;
    /// Whether an item is currently equipped (talismans, covenant, quick slots, pouch).
    fn is_item_equipped(&self, item_id: u32, category: ItemKind) -> Option<bool>;
    /// State of an event flag.
    fn get_flag(&self, flag_id: u32) -> Option<bool>;
    /// Raw map / subregion id from `FieldArea` (divide by 1000 for region lookup).
    fn get_current_subregion_id(&self) -> Option<u32>;
    fn get_status(&self) -> GameStateDiagnostics;
    fn bosses_total(&self) -> u32;
}

#[cfg(any(test, feature = "mock"))]
pub mod mock {
    use super::*;
    use er_overlay_common::{BackendKind, GameStateDiagnostics, GameTime};

    pub struct MockGameState {
        pub igt: Option<GameTime>,
        pub deaths: Option<u32>,
        pub ng_cycle: Option<u32>,
        pub scadutree_blessing: Option<u32>,
        pub bosses_killed: Option<u32>,
        pub subregion_id: Option<u32>,
    }

    impl Default for MockGameState {
        fn default() -> Self {
            Self {
                igt: Some(GameTime::from_ms(3_661_000)),
                deaths: Some(42),
                ng_cycle: Some(2),
                scadutree_blessing: Some(12),
                bosses_killed: Some(8),
                subregion_id: Some(6_100_000),
            }
        }
    }

    impl GameStateSource for MockGameState {
        fn get_igt(&self) -> Option<GameTime> {
            self.igt
        }
        fn get_death_count(&self) -> Option<u32> {
            self.deaths
        }
        fn get_ng_cycle(&self) -> Option<u32> {
            self.ng_cycle
        }
        fn get_scadutree_blessing(&self) -> Option<u32> {
            self.scadutree_blessing
        }
        fn get_killed_boss_count(&self) -> Option<u32> {
            self.bosses_killed
        }
        fn get_goods_quantity(&self, _item_id: u32) -> Option<u32> {
            Some(0)
        }
        fn has_item(&self, _item_id: u32, _category: ItemKind) -> Option<bool> {
            Some(false)
        }
        fn is_item_equipped(&self, _item_id: u32, _category: ItemKind) -> Option<bool> {
            Some(false)
        }
        fn get_flag(&self, _flag_id: u32) -> Option<bool> {
            Some(false)
        }
        fn get_current_subregion_id(&self) -> Option<u32> {
            self.subregion_id
        }
        fn get_status(&self) -> GameStateDiagnostics {
            GameStateDiagnostics {
                backend: BackendKind::Unavailable,
                boss_flags_loaded: super::bosses_total_count() as u32,
                great_rune_flags_loaded: super::group_size("great_runes"),
                ..Default::default()
            }
        }
        fn bosses_total(&self) -> u32 {
            super::bosses_total_count() as u32
        }
    }
}
