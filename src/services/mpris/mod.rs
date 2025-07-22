//! MPRIS-based media service implementation
//!
//! This module provides reactive media player control through the D-Bus MPRIS protocol.
//! It automatically discovers players and provides streams for UI updates.

/// Error types for the MPRIS service
pub mod error;
/// Main MPRIS service implementation
pub mod service;
/// Type definitions for MPRIS functionality
pub mod types;

pub mod player;
mod proxy;

pub use error::MediaError;
pub use player::{Player, TrackMetadata, UNKNOWN_METADATA};
pub use service::{Config, MprisService};
pub use types::*;

pub use service::MprisService as MediaService;
