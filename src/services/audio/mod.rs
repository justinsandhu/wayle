use std::{
    collections::HashMap,
    pin::Pin,
    sync::{Arc, RwLock},
};

use async_stream::stream;
use async_trait::async_trait;
use futures::{Stream, StreamExt, pin_mut};
use tokio::sync::{broadcast, mpsc};

/// Audio device management
pub mod device;
/// Audio error types
pub mod error;
/// Audio event types
pub mod events;
/// PulseAudio-specific implementation
pub mod pulse;
/// Audio service trait definitions
pub mod service;
/// Audio stream management
pub mod stream;
/// Test module for PulseAudio reactive streams
pub mod test_pulse;
/// Tokio mainloop for PulseAudio
pub mod tokio_mainloop;
/// Volume control types
pub mod volume;

pub use device::*;
pub use error::*;
pub use events::*;
pub use pulse::PulseCommand;
pub use service::*;
pub use stream::*;
pub use volume::*;

/// PulseAudio service errors
#[derive(thiserror::Error, Debug)]
pub enum PulseError {
    /// PulseAudio connection failed
    #[error("PulseAudio connection failed: {0}")]
    ConnectionFailed(String),

    /// PulseAudio operation failed
    #[error("PulseAudio operation failed: {0}")]
    OperationFailed(String),

    /// Volume conversion failed
    #[error("Volume conversion failed: {0}")]
    VolumeConversion(#[from] VolumeError),

    /// Device not found
    #[error("Device {0:?} not found")]
    DeviceNotFound(DeviceIndex),

    /// Stream not found
    #[error("Stream {0:?} not found")]
    StreamNotFound(StreamIndex),

    /// Thread communication failed
    #[error("PulseAudio thread communication failed")]
    ThreadCommunication,
}

use pulse::PulseImplementation;

/// PulseAudio service implementation
///
/// Main service struct that coordinates audio device and stream management
/// through PulseAudio. Provides reactive streams for UI updates and control
/// methods for audio operations.
pub struct PulseAudioService {
    command_tx: mpsc::UnboundedSender<PulseCommand>,

    device_list_tx: Arc<broadcast::Sender<Vec<DeviceInfo>>>,
    stream_list_tx: Arc<broadcast::Sender<Vec<StreamInfo>>>,
    events_tx: Arc<broadcast::Sender<AudioEvent>>,

    devices: Arc<RwLock<HashMap<DeviceKey, DeviceInfo>>>,
    streams: Arc<RwLock<HashMap<StreamIndex, StreamInfo>>>,
    default_input: Arc<RwLock<Option<DeviceInfo>>>,
    default_output: Arc<RwLock<Option<DeviceInfo>>>,
    server_info: Arc<RwLock<Option<String>>>,

    monitoring_handle: Option<tokio::task::JoinHandle<()>>,
}

impl Clone for PulseAudioService {
    fn clone(&self) -> Self {
        Self {
            command_tx: self.command_tx.clone(),
            device_list_tx: self.device_list_tx.clone(),
            stream_list_tx: self.stream_list_tx.clone(),
            events_tx: self.events_tx.clone(),
            devices: Arc::clone(&self.devices),
            streams: Arc::clone(&self.streams),
            default_input: Arc::clone(&self.default_input),
            default_output: Arc::clone(&self.default_output),
            server_info: Arc::clone(&self.server_info),
            monitoring_handle: None,
        }
    }
}

impl PulseAudioService {
    /// Creates a new PulseAudio service instance with active monitoring
    ///
    /// Initializes broadcast channels for device/stream updates and spawns a background
    /// task to monitor PulseAudio events. The service maintains real-time state of all
    /// audio devices and streams accessible through reactive streams.
    ///
    /// # Errors
    /// Returns error if PulseAudio connection fails or monitoring task spawn fails
    pub async fn new() -> Result<Self, PulseError> {
        let (command_tx, command_rx) = mpsc::unbounded_channel();

        let (device_list_tx, _) = broadcast::channel(100);
        let (stream_list_tx, _) = broadcast::channel(100);
        let (events_tx, _) = broadcast::channel(100);

        let devices = Arc::new(RwLock::new(HashMap::new()));
        let streams = Arc::new(RwLock::new(HashMap::new()));
        let default_input = Arc::new(RwLock::new(None));
        let default_output = Arc::new(RwLock::new(None));
        let server_info = Arc::new(RwLock::new(None));

        let monitoring_handle = PulseImplementation::spawn_monitoring_task(
            command_rx,
            device_list_tx.clone(),
            stream_list_tx.clone(),
            events_tx.clone(),
            devices.clone(),
            streams.clone(),
            default_input.clone(),
            default_output.clone(),
            server_info.clone(),
        )
        .await?;

        Ok(Self {
            command_tx,
            device_list_tx: Arc::new(device_list_tx),
            stream_list_tx: Arc::new(stream_list_tx),
            events_tx: Arc::new(events_tx),
            devices,
            streams,
            default_input,
            default_output,
            server_info,
            monitoring_handle: Some(monitoring_handle),
        })
    }
}

impl Drop for PulseAudioService {
    fn drop(&mut self) {
        if let Some(handle) = self.monitoring_handle.take() {
            handle.abort();
        }
    }
}

#[async_trait]
impl AudioService for PulseAudioService {
    type Error = PulseError;

    fn devices(&self) -> Pin<Box<dyn Stream<Item = Vec<DeviceInfo>> + Send>> {
        let devices = std::sync::Arc::clone(&self.devices);
        let mut device_list_rx = self.device_list_tx.subscribe();

        Box::pin(stream! {
            let mut current_devices = if let Ok(devices_guard) = devices.read() {
                let device_list: Vec<DeviceInfo> = devices_guard.values().cloned().collect();
                device_list
            } else {
                Vec::new()
            };

            if !current_devices.is_empty() {
                yield current_devices.clone();
            }

            while let Ok(device_list) = device_list_rx.recv().await {
                if current_devices != device_list {
                    current_devices = device_list.clone();
                    yield device_list;
                }
            }
        })
    }

    fn input_devices(&self) -> Pin<Box<dyn Stream<Item = Vec<DeviceInfo>> + Send>> {
        let devices_stream = self.devices();
        Box::pin(stream! {
            pin_mut!(devices_stream);
            while let Some(device_list) = devices_stream.next().await {
                let input_devices: Vec<DeviceInfo> = device_list
                    .into_iter()
                    .filter(|d| d.device_type == DeviceType::Input)
                    .collect();
                yield input_devices;
            }
        })
    }

    fn output_devices(&self) -> Pin<Box<dyn Stream<Item = Vec<DeviceInfo>> + Send>> {
        let devices_stream = self.devices();
        Box::pin(stream! {
            pin_mut!(devices_stream);
            while let Some(device_list) = devices_stream.next().await {
                let output_devices: Vec<DeviceInfo> = device_list
                    .into_iter()
                    .filter(|d| d.device_type == DeviceType::Output)
                    .collect();
                yield output_devices;
            }
        })
    }

    fn streams(&self) -> Pin<Box<dyn Stream<Item = Vec<StreamInfo>> + Send>> {
        let streams = std::sync::Arc::clone(&self.streams);
        let mut stream_list_rx = self.stream_list_tx.subscribe();

        Box::pin(stream! {
            {
                let stream_list = if let Ok(streams_guard) = streams.read() {
                    streams_guard.values().cloned().collect()
                } else {
                    Vec::new()
                };
                yield stream_list;
            }

            while let Ok(stream_list) = stream_list_rx.recv().await {
                yield stream_list;
            }
        })
    }

    fn playback_streams(&self) -> Pin<Box<dyn Stream<Item = Vec<StreamInfo>> + Send>> {
        let streams_stream = self.streams();
        Box::pin(stream! {
            pin_mut!(streams_stream);
            while let Some(stream_list) = streams_stream.next().await {
                let playback_streams: Vec<StreamInfo> = stream_list
                    .into_iter()
                    .filter(|s| s.stream_type == StreamType::Playback)
                    .collect();
                yield playback_streams;
            }
        })
    }

    fn recording_streams(&self) -> Pin<Box<dyn Stream<Item = Vec<StreamInfo>> + Send>> {
        let streams_stream = self.streams();
        Box::pin(stream! {
            pin_mut!(streams_stream);
            while let Some(stream_list) = streams_stream.next().await {
                let recording_streams: Vec<StreamInfo> = stream_list
                    .into_iter()
                    .filter(|s| matches!(s.stream_type, StreamType::Record | StreamType::Capture))
                    .collect();
                yield recording_streams;
            }
        })
    }

    fn events(&self) -> Pin<Box<dyn Stream<Item = AudioEvent> + Send>> {
        let mut events_rx = self.events_tx.subscribe();
        Box::pin(stream! {
            while let Ok(event) = events_rx.recv().await {
                yield event;
            }
        })
    }

    fn device_events(&self) -> Pin<Box<dyn Stream<Item = AudioEvent> + Send>> {
        let events_stream = self.events();
        Box::pin(stream! {
            pin_mut!(events_stream);
            while let Some(event) = events_stream.next().await {
                match event {
                    AudioEvent::DeviceAdded(_) |
                    AudioEvent::DeviceRemoved(_) |
                    AudioEvent::DeviceVolumeChanged { .. } |
                    AudioEvent::DeviceMuteChanged { .. } |
                    AudioEvent::DeviceChanged(_) |
                    AudioEvent::DefaultInputChanged(_) |
                    AudioEvent::DefaultOutputChanged(_) => {
                        yield event;
                    }
                    _ => {}
                }
            }
        })
    }

    fn stream_events(&self) -> Pin<Box<dyn Stream<Item = AudioEvent> + Send>> {
        let events_stream = self.events();
        Box::pin(stream! {
            pin_mut!(events_stream);
            while let Some(event) = events_stream.next().await {
                match event {
                    AudioEvent::StreamAdded(_) |
                    AudioEvent::StreamRemoved(_) |
                    AudioEvent::StreamVolumeChanged { .. } |
                    AudioEvent::StreamMuteChanged { .. } |
                    AudioEvent::StreamChanged(_) |
                    AudioEvent::StreamMoved { .. } => {
                        yield event;
                    }
                    _ => {}
                }
            }
        })
    }

    fn default_input(&self) -> Pin<Box<dyn Stream<Item = DeviceInfo> + Send>> {
        let default_input = std::sync::Arc::clone(&self.default_input);
        let mut events_rx = self.events_tx.subscribe();

        Box::pin(stream! {
            {
                let device_opt = if let Ok(default_guard) = default_input.read() {
                    default_guard.as_ref().cloned()
                } else {
                    None
                };
                if let Some(device) = device_opt {
                    yield device;
                }
            }

            while let Ok(AudioEvent::DefaultInputChanged(device_info)) = events_rx.recv().await {
                yield device_info;
            }
        })
    }

    fn default_output(&self) -> Pin<Box<dyn Stream<Item = DeviceInfo> + Send>> {
        let default_output = std::sync::Arc::clone(&self.default_output);
        let mut events_rx = self.events_tx.subscribe();

        Box::pin(stream! {
            {
                let device_opt = if let Ok(default_guard) = default_output.read() {
                    default_guard.as_ref().cloned()
                } else {
                    None
                };
                if let Some(device) = device_opt {
                    yield device;
                }
            }

            while let Ok(AudioEvent::DefaultOutputChanged(device_info)) = events_rx.recv().await {
                yield device_info;
            }
        })
    }

    fn device_volume(&self, device: DeviceIndex) -> Pin<Box<dyn Stream<Item = Volume> + Send>> {
        let events_stream = self.events();
        Box::pin(stream! {
            pin_mut!(events_stream);
            while let Some(event) = events_stream.next().await {
                if let AudioEvent::DeviceVolumeChanged { device_index, volume, .. } = event {
                    if device_index == device {
                        yield volume;
                    }
                }
            }
        })
    }

    fn device_mute(&self, device: DeviceIndex) -> Pin<Box<dyn Stream<Item = bool> + Send>> {
        let events_stream = self.events();
        Box::pin(stream! {
            pin_mut!(events_stream);
            while let Some(event) = events_stream.next().await {
                if let AudioEvent::DeviceMuteChanged { device_index, muted, .. } = event {
                    if device_index == device {
                        yield muted;
                    }
                }
            }
        })
    }

    fn device_state(&self, device: DeviceIndex) -> Pin<Box<dyn Stream<Item = DeviceInfo> + Send>> {
        let events_stream = self.events();
        Box::pin(stream! {
            pin_mut!(events_stream);
            while let Some(event) = events_stream.next().await {
                match event {
                    AudioEvent::DeviceChanged(device_info) if device_info.index == device => {
                        yield device_info;
                    }
                    _ => {}
                }
            }
        })
    }

    fn stream_volume(&self, stream: StreamIndex) -> Pin<Box<dyn Stream<Item = Volume> + Send>> {
        let events_stream = self.events();
        Box::pin(stream! {
            pin_mut!(events_stream);
            while let Some(event) = events_stream.next().await {
                if let AudioEvent::StreamVolumeChanged { stream_index, volume, .. } = event {
                    if stream_index == stream {
                        yield volume;
                    }
                }
            }
        })
    }

    fn stream_mute(&self, stream: StreamIndex) -> Pin<Box<dyn Stream<Item = bool> + Send>> {
        let events_stream = self.events();
        Box::pin(stream! {
            pin_mut!(events_stream);
            while let Some(event) = events_stream.next().await {
                if let AudioEvent::StreamMuteChanged { stream_index, muted, .. } = event {
                    if stream_index == stream {
                        yield muted;
                    }
                }
            }
        })
    }

    fn stream_state(&self, stream: StreamIndex) -> Pin<Box<dyn Stream<Item = StreamInfo> + Send>> {
        let events_stream = self.events();
        Box::pin(stream! {
            pin_mut!(events_stream);
            while let Some(event) = events_stream.next().await {
                match event {
                    AudioEvent::StreamChanged(stream_info) if stream_info.index == stream => {
                        yield stream_info;
                    }
                    _ => {}
                }
            }
        })
    }

    async fn device(&self, device: DeviceIndex) -> Result<DeviceInfo, Self::Error> {
        let devices = self
            .devices
            .read()
            .map_err(|_| PulseError::ThreadCommunication)?;
        devices
            .values()
            .find(|d| d.index == device)
            .cloned()
            .ok_or(PulseError::DeviceNotFound(device))
    }

    async fn stream(&self, stream: StreamIndex) -> Result<StreamInfo, Self::Error> {
        let streams = self
            .streams
            .read()
            .map_err(|_| PulseError::ThreadCommunication)?;
        streams
            .get(&stream)
            .cloned()
            .ok_or(PulseError::StreamNotFound(stream))
    }

    async fn devices_by_type(
        &self,
        device_type: DeviceType,
    ) -> Result<Vec<DeviceInfo>, Self::Error> {
        let devices = self
            .devices
            .read()
            .map_err(|_| PulseError::ThreadCommunication)?;
        let filtered_devices: Vec<DeviceInfo> = devices
            .values()
            .filter(|d| d.device_type == device_type)
            .cloned()
            .collect();
        Ok(filtered_devices)
    }

    async fn current_default_input(&self) -> Result<Option<DeviceInfo>, Self::Error> {
        let default_input = self
            .default_input
            .read()
            .map_err(|_| PulseError::ThreadCommunication)?;
        Ok(default_input.clone())
    }

    async fn current_default_output(&self) -> Result<Option<DeviceInfo>, Self::Error> {
        let default_output = self
            .default_output
            .read()
            .map_err(|_| PulseError::ThreadCommunication)?;
        Ok(default_output.clone())
    }

    async fn set_device_volume(
        &self,
        device: DeviceIndex,
        volume: Volume,
    ) -> Result<(), Self::Error> {
        let pulse_volume = PulseImplementation::convert_volume_to_pulse(&volume)?;
        let _ = self.command_tx.send(PulseCommand::SetDeviceVolume {
            device,
            volume: pulse_volume,
        });
        Ok(())
    }

    async fn set_device_mute(&self, device: DeviceIndex, muted: bool) -> Result<(), Self::Error> {
        let _ = self
            .command_tx
            .send(PulseCommand::SetDeviceMute { device, muted });
        Ok(())
    }

    async fn set_stream_volume(
        &self,
        stream: StreamIndex,
        volume: Volume,
    ) -> Result<(), Self::Error> {
        let pulse_volume = PulseImplementation::convert_volume_to_pulse(&volume)?;
        let _ = self.command_tx.send(PulseCommand::SetStreamVolume {
            stream,
            volume: pulse_volume,
        });
        Ok(())
    }

    async fn set_stream_mute(&self, stream: StreamIndex, muted: bool) -> Result<(), Self::Error> {
        let _ = self
            .command_tx
            .send(PulseCommand::SetStreamMute { stream, muted });
        Ok(())
    }

    async fn set_default_input(&self, device: DeviceIndex) -> Result<(), Self::Error> {
        let _ = self
            .command_tx
            .send(PulseCommand::SetDefaultInput { device });
        Ok(())
    }

    async fn set_default_output(&self, device: DeviceIndex) -> Result<(), Self::Error> {
        let _ = self
            .command_tx
            .send(PulseCommand::SetDefaultOutput { device });
        Ok(())
    }

    async fn move_stream(
        &self,
        stream: StreamIndex,
        device: DeviceIndex,
    ) -> Result<(), Self::Error> {
        let _ = self
            .command_tx
            .send(PulseCommand::MoveStream { stream, device });
        Ok(())
    }
}
