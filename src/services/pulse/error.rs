use super::{DeviceIndex, StreamIndex, VolumeError};

/// PulseAudio service errors
#[derive(thiserror::Error, Debug)]
pub enum PulseError {
    /// PulseAudio connection failed
    #[error("PulseAudio connection failed: {0}")]
    ConnectionFailed(String),

    /// PulseAudio operation failed
    #[error("PulseAudio operation failed: {0}")]
    OperationFailed(String),

    /// Volume conversion failed
    #[error("Volume conversion failed")]
    VolumeConversion(#[from] VolumeError),

    /// Device not found
    #[error("Device {0:?} not found")]
    DeviceNotFound(DeviceIndex),

    /// Stream not found
    #[error("Stream {0:?} not found")]
    StreamNotFound(StreamIndex),

    /// Thread communication failed
    #[error("PulseAudio thread communication failed")]
    ThreadCommunication,

    /// Service initialization failed
    #[error("Service initialization failed: {0}")]
    InitializationFailed(String),
}
