use std::sync::{Arc, Weak};

use futures::StreamExt;
use tracing::debug;
use zbus::{Connection, zvariant::OwnedObjectPath};

use crate::services::network_manager::{
    NetworkError, proxy::devices::wireless::DeviceWirelessProxy, types::NM80211Mode,
};

use super::DeviceWifi;

/// Monitors D-Bus properties and updates the reactive DeviceWifi model.
pub(crate) struct DeviceWifiMonitor;

impl DeviceWifiMonitor {
    pub(super) async fn start(
        device: Arc<DeviceWifi>,
        connection: &Connection,
        path: OwnedObjectPath,
    ) -> Result<(), NetworkError> {
        let weak = Arc::downgrade(&device);

        let proxy = DeviceWirelessProxy::new(connection, path)
            .await
            .map_err(NetworkError::DbusError)?;

        tokio::spawn(async move {
            Self::monitor(weak, proxy).await;
        });

        Ok(())
    }

    #[allow(clippy::cognitive_complexity)]
    async fn monitor(weak: Weak<DeviceWifi>, proxy: DeviceWirelessProxy<'static>) {
        let mut perm_hw_address_changed = proxy.receive_perm_hw_address_changed().await;
        let mut mode_changed = proxy.receive_mode_changed().await;
        let mut bitrate_changed = proxy.receive_bitrate_changed().await;
        let mut access_points_changed = proxy.receive_access_points_changed().await;
        let mut active_access_point_changed = proxy.receive_active_access_point_changed().await;
        let mut wireless_capabilities_changed = proxy.receive_wireless_capabilities_changed().await;
        let mut last_scan_changed = proxy.receive_last_scan_changed().await;

        loop {
            let Some(device) = weak.upgrade() else {
                debug!("DeviceWifi dropped, stopping monitor");
                return;
            };

            tokio::select! {
                Some(change) = perm_hw_address_changed.next() => {
                    if let Ok(value) = change.get().await {
                        device.perm_hw_address.set(value);
                    }
                }
                Some(change) = mode_changed.next() => {
                    if let Ok(value) = change.get().await {
                        device.mode.set(NM80211Mode::from_u32(value));
                    }
                }
                Some(change) = bitrate_changed.next() => {
                    if let Ok(value) = change.get().await {
                        device.bitrate.set(value);
                    }
                }
                Some(change) = access_points_changed.next() => {
                    if let Ok(value) = change.get().await {
                        device.access_points.set(
                            value.into_iter().map(|p| p.to_string()).collect()
                        );
                    }
                }
                Some(change) = active_access_point_changed.next() => {
                    if let Ok(value) = change.get().await {
                        device.active_access_point.set(value.to_string());
                    }
                }
                Some(change) = wireless_capabilities_changed.next() => {
                    if let Ok(value) = change.get().await {
                        device.wireless_capabilities.set(value);
                    }
                }
                Some(change) = last_scan_changed.next() => {
                    if let Ok(value) = change.get().await {
                        device.last_scan.set(value);
                    }
                }
                else => {
                    debug!("All property streams ended for DeviceWifi");
                    break;
                }
            }

            drop(device);
        }

        debug!("Property monitoring ended for DeviceWifi");
    }
}
