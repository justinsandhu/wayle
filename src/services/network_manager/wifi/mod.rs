use std::ops::Deref;

use zbus::Connection;

use crate::services::common::Property;

use super::{NetworkStatus, core::device::wifi::DeviceWifi};

mod access_points;
mod control;
mod manager;
mod saved_connections;

pub struct Wifi {
    connection: Connection,
    device: DeviceWifi,

    pub enabled: Property<bool>,
    pub connectivity: Property<NetworkStatus>,
    pub ssid: Property<Option<String>>,
    pub strength: Property<u8>,
}

impl Deref for Wifi {
    type Target = DeviceWifi;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

impl Wifi {
    pub fn from_device(connection: Connection, device: DeviceWifi) -> Self {
        Self {
            connection,
            device,
            enabled: Property::new(false),
            connectivity: Property::new(NetworkStatus::Disconnected),
            ssid: Property::new(None),
            strength: Property::new(0),
        }
    }
}
