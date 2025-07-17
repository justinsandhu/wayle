use std::{collections::HashMap, sync::Arc, time::Duration};

use futures::StreamExt;
use tokio::sync::RwLock;
use tracing::{info, instrument, warn};
use zbus::{Connection, fdo};

use super::{
    LoopMode, MediaError, MediaPlayer2PlayerProxy, MediaPlayer2Proxy, PlaybackState,
    PlayerCapabilities, PlayerEvent, PlayerEventSender, PlayerId, PlayerInfo, PlayerListSender,
    ShuffleMode, TrackMetadata, monitoring::PlayerMonitoring, player::state::PlayerStateTracker,
    utils,
};

/// Handles player discovery and lifecycle management
pub struct PlayerDiscovery {
    connection: Connection,
    players: Arc<RwLock<HashMap<PlayerId, PlayerStateTracker>>>,
    player_list_tx: PlayerListSender,
    events_tx: PlayerEventSender,
    active_player: Arc<RwLock<Option<PlayerId>>>,
    monitoring: PlayerMonitoring,
    ignored_players: Arc<RwLock<Vec<String>>>,
}

impl PlayerDiscovery {
    /// Create a new player discovery handler
    pub fn new(
        connection: Connection,
        players: Arc<RwLock<HashMap<PlayerId, PlayerStateTracker>>>,
        player_list_tx: PlayerListSender,
        events_tx: PlayerEventSender,
        active_player: Arc<RwLock<Option<PlayerId>>>,
        ignored_players: Arc<RwLock<Vec<String>>>,
    ) -> Self {
        let monitoring = PlayerMonitoring::new(players.clone(), events_tx.clone());

        Self {
            connection,
            players,
            player_list_tx,
            events_tx,
            active_player,
            monitoring,
            ignored_players,
        }
    }

    /// Start monitoring for new players
    ///
    /// # Errors
    /// Returns error if D-Bus proxy creation or signal subscription fails
    #[instrument(skip(self))]
    pub async fn start_discovery(&self) -> Result<(), MediaError> {
        info!("Starting MPRIS player discovery monitoring");
        let dbus_proxy = fdo::DBusProxy::new(&self.connection)
            .await
            .map_err(|e| MediaError::InitializationFailed(format!("DBus proxy failed: {e}")))?;

        let mut name_owner_changed =
            dbus_proxy.receive_name_owner_changed().await.map_err(|e| {
                MediaError::InitializationFailed(format!("Signal subscription failed: {e}"))
            })?;

        let discovery = self.clone();
        tokio::spawn(async move {
            while let Some(signal) = name_owner_changed.next().await {
                let Ok(args) = signal.args().map_err(MediaError::DbusError) else {
                    continue;
                };

                if !args.name().starts_with("org.mpris.MediaPlayer2.") {
                    continue;
                }

                let player_id = PlayerId::from_bus_name(args.name());

                match (args.old_owner().as_deref(), args.new_owner().as_deref()) {
                    (Some(_), None) => {
                        discovery.handle_player_removed(player_id).await;
                    }
                    (None, Some(_)) => {
                        if let Err(e) = discovery.handle_player_added(player_id).await {
                            warn!("Failed to add player: {e}");
                        }
                    }
                    _ => {}
                }
            }
        });

        info!("MPRIS player discovery monitoring started successfully");
        Ok(())
    }

    /// Discover existing players on the bus
    ///
    /// # Errors
    /// Returns error if D-Bus proxy creation or name listing fails
    #[instrument(skip(self))]
    pub async fn discover_existing_players(&self) -> Result<(), MediaError> {
        info!("Discovering existing MPRIS players on D-Bus");
        let dbus_proxy = fdo::DBusProxy::new(&self.connection)
            .await
            .map_err(|e| MediaError::InitializationFailed(format!("DBus proxy failed: {e}")))?;

        let names = dbus_proxy
            .list_names()
            .await
            .map_err(|e| MediaError::DbusError(e.into()))?;

        for name in names {
            if !name.starts_with("org.mpris.MediaPlayer2.") {
                continue;
            }

            let player_id = PlayerId::from_bus_name(&name);
            if let Err(e) = self.handle_player_added(player_id).await {
                warn!("Failed to add existing player {}: {}", name, e);
            }
        }

        info!("Finished discovering existing MPRIS players");
        Ok(())
    }

    /// Handle a new player being added to the bus
    ///
    /// # Errors
    /// Returns error if player proxy creation or monitoring setup fails
    #[instrument(skip(self), fields(bus_name = %player_id.bus_name()))]
    pub async fn handle_player_added(&self, player_id: PlayerId) -> Result<(), MediaError> {
        if self.should_ignore_player(player_id.bus_name()).await {
            info!("Ignoring player based on configuration");
            return Ok(());
        }

        info!("Adding new MPRIS player");
        let base_proxy = MediaPlayer2Proxy::builder(&self.connection)
            .destination(player_id.bus_name().to_string())
            .map_err(MediaError::DbusError)?
            .build()
            .await
            .map_err(MediaError::DbusError)?;

        let player_proxy = MediaPlayer2PlayerProxy::builder(&self.connection)
            .destination(player_id.bus_name().to_string())
            .map_err(MediaError::DbusError)?
            .build()
            .await
            .map_err(MediaError::DbusError)?;

        let info = self
            .create_player_info(&player_id, &base_proxy, &player_proxy)
            .await?;

        let metadata = self
            .get_current_metadata(&player_proxy)
            .await
            .unwrap_or_default();

        let position = self
            .get_current_position(&player_proxy)
            .await
            .unwrap_or_default();

        let playback_state = self
            .get_current_playback_state(&player_proxy)
            .await
            .unwrap_or(PlaybackState::Stopped);

        let loop_mode = self
            .get_current_loop_mode(&player_proxy)
            .await
            .unwrap_or(LoopMode::None);

        let shuffle_mode = self
            .get_current_shuffle_mode(&player_proxy)
            .await
            .unwrap_or(ShuffleMode::Off);

        let tracker = PlayerStateTracker::new(
            info.clone(),
            player_proxy,
            metadata,
            position,
            playback_state,
            loop_mode,
            shuffle_mode,
        );

        {
            let mut players = self.players.write().await;
            players.insert(player_id.clone(), tracker);
        }

        let should_set_active = {
            let active = self.active_player.read().await;
            active.is_none()
        };

        if should_set_active {
            let mut active = self.active_player.write().await;
            *active = Some(player_id.clone());
        }

        let _ = self.events_tx.send(PlayerEvent::PlayerAdded(info));
        self.broadcast_player_list().await;

        let handle = self.monitoring.start_monitoring(player_id.clone()).await;
        {
            let mut players = self.players.write().await;
            if let Some(tracker) = players.get_mut(&player_id) {
                tracker.monitoring_handle = Some(handle);
            }
        }

        info!("MPRIS player added and monitoring started successfully");
        Ok(())
    }

    /// Handle a player being removed from the bus
    #[instrument(skip(self), fields(bus_name = %player_id.bus_name()))]
    pub async fn handle_player_removed(&self, player_id: PlayerId) {
        info!("Removing MPRIS player");
        {
            let mut players = self.players.write().await;
            if let Some(mut tracker) = players.remove(&player_id) {
                if let Some(handle) = tracker.monitoring_handle.take() {
                    handle.abort();
                }
            }
        }

        {
            let mut active = self.active_player.write().await;
            if active.as_ref() == Some(&player_id) {
                let players = self.players.read().await;
                *active = players.keys().next().cloned();
            }
        }

        let _ = self.events_tx.send(PlayerEvent::PlayerRemoved(player_id));
        self.broadcast_player_list().await;
        info!("MPRIS player removed successfully");
    }

    /// Check if a player should be ignored based on its bus name
    pub async fn should_ignore_player(&self, bus_name: &str) -> bool {
        let ignored = self.ignored_players.read().await;
        ignored.iter().any(|pattern| bus_name.contains(pattern))
    }

    async fn create_player_info(
        &self,
        player_id: &PlayerId,
        base_proxy: &MediaPlayer2Proxy<'_>,
        player_proxy: &MediaPlayer2PlayerProxy<'_>,
    ) -> Result<PlayerInfo, MediaError> {
        let identity = base_proxy
            .identity()
            .await
            .unwrap_or_else(|_| player_id.bus_name().to_string());

        let can_control = player_proxy.can_control().await.unwrap_or(false);
        let can_play = player_proxy.can_play().await.unwrap_or(false);
        let can_go_next = player_proxy.can_go_next().await.unwrap_or(false);
        let can_go_previous = player_proxy.can_go_previous().await.unwrap_or(false);
        let can_seek = player_proxy.can_seek().await.unwrap_or(false);

        let can_loop = player_proxy.loop_status().await.is_ok();
        let can_shuffle = player_proxy.shuffle().await.is_ok();

        Ok(PlayerInfo {
            id: player_id.clone(),
            identity,
            can_control,
            capabilities: PlayerCapabilities {
                can_play,
                can_go_next,
                can_go_previous,
                can_seek,
                can_loop,
                can_shuffle,
            },
        })
    }

    async fn get_current_metadata(
        &self,
        proxy: &MediaPlayer2PlayerProxy<'_>,
    ) -> Result<TrackMetadata, MediaError> {
        let metadata_map = proxy.metadata().await.map_err(MediaError::DbusError)?;
        Ok(TrackMetadata::from(metadata_map))
    }

    async fn get_current_position(
        &self,
        proxy: &MediaPlayer2PlayerProxy<'_>,
    ) -> Result<Duration, MediaError> {
        let position = proxy.position().await.map_err(MediaError::DbusError)?;
        Ok(utils::from_mpris_micros(position))
    }

    async fn get_current_playback_state(
        &self,
        proxy: &MediaPlayer2PlayerProxy<'_>,
    ) -> Result<PlaybackState, MediaError> {
        let status = proxy
            .playback_status()
            .await
            .map_err(MediaError::DbusError)?;
        Ok(PlaybackState::from(status.as_str()))
    }

    async fn get_current_loop_mode(
        &self,
        proxy: &MediaPlayer2PlayerProxy<'_>,
    ) -> Result<LoopMode, MediaError> {
        let status = proxy.loop_status().await.map_err(MediaError::DbusError)?;
        Ok(LoopMode::from(status.as_str()))
    }

    async fn get_current_shuffle_mode(
        &self,
        proxy: &MediaPlayer2PlayerProxy<'_>,
    ) -> Result<ShuffleMode, MediaError> {
        let shuffle = proxy.shuffle().await.map_err(MediaError::DbusError)?;
        Ok(ShuffleMode::from(shuffle))
    }

    async fn broadcast_player_list(&self) {
        let player_ids: Vec<PlayerId> = {
            let players = self.players.read().await;
            players.keys().cloned().collect()
        };

        let _ = self.player_list_tx.send(player_ids);
    }
}

impl Clone for PlayerDiscovery {
    fn clone(&self) -> Self {
        Self {
            connection: self.connection.clone(),
            players: Arc::clone(&self.players),
            player_list_tx: self.player_list_tx.clone(),
            events_tx: self.events_tx.clone(),
            active_player: Arc::clone(&self.active_player),
            monitoring: self.monitoring.clone(),
            ignored_players: Arc::clone(&self.ignored_players),
        }
    }
}
