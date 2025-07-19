use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use futures::Stream;
use tracing::{info, instrument};
use zbus::Connection;

use crate::services::mpris::{
    core::Core,
    error::MediaError,
    streams,
    subsystems::{control, discovery::Discovery, management, monitoring, query},
    types::{
        LoopMode, PlaybackState, PlayerId, PlayerInfo, PlayerState, ShuffleMode, TrackMetadata,
    },
};

/// Configuration for the MPRIS service
#[derive(Default)]
pub struct Config {
    /// Patterns to ignore when discovering players
    pub ignored_players: Vec<String>,
}

/// MPRIS media service
///
/// Provides reactive media player control through D-Bus MPRIS protocol.
/// This is a thin facade that delegates to subsystems for actual functionality.
#[derive(Clone)]
pub struct MprisService {
    /// Core shared state
    core: Arc<Core>,

    /// Discovery subsystem handle (kept alive for its Drop impl)
    _discovery: Option<Arc<Discovery>>,

    /// Event monitoring task handle
    event_monitor: Option<Arc<tokio::task::JoinHandle<()>>>,
}

impl MprisService {
    /// Create a new MPRIS service (compatibility method)
    ///
    /// This is a compatibility method for the old API. Prefer using `start()` instead.
    ///
    /// # Errors
    /// Returns error if D-Bus connection or discovery initialization fails
    pub async fn new(ignored_players: Vec<String>) -> Result<Self, MediaError> {
        Self::start(Config { ignored_players }).await
    }

    /// Start the MPRIS service
    ///
    /// This will establish a D-Bus connection, discover existing players,
    /// and begin monitoring for new players.
    ///
    /// # Errors
    /// Returns error if D-Bus connection or discovery initialization fails
    #[instrument(skip(config))]
    pub async fn start(config: Config) -> Result<Self, MediaError> {
        info!("Starting MPRIS service");

        let connection = Connection::session().await.map_err(|e| {
            MediaError::InitializationFailed(format!("D-Bus connection failed: {e}"))
        })?;

        let core = Core::new(connection, config.ignored_players).await;

        if let Some(player_id) = management::load_active_player().await {
            let mut active = core.active_player.write().await;
            *active = Some(player_id);
        }

        let discovery = Discovery::start(Arc::clone(&core)).await?;

        let core_clone = Arc::clone(&core);
        let event_monitor = tokio::spawn(async move {
            monitor_events(core_clone).await;
        });

        info!("MPRIS service started successfully");
        Ok(Self {
            core,
            _discovery: Some(Arc::new(discovery)),
            event_monitor: Some(Arc::new(event_monitor)),
        })
    }

    /// Get a list of all current players
    pub async fn list_players(&self) -> Vec<PlayerState> {
        query::list_players(&self.core).await
    }

    /// Get a stream of player list updates
    ///
    /// This returns a stream that yields the current players immediately,
    /// then yields updates whenever players are added or removed.
    pub fn players(&self) -> impl Stream<Item = Vec<PlayerState>> + Send {
        streams::players(&self.core)
    }

    /// Get information about a specific player
    pub async fn player_info(&self, player_id: &PlayerId) -> Option<PlayerInfo> {
        query::player_info(&self.core, player_id).await
    }

    /// Get the current state of a specific player
    pub async fn player_state_snapshot(&self, player_id: &PlayerId) -> Option<PlayerState> {
        query::player_state(&self.core, player_id).await
    }

    /// Get a stream of state updates for a specific player
    ///
    /// This returns a stream that yields the current state immediately,
    /// then yields updates whenever the state changes.
    pub fn player_state(&self, player_id: PlayerId) -> impl Stream<Item = PlayerState> + Send {
        streams::player_state(&self.core, player_id)
    }

    /// Get a stream of playback state changes for a specific player
    pub fn playback_state(&self, player_id: PlayerId) -> impl Stream<Item = PlaybackState> + Send {
        streams::playback_state(&self.core, player_id)
    }

    /// Get a stream of track metadata changes for a specific player
    pub fn metadata(&self, player_id: PlayerId) -> impl Stream<Item = TrackMetadata> + Send {
        streams::metadata(&self.core, player_id)
    }

    /// Get a stream of position updates for a specific player
    pub fn position(&self, player_id: PlayerId) -> impl Stream<Item = Duration> + Send {
        streams::position(&self.core, player_id)
    }

    /// Get the current playback position for a player
    pub async fn current_position(&self, player_id: &PlayerId) -> Option<Duration> {
        query::player_state(&self.core, player_id)
            .await
            .map(|state| state.position)
    }

    /// Get a stream of loop mode changes for a specific player
    pub fn loop_mode(&self, player_id: PlayerId) -> impl Stream<Item = LoopMode> + Send {
        streams::loop_mode(&self.core, player_id)
    }

    /// Get a stream of shuffle mode changes for a specific player  
    pub fn shuffle_mode(&self, player_id: PlayerId) -> impl Stream<Item = ShuffleMode> + Send {
        streams::shuffle_mode(&self.core, player_id)
    }

    /// Get a stream of active player changes
    pub fn active_player(&self) -> impl Stream<Item = Option<PlayerId>> + Send {
        streams::active_player(&self.core)
    }

    /// Get the currently active player
    pub async fn get_active_player(&self) -> Option<PlayerId> {
        query::active_player(&self.core).await
    }

    /// Get the list of ignored player patterns
    pub async fn ignored_players(&self) -> Vec<String> {
        query::ignored_patterns(&self.core).await
    }

    /// Toggle play/pause for a player
    ///
    /// # Errors
    /// Returns error if player not found or D-Bus operation fails
    pub async fn play_pause(&self, player_id: PlayerId) -> Result<(), MediaError> {
        control::play_pause(&self.core, player_id).await
    }

    /// Start playback
    ///
    /// # Errors
    /// Returns error if player not found or D-Bus operation fails
    pub async fn play(&self, player_id: PlayerId) -> Result<(), MediaError> {
        control::play(&self.core, player_id).await
    }

    /// Pause playback
    ///
    /// # Errors
    /// Returns error if player not found or D-Bus operation fails
    pub async fn pause(&self, player_id: PlayerId) -> Result<(), MediaError> {
        control::pause(&self.core, player_id).await
    }

    /// Stop playback
    ///
    /// # Errors
    /// Returns error if player not found or D-Bus operation fails
    pub async fn stop(&self, player_id: PlayerId) -> Result<(), MediaError> {
        control::stop(&self.core, player_id).await
    }

    /// Skip to next track
    ///
    /// # Errors
    /// Returns error if player not found, doesn't support next, or D-Bus operation fails
    pub async fn next(&self, player_id: PlayerId) -> Result<(), MediaError> {
        control::next(&self.core, player_id).await
    }

    /// Go to previous track
    ///
    /// # Errors
    /// Returns error if player not found, doesn't support previous, or D-Bus operation fails
    pub async fn previous(&self, player_id: PlayerId) -> Result<(), MediaError> {
        control::previous(&self.core, player_id).await
    }

    /// Seek to a position
    ///
    /// # Errors
    /// Returns error if player not found, doesn't support seeking, or D-Bus operation fails
    pub async fn seek(&self, player_id: PlayerId, position: Duration) -> Result<(), MediaError> {
        control::seek(&self.core, player_id, position).await
    }

    /// Toggle loop mode
    ///
    /// # Errors
    /// Returns error if player not found, doesn't support loop modes, or D-Bus operation fails
    pub async fn toggle_loop(&self, player_id: PlayerId) -> Result<(), MediaError> {
        control::toggle_loop(&self.core, player_id).await
    }

    /// Set loop mode
    ///
    /// # Errors
    /// Returns error if player not found, doesn't support loop modes, or D-Bus operation fails
    pub async fn set_loop_mode(
        &self,
        player_id: PlayerId,
        mode: LoopMode,
    ) -> Result<(), MediaError> {
        control::set_loop_mode(&self.core, player_id, mode).await
    }

    /// Toggle shuffle mode
    ///
    /// # Errors
    /// Returns error if player not found, doesn't support shuffle, or D-Bus operation fails
    pub async fn toggle_shuffle(&self, player_id: PlayerId) -> Result<(), MediaError> {
        control::toggle_shuffle(&self.core, player_id).await
    }

    /// Set shuffle mode
    ///
    /// # Errors
    /// Returns error if player not found, doesn't support shuffle, or D-Bus operation fails
    pub async fn set_shuffle_mode(
        &self,
        player_id: PlayerId,
        mode: ShuffleMode,
    ) -> Result<(), MediaError> {
        control::set_shuffle_mode(&self.core, player_id, mode).await
    }

    /// Set the active player
    ///
    /// # Errors
    /// Returns error if player not found when specified
    pub async fn set_active_player(&self, player_id: Option<PlayerId>) -> Result<(), MediaError> {
        management::set_active_player(&self.core, player_id).await
    }

    /// Set patterns for players to ignore
    pub async fn set_ignored_players(&self, patterns: Vec<String>) {
        management::set_ignored_players(&self.core, patterns).await
    }

    /// Execute an action on the active player
    pub async fn control_active_player<F, R>(&self, action: F) -> Option<Result<R, MediaError>>
    where
        F: FnOnce(PlayerId) -> Pin<Box<dyn Future<Output = Result<R, MediaError>> + Send>> + Send,
        R: Send,
    {
        let active_id = self.get_active_player().await?;
        Some(action(active_id).await)
    }
}

impl Drop for MprisService {
    fn drop(&mut self) {
        if let Some(monitor) = self.event_monitor.take() {
            if let Ok(handle) = Arc::try_unwrap(monitor) {
                handle.abort();
            }
        }
    }
}

/// Monitor events and update core state
async fn monitor_events(core: Arc<Core>) {
    let mut events_rx = core.events.subscribe();

    while let Ok(event) = events_rx.recv().await {
        monitoring::update_player_state(&core, &event).await;
    }
}
