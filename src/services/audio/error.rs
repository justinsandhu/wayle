use super::{DeviceIndex, StreamIndex, VolumeError};

/// Errors that can occur during audio operations
#[derive(thiserror::Error, Debug)]
pub enum AudioError {
    /// Device with the given index was not found
    #[error("Device {0:?} not found")]
    DeviceNotFound(DeviceIndex),

    /// Stream with the given index was not found
    #[error("Stream {0:?} not found")]
    StreamNotFound(StreamIndex),

    /// Volume operation failed
    #[error("Volume operation failed: {0}")]
    VolumeError(#[from] VolumeError),

    /// PulseAudio connection error
    #[error("PulseAudio connection failed: {0}")]
    ConnectionError(String),

    /// PulseAudio operation failed
    #[error("PulseAudio operation failed: {0}")]
    OperationError(String),

    /// Device operation not supported
    #[error("Device {device:?} doesn't support {operation}")]
    UnsupportedOperation {
        /// Device that doesn't support the operation
        device: DeviceIndex,
        /// Name of the unsupported operation
        operation: String,
    },

    /// Audio service not responding
    #[error("Audio service not responding")]
    ServiceUnresponsive,

    /// Failed to initialize the audio service
    #[error("Failed to initialize audio service: {0}")]
    InitializationFailed(String),

    /// Permission denied for audio operation
    #[error("Permission denied for audio operation: {0}")]
    PermissionDenied(String),

    /// Invalid audio configuration
    #[error("Invalid audio configuration: {0}")]
    InvalidConfiguration(String),
}
