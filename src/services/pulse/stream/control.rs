use std::error::Error;

use async_trait::async_trait;
use futures::Stream;

use super::{StreamIndex, StreamInfo};
use crate::services::pulse::{AudioEvent, PulseService, device::DeviceIndex, volume::Volume};

/// Stream management operations
#[async_trait]
pub trait StreamManager {
    /// Error type for stream operations
    type Error: Error + Send + Sync + 'static;

    /// Get specific stream information
    ///
    /// # Errors
    /// Returns error if stream is not found or communication fails
    async fn stream(&self, stream: StreamIndex) -> Result<StreamInfo, Self::Error>;

    /// Move stream to different device
    ///
    /// # Errors
    /// Returns error if stream or device is not found, or operation fails
    async fn move_stream(
        &self,
        stream: StreamIndex,
        device: DeviceIndex,
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
        stream: StreamIndex,
        volume: Volume,
    ) -> Result<(), Self::Error>;

    /// Set stream mute state
    ///
    /// # Errors
    /// Returns error if stream is not found or operation fails
    async fn set_stream_mute(&self, stream: StreamIndex, muted: bool) -> Result<(), Self::Error>;
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
    fn stream_volume(&self, stream: StreamIndex) -> impl Stream<Item = Volume> + Send;

    /// Stream of mute changes for specific stream
    fn stream_mute(&self, stream: StreamIndex) -> impl Stream<Item = bool> + Send;

    /// Stream of state changes for specific stream
    fn stream_state(&self, stream: StreamIndex) -> impl Stream<Item = StreamInfo> + Send;
}

impl StreamStreams for PulseService {
    fn streams(&self) -> impl Stream<Item = Vec<StreamInfo>> + Send {
        use async_stream::stream;

        let streams = self.streams.clone();
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
                    .filter(|s| s.stream_type == super::StreamType::Playback)
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
                    .filter(|s| matches!(s.stream_type, super::StreamType::Record | super::StreamType::Capture))
                    .collect();
                yield recording_streams;
            }
        }
    }

    fn stream_volume(&self, stream: StreamIndex) -> impl Stream<Item = Volume> + Send {
        use async_stream::stream;
        use futures::{StreamExt, pin_mut};

        let events_stream = self.events();
        stream! {
            pin_mut!(events_stream);
            while let Some(event) = events_stream.next().await {
                if let AudioEvent::StreamVolumeChanged { stream_index, volume, .. } = event {
                    if stream_index == stream {
                        yield volume;
                    }
                }
            }
        }
    }

    fn stream_mute(&self, stream: StreamIndex) -> impl Stream<Item = bool> + Send {
        use async_stream::stream;
        use futures::{StreamExt, pin_mut};

        let events_stream = self.events();
        stream! {
            pin_mut!(events_stream);
            while let Some(event) = events_stream.next().await {
                if let AudioEvent::StreamMuteChanged { stream_index, muted, .. } = event {
                    if stream_index == stream {
                        yield muted;
                    }
                }
            }
        }
    }

    fn stream_state(&self, stream: StreamIndex) -> impl Stream<Item = StreamInfo> + Send {
        use async_stream::stream;
        use futures::{StreamExt, pin_mut};

        let events_stream = self.events();
        stream! {
            pin_mut!(events_stream);
            while let Some(event) = events_stream.next().await {
                match event {
                    AudioEvent::StreamChanged(stream_info) if stream_info.index == stream => {
                        yield stream_info;
                    }
                    _ => {}
                }
            }
        }
    }
}
