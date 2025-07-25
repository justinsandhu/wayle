/// D-Bus object path reference.
///
/// Represents a D-Bus object path as a string (e.g., "/org/freedesktop/NetworkManager/Devices/3").
/// Used throughout the NetworkManager service to store references to D-Bus objects without
/// holding the actual object data. Objects can be retrieved on-demand using these paths.
pub type ObjectPath = String;
