use std::time::Duration;

use async_trait::async_trait;

use crate::{
    cli::{
        CliError, Command, CommandResult,
        types::{ArgType, CommandArg, CommandMetadata},
    },
    services::mpris::{
        LoopMode, MediaService, PlaybackState, PlayerId, ShuffleMode, TrackMetadata,
        UNKNOWN_METADATA,
    },
};

use super::utils::get_player_id_or_active;

/// Command to show detailed information about a media player
///
/// Displays current track, playback state, position, and player capabilities
pub struct InfoCommand {}

impl InfoCommand {
    /// Creates a new InfoCommand
    pub fn new() -> Self {
        Self {}
    }

    /// Format duration as MM:SS
    fn format_duration(duration: Duration) -> String {
        let total_seconds = duration.as_secs();
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        format!("{minutes:02}:{seconds:02}")
    }
}

#[async_trait]
impl Command for InfoCommand {
    /// Show detailed information about a media player
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
                    details: e.to_string(),
                })?;
        let player_id = get_player_id_or_active(&media_service, args.first()).await?;
        let mut output = String::new();

        self.add_player_info(&media_service, &player_id, &mut output)
            .await;
        self.add_playback_state(&media_service, &player_id, &mut output)
            .await;
        self.add_modes(&media_service, &player_id, &mut output)
            .await;
        self.add_track_info(&media_service, &player_id, &mut output)
            .await;

        Ok(output)
    }

    fn metadata(&self) -> CommandMetadata {
        CommandMetadata {
            name: "info".to_string(),
            description: "Show detailed information about a media player".to_string(),
            category: "media".to_string(),
            args: vec![CommandArg {
                name: "player-id".to_string(),
                description: "Player identifier - can be a number (1, 2, etc.) or partial name match. Uses active player if not specified.".to_string(),
                required: false,
                value_type: ArgType::String,
            }],
            examples: vec![
                "wayle media info".to_string(),
                "wayle media info 1".to_string(),
                "wayle media info spotify".to_string(),
            ],
        }
    }
}

impl InfoCommand {
    async fn add_player_info(
        &self,
        service: &MediaService,
        player_id: &PlayerId,
        output: &mut String,
    ) {
        if let Some(player) = service.player(player_id).await {
            output.push_str(&format!("Player: {}\n", player.identity.get()));
            output.push_str(&format!("Bus Name: {}\n", player_id.bus_name()));
            output.push_str(&format!("Can Control: {}\n\n", player.can_control.get()));

            output.push_str("Capabilities:\n");
            output.push_str(&format!("  Play/Pause: {}\n", player.can_play.get()));
            output.push_str(&format!("  Next Track: {}\n", player.can_go_next.get()));
            output.push_str(&format!(
                "  Previous Track: {}\n",
                player.can_go_previous.get()
            ));
            output.push_str(&format!("  Seek: {}\n", player.can_seek.get()));
            output.push_str(&format!("  Loop: {}\n", player.can_loop.get()));
            output.push_str(&format!("  Shuffle: {}\n\n", player.can_shuffle.get()));
        }
    }

    async fn add_playback_state(
        &self,
        service: &MediaService,
        player_id: &PlayerId,
        output: &mut String,
    ) {
        if let Some(player) = service.player(player_id).await {
            let state = player.playback_state.get();
            let state_str = match state {
                PlaybackState::Playing => "▶ Playing",
                PlaybackState::Paused => "⏸ Paused",
                PlaybackState::Stopped => "⏹ Stopped",
            };
            output.push_str(&format!("Playback State: {state_str}\n"));
        }
    }

    async fn add_modes(&self, service: &MediaService, player_id: &PlayerId, output: &mut String) {
        if let Some(player) = service.player(player_id).await {
            let loop_mode = player.loop_mode.get();
            let loop_str = match loop_mode {
                LoopMode::None => "Off",
                LoopMode::Track => "Track",
                LoopMode::Playlist => "Playlist",
                LoopMode::Unsupported => "Unsupported",
            };
            output.push_str(&format!("Loop Mode: {loop_str}\n"));

            let shuffle_mode = player.shuffle_mode.get();
            let shuffle_str = match shuffle_mode {
                ShuffleMode::On => "On",
                ShuffleMode::Off => "Off",
                ShuffleMode::Unsupported => "Unsupported",
            };
            output.push_str(&format!("Shuffle: {shuffle_str}\n\n"));
        }
    }

    async fn add_track_info(
        &self,
        service: &MediaService,
        player_id: &PlayerId,
        output: &mut String,
    ) {
        if let Some(player) = service.player(player_id).await {
            output.push_str("Current Track:\n");
            let title = player.title.get();
            if !title.is_empty() && title != UNKNOWN_METADATA {
                output.push_str(&format!("  Title: {title}\n"));
            }
            let artist = player.artist.get();
            if !artist.is_empty() && artist != UNKNOWN_METADATA {
                output.push_str(&format!("  Artist: {artist}\n"));
            }
            let album = player.album.get();
            if !album.is_empty() && album != UNKNOWN_METADATA {
                output.push_str(&format!("  Album: {album}\n"));
            }

            let metadata = TrackMetadata {
                title,
                artist,
                album,
                album_artist: player.album_artist.get(),
                length: player.length.get(),
                art_url: player.art_url.get(),
                track_id: player.track_id.get(),
            };
            self.add_position_info(service, player_id, &metadata, output)
                .await;

            if let Some(url) = player.art_url.get() {
                output.push_str(&format!("  Artwork URL: {url}\n"));
            }
        } else {
            output.push_str("No track currently loaded\n");
        }
    }

    async fn add_position_info(
        &self,
        service: &MediaService,
        player_id: &PlayerId,
        metadata: &TrackMetadata,
        output: &mut String,
    ) {
        let position = service.position(player_id).await.unwrap_or(Duration::ZERO);

        if let Some(length) = metadata.length {
            let percentage = (position.as_secs_f64() / length.as_secs_f64() * 100.0) as u32;
            output.push_str(&format!(
                "  Position: {} / {} ({percentage}%)\n",
                Self::format_duration(position),
                Self::format_duration(length),
            ));

            self.add_progress_bar(percentage, output);
        } else {
            output.push_str(&format!(
                "  Position: {}\n",
                Self::format_duration(position)
            ));
        }
    }

    fn add_progress_bar(&self, percentage: u32, output: &mut String) {
        let bar_width = 30_usize;
        let filled = bar_width * percentage as usize / 100;
        let empty = bar_width - filled;
        output.push_str("  Progress: [");
        output.push_str(&"=".repeat(filled));
        output.push_str(&" ".repeat(empty));
        output.push_str("]\n");
    }
}
