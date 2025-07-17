use std::{sync::Arc, time::Duration};

use async_stream::stream;
use futures::Stream;

use crate::services::mpris::{
    LoopMode, MediaError, MprisMediaService, PlaybackState, PlayerEvent, PlayerId, PlayerInfo,
    PlayerState, ShuffleMode, TrackMetadata,
};

/// Reactive data streams for media players
pub trait PlayerStreams {
    /// Stream of currently available media players
    fn players(&self) -> impl Stream<Item = Vec<PlayerId>> + Send;

    /// Stream of player information for a specific player
    fn player_info(
        &self,
        player_id: PlayerId,
    ) -> impl Stream<Item = Result<PlayerInfo, MediaError>> + Send;

    /// Stream of playback state changes for a specific player
    fn playback_state(&self, player_id: PlayerId) -> impl Stream<Item = PlaybackState> + Send;

    /// Stream of playback position updates for a specific player
    fn position(&self, player_id: PlayerId) -> impl Stream<Item = Duration> + Send;

    /// Stream of track metadata changes for a specific player
    fn metadata(&self, player_id: PlayerId) -> impl Stream<Item = TrackMetadata> + Send;

    /// Stream of loop mode changes for a specific player
    fn loop_mode(&self, player_id: PlayerId) -> impl Stream<Item = LoopMode> + Send;

    /// Stream of shuffle mode changes for a specific player
    fn shuffle_mode(&self, player_id: PlayerId) -> impl Stream<Item = ShuffleMode> + Send;

    /// Stream of complete player state changes for a specific player
    fn player_state(&self, player_id: PlayerId) -> impl Stream<Item = PlayerState> + Send;
}

impl PlayerStreams for MprisMediaService {
    fn players(&self) -> impl Stream<Item = Vec<PlayerId>> + Send {
        let mut rx = self.player_list_tx.subscribe();

        stream! {
            let current_players: Vec<PlayerId> = {
                let players = self.player_manager.players.read().await;
                players.keys().cloned().collect()
            };
            yield current_players;

            while let Ok(players) = rx.recv().await {
                yield players;
            }
        }
    }

    fn player_info(
        &self,
        player_id: PlayerId,
    ) -> impl Stream<Item = Result<PlayerInfo, MediaError>> + Send {
        let players = Arc::clone(&self.player_manager.players);
        let mut events_rx = self.events_tx.subscribe();

        stream! {
            let current_info = {
                let players_guard = players.read().await;
                players_guard.get(&player_id).map(|tracker| tracker.info.clone())
            };

            if let Some(info) = current_info {
                yield Ok(info);
            }

            while let Ok(event) = events_rx.recv().await {
                match event {
                    PlayerEvent::PlayerRemoved(id) if id == player_id => {
                        return;
                    }
                    PlayerEvent::PlayerAdded(info) if info.id != player_id => {
                        continue;
                    }
                    PlayerEvent::PlayerAdded(info) => {
                        yield Ok(info);
                    }
                    PlayerEvent::CapabilitiesChanged { player_id: id, .. } if id != player_id => {
                        continue;
                    }
                    PlayerEvent::CapabilitiesChanged { capabilities, .. } => {
                        let updated_info = {
                            let players_guard = players.read().await;
                            players_guard.get(&player_id).map(|tracker| {
                                let mut info = tracker.info.clone();
                                info.capabilities = capabilities;
                                info
                            })
                        };

                        if let Some(info) = updated_info {
                            yield Ok(info);
                        }
                    }
                    _ => continue,
                }
            }
        }
    }

    fn playback_state(&self, player_id: PlayerId) -> impl Stream<Item = PlaybackState> + Send {
        let players = Arc::clone(&self.player_manager.players);
        let mut events_rx = self.events_tx.subscribe();

        stream! {
            let current_state = {
                let players_guard = players.read().await;
                players_guard.get(&player_id).map(|tracker| tracker.last_playback_state.clone())
            };

            if let Some(state) = current_state {
                yield state;
            }

            while let Ok(event) = events_rx.recv().await {
                let PlayerEvent::PlaybackStateChanged { player_id: id, state } = event else {
                    continue;
                };

                if id != player_id {
                    continue;
                }

                yield state;
            }
        }
    }

    fn position(&self, player_id: PlayerId) -> impl Stream<Item = Duration> + Send {
        let players = self.player_manager.players.clone();
        let mut events_rx = self.events_tx.subscribe();

        stream! {
            let current_position = {
                let players_guard = players.read().await;
                players_guard.get(&player_id).map(|tracker| tracker.last_position)
            };

            if let Some(position) = current_position {
                yield position;
            }

            while let Ok(event) = events_rx.recv().await {
                let PlayerEvent::PositionChanged { player_id: id, position } = event else {
                    continue;
                };

                if id != player_id {
                    continue;
                }

                yield position;
            }
        }
    }

    fn metadata(&self, player_id: PlayerId) -> impl Stream<Item = TrackMetadata> + Send {
        let players = self.player_manager.players.clone();
        let mut events_rx = self.events_tx.subscribe();

        stream! {
            let current_metadata = {
                let players_guard = players.read().await;
                players_guard.get(&player_id).map(|tracker| tracker.last_metadata.clone())
            };

            if let Some(metadata) = current_metadata {
                yield metadata;
            }

            while let Ok(event) = events_rx.recv().await {
                let PlayerEvent::MetadataChanged { player_id: id, metadata } = event else {
                    continue;
                };

                if id != player_id {
                    continue;
                }

                yield metadata;
            }
        }
    }

    fn loop_mode(&self, player_id: PlayerId) -> impl Stream<Item = LoopMode> + Send {
        let players = self.player_manager.players.clone();
        let mut events_rx = self.events_tx.subscribe();

        stream! {
            let current_mode = {
                let players_guard = players.read().await;
                players_guard.get(&player_id).map(|tracker| tracker.last_loop_mode.clone())
            };

            if let Some(mode) = current_mode {
                yield mode;
            }

            while let Ok(event) = events_rx.recv().await {
                let PlayerEvent::LoopModeChanged { player_id: id, mode } = event else {
                    continue;
                };

                if id != player_id {
                    continue;
                }

                yield mode;
            }
        }
    }

    fn shuffle_mode(&self, player_id: PlayerId) -> impl Stream<Item = ShuffleMode> + Send {
        let players = self.player_manager.players.clone();
        let mut events_rx = self.events_tx.subscribe();

        stream! {
            let current_mode = {
                let players_guard = players.read().await;
                players_guard.get(&player_id).map(|tracker| tracker.last_shuffle_mode.clone())
            };

            if let Some(mode) = current_mode {
                yield mode;
            }

            while let Ok(event) = events_rx.recv().await {
                let PlayerEvent::ShuffleModeChanged { player_id: id, mode } = event else {
                    continue;
                };

                if id != player_id {
                    continue;
                }

                yield mode;
            }
        }
    }

    fn player_state(&self, player_id: PlayerId) -> impl Stream<Item = PlayerState> + Send {
        let players = self.player_manager.players.clone();
        let mut events_rx = self.events_tx.subscribe();

        stream! {
            let current_state = {
                let players_guard = players.read().await;
                players_guard.get(&player_id).map(|tracker| PlayerState {
                    player_info: tracker.info.clone(),
                    playback_state: tracker.last_playback_state.clone(),
                    metadata: tracker.last_metadata.clone(),
                    position: tracker.last_position,
                    loop_mode: tracker.last_loop_mode.clone(),
                    shuffle_mode: tracker.last_shuffle_mode.clone(),
                })
            };

            if let Some(state) = current_state {
                yield state;
            }

            while let Ok(event) = events_rx.recv().await {
                match event {
                    PlayerEvent::PlayerRemoved(id) if id == player_id => {
                        return;
                    }
                    PlayerEvent::PlaybackStateChanged { player_id: id, .. } |
                    PlayerEvent::MetadataChanged { player_id: id, .. } |
                    PlayerEvent::PositionChanged { player_id: id, .. } |
                    PlayerEvent::LoopModeChanged { player_id: id, .. } |
                    PlayerEvent::ShuffleModeChanged { player_id: id, .. } |
                    PlayerEvent::CapabilitiesChanged { player_id: id, .. } if id != player_id => {
                        continue;
                    }
                    PlayerEvent::PlaybackStateChanged { .. } |
                    PlayerEvent::MetadataChanged { .. } |
                    PlayerEvent::PositionChanged { .. } |
                    PlayerEvent::LoopModeChanged { .. } |
                    PlayerEvent::ShuffleModeChanged { .. } |
                    PlayerEvent::CapabilitiesChanged { .. } => {
                        let updated_state = {
                            let players_guard = players.read().await;
                            players_guard.get(&player_id).map(|tracker| PlayerState {
                                player_info: tracker.info.clone(),
                                playback_state: tracker.last_playback_state.clone(),
                                metadata: tracker.last_metadata.clone(),
                                position: tracker.last_position,
                                loop_mode: tracker.last_loop_mode.clone(),
                                shuffle_mode: tracker.last_shuffle_mode.clone(),
                            })
                        };

                        if let Some(state) = updated_state {
                            yield state;
                        }
                    }
                    _ => continue,
                }
            }
        }
    }
}
