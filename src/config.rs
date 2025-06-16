mod battery;
mod clock;
mod general;
mod loading;
mod modules;

use general::GeneralConfig;
use modules::ModulesConfig;
use serde::{Deserialize, Serialize};

/// Main configuration structure for Wayle.
///
/// Contains all configuration settings including general settings
/// and module-specific configurations.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// General application settings.
    #[serde(default)]
    pub general: GeneralConfig,

    /// Module-specific configurations.
    #[serde(default)]
    pub modules: ModulesConfig,
}
