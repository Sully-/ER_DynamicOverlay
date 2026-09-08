//! Version-independent resolution of the two executable addresses the overlay
//! needs from game memory.
//!
//! The `eldenring` crate resolves these from a table keyed on the exact game
//! build and panics on any build it doesn't list, so every Elden Ring patch
//! broke the overlay until the crate was regenerated and re-released. The byte
//! patterns below are the ones upstream feeds to its own table generator, so
//! scanning for them here yields the same addresses without tying us to a
//! version string.
//!
//! Everything else the overlay reads is already version-independent:
//! `CSEventFlagMan`, `WorldChrMan` and `CSMenuManImp` are looked up by name
//! through DLRF reflection, and `FieldArea` has its own scan in
//! [`crate::field_area`].

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use fromsoftware_shared::program::Program;
use pelite::pattern;
use pelite::pe64::Pe;
use tracing::{debug, warn};

/// `mov [rip+disp32], rax` storing the freshly built `GameDataMan` into its
/// global, followed by the epilogue that identifies this particular store. `$`
/// resolves the displacement so `'` captures the global's own RVA.
const GAME_DATA_MAN_PATTERN: &[pattern::Atom] = pattern!("48 89 05 $ { ' } 48 83 c4 38 e9 $ { }");

/// Tail of the `CSWindow` setup that stores the process `hInstance` into a
/// global. It is written after the CRT init and the duplicate-instance checks,
/// which is what makes a non-zero value a usable "engine is up" signal.
const GLOBAL_HINSTANCE_PATTERN: &[pattern::Atom] =
    pattern!("48 8b ce 48 8b f8 e8 $ { 48 89 0d $ { ' } c3 }");

static GAME_DATA_MAN_STATIC: Mutex<Option<Option<usize>>> = Mutex::new(None);
static GLOBAL_HINSTANCE: Mutex<Option<Option<usize>>> = Mutex::new(None);

/// Set once the engine has come up. Initialization is monotonic, so callers on
/// the hot path can skip re-reading the global after the first success.
static SYSTEM_UP: AtomicBool = AtomicBool::new(false);

/// Scans the loaded executable for `pat` and returns the virtual address of the
/// global it captures.
///
/// A pattern that matches more than once no longer identifies a single site;
/// picking one arbitrarily would silently read an unrelated global, so this
/// reports failure instead. Callers then degrade to blank metrics.
fn scan_unique(name: &str, pat: &[pattern::Atom]) -> Option<usize> {
    let pe = Program::current();
    // save[0] receives the start of the match; the `'` capture lands in save[1].
    let mut save = [0u32; 4];
    let mut matches = pe.scanner().matches_code(pat);

    if !matches.next(&mut save) {
        warn!("{name}: byte pattern not found in the loaded executable");
        return None;
    }
    if matches.next(&mut [0u32; 4]) {
        warn!("{name}: byte pattern matched more than once; refusing to guess");
        return None;
    }

    let rva = save[1];
    let va = pe.rva_to_va(rva).ok()? as usize;
    debug!("{name} resolved to {va:#x} (rva {rva:#x})");
    Some(va)
}

/// Memoizes a scan, including its failures: a pattern that does not match will
/// not match on the next poll either, and re-scanning an 80 MB image every
/// 250 ms would be wasteful.
fn cached(cell: &Mutex<Option<Option<usize>>>, name: &str, pat: &[pattern::Atom]) -> Option<usize> {
    let mut guard = cell.lock().ok()?;
    *guard.get_or_insert_with(|| scan_unique(name, pat))
}

/// Address of the global holding the `GameDataMan` pointer.
pub fn game_data_man_static() -> Option<usize> {
    cached(
        &GAME_DATA_MAN_STATIC,
        "GameDataMan static",
        GAME_DATA_MAN_PATTERN,
    )
}

/// Whether both addresses could be resolved in the loaded executable. False
/// means this build is too different for the overlay to read anything.
pub fn addresses_available() -> bool {
    game_data_man_static().is_some()
        && cached(
            &GLOBAL_HINSTANCE,
            "global hInstance",
            GLOBAL_HINSTANCE_PATTERN,
        )
        .is_some()
}

/// Whether the game engine has finished initializing.
///
/// Mirrors `eldenring::util::system::wait_for_system_init`, but without the
/// version table: the DLRF reflection data the singleton lookups rely on is
/// populated by the time this global is non-zero.
pub fn system_initialized() -> bool {
    if SYSTEM_UP.load(Ordering::Relaxed) {
        return true;
    }
    let Some(addr) = cached(
        &GLOBAL_HINSTANCE,
        "global hInstance",
        GLOBAL_HINSTANCE_PATTERN,
    ) else {
        return false;
    };
    // SAFETY: `addr` points at a `HINSTANCE`-sized global inside the loaded
    // image, resolved from the instruction that writes it.
    let hinstance = unsafe { std::ptr::read_unaligned(addr as *const usize) };
    if hinstance == 0 {
        return false;
    }
    SYSTEM_UP.store(true, Ordering::Relaxed);
    true
}
