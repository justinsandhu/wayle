use std::ops::Deref;
use std::sync::Arc;

use monitoring::WifiMonitoring;
use zbus::Connection;

use crate::services::common::Property;

use super::{
    NetworkError, NetworkStatus,
    core::{access_point::AccessPoint, device::wifi::DeviceWifi},
};
mod control;
mod manager;
mod monitoring;

/// Manages WiFi network connectivity and device state.
///
/// Provides high-level interface for WiFi operations including connection
/// management, access point scanning, and saved network handling. Wraps
/// the lower-level DeviceWifi D-Bus proxy with reactive properties for
/// state monitoring.
#[derive(Clone, Debug)]
pub struct Wifi {
    pub(crate) connection: Connection,
    /// The underlying WiFi device.
    pub device: DeviceWifi,

    /// Whether WiFi is enabled on the system.
    pub enabled: Property<bool>,
    /// Current WiFi connectivity status.
    pub connectivity: Property<NetworkStatus>,
    /// SSID of the currently connected network, if any.
    pub ssid: Property<Option<String>>,
    /// Signal strength of current connection (0-100).
    pub strength: Property<u8>,
    /// List of available access points.
    pub access_points: Property<Vec<Arc<AccessPoint>>>,
}

impl Deref for Wifi {
    type Target = DeviceWifi;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

impl PartialEq for Wifi {
    fn eq(&self, other: &Self) -> bool {
        self.device.path.get() == other.device.path.get()
    }
}

impl Wifi {
    pub(crate) async fn from_device_and_connection(
        connection: Connection,
        device: DeviceWifi,
    ) -> Result<Self, NetworkError> {
        let access_points: Property<Vec<Arc<AccessPoint>>> = Property::new(vec![]);
        WifiMonitoring::start(connection.clone(), &device, &access_points).await?;

        Ok(Self {
            connection,
            device: device.clone(),
            enabled: Property::new(false),
            connectivity: Property::new(NetworkStatus::Disconnected),
            ssid: Property::new(None),
            strength: Property::new(0),
            access_points: access_points.clone(),
        })
    }
}
