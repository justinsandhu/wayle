use super::Config;
use crate::{Result, WayleError};
use std::{
    fs,
    path::{Path, PathBuf},
};

impl Config {
    /// Loads a configuration file with support for importing other TOML files.
    ///
    /// Import paths are specified using the `@` prefix in the TOML file.
    /// Imported configurations are merged with the main configuration,
    /// with the main configuration taking precedence in case of conflicts.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the main configuration file
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The configuration file cannot be read
    /// - The TOML content is invalid
    /// - Any imported files cannot be loaded
    /// - The merged configuration is invalid
    ///
    /// # Example
    ///
    /// ```rust
    /// use wayle::config::Config;
    /// use std::path::Path;
    ///
    /// let config = Config::load_with_imports(Path::new("config.toml"))?;
    /// ```
    /// FIX: Circular imports
    pub fn load_with_imports(path: &Path) -> Result<Config> {
        let main_config_content = fs::read_to_string(path)?;
        let imports = Self::extract_imports(&main_config_content)?;

        let imported_configs: Result<Vec<toml::Value>> = imports
            .iter()
            .map(|import_path| {
                let resolved_path = Self::resolve_import_path(path, import_path)?;
                Self::load_import_file(&resolved_path)
            })
            .collect();
        let imported_configs = imported_configs?;

        let main_config: toml::Value = toml::from_str(&main_config_content)
            .map_err(|e| WayleError::toml_parse(e, Some(path)))?;

        let merged_config = Self::merge_toml_values(imported_configs, main_config);
        let config: Config = merged_config
            .try_into()
            .map_err(|e| WayleError::Config(format!("Configuration validation failed: {e}")))?;

        Ok(config)
    }

    fn extract_imports(config_content: &str) -> Result<Vec<String>> {
        const IMPORT_PREFIX: char = '@';

        let value = toml::from_str(config_content).map_err(|e| WayleError::toml_parse(e, None))?;

        let import_paths = if let toml::Value::Table(table) = value {
            table
                .keys()
                .filter_map(|key| key.strip_prefix(IMPORT_PREFIX))
                .map(|path| path.to_owned())
                .collect::<Vec<String>>()
        } else {
            Vec::new()
        };

        Ok(import_paths)
    }

    fn resolve_import_path(base_path: &Path, import_path: &str) -> Result<PathBuf> {
        let parent_dir = base_path.parent().ok_or_else(|| {
            let error_msg = format!("Invalid base path: {base_path:?}");
            WayleError::Import(error_msg)
        })?;

        let mut import_path_buf = PathBuf::from(import_path);
        if import_path_buf.extension().is_none() {
            import_path_buf.set_extension("toml");
        }

        let resolved_path = parent_dir.join(import_path_buf);
        Ok(resolved_path)
    }

    fn load_import_file(path: &Path) -> Result<toml::Value> {
        let content = fs::read_to_string(path).map_err(|e| WayleError::import(e, path))?;
        toml::from_str(&content).map_err(|e| WayleError::toml_parse(e, Some(path)))
    }

    fn merge_toml_values(imports: Vec<toml::Value>, main: toml::Value) -> toml::Value {
        let mut accumulated = toml::Value::Table(toml::map::Map::new());

        for import in imports {
            accumulated = Self::merge_two_toml_values(accumulated, import);
        }

        Self::merge_two_toml_values(accumulated, main)
    }

    /// Deep merges TOML tables while preserving precedence.
    ///
    /// We start with overlay as base, then selectively add missing
    /// keys from base. This ensures overlay values always win, but we don't lose
    /// base values that aren't being overridden. For non-table values, overlay
    /// completely replaces base (no attempt to merge primitives).
    fn merge_two_toml_values(base: toml::Value, overlay: toml::Value) -> toml::Value {
        match (base, overlay) {
            (toml::Value::Table(base_table), toml::Value::Table(overlay_table)) => {
                let mut merged_table = overlay_table;

                for (key, base_value) in base_table {
                    match merged_table.remove(&key) {
                        None => {
                            merged_table.insert(key, base_value);
                        }
                        Some(overlay_value) => {
                            let merged_value =
                                Self::merge_two_toml_values(base_value, overlay_value);
                            merged_table.insert(key, merged_value);
                        }
                    }
                }

                toml::Value::Table(merged_table)
            }
            (_, overlay) => overlay,
        }
    }
}
