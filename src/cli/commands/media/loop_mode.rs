use async_trait::async_trait;

use crate::{
    cli::{
        CliError, Command, CommandResult,
        types::{ArgType, CommandArg, CommandMetadata},
    },
    services::media::{Config, MediaService},
};

use super::utils::get_player_id_or_active;

/// Command to toggle or set loop mode
///
/// Controls the active player by default, or a specific player if provided.
pub struct LoopCommand {}

impl LoopCommand {
    /// Creates a new LoopCommand
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Command for LoopCommand {
    /// Toggle or set loop mode for a media player
    ///
    /// # Arguments
    ///
    /// * `args` - Optional player identifier (partial name match or index)
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
        let player = get_player_id_or_active(&media_service, args.first()).await?;
        let player_name = player.identity.get();

        player
            .toggle_loop()
            .await
            .map_err(|e| CliError::ServiceError {
                service: "Media".to_string(),
                details: e.to_string(),
            })?;

        Ok(format!("Loop mode changed for: {player_name}"))
    }

    fn metadata(&self) -> CommandMetadata {
        CommandMetadata {
            name: "loop".to_string(),
            description: "Toggle loop mode for a media player (cycles through: Off → Track → Playlist → Off)".to_string(),
            category: "media".to_string(),
            args: vec![CommandArg {
                name: "player-id".to_string(),
                description: "Player identifier - can be a number (1, 2, etc.) or partial name match (e.g., 'spotify', 'firefox'). Uses active player if not specified.".to_string(),
                required: false,
                value_type: ArgType::String,
            }],
            examples: vec![
                "wayle media loop".to_string(),
                "wayle media loop 1".to_string(),
                "wayle media loop spotify".to_string(),
            ],
        }
    }
}
