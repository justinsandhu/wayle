use std::time::Duration;

use super::PlayerInfo;
use crate::services::mpris::{MediaPlayer2PlayerProxy, TrackMetadata};

/// Current playback state of a media player
#[derive(Debug, Clone, Copy, PartialEq)]
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
#[derive(Debug, Clone, Copy, PartialEq)]
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
#[derive(Debug, Clone, Copy, PartialEq)]
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

/// Complete state of a media player
#[derive(Debug, Clone)]
pub struct PlayerState {
    /// Basic player information
    pub player_info: PlayerInfo,

    /// Current playback state
    pub playback_state: PlaybackState,

    /// Current track metadata
    pub metadata: TrackMetadata,

    /// Current playback position
    pub position: Duration,

    /// Current loop mode
    pub loop_mode: LoopMode,

    /// Current shuffle mode
    pub shuffle_mode: ShuffleMode,
}

/// Tracks the current state of a media player
#[derive(Debug)]
pub struct PlayerStateTracker {
    /// Complete cached player state
    pub state: PlayerState,
    /// D-Bus proxy for player control
    pub player_proxy: MediaPlayer2PlayerProxy<'static>,
    /// Handle to the monitoring task for cleanup
    pub monitoring_handle: Option<tokio::task::JoinHandle<()>>,
}

impl PlayerStateTracker {
    /// Create a new player state tracker
    pub fn new(
        info: PlayerInfo,
        player_proxy: MediaPlayer2PlayerProxy<'static>,
        metadata: TrackMetadata,
        position: Duration,
        playback_state: PlaybackState,
        loop_mode: LoopMode,
        shuffle_mode: ShuffleMode,
    ) -> Self {
        Self {
            state: PlayerState {
                player_info: info,
                playback_state,
                metadata,
                position,
                loop_mode,
                shuffle_mode,
            },
            player_proxy,
            monitoring_handle: None,
        }
    }
}
