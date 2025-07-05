use super::{ConfigChange, ConfigError, ConfigStore, diff};
use crate::config::{Config, ConfigPaths};

use notify::{
    EventKind, RecursiveMode, Watcher,
    event::{AccessKind, AccessMode},
    recommended_watcher,
};
use std::{
    sync::mpsc::{self, Receiver, RecvTimeoutError, Sender},
    thread,
    time::{Duration, Instant},
};

/// File watcher that monitors configuration changes.
///
/// When this watcher is dropped, file watching stops automatically.
/// This ensures clean resource management without leaks.
pub struct FileWatcher {
    /// Keeps the watcher alive via RAII
    _watcher: Box<dyn Watcher + Send>,
    /// Channel to signal thread shutdown
    shutdown_tx: Sender<()>,
    /// Background thread processing file events
    thread: Option<thread::JoinHandle<()>>,
}

impl Drop for FileWatcher {
    fn drop(&mut self) {
        let _ = self.shutdown_tx.send(());

        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl ConfigStore {
    /// Starts monitoring configuration files for changes and broadcasts updates.
    ///
    /// Returns a `FileWatchHandle` that controls the file watching lifecycle.
    /// When the handle is dropped, file watching stops automatically.
    ///
    /// # Errors
    /// Returns `ConfigError::FileWatcherInitError` if file watching cannot be initialized
    /// or configuration directory cannot be accessed.
    pub fn start_file_watching(&self) -> Result<FileWatcher, ConfigError> {
        let (tx, rx) = mpsc::channel();

        let mut watcher = recommended_watcher(move |res| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        })
        .map_err(|e| ConfigError::FileWatcherInitError {
            details: format!("Failed to create watcher: {}", e),
        })?;

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
        let (shutdown_tx, shutdown_rx) = mpsc::channel();

        let thread = thread::spawn(move || {
            file_watch_loop(rx, shutdown_rx, store);
        });

        Ok(FileWatcher {
            _watcher: Box::new(watcher),
            shutdown_tx,
            thread: Some(thread),
        })
    }

    fn reload_from_files(&self) -> Result<(), ConfigError> {
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

/// Main file watching loop that processes events until shutdown.
fn file_watch_loop(
    event_rx: Receiver<notify::Event>,
    shutdown_rx: Receiver<()>,
    store: ConfigStore,
) {
    let mut pending_changes = false;
    let mut last_change = Instant::now();
    let debounce_duration = Duration::from_millis(100);

    loop {
        if shutdown_rx.try_recv().is_ok() {
            break;
        }

        match event_rx.recv_timeout(debounce_duration) {
            Ok(event) => {
                if is_relevant_event(&event) {
                    pending_changes = true;
                    last_change = Instant::now();
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                if pending_changes && last_change.elapsed() >= debounce_duration {
                    if let Err(e) = store.reload_from_files() {
                        eprintln!("Failed to reload config: {}", e);
                    }
                    pending_changes = false;
                }
            }
            Err(RecvTimeoutError::Disconnected) => {
                break;
            }
        }
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
