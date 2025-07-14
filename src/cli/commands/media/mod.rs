/// Media player control commands
mod active;
mod info;
mod list;
mod loop_mode;
mod next;
mod play_pause;
mod previous;
mod seek;
mod shuffle;
mod utils;

pub use active::ActiveCommand;
pub use info::InfoCommand;
pub use list::ListCommand;
pub use loop_mode::LoopCommand;
pub use next::NextCommand;
pub use play_pause::PlayPauseCommand;
pub use previous::PreviousCommand;
pub use seek::SeekCommand;
pub use shuffle::ShuffleCommand;

use crate::cli::CommandRegistry;

/// Registers all media-related commands with the command registry
///
/// Registers commands in the "media" category for testing and interacting
/// with media players.
///
/// # Arguments
///
/// * `registry` - Mutable reference to the command registry
pub fn register_commands(registry: &mut CommandRegistry) {
    const CATEGORY_NAME: &str = "media";

    registry.register_command(CATEGORY_NAME, Box::new(ListCommand::new()));
    registry.register_command(CATEGORY_NAME, Box::new(PlayPauseCommand::new()));
    registry.register_command(CATEGORY_NAME, Box::new(NextCommand::new()));
    registry.register_command(CATEGORY_NAME, Box::new(PreviousCommand::new()));
    registry.register_command(CATEGORY_NAME, Box::new(SeekCommand::new()));
    registry.register_command(CATEGORY_NAME, Box::new(ShuffleCommand::new()));
    registry.register_command(CATEGORY_NAME, Box::new(LoopCommand::new()));
    registry.register_command(CATEGORY_NAME, Box::new(ActiveCommand::new()));
    registry.register_command(CATEGORY_NAME, Box::new(InfoCommand::new()));
}
