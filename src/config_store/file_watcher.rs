use std::{
    collections::HashSet,
    error,
    path::{Path, PathBuf},
    sync::Arc,
};

use notify::{Event, EventKind, RecommendedWatcher, Watcher, recommended_watcher};
use tokio::sync::{RwLock, mpsc};

/// Represents a file system event for a watched file.
#[derive(Debug, Clone)]
pub struct FileEvent {
    /// The path of the file that changed
    pub path: PathBuf,
    /// The type of change that occurred
    pub kind: FileEventKind,
}

/// The type of file system change that occurred.
#[derive(Debug, Clone, PartialEq)]
pub enum FileEventKind {
    /// File was modified
    Modified,
    /// File was created
    Created,
    /// File was removed
    Removed,
}

/// Cross-platform file system watcher for monitoring configuration files.
///
/// Provides an async interface over the notify crate for watching file changes
/// and converting them to Tokio-compatible async events.
pub struct FileWatcher {
    watcher: RecommendedWatcher,
    watched_files: Arc<RwLock<HashSet<PathBuf>>>,
    event_tx: mpsc::UnboundedSender<FileEvent>,
}

impl FileWatcher {
    /// Creates a new file watcher and returns the watcher and event receiver.
    ///
    /// The receiver can be used to listen for file system events from any watched files.
    /// Uses an unbounded channel since file events are typically infrequent but bursty.
    ///
    /// # Errors
    /// Returns error if the underlying file system watcher cannot be initialized.
    pub fn new() -> Result<(Self, mpsc::UnboundedReceiver<FileEvent>), Box<dyn error::Error>> {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let event_tx_clone = event_tx.clone();

        let watcher = recommended_watcher(move |res: notify::Result<Event>| {
            let Ok(event) = res else {
                return;
            };

            let kind = match event.kind {
                EventKind::Create(_) => FileEventKind::Created,
                EventKind::Modify(_) => FileEventKind::Modified,
                EventKind::Remove(_) => FileEventKind::Removed,
                _ => return,
            };

            for path in event.paths {
                let _ = event_tx_clone.send(FileEvent {
                    path,
                    kind: kind.clone(),
                });
            }
        })?;

        let file_watcher = (
            Self {
                watcher,
                watched_files: Arc::new(RwLock::new(HashSet::new())),
                event_tx,
            },
            event_rx,
        );

        Ok(file_watcher)
    }

    /// Adds a file to the watch list for monitoring changes.
    ///
    /// Files are automatically canonicalized to handle symlinks and relative paths.
    /// If the file is already being watched, this operation is a no-op.
    ///
    /// # Arguments
    /// * `path` - Path to the file to watch
    ///
    /// # Errors
    /// Returns error if the path cannot be canonicalized or the watcher fails to monitor it.
    pub async fn watch_file(&mut self, path: impl AsRef<Path>) -> Result<(), notify::Error> {
        let path = path.as_ref();
        let canonical = path.canonicalize()?;

        let mut watched = self.watched_files.write().await;
        if watched.contains(&canonical) {
            return Ok(());
        }

        self.watcher
            .watch(&canonical, notify::RecursiveMode::NonRecursive)?;

        watched.insert(canonical);

        Ok(())
    }

    /// Removes a file from the watch list.
    ///
    /// If the file is not currently being watched, this operation is a no-op.
    ///
    /// # Arguments
    /// * `path` - Path to the file to stop watching
    ///
    /// # Errors
    /// Returns error if the path cannot be canonicalized or the watcher fails to stop monitoring.
    pub async fn unwatch_file(&mut self, path: impl AsRef<Path>) -> Result<(), notify::Error> {
        let path = path.as_ref();
        let canonical = path.canonicalize()?;

        let mut watched = self.watched_files.write().await;
        if watched.remove(&canonical) {
            self.watcher.unwatch(&canonical)?;
        }

        Ok(())
    }

    /// Updates the complete set of watched files, adding new ones and removing old ones.
    ///
    /// This is more efficient than manually adding/removing individual files when the
    /// entire watch list needs to be updated.
    ///
    /// # Arguments
    /// * `new_files` - Complete list of files that should be watched
    ///
    /// # Errors
    /// Returns error if any file operations fail during the update process.
    pub async fn update_watched_files(
        &mut self,
        new_files: Vec<PathBuf>,
    ) -> Result<(), notify::Error> {
        let new_set: HashSet<PathBuf> = new_files
            .into_iter()
            .filter_map(|p| p.canonicalize().ok())
            .collect();
        let current = self.watched_files.read().await.clone();

        for file in current.difference(&new_set) {
            self.unwatch_file(file).await?;
        }

        for file in new_set.difference(&current) {
            self.watch_file(file).await?;
        }

        Ok(())
    }
}
