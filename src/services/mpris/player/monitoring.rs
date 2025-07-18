use std::collections::HashMap;
use std::sync::Arc;

use futures::StreamExt;
use tokio::sync::{RwLock, broadcast};
use zbus::{proxy::PropertyChanged, zvariant::OwnedValue};

use super::state::PlayerStateTracker;
use crate::services::mpris::{
    LoopMode, MediaError, MediaPlayer2PlayerProxy, PlaybackState, PlayerEvent, PlayerEventSender,
    PlayerId, ShuffleMode, TrackMetadata, utils,
};

/// Thread-safe collection of active media players and their state trackers.
///
/// Maps player IDs to their corresponding state trackers, allowing concurrent
/// access from multiple components. Used throughout the MPRIS service for
/// player state management and property monitoring.
pub type PlayerList = Arc<RwLock<HashMap<PlayerId, PlayerStateTracker>>>;

/// Handles player property monitoring and state updates
pub struct PlayerMonitoring {
    players: PlayerList,
    events_tx: broadcast::Sender<PlayerEvent>,
}

impl PlayerMonitoring {
    /// Create a new player monitoring handler
    pub fn new(players: PlayerList, events_tx: PlayerEventSender) -> Self {
        Self { players, events_tx }
    }

    /// Start monitoring a player's properties
    pub async fn start_monitoring(&self, player_id: PlayerId) -> tokio::task::JoinHandle<()> {
        let monitoring = self.clone();

        tokio::spawn(async move {
            if let Err(e) = monitoring.monitor_player_properties(player_id).await {
                println!("Player monitoring failed: {e}");
            }
        })
    }

    /// Monitor a player's properties for changes
    async fn monitor_player_properties(&self, player_id: PlayerId) -> Result<(), MediaError> {
        let player_proxy = self.get_player_proxy_for_monitoring(&player_id).await?;
        self.run_property_monitoring_loop(player_id, player_proxy)
            .await;
        Ok(())
    }

    async fn get_player_proxy_for_monitoring(
        &self,
        player_id: &PlayerId,
    ) -> Result<MediaPlayer2PlayerProxy<'static>, MediaError> {
        let players = self.players.read().await;
        let tracker = players
            .get(player_id)
            .ok_or_else(|| MediaError::PlayerNotFound(player_id.clone()))?;
        Ok(tracker.player_proxy.clone())
    }

    #[allow(clippy::cognitive_complexity)]
    async fn run_property_monitoring_loop(
        &self,
        player_id: PlayerId,
        player_proxy: MediaPlayer2PlayerProxy<'static>,
    ) {
        let mut position_changes = player_proxy.receive_position_changed().await;
        let mut playback_status_changes = player_proxy.receive_playback_status_changed().await;
        let mut metadata_changes = player_proxy.receive_metadata_changed().await;
        let mut loop_status_changes = player_proxy.receive_loop_status_changed().await;
        let mut shuffle_changes = player_proxy.receive_shuffle_changed().await;

        loop {
            tokio::select! {
                signal = position_changes.next() => {
                    match signal {
                        Some(signal) => self.handle_position_signal(player_id.clone(), signal).await,
                        None => tracing::debug!("Position updates stopped for player {player_id:?}"),
                    }
                }
                signal = playback_status_changes.next() => {
                    match signal {
                        Some(signal) => self.handle_playback_status_signal(player_id.clone(), signal).await,
                        None => tracing::debug!("Playback status updates stopped for player {player_id:?}"),

                    }
                }
                signal = metadata_changes.next() => {
                    match signal {
                        Some(signal) => self.handle_metadata_signal(player_id.clone(), signal).await,
                        None => tracing::debug!("Metadata updates stopped for player {player_id:?}"),
                    }
                }
                signal = loop_status_changes.next() => {
                    match signal {
                        Some(signal) => self.handle_loop_status_signal(player_id.clone(), signal).await,
                        None => tracing::debug!("Loop status updates stopped for player {player_id:?}"),
                    }
                }
                signal = shuffle_changes.next() => {
                    match signal {
                        Some(signal) => self.handle_shuffle_signal(player_id.clone(), signal).await,
                        None => tracing::debug!("Shuffle updates stopped for player {player_id:?}"),
                    }
                }
            }
        }
    }

    async fn handle_position_signal(&self, player_id: PlayerId, signal: PropertyChanged<'_, i64>) {
        let Ok(position) = signal.get().await else {
            return;
        };

        let duration = utils::from_mpris_micros(position);

        {
            let mut players = self.players.write().await;
            if let Some(tracker) = players.get_mut(&player_id) {
                tracker.state.position = duration;
            }
        }

        let _ = self.events_tx.send(PlayerEvent::PositionChanged {
            player_id,
            position: duration,
        });
    }

    async fn handle_playback_status_signal(
        &self,
        player_id: PlayerId,
        signal: PropertyChanged<'_, String>,
    ) {
        let Ok(status) = signal.get().await else {
            return;
        };

        let state = PlaybackState::from(status.as_str());

        {
            let mut players = self.players.write().await;
            if let Some(tracker) = players.get_mut(&player_id) {
                tracker.state.playback_state = state;
            }
        }

        let _ = self
            .events_tx
            .send(PlayerEvent::PlaybackStateChanged { player_id, state });
    }

    async fn handle_metadata_signal(
        &self,
        player_id: PlayerId,
        signal: PropertyChanged<'_, HashMap<String, OwnedValue>>,
    ) {
        let Ok(metadata_map) = signal.get().await else {
            return;
        };

        let metadata = TrackMetadata::from(metadata_map);

        {
            let mut players = self.players.write().await;
            if let Some(tracker) = players.get_mut(&player_id) {
                tracker.state.metadata = metadata.clone();
            }
        }

        let _ = self.events_tx.send(PlayerEvent::MetadataChanged {
            player_id,
            metadata,
        });
    }

    async fn handle_loop_status_signal(
        &self,
        player_id: PlayerId,
        signal: PropertyChanged<'_, String>,
    ) {
        let Ok(status) = signal.get().await else {
            return;
        };

        let mode = LoopMode::from(status.as_str());

        {
            let mut players = self.players.write().await;
            if let Some(tracker) = players.get_mut(&player_id) {
                tracker.state.loop_mode = mode;
            }
        }

        let _ = self
            .events_tx
            .send(PlayerEvent::LoopModeChanged { player_id, mode });
    }

    async fn handle_shuffle_signal(&self, player_id: PlayerId, signal: PropertyChanged<'_, bool>) {
        let Ok(shuffle) = signal.get().await else {
            return;
        };

        let mode = ShuffleMode::from(shuffle);

        {
            let mut players = self.players.write().await;
            if let Some(tracker) = players.get_mut(&player_id) {
                tracker.state.shuffle_mode = mode;
            }
        }

        let _ = self
            .events_tx
            .send(PlayerEvent::ShuffleModeChanged { player_id, mode });
    }
}

impl Clone for PlayerMonitoring {
    fn clone(&self) -> Self {
        Self {
            players: Arc::clone(&self.players),
            events_tx: self.events_tx.clone(),
        }
    }
}
