use std::collections::HashMap;

use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};

use crate::docs::{BehaviorConfigs, ModuleInfo, ModuleInfoProvider, StylingConfigs};

use super::{ButtonStyling, DropdownStyling};

/// General configuration settings for the clock module.
///
/// Contains core settings that apply to the clock module regardless of
/// where it's displayed (status bar, dropdown, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClockConfig {
    /// Whether the clock module is displayed in the status bar.
    #[serde(default)]
    pub enabled: bool,

    /// Time format string using strftime syntax (e.g., "%H:%M" for 24-hour time).
    #[serde(default)]
    pub format: String,
}

/// Configuration for the clock's appearance in the status bar.
///
/// Controls visual elements specific to how the clock module appears
/// when displayed in the main status bar.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClockBarConfig {
    /// Whether to display a clock icon alongside the time text.
    #[serde(default)]
    pub show_icon: bool,
}

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

impl Default for ClockConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            format: "%H:%M".to_string(),
        }
    }
}

impl ModuleInfoProvider for ClockConfig {
    fn module_info() -> ModuleInfo {
        let mut styling_configs: StylingConfigs = HashMap::new();
        let mut behavior_configs: BehaviorConfigs = HashMap::new();

        styling_configs.insert("button".to_string(), || schema_for!(ButtonStyling));
        styling_configs.insert("dropdown".to_string(), || schema_for!(DropdownStyling));

        behavior_configs.insert("general".to_string(), || schema_for!(ClockConfig));
        behavior_configs.insert("button".to_string(), || schema_for!(ClockBarConfig));
        behavior_configs.insert("dropdown".to_string(), || schema_for!(ClockDropdownConfig));

        ModuleInfo {
            name: "clock".to_string(),
            icon: "󰥔".to_string(),
            description: "Controls the clock display and calendar settings".to_string(),
            behavior_configs,
            styling_configs,
        }
    }
}
