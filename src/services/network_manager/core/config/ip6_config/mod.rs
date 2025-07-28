use std::net::Ipv6Addr;
use zbus::zvariant::OwnedObjectPath;

/// IPv6 Configuration Set.
///
/// Represents the current IPv6 configuration including addresses, routes,
/// DNS servers, and other network parameters.
#[derive(Debug, Clone, Default)]
pub struct Ip6Config {
    /// D-Bus object path for this IP6Config
    pub path: OwnedObjectPath,

    /// Array of IP address data objects.
    pub address_data: Vec<Ipv6Address>,

    /// The gateway in use.
    pub gateway: Option<Ipv6Addr>,

    /// The nameservers in use.
    pub nameservers: Vec<Ipv6Addr>,

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
    pub route_data: Vec<Ipv6Route>,
}

/// IPv6 address with prefix length
#[derive(Debug, Clone, PartialEq)]
pub struct Ipv6Address {
    /// The IPv6 address.
    pub address: Ipv6Addr,
    /// Network prefix length in bits (0-128).
    pub prefix: u8,
}

/// IPv6 route entry
#[derive(Debug, Clone, PartialEq)]
pub struct Ipv6Route {
    /// Destination network address.
    pub destination: Ipv6Addr,
    /// Network prefix length in bits (0-128).
    pub prefix: u8,
    /// Gateway address for this route, if any.
    pub next_hop: Option<Ipv6Addr>,
    /// Route metric for priority ordering (lower is higher priority).
    pub metric: Option<u32>,
}
