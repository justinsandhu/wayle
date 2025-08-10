//! Common utilities and abstractions for services

/// Reactive property system for fine-grained state updates
pub mod property;
// Service macros
#[macro_use]
mod macros;

pub use property::{ComputedProperty, Property};
