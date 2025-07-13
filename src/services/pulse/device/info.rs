use crate::services::pulse::Volume;
use std::fmt;

/// Device index identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeviceIndex(pub u32);

/// Device type enumeration
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DeviceType {
    /// Audio input device (microphone, line-in)
    Input,
    /// Audio output device (speakers, headphones)
    Output,
}

/// Device state enumeration
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DeviceState {
    /// Device is running and available
    Running,
    /// Device is idle
    Idle,
    /// Device is suspended
    Suspended,
    /// Device is offline or unavailable
    Offline,
}

/// Device name wrapper for type safety
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeviceName(String);

impl DeviceName {
    /// Create a new device name
    pub fn new(name: String) -> Self {
        Self(name)
    }

    /// Get device name as string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DeviceName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Device port information
#[derive(Debug, Clone, PartialEq)]
pub struct DevicePort {
    /// Port name
    pub name: String,
    /// Port description
    pub description: String,
    /// Port priority
    pub priority: u32,
    /// Port availability
    pub available: bool,
}

/// Device key for unique identification
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeviceKey {
    /// Device index
    pub index: u32,
    /// Device type
    pub device_type: DeviceType,
}

impl DeviceKey {
    /// Create a new device key
    pub fn new(index: u32, device_type: DeviceType) -> Self {
        Self { index, device_type }
    }
}

/// Complete device information
#[derive(Debug, Clone, PartialEq)]
pub struct DeviceInfo {
    /// Device index
    pub index: DeviceIndex,
    /// Device type
    pub device_type: DeviceType,
    /// Device name
    pub name: DeviceName,
    /// Human-readable description
    pub description: String,
    /// Device state
    pub state: DeviceState,
    /// Whether device is muted
    pub muted: bool,
    /// Device volume
    pub volume: Volume,
    /// Available ports
    pub ports: Vec<DevicePort>,
    /// Currently active port
    pub active_port: Option<String>,
    /// Unique device key
    pub key: DeviceKey,
}

impl DeviceInfo {
    /// Create a new device info
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        index: u32,
        device_type: DeviceType,
        name: DeviceName,
        description: String,
        state: DeviceState,
        muted: bool,
        volume: Volume,
        ports: Vec<DevicePort>,
        active_port: Option<String>,
    ) -> Self {
        let index = DeviceIndex(index);
        let key = DeviceKey::new(index.0, device_type.clone());

        Self {
            index,
            device_type,
            name,
            description,
            state,
            muted,
            volume,
            ports,
            active_port,
            key,
        }
    }

    /// Check if device properties have changed, excluding volume and mute state
    ///
    /// Compares non-audio properties that would trigger a DeviceChanged event.
    /// Volume and mute changes are handled separately via specific events.
    ///
    /// # Arguments
    /// * `other_device` - Device info to compare against
    ///
    /// # Returns
    /// `true` if any tracked properties differ, `false` if all are identical
    pub fn properties_changed(&self, other: &DeviceInfo) -> bool {
        self.name != other.name
            || self.description != other.description
            || self.state != other.state
            || self.active_port != other.active_port
            || self.ports != other.ports
    }
}
