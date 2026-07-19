use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use er_game_state::{
    bosses_total_count, clear_checks_seed_flags, reload_boss_table_if_modified,
    reload_checks_flags_if_modified, reload_checks_table_if_modified, resolve_checks_table_path,
    resolve_locale_id, GameStateReader, GameStateSource,
};
use er_overlay_common::layout::LayoutStyle;
use er_overlay_common::{
    default_base_dir, default_challenge_state_path, load_layout, parse_hotkey,
    resolve_configured_path, resolve_layout_path, ChallengeTracker, HotkeyBinding, LayoutConfig,
    OverlayConfig, OverlayKey,
};
use er_overlay_ui::{
    build_view_model, empty_view_model, render_overlay, setup_overlay_fonts, BossPanelState,
    ChecksPanelState, HudDragState, IconAtlas,
};
use hudhook::{ImguiRenderLoop, MessageFilter, RenderContext};
use imgui::{Context, Key, WindowHoveredFlags};
use tracing::{debug, info, warn};

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
    parsed_checks_hotkey: Option<HotkeyBinding>,
    checks_hotkey_raw: Option<String>,
    parsed_hide_all_hotkey: Option<HotkeyBinding>,
    hide_all_hotkey_raw: Option<String>,
    show_overlay: bool,
    show_boss_panel: bool,
    boss_panel: BossPanelState,
    show_checks_panel: bool,
    checks_panel: ChecksPanelState,
    last_config_reload: Instant,
    reader: GameStateReader,
    font_bytes: Vec<u8>,
    /// `(text_size, scale)` the font atlas is currently baked for. When it changes
    /// (config reload), the atlas is re-rasterized and re-uploaded so text stays crisp.
    applied_font_sig: Option<(f32, f32)>,
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
    checks_table_mtime: Option<SystemTime>,
    active_checks_locale: Option<String>,
    checks_flags_mtime: Option<SystemTime>,
    /// `(mtime, len)` of the watched regulation.bin, to detect a new seed.
    regulation_sig: Option<(SystemTime, u64)>,
    /// Guards against spawning the extractor while a previous run is in flight.
    extractor_running: Arc<AtomicBool>,
    challenge: ChallengeTracker,
    /// Set once the render loop draws its first frame, to log that milestone exactly once.
    first_render_logged: bool,
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
        let checks_hotkey_raw = config.checks_panel_hotkey.clone();
        let parsed_checks_hotkey = checks_hotkey_raw.as_deref().and_then(parse_hotkey);
        if checks_hotkey_raw.is_some() && parsed_checks_hotkey.is_none() {
            warn!("Invalid checks_panel_hotkey: {:?}", checks_hotkey_raw);
        }
        let hide_all_hotkey_raw = config.hide_all_hotkey.clone();
        let parsed_hide_all_hotkey = hide_all_hotkey_raw.as_deref().and_then(parse_hotkey);
        if hide_all_hotkey_raw.is_some() && parsed_hide_all_hotkey.is_none() {
            warn!("Invalid hide_all_hotkey: {:?}", hide_all_hotkey_raw);
        }
        let reader = GameStateReader::new();
        let boss_panel = BossPanelState::default();
        let show_boss_panel = config.boss_panel_visible;
        let checks_panel = ChecksPanelState::default();
        // Boss / checks / extended are mutually exclusive, including at startup: if both panels
        // are configured visible, the boss panel wins and checks stay hidden until toggled (F6).
        let show_checks_panel = config.checks_panel_visible && !show_boss_panel;
        let show_overlay = config.overlay_visible;
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
        let mut checks_table_mtime = None;
        let mut active_checks_locale = None;
        reload_checks_table_if_modified(
            &er_overlay_common::default_base_dir(),
            &locale_id,
            None,
            &mut checks_table_mtime,
            &mut active_checks_locale,
        );
        let view_model = empty_view_model(config.boss_panel_scope, config.checks_panel_scope);
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
            parsed_checks_hotkey,
            checks_hotkey_raw,
            parsed_hide_all_hotkey,
            hide_all_hotkey_raw,
            show_overlay,
            show_boss_panel,
            boss_panel,
            show_checks_panel,
            checks_panel,
            last_config_reload: Instant::now(),
            reader,
            font_bytes: Vec::new(),
            applied_font_sig: None,
            icon_atlas: IconAtlas::new(),
            icon_signature,
            icons_dirty: true,
            hud_drag: HudDragState::default(),
            view_model,
            last_state_poll: Instant::now(),
            boss_table_mtime,
            active_boss_locale,
            checks_table_mtime,
            active_checks_locale,
            checks_flags_mtime: None,
            regulation_sig: None,
            extractor_running: Arc::new(AtomicBool::new(false)),
            challenge,
            first_render_logged: false,
        };
        app.sync_section_state();
        app.maybe_reload_boss_table();
        app.maybe_sync_checks();
        info!(
            "OverlayApp built: layout={}, overlay_visible={}, boss_panel={}, checks_panel={}",
            app.layout
                .as_ref()
                .map(|_| "loaded")
                .unwrap_or("none"),
            app.show_overlay,
            app.show_boss_panel,
            app.show_checks_panel
        );
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
        let historic_refs = self
            .layout
            .as_ref()
            .map(|l| l.collect_historic_refs())
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
            &historic_refs,
            self.config.boss_panel_scope,
            self.config.checks_panel_scope,
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

        let checks_raw = self.config.checks_panel_hotkey.clone();
        if self.checks_hotkey_raw.as_ref() != checks_raw.as_ref() {
            self.checks_hotkey_raw = checks_raw.clone();
            self.parsed_checks_hotkey = checks_raw.as_deref().and_then(parse_hotkey);
            if checks_raw.is_some() && self.parsed_checks_hotkey.is_none() {
                warn!("Invalid checks_panel_hotkey: {:?}", checks_raw);
            }
        }

        let hide_raw = self.config.hide_all_hotkey.clone();
        if self.hide_all_hotkey_raw.as_ref() != hide_raw.as_ref() {
            self.hide_all_hotkey_raw = hide_raw.clone();
            self.parsed_hide_all_hotkey = hide_raw.as_deref().and_then(parse_hotkey);
            if hide_raw.is_some() && self.parsed_hide_all_hotkey.is_none() {
                warn!("Invalid hide_all_hotkey: {:?}", hide_raw);
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
            let opening = !self.show_boss_panel;
            self.show_boss_panel = !self.show_boss_panel;
            self.show_overlay = true;
            if opening {
                // Boss / checks / extended are mutually exclusive: opening boss closes checks.
                self.show_checks_panel = false;
                self.section_state.active_index = self.compact_layout_section_index();
                self.boss_panel.on_reopened();
                self.refresh_view_model();
            }
            debug!("Boss panel toggled: {}", self.show_boss_panel);
        }
    }

    fn maybe_toggle_checks_panel(&mut self, ui: &imgui::Ui) {
        let Some(hk) = self.parsed_checks_hotkey else {
            return;
        };
        if !modifiers_match(ui, hk) {
            return;
        }
        let key = overlay_key_to_imgui(hk.key);
        if ui.is_key_pressed_no_repeat(key) {
            let opening = !self.show_checks_panel;
            self.show_checks_panel = !self.show_checks_panel;
            self.show_overlay = true;
            if opening {
                // Boss / checks / extended are mutually exclusive: opening checks closes boss.
                self.show_boss_panel = false;
                self.section_state.active_index = self.compact_layout_section_index();
                self.checks_panel.on_reopened();
                self.refresh_view_model();
            }
            debug!("Checks panel toggled: {}", self.show_checks_panel);
        }
    }

    fn maybe_toggle_overlay_visibility(&mut self, ui: &imgui::Ui) {
        let Some(hk) = self.parsed_hide_all_hotkey else {
            return;
        };
        if !modifiers_match(ui, hk) {
            return;
        }
        let key = overlay_key_to_imgui(hk.key);
        if ui.is_key_pressed_no_repeat(key) {
            self.show_overlay = !self.show_overlay;
            debug!("Overlay visibility toggled: {}", self.show_overlay);
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
        if self.show_checks_panel {
            self.show_checks_panel = false;
            debug!("Checks panel hidden (extended layout)");
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
            self.show_overlay = true;
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

    /// Reloads the checks table on locale/file change, drives the per-seed extractor, and
    /// reloads `checks_flags.toml` when a new mapping is produced.
    fn maybe_sync_checks(&mut self) {
        let base = default_base_dir();
        let locale_id = resolve_locale_id(self.config.boss_locale.as_deref());
        let mut changed = reload_checks_table_if_modified(
            &base,
            &locale_id,
            None,
            &mut self.checks_table_mtime,
            &mut self.active_checks_locale,
        );

        let regulation = self
            .config
            .regulation_path
            .clone()
            .filter(|p| !p.is_empty());
        match regulation {
            Some(reg) => {
                self.maybe_run_extractor(&base, &locale_id, Path::new(&reg));
                let lot_flags_path = base.join("lot_flags.toml");
                let legacy_flags_path = base.join("checks_flags.toml");
                let flags_path = if lot_flags_path.is_file() {
                    lot_flags_path
                } else {
                    legacy_flags_path
                };
                if reload_checks_flags_if_modified(&flags_path, &mut self.checks_flags_mtime) {
                    changed = true;
                }
            }
            None => {
                // No regulation configured: ensure dynamic checks use their vanilla flags.
                self.regulation_sig = None;
                if clear_checks_seed_flags() {
                    self.checks_flags_mtime = None;
                    changed = true;
                }
            }
        }

        if changed {
            self.refresh_view_model();
        }
    }

    /// Spawns the checks extractor (in a background thread) when the watched regulation.bin
    /// changes, so the modded seed's randomized loot flags get resolved.
    fn maybe_run_extractor(&mut self, base: &Path, locale_id: &str, regulation: &Path) {
        let Ok(meta) = std::fs::metadata(regulation) else {
            return;
        };
        let sig = (
            meta.modified().ok().unwrap_or(SystemTime::UNIX_EPOCH),
            meta.len(),
        );
        if self.regulation_sig == Some(sig) {
            return;
        }
        if self.extractor_running.load(Ordering::Acquire) {
            return;
        }

        let Some(extractor) = self.resolve_extractor_path(base) else {
            warn!("regulation_path is set but er_checks_extractor was not found; dynamic checks use vanilla flags");
            // Mark as handled so we don't warn every tick for the same regulation.
            self.regulation_sig = Some(sig);
            return;
        };

        let checks_toml = {
            let p = resolve_checks_table_path(base, locale_id);
            if p.is_file() {
                p
            } else {
                resolve_checks_table_path(base, er_game_state::DEFAULT_LOCALE_ID)
            }
        };
        let out_path = base.join("lot_flags.toml");
        let goods_toml = base.join("tables").join("goods.toml");
        let layout_path = resolve_layout_path(base, self.config.layout_file.as_deref());
        let regulation = regulation.to_path_buf();
        // The game install dir holds oo2core_*.dll, which the extractor needs to decompress the
        // regulation. We run inside the game, so current_exe() is the game executable.
        let game_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf));
        let running = Arc::clone(&self.extractor_running);
        running.store(true, Ordering::Release);
        self.regulation_sig = Some(sig);

        let spawned = std::thread::Builder::new()
            .name("er_checks_extractor".into())
            .spawn(move || {
                let mut cmd = std::process::Command::new(&extractor);
                cmd.arg(&regulation).arg(&checks_toml).arg(&out_path);
                if let Some(dir) = game_dir.as_deref() {
                    cmd.arg(dir);
                }
                // `goods.toml` is normally embedded in the companion like it is in er_game_state.
                // Only pass a loose file when an explicit runtime override exists.
                if goods_toml.is_file() {
                    cmd.arg("--goods").arg(&goods_toml);
                }
                if let Some(layout) = layout_path.as_deref() {
                    cmd.arg("--layout").arg(layout);
                }
                let result = cmd.output();
                match result {
                    Ok(out) if out.status.success() => {
                        debug!(
                            "checks extractor ok: {}",
                            String::from_utf8_lossy(&out.stdout).trim()
                        );
                    }
                    Ok(out) => warn!(
                        "checks extractor failed ({}): {}",
                        out.status,
                        String::from_utf8_lossy(&out.stderr).trim()
                    ),
                    Err(e) => warn!("failed to launch checks extractor: {e}"),
                }
                running.store(false, Ordering::Release);
            });

        if let Err(e) = spawned {
            warn!("could not spawn extractor thread: {e}");
            self.extractor_running.store(false, Ordering::Release);
        }
    }

    /// Configured path, else `companion/er_checks_extractor.exe`, else `er_checks_extractor.exe`.
    fn resolve_extractor_path(&self, base: &Path) -> Option<PathBuf> {
        if let Some(raw) = self
            .config
            .checks_extractor_path
            .as_deref()
            .filter(|p| !p.is_empty())
        {
            let p = Path::new(raw);
            let resolved = if p.is_absolute() {
                p.to_path_buf()
            } else {
                base.join(p)
            };
            return resolved.is_file().then_some(resolved);
        }
        let candidates = [
            base.join("companion").join("er_checks_extractor.exe"),
            base.join("er_checks_extractor.exe"),
        ];
        candidates.into_iter().find(|p| p.is_file())
    }

    fn maybe_reload_config(&mut self) {
        if self.last_config_reload.elapsed() < Duration::from_secs(2) {
            return;
        }
        self.last_config_reload = Instant::now();
        match er_overlay_common::load_or_create_config(&self.config_path) {
            Ok(cfg) => {
                let locale_settings_changed = self.config.boss_locale != cfg.boss_locale;
                let regulation_changed = self.config.regulation_path != cfg.regulation_path;
                // When focus-keeping is turned off at runtime, release the game mouse-cursor
                // bit once. Otherwise it stays stuck at the last "visible" value we forced and
                // the game keeps the focus even though the feature is now disabled.
                let focus_disabled = self.config.keep_overlay_focus && !cfg.keep_overlay_focus;
                self.layout = Self::load_layout_from_config(&cfg);
                self.config = cfg;
                if focus_disabled {
                    let _ = er_game_state::set_menu_cursor_visible(false);
                }
                self.challenge.sync_config(&self.config.challenge);
                if locale_settings_changed {
                    self.boss_table_mtime = None;
                    self.active_boss_locale = None;
                    self.checks_table_mtime = None;
                    self.active_checks_locale = None;
                }
                if regulation_changed {
                    self.regulation_sig = None;
                }
                self.maybe_reload_boss_table();
                self.maybe_sync_checks();
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

    /// Re-rasterizes and re-uploads the font atlas when `text_size`/`scale` changed.
    ///
    /// hudhook only builds the font texture once (at renderer setup), so a live config
    /// change would otherwise keep the old baked size. We rebuild the atlas here — in
    /// `before_render`, the sanctioned place for texture uploads — and point the font
    /// atlas at a freshly loaded texture so the new size renders crisp immediately.
    fn rebuild_fonts_if_dirty(&mut self, ctx: &mut Context, render_ctx: &mut dyn RenderContext) {
        let sig = (self.config.text_size, self.config.scale);
        if self.applied_font_sig == Some(sig) {
            return;
        }
        setup_overlay_fonts(ctx, &mut self.font_bytes, &self.config);
        let fonts = ctx.fonts();
        let texture = fonts.build_rgba32_texture();
        match render_ctx.load_texture(texture.data, texture.width, texture.height) {
            Ok(id) => {
                fonts.tex_id = id;
                self.applied_font_sig = Some(sig);
                debug!(
                    "Rebuilt font atlas (text_size={}, scale={})",
                    self.config.text_size, self.config.scale
                );
            }
            Err(e) => warn!("Font atlas rebuild failed: {e:?}"),
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
        // Key milestone: reaching this means hudhook's Present hook fired and ImGui is set up.
        // If "Hudhook applied" is logged but this never is, the hook never triggered (wrong
        // renderer, another overlay taking over Present, etc.).
        info!("ImGui initialize: render hook active, setting up overlay");
        setup_overlay_fonts(ctx, &mut self.font_bytes, &self.config);
        self.applied_font_sig = Some((self.config.text_size, self.config.scale));
        ctx.io_mut().config_windows_move_from_title_bar_only = false;
        self.load_icons_if_dirty(render_ctx);
        self.refresh_view_model();
        let imgui_style = ctx.style_mut();
        imgui_style.window_rounding = 6.0;
        imgui_style.frame_rounding = 4.0;
        let default_layout_style = LayoutStyle::default();
        let layout_style = self
            .layout
            .as_ref()
            .map(|l| &l.style)
            .unwrap_or(&default_layout_style);
        let bg = layout_style.window_bg_rgba_f32();
        imgui_style.colors[imgui::StyleColor::WindowBg as usize] = imgui::ImColor32::from_rgba(
            (bg[0] * 255.0) as u8,
            (bg[1] * 255.0) as u8,
            (bg[2] * 255.0) as u8,
            (bg[3] * 255.0) as u8,
        )
        .into();
        imgui_style.colors[imgui::StyleColor::Text as usize] =
            imgui::ImColor32::from_rgba(245, 245, 250, 255).into();
    }

    fn before_render(&mut self, ctx: &mut Context, render_ctx: &mut dyn RenderContext) {
        // Picks up text_size/scale and icon-key changes produced by `maybe_reload_config`
        // (runs in `render`). `Context`/`RenderContext` are only available here, not in `render`.
        self.rebuild_fonts_if_dirty(ctx, render_ctx);
        self.load_icons_if_dirty(render_ctx);
    }

    fn render(&mut self, ui: &mut imgui::Ui) {
        if !self.first_render_logged {
            self.first_render_logged = true;
            info!(
                "First overlay frame rendered (show_overlay={})",
                self.show_overlay
            );
        }
        self.maybe_reload_config();
        self.maybe_toggle_overlay_visibility(ui);
        self.maybe_cycle_section(ui);
        self.maybe_toggle_boss_panel(ui);
        self.maybe_toggle_checks_panel(ui);
        if !self.show_overlay {
            if self.config.keep_overlay_focus {
                let _ = er_game_state::set_menu_cursor_visible(false);
            }
            return;
        }
        self.maybe_refresh_view_model();
        let vm = &self.view_model;
        let atlas = if self.config.use_item_icons && self.icon_atlas.is_loaded() {
            Some(&self.icon_atlas)
        } else {
            None
        };
        let section_allows = self.section_allows_boss_panel(self.section_state.active_index);
        let show_boss_panel = self.show_boss_panel && section_allows;
        let show_checks_panel = self.show_checks_panel && section_allows;

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
            show_checks_panel,
            &mut self.checks_panel,
        );
        if self.config.keep_overlay_focus {
            let imgui_hovered = ui.is_window_hovered_with_flags(
                WindowHoveredFlags::ANY_WINDOW
                    | WindowHoveredFlags::ALLOW_WHEN_BLOCKED_BY_ACTIVE_ITEM,
            );
            let interactive_visible = show_boss_panel || show_checks_panel || self.config.show_debug;
            let _ = er_game_state::set_menu_cursor_visible(interactive_visible || imgui_hovered);
        }
    }

    fn message_filter(&self, io: &imgui::Io) -> MessageFilter {
        if !self.config.keep_overlay_focus {
            return MessageFilter::empty();
        }
        if !self.show_overlay {
            let _ = er_game_state::set_menu_cursor_visible(false);
            return MessageFilter::empty();
        }

        let mut filter = MessageFilter::empty();
        if io.want_capture_mouse {
            filter |= MessageFilter::InputMouse | MessageFilter::InputRaw;
        }
        if io.want_capture_keyboard || io.want_text_input {
            filter |= MessageFilter::InputKeyboard | MessageFilter::InputRaw;
        }
        filter
    }
}
