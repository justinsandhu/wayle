use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClockConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub format: String,
}

impl Default for ClockConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            format: "%H:%M".to_string(),
        }
    }
}
