use serde::{Deserialize, Serialize};

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
