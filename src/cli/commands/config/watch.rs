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

        // let mut stream = self.config_store.subscribe_to_path(path);

        print!("Watching changes on path '{}'...", path);
        print!("Press Ctrl+C to stop");

        // tokio::spawn(async move {
        //     while let Some(change) = stream.next().await {
        //         println!(
        //             "[{}s] {} -> {} (source: {:?})",
        //             change.timestamp.elapsed().as_secs(),
        //             change.path,
        //             change.new_value.as_ref().map_or("(removed)".to_string(), format_toml_value),
        //             change.source
        //         );
        //     }
        // });

        Ok("Watch started (TODO: Implement async watching".to_string())
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
