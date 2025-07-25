//! NetworkManager device types.

/// NMDeviceType values indicate the type of hardware represented by a device object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NMDeviceType {
    /// unknown device
    Unknown = 0,
    /// a wired ethernet device
    Ethernet = 1,
    /// an 802.11 Wi-Fi device
    Wifi = 2,
    /// not used
    Unused1 = 3,
    /// not used
    Unused2 = 4,
    /// a Bluetooth device supporting PAN or DUN access protocols
    Bt = 5,
    /// an OLPC XO mesh networking device
    OlpcMesh = 6,
    /// an 802.16e Mobile WiMAX broadband device
    Wimax = 7,
    /// a modem supporting analog telephone, CDMA/EVDO, GSM/UMTS, or LTE network access
    /// protocols
    Modem = 8,
    /// an IP-over-InfiniBand device
    Infiniband = 9,
    /// a bond master interface
    Bond = 10,
    /// an 802.1Q VLAN interface
    Vlan = 11,
    /// ADSL modem
    Adsl = 12,
    /// a bridge master interface
    Bridge = 13,
    /// generic support for unrecognized device types
    Generic = 14,
    /// a team master interface
    Team = 15,
    /// a TUN or TAP interface
    Tun = 16,
    /// a IP tunnel interface
    IpTunnel = 17,
    /// a MACVLAN interface
    Macvlan = 18,
    /// a VXLAN interface
    Vxlan = 19,
    /// a VETH interface
    Veth = 20,
    /// a MACsec interface
    Macsec = 21,
    /// a dummy interface
    Dummy = 22,
    /// a PPP interface
    Ppp = 23,
    /// a Open vSwitch interface
    OvsInterface = 24,
    /// a Open vSwitch port
    OvsPort = 25,
    /// a Open vSwitch bridge
    OvsBridge = 26,
    /// a IEEE 802.15.4 (WPAN) MAC Layer Device
    Wpan = 27,
    /// 6LoWPAN interface
    SixLowpan = 28,
    /// a WireGuard interface
    Wireguard = 29,
    /// an 802.11 Wi-Fi P2P device. Since: 1.16.
    WifiP2p = 30,
    /// A VRF (Virtual Routing and Forwarding) interface. Since: 1.24.
    Vrf = 31,
}

/// The tunneling mode.
///
/// Since: 1.2
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NMIPTunnelMode {
    /// Unknown/unset tunnel mode
    Unknown = 0,
    /// IP in IP tunnel
    Ipip = 1,
    /// GRE tunnel
    Gre = 2,
    /// SIT tunnel
    Sit = 3,
    /// ISATAP tunnel
    Isatap = 4,
    /// VTI tunnel
    Vti = 5,
    /// IPv6 in IPv6 tunnel
    Ip6ip6 = 6,
    /// IPv4 in IPv6 tunnel
    Ipip6 = 7,
    /// IPv6 GRE tunnel
    Ip6gre = 8,
    /// IPv6 VTI tunnel
    Vti6 = 9,
    /// GRETAP tunnel
    Gretap = 10,
    /// IPv6 GRETAP tunnel
    Ip6gretap = 11,
}

/// LLDP (Link Layer Discovery Protocol) neighbor information.
///
/// Contains information advertised by directly connected network devices
/// using the LLDP protocol. Used for network topology discovery.
#[derive(Debug, Clone)]
pub struct LldpNeighbor {
    pub chassis_id: Option<String>,
    pub port_id: Option<String>,
    pub port_description: Option<String>,
    pub system_name: Option<String>,
    pub system_description: Option<String>,
    pub system_capabilities: Option<u32>,
    pub management_addresses: Option<Vec<String>>,
}
