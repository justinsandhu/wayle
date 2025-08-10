use std::sync::Arc;

use crate::{
    cli::CliError,
    services::mpris::{MediaService, Player, PlayerId},
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
pub fn find_player_by_identifier(
    service: &MediaService,
    identifier: &str,
) -> Result<PlayerId, CliError> {
    let players = service.players();

    if players.is_empty() {
        return Err(CliError::InvalidArgument {
            arg: "player-id".to_string(),
            reason: "No media players found".to_string(),
        });
    }

    if let Ok(index) = identifier.parse::<usize>() {
        if index > 0 && index <= players.len() {
            return Ok(players[index - 1].id.clone());
        } else {
            return Err(CliError::InvalidArgument {
                arg: "player-id".to_string(),
                reason: format!("Invalid player index. Valid range: 1-{}", players.len()),
            });
        }
    }

    let identifier_lower = identifier.to_lowercase();
    let mut matches = Vec::new();

    for player in &players {
        let identity_lower = player.identity.get().to_lowercase();
        let bus_name_lower = player.id.bus_name().to_lowercase();

        if identity_lower.contains(&identifier_lower) || bus_name_lower.contains(&identifier_lower)
        {
            matches.push((player.id.clone(), player.identity.get()));
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
) -> Result<Arc<Player>, CliError> {
    if let Some(id) = identifier {
        let player_id = find_player_by_identifier(service, id)?;

        service
            .set_active_player(Some(player_id.clone()))
            .await
            .map_err(|e| CliError::ServiceError {
                service: "Media".to_string(),
                details: e.to_string(),
            })?;

        service
            .player(&player_id)
            .await
            .map_err(|e| CliError::ServiceError {
                service: "Media".to_string(),
                details: format!("Failed to get player '{player_id}': {e}"),
            })
    } else {
        service.active_player().ok_or_else(|| {
            CliError::InvalidArgument {
                arg: "player-id".to_string(),
                reason: "No active player set. Specify a player ID or set one first with 'wayle media active <player-id>'.".to_string(),
            }
        })
    }
}
