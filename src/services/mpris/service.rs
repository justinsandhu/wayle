use std::collections::HashMap;
use std::sync::Arc;

use futures::Stream;
use tokio::sync::RwLock;
use tracing::{info, instrument};
use zbus::Connection;

use crate::services::common::Property;

use super::{MediaError, PlayerId, core::Player, monitoring::MprisMonitoring};

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
    players: Arc<RwLock<HashMap<PlayerId, Arc<Player>>>>,
    player_list: Property<Vec<Arc<Player>>>,
    active_player: Property<Option<Arc<Player>>>,
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

        MprisMonitoring::start(
            service.connection.clone(),
            Arc::clone(&service.players),
            service.player_list.clone(),
            service.active_player.clone(),
            service.ignored_patterns.clone(),
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
    pub fn watch_active_player(&self) -> impl Stream<Item = Option<Arc<Player>>> + Send {
        self.active_player.watch()
    }

    /// Get the currently active player ID.
    pub fn active_player(&self) -> Option<Arc<Player>> {
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

        let player = match player_id {
            Some(ref id) => self.player(id),
            None => None,
        };

        self.active_player.set(player);

        Ok(())
    }
}
