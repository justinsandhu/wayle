use std::sync::Arc;

use libpulse_binding::{context::Context, volume::ChannelVolumes};

use crate::services::{
    DeviceType, StreamType,
    audio::{device::DeviceKey, stream::StreamKey},
};

use super::{
    discovery::{trigger_device_discovery, trigger_server_info_query, trigger_stream_discovery},
    types::{
        DefaultDevice, DeviceListSender, DeviceStore, EventSender, ExternalCommand,
        InternalCommand, StreamListSender, StreamStore,
    },
};

/// Handle internal PulseAudio commands (event-driven)
#[allow(clippy::too_many_arguments)]
pub fn handle_internal_command(
    context: &mut Context,
    command: InternalCommand,
    devices: &DeviceStore,
    streams: &StreamStore,
    events_tx: &EventSender,
    device_list_tx: &DeviceListSender,
    stream_list_tx: &StreamListSender,
    default_input: &DefaultDevice,
    default_output: &DefaultDevice,
) {
    match command {
        InternalCommand::RefreshDevices => {
            trigger_device_discovery(context, devices, device_list_tx, events_tx);
        }
        InternalCommand::RefreshStreams => {
            trigger_stream_discovery(context, streams, stream_list_tx, events_tx);
        }
        InternalCommand::RefreshServerInfo => {
            trigger_server_info_query(context, devices, events_tx, default_input, default_output);
        }
    }
}

/// Handle external PulseAudio commands (user-initiated)
pub fn handle_external_command(
    context: &mut Context,
    command: ExternalCommand,
    devices: &DeviceStore,
    streams: &StreamStore,
) {
    match command {
        ExternalCommand::SetDeviceVolume { device_key, volume } => {
            set_device_volume(context, device_key, volume, devices);
        }
        ExternalCommand::SetDeviceMute { device_key, muted } => {
            set_device_mute(context, device_key, muted, devices);
        }
        ExternalCommand::SetDefaultInput { device_key } => {
            set_default_input(context, device_key, devices);
        }
        ExternalCommand::SetDefaultOutput { device_key } => {
            set_default_output(context, device_key, devices);
        }
        ExternalCommand::SetStreamVolume { stream_key, volume } => {
            set_stream_volume(context, stream_key, volume, streams);
        }
        ExternalCommand::SetStreamMute { stream_key, muted } => {
            set_stream_mute(context, stream_key, muted, streams);
        }
        ExternalCommand::MoveStream {
            stream_key,
            device_key,
        } => {
            move_stream(context, stream_key, device_key, streams);
        }
        ExternalCommand::Shutdown => {
            // Shutdown handled in main loop
        }
    }
}

fn set_device_volume(
    context: &Context,
    device_key: DeviceKey,
    volume: ChannelVolumes,
    devices: &DeviceStore,
) {
    let devices_clone = Arc::clone(devices);
    let mut introspect = context.introspect();

    let device_info = {
        if let Ok(devices_guard) = devices_clone.read() {
            devices_guard
                .values()
                .find(|d| d.key == device_key)
                .cloned()
        } else {
            return;
        }
    };

    if let Some(info) = device_info {
        match info.device_type {
            DeviceType::Output => {
                introspect.set_sink_volume_by_index(device_key.index, &volume, None);
            }
            DeviceType::Input => {
                introspect.set_source_volume_by_index(device_key.index, &volume, None);
            }
        }
    }
}

fn set_device_mute(context: &Context, device_key: DeviceKey, muted: bool, devices: &DeviceStore) {
    let devices_clone = Arc::clone(devices);
    let mut introspect = context.introspect();

    let device_info = {
        if let Ok(devices_guard) = devices_clone.read() {
            devices_guard
                .values()
                .find(|d| d.key == device_key)
                .cloned()
        } else {
            return;
        }
    };

    if let Some(info) = device_info {
        match info.device_type {
            DeviceType::Output => {
                introspect.set_sink_mute_by_index(device_key.index, muted, None);
            }
            DeviceType::Input => {
                introspect.set_source_mute_by_index(device_key.index, muted, None);
            }
        }
    }
}

fn set_default_input(context: &mut Context, device_key: DeviceKey, devices: &DeviceStore) {
    if let Ok(devices_guard) = devices.read() {
        if let Some(device_info) = devices_guard.values().find(|d| d.key == device_key) {
            context.set_default_source(device_info.name.as_str(), |_success| {});
        }
    }
}

fn set_default_output(context: &mut Context, device_key: DeviceKey, devices: &DeviceStore) {
    if let Ok(devices_guard) = devices.read() {
        if let Some(device_info) = devices_guard.values().find(|d| d.key == device_key) {
            context.set_default_sink(device_info.name.as_str(), |_success| {});
        }
    }
}

fn set_stream_volume(
    context: &Context,
    stream_key: StreamKey,
    volume: ChannelVolumes,
    streams: &StreamStore,
) {
    let streams_clone = Arc::clone(streams);
    let mut introspect = context.introspect();

    let stream_info = {
        if let Ok(streams_guard) = streams_clone.read() {
            streams_guard.get(&stream_key).cloned()
        } else {
            return;
        }
    };

    if let Some(info) = stream_info {
        match info.stream_type {
            StreamType::Playback => {
                introspect.set_sink_input_volume(stream_key.index, &volume, None);
            }
            StreamType::Record | StreamType::Capture => {
                introspect.set_source_output_volume(stream_key.index, &volume, None);
            }
        }
    }
}

fn set_stream_mute(context: &Context, stream_key: StreamKey, muted: bool, streams: &StreamStore) {
    let streams_clone = Arc::clone(streams);
    let mut introspect = context.introspect();

    let stream_info = {
        if let Ok(streams_guard) = streams_clone.read() {
            streams_guard.get(&stream_key).cloned()
        } else {
            return;
        }
    };

    if let Some(info) = stream_info {
        match info.stream_type {
            StreamType::Playback => {
                introspect.set_sink_input_mute(stream_key.index, muted, None);
            }
            StreamType::Record | StreamType::Capture => {
                introspect.set_source_output_mute(stream_key.index, muted, None);
            }
        }
    }
}

fn move_stream(
    context: &Context,
    stream_key: StreamKey,
    device_key: DeviceKey,
    streams: &StreamStore,
) {
    let streams_clone = Arc::clone(streams);
    let mut introspect = context.introspect();

    let stream_info = {
        if let Ok(streams_guard) = streams_clone.read() {
            streams_guard.get(&stream_key).cloned()
        } else {
            return;
        }
    };

    if let Some(info) = stream_info {
        match info.stream_type {
            StreamType::Playback => {
                introspect.move_sink_input_by_index(stream_key.index, device_key.index, None);
            }
            StreamType::Record | StreamType::Capture => {
                introspect.move_source_output_by_index(stream_key.index, device_key.index, None);
            }
        }
    }
}
