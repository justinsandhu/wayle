use std::{collections::HashMap, sync::Arc};

use tokio::sync::{RwLock, broadcast};
use zbus::Connection;

use super::{
    LoopMode, MediaError, MediaPlayer2PlayerProxy, PlayerEvent, PlayerId,
    discovery::PlayerDiscovery, monitoring::PlayerMonitoring, player::PlayerStateTracker,
};

/// MPRIS-based media service implementation
///
/// Provides reactive media player control through D-Bus MPRIS protocol.
/// Automatically discovers players and provides streams for UI updates.
pub struct MprisMediaService {
    /// D-Bus session connection
    pub(super) connection: Connection,

    /// Map of active players and their state trackers
    pub(super) players: Arc<RwLock<HashMap<PlayerId, PlayerStateTracker>>>,

    /// Currently active player ID
    pub(super) active_player: Arc<RwLock<Option<PlayerId>>>,

    /// Broadcast channel for player list updates
    pub(super) player_list_tx: Arc<broadcast::Sender<Vec<PlayerId>>>,

    /// Broadcast channel for player events
    pub(super) events_tx: Arc<broadcast::Sender<PlayerEvent>>,

    /// Player discovery handler
    pub(super) discovery: PlayerDiscovery,

    /// Player monitoring handler
    pub(super) monitoring: PlayerMonitoring,

    /// Handle to the player discovery task
    discovery_handle: Option<tokio::task::JoinHandle<()>>,

    /// List of player bus names to ignore during discovery
    pub(super) ignored_players: Arc<RwLock<Vec<String>>>,
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
        let persisted_active = Self::load_active_player_from_file().await;
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

        let mut service = Self {
            connection,
            players,
            active_player,
            player_list_tx,
            events_tx,
            discovery,
            monitoring,
            discovery_handle: None,
            ignored_players,
        };

        let discovery_clone = service.discovery.clone();
        let discovery_handle = tokio::spawn(async move {
            if let Err(e) = discovery_clone.start_discovery().await {
                eprintln!("Player discovery failed: {e}");
            }
        });

        service.discovery_handle = Some(discovery_handle);
        service.discovery.discover_existing_players().await?;

        service.validate_loaded_active_player().await;

        Ok(service)
    }

    pub(super) async fn get_player_proxy(
        &self,
        player_id: &PlayerId,
    ) -> Result<MediaPlayer2PlayerProxy<'static>, MediaError> {
        let players = self.players.read().await;
        let tracker = players
            .get(player_id)
            .ok_or_else(|| MediaError::PlayerNotFound(player_id.clone()))?;
        Ok(tracker.player_proxy.clone())
    }

    pub(super) async fn get_current_loop_mode(
        &self,
        proxy: &MediaPlayer2PlayerProxy<'_>,
    ) -> Result<LoopMode, MediaError> {
        let status = proxy.loop_status().await.map_err(MediaError::DbusError)?;
        Ok(LoopMode::from(status.as_str()))
    }

    /// Shutdown the service and clean up all resources
    pub async fn shutdown(&mut self) {
        if let Some(handle) = self.discovery_handle.take() {
            handle.abort();
        }

        let mut players = self.players.write().await;
        for (_, mut tracker) in players.drain() {
            if let Some(handle) = tracker.monitoring_handle.take() {
                handle.abort();
            }
        }
    }

    /// Load active player from runtime state file
    async fn load_active_player_from_file() -> Option<PlayerId> {
        if let Ok(Some(player_bus_name)) =
            crate::runtime_state::RuntimeState::get_active_player().await
        {
            Some(PlayerId::from_bus_name(&player_bus_name))
        } else {
            None
        }
    }

    /// Save active player to runtime state file
    pub(super) async fn save_active_player_to_file(&self, player_id: Option<PlayerId>) {
        let player_bus_name = player_id.map(|p| p.bus_name().to_string());
        let _ = crate::runtime_state::RuntimeState::set_active_player(player_bus_name).await;
    }

    /// Find and set a fallback player when the current active player is invalid
    pub(super) async fn find_and_set_fallback_player(&self) -> Option<PlayerId> {
        let players = self.players.read().await;
        let fallback_player = players.keys().next().cloned();

        if let Some(ref player_id) = fallback_player {
            let mut active = self.active_player.write().await;
            *active = Some(player_id.clone());

            self.save_active_player_to_file(Some(player_id.clone()))
                .await;
        } else {
            let mut active = self.active_player.write().await;
            *active = None;

            self.save_active_player_to_file(None).await;
        }

        fallback_player
    }

    /// Validate the loaded active player after discovery completes
    async fn validate_loaded_active_player(&self) {
        let needs_fallback = {
            let active = self.active_player.read().await;
            if let Some(ref player_id) = *active {
                let players = self.players.read().await;
                !players.contains_key(player_id)
            } else {
                false
            }
        };

        if needs_fallback {
            self.find_and_set_fallback_player().await;
        }
    }

    /// Configure which players to ignore during discovery
    ///
    /// Players matching any of the provided patterns will be ignored.
    /// Patterns are matched using `contains()` against the D-Bus bus name.
    ///
    /// # Arguments
    /// * `patterns` - List of patterns to match against player bus names
    pub async fn set_ignored_players(&self, patterns: Vec<String>) {
        let mut ignored = self.ignored_players.write().await;
        *ignored = patterns;
    }

    /// Get currently ignored player patterns
    pub async fn get_ignored_players(&self) -> Vec<String> {
        let ignored = self.ignored_players.read().await;
        ignored.clone()
    }

    /// Check if a player should be ignored based on its bus name
    pub async fn should_ignore_player(&self, bus_name: &str) -> bool {
        self.discovery.should_ignore_player(bus_name).await
    }
}

impl Clone for MprisMediaService {
    fn clone(&self) -> Self {
        Self {
            connection: self.connection.clone(),
            players: self.players.clone(),
            active_player: self.active_player.clone(),
            player_list_tx: self.player_list_tx.clone(),
            events_tx: self.events_tx.clone(),
            discovery: self.discovery.clone(),
            monitoring: self.monitoring.clone(),
            discovery_handle: None,
            ignored_players: self.ignored_players.clone(),
        }
    }
}

impl Drop for MprisMediaService {
    fn drop(&mut self) {
        if let Some(handle) = self.discovery_handle.take() {
            handle.abort();
        }

        let mut players = match self.players.try_write() {
            Ok(players) => players,
            Err(_) => {
                eprintln!("Warning: Failed to acquire write lock during drop");
                return;
            }
        };

        for (_, mut tracker) in players.drain() {
            if let Some(handle) = tracker.monitoring_handle.take() {
                handle.abort();
            }
        }
    }
}
