use thiserror::Error;

/// Errors that can occur during CLI command execution.
///
/// This enum represents all possible error conditions in the CLI system,
/// from command discovery failures to execution errors. Each variant provides
/// contextual information to help users understand what went wrong.
#[derive(Error, Debug)]
pub enum CliError {
    /// A command or category was not found in the registry.
    ///
    /// This occurs when users specify a command that doesn't exist, either
    /// because the category is invalid or the command name is wrong within
    /// a valid category.
    #[error("Command not found: {0}")]
    CommandNotFound(String),

    /// Invalid arguments were provided to a command.
    ///
    /// This error is returned when argument validation fails, such as
    /// missing required arguments, too many arguments, or arguments
    /// that don't match the expected format.
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),

    /// An error occurred in the configuration system.
    ///
    /// This wraps errors from the reactive config store, such as
    /// invalid paths, type mismatches, or file system issues.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// A general service error occurred.
    ///
    /// This is used for errors that don't fit other categories,
    /// such as missing dependencies or service unavailability.
    #[error("Service error: {0}")]
    ServiceError(String),

    /// An I/O operation failed.
    ///
    /// This automatically converts from `std::io::Error` for file
    /// operations, network requests, or other I/O-related failures.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Type alias for command execution results.
///
/// All CLI commands return this type, providing either a success message
/// as a String or a CliError describing what went wrong. This standardizes
/// error handling across the entire CLI system.
pub type CommandResult = Result<String, CliError>;

/// Specification for a single command argument.
///
/// This struct defines the metadata for command arguments, enabling
/// automatic help generation, validation, and type checking. Arguments
/// can be required or optional, with type hints for better user experience.
#[derive(Debug, Clone)]
pub struct CommandArg {
    /// The name of the argument (e.g., "path", "value", "file").
    pub name: String,

    /// Human-readable description of what this argument does.
    pub description: String,

    /// Whether this argument is required for command execution.
    pub required: bool,

    /// The expected type of this argument for validation and help display.
    pub value_type: ArgType,
}

/// Type classification for command arguments.
///
/// This enum helps with argument validation and provides hints
/// in help text about what kind of value is expected. The type
/// information improves user experience and enables better error messages.
#[derive(Debug, Clone)]
pub enum ArgType {
    /// A general string value.
    String,

    /// A numeric value (integer or float).
    Number,

    /// A boolean value (true/false, yes/no, 1/0).
    Boolean,

    /// A file system path or configuration path.
    Path,
}

/// Complete metadata for a CLI command.
///
/// This struct serves as the single source of truth for everything about
/// a command: its identity, arguments, usage examples, and categorization.
/// The CLI system uses this metadata for help generation, argument validation,
/// and command discovery.
#[derive(Debug, Clone)]
pub struct CommandMetadata {
    /// The command name (e.g., "get", "set", "watch").
    pub name: String,

    /// Brief description of what this command does.
    pub description: String,

    /// Specification of all arguments this command accepts.
    pub args: Vec<CommandArg>,

    /// Example usage strings to show in help text.
    pub examples: Vec<String>,

    /// Category this command belongs to (e.g., "config", "system").
    pub category: String,
}

/// Trait defining the interface for all CLI commands.
///
/// All commands implement this trait to provide consistent execution
/// and metadata discovery. Commands receive dependencies through
/// their constructors.
pub trait Command: Send + Sync {
    /// Executes the command with the provided arguments.
    ///
    /// The command is responsible for its own argument validation and
    /// business logic. Arguments are provided as a slice of strings,
    /// with the registry having already performed basic count validation
    /// against the command's metadata.
    ///
    /// # Arguments
    ///
    /// * `args` - Command-line arguments passed by the user
    ///
    /// # Errors
    ///
    /// Returns `CliError` for any execution failures, including:
    /// - Invalid argument values
    /// - Configuration system errors
    /// - Service unavailability
    /// - I/O failures
    fn execute(&self, args: &[String]) -> CommandResult;

    /// Returns the complete metadata for this command.
    ///
    /// This metadata is used by the CLI system for help generation,
    /// argument validation, and command discovery. The metadata should
    /// be consistent and complete, as it serves as the authoritative
    /// definition of the command's interface.
    fn metadata(&self) -> CommandMetadata;
}
