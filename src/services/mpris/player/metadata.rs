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
        Self {
            title: metadata
                .get("xesam:title")
                .and_then(extract_string)
                .unwrap_or_default(),

            artist: metadata
                .get("xesam:artist")
                .and_then(extract_string_array)
                .unwrap_or_default(),

            album: metadata
                .get("xesam:album")
                .and_then(extract_string)
                .unwrap_or_default(),

            album_artist: metadata
                .get("xesam:albumArtist")
                .and_then(extract_string_array)
                .unwrap_or_default(),

            art_url: metadata.get("mpris:artUrl").and_then(extract_string),

            length: metadata.get("mpris:length").and_then(extract_duration),

            track_id: metadata.get("mpris:trackid").and_then(extract_string),
        }
    }
}

fn extract_string(value: &OwnedValue) -> Option<String> {
    if let Ok(s) = String::try_from(value.clone()) {
        return Some(s);
    }
    if let Ok(s) = value.downcast_ref::<String>() {
        return Some(s.clone());
    }
    if let Ok(s) = value.downcast_ref::<&str>() {
        return Some(s.to_string());
    }
    None
}

fn extract_string_array(value: &OwnedValue) -> Option<String> {
    if let Ok(array) = <&zbus::zvariant::Array>::try_from(value) {
        let strings: Vec<String> = array
            .iter()
            .filter_map(|item| {
                item.downcast_ref::<String>()
                    .or_else(|_| item.downcast_ref::<&str>().map(|s| s.to_string()))
                    .ok()
            })
            .collect();

        if !strings.is_empty() {
            return Some(strings.join(", "));
        }
    }

    extract_string(value)
}

fn extract_duration(value: &OwnedValue) -> Option<Duration> {
    if let Ok(length) = i64::try_from(value.clone())
        && length > 0
    {
        return Some(Duration::from_micros(length as u64));
    }

    if let Ok(length) = u64::try_from(value.clone())
        && length > 0
    {
        return Some(Duration::from_micros(length));
    }

    None
}
