use fromsoftware_shared::game_version::{DetectError, GameVersion, LANG_ID_EN, LANG_ID_JP};
use pelite::pe64::PeView;
use tracing::{info, warn};
use windows::core::PCSTR;
use windows::Win32::System::LibraryLoader::GetModuleHandleA;

/// Human-readable list of game builds supported by the bundled `eldenring` crate.
pub const SUPPORTED_GAME_VERSIONS: &str = "2.6.2.0 (WW/EN), 2.6.2.1 (JP)";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportedGameVersion {
    Ww262,
    Jp2621,
}

impl SupportedGameVersion {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ww262 => "2.6.2.0 WW",
            Self::Jp2621 => "2.6.2.1 JP",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ErGameVersion {
    Ww262,
    Jp2621,
}

impl From<ErGameVersion> for SupportedGameVersion {
    fn from(v: ErGameVersion) -> Self {
        match v {
            ErGameVersion::Ww262 => Self::Ww262,
            ErGameVersion::Jp2621 => Self::Jp2621,
        }
    }
}

impl GameVersion for ErGameVersion {
    const NAME: &'static str = "elden ring";

    fn from_lang_version(lang_id: u16, version: &str) -> Option<Self> {
        match (lang_id, version) {
            (LANG_ID_EN, "2.6.2.0") => Some(Self::Ww262),
            (LANG_ID_JP, "2.6.2.1") => Some(Self::Jp2621),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GameExeProbe {
    pub detected_version: Option<String>,
    pub supported: Option<SupportedGameVersion>,
    pub error: Option<String>,
}

impl GameExeProbe {
    pub fn is_supported(&self) -> bool {
        self.supported.is_some()
    }
}

/// Inspects the loaded `eldenring.exe` PE metadata without touching game memory.
pub fn probe_game_exe() -> GameExeProbe {
    let module = unsafe {
        PeView::module(GetModuleHandleA(PCSTR(std::ptr::null())).unwrap().0 as *const u8)
    };
    match ErGameVersion::detect(&module) {
        Ok(supported) => {
            let supported: SupportedGameVersion = supported.into();
            GameExeProbe {
                detected_version: Some(supported.label().to_string()),
                supported: Some(supported),
                error: None,
            }
        }
        Err(e) => {
            let detected_version = match &e {
                DetectError::UnsupportedVersion(v) => Some(v.clone()),
                _ => None,
            };
            GameExeProbe {
                detected_version,
                supported: None,
                error: Some(e.to_string()),
            }
        }
    }
}

pub fn log_startup_context(overlay_version: &str) {
    info!("er_overlay version {overlay_version}");
    log_probe(&probe_game_exe());
}

pub fn log_probe(probe: &GameExeProbe) {
    match probe.supported {
        Some(v) => info!(
            "Game executable supported ({}) — matches eldenring 0.14",
            v.label()
        ),
        None => warn!(
            detected = ?probe.detected_version,
            error = probe.error.as_deref().unwrap_or("unknown"),
            "Game executable is not supported. Expected: {SUPPORTED_GAME_VERSIONS}. \
             Metrics will show '---' until you update the game or the overlay. \
             Set show_debug = true in er_overlay.toml for live pointer status."
        ),
    }
}

pub fn log_pointer_summary(gamedata: bool, event_flags: bool, world_chr: bool, field_area: bool) {
    info!(
        "Game pointers resolved: GameDataMan={gamedata} EventFlagMan={event_flags} \
         WorldChrMan={world_chr} FieldArea={field_area}"
    );
}
