use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::config::target::{
    Runtime, TargetAppBackgroundMode, TargetAppDeclaration, TargetAppForegroundMode,
    TargetAppIdentityDeclaration, TargetAppNotificationCategoryDeclaration, TargetAppPermission,
};

use super::super::policy::{ExecutionMode, ExecutionModeJson};
use super::{
    HeapOptions, HeapOptionsJson, PlatformAudioOptions, PlatformAudioOptionsJson,
    PlatformCryptoOptions, PlatformCryptoOptionsJson, PlatformDebugOptions,
    PlatformDebugOptionsJson, PlatformDeviceOptions, PlatformDeviceOptionsJson,
    PlatformDisplayOptions, PlatformDisplayOptionsJson, PlatformErrorOptions,
    PlatformErrorOptionsJson, PlatformFfiOptions, PlatformFfiOptionsJson, PlatformFsOptions,
    PlatformFsOptionsJson, PlatformGpuOptions, PlatformGpuOptionsJson, PlatformInputOptions,
    PlatformInputOptionsJson, PlatformIoOptions, PlatformIoOptionsJson, PlatformIpcOptions,
    PlatformIpcOptionsJson, PlatformNetOptions, PlatformNetOptionsJson, PlatformOptions,
    PlatformOptionsJson, PlatformOsOptions, PlatformOsOptionsJson, PlatformProcessOptions,
    PlatformProcessOptionsJson, PlatformResourceOptions, PlatformResourceOptionsJson,
    PlatformSecurityOptions, PlatformSecurityOptionsJson, PlatformThreadOptions,
    PlatformThreadOptionsJson, PlatformTlsOptions, PlatformTlsOptionsJson, PlatformTtyOptions,
    PlatformTtyOptionsJson, RandomOptions, RandomOptionsJson, ReplayOptions, ReplayOptionsJson,
    RuntimeAccess, RuntimeAccessJson, RuntimeDiagnosticOptions, RuntimeDiagnosticOptionsJson,
    RuntimeRule, RuntimeRuleJson, RuntimeWorld, RuntimeWorldJson, SchedulerOptions,
    SchedulerOptionsJson, TimeOptions, TimeOptionsJson,
};

/// Default identity options for one runtime primary worker.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct RuntimeWorkerOptions {
    /// Default primary worker name for policy selection.
    pub name: Option<String>,
    /// Default primary worker labels for policy selection.
    pub labels: BTreeMap<String, String>,
}

/// Runtime-facing app declaration used by Host availability checks.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct RuntimeAppDeclaration {
    /// Runtime-facing app identity.
    pub identity: RuntimeAppIdentityDeclaration,
    /// Declared app permissions.
    pub permissions: BTreeSet<RuntimeAppPermission>,
    /// Runtime-facing intent declaration.
    pub intents: RuntimeAppIntentDeclaration,
    /// Runtime-facing notification declaration.
    pub notifications: RuntimeAppNotificationDeclaration,
    /// Runtime-facing background execution declaration.
    pub background: RuntimeAppBackgroundDeclaration,
    /// Runtime-facing service declaration.
    pub services: RuntimeAppServiceDeclaration,
    /// Runtime-facing document declaration.
    pub document: RuntimeAppDocumentDeclaration,
    /// Runtime-facing credential declaration.
    pub credentials: RuntimeAppCredentialDeclaration,
    /// Runtime-facing location declaration.
    pub location: RuntimeAppLocationDeclaration,
}

/// Runtime-facing app identity used by Host integrations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct RuntimeAppIdentityDeclaration {
    /// Stable application identifier, ideally reverse-DNS style.
    pub identifier: Option<String>,
    /// Human-facing app display name.
    pub display_name: Option<String>,
    /// Optional host-facing icon path for desktop shell integration.
    pub icon_path: Option<PathBuf>,
}

/// Runtime-facing intent declaration used by Host request checks.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct RuntimeAppIntentDeclaration {
    /// Declared outbound URL query schemes.
    pub query_schemes: BTreeSet<String>,
    /// Whether the app declares outbound local file sharing.
    pub shares_files: bool,
    /// URL schemes that activate the app inbound.
    pub handled_schemes: BTreeSet<String>,
    /// Verified web domains associated with direct app activation.
    pub verified_domains: BTreeSet<String>,
    /// File types the app declares for inbound open activation.
    pub handled_file_types: BTreeSet<String>,
    /// Whether the app declares inbound shared-text activation.
    pub receives_shared_text: bool,
    /// Share payload types the app declares for inbound activation.
    pub handled_share_types: BTreeSet<String>,
    /// Custom actions that activate the app.
    pub custom_actions: BTreeSet<String>,
}

/// Runtime-facing notification declaration used by Host request checks.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct RuntimeAppNotificationDeclaration {
    /// Whether notification authorization is declared.
    pub enabled: bool,
    /// Whether remote or push notification support is declared.
    pub remote: bool,
    /// Whether badge support is declared.
    pub badges: bool,
    /// Whether sound support is declared.
    pub sounds: bool,
    /// Whether time-sensitive notification support is declared.
    pub time_sensitive: bool,
    /// Whether critical-alert support is declared.
    pub critical_alerts: bool,
    /// Declared notification categories.
    pub categories: Vec<RuntimeAppNotificationCategoryDeclaration>,
}

/// Runtime-facing notification category declaration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct RuntimeAppNotificationCategoryDeclaration {
    /// Stable notification category identifier.
    pub identifier: String,
    /// Stable action identifiers available for this category.
    pub action_identifiers: BTreeSet<String>,
}

/// Runtime-facing background declaration used by Host request checks.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct RuntimeAppBackgroundDeclaration {
    /// Declared background execution modes.
    pub modes: BTreeSet<RuntimeAppBackgroundMode>,
    /// Declared stable background task identifiers.
    pub task_identifiers: BTreeSet<String>,
}

/// Runtime-facing background mode selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RuntimeAppBackgroundMode {
    /// Audio playback or capture in the background.
    Audio,
    /// Location updates in the background.
    Location,
    /// Periodic background fetch.
    Fetch,
    /// General background processing.
    Processing,
    /// Remote notification wake handling.
    RemoteNotifications,
    /// Voice over IP background mode.
    Voip,
    /// Bluetooth central mode.
    BluetoothCentral,
    /// Bluetooth peripheral mode.
    BluetoothPeripheral,
    /// Motion sensor processing in the background.
    Motion,
    /// Picture-in-picture presentation.
    PictureInPicture,
}

impl From<TargetAppBackgroundMode> for RuntimeAppBackgroundMode {
    fn from(mode: TargetAppBackgroundMode) -> Self {
        match mode {
            TargetAppBackgroundMode::Audio => Self::Audio,
            TargetAppBackgroundMode::Location => Self::Location,
            TargetAppBackgroundMode::Fetch => Self::Fetch,
            TargetAppBackgroundMode::Processing => Self::Processing,
            TargetAppBackgroundMode::RemoteNotifications => Self::RemoteNotifications,
            TargetAppBackgroundMode::Voip => Self::Voip,
            TargetAppBackgroundMode::BluetoothCentral => Self::BluetoothCentral,
            TargetAppBackgroundMode::BluetoothPeripheral => Self::BluetoothPeripheral,
            TargetAppBackgroundMode::Motion => Self::Motion,
            TargetAppBackgroundMode::PictureInPicture => Self::PictureInPicture,
        }
    }
}

/// Runtime-facing service declaration used by Host request checks.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct RuntimeAppServiceDeclaration {
    /// Declared foreground service classes.
    pub foreground_modes: BTreeSet<RuntimeAppForegroundMode>,
}

/// Runtime-facing foreground service selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RuntimeAppForegroundMode {
    /// Audio playback or long-running audio work.
    Audio,
    /// Location tracking or navigation work.
    Location,
    /// Camera capture work.
    Camera,
    /// Microphone capture work.
    Microphone,
    /// Connected-device communication work.
    ConnectedDevice,
    /// Data synchronization work.
    DataSync,
    /// Media projection or screen-capture work.
    ScreenCapture,
    /// Phone-call or voice-call work.
    PhoneCall,
    /// Remote messaging work.
    RemoteMessaging,
    /// Health or fitness sensor work.
    Health,
}

impl From<TargetAppForegroundMode> for RuntimeAppForegroundMode {
    fn from(mode: TargetAppForegroundMode) -> Self {
        match mode {
            TargetAppForegroundMode::Audio => Self::Audio,
            TargetAppForegroundMode::Location => Self::Location,
            TargetAppForegroundMode::Camera => Self::Camera,
            TargetAppForegroundMode::Microphone => Self::Microphone,
            TargetAppForegroundMode::ConnectedDevice => Self::ConnectedDevice,
            TargetAppForegroundMode::DataSync => Self::DataSync,
            TargetAppForegroundMode::ScreenCapture => Self::ScreenCapture,
            TargetAppForegroundMode::PhoneCall => Self::PhoneCall,
            TargetAppForegroundMode::RemoteMessaging => Self::RemoteMessaging,
            TargetAppForegroundMode::Health => Self::Health,
        }
    }
}

/// Runtime-facing document declaration used by Host request checks.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct RuntimeAppDocumentDeclaration {
    /// Types the app declares for document import or open.
    pub open_types: BTreeSet<String>,
    /// Types the app declares for document export or save.
    pub save_types: BTreeSet<String>,
    /// Whether the app declares open-in-place support.
    pub supports_open_in_place: bool,
}

/// Runtime-facing credential declaration used by Host request checks.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct RuntimeAppCredentialDeclaration {
    /// Human-facing biometric usage text.
    pub biometric_usage: Option<String>,
    /// Shared secure-store access groups.
    pub access_groups: BTreeSet<String>,
    /// Shared web credential domains.
    pub credential_domains: BTreeSet<String>,
}

/// Runtime-facing location declaration used by Host request checks.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct RuntimeAppLocationDeclaration {
    /// Whether background location updates are declared.
    pub allows_background_updates: bool,
    /// Whether precise location is declared as the default behavior.
    pub precise_by_default: bool,
    /// Purpose keys for temporarily requesting precise location.
    pub temporary_precise_purposes: BTreeSet<String>,
}

impl From<&TargetAppNotificationCategoryDeclaration> for RuntimeAppNotificationCategoryDeclaration {
    fn from(category: &TargetAppNotificationCategoryDeclaration) -> Self {
        Self {
            identifier: category.identifier.clone(),
            action_identifiers: category
                .actions
                .iter()
                .map(|action| action.identifier.clone())
                .collect(),
        }
    }
}

impl From<&TargetAppIdentityDeclaration> for RuntimeAppIdentityDeclaration {
    fn from(identity: &TargetAppIdentityDeclaration) -> Self {
        Self {
            identifier: identity.identifier.clone(),
            display_name: identity.display_name.clone(),
            icon_path: identity.icon_path.clone(),
        }
    }
}

impl From<&TargetAppDeclaration> for RuntimeAppDeclaration {
    fn from(declaration: &TargetAppDeclaration) -> Self {
        // permissions
        let permissions = declaration
            .permissions
            .keys()
            .copied()
            .map(RuntimeAppPermission::from)
            .collect();

        // runtime app declaration
        Self {
            identity: (&declaration.identity).into(),
            permissions,
            intents: RuntimeAppIntentDeclaration {
                query_schemes: declaration.intents.query_schemes.iter().cloned().collect(),
                shares_files: declaration.intents.shares_files,
                handled_schemes: declaration
                    .intents
                    .handled_schemes
                    .iter()
                    .cloned()
                    .collect(),
                verified_domains: declaration
                    .intents
                    .verified_domains
                    .iter()
                    .cloned()
                    .collect(),
                handled_file_types: declaration
                    .intents
                    .handled_file_types
                    .iter()
                    .cloned()
                    .collect(),
                receives_shared_text: declaration.intents.receives_shared_text,
                handled_share_types: declaration
                    .intents
                    .handled_share_types
                    .iter()
                    .cloned()
                    .collect(),
                custom_actions: declaration.intents.custom_actions.iter().cloned().collect(),
            },
            notifications: RuntimeAppNotificationDeclaration {
                enabled: declaration.notifications.enabled,
                remote: declaration.notifications.remote,
                badges: declaration.notifications.badges,
                sounds: declaration.notifications.sounds,
                time_sensitive: declaration.notifications.time_sensitive,
                critical_alerts: declaration.notifications.critical_alerts,
                categories: declaration
                    .notifications
                    .categories
                    .iter()
                    .map(Into::into)
                    .collect(),
            },
            background: RuntimeAppBackgroundDeclaration {
                modes: declaration
                    .background
                    .modes
                    .iter()
                    .copied()
                    .map(Into::into)
                    .collect(),
                task_identifiers: declaration
                    .background
                    .task_identifiers
                    .iter()
                    .cloned()
                    .collect(),
            },
            services: RuntimeAppServiceDeclaration {
                foreground_modes: declaration
                    .services
                    .foreground_modes
                    .iter()
                    .copied()
                    .map(Into::into)
                    .collect(),
            },
            document: RuntimeAppDocumentDeclaration {
                open_types: declaration.document.open_types.iter().cloned().collect(),
                save_types: declaration.document.save_types.iter().cloned().collect(),
                supports_open_in_place: declaration.document.supports_open_in_place,
            },
            credentials: RuntimeAppCredentialDeclaration {
                biometric_usage: declaration.credentials.biometric_usage.clone(),
                access_groups: declaration
                    .credentials
                    .access_groups
                    .iter()
                    .cloned()
                    .collect(),
                credential_domains: declaration
                    .credentials
                    .credential_domains
                    .iter()
                    .cloned()
                    .collect(),
            },
            location: RuntimeAppLocationDeclaration {
                allows_background_updates: declaration.location.allows_background_updates,
                precise_by_default: declaration.location.precise_by_default,
                temporary_precise_purposes: declaration
                    .location
                    .temporary_precise_purposes
                    .iter()
                    .cloned()
                    .collect(),
            },
        }
    }
}

/// Runtime-facing permission selector used by Host availability checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RuntimeAppPermission {
    /// Foreground location access.
    Location,
    /// Background location access.
    LocationBackground,
    /// Camera access.
    Camera,
    /// Microphone access.
    Microphone,
    /// Bluetooth access.
    Bluetooth,
    /// Notification delivery access.
    Notifications,
    /// Contacts read access.
    ContactsRead,
    /// Contacts write access.
    ContactsWrite,
    /// Media-library read access.
    MediaRead,
    /// Media-library write access.
    MediaWrite,
    /// Motion or activity sensor access.
    Motion,
    /// Clipboard read access.
    ClipboardRead,
    /// Calendar read access.
    CalendarRead,
    /// Calendar write access.
    CalendarWrite,
}

impl From<TargetAppPermission> for RuntimeAppPermission {
    fn from(permission: TargetAppPermission) -> Self {
        match permission {
            TargetAppPermission::Location => Self::Location,
            TargetAppPermission::LocationBackground => Self::LocationBackground,
            TargetAppPermission::Camera => Self::Camera,
            TargetAppPermission::Microphone => Self::Microphone,
            TargetAppPermission::Bluetooth => Self::Bluetooth,
            TargetAppPermission::Notifications => Self::Notifications,
            TargetAppPermission::ContactsRead => Self::ContactsRead,
            TargetAppPermission::ContactsWrite => Self::ContactsWrite,
            TargetAppPermission::MediaRead => Self::MediaRead,
            TargetAppPermission::MediaWrite => Self::MediaWrite,
            TargetAppPermission::Motion => Self::Motion,
            TargetAppPermission::ClipboardRead => Self::ClipboardRead,
            TargetAppPermission::CalendarRead => Self::CalendarRead,
            TargetAppPermission::CalendarWrite => Self::CalendarWrite,
        }
    }
}

/// Runtime execution options for scheduler, time, randomness, and heap behavior.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct RuntimeOptions {
    /// Execution host semantics for this runtime.
    pub host: Runtime,
    /// Runtime version for versioned library selection.
    pub version: Option<String>,
    /// Stable runtime name for policy selection.
    pub name: Option<String>,
    /// Runtime labels for policy selection.
    pub labels: BTreeMap<String, String>,
    /// Default primary worker identity for policy selection.
    pub primary_worker: RuntimeWorkerOptions,
    /// Resolved target app declaration for host availability checks.
    pub app: RuntimeAppDeclaration,
    /// Execution mode for runtime scheduling and replay.
    pub execution: ExecutionMode,
    /// Default world for bindings without a matching rule.
    pub world: RuntimeWorld,
    /// Default access policy for bindings without a matching access rule.
    pub access: RuntimeAccess,
    /// Ordered static runtime rules.
    pub rules: Vec<RuntimeRule>,
    /// Replay configuration.
    pub replay: ReplayOptions,
    /// Runtime clock configuration.
    pub time: TimeOptions,
    /// Runtime randomness configuration.
    pub random: RandomOptions,
    /// Runtime scheduler configuration.
    pub scheduler: SchedulerOptions,
    /// Runtime heap configuration.
    pub heap: HeapOptions,
    /// Runtime diagnostics configuration.
    pub diagnostic: RuntimeDiagnosticOptions,
    /// Global filesystem runtime defaults.
    pub fs: PlatformFsOptions,
    /// Global network runtime defaults.
    pub net: PlatformNetOptions,
    /// Global process runtime defaults.
    pub process: PlatformProcessOptions,
    /// Global audio runtime defaults.
    pub audio: PlatformAudioOptions,
    /// Global input runtime defaults.
    pub input: PlatformInputOptions,
    /// Global GPU runtime defaults.
    pub gpu: PlatformGpuOptions,
    /// Global TLS runtime defaults.
    pub tls: PlatformTlsOptions,
    /// Global security runtime defaults.
    pub security: PlatformSecurityOptions,
    /// Global OS service runtime defaults.
    pub os: PlatformOsOptions,
    /// Global device service runtime defaults.
    pub device: PlatformDeviceOptions,
    /// Debug runtime options.
    pub debug: PlatformDebugOptions,
    /// Display runtime options.
    pub display: PlatformDisplayOptions,
    /// Error runtime options.
    pub error: PlatformErrorOptions,
    /// FFI runtime options.
    pub ffi: PlatformFfiOptions,
    /// I/O runtime options.
    pub io: PlatformIoOptions,
    /// IPC runtime options.
    pub ipc: PlatformIpcOptions,
    /// Resource runtime options.
    pub resource: PlatformResourceOptions,
    /// Thread runtime options.
    pub thread: PlatformThreadOptions,
    /// TTY runtime options.
    pub tty: PlatformTtyOptions,
    /// Global crypto runtime defaults.
    pub crypto: PlatformCryptoOptions,
    /// Platform-specific host runtime overrides.
    pub platform: PlatformOptions,
}

/// Runtime config JSON.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum RuntimeConfigJson {
    /// Runtime host shorthand.
    Host(String),
    /// Full runtime configuration object.
    Options(Box<RuntimeOptionsJson>),
}

impl Default for RuntimeConfigJson {
    fn default() -> Self {
        Self::Options(Box::default())
    }
}

impl RuntimeConfigJson {
    /// Return this runtime config as object form.
    pub fn as_options_json(&self) -> RuntimeOptionsJson {
        match self {
            Self::Host(host) => RuntimeOptionsJson {
                host: Some(host.clone()),
                ..RuntimeOptionsJson::default()
            },
            Self::Options(options) => options.as_ref().clone(),
        }
    }
}

pub(crate) fn runtime_options_from_json(json: Option<&RuntimeConfigJson>) -> RuntimeOptions {
    runtime_options_with_base(&RuntimeOptions::default(), json)
}

/// Derive runtime options from a base set of options plus overrides.
pub(crate) fn runtime_options_with_base(
    base: &RuntimeOptions,
    overrides: Option<&RuntimeConfigJson>,
) -> RuntimeOptions {
    // start from the base options
    let mut options = base.clone();

    // apply overrides when present
    if let Some(overrides) = overrides {
        overrides.as_options_json().apply_to(&mut options);
    }

    options
}

/// Runtime options (object form).
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct RuntimeOptionsJson {
    /// Execution host semantics for this runtime.
    pub host: Option<String>,
    /// Runtime version for versioned library selection.
    pub version: Option<String>,
    /// Stable runtime name for policy selection.
    pub name: Option<String>,
    /// Runtime labels for policy selection.
    pub labels: Option<BTreeMap<String, String>>,
    /// Default primary worker identity for policy selection.
    pub primary_worker: Option<RuntimeWorkerOptionsJson>,
    /// Execution mode for runtime scheduling and replay.
    pub execution: Option<ExecutionModeJson>,
    /// Default world for bindings without matching world rules.
    pub world: Option<RuntimeWorldJson>,
    /// Default access policy for bindings without matching access rules.
    pub access: Option<RuntimeAccessJson>,
    /// Ordered static runtime rules.
    pub rules: Option<Vec<RuntimeRuleJson>>,
    /// Replay configuration.
    pub replay: Option<ReplayOptionsJson>,
    /// Runtime clock configuration.
    pub time: Option<TimeOptionsJson>,
    /// Runtime randomness configuration.
    pub random: Option<RandomOptionsJson>,
    /// Runtime scheduler configuration.
    pub scheduler: Option<SchedulerOptionsJson>,
    /// Runtime heap configuration.
    pub heap: Option<HeapOptionsJson>,
    /// Runtime diagnostics configuration.
    pub diagnostic: Option<RuntimeDiagnosticOptionsJson>,
    /// Global filesystem runtime defaults.
    pub fs: Option<PlatformFsOptionsJson>,
    /// Global network runtime defaults.
    pub net: Option<PlatformNetOptionsJson>,
    /// Global process runtime defaults.
    pub process: Option<PlatformProcessOptionsJson>,
    /// Global audio runtime defaults.
    pub audio: Option<PlatformAudioOptionsJson>,
    /// Global input runtime defaults.
    pub input: Option<PlatformInputOptionsJson>,
    /// Global GPU runtime defaults.
    pub gpu: Option<PlatformGpuOptionsJson>,
    /// Global TLS runtime defaults.
    pub tls: Option<PlatformTlsOptionsJson>,
    /// Global security runtime defaults.
    pub security: Option<PlatformSecurityOptionsJson>,
    /// Global OS service runtime defaults.
    pub os: Option<PlatformOsOptionsJson>,
    /// Global device service runtime defaults.
    pub device: Option<PlatformDeviceOptionsJson>,
    /// Global crypto runtime defaults.
    pub crypto: Option<PlatformCryptoOptionsJson>,
    /// Global debug runtime defaults.
    pub debug: Option<PlatformDebugOptionsJson>,
    /// Global display runtime defaults.
    pub display: Option<PlatformDisplayOptionsJson>,
    /// Global error runtime defaults.
    pub error: Option<PlatformErrorOptionsJson>,
    /// Global ffi runtime defaults.
    pub ffi: Option<PlatformFfiOptionsJson>,
    /// Global io runtime defaults.
    pub io: Option<PlatformIoOptionsJson>,
    /// Global ipc runtime defaults.
    pub ipc: Option<PlatformIpcOptionsJson>,
    /// Global resource runtime defaults.
    pub resource: Option<PlatformResourceOptionsJson>,
    /// Global thread runtime defaults.
    pub thread: Option<PlatformThreadOptionsJson>,
    /// Global tty runtime defaults.
    pub tty: Option<PlatformTtyOptionsJson>,
    /// Platform-specific host runtime overrides.
    pub platform: Option<PlatformOptionsJson>,
}

/// Primary runtime worker options for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct RuntimeWorkerOptionsJson {
    /// Primary worker name for policy selection.
    pub name: Option<String>,
    /// Primary worker labels for policy selection.
    pub labels: Option<BTreeMap<String, String>>,
}

impl RuntimeWorkerOptionsJson {
    /// Apply primary worker overrides to one base set of runtime worker options.
    pub fn apply_to(&self, options: &mut RuntimeWorkerOptions) {
        // apply primary worker name override
        if let Some(name) = &self.name {
            options.name = Some(name.clone());
        }

        // apply primary worker label override
        if let Some(labels) = &self.labels {
            options.labels = labels.clone();
        }
    }
}

impl RuntimeOptionsJson {
    /// Inherit unset runtime settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.host.is_none() {
            self.host = parent.host.clone();
        }
        if self.version.is_none() {
            self.version = parent.version.clone();
        }
        if self.name.is_none() {
            self.name = parent.name.clone();
        }
        if self.labels.is_none() {
            self.labels = parent.labels.clone();
        }
        if self.primary_worker.is_none() {
            self.primary_worker = parent.primary_worker.clone();
        }
        if self.execution.is_none() {
            self.execution = parent.execution;
        }
        if self.world.is_none() {
            self.world = parent.world;
        }
        if self.access.is_none() {
            self.access = parent.access;
        }
        if self.rules.is_none() {
            self.rules = parent.rules.clone();
        }
        if self.replay.is_none() {
            self.replay = parent.replay.clone();
        }
        if self.time.is_none() {
            self.time = parent.time.clone();
        }
        if self.random.is_none() {
            self.random = parent.random.clone();
        }
        if let Some(scheduler) = &mut self.scheduler {
            if let Some(parent_scheduler) = &parent.scheduler {
                scheduler.extend_from(parent_scheduler);
            }
        } else {
            self.scheduler = parent.scheduler.clone();
        }
        if let Some(heap) = &mut self.heap {
            if let Some(parent_heap) = &parent.heap {
                heap.extend_from(parent_heap);
            }
        } else {
            self.heap = parent.heap.clone();
        }
        if self.diagnostic.is_none() {
            self.diagnostic = parent.diagnostic.clone();
        }
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
        if self.security.is_none() {
            self.security = parent.security.clone();
        }
        if self.os.is_none() {
            self.os = parent.os.clone();
        }
        if self.device.is_none() {
            self.device = parent.device.clone();
        }
        if self.crypto.is_none() {
            self.crypto = parent.crypto.clone();
        }
        if self.debug.is_none() {
            self.debug = parent.debug.clone();
        }
        if self.display.is_none() {
            self.display = parent.display.clone();
        }
        if self.error.is_none() {
            self.error = parent.error.clone();
        }
        if self.ffi.is_none() {
            self.ffi = parent.ffi.clone();
        }
        if self.io.is_none() {
            self.io = parent.io.clone();
        }
        if self.ipc.is_none() {
            self.ipc = parent.ipc.clone();
        }
        if self.resource.is_none() {
            self.resource = parent.resource.clone();
        }
        if self.thread.is_none() {
            self.thread = parent.thread.clone();
        }
        if self.tty.is_none() {
            self.tty = parent.tty.clone();
        }
        if self.platform.is_none() {
            self.platform = parent.platform.clone();
        }
    }

    /// Apply runtime option overrides to a base set of options.
    pub fn apply_to(&self, options: &mut RuntimeOptions) {
        // host and version
        if let Some(host) = &self.host
            && let Some(host) = Runtime::parse(host)
        {
            options.host = host;
        }
        if let Some(version) = &self.version {
            options.version = Some(version.clone());
        }

        // apply runtime identity overrides
        if let Some(name) = &self.name {
            options.name = Some(name.clone());
        }
        if let Some(labels) = &self.labels {
            options.labels = labels.clone();
        }

        // apply default primary worker identity overrides
        if let Some(primary_worker) = &self.primary_worker {
            primary_worker.apply_to(&mut options.primary_worker);
        }

        // apply execution mode overrides
        if let Some(execution_mode) = self.execution {
            options.execution = ExecutionMode::from(execution_mode);
        }

        // apply default world overrides
        if let Some(default_world) = self.world {
            options.world = RuntimeWorld::from(default_world);
        }

        // apply default access overrides
        if let Some(default_access) = self.access {
            options.access = RuntimeAccess::from(default_access);
        }

        // apply static runtime rules
        if let Some(rules) = &self.rules {
            options.rules = rules.iter().map(RuntimeRule::from).collect();
        }

        // apply replay overrides
        if let Some(replay) = &self.replay {
            replay.apply_to(&mut options.replay);
        }

        // apply time overrides
        if let Some(time) = &self.time {
            time.apply_to(&mut options.time);
        }

        // apply random overrides
        if let Some(random) = &self.random {
            random.apply_to(&mut options.random);
        }

        // apply scheduler overrides
        if let Some(scheduler) = &self.scheduler {
            scheduler.apply_to(&mut options.scheduler);
        }

        // apply heap overrides
        if let Some(heap) = &self.heap {
            heap.apply_to(&mut options.heap);
        }

        // apply diagnostic overrides
        if let Some(diagnostic) = &self.diagnostic {
            diagnostic.apply_to(&mut options.diagnostic);
        }

        // apply filesystem defaults
        if let Some(fs) = &self.fs {
            fs.apply_to(&mut options.fs);
        }

        // apply network defaults
        if let Some(net) = &self.net {
            net.apply_to(&mut options.net);
        }

        // apply process defaults
        if let Some(process) = &self.process {
            process.apply_to(&mut options.process);
        }

        // apply audio defaults
        if let Some(audio) = &self.audio {
            audio.apply_to(&mut options.audio);
        }

        // apply input defaults
        if let Some(input) = &self.input {
            input.apply_to(&mut options.input);
        }

        // apply gpu defaults
        if let Some(gpu) = &self.gpu {
            gpu.apply_to(&mut options.gpu);
        }

        // apply tls defaults
        if let Some(tls) = &self.tls {
            tls.apply_to(&mut options.tls);
        }

        // apply security defaults
        if let Some(security) = &self.security {
            security.apply_to(&mut options.security);
        }

        // apply os defaults
        if let Some(os) = &self.os {
            os.apply_to(&mut options.os);
        }

        // apply device defaults
        if let Some(device) = &self.device {
            device.apply_to(&mut options.device);
        }

        // apply crypto defaults
        if let Some(crypto) = &self.crypto {
            crypto.apply_to(&mut options.crypto);
        }

        // apply debug defaults
        if let Some(debug) = &self.debug {
            debug.apply_to(&mut options.debug);
        }

        // apply display defaults
        if let Some(display) = &self.display {
            display.apply_to(&mut options.display);
        }

        // apply error defaults
        if let Some(error) = &self.error {
            error.apply_to(&mut options.error);
        }

        // apply ffi defaults
        if let Some(ffi) = &self.ffi {
            ffi.apply_to(&mut options.ffi);
        }

        // apply io defaults
        if let Some(io) = &self.io {
            io.apply_to(&mut options.io);
        }

        // apply ipc defaults
        if let Some(ipc) = &self.ipc {
            ipc.apply_to(&mut options.ipc);
        }

        // apply resource defaults
        if let Some(resource) = &self.resource {
            resource.apply_to(&mut options.resource);
        }

        // apply thread defaults
        if let Some(thread) = &self.thread {
            thread.apply_to(&mut options.thread);
        }

        // apply tty defaults
        if let Some(tty) = &self.tty {
            tty.apply_to(&mut options.tty);
        }

        // apply platform overrides
        if let Some(platform) = &self.platform {
            platform.apply_to(&mut options.platform);
        }
    }
}
