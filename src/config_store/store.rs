use std::sync::{Arc, RwLock};

use futures::Stream;
use tokio::sync::broadcast;
use toml::Value;

use crate::config::{Config, ConfigPaths};

use super::{ConfigChange, ConfigError};

/// A thread-safe configuration store that manages application settings and broadcasts changes
///
/// The ConfigStore provides a centralized way to read, write, and observe configuration changes
/// across the application.
#[derive(Clone)]
pub struct ConfigStore {
    config: Arc<RwLock<Config>>,

    change_sender: broadcast::Sender<ConfigChange>,
}

impl ConfigStore {
    /// Creates a new ConfigStore with default configuration values and a broadcast channel for change notifications
    pub fn with_defaults() -> Self {
        let config = Config::default();
        let (change_sender, _) = broadcast::channel(1000);

        Self {
            config: Arc::new(RwLock::new(config)),
            change_sender,
        }
    }

    /// Loads a ConfigStore from the main configuration file
    ///
    /// # Errors
    /// * `ConfigError::PersistenceError` - If the configuration file cannot be loaded
    pub fn load() -> Result<Self, ConfigError> {
        let main_config = ConfigPaths::main_config();
        let config = Config::load_with_imports(&main_config)
            .map_err(|e| ConfigError::PersistenceError(e.to_string()))?;
        let (change_sender, _) = broadcast::channel(1000);

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            change_sender,
        })
    }

    /// Sets a configuration value at the specified path and broadcasts the change
    ///
    /// # Arguments
    /// * `path` - Dot-separated path to the configuration field (e.g., "server.port")
    /// * `value` - The new TOML value to set at the path
    ///
    /// # Errors
    /// * `ConfigError::InvalidPath` - If the path doesn't exist
    /// * `ConfigError::PersistenceError` - If the write lock cannot be acquired
    /// * `ConfigError::SerializationError` - If the config cannot be serialized
    /// * `ConfigError::DeserializationError` - If the config cannot be deserialized
    pub fn set_by_path(&self, path: &str, value: Value) -> Result<(), ConfigError> {
        let old_value = self.get_by_path(path)?;

        {
            let mut config = self.config.write().map_err(|_| {
                ConfigError::PersistenceError("Failed to acquire write lock".into())
            })?;

            self.set_config_field(&mut config, path, &value)?;
        }

        self.save_gui_config()?;

        let change = ConfigChange::new(
            path.to_string(),
            Some(old_value),
            value,
            super::ChangeSource::Gui,
        );

        let _ = self.change_sender.send(change);
        Ok(())
    }

    /// Retrieves a configuration value at the specified path
    ///
    /// # Arguments
    /// * `path` - Dot-separated path to the configuration field (e.g., "server.port")
    ///
    /// # Errors
    /// * `ConfigError::InvalidPath` - If the path doesn't exist
    /// * `ConfigError::PersistenceError` - If the read lock cannot be acquired
    pub fn get_by_path(&self, path: &str) -> Result<Value, ConfigError> {
        let config = self
            .config
            .read()
            .map_err(|_| ConfigError::PersistenceError("Failed to acquire read lock".into()))?;

        Self::get_config_field(&config, path)
    }

    /// Returns a clone of the current configuration, handling poisoned locks gracefully
    pub fn get_current(&self) -> Config {
        match self.config.read() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    /// Creates a stream that yields ConfigChange events matching the specified path pattern
    ///
    /// # Arguments
    /// * `pattern` - A pattern to match configuration paths (supports "*" wildcards)
    pub fn subscribe_to_path(&self, pattern: &str) -> impl Stream<Item = ConfigChange> {
        let pattern = pattern.to_string();
        let receiver = self.change_sender.subscribe();

        futures::stream::unfold(receiver, move |mut receiver| {
            let pattern = pattern.clone();
            async move {
                loop {
                    match receiver.recv().await {
                        Ok(change) => {
                            if path_matches(&change.path, &pattern) {
                                return Some((change, receiver));
                            }
                        }
                        Err(_) => return None,
                    }
                }
            }
        })
    }

    fn set_config_field(
        &self,
        config: &mut Config,
        path: &str,
        value: &Value,
    ) -> Result<(), ConfigError> {
        let mut config_value = Value::try_from(config.clone())
            .map_err(|e| ConfigError::SerializationError(e.to_string()))?;

        set_value_at_path(&mut config_value, path, value.clone())?;

        *config = config_value
            .try_into()
            .map_err(|e| ConfigError::DeserializationError(e.to_string()))?;

        Ok(())
    }

    fn get_config_field(config: &Config, path: &str) -> Result<Value, ConfigError> {
        let config_value = Value::try_from(config.clone())
            .map_err(|e| ConfigError::SerializationError(e.to_string()))?;

        navigate_path(&config_value, path)
    }

    /// Saves the current configuration to the GUI-specific config file
    ///
    /// # Errors
    /// * `ConfigError::PersistenceError` - If the configuration cannot be saved
    // TODO: Implement
    pub fn save_gui_config(&self) -> Result<(), ConfigError> {
        Ok(())
    }
}

/// Checks if a configuration path matches a given pattern
///
/// # Arguments
/// * `path` - The actual configuration path
/// * `pattern` - The pattern to match against (supports "*" as wildcard)
///
/// # Examples
/// * `"server.port"` matches `"server.port"`
/// * `"server.port"` matches `"server.*"`
/// * `"server.port"` matches `"*"`
fn path_matches(path: &str, pattern: &str) -> bool {
    const WILDCARD: &str = "*";

    if pattern == WILDCARD {
        return true;
    };

    let path_parts: Vec<&str> = path.split(".").collect();
    let pattern_parts: Vec<&str> = pattern.split(".").collect();

    for (path_part, pattern_part) in path_parts.iter().zip(pattern_parts.iter()) {
        if pattern_part == &WILDCARD {
            continue;
        }

        if path_part != pattern_part {
            return false;
        }
    }

    true
}

/// Navigates through a TOML value structure following a dot-separated path
///
/// # Arguments
/// * `value` - The root TOML value to navigate from
/// * `path` - Dot-separated path (e.g., "server.port" or "array.0.field")
///
/// # Errors
/// * `ConfigError::InvalidPath` - If the path doesn't exist or is malformed
fn navigate_path(value: &Value, path: &str) -> Result<Value, ConfigError> {
    let parts: Vec<&str> = path.split(".").collect();
    let mut current = value;

    for (i, part) in parts.iter().enumerate() {
        match current {
            Value::Table(table) => {
                current = table.get(*part).ok_or_else(|| {
                    ConfigError::InvalidPath(format!(
                        "Key '{}' not found in table at path '{}'",
                        part,
                        parts[..i].join(".")
                    ))
                })?;
            }
            Value::Array(array) => {
                let index = part.parse::<usize>().map_err(|_| {
                    ConfigError::InvalidPath(format!(
                        "Invalid array index '{}' at path '{}'",
                        part,
                        parts[..i].join(".")
                    ))
                })?;

                current = array.get(index).ok_or_else(|| {
                    ConfigError::InvalidPath(format!(
                        "Array index '{}' out of bounds at path '{}'",
                        index,
                        parts[..i].join(".")
                    ))
                })?;
            }
            _ => {
                return Err(ConfigError::InvalidPath(format!(
                    "Cannot navigate into {:?} at path '{}'",
                    current.type_str(),
                    parts[..i].join("."),
                )));
            }
        }
    }

    Ok(current.clone())
}

/// Sets a value at the specified path within a mutable TOML value structure
///
/// # Arguments
/// * `value` - The root TOML value to modify
/// * `path` - Dot-separated path to the target location
/// * `new_value` - The value to insert at the path
///
/// # Errors
/// * `ConfigError::InvalidPath` - If the path is empty or doesn't exist
fn set_value_at_path(
    value: &mut Value,
    path: &str,
    new_value: Value,
) -> Result<(), ConfigError> {
    let parts: Vec<&str> = path.split('.').collect();

    if parts.is_empty() {
        return Err(ConfigError::InvalidPath("Empty path".to_string()));
    }

    let (parent, last_key) = navigate_to_parent_mut(value, &parts)?;

    insert_value(parent, last_key, new_value)
}

/// Navigates to the parent container of the target element in a mutable TOML structure
///
/// # Arguments
/// * `value` - The root TOML value to navigate
/// * `parts` - Path components split by dots
///
/// # Returns
/// A tuple of (parent container, last key) for insertion
///
/// # Errors
/// * `ConfigError::InvalidPath` - If navigation fails at any point
fn navigate_to_parent_mut<'a>(
    value: &'a mut Value,
    parts: &'a [&'a str],
) -> Result<(&'a mut Value, &'a str), ConfigError> {
    let mut current = value;

    for (i, part) in parts[..parts.len() - 1].iter().enumerate() {
        current = navigate_step_mut(current, part, &parts[..=i])?;
    }

    Ok((current, parts[parts.len() - 1]))
}

/// Performs a single navigation step in a mutable TOML structure
///
/// # Arguments
/// * `current` - The current TOML value
/// * `key` - The key or index to navigate to
/// * `path_so_far` - The path traversed so far (for error messages)
///
/// # Errors
/// * `ConfigError::InvalidPath` - If the key doesn't exist or index is invalid
fn navigate_step_mut<'a>(
    current: &'a mut Value,
    key: &str,
    path_so_far: &[&str],
) -> Result<&'a mut Value, ConfigError> {
    match current {
        Value::Table(table) => table.get_mut(key).ok_or_else(|| {
            ConfigError::InvalidPath(format!(
                "Key '{}' not found at path '{}'",
                key,
                path_so_far.join(".")
            ))
        }),
        Value::Array(arr) => {
            let index = key.parse::<usize>().map_err(|_| {
                ConfigError::InvalidPath(format!(
                    "Invalid array index '{}' at path '{}'",
                    key,
                    path_so_far.join(".")
                ))
            })?;

            arr.get_mut(index).ok_or_else(|| {
                ConfigError::InvalidPath(format!(
                    "Array index {} out of bounds at path '{}'",
                    index,
                    path_so_far.join(".")
                ))
            })
        }
        _ => Err(ConfigError::InvalidPath(format!(
            "Cannot navigate into {} at path '{}'",
            current.type_str(),
            path_so_far.join(".")
        ))),
    }
}

/// Inserts a value into a TOML container (table or array)
///
/// # Arguments
/// * `container` - The container to insert into
/// * `key` - The key (for tables) or index (for arrays)
/// * `new_value` - The value to insert
///
/// # Errors
/// * `ConfigError::InvalidPath` - If the container type doesn't support insertion or index is invalid
fn insert_value(
    container: &mut Value,
    key: &str,
    new_value: Value,
) -> Result<(), ConfigError> {
    match container {
        Value::Table(table) => {
            table.insert(key.to_string(), new_value);
            Ok(())
        }
        Value::Array(arr) => {
            let index = key
                .parse::<usize>()
                .map_err(|_| ConfigError::InvalidPath(format!("Invalid array index '{}'", key)))?;

            arr.get_mut(index)
                .map(|elem| *elem = new_value)
                .ok_or_else(|| {
                    ConfigError::InvalidPath(format!("Array index {} out of bounds", index))
                })
        }
        _ => Err(ConfigError::InvalidPath(format!(
            "Cannot insert into {}",
            container.type_str()
        ))),
    }
}
