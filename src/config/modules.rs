use serde::{Deserialize, Serialize};

use super::{battery::BatteryConfig, clock::ClockConfig};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModulesConfig {
    pub battery: Option<BatteryConfig>,
    pub clock: Option<ClockConfig>,
}
