use std::sync::Arc;

use futures::StreamExt;
use tokio::task::JoinHandle;
use tracing::debug;

use crate::services::network_manager::{
    AccessPointProxy, NM80211ApFlags, NM80211ApSecurityFlags, NM80211Mode,
};

use super::{AccessPoint, BSSID, SSID, SecurityType};

/// Monitors D-Bus properties and updates the reactive AccessPoint model.
pub(crate) struct AccessPointMonitor {
    handle: JoinHandle<()>,
}

impl AccessPointMonitor {
    /// Start monitoring an access point's D-Bus properties.
    ///
    /// Returns a handle that will abort the monitoring task when dropped.
    pub fn start(access_point: Arc<AccessPoint>, proxy: AccessPointProxy<'static>) -> Self {
        debug!("Starting property monitoring for access point");

        let handle = tokio::spawn(async move {
            Self::monitor_properties(access_point, proxy).await;
        });

        Self { handle }
    }

    #[allow(clippy::cognitive_complexity)]
    async fn monitor_properties(access_point: Arc<AccessPoint>, proxy: AccessPointProxy<'static>) {
        let mut flag_changes = proxy.receive_flags_changed().await;
        let mut wpa_flags_changes = proxy.receive_wpa_flags_changed().await;
        let mut rsn_flags_changes = proxy.receive_rsn_flags_changed().await;
        let mut ssid_changes = proxy.receive_ssid_changed().await;
        let mut frequency_changes = proxy.receive_frequency_changed().await;
        let mut hw_address_changes = proxy.receive_hw_address_changed().await;
        let mut mode_changes = proxy.receive_mode_changed().await;
        let mut max_bitrate_changes = proxy.receive_max_bitrate_changed().await;
        let mut strength_changes = proxy.receive_strength_changed().await;
        let mut last_seen_changes = proxy.receive_last_seen_changed().await;

        loop {
            tokio::select! {
                Some(change) = flag_changes.next() => {
                    if let Ok(new_flags) = change.get().await {
                        let flags = NM80211ApFlags::from_bits_truncate(new_flags);
                        access_point.flags.set(flags);

                        let security = SecurityType::from_flags(
                            flags,
                            access_point.wpa_flags.get(),
                            access_point.rsn_flags.get(),
                        );
                        access_point.security.set(security);
                    }
                }
                Some(change) = wpa_flags_changes.next() => {
                    if let Ok(new_flags) = change.get().await {
                        let wpa_flags = NM80211ApSecurityFlags::from_bits_truncate(new_flags);
                        access_point.wpa_flags.set(wpa_flags);

                        let security = SecurityType::from_flags(
                            access_point.flags.get(),
                            wpa_flags,
                            access_point.rsn_flags.get(),
                        );
                        access_point.security.set(security);
                    }
                }
                Some(change) = rsn_flags_changes.next() => {
                    if let Ok(new_flags) = change.get().await {
                        let rsn_flags = NM80211ApSecurityFlags::from_bits_truncate(new_flags);
                        access_point.rsn_flags.set(rsn_flags);

                        let security = SecurityType::from_flags(
                            access_point.flags.get(),
                            access_point.wpa_flags.get(),
                            rsn_flags,
                        );
                        access_point.security.set(security);
                    }
                }
                Some(change) = ssid_changes.next() => {
                    if let Ok(new_ssid) = change.get().await {
                        let ssid = SSID::new(new_ssid);
                        let is_hidden = ssid.is_empty();
                        access_point.ssid.set(ssid);
                        access_point.is_hidden.set(is_hidden);
                    }
                }
                Some(change) = frequency_changes.next() => {
                    if let Ok(new_frequency) = change.get().await {
                        access_point.frequency.set(new_frequency);
                    }
                }
                Some(change) = hw_address_changes.next() => {
                    if let Ok(new_hw_address) = change.get().await {
                        let bssid = BSSID::new(new_hw_address.into_bytes());
                        access_point.bssid.set(bssid);
                    }
                }
                Some(change) = mode_changes.next() => {
                    if let Ok(new_mode) = change.get().await {
                        let mode = NM80211Mode::from_u32(new_mode);
                        access_point.mode.set(mode);
                    }
                }
                Some(change) = max_bitrate_changes.next() => {
                    if let Ok(new_bitrate) = change.get().await {
                        access_point.max_bitrate.set(new_bitrate);
                    }
                }
                Some(change) = strength_changes.next() => {
                    if let Ok(new_strength) = change.get().await {
                        access_point.strength.set(new_strength);
                    }
                }
                Some(change) = last_seen_changes.next() => {
                    if let Ok(new_last_seen) = change.get().await {
                        access_point.last_seen.set(new_last_seen);
                    }
                }
                else => {
                    debug!("All property streams ended for access point");
                    break;
                }
            }
        }

        debug!("Property monitoring ended for access point");
    }
}

impl Drop for AccessPointMonitor {
    fn drop(&mut self) {
        self.handle.abort();
    }
}
