/// Unique identifier for a media player
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlayerId(pub String);

impl PlayerId {
    /// Creates a PlayerId from a D-Bus service name
    pub fn from_bus_name(bus_name: &str) -> Self {
        Self(bus_name.to_string())
    }

    /// Returns the D-Bus service name for this player
    pub fn bus_name(&self) -> &str {
        &self.0
    }
}

/// Information about a media player
#[derive(Debug, Clone)]
pub struct PlayerInfo {
    /// Unique identifier for the player
    pub id: PlayerId,

    /// Human-readable name of the player
    pub identity: String,

    /// Whether the player can be controlled
    pub can_control: bool,

    /// Specific capabilities supported by the player
    pub capabilities: PlayerCapabilities,
}

/// Capabilities supported by a media player
#[derive(Debug, Clone)]
pub struct PlayerCapabilities {
    /// Whether the player supports play/pause
    pub can_play: bool,

    /// Whether the player can skip to next track
    pub can_go_next: bool,

    /// Whether the player can skip to previous track
    pub can_go_previous: bool,

    /// Whether the player supports seeking within tracks
    pub can_seek: bool,

    /// Whether the player supports loop mode changes
    pub can_loop: bool,

    /// Whether the player supports shuffle mode changes
    pub can_shuffle: bool,
}