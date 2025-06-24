//! Wayle orchestrator - Main entry point that manages panel and settings processes
//!
//! This binary is designed to always start successfully, even if dependencies are missing,
//! so it can provide diagnostic information to help users resolve issues.

use std::env;

use wayle::{
    cli::{CliError, CliService},
    config_store::ConfigStore,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let config_store = ConfigStore::load()?;
    let cli_service = CliService::new(config_store);

    if args.len() < 3 {
        eprintln!("Usage: wayle <category> <command> [args...]");
        eprintln!("Example: wayle config get modules.battery.enabled");
        return Ok(());
    }

    let category = args.get(1).ok_or_else(|| {
        CliError::InvalidArguments("Comand argument <category> not provided".to_string())
    })?;
    let command = args.get(2).ok_or_else(|| {
        CliError::InvalidArguments("Comand argument <command> not provided".to_string())
    })?;
    let command_args = args.get(3..).ok_or_else(|| {
        CliError::InvalidArguments("Comand argument <[args...]> not provided".to_string())
    })?;

    match cli_service.execute_command(category, command, command_args) {
        Ok(result) => print!("{}", result),
        Err(err) => eprintln!("Error: {}", err),
    }

    Ok(())
}
