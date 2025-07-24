/// Ethernet specific functionality
mod ethernet;
/// Core network functionality
mod network;
/// D-Bus proxy implementations for NetworkManager interfaces.
mod proxy;
/// High-level service API for network operations.
mod service;
/// Type definitions for NetworkManager enums, flags, and states.
mod types;
/// Wi-Fi specific functionality
mod wifi;

pub use proxy::*;
pub use service::*;
pub use types::*;
pub use wifi::*;
