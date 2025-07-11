use super::{DeviceIndex, DeviceInfo, DeviceName, StreamIndex, StreamInfo, Volume};

/// Events emitted by audio system
#[derive(Debug, Clone)]
pub enum AudioEvent {
    /// A new device became available
    DeviceAdded(DeviceInfo),

    /// A device was removed (includes full device info for display)
    DeviceRemoved(DeviceInfo),

    /// Device volume changed
    DeviceVolumeChanged {
        /// Device that changed volume
        device_index: DeviceIndex,
        /// Device name for display
        device_name: DeviceName,
        /// New volume
        volume: Volume,
    },

    /// Device mute state changed
    DeviceMuteChanged {
        /// Device that changed mute state
        device_index: DeviceIndex,
        /// Device name for display
        device_name: DeviceName,
        /// New mute state
        muted: bool,
    },

    /// Device properties changed (fallback for unspecified changes)
    DeviceChanged(DeviceInfo),

    /// Stream properties changed (fallback for unspecified changes)
    StreamChanged(StreamInfo),

    /// Default input device changed
    DefaultInputChanged(DeviceInfo),

    /// Default output device changed
    DefaultOutputChanged(DeviceInfo),

    /// A new stream was created
    StreamAdded(StreamInfo),

    /// A stream was removed (includes full stream info for display)
    StreamRemoved(StreamInfo),

    /// Stream volume changed
    StreamVolumeChanged {
        /// Stream that changed volume
        stream_index: StreamIndex,
        /// Stream name for display
        stream_name: String,
        /// Application name for context
        application_name: String,
        /// New volume
        volume: Volume,
    },

    /// Stream mute state changed
    StreamMuteChanged {
        /// Stream that changed mute state
        stream_index: StreamIndex,
        /// Stream name for display
        stream_name: String,
        /// Application name for context
        application_name: String,
        /// New mute state
        muted: bool,
    },

    /// Stream moved to different device
    StreamMoved {
        /// Stream that was moved
        stream_index: StreamIndex,
        /// Stream name for display
        stream_name: String,
        /// Application name for context
        application_name: String,
        /// New device
        device_index: DeviceIndex,
        /// New device name for display
        device_name: DeviceName,
    },
}
