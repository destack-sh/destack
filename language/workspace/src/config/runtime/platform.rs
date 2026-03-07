use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PlatformOptions {
    /// Android runtime host overrides.
    pub android: PlatformAndroidOptions,
    /// DragonFly BSD runtime host overrides.
    pub dragonfly: PlatformDragonflyOptions,
    /// FreeBSD runtime host overrides.
    pub freebsd: PlatformFreeBsdOptions,
    /// Haiku runtime host overrides.
    pub haiku: PlatformHaikuOptions,
    /// illumos runtime host overrides.
    pub illumos: PlatformIllumosOptions,
    /// iOS runtime host overrides.
    pub ios: PlatformIosOptions,
    /// Linux runtime host overrides.
    pub linux: PlatformLinuxOptions,
    /// macOS runtime host overrides.
    pub macos: PlatformMacosOptions,
    /// NetBSD runtime host overrides.
    pub netbsd: PlatformNetBsdOptions,
    /// OpenBSD runtime host overrides.
    pub openbsd: PlatformOpenBsdOptions,
    /// Solaris runtime host overrides.
    pub solaris: PlatformSolarisOptions,
    /// Windows runtime host overrides.
    pub windows: PlatformWindowsOptions,
}

/// Host integration options shared across platform runtime configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlatformHostOptions {
    /// Whether host lifecycle events are enabled.
    pub enable_lifecycle_events: bool,
    /// Whether host window events are enabled.
    pub enable_window_events: bool,
    /// Whether host permission events are enabled.
    pub enable_permission_events: bool,
    /// Whether host interruption events are enabled.
    pub enable_interruption_events: bool,
    /// Queue capacity for host events before drop behavior applies.
    pub event_queue_capacity: Option<u64>,
}

impl Default for PlatformHostOptions {
    fn default() -> Self {
        Self {
            enable_lifecycle_events: true,
            enable_window_events: true,
            enable_permission_events: true,
            enable_interruption_events: true,
            event_queue_capacity: None,
        }
    }
}

/// Crypto runtime options shared across platform runtime configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PlatformCryptoOptions {
    /// Host key-store path overrides for crypto store lanes.
    pub host_store_paths: PlatformCryptoHostStorePaths,
    /// Override list for Unix system certificate bundle files.
    pub system_certificate_files: Vec<PathBuf>,
    /// Override list for Unix system certificate directories.
    pub system_certificate_directories: Vec<PathBuf>,
    /// Override service name for macOS keychain snapshot storage.
    pub macos_keychain_snapshot_service: Option<String>,
    /// Override account name for macOS keychain snapshot storage.
    pub macos_keychain_snapshot_account: Option<String>,
}

/// Host key-store path overrides for crypto store lanes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PlatformCryptoHostStorePaths {
    /// Override path for the user store lane.
    pub user: Option<PathBuf>,
    /// Override path for the machine store lane.
    pub machine: Option<PathBuf>,
}

/// Filesystem runtime options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PlatformFsOptions {
    /// Optional sandbox root for filesystem operations.
    pub sandbox_root: Option<PathBuf>,
    /// Optional temporary directory override.
    pub temporary_directory: Option<PathBuf>,
    /// Optional cache directory override.
    pub cache_directory: Option<PathBuf>,
}

/// Network runtime options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PlatformNetOptions {
    /// Optional DNS server override list.
    pub dns_servers: Vec<String>,
    /// Optional proxy URL override.
    pub proxy_url: Option<String>,
    /// Optional default egress interface binding.
    pub bind_interface: Option<String>,
}

/// Process runtime options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PlatformProcessOptions {
    /// Optional default working directory for spawned child processes.
    pub default_working_directory: Option<PathBuf>,
    /// Whether child processes inherit environment variables by default.
    pub inherit_environment: Option<bool>,
    /// Optional allow-list for inherited environment variables.
    pub environment_allowlist: Vec<String>,
}

/// Audio runtime options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PlatformAudioOptions {
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
    /// Optional monitor worker poll interval in nanoseconds.
    pub event_monitor_poll_interval_ns: Option<u64>,
    /// Optional audio event queue capacity override.
    pub event_queue_capacity: Option<u64>,
    /// Optional default audio event poll interval in nanoseconds.
    pub default_event_poll_interval_ns: Option<u64>,
    /// Optional stream wait slice in nanoseconds.
    pub stream_wait_slice_ns: Option<u64>,
    /// Optional maximum stream read bytes per operation.
    pub max_stream_read_bytes: Option<u64>,
    /// Optional maximum queued stream frame budget.
    pub max_queued_frames: Option<u64>,
    /// Optional worker poll interval in nanoseconds.
    pub worker_poll_interval_ns: Option<u64>,
}

/// Input runtime options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PlatformInputOptions {
    /// Optional preferred input backend name.
    pub backend: Option<String>,
    /// Optional input event queue capacity override.
    pub event_queue_capacity: Option<u64>,
}

/// GPU runtime options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PlatformGpuOptions {
    /// Optional preferred GPU backend name.
    pub backend: Option<String>,
    /// Optional preferred adapter name filter.
    pub adapter_name: Option<String>,
    /// Optional power preference hint.
    pub power_preference: Option<String>,
    /// Optional shader cache directory override.
    pub shader_cache_directory: Option<PathBuf>,
}

/// TLS runtime options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PlatformTlsOptions {
    /// Optional trust store path override.
    pub trust_store_path: Option<PathBuf>,
    /// Optional client certificate store identifier.
    pub client_certificate_store: Option<String>,
}

/// Security runtime options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PlatformSecurityOptions {
    /// Optional sandbox profile selector.
    pub sandbox_profile: Option<String>,
    /// Optional capability profile selector.
    pub capability_profile: Option<String>,
}

/// OS service runtime options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PlatformOsOptions {
    /// Optional default locale override.
    pub default_locale: Option<String>,
    /// Optional application data directory override.
    pub data_directory: Option<PathBuf>,
    /// Optional application state directory override.
    pub state_directory: Option<PathBuf>,
}

/// Device service runtime options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PlatformDeviceOptions {
    /// Optional allow-list of device classes.
    pub allow_classes: Vec<String>,
    /// Optional deny-list of device classes.
    pub deny_classes: Vec<String>,
}

/// Debug runtime options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PlatformDebugOptions {}

/// Display runtime options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PlatformDisplayOptions {
    /// Optional display event queue capacity override.
    pub event_queue_capacity: Option<u64>,
    /// Optional display stream wait slice in nanoseconds.
    pub stream_wait_slice_ns: Option<u64>,
}

/// Error runtime options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PlatformErrorOptions {}

/// FFI runtime options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PlatformFfiOptions {}

/// I/O runtime options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PlatformIoOptions {}

/// IPC runtime options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PlatformIpcOptions {
    /// Optional Unix semaphore poll interval in nanoseconds.
    pub unix_semaphore_poll_interval_ns: Option<u64>,
}

/// Memory runtime options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PlatformMemoryOptions {}

/// Resource runtime options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PlatformResourceOptions {}

/// Thread runtime options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PlatformThreadOptions {}

/// TTY runtime options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PlatformTtyOptions {}

/// Android runtime host overrides.
pub type PlatformAndroidOptions = PlatformHostOptions;

/// DragonFly BSD runtime host overrides.
pub type PlatformDragonflyOptions = PlatformHostOptions;

/// FreeBSD runtime host overrides.
pub type PlatformFreeBsdOptions = PlatformHostOptions;

/// Haiku runtime host overrides.
pub type PlatformHaikuOptions = PlatformHostOptions;

/// illumos runtime host overrides.
pub type PlatformIllumosOptions = PlatformHostOptions;

/// iOS runtime host overrides.
pub type PlatformIosOptions = PlatformHostOptions;

/// Linux runtime host overrides.
pub type PlatformLinuxOptions = PlatformHostOptions;

/// macOS runtime host overrides.
pub type PlatformMacosOptions = PlatformHostOptions;

/// NetBSD runtime host overrides.
pub type PlatformNetBsdOptions = PlatformHostOptions;

/// OpenBSD runtime host overrides.
pub type PlatformOpenBsdOptions = PlatformHostOptions;

/// Solaris runtime host overrides.
pub type PlatformSolarisOptions = PlatformHostOptions;

/// Windows packet backend selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum PlatformWindowsPacketBackend {
    /// Disable packet backend lanes.
    #[default]
    Disabled,
    /// Use the raw-socket packet backend.
    RawSocket,
    /// Use the host-bridge packet backend.
    HostBackend,
}

/// Windows runtime host overrides.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlatformWindowsOptions {
    /// Whether host lifecycle events are enabled.
    pub enable_lifecycle_events: bool,
    /// Whether host window events are enabled.
    pub enable_window_events: bool,
    /// Whether host permission events are enabled.
    pub enable_permission_events: bool,
    /// Whether host interruption events are enabled.
    pub enable_interruption_events: bool,
    /// Queue capacity for host events before drop behavior applies.
    pub event_queue_capacity: Option<u64>,
    /// Optional POSIX domain SID for uid or gid mapping.
    pub posix_domain_sid: Option<String>,
    /// Packet backend mode for Windows packet lanes.
    pub net_packet_backend: PlatformWindowsPacketBackend,
}

impl Default for PlatformWindowsOptions {
    fn default() -> Self {
        Self {
            enable_lifecycle_events: true,
            enable_window_events: true,
            enable_permission_events: true,
            enable_interruption_events: true,
            event_queue_capacity: None,
            posix_domain_sid: None,
            net_packet_backend: PlatformWindowsPacketBackend::Disabled,
        }
    }
}

impl PlatformWindowsOptions {
    /// Build host integration options from Windows runtime host overrides.
    pub fn host_options(&self) -> PlatformHostOptions {
        PlatformHostOptions {
            enable_lifecycle_events: self.enable_lifecycle_events,
            enable_window_events: self.enable_window_events,
            enable_permission_events: self.enable_permission_events,
            enable_interruption_events: self.enable_interruption_events,
            event_queue_capacity: self.event_queue_capacity,
        }
    }
}
/// Platform-specific host runtime overrides.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlatformOptionsJson {
    /// Android-specific runtime host overrides.
    pub android: Option<PlatformAndroidOptionsJson>,
    /// DragonFly BSD-specific runtime host overrides.
    pub dragonfly: Option<PlatformDragonflyOptionsJson>,
    /// FreeBSD-specific runtime host overrides.
    pub freebsd: Option<PlatformFreeBsdOptionsJson>,
    /// Haiku-specific runtime host overrides.
    pub haiku: Option<PlatformHaikuOptionsJson>,
    /// illumos-specific runtime host overrides.
    pub illumos: Option<PlatformIllumosOptionsJson>,
    /// iOS-specific runtime host overrides.
    pub ios: Option<PlatformIosOptionsJson>,
    /// Linux-specific runtime host overrides.
    pub linux: Option<PlatformLinuxOptionsJson>,
    /// macOS-specific runtime host overrides.
    pub macos: Option<PlatformMacosOptionsJson>,
    /// NetBSD-specific runtime host overrides.
    pub netbsd: Option<PlatformNetBsdOptionsJson>,
    /// OpenBSD-specific runtime host overrides.
    pub openbsd: Option<PlatformOpenBsdOptionsJson>,
    /// Solaris-specific runtime host overrides.
    pub solaris: Option<PlatformSolarisOptionsJson>,
    /// Windows-specific runtime host overrides.
    pub windows: Option<PlatformWindowsOptionsJson>,
}

impl PlatformOptionsJson {
    /// Apply platform overrides to a base set of options.
    pub fn apply_to(&self, options: &mut PlatformOptions) {
        // apply android overrides
        if let Some(android) = &self.android {
            android.apply_to(&mut options.android);
        }

        // apply dragonfly overrides
        if let Some(dragonfly) = &self.dragonfly {
            dragonfly.apply_to(&mut options.dragonfly);
        }

        // apply freebsd overrides
        if let Some(freebsd) = &self.freebsd {
            freebsd.apply_to(&mut options.freebsd);
        }

        // apply haiku overrides
        if let Some(haiku) = &self.haiku {
            haiku.apply_to(&mut options.haiku);
        }

        // apply illumos overrides
        if let Some(illumos) = &self.illumos {
            illumos.apply_to(&mut options.illumos);
        }

        // apply ios overrides
        if let Some(ios) = &self.ios {
            ios.apply_to(&mut options.ios);
        }

        // apply linux overrides
        if let Some(linux) = &self.linux {
            linux.apply_to(&mut options.linux);
        }

        // apply macos overrides
        if let Some(macos) = &self.macos {
            macos.apply_to(&mut options.macos);
        }

        // apply netbsd overrides
        if let Some(netbsd) = &self.netbsd {
            netbsd.apply_to(&mut options.netbsd);
        }

        // apply openbsd overrides
        if let Some(openbsd) = &self.openbsd {
            openbsd.apply_to(&mut options.openbsd);
        }

        // apply solaris overrides
        if let Some(solaris) = &self.solaris {
            solaris.apply_to(&mut options.solaris);
        }

        // apply windows overrides
        if let Some(windows) = &self.windows {
            windows.apply_to(&mut options.windows);
        }
    }
}

/// Android runtime host overrides.
pub type PlatformAndroidOptionsJson = PlatformHostOptionsJson;

/// DragonFly BSD runtime host overrides.
pub type PlatformDragonflyOptionsJson = PlatformHostOptionsJson;

/// FreeBSD runtime host overrides.
pub type PlatformFreeBsdOptionsJson = PlatformHostOptionsJson;

/// Haiku runtime host overrides.
pub type PlatformHaikuOptionsJson = PlatformHostOptionsJson;

/// illumos runtime host overrides.
pub type PlatformIllumosOptionsJson = PlatformHostOptionsJson;

/// iOS runtime host overrides.
pub type PlatformIosOptionsJson = PlatformHostOptionsJson;

/// Linux runtime host overrides.
pub type PlatformLinuxOptionsJson = PlatformHostOptionsJson;

/// macOS runtime host overrides.
pub type PlatformMacosOptionsJson = PlatformHostOptionsJson;

/// NetBSD runtime host overrides.
pub type PlatformNetBsdOptionsJson = PlatformHostOptionsJson;

/// OpenBSD runtime host overrides.
pub type PlatformOpenBsdOptionsJson = PlatformHostOptionsJson;

/// Solaris runtime host overrides.
pub type PlatformSolarisOptionsJson = PlatformHostOptionsJson;

/// Windows packet backend selection for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum PlatformWindowsPacketBackendJson {
    /// Disable packet backend lanes.
    Disabled,
    /// Use the raw-socket packet backend.
    RawSocket,
    /// Use the host-bridge packet backend.
    HostBackend,
}

impl From<PlatformWindowsPacketBackendJson> for PlatformWindowsPacketBackend {
    fn from(value: PlatformWindowsPacketBackendJson) -> Self {
        match value {
            PlatformWindowsPacketBackendJson::Disabled => PlatformWindowsPacketBackend::Disabled,
            PlatformWindowsPacketBackendJson::RawSocket => PlatformWindowsPacketBackend::RawSocket,
            PlatformWindowsPacketBackendJson::HostBackend => {
                PlatformWindowsPacketBackend::HostBackend
            }
        }
    }
}

impl From<PlatformWindowsPacketBackend> for PlatformWindowsPacketBackendJson {
    fn from(value: PlatformWindowsPacketBackend) -> Self {
        match value {
            PlatformWindowsPacketBackend::Disabled => PlatformWindowsPacketBackendJson::Disabled,
            PlatformWindowsPacketBackend::RawSocket => PlatformWindowsPacketBackendJson::RawSocket,
            PlatformWindowsPacketBackend::HostBackend => {
                PlatformWindowsPacketBackendJson::HostBackend
            }
        }
    }
}

/// Windows runtime host overrides.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlatformWindowsOptionsJson {
    /// Whether host lifecycle events are enabled.
    pub enable_lifecycle_events: Option<bool>,
    /// Whether host window events are enabled.
    pub enable_window_events: Option<bool>,
    /// Whether host permission events are enabled.
    pub enable_permission_events: Option<bool>,
    /// Whether host interruption events are enabled.
    pub enable_interruption_events: Option<bool>,
    /// Queue capacity for host events before drop behavior applies.
    pub event_queue_capacity: Option<u64>,
    /// Optional POSIX domain SID for uid or gid mapping.
    pub posix_domain_sid: Option<String>,
    /// Packet backend mode for Windows packet lanes.
    pub net_packet_backend: Option<PlatformWindowsPacketBackendJson>,
}

impl PlatformWindowsOptionsJson {
    /// Apply windows host overrides to a base set of options.
    pub fn apply_to(&self, options: &mut PlatformWindowsOptions) {
        // apply host lifecycle override
        if let Some(enable_lifecycle_events) = self.enable_lifecycle_events {
            options.enable_lifecycle_events = enable_lifecycle_events;
        }

        // apply host window-event override
        if let Some(enable_window_events) = self.enable_window_events {
            options.enable_window_events = enable_window_events;
        }

        // apply host permission-event override
        if let Some(enable_permission_events) = self.enable_permission_events {
            options.enable_permission_events = enable_permission_events;
        }

        // apply host interruption-event override
        if let Some(enable_interruption_events) = self.enable_interruption_events {
            options.enable_interruption_events = enable_interruption_events;
        }

        // apply host queue-capacity override
        if let Some(event_queue_capacity) = self.event_queue_capacity {
            options.event_queue_capacity = Some(event_queue_capacity);
        }

        // apply domain sid override
        if let Some(posix_domain_sid) = &self.posix_domain_sid {
            options.posix_domain_sid = Some(posix_domain_sid.clone());
        }

        // apply packet backend override
        if let Some(net_packet_backend) = self.net_packet_backend {
            options.net_packet_backend = PlatformWindowsPacketBackend::from(net_packet_backend);
        }
    }
}
/// Crypto runtime options.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlatformCryptoOptionsJson {
    /// Host key-store path overrides for crypto store lanes.
    pub host_store_paths: Option<PlatformCryptoHostStorePathsJson>,
    /// Override list for Unix system certificate bundle files.
    pub system_certificate_files: Option<Vec<String>>,
    /// Override list for Unix system certificate directories.
    pub system_certificate_directories: Option<Vec<String>>,
    /// Override service name for macOS keychain snapshot storage.
    pub macos_keychain_snapshot_service: Option<String>,
    /// Override account name for macOS keychain snapshot storage.
    pub macos_keychain_snapshot_account: Option<String>,
}

impl PlatformCryptoOptionsJson {
    /// Apply crypto overrides to a base set of options.
    pub fn apply_to(&self, options: &mut PlatformCryptoOptions) {
        // apply host store path overrides
        if let Some(host_store_paths) = &self.host_store_paths {
            host_store_paths.apply_to(&mut options.host_store_paths);
        }

        // apply system certificate file overrides
        if let Some(system_certificate_files) = &self.system_certificate_files {
            options.system_certificate_files =
                system_certificate_files.iter().map(PathBuf::from).collect();
        }

        // apply system certificate directory overrides
        if let Some(system_certificate_directories) = &self.system_certificate_directories {
            options.system_certificate_directories = system_certificate_directories
                .iter()
                .map(PathBuf::from)
                .collect();
        }

        // apply macOS keychain snapshot service overrides
        if let Some(macos_keychain_snapshot_service) = &self.macos_keychain_snapshot_service {
            options.macos_keychain_snapshot_service = Some(macos_keychain_snapshot_service.clone());
        }

        // apply macOS keychain snapshot account overrides
        if let Some(macos_keychain_snapshot_account) = &self.macos_keychain_snapshot_account {
            options.macos_keychain_snapshot_account = Some(macos_keychain_snapshot_account.clone());
        }
    }
}

/// Host key-store path overrides for crypto store lanes.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlatformCryptoHostStorePathsJson {
    /// Override path for the user store lane.
    pub user: Option<String>,
    /// Override path for the machine store lane.
    pub machine: Option<String>,
}

impl PlatformCryptoHostStorePathsJson {
    /// Apply host key-store path overrides to one options value.
    pub fn apply_to(&self, options: &mut PlatformCryptoHostStorePaths) {
        // apply user-lane path overrides
        if let Some(user) = &self.user {
            options.user = Some(PathBuf::from(user));
        }

        // apply machine-lane path overrides
        if let Some(machine) = &self.machine {
            options.machine = Some(PathBuf::from(machine));
        }
    }
}

/// Filesystem runtime options.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlatformFsOptionsJson {
    /// Optional sandbox root for filesystem operations.
    pub sandbox_root: Option<String>,
    /// Optional temporary directory override.
    pub temporary_directory: Option<String>,
    /// Optional cache directory override.
    pub cache_directory: Option<String>,
}

impl PlatformFsOptionsJson {
    /// Apply filesystem overrides to a base set of options.
    pub fn apply_to(&self, options: &mut PlatformFsOptions) {
        // apply sandbox root overrides
        if let Some(sandbox_root) = &self.sandbox_root {
            options.sandbox_root = Some(PathBuf::from(sandbox_root));
        }

        // apply temporary directory overrides
        if let Some(temporary_directory) = &self.temporary_directory {
            options.temporary_directory = Some(PathBuf::from(temporary_directory));
        }

        // apply cache directory overrides
        if let Some(cache_directory) = &self.cache_directory {
            options.cache_directory = Some(PathBuf::from(cache_directory));
        }
    }
}

/// Network runtime options.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlatformNetOptionsJson {
    /// Optional DNS server override list.
    pub dns_servers: Option<Vec<String>>,
    /// Optional proxy URL override.
    pub proxy_url: Option<String>,
    /// Optional default egress interface binding.
    pub bind_interface: Option<String>,
}

impl PlatformNetOptionsJson {
    /// Apply network overrides to a base set of options.
    pub fn apply_to(&self, options: &mut PlatformNetOptions) {
        // apply dns server overrides
        if let Some(dns_servers) = &self.dns_servers {
            options.dns_servers = dns_servers.clone();
        }

        // apply proxy overrides
        if let Some(proxy_url) = &self.proxy_url {
            options.proxy_url = Some(proxy_url.clone());
        }

        // apply interface binding overrides
        if let Some(bind_interface) = &self.bind_interface {
            options.bind_interface = Some(bind_interface.clone());
        }
    }
}

/// Process runtime options.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlatformProcessOptionsJson {
    /// Optional default working directory for spawned child processes.
    pub default_working_directory: Option<String>,
    /// Whether child processes inherit environment variables by default.
    pub inherit_environment: Option<bool>,
    /// Optional allow-list for inherited environment variables.
    pub environment_allowlist: Option<Vec<String>>,
}

impl PlatformProcessOptionsJson {
    /// Apply process overrides to a base set of options.
    pub fn apply_to(&self, options: &mut PlatformProcessOptions) {
        // apply working directory overrides
        if let Some(default_working_directory) = &self.default_working_directory {
            options.default_working_directory = Some(PathBuf::from(default_working_directory));
        }

        // apply environment inheritance overrides
        if let Some(inherit_environment) = self.inherit_environment {
            options.inherit_environment = Some(inherit_environment);
        }

        // apply environment allow-list overrides
        if let Some(environment_allowlist) = &self.environment_allowlist {
            options.environment_allowlist = environment_allowlist.clone();
        }
    }
}

/// Audio runtime options.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlatformAudioOptionsJson {
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
    /// Optional monitor worker poll interval in nanoseconds.
    pub event_monitor_poll_interval_ns: Option<u64>,
    /// Optional audio event queue capacity override.
    pub event_queue_capacity: Option<u64>,
    /// Optional default audio event poll interval in nanoseconds.
    pub default_event_poll_interval_ns: Option<u64>,
    /// Optional stream wait slice in nanoseconds.
    pub stream_wait_slice_ns: Option<u64>,
    /// Optional maximum stream read bytes per operation.
    pub max_stream_read_bytes: Option<u64>,
    /// Optional maximum queued stream frame budget.
    pub max_queued_frames: Option<u64>,
    /// Optional worker poll interval in nanoseconds.
    pub worker_poll_interval_ns: Option<u64>,
}

impl PlatformAudioOptionsJson {
    /// Apply audio overrides to a base set of options.
    pub fn apply_to(&self, options: &mut PlatformAudioOptions) {
        // apply backend overrides
        if let Some(backend) = &self.backend {
            options.backend = Some(backend.clone());
        }

        // apply output device overrides
        if let Some(output_device) = &self.output_device {
            options.output_device = Some(output_device.clone());
        }

        // apply input device overrides
        if let Some(input_device) = &self.input_device {
            options.input_device = Some(input_device.clone());
        }

        // apply latency overrides
        if let Some(target_latency_frames) = self.target_latency_frames {
            options.target_latency_frames = Some(target_latency_frames);
        }

        // apply period-size overrides
        if let Some(target_period_frames) = self.target_period_frames {
            options.target_period_frames = Some(target_period_frames);
        }

        // apply monitor interval overrides
        if let Some(event_monitor_poll_interval_ns) = self.event_monitor_poll_interval_ns {
            options.event_monitor_poll_interval_ns = Some(event_monitor_poll_interval_ns);
        }

        // apply event queue overrides
        if let Some(event_queue_capacity) = self.event_queue_capacity {
            options.event_queue_capacity = Some(event_queue_capacity);
        }

        // apply event poll interval overrides
        if let Some(default_event_poll_interval_ns) = self.default_event_poll_interval_ns {
            options.default_event_poll_interval_ns = Some(default_event_poll_interval_ns);
        }

        // apply stream wait slice overrides
        if let Some(stream_wait_slice_ns) = self.stream_wait_slice_ns {
            options.stream_wait_slice_ns = Some(stream_wait_slice_ns);
        }

        // apply max stream read overrides
        if let Some(max_stream_read_bytes) = self.max_stream_read_bytes {
            options.max_stream_read_bytes = Some(max_stream_read_bytes);
        }

        // apply max queued frame overrides
        if let Some(max_queued_frames) = self.max_queued_frames {
            options.max_queued_frames = Some(max_queued_frames);
        }

        // apply worker poll interval overrides
        if let Some(worker_poll_interval_ns) = self.worker_poll_interval_ns {
            options.worker_poll_interval_ns = Some(worker_poll_interval_ns);
        }
    }
}

/// Input runtime options.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlatformInputOptionsJson {
    /// Optional preferred input backend name.
    pub backend: Option<String>,
    /// Optional input event queue capacity override.
    pub event_queue_capacity: Option<u64>,
}

impl PlatformInputOptionsJson {
    /// Apply input overrides to a base set of options.
    pub fn apply_to(&self, options: &mut PlatformInputOptions) {
        // apply backend overrides
        if let Some(backend) = &self.backend {
            options.backend = Some(backend.clone());
        }

        // apply event queue capacity overrides
        if let Some(event_queue_capacity) = self.event_queue_capacity {
            options.event_queue_capacity = Some(event_queue_capacity);
        }
    }
}

/// GPU runtime options.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlatformGpuOptionsJson {
    /// Optional preferred GPU backend name.
    pub backend: Option<String>,
    /// Optional preferred adapter name filter.
    pub adapter_name: Option<String>,
    /// Optional power preference hint.
    pub power_preference: Option<String>,
    /// Optional shader cache directory override.
    pub shader_cache_directory: Option<String>,
}

impl PlatformGpuOptionsJson {
    /// Apply GPU overrides to a base set of options.
    pub fn apply_to(&self, options: &mut PlatformGpuOptions) {
        // apply backend overrides
        if let Some(backend) = &self.backend {
            options.backend = Some(backend.clone());
        }

        // apply adapter-name overrides
        if let Some(adapter_name) = &self.adapter_name {
            options.adapter_name = Some(adapter_name.clone());
        }

        // apply power preference overrides
        if let Some(power_preference) = &self.power_preference {
            options.power_preference = Some(power_preference.clone());
        }

        // apply shader cache directory overrides
        if let Some(shader_cache_directory) = &self.shader_cache_directory {
            options.shader_cache_directory = Some(PathBuf::from(shader_cache_directory));
        }
    }
}

/// TLS runtime options.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlatformTlsOptionsJson {
    /// Optional trust store path override.
    pub trust_store_path: Option<String>,
    /// Optional client certificate store identifier.
    pub client_certificate_store: Option<String>,
}

impl PlatformTlsOptionsJson {
    /// Apply TLS overrides to a base set of options.
    pub fn apply_to(&self, options: &mut PlatformTlsOptions) {
        // apply trust-store overrides
        if let Some(trust_store_path) = &self.trust_store_path {
            options.trust_store_path = Some(PathBuf::from(trust_store_path));
        }

        // apply client-certificate store overrides
        if let Some(client_certificate_store) = &self.client_certificate_store {
            options.client_certificate_store = Some(client_certificate_store.clone());
        }
    }
}

/// Security runtime options.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlatformSecurityOptionsJson {
    /// Optional sandbox profile selector.
    pub sandbox_profile: Option<String>,
    /// Optional capability profile selector.
    pub capability_profile: Option<String>,
}

impl PlatformSecurityOptionsJson {
    /// Apply security overrides to a base set of options.
    pub fn apply_to(&self, options: &mut PlatformSecurityOptions) {
        // apply sandbox profile overrides
        if let Some(sandbox_profile) = &self.sandbox_profile {
            options.sandbox_profile = Some(sandbox_profile.clone());
        }

        // apply capability profile overrides
        if let Some(capability_profile) = &self.capability_profile {
            options.capability_profile = Some(capability_profile.clone());
        }
    }
}

/// OS service runtime options.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlatformOsOptionsJson {
    /// Optional default locale override.
    pub default_locale: Option<String>,
    /// Optional application data directory override.
    pub data_directory: Option<String>,
    /// Optional application state directory override.
    pub state_directory: Option<String>,
}

impl PlatformOsOptionsJson {
    /// Apply OS service overrides to a base set of options.
    pub fn apply_to(&self, options: &mut PlatformOsOptions) {
        // apply locale overrides
        if let Some(default_locale) = &self.default_locale {
            options.default_locale = Some(default_locale.clone());
        }

        // apply data-directory overrides
        if let Some(data_directory) = &self.data_directory {
            options.data_directory = Some(PathBuf::from(data_directory));
        }

        // apply state-directory overrides
        if let Some(state_directory) = &self.state_directory {
            options.state_directory = Some(PathBuf::from(state_directory));
        }
    }
}

/// Device service runtime options.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlatformDeviceOptionsJson {
    /// Optional allow-list of device classes.
    pub allow_classes: Option<Vec<String>>,
    /// Optional deny-list of device classes.
    pub deny_classes: Option<Vec<String>>,
}

impl PlatformDeviceOptionsJson {
    /// Apply device service overrides to a base set of options.
    pub fn apply_to(&self, options: &mut PlatformDeviceOptions) {
        // apply allow-list overrides
        if let Some(allow_classes) = &self.allow_classes {
            options.allow_classes = allow_classes.clone();
        }

        // apply deny-list overrides
        if let Some(deny_classes) = &self.deny_classes {
            options.deny_classes = deny_classes.clone();
        }
    }
}

/// Debug runtime options.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlatformDebugOptionsJson {}

impl PlatformDebugOptionsJson {
    /// Apply debug overrides to a base set of options.
    pub fn apply_to(&self, _options: &mut PlatformDebugOptions) {}
}

/// Display runtime options.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlatformDisplayOptionsJson {
    /// Optional display event queue capacity override.
    pub event_queue_capacity: Option<u64>,
    /// Optional display stream wait slice in nanoseconds.
    pub stream_wait_slice_ns: Option<u64>,
}

impl PlatformDisplayOptionsJson {
    /// Apply display overrides to a base set of options.
    pub fn apply_to(&self, options: &mut PlatformDisplayOptions) {
        // apply queue-capacity overrides
        if let Some(event_queue_capacity) = self.event_queue_capacity {
            options.event_queue_capacity = Some(event_queue_capacity);
        }

        // apply wait-slice overrides
        if let Some(stream_wait_slice_ns) = self.stream_wait_slice_ns {
            options.stream_wait_slice_ns = Some(stream_wait_slice_ns);
        }
    }
}

/// Error runtime options.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlatformErrorOptionsJson {}

impl PlatformErrorOptionsJson {
    /// Apply error overrides to a base set of options.
    pub fn apply_to(&self, _options: &mut PlatformErrorOptions) {}
}

/// FFI runtime options.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlatformFfiOptionsJson {}

impl PlatformFfiOptionsJson {
    /// Apply ffi overrides to a base set of options.
    pub fn apply_to(&self, _options: &mut PlatformFfiOptions) {}
}

/// I/O runtime options.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlatformIoOptionsJson {}

impl PlatformIoOptionsJson {
    /// Apply io overrides to a base set of options.
    pub fn apply_to(&self, _options: &mut PlatformIoOptions) {}
}

/// IPC runtime options.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlatformIpcOptionsJson {
    /// Optional Unix semaphore poll interval in nanoseconds.
    pub unix_semaphore_poll_interval_ns: Option<u64>,
}

impl PlatformIpcOptionsJson {
    /// Apply ipc overrides to a base set of options.
    pub fn apply_to(&self, options: &mut PlatformIpcOptions) {
        if let Some(unix_semaphore_poll_interval_ns) = self.unix_semaphore_poll_interval_ns {
            options.unix_semaphore_poll_interval_ns = Some(unix_semaphore_poll_interval_ns);
        }
    }
}

/// Memory runtime options.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlatformMemoryOptionsJson {}

impl PlatformMemoryOptionsJson {
    /// Apply memory overrides to a base set of options.
    pub fn apply_to(&self, _options: &mut PlatformMemoryOptions) {}
}

/// Resource runtime options.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlatformResourceOptionsJson {}

impl PlatformResourceOptionsJson {
    /// Apply resource overrides to a base set of options.
    pub fn apply_to(&self, _options: &mut PlatformResourceOptions) {}
}

/// Thread runtime options.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlatformThreadOptionsJson {}

impl PlatformThreadOptionsJson {
    /// Apply thread overrides to a base set of options.
    pub fn apply_to(&self, _options: &mut PlatformThreadOptions) {}
}

/// TTY runtime options.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlatformTtyOptionsJson {}

impl PlatformTtyOptionsJson {
    /// Apply tty overrides to a base set of options.
    pub fn apply_to(&self, _options: &mut PlatformTtyOptions) {}
}

/// Host integration options.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlatformHostOptionsJson {
    /// Whether host lifecycle events are enabled.
    pub enable_lifecycle_events: Option<bool>,
    /// Whether host window events are enabled.
    pub enable_window_events: Option<bool>,
    /// Whether host permission events are enabled.
    pub enable_permission_events: Option<bool>,
    /// Whether host interruption events are enabled.
    pub enable_interruption_events: Option<bool>,
    /// Queue capacity for host events before drop behavior applies.
    pub event_queue_capacity: Option<u64>,
}

impl PlatformHostOptionsJson {
    /// Apply host integration overrides to a base set of options.
    pub fn apply_to(&self, options: &mut PlatformHostOptions) {
        // apply lifecycle event overrides
        if let Some(enable_lifecycle_events) = self.enable_lifecycle_events {
            options.enable_lifecycle_events = enable_lifecycle_events;
        }

        // apply window event overrides
        if let Some(enable_window_events) = self.enable_window_events {
            options.enable_window_events = enable_window_events;
        }

        // apply permission event overrides
        if let Some(enable_permission_events) = self.enable_permission_events {
            options.enable_permission_events = enable_permission_events;
        }

        // apply interruption event overrides
        if let Some(enable_interruption_events) = self.enable_interruption_events {
            options.enable_interruption_events = enable_interruption_events;
        }

        // apply host event queue capacity overrides
        if let Some(event_queue_capacity) = self.event_queue_capacity {
            options.event_queue_capacity = Some(event_queue_capacity);
        }
    }
}
