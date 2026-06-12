use er_overlay_common::{LayoutConfig, OverlayConfig};
use imgui::{Condition, MouseButton, Ui};

use crate::boss_panel::{render_boss_panel, BossPanelState};
use crate::hud_window::{
    debug_window_flags, hud_window_flags, hud_window_placement, top_left_from_placement,
    HudBounds, HudDragState,
};
use crate::icon_atlas::IconAtlas;
use crate::layout_engine::render_layout_dashboard;
use crate::tile_render::rgba;
use crate::view_model::OverlayViewModel;

pub fn render_overlay(
    ui: &Ui,
    config: &OverlayConfig,
    vm: &OverlayViewModel,
    atlas: Option<&IconAtlas>,
    layout: Option<&LayoutConfig>,
    active_section_index: usize,
    drag: &mut HudDragState,
    show_boss_panel: bool,
    boss_panel: &mut BossPanelState,
) {
    let hud_anchor = layout.as_ref().map(|layout| {
        drag.sync_anchor(config);
        minimalist_hud_bounds(ui, config, layout, drag)
    });

    if let Some(layout) = layout {
        render_overlay_layout(ui, config, vm, atlas, layout, active_section_index, drag);
    }

    if show_boss_panel {
        render_boss_panel(ui, config, vm, boss_panel, hud_anchor);
    }

    if config.show_debug {
        render_debug_window(ui, vm, drag);
    }
}

/// Bounds of the minimalist HUD section — used to anchor the boss panel underneath.
fn minimalist_hud_bounds(
    ui: &Ui,
    config: &OverlayConfig,
    layout: &LayoutConfig,
    drag: &HudDragState,
) -> HudBounds {
    let idx = layout.section_index("minimalist").unwrap_or(0);
    let tiles = layout.tiles_for_section(idx);
    let size = layout.grid_pixel_size_for(tiles, config.scale);
    let placement = hud_window_placement(ui, config, drag);
    let pivot = placement.pivot.unwrap_or([0.0, 0.0]);
    let pos = top_left_from_placement(placement.pos, pivot, size);
    HudBounds { pos, size }
}

fn render_overlay_layout(
    ui: &Ui,
    config: &OverlayConfig,
    vm: &OverlayViewModel,
    atlas: Option<&IconAtlas>,
    layout: &LayoutConfig,
    active_section_index: usize,
    drag: &mut HudDragState,
) {
    let (_index, tiles) = layout.resolve_section_tiles(
        active_section_index,
        config.default_layout_section.as_deref(),
    );
    let [width, height] = layout.grid_pixel_size_for(tiles, config.scale);
    let text_size = config.text_size * config.scale;
    drag.sync_anchor(config);
    let placement = hud_window_placement(ui, config, drag);

    let mut window = ui
        .window("##er_overlay_hud")
        .flags(hud_window_flags(true))
        .position(placement.pos, placement.condition);
    if let Some(pivot) = placement.pivot {
        window = window.position_pivot(pivot);
    }

    window
        .bg_alpha(config.background_opacity)
        .size([width, height], Condition::Always)
        .build(|| {
            let pad = layout.grid.window_padding * config.scale;
            let _pad = ui.push_style_var(imgui::StyleVar::WindowPadding([pad, pad]));
            let _spacing = ui.push_style_var(imgui::StyleVar::ItemSpacing([0.0, 0.0]));
            let _bg = ui.push_style_color(
                imgui::StyleColor::WindowBg,
                rgba(12, 12, 18, (config.background_opacity * 255.0) as u8),
            );
            ui.set_window_font_scale((text_size / 18.0).max(0.5));
            let origin = ui.cursor_screen_pos();
            let inner_w = (width - pad * 2.0).max(1.0);
            let inner_h = (height - pad * 2.0).max(1.0);
            ui.dummy([inner_w, inner_h]);
            render_layout_dashboard(ui, config, layout, tiles, vm, atlas, origin);
            if ui.is_window_hovered() && ui.is_mouse_dragging(MouseButton::Left) {
                drag.capture_pos(ui);
            }
        });
}

fn render_debug_window(ui: &Ui, vm: &OverlayViewModel, drag: &HudDragState) {
    let pos = drag
        .pos
        .map(|[x, y]| [x, y + 120.0])
        .unwrap_or([32.0, 32.0]);

    ui.window("##er_overlay_debug")
        .flags(debug_window_flags())
        .position(pos, Condition::FirstUseEver)
        .build(|| {
            ui.text("Debug");
            render_debug(ui, vm);
        });
}

fn render_debug(ui: &Ui, vm: &OverlayViewModel) {
    let d = &vm.diagnostics;

    ui.text(format!("Backend: {:?}", d.backend));
    ui.text(format!("GameDataMan: {}", d.gamedata_man_resolved));
    ui.text(format!("EventFlagMan: {}", d.event_flag_man_resolved));
    ui.text(format!("WorldChrMan: {}", d.world_chr_man_resolved));
    ui.text(format!("Boss flags: {}", d.boss_flags_loaded));
    ui.text(format!("Rune flags: {}", d.great_rune_flags_loaded));
    ui.text(format!("FieldArea: {}", d.field_area_resolved));
    if let Some(id) = vm.current_subregion_id {
        ui.text(format!("Map id: {id} (key: {})", id / 1000));
    } else {
        ui.text("Map id: (unavailable)");
    }
    if let Some(ref region) = vm.current_region {
        ui.text(format!("Current region: {region}"));
    }
    ui.text(format!(
        "Boss panel: {:?} ({}/{})",
        vm.boss_panel_scope, vm.boss_panel_killed, vm.boss_panel_total
    ));
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::view_model::build_view_model;

    use er_game_state::mock::MockGameState;

    #[test]
    fn view_model_builds() {
        let mock = MockGameState::default();
        let vm = build_view_model(
            &mock,
            &[],
            &HashSet::new(),
            er_overlay_common::BossPanelScope::CurrentRegion,
        );
        assert!(vm.igt.is_some());
        assert_eq!(vm.deaths, Some(42));
    }
}
