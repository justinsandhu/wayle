use std::{collections::HashMap, sync::Arc};

/// Nested command storage: category name -> (command name -> command implementation)
pub type CommandCategories = HashMap<String, HashMap<String, Box<dyn Command>>>;

use crate::config_store::ConfigStore;

use super::{
    CliError, Command,
    commands::{audio, config, media},
    types::CommandMetadata,
};

/// Registry for CLI commands organized by category.
///
/// The CommandRegistry provides a hierarchical structure for managing CLI commands,
/// allowing commands to be grouped by logical categories (e.g., "config", "system", "panel").
/// This design enables scalable command organization and avoids the anti-pattern of
/// giant match statements that become unmaintainable as the CLI grows.
///
/// # Example Structure
///
/// ```text
/// registry
/// ├── config
/// │   ├── get
/// │   ├── set
/// │   └── watch
/// ├── system
/// │   ├── status
/// │   └── restart
/// └── panel
///     └── modules
/// ```
pub struct CommandRegistry {
    /// Nested HashMap structure: category name -> (command name -> command implementation)
    categories: CommandCategories,
    config_store: Arc<ConfigStore>,
}

impl CommandRegistry {
    /// Creates a new empty command registry.
    ///
    /// The registry starts with no commands registered. Commands must be added
    /// using the `register_command` method, typically during application initialization.
    pub fn new(config_store: Arc<ConfigStore>) -> Self {
        let categories = CommandCategories::new();
        Self {
            categories,
            config_store,
        }
    }

    /// Registers a command in the specified category.
    ///
    /// Commands are automatically organized by category, with the command's name
    /// (from its metadata) used as the key within that category. If a command
    /// with the same name already exists in the category, it will be replaced.
    ///
    /// # Arguments
    ///
    /// * `category` - The category to register the command under (e.g., "config", "system")
    /// * `command` - The command implementation to register
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use wayle::cli::CommandRegistry;
    /// use wayle::config_store::ConfigStore;
    /// use std::sync::Arc;
    ///
    /// let config_store = Arc::new(ConfigStore::with_defaults());
    /// let mut registry = CommandRegistry::new(config_store);
    /// // registry.register_command("config", Box::new(SomeCommand::new()));
    /// ```
    pub fn register_command(&mut self, category: &str, command: Box<dyn Command>) {
        self.categories
            .entry(category.to_string())
            .or_default()
            .insert(command.metadata().name, command);
    }

    /// Executes a command by category and name with the provided arguments.
    ///
    /// This method looks up the command in the registry's hierarchical structure
    /// and delegates execution to the command implementation. The command is
    /// responsible for its own argument validation and execution logic.
    ///
    /// # Arguments
    ///
    /// * `category` - The category containing the command
    /// * `command_name` - The name of the command to execute
    /// * `args` - Arguments to pass to the command
    ///
    /// # Errors
    ///
    /// Returns `CliError::CommandNotFound` if:
    /// - The specified category doesn't exist
    /// - The specified command doesn't exist within the category
    ///
    /// Other errors may be returned by the command's execute method.
    pub async fn execute(
        &self,
        category: &str,
        command_name: &str,
        args: &[String],
    ) -> Result<String, CliError> {
        let found_category =
            self.categories
                .get(category)
                .ok_or_else(|| CliError::CommandNotFound {
                    command: format!("{category} (category)"),
                })?;

        let found_command =
            found_category
                .get(command_name)
                .ok_or_else(|| CliError::CommandNotFound {
                    command: format!("{category} {command_name}"),
                })?;

        Self::validate_args(&found_command.metadata(), args)?;

        found_command.execute(args).await
    }

    /// Lists all registered commands organized by category.
    ///
    /// Returns a vector of tuples where each tuple contains:
    /// - Category name
    /// - Vector of command names within that category
    ///
    /// Categories and commands are sorted alphabetically for consistent display.
    pub fn list_commands(&self) -> Vec<(String, Vec<String>)> {
        let mut categories: Vec<(String, Vec<String>)> = self
            .categories
            .iter()
            .map(|(category, commands)| {
                let mut command_list: Vec<String> = commands.keys().cloned().collect();
                command_list.sort();

                (category.clone(), command_list)
            })
            .collect();

        categories.sort();

        categories
    }

    /// Returns all registered category names.
    ///
    /// Categories are returned in arbitrary order. Use this method to discover
    /// available command categories for help generation or command discovery.
    ///
    /// # Returns
    /// Vector of category names currently registered in the registry
    pub fn get_categories(&self) -> Vec<String> {
        self.categories.keys().cloned().collect()
    }

    /// Returns all command names within a specific category.
    ///
    /// Use this method to list available commands for a category when generating
    /// help text or implementing command discovery features.
    ///
    /// # Arguments
    /// * `category` - The category to list commands for
    ///
    /// # Returns
    /// Some(commands) if the category exists, None if category not found
    pub fn get_commands_in_category(&self, category: &str) -> Option<Vec<String>> {
        self.categories
            .get(category)
            .map(|commands| commands.keys().cloned().collect())
    }

    /// Retrieves metadata for a specific command.
    ///
    /// Command metadata includes the command's description, arguments, and examples.
    /// Use this for generating detailed help text for individual commands.
    ///
    /// # Arguments
    /// * `category` - The category containing the command
    /// * `command` - The command name to get metadata for
    ///
    /// # Returns
    /// Some(metadata) if command exists, None if category or command not found
    pub fn get_command_metadata(&self, category: &str, command: &str) -> Option<CommandMetadata> {
        self.categories
            .get(category)?
            .get(command)
            .map(|cmd| cmd.metadata())
    }

    fn validate_args(metadata: &CommandMetadata, args: &[String]) -> Result<(), CliError> {
        let required_count = metadata.args.iter().filter(|arg| arg.required).count();
        let total_count = metadata.args.len();

        if args.len() < required_count {
            let missing_args: Vec<String> = metadata
                .args
                .iter()
                .skip(args.len())
                .filter(|arg| arg.required)
                .map(|arg| format!("<{}>", arg.name.to_uppercase()))
                .collect();

            let mut usage = format!("wayle {} {}", metadata.category, metadata.name);
            for arg in &metadata.args {
                if arg.required {
                    usage.push_str(&format!(" <{}>", arg.name.to_uppercase()));
                } else {
                    usage.push_str(&format!(" [{}]", arg.name.to_uppercase()));
                }
            }

            return Err(CliError::MissingArguments {
                missing: missing_args.join(", "),
                usage,
            });
        }

        if args.len() > total_count {
            return Err(CliError::TooManyArguments {
                expected: total_count,
                actual: args.len(),
            });
        }

        Ok(())
    }

    /// Registers all available CLI commands in their respective categories.
    ///
    /// This function serves as the central registration point for all CLI commands,
    /// delegating to individual modules to register their commands.
    pub fn register_all_commands(&mut self) {
        config::register_commands(self, self.config_store.clone());
        media::register_commands(self);
        audio::register_commands(self);
    }
}
