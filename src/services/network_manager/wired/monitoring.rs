pub(crate) struct WiredMonitor;

use crate::services::network_manager::{
    NetworkError, core::device::wired::DeviceWired, wired_proxy::DeviceWiredProxy,
};
use tokio::task::JoinHandle;
use zbus::Connection;

impl WiredMonitor {
    pub async fn start(
        connection: Connection,
        device: &DeviceWired,
    ) -> Result<JoinHandle<()>, NetworkError> {
        let _ethernet_proxy = DeviceWiredProxy::new(&connection, device.object_path.clone())
            .await
            .map_err(NetworkError::DbusError)?;

        let handle = Self::spawn_monitoring_task(connection);

        Ok(handle)
    }

    fn spawn_monitoring_task(_connection: Connection) -> JoinHandle<()> {
        tokio::spawn(async move {
            // TODO: Implement wired device monitoring
        })
    }
}
