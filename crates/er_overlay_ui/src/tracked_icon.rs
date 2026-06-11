use er_overlay_common::{OverlayConfig, TrackKind};
use imgui::{ImColor32, Ui};

use crate::icon_atlas::IconAtlas;
use crate::view_model::TrackedEntryRow;

pub fn draw_icon_key_at(
    ui: &Ui,
    key: &str,
    pos: [f32; 2],
    size: f32,
    tint: f32,
    atlas: Option<&IconAtlas>,
    config: &OverlayConfig,
) {
    if config.use_item_icons {
        if let Some(atlas) = atlas {
            if atlas.draw_key_at(ui, key, pos, size, icon_tint_color(tint)) {
                return;
            }
        }
    }
    let draw = ui.get_window_draw_list();
    draw.add_rect(
        [pos[0], pos[1]],
        [pos[0] + size, pos[1] + size],
        icon_tint_color(tint),
    )
    .filled(true)
    .rounding(3.0)
    .build();
}

fn icon_tint_color(tint: f32) -> ImColor32 {
    let c = (255.0 * tint.clamp(0.0, 1.0)) as u8;
    ImColor32::from_rgba(c, c, c, 255)
}

pub fn track_status(kind: &TrackKind) -> (bool, bool) {
    match kind {
        TrackKind::Unique {
            acquired: Some(true),
        } => (true, false),
        TrackKind::Unique {
            acquired: Some(false),
        } => (false, false),
        TrackKind::Unique { acquired: None } => (false, true),
        TrackKind::Countable { count: Some(n) } => (*n > 0, false),
        TrackKind::Countable { count: None } => (false, true),
    }
}

/// Absolute draw via DrawList (does not alter ImGui layout).
pub fn draw_status_icon_at(
    ui: &Ui,
    pos: [f32; 2],
    row: &TrackedEntryRow,
    size: f32,
    gray: f32,
    atlas: Option<&IconAtlas>,
    config: &OverlayConfig,
) {
    let (acquired, unknown) = track_status(&row.kind);
    let color = status_icon_color(acquired, unknown, gray);

    if config.use_item_icons {
        if let Some(atlas) = atlas {
            if atlas.draw_key_at(ui, &row.icon_key, pos, size, color) {
                return;
            }
        }
    }

    let draw = ui.get_window_draw_list();
    draw.add_rect([pos[0], pos[1]], [pos[0] + size, pos[1] + size], color)
        .filled(true)
        .rounding(2.0)
        .build();
}

fn status_icon_color(acquired: bool, unknown: bool, gray: f32) -> ImColor32 {
    if unknown {
        let c = (140.0 * gray.clamp(0.3, 1.0)) as u8;
        return ImColor32::from_rgba(c, c, c, 140);
    }
    if acquired {
        ImColor32::from_rgba(255, 255, 255, 255)
    } else {
        let g = gray.clamp(0.25, 0.65);
        let c = (255.0 * g) as u8;
        let a = (255.0 * g * 0.45) as u8;
        ImColor32::from_rgba(c, c, c, a.max(70))
    }
}
