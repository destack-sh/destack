use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::policy::{ExecutionModeJson, ReplayPayloadModeJson};
use crate::{
    ExecutionMode, GcLogging, GcOptions, PlatformOptions, PlatformWindowsOptions, PollerBackend,
    RandomMode, RandomOptions, ReplayLogOptions, RuntimeAccess, RuntimeFault,
    RuntimeFaultDistribution, RuntimeOptions, RuntimeRule, RuntimeRuleBlocking, RuntimeRuleEffect,
    RuntimeRuleEngine, RuntimeRuleFilter, RuntimeRuleScope, RuntimeWorld, SchedulerOptions,
    SchedulerPolicy, TimeMode, TimeOptions,
};

/// Derive runtime options from optional JSON overrides.
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
    /// Platform-specific runtime configuration.
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
    /// Use simulated platform bindings.
    #[serde(alias = "sim")]
    Simulated,
}

impl From<RuntimeWorldJson> for RuntimeWorld {
    fn from(value: RuntimeWorldJson) -> Self {
        match value {
            RuntimeWorldJson::Host => RuntimeWorld::Host,
            RuntimeWorldJson::Simulated => RuntimeWorld::Simulated,
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
pub struct RuntimeRuleFilterJson {
    /// Glob selector for full binding names.
    pub binding: Option<String>,
    /// Glob selector for capability names.
    pub capability: Option<String>,
    /// Glob selector for module names.
    pub module: Option<String>,
    /// Engine selector.
    pub engine: Option<RuntimeRuleEngineJson>,
    /// Execution mode selector.
    pub execution: Option<Vec<ExecutionModeJson>>,
    /// Platform selector.
    pub platforms: Option<Vec<String>>,
    /// Binding scope selector.
    pub scope: Option<RuntimeRuleScopeJson>,
    /// Binding blocking selector.
    pub blocking: Option<RuntimeRuleBlockingJson>,
    /// Binding effect selector.
    pub effect: Option<RuntimeRuleEffectJson>,
}

impl From<&RuntimeRuleFilterJson> for RuntimeRuleFilter {
    fn from(value: &RuntimeRuleFilterJson) -> Self {
        Self {
            binding: value.binding.clone(),
            capability: value.capability.clone(),
            module: value.module.clone(),
            engine: value.engine.map(RuntimeRuleEngine::from),
            execution_modes: value
                .execution
                .as_ref()
                .map(|modes| modes.iter().copied().map(ExecutionMode::from).collect()),
            platforms: value.platforms.clone(),
            scope: value.scope.map(RuntimeRuleScope::from),
            blocking: value.blocking.map(RuntimeRuleBlocking::from),
            effect: value.effect.map(RuntimeRuleEffect::from),
        }
    }
}

/// Runtime rule engine selector for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum RuntimeRuleEngineJson {
    /// Match VM engine execution.
    Vm,
    /// Match native engine execution.
    Native,
}

impl From<RuntimeRuleEngineJson> for RuntimeRuleEngine {
    fn from(value: RuntimeRuleEngineJson) -> Self {
        match value {
            RuntimeRuleEngineJson::Vm => RuntimeRuleEngine::Vm,
            RuntimeRuleEngineJson::Native => RuntimeRuleEngine::Native,
        }
    }
}

/// Runtime rule scope selector for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum RuntimeRuleScopeJson {
    /// Match OS scope bindings.
    Os,
    /// Match runtime scope bindings.
    Runtime,
    /// Match hybrid scope bindings.
    Hybrid,
}

impl From<RuntimeRuleScopeJson> for RuntimeRuleScope {
    fn from(value: RuntimeRuleScopeJson) -> Self {
        match value {
            RuntimeRuleScopeJson::Os => RuntimeRuleScope::Os,
            RuntimeRuleScopeJson::Runtime => RuntimeRuleScope::Runtime,
            RuntimeRuleScopeJson::Hybrid => RuntimeRuleScope::Hybrid,
        }
    }
}

/// Runtime rule blocking selector for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum RuntimeRuleBlockingJson {
    /// Match always-blocking bindings.
    Always,
    /// Match never-blocking bindings.
    Never,
    /// Match conditionally blocking bindings.
    Sometimes,
}

impl From<RuntimeRuleBlockingJson> for RuntimeRuleBlocking {
    fn from(value: RuntimeRuleBlockingJson) -> Self {
        match value {
            RuntimeRuleBlockingJson::Always => RuntimeRuleBlocking::Always,
            RuntimeRuleBlockingJson::Never => RuntimeRuleBlocking::Never,
            RuntimeRuleBlockingJson::Sometimes => RuntimeRuleBlocking::Sometimes,
        }
    }
}

/// Runtime rule effect selector for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum RuntimeRuleEffectJson {
    /// Match pure bindings.
    Pure,
    /// Match deterministic bindings.
    Deterministic,
    /// Match recordable external bindings.
    ExternalRecordable,
    /// Match non-recordable external bindings.
    ExternalNonRecordable,
}

impl From<RuntimeRuleEffectJson> for RuntimeRuleEffect {
    fn from(value: RuntimeRuleEffectJson) -> Self {
        match value {
            RuntimeRuleEffectJson::Pure => RuntimeRuleEffect::Pure,
            RuntimeRuleEffectJson::Deterministic => RuntimeRuleEffect::Deterministic,
            RuntimeRuleEffectJson::ExternalRecordable => RuntimeRuleEffect::ExternalRecordable,
            RuntimeRuleEffectJson::ExternalNonRecordable => {
                RuntimeRuleEffect::ExternalNonRecordable
            }
        }
    }
}

/// Jitter distribution for delay faults in JSON.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum RuntimeFaultDistributionJson {
    /// Use a uniform random distribution.
    Uniform,
    /// Use a normal random distribution.
    Normal,
    /// Use an exponential random distribution.
    Exponential,
}

impl From<RuntimeFaultDistributionJson> for RuntimeFaultDistribution {
    fn from(value: RuntimeFaultDistributionJson) -> Self {
        match value {
            RuntimeFaultDistributionJson::Uniform => RuntimeFaultDistribution::Uniform,
            RuntimeFaultDistributionJson::Normal => RuntimeFaultDistribution::Normal,
            RuntimeFaultDistributionJson::Exponential => RuntimeFaultDistribution::Exponential,
        }
    }
}

/// Runtime fault payload for JSON deserialization.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RuntimeFaultJson {
    /// Inject a binding error.
    Error {
        /// Error code name for error injection.
        code: String,
        /// Probability in parts-per-million.
        probability_ppm: Option<u32>,
    },
    /// Inject base delay and optional jitter before returning.
    Delay {
        /// Base delay in nanoseconds.
        base_ns: u64,
        /// Jitter range in nanoseconds.
        jitter_ns: Option<u64>,
        /// Jitter distribution selector.
        distribution: Option<RuntimeFaultDistributionJson>,
        /// Probability in parts-per-million.
        probability_ppm: Option<u32>,
    },
    /// Drop the call result.
    Drop {
        /// Probability in parts-per-million.
        probability_ppm: Option<u32>,
    },
    /// Duplicate delivery of a result.
    Duplicate {
        /// Number of result copies to emit.
        copies: u32,
        /// Probability in parts-per-million.
        probability_ppm: Option<u32>,
    },
    /// Reorder result delivery within a bounded window.
    Reorder {
        /// Reorder window size.
        window: u32,
        /// Probability in parts-per-million.
        probability_ppm: Option<u32>,
    },
    /// Force timeout behavior for the call.
    Timeout {
        /// Timeout duration in nanoseconds.
        timeout_ns: u64,
        /// Optional timeout error code override.
        code: Option<String>,
        /// Probability in parts-per-million.
        probability_ppm: Option<u32>,
    },
    /// Simulate a connection or handle disconnect.
    Disconnect {
        /// Probability in parts-per-million.
        probability_ppm: Option<u32>,
    },
}

impl From<&RuntimeFaultJson> for RuntimeFault {
    fn from(value: &RuntimeFaultJson) -> Self {
        match value {
            RuntimeFaultJson::Error {
                code,
                probability_ppm,
            } => RuntimeFault::Error {
                code: code.clone(),
                probability_ppm: *probability_ppm,
            },
            RuntimeFaultJson::Delay {
                base_ns,
                jitter_ns,
                distribution,
                probability_ppm,
            } => RuntimeFault::Delay {
                base_ns: *base_ns,
                jitter_ns: *jitter_ns,
                distribution: distribution
                    .as_ref()
                    .map(|mode| RuntimeFaultDistribution::from(*mode)),
                probability_ppm: *probability_ppm,
            },
            RuntimeFaultJson::Drop { probability_ppm } => RuntimeFault::Drop {
                probability_ppm: *probability_ppm,
            },
            RuntimeFaultJson::Duplicate {
                copies,
                probability_ppm,
            } => RuntimeFault::Duplicate {
                copies: *copies,
                probability_ppm: *probability_ppm,
            },
            RuntimeFaultJson::Reorder {
                window,
                probability_ppm,
            } => RuntimeFault::Reorder {
                window: *window,
                probability_ppm: *probability_ppm,
            },
            RuntimeFaultJson::Timeout {
                timeout_ns,
                code,
                probability_ppm,
            } => RuntimeFault::Timeout {
                timeout_ns: *timeout_ns,
                code: code.clone(),
                probability_ppm: *probability_ppm,
            },
            RuntimeFaultJson::Disconnect { probability_ppm } => RuntimeFault::Disconnect {
                probability_ppm: *probability_ppm,
            },
        }
    }
}

/// Runtime rule for JSON deserialization.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct RuntimeRuleJson {
    /// Optional stable rule identifier.
    pub id: Option<String>,
    /// Rule filter clause.
    pub when: RuntimeRuleFilterJson,
    /// World action for this rule.
    pub world: Option<RuntimeWorldJson>,
    /// Access action for this rule.
    pub access: Option<RuntimeAccessJson>,
    /// Fault action for this rule.
    pub fault: Option<RuntimeFaultJson>,
}

impl From<&RuntimeRuleJson> for RuntimeRule {
    fn from(value: &RuntimeRuleJson) -> Self {
        Self {
            id: value.id.clone(),
            when: RuntimeRuleFilter::from(&value.when),
            world: value.world.map(RuntimeWorld::from),
            access: value.access.map(RuntimeAccess::from),
            fault: value.fault.as_ref().map(RuntimeFault::from),
        }
    }
}

/// Platform runtime options for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlatformOptionsJson {
    /// Windows-specific runtime configuration.
    pub windows: Option<PlatformWindowsOptionsJson>,
}

impl PlatformOptionsJson {
    /// Apply platform overrides to a base set of options.
    pub fn apply_to(&self, options: &mut PlatformOptions) {
        // apply windows overrides
        if let Some(windows) = &self.windows {
            windows.apply_to(&mut options.windows);
        }
    }
}

/// Windows runtime options for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlatformWindowsOptionsJson {
    /// Optional POSIX domain SID for uid/gid mapping.
    pub posix_domain_sid: Option<String>,
}

impl PlatformWindowsOptionsJson {
    /// Apply windows overrides to a base set of options.
    pub fn apply_to(&self, options: &mut PlatformWindowsOptions) {
        // apply domain SID overrides
        if let Some(domain_sid) = &self.posix_domain_sid {
            options.posix_domain_sid = Some(domain_sid.clone());
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
    /// Use the Windows IOCP backend.
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
