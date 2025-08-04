use std::ops::Deref;
use std::sync::Arc;

use futures::stream::Stream;
use monitoring::WifiMonitor;
use zbus::{Connection, zvariant::OwnedObjectPath};

use crate::{services::common::Property, watch_all};

use super::{
    AccessPointProxy, NetworkError, NetworkManagerProxy, NetworkStatus, SSID,
    core::{access_point::AccessPoint, device::wifi::DeviceWifi},
};

mod controls;
mod monitoring;

use controls::WifiControls;

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
    pub strength: Property<Option<u8>>,
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
    /// Get a snapshot of the current WiFi state (no monitoring).
    ///
    /// Fetches the device and current state from NetworkManager via D-Bus.
    ///
    /// # Errors
    ///
    /// Returns `NetworkError::InitializationFailed` if the WiFi device cannot be created
    pub async fn get(
        connection: Connection,
        device_path: OwnedObjectPath,
    ) -> Result<Arc<Self>, NetworkError> {
        let device_arc = DeviceWifi::get(connection.clone(), device_path)
            .await
            .ok_or_else(|| {
                NetworkError::InitializationFailed("Failed to create WiFi device".into())
            })?;
        let device = DeviceWifi::clone(&device_arc);

        let wifi = Self::from_device(connection, device).await?;
        Ok(Arc::new(wifi))
    }

    /// Get a live-updating WiFi instance (with monitoring).
    ///
    /// Fetches the device, current state and starts monitoring for updates.
    ///
    /// # Errors
    ///
    /// Returns `NetworkError::InitializationFailed` if:
    /// - The WiFi device cannot be created
    /// - Failed to start monitoring
    pub async fn get_live(
        connection: Connection,
        device_path: OwnedObjectPath,
    ) -> Result<Arc<Self>, NetworkError> {
        let device_arc = DeviceWifi::get_live(connection.clone(), device_path).await?;
        let device = DeviceWifi::clone(&device_arc);

        let wifi = Self::from_device(connection.clone(), device.clone()).await?;

        WifiMonitor::start(connection, &wifi).await?;

        Ok(Arc::new(wifi))
    }

    /// Watch for any WiFi property changes.
    ///
    /// Emits whenever any WiFi property changes (enabled, connectivity, ssid, strength, or access points).
    pub fn watch(&self) -> impl Stream<Item = Wifi> + Send {
        watch_all!(self, enabled, connectivity, ssid, strength, access_points)
    }

    /// Enable or disable WiFi on the system.
    ///
    /// Controls the system-wide WiFi state through NetworkManager. When disabled,
    /// all WiFi connections are terminated.
    ///
    /// # Errors
    ///
    /// Returns `NetworkError::OperationFailed` if the operation fails.
    pub async fn set_enabled(&self, enabled: bool) -> Result<(), NetworkError> {
        WifiControls::set_enabled(&self.connection, enabled).await
    }

    /// Connect to a WiFi access point.
    ///
    /// Attempts to connect to the specified access point. NetworkManager will
    /// automatically check for existing connection profiles that match this network
    /// and reuse them if found, or create a new profile if needed.
    ///
    /// # Arguments
    ///
    /// * `ap_path` - D-Bus path of the access point to connect to
    /// * `password` - WiFi password (None for open networks)
    ///
    /// # Errors
    ///
    /// Returns `NetworkError::OperationFailed` if the connection fails
    pub async fn connect_to_ap(
        &self,
        ap_path: OwnedObjectPath,
        password: Option<String>,
    ) -> Result<(), NetworkError> {
        WifiControls::connect_to_ap(&self.connection, &self.device.path.get(), ap_path, password)
            .await
    }

    async fn from_device(connection: Connection, device: DeviceWifi) -> Result<Self, NetworkError> {
        let nm_proxy = NetworkManagerProxy::new(&connection).await?;

        let enabled_state = nm_proxy.wireless_enabled().await.unwrap_or(false);
        let device_state = &device.state.get();

        let active_ap_path = &device.active_access_point.get();
        let (ssid, strength) =
            match AccessPointProxy::new(&connection, active_ap_path.to_string()).await {
                Ok(ap_proxy) => {
                    let ssid = ap_proxy
                        .ssid()
                        .await
                        .ok()
                        .map(|raw_ssid| SSID::new(raw_ssid).to_string());

                    let strength = ap_proxy.strength().await.ok();
                    (ssid, strength)
                }
                Err(_) => (None, None),
            };

        Ok(Self {
            connection,
            device,
            enabled: Property::new(enabled_state),
            connectivity: Property::new(NetworkStatus::from_device_state(*device_state)),
            ssid: Property::new(ssid),
            strength: Property::new(strength),
            access_points: Property::new(vec![]),
        })
    }
}
