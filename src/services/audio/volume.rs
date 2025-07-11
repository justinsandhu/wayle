/// Multi-channel volume with validation
#[derive(Debug, Clone, PartialEq)]
pub struct Volume {
    volumes: Vec<f64>,
}

impl Volume {
    /// Create a new volume with the given channel volumes
    ///
    /// # Arguments
    /// * `volumes` - Volume levels for each channel (0.0 to 10.0)
    ///
    /// # Errors
    /// Returns error if any volume is outside valid range
    pub fn new(volumes: Vec<f64>) -> Result<Self, VolumeError> {
        for (i, &vol) in volumes.iter().enumerate() {
            if !(0.0..=10.0).contains(&vol) {
                return Err(VolumeError::InvalidVolume {
                    channel: i,
                    volume: vol,
                });
            }
        }
        Ok(Self { volumes })
    }

    /// Create a mono volume
    ///
    /// # Errors
    /// Returns error if volume is outside valid range 0.0-10.0
    pub fn mono(volume: f64) -> Result<Self, VolumeError> {
        Self::new(vec![volume])
    }

    /// Create a stereo volume
    ///
    /// # Errors
    /// Returns error if either volume is outside valid range 0.0-10.0
    pub fn stereo(left: f64, right: f64) -> Result<Self, VolumeError> {
        Self::new(vec![left, right])
    }

    /// Get volume for a specific channel
    pub fn get_channel(&self, channel: usize) -> Option<f64> {
        self.volumes.get(channel).copied()
    }

    /// Set volume for a specific channel
    ///
    /// # Errors
    /// Returns error if volume is outside valid range 0.0-10.0 or channel doesn't exist
    pub fn set_channel(&mut self, channel: usize, volume: f64) -> Result<(), VolumeError> {
        if !(0.0..=10.0).contains(&volume) {
            return Err(VolumeError::InvalidVolume { channel, volume });
        }
        if let Some(vol) = self.volumes.get_mut(channel) {
            *vol = volume;
            Ok(())
        } else {
            Err(VolumeError::InvalidChannel { channel })
        }
    }

    /// Get average volume across all channels
    pub fn average(&self) -> f64 {
        if self.volumes.is_empty() {
            0.0
        } else {
            self.volumes.iter().sum::<f64>() / self.volumes.len() as f64
        }
    }

    /// Get number of channels
    pub fn channels(&self) -> usize {
        self.volumes.len()
    }

    /// Get all channel volumes
    pub fn as_slice(&self) -> &[f64] {
        &self.volumes
    }
}

/// Volume-related errors
#[derive(thiserror::Error, Debug, Clone, PartialEq)]
pub enum VolumeError {
    /// Invalid volume level
    #[error("Invalid volume {volume} for channel {channel} (must be 0.0-10.0)")]
    InvalidVolume {
        /// Channel index
        channel: usize,
        /// Invalid volume value
        volume: f64,
    },
    /// Invalid channel index
    #[error("Invalid channel index {channel}")]
    InvalidChannel {
        /// Channel index
        channel: usize,
    },
}
