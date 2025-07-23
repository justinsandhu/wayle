//! NetworkManager OVS Interface Device interface.

use zbus::proxy;

/// OVS Interface Device.
#[proxy(
    default_service = "org.freedesktop.NetworkManager",
    interface = "org.freedesktop.NetworkManager.Device.OvsInterface"
)]
pub trait DeviceOvsInterface {
    // No properties
}

