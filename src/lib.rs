//! Wayle - Compositor-agnostic desktop environment framework.
//!
//! Wayle provides a unified framework for building desktop environment components
//! that work across different Wayland compositors. The main features include:
//!
//! - Reactive configuration system with TOML imports
//! - CLI interface for configuration management
//! - Compositor abstraction layer
//! - Panel and widget system
//!
//! # Quick Start
//!
//! ```rust
//! use wayle::{Result, WayleError};
//! 
//! // Load configuration
//! let config_store = wayle::config_store::ConfigStore::load()?;
//! 
//! // Access configuration values
//! let theme = config_store.get_by_path("general.theme")?;
//! ```

/// Configuration schema definitions and validation.
pub mod config;

/// Core error types and result aliases.
pub mod core;

/// Documentation generation for configuration schemas.
pub mod docs;

/// Reactive configuration store with change tracking.
pub mod config_store;

/// Command-line interface for configuration management.
pub mod cli;

/// Re-exported core types for convenience.
pub use core::{Result, WayleError};
