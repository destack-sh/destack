use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// Runtime host module defaults.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct HostOptions {
    /// Filesystem host defaults.
    pub fs: HostFsOptions,
    /// Network host defaults.
    pub net: HostNetOptions,
    /// Process host defaults.
    pub process: HostProcessOptions,
    /// Audio host defaults.
    pub audio: HostAudioOptions,
    /// Display host defaults.
    pub display: HostDisplayOptions,
    /// Input host defaults.
    pub input: HostInputOptions,
    /// GPU host defaults.
    pub gpu: HostGpuOptions,
    /// TLS host defaults.
    pub tls: HostTlsOptions,
    /// OS host defaults.
    pub os: HostOsOptions,
}

/// Filesystem host defaults.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct HostFsOptions {
    /// Optional temporary directory override.
    pub temporary_directory: Option<PathBuf>,
    /// Optional cache directory override.
    pub cache_directory: Option<PathBuf>,
}

/// Network host defaults.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct HostNetOptions {
    /// Optional DNS server override list.
    pub dns_servers: Vec<String>,
    /// Optional proxy URL override.
    pub proxy_url: Option<String>,
    /// Optional default egress interface binding.
    pub bind_interface: Option<String>,
}

/// Process host defaults.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct HostProcessOptions {
    /// Optional default working directory for spawned child processes.
    pub default_working_directory: Option<PathBuf>,
    /// Whether child processes inherit environment variables by default.
    pub inherit_environment: Option<bool>,
    /// Optional allow-list for inherited environment variables.
    pub environment_allowlist: Vec<String>,
}

/// Audio host defaults.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct HostAudioOptions {
    /// Optional preferred audio backend name.
    pub backend: Option<String>,
    /// Optional preferred output device identifier.
    pub output_device: Option<String>,
    /// Optional preferred input device identifier.
    pub input_device: Option<String>,
}

/// Display host defaults.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct HostDisplayOptions {
    /// Optional preferred display backend name.
    pub backend: Option<String>,
}

/// Input host defaults.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct HostInputOptions {
    /// Optional preferred input backend name.
    pub backend: Option<String>,
}

/// GPU host defaults.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct HostGpuOptions {
    /// Optional preferred GPU backend name.
    pub backend: Option<String>,
    /// Optional preferred adapter name filter.
    pub adapter_name: Option<String>,
}

/// TLS host defaults.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct HostTlsOptions {
    /// Optional trust store path override.
    pub trust_store_path: Option<PathBuf>,
    /// Override list for Unix system certificate bundle files.
    pub system_certificate_files: Vec<PathBuf>,
    /// Override list for Unix system certificate directories.
    pub system_certificate_directories: Vec<PathBuf>,
}

/// OS host defaults.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct HostOsOptions {
    /// Optional default locale override.
    pub default_locale: Option<String>,
    /// Optional application data directory override.
    pub data_directory: Option<PathBuf>,
    /// Optional application state directory override.
    pub state_directory: Option<PathBuf>,
}
