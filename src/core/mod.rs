use std::path::Path;

use thiserror::Error;

/// Error types for the Wayle application.
///
/// This enum represents all possible errors that can occur during
/// configuration loading, parsing, and import operations.
#[derive(Error, Debug)]
pub enum WayleError {
    /// Configuration validation error.
    #[error("{0}")]
    Config(String),

    /// I/O operation error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// TOML parsing error.
    #[error("{0}")]
    TomlParse(String),

    /// Import operation error.
    #[error("{0}")]
    Import(String),
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
        match path {
            Some(p) => {
                let clean_path = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
                WayleError::TomlParse(format!("Failed to parse TOML at {clean_path:?}: {error}",))
            }
            None => WayleError::TomlParse(format!("Failed to parse TOML: {}", error)),
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

        WayleError::Import(format!(
            "Failed to import file path: {clean_path:?}. Error: {error}"
        ))
    }
}
