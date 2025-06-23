use std::{collections::HashMap, sync::Arc};

use crate::config_store::ConfigStore;

use super::{
    CliError, Command,
    commands::config::{self},
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
    categories: HashMap<String, HashMap<String, Box<dyn Command>>>,
    config_store: Arc<ConfigStore>,
}

impl CommandRegistry {
    /// Creates a new empty command registry.
    ///
    /// The registry starts with no commands registered. Commands must be added
    /// using the `register_command` method, typically during application initialization.
    pub fn new(config_store: Arc<ConfigStore>) -> Self {
        let categories = HashMap::new();
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
    /// ```
    /// let mut registry = CommandRegistry::new();
    /// registry.register_command("config", Box::new(GetCommand::new(config_store)));
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
    pub fn execute(
        &self,
        category: &str,
        command_name: &str,
        args: &[String],
    ) -> Result<String, CliError> {
        let found_category = self.categories.get(category).ok_or_else(|| {
            CliError::CommandNotFound(format!("Failed to find category '{category}'"))
        })?;

        let found_command = found_category.get(command_name).ok_or_else(|| {
            CliError::CommandNotFound(format!("Failed to find command '{command_name}'"))
        })?;

        Self::validate_args(&found_command.metadata(), args)?;

        found_command.execute(args)
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

    fn validate_args(metadata: &CommandMetadata, args: &[String]) -> Result<(), CliError> {
        let required_count = metadata.args.iter().filter(|arg| arg.required).count();
        let total_count = metadata.args.len();

        if args.len() < required_count {
            return Err(CliError::InvalidArguments(format!(
                "Expected at least {} arguments, got {}",
                required_count,
                args.len(),
            )));
        }

        if args.len() > total_count {
            return Err(CliError::InvalidArguments(format!(
                "Expected at most {} arguments, got {}",
                total_count,
                args.len(),
            )));
        }

        Ok(())
    }

    /// Registers all available CLI commands in their respective categories.
    ///
    /// This function serves as the central registration point for all CLI commands,
    /// delegating to individual modules to register their commands.
    pub fn register_all_commands(&mut self) {
        config::register_commands(self, self.config_store.clone());
    }
}
