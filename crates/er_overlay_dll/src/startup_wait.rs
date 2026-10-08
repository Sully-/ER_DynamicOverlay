//! Defers overlay setup until the game has a window.
//!
//! ModEngine2 loads `external_dlls` at process start, long before the game has created its
//! window and DX12 swap chain. Installing the hudhook hooks that early makes it build its probe
//! device/swap chain while the game is still booting and attach through a different code path
//! than when the DLL is injected into a running game, which is the path known to work. Waiting
//! for the game window puts both loading methods in the same conditions.

use std::thread;
use std::time::{Duration, Instant};

use tracing::{info, warn};
use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetWindowThreadProcessId, IsWindowVisible,
};

const POLL_INTERVAL: Duration = Duration::from_millis(250);

/// Past this, the hooks are installed anyway so a game whose window we fail to detect still
/// gets an overlay (with the previous, immediate-install behavior).
const TIMEOUT: Duration = Duration::from_secs(90);

/// The window shows up shortly before the swap chain is created; this margin lets the render
/// loop start so hudhook attaches to a running game, as with a late injection.
const SETTLE_DELAY: Duration = Duration::from_secs(2);

/// Blocks the calling thread until the current process has a visible top-level window.
///
/// Returns immediately when the window already exists (DLL injected into a running game).
pub fn wait_for_game_window() {
    if process_has_visible_window() {
        info!("Game window already present; installing hooks now");
        return;
    }

    info!("Loaded before the game window exists (e.g. via ModEngine2); waiting for it");
    let start = Instant::now();
    loop {
        thread::sleep(POLL_INTERVAL);
        if process_has_visible_window() {
            info!(
                "Game window detected after {} ms; installing hooks in {} ms",
                start.elapsed().as_millis(),
                SETTLE_DELAY.as_millis()
            );
            thread::sleep(SETTLE_DELAY);
            return;
        }
        if start.elapsed() >= TIMEOUT {
            warn!(
                "No game window after {} s; installing hooks anyway",
                TIMEOUT.as_secs()
            );
            return;
        }
    }
}

fn process_has_visible_window() -> bool {
    let mut found = false;
    // EnumWindows reports an error when the callback stops the enumeration early, which is
    // exactly the "found" case, so its result carries no information here.
    let _ = unsafe {
        EnumWindows(
            Some(visible_window_callback),
            LPARAM(&mut found as *mut bool as isize),
        )
    };
    found
}

unsafe extern "system" fn visible_window_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid != GetCurrentProcessId() || !IsWindowVisible(hwnd).as_bool() || is_console(hwnd) {
        return BOOL::from(true);
    }
    *(lparam.0 as *mut bool) = true;
    BOOL::from(false)
}

/// ModEngine2 can open a debug console; it belongs to the process but is not the game window.
fn is_console(hwnd: HWND) -> bool {
    let mut buf = [0u16; 64];
    let len = unsafe { GetClassNameW(hwnd, &mut buf) };
    len > 0 && String::from_utf16_lossy(&buf[..len as usize]) == "ConsoleWindowClass"
}
