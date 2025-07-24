/// Common utilities and abstractions for services
pub mod common;
/// MPRIS media player control service
pub mod mpris;
/// Network Manager control service
pub mod network_manager;
/// PulseAudio control service
pub mod pulse;

pub use mpris::MediaService;
pub use pulse::{
    AudioEvent, DeviceIndex, DeviceInfo, DeviceManager, DeviceStreams, DeviceType,
    DeviceVolumeController, PulseError, PulseService, StreamIndex, StreamInfo, StreamManager,
    StreamStreams, StreamType, StreamVolumeController, Volume,
};
