use async_trait::async_trait;

use crate::{
    cli::{
        CliError, Command, CommandResult,
        types::{ArgType, CommandArg, CommandMetadata},
    },
    services::mpris::{MediaService},
};

use super::utils::{get_player_display_name, get_player_id_or_active};

/// Command to toggle play/pause state of a media player
///
/// Controls the active player by default, or a specific player if provided.
pub struct PlayPauseCommand;

impl PlayPauseCommand {
    /// Creates a new PlayPauseCommand
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Command for PlayPauseCommand {
    /// Toggle play/pause state of a media player
    ///
    /// # Arguments
    ///
    /// * `args` - Optional player identifier (partial name match or index)
    ///
    /// # Errors
    ///
    /// Returns CliError if media service fails or player not found
    async fn execute(&self, args: &[String]) -> CommandResult {
        let media_service =
            MediaService::new(Vec::new())
                .await
                .map_err(|e| CliError::ServiceError {
                    service: "Media".to_string(),
                    details: format!("Failed to initialize media service: {e}"),
                })?;

        let player_id = get_player_id_or_active(&media_service, args.first()).await?;
        let player_name = get_player_display_name(&media_service, &player_id).await;

        media_service
            .play_pause(player_id)
            .await
            .map_err(|e| CliError::ServiceError {
                service: "Media".to_string(),
                details: e.to_string(),
            })?;

        Ok(format!("Toggled playback for: {player_name}"))
    }

    fn metadata(&self) -> CommandMetadata {
        CommandMetadata {
            name: "play-pause".to_string(),
            description: "Toggle play/pause state of a media player".to_string(),
            category: "media".to_string(),
            args: vec![CommandArg {
                name: "player-id".to_string(),
                description: "Player identifier - can be a number (1, 2, etc.) or partial name match (e.g., 'spotify', 'firefox'). Uses active player if not specified.".to_string(),
                required: false,
                value_type: ArgType::String,
            }],
            examples: vec![
                "wayle media play-pause".to_string(),
                "wayle media play-pause 1".to_string(),
                "wayle media play-pause spotify".to_string(),
                "wayle media play-pause fire".to_string(),
            ],
        }
    }
}
