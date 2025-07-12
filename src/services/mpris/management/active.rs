use std::{error::Error, future::Future, pin::Pin};

use async_trait::async_trait;

use crate::services::mpris::{MediaError, MprisMediaService, PlayerId};

/// Active player management operations
#[async_trait]
pub trait PlayerManager {
    /// Error type for player management operations
    type Error: Error + Send + Sync + 'static;

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

#[async_trait]
impl PlayerManager for MprisMediaService {
    type Error = MediaError;

    async fn active_player(&self) -> Option<PlayerId> {
        self.player_manager.active_player().await
    }

    async fn set_active_player(&self, player_id: Option<PlayerId>) -> Result<(), Self::Error> {
        self.player_manager.set_active_player(player_id).await
    }

    async fn control_active_player<F, R>(&self, action: F) -> Option<Result<R, Self::Error>>
    where
        F: FnOnce(PlayerId) -> Pin<Box<dyn Future<Output = Result<R, Self::Error>> + Send>> + Send,
        R: Send,
    {
        let active_id = self.active_player().await?;
        Some(action(active_id).await)
    }
}
