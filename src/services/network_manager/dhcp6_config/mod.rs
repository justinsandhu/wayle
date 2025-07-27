use std::collections::HashMap;
use zbus::zvariant::{OwnedObjectPath, OwnedValue};

/// IPv6 DHCP Client State.
///
/// This corresponds to the org.freedesktop.NetworkManager.DHCP6Config interface which
/// provides access to configuration options returned by the IPv6 DHCP server.
#[derive(Debug, Clone)]
pub struct Dhcp6Config {
    /// D-Bus object path for this DHCP6 configuration
    pub path: OwnedObjectPath,

    /// Configuration options returned by the DHCP server.
    pub options: HashMap<String, OwnedValue>,
}
