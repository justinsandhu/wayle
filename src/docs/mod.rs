//! Documentation generation for configuration schemas.
//!
//! Automatically generates markdown documentation from Rust configuration
//! structures using reflection and type information.

mod generator;
mod markdown;
mod module;
mod registry;
mod schema;

pub use generator::DocsGenerator;
pub use markdown::{generate_module_page, generate_property_table};
pub use module::{
    BehaviorConfigs, ModuleInfo, ModuleInfoProvider, StylingConfigs, get_all_modules,
};
pub use registry::ModuleRegistry;
pub use schema::{PropertyInfo, extract_property_info};
