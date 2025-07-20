use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

use zbus::zvariant::OwnedValue;

/// Unique identifier for a media player
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlayerId(String);

impl PlayerId {
    /// Create a PlayerId from a D-Bus bus name
    pub fn from_bus_name(bus_name: &str) -> Self {
        Self(bus_name.to_string())
    }

    /// Get the D-Bus bus name
    pub fn bus_name(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PlayerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Complete player information including identity, state, and capabilities
#[derive(Debug, Clone)]
pub struct Player {
    /// Unique player identifier
    pub id: PlayerId,

    /// Human-readable player name
    pub identity: String,

    /// Desktop entry name (if available)
    pub desktop_entry: Option<String>,

    /// Current playback state
    pub playback_state: PlaybackState,

    /// Current loop mode
    pub loop_mode: LoopMode,

    /// Current shuffle mode
    pub shuffle_mode: ShuffleMode,

    /// Track title
    pub title: String,

    /// Track artist(s)
    pub artist: String,

    /// Album name
    pub album: String,

    /// Album artist(s)
    pub album_artist: String,

    /// Track length
    pub length: Option<Duration>,

    /// Artwork URL
    pub art_url: Option<String>,

    /// Track ID (unique identifier)
    pub track_id: Option<String>,

    /// Whether the player can be controlled
    pub can_control: bool,

    /// Can start playback
    pub can_play: bool,

    /// Can skip to next track
    pub can_go_next: bool,

    /// Can go to previous track
    pub can_go_previous: bool,

    /// Can seek within tracks
    pub can_seek: bool,

    /// Supports loop modes
    pub can_loop: bool,

    /// Supports shuffle mode
    pub can_shuffle: bool,
}

/// Current playback state of a media player
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlaybackState {
    /// Player is currently playing
    Playing,

    /// Player is paused
    Paused,

    /// Player is stopped
    Stopped,
}

impl From<&str> for PlaybackState {
    fn from(status: &str) -> Self {
        match status {
            "Playing" => Self::Playing,
            "Paused" => Self::Paused,
            _ => Self::Stopped,
        }
    }
}

/// Loop mode for track or playlist repetition
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoopMode {
    /// No looping
    None,

    /// Loop current track
    Track,

    /// Loop entire playlist
    Playlist,

    /// Loop mode not supported by player
    Unsupported,
}

impl From<&str> for LoopMode {
    fn from(status: &str) -> Self {
        match status {
            "None" => Self::None,
            "Track" => Self::Track,
            "Playlist" => Self::Playlist,
            _ => Self::Unsupported,
        }
    }
}

/// Shuffle mode for randomizing playback order
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShuffleMode {
    /// Shuffle enabled
    On,

    /// Shuffle disabled
    Off,

    /// Shuffle mode not supported by player
    Unsupported,
}

impl From<bool> for ShuffleMode {
    fn from(shuffle: bool) -> Self {
        if shuffle { Self::On } else { Self::Off }
    }
}

/// Metadata for a media track
#[derive(Debug, Clone, Default)]
pub struct TrackMetadata {
    /// Track title
    pub title: String,

    /// Track artist(s)
    pub artist: String,

    /// Album name
    pub album: String,

    /// Album artist(s)
    pub album_artist: String,

    /// Track length
    pub length: Option<Duration>,

    /// Artwork URL
    pub art_url: Option<String>,

    /// Track ID (unique identifier)
    pub track_id: Option<String>,
}

impl TrackMetadata {
    /// Create empty metadata with "Unknown" defaults
    pub fn unknown() -> Self {
        Self {
            title: "Unknown".to_string(),
            artist: "Unknown".to_string(),
            album: "Unknown".to_string(),
            album_artist: "Unknown".to_string(),
            ..Default::default()
        }
    }
}

impl From<HashMap<String, OwnedValue>> for TrackMetadata {
    fn from(metadata: HashMap<String, OwnedValue>) -> Self {
        let mut track = Self::default();

        if let Some(title) = metadata.get("xesam:title") {
            if let Ok(title_str) = String::try_from(title.clone()) {
                track.title = title_str;
            }
        }

        if let Some(artist) = metadata.get("xesam:artist") {
            if let Ok(array) = <&zbus::zvariant::Array>::try_from(artist) {
                let artists: Vec<String> = array
                    .iter()
                    .filter_map(|artist| {
                        if let Ok(s) = artist.downcast_ref::<String>() {
                            Some(s.clone())
                        } else if let Ok(s) = artist.downcast_ref::<&str>() {
                            Some(s.to_string())
                        } else {
                            None
                        }
                    })
                    .collect();
                if !artists.is_empty() {
                    track.artist = artists.join(", ");
                }
            } else if let Ok(artist_str) = artist.downcast_ref::<String>() {
                track.artist = artist_str.clone();
            } else if let Ok(artist_str) = artist.downcast_ref::<&str>() {
                track.artist = artist_str.to_string();
            }
        }

        if let Some(album) = metadata.get("xesam:album") {
            if let Ok(album_str) = String::try_from(album.clone()) {
                track.album = album_str;
            }
        }

        if let Some(album_artist) = metadata.get("xesam:albumArtist") {
            if let Ok(array) = <&zbus::zvariant::Array>::try_from(album_artist) {
                let artists: Vec<String> = array
                    .iter()
                    .filter_map(|artist| {
                        if let Ok(s) = artist.downcast_ref::<String>() {
                            Some(s.clone())
                        } else if let Ok(s) = artist.downcast_ref::<&str>() {
                            Some(s.to_string())
                        } else {
                            None
                        }
                    })
                    .collect();
                if !artists.is_empty() {
                    track.album_artist = artists.join(", ");
                }
            } else if let Ok(artist_str) = album_artist.downcast_ref::<String>() {
                track.album_artist = artist_str.clone();
            } else if let Ok(artist_str) = album_artist.downcast_ref::<&str>() {
                track.album_artist = artist_str.to_string();
            }
        }

        if let Some(art_url) = metadata.get("mpris:artUrl") {
            if let Ok(url_str) = String::try_from(art_url.clone()) {
                track.art_url = Some(url_str);
            }
        }

        if let Some(length) = metadata.get("mpris:length") {
            if let Ok(length_micros) = u64::try_from(length.clone()) {
                if length_micros > 0 {
                    track.length = Some(Duration::from_micros(length_micros));
                }
            }
        }

        if let Some(track_id) = metadata.get("mpris:trackid") {
            if let Ok(id_str) = String::try_from(track_id.clone()) {
                track.track_id = Some(id_str);
            }
        }

        track
    }
}

/// Events emitted by the MPRIS service
#[derive(Debug, Clone)]
pub enum PlayerEvent {
    /// A new player was discovered
    PlayerAdded(Player),

    /// A player was removed
    PlayerRemoved(PlayerId),

    /// Playback state changed
    PlaybackStateChanged {
        /// ID of the player whose state changed
        player_id: PlayerId,
        /// New playback state
        state: PlaybackState,
    },

    /// Track metadata changed
    MetadataChanged {
        /// ID of the player whose metadata changed
        player_id: PlayerId,
        /// New track metadata
        metadata: TrackMetadata,
    },

    /// Loop mode changed
    LoopModeChanged {
        /// ID of the player whose loop mode changed
        player_id: PlayerId,
        /// New loop mode
        mode: LoopMode,
    },

    /// Shuffle mode changed
    ShuffleModeChanged {
        /// ID of the player whose shuffle mode changed
        player_id: PlayerId,
        /// New shuffle mode
        mode: ShuffleMode,
    },
}

/// Player control actions
#[derive(Debug, Clone)]
pub enum PlayerAction {
    /// Toggle play/pause
    PlayPause,

    /// Skip to next track
    Next,

    /// Go to previous track
    Previous,

    /// Seek to position
    Seek(Duration),

    /// Set loop mode
    SetLoopMode(LoopMode),

    /// Set shuffle mode
    SetShuffleMode(ShuffleMode),
}
