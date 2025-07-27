pub mod dhcp4_config;
pub mod dhcp6_config;
pub mod ip4_config;
pub mod ip6_config;

pub use dhcp4_config::Dhcp4Config;
pub use dhcp6_config::Dhcp6Config;
pub use ip4_config::{Ip4Config, Ipv4Address, Ipv4Route};
pub use ip6_config::{Ip6Config, Ipv6Address, Ipv6Route};