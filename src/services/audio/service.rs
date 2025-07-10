use async_trait::async_trait;
use futures::Stream;
use std::{error::Error, pin::Pin};

use super::{AudioEvent, DeviceIndex, DeviceInfo, StreamIndex, StreamInfo, Volume};

#[async_trait]
/// Reactive audio service interface
/// 
/// Provides streaming data for UI reactivity and control methods for user actions.
/// All streams automatically handle device/stream lifecycle and provide clean domain objects.
pub trait AudioService: Clone + Send + Sync + 'static {
    /// Error type for audio operations
    type Error: Error + Send + Sync + 'static;

    /// Stream of currently available audio devices
    fn devices(&self) -> Pin<Box<dyn Stream<Item = Vec<DeviceInfo>> + Send>>;

    /// Stream of currently active audio streams
    fn streams(&self) -> Pin<Box<dyn Stream<Item = Vec<StreamInfo>> + Send>>;

    /// Stream of all audio events
    fn events(&self) -> Pin<Box<dyn Stream<Item = AudioEvent> + Send>>;

    /// Stream of default input device changes
    fn default_input(&self) -> Pin<Box<dyn Stream<Item = DeviceInfo> + Send>>;

    /// Stream of default output device changes
    fn default_output(&self) -> Pin<Box<dyn Stream<Item = DeviceInfo> + Send>>;

    /// Get current device information
    async fn device(&self, device: DeviceIndex) -> Result<DeviceInfo, Self::Error>;

    /// Get current stream information
    async fn stream(&self, stream: StreamIndex) -> Result<StreamInfo, Self::Error>;

    /// Set device volume
    async fn set_device_volume(&self, device: DeviceIndex, volume: Volume) -> Result<(), Self::Error>;

    /// Set device mute state
    async fn set_device_mute(&self, device: DeviceIndex, muted: bool) -> Result<(), Self::Error>;

    /// Set default device
    async fn set_default_device(&self, device: DeviceIndex) -> Result<(), Self::Error>;

    /// Set stream volume
    async fn set_stream_volume(&self, stream: StreamIndex, volume: Volume) -> Result<(), Self::Error>;

    /// Set stream mute state
    async fn set_stream_mute(&self, stream: StreamIndex, muted: bool) -> Result<(), Self::Error>;

    /// Move stream to different device
    async fn move_stream(&self, stream: StreamIndex, device: DeviceIndex) -> Result<(), Self::Error>;
}