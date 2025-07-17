use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc, time::Duration};

use async_stream::stream;
use async_trait::async_trait;
use futures::Stream;
use tokio::sync::{RwLock, broadcast};
use tracing::{info, instrument};
use zbus::Connection;

/// Thread-safe storage for media player state trackers
pub type PlayerStore = Arc<RwLock<HashMap<PlayerId, player::PlayerStateTracker>>>;

/// Channel sender for player list updates
pub type PlayerListSender = broadcast::Sender<Vec<PlayerId>>;

/// Channel sender for player events
pub type PlayerEventSender = broadcast::Sender<PlayerEvent>;

/// Media control operations
pub mod controller;
/// Player discovery and lifecycle management
/// Media player error types
pub mod error;
/// Player management functionality
/// Player types and capabilities
pub mod player;
/// D-Bus proxy trait definitions
pub mod proxy;
/// Domain service trait definitions
pub mod service;
/// Track-related types
pub mod track;
/// MPRIS utility functions
pub mod utils;

pub use error::*;
pub use player::{PlayerCapabilities, PlayerEvent, PlayerId, PlayerInfo, PlayerState, state::*};
pub use proxy::*;
pub use service::*;
pub use track::*;

use controller::MediaControl;
use player::discovery::PlayerDiscovery;
use player::manager::PlayerManagement;
use player::monitoring::PlayerMonitoring;

/// MPRIS-based media service implementation
///
/// Provides reactive media player control through D-Bus MPRIS protocol.
/// Automatically discovers players and provides streams for UI updates.
pub struct MprisMediaService {
    /// Player management functionality
    player_manager: PlayerManagement,

    /// Media control operations
    control: MediaControl,

    /// Broadcast channel for player list updates
    player_list_tx: PlayerListSender,

    /// Broadcast channel for player events
    events_tx: PlayerEventSender,
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
    #[instrument(skip(ignored_players))]
    pub async fn new(ignored_players: Vec<String>) -> Result<Self, MediaError> {
        info!("Initializing MPRIS media service");
        let connection = Connection::session().await.map_err(|e| {
            MediaError::InitializationFailed(format!("D-Bus connection failed: {e}"))
        })?;
        info!("D-Bus session connection established");

        let (player_list_tx, _) = broadcast::channel(32);
        let (events_tx, _) = broadcast::channel(1024);

        let players = Arc::new(RwLock::new(HashMap::new()));
        let persisted_active = PlayerManagement::load_active_player_from_file().await;
        let active_player = Arc::new(RwLock::new(persisted_active));
        let ignored_players = Arc::new(RwLock::new(ignored_players));

        info!("Setting up player discovery and monitoring");
        let discovery = PlayerDiscovery::new(
            connection.clone(),
            Arc::clone(&players),
            player_list_tx.clone(),
            events_tx.clone(),
            Arc::clone(&active_player),
            Arc::clone(&ignored_players),
        );

        let monitoring = PlayerMonitoring::new(Arc::clone(&players), events_tx.clone());
        let control = MediaControl::new(Arc::clone(&players));

        let mut player_manager = PlayerManagement::new(
            connection,
            players,
            active_player,
            discovery,
            monitoring,
            ignored_players,
        );

        info!("Starting MPRIS player discovery");
        player_manager.start_discovery().await?;
        info!("MPRIS service fully initialized");

        Ok(Self {
            player_manager,
            control,
            player_list_tx: player_list_tx.clone(),
            events_tx: events_tx.clone(),
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
            player_manager: PlayerManagement::new(
                self.player_manager.connection.clone(),
                Arc::clone(&self.player_manager.players),
                Arc::clone(&self.player_manager.active_player),
                self.player_manager.discovery.clone(),
                self.player_manager.monitoring.clone(),
                Arc::clone(&self.player_manager.ignored_players),
            ),
            control: MediaControl::new(Arc::clone(&self.player_manager.players)),
            player_list_tx: self.player_list_tx.clone(),
            events_tx: self.events_tx.clone(),
        }
    }
}

impl MprisMediaService {
    // Stream methods
    fn players_stream(&self) -> impl Stream<Item = Vec<PlayerId>> + Send {
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

    fn player_info_stream(
        &self,
        player_id: PlayerId,
    ) -> impl Stream<Item = Result<PlayerInfo, MediaError>> + Send {
        let players = Arc::clone(&self.player_manager.players);
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

    fn playback_state_stream(&self, player_id: PlayerId) -> impl Stream<Item = PlaybackState> + Send {
        let players = Arc::clone(&self.player_manager.players);
        let mut events_rx = self.events_tx.subscribe();

        stream! {
            let current_state = {
                let players_guard = players.read().await;
                players_guard.get(&player_id).map(|tracker| tracker.last_playback_state)
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

    fn position_stream(&self, player_id: PlayerId) -> impl Stream<Item = Duration> + Send {
        let players = Arc::clone(&self.player_manager.players);
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

    fn metadata_stream(&self, player_id: PlayerId) -> impl Stream<Item = TrackMetadata> + Send {
        let players = Arc::clone(&self.player_manager.players);
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

    fn loop_mode_stream(&self, player_id: PlayerId) -> impl Stream<Item = LoopMode> + Send {
        let players = Arc::clone(&self.player_manager.players);
        let mut events_rx = self.events_tx.subscribe();

        stream! {
            let current_mode = {
                let players_guard = players.read().await;
                players_guard.get(&player_id).map(|tracker| tracker.last_loop_mode)
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

    fn shuffle_mode_stream(&self, player_id: PlayerId) -> impl Stream<Item = ShuffleMode> + Send {
        let players = Arc::clone(&self.player_manager.players);
        let mut events_rx = self.events_tx.subscribe();

        stream! {
            let current_mode = {
                let players_guard = players.read().await;
                players_guard.get(&player_id).map(|tracker| tracker.last_shuffle_mode)
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

    fn player_state_stream(&self, player_id: PlayerId) -> impl Stream<Item = PlayerState> + Send {
        let players = Arc::clone(&self.player_manager.players);
        let mut events_rx = self.events_tx.subscribe();

        stream! {
            loop {
                let current_state = {
                    let players_guard = players.read().await;
                    players_guard.get(&player_id).map(|tracker| PlayerState {
                        player_info: tracker.info.clone(),
                        playback_state: tracker.last_playback_state,
                        position: tracker.last_position,
                        metadata: tracker.last_metadata.clone(),
                        loop_mode: tracker.last_loop_mode,
                        shuffle_mode: tracker.last_shuffle_mode,
                    })
                };

                if let Some(state) = current_state {
                    yield state;
                    break;
                }

                tokio::time::sleep(Duration::from_millis(100)).await;
            }

            while let Ok(event) = events_rx.recv().await {
                match event {
                    PlayerEvent::PlayerRemoved(id) if id == player_id => {
                        return;
                    }
                    PlayerEvent::PlayerAdded(_) => continue,
                    PlayerEvent::PlaybackStateChanged { player_id: id, .. }
                    | PlayerEvent::PositionChanged { player_id: id, .. }
                    | PlayerEvent::MetadataChanged { player_id: id, .. }
                    | PlayerEvent::LoopModeChanged { player_id: id, .. }
                    | PlayerEvent::ShuffleModeChanged { player_id: id, .. }
                    | PlayerEvent::CapabilitiesChanged { player_id: id, .. } => {
                        if id != player_id {
                            continue;
                        }

                        let state = {
                            let players_guard = players.read().await;
                            players_guard.get(&player_id).map(|tracker| PlayerState {
                                player_info: tracker.info.clone(),
                                playback_state: tracker.last_playback_state,
                                position: tracker.last_position,
                                metadata: tracker.last_metadata.clone(),
                                loop_mode: tracker.last_loop_mode,
                                shuffle_mode: tracker.last_shuffle_mode,
                            })
                        };

                        if let Some(state) = state {
                            yield state;
                        }
                    }
                    _ => continue,
                }
            }
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
        self.players_stream()
    }

    fn player_info(
        &self,
        player_id: PlayerId,
    ) -> impl Stream<Item = Result<PlayerInfo, Self::Error>> + Send {
        self.player_info_stream(player_id)
    }

    fn playback_state(&self, player_id: PlayerId) -> impl Stream<Item = PlaybackState> + Send {
        self.playback_state_stream(player_id)
    }

    fn position(&self, player_id: PlayerId) -> impl Stream<Item = Duration> + Send {
        self.position_stream(player_id)
    }

    fn metadata(&self, player_id: PlayerId) -> impl Stream<Item = TrackMetadata> + Send {
        self.metadata_stream(player_id)
    }

    fn loop_mode(&self, player_id: PlayerId) -> impl Stream<Item = LoopMode> + Send {
        self.loop_mode_stream(player_id)
    }

    fn shuffle_mode(&self, player_id: PlayerId) -> impl Stream<Item = ShuffleMode> + Send {
        self.shuffle_mode_stream(player_id)
    }

    fn player_state(&self, player_id: PlayerId) -> impl Stream<Item = PlayerState> + Send {
        self.player_state_stream(player_id)
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
