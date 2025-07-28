use tracing::{instrument, warn};
use zbus::Connection;

use crate::services::{
    common::Property,
    network_manager::{
        core::device::{wifi::DeviceWifi, wired::DeviceWired},
        discovery::NetworkServiceDiscovery,
        monitoring::NetworkMonitoring,
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
    /// Current WiFi device state, if available.
    pub wifi: Property<Option<Wifi>>,
    /// Current wired device state, if available.
    pub wired: Property<Option<Wired>>,
    /// Type of the primary network connection.
    pub primary: Property<ConnectionType>,
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
        let connection = Connection::system().await.map_err(|err| {
            NetworkError::InitializationFailed(format!("D-Bus connection failed: {err}"))
        })?;

        let wifi_device_path = NetworkServiceDiscovery::wifi_device_path(&connection).await?;
        let wired_device_path = NetworkServiceDiscovery::wired_device_path(&connection).await?;

        let wifi_device = if let Some(path) = wifi_device_path {
            let device =
                DeviceWifi::from_path_and_connection(connection.clone(), path.clone()).await;
            if device.is_none() {
                warn!("Failed to create WiFi device from path: {}", path);
            }
            device
        } else {
            None
        };

        let wired_device = if let Some(path) = wired_device_path {
            DeviceWired::from_path_and_connection(connection.clone(), path).await
        } else {
            None
        };

        let wifi = match wifi_device {
            Some(device) => {
                match Wifi::from_device_and_connection(connection.clone(), device).await {
                    Ok(wifi) => Some(wifi),
                    Err(e) => {
                        warn!("Failed to create WiFi service: {}", e);
                        None
                    }
                }
            }
            None => None,
        };

        let wired = wired_device
            .map(|device| Wired::from_device_and_connection(connection.clone(), device));

        let wifi_property = Property::new(wifi);
        let wired_property = Property::new(wired);
        let primary = Property::new(ConnectionType::Unknown);

        NetworkMonitoring::start(
            connection.clone(),
            wifi_property.clone(),
            wired_property.clone(),
            primary.clone(),
        )
        .await?;

        let service = Self {
            zbus_connection: connection.clone(),
            wifi: wifi_property,
            wired: wired_property,
            primary,
        };

        Ok(service)
    }
}
