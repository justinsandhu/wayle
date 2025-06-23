//! Configuration schema definitions and validation.
//!
//! Defines the complete configuration structure for Wayle, including
//! general settings and module-specific configurations. All configurations
//! are serializable to/from TOML format.

mod battery;
mod clock;
mod general;
mod loading;
mod modules;
mod paths;
mod styling;

pub use clock::ClockConfig;
pub use paths::ConfigPaths;
pub use styling::*;

use general::GeneralConfig;
use modules::ModulesConfig;
use serde::{Deserialize, Serialize};

/// Main configuration structure for Wayle.
///
/// Represents the complete configuration schema that can be loaded
/// from TOML files. All fields have sensible defaults.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// General application settings.
    #[serde(default)]
    pub general: GeneralConfig,

    /// Module-specific configurations.
    #[serde(default)]
    pub modules: ModulesConfig,
}
