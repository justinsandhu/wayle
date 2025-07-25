//! Common utilities and abstractions for services

/// Reactive property system for fine-grained state updates
pub mod property;
pub mod types;

pub use property::{ComputedProperty, Property};
