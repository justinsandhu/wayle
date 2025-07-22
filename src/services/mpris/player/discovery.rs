use std::collections::HashMap;
use std::sync::Arc;

use futures::StreamExt;
use tokio::sync::RwLock;
use zbus::Connection;
use zbus::fdo::DBusProxy;

use super::handle::PlayerHandle;
use super::model::Player;
use crate::services::common::Property;
use crate::services::mpris::{MediaError, PlayerId};

use super::manager::PlayerManager;

/// MPRIS service name prefix for D-Bus.
const MPRIS_BUS_PREFIX: &str = "org.mpris.MediaPlayer2.";

/// Handles D-Bus player discovery and monitoring.
///
/// Discovers existing players on startup and monitors for new players
/// being added or removed from the system.
pub(crate) struct PlayerDiscovery;

impl PlayerDiscovery {
    /// Start player discovery process.
    ///
    /// Discovers all existing MPRIS players and sets up monitoring
    /// for future player additions/removals.
    ///
    /// # Errors
    ///
    /// Returns error if D-Bus proxy creation fails
    pub(crate) async fn start(
        connection: &Connection,
        players: &Arc<RwLock<HashMap<PlayerId, PlayerHandle>>>,
        player_list: &Property<Vec<Arc<Player>>>,
        active_player: &Property<Option<PlayerId>>,
        ignored_patterns: &[String],
    ) -> Result<(), MediaError> {
        let dbus_proxy = DBusProxy::new(connection)
            .await
            .map_err(|e| MediaError::InitializationFailed(format!("DBus proxy failed: {e}")))?;

        let names = dbus_proxy
            .list_names()
            .await
            .map_err(|e| MediaError::DbusError(e.into()))?;

        for name in names {
            if name.starts_with(MPRIS_BUS_PREFIX) && !Self::should_ignore(&name, ignored_patterns) {
                let player_id = PlayerId::from_bus_name(&name);
                if let Err(e) = PlayerManager::add_player(
                    connection,
                    players,
                    player_list,
                    active_player,
                    player_id,
                )
                .await
                {
                    tracing::warn!("Failed to add player {}: {}", name, e);
                }
            }
        }

        Self::spawn_name_monitoring(
            connection.clone(),
            Arc::clone(players),
            player_list.clone(),
            active_player.clone(),
            ignored_patterns.to_vec(),
        );

        Ok(())
    }

    /// Spawn background task to monitor D-Bus name changes.
    ///
    /// Watches for MPRIS players appearing and disappearing.
    fn spawn_name_monitoring(
        connection: Connection,
        players: Arc<RwLock<HashMap<PlayerId, PlayerHandle>>>,
        player_list: Property<Vec<Arc<Player>>>,
        active_player: Property<Option<PlayerId>>,
        ignored_patterns: Vec<String>,
    ) {
        tokio::spawn(async move {
            let Ok(dbus_proxy) = DBusProxy::new(&connection).await else {
                return;
            };

            let Ok(mut name_owner_changed) = dbus_proxy.receive_name_owner_changed().await else {
                return;
            };

            while let Some(signal) = name_owner_changed.next().await {
                let Ok(args) = signal.args() else { continue };

                if !args.name().starts_with(MPRIS_BUS_PREFIX) {
                    continue;
                }

                let player_id = PlayerId::from_bus_name(args.name());

                let is_player_added = args.old_owner().is_none() && args.new_owner().is_some();
                let is_player_removed = args.old_owner().is_some() && args.new_owner().is_none();

                if is_player_added && !Self::should_ignore(args.name(), &ignored_patterns) {
                    if let Err(e) = PlayerManager::add_player(
                        &connection,
                        &players,
                        &player_list,
                        &active_player,
                        player_id.clone(),
                    )
                    .await
                    {
                        tracing::warn!("Failed to add player {}: {}", player_id, e);
                    }
                } else if is_player_removed {
                    PlayerManager::remove_player(&players, &player_list, &active_player, player_id)
                        .await;
                }
            }
        });
    }

    fn should_ignore(bus_name: &str, ignored_patterns: &[String]) -> bool {
        ignored_patterns
            .iter()
            .any(|pattern| bus_name.contains(pattern))
    }
}
