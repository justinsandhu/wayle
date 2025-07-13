use std::sync::Arc;

use libpulse_binding::{callbacks::ListResult, context::Context};

use crate::services::{
    AudioEvent, DeviceIndex,
    pulse::backend::{
        DeviceListSender, DeviceStore, EventSender,
        conversion::{create_device_info_from_sink, create_device_info_from_source},
    },
};

/// Trigger device discovery from PulseAudio
pub fn trigger_device_discovery(
    context: &Context,
    devices: &DeviceStore,
    device_list_tx: &DeviceListSender,
    events_tx: &EventSender,
) {
    discover_sinks(context, devices, device_list_tx, events_tx);
    discover_sources(context, devices, device_list_tx, events_tx);
}

/// Discover output devices (sinks)
fn discover_sinks(
    context: &Context,
    devices: &DeviceStore,
    device_list_tx: &DeviceListSender,
    events_tx: &EventSender,
) {
    let devices_clone = Arc::clone(devices);
    let device_list_tx_clone = device_list_tx.clone();
    let events_tx_clone = events_tx.clone();
    let introspect = context.introspect();

    introspect.get_sink_info_list(move |result| match result {
        ListResult::Item(sink_info) => {
            let device_info = create_device_info_from_sink(sink_info);
            process_device_info(
                device_info,
                DeviceIndex(sink_info.index),
                &devices_clone,
                &events_tx_clone,
            );
        }
        ListResult::End => {
            broadcast_device_list(&device_list_tx_clone, &devices_clone);
        }
        ListResult::Error => {}
    });
}

/// Discover input devices (sources)
fn discover_sources(
    context: &Context,
    devices: &DeviceStore,
    device_list_tx: &DeviceListSender,
    events_tx: &EventSender,
) {
    let devices_clone = Arc::clone(devices);
    let device_list_tx_clone = device_list_tx.clone();
    let events_tx_clone = events_tx.clone();
    let introspect = context.introspect();

    introspect.get_source_info_list(move |result| match result {
        ListResult::Item(source_info) => {
            let device_info = create_device_info_from_source(source_info);
            process_device_info(
                device_info,
                DeviceIndex(source_info.index),
                &devices_clone,
                &events_tx_clone,
            );
        }
        ListResult::End => {
            broadcast_device_list(&device_list_tx_clone, &devices_clone);
        }
        ListResult::Error => {}
    });
}

/// Process device information and emit appropriate events
fn process_device_info(
    device_info: crate::services::DeviceInfo,
    device_index: DeviceIndex,
    devices: &DeviceStore,
    events_tx: &EventSender,
) {
    if let Ok(mut devices_guard) = devices.write() {
        let device_key = device_info.key.clone();
        let is_new_device = !devices_guard.contains_key(&device_key);

        if let Some(existing_device) = devices_guard.get(&device_key) {
            emit_device_change_events(existing_device, &device_info, device_index, events_tx);
        }

        devices_guard.insert(device_key, device_info.clone());

        if is_new_device {
            let _ = events_tx.send(AudioEvent::DeviceAdded(device_info));
        }
    }
}

/// Emit events for device property changes
fn emit_device_change_events(
    existing_device: &crate::services::DeviceInfo,
    new_device: &crate::services::DeviceInfo,
    device_index: DeviceIndex,
    events_tx: &EventSender,
) {
    if existing_device.volume.as_slice() != new_device.volume.as_slice() {
        let _ = events_tx.send(AudioEvent::DeviceVolumeChanged {
            device_index,
            volume: new_device.volume.clone(),
        });
    }

    if existing_device.muted != new_device.muted {
        let _ = events_tx.send(AudioEvent::DeviceMuteChanged {
            device_index,
            muted: new_device.muted,
        });
    }

    if existing_device.properties_changed(new_device) {
        let _ = events_tx.send(AudioEvent::DeviceChanged(new_device.clone()));
    }
}

/// Broadcast current device list to subscribers
pub fn broadcast_device_list(device_list_tx: &DeviceListSender, devices: &DeviceStore) {
    if let Ok(devices_guard) = devices.read() {
        let device_list = devices_guard.values().cloned().collect();
        let _ = device_list_tx.send(device_list);
    }
}

