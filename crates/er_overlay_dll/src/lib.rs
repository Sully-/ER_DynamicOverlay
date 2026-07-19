mod overlay_app;

use std::ffi::c_void;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::thread;

use hudhook::hooks::dx12::ImguiDx12Hooks;
use hudhook::{eject, Hudhook};
use tracing::{error, info, warn};
use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::System::LibraryLoader::{DisableThreadLibraryCalls, GetModuleFileNameW};

use crate::overlay_app::OverlayApp;

const DLL_PROCESS_ATTACH: u32 = 1;

/// Last-resort sink written directly to disk, independent of the tracing worker.
const PANIC_LOG_FILE: &str = "er_overlay_panic.log";

/// Keeps the `tracing-appender` worker thread alive for the whole process lifetime.
///
/// Without this, the guard would be dropped when `init_overlay` returns (right after
/// `Hudhook::apply()` installs the hooks and hands control back), stopping the log
/// writer and silently discarding every record emitted afterwards from the game's
/// render thread — exactly the moment the overlay shows up or fails to.
static LOG_GUARD: OnceLock<er_overlay_common::LogGuard> = OnceLock::new();

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
    // Resolve the directory the DLL lives in so config/assets/logs sit next to it.
    let base_dir = dll_directory(hmodule);
    if let Some(dir) = base_dir.clone() {
        er_overlay_common::set_overlay_base_dir(dir);
    }

    // Load the config *before* logging so we know whether logging is enabled. Any
    // error is captured and reported once logging (if any) is up, rather than swallowed.
    let config_path = er_overlay_common::default_config_path();
    let (config, config_error) = match er_overlay_common::load_or_create_config(&config_path) {
        Ok(cfg) => (cfg, None),
        Err(e) => (
            er_overlay_common::OverlayConfig::default(),
            Some(format!("{e:?}")),
        ),
    };

    // Logging is opt-in via er_overlay.toml (`log_enabled`), with an env-var escape hatch.
    if config.log_enabled || env_flag_enabled("ER_OVERLAY_LOG") {
        match er_overlay_common::init_file_logging(
            "overlay",
            "er_overlay.log",
            config.log_level.as_deref(),
        ) {
            Ok(guard) => {
                let _ = LOG_GUARD.set(guard);
            }
            Err(e) => append_diag_file(&format!("Failed to init logging: {e:?}")),
        }
        install_panic_hook();
    }

    // Beyond this point tracing macros are cheap no-ops unless logging was enabled above.
    info!(
        "er_overlay DLL init thread started (v{})",
        env!("CARGO_PKG_VERSION")
    );
    match &base_dir {
        Some(dir) => info!("Overlay base dir: {}", dir.display()),
        None => warn!(
            "GetModuleFileNameW failed; falling back to the game exe directory for the base path"
        ),
    }
    match &config_error {
        Some(err) => error!(
            "Config load failed ({err}); using defaults ({})",
            config_path.display()
        ),
        None => info!("Config loaded from {}", config_path.display()),
    }

    info!("Building OverlayApp");
    let app = OverlayApp::new(config, config_path);

    info!("Installing DX12 hooks via hudhook");
    let result = Hudhook::builder()
        .with::<ImguiDx12Hooks>(app)
        .with_hmodule(hmodule)
        .build()
        .apply();

    match result {
        Ok(()) => info!("Hudhook applied successfully; waiting for the game's render loop"),
        Err(e) => {
            error!("Hudhook apply failed: {e:?}; ejecting DLL");
            eject();
        }
    }
}

/// Treats an env var as a boolean flag (`1`/`true`/any non-empty value except `0`/`false`).
fn env_flag_enabled(name: &str) -> bool {
    std::env::var(name)
        .map(|v| {
            let v = v.trim();
            !v.is_empty() && v != "0" && !v.eq_ignore_ascii_case("false")
        })
        .unwrap_or(false)
}

/// Installs a panic hook that records the panic to the log and to a last-resort file.
///
/// This matters because the workspace builds with `panic = "abort"`, so `catch_unwind`
/// never catches anything: a panic tears the whole game down. The hook still runs
/// *before* the abort, giving us a written trace instead of a silent crash.
fn install_panic_hook() {
    static INSTALLED: OnceLock<()> = OnceLock::new();
    if INSTALLED.set(()).is_err() {
        return;
    }

    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());
        let message = panic_payload_str(info.payload());
        let thread = std::thread::current();
        let thread_name = thread.name().unwrap_or("unnamed");
        let backtrace = std::backtrace::Backtrace::force_capture();

        let report =
            format!("PANIC on thread '{thread_name}' at {location}: {message}\n{backtrace}");
        error!(target: "er_overlay", "{report}");
        append_diag_file(&report);

        previous(info);
    }));
}

fn panic_payload_str(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic payload>".to_string()
    }
}

/// Appends a diagnostic record straight to `logs/er_overlay_panic.log`, bypassing the
/// tracing worker so crashes and logging-init failures are captured even if tracing is
/// unavailable. Falls back to stderr if the file cannot be written.
fn append_diag_file(contents: &str) {
    let dir = er_overlay_common::log_directory();
    if std::fs::create_dir_all(&dir).is_err() {
        eprintln!("er_overlay: {contents}");
        return;
    }
    match OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(PANIC_LOG_FILE))
    {
        Ok(mut file) => {
            let _ = writeln!(file, "{contents}");
        }
        Err(_) => eprintln!("er_overlay: {contents}"),
    }
}
