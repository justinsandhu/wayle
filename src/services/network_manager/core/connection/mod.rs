mod monitoring;
use std::sync::Arc;

use monitoring::ActiveConnectionMonitor;
use tracing::warn;
use zbus::{Connection, zvariant::OwnedObjectPath};

use crate::services::{
    common::Property,
    network_manager::{ConnectionActiveProxy, NMActivationStateFlags, NMActiveConnectionState},
};

/// Active network connection in NetworkManager.
///
/// Tracks state and configuration of currently active connections,
/// including devices, IP configuration, and connection properties.
/// Properties update reactively as connection state changes.
#[derive(Debug, Clone)]
pub struct ActiveConnection {
    /// The zbus connection
    connection: Connection,
    /// The object path of the active connection
    pub(crate) path: Property<OwnedObjectPath>,

    /// The path of the connection object that this ActiveConnection is using.
    pub connection_path: Property<OwnedObjectPath>,

    /// Specific object associated with the active connection. Reflects the
    /// object used during connection activation, and will not change over the
    /// lifetime of the ActiveConnection once set.
    pub specific_object: Property<OwnedObjectPath>,

    /// The ID of the connection, provided for convenience.
    pub id: Property<String>,

    /// The UUID of the connection, provided for convenience.
    pub uuid: Property<String>,

    /// The type of the connection, provided for convenience.
    pub type_: Property<String>,

    /// Array of object paths representing devices which are part of this active
    /// connection.
    pub devices: Property<Vec<OwnedObjectPath>>,

    /// The state of this active connection.
    pub state: Property<NMActiveConnectionState>,

    /// The state flags of this active connection. See NMActivationStateFlags.
    pub state_flags: Property<NMActivationStateFlags>,

    /// Whether this active connection is the default IPv4 connection, i.e. whether it
    /// currently owns the default IPv4 route.
    pub default: Property<bool>,

    /// Object path of the Ip4Config object describing the configuration of the
    /// connection. Only valid when the connection is in the
    /// NM_ACTIVE_CONNECTION_STATE_ACTIVATED state.
    pub ip4_config: Property<OwnedObjectPath>,

    /// Object path of the Dhcp4Config object describing the DHCP options returned by the
    /// DHCP server (assuming the connection used DHCP). Only valid when the connection is
    /// in the NM_ACTIVE_CONNECTION_STATE_ACTIVATED state.
    pub dhcp4_config: Property<OwnedObjectPath>,

    /// Whether this active connection is the default IPv6 connection, i.e. whether it
    /// currently owns the default IPv6 route.
    pub default6: Property<bool>,

    /// Object path of the Ip6Config object describing the configuration of the
    /// connection. Only valid when the connection is in the
    /// NM_ACTIVE_CONNECTION_STATE_ACTIVATED state.
    pub ip6_config: Property<OwnedObjectPath>,

    /// Object path of the Dhcp6Config object describing the DHCP options returned by the
    /// DHCP server (assuming the connection used DHCP). Only valid when the connection is
    /// in the NM_ACTIVE_CONNECTION_STATE_ACTIVATED state.
    pub dhcp6_config: Property<OwnedObjectPath>,

    /// Whether this active connection is also a VPN connection.
    pub vpn: Property<bool>,

    /// The path to the controller device if the connection is a port.
    pub controller: Property<OwnedObjectPath>,
}

impl ActiveConnection {
    /// Get a snapshot of the current connection state (no monitoring).
    pub async fn get(connection: Connection, path: OwnedObjectPath) -> Option<Arc<Self>> {
        Self::create_from_path(connection, path).await
    }

    /// Get a live-updating connection instance (with monitoring).
    pub async fn get_live(connection: Connection, path: OwnedObjectPath) -> Option<Arc<Self>> {
        let active_connection = Self::create_from_path(connection.clone(), path.clone()).await?;

        ActiveConnectionMonitor::start(active_connection.clone(), connection, path).await;

        Some(active_connection)
    }

    async fn create_from_path(connection: Connection, path: OwnedObjectPath) -> Option<Arc<Self>> {
        let connection_proxy = ConnectionActiveProxy::new(&connection, path.clone())
            .await
            .ok()?;

        if connection_proxy.connection().await.is_err() {
            warn!(
                "Active Connection at path '{}' does not exist.",
                path.clone()
            );
            return None;
        }

        let (
            connection_path,
            specific_object,
            id,
            uuid,
            type_,
            devices,
            state,
            state_flags,
            default,
            ip4_config,
            dhcp4_config,
            default6,
            ip6_config,
            dhcp6_config,
            vpn,
            controller,
        ) = tokio::join!(
            connection_proxy.connection(),
            connection_proxy.specific_object(),
            connection_proxy.id(),
            connection_proxy.uuid(),
            connection_proxy.type_(),
            connection_proxy.devices(),
            connection_proxy.state(),
            connection_proxy.state_flags(),
            connection_proxy.default(),
            connection_proxy.ip4_config(),
            connection_proxy.dhcp4_config(),
            connection_proxy.default6(),
            connection_proxy.ip6_config(),
            connection_proxy.dhcp6_config(),
            connection_proxy.vpn(),
            connection_proxy.controller(),
        );

        let connection_path = connection_path.unwrap_or_default();
        let specific_object = specific_object.unwrap_or_default();
        let id = id.unwrap_or_default();
        let uuid = uuid.unwrap_or_default();
        let type_ = type_.unwrap_or_default();
        let devices = devices.unwrap_or_default();
        let state = NMActiveConnectionState::from_u32(state.unwrap_or_default());
        let state_flags =
            NMActivationStateFlags::from_bits_truncate(state_flags.unwrap_or_default());
        let default = default.unwrap_or_default();
        let ip4_config = ip4_config.unwrap_or_default();
        let dhcp4_config = dhcp4_config.unwrap_or_default();
        let default6 = default6.unwrap_or_default();
        let ip6_config = ip6_config.unwrap_or_default();
        let dhcp6_config = dhcp6_config.unwrap_or_default();
        let vpn = vpn.unwrap_or_default();
        let controller = controller.unwrap_or_default();

        Some(Arc::new(Self {
            connection,
            connection_path: Property::new(connection_path),
            path: Property::new(path),
            specific_object: Property::new(specific_object),
            id: Property::new(id),
            uuid: Property::new(uuid),
            type_: Property::new(type_),
            devices: Property::new(devices),
            state: Property::new(state),
            state_flags: Property::new(state_flags),
            default: Property::new(default),
            ip4_config: Property::new(ip4_config),
            dhcp4_config: Property::new(dhcp4_config),
            default6: Property::new(default6),
            ip6_config: Property::new(ip6_config),
            dhcp6_config: Property::new(dhcp6_config),
            vpn: Property::new(vpn),
            controller: Property::new(controller),
        }))
    }
}
