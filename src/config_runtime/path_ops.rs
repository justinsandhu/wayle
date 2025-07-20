use toml::Value;

use super::ConfigError;

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
pub(super) fn path_matches(path: &str, pattern: &str) -> bool {
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
pub(super) fn navigate_path(value: &Value, path: &str) -> Result<Value, ConfigError> {
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
pub(super) fn set_value_at_path(
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
pub(super) fn navigate_to_parent_mut<'a>(
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
pub(super) fn navigate_step_mut<'a>(
    current: &'a mut Value,
    key: &str,
    path_so_far: &[&str],
) -> Result<&'a mut Value, ConfigError> {
    match current {
        Value::Table(table) => {
            if !table.contains_key(key) {
                table.insert(key.to_string(), Value::Table(toml::map::Map::new()));
            }

            table.get_mut(key).ok_or_else(|| {
                ConfigError::InvalidPath(format!(
                    "Key '{}' not found at path '{}'",
                    key,
                    path_so_far.join(".")
                ))
            })
        }
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
pub(super) fn insert_value(
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
                .map_err(|_| ConfigError::InvalidPath(format!("Invalid array index '{key}'")))?;

            arr.get_mut(index)
                .map(|elem| *elem = new_value)
                .ok_or_else(|| {
                    ConfigError::InvalidPath(format!("Array index {index} out of bounds"))
                })
        }
        _ => Err(ConfigError::InvalidPath(format!(
            "Cannot insert into {}",
            container.type_str()
        ))),
    }
}
