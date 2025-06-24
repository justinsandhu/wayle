//! Formatting utilities for CLI output.
//!
//! Provides consistent formatting for configuration values when
//! displaying them to users in CLI commands.

use toml::Value;

/// Formats a TOML value for human-readable CLI output.
///
/// Converts TOML values into string representations suitable for
/// display in command-line interfaces. Complex types like arrays
/// and tables show their size rather than full contents.
///
/// # Arguments
///
/// * `value` - The TOML value to format
///
/// # Examples
///
/// ```
/// let value = toml::Value::String("hello".to_string());
/// assert_eq!(format_toml_value(&value), "\"hello\"");
///
/// let value = toml::Value::Integer(42);
/// assert_eq!(format_toml_value(&value), "42");
/// ```
pub fn format_toml_value(value: &Value) -> String {
    match value {
        Value::String(s) => format!("\"{}\"", s),
        Value::Integer(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Boolean(b) => b.to_string(),
        Value::Array(arr) => format!("[{}]", arr.len()),
        Value::Table(table) => format!("{{{}}}", table.len()),
        _ => "complex_value".to_string(),
    }
}