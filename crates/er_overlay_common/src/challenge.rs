use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// EROverlay uses event flag 101 (Stranded Graveyard / Cave of Knowledge exit).
pub const DEFAULT_CHALLENGE_START_FLAG: u32 = 101;

/// Challenge mode settings (from `er_overlay.toml`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChallengeConfig {
    /// When false, challenge metrics show `---` and no state is updated.
    #[serde(default)]
    pub enabled: bool,
    /// Maximum deaths allowed per run (inclusive). Challenge fails when deaths exceed this.
    #[serde(default)]
    pub max_deaths: u32,
    /// Event flag id that marks the start of a challenge run.
    #[serde(default = "default_start_flag")]
    pub start_flag: u32,
}

fn default_start_flag() -> u32 {
    DEFAULT_CHALLENGE_START_FLAG
}

impl Default for ChallengeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_deaths: 0,
            start_flag: DEFAULT_CHALLENGE_START_FLAG,
        }
    }
}

/// Persisted challenge progress (survives overlay restarts). Mirrors EROverlay `Challenge.txt`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
struct ChallengePersisted {
    #[serde(default)]
    pb: u32,
    #[serde(default)]
    tries: u32,
    #[serde(default)]
    deaths_on_start: u32,
    #[serde(default)]
    run_failed: bool,
}

/// Challenge metrics for the overlay UI (`pb`, `nbtries` layout tiles).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChallengeSnapshot {
    pub enabled: bool,
    /// Personal best: highest total boss kill count recorded while within the death budget.
    pub pb: u32,
    /// Number of failed challenge runs (`tries` in EROverlay).
    pub tries: u32,
}

pub fn default_challenge_state_path() -> PathBuf {
    crate::config::default_base_dir().join("challenge_state.toml")
}

fn load_challenge_state(path: &Path) -> ChallengePersisted {
    if !path.exists() {
        return ChallengePersisted::default();
    }
    match fs::read_to_string(path) {
        Ok(raw) => match toml::from_str(&raw) {
            Ok(state) => state,
            Err(e) => {
                tracing::warn!("Failed to parse challenge state at {}: {e}", path.display());
                ChallengePersisted::default()
            }
        },
        Err(e) => {
            tracing::warn!("Failed to read challenge state at {}: {e}", path.display());
            ChallengePersisted::default()
        }
    }
}

fn write_challenge_state(path: &Path, state: &ChallengePersisted) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let raw = toml::to_string_pretty(state).context("Failed to serialize challenge state")?;
    fs::write(path, raw).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

/// Tracks challenge state using the same rules as [soarqin/EROverlay](https://github.com/soarqin/EROverlay).
pub struct ChallengeTracker {
    config: ChallengeConfig,
    state_path: PathBuf,
    persisted: ChallengePersisted,
    reached_start: bool,
    last_player_deaths: Option<u32>,
    dirty: bool,
}

impl ChallengeTracker {
    pub fn new(config: ChallengeConfig, state_path: PathBuf) -> Self {
        let persisted = load_challenge_state(&state_path);
        Self {
            config,
            state_path,
            persisted,
            // EROverlay defaults `reachedGraveyard = true` to avoid rebaselining on inject.
            reached_start: true,
            last_player_deaths: None,
            dirty: false,
        }
    }

    pub fn sync_config(&mut self, config: &ChallengeConfig) {
        if self.config != *config {
            self.config = config.clone();
        }
    }

    /// Poll live counters and the challenge start flag.
    pub fn update(
        &mut self,
        deaths: Option<u32>,
        bosses: Option<u32>,
        start_flag: Option<bool>,
    ) -> ChallengeSnapshot {
        if !self.config.enabled {
            return ChallengeSnapshot::default();
        }

        let Some(deaths) = deaths else {
            return self.snapshot();
        };
        let bosses = bosses.unwrap_or(0);
        let mut dirty = false;

        if let Some(reached) = start_flag {
            if reached != self.reached_start {
                self.reached_start = reached;
                if reached {
                    debug!(
                        deaths_on_start = deaths,
                        "Challenge run started (start flag set)"
                    );
                    self.persisted.deaths_on_start = deaths;
                } else {
                    debug!("Challenge run reset (start flag cleared)");
                    self.persisted.deaths_on_start = 0;
                    self.last_player_deaths = Some(0);
                    if deaths == 0 {
                        self.persisted.run_failed = false;
                    }
                }
                dirty = true;
            }
        }

        let run_deaths = self.run_deaths(deaths);
        if run_deaths > self.config.max_deaths {
            self.persisted.run_failed = true;
        }

        if self.last_player_deaths != Some(deaths) {
            self.last_player_deaths = Some(deaths);
            if run_deaths == self.config.max_deaths.saturating_add(1) {
                self.persisted.run_failed = true;
                self.persisted.tries = self.persisted.tries.saturating_add(1);
                debug!(
                    tries = self.persisted.tries,
                    run_deaths, "Challenge run failed (death limit exceeded)"
                );
                dirty = true;
            }
        }

        if self.reached_start
            && !self.persisted.run_failed
            && bosses > self.persisted.pb
            && run_deaths <= self.config.max_deaths
        {
            info!(
                old_pb = self.persisted.pb,
                new_pb = bosses,
                "Challenge personal best updated"
            );
            self.persisted.pb = bosses;
            dirty = true;
        }

        if dirty {
            self.mark_dirty();
        }

        self.snapshot()
    }

    fn run_deaths(&self, player_deaths: u32) -> u32 {
        if !self.reached_start {
            return 0;
        }
        player_deaths.saturating_sub(self.persisted.deaths_on_start)
    }

    /// Current challenge metrics without applying live game reads.
    pub fn snapshot(&self) -> ChallengeSnapshot {
        ChallengeSnapshot {
            enabled: self.config.enabled,
            pb: self.persisted.pb,
            tries: self.persisted.tries,
        }
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn flush(&mut self) {
        if !self.dirty {
            return;
        }
        if let Err(e) = write_challenge_state(&self.state_path, &self.persisted) {
            tracing::warn!("Failed to persist challenge state: {e:?}");
        } else {
            self.dirty = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_state_path() -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("er_overlay_challenge_test_{n}.toml"))
    }

    fn tracker(max_deaths: u32) -> ChallengeTracker {
        ChallengeTracker::new(
            ChallengeConfig {
                enabled: true,
                max_deaths,
                ..ChallengeConfig::default()
            },
            temp_state_path(),
        )
    }

    fn started(t: &mut ChallengeTracker, deaths: u32, bosses: u32) -> ChallengeSnapshot {
        t.update(Some(deaths), Some(bosses), Some(true))
    }

    #[test]
    fn disabled_returns_empty_snapshot() {
        let mut t = ChallengeTracker::new(ChallengeConfig::default(), temp_state_path());
        let snap = t.update(Some(0), Some(0), Some(true));
        assert!(!snap.enabled);
        assert_eq!(snap.pb, 0);
    }

    #[test]
    fn deaths_before_start_flag_are_ignored() {
        let mut t = tracker(0);
        let snap = t.update(Some(3), Some(2), Some(false));
        assert_eq!(snap.tries, 0);
        assert_eq!(snap.pb, 0);
    }

    #[test]
    fn pb_updates_while_within_budget() {
        let mut t = tracker(0);
        started(&mut t, 0, 3);
        let snap = started(&mut t, 0, 5);
        assert_eq!(snap.pb, 5);
    }

    #[test]
    fn deathless_increments_tries_once_on_first_death() {
        let mut t = tracker(0);
        started(&mut t, 0, 3);
        let failed = started(&mut t, 1, 3);
        assert_eq!(failed.pb, 3);
        assert_eq!(failed.tries, 1);

        let second_death = started(&mut t, 2, 4);
        assert_eq!(second_death.tries, 1);
        assert_eq!(second_death.pb, 3);
    }

    #[test]
    fn two_deaths_on_same_run_count_one_try() {
        let mut t = tracker(0);
        started(&mut t, 0, 1);
        started(&mut t, 1, 1);
        started(&mut t, 1, 2);
        let snap = started(&mut t, 2, 2);
        assert_eq!(snap.pb, 1);
        assert_eq!(snap.tries, 1);
    }

    #[test]
    fn pb_frozen_after_death_on_same_run() {
        let mut t = tracker(0);
        started(&mut t, 0, 1);
        started(&mut t, 1, 1);
        let snap = started(&mut t, 1, 2);
        assert_eq!(snap.pb, 1);
        assert_eq!(snap.tries, 1);
    }

    #[test]
    fn pb_stays_frozen_if_start_flag_rebaseline_after_fail() {
        let mut t = tracker(0);
        started(&mut t, 0, 1);
        started(&mut t, 1, 1);
        t.update(Some(1), Some(1), Some(false));
        let snap = t.update(Some(1), Some(2), Some(true));
        assert_eq!(snap.pb, 1);
        assert_eq!(snap.tries, 1);
    }

    #[test]
    fn max_deaths_allows_budget_before_fail() {
        let mut t = tracker(2);
        started(&mut t, 0, 0);
        started(&mut t, 1, 2);
        assert_eq!(started(&mut t, 2, 4).pb, 4);

        let failed = started(&mut t, 3, 5);
        assert_eq!(failed.tries, 1);
        assert_eq!(failed.pb, 4);
    }

    #[test]
    fn clearing_start_flag_resets_run_on_new_game() {
        let mut t = tracker(0);
        // Default `reached_start = true`; clear flag first so the next set rebaselines.
        t.update(Some(5), Some(2), Some(false));
        started(&mut t, 5, 2);
        started(&mut t, 6, 2);
        let reset = t.update(Some(0), Some(0), Some(false));
        assert_eq!(reset.tries, 1);
        assert_eq!(reset.pb, 2);
        let new_run = started(&mut t, 0, 3);
        assert_eq!(new_run.pb, 3);
    }

    #[test]
    fn persisted_state_roundtrip() {
        let path = temp_state_path();
        let state = ChallengePersisted {
            pb: 42,
            tries: 7,
            deaths_on_start: 10,
            run_failed: true,
        };
        write_challenge_state(&path, &state).unwrap();
        let loaded = load_challenge_state(&path);
        assert_eq!(loaded, state);
        let _ = fs::remove_file(path);
    }
}
