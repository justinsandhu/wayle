//! NetworkManager flag types.

use bitflags::bitflags;

bitflags! {
    /// Device capability flags.
    pub struct NMDeviceCapabilities: u32 {
        /// No capabilities.
        const NONE = 0x00000000;
        /// NetworkManager supports this device.
        const NM_SUPPORTED = 0x00000001;
        /// Device supports carrier detection.
        const CARRIER_DETECT = 0x00000002;
        /// Device is a software device.
        const IS_SOFTWARE = 0x00000004;
        /// Device supports SR-IOV.
        const SRIOV = 0x00000008;
    }

    /// Access point capability flags.
    pub struct NM80211ApFlags: u32 {
        /// No flags.
        const NONE = 0x00000000;
        /// Access point supports privacy/encryption.
        const PRIVACY = 0x00000001;
        /// Access point supports Wi-Fi Protected Setup.
        const WPS = 0x00000002;
        /// Access point supports push-button WPS.
        const WPS_PBC = 0x00000004;
        /// Access point supports PIN-based WPS.
        const WPS_PIN = 0x00000008;
    }

    /// Access point security flags.
    pub struct NM80211ApSecurityFlags: u32 {
        /// No security.
        const NONE = 0x00000000;
        /// Pairwise 40-bit WEP encryption.
        const PAIR_WEP40 = 0x00000001;
        /// Pairwise 104-bit WEP encryption.
        const PAIR_WEP104 = 0x00000002;
        /// Pairwise TKIP encryption.
        const PAIR_TKIP = 0x00000004;
        /// Pairwise CCMP encryption.
        const PAIR_CCMP = 0x00000008;
        /// Group 40-bit WEP encryption.
        const GROUP_WEP40 = 0x00000010;
        /// Group 104-bit WEP encryption.
        const GROUP_WEP104 = 0x00000020;
        /// Group TKIP encryption.
        const GROUP_TKIP = 0x00000040;
        /// Group CCMP encryption.
        const GROUP_CCMP = 0x00000080;
        /// Pre-shared key authentication.
        const KEY_MGMT_PSK = 0x00000100;
        /// 802.1X authentication.
        const KEY_MGMT_802_1X = 0x00000200;
        /// Simultaneous Authentication of Equals.
        const KEY_MGMT_SAE = 0x00000400;
        /// Opportunistic Wireless Encryption.
        const KEY_MGMT_OWE = 0x00000800;
        /// Opportunistic Wireless Encryption transition mode.
        const KEY_MGMT_OWE_TM = 0x00001000;
        /// EAP Suite B 192-bit authentication.
        const KEY_MGMT_EAP_SUITE_B_192 = 0x00002000;
    }

    /// Wi-Fi device capabilities.
    pub struct NMDeviceWifiCapabilities: u32 {
        /// No capabilities.
        const NONE = 0x00000000;
        /// Device supports 40-bit WEP encryption.
        const CIPHER_WEP40 = 0x00000001;
        /// Device supports 104-bit WEP encryption.
        const CIPHER_WEP104 = 0x00000002;
        /// Device supports TKIP encryption.
        const CIPHER_TKIP = 0x00000004;
        /// Device supports AES/CCMP encryption.
        const CIPHER_CCMP = 0x00000008;
        /// Device supports WPA authentication.
        const WPA = 0x00000010;
        /// Device supports WPA2/RSN authentication.
        const RSN = 0x00000020;
        /// Device supports Access Point mode.
        const AP = 0x00000040;
        /// Device supports Ad-Hoc mode.
        const ADHOC = 0x00000080;
        /// Device reports valid frequency information.
        const FREQ_VALID = 0x00000100;
        /// Device supports 2.4GHz frequencies.
        const FREQ_2GHZ = 0x00000200;
        /// Device supports 5GHz frequencies.
        const FREQ_5GHZ = 0x00000400;
        /// Device supports mesh networking.
        const MESH = 0x00001000;
        /// Device supports WPA2/RSN in IBSS mode.
        const IBSS_RSN = 0x00002000;
    }

    /// Bluetooth device capabilities.
    pub struct NMBluetoothCapabilities: u32 {
        /// No capabilities.
        const NONE = 0x00000000;
        /// Device supports Dial-Up Networking.
        const DUN = 0x00000001;
        /// Device supports Network Access Point.
        const NAP = 0x00000002;
    }

    /// Modem capabilities.
    pub struct NMDeviceModemCapabilities: u32 {
        /// No capabilities.
        const NONE = 0x00000000;
        /// Modem supports analog telephone line.
        const POTS = 0x00000001;
        /// Modem supports CDMA/EVDO.
        const CDMA_EVDO = 0x00000002;
        /// Modem supports GSM/UMTS.
        const GSM_UMTS = 0x00000004;
        /// Modem supports LTE.
        const LTE = 0x00000008;
        /// Modem supports 5G.
        const G5 = 0x00000010;
    }

    /// Secret agent capabilities.
    pub struct NMSecretAgentCapabilities: u32 {
        /// No capabilities.
        const NONE = 0x00000000;
        /// Agent supports VPN hints for authentication.
        const VPN_HINTS = 0x00000001;
    }

    /// Secret agent get secrets flags.
    pub struct NMSecretAgentGetSecretsFlags: u32 {
        /// No special behavior.
        const NONE = 0x00000000;
        /// Allow user interaction to get secrets.
        const ALLOW_INTERACTION = 0x00000001;
        /// Request new secrets from the user.
        const REQUEST_NEW = 0x00000002;
        /// User initiated the secrets request.
        const USER_REQUESTED = 0x00000004;
        /// WPS push-button mode is active.
        const WPS_PBC_ACTIVE = 0x00000008;
        /// Only system secrets are requested.
        const ONLY_SYSTEM = 0x80000000;
        /// Suppress error messages.
        const NO_ERRORS = 0x40000000;
    }

    /// Settings add connection flags.
    pub struct NMSettingsAddConnection2Flags: u32 {
        /// No special behavior.
        const NONE = 0x00000000;
        /// Save connection to disk.
        const TO_DISK = 0x00000001;
        /// Connection is temporary/in-memory only.
        const IN_MEMORY = 0x00000002;
        /// Block autoconnect on the connection.
        const BLOCK_AUTOCONNECT = 0x00000020;
    }

    /// Settings update flags.
    pub struct NMSettingsUpdate2Flags: u32 {
        /// No special behavior.
        const NONE = 0x00000000;
        /// Save changes to disk.
        const TO_DISK = 0x00000001;
        /// Update in-memory configuration.
        const IN_MEMORY = 0x00000002;
        /// Detach the in-memory configuration from disk.
        const IN_MEMORY_DETACHED = 0x00000004;
        /// Only update in-memory configuration.
        const IN_MEMORY_ONLY = 0x00000008;
        /// Mark connection as volatile.
        const VOLATILE = 0x00000010;
        /// Block autoconnect on the connection.
        const BLOCK_AUTOCONNECT = 0x00000020;
        /// Don't reapply connection to devices.
        const NO_REAPPLY = 0x00000040;
    }

    /// Checkpoint creation flags.
    pub struct NMCheckpointCreateFlags: u32 {
        /// No special behavior.
        const NONE = 0x00000000;
        /// Destroy all existing checkpoints.
        const DESTROY_ALL = 0x00000001;
        /// Delete new connections on rollback.
        const DELETE_NEW_CONNECTIONS = 0x00000002;
        /// Disconnect new devices on rollback.
        const DISCONNECT_NEW_DEVICES = 0x00000004;
        /// Allow overlapping checkpoints.
        const ALLOW_OVERLAPPING = 0x00000008;
        /// Don't preserve external ports on rollback.
        const NO_PRESERVE_EXTERNAL_PORTS = 0x00000010;
        /// Track internal global DNS configuration.
        const TRACK_INTERNAL_GLOBAL_DNS = 0x00000020;
    }

    /// Settings connection flags.
    pub struct NMSettingsConnectionFlags: u32 {
        /// No flags.
        const NONE = 0x00000000;
        /// Connection has unsaved changes.
        const UNSAVED = 0x00000001;
        /// Connection was generated by NetworkManager.
        const NM_GENERATED = 0x00000002;
        /// Connection is volatile/temporary.
        const VOLATILE = 0x00000004;
        /// Connection is managed externally.
        const EXTERNAL = 0x00000008;
    }

    /// Device interface flags.
    pub struct NMDeviceInterfaceFlags: u32 {
        /// No flags.
        const NONE = 0x00000000;
        /// Interface is administratively up.
        const UP = 0x00000001;
        /// Physical link is up.
        const LOWER_UP = 0x00000002;
        /// Interface has carrier.
        const CARRIER = 0x00010000;
    }

    /// Activation state flags.
    pub struct NMActivationStateFlags: u32 {
        /// No flags.
        const NONE = 0x00000000;
        /// Device is a master.
        const IS_MASTER = 0x00000001;
        /// Device is a slave.
        const IS_SLAVE = 0x00000002;
        /// Layer 2 is ready.
        const LAYER2_READY = 0x00000004;
        /// IPv4 is ready.
        const IP4_READY = 0x00000008;
        /// IPv6 is ready.
        const IP6_READY = 0x00000010;
        /// Master has slave devices.
        const MASTER_HAS_SLAVES = 0x00000020;
        /// Activation lifetime bound to profile visibility.
        const LIFETIME_BOUND_TO_PROFILE_VISIBILITY = 0x00000040;
        /// Activation is managed externally.
        const EXTERNAL = 0x00000080;
    }
}

