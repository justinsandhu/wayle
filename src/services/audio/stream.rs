use super::{DeviceIndex, Volume};

/// Audio stream index
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StreamIndex(pub u32);

/// Audio stream type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamType {
    /// Playback stream (audio output)
    Playback,
    /// Record stream (audio input)
    Record,
    /// Capture stream (audio input)
    Capture,
}

/// Audio stream state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamState {
    /// Stream is running
    Running,
    /// Stream is idle
    Idle,
    /// Stream is suspended
    Suspended,
    /// Stream is terminated
    Terminated,
}

/// Audio stream information
#[derive(Debug, Clone)]
pub struct StreamInfo {
    /// Stream index
    pub index: StreamIndex,
    /// Stream name
    pub name: String,
    /// Application name
    pub application_name: String,
    /// Stream type
    pub stream_type: StreamType,
    /// Stream state
    pub state: StreamState,
    /// Device this stream is connected to
    pub device_index: DeviceIndex,
    /// Stream volume
    pub volume: Volume,
    /// Whether stream is muted
    pub muted: bool,
    /// Stream format information
    pub format: StreamFormat,
}

/// Audio stream format
#[derive(Debug, Clone, PartialEq)]
pub struct StreamFormat {
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Number of channels
    pub channels: u8,
    /// Sample format
    pub sample_format: SampleFormat,
}

/// Audio sample format
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SampleFormat {
    /// 8-bit unsigned
    U8,
    /// 16-bit signed little-endian
    S16LE,
    /// 24-bit signed little-endian
    S24LE,
    /// 32-bit signed little-endian
    S32LE,
    /// 32-bit float little-endian
    F32LE,
    /// Unknown format
    Unknown,
}
