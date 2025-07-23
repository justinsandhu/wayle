//! NetworkManager Loopback Device interface.

use zbus::proxy;

/// Loopback Device.
#[proxy(
    default_service = "org.freedesktop.NetworkManager",
    interface = "org.freedesktop.NetworkManager.Device.Loopback"
)]
pub trait DeviceLoopback {
    // No properties
}

