use std::sync::Arc;

use tokio::sync::OnceCell;

use crate::services::mpris::MprisMediaService;

/// Global media service instance
static MEDIA_SERVICE: OnceCell<Arc<MprisMediaService>> = OnceCell::const_new();

/// Gets or creates the media service instance
///
/// Creates the service on first call, then returns the same instance
/// for all subsequent calls to maintain persistent connections.
///
/// # Errors
///
/// Returns error if media service initialization fails on first call
pub async fn get_media_service() -> Result<Arc<MprisMediaService>, Box<dyn std::error::Error>> {
    let service = MEDIA_SERVICE
        .get_or_try_init(|| async {
            let service = MprisMediaService::new().await?;
            Ok::<_, Box<dyn std::error::Error>>(Arc::new(service))
        })
        .await?;

    Ok(service.clone())
}

