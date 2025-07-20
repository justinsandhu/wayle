use std::sync::Arc;

use crate::{
    cli::{
        CliError, Command, CommandResult,
        types::{ArgType, CommandArg, CommandMetadata},
    },
    config_runtime::{ConfigError, ConfigRuntime},
};
use async_trait::async_trait;
use serde_json;
use toml;

pub struct SetCommand {
    config_store: Arc<ConfigRuntime>,
}

impl SetCommand {
    /// Creates a new SetCommand with the provided config store.
    ///
    /// # Arguments
    ///
    /// * `config_store` - Shared reference to the configuration store
    pub fn new(config_store: Arc<ConfigRuntime>) -> Self {
        Self { config_store }
    }

    fn parse_config_value(&self, value_str: &str) -> toml::Value {
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(value_str) {
            if let Ok(toml_value) = toml::Value::try_from(json_value) {
                return toml_value;
            }
        }

        if let Ok(b) = value_str.parse::<bool>() {
            return toml::Value::Boolean(b);
        }

        if let Ok(i) = value_str.parse::<i64>() {
            return toml::Value::Integer(i);
        }

        if let Ok(f) = value_str.parse::<f64>() {
            return toml::Value::Float(f);
        }

        toml::Value::String(value_str.to_string())
    }
}

#[async_trait]
impl Command for SetCommand {
    async fn execute(&self, args: &[String]) -> CommandResult {
        let path = args.first().ok_or(CliError::MissingPath)?;

        let value_str = args.get(1).ok_or(CliError::MissingValue)?;

        let value = self.parse_config_value(value_str);

        self.config_store
            .set_by_path(path, value)
            .map_err(|e| match e {
                ConfigError::InvalidPath(_) => CliError::ConfigPathNotFound { path: path.clone() },
                ConfigError::TypeMismatch {
                    path,
                    expected_type,
                    actual_value,
                } => CliError::InvalidConfigValue {
                    path: path.clone(),
                    reason: format!("expected {expected_type}, got {actual_value:?}"),
                },
                _ => CliError::ConfigOperationFailed {
                    operation: "set".to_string(),
                    path: path.clone(),
                    details: e.to_string(),
                },
            })?;

        Ok(format!("Set new value '{value_str}' at path '{path}'"))
    }

    fn metadata(&self) -> CommandMetadata {
        CommandMetadata {
            name: "set".to_string(),
            description: "Set configuration value".to_string(),
            category: "config".to_string(),
            args: vec![
                CommandArg {
                    name: "path".to_string(),
                    description: "Configuration path".to_string(),
                    required: true,
                    value_type: ArgType::Path,
                },
                CommandArg {
                    name: "value".to_string(),
                    description: "New value (auto-detected type)".to_string(),
                    required: true,
                    value_type: ArgType::String,
                },
            ],
            examples: vec![
                "wayle config set modules.battery.enabled true".to_string(),
                "wayle config set modules.battery.warning_threshold 20".to_string(),
            ],
        }
    }
}
