/// Audio device index
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeviceIndex(pub u32);

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
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// Device index
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