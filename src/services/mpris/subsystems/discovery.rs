use std::sync::Arc;

use futures::StreamExt;
use tokio::task::JoinHandle;
use tracing::{info, instrument, warn};
use zbus::fdo::DBusProxy;

use crate::services::mpris::{
    core::{Core, PlayerHandle},
    error::MediaError,
    proxy::{MediaPlayer2PlayerProxy, MediaPlayer2Proxy},
    subsystems::{management, monitoring},
    types::{
        LoopMode, PlaybackState, PlayerCapabilities, PlayerEvent, PlayerId, PlayerInfo,
        PlayerState, ShuffleMode, TrackMetadata,
    },
    utils,
};

/// Player discovery subsystem
///
/// Handles discovery of new MPRIS players on D-Bus and manages their lifecycle.
/// This struct owns the discovery task handle.
pub struct Discovery {
    /// Handle to the discovery monitoring task
    handle: JoinHandle<()>,
}

impl Discovery {
    /// Start the discovery subsystem
    ///
    /// This will:
    /// 1. Discover existing players on the bus
    /// 2. Monitor for new players being added/removed
    ///
    /// # Errors
    /// Returns error if D-Bus proxy creation fails
    #[instrument(skip(core))]
    pub async fn start(core: Arc<Core>) -> Result<Self, MediaError> {
        info!("Starting MPRIS player discovery");

        discover_existing_players(&core).await?;

        let handle = tokio::spawn(monitor_player_changes(Arc::clone(&core)));

        Ok(Self { handle })
    }

}

impl Drop for Discovery {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

/// Discover players already on the bus
#[instrument(skip(core))]
async fn discover_existing_players(core: &Core) -> Result<(), MediaError> {
    info!("Discovering existing MPRIS players");

    let dbus_proxy = DBusProxy::new(&core.connection)
        .await
        .map_err(|e| MediaError::InitializationFailed(format!("DBus proxy failed: {e}")))?;

    let names = dbus_proxy
        .list_names()
        .await
        .map_err(|e| MediaError::DbusError(e.into()))?;

    let mpris_count = names
        .iter()
        .filter(|n| n.starts_with("org.mpris.MediaPlayer2."))
        .count();
    info!("Found {} MPRIS players on D-Bus", mpris_count);

    for name in names {
        if !name.starts_with("org.mpris.MediaPlayer2.") {
            continue;
        }

        let player_id = PlayerId::from_bus_name(&name);
        if should_ignore_player(&player_id, core).await {
            info!("Ignoring player: {}", name);
            continue;
        }

        if let Err(e) = add_player(core, player_id).await {
            warn!("Failed to add existing player {}: {}", name, e);
        }
    }

    info!("Finished discovering existing players");
    Ok(())
}

/// Monitor for player additions/removals
async fn monitor_player_changes(core: Arc<Core>) {
    let Ok(dbus_proxy) = DBusProxy::new(&core.connection).await else {
        warn!("Failed to create DBus proxy for monitoring");
        return;
    };

    let Ok(mut name_owner_changed) = dbus_proxy.receive_name_owner_changed().await else {
        warn!("Failed to subscribe to name owner changes");
        return;
    };

    while let Some(signal) = name_owner_changed.next().await {
        let Ok(args) = signal.args() else { continue };

        if !args.name().starts_with("org.mpris.MediaPlayer2.") {
            continue;
        }

        let player_id = PlayerId::from_bus_name(args.name());

        match (args.old_owner().as_deref(), args.new_owner().as_deref()) {
            (Some(_), None) => {
                handle_player_removed(&core, player_id).await;
            }
            (None, Some(_)) => {
                if !should_ignore_player(&player_id, &core).await {
                    if let Err(e) = add_player(&core, player_id).await {
                        warn!("Failed to add new player: {e}");
                    }
                }
            }
            _ => {}
        }
    }
}

/// Check if a player should be ignored
async fn should_ignore_player(player_id: &PlayerId, core: &Core) -> bool {
    let ignored = core.ignored_players.read().await;
    let bus_name = player_id.bus_name();
    ignored.iter().any(|pattern| bus_name.contains(pattern))
}

/// Add a new player to the service
#[instrument(skip(core), fields(bus_name = %player_id.bus_name()))]
async fn add_player(core: &Core, player_id: PlayerId) -> Result<(), MediaError> {
    info!("Adding new MPRIS player");

    let base_proxy = MediaPlayer2Proxy::builder(&core.connection)
        .destination(player_id.bus_name().to_string())
        .map_err(MediaError::DbusError)?
        .build()
        .await
        .map_err(MediaError::DbusError)?;

    let player_proxy = MediaPlayer2PlayerProxy::builder(&core.connection)
        .destination(player_id.bus_name().to_string())
        .map_err(MediaError::DbusError)?
        .build()
        .await
        .map_err(MediaError::DbusError)?;

    let info = create_player_info(&player_id, &base_proxy, &player_proxy).await?;
    let state = create_initial_state(info.clone(), &player_proxy).await?;

    let monitor_handle =
        monitoring::monitor_player(player_id.clone(), player_proxy.clone(), core.events.clone());

    let handle = PlayerHandle::new(info.clone(), state, player_proxy, monitor_handle);

    {
        let mut players = core.players.write().await;
        players.insert(player_id.clone(), handle);
    }

    {
        let mut active = core.active_player.write().await;
        if active.is_none() {
            *active = Some(player_id);
        }
    }

    let _ = core.events.send(PlayerEvent::PlayerAdded(info));

    info!("Player added successfully");
    Ok(())
}

/// Remove a player from the service
#[instrument(skip(core), fields(bus_name = %player_id.bus_name()))]
async fn handle_player_removed(core: &Core, player_id: PlayerId) {
    info!("Removing MPRIS player");

    let removed = {
        let mut players = core.players.write().await;
        players.remove(&player_id).is_some()
    };

    if removed {
        {
            let mut active = core.active_player.write().await;
            if active.as_ref() == Some(&player_id) {
                *active = management::find_fallback_player(core).await;
            }
        }

        let _ = core.events.send(PlayerEvent::PlayerRemoved(player_id));

        info!("Player removed successfully");
    }
}

/// Create player info from D-Bus proxies
async fn create_player_info(
    player_id: &PlayerId,
    base_proxy: &MediaPlayer2Proxy<'_>,
    player_proxy: &MediaPlayer2PlayerProxy<'_>,
) -> Result<PlayerInfo, MediaError> {
    let identity = base_proxy
        .identity()
        .await
        .unwrap_or_else(|_| player_id.bus_name().to_string());

    let can_control = player_proxy.can_control().await.unwrap_or(false);

    let capabilities = PlayerCapabilities {
        can_play: player_proxy.can_play().await.unwrap_or(false),
        can_go_next: player_proxy.can_go_next().await.unwrap_or(false),
        can_go_previous: player_proxy.can_go_previous().await.unwrap_or(false),
        can_seek: player_proxy.can_seek().await.unwrap_or(false),
        can_loop: player_proxy.loop_status().await.is_ok(),
        can_shuffle: player_proxy.shuffle().await.is_ok(),
    };

    Ok(PlayerInfo {
        id: player_id.clone(),
        identity,
        can_control,
        capabilities,
    })
}

/// Create initial player state
async fn create_initial_state(
    info: PlayerInfo,
    proxy: &MediaPlayer2PlayerProxy<'_>,
) -> Result<PlayerState, MediaError> {
    let playback_state = proxy
        .playback_status()
        .await
        .map(|s| PlaybackState::from(s.as_str()))
        .unwrap_or(PlaybackState::Stopped);

    let metadata = proxy
        .metadata()
        .await
        .map(TrackMetadata::from)
        .unwrap_or_default();

    let position = proxy
        .position()
        .await
        .map(utils::from_mpris_micros)
        .unwrap_or_default();

    let loop_mode = proxy
        .loop_status()
        .await
        .map(|s| LoopMode::from(s.as_str()))
        .unwrap_or(LoopMode::None);

    let shuffle_mode = proxy
        .shuffle()
        .await
        .map(ShuffleMode::from)
        .unwrap_or(ShuffleMode::Off);

    Ok(PlayerState {
        player_info: info,
        playback_state,
        metadata,
        position,
        loop_mode,
        shuffle_mode,
    })
}
