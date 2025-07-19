use crate::services::mpris::{
    core::Core,
    types::{PlayerId, PlayerInfo, PlayerState},
};

/// Get a list of all current players
pub async fn list_players(core: &Core) -> Vec<PlayerState> {
    let players = core.players.read().await;
    players
        .values()
        .map(|handle| handle.state.clone())
        .collect()
}

/// Get information about a specific player
pub async fn player_info(core: &Core, player_id: &PlayerId) -> Option<PlayerInfo> {
    let players = core.players.read().await;
    players.get(player_id).map(|handle| handle.info.clone())
}

/// Get the current state of a specific player
pub async fn player_state(core: &Core, player_id: &PlayerId) -> Option<PlayerState> {
    let players = core.players.read().await;
    players.get(player_id).map(|handle| handle.state.clone())
}

/// Get the currently active player
pub async fn active_player(core: &Core) -> Option<PlayerId> {
    let active = core.active_player.read().await;

    if let Some(ref player_id) = *active {
        let players = core.players.read().await;
        if players.contains_key(player_id) {
            return active.clone();
        }
    }

    None
}

/// Get the list of ignored player patterns
pub async fn ignored_patterns(core: &Core) -> Vec<String> {
    let ignored = core.ignored_players.read().await;
    ignored.clone()
}
