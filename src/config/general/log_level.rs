use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Logging level for the application.
///
/// Controls the verbosity of log output, from critical errors only
/// to detailed trace information.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Only show critical errors that prevent the application from functioning.
    Error,

    /// Show warnings and errors (potential issues that don't break functionality).
    Warn,

    /// Show informational messages, warnings, and errors (default level).
    #[default]
    Info,

    /// Show debug information useful for development and troubleshooting.
    Debug,

    /// Show detailed trace information including function entry/exit (very verbose).
    Trace,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Error => write!(f, "error"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Trace => write!(f, "trace"),
        }
    }
}
