//! NetworkManager state types.

/// Overall network state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NMState {
    Unknown = 0,
    Asleep = 10,
    Disconnected = 20,
    Disconnecting = 30,
    Connecting = 40,
    ConnectedLocal = 50,
    ConnectedSite = 60,
    ConnectedGlobal = 70,
}

/// Device-specific states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NMDeviceState {
    Unknown = 0,
    Unmanaged = 10,
    Unavailable = 20,
    Disconnected = 30,
    Prepare = 40,
    Config = 50,
    NeedAuth = 60,
    IpConfig = 70,
    IpCheck = 80,
    Secondaries = 90,
    Activated = 100,
    Deactivating = 110,
    Failed = 120,
}

/// States for active connections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NMActiveConnectionState {
    Unknown = 0,
    Activating = 1,
    Activated = 2,
    Deactivating = 3,
    Deactivated = 4,
}

/// VPN connection states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NMVpnConnectionState {
    Unknown = 0,
    Prepare = 1,
    NeedAuth = 2,
    Connect = 3,
    IpConfigGet = 4,
    Activated = 5,
    Failed = 6,
    Disconnected = 7,
}

/// Device state change reasons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NMDeviceStateReason {
    None = 0,
    Unknown = 1,
    NowManaged = 2,
    NowUnmanaged = 3,
    ConfigFailed = 4,
    IpConfigUnavailable = 5,
    IpConfigExpired = 6,
    NoSecrets = 7,
    SupplicantDisconnect = 8,
    SupplicantConfigFailed = 9,
    SupplicantFailed = 10,
    SupplicantTimeout = 11,
    PppStartFailed = 12,
    PppDisconnect = 13,
    PppFailed = 14,
    DhcpStartFailed = 15,
    DhcpError = 16,
    DhcpFailed = 17,
    SharedStartFailed = 18,
    SharedFailed = 19,
    AutoIpStartFailed = 20,
    AutoIpError = 21,
    AutoIpFailed = 22,
    ModemBusy = 23,
    ModemNoDialTone = 24,
    ModemNoCarrier = 25,
    ModemDialTimeout = 26,
    ModemDialFailed = 27,
    ModemInitFailed = 28,
    GsmApnFailed = 29,
    GsmRegistrationNotSearching = 30,
    GsmRegistrationDenied = 31,
    GsmRegistrationTimeout = 32,
    GsmRegistrationFailed = 33,
    GsmPinCheckFailed = 34,
    FirmwareMissing = 35,
    Removed = 36,
    Sleeping = 37,
    ConnectionRemoved = 38,
    UserRequested = 39,
    Carrier = 40,
    ConnectionAssumed = 41,
    SupplicantAvailable = 42,
    ModemNotFound = 43,
    BtFailed = 44,
    GsmSimNotInserted = 45,
    GsmSimPinRequired = 46,
    GsmSimPukRequired = 47,
    GsmSimWrong = 48,
    InfinibandMode = 49,
    DependencyFailed = 50,
    Br2684Failed = 51,
    ModemManagerUnavailable = 52,
    SsidNotFound = 53,
    SecondaryConnectionFailed = 54,
    DcbFcoeFailed = 55,
    TeamdControlFailed = 56,
    ModemFailed = 57,
    ModemAvailable = 58,
    SimPinIncorrect = 59,
    NewActivation = 60,
    ParentChanged = 61,
    ParentManagedChanged = 62,
    OvsdbFailed = 63,
    IpAddressDuplicate = 64,
    IpMethodUnsupported = 65,
    SriovConfigurationFailed = 66,
    PeerNotFound = 67,
}

/// Active connection state change reasons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NMActiveConnectionStateReason {
    Unknown = 0,
    None = 1,
    UserDisconnected = 2,
    DeviceDisconnected = 3,
    ServiceStopped = 4,
    IpConfigInvalid = 5,
    ConnectTimeout = 6,
    ServiceStartTimeout = 7,
    ServiceStartFailed = 8,
    NoSecrets = 9,
    LoginFailed = 10,
    ConnectionRemoved = 11,
    DependencyFailed = 12,
    DeviceRealizeFailed = 13,
    DeviceRemoved = 14,
}

/// VPN state change reasons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NMVpnConnectionStateReason {
    Unknown = 0,
    None = 1,
    UserDisconnected = 2,
    DeviceDisconnected = 3,
    ServiceStopped = 4,
    IpConfigInvalid = 5,
    ConnectTimeout = 6,
    ServiceStartTimeout = 7,
    ServiceStartFailed = 8,
    NoSecrets = 9,
    LoginFailed = 10,
    ConnectionRemoved = 11,
}

/// Checkpoint rollback results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NMRollbackResult {
    Ok = 0,
    ErrNoDevice = 1,
    ErrDeviceCheckpointNotFound = 2,
    ErrFailedToRestore = 3,
    ErrUnknownDevice = 4,
}