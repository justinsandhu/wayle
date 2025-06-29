use std::sync::Arc;

use futures::StreamExt;

use crate::{
    cli::{
        CliError, Command, CommandResult,
        formatting::format_toml_value,
        types::{ArgType, CommandArg, CommandMetadata},
    },
    config_store::ConfigStore,
};

pub struct WatchCommand {
    config_store: Arc<ConfigStore>,
}

impl WatchCommand {
    pub fn new(config_store: Arc<ConfigStore>) -> Self {
        Self { config_store }
    }
}

impl Command for WatchCommand {
    fn execute(&self, args: &[String]) -> CommandResult {
        let path = args.first().ok_or_else(|| {
            CliError::InvalidArguments("Expected <path> argument for 'watch' command".to_string())
        })?;

        println!("Watching changes on path '{}'...", path);
        println!("Press Ctrl+C to stop");

        let config_store = self.config_store.clone();
        let path = path.to_string();

        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| CliError::ServiceError(format!("Failed to create runtime: {}", e)))?;

        runtime.block_on(async move {
            let mut stream = config_store.subscribe_to_path(&path);

            while let Some(change) = stream.next().await {
                println!(
                    "[{}s] {} -> {} (source: {:?})",
                    change.timestamp.elapsed().as_secs(),
                    change.path,
                    format_toml_value(&change.new_value),
                    change.source
                );
            }
        });

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
