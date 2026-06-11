mod inject;
mod process;

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::Parser;
use tracing::{error, info, warn};

use crate::inject::inject_loadlibrary;
use crate::process::{find_process_by_name, is_process_x64, list_loaded_modules, ProcessInfo};

const TARGET_PROCESS: &str = "eldenring.exe";
const EAC_MODULE_HINTS: &[&str] = &[
    "EasyAntiCheat_EOS.dll",
    "EasyAntiCheat.dll",
    "start_protected_game.exe",
];

#[derive(Parser, Debug)]
#[command(name = "er_overlay_injector")]
#[command(
    about = "Inject er_overlay.dll into a running eldenring.exe (offline, read-only overlay)"
)]
struct Args {
    /// Path to er_overlay.dll (default: next to this executable)
    #[arg(long)]
    dll: Option<PathBuf>,

    /// Target process PID (default: find eldenring.exe)
    #[arg(long)]
    pid: Option<u32>,

    /// Validate only; do not inject
    #[arg(long)]
    dry_run: bool,
}

fn main() -> Result<()> {
    let _guard = er_overlay_common::init_file_logging("injector", "er_injector.log")
        .context("Failed to initialize logging")?;

    let args = Args::parse();
    info!("er_overlay_injector starting");

    let proc = if let Some(pid) = args.pid {
        info!("Using PID {pid}");
        ProcessInfo::open(pid)?
    } else {
        info!("Searching for {TARGET_PROCESS}...");
        match find_process_by_name(TARGET_PROCESS)? {
            Some(p) => {
                info!("Found {TARGET_PROCESS} pid={} name={}", p.pid, p.image_name);
                p
            }
            None => {
                error!("Process {TARGET_PROCESS} not found");
                bail!("{TARGET_PROCESS} is not running");
            }
        }
    };

    if !is_process_x64(proc.pid)? {
        error!("Target process is not x64");
        bail!("Target process architecture must be x64");
    }
    info!("Architecture validation: x64 OK");

    let dll_path = resolve_dll_path(args.dll)?;
    info!("DLL path: {}", dll_path.display());

    if !dll_path.exists() {
        error!("DLL not found at {}", dll_path.display());
        bail!("DLL file does not exist");
    }
    if dll_path.extension().and_then(|e| e.to_str()) != Some("dll") {
        bail!("Expected a .dll file");
    }
    info!("DLL path validation OK");

    warn_if_eac_modules(proc.pid)?;

    if args.dry_run {
        info!("Dry-run complete — injection skipped");
        return Ok(());
    }

    info!("Starting injection (LoadLibraryW via CreateRemoteThread)...");
    match inject_loadlibrary(proc.pid, &dll_path) {
        Ok(()) => {
            info!("Injection succeeded");
            Ok(())
        }
        Err(e) => {
            error!("Injection failed: {e:?}");
            Err(e)
        }
    }
}

fn resolve_dll_path(explicit: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(p) = explicit {
        return Ok(p);
    }
    let exe_dir = std::env::current_exe()
        .context("current_exe")?
        .parent()
        .map(|p| p.to_path_buf())
        .context("exe has no parent")?;
    Ok(exe_dir.join("er_overlay.dll"))
}

fn warn_if_eac_modules(pid: u32) -> Result<()> {
    let modules = list_loaded_modules(pid).context("Failed to enumerate modules")?;
    let hits: Vec<_> = modules
        .iter()
        .filter(|m| {
            EAC_MODULE_HINTS
                .iter()
                .any(|hint| m.eq_ignore_ascii_case(hint))
        })
        .cloned()
        .collect();
    if hits.is_empty() {
        info!("EAC module check: no obvious EAC modules detected");
        return Ok(());
    }
    warn!(
        "EasyAntiCheat or protected launcher modules detected: {}. \
         Injection may fail. Use offline mode (launch eldenring.exe directly with steam_appid.txt). \
         Continuing anyway (warn-only).",
        hits.join(", ")
    );
    Ok(())
}
