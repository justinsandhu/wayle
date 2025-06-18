use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Configuration for the battery status module.
/// 
/// Controls the display and behavior of battery information in the status bar,
/// including percentage display and low battery warnings.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BatteryConfig {
    /// Whether the battery module is displayed in the status bar.
    #[serde(default)]
    pub enabled: bool,

    /// Whether to show the battery percentage alongside the icon.
    #[serde(default)]
    pub show_percentage: bool,

    /// Battery percentage threshold for triggering a low battery warning.
    #[serde(default)]
    pub battery_warning: u8,
}

impl Default for BatteryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            show_percentage: true,
            battery_warning: 20,
        }
    }
}
