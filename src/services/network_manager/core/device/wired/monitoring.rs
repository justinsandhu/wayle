use std::sync::{Arc, Weak};

use futures::StreamExt;
use tracing::debug;
use zbus::{Connection, zvariant::OwnedObjectPath};

use crate::services::network_manager::{NetworkError, wired_proxy::DeviceWiredProxy};

use super::DeviceWired;

/// Monitors D-Bus properties and updates the reactive DeviceWired model.
pub(crate) struct DeviceWiredMonitor;

impl DeviceWiredMonitor {
    pub(super) async fn start(
        device: Arc<DeviceWired>,
        connection: &Connection,
        path: OwnedObjectPath,
    ) -> Result<(), NetworkError> {
        let weak = Arc::downgrade(&device);

        let proxy = DeviceWiredProxy::new(connection, path)
            .await
            .map_err(NetworkError::DbusError)?;

        tokio::spawn(async move {
            Self::monitor(weak, proxy).await;
        });

        Ok(())
    }

    async fn monitor(weak: Weak<DeviceWired>, proxy: DeviceWiredProxy<'static>) {
        let mut perm_hw_address_changed = proxy.receive_perm_hw_address_changed().await;
        let mut speed_changed = proxy.receive_speed_changed().await;
        let mut s390_subchannels_changed = proxy.receive_s390_subchannels_changed().await;

        loop {
            let Some(device) = weak.upgrade() else {
                debug!("DeviceWired dropped, stopping monitor");
                return;
            };

            tokio::select! {
                Some(change) = perm_hw_address_changed.next() => {
                    if let Ok(value) = change.get().await {
                        device.perm_hw_address.set(value);
                    }
                }
                Some(change) = speed_changed.next() => {
                    if let Ok(value) = change.get().await {
                        device.speed.set(value);
                    }
                }
                Some(change) = s390_subchannels_changed.next() => {
                    if let Ok(value) = change.get().await {
                        device.s390_subchannels.set(value);
                    }
                }
                else => {
                    debug!("All property streams ended for DeviceWired");
                    break;
                }
            }

            drop(device);
        }

        debug!("Property monitoring ended for DeviceWired");
    }
}
