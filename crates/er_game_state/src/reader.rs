use std::collections::{HashMap, HashSet};
use std::time::Duration;

use eldenring::cs::{CSEventFlagMan, GameDataMan};
use eldenring::util::system::{wait_for_system_init, SystemInitError};
use er_overlay_common::{BackendKind, GameStateDiagnostics, GameTime};
use fromsoftware_shared::{program::Program, FromStatic};
use tracing::{debug, warn};

use crate::boss_table::bosses_total_count;
use crate::game_version::game_supported;
use crate::tables::{boss_entries, group_size};
use crate::{GameStateSource, ItemKind};

/// Per-poll budget spent waiting for the FromSoftware system to come up. Kept
/// short so the render thread is never blocked for long; init simply retries on
/// the next poll until it succeeds.
const SYSTEM_INIT_POLL_TIMEOUT: Duration = Duration::from_millis(20);

pub struct GameStateReader {
    diagnostics: GameStateDiagnostics,
    initialized: bool,
    init_timed_out_logged: bool,
    pointer_summary_logged: bool,
    owned_item_ids: Option<HashSet<u32>>,
    /// Goods param_id → quantity, filled during the same walk as `owned_item_ids`.
    goods_quantities: Option<HashMap<u32, u32>>,
    equipped_item_ids: Option<HashSet<u32>>,
    killed_boss_count: Option<u32>,
    /// Per-boss flag values from the last `refresh_boss_cache` (avoids re-reading
    /// every flag while building the boss panel view model).
    boss_flag_cache: Option<HashMap<u32, bool>>,
    current_subregion_id: Option<u32>,
}

impl Default for GameStateReader {
    fn default() -> Self {
        Self::new()
    }
}

impl GameStateReader {
    pub fn new() -> Self {
        Self {
            diagnostics: GameStateDiagnostics::default(),
            initialized: false,
            init_timed_out_logged: false,
            pointer_summary_logged: false,
            owned_item_ids: None,
            goods_quantities: None,
            equipped_item_ids: None,
            killed_boss_count: None,
            boss_flag_cache: None,
            current_subregion_id: None,
        }
    }

    fn refresh_inventory_cache(&mut self) {
        match crate::inventory::game::owned_and_goods_quantities() {
            Some((owned, goods_qty)) => {
                self.owned_item_ids = Some(owned);
                self.goods_quantities = Some(goods_qty);
            }
            None => {
                self.owned_item_ids = None;
                self.goods_quantities = None;
            }
        }
        self.equipped_item_ids = crate::inventory::game::equipped_item_ids();
    }

    fn has_param(&self, param_id: u32, category: ItemKind) -> Option<bool> {
        let owned = self.owned_item_ids.as_ref()?;
        Some(crate::inventory::game::owned_contains(
            owned, param_id, category,
        ))
    }

    fn is_equipped(&self, param_id: u32, category: ItemKind) -> Option<bool> {
        let equipped = self.equipped_item_ids.as_ref()?;
        Some(crate::inventory::game::equipped_contains(
            equipped, param_id, category,
        ))
    }

    /// Attempts initialization without blocking the caller (render thread) for
    /// long. Returns immediately once initialized; otherwise spends only a small
    /// time budget and retries on the next poll.
    pub fn ensure_initialized(&mut self) {
        if self.initialized {
            return;
        }
        // Every read below goes through the `eldenring` RVA table, which panics
        // on a build it doesn't know. Checking the exe metadata first turns a
        // game-breaking abort after an Elden Ring patch into blank metrics.
        if !game_supported() {
            self.diagnostics.backend = BackendKind::Unavailable;
            return;
        }
        match wait_for_system_init(&Program::current(), SYSTEM_INIT_POLL_TIMEOUT) {
            Ok(()) => {
                debug!("fromsoftware system init OK");
                self.diagnostics.backend = BackendKind::FromSoftwareRs;
                self.initialized = true;
            }
            Err(SystemInitError::Timeout) => {
                // Not ready yet (e.g. still on a loading screen). Retry next poll
                // instead of stalling the render thread.
                if !self.init_timed_out_logged {
                    debug!("fromsoftware system not ready yet; will retry");
                    self.init_timed_out_logged = true;
                }
                self.diagnostics.backend = BackendKind::Unavailable;
            }
            Err(e) => {
                warn!("wait_for_system_init failed: {e:?}");
                self.diagnostics.backend = BackendKind::Unavailable;
            }
        }
    }

    fn refresh_diag_flags(&mut self) {
        self.diagnostics.boss_flags_loaded = boss_entries().bosses.len() as u32;
        self.diagnostics.great_rune_flags_loaded = group_size("great_runes");
        if !game_supported() {
            return;
        }
        self.diagnostics.gamedata_man_resolved = unsafe { GameDataMan::instance().is_ok() };
        self.diagnostics.event_flag_man_resolved = unsafe { CSEventFlagMan::instance().is_ok() };
        self.diagnostics.world_chr_man_resolved = crate::inventory::game::inventory_available();
        self.diagnostics.field_area_resolved = crate::field_area::field_area_available();
    }

    fn read_flag(flag_id: u32) -> Option<bool> {
        if !game_supported() {
            return None;
        }
        let man = unsafe { CSEventFlagMan::instance().ok()? };
        Some(man.virtual_memory_flag.get_flag(flag_id))
    }

    /// Single guarded entry point to `GameDataMan`, which resolves through the
    /// version-specific RVA table and would panic on an unsupported build.
    fn game_data_man() -> Option<&'static GameDataMan> {
        if !game_supported() {
            return None;
        }
        unsafe { GameDataMan::instance().ok() }
    }

    /// Recomputes the killed-boss count by scanning every boss flag once. Cached
    /// so the view-model build doesn't re-read 200+ flags repeatedly.
    fn refresh_boss_cache(&mut self) {
        let mut any = false;
        let mut killed = 0u32;
        let mut flags = HashMap::with_capacity(boss_entries().bosses.len());
        for b in &boss_entries().bosses {
            match Self::read_flag(b.flag_id) {
                Some(true) => {
                    any = true;
                    killed += 1;
                    flags.insert(b.flag_id, true);
                }
                Some(false) => {
                    any = true;
                    flags.insert(b.flag_id, false);
                }
                None => {
                    self.killed_boss_count = None;
                    self.boss_flag_cache = None;
                    return;
                }
            }
        }
        self.killed_boss_count = any.then_some(killed);
        self.boss_flag_cache = any.then_some(flags);
    }
}

impl GameStateSource for GameStateReader {
    fn get_igt(&self) -> Option<GameTime> {
        Some(GameTime::from_ms(Self::game_data_man()?.play_time))
    }

    fn get_death_count(&self) -> Option<u32> {
        Some(Self::game_data_man()?.death_count)
    }

    fn get_ng_cycle(&self) -> Option<u32> {
        Some(Self::game_data_man()?.ng_lvl)
    }

    fn get_scadutree_blessing(&self) -> Option<u32> {
        Some(
            Self::game_data_man()?
                .main_player_game_data
                .scadutree_blessing as u32,
        )
    }

    fn get_goods_quantity(&self, item_id: u32) -> Option<u32> {
        let qty = self.goods_quantities.as_ref()?;
        // Missing from the map means the good is not in inventory → quantity 0.
        Some(qty.get(&item_id).copied().unwrap_or(0))
    }

    fn has_item(&self, item_id: u32, category: ItemKind) -> Option<bool> {
        self.has_param(item_id, category)
    }

    fn is_item_equipped(&self, item_id: u32, category: ItemKind) -> Option<bool> {
        self.is_equipped(item_id, category)
    }

    fn get_flag(&self, flag_id: u32) -> Option<bool> {
        if let Some(cache) = self.boss_flag_cache.as_ref() {
            if let Some(&v) = cache.get(&flag_id) {
                return Some(v);
            }
        }
        Self::read_flag(flag_id)
    }

    fn get_current_subregion_id(&self) -> Option<u32> {
        self.current_subregion_id
    }

    fn get_killed_boss_count(&self) -> Option<u32> {
        self.killed_boss_count
    }

    fn get_status(&self) -> GameStateDiagnostics {
        let mut d = self.diagnostics.clone();
        d.igt_readable = self.get_igt().is_some();
        d.death_count_readable = self.get_death_count().is_some();
        d.inventory_readable = self.owned_item_ids.is_some();
        d
    }

    fn bosses_total(&self) -> u32 {
        bosses_total_count() as u32
    }
}

impl GameStateReader {
    /// Clears cached boss kill counts after `bosses.toml` is reloaded at runtime.
    pub fn invalidate_boss_cache(&mut self) {
        self.killed_boss_count = None;
        self.boss_flag_cache = None;
    }

    fn refresh_subregion(&mut self) {
        self.current_subregion_id = crate::field_area::read_current_subregion_id();
    }

    /// Whether game-memory reads are available.
    pub fn is_ready(&self) -> bool {
        self.initialized
    }

    /// Whether challenge counters should be polled (stable gameplay, like EROverlay).
    pub fn challenge_update_ready(&self) -> bool {
        if !self.initialized {
            return false;
        }
        let igt_ms = self.get_igt().map(|t| t.total_ms).unwrap_or(0);
        crate::screen_state::challenge_update_ready(
            igt_ms,
            crate::screen_state::read_screen_state(),
        )
    }

    pub fn poll(&mut self) {
        self.ensure_initialized();
        if self.initialized {
            self.refresh_inventory_cache();
            self.refresh_boss_cache();
            self.refresh_subregion();
        }
        self.refresh_diag_flags();
        if self.initialized && !self.pointer_summary_logged {
            self.pointer_summary_logged = true;
            crate::game_version::log_pointer_summary(
                self.diagnostics.gamedata_man_resolved,
                self.diagnostics.event_flag_man_resolved,
                self.diagnostics.world_chr_man_resolved,
                self.diagnostics.field_area_resolved,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reader_constructed() {
        let r = GameStateReader::new();
        assert_eq!(r.bosses_total(), bosses_total_count() as u32);
    }
}
