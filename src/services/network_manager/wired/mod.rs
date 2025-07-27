use std::ops::Deref;

use crate::services::common::Property;

use super::{NetworkStatus, core::device::wired::DeviceWired};

pub struct Wired {
    device: DeviceWired,

    pub connectivity: Property<NetworkStatus>,
}

impl Deref for Wired {
    type Target = DeviceWired;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

impl Wired {
    pub fn from_device(device: DeviceWired) -> Self {
        Self {
            device,
            connectivity: Property::new(NetworkStatus::Disconnected),
        }
    }
}
