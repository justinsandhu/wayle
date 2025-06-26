use crate::config::Config;
use std::path::PathBuf;

use super::{ChangeSource, ConfigChange, ConfigError, ConfigStore, diff};

impl ConfigStore {
    /// Starts monitoring configuration files for changes and broadcasts updates.
    ///
    /// Initiates file system watching for all configuration files used by this store.
    /// When changes are detected, the configuration is automatically reloaded and
    /// change events are broadcast to subscribers.
    ///
    /// # Errors
    /// Returns error if file watching cannot be initialized or configuration files
    /// cannot be accessed.
    pub async fn start_file_watching(&self) -> Result<(), ConfigError> {
        todo!("Implement file watching")
    }

    async fn get_config_files(&self) -> Result<Vec<PathBuf>, ConfigError> {
        todo!("Get list of config files to watch")
    }

    async fn reload_from_files(&self) -> Result<(), ConfigError> {
        todo!("Reload config from files")
    }

    fn diff_configs(
        &self,
        old: &Config,
        new: &Config,
    ) -> Result<Vec<ConfigChange>, Box<dyn std::error::Error>> {
        diff::diff_configs(old, new, ChangeSource::FileEdit)
    }
}
