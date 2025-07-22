use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use futures::Stream;
use tokio::sync::RwLock;
use tracing::{info, instrument};
use zbus::Connection;

use crate::services::common::Property;

use super::player::{Control, Player, PlayerDiscovery, PlayerHandle};
use super::{LoopMode, MediaError, PlayerId, ShuffleMode, Volume};

/// Configuration for the MPRIS service
#[derive(Default)]
pub struct Config {
    /// Patterns to ignore when discovering players
    pub ignored_players: Vec<String>,
}

/// MPRIS service with reactive property-based architecture.
///
/// Provides fine-grained reactive updates for efficient UI rendering.
#[derive(Clone)]
pub struct MprisService {
    connection: Connection,
    players: Arc<RwLock<HashMap<PlayerId, PlayerHandle>>>,
    player_list: Property<Vec<Arc<Player>>>,
    active_player: Property<Option<PlayerId>>,
    ignored_patterns: Vec<String>,
}

impl MprisService {
    /// Create a new MPRIS service (compatibility method).
    ///
    /// This is a compatibility method for the old API. Prefer using `start()` instead.
    ///
    /// # Errors
    ///
    /// Returns `MediaError::InitializationFailed` if D-Bus connection fails
    pub async fn new(ignored_players: Vec<String>) -> Result<Self, MediaError> {
        Self::start(Config { ignored_players }).await
    }

    /// Start the MPRIS service with the given configuration.
    ///
    /// # Errors
    ///
    /// Returns `MediaError::InitializationFailed` if D-Bus connection fails
    #[instrument(skip(config))]
    pub async fn start(config: Config) -> Result<Self, MediaError> {
        info!("Starting MPRIS service with property-based architecture");

        let connection = Connection::session().await.map_err(|e| {
            MediaError::InitializationFailed(format!("D-Bus connection failed: {e}"))
        })?;

        let service = Self {
            connection,
            players: Arc::new(RwLock::new(HashMap::new())),
            player_list: Property::new(Vec::new()),
            active_player: Property::new(None),
            ignored_patterns: config.ignored_players,
        };

        PlayerDiscovery::start(
            &service.connection,
            &service.players,
            &service.player_list,
            &service.active_player,
            &service.ignored_patterns,
        )
        .await?;

        Ok(service)
    }

    /// Get a reactive player by ID.
    pub fn player(&self, player_id: &PlayerId) -> Option<Arc<Player>> {
        self.player_list
            .get()
            .into_iter()
            .find(|p| &p.id == player_id)
    }

    /// Get all players.
    pub fn players(&self) -> Vec<Arc<Player>> {
        self.player_list.get()
    }

    /// Watch for changes to the player list.
    ///
    /// Emits whenever players are added or removed from the system.
    pub fn watch_players(&self) -> impl Stream<Item = Vec<Arc<Player>>> + Send {
        self.player_list.watch()
    }

    /// Watch for changes to the active player.
    pub fn watch_active_player(&self) -> impl Stream<Item = Option<PlayerId>> + Send {
        self.active_player.watch()
    }

    /// Get the currently active player ID.
    pub fn active_player(&self) -> Option<PlayerId> {
        self.active_player.get()
    }

    /// Set the active player.
    ///
    /// # Errors
    ///
    /// Returns `MediaError::PlayerNotFound` if the specified player doesn't exist
    pub async fn set_active_player(&self, player_id: Option<PlayerId>) -> Result<(), MediaError> {
        if let Some(ref id) = player_id {
            let players = self.players.read().await;
            if !players.contains_key(id) {
                return Err(MediaError::PlayerNotFound(id.clone()));
            }
        }
        self.active_player.set(player_id);
        Ok(())
    }

    /// Get current playback position for a player.
    ///
    /// Position is polled on-demand rather than streamed.
    pub async fn position(&self, player_id: &PlayerId) -> Option<Duration> {
        let players = self.players.read().await;
        let handle = players.get(player_id)?;
        Control::position(handle, &self.connection).await
    }

    /// Watch position changes for a player.
    ///
    /// Polls position every second. Use `watch_position_with_interval` for custom intervals.
    pub fn watch_position(&self, player_id: PlayerId) -> impl Stream<Item = Duration> + Send {
        self.watch_position_with_interval(player_id, Duration::from_secs(1))
    }

    /// Watch position changes with a specified polling interval.
    ///
    /// Returns a stream that emits the current position at the specified interval.
    /// Only emits when position actually changes to avoid redundant updates.
    pub fn watch_position_with_interval(
        &self,
        player_id: PlayerId,
        interval: Duration,
    ) -> impl Stream<Item = Duration> + Send {
        let service = self.clone();
        async_stream::stream! {
            let mut last_position: Option<Duration> = None;

            loop {
                if let Some(position) = service.position(&player_id).await {
                    if last_position != Some(position) {
                        last_position = Some(position);
                        yield position;
                    }
                } else {
                    break;
                }
                tokio::time::sleep(interval).await;
            }
        }
    }

    /// Control playback for a player.
    ///
    /// # Errors
    ///
    /// Returns `MediaError::PlayerNotFound` if player doesn't exist,
    /// or `MediaError::ControlFailed` if the D-Bus operation fails
    pub async fn play_pause(&self, player_id: &PlayerId) -> Result<(), MediaError> {
        let players = self.players.read().await;
        let handle = players
            .get(player_id)
            .ok_or_else(|| MediaError::PlayerNotFound(player_id.clone()))?;
        Control::play_pause(handle).await
    }

    /// Skip to next track.
    ///
    /// # Errors
    ///
    /// Returns `MediaError::PlayerNotFound` if player doesn't exist,
    /// or `MediaError::ControlFailed` if the D-Bus operation fails
    pub async fn next(&self, player_id: &PlayerId) -> Result<(), MediaError> {
        let players = self.players.read().await;
        let handle = players
            .get(player_id)
            .ok_or_else(|| MediaError::PlayerNotFound(player_id.clone()))?;
        Control::next(handle).await
    }

    /// Go to previous track.
    ///
    /// # Errors
    ///
    /// Returns `MediaError::PlayerNotFound` if player doesn't exist,
    /// or `MediaError::ControlFailed` if the D-Bus operation fails
    pub async fn previous(&self, player_id: &PlayerId) -> Result<(), MediaError> {
        let players = self.players.read().await;
        let handle = players
            .get(player_id)
            .ok_or_else(|| MediaError::PlayerNotFound(player_id.clone()))?;
        Control::previous(handle).await
    }

    /// Seek by offset (relative position change).
    ///
    /// # Errors
    ///
    /// Returns `MediaError::PlayerNotFound` if player doesn't exist,
    /// or `MediaError::ControlFailed` if the D-Bus operation fails
    pub async fn seek(&self, player_id: &PlayerId, offset: Duration) -> Result<(), MediaError> {
        let players = self.players.read().await;
        let handle = players
            .get(player_id)
            .ok_or_else(|| MediaError::PlayerNotFound(player_id.clone()))?;
        Control::seek(handle, offset).await
    }

    /// Set position to an absolute value.
    ///
    /// # Errors
    ///
    /// Returns `MediaError::PlayerNotFound` if player doesn't exist,
    /// or `MediaError::ControlFailed` if the D-Bus operation fails
    pub async fn set_position(
        &self,
        player_id: &PlayerId,
        position: Duration,
    ) -> Result<(), MediaError> {
        let players = self.players.read().await;
        let handle = players
            .get(player_id)
            .ok_or_else(|| MediaError::PlayerNotFound(player_id.clone()))?;
        Control::set_position(handle, position).await
    }

    /// Set loop mode.
    ///
    /// # Errors
    ///
    /// Returns `MediaError::PlayerNotFound` if player doesn't exist,
    /// `MediaError::ControlFailed` if the D-Bus operation fails,
    /// or if the loop mode is unsupported
    pub async fn set_loop_mode(
        &self,
        player_id: &PlayerId,
        mode: LoopMode,
    ) -> Result<(), MediaError> {
        let players = self.players.read().await;
        let handle = players
            .get(player_id)
            .ok_or_else(|| MediaError::PlayerNotFound(player_id.clone()))?;
        Control::set_loop_mode(handle, mode).await
    }

    /// Set shuffle mode.
    ///
    /// # Errors
    ///
    /// Returns `MediaError::PlayerNotFound` if player doesn't exist,
    /// `MediaError::ControlFailed` if the D-Bus operation fails,
    /// or if shuffle is unsupported
    pub async fn set_shuffle_mode(
        &self,
        player_id: &PlayerId,
        mode: ShuffleMode,
    ) -> Result<(), MediaError> {
        let players = self.players.read().await;
        let handle = players
            .get(player_id)
            .ok_or_else(|| MediaError::PlayerNotFound(player_id.clone()))?;
        Control::set_shuffle_mode(handle, mode).await
    }

    /// Set volume.
    ///
    /// # Errors
    ///
    /// Returns `MediaError::PlayerNotFound` if player doesn't exist,
    /// or `MediaError::ControlFailed` if the D-Bus operation fails
    pub async fn set_volume(&self, player_id: &PlayerId, volume: Volume) -> Result<(), MediaError> {
        let players = self.players.read().await;
        let handle = players
            .get(player_id)
            .ok_or_else(|| MediaError::PlayerNotFound(player_id.clone()))?;
        Control::set_volume(handle, volume).await
    }

    /// Toggle loop mode to the next state.
    ///
    /// Cycles through: None → Track → Playlist → None
    ///
    /// # Errors
    ///
    /// Returns `MediaError::PlayerNotFound` if player doesn't exist,
    /// or `MediaError::OperationNotSupported` if loop mode is unsupported
    pub async fn toggle_loop(&self, player_id: &PlayerId) -> Result<(), MediaError> {
        let players = self.players.read().await;
        let handle = players
            .get(player_id)
            .ok_or_else(|| MediaError::PlayerNotFound(player_id.clone()))?;

        let current = handle.player.loop_mode.get();
        let next = match current {
            LoopMode::None => LoopMode::Track,
            LoopMode::Track => LoopMode::Playlist,
            LoopMode::Playlist => LoopMode::None,
            LoopMode::Unsupported => {
                return Err(MediaError::OperationNotSupported(
                    "Loop mode not supported".to_string(),
                ));
            }
        };

        drop(players);
        self.set_loop_mode(player_id, next).await
    }

    /// Toggle shuffle mode between on and off.
    ///
    /// # Errors
    ///
    /// Returns `MediaError::PlayerNotFound` if player doesn't exist,
    /// or `MediaError::OperationNotSupported` if shuffle is unsupported
    pub async fn toggle_shuffle(&self, player_id: &PlayerId) -> Result<(), MediaError> {
        let players = self.players.read().await;
        let handle = players
            .get(player_id)
            .ok_or_else(|| MediaError::PlayerNotFound(player_id.clone()))?;

        let current = handle.player.shuffle_mode.get();
        let next = match current {
            ShuffleMode::Off => ShuffleMode::On,
            ShuffleMode::On => ShuffleMode::Off,
            ShuffleMode::Unsupported => {
                return Err(MediaError::OperationNotSupported(
                    "Shuffle not supported".to_string(),
                ));
            }
        };

        drop(players);
        self.set_shuffle_mode(player_id, next).await
    }
}
