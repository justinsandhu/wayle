use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    cli::{
        CliError, Command, CommandResult,
        types::{ArgType, CommandArg, CommandMetadata},
    },
    services::mpris::{MediaService, MprisMediaService},
};

use super::utils::{find_player_by_identifier, get_player_display_name};

/// Command to get or set the active media player
///
/// Without arguments, shows the current active player.
/// With an argument, sets the specified player as active.
pub struct ActiveCommand {
    media_service: Arc<MprisMediaService>,
}

impl ActiveCommand {
    /// Creates a new ActiveCommand
    ///
    /// # Arguments
    ///
    /// * `media_service` - Shared reference to the media service
    pub fn new(media_service: Arc<MprisMediaService>) -> Self {
        Self { media_service }
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
        if let Some(identifier) = args.first() {
            let player_id = find_player_by_identifier(&self.media_service, identifier).await?;
            let player_name = get_player_display_name(&self.media_service, &player_id).await;

            self.media_service
                .set_active_player(Some(player_id))
                .await
                .map_err(|e| CliError::ServiceError {
                    service: "Media".to_string(),
                    details: e.to_string(),
                })?;

            Ok(format!("Set active player to: {player_name}"))
        } else {
            match self.media_service.active_player().await {
                Some(player_id) => {
                    let player_name = get_player_display_name(&self.media_service, &player_id).await;
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
