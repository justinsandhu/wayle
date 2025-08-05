use std::sync::{Arc, Weak};

use futures::StreamExt;
use tracing::debug;
use zbus::{Connection, zvariant::OwnedObjectPath};

use crate::services::network_manager::{
    NMConnectionSettingsFlags, NetworkError, proxy::settings::connection::SettingsConnectionProxy,
};

use super::ConnectionSettings;

/// Monitors D-Bus properties and updates the reactive SettingsConnection model.
pub(crate) struct ConnectionSettingsMonitor;

impl ConnectionSettingsMonitor {
    pub(super) async fn start(
        settings: Arc<ConnectionSettings>,
        connection: Connection,
        path: OwnedObjectPath,
    ) -> Result<(), NetworkError> {
        let weak = Arc::downgrade(&settings);

        let proxy = SettingsConnectionProxy::new(&connection, path)
            .await
            .map_err(NetworkError::DbusError)?;

        tokio::spawn(async move {
            Self::monitor(weak, proxy).await;
        });

        Ok(())
    }

    async fn monitor(weak: Weak<ConnectionSettings>, proxy: SettingsConnectionProxy<'static>) {
        let mut unsaved_changed = proxy.receive_unsaved_changed().await;
        let mut flags_changed = proxy.receive_flags_changed().await;
        let mut filename_changed = proxy.receive_filename_changed().await;

        loop {
            let Some(settings) = weak.upgrade() else {
                debug!("SettingsConnection dropped, stopping monitor");
                return;
            };

            tokio::select! {
                Some(change) = unsaved_changed.next() => {
                    if let Ok(value) = change.get().await {
                        settings.unsaved.set(value);
                    }
                }
                Some(change) = flags_changed.next() => {
                    if let Ok(value) = change.get().await {
                        settings.flags.set(NMConnectionSettingsFlags::from_bits_truncate(value));
                    }
                }
                Some(change) = filename_changed.next() => {
                    if let Ok(value) = change.get().await {
                        settings.filename.set(value);
                    }
                }
                else => {
                    debug!("All property streams ended for SettingsConnection");
                    break;
                }
            }

            drop(settings);
        }
    }
}
