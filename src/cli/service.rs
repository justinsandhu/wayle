use std::sync::Arc;

use crate::config_store::ConfigStore;

use super::{CliError, CommandRegistry, CommandResult, formatting::*};

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
    /// Other errors may be returned by the command's execute method.
    pub async fn execute_command(&self, category: &str, command: &str, args: &[String]) -> CommandResult {
        if self.is_help_request(category, command, args) {
            return self.handle_help_request(category, command, args);
        }

        self.registry.execute(category, command, args).await
    }

    fn is_help_request(&self, category: &str, command: &str, args: &[String]) -> bool {
        category == "help"
            || command == "help"
            || command.is_empty()
            || args.first().map(|s| s.as_str()) == Some("help")
    }

    fn handle_help_request(&self, category: &str, command: &str, args: &[String]) -> CommandResult {
        match (category, command, args.first().map(|s| s.as_str())) {
            ("help", "", None) => self.generate_general_help(),
            (category_name, "", None) if category_name != "help" => {
                self.generate_category_help(category_name)
            }
            (category_name, "help", None) => self.generate_category_help(category_name),
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
        let mut help = String::new();

        help.push_str(&format!("{}\n", format_header("Wayle Desktop Shell")));
        help.push_str(&format!(
            "{}\n\n",
            format_description("A beautiful Wayland desktop shell and panel system")
        ));

        help.push_str(&format!("{}\n", format_subheader("USAGE")));
        help.push_str("    wayle <CATEGORY> <COMMAND> [ARGS...]\n\n");

        help.push_str(&format!("{}\n", format_subheader("CATEGORIES")));
        for category in categories {
            let commands = self
                .registry
                .get_commands_in_category(&category)
                .unwrap_or_default();
            let description = if !commands.is_empty() {
                format!("{category} management")
            } else {
                "No commands".to_string()
            };
            help.push_str(&format!(
                "    {:<16} {}\n",
                format_category(&category),
                format_description(&description)
            ));
        }

        help.push_str(&format!(
            "\n{}\n",
            format_description("Use 'wayle <category>' for category-specific help")
        ));
        help.push_str(&format!(
            "{}\n",
            format_description("Use 'wayle <category> <command> help' for command-specific help")
        ));

        Ok(help)
    }

    fn generate_category_help(&self, category: &str) -> CommandResult {
        let commands = self
            .registry
            .get_commands_in_category(category)
            .ok_or_else(|| CliError::CommandNotFound {
                command: format!("{category} (category)"),
            })?;

        let mut help = String::new();

        help.push_str(&format!(
            "{}\n\n",
            format_subheader(&format!("{category} commands"))
        ));

        for command_name in commands {
            if let Some(metadata) = self.registry.get_command_metadata(category, &command_name) {
                help.push_str(&format!(
                    "    {:<16} {}\n",
                    format_command(&command_name),
                    format_description(&metadata.description)
                ));
            }
        }

        help.push_str(&format!(
            "\n{}\n",
            format_description(&format!(
                "Use 'wayle {category} <command> help' for detailed help"
            ))
        ));

        Ok(help)
    }

    fn generate_command_help(&self, category: &str, command: &str) -> CommandResult {
        let metadata = self
            .registry
            .get_command_metadata(category, command)
            .ok_or_else(|| CliError::CommandNotFound {
                command: format!("{category} {command}"),
            })?;

        let mut help = String::new();

        help.push_str(&format!("{}\n", format_subheader("DESCRIPTION")));
        help.push_str(&format!("    {}\n\n", metadata.description));

        help.push_str(&format!("{}\n", format_subheader("USAGE")));
        help.push_str(&format!(
            "    {} {} {}",
            format_header("wayle"),
            format_category(category),
            format_command(command)
        ));

        for arg in &metadata.args {
            if arg.required {
                help.push_str(&format!(
                    " {}",
                    format_usage(&format!("<{}>", arg.name.to_uppercase()))
                ));
            } else {
                help.push_str(&format!(
                    " {}",
                    format_description(&format!("[{}]", arg.name.to_uppercase()))
                ));
            }
        }
        help.push_str("\n\n");

        if !metadata.args.is_empty() {
            help.push_str(&format!("{}\n", format_subheader("ARGUMENTS")));
            for arg in &metadata.args {
                let arg_display = if arg.required {
                    format!("<{}>", arg.name.to_uppercase())
                } else {
                    format!("[{}]", arg.name.to_uppercase())
                };
                let required_suffix = if arg.required { "" } else { " (optional)" };

                help.push_str(&format!(
                    "    {:<16} {}{}\n",
                    format_usage(&arg_display),
                    format_description(&arg.description),
                    format_description(required_suffix)
                ));
            }
            help.push('\n');
        }

        if !metadata.examples.is_empty() {
            help.push_str(&format!("{}\n", format_subheader("EXAMPLES")));
            for example in &metadata.examples {
                help.push_str(&format!("    {}\n", format_usage(example)));
            }
        }

        Ok(help)
    }
}
