use std::collections::HashMap;
use std::sync::Arc;
use zbus::{
    Connection,
    zvariant::{OwnedObjectPath, OwnedValue},
};

use crate::services::{
    common::Property,
    network_manager::{NetworkError, proxy::dhcp4_config::DHCP4ConfigProxy},
};

/// IPv4 DHCP Client State.
///
/// This corresponds to the org.freedesktop.NetworkManager.DHCP4Config interface which
/// provides access to configuration options returned by the IPv4 DHCP server.
#[derive(Debug, Clone)]
pub struct Dhcp4Config {
    /// D-Bus object path for this DHCP4 configuration
    pub path: Property<OwnedObjectPath>,

    /// Configuration options returned by the DHCP server.
    pub options: Property<HashMap<String, OwnedValue>>,
}

impl Dhcp4Config {
    /// Get a snapshot of the current DHCP4 configuration state (no monitoring).
    pub async fn get(connection: &Connection, path: OwnedObjectPath) -> Option<Arc<Self>> {
        let config = Self::from_path(connection, path).await?;
        Some(Arc::new(config))
    }

    async fn from_path(connection: &Connection, path: OwnedObjectPath) -> Option<Self> {
        let options = Self::fetch_options(connection, &path).await.ok()?;
        Some(Self::from_options(path, options))
    }

    async fn fetch_options(
        connection: &Connection,
        path: &OwnedObjectPath,
    ) -> Result<HashMap<String, OwnedValue>, NetworkError> {
        let proxy = DHCP4ConfigProxy::new(connection, path.clone())
            .await
            .map_err(NetworkError::DbusError)?;

        let options = proxy.options().await.map_err(NetworkError::DbusError)?;

        let mut converted = HashMap::new();
        for (key, value) in options {
            match OwnedValue::try_from(&value) {
                Ok(owned_value) => {
                    converted.insert(key, owned_value);
                }
                Err(_) => {
                    return Err(NetworkError::InitializationFailed(format!(
                        "Failed to convert DHCP option '{}'",
                        key
                    )));
                }
            }
        }
        Ok(converted)
    }

    fn from_options(path: OwnedObjectPath, options: HashMap<String, OwnedValue>) -> Self {
        Self {
            path: Property::new(path),
            options: Property::new(options),
        }
    }
}
