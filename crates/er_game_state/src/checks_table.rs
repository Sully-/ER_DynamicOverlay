//! Checks: a unified "is this validated?" concept (boss killed OR item looted).
//!
//! A check resolves to an event flag read via [`GameStateSource::get_flag`]. Two kinds:
//! - fixed (`dynamic = false`): the flag is stable across seeds (bosses, chests, events,
//!   non-relocated items). Read directly from `flag`.
//! - dynamic (`dynamic = true`): a ground loot the item randomizer relocates, so the flag
//!   changes per seed. We anchor on the stable `lot_id` (an `ItemLotParam` row) and read the
//!   current `getItemFlagId` from `checks_flags.toml` (generated per seed by er_checks_extractor).
//!   Falls back to `vanilla_flag` when no seed mapping is loaded (vanilla / no regulation).
//!
//! `checks.toml` is hot-reloaded like `bosses.toml`; `checks_flags.toml` is reloaded whenever
//! the extractor regenerates it.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, RwLock};
use std::time::SystemTime;

use serde::Deserialize;
use tracing::{info, warn};

use crate::boss_table::DEFAULT_LOCALE_ID;

const EMBEDDED_CHECKS_TOML: &str = include_str!("../tables/en/checks.toml");

/// Which `ItemLotParam` table a dynamic check's `lot_id` lives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LotParam {
    /// `ItemLotParam_map` — world treasure / ground loot.
    #[default]
    Map,
    /// `ItemLotParam_enemy` — enemy drops.
    Enemy,
}

#[derive(Debug, Clone)]
pub struct CheckEntry {
    pub name: String,
    pub place: Option<String>,
    pub region: String,
    pub dlc: bool,
    /// Whether the flag must be resolved per seed from the regulation (relocated ground loot).
    pub dynamic: bool,
    /// Stable anchor for dynamic checks: the `ItemLotParam` row id.
    pub lot_id: Option<u32>,
    pub lot_param: LotParam,
    /// Vanilla `getItemFlagId` of the lot; fallback when no seed mapping is loaded.
    pub vanilla_flag: Option<u32>,
    /// Fixed flag for non-dynamic checks.
    pub flag: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct ChecksTableData {
    pub checks: Vec<CheckEntry>,
    pub region_names: Vec<String>,
    pub subregion_to_region: HashMap<u32, String>,
}

#[derive(Debug, Deserialize)]
struct ChecksTableFile {
    #[serde(default)]
    region_display_order: Vec<String>,
    #[serde(default)]
    region: Vec<RegionRow>,
    #[serde(default)]
    check: Vec<CheckRow>,
}

#[derive(Debug, Deserialize)]
struct RegionRow {
    name: String,
    subregions: Vec<u32>,
}

#[derive(Debug, Deserialize)]
struct CheckRow {
    name: String,
    region: String,
    #[serde(default)]
    place: Option<String>,
    #[serde(default)]
    dlc: bool,
    #[serde(default)]
    dynamic: bool,
    #[serde(default)]
    lot_id: Option<u32>,
    #[serde(default)]
    lot_param: LotParam,
    #[serde(default)]
    vanilla_flag: Option<u32>,
    #[serde(default)]
    flag: Option<u32>,
}

static CHECKS_STORE: LazyLock<RwLock<Arc<ChecksTableData>>> = LazyLock::new(|| {
    let data = parse_checks_table(EMBEDDED_CHECKS_TOML).expect("embedded en/checks.toml must parse");
    RwLock::new(Arc::new(data))
});

static ACTIVE_CHECKS_LOCALE: LazyLock<RwLock<String>> =
    LazyLock::new(|| RwLock::new(DEFAULT_LOCALE_ID.to_string()));

/// Parses and validates a checks table TOML payload.
pub fn parse_checks_table(raw: &str) -> Result<ChecksTableData, String> {
    let table: ChecksTableFile = toml::from_str(raw).map_err(|e| e.to_string())?;

    let mut checks = Vec::with_capacity(table.check.len());
    for row in table.check {
        if row.dynamic && row.lot_id.is_none() && row.vanilla_flag.is_none() {
            return Err(format!(
                "dynamic check '{}' has neither lot_id nor vanilla_flag",
                row.name
            ));
        }
        if !row.dynamic && row.flag.is_none() {
            return Err(format!("fixed check '{}' has no flag", row.name));
        }
        checks.push(CheckEntry {
            name: row.name,
            place: row.place,
            region: row.region,
            dlc: row.dlc,
            dynamic: row.dynamic,
            lot_id: row.lot_id,
            lot_param: row.lot_param,
            vanilla_flag: row.vanilla_flag,
            flag: row.flag,
        });
    }

    let region_names = if !table.region_display_order.is_empty() {
        table.region_display_order
    } else {
        table.region.iter().map(|r| r.name.clone()).collect()
    };

    let mut subregion_to_region = HashMap::new();
    for row in table.region {
        for sid in row.subregions {
            subregion_to_region.insert(sid, row.name.clone());
        }
    }

    Ok(ChecksTableData {
        checks,
        region_names,
        subregion_to_region,
    })
}

pub fn checks_table() -> Arc<ChecksTableData> {
    CHECKS_STORE.read().expect("checks store poisoned").clone()
}

pub fn active_checks_locale() -> String {
    ACTIVE_CHECKS_LOCALE
        .read()
        .expect("checks locale poisoned")
        .clone()
}

pub fn checks_total_count() -> usize {
    checks_table().checks.len()
}

/// Checks whose `region` label matches `region`, in table order.
pub fn checks_in_region(region: &str) -> Vec<CheckEntry> {
    checks_table()
        .checks
        .iter()
        .filter(|c| c.region == region)
        .cloned()
        .collect()
}

/// Region labels in checks.toml display order.
pub fn checks_region_names() -> Vec<String> {
    checks_table().region_names.clone()
}

/// Resolves the checks.toml region label for a live map id (`map_id / 1000`, with fallback).
pub fn checks_region_label_for_subregion(map_id: u32) -> Option<String> {
    let table = checks_table();
    let key = map_id / 1000;
    table
        .subregion_to_region
        .get(&key)
        .or_else(|| table.subregion_to_region.get(&map_id))
        .cloned()
}

pub fn resolve_checks_table_path(base: &Path, locale_id: &str) -> PathBuf {
    base.join("tables").join(locale_id).join("checks.toml")
}

fn resolve_load_path(base: &Path, locale_id: &str, override_path: Option<&Path>) -> PathBuf {
    if let Some(path) = override_path.filter(|p| p.is_file()) {
        return path.to_path_buf();
    }
    let path = resolve_checks_table_path(base, locale_id);
    let fallback = resolve_checks_table_path(base, DEFAULT_LOCALE_ID);
    if path.is_file() {
        path
    } else if locale_id != DEFAULT_LOCALE_ID && fallback.is_file() {
        fallback
    } else {
        path
    }
}

fn apply_checks_table(data: ChecksTableData, locale_id: &str) {
    *CHECKS_STORE.write().expect("checks store poisoned") = Arc::new(data);
    *ACTIVE_CHECKS_LOCALE
        .write()
        .expect("checks locale poisoned") = locale_id.to_string();
}

/// Loads a checks table from disk. On failure, keeps the current in-memory table.
pub fn load_checks_table_from_path(locale_id: &str, path: &Path) -> bool {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(e) => {
            warn!("Failed to read checks table at {}: {e}", path.display());
            return false;
        }
    };

    match parse_checks_table(&raw) {
        Ok(data) => {
            let count = data.checks.len();
            apply_checks_table(data, locale_id);
            info!(
                "Loaded checks table '{}' from {} ({} checks)",
                locale_id,
                path.display(),
                count
            );
            true
        }
        Err(e) => {
            warn!(
                "Failed to parse checks table at {}: {e} (keeping previous table)",
                path.display()
            );
            false
        }
    }
}

/// Reloads when the language or file mtime changes. Falls back to `tables/en/checks.toml`,
/// then the embedded table.
pub fn reload_checks_table_if_modified(
    base: &Path,
    locale_id: &str,
    override_path: Option<&Path>,
    last_mtime: &mut Option<SystemTime>,
    active_locale: &mut Option<String>,
) -> bool {
    let locale_id = if locale_id.is_empty() {
        DEFAULT_LOCALE_ID.to_string()
    } else {
        locale_id.to_string()
    };

    let load_path = resolve_load_path(base, &locale_id, override_path);
    let using_override = override_path.is_some_and(|p| p.is_file() && load_path == p);
    let effective_locale =
        if using_override || load_path == resolve_checks_table_path(base, &locale_id) {
            locale_id.as_str()
        } else {
            DEFAULT_LOCALE_ID
        };

    let locale_changed = active_locale.as_deref() != Some(effective_locale);
    let mtime = fs::metadata(&load_path).and_then(|m| m.modified()).ok();
    let file_changed = match (mtime, last_mtime.as_ref()) {
        (Some(t), Some(prev)) => t != *prev,
        (Some(_), None) => true,
        _ => locale_changed,
    };

    if !locale_changed && !file_changed {
        return false;
    }

    if !load_path.is_file() {
        if locale_changed {
            *active_locale = Some(effective_locale.to_string());
            *last_mtime = None;
            let data =
                parse_checks_table(EMBEDDED_CHECKS_TOML).expect("embedded en/checks.toml");
            apply_checks_table(data, DEFAULT_LOCALE_ID);
            return true;
        }
        return false;
    }

    if load_checks_table_from_path(effective_locale, &load_path) {
        *active_locale = Some(effective_locale.to_string());
        *last_mtime = mtime;
        true
    } else {
        false
    }
}

// --- Per-seed flag mapping (checks_flags.toml) -----------------------------------------

/// Per-seed mapping `lot_id -> current event flag`, plus the regulation hash it came from.
#[derive(Debug, Clone, Default)]
pub struct ChecksFlagsData {
    pub regulation_sha256: Option<String>,
    pub flags: HashMap<u32, u32>,
}

/// `None` means no seed mapping is loaded (vanilla, or no `regulation_path` configured):
/// dynamic checks then fall back to `vanilla_flag`.
static CHECKS_FLAGS: LazyLock<RwLock<Option<Arc<ChecksFlagsData>>>> =
    LazyLock::new(|| RwLock::new(None));

#[derive(Debug, Deserialize)]
struct ChecksFlagsFile {
    #[serde(default)]
    regulation_sha256: Option<String>,
    #[serde(default)]
    flags: HashMap<String, u32>,
}

pub fn checks_seed_flags() -> Option<Arc<ChecksFlagsData>> {
    CHECKS_FLAGS.read().expect("checks flags poisoned").clone()
}

pub fn checks_seed_flags_loaded() -> bool {
    CHECKS_FLAGS.read().expect("checks flags poisoned").is_some()
}

pub fn checks_seed_regulation_hash() -> Option<String> {
    checks_seed_flags().and_then(|d| d.regulation_sha256.clone())
}

pub fn parse_checks_flags(raw: &str) -> Result<ChecksFlagsData, String> {
    let file: ChecksFlagsFile = toml::from_str(raw).map_err(|e| e.to_string())?;
    let mut flags = HashMap::with_capacity(file.flags.len());
    for (k, v) in file.flags {
        let lot: u32 = k
            .parse()
            .map_err(|_| format!("invalid lot_id key in checks_flags.toml: {k}"))?;
        flags.insert(lot, v);
    }
    Ok(ChecksFlagsData {
        regulation_sha256: file.regulation_sha256,
        flags,
    })
}

fn set_checks_flags(data: Option<ChecksFlagsData>) {
    *CHECKS_FLAGS.write().expect("checks flags poisoned") = data.map(Arc::new);
}

/// Drops any loaded seed mapping (e.g. `regulation_path` was unset): dynamic checks revert to
/// their vanilla flags. Returns whether a mapping was actually cleared.
pub fn clear_checks_seed_flags() -> bool {
    let mut guard = CHECKS_FLAGS.write().expect("checks flags poisoned");
    if guard.is_some() {
        *guard = None;
        true
    } else {
        false
    }
}

/// Loads `checks_flags.toml`. On parse failure, keeps the previous mapping.
pub fn load_checks_flags_from_path(path: &Path) -> bool {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(e) => {
            warn!("Failed to read checks_flags at {}: {e}", path.display());
            return false;
        }
    };
    match parse_checks_flags(&raw) {
        Ok(data) => {
            let count = data.flags.len();
            set_checks_flags(Some(data));
            info!(
                "Loaded checks_flags from {} ({} dynamic flags)",
                path.display(),
                count
            );
            true
        }
        Err(e) => {
            warn!(
                "Failed to parse checks_flags at {}: {e} (keeping previous mapping)",
                path.display()
            );
            false
        }
    }
}

/// Reloads `checks_flags.toml` when its mtime changes. When the file is absent, clears any
/// loaded mapping (so dynamic checks revert to vanilla flags). Returns whether state changed.
pub fn reload_checks_flags_if_modified(path: &Path, last_mtime: &mut Option<SystemTime>) -> bool {
    let mtime = fs::metadata(path).and_then(|m| m.modified()).ok();
    match mtime {
        Some(t) => {
            let changed = last_mtime.as_ref() != Some(&t);
            if !changed {
                return false;
            }
            if load_checks_flags_from_path(path) {
                *last_mtime = Some(t);
                true
            } else {
                false
            }
        }
        None => {
            // File gone: drop the mapping once.
            if last_mtime.is_some() || checks_seed_flags_loaded() {
                set_checks_flags(None);
                *last_mtime = None;
                return true;
            }
            false
        }
    }
}

/// The effective event flag to read for a check given the currently loaded seed mapping.
/// `None` for a dynamic check that is untraceable this seed (its lot got a flagless item).
pub fn effective_flag(check: &CheckEntry) -> Option<u32> {
    if !check.dynamic {
        return check.flag;
    }
    match checks_seed_flags() {
        Some(data) => match check.lot_id {
            // Lot present in the seed mapping -> current flag; absent -> untraceable this seed.
            Some(lot) => data.flags.get(&lot).copied(),
            None => check.vanilla_flag,
        },
        // No seed mapping loaded (vanilla / no regulation): use the vanilla lot flag.
        None => check.vanilla_flag,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_checks_table_parses() {
        let data = parse_checks_table(EMBEDDED_CHECKS_TOML).unwrap();
        assert!(!data.checks.is_empty());
        assert!(!data.region_names.is_empty());
        assert!(data.checks.iter().any(|c| c.dynamic));
    }

    #[test]
    fn academy_key_is_dynamic_with_lot() {
        let data = parse_checks_table(EMBEDDED_CHECKS_TOML).unwrap();
        let key = data
            .checks
            .iter()
            .find(|c| c.name == "Academy Glintstone Key")
            .expect("Academy Glintstone Key check");
        assert!(key.dynamic);
        assert_eq!(key.lot_id, Some(1034450100));
        assert_eq!(key.vanilla_flag, Some(1034457100));
        assert_eq!(key.lot_param, LotParam::Map);
    }

    #[test]
    fn dynamic_check_resolution_prefers_seed_then_vanilla() {
        let check = CheckEntry {
            name: "X".into(),
            place: None,
            region: "R".into(),
            dlc: false,
            dynamic: true,
            lot_id: Some(1034450100),
            lot_param: LotParam::Map,
            vanilla_flag: Some(1034457100),
            flag: None,
        };

        // No seed mapping -> vanilla flag.
        set_checks_flags(None);
        assert_eq!(effective_flag(&check), Some(1034457100));

        // Seed mapping present with the lot -> seed flag.
        let mut flags = HashMap::new();
        flags.insert(1034450100u32, 65160u32);
        set_checks_flags(Some(ChecksFlagsData {
            regulation_sha256: Some("abc".into()),
            flags,
        }));
        assert_eq!(effective_flag(&check), Some(65160));

        // Seed mapping present but lot absent -> untraceable this seed.
        set_checks_flags(Some(ChecksFlagsData::default()));
        assert_eq!(effective_flag(&check), None);

        // Reset global state for other tests.
        set_checks_flags(None);
    }

    #[test]
    fn fixed_check_uses_flag() {
        let check = CheckEntry {
            name: "Boss".into(),
            place: None,
            region: "R".into(),
            dlc: false,
            dynamic: false,
            lot_id: None,
            lot_param: LotParam::Map,
            vanilla_flag: None,
            flag: Some(10000800),
        };
        assert_eq!(effective_flag(&check), Some(10000800));
    }

    #[test]
    fn parse_checks_flags_numeric_keys() {
        let raw = "regulation_sha256 = \"deadbeef\"\n[flags]\n1034450100 = 65160\n10000500 = 1\n";
        let data = parse_checks_flags(raw).unwrap();
        assert_eq!(data.regulation_sha256.as_deref(), Some("deadbeef"));
        assert_eq!(data.flags.get(&1034450100), Some(&65160));
    }
}
