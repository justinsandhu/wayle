pub mod devices;
pub mod volume;
pub mod mute;

pub use devices::*;
pub use volume::*;
pub use mute::*;

use crate::{cli::CommandRegistry, service_manager::Services};

/// Registers all audio-related commands with the command registry
///
/// Registers commands in the "audio" category for controlling audio devices,
/// volume, and mute states.
///
/// # Arguments
///
/// * `registry` - Mutable reference to the command registry
/// * `services` - Application services container
pub fn register_commands(registry: &mut CommandRegistry, services: &Services) {
    const CATEGORY_NAME: &str = "audio";

    registry.register_command(
        CATEGORY_NAME,
        Box::new(DevicesCommand::new(services.audio.clone())),
    );
    registry.register_command(
        CATEGORY_NAME,
        Box::new(VolumeCommand::new(services.audio.clone())),
    );
    registry.register_command(
        CATEGORY_NAME,
        Box::new(MuteCommand::new(services.audio.clone())),
    );
}