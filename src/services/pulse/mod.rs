use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use async_trait::async_trait;
use device::DeviceKey;
use stream::StreamKey;
use tokio::sync::{broadcast, mpsc};

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
/// Stream management domain
pub mod stream;
/// Tokio mainloop for PulseAudio
pub mod tokio_mainloop;
/// Volume control domain
pub mod volume;

pub use device::{
    DeviceIndex, DeviceInfo, DeviceManager, DeviceStreams, DeviceType, DeviceVolumeController,
};
pub use error::PulseError;
pub use events::AudioEvent;
pub use stream::{
    StreamIndex, StreamInfo, StreamManager, StreamStreams, StreamType, StreamVolumeController,
};
pub use volume::{Volume, VolumeError};

use backend::{
    CommandSender, DefaultDevice, DeviceListSender, DeviceStore, EventSender, ExternalCommand,
    PulseBackend, ServerInfo, StreamListSender, StreamStore,
};

/// PulseAudio service implementation
///
/// Provides device and stream management through PulseAudio backend.
pub struct PulseService {
    command_tx: CommandSender,

    device_list_tx: DeviceListSender,
    stream_list_tx: StreamListSender,
    events_tx: EventSender,

    devices: DeviceStore,
    streams: StreamStore,
    default_input: DefaultDevice,
    default_output: DefaultDevice,
    server_info: ServerInfo,

    monitoring_handle: Option<tokio::task::JoinHandle<()>>,
}

impl Clone for PulseService {
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
            Arc::clone(&devices),
            Arc::clone(&streams),
            Arc::clone(&default_input),
            Arc::clone(&default_output),
            Arc::clone(&server_info),
        )
        .await?;

        Ok(PulseService {
            command_tx,
            device_list_tx,
            stream_list_tx,
            events_tx,
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
        let _ = self.command_tx.send(ExternalCommand::Shutdown);

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

#[async_trait]
impl DeviceManager for PulseService {
    type Error = PulseError;

    async fn device(&self, device: DeviceKey) -> Result<DeviceInfo, Self::Error> {
        let devices = self
            .devices
            .read()
            .map_err(|e| PulseError::LockPoisoned(format!("shared data lock: {e}")))?;
        devices
            .values()
            .find(|d| d.index.0 == device.index && d.device_type == device.device_type)
            .cloned()
            .ok_or(PulseError::DeviceNotFound(device.index, device.device_type))
    }

    async fn devices_by_type(
        &self,
        device_type: DeviceType,
    ) -> Result<Vec<DeviceInfo>, Self::Error> {
        let devices = self
            .devices
            .read()
            .map_err(|e| PulseError::LockPoisoned(format!("shared data lock: {e}")))?;
        let filtered_devices: Vec<DeviceInfo> = devices
            .values()
            .filter(|d| d.device_type == device_type)
            .cloned()
            .collect();
        Ok(filtered_devices)
    }

    async fn default_input(&self) -> Result<Option<DeviceInfo>, Self::Error> {
        let input = self
            .default_input
            .read()
            .map_err(|e| PulseError::LockPoisoned(format!("shared data lock: {e}")))?;
        Ok(input.clone())
    }

    async fn default_output(&self) -> Result<Option<DeviceInfo>, Self::Error> {
        let output = self
            .default_output
            .read()
            .map_err(|e| PulseError::LockPoisoned(format!("shared data lock: {e}")))?;
        Ok(output.clone())
    }

    async fn set_default_input(&self, device_key: DeviceKey) -> Result<(), Self::Error> {
        self.command_tx
            .send(ExternalCommand::SetDefaultInput { device_key })
            .map_err(|e| PulseError::LockPoisoned(format!("shared data lock: {e}")))?;
        Ok(())
    }

    async fn set_default_output(&self, device_key: DeviceKey) -> Result<(), Self::Error> {
        self.command_tx
            .send(ExternalCommand::SetDefaultOutput { device_key })
            .map_err(|e| PulseError::LockPoisoned(format!("shared data lock: {e}")))?;
        Ok(())
    }
}

#[async_trait]
impl DeviceVolumeController for PulseService {
    type Error = PulseError;

    async fn set_device_volume(
        &self,
        device_key: DeviceKey,
        level: f64,
    ) -> Result<(), Self::Error> {
        println!(
            "Setting volume for device '{}-{:?}': {}",
            device_key.index, device_key.device_type, level
        );
        let device_info = self.device(device_key).await?;
        let channel_count = device_info.volume.channels();
        let volume = Volume::new(vec![level; channel_count]);
        let pulse_volume = PulseBackend::convert_volume_to_pulse(&volume);
        self.command_tx
            .send(ExternalCommand::SetDeviceVolume {
                device_key,
                volume: pulse_volume,
            })
            .map_err(|e| PulseError::LockPoisoned(format!("shared data lock: {e}")))?;
        Ok(())
    }

    async fn set_device_mute(&self, device_key: DeviceKey, muted: bool) -> Result<(), Self::Error> {
        self.command_tx
            .send(ExternalCommand::SetDeviceMute { device_key, muted })
            .map_err(|e| PulseError::LockPoisoned(format!("shared data lock: {e}")))?;
        Ok(())
    }
}

#[async_trait]
impl StreamManager for PulseService {
    type Error = PulseError;

    async fn stream(&self, stream_key: StreamKey) -> Result<stream::StreamInfo, Self::Error> {
        let streams = self
            .streams
            .read()
            .map_err(|e| PulseError::LockPoisoned(format!("shared data lock: {e}")))?;
        streams
            .get(&stream_key)
            .cloned()
            .ok_or(PulseError::StreamNotFound(
                stream_key.index,
                stream_key.stream_type,
            ))
    }

    async fn move_stream(
        &self,
        stream_key: StreamKey,
        device_key: DeviceKey,
    ) -> Result<(), Self::Error> {
        self.command_tx
            .send(ExternalCommand::MoveStream {
                stream_key,
                device_key,
            })
            .map_err(|e| PulseError::LockPoisoned(format!("shared data lock: {e}")))?;
        Ok(())
    }
}

#[async_trait]
impl StreamVolumeController for PulseService {
    type Error = PulseError;

    async fn set_stream_volume(
        &self,
        stream_key: StreamKey,
        volume: Volume,
    ) -> Result<(), Self::Error> {
        let pulse_volume = PulseBackend::convert_volume_to_pulse(&volume);
        self.command_tx
            .send(ExternalCommand::SetStreamVolume {
                stream_key,
                volume: pulse_volume,
            })
            .map_err(|e| PulseError::LockPoisoned(format!("shared data lock: {e}")))?;
        Ok(())
    }

    async fn set_stream_mute(&self, stream_key: StreamKey, muted: bool) -> Result<(), Self::Error> {
        self.command_tx
            .send(ExternalCommand::SetStreamMute { stream_key, muted })
            .map_err(|e| PulseError::LockPoisoned(format!("shared data lock: {e}")))?;
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
