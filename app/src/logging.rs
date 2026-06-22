use std::sync::OnceLock;

use hi_core::{normalize_log_level, Config, logs_directory};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

fn build_env_filter() -> EnvFilter {
    if std::env::var("RUST_LOG")
        .ok()
        .is_some_and(|v| !v.trim().is_empty())
    {
        return EnvFilter::from_default_env();
    }

    let level = Config::load()
        .map(|c| c.logging.normalized_level())
        .unwrap_or_else(|_| "info".into());
    EnvFilter::new(level)
}

/// Init tracing to `~/.hi/logs/hi.log` with daily rotation.
///
/// Level: `RUST_LOG` env overrides `[logging].level` in hi.toml (default `info`).
pub fn init() {
    let _ = LOG_GUARD.get_or_init(|| {
        let log_dir = logs_directory();
        let _ = std::fs::create_dir_all(&log_dir);
        let file_appender = RollingFileAppender::new(Rotation::DAILY, &log_dir, "hi.log");
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

        let filter = build_env_filter();
        let level = normalize_log_level(
            &Config::load()
                .map(|c| c.logging.level)
                .unwrap_or_else(|_| "info".into()),
        );

        tracing_subscriber::registry()
            .with(filter)
            .with(
                tracing_subscriber::fmt::layer()
                    .with_writer(non_blocking)
                    .with_ansi(false),
            )
            .init();

        tracing::info!(
            level = %level,
            log_dir = %log_dir.display(),
            "hi logging initialized"
        );

        guard
    });
}
