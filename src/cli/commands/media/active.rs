use async_trait::async_trait;

use crate::{
    cli::{
        CliError, Command, CommandResult,
        types::{ArgType, CommandArg, CommandMetadata},
    },
    runtime_state::RuntimeState,
    services::mpris::{Config, MediaService},
};

use super::utils::find_player_by_identifier;

/// Command to get or set the active media player
///
/// Without arguments, shows the current active player.
/// With an argument, sets the specified player as active.
pub struct ActiveCommand {}

impl ActiveCommand {
    /// Creates a new ActiveCommand
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Command for ActiveCommand {
    /// Get or set the active media player
    ///
    /// # Arguments
    ///
    /// * `args` - Optional player identifier to set as active
    ///
    /// # Errors
    ///
    /// Returns CliError if media service fails or player not found
    async fn execute(&self, args: &[String]) -> CommandResult {
        let media_service = MediaService::start(Config {
            ignored_players: vec![],
        })
        .await
        .map_err(|e| CliError::ServiceError {
            service: "Media".to_string(),
            details: e.to_string(),
        })?;

        if let Some(identifier) = args.first() {
            let player_id = find_player_by_identifier(&media_service, identifier)?;
            let player =
                media_service
                    .player(&player_id)
                    .ok_or_else(|| CliError::ServiceError {
                        service: "Media".to_string(),
                        details: format!("Player not found: {player_id:?}"),
                    })?;
            let player_name = player.identity.get();

            media_service
                .set_active_player(Some(player_id.clone()))
                .await
                .map_err(|e| CliError::ServiceError {
                    service: "Media".to_string(),
                    details: e.to_string(),
                })?;

            RuntimeState::set_active_player(Some(player_id.bus_name().to_string()))
                .await
                .map_err(|e| CliError::ServiceError {
                    service: "Media".to_string(),
                    details: format!("Failed to save active player: {e}"),
                })?;

            Ok(format!("Set active player to: {player_name}"))
        } else {
            match media_service.active_player() {
                Some(player) => {
                    let player_name = player.identity.get();
                    Ok(format!("Active player: {player_name}"))
                }
                None => Ok("No active player set".to_string()),
            }
        }
    }

    fn metadata(&self) -> CommandMetadata {
        CommandMetadata {
            name: "active".to_string(),
            description: "Get or set the active media player".to_string(),
            category: "media".to_string(),
            args: vec![CommandArg {
                name: "player-id".to_string(),
                description: "Player to set as active - can be a number (1, 2, etc.) or partial name match. If not provided, shows current active player.".to_string(),
                required: false,
                value_type: ArgType::String,
            }],
            examples: vec![
                "wayle media active".to_string(),
                "wayle media active 1".to_string(),
                "wayle media active spotify".to_string(),
            ],
        }
    }
}
