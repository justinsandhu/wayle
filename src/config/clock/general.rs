use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Core clock functionality settings.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClockGeneralConfig {
    /// Time format string using strftime syntax.
    #[serde(default)]
    pub format: String,
}

impl Default for ClockGeneralConfig {
    fn default() -> Self {
        Self {
            format: "%H:%M".to_string(),
        }
    }
}
