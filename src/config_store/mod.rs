//! Reactive configuration store with change tracking.
//!
//! Provides a thread-safe configuration store that can load TOML files,
//! track changes, and notify subscribers of configuration updates.

mod broadcast;
mod changes;
mod diff;
mod file_watching;
mod path_ops;
mod store;

#[cfg(test)]
mod tests;

pub use broadcast::Subscription;
pub use changes::{ConfigChange, ConfigError};
pub use file_watching::FileWatcher;
pub use store::ConfigStore;
