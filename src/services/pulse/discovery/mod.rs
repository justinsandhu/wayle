/// Device discovery and management
pub mod device;
/// Server information queries
pub mod server;
/// Stream discovery and management
pub mod stream;

pub use device::{broadcast_device_list, trigger_device_discovery};
pub use server::trigger_server_info_query;
pub use stream::{broadcast_stream_list, trigger_stream_discovery};
