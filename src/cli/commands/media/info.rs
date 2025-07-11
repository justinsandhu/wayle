use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use futures::StreamExt;
use tokio::pin;

use crate::{
    cli::{
        Command, CommandResult,
        types::{ArgType, CommandArg, CommandMetadata},
    },
    services::mpris::{
        LoopMode, MediaService, MprisMediaService, PlaybackState, PlayerId, ShuffleMode,
        TrackMetadata,
    },
};

use super::utils::get_player_id_or_active;

/// Command to show detailed information about a media player
///
/// Displays current track, playback state, position, and player capabilities
pub struct InfoCommand {
    media_service: Arc<MprisMediaService>,
}

impl InfoCommand {
    /// Creates a new InfoCommand
    ///
    /// # Arguments
    ///
    /// * `media_service` - Shared reference to the media service
    pub fn new(media_service: Arc<MprisMediaService>) -> Self {
        Self { media_service }
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
        let player_id = get_player_id_or_active(&self.media_service, args.first()).await?;
        let mut output = String::new();

        self.add_player_info(self.media_service.as_ref(), &player_id, &mut output)
            .await;
        self.add_playback_state(self.media_service.as_ref(), &player_id, &mut output)
            .await;
        self.add_modes(self.media_service.as_ref(), &player_id, &mut output)
            .await;
        self.add_track_info(self.media_service.as_ref(), &player_id, &mut output)
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
        service: &impl MediaService,
        player_id: &PlayerId,
        output: &mut String,
    ) {
        let info_stream = service.player_info(player_id.clone());
        pin!(info_stream);
        if let Some(Ok(info)) = info_stream.next().await {
            output.push_str(&format!("Player: {}\n", info.identity));
            output.push_str(&format!("Bus Name: {}\n", player_id.bus_name()));
            output.push_str(&format!("Can Control: {}\n\n", info.can_control));

            output.push_str("Capabilities:\n");
            output.push_str(&format!("  Play/Pause: {}\n", info.capabilities.can_play));
            output.push_str(&format!(
                "  Next Track: {}\n",
                info.capabilities.can_go_next
            ));
            output.push_str(&format!(
                "  Previous Track: {}\n",
                info.capabilities.can_go_previous
            ));
            output.push_str(&format!("  Seek: {}\n", info.capabilities.can_seek));
            output.push_str(&format!("  Loop: {}\n", info.capabilities.can_loop));
            output.push_str(&format!("  Shuffle: {}\n\n", info.capabilities.can_shuffle));
        }
    }

    async fn add_playback_state(
        &self,
        service: &impl MediaService,
        player_id: &PlayerId,
        output: &mut String,
    ) {
        let state_stream = service.playback_state(player_id.clone());
        pin!(state_stream);
        if let Some(state) = state_stream.next().await {
            let state_str = match state {
                PlaybackState::Playing => "▶ Playing",
                PlaybackState::Paused => "⏸ Paused",
                PlaybackState::Stopped => "⏹ Stopped",
            };
            output.push_str(&format!("Playback State: {state_str}\n"));
        }
    }

    async fn add_modes(
        &self,
        service: &impl MediaService,
        player_id: &PlayerId,
        output: &mut String,
    ) {
        let loop_stream = service.loop_mode(player_id.clone());
        pin!(loop_stream);
        if let Some(loop_mode) = loop_stream.next().await {
            let loop_str = match loop_mode {
                LoopMode::None => "Off",
                LoopMode::Track => "Track",
                LoopMode::Playlist => "Playlist",
                LoopMode::Unsupported => "Unsupported",
            };
            output.push_str(&format!("Loop Mode: {loop_str}\n"));
        }

        let shuffle_stream = service.shuffle_mode(player_id.clone());
        pin!(shuffle_stream);
        if let Some(shuffle_mode) = shuffle_stream.next().await {
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
        service: &impl MediaService,
        player_id: &PlayerId,
        output: &mut String,
    ) {
        let metadata_stream = service.metadata(player_id.clone());
        pin!(metadata_stream);
        if let Some(metadata) = metadata_stream.next().await {
            output.push_str("Current Track:\n");
            if !metadata.title.is_empty() {
                output.push_str(&format!("  Title: {}\n", metadata.title));
            }
            if !metadata.artist.is_empty() {
                output.push_str(&format!("  Artist: {}\n", metadata.artist));
            }
            if !metadata.album.is_empty() {
                output.push_str(&format!("  Album: {}\n", metadata.album));
            }

            self.add_position_info(service, player_id, &metadata, output)
                .await;

            if let Some(url) = &metadata.artwork_url {
                output.push_str(&format!("  Artwork URL: {url}\n"));
            }
        } else {
            output.push_str("No track currently loaded\n");
        }
    }

    async fn add_position_info(
        &self,
        service: &impl MediaService,
        player_id: &PlayerId,
        metadata: &TrackMetadata,
        output: &mut String,
    ) {
        let position_stream = service.position(player_id.clone());
        pin!(position_stream);
        let position = position_stream.next().await.unwrap_or(Duration::ZERO);

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
