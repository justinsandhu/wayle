use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// General configuration settings for the Wayle application.
/// 
/// Contains global settings that affect the overall behavior of the application,
/// such as logging configuration.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GeneralConfig {
    /// Logging level for the application (e.g., "debug", "info", "warn", "error").
    #[serde(default)]
    pub log_level: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
        }
    }
}
