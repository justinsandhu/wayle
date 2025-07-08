//! Unit tests for config_store module
//! No filesystem, timing, or external dependencies.

#![allow(clippy::panic)]

use core::f64;

use crate::config_store::{ConfigChange, ConfigError, ConfigStore};
use toml::Value;

#[test]
fn config_change_new() {
    let change = ConfigChange::new(
        "test.path".to_string(),
        Some(Value::String("old".to_string())),
        Value::String("new".to_string()),
    );

    assert_eq!(change.path, "test.path");
    assert_eq!(change.old_value, Some(Value::String("old".to_string())));
    assert_eq!(change.new_value, Value::String("new".to_string()));
    assert!(change.timestamp.elapsed().as_secs() < 1);
}

#[test]
fn config_change_new_field() {
    let change = ConfigChange::new("modules.new_module".to_string(), None, Value::Boolean(true));

    assert_eq!(change.path, "modules.new_module");
    assert_eq!(change.old_value, None);
    assert_eq!(change.new_value, Value::Boolean(true));
}

#[test]
fn config_change_value_types() {
    let change = ConfigChange::new(
        "string_field".to_string(),
        Some(Value::String("old".to_string())),
        Value::String("new".to_string()),
    );
    assert_eq!(change.new_value, Value::String("new".to_string()));

    let change = ConfigChange::new(
        "bool_field".to_string(),
        Some(Value::Boolean(false)),
        Value::Boolean(true),
    );
    assert_eq!(change.old_value, Some(Value::Boolean(false)));
    assert_eq!(change.new_value, Value::Boolean(true));

    let change = ConfigChange::new(
        "int_field".to_string(),
        Some(Value::Integer(10)),
        Value::Integer(42),
    );
    assert_eq!(change.old_value, Some(Value::Integer(10)));
    assert_eq!(change.new_value, Value::Integer(42));

    let change = ConfigChange::new(
        "float_field".to_string(),
        None,
        Value::Float(f64::consts::PI),
    );
    assert_eq!(change.old_value, None);
    assert_eq!(change.new_value, Value::Float(f64::consts::PI));
}

#[test]
fn config_error_variants() {
    let error = ConfigError::InvalidPath("test.invalid.path".to_string());
    assert!(matches!(error, ConfigError::InvalidPath(_)));

    let error = ConfigError::LockError {
        lock_type: "read".to_string(),
        details: "lock poisoned".to_string(),
    };
    assert!(matches!(error, ConfigError::LockError { .. }));

    let error = ConfigError::SerializationError {
        content_type: "toml".to_string(),
        details: "invalid syntax".to_string(),
    };
    assert!(matches!(error, ConfigError::SerializationError { .. }));

    let error = ConfigError::ConversionError {
        from: "Value".to_string(),
        to: "Config".to_string(),
        details: "type mismatch".to_string(),
    };
    assert!(matches!(error, ConfigError::ConversionError { .. }));
}

#[tokio::test]
async fn config_store_with_defaults() {
    let store = ConfigStore::with_defaults();
    let config = store.get_current();

    assert!(!format!("{:?}", config.general).is_empty());
    assert!(!format!("{:?}", config.modules).is_empty());
}

#[tokio::test]
async fn config_store_clone() {
    let store1 = ConfigStore::with_defaults();
    let store2 = store1.clone();

    let config1 = store1.get_current();
    let config2 = store2.get_current();

    assert_eq!(format!("{config1:?}"), format!("{config2:?}"));
}

#[tokio::test]
async fn config_store_subscription() {
    let store = ConfigStore::with_defaults();

    let mut sub1 = store.subscribe_to_path("general.*").await.unwrap();
    let mut sub2 = store.subscribe_to_path("modules.*").await.unwrap();
    let mut sub3 = store.subscribe_to_path("*").await.unwrap();

    assert!(sub1.receiver_mut().try_recv().is_err());
    assert!(sub2.receiver_mut().try_recv().is_err());
    assert!(sub3.receiver_mut().try_recv().is_err());
}

#[test]
fn toml_value_operations() {
    let mut table = toml::map::Map::new();
    table.insert("enabled".to_string(), Value::Boolean(true));
    table.insert("count".to_string(), Value::Integer(5));
    table.insert("name".to_string(), Value::String("test".to_string()));

    let value = Value::Table(table);

    if let Value::Table(table) = &value {
        assert_eq!(table.get("enabled"), Some(&Value::Boolean(true)));
        assert_eq!(table.get("count"), Some(&Value::Integer(5)));
        assert_eq!(table.get("name"), Some(&Value::String("test".to_string())));
        assert_eq!(table.get("missing"), None);
        assert_eq!(table.len(), 3);
    } else {
        panic!("Expected table value");
    }
}

#[test]
fn toml_nested_structure() {
    let mut battery_table = toml::map::Map::new();
    battery_table.insert("enabled".to_string(), Value::Boolean(true));

    let mut modules_table = toml::map::Map::new();
    modules_table.insert("battery".to_string(), Value::Table(battery_table));

    let mut general_table = toml::map::Map::new();
    general_table.insert("log_level".to_string(), Value::String("debug".to_string()));

    let mut root_table = toml::map::Map::new();
    root_table.insert("general".to_string(), Value::Table(general_table));
    root_table.insert("modules".to_string(), Value::Table(modules_table));

    let root_value = Value::Table(root_table);

    if let Value::Table(root) = &root_value {
        if let Some(Value::Table(general)) = root.get("general") {
            assert_eq!(
                general.get("log_level"),
                Some(&Value::String("debug".to_string()))
            );
        } else {
            panic!("Expected general table");
        }

        if let Some(Value::Table(modules)) = root.get("modules") {
            if let Some(Value::Table(battery)) = modules.get("battery") {
                assert_eq!(battery.get("enabled"), Some(&Value::Boolean(true)));
            } else {
                panic!("Expected battery table");
            }
        } else {
            panic!("Expected modules table");
        }
    } else {
        panic!("Expected root table");
    }
}

#[test]
fn toml_value_type_checking() {
    let string_val = Value::String("test".to_string());
    let bool_val = Value::Boolean(true);
    let int_val = Value::Integer(42);
    let float_val = Value::Float(f64::consts::PI);
    let array_val = Value::Array(vec![Value::Integer(1), Value::Integer(2)]);
    let table_val = Value::Table(toml::map::Map::new());

    assert!(string_val.is_str());
    assert!(bool_val.is_bool());
    assert!(int_val.is_integer());
    assert!(float_val.is_float());
    assert!(array_val.is_array());
    assert!(table_val.is_table());

    assert_eq!(string_val.as_str(), Some("test"));
    assert_eq!(bool_val.as_bool(), Some(true));
    assert_eq!(int_val.as_integer(), Some(42));
    assert_eq!(float_val.as_float(), Some(f64::consts::PI));
    assert_eq!(array_val.as_array().unwrap().len(), 2);
    assert_eq!(table_val.as_table().unwrap().len(), 0);

    assert_eq!(string_val.as_bool(), None);
    assert_eq!(bool_val.as_integer(), None);
    assert_eq!(int_val.as_str(), None);
}

#[test]
fn toml_array_operations() {
    let array = vec![
        Value::String("first".to_string()),
        Value::String("second".to_string()),
        Value::Integer(42),
    ];

    let array_value = Value::Array(array);

    if let Value::Array(arr) = &array_value {
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0], Value::String("first".to_string()));
        assert_eq!(arr[1], Value::String("second".to_string()));
        assert_eq!(arr[2], Value::Integer(42));

        let mut count = 0;
        for item in arr {
            count += 1;
            assert!(item.is_str() || item.is_integer());
        }
        assert_eq!(count, 3);
    } else {
        panic!("Expected array value");
    }
}

#[tokio::test]
async fn subscription_raii_cleanup() {
    let store = ConfigStore::with_defaults();

    {
        let _sub1 = store.subscribe_to_path("general.*").await.unwrap();
        let _sub2 = store.subscribe_to_path("modules.*").await.unwrap();
    }

    let _sub3 = store.subscribe_to_path("test.*").await.unwrap();
}
