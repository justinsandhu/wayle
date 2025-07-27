use std::ops::Deref;

use crate::services::common::Property;

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
    /// Create a new wired device from a base device.
    pub fn new(base: Device) -> Self {
        Self {
            base,
            perm_hw_address: Property::new(String::new()),
            speed: Property::new(0),
            s390_subchannels: Property::new(Vec::new()),
        }
    }
}
