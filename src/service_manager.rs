use std::{error::Error, sync::Arc};

use tracing::{info, instrument};

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
    #[instrument(skip(config_store))]
    pub async fn new(config_store: &ConfigStore) -> Result<Self, Box<dyn Error>> {
        let config = config_store.get_current();

        info!("Initializing MPRIS media service");
        let media_service = MprisMediaService::new(config.media.ignored_players).await?;
        info!("MPRIS service started successfully");

        info!("Initializing PulseAudio service");
        let audio_service = PulseService::new().await?;
        info!("PulseAudio service started successfully");

        info!("All services initialized successfully");
        Ok(Self {
            media: Arc::new(media_service),
            audio: Arc::new(audio_service),
        })
    }
}
