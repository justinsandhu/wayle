use crate::config::Config;
use std::collections::HashSet;
use std::sync::OnceLock;
use std::time::Instant;

use super::{ChangeSource, ConfigChange, ConfigError, path_ops::navigate_path};

static DEFAULT_CONFIG: OnceLock<toml::Value> = OnceLock::new();

/// Compares two configurations and identifies specific field-level changes.
///
/// Performs a recursive comparison of configuration structures by converting them
/// to TOML and analyzing differences at the field level. When fields are removed
/// from the new config, their default values are automatically provided.
///
/// # Arguments
/// * `old` - The previous configuration state
/// * `new` - The new configuration state to compare against
/// * `source` - The source that triggered this configuration change
///
/// # Errors
/// Returns error if either configuration cannot be serialized to TOML format.
pub fn diff_configs(
    old: &Config,
    new: &Config,
    source: ChangeSource,
) -> Result<Vec<ConfigChange>, Box<dyn std::error::Error>> {
    let old_toml = toml::to_string(old)?;
    let new_toml = toml::to_string(new)?;

    let old_value: toml::Value = toml::from_str(&old_toml)?;
    let new_value: toml::Value = toml::from_str(&new_toml)?;

    let changes = diff_toml_values("", &old_value, &new_value, source, Instant::now());

    Ok(changes)
}

fn diff_toml_values(
    path: &str,
    old: &toml::Value,
    new: &toml::Value,
    source: ChangeSource,
    timestamp: Instant,
) -> Vec<ConfigChange> {
    use toml::Value;

    let mut changes = Vec::new();

    match (old, new) {
        (Value::Table(old_table), Value::Table(new_table)) => {
            let mut all_keys = HashSet::new();
            all_keys.extend(old_table.keys());
            all_keys.extend(new_table.keys());

            for key in all_keys {
                let field_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", path, key)
                };

                match (old_table.get(key), new_table.get(key)) {
                    (Some(old_val), Some(new_val)) => {
                        changes.extend(diff_toml_values(
                            &field_path,
                            old_val,
                            new_val,
                            source.clone(),
                            timestamp,
                        ));
                    }
                    (Some(old_val), None) => {
                        if let Ok(default_value) = get_default_for_path(&field_path) {
                            changes.push(ConfigChange {
                                path: field_path,
                                old_value: Some(old_val.clone()),
                                new_value: default_value,
                                timestamp,
                                source: source.clone(),
                            });
                        }
                    }
                    (None, Some(new_val)) => {
                        changes.push(ConfigChange {
                            path: field_path,
                            old_value: None,
                            new_value: new_val.clone(),
                            timestamp,
                            source: source.clone(),
                        });
                    }
                    (None, None) => unreachable!(),
                }
            }
        }
        _ => {
            if old != new {
                changes.push(ConfigChange {
                    path: path.to_string(),
                    old_value: Some(old.clone()),
                    new_value: new.clone(),
                    timestamp,
                    source,
                });
            }
        }
    }

    changes
}

#[allow(clippy::expect_used)]
fn get_default_config() -> &'static toml::Value {
    DEFAULT_CONFIG.get_or_init(|| {
        let default_config = Config::default();
        let default_toml = toml::to_string(&default_config)
            .expect("Config::default() must serialize to valid TOML");

        toml::from_str(&default_toml).expect("Config::default() serialization must be valid TOML")
    })
}

fn get_default_for_path(path: &str) -> Result<toml::Value, ConfigError> {
    let default_config = get_default_config();
    navigate_path(default_config, path)
}
