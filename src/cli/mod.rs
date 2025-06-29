//! Command-line interface for configuration management.
//!
//! Provides a hierarchical command system for interacting with Wayle's
//! reactive configuration store. Commands are organized by category
//! and automatically generate help text from metadata.

mod commands;
pub mod formatting;
mod registry;
mod service;
mod types;

pub use commands::config::GetCommand;
pub use registry::CommandRegistry;
pub use service::CliService;
pub use types::{CliError, Command, CommandResult};
