use async_trait::async_trait;
use futures::StreamExt;
use tokio::pin;

use crate::{
    cli::{CliError, Command, CommandResult, types::CommandMetadata},
    services::mpris::{MediaService, PlaybackState},
};

/// Command to list all available media players
///
/// Shows player index, name, and current playback state
pub struct ListCommand {}

impl ListCommand {
    /// Creates a new ListCommand
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Command for ListCommand {
    /// Lists all available media players with their current state
    ///
    /// # Arguments
    ///
    /// * `args` - No arguments used
    ///
    /// # Errors
    ///
    /// Returns CliError if media service initialization fails
    async fn execute(&self, _args: &[String]) -> CommandResult {
        let media_service =
            MediaService::new(Vec::new())
                .await
                .map_err(|e| CliError::ServiceError {
                    service: "Media".to_string(),
                    details: e.to_string(),
                })?;
        let players_stream = media_service.players();
        pin!(players_stream);
        let players = players_stream
            .next()
            .await
            .ok_or_else(|| CliError::ServiceError {
                service: "Media".to_string(),
                details: "Failed to get player list".to_string(),
            })?;

        if players.is_empty() {
            return Ok("No media players found".to_string());
        }

        let active_player = media_service.get_active_player().await;
        let mut output = format!("Found {} media player(s):\n\n", players.len());

        for (index, player_state) in players.iter().enumerate() {
            let player_num = index + 1;
            let player_id = &player_state.player_info.id;
            let is_active = active_player.as_ref() == Some(player_id);
            let active_marker = if is_active { " (active)" } else { "" };

            let identity = player_state.player_info.identity.clone();

            let playback_state = match player_state.playback_state {
                PlaybackState::Playing => "▶ Playing",
                PlaybackState::Paused => "⏸ Paused",
                PlaybackState::Stopped => "⏹ Stopped",
            };

            let track_info = if !player_state.metadata.title.is_empty() {
                format!(
                    " - {} by {}",
                    player_state.metadata.title, player_state.metadata.artist
                )
            } else {
                String::new()
            };

            output.push_str(&format!(
                "{player_num:2}. {identity:<30} {playback_state:>12}{track_info}{active_marker}\n"
            ));
        }

        output.push_str("\nUse player number or partial name with other commands.");
        Ok(output)
    }

    fn metadata(&self) -> CommandMetadata {
        CommandMetadata {
            name: "list".to_string(),
            description: "List all available media players".to_string(),
            category: "media".to_string(),
            args: vec![],
            examples: vec!["wayle media list".to_string()],
        }
    }
}
