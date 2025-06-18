//! Wayle - A Wayland status bar.
//!
//! This crate provides the core functionality for the Wayle status bar,
//! including configuration management and error handling.

/// Configuration management module.
pub mod config;

/// Core types and error handling.
pub mod core;

/// Document generation module
pub mod docs;

pub use core::{Result, WayleError};
