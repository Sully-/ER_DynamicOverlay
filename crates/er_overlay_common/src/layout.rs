use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridConfig {
    #[serde(default = "default_columns")]
    pub columns: u32,
    /// Size of one unit (standard square for an icon).
    #[serde(default = "default_unit_size")]
    pub unit_size: f32,
    /// Deprecated: use `unit_size`. If present, overrides unit width.
    #[serde(default)]
    pub cell_width: Option<f32>,
    /// Deprecated: use `unit_size`. If present, overrides unit height.
    #[serde(default)]
    pub cell_height: Option<f32>,
    #[serde(default)]
    pub gap: f32,
    #[serde(default = "default_border_radius")]
    pub border_radius: f32,
    /// Inner padding of the overlay window around the grid (px, before scale).
    #[serde(default = "default_window_padding")]
    pub window_padding: f32,
}

fn default_columns() -> u32 {
    8
}

fn default_unit_size() -> f32 {
    48.0
}

fn default_border_radius() -> f32 {
    6.0
}

fn default_window_padding() -> f32 {
    8.0
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            columns: default_columns(),
            unit_size: default_unit_size(),
            cell_width: None,
            cell_height: None,
            gap: 0.0,
            border_radius: default_border_radius(),
            window_padding: default_window_padding(),
        }
    }
}

impl GridConfig {
    pub fn unit_width(&self) -> f32 {
        self.cell_width.unwrap_or(self.unit_size)
    }

    pub fn unit_height(&self) -> f32 {
        self.cell_height.unwrap_or(self.unit_size)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutStyle {
    #[serde(default = "default_border_default")]
    pub border_default: [u8; 4],
    #[serde(default = "default_border_complete")]
    pub border_complete: [u8; 4],
    #[serde(default = "default_tile_bg")]
    pub tile_bg: [u8; 4],
    #[serde(default = "default_label_scale")]
    pub label_scale: f32,
    #[serde(default = "default_value_scale")]
    pub value_scale: f32,
}

fn default_border_default() -> [u8; 4] {
    [100, 100, 110, 200]
}

fn default_border_complete() -> [u8; 4] {
    [60, 200, 90, 255]
}

fn default_tile_bg() -> [u8; 4] {
    [12, 12, 18, 180]
}

fn default_label_scale() -> f32 {
    0.55
}

fn default_value_scale() -> f32 {
    1.0
}

impl Default for LayoutStyle {
    fn default() -> Self {
        Self {
            border_default: default_border_default(),
            border_complete: default_border_complete(),
            tile_bg: default_tile_bg(),
            label_scale: default_label_scale(),
            value_scale: default_value_scale(),
        }
    }
}

/// Position and size in grid units.
/// `w` / `h` are aliases for `col_span` / `row_span`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TilePosition {
    pub col: u32,
    pub row: u32,
    #[serde(default = "default_span", alias = "w")]
    pub col_span: u32,
    #[serde(default = "default_span", alias = "h")]
    pub row_span: u32,
}

fn default_span() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TileDef {
    /// Counter, time, or aggregate. `metric` is a built-in id (`igt`, `deaths`, `ng_cycle`,
    /// `bosses`), an aggregate group name from `goods.toml`, or a single good key (quantity).
    Metric {
        #[serde(default)]
        id: Option<String>,
        metric: String,
        #[serde(flatten)]
        position: TilePosition,
        #[serde(default)]
        label: String,
        #[serde(default)]
        show_max: bool,
        /// PNG key in `assets/icons` (e.g. `kindling`). If absent, no icon.
        #[serde(default)]
        icon: Option<String>,
    },
    /// A tracked good or boss — unique key (`goods.toml` or `bosses.toml`).
    Item {
        #[serde(default)]
        id: Option<String>,
        #[serde(rename = "key")]
        good_key: String,
        #[serde(flatten)]
        position: TilePosition,
    },
    /// Decorative text (header, separator…) — no game data.
    Label {
        #[serde(default)]
        id: Option<String>,
        #[serde(flatten)]
        position: TilePosition,
        #[serde(default)]
        label: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutSection {
    pub name: String,
    #[serde(rename = "tile", default)]
    pub tiles: Vec<TileDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    #[serde(default)]
    pub grid: GridConfig,
    #[serde(default)]
    pub style: LayoutStyle,
    #[serde(default)]
    pub default_section: Option<String>,
    /// Flat tile list — forms an implicit `"default"` section when `sections` is empty.
    #[serde(rename = "tile", default)]
    pub tiles: Vec<TileDef>,
    #[serde(default, rename = "section")]
    pub sections: Vec<LayoutSection>,
}

impl LayoutConfig {
    pub fn section_count(&self) -> usize {
        if self.sections.is_empty() {
            if self.tiles.is_empty() {
                0
            } else {
                1
            }
        } else {
            self.sections.len()
        }
    }

    pub fn section_name(&self, index: usize) -> Option<&str> {
        if self.sections.is_empty() {
            if index == 0 && !self.tiles.is_empty() {
                Some("default")
            } else {
                None
            }
        } else {
            self.sections.get(index).map(|s| s.name.as_str())
        }
    }

    pub fn section_index(&self, name: &str) -> Option<usize> {
        if self.sections.is_empty() {
            if name == "default" && !self.tiles.is_empty() {
                Some(0)
            } else {
                None
            }
        } else {
            self.sections.iter().position(|s| s.name == name)
        }
    }

    pub fn resolve_default_section_index(&self, config_default: Option<&str>) -> usize {
        if let Some(name) = config_default {
            if let Some(idx) = self.section_index(name) {
                return idx;
            }
        }
        if let Some(name) = &self.default_section {
            if let Some(idx) = self.section_index(name) {
                return idx;
            }
        }
        0
    }

    /// Clamp `index`, fall back to the default section if it has no tiles.
    pub fn resolve_section_tiles(
        &self,
        mut index: usize,
        config_default: Option<&str>,
    ) -> (usize, &[TileDef]) {
        let count = self.section_count();
        if count == 0 {
            return (0, &[]);
        }
        if index >= count {
            index = 0;
        }
        let tiles = self.tiles_for_section(index);
        if tiles.is_empty() {
            index = self
                .resolve_default_section_index(config_default)
                .min(count - 1);
            return (index, self.tiles_for_section(index));
        }
        (index, tiles)
    }

    pub fn tiles_for_section(&self, index: usize) -> &[TileDef] {
        if self.sections.is_empty() {
            if index == 0 {
                &self.tiles
            } else {
                &[]
            }
        } else {
            self.sections
                .get(index)
                .map(|s| s.tiles.as_slice())
                .unwrap_or(&[])
        }
    }

    pub fn section_names(&self) -> Vec<&str> {
        if self.sections.is_empty() {
            if self.tiles.is_empty() {
                vec![]
            } else {
                vec!["default"]
            }
        } else {
            self.sections.iter().map(|s| s.name.as_str()).collect()
        }
    }

    /// PNG keys referenced by layout tiles (`item` keys + metric `icon` fields).
    pub fn collect_icon_keys(&self) -> Vec<String> {
        let mut keys = Vec::new();
        let mut push = |key: &str| {
            if !key.is_empty() && !keys.iter().any(|k| k == key) {
                keys.push(key.to_string());
            }
        };

        if self.sections.is_empty() {
            Self::collect_icon_keys_from_tiles(&self.tiles, &mut push);
        } else {
            for section in &self.sections {
                Self::collect_icon_keys_from_tiles(&section.tiles, &mut push);
            }
        }
        keys
    }

    fn collect_icon_keys_from_tiles(tiles: &[TileDef], push: &mut impl FnMut(&str)) {
        for tile in tiles {
            match tile {
                TileDef::Item { good_key, .. } => push(good_key),
                TileDef::Metric { icon, .. } => {
                    if let Some(key) = icon.as_deref() {
                        push(key);
                    }
                }
                TileDef::Label { .. } => {}
            }
        }
    }

    /// Data keys referenced by tiles: `item` good keys + `metric` refs (built-ins, group names,
    /// good keys). The view model resolves which are goods vs groups vs built-ins.
    pub fn collect_data_refs(&self) -> Vec<String> {
        let mut keys = Vec::new();
        let mut push = |key: &str| {
            if !key.is_empty() && !keys.iter().any(|k| k == key) {
                keys.push(key.to_string());
            }
        };

        if self.sections.is_empty() {
            Self::collect_data_refs_from_tiles(&self.tiles, &mut push);
        } else {
            for section in &self.sections {
                Self::collect_data_refs_from_tiles(&section.tiles, &mut push);
            }
        }
        keys
    }

    fn collect_data_refs_from_tiles(tiles: &[TileDef], push: &mut impl FnMut(&str)) {
        for tile in tiles {
            match tile {
                TileDef::Item { good_key, .. } => push(good_key),
                TileDef::Metric { metric, .. } => push(metric),
                TileDef::Label { .. } => {}
            }
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.grid.columns == 0 {
            bail!("grid.columns must be > 0");
        }
        if self.grid.unit_width() <= 0.0 {
            bail!("grid unit width must be > 0");
        }
        if self.grid.unit_height() <= 0.0 {
            bail!("grid unit height must be > 0");
        }
        if self.grid.gap < 0.0 {
            bail!("grid.gap must be >= 0");
        }
        if self.grid.window_padding < 0.0 {
            bail!("grid.window_padding must be >= 0");
        }

        if !self.sections.is_empty() && !self.tiles.is_empty() {
            warn!(
                "Layout defines both [[tile]] and [[section]]; flat [[tile]] entries are ignored"
            );
        }

        if self.sections.is_empty() {
            if self.tiles.is_empty() {
                bail!("layout must define at least one tile or section");
            }
            self.validate_tiles(&self.tiles, "default")?;
        } else {
            let mut seen = std::collections::HashSet::new();
            for section in &self.sections {
                if section.name.is_empty() {
                    bail!("section name must not be empty");
                }
                if !seen.insert(section.name.as_str()) {
                    bail!("duplicate section name '{}'", section.name);
                }
                if section.tiles.is_empty() {
                    bail!("section '{}' must define at least one tile", section.name);
                }
                self.validate_tiles(&section.tiles, &section.name)?;
            }
        }

        Ok(())
    }

    fn validate_tiles(&self, tiles: &[TileDef], section_name: &str) -> Result<()> {
        let mut occupied: Vec<(u32, u32, &str)> = Vec::new();

        for tile in tiles {
            let (col, row, col_span, row_span, id) = match tile {
                TileDef::Metric { position, id, .. } => (
                    position.col,
                    position.row,
                    position.col_span,
                    position.row_span,
                    id.as_deref().unwrap_or("metric"),
                ),
                TileDef::Item {
                    position,
                    id,
                    good_key,
                    ..
                } => {
                    if good_key.is_empty() {
                        bail!("item tile '{id:?}' key must not be empty");
                    }
                    (
                        position.col,
                        position.row,
                        position.col_span,
                        position.row_span,
                        id.as_deref().unwrap_or(good_key.as_str()),
                    )
                }
                TileDef::Label {
                    position,
                    id,
                    ..
                } => (
                    position.col,
                    position.row,
                    position.col_span,
                    position.row_span,
                    id.as_deref().unwrap_or("label"),
                ),
            };

            if col_span == 0 || row_span == 0 {
                bail!("tile '{id}' span must be > 0");
            }
            if col + col_span > self.grid.columns {
                bail!(
                    "section '{section_name}': tile '{id}' exceeds grid width (col={col}, w={col_span}, columns={})",
                    self.grid.columns
                );
            }

            for dr in 0..row_span {
                for dc in 0..col_span {
                    let c = col + dc;
                    let r = row + dr;
                    if let Some((oc, or, oid)) =
                        occupied.iter().find(|(oc, or, _)| *oc == c && *or == r)
                    {
                        bail!(
                            "section '{section_name}': tile '{id}' at ({c},{r}) overlaps tile '{oid}' at ({oc},{or})"
                        );
                    }
                    occupied.push((c, r, id));
                }
            }
        }

        Ok(())
    }

    pub fn grid_pixel_size_for(&self, tiles: &[TileDef], scale: f32) -> [f32; 2] {
        let mut max_row = 1u32;
        let mut max_col = 0u32;
        for tile in tiles {
            let (col, row, col_span, row_span) = match tile {
                TileDef::Metric { position, .. }
                | TileDef::Item { position, .. }
                | TileDef::Label { position, .. } => (
                    position.col,
                    position.row,
                    position.col_span,
                    position.row_span,
                ),
            };
            max_row = max_row.max(row + row_span);
            max_col = max_col.max(col + col_span);
        }

        let unit_w = self.grid.unit_width() * scale;
        let unit_h = self.grid.unit_height() * scale;
        let gap = self.grid.gap * scale;

        // Window size follows the active section's tile bounds (4 cols, 10 cols, etc.).
        // `grid.columns` is only a placement limit at edit/validate time.
        let cols = max_col.max(1);
        let width = cols as f32 * unit_w + cols.saturating_sub(1) as f32 * gap;
        let height = max_row as f32 * unit_h + max_row.saturating_sub(1) as f32 * gap;
        let pad = self.grid.window_padding * scale;
        // Tile stroke (1.5px) + ImGui window border — avoids clipping the rightmost column.
        let bleed = 4.0 * scale;
        [width + pad * 2.0 + bleed, height + pad * 2.0 + bleed]
    }

    pub fn tile_origin(&self, col: u32, row: u32, scale: f32) -> [f32; 2] {
        let unit_w = self.grid.unit_width() * scale;
        let unit_h = self.grid.unit_height() * scale;
        let gap = self.grid.gap * scale;
        [col as f32 * (unit_w + gap), row as f32 * (unit_h + gap)]
    }

    pub fn tile_size(&self, col_span: u32, row_span: u32, scale: f32) -> [f32; 2] {
        let unit_w = self.grid.unit_width() * scale;
        let unit_h = self.grid.unit_height() * scale;
        let gap = self.grid.gap * scale;
        [
            col_span as f32 * unit_w + (col_span.saturating_sub(1)) as f32 * gap,
            row_span as f32 * unit_h + (row_span.saturating_sub(1)) as f32 * gap,
        ]
    }
}

pub fn load_layout(path: &Path) -> Result<LayoutConfig> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("Failed to read layout at {}", path.display()))?;
    let config: LayoutConfig = toml::from_str(&raw)
        .with_context(|| format!("Failed to parse layout at {}", path.display()))?;
    config.validate()?;
    Ok(config)
}

pub fn resolve_layout_path(
    base_dir: &Path,
    configured: Option<&str>,
) -> Option<std::path::PathBuf> {
    let rel = configured?;
    let path = if Path::new(rel).is_absolute() {
        std::path::PathBuf::from(rel)
    } else {
        base_dir.join(rel)
    };
    if path.exists() {
        Some(path)
    } else {
        warn!("Layout file not found: {}", path.display());
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_layout() -> LayoutConfig {
        LayoutConfig {
            grid: GridConfig::default(),
            style: LayoutStyle::default(),
            default_section: None,
            tiles: vec![
                TileDef::Metric {
                    id: Some("igt".into()),
                    metric: "igt".into(),
                    position: TilePosition {
                        col: 0,
                        row: 0,
                        col_span: 1,
                        row_span: 1,
                    },
                    label: "IGT".into(),
                    show_max: false,
                    icon: None,
                },
                TileDef::Item {
                    id: Some("godrick_rune".into()),
                    good_key: "godrick_rune".into(),
                    position: TilePosition {
                        col: 1,
                        row: 1,
                        col_span: 1,
                        row_span: 1,
                    },
                },
            ],
            sections: vec![],
        }
    }

    #[test]
    fn w_h_aliases_parse() {
        let raw = r#"
[[tile]]
kind = "metric"
metric = "igt"
col = 0
row = 0
w = 2
h = 1
label = "IGT"
"#;
        let layout: LayoutConfig = toml::from_str(raw).unwrap();
        match &layout.tiles[0] {
            TileDef::Metric { position, .. } => {
                assert_eq!(position.col_span, 2);
                assert_eq!(position.row_span, 1);
            }
            _ => panic!("expected metric"),
        }
    }

    #[test]
    fn layout_validates_no_overlap() {
        let mut layout = sample_layout();
        layout.tiles.push(TileDef::Metric {
            id: Some("dup".into()),
            metric: "deaths".into(),
            position: TilePosition {
                col: 0,
                row: 0,
                col_span: 1,
                row_span: 1,
            },
            label: "DEATHS".into(),
            show_max: false,
            icon: None,
        });
        assert!(layout.validate().is_err());
    }

    #[test]
    fn tile_origin_and_size() {
        let layout = sample_layout();
        let origin = layout.tile_origin(2, 1, 1.0);
        assert_eq!(origin[0], 2.0 * 48.0);
        let size = layout.tile_size(2, 1, 1.0);
        assert_eq!(size[0], 96.0);
    }

    #[test]
    fn parse_sections_toml() {
        let raw = r#"
[[section]]
name = "minimalist"

[[section.tile]]
kind = "metric"
metric = "igt"
col = 0
row = 0
label = "IGT"

[[section]]
name = "extended"

[[section.tile]]
kind = "metric"
metric = "bosses"
col = 0
row = 0
label = "BOSS"
"#;
        let layout: LayoutConfig = toml::from_str(raw).unwrap();
        assert_eq!(layout.section_count(), 2);
        assert_eq!(layout.section_name(0), Some("minimalist"));
        assert_eq!(layout.section_name(1), Some("extended"));
        assert_eq!(layout.tiles_for_section(0).len(), 1);
        assert_eq!(layout.tiles_for_section(1).len(), 1);
        layout.validate().unwrap();
    }

    #[test]
    fn sections_allow_same_coords_across_sections() {
        let raw = r#"
[[section]]
name = "a"

[[section.tile]]
kind = "metric"
metric = "igt"
col = 0
row = 0
label = "IGT"

[[section]]
name = "b"

[[section.tile]]
kind = "metric"
metric = "igt"
col = 0
row = 0
label = "IGT"
"#;
        let layout: LayoutConfig = toml::from_str(raw).unwrap();
        layout.validate().unwrap();
    }

    #[test]
    fn parse_metric_without_label() {
        let raw = r#"
[[tile]]
kind = "metric"
metric = "igt"
col = 0
row = 0
"#;
        let layout: LayoutConfig = toml::from_str(raw).unwrap();
        match &layout.tiles[0] {
            TileDef::Metric { label, .. } => assert!(label.is_empty()),
            _ => panic!("expected metric"),
        }
        layout.validate().unwrap();
    }

    #[test]
    fn parse_label_tile_empty_label() {
        let raw = r#"
[[tile]]
kind = "label"
col = 0
row = 0
label = ""
"#;
        let layout: LayoutConfig = toml::from_str(raw).unwrap();
        layout.validate().unwrap();
    }

    #[test]
    fn parse_label_tile() {
        let raw = r#"
[[tile]]
kind = "label"
col = 0
row = 0
w = 3
h = 1
label = "STONES"
"#;
        let layout: LayoutConfig = toml::from_str(raw).unwrap();
        match &layout.tiles[0] {
            TileDef::Label {
                label, position, ..
            } => {
                assert_eq!(label, "STONES");
                assert_eq!(position.col_span, 3);
            }
            _ => panic!("expected label"),
        }
        layout.validate().unwrap();
    }

    #[test]
    fn parse_window_padding() {
        let raw = r#"
[grid]
window_padding = 0

[[tile]]
kind = "label"
col = 0
row = 0
label = "X"
"#;
        let layout: LayoutConfig = toml::from_str(raw).unwrap();
        assert_eq!(layout.grid.window_padding, 0.0);
        layout.validate().unwrap();
    }

    #[test]
    fn grid_pixel_size_for_subset() {
        let layout = sample_layout();
        let full = layout.grid_pixel_size_for(&layout.tiles, 1.0);
        let subset = layout.grid_pixel_size_for(&layout.tiles[..1], 1.0);
        assert!(subset[0] < full[0], "narrower tile set → narrower window");
        assert!(subset[1] <= full[1]);
    }

    #[test]
    fn grid_pixel_size_ignores_global_columns_when_tiles_are_narrower() {
        let layout = LayoutConfig {
            grid: GridConfig {
                columns: 10,
                ..GridConfig::default()
            },
            ..sample_layout()
        };
        // One tile at col 0 → window should be 1 column wide, not 10.
        let [w, _] = layout.grid_pixel_size_for(&layout.tiles[..1], 1.0);
        let one_col = 48.0 + 4.0 + 8.0 * 2.0; // unit + bleed + padding
        assert!((w - one_col).abs() < 0.01, "expected ~{one_col}px, got {w}");
    }

    #[test]
    fn resolve_section_tiles_clamps_index() {
        let raw = include_str!("../../../layouts/dashboard.toml");
        let layout: LayoutConfig = toml::from_str(raw).unwrap();
        let (idx, tiles) = layout.resolve_section_tiles(99, Some("minimalist"));
        assert_eq!(idx, 0);
        assert_eq!(tiles.len(), 5);
    }

    #[test]
    fn dashboard_sections_have_tiles() {
        let raw = include_str!("../../../layouts/dashboard.toml");
        let layout: LayoutConfig = toml::from_str(raw).unwrap();
        assert_eq!(layout.section_count(), 2);
        assert_eq!(layout.tiles_for_section(0).len(), 5, "minimalist");
        assert!(layout.tiles_for_section(1).len() > 10, "extended");
    }

    #[test]
    fn parse_dashboard_toml() {
        let raw = include_str!("../../../layouts/dashboard.toml");
        let layout: LayoutConfig = toml::from_str(raw).expect("dashboard.toml should parse");
        layout.validate().expect("dashboard.toml should validate");
    }
}
