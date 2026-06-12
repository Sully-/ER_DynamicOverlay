//! Detects the game UI language via Steam API (same approach as ER_boss_checklist_R).

use std::ffi::{CStr, CString};
use std::sync::OnceLock;

use tracing::debug;
use windows::core::PCSTR;
use windows::core::PCWSTR;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};

#[repr(C)]
struct ISteamApps;

type SteamAppsFn = unsafe extern "C" fn() -> *mut ISteamApps;
type GetGameLanguageFn = unsafe extern "C" fn(*mut ISteamApps) -> *const u8;

static DETECTED: OnceLock<String> = OnceLock::new();

/// Short language id for `tables/<lang>/bosses.toml` (e.g. `en`, `fr`).
pub fn detect_game_language() -> String {
    DETECTED.get_or_init(read_steam_language).clone()
}

fn read_steam_language() -> String {
    match try_read_steam_language() {
        Some(lang) => {
            debug!("Detected game language: {lang}");
            lang
        }
        None => {
            debug!("Game language detection unavailable, defaulting to en");
            "en".into()
        }
    }
}

fn try_read_steam_language() -> Option<String> {
    unsafe {
        let dll_name: Vec<u16> = "steam_api64.dll"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let handle: HMODULE = GetModuleHandleW(PCWSTR(dll_name.as_ptr())).ok()?;

        let apps_ctor: SteamAppsFn =
            std::mem::transmute(load_proc(handle, "SteamAPI_SteamApps_v008")?);
        let get_lang: GetGameLanguageFn = std::mem::transmute(load_proc(
            handle,
            "SteamAPI_ISteamApps_GetCurrentGameLanguage",
        )?);

        let apps = apps_ctor();
        if apps.is_null() {
            return None;
        }

        let lang_ptr = get_lang(apps);
        if lang_ptr.is_null() {
            return None;
        }

        let steam_lang = CStr::from_ptr(lang_ptr as *const i8)
            .to_string_lossy()
            .into_owned();
        Some(map_steam_language(&steam_lang))
    }
}

unsafe fn load_proc(handle: HMODULE, name: &str) -> Option<usize> {
    let c_name = CString::new(name).ok()?;
    GetProcAddress(handle, PCSTR(c_name.as_ptr() as *const u8)).map(|ptr| ptr as usize)
}

fn map_steam_language(steam_lang: &str) -> String {
    let lang = match steam_lang {
        "english" => "en",
        "german" => "de",
        "french" => "fr",
        "italian" => "it",
        "japanese" => "ja",
        "koreana" => "ko",
        "polish" => "pl",
        "portuguese" => "pt",
        "russian" => "ru",
        "latam" => "es",
        "spanish" => "es",
        "thai" => "th",
        "schinese" => "zh-cn",
        "tchinese" => "zh-tw",
        _ => "en",
    };
    lang.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steam_language_maps_to_short_codes() {
        assert_eq!(map_steam_language("french"), "fr");
        assert_eq!(map_steam_language("english"), "en");
    }
}
