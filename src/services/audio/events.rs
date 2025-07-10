use super::{DeviceIndex, DeviceInfo, StreamIndex, StreamInfo, Volume};

/// Events emitted by audio system
#[derive(Debug, Clone)]
pub enum AudioEvent {
    /// A new device became available
    DeviceAdded(DeviceInfo),

    /// A device was removed
    DeviceRemoved(DeviceIndex),

    /// Device volume changed
    DeviceVolumeChanged {
        /// Device that changed volume
        device: DeviceIndex,
        /// New volume
        volume: Volume,
    },

    /// Device mute state changed
    DeviceMuteChanged {
        /// Device that changed mute state
        device: DeviceIndex,
        /// New mute state
        muted: bool,
    },

    /// Default device changed
    DefaultDeviceChanged {
        /// New default device
        device: DeviceIndex,
    },

    /// Device properties changed
    DeviceChanged(DeviceInfo),

    /// Stream properties changed
    StreamChanged(StreamInfo),

    /// Default input device changed
    DefaultInputChanged(DeviceInfo),

    /// Default output device changed
    DefaultOutputChanged(DeviceInfo),

    /// A new stream was created
    StreamAdded(StreamInfo),

    /// A stream was removed
    StreamRemoved(StreamIndex),

    /// Stream volume changed
    StreamVolumeChanged {
        /// Stream that changed volume
        stream: StreamIndex,
        /// New volume
        volume: Volume,
    },

    /// Stream mute state changed
    StreamMuteChanged {
        /// Stream that changed mute state
        stream: StreamIndex,
        /// New mute state
        muted: bool,
    },

    /// Stream moved to different device
    StreamMoved {
        /// Stream that was moved
        stream: StreamIndex,
        /// New device
        device: DeviceIndex,
    },
}