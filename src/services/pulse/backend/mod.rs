/// PulseAudio command handling
pub mod commands;
/// Data conversion utilities
pub mod conversion;
/// Event subscription and handling
pub mod events;
/// Type definitions and aliases
pub mod types;

// Re-export public types for easy access
pub use types::{
    CommandSender, DefaultDevice, DeviceListSender, DeviceStore, EventSender, PulseCommand,
    ServerInfo, StreamListSender, StreamStore,
};

// Re-export conversion functions
pub use conversion::{convert_volume_from_pulse, convert_volume_to_pulse};

use std::sync::Arc;

use libpulse_binding::context::{Context, FlagSet as ContextFlags};
use tokio::sync::mpsc;

use crate::services::PulseError;

use super::{discovery, tokio_mainloop::TokioMain, volume};
use commands::handle_command;
use events::{process_change_notification, setup_event_subscription};
use types::{ChangeNotification, CommandReceiver};

/// PulseAudio backend implementation
pub struct PulseBackend;

impl PulseBackend {
    /// Convert our volume to PulseAudio volume
    pub fn convert_volume_to_pulse(
        volume: &volume::Volume,
    ) -> libpulse_binding::volume::ChannelVolumes {
        conversion::convert_volume_to_pulse(volume)
    }

    /// Convert PulseAudio volume to our volume
    pub fn convert_volume_from_pulse(
        pulse_volume: &libpulse_binding::volume::ChannelVolumes,
    ) -> volume::Volume {
        conversion::convert_volume_from_pulse(pulse_volume)
    }
    /// Spawn the monitoring task for PulseAudio events
    ///
    /// # Errors
    /// Returns error if PulseAudio connection or monitoring setup fails
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::too_many_lines)]
    pub async fn spawn_monitoring_task(
        mut command_rx: CommandReceiver,
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
            let result: Result<(), PulseError> = (|| {
                let rt = tokio::runtime::Runtime::new().map_err(|e| {
                    PulseError::ConnectionFailed(format!("Failed to create runtime: {e}"))
                })?;
                rt.block_on(async {
                    let mut mainloop = TokioMain::new();
                    let mut context = Context::new(&mainloop, "wayle-pulse").ok_or_else(|| {
                        PulseError::ConnectionFailed("Failed to create context".to_string())
                    })?;

                    context
                        .connect(None, ContextFlags::NOFLAGS, None)
                        .map_err(|e| {
                            PulseError::ConnectionFailed(format!("Connection failed: {e}"))
                        })?;

                    mainloop.wait_for_ready(&context).await.map_err(|e| {
                        PulseError::ConnectionFailed(format!(
                            "Context failed to become ready: {e:?}"
                        ))
                    })?;

                    let (change_tx, mut change_rx) =
                        mpsc::unbounded_channel::<ChangeNotification>();
                    let (internal_command_tx, mut internal_command_rx) =
                        mpsc::unbounded_channel::<PulseCommand>();

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

                    discovery::trigger_device_discovery(
                        &context,
                        &devices,
                        &device_list_tx,
                        &events_tx,
                    );
                    discovery::trigger_stream_discovery(
                        &context,
                        &streams,
                        &stream_list_tx,
                        &events_tx,
                    );

                    tokio::select! {
                        _ = mainloop.run() => {}
                        _ = async {
                            loop {
                                tokio::select! {
                                    command = command_rx.recv() => {
                                        if let Some(command) = command {
                                            match command {
                                                PulseCommand::Shutdown => break,
                                                _ => handle_command(
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
                                            handle_command(
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
}
