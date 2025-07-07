//! Unit tests for CLI module
//!
//! Tests command registry, formatting, and CLI utilities.
//! No external dependencies or actual command execution.

use crate::cli::{CommandRegistry, formatting::format_toml_value};
use crate::config_store::ConfigStore;
use core::f64;
use std::sync::Arc;
use toml::Value;

#[test]
fn format_toml_value_string() {
    let value = Value::String("hello world".to_string());
    assert_eq!(format_toml_value(&value), "\"hello world\"");

    let value = Value::String("".to_string());
    assert_eq!(format_toml_value(&value), "\"\"");

    let value = Value::String("with \"quotes\"".to_string());
    assert_eq!(format_toml_value(&value), "\"with \"quotes\"\"");
}

#[test]
fn format_toml_value_integer() {
    let value = Value::Integer(42);
    assert_eq!(format_toml_value(&value), "42");

    let value = Value::Integer(0);
    assert_eq!(format_toml_value(&value), "0");

    let value = Value::Integer(-123);
    assert_eq!(format_toml_value(&value), "-123");
}

#[test]
fn format_toml_value_float() {
    let value = Value::Float(f64::consts::PI);
    assert_eq!(format_toml_value(&value), "3.141592653589793");

    let value = Value::Float(0.0);
    assert_eq!(format_toml_value(&value), "0");

    let value = Value::Float(-2.5);
    assert_eq!(format_toml_value(&value), "-2.5");
}

#[test]
fn format_toml_value_boolean() {
    let value = Value::Boolean(true);
    assert_eq!(format_toml_value(&value), "true");

    let value = Value::Boolean(false);
    assert_eq!(format_toml_value(&value), "false");
}

#[test]
fn format_toml_value_array() {
    let value = Value::Array(vec![
        Value::Integer(1),
        Value::Integer(2),
        Value::Integer(3),
    ]);
    assert_eq!(format_toml_value(&value), "[3]");

    let value = Value::Array(vec![
        Value::String("a".to_string()),
        Value::String("b".to_string()),
    ]);
    assert_eq!(format_toml_value(&value), "[2]");

    let value = Value::Array(vec![]);
    assert_eq!(format_toml_value(&value), "[0]");
}

#[test]
fn format_toml_value_table() {
    let mut table = toml::map::Map::new();
    table.insert("key1".to_string(), Value::String("value1".to_string()));
    table.insert("key2".to_string(), Value::Integer(42));

    let value = Value::Table(table);
    let formatted = format_toml_value(&value);

    assert_eq!(formatted, "{2}");
}

#[test]
fn format_toml_value_nested_structures() {
    let value = Value::Array(vec![
        Value::String("text".to_string()),
        Value::Integer(123),
        Value::Boolean(true),
    ]);
    assert_eq!(format_toml_value(&value), "[3]");

    let value = Value::Array(vec![
        Value::Array(vec![Value::Integer(1), Value::Integer(2)]),
        Value::Array(vec![Value::Integer(3), Value::Integer(4)]),
    ]);
    assert_eq!(format_toml_value(&value), "[2]");
}

#[tokio::test]
async fn command_registry_creation() {
    let config_store = Arc::new(ConfigStore::with_defaults());
    let registry = CommandRegistry::new(config_store);

    drop(registry);
}

#[tokio::test]
async fn command_registry_categories() {
    let config_store = Arc::new(ConfigStore::with_defaults());
    let registry = CommandRegistry::new(config_store);

    let categories = registry.get_categories();
    assert!(categories.is_empty());
}

#[test]
fn format_toml_value_edge_cases() {
    let value = Value::Integer(i64::MAX);
    let formatted = format_toml_value(&value);
    assert!(formatted.parse::<i64>().is_ok());

    let value = Value::Integer(i64::MIN);
    let formatted = format_toml_value(&value);
    assert!(formatted.parse::<i64>().is_ok());

    let value = Value::Float(f64::INFINITY);
    let formatted = format_toml_value(&value);
    assert!(formatted.contains("inf") || formatted.contains("Infinity"));

    let value = Value::Float(f64::NEG_INFINITY);
    let formatted = format_toml_value(&value);
    assert!(formatted.contains("-inf") || formatted.contains("-Infinity"));

    let value = Value::Float(f64::NAN);
    let formatted = format_toml_value(&value);
    assert!(!formatted.is_empty());
}

#[test]
fn format_toml_value_empty_table() {
    let table = toml::map::Map::new();
    let value = Value::Table(table);
    let formatted = format_toml_value(&value);

    assert_eq!(formatted, "{0}");
}

#[test]
fn format_toml_value_special_strings() {
    let value = Value::String("line1\nline2".to_string());
    let formatted = format_toml_value(&value);
    assert!(formatted.contains("line1"));
    assert!(formatted.contains("line2"));

    let value = Value::String("col1\tcol2".to_string());
    let formatted = format_toml_value(&value);
    assert!(formatted.contains("col1"));
    assert!(formatted.contains("col2"));

    let value = Value::String("Hello 世界 🌍".to_string());
    let formatted = format_toml_value(&value);
    assert!(formatted.contains("Hello"));
    assert!(formatted.contains("世界"));
    assert!(formatted.contains("🌍"));
}

#[test]
fn format_toml_value_mixed_array() {
    let value = Value::Array(vec![
        Value::String("text".to_string()),
        Value::Integer(42),
        Value::Boolean(false),
        Value::Float(f64::consts::PI),
    ]);

    let formatted = format_toml_value(&value);

    assert_eq!(formatted, "[4]");
}

#[test]
fn format_toml_value_deeply_nested() {
    let mut inner_table = toml::map::Map::new();
    inner_table.insert("key".to_string(), Value::String("value".to_string()));

    let mut middle_table = toml::map::Map::new();
    middle_table.insert("subsection".to_string(), Value::Table(inner_table));

    let mut outer_table = toml::map::Map::new();
    outer_table.insert("section".to_string(), Value::Table(middle_table));

    let value = Value::Table(outer_table);
    let formatted = format_toml_value(&value);

    assert_eq!(formatted, "{1}");
}
