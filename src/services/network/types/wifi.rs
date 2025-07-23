//! NetworkManager Wi-Fi types.

/// Wi-Fi operation modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NM80211Mode {
    Unknown = 0,
    Adhoc = 1,
    Infra = 2,
    Ap = 3,
    Mesh = 4,
}