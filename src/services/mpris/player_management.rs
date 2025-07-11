use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;
use zbus::Connection;

use crate::runtime_state::RuntimeState;

use super::{
    MediaError, PlayerId, PlayerStateTracker, discovery::PlayerDiscovery,
    monitoring::PlayerMonitoring,
};

/// Player management functionality
pub struct PlayerManager {
    /// D-Bus session connection
    pub connection: Connection,

    /// Map of active players and their state trackers
    pub players: Arc<RwLock<HashMap<PlayerId, PlayerStateTracker>>>,

    /// Currently active player ID
    pub active_player: Arc<RwLock<Option<PlayerId>>>,

    /// Player discovery handler
    pub discovery: PlayerDiscovery,

    /// Player monitoring handler
    pub monitoring: PlayerMonitoring,

    /// Handle to the player discovery task
    pub discovery_handle: Option<tokio::task::JoinHandle<()>>,

    /// List of player bus names to ignore during discovery
    pub ignored_players: Arc<RwLock<Vec<String>>>,
}

impl PlayerManager {
    /// Create a new player manager
    pub fn new(
        connection: Connection,
        players: Arc<RwLock<HashMap<PlayerId, PlayerStateTracker>>>,
        active_player: Arc<RwLock<Option<PlayerId>>>,
        discovery: PlayerDiscovery,
        monitoring: PlayerMonitoring,
        ignored_players: Arc<RwLock<Vec<String>>>,
    ) -> Self {
        Self {
            connection,
            players,
            active_player,
            discovery,
            monitoring,
            discovery_handle: None,
            ignored_players,
        }
    }

    /// Start player discovery
    ///
    /// # Errors
    /// Returns error if discovery initialization fails
    pub async fn start_discovery(&mut self) -> Result<(), MediaError> {
        let discovery_clone = self.discovery.clone();
        let discovery_handle = tokio::spawn(async move {
            if let Err(e) = discovery_clone.start_discovery().await {
                eprintln!("Player discovery failed: {e}");
            }
        });

        self.discovery_handle = Some(discovery_handle);
        self.discovery.discover_existing_players().await?;
        self.validate_loaded_active_player().await;
        Ok(())
    }

    /// Load active player from runtime state file
    pub async fn load_active_player_from_file() -> Option<PlayerId> {
        if let Ok(Some(player_bus_name)) =
            RuntimeState::get_active_player().await
        {
            Some(PlayerId::from_bus_name(&player_bus_name))
        } else {
            None
        }
    }

    /// Save active player to runtime state file
    pub async fn save_active_player_to_file(&self, player_id: Option<PlayerId>) {
        let player_bus_name = player_id.map(|p| p.bus_name().to_string());
        let _ = RuntimeState::set_active_player(player_bus_name).await;
    }

    /// Find and set a fallback player when the current active player is invalid
    pub async fn find_and_set_fallback_player(&self) -> Option<PlayerId> {
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

    /// Get active player
    pub async fn active_player(&self) -> Option<PlayerId> {
        let active = self.active_player.read().await;

        if let Some(ref player_id) = *active {
            let players = self.players.read().await;

            if players.contains_key(player_id) {
                return active.clone();
            }

            self.find_and_set_fallback_player().await
        } else {
            None
        }
    }

    /// Set the active player
    ///
    /// # Errors
    /// Returns error if the specified player is not found
    pub async fn set_active_player(&self, player_id: Option<PlayerId>) -> Result<(), MediaError> {
        if let Some(ref id) = player_id {
            let players = self.players.read().await;

            if !players.contains_key(id) {
                return Err(MediaError::PlayerNotFound(id.clone()));
            }
        }

        let mut active = self.active_player.write().await;
        *active = player_id.clone();

        self.save_active_player_to_file(player_id).await;
        Ok(())
    }

    /// Shutdown the player manager and clean up all resources
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
}

impl Drop for PlayerManager {
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
