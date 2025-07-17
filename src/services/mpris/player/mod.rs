/// Player discovery and lifecycle management
pub mod discovery;
/// Player-specific events
pub mod events;
/// Player identification and basic info
pub mod info;
/// Player management functionality
pub mod manager;
/// Player property monitoring
pub mod monitoring;
/// Player state management
pub mod state;

pub use events::*;
pub use info::*;
pub use state::*;
