use super::{ConfigChange, ConfigError, ConfigStore, diff};
use crate::config::{Config, ConfigPaths};

use notify::{
    EventKind, RecursiveMode, Watcher,
    event::{AccessKind, AccessMode},
    recommended_watcher,
};
use std::{
    sync::mpsc,
    time::{Duration, Instant},
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
        let (std_tx, std_rx) = mpsc::channel();
        let (tokio_tx, mut tokio_rx) = tokio::sync::mpsc::unbounded_channel();

        let mut watcher = recommended_watcher(move |res| {
            if let Ok(event) = res {
                let _ = std_tx.send(event);
            }
        })
        .map_err(|e| ConfigError::FileWatcherInitError {
            details: format!("Failed to create watcher: {}", e),
        })?;

        let bridge_tx = tokio_tx.clone();
        std::thread::spawn(move || {
            while let Ok(event) = std_rx.recv() {
                if bridge_tx.send(event).is_err() {
                    break;
                }
            }
        });

        let config_dir =
            ConfigPaths::config_dir().map_err(|e| ConfigError::FileWatcherInitError {
                details: format!("Failed to get config directory: {}", e),
            })?;

        watcher
            .watch(&config_dir, RecursiveMode::NonRecursive)
            .map_err(|e| ConfigError::FileWatcherInitError {
                details: format!("Failed to watch config directory: {}", e),
            })?;

        let store = self.clone();

        tokio::spawn(async move {
            let _watcher = watcher;
            let mut pending_changes = false;
            let mut last_change = Instant::now();
            let debounce_duration = Duration::from_millis(100);

            loop {
                tokio::select! {
                    event = tokio_rx.recv() => {
                        let Some(event) = event else { break };

                        if is_relevant_event(&event) {
                            pending_changes = true;
                            last_change = Instant::now();
                        }
                    }

                    _ = tokio::time::sleep(debounce_duration), if pending_changes => {
                        if last_change.elapsed() < debounce_duration {
                            continue;
                        }

                        if let Err(e) = store.reload_from_files().await {
                            eprintln!("Failed to reload config: {}", e);
                        }

                        pending_changes = false;
                    }
                }
            }
        });

        Ok(())
    }

    async fn reload_from_files(&self) -> Result<(), ConfigError> {
        let old_config = self.get_current();
        let new_config = Config::load_with_imports(&ConfigPaths::main_config()).map_err(|e| {
            ConfigError::ProcessingError {
                operation: "reload config".to_string(),
                details: e.to_string(),
            }
        })?;

        let changes = self.diff_configs(&old_config, &new_config).map_err(|e| {
            ConfigError::ProcessingError {
                operation: "diff configs".to_string(),
                details: e.to_string(),
            }
        })?;

        self.update_config(new_config)?;

        for change in &changes {
            self.broadcast_change(change.clone());
        }

        Ok(())
    }

    fn diff_configs(
        &self,
        old: &Config,
        new: &Config,
    ) -> Result<Vec<ConfigChange>, Box<dyn std::error::Error>> {
        diff::diff_configs(old, new)
    }
}

/// Checks if a file system event is relevant for config reloading.
fn is_relevant_event(event: &notify::Event) -> bool {
    let is_write_event = matches!(
        event.kind,
        EventKind::Modify(_)
            | EventKind::Create(_)
            | EventKind::Remove(_)
            | EventKind::Access(AccessKind::Close(AccessMode::Write))
    );

    is_write_event
        && event.paths.iter().any(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|name| name.ends_with(".toml"))
        })
}
