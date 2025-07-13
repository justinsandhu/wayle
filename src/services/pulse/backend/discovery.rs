use std::sync::Arc;

use libpulse_binding::{callbacks::ListResult, context::Context};

use crate::services::{AudioEvent, DeviceIndex};

use super::{
    conversion::{
        create_device_info_from_sink, create_device_info_from_source,
        create_stream_info_from_sink_input, create_stream_info_from_source_output,
    },
    types::{DeviceListSender, DeviceStore, EventSender, StreamListSender, StreamStore},
};

/// Trigger device discovery from PulseAudio
#[allow(clippy::too_many_lines)]
pub fn trigger_device_discovery(
    context: &Context,
    devices: &DeviceStore,
    device_list_tx: &DeviceListSender,
    events_tx: &EventSender,
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
            let device_info = create_device_info_from_sink(sink_info);

            if let Ok(mut devices_guard) = devices_clone_for_sink.write() {
                let device_key = device_info.key.clone();
                let is_new_device = !devices_guard.contains_key(&device_key);

                if let Some(existing_device) = devices_guard.get(&device_key) {
                    if existing_device.volume.as_slice() != device_info.volume.as_slice() {
                        let _ = events_tx_clone_for_sink.send(AudioEvent::DeviceVolumeChanged {
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

                    if existing_device.properties_changed(&device_info) {
                        let _ = events_tx_clone_for_sink
                            .send(AudioEvent::DeviceChanged(device_info.clone()));
                    }
                }

                devices_guard.insert(device_key, device_info.clone());

                if is_new_device {
                    let _ = events_tx_clone_for_sink.send(AudioEvent::DeviceAdded(device_info));
                }
            }
        }
        ListResult::End => {
            broadcast_device_list(&device_list_tx_clone_for_sink, &devices_clone_for_sink);
        }
        ListResult::Error => {}
    });

    introspect.get_source_info_list(move |result| match result {
        ListResult::Item(source_info) => {
            let device_info = create_device_info_from_source(source_info);

            if let Ok(mut devices_guard) = devices_clone_for_source.write() {
                let device_key = device_info.key.clone();
                let is_new_device = !devices_guard.contains_key(&device_key);

                if let Some(existing_device) = devices_guard.get(&device_key) {
                    if existing_device.volume.as_slice() != device_info.volume.as_slice() {
                        let _ = events_tx_clone_for_source.send(AudioEvent::DeviceVolumeChanged {
                            device_index: DeviceIndex(source_info.index),
                            volume: device_info.volume.clone(),
                        });
                    }

                    if existing_device.muted != device_info.muted {
                        let _ = events_tx_clone_for_source.send(AudioEvent::DeviceMuteChanged {
                            device_index: DeviceIndex(source_info.index),
                            muted: device_info.muted,
                        });
                    }
                }

                devices_guard.insert(device_key, device_info.clone());

                if is_new_device {
                    let _ = events_tx_clone_for_source.send(AudioEvent::DeviceAdded(device_info));
                }
            }
        }
        ListResult::End => {
            broadcast_device_list(&device_list_tx_clone_for_source, &devices_clone_for_source);
        }
        ListResult::Error => {}
    });
}

/// Query server information for default device detection
pub fn trigger_server_info_query(
    context: &Context,
    devices: &DeviceStore,
    events_tx: &EventSender,
) {
    let devices_clone = Arc::clone(devices);
    let events_tx_clone = events_tx.clone();
    let introspect = context.introspect();

    introspect.get_server_info(move |server_info| {
        if let Ok(devices_guard) = devices_clone.read() {
            if let Some(default_sink_name) = &server_info.default_sink_name {
                if let Some(default_output) = devices_guard
                    .values()
                    .find(|d| d.name.as_str() == default_sink_name.as_ref())
                {
                    let _ = events_tx_clone
                        .send(AudioEvent::DefaultOutputChanged(default_output.clone()));
                }
            }

            if let Some(default_source_name) = &server_info.default_source_name {
                if let Some(default_input) = devices_guard
                    .values()
                    .find(|d| d.name.as_str() == default_source_name.as_ref())
                {
                    let _ = events_tx_clone
                        .send(AudioEvent::DefaultInputChanged(default_input.clone()));
                }
            }
        }
    });
}

/// Trigger stream discovery from PulseAudio  
pub fn trigger_stream_discovery(
    context: &Context,
    streams: &StreamStore,
    stream_list_tx: &StreamListSender,
    events_tx: &EventSender,
) {
    let streams_clone_for_sink_input = Arc::clone(streams);
    let streams_clone_for_source_output = Arc::clone(streams);
    let stream_list_tx_clone_for_sink_input = stream_list_tx.clone();
    let stream_list_tx_clone_for_source_output = stream_list_tx.clone();
    let events_tx_clone_for_sink_input = events_tx.clone();
    let events_tx_clone_for_source_output = events_tx.clone();
    let introspect = context.introspect();

    introspect.get_sink_input_info_list(move |result| match result {
        ListResult::Item(sink_input_info) => {
            let stream_info = create_stream_info_from_sink_input(sink_input_info);

            if let Ok(mut streams_guard) = streams_clone_for_sink_input.write() {
                let stream_index = stream_info.index;
                let is_new_stream = !streams_guard.contains_key(&stream_index);

                if let Some(existing_stream) = streams_guard.get(&stream_index) {
                    if existing_stream.volume.as_slice() != stream_info.volume.as_slice() {
                        let _ =
                            events_tx_clone_for_sink_input.send(AudioEvent::StreamVolumeChanged {
                                stream_index: stream_info.index,
                                volume: stream_info.volume.clone(),
                            });
                    }

                    if existing_stream.muted != stream_info.muted {
                        let _ =
                            events_tx_clone_for_sink_input.send(AudioEvent::StreamMuteChanged {
                                stream_index: stream_info.index,
                                muted: stream_info.muted,
                            });
                    }

                    if existing_stream.properties_changed(&stream_info) {
                        let _ = events_tx_clone_for_sink_input
                            .send(AudioEvent::StreamChanged(stream_info.clone()));
                    }
                }
                streams_guard.insert(stream_info.index, stream_info.clone());

                if is_new_stream {
                    let _ =
                        events_tx_clone_for_sink_input.send(AudioEvent::StreamAdded(stream_info));
                }
            }
        }
        ListResult::End => {
            broadcast_stream_list(
                &stream_list_tx_clone_for_sink_input,
                &streams_clone_for_sink_input,
            );
        }
        ListResult::Error => {}
    });

    introspect.get_source_output_info_list(move |result| match result {
        ListResult::Item(source_output_info) => {
            let stream_info = create_stream_info_from_source_output(source_output_info);

            if let Ok(mut streams_guard) = streams_clone_for_source_output.write() {
                let stream_index = stream_info.index;
                let is_new_stream = !streams_guard.contains_key(&stream_index);

                if let Some(existing_stream) = streams_guard.get(&stream_index) {
                    if existing_stream.volume.as_slice() != stream_info.volume.as_slice() {
                        let _ = events_tx_clone_for_source_output.send(
                            AudioEvent::StreamVolumeChanged {
                                stream_index: stream_info.index,
                                volume: stream_info.volume.clone(),
                            },
                        );
                    }

                    if existing_stream.muted != stream_info.muted {
                        let _ =
                            events_tx_clone_for_source_output.send(AudioEvent::StreamMuteChanged {
                                stream_index: stream_info.index,
                                muted: stream_info.muted,
                            });
                    }
                }
                streams_guard.insert(stream_info.index, stream_info.clone());

                if is_new_stream {
                    let _ = events_tx_clone_for_source_output
                        .send(AudioEvent::StreamAdded(stream_info));
                }
            }
        }
        ListResult::End => {
            broadcast_stream_list(
                &stream_list_tx_clone_for_source_output,
                &streams_clone_for_source_output,
            );
        }
        ListResult::Error => {}
    });
}

/// Broadcast current device list to subscribers
pub fn broadcast_device_list(device_list_tx: &DeviceListSender, devices: &DeviceStore) {
    if let Ok(devices_guard) = devices.read() {
        let device_list = devices_guard.values().cloned().collect();
        let _ = device_list_tx.send(device_list);
    }
}

/// Broadcast current stream list to subscribers
pub fn broadcast_stream_list(stream_list_tx: &StreamListSender, streams: &StreamStore) {
    if let Ok(streams_guard) = streams.read() {
        let stream_list = streams_guard.values().cloned().collect();
        let _ = stream_list_tx.send(stream_list);
    }
}
