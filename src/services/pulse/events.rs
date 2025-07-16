use super::{
    DeviceIndex, Volume,
    device::{DeviceInfo, DeviceKey},
    stream::{StreamInfo, StreamKey},
};

/// Audio system events
#[derive(Debug, Clone)]
pub enum AudioEvent {
    /// Device was added to the system
    DeviceAdded(DeviceInfo),
    /// Device was removed from the system
    DeviceRemoved(DeviceInfo),
    /// Device volume changed
    DeviceVolumeChanged {
        /// Device that changed
        device_key: DeviceKey,
        /// New volume
        volume: Volume,
    },
    /// Device mute state changed
    DeviceMuteChanged {
        /// Device that changed
        device_key: DeviceKey,
        /// New mute state
        muted: bool,
    },
    /// Device state changed
    DeviceChanged(DeviceInfo),
    /// Default input device changed
    DefaultInputChanged(DeviceInfo),
    /// Default output device changed
    DefaultOutputChanged(DeviceInfo),
    /// Stream was added to the system
    StreamAdded(StreamInfo),
    /// Stream was removed from the system
    StreamRemoved(StreamInfo),
    /// Stream volume changed
    StreamVolumeChanged {
        /// Stream that changed
        stream_key: StreamKey,
        /// New volume
        volume: Volume,
    },
    /// Stream mute state changed
    StreamMuteChanged {
        /// Stream that changed
        stream_key: StreamKey,
        /// New mute state
        muted: bool,
    },
    /// Stream state changed
    StreamChanged(StreamInfo),
    /// Stream moved to different device
    StreamMoved {
        /// Stream that moved
        stream_key: StreamKey,
        /// Source device
        from_device: DeviceIndex,
        /// Destination device
        to_device: DeviceIndex,
    },
}
