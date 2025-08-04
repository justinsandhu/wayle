use super::ConnectionType;

/// Network service errors
#[derive(thiserror::Error, Debug)]
pub enum NetworkError {
    /// D-Bus communication error
    #[error("D-Bus operation failed: {0}")]
    DbusError(#[from] zbus::Error),

    /// Device not found
    #[error("Device {0} not found")]
    DeviceNotFound(String),

    /// Access point not found
    #[error("Access point {ssid} not found")]
    AccessPointNotFound {
        /// SSID of the access point that was not found.
        ssid: String,
    },

    /// Connection activation failed
    #[error("Failed to activate connection: {0}")]
    ActivationFailed(String),

    /// No active connection
    #[error("No active {0:?} connection")]
    NoActiveConnection(ConnectionType),

    /// Service initialization failed
    #[error("Failed to initialize network service: {0}")]
    InitializationFailed(String),

    /// Object not found at the specified D-Bus path
    #[error("Object not found at path: {0}")]
    ObjectNotFound(String),

    /// Network operation failed
    #[error("Network operation failed: {operation} - {reason}")]
    OperationFailed {
        /// The operation that failed
        operation: &'static str,
        /// The reason the operation failed
        reason: String,
    },
}
