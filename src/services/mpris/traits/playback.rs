/// Current playback state of a media player
#[derive(Debug, Clone, PartialEq)]
pub enum PlaybackState {
    /// Player is currently playing
    Playing,

    /// Player is paused
    Paused,

    /// Player is stopped
    Stopped,
}

impl From<&str> for PlaybackState {
    fn from(status: &str) -> Self {
        match status {
            "Playing" => Self::Playing,
            "Paused" => Self::Paused,
            "Stopped" => Self::Stopped,
            _ => Self::Stopped,
        }
    }
}

impl From<PlaybackState> for &'static str {
    fn from(state: PlaybackState) -> Self {
        match state {
            PlaybackState::Playing => "Playing",
            PlaybackState::Paused => "Paused",
            PlaybackState::Stopped => "Stopped",
        }
    }
}

/// Loop mode for track or playlist repetition
#[derive(Debug, Clone, PartialEq)]
pub enum LoopMode {
    /// No looping
    None,

    /// Loop current track
    Track,

    /// Loop entire playlist
    Playlist,

    /// Loop mode not supported by player
    Unsupported,
}

impl From<&str> for LoopMode {
    fn from(status: &str) -> Self {
        match status {
            "None" => Self::None,
            "Track" => Self::Track,
            "Playlist" => Self::Playlist,
            _ => Self::Unsupported,
        }
    }
}

impl From<LoopMode> for &'static str {
    fn from(mode: LoopMode) -> Self {
        match mode {
            LoopMode::None => "None",
            LoopMode::Track => "Track",
            LoopMode::Playlist => "Playlist",
            LoopMode::Unsupported => "None",
        }
    }
}

/// Shuffle mode for randomizing playback order
#[derive(Debug, Clone, PartialEq)]
pub enum ShuffleMode {
    /// Shuffle enabled
    On,

    /// Shuffle disabled
    Off,

    /// Shuffle mode not supported by player
    Unsupported,
}

impl From<bool> for ShuffleMode {
    fn from(shuffle: bool) -> Self {
        if shuffle { Self::On } else { Self::Off }
    }
}

impl From<ShuffleMode> for bool {
    fn from(mode: ShuffleMode) -> Self {
        match mode {
            ShuffleMode::On => true,
            ShuffleMode::Off => false,
            ShuffleMode::Unsupported => false,
        }
    }
}

