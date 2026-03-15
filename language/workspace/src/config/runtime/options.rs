use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::config::target::{
    TargetAppBackgroundMode, TargetAppDeclaration, TargetAppForegroundMode,
    TargetAppNotificationCategoryDeclaration, TargetAppPermission,
};

#[cfg(test)]
use super::super::policy::ReplayPayloadMode;
use super::super::policy::{ExecutionMode, ExecutionModeJson};
#[cfg(test)]
use super::RuntimeSelector;
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

/// Default identity options for one runtime primary agent.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct RuntimeAgentOptions {
    /// Default primary agent name for policy selection.
    pub name: Option<String>,
    /// Default primary agent labels for policy selection.
    pub labels: BTreeMap<String, String>,
}

/// Runtime-facing app declaration used by Host availability checks.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct RuntimeAppDeclaration {
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
    /// Stable runtime name for policy selection.
    pub name: Option<String>,
    /// Runtime labels for policy selection.
    pub labels: BTreeMap<String, String>,
    /// Default primary agent identity for policy selection.
    pub primary_agent: RuntimeAgentOptions,
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
pub(crate) fn runtime_options_from_json(
    json: Option<&DsConfigRuntimeOptionsJson>,
) -> RuntimeOptions {
    runtime_options_with_base(&RuntimeOptions::default(), json)
}

/// Derive runtime options from a base set of options plus overrides.
pub(crate) fn runtime_options_with_base(
    base: &RuntimeOptions,
    overrides: Option<&DsConfigRuntimeOptionsJson>,
) -> RuntimeOptions {
    // start from the base options
    let mut options = base.clone();

    // apply overrides when present
    if let Some(overrides) = overrides {
        overrides.apply_to(&mut options);
    }

    options
}

/// Runtime options (top-level).
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct DsConfigRuntimeOptionsJson {
    /// Stable runtime name for policy selection.
    pub name: Option<String>,
    /// Runtime labels for policy selection.
    pub labels: Option<BTreeMap<String, String>>,
    /// Default primary agent identity for policy selection.
    pub primary_agent: Option<RuntimeAgentOptionsJson>,
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

/// Primary runtime agent options for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct RuntimeAgentOptionsJson {
    /// Primary agent name for policy selection.
    pub name: Option<String>,
    /// Primary agent labels for policy selection.
    pub labels: Option<BTreeMap<String, String>>,
}

impl RuntimeAgentOptionsJson {
    /// Apply primary agent overrides to one base set of runtime agent options.
    pub fn apply_to(&self, options: &mut RuntimeAgentOptions) {
        // apply primary agent name override
        if let Some(name) = &self.name {
            options.name = Some(name.clone());
        }

        // apply primary agent label override
        if let Some(labels) = &self.labels {
            options.labels = labels.clone();
        }
    }
}

impl DsConfigRuntimeOptionsJson {
    /// Apply runtime option overrides to a base set of options.
    pub fn apply_to(&self, options: &mut RuntimeOptions) {
        // apply runtime identity overrides
        if let Some(name) = &self.name {
            options.name = Some(name.clone());
        }
        if let Some(labels) = &self.labels {
            options.labels = labels.clone();
        }

        // apply default primary agent identity overrides
        if let Some(primary_agent) = &self.primary_agent {
            primary_agent.apply_to(&mut options.primary_agent);
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
#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        DsConfigRuntimeOptionsJson, ExecutionMode, ReplayPayloadMode, RuntimeAccess,
        RuntimeSelector, RuntimeWorld,
    };

    /// Ensure runtime options apply shorthand and object selector rules.
    #[test]
    fn test_runtime_options_apply_parses_shorthand_and_object_rules() {
        let runtime_json: DsConfigRuntimeOptionsJson = serde_json::from_value(json!({
            "rules": [
                {
                    "when": "destack.net.*",
                    "access": "deny"
                },
                {
                    "when": { "binding": "destack.fs.*" },
                    "world": "simulation"
                },
                {
                    "when": { "binding": "destack.crypto.*" },
                    "replay": "argumentsAndResults"
                }
            ]
        }))
        .expect("runtime options json should parse");

        let mut options = super::RuntimeOptions::default();
        runtime_json.apply_to(&mut options);

        assert_eq!(options.rules.len(), 3);
        assert_eq!(
            options.rules[0].when,
            RuntimeSelector::binding("destack.net.*")
        );
        assert_eq!(options.rules[0].access, Some(RuntimeAccess::Deny));
        assert_eq!(options.rules[0].world, None);
        assert_eq!(options.rules[0].replay, None);
        assert_eq!(
            options.rules[1].when,
            RuntimeSelector::binding("destack.fs.*")
        );
        assert_eq!(options.rules[1].access, None);
        assert_eq!(options.rules[1].world, Some(RuntimeWorld::Simulation));
        assert_eq!(options.rules[1].replay, None);
        assert_eq!(
            options.rules[2].when,
            RuntimeSelector::binding("destack.crypto.*")
        );
        assert_eq!(options.rules[2].access, None);
        assert_eq!(options.rules[2].world, None);
        assert_eq!(
            options.rules[2].replay,
            Some(ReplayPayloadMode::ArgumentsAndResults)
        );
    }

    /// Ensure runtime selector execution accepts one mode and many modes.
    #[test]
    fn test_runtime_options_apply_parses_execution_string_or_array() {
        let runtime_json: DsConfigRuntimeOptionsJson = serde_json::from_value(json!({
            "rules": [
                {
                    "when": { "binding": "destack.net.*", "execution": "record" },
                    "access": "deny"
                },
                {
                    "when": { "binding": "destack.fs.*", "execution": ["record", "replay"] },
                    "world": "simulation"
                }
            ]
        }))
        .expect("runtime options json should parse");

        let mut options = super::RuntimeOptions::default();
        runtime_json.apply_to(&mut options);

        assert_eq!(options.rules.len(), 2);
        assert_eq!(
            options.rules[0].when.execution_modes,
            Some(vec![ExecutionMode::Record])
        );
        assert_eq!(
            options.rules[1].when.execution_modes,
            Some(vec![ExecutionMode::Record, ExecutionMode::Replay])
        );
    }
}
