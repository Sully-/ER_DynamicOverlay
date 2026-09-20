use std::collections::HashMap;
use std::path::Path;

use er_game_state::good_by_key;
use hudhook::RenderContext;
use imgui::{ImColor32, TextureId, Ui};
use tracing::warn;

pub struct IconAtlas {
    textures: HashMap<String, TextureId>,
}

impl IconAtlas {
    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
        }
    }

    pub fn is_loaded(&self) -> bool {
        !self.textures.is_empty()
    }

    pub fn load_keys(
        &mut self,
        render_ctx: &mut dyn RenderContext,
        icons_dir: &Path,
        keys: &[String],
        enabled: bool,
    ) {
        self.textures.clear();

        if !enabled {
            return;
        }

        if !icons_dir.is_dir() {
            warn!("Icons directory not found: {}", icons_dir.display());
            return;
        }

        for key in keys {
            let file = good_by_key(key)
                .map(|g| g.file)
                .unwrap_or_else(|| format!("{}.png", key));
            let path = icons_dir.join(&file);
            if !path.is_file() {
                continue;
            }
            match load_png_texture(render_ctx, &path) {
                Ok(tex_id) => {
                    self.textures.insert(key.clone(), tex_id);
                }
                Err(err) => {
                    warn!(key = %key, ?err, "Failed to load icon PNG");
                }
            }
        }

        if self.textures.is_empty() {
            warn!(
                "No item icons loaded from {} — run tools/goods/fetch_goods_icons.py or set ELDEN_RING_GAME",
                icons_dir.display()
            );
        }
    }

    /// Absolute draw via DrawList (does not alter ImGui layout).
    pub fn draw_key_at(
        &self,
        ui: &Ui,
        key: &str,
        pos: [f32; 2],
        size: f32,
        tint: ImColor32,
    ) -> bool {
        let Some(tex_id) = self.textures.get(key) else {
            return false;
        };
        let draw = ui.get_window_draw_list();
        draw.add_image(*tex_id, pos, [pos[0] + size, pos[1] + size])
            .col(tint)
            .build();
        true
    }
}

impl Default for IconAtlas {
    fn default() -> Self {
        Self::new()
    }
}

/// hudhook uploads a single mip (`MaxLOD = 0`). Minifying a 1k PNG onto a ~64px
/// tile then aliases badly (pixelated text). Pre-filter to this edge on the CPU.
const MAX_ICON_GPU_EDGE: u32 = 256;

fn prepare_icon_rgba(image: image::RgbaImage) -> image::RgbaImage {
    let (w, h) = image.dimensions();
    let longest = w.max(h);
    if longest <= MAX_ICON_GPU_EDGE {
        return image;
    }
    let scale = MAX_ICON_GPU_EDGE as f32 / longest as f32;
    let nw = ((w as f32 * scale).round() as u32).max(1);
    let nh = ((h as f32 * scale).round() as u32).max(1);
    image::imageops::resize(&image, nw, nh, image::imageops::FilterType::Lanczos3)
}

fn load_png_texture(
    render_ctx: &mut dyn RenderContext,
    path: &Path,
) -> Result<TextureId, Box<dyn std::error::Error>> {
    let image = prepare_icon_rgba(image::ImageReader::open(path)?.decode()?.into_rgba8());
    let width = image.width();
    let height = image.height();
    Ok(render_ctx.load_texture(image.as_raw(), width, height)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_icons_are_uploaded_unchanged() {
        let img = image::RgbaImage::new(128, 128);
        let out = prepare_icon_rgba(img);
        assert_eq!(out.dimensions(), (128, 128));
    }

    #[test]
    fn large_icons_are_downscaled_to_max_edge() {
        let img = image::RgbaImage::new(1042, 1042);
        let out = prepare_icon_rgba(img);
        assert_eq!(out.dimensions(), (MAX_ICON_GPU_EDGE, MAX_ICON_GPU_EDGE));
    }
}
