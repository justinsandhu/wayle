use std::sync::Arc;

use crate::{
    cli::{
        CliError, Command, CommandResult,
        types::{ArgType, CommandArg, CommandMetadata},
    },
    config_store::ConfigStore,
};

/// Command for retrieving configuration values from the config store.
///
/// Provides read access to the configuration system, allowing users to
/// query specific configuration paths and receive formatted output.
///
/// # Example Usage
///
/// ```bash
/// wayle config get modules.battery.enabled
/// wayle config get general.theme
/// wayle config get modules.clock.format
/// ```
pub struct GetCommand {
    /// Shared reference to the configuration store.
    config_store: Arc<ConfigStore>,
}

impl GetCommand {
    /// Creates a new GetCommand with the provided config store.
    ///
    /// # Arguments
    ///
    /// * `config_store` - Shared reference to the configuration store
    pub fn new(config_store: Arc<ConfigStore>) -> Self {
        Self { config_store }
    }
}

impl Command for GetCommand {
    /// Retrieves and formats a configuration value from the specified path.
    ///
    /// # Arguments
    ///
    /// * `args` - Command arguments, expecting exactly one path argument
    ///
    /// # Errors
    ///
    /// * `CliError::InvalidArguments` - If no path argument is provided
    /// * `CliError::ConfigError` - If the config store operation fails
    fn execute(&self, args: &[String]) -> CommandResult {
        let path = args.first().ok_or_else(|| {
            CliError::InvalidArguments("Expected <path> argument for 'get' command".to_string())
        })?;

        let value = self
            .config_store
            .get_by_path(path)
            .map_err(|e| CliError::ConfigError(e.to_string()))?;

        Ok(format!("{}: {}", path, self.format_value(&value)))
    }

    fn metadata(&self) -> CommandMetadata {
        CommandMetadata {
            name: "get".to_string(),
            description: "Get configuration value".to_string(),
            category: "config".to_string(),
            args: vec![CommandArg {
                name: "path".to_string(),
                description: "Configuration path (e.g., modules.battery.enabled)".to_string(),
                required: true,
                value_type: ArgType::Path,
            }],
            examples: vec![
                "wayle config get modules.battery.enabled".to_string(),
                "wayle config get general.log_level".to_string(),
                "wayle config get modules.clock.format".to_string(),
            ],
        }
    }
}

impl GetCommand {
    fn format_value(&self, value: &toml::Value) -> String {
        match value {
            toml::Value::String(s) => format!("\"{}\"", s),
            toml::Value::Integer(i) => i.to_string(),
            toml::Value::Float(f) => f.to_string(),
            toml::Value::Boolean(b) => b.to_string(),
            toml::Value::Array(arr) => format!("[{}]", arr.len()),
            toml::Value::Table(table) => format!("{{{}}}", table.len()),
            _ => "complex_value".to_string(),
        }
    }
}
