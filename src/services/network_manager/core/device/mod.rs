/// WiFi device functionality and management.
pub mod wifi;
/// Wired (ethernet) device functionality and management.
pub mod wired;

use zbus::Connection;

use crate::services::{
    common::{Property, types::ObjectPath},
    network_manager::{
        LldpNeighbor, NMConnectivityState, NMDeviceCapabilities, NMDeviceInterfaceFlags,
        NMDeviceState, NMDeviceStateReason, NMDeviceType, NMMetered, proxy::devices::DeviceProxy,
    },
};

/// Wrapper around zbus::Connection that implements Debug.
#[derive(Clone)]
pub(crate) struct DbusConnection(pub Connection);

impl std::fmt::Debug for DbusConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection").finish()
    }
}

impl std::ops::Deref for DbusConnection {
    type Target = Connection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Network device managed by NetworkManager.
///
/// Common functionality for all network interfaces (WiFi, ethernet, etc).
/// Contains hardware information, state, configuration, and statistics.
#[derive(Debug, Clone)]
pub struct Device {
    /// D-Bus connection for this device.
    pub(crate) connection: DbusConnection,

    /// D-Bus object path for this device.
    pub object_path: ObjectPath,

    /// Operating-system specific transient device hardware identifier. Opaque
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

    /// The name of the device's data interface when available. May not refer
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
    /// device. Can be used to recognize when two seemingly-separate hardware devices
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

/// Fetched device properties from D-Bus
struct DeviceProperties {
    udi: String,
    path: String,
    interface: String,
    ip_interface: String,
    driver: String,
    driver_version: String,
    firmware_version: String,
    capabilities: u32,
    state: u32,
    state_reason: (u32, u32),
    active_connection: String,
    ip4_config: String,
    dhcp4_config: String,
    ip6_config: String,
    dhcp6_config: String,
    managed: bool,
    autoconnect: bool,
    firmware_missing: bool,
    nm_plugin_missing: bool,
    device_type: u32,
    available_connections: Vec<String>,
    physical_port_id: String,
    mtu: u32,
    metered: u32,
    real: bool,
    ip4_connectivity: u32,
    ip6_connectivity: u32,
    interface_flags: u32,
    hw_address: String,
    ports: Vec<String>,
}

impl Device {
    /// Create a Device from a D-Bus connection and path.
    pub async fn from_connection_and_path(
        connection: Connection,
        object_path: ObjectPath,
    ) -> Option<Self> {
        let proxy = DeviceProxy::new(&connection, object_path.clone())
            .await
            .ok()?;
        let props = Self::fetch_properties(&proxy).await?;
        Some(Self::from_properties(props, connection, object_path))
    }

    /// Fetch all properties from the proxy
    #[allow(clippy::too_many_lines)]
    async fn fetch_properties(proxy: &DeviceProxy<'_>) -> Option<DeviceProperties> {
        let (udi, path, interface, ip_interface, driver, driver_version, firmware_version) = tokio::join!(
            proxy.udi(),
            proxy.path(),
            proxy.interface(),
            proxy.ip_interface(),
            proxy.driver(),
            proxy.driver_version(),
            proxy.firmware_version(),
        );

        let (
            capabilities,
            state,
            state_reason,
            active_connection,
            ip4_config,
            dhcp4_config,
            ip6_config,
            dhcp6_config,
        ) = tokio::join!(
            proxy.capabilities(),
            proxy.state(),
            proxy.state_reason(),
            proxy.active_connection(),
            proxy.ip4_config(),
            proxy.dhcp4_config(),
            proxy.ip6_config(),
            proxy.dhcp6_config(),
        );

        let (
            managed,
            autoconnect,
            firmware_missing,
            nm_plugin_missing,
            device_type,
            available_connections,
            physical_port_id,
            mtu,
        ) = tokio::join!(
            proxy.managed(),
            proxy.autoconnect(),
            proxy.firmware_missing(),
            proxy.nm_plugin_missing(),
            proxy.device_type(),
            proxy.available_connections(),
            proxy.physical_port_id(),
            proxy.mtu(),
        );

        let (
            metered,
            real,
            ip4_connectivity,
            ip6_connectivity,
            interface_flags,
            hw_address,
            ports,
            _lldp_neighbors,
        ) = tokio::join!(
            proxy.metered(),
            proxy.real(),
            proxy.ip4_connectivity(),
            proxy.ip6_connectivity(),
            proxy.interface_flags(),
            proxy.hw_address(),
            proxy.ports(),
            proxy.lldp_neighbors(),
        );

        Some(DeviceProperties {
            udi: udi.ok()?,
            path: path.ok()?,
            interface: interface.ok()?,
            ip_interface: ip_interface.ok()?,
            driver: driver.ok()?,
            driver_version: driver_version.ok()?,
            firmware_version: firmware_version.ok()?,
            capabilities: capabilities.ok()?,
            state: state.ok()?,
            state_reason: state_reason.ok()?,
            active_connection: active_connection.ok()?.to_string(),
            ip4_config: ip4_config.ok()?.to_string(),
            dhcp4_config: dhcp4_config.ok()?.to_string(),
            ip6_config: ip6_config.ok()?.to_string(),
            dhcp6_config: dhcp6_config.ok()?.to_string(),
            managed: managed.ok()?,
            autoconnect: autoconnect.ok()?,
            firmware_missing: firmware_missing.ok()?,
            nm_plugin_missing: nm_plugin_missing.ok()?,
            device_type: device_type.ok()?,
            available_connections: available_connections
                .ok()?
                .into_iter()
                .map(|p| p.to_string())
                .collect(),
            physical_port_id: physical_port_id.ok()?,
            mtu: mtu.ok()?,
            metered: metered.ok()?,
            real: real.ok()?,
            ip4_connectivity: ip4_connectivity.ok()?,
            ip6_connectivity: ip6_connectivity.ok()?,
            interface_flags: interface_flags.ok()?,
            hw_address: hw_address.ok()?,
            ports: ports.ok()?.into_iter().map(|p| p.to_string()).collect(),
        })
    }

    /// Convert fetched properties to Device
    fn from_properties(
        props: DeviceProperties,
        connection: Connection,
        object_path: ObjectPath,
    ) -> Self {
        Self {
            connection: DbusConnection(connection),
            object_path,
            udi: Property::new(props.udi),
            path: Property::new(props.path),
            interface: Property::new(props.interface),
            ip_interface: Property::new(props.ip_interface),
            driver: Property::new(props.driver),
            driver_version: Property::new(props.driver_version),
            firmware_version: Property::new(props.firmware_version),
            capabilities: Property::new(NMDeviceCapabilities::from_bits_truncate(
                props.capabilities,
            )),
            state: Property::new(NMDeviceState::from_u32(props.state)),
            state_reason: Property::new((
                NMDeviceState::from_u32(props.state_reason.0),
                NMDeviceStateReason::from_u32(props.state_reason.1),
            )),
            active_connection: Property::new(props.active_connection),
            ip4_config: Property::new(props.ip4_config),
            dhcp4_config: Property::new(props.dhcp4_config),
            ip6_config: Property::new(props.ip6_config),
            dhcp6_config: Property::new(props.dhcp6_config),
            managed: Property::new(props.managed),
            autoconnect: Property::new(props.autoconnect),
            firmware_missing: Property::new(props.firmware_missing),
            nm_plugin_missing: Property::new(props.nm_plugin_missing),
            device_type: Property::new(NMDeviceType::from_u32(props.device_type)),
            available_connections: Property::new(props.available_connections),
            physical_port_id: Property::new(props.physical_port_id),
            mtu: Property::new(props.mtu),
            metered: Property::new(NMMetered::from_u32(props.metered)),
            real: Property::new(props.real),
            ip4_connectivity: Property::new(NMConnectivityState::from_u32(props.ip4_connectivity)),
            ip6_connectivity: Property::new(NMConnectivityState::from_u32(props.ip6_connectivity)),
            interface_flags: Property::new(NMDeviceInterfaceFlags::from_bits_truncate(
                props.interface_flags,
            )),
            hw_address: Property::new(props.hw_address),
            ports: Property::new(props.ports),
            // No idea what the properties for LLDP are - feel free to open a PR if you need this
            lldp_neighbors: Property::new(vec![]),
        }
    }
}
