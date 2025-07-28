use std::ops::Deref;

use tracing::warn;
use zbus::{Connection, zvariant::OwnedObjectPath};

use crate::services::{
    common::{Property, types::ObjectPath},
    network_manager::{
        NMDeviceType,
        proxy::devices::{DeviceProxy, wireless::DeviceWirelessProxy},
        types::NM80211Mode,
    },
};

use super::Device;

/// Bitrate in kilobits per second (Kb/s).
pub type BitrateKbps = u32;

/// Timestamp in CLOCK_BOOTTIME milliseconds.
pub type BootTimeMs = i64;

/// Wireless device capabilities.
pub type WirelessCapabilities = u32;

/// Wireless (Wi-Fi) network device.
///
/// Provides access to wireless-specific properties like access points, signal
/// strength, and scanning while inheriting all base device properties through Deref.
#[derive(Debug, Clone)]
pub struct DeviceWifi {
    base: Device,

    /// Permanent hardware address of the device.
    pub perm_hw_address: Property<String>,

    /// The operating mode of the wireless device.
    pub mode: Property<NM80211Mode>,

    /// The bit rate currently used by the wireless device, in kilobits/second (Kb/s).
    pub bitrate: Property<BitrateKbps>,

    /// List of object paths of access points visible to this wireless device.
    pub access_points: Property<Vec<ObjectPath>>,

    /// Object path of the access point currently used by the wireless device.
    pub active_access_point: Property<ObjectPath>,

    /// The capabilities of the wireless device.
    pub wireless_capabilities: Property<WirelessCapabilities>,

    /// The timestamp (in CLOCK_BOOTTIME milliseconds) for the last finished network scan.
    /// A value of -1 means the device never scanned for access points.
    pub last_scan: Property<BootTimeMs>,
}

impl Deref for DeviceWifi {
    type Target = Device;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DeviceWifi {
    /// Create a new wireless device from a D-Bus path.
    pub async fn from_path(path: OwnedObjectPath) -> Option<Self> {
        let connection = Connection::session().await.ok()?;
        Self::from_path_and_connection(connection, path).await
    }

    pub(crate) async fn from_path_and_connection(
        connection: Connection,
        path: OwnedObjectPath,
    ) -> Option<Self> {
        let device_proxy = DeviceProxy::builder(&connection)
            .path(path.clone())
            .ok()?
            .build()
            .await
            .ok()?;

        let device_type = device_proxy.device_type().await.ok()?;
        if device_type != NMDeviceType::Wifi as u32 {
            warn!("Device at {path} is not a wifi device");
            return None;
        }

        let wifi_proxy = DeviceWirelessProxy::builder(&connection)
            .path(path.clone())
            .ok()?
            .build()
            .await
            .ok()?;

        let base = Device::from_proxy(&device_proxy).await?;

        let (
            perm_hw_address,
            mode,
            bitrate,
            access_points,
            active_access_point,
            wireless_capabilities,
            last_scan,
        ) = tokio::join!(
            wifi_proxy.perm_hw_address(),
            wifi_proxy.mode(),
            wifi_proxy.bitrate(),
            wifi_proxy.access_points(),
            wifi_proxy.active_access_point(),
            wifi_proxy.wireless_capabilities(),
            wifi_proxy.last_scan(),
        );

        Some(Self {
            base,
            perm_hw_address: Property::new(perm_hw_address.ok().unwrap_or_default()),
            mode: Property::new(NM80211Mode::from_u32(mode.ok().unwrap_or(0))),
            bitrate: Property::new(bitrate.ok().unwrap_or(0)),
            access_points: Property::new(
                access_points
                    .ok()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|p| p.to_string())
                    .collect(),
            ),
            active_access_point: Property::new(
                active_access_point
                    .ok()
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| "/".to_string()),
            ),
            wireless_capabilities: Property::new(wireless_capabilities.ok().unwrap_or(0)),
            last_scan: Property::new(last_scan.ok().unwrap_or(-1)),
        })
    }
}
