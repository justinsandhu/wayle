//! Common utilities and abstractions for services

/// Reactive property system for fine-grained state updates
pub mod property;
/// Common type definitions and conversions used across services.
pub mod types;
// Service macros
#[macro_use]
mod macros;

pub use property::{ComputedProperty, Property};
