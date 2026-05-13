use destack_core::fnv1a_128;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};

/// Builtin profile name for one empty action set.
pub const ACTION_PROFILE_NONE: &str = "none";

/// Builtin profile name for one permissive action set.
pub const ACTION_PROFILE_ALL: &str = "all";

/// Builtin profile alias for one permissive action set.
pub const ACTION_PROFILE_FULL: &str = "full";

macro_rules! define_actions {
    ($($namespace:literal { $($variant:ident => $suffix:literal,)+ })+) => {
        /// Canonical policy action.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Action {
            $($(
                #[doc = concat!("Action `", $namespace, ".", $suffix, "`.")]
                $variant,
            )+)+
        }

        impl Action {
            /// All canonical actions.
            pub const ALL: &'static [Self] = &[
                $($(Self::$variant,)+)+
            ];

            /// Return the canonical action name.
            pub const fn name(self) -> &'static str {
                match self {
                    $($(Self::$variant => concat!($namespace, ".", $suffix),)+)+
                }
            }

            /// Return the stable action identifier.
            pub const fn id(self) -> ActionId {
                ActionId::from_name(self.name())
            }

            /// Resolve one action from one canonical action name.
            pub fn from_name(name: &str) -> Option<Self> {
                match name {
                    $($(concat!($namespace, ".", $suffix) => Some(Self::$variant),)+)+
                    _ => None,
                }
            }

            /// Return whether one action name exists in the canonical action table.
            pub fn is_known_name(name: &str) -> bool {
                Self::from_name(name).is_some()
            }
        }
    };
}

define_actions! {
    "host" {
        AudioBackend => "audio.backend",
        AudioClock => "audio.clock",
        AudioDevice => "audio.device",
        AudioDeviceWatch => "audio.device.watch",
        AudioEvent => "audio.event",
        AudioRead => "audio.read",
        AudioStream => "audio.stream",
        AudioWrite => "audio.write",
        AccessibilityAction => "accessibility.action",
        AccessibilityNotification => "accessibility.notification",
        AccessibilityText => "accessibility.text",
        AccessibilityTree => "accessibility.tree",
        CryptoCertificateRead => "crypto.certificate.read",
        CryptoCertificateWrite => "crypto.certificate.write",
        CryptoCapabilityRead => "crypto.capability.read",
        CryptoCipher => "crypto.cipher",
        CryptoDigest => "crypto.digest",
        CryptoKdf => "crypto.kdf",
        CryptoKeyAgree => "crypto.key.agree",
        CryptoKeyDecrypt => "crypto.key.decrypt",
        CryptoKeyEncrypt => "crypto.key.encrypt",
        CryptoKeyExport => "crypto.key.export",
        CryptoKeyGenerate => "crypto.key.generate",
        CryptoKeyImport => "crypto.key.import",
        CryptoKeySign => "crypto.key.sign",
        CryptoKeyDelete => "crypto.key.delete",
        CryptoKeyUnwrap => "crypto.key.unwrap",
        CryptoKeyVerify => "crypto.key.verify",
        CryptoKeyWrap => "crypto.key.wrap",
        CryptoMac => "crypto.mac",
        CryptoStoreRead => "crypto.store.read",
        CryptoStoreWrite => "crypto.store.write",
        DisplayBackend => "display.backend",
        DisplayDrag => "display.drag",
        DisplayEvent => "display.event",
        DisplayFrame => "display.frame",
        DisplayMonitor => "display.monitor",
        DisplayMonitorMode => "display.monitor.mode",
        DisplayWindow => "display.window",
        ConsoleWrite => "console.write",
        DeviceBluetoothConnect => "device.bluetooth.connect",
        DeviceBluetoothGatt => "device.bluetooth.gatt",
        DeviceBluetoothList => "device.bluetooth.list",
        DeviceBluetoothScan => "device.bluetooth.scan",
        DeviceCameraCapture => "device.camera.capture",
        DeviceCameraConfigure => "device.camera.configure",
        DeviceCameraList => "device.camera.list",
        DeviceCameraOpen => "device.camera.open",
        DeviceHidFeature => "device.hid.feature",
        DeviceHidList => "device.hid.list",
        DeviceHidOpen => "device.hid.open",
        DeviceHidRead => "device.hid.read",
        DeviceHidWrite => "device.hid.write",
        DeviceMidiList => "device.midi.list",
        DeviceMidiObserve => "device.midi.observe",
        DeviceMidiOpen => "device.midi.open",
        DeviceMidiRead => "device.midi.read",
        DeviceMidiVirtual => "device.midi.virtual",
        DeviceMidiWrite => "device.midi.write",
        DeviceSerialConfigure => "device.serial.configure",
        DeviceSerialList => "device.serial.list",
        DeviceSerialObserve => "device.serial.observe",
        DeviceSerialOpen => "device.serial.open",
        DeviceSerialRead => "device.serial.read",
        DeviceSerialWrite => "device.serial.write",
        DeviceUsbConfigure => "device.usb.configure",
        DeviceUsbList => "device.usb.list",
        DeviceUsbOpen => "device.usb.open",
        DeviceUsbTransfer => "device.usb.transfer",
        FsLink => "fs.link",
        FsLock => "fs.lock",
        FsMetadata => "fs.metadata",
        FsMode => "fs.mode",
        FsMountList => "fs.mount.list",
        FsNode => "fs.node",
        FsOpen => "fs.open",
        FsOwner => "fs.owner",
        FsRead => "fs.read",
        FsSync => "fs.sync",
        FsTemp => "fs.temp",
        FsWatch => "fs.watch",
        FsWrite => "fs.write",
        FsXattr => "fs.xattr",
        GpuAdapter => "gpu.adapter",
        GpuBind => "gpu.bind",
        GpuCommand => "gpu.command",
        GpuDebug => "gpu.debug",
        GpuDevice => "gpu.device",
        GpuPresent => "gpu.present",
        GpuPipeline => "gpu.pipeline",
        GpuQueue => "gpu.queue",
        GpuResource => "gpu.resource",
        GpuSync => "gpu.sync",
        InputClipboardRead => "input.clipboard.read",
        InputClipboardWrite => "input.clipboard.write",
        InputDevice => "input.device",
        InputEvent => "input.event",
        InputGamepad => "input.gamepad",
        InputGrab => "input.grab",
        InputHaptics => "input.haptics",
        InputKeyboard => "input.keyboard",
        InputPointer => "input.pointer",
        InputSensor => "input.sensor",
        InputText => "input.text",
        InputTouch => "input.touch",
        IpcEventCounter => "ipc.eventCounter",
        IpcFutex => "ipc.futex",
        IpcHandlePass => "ipc.handle.pass",
        IpcMemory => "ipc.memory",
        IpcMessage => "ipc.message",
        IpcPipe => "ipc.pipe",
        IpcSemaphore => "ipc.semaphore",
        IpcUnix => "ipc.unix",
        IoCompletion => "io.completion",
        IoControl => "io.control",
        IoPoll => "io.poll",
        IoUring => "io.uring",
        MemoryAdvise => "memory.advise",
        MemoryDiscard => "memory.discard",
        MemoryExecute => "memory.execute",
        MemoryLayoutRead => "memory.layout.read",
        MemoryLock => "memory.lock",
        MemoryMap => "memory.map",
        MemoryVirtual => "memory.virtual",
        NetAccept => "net.accept",
        NetClose => "net.close",
        NetConnect => "net.connect",
        NetConnectivityRead => "net.connectivity.read",
        NetConnectivityWatch => "net.connectivity.watch",
        NetDns => "net.dns",
        NetInterface => "net.interface",
        NetListen => "net.listen",
        NetMulticast => "net.multicast",
        NetOpen => "net.open",
        NetOption => "net.option",
        NetRaw => "net.raw",
        NetRead => "net.read",
        NetRouteRead => "net.route.read",
        NetRouteWrite => "net.route.write",
        NetStatus => "net.status",
        NetWrite => "net.write",
        OsCapacityRead => "os.capacity.read",
        OsIdentityRead => "os.identity.read",
        OsPowerRead => "os.power.read",
        OsPressureRead => "os.pressure.read",
        ProcessAffinity => "process.affinity",
        ProcessArguments => "process.arguments",
        ProcessCgroupRead => "process.cgroup.read",
        ProcessCgroupWrite => "process.cgroup.write",
        ProcessEnvironmentRead => "process.environment.read",
        ProcessEnvironmentWrite => "process.environment.write",
        ProcessHandle => "process.handle",
        ProcessExec => "process.exec",
        ProcessExit => "process.exit",
        ProcessIdentityRead => "process.identity.read",
        ProcessIdentityWrite => "process.identity.write",
        ProcessJob => "process.job",
        ProcessLimitRead => "process.limit.read",
        ProcessLimitWrite => "process.limit.write",
        ProcessNamespace => "process.namespace",
        ProcessPriority => "process.priority",
        ProcessSpawn => "process.spawn",
        ProcessStdio => "process.stdio",
        ProcessSession => "process.session",
        ProcessSignalReceive => "process.signal.receive",
        ProcessSignalSend => "process.signal.send",
        ProcessUmask => "process.umask",
        ProcessWait => "process.wait",
        ProcessWorkdirRead => "process.workdir.read",
        ProcessWorkdirWrite => "process.workdir.write",
        RandomBytes => "random.bytes",
        TimeClockRead => "time.clock.read",
        TimeTimer => "time.timer",
        TlsCertificateRead => "tls.certificate.read",
        TlsConnection => "tls.connection",
        TlsContext => "tls.context",
        TlsExporter => "tls.exporter",
        TlsHandshake => "tls.handshake",
        TlsHostnameVerify => "tls.hostname.verify",
        TlsIdentityUse => "tls.identity.use",
        TlsIdentityWrite => "tls.identity.write",
        TlsPolicy => "tls.policy",
        TlsResumption => "tls.resumption",
        TlsTrustWrite => "tls.trust.write",
        TtyClose => "tty.close",
        TtyMode => "tty.mode",
        TtyOpen => "tty.open",
        TtyPty => "tty.pty",
        TtyPtyRead => "tty.pty.read",
        TtyPtyWrite => "tty.pty.write",
        TtyRead => "tty.read",
        TtySize => "tty.size",
        TtyStatus => "tty.status",
        TtyTermios => "tty.termios",
        TtyWrite => "tty.write",
    }
    "runtime" {
        BranchControl => "branch.control",
        BranchRead => "branch.read",
        CheckpointCreate => "checkpoint.create",
        CheckpointRead => "checkpoint.read",
        Control => "control",
        EntityRead => "entity.read",
        EntityWrite => "entity.write",
        ExecutorControl => "executor.control",
        ExecutorRead => "executor.read",
        PolicyRead => "policy.read",
        PolicyTest => "policy.test",
        PolicyWrite => "policy.write",
        RandomControl => "random.control",
        RandomRead => "random.read",
        ResourceClose => "resource.close",
        ResourceRead => "resource.read",
        ResourceTransfer => "resource.transfer",
        ResourceWrite => "resource.write",
        RevisionControl => "revision.control",
        RevisionRead => "revision.read",
        RuntimeControl => "runtime.control",
        RuntimeCreate => "runtime.create",
        RuntimeRead => "runtime.read",
        SnapshotCreate => "snapshot.create",
        SnapshotRead => "snapshot.read",
        SnapshotRestore => "snapshot.restore",
        TraceControl => "trace.control",
        TraceRead => "trace.read",
        WorkerControl => "worker.control",
        WorkerCreate => "worker.create",
        WorkerRead => "worker.read",
        WorldControl => "world.control",
        WorldCreate => "world.create",
        WorldRead => "world.read",
    }
}

/// Stable identifier for one canonical action name.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActionId(
    /// Stable hash of the canonical action string.
    pub u128,
);

impl ActionId {
    /// Build one action identifier from one action name.
    pub const fn from_name(name: &str) -> Self {
        Self(fnv1a_128(name.as_bytes()))
    }
}

/// Set of canonical action identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ActionSet {
    /// Stored action identifiers.
    identifiers: FxHashSet<ActionId>,
}

impl ActionSet {
    /// Create one empty action set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create one action set from one iterator of action names.
    pub fn from_names<T>(names: impl IntoIterator<Item = T>) -> Self
    where
        T: AsRef<str>,
    {
        let mut set = Self::new();
        set.extend_names(names);

        set
    }

    /// Create one action set from one iterator of action identifiers.
    pub fn from_ids(ids: impl IntoIterator<Item = ActionId>) -> Self {
        ids.into_iter().collect()
    }

    /// Create one action set from one iterator of action kinds.
    pub fn from_actions(actions: impl IntoIterator<Item = Action>) -> Self {
        let action_ids = actions.into_iter().map(Action::id);
        Self::from_ids(action_ids)
    }

    /// Insert one action identifier.
    pub fn insert_id(&mut self, action: ActionId) -> bool {
        self.identifiers.insert(action)
    }

    /// Insert one action name.
    pub fn insert_name(&mut self, action_name: &str) -> bool {
        self.insert_id(ActionId::from_name(action_name))
    }

    /// Insert one action kind.
    pub fn insert_action(&mut self, action: Action) -> bool {
        self.insert_id(action.id())
    }

    /// Extend this set from one iterator of action names.
    pub fn extend_names<T>(&mut self, names: impl IntoIterator<Item = T>)
    where
        T: AsRef<str>,
    {
        for name in names {
            self.insert_name(name.as_ref());
        }
    }

    /// Extend this set from one iterator of action kinds.
    pub fn extend_actions(&mut self, actions: impl IntoIterator<Item = Action>) {
        for action in actions {
            self.insert_action(action);
        }
    }

    /// Return whether this set contains one action identifier.
    pub fn contains_id(&self, action: ActionId) -> bool {
        self.identifiers.contains(&action)
    }

    /// Return whether this set contains one action name.
    pub fn contains_name(&self, action_name: &str) -> bool {
        self.contains_id(ActionId::from_name(action_name))
    }

    /// Return whether this set contains one action kind.
    pub fn contains_action(&self, action: Action) -> bool {
        self.contains_id(action.id())
    }

    /// Return the number of actions in this set.
    pub fn len(&self) -> usize {
        self.identifiers.len()
    }

    /// Return whether this set is empty.
    pub fn is_empty(&self) -> bool {
        self.identifiers.is_empty()
    }

    /// Iterate the action identifiers in this set.
    pub fn iter(&self) -> impl Iterator<Item = &ActionId> {
        self.identifiers.iter()
    }
}

impl FromIterator<ActionId> for ActionSet {
    fn from_iter<T: IntoIterator<Item = ActionId>>(iter: T) -> Self {
        let mut set = Self::new();
        set.identifiers.extend(iter);

        set
    }
}

impl FromIterator<Action> for ActionSet {
    fn from_iter<T: IntoIterator<Item = Action>>(iter: T) -> Self {
        let mut set = Self::new();
        set.extend_actions(iter);

        set
    }
}

/// Return all builtin profile names.
pub const fn builtin_action_profiles() -> &'static [&'static str] {
    &[ACTION_PROFILE_NONE, ACTION_PROFILE_ALL, ACTION_PROFILE_FULL]
}

/// Resolve one action profile expression into one action set.
///
/// Supports builtin profile names (`none`, `all`, `full`) and explicit
/// comma or whitespace separated action name lists.
pub fn resolve_action_profile(profile: &str) -> Result<ActionSet, String> {
    let profile = profile.trim();
    if profile.is_empty() {
        return Err("action profile must not be empty".to_string());
    }

    if profile.eq_ignore_ascii_case(ACTION_PROFILE_NONE) {
        return Ok(ActionSet::new());
    }

    if profile.eq_ignore_ascii_case(ACTION_PROFILE_ALL)
        || profile.eq_ignore_ascii_case(ACTION_PROFILE_FULL)
    {
        return Ok(ActionSet::from_actions(Action::ALL.iter().copied()));
    }

    let mut set = ActionSet::new();
    for action_name in profile_tokens(profile) {
        if !Action::is_known_name(action_name) {
            return Err(format!(
                "unknown action `{action_name}` in profile `{profile}`"
            ));
        }

        set.insert_name(action_name);
    }

    if set.is_empty() {
        return Err(format!(
            "action profile `{profile}` did not resolve to any actions"
        ));
    }

    Ok(set)
}

/// Split one profile expression into action name tokens.
fn profile_tokens(profile: &str) -> impl Iterator<Item = &str> {
    profile
        .split(|character: char| character == ',' || character.is_whitespace())
        .map(str::trim)
        .filter(|token| !token.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{
        ACTION_PROFILE_ALL, ACTION_PROFILE_NONE, Action, ActionId, ActionSet,
        builtin_action_profiles, resolve_action_profile,
    };

    #[test]
    fn test_from_actions_contains_action_kinds_and_ids() {
        let set = ActionSet::from_actions([Action::FsRead, Action::NetConnect]);

        assert!(set.contains_action(Action::FsRead));
        assert!(set.contains_action(Action::NetConnect));
        assert!(set.contains_id(ActionId::from_name("host.fs.read")));
        assert!(set.contains_id(ActionId::from_name("host.net.connect")));
    }

    #[test]
    fn test_extend_actions_merges_into_existing_set() {
        let mut set = ActionSet::from_names(["host.gpu.device"]);
        set.extend_actions([Action::GpuQueue, Action::GpuPresent]);

        assert!(set.contains_name("host.gpu.device"));
        assert!(set.contains_action(Action::GpuQueue));
        assert!(set.contains_action(Action::GpuPresent));
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn test_insert_action_deduplicates_existing_action() {
        let mut set = ActionSet::new();
        let first_insert = set.insert_action(Action::IoPoll);
        let second_insert = set.insert_action(Action::IoPoll);

        assert!(first_insert);
        assert!(!second_insert);
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn test_resolve_action_profile_accepts_builtin_profiles() {
        let none =
            resolve_action_profile(ACTION_PROFILE_NONE).expect("none profile should resolve");
        let all = resolve_action_profile(ACTION_PROFILE_ALL).expect("all profile should resolve");

        assert!(none.is_empty());
        assert!(!all.is_empty());
        assert!(all.len() > none.len());
    }

    #[test]
    fn test_resolve_action_profile_accepts_explicit_action_list() {
        let set = resolve_action_profile("host.fs.read, host.net.connect host.random.bytes")
            .expect("action list should resolve");

        assert!(set.contains_name("host.fs.read"));
        assert!(set.contains_name("host.net.connect"));
        assert!(set.contains_name("host.random.bytes"));
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn test_resolve_action_profile_rejects_unknown_action_name() {
        let error = resolve_action_profile("host.fs.read,not.a.action")
            .expect_err("unknown action should fail");

        assert!(error.contains("unknown action"));
    }

    #[test]
    fn test_builtin_action_profiles_lists_known_profiles() {
        let profiles = builtin_action_profiles();

        assert!(profiles.contains(&"none"));
        assert!(profiles.contains(&"all"));
        assert!(profiles.contains(&"full"));
    }
}
