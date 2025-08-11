use std::{error::Error, sync::Arc};

use async_trait::async_trait;
use futures::Stream;

use super::{StreamInfo, StreamKey};
use crate::services::{
    StreamType,
    audio::{AudioEvent, AudioService, device::DeviceKey, volume::Volume},
};

/// Stream management operations
#[async_trait]
pub trait StreamManager {
    /// Error type for stream operations
    type Error: Error + Send + Sync + 'static;

    /// Get specific stream information
    ///
    /// # Errors
    /// Returns error if stream is not found or communication fails
    async fn stream(&self, stream_key: StreamKey) -> Result<StreamInfo, Self::Error>;

    /// Move stream to different device
    ///
    /// # Errors
    /// Returns error if stream or device is not found, or operation fails
    async fn move_stream(
        &self,
        stream_key: StreamKey,
        device_key: DeviceKey,
    ) -> Result<(), Self::Error>;
}

/// Stream volume control operations
#[async_trait]
pub trait StreamVolumeController {
    /// Error type for volume operations
    type Error: Error + Send + Sync + 'static;

    /// Set stream volume
    ///
    /// # Errors
    /// Returns error if stream is not found, volume is invalid, or operation fails
    async fn set_stream_volume(
        &self,
        stream_key: StreamKey,
        volume: Volume,
    ) -> Result<(), Self::Error>;

    /// Set stream mute state
    ///
    /// # Errors
    /// Returns error if stream is not found or operation fails
    async fn set_stream_mute(&self, stream_key: StreamKey, muted: bool) -> Result<(), Self::Error>;
}

/// Stream monitoring streams
pub trait StreamStreams {
    /// Stream of all audio streams
    fn streams(&self) -> impl Stream<Item = Vec<StreamInfo>> + Send;

    /// Stream of playback streams only
    fn playback_streams(&self) -> impl Stream<Item = Vec<StreamInfo>> + Send;

    /// Stream of recording streams only
    fn recording_streams(&self) -> impl Stream<Item = Vec<StreamInfo>> + Send;

    /// Stream of volume changes for specific stream
    fn stream_volume(&self, stream: StreamKey) -> impl Stream<Item = Volume> + Send;

    /// Stream of mute changes for specific stream
    fn stream_mute(&self, stream: StreamKey) -> impl Stream<Item = bool> + Send;

    /// Stream of state changes for specific stream
    fn stream_state(&self, stream: StreamKey) -> impl Stream<Item = StreamInfo> + Send;
}

impl StreamStreams for AudioService {
    fn streams(&self) -> impl Stream<Item = Vec<StreamInfo>> + Send {
        use async_stream::stream;

        let streams = Arc::clone(&self.streams);
        let mut stream_list_rx = self.stream_list_tx.subscribe();

        stream! {
            {
                let stream_list = if let Ok(streams_guard) = streams.read() {
                    streams_guard.values().cloned().collect()
                } else {
                    Vec::new()
                };
                yield stream_list;
            }

            while let Ok(stream_list) = stream_list_rx.recv().await {
                yield stream_list;
            }
        }
    }

    fn playback_streams(&self) -> impl Stream<Item = Vec<StreamInfo>> + Send {
        use async_stream::stream;
        use futures::{StreamExt, pin_mut};

        let streams_stream = self.streams();
        stream! {
            pin_mut!(streams_stream);
            while let Some(stream_list) = streams_stream.next().await {
                let playback_streams: Vec<StreamInfo> = stream_list
                    .into_iter()
                    .filter(|s| s.stream_type == StreamType::Playback)
                    .collect();
                yield playback_streams;
            }
        }
    }

    fn recording_streams(&self) -> impl Stream<Item = Vec<StreamInfo>> + Send {
        use async_stream::stream;
        use futures::{StreamExt, pin_mut};

        let streams_stream = self.streams();
        stream! {
            pin_mut!(streams_stream);
            while let Some(stream_list) = streams_stream.next().await {
                let recording_streams: Vec<StreamInfo> = stream_list
                    .into_iter()
                    .filter(|s| matches!(s.stream_type, StreamType::Record | StreamType::Capture))
                    .collect();
                yield recording_streams;
            }
        }
    }

    fn stream_volume(&self, stream: StreamKey) -> impl Stream<Item = Volume> + Send {
        use async_stream::stream;
        use futures::{StreamExt, pin_mut};

        let events_stream = self.events();
        stream! {
            pin_mut!(events_stream);
            while let Some(event) = events_stream.next().await {
                if let AudioEvent::StreamVolumeChanged { stream_key, volume, .. } = event {
                    if stream_key == stream {
                        yield volume;
                    }
                }
            }
        }
    }

    fn stream_mute(&self, stream: StreamKey) -> impl Stream<Item = bool> + Send {
        use async_stream::stream;
        use futures::{StreamExt, pin_mut};

        let events_stream = self.events();
        stream! {
            pin_mut!(events_stream);
            while let Some(event) = events_stream.next().await {
                if let AudioEvent::StreamMuteChanged { stream_key, muted, .. } = event {
                    if stream_key == stream {
                        yield muted;
                    }
                }
            }
        }
    }

    fn stream_state(&self, stream_key: StreamKey) -> impl Stream<Item = StreamInfo> + Send {
        use async_stream::stream;
        use futures::{StreamExt, pin_mut};

        let events_stream = self.events();
        stream! {
            pin_mut!(events_stream);
            while let Some(event) = events_stream.next().await {
                match event {
                    AudioEvent::StreamChanged(stream_info) if stream_info.key == stream_key => {
                        yield stream_info;
                    }
                    _ => {}
                }
            }
        }
    }
}
