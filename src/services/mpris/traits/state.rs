use std::time::Duration;

use super::{LoopMode, PlaybackState, PlayerInfo, ShuffleMode, TrackMetadata};

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
