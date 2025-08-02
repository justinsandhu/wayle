use std::sync::Arc;

use tokio::task::JoinHandle;
use tokio_stream::StreamExt;
use zbus::{Connection, zvariant::OwnedObjectPath};

use crate::services::{
    common::Property,
    network_manager::{
        AccessPoint, NetworkError,
        core::device::wifi::DeviceWifi,
        wireless::{AccessPointAddedStream, AccessPointRemovedStream, DeviceWirelessProxy},
    },
};

pub(crate) struct WifiMonitoring;

impl WifiMonitoring {
    pub async fn start(
        connection: Connection,
        device: &DeviceWifi,
        access_points: &Property<Vec<Arc<AccessPoint>>>,
    ) -> Result<JoinHandle<()>, NetworkError> {
        let wireless_proxy = DeviceWirelessProxy::new(&connection, device.object_path.clone())
            .await
            .map_err(NetworkError::DbusError)?;

        let ap_added = wireless_proxy
            .receive_access_point_added()
            .await
            .map_err(NetworkError::DbusError)?;
        let ap_removed = wireless_proxy
            .receive_access_point_removed()
            .await
            .map_err(NetworkError::DbusError)?;

        Self::populate_existing_access_points(&connection, device, access_points).await;

        let handle =
            Self::spawn_monitoring_task(connection, access_points.clone(), ap_added, ap_removed);

        Ok(handle)
    }

    async fn populate_existing_access_points(
        connection: &Connection,
        device: &DeviceWifi,
        access_points: &Property<Vec<Arc<AccessPoint>>>,
    ) {
        let existing_paths = device.access_points.get();
        let mut initial_aps = Vec::with_capacity(existing_paths.len());

        for ap_path in existing_paths {
            let Ok(path) = OwnedObjectPath::try_from(ap_path.as_str()) else {
                continue;
            };

            if let Some(ap) = AccessPoint::get_live(connection.clone(), path).await {
                initial_aps.push(ap);
            }
        }

        if !initial_aps.is_empty() {
            access_points.set(initial_aps);
        }
    }

    fn spawn_monitoring_task(
        connection: Connection,
        access_points: Property<Vec<Arc<AccessPoint>>>,
        mut ap_added: AccessPointAddedStream,
        mut ap_removed: AccessPointRemovedStream,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(added) = ap_added.next() => {
                        let Ok(args) = added.args() else { continue };

                        let Some(new_ap) = AccessPoint::get_live(connection.clone(), args.access_point).await else {
                            continue;
                        };

                        let mut aps = access_points.get();
                        aps.push(new_ap);
                        access_points.set(aps);
                    }

                    Some(removed) = ap_removed.next() => {
                        let Ok(args) = removed.args() else { continue };

                        let mut aps = access_points.get();
                        aps.retain(|ap| ap.path != args.access_point);
                        access_points.set(aps);
                    }

                    else => {
                        break;
                    }
                }
            }
        })
    }
}
