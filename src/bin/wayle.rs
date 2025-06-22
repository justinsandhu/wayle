//! Wayle binary - Entry point for the Wayle status bar application.

use std::path::Path;
use wayle::{Result, config::Config};

fn main() -> Result<()> {
    let config_path = std::env::args().nth(1).unwrap_or_else(|| "config.toml".to_string());
    let config = match Config::load_with_imports(Path::new(&config_path)) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    println!("Successfully loaded toml with imports");
    println!("{:#?}", config);

    if let Some(battery) = &config.modules.battery {
        println!(
            "\nBattery module: enabled={}, show_percentage={}",
            battery.enabled, battery.show_percentage
        );
    }

    println!("\nLog level: {}", config.general.log_level);

    Ok(())
}
