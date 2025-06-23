use std::sync::Arc;

use crate::config_store::ConfigStore;

use super::{CliError, CommandRegistry};

/// High-level service for managing and executing CLI commands.
///
/// Provides a unified interface for command registration, discovery, and execution.
/// Commands are organized by category and can be listed or executed by name.
pub struct CliService {
    registry: CommandRegistry,
}

impl CliService {
    /// Creates a new CLI service with all available commands registered.
    ///
    /// Initializes the command registry and automatically registers all built-in
    /// commands across all categories. The config store is shared across commands
    /// that need configuration access.
    ///
    /// # Arguments
    /// * `config_store` - Configuration store for commands that need config access
    pub fn new(config_store: ConfigStore) -> Self {
        let config_store = Arc::new(config_store);
        let mut registry = CommandRegistry::new(config_store);
        registry.register_all_commands();

        CliService { registry }
    }

    /// Executes a command by category and name with the provided arguments.
    ///
    /// Looks up the command in the specified category and executes it with the
    /// given arguments. Returns the command output as a string on success.
    ///
    /// # Arguments
    /// * `category` - Command category to search in
    /// * `command_name` - Name of the command to execute
    /// * `args` - Command-line arguments to pass to the command
    ///
    /// # Errors
    /// Returns `CliError::CommandNotFound` if the command doesn't exist in the category.
    /// Returns `CliError::ExecutionError` if the command fails during execution.
    pub fn execute_command(
        &self,
        category: &str,
        command_name: &str,
        args: &[String],
    ) -> Result<String, CliError> {
        self.registry.execute(category, command_name, args)
    }

    /// Lists all available commands organized by category.
    ///
    /// Returns a vector of tuples where each tuple contains a category name
    /// and a vector of command names within that category. Useful for generating
    /// help text or command discovery interfaces.
    ///
    /// # Returns
    /// Vector of (category_name, command_names) tuples
    pub fn list_all(&self) -> Vec<(String, Vec<String>)> {
        self.registry.list_commands()
    }
}
