use tracing::instrument;

use crate::runtime_state::RuntimeState;
use crate::services::mpris::{core::Core, error::MediaError, types::PlayerId};

/// Set the active player
///
/// The active player is persisted to disk and restored on restart.
///
/// # Errors
/// Returns error if the specified player doesn't exist
#[instrument(skip(core))]
pub async fn set_active_player(core: &Core, player_id: Option<PlayerId>) -> Result<(), MediaError> {
    if let Some(ref id) = player_id {
        let players = core.players.read().await;
        if !players.contains_key(id) {
            return Err(MediaError::PlayerNotFound(id.clone()));
        }
    }

    {
        let mut active = core.active_player.write().await;
        *active = player_id.clone();
    }

    save_active_player(player_id).await;

    Ok(())
}

/// Set patterns for players to ignore during discovery
pub async fn set_ignored_players(core: &Core, patterns: Vec<String>) {
    let mut ignored = core.ignored_players.write().await;
    *ignored = patterns;
}

/// Load active player from persistent storage
pub async fn load_active_player() -> Option<PlayerId> {
    if let Ok(Some(player_bus_name)) = RuntimeState::get_active_player().await {
        Some(PlayerId::from_bus_name(&player_bus_name))
    } else {
        None
    }
}

/// Save active player to persistent storage
async fn save_active_player(player_id: Option<PlayerId>) {
    let player_bus_name = player_id.map(|p| p.bus_name().to_string());
    let _ = RuntimeState::set_active_player(player_bus_name).await;
}

/// Find a fallback player when the active player is removed
pub async fn find_fallback_player(core: &Core) -> Option<PlayerId> {
    let players = core.players.read().await;
    players.keys().next().cloned()
}
