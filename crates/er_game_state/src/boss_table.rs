use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, RwLock};
use std::time::SystemTime;

use serde::Deserialize;
use tracing::{info, warn};

#[cfg(feature = "game")]
use crate::game_language::detect_game_language;

#[cfg(not(feature = "game"))]
fn detect_game_language() -> String {
    DEFAULT_LOCALE_ID.to_string()
}

use crate::BossEntry;

const EMBEDDED_BOSSES_TOML: &str = include_str!("../tables/en/bosses.toml");

/// Default language when config is empty and game language cannot be detected.
pub const DEFAULT_LOCALE_ID: &str = "en";

#[derive(Debug, Clone)]
pub struct BossTableData {
    pub bosses: Vec<BossEntry>,
    pub region_names: Vec<String>,
    pub subregion_to_region: HashMap<u32, String>,
}

#[derive(Debug, Deserialize)]
struct BossTableFile {
    #[serde(default)]
    region_display_order: Vec<String>,
    #[serde(default)]
    region: Vec<RegionRow>,
    boss: Vec<BossRow>,
}

#[derive(Debug, Deserialize)]
struct RegionRow {
    name: String,
    subregions: Vec<u32>,
}

#[derive(Debug, Deserialize)]
struct BossRow {
    flag_id: u32,
    name: String,
    region: String,
    icon: String,
    #[serde(default)]
    place: Option<String>,
    #[serde(default)]
    dlc: bool,
}

static BOSS_STORE: LazyLock<RwLock<Arc<BossTableData>>> = LazyLock::new(|| {
    let data = parse_boss_table(EMBEDDED_BOSSES_TOML).expect("embedded en/bosses.toml must parse");
    RwLock::new(Arc::new(data))
});

static ACTIVE_LOCALE_ID: LazyLock<RwLock<String>> =
    LazyLock::new(|| RwLock::new(DEFAULT_LOCALE_ID.to_string()));

/// Resolves the language id (`en`, `fr`, …).
pub fn resolve_locale_id(config_locale: Option<&str>) -> String {
    match config_locale.map(str::trim).filter(|s| !s.is_empty()) {
        None | Some("auto") => detect_game_language(),
        Some(id) => normalize_locale_id(id),
    }
}

pub fn normalize_locale_id(raw: &str) -> String {
    match raw.trim().to_lowercase().as_str() {
        "" | "auto" => detect_game_language(),
        "en" | "english" | "eng" | "engus" => DEFAULT_LOCALE_ID.into(),
        "fr" | "french" | "fra" | "frafr" => "fr".into(),
        "de" | "german" | "deude" => "de".into(),
        "es" | "spanish" | "spaes" => "es".into(),
        "it" | "italian" | "itait" => "it".into(),
        "ja" | "japanese" | "jpnjp" => "ja".into(),
        "ko" | "koreana" | "korkr" => "ko".into(),
        "pl" | "polish" | "polpl" => "pl".into(),
        "pt" | "portuguese" | "porbr" => "pt".into(),
        "ru" | "russian" | "rusru" => "ru".into(),
        "zh-cn" | "schinese" | "zhocn" => "zh-cn".into(),
        "zh-tw" | "tchinese" | "zhotw" => "zh-tw".into(),
        other => other.to_string(),
    }
}

/// Path to `tables/<lang>/bosses.toml` relative to the DLL directory.
pub fn resolve_boss_table_path(base: &Path, locale_id: &str) -> PathBuf {
    base.join("tables").join(locale_id).join("bosses.toml")
}

/// Parses and validates a boss table TOML payload.
pub fn parse_boss_table(raw: &str) -> Result<BossTableData, String> {
    let table: BossTableFile = toml::from_str(raw).map_err(|e| e.to_string())?;
    if table.boss.is_empty() {
        return Err("boss table defines no [[boss]] entries".into());
    }

    let mut bosses = Vec::with_capacity(table.boss.len());
    let mut seen_flags = HashMap::new();
    for row in table.boss {
        if seen_flags.insert(row.flag_id, ()).is_some() {
            return Err(format!(
                "duplicate boss flag_id {} in boss table",
                row.flag_id
            ));
        }
        bosses.push(BossEntry {
            flag_id: row.flag_id,
            name: row.name,
            region: row.region,
            icon: row.icon,
            place: row.place,
            dlc: row.dlc,
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

    Ok(BossTableData {
        bosses,
        region_names,
        subregion_to_region,
    })
}

pub fn boss_table() -> Arc<BossTableData> {
    BOSS_STORE.read().expect("boss store poisoned").clone()
}

pub fn active_boss_locale() -> String {
    ACTIVE_LOCALE_ID.read().expect("locale id poisoned").clone()
}

pub fn bosses_total_count() -> usize {
    // Number of `[[boss]]` rows in the currently loaded table (disk hot-reload or embedded fallback).
    boss_table().bosses.len()
}

fn resolve_load_path(base: &Path, locale_id: &str, override_path: Option<&Path>) -> PathBuf {
    if let Some(path) = override_path.filter(|p| p.is_file()) {
        return path.to_path_buf();
    }
    if let Some(path) = override_path {
        warn!(
            "Boss table override missing at {} — using tables/{locale_id}/bosses.toml",
            path.display()
        );
    }

    let path = resolve_boss_table_path(base, locale_id);
    let fallback = resolve_boss_table_path(base, DEFAULT_LOCALE_ID);
    if path.is_file() {
        path
    } else if locale_id != DEFAULT_LOCALE_ID && fallback.is_file() {
        warn!(
            "Boss table missing at {} (locale '{locale_id}'), falling back to {}",
            path.display(),
            fallback.display()
        );
        fallback
    } else {
        path
    }
}

fn apply_boss_table(data: BossTableData, locale_id: &str) {
    *BOSS_STORE.write().expect("boss store poisoned") = Arc::new(data);
    *ACTIVE_LOCALE_ID.write().expect("locale id poisoned") = locale_id.to_string();
}

/// Loads a boss table from disk. On failure, keeps the current in-memory table.
pub fn load_boss_table_from_path(locale_id: &str, path: &Path) -> bool {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(e) => {
            warn!("Failed to read boss table at {}: {e}", path.display());
            return false;
        }
    };

    match parse_boss_table(&raw) {
        Ok(data) => {
            let count = data.bosses.len();
            apply_boss_table(data, locale_id);
            info!(
                "Loaded boss table '{}' from {} ({} bosses)",
                locale_id,
                path.display(),
                count
            );
            true
        }
        Err(e) => {
            warn!(
                "Failed to parse boss table at {}: {e} (keeping previous table)",
                path.display()
            );
            false
        }
    }
}

/// Reloads when the language or file mtime changes. Falls back to `tables/en/bosses.toml`.
pub fn reload_boss_table_if_modified(
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
        if using_override || load_path == resolve_boss_table_path(base, &locale_id) {
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
            warn!(
                "Boss table file missing at {} (locale '{}'), using embedded table",
                load_path.display(),
                effective_locale
            );
            *active_locale = Some(effective_locale.to_string());
            *last_mtime = None;
            let data = parse_boss_table(EMBEDDED_BOSSES_TOML).expect("embedded en/bosses.toml");
            apply_boss_table(data, DEFAULT_LOCALE_ID);
            return true;
        }
        return false;
    }

    if load_boss_table_from_path(effective_locale, &load_path) {
        *active_locale = Some(effective_locale.to_string());
        *last_mtime = mtime;
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_boss_table_counts_entries() {
        let raw = r#"
[[boss]]
flag_id = 1
name = "A"
region = "Limgrave"
icon = "a"

[[boss]]
flag_id = 2
name = "B"
region = "Limgrave"
icon = "b"
"#;
        assert_eq!(parse_boss_table(raw).unwrap().bosses.len(), 2);
    }

    #[test]
    fn embedded_table_parses() {
        let data = parse_boss_table(EMBEDDED_BOSSES_TOML).unwrap();
        assert_eq!(data.bosses.len(), 207);
        assert!(!data.region_names.is_empty());
        assert_eq!(
            data.subregion_to_region.get(&6100).map(String::as_str),
            Some("Limgrave")
        );
    }

    #[test]
    fn normalize_locale_aliases() {
        assert_eq!(normalize_locale_id("french"), "fr");
        assert_eq!(normalize_locale_id("frafr"), "fr");
        assert_eq!(normalize_locale_id("engUS"), DEFAULT_LOCALE_ID);
    }

    #[test]
    fn resolve_boss_table_path_layout() {
        let path = resolve_boss_table_path(Path::new("C:/game"), "fr");
        assert_eq!(path, Path::new("C:/game/tables/fr/bosses.toml"));
    }
}
