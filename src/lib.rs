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
//! use wayle::config_store::ConfigStore;
//!
//! // Create configuration store with defaults
//! let config_store = ConfigStore::with_defaults();
//!
//! // Access configuration values  
//! let config = config_store.get_current();
//! println!("Config loaded: {:?}", config.general);
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
