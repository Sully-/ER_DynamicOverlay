use std::sync::atomic::{AtomicU32, Ordering};

use imgui::{Context, FontSource, Ui};

use er_overlay_common::OverlayConfig;

/// Bounds for the rasterized atlas size. The atlas is baked once at
/// `text_size * scale`; clamping keeps the font texture a sane size.
const MIN_ATLAS_PX: f32 = 12.0;
const MAX_ATLAS_PX: f32 = 96.0;

/// Fixed pixel size of ImGui's built-in bitmap font (used only when Segoe UI is absent).
const DEFAULT_FONT_PX: f32 = 13.0;

/// Pixel size the TTF atlas was actually rasterized at, stored as `f32` bits
/// (`0` = not built yet). Recorded by [`setup_overlay_fonts`] and read back by
/// [`overlay_font_scale`] so the window font scale renders at the desired size
/// regardless of what size the atlas happened to be baked at.
static ATLAS_FONT_PX: AtomicU32 = AtomicU32::new(0);

fn record_atlas_font_px(px: f32) {
    ATLAS_FONT_PX.store(px.to_bits(), Ordering::Relaxed);
}

fn atlas_font_px() -> Option<f32> {
    match ATLAS_FONT_PX.load(Ordering::Relaxed) {
        0 => None,
        bits => Some(f32::from_bits(bits)),
    }
}

/// On-screen base font size we want to render at (before per-tile sub-scales).
fn desired_font_px(config: &OverlayConfig) -> f32 {
    config.text_size * config.scale.max(0.5)
}

/// Window font-scale multiplier applied on top of the baked atlas.
///
/// Text is baked into the atlas at `text_size * scale` (see [`setup_overlay_fonts`]),
/// so at the configured scale this returns ~1.0 and glyphs render pixel-for-pixel
/// (crisp) instead of upscaling a small bitmap. If `scale` changes at runtime — the
/// atlas can't be rebuilt while injected — this ratio still resizes text correctly
/// (only re-blurring if scaled beyond the baked size).
pub fn overlay_font_scale(config: &OverlayConfig) -> f32 {
    let desired = desired_font_px(config);
    let atlas = atlas_font_px().unwrap_or_else(|| desired.clamp(MIN_ATLAS_PX, MAX_ATLAS_PX));
    desired / atlas
}

/// Line height at a given window font scale (matches ImGui's single-line layout).
///
/// `font.ascent`/`font.descent` are native-size metrics and are *not* affected by
/// `set_window_font_scale`, whereas rendered text (and `calc_text_size`) scales with it.
/// Multiply by `font_scale` so the height stays consistent with the measured text width;
/// otherwise vertical centering drifts as soon as `scale != 1`.
pub fn line_height_at_scale(ui: &Ui, font_scale: f32) -> f32 {
    ui.set_window_font_scale(font_scale);
    let font = ui.current_font();
    (font.ascent - font.descent) * font_scale
}

/// Vertical offset to top-align single-line text centered in `region_h`.
pub fn centered_text_y(ui: &Ui, region_y: f32, region_h: f32, font_scale: f32) -> f32 {
    let line_h = line_height_at_scale(ui, font_scale);
    region_y + (region_h - line_h) * 0.5
}

/// Load a readable UI font (Segoe UI on Windows, default ImGui font otherwise).
///
/// The atlas is rasterized at `text_size * scale` (clamped) so text stays crisp at the
/// configured scale rather than being upscaled from a small bitmap by the window font
/// scale. hudhook bakes this into the GPU font texture once, so the scale in effect at
/// startup is the one that renders sharpest.
pub fn setup_overlay_fonts(ctx: &mut Context, storage: &mut Vec<u8>, config: &OverlayConfig) {
    storage.clear();

    #[cfg(windows)]
    {
        if let Ok(data) = std::fs::read(r"C:\Windows\Fonts\segoeui.ttf") {
            storage.extend(data);
        }
    }

    let fonts = ctx.fonts();
    fonts.clear();

    let size = desired_font_px(config).clamp(MIN_ATLAS_PX, MAX_ATLAS_PX);
    if storage.is_empty() {
        fonts.add_font(&[FontSource::DefaultFontData { config: None }]);
        // ImGui's built-in proggy font is a fixed 13px bitmap; record its true size so
        // the window font scale still resizes text as before (fallback path only).
        record_atlas_font_px(DEFAULT_FONT_PX);
    } else {
        fonts.add_font(&[FontSource::TtfData {
            data: storage,
            size_pixels: size,
            config: None,
        }]);
        record_atlas_font_px(size);
    }

    fonts.build_rgba32_texture();
}
