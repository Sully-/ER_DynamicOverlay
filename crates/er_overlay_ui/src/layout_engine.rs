use er_overlay_common::{LayoutConfig, OverlayConfig, TileDef};
use imgui::Ui;

use crate::icon_atlas::IconAtlas;
use crate::metric_registry::{
    apply_metric_max, metric_is_complete, resolve_metric, resolve_tracked_key,
};
use crate::tile_render::{
    draw_item_tile, draw_label_tile, draw_metric_tile, draw_unavailable_metric_tile,
    metric_value_for_tile, TileDrawCtx,
};
use crate::view_model::OverlayViewModel;

pub fn render_layout_dashboard(
    ui: &Ui,
    config: &OverlayConfig,
    layout: &LayoutConfig,
    tiles: &[TileDef],
    vm: &OverlayViewModel,
    atlas: Option<&IconAtlas>,
    window_origin: [f32; 2],
) {
    let scale = config.scale;
    let radius = layout.grid.border_radius * scale;

    let mut sorted: Vec<&TileDef> = tiles.iter().collect();
    sorted.sort_by_key(|t| match t {
        TileDef::Metric { position, .. }
        | TileDef::Item { position, .. }
        | TileDef::Label { position, .. } => (position.row, position.col),
    });

    for tile in sorted {
        match tile {
            TileDef::Metric {
                metric,
                position,
                label,
                show_max,
                max,
                icon,
                ..
            } => {
                let origin = layout.tile_origin(position.col, position.row, scale);
                let pos = [window_origin[0] + origin[0], window_origin[1] + origin[1]];
                let size = layout.tile_size(position.col_span, position.row_span, scale);
                let value = apply_metric_max(resolve_metric(metric, vm), *max);
                let icon_key = icon.as_deref();
                let ctx = TileDrawCtx {
                    ui,
                    pos,
                    size,
                    style: &layout.style,
                    config,
                    atlas,
                    radius,
                };

                if value == crate::metric_registry::MetricValue::Unavailable {
                    draw_unavailable_metric_tile(&ctx, label, icon_key);
                } else {
                    let complete = metric_is_complete(&value);
                    let text = metric_value_for_tile(&value, *show_max);
                    draw_metric_tile(&ctx, label, &text, complete, icon_key);
                }
            }
            TileDef::Item {
                good_key,
                position,
                track_equipped,
                ..
            } => {
                let origin = layout.tile_origin(position.col, position.row, scale);
                let pos = [window_origin[0] + origin[0], window_origin[1] + origin[1]];
                let size = layout.tile_size(position.col_span, position.row_span, scale);

                let ctx = TileDrawCtx {
                    ui,
                    pos,
                    size,
                    style: &layout.style,
                    config,
                    atlas,
                    radius,
                };

                if let Some(row) = resolve_tracked_key(good_key, vm) {
                    draw_item_tile(&ctx, row, None, *track_equipped);
                } else {
                    draw_unavailable_metric_tile(&ctx, "?", Some(good_key.as_str()));
                }
            }
            TileDef::Label {
                position, label, ..
            } => {
                let origin = layout.tile_origin(position.col, position.row, scale);
                let pos = [window_origin[0] + origin[0], window_origin[1] + origin[1]];
                let size = layout.tile_size(position.col_span, position.row_span, scale);
                draw_label_tile(ui, pos, size, label, &layout.style, config, radius);
            }
        }
    }
}
