use std::{collections::HashMap, time::Duration};
use zbus::zvariant::OwnedValue;

/// Metadata information for a music track
#[derive(Debug, Clone)]
pub struct TrackMetadata {
    /// Track title
    pub title: String,

    /// Track artist
    pub artist: String,

    /// Album name
    pub album: String,

    /// URL to album artwork image
    pub artwork_url: Option<String>,

    /// Track duration
    pub length: Option<Duration>,

    /// MPRIS track identifier
    pub track_id: Option<String>,
}

impl Default for TrackMetadata {
    fn default() -> Self {
        Self {
            title: "Unknown".to_string(),
            artist: "Unknown".to_string(),
            album: "Unknown".to_string(),
            artwork_url: None,
            length: None,
            track_id: None,
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

        if let Some(art_url) = metadata.get("mpris:artUrl") {
            if let Ok(url_str) = String::try_from(art_url.clone()) {
                track.artwork_url = Some(url_str);
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
