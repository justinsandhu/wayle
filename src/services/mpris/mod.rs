/// MPRIS service adapter implementation
pub mod adapter;
/// Player discovery and lifecycle management
pub mod discovery;
/// Player property monitoring
pub mod monitoring;
/// D-Bus proxy trait definitions
pub mod proxy;
/// Domain service trait definitions
pub mod service;
/// Player state tracking
pub mod state;
/// Domain types and conversions
pub mod traits;
/// MPRIS utility functions
pub mod utils;

pub use adapter::*;
pub use proxy::*;
pub use service::*;
pub use traits::*;
