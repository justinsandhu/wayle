//! NetworkManager connectivity types.

/// Internet connectivity state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NMConnectivityState {
    Unknown = 0,
    None = 1,
    Portal = 2,
    Limited = 3,
    Full = 4,
}

/// Metered connection status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NMMetered {
    Unknown = 0,
    Yes = 1,
    No = 2,
    GuessYes = 3,
    GuessNo = 4,
}
