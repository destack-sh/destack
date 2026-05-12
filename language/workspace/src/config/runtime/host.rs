use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Runtime host module defaults.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct HostOptions {
    /// Filesystem host defaults.
    pub fs: HostFsOptions,
    /// Network host defaults.
    pub net: HostNetOptions,
    /// Process host defaults.
    pub process: HostProcessOptions,
    /// Audio host defaults.
    pub audio: HostAudioOptions,
    /// Input host defaults.
    pub input: HostInputOptions,
    /// GPU host defaults.
    pub gpu: HostGpuOptions,
    /// TLS host defaults.
    pub tls: HostTlsOptions,
    /// OS host defaults.
    pub os: HostOsOptions,
    /// Crypto host defaults.
    pub crypto: HostCryptoOptions,
}

/// Crypto host defaults.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct HostCryptoOptions {
    /// Host key store path overrides.
    pub key_store_paths: HostCryptoStorePaths,
    /// Override list for Unix system certificate bundle files.
    pub system_certificate_files: Vec<PathBuf>,
    /// Override list for Unix system certificate directories.
    pub system_certificate_directories: Vec<PathBuf>,
}

/// Host key store path overrides.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct HostCryptoStorePaths {
    /// Override path for the user store.
    pub user: Option<PathBuf>,
    /// Override path for the machine store.
    pub machine: Option<PathBuf>,
}

/// Filesystem host defaults.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct HostFsOptions {
    /// Optional temporary directory override.
    pub temporary_directory: Option<PathBuf>,
    /// Optional cache directory override.
    pub cache_directory: Option<PathBuf>,
}

/// Network host defaults.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct HostNetOptions {
    /// Optional DNS server override list.
    pub dns_servers: Vec<String>,
    /// Optional proxy URL override.
    pub proxy_url: Option<String>,
    /// Optional default egress interface binding.
    pub bind_interface: Option<String>,
}

/// Process host defaults.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct HostProcessOptions {
    /// Optional default working directory for spawned child processes.
    pub default_working_directory: Option<PathBuf>,
    /// Whether child processes inherit environment variables by default.
    pub inherit_environment: Option<bool>,
    /// Optional allow-list for inherited environment variables.
    pub environment_allowlist: Vec<String>,
}

/// Audio host defaults.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct HostAudioOptions {
    /// Optional preferred audio backend name.
    pub backend: Option<String>,
    /// Optional preferred output device identifier.
    pub output_device: Option<String>,
    /// Optional preferred input device identifier.
    pub input_device: Option<String>,
    /// Optional target latency in frames.
    pub target_latency_frames: Option<u32>,
    /// Optional target period size in frames.
    pub target_period_frames: Option<u32>,
}

/// Input host defaults.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct HostInputOptions {
    /// Optional preferred input backend name.
    pub backend: Option<String>,
}

/// GPU host defaults.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct HostGpuOptions {
    /// Optional preferred GPU backend name.
    pub backend: Option<String>,
    /// Optional preferred adapter name filter.
    pub adapter_name: Option<String>,
}

/// TLS host defaults.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct HostTlsOptions {
    /// Optional trust store path override.
    pub trust_store_path: Option<PathBuf>,
    /// Optional client certificate store identifier.
    pub client_certificate_store: Option<String>,
}

/// OS host defaults.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct HostOsOptions {
    /// Optional default locale override.
    pub default_locale: Option<String>,
    /// Optional application data directory override.
    pub data_directory: Option<PathBuf>,
    /// Optional application state directory override.
    pub state_directory: Option<PathBuf>,
}

/// Runtime host module defaults for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct HostOptionsJson {
    /// Filesystem host defaults.
    pub fs: Option<HostFsOptionsJson>,
    /// Network host defaults.
    pub net: Option<HostNetOptionsJson>,
    /// Process host defaults.
    pub process: Option<HostProcessOptionsJson>,
    /// Audio host defaults.
    pub audio: Option<HostAudioOptionsJson>,
    /// Input host defaults.
    pub input: Option<HostInputOptionsJson>,
    /// GPU host defaults.
    pub gpu: Option<HostGpuOptionsJson>,
    /// TLS host defaults.
    pub tls: Option<HostTlsOptionsJson>,
    /// OS host defaults.
    pub os: Option<HostOsOptionsJson>,
    /// Crypto host defaults.
    pub crypto: Option<HostCryptoOptionsJson>,
}

impl HostOptionsJson {
    /// Inherit unset host settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.fs.is_none() {
            self.fs = parent.fs.clone();
        }
        if self.net.is_none() {
            self.net = parent.net.clone();
        }
        if self.process.is_none() {
            self.process = parent.process.clone();
        }
        if self.audio.is_none() {
            self.audio = parent.audio.clone();
        }
        if self.input.is_none() {
            self.input = parent.input.clone();
        }
        if self.gpu.is_none() {
            self.gpu = parent.gpu.clone();
        }
        if self.tls.is_none() {
            self.tls = parent.tls.clone();
        }
        if self.os.is_none() {
            self.os = parent.os.clone();
        }
        if self.crypto.is_none() {
            self.crypto = parent.crypto.clone();
        }
    }

    /// Apply host defaults to a base set of options.
    pub fn apply_to(&self, options: &mut HostOptions) {
        if let Some(fs) = &self.fs {
            fs.apply_to(&mut options.fs);
        }
        if let Some(net) = &self.net {
            net.apply_to(&mut options.net);
        }
        if let Some(process) = &self.process {
            process.apply_to(&mut options.process);
        }
        if let Some(audio) = &self.audio {
            audio.apply_to(&mut options.audio);
        }
        if let Some(input) = &self.input {
            input.apply_to(&mut options.input);
        }
        if let Some(gpu) = &self.gpu {
            gpu.apply_to(&mut options.gpu);
        }
        if let Some(tls) = &self.tls {
            tls.apply_to(&mut options.tls);
        }
        if let Some(os) = &self.os {
            os.apply_to(&mut options.os);
        }
        if let Some(crypto) = &self.crypto {
            crypto.apply_to(&mut options.crypto);
        }
    }
}

/// Crypto host defaults for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct HostCryptoOptionsJson {
    /// Host key store path overrides.
    pub key_store_paths: Option<HostCryptoStorePathsJson>,
    /// Override list for Unix system certificate bundle files.
    pub system_certificate_files: Option<Vec<String>>,
    /// Override list for Unix system certificate directories.
    pub system_certificate_directories: Option<Vec<String>>,
}

impl HostCryptoOptionsJson {
    /// Apply crypto defaults to a base set of options.
    pub fn apply_to(&self, options: &mut HostCryptoOptions) {
        if let Some(key_store_paths) = &self.key_store_paths {
            key_store_paths.apply_to(&mut options.key_store_paths);
        }

        if let Some(system_certificate_files) = &self.system_certificate_files {
            options.system_certificate_files =
                system_certificate_files.iter().map(PathBuf::from).collect();
        }
        if let Some(system_certificate_directories) = &self.system_certificate_directories {
            options.system_certificate_directories = system_certificate_directories
                .iter()
                .map(PathBuf::from)
                .collect();
        }
    }
}

/// Host key store path overrides for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct HostCryptoStorePathsJson {
    /// Override path for the user store.
    pub user: Option<String>,
    /// Override path for the machine store.
    pub machine: Option<String>,
}

impl HostCryptoStorePathsJson {
    /// Apply host key store path overrides to one options value.
    pub fn apply_to(&self, options: &mut HostCryptoStorePaths) {
        if let Some(user) = &self.user {
            options.user = Some(PathBuf::from(user));
        }

        if let Some(machine) = &self.machine {
            options.machine = Some(PathBuf::from(machine));
        }
    }
}

/// Filesystem host defaults for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct HostFsOptionsJson {
    /// Optional temporary directory override.
    pub temporary_directory: Option<String>,
    /// Optional cache directory override.
    pub cache_directory: Option<String>,
}

impl HostFsOptionsJson {
    /// Apply filesystem defaults to a base set of options.
    pub fn apply_to(&self, options: &mut HostFsOptions) {
        if let Some(temporary_directory) = &self.temporary_directory {
            options.temporary_directory = Some(PathBuf::from(temporary_directory));
        }

        if let Some(cache_directory) = &self.cache_directory {
            options.cache_directory = Some(PathBuf::from(cache_directory));
        }
    }
}

/// Network host defaults for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct HostNetOptionsJson {
    /// Optional DNS server override list.
    pub dns_servers: Option<Vec<String>>,
    /// Optional proxy URL override.
    pub proxy_url: Option<String>,
    /// Optional default egress interface binding.
    pub bind_interface: Option<String>,
}

impl HostNetOptionsJson {
    /// Apply network defaults to a base set of options.
    pub fn apply_to(&self, options: &mut HostNetOptions) {
        if let Some(dns_servers) = &self.dns_servers {
            options.dns_servers = dns_servers.clone();
        }

        if let Some(proxy_url) = &self.proxy_url {
            options.proxy_url = Some(proxy_url.clone());
        }

        if let Some(bind_interface) = &self.bind_interface {
            options.bind_interface = Some(bind_interface.clone());
        }
    }
}

/// Process host defaults for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct HostProcessOptionsJson {
    /// Optional default working directory for spawned child processes.
    pub default_working_directory: Option<String>,
    /// Whether child processes inherit environment variables by default.
    pub inherit_environment: Option<bool>,
    /// Optional allow-list for inherited environment variables.
    pub environment_allowlist: Option<Vec<String>>,
}

impl HostProcessOptionsJson {
    /// Apply process defaults to a base set of options.
    pub fn apply_to(&self, options: &mut HostProcessOptions) {
        if let Some(default_working_directory) = &self.default_working_directory {
            options.default_working_directory = Some(PathBuf::from(default_working_directory));
        }

        if let Some(inherit_environment) = self.inherit_environment {
            options.inherit_environment = Some(inherit_environment);
        }

        if let Some(environment_allowlist) = &self.environment_allowlist {
            options.environment_allowlist = environment_allowlist.clone();
        }
    }
}

/// Audio host defaults for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct HostAudioOptionsJson {
    /// Optional preferred audio backend name.
    pub backend: Option<String>,
    /// Optional preferred output device identifier.
    pub output_device: Option<String>,
    /// Optional preferred input device identifier.
    pub input_device: Option<String>,
    /// Optional target latency in frames.
    pub target_latency_frames: Option<u32>,
    /// Optional target period size in frames.
    pub target_period_frames: Option<u32>,
}

impl HostAudioOptionsJson {
    /// Apply audio defaults to a base set of options.
    pub fn apply_to(&self, options: &mut HostAudioOptions) {
        if let Some(backend) = &self.backend {
            options.backend = Some(backend.clone());
        }
        if let Some(output_device) = &self.output_device {
            options.output_device = Some(output_device.clone());
        }
        if let Some(input_device) = &self.input_device {
            options.input_device = Some(input_device.clone());
        }

        if let Some(target_latency_frames) = self.target_latency_frames {
            options.target_latency_frames = Some(target_latency_frames);
        }
        if let Some(target_period_frames) = self.target_period_frames {
            options.target_period_frames = Some(target_period_frames);
        }
    }
}

/// Input host defaults for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct HostInputOptionsJson {
    /// Optional preferred input backend name.
    pub backend: Option<String>,
}

impl HostInputOptionsJson {
    /// Apply input defaults to a base set of options.
    pub fn apply_to(&self, options: &mut HostInputOptions) {
        if let Some(backend) = &self.backend {
            options.backend = Some(backend.clone());
        }
    }
}

/// GPU host defaults for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct HostGpuOptionsJson {
    /// Optional preferred GPU backend name.
    pub backend: Option<String>,
    /// Optional preferred adapter name filter.
    pub adapter_name: Option<String>,
}

impl HostGpuOptionsJson {
    /// Apply GPU defaults to a base set of options.
    pub fn apply_to(&self, options: &mut HostGpuOptions) {
        if let Some(backend) = &self.backend {
            options.backend = Some(backend.clone());
        }
        if let Some(adapter_name) = &self.adapter_name {
            options.adapter_name = Some(adapter_name.clone());
        }
    }
}

/// TLS host defaults for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct HostTlsOptionsJson {
    /// Optional trust store path override.
    pub trust_store_path: Option<String>,
    /// Optional client certificate store identifier.
    pub client_certificate_store: Option<String>,
}

impl HostTlsOptionsJson {
    /// Apply TLS defaults to a base set of options.
    pub fn apply_to(&self, options: &mut HostTlsOptions) {
        if let Some(trust_store_path) = &self.trust_store_path {
            options.trust_store_path = Some(PathBuf::from(trust_store_path));
        }

        if let Some(client_certificate_store) = &self.client_certificate_store {
            options.client_certificate_store = Some(client_certificate_store.clone());
        }
    }
}

/// OS host defaults for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct HostOsOptionsJson {
    /// Optional default locale override.
    pub default_locale: Option<String>,
    /// Optional application data directory override.
    pub data_directory: Option<String>,
    /// Optional application state directory override.
    pub state_directory: Option<String>,
}

impl HostOsOptionsJson {
    /// Apply OS defaults to a base set of options.
    pub fn apply_to(&self, options: &mut HostOsOptions) {
        if let Some(default_locale) = &self.default_locale {
            options.default_locale = Some(default_locale.clone());
        }

        if let Some(data_directory) = &self.data_directory {
            options.data_directory = Some(PathBuf::from(data_directory));
        }

        if let Some(state_directory) = &self.state_directory {
            options.state_directory = Some(PathBuf::from(state_directory));
        }
    }
}
