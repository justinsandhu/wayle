use std::time::Duration;

use async_trait::async_trait;

use crate::{
    cli::{
        CliError, Command, CommandResult,
        types::{ArgType, CommandArg, CommandMetadata},
    },
    services::mpris::{Config, MediaService},
};

use super::utils::get_player_id_or_active;

/// Command to seek to a specific position in the current track
///
/// Supports various time formats like seconds, mm:ss, or percentage
pub struct SeekCommand {}

impl SeekCommand {
    /// Creates a new SeekCommand
    pub fn new() -> Self {
        Self {}
    }

    fn parse_position(
        position_str: &str,
        current_position: Option<Duration>,
        track_length: Option<Duration>,
    ) -> Result<Duration, CliError> {
        if let Some(percentage_str) = position_str.strip_suffix('%') {
            let percentage =
                percentage_str
                    .parse::<f64>()
                    .map_err(|_| CliError::InvalidArgument {
                        arg: "position".to_string(),
                        reason: "Invalid percentage format".to_string(),
                    })?;

            if !(0.0..=100.0).contains(&percentage) {
                return Err(CliError::InvalidArgument {
                    arg: "position".to_string(),
                    reason: "Percentage must be between 0 and 100".to_string(),
                });
            }

            let track_length = track_length.ok_or_else(|| CliError::InvalidArgument {
                arg: "position".to_string(),
                reason: "Cannot use percentage - track length unknown".to_string(),
            })?;

            let position_secs = track_length.as_secs_f64() * (percentage / 100.0);
            return Ok(Duration::from_secs_f64(position_secs));
        }

        if position_str.starts_with('+') || position_str.starts_with('-') {
            let current = current_position.ok_or_else(|| CliError::InvalidArgument {
                arg: "position".to_string(),
                reason: "Cannot use relative seeking - current position unknown".to_string(),
            })?;

            let delta_str = &position_str[1..];
            let delta_secs = delta_str
                .parse::<i64>()
                .map_err(|_| CliError::InvalidArgument {
                    arg: "position".to_string(),
                    reason: "Invalid relative seek format".to_string(),
                })?;

            let new_position = if position_str.starts_with('+') {
                current.saturating_add(Duration::from_secs(delta_secs.unsigned_abs()))
            } else {
                current.saturating_sub(Duration::from_secs(delta_secs.unsigned_abs()))
            };

            return Ok(new_position);
        }

        if position_str.contains(':') {
            let parts: Vec<&str> = position_str.split(':').collect();
            if parts.len() != 2 {
                return Err(CliError::InvalidArgument {
                    arg: "position".to_string(),
                    reason: "Invalid time format. Use mm:ss".to_string(),
                });
            }

            let minutes = parts[0]
                .parse::<u64>()
                .map_err(|_| CliError::InvalidArgument {
                    arg: "position".to_string(),
                    reason: "Invalid minutes value".to_string(),
                })?;

            let seconds = parts[1]
                .parse::<u64>()
                .map_err(|_| CliError::InvalidArgument {
                    arg: "position".to_string(),
                    reason: "Invalid seconds value".to_string(),
                })?;

            if seconds >= 60 {
                return Err(CliError::InvalidArgument {
                    arg: "position".to_string(),
                    reason: "Seconds must be less than 60".to_string(),
                });
            }

            return Ok(Duration::from_secs(minutes * 60 + seconds));
        }

        let seconds = position_str.parse::<u64>().map_err(|_| {
            CliError::InvalidArgument {
                arg: "position".to_string(),
                reason: "Invalid position format. Use seconds, mm:ss, percentage (50%), or relative (+10, -10)".to_string(),
            }
        })?;

        Ok(Duration::from_secs(seconds))
    }
}

#[async_trait]
impl Command for SeekCommand {
    /// Seek to a specific position in the current track
    ///
    /// # Arguments
    ///
    /// * `args` - Position (required) and optional player identifier
    ///
    /// # Errors
    ///
    /// Returns CliError if media service fails, player not found, or invalid position
    async fn execute(&self, args: &[String]) -> CommandResult {
        if args.is_empty() {
            return Err(CliError::MissingArgument {
                arg: "position".to_string(),
                command: "seek".to_string(),
            });
        }

        let position_str = &args[0];
        let player_arg = args.get(1);

        let media_service = MediaService::start(Config {
            ignored_players: vec![],
        })
        .await
        .map_err(|e| CliError::ServiceError {
            service: "Media".to_string(),
            details: e.to_string(),
        })?;
        let player = get_player_id_or_active(&media_service, player_arg).await?;
        let player_name = player.identity.get();

        let current_position = player.position().await.ok();

        let track_length = player.metadata.length.get();

        let target_position = Self::parse_position(position_str, current_position, track_length)?;

        if let Some(length) = track_length {
            if target_position > length {
                return Err(CliError::InvalidArgument {
                    arg: "position".to_string(),
                    reason: format!("Position {target_position:?} exceeds track length {length:?}"),
                });
            }
        }

        player
            .seek(target_position)
            .await
            .map_err(|e| CliError::ServiceError {
                service: "Media".to_string(),
                details: e.to_string(),
            })?;

        Ok(format!(
            "Seeked to {:02}:{:02} on: {}",
            target_position.as_secs() / 60,
            target_position.as_secs() % 60,
            player_name
        ))
    }

    fn metadata(&self) -> CommandMetadata {
        CommandMetadata {
            name: "seek".to_string(),
            description: "Seek to a specific position in the current track".to_string(),
            category: "media".to_string(),
            args: vec![
                CommandArg {
                    name: "position".to_string(),
                    description: "Target position - seconds (30), time (1:30), percentage (50%), or relative (+10, -10)".to_string(),
                    required: true,
                    value_type: ArgType::String,
                },
                CommandArg {
                    name: "player-id".to_string(),
                    description: "Player identifier - can be a number (1, 2, etc.) or partial name match. Uses active player if not specified.".to_string(),
                    required: false,
                    value_type: ArgType::String,
                },
            ],
            examples: vec![
                "wayle media seek 30".to_string(),
                "wayle media seek 1:30".to_string(),
                "wayle media seek 50%".to_string(),
                "wayle media seek +10".to_string(),
                "wayle media seek -15 spotify".to_string(),
            ],
        }
    }
}
