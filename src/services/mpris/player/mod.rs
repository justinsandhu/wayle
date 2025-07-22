//! Player domain - models, monitoring, control, and lifecycle management.

mod control;
mod discovery;
mod handle;
mod manager;
mod metadata;
mod model;
mod monitoring;

pub(crate) use control::Control;
pub(crate) use discovery::PlayerDiscovery;
pub(crate) use handle::PlayerHandle;
pub use metadata::{TrackMetadata, UNKNOWN_METADATA};
pub use model::Player;
pub(crate) use monitoring::PlayerMonitor;
