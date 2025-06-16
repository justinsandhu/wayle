use std::path::Path;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum WayleError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    TomlParse(String),

    #[error("{0}")]
    Import(String),
}

pub type Result<T> = std::result::Result<T, WayleError>;

impl WayleError {
    pub fn toml_parse(error: impl std::fmt::Display, path: Option<&Path>) -> Self {
        match path {
            Some(p) => {
                let clean_path = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
                WayleError::TomlParse(format!(
                    "Failed to parse TOML at {:?}: {}",
                    clean_path, error
                ))
            }
            None => WayleError::TomlParse(format!("Failed to parse TOML: {}", error)),
        }
    }

    pub fn import(error: impl std::fmt::Display, path: &Path) -> Self {
        let clean_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

        WayleError::Import(format!(
            "Failed to import file path: {:?}. Error {}",
            clean_path, error
        ))
    }
}
