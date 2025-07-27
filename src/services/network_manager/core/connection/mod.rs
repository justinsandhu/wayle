use crate::services::{
    common::Property,
    network_manager::{NMActivationStateFlags, NMActiveConnectionState},
};

#[derive(Debug, Clone)]
pub struct ActiveConnection {
    /// The path of the connection object that this ActiveConnection is using.
    pub connection: Property<String>,

    /// A specific object associated with the active connection. This property reflects the
    /// specific object used during connection activation, and will not change over the
    /// lifetime of the ActiveConnection once set.
    pub specific_object: Property<String>,

    /// The ID of the connection, provided for convenience.
    pub id: Property<String>,

    /// The UUID of the connection, provided for convenience.
    pub uuid: Property<String>,

    /// The type of the connection, provided for convenience.
    pub type_: Property<String>,

    /// Array of object paths representing devices which are part of this active
    /// connection.
    pub devices: Property<Vec<String>>,

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
    pub ip4_config: Property<String>,

    /// Object path of the Dhcp4Config object describing the DHCP options returned by the
    /// DHCP server (assuming the connection used DHCP). Only valid when the connection is
    /// in the NM_ACTIVE_CONNECTION_STATE_ACTIVATED state.
    pub dhcp4_config: Property<String>,

    /// Whether this active connection is the default IPv6 connection, i.e. whether it
    /// currently owns the default IPv6 route.
    pub default6: Property<bool>,

    /// Object path of the Ip6Config object describing the configuration of the
    /// connection. Only valid when the connection is in the
    /// NM_ACTIVE_CONNECTION_STATE_ACTIVATED state.
    pub ip6_config: Property<String>,

    /// Object path of the Dhcp6Config object describing the DHCP options returned by the
    /// DHCP server (assuming the connection used DHCP). Only valid when the connection is
    /// in the NM_ACTIVE_CONNECTION_STATE_ACTIVATED state.
    pub dhcp6_config: Property<String>,

    /// Whether this active connection is also a VPN connection.
    pub vpn: Property<bool>,

    /// The path to the controller device if the connection is a port.
    pub controller: Property<String>,
}
