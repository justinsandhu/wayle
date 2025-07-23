use std::time::Duration;

use zbus::Connection;
use zbus::fdo::PropertiesProxy;
use zbus::names::{InterfaceName, MemberName};
use zbus::zvariant::ObjectPath;

use super::handle::PlayerHandle;
use crate::services::mpris::{LoopMode, MediaError, ShuffleMode, Volume};

/// MPRIS service name for D-Bus Player.
const MPRIS_BUS_PLAYER_PATH: &str = "org.mpris.MediaPlayer2.Player";

/// Playback control operations for MPRIS players.
///
/// This module handles all player control operations like play/pause,
/// seek, volume control, etc.
pub(crate) struct Control;

impl Control {
    /// Control playback for a player.
    ///
    /// # Errors
    ///
    /// Returns `MediaError::ControlFailed` if the D-Bus operation fails
    pub(crate) async fn play_pause(handle: &PlayerHandle) -> Result<(), MediaError> {
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
    pub(crate) async fn next(handle: &PlayerHandle) -> Result<(), MediaError> {
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
    pub(crate) async fn previous(handle: &PlayerHandle) -> Result<(), MediaError> {
        handle
            .proxy
            .previous()
            .await
            .map_err(|e| MediaError::ControlFailed(format!("Previous failed: {e}")))?;
        Ok(())
    }

    /// Seek by offset (relative position change).
    ///
    /// # Errors
    ///
    /// Returns `MediaError::ControlFailed` if the D-Bus operation fails
    pub(crate) async fn seek(handle: &PlayerHandle, offset: Duration) -> Result<(), MediaError> {
        let offset_micros = offset.as_micros() as i64;
        handle
            .proxy
            .seek(offset_micros)
            .await
            .map_err(|e| MediaError::ControlFailed(format!("Seek failed: {e}")))?;
        Ok(())
    }

    /// Set position to an absolute value.
    ///
    /// # Errors
    ///
    /// Returns `MediaError::ControlFailed` if the D-Bus operation fails
    pub(crate) async fn set_position(
        handle: &PlayerHandle,
        position: Duration,
    ) -> Result<(), MediaError> {
        let position_micros = position.as_micros() as i64;
        let track_id = handle.player.track_id.get();
        let track_path = track_id.as_deref().unwrap_or("/");
        let track_object_path = ObjectPath::try_from(track_path)
            .map_err(|e| MediaError::ControlFailed(format!("Invalid track ID: {e}")))?;

        handle
            .proxy
            .set_position(&track_object_path, position_micros)
            .await
            .map_err(|e| MediaError::ControlFailed(format!("Set position failed: {e}")))?;
        Ok(())
    }

    /// Set loop mode.
    ///
    /// # Errors
    ///
    /// Returns `MediaError::ControlFailed` if the D-Bus operation fails,
    /// or if the loop mode is unsupported
    pub(crate) async fn set_loop_mode(
        handle: &PlayerHandle,
        mode: LoopMode,
    ) -> Result<(), MediaError> {
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
    pub(crate) async fn set_shuffle_mode(
        handle: &PlayerHandle,
        mode: ShuffleMode,
    ) -> Result<(), MediaError> {
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
    pub(crate) async fn set_volume(
        handle: &PlayerHandle,
        volume: Volume,
    ) -> Result<(), MediaError> {
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
    /// Creates a fresh properties proxy to avoid caching.
    pub(crate) async fn position(
        handle: &PlayerHandle,
        connection: &Connection,
    ) -> Option<Duration> {
        let destination = handle.proxy.inner().destination().to_owned();
        let path = handle.proxy.inner().path().to_owned();

        let proxy = PropertiesProxy::builder(connection)
            .destination(destination)
            .ok()?
            .path(path)
            .ok()?
            .build()
            .await
            .ok()?;

        let interface = InterfaceName::try_from(MPRIS_BUS_PLAYER_PATH).ok()?;
        let property = MemberName::try_from("Position").ok()?;

        match proxy.get(interface, &property).await {
            Ok(value) => {
                if let Ok(micros) = i64::try_from(&value) {
                    let duration = Duration::from_micros(micros.max(0) as u64);
                    Some(duration)
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }
}
