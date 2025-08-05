mod monitoring;

use super::{NetworkError, NetworkStatus, core::device::wired::DeviceWired};
use crate::services::common::Property;
use monitoring::WiredMonitor;
use std::ops::Deref;
use std::sync::Arc;
use zbus::{Connection, zvariant::OwnedObjectPath};

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
    /// Get a snapshot of the current wired state (no monitoring).
    ///
    /// # Errors
    ///
    /// Returns `NetworkError::InitializationFailed` if the wired device cannot be created
    pub async fn get(
        connection: Connection,
        device_path: OwnedObjectPath,
    ) -> Result<Arc<Self>, NetworkError> {
        let device_arc = DeviceWired::get(connection.clone(), device_path)
            .await
            .ok_or_else(|| {
                NetworkError::InitializationFailed("Failed to create wired device".into())
            })?;
        let device = DeviceWired::clone(&device_arc);

        let wired = Self::create_from_device(connection, device).await?;
        Ok(Arc::new(wired))
    }

    /// Get a live-updating wired instance (with monitoring).
    ///
    /// Fetches the device, current state and starts monitoring for updates.
    ///
    /// # Errors
    ///
    /// Returns `NetworkError::InitializationFailed` if:
    /// - The wired device cannot be created
    /// - Failed to start monitoring
    pub async fn get_live(
        connection: Connection,
        device_path: OwnedObjectPath,
    ) -> Result<Arc<Self>, NetworkError> {
        let device_arc = DeviceWired::get_live(connection.clone(), device_path).await?;
        let device = DeviceWired::clone(&device_arc);

        let wired = Self::create_from_device(connection.clone(), device.clone()).await?;
        let wired_arc = Arc::new(wired);

        let _monitoring_handle = WiredMonitor::start(connection, &wired_arc).await?;

        Ok(wired_arc)
    }

    async fn create_from_device(
        connection: Connection,
        device: DeviceWired,
    ) -> Result<Self, NetworkError> {
        let device_state = &device.state.get();
        let connectivity = NetworkStatus::from_device_state(*device_state);

        Ok(Self {
            connection,
            device,
            connectivity: Property::new(connectivity),
        })
    }
}
