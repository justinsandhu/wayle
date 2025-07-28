use std::net::Ipv4Addr;
use zbus::zvariant::OwnedObjectPath;

/// IPv4 configuration for a network device.
///
/// Represents the current IPv4 configuration including addresses, routes,
/// DNS servers, and other network parameters.
#[derive(Debug, Clone, Default)]
pub struct Ip4Config {
    /// D-Bus object path for this IP4Config
    pub path: OwnedObjectPath,

    /// Array of IP address data objects.
    pub address_data: Vec<Ipv4Address>,

    /// The gateway in use.
    pub gateway: Option<Ipv4Addr>,

    /// Array of nameserver data objects.
    pub nameserver_data: Vec<Ipv4Addr>,

    /// A list of domains this address belongs to.
    pub domains: Vec<String>,

    /// A list of dns searches.
    pub searches: Vec<String>,

    /// A list of DNS options that modify the behavior of the DNS resolver.
    /// See resolv.conf(5) manual page for the list of supported options.
    pub dns_options: Vec<String>,

    /// The relative priority of DNS servers.
    pub dns_priority: i32,

    /// Array of IP route data objects.
    pub route_data: Vec<Ipv4Route>,

    /// The Windows Internet Name Service servers associated with the connection.
    pub wins_server_data: Vec<Ipv4Addr>,
}

/// IPv4 address with prefix length
#[derive(Debug, Clone, PartialEq)]
pub struct Ipv4Address {
    /// The IPv4 address.
    pub address: Ipv4Addr,
    /// Network prefix length in bits (0-32).
    pub prefix: u8,
}

/// IPv4 route entry
#[derive(Debug, Clone, PartialEq)]
pub struct Ipv4Route {
    /// Destination network address.
    pub destination: Ipv4Addr,
    /// Network prefix length in bits (0-32).
    pub prefix: u8,
    /// Gateway address for this route, if any.
    pub next_hop: Option<Ipv4Addr>,
    /// Route metric for priority ordering (lower is higher priority).
    pub metric: Option<u32>,
}
