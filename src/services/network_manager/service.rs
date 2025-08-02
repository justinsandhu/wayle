use tracing::{instrument, warn};
use zbus::Connection;

use std::sync::Arc;
use zbus::zvariant::OwnedObjectPath;

use crate::services::{
    common::Property,
    network_manager::{
        core::{
            access_point::AccessPoint,
            connection::ActiveConnection,
            device::{wifi::DeviceWifi, wired::DeviceWired},
        },
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
    pub wifi: Option<Arc<Wifi>>,
    /// Current wired device state, if available.
    pub wired: Option<Arc<Wired>>,
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
                    Ok(wifi) => Some(Arc::new(wifi)),
                    Err(e) => {
                        warn!("Failed to create WiFi service: {}", e);
                        None
                    }
                }
            }
            None => None,
        };

        let wired = wired_device.map(|device| {
            Arc::new(Wired::from_device_and_connection(
                connection.clone(),
                device,
            ))
        });

        let primary = Property::new(ConnectionType::Unknown);

        NetworkMonitoring::start(
            connection.clone(),
            wifi.clone(),
            wired.clone(),
            primary.clone(),
        )
        .await?;

        let service = Self {
            zbus_connection: connection.clone(),
            wifi,
            wired,
            primary,
        };

        Ok(service)
    }

    /// Get a connection snapshot (no monitoring).
    ///
    /// Use this for one-time queries or when you don't need live updates.
    ///
    /// # Errors
    /// Returns `NetworkError::ObjectNotFound` if the connection doesn't exist.
    pub async fn connection(
        &self,
        path: OwnedObjectPath,
    ) -> Result<Arc<ActiveConnection>, NetworkError> {
        ActiveConnection::get(self.zbus_connection.clone(), path.clone())
            .await
            .ok_or_else(|| NetworkError::ObjectNotFound(path.to_string()))
    }

    /// Get a monitored connection that updates automatically.
    ///
    /// Use this for UI components that need to react to connection state changes.
    /// The connection will automatically stop monitoring when dropped.
    ///
    /// # Errors
    /// Returns `NetworkError::ObjectNotFound` if the connection doesn't exist.
    pub async fn monitored_connection(
        &self,
        path: OwnedObjectPath,
    ) -> Result<Arc<ActiveConnection>, NetworkError> {
        ActiveConnection::get_live(self.zbus_connection.clone(), path.clone())
            .await
            .ok_or_else(|| NetworkError::ObjectNotFound(path.to_string()))
    }

    /// Get an access point snapshot (no monitoring).
    ///
    /// Use this for one-time queries or when listing access points.
    ///
    /// # Errors
    /// Returns `NetworkError::ObjectNotFound` if the access point doesn't exist.
    pub async fn access_point(
        &self,
        path: OwnedObjectPath,
    ) -> Result<Arc<AccessPoint>, NetworkError> {
        AccessPoint::get(self.zbus_connection.clone(), path.clone())
            .await
            .ok_or_else(|| NetworkError::ObjectNotFound(path.to_string()))
    }

    /// Get a monitored access point that updates automatically.
    ///
    /// Use this for UI components showing access point signal strength or properties.
    /// The access point will automatically stop monitoring when dropped.
    ///
    /// # Errors
    /// Returns `NetworkError::ObjectNotFound` if the access point doesn't exist.
    pub async fn monitored_access_point(
        &self,
        path: OwnedObjectPath,
    ) -> Result<Arc<AccessPoint>, NetworkError> {
        AccessPoint::get_live(self.zbus_connection.clone(), path.clone())
            .await
            .ok_or_else(|| NetworkError::ObjectNotFound(path.to_string()))
    }
}
