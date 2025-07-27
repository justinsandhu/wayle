use crate::services::common::Property;

use super::NMSettingsConnectionFlags;

/// Connection Settings Profile.
///
/// Represents a single network connection configuration stored in NetworkManager.
/// This includes the connection's settings, whether it has unsaved changes, and
/// where it's stored on disk (if file-backed).
#[derive(Debug, Clone)]
pub struct SettingsConnection {
    /// If set, indicates that the in-memory state of the connection does not
    /// match the on-disk state. This flag will be set when UpdateUnsaved() is
    /// called or when any connection details change, and cleared when the
    /// connection is saved to disk via Save() or from internal operations.
    pub unsaved: Property<bool>,

    /// Additional flags of the connection profile.
    pub flags: Property<NMSettingsConnectionFlags>,

    /// File that stores the connection in case the connection is file-backed.
    pub filename: Property<String>,
}
