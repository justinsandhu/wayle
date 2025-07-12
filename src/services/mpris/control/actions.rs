use std::{collections::HashMap, error::Error, sync::Arc, time::Duration};

use async_trait::async_trait;
use tokio::sync::RwLock;
use zbus::zvariant::ObjectPath;

use crate::services::mpris::{
    LoopMode, MediaError, MediaPlayer2PlayerProxy, MprisMediaService, PlayerId, PlayerStateTracker,
    utils,
};

/// Media control operations struct
pub struct MediaControl {
    /// Map of active players and their state trackers
    players: Arc<RwLock<HashMap<PlayerId, PlayerStateTracker>>>,
}

impl MediaControl {
    /// Create a new media control instance
    pub fn new(players: Arc<RwLock<HashMap<PlayerId, PlayerStateTracker>>>) -> Self {
        Self { players }
    }

    /// Get player proxy for the given player ID
    ///
    /// # Errors
    /// Returns error if player is not found
    pub async fn get_player_proxy(
        &self,
        player_id: &PlayerId,
    ) -> Result<MediaPlayer2PlayerProxy<'static>, MediaError> {
        let players = self.players.read().await;
        let tracker = players
            .get(player_id)
            .ok_or_else(|| MediaError::PlayerNotFound(player_id.clone()))?;
        Ok(tracker.player_proxy.clone())
    }

    /// Get current loop mode for a player
    ///
    /// # Errors
    /// Returns error if D-Bus communication fails
    pub async fn get_current_loop_mode(
        &self,
        proxy: &MediaPlayer2PlayerProxy<'_>,
    ) -> Result<LoopMode, MediaError> {
        let status = proxy.loop_status().await.map_err(MediaError::DbusError)?;
        Ok(LoopMode::from(status.as_str()))
    }

    /// Toggle play/pause state for a specific player
    ///
    /// # Errors
    /// Returns error if player is not found or doesn't support play/pause
    pub async fn play_pause(&self, player_id: PlayerId) -> Result<(), MediaError> {
        let proxy = self.get_player_proxy(&player_id).await?;
        proxy.play_pause().await.map_err(MediaError::DbusError)
    }

    /// Skip to next track for a specific player
    ///
    /// # Errors
    /// Returns error if player is not found or doesn't support next track
    pub async fn next(&self, player_id: PlayerId) -> Result<(), MediaError> {
        let proxy = self.get_player_proxy(&player_id).await?;
        proxy.next().await.map_err(MediaError::DbusError)
    }

    /// Skip to previous track for a specific player
    ///
    /// # Errors
    /// Returns error if player is not found or doesn't support previous track
    pub async fn previous(&self, player_id: PlayerId) -> Result<(), MediaError> {
        let proxy = self.get_player_proxy(&player_id).await?;
        proxy.previous().await.map_err(MediaError::DbusError)
    }

    /// Seek to a specific position in the current track
    ///
    /// # Errors
    /// Returns error if player is not found, doesn't support seeking, or position is invalid
    pub async fn seek(&self, player_id: PlayerId, position: Duration) -> Result<(), MediaError> {
        let proxy = self.get_player_proxy(&player_id).await?;

        let track_id_str = {
            let players = self.players.read().await;
            let Some(tracker) = players.get(&player_id) else {
                return Err(MediaError::PlayerNotFound(player_id));
            };

            if let Some(length) = tracker.last_metadata.length {
                if position > length {
                    return Err(MediaError::InvalidSeekPosition {
                        position,
                        length: Some(length),
                    });
                }
            }

            tracker
                .last_metadata
                .track_id
                .clone()
                .unwrap_or_else(|| "/".to_string())
        };

        let track_id = ObjectPath::try_from(track_id_str.as_str())
            .map_err(|e| MediaError::DbusError(e.into()))?;
        let position_micros = utils::to_mpris_micros(position);

        proxy
            .set_position(&track_id, position_micros)
            .await
            .map_err(MediaError::DbusError)
    }

    /// Toggle loop mode for a specific player
    ///
    /// # Errors
    /// Returns error if player is not found or doesn't support loop mode changes
    pub async fn toggle_loop(&self, player_id: PlayerId) -> Result<(), MediaError> {
        let proxy = self.get_player_proxy(&player_id).await?;

        let current_mode = self.get_current_loop_mode(&proxy).await?;
        let next_mode = match current_mode {
            LoopMode::None => LoopMode::Track,
            LoopMode::Track => LoopMode::Playlist,
            LoopMode::Playlist => LoopMode::None,
            LoopMode::Unsupported => {
                return Err(MediaError::UnsupportedOperation {
                    player: player_id,
                    operation: "loop".to_string(),
                });
            }
        };

        let mpris_mode: &str = next_mode.into();
        proxy
            .set_loop_status(mpris_mode)
            .await
            .map_err(MediaError::DbusError)
    }

    /// Toggle shuffle mode for a specific player
    ///
    /// # Errors
    /// Returns error if player is not found or doesn't support shuffle mode changes
    pub async fn toggle_shuffle(&self, player_id: PlayerId) -> Result<(), MediaError> {
        let proxy = self.get_player_proxy(&player_id).await?;

        let current_shuffle = proxy.shuffle().await.map_err(MediaError::DbusError)?;

        proxy
            .set_shuffle(!current_shuffle)
            .await
            .map_err(MediaError::DbusError)
    }
}

/// Media control trait operations
#[async_trait]
pub trait MediaController {
    /// Error type for control operations
    type Error: Error + Send + Sync + 'static;

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
}

#[async_trait]
impl MediaController for MprisMediaService {
    type Error = MediaError;

    async fn play_pause(&self, player_id: PlayerId) -> Result<(), Self::Error> {
        self.control.play_pause(player_id).await
    }

    async fn next(&self, player_id: PlayerId) -> Result<(), Self::Error> {
        self.control.next(player_id).await
    }

    async fn previous(&self, player_id: PlayerId) -> Result<(), Self::Error> {
        self.control.previous(player_id).await
    }

    async fn seek(&self, player_id: PlayerId, position: Duration) -> Result<(), Self::Error> {
        self.control.seek(player_id, position).await
    }

    async fn toggle_loop(&self, player_id: PlayerId) -> Result<(), Self::Error> {
        self.control.toggle_loop(player_id).await
    }

    async fn toggle_shuffle(&self, player_id: PlayerId) -> Result<(), Self::Error> {
        self.control.toggle_shuffle(player_id).await
    }
}
