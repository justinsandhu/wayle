mod monitoring;

use std::ops::Deref;
use std::sync::Arc;

use monitoring::DeviceWiredMonitor;
use zbus::{Connection, zvariant::OwnedObjectPath};

use crate::services::{
    common::Property,
    network_manager::{DeviceProxy, NMDeviceType, NetworkError, wired_proxy::DeviceWiredProxy},
};

use super::Device;

/// Speed in megabits per second (Mb/s).
pub type SpeedMbps = u32;

/// Wired-specific properties fetched from D-Bus
struct WiredProperties {
    perm_hw_address: String,
    speed: u32,
    s390_subchannels: Vec<String>,
}

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
    /// Get a snapshot of the current wired device state (no monitoring).
    pub async fn get(connection: Connection, device_path: OwnedObjectPath) -> Option<Arc<Self>> {
        let device = Self::from_path(connection, device_path).await?;
        Some(Arc::new(device))
    }

    /// Get a live-updating wired device instance (with monitoring).
    ///
    /// Fetches current state and starts monitoring for updates.
    ///
    /// # Errors
    ///
    /// Returns `NetworkError::InitializationFailed` if:
    /// - Device at path is not a wired (ethernet) device
    /// - Failed to fetch wired properties from D-Bus
    /// - Failed to start monitoring
    pub async fn get_live(
        connection: Connection,
        device_path: OwnedObjectPath,
    ) -> Result<Arc<Self>, NetworkError> {
        Self::verify_is_ethernet_device(&connection, &device_path).await?;

        let base_device = Device::get_live(connection.clone(), device_path.clone()).await?;
        let base = Device::clone(&base_device);
        let wired_props = Self::fetch_wired_properties(&connection, &device_path).await?;
        let device = Arc::new(Self::from_props(base, wired_props));

        DeviceWiredMonitor::start(device.clone(), connection, device_path).await?;

        Ok(device)
    }

    async fn verify_is_ethernet_device(
        connection: &Connection,
        device_path: &OwnedObjectPath,
    ) -> Result<(), NetworkError> {
        let device_proxy = DeviceProxy::new(connection, device_path.clone())
            .await
            .map_err(NetworkError::DbusError)?;

        let device_type = device_proxy
            .device_type()
            .await
            .map_err(NetworkError::DbusError)?;

        if device_type != NMDeviceType::Ethernet as u32 {
            return Err(NetworkError::InitializationFailed(format!(
                "Device at {device_path} is not an ethernet device"
            )));
        }

        Ok(())
    }

    async fn fetch_wired_properties(
        connection: &Connection,
        device_path: &OwnedObjectPath,
    ) -> Result<WiredProperties, NetworkError> {
        let wired_proxy = DeviceWiredProxy::new(connection, device_path.clone())
            .await
            .map_err(NetworkError::DbusError)?;

        let (perm_hw_address, speed, s390_subchannels) = tokio::join!(
            wired_proxy.perm_hw_address(),
            wired_proxy.speed(),
            wired_proxy.s390_subchannels(),
        );

        Ok(WiredProperties {
            perm_hw_address: perm_hw_address.map_err(NetworkError::DbusError)?,
            speed: speed.map_err(NetworkError::DbusError)?,
            s390_subchannels: s390_subchannels.map_err(NetworkError::DbusError)?,
        })
    }

    fn from_props(base: Device, props: WiredProperties) -> Self {
        Self {
            base,
            perm_hw_address: Property::new(props.perm_hw_address),
            speed: Property::new(props.speed),
            s390_subchannels: Property::new(props.s390_subchannels),
        }
    }

    async fn from_path(connection: Connection, path: OwnedObjectPath) -> Option<Self> {
        Self::verify_is_ethernet_device(&connection, &path)
            .await
            .ok()?;

        let base = Device::from_path(connection.clone(), path.to_string()).await?;
        let wired_props = Self::fetch_wired_properties(&connection, &path)
            .await
            .ok()?;

        Some(Self::from_props(base, wired_props))
    }
}
