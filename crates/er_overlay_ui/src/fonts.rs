use imgui::{Context, FontSource};

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
