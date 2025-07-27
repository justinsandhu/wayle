use tracing::instrument;
use zbus::Connection;

use crate::services::{common::Property, network_manager::discovery::NetworkServiceDiscovery};

use super::{ConnectionType, NetworkError, Wifi, Wired};

pub struct NetworkService {
    zbus_connection: Connection,
    wifi: Option<Wifi>,
    wired: Option<Wired>,
    primary: Property<ConnectionType>,
}

impl NetworkService {
    pub async fn new() -> Result<Self, NetworkError> {
        Self::start().await
    }

    #[instrument]
    pub async fn start() -> Result<Self, NetworkError> {
        let connection = Connection::session().await.map_err(|err| {
            NetworkError::InitializationFailed(format!("D-Bus connection failed: {err}"))
        })?;

        let _wifi_device = NetworkServiceDiscovery::wifi_device_path(&connection).await?;
        let _wired_device = NetworkServiceDiscovery::wired_device_path(&connection).await?;

        let _service = Self {
            zbus_connection: connection,
            wifi: None,  // TODO: Create Wifi from device path
            wired: None, // TODO: Create Wired from device path
            primary: Property::new(ConnectionType::Unknown),
        };
        todo!()
    }
}
