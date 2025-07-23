//! NetworkManager device types.

/// Types of network devices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NMDeviceType {
    Unknown = 0,
    Ethernet = 1,
    Wifi = 2,
    Unused1 = 3,
    Unused2 = 4,
    Bt = 5,
    OlpcMesh = 6,
    Wimax = 7,
    Modem = 8,
    Infiniband = 9,
    Bond = 10,
    Vlan = 11,
    Adsl = 12,
    Bridge = 13,
    Loopback = 14,
    Team = 15,
    Tun = 16,
    IpTunnel = 17,
    Macvlan = 18,
    Vxlan = 19,
    Veth = 20,
    Macsec = 21,
    Dummy = 22,
    Ppp = 23,
    OvsInterface = 24,
    OvsPort = 25,
    OvsBridge = 26,
    Wpan = 27,
    SixLowpan = 28,
    Wireguard = 29,
    WifiP2p = 30,
    Vrf = 31,
}

/// IP tunnel modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NMIPTunnelMode {
    Unknown = 0,
    Ipip = 1,
    Gre = 2,
    Sit = 3,
    Isatap = 4,
    Vti = 5,
    Ip6ip6 = 6,
    Ipip6 = 7,
    Ip6gre = 8,
    Vti6 = 9,
    Gretap = 10,
    Ip6gretap = 11,
}