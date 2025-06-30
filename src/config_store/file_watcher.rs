use std::{
    error,
    path::Path,
};

use notify::{Event, EventKind, RecommendedWatcher, Watcher, recommended_watcher};
use tokio::sync::mpsc;

/// Represents a file system event for a watched directory.
#[derive(Debug, Clone)]
pub struct FileEvent {
    /// The path of the file that changed
    pub path: std::path::PathBuf,
}

/// Cross-platform directory watcher for monitoring configuration changes.
///
/// Watches a directory for file changes and filters for relevant config files.
pub struct FileWatcher {
    watcher: RecommendedWatcher,
}

impl FileWatcher {
    /// Creates a new directory watcher and returns the watcher and event receiver.
    ///
    /// The receiver can be used to listen for file system events from the watched directory.
    /// Only relevant file changes (based on extension) are forwarded.
    ///
    /// # Errors
    /// Returns error if the underlying file system watcher cannot be initialized.
    pub fn new() -> Result<(Self, mpsc::UnboundedReceiver<FileEvent>), Box<dyn error::Error>> {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let watcher = recommended_watcher(move |res: notify::Result<Event>| {
            let Ok(event) = res else {
                eprintln!("[DEBUG] Notify error: {:?}", res);
                return;
            };

            eprintln!("[DEBUG] Raw event: {:?}", event);

            // Filter out read-only access events to avoid feedback loops
            let is_write_event = match event.kind {
                EventKind::Modify(_) => true,
                EventKind::Create(_) => true,
                EventKind::Remove(_) => true,
                EventKind::Access(_) => false, // Ignore all access events
                _ => false,
            };

            if !is_write_event {
                eprintln!("[DEBUG] Ignoring non-write event: {:?}", event.kind);
                return;
            }

            // Only send events for .toml files
            for path in event.paths {
                if path.extension().and_then(|ext| ext.to_str()) == Some("toml") {
                    eprintln!("[DEBUG] Sending event for: {}", path.display());
                    let _ = event_tx.send(FileEvent { path });
                } else {
                    eprintln!("[DEBUG] Ignoring non-toml file: {}", path.display());
                }
            }
        })?;

        Ok((Self { watcher }, event_rx))
    }

    /// Watches a directory for file changes.
    ///
    /// Sets up monitoring for the specified directory. Only .toml file changes
    /// will generate events to reduce noise.
    ///
    /// # Arguments
    /// * `path` - Path to the directory to watch
    ///
    /// # Errors
    /// Returns error if the directory cannot be watched.
    pub fn watch_directory(&mut self, path: impl AsRef<Path>) -> Result<(), notify::Error> {
        let path = path.as_ref();
        self.watcher.watch(path, notify::RecursiveMode::NonRecursive)
    }
}
