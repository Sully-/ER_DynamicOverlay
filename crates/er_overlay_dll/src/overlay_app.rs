use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

use er_game_state::{bosses_total_count, reload_boss_table_if_modified, resolve_locale_id, GameStateReader, GameStateSource};
use er_overlay_common::{
    default_challenge_state_path, load_layout, parse_hotkey, resolve_configured_path,
    resolve_layout_path, ChallengeTracker, HotkeyBinding, LayoutConfig, OverlayConfig, OverlayKey,
};
use er_overlay_ui::{
    build_view_model, empty_view_model, render_overlay, setup_overlay_fonts, BossPanelState,
    HudDragState, IconAtlas,
};
use hudhook::ImguiRenderLoop;
use hudhook::RenderContext;
use imgui::{Context, Key};
use tracing::{debug, warn};

struct LayoutSectionState {
    active_index: usize,
    known_sections: Vec<String>,
}

pub struct OverlayApp {
    config: OverlayConfig,
    config_path: PathBuf,
    layout: Option<LayoutConfig>,
    section_state: LayoutSectionState,
    parsed_hotkey: Option<HotkeyBinding>,
    hotkey_raw: Option<String>,
    parsed_boss_hotkey: Option<HotkeyBinding>,
    boss_hotkey_raw: Option<String>,
    show_boss_panel: bool,
    boss_panel: BossPanelState,
    last_config_reload: Instant,
    reader: GameStateReader,
    font_bytes: Vec<u8>,
    icon_atlas: IconAtlas,
    /// `(use_item_icons, icon_keys)` the atlas was last loaded for. Recomputed on
    /// config/layout reload; a change flips `icons_dirty`.
    icon_signature: (bool, Vec<String>),
    icons_dirty: bool,
    hud_drag: HudDragState,
    view_model: er_overlay_ui::OverlayViewModel,
    last_state_poll: Instant,
    boss_table_mtime: Option<SystemTime>,
    active_boss_locale: Option<String>,
    challenge: ChallengeTracker,
}

impl OverlayApp {
    pub fn new(config: OverlayConfig, config_path: PathBuf) -> Self {
        let layout = Self::load_layout_from_config(&config);
        let hotkey_raw = config.layout_section_hotkey.clone();
        let parsed_hotkey = hotkey_raw.as_deref().and_then(parse_hotkey);
        if hotkey_raw.is_some() && parsed_hotkey.is_none() {
            warn!("Invalid layout_section_hotkey: {:?}", hotkey_raw);
        }
        let boss_hotkey_raw = config.boss_panel_hotkey.clone();
        let parsed_boss_hotkey = boss_hotkey_raw.as_deref().and_then(parse_hotkey);
        if boss_hotkey_raw.is_some() && parsed_boss_hotkey.is_none() {
            warn!("Invalid boss_panel_hotkey: {:?}", boss_hotkey_raw);
        }
        let reader = GameStateReader::new();
        let boss_panel = BossPanelState::default();
        let show_boss_panel = config.boss_panel_visible;
        let icon_signature = Self::icon_signature_for(&config, layout.as_ref());
        let challenge =
            ChallengeTracker::new(config.challenge.clone(), default_challenge_state_path());
        let mut boss_table_mtime = None;
        let mut active_boss_locale = None;
        let locale_id = resolve_locale_id(config.boss_locale.as_deref());
        reload_boss_table_if_modified(
            &er_overlay_common::default_base_dir(),
            &locale_id,
            None,
            &mut boss_table_mtime,
            &mut active_boss_locale,
        );
        let view_model = empty_view_model(config.boss_panel_scope);
        let mut app = Self {
            config,
            config_path,
            layout,
            section_state: LayoutSectionState {
                active_index: 0,
                known_sections: Vec::new(),
            },
            parsed_hotkey,
            hotkey_raw,
            parsed_boss_hotkey,
            boss_hotkey_raw,
            show_boss_panel,
            boss_panel,
            last_config_reload: Instant::now(),
            reader,
            font_bytes: Vec::new(),
            icon_atlas: IconAtlas::new(),
            icon_signature,
            icons_dirty: true,
            hud_drag: HudDragState::default(),
            view_model,
            last_state_poll: Instant::now(),
            boss_table_mtime,
            active_boss_locale,
            challenge,
        };
        app.sync_section_state();
        app.maybe_reload_boss_table();
        app
    }

    /// `(use_item_icons, icon_keys)` referenced by a config + layout pair.
    fn icon_signature_for(
        config: &OverlayConfig,
        layout: Option<&LayoutConfig>,
    ) -> (bool, Vec<String>) {
        let keys = layout.map(|l| l.collect_icon_keys()).unwrap_or_default();
        (config.use_item_icons, keys)
    }

    fn refresh_view_model(&mut self) {
        self.reader.poll();
        self.view_model.bosses_total = bosses_total_count() as u32;
        if !self.reader.is_ready() {
            debug!("Skipping build_view_model: game state not ready yet");
            return;
        }
        let data_refs = self
            .layout
            .as_ref()
            .map(|l| l.collect_data_refs())
            .unwrap_or_default();
        let equipped_refs = self
            .layout
            .as_ref()
            .map(|l| l.collect_equipped_refs())
            .unwrap_or_default();
        let challenge_snapshot = if self.reader.challenge_update_ready() {
            self.challenge.update(
                self.reader.get_death_count(),
                self.reader.get_killed_boss_count(),
                self.reader.get_flag(self.config.challenge.start_flag),
            )
        } else {
            self.challenge.snapshot()
        };
        self.challenge.flush();
        self.view_model = build_view_model(
            &self.reader,
            &data_refs,
            &equipped_refs,
            self.config.boss_panel_scope,
            challenge_snapshot,
        );
        self.last_state_poll = Instant::now();
    }

    fn maybe_refresh_view_model(&mut self) {
        if self.last_state_poll.elapsed() >= Duration::from_millis(250) {
            self.refresh_view_model();
        }
    }

    fn load_layout_from_config(config: &OverlayConfig) -> Option<LayoutConfig> {
        let base_dir = er_overlay_common::default_base_dir();
        let path = resolve_layout_path(&base_dir, config.layout_file.as_deref())?;
        match load_layout(&path) {
            Ok(layout) => {
                debug!(
                    "Loaded layout from {} ({} sections, section 0: {} tiles)",
                    path.display(),
                    layout.section_count(),
                    layout.tiles_for_section(0).len()
                );
                Some(layout)
            }
            Err(e) => {
                warn!("Layout load failed ({}): {e:?}", path.display());
                None
            }
        }
    }

    fn sync_hotkey(&mut self) {
        let raw = self.config.layout_section_hotkey.clone();
        if self.hotkey_raw.as_ref() != raw.as_ref() {
            self.hotkey_raw = raw.clone();
            self.parsed_hotkey = raw.as_deref().and_then(parse_hotkey);
            if raw.is_some() && self.parsed_hotkey.is_none() {
                warn!("Invalid layout_section_hotkey: {:?}", raw);
            }
        }

        let boss_raw = self.config.boss_panel_hotkey.clone();
        if self.boss_hotkey_raw.as_ref() != boss_raw.as_ref() {
            self.boss_hotkey_raw = boss_raw.clone();
            self.parsed_boss_hotkey = boss_raw.as_deref().and_then(parse_hotkey);
            if boss_raw.is_some() && self.parsed_boss_hotkey.is_none() {
                warn!("Invalid boss_panel_hotkey: {:?}", boss_raw);
            }
        }
    }

    fn sync_section_state(&mut self) {
        let Some(layout) = self.layout.as_ref() else {
            self.section_state.active_index = 0;
            self.section_state.known_sections.clear();
            return;
        };

        let names: Vec<String> = layout
            .section_names()
            .into_iter()
            .map(str::to_string)
            .collect();
        if self.section_state.known_sections == names {
            let max = layout.section_count().saturating_sub(1);
            if self.section_state.active_index > max {
                self.section_state.active_index = max;
            }
            if layout
                .tiles_for_section(self.section_state.active_index)
                .is_empty()
                && max > 0
            {
                self.section_state.active_index = layout
                    .resolve_default_section_index(self.config.default_layout_section.as_deref())
                    .min(max);
            }
            return;
        }

        let old_name = self
            .section_state
            .known_sections
            .get(self.section_state.active_index)
            .map(String::as_str);
        let new_index = old_name
            .and_then(|name| layout.section_index(name))
            .unwrap_or_else(|| {
                layout.resolve_default_section_index(self.config.default_layout_section.as_deref())
            });
        let max = layout.section_count().saturating_sub(1);
        self.section_state.active_index = new_index.min(max);
        self.section_state.known_sections = names;

        if layout
            .tiles_for_section(self.section_state.active_index)
            .is_empty()
        {
            self.section_state.active_index = layout
                .resolve_default_section_index(self.config.default_layout_section.as_deref())
                .min(max);
        }
    }

    fn maybe_toggle_boss_panel(&mut self, ui: &imgui::Ui) {
        let Some(hk) = self.parsed_boss_hotkey else {
            return;
        };
        if !modifiers_match(ui, hk) {
            return;
        }
        let key = overlay_key_to_imgui(hk.key);
        if ui.is_key_pressed_no_repeat(key) {
            if !self.section_allows_boss_panel(self.section_state.active_index) {
                return;
            }
            let opening = !self.show_boss_panel;
            self.show_boss_panel = !self.show_boss_panel;
            if opening {
                self.boss_panel.on_reopened();
                self.refresh_view_model();
            }
            debug!("Boss panel toggled: {}", self.show_boss_panel);
        }
    }

    /// Index of the compact HUD section (minimalist tracker); boss panel stays with it only.
    fn compact_layout_section_index(&self) -> usize {
        let Some(layout) = self.layout.as_ref() else {
            return 0;
        };
        if let Some(name) = self.config.default_layout_section.as_deref() {
            if let Some(idx) = layout.section_index(name) {
                return idx;
            }
        }
        layout.resolve_default_section_index(None)
    }

    fn section_allows_boss_panel(&self, section_index: usize) -> bool {
        section_index == self.compact_layout_section_index()
    }

    fn hide_boss_panel_for_layout(&mut self) {
        if self.show_boss_panel {
            self.show_boss_panel = false;
            debug!("Boss panel hidden (extended layout)");
        }
    }

    fn maybe_cycle_section(&mut self, ui: &imgui::Ui) {
        let Some(hk) = self.parsed_hotkey else {
            return;
        };
        let Some(layout) = self.layout.as_ref() else {
            return;
        };
        if layout.section_count() < 2 {
            return;
        }
        if !modifiers_match(ui, hk) {
            return;
        }
        let key = overlay_key_to_imgui(hk.key);
        if ui.is_key_pressed_no_repeat(key) {
            self.section_state.active_index =
                (self.section_state.active_index + 1) % layout.section_count();
            if !self.section_allows_boss_panel(self.section_state.active_index) {
                self.hide_boss_panel_for_layout();
            }
        }
    }

    fn maybe_reload_boss_table(&mut self) {
        let locale_id = resolve_locale_id(self.config.boss_locale.as_deref());
        if reload_boss_table_if_modified(
            &er_overlay_common::default_base_dir(),
            &locale_id,
            None,
            &mut self.boss_table_mtime,
            &mut self.active_boss_locale,
        ) {
            self.reader.invalidate_boss_cache();
            self.refresh_view_model();
        }
    }

    fn maybe_reload_config(&mut self) {
        if self.last_config_reload.elapsed() < Duration::from_secs(2) {
            return;
        }
        self.last_config_reload = Instant::now();
        match er_overlay_common::load_or_create_config(&self.config_path) {
            Ok(cfg) => {
                let locale_settings_changed = self.config.boss_locale != cfg.boss_locale;
                self.layout = Self::load_layout_from_config(&cfg);
                self.config = cfg;
                self.challenge.sync_config(&self.config.challenge);
                if locale_settings_changed {
                    self.boss_table_mtime = None;
                    self.active_boss_locale = None;
                }
                self.maybe_reload_boss_table();
                self.sync_hotkey();
                self.sync_section_state();
                // Reload icons only when the referenced keys (or the toggle)
                // actually changed — avoids reloading textures every reload tick.
                let new_signature = Self::icon_signature_for(&self.config, self.layout.as_ref());
                if new_signature != self.icon_signature {
                    self.icon_signature = new_signature;
                    self.icons_dirty = true;
                }
            }
            Err(e) => warn!("Config reload failed: {e:?}"),
        }
    }

    /// Loads the icon atlas for the current `icon_signature`. Cheap to call every
    /// frame: it no-ops unless `icons_dirty` is set (initial load or signature
    /// change after a config/layout reload).
    fn load_icons_if_dirty(&mut self, render_ctx: &mut dyn RenderContext) {
        if !self.icons_dirty {
            return;
        }
        let base_dir = er_overlay_common::default_base_dir();
        let icons_dir = resolve_configured_path(
            self.config.icons_dir.as_deref().map(std::path::Path::new),
            &base_dir,
        );
        let (enabled, keys) = &self.icon_signature;
        debug!(
            "Loading {} item icon(s) from {}",
            keys.len(),
            icons_dir.display()
        );
        self.icon_atlas
            .load_keys(render_ctx, &icons_dir, keys, *enabled);
        self.icons_dirty = false;
    }
}

fn modifiers_match(ui: &imgui::Ui, hk: HotkeyBinding) -> bool {
    let io = ui.io();
    io.key_ctrl == hk.ctrl && io.key_alt == hk.alt && io.key_shift == hk.shift
}

fn overlay_key_to_imgui(key: OverlayKey) -> Key {
    match key {
        OverlayKey::F1 => Key::F1,
        OverlayKey::F2 => Key::F2,
        OverlayKey::F3 => Key::F3,
        OverlayKey::F4 => Key::F4,
        OverlayKey::F5 => Key::F5,
        OverlayKey::F6 => Key::F6,
        OverlayKey::F7 => Key::F7,
        OverlayKey::F8 => Key::F8,
        OverlayKey::F9 => Key::F9,
        OverlayKey::F10 => Key::F10,
        OverlayKey::F11 => Key::F11,
        OverlayKey::F12 => Key::F12,
        OverlayKey::A => Key::A,
        OverlayKey::B => Key::B,
        OverlayKey::C => Key::C,
        OverlayKey::D => Key::D,
        OverlayKey::E => Key::E,
        OverlayKey::F => Key::F,
        OverlayKey::G => Key::G,
        OverlayKey::H => Key::H,
        OverlayKey::I => Key::I,
        OverlayKey::J => Key::J,
        OverlayKey::K => Key::K,
        OverlayKey::L => Key::L,
        OverlayKey::M => Key::M,
        OverlayKey::N => Key::N,
        OverlayKey::O => Key::O,
        OverlayKey::P => Key::P,
        OverlayKey::Q => Key::Q,
        OverlayKey::R => Key::R,
        OverlayKey::S => Key::S,
        OverlayKey::T => Key::T,
        OverlayKey::U => Key::U,
        OverlayKey::V => Key::V,
        OverlayKey::W => Key::W,
        OverlayKey::X => Key::X,
        OverlayKey::Y => Key::Y,
        OverlayKey::Z => Key::Z,
        OverlayKey::Key0 => Key::Alpha0,
        OverlayKey::Key1 => Key::Alpha1,
        OverlayKey::Key2 => Key::Alpha2,
        OverlayKey::Key3 => Key::Alpha3,
        OverlayKey::Key4 => Key::Alpha4,
        OverlayKey::Key5 => Key::Alpha5,
        OverlayKey::Key6 => Key::Alpha6,
        OverlayKey::Key7 => Key::Alpha7,
        OverlayKey::Key8 => Key::Alpha8,
        OverlayKey::Key9 => Key::Alpha9,
        OverlayKey::GraveAccent => Key::GraveAccent,
        OverlayKey::Minus => Key::Minus,
        OverlayKey::Equal => Key::Equal,
        OverlayKey::LeftBracket => Key::LeftBracket,
        OverlayKey::RightBracket => Key::RightBracket,
        OverlayKey::Backslash => Key::Backslash,
        OverlayKey::Semicolon => Key::Semicolon,
        OverlayKey::Apostrophe => Key::Apostrophe,
        OverlayKey::Comma => Key::Comma,
        OverlayKey::Period => Key::Period,
        OverlayKey::Slash => Key::Slash,
    }
}

impl ImguiRenderLoop for OverlayApp {
    fn initialize(&mut self, ctx: &mut Context, render_ctx: &mut dyn RenderContext) {
        debug!("ImGui initialize");
        setup_overlay_fonts(ctx, &mut self.font_bytes, self.config.text_size);
        ctx.io_mut().config_windows_move_from_title_bar_only = false;
        self.load_icons_if_dirty(render_ctx);
        self.refresh_view_model();
        let style = ctx.style_mut();
        style.window_rounding = 6.0;
        style.frame_rounding = 4.0;
        style.colors[imgui::StyleColor::WindowBg as usize] =
            imgui::ImColor32::from_rgba(12, 12, 18, 180).into();
        style.colors[imgui::StyleColor::Text as usize] =
            imgui::ImColor32::from_rgba(245, 245, 250, 255).into();
    }

    fn before_render(&mut self, _ctx: &mut Context, render_ctx: &mut dyn RenderContext) {
        // Picks up icon-key changes produced by `maybe_reload_config` (runs in
        // `render`). `RenderContext` is only available here, not in `render`.
        self.load_icons_if_dirty(render_ctx);
    }

    fn render(&mut self, ui: &mut imgui::Ui) {
        self.maybe_reload_config();
        self.maybe_cycle_section(ui);
        self.maybe_toggle_boss_panel(ui);
        self.maybe_refresh_view_model();
        let vm = &self.view_model;
        let atlas = if self.config.use_item_icons && self.icon_atlas.is_loaded() {
            Some(&self.icon_atlas)
        } else {
            None
        };
        let show_boss_panel =
            self.show_boss_panel && self.section_allows_boss_panel(self.section_state.active_index);

        render_overlay(
            ui,
            &self.config,
            vm,
            atlas,
            self.layout.as_ref(),
            self.section_state.active_index,
            &mut self.hud_drag,
            show_boss_panel,
            &mut self.boss_panel,
        );
    }
}
