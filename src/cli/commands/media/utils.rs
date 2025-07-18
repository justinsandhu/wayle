use futures::StreamExt;
use tokio::pin;

use crate::{
    cli::CliError,
    services::mpris::{MediaService, PlayerId},
};

/// Finds a player by identifier (index or partial name match)
///
/// Supports:
/// - Numeric indices (1-based): "1", "2", etc.
/// - Partial name matching: "spotify", "fire" (matches Firefox), etc.
/// - Case-insensitive matching
///
/// # Arguments
///
/// * `service` - The media service instance
/// * `identifier` - The player identifier (index or partial name)
///
/// # Errors
///
/// Returns CliError if no matching player is found or multiple matches exist
pub async fn find_player_by_identifier(
    service: &MediaService,
    identifier: &str,
) -> Result<PlayerId, CliError> {
    let players_stream = service.players();
    pin!(players_stream);
    let players = players_stream
        .next()
        .await
        .ok_or_else(|| CliError::ServiceError {
            service: "Media".to_string(),
            details: "Failed to get player list".to_string(),
        })?;

    if players.is_empty() {
        return Err(CliError::InvalidArgument {
            arg: "player-id".to_string(),
            reason: "No media players found".to_string(),
        });
    }

    if let Ok(index) = identifier.parse::<usize>() {
        if index > 0 && index <= players.len() {
            return Ok(players[index - 1].player_info.id.clone());
        } else {
            return Err(CliError::InvalidArgument {
                arg: "player-id".to_string(),
                reason: format!("Invalid player index. Valid range: 1-{}", players.len()),
            });
        }
    }

    let identifier_lower = identifier.to_lowercase();
    let mut matches = Vec::new();

    for player_state in &players {
        let info = &player_state.player_info;
        let identity_lower = info.identity.to_lowercase();
        let bus_name_lower = info.id.bus_name().to_lowercase();

        if identity_lower.contains(&identifier_lower) || bus_name_lower.contains(&identifier_lower)
        {
            matches.push((info.id.clone(), info.identity.clone()));
        }
    }

    match matches.len() {
        0 => Err(CliError::InvalidArgument {
            arg: "player-id".to_string(),
            reason: format!("No player found matching '{identifier}'"),
        }),
        1 => Ok(matches[0].0.clone()),
        _ => {
            let names: Vec<String> = matches.iter().map(|(_, name)| name.clone()).collect();
            Err(CliError::InvalidArgument {
                arg: "player-id".to_string(),
                reason: format!(
                    "Multiple players match '{}': {}. Please be more specific.",
                    identifier,
                    names.join(", ")
                ),
            })
        }
    }
}

/// Gets a player ID from optional identifier or returns active player from runtime state
///
/// If identifier is provided, finds the player by that identifier and saves it as active.
/// If no identifier is provided, uses the active player from persistent runtime state.
///
/// # Arguments
///
/// * `service` - The media service instance
/// * `identifier` - Optional player identifier
///
/// # Errors
///
/// Returns CliError if no player is found
pub async fn get_player_id_or_active(
    service: &MediaService,
    identifier: Option<&String>,
) -> Result<PlayerId, CliError> {
    if let Some(id) = identifier {
        let player_id = find_player_by_identifier(service, id).await?;

        service
            .set_active_player(Some(player_id.clone()))
            .await
            .map_err(|e| CliError::ServiceError {
                service: "Media".to_string(),
                details: e.to_string(),
            })?;

        Ok(player_id)
    } else {
        service.active_player().await.ok_or_else(|| {
            CliError::InvalidArgument {
                arg: "player-id".to_string(),
                reason: "No active player set. Specify a player ID or set one first with 'wayle media active <player-id>'.".to_string(),
            }
        })
    }
}

/// Formats a player's display name for output
///
/// Returns the player's identity if available, otherwise returns the bus name
pub async fn get_player_display_name(service: &MediaService, player_id: &PlayerId) -> String {
    let info_stream = service.player_info(player_id.clone());
    pin!(info_stream);
    if let Some(Ok(info)) = info_stream.next().await {
        info.identity
    } else {
        player_id.bus_name().to_string()
    }
}
