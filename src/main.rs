//! Wayle orchestrator - Main entry point that manages panel and settings processes
//!
//! This binary is designed to always start successfully, even if dependencies are missing,
//! so it can provide diagnostic information to help users resolve issues.

use std::{env, error::Error, fs, process};

use tracing::{Level, info, instrument, span};
use wayle::{
    cli::{CliService, formatting::format_error},
    config::ConfigPaths,
    config_runtime::ConfigRuntime,
    tracing_config,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    let is_cli_command = !matches!(
        args.get(1).map(|s| s.as_str()),
        Some("start") | Some("stop") | Some("restart") | Some("status") | Some("settings")
    );

    if is_cli_command {
        tracing_config::init_cli_mode()?;
    } else {
        tracing_config::init_with_file()?;
        let _span = span!(Level::INFO, "wayle_main").entered();
        info!("Starting Wayle desktop environment framework");
    }

    ensure_wayle_directories()?;

    match args.get(1).map(|s| s.as_str()) {
        Some("start") => todo!(),
        Some("stop") => todo!(),
        Some("restart") => todo!(),
        Some("status") => todo!(),
        Some("settings") => todo!(),
        _ => run_cli_command(&args[1..]).await?,
    }

    Ok(())
}

/// Executes CLI commands through the CliService.
///
/// Parses command line arguments and routes them to the appropriate command
/// handler. Supports both category-based commands (config get, config set)
/// and general help commands.
///
/// # Arguments
/// * `args` - Command line arguments (excluding program name)
///
/// # Errors
/// Returns error if command execution fails or config store initialization fails.
async fn run_cli_command(args: &[String]) -> Result<(), Box<dyn Error>> {
    let config_runtime = ConfigRuntime::load()?;
    let cli_service = CliService::new(config_runtime);

    let category = args.first().map(|s| s.as_str()).unwrap_or("help");
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("");
    let command_args = args.get(2..).unwrap_or(&[]);

    let result = cli_service
        .execute_command(category, command, command_args)
        .await;

    match result {
        Ok(output) => {
            if !output.trim().is_empty() {
                println!("{output}");
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("{}", format_error(&e.to_string()));
            process::exit(1);
        }
    }
}

#[instrument]
fn ensure_wayle_directories() -> Result<(), Box<dyn Error>> {
    let config_dir = ConfigPaths::config_dir()?;
    if !config_dir.exists() {
        info!("Creating config directory: {}", config_dir.display());
        fs::create_dir_all(&config_dir)?;
    }
    Ok(())
}
