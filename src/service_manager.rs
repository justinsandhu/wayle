use std::{error::Error, sync::Arc};

use crate::config_store::ConfigStore;
use crate::services::mpris::MprisMediaService;
use crate::services::pulse::PulseService;

/// Container for all application services
///
/// Holds references to all initialized services that can be shared
/// across the application. Services are created once during startup
/// and then shared via Arc references.
pub struct Services {
    /// Media player service for MPRIS control
    pub media: Arc<MprisMediaService>,
    /// Audio system service for PulseAudio control
    pub audio: Arc<PulseService>,
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
        let audio_service = PulseService::new().await?;

        Ok(Self {
            media: Arc::new(media_service),
            audio: Arc::new(audio_service),
        })
    }
}
