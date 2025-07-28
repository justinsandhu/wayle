use std::sync::Arc;
use tokio::sync::RwLock;
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
        wifi: Arc<RwLock<Option<Wifi>>>,
        wired: Arc<RwLock<Option<Wired>>>,
        primary: Property<ConnectionType>,
    ) -> Result<(), NetworkError> {
        // TODO: Get NetworkManager proxy

        // TODO: Spawn device monitoring

        // TODO: Spawn primary connection monitoring

        Ok(())
    }

    async fn spawn_device_monitoring(
        connection: Connection,
        wifi: Arc<RwLock<Option<Wifi>>>,
        wired: Arc<RwLock<Option<Wired>>>,
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

        todo!()
    }

    async fn handle_device_added(
        connection: &Connection,
        path: OwnedObjectPath,
        wifi: &Arc<RwLock<Option<Wifi>>>,
        wired: &Arc<RwLock<Option<Wired>>>,
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

        match NMDeviceType::from_u32(device_type) {
            NMDeviceType::Wifi => {
                {
                    let wifi_guard = wifi.read().await;
                    if wifi_guard.is_some() {
                        debug!("A WiFi device already exists, ignoring...");
                        return;
                    }
                }

                let maybe_device =
                    DeviceWifi::from_path_and_connection(connection.clone(), path.clone()).await;

                if let Some(wifi_device) = maybe_device {
                    let wifi_service =
                        Wifi::from_device_and_connection(connection.clone(), wifi_device);

                    // if let Err(e) = wifi_service.start_monitoring().await {
                    //     warn!("Failed to start WiFi monitoring: {e}");
                    // }

                    *wifi.write().await = Some(wifi_service);
                    debug!("WiFi device added and monitoring started");
                }
            }
            NMDeviceType::Ethernet => {
                {
                    let wired_guard = wired.read().await;
                    if wired_guard.is_some() {
                        debug!("An Ethernet device already exists, ignoring...");
                        return;
                    }
                }

                let maybe_device =
                    DeviceWired::from_path_and_connection(connection.clone(), path.clone()).await;

                if let Some(wired_device) = maybe_device {
                    let wired_service =
                        Wired::from_device_and_connection(connection.clone(), wired_device);

                    // if let Err(e) = wired_service.start_monitoring().await {
                    //     warn!("Failed to start Ethernet monitoring: {e}");
                    // }

                    *wired.write().await = Some(wired_service);
                    debug!("Ethernet device added and monitoring started");
                }
            }
            _ => {
                debug!("Ignoring device of type {device_type:#?}");
            }
        }

        todo!()
    }

    async fn handle_device_removed(
        connection: &Connection,
        path: OwnedObjectPath,
        wifi: &Arc<RwLock<Option<Wifi>>>,
        wired: &Arc<RwLock<Option<Wired>>>,
    ) {
        todo!()
    }
}
