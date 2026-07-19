use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::challenge::ChallengeConfig;
use crate::panel_layout::parse_panel_layout;

/// Which bosses appear in the checklist panel. Region detection always follows the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BossPanelScope {
    /// Only bosses belonging to the player's current region.
    #[default]
    CurrentRegion,
    /// Every boss, grouped by region (scrollable).
    AllRegions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Anchor {
    TopLeft,
    #[default]
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayConfig {
    #[serde(default)]
    pub show_debug: bool,
    #[serde(default)]
    pub anchor: Anchor,
    #[serde(default)]
    pub offset_x: f32,
    #[serde(default = "default_offset_y")]
    pub offset_y: f32,
    #[serde(default = "default_scale")]
    pub scale: f32,
    #[serde(default = "default_text_size")]
    pub text_size: f32,
    #[serde(default = "default_icon_size")]
    pub icon_size: f32,
    #[serde(default = "default_gray_tint")]
    pub gray_tint: f32,
    #[serde(default = "default_true")]
    pub use_item_icons: bool,
    #[serde(default)]
    pub icons_dir: Option<String>,
    #[serde(default = "default_layout_file")]
    pub layout_file: Option<String>,
    /// Hotkey to cycle layout sections (e.g. `"F8"`, `"Ctrl+Shift+F1"`).
    #[serde(default)]
    pub layout_section_hotkey: Option<String>,
    /// Hotkey to toggle the regional boss checklist panel (e.g. `"F7"`).
    #[serde(default = "default_boss_panel_hotkey")]
    pub boss_panel_hotkey: Option<String>,
    /// Hotkey to show/hide the entire overlay (HUD + boss panel).
    #[serde(default)]
    pub hide_all_hotkey: Option<String>,
    /// Boss panel filter: `current-region` or `all-regions` (player location is always tracked).
    #[serde(default)]
    pub boss_panel_scope: BossPanelScope,
    /// In `all-regions` mode, keep every boss region expanded instead of only following current.
    #[serde(default)]
    pub boss_panel_expand_all_regions: bool,
    /// Show the boss panel when the overlay starts.
    #[serde(default = "default_true")]
    pub boss_panel_visible: bool,
    /// Boss panel placement: `x,y,width,height` (pixels or `%`). Use `auto` or omit to follow the HUD.
    #[serde(default)]
    pub boss_panel_layout: Option<String>,
    /// Initial layout section name; overrides `default_section` in the layout file.
    #[serde(default)]
    pub default_layout_section: Option<String>,
    /// Boss table language (`en`, `fr`, …). Use `auto` or omit to detect from the game.
    #[serde(default)]
    pub boss_locale: Option<String>,
    /// Path to the `regulation.bin` the game actually loads (ModEngine mod). When set, the
    /// overlay resolves randomized ground-loot checks per seed by running the checks extractor.
    /// Omit (or leave empty) for vanilla: dynamic checks then use their vanilla flags.
    #[serde(default)]
    pub regulation_path: Option<String>,
    /// Path to the `er_checks_extractor` executable. Omit to auto-locate next to the overlay
    /// (`companion/er_checks_extractor.exe`, then `er_checks_extractor.exe`).
    #[serde(default)]
    pub checks_extractor_path: Option<String>,
    /// Hotkey to toggle the checks (boss + loot) checklist panel (e.g. `"F6"`).
    #[serde(default = "default_checks_panel_hotkey")]
    pub checks_panel_hotkey: Option<String>,
    /// Checks panel filter: `current-region` or `all-regions` (player location is always tracked).
    #[serde(default)]
    pub checks_panel_scope: BossPanelScope,
    /// In `all-regions` mode, keep every checks region expanded instead of only following current.
    #[serde(default)]
    pub checks_panel_expand_all_regions: bool,
    /// Show the checks panel when the overlay starts.
    #[serde(default)]
    pub checks_panel_visible: bool,
    /// Checks panel placement: `x,y,width,height` (pixels or `%`). Use `auto` or omit for the
    /// default left-edge placement.
    #[serde(default)]
    pub checks_panel_layout: Option<String>,
    /// Show the overlay when the DLL starts (toggle at runtime with `hide_all_hotkey`).
    #[serde(default = "default_true")]
    pub overlay_visible: bool,
    /// Keep the focus on the overlay: capture mouse/keyboard input while the overlay is
    /// hovered/interactive and toggle the game mouse cursor accordingly. Disable to let the
    /// game keep the focus at all times (input is never intercepted by the overlay).
    #[serde(default = "default_true")]
    pub keep_overlay_focus: bool,
    /// Challenge mode (PB / failed runs), aligned with EROverlay boss challenge.
    #[serde(default)]
    pub challenge: ChallengeConfig,
    /// Write a diagnostic log file to `logs/er_overlay.log` next to the DLL. Off by default;
    /// enable it to troubleshoot injection/startup issues (e.g. the overlay not showing up).
    #[serde(default)]
    pub log_enabled: bool,
    /// Log verbosity filter. A level (`error`, `warn`, `info`, `debug`, `trace`) or a full
    /// `tracing` filter (e.g. `"info,er_overlay=debug"`). Omit for the default filter.
    /// The `RUST_LOG` environment variable, when set, always takes precedence.
    #[serde(default)]
    pub log_level: Option<String>,
}

fn default_true() -> bool {
    true
}

fn default_offset_y() -> f32 {
    16.0
}

fn default_scale() -> f32 {
    1.0
}

fn default_text_size() -> f32 {
    18.0
}

fn default_icon_size() -> f32 {
    24.0
}

fn default_gray_tint() -> f32 {
    0.40
}

fn default_layout_file() -> Option<String> {
    Some("layouts/dashboard.toml".into())
}

fn default_boss_panel_hotkey() -> Option<String> {
    Some("F7".into())
}

fn default_checks_panel_hotkey() -> Option<String> {
    Some("F6".into())
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            show_debug: false,
            anchor: Anchor::TopRight,
            offset_x: 16.0,
            offset_y: 16.0,
            scale: 1.0,
            text_size: 18.0,
            icon_size: 24.0,
            gray_tint: 0.40,
            use_item_icons: true,
            icons_dir: None,
            layout_file: default_layout_file(),
            layout_section_hotkey: None,
            boss_panel_hotkey: default_boss_panel_hotkey(),
            hide_all_hotkey: None,
            boss_panel_scope: BossPanelScope::default(),
            boss_panel_expand_all_regions: false,
            boss_panel_visible: true,
            boss_panel_layout: None,
            default_layout_section: None,
            boss_locale: None,
            regulation_path: None,
            checks_extractor_path: None,
            checks_panel_hotkey: default_checks_panel_hotkey(),
            checks_panel_scope: BossPanelScope::default(),
            checks_panel_expand_all_regions: false,
            checks_panel_visible: false,
            checks_panel_layout: None,
            overlay_visible: true,
            keep_overlay_focus: true,
            challenge: ChallengeConfig::default(),
            log_enabled: false,
            log_level: None,
        }
    }
}

impl OverlayConfig {
    pub fn validate_and_clamp(&mut self) {
        if self.scale <= 0.0 || self.scale > 4.0 {
            warn!("Invalid scale {}, clamping to 1.0", self.scale);
            self.scale = 1.0;
        }
        if self.text_size <= 0.0 || self.text_size > 72.0 {
            warn!("Invalid text_size {}, clamping to 18.0", self.text_size);
            self.text_size = 18.0;
        }
        if !(0.0..=1.0).contains(&self.gray_tint) {
            warn!("Invalid gray_tint {}, clamping to 0.40", self.gray_tint);
            self.gray_tint = 0.40;
        }
        if self.icon_size <= 0.0 || self.icon_size > 128.0 {
            warn!("Invalid icon_size {}, clamping to 24.0", self.icon_size);
            self.icon_size = 24.0;
        }
        if let Some(ref raw) = self.boss_panel_layout {
            if parse_panel_layout(raw).is_none() && !raw.trim().is_empty() {
                if !raw.trim().eq_ignore_ascii_case("auto") {
                    warn!(
                        "Invalid boss_panel_layout {:?}, falling back to auto placement",
                        raw
                    );
                }
                self.boss_panel_layout = None;
            }
        }
        if let Some(ref raw) = self.checks_panel_layout {
            if parse_panel_layout(raw).is_none() && !raw.trim().is_empty() {
                if !raw.trim().eq_ignore_ascii_case("auto") {
                    warn!(
                        "Invalid checks_panel_layout {:?}, falling back to auto placement",
                        raw
                    );
                }
                self.checks_panel_layout = None;
            }
        }
    }
}

impl BossPanelScope {
    pub fn parse_loose(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "all" | "all-regions" | "all_regions" | "allregions" => Self::AllRegions,
            _ => Self::CurrentRegion,
        }
    }
}

static OVERLAY_BASE_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Base directory for config, assets and logs when running inside an injected DLL.
/// Call once from DllMain before any other overlay init (falls back to `current_exe()`).
pub fn set_overlay_base_dir(dir: PathBuf) {
    let _ = OVERLAY_BASE_DIR.set(dir);
}

pub fn default_config_path() -> PathBuf {
    default_base_dir().join("er_overlay.toml")
}

pub fn default_base_dir() -> PathBuf {
    OVERLAY_BASE_DIR
        .get()
        .cloned()
        .unwrap_or_else(default_exe_dir)
}

pub fn default_exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn default_icons_dir() -> PathBuf {
    default_base_dir().join("assets/icons")
}

pub fn resolve_configured_path(configured: Option<&Path>, base: &Path) -> PathBuf {
    match configured {
        Some(path) if path.is_absolute() => path.to_path_buf(),
        Some(path) => base.join(path),
        None => default_icons_dir(),
    }
}

pub fn load_or_create_config(path: &Path) -> Result<OverlayConfig> {
    if path.exists() {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config at {}", path.display()))?;
        let mut config: OverlayConfig = toml::from_str(&raw)
            .with_context(|| format!("Failed to parse config at {}", path.display()))?;
        config.validate_and_clamp();
        info!("Loaded config from {}", path.display());
        Ok(config)
    } else {
        let config = OverlayConfig::default();
        write_config(path, &config)?;
        info!("Created default config at {}", path.display());
        Ok(config)
    }
}

pub fn write_config(path: &Path, config: &OverlayConfig) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let raw = toml::to_string_pretty(config).context("Failed to serialize config")?;
    fs::write(path, raw)
        .with_context(|| format!("Failed to write config to {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_roundtrip() {
        let raw = toml::to_string(&OverlayConfig::default()).unwrap();
        let parsed: OverlayConfig = toml::from_str(&raw).unwrap();
        assert_eq!(
            parsed.layout_file.as_deref(),
            Some("layouts/dashboard.toml")
        );
        assert_eq!(parsed.anchor, Anchor::TopRight);
    }

    #[test]
    fn clamp_invalid_values() {
        let mut cfg = OverlayConfig {
            scale: -1.0,
            text_size: 999.0,
            ..Default::default()
        };
        cfg.validate_and_clamp();
        assert_eq!(cfg.scale, 1.0);
        assert_eq!(cfg.text_size, 18.0);
    }
}
