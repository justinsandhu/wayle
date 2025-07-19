use std::sync::Arc;

use futures::StreamExt;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tracing::{debug, info, instrument, warn};
use zbus::fdo::DBusProxy;

use crate::services::mpris::{
    core::{Core, PlayerHandle},
    error::MediaError,
    proxy::{MediaPlayer2PlayerProxy, MediaPlayer2Proxy},
    subsystems::management,
    types::{LoopMode, PlaybackState, Player, PlayerEvent, PlayerId, ShuffleMode, TrackMetadata},
    utils,
};
use zbus::proxy::PropertyChanged;
use zbus::zvariant;

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

        discover_existing_players(&Arc::clone(&core)).await?;

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
async fn discover_existing_players(core: &Arc<Core>) -> Result<(), MediaError> {
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
        if should_ignore_player(&player_id, &Arc::clone(core)).await {
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
async fn should_ignore_player(player_id: &PlayerId, core: &Arc<Core>) -> bool {
    let ignored = core.ignored_players.read().await;
    let bus_name = player_id.bus_name();
    ignored.iter().any(|pattern| bus_name.contains(pattern))
}

/// Add a new player to the service
#[instrument(skip(core), fields(bus_name = %player_id.bus_name()))]
async fn add_player(core: &Arc<Core>, player_id: PlayerId) -> Result<(), MediaError> {
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

    let player = create_player(&player_id, &base_proxy, &player_proxy).await?;

    let monitor_handle = monitor_player(
        Arc::clone(core),
        player_id.clone(),
        player_proxy.clone(),
        core.events.clone(),
    );

    let handle = PlayerHandle::new(player.clone(), player_proxy, monitor_handle);

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

    let _ = core.events.send(PlayerEvent::PlayerAdded(player));

    info!("Player added successfully");
    Ok(())
}

/// Remove a player from the service
#[instrument(skip(core), fields(bus_name = %player_id.bus_name()))]
async fn handle_player_removed(core: &Arc<Core>, player_id: PlayerId) {
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

/// Create a complete player from D-Bus proxies
async fn create_player(
    player_id: &PlayerId,
    base_proxy: &MediaPlayer2Proxy<'_>,
    player_proxy: &MediaPlayer2PlayerProxy<'_>,
) -> Result<Player, MediaError> {
    let identity = base_proxy
        .identity()
        .await
        .unwrap_or_else(|_| player_id.bus_name().to_string());

    let desktop_entry = base_proxy.desktop_entry().await.ok();

    let playback_state = player_proxy
        .playback_status()
        .await
        .map(|s| PlaybackState::from(s.as_str()))
        .unwrap_or(PlaybackState::Stopped);

    let metadata_map = player_proxy.metadata().await.unwrap_or_default();

    let metadata = TrackMetadata::from(metadata_map);

    let position = player_proxy
        .position()
        .await
        .map(utils::from_mpris_micros)
        .unwrap_or_default();

    let loop_mode = player_proxy
        .loop_status()
        .await
        .map(|s| LoopMode::from(s.as_str()))
        .unwrap_or(LoopMode::None);

    let shuffle_mode = player_proxy
        .shuffle()
        .await
        .map(ShuffleMode::from)
        .unwrap_or(ShuffleMode::Off);

    let can_control = player_proxy.can_control().await.unwrap_or(false);
    let can_play = player_proxy.can_play().await.unwrap_or(false);
    let can_go_next = player_proxy.can_go_next().await.unwrap_or(false);
    let can_go_previous = player_proxy.can_go_previous().await.unwrap_or(false);
    let can_seek = player_proxy.can_seek().await.unwrap_or(false);
    let can_loop = player_proxy.loop_status().await.is_ok();
    let can_shuffle = player_proxy.shuffle().await.is_ok();

    Ok(Player {
        id: player_id.clone(),
        identity,
        desktop_entry,
        playback_state,
        position,
        loop_mode,
        shuffle_mode,
        title: metadata.title,
        artist: metadata.artist,
        album: metadata.album,
        album_artist: metadata.album_artist,
        length: metadata.length,
        art_url: metadata.art_url,
        track_id: metadata.track_id,
        can_control,
        can_play,
        can_go_next,
        can_go_previous,
        can_seek,
        can_loop,
        can_shuffle,
    })
}

/// Start monitoring a player's properties for changes
///
/// Returns a task handle that should be stored with the player.
/// The task will run until aborted.
#[instrument(skip(core, proxy, events))]
fn monitor_player(
    core: Arc<Core>,
    player_id: PlayerId,
    proxy: MediaPlayer2PlayerProxy<'static>,
    events: broadcast::Sender<PlayerEvent>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        monitor_player_properties(core, player_id, proxy, events).await;
    })
}

/// Main monitoring loop for a player
#[allow(clippy::cognitive_complexity)]
async fn monitor_player_properties(
    core: Arc<Core>,
    player_id: PlayerId,
    player_proxy: MediaPlayer2PlayerProxy<'static>,
    events: broadcast::Sender<PlayerEvent>,
) {
    let mut position_changes = player_proxy.receive_position_changed().await;
    let mut playback_status_changes = player_proxy.receive_playback_status_changed().await;
    let mut metadata_changes = player_proxy.receive_metadata_changed().await;
    let mut loop_status_changes = player_proxy.receive_loop_status_changed().await;
    let mut shuffle_changes = player_proxy.receive_shuffle_changed().await;

    loop {
        tokio::select! {
            signal = position_changes.next() => {
                match signal {
                    Some(signal) => {
                        handle_position_change(&player_id, signal, &events).await;
                        if let Err(e) = core.refresh_player(&player_id).await {
                            debug!("Failed to refresh player {}: {:?}", player_id, e);
                        }
                    }
                    None => {
                        debug!("Position stream ended for {}", player_id);
                        break;
                    }
                }
            }
            signal = playback_status_changes.next() => {
                match signal {
                    Some(signal) => {
                        handle_playback_status_change(&player_id, signal, &events).await;
                        if let Err(e) = core.refresh_player(&player_id).await {
                            debug!("Failed to refresh player {}: {:?}", player_id, e);
                        }
                    }
                    None => {
                        debug!("Playback status stream ended for {}", player_id);
                        break;
                    }
                }
            }
            signal = metadata_changes.next() => {
                match signal {
                    Some(signal) => {
                        handle_metadata_change(&player_id, signal, &events).await;
                        if let Err(e) = core.refresh_player(&player_id).await {
                            debug!("Failed to refresh player {}: {:?}", player_id, e);
                        }
                    }
                    None => {
                        debug!("Metadata stream ended for {}", player_id);
                        break;
                    }
                }
            }
            signal = loop_status_changes.next() => {
                match signal {
                    Some(signal) => {
                        handle_loop_status_change(&player_id, signal, &events).await;
                        if let Err(e) = core.refresh_player(&player_id).await {
                            debug!("Failed to refresh player {}: {:?}", player_id, e);
                        }
                    }
                    None => {
                        debug!("Loop status stream ended for {}", player_id);
                        break;
                    }
                }
            }
            signal = shuffle_changes.next() => {
                match signal {
                    Some(signal) => {
                        handle_shuffle_change(&player_id, signal, &events).await;
                        if let Err(e) = core.refresh_player(&player_id).await {
                            debug!("Failed to refresh player {}: {:?}", player_id, e);
                        }
                    }
                    None => {
                        debug!("Shuffle stream ended for {}", player_id);
                        break;
                    }
                }
            }
        }
    }

    debug!("Monitoring ended for player {}", player_id);
}

/// Handle position change events
async fn handle_position_change(
    player_id: &PlayerId,
    signal: PropertyChanged<'_, i64>,
    events: &broadcast::Sender<PlayerEvent>,
) {
    if let Ok(position_micros) = signal.get().await {
        let position = utils::from_mpris_micros(position_micros);
        let _ = events.send(PlayerEvent::PositionChanged {
            player_id: player_id.clone(),
            position,
        });
    }
}

/// Handle playback status change events
async fn handle_playback_status_change(
    player_id: &PlayerId,
    signal: PropertyChanged<'_, String>,
    events: &broadcast::Sender<PlayerEvent>,
) {
    if let Ok(status) = signal.get().await {
        let state = PlaybackState::from(status.as_str());
        let _ = events.send(PlayerEvent::PlaybackStateChanged {
            player_id: player_id.clone(),
            state,
        });
    }
}

/// Handle metadata change events
async fn handle_metadata_change(
    player_id: &PlayerId,
    signal: PropertyChanged<'_, std::collections::HashMap<String, zvariant::OwnedValue>>,
    events: &broadcast::Sender<PlayerEvent>,
) {
    if let Ok(metadata_map) = signal.get().await {
        let metadata = TrackMetadata::from(metadata_map);
        let _ = events.send(PlayerEvent::MetadataChanged {
            player_id: player_id.clone(),
            metadata,
        });
    }
}

/// Handle loop status change events
async fn handle_loop_status_change(
    player_id: &PlayerId,
    signal: PropertyChanged<'_, String>,
    events: &broadcast::Sender<PlayerEvent>,
) {
    if let Ok(status) = signal.get().await {
        let mode = LoopMode::from(status.as_str());
        let _ = events.send(PlayerEvent::LoopModeChanged {
            player_id: player_id.clone(),
            mode,
        });
    }
}

/// Handle shuffle change events
async fn handle_shuffle_change(
    player_id: &PlayerId,
    signal: PropertyChanged<'_, bool>,
    events: &broadcast::Sender<PlayerEvent>,
) {
    if let Ok(shuffle) = signal.get().await {
        let mode = ShuffleMode::from(shuffle);
        let _ = events.send(PlayerEvent::ShuffleModeChanged {
            player_id: player_id.clone(),
            mode,
        });
    }
}
