use std::sync::Arc;

use libpulse_binding::{context::Context, volume::ChannelVolumes};

use crate::services::{DeviceIndex, DeviceType, StreamIndex, StreamType};

use super::{
    discovery::{trigger_device_discovery, trigger_server_info_query, trigger_stream_discovery},
    types::{
        DeviceListSender, DeviceStore, EventSender, ExternalCommand, InternalCommand,
        StreamListSender, StreamStore,
    },
};

/// Handle internal PulseAudio commands (event-driven)
pub fn handle_internal_command(
    context: &mut Context,
    command: InternalCommand,
    devices: &DeviceStore,
    streams: &StreamStore,
    events_tx: &EventSender,
    device_list_tx: &DeviceListSender,
    stream_list_tx: &StreamListSender,
) {
    match command {
        InternalCommand::RefreshDevices => {
            trigger_device_discovery(context, devices, device_list_tx, events_tx);
        }
        InternalCommand::RefreshStreams => {
            trigger_stream_discovery(context, streams, stream_list_tx, events_tx);
        }
        InternalCommand::RefreshServerInfo => {
            trigger_server_info_query(context, devices, events_tx);
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
        ExternalCommand::SetDeviceVolume { device, volume } => {
            set_device_volume(context, device, volume, devices);
        }
        ExternalCommand::SetDeviceMute { device, muted } => {
            set_device_mute(context, device, muted, devices);
        }
        ExternalCommand::SetDefaultInput { device } => {
            set_default_input(context, device, devices);
        }
        ExternalCommand::SetDefaultOutput { device } => {
            set_default_output(context, device, devices);
        }
        ExternalCommand::SetStreamVolume { stream, volume } => {
            set_stream_volume(context, stream, volume, streams);
        }
        ExternalCommand::SetStreamMute { stream, muted } => {
            set_stream_mute(context, stream, muted, streams);
        }
        ExternalCommand::MoveStream { stream, device } => {
            move_stream(context, stream, device, streams);
        }
        ExternalCommand::Shutdown => {
            // Shutdown handled in main loop
        }
    }
}

/// Set device volume through PulseAudio
fn set_device_volume(
    context: &Context,
    device: DeviceIndex,
    volume: ChannelVolumes,
    devices: &DeviceStore,
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
        let avg_vol = volume.avg();
        let mut channel_volumes = ChannelVolumes::default();
        channel_volumes.set(info.volume.channels() as u8, avg_vol);

        match info.device_type {
            DeviceType::Output => {
                introspect.set_sink_volume_by_index(device.0, &channel_volumes, None);
            }
            DeviceType::Input => {
                introspect.set_source_volume_by_index(device.0, &channel_volumes, None);
            }
        }
    }
}

/// Set device mute state through PulseAudio
fn set_device_mute(context: &Context, device: DeviceIndex, muted: bool, devices: &DeviceStore) {
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

/// Set default input device
fn set_default_input(context: &mut Context, device: DeviceIndex, devices: &DeviceStore) {
    if let Ok(devices_guard) = devices.read() {
        if let Some(device_info) = devices_guard.values().find(|d| d.index == device) {
            context.set_default_source(device_info.name.as_str(), |_success| {});
        }
    }
}

/// Set default output device
fn set_default_output(context: &mut Context, device: DeviceIndex, devices: &DeviceStore) {
    if let Ok(devices_guard) = devices.read() {
        if let Some(device_info) = devices_guard.values().find(|d| d.index == device) {
            context.set_default_sink(device_info.name.as_str(), |_success| {});
        }
    }
}

/// Set stream volume through PulseAudio
fn set_stream_volume(
    context: &Context,
    stream: StreamIndex,
    volume: ChannelVolumes,
    streams: &StreamStore,
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

/// Set stream mute state through PulseAudio
fn set_stream_mute(context: &Context, stream: StreamIndex, muted: bool, streams: &StreamStore) {
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

/// Move stream to different device
fn move_stream(context: &Context, stream: StreamIndex, device: DeviceIndex, streams: &StreamStore) {
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
