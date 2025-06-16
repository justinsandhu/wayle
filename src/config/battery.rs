use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub show_percentage: bool,

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
