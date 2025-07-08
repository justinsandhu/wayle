use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Media service configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct MediaConfig {
    /// List of player bus name patterns to ignore during discovery
    pub ignored_players: Vec<String>,
}

impl Default for MediaConfig {
    fn default() -> Self {
        Self {
            ignored_players: Vec::new(),
        }
    }
}
