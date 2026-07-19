use std::path::PathBuf;

use anyhow::{Context, Result};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::config::default_base_dir;

/// Keeps the background log-writing thread alive. Must stay alive for as long as
/// logs should be flushed; drop it and buffered records are lost.
pub type LogGuard = WorkerGuard;

const DEFAULT_FILTER: &str = "info,er_overlay=debug,er_game_state=debug";

/// Initialize file logging into `logs/<log_file_name>` next to the overlay.
///
/// The verbosity filter is resolved with the following precedence:
/// `RUST_LOG` (environment) > `level` (caller/config) > the built-in default.
/// `level` accepts either a bare level (`debug`) or a full `tracing` filter
/// (`info,er_overlay=debug`).
pub fn init_file_logging(
    component: &str,
    log_file_name: &str,
    level: Option<&str>,
) -> Result<WorkerGuard> {
    let log_dir = log_directory();
    std::fs::create_dir_all(&log_dir)
        .with_context(|| format!("Failed to create log directory {}", log_dir.display()))?;

    let file_appender = tracing_appender::rolling::never(&log_dir, log_file_name);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        let raw = level.map(str::trim).filter(|s| !s.is_empty());
        EnvFilter::new(raw.unwrap_or(DEFAULT_FILTER))
    });

    tracing_subscriber::registry()
        .with(filter)
        .with(
            fmt::layer()
                .with_target(true)
                .with_ansi(false)
                .with_writer(non_blocking),
        )
        .init();

    tracing::info!("Logging initialized for {component} -> {log_dir:?}/{log_file_name}");
    Ok(guard)
}

/// Directory where log files are written (`<base_dir>/logs`).
pub fn log_directory() -> PathBuf {
    default_base_dir().join("logs")
}
