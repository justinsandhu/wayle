/// Audio device index
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeviceIndex(pub u32);

/// Composite device key that uniquely identifies a device
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeviceKey {
    /// Numeric index from PulseAudio/PipeWire
    pub index: u32,
    /// Device type (Input/Output)
    pub device_type: DeviceType,
}

impl DeviceKey {
    /// Create a new device key
    pub fn new(index: u32, device_type: DeviceType) -> Self {
        Self { index, device_type }
    }

    /// Get the DeviceIndex for this key
    pub fn device_index(&self) -> DeviceIndex {
        DeviceIndex(self.index)
    }
}

/// Audio device name
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeviceName(pub String);

impl DeviceName {
    /// Create a DeviceName from a string
    pub fn new(name: String) -> Self {
        Self(name)
    }

    /// Get the device name as a string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Audio device type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceType {
    /// Audio input device (microphone, line-in)
    Input,
    /// Audio output device (speakers, headphones)
    Output,
}

/// Audio device state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceState {
    /// Device is idle
    Idle,
    /// Device is running
    Running,
    /// Device is suspended
    Suspended,
    /// Device state is unknown
    Unknown,
}

/// Audio device port information
#[derive(Debug, Clone, PartialEq)]
pub struct DevicePort {
    /// Port name
    pub name: String,
    /// Port description
    pub description: String,
    /// Port priority
    pub priority: u32,
    /// Whether port is available
    pub available: bool,
}

/// Audio device information
#[derive(Debug, Clone, PartialEq)]
pub struct DeviceInfo {
    /// Composite device key (unique identifier)
    pub key: DeviceKey,
    /// Device index (for backward compatibility)
    pub index: DeviceIndex,
    /// Device name
    pub name: DeviceName,
    /// Device description
    pub description: String,
    /// Device type
    pub device_type: DeviceType,
    /// Device state
    pub state: DeviceState,
    /// Whether device is default
    pub is_default: bool,
    /// Device port information
    pub ports: Vec<DevicePort>,
    /// Active port
    pub active_port: Option<String>,
}

impl DeviceInfo {
    /// Create a new DeviceInfo with proper key generation
    pub fn new(
        index: u32,
        device_type: DeviceType,
        name: DeviceName,
        description: String,
        state: DeviceState,
        is_default: bool,
        ports: Vec<DevicePort>,
        active_port: Option<String>,
    ) -> Self {
        let key = DeviceKey::new(index, device_type);
        Self {
            key,
            index: DeviceIndex(index),
            name,
            description,
            device_type,
            state,
            is_default,
            ports,
            active_port,
        }
    }
}
