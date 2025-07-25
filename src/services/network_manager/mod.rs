mod active_connection;
mod device;
mod dhcp4_config;
mod dhcp6_config;
/// Ethernet specific functionality
mod ethernet;
mod ip4_config;
mod ip6_config;
/// Core network functionality
mod network;
/// D-Bus proxy implementations for NetworkManager interfaces.
mod proxy;
/// High-level service API for network operations.
mod service;
mod settings_connection;
/// Type definitions for NetworkManager enums, flags, and states.
mod types;
/// Wi-Fi specific functionality
mod wifi;

pub use proxy::*;
pub use service::*;
pub use types::*;
pub use wifi::*;
