use async_trait::async_trait;

use crate::{
    cli::{CliError, Command, CommandResult, types::CommandMetadata},
    services::media::{Config, MediaService, PlaybackState, UNKNOWN_METADATA},
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
        let media_service = MediaService::start(Config {
            ignored_players: vec![],
        })
        .await
        .map_err(|e| CliError::ServiceError {
            service: "Media".to_string(),
            details: e.to_string(),
        })?;
        let players = media_service.players();

        if players.is_empty() {
            return Ok("No media players found".to_string());
        }

        let active_player = media_service.active_player();
        let mut output = format!("Found {} media player(s):\n\n", players.len());

        for (index, player) in players.iter().enumerate() {
            let player_num = index + 1;
            let player_id = &player.id;
            let is_active = active_player.as_ref().map(|p| &p.id) == Some(player_id);
            let active_marker = if is_active { " (active)" } else { "" };

            let identity = player.identity.get();

            let playback_state = match player.playback_state.get() {
                PlaybackState::Playing => "▶ Playing",
                PlaybackState::Paused => "⏸ Paused",
                PlaybackState::Stopped => "⏹ Stopped",
            };

            let title = player.metadata.title.get();
            let artist = player.metadata.artist.get();
            let track_info = if !title.is_empty() && title != UNKNOWN_METADATA {
                format!(" - {title} by {artist}")
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
