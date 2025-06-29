use crate::config_store::{ChangeSource, ConfigChange, ConfigError};
use std::f64::consts;
use toml::Value;

#[cfg(test)]
mod tests {
    use super::*;
    use Value;

    #[test]
    fn config_change_creation() {
        let change = ConfigChange::new(
            "modules.clock.general.format".to_string(),
            Some(Value::String("%H:%M".to_string())),
            Value::String("%I:%M %p".to_string()),
            ChangeSource::Gui,
        );

        assert_eq!(change.path, "modules.clock.general.format");
        assert_eq!(change.old_value, Some(Value::String("%H:%M".to_string())));
        assert_eq!(change.new_value, Value::String("%I:%M %p".to_string()));
        assert_eq!(change.source, ChangeSource::Gui);
        assert!(change.timestamp.elapsed().as_secs() < 1);
    }

    #[test]
    fn extract_string_success() {
        let change = ConfigChange::new(
            "test.path".to_string(),
            None,
            Value::String("test_value".to_string()),
            ChangeSource::FileEdit,
        );

        let result: Result<String, _> = change.extract();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test_value");
    }

    #[test]
    fn extract_boolean_success() {
        let change = ConfigChange::new(
            "test.boolean".to_string(),
            None,
            Value::Boolean(true),
            ChangeSource::FileReload,
        );

        let result: Result<bool, _> = change.extract();
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn extract_integer_success() {
        let change = ConfigChange::new(
            "test.number".to_string(),
            None,
            Value::Integer(42),
            ChangeSource::CliCommand("set level 42".to_string()),
        );

        let result: Result<i64, _> = change.extract();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn extract_float_success() {
        let change = ConfigChange::new(
            "test.float".to_string(),
            None,
            Value::Float(consts::PI),
            ChangeSource::Ipc,
        );

        let result: Result<f64, _> = change.extract();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), consts::PI);
    }

    #[test]
    fn extract_type_mismatch() {
        let change = ConfigChange::new(
            "test.mismatch".to_string(),
            None,
            Value::String("not_a_number".to_string()),
            ChangeSource::Gui,
        );

        let result: Result<i64, _> = change.extract();
        assert!(result.is_err());

        match result.unwrap_err() {
            ConfigError::TypeMismatch {
                path,
                expected_type,
                actual_value,
            } => {
                assert_eq!(path, "test.mismatch");
                assert_eq!(expected_type, "i64");
                assert_eq!(actual_value, Value::String("not_a_number".to_string()));
            }
            _ => unreachable!("Expected TypeMismatch error"),
        }
    }

    #[test]
    fn extract_complex_type() {
        use serde::Deserialize;

        #[derive(Debug, Deserialize, PartialEq)]
        struct TestStruct {
            name: String,
            count: i64,
        }

        let mut table = toml::map::Map::new();
        table.insert("name".to_string(), Value::String("test".to_string()));
        table.insert("count".to_string(), Value::Integer(10));

        let change = ConfigChange::new(
            "test.complex".to_string(),
            None,
            Value::Table(table),
            ChangeSource::PresetLoad("test_preset".to_string()),
        );

        let result: Result<TestStruct, _> = change.extract();
        assert!(result.is_ok());

        let extracted = result.unwrap();
        assert_eq!(extracted.name, "test");
        assert_eq!(extracted.count, 10);
    }

    #[test]
    fn extract_array() {
        let array = vec![
            Value::String("item1".to_string()),
            Value::String("item2".to_string()),
            Value::String("item3".to_string()),
        ];

        let change = ConfigChange::new(
            "test.array".to_string(),
            None,
            Value::Array(array),
            ChangeSource::FileEdit,
        );

        let result: Result<Vec<String>, _> = change.extract();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec!["item1", "item2", "item3"]);
    }

    #[test]
    fn as_string_success() {
        let change = ConfigChange::new(
            "test.string".to_string(),
            None,
            Value::String("hello".to_string()),
            ChangeSource::Gui,
        );

        assert_eq!(change.as_string(), Some("hello".to_string()));
    }

    #[test]
    fn as_string_failure() {
        let change = ConfigChange::new(
            "test.not_string".to_string(),
            None,
            Value::Boolean(true),
            ChangeSource::Gui,
        );

        assert_eq!(change.as_string(), None);
    }

    #[test]
    fn as_string_or_with_string() {
        let change = ConfigChange::new(
            "test.string".to_string(),
            None,
            Value::String("actual".to_string()),
            ChangeSource::Gui,
        );

        assert_eq!(change.as_string_or("default"), "actual");
    }

    #[test]
    fn as_string_or_with_fallback() {
        let change = ConfigChange::new(
            "test.not_string".to_string(),
            None,
            Value::Integer(42),
            ChangeSource::Gui,
        );

        assert_eq!(change.as_string_or("default"), "default");
    }

    #[test]
    fn change_source_variants() {
        let sources = vec![
            ChangeSource::Gui,
            ChangeSource::FileEdit,
            ChangeSource::FileReload,
            ChangeSource::PresetLoad("dark_theme".to_string()),
            ChangeSource::CliCommand("wayle set theme dark".to_string()),
            ChangeSource::Ipc,
        ];

        for (i, source) in sources.into_iter().enumerate() {
            let change = ConfigChange::new(
                format!("test.path.{}", i),
                None,
                Value::String("test".to_string()),
                source.clone(),
            );
            assert_eq!(change.source, source);
        }
    }

    #[test]
    fn config_change_clone() {
        let original = ConfigChange::new(
            "test.clone".to_string(),
            None,
            Value::String("clone_test".to_string()),
            ChangeSource::Gui,
        );

        let cloned = original.clone();
        assert_eq!(original.path, cloned.path);
        assert_eq!(original.new_value, cloned.new_value);
        assert_eq!(original.source, cloned.source);
    }

    #[test]
    fn config_change_debug() {
        let change = ConfigChange::new(
            "test.debug".to_string(),
            None,
            Value::Boolean(true),
            ChangeSource::FileEdit,
        );

        let debug_str = format!("{:?}", change);
        assert!(debug_str.contains("test.debug"));
        assert!(debug_str.contains("Boolean(true)"));
        assert!(debug_str.contains("FileEdit"));
    }

    #[test]
    fn config_error_display() {
        let error = ConfigError::InvalidPath("invalid.path".to_string());
        assert_eq!(error.to_string(), "Invalid config path: invalid.path");

        let error = ConfigError::TypeMismatch {
            path: "test.path".to_string(),
            expected_type: "bool",
            actual_value: Value::String("not_bool".to_string()),
        };
        assert!(error.to_string().contains("Type mismatch at test.path"));
        assert!(error.to_string().contains("Expected bool"));

        let error = ConfigError::FieldRemoved("old.field".to_string());
        assert_eq!(error.to_string(), "Config field removed: old.field");

        let error = ConfigError::PatternError {
            pattern: "bad.pattern".to_string(),
            reason: "invalid syntax".to_string(),
        };
        assert_eq!(error.to_string(), "invalid path pattern 'bad.pattern': invalid syntax");

        let error = ConfigError::PersistenceError {
            path: std::path::PathBuf::from("/config/path"),
            details: "disk full".to_string(),
        };
        assert_eq!(error.to_string(), "failed to persist config to '/config/path': disk full");
    }
}
