use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    cli::{CliError, Command, CommandResult, types::{CommandMetadata, CommandArg, ArgType}},
    services::audio::{AudioService, DeviceIndex, PulseAudioService, Volume},
};

/// Command to set device volume
///
/// Sets the volume level for a specific audio device
pub struct VolumeCommand {
    audio_service: Arc<PulseAudioService>,
}

impl VolumeCommand {
    /// Creates a new VolumeCommand
    ///
    /// # Arguments
    ///
    /// * `audio_service` - Shared reference to the audio service
    pub fn new(audio_service: Arc<PulseAudioService>) -> Self {
        Self { audio_service }
    }
}

#[async_trait]
impl Command for VolumeCommand {
    /// Sets the volume for a specific device
    ///
    /// # Arguments
    ///
    /// * `args` - [device_id, volume_level]
    ///
    /// # Errors
    ///
    /// Returns CliError if:
    /// - Wrong number of arguments
    /// - Invalid device ID or volume level
    /// - Device not found
    /// - Audio service operation fails
    async fn execute(&self, args: &[String]) -> CommandResult {
        if args.len() != 2 {
            return Err(CliError::MissingArguments {
                missing: "device_id, volume_level".to_string(),
                usage: "wayle audio volume <device_id> <level>".to_string(),
            });
        }

        let device_id = args[0].parse::<u32>().map_err(|_| CliError::InvalidArgument {
            arg: "device_id".to_string(),
            reason: "must be a valid device ID number".to_string(),
        })?;

        let level = args[1].parse::<f64>().map_err(|_| CliError::InvalidArgument {
            arg: "volume_level".to_string(),
            reason: "must be a number between 0.0 and 10.0".to_string(),
        })?;

        if level < 0.0 || level > 10.0 {
            return Err(CliError::InvalidArgument {
                arg: "volume_level".to_string(),
                reason: "must be between 0.0 and 10.0".to_string(),
            });
        }

        let device_index = DeviceIndex(device_id);
        
        // Verify device exists
        let device = self.audio_service.device(device_index).await.map_err(|e| {
            CliError::ServiceError {
                service: "Audio".to_string(),
                details: format!("Device {} not found: {}", device_id, e),
            }
        })?;
        
        // Create volume and set it
        let volume = Volume::mono(level).map_err(|e| {
            CliError::InvalidArgument {
                arg: "volume_level".to_string(),
                reason: format!("volume creation failed: {}", e),
            }
        })?;

        self.audio_service.set_device_volume(device_index, volume).await.map_err(|e| {
            CliError::ServiceError {
                service: "Audio".to_string(),
                details: format!("Failed to set volume: {}", e),
            }
        })?;
        
        Ok(format!(
            "Set volume to {:.1} for device {} ({})", 
            level, 
            device_id,
            device.name.as_str()
        ))
    }

    fn metadata(&self) -> CommandMetadata {
        CommandMetadata {
            name: "volume".to_string(),
            description: "Set volume for an audio device".to_string(),
            category: "audio".to_string(),
            args: vec![
                CommandArg {
                    name: "device_id".to_string(),
                    description: "ID of the device to control".to_string(),
                    required: true,
                    value_type: ArgType::Number,
                },
                CommandArg {
                    name: "level".to_string(),
                    description: "Volume level (0.0 to 10.0)".to_string(),
                    required: true,
                    value_type: ArgType::Number,
                },
            ],
            examples: vec![
                "wayle audio volume 1 5.0".to_string(),
                "wayle audio volume 2 0.8".to_string(),
            ],
        }
    }
}