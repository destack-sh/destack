use serde::{Deserialize, Serialize};

/// Platform-specific runtime overrides.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct PlatformOptions {
    /// Windows runtime overrides.
    pub windows: WindowsOptions,
}

/// Windows runtime overrides.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct WindowsOptions {
    /// Optional POSIX domain SID for uid or gid mapping.
    pub posix_domain_sid: Option<String>,
    /// Windows network overrides.
    pub net: WindowsNetOptions,
}

/// Windows network overrides.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct WindowsNetOptions {
    /// Packet backend mode for Windows packet operations.
    pub packet_backend: WindowsPacketBackend,
}

/// Windows packet backend selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum WindowsPacketBackend {
    /// Disable packet backends.
    #[default]
    Disabled,
    /// Use the raw-socket packet backend.
    RawSocket,
    /// Use the host-bridge packet backend.
    HostBridge,
}
