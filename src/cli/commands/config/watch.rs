use std::sync::Arc;

use crate::{
    cli::{
        CliError, Command, CommandResult,
        formatting::format_toml_value,
        types::{ArgType, CommandArg, CommandMetadata},
    },
    config_runtime::ConfigRuntime,
};
use async_trait::async_trait;

pub struct WatchCommand {
    config_store: Arc<ConfigRuntime>,
}

impl WatchCommand {
    pub fn new(config_store: Arc<ConfigRuntime>) -> Self {
        Self { config_store }
    }
}

#[async_trait]
impl Command for WatchCommand {
    async fn execute(&self, args: &[String]) -> CommandResult {
        let path = args.first().ok_or(CliError::MissingPath)?;

        println!("Watching changes on path '{}'...", path);
        println!("Press Ctrl+C to stop");

        let _file_watch_handle = self.config_store.start_file_watching().map_err(|e| {
            CliError::ConfigOperationFailed {
                operation: "start file watching".to_string(),
                path: path.clone(),
                details: e.to_string(),
            }
        })?;

        let mut subscription = self
            .config_store
            .subscribe_to_path(path)
            .await
            .map_err(|e| CliError::ConfigOperationFailed {
                operation: "subscribe to path".to_string(),
                path: path.clone(),
                details: e.to_string(),
            })?;

        while let Some(change) = subscription.receiver_mut().recv().await {
            println!(
                "[{}s] {} -> {}",
                change.timestamp.elapsed().as_secs(),
                change.path,
                format_toml_value(&change.new_value)
            );
        }

        Ok("Watch ended".to_string())
    }

    fn metadata(&self) -> CommandMetadata {
        CommandMetadata {
            name: "watch".to_string(),
            description: "Watch configuration changes for a configuration path".to_string(),
            category: "config".to_string(),
            args: vec![CommandArg {
                name: "path".to_string(),
                description: "The path of the configuration to watch".to_string(),
                required: true,
                value_type: ArgType::String,
            }],
            examples: vec!["wayle config watch modules.battery.enabled".to_string()],
        }
    }
}
