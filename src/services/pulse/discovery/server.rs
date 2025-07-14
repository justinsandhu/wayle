use std::{
    collections::HashMap,
    sync::{Arc, RwLockReadGuard},
};

use libpulse_binding::context::{Context, introspect};

use crate::services::{
    AudioEvent, DeviceInfo,
    pulse::{
        self,
        backend::{DeviceStore, EventSender},
    },
};

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
            check_default_output_change(server_info, &devices_guard, &events_tx_clone);
            check_default_input_change(server_info, &devices_guard, &events_tx_clone);
        }
    });
}

/// Check if default output device has changed
fn check_default_output_change(
    server_info: &introspect::ServerInfo,
    devices_guard: &RwLockReadGuard<HashMap<pulse::device::DeviceKey, DeviceInfo>>,
    events_tx: &EventSender,
) {
    if let Some(default_sink_name) = &server_info.default_sink_name {
        if let Some(default_output) = devices_guard
            .values()
            .find(|d| d.name.as_str() == default_sink_name.as_ref())
        {
            let _ = events_tx.send(AudioEvent::DefaultOutputChanged(default_output.clone()));
        }
    }
}

/// Check if default input device has changed
fn check_default_input_change(
    server_info: &introspect::ServerInfo,
    devices_guard: &RwLockReadGuard<HashMap<pulse::device::DeviceKey, DeviceInfo>>,
    events_tx: &EventSender,
) {
    if let Some(default_source_name) = &server_info.default_source_name {
        if let Some(default_input) = devices_guard
            .values()
            .find(|d| d.name.as_str() == default_source_name.as_ref())
        {
            let _ = events_tx.send(AudioEvent::DefaultInputChanged(default_input.clone()));
        }
    }
}
