/// Common utilities and abstractions for services
pub mod common;
/// Media player control service
pub mod media;
/// Network control service
pub mod network;
/// Audio control service
pub mod audio;

pub use media::MediaService;
pub use audio::{
    AudioError, AudioEvent, AudioService, DeviceIndex, DeviceInfo, DeviceManager, DeviceStreams,
    DeviceType, DeviceVolumeController, StreamIndex, StreamInfo, StreamManager, StreamStreams,
    StreamType, StreamVolumeController, Volume,
};
