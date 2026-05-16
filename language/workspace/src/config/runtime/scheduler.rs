use serde::{Deserialize, Serialize};

/// Runtime scheduler mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum SchedulerMode {
    /// Run all world progress cooperatively on one scheduler.
    Cooperative,
    /// Allow host-parallel progress across multiple workers.
    #[default]
    Parallel,
}

/// Runtime task scheduling policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
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
