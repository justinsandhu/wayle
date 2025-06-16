use super::Config;

use crate::{Result, WayleError};

use std::{
    fs,
    path::{Path, PathBuf},
};

impl Config {
    pub fn load_with_imports(path: &Path) -> Result<Config> {
        let file_content = fs::read_to_string(path)?;
        let imports = Self::extract_imports(&file_content)?;
        let imported_tomls: Result<Vec<toml::Value>> = imports
            .iter()
            .map(|import_path| {
                let resolved_path = Self::resolve_import_path(path, import_path)?;
                Self::load_import_file(&resolved_path)
            })
            .collect();
        let imported_tomls = imported_tomls?;

        let main_toml: toml::Value =
            toml::from_str(&file_content).map_err(|e| WayleError::toml_parse(e, Some(path)))?;

        let merged_toml = Self::merge_toml_values(imported_tomls, main_toml);
        let config: Config = merged_toml.try_into().map_err(|e| {
            WayleError::Config(format!(
                "Invalid configuration after merging imports: {}",
                e
            ))
        })?;

        Ok(config)
    }

    fn merge_toml_values(toml_list: Vec<toml::Value>, main_toml: toml::Value) -> toml::Value {
        let mut accumulated = toml::Value::Table(toml::map::Map::new());

        for import_toml in toml_list {
            accumulated = Self::merge_two_toml_values(accumulated, import_toml);
        }

        Self::merge_two_toml_values(accumulated, main_toml)
    }

    fn merge_two_toml_values(base: toml::Value, overlay: toml::Value) -> toml::Value {
        match (base, overlay) {
            (toml::Value::Table(base_table), toml::Value::Table(overlay_table)) => {
                let mut merged_table = overlay_table;

                for (key, base_value) in base_table {
                    if !merged_table.contains_key(&key) {
                        merged_table.insert(key, base_value);
                    } else {
                        let overlay_value = merged_table.remove(&key).unwrap();
                        let merged_value = Self::merge_two_toml_values(base_value, overlay_value);
                        merged_table.insert(key, merged_value);
                    }
                }

                toml::Value::Table(merged_table)
            }

            (_, overlay) => overlay,
        }
    }

    fn extract_imports(toml_content: &str) -> Result<Vec<String>> {
        let value = toml::from_str(toml_content).map_err(|e| {
            WayleError::TomlParse(format!(
                "Failed to parse toml while extracting import paths. Error: {}",
                e
            ))
        })?;

        let import_paths = if let toml::Value::Table(table) = value {
            table
                .keys()
                .filter_map(|key| key.strip_prefix('@'))
                .map(|path| path.to_string())
                .collect::<Vec<String>>()
        } else {
            Vec::new()
        };

        Ok(import_paths)
    }

    // fn resolve_import_paths(
    //     base_config_path: &Path,
    //     import_paths: Vec<String>,
    // ) -> Result<Vec<PathBuf>> {
    // }

    pub fn resolve_import_path(base_path: &Path, import_path: &str) -> Result<PathBuf> {
        let parent = base_path.parent().ok_or_else(|| {
            let formatted_error = format!("Invalid base path: {:?}", base_path);
            WayleError::Import(formatted_error)
        })?;

        let mut import_pathbuf = PathBuf::from(import_path);
        if import_pathbuf.extension().is_none() {
            import_pathbuf.set_extension("toml");
        }

        let resolved_path = parent.join(import_pathbuf);

        Ok(resolved_path)
    }

    pub fn load_import_file(path: &Path) -> Result<toml::Value> {
        let file_content = fs::read_to_string(path).map_err(|e| WayleError::import(e, path))?;

        toml::from_str(&file_content).map_err(|e| WayleError::toml_parse(e, Some(path)))
    }
}
