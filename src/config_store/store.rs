use std::{
    collections::HashMap,
    fs,
    sync::{Arc, RwLock},
};

use futures::Stream;
use tokio::sync::broadcast;
use toml::Value;

use crate::config::{Config, ConfigPaths};

use super::{
    ChangeSource, ConfigChange, ConfigError,
    path_ops::{navigate_path, path_matches, set_value_at_path},
};

/// Thread-safe configuration store with reactive change notifications.
///
/// Provides centralized configuration management with the ability to
/// read, write, and observe configuration changes across the application.
#[derive(Clone)]
pub struct ConfigStore {
    config: Arc<RwLock<Config>>,
    change_sender: broadcast::Sender<ConfigChange>,
    runtime_config: Arc<RwLock<HashMap<String, Value>>>,
}

#[derive(Clone)]
pub struct ConfigWriter {
    store: ConfigStore,

    source: ChangeSource,
}

impl ConfigStore {
    /// Creates a configuration writer for GUI-originated changes.
    ///
    /// Returns a `ConfigWriter` that automatically tags all changes
    /// as originating from the GUI settings interface.
    pub fn gui_writer(&self) -> ConfigWriter {
        ConfigWriter {
            store: self.clone(),
            source: ChangeSource::Gui,
        }
    }

    /// Creates a configuration writer for CLI-originated changes.
    ///
    /// Returns a `ConfigWriter` that tags all changes with the specific
    /// CLI command that triggered them for audit and debugging purposes.
    ///
    /// # Arguments
    /// * `command` - The CLI command string that initiated the changes
    pub fn cli_writer(&self, command: String) -> ConfigWriter {
        ConfigWriter {
            store: self.clone(),
            source: ChangeSource::CliCommand(command),
        }
    }

    /// Creates a configuration writer for system-originated changes.
    ///
    /// Returns a `ConfigWriter` for internal system operations such as
    /// automatic adjustments, migrations, or default value applications.
    pub fn system_writer(&self) -> ConfigWriter {
        ConfigWriter {
            store: self.clone(),
            source: ChangeSource::System,
        }
    }

    /// Creates a new ConfigStore with default configuration values.
    pub fn with_defaults() -> Self {
        let config = Config::default();
        let (change_sender, _) = broadcast::channel(1000);

        Self {
            config: Arc::new(RwLock::new(config)),
            runtime_config: Arc::new(RwLock::new(HashMap::new())),
            change_sender,
        }
    }

    /// Loads a ConfigStore from the main configuration file.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::PersistenceError` if the configuration file cannot be loaded.
    pub fn load() -> Result<Self, ConfigError> {
        let main_config = ConfigPaths::main_config();
        let config = Config::load_with_imports(&main_config)
            .map_err(|e| ConfigError::PersistenceError(e.to_string()))?;
        let (change_sender, _) = broadcast::channel(1000);

        let runtime_config = Self::load_runtime_config()?;

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            runtime_config: Arc::new(RwLock::new(runtime_config)),
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
    pub fn set_by_path_with_source(
        &self,
        path: &str,
        value: Value,
        source: ChangeSource,
    ) -> Result<(), ConfigError> {
        let old_value = self.get_by_path(path).ok();

        if matches!(source, ChangeSource::Gui | ChangeSource::CliCommand(_)) {
            self.runtime_config
                .write()
                .map_err(|e| {
                    ConfigError::PatternError(format!(
                        "Failed to acquire write lock for runtime_config: {}",
                        e
                    ))
                })?
                .insert(path.to_string(), value.clone());
        }

        {
            let mut config = self.config.write().map_err(|_| {
                ConfigError::PersistenceError("Failed to acquire write lock".into())
            })?;

            self.set_config_field(&mut config, path, &value)?;
        }

        self.save_config()?;

        let change = ConfigChange::new(path.to_string(), old_value, value, source);

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
    pub fn subscribe_to_path(&self, pattern: &str) -> impl Stream<Item = ConfigChange> + Unpin {
        let pattern = pattern.to_string();
        let receiver = self.change_sender.subscribe();

        Box::pin(futures::stream::unfold(receiver, move |mut receiver| {
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
        }))
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

    /// Saves the current configuration to the runtime config file
    ///
    /// # Errors
    /// * `ConfigError::PersistenceError` - If the configuration cannot be saved
    pub fn save_config(&self) -> Result<(), ConfigError> {
        let runtime_config_map = self
            .runtime_config
            .read()
            .map_err(|_| ConfigError::PersistenceError("Failed to acquire read lock".into()))?;

        let mut runtime_value = Value::Table(toml::Table::new());

        for (path, value) in runtime_config_map.iter() {
            set_value_at_path(&mut runtime_value, path, value.clone())?;
        }

        let config_path = ConfigPaths::runtime_config();
        let temp_path = config_path.with_extension("tmp");

        let toml_str = toml::to_string_pretty(&runtime_value)
            .map_err(|e| ConfigError::SerializationError(e.to_string()))?;

        std::fs::write(&temp_path, toml_str)
            .map_err(|e| ConfigError::PersistenceError(e.to_string()))?;

        std::fs::rename(temp_path, config_path)
            .map_err(|e| ConfigError::PersistenceError(e.to_string()))?;

        let main_path = ConfigPaths::main_config();
        let mut main_config_toml = fs::read_to_string(&main_path).map_err(|_| {
            ConfigError::PersistenceError(format!(
                "Failed to persist config. Main config file {} was not found.",
                main_path.to_string_lossy()
            ))
        })?;

        if !main_config_toml.contains("\"@runtime\"") {
            main_config_toml = Self::ensure_runtime_import(&main_config_toml);
            fs::write(main_path, main_config_toml).map_err(|e| {
                ConfigError::PersistenceError(format!(
                    "Failed to add runtime import to main config: {}",
                    e
                ))
            })?;
        }

        Ok(())
    }

    fn ensure_runtime_import(config: &str) -> String {
        let mut doc: Value = toml::from_str(config).unwrap_or_else(|_| {
            let mut table = toml::map::Map::new();
            table.insert("imports".to_string(), Value::Array(vec![]));

            Value::Table(table)
        });

        if let Value::Table(table) = &mut doc {
            let imports = table
                .entry("imports")
                .or_insert_with(|| Value::Array(vec![]));

            if let Value::Array(arr) = imports {
                let runtime_import = Value::String("@runtime".to_string());

                if !arr.iter().any(|v| v.as_str() == Some("@runtime")) {
                    arr.push(runtime_import);
                }
            }
        }

        toml::to_string(&doc).unwrap_or_else(|_| config.to_string())
    }

    fn load_runtime_config() -> Result<HashMap<String, Value>, ConfigError> {
        let runtime_path = ConfigPaths::runtime_config();
        if runtime_path.exists() {
            let runtime_config = fs::read_to_string(&runtime_path).map_err(|e| {
                ConfigError::InvalidPath(format!("Failed to read runtime.toml: {e}"))
            })?;

            let runtime_toml: Value = toml::from_str(&runtime_config).map_err(|e| {
                ConfigError::DeserializationError(format!(
                    "Failed to parse {}: {}",
                    runtime_path.to_string_lossy(),
                    e
                ))
            })?;

            let mut flat_map: HashMap<String, Value> = HashMap::new();

            Self::flatten_toml_to_paths(&runtime_toml, "", &mut flat_map);
            Ok(flat_map)
        } else {
            Ok(HashMap::new())
        }
    }

    fn flatten_toml_to_paths(value: &Value, prefix: &str, map: &mut HashMap<String, Value>) {
        match value {
            Value::Table(table) => {
                for (key, value) in table {
                    let path = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", prefix, key)
                    };
                    Self::flatten_toml_to_paths(value, &path, map);
                }
            }
            _ => {
                map.insert(prefix.to_string(), value.clone());
            }
        }
    }
}

impl ConfigWriter {
    /// Sets a configuration value at the specified path
    ///
    /// The change source is automatically included from the writer's context.
    ///
    /// # Arguments
    /// * `path` - Dot-separated path to the configuration field
    /// * `value` - The new TOML value to set
    ///
    /// # Errors
    /// Returns errors from the underlying ConfigStore::set_by_path operation
    pub fn set(&self, path: &str, value: Value) -> Result<(), ConfigError> {
        self.store
            .set_by_path_with_source(path, value, self.source.clone())
    }
}
