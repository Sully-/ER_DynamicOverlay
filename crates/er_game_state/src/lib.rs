mod inventory;
mod tables;

#[cfg(feature = "game")]
mod reader;

pub use tables::{
    good_by_key, group_members, group_names, group_progress, group_size, item_owned, GoodEntry,
    ItemKind, BOSSES_TOTAL,
};

#[cfg(feature = "game")]
pub use reader::GameStateReader;

use er_overlay_common::{GameStateDiagnostics, GameTime};

/// Trait for reading game state (live or mock). Item- and group-level meaning is layered
/// on top of these primitives by the free functions in `tables` (`item_owned`, `group_progress`).
pub trait GameStateSource {
    fn get_igt(&self) -> Option<GameTime>;
    fn get_death_count(&self) -> Option<u32>;
    fn get_ng_cycle(&self) -> Option<u32>;
    fn get_killed_boss_count(&self) -> Option<u32>;
    /// Inventory quantity of a good (`ItemCategory::Goods`).
    fn get_goods_quantity(&self, item_id: u32) -> Option<u32>;
    /// Whether an item id is present in the inventory, scoped to its category
    /// (goods vs accessory/talisman) to avoid cross-category param-id collisions.
    fn has_item(&self, item_id: u32, category: ItemKind) -> Option<bool>;
    /// State of an event flag.
    fn get_flag(&self, flag_id: u32) -> Option<bool>;
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
        pub bosses_killed: Option<u32>,
    }

    impl Default for MockGameState {
        fn default() -> Self {
            Self {
                igt: Some(GameTime::from_ms(3_661_000)),
                deaths: Some(42),
                ng_cycle: Some(2),
                bosses_killed: Some(8),
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
        fn get_killed_boss_count(&self) -> Option<u32> {
            self.bosses_killed
        }
        fn get_goods_quantity(&self, _item_id: u32) -> Option<u32> {
            Some(0)
        }
        fn has_item(&self, _item_id: u32, _category: ItemKind) -> Option<bool> {
            Some(false)
        }
        fn get_flag(&self, _flag_id: u32) -> Option<bool> {
            Some(false)
        }
        fn get_status(&self) -> GameStateDiagnostics {
            GameStateDiagnostics {
                backend: BackendKind::Unavailable,
                boss_flags_loaded: super::BOSSES_TOTAL as u32,
                great_rune_flags_loaded: super::group_size("great_runes"),
                ..Default::default()
            }
        }
        fn bosses_total(&self) -> u32 {
            super::BOSSES_TOTAL as u32
        }
    }
}
