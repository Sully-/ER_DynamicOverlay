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

fn load_png_texture(
    render_ctx: &mut dyn RenderContext,
    path: &Path,
) -> Result<TextureId, Box<dyn std::error::Error>> {
    let image = image::ImageReader::open(path)?.decode()?.into_rgba8();
    let width = image.width();
    let height = image.height();
    Ok(render_ctx.load_texture(image.as_raw(), width, height)?)
}
