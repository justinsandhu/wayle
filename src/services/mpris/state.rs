use std::time::Duration;

use super::{MediaPlayer2PlayerProxy, PlayerInfo, TrackMetadata, PlaybackState, LoopMode, ShuffleMode};

/// Tracks the current state of a media player
#[derive(Debug)]
pub struct PlayerStateTracker {
    /// Player information and capabilities
    pub info: PlayerInfo,
    /// D-Bus proxy for player control
    pub player_proxy: MediaPlayer2PlayerProxy<'static>,
    /// Last known track metadata
    pub last_metadata: TrackMetadata,
    /// Last known playback position
    pub last_position: Duration,
    /// Last known playback state
    pub last_playback_state: PlaybackState,
    /// Last known loop mode
    pub last_loop_mode: LoopMode,
    /// Last known shuffle mode
    pub last_shuffle_mode: ShuffleMode,
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
            info,
            player_proxy,
            last_metadata: metadata,
            last_position: position,
            last_playback_state: playback_state,
            last_loop_mode: loop_mode,
            last_shuffle_mode: shuffle_mode,
            monitoring_handle: None,
        }
    }
}