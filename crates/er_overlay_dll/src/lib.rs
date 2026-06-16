mod overlay_app;

use std::ffi::c_void;
use std::path::PathBuf;
use std::thread;

use hudhook::hooks::dx12::ImguiDx12Hooks;
use hudhook::{eject, Hudhook};
use tracing::error;
use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::System::LibraryLoader::{DisableThreadLibraryCalls, GetModuleFileNameW};

use crate::overlay_app::OverlayApp;

const DLL_PROCESS_ATTACH: u32 = 1;

fn dll_directory(hmodule: HINSTANCE) -> Option<PathBuf> {
    let mut buf = [0u16; 1024];
    let len = unsafe { GetModuleFileNameW(Some(hmodule.into()), &mut buf) };
    if len == 0 {
        return None;
    }
    let path = String::from_utf16_lossy(&buf[..len as usize]);
    PathBuf::from(path).parent().map(|p| p.to_path_buf())
}

/// Windows DLL entry point. Must not block or perform heavy work on attach.
///
/// # Safety
///
/// `hmodule` must be a valid module handle supplied by the loader. Called from
/// the loader lock; only lightweight setup is performed here.
#[no_mangle]
pub unsafe extern "system" fn DllMain(
    hmodule: HINSTANCE,
    reason: u32,
    _reserved: *mut c_void,
) -> bool {
    if reason == DLL_PROCESS_ATTACH {
        let _ = DisableThreadLibraryCalls(hmodule.into());
        let hmodule_addr = hmodule.0 as usize;
        thread::spawn(move || init_overlay(HINSTANCE(hmodule_addr as *mut c_void)));
    }
    true
}

fn init_overlay(hmodule: HINSTANCE) {
    if let Some(dir) = dll_directory(hmodule) {
        er_overlay_common::set_overlay_base_dir(dir);
    }

    let _log_guard = match er_overlay_common::init_file_logging("overlay", "er_overlay.log") {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Failed to init logging: {e:?}");
            return;
        }
    };

    let result = std::panic::catch_unwind(|| {
        tracing::info!("er_overlay DLL init thread started");

        let config_path = er_overlay_common::default_config_path();
        let config = er_overlay_common::load_or_create_config(&config_path).unwrap_or_else(|e| {
            tracing::error!("Config load failed: {e:?}, using defaults");
            er_overlay_common::OverlayConfig::default()
        });

        let app = OverlayApp::new(config, config_path);

        Hudhook::builder()
            .with::<ImguiDx12Hooks>(app)
            .with_hmodule(hmodule)
            .build()
            .apply()
    });

    match result {
        Ok(Ok(())) => tracing::info!("Hudhook applied successfully"),
        Ok(Err(e)) => {
            error!("Hudhook apply failed: {e:?}");
            eject();
        }
        Err(_) => {
            error!("Panic during overlay init");
            eject();
        }
    }
}
