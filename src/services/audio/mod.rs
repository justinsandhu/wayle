/// PulseAudio backend implementation
pub mod backend;
/// Device management domain
pub mod device;
/// Discovery functionality
pub mod discovery;
/// Error types
pub mod error;
/// Event types and handling
pub mod events;
/// Audio service implementation
mod service;
/// Stream management domain
pub mod stream;
/// Tokio mainloop for PulseAudio
pub mod tokio_mainloop;
/// Volume control domain
pub mod volume;

pub use device::{
    DeviceIndex, DeviceInfo, DeviceManager, DeviceStreams, DeviceType, DeviceVolumeController,
};
pub use error::AudioError;
pub use events::AudioEvent;
pub use service::AudioService;
pub use stream::{
    StreamIndex, StreamInfo, StreamManager, StreamStreams, StreamType, StreamVolumeController,
};
pub use volume::{Volume, VolumeError};
