/// PulseAudio command handling
pub mod commands;
/// Data conversion utilities
pub mod conversion;
/// Event subscription and handling
pub mod events;
/// Type definitions and aliases
pub mod types;

pub use types::{
    CommandSender, DefaultDevice, DeviceListSender, DeviceStore, EventSender, ExternalCommand,
    InternalCommand, ServerInfo, StreamListSender, StreamStore,
};

pub use conversion::{convert_volume_from_pulse, convert_volume_to_pulse};

use std::sync::Arc;

use libpulse_binding::{
    context::{Context, FlagSet as ContextFlags},
    volume::ChannelVolumes,
};
use tokio::sync::mpsc;
use tracing::{error, info, instrument};

use crate::services::PulseError;

use super::{discovery, tokio_mainloop::TokioMain, volume};
use commands::{handle_external_command, handle_internal_command};
use events::{process_change_notification, setup_event_subscription};
use types::{ChangeNotification, ExternalCommandReceiver, InternalCommandReceiver};

/// PulseAudio backend implementation
pub struct PulseBackend;

impl PulseBackend {
    /// Spawn the monitoring task for PulseAudio events
    ///
    /// Creates a background task that monitors PulseAudio for device and stream changes,
    /// processes commands, and manages the connection lifecycle.
    ///
    /// # Errors
    /// Returns error if PulseAudio connection or monitoring setup fails
    #[allow(clippy::too_many_arguments)]
    #[instrument(name = "pulse_backend_spawn", skip_all)]
    pub async fn spawn_monitoring_task(
        command_rx: ExternalCommandReceiver,
        device_list_tx: DeviceListSender,
        stream_list_tx: StreamListSender,
        events_tx: EventSender,
        devices: DeviceStore,
        streams: StreamStore,
        default_input: DefaultDevice,
        default_output: DefaultDevice,
        _server_info: ServerInfo,
    ) -> Result<tokio::task::JoinHandle<()>, PulseError> {
        let handle = tokio::task::spawn_blocking(move || {
            let runtime = tokio::runtime::Handle::current();
            runtime.block_on(async move {
                if let Err(e) = Self::monitor_pulse_events(
                    command_rx,
                    device_list_tx,
                    stream_list_tx,
                    events_tx,
                    devices,
                    streams,
                    default_input,
                    default_output,
                )
                .await
                {
                    error!("PulseAudio monitoring task failed: {e}");
                }
            });
        });

        Ok(handle)
    }

    /// Convert our volume to PulseAudio volume
    pub fn convert_volume_to_pulse(volume: &volume::Volume) -> ChannelVolumes {
        conversion::convert_volume_to_pulse(volume)
    }

    /// Convert PulseAudio volume to our volume
    pub fn convert_volume_from_pulse(pulse_volume: &ChannelVolumes) -> volume::Volume {
        conversion::convert_volume_from_pulse(pulse_volume)
    }

    /// Main monitoring loop for PulseAudio events
    ///
    /// Establishes connection to PulseAudio server and runs the event processing loop.
    /// Handles both external commands and internal change notifications.
    ///
    /// # Errors
    /// Returns error if connection fails or event setup fails
    #[allow(clippy::too_many_arguments)]
    async fn monitor_pulse_events(
        command_rx: ExternalCommandReceiver,
        device_list_tx: DeviceListSender,
        stream_list_tx: StreamListSender,
        events_tx: EventSender,
        devices: DeviceStore,
        streams: StreamStore,
        default_input: DefaultDevice,
        default_output: DefaultDevice,
    ) -> Result<(), PulseError> {
        info!("Starting PulseAudio backend monitoring task");

        let mut mainloop = TokioMain::new();
        info!("Creating PulseAudio context");
        let mut context = Context::new(&mainloop, "wayle-pulse")
            .ok_or_else(|| PulseError::ConnectionFailed("Failed to create context".to_string()))?;

        info!("Connecting to PulseAudio server");
        context
            .connect(None, ContextFlags::NOFLAGS, None)
            .map_err(|e| PulseError::ConnectionFailed(format!("Connection failed: {e}")))?;

        info!("Waiting for PulseAudio context to become ready");
        mainloop.wait_for_ready(&context).await.map_err(|e| {
            PulseError::ConnectionFailed(format!("Context failed to become ready: {e:?}"))
        })?;

        let (change_tx, mut change_rx) = mpsc::unbounded_channel::<ChangeNotification>();
        let (internal_command_tx, internal_command_rx) =
            mpsc::unbounded_channel::<InternalCommand>();

        info!("Setting up PulseAudio event subscription");
        setup_event_subscription(&mut context, change_tx, internal_command_tx.clone())?;

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
                process_change_notification(
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

        info!("Triggering initial device and stream discovery");
        discovery::trigger_device_discovery(&context, &devices, &device_list_tx, &events_tx);
        discovery::trigger_stream_discovery(&context, &streams, &stream_list_tx, &events_tx);

        info!("PulseAudio backend fully initialized and monitoring");

        tokio::select! {
            _ = mainloop.run() => {
                info!("PulseAudio mainloop exited");
            }
            _ = Self::run_command_loop(
                &mut context,
                command_rx,
                internal_command_rx,
                &devices,
                &streams,
                &events_tx,
                &device_list_tx,
                &stream_list_tx,
            ) => {
                info!("Command processing loop exited");
            }
        }

        Ok(())
    }

    /// Run the command processing loop
    ///
    /// Processes both external and internal commands until shutdown is requested
    /// or a channel is closed.
    #[allow(clippy::too_many_arguments)]
    async fn run_command_loop(
        context: &mut Context,
        mut command_rx: ExternalCommandReceiver,
        mut internal_command_rx: InternalCommandReceiver,
        devices: &DeviceStore,
        streams: &StreamStore,
        events_tx: &EventSender,
        device_list_tx: &DeviceListSender,
        stream_list_tx: &StreamListSender,
    ) {
        loop {
            tokio::select! {
                external_cmd = command_rx.recv() => {
                    match external_cmd {
                        Some(ExternalCommand::Shutdown) => {
                            info!("Received shutdown command");
                            break;
                        }
                        Some(command) => {
                            handle_external_command(
                                context,
                                command,
                                devices,
                                streams,
                            );
                        }
                        None => {
                            info!("External command channel closed");
                            break;
                        }
                    }
                }
                internal_cmd = internal_command_rx.recv() => {
                    if let Some(command) = internal_cmd {
                        handle_internal_command(
                            context,
                            command,
                            devices,
                            streams,
                            events_tx,
                            device_list_tx,
                            stream_list_tx,
                        );
                    }
                }
            }
        }
    }
}
