use std::{future::Future, pin::Pin, time::Duration};

use async_stream::stream;
use async_trait::async_trait;
use futures::Stream;
use zbus::zvariant::ObjectPath;

use super::{
    LoopMode, MediaError, MediaService, PlaybackState, PlayerEvent, PlayerId, PlayerInfo,
    PlayerState, ShuffleMode, TrackMetadata, mpris::MprisMediaService, utils,
};

#[async_trait]
impl MediaService for MprisMediaService {
    type Error = MediaError;

    fn players(&self) -> impl Stream<Item = Vec<PlayerId>> + Send {
        let mut rx = self.player_list_tx.subscribe();

        stream! {
            let current_players: Vec<PlayerId> = {
                let players = self.players.read().await;
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
    ) -> impl Stream<Item = Result<PlayerInfo, Self::Error>> + Send {
        let players = self.players.clone();
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
        let players = self.players.clone();
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
        let players = self.players.clone();
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
        let players = self.players.clone();
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
        let players = self.players.clone();
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
        let players = self.players.clone();
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
        let players = self.players.clone();
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

    async fn play_pause(&self, player_id: PlayerId) -> Result<(), Self::Error> {
        let proxy = self.get_player_proxy(&player_id).await?;
        proxy.play_pause().await.map_err(MediaError::DbusError)
    }

    async fn next(&self, player_id: PlayerId) -> Result<(), Self::Error> {
        let proxy = self.get_player_proxy(&player_id).await?;
        proxy.next().await.map_err(MediaError::DbusError)
    }

    async fn previous(&self, player_id: PlayerId) -> Result<(), Self::Error> {
        let proxy = self.get_player_proxy(&player_id).await?;
        proxy.previous().await.map_err(MediaError::DbusError)
    }

    async fn seek(&self, player_id: PlayerId, position: Duration) -> Result<(), Self::Error> {
        let proxy = self.get_player_proxy(&player_id).await?;

        let track_id_str = {
            let players = self.players.read().await;
            let Some(tracker) = players.get(&player_id) else {
                return Err(MediaError::PlayerNotFound(player_id));
            };

            if let Some(length) = tracker.last_metadata.length {
                if position > length {
                    return Err(MediaError::InvalidSeekPosition {
                        position,
                        length: Some(length),
                    });
                }
            }

            tracker
                .last_metadata
                .track_id
                .clone()
                .unwrap_or_else(|| "/".to_string())
        };

        let track_id = ObjectPath::try_from(track_id_str.as_str())
            .map_err(|e| MediaError::DbusError(e.into()))?;
        let position_micros = utils::to_mpris_micros(position);

        proxy
            .set_position(&track_id, position_micros)
            .await
            .map_err(MediaError::DbusError)
    }

    async fn toggle_loop(&self, player_id: PlayerId) -> Result<(), Self::Error> {
        let proxy = self.get_player_proxy(&player_id).await?;

        let current_mode = self.get_current_loop_mode(&proxy).await?;
        let next_mode = match current_mode {
            LoopMode::None => LoopMode::Track,
            LoopMode::Track => LoopMode::Playlist,
            LoopMode::Playlist => LoopMode::None,
            LoopMode::Unsupported => {
                return Err(MediaError::UnsupportedOperation {
                    player: player_id,
                    operation: "loop".to_string(),
                });
            }
        };

        let mpris_mode: &str = next_mode.into();
        proxy
            .set_loop_status(mpris_mode)
            .await
            .map_err(MediaError::DbusError)
    }

    async fn toggle_shuffle(&self, player_id: PlayerId) -> Result<(), Self::Error> {
        let proxy = self.get_player_proxy(&player_id).await?;

        let current_shuffle = proxy.shuffle().await.map_err(MediaError::DbusError)?;

        proxy
            .set_shuffle(!current_shuffle)
            .await
            .map_err(MediaError::DbusError)
    }

    async fn active_player(&self) -> Option<PlayerId> {
        let active = self.active_player.read().await;

        if let Some(ref player_id) = *active {
            let players = self.players.read().await;

            if players.contains_key(player_id) {
                return active.clone();
            }

            self.find_and_set_fallback_player().await
        } else {
            None
        }
    }

    async fn set_active_player(&self, player_id: Option<PlayerId>) -> Result<(), Self::Error> {
        if let Some(ref id) = player_id {
            let players = self.players.read().await;

            if !players.contains_key(id) {
                return Err(MediaError::PlayerNotFound(id.clone()));
            }
        }

        let mut active = self.active_player.write().await;
        *active = player_id.clone();

        self.save_active_player_to_file(player_id).await;
        Ok(())
    }

    async fn control_active_player<F, R>(&self, action: F) -> Option<Result<R, Self::Error>>
    where
        F: FnOnce(PlayerId) -> Pin<Box<dyn Future<Output = Result<R, Self::Error>> + Send>> + Send,
        R: Send,
    {
        let active_id = self.active_player().await?;
        Some(action(active_id).await)
    }
}
