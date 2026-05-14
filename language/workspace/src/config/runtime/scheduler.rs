use serde::{Deserialize, Serialize};

/// Runtime scheduler mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SchedulerMode {
    /// Run all world progress cooperatively on one scheduler.
    Cooperative,
    /// Allow host-parallel progress across multiple workers.
    #[default]
    Parallel,
}

/// Runtime task scheduling policy.
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

/// Runtime scheduler configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct SchedulerOptions {
    /// Runtime scheduler mode.
    pub mode: SchedulerMode,
    // event loop work
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
    pub poller_backend: PollerBackend,

    // scheduled work
    /// Scheduling policy for ready work.
    pub policy: SchedulerPolicy,
    /// Maximum number of live scheduled tasks.
    pub task_limit: Option<u64>,
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
    /// Use poll (portable Unix backend).
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

/// Runtime scheduler options for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct SchedulerOptionsJson {
    /// Runtime scheduler mode.
    pub mode: Option<SchedulerModeJson>,
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
    /// Scheduling policy for ready work.
    pub policy: Option<SchedulerPolicyJson>,
    /// Maximum number of live scheduled tasks.
    pub task_limit: Option<u64>,
}

impl SchedulerOptionsJson {
    /// Inherit unset scheduler settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.mode.is_none() {
            self.mode = parent.mode;
        }
        if self.tick_budget_ns.is_none() {
            self.tick_budget_ns = parent.tick_budget_ns;
        }
        if self.microtask_budget.is_none() {
            self.microtask_budget = parent.microtask_budget;
        }
        if self.max_microtask_depth.is_none() {
            self.max_microtask_depth = parent.max_microtask_depth;
        }
        if self.timer_resolution_ns.is_none() {
            self.timer_resolution_ns = parent.timer_resolution_ns;
        }
        if self.max_timer_coalesce_ns.is_none() {
            self.max_timer_coalesce_ns = parent.max_timer_coalesce_ns;
        }
        if self.preempt_interval_ns.is_none() {
            self.preempt_interval_ns = parent.preempt_interval_ns;
        }
        if self.poller_backend.is_none() {
            self.poller_backend = parent.poller_backend;
        }
        if self.policy.is_none() {
            self.policy = parent.policy;
        }
        if self.task_limit.is_none() {
            self.task_limit = parent.task_limit;
        }
    }

    /// Apply scheduler overrides to a base set of options.
    pub fn apply_to(&self, options: &mut SchedulerOptions) {
        // scheduler mode
        if let Some(mode) = self.mode {
            options.mode = mode.into();
        }

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
        if let Some(task_limit) = self.task_limit {
            options.task_limit = Some(task_limit);
        }
    }
}

/// Runtime scheduler mode for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum SchedulerModeJson {
    /// Run all world progress cooperatively on one scheduler.
    Cooperative,
    /// Allow host-parallel progress across multiple workers.
    Parallel,
}

impl From<SchedulerModeJson> for SchedulerMode {
    fn from(value: SchedulerModeJson) -> Self {
        match value {
            SchedulerModeJson::Cooperative => Self::Cooperative,
            SchedulerModeJson::Parallel => Self::Parallel,
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
    /// Use poll (portable Unix backend).
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
