use std::ops::Deref;

use crate::services::{
    common::{Property, types::ObjectPath},
    network_manager::types::NM80211Mode,
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
    /// Create a new wireless device from a base device.
    pub fn new(base: Device) -> Self {
        Self {
            base,
            perm_hw_address: Property::new(String::new()),
            mode: Property::new(NM80211Mode::Unknown),
            bitrate: Property::new(0),
            access_points: Property::new(Vec::new()),
            active_access_point: Property::new(ObjectPath::from("/")),
            wireless_capabilities: Property::new(0),
            last_scan: Property::new(-1),
        }
    }
}
