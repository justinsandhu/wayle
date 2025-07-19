use std::sync::Arc;
use std::time::Duration;

use async_stream::stream;
use futures::Stream;

use crate::services::mpris::{
    core::Core,
    types::{
        LoopMode, PlaybackState, PlayerEvent, PlayerId, Player, ShuffleMode, TrackMetadata,
    },
};

/// Create a stream of player list updates
pub fn players(core: &Arc<Core>) -> impl Stream<Item = Vec<Player>> + Send {
    let mut events_rx = core.events.subscribe();
    let core = Arc::clone(core);

    stream! {
        yield core.list_players().await;

        while let Ok(event) = events_rx.recv().await {
            match event {
                PlayerEvent::PlayerAdded(_) | PlayerEvent::PlayerRemoved(_) => {
                    yield core.list_players().await;
                }
                _ => {
                    // Ignore other events - they're for individual player state updates
                }
            }
        }
    }
}

/// Create a stream of updates for a specific player
pub fn player(
    core: &Arc<Core>,
    player_id: PlayerId,
) -> impl Stream<Item = Player> + Send {
    let mut events_rx = core.events.subscribe();
    let core = Arc::clone(core);

    stream! {
        if let Some(player) = core.get_player(&player_id).await {
            yield player;
        } else {
            loop {
                match events_rx.recv().await {
                    Ok(PlayerEvent::PlayerAdded(info)) if info.id == player_id => {
                        if let Some(player) = core.get_player(&player_id).await {
                            yield player;
                            break;
                        }
                    }
                    Ok(_) => continue,
                    Err(_) => return,
                }
            }
        }

        while let Ok(event) = events_rx.recv().await {
            match event {
                PlayerEvent::PlayerRemoved(id) if id == player_id => {
                    return;
                }
                PlayerEvent::PlaybackStateChanged { player_id: id, .. }
                | PlayerEvent::MetadataChanged { player_id: id, .. }
                | PlayerEvent::PositionChanged { player_id: id, .. }
                | PlayerEvent::LoopModeChanged { player_id: id, .. }
                | PlayerEvent::ShuffleModeChanged { player_id: id, .. } => {
                    if id == player_id {
                        if let Some(player) = core.get_player(&player_id).await {
                            yield player;
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

/// Create a stream of playback state changes for a player
pub fn playback_state(
    core: &Arc<Core>,
    player_id: PlayerId,
) -> impl Stream<Item = PlaybackState> + Send {
    let mut events_rx = core.events.subscribe();
    let core = Arc::clone(core);

    stream! {
        if let Some(player) = core.get_player(&player_id).await {
            yield player.playback_state;
        }

        while let Ok(event) = events_rx.recv().await {
            if let PlayerEvent::PlaybackStateChanged { player_id: id, state } = event {
                if id == player_id {
                    yield state;
                }
            }
        }
    }
}

/// Create a stream of metadata changes for a player
pub fn metadata(core: &Arc<Core>, player_id: PlayerId) -> impl Stream<Item = TrackMetadata> + Send {
    let mut events_rx = core.events.subscribe();
    let core = Arc::clone(core);

    stream! {
        if let Some(player) = core.get_player(&player_id).await {
            yield TrackMetadata {
                title: player.title,
                artist: player.artist,
                album: player.album,
                album_artist: player.album_artist,
                length: player.length,
                art_url: player.art_url,
                track_id: player.track_id,
            };
        }

        while let Ok(event) = events_rx.recv().await {
            if let PlayerEvent::MetadataChanged { player_id: id, metadata } = event {
                if id == player_id {
                    yield metadata;
                }
            }
        }
    }
}

/// Create a stream of position updates for a player
pub fn position(core: &Arc<Core>, player_id: PlayerId) -> impl Stream<Item = Duration> + Send {
    let mut events_rx = core.events.subscribe();
    let core = Arc::clone(core);

    stream! {
        if let Some(player) = core.get_player(&player_id).await {
            yield player.position;
        }

        while let Ok(event) = events_rx.recv().await {
            if let PlayerEvent::PositionChanged { player_id: id, position } = event {
                if id == player_id {
                    yield position;
                }
            }
        }
    }
}

/// Create a stream of loop mode changes for a player
pub fn loop_mode(core: &Arc<Core>, player_id: PlayerId) -> impl Stream<Item = LoopMode> + Send {
    let mut events_rx = core.events.subscribe();
    let core = Arc::clone(core);

    stream! {
        if let Some(player) = core.get_player(&player_id).await {
            yield player.loop_mode;
        }

        while let Ok(event) = events_rx.recv().await {
            if let PlayerEvent::LoopModeChanged { player_id: id, mode } = event {
                if id == player_id {
                    yield mode;
                }
            }
        }
    }
}

/// Create a stream of shuffle mode changes for a player
pub fn shuffle_mode(
    core: &Arc<Core>,
    player_id: PlayerId,
) -> impl Stream<Item = ShuffleMode> + Send {
    let mut events_rx = core.events.subscribe();
    let core = Arc::clone(core);

    stream! {
        if let Some(player) = core.get_player(&player_id).await {
            yield player.shuffle_mode;
        }

        while let Ok(event) = events_rx.recv().await {
            if let PlayerEvent::ShuffleModeChanged { player_id: id, mode } = event {
                if id == player_id {
                    yield mode;
                }
            }
        }
    }
}

/// Create a stream of active player changes
pub fn active_player(core: &Arc<Core>) -> impl Stream<Item = Option<PlayerId>> + Send {
    let mut events_rx = core.events.subscribe();
    let core = Arc::clone(core);

    stream! {
        let mut last_active = core.active_player().await;
        yield last_active.clone();

        while let Ok(event) = events_rx.recv().await {
            match event {
                PlayerEvent::PlayerAdded(_info) => {
                    let current = core.active_player().await;
                    if current != last_active {
                        last_active = current.clone();
                        yield current;
                    }
                }
                PlayerEvent::PlayerRemoved(_) => {
                    let current = core.active_player().await;
                    if current != last_active {
                        last_active = current.clone();
                        yield current;
                    }
                }
                _ => {}
            }
        }
    }
}

/// Create a stream of all player events
///
/// This is useful for applications that want to handle all events in one place
pub fn events(core: &Arc<Core>) -> impl Stream<Item = PlayerEvent> + Send {
    let mut events_rx = core.events.subscribe();

    stream! {
        while let Ok(event) = events_rx.recv().await {
            yield event;
        }
    }
}

/// Create a stream of events for a specific player
///
/// This filters the global event stream to only include events for the specified player
pub fn player_events(
    core: &Arc<Core>,
    player_id: PlayerId,
) -> impl Stream<Item = PlayerEvent> + Send {
    let mut events_rx = core.events.subscribe();

    stream! {
        while let Ok(event) = events_rx.recv().await {
            match &event {
                PlayerEvent::PlayerAdded(player) if player.id == player_id => {
                    yield event;
                }
                PlayerEvent::PlayerRemoved(id) if *id == player_id => {
                    yield event;
                }
                PlayerEvent::PlaybackStateChanged { player_id: id, .. }
                | PlayerEvent::MetadataChanged { player_id: id, .. }
                | PlayerEvent::PositionChanged { player_id: id, .. }
                | PlayerEvent::LoopModeChanged { player_id: id, .. }
                | PlayerEvent::ShuffleModeChanged { player_id: id, .. } => {
                    if *id == player_id {
                        yield event;
                    }
                }
                _ => {}
            }
        }
    }
}