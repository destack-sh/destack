use serde::{Deserialize, Serialize};

/// Platform-specific runtime overrides.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PlatformOptions {
    /// Windows runtime overrides.
    pub windows: WindowsOptions,
}

/// Windows runtime overrides.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct WindowsOptions {
    /// Optional POSIX domain SID for uid or gid mapping.
    pub posix_domain_sid: Option<String>,
    /// Windows network overrides.
    pub net: WindowsNetOptions,
}

/// Windows network overrides.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct WindowsNetOptions {
    /// Packet backend mode for Windows packet operations.
    pub packet_backend: WindowsPacketBackend,
}

/// Windows packet backend selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum WindowsPacketBackend {
    /// Disable packet backends.
    #[default]
    Disabled,
    /// Use the raw-socket packet backend.
    RawSocket,
    /// Use the host-bridge packet backend.
    HostBridge,
}

/// Platform-specific runtime overrides for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlatformOptionsJson {
    /// Windows runtime overrides.
    pub windows: Option<WindowsOptionsJson>,
}

impl PlatformOptionsJson {
    /// Apply platform overrides to a base set of options.
    pub fn apply_to(&self, options: &mut PlatformOptions) {
        if let Some(windows) = &self.windows {
            windows.apply_to(&mut options.windows);
        }
    }
}

/// Windows runtime overrides for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct WindowsOptionsJson {
    /// Optional POSIX domain SID for uid or gid mapping.
    pub posix_domain_sid: Option<String>,
    /// Windows network overrides.
    pub net: Option<WindowsNetOptionsJson>,
}

impl WindowsOptionsJson {
    /// Apply windows overrides to a base set of options.
    pub fn apply_to(&self, options: &mut WindowsOptions) {
        if let Some(posix_domain_sid) = &self.posix_domain_sid {
            options.posix_domain_sid = Some(posix_domain_sid.clone());
        }

        if let Some(net) = &self.net {
            net.apply_to(&mut options.net);
        }
    }
}

/// Windows network overrides for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct WindowsNetOptionsJson {
    /// Packet backend mode for Windows packet operations.
    pub packet_backend: Option<WindowsPacketBackendJson>,
}

impl WindowsNetOptionsJson {
    /// Apply windows network overrides to a base set of options.
    pub fn apply_to(&self, options: &mut WindowsNetOptions) {
        if let Some(packet_backend) = self.packet_backend {
            options.packet_backend = WindowsPacketBackend::from(packet_backend);
        }
    }
}

/// Windows packet backend selection for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum WindowsPacketBackendJson {
    /// Disable packet backends.
    Disabled,
    /// Use the raw-socket packet backend.
    RawSocket,
    /// Use the host-bridge packet backend.
    HostBridge,
}

impl From<WindowsPacketBackendJson> for WindowsPacketBackend {
    fn from(value: WindowsPacketBackendJson) -> Self {
        match value {
            WindowsPacketBackendJson::Disabled => Self::Disabled,
            WindowsPacketBackendJson::RawSocket => Self::RawSocket,
            WindowsPacketBackendJson::HostBridge => Self::HostBridge,
        }
    }
}
