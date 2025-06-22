mod circular_detection;
mod file_creation;
mod merging;

use super::Config;
use crate::{Result, WayleError};
use circular_detection::CircularDetector;
use file_creation::create_default_config_file;
use merging::merge_toml_configs;
use std::{
    fs,
    path::{Path, PathBuf},
};

impl Config {
    /// Loads a configuration file with support for importing other TOML files
    ///
    /// Import paths are specified using the `@` prefix in the TOML file.
    /// Imported configurations are merged with the main configuration,
    /// with the main configuration taking precedence in case of conflicts.
    /// Also checks for circular imports.
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
    /// - Circular imports are detected
    ///
    /// # Example
    ///
    /// ```rust
    /// use wayle::config::Config;
    /// use std::path::Path;
    ///
    /// let config = Config::load_with_imports(Path::new("config.toml"))?;
    /// ```
    pub fn load_with_imports(path: &Path) -> Result<Config> {
        if !path.exists() {
            create_default_config_file(path)?;
        }

        let canonical_path = path.canonicalize().map_err(|e| {
            WayleError::Config(format!("Failed to resolve path {}: {}", path.display(), e))
        })?;

        let mut detector = CircularDetector::new();
        Self::load_config_with_tracking(&canonical_path, &mut detector)
    }

    fn load_config_with_tracking(path: &Path, detector: &mut CircularDetector) -> Result<Config> {
        detector.detect_circular_import(path)?;
        detector.push_to_chain(path);

        let result = Self::load_main_config(path, detector);
        detector.pop_from_chain();
        result
    }

    fn load_main_config(path: &Path, detector: &mut CircularDetector) -> Result<Config> {
        let main_config_content = fs::read_to_string(path)?;
        let import_paths = Self::extract_import_paths(&main_config_content)?;
        let imported_configs = Self::load_all_imports(path, &import_paths, detector)?;

        let main_config: toml::Value = toml::from_str(&main_config_content)
            .map_err(|e| WayleError::toml_parse(e, Some(path)))?;

        let merged_config = merge_toml_configs(imported_configs, main_config);
        merged_config
            .try_into()
            .map_err(|e| WayleError::Config(format!("Configuration validation failed: {e}")))
    }

    fn load_all_imports(
        base_path: &Path,
        import_paths: &[String],
        detector: &mut CircularDetector,
    ) -> Result<Vec<toml::Value>> {
        import_paths
            .iter()
            .map(|import_path| {
                let resolved_path = Self::resolve_import_path(base_path, import_path)?;
                let canonical_import = resolved_path
                    .canonicalize()
                    .map_err(|e| WayleError::import(e, &resolved_path))?;

                Self::load_imported_file_with_tracking(&canonical_import, detector)
            })
            .collect()
    }

    fn load_imported_file_with_tracking(
        path: &Path,
        detector: &mut CircularDetector,
    ) -> Result<toml::Value> {
        detector.detect_circular_import(path)?;
        detector.push_to_chain(path);

        let result = Self::load_toml_file_with_imports(path, detector);
        detector.pop_from_chain();
        result
    }

    fn load_toml_file_with_imports(
        path: &Path,
        detector: &mut CircularDetector,
    ) -> Result<toml::Value> {
        let content = fs::read_to_string(path).map_err(|e| WayleError::import(e, path))?;
        let import_paths = Self::extract_import_paths(&content)?;
        let imported_configs = Self::load_all_imports(path, &import_paths, detector)?;

        let main_value: toml::Value =
            toml::from_str(&content).map_err(|e| WayleError::toml_parse(e, Some(path)))?;

        Ok(merge_toml_configs(imported_configs, main_value))
    }

    fn extract_import_paths(config_content: &str) -> Result<Vec<String>> {
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
}
