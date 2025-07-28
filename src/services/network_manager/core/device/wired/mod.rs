use std::ops::Deref;

use tracing::warn;
use zbus::{Connection, zvariant::OwnedObjectPath};

use crate::services::{
    common::Property,
    network_manager::{DeviceProxy, NMDeviceType, wired_proxy::DeviceWiredProxy},
};

use super::Device;

/// Speed in megabits per second (Mb/s).
pub type SpeedMbps = u32;

/// Wired (Ethernet) network device.
///
/// Provides access to wired-specific properties like link speed and permanent
/// hardware address while inheriting all base device properties through Deref.
#[derive(Debug, Clone)]
pub struct DeviceWired {
    base: Device,

    /// Permanent hardware address of the device.
    pub perm_hw_address: Property<String>,

    /// Design speed of the device, in megabits/second (Mb/s).
    pub speed: Property<SpeedMbps>,

    /// Array of S/390 subchannels for S/390 or z/Architecture devices.
    pub s390_subchannels: Property<Vec<String>>,
}

impl Deref for DeviceWired {
    type Target = Device;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DeviceWired {
    /// Create a new wired device from a D-Bus path.
    pub async fn from_path(path: OwnedObjectPath) -> Option<Self> {
        let connection = Connection::session().await.ok()?;
        Self::from_path_and_connection(connection, path).await
    }

    pub(crate) async fn from_path_and_connection(
        connection: Connection,
        path: OwnedObjectPath,
    ) -> Option<Self> {
        let device_proxy = DeviceProxy::new(&connection, path.clone()).await.ok()?;

        let device_type = device_proxy.device_type().await.ok()?;
        if device_type != NMDeviceType::Ethernet as u32 {
            warn!("Device at {path} is not an ethernet device");
            return None;
        }

        let wired_proxy = DeviceWiredProxy::new(&connection, path.clone())
            .await
            .ok()?;
        let base = Device::from_proxy(&device_proxy).await?;

        let (perm_hw_address, speed, s390_subchannels) = tokio::join!(
            wired_proxy.perm_hw_address(),
            wired_proxy.speed(),
            wired_proxy.s390_subchannels(),
        );

        Some(Self {
            base,
            perm_hw_address: Property::new(perm_hw_address.ok().unwrap_or_default()),
            speed: Property::new(speed.ok().unwrap_or_default()),
            s390_subchannels: Property::new(s390_subchannels.ok().unwrap_or_default()),
        })
    }
}
