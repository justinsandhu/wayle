use std::{env, path::PathBuf};

/// Utility struct for managing configuration file paths
///
/// Provides methods to locate configuration directories and files following
/// the XDG Base Directory specification
pub struct ConfigPaths;

impl ConfigPaths {
    /// Returns the configuration directory path for the application
    ///
    /// Follows the XDG Base Directory specification:
    /// - First checks `XDG_CONFIG_HOME`
    /// - Falls back to `$HOME/.config`
    /// - Appends "wayle" to the base config directory
    ///
    /// # Errors
    /// Returns an error if neither `XDG_CONFIG_HOME` nor `HOME` environment variables are set
    pub fn config_dir() -> Result<PathBuf, std::io::Error> {
        let config_home = env::var("XDG_CONFIG_HOME")
            .or_else(|_| env::var("HOME").map(|home| format!("{}/.config", home)))
            .map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Neither XDG_CONFIG_HOME nor HOME environment variable found",
                )
            })?;

        Ok(PathBuf::from(config_home).join("wayle"))
    }

    /// Returns the path to the main configuration file
    ///
    /// # Panics
    /// Panics if neither HOME nor XDG_CONFIG_HOME environment variables are set
    #[allow(clippy::panic)]
    pub fn main_config() -> PathBuf {
        match Self::config_dir() {
            Ok(dir) => dir.join("config.toml"),
            Err(_) => {
                panic!("Failed to determine config directory - is $HOME or $XDG_CONFIG_HOME set?")
            }
        }
    }

    /// Returns the path to the GUI configuration file
    ///
    /// # Panics
    /// Panics if neither HOME nor XDG_CONFIG_HOME environment variables are set
    #[allow(clippy::panic)]
    pub fn runtime_config() -> PathBuf {
        match Self::config_dir() {
            Ok(dir) => dir.join("runtime.toml"),
            Err(_) => {
                panic!("Failed to determine config directory - is $HOME or $XDG_CONFIG_HOME set?")
            }
        }
    }
}
