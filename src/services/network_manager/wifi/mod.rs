use std::ops::Deref;

use zbus::Connection;

use crate::services::common::Property;

use super::{NetworkStatus, core::device::wifi::DeviceWifi};

mod access_points;
mod control;
mod manager;
mod saved_connections;

/// Manages WiFi network connectivity and device state.
///
/// Provides high-level interface for WiFi operations including connection
/// management, access point scanning, and saved network handling. Wraps
/// the lower-level DeviceWifi D-Bus proxy with reactive properties for
/// state monitoring.
#[derive(Clone, Debug)]
pub struct Wifi {
    connection: Connection,
    device: DeviceWifi,

    /// Whether WiFi is enabled on the system.
    pub enabled: Property<bool>,
    /// Current WiFi connectivity status.
    pub connectivity: Property<NetworkStatus>,
    /// SSID of the currently connected network, if any.
    pub ssid: Property<Option<String>>,
    /// Signal strength of current connection (0-100).
    pub strength: Property<u8>,
}

impl Deref for Wifi {
    type Target = DeviceWifi;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

impl Wifi {
    pub(crate) fn from_device_and_connection(connection: Connection, device: DeviceWifi) -> Self {
        Self {
            connection,
            device,
            enabled: Property::new(false),
            connectivity: Property::new(NetworkStatus::Disconnected),
            ssid: Property::new(None),
            strength: Property::new(0),
        }
    }
}
