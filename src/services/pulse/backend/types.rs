use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use tokio::sync::{broadcast, mpsc};

use crate::services::{
    AudioEvent, DeviceInfo, StreamIndex, StreamInfo,
    pulse::device::{self, DeviceKey},
};

/// Thread-safe storage for audio devices
pub type DeviceStore = Arc<RwLock<HashMap<DeviceKey, DeviceInfo>>>;

/// Thread-safe storage for audio streams  
pub type StreamStore = Arc<RwLock<HashMap<StreamIndex, StreamInfo>>>;

/// Thread-safe storage for default device information
pub type DefaultDevice = Arc<RwLock<Option<DeviceInfo>>>;

/// Channel sender for audio events
pub type EventSender = broadcast::Sender<AudioEvent>;

/// Channel sender for device list updates
pub type DeviceListSender = broadcast::Sender<Vec<DeviceInfo>>;

/// Channel sender for stream list updates
pub type StreamListSender = broadcast::Sender<Vec<StreamInfo>>;

/// Channel sender for backend commands
pub type CommandSender = mpsc::UnboundedSender<PulseCommand>;

/// Thread-safe storage for server information
pub type ServerInfo = Arc<RwLock<Option<String>>>;

pub(super) type CommandReceiver = mpsc::UnboundedReceiver<PulseCommand>;

/// Change notifications from PulseAudio subscription
#[derive(Debug, Clone)]
pub enum ChangeNotification {
    /// Device-related change notification
    Device {
        /// PulseAudio facility type
        facility: libpulse_binding::context::subscribe::Facility,
        /// Operation performed on the device
        operation: libpulse_binding::context::subscribe::Operation,
        /// Device index
        index: u32,
    },
    /// Stream-related change notification
    Stream {
        /// PulseAudio facility type
        facility: libpulse_binding::context::subscribe::Facility,
        /// Operation performed on the stream
        operation: libpulse_binding::context::subscribe::Operation,
        /// Stream index
        index: u32,
    },
    /// Server-related change notification
    Server {
        /// PulseAudio facility type
        facility: libpulse_binding::context::subscribe::Facility,
        /// Operation performed on the server
        operation: libpulse_binding::context::subscribe::Operation,
        /// Server index
        index: u32,
    },
}

/// PulseAudio commands for backend communication
#[derive(Debug)]
pub enum PulseCommand {
    /// Trigger device discovery refresh
    TriggerDeviceDiscovery,
    /// Trigger stream discovery refresh
    TriggerStreamDiscovery,
    /// Trigger server info query for default device detection
    TriggerServerInfoQuery,
    /// Set device volume
    SetDeviceVolume {
        /// Target device
        device: device::DeviceIndex,
        /// New volume levels
        volume: libpulse_binding::volume::ChannelVolumes,
    },
    /// Set device mute state
    SetDeviceMute {
        /// Target device
        device: device::DeviceIndex,
        /// Mute state
        muted: bool,
    },
    /// Set stream volume
    SetStreamVolume {
        /// Target stream
        stream: StreamIndex,
        /// New volume levels
        volume: libpulse_binding::volume::ChannelVolumes,
    },
    /// Set stream mute state
    SetStreamMute {
        /// Target stream
        stream: StreamIndex,
        /// Mute state
        muted: bool,
    },
    /// Set default input device
    SetDefaultInput {
        /// Target device
        device: device::DeviceIndex,
    },
    /// Set default output device
    SetDefaultOutput {
        /// Target device
        device: device::DeviceIndex,
    },
    /// Move stream to different device
    MoveStream {
        /// Target stream
        stream: StreamIndex,
        /// Destination device
        device: device::DeviceIndex,
    },
    /// Shutdown backend
    Shutdown,
}
