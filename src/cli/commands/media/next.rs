use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    cli::{
        CliError, Command, CommandResult,
        types::{ArgType, CommandArg, CommandMetadata},
    },
    services::mpris::{MediaService, MprisMediaService},
};

use super::utils::{get_player_display_name, get_player_id_or_active};

/// Command to skip to the next track
///
/// Controls the active player by default, or a specific player if provided.
pub struct NextCommand {
    media_service: Arc<MprisMediaService>,
}

impl NextCommand {
    /// Creates a new NextCommand
    ///
    /// # Arguments
    ///
    /// * `media_service` - Shared reference to the media service
    pub fn new(media_service: Arc<MprisMediaService>) -> Self {
        Self { media_service }
    }
}

#[async_trait]
impl Command for NextCommand {
    /// Skip to the next track
    ///
    /// # Arguments
    ///
    /// * `args` - Optional player identifier (partial name match or index)
    ///
    /// # Errors
    ///
    /// Returns CliError if media service fails or player not found
    async fn execute(&self, args: &[String]) -> CommandResult {
        let player_id = get_player_id_or_active(&self.media_service, args.first()).await?;
        let player_name = get_player_display_name(&self.media_service, &player_id).await;

        self.media_service
            .next(player_id)
            .await
            .map_err(|e| CliError::ServiceError {
                service: "Media".to_string(),
                details: e.to_string(),
            })?;

        Ok(format!("Skipped to next track on: {player_name}"))
    }

    fn metadata(&self) -> CommandMetadata {
        CommandMetadata {
            name: "next".to_string(),
            description: "Skip to the next track".to_string(),
            category: "media".to_string(),
            args: vec![CommandArg {
                name: "player-id".to_string(),
                description: "Player identifier - can be a number (1, 2, etc.) or partial name match (e.g., 'spotify', 'firefox'). Uses active player if not specified.".to_string(),
                required: false,
                value_type: ArgType::String,
            }],
            examples: vec![
                "wayle media next".to_string(),
                "wayle media next 1".to_string(),
                "wayle media next spotify".to_string(),
            ],
        }
    }
}

