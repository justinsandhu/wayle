pub mod wifi;
pub mod wired;

use super::{LldpNeighbor, NMConnectivityState, NMDeviceInterfaceFlags, NMDeviceType, NMMetered};
use crate::services::{
    common::{Property, types::ObjectPath},
    network_manager::{NMDeviceCapabilities, NMDeviceState, NMDeviceStateReason},
};

#[derive(Debug, Clone)]
pub struct Device {
    /// Operating-system specific transient device hardware identifier. This is an opaque
    /// string representing the underlying hardware for the device, and shouldn't be used to
    /// keep track of individual devices. For some device types (Bluetooth, Modems) it is an
    /// identifier used by the hardware service (eg bluez or ModemManager) to refer to that
    /// device, and client programs use it get additional information from those services
    /// which NM does not provide. The Udi is not guaranteed to be consistent across reboots
    /// or hotplugs of the hardware.
    pub udi: Property<String>,

    /// The path of the device as exposed by the udev property ID_PATH.
    pub path: Property<String>,

    /// The name of the device's control (and often data) interface. Note that non UTF-8
    /// characters are backslash escaped, so the resulting name may be longer then 15
    /// characters. Use g_strcompress() to revert the escaping.
    pub interface: Property<String>,

    /// The name of the device's data interface when available. This property may not refer
    /// to the actual data interface until the device has successfully established a data
    /// connection, indicated by the device's State becoming ACTIVATED. Note that non UTF-8
    /// characters are backslash escaped, so the resulting name may be longer then 15
    /// characters. Use g_strcompress() to revert the escaping.
    pub ip_interface: Property<String>,

    /// The driver handling the device. Non-UTF-8 sequences are backslash escaped.
    pub driver: Property<String>,

    /// The version of the driver handling the device. Non-UTF-8 sequences are backslash
    /// escaped.
    pub driver_version: Property<String>,

    /// The firmware version for the device. Non-UTF-8 sequences are backslash escaped.
    pub firmware_version: Property<String>,

    /// Flags describing the capabilities of the device. See NMDeviceCapabilities.
    pub capabilities: Property<NMDeviceCapabilities>,

    /// The current state of the device.
    pub state: Property<NMDeviceState>,

    /// The current state and reason for that state.
    pub state_reason: Property<(NMDeviceState, NMDeviceStateReason)>,

    /// Object path of an ActiveConnection object that "owns" this device during activation.
    /// The ActiveConnection object tracks the life-cycle of a connection to a specific
    /// network and implements the org.freedesktop.NetworkManager.Connection.Active D-Bus
    /// interface.
    pub active_connection: Property<ObjectPath>,

    /// Object path of the Ip4Config object describing the configuration of the device. Only
    /// valid when the device is in the NM_DEVICE_STATE_ACTIVATED state.
    pub ip4_config: Property<ObjectPath>,

    /// Object path of the Dhcp4Config object describing the DHCP options returned by the
    /// DHCP server. Only valid when the device is in the NM_DEVICE_STATE_ACTIVATED state.
    pub dhcp4_config: Property<ObjectPath>,

    /// Object path of the Ip6Config object describing the configuration of the device. Only
    /// valid when the device is in the NM_DEVICE_STATE_ACTIVATED state.
    pub ip6_config: Property<ObjectPath>,

    /// Object path of the Dhcp6Config object describing the DHCP options returned by the
    /// DHCP server. Only valid when the device is in the NM_DEVICE_STATE_ACTIVATED state.
    pub dhcp6_config: Property<ObjectPath>,

    /// Whether or not this device is managed by NetworkManager. Setting this property has a
    /// similar effect to configuring the device as unmanaged via the
    /// keyfile.unmanaged-devices setting in NetworkManager.conf.
    pub managed: Property<bool>,

    /// If TRUE, indicates the device is allowed to autoconnect. If FALSE, manual
    /// intervention is required before the device will automatically connect to a known
    /// network, such as activating a connection using the device, or setting this property
    /// to TRUE.
    pub autoconnect: Property<bool>,

    /// If TRUE, indicates the device is likely missing firmware necessary for its
    /// operation.
    pub firmware_missing: Property<bool>,

    /// If TRUE, indicates the NetworkManager plugin for the device is likely missing or
    /// misconfigured.
    pub nm_plugin_missing: Property<bool>,

    /// The general type of the network device.
    pub device_type: Property<NMDeviceType>,

    /// An array of object paths of every configured connection that is currently 'available'
    /// through this device.
    pub available_connections: Property<Vec<ObjectPath>>,

    /// If non-empty, an (opaque) indicator of the physical network port associated with the
    /// device. This can be used to recognize when two seemingly-separate hardware devices
    /// are actually just different virtual interfaces to the same physical port.
    pub physical_port_id: Property<String>,

    /// The MTU of the device.
    pub mtu: Property<u32>,

    /// Whether the amount of traffic flowing through the device is subject to limitations,
    /// for example set by service providers.
    pub metered: Property<NMMetered>,

    /// Array of LLDP neighbors; each element is a dictionary mapping LLDP TLV names to
    /// variant boxed values.
    pub lldp_neighbors: Property<Vec<LldpNeighbor>>,

    /// True if the device exists, or False for placeholder devices that do not yet exist but
    /// could be automatically created by NetworkManager if one of their
    /// AvailableConnections was activated.
    pub real: Property<bool>,

    /// The result of the last IPv4 connectivity check.
    pub ip4_connectivity: Property<NMConnectivityState>,

    /// The result of the last IPv6 connectivity check.
    pub ip6_connectivity: Property<NMConnectivityState>,

    /// The flags of the network interface. See NMDeviceInterfaceFlags for the currently
    /// defined flags.
    pub interface_flags: Property<NMDeviceInterfaceFlags>,

    /// The hardware address of the device.
    pub hw_address: Property<String>,

    /// The port devices of the controller device. Array of object paths of port devices for
    /// controller devices. For devices that are not controllers this is an empty array.
    pub ports: Property<Vec<ObjectPath>>,
}
