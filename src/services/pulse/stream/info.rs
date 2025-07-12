use crate::services::pulse::{device::DeviceIndex, volume::Volume};

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

/// Audio sample format
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SampleFormat {
    /// 8-bit unsigned
    U8,
    /// 16-bit signed little-endian
    S16LE,
    /// 16-bit signed big-endian  
    S16BE,
    /// 24-bit signed little-endian
    S24LE,
    /// 24-bit signed big-endian
    S24BE,
    /// 32-bit signed little-endian
    S32LE,
    /// 32-bit signed big-endian
    S32BE,
    /// 32-bit float little-endian (high quality)
    F32LE,
    /// 32-bit float big-endian
    F32BE,
    /// Unknown or unsupported format
    Unknown,
}

impl SampleFormat {
    /// Get the bit depth of the sample format
    pub fn bit_depth(&self) -> Option<u32> {
        match self {
            Self::U8 => Some(8),
            Self::S16LE | Self::S16BE => Some(16),
            Self::S24LE | Self::S24BE => Some(24),
            Self::S32LE | Self::S32BE | Self::F32LE | Self::F32BE => Some(32),
            Self::Unknown => None,
        }
    }

    /// Check if the format is little-endian
    pub fn is_little_endian(&self) -> bool {
        matches!(self, Self::S16LE | Self::S24LE | Self::S32LE | Self::F32LE)
    }

    /// Check if the format is big-endian
    pub fn is_big_endian(&self) -> bool {
        matches!(self, Self::S16BE | Self::S24BE | Self::S32BE | Self::F32BE)
    }

    /// Check if the format is floating point
    pub fn is_float(&self) -> bool {
        matches!(self, Self::F32LE | Self::F32BE)
    }

    /// Check if the format is signed integer
    pub fn is_signed_int(&self) -> bool {
        matches!(
            self,
            Self::S16LE | Self::S16BE | Self::S24LE | Self::S24BE | Self::S32LE | Self::S32BE
        )
    }

    /// Check if the format is unsigned integer
    pub fn is_unsigned_int(&self) -> bool {
        matches!(self, Self::U8)
    }

    /// Get a human-readable description of the format
    pub fn description(&self) -> &'static str {
        match self {
            Self::U8 => "8-bit unsigned PCM",
            Self::S16LE => "16-bit signed PCM (little-endian)",
            Self::S16BE => "16-bit signed PCM (big-endian)",
            Self::S24LE => "24-bit signed PCM (little-endian)",
            Self::S24BE => "24-bit signed PCM (big-endian)",
            Self::S32LE => "32-bit signed PCM (little-endian)",
            Self::S32BE => "32-bit signed PCM (big-endian)",
            Self::F32LE => "32-bit float PCM (little-endian)",
            Self::F32BE => "32-bit float PCM (big-endian)",
            Self::Unknown => "Unknown format",
        }
    }
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

impl StreamFormat {
    /// Get a human-readable description of the complete format
    pub fn description(&self) -> String {
        let channel_desc = match self.channels {
            1 => "Mono".to_string(),
            2 => "Stereo".to_string(),
            n => format!("{n}-channel"),
        };

        format!(
            "{} Hz, {}, {}",
            self.sample_rate,
            channel_desc,
            self.sample_format.description()
        )
    }

    /// Check if this is a standard CD quality format (44.1kHz, 16-bit, stereo)
    pub fn is_cd_quality(&self) -> bool {
        self.sample_rate == 44100
            && self.channels == 2
            && matches!(
                self.sample_format,
                SampleFormat::S16LE | SampleFormat::S16BE
            )
    }

    /// Check if this is a high-resolution audio format (>48kHz or >16-bit)
    pub fn is_high_resolution(&self) -> bool {
        self.sample_rate > 48000
            || self.sample_format.bit_depth().unwrap_or(16) > 16
            || self.sample_format.is_float()
    }

    /// Calculate bytes per second for this format
    pub fn bytes_per_second(&self) -> u32 {
        let bits_per_sample = self.sample_format.bit_depth().unwrap_or(16);
        self.sample_rate * self.channels as u32 * (bits_per_sample / 8)
    }
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
