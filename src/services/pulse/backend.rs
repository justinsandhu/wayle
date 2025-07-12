use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use libpulse_binding::{
    callbacks::ListResult,
    context::{
        Context, FlagSet as ContextFlags,
        subscribe::{Facility, InterestMaskSet, Operation},
    },
    def::PortAvailable,
    sample::Format as PulseFormat,
    volume::{ChannelVolumes, Volume as PulseVolume},
};
use tokio::sync::{broadcast, mpsc};

use super::{
    device::{DeviceIndex, DeviceInfo, DeviceKey, DeviceName, DevicePort, DeviceState, DeviceType},
    events::AudioEvent,
    stream::{SampleFormat, StreamFormat, StreamIndex, StreamInfo, StreamState, StreamType},
    tokio_mainloop::TokioMain,
    volume::Volume,
};

#[derive(Debug, Clone)]
enum ChangeNotification {
    Device {
        facility: Facility,
        operation: Operation,
        index: u32,
    },
    Stream {
        facility: Facility,
        operation: Operation,
        index: u32,
    },
    Server,
}

/// PulseAudio commands for backend communication
#[derive(Debug)]
pub enum PulseCommand {
    /// Trigger device discovery refresh
    TriggerDeviceDiscovery,
    /// Trigger stream discovery refresh
    TriggerStreamDiscovery,
    /// Set device volume
    SetDeviceVolume {
        /// Target device
        device: DeviceIndex,
        /// New volume levels
        volume: ChannelVolumes,
    },
    /// Set device mute state
    SetDeviceMute {
        /// Target device
        device: DeviceIndex,
        /// Mute state
        muted: bool,
    },
    /// Set stream volume
    SetStreamVolume {
        /// Target stream
        stream: StreamIndex,
        /// New volume levels
        volume: ChannelVolumes,
    },
    /// Set stream mute state
    SetStreamMute {
        /// Target stream
        stream: StreamIndex,
        /// Mute state
        muted: bool,
    },
    /// Set default input device
    SetDefaultInput {
        /// Target device
        device: DeviceIndex,
    },
    /// Set default output device
    SetDefaultOutput {
        /// Target device
        device: DeviceIndex,
    },
    /// Move stream to different device
    MoveStream {
        /// Target stream
        stream: StreamIndex,
        /// Destination device
        device: DeviceIndex,
    },
    /// Shutdown backend
    Shutdown,
}

/// PulseAudio backend implementation
pub struct PulseBackend;

impl PulseBackend {
    /// Spawn the monitoring task for PulseAudio events
    ///
    /// # Errors
    /// Returns error if PulseAudio connection or monitoring setup fails
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::too_many_lines)]
    pub async fn spawn_monitoring_task(
        mut command_rx: mpsc::UnboundedReceiver<PulseCommand>,
        device_list_tx: broadcast::Sender<Vec<DeviceInfo>>,
        stream_list_tx: broadcast::Sender<Vec<StreamInfo>>,
        events_tx: broadcast::Sender<AudioEvent>,
        devices: Arc<RwLock<HashMap<DeviceKey, DeviceInfo>>>,
        streams: Arc<RwLock<HashMap<StreamIndex, StreamInfo>>>,
        default_input: Arc<RwLock<Option<DeviceInfo>>>,
        default_output: Arc<RwLock<Option<DeviceInfo>>>,
        _server_info: Arc<RwLock<Option<String>>>,
    ) -> Result<tokio::task::JoinHandle<()>, super::PulseError> {
        let handle = tokio::task::spawn_blocking(move || {
            let result: Result<(), super::PulseError> = (|| {
                let rt = tokio::runtime::Runtime::new().map_err(|e| {
                    super::PulseError::ConnectionFailed(format!("Failed to create runtime: {e}"))
                })?;
                rt.block_on(async {
                    let mut mainloop = TokioMain::new();
                    let mut context = Context::new(&mainloop, "wayle-pulse").ok_or_else(|| {
                        super::PulseError::ConnectionFailed("Failed to create context".to_string())
                    })?;

                    context
                        .connect(None, ContextFlags::NOFLAGS, None)
                        .map_err(|e| {
                            super::PulseError::ConnectionFailed(format!("Connection failed: {e}"))
                        })?;

                    mainloop.wait_for_ready(&context).await.map_err(|e| {
                        super::PulseError::ConnectionFailed(format!(
                            "Context failed to become ready: {e:?}"
                        ))
                    })?;

                    let (change_tx, mut change_rx) =
                        mpsc::unbounded_channel::<ChangeNotification>();
                    let (internal_command_tx, mut internal_command_rx) =
                        mpsc::unbounded_channel::<PulseCommand>();

                    Self::setup_event_subscription(
                        &mut context,
                        change_tx,
                        internal_command_tx.clone(),
                    )?;

                    let devices_clone = Arc::clone(&devices);
                    let streams_clone = Arc::clone(&streams);
                    let default_input_clone = Arc::clone(&default_input);
                    let default_output_clone = Arc::clone(&default_output);
                    let events_tx_clone = events_tx.clone();
                    let device_list_tx_clone = device_list_tx.clone();
                    let stream_list_tx_clone = stream_list_tx.clone();
                    let command_tx_clone = internal_command_tx.clone();

                    tokio::spawn(async move {
                        while let Some(notification) = change_rx.recv().await {
                            Self::process_change_notification(
                                notification,
                                &devices_clone,
                                &streams_clone,
                                &default_input_clone,
                                &default_output_clone,
                                &events_tx_clone,
                                &device_list_tx_clone,
                                &stream_list_tx_clone,
                                &command_tx_clone,
                            )
                            .await;
                        }
                    });

                    Self::trigger_device_discovery(&context, &devices, &device_list_tx, &events_tx);
                    Self::trigger_stream_discovery(&context, &streams, &stream_list_tx);

                    tokio::select! {
                        _ = mainloop.run() => {}
                        _ = async {
                            loop {
                                tokio::select! {
                                    command = command_rx.recv() => {
                                        if let Some(command) = command {
                                            match command {
                                                PulseCommand::Shutdown => break,
                                                _ => Self::handle_command(
                                                    &mut context,
                                                    command,
                                                    &devices,
                                                    &streams,
                                                    &events_tx,
                                                    &device_list_tx,
                                                    &stream_list_tx,
                                                ),
                                            }
                                        } else {
                                            break;
                                        }
                                    }
                                    command = internal_command_rx.recv() => {
                                        if let Some(command) = command {
                                            Self::handle_command(
                                                &mut context,
                                                command,
                                                &devices,
                                                &streams,
                                                &events_tx,
                                                &device_list_tx,
                                                &stream_list_tx,
                                            );
                                        }
                                    }
                                }
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

    /// Convert our volume to PulseAudio volume
    ///
    /// Maps our 0.0-4.0 range to PulseAudio's 0-MAX range:
    /// - 0.0 → PA_VOLUME_MUTED (0)
    /// - 1.0 → PA_VOLUME_NORM (65536)
    /// - 4.0 → PA_VOLUME_MAX (262144)
    pub fn convert_volume_to_pulse(volume: &Volume) -> ChannelVolumes {
        if volume.channels() == 0 {
            let mut pulse_volume = ChannelVolumes::default();
            pulse_volume.set_len(1);
            pulse_volume.set(0, PulseVolume::NORMAL);
            return pulse_volume;
        }

        let mut pulse_volume = ChannelVolumes::default();
        pulse_volume.set_len(volume.channels() as u8);

        for (i, &vol) in volume.as_slice().iter().enumerate() {
            let pulse_vol = (vol * PulseVolume::NORMAL.0 as f64) as u32;
            pulse_volume.set(i as u8, PulseVolume(pulse_vol));
        }

        pulse_volume
    }

    /// Convert PulseAudio volume to our volume
    ///
    /// Maps PulseAudio's 0-MAX range to our 0.0-4.0 range:
    /// - PA_VOLUME_MUTED (0) → 0.0
    /// - PA_VOLUME_NORM (65536) → 1.0
    /// - PA_VOLUME_MAX (262144) → 4.0
    pub fn convert_volume_from_pulse(pulse_volume: &ChannelVolumes) -> Volume {
        let volumes: Vec<f64> = (0..pulse_volume.len())
            .map(|i| {
                let pulse_vol = pulse_volume.get()[i as usize].0 as f64;
                pulse_vol / PulseVolume::NORMAL.0 as f64
            })
            .collect();

        Volume::new(volumes)
    }

    /// Convert PulseAudio sample format to our format
    pub fn convert_sample_format(format: PulseFormat) -> SampleFormat {
        match format {
            PulseFormat::U8 => SampleFormat::U8,
            PulseFormat::S16le => SampleFormat::S16LE,
            PulseFormat::S16be => SampleFormat::S16BE,
            PulseFormat::S24le => SampleFormat::S24LE,
            PulseFormat::S24be => SampleFormat::S24BE,
            PulseFormat::S32le => SampleFormat::S32LE,
            PulseFormat::S32be => SampleFormat::S32BE,
            PulseFormat::F32le => SampleFormat::F32LE,
            PulseFormat::F32be => SampleFormat::F32BE,
            _ => SampleFormat::Unknown,
        }
    }

    fn setup_event_subscription(
        context: &mut Context,
        change_tx: mpsc::UnboundedSender<ChangeNotification>,
        _command_tx: mpsc::UnboundedSender<PulseCommand>,
    ) -> Result<(), super::PulseError> {
        let interest_mask = InterestMaskSet::SINK
            | InterestMaskSet::SOURCE
            | InterestMaskSet::SINK_INPUT
            | InterestMaskSet::SOURCE_OUTPUT
            | InterestMaskSet::SERVER;

        context.set_subscribe_callback(Some(Box::new(move |facility, operation, index| {
            match facility {
                Some(Facility::Sink) | Some(Facility::Source) => {
                    if let (Some(f), Some(op)) = (facility, operation) {
                        let _ = change_tx.send(ChangeNotification::Device {
                            facility: f,
                            operation: op,
                            index,
                        });
                    }
                }
                Some(Facility::SinkInput) | Some(Facility::SourceOutput) => {
                    if let (Some(f), Some(op)) = (facility, operation) {
                        let _ = change_tx.send(ChangeNotification::Stream {
                            facility: f,
                            operation: op,
                            index,
                        });
                    }
                }
                Some(Facility::Server) => {
                    let _ = change_tx.send(ChangeNotification::Server);
                }
                _ => {}
            }
        })));

        context.subscribe(interest_mask, |_success: bool| {});

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn process_change_notification(
        notification: ChangeNotification,
        devices: &Arc<RwLock<HashMap<DeviceKey, DeviceInfo>>>,
        streams: &Arc<RwLock<HashMap<StreamIndex, StreamInfo>>>,
        _default_input: &Arc<RwLock<Option<DeviceInfo>>>,
        _default_output: &Arc<RwLock<Option<DeviceInfo>>>,
        events_tx: &broadcast::Sender<AudioEvent>,
        device_list_tx: &broadcast::Sender<Vec<DeviceInfo>>,
        stream_list_tx: &broadcast::Sender<Vec<StreamInfo>>,
        command_tx: &mpsc::UnboundedSender<PulseCommand>,
    ) {
        match notification {
            ChangeNotification::Device {
                facility,
                operation,
                index,
            } => {
                Self::handle_device_change(
                    facility,
                    operation,
                    index,
                    devices,
                    events_tx,
                    device_list_tx,
                    command_tx,
                )
                .await;
            }
            ChangeNotification::Stream {
                facility,
                operation,
                index,
            } => {
                Self::handle_stream_change(
                    facility,
                    operation,
                    index,
                    streams,
                    events_tx,
                    stream_list_tx,
                )
                .await;
            }
            ChangeNotification::Server => {
                Self::broadcast_device_list(device_list_tx, devices);
            }
        }
    }

    async fn handle_device_change(
        facility: Facility,
        operation: Operation,
        index: u32,
        devices: &Arc<RwLock<HashMap<DeviceKey, DeviceInfo>>>,
        events_tx: &broadcast::Sender<AudioEvent>,
        device_list_tx: &broadcast::Sender<Vec<DeviceInfo>>,
        command_tx: &mpsc::UnboundedSender<PulseCommand>,
    ) {
        let device_type = match facility {
            Facility::Sink => DeviceType::Output,
            Facility::Source => DeviceType::Input,
            _ => return,
        };
        let device_key = DeviceKey::new(index, device_type.clone());

        match operation {
            Operation::Removed => {
                let removed_device = if let Ok(mut devices_guard) = devices.write() {
                    devices_guard.remove(&device_key)
                } else {
                    None
                };

                if let Some(device_info) = removed_device {
                    let _ = events_tx.send(AudioEvent::DeviceRemoved(device_info));
                }
                Self::broadcast_device_list(device_list_tx, devices);
            }
            Operation::New => {
                let _ = command_tx.send(PulseCommand::TriggerDeviceDiscovery);
                Self::broadcast_device_list(device_list_tx, devices);
            }
            Operation::Changed => {
                let _ = command_tx.send(PulseCommand::TriggerDeviceDiscovery);
                Self::broadcast_device_list(device_list_tx, devices);
            }
        }
    }

    async fn handle_stream_change(
        _facility: Facility,
        operation: Operation,
        index: u32,
        streams: &Arc<RwLock<HashMap<StreamIndex, StreamInfo>>>,
        events_tx: &broadcast::Sender<AudioEvent>,
        stream_list_tx: &broadcast::Sender<Vec<StreamInfo>>,
    ) {
        let stream_index = StreamIndex(index);

        match operation {
            Operation::Removed => {
                let removed_stream = if let Ok(mut streams_guard) = streams.write() {
                    streams_guard.remove(&stream_index)
                } else {
                    None
                };

                if let Some(stream_info) = removed_stream {
                    let _ = events_tx.send(AudioEvent::StreamRemoved(stream_info));
                }
                Self::broadcast_stream_list(stream_list_tx, streams);
            }
            Operation::New => {
                if let Ok(streams_guard) = streams.read() {
                    if let Some(new_stream) = streams_guard.get(&stream_index).cloned() {
                        let _ = events_tx.send(AudioEvent::StreamAdded(new_stream));
                    }
                }
                Self::broadcast_stream_list(stream_list_tx, streams);
            }
            Operation::Changed => {
                Self::broadcast_stream_list(stream_list_tx, streams);
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn trigger_device_discovery(
        context: &Context,
        devices: &Arc<RwLock<HashMap<DeviceKey, DeviceInfo>>>,
        device_list_tx: &broadcast::Sender<Vec<DeviceInfo>>,
        events_tx: &broadcast::Sender<AudioEvent>,
    ) {
        let devices_clone_for_sink = Arc::clone(devices);
        let devices_clone_for_source = Arc::clone(devices);
        let device_list_tx_clone_for_sink = device_list_tx.clone();
        let device_list_tx_clone_for_source = device_list_tx.clone();
        let events_tx_clone_for_sink = events_tx.clone();
        let events_tx_clone_for_source = events_tx.clone();
        let introspect = context.introspect();

        introspect.get_sink_info_list(move |result| match result {
            ListResult::Item(sink_info) => {
                let volume = Self::convert_volume_from_pulse(&sink_info.volume);
                let device_info = DeviceInfo::new(
                    sink_info.index,
                    DeviceType::Output,
                    DeviceName::new(
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
                    sink_info.mute,
                    volume,
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

                if let Ok(mut devices_guard) = devices_clone_for_sink.write() {
                    let device_key = device_info.key.clone();
                    let is_new_device = !devices_guard.contains_key(&device_key);

                    if let Some(existing_device) = devices_guard.get(&device_key) {
                        if existing_device.volume.as_slice() != device_info.volume.as_slice() {
                            let _ =
                                events_tx_clone_for_sink.send(AudioEvent::DeviceVolumeChanged {
                                    device_index: DeviceIndex(sink_info.index),
                                    volume: device_info.volume.clone(),
                                });
                        }

                        if existing_device.muted != device_info.muted {
                            let _ = events_tx_clone_for_sink.send(AudioEvent::DeviceMuteChanged {
                                device_index: DeviceIndex(sink_info.index),
                                muted: device_info.muted,
                            });
                        }
                    }

                    devices_guard.insert(device_key, device_info.clone());

                    if is_new_device {
                        let _ = events_tx_clone_for_sink.send(AudioEvent::DeviceAdded(device_info));
                    }
                }
            }
            ListResult::End => {
                Self::broadcast_device_list(
                    &device_list_tx_clone_for_sink,
                    &devices_clone_for_sink,
                );
            }
            ListResult::Error => {}
        });

        introspect.get_source_info_list(move |result| match result {
            ListResult::Item(source_info) => {
                let volume = Self::convert_volume_from_pulse(&source_info.volume);
                let device_info = DeviceInfo::new(
                    source_info.index,
                    DeviceType::Input,
                    DeviceName::new(
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
                    source_info.mute,
                    volume,
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

                if let Ok(mut devices_guard) = devices_clone_for_source.write() {
                    let device_key = device_info.key.clone();
                    let is_new_device = !devices_guard.contains_key(&device_key);

                    if let Some(existing_device) = devices_guard.get(&device_key) {
                        if existing_device.volume.as_slice() != device_info.volume.as_slice() {
                            let _ =
                                events_tx_clone_for_source.send(AudioEvent::DeviceVolumeChanged {
                                    device_index: DeviceIndex(source_info.index),
                                    volume: device_info.volume.clone(),
                                });
                        }

                        if existing_device.muted != device_info.muted {
                            let _ =
                                events_tx_clone_for_source.send(AudioEvent::DeviceMuteChanged {
                                    device_index: DeviceIndex(source_info.index),
                                    muted: device_info.muted,
                                });
                        }
                    }

                    devices_guard.insert(device_key, device_info.clone());

                    if is_new_device {
                        let _ =
                            events_tx_clone_for_source.send(AudioEvent::DeviceAdded(device_info));
                    }
                }
            }
            ListResult::End => {
                Self::broadcast_device_list(
                    &device_list_tx_clone_for_source,
                    &devices_clone_for_source,
                );
            }
            ListResult::Error => {}
        });
    }

    fn trigger_stream_discovery(
        context: &Context,
        streams: &Arc<RwLock<HashMap<StreamIndex, StreamInfo>>>,
        stream_list_tx: &broadcast::Sender<Vec<StreamInfo>>,
    ) {
        let streams_clone_for_sink_input = Arc::clone(streams);
        let streams_clone_for_source_output = Arc::clone(streams);
        let stream_list_tx_clone_for_sink_input = stream_list_tx.clone();
        let stream_list_tx_clone_for_source_output = stream_list_tx.clone();
        let introspect = context.introspect();

        introspect.get_sink_input_info_list(move |result| match result {
            ListResult::Item(sink_input_info) => {
                let volume = Self::convert_volume_from_pulse(&sink_input_info.volume);
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

                if let Ok(mut streams_guard) = streams_clone_for_sink_input.write() {
                    streams_guard.insert(stream_info.index, stream_info);
                }
            }
            ListResult::End => {
                Self::broadcast_stream_list(
                    &stream_list_tx_clone_for_sink_input,
                    &streams_clone_for_sink_input,
                );
            }
            ListResult::Error => {}
        });

        introspect.get_source_output_info_list(move |result| match result {
            ListResult::Item(source_output_info) => {
                let volume = Self::convert_volume_from_pulse(&source_output_info.volume);
                let stream_info = StreamInfo {
                    index: StreamIndex(source_output_info.index),
                    name: source_output_info
                        .name
                        .clone()
                        .unwrap_or_default()
                        .to_string(),
                    application_name: source_output_info
                        .proplist
                        .get_str("application.name")
                        .unwrap_or_default(),
                    stream_type: StreamType::Record,
                    state: StreamState::Running,
                    device_index: DeviceIndex(source_output_info.source),
                    volume,
                    muted: source_output_info.mute,
                    format: StreamFormat {
                        sample_rate: source_output_info.sample_spec.rate,
                        channels: source_output_info.sample_spec.channels,
                        sample_format: Self::convert_sample_format(
                            source_output_info.sample_spec.format,
                        ),
                    },
                };

                if let Ok(mut streams_guard) = streams_clone_for_source_output.write() {
                    streams_guard.insert(stream_info.index, stream_info);
                }
            }
            ListResult::End => {
                Self::broadcast_stream_list(
                    &stream_list_tx_clone_for_source_output,
                    &streams_clone_for_source_output,
                );
            }
            ListResult::Error => {}
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
        context: &mut Context,
        command: PulseCommand,
        devices: &Arc<RwLock<HashMap<DeviceKey, DeviceInfo>>>,
        streams: &Arc<RwLock<HashMap<StreamIndex, StreamInfo>>>,
        events_tx: &broadcast::Sender<AudioEvent>,
        device_list_tx: &broadcast::Sender<Vec<DeviceInfo>>,
        stream_list_tx: &broadcast::Sender<Vec<StreamInfo>>,
    ) {
        match command {
            PulseCommand::TriggerDeviceDiscovery => {
                Self::trigger_device_discovery(context, devices, device_list_tx, events_tx);
            }
            PulseCommand::TriggerStreamDiscovery => {
                Self::trigger_stream_discovery(context, streams, stream_list_tx);
            }
            PulseCommand::SetDeviceVolume { device, volume } => {
                Self::set_device_volume(context, device, volume, devices);
            }
            PulseCommand::SetDeviceMute { device, muted } => {
                Self::set_device_mute(context, device, muted, devices);
            }
            PulseCommand::SetDefaultInput { device } => {
                Self::set_default_input(context, device, devices);
            }
            PulseCommand::SetDefaultOutput { device } => {
                Self::set_default_output(context, device, devices);
            }
            PulseCommand::SetStreamVolume { stream, volume } => {
                Self::set_stream_volume(context, stream, volume, streams);
            }
            PulseCommand::SetStreamMute { stream, muted } => {
                Self::set_stream_mute(context, stream, muted, streams);
            }
            PulseCommand::MoveStream { stream, device } => {
                Self::move_stream(context, stream, device, streams);
            }
            PulseCommand::Shutdown => {
                // Shutdown handled in main loop
            }
        }
    }

    fn set_device_volume(
        context: &Context,
        device: DeviceIndex,
        volume: ChannelVolumes,
        devices: &Arc<RwLock<HashMap<DeviceKey, DeviceInfo>>>,
    ) {
        let devices_clone = Arc::clone(devices);
        let mut introspect = context.introspect();

        let device_info = {
            if let Ok(devices_guard) = devices_clone.read() {
                devices_guard.values().find(|d| d.index == device).cloned()
            } else {
                return;
            }
        };

        if let Some(info) = device_info {
            match info.device_type {
                DeviceType::Output => {
                    introspect.set_sink_volume_by_index(device.0, &volume, None);
                }
                DeviceType::Input => {
                    introspect.set_source_volume_by_index(device.0, &volume, None);
                }
            }
        }
    }

    fn set_device_mute(
        context: &Context,
        device: DeviceIndex,
        muted: bool,
        devices: &Arc<RwLock<HashMap<DeviceKey, DeviceInfo>>>,
    ) {
        let devices_clone = Arc::clone(devices);
        let mut introspect = context.introspect();

        let device_info = {
            if let Ok(devices_guard) = devices_clone.read() {
                devices_guard.values().find(|d| d.index == device).cloned()
            } else {
                return;
            }
        };

        if let Some(info) = device_info {
            match info.device_type {
                DeviceType::Output => {
                    introspect.set_sink_mute_by_index(device.0, muted, None);
                }
                DeviceType::Input => {
                    introspect.set_source_mute_by_index(device.0, muted, None);
                }
            }
        }
    }

    fn set_default_input(
        context: &mut Context,
        device: DeviceIndex,
        devices: &Arc<RwLock<HashMap<DeviceKey, DeviceInfo>>>,
    ) {
        if let Ok(devices_guard) = devices.read() {
            if let Some(device_info) = devices_guard.values().find(|d| d.index == device) {
                context.set_default_source(device_info.name.as_str(), |_success| {});
            }
        }
    }

    fn set_default_output(
        context: &mut Context,
        device: DeviceIndex,
        devices: &Arc<RwLock<HashMap<DeviceKey, DeviceInfo>>>,
    ) {
        if let Ok(devices_guard) = devices.read() {
            if let Some(device_info) = devices_guard.values().find(|d| d.index == device) {
                context.set_default_sink(device_info.name.as_str(), |_success| {});
            }
        }
    }

    fn set_stream_volume(
        context: &Context,
        stream: StreamIndex,
        volume: ChannelVolumes,
        streams: &Arc<RwLock<HashMap<StreamIndex, StreamInfo>>>,
    ) {
        let streams_clone = Arc::clone(streams);
        let mut introspect = context.introspect();

        let stream_info = {
            if let Ok(streams_guard) = streams_clone.read() {
                streams_guard.get(&stream).cloned()
            } else {
                return;
            }
        };

        if let Some(info) = stream_info {
            match info.stream_type {
                StreamType::Playback => {
                    introspect.set_sink_input_volume(stream.0, &volume, None);
                }
                StreamType::Record | StreamType::Capture => {
                    introspect.set_source_output_volume(stream.0, &volume, None);
                }
            }
        }
    }

    fn set_stream_mute(
        context: &Context,
        stream: StreamIndex,
        muted: bool,
        streams: &Arc<RwLock<HashMap<StreamIndex, StreamInfo>>>,
    ) {
        let streams_clone = Arc::clone(streams);
        let mut introspect = context.introspect();

        let stream_info = {
            if let Ok(streams_guard) = streams_clone.read() {
                streams_guard.get(&stream).cloned()
            } else {
                return;
            }
        };

        if let Some(info) = stream_info {
            match info.stream_type {
                StreamType::Playback => {
                    introspect.set_sink_input_mute(stream.0, muted, None);
                }
                StreamType::Record | StreamType::Capture => {
                    introspect.set_source_output_mute(stream.0, muted, None);
                }
            }
        }
    }

    fn move_stream(
        context: &Context,
        stream: StreamIndex,
        device: DeviceIndex,
        streams: &Arc<RwLock<HashMap<StreamIndex, StreamInfo>>>,
    ) {
        let streams_clone = Arc::clone(streams);
        let mut introspect = context.introspect();

        let stream_info = {
            if let Ok(streams_guard) = streams_clone.read() {
                streams_guard.get(&stream).cloned()
            } else {
                return;
            }
        };

        if let Some(info) = stream_info {
            match info.stream_type {
                StreamType::Playback => {
                    introspect.move_sink_input_by_index(stream.0, device.0, None);
                }
                StreamType::Record | StreamType::Capture => {
                    introspect.move_source_output_by_index(stream.0, device.0, None);
                }
            }
        }
    }
}
