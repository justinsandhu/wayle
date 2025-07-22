use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use zbus::Connection;
use zbus::names::OwnedBusName;

use crate::services::common::Property;

use super::handle::PlayerHandle;
use super::metadata::TrackMetadata;
use super::model::Player;
use super::monitoring::PlayerMonitor;
use crate::services::mpris::proxy::{MediaPlayer2PlayerProxy, MediaPlayer2Proxy};
use crate::services::mpris::{LoopMode, MediaError, PlaybackState, PlayerId, ShuffleMode, Volume};

/// Manages player lifecycle operations.
///
/// Handles adding, removing, and refreshing player state.
pub(crate) struct PlayerManager;

impl PlayerManager {
    /// Add a new player to the service.
    ///
    /// Creates player model, sets up monitoring, and initializes properties.
    ///
    /// # Errors
    ///
    /// Returns error if D-Bus proxy creation fails or player initialization fails
    pub(crate) async fn add_player(
        connection: &Connection,
        players: &Arc<RwLock<HashMap<PlayerId, PlayerHandle>>>,
        player_list: &Property<Vec<PlayerId>>,
        active_player: &Property<Option<PlayerId>>,
        player_id: PlayerId,
    ) -> Result<(), MediaError> {
        let bus_name = OwnedBusName::try_from(player_id.bus_name())
            .map_err(|e| MediaError::InitializationFailed(format!("Invalid bus name: {e}")))?;

        let base_proxy = MediaPlayer2Proxy::builder(connection)
            .destination(bus_name.clone())
            .map_err(MediaError::DbusError)?
            .build()
            .await
            .map_err(MediaError::DbusError)?;

        let player_proxy = MediaPlayer2PlayerProxy::builder(connection)
            .destination(bus_name)
            .map_err(MediaError::DbusError)?
            .build()
            .await
            .map_err(MediaError::DbusError)?;

        let identity = base_proxy
            .identity()
            .await
            .unwrap_or_else(|_| player_id.bus_name().to_string());
        let desktop_entry = base_proxy.desktop_entry().await.ok();

        let player = Arc::new(Player::new(player_id.clone(), identity));
        player.desktop_entry.set(desktop_entry);

        Self::refresh_player_properties(&player, &player_proxy).await;

        let monitor =
            PlayerMonitor::start(player_id.clone(), Arc::clone(&player), player_proxy.clone());

        let handle = PlayerHandle {
            player: Arc::clone(&player),
            proxy: player_proxy,
            _monitor: monitor,
        };

        let mut players_map = players.write().await;
        players_map.insert(player_id.clone(), handle);

        if active_player.get().is_none() {
            active_player.set(Some(player_id.clone()));
        }

        let mut current_list = player_list.get();
        current_list.push(player_id);
        player_list.set(current_list);

        Ok(())
    }

    /// Remove a player from the service.
    ///
    /// Also updates active player if the removed player was active.
    pub(crate) async fn remove_player(
        players: &Arc<RwLock<HashMap<PlayerId, PlayerHandle>>>,
        player_list: &Property<Vec<PlayerId>>,
        active_player: &Property<Option<PlayerId>>,
        player_id: PlayerId,
    ) {
        let mut players_map = players.write().await;
        players_map.remove(&player_id);

        if active_player.get().as_ref() == Some(&player_id) {
            let new_active = players_map.keys().next().cloned();
            active_player.set(new_active);
        }

        let current_list = player_list.get();
        let updated_list: Vec<PlayerId> = current_list
            .into_iter()
            .filter(|id| id != &player_id)
            .collect();
        player_list.set(updated_list);
    }

    /// Refresh all player properties from D-Bus.
    ///
    /// Updates playback state, metadata, capabilities, etc.
    pub async fn refresh_player_properties(player: &Player, proxy: &MediaPlayer2PlayerProxy<'_>) {
        if let Ok(status) = proxy.playback_status().await {
            player
                .playback_state
                .set(PlaybackState::from(status.as_str()));
        }

        if let Ok(metadata_map) = proxy.metadata().await {
            let metadata = TrackMetadata::from(metadata_map);
            player.update_metadata(metadata);
        }

        if let Ok(loop_status) = proxy.loop_status().await {
            player.loop_mode.set(LoopMode::from(loop_status.as_str()));
        }

        if let Ok(shuffle) = proxy.shuffle().await {
            player.shuffle_mode.set(ShuffleMode::from(shuffle));
        }

        if let Ok(volume) = proxy.volume().await {
            player.volume.set(Volume::from(volume));
        }

        let can_control = proxy.can_control().await.unwrap_or(false);
        let can_play = proxy.can_play().await.unwrap_or(false);
        let can_go_next = proxy.can_go_next().await.unwrap_or(false);
        let can_go_previous = proxy.can_go_previous().await.unwrap_or(false);
        let can_seek = proxy.can_seek().await.unwrap_or(false);
        let can_loop = proxy.loop_status().await.is_ok();
        let can_shuffle = proxy.shuffle().await.is_ok();

        player.update_capabilities(
            can_control,
            can_play,
            can_go_next,
            can_go_previous,
            can_seek,
            can_loop,
            can_shuffle,
        );
    }
}
