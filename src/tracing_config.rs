use std::env;

use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize tracing for the application
///
/// Sets up structured logging with info level by default.
/// Uses RUST_LOG environment variable if set, otherwise defaults to "info".
/// Supports both pretty console output and JSON output based on WAYLE_LOG_FORMAT.
///
/// # Errors
/// Returns error if tracing subscriber initialization fails
pub fn init() -> Result<(), Box<dyn std::error::Error>> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let format = env::var("WAYLE_LOG_FORMAT").unwrap_or_else(|_| "pretty".to_string());

    let registry = tracing_subscriber::registry().with(env_filter);

    match format.as_str() {
        "json" => {
            registry
                .with(fmt::layer().json().with_target(true).with_level(true))
                .try_init()?;
        }
        _ => {
            registry
                .with(
                    fmt::layer()
                        .pretty()
                        .with_target(true)
                        .with_level(true)
                        .with_thread_ids(true)
                        .with_thread_names(true),
                )
                .try_init()?;
        }
    }

    Ok(())
}

/// Initialize tracing with file output
///
/// Similar to init() but also writes logs to a file in addition to stdout.
/// File is created in the wayle logs directory.
///
/// # Errors
/// Returns error if file creation or tracing subscriber initialization fails
pub fn init_with_file() -> Result<(), Box<dyn std::error::Error>> {
    const DAYS_TO_KEEP: usize = 7;
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let log_dir = crate::config::ConfigPaths::log_dir()?;

    let file_appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .max_log_files(DAYS_TO_KEEP)
        .filename_prefix("wayle")
        .filename_suffix("log")
        .build(&log_dir)?;
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let format = env::var("WAYLE_LOG_FORMAT").unwrap_or_else(|_| "pretty".to_string());

    let registry = tracing_subscriber::registry().with(env_filter);

    match format.as_str() {
        "json" => {
            registry
                .with(
                    fmt::layer()
                        .json()
                        .with_target(true)
                        .with_level(true)
                        .with_writer(std::io::stdout),
                )
                .with(
                    fmt::layer()
                        .json()
                        .with_target(true)
                        .with_level(true)
                        .with_writer(non_blocking)
                        .with_ansi(false),
                )
                .try_init()?;
        }
        _ => {
            registry
                .with(
                    fmt::layer()
                        .pretty()
                        .with_target(true)
                        .with_level(true)
                        .with_thread_ids(true)
                        .with_thread_names(true)
                        .with_writer(std::io::stdout),
                )
                .with(
                    fmt::layer()
                        .compact()
                        .with_target(true)
                        .with_level(true)
                        .with_writer(non_blocking)
                        .with_ansi(false),
                )
                .try_init()?;
        }
    }

    std::mem::forget(_guard);

    Ok(())
}
