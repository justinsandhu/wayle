mod battery;
mod clock;
mod general;
mod loading;
mod modules;

use general::GeneralConfig;
use modules::ModulesConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,

    #[serde(default)]
    pub modules: ModulesConfig,
}
