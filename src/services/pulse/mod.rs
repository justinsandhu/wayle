use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use async_trait::async_trait;
use tokio::sync::{broadcast, mpsc};

/// PulseAudio backend implementation
pub mod backend;
/// Device management domain
pub mod device;
/// Error types
pub mod error;
/// Event types and handling
pub mod events;
/// Stream management domain
pub mod stream;
/// Tokio mainloop for PulseAudio
pub mod tokio_mainloop;
/// Volume control domain
pub mod volume;

// Clean public API - only export what users need
pub use device::{
    DeviceIndex, DeviceInfo, DeviceManager, DeviceStreams, DeviceType, DeviceVolumeController,
};
pub use error::PulseError;
pub use events::AudioEvent;
pub use stream::{
    StreamIndex, StreamInfo, StreamManager, StreamStreams, StreamType, StreamVolumeController,
};
pub use volume::{Volume, VolumeError};

use backend::{PulseBackend, PulseCommand};
use device::DeviceKey;

/// PulseAudio service implementation
///
/// Provides device and stream management through PulseAudio backend.
pub struct PulseService {
    command_tx: mpsc::UnboundedSender<PulseCommand>,

    device_list_tx: Arc<broadcast::Sender<Vec<DeviceInfo>>>,
    stream_list_tx: Arc<broadcast::Sender<Vec<stream::StreamInfo>>>,
    events_tx: Arc<broadcast::Sender<AudioEvent>>,

    devices: Arc<RwLock<HashMap<DeviceKey, DeviceInfo>>>,
    streams: Arc<RwLock<HashMap<StreamIndex, stream::StreamInfo>>>,
    default_input: Arc<RwLock<Option<DeviceInfo>>>,
    default_output: Arc<RwLock<Option<DeviceInfo>>>,
    server_info: Arc<RwLock<Option<String>>>,

    monitoring_handle: Option<tokio::task::JoinHandle<()>>,
}

impl Clone for PulseService {
    fn clone(&self) -> Self {
        Self {
            command_tx: self.command_tx.clone(),
            device_list_tx: self.device_list_tx.clone(),
            stream_list_tx: self.stream_list_tx.clone(),
            events_tx: self.events_tx.clone(),
            devices: self.devices.clone(),
            streams: self.streams.clone(),
            default_input: self.default_input.clone(),
            default_output: self.default_output.clone(),
            server_info: self.server_info.clone(),
            monitoring_handle: None,
        }
    }
}

impl PulseService {
    /// Create a new PulseAudio service with default settings
    ///
    /// # Errors
    /// Returns error if PulseAudio connection fails or service initialization fails
    pub async fn new() -> Result<Self, PulseError> {
        const DEVICE_BUFFER_SIZE: usize = 100;
        const STREAM_BUFFER_SIZE: usize = 100;
        const EVENTS_BUFFER_SIZE: usize = 100;

        let (command_tx, command_rx) = mpsc::unbounded_channel();

        let (device_list_tx, _) = broadcast::channel(DEVICE_BUFFER_SIZE);
        let (stream_list_tx, _) = broadcast::channel(STREAM_BUFFER_SIZE);
        let (events_tx, _) = broadcast::channel(EVENTS_BUFFER_SIZE);

        let devices = Arc::new(RwLock::new(HashMap::new()));
        let streams = Arc::new(RwLock::new(HashMap::new()));
        let default_input = Arc::new(RwLock::new(None));
        let default_output = Arc::new(RwLock::new(None));
        let server_info = Arc::new(RwLock::new(None));

        let monitoring_handle = PulseBackend::spawn_monitoring_task(
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

        Ok(PulseService {
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

    /// Gracefully shutdown the service
    ///
    /// Stops background monitoring and cleans up resources.
    ///
    /// # Errors
    /// Returns error if shutdown operations fail
    pub async fn shutdown(mut self) -> Result<(), PulseError> {
        let _ = self.command_tx.send(PulseCommand::Shutdown);

        if let Some(handle) = self.monitoring_handle.take() {
            let _ = handle.await;
        }

        Ok(())
    }
}

impl Drop for PulseService {
    fn drop(&mut self) {
        if let Some(handle) = self.monitoring_handle.take() {
            handle.abort();
        }
    }
}

// Implement the focused domain traits
#[async_trait]
impl DeviceManager for PulseService {
    type Error = PulseError;

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
}

#[async_trait]
impl DeviceVolumeController for PulseService {
    type Error = PulseError;

    async fn set_device_volume(
        &self,
        device: DeviceIndex,
        volume: Volume,
    ) -> Result<(), Self::Error> {
        let pulse_volume = PulseBackend::convert_volume_to_pulse(&volume)?;
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
}

#[async_trait]
impl StreamManager for PulseService {
    type Error = PulseError;

    async fn stream(&self, stream: StreamIndex) -> Result<stream::StreamInfo, Self::Error> {
        let streams = self
            .streams
            .read()
            .map_err(|_| PulseError::ThreadCommunication)?;
        streams
            .get(&stream)
            .cloned()
            .ok_or(PulseError::StreamNotFound(stream))
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

#[async_trait]
impl StreamVolumeController for PulseService {
    type Error = PulseError;

    async fn set_stream_volume(
        &self,
        stream: StreamIndex,
        volume: Volume,
    ) -> Result<(), Self::Error> {
        let pulse_volume = PulseBackend::convert_volume_to_pulse(&volume)?;
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
}

impl PulseService {
    /// Stream of all audio events
    pub fn events(&self) -> impl futures::Stream<Item = AudioEvent> + Send {
        use async_stream::stream;

        let mut events_rx = self.events_tx.subscribe();
        stream! {
            while let Ok(event) = events_rx.recv().await {
                yield event;
            }
        }
    }
}
