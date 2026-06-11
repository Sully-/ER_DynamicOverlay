use std::path::PathBuf;

use anyhow::{Context, Result};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::config::default_base_dir;

pub fn init_file_logging(component: &str, log_file_name: &str) -> Result<WorkerGuard> {
    let log_dir = log_directory();
    std::fs::create_dir_all(&log_dir)
        .with_context(|| format!("Failed to create log directory {}", log_dir.display()))?;

    let file_appender = tracing_appender::rolling::never(&log_dir, log_file_name);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,er_overlay=debug,er_game_state=debug"));

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

fn log_directory() -> PathBuf {
    default_base_dir().join("logs")
}
