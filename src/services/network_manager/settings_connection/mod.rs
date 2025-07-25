use crate::services::common::Property;

use super::NMSettingsConnectionFlags;

#[derive(Debug, Clone)]
pub struct SettingsConnection {
    pub unsaved: Property<bool>,
    pub flags: Property<NMSettingsConnectionFlags>,
    pub filename: Property<String>,
}
