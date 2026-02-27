use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::policy::{ExecutionMode, ExecutionModeJson, ReplayPayloadMode, ReplayPayloadModeJson};

/// Policy for runtime task scheduling in thread pools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SchedulerPolicy {
    /// First-in, first-out scheduling.
    #[default]
    Fifo,
    /// Fair scheduling with time slicing.
    Fair,
    /// Work-stealing scheduling for throughput.
    WorkStealing,
}

/// Time source selection for runtime clocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum TimeMode {
    /// Use the host clock directly.
    #[default]
    Host,
    /// Use a virtualized clock derived from runtime state.
    Virtual,
}

/// Randomness source selection for the runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum RandomMode {
    /// Use the host randomness source.
    #[default]
    Host,
    /// Use deterministic runtime-managed randomness.
    Deterministic,
}

/// Garbage collector logging verbosity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum GcLogging {
    /// Disable GC logging.
    #[default]
    Off,
    /// Emit summary GC events.
    Summary,
    /// Emit verbose GC events.
    Verbose,
}

/// Replay log configuration for runtime record/replay.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReplayLogOptions {
    /// Base path for replay logs (file or directory).
    pub path: Option<PathBuf>,
    /// Template for auto-generated log file names.
    pub template: Option<String>,
    /// Chunk size in megabytes for log rotation.
    pub chunk_size_mb: Option<u64>,
    /// Replay payload selection for record mode.
    pub payload: ReplayPayloadMode,
}

impl Default for ReplayLogOptions {
    fn default() -> Self {
        Self {
            path: None,
            template: None,
            chunk_size_mb: None,
            payload: ReplayPayloadMode::ResultsOnly,
        }
    }
}

/// Runtime clock configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct TimeOptions {
    /// Clock mode selection.
    pub mode: TimeMode,
    /// Epoch in nanoseconds for virtual time.
    pub epoch_ns: Option<u64>,
    /// Tick size in nanoseconds for virtual time.
    pub tick_ns: Option<u64>,
    /// Time zone identifier or fixed offset string.
    pub time_zone: Option<String>,
}

/// Runtime randomness configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct RandomOptions {
    /// Randomness source selection.
    pub mode: RandomMode,
    /// Seed for deterministic randomness streams.
    pub seed: Option<u64>,
    /// Whether to use a per-task random stream.
    pub per_task: bool,
}

/// Runtime scheduler configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct SchedulerOptions {
    // event loop
    /// Event loop tick budget in nanoseconds.
    pub tick_budget_ns: Option<u64>,
    /// Maximum number of microtasks per tick.
    pub microtask_budget: Option<u64>,
    /// Maximum host semantic events dispatched in sequence before one poller event.
    pub host_event_budget: Option<u64>,
    /// Maximum microtask nesting depth.
    pub max_microtask_depth: Option<u64>,
    /// Timer resolution in nanoseconds.
    pub timer_resolution_ns: Option<u64>,
    /// Maximum timer coalescing window in nanoseconds.
    pub max_timer_coalesce_ns: Option<u64>,
    /// Preemption interval for long-running tasks in nanoseconds.
    pub preempt_interval_ns: Option<u64>,
    /// Platform poller backend selection.
    pub poller_backend: PollerBackend,

    // task pools
    /// Scheduling policy for task pools.
    pub policy: SchedulerPolicy,
    /// Number of worker threads for parallel tasks.
    pub worker_threads: Option<u64>,
    /// Number of I/O threads.
    pub io_threads: Option<u64>,
    /// Number of blocking worker threads.
    pub blocking_threads: Option<u64>,
    /// Maximum number of concurrent tasks.
    pub max_tasks: Option<u64>,
}

/// Platform poller backend selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum PollerBackend {
    /// Choose the best available backend for the platform.
    #[default]
    Auto,
    /// Use io_uring (Linux only).
    IoUring,
    /// Use epoll (Linux only).
    Epoll,
    /// Use kqueue (BSD/macOS only).
    Kqueue,
    /// Use poll (portable Unix fallback).
    Poll,
    /// Use the Windows readiness backend.
    Windows,
}

impl std::str::FromStr for PollerBackend {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().replace('-', "_").as_str() {
            "auto" => Ok(Self::Auto),
            "io_uring" | "uring" => Ok(Self::IoUring),
            "epoll" => Ok(Self::Epoll),
            "kqueue" => Ok(Self::Kqueue),
            "poll" => Ok(Self::Poll),
            "windows" => Ok(Self::Windows),
            _ => Err(()),
        }
    }
}

impl PollerBackend {
    /// Parse a poller backend from a string.
    pub fn parse(value: &str) -> Option<Self> {
        value.parse().ok()
    }
}

/// Runtime garbage collector configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GcOptions {
    /// Whether the garbage collector is enabled.
    pub enabled: bool,
    /// Heap growth target percentage.
    pub heap_growth_percent: u32,
    /// Soft heap limit in bytes.
    pub heap_soft_limit_bytes: Option<u64>,
    /// Initial heap size hint in bytes.
    pub heap_initial_bytes: Option<u64>,
    /// GC logging verbosity.
    pub logging: GcLogging,
}

impl Default for GcOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            heap_growth_percent: 100,
            heap_soft_limit_bytes: None,
            heap_initial_bytes: None,
            logging: GcLogging::Off,
        }
    }
}

/// Runtime world selection for external bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum RuntimeWorld {
    /// Use host-backed platform bindings.
    #[default]
    Host,
    /// Use simulation-backed platform bindings.
    Simulation,
}

/// Runtime access policy for binding execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum RuntimeAccess {
    /// Allow the binding call.
    #[default]
    Allow,
    /// Deny the binding call.
    Deny,
}

/// Engine selector for runtime rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BindingEngine {
    /// Match VM engine execution.
    Vm,
    /// Match native engine execution.
    Native,
}

/// Binding scope selector for runtime rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BindingScope {
    /// Match host scope bindings.
    Host,
    /// Match runtime scope bindings.
    Runtime,
}

/// Blocking behavior selector for runtime rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BindingBlocking {
    /// Match always-blocking bindings.
    Always,
    /// Match never-blocking bindings.
    Never,
    /// Match conditionally blocking bindings.
    Sometimes,
}

/// Binding effect class selector for runtime rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BindingEffect {
    /// Match pure bindings.
    Pure,
    /// Match deterministic bindings.
    Deterministic,
    /// Match recordable external bindings.
    ExternalRecordable,
    /// Match non-recordable external bindings.
    ExternalNonRecordable,
}

/// Rule filters for runtime binding policies.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct RuntimeFilter {
    /// Glob selector for full binding names.
    pub binding: Option<String>,
    /// Glob selector for capability names.
    pub capability: Option<String>,
    /// Glob selector for component names.
    pub component: Option<String>,
    /// Glob selector for module names.
    pub module: Option<String>,
    /// Engine selector.
    pub engine: Option<BindingEngine>,
    /// Execution modes selector.
    pub execution_modes: Option<Vec<ExecutionMode>>,
    /// Platform selector.
    pub platforms: Option<Vec<String>>,
    /// Binding scope selector.
    pub scope: Option<BindingScope>,
    /// Binding blocking selector.
    pub blocking: Option<BindingBlocking>,
    /// Binding effect selector.
    pub effect: Option<BindingEffect>,
}

impl RuntimeFilter {
    /// Whether this selector has no filtering clauses.
    pub fn is_empty(&self) -> bool {
        self.binding.is_none()
            && self.capability.is_none()
            && self.component.is_none()
            && self.module.is_none()
            && self.engine.is_none()
            && self.execution_modes.is_none()
            && self.platforms.is_none()
            && self.scope.is_none()
            && self.blocking.is_none()
            && self.effect.is_none()
    }
}

/// Jitter distribution for runtime delay faults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuntimeJitterDistribution {
    /// Use a uniform random distribution.
    Uniform,
    /// Use a normal random distribution.
    Normal,
    /// Use an exponential random distribution.
    Exponential,
}

/// Activation behavior for runtime rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuntimeRuleActivation {
    /// Activate the rule immediately.
    Immediate,
    /// Activate once virtual time reaches this timestamp.
    AtVirtualNs {
        /// Activation timestamp in virtual nanoseconds.
        virtual_ns: u64,
    },
    /// Activate once this number of matching calls has elapsed.
    AfterCallCount {
        /// Matching call count before activation.
        call_count: u64,
    },
}

/// Lifetime behavior for runtime rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuntimeRuleLifetime {
    /// Keep the rule active until explicitly disabled.
    UntilDisabled,
    /// Keep the rule active for this virtual duration.
    ForDurationNs {
        /// Active duration in virtual nanoseconds.
        duration_ns: u64,
    },
    /// Keep the rule active for this number of matching calls.
    ForCallCount {
        /// Active call budget before expiration.
        call_count: u64,
    },
}

/// Hook for runtime effect rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeHook {
    /// Trigger before invoking one binding implementation.
    BindingBefore,
    /// Trigger after invoking one binding implementation.
    BindingAfter,
    /// Trigger when one task is enqueued.
    SchedulerEnqueue,
    /// Trigger when one task is dequeued.
    SchedulerDequeue,
    /// Trigger when one timer fires.
    SchedulerTimerFire,
    /// Trigger when one external event wakes the scheduler.
    SchedulerEventWake,
    /// Trigger when time is read.
    TimeRead,
    /// Trigger when random data is read.
    RandomRead,
    /// Trigger when one resource is attached.
    ResourceAttach,
    /// Trigger when one resource is detached.
    ResourceDetach,
}

/// Control effect payload for runtime rules.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "control", rename_all = "camelCase")]
pub enum RuntimeControlEffect {
    /// Trigger one immediate GC cycle.
    GcCycleCollect {},
    /// Delay one GC cycle.
    GcCycleDelay {
        /// Delay duration in nanoseconds.
        delay_ns: Option<u64>,
    },
    /// Apply one temporary heap limit.
    GcHeapLimit {
        /// Heap limit in bytes.
        limit_bytes: u64,
    },
}

/// Runtime rule action payload.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RuntimeAction {
    /// Set the matching binding world.
    SetWorld {
        /// The selected world for matching bindings.
        world: RuntimeWorld,
    },
    /// Set the matching binding access mode.
    SetAccess {
        /// The selected access mode for matching bindings.
        access: RuntimeAccess,
    },
    /// Set the matching binding replay payload policy.
    SetReplay {
        /// The selected replay payload mode for matching bindings.
        payload: ReplayPayloadMode,
    },
    /// Apply one runtime fault effect.
    Fault {
        /// Fault payload for this rule.
        fault: RuntimeFaultEffect,
    },
    /// Apply one runtime control effect.
    Control {
        /// Control payload for this rule.
        control: RuntimeControlEffect,
    },
}

/// Runtime trigger controls.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RuntimeTrigger {
    /// Hook for this trigger.
    pub on: RuntimeHook,
    /// Trigger probability in parts-per-million.
    pub probability_ppm: Option<i64>,
    /// Maximum number of effect firings.
    pub max_occurrences: Option<u64>,
    /// Cooldown duration between firings in nanoseconds.
    pub cooldown_ns: Option<u64>,
    /// Number of firings per trigger hit.
    pub burst: Option<u32>,
    /// Activation window for this trigger.
    pub activation: Option<RuntimeRuleActivation>,
    /// Lifetime window for this trigger.
    pub lifetime: Option<RuntimeRuleLifetime>,
}

/// Fault injection payload for one runtime effect.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "name", rename_all = "camelCase")]
pub enum RuntimeFaultEffect {
    /// Inject deterministic runtime delay.
    #[serde(rename = "runtime.delay")]
    RuntimeDelay {
        /// Base delay in nanoseconds.
        base_ns: u64,
        /// Jitter delay in nanoseconds.
        jitter_ns: Option<u64>,
        /// Delay distribution mode.
        distribution: Option<RuntimeJitterDistribution>,
    },
    /// Inject one binding error.
    #[serde(rename = "runtime.error")]
    RuntimeError {
        /// Error code name for this injected failure.
        code: String,
    },
    /// Inject deterministic timeout behavior.
    #[serde(rename = "runtime.timeout")]
    RuntimeTimeout {
        /// Timeout duration in nanoseconds.
        timeout_ns: u64,
        /// Optional timeout error code override.
        code: Option<String>,
    },
    /// Inject connection disconnect on transport edges.
    #[serde(rename = "net.connection.disconnect")]
    NetConnectionDisconnect {},
    /// Inject connection reset behavior.
    #[serde(rename = "net.connection.reset")]
    NetConnectionReset {},
    /// Inject connection timeout behavior.
    #[serde(rename = "net.connection.timeout")]
    NetConnectionTimeout {
        /// Timeout duration in nanoseconds.
        timeout_ns: u64,
    },
    /// Inject packet drop faults.
    #[serde(rename = "net.packet.drop")]
    NetPacketDrop {},
    /// Inject packet duplication faults.
    #[serde(rename = "net.packet.duplicate")]
    NetPacketDuplicate {
        /// Number of duplicates emitted when the fault triggers.
        copies: Option<u32>,
    },
    /// Inject packet reordering faults.
    #[serde(rename = "net.packet.reorder")]
    NetPacketReorder {
        /// Reordering window size.
        window: Option<u32>,
    },
    /// Inject packet corruption faults.
    #[serde(rename = "net.packet.corrupt")]
    NetPacketCorrupt {},
    /// Inject route partition faults.
    #[serde(rename = "net.route.partition")]
    NetRoutePartition {
        /// Direction filter for one-way or two-way partition.
        direction: Option<RuntimeNetRouteDirection>,
    },
    /// Inject route blackhole faults.
    #[serde(rename = "net.route.blackhole")]
    NetRouteBlackhole {
        /// Direction filter for one-way or two-way blackhole.
        direction: Option<RuntimeNetRouteDirection>,
    },
    /// Inject link bandwidth limiting faults.
    #[serde(rename = "net.bandwidth.limit")]
    NetBandwidthLimit {
        /// Maximum throughput in bytes per second.
        bytes_per_second: u64,
    },
    /// Inject DNS timeout faults.
    #[serde(rename = "net.dns.timeout")]
    NetDnsTimeout {},
    /// Inject DNS NXDOMAIN faults.
    #[serde(rename = "net.dns.nxdomain")]
    NetDnsNxdomain {},
    /// Inject DNS SERVFAIL faults.
    #[serde(rename = "net.dns.servfail")]
    NetDnsServfail {},
    /// Inject ENOSPC-like filesystem I/O failures.
    #[serde(rename = "fs.io.enospc")]
    FsIoEnospc {},
    /// Inject low-level filesystem I/O faults.
    #[serde(rename = "fs.io.eio")]
    FsIoEio {},
    /// Inject per-process file descriptor exhaustion.
    #[serde(rename = "fs.io.emfile")]
    FsIoEmfile {},
    /// Inject global file descriptor exhaustion.
    #[serde(rename = "fs.io.enfile")]
    FsIoEnfile {},
    /// Inject filesystem quota exceeded failures.
    #[serde(rename = "fs.io.quota")]
    FsIoQuota {},
    /// Inject short write behavior.
    #[serde(rename = "fs.write.short")]
    FsWriteShort {
        /// Optional maximum bytes written before returning.
        max_bytes: Option<u64>,
    },
    /// Inject successful sync acknowledgements without persistence.
    #[serde(rename = "fs.sync.lie")]
    FsSyncLie {},
    /// Inject process crash lifecycle events.
    #[serde(rename = "process.lifecycle.crash")]
    ProcessLifecycleCrash {
        /// Optional crash signal number.
        signal: Option<i64>,
    },
    /// Inject process signal delivery events.
    #[serde(rename = "process.signal.deliver")]
    ProcessSignalDeliver {
        /// Signal number.
        signal: i64,
    },
    /// Inject process wait timeouts.
    #[serde(rename = "process.wait.timeout")]
    ProcessWaitTimeout {
        /// Timeout duration in nanoseconds.
        timeout_ns: u64,
    },
    /// Inject scheduler timer skew.
    #[serde(rename = "scheduler.timer.skew")]
    SchedulerTimerSkew {
        /// Signed skew in nanoseconds.
        skew_ns: i64,
    },
    /// Inject scheduler queue starvation.
    #[serde(rename = "scheduler.queue.starve")]
    SchedulerQueueStarve {
        /// Queue target selector.
        target: Option<String>,
        /// Starvation duration in nanoseconds.
        duration_ns: Option<u64>,
    },
    /// Inject wall or monotonic clock jumps.
    #[serde(rename = "time.clock.jump")]
    TimeClockJump {
        /// Signed jump in nanoseconds.
        delta_ns: i64,
    },
    /// Inject persistent wall or monotonic drift.
    #[serde(rename = "time.clock.drift")]
    TimeClockDrift {
        /// Signed rate offset in parts-per-million.
        rate_ppm: i64,
    },
}

/// Direction selector for route-level network faults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeNetRouteDirection {
    /// Affect ingress traffic.
    Ingress,
    /// Affect egress traffic.
    Egress,
    /// Affect both directions.
    Both,
}

impl RuntimeFaultEffect {
    /// Return the canonical fault name.
    pub const fn name(&self) -> &'static str {
        match self {
            RuntimeFaultEffect::RuntimeDelay { .. } => "runtime.delay",
            RuntimeFaultEffect::RuntimeError { .. } => "runtime.error",
            RuntimeFaultEffect::RuntimeTimeout { .. } => "runtime.timeout",
            RuntimeFaultEffect::NetConnectionDisconnect { .. } => "net.connection.disconnect",
            RuntimeFaultEffect::NetConnectionReset { .. } => "net.connection.reset",
            RuntimeFaultEffect::NetConnectionTimeout { .. } => "net.connection.timeout",
            RuntimeFaultEffect::NetPacketDrop { .. } => "net.packet.drop",
            RuntimeFaultEffect::NetPacketDuplicate { .. } => "net.packet.duplicate",
            RuntimeFaultEffect::NetPacketReorder { .. } => "net.packet.reorder",
            RuntimeFaultEffect::NetPacketCorrupt { .. } => "net.packet.corrupt",
            RuntimeFaultEffect::NetRoutePartition { .. } => "net.route.partition",
            RuntimeFaultEffect::NetRouteBlackhole { .. } => "net.route.blackhole",
            RuntimeFaultEffect::NetBandwidthLimit { .. } => "net.bandwidth.limit",
            RuntimeFaultEffect::NetDnsTimeout { .. } => "net.dns.timeout",
            RuntimeFaultEffect::NetDnsNxdomain { .. } => "net.dns.nxdomain",
            RuntimeFaultEffect::NetDnsServfail { .. } => "net.dns.servfail",
            RuntimeFaultEffect::FsIoEnospc { .. } => "fs.io.enospc",
            RuntimeFaultEffect::FsIoEio { .. } => "fs.io.eio",
            RuntimeFaultEffect::FsIoEmfile { .. } => "fs.io.emfile",
            RuntimeFaultEffect::FsIoEnfile { .. } => "fs.io.enfile",
            RuntimeFaultEffect::FsIoQuota { .. } => "fs.io.quota",
            RuntimeFaultEffect::FsWriteShort { .. } => "fs.write.short",
            RuntimeFaultEffect::FsSyncLie { .. } => "fs.sync.lie",
            RuntimeFaultEffect::ProcessLifecycleCrash { .. } => "process.lifecycle.crash",
            RuntimeFaultEffect::ProcessSignalDeliver { .. } => "process.signal.deliver",
            RuntimeFaultEffect::ProcessWaitTimeout { .. } => "process.wait.timeout",
            RuntimeFaultEffect::SchedulerTimerSkew { .. } => "scheduler.timer.skew",
            RuntimeFaultEffect::SchedulerQueueStarve { .. } => "scheduler.queue.starve",
            RuntimeFaultEffect::TimeClockJump { .. } => "time.clock.jump",
            RuntimeFaultEffect::TimeClockDrift { .. } => "time.clock.drift",
        }
    }
}

/// One runtime rule for world, access, or fault effects.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RuntimeRule {
    /// Optional stable rule identifier.
    pub id: Option<String>,
    /// Rule filter clause.
    pub when: RuntimeFilter,
    /// Action payload for this rule.
    pub action: RuntimeAction,
    /// Trigger controls for effect rules.
    /// Dispatch rules should leave this empty.
    pub trigger: Option<RuntimeTrigger>,
}

/// Runtime execution options for scheduler, time, randomness, and GC.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct RuntimeOptions {
    /// Execution mode for runtime scheduling and replay.
    pub execution: ExecutionMode,
    /// Default world for bindings without a matching route.
    pub world: RuntimeWorld,
    /// Default access policy for bindings without a matching access rule.
    pub access: RuntimeAccess,
    /// Ordered runtime rules for world, access, and fault policy.
    pub rules: Vec<RuntimeRule>,
    /// Replay log configuration.
    pub replay_log: ReplayLogOptions,
    /// Runtime clock configuration.
    pub time: TimeOptions,
    /// Runtime randomness configuration.
    pub random: RandomOptions,
    /// Runtime scheduler configuration.
    pub scheduler: SchedulerOptions,
    /// Runtime garbage collector configuration.
    pub gc: GcOptions,
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
    /// Memory runtime options.
    pub memory: PlatformMemoryOptions,
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

/// Platform-specific host runtime overrides.
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
pub struct PlatformDisplayOptions {}

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
pub struct PlatformIpcOptions {}

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
pub(super) fn runtime_options_from_json(
    json: Option<&DsConfigRuntimeOptionsJson>,
) -> RuntimeOptions {
    runtime_options_with_base(&RuntimeOptions::default(), json)
}

/// Derive runtime options from a base set of options plus overrides.
pub(super) fn runtime_options_with_base(
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
    /// Execution mode for runtime scheduling and replay.
    pub execution: Option<ExecutionModeJson>,
    /// Default world for bindings without matching world rules.
    pub world: Option<RuntimeWorldJson>,
    /// Default access policy for bindings without matching access rules.
    pub access: Option<RuntimeAccessJson>,
    /// Ordered world, access, and fault rules.
    pub rules: Option<Vec<RuntimeRuleJson>>,
    /// Replay log configuration.
    pub replay_log: Option<ReplayLogOptionsJson>,
    /// Runtime clock configuration.
    pub time: Option<TimeOptionsJson>,
    /// Runtime randomness configuration.
    pub random: Option<RandomOptionsJson>,
    /// Runtime scheduler configuration.
    pub scheduler: Option<SchedulerOptionsJson>,
    /// Runtime garbage collector configuration.
    pub gc: Option<GcOptionsJson>,
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
    /// Global memory runtime defaults.
    pub memory: Option<PlatformMemoryOptionsJson>,
    /// Global resource runtime defaults.
    pub resource: Option<PlatformResourceOptionsJson>,
    /// Global thread runtime defaults.
    pub thread: Option<PlatformThreadOptionsJson>,
    /// Global tty runtime defaults.
    pub tty: Option<PlatformTtyOptionsJson>,
    /// Platform-specific host runtime overrides.
    pub platform: Option<PlatformOptionsJson>,
}

impl DsConfigRuntimeOptionsJson {
    /// Apply runtime option overrides to a base set of options.
    pub fn apply_to(&self, options: &mut RuntimeOptions) {
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

        // apply runtime rule overrides
        if let Some(rules) = &self.rules {
            options.rules = rules.iter().map(RuntimeRule::from).collect();
        }

        // apply replay log overrides
        if let Some(replay_log) = &self.replay_log {
            replay_log.apply_to(&mut options.replay_log);
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

        // apply gc overrides
        if let Some(gc) = &self.gc {
            gc.apply_to(&mut options.gc);
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

        // apply memory defaults
        if let Some(memory) = &self.memory {
            memory.apply_to(&mut options.memory);
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

/// Runtime world selection for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum RuntimeWorldJson {
    /// Use host-backed platform bindings.
    Host,
    /// Use simulation-backed platform bindings.
    Simulation,
}

impl From<RuntimeWorldJson> for RuntimeWorld {
    fn from(value: RuntimeWorldJson) -> Self {
        match value {
            RuntimeWorldJson::Host => RuntimeWorld::Host,
            RuntimeWorldJson::Simulation => RuntimeWorld::Simulation,
        }
    }
}

/// Runtime default access policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum RuntimeAccessJson {
    /// Allow matching binding calls.
    Allow,
    /// Deny matching binding calls.
    Deny,
}

impl From<RuntimeAccessJson> for RuntimeAccess {
    fn from(value: RuntimeAccessJson) -> Self {
        match value {
            RuntimeAccessJson::Allow => RuntimeAccess::Allow,
            RuntimeAccessJson::Deny => RuntimeAccess::Deny,
        }
    }
}

/// Runtime rule filter clause for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct RuntimeFilterJson {
    /// Glob selector for full binding names.
    pub binding: Option<String>,
    /// Glob selector for capability names.
    pub capability: Option<String>,
    /// Glob selector for component names.
    pub component: Option<String>,
    /// Glob selector for module names.
    pub module: Option<String>,
    /// Engine selector.
    pub engine: Option<BindingEngineJson>,
    /// Execution mode selector.
    pub execution: Option<Vec<ExecutionModeJson>>,
    /// Platform selector.
    pub platforms: Option<Vec<String>>,
    /// Binding scope selector.
    pub scope: Option<BindingScopeJson>,
    /// Binding blocking selector.
    pub blocking: Option<BindingBlockingJson>,
    /// Binding effect selector.
    pub effect: Option<BindingEffectJson>,
}

impl From<&RuntimeFilterJson> for RuntimeFilter {
    fn from(value: &RuntimeFilterJson) -> Self {
        Self {
            binding: value.binding.clone(),
            capability: value.capability.clone(),
            component: value.component.clone(),
            module: value.module.clone(),
            engine: value.engine.map(BindingEngine::from),
            execution_modes: value
                .execution
                .as_ref()
                .map(|modes| modes.iter().copied().map(ExecutionMode::from).collect()),
            platforms: value.platforms.clone(),
            scope: value.scope.map(BindingScope::from),
            blocking: value.blocking.map(BindingBlocking::from),
            effect: value.effect.map(BindingEffect::from),
        }
    }
}

/// Runtime rule engine selector for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum BindingEngineJson {
    /// Match VM engine execution.
    Vm,
    /// Match native engine execution.
    Native,
}

impl From<BindingEngineJson> for BindingEngine {
    fn from(value: BindingEngineJson) -> Self {
        match value {
            BindingEngineJson::Vm => BindingEngine::Vm,
            BindingEngineJson::Native => BindingEngine::Native,
        }
    }
}

/// Runtime rule scope selector for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum BindingScopeJson {
    /// Match host scope bindings.
    Host,
    /// Match runtime scope bindings.
    Runtime,
}

impl From<BindingScopeJson> for BindingScope {
    fn from(value: BindingScopeJson) -> Self {
        match value {
            BindingScopeJson::Host => BindingScope::Host,
            BindingScopeJson::Runtime => BindingScope::Runtime,
        }
    }
}

/// Runtime rule blocking selector for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum BindingBlockingJson {
    /// Match always-blocking bindings.
    Always,
    /// Match never-blocking bindings.
    Never,
    /// Match conditionally blocking bindings.
    Sometimes,
}

impl From<BindingBlockingJson> for BindingBlocking {
    fn from(value: BindingBlockingJson) -> Self {
        match value {
            BindingBlockingJson::Always => BindingBlocking::Always,
            BindingBlockingJson::Never => BindingBlocking::Never,
            BindingBlockingJson::Sometimes => BindingBlocking::Sometimes,
        }
    }
}

/// Runtime rule binding effect selector for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum BindingEffectJson {
    /// Match pure bindings.
    Pure,
    /// Match deterministic bindings.
    Deterministic,
    /// Match recordable external bindings.
    ExternalRecordable,
    /// Match non-recordable external bindings.
    ExternalNonRecordable,
}

impl From<BindingEffectJson> for BindingEffect {
    fn from(value: BindingEffectJson) -> Self {
        match value {
            BindingEffectJson::Pure => BindingEffect::Pure,
            BindingEffectJson::Deterministic => BindingEffect::Deterministic,
            BindingEffectJson::ExternalRecordable => BindingEffect::ExternalRecordable,
            BindingEffectJson::ExternalNonRecordable => BindingEffect::ExternalNonRecordable,
        }
    }
}

/// Runtime rule activation behavior for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum RuntimeRuleActivationJson {
    /// Activate immediately.
    Immediate,
    /// Activate once virtual time reaches this timestamp.
    AtVirtualNs {
        /// Activation timestamp in virtual nanoseconds.
        virtual_ns: u64,
    },
    /// Activate once this matching call count has elapsed.
    AfterCallCount {
        /// Matching call count before activation.
        call_count: u64,
    },
}

impl From<RuntimeRuleActivationJson> for RuntimeRuleActivation {
    fn from(value: RuntimeRuleActivationJson) -> Self {
        match value {
            RuntimeRuleActivationJson::Immediate => RuntimeRuleActivation::Immediate,
            RuntimeRuleActivationJson::AtVirtualNs { virtual_ns } => {
                RuntimeRuleActivation::AtVirtualNs { virtual_ns }
            }
            RuntimeRuleActivationJson::AfterCallCount { call_count } => {
                RuntimeRuleActivation::AfterCallCount { call_count }
            }
        }
    }
}

/// Runtime rule lifetime behavior for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum RuntimeRuleLifetimeJson {
    /// Keep active until explicitly disabled.
    UntilDisabled,
    /// Keep active for this virtual duration.
    ForDurationNs {
        /// Active duration in virtual nanoseconds.
        duration_ns: u64,
    },
    /// Keep active for this matching call budget.
    ForCallCount {
        /// Active call count before expiration.
        call_count: u64,
    },
}

impl From<RuntimeRuleLifetimeJson> for RuntimeRuleLifetime {
    fn from(value: RuntimeRuleLifetimeJson) -> Self {
        match value {
            RuntimeRuleLifetimeJson::UntilDisabled => RuntimeRuleLifetime::UntilDisabled,
            RuntimeRuleLifetimeJson::ForDurationNs { duration_ns } => {
                RuntimeRuleLifetime::ForDurationNs { duration_ns }
            }
            RuntimeRuleLifetimeJson::ForCallCount { call_count } => {
                RuntimeRuleLifetime::ForCallCount { call_count }
            }
        }
    }
}

/// Jitter distribution for runtime delay faults in JSON.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum RuntimeJitterDistributionJson {
    /// Use a uniform random distribution.
    Uniform,
    /// Use a normal random distribution.
    Normal,
    /// Use an exponential random distribution.
    Exponential,
}

impl From<RuntimeJitterDistributionJson> for RuntimeJitterDistribution {
    fn from(value: RuntimeJitterDistributionJson) -> Self {
        match value {
            RuntimeJitterDistributionJson::Uniform => RuntimeJitterDistribution::Uniform,
            RuntimeJitterDistributionJson::Normal => RuntimeJitterDistribution::Normal,
            RuntimeJitterDistributionJson::Exponential => RuntimeJitterDistribution::Exponential,
        }
    }
}

/// Direction selector for route-level network faults in JSON.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum RuntimeNetRouteDirectionJson {
    /// Affect ingress traffic.
    Ingress,
    /// Affect egress traffic.
    Egress,
    /// Affect both directions.
    Both,
}

impl From<RuntimeNetRouteDirectionJson> for RuntimeNetRouteDirection {
    fn from(value: RuntimeNetRouteDirectionJson) -> Self {
        match value {
            RuntimeNetRouteDirectionJson::Ingress => RuntimeNetRouteDirection::Ingress,
            RuntimeNetRouteDirectionJson::Egress => RuntimeNetRouteDirection::Egress,
            RuntimeNetRouteDirectionJson::Both => RuntimeNetRouteDirection::Both,
        }
    }
}

/// Runtime hook for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum RuntimeHookJson {
    /// Trigger before invoking one binding implementation.
    BindingBefore,
    /// Trigger after invoking one binding implementation.
    BindingAfter,
    /// Trigger when one task is enqueued.
    SchedulerEnqueue,
    /// Trigger when one task is dequeued.
    SchedulerDequeue,
    /// Trigger when one timer fires.
    SchedulerTimerFire,
    /// Trigger when one external event wakes the scheduler.
    SchedulerEventWake,
    /// Trigger when time is read.
    TimeRead,
    /// Trigger when random data is read.
    RandomRead,
    /// Trigger when one resource is attached.
    ResourceAttach,
    /// Trigger when one resource is detached.
    ResourceDetach,
}

impl From<RuntimeHookJson> for RuntimeHook {
    fn from(value: RuntimeHookJson) -> Self {
        match value {
            RuntimeHookJson::BindingBefore => RuntimeHook::BindingBefore,
            RuntimeHookJson::BindingAfter => RuntimeHook::BindingAfter,
            RuntimeHookJson::SchedulerEnqueue => RuntimeHook::SchedulerEnqueue,
            RuntimeHookJson::SchedulerDequeue => RuntimeHook::SchedulerDequeue,
            RuntimeHookJson::SchedulerTimerFire => RuntimeHook::SchedulerTimerFire,
            RuntimeHookJson::SchedulerEventWake => RuntimeHook::SchedulerEventWake,
            RuntimeHookJson::TimeRead => RuntimeHook::TimeRead,
            RuntimeHookJson::RandomRead => RuntimeHook::RandomRead,
            RuntimeHookJson::ResourceAttach => RuntimeHook::ResourceAttach,
            RuntimeHookJson::ResourceDetach => RuntimeHook::ResourceDetach,
        }
    }
}

/// Runtime control effect payload for JSON deserialization.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "control", rename_all = "camelCase")]
pub enum RuntimeControlEffectJson {
    /// Trigger one immediate GC cycle.
    GcCycleCollect {},
    /// Delay one GC cycle.
    GcCycleDelay {
        /// Delay duration in nanoseconds.
        delay_ns: Option<u64>,
    },
    /// Apply one temporary heap limit.
    GcHeapLimit {
        /// Heap limit in bytes.
        limit_bytes: u64,
    },
}

impl From<&RuntimeControlEffectJson> for RuntimeControlEffect {
    fn from(value: &RuntimeControlEffectJson) -> Self {
        match value {
            RuntimeControlEffectJson::GcCycleCollect {} => RuntimeControlEffect::GcCycleCollect {},
            RuntimeControlEffectJson::GcCycleDelay { delay_ns } => {
                RuntimeControlEffect::GcCycleDelay {
                    delay_ns: *delay_ns,
                }
            }
            RuntimeControlEffectJson::GcHeapLimit { limit_bytes } => {
                RuntimeControlEffect::GcHeapLimit {
                    limit_bytes: *limit_bytes,
                }
            }
        }
    }
}

/// Runtime rule action payload for JSON deserialization.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RuntimeActionJson {
    /// Set the matching binding world.
    SetWorld {
        /// The selected world for matching bindings.
        world: RuntimeWorldJson,
    },
    /// Set the matching binding access mode.
    SetAccess {
        /// The selected access mode for matching bindings.
        access: RuntimeAccessJson,
    },
    /// Set the matching binding replay payload policy.
    SetReplay {
        /// The selected replay payload mode for matching bindings.
        payload: ReplayPayloadModeJson,
    },
    /// Apply one runtime fault effect.
    Fault {
        /// Fault payload for this rule.
        fault: RuntimeFaultEffectJson,
    },
    /// Apply one runtime control effect.
    Control {
        /// Control payload for this rule.
        control: RuntimeControlEffectJson,
    },
}

impl From<&RuntimeActionJson> for RuntimeAction {
    fn from(value: &RuntimeActionJson) -> Self {
        match value {
            RuntimeActionJson::SetWorld { world } => RuntimeAction::SetWorld {
                world: RuntimeWorld::from(*world),
            },
            RuntimeActionJson::SetAccess { access } => RuntimeAction::SetAccess {
                access: RuntimeAccess::from(*access),
            },
            RuntimeActionJson::SetReplay { payload } => RuntimeAction::SetReplay {
                payload: ReplayPayloadMode::from(*payload),
            },
            RuntimeActionJson::Fault { fault } => RuntimeAction::Fault {
                fault: RuntimeFaultEffect::from(fault),
            },
            RuntimeActionJson::Control { control } => RuntimeAction::Control {
                control: RuntimeControlEffect::from(control),
            },
        }
    }
}

/// Runtime trigger controls for JSON deserialization.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct RuntimeTriggerJson {
    /// Hook for this trigger.
    pub on: RuntimeHookJson,
    /// Trigger probability in the range 0.0 to 1.0.
    pub probability: Option<f64>,
    /// Maximum number of effect firings.
    pub max_occurrences: Option<u64>,
    /// Cooldown duration between firings in nanoseconds.
    pub cooldown_ns: Option<u64>,
    /// Number of firings per trigger hit.
    pub burst: Option<u32>,
    /// Activation window for this trigger.
    pub activation: Option<RuntimeRuleActivationJson>,
    /// Lifetime window for this trigger.
    pub lifetime: Option<RuntimeRuleLifetimeJson>,
}

impl From<&RuntimeTriggerJson> for RuntimeTrigger {
    fn from(value: &RuntimeTriggerJson) -> Self {
        Self {
            on: RuntimeHook::from(value.on),
            probability_ppm: probability_to_ppm(value.probability),
            max_occurrences: value.max_occurrences,
            cooldown_ns: value.cooldown_ns,
            burst: value.burst,
            activation: value.activation.map(RuntimeRuleActivation::from),
            lifetime: value.lifetime.map(RuntimeRuleLifetime::from),
        }
    }
}

/// Runtime fault payload for JSON deserialization.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "name", rename_all = "camelCase")]
pub enum RuntimeFaultEffectJson {
    /// Inject deterministic runtime delay.
    #[serde(rename = "runtime.delay")]
    RuntimeDelay {
        /// Base delay in nanoseconds.
        base_ns: u64,
        /// Jitter delay in nanoseconds.
        jitter_ns: Option<u64>,
        /// Delay distribution mode.
        distribution: Option<RuntimeJitterDistributionJson>,
    },
    /// Inject one binding error.
    #[serde(rename = "runtime.error")]
    RuntimeError {
        /// Error code name for this injected failure.
        code: String,
    },
    /// Inject deterministic timeout behavior.
    #[serde(rename = "runtime.timeout")]
    RuntimeTimeout {
        /// Timeout duration in nanoseconds.
        timeout_ns: u64,
        /// Optional timeout error code override.
        code: Option<String>,
    },
    /// Inject connection disconnect on transport edges.
    #[serde(rename = "net.connection.disconnect")]
    NetConnectionDisconnect {},
    /// Inject connection reset behavior.
    #[serde(rename = "net.connection.reset")]
    NetConnectionReset {},
    /// Inject connection timeout behavior.
    #[serde(rename = "net.connection.timeout")]
    NetConnectionTimeout {
        /// Timeout duration in nanoseconds.
        timeout_ns: u64,
    },
    /// Inject packet drop faults.
    #[serde(rename = "net.packet.drop")]
    NetPacketDrop {},
    /// Inject packet duplication faults.
    #[serde(rename = "net.packet.duplicate")]
    NetPacketDuplicate {
        /// Number of duplicates emitted when the fault triggers.
        copies: Option<u32>,
    },
    /// Inject packet reordering faults.
    #[serde(rename = "net.packet.reorder")]
    NetPacketReorder {
        /// Reordering window size.
        window: Option<u32>,
    },
    /// Inject packet corruption faults.
    #[serde(rename = "net.packet.corrupt")]
    NetPacketCorrupt {},
    /// Inject route partition faults.
    #[serde(rename = "net.route.partition")]
    NetRoutePartition {
        /// Direction filter for one-way or two-way partition.
        direction: Option<RuntimeNetRouteDirectionJson>,
    },
    /// Inject route blackhole faults.
    #[serde(rename = "net.route.blackhole")]
    NetRouteBlackhole {
        /// Direction filter for one-way or two-way blackhole.
        direction: Option<RuntimeNetRouteDirectionJson>,
    },
    /// Inject link bandwidth limiting faults.
    #[serde(rename = "net.bandwidth.limit")]
    NetBandwidthLimit {
        /// Maximum throughput in bytes per second.
        bytes_per_second: u64,
    },
    /// Inject DNS timeout faults.
    #[serde(rename = "net.dns.timeout")]
    NetDnsTimeout {},
    /// Inject DNS NXDOMAIN faults.
    #[serde(rename = "net.dns.nxdomain")]
    NetDnsNxdomain {},
    /// Inject DNS SERVFAIL faults.
    #[serde(rename = "net.dns.servfail")]
    NetDnsServfail {},
    /// Inject ENOSPC-like filesystem I/O failures.
    #[serde(rename = "fs.io.enospc")]
    FsIoEnospc {},
    /// Inject low-level filesystem I/O faults.
    #[serde(rename = "fs.io.eio")]
    FsIoEio {},
    /// Inject per-process file descriptor exhaustion.
    #[serde(rename = "fs.io.emfile")]
    FsIoEmfile {},
    /// Inject global file descriptor exhaustion.
    #[serde(rename = "fs.io.enfile")]
    FsIoEnfile {},
    /// Inject filesystem quota exceeded failures.
    #[serde(rename = "fs.io.quota")]
    FsIoQuota {},
    /// Inject short write behavior.
    #[serde(rename = "fs.write.short")]
    FsWriteShort {
        /// Optional maximum bytes written before returning.
        max_bytes: Option<u64>,
    },
    /// Inject successful sync acknowledgements without persistence.
    #[serde(rename = "fs.sync.lie")]
    FsSyncLie {},
    /// Inject process crash lifecycle events.
    #[serde(rename = "process.lifecycle.crash")]
    ProcessLifecycleCrash {
        /// Optional crash signal number.
        signal: Option<i64>,
    },
    /// Inject process signal delivery events.
    #[serde(rename = "process.signal.deliver")]
    ProcessSignalDeliver {
        /// Signal number.
        signal: i64,
    },
    /// Inject process wait timeouts.
    #[serde(rename = "process.wait.timeout")]
    ProcessWaitTimeout {
        /// Timeout duration in nanoseconds.
        timeout_ns: u64,
    },
    /// Inject scheduler timer skew.
    #[serde(rename = "scheduler.timer.skew")]
    SchedulerTimerSkew {
        /// Signed skew in nanoseconds.
        skew_ns: i64,
    },
    /// Inject scheduler queue starvation.
    #[serde(rename = "scheduler.queue.starve")]
    SchedulerQueueStarve {
        /// Queue target selector.
        target: Option<String>,
        /// Starvation duration in nanoseconds.
        duration_ns: Option<u64>,
    },
    /// Inject wall or monotonic clock jumps.
    #[serde(rename = "time.clock.jump")]
    TimeClockJump {
        /// Signed jump in nanoseconds.
        delta_ns: i64,
    },
    /// Inject persistent wall or monotonic drift.
    #[serde(rename = "time.clock.drift")]
    TimeClockDrift {
        /// Signed rate offset in parts-per-million.
        rate_ppm: i64,
    },
}

fn probability_to_ppm(probability: Option<f64>) -> Option<i64> {
    let probability = probability?;

    // propagate non-finite values to validator as an invalid sentinel
    if !probability.is_finite() {
        return Some(i64::MIN);
    }

    let scaled_probability = probability * 1_000_000.0;
    Some(scaled_probability.round() as i64)
}

impl From<&RuntimeFaultEffectJson> for RuntimeFaultEffect {
    fn from(value: &RuntimeFaultEffectJson) -> Self {
        match value {
            RuntimeFaultEffectJson::RuntimeDelay {
                base_ns,
                jitter_ns,
                distribution,
            } => RuntimeFaultEffect::RuntimeDelay {
                base_ns: *base_ns,
                jitter_ns: *jitter_ns,
                distribution: distribution.map(RuntimeJitterDistribution::from),
            },
            RuntimeFaultEffectJson::RuntimeError { code } => {
                RuntimeFaultEffect::RuntimeError { code: code.clone() }
            }
            RuntimeFaultEffectJson::RuntimeTimeout { timeout_ns, code } => {
                RuntimeFaultEffect::RuntimeTimeout {
                    timeout_ns: *timeout_ns,
                    code: code.clone(),
                }
            }
            RuntimeFaultEffectJson::NetConnectionDisconnect {} => {
                RuntimeFaultEffect::NetConnectionDisconnect {}
            }
            RuntimeFaultEffectJson::NetConnectionReset {} => {
                RuntimeFaultEffect::NetConnectionReset {}
            }
            RuntimeFaultEffectJson::NetConnectionTimeout { timeout_ns } => {
                RuntimeFaultEffect::NetConnectionTimeout {
                    timeout_ns: *timeout_ns,
                }
            }
            RuntimeFaultEffectJson::NetPacketDrop {} => RuntimeFaultEffect::NetPacketDrop {},
            RuntimeFaultEffectJson::NetPacketDuplicate { copies } => {
                RuntimeFaultEffect::NetPacketDuplicate { copies: *copies }
            }
            RuntimeFaultEffectJson::NetPacketReorder { window } => {
                RuntimeFaultEffect::NetPacketReorder { window: *window }
            }
            RuntimeFaultEffectJson::NetPacketCorrupt {} => RuntimeFaultEffect::NetPacketCorrupt {},
            RuntimeFaultEffectJson::NetRoutePartition { direction } => {
                RuntimeFaultEffect::NetRoutePartition {
                    direction: direction.map(RuntimeNetRouteDirection::from),
                }
            }
            RuntimeFaultEffectJson::NetRouteBlackhole { direction } => {
                RuntimeFaultEffect::NetRouteBlackhole {
                    direction: direction.map(RuntimeNetRouteDirection::from),
                }
            }
            RuntimeFaultEffectJson::NetBandwidthLimit { bytes_per_second } => {
                RuntimeFaultEffect::NetBandwidthLimit {
                    bytes_per_second: *bytes_per_second,
                }
            }
            RuntimeFaultEffectJson::NetDnsTimeout {} => RuntimeFaultEffect::NetDnsTimeout {},
            RuntimeFaultEffectJson::NetDnsNxdomain {} => RuntimeFaultEffect::NetDnsNxdomain {},
            RuntimeFaultEffectJson::NetDnsServfail {} => RuntimeFaultEffect::NetDnsServfail {},
            RuntimeFaultEffectJson::FsIoEnospc {} => RuntimeFaultEffect::FsIoEnospc {},
            RuntimeFaultEffectJson::FsIoEio {} => RuntimeFaultEffect::FsIoEio {},
            RuntimeFaultEffectJson::FsIoEmfile {} => RuntimeFaultEffect::FsIoEmfile {},
            RuntimeFaultEffectJson::FsIoEnfile {} => RuntimeFaultEffect::FsIoEnfile {},
            RuntimeFaultEffectJson::FsIoQuota {} => RuntimeFaultEffect::FsIoQuota {},
            RuntimeFaultEffectJson::FsWriteShort { max_bytes } => {
                RuntimeFaultEffect::FsWriteShort {
                    max_bytes: *max_bytes,
                }
            }
            RuntimeFaultEffectJson::FsSyncLie {} => RuntimeFaultEffect::FsSyncLie {},
            RuntimeFaultEffectJson::ProcessLifecycleCrash { signal } => {
                RuntimeFaultEffect::ProcessLifecycleCrash { signal: *signal }
            }
            RuntimeFaultEffectJson::ProcessSignalDeliver { signal } => {
                RuntimeFaultEffect::ProcessSignalDeliver { signal: *signal }
            }
            RuntimeFaultEffectJson::ProcessWaitTimeout { timeout_ns } => {
                RuntimeFaultEffect::ProcessWaitTimeout {
                    timeout_ns: *timeout_ns,
                }
            }
            RuntimeFaultEffectJson::SchedulerTimerSkew { skew_ns } => {
                RuntimeFaultEffect::SchedulerTimerSkew { skew_ns: *skew_ns }
            }
            RuntimeFaultEffectJson::SchedulerQueueStarve {
                target,
                duration_ns,
            } => RuntimeFaultEffect::SchedulerQueueStarve {
                target: target.clone(),
                duration_ns: *duration_ns,
            },
            RuntimeFaultEffectJson::TimeClockJump { delta_ns } => {
                RuntimeFaultEffect::TimeClockJump {
                    delta_ns: *delta_ns,
                }
            }
            RuntimeFaultEffectJson::TimeClockDrift { rate_ppm } => {
                RuntimeFaultEffect::TimeClockDrift {
                    rate_ppm: *rate_ppm,
                }
            }
        }
    }
}

/// Runtime rule for JSON deserialization.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct RuntimeRuleJson {
    /// Optional stable rule identifier.
    pub id: Option<String>,
    /// Rule filter clause.
    pub when: RuntimeFilterJson,
    /// Rule action payload.
    pub action: RuntimeActionJson,
    /// Trigger controls for effect rules.
    pub trigger: Option<RuntimeTriggerJson>,
}

impl From<&RuntimeRuleJson> for RuntimeRule {
    fn from(value: &RuntimeRuleJson) -> Self {
        Self {
            id: value.id.clone(),
            when: RuntimeFilter::from(&value.when),
            action: RuntimeAction::from(&value.action),
            trigger: value.trigger.as_ref().map(RuntimeTrigger::from),
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
pub struct PlatformDisplayOptionsJson {}

impl PlatformDisplayOptionsJson {
    /// Apply display overrides to a base set of options.
    pub fn apply_to(&self, _options: &mut PlatformDisplayOptions) {}
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
pub struct PlatformIpcOptionsJson {}

impl PlatformIpcOptionsJson {
    /// Apply ipc overrides to a base set of options.
    pub fn apply_to(&self, _options: &mut PlatformIpcOptions) {}
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

/// Replay log options for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ReplayLogOptionsJson {
    /// Base path for replay logs (file or directory).
    pub path: Option<String>,
    /// Template for auto-generated log file names.
    pub template: Option<String>,
    /// Chunk size in megabytes for log rotation.
    pub chunk_size_mb: Option<u64>,
    /// Replay payload selection for record mode.
    pub payload: Option<ReplayPayloadModeJson>,
}

impl ReplayLogOptionsJson {
    /// Apply replay log overrides to a base set of options.
    pub fn apply_to(&self, options: &mut ReplayLogOptions) {
        // apply path overrides
        if let Some(path) = &self.path {
            options.path = Some(PathBuf::from(path));
        }

        // apply template overrides
        if let Some(template) = &self.template {
            options.template = Some(template.clone());
        }

        // apply chunk sizing overrides
        if let Some(chunk_size_mb) = self.chunk_size_mb {
            options.chunk_size_mb = Some(chunk_size_mb);
        }

        // apply replay payload overrides
        if let Some(payload) = self.payload {
            options.payload = payload.into();
        }
    }
}

/// Runtime time options for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct TimeOptionsJson {
    /// Clock mode selection.
    pub mode: Option<TimeModeJson>,
    /// Epoch in nanoseconds for virtual time.
    pub epoch_ns: Option<u64>,
    /// Tick size in nanoseconds for virtual time.
    pub tick_ns: Option<u64>,
    /// Time zone identifier or fixed offset string.
    pub time_zone: Option<String>,
}

impl TimeOptionsJson {
    /// Apply time overrides to a base set of options.
    pub fn apply_to(&self, options: &mut TimeOptions) {
        // apply mode overrides
        if let Some(mode) = self.mode {
            options.mode = TimeMode::from(mode);
        }

        // apply epoch overrides
        if let Some(epoch_ns) = self.epoch_ns {
            options.epoch_ns = Some(epoch_ns);
        }

        // apply tick overrides
        if let Some(tick_ns) = self.tick_ns {
            options.tick_ns = Some(tick_ns);
        }

        // apply timezone overrides
        if let Some(time_zone) = &self.time_zone {
            options.time_zone = Some(time_zone.clone());
        }
    }
}

/// Runtime randomness options for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct RandomOptionsJson {
    /// Randomness source selection.
    pub mode: Option<RandomModeJson>,
    /// Seed for deterministic randomness streams.
    pub seed: Option<u64>,
    /// Whether to use a per-task random stream.
    pub per_task: Option<bool>,
}

impl RandomOptionsJson {
    /// Apply random overrides to a base set of options.
    pub fn apply_to(&self, options: &mut RandomOptions) {
        // apply mode overrides
        if let Some(mode) = self.mode {
            options.mode = RandomMode::from(mode);
        }

        // apply seed overrides
        if let Some(seed) = self.seed {
            options.seed = Some(seed);
        }

        // apply per-task overrides
        if let Some(per_task) = self.per_task {
            options.per_task = per_task;
        }
    }
}

/// Runtime scheduler options for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct SchedulerOptionsJson {
    /// Event loop tick budget in nanoseconds.
    pub tick_budget_ns: Option<u64>,
    /// Maximum number of microtasks per tick.
    pub microtask_budget: Option<u64>,
    /// Maximum host semantic events dispatched in sequence before one poller event.
    pub host_event_budget: Option<u64>,
    /// Maximum microtask nesting depth.
    pub max_microtask_depth: Option<u64>,
    /// Timer resolution in nanoseconds.
    pub timer_resolution_ns: Option<u64>,
    /// Maximum timer coalescing window in nanoseconds.
    pub max_timer_coalesce_ns: Option<u64>,
    /// Preemption interval for long-running tasks in nanoseconds.
    pub preempt_interval_ns: Option<u64>,
    /// Platform poller backend selection.
    pub poller_backend: Option<PollerBackendJson>,
    /// Scheduling policy for task pools.
    pub policy: Option<SchedulerPolicyJson>,
    /// Number of worker threads for parallel tasks.
    pub worker_threads: Option<u64>,
    /// Number of I/O threads.
    pub io_threads: Option<u64>,
    /// Number of blocking worker threads.
    pub blocking_threads: Option<u64>,
    /// Maximum number of concurrent tasks.
    pub max_tasks: Option<u64>,
}

impl SchedulerOptionsJson {
    /// Apply scheduler overrides to a base set of options.
    pub fn apply_to(&self, options: &mut SchedulerOptions) {
        // apply event loop overrides
        if let Some(tick_budget_ns) = self.tick_budget_ns {
            options.tick_budget_ns = Some(tick_budget_ns);
        }
        if let Some(microtask_budget) = self.microtask_budget {
            options.microtask_budget = Some(microtask_budget);
        }
        if let Some(host_event_budget) = self.host_event_budget {
            options.host_event_budget = Some(host_event_budget);
        }
        if let Some(max_microtask_depth) = self.max_microtask_depth {
            options.max_microtask_depth = Some(max_microtask_depth);
        }
        if let Some(timer_resolution_ns) = self.timer_resolution_ns {
            options.timer_resolution_ns = Some(timer_resolution_ns);
        }
        if let Some(max_timer_coalesce_ns) = self.max_timer_coalesce_ns {
            options.max_timer_coalesce_ns = Some(max_timer_coalesce_ns);
        }
        if let Some(preempt_interval_ns) = self.preempt_interval_ns {
            options.preempt_interval_ns = Some(preempt_interval_ns);
        }
        if let Some(poller_backend) = self.poller_backend {
            options.poller_backend = PollerBackend::from(poller_backend);
        }

        // apply task pool overrides
        if let Some(policy) = self.policy {
            options.policy = SchedulerPolicy::from(policy);
        }
        if let Some(worker_threads) = self.worker_threads {
            options.worker_threads = Some(worker_threads);
        }
        if let Some(io_threads) = self.io_threads {
            options.io_threads = Some(io_threads);
        }
        if let Some(blocking_threads) = self.blocking_threads {
            options.blocking_threads = Some(blocking_threads);
        }
        if let Some(max_tasks) = self.max_tasks {
            options.max_tasks = Some(max_tasks);
        }
    }
}

/// Poller backend options for JSON deserialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum PollerBackendJson {
    /// Choose the best available backend for the platform.
    Auto,
    /// Use io_uring (Linux only).
    IoUring,
    /// Use epoll (Linux only).
    Epoll,
    /// Use kqueue (BSD/macOS only).
    Kqueue,
    /// Use poll (portable Unix fallback).
    Poll,
    /// Use the Windows readiness backend.
    Windows,
}

impl From<PollerBackendJson> for PollerBackend {
    fn from(value: PollerBackendJson) -> Self {
        match value {
            PollerBackendJson::Auto => Self::Auto,
            PollerBackendJson::IoUring => Self::IoUring,
            PollerBackendJson::Epoll => Self::Epoll,
            PollerBackendJson::Kqueue => Self::Kqueue,
            PollerBackendJson::Poll => Self::Poll,
            PollerBackendJson::Windows => Self::Windows,
        }
    }
}

/// Runtime GC options for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct GcOptionsJson {
    /// Whether the garbage collector is enabled.
    pub enabled: Option<bool>,
    /// Heap growth target percentage.
    pub heap_growth_percent: Option<u32>,
    /// Soft heap limit in bytes.
    pub heap_soft_limit_bytes: Option<u64>,
    /// Initial heap size hint in bytes.
    pub heap_initial_bytes: Option<u64>,
    /// GC logging verbosity.
    pub logging: Option<GcLoggingJson>,
}

impl GcOptionsJson {
    /// Apply GC overrides to a base set of options.
    pub fn apply_to(&self, options: &mut GcOptions) {
        // apply enablement overrides
        if let Some(enabled) = self.enabled {
            options.enabled = enabled;
        }

        // apply pacing overrides
        if let Some(heap_growth_percent) = self.heap_growth_percent {
            options.heap_growth_percent = heap_growth_percent;
        }

        // apply memory limit overrides
        if let Some(heap_soft_limit_bytes) = self.heap_soft_limit_bytes {
            options.heap_soft_limit_bytes = Some(heap_soft_limit_bytes);
        }
        if let Some(heap_initial_bytes) = self.heap_initial_bytes {
            options.heap_initial_bytes = Some(heap_initial_bytes);
        }

        // apply logging overrides
        if let Some(logging) = self.logging {
            options.logging = GcLogging::from(logging);
        }
    }
}

/// Time mode for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum TimeModeJson {
    /// Use the host clock directly.
    Host,
    /// Use a virtualized clock derived from runtime state.
    Virtual,
}

impl From<TimeModeJson> for TimeMode {
    fn from(value: TimeModeJson) -> Self {
        match value {
            TimeModeJson::Host => TimeMode::Host,
            TimeModeJson::Virtual => TimeMode::Virtual,
        }
    }
}

/// Randomness mode for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum RandomModeJson {
    /// Use the host randomness source.
    Host,
    /// Use deterministic runtime-managed randomness.
    Deterministic,
}

impl From<RandomModeJson> for RandomMode {
    fn from(value: RandomModeJson) -> Self {
        match value {
            RandomModeJson::Host => RandomMode::Host,
            RandomModeJson::Deterministic => RandomMode::Deterministic,
        }
    }
}

/// Scheduler policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum SchedulerPolicyJson {
    /// First-in, first-out scheduling.
    Fifo,
    /// Fair scheduling with time slicing.
    Fair,
    /// Work-stealing scheduling for throughput.
    #[serde(
        rename = "work_stealing",
        alias = "work-stealing",
        alias = "workstealing"
    )]
    WorkStealing,
}

impl From<SchedulerPolicyJson> for SchedulerPolicy {
    fn from(value: SchedulerPolicyJson) -> Self {
        match value {
            SchedulerPolicyJson::Fifo => SchedulerPolicy::Fifo,
            SchedulerPolicyJson::Fair => SchedulerPolicy::Fair,
            SchedulerPolicyJson::WorkStealing => SchedulerPolicy::WorkStealing,
        }
    }
}

/// GC logging for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum GcLoggingJson {
    /// Disable GC logging.
    Off,
    /// Emit summary GC events.
    Summary,
    /// Emit verbose GC events.
    Verbose,
}

impl From<GcLoggingJson> for GcLogging {
    fn from(value: GcLoggingJson) -> Self {
        match value {
            GcLoggingJson::Off => GcLogging::Off,
            GcLoggingJson::Summary => GcLogging::Summary,
            GcLoggingJson::Verbose => GcLogging::Verbose,
        }
    }
}
