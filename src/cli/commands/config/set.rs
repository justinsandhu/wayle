use std::sync::Arc;

use crate::{
    cli::{
        CliError, Command, CommandResult,
        types::{ArgType, CommandArg, CommandMetadata},
    },
    config_store::ConfigStore,
};

pub struct SetCommand {
    config_store: Arc<ConfigStore>,
}

impl SetCommand {
    /// Creates a new SetCommand with the provided config store.
    ///
    /// # Arguments
    ///
    /// * `config_store` - Shared reference to the configuration store
    pub fn new(config_store: Arc<ConfigStore>) -> Self {
        Self { config_store }
    }

    fn parse_config_value(&self, value_str: &str) -> Result<toml::Value, CliError> {
        if let Ok(b) = value_str.parse::<bool>() {
            return Ok(toml::Value::Boolean(b));
        }

        if let Ok(i) = value_str.parse::<i64>() {
            return Ok(toml::Value::Integer(i));
        }

        if let Ok(f) = value_str.parse::<f64>() {
            return Ok(toml::Value::Float(f));
        }

        Ok(toml::Value::String(value_str.to_string()))
    }
}

impl Command for SetCommand {
    fn execute(&self, args: &[String]) -> CommandResult {
        let path = args.first().ok_or_else(|| {
            CliError::InvalidArguments("Expected <path> argument for 'set' command".to_string())
        })?;

        let value_str = args.get(1).ok_or_else(|| {
            CliError::InvalidArguments("Expected <value> argument for 'set' command".to_string())
        })?;
        let value = self.parse_config_value(value_str)?;

        match self.config_store.set_by_path(path, value) {
            Ok(()) => Ok(format!("Set new value '{}' at path '{}'", value_str, path)),
            Err(e) => Err(CliError::ConfigError(e.to_string())),
        }
    }

    fn metadata(&self) -> CommandMetadata {
        CommandMetadata {
            name: "set".to_string(),
            description: "Set conifugration value".to_string(),
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
