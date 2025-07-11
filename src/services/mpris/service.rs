use async_trait::async_trait;
use futures::Stream;
use std::{error::Error, future::Future, pin::Pin, time::Duration};

use super::{
    LoopMode, PlaybackState, PlayerId, PlayerInfo, PlayerState, ShuffleMode, TrackMetadata,
};

/// Reactive media service interface
///
/// Provides streaming data for UI reactivity and control methods for user actions.
/// All streams automatically handle player lifecycle and provide clean domain objects.
#[async_trait]
pub trait MediaService: Clone + Send + Sync + 'static {
    /// Error type for media operations
    type Error: Error + Send + Sync + 'static;

    /// Stream of currently available media players
    fn players(&self) -> impl Stream<Item = Vec<PlayerId>> + Send;

    /// Stream of player information for a specific player
    ///
    /// # Errors
    /// Returns error if player is not found or becomes unavailable
    fn player_info(
        &self,
        player_id: PlayerId,
    ) -> impl Stream<Item = Result<PlayerInfo, Self::Error>> + Send;

    /// Stream of playback state changes for a specific player
    fn playback_state(&self, player_id: PlayerId) -> impl Stream<Item = PlaybackState> + Send;

    /// Stream of playback position updates for a specific player
    fn position(&self, player_id: PlayerId) -> impl Stream<Item = Duration> + Send;

    /// Stream of track metadata changes for a specific player
    fn metadata(&self, player_id: PlayerId) -> impl Stream<Item = TrackMetadata> + Send;

    /// Stream of loop mode changes for a specific player
    fn loop_mode(&self, player_id: PlayerId) -> impl Stream<Item = LoopMode> + Send;

    /// Stream of shuffle mode changes for a specific player
    fn shuffle_mode(&self, player_id: PlayerId) -> impl Stream<Item = ShuffleMode> + Send;

    /// Stream of complete player state changes for a specific player
    fn player_state(&self, player_id: PlayerId) -> impl Stream<Item = PlayerState> + Send;

    /// Toggle play/pause state for a specific player
    ///
    /// # Errors
    /// Returns error if player is not found or doesn't support play/pause
    async fn play_pause(&self, player_id: PlayerId) -> Result<(), Self::Error>;

    /// Skip to next track for a specific player
    ///
    /// # Errors
    /// Returns error if player is not found or doesn't support next track
    async fn next(&self, player_id: PlayerId) -> Result<(), Self::Error>;

    /// Skip to previous track for a specific player
    ///
    /// # Errors
    /// Returns error if player is not found or doesn't support previous track
    async fn previous(&self, player_id: PlayerId) -> Result<(), Self::Error>;

    /// Seek to a specific position in the current track
    ///
    /// # Errors
    /// Returns error if player is not found, doesn't support seeking, or position is invalid
    async fn seek(&self, player_id: PlayerId, position: Duration) -> Result<(), Self::Error>;

    /// Toggle loop mode for a specific player
    ///
    /// # Errors
    /// Returns error if player is not found or doesn't support loop mode changes
    async fn toggle_loop(&self, player_id: PlayerId) -> Result<(), Self::Error>;

    /// Toggle shuffle mode for a specific player
    ///
    /// # Errors
    /// Returns error if player is not found or doesn't support shuffle mode changes
    async fn toggle_shuffle(&self, player_id: PlayerId) -> Result<(), Self::Error>;

    /// Get the currently active player
    async fn active_player(&self) -> Option<PlayerId>;

    /// Set the active player
    ///
    /// # Errors
    /// Returns error if the specified player is not found
    async fn set_active_player(&self, player_id: Option<PlayerId>) -> Result<(), Self::Error>;

    /// Execute an action on the active player
    ///
    /// Returns None if no player is active, otherwise returns the result of the action
    async fn control_active_player<F, R>(&self, action: F) -> Option<Result<R, Self::Error>>
    where
        F: FnOnce(PlayerId) -> Pin<Box<dyn Future<Output = Result<R, Self::Error>> + Send>> + Send,
        R: Send;
}
