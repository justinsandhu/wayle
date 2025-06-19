use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Configuration for the clock's dropdown menu.
///
/// Controls the content and behavior of the dropdown that appears
/// when clicking on the clock module.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClockDropdownConfig {
    /// Whether to display a calendar widget in the dropdown menu.
    #[serde(default)]
    pub show_calendar: bool,
}

impl Default for ClockDropdownConfig {
    fn default() -> Self {
        Self {
            show_calendar: true,
        }
    }
}
