use crate::{Result, WayleError};
use std::{fs, path::Path};

/// Creates a default configuration file if it doesn't exist
pub fn create_default_config_file(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| WayleError::IoError {
            path: parent.to_path_buf(),
            details: format!("Failed to create config directory: {e}"),
        })?;
    }

    fs::write(path, "# Wayle configuration file\n").map_err(|e| WayleError::IoError {
        path: path.to_path_buf(),
        details: format!("Failed to create config file: {e}"),
    })?;

    Ok(())
}
