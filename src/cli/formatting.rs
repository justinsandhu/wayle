//! Formatting utilities for CLI output.
//!
//! Provides consistent formatting for configuration values and beautiful
//! help text with colors and styled output for CLI commands.

use toml::Value;

/// ANSI color codes for terminal output
pub struct Colors;

impl Colors {
    /// Reset all formatting
    pub const RESET: &'static str = "\x1b[0m";
    /// Bold text
    pub const BOLD: &'static str = "\x1b[1m";
    /// Dim text
    pub const DIM: &'static str = "\x1b[2m";

    /// Red color
    pub const RED: &'static str = "\x1b[31m";
    /// Green color
    pub const GREEN: &'static str = "\x1b[32m";
    /// Yellow color
    pub const YELLOW: &'static str = "\x1b[33m";
    /// Blue color
    pub const BLUE: &'static str = "\x1b[34m";
    /// Magenta color
    pub const MAGENTA: &'static str = "\x1b[35m";
    /// Cyan color
    pub const CYAN: &'static str = "\x1b[36m";
    /// White color
    pub const WHITE: &'static str = "\x1b[37m";

    /// Bright black color
    pub const BRIGHT_BLACK: &'static str = "\x1b[90m";
    /// Bright red color
    pub const BRIGHT_RED: &'static str = "\x1b[91m";
    /// Bright green color
    pub const BRIGHT_GREEN: &'static str = "\x1b[92m";
    /// Bright yellow color
    pub const BRIGHT_YELLOW: &'static str = "\x1b[93m";
    /// Bright blue color
    pub const BRIGHT_BLUE: &'static str = "\x1b[94m";
    /// Bright magenta color
    pub const BRIGHT_MAGENTA: &'static str = "\x1b[95m";
    /// Bright cyan color
    pub const BRIGHT_CYAN: &'static str = "\x1b[96m";
    /// Bright white color
    pub const BRIGHT_WHITE: &'static str = "\x1b[97m";
}

/// Formats section headers with styling
pub fn format_header(text: &str) -> String {
    format!("{}{}{}{}", Colors::BOLD, Colors::CYAN, text, Colors::RESET)
}

/// Formats subheaders with styling
pub fn format_subheader(text: &str) -> String {
    format!(
        "{}{}{}{}",
        Colors::BOLD,
        Colors::YELLOW,
        text,
        Colors::RESET
    )
}

/// Formats command names with styling
pub fn format_command(text: &str) -> String {
    format!("{}{}{}{}", Colors::BOLD, Colors::GREEN, text, Colors::RESET)
}

/// Formats category names with styling
pub fn format_category(text: &str) -> String {
    format!("{}{}{}{}", Colors::BOLD, Colors::BLUE, text, Colors::RESET)
}

/// Formats descriptions with muted styling
pub fn format_description(text: &str) -> String {
    format!("{}{}{}", Colors::DIM, text, Colors::RESET)
}

/// Formats usage examples with styling
pub fn format_usage(text: &str) -> String {
    format!("{}{}{}", Colors::DIM, text, Colors::RESET)
}

/// Formats error messages with red styling
pub fn format_error(text: &str) -> String {
    format!("{}{}{}{}", Colors::BOLD, Colors::RED, text, Colors::RESET)
}

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
/// use wayle::cli::formatting::format_toml_value;
///
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
