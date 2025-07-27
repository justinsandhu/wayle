use std::collections::HashMap;
use zbus::zvariant::{OwnedObjectPath, OwnedValue};

/// IPv4 DHCP Client State.
///
/// This corresponds to the org.freedesktop.NetworkManager.DHCP4Config interface which
/// provides access to configuration options returned by the IPv4 DHCP server.
#[derive(Debug, Clone)]
pub struct Dhcp4Config {
    /// D-Bus object path for this DHCP4 configuration
    pub path: OwnedObjectPath,

    /// Configuration options returned by the DHCP server.
    pub options: HashMap<String, OwnedValue>,
}
