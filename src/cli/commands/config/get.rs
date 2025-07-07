use std::sync::Arc;

use crate::{
    cli::{
        CliError, Command, CommandResult,
        formatting::format_toml_value,
        types::{ArgType, CommandArg, CommandMetadata},
    },
    config_store::{ConfigError, ConfigStore},
};
use async_trait::async_trait;

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

#[async_trait]
impl Command for GetCommand {
    /// Retrieves and formats a configuration value from the specified path.
    ///
    /// # Arguments
    ///
    /// * `args` - Command arguments, expecting exactly one path argument
    ///
    /// # Errors
    ///
    /// * `CliError::MissingPath` - If no path argument is provided
    /// * `CliError::ConfigPathNotFound` - If the configuration path doesn't exist
    /// * `CliError::ConfigOperationFailed` - If the config store operation fails
    async fn execute(&self, args: &[String]) -> CommandResult {
        let path = args.first().ok_or(CliError::MissingPath)?;

        let value = self.config_store.get_by_path(path).map_err(|e| match e {
            ConfigError::InvalidPath(_) => CliError::ConfigPathNotFound { path: path.clone() },
            _ => CliError::ConfigOperationFailed {
                operation: "get".to_string(),
                path: path.clone(),
                details: e.to_string(),
            },
        })?;

        Ok(format_toml_value(&value).to_string())
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
