use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{RwLock, broadcast};
use zbus::Connection;

use crate::services::mpris::proxy::MediaPlayer2PlayerProxy;
use crate::services::mpris::{PlayerEvent, PlayerId, PlayerInfo, PlayerState};

/// Core shared state for the MPRIS service
///
/// This contains only the essential shared data that subsystems need to access.
/// Business logic lives in the subsystems, not here.
pub struct Core {
    /// All discovered players and their state
    pub players: RwLock<HashMap<PlayerId, PlayerHandle>>,

    /// Currently active player for quick access
    pub active_player: RwLock<Option<PlayerId>>,

    /// Players to ignore during discovery
    pub ignored_players: RwLock<Vec<String>>,

    /// Event broadcasting for reactive updates
    pub events: broadcast::Sender<PlayerEvent>,

    /// Shared D-Bus connection
    pub connection: Connection,
}

/// Per-player state and resources
pub struct PlayerHandle {
    /// Player information (identity, capabilities)
    pub info: PlayerInfo,

    /// Current player state (playback, position, etc.)
    pub state: PlayerState,

    /// D-Bus proxy for controlling this player
    pub proxy: MediaPlayer2PlayerProxy<'static>,

    /// Handle to the monitoring task
    pub monitor_handle: tokio::task::JoinHandle<()>,
}

impl PlayerHandle {
    /// Create a new player handle
    pub fn new(
        info: PlayerInfo,
        state: PlayerState,
        proxy: MediaPlayer2PlayerProxy<'static>,
        monitor_handle: tokio::task::JoinHandle<()>,
    ) -> Self {
        Self {
            info,
            state,
            proxy,
            monitor_handle,
        }
    }
}

impl Drop for PlayerHandle {
    fn drop(&mut self) {
        self.monitor_handle.abort();
    }
}

impl Core {
    /// Create a new core state
    pub async fn new(connection: Connection, ignored_players: Vec<String>) -> Arc<Self> {
        let (events_tx, _) = broadcast::channel(1024);

        Arc::new(Self {
            players: RwLock::new(HashMap::new()),
            active_player: RwLock::new(None),
            ignored_players: RwLock::new(ignored_players),
            events: events_tx,
            connection,
        })
    }
}

