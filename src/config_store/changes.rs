use std::time::Instant;

use toml::Value;

/// Represents a configuration change with path-based identification.
///
/// This struct captures all relevant information about a configuration change,
/// including what changed, when it changed, and what triggered the change.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigChange {
    /// Path to the changed field using dot notation (e.g., "modules.clock.general.format").
    pub path: String,
    /// The previous value of the field, if available.
    pub old_value: Option<Value>,
    /// The new value of the field.
    pub new_value: Value,
    /// Timestamp when the change occurred.
    pub timestamp: Instant,
    /// What triggered this configuration change.
    pub source: ChangeSource,
}

/// Identifies what triggered a configuration change.
///
/// This enum helps with debugging and conditional logic by tracking
/// the origin of each configuration change.
#[derive(Debug, Clone, PartialEq)]
pub enum ChangeSource {
    /// Change made through the GUI settings window.
    Gui,
    /// Configuration file was edited externally (e.g., text editor).
    FileEdit,
    /// Configuration file was reloaded (e.g., on startup or manual reload).
    FileReload,
    /// A preset configuration was loaded.
    PresetLoad(String),
    /// Change made through CLI command.
    CliCommand(String),
    /// Change made by wayle itself.
    System,
    /// Change made through IPC (D-Bus or other inter-process communication).
    Ipc,
}

/// Errors that can occur during configuration operations.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// The specified configuration path does not exist.
    #[error("Invalid config path: {0}")]
    InvalidPath(String),

    /// The value type does not match the expected type for the field.
    #[error("Type mismatch at {path}: Expected {expected_type}, got {actual_value:?}")]
    TypeMismatch {
        /// The path where the type mismatch occurred.
        path: String,
        /// The expected type name.
        expected_type: &'static str,
        /// The actual value that was provided.
        actual_value: Value,
    },

    /// A configuration field that was previously available has been removed.
    #[error("Config field removed: {0}")]
    FieldRemoved(String),

    /// Error in path pattern matching or parsing.
    #[error("Path pattern error: {0}")]
    PatternError(String),

    /// Error occurred while persisting configuration to disk.
    #[error("Persistence error: {0}")]
    PersistenceError(String),

    /// Error occurred while serializing toml
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Error occurred while deserializing toml
    #[error("Deserialization error: {0}")]
    DeserializationError(String),
}

impl ConfigChange {
    /// Creates a new configuration change.
    ///
    /// # Arguments
    ///
    /// * `path` - The dot-separated path to the configuration field
    /// * `old_value` - The previous value of the field (if known)
    /// * `new_value` - The new value of the field
    /// * `source` - What triggered this change
    pub fn new(
        path: String,
        old_value: Option<Value>,
        new_value: Value,
        source: ChangeSource,
    ) -> Self {
        Self {
            path,
            old_value,
            new_value,
            timestamp: Instant::now(),
            source,
        }
    }

    /// Extracts the new value as a specific type.
    ///
    /// This method attempts to deserialize the new value into the requested type.
    /// It provides type safety when extracting configuration values.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::TypeMismatch` if the value cannot be deserialized
    /// into the requested type.
    pub fn extract<T>(&self) -> Result<T, ConfigError>
    where
        T: serde::de::DeserializeOwned,
    {
        let handle_err = |_e: toml::de::Error| -> ConfigError {
            ConfigError::TypeMismatch {
                path: self.path.clone(),
                expected_type: std::any::type_name::<T>(),
                actual_value: self.new_value.clone(),
            }
        };

        T::deserialize(self.new_value.clone()).map_err(handle_err)
    }

    /// Attempts to extract the new value as a string.
    ///
    /// Returns `None` if the value is not a string.
    pub fn as_string(&self) -> Option<String> {
        match &self.new_value {
            Value::String(s) => Some(s.clone()),
            _ => None,
        }
    }

    /// Extracts the new value as a string with a fallback default.
    ///
    /// If the value is not a string, returns the provided default value.
    pub fn as_string_or(&self, default: &str) -> String {
        self.as_string().unwrap_or_else(|| default.to_string())
    }
}
