use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use async_stream::stream;
use async_trait::async_trait;
use futures::Stream;
use libpulse_binding::{
    callbacks::ListResult,
    context::{
        Context, FlagSet as ContextFlags,
        subscribe::{Facility, InterestMaskSet, Operation},
    },
    def::PortAvailable,
    volume::ChannelVolumes,
};
use tokio::sync::{broadcast, mpsc};

use super::{
    AudioEvent, AudioService, DeviceIndex, DeviceInfo, DeviceKey, DevicePort, DeviceState, DeviceType,
    SampleFormat, StreamFormat, StreamIndex, StreamInfo, StreamState, StreamType, Volume,
    VolumeError, tokio_mainloop::TokioMain,
};

/// PulseAudio service implementation
pub struct PulseAudioService {
    command_tx: mpsc::UnboundedSender<PulseCommand>,

    device_list_tx: Arc<broadcast::Sender<Vec<DeviceInfo>>>,
    stream_list_tx: Arc<broadcast::Sender<Vec<StreamInfo>>>,
    events_tx: Arc<broadcast::Sender<AudioEvent>>,

    devices: Arc<RwLock<HashMap<DeviceKey, DeviceInfo>>>,
    streams: Arc<RwLock<HashMap<StreamIndex, StreamInfo>>>,
    default_input: Arc<RwLock<Option<DeviceInfo>>>,
    default_output: Arc<RwLock<Option<DeviceInfo>>>,

    monitoring_handle: Option<tokio::task::JoinHandle<()>>,
}

#[derive(Debug)]
enum PulseCommand {
    TriggerDeviceDiscovery,
    TriggerStreamDiscovery,
}

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
            monitoring_handle: None,
        }
    }
}

impl PulseAudioService {
    /// Creates a new PulseAudio service instance
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

        let monitoring_handle = Self::spawn_monitoring_task(
            command_rx,
            device_list_tx.clone(),
            stream_list_tx.clone(),
            events_tx.clone(),
            devices.clone(),
            streams.clone(),
            default_input.clone(),
            default_output.clone(),
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
            monitoring_handle: Some(monitoring_handle),
        })
    }

    async fn spawn_monitoring_task(
        mut command_rx: mpsc::UnboundedReceiver<PulseCommand>,
        device_list_tx: broadcast::Sender<Vec<DeviceInfo>>,
        stream_list_tx: broadcast::Sender<Vec<StreamInfo>>,
        events_tx: broadcast::Sender<AudioEvent>,
        devices: Arc<RwLock<HashMap<DeviceKey, DeviceInfo>>>,
        streams: Arc<RwLock<HashMap<StreamIndex, StreamInfo>>>,
        default_input: Arc<RwLock<Option<DeviceInfo>>>,
        default_output: Arc<RwLock<Option<DeviceInfo>>>,
    ) -> Result<tokio::task::JoinHandle<()>, PulseError> {
        let handle = tokio::task::spawn_blocking(move || {
            let result: Result<(), PulseError> = (|| {
                let rt = tokio::runtime::Runtime::new().map_err(|e| {
                    PulseError::ConnectionFailed(format!("Failed to create runtime: {e}"))
                })?;
                rt.block_on(async {
                let mut mainloop = TokioMain::new();
                let mut context = Context::new(&mainloop, "wayle-audio").ok_or_else(|| {
                    PulseError::ConnectionFailed("Failed to create context".to_string())
                })?;
                
                context
                    .connect(None, ContextFlags::NOFLAGS, None)
                    .map_err(|e| PulseError::ConnectionFailed(format!("Connection failed: {e}")))?;
                
                mainloop.wait_for_ready(&context).await.map_err(|e| {
                    PulseError::ConnectionFailed(format!("Context failed to become ready: {e:?}"))
                })?;
                
                Self::setup_event_subscription(
                    &mut context,
                    &device_list_tx,
                    &stream_list_tx,
                    &events_tx,
                    &devices,
                    &streams,
                    &default_input,
                    &default_output,
                )?;
                
                Self::trigger_device_discovery(&context, &devices, &device_list_tx);
                Self::trigger_stream_discovery(&context, &streams, &stream_list_tx);
                
                tokio::select! {
                    _ = mainloop.run() => {
                    }
                    _ = async {
                        while let Some(command) = command_rx.recv().await {
                            Self::handle_command(&context, command, &devices, &streams, &device_list_tx, &stream_list_tx);
                        }
                    } => {
                    }
                }
                
                Ok(())
            })
            })();
            
            if let Err(_e) = result {
                // Error handling - task continues
            }
        });

        Ok(handle)
    }

    fn setup_event_subscription(
        context: &mut Context,
        device_list_tx: &broadcast::Sender<Vec<DeviceInfo>>,
        stream_list_tx: &broadcast::Sender<Vec<StreamInfo>>,
        events_tx: &broadcast::Sender<AudioEvent>,
        devices: &Arc<RwLock<HashMap<DeviceKey, DeviceInfo>>>,
        streams: &Arc<RwLock<HashMap<StreamIndex, StreamInfo>>>,
        _default_input: &Arc<RwLock<Option<DeviceInfo>>>,
        _default_output: &Arc<RwLock<Option<DeviceInfo>>>,
    ) -> Result<(), PulseError> {
        let interest_mask = InterestMaskSet::SINK
            | InterestMaskSet::SOURCE
            | InterestMaskSet::SINK_INPUT
            | InterestMaskSet::SOURCE_OUTPUT
            | InterestMaskSet::SERVER;

        let devices_clone = Arc::clone(devices);
        let streams_clone = Arc::clone(streams);
        let device_list_tx_clone = device_list_tx.clone();
        let stream_list_tx_clone = stream_list_tx.clone();
        let events_tx_clone = events_tx.clone();

        context.set_subscribe_callback(Some(Box::new(move |facility, operation, index| {
            match facility {
                Some(Facility::Sink) | Some(Facility::Source) => match operation {
                    Some(Operation::Removed) => {
                        let device_index = DeviceIndex(index);
                        let device_type = match facility {
                            Some(Facility::Sink) => DeviceType::Output,
                            Some(Facility::Source) => DeviceType::Input,
                            _ => unreachable!(),
                        };
                        let device_key = DeviceKey::new(index, device_type);
                        if let Ok(mut devices_guard) = devices_clone.write() {
                            devices_guard.remove(&device_key);
                        }
                        let _ = events_tx_clone.send(AudioEvent::DeviceRemoved(device_index));
                        Self::broadcast_device_list(&device_list_tx_clone, &devices_clone);
                    }
                    Some(Operation::New) | Some(Operation::Changed) => {
                        Self::broadcast_device_list(&device_list_tx_clone, &devices_clone);
                    }
                    _ => {}
                },
                Some(Facility::SinkInput) | Some(Facility::SourceOutput) => match operation {
                    Some(Operation::Removed) => {
                        let stream_index = StreamIndex(index);
                        if let Ok(mut streams_guard) = streams_clone.write() {
                            streams_guard.remove(&stream_index);
                        }
                        let _ = events_tx_clone.send(AudioEvent::StreamRemoved(stream_index));
                        Self::broadcast_stream_list(&stream_list_tx_clone, &streams_clone);
                    }
                    Some(Operation::New) | Some(Operation::Changed) => {
                        Self::broadcast_stream_list(&stream_list_tx_clone, &streams_clone);
                    }
                    _ => {}
                },
                Some(Facility::Server) => {}
                _ => {}
            }
        })));

        context.subscribe(interest_mask, |_success: bool| {
            // Event subscription callback - no action needed
        });

        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    fn trigger_device_discovery(
        context: &Context,
        devices: &Arc<RwLock<HashMap<DeviceKey, DeviceInfo>>>,
        device_list_tx: &broadcast::Sender<Vec<DeviceInfo>>,
    ) {
        let devices_clone = Arc::clone(devices);
        let device_list_tx_clone = device_list_tx.clone();
        let introspect = context.introspect();

        introspect.get_sink_info_list(move |result| {
            match result {
                ListResult::Item(sink_info) => {
                    let device_info = DeviceInfo::new(
                        sink_info.index,
                        DeviceType::Output,
                        super::DeviceName::new(
                            sink_info
                                .name
                                .as_ref()
                                .map(|s| s.to_string())
                                .unwrap_or_default(),
                        ),
                        sink_info
                            .description
                            .as_ref()
                            .map(|s| s.to_string())
                            .unwrap_or_default(),
                        DeviceState::Running,
                        false,
                        sink_info
                            .ports
                            .iter()
                            .map(|port| DevicePort {
                                name: port
                                    .name
                                    .as_ref()
                                    .map(|s| s.to_string())
                                    .unwrap_or_default(),
                                description: port
                                    .description
                                    .as_ref()
                                    .map(|s| s.to_string())
                                    .unwrap_or_default(),
                                priority: port.priority,
                                available: port.available == PortAvailable::Yes,
                            })
                            .collect(),
                        sink_info
                            .active_port
                            .as_ref()
                            .and_then(|p| p.name.as_ref().map(|s| s.to_string())),
                    );

                    if let Ok(mut devices_guard) = devices_clone.write() {
                        devices_guard.insert(device_info.key, device_info);
                    }
                }
                ListResult::End => {
                    Self::broadcast_device_list(&device_list_tx_clone, &devices_clone);
                }
                ListResult::Error => {
                    // Device discovery failed - continue without error
                }
            }
        });

        let devices_clone = Arc::clone(devices);
        let device_list_tx_clone = device_list_tx.clone();

        introspect.get_source_info_list(move |result| {
            match result {
                ListResult::Item(source_info) => {
                    let device_info = DeviceInfo::new(
                        source_info.index,
                        DeviceType::Input,
                        super::DeviceName::new(
                            source_info
                                .name
                                .as_ref()
                                .map(|s| s.to_string())
                                .unwrap_or_default(),
                        ),
                        source_info
                            .description
                            .as_ref()
                            .map(|s| s.to_string())
                            .unwrap_or_default(),
                        DeviceState::Running,
                        false,
                        source_info
                            .ports
                            .iter()
                            .map(|port| DevicePort {
                                name: port
                                    .name
                                    .as_ref()
                                    .map(|s| s.to_string())
                                    .unwrap_or_default(),
                                description: port
                                    .description
                                    .as_ref()
                                    .map(|s| s.to_string())
                                    .unwrap_or_default(),
                                priority: port.priority,
                                available: port.available == PortAvailable::Yes,
                            })
                            .collect(),
                        source_info
                            .active_port
                            .as_ref()
                            .and_then(|p| p.name.as_ref().map(|s| s.to_string())),
                    );

                    if let Ok(mut devices_guard) = devices_clone.write() {
                        devices_guard.insert(device_info.key, device_info);
                    }
                }
                ListResult::End => {
                    Self::broadcast_device_list(&device_list_tx_clone, &devices_clone);
                }
                ListResult::Error => {
                    // Device discovery failed - continue without error
                }
            }
        });
    }

    fn trigger_stream_discovery(
        context: &Context,
        streams: &Arc<RwLock<HashMap<StreamIndex, StreamInfo>>>,
        stream_list_tx: &broadcast::Sender<Vec<StreamInfo>>,
    ) {
        let streams_clone = Arc::clone(streams);
        let stream_list_tx_clone = stream_list_tx.clone();
        let introspect = context.introspect();

        introspect.get_sink_input_info_list(move |result| match result {
            ListResult::Item(sink_input_info) => {
                if let Ok(volume) = Self::convert_volume_from_pulse(&sink_input_info.volume) {
                    let stream_info = StreamInfo {
                        index: StreamIndex(sink_input_info.index),
                        name: sink_input_info.name.clone().unwrap_or_default().to_string(),
                        application_name: sink_input_info
                            .proplist
                            .get_str("application.name")
                            .unwrap_or_default(),
                        stream_type: StreamType::Playback,
                        state: StreamState::Running,
                        device_index: DeviceIndex(sink_input_info.sink),
                        volume,
                        muted: sink_input_info.mute,
                        format: StreamFormat {
                            sample_rate: sink_input_info.sample_spec.rate,
                            channels: sink_input_info.sample_spec.channels,
                            sample_format: Self::convert_sample_format(
                                sink_input_info.sample_spec.format,
                            ),
                        },
                    };

                    if let Ok(mut streams_guard) = streams_clone.write() {
                        streams_guard.insert(stream_info.index, stream_info);
                    }
                }
            }
            ListResult::End => {
                Self::broadcast_stream_list(&stream_list_tx_clone, &streams_clone);
            }
            ListResult::Error => {
                // Stream discovery failed - continue without error
            }
        });
    }

    fn broadcast_device_list(
        device_list_tx: &broadcast::Sender<Vec<DeviceInfo>>,
        devices: &Arc<RwLock<HashMap<DeviceKey, DeviceInfo>>>,
    ) {
        if let Ok(devices_guard) = devices.read() {
            let device_list: Vec<DeviceInfo> = devices_guard.values().cloned().collect();
            let _ = device_list_tx.send(device_list);
        }
    }

    fn broadcast_stream_list(
        stream_list_tx: &broadcast::Sender<Vec<StreamInfo>>,
        streams: &Arc<RwLock<HashMap<StreamIndex, StreamInfo>>>,
    ) {
        if let Ok(streams_guard) = streams.read() {
            let stream_list: Vec<StreamInfo> = streams_guard.values().cloned().collect();
            let _ = stream_list_tx.send(stream_list);
        }
    }

    fn handle_command(
        context: &Context,
        command: PulseCommand,
        devices: &Arc<RwLock<HashMap<DeviceKey, DeviceInfo>>>,
        streams: &Arc<RwLock<HashMap<StreamIndex, StreamInfo>>>,
        device_list_tx: &broadcast::Sender<Vec<DeviceInfo>>,
        stream_list_tx: &broadcast::Sender<Vec<StreamInfo>>,
    ) {
        match command {
            PulseCommand::TriggerDeviceDiscovery => {
                Self::trigger_device_discovery(context, devices, device_list_tx);
            }
            PulseCommand::TriggerStreamDiscovery => {
                Self::trigger_stream_discovery(context, streams, stream_list_tx);
            }
        }
    }

    fn convert_sample_format(format: libpulse_binding::sample::Format) -> SampleFormat {
        match format {
            libpulse_binding::sample::Format::U8 => SampleFormat::U8,
            libpulse_binding::sample::Format::S16le => SampleFormat::S16LE,
            libpulse_binding::sample::Format::S24le => SampleFormat::S24LE,
            libpulse_binding::sample::Format::S32le => SampleFormat::S32LE,
            libpulse_binding::sample::Format::F32le => SampleFormat::F32LE,
            _ => SampleFormat::Unknown,
        }
    }

    fn convert_volume_to_pulse(volume: &Volume) -> Result<ChannelVolumes, VolumeError> {
        if volume.channels() == 0 {
            return Err(VolumeError::InvalidChannel { channel: 0 });
        }

        let mut pulse_volume = ChannelVolumes::default();
        pulse_volume.set_len(volume.channels() as u8);

        for (i, &vol) in volume.as_slice().iter().enumerate() {
            if !(0.0..=10.0).contains(&vol) {
                return Err(VolumeError::InvalidVolume { 
                    channel: i, 
                    volume: vol 
                });
            }

            let pulse_vol = (vol * libpulse_binding::volume::Volume::NORMAL.0 as f64 / 10.0) as u32;
            pulse_volume.set(i as u8, libpulse_binding::volume::Volume(pulse_vol));
        }

        Ok(pulse_volume)
    }

    fn convert_volume_from_pulse(pulse_volume: &ChannelVolumes) -> Result<Volume, VolumeError> {
        let volumes: Vec<f64> = (0..pulse_volume.len())
            .map(|i| {
                let pulse_vol = pulse_volume.get()[i as usize].0 as f64;
                pulse_vol * 10.0 / libpulse_binding::volume::Volume::NORMAL.0 as f64
            })
            .collect();

        Volume::new(volumes)
    }
}

#[async_trait]
impl AudioService for PulseAudioService {
    type Error = PulseError;

    fn devices(&self) -> std::pin::Pin<Box<dyn Stream<Item = Vec<DeviceInfo>> + Send>> {
        let devices = Arc::clone(&self.devices);
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

    fn streams(&self) -> std::pin::Pin<Box<dyn Stream<Item = Vec<StreamInfo>> + Send>> {
        let streams = Arc::clone(&self.streams);
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

    fn events(&self) -> std::pin::Pin<Box<dyn Stream<Item = AudioEvent> + Send>> {
        let mut events_rx = self.events_tx.subscribe();
        Box::pin(stream! {
            while let Ok(event) = events_rx.recv().await {
                yield event;
            }
        })
    }

    fn default_input(&self) -> std::pin::Pin<Box<dyn Stream<Item = DeviceInfo> + Send>> {
        let default_input = Arc::clone(&self.default_input);
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

    fn default_output(&self) -> std::pin::Pin<Box<dyn Stream<Item = DeviceInfo> + Send>> {
        let default_output = Arc::clone(&self.default_output);
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
}

impl Drop for PulseAudioService {
    fn drop(&mut self) {
        if let Some(handle) = self.monitoring_handle.take() {
            handle.abort();
        }
    }
}
