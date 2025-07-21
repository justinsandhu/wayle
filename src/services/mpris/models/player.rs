use std::time::Duration;

use futures::StreamExt;
use futures::stream::Stream;
use tokio_stream::StreamExt as TokioStreamExt;

use crate::services::common::Property;
use crate::services::mpris::types::{
    LoopMode, PlaybackState, PlayerId, ShuffleMode, TrackMetadata, Volume, UNKNOWN_METADATA,
};

/// Reactive player model with fine-grained property updates.
///
/// Each property can be watched independently for efficient UI updates.
/// Properties are updated by the D-Bus monitoring layer.
#[derive(Clone)]
pub struct Player {
    /// Unique identifier for this player instance
    pub id: PlayerId,
    /// Human-readable name of the player application
    pub identity: Property<String>,
    /// Desktop file name for the player application
    pub desktop_entry: Property<Option<String>>,

    /// Current playback state (Playing, Paused, Stopped)
    pub playback_state: Property<PlaybackState>,
    /// Current loop mode (None, Track, Playlist)
    pub loop_mode: Property<LoopMode>,
    /// Current shuffle mode (On, Off, Unsupported)
    pub shuffle_mode: Property<ShuffleMode>,
    /// Current volume level
    pub volume: Property<Volume>,

    /// Track title
    pub title: Property<String>,
    /// Track artist
    pub artist: Property<String>,
    /// Album name
    pub album: Property<String>,
    /// Album artist (may differ from track artist)
    pub album_artist: Property<String>,
    /// Track duration
    pub length: Property<Option<Duration>>,
    /// Album art URL
    pub art_url: Property<Option<String>>,
    /// Unique track identifier
    pub track_id: Property<Option<String>>,

    /// Whether the player can be controlled
    pub can_control: Property<bool>,
    /// Whether playback can be started
    pub can_play: Property<bool>,
    /// Whether the player can skip to the next track
    pub can_go_next: Property<bool>,
    /// Whether the player can go to the previous track
    pub can_go_previous: Property<bool>,
    /// Whether the player supports seeking
    pub can_seek: Property<bool>,
    /// Whether the player supports loop modes
    pub can_loop: Property<bool>,
    /// Whether the player supports shuffle
    pub can_shuffle: Property<bool>,
}

impl Player {
    /// Create a new player with default values.
    pub fn new(id: PlayerId, identity: String) -> Self {
        Self {
            id,
            identity: Property::new(identity),
            desktop_entry: Property::new(None),

            playback_state: Property::new(PlaybackState::Stopped),
            loop_mode: Property::new(LoopMode::None),
            shuffle_mode: Property::new(ShuffleMode::Off),
            volume: Property::new(Volume::default()),

            title: Property::new(UNKNOWN_METADATA.to_string()),
            artist: Property::new(UNKNOWN_METADATA.to_string()),
            album: Property::new(UNKNOWN_METADATA.to_string()),
            album_artist: Property::new(UNKNOWN_METADATA.to_string()),
            length: Property::new(None),
            art_url: Property::new(None),
            track_id: Property::new(None),

            can_control: Property::new(false),
            can_play: Property::new(false),
            can_go_next: Property::new(false),
            can_go_previous: Property::new(false),
            can_seek: Property::new(false),
            can_loop: Property::new(false),
            can_shuffle: Property::new(false),
        }
    }

    /// Update metadata from TrackMetadata struct.
    ///
    /// Performs batch update. Property internally handles change detection,
    /// so watchers are only notified when values actually change.
    pub(crate) fn update_metadata(&self, metadata: TrackMetadata) {
        self.title.set(metadata.title);
        self.artist.set(metadata.artist);
        self.album.set(metadata.album);
        self.album_artist.set(metadata.album_artist);
        self.length.set(metadata.length);
        self.art_url.set(metadata.art_url);
        self.track_id.set(metadata.track_id);
    }

    /// Update capabilities from D-Bus properties.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn update_capabilities(
        &self,
        can_control: bool,
        can_play: bool,
        can_go_next: bool,
        can_go_previous: bool,
        can_seek: bool,
        can_loop: bool,
        can_shuffle: bool,
    ) {
        self.can_control.set(can_control);
        self.can_play.set(can_play);
        self.can_go_next.set(can_go_next);
        self.can_go_previous.set(can_go_previous);
        self.can_seek.set(can_seek);
        self.can_loop.set(can_loop);
        self.can_shuffle.set(can_shuffle);
    }

    /// Watch for any metadata changes.
    ///
    /// Combines title, artist, album, art_url changes into a single stream.
    pub fn watch_metadata(&self) -> impl Stream<Item = ()> + Send {
        let title_stream = StreamExt::map(self.title.watch(), |_| ());
        let artist_stream = StreamExt::map(self.artist.watch(), |_| ());
        let album_stream = StreamExt::map(self.album.watch(), |_| ());
        let art_url_stream = StreamExt::map(self.art_url.watch(), |_| ());

        TokioStreamExt::merge(
            TokioStreamExt::merge(title_stream, artist_stream),
            TokioStreamExt::merge(album_stream, art_url_stream),
        )
    }

    /// Watch for any capability changes.
    pub fn watch_capabilities(&self) -> impl Stream<Item = ()> + Send {
        let can_play = StreamExt::map(self.can_play.watch(), |_| ());
        let can_go_next = StreamExt::map(self.can_go_next.watch(), |_| ());
        let can_go_previous = StreamExt::map(self.can_go_previous.watch(), |_| ());
        let can_seek = StreamExt::map(self.can_seek.watch(), |_| ());

        TokioStreamExt::merge(
            TokioStreamExt::merge(can_play, can_go_next),
            TokioStreamExt::merge(can_go_previous, can_seek),
        )
    }
}
