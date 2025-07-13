use libpulse_binding::context::{
    Context,
    subscribe::{Facility, InterestMaskSet, Operation},
};
use tokio::sync::mpsc;

use crate::services::{AudioEvent, DeviceType, PulseError, StreamIndex, pulse::device::DeviceKey};

use super::super::discovery::{broadcast_device_list, broadcast_stream_list};
use super::types::{
    ChangeNotification, CommandSender, DefaultDevice, DeviceListSender, DeviceStore, EventSender,
    PulseCommand, StreamListSender, StreamStore,
};

/// Setup PulseAudio event subscription
///
/// # Errors
/// Returns error if PulseAudio subscription setup fails
pub fn setup_event_subscription(
    context: &mut Context,
    change_tx: mpsc::UnboundedSender<ChangeNotification>,
    _command_tx: CommandSender,
) -> Result<(), PulseError> {
    let interest_mask = InterestMaskSet::SINK
        | InterestMaskSet::SOURCE
        | InterestMaskSet::SINK_INPUT
        | InterestMaskSet::SOURCE_OUTPUT
        | InterestMaskSet::SERVER;

    context.set_subscribe_callback(Some(Box::new(
        move |facility, operation, index| match facility {
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
                if let (Some(f), Some(op)) = (facility, operation) {
                    let _ = change_tx.send(ChangeNotification::Server {
                        facility: f,
                        operation: op,
                        index,
                    });
                }
            }
            _ => {}
        },
    )));

    context.subscribe(interest_mask, |_success: bool| {});

    Ok(())
}

/// Process change notifications from PulseAudio
#[allow(clippy::too_many_arguments)]
pub async fn process_change_notification(
    notification: ChangeNotification,
    devices: &DeviceStore,
    streams: &StreamStore,
    _default_input: &DefaultDevice,
    _default_output: &DefaultDevice,
    events_tx: &EventSender,
    device_list_tx: &DeviceListSender,
    stream_list_tx: &StreamListSender,
    command_tx: &CommandSender,
) {
    match notification {
        ChangeNotification::Device {
            facility,
            operation,
            index,
        } => {
            handle_device_change(
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
            handle_stream_change(
                facility,
                operation,
                index,
                streams,
                events_tx,
                stream_list_tx,
                command_tx,
            )
            .await;
        }
        ChangeNotification::Server {
            facility,
            operation,
            index,
        } => {
            handle_server_change(
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
    }
}

/// Handle device-related change notifications
async fn handle_device_change(
    facility: Facility,
    operation: Operation,
    index: u32,
    devices: &DeviceStore,
    events_tx: &EventSender,
    device_list_tx: &DeviceListSender,
    command_tx: &CommandSender,
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
            broadcast_device_list(device_list_tx, devices);
        }
        Operation::New => {
            let _ = command_tx.send(PulseCommand::TriggerDeviceDiscovery);
            broadcast_device_list(device_list_tx, devices);
        }
        Operation::Changed => {
            let _ = command_tx.send(PulseCommand::TriggerDeviceDiscovery);
            broadcast_device_list(device_list_tx, devices);
        }
    }
}

/// Handle stream-related change notifications
async fn handle_stream_change(
    _facility: Facility,
    operation: Operation,
    index: u32,
    streams: &StreamStore,
    events_tx: &EventSender,
    stream_list_tx: &StreamListSender,
    command_tx: &CommandSender,
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
            broadcast_stream_list(stream_list_tx, streams);
        }
        Operation::New => {
            let _ = command_tx.send(PulseCommand::TriggerStreamDiscovery);
            broadcast_stream_list(stream_list_tx, streams);
        }
        Operation::Changed => {
            let _ = command_tx.send(PulseCommand::TriggerStreamDiscovery);
            broadcast_stream_list(stream_list_tx, streams);
        }
    }
}

/// Handle server-related change notifications
async fn handle_server_change(
    _facility: Facility,
    operation: Operation,
    _index: u32,
    _devices: &DeviceStore,
    _events_tx: &EventSender,
    device_list_tx: &DeviceListSender,
    command_tx: &CommandSender,
) {
    match operation {
        Operation::Changed => {
            let _ = command_tx.send(PulseCommand::TriggerServerInfoQuery);
            broadcast_device_list(device_list_tx, _devices);
        }
        _ => {
            broadcast_device_list(device_list_tx, _devices);
        }
    }
}
