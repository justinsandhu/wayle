use std::ops::Deref;

use zbus::Connection;

use crate::services::common::Property;

use super::{NetworkStatus, core::device::wired::DeviceWired};

/// Manages wired (ethernet) network connectivity and device state.
///
/// Provides interface for monitoring ethernet connection status.
/// Unlike WiFi, wired connections are typically automatic and don't
/// require manual connection management or authentication.
#[derive(Clone, Debug)]
pub struct Wired {
    pub(crate) connection: Connection,
    /// The underlying wired device.
    pub device: DeviceWired,

    /// Current wired network connectivity status.
    pub connectivity: Property<NetworkStatus>,
}

impl Deref for Wired {
    type Target = DeviceWired;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

impl PartialEq for Wired {
    fn eq(&self, other: &Self) -> bool {
        self.device.path.get() == other.device.path.get()
    }
}

impl Wired {
    pub(crate) fn from_device_and_connection(connection: Connection, device: DeviceWired) -> Self {
        Self {
            connection,
            device,
            connectivity: Property::new(NetworkStatus::Disconnected),
        }
    }
}
