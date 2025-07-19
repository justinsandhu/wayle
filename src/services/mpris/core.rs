use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{RwLock, broadcast};
use zbus::Connection;

use crate::services::mpris::proxy::MediaPlayer2PlayerProxy;
use crate::services::mpris::{PlayerEvent, PlayerId, Player};

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
    /// Complete player information
    pub player: Player,

    /// D-Bus proxy for controlling this player
    pub proxy: MediaPlayer2PlayerProxy<'static>,

    /// Handle to the monitoring task
    pub monitor_handle: tokio::task::JoinHandle<()>,
}

impl PlayerHandle {
    /// Create a new player handle
    pub fn new(
        player: Player,
        proxy: MediaPlayer2PlayerProxy<'static>,
        monitor_handle: tokio::task::JoinHandle<()>,
    ) -> Self {
        Self {
            player,
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

    /// Get a list of all current players
    pub async fn list_players(&self) -> Vec<Player> {
        self.players
            .read()
            .await
            .values()
            .map(|handle| handle.player.clone())
            .collect()
    }

    /// Get a specific player
    pub async fn get_player(&self, player_id: &PlayerId) -> Option<Player> {
        self.players
            .read()
            .await
            .get(player_id)
            .map(|handle| handle.player.clone())
    }

    /// Get the currently active player
    pub async fn active_player(&self) -> Option<PlayerId> {
        let active = self.active_player.read().await;

        if let Some(ref player_id) = *active {
            let players = self.players.read().await;
            if players.contains_key(player_id) {
                return active.clone();
            }
        }

        None
    }

    /// Get the list of ignored player patterns
    pub async fn ignored_patterns(&self) -> Vec<String> {
        self.ignored_players.read().await.clone()
    }

    /// Refresh all properties for a player from D-Bus
    ///
    /// This fetches all current properties from the player and updates
    /// the stored state. Useful when we need to ensure consistency.
    #[tracing::instrument(skip(self))]
    pub async fn refresh_player(
        &self,
        player_id: &PlayerId,
    ) -> Result<(), crate::services::mpris::error::MediaError> {
        use crate::services::mpris::types::{
            LoopMode, PlaybackState, ShuffleMode, TrackMetadata,
        };

        let mut players = self.players.write().await;

        if let Some(handle) = players.get_mut(player_id) {
            let proxy = &handle.proxy;

            let (
                playback_status,
                metadata_map,
                position,
                loop_status,
                shuffle,
                can_control,
                can_play,
                can_go_next,
                can_go_previous,
                can_seek,
            ) = tokio::try_join!(
                proxy.playback_status(),
                proxy.metadata(),
                proxy.position(),
                proxy.loop_status(),
                proxy.shuffle(),
                proxy.can_control(),
                proxy.can_play(),
                proxy.can_go_next(),
                proxy.can_go_previous(),
                proxy.can_seek(),
            )?;

            let can_loop = proxy.loop_status().await.is_ok();
            let can_shuffle = proxy.shuffle().await.is_ok();

            // Update all player fields
            let metadata = TrackMetadata::from(metadata_map);
            handle.player.playback_state = PlaybackState::from(playback_status.as_str());
            handle.player.position = std::time::Duration::from_micros(position.max(0) as u64);
            handle.player.loop_mode = LoopMode::from(loop_status.as_str());
            handle.player.shuffle_mode = ShuffleMode::from(shuffle);
            handle.player.title = metadata.title;
            handle.player.artist = metadata.artist;
            handle.player.album = metadata.album;
            handle.player.album_artist = metadata.album_artist;
            handle.player.length = metadata.length;
            handle.player.art_url = metadata.art_url;
            handle.player.track_id = metadata.track_id;
            handle.player.can_control = can_control;
            handle.player.can_play = can_play;
            handle.player.can_go_next = can_go_next;
            handle.player.can_go_previous = can_go_previous;
            handle.player.can_seek = can_seek;
            handle.player.can_shuffle = can_shuffle;
            handle.player.can_loop = can_loop;

            Ok(())
        } else {
            Err(crate::services::mpris::error::MediaError::PlayerNotFound(
                player_id.clone(),
            ))
        }
    }
}
