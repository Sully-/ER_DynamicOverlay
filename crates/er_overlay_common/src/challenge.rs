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
    /// Metric (relative to run start) that bounds a run. Reaching `budget_max` ends the run.
    /// Defaults to `deaths` (the classic EROverlay death budget).
    #[serde(default = "default_budget_metric")]
    pub budget_metric: String,
    /// Threshold on `budget_metric` that bounds a run. For a `max` PB it is a cap (the run fails
    /// when exceeded); for a `min` PB it is a goal (the run completes when reached).
    /// Historical `max_deaths` key is accepted as an alias.
    #[serde(default, alias = "max_deaths")]
    pub budget_max: u32,
    /// Event flag id that marks the start of a challenge run.
    #[serde(default = "default_start_flag")]
    pub start_flag: u32,
}

fn default_start_flag() -> u32 {
    DEFAULT_CHALLENGE_START_FLAG
}

fn default_budget_metric() -> String {
    "deaths".to_string()
}

/// Direction of the personal best: keep the highest (`Max`) or lowest (`Min`) observed value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PbDirection {
    #[default]
    Max,
    Min,
}

impl Default for ChallengeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            budget_metric: default_budget_metric(),
            budget_max: 0,
            start_flag: DEFAULT_CHALLENGE_START_FLAG,
        }
    }
}

/// Persisted challenge progress (survives overlay restarts). Mirrors EROverlay `Challenge.txt`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
struct ChallengePersisted {
    #[serde(default)]
    pb: u32,
    /// Whether `pb` holds a real observation (a `Min` PB is undefined until the first valid frame).
    #[serde(default)]
    has_pb: bool,
    /// Metric key the PB is computed on (e.g. `bosses`, `deaths`). Empty = legacy state.
    #[serde(default)]
    pb_source: String,
    /// Direction of the PB (`max` / `min`).
    #[serde(default)]
    pb_mode: PbDirection,
    /// Budget metric the PB was recorded against (change detection → reset PB).
    #[serde(default)]
    budget_metric: String,
    /// Budget threshold the PB was recorded against (change detection → reset PB).
    #[serde(default)]
    budget_max: u32,
    #[serde(default)]
    tries: u32,
    /// Budget metric reading captured when the current run started (baseline for relative values).
    #[serde(default, alias = "deaths_on_start")]
    budget_on_start: u32,
    #[serde(default)]
    run_failed: bool,
    /// Whether the PB was already recorded for the current run (`min` mode records once per run).
    #[serde(default)]
    pb_recorded_this_run: bool,
}

/// Challenge metrics for the overlay UI (`pb`, `nbtries` layout tiles).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChallengeSnapshot {
    pub enabled: bool,
    /// Personal best value for the configured metric, recorded while within the death budget.
    pub pb: u32,
    /// Whether `pb` holds a real value (a `Min` PB is `---` until the first valid observation).
    pub pb_available: bool,
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
    last_budget: Option<u32>,
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
            last_budget: None,
            dirty: false,
        }
    }

    pub fn sync_config(&mut self, config: &ChallengeConfig) {
        if self.config != *config {
            self.config = config.clone();
        }
    }

    /// Configure the PB source metric / direction (from the layout `pb` tile) and the budget
    /// metric / threshold (from `[challenge]`). Resets the PB when any of these change, since a
    /// stored value is meaningless for a different metric, direction, or budget.
    pub fn configure(
        &mut self,
        pb_source: &str,
        pb_mode: PbDirection,
        budget_metric: &str,
        budget_max: u32,
    ) {
        let unchanged = self.persisted.pb_source == pb_source
            && self.persisted.pb_mode == pb_mode
            && self.persisted.budget_metric == budget_metric
            && self.persisted.budget_max == budget_max;
        if unchanged {
            return;
        }
        let legacy = self.persisted.pb_source.is_empty() && self.persisted.budget_metric.is_empty();
        self.persisted.pb_source = pb_source.to_string();
        self.persisted.pb_mode = pb_mode;
        self.persisted.budget_metric = budget_metric.to_string();
        self.persisted.budget_max = budget_max;
        if legacy {
            // Upgrade from a state file without PB metadata: keep any existing value.
            self.persisted.has_pb = matches!(pb_mode, PbDirection::Max) || self.persisted.pb > 0;
        } else {
            self.persisted.pb = 0;
            self.persisted.has_pb = matches!(pb_mode, PbDirection::Max);
            self.persisted.pb_recorded_this_run = false;
        }
        self.mark_dirty();
    }

    /// Poll the budget metric, the PB metric, and the challenge start flag.
    /// `budget_value` / `pb_value` are the current readings of the configured metrics.
    pub fn update(
        &mut self,
        budget_value: Option<u32>,
        pb_value: Option<u32>,
        start_flag: Option<bool>,
    ) -> ChallengeSnapshot {
        if !self.config.enabled {
            return ChallengeSnapshot::default();
        }

        let Some(budget_value) = budget_value else {
            return self.snapshot();
        };
        let mut dirty = false;

        if let Some(reached) = start_flag {
            if reached != self.reached_start {
                self.reached_start = reached;
                if reached {
                    debug!(budget_on_start = budget_value, "Challenge run started");
                    self.persisted.budget_on_start = budget_value;
                    self.persisted.pb_recorded_this_run = false;
                } else {
                    debug!("Challenge run reset (start flag cleared)");
                    self.persisted.budget_on_start = 0;
                    self.last_budget = Some(0);
                    self.persisted.pb_recorded_this_run = false;
                    if budget_value == 0 {
                        self.persisted.run_failed = false;
                    }
                }
                dirty = true;
            }
        }

        let budget = self.run_budget(budget_value);
        let budget_max = self.config.budget_max;
        let crossed = self.last_budget != Some(budget);
        if crossed {
            self.last_budget = Some(budget);
        }

        match self.persisted.pb_mode {
            // Cap semantics: keep the highest PB reached while the run stays within budget.
            PbDirection::Max => {
                if budget > budget_max {
                    self.persisted.run_failed = true;
                }
                if crossed && budget == budget_max.saturating_add(1) {
                    self.persisted.run_failed = true;
                    self.persisted.tries = self.persisted.tries.saturating_add(1);
                    debug!(tries = self.persisted.tries, budget, "Challenge run failed");
                    dirty = true;
                }
                if self.reached_start && !self.persisted.run_failed && budget <= budget_max {
                    if let Some(value) = pb_value {
                        if !self.persisted.has_pb || value > self.persisted.pb {
                            info!(old_pb = self.persisted.pb, new_pb = value, "PB updated");
                            self.persisted.pb = value;
                            self.persisted.has_pb = true;
                            dirty = true;
                        }
                    }
                }
            }
            // Goal semantics: record the PB once, when the budget metric reaches its target.
            PbDirection::Min => {
                if self.reached_start
                    && !self.persisted.pb_recorded_this_run
                    && budget_max > 0
                    && budget >= budget_max
                {
                    if let Some(value) = pb_value {
                        self.persisted.pb_recorded_this_run = true;
                        self.persisted.tries = self.persisted.tries.saturating_add(1);
                        if !self.persisted.has_pb || value < self.persisted.pb {
                            info!(old_pb = self.persisted.pb, new_pb = value, "PB updated");
                            self.persisted.pb = value;
                            self.persisted.has_pb = true;
                        }
                        dirty = true;
                    }
                }
            }
        }

        if dirty {
            self.mark_dirty();
        }

        self.snapshot()
    }

    /// Budget metric value relative to the current run's start.
    fn run_budget(&self, budget_value: u32) -> u32 {
        if !self.reached_start {
            return 0;
        }
        budget_value.saturating_sub(self.persisted.budget_on_start)
    }

    /// Current challenge metrics without applying live game reads.
    pub fn snapshot(&self) -> ChallengeSnapshot {
        ChallengeSnapshot {
            enabled: self.config.enabled,
            pb: self.persisted.pb,
            pb_available: self.persisted.has_pb,
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
                budget_max: max_deaths,
                ..ChallengeConfig::default()
            },
            temp_state_path(),
        )
    }

    fn tracker_budget(budget_metric: &str, budget_max: u32) -> ChallengeTracker {
        ChallengeTracker::new(
            ChallengeConfig {
                enabled: true,
                budget_metric: budget_metric.to_string(),
                budget_max,
                ..ChallengeConfig::default()
            },
            temp_state_path(),
        )
    }

    /// Classic max/deaths challenge frame: budget = deaths, PB metric = bosses.
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
        assert!(snap.pb_available);
    }

    /// Goal challenge frame: budget = bosses, PB metric = deaths (fewest deaths to N bosses).
    fn goal(t: &mut ChallengeTracker, bosses: u32, deaths: u32) -> ChallengeSnapshot {
        t.update(Some(bosses), Some(deaths), Some(true))
    }

    #[test]
    fn pb_min_records_deaths_when_boss_goal_reached() {
        let mut t = tracker_budget("bosses", 3);
        t.configure("deaths", PbDirection::Min, "bosses", 3);
        // Min PB is undefined until the goal is reached.
        assert!(!goal(&mut t, 1, 4).pb_available);
        // Bosses reaches 3 with 5 deaths → record 5.
        let done = goal(&mut t, 3, 5);
        assert_eq!(done.pb, 5);
        assert!(done.pb_available);
        assert_eq!(done.tries, 1);
        // Deaths keep rising after the goal, but the PB stays frozen at the recorded value.
        assert_eq!(goal(&mut t, 3, 9).pb, 5);
    }

    #[test]
    fn pb_min_keeps_lowest_across_runs() {
        let mut t = tracker_budget("bosses", 2);
        t.configure("deaths", PbDirection::Min, "bosses", 2);
        assert_eq!(goal(&mut t, 2, 8).pb, 8);
        // New game: clear then set the start flag to rebaseline the run.
        t.update(Some(0), Some(0), Some(false));
        t.update(Some(0), Some(0), Some(true));
        let better = t.update(Some(2), Some(3), Some(true));
        assert_eq!(better.pb, 3);
        assert_eq!(better.tries, 2);
    }

    #[test]
    fn changing_pb_settings_resets_pb() {
        let mut t = tracker(3);
        t.configure("bosses", PbDirection::Max, "deaths", 3);
        assert_eq!(started(&mut t, 0, 7).pb, 7);
        t.configure("deaths", PbDirection::Min, "bosses", 3);
        let snap = t.snapshot();
        assert_eq!(snap.pb, 0);
        assert!(!snap.pb_available);
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
            has_pb: true,
            pb_source: "bosses".into(),
            pb_mode: PbDirection::Max,
            budget_metric: "deaths".into(),
            budget_max: 3,
            tries: 7,
            budget_on_start: 10,
            run_failed: true,
            pb_recorded_this_run: false,
        };
        write_challenge_state(&path, &state).unwrap();
        let loaded = load_challenge_state(&path);
        assert_eq!(loaded, state);
        let _ = fs::remove_file(path);
    }
}
