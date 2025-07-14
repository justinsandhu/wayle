use std::borrow::Cow;

use libpulse_binding::{
    context::introspect::{SinkInfo, SinkInputInfo, SourceInfo, SourceOutputInfo},
    def::PortAvailable,
    sample::Format as PulseFormat,
    volume::{ChannelVolumes, Volume as PulseVolume},
};

use crate::services::{
    DeviceInfo, DeviceType, StreamIndex, StreamInfo, StreamType, Volume,
    pulse::{
        device::{self, DeviceName, DevicePort, DeviceState},
        stream::{SampleFormat, StreamFormat, StreamState},
    },
};

/// Convert our volume to PulseAudio volume
///
/// Maps our 0.0-4.0 range to PulseAudio's 0-MAX range:
/// - 0.0 → PA_VOLUME_MUTED (0)
/// - 1.0 → PA_VOLUME_NORM (65536)
/// - 4.0 → PA_VOLUME_MAX (262144)
pub fn convert_volume_to_pulse(volume: &Volume) -> ChannelVolumes {
    let channels = volume.channels();
    if channels == 0 {
        return ChannelVolumes::default();
    }

    let avg_level = volume.average();
    let pulse_vol = PulseVolume((avg_level * PulseVolume::NORMAL.0 as f64) as u32);

    let mut pulse_volume = ChannelVolumes::default();
    pulse_volume.set(channels as u8, pulse_vol);

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

fn cow_str_to_string(cow_str: Option<&Cow<str>>) -> String {
    cow_str.map(|s| s.to_string()).unwrap_or_default()
}

/// Create device info from PulseAudio sink information
pub fn create_device_info_from_sink(sink_info: &SinkInfo) -> DeviceInfo {
    let volume = convert_volume_from_pulse(&sink_info.volume);
    let name = cow_str_to_string(sink_info.name.as_ref());
    let description = cow_str_to_string(sink_info.description.as_ref());
    let ports: Vec<DevicePort> = sink_info
        .ports
        .iter()
        .map(|port| DevicePort {
            name: cow_str_to_string(port.name.as_ref()),
            description: cow_str_to_string(port.description.as_ref()),
            priority: port.priority,
            available: port.available == PortAvailable::Yes,
        })
        .collect();
    let active_port = sink_info
        .active_port
        .as_ref()
        .and_then(|p| p.name.as_ref().map(|s| s.to_string()));

    DeviceInfo::new(
        sink_info.index,
        DeviceType::Output,
        DeviceName::new(name),
        description,
        DeviceState::Running,
        sink_info.mute,
        volume,
        ports,
        active_port,
    )
}

/// Create device info from PulseAudio source information
pub fn create_device_info_from_source(source_info: &SourceInfo) -> DeviceInfo {
    let volume = convert_volume_from_pulse(&source_info.volume);
    let name = cow_str_to_string(source_info.name.as_ref());
    let description = cow_str_to_string(source_info.description.as_ref());
    let ports: Vec<DevicePort> = source_info
        .ports
        .iter()
        .map(|port| DevicePort {
            name: cow_str_to_string(port.name.as_ref()),
            description: cow_str_to_string(port.description.as_ref()),
            priority: port.priority,
            available: port.available == PortAvailable::Yes,
        })
        .collect();
    let active_port = source_info
        .active_port
        .as_ref()
        .and_then(|p| p.name.as_ref().map(|s| s.to_string()));

    DeviceInfo::new(
        source_info.index,
        DeviceType::Input,
        DeviceName::new(name),
        description,
        DeviceState::Running,
        source_info.mute,
        volume,
        ports,
        active_port,
    )
}

/// Create stream info from PulseAudio sink input information
pub fn create_stream_info_from_sink_input(sink_input_info: &SinkInputInfo) -> StreamInfo {
    let volume = convert_volume_from_pulse(&sink_input_info.volume);
    let name = sink_input_info.name.clone().unwrap_or_default().to_string();
    let application_name = sink_input_info
        .proplist
        .get_str("application.name")
        .unwrap_or_default();
    let format = StreamFormat {
        sample_rate: sink_input_info.sample_spec.rate,
        channels: sink_input_info.sample_spec.channels,
        sample_format: convert_sample_format(sink_input_info.sample_spec.format),
    };

    StreamInfo {
        index: StreamIndex(sink_input_info.index),
        name,
        application_name,
        stream_type: StreamType::Playback,
        state: StreamState::Running,
        device_index: device::DeviceIndex(sink_input_info.sink),
        volume,
        muted: sink_input_info.mute,
        format,
    }
}

/// Create stream info from PulseAudio source output information
pub fn create_stream_info_from_source_output(source_output_info: &SourceOutputInfo) -> StreamInfo {
    let volume = convert_volume_from_pulse(&source_output_info.volume);
    let name = source_output_info
        .name
        .clone()
        .unwrap_or_default()
        .to_string();
    let application_name = source_output_info
        .proplist
        .get_str("application.name")
        .unwrap_or_default();
    let format = StreamFormat {
        sample_rate: source_output_info.sample_spec.rate,
        channels: source_output_info.sample_spec.channels,
        sample_format: convert_sample_format(source_output_info.sample_spec.format),
    };

    StreamInfo {
        index: StreamIndex(source_output_info.index),
        name,
        application_name,
        stream_type: StreamType::Record,
        state: StreamState::Running,
        device_index: device::DeviceIndex(source_output_info.source),
        volume,
        muted: source_output_info.mute,
        format,
    }
}
