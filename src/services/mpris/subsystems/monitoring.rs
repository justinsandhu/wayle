use std::sync::Arc;

use futures::StreamExt;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tracing::{debug, instrument};
use zbus::proxy::PropertyChanged;
use zbus::zvariant;

use crate::services::mpris::{
    core::Core,
    proxy::MediaPlayer2PlayerProxy,
    types::{LoopMode, PlaybackState, PlayerEvent, PlayerId, ShuffleMode, TrackMetadata},
    utils,
};

/// Start monitoring a player's properties for changes
///
/// Returns a task handle that should be stored with the player.
/// The task will run until aborted.
#[instrument(skip(proxy, events))]
pub fn monitor_player(
    player_id: PlayerId,
    proxy: MediaPlayer2PlayerProxy<'static>,
    events: broadcast::Sender<PlayerEvent>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        monitor_player_properties(player_id, proxy, events).await;
    })
}

/// Main monitoring loop for a player
#[allow(clippy::cognitive_complexity)]
async fn monitor_player_properties(
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
                    Some(signal) => handle_position_change(&player_id, signal, &events).await,
                    None => {
                        debug!("Position stream ended for {}", player_id);
                        break;
                    }
                }
            }
            signal = playback_status_changes.next() => {
                match signal {
                    Some(signal) => handle_playback_status_change(&player_id, signal, &events).await,
                    None => {
                        debug!("Playback status stream ended for {}", player_id);
                        break;
                    }
                }
            }
            signal = metadata_changes.next() => {
                match signal {
                    Some(signal) => handle_metadata_change(&player_id, signal, &events).await,
                    None => {
                        debug!("Metadata stream ended for {}", player_id);
                        break;
                    }
                }
            }
            signal = loop_status_changes.next() => {
                match signal {
                    Some(signal) => handle_loop_status_change(&player_id, signal, &events).await,
                    None => {
                        debug!("Loop status stream ended for {}", player_id);
                        break;
                    }
                }
            }
            signal = shuffle_changes.next() => {
                match signal {
                    Some(signal) => handle_shuffle_change(&player_id, signal, &events).await,
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

/// Update player state in core when monitoring detects changes
///
/// This is called by the service to sync core state with property changes.
pub async fn update_player_state(core: &Arc<Core>, event: &PlayerEvent) {
    let mut players = core.players.write().await;

    match event {
        PlayerEvent::PlaybackStateChanged { player_id, state } => {
            if let Some(handle) = players.get_mut(player_id) {
                handle.state.playback_state = *state;
            }
        }
        PlayerEvent::MetadataChanged {
            player_id,
            metadata,
        } => {
            if let Some(handle) = players.get_mut(player_id) {
                handle.state.metadata = metadata.clone();
            }
        }
        PlayerEvent::PositionChanged {
            player_id,
            position,
        } => {
            if let Some(handle) = players.get_mut(player_id) {
                handle.state.position = *position;
            }
        }
        PlayerEvent::LoopModeChanged { player_id, mode } => {
            if let Some(handle) = players.get_mut(player_id) {
                handle.state.loop_mode = *mode;
            }
        }
        PlayerEvent::ShuffleModeChanged { player_id, mode } => {
            if let Some(handle) = players.get_mut(player_id) {
                handle.state.shuffle_mode = *mode;
            }
        }
        _ => {}
    }
}
