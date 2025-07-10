use std::sync::Arc;

use async_trait::async_trait;
use futures::StreamExt;
use tokio::pin;

use crate::{
    cli::{CliError, Command, CommandResult, types::CommandMetadata},
    services::audio::{AudioService, PulseAudioService},
};

/// Command to list all available audio devices
///
/// Shows device ID, type, name, description, and default status
pub struct DevicesCommand {
    audio_service: Arc<PulseAudioService>,
}

impl DevicesCommand {
    /// Creates a new DevicesCommand
    ///
    /// # Arguments
    ///
    /// * `audio_service` - Shared reference to the audio service
    pub fn new(audio_service: Arc<PulseAudioService>) -> Self {
        Self { audio_service }
    }
}

#[async_trait]
impl Command for DevicesCommand {
    /// Lists all available audio devices with their information
    ///
    /// # Arguments
    ///
    /// * `args` - No arguments used
    ///
    /// # Errors
    ///
    /// Returns CliError if audio service fails to get device list
    async fn execute(&self, _args: &[String]) -> CommandResult {
        let devices_stream = self.audio_service.devices();
        pin!(devices_stream);
        let devices = devices_stream
            .next()
            .await
            .ok_or_else(|| CliError::ServiceError {
                service: "Audio".to_string(),
                details: "Failed to get device list".to_string(),
            })?;

        if devices.is_empty() {
            return Ok("No audio devices found".to_string());
        }

        let mut output = format!("Found {} audio device(s):\n\n", devices.len());
        output.push_str(&format!("{:<4} {:<8} {:<30} {}\n", "ID", "Type", "Name", "Description"));
        output.push_str(&format!("{}\n", "-".repeat(70)));
        
        for device in devices {
            let device_type = match device.device_type {
                crate::services::audio::DeviceType::Input => "Input",
                crate::services::audio::DeviceType::Output => "Output",
            };
            
            let default_marker = if device.is_default { " (default)" } else { "" };
            
            output.push_str(&format!(
                "{:<4} {:<8} {:<30} {}{}\n",
                device.index.0,
                device_type,
                device.name.as_str(),
                device.description,
                default_marker
            ));
        }
        
        output.push_str("\nUse device ID with other audio commands.");
        Ok(output)
    }

    fn metadata(&self) -> CommandMetadata {
        CommandMetadata {
            name: "devices".to_string(),
            description: "List all available audio devices".to_string(),
            category: "audio".to_string(),
            args: vec![],
            examples: vec!["wayle audio devices".to_string()],
        }
    }
}