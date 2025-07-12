/// MPRIS media player control service
pub mod mpris;
/// PulseAudio control service
pub mod pulse;

pub use mpris::{MediaService, MprisMediaService};
pub use pulse::{
    AudioEvent, DeviceIndex, DeviceInfo, DeviceManager, DeviceStreams, DeviceType,
    DeviceVolumeController, PulseError, PulseService, StreamIndex, StreamInfo, StreamManager,
    StreamStreams, StreamType, StreamVolumeController, Volume,
};
