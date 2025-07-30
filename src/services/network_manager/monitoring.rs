use tokio_stream::StreamExt;
use tracing::{debug, warn};
use zbus::{Connection, zvariant::OwnedObjectPath};

use super::{ConnectionType, DeviceProxy, NetworkError, NetworkManagerProxy, Wifi, Wired};
use crate::services::{
    common::Property,
    network_manager::{
        NMDeviceType,
        core::device::{wifi::DeviceWifi, wired::DeviceWired},
    },
};

/// Handles ongoing monitoring of network devices and connections.
pub(crate) struct NetworkMonitoring;

impl NetworkMonitoring {
    /// Start all network monitoring tasks.
    pub(crate) async fn start(
        connection: Connection,
        wifi: Property<Option<Wifi>>,
        wired: Property<Option<Wired>>,
        primary: Property<ConnectionType>,
    ) -> Result<(), NetworkError> {
        Self::spawn_device_monitoring(connection.clone(), wifi.clone(), wired.clone()).await?;
        Self::spawn_primary_monitoring(connection, wifi, wired, primary).await?;

        Ok(())
    }

    async fn spawn_device_monitoring(
        connection: Connection,
        wifi: Property<Option<Wifi>>,
        wired: Property<Option<Wired>>,
    ) -> Result<(), NetworkError> {
        let nm_proxy = NetworkManagerProxy::new(&connection)
            .await
            .map_err(NetworkError::DbusError)?;

        tokio::spawn(async move {
            let mut device_added = match nm_proxy.receive_device_added().await {
                Ok(stream) => stream,
                Err(e) => {
                    warn!("Failed to subscribe to DeviceAdded: {e}");
                    return;
                }
            };

            let mut device_removed = match nm_proxy.receive_device_removed().await {
                Ok(stream) => stream,
                Err(e) => {
                    warn!("Failed to subscribe to DeviceRemoved: {e}");
                    return;
                }
            };

            loop {
                tokio::select! {
                    Some(signal) = device_added.next() => {
                        if let Ok(args) = signal.args() {
                            Self::handle_device_added(
                                &connection,
                                args.device_path,
                                &wifi,
                                &wired
                            ).await
                        }
                    }
                    Some(signal) = device_removed.next() => {
                        if let Ok(args) = signal.args() {
                            Self::handle_device_removed(
                                &connection,
                                args.device_path,
                                &wifi,
                                &wired
                            ).await
                        }
                    }
                    else => break,
                }
            }
        });

        Ok(())
    }

    async fn handle_device_added(
        connection: &Connection,
        path: OwnedObjectPath,
        wifi: &Property<Option<Wifi>>,
        wired: &Property<Option<Wired>>,
    ) {
        let device_proxy = match DeviceProxy::new(connection, path.clone()).await {
            Ok(proxy) => proxy,
            Err(e) => {
                warn!("Failed to create device proxy for {path}: {e}");
                return;
            }
        };

        let device_type = match device_proxy.device_type().await {
            Ok(t) => t,
            Err(e) => {
                warn!("Failed to get device type for {}: {e}", path.clone());
                return;
            }
        };

        debug!(
            "Device added: {} with type {:?}",
            path,
            NMDeviceType::from_u32(device_type)
        );

        match NMDeviceType::from_u32(device_type) {
            NMDeviceType::Wifi => {
                if wifi.get().is_some() {
                    debug!("A WiFi device already exists, ignoring...");
                    return;
                }

                let maybe_device =
                    DeviceWifi::from_path_and_connection(connection.clone(), path.clone()).await;

                if let Some(wifi_device) = maybe_device {
                    let wifi_service =
                        Wifi::from_device_and_connection(connection.clone(), wifi_device)
                            .await
                            .ok();

                    if let Some(wifi_service) = wifi_service {
                        wifi.set(Some(wifi_service));
                        debug!("WiFi device added and monitoring started");
                    }
                }
            }
            NMDeviceType::Ethernet => {
                if wired.get().is_some() {
                    debug!("An Ethernet device already exists, ignoring...");
                    return;
                }

                let maybe_device =
                    DeviceWired::from_path_and_connection(connection.clone(), path.clone()).await;

                if let Some(wired_device) = maybe_device {
                    let wired_service =
                        Wired::from_device_and_connection(connection.clone(), wired_device);

                    wired.set(Some(wired_service));
                    debug!("Ethernet device added and monitoring started");
                }
            }
            _ => {
                debug!(
                    "Ignoring device of type {:?}",
                    NMDeviceType::from_u32(device_type)
                );
            }
        }
    }

    async fn handle_device_removed(
        _connection: &Connection,
        path: OwnedObjectPath,
        wifi: &Property<Option<Wifi>>,
        wired: &Property<Option<Wired>>,
    ) {
        if let Some(ref wifi_service) = wifi.get()
            && wifi_service.device.path.get() == path.to_string()
        {
            wifi.set(None);
            debug!("WiFi device removed");
            return;
        }

        if let Some(ref wired_service) = wired.get()
            && wired_service.device.path.get() == path.to_string()
        {
            wired.set(None);
            debug!("Ethernet device removed");
            return;
        }

        debug!("Unknown device removed: {}", path);
    }

    async fn spawn_primary_monitoring(
        connection: Connection,
        wifi: Property<Option<Wifi>>,
        wired: Property<Option<Wired>>,
        primary: Property<ConnectionType>,
    ) -> Result<(), NetworkError> {
        let nm_proxy = NetworkManagerProxy::new(&connection)
            .await
            .map_err(NetworkError::DbusError)?;

        let primary_connection = nm_proxy.primary_connection().await?;
        Self::update_primary_connection(primary_connection, &wifi, &wired, &primary).await;

        let mut primary_changed = nm_proxy.receive_primary_connection_changed().await;

        tokio::spawn(async move {
            while let Some(change) = primary_changed.next().await {
                if let Ok(new_primary_connection) = change.get().await {
                    println!("Primary Connection: {new_primary_connection}");
                    Self::update_primary_connection(
                        new_primary_connection,
                        &wifi,
                        &wired,
                        &primary,
                    )
                    .await;
                }
            }
        });

        Ok(())
    }

    async fn update_primary_connection(
        connection: OwnedObjectPath,
        wifi: &Property<Option<Wifi>>,
        wired: &Property<Option<Wired>>,
        primary: &Property<ConnectionType>,
    ) {
        if let Some(ref wifi_service) = wifi.get() {
            if wifi_service.active_connection.get().as_str() == connection.as_str() {
                primary.set(ConnectionType::Wifi);
                return;
            }
        }

        if let Some(ref wired_service) = wired.get() {
            if wired_service.active_connection.get().as_str() == connection.as_str() {
                primary.set(ConnectionType::Wired);
                return;
            }
        }

        primary.set(ConnectionType::Unknown);
    }
}
