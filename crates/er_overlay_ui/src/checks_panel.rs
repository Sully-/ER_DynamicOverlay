use er_overlay_common::{
    layout::LayoutStyle, parse_panel_layout, resolve_panel_rect, BossPanelScope, OverlayConfig,
    PanelRect,
};
use imgui::{Condition, StyleColor, Ui, WindowFlags};

use crate::fonts::overlay_font_scale;
use crate::hud_window::{draw_window_border, suppress_imgui_window_border, HudBounds};
use crate::view_model::{CheckPanelRow, CheckPanelSection, OverlayViewModel};

const DONE_COLOR: [f32; 4] = [0.5, 0.5, 0.5, 0.7];
const UNTRACEABLE_COLOR: [f32; 4] = [0.55, 0.55, 0.62, 0.6];
const GAP_BELOW_HUD: f32 = 8.0;
/// Default auto layout: `-5, 10, 25%, 92%` (same placement as the boss panel).
const DEFAULT_PANEL_LAYOUT: [f32; 4] = [-5.0, 10.0, 0.25, 0.92];

/// Drag position + scroll bookkeeping for the checks panel window.
#[derive(Debug, Clone, Default)]
pub struct ChecksPanelState {
    pub pos: Option<[f32; 2]>,
    last_scrolled_section: Option<usize>,
    scroll_follow_up_frames: u8,
    last_current_section: Option<usize>,
}

impl ChecksPanelState {
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

struct ChecksPanelGeometry {
    pos: [f32; 2],
    pivot: [f32; 2],
    width: f32,
    window_height: f32,
}

pub fn render_checks_panel(
    ui: &Ui,
    config: &OverlayConfig,
    style: &LayoutStyle,
    vm: &OverlayViewModel,
    state: &mut ChecksPanelState,
    hud_anchor: Option<HudBounds>,
    border_radius: f32,
) {
    let text_scale = overlay_font_scale(config);
    let viewport = ui.io().display_size;
    let geometry = checks_panel_geometry(config, state, hud_anchor, viewport);

    let mut window = ui
        .window("##er_checks_panel")
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

        if vm.checks_panel_sections.is_empty() {
            ui.text("Unknown region");
            if let Some(id) = vm.current_subregion_id {
                ui.text(format!("map_id: {id} (key: {})", id / 1000));
            } else {
                ui.text("(not in-game or data unavailable)");
            }
        } else {
            ui.child_window("##checks_list")
                .size(ui.content_region_avail())
                .build(|| match vm.checks_panel_scope {
                    BossPanelScope::AllRegions => {
                        let current_index =
                            vm.checks_panel_sections.iter().position(|s| s.is_current);
                        let region_just_changed = state.last_current_section != current_index;
                        if region_just_changed {
                            state.last_current_section = current_index;
                        }
                        let scroll_to_current = state.should_scroll_to_section(current_index);
                        let scroll_follow_up = state.consume_scroll_follow_up();
                        let scroll_current = scroll_to_current || scroll_follow_up;
                        for (i, section) in vm.checks_panel_sections.iter().enumerate() {
                            let forced_open =
                                region_just_changed.then_some(current_index == Some(i));
                            if current_index == Some(i) && scroll_current {
                                scroll_current_region_into_view(ui);
                            }
                            render_region_tree(ui, section, forced_open);
                        }
                    }
                    BossPanelScope::CurrentRegion => {
                        for row in vm
                            .checks_panel_sections
                            .first()
                            .into_iter()
                            .flat_map(|s| s.rows.iter())
                        {
                            render_check_row(ui, row);
                        }
                    }
                });
        }

        draw_window_border(ui, style, border_radius * config.scale, config.scale);
    });
}

fn render_header(ui: &Ui, vm: &OverlayViewModel) {
    let seed = if vm.checks_seed_active { " [seed]" } else { "" };
    match vm.checks_panel_scope {
        BossPanelScope::CurrentRegion => {
            if let Some(section) = vm.checks_panel_sections.first() {
                ui.text(format!(
                    "{} ({}/{}){seed}",
                    section.region, section.done, section.total
                ));
            }
        }
        BossPanelScope::AllRegions => {
            let region = vm
                .checks_current_region
                .clone()
                .unwrap_or_else(|| "?".to_string());
            ui.text(format!(
                "Checks {}/{} - region: {region}{seed}",
                vm.checks_panel_done, vm.checks_panel_total
            ));
        }
    }
}

fn scroll_current_region_into_view(ui: &Ui) {
    ui.set_scroll_from_pos_y_with_ratio(ui.cursor_pos()[1], 0.2);
}

fn render_region_tree(ui: &Ui, section: &CheckPanelSection, forced_open: Option<bool>) {
    // The text after `###` is a stable ImGui id: without it the node id would change as
    // done/total move, making ImGui treat it as a new (collapsed) node every time a check flips.
    let label = format!(
        "{} ({}/{})###checks_region_{}",
        section.region, section.done, section.total, section.region
    );
    let mut node = ui.tree_node_config(&label);
    if let Some(open) = forced_open {
        node = node.opened(open, Condition::Always);
    }
    node.build(|| {
        for row in &section.rows {
            render_check_row(ui, row);
        }
    });
}

fn render_check_row(ui: &Ui, row: &CheckPanelRow) {
    let label = if row.dlc {
        format!("{} [DLC]", row.name)
    } else {
        row.name.clone()
    };

    if !row.traceable {
        // Dynamic check whose lot holds a flagless item this seed: listed but not trackable.
        let _token = ui.push_style_color(StyleColor::Text, UNTRACEABLE_COLOR);
        ui.text(format!("- {label}"));
        if ui.is_item_hovered() {
            ui.tooltip(|| {
                if let Some(ref place) = row.place {
                    ui.text(place.as_str());
                }
                ui.text("Untraceable this seed (no acquisition flag)");
            });
        }
        return;
    }

    let done = row.done.unwrap_or(false);
    let mut checked = done;
    let _token = if done {
        Some(ui.push_style_color(StyleColor::Text, DONE_COLOR))
    } else {
        None
    };
    ui.checkbox(label, &mut checked);

    if ui.is_item_hovered() {
        ui.tooltip(|| {
            if let Some(ref place) = row.place {
                ui.text(place.as_str());
            } else {
                ui.text("(unknown location)");
            }
        });
    }
}

fn checks_panel_geometry(
    config: &OverlayConfig,
    state: &ChecksPanelState,
    hud_anchor: Option<HudBounds>,
    viewport: [f32; 2],
) -> ChecksPanelGeometry {
    let default_geom = default_checks_panel_geometry(viewport, hud_anchor, config);

    if let Some(pos) = state.pos {
        return ChecksPanelGeometry {
            pos,
            pivot: [0.0, 0.0],
            width: default_geom.width,
            window_height: default_geom.window_height,
        };
    }

    if let Some(raw) = config
        .checks_panel_layout
        .as_deref()
        .and_then(parse_panel_layout)
    {
        let PanelRect { pos, size, pivot } = resolve_panel_rect(viewport, raw);
        let mut geom = ChecksPanelGeometry {
            pos,
            pivot,
            width: size[0].max(120.0),
            window_height: size[1].max(80.0),
        };
        if let Some(hud) = hud_anchor {
            geom = adjust_below_hud(geom, hud, GAP_BELOW_HUD * config.scale);
        }
        return geom;
    }

    default_geom
}

fn default_checks_panel_geometry(
    viewport: [f32; 2],
    hud_anchor: Option<HudBounds>,
    config: &OverlayConfig,
) -> ChecksPanelGeometry {
    let PanelRect { pos, size, pivot } = resolve_panel_rect(viewport, DEFAULT_PANEL_LAYOUT);
    let mut geom = ChecksPanelGeometry {
        pos,
        pivot,
        width: size[0].max(120.0),
        window_height: size[1].max(80.0),
    };
    if let Some(hud) = hud_anchor {
        geom = adjust_below_hud(geom, hud, GAP_BELOW_HUD * config.scale);
    }
    geom
}

fn panel_top_y(geom: &ChecksPanelGeometry) -> f32 {
    geom.pos[1] - geom.pivot[1] * geom.window_height
}

/// Shifts a panel down so it sits below the HUD when they would overlap (keeps the bottom edge).
fn adjust_below_hud(
    mut geom: ChecksPanelGeometry,
    hud: HudBounds,
    gap: f32,
) -> ChecksPanelGeometry {
    let panel_left = geom.pos[0] - geom.pivot[0] * geom.width;
    let panel_right = panel_left + geom.width;
    let hud_left = hud.pos[0];
    let hud_right = hud.pos[0] + hud.size[0];
    let horizontally_overlaps = panel_left < hud_right && panel_right > hud_left;
    if !horizontally_overlaps {
        return geom;
    }
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
    fn default_panel_matches_boss_panel_layout_on_1080p() {
        let geom = default_checks_panel_geometry([1920.0, 1080.0], None, &OverlayConfig::default());
        assert_eq!(geom.pivot, [1.0, 0.0]);
        assert!((geom.pos[0] - 1915.0).abs() < 0.01);
        assert!((geom.width - 480.0).abs() < 0.01);
    }

    #[test]
    fn default_panel_shifts_below_minimalist_hud() {
        let hud = HudBounds {
            pos: [1412.0, 16.0],
            size: [492.0, 84.0],
        };
        let geom =
            default_checks_panel_geometry([1920.0, 1080.0], Some(hud), &OverlayConfig::default());
        assert!((panel_top_y(&geom) - 108.0).abs() < 0.01);
    }
}
