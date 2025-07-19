use std::time::Duration;

use tracing::instrument;

use crate::services::mpris::{
    core::Core,
    error::MediaError,
    types::{LoopMode, PlayerId, ShuffleMode},
    utils,
};
use zbus::zvariant::ObjectPath;

/// Toggle play/pause state for a player
///
/// # Errors
/// Returns error if player not found or D-Bus operation fails
#[instrument(skip(core))]
pub async fn play_pause(core: &Core, player_id: PlayerId) -> Result<(), MediaError> {
    let players = core.players.read().await;
    let handle = players
        .get(&player_id)
        .ok_or_else(|| MediaError::PlayerNotFound(player_id.clone()))?;

    handle
        .proxy
        .play_pause()
        .await
        .map_err(MediaError::DbusError)
}

/// Start playback for a player
///
/// # Errors
/// Returns error if player not found or D-Bus operation fails
#[instrument(skip(core))]
pub async fn play(core: &Core, player_id: PlayerId) -> Result<(), MediaError> {
    let players = core.players.read().await;
    let handle = players
        .get(&player_id)
        .ok_or_else(|| MediaError::PlayerNotFound(player_id.clone()))?;

    handle.proxy.play().await.map_err(MediaError::DbusError)
}

/// Pause playback for a player
///
/// # Errors
/// Returns error if player not found or D-Bus operation fails
#[instrument(skip(core))]
pub async fn pause(core: &Core, player_id: PlayerId) -> Result<(), MediaError> {
    let players = core.players.read().await;
    let handle = players
        .get(&player_id)
        .ok_or_else(|| MediaError::PlayerNotFound(player_id.clone()))?;

    handle.proxy.pause().await.map_err(MediaError::DbusError)
}

/// Stop playback for a player
///
/// # Errors
/// Returns error if player not found or D-Bus operation fails
#[instrument(skip(core))]
pub async fn stop(core: &Core, player_id: PlayerId) -> Result<(), MediaError> {
    let players = core.players.read().await;
    let handle = players
        .get(&player_id)
        .ok_or_else(|| MediaError::PlayerNotFound(player_id.clone()))?;

    handle.proxy.stop().await.map_err(MediaError::DbusError)
}

/// Skip to next track
///
/// # Errors
/// Returns error if player not found, doesn't support next, or D-Bus operation fails
#[instrument(skip(core))]
pub async fn next(core: &Core, player_id: PlayerId) -> Result<(), MediaError> {
    let players = core.players.read().await;
    let handle = players
        .get(&player_id)
        .ok_or_else(|| MediaError::PlayerNotFound(player_id.clone()))?;

    if !handle.info.capabilities.can_go_next {
        return Err(MediaError::OperationNotSupported("Next track".to_string()));
    }

    handle.proxy.next().await.map_err(MediaError::DbusError)
}

/// Go to previous track
///
/// # Errors
/// Returns error if player not found, doesn't support previous, or D-Bus operation fails
#[instrument(skip(core))]
pub async fn previous(core: &Core, player_id: PlayerId) -> Result<(), MediaError> {
    let players = core.players.read().await;
    let handle = players
        .get(&player_id)
        .ok_or_else(|| MediaError::PlayerNotFound(player_id.clone()))?;

    if !handle.info.capabilities.can_go_previous {
        return Err(MediaError::OperationNotSupported(
            "Previous track".to_string(),
        ));
    }

    handle.proxy.previous().await.map_err(MediaError::DbusError)
}

/// Seek to a specific position in the current track
///
/// # Errors
/// Returns error if player not found, doesn't support seeking, or D-Bus operation fails
#[instrument(skip(core))]
pub async fn seek(core: &Core, player_id: PlayerId, position: Duration) -> Result<(), MediaError> {
    let players = core.players.read().await;
    let handle = players
        .get(&player_id)
        .ok_or_else(|| MediaError::PlayerNotFound(player_id.clone()))?;

    if !handle.info.capabilities.can_seek {
        return Err(MediaError::OperationNotSupported("Seek".to_string()));
    }

    let position_micros = utils::to_mpris_micros(position);
    let track_id = handle.state.metadata.track_id.as_deref().unwrap_or("/");
    let track_path = ObjectPath::try_from(track_id)
        .map_err(|e| MediaError::OperationNotSupported(format!("Invalid track ID: {e}")))?;

    handle
        .proxy
        .set_position(&track_path, position_micros)
        .await
        .map_err(MediaError::DbusError)
}

/// Toggle loop mode for a player
///
/// Cycles through: None → Track → Playlist → None
///
/// # Errors
/// Returns error if player not found, doesn't support loop modes, or D-Bus operation fails
#[instrument(skip(core))]
pub async fn toggle_loop(core: &Core, player_id: PlayerId) -> Result<(), MediaError> {
    let players = core.players.read().await;
    let handle = players
        .get(&player_id)
        .ok_or_else(|| MediaError::PlayerNotFound(player_id.clone()))?;

    if !handle.info.capabilities.can_loop {
        return Err(MediaError::OperationNotSupported("Loop mode".to_string()));
    }

    let current_mode = handle.state.loop_mode;
    let next_mode = match current_mode {
        LoopMode::None => LoopMode::Track,
        LoopMode::Track => LoopMode::Playlist,
        LoopMode::Playlist | LoopMode::Unsupported => LoopMode::None,
    };

    set_loop_mode(core, player_id, next_mode).await
}

/// Set loop mode for a player
///
/// # Errors
/// Returns error if player not found, doesn't support loop modes, or D-Bus operation fails
#[instrument(skip(core))]
pub async fn set_loop_mode(
    core: &Core,
    player_id: PlayerId,
    mode: LoopMode,
) -> Result<(), MediaError> {
    let players = core.players.read().await;
    let handle = players
        .get(&player_id)
        .ok_or_else(|| MediaError::PlayerNotFound(player_id.clone()))?;

    if !handle.info.capabilities.can_loop {
        return Err(MediaError::OperationNotSupported("Loop mode".to_string()));
    }

    let mode_str = match mode {
        LoopMode::None => "None",
        LoopMode::Track => "Track",
        LoopMode::Playlist => "Playlist",
        LoopMode::Unsupported => return Ok(()),
    };

    handle
        .proxy
        .set_loop_status(mode_str)
        .await
        .map_err(MediaError::DbusError)
}

/// Toggle shuffle mode for a player
///
/// # Errors
/// Returns error if player not found, doesn't support shuffle, or D-Bus operation fails
#[instrument(skip(core))]
pub async fn toggle_shuffle(core: &Core, player_id: PlayerId) -> Result<(), MediaError> {
    let players = core.players.read().await;
    let handle = players
        .get(&player_id)
        .ok_or_else(|| MediaError::PlayerNotFound(player_id.clone()))?;

    if !handle.info.capabilities.can_shuffle {
        return Err(MediaError::OperationNotSupported(
            "Shuffle mode".to_string(),
        ));
    }

    let current_mode = handle.state.shuffle_mode;
    let next_mode = match current_mode {
        ShuffleMode::Off => ShuffleMode::On,
        ShuffleMode::On | ShuffleMode::Unsupported => ShuffleMode::Off,
    };

    set_shuffle_mode(core, player_id, next_mode).await
}

/// Set shuffle mode for a player
///
/// # Errors
/// Returns error if player not found, doesn't support shuffle, or D-Bus operation fails
#[instrument(skip(core))]
pub async fn set_shuffle_mode(
    core: &Core,
    player_id: PlayerId,
    mode: ShuffleMode,
) -> Result<(), MediaError> {
    let players = core.players.read().await;
    let handle = players
        .get(&player_id)
        .ok_or_else(|| MediaError::PlayerNotFound(player_id.clone()))?;

    if !handle.info.capabilities.can_shuffle {
        return Err(MediaError::OperationNotSupported(
            "Shuffle mode".to_string(),
        ));
    }

    let shuffle_value = match mode {
        ShuffleMode::On => true,
        ShuffleMode::Off => false,
        ShuffleMode::Unsupported => return Ok(()),
    };

    handle
        .proxy
        .set_shuffle(shuffle_value)
        .await
        .map_err(MediaError::DbusError)
}
