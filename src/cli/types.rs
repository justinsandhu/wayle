use thiserror::Error;

/// Errors that can occur during CLI command execution.
#[derive(Error, Debug)]
pub enum CliError {
    /// Command not found in registry
    #[error("command '{}' not found", command)]
    CommandNotFound {
        /// The command that was not found
        command: String,
    },

    /// Missing required arguments
    #[error("missing required argument(s): {}\nUsage: {}", missing, usage)]
    MissingArguments {
        /// Comma-separated list of missing arguments
        missing: String,
        /// Usage string showing correct syntax
        usage: String,
    },

    /// Too many arguments provided
    #[error("too many arguments (expected {}, got {})", expected, actual)]
    TooManyArguments {
        /// Expected number of arguments
        expected: usize,
        /// Actual number provided
        actual: usize,
    },

    /// Missing path argument for config commands
    #[error("missing required path argument")]
    MissingPath,

    /// Missing value argument for set command
    #[error("missing required value argument")]
    MissingValue,

    /// Invalid configuration value format
    #[error("invalid value format for path '{path}': {reason}")]
    InvalidConfigValue {
        /// The configuration path being set
        path: String,
        /// Reason why the value is invalid
        reason: String,
    },

    /// Configuration path not found
    #[error("configuration path '{path}' not found")]
    ConfigPathNotFound {
        /// The path that was not found
        path: String,
    },

    /// Configuration store operation failed
    #[error("failed to {operation} config at '{path}': {details}")]
    ConfigOperationFailed {
        /// The operation that failed (get, set, watch)
        operation: String,
        /// The configuration path involved
        path: String,
        /// Additional error details
        details: String,
    },

    /// Runtime initialization failed
    #[error("failed to initialize runtime: {details}")]
    RuntimeInitFailed {
        /// Error details from runtime creation
        details: String,
    },

    /// Service connection failed
    #[error("failed to connect to {service}: {details}")]
    ServiceConnectionFailed {
        /// The service that failed to connect
        service: String,
        /// Connection failure details
        details: String,
    },

    /// I/O operation failed
    #[error(transparent)]
    Io(#[from] std::io::Error),
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

    /// Category this command belongs to (e.g., "config", "system").
    pub category: String,

    /// Specification of all arguments this command accepts.
    pub args: Vec<CommandArg>,

    /// Example usage strings to show in help text.
    pub examples: Vec<String>,
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
