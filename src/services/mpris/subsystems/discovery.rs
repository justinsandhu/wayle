use std::sync::Arc;

use futures::StreamExt;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, instrument, warn};
use zbus::fdo::DBusProxy;

use crate::services::mpris::{
    Volume,
    core::{Core, PlayerHandle},
    error::MediaError,
    proxy::{MediaPlayer2PlayerProxy, MediaPlayer2Proxy},
    subsystems::management,
    types::{LoopMode, PlaybackState, Player, PlayerEvent, PlayerId, ShuffleMode, TrackMetadata},
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

#[instrument(skip(core))]
async fn discover_existing_players(core: &Arc<Core>) -> Result<(), MediaError> {
    let dbus_proxy = DBusProxy::new(&core.connection)
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
        if should_ignore_player(&player_id, &Arc::clone(core)).await {
            info!("Ignoring player: {}", name);
            continue;
        }

        if let Err(e) = add_player(core, player_id).await {
            warn!("Failed to add existing player {}: {}", name, e);
        }
    }

    Ok(())
}

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
            (Some(old), None) => {
                info!("Player {} lost owner (was: {})", args.name(), old);
                handle_player_removed(&core, player_id).await;
            }
            (None, Some(new)) => {
                info!("Player {} gained owner: {}", args.name(), new);
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

async fn should_ignore_player(player_id: &PlayerId, core: &Arc<Core>) -> bool {
    let ignored = core.ignored_players.read().await;
    let bus_name = player_id.bus_name();
    ignored.iter().any(|pattern| bus_name.contains(pattern))
}

#[instrument(skip(core), fields(bus_name = %player_id.bus_name()))]
async fn add_player(core: &Arc<Core>, player_id: PlayerId) -> Result<(), MediaError> {
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

    Ok(())
}

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

        let _ = core
            .events
            .send(PlayerEvent::PlayerRemoved(player_id.clone()));

        info!("Player removed successfully");
    } else {
        warn!("Player was not in our list");
    }
}

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

    let volume = player_proxy.volume().await.unwrap_or(0.0);

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
        loop_mode,
        shuffle_mode,
        volume: Volume::from(volume),
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

#[allow(clippy::cognitive_complexity)]
async fn monitor_player_properties(
    core: Arc<Core>,
    player_id: PlayerId,
    player_proxy: MediaPlayer2PlayerProxy<'static>,
    events: broadcast::Sender<PlayerEvent>,
) {
    let mut playback_status_changes = player_proxy.receive_playback_status_changed().await;
    let mut metadata_changes = player_proxy.receive_metadata_changed().await;
    let mut loop_status_changes = player_proxy.receive_loop_status_changed().await;
    let mut shuffle_changes = player_proxy.receive_shuffle_changed().await;
    let mut volume_changes = player_proxy.receive_volume_changed().await;

    loop {
        tokio::select! {
            signal = playback_status_changes.next() => {
                match signal {
                    Some(signal) => {
                        if let Err(e) = core.refresh_player(&player_id).await {
                            error!("Failed to refresh player {}: {:?}", player_id, e);
                        }
                        handle_playback_status_change(&player_id, signal, &events).await;
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
                        if let Err(err) = core.refresh_player(&player_id).await {
                            debug!("Failed to refresh player {}: {:?}", player_id, err);
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
                        if let Err(err) = core.refresh_player(&player_id).await {
                            debug!("Failed to refresh player {}: {:?}", player_id, err);
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
                        if let Err(err) = core.refresh_player(&player_id).await {
                            debug!("Failed to refresh player {}: {:?}", player_id, err);
                        }
                    }
                    None => {
                        debug!("Shuffle stream ended for {}", player_id);
                        break;
                    }
                }
            }
            signal = volume_changes.next() => {
                match signal {
                    Some(signal) => {
                        handle_volume_change(&player_id, signal, &events).await;
                        if let Err(err) = core.refresh_player(&player_id).await {
                            debug!("Failed to refresh player {}: {:?}", player_id, err);
                        }
                    }
                    None => {
                        debug!("Volume stream ended for {}", player_id);
                        break;
                    }
                }
            }
        }
    }

    debug!("Monitoring ended for player {}", player_id);
}

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
    } else {
        error!("Failed to get playback status from signal");
    }
}

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

async fn handle_volume_change(
    player_id: &PlayerId,
    signal: PropertyChanged<'_, f64>,
    events: &broadcast::Sender<PlayerEvent>,
) {
    if let Ok(vol) = signal.get().await {
        let volume = Volume::from(vol);
        let _ = events.send(PlayerEvent::VolumeChanged {
            player_id: player_id.clone(),
            volume,
        });
    } else {
        error!("Failed to get volume from signal");
    }
}
