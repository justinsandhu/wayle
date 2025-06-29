use std::sync::Arc;

use crate::config_store::ConfigStore;

use super::{CliError, CommandRegistry, CommandResult};

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
    pub fn execute_command(&self, category: &str, command: &str, args: &[String]) -> CommandResult {
        if self.is_help_request(category, command, args) {
            return self.handle_help_request(category, command, args);
        }

        self.registry.execute(category, command, args)
    }

    fn is_help_request(&self, category: &str, command: &str, args: &[String]) -> bool {
        category == "help" || command == "help" || args.first().map(|s| s.as_str()) == Some("help")
    }

    fn handle_help_request(&self, category: &str, command: &str, args: &[String]) -> CommandResult {
        match (category, command, args.first().map(|s| s.as_str())) {
            // wayle help (general only)
            ("help", "", None) => self.generate_general_help(),
            // wayle <category> help
            (category_name, "help", None) => self.generate_category_help(category_name),
            // wayle <category> <command> help
            (category_name, command_name, Some("help")) => {
                self.generate_command_help(category_name, command_name)
            }
            _ => self.generate_general_help(),
        }
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

    fn generate_general_help(&self) -> CommandResult {
        let categories = self.registry.get_categories();
        let mut help = String::from(
            "Wayle Desktop Shell CLI\n\nUSAGE:\n    wayle <CATEGORY> <COMMAND> [ARGS...]\n\nCATEGORIES:\n",
        );

        for category in categories {
            let commands = self
                .registry
                .get_commands_in_category(&category)
                .unwrap_or_default();
            let description = if !commands.is_empty() {
                format!("{} management", category)
            } else {
                "No commands".to_string()
            };
            help.push_str(&format!("    {:<12} {}\n", category, description));
        }

        help.push_str("\nUse 'wayle <category> help' for category-specific help.");
        help.push_str("\nUse 'wayle <category> <command> help' for command-specific help.");

        Ok(help)
    }

    fn generate_category_help(&self, category: &str) -> CommandResult {
        let commands = self
            .registry
            .get_commands_in_category(category)
            .ok_or_else(|| CliError::CommandNotFound(format!("Category: {}", category)))?;

        let mut help = format!("{} commands:\n\n", category.to_uppercase());

        for command_name in commands {
            if let Some(metadata) = self.registry.get_command_metadata(category, &command_name) {
                help.push_str(&format!(
                    "    {:<12} {}\n",
                    command_name, metadata.description
                ));
            }
        }

        help.push_str(&format!(
            "\nUse 'wayle {} <command> help' for command-specific help.",
            category
        ));

        Ok(help)
    }

    fn generate_command_help(&self, category: &str, command: &str) -> CommandResult {
        let metadata = self
            .registry
            .get_command_metadata(category, command)
            .ok_or_else(|| CliError::CommandNotFound(format!("{} {}", category, command)))?;

        let mut help = format!(
            "{}\n\nUSAGE:\n    wayle {} {}",
            metadata.description, category, command
        );

        for arg in &metadata.args {
            if arg.required {
                help.push_str(&format!(" <{}>", arg.name.to_uppercase()));
            } else {
                help.push_str(&format!(" [{}]", arg.name.to_uppercase()));
            }
        }
        help.push('\n');

        if !metadata.args.is_empty() {
            help.push_str("\nARGUMENTS:\n");
            for arg in &metadata.args {
                let arg_display = if arg.required {
                    format!("<{}>", arg.name.to_uppercase())
                } else {
                    format!("[{}]", arg.name.to_uppercase())
                };
                help.push_str(&format!("    {:<12} {}\n", arg_display, arg.description));
            }
        }

        if !metadata.examples.is_empty() {
            help.push_str("\nEXAMPLES:\n");
            for example in &metadata.examples {
                help.push_str(&format!("    {}\n", example));
            }
        }

        Ok(help)
    }
}
