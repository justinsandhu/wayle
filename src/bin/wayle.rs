//! Wayle orchestrator - Main entry point that manages panel and settings processes
//!
//! This binary is designed to always start successfully, even if dependencies are missing,
//! so it can provide diagnostic information to help users resolve issues.

use std::fs;

use wayle::{
    cli::{CliService, formatting::format_error},
    config::ConfigPaths,
    config_store::ConfigStore,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    ensure_wayle_directories()?;

    let config_store = ConfigStore::load()?;
    let cli_service = CliService::new(config_store);

    let category = args.get(1).map(|s| s.as_str()).unwrap_or("help");
    let command = args.get(2).map(|s| s.as_str()).unwrap_or("");
    let remaining_args = args.get(3..).unwrap_or(&[]);

    match cli_service.execute_command(category, command, remaining_args) {
        Ok(result) => println!("{}", result),
        Err(e) => {
            eprintln!("{}: {}", format_error("Error"), e);
            std::process::exit(1);
        }
    }

    Ok(())
}

fn ensure_wayle_directories() -> Result<(), Box<dyn std::error::Error>> {
    let config_dir = ConfigPaths::config_dir()?;
    fs::create_dir_all(&config_dir)?;
    Ok(())
}
