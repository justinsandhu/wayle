/// Core domain models for NetworkManager objects
pub mod core;
/// Core domain models for NetworkManager objects
mod discovery;
/// Network service errors
mod error;
/// D-Bus proxy implementations for NetworkManager interfaces.
mod proxy;
/// High-level service API for network operations.
mod service;
/// Type definitions for NetworkManager enums, flags, and states.
mod types;
/// Wi-Fi service API
mod wifi;
/// Wired service API
mod wired;

pub use error::NetworkError;
pub use proxy::*;
pub use service::NetworkService;
pub use types::*;
pub use wifi::Wifi;
pub use wired::Wired;
