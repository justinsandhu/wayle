use std::sync::Arc;

use tracing::warn;
use zbus::{Connection, zvariant::OwnedObjectPath};

use crate::services::{
    common::Property,
    network_manager::{AccessPointProxy, NM80211ApFlags, NM80211ApSecurityFlags, NM80211Mode},
};

mod monitoring;
mod types;

use monitoring::AccessPointMonitor;
pub use types::{BSSID, NetworkIdentifier, SSID, SecurityType};

/// WiFi access point representation.
///
/// Provides information about a detected WiFi access point including its
/// security configuration, signal strength, frequency, and identification.
/// Access points are discovered and monitored through the WiFi device interface.
#[derive(Debug, Clone)]
pub struct AccessPoint {
    /// D-Bus object path for this access point.
    pub(crate) path: OwnedObjectPath,
    /// Flags describing the capabilities of the access point. See NM80211ApFlags.
    pub flags: Property<NM80211ApFlags>,

    /// Flags describing the access point's capabilities according to WPA (Wifi Protected Access).
    /// See NM80211ApSecurityFlags.
    pub wpa_flags: Property<NM80211ApSecurityFlags>,

    /// Flags describing the access point's capabilities according to the
    /// RSN (Robust Secure Network) protocol. See NM80211ApSecurityFlags.
    pub rsn_flags: Property<NM80211ApSecurityFlags>,

    /// The Service Set Identifier identifying the access point.
    /// The SSID is a binary array to support non-UTF-8 SSIDs.
    pub ssid: Property<SSID>,

    /// The radio channel frequency in use by the access point, in MHz.
    pub frequency: Property<u32>,

    /// The hardware address (BSSID) of the access point.
    pub bssid: Property<BSSID>,

    /// Describes the operating mode of the access point.
    pub mode: Property<NM80211Mode>,

    /// The maximum bitrate this access point is capable of, in kilobits/second (Kb/s).
    pub max_bitrate: Property<u32>,

    /// The current signal quality of the access point, in percent.
    pub strength: Property<u8>,

    /// The timestamp (in CLOCK_BOOTTIME seconds) for the last time the access point
    /// was found in scan results. A value of -1 means the access point has never
    /// been found in scan results.
    pub last_seen: Property<i32>,

    /// Simplified security type derived from flags.
    ///
    /// Provides a user-friendly classification of the AP's security.
    pub security: Property<SecurityType>,

    /// Whether this is a hidden network (non-broadcasting SSID).
    pub is_hidden: Property<bool>,
}

impl PartialEq for AccessPoint {
    fn eq(&self, other: &Self) -> bool {
        self.bssid.get() == other.bssid.get()
    }
}

impl AccessPoint {
    /// Get a snapshot of the current access point state (no monitoring).
    pub async fn get(connection: Connection, path: OwnedObjectPath) -> Option<Arc<Self>> {
        Self::create_from_path(connection, path).await
    }

    /// Get a live-updating access point instance (with monitoring).
    pub async fn get_live(connection: Connection, path: OwnedObjectPath) -> Option<Arc<Self>> {
        let access_point = Self::create_from_path(connection.clone(), path.clone()).await?;

        AccessPointMonitor::start(access_point.clone(), connection, path).await;

        Some(access_point)
    }

    /// Creates an access point instance from a D-Bus path and connection.
    ///
    /// Retrieves all access point properties from NetworkManager via D-Bus
    /// and initializes reactive properties for each value. Returns None if
    /// the access point doesn't exist at the given path.
    async fn create_from_path(connection: Connection, path: OwnedObjectPath) -> Option<Arc<Self>> {
        let ap_proxy = AccessPointProxy::new(&connection, path.clone())
            .await
            .ok()?;

        if ap_proxy.strength().await.is_err() {
            warn!("Access point at path '{}' does not exist.", path.clone());
            return None;
        }

        let (
            flags,
            wpa_flags,
            rsn_flags,
            ssid,
            frequency,
            hw_address,
            mode,
            max_bitrate,
            strength,
            last_seen,
        ) = tokio::join!(
            ap_proxy.flags(),
            ap_proxy.wpa_flags(),
            ap_proxy.rsn_flags(),
            ap_proxy.ssid(),
            ap_proxy.frequency(),
            ap_proxy.hw_address(),
            ap_proxy.mode(),
            ap_proxy.max_bitrate(),
            ap_proxy.strength(),
            ap_proxy.last_seen(),
        );

        let flags = NM80211ApFlags::from_bits_truncate(flags.unwrap_or_default());
        let wpa_flags = NM80211ApSecurityFlags::from_bits_truncate(wpa_flags.unwrap_or_default());
        let rsn_flags = NM80211ApSecurityFlags::from_bits_truncate(rsn_flags.unwrap_or_default());
        let ssid = SSID::new(ssid.unwrap_or_default());
        let frequency = frequency.unwrap_or_default();
        let hw_address = BSSID::new(hw_address.unwrap_or_default().into_bytes());
        let mode = NM80211Mode::from_u32(mode.unwrap_or_default());
        let max_bitrate = max_bitrate.unwrap_or_default();
        let strength = strength.unwrap_or_default();
        let last_seen = last_seen.unwrap_or(-1);

        let security = SecurityType::from_flags(flags, wpa_flags, rsn_flags);
        let is_hidden = ssid.is_empty();

        Some(Arc::new(Self {
            path,
            flags: Property::new(flags),
            wpa_flags: Property::new(wpa_flags),
            rsn_flags: Property::new(rsn_flags),
            ssid: Property::new(ssid),
            frequency: Property::new(frequency),
            bssid: Property::new(hw_address),
            mode: Property::new(mode),
            max_bitrate: Property::new(max_bitrate),
            strength: Property::new(strength),
            last_seen: Property::new(last_seen),
            security: Property::new(security),
            is_hidden: Property::new(is_hidden),
        }))
    }
}
