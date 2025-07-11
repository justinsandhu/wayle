use async_trait::async_trait;
use futures::Stream;
use std::{error::Error, pin::Pin};

use super::{AudioEvent, DeviceIndex, DeviceInfo, DeviceType, StreamIndex, StreamInfo, Volume};

#[async_trait]
/// Reactive audio service interface
///
/// Provides streaming data for UI reactivity and control methods for user actions.
/// Follows WirePlumber patterns with collection access, default device tracking,
/// and resource-specific subscriptions for efficient audio system management.
pub trait AudioService: Clone + Send + Sync + 'static {
    /// Error type for audio operations
    type Error: Error + Send + Sync + 'static;

    // === Collection Streams (WirePlumber: bulk collection access) ===

    /// Stream of all available audio devices
    fn devices(&self) -> Pin<Box<dyn Stream<Item = Vec<DeviceInfo>> + Send>>;

    /// Stream of input devices only
    fn input_devices(&self) -> Pin<Box<dyn Stream<Item = Vec<DeviceInfo>> + Send>>;

    /// Stream of output devices only  
    fn output_devices(&self) -> Pin<Box<dyn Stream<Item = Vec<DeviceInfo>> + Send>>;

    /// Stream of currently active audio streams
    fn streams(&self) -> Pin<Box<dyn Stream<Item = Vec<StreamInfo>> + Send>>;

    /// Stream of playback streams only
    fn playback_streams(&self) -> Pin<Box<dyn Stream<Item = Vec<StreamInfo>> + Send>>;

    /// Stream of recording streams only
    fn recording_streams(&self) -> Pin<Box<dyn Stream<Item = Vec<StreamInfo>> + Send>>;

    // === Global Event Streams (WirePlumber: main service events) ===

    /// Stream of all audio events
    fn events(&self) -> Pin<Box<dyn Stream<Item = AudioEvent> + Send>>;

    /// Stream of device-related events only
    fn device_events(&self) -> Pin<Box<dyn Stream<Item = AudioEvent> + Send>>;

    /// Stream of stream-related events only
    fn stream_events(&self) -> Pin<Box<dyn Stream<Item = AudioEvent> + Send>>;

    // === Default Device Streams (WirePlumber: default device access) ===

    /// Stream of default input device changes
    fn default_input(&self) -> Pin<Box<dyn Stream<Item = DeviceInfo> + Send>>;

    /// Stream of default output device changes
    fn default_output(&self) -> Pin<Box<dyn Stream<Item = DeviceInfo> + Send>>;

    // === Resource-Specific Streams (WirePlumber: per-endpoint subscriptions) ===

    /// Stream of volume changes for a specific device
    fn device_volume(&self, device: DeviceIndex) -> Pin<Box<dyn Stream<Item = Volume> + Send>>;

    /// Stream of mute state changes for a specific device
    fn device_mute(&self, device: DeviceIndex) -> Pin<Box<dyn Stream<Item = bool> + Send>>;

    /// Stream of state changes for a specific device
    fn device_state(&self, device: DeviceIndex) -> Pin<Box<dyn Stream<Item = DeviceInfo> + Send>>;

    /// Stream of volume changes for a specific stream
    fn stream_volume(&self, stream: StreamIndex) -> Pin<Box<dyn Stream<Item = Volume> + Send>>;

    /// Stream of mute state changes for a specific stream
    fn stream_mute(&self, stream: StreamIndex) -> Pin<Box<dyn Stream<Item = bool> + Send>>;

    /// Stream of state changes for a specific stream
    fn stream_state(&self, stream: StreamIndex) -> Pin<Box<dyn Stream<Item = StreamInfo> + Send>>;

    // === Point-in-Time Queries (WirePlumber: getter pattern) ===

    /// Get current device information
    async fn device(&self, device: DeviceIndex) -> Result<DeviceInfo, Self::Error>;

    /// Get current stream information
    async fn stream(&self, stream: StreamIndex) -> Result<StreamInfo, Self::Error>;

    /// Get all devices of specific type
    async fn devices_by_type(
        &self,
        device_type: DeviceType,
    ) -> Result<Vec<DeviceInfo>, Self::Error>;

    /// Get current default input device
    async fn current_default_input(&self) -> Result<Option<DeviceInfo>, Self::Error>;

    /// Get current default output device
    async fn current_default_output(&self) -> Result<Option<DeviceInfo>, Self::Error>;

    // === Control Operations (WirePlumber: device/stream control) ===

    /// Set device volume
    async fn set_device_volume(
        &self,
        device: DeviceIndex,
        volume: Volume,
    ) -> Result<(), Self::Error>;

    /// Set device mute state
    async fn set_device_mute(&self, device: DeviceIndex, muted: bool) -> Result<(), Self::Error>;

    /// Set stream volume
    async fn set_stream_volume(
        &self,
        stream: StreamIndex,
        volume: Volume,
    ) -> Result<(), Self::Error>;

    /// Set stream mute state
    async fn set_stream_mute(&self, stream: StreamIndex, muted: bool) -> Result<(), Self::Error>;

    /// Set default input device
    async fn set_default_input(&self, device: DeviceIndex) -> Result<(), Self::Error>;

    /// Set default output device
    async fn set_default_output(&self, device: DeviceIndex) -> Result<(), Self::Error>;

    /// Move stream to different device
    async fn move_stream(
        &self,
        stream: StreamIndex,
        device: DeviceIndex,
    ) -> Result<(), Self::Error>;
}
