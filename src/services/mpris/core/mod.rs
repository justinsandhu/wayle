/// Track metadata representation.
pub mod metadata;
/// Media player representation with reactive properties.
pub mod player;

pub use metadata::{TrackMetadata, UNKNOWN_METADATA};
pub use player::Player;
