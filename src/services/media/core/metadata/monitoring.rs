use std::sync::{Arc, Weak};

use futures::StreamExt;
use tracing::debug;

use crate::services::media::proxy::MediaPlayer2PlayerProxy;

use super::TrackMetadata;

/// Monitors D-Bus metadata properties and updates the reactive TrackMetadata model.
pub(crate) struct TrackMetadataMonitor;

impl TrackMetadataMonitor {
    /// Start monitoring for metadata changes.
    ///
    /// Monitoring stops automatically when the TrackMetadata is dropped.
    pub fn start(metadata: Arc<TrackMetadata>, proxy: MediaPlayer2PlayerProxy<'static>) {
        let weak = Arc::downgrade(&metadata);

        tokio::spawn(async move {
            Self::monitor(weak, proxy).await;
        });
    }

    async fn monitor(weak: Weak<TrackMetadata>, proxy: MediaPlayer2PlayerProxy<'static>) {
        let mut metadata_changed = proxy.receive_metadata_changed().await;

        while let Some(change) = metadata_changed.next().await {
            let Some(metadata) = weak.upgrade() else {
                debug!("TrackMetadata dropped, stopping monitor");
                return;
            };

            if let Ok(new_metadata) = change.get().await {
                TrackMetadata::update_from_dbus(&metadata, new_metadata);
            }

            drop(metadata);
        }

        debug!("Metadata monitoring ended");
    }
}
