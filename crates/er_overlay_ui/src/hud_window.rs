use er_overlay_common::layout::LayoutStyle;
use er_overlay_common::{Anchor, OverlayConfig};
use imgui::{Condition, ImColor32, StyleVar, Ui, WindowFlags};

/// Top-left corner and size of a positioned window.
#[derive(Debug, Clone, Copy)]
pub struct HudBounds {
    pub pos: [f32; 2],
    pub size: [f32; 2],
}

/// Position remembered when the user moves the overlay.
#[derive(Debug, Clone)]
pub struct HudDragState {
    pub pos: Option<[f32; 2]>,
    anchor: Anchor,
    offset_x: f32,
    offset_y: f32,
}

impl Default for HudDragState {
    fn default() -> Self {
        Self {
            pos: None,
            anchor: Anchor::default(),
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }
}

impl HudDragState {
    pub fn sync_anchor(&mut self, config: &OverlayConfig) {
        if self.anchor != config.anchor
            || self.offset_x != config.offset_x
            || self.offset_y != config.offset_y
        {
            self.pos = None;
            self.anchor = config.anchor;
            self.offset_x = config.offset_x;
            self.offset_y = config.offset_y;
        }
    }

    pub fn capture_pos(&mut self, ui: &Ui) {
        self.pos = Some(ui.window_pos());
    }
}

pub struct HudWindowPlacement {
    pub pos: [f32; 2],
    pub condition: Condition,
    pub pivot: Option<[f32; 2]>,
}

/// Converts an anchored pivot position into a top-left corner.
pub fn top_left_from_placement(pos: [f32; 2], pivot: [f32; 2], size: [f32; 2]) -> [f32; 2] {
    [pos[0] - pivot[0] * size[0], pos[1] - pivot[1] * size[1]]
}

pub fn hud_window_placement(
    ui: &Ui,
    config: &OverlayConfig,
    drag: &HudDragState,
) -> HudWindowPlacement {
    if let Some(pos) = drag.pos {
        return HudWindowPlacement {
            pos,
            condition: Condition::Always,
            pivot: None,
        };
    }

    let display = ui.io().display_size;
    let w = display[0];
    let h = display[1];
    let ox = config.offset_x * config.scale;
    let oy = config.offset_y * config.scale;
    let (pos, pivot) = match config.anchor {
        Anchor::TopLeft => ([ox, oy], [0.0, 0.0]),
        Anchor::TopRight => ([w - ox, oy], [1.0, 0.0]),
        Anchor::BottomLeft => ([ox, h - oy], [0.0, 1.0]),
        Anchor::BottomRight => ([w - ox, h - oy], [1.0, 1.0]),
    };
    // `Always` so the anchor is re-applied every frame: when the active section
    // changes the window width (e.g. minimalist -> extended), a top-right anchor
    // must keep its right edge pinned instead of growing off-screen.
    HudWindowPlacement {
        pos,
        condition: Condition::Always,
        pivot: Some(pivot),
    }
}

pub fn hud_window_flags(fixed_size: bool) -> WindowFlags {
    let mut flags = WindowFlags::NO_TITLE_BAR
        | WindowFlags::NO_SCROLLBAR
        | WindowFlags::NO_COLLAPSE
        | WindowFlags::NO_FOCUS_ON_APPEARING
        | WindowFlags::NO_BRING_TO_FRONT_ON_FOCUS
        | WindowFlags::NO_NAV;

    if fixed_size {
        flags |= WindowFlags::NO_RESIZE;
    } else {
        flags |= WindowFlags::ALWAYS_AUTO_RESIZE;
    }

    flags
}

/// ImGui draws its own 1px window border at `Begin()` — disable it; we draw manually when enabled.
pub fn suppress_imgui_window_border<'ui>(ui: &'ui Ui) -> imgui::StyleStackToken<'ui> {
    ui.push_style_var(StyleVar::WindowBorderSize(0.0))
}

/// Outer window frame when `window_border` is set (uses `border_default`).
pub fn draw_window_border(ui: &Ui, style: &LayoutStyle, rounding: f32, scale: f32) {
    if !style.window_border {
        return;
    }
    let draw = ui.get_window_draw_list();
    let pos = ui.window_pos();
    let [w, h] = ui.window_size();
    let p1 = [pos[0] + w, pos[1] + h];
    let c = style.border_default;
    let color = ImColor32::from_rgba(c[0], c[1], c[2], c[3]);
    draw.add_rect(pos, p1, color)
        .filled(false)
        .thickness((1.5 * scale).max(1.0))
        .rounding(rounding)
        .build();
}

pub fn debug_window_flags() -> WindowFlags {
    WindowFlags::NO_COLLAPSE
        | WindowFlags::NO_FOCUS_ON_APPEARING
        | WindowFlags::NO_BRING_TO_FRONT_ON_FOCUS
}
