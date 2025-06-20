mod log_level;

pub use log_level::LogLevel;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// General configuration settings for the Wayle application.
///
/// Contains global settings that affect the overall behavior of the application,
/// such as logging configuration.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct GeneralConfig {
    /// Logging level for the application.
    #[serde(default)]
    pub log_level: LogLevel,
}
