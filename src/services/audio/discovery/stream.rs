use std::sync::Arc;

use libpulse_binding::{callbacks::ListResult, context::Context};
use tracing::{debug, instrument, warn};

use crate::services::{
    AudioEvent, StreamInfo,
    audio::backend::{
        EventSender, StreamListSender, StreamStore,
        conversion::{create_stream_info_from_sink_input, create_stream_info_from_source_output},
    },
};

/// Trigger stream discovery from PulseAudio
#[instrument(skip_all)]
pub fn trigger_stream_discovery(
    context: &Context,
    streams: &StreamStore,
    stream_list_tx: &StreamListSender,
    events_tx: &EventSender,
) {
    debug!("Starting PulseAudio stream discovery");
    discover_sink_inputs(context, streams, stream_list_tx, events_tx);
    discover_source_outputs(context, streams, stream_list_tx, events_tx);
    debug!("PulseAudio stream discovery initiated");
}

fn discover_sink_inputs(
    context: &Context,
    streams: &StreamStore,
    stream_list_tx: &StreamListSender,
    events_tx: &EventSender,
) {
    let streams_clone = Arc::clone(streams);
    let stream_list_tx_clone = stream_list_tx.clone();
    let events_tx_clone = events_tx.clone();
    let introspect = context.introspect();

    introspect.get_sink_input_info_list(move |result| match result {
        ListResult::Item(sink_input_info) => {
            let stream_info = create_stream_info_from_sink_input(sink_input_info);
            process_stream_info(stream_info, &streams_clone, &events_tx_clone);
        }
        ListResult::End => {
            debug!("Completed sink input discovery");
            broadcast_stream_list(&stream_list_tx_clone, &streams_clone);
        }
        ListResult::Error => {}
    });
}

fn discover_source_outputs(
    context: &Context,
    streams: &StreamStore,
    stream_list_tx: &StreamListSender,
    events_tx: &EventSender,
) {
    let streams_clone = Arc::clone(streams);
    let stream_list_tx_clone = stream_list_tx.clone();
    let events_tx_clone = events_tx.clone();
    let introspect = context.introspect();

    introspect.get_source_output_info_list(move |result| match result {
        ListResult::Item(source_output_info) => {
            let stream_info = create_stream_info_from_source_output(source_output_info);
            process_stream_info(stream_info, &streams_clone, &events_tx_clone);
        }
        ListResult::End => {
            debug!("Completed source output discovery");
            broadcast_stream_list(&stream_list_tx_clone, &streams_clone);
        }
        ListResult::Error => {}
    });
}

fn process_stream_info(stream_info: StreamInfo, streams: &StreamStore, events_tx: &EventSender) {
    if let Ok(mut streams_guard) = streams.write() {
        let is_new_stream = !streams_guard.contains_key(&stream_info.key);

        if let Some(existing_stream) = streams_guard.get(&stream_info.key) {
            emit_stream_change_events(existing_stream, &stream_info, events_tx);
        }

        streams_guard.insert(stream_info.key, stream_info.clone());

        if is_new_stream {
            let _ = events_tx.send(AudioEvent::StreamAdded(stream_info));
        }
    }
}

fn emit_stream_change_events(
    existing_stream: &StreamInfo,
    new_stream: &StreamInfo,
    events_tx: &EventSender,
) {
    if existing_stream.volume.as_slice() != new_stream.volume.as_slice() {
        let _ = events_tx.send(AudioEvent::StreamVolumeChanged {
            stream_key: new_stream.key,
            volume: new_stream.volume.clone(),
        });
    }

    if existing_stream.muted != new_stream.muted {
        let _ = events_tx.send(AudioEvent::StreamMuteChanged {
            stream_key: new_stream.key,
            muted: new_stream.muted,
        });
    }

    if existing_stream.device_index != new_stream.device_index {
        let _ = events_tx.send(AudioEvent::StreamMoved {
            stream_key: new_stream.key,
            from_device: existing_stream.device_index,
            to_device: new_stream.device_index,
        });
    }

    if existing_stream.properties_changed(new_stream) {
        let _ = events_tx.send(AudioEvent::StreamChanged(new_stream.clone()));
    }
}

/// Broadcast current stream list to subscribers
pub fn broadcast_stream_list(stream_list_tx: &StreamListSender, streams: &StreamStore) {
    if let Ok(streams_guard) = streams.read() {
        let stream_list = streams_guard.values().cloned().collect();
        let _ = stream_list_tx.send(stream_list);
    }
}
