use std::fmt::{self, Display};

use crate::services::{
    common::Property,
    network_manager::{NM80211ApFlags, NM80211ApSecurityFlags, types::NM80211Mode},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(clippy::upper_case_acronyms)]
pub enum SecurityType {
    None,
    WEP,
    WPA,
    WPA2,
    WPA3,
    Enterprise,
}
impl SecurityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "Open",
            Self::WEP => "WEP",
            Self::WPA => "WPA",
            Self::WPA2 => "WPA2",
            Self::WPA3 => "WPA3",
            Self::Enterprise => "Enterprise",
        }
    }
}

impl Display for SecurityType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(clippy::upper_case_acronyms)]
pub struct NetworkIdentifier(Vec<u8>);

#[allow(clippy::upper_case_acronyms)]
pub type SSID = NetworkIdentifier;

#[allow(clippy::upper_case_acronyms)]
pub type BSSID = NetworkIdentifier;

impl NetworkIdentifier {
    /// Creates a new SSID from raw bytes.
    ///
    /// SSIDs are typically UTF-8 encoded strings, but the 802.11 standard allows
    /// arbitrary byte sequences up to 32 octets.
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// Returns the SSID as a UTF-8 string, replacing invalid sequences with �.
    ///
    /// Most SSIDs are valid UTF-8, but some routers may use non-standard encodings.
    /// Ensures a displayable string is always returned.
    pub fn as_str(&self) -> String {
        String::from_utf8_lossy(&self.0).to_string()
    }

    /// Returns the raw bytes of the SSID.
    ///
    /// Provides the exact byte sequence for network operations
    /// or byte-for-byte comparisons.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Checks if this is a hidden network (non-broadcasting SSID).
    ///
    /// Hidden networks have an empty SSID in beacon frames. The actual SSID
    /// is only revealed after authentication.
    pub fn is_hidden(&self) -> bool {
        self.0.is_empty()
    }
}

/// Represents a wireless access point detected by NetworkManager.
///
/// Access points are identified by their BSSID (hardware address) and provide
/// information about available wireless networks including signal strength,
/// security capabilities, and network properties.
#[derive(Debug, Clone)]
pub struct AccessPoint {
    /// The Service Set Identifier identifying the access point.
    /// The SSID is a binary array to support non-UTF-8 SSIDs.
    pub ssid: Property<SSID>,

    /// The hardware address (BSSID) of the access point.
    pub bssid: Property<BSSID>,

    /// Flags describing the capabilities of the access point. See NM80211ApFlags.
    pub flags: Property<NM80211ApFlags>,

    /// Flags describing the access point's capabilities according to WPA
    /// (Wifi Protected Access). See NM80211ApSecurityFlags.
    pub wpa_flags: Property<NM80211ApSecurityFlags>,

    /// Flags describing the access point's capabilities according to the RSN
    /// (Robust Secure Network) protocol. See NM80211ApSecurityFlags.
    pub rsn_flags: Property<NM80211ApSecurityFlags>,

    /// The radio channel frequency in use by the access point, in MHz.
    pub frequency: Property<u32>,

    /// Describes the operating mode of the access point.
    pub mode: Property<NM80211Mode>,

    /// The maximum bitrate this access point is capable of, in kilobits/second (Kb/s).
    pub max_bitrate: Property<u32>,

    /// The current signal quality of the access point, in percent.
    pub strength: Property<u8>,

    /// The timestamp (in CLOCK_BOOTTIME seconds) for the last time the access point
    /// was found in scan results.
    ///
    /// A value of -1 means the access point has never been found in scan results.
    pub last_seen: Property<i32>,

    /// Simplified security type derived from flags.
    ///
    /// Provides a user-friendly classification of the AP's security.
    pub security: Property<SecurityType>,
}

impl PartialEq for AccessPoint {
    fn eq(&self, other: &Self) -> bool {
        self.bssid.get() == other.bssid.get()
    }
}

impl AccessPoint {
    pub fn new(bssid: BSSID) -> Self {
        Self {
            ssid: Property::new(SSID::new(vec![])),
            bssid: Property::new(bssid),

            flags: Property::new(NM80211ApFlags::NONE),
            wpa_flags: Property::new(NM80211ApSecurityFlags::NONE),
            rsn_flags: Property::new(NM80211ApSecurityFlags::NONE),

            frequency: Property::new(0),
            mode: Property::new(NM80211Mode::Unknown),
            max_bitrate: Property::new(0),
            strength: Property::new(0),
            last_seen: Property::new(-1),
            security: Property::new(SecurityType::None),
        }
    }
}
