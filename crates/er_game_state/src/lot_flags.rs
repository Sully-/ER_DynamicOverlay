use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, LazyLock, RwLock};
use std::time::SystemTime;

use serde::Deserialize;
use tracing::{info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LotTable {
    Map,
    Enemy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub struct LotRef {
    pub table: LotTable,
    pub lot_id: u32,
    pub vanilla_flag: Option<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct LotFlagsData {
    pub regulation_sha256: Option<String>,
    pub map: HashMap<u32, u32>,
    pub enemy: HashMap<u32, u32>,
    pub goods: HashMap<String, u32>,
}

/// `None` means no seed mapping is loaded (vanilla, or no `regulation_path` configured):
/// dynamic lot refs then fall back to `vanilla_flag`.
static LOT_FLAGS: LazyLock<RwLock<Option<Arc<LotFlagsData>>>> =
    LazyLock::new(|| RwLock::new(None));

#[derive(Debug, Deserialize)]
struct LotFlagsFile {
    #[serde(default)]
    regulation_sha256: Option<String>,
    /// Legacy checks_flags.toml section. These are map lots.
    #[serde(default)]
    flags: HashMap<String, u32>,
    #[serde(default)]
    map: HashMap<String, u32>,
    #[serde(default)]
    enemy: HashMap<String, u32>,
    #[serde(default)]
    goods: HashMap<String, u32>,
}

pub fn lot_seed_flags() -> Option<Arc<LotFlagsData>> {
    LOT_FLAGS.read().expect("lot flags poisoned").clone()
}

pub fn lot_seed_flags_loaded() -> bool {
    LOT_FLAGS.read().expect("lot flags poisoned").is_some()
}

pub fn lot_seed_regulation_hash() -> Option<String> {
    lot_seed_flags().and_then(|d| d.regulation_sha256.clone())
}

pub fn parse_lot_flags(raw: &str) -> Result<LotFlagsData, String> {
    let file: LotFlagsFile = toml::from_str(raw).map_err(|e| e.to_string())?;
    let mut map = parse_flag_map("map", file.map)?;
    for (lot, flag) in parse_flag_map("flags", file.flags)? {
        map.insert(lot, flag);
    }
    Ok(LotFlagsData {
        regulation_sha256: file.regulation_sha256,
        map,
        enemy: parse_flag_map("enemy", file.enemy)?,
        goods: file.goods,
    })
}

fn parse_flag_map(name: &str, raw: HashMap<String, u32>) -> Result<HashMap<u32, u32>, String> {
    let mut flags = HashMap::with_capacity(raw.len());
    for (k, v) in raw {
        let lot: u32 = k
            .parse()
            .map_err(|_| format!("invalid lot_id key in lot flags [{name}]: {k}"))?;
        flags.insert(lot, v);
    }
    Ok(flags)
}

pub(crate) fn set_lot_flags(data: Option<LotFlagsData>) {
    *LOT_FLAGS.write().expect("lot flags poisoned") = data.map(Arc::new);
}

/// Drops any loaded seed mapping. Returns whether a mapping was actually cleared.
pub fn clear_lot_seed_flags() -> bool {
    let mut guard = LOT_FLAGS.write().expect("lot flags poisoned");
    if guard.is_some() {
        *guard = None;
        true
    } else {
        false
    }
}

/// Loads `lot_flags.toml` (or the legacy `checks_flags.toml`). On parse failure, keeps the
/// previous mapping.
pub fn load_lot_flags_from_path(path: &Path) -> bool {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(e) => {
            warn!("Failed to read lot flags at {}: {e}", path.display());
            return false;
        }
    };
    match parse_lot_flags(&raw) {
        Ok(data) => {
            let count = data.map.len() + data.enemy.len() + data.goods.len();
            set_lot_flags(Some(data));
            info!(
                "Loaded lot flags from {} ({} dynamic flags)",
                path.display(),
                count
            );
            true
        }
        Err(e) => {
            warn!(
                "Failed to parse lot flags at {}: {e} (keeping previous mapping)",
                path.display()
            );
            false
        }
    }
}

/// Reloads a generated lot-flags file when its mtime changes. When the file is absent, clears any
/// loaded mapping (so dynamic refs revert to vanilla flags). Returns whether state changed.
pub fn reload_lot_flags_if_modified(path: &Path, last_mtime: &mut Option<SystemTime>) -> bool {
    let mtime = fs::metadata(path).and_then(|m| m.modified()).ok();
    match mtime {
        Some(t) => {
            let changed = last_mtime.as_ref() != Some(&t);
            if !changed {
                return false;
            }
            if load_lot_flags_from_path(path) {
                *last_mtime = Some(t);
                true
            } else {
                false
            }
        }
        None => {
            if last_mtime.is_some() || lot_seed_flags_loaded() {
                set_lot_flags(None);
                *last_mtime = None;
                return true;
            }
            false
        }
    }
}

/// The effective event flag to read for a lot ref given the currently loaded seed mapping.
/// `None` for a lot that is untraceable this seed (its current item has no acquisition flag).
pub fn effective_lot_flag(lot: LotRef) -> Option<u32> {
    match lot_seed_flags() {
        Some(data) => {
            let flags = match lot.table {
                LotTable::Map => &data.map,
                LotTable::Enemy => &data.enemy,
            };
            flags.get(&lot.lot_id).copied()
        }
        None => lot.vanilla_flag,
    }
}

/// Effective flag for a historically tracked good. With a seed mapping loaded, goods are resolved
/// by item key because the runtime extractor has to find where that item was placed in this seed.
/// Without a seed mapping, fall back to the vanilla lot metadata from goods.toml.
pub fn effective_good_flag(key: &str, vanilla_lot: Option<LotRef>) -> Option<u32> {
    match lot_seed_flags() {
        Some(data) => data.goods.get(key).copied(),
        None => vanilla_lot.and_then(effective_lot_flag),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_lot_flags_supports_new_and_legacy_sections() {
        let raw = r#"
regulation_sha256 = "abc"

[flags]
100 = 200

[map]
300 = 400

[enemy]
500 = 600

[goods]
fire_scorpion_charm = 700
"#;
        let data = parse_lot_flags(raw).unwrap();
        assert_eq!(data.regulation_sha256.as_deref(), Some("abc"));
        assert_eq!(data.map.get(&100), Some(&200));
        assert_eq!(data.map.get(&300), Some(&400));
        assert_eq!(data.enemy.get(&500), Some(&600));
        assert_eq!(data.goods.get("fire_scorpion_charm"), Some(&700));
    }

    #[test]
    fn effective_lot_flag_prefers_seed_then_vanilla() {
        let lot = LotRef {
            table: LotTable::Map,
            lot_id: 100,
            vanilla_flag: Some(200),
        };
        set_lot_flags(None);
        assert_eq!(effective_lot_flag(lot), Some(200));

        let mut map = HashMap::new();
        map.insert(100, 300);
        set_lot_flags(Some(LotFlagsData {
            regulation_sha256: None,
            map,
            enemy: HashMap::new(),
            goods: HashMap::new(),
        }));
        assert_eq!(effective_lot_flag(lot), Some(300));

        set_lot_flags(Some(LotFlagsData::default()));
        assert_eq!(effective_lot_flag(lot), None);
        set_lot_flags(None);
    }

    #[test]
    fn effective_good_flag_prefers_seed_item_mapping_then_vanilla_lot() {
        let lot = LotRef {
            table: LotTable::Map,
            lot_id: 100,
            vanilla_flag: Some(200),
        };
        set_lot_flags(None);
        assert_eq!(effective_good_flag("fire_scorpion_charm", Some(lot)), Some(200));

        let mut goods = HashMap::new();
        goods.insert("fire_scorpion_charm".to_string(), 700);
        set_lot_flags(Some(LotFlagsData {
            regulation_sha256: None,
            map: HashMap::new(),
            enemy: HashMap::new(),
            goods,
        }));
        assert_eq!(effective_good_flag("fire_scorpion_charm", Some(lot)), Some(700));
        assert_eq!(effective_good_flag("missing", Some(lot)), None);
        set_lot_flags(None);
    }
}
