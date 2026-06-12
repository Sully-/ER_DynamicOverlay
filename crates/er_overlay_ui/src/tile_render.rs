use er_overlay_common::layout::LayoutStyle;
use er_overlay_common::{OverlayConfig, TrackKind};
use imgui::{ImColor32, Ui};

use crate::icon_atlas::IconAtlas;
use crate::metric_registry::MetricValue;
use crate::tracked_icon::{draw_icon_key_at, draw_status_icon_at};
use crate::view_model::TrackedEntryRow;

pub(crate) fn rgba(r: u8, g: u8, b: u8, a: u8) -> [f32; 4] {
    [
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0,
        a as f32 / 255.0,
    ]
}

fn color_from_rgba(c: [u8; 4]) -> ImColor32 {
    ImColor32::from_rgba(c[0], c[1], c[2], c[3])
}

pub struct TileDrawCtx<'a> {
    pub ui: &'a Ui,
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub style: &'a LayoutStyle,
    pub config: &'a OverlayConfig,
    pub atlas: Option<&'a IconAtlas>,
    pub radius: f32,
}

/// Largest font scale (≤ `scale`) at which `text` fits in `max_width`.
///
/// ImGui text width is linear in the window font scale, so a single measurement
/// is enough to solve for the fitting scale directly — no iterative shrink loop.
fn fit_font_scale(ui: &Ui, text: &str, max_width: f32, scale: f32) -> f32 {
    const MIN_SCALE: f32 = 0.32;
    if text.is_empty() || max_width <= 0.0 {
        return scale;
    }
    ui.set_window_font_scale(scale);
    let width = ui.calc_text_size(text)[0];
    if width <= max_width || width <= 0.0 {
        return scale;
    }
    (scale * max_width / width).max(MIN_SCALE)
}

pub fn draw_tile_frame(
    ui: &Ui,
    pos: [f32; 2],
    size: [f32; 2],
    border: [u8; 4],
    bg: [u8; 4],
    radius: f32,
) {
    let draw = ui.get_window_draw_list();
    let p0 = pos;
    let p1 = [pos[0] + size[0], pos[1] + size[1]];
    draw.add_rect(p0, p1, color_from_rgba(bg))
        .filled(true)
        .rounding(radius)
        .build();
    draw.add_rect(p0, p1, color_from_rgba(border))
        .filled(false)
        .thickness(1.5)
        .rounding(radius)
        .build();
}

pub fn draw_metric_tile(
    ctx: &TileDrawCtx<'_>,
    label: &str,
    value_text: &str,
    complete: bool,
    icon_key: Option<&str>,
) {
    let TileDrawCtx {
        ui,
        pos,
        size,
        style,
        config,
        atlas,
        radius,
    } = ctx;
    let border = if complete {
        style.border_complete
    } else {
        style.border_default
    };
    draw_tile_frame(ui, *pos, *size, border, style.tile_bg, *radius);

    let base_scale = (config.text_size * config.scale / 18.0).max(0.5);
    let label_scale = base_scale * style.label_scale;
    let value_scale = base_scale * style.value_scale;

    let label_color = rgba(170, 170, 180, 255);
    let value_color = rgba(245, 245, 250, 255);

    let pad_x = size[0] * 0.1;
    let max_text_w = (size[0] - pad_x * 2.0).max(8.0);

    let has_label = !label.is_empty();
    let label_h = label_scale * 18.0;
    let fitted_value_scale = fit_font_scale(ui, value_text, max_text_w, value_scale);
    let value_h = fitted_value_scale * 18.0;

    let has_icon = icon_key.is_some() && config.use_item_icons;
    let min_dim = size[0].min(size[1]);
    let icon_size = if has_icon {
        if has_label {
            min_dim * 0.38
        } else {
            let vertical_pad = min_dim * 0.06;
            let value_gap = value_h * 0.25;
            let max_from_height = size[1] - value_h - value_gap - vertical_pad * 2.0;
            min_dim.min(max_from_height).clamp(min_dim * 0.38, min_dim * 0.72)
        }
    } else {
        0.0
    };

    let block_h = if has_icon {
        if has_label {
            icon_size + label_h * 0.35 + label_h + value_h
        } else {
            icon_size + value_h * 0.25 + value_h
        }
    } else if has_label {
        label_h + value_h * 1.1
    } else {
        value_h
    };

    let block_top = pos[1] + (size[1] - block_h) * 0.5;
    let mut y = block_top;

    if let Some(key) = icon_key {
        let ix = pos[0] + (size[0] - icon_size) * 0.5;
        draw_icon_key_at(ui, key, [ix, y], icon_size, 1.0, *atlas, config);
        y += icon_size + if has_label { label_h * 0.25 } else { value_h * 0.25 };
    }

    if has_label {
        ui.set_window_font_scale(label_scale);
        let label_w = ui.calc_text_size(label)[0].min(max_text_w);
        ui.set_cursor_screen_pos([pos[0] + (size[0] - label_w) * 0.5, y]);
        ui.text_colored(label_color, label);
        y += label_h;
    }

    ui.set_window_font_scale(fitted_value_scale);
    let value_w = ui.calc_text_size(value_text)[0].min(max_text_w);
    ui.set_cursor_screen_pos([pos[0] + (size[0] - value_w) * 0.5, y]);
    ui.text_colored(value_color, value_text);

    ui.set_window_font_scale(base_scale);
}

pub fn draw_label_tile(
    ui: &Ui,
    pos: [f32; 2],
    size: [f32; 2],
    label: &str,
    style: &LayoutStyle,
    config: &OverlayConfig,
    radius: f32,
) {
    draw_tile_frame(ui, pos, size, style.border_default, style.tile_bg, radius);

    if label.is_empty() {
        return;
    }

    let base_scale = (config.text_size * config.scale / 18.0).max(0.5);
    let text_scale = base_scale * style.value_scale;
    let text_color = rgba(245, 245, 250, 255);

    let pad_x = size[0] * 0.1;
    let max_text_w = (size[0] - pad_x * 2.0).max(8.0);
    let fitted_scale = fit_font_scale(ui, label, max_text_w, text_scale);

    ui.set_window_font_scale(fitted_scale);
    let measured = ui.calc_text_size(label);
    let text_w = measured[0].min(max_text_w);
    let text_h = measured[1];

    ui.set_cursor_screen_pos([
        pos[0] + (size[0] - text_w) * 0.5,
        pos[1] + (size[1] - text_h) * 0.5,
    ]);
    ui.text_colored(text_color, label);
    ui.set_window_font_scale(base_scale);
}

pub fn draw_unavailable_metric_tile(ctx: &TileDrawCtx<'_>, label: &str, icon_key: Option<&str>) {
    draw_metric_tile(ctx, label, "---", false, icon_key);
}

/// One box per yes/no item: centered icon, colored when owned, grayed out otherwise.
pub fn draw_item_tile(ctx: &TileDrawCtx<'_>, row: &TrackedEntryRow, icon_override: Option<&str>) {
    let TileDrawCtx {
        ui,
        pos,
        size,
        style,
        config,
        atlas,
        radius,
    } = ctx;
    let (acquired, unknown) = crate::tracked_icon::track_status(&row.kind);
    // Neutral border: state is read from the icon (color vs transparent gray), not the frame.
    let border = style.border_default;
    let mut bg = style.tile_bg;
    if !acquired || unknown {
        bg[3] = (f32::from(bg[3]) * 0.55) as u8;
    }
    draw_tile_frame(ui, *pos, *size, border, bg, *radius);

    let is_countable = matches!(row.kind, TrackKind::Countable { .. });
    let icon_size = if is_countable {
        size[0].min(size[1]) * 0.58
    } else {
        size[0].min(size[1]) * 0.78
    };
    let ix = pos[0] + (size[0] - icon_size) * 0.5;
    let iy = if is_countable {
        pos[1] + size[1] * 0.12
    } else {
        pos[1] + (size[1] - icon_size) * 0.5
    };

    let mut display_row = row.clone();
    if let Some(key) = icon_override {
        display_row.icon_key = key.to_string();
    }

    draw_status_icon_at(
        ui,
        [ix, iy],
        &display_row,
        icon_size,
        config.gray_tint,
        *atlas,
        config,
    );

    if let TrackKind::Countable { count } = row.kind {
        let count_text = match count {
            Some(n) => n.to_string(),
            None => "---".to_string(),
        };
        let base_scale = (config.text_size * config.scale / 18.0).max(0.5);
        let count_scale = base_scale * style.value_scale * 0.85;
        let max_text_w = (size[0] * 0.9).max(8.0);
        let fitted = fit_font_scale(ui, &count_text, max_text_w, count_scale);
        ui.set_window_font_scale(fitted);
        let text_w = ui.calc_text_size(&count_text)[0].min(max_text_w);
        let count_y = pos[1] + size[1] - fitted * 16.0 - 2.0;
        ui.set_cursor_screen_pos([pos[0] + (size[0] - text_w) * 0.5, count_y]);
        ui.text_colored(rgba(245, 245, 250, 255), &count_text);
        ui.set_window_font_scale(base_scale);
    }
}

pub fn metric_value_for_tile(value: &MetricValue, show_max: bool) -> String {
    crate::metric_registry::format_metric_value(value, show_max)
}
