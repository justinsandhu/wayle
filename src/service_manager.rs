use std::{error::Error, sync::Arc};

use crate::config_store::ConfigStore;
use crate::services::mpris::MprisMediaService;

/// Container for all application services
///
/// Holds references to all initialized services that can be shared
/// across the application. Services are created once during startup
/// and then shared via Arc references.
pub struct Services {
    /// Media player service for MPRIS control
    pub media: Arc<MprisMediaService>,
}

impl Services {
    /// Create all application services
    ///
    /// Initializes all required services using the provided configuration.
    /// Services are created with proper dependency injection from config.
    ///
    /// # Arguments
    /// * `config_store` - Configuration store for loading service settings
    ///
    /// # Errors
    /// Returns error if any service initialization fails
    pub async fn new(config_store: &ConfigStore) -> Result<Self, Box<dyn Error>> {
        let config = config_store.get_current();

        let media_service = MprisMediaService::new(config.media.ignored_players).await?;

        Ok(Self {
            media: Arc::new(media_service),
        })
    }
}
