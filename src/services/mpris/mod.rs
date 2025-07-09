/// MPRIS service adapter implementation
pub mod adapter;
/// Player discovery and lifecycle management
pub mod discovery;
/// Media player error types
pub mod error;
/// Track metadata types
pub mod metadata;
/// Player property monitoring
pub mod monitoring;
/// Player types and capabilities
pub mod player;
/// D-Bus proxy trait definitions
pub mod proxy;
/// Domain service trait definitions
pub mod service;
/// MPRIS utility functions
pub mod utils;

pub use adapter::*;
pub use error::*;
pub use metadata::*;
pub use player::*;
pub use proxy::*;
pub use service::*;
