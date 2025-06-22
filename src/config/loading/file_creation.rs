use crate::{Result, WayleError};
use std::{fs, path::Path};

/// Creates a default configuration file if it doesn't exist
pub fn create_default_config_file(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            WayleError::Config(format!(
                "Failed to create config directory {}: {}",
                parent.display(),
                e
            ))
        })?;
    }

    fs::write(path, "# Wayle configuration file\n").map_err(|e| {
        WayleError::Config(format!(
            "Failed to create config file {}: {}",
            path.display(),
            e
        ))
    })?;

    Ok(())
}

