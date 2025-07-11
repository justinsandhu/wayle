use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc, time::Duration};

use async_stream::stream;
use async_trait::async_trait;
use futures::Stream;
use tokio::sync::{RwLock, broadcast};
use zbus::Connection;

/// Media control operations
pub mod control;
/// Player discovery and lifecycle management
pub mod discovery;
/// Media player error types
pub mod error;
/// Track metadata types
pub mod metadata;
/// Player property monitoring
pub mod monitoring;
/// Player types and capabilities
pub mod player;
/// Player management functionality
pub mod player_management;
/// D-Bus proxy trait definitions
pub mod proxy;
/// Domain service trait definitions
pub mod service;
/// MPRIS utility functions
pub mod utils;

pub use control::*;
pub use error::*;
pub use metadata::*;
pub use player::*;
pub use player_management::*;
pub use proxy::*;
pub use service::*;

use discovery::PlayerDiscovery;
use monitoring::PlayerMonitoring;

/// MPRIS-based media service implementation
///
/// Provides reactive media player control through D-Bus MPRIS protocol.
/// Automatically discovers players and provides streams for UI updates.
pub struct MprisMediaService {
    /// Player management functionality
    player_manager: PlayerManager,

    /// Media control operations
    control: MediaControl,

    /// Broadcast channel for player list updates
    player_list_tx: Arc<broadcast::Sender<Vec<PlayerId>>>,

    /// Broadcast channel for player events
    events_tx: Arc<broadcast::Sender<PlayerEvent>>,
}

impl MprisMediaService {
    /// Create a new MPRIS media service
    ///
    /// Initializes D-Bus connection and starts player discovery.
    /// Players matching any pattern in ignored_players will be skipped during discovery.
    ///
    /// # Arguments
    /// * `ignored_players` - List of patterns to match against player bus names for ignoring
    ///
    /// # Errors
    /// Returns error if D-Bus session connection fails or player discovery initialization fails
    pub async fn new(ignored_players: Vec<String>) -> Result<Self, MediaError> {
        let connection = Connection::session().await.map_err(|e| {
            MediaError::InitializationFailed(format!("D-Bus connection failed: {e}"))
        })?;

        let (player_list_tx, _) = broadcast::channel(32);
        let (events_tx, _) = broadcast::channel(1024);

        let players = Arc::new(RwLock::new(HashMap::new()));
        let persisted_active = PlayerManager::load_active_player_from_file().await;
        let active_player = Arc::new(RwLock::new(persisted_active));
        let player_list_tx = Arc::new(player_list_tx);
        let events_tx = Arc::new(events_tx);
        let ignored_players = Arc::new(RwLock::new(ignored_players));

        let discovery = PlayerDiscovery::new(
            connection.clone(),
            players.clone(),
            player_list_tx.clone(),
            events_tx.clone(),
            active_player.clone(),
            ignored_players.clone(),
        );

        let monitoring = PlayerMonitoring::new(players.clone(), events_tx.clone());
        let control = MediaControl::new(players.clone());

        let mut player_manager = PlayerManager::new(
            connection,
            players,
            active_player,
            discovery,
            monitoring,
            ignored_players,
        );

        player_manager.start_discovery().await?;

        Ok(Self {
            player_manager,
            control,
            player_list_tx,
            events_tx,
        })
    }

    /// Shutdown the service and clean up all resources
    pub async fn shutdown(&mut self) {
        self.player_manager.shutdown().await;
    }

    /// Configure which players to ignore during discovery
    ///
    /// Players matching any of the provided patterns will be ignored.
    /// Patterns are matched using `contains()` against the D-Bus bus name.
    ///
    /// # Arguments
    /// * `patterns` - List of patterns to match against player bus names
    pub async fn set_ignored_players(&self, patterns: Vec<String>) {
        self.player_manager.set_ignored_players(patterns).await;
    }

    /// Get currently ignored player patterns
    pub async fn get_ignored_players(&self) -> Vec<String> {
        self.player_manager.get_ignored_players().await
    }

    /// Check if a player should be ignored based on its bus name
    pub async fn should_ignore_player(&self, bus_name: &str) -> bool {
        self.player_manager.should_ignore_player(bus_name).await
    }
}

impl Clone for MprisMediaService {
    fn clone(&self) -> Self {
        Self {
            player_manager: PlayerManager::new(
                self.player_manager.connection.clone(),
                self.player_manager.players.clone(),
                self.player_manager.active_player.clone(),
                self.player_manager.discovery.clone(),
                self.player_manager.monitoring.clone(),
                self.player_manager.ignored_players.clone(),
            ),
            control: MediaControl::new(self.player_manager.players.clone()),
            player_list_tx: self.player_list_tx.clone(),
            events_tx: self.events_tx.clone(),
        }
    }
}

impl Drop for MprisMediaService {
    fn drop(&mut self) {
        // PlayerManager handles its own cleanup in Drop
    }
}

#[async_trait]
impl MediaService for MprisMediaService {
    type Error = MediaError;

    fn players(&self) -> impl Stream<Item = Vec<PlayerId>> + Send {
        let mut rx = self.player_list_tx.subscribe();

        stream! {
            let current_players: Vec<PlayerId> = {
                let players = self.player_manager.players.read().await;
                players.keys().cloned().collect()
            };
            yield current_players;

            while let Ok(players) = rx.recv().await {
                yield players;
            }
        }
    }

    fn player_info(
        &self,
        player_id: PlayerId,
    ) -> impl Stream<Item = Result<PlayerInfo, Self::Error>> + Send {
        let players = self.player_manager.players.clone();
        let mut events_rx = self.events_tx.subscribe();

        stream! {
            let current_info = {
                let players_guard = players.read().await;
                players_guard.get(&player_id).map(|tracker| tracker.info.clone())
            };

            if let Some(info) = current_info {
                yield Ok(info);
            }

            while let Ok(event) = events_rx.recv().await {
                match event {
                    PlayerEvent::PlayerRemoved(id) if id == player_id => {
                        return;
                    }
                    PlayerEvent::PlayerAdded(info) if info.id != player_id => {
                        continue;
                    }
                    PlayerEvent::PlayerAdded(info) => {
                        yield Ok(info);
                    }
                    PlayerEvent::CapabilitiesChanged { player_id: id, .. } if id != player_id => {
                        continue;
                    }
                    PlayerEvent::CapabilitiesChanged { capabilities, .. } => {
                        let updated_info = {
                            let players_guard = players.read().await;
                            players_guard.get(&player_id).map(|tracker| {
                                let mut info = tracker.info.clone();
                                info.capabilities = capabilities;
                                info
                            })
                        };

                        if let Some(info) = updated_info {
                            yield Ok(info);
                        }
                    }
                    _ => continue,
                }
            }
        }
    }

    fn playback_state(&self, player_id: PlayerId) -> impl Stream<Item = PlaybackState> + Send {
        let players = self.player_manager.players.clone();
        let mut events_rx = self.events_tx.subscribe();

        stream! {
            let current_state = {
                let players_guard = players.read().await;
                players_guard.get(&player_id).map(|tracker| tracker.last_playback_state.clone())
            };

            if let Some(state) = current_state {
                yield state;
            }

            while let Ok(event) = events_rx.recv().await {
                let PlayerEvent::PlaybackStateChanged { player_id: id, state } = event else {
                    continue;
                };

                if id != player_id {
                    continue;
                }

                yield state;
            }
        }
    }

    fn position(&self, player_id: PlayerId) -> impl Stream<Item = Duration> + Send {
        let players = self.player_manager.players.clone();
        let mut events_rx = self.events_tx.subscribe();

        stream! {
            let current_position = {
                let players_guard = players.read().await;
                players_guard.get(&player_id).map(|tracker| tracker.last_position)
            };

            if let Some(position) = current_position {
                yield position;
            }

            while let Ok(event) = events_rx.recv().await {
                let PlayerEvent::PositionChanged { player_id: id, position } = event else {
                    continue;
                };

                if id != player_id {
                    continue;
                }

                yield position;
            }
        }
    }

    fn metadata(&self, player_id: PlayerId) -> impl Stream<Item = TrackMetadata> + Send {
        let players = self.player_manager.players.clone();
        let mut events_rx = self.events_tx.subscribe();

        stream! {
            let current_metadata = {
                let players_guard = players.read().await;
                players_guard.get(&player_id).map(|tracker| tracker.last_metadata.clone())
            };

            if let Some(metadata) = current_metadata {
                yield metadata;
            }

            while let Ok(event) = events_rx.recv().await {
                let PlayerEvent::MetadataChanged { player_id: id, metadata } = event else {
                    continue;
                };

                if id != player_id {
                    continue;
                }

                yield metadata;
            }
        }
    }

    fn loop_mode(&self, player_id: PlayerId) -> impl Stream<Item = LoopMode> + Send {
        let players = self.player_manager.players.clone();
        let mut events_rx = self.events_tx.subscribe();

        stream! {
            let current_mode = {
                let players_guard = players.read().await;
                players_guard.get(&player_id).map(|tracker| tracker.last_loop_mode.clone())
            };

            if let Some(mode) = current_mode {
                yield mode;
            }

            while let Ok(event) = events_rx.recv().await {
                let PlayerEvent::LoopModeChanged { player_id: id, mode } = event else {
                    continue;
                };

                if id != player_id {
                    continue;
                }

                yield mode;
            }
        }
    }

    fn shuffle_mode(&self, player_id: PlayerId) -> impl Stream<Item = ShuffleMode> + Send {
        let players = self.player_manager.players.clone();
        let mut events_rx = self.events_tx.subscribe();

        stream! {
            let current_mode = {
                let players_guard = players.read().await;
                players_guard.get(&player_id).map(|tracker| tracker.last_shuffle_mode.clone())
            };

            if let Some(mode) = current_mode {
                yield mode;
            }

            while let Ok(event) = events_rx.recv().await {
                let PlayerEvent::ShuffleModeChanged { player_id: id, mode } = event else {
                    continue;
                };

                if id != player_id {
                    continue;
                }

                yield mode;
            }
        }
    }

    fn player_state(&self, player_id: PlayerId) -> impl Stream<Item = PlayerState> + Send {
        let players = self.player_manager.players.clone();
        let mut events_rx = self.events_tx.subscribe();

        stream! {
            let current_state = {
                let players_guard = players.read().await;
                players_guard.get(&player_id).map(|tracker| PlayerState {
                    player_info: tracker.info.clone(),
                    playback_state: tracker.last_playback_state.clone(),
                    metadata: tracker.last_metadata.clone(),
                    position: tracker.last_position,
                    loop_mode: tracker.last_loop_mode.clone(),
                    shuffle_mode: tracker.last_shuffle_mode.clone(),
                })
            };

            if let Some(state) = current_state {
                yield state;
            }

            while let Ok(event) = events_rx.recv().await {
                match event {
                    PlayerEvent::PlayerRemoved(id) if id == player_id => {
                        return;
                    }
                    PlayerEvent::PlaybackStateChanged { player_id: id, .. } |
                    PlayerEvent::MetadataChanged { player_id: id, .. } |
                    PlayerEvent::PositionChanged { player_id: id, .. } |
                    PlayerEvent::LoopModeChanged { player_id: id, .. } |
                    PlayerEvent::ShuffleModeChanged { player_id: id, .. } |
                    PlayerEvent::CapabilitiesChanged { player_id: id, .. } if id != player_id => {
                        continue;
                    }
                    PlayerEvent::PlaybackStateChanged { .. } |
                    PlayerEvent::MetadataChanged { .. } |
                    PlayerEvent::PositionChanged { .. } |
                    PlayerEvent::LoopModeChanged { .. } |
                    PlayerEvent::ShuffleModeChanged { .. } |
                    PlayerEvent::CapabilitiesChanged { .. } => {
                        let updated_state = {
                            let players_guard = players.read().await;
                            players_guard.get(&player_id).map(|tracker| PlayerState {
                                player_info: tracker.info.clone(),
                                playback_state: tracker.last_playback_state.clone(),
                                metadata: tracker.last_metadata.clone(),
                                position: tracker.last_position,
                                loop_mode: tracker.last_loop_mode.clone(),
                                shuffle_mode: tracker.last_shuffle_mode.clone(),
                            })
                        };

                        if let Some(state) = updated_state {
                            yield state;
                        }
                    }
                    _ => continue,
                }
            }
        }
    }

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
