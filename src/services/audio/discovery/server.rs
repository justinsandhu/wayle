use std::sync::Arc;

use libpulse_binding::context::Context;

use crate::services::{
    AudioEvent,
    audio::backend::{DefaultDevice, DeviceStore, EventSender},
};

/// Query server information for default device detection and update storage
pub fn trigger_server_info_query(
    context: &Context,
    devices: &DeviceStore,
    events_tx: &EventSender,
    default_input: &DefaultDevice,
    default_output: &DefaultDevice,
) {
    let devices_clone = Arc::clone(devices);
    let events_tx_clone = events_tx.clone();
    let default_input_clone = Arc::clone(default_input);
    let default_output_clone = Arc::clone(default_output);
    let introspect = context.introspect();

    introspect.get_server_info(move |server_info| {
        if let Ok(devices_guard) = devices_clone.read() {
            if let Some(default_sink_name) = &server_info.default_sink_name {
                if let Some(default_device) = devices_guard
                    .values()
                    .find(|d| d.name.as_str() == default_sink_name.as_ref())
                {
                    let _ = default_output_clone.write().map(|mut guard| {
                        *guard = Some(default_device.clone());
                    });
                    let _ = events_tx_clone
                        .send(AudioEvent::DefaultOutputChanged(default_device.clone()));
                }
            }

            if let Some(default_source_name) = &server_info.default_source_name {
                if let Some(default_device) = devices_guard
                    .values()
                    .find(|d| d.name.as_str() == default_source_name.as_ref())
                {
                    let _ = default_input_clone.write().map(|mut guard| {
                        *guard = Some(default_device.clone());
                    });
                    let _ = events_tx_clone
                        .send(AudioEvent::DefaultInputChanged(default_device.clone()));
                }
            }
        }
    });
}
