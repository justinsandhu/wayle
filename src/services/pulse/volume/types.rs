/// Multi-channel volume with automatic clamping
///
/// Volume range: 0.0 (muted) to 4.0 (400% amplification)
/// - 0.0 = Muted
/// - 1.0 = Normal volume (100%)
/// - 2.0 = 200% amplification
/// - 4.0 = Maximum amplification (400%)
#[derive(Debug, Clone, PartialEq)]
pub struct Volume {
    volumes: Vec<f64>,
}

impl Volume {
    /// Create a new volume with the given channel volumes
    ///
    /// Volume levels are automatically clamped to valid range (0.0 to 4.0).
    /// - 0.0 = Muted
    /// - 1.0 = Normal volume (100%)
    /// - 4.0 = Maximum amplification (400%)
    pub fn new(volumes: Vec<f64>) -> Self {
        let volumes = volumes.into_iter().map(|v| v.clamp(0.0, 4.0)).collect();
        Self { volumes }
    }

    /// Create a mono volume
    ///
    /// Volume is automatically clamped to valid range (0.0 to 4.0).
    /// Use 1.0 for normal volume, values above 1.0 for amplification.
    pub fn mono(volume: f64) -> Self {
        Self::new(vec![volume])
    }

    /// Create a stereo volume
    ///
    /// Volume levels are automatically clamped to valid range (0.0 to 4.0).
    /// Use 1.0 for normal volume, values above 1.0 for amplification.
    pub fn stereo(left: f64, right: f64) -> Self {
        Self::new(vec![left, right])
    }

    /// Get volume for a specific channel
    pub fn channel(&self, channel: usize) -> Option<f64> {
        self.volumes.get(channel).copied()
    }

    /// Set volume for a specific channel
    ///
    /// Volume is automatically clamped to valid range (0.0 to 4.0).
    /// Returns true if channel exists, false otherwise.
    pub fn set_channel(&mut self, channel: usize, volume: f64) -> bool {
        if let Some(vol) = self.volumes.get_mut(channel) {
            *vol = volume.clamp(0.0, 4.0);
            true
        } else {
            false
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

    /// Create a muted volume (0.0)
    pub fn muted(channels: usize) -> Self {
        Self::new(vec![0.0; channels])
    }

    /// Create a normal volume (1.0 = 100%)
    pub fn normal(channels: usize) -> Self {
        Self::new(vec![1.0; channels])
    }

    /// Create a volume from percentage (0-100% maps to 0.0-1.0)
    pub fn from_percentage(percentage: f64, channels: usize) -> Self {
        let volume = percentage / 100.0;
        Self::new(vec![volume; channels])
    }

    /// Get volume as percentage (1.0 = 100%)
    pub fn to_percentage(&self) -> Vec<f64> {
        self.volumes.iter().map(|&v| v * 100.0).collect()
    }

    /// Check if volume is muted (all channels at 0.0)
    pub fn is_muted(&self) -> bool {
        self.volumes.iter().all(|&v| v == 0.0)
    }

    /// Check if volume is at normal level (all channels at 1.0)
    pub fn is_normal(&self) -> bool {
        self.volumes.iter().all(|&v| v == 1.0)
    }
}
