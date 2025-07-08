use std::time::Duration;

use super::{
    LoopMode, PlaybackState, PlayerCapabilities, PlayerId, PlayerInfo, ShuffleMode, TrackMetadata,
};

/// Events emitted by media players
#[derive(Debug, Clone)]
pub enum PlayerEvent {
    /// A new player became available
    PlayerAdded(PlayerInfo),

    /// A player was removed
    PlayerRemoved(PlayerId),

    /// Player's playback state changed
    PlaybackStateChanged {
        /// Player that changed state
        player_id: PlayerId,
        /// New playback state
        state: PlaybackState,
    },

    /// Player's track metadata changed
    MetadataChanged {
        /// Player that changed metadata
        player_id: PlayerId,
        /// New track metadata
        metadata: TrackMetadata,
    },

    /// Player's playback position changed
    PositionChanged {
        /// Player that changed position
        player_id: PlayerId,
        /// New playback position
        position: Duration,
    },

    /// Player's loop mode changed
    LoopModeChanged {
        /// Player that changed loop mode
        player_id: PlayerId,
        /// New loop mode
        mode: LoopMode,
    },

    /// Player's shuffle mode changed
    ShuffleModeChanged {
        /// Player that changed shuffle mode
        player_id: PlayerId,
        /// New shuffle mode
        mode: ShuffleMode,
    },

    /// Player's capabilities changed
    CapabilitiesChanged {
        /// Player that changed capabilities
        player_id: PlayerId,
        /// New capabilities
        capabilities: PlayerCapabilities,
    },
}
