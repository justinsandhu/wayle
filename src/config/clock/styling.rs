use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Styling configuration for the clock module.
///
/// Controls the visual appearance of the clock in both the status bar
/// and dropdown views, including colors, fonts, and icons.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct ClockStyling {
    /// Styling options for the clock button in the bar.
    #[serde(default)]
    pub button: ClockButtonStyling,

    /// Styling options for the clock dropdown panel.
    #[serde(default)]
    pub dropdown: ClockDropdownStyling,
}

/// Styling configuration for the clock button in the status bar.
///
/// Defines visual properties specific to how the clock appears when
/// displayed as a button in the main status bar.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClockButtonStyling {
    /// Color of the clock icon in the bar button.
    #[serde(default)]
    pub icon: String,
}

impl Default for ClockButtonStyling {
    fn default() -> Self {
        Self {
            icon: "red".to_string(),
        }
    }
}

/// Styling configuration for the clock dropdown view.
///
/// Controls the visual appearance of the clock when displayed in the
/// dropdown panel, including calendar and time display styling.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClockDropdownStyling {
    /// Color of the clock display in the dropdown panel.
    #[serde(default)]
    pub clock: String,
}

impl Default for ClockDropdownStyling {
    fn default() -> Self {
        Self {
            clock: "red".to_string(),
        }
    }
}
