use std::collections::HashMap;
use std::time::Duration;

use zbus::zvariant::OwnedValue;

/// Default value for unknown metadata fields.
pub const UNKNOWN_METADATA: &str = "Unknown";

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
            title: UNKNOWN_METADATA.to_string(),
            artist: UNKNOWN_METADATA.to_string(),
            album: UNKNOWN_METADATA.to_string(),
            album_artist: UNKNOWN_METADATA.to_string(),
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
            if let Ok(length_i64) = i64::try_from(length.clone())
                && length_i64 > 0
            {
                track.length = Some(Duration::from_micros(length_i64 as u64));
            } else if let Ok(length_u64) = u64::try_from(length.clone())
                && length_u64 > 0
            {
                track.length = Some(Duration::from_micros(length_u64));
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
