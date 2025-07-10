use std::{
    collections::HashMap,
    sync::{Arc, RwLock as StdRwLock},
    time::Duration,
};

use async_stream::stream;
use futures::Stream;
use libpulse_binding::{
    callbacks::ListResult,
    context::{
        Context, FlagSet as ContextFlags, State as ContextState,
        subscribe::{Facility, InterestMaskSet, Operation},
    },
    def::PortAvailable,
    mainloop::threaded::Mainloop,
    volume::ChannelVolumes,
};
use tokio::sync::{broadcast, mpsc, oneshot};

use super::{
    AudioEvent, AudioService, DeviceIndex, DeviceInfo, DevicePort, DeviceState, DeviceType,
    SampleFormat, StreamFormat, StreamIndex, StreamInfo, StreamState, StreamType, Volume,
    VolumeError,
};

/// PulseAudio service implementation
pub struct PulseAudioService {
    command_tx: mpsc::UnboundedSender<PulseCommand>,
    event_rx: broadcast::Receiver<AudioEvent>,
    event_tx: broadcast::Sender<AudioEvent>,
    devices: Arc<StdRwLock<HashMap<DeviceIndex, DeviceInfo>>>,
    streams: Arc<StdRwLock<HashMap<StreamIndex, StreamInfo>>>,
    default_input: Arc<StdRwLock<Option<DeviceInfo>>>,
    default_output: Arc<StdRwLock<Option<DeviceInfo>>>,
    pulse_handle: Option<tokio::task::JoinHandle<()>>,
}

impl Clone for PulseAudioService {
    fn clone(&self) -> Self {
        Self {
            command_tx: self.command_tx.clone(),
            event_rx: self.event_tx.subscribe(),
            event_tx: self.event_tx.clone(),
            devices: Arc::clone(&self.devices),
            streams: Arc::clone(&self.streams),
            default_input: Arc::clone(&self.default_input),
            default_output: Arc::clone(&self.default_output),
            pulse_handle: None, // Clones don't own the background task
        }
    }
}

/// Commands sent to PulseAudio thread
#[derive(Debug)]
enum PulseCommand {
    SetDeviceVolume {
        device: DeviceIndex,
        volume: Volume,
        result_tx: oneshot::Sender<Result<(), PulseError>>,
    },
    SetDeviceMute {
        device: DeviceIndex,
        muted: bool,
        result_tx: oneshot::Sender<Result<(), PulseError>>,
    },
    SetDefaultDevice {
        device: DeviceIndex,
        result_tx: oneshot::Sender<Result<(), PulseError>>,
    },
    SetStreamVolume {
        stream: StreamIndex,
        volume: Volume,
        result_tx: oneshot::Sender<Result<(), PulseError>>,
    },
    SetStreamMute {
        stream: StreamIndex,
        muted: bool,
        result_tx: oneshot::Sender<Result<(), PulseError>>,
    },
    MoveStream {
        stream: StreamIndex,
        device: DeviceIndex,
        result_tx: oneshot::Sender<Result<(), PulseError>>,
    },
}

/// PulseAudio-specific error
#[derive(thiserror::Error, Debug)]
pub enum PulseError {
    /// Connection to PulseAudio failed
    #[error("PulseAudio connection failed: {0}")]
    ConnectionFailed(String),

    /// PulseAudio operation failed
    #[error("PulseAudio operation failed: {0}")]
    OperationFailed(String),

    /// Volume conversion error
    #[error("Volume conversion failed: {0}")]
    VolumeConversion(#[from] VolumeError),

    /// Device not found
    #[error("Device {0:?} not found")]
    DeviceNotFound(DeviceIndex),

    /// Stream not found
    #[error("Stream {0:?} not found")]
    StreamNotFound(StreamIndex),

    /// Communication with PulseAudio thread failed
    #[error("PulseAudio thread communication failed")]
    ThreadCommunication,
}

impl PulseAudioService {
    /// Create a new PulseAudio service
    pub async fn new() -> Result<Self, PulseError> {
        let (event_tx, event_rx) = broadcast::channel(100);
        let (command_tx, command_rx) = mpsc::unbounded_channel();

        let devices = Arc::new(StdRwLock::new(HashMap::new()));
        let streams = Arc::new(StdRwLock::new(HashMap::new()));
        let default_input = Arc::new(StdRwLock::new(None));
        let default_output = Arc::new(StdRwLock::new(None));

        let pulse_handle = Self::spawn_pulse_thread(
            command_rx,
            event_tx.clone(),
            devices.clone(),
            streams.clone(),
            default_input.clone(),
            default_output.clone(),
        )
        .await?;

        let service = Self {
            command_tx,
            event_rx,
            event_tx,
            devices: Arc::clone(&devices),
            streams: Arc::clone(&streams),
            default_input: Arc::clone(&default_input),
            default_output: Arc::clone(&default_output),
            pulse_handle: Some(pulse_handle),
        };

        Ok(service)
    }

    async fn spawn_pulse_thread(
        mut command_rx: mpsc::UnboundedReceiver<PulseCommand>,
        event_tx: broadcast::Sender<AudioEvent>,
        devices: Arc<StdRwLock<HashMap<DeviceIndex, DeviceInfo>>>,
        streams: Arc<StdRwLock<HashMap<StreamIndex, StreamInfo>>>,
        default_input: Arc<StdRwLock<Option<DeviceInfo>>>,
        default_output: Arc<StdRwLock<Option<DeviceInfo>>>,
    ) -> Result<tokio::task::JoinHandle<()>, PulseError> {
        let handle = tokio::task::spawn_blocking(move || {
            let result: Result<(), PulseError> = (|| {
                let mut mainloop = Mainloop::new().ok_or_else(|| {
                    PulseError::ConnectionFailed("Failed to create mainloop".to_string())
                })?;

                mainloop.start().map_err(|e| {
                    PulseError::ConnectionFailed(format!("Failed to start mainloop: {e}"))
                })?;

                let mut context = Context::new(&mainloop, "wayle-audio").ok_or_else(|| {
                    PulseError::ConnectionFailed("Failed to create context".to_string())
                })?;

                context
                    .connect(None, ContextFlags::NOFLAGS, None)
                    .map_err(|e| PulseError::ConnectionFailed(format!("Connection failed: {e}")))?;

                loop {
                    match context.get_state() {
                        ContextState::Ready => break,
                        ContextState::Failed | ContextState::Terminated => {
                            return Err(PulseError::ConnectionFailed("Context failed".to_string()));
                        }
                        _ => std::thread::sleep(Duration::from_millis(10)),
                    }
                }

                Self::setup_subscriptions(
                    &mut context,
                    &event_tx,
                    &devices,
                    &streams,
                    &default_input,
                    &default_output,
                )?;
                Self::initial_load(
                    &context,
                    &devices,
                    &streams,
                    &default_input,
                    &default_output,
                )?;

                loop {
                    mainloop.wait();

                    if let Ok(command) = command_rx.try_recv() {
                        Self::handle_command(&mut context, command);
                    }

                    mainloop.signal(false);
                }
            })();

            if let Err(e) = result {
                eprintln!("PulseAudio thread error: {}", e);
            }
        });

        Ok(handle)
    }

    fn setup_subscriptions(
        context: &mut Context,
        event_tx: &broadcast::Sender<AudioEvent>,
        devices: &Arc<StdRwLock<HashMap<DeviceIndex, DeviceInfo>>>,
        streams: &Arc<StdRwLock<HashMap<StreamIndex, StreamInfo>>>,
        _default_input: &Arc<StdRwLock<Option<DeviceInfo>>>,
        _default_output: &Arc<StdRwLock<Option<DeviceInfo>>>,
    ) -> Result<(), PulseError> {
        let interest_mask = InterestMaskSet::SINK
            | InterestMaskSet::SOURCE
            | InterestMaskSet::SINK_INPUT
            | InterestMaskSet::SOURCE_OUTPUT
            | InterestMaskSet::SERVER;

        let devices_clone = Arc::clone(devices);
        let streams_clone = Arc::clone(streams);
        let event_tx_clone = event_tx.clone();

        context.set_subscribe_callback(Some(Box::new(move |facility, operation, index| {
            match facility {
                Some(Facility::Sink) => {
                    if let Some(Operation::Removed) = operation {
                        if let Ok(mut devices_guard) = devices_clone.write() {
                            devices_guard.remove(&DeviceIndex(index));
                            let _ =
                                event_tx_clone.send(AudioEvent::DeviceRemoved(DeviceIndex(index)));
                        }
                    }
                }
                Some(Facility::Source) => {
                    if let Some(Operation::Removed) = operation {
                        if let Ok(mut devices_guard) = devices_clone.write() {
                            devices_guard.remove(&DeviceIndex(index));
                            let _ =
                                event_tx_clone.send(AudioEvent::DeviceRemoved(DeviceIndex(index)));
                        }
                    }
                }
                Some(Facility::SinkInput) => {
                    if let Some(Operation::Removed) = operation {
                        if let Ok(mut streams_guard) = streams_clone.write() {
                            streams_guard.remove(&StreamIndex(index));
                            let _ =
                                event_tx_clone.send(AudioEvent::StreamRemoved(StreamIndex(index)));
                        }
                    }
                }
                Some(Facility::SourceOutput) => {
                    if let Some(Operation::Removed) = operation {
                        if let Ok(mut streams_guard) = streams_clone.write() {
                            streams_guard.remove(&StreamIndex(index));
                            let _ =
                                event_tx_clone.send(AudioEvent::StreamRemoved(StreamIndex(index)));
                        }
                    }
                }
                Some(Facility::Server) => {
                    // Server changes require re-querying for defaults
                }
                _ => {}
            }
        })));

        context.subscribe(interest_mask, |success: bool| {
            if !success {
                eprintln!("Failed to subscribe to PulseAudio events");
            }
        });

        Ok(())
    }

    fn initial_load(
        context: &Context,
        devices: &Arc<StdRwLock<HashMap<DeviceIndex, DeviceInfo>>>,
        streams: &Arc<StdRwLock<HashMap<StreamIndex, StreamInfo>>>,
        default_input: &Arc<StdRwLock<Option<DeviceInfo>>>,
        default_output: &Arc<StdRwLock<Option<DeviceInfo>>>,
    ) -> Result<(), PulseError> {
        Self::load_devices(context, devices);
        Self::load_streams(context, streams);
        Self::load_defaults(context, default_input, default_output);
        Ok(())
    }

    fn load_devices(context: &Context, devices: &Arc<StdRwLock<HashMap<DeviceIndex, DeviceInfo>>>) {
        let devices_clone = Arc::clone(devices);
        let introspect = context.introspect();

        introspect.get_sink_info_list(move |result| {
            if let ListResult::Item(sink_info) = result {
                let device_info = DeviceInfo {
                    index: DeviceIndex(sink_info.index),
                    name: super::DeviceName::new(
                        sink_info
                            .name
                            .as_ref()
                            .map(|s| s.to_string())
                            .unwrap_or_default(),
                    ),
                    description: sink_info
                        .description
                        .as_ref()
                        .map(|s| s.to_string())
                        .unwrap_or_default(),
                    device_type: DeviceType::Output,
                    state: DeviceState::Running,
                    is_default: false,
                    ports: sink_info
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
                    active_port: sink_info
                        .active_port
                        .as_ref()
                        .and_then(|p| p.name.as_ref().map(|s| s.to_string())),
                };

                if let Ok(mut devices_guard) = devices_clone.write() {
                    devices_guard.insert(DeviceIndex(sink_info.index), device_info);
                }
            }
        });

        let devices_clone2 = Arc::clone(devices);
        introspect.get_source_info_list(move |result| {
            if let ListResult::Item(source_info) = result {
                let device_info = DeviceInfo {
                    index: DeviceIndex(source_info.index),
                    name: super::DeviceName::new(
                        source_info
                            .name
                            .as_ref()
                            .map(|s| s.to_string())
                            .unwrap_or_default(),
                    ),
                    description: source_info
                        .description
                        .as_ref()
                        .map(|s| s.to_string())
                        .unwrap_or_default(),
                    device_type: DeviceType::Input,
                    state: DeviceState::Running,
                    is_default: false,
                    ports: source_info
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
                    active_port: source_info
                        .active_port
                        .as_ref()
                        .and_then(|p| p.name.as_ref().map(|s| s.to_string())),
                };

                if let Ok(mut devices_guard) = devices_clone2.write() {
                    devices_guard.insert(DeviceIndex(source_info.index), device_info);
                }
            }
        });
    }

    fn load_streams(context: &Context, streams: &Arc<StdRwLock<HashMap<StreamIndex, StreamInfo>>>) {
        let streams_clone = Arc::clone(streams);
        let introspect = context.introspect();

        introspect.get_sink_input_info_list(move |result| {
            if let ListResult::Item(sink_input_info) = result {
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
                    volume: Self::convert_volume_from_pulse(&sink_input_info.volume)
                        .unwrap_or_else(|_| Self::default_volume()),
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
                    streams_guard.insert(StreamIndex(sink_input_info.index), stream_info);
                }
            }
        });
    }

    fn load_defaults(
        context: &Context,
        _default_input: &Arc<StdRwLock<Option<DeviceInfo>>>,
        _default_output: &Arc<StdRwLock<Option<DeviceInfo>>>,
    ) {
        let introspect = context.introspect();

        introspect.get_server_info(|_server_info| {
            // We'll query for defaults separately
        });
    }

    fn handle_command(context: &mut Context, command: PulseCommand) {
        match command {
            PulseCommand::SetDeviceVolume {
                device,
                volume,
                result_tx,
            } => {
                let result = Self::set_device_volume_impl(context, device, volume);
                let _ = result_tx.send(result);
            }
            PulseCommand::SetDeviceMute {
                device,
                muted,
                result_tx,
            } => {
                let result = Self::set_device_mute_impl(context, device, muted);
                let _ = result_tx.send(result);
            }
            PulseCommand::SetDefaultDevice { device, result_tx } => {
                let result = Self::set_default_device_impl(context, device);
                let _ = result_tx.send(result);
            }
            PulseCommand::SetStreamVolume {
                stream,
                volume,
                result_tx,
            } => {
                let result = Self::set_stream_volume_impl(context, stream, volume);
                let _ = result_tx.send(result);
            }
            PulseCommand::SetStreamMute {
                stream,
                muted,
                result_tx,
            } => {
                let result = Self::set_stream_mute_impl(context, stream, muted);
                let _ = result_tx.send(result);
            }
            PulseCommand::MoveStream {
                stream,
                device,
                result_tx,
            } => {
                let result = Self::move_stream_impl(context, stream, device);
                let _ = result_tx.send(result);
            }
        }
    }

    fn default_volume() -> Volume {
        match Volume::mono(1.0) {
            Ok(volume) => volume,
            Err(_) => {
                eprintln!("Failed to create default volume with 1.0, trying fallback");
                match Volume::new(vec![1.0]) {
                    Ok(volume) => volume,
                    Err(_) => {
                        eprintln!("Critical error: cannot create any valid volume");
                        std::process::exit(1);
                    }
                }
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
        let mut pulse_volume = ChannelVolumes::default();
        pulse_volume.set_len(volume.channels() as u8);

        for (i, &vol) in volume.as_slice().iter().enumerate() {
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

    fn set_device_volume_impl(
        context: &mut Context,
        device: DeviceIndex,
        volume: Volume,
    ) -> Result<(), PulseError> {
        let pulse_volume = Self::convert_volume_to_pulse(&volume)?;
        let mut introspect = context.introspect();

        introspect.set_sink_volume_by_index(
            device.0,
            &pulse_volume,
            Some(Box::new(|success: bool| {
                if !success {
                    eprintln!("Failed to set sink volume");
                }
            })),
        );

        Ok(())
    }

    fn set_device_mute_impl(
        context: &mut Context,
        device: DeviceIndex,
        muted: bool,
    ) -> Result<(), PulseError> {
        let mut introspect = context.introspect();

        introspect.set_sink_mute_by_index(
            device.0,
            muted,
            Some(Box::new(|success: bool| {
                if !success {
                    eprintln!("Failed to set sink mute");
                }
            })),
        );

        Ok(())
    }

    fn set_default_device_impl(
        context: &mut Context,
        device: DeviceIndex,
    ) -> Result<(), PulseError> {
        context.set_default_sink(&format!("@DEFAULT_SINK@{}", device.0), |success: bool| {
            if !success {
                eprintln!("Failed to set default sink");
            }
        });

        Ok(())
    }

    fn set_stream_volume_impl(
        context: &mut Context,
        stream: StreamIndex,
        volume: Volume,
    ) -> Result<(), PulseError> {
        let pulse_volume = Self::convert_volume_to_pulse(&volume)?;
        let mut introspect = context.introspect();

        introspect.set_sink_input_volume(
            stream.0,
            &pulse_volume,
            Some(Box::new(|success: bool| {
                if !success {
                    eprintln!("Failed to set sink input volume");
                }
            })),
        );

        Ok(())
    }

    fn set_stream_mute_impl(
        context: &mut Context,
        stream: StreamIndex,
        muted: bool,
    ) -> Result<(), PulseError> {
        let mut introspect = context.introspect();

        introspect.set_sink_input_mute(
            stream.0,
            muted,
            Some(Box::new(|success: bool| {
                if !success {
                    eprintln!("Failed to set sink input mute");
                }
            })),
        );

        Ok(())
    }

    fn move_stream_impl(
        context: &mut Context,
        stream: StreamIndex,
        device: DeviceIndex,
    ) -> Result<(), PulseError> {
        let mut introspect = context.introspect();

        introspect.move_sink_input_by_index(
            stream.0,
            device.0,
            Some(Box::new(|success: bool| {
                if !success {
                    eprintln!("Failed to move sink input");
                }
            })),
        );

        Ok(())
    }
}

#[async_trait::async_trait]
impl AudioService for PulseAudioService {
    type Error = PulseError;

    fn devices(&self) -> std::pin::Pin<Box<dyn Stream<Item = Vec<DeviceInfo>> + Send>> {
        let devices = Arc::clone(&self.devices);
        let mut event_rx = self.event_tx.subscribe();

        Box::pin(stream! {
            {
                let device_list = if let Ok(devices_guard) = devices.read() {
                    devices_guard.values().cloned().collect()
                } else {
                    Vec::new()
                };
                yield device_list;
            }

            loop {
                if let Ok(event) = event_rx.recv().await {
                    match event {
                        AudioEvent::DeviceAdded(_) | AudioEvent::DeviceRemoved(_) | AudioEvent::DeviceChanged(_) => {
                            let device_list = if let Ok(devices_guard) = devices.read() {
                                devices_guard.values().cloned().collect()
                            } else {
                                Vec::new()
                            };
                            yield device_list;
                        }
                        _ => {}
                    }
                }
            }
        })
    }

    fn streams(&self) -> std::pin::Pin<Box<dyn Stream<Item = Vec<StreamInfo>> + Send>> {
        let streams = Arc::clone(&self.streams);
        let mut event_rx = self.event_tx.subscribe();

        Box::pin(stream! {
            {
                let stream_list = if let Ok(streams_guard) = streams.read() {
                    streams_guard.values().cloned().collect()
                } else {
                    Vec::new()
                };
                yield stream_list;
            }

            loop {
                if let Ok(event) = event_rx.recv().await {
                    match event {
                        AudioEvent::StreamAdded(_) | AudioEvent::StreamRemoved(_) | AudioEvent::StreamChanged(_) => {
                            let stream_list = if let Ok(streams_guard) = streams.read() {
                                streams_guard.values().cloned().collect()
                            } else {
                                Vec::new()
                            };
                            yield stream_list;
                        }
                        _ => {}
                    }
                }
            }
        })
    }

    fn events(&self) -> std::pin::Pin<Box<dyn Stream<Item = AudioEvent> + Send>> {
        let mut event_rx = self.event_tx.subscribe();
        Box::pin(stream! {
            loop {
                if let Ok(event) = event_rx.recv().await {
                    yield event;
                }
            }
        })
    }

    fn default_input(&self) -> std::pin::Pin<Box<dyn Stream<Item = DeviceInfo> + Send>> {
        let default_input = Arc::clone(&self.default_input);
        let mut event_rx = self.event_tx.subscribe();

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

            loop {
                if let Ok(event) = event_rx.recv().await {
                    match event {
                        AudioEvent::DefaultInputChanged(_) => {
                            let device_opt = if let Ok(default_guard) = default_input.read() {
                                default_guard.as_ref().cloned()
                            } else {
                                None
                            };
                            if let Some(device) = device_opt {
                                yield device;
                            }
                        }
                        _ => {}
                    }
                }
            }
        })
    }

    fn default_output(&self) -> std::pin::Pin<Box<dyn Stream<Item = DeviceInfo> + Send>> {
        let default_output = Arc::clone(&self.default_output);
        let mut event_rx = self.event_tx.subscribe();

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

            loop {
                if let Ok(event) = event_rx.recv().await {
                    match event {
                        AudioEvent::DefaultOutputChanged(_) => {
                            let device_opt = if let Ok(default_guard) = default_output.read() {
                                default_guard.as_ref().cloned()
                            } else {
                                None
                            };
                            if let Some(device) = device_opt {
                                yield device;
                            }
                        }
                        _ => {}
                    }
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
            .get(&device)
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

    async fn set_device_volume(
        &self,
        device: DeviceIndex,
        volume: Volume,
    ) -> Result<(), Self::Error> {
        let (result_tx, result_rx) = oneshot::channel();
        self.command_tx
            .send(PulseCommand::SetDeviceVolume {
                device,
                volume,
                result_tx,
            })
            .map_err(|_| PulseError::ThreadCommunication)?;
        result_rx
            .await
            .map_err(|_| PulseError::ThreadCommunication)?
    }

    async fn set_device_mute(&self, device: DeviceIndex, muted: bool) -> Result<(), Self::Error> {
        let (result_tx, result_rx) = oneshot::channel();
        self.command_tx
            .send(PulseCommand::SetDeviceMute {
                device,
                muted,
                result_tx,
            })
            .map_err(|_| PulseError::ThreadCommunication)?;
        result_rx
            .await
            .map_err(|_| PulseError::ThreadCommunication)?
    }

    async fn set_default_device(&self, device: DeviceIndex) -> Result<(), Self::Error> {
        let (result_tx, result_rx) = oneshot::channel();
        self.command_tx
            .send(PulseCommand::SetDefaultDevice { device, result_tx })
            .map_err(|_| PulseError::ThreadCommunication)?;
        result_rx
            .await
            .map_err(|_| PulseError::ThreadCommunication)?
    }

    async fn set_stream_volume(
        &self,
        stream: StreamIndex,
        volume: Volume,
    ) -> Result<(), Self::Error> {
        let (result_tx, result_rx) = oneshot::channel();
        self.command_tx
            .send(PulseCommand::SetStreamVolume {
                stream,
                volume,
                result_tx,
            })
            .map_err(|_| PulseError::ThreadCommunication)?;
        result_rx
            .await
            .map_err(|_| PulseError::ThreadCommunication)?
    }

    async fn set_stream_mute(&self, stream: StreamIndex, muted: bool) -> Result<(), Self::Error> {
        let (result_tx, result_rx) = oneshot::channel();
        self.command_tx
            .send(PulseCommand::SetStreamMute {
                stream,
                muted,
                result_tx,
            })
            .map_err(|_| PulseError::ThreadCommunication)?;
        result_rx
            .await
            .map_err(|_| PulseError::ThreadCommunication)?
    }

    async fn move_stream(
        &self,
        stream: StreamIndex,
        device: DeviceIndex,
    ) -> Result<(), Self::Error> {
        let (result_tx, result_rx) = oneshot::channel();
        self.command_tx
            .send(PulseCommand::MoveStream {
                stream,
                device,
                result_tx,
            })
            .map_err(|_| PulseError::ThreadCommunication)?;
        result_rx
            .await
            .map_err(|_| PulseError::ThreadCommunication)?
    }
}

impl Drop for PulseAudioService {
    fn drop(&mut self) {
        if let Some(handle) = self.pulse_handle.take() {
            handle.abort();
        }
    }
}
