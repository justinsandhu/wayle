use std::sync::Arc;

use tokio::task::JoinHandle;
use tokio_stream::StreamExt;
use zbus::Connection;

use crate::services::network_manager::{DeviceProxy, NMDeviceState, NetworkError, NetworkStatus};

use super::Wired;

pub(crate) struct WiredMonitor;

impl WiredMonitor {
    pub async fn start(
        connection: &Connection,
        wired: &Arc<Wired>,
    ) -> Result<JoinHandle<()>, NetworkError> {
        Self::spawn_monitoring_task(connection, wired).await
    }

    async fn spawn_monitoring_task(
        connection: &Connection,
        wired: &Arc<Wired>,
    ) -> Result<JoinHandle<()>, NetworkError> {
        let connectivity_prop = wired.connectivity.clone();
        let device_path = wired.device.path.get();

        let device_proxy = DeviceProxy::new(connection, device_path)
            .await
            .map_err(NetworkError::DbusError)?;

        let mut connectivity_changed = device_proxy.receive_state_changed().await;

        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(change) = connectivity_changed.next() => {
                        if let Ok(new_connectivity) = change.get().await {
                            let device_state = NMDeviceState::from_u32(new_connectivity);
                            connectivity_prop.set(NetworkStatus::from_device_state(device_state));
                        }
                    }
                    else => {
                        break;
                    }
                }
            }
        });

        Ok(handle)
    }
}
