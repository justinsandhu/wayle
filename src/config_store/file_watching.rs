use crate::config::{Config, ConfigPaths};
use std::{
    collections::HashMap,
    path::PathBuf,
    time::{Duration, Instant},
};

use super::{
    ChangeSource, ConfigChange, ConfigError, ConfigStore, diff, file_watcher::FileWatcher,
};

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
        let (mut watcher, mut event_rx) = FileWatcher::new().map_err(|e| {
            ConfigError::FileWatchError(format!("Failed to create file watcher: {}", e))
        })?;

        let files_to_watch = self.get_config_files().await?;

        watcher
            .update_watched_files(files_to_watch)
            .await
            .map_err(|e| {
                ConfigError::FileWatchError(format!("Failed to updated watched files: {}", e))
            })?;

        let store = self.clone();

        tokio::spawn(async move {
            let mut pending_reloads: HashMap<PathBuf, Instant> = HashMap::new();

            let debounce_duration = Duration::from_millis(500);
            let debounce_sleep = tokio::time::sleep(debounce_duration);

            tokio::pin!(debounce_sleep);

            loop {
                tokio::select! {
                    Some(event) = event_rx.recv() => {
                        pending_reloads.insert(event.path.clone(), Instant::now());

                        debounce_sleep.as_mut().reset(tokio::time::Instant::now() + debounce_duration);
                    }

                    _ = &mut debounce_sleep, if !pending_reloads.is_empty() => {
                        if let Err(e) = store.reload_from_files().await {
                            eprintln!("Failed to reload config: {}", e);
                        }

                        pending_reloads.clear();
                    }
                }
            }
        });

        Ok(())
    }

    async fn get_config_files(&self) -> Result<Vec<PathBuf>, ConfigError> {
        let mut files = Config::get_all_config_files(&ConfigPaths::main_config())
            .map_err(|e| ConfigError::IoError(format!("Failed to collect config files: {}", e)))?;

        let runtime_path = ConfigPaths::runtime_config();
        if !files.contains(&runtime_path) {
            files.push(runtime_path);
        }

        Ok(files)
    }

    async fn reload_from_files(&self) -> Result<(), ConfigError> {
        let old_config = self.get_current();
        let new_config = Config::load_with_imports(&ConfigPaths::main_config())
            .map_err(|e| ConfigError::IoError(format!("Failed to reload config: {}", e)))?;

        let changes = self
            .diff_configs(&old_config, &new_config)
            .map_err(|e| ConfigError::ProcessingError(format!("Failed to diff configs: {}", e)))?;

        for change in changes {
            self.broadcast_change(change);
        }

        Ok(())
    }

    fn diff_configs(
        &self,
        old: &Config,
        new: &Config,
    ) -> Result<Vec<ConfigChange>, Box<dyn std::error::Error>> {
        diff::diff_configs(old, new, ChangeSource::FileEdit)
    }
}
