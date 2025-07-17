use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use tokio::sync::{RwLock, broadcast};
use zbus::{proxy::PropertyChanged, zvariant::OwnedValue};

use super::{
    LoopMode, MediaError, MediaPlayer2PlayerProxy, PlaybackState, PlayerEvent, PlayerEventSender,
    PlayerId, ShuffleMode, TrackMetadata, player::state::PlayerStateTracker, utils,
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
        if let Ok(position) = signal.get().await {
            let duration = utils::from_mpris_micros(position);
            self.handle_position_changed(player_id, duration).await;
        }
    }

    async fn handle_playback_status_signal(
        &self,
        player_id: PlayerId,
        signal: PropertyChanged<'_, String>,
    ) {
        if let Ok(status) = signal.get().await {
            let state = PlaybackState::from(status.as_str());
            self.handle_playback_state_changed(player_id, state).await;
        }
    }

    async fn handle_metadata_signal(
        &self,
        player_id: PlayerId,
        signal: PropertyChanged<'_, HashMap<String, OwnedValue>>,
    ) {
        if let Ok(metadata_map) = signal.get().await {
            let metadata = TrackMetadata::from(metadata_map);
            self.handle_metadata_changed(player_id, metadata).await;
        }
    }

    async fn handle_loop_status_signal(
        &self,
        player_id: PlayerId,
        signal: PropertyChanged<'_, String>,
    ) {
        if let Ok(status) = signal.get().await {
            let mode = LoopMode::from(status.as_str());
            self.handle_loop_mode_changed(player_id, mode).await;
        }
    }

    async fn handle_shuffle_signal(&self, player_id: PlayerId, signal: PropertyChanged<'_, bool>) {
        if let Ok(shuffle) = signal.get().await {
            let mode = ShuffleMode::from(shuffle);
            self.handle_shuffle_mode_changed(player_id, mode).await;
        }
    }

    async fn handle_position_changed(&self, player_id: PlayerId, position: Duration) {
        {
            let mut players = self.players.write().await;
            if let Some(tracker) = players.get_mut(&player_id) {
                tracker.last_position = position;
            }
        }

        let _ = self.events_tx.send(PlayerEvent::PositionChanged {
            player_id,
            position,
        });
    }

    async fn handle_playback_state_changed(&self, player_id: PlayerId, state: PlaybackState) {
        {
            let mut players = self.players.write().await;
            if let Some(tracker) = players.get_mut(&player_id) {
                tracker.last_playback_state = state;
            }
        }

        let _ = self
            .events_tx
            .send(PlayerEvent::PlaybackStateChanged { player_id, state });
    }

    async fn handle_metadata_changed(&self, player_id: PlayerId, metadata: TrackMetadata) {
        {
            let mut players = self.players.write().await;
            if let Some(tracker) = players.get_mut(&player_id) {
                tracker.last_metadata = metadata.clone();
            }
        }

        let _ = self.events_tx.send(PlayerEvent::MetadataChanged {
            player_id,
            metadata,
        });
    }

    async fn handle_loop_mode_changed(&self, player_id: PlayerId, mode: LoopMode) {
        {
            let mut players = self.players.write().await;
            if let Some(tracker) = players.get_mut(&player_id) {
                tracker.last_loop_mode = mode;
            }
        }

        let _ = self
            .events_tx
            .send(PlayerEvent::LoopModeChanged { player_id, mode });
    }

    async fn handle_shuffle_mode_changed(&self, player_id: PlayerId, mode: ShuffleMode) {
        {
            let mut players = self.players.write().await;
            if let Some(tracker) = players.get_mut(&player_id) {
                tracker.last_shuffle_mode = mode;
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
