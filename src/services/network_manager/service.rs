use tracing::instrument;
use zbus::Connection;

use crate::services::{
    common::Property,
    network_manager::{
        core::device::{wifi::DeviceWifi, wired::DeviceWired},
        discovery::NetworkServiceDiscovery,
    },
};

use super::{ConnectionType, NetworkError, Wifi, Wired};

/// Manages network connectivity through NetworkManager D-Bus interface.
///
/// Provides unified access to both WiFi and wired network interfaces,
/// tracking their state and managing connections. The service monitors
/// the primary connection type and exposes reactive properties for
/// network status changes.
pub struct NetworkService {
    zbus_connection: Connection,
    wifi: Property<Option<Wifi>>,
    wired: Property<Option<Wired>>,
    primary: Property<ConnectionType>,
}

impl NetworkService {
    /// Creates a new network service instance.
    ///
    /// Initializes D-Bus connection and discovers available network devices.
    /// The service will automatically detect WiFi and wired interfaces if present.
    ///
    /// # Errors
    /// Returns `NetworkError::InitializationFailed` if:
    /// - D-Bus connection cannot be established
    /// - NetworkManager service is not available
    /// - Device discovery fails
    pub async fn new() -> Result<Self, NetworkError> {
        Self::start().await
    }

    /// Starts the network service and initializes all components.
    ///
    /// Performs device discovery, creates WiFi and wired service instances
    /// for available devices, and sets up property monitoring. Handles
    /// the actual initialization logic for the service.
    ///
    /// # Errors
    /// Returns `NetworkError::InitializationFailed` if:
    /// - D-Bus session connection fails
    /// - Device path discovery encounters errors
    /// - Device proxy creation fails
    #[instrument]
    pub async fn start() -> Result<Self, NetworkError> {
        let connection = Connection::session().await.map_err(|err| {
            NetworkError::InitializationFailed(format!("D-Bus connection failed: {err}"))
        })?;

        let wifi_device_path = NetworkServiceDiscovery::wifi_device_path(&connection).await?;
        let wired_device_path = NetworkServiceDiscovery::wired_device_path(&connection).await?;

        let wifi_device = if let Some(path) = wifi_device_path {
            DeviceWifi::from_path_and_connection(connection.clone(), path).await
        } else {
            None
        };

        let wired_device = if let Some(path) = wired_device_path {
            DeviceWired::from_path_and_connection(connection.clone(), path).await
        } else {
            None
        };

        let wifi =
            wifi_device.map(|device| Wifi::from_device_and_connection(connection.clone(), device));

        let wired = wired_device
            .map(|device| Wired::from_device_and_connection(connection.clone(), device));

        let service = Self {
            zbus_connection: connection.clone(),
            wifi: Property::new(wifi),
            wired: Property::new(wired),
            primary: Property::new(ConnectionType::Unknown),
        };

        Ok(service)
    }
}
