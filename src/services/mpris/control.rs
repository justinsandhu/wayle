use std::time::Duration;

use super::{MediaError, LoopMode, ShuffleMode, Volume};
use super::service::PlayerHandle;

/// Playback control operations for MPRIS players.
/// 
/// This module handles all player control operations like play/pause,
/// seek, volume control, etc.
pub struct Control;

impl Control {
    /// Control playback for a player.
    ///
    /// # Errors
    ///
    /// Returns `MediaError::ControlFailed` if the D-Bus operation fails
    pub async fn play_pause(handle: &PlayerHandle) -> Result<(), MediaError> {
        handle
            .proxy
            .play_pause()
            .await
            .map_err(|e| MediaError::ControlFailed(format!("Play/pause failed: {e}")))?;
        Ok(())
    }

    /// Skip to next track.
    ///
    /// # Errors
    ///
    /// Returns `MediaError::ControlFailed` if the D-Bus operation fails
    pub async fn next(handle: &PlayerHandle) -> Result<(), MediaError> {
        handle
            .proxy
            .next()
            .await
            .map_err(|e| MediaError::ControlFailed(format!("Next failed: {e}")))?;
        Ok(())
    }

    /// Go to previous track.
    ///
    /// # Errors
    ///
    /// Returns `MediaError::ControlFailed` if the D-Bus operation fails
    pub async fn previous(handle: &PlayerHandle) -> Result<(), MediaError> {
        handle
            .proxy
            .previous()
            .await
            .map_err(|e| MediaError::ControlFailed(format!("Previous failed: {e}")))?;
        Ok(())
    }

    /// Seek to position.
    ///
    /// # Errors
    ///
    /// Returns `MediaError::ControlFailed` if the D-Bus operation fails
    pub async fn seek(handle: &PlayerHandle, offset: Duration) -> Result<(), MediaError> {
        let offset_micros = offset.as_micros() as i64;
        handle
            .proxy
            .seek(offset_micros)
            .await
            .map_err(|e| MediaError::ControlFailed(format!("Seek failed: {e}")))?;
        Ok(())
    }

    /// Set loop mode.
    ///
    /// # Errors
    ///
    /// Returns `MediaError::ControlFailed` if the D-Bus operation fails,
    /// or if the loop mode is unsupported
    pub async fn set_loop_mode(handle: &PlayerHandle, mode: LoopMode) -> Result<(), MediaError> {
        let status = match mode {
            LoopMode::None => "None",
            LoopMode::Track => "Track",
            LoopMode::Playlist => "Playlist",
            LoopMode::Unsupported => {
                return Err(MediaError::ControlFailed(
                    "Loop mode not supported".to_string(),
                ));
            }
        };

        handle
            .proxy
            .set_loop_status(status)
            .await
            .map_err(|e| MediaError::ControlFailed(format!("Set loop mode failed: {e}")))?;
        Ok(())
    }

    /// Set shuffle mode.
    ///
    /// # Errors
    ///
    /// Returns `MediaError::ControlFailed` if the D-Bus operation fails,
    /// or if shuffle is unsupported
    pub async fn set_shuffle_mode(handle: &PlayerHandle, mode: ShuffleMode) -> Result<(), MediaError> {
        let shuffle = match mode {
            ShuffleMode::On => true,
            ShuffleMode::Off => false,
            ShuffleMode::Unsupported => {
                return Err(MediaError::ControlFailed(
                    "Shuffle not supported".to_string(),
                ));
            }
        };

        handle
            .proxy
            .set_shuffle(shuffle)
            .await
            .map_err(|e| MediaError::ControlFailed(format!("Set shuffle failed: {e}")))?;
        Ok(())
    }

    /// Set volume.
    ///
    /// # Errors
    ///
    /// Returns `MediaError::ControlFailed` if the D-Bus operation fails
    pub async fn set_volume(handle: &PlayerHandle, volume: Volume) -> Result<(), MediaError> {
        handle
            .proxy
            .set_volume(*volume)
            .await
            .map_err(|e| MediaError::ControlFailed(format!("Set volume failed: {e}")))?;
        Ok(())
    }

    /// Get current playback position.
    ///
    /// Position is polled on-demand rather than streamed.
    pub async fn position(handle: &PlayerHandle) -> Option<Duration> {
        match handle.proxy.position().await {
            Ok(micros) => Some(Duration::from_micros(micros.max(0) as u64)),
            Err(_) => None,
        }
    }
}