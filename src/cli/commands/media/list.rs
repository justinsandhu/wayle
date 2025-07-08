use std::sync::Arc;

use async_trait::async_trait;
use futures::StreamExt;
use tokio::pin;

use crate::{
    cli::{CliError, Command, CommandResult, types::CommandMetadata},
    services::mpris::{MediaService, MprisMediaService, PlaybackState},
};

/// Command to list all available media players
///
/// Shows player index, name, and current playback state
pub struct ListCommand {
    media_service: Arc<MprisMediaService>,
}

impl ListCommand {
    /// Creates a new ListCommand
    ///
    /// # Arguments
    ///
    /// * `media_service` - Shared reference to the media service
    pub fn new(media_service: Arc<MprisMediaService>) -> Self {
        Self { media_service }
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
        let players_stream = self.media_service.players();
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

        let active_player = self.media_service.active_player().await;
        let mut output = format!("Found {} media player(s):\n\n", players.len());

        for (index, player_id) in players.iter().enumerate() {
            let player_num = index + 1;
            let is_active = active_player.as_ref() == Some(player_id);
            let active_marker = if is_active { " (active)" } else { "" };

            let info_stream = self.media_service.player_info(player_id.clone());
            pin!(info_stream);
            let identity = if let Some(Ok(info)) = info_stream.next().await {
                info.identity
            } else {
                player_id.bus_name().to_string()
            };

            let state_stream = self.media_service.playback_state(player_id.clone());
            pin!(state_stream);
            let playback_state = if let Some(state) = state_stream.next().await {
                match state {
                    PlaybackState::Playing => "▶ Playing",
                    PlaybackState::Paused => "⏸ Paused",
                    PlaybackState::Stopped => "⏹ Stopped",
                }
            } else {
                "Unknown"
            };

            let metadata_stream = self.media_service.metadata(player_id.clone());
            pin!(metadata_stream);
            let track_info = if let Some(metadata) = metadata_stream.next().await {
                if !metadata.title.is_empty() {
                    format!(" - {} by {}", metadata.title, metadata.artist)
                } else {
                    String::new()
                }
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
