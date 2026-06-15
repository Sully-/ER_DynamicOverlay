use imgui::{Context, FontSource, Ui};

use er_overlay_common::OverlayConfig;

/// Window font scale multiplier. The TTF atlas is already built at `config.text_size` px.
pub fn overlay_font_scale(config: &OverlayConfig) -> f32 {
    config.scale.max(0.5)
}

/// Line height at a given window font scale (matches ImGui's single-line layout).
pub fn line_height_at_scale(ui: &Ui, font_scale: f32) -> f32 {
    ui.set_window_font_scale(font_scale);
    let font = ui.current_font();
    font.ascent - font.descent
}

/// Vertical offset to top-align single-line text centered in `region_h`.
pub fn centered_text_y(ui: &Ui, region_y: f32, region_h: f32, font_scale: f32) -> f32 {
    let line_h = line_height_at_scale(ui, font_scale);
    region_y + (region_h - line_h) * 0.5
}

/// Load a readable UI font (Segoe UI on Windows, default ImGui font otherwise).
pub fn setup_overlay_fonts(ctx: &mut Context, storage: &mut Vec<u8>, size_px: f32) {
    storage.clear();

    #[cfg(windows)]
    {
        if let Ok(data) = std::fs::read(r"C:\Windows\Fonts\segoeui.ttf") {
            storage.extend(data);
        }
    }

    let fonts = ctx.fonts();
    fonts.clear();

    let size = size_px.clamp(12.0, 48.0);
    if storage.is_empty() {
        fonts.add_font(&[FontSource::DefaultFontData { config: None }]);
    } else {
        fonts.add_font(&[FontSource::TtfData {
            data: storage,
            size_pixels: size,
            config: None,
        }]);
    }

    fonts.build_rgba32_texture();
}
