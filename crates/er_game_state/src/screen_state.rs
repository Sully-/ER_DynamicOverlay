//! Gameplay stability probes aligned with [EROverlay](https://github.com/soarqin/EROverlay)
//! (`Hooking::screenState` / `readInGameTime` gates in `BossDataSet::update`).

use eldenring::cs::CSMenuManImp;
use fromsoftware_shared::FromStatic;

/// `CSMenuManImp` menu-info block offset (game 1.12+ / our supported 2.6.2 builds).
const MENU_INFO_OFFSET: usize = 0x720;
const SCREEN_STATE_FIELD_OFFSET: usize = 0x10;
const MOUSE_CURSOR_FLAGS_OFFSET: usize = 0xAC;

/// Reads the frontend screen-state integer from `CSMenuManImp`.
///
/// Returns `None` when the singleton is unavailable. EROverlay treats a missing
/// menu manager as non-zero (skip updates); callers should do the same.
pub fn read_screen_state() -> Option<i32> {
    let menu_man = unsafe { CSMenuManImp::instance().ok()? };
    let base = menu_man as *const CSMenuManImp as *const u8;
    // SAFETY: offset matches EROverlay `Hooking::screenState()` for 1.12+ builds.
    Some(unsafe { *(base.add(MENU_INFO_OFFSET + SCREEN_STATE_FIELD_OFFSET) as *const i32) })
}

/// Mirrors EROverlay's `Hooking::showMouseCursor`: toggle the game menu cursor bit.
pub fn set_menu_cursor_visible(visible: bool) -> Option<()> {
    let menu_man = unsafe { CSMenuManImp::instance().ok()? };
    let base = menu_man as *const CSMenuManImp as *mut u8;
    let flags = unsafe { base.add(MOUSE_CURSOR_FLAGS_OFFSET) };
    unsafe {
        if visible {
            *flags |= 1;
        } else {
            *flags &= !1;
        }
    }
    Some(())
}

/// Whether challenge counters should be polled this frame.
///
/// Mirrors EROverlay: skip when `screenState != 0` or in-game time is not running yet.
pub fn challenge_update_ready(igt_ms: u32, screen_state: Option<i32>) -> bool {
    if igt_ms == 0 {
        return false;
    }
    matches!(screen_state, Some(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn challenge_ready_requires_running_igt_and_zero_screen_state() {
        assert!(!challenge_update_ready(0, Some(0)));
        assert!(!challenge_update_ready(1000, None));
        assert!(!challenge_update_ready(1000, Some(1)));
        assert!(challenge_update_ready(1000, Some(0)));
    }
}
