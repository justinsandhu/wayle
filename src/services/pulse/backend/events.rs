use libpulse_binding::context::{
    Context,
    subscribe::{Facility, InterestMaskSet, Operation},
};
use tokio::sync::mpsc;

use crate::services::{
    AudioEvent, DeviceType, PulseError, StreamType,
    pulse::{device::DeviceKey, stream::StreamKey},
};

use super::super::discovery::{broadcast_device_list, broadcast_stream_list};
use super::types::{
    ChangeNotification, DefaultDevice, DeviceListSender, DeviceStore, EventSender, InternalCommand,
    InternalCommandSender, StreamListSender, StreamStore,
};

type SubscriptionCallback = Option<Box<dyn FnMut(Option<Facility>, Option<Operation>, u32)>>;

/// Setup PulseAudio event subscription
///
/// # Errors
/// Returns error if PulseAudio subscription setup fails
pub fn setup_event_subscription(
    context: &mut Context,
    change_tx: mpsc::UnboundedSender<ChangeNotification>,
    _command_tx: InternalCommandSender,
) -> Result<(), PulseError> {
    let interest_mask = InterestMaskSet::SINK
        | InterestMaskSet::SOURCE
        | InterestMaskSet::SINK_INPUT
        | InterestMaskSet::SOURCE_OUTPUT
        | InterestMaskSet::SERVER;

    let subscription_callback: SubscriptionCallback =
        Some(Box::new(move |facility, operation, index| {
            let notification = match (facility, operation) {
                (Some(facil @ (Facility::Sink | Facility::Source)), Some(oper)) => {
                    Some(ChangeNotification::Device {
                        facility: facil,
                        operation: oper,
                        index,
                    })
                }
                (Some(facil @ (Facility::SinkInput | Facility::SourceOutput)), Some(oper)) => {
                    Some(ChangeNotification::Stream {
                        facility: facil,
                        operation: oper,
                        index,
                    })
                }
                (Some(facil @ Facility::Server), Some(oper)) => Some(ChangeNotification::Server {
                    facility: facil,
                    operation: oper,
                    index,
                }),
                _ => None,
            };

            if let Some(notification) = notification {
                let _ = change_tx.send(notification);
            }
        }));

    context.set_subscribe_callback(subscription_callback);

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
    command_tx: &InternalCommandSender,
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

async fn handle_device_change(
    facility: Facility,
    operation: Operation,
    index: u32,
    devices: &DeviceStore,
    events_tx: &EventSender,
    device_list_tx: &DeviceListSender,
    command_tx: &InternalCommandSender,
) {
    let device_type = match facility {
        Facility::Sink => DeviceType::Output,
        Facility::Source => DeviceType::Input,
        _ => return,
    };
    let device_key = DeviceKey::new(index, device_type);

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
            let _ = command_tx.send(InternalCommand::RefreshDevices);
            broadcast_device_list(device_list_tx, devices);
        }
        Operation::Changed => {
            let _ = command_tx.send(InternalCommand::RefreshDevices);
            broadcast_device_list(device_list_tx, devices);
        }
    }
}

async fn handle_stream_change(
    facility: Facility,
    operation: Operation,
    stream_index: u32,
    streams: &StreamStore,
    events_tx: &EventSender,
    stream_list_tx: &StreamListSender,
    command_tx: &InternalCommandSender,
) {
    let stream_type = match facility {
        Facility::SinkInput => StreamType::Playback,
        Facility::SourceOutput => StreamType::Record,
        _ => return,
    };

    let stream_key = StreamKey {
        stream_type,
        index: stream_index,
    };

    match operation {
        Operation::Removed => {
            let removed_stream = if let Ok(mut streams_guard) = streams.write() {
                streams_guard.remove(&stream_key)
            } else {
                None
            };

            if let Some(stream_info) = removed_stream {
                let _ = events_tx.send(AudioEvent::StreamRemoved(stream_info));
            }
            broadcast_stream_list(stream_list_tx, streams);
        }
        Operation::New => {
            let _ = command_tx.send(InternalCommand::RefreshStreams);
            broadcast_stream_list(stream_list_tx, streams);
        }
        Operation::Changed => {
            let _ = command_tx.send(InternalCommand::RefreshStreams);
            broadcast_stream_list(stream_list_tx, streams);
        }
    }
}

async fn handle_server_change(
    _facility: Facility,
    operation: Operation,
    _index: u32,
    _devices: &DeviceStore,
    _events_tx: &EventSender,
    device_list_tx: &DeviceListSender,
    command_tx: &InternalCommandSender,
) {
    match operation {
        Operation::Changed => {
            let _ = command_tx.send(InternalCommand::RefreshServerInfo);
            broadcast_device_list(device_list_tx, _devices);
        }
        _ => {
            broadcast_device_list(device_list_tx, _devices);
        }
    }
}
