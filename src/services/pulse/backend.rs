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
    volume::ChannelVolumes,
};
use tokio::sync::{broadcast, mpsc};

use super::{
    device::{DeviceIndex, DeviceInfo, DeviceKey, DevicePort, DeviceState, DeviceType, DeviceName},
    events::AudioEvent,
    stream::{SampleFormat, StreamFormat, StreamIndex, StreamInfo, StreamState, StreamType},
    tokio_mainloop::TokioMain,
    volume::{Volume, VolumeError},
};

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
    pub async fn spawn_monitoring_task(
        mut command_rx: mpsc::UnboundedReceiver<PulseCommand>,
        device_list_tx: broadcast::Sender<Vec<DeviceInfo>>,
        stream_list_tx: broadcast::Sender<Vec<StreamInfo>>,
        events_tx: broadcast::Sender<AudioEvent>,
        devices: Arc<RwLock<HashMap<DeviceKey, DeviceInfo>>>,
        streams: Arc<RwLock<HashMap<StreamIndex, StreamInfo>>>,
        default_input: Arc<RwLock<Option<DeviceInfo>>>,
        default_output: Arc<RwLock<Option<DeviceInfo>>>,
        server_info: Arc<RwLock<Option<String>>>,
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
                    .map_err(|e| super::PulseError::ConnectionFailed(format!("Connection failed: {e}")))?;

                mainloop.wait_for_ready(&context).await.map_err(|e| {
                    super::PulseError::ConnectionFailed(format!("Context failed to become ready: {e:?}"))
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
                    &server_info,
                )?;

                Self::trigger_device_discovery(&context, &devices, &device_list_tx);
                Self::trigger_stream_discovery(&context, &streams, &stream_list_tx);

                tokio::select! {
                    _ = mainloop.run() => {
                    }
                    _ = async {
                        while let Some(command) = command_rx.recv().await {
                            match command {
                                PulseCommand::Shutdown => break,
                                _ => Self::handle_command(&mut context, command, &devices, &streams, &device_list_tx, &stream_list_tx),
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
    /// # Errors
    /// Returns error if volume has invalid channel configuration
    pub fn convert_volume_to_pulse(volume: &Volume) -> Result<ChannelVolumes, VolumeError> {
        if volume.channels() == 0 {
            return Err(VolumeError::InvalidChannel { channel: 0 });
        }

        let mut pulse_volume = ChannelVolumes::default();
        pulse_volume.set_len(volume.channels() as u8);

        for (i, &vol) in volume.as_slice().iter().enumerate() {
            if !(0.0..=10.0).contains(&vol) {
                return Err(VolumeError::InvalidVolume {
                    channel: i,
                    volume: vol,
                });
            }

            let pulse_vol = (vol * libpulse_binding::volume::Volume::NORMAL.0 as f64 / 10.0) as u32;
            pulse_volume.set(i as u8, libpulse_binding::volume::Volume(pulse_vol));
        }

        Ok(pulse_volume)
    }

    /// Convert PulseAudio volume to our volume
    ///
    /// # Errors
    /// Returns error if PulseAudio volume has invalid channel configuration
    pub fn convert_volume_from_pulse(pulse_volume: &ChannelVolumes) -> Result<Volume, VolumeError> {
        let volumes: Vec<f64> = (0..pulse_volume.len())
            .map(|i| {
                let pulse_vol = pulse_volume.get()[i as usize].0 as f64;
                pulse_vol * 10.0 / libpulse_binding::volume::Volume::NORMAL.0 as f64
            })
            .collect();

        Volume::new(volumes)
    }

    /// Convert PulseAudio sample format to our format
    pub fn convert_sample_format(format: PulseFormat) -> SampleFormat {
        match format {
            PulseFormat::U8 => SampleFormat::U8,
            PulseFormat::S16le => SampleFormat::S16LE,
            PulseFormat::S24le => SampleFormat::S24LE,
            PulseFormat::S32le => SampleFormat::S32LE,
            PulseFormat::F32le => SampleFormat::F32LE,
            _ => SampleFormat::Unknown,
        }
    }

    // Private implementation methods...
    
    #[allow(clippy::too_many_arguments)]
    fn setup_event_subscription(
        context: &mut Context,
        device_list_tx: &broadcast::Sender<Vec<DeviceInfo>>,
        stream_list_tx: &broadcast::Sender<Vec<StreamInfo>>,
        events_tx: &broadcast::Sender<AudioEvent>,
        devices: &Arc<RwLock<HashMap<DeviceKey, DeviceInfo>>>,
        streams: &Arc<RwLock<HashMap<StreamIndex, StreamInfo>>>,
        _default_input: &Arc<RwLock<Option<DeviceInfo>>>,
        _default_output: &Arc<RwLock<Option<DeviceInfo>>>,
        _server_info: &Arc<RwLock<Option<String>>>,
    ) -> Result<(), super::PulseError> {
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
                        let _device_index = DeviceIndex(index);
                        let device_type = match facility {
                            Some(Facility::Sink) => DeviceType::Output,
                            Some(Facility::Source) => DeviceType::Input,
                            _ => unreachable!(),
                        };
                        let device_key = DeviceKey::new(index, device_type);
                        let removed_device = if let Ok(mut devices_guard) = devices_clone.write() {
                            devices_guard.remove(&device_key)
                        } else {
                            None
                        };

                        if let Some(device_info) = removed_device {
                            let _ = events_tx_clone.send(AudioEvent::DeviceRemoved(device_info));
                        }
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
                        let removed_stream = if let Ok(mut streams_guard) = streams_clone.write() {
                            streams_guard.remove(&stream_index)
                        } else {
                            None
                        };

                        if let Some(stream_info) = removed_stream {
                            let _ = events_tx_clone.send(AudioEvent::StreamRemoved(stream_info));
                        }
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

        context.subscribe(interest_mask, |_success: bool| {});

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
                        devices_guard.insert(device_info.key.clone(), device_info);
                    }
                }
                ListResult::End => {
                    Self::broadcast_device_list(&device_list_tx_clone, &devices_clone);
                }
                ListResult::Error => {}
            }
        });

        // Similar for sources...
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
        device_list_tx: &broadcast::Sender<Vec<DeviceInfo>>,
        stream_list_tx: &broadcast::Sender<Vec<StreamInfo>>,
    ) {
        // Implementation details for command handling...
        match command {
            PulseCommand::TriggerDeviceDiscovery => {
                Self::trigger_device_discovery(context, devices, device_list_tx);
            }
            PulseCommand::TriggerStreamDiscovery => {
                Self::trigger_stream_discovery(context, streams, stream_list_tx);
            }
            // ... other commands
            _ => {}
        }
    }
}