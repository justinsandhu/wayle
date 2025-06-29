use std::path::Path;

use thiserror::Error;

/// Error types for the Wayle application.
///
/// This enum represents all possible errors that can occur during
/// configuration loading, parsing, and import operations.
#[derive(Error, Debug)]
pub enum WayleError {
    /// Configuration validation error
    #[error("configuration validation failed for '{component}': {details}")]
    ConfigValidation {
        /// Component that failed validation
        component: String,
        /// Validation error details
        details: String,
    },

    /// Configuration field missing or invalid
    #[error("invalid config field '{field}' in {component}: {reason}")]
    InvalidConfigField {
        /// The field that is invalid
        field: String,
        /// Component containing the field
        component: String,
        /// Reason why the field is invalid
        reason: String,
    },

    /// I/O operation error
    #[error("I/O error on '{path}': {details}")]
    IoError {
        /// Path where I/O error occurred
        path: std::path::PathBuf,
        /// I/O error details
        details: String,
    },

    /// Standard I/O operation error (for compatibility)
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// TOML parsing error with location context
    #[error("failed to parse TOML at '{location}': {details}")]
    TomlParseError {
        /// Location of TOML being parsed (file path or "string")
        location: String,
        /// Parse error details
        details: String,
    },

    /// Import operation error with file context
    #[error("failed to import '{path}': {details}")]
    ImportError {
        /// Path of file being imported
        path: std::path::PathBuf,
        /// Import error details
        details: String,
    },
}

/// A specialized `Result` type for Wayle operations.
///
/// This type alias simplifies error handling by defaulting the error type
/// to `WayleError` for all Wayle operations.
pub type Result<T> = std::result::Result<T, WayleError>;

impl WayleError {
    /// Creates a TOML parsing error with optional file path context.
    ///
    /// # Arguments
    ///
    /// * `error` - The underlying parsing error
    /// * `path` - Optional path to the file that failed to parse
    pub fn toml_parse(error: impl std::fmt::Display, path: Option<&Path>) -> Self {
        let location = match path {
            Some(p) => {
                let clean_path = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
                clean_path.to_string_lossy().to_string()
            }
            None => "string".to_string(),
        };

        WayleError::TomlParseError {
            location,
            details: error.to_string(),
        }
    }

    /// Creates an import error with file path context.
    ///
    /// # Arguments
    ///
    /// * `error` - The underlying import error
    /// * `path` - Path to the file that failed to import
    pub fn import(error: impl std::fmt::Display, path: &Path) -> Self {
        let clean_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

        WayleError::ImportError {
            path: clean_path,
            details: error.to_string(),
        }
    }
}
