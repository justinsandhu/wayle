//! Integration tests for the config store functionality.

#![allow(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used))]

use std::fs;
use tempfile::TempDir;
use wayle::config_store::ConfigStore;

fn setup_test_dir() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".config/wayle");
    fs::create_dir_all(&config_dir).unwrap();

    unsafe {
        std::env::set_var("HOME", temp_dir.path());
    }

    temp_dir
}

fn create_test_config(temp_dir: &TempDir, filename: &str, content: &str) {
    let config_path = temp_dir.path().join(".config/wayle").join(filename);
    fs::write(config_path, content).unwrap();
}

fn create_test_config_in_dir(temp_dir: &TempDir, dir: &str, filename: &str, content: &str) {
    let dir_path = temp_dir.path().join(".config/wayle").join(dir);
    fs::create_dir_all(&dir_path).unwrap();
    fs::write(dir_path.join(filename), content).unwrap();
}

mod basic_operations {
    use super::*;

    #[test]
    fn loads_config_with_all_field_types() {
        let _temp = setup_test_dir();

        create_test_config(
            &_temp,
            "config.toml",
            r#"
[general]
log_level = "debug"

[modules.battery]
enabled = true

[modules.clock.general]
format = "%H:%M"
"#,
        );

        let store = ConfigStore::load().unwrap();

        let log_level = store.get_by_path("general.log_level").unwrap();
        assert_eq!(log_level.as_str().unwrap(), "debug");

        let battery_enabled = store.get_by_path("modules.battery.enabled").unwrap();
        assert!(battery_enabled.as_bool().unwrap());

        let clock_format = store.get_by_path("modules.clock.general.format").unwrap();
        assert_eq!(clock_format.as_str().unwrap(), "%H:%M");
    }

    #[test]
    fn sets_and_gets_values_through_writers() {
        let _temp = setup_test_dir();

        create_test_config(
            &_temp,
            "config.toml",
            r#"
[general]
log_level = "info"

[modules.battery]
enabled = false
"#,
        );

        let store = ConfigStore::load().unwrap();
        let cli_writer = store.cli_writer("test command".to_string());
        let gui_writer = store.gui_writer();

        cli_writer
            .set(
                "general.log_level",
                toml::Value::String("debug".to_string()),
            )
            .unwrap();
        gui_writer
            .set("modules.battery.enabled", toml::Value::Boolean(true))
            .unwrap();

        assert_eq!(
            store
                .get_by_path("general.log_level")
                .unwrap()
                .as_str()
                .unwrap(),
            "debug"
        );
        assert!(
            store
                .get_by_path("modules.battery.enabled")
                .unwrap()
                .as_bool()
                .unwrap()
        );
    }

    #[test]
    fn handles_nonexistent_paths_gracefully() {
        let _temp = setup_test_dir();

        create_test_config(
            &_temp,
            "config.toml",
            r#"
[general]
log_level = "info"

[modules.battery]
enabled = false
"#,
        );

        let store = ConfigStore::load().unwrap();

        assert!(store.get_by_path("non.existent.path").is_err());
        assert!(store.get_by_path("general.log_level.field").is_err());
        assert!(store.get_by_path("modules.nonexistent").is_err());
    }
}

mod runtime_persistence {
    use super::*;

    #[test]
    fn persists_cli_and_gui_changes_across_sessions() {
        let _temp = setup_test_dir();

        create_test_config(
            &_temp,
            "config.toml",
            r#"
[other]
some_field = "value"
"#,
        );

        {
            let store = ConfigStore::load().unwrap();
            let cli_writer = store.cli_writer("set log_level".to_string());
            let gui_writer = store.gui_writer();

            cli_writer
                .set(
                    "general.log_level",
                    toml::Value::String("debug".to_string()),
                )
                .unwrap();
            gui_writer
                .set("modules.battery.enabled", toml::Value::Boolean(true))
                .unwrap();

            let runtime_path = _temp.path().join(".config/wayle/runtime.toml");
            assert!(runtime_path.exists());
        }

        {
            let store = ConfigStore::load().unwrap();

            assert_eq!(
                store
                    .get_by_path("general.log_level")
                    .unwrap()
                    .as_str()
                    .unwrap(),
                "debug"
            );
            assert!(
                store
                    .get_by_path("modules.battery.enabled")
                    .unwrap()
                    .as_bool()
                    .unwrap()
            );
        }
    }

    #[test]
    fn creates_runtime_config_when_setting_values() {
        let _temp = setup_test_dir();

        create_test_config(
            &_temp,
            "config.toml",
            r#"
[general]
log_level = "info"
"#,
        );

        let store = ConfigStore::load().unwrap();
        assert_eq!(
            store
                .get_by_path("general.log_level")
                .unwrap()
                .as_str()
                .unwrap(),
            "info"
        );

        let cli_writer = store.cli_writer("test".to_string());
        cli_writer
            .set(
                "general.log_level",
                toml::Value::String("debug".to_string()),
            )
            .unwrap();

        let runtime_path = _temp.path().join(".config/wayle/runtime.toml");
        assert!(runtime_path.exists());
    }

    #[test]
    fn filters_sources_in_runtime_config() {
        let _temp = setup_test_dir();

        create_test_config(
            &_temp,
            "config.toml",
            r#"
[general]
log_level = "info"

[modules.battery]
enabled = false
"#,
        );

        let store = ConfigStore::load().unwrap();
        let cli_writer = store.cli_writer("test".to_string());
        let gui_writer = store.gui_writer();
        let system_writer = store.system_writer();

        cli_writer
            .set(
                "general.log_level",
                toml::Value::String("debug".to_string()),
            )
            .unwrap();
        gui_writer
            .set("modules.battery.enabled", toml::Value::Boolean(true))
            .unwrap();
        system_writer
            .set(
                "general.log_level",
                toml::Value::String("error".to_string()),
            )
            .unwrap();

        drop(store);

        let runtime_path = _temp.path().join(".config/wayle/runtime.toml");
        if runtime_path.exists() {
            let runtime_content = fs::read_to_string(runtime_path).unwrap();
            assert!(runtime_content.contains("debug"));
            assert!(runtime_content.contains("true"));
            assert!(!runtime_content.contains("error"));
        }
    }
}

mod import_system {
    use super::*;

    #[test]
    fn resolves_basic_imports() {
        let _temp = setup_test_dir();

        create_test_config_in_dir(
            &_temp,
            "themes",
            "dark.toml",
            r#"
[modules.clock.styling.button]
icon = "dark_icon"
"#,
        );

        create_test_config(
            &_temp,
            "config.toml",
            r#"
imports = ["@themes/dark"]

[general]
log_level = "info"
"#,
        );

        let store = ConfigStore::load().unwrap();

        let icon = store
            .get_by_path("modules.clock.styling.button.icon")
            .unwrap();
        assert_eq!(icon.as_str().unwrap(), "dark_icon");

        let log_level = store.get_by_path("general.log_level").unwrap();
        assert_eq!(log_level.as_str().unwrap(), "info");
    }

    #[test]
    fn respects_import_precedence_order() {
        let _temp = setup_test_dir();

        create_test_config_in_dir(
            &_temp,
            "modules",
            "base.toml",
            r#"
[general]
log_level = "warn"

[modules.battery]
enabled = false
"#,
        );

        create_test_config_in_dir(
            &_temp,
            "modules",
            "override.toml",
            r#"
[general]
log_level = "debug"
"#,
        );

        create_test_config(
            &_temp,
            "config.toml",
            r#"
imports = ["@modules/base", "@modules/override"]

[modules.battery]
enabled = true
"#,
        );

        let store = ConfigStore::load().unwrap();

        assert_eq!(
            store
                .get_by_path("general.log_level")
                .unwrap()
                .as_str()
                .unwrap(),
            "debug"
        );
        assert!(
            store
                .get_by_path("modules.battery.enabled")
                .unwrap()
                .as_bool()
                .unwrap()
        );
    }

    #[test]
    fn adds_runtime_import_to_main_config() {
        let _temp = setup_test_dir();

        create_test_config_in_dir(&_temp, "modules", "test.toml", "");

        create_test_config(
            &_temp,
            "config.toml",
            r#"
imports = ["@modules/test"]

[general]
log_level = "info"
"#,
        );

        let store = ConfigStore::load().unwrap();
        let cli_writer = store.cli_writer("set command".to_string());

        cli_writer
            .set(
                "general.log_level",
                toml::Value::String("debug".to_string()),
            )
            .unwrap();

        let main_content =
            fs::read_to_string(_temp.path().join(".config/wayle/config.toml")).unwrap();
        assert!(main_content.contains("@runtime"));
    }
}

mod path_operations {
    use super::*;

    #[test]
    fn creates_nested_config_paths() {
        let _temp = setup_test_dir();

        create_test_config(
            &_temp,
            "config.toml",
            r#"
[general]
log_level = "info"

[modules.battery]
enabled = false
"#,
        );

        let store = ConfigStore::load().unwrap();
        let cli_writer = store.cli_writer("set command".to_string());

        cli_writer
            .set("modules.battery.enabled", toml::Value::Boolean(true))
            .unwrap();

        assert!(
            store
                .get_by_path("modules.battery.enabled")
                .unwrap()
                .as_bool()
                .unwrap()
        );
        assert!(store.get_by_path("modules").is_ok());
        assert!(store.get_by_path("modules.battery").is_ok());
    }

    #[test]
    fn modifies_config_values_in_place() {
        let _temp = setup_test_dir();

        create_test_config(
            &_temp,
            "config.toml",
            r#"
[general]
log_level = "info"
"#,
        );

        let store = ConfigStore::load().unwrap();
        let cli_writer = store.cli_writer("set command".to_string());

        assert_eq!(
            store
                .get_by_path("general.log_level")
                .unwrap()
                .as_str()
                .unwrap(),
            "info"
        );

        cli_writer
            .set(
                "general.log_level",
                toml::Value::String("debug".to_string()),
            )
            .unwrap();
        assert_eq!(
            store
                .get_by_path("general.log_level")
                .unwrap()
                .as_str()
                .unwrap(),
            "debug"
        );
    }
}

mod config_writers {
    use super::*;

    #[test]
    fn provides_writer_interfaces_for_different_sources() {
        let _temp = setup_test_dir();

        create_test_config(
            &_temp,
            "config.toml",
            r#"
[general]
log_level = "info"
"#,
        );

        let store = ConfigStore::load().unwrap();

        let _gui_writer = store.gui_writer();
        let _cli_writer = store.cli_writer("command".to_string());
        let _system_writer = store.system_writer();
    }

    #[test]
    fn executes_config_changes_through_writers() {
        let _temp = setup_test_dir();

        create_test_config(
            &_temp,
            "config.toml",
            r#"
[general]
log_level = "info"

[modules.battery]
enabled = false
"#,
        );

        let store = ConfigStore::load().unwrap();
        let writer = store.cli_writer("test command".to_string());

        writer
            .set("modules.battery.enabled", toml::Value::Boolean(true))
            .unwrap();

        assert!(
            store
                .get_by_path("modules.battery.enabled")
                .unwrap()
                .as_bool()
                .unwrap()
        );
    }
}

mod type_handling {
    use super::*;

    #[test]
    fn handles_different_toml_value_types() {
        let _temp = setup_test_dir();

        create_test_config(
            &_temp,
            "config.toml",
            r#"
[general]
log_level = "info"

[modules.battery]
enabled = false
"#,
        );

        let store = ConfigStore::load().unwrap();
        let cli_writer = store.cli_writer("test".to_string());

        assert_eq!(
            store
                .get_by_path("general.log_level")
                .unwrap()
                .as_str()
                .unwrap(),
            "info"
        );
        assert!(
            !store
                .get_by_path("modules.battery.enabled")
                .unwrap()
                .as_bool()
                .unwrap()
        );

        cli_writer
            .set(
                "general.log_level",
                toml::Value::String("debug".to_string()),
            )
            .unwrap();
        cli_writer
            .set("modules.battery.enabled", toml::Value::Boolean(true))
            .unwrap();

        assert_eq!(
            store
                .get_by_path("general.log_level")
                .unwrap()
                .as_str()
                .unwrap(),
            "debug"
        );
        assert!(
            store
                .get_by_path("modules.battery.enabled")
                .unwrap()
                .as_bool()
                .unwrap()
        );
    }
}

mod error_conditions {
    use super::*;

    #[test]
    fn handles_invalid_toml_syntax() {
        let _temp = setup_test_dir();

        create_test_config(
            &_temp,
            "config.toml",
            r#"
[general
invalid toml syntax
[[[broken
"#,
        );

        let result = ConfigStore::load();
        assert!(result.is_err());
    }

    #[test]
    fn handles_missing_import_files() {
        let _temp = setup_test_dir();

        create_test_config(
            &_temp,
            "config.toml",
            r#"
imports = ["@nonexistent/file"]

[general]
log_level = "info"
"#,
        );

        let result = ConfigStore::load();
        assert!(result.is_err());
    }

    #[test]
    fn handles_circular_imports() {
        let _temp = setup_test_dir();

        create_test_config_in_dir(
            &_temp,
            "circular",
            "a.toml",
            r#"
imports = ["@circular/b"]

[config_a]
value = "from_a"
"#,
        );

        create_test_config_in_dir(
            &_temp,
            "circular",
            "b.toml",
            r#"
imports = ["@circular/a"]

[config_b]
value = "from_b"
"#,
        );

        create_test_config(
            &_temp,
            "config.toml",
            r#"
imports = ["@circular/a"]

[general]
log_level = "info"
"#,
        );

        let result = ConfigStore::load();
        assert!(result.is_err());
    }

    #[test]
    fn handles_self_referencing_import() {
        let _temp = setup_test_dir();

        create_test_config(
            &_temp,
            "config.toml",
            r#"
imports = ["@config"]

[general]
log_level = "info"
"#,
        );

        let result = ConfigStore::load();
        assert!(result.is_err());
    }

    #[test]
    fn handles_invalid_import_syntax() {
        let _temp = setup_test_dir();

        create_test_config_in_dir(
            &_temp,
            "broken",
            "invalid.toml",
            r#"
[general
broken syntax here
"#,
        );

        create_test_config(
            &_temp,
            "config.toml",
            r#"
imports = ["@broken/invalid"]

[general]
log_level = "info"
"#,
        );

        let result = ConfigStore::load();
        assert!(result.is_err());
    }

    #[test]
    fn handles_deeply_nested_invalid_paths() {
        let _temp = setup_test_dir();

        create_test_config(
            &_temp,
            "config.toml",
            r#"
[general]
log_level = "info"
"#,
        );

        let store = ConfigStore::load().unwrap();

        assert!(store.get_by_path("").is_err());
        assert!(
            store
                .get_by_path("general.log_level.nonexistent.deep.path")
                .is_err()
        );
        assert!(store.get_by_path("completely.nonexistent.path").is_err());
    }

    #[test]
    fn handles_empty_config_file() {
        let _temp = setup_test_dir();

        create_test_config(&_temp, "config.toml", "");

        let result = ConfigStore::load();
        assert!(result.is_ok());
    }

    #[test]
    fn handles_whitespace_only_config() {
        let _temp = setup_test_dir();

        create_test_config(&_temp, "config.toml", "   \n\t  \n  ");

        let result = ConfigStore::load();
        assert!(result.is_ok());
    }

    #[test]
    fn handles_setting_values_on_wrong_types() {
        let _temp = setup_test_dir();

        create_test_config(
            &_temp,
            "config.toml",
            r#"
[general]
log_level = "info"
"#,
        );

        let store = ConfigStore::load().unwrap();
        let cli_writer = store.cli_writer("test".to_string());

        let result = cli_writer.set(
            "general.log_level.invalid_subfield",
            toml::Value::String("value".to_string()),
        );
        assert!(result.is_err());
    }

    #[test]
    fn handles_unicode_and_special_characters() {
        let _temp = setup_test_dir();

        create_test_config(
            &_temp,
            "config.toml",
            r#"
[general]
log_level = "info"
"#,
        );

        let store = ConfigStore::load().unwrap();
        let cli_writer = store.cli_writer("test unicode 🦀".to_string());

        let result = cli_writer.set(
            "general.log_level",
            toml::Value::String("debug".to_string()),
        );
        assert!(result.is_ok());

        let result_invalid = cli_writer.set(
            "general.log_level",
            toml::Value::String("测试 🦀 invalid".to_string()),
        );
        assert!(result_invalid.is_err());
    }

    #[test]
    fn handles_very_long_paths() {
        let _temp = setup_test_dir();

        create_test_config(
            &_temp,
            "config.toml",
            r#"
[modules.battery]
enabled = true
"#,
        );

        let store = ConfigStore::load().unwrap();

        let value = store.get_by_path("modules.battery.enabled").unwrap();
        assert!(value.as_bool().unwrap());

        assert!(
            store
                .get_by_path(
                    "extremely.long.nonexistent.path.that.does.not.exist.anywhere.in.config"
                )
                .is_err()
        );
    }

    #[test]
    fn handles_concurrent_writers_same_path() {
        let _temp = setup_test_dir();

        create_test_config(
            &_temp,
            "config.toml",
            r#"
[general]
log_level = "info"
"#,
        );

        let store = ConfigStore::load().unwrap();
        let cli_writer1 = store.cli_writer("command1".to_string());
        let cli_writer2 = store.cli_writer("command2".to_string());

        let result1 = cli_writer1.set(
            "general.log_level",
            toml::Value::String("debug".to_string()),
        );
        let result2 = cli_writer2.set(
            "general.log_level",
            toml::Value::String("error".to_string()),
        );

        assert!(result1.is_ok());
        assert!(result2.is_ok());

        let final_value = store.get_by_path("general.log_level").unwrap();
        assert!(
            final_value.as_str().unwrap() == "debug" || final_value.as_str().unwrap() == "error"
        );
    }
}
