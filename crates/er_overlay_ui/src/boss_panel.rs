use er_overlay_common::{
    layout::LayoutStyle, parse_panel_layout, resolve_panel_rect, BossPanelScope, OverlayConfig,
    PanelRect,
};
use imgui::{Condition, StyleColor, Ui, WindowFlags};

use crate::hud_window::{draw_window_border, suppress_imgui_window_border, HudBounds};
use crate::view_model::OverlayViewModel;

const KILLED_COLOR: [f32; 4] = [0.5, 0.5, 0.5, 0.7];
const GAP_BELOW_HUD: f32 = 8.0;
/// Default auto layout: `-5, 10, 25%, 92%` (right edge, full height).
const DEFAULT_PANEL_LAYOUT: [f32; 4] = [-5.0, 10.0, 0.25, 0.92];

/// Drag position for the boss panel window.
#[derive(Debug, Clone, Default)]
pub struct BossPanelState {
    pub pos: Option<[f32; 2]>,
    /// Section index we last scrolled to (`None` = scroll on next render).
    last_scrolled_section: Option<usize>,
    /// Second scroll pass once the tree node has expanded (updates scroll_max).
    scroll_follow_up_frames: u8,
}

impl BossPanelState {
    /// Call when the panel is shown again (hotkey toggle).
    pub fn on_reopened(&mut self) {
        self.last_scrolled_section = None;
        self.scroll_follow_up_frames = 0;
    }

    fn should_scroll_to_section(&mut self, current_index: Option<usize>) -> bool {
        if self.last_scrolled_section != current_index {
            self.last_scrolled_section = current_index;
            self.scroll_follow_up_frames = 1;
            true
        } else {
            false
        }
    }

    fn consume_scroll_follow_up(&mut self) -> bool {
        if self.scroll_follow_up_frames > 0 {
            self.scroll_follow_up_frames -= 1;
            true
        } else {
            false
        }
    }
}

struct BossPanelGeometry {
    pos: [f32; 2],
    pivot: [f32; 2],
    width: f32,
    window_height: f32,
}

pub fn render_boss_panel(
    ui: &Ui,
    config: &OverlayConfig,
    style: &LayoutStyle,
    vm: &OverlayViewModel,
    state: &mut BossPanelState,
    hud_anchor: Option<HudBounds>,
    border_radius: f32,
) {
    let text_scale = (config.text_size * config.scale / 18.0).max(0.5);
    let viewport = ui.io().display_size;
    let geometry = boss_panel_geometry(config, state, hud_anchor, viewport);

    let mut window = ui
        .window("##er_boss_panel")
        .flags(
            WindowFlags::NO_COLLAPSE
                | WindowFlags::NO_FOCUS_ON_APPEARING
                | WindowFlags::NO_NAV
                | WindowFlags::NO_SCROLLBAR
                | WindowFlags::NO_RESIZE,
        )
        .position(geometry.pos, Condition::Always)
        .size([geometry.width, geometry.window_height], Condition::Always)
        .bg_alpha(style.window_bg_alpha());

    if geometry.pivot != [0.0, 0.0] {
        window = window.position_pivot(geometry.pivot);
    }

    let _no_native_border = suppress_imgui_window_border(ui);
    window.build(|| {
        let _bg = style
            .has_window_background()
            .then(|| ui.push_style_color(StyleColor::WindowBg, style.window_bg_rgba_f32()));
        ui.set_window_font_scale(text_scale);

        if ui.is_window_hovered() && ui.is_mouse_dragging(imgui::MouseButton::Left) {
            state.pos = Some(ui.window_pos());
        }

        render_header(ui, vm);
        ui.separator();

        if vm.boss_panel_sections.is_empty() {
            ui.text("Unknown region");
            if let Some(id) = vm.current_subregion_id {
                ui.text(format!("map_id: {id} (key: {})", id / 1000));
            } else {
                ui.text("(not in-game or data unavailable)");
            }
        } else {
            ui.child_window("##boss_list")
                .size(ui.content_region_avail())
                .build(|| match vm.boss_panel_scope {
                    BossPanelScope::AllRegions => {
                        let current_index =
                            vm.boss_panel_sections.iter().position(|s| s.is_current);
                        let scroll_to_current = state.should_scroll_to_section(current_index);
                        let scroll_follow_up = state.consume_scroll_follow_up();
                        let scroll_current = scroll_to_current || scroll_follow_up;
                        for (i, section) in vm.boss_panel_sections.iter().enumerate() {
                            let should_open = current_index == Some(i);
                            if should_open && scroll_current {
                                scroll_current_region_into_view(ui);
                            }
                            render_region_tree(ui, section, should_open);
                        }
                    }
                    BossPanelScope::CurrentRegion => {
                        for boss in vm
                            .boss_panel_sections
                            .first()
                            .into_iter()
                            .flat_map(|s| s.bosses.iter())
                        {
                            render_boss_row(ui, boss);
                        }
                    }
                });
        }

        draw_window_border(ui, style, border_radius * config.scale, config.scale);
    });
}

fn render_header(ui: &Ui, vm: &OverlayViewModel) {
    match vm.boss_panel_scope {
        BossPanelScope::CurrentRegion => {
            if let Some(section) = vm.boss_panel_sections.first() {
                ui.text(format!(
                    "{} ({}/{})",
                    section.region, section.killed, section.total
                ));
            }
        }
        BossPanelScope::AllRegions => {
            let region = vm.current_region.clone().unwrap_or_else(|| "?".to_string());
            ui.text(format!(
                "Bosses {}/{} — region: {}",
                vm.boss_panel_killed, vm.boss_panel_total, region
            ));
        }
    }
}

fn scroll_current_region_into_view(ui: &Ui) {
    ui.set_scroll_from_pos_y_with_ratio(ui.cursor_pos()[1], 0.2);
}

fn render_region_tree(ui: &Ui, section: &crate::view_model::BossPanelSection, should_open: bool) {
    let label = format!("{} ({}/{})", section.region, section.killed, section.total);
    // Same `should_open` logic as ER_boss_checklist_R; `Always` re-opens on region change
    // (needed because DLC is one aggregated node — `default_open` alone is FirstUseEver only).
    ui.tree_node_config(&label)
        .opened(should_open, Condition::Always)
        .build(|| {
            for boss in &section.bosses {
                render_boss_row(ui, boss);
            }
        });
}

fn render_boss_row(ui: &Ui, boss: &crate::view_model::BossPanelRow) {
    let killed = boss.killed.unwrap_or(false);
    let mut checked = killed;
    let label = if boss.dlc {
        format!("{} [DLC]", boss.name)
    } else {
        boss.name.clone()
    };
    let _token = if killed {
        Some(ui.push_style_color(StyleColor::Text, KILLED_COLOR))
    } else {
        None
    };
    ui.checkbox(label, &mut checked);

    if ui.is_item_hovered() {
        ui.tooltip(|| {
            if let Some(ref place) = boss.place {
                ui.text(place.as_str());
            } else {
                ui.text("(unknown location)");
            }
        });
    }
}

fn boss_panel_geometry(
    config: &OverlayConfig,
    state: &BossPanelState,
    hud_anchor: Option<HudBounds>,
    viewport: [f32; 2],
) -> BossPanelGeometry {
    let default_geom = default_boss_panel_geometry(viewport, hud_anchor, config);

    if let Some(pos) = state.pos {
        return BossPanelGeometry {
            pos,
            pivot: [0.0, 0.0],
            width: default_geom.width,
            window_height: default_geom.window_height,
        };
    }

    if let Some(raw) = config
        .boss_panel_layout
        .as_deref()
        .and_then(parse_panel_layout)
    {
        let PanelRect { pos, size, pivot } = resolve_panel_rect(viewport, raw);
        let mut geom = BossPanelGeometry {
            pos,
            pivot,
            width: size[0].max(120.0),
            window_height: size[1].max(80.0),
        };
        if let Some(hud) = hud_anchor {
            geom = adjust_boss_panel_below_hud(geom, hud, GAP_BELOW_HUD * config.scale);
        }
        return geom;
    }

    default_geom
}

fn default_boss_panel_geometry(
    viewport: [f32; 2],
    hud_anchor: Option<HudBounds>,
    config: &OverlayConfig,
) -> BossPanelGeometry {
    let PanelRect { pos, size, pivot } = resolve_panel_rect(viewport, DEFAULT_PANEL_LAYOUT);
    let mut geom = BossPanelGeometry {
        pos,
        pivot,
        width: size[0].max(120.0),
        window_height: size[1].max(80.0),
    };
    if let Some(hud) = hud_anchor {
        geom = adjust_boss_panel_below_hud(geom, hud, GAP_BELOW_HUD * config.scale);
    }
    geom
}

fn panel_top_y(geom: &BossPanelGeometry) -> f32 {
    geom.pos[1] - geom.pivot[1] * geom.window_height
}

/// Shifts a configured panel down so it sits below the minimalist HUD (keeps the bottom edge).
fn adjust_boss_panel_below_hud(
    mut geom: BossPanelGeometry,
    hud: HudBounds,
    gap: f32,
) -> BossPanelGeometry {
    let min_top = hud.pos[1] + hud.size[1] + gap;
    let top = panel_top_y(&geom);
    if top < min_top {
        let shift = min_top - top;
        geom.pos[1] += shift;
        geom.window_height = (geom.window_height - shift).max(80.0);
    }
    geom
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_panel_matches_layout_on_1080p() {
        let geom = default_boss_panel_geometry([1920.0, 1080.0], None, &OverlayConfig::default());
        assert!((geom.width - 480.0).abs() < 0.01);
        assert!((geom.window_height - 993.6).abs() < 0.1);
        assert_eq!(geom.pivot, [1.0, 0.0]);
        assert!((geom.pos[0] - 1915.0).abs() < 0.01);
    }

    #[test]
    fn default_panel_shifts_below_minimalist_hud() {
        let hud = HudBounds {
            pos: [1412.0, 16.0],
            size: [492.0, 84.0],
        };
        let geom =
            default_boss_panel_geometry([1920.0, 1080.0], Some(hud), &OverlayConfig::default());
        assert!((panel_top_y(&geom) - 108.0).abs() < 0.01);
        assert!((geom.width - 480.0).abs() < 0.01);
    }

    #[test]
    fn layout_panel_shifts_below_minimalist_hud() {
        let geom = BossPanelGeometry {
            pos: [1915.0, 10.0],
            pivot: [1.0, 0.0],
            width: 960.0,
            window_height: 990.0,
        };
        let hud = HudBounds {
            pos: [1412.0, 16.0],
            size: [492.0, 84.0],
        };
        let adjusted = adjust_boss_panel_below_hud(geom, hud, 8.0);
        assert!((panel_top_y(&adjusted) - 108.0).abs() < 0.01);
        assert!((adjusted.window_height - 892.0).abs() < 0.01);
    }
}
