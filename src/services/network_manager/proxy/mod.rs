//! NetworkManager D-Bus proxy definitions.
//!
//! This module provides type-safe proxy interfaces for all NetworkManager D-Bus objects.

#![allow(missing_docs)]
pub mod access_point;
pub mod active_connection;
pub mod agent_manager;
pub mod checkpoint;
pub mod devices;
pub mod dhcp4_config;
pub mod dhcp6_config;
pub mod dns_manager;
pub mod ip4_config;
pub mod ip6_config;
pub mod manager;
pub mod ppp;
pub mod settings;
pub mod wifi_p2p_peer;

pub use access_point::*;
pub use active_connection::*;
pub use agent_manager::*;
pub use checkpoint::*;
pub use devices::{
    DeviceProxy, DeviceProxyBlocking, StateChanged as DeviceStateChanged,
    StateChangedArgs as DeviceStateChangedArgs, StateChangedIterator as DeviceStateChangedIterator,
    StateChangedStream as DeviceStateChangedStream, adsl, bluetooth, bond, bridge, dummy, generic,
    hsr, infiniband, ip_tunnel, ipvlan, loopback, lowpan, macsec, macvlan, modem, olpc_mesh,
    ovs_bridge, ovs_interface, ovs_port, ppp as devices_ppp, statistics, team, tun, veth, vlan,
    vrf, vxlan, wifi_p2p, wired as wired_proxy, wireguard, wireless, wpan,
};
pub use dhcp4_config::*;
pub use dhcp6_config::*;
pub use dns_manager::*;
pub use ip4_config::*;
pub use ip6_config::*;
pub use manager::{
    NetworkManagerProxy, NetworkManagerProxyBlocking, StateChanged as NMStateChanged,
    StateChangedArgs as NMStateChangedArgs, StateChangedIterator as NMStateChangedIterator,
    StateChangedStream as NMStateChangedStream,
};
pub use ppp::*;
pub use settings::*;
pub use wifi_p2p_peer::*;
