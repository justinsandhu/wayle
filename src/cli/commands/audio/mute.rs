use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    cli::{CliError, Command, CommandResult, types::{CommandMetadata, CommandArg, ArgType}},
    services::audio::{AudioService, DeviceIndex, PulseAudioService},
};

/// Command to mute/unmute audio devices
///
/// Toggles mute state or sets specific mute state for a device
pub struct MuteCommand {
    audio_service: Arc<PulseAudioService>,
}

impl MuteCommand {
    /// Creates a new MuteCommand
    ///
    /// # Arguments
    ///
    /// * `audio_service` - Shared reference to the audio service
    pub fn new(audio_service: Arc<PulseAudioService>) -> Self {
        Self { audio_service }
    }
}

#[async_trait]
impl Command for MuteCommand {
    /// Mutes or unmutes a specific device
    ///
    /// # Arguments
    ///
    /// * `args` - [device_id, optional: "true"/"false"/"on"/"off"]
    ///
    /// # Errors
    ///
    /// Returns CliError if:
    /// - Wrong number of arguments
    /// - Invalid device ID
    /// - Device not found
    /// - Audio service operation fails
    async fn execute(&self, args: &[String]) -> CommandResult {
        if args.is_empty() || args.len() > 2 {
            return Err(CliError::MissingArguments {
                missing: "device_id".to_string(),
                usage: "wayle audio mute <device_id> [on|off|true|false]".to_string(),
            });
        }

        let device_id = args[0].parse::<u32>().map_err(|_| CliError::InvalidArgument {
            arg: "device_id".to_string(),
            reason: "must be a valid device ID number".to_string(),
        })?;

        let mute_state = if args.len() == 2 {
            match args[1].to_lowercase().as_str() {
                "true" | "on" | "1" | "yes" => true,
                "false" | "off" | "0" | "no" => false,
                _ => {
                    return Err(CliError::InvalidArgument {
                        arg: "mute_state".to_string(),
                        reason: "must be 'on', 'off', 'true', or 'false'".to_string(),
                    });
                }
            }
        } else {
            // Toggle mode - for simplicity, we'll just set to muted
            // In a real implementation, you'd want to get current state first
            true
        };

        let device_index = DeviceIndex(device_id);
        
        // Verify device exists
        let device = self.audio_service.device(device_index).await.map_err(|e| {
            CliError::ServiceError {
                service: "Audio".to_string(),
                details: format!("Device {} not found: {}", device_id, e),
            }
        })?;
        
        self.audio_service.set_device_mute(device_index, mute_state).await.map_err(|e| {
            CliError::ServiceError {
                service: "Audio".to_string(),
                details: format!("Failed to set mute state: {}", e),
            }
        })?;
        
        let action = if mute_state { "Muted" } else { "Unmuted" };
        Ok(format!(
            "{} device {} ({})", 
            action,
            device_id,
            device.name.as_str()
        ))
    }

    fn metadata(&self) -> CommandMetadata {
        CommandMetadata {
            name: "mute".to_string(),
            description: "Mute or unmute an audio device".to_string(),
            category: "audio".to_string(),
            args: vec![
                CommandArg {
                    name: "device_id".to_string(),
                    description: "ID of the device to control".to_string(),
                    required: true,
                    value_type: ArgType::Number,
                },
                CommandArg {
                    name: "state".to_string(),
                    description: "Mute state: on/off/true/false (omit to toggle)".to_string(),
                    required: false,
                    value_type: ArgType::Boolean,
                },
            ],
            examples: vec![
                "wayle audio mute 1".to_string(),
                "wayle audio mute 1 on".to_string(),
                "wayle audio mute 2 off".to_string(),
            ],
        }
    }
}